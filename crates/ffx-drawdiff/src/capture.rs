//! A capture file read into keyed items: the objects it declares (kept by
//! id, as the stream says: a new object at a reused id replaces the old
//! one) and, per frame, one [`Item`] per draw, clear and copy.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use ffx_gfx::{BufferDesc, Cmd, Draw, DrawKind, Element, Frame, Id, Msg, StateKind, TextureDesc};

use crate::cb::{self, Constants};
use crate::key::{self, Fnv, Key};
use crate::state::{Blend, Depth, Raster, Sampler};
use crate::{Class, Field, Item, ItemKind, Value};

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct BufferObj {
    pub desc: BufferDesc,
    pub data: Vec<u8>,
    /// [`key::buffer`] of the contents as declared.
    pub key: Key,
}

#[derive(Debug, Clone)]
pub struct TextureObj {
    pub desc: TextureDesc,
    /// [`key::texture`].
    pub key: Key,
}

#[derive(Debug, Clone)]
pub struct ViewObj {
    pub kind: ffx_gfx::ViewKind,
    pub resource: Id,
    pub desc: Option<[u32; 6]>,
}

#[derive(Debug, Clone)]
pub struct ShaderObj {
    pub kind: u8,
    /// [`key::shader`].
    pub key: Key,
    pub layout: Arc<cb::Layout>,
}

/// The objects a stream has declared so far, by id.
#[derive(Debug, Clone, Default)]
pub struct Objects {
    pub buffers: HashMap<Id, BufferObj>,
    pub textures: HashMap<Id, TextureObj>,
    pub views: HashMap<Id, ViewObj>,
    pub shaders: HashMap<Id, ShaderObj>,
    pub layouts: HashMap<Id, (Key, Vec<Element>)>,
    pub states: HashMap<Id, (StateKind, Vec<u8>)>,
    /// Reflection per shader key, shared by every id with that bytecode.
    reflections: HashMap<Key, Arc<cb::Layout>>,
}

impl Objects {
    fn forget(&mut self, id: Id) {
        self.buffers.remove(&id);
        self.textures.remove(&id);
        self.views.remove(&id);
        self.shaders.remove(&id);
        self.layouts.remove(&id);
        self.states.remove(&id);
    }

    /// Takes in one object message (frames and other messages are left
    /// alone).
    pub fn apply(&mut self, msg: &Msg) {
        match msg {
            Msg::Buffer { id, desc, data } => {
                self.forget(*id);
                self.buffers.insert(
                    *id,
                    BufferObj {
                        desc: *desc,
                        data: data.clone(),
                        key: key::buffer(desc, data),
                    },
                );
            }
            Msg::Texture {
                id,
                desc,
                subresources,
            } => {
                self.forget(*id);
                self.textures.insert(
                    *id,
                    TextureObj {
                        desc: *desc,
                        key: key::texture(desc, subresources),
                    },
                );
            }
            Msg::View {
                id,
                kind,
                resource,
                desc,
            } => {
                self.forget(*id);
                self.views.insert(
                    *id,
                    ViewObj {
                        kind: *kind,
                        resource: *resource,
                        desc: *desc,
                    },
                );
            }
            Msg::Shader { id, kind, bytecode } => {
                self.forget(*id);
                let k = key::shader(bytecode);
                let layout = self
                    .reflections
                    .entry(k)
                    .or_insert_with(|| Arc::new(cb::Layout::of(bytecode)))
                    .clone();
                self.shaders.insert(
                    *id,
                    ShaderObj {
                        kind: *kind,
                        key: k,
                        layout,
                    },
                );
            }
            Msg::Layout { id, elements } => {
                self.forget(*id);
                self.layouts
                    .insert(*id, (key::layout(elements), elements.clone()));
            }
            Msg::State { id, kind, desc } => {
                self.forget(*id);
                self.states.insert(*id, (*kind, desc.clone()));
            }
            Msg::UpdateBuffer { id, offset, data } => {
                if let Some(b) = self.buffers.get_mut(id) {
                    write_at(&mut b.data, *offset, data);
                    b.key = key::buffer(&b.desc, &b.data);
                }
            }
            Msg::Release { id } => self.forget(*id),
            _ => {}
        }
    }

