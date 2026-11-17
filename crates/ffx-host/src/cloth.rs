//! The characters' cloth as the game draws it (docs/state-api.md,
//! "Cloth"): an opt-in capture of the simulated vertices
//! `graphicCharacterInstanceSetClothVertexBuffers` (`0x0063b740`) hands the
//! renderer each frame, for a frontend that draws the characters' hair and
//! cloth moving (gd-ffx).
//!
//! The cloth is a vertex simulation, not bones: `Ch_DrawMain` queues one
//! `Ch_Cloth_PerCharacter` job per cloth character; `Sg_ClothStep`
//! (`0x00822de0`) moves the PS2 model's cloth points and
//! `Sg_ClothPlaceAnchors` (`0x0082dbc0`) puts each HD vertex of a cloth
//! segment on its PS2 triangle (the binding `Ch_PreCalcActiveClothMesh`
//! made), into the segment's cloth object (`+0x4c`, 16 bytes a vertex, in
//! the segment's vertex order). `Sg_MainLoop` waits for the jobs at the
//! end of the update. Then `FFXApplication_render` (`0x0042f930`, the
//! main thread) runs `graphicCharacterInstanceSetClothVertexBuffers`,
//! whose `batchSetClothVertexBuffers` (`0x006365f0`) gives each cloth
//! segment's draw those vertices and the object's first 16 floats as the
//! shader's `ClothWorld` (docs/host.md, "The cloth race").
//!
//! The capture is off unless a frontend switches it on
//! ([`Capture::set_enabled`]); off, the wrapper only checks the switch and
//! calls the port. On, it first reads what the call is about to hand over,
//! with the call's own conditions: per character of the cloth table
//! (`0x00ccb254`, count `0x00ccb250`) that it submits, per instance and
//! cloth segment, the segment's model and piece (`PMeshSegment`
//! `+0x20`/`+0x24`), `ClothWorld` and the vertices. It reads after the
//! cloth jobs are done and before the renderer copies the buffers, so it
//! never races the jobs. It only reads game memory: on or off, the game
//! runs the same.
//!
//! It runs between update `n` and the boundary of update `n + 1`, in the
//! same `FFXApplication_render` as `PhyreScene_DrawMostVFX`
//! ([`crate::vfx`]): [`Frame::update`] is `n`, the
//! [`ffx_state::Snapshot::update`] of the state it shows.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};
use crate::vfx::{In, Out, Reader};

/// `graphicCharacterInstanceSetClothVertexBuffers()` (cdecl, no arguments).
pub const SET_CLOTH_VERTEX_BUFFERS: u32 = 0x0063_b740;
/// The cloth characters' table: the slot count and the slots (one cloth
/// record each, a character's `+0x834`).
pub const TABLE_COUNT: u32 = 0x00cc_b250;
pub const TABLE: u32 = 0x00cc_b254;
/// The encoding's first four bytes ([`Frame::encode`]).
pub const MAGIC: [u8; 4] = *b"CLTH";
/// The encoding's version.
pub const VERSION: u16 = 1;
/// A cloth vertex counts as simulated by this much: the cloth shaders mix
/// the skinned position and the cloth one by `min(|v|^2 * 10000, 1)` of
/// the vertex's `x, y, z` ([`Segment::weight`]); a vertex the simulation
/// leaves at zero stays skinned.
pub const WEIGHT_SCALE: f32 = 10_000.0;
/// A character's cloth record (`graphicCharacterInstanceGetClothVertexBuffers`).
const RECORD_CLOTH: u32 = 0x834;
/// Bounds on what one frame reads (guards against a torn table).
const SLOTS_MAX: u32 = 256;
const INSTANCES_MAX: u32 = 64;
const SEGMENTS_MAX: u32 = 256;
const VERTICES_MAX: u32 = 0x1_0000;
/// A `PMeshSegment`'s size.
const SEGMENT_SIZE: u32 = 0x6c;
/// Frames kept ([`Capture::frame`]).
const KEPT: usize = 4;

