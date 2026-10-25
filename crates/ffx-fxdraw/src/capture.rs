//! A capture's draws with the bytes they read: the objects through
//! `ffx-drawdiff`'s [`Objects`] (content keys, shader reflection), the
//! frames' commands as `ffx-gfx` decodes them, and every dynamic buffer
//! replayed through the frame's `UpdateBuffer`s so each draw sees its own
//! version (`docs/renderer.md`, "Reference captures": hash captures send
//! the ring bytes a draw reads right before it).

use std::collections::HashMap;
use std::path::Path;

use ffx_drawdiff::capture::Objects;
use ffx_drawdiff::key::{self, Key};
use ffx_gfx::{Cmd, Draw, DrawKind, Id, Msg};

/// A capture could not be read.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// One vertex slot of a draw.
#[derive(Debug, Clone)]
pub struct Slot {
    pub slot: u8,
    pub buffer: Id,
    pub stride: u32,
    pub offset: u32,
    /// `D3D11_USAGE_DYNAMIC` (a ring).
    pub ring: bool,
    /// Per-instance data (the layout says so).
    pub per_instance: bool,
    /// The records the draw reads, from the binding's offset: the first
    /// one's index and their bytes (rings only; static buffers are only
    /// hashed in hash captures).
    pub first: u32,
    pub bytes: Vec<u8>,
}

/// One draw as the census sees it.
#[derive(Debug, Clone)]
pub struct DrawRec {
    /// Index among the frame's draws, from 0.
    pub n: usize,
    /// Index in the frame's command list.
    pub cmd: usize,
    pub draw: Draw,
    pub vs: Key,
    pub ps: Key,
    pub slots: Vec<Slot>,
    /// The indices read (indexed draws), before `base`.
    pub indices: Option<Vec<u32>>,
    /// The index buffer is a ring.
    pub ring_indices: bool,
    /// Render target resources (slot, resource) and the depth target's.
    pub targets: Vec<(u8, Id)>,
    pub depth_target: Id,
}

impl DrawRec {
    /// Whether any vertex slot or the index buffer is a ring.
    #[must_use]
    pub fn uses_rings(&self) -> bool {
        self.ring_indices || self.slots.iter().any(|s| s.ring)
    }

    /// Two render targets and a depth target: the scene targets
    /// (`docs/formats/scene.md`; `gfx/live.rs`).
    #[must_use]
    pub fn is_scene(&self) -> bool {
        let bound = |s: u8| self.targets.iter().any(|&(slot, r)| slot == s && r != 0);
        bound(0) && bound(1) && self.depth_target != 0
    }
}

/// One captured frame.
#[derive(Debug, Clone)]
pub struct FrameRec {
    pub number: u32,
    pub width: u32,
    pub height: u32,
    pub draws: Vec<DrawRec>,
    /// Clears and copies as (command index, description).
    pub others: Vec<(usize, String)>,
}

/// A capture: the objects as the stream left them, the scene name of the
/// `World` message and the frames.
pub struct Capture {
    pub objects: Objects,
    pub scene: String,
    pub frames: Vec<FrameRec>,
}

impl Capture {
    /// Reads a `capture.ffxgfx` (or the folder holding one).
    ///
    /// # Errors
    /// When the file cannot be read or decoded.
    pub fn read(path: &Path) -> Result<Self, Error> {
        let path = if path.is_dir() {
            path.join("capture.ffxgfx")
        } else {
            path.to_owned()
        };
        let bytes = std::fs::read(&path).map_err(|e| Error(format!("{}: {e}", path.display())))?;
        Self::parse(&bytes).map_err(|e| Error(format!("{}: {e}", path.display())))
    }

    /// # Errors
    /// When the bytes are not a capture.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let body = bytes
            .strip_prefix(ffx_gfx::CAPTURE_MAGIC)
            .ok_or_else(|| Error("not a capture (no FFXGFX01)".into()))?;
        let msgs = ffx_gfx::read_all(body).map_err(|e| Error(e.to_string()))?;
        Ok(Self::from_msgs(&msgs))
    }

    /// From decoded messages, in stream order.
    #[must_use]
    pub fn from_msgs(msgs: &[Msg]) -> Self {
        let mut c = Self {
            objects: Objects::default(),
            scene: String::new(),
            frames: Vec::new(),
        };
        for m in msgs {
            match m {
                Msg::World { scene, .. } => c.scene.clone_from(scene),
                Msg::Frame(f) => {
                    let frame = frame(&c.objects, f);
                    c.frames.push(frame);
                }
                m => c.objects.apply(m),
            }
        }
        c
    }

    /// The resource behind a view (the id itself when it is no view).
    #[must_use]
    pub fn resource_of(&self, view: Id) -> Id {
        self.objects.views.get(&view).map_or(view, |v| v.resource)
    }

    /// A short description of a texture resource: `<w>x<h> f<format>`.
    #[must_use]
    pub fn texture_label(&self, res: Id) -> String {
        self.objects.textures.get(&res).map_or_else(
            || format!("{res:#x}"),
            |t| format!("{}x{} f{}", t.desc[0], t.desc[1], t.desc[4]),
        )
    }
}