    /// The content key of a shader id (0 when unknown or null).
    #[must_use]
    pub fn shader_key(&self, id: Id) -> Key {
        self.shaders.get(&id).map_or(0, |s| s.key)
    }

    /// The content key of a texture id, when it is known and not a target.
    #[must_use]
    pub fn texture_key(&self, id: Id) -> Option<Key> {
        self.textures
            .get(&id)
            .filter(|t| !key::is_target(&t.desc))
            .map(|t| t.key)
    }

    /// The content key of a static buffer id.
    #[must_use]
    pub fn buffer_key(&self, id: Id) -> Option<Key> {
        self.buffers
            .get(&id)
            .filter(|b| !key::is_ring(&b.desc))
            .map(|b| b.key)
    }

    #[must_use]
    pub fn layout_key(&self, id: Id) -> Option<Key> {
        self.layouts.get(&id).map(|l| l.0)
    }

    fn state(&self, id: Id) -> Option<&[u8]> {
        self.states.get(&id).map(|s| s.1.as_slice())
    }
}

fn write_at(buf: &mut Vec<u8>, offset: u32, data: &[u8]) {
    let at = offset as usize;
    if buf.len() < at + data.len() {
        buf.resize(at + data.len(), 0);
    }
    buf[at..at + data.len()].copy_from_slice(data);
}

/// One frame as items.
#[derive(Debug, Clone)]
pub struct FrameItems {
    pub number: u32,
    /// The scene of the `World` message before it (`cdsp0000`), or empty.
    pub scene: String,
    pub items: Vec<Item>,
}

/// A capture: its frames as items, and the objects as the stream left
/// them.
#[derive(Debug, Clone, Default)]
pub struct Capture {
    pub frames: Vec<FrameItems>,
    pub objects: Objects,
}

impl Capture {
    /// Reads a `capture.ffxgfx` (or a folder holding one).
    ///
    /// # Errors
    /// When the file cannot be read or is not a capture.
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
        let mut c = Self::default();
        let mut scene = String::new();
        for m in msgs {
            match m {
                Msg::World { scene: s, .. } => scene.clone_from(s),
                Msg::Frame(f) => {
                    let items = frame_items(&c.objects, f);
                    c.frames.push(FrameItems {
                        number: f.number,
                        scene: scene.clone(),
                        items,
                    });
                }
                m => c.objects.apply(m),
            }
        }
        c
    }
}

/// Target roles of one frame: resources in order of first use.
#[derive(Default)]
struct Roles {
    order: HashMap<Id, u32>,
}

/// Builds a frame's items against the objects as they are.
struct Builder<'a> {
    objects: &'a Objects,
    roles: Roles,
    /// Dynamic buffer contents as the frame's updates leave them.
    contents: HashMap<Id, Vec<u8>>,
    /// Resources of the scene targets.
    scene_targets: HashSet<Id>,
}

fn is_scene_draw(d: &Draw) -> bool {
    let bound = |s: u8| d.rtv.iter().any(|&(slot, v)| slot == s && v != 0);
    bound(0) && bound(1) && d.dsv != 0
}