/// One cloth segment of a character's instance, as the game hands it to
/// the draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Segment {
    /// The instance's place in the character's instance list (the cloth
    /// record's `+0x14` of them).
    pub instance: u16,
    /// The segment's index in its mesh (`PMesh` `+0`/`+4`, `0x6c` bytes
    /// each).
    pub segment: u16,
    /// The PS2 model and cloth piece the segment draws (`PMeshSegment`
    /// `+0x20`, `+0x24`; the `.cdf`'s model and piece).
    pub model: u16,
    pub piece: u16,
    /// The segment's own vertex count (its first `PDataBlock`'s
    /// `m_elementCount`, `+0x28`/`+0x2c`; 0 when unread). Equal to
    /// `local.len()` on every segment measured.
    pub segment_vertices: u32,
    /// `ClothWorld` as the game holds it (the cloth object's first 16
    /// floats, memory order): `Ch_UpdateClothMatrix` (`0x008314c0`) sets
    /// it from the model's base element (translation × 1000), the
    /// character's root and the flip of x and y. [`to_render`] applies it.
    pub world: [f32; 16],
    /// The vertices as the game uploads them (the cloth object's `+0x4c`,
    /// `+0x44` of them, 16 bytes each, in the segment's vertex order): the
    /// cloth position in the segment's cloth space (`x, y, z`) and a fourth
    /// float the shaders do not read for it.
    pub local: Vec<[f32; 4]>,
    /// Per vertex: the cloth position in render space (`world` applied,
    /// [`to_render`]) and its weight ([`weight`]).
    pub positions: Vec<[f32; 3]>,
    pub weights: Vec<f32>,
}

impl Segment {
    /// A segment from what the game holds, `positions` and `weights`
    /// computed.
    #[must_use]
    pub fn new(
        (instance, segment): (u16, u16),
        (model, piece): (u16, u16),
        segment_vertices: u32,
        world: [f32; 16],
        local: Vec<[f32; 4]>,
    ) -> Self {
        let positions = local.iter().map(|v| to_render(&world, *v)).collect();
        let weights = local.iter().map(|v| weight(*v)).collect();
        Self {
            instance,
            segment,
            model,
            piece,
            segment_vertices,
            world,
            local,
            positions,
            weights,
        }
    }
}

/// The cloth of one character the game submits this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Character {
    /// The `Snapshot.actors` `slot` (`g_ChrArray` index) whose `+0x834` the
    /// cloth record is; -1 when no record holds it.
    pub actor: i16,
    /// The character's id (its `+0`; 0 without a record).
    pub id: u16,
    /// The slot in the cloth table (`0x00ccb254`).
    pub table_slot: u16,
    /// Its cloth segments, by instance and then segment.
    pub segments: Vec<Segment>,
}

/// One `graphicCharacterInstanceSetClothVertexBuffers` call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// The updates done when it ran: the [`ffx_state::Snapshot::update`] of
    /// the state it draws.
    pub update: u32,
    /// The characters it submits, in the table's order.
    pub characters: Vec<Character>,
}

impl Frame {
    /// The cloth of the actor with `Snapshot.actors` slot `actor`.
    #[must_use]
    pub fn actor(&self, actor: i16) -> Option<&Character> {
        self.characters.iter().find(|c| c.actor == actor)
    }
}

/// `local` placed by `ClothWorld` (memory order, as [`Segment::world`]):
/// the game's matrices take row vectors, so the point is
/// `x · row 0 + y · row 1 + z · row 2 + row 3`. The result is in render
/// space (docs/state-api.md, "Spaces": the field's x and y negated), the
/// space `Camera.world` and the effect draws use.
#[must_use]
pub fn to_render(world: &[f32; 16], local: [f32; 4]) -> [f32; 3] {
    std::array::from_fn(|k| {
        local[0] * world[k] + local[1] * world[4 + k] + local[2] * world[8 + k] + world[12 + k]
    })
}

