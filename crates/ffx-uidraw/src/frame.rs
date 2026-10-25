//! A capture's frames, draw by draw, with what each draw read: its
//! vertices and indices (dynamic rings as the frame's updates left them
//! at that draw), its textures' top levels, its states and its constant
//! windows. Read through `ffx-gfx`'s stream parser and `ffx-drawdiff`'s
//! object table, read only.

use std::collections::{HashMap, HashSet};

use ffx_drawdiff::Key;
use ffx_drawdiff::capture::Objects;
use ffx_gfx::{Cmd, Draw, Id, Msg, StateKind, TextureDesc};

use crate::index::content;

/// A texture as captured: description and the top level's texels (rows
/// packed), when the capture holds contents.
#[derive(Debug, Clone)]
pub struct Texture {
    pub desc: TextureDesc,
    pub top: Option<Vec<u8>>,
    /// A render or depth target (bind flags), or a texture a copy wrote.
    pub target: bool,
}

impl Texture {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.desc[0]
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.desc[1]
    }

    #[must_use]
    pub fn format(&self) -> u32 {
        self.desc[4]
    }

    /// [`content`] of the top level.
    #[must_use]
    pub fn content(&self) -> Option<Key> {
        self.top.as_deref().map(content)
    }
}

/// One draw with its inputs resolved.
#[derive(Debug, Clone)]
pub struct DrawRec {
    /// Index among the frame's draws, clears and copies (`ffx-drawdiff`'s
    /// item numbers).
    pub item: usize,
    /// Index among the frame's draws only.
    pub draw_no: usize,
    pub draw: Draw,
    pub vs: Key,
    pub ps: Key,
    /// Into the scene targets (two render targets and a depth target).
    pub scene: bool,
    /// Render target 0's resource and its role (n-th target of the frame
    /// in order of first use).
    pub target: Option<(Id, u32)>,
    /// Per vertex slot: (slot, stride, the bytes of the vertices the draw
    /// reads, from vertex [`DrawRec::first_vertex`] on, as the frame's
    /// updates left them, and whether the buffer is a dynamic ring).
    pub vertex: Vec<(u8, u32, Vec<u8>, bool)>,
    /// The first vertex the windows hold (smallest index + base, or start).
    pub first_vertex: u64,
    /// The indices read (indexed draws).
    pub indices: Vec<u32>,
    /// Whether the index buffer is a ring.
    pub index_ring: bool,
    /// Pixel shader textures: (slot, resource id).
    pub ps_textures: Vec<(u8, Id)>,
}

/// A capture's frames and objects.
pub struct Capture {
    pub objects: Objects,
    pub textures: HashMap<Id, Texture>,
    /// Shader bytecode by id.
    pub bytecode: HashMap<Id, Vec<u8>>,
    pub frames: Vec<Vec<DrawRec>>,
    /// The `World` message's scene name.
    pub scene: String,
}

/// Bytes per row of a format at a width (block formats by their blocks).
#[must_use]
pub fn row_bytes(format: u32, width: u32) -> Option<usize> {
    ffx_drawdiff::key::row_bytes(format, width)
}

fn write_at(buf: &mut Vec<u8>, offset: u32, data: &[u8]) {
    let at = offset as usize;
    if buf.len() < at + data.len() {
        buf.resize(at + data.len(), 0);
    }
    buf[at..at + data.len()].copy_from_slice(data);
}