/// The frame's items.
#[must_use]
pub fn frame_items(objects: &Objects, frame: &Frame) -> Vec<Item> {
    let mut b = Builder {
        objects,
        roles: Roles::default(),
        contents: HashMap::new(),
        scene_targets: HashSet::new(),
    };
    for c in &frame.cmds {
        if let Cmd::Draw(d) = c
            && is_scene_draw(d)
        {
            for v in d.rtv.iter().map(|r| r.1).chain([d.dsv]) {
                b.scene_targets.insert(b.resource_of(v));
            }
        }
    }
    let mut items = Vec::new();
    for (i, c) in frame.cmds.iter().enumerate() {
        match c {
            Cmd::UpdateBuffer { id, offset, data } => {
                let buf = b.contents.entry(*id).or_insert_with(|| {
                    objects
                        .buffers
                        .get(id)
                        .map(|x| x.data.clone())
                        .unwrap_or_default()
                });
                write_at(buf, *offset, data);
            }
            Cmd::Draw(d) => items.push(b.draw(i, d)),
            Cmd::ClearTarget { view, color } => {
                let mut fields = vec![b.target_field("target", *view)];
                for (n, c) in ["r", "g", "b", "a"].iter().zip(color) {
                    fields.push(field(
                        &format!("clear.color.{n}"),
                        Class::State,
                        Value::F(c.to_bits()),
                    ));
                }
                items.push(b.simple(i, ItemKind::ClearTarget, *view, fields));
            }
            Cmd::ClearDepth {
                view,
                flags,
                depth,
                stencil,
            } => {
                let fields = vec![
                    b.target_field("target", *view),
                    field("clear.flags", Class::State, Value::U(*flags)),
                    field("clear.depth", Class::State, Value::F(depth.to_bits())),
                    field("clear.stencil", Class::State, Value::U(u32::from(*stencil))),
                ];
                items.push(b.simple(i, ItemKind::ClearDepth, *view, fields));
            }
            Cmd::CopyTexture { dst, src } => {
                let fields = vec![
                    b.resource_field("copy.dst", *dst),
                    b.resource_field("copy.src", *src),
                ];
                let mut item = b.simple(i, ItemKind::Copy, 0, fields);
                item.scene = b.scene_targets.contains(dst) || b.scene_targets.contains(src);
                item.match_key = Fnv::new().u32(3).u64(b.resource_key(*dst).0).finish();
                item.fingerprint = fingerprint(item.match_key, &item.fields, &[]);
                items.push(item);
            }
        }
    }
    items
}

/// What a ring contributes to a match key.
const RING_SLOT: Key = 0x7269_6e67_736c_6f74;

fn field(name: &str, class: Class, value: Value) -> Field {
    Field {
        name: name.to_owned(),
        class,
        value,
    }
}

fn fingerprint(match_key: Key, fields: &[Field], constants: &[Constants]) -> Key {
    let mut h = Fnv::new();
    h.u64(match_key);
    for f in fields {
        h.str(&f.name);
        f.value.hash_into(&mut h);
    }
    for c in constants {
        h.str(c.stage).u32(u32::from(c.slot));
        for v in &c.vars {
            h.str(&v.name).u32(v.offset);
            for (w, _) in &v.words {
                h.u32(w.map_or(0x1_0000, |_| 0)).u32(w.unwrap_or(0));
            }
        }
    }
    h.finish()
}