/// How much of the cloth position the shaders take over the skinned one:
/// `min((x² + y² + z²) · 10000, 1)` ([`WEIGHT_SCALE`]); 0 for a vertex the
/// simulation does not move (anchored, or skinned by its colour).
#[must_use]
pub fn weight(local: [f32; 4]) -> f32 {
    let d = local[0] * local[0] + local[1] * local[1] + local[2] * local[2];
    (d * WEIGHT_SCALE).min(1.0)
}

/// The capture's switch and its newest frames (`Shared::cloth`).
#[derive(Debug, Default)]
pub struct Capture {
    on: AtomicBool,
    frames: Mutex<Vec<Arc<Frame>>>,
}

impl Capture {
    /// Switches the capture on or off (off: the frames kept are dropped).
    pub fn set_enabled(&self, on: bool) {
        self.on.store(on, Ordering::Release);
        if !on {
            self.frames
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear();
        }
    }

    /// Whether the capture is on.
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }

    /// The newest frame.
    #[must_use]
    pub fn latest(&self) -> Option<Arc<Frame>> {
        let f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        f.last().cloned()
    }

    /// The frame of update `update` (a snapshot's), among the last few.
    #[must_use]
    pub fn frame(&self, update: u32) -> Option<Arc<Frame>> {
        let f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        f.iter().rev().find(|x| x.update == update).cloned()
    }

    /// Keeps `frame` as the newest.
    pub fn push(&self, frame: Frame) {
        let mut f = self.frames.lock().unwrap_or_else(PoisonError::into_inner);
        // switched off while this frame was read: keep nothing (set_enabled
        // cleared the list under the same lock)
        if !self.enabled() {
            return;
        }
        if f.len() >= KEPT {
            f.remove(0);
        }
        f.push(Arc::new(frame));
    }
}

/// Registers the wrapper on `graphicCharacterInstanceSetClothVertexBuffers`.
pub fn register(t: &mut Table) {
    // push ebp; mov ebp, esp; sub esp, 8
    t.wrap(
        SET_CLOTH_VERTEX_BUFFERS,
        Entry {
            name: "graphicCharacterInstanceSetClothVertexBuffers",
            conv: Conv::Cdecl,
            words: 0,
            port: Port::Exec(set_cloth_vertex_buffers),
        },
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
    );
}

/// `graphicCharacterInstanceSetClothVertexBuffers()`: with the capture on,
/// the frame is read first; then the original (its port) submits.
fn set_cloth_vertex_buffers(e: &mut Exec, args: &Args) -> Ret {
    let shared = e.shared();
    if shared.cloth.enabled() {
        let frame = read_frame(&Reader(shared.arena()), shared.clock.frame());
        shared.cloth.push(frame);
    }
    e.call_original(SET_CLOTH_VERTEX_BUFFERS, Conv::Cdecl, args)
        .unwrap_or(Ret::eax(0))
}

/// `EighthAtLeastOne(n)` (`0x006db870`): `n / 8` above 8, else 1.
fn batch_size(n: i32) -> i32 {
    if n > 8 { n / 8 } else { 1 }
}

/// Whether table entry `entry` is ready to submit
/// (`graphicCharacterInstanceSetClothVertexBuffers`' test, again in
/// `batchSetClothVertexBuffers` with the parallel list): its instance list
/// `+0` set, `+8` (the hide byte) 0, `+0xc` (the state) 2.
fn ready(m: &Reader, entry: u32) -> bool {
    m.ptr(entry).is_some()
        && m.u32(entry.wrapping_add(8)) == Some(0)
        && m.u32(entry.wrapping_add(0xc)) == Some(2)
}