fn is_ring(objects: &Objects, id: Id) -> bool {
    objects
        .buffers
        .get(&id)
        .is_some_and(|b| key::is_ring(&b.desc))
}

fn frame(objects: &Objects, f: &ffx_gfx::Frame) -> FrameRec {
    let mut contents: HashMap<Id, Vec<u8>> = HashMap::new();
    let mut draws = Vec::new();
    let mut others = Vec::new();
    for (i, c) in f.cmds.iter().enumerate() {
        match c {
            Cmd::UpdateBuffer { id, offset, data } => {
                let buf = contents.entry(*id).or_insert_with(|| {
                    objects
                        .buffers
                        .get(id)
                        .map(|b| b.data.clone())
                        .unwrap_or_default()
                });
                let at = *offset as usize;
                if buf.len() < at + data.len() {
                    buf.resize(at + data.len(), 0);
                }
                buf[at..at + data.len()].copy_from_slice(data);
            }
            Cmd::Draw(d) => {
                let n = draws.len();
                draws.push(draw(objects, &contents, n, i, d));
            }
            Cmd::ClearTarget { view, color } => {
                others.push((i, format!("clear {view:#x} {color:?}")));
            }
            Cmd::ClearDepth { view, depth, .. } => {
                others.push((i, format!("clear-depth {view:#x} {depth}")));
            }
            Cmd::CopyTexture { dst, src } => {
                others.push((i, format!("copy {src:#x} -> {dst:#x}")));
            }
        }
    }
    FrameRec {
        number: f.number,
        width: f.width,
        height: f.height,
        draws,
        others,
    }
}

fn read_indices(data: &[u8], format: u32, offset: u32, start: u32, count: u32) -> Option<Vec<u32>> {
    let size = if format == 42 { 4 } else { 2 };
    let from = offset as usize + start as usize * size;
    let bytes = data.get(from..from + count as usize * size)?;
    Some(
        bytes
            .chunks_exact(size)
            .map(|c| {
                if size == 4 {
                    u32::from_le_bytes([c[0], c[1], c[2], c[3]])
                } else {
                    u32::from(u16::from_le_bytes([c[0], c[1]]))
                }
            })
            .collect(),
    )
}

fn draw(
    objects: &Objects,
    contents: &HashMap<Id, Vec<u8>>,
    n: usize,
    cmd: usize,
    d: &Draw,
) -> DrawRec {
    let data_of = |id: Id| -> Option<&[u8]> {
        contents
            .get(&id)
            .map(Vec::as_slice)
            .or_else(|| objects.buffers.get(&id).map(|b| b.data.as_slice()))
    };
    let indexed = matches!(d.kind, DrawKind::Indexed | DrawKind::IndexedInstanced);
    let (ib, format, ib_offset) = d.index;
    let ring_indices = indexed && is_ring(objects, ib);
    let indices = if indexed {
        data_of(ib).and_then(|data| read_indices(data, format, ib_offset, d.start, d.count))
    } else {
        None
    };
    let per_instance_slots: Vec<u32> = objects
        .layouts
        .get(&d.layout)
        .map(|(_, els)| {
            els.iter()
                .filter(|e| e.per_instance)
                .map(|e| e.slot)
                .collect()
        })
        .unwrap_or_default();
    // The vertex records a per-vertex slot reads.
    let (lo, hi) = match (&indices, indexed) {
        (Some(ix), true) if !ix.is_empty() => {
            let min = ix.iter().copied().min().unwrap_or(0);
            let max = ix.iter().copied().max().unwrap_or(0);
            let b = i64::from(d.base);
            (
                u32::try_from(i64::from(min) + b).unwrap_or(0),
                u32::try_from(i64::from(max) + b).unwrap_or(0),
            )
        }
        (_, true) => (0, 0),
        (_, false) => (d.start, d.start + d.count.max(1) - 1),
    };
    let slots = d
        .vertex
        .iter()
        .map(|&(slot, buffer, stride, offset)| {
            let ring = is_ring(objects, buffer);
            let per_instance = per_instance_slots.contains(&u32::from(slot));
            let (first, last) = if per_instance {
                (d.first_instance, d.first_instance + d.instances.max(1) - 1)
            } else {
                (lo, hi)
            };
            let bytes = if ring && stride > 0 {
                let from = offset as usize + first as usize * stride as usize;
                let to = offset as usize + (last as usize + 1) * stride as usize;
                data_of(buffer)
                    .and_then(|b| b.get(from..to))
                    .map(<[u8]>::to_vec)
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Slot {
                slot,
                buffer,
                stride,
                offset,
                ring,
                per_instance,
                first,
                bytes,
            }
        })
        .collect();
    let resource = |v: Id| objects.views.get(&v).map_or(v, |x| x.resource);
    DrawRec {
        n,
        cmd,
        vs: objects.shader_key(d.vs),
        ps: objects.shader_key(d.ps),
        slots,
        indices,
        ring_indices,
        targets: d
            .rtv
            .iter()
            .filter(|r| r.1 != 0)
            .map(|&(s, v)| (s, resource(v)))
            .collect(),
        depth_target: if d.dsv == 0 { 0 } else { resource(d.dsv) },
        draw: d.clone(),
    }
}