fn indices(data: &[u8], format: u32, offset: u32, start: u32, count: u32) -> Option<Vec<u32>> {
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

impl Builder<'_> {
    fn resource_of(&self, view: Id) -> Id {
        self.objects.views.get(&view).map_or(view, |v| v.resource)
    }

    /// A resource's key and description: targets by role, textures and
    /// static buffers by content.
    fn resource_key(&mut self, res: Id) -> (Key, String) {
        if res == 0 {
            return (0, "none".into());
        }
        let o = self.objects;
        if let Some(t) = o.textures.get(&res) {
            let d = &t.desc;
            if key::is_target(d) {
                let n = self.role(res);
                return (
                    key::role(n),
                    format!("target{n} {}x{} f{}", d[0], d[1], d[4]),
                );
            }
            return (
                t.key,
                format!("tex {:016x} {}x{} f{} m{}", t.key, d[0], d[1], d[4], d[2]),
            );
        }
        if let Some(b) = o.buffers.get(&res) {
            return (b.key, format!("buf {:016x} {} B", b.key, b.desc[0]));
        }
        // Not declared (the swap chain's back buffer): a target by role.
        let n = self.role(res);
        (key::role(n), format!("target{n} (undeclared)"))
    }

    fn role(&mut self, res: Id) -> u32 {
        let next = u32::try_from(self.roles.order.len()).unwrap_or(u32::MAX);
        *self.roles.order.entry(res).or_insert(next)
    }

    /// A view's key: its resource's and its description.
    fn view_value(&mut self, view: Id) -> Value {
        let (res, desc) = self
            .objects
            .views
            .get(&view)
            .map_or((view, None), |v| (v.resource, v.desc));
        let (k, label) = self.resource_key(res);
        let mut h = Fnv::new();
        h.u64(k);
        if let Some(d) = desc {
            for w in d {
                h.u32(w);
            }
        }
        let label = match desc {
            Some(d) => format!("{label} view {d:?}"),
            None => label,
        };
        Value::Key(h.finish(), label)
    }

    fn target_field(&mut self, name: &str, view: Id) -> Field {
        field(name, Class::Resource, self.view_value(view))
    }

    fn resource_field(&mut self, name: &str, res: Id) -> Field {
        let (k, label) = self.resource_key(res);
        field(name, Class::Resource, Value::Key(k, label))
    }

    fn simple(&mut self, cmd: usize, kind: ItemKind, view: Id, mut fields: Vec<Field>) -> Item {
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        let target = fields
            .iter()
            .find(|f| f.name == "target")
            .map_or(0, |f| match f.value {
                Value::Key(k, _) => k,
                _ => 0,
            });
        let match_key = Fnv::new().u32(kind as u32).u64(target).finish();
        let label = fields
            .iter()
            .find(|f| f.name == "target" || f.name == "copy.dst")
            .map_or_else(
                || kind.name().to_owned(),
                |f| format!("{} {}", kind.name(), f.value),
            );
        Item {
            cmd,
            kind,
            vs: 0,
            ps: 0,
            scene: view != 0 && self.scene_targets.contains(&self.resource_of(view)),
            match_key,
            fingerprint: fingerprint(match_key, &fields, &[]),
            fields,
            constants: Vec::new(),
            label,
        }
    }

    fn bytes(&self, id: Id) -> Option<&[u8]> {
        self.contents
            .get(&id)
            .map(Vec::as_slice)
            .or_else(|| self.objects.buffers.get(&id).map(|b| b.data.as_slice()))
    }

    fn ring(&self, id: Id) -> bool {
        self.objects
            .buffers
            .get(&id)
            .is_some_and(|b| key::is_ring(&b.desc))
    }

    #[allow(clippy::too_many_lines)] // one draw: every field it has bound
    fn draw(&mut self, cmd: usize, d: &Draw) -> Item {
        let o = self.objects;
        let indexed = matches!(d.kind, DrawKind::Indexed | DrawKind::IndexedInstanced);
        let instanced = matches!(d.kind, DrawKind::IndexedInstanced | DrawKind::Instanced);
        let layout = o.layouts.get(&d.layout);
        let per_instance = |slot: u8| -> Option<u32> {
            layout?
                .1
                .iter()
                .find(|e| e.slot == u32::from(slot) && e.per_instance)
                .map(|e| e.step)
        };
        let vertex: Vec<_> = d.vertex.iter().filter(|v| v.1 != 0).collect();
        let vertex_rings = vertex
            .iter()
            .filter(|v| per_instance(v.0).is_none())
            .all(|v| self.ring(v.1))
            && vertex.iter().any(|v| per_instance(v.0).is_none());
        let instance_slots: Vec<_> = vertex
            .iter()
            .filter(|v| per_instance(v.0).is_some())
            .collect();
        let instance_rings =
            !instance_slots.is_empty() && instance_slots.iter().all(|v| self.ring(v.1));
        let (ib, ifmt, ioff) = d.index;
        let ib_ring = indexed && self.ring(ib);
        // The indices the draw reads, when a ring is involved.
        let any_vertex_ring = vertex
            .iter()
            .any(|v| per_instance(v.0).is_none() && self.ring(v.1));
        let idx = (indexed && (ib_ring || any_vertex_ring))
            .then(|| {
                self.bytes(ib)
                    .and_then(|data| indices(data, ifmt, ioff, d.start, d.count))
            })
            .flatten();
        let (lo, hi) = idx.as_ref().map_or((0, 0), |v| {
            (
                v.iter().copied().min().unwrap_or(0),
                v.iter().copied().max().unwrap_or(0),
            )
        });
        let mut fields = Vec::new();
        let st = |fields: &mut Vec<Field>, name: &str, v: Value| {
            fields.push(field(name, Class::State, v));
        };
        st(&mut fields, "draw.kind", Value::U(d.kind as u32));
        st(&mut fields, "draw.count", Value::U(d.count));
        if instanced {
            st(&mut fields, "draw.instances", Value::U(d.instances));
            if !instance_rings {
                st(
                    &mut fields,
                    "draw.first_instance",
                    Value::U(d.first_instance),
                );
            }
        }
        // start: an offset into a ring is not compared (the bytes are)
        let start_is_ring = if indexed { ib_ring } else { vertex_rings };
        let start = if start_is_ring { 0 } else { d.start };
        if !start_is_ring {
            st(&mut fields, "draw.start", Value::U(d.start));
        }
        if indexed && !vertex_rings {
            st(&mut fields, "draw.base", Value::I(d.base));
        }
        st(&mut fields, "topology", Value::U(d.topology));
        let layout_value = layout.map_or(Value::Absent, |(k, els)| {
            let names: Vec<String> = els
                .iter()
                .map(|e| {
                    format!(
                        "{}{}:f{}:s{}:o{}",
                        e.semantic, e.index, e.format, e.slot, e.offset
                    )
                })
                .collect();
            Value::Key(*k, format!("layout {k:016x} {}", names.join(",")))
        });
        fields.push(field("layout", Class::Resource, layout_value));
        let vs = o.shader_key(d.vs);
        let ps = o.shader_key(d.ps);
        fields.push(field("vs", Class::Resource, Value::Key(vs, String::new())));
        fields.push(field("ps", Class::Resource, Value::Key(ps, String::new())));
        // Vertex buffers: static by content; rings by the records used.
        let mut mh = Fnv::new();
        mh.u32(d.kind as u32).u64(vs).u64(ps);
        for &&(slot, id, stride, offset) in &vertex {
            let (k, label) = if self.ring(id) {
                let (first, last) = match per_instance(slot) {
                    Some(step) => {
                        let n = if step == 0 {
                            1
                        } else {
                            d.instances.div_ceil(step)
                        };
                        (d.first_instance, d.first_instance + n.max(1) - 1)
                    }
                    None if indexed => (
                        lo.wrapping_add_signed(d.base),
                        hi.wrapping_add_signed(d.base),
                    ),
                    None => (d.start, d.start + d.count.max(1) - 1),
                };
                let from = offset as usize + first as usize * stride as usize;
                // A stride of 0 reads one record for every vertex: its
                // first 16 bytes stand for it.
                let to = if stride == 0 {
                    from + 16
                } else {
                    offset as usize + (last as usize + 1) * stride as usize
                };
                match self.bytes(id).and_then(|b| b.get(from..to)) {
                    Some(bytes) => (
                        key::ring(bytes),
                        format!("ring {:016x} {} B", key::ring(bytes), bytes.len()),
                    ),
                    None => (key::ring(&[]), format!("ring out of bounds {from}..{to}")),
                }
            } else {
                let (k, label) = self.resource_key(id);
                fields.push(field(
                    &format!("vb{slot}.offset"),
                    Class::State,
                    Value::U(offset),
                ));
                (k, label)
            };
            // Alignment takes a ring as a ring: other contents in the same
            // place are a changed draw (a resource difference), not a
            // missing and an extra one.
            mh.u32(u32::from(slot))
                .u64(if self.ring(id) { RING_SLOT } else { k });
            fields.push(field(
                &format!("vb{slot}"),
                Class::Resource,
                Value::Key(k, label),
            ));
            fields.push(field(
                &format!("vb{slot}.stride"),
                Class::State,
                Value::U(stride),
            ));
        }
        if indexed {
            let (k, label) = if ib_ring {
                let mut h = Fnv::new();
                h.u32(0x6964_7820);
                let shift = if vertex_rings { lo } else { 0 };
                for &i in idx.as_deref().unwrap_or(&[]) {
                    h.u32(i - shift);
                }
                let k = h.finish();
                (k, format!("ring indices {k:016x}"))
            } else {
                st(&mut fields, "ib.offset", Value::U(ioff));
                self.resource_key(ib)
            };
            mh.u64(if ib_ring { RING_SLOT } else { k });
            fields.push(field("ib", Class::Resource, Value::Key(k, label)));
            st(&mut fields, "ib.format", Value::U(ifmt));
        }
        mh.u32(start).u32(d.count);
        let match_key = mh.finish();
        // Shader inputs: the registers the shader reads.
        let vs_layout = o.shaders.get(&d.vs).map(|s| s.layout.clone());
        let ps_layout = o.shaders.get(&d.ps).map(|s| s.layout.clone());
        let mut constants = Vec::new();
        for (stage, sl, srv, samp, cbs) in [
            ("vs", &vs_layout, &d.vs_srv, &d.vs_samplers, &d.vs_cb),
            ("ps", &ps_layout, &d.ps_srv, &d.ps_samplers, &d.ps_cb),
        ] {
            let Some(l) = sl else { continue };
            for &(slot, view) in srv {
                if view != 0 && l.uses_texture(slot) {
                    let v = self.view_value(view);
                    fields.push(field(&format!("{stage}.srv{slot}"), Class::Resource, v));
                }
            }
            for &(slot, id) in samp {
                if id != 0 && l.uses_sampler(slot) {
                    let mut f = Vec::new();
                    Sampler::decode(o.state(id)).fields(&format!("{stage}.sampler{slot}"), &mut f);
                    fields.extend(f.into_iter().map(|(n, v)| field(&n, Class::State, v)));
                }
            }
            for (slot, window) in cbs {
                if let Some(c) = l.decode(stage, *slot, window) {
                    constants.push(c);
                }
            }
        }
        // Output merger, rasterizer.
        let mut f = Vec::new();
        let blend = Blend::decode(o.state(d.blend));
        blend.fields(&mut f);
        if blend.uses_factor() {
            for (i, v) in d.blend_factor.iter().enumerate() {
                f.push((format!("blend_factor.{i}"), Value::F(v.to_bits())));
            }
        }
        f.push(("sample_mask".into(), Value::U(d.sample_mask)));
        let depth = Depth::decode(o.state(d.depth));
        depth.fields(&mut f);
        if depth.stencil_enable != 0 {
            f.push(("stencil_ref".into(), Value::U(d.stencil_ref)));
        }
        let raster = Raster::decode(o.state(d.raster));
        raster.fields(&mut f);
        for (n, v) in ["x", "y", "width", "height", "min_depth", "max_depth"]
            .iter()
            .zip(d.viewport)
        {
            f.push((format!("viewport.{n}"), Value::F(v.to_bits())));
        }
        if raster.scissor() {
            for (n, v) in ["left", "top", "right", "bottom"].iter().zip(d.scissor) {
                f.push((format!("scissor.{n}"), Value::I(v)));
            }
        }
        fields.extend(f.into_iter().map(|(n, v)| field(&n, Class::State, v)));
        for &(slot, view) in &d.rtv {
            if view != 0 {
                let v = self.view_value(view);
                fields.push(field(&format!("rtv{slot}"), Class::Resource, v));
            }
        }
        if d.dsv != 0 {
            let v = self.view_value(d.dsv);
            fields.push(field("dsv", Class::Resource, v));
        }
        fields.sort_by(|a, b| a.name.cmp(&b.name));
        let label = format!(
            "draw {:?} count {} vs {vs:016x} ps {ps:016x}",
            d.kind, d.count
        );
        Item {
            cmd,
            kind: ItemKind::Draw,
            vs,
            ps,
            scene: is_scene_draw(d),
            match_key,
            fingerprint: fingerprint(match_key, &fields, &constants),
            fields,
            constants,
            label,
        }
    }
}