/// The slots `graphicCharacterInstanceSetClothVertexBuffers` hands to
/// `batchSetClothVertexBuffers`: it counts the ready slots and submits the
/// slots since the last batch each time the count reaches
/// `EighthAtLeastOne(slot count)`; ready slots after the last full batch
/// are not submitted (a short batch at the end, with 16 slots or more).
fn submitted_slots(m: &Reader) -> Vec<u32> {
    let count = m.u32(TABLE_COUNT).unwrap_or(0).min(SLOTS_MAX);
    let batch = batch_size(count.cast_signed());
    let mut out = Vec::new();
    let (mut start, mut n) = (0u32, 0i32);
    for i in 0..count {
        let Some(entry) = m.ptr(TABLE.wrapping_add(4 * i)) else {
            continue;
        };
        if !ready(m, entry) {
            continue;
        }
        n += 1;
        if n == batch {
            out.extend(start..=i);
            start = i + 1;
            n = 0;
        }
    }
    out
}

/// The `g_ChrArray` records by their cloth record (`+0x834`).
fn actors(m: &Reader) -> HashMap<u32, (i16, u16)> {
    let mut out = HashMap::new();
    let Some(base) = m.ptr(crate::vfx::CHR_ARRAY) else {
        return out;
    };
    let count = m
        .u32(crate::vfx::CHR_COUNT)
        .unwrap_or(0)
        .cast_signed()
        .clamp(0, i32::from(i16::MAX));
    for i in 0..count.cast_unsigned() {
        let chr = base.wrapping_add(i * crate::vfx::CHR_SIZE);
        if let Some(rc) = m.ptr(chr.wrapping_add(RECORD_CLOTH)) {
            let id = m
                .bytes(chr, 2)
                .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
            out.entry(rc)
                .or_insert((i16::try_from(i).unwrap_or(-1), id));
        }
    }
    out
}

/// The frame `graphicCharacterInstanceSetClothVertexBuffers` is about to
/// submit: for each slot it hands over whose entry is ready, with its
/// parallel list (`+4`) set, each instance `k` (`+0x14` of them; the
/// instance list's items at `[+0] + 4`) with a cloth table
/// (`entry + 0x30 + 4k`) and a parallel table (`[+4] + 4 + 4k`), each
/// segment `s` of the instance's mesh whose `+0x20` and `+0x24` halves are
/// not `0xffff`, with a cloth object (`table + 4 + 4s`) and a parallel word
/// (`+4 + 4s` of its table): the object's matrix and vertices (as
/// `batchSetClothVertexBuffers`).
#[must_use]
pub fn read_frame(m: &Reader, update: u32) -> Frame {
    let by_record = actors(m);
    let mut frame = Frame {
        update,
        characters: Vec::new(),
    };
    for slot in submitted_slots(m) {
        let Some(entry) = m.ptr(TABLE.wrapping_add(4 * slot)) else {
            continue;
        };
        if !ready(m, entry) {
            continue;
        }
        let (Some(list), Some(parallel)) = (m.ptr(entry), m.ptr(entry.wrapping_add(4))) else {
            continue;
        };
        let (actor, id) = by_record.get(&entry).copied().unwrap_or((-1, 0));
        let mut c = Character {
            actor,
            id,
            table_slot: u16::try_from(slot).unwrap_or(u16::MAX),
            segments: Vec::new(),
        };
        let instances = m
            .u32(entry.wrapping_add(0x14))
            .unwrap_or(0)
            .cast_signed()
            .clamp(0, INSTANCES_MAX.cast_signed())
            .cast_unsigned();
        let items = m.u32(list.wrapping_add(4)).unwrap_or(0);
        for k in 0..instances {
            let Some(instance) = m.ptr(items.wrapping_add(4 * k)) else {
                continue;
            };
            let (Some(cloths), Some(words)) = (
                m.ptr(entry.wrapping_add(0x30 + 4 * k)),
                m.ptr(parallel.wrapping_add(4 + 4 * k)),
            ) else {
                continue;
            };
            read_instance(m, instance, (cloths, words), k, &mut c.segments);
        }
        frame.characters.push(c);
    }
    frame
}

