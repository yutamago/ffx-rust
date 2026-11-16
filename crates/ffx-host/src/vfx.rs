//! The effects' draws as the game makes them (docs/state-api.md, "Effect
//! draws"): an opt-in capture of what `PhyreScene_DrawMostVFX`
//! (`0x00650670`) draws each frame, for a frontend that draws the game's
//! own effects (gd-ffx).
//!
//! The capture is off unless a frontend switches it on
//! ([`Capture::set_enabled`]); off, the wrapper only checks the switch and
//! calls the port. On, it first reads the scene's list `+0x38cc` (count
//! `+0xf8cc`), its draw events (`+0xfb5c`, count `+0xfb54`) and, for each
//! entry, the `VFXDynamicGeometry` of `g_VFXRenderDataTable` whose mesh
//! instance (`+0xc`) it is: the instance key, the VSP flags (`+0x3c`), the
//! mesh instance's world matrix and each segment with indices, with its
//! texture name and vertex arrays (`docs/formats/effects-draw.md`). It only
//! reads game memory: on or off, the game runs the same.
//!
//! `DrawMostVFX` runs on the game's main thread between two updates, after
//! update `n` and before the boundary of update `n + 1`: the frame it draws
//! is the state after `n` updates, the [`ffx_state::Snapshot`] whose
//! `update` is `n` ([`Frame::update`]; checked against reads at the
//! boundary on Besaid, docs/state-api.md).
//!
//! The newest frames are kept ([`Capture::latest`], [`Capture::frame`]) and
//! encode to a blob of their own ([`Frame::encode`]), apart from the
//! snapshot.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use crate::arena::Arena;
use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `PhyreScene_DrawMostVFX(this)` (thiscall, `eax` 0 or an event's error).
pub const DRAW_MOST_VFX: u32 = 0x0065_0670;
/// `g_VFXRenderDataTable`: the pointer to the table.
pub const TABLE: u32 = 0x00cc_ca6c;
/// `g_MagicFile_MagicId` (what `MagicFile_GetMagicId` `0x009da3b0` reads).
pub const MAGIC_ID: u32 = 0x00c6_4ca0;
/// The magic whose geometry [`LAST_DRAWN_KEY`] `DrawMostVFX` draws last.
pub const LAST_DRAWN_MAGIC: u32 = 0x296;
/// The key of that geometry, `(lo, hi)` = `(0x78, 0x5bc)`.
pub const LAST_DRAWN_KEY: u64 = (0x5bc << 32) | 0x78;
/// The encoding's first four bytes ([`Frame::encode`]).
pub const MAGIC: [u8; 4] = *b"FXDR";
/// The encoding's version.
pub const VERSION: u16 = 1;
/// The list's capacity (`+0x38cc`, 0xc00 entries).
const LIST_MAX: u32 = 0xc00;
/// Bounds on what one frame reads (guards against a torn list).
const SEGMENTS_MAX: i32 = 256;
const VERTICES_MAX: u32 = 0x1_0000;
const INDICES_MAX: u32 = 0x3_0000;
const EVENTS_MAX: u32 = 0x1000;
/// The identity in [`Draw::world`]'s layout.
pub const IDENTITY: [f32; 12] = [0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
/// Frames kept ([`Capture::frame`]).
const KEPT: usize = 4;

/// One indexed draw: one segment of one list entry's geometry.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Draw {
    /// The entry's place in the list (`+0x38cc`), the position draw events
    /// are given in.
    pub place: u32,
    /// The instance key `(hi << 32) | lo`, as the effect code passes it to
    /// `graphicVFXGetRenderDataByInstance(lo, hi)`: for a ppp program
    /// `(g_pppCurrentProgram, object)`, for the op system `(0, record)`, a
    /// part `i` of a several-part effect at `lo + 10000 i`
    /// (`docs/formats/effects-draw.md`, "Instance keys and makers").
    pub key: u64,
    /// The `VFXDynamicGeometry` (an address, for logs).
    pub geometry: u32,
    /// Its VSP flags (`+0x3c`): `0x8` modulate, `0xa` additive, `0x18`
    /// subtract, `0x9` z-matrix, `0x2` additive untextured, …
    /// (`vfxShader_fx.txt`; docs/formats/effects-draw.md, "Shaders").
    pub flags: u32,
    /// The geometry is in the table's delete pool (destroyed this frame,
    /// still drawn).
    pub pooled: bool,
    /// The segment's place in the geometry's render data.
    pub segment: u16,
    /// The mesh instance's world matrix (`*(mesh instance +4)`), 12 floats
    /// laid out as `ffx_state::Camera::world`:
    /// `[Y.x Y.y Y.z X.x | Z.x Z.y Z.z X.y | P.x P.y P.z X.z]` (axes X, Y,
    /// Z and the place P, render space). [`Draw::positions`] are already
    /// placed by it ([`to_render`]). [`IDENTITY`] when there is none.
    pub world: [f32; 12],
    /// The segment's texture name (`+0x20`) as the game holds it, e.g.
    /// `../../../FFX_Data/GameData/PS3Data/map/bsil/bsil00/fp/tex/D3D11/13824_19_0_0_128_128.dds.phyre`;
    /// `NoTexture` for untextured draws, empty when unreadable.
    pub texture: String,
    /// Positions in render space.
    pub positions: Vec<[f32; 3]>,
    /// Colours (R, G, B, A) as the arrays hold them (`0.5` is the PS2's
    /// `0x80`, which the VFX shaders double). As many as positions.
    pub colours: Vec<[f32; 4]>,
    /// Texture coordinates; as many as positions (not meaningful without a
    /// texture: the arrays are left uninitialised).
    pub uvs: Vec<[f32; 2]>,
    /// Triangle-list indices into the vertices.
    pub indices: Vec<u16>,
}

/// A `PhyreScene_addVFXDrawEvent` record, run once `at` list places are
/// drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Event {
    /// 0 characters with the event camera, 1 copy the colour or depth
    /// buffer, 2 screen fade, 3 PS2 lighting, 4 specular, 5 draw a
    /// character, 6 z pre-pass (`PhyreScene_processVFXDrawEvent`).
    pub kind: u32,
    pub at: u32,
    pub words: [u32; 2],
}