fn indices(data: &[u8], format: u32, offset: u32, start: u32, count: u32) -> Vec<u32> {
    let size = if format == 42 { 4 } else { 2 };
    let from = offset as usize + start as usize * size;
    data.get(from..from + count as usize * size)
        .map(|b| {
            b.chunks_exact(size)
                .map(|c| {
                    if size == 4 {
                        u32::from_le_bytes([c[0], c[1], c[2], c[3]])
                    } else {
                        u32::from(u16::from_le_bytes([c[0], c[1]]))
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

impl Capture {
    /// Reads a `capture.ffxgfx` or a folder holding one.
    ///
    /// # Errors
    /// When the file cannot be read or is not a capture.
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let path = if path.is_dir() {
            path.join("capture.ffxgfx")
        } else {
            path.to_owned()
        };
        let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let body = bytes
            .strip_prefix(ffx_gfx::CAPTURE_MAGIC)
            .ok_or_else(|| format!("{}: not a capture", path.display()))?;
        let msgs = ffx_gfx::read_all(body).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self::from_msgs(&msgs))
    }

    /// From decoded messages, in stream order.
    #[must_use]
    pub fn from_msgs(msgs: &[Msg]) -> Self {
        let mut c = Self {
            objects: Objects::default(),
            textures: HashMap::new(),
            bytecode: HashMap::new(),
            frames: Vec::new(),
            scene: String::new(),
        };
        for m in msgs {
            match m {
                Msg::World { scene, .. } => c.scene.clone_from(scene),
                Msg::Frame(f) => {
                    let draws = c.walk(&f.cmds);
                    c.frames.push(draws);
                }
                Msg::Texture {
                    id,
                    desc,
                    subresources,
                } => {
                    let top = row_bytes(desc[4], desc[0]).and_then(|row| {
                        let sub = subresources.first()?;
                        (sub.row_pitch != 0)
                            .then(|| pack_rows(&sub.data, sub.row_pitch as usize, row))
                    });
                    c.textures.insert(
                        *id,
                        Texture {
                            desc: *desc,
                            top,
                            target: ffx_drawdiff::key::is_target(desc),
                        },
                    );
                    c.objects.apply(m);
                }
                Msg::Shader { id, bytecode, .. } => {
                    c.bytecode.insert(*id, bytecode.clone());
                    c.objects.apply(m);
                }
                m => c.objects.apply(m),
            }
        }
        c
    }

    fn resource_of(&self, view: Id) -> Id {
        self.objects.views.get(&view).map_or(view, |v| v.resource)
    }

    #[allow(clippy::too_many_lines)] // one pass over the commands, a case each
    fn walk(&self, cmds: &[Cmd]) -> Vec<DrawRec> {
        let mut contents: HashMap<Id, Vec<u8>> = HashMap::new();
        let mut roles: HashMap<Id, u32> = HashMap::new();
        let role = |res: Id, roles: &mut HashMap<Id, u32>| {
            let n = u32::try_from(roles.len()).unwrap_or(u32::MAX);
            *roles.entry(res).or_insert(n)
        };
        let mut out = Vec::new();
        let mut item = 0;
        for c in cmds {
            match c {
                Cmd::UpdateBuffer { id, offset, data } => {
                    let buf = contents.entry(*id).or_insert_with(|| {
                        self.objects
                            .buffers
                            .get(id)
                            .map(|b| b.data.clone())
                            .unwrap_or_default()
                    });
                    write_at(buf, *offset, data);
                }
                Cmd::ClearTarget { view, .. } | Cmd::ClearDepth { view, .. } => {
                    role(self.resource_of(*view), &mut roles);
                    item += 1;
                }
                Cmd::CopyTexture { dst, src } => {
                    role(*dst, &mut roles);
                    role(*src, &mut roles);
                    item += 1;
                }
                Cmd::Draw(d) => {
                    let data = |id: Id| -> (&[u8], bool) {
                        let ring = self
                            .objects
                            .buffers
                            .get(&id)
                            .is_some_and(|b| ffx_drawdiff::key::is_ring(&b.desc));
                        let bytes = contents
                            .get(&id)
                            .map(Vec::as_slice)
                            .or_else(|| self.objects.buffers.get(&id).map(|b| b.data.as_slice()));
                        (bytes.unwrap_or_default(), ring)
                    };
                    let (ib, ib_format, ib_offset) = d.index;
                    let (index_bytes, index_ring) =
                        if ib == 0 { (&[][..], false) } else { data(ib) };
                    let indices = if matches!(d.kind, ffx_gfx::DrawKind::Indexed) {
                        indices(index_bytes, ib_format, ib_offset, d.start, d.count)
                    } else {
                        Vec::new()
                    };
                    // The vertices read: indices + base, or start..start+count.
                    let (first, last) = match (indices.iter().min(), indices.iter().max()) {
                        (Some(&lo), Some(&hi)) => (
                            i64::from(lo) + i64::from(d.base),
                            i64::from(hi) + i64::from(d.base),
                        ),
                        _ => (
                            i64::from(d.start),
                            i64::from(d.start) + i64::from(d.count) - 1,
                        ),
                    };
                    let first = u64::try_from(first).unwrap_or(0);
                    let last = u64::try_from(last).unwrap_or(0).max(first);
                    let vertex = d
                        .vertex
                        .iter()
                        .map(|&(slot, id, stride, offset)| {
                            let (bytes, ring) = data(id);
                            let s = u64::from(stride);
                            let from = usize::try_from(u64::from(offset) + first * s)
                                .unwrap_or(usize::MAX);
                            let to = usize::try_from(u64::from(offset) + (last + 1) * s.max(1))
                                .unwrap_or(usize::MAX);
                            let window = bytes
                                .get(from.min(bytes.len())..to.min(bytes.len()))
                                .unwrap_or_default()
                                .to_vec();
                            (slot, stride, window, ring)
                        })
                        .collect();
                    let target = d.rtv.iter().find(|r| r.0 == 0 && r.1 != 0).map(|r| {
                        let res = self.resource_of(r.1);
                        (res, role(res, &mut roles))
                    });
                    for &(_, v) in &d.ps_srv {
                        role(self.resource_of(v), &mut roles);
                    }
                    let bound = |s: u8| d.rtv.iter().any(|&(slot, v)| slot == s && v != 0);
                    out.push(DrawRec {
                        item,
                        draw_no: out.len(),
                        vs: self.objects.shader_key(d.vs),
                        ps: self.objects.shader_key(d.ps),
                        scene: bound(0) && bound(1) && d.dsv != 0,
                        target,
                        vertex,
                        first_vertex: first,
                        indices,
                        index_ring,
                        ps_textures: d
                            .ps_srv
                            .iter()
                            .map(|&(s, v)| (s, self.resource_of(v)))
                            .collect(),
                        draw: (**d).clone(),
                    });
                    item += 1;
                }
            }
        }
        out
    }

    /// A state object's description bytes.
    #[must_use]
    pub fn state(&self, id: Id, kind: StateKind) -> Option<&[u8]> {
        self.objects
            .states
            .get(&id)
            .filter(|s| s.0 == kind)
            .map(|s| s.1.as_slice())
    }

    /// Resources ever bound as a render target in a frame.
    #[must_use]
    pub fn targets(frame: &[DrawRec]) -> HashSet<Id> {
        frame.iter().filter_map(|d| d.target.map(|t| t.0)).collect()
    }
}

/// Rows of `row` bytes out of a buffer with a row pitch.
#[must_use]
pub fn pack_rows(data: &[u8], pitch: usize, row: usize) -> Vec<u8> {
    if pitch == row || pitch == 0 {
        return data.to_vec();
    }
    data.chunks(pitch)
        .flat_map(|r| &r[..row.min(r.len())])
        .copied()
        .collect()
}