/// The cloth segments of instance `k` (`instance`, its mesh `*instance`).
fn read_instance(
    m: &Reader,
    instance: u32,
    (cloths, words): (u32, u32),
    k: u32,
    out: &mut Vec<Segment>,
) {
    let Some(mesh) = m.ptr(instance) else {
        return;
    };
    let segments = (m.u32(mesh).unwrap_or(0) & 0x7fff_ffff).min(SEGMENTS_MAX);
    let Some(base) = m.ptr(mesh.wrapping_add(4)) else {
        return;
    };
    for s in 0..segments {
        let seg = base.wrapping_add(SEGMENT_SIZE * s);
        let Some(h) = m.bytes(seg.wrapping_add(0x20), 8) else {
            continue;
        };
        let (model, piece) = (
            u16::from_le_bytes([h[0], h[1]]),
            u16::from_le_bytes([h[4], h[5]]),
        );
        if model == 0xffff || piece == 0xffff {
            continue;
        }
        let Some(cloth) = m.ptr(cloths.wrapping_add(4 + 4 * s)) else {
            continue;
        };
        if m.ptr(words.wrapping_add(4 + 4 * s)).is_none() {
            continue;
        }
        let n = m
            .u32(cloth.wrapping_add(0x44))
            .unwrap_or(0)
            .min(VERTICES_MAX);
        let local = m.f32s::<4>(m.u32(cloth.wrapping_add(0x4c)).unwrap_or(0), n);
        let world = m.f32s::<16>(cloth, 1).first().copied().unwrap_or_default();
        // the first PDataBlock's m_elementCount
        let segment_vertices = (m.u32(seg.wrapping_add(0x28)).unwrap_or(0) & 0x7fff_ffff > 0)
            .then(|| m.ptr(seg.wrapping_add(0x2c)))
            .flatten()
            .and_then(|b| m.u32(b.wrapping_add(4)))
            .unwrap_or(0);
        out.push(Segment::new(
            (
                u16::try_from(k).unwrap_or(u16::MAX),
                u16::try_from(s).unwrap_or(u16::MAX),
            ),
            (model, piece),
            segment_vertices,
            world,
            local,
        ));
    }
}

// ---- the encoding ----------------------------------------------------------