/// One `DrawMostVFX` call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// The updates done when it drew: the [`ffx_state::Snapshot::update`]
    /// of the state it shows.
    pub update: u32,
    /// The draws, in draw order.
    pub draws: Vec<Draw>,
    pub events: Vec<Event>,
    /// List entries whose mesh instance is in no geometry of the table (the
    /// game draws them; not captured).
    pub unresolved: u32,
}

/// The capture's switch and its newest frames (`Shared::vfx`).
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
        if f.len() >= KEPT {
            f.remove(0);
        }
        f.push(Arc::new(frame));
    }
}

/// Registers the wrapper on `PhyreScene_DrawMostVFX`.
pub fn register(t: &mut Table) {
    // push ebp; mov ebp, esp; sub esp, 0x14
    t.wrap(
        DRAW_MOST_VFX,
        Entry {
            name: "PhyreScene_DrawMostVFX",
            conv: Conv::Thiscall,
            words: 0,
            port: Port::Exec(draw_most_vfx),
        },
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x14],
    );
}

/// `PhyreScene_DrawMostVFX(scene)`: with the capture on, the frame is read
/// first; then the original (its port) draws.
fn draw_most_vfx(e: &mut Exec, args: &Args) -> Ret {
    let shared = e.shared();
    if shared.vfx.enabled() {
        let frame = read_frame(&Reader(shared.arena()), args.ecx, shared.clock.frame());
        shared.vfx.push(frame);
    }
    e.call_original(DRAW_MOST_VFX, Conv::Thiscall, args)
        .unwrap_or(Ret::eax(0))
}

/// Game memory, read-only; a read of unmapped memory fails.
pub struct Reader<'a>(pub &'a Arena);

impl Reader<'_> {
    fn bytes(&self, at: u32, n: usize) -> Option<Vec<u8>> {
        if at == 0 || !self.0.is_mapped(at, n) {
            return None;
        }
        let mut v = vec![0; n];
        self.0.read_bytes(at, &mut v);
        Some(v)
    }

    fn u32(&self, at: u32) -> Option<u32> {
        let b = self.bytes(at, 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u8(&self, at: u32) -> Option<u8> {
        self.bytes(at, 1).map(|b| b[0])
    }

    fn ptr(&self, at: u32) -> Option<u32> {
        self.u32(at).filter(|&p| p != 0)
    }

    /// A NUL-terminated string at `at` (at most 256 bytes).
    fn c_str(&self, at: u32) -> Option<String> {
        let mut s = Vec::new();
        for i in 0..256 {
            let b = self.u8(at.wrapping_add(i))?;
            if b == 0 {
                return Some(String::from_utf8_lossy(&s).into_owned());
            }
            s.push(b);
        }
        None
    }

    /// `n` groups of `N` floats at `at` (none when unreadable).
    fn f32s<const N: usize>(&self, at: u32, n: u32) -> Vec<[f32; N]> {
        let Some(b) = self.bytes(at, 4 * N * n as usize) else {
            return Vec::new();
        };
        b.chunks_exact(4 * N)
            .map(|c| {
                std::array::from_fn(|k| {
                    f32::from_le_bytes([c[4 * k], c[4 * k + 1], c[4 * k + 2], c[4 * k + 3]])
                })
            })
            .collect()
    }

    fn u16s(&self, at: u32, n: u32) -> Vec<u16> {
        self.bytes(at, 2 * n as usize)
            .map(|b| {
                b.chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A geometry of the table: its key, address, whether pooled.
#[derive(Debug, Clone, Copy)]
struct Found {
    key: u64,
    geometry: u32,
    pooled: bool,
}

/// The table's geometries by mesh instance (`+0xc`): the map (an
/// in-order walk of the MSVC tree: `+0` left, `+8` right, `+0xd` is-nil,
/// `+0x10` key, `+0x18` geometry), then the delete pool (count `+8`, bit 31
/// a flag; keys `+0xc`, geometries `+0x1c`).
fn geometries(m: &Reader) -> HashMap<u32, Found> {
    let mut out = HashMap::new();
    let Some(table) = m.ptr(TABLE) else {
        return out;
    };
    let mut add = |key: u64, g: u32, pooled: bool| {
        if let Some(mi) = m.ptr(g + 0xc) {
            out.entry(mi).or_insert(Found {
                key,
                geometry: g,
                pooled,
            });
        }
    };
    if let Some(head) = m.ptr(table) {
        let mut stack = Vec::new();
        let mut node = m.u32(head + 4).unwrap_or(head);
        let mut guard = 0;
        loop {
            while node != head && node != 0 && m.u8(node + 0xd) == Some(0) && guard < 100_000 {
                guard += 1;
                stack.push(node);
                node = m.u32(node).unwrap_or(head);
            }
            let Some(n) = stack.pop() else { break };
            let lo = m.u32(n + 0x10).unwrap_or(0);
            let hi = m.u32(n + 0x14).unwrap_or(0);
            if let Some(g) = m.ptr(n + 0x18) {
                add((u64::from(hi) << 32) | u64::from(lo), g, false);
            }
            node = m.u32(n + 8).unwrap_or(head);
        }
    }
    let n = m.u32(table + 8).unwrap_or(0) & 0x7fff_ffff;
    if let (Some(keys), Some(geoms)) = (m.ptr(table + 0xc), m.ptr(table + 0x1c)) {
        for i in 0..n.min(0x1_0000) {
            let lo = m.u32(keys + 8 * i).unwrap_or(0);
            let hi = m.u32(keys + 8 * i + 4).unwrap_or(0);
            if lo == 0 && hi == 0 {
                continue;
            }
            if let Some(g) = m.ptr(geoms + 4 * i) {
                add((u64::from(hi) << 32) | u64::from(lo), g, true);
            }
        }
    }
    out
}

/// `p` placed by a world matrix laid out as [`Draw::world`].
#[must_use]
pub fn to_render(w: &[f32; 12], p: [f32; 3]) -> [f32; 3] {
    let x = [w[3], w[7], w[11]];
    let y = [w[0], w[1], w[2]];
    let z = [w[4], w[5], w[6]];
    std::array::from_fn(|k| x[k] * p[0] + y[k] * p[1] + z[k] * p[2] + w[8 + k])
}

/// The draws of one list entry: each segment with indices (`+8` > 0) of
/// its geometry's render data (`+0x24`: `+0x90` count, `+0x94` segments of
/// `0x6c` bytes), with the segment's own arrays (`+4` vertices, `+8`
/// indices, `+0xc` positions, `+0x14` colours, `+0x18` uvs, `+0x1c`
/// indices; `docs/formats/render.md`), placed by the mesh instance's world
/// matrix (`*(mi +4)`).
fn entry_draws(m: &Reader, place: u32, mi: u32, found: Found, out: &mut Vec<Draw>) {
    let g = found.geometry;
    let flags = m.u32(g + 0x3c).unwrap_or(0);
    let world = m
        .ptr(mi + 4)
        .and_then(|p| m.f32s::<12>(p, 1).first().copied())
        .unwrap_or(IDENTITY);
    let Some(data) = m.ptr(g + 0x24) else { return };
    let n = m
        .u32(data + 0x90)
        .map_or(0, |n| n.cast_signed().clamp(0, SEGMENTS_MAX));
    let Some(base) = m.ptr(data + 0x94) else {
        return;
    };
    for i in 0..n.cast_unsigned() {
        let s = base + 0x6c * i;
        let indices = m.u32(s + 8).unwrap_or(0);
        if indices.cast_signed() <= 0 {
            continue;
        }
        let indices = indices.min(INDICES_MAX);
        let vertices = m.u32(s + 4).unwrap_or(0).min(VERTICES_MAX);
        let positions: Vec<[f32; 3]> = m
            .f32s::<3>(m.u32(s + 0xc).unwrap_or(0), vertices)
            .iter()
            .map(|p| to_render(&world, *p))
            .collect();
        let n = positions.len();
        let mut colours = m.f32s::<4>(m.u32(s + 0x14).unwrap_or(0), vertices);
        colours.resize(n, [0.0; 4]);
        let mut uvs = m.f32s::<2>(m.u32(s + 0x18).unwrap_or(0), vertices);
        uvs.resize(n, [0.0; 2]);
        out.push(Draw {
            place,
            key: found.key,
            geometry: g,
            flags,
            pooled: found.pooled,
            segment: u16::try_from(i).unwrap_or(u16::MAX),
            world,
            texture: m.ptr(s + 0x20).and_then(|p| m.c_str(p)).unwrap_or_default(),
            positions,
            colours,
            uvs,
            indices: m.u16s(m.u32(s + 0x1c).unwrap_or(0), indices),
        });
    }
}

/// The frame `DrawMostVFX` is about to draw for `scene`, in its order: the
/// list's entries `0..count` (and up to the last event's place, as the
/// events may extend it), each non-zero entry's segments; while magic
/// [`LAST_DRAWN_MAGIC`] plays, the entry of geometry [`LAST_DRAWN_KEY`]
/// last.
#[must_use]
pub fn read_frame(m: &Reader, scene: u32, update: u32) -> Frame {
    let mut frame = Frame {
        update,
        ..Frame::default()
    };
    let count = m.u32(scene + 0xf8cc).unwrap_or(0).cast_signed().max(0);
    let n_events = m.u32(scene + 0xfb54).unwrap_or(0).min(EVENTS_MAX);
    let mut end = count.cast_unsigned();
    if let Some(list) = m.ptr(scene + 0xfb5c) {
        for i in 0..n_events {
            let at = list + 16 * i;
            let ev = Event {
                kind: m.u32(at).unwrap_or(u32::MAX),
                at: m.u32(at + 4).unwrap_or(0),
                words: [m.u32(at + 8).unwrap_or(0), m.u32(at + 12).unwrap_or(0)],
            };
            end = end.max(ev.at);
            frame.events.push(ev);
        }
    }
    let end = end.min(LIST_MAX);
    let table = geometries(m);
    let held = m.u32(MAGIC_ID) == Some(LAST_DRAWN_MAGIC);
    let mut last = Vec::new();
    for place in 0..end {
        let Some(mi) = m.ptr(scene + 0x38cc + 4 * place) else {
            continue;
        };
        let Some(&found) = table.get(&mi) else {
            frame.unresolved += 1;
            continue;
        };
        if held && found.key == LAST_DRAWN_KEY && !found.pooled {
            entry_draws(m, place, mi, found, &mut last);
        } else {
            entry_draws(m, place, mi, found, &mut frame.draws);
        }
    }
    frame.draws.append(&mut last);
    frame
}

// ---- the encoding ----------------------------------------------------------

/// Appends little-endian values.
struct Out(Vec<u8>);

impl Out {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
}

/// Reads little-endian values.
struct In<'a>(&'a [u8]);

impl In<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self) -> Option<u64> {
        let lo = self.u32()?;
        let hi = self.u32()?;
        Some((u64::from(hi) << 32) | u64::from(lo))
    }
    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    fn f32s<const N: usize>(&mut self, n: usize) -> Option<Vec<[f32; N]>> {
        let mut v = Vec::with_capacity(n.min(self.0.len() / (4 * N)));
        for _ in 0..n {
            let mut a = [0.0; N];
            for x in &mut a {
                *x = self.f32()?;
            }
            v.push(a);
        }
        Some(v)
    }
}

impl Frame {
    /// The frame as bytes (docs/state-api.md, "Effect draws"): little
    /// endian; a header (`FXDR`, version, update, unresolved, event and
    /// draw counts), the events, then each draw with its arrays.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out(Vec::new());
        o.0.extend_from_slice(&MAGIC);
        o.u16(VERSION);
        o.u16(0);
        o.u32(self.update);
        o.u32(self.unresolved);
        o.u32(u32::try_from(self.events.len()).unwrap_or(u32::MAX));
        o.u32(u32::try_from(self.draws.len()).unwrap_or(u32::MAX));
        for e in &self.events {
            o.u32(e.kind);
            o.u32(e.at);
            o.u32(e.words[0]);
            o.u32(e.words[1]);
        }
        for d in &self.draws {
            o.u32(d.place);
            o.u64(d.key);
            o.u32(d.geometry);
            o.u32(d.flags);
            o.u16(d.segment);
            o.u8(u8::from(d.pooled));
            o.u8(0);
            for &w in &d.world {
                o.f32(w);
            }
            let name = d.texture.as_bytes();
            let len = name.len().min(usize::from(u16::MAX));
            o.u16(u16::try_from(len).unwrap_or(u16::MAX));
            o.0.extend_from_slice(&name[..len]);
            let n = d.positions.len();
            o.u32(u32::try_from(n).unwrap_or(u32::MAX));
            o.u32(u32::try_from(d.indices.len()).unwrap_or(u32::MAX));
            for p in &d.positions {
                p.iter().for_each(|&v| o.f32(v));
            }
            for k in 0..n {
                d.colours
                    .get(k)
                    .unwrap_or(&[0.0; 4])
                    .iter()
                    .for_each(|&v| o.f32(v));
            }
            for k in 0..n {
                d.uvs
                    .get(k)
                    .unwrap_or(&[0.0; 2])
                    .iter()
                    .for_each(|&v| o.f32(v));
            }
            for &i in &d.indices {
                o.u16(i);
            }
        }
        o.0
    }

    /// A frame from [`Frame::encode`]'s bytes; `None` when they are not one
    /// (another magic or version, cut short).
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut i = In(bytes);
        if i.take(4)? != MAGIC || i.u16()? != VERSION {
            return None;
        }
        i.u16()?;
        let update = i.u32()?;
        let unresolved = i.u32()?;
        let n_events = i.u32()? as usize;
        let n_draws = i.u32()? as usize;
        let mut events = Vec::with_capacity(n_events.min(EVENTS_MAX as usize));
        for _ in 0..n_events {
            events.push(Event {
                kind: i.u32()?,
                at: i.u32()?,
                words: [i.u32()?, i.u32()?],
            });
        }
        let mut draws = Vec::with_capacity(n_draws.min(LIST_MAX as usize));
        for _ in 0..n_draws {
            let place = i.u32()?;
            let key = i.u64()?;
            let geometry = i.u32()?;
            let flags = i.u32()?;
            let segment = i.u16()?;
            let pooled = i.u8()? != 0;
            i.u8()?;
            let mut world = [0.0; 12];
            for w in &mut world {
                *w = i.f32()?;
            }
            let len = usize::from(i.u16()?);
            let texture = String::from_utf8_lossy(i.take(len)?).into_owned();
            let n = i.u32()? as usize;
            let m = i.u32()? as usize;
            let positions = i.f32s::<3>(n)?;
            let colours = i.f32s::<4>(n)?;
            let uvs = i.f32s::<2>(n)?;
            let mut indices = Vec::with_capacity(m.min(i.0.len() / 2));
            for _ in 0..m {
                indices.push(i.u16()?);
            }
            draws.push(Draw {
                place,
                key,
                geometry,
                flags,
                pooled,
                segment,
                world,
                texture,
                positions,
                colours,
                uvs,
                indices,
            });
        }
        Some(Self {
            update,
            draws,
            events,
            unresolved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(place: u32) -> Draw {
        Draw {
            place,
            key: (0xd2_a318 << 32) | 6,
            geometry: 0x32c1_dbbc,
            flags: 0x8,
            pooled: place == 2,
            segment: 1,
            world: [0.5, 0.0, 0.0, 0.0, 0.0, 0.5, 0.0, 0.5, 309.7, 14.9, -476.9, 0.0],
            texture: "../../../FFX_Data/GameData/PS3Data/map/bsil/bsil00/fp/tex/D3D11/14336_19_0_0_128_128.dds.phyre".into(),
            positions: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
            colours: vec![[0.5, 0.875, 1.0, 0.125]; 3],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [0.5, 0.0]],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn a_frame_encodes_and_decodes_unchanged() {
        let f = Frame {
            update: 155,
            draws: vec![draw(19), draw(2), Draw::default()],
            events: vec![Event {
                kind: 2,
                at: 1,
                words: [3, 4],
            }],
            unresolved: 1,
        };
        let bytes = f.encode();
        assert_eq!(&bytes[..4], b"FXDR");
        assert_eq!(Frame::decode(&bytes), Some(f));
    }

    #[test]
    fn the_layout_is_the_documented_one() {
        let f = Frame {
            update: 7,
            draws: vec![draw(19)],
            ..Frame::default()
        };
        let b = f.encode();
        // header 24, draw head 4+8+4+4+2+1+1 = 24, world 48, name 2 + 96,
        // counts 8, arrays 3 * (12 + 16 + 8) + 3 * 2
        assert_eq!(b.len(), 24 + 24 + 48 + 2 + 96 + 8 + 3 * 36 + 6);
        assert_eq!(u32::from_le_bytes([b[8], b[9], b[10], b[11]]), 7);
        assert_eq!(u32::from_le_bytes([b[20], b[21], b[22], b[23]]), 1);
        // the key, low word first
        assert_eq!(&b[28..36], &[6, 0, 0, 0, 0x18, 0xa3, 0xd2, 0]);
        assert_eq!(u32::from_le_bytes([b[40], b[41], b[42], b[43]]), 0x8);
    }

    #[test]
    fn a_cut_or_foreign_blob_is_refused() {
        let b = Frame {
            draws: vec![draw(0)],
            ..Frame::default()
        }
        .encode();
        assert_eq!(Frame::decode(&b[..b.len() - 1]), None);
        let mut other = b.clone();
        other[4] = 2;
        assert_eq!(Frame::decode(&other), None);
        assert_eq!(Frame::decode(b"FXD"), None);
    }

    #[test]
    fn the_world_matrix_places_as_the_camera_layout() {
        // the field-to-render flip of Besaid's sea (X = -x, Y = -y, Z = z)
        // and a lift of 1.8 down
        let w = [
            0.0, -1.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.8, 0.0, 0.0,
        ];
        assert_eq!(to_render(&w, [10.0, 1.0, -20.0]), [-10.0, -2.8, -20.0]);
        assert_eq!(to_render(&IDENTITY, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn frames_are_kept_by_update_and_dropped_when_off() {
        let c = Capture::default();
        assert!(!c.enabled());
        c.set_enabled(true);
        for u in 0..6 {
            c.push(Frame {
                update: u,
                ..Frame::default()
            });
        }
        assert_eq!(c.latest().map(|f| f.update), Some(5));
        assert_eq!(c.frame(2).map(|f| f.update), Some(2));
        assert_eq!(c.frame(1), None);
        c.set_enabled(false);
        assert_eq!(c.latest(), None);
    }
}