fn len32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Frame {
    /// The frame as bytes (docs/state-api.md, "Cloth"): little endian;
    /// `CLTH`, version, 0, update, character count; per character its
    /// actor slot, id, table slot and segment count; per segment its
    /// instance, segment, model, piece, segment vertex count, vertex count
    /// `n`, `ClothWorld` (16 floats), then `n` local vertices (4 floats),
    /// `n` render-space positions (3 floats) and `n` weights.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out(Vec::new());
        o.0.extend_from_slice(&MAGIC);
        o.u16(VERSION);
        o.u16(0);
        o.u32(self.update);
        o.u32(len32(self.characters.len()));
        for c in &self.characters {
            o.u16(c.actor.cast_unsigned());
            o.u16(c.id);
            o.u16(c.table_slot);
            o.u16(u16::try_from(c.segments.len()).unwrap_or(u16::MAX));
            for s in c.segments.iter().take(usize::from(u16::MAX)) {
                o.u16(s.instance);
                o.u16(s.segment);
                o.u16(s.model);
                o.u16(s.piece);
                o.u32(s.segment_vertices);
                let n = s.local.len();
                o.u32(len32(n));
                for &w in &s.world {
                    o.f32(w);
                }
                for &v in s.local.iter().flatten() {
                    o.f32(v);
                }
                for k in 0..n {
                    for &v in s.positions.get(k).unwrap_or(&[0.0; 3]) {
                        o.f32(v);
                    }
                }
                for k in 0..n {
                    o.f32(s.weights.get(k).copied().unwrap_or(0.0));
                }
            }
        }
        o.0
    }

    /// A frame from [`Frame::encode`]'s bytes; `None` when they are not one
    /// (another magic or version, cut short, bytes left over).
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut i = In(bytes);
        if i.take(4)? != MAGIC || i.u16()? != VERSION {
            return None;
        }
        i.u16()?;
        let update = i.u32()?;
        let n_chars = i.u32()? as usize;
        let mut characters = Vec::with_capacity(n_chars.min(SLOTS_MAX as usize));
        for _ in 0..n_chars {
            let actor = i.u16()?.cast_signed();
            let id = i.u16()?;
            let table_slot = i.u16()?;
            let n_segs = i.u16()?;
            let mut segments = Vec::with_capacity(usize::from(n_segs).min(SEGMENTS_MAX as usize));
            for _ in 0..n_segs {
                let instance = i.u16()?;
                let segment = i.u16()?;
                let model = i.u16()?;
                let piece = i.u16()?;
                let segment_vertices = i.u32()?;
                let n = i.u32()? as usize;
                let world = i.f32s::<16>(1)?[0];
                let local = i.f32s::<4>(n)?;
                let positions = i.f32s::<3>(n)?;
                let weights = i.f32s::<1>(n)?.into_iter().map(|[w]| w).collect();
                segments.push(Segment {
                    instance,
                    segment,
                    model,
                    piece,
                    segment_vertices,
                    world,
                    local,
                    positions,
                    weights,
                });
            }
            characters.push(Character {
                actor,
                id,
                table_slot,
                segments,
            });
        }
        if !i.0.is_empty() {
            return None;
        }
        Some(Self { update, characters })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;

    fn sample() -> Frame {
        let world = [
            -1.0, 0.0, 0.0, 0.0, //
            0.0, -1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            10.0, 20.0, 30.0, 1.0,
        ];
        Frame {
            update: 412,
            characters: vec![
                Character {
                    actor: 3,
                    id: 1,
                    table_slot: 0,
                    segments: vec![
                        Segment::new(
                            (0, 5),
                            (1, 2),
                            3,
                            world,
                            vec![[1.0, 2.0, 3.0, 0.5], [0.0; 4], [0.001, 0.0, 0.0, 1.0]],
                        ),
                        Segment::new((1, 0), (2, 0), 0, world, Vec::new()),
                    ],
                },
                Character {
                    actor: -1,
                    id: 0,
                    table_slot: 7,
                    segments: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn the_point_and_weight_follow_the_shader() {
        let s = &sample().characters[0].segments[0];
        // x and y flipped, then moved
        assert_eq!(s.positions[0], [9.0, 18.0, 33.0]);
        assert_eq!(s.positions[1], [10.0, 20.0, 30.0]);
        assert_eq!(s.weights[0], 1.0);
        assert_eq!(s.weights[1], 0.0);
        // |v|^2 = 1e-6: a hundredth of the way
        assert!((s.weights[2] - 0.01).abs() < 1e-6, "{}", s.weights[2]);
    }

    #[test]
    fn a_frame_encodes_and_decodes_unchanged() {
        let f = sample();
        assert_eq!(Frame::decode(&f.encode()), Some(f.clone()));
        let empty = Frame::default();
        assert_eq!(Frame::decode(&empty.encode()), Some(empty));
        let b = f.encode();
        assert_eq!(Frame::decode(&b[..b.len() - 1]), None, "cut short");
        let mut longer = b.clone();
        longer.push(0);
        assert_eq!(Frame::decode(&longer), None, "bytes left over");
        let mut other = b;
        other[4] = 2;
        assert_eq!(Frame::decode(&other), None, "another version");
    }

    #[test]
    fn the_layout_is_the_documented_one() {
        let f = sample();
        let b = f.encode();
        let u16_at = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
        let u32_at = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let f32_at = |o: usize| f32::from_bits(u32_at(o));
        assert_eq!(&b[..4], b"CLTH");
        assert_eq!(u16_at(4), VERSION);
        assert_eq!(u16_at(6), 0);
        assert_eq!(u32_at(8), 412);
        assert_eq!(u32_at(12), 2);
        // character 0: actor, id, table slot, segment count
        assert_eq!(
            [u16_at(16), u16_at(18), u16_at(20), u16_at(22)],
            [3, 1, 0, 2]
        );
        // segment 0: instance, segment, model, piece, segment vertices, n
        let s = 24;
        assert_eq!(
            [u16_at(s), u16_at(s + 2), u16_at(s + 4), u16_at(s + 6)],
            [0, 5, 1, 2]
        );
        assert_eq!(u32_at(s + 8), 3);
        assert_eq!(u32_at(s + 12), 3);
        // ClothWorld, then local (4 floats), positions (3), weights (1)
        assert_eq!(f32_at(s + 16), -1.0);
        assert_eq!(f32_at(s + 16 + 4 * 12), 10.0);
        let local = s + 16 + 64;
        assert_eq!(f32_at(local + 12), 0.5);
        let positions = local + 3 * 16;
        assert_eq!(f32_at(positions + 8), 33.0);
        let weights = positions + 3 * 12;
        assert_eq!(f32_at(weights), 1.0);
        // segment 1: no vertices
        let s1 = weights + 3 * 4;
        assert_eq!([u16_at(s1), u16_at(s1 + 2), u16_at(s1 + 4)], [1, 0, 2]);
        assert_eq!(u32_at(s1 + 12), 0);
        // character 1
        let c1 = s1 + 16 + 64;
        assert_eq!(
            [u16_at(c1), u16_at(c1 + 2), u16_at(c1 + 4), u16_at(c1 + 6)],
            [0xffff, 0, 7, 0]
        );
        assert_eq!(b.len(), c1 + 8);
    }

    const HEAP: u32 = 0x0200_0000;
    const CHRS: u32 = 0x0300_0000;

    /// Game memory for [`read_frame`]: a heap, `g_ChrArray` and the cloth
    /// table.
    struct Fixture {
        arena: Arena,
        next: u32,
    }

    impl Fixture {
        fn new() -> Self {
            let arena = Arena::new().unwrap();
            arena.map(TABLE & !0xfff, 0x1000, "cloth table").unwrap();
            arena
                .map(crate::vfx::CHR_ARRAY & !0xfff, 0x1000, "chr globals")
                .unwrap();
            arena.map(HEAP, 0x10_0000, "heap").unwrap();
            arena.map(CHRS, 0x1_0000, "chrs").unwrap();
            arena.write_bytes(crate::vfx::CHR_ARRAY, &CHRS.to_le_bytes());
            arena.write_bytes(crate::vfx::CHR_COUNT, &4u32.to_le_bytes());
            Self { arena, next: HEAP }
        }

        fn alloc(&mut self, n: u32) -> u32 {
            let at = self.next;
            self.next += n.next_multiple_of(16);
            at
        }

        fn w32(&self, at: u32, v: u32) {
            self.arena.write_bytes(at, &v.to_le_bytes());
        }

        fn floats(&self, at: u32, v: &[f32]) {
            let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
            self.arena.write_bytes(at, &b);
        }

        /// A cloth record in table slot `slot` for `g_ChrArray` record
        /// `actor` (id `0x100 + actor`), ready (or hidden), with one
        /// instance whose mesh has three segments: 0 not cloth, 1 cloth
        /// (model 1, piece `piece`, two vertices), 2 cloth without a cloth
        /// object.
        fn character(&mut self, slot: u32, actor: Option<u32>, hidden: bool, piece: u16) -> u32 {
            let entry = self.alloc(0x40);
            self.w32(TABLE + 4 * slot, entry);
            let segs = self.alloc(3 * SEGMENT_SIZE);
            for s in 0..3 {
                let seg = segs + SEGMENT_SIZE * s;
                let (model, p): (u16, u16) = if s == 0 { (0xffff, 0xffff) } else { (1, piece) };
                self.w32(seg + 0x20, u32::from(model) | 0xffff_0000);
                self.w32(seg + 0x24, u32::from(p) | 0xffff_0000);
            }
            let block = self.alloc(0x40);
            self.w32(block + 4, 2);
            self.w32(segs + SEGMENT_SIZE + 0x28, 1);
            self.w32(segs + SEGMENT_SIZE + 0x2c, block);
            let mesh = self.alloc(8);
            self.w32(mesh, 3);
            self.w32(mesh + 4, segs);
            let instance = self.alloc(0x20);
            self.w32(instance, mesh);
            let items = self.alloc(4);
            self.w32(items, instance);
            let list = self.alloc(0x10);
            self.w32(list + 4, items);
            let cloth = self.alloc(0x50);
            let mut world = [0.0f32; 16];
            for k in [0, 5, 10, 15] {
                world[k] = 1.0;
            }
            world[12] = 100.0;
            self.floats(cloth, &world);
            let verts = self.alloc(32);
            self.floats(verts, &[1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
            self.w32(cloth + 0x44, 2);
            self.w32(cloth + 0x4c, verts);
            let cloths = self.alloc(16);
            self.w32(cloths, 3);
            self.w32(cloths + 4 + 4, cloth);
            let words = self.alloc(16);
            self.w32(words + 4 + 4, 1);
            self.w32(words + 4 + 8, 1);
            let par_inner = words;
            let parallel = self.alloc(8);
            self.w32(parallel + 4, par_inner);
            self.w32(entry, list);
            self.w32(entry + 4, parallel);
            self.w32(entry + 8, u32::from(hidden));
            self.w32(entry + 0xc, 2);
            self.w32(entry + 0x14, 1);
            self.w32(entry + 0x30, cloths);
            if let Some(a) = actor {
                let chr = CHRS + crate::vfx::CHR_SIZE * a;
                self.w32(chr, 0x100 + a);
                self.w32(chr + RECORD_CLOTH, entry);
            }
            entry
        }
    }

    #[test]
    fn the_frame_is_what_the_call_submits() {
        let mut f = Fixture::new();
        f.w32(TABLE_COUNT, 4);
        f.character(0, Some(2), false, 7);
        f.character(1, None, true, 8);
        // slot 2 empty
        f.character(3, None, false, 9);
        let frame = read_frame(&Reader(&f.arena), 99);
        assert_eq!(frame.update, 99);
        assert_eq!(frame.characters.len(), 2, "{frame:?}");
        let c = &frame.characters[0];
        assert_eq!((c.actor, c.id, c.table_slot), (2, 0x102, 0));
        assert_eq!(c.segments.len(), 1, "only the segment with a cloth object");
        let s = &c.segments[0];
        assert_eq!((s.instance, s.segment, s.model, s.piece), (0, 1, 1, 7));
        assert_eq!(s.segment_vertices, 2);
        assert_eq!(s.local, vec![[1.0, 2.0, 3.0, 0.0], [0.0; 4]]);
        assert_eq!(s.positions, vec![[101.0, 2.0, 3.0], [100.0, 0.0, 0.0]]);
        assert_eq!(s.weights, vec![1.0, 0.0]);
        let c = &frame.characters[1];
        assert_eq!((c.actor, c.id, c.table_slot), (-1, 0, 3));
        assert_eq!(c.segments[0].piece, 9);
    }

    #[test]
    fn ready_slots_after_the_last_full_batch_are_not_submitted() {
        let mut f = Fixture::new();
        // 18 slots: batches of 2; three ready (0, 5, 9): 0 and 5 go, 9 waits
        f.w32(TABLE_COUNT, 18);
        for (slot, piece) in [(0, 1), (5, 2), (9, 3)] {
            f.character(slot, None, false, piece);
        }
        let frame = read_frame(&Reader(&f.arena), 1);
        let slots: Vec<u16> = frame.characters.iter().map(|c| c.table_slot).collect();
        assert_eq!(slots, vec![0, 5]);
        assert_eq!(batch_size(8), 1);
        assert_eq!(batch_size(9), 1);
        assert_eq!(batch_size(18), 2);
    }
}
