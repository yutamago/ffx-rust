//! A captured frame (`capture.ffxgfx`, see `ffx-gfx` and the graphics
//! layer's `gfxcapture`) issued again through a [`Backend`].
//!
//! The capture names objects by the game's addresses. The replayer makes
//! a backend object for each one a replayed command uses, on first use,
//! from what the capture recorded: shaders from their DXBC, layouts from
//! their elements (checked against the draw's vertex shader), states from
//! their D3D11 descriptions, buffers and textures with their contents.
//! The frame's outputs are not made: the host brings its own targets in
//! ([`Replayer::import`]), and the captured ids of those targets stand for
//! them.
//!
//! Constant buffers are not objects in the capture: each draw carries the
//! bytes its shaders see per slot. The replayer writes them into a
//! dynamic buffer of that size per stage and slot before the draw
//! ([`Cache::constants`]).

use std::collections::{BTreeMap, HashMap};

use ffx_gfx::{Cmd, Frame, Id, Msg};

use crate::backend::{
    Backend, Blend, Buffer, DepthState, InitData, Raster, Sampler, Texture, View,
};
use crate::cache::{Cache, fnv1a};
use crate::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
    StateKind, TextureDesc, ViewDesc, ViewKind,
};
use crate::state::{CbBinding, DrawCall, DrawState, IndexBinding, StageBindings, VertexBinding};

/// A parsed capture: every object it describes, the camera message, and
/// the frame.
pub struct Capture {
    pub objects: HashMap<Id, Msg>,
    /// Object messages in file order (for writing a capture back).
    pub order: Vec<Id>,
    pub world: Option<Msg>,
    pub frame: Frame,
}

impl Capture {
    /// # Errors
    /// When the bytes are not a capture.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let body = bytes
            .strip_prefix(ffx_gfx::CAPTURE_MAGIC.as_slice())
            .ok_or("not a capture (magic)")?;
        let msgs = ffx_gfx::read_all(body).map_err(|e| e.to_string())?;
        let mut objects = HashMap::new();
        let mut order = Vec::new();
        let mut world = None;
        let mut frame = None;
        for m in msgs {
            let id = match &m {
                Msg::Buffer { id, .. }
                | Msg::Texture { id, .. }
                | Msg::View { id, .. }
                | Msg::Shader { id, .. }
                | Msg::Layout { id, .. }
                | Msg::State { id, .. } => Some(*id),
                _ => None,
            };
            match (id, m) {
                (Some(id), m) => {
                    if objects.insert(id, m).is_none() {
                        order.push(id);
                    }
                }
                (None, Msg::Frame(f)) => frame = Some(f),
                (None, m @ Msg::World { .. }) => world = Some(m),
                _ => {}
            }
        }
        Ok(Self {
            objects,
            order,
            world,
            frame: frame.ok_or("no frame in the capture")?,
        })
    }

    /// The resource a view shows.
    #[must_use]
    pub fn view_resource(&self, view: Id) -> Option<Id> {
        match self.objects.get(&view) {
            Some(Msg::View { resource, .. }) => Some(*resource),
            _ => None,
        }
    }

    /// The description of the texture a view shows.
    #[must_use]
    pub fn view_texture_desc(&self, view: Id) -> Option<TextureDesc> {
        match self.objects.get(&self.view_resource(view)?) {
            Some(Msg::Texture { desc, .. }) => Some(TextureDesc::from_words(*desc)),
            _ => None,
        }
    }

    /// The captured contents of mip 0, slice 0 of the texture behind a
    /// view (read back when the capture ended, not where the scene did).
    #[must_use]
    pub fn view_texture_data(&self, view: Id) -> Option<&ffx_gfx::Subresource> {
        match self.objects.get(&self.view_resource(view)?) {
            Some(Msg::Texture { subresources, .. }) => subresources.first(),
            _ => None,
        }
    }
}

/// The scene's targets: colour, normal and depth (the graphics layer's
/// rule: two render targets with a depth buffer, nothing else).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneTargets {
    pub color: Id,
    pub normal: Id,
    pub depth: Id,
}

impl SceneTargets {
    fn of(d: &ffx_gfx::Draw) -> Option<Self> {
        match d.rtv.as_slice() {
            [(0, color), (1, normal)] if d.dsv != 0 => Some(Self {
                color: *color,
                normal: *normal,
                depth: d.dsv,
            }),
            _ => None,
        }
    }

    #[must_use]
    pub fn contains(&self, view: Id) -> bool {
        view == self.color || view == self.normal || view == self.depth
    }
}

/// The commands of a capture that make its scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub targets: SceneTargets,
    /// Indices into the frame's commands, in order.
    pub cmds: Vec<usize>,
    /// Scene draws among them, and the 1-based number of each in the
    /// frame's draws (the `draw N` lines of `frame.txt`).
    pub draws: usize,
    pub draw_numbers: Vec<u32>,
    /// Draws into other targets between the scene's first and last draw
    /// (left out).
    pub other_draws: usize,
}

/// The scene of a frame: the clears of its targets before or during it,
/// buffer updates before its last draw, texture copies during it, and the
/// draws into its targets, from the first scene draw to the last one of
/// the same run (the layer's live link ends a scene when the game binds
/// other targets after scene draws).
#[must_use]
pub fn select_scene(frame: &Frame) -> Option<Selection> {
    let mut targets = None;
    let mut first = None;
    let mut last = None;
    for (i, c) in frame.cmds.iter().enumerate() {
        let Cmd::Draw(d) = c else { continue };
        match (targets, SceneTargets::of(d)) {
            (None, Some(t)) => {
                targets = Some(t);
                first = Some(i);
                last = Some(i);
            }
            (Some(t), Some(u)) if t == u => last = Some(i),
            (Some(_), _) => break,
            (None, None) => {}
        }
    }
    let (targets, first, last) = (targets?, first?, last?);
    let mut sel = Selection {
        targets,
        cmds: Vec::new(),
        draws: 0,
        draw_numbers: Vec::new(),
        other_draws: 0,
    };
    let mut number = 0u32;
    for (i, c) in frame.cmds.iter().enumerate().take(last + 1) {
        let keep = match c {
            Cmd::Draw(d) => {
                number += 1;
                let scene = i >= first && SceneTargets::of(d) == Some(targets);
                if scene {
                    sel.draws += 1;
                    sel.draw_numbers.push(number);
                } else if i > first {
                    sel.other_draws += 1;
                }
                scene
            }
            Cmd::ClearTarget { view, .. } | Cmd::ClearDepth { view, .. } => targets.contains(*view),
            Cmd::UpdateBuffer { .. } => true,
            Cmd::CopyTexture { .. } => i > first,
        };
        if keep {
            sel.cmds.push(i);
        }
    }
    Some(sel)
}

/// Captured objects the replay could not make as they were, and objects
/// it had to make that the capture does not have.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Gaps {
    /// What went wrong or was missing, with how often.
    pub notes: BTreeMap<String, u32>,
}

impl Gaps {
    fn note(&mut self, what: impl Into<String>) {
        *self.notes.entry(what.into()).or_insert(0) += 1;
    }
}

/// Backend objects made for (or brought in as) the capture's ids.
#[derive(Default)]
pub struct Replayer {
    pub cache: Cache,
    shaders: HashMap<Id, crate::backend::Shader>,
    layouts: HashMap<Id, crate::backend::Layout>,
    blends: HashMap<Id, Blend>,
    depths: HashMap<Id, DepthState>,
    rasters: HashMap<Id, Raster>,
    samplers: HashMap<Id, Sampler>,
    buffers: HashMap<Id, Buffer>,
    textures: HashMap<Id, Texture>,
    views: HashMap<Id, View>,
    /// Back from handles to a captured id (for the replayed stream).
    back: Back,
    pub gaps: Gaps,
    /// Objects made: textures, views, buffers.
    pub made: [u32; 3],
}

/// Handle → a captured id it stands for.
#[derive(Default)]
struct Back {
    shaders: HashMap<crate::backend::Shader, Id>,
    layouts: HashMap<crate::backend::Layout, Id>,
    blends: HashMap<Blend, Id>,
    depths: HashMap<DepthState, Id>,
    rasters: HashMap<Raster, Id>,
    samplers: HashMap<Sampler, Id>,
    buffers: HashMap<Buffer, Id>,
    views: HashMap<View, Id>,
}

fn stage_of(kind: u8) -> Option<Stage> {
    match kind {
        b'v' => Some(Stage::Vertex),
        b'p' => Some(Stage::Pixel),
        b'g' => Some(Stage::Geometry),
        _ => None,
    }
}

fn state_kind(k: ffx_gfx::StateKind) -> StateKind {
    match k {
        ffx_gfx::StateKind::Blend => StateKind::Blend,
        ffx_gfx::StateKind::Depth => StateKind::Depth,
        ffx_gfx::StateKind::Raster => StateKind::Raster,
        ffx_gfx::StateKind::Sampler => StateKind::Sampler,
    }
}

impl Replayer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Brings in a host view (and its texture) for captured view `id` and
    /// its resource `resource`.
    pub fn import(&mut self, id: Id, view: View, resource: Option<(Id, Texture)>) {
        self.views.insert(id, view);
        self.back.views.insert(view, id);
        if let Some((rid, tex)) = resource {
            self.textures.insert(rid, tex);
        }
    }

    fn shader<B: Backend>(
        &mut self,
        b: &mut B,
        cap: &Capture,
        id: Id,
    ) -> Option<crate::backend::Shader> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.shaders.get(&id) {
            return Some(s);
        }
        let Some(Msg::Shader { kind, bytecode, .. }) = cap.objects.get(&id) else {
            self.gaps.note("shader missing from the capture");
            return None;
        };
        let stage = stage_of(*kind)?;
        match self.cache.shader(b, stage, bytecode) {
            Ok(s) => {
                self.shaders.insert(id, s);
                self.back.shaders.entry(s).or_insert(id);
                Some(s)
            }
            Err(e) => {
                self.gaps.note(format!("shader refused: {e}"));
                None
            }
        }
    }

    fn layout<B: Backend>(
        &mut self,
        b: &mut B,
        cap: &Capture,
        id: Id,
        vs: Id,
    ) -> Option<crate::backend::Layout> {
        if id == 0 {
            return None;
        }
        if let Some(&l) = self.layouts.get(&id) {
            return Some(l);
        }
        let (Some(Msg::Layout { elements, .. }), Some(Msg::Shader { bytecode, .. })) =
            (cap.objects.get(&id), cap.objects.get(&vs))
        else {
            self.gaps.note("layout or its vertex shader missing");
            return None;
        };
        let elements: Vec<InputElement> = elements
            .iter()
            .map(|e| InputElement {
                semantic: e.semantic.clone(),
                index: e.index,
                format: e.format,
                slot: e.slot,
                offset: e.offset,
                per_instance: e.per_instance,
                step: e.step,
            })
            .collect();
        match self.cache.layout(b, &elements, bytecode) {
            Ok(l) => {
                self.layouts.insert(id, l);
                self.back.layouts.entry(l).or_insert(id);
                Some(l)
            }
            Err(e) => {
                self.gaps.note(format!("layout refused: {e}"));
                None
            }
        }
    }

    fn state_desc<'c>(&mut self, cap: &'c Capture, id: Id, want: StateKind) -> Option<&'c [u8]> {
        match cap.objects.get(&id) {
            Some(Msg::State { kind, desc, .. }) if state_kind(*kind) == want => Some(desc),
            _ => {
                self.gaps
                    .note(format!("{want:?} state missing from the capture"));
                None
            }
        }
    }

    fn blend<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<Blend> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.blends.get(&id) {
            return Some(s);
        }
        let d = BlendDesc::from_d3d11(self.state_desc(cap, id, StateKind::Blend)?).ok()?;
        let s = self
            .cache
            .blend(b, &d)
            .map_err(|e| self.gaps.note(format!("blend refused: {e}")))
            .ok()?;
        self.blends.insert(id, s);
        self.back.blends.entry(s).or_insert(id);
        Some(s)
    }

    fn depth<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<DepthState> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.depths.get(&id) {
            return Some(s);
        }
        let d = DepthStencilDesc::from_d3d11(self.state_desc(cap, id, StateKind::Depth)?).ok()?;
        let s = self
            .cache
            .depth(b, &d)
            .map_err(|e| self.gaps.note(format!("depth state refused: {e}")))
            .ok()?;
        self.depths.insert(id, s);
        self.back.depths.entry(s).or_insert(id);
        Some(s)
    }

    fn raster<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<Raster> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.rasters.get(&id) {
            return Some(s);
        }
        let d = RasterDesc::from_d3d11(self.state_desc(cap, id, StateKind::Raster)?).ok()?;
        let s = self
            .cache
            .raster(b, &d)
            .map_err(|e| self.gaps.note(format!("raster state refused: {e}")))
            .ok()?;
        self.rasters.insert(id, s);
        self.back.rasters.entry(s).or_insert(id);
        Some(s)
    }

    fn sampler<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<Sampler> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.samplers.get(&id) {
            return Some(s);
        }
        let d = SamplerDesc::from_d3d11(self.state_desc(cap, id, StateKind::Sampler)?).ok()?;
        let s = self
            .cache
            .sampler(b, &d)
            .map_err(|e| self.gaps.note(format!("sampler refused: {e}")))
            .ok()?;
        self.samplers.insert(id, s);
        self.back.samplers.entry(s).or_insert(id);
        Some(s)
    }

    fn buffer<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<Buffer> {
        if id == 0 {
            return None;
        }
        if let Some(&s) = self.buffers.get(&id) {
            return Some(s);
        }
        let Some(Msg::Buffer { desc, data, .. }) = cap.objects.get(&id) else {
            self.gaps.note("buffer missing from the capture");
            return None;
        };
        let desc = BufferDesc::from_words(*desc);
        let data = (data.len() == desc.size as usize).then_some(data.as_slice());
        if data.is_none() {
            self.gaps.note(format!(
                "buffer without contents (usage {}, bind {:#x})",
                desc.usage, desc.bind
            ));
        }
        let made = b.create_buffer(&desc, data).or_else(|e| {
            // Immutable buffers need their contents.
            let mut d = desc;
            d.usage = crate::desc::USAGE_DEFAULT;
            self.gaps
                .note(format!("buffer refused ({e}); made as default usage"));
            b.create_buffer(&d, data)
        });
        match made {
            Ok(s) => {
                self.made[2] += 1;
                self.buffers.insert(id, s);
                self.back.buffers.insert(s, id);
                Some(s)
            }
            Err(e) => {
                self.gaps.note(format!("buffer refused: {e}"));
                None
            }
        }
    }

    fn texture<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<Texture> {
        if let Some(&t) = self.textures.get(&id) {
            return Some(t);
        }
        let Some(Msg::Texture {
            desc, subresources, ..
        }) = cap.objects.get(&id)
        else {
            self.gaps.note("texture missing from the capture");
            return None;
        };
        let desc = TextureDesc::from_words(*desc);
        let want = desc.mips.max(1) * desc.array_size.max(1);
        let init: Vec<InitData> = if subresources.len() == want as usize {
            subresources
                .iter()
                .map(|s| InitData {
                    row_pitch: s.row_pitch,
                    data: &s.data,
                })
                .collect()
        } else {
            self.gaps.note(format!(
                "texture without contents (format {}, bind {:#x})",
                desc.format, desc.bind
            ));
            Vec::new()
        };
        let made = b.create_texture(&desc, &init).or_else(|e| {
            self.gaps.note(format!(
                "texture with contents refused ({e}, format {}, bind {:#x}): made empty",
                desc.format, desc.bind
            ));
            b.create_texture(&desc, &[])
        });
        match made {
            Ok(t) => {
                self.made[0] += 1;
                self.textures.insert(id, t);
                Some(t)
            }
            Err(e) => {
                self.gaps.note(format!("texture refused: {e}"));
                None
            }
        }
    }

    fn view<B: Backend>(&mut self, b: &mut B, cap: &Capture, id: Id) -> Option<View> {
        if id == 0 {
            return None;
        }
        if let Some(&v) = self.views.get(&id) {
            return Some(v);
        }
        let Some(Msg::View {
            kind,
            resource,
            desc,
            ..
        }) = cap.objects.get(&id)
        else {
            self.gaps.note("view missing from the capture");
            return None;
        };
        let tex = self.texture(b, cap, *resource)?;
        let kind = match kind {
            ffx_gfx::ViewKind::ShaderResource => ViewKind::ShaderResource,
            ffx_gfx::ViewKind::RenderTarget => ViewKind::RenderTarget,
            ffx_gfx::ViewKind::DepthStencil => ViewKind::DepthStencil,
        };
        let desc = desc.map(ViewDesc::from_words);
        match b.create_view(tex, kind, desc.as_ref()) {
            Ok(v) => {
                self.made[1] += 1;
                self.views.insert(id, v);
                self.back.views.insert(v, id);
                Some(v)
            }
            Err(e) => {
                self.gaps.note(format!("view refused: {e}"));
                None
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // one stage's three slot lists
    fn stage<B: Backend>(
        &mut self,
        b: &mut B,
        cap: &Capture,
        stage: Stage,
        shader: Id,
        cbs: &[(u8, Vec<u8>)],
        srvs: &[(u8, Id)],
        samplers: &[(u8, Id)],
    ) -> StageBindings {
        let mut out = StageBindings {
            shader: self.shader(b, cap, shader),
            ..StageBindings::default()
        };
        for (slot, data) in cbs {
            let Some(at) = out.cbs.get_mut(*slot as usize) else {
                continue;
            };
            if data.is_empty() {
                self.gaps.note("constant buffer without contents");
            }
            match self.cache.constants(b, stage, *slot, data) {
                Ok(buffer) => {
                    *at = Some(CbBinding {
                        buffer,
                        first: 0,
                        count: 0,
                    });
                }
                Err(e) => self.gaps.note(format!("constant buffer refused: {e}")),
            }
        }
        for &(slot, id) in srvs {
            let v = self.view(b, cap, id);
            if let Some(at) = out.srvs.get_mut(slot as usize) {
                *at = v;
            }
        }
        for &(slot, id) in samplers {
            let s = self.sampler(b, cap, id);
            if let Some(at) = out.samplers.get_mut(slot as usize) {
                *at = s;
            }
        }
        out
    }

    /// The draw state for a captured draw (objects made as needed).
    pub fn draw_state<B: Backend>(
        &mut self,
        b: &mut B,
        cap: &Capture,
        d: &ffx_gfx::Draw,
    ) -> DrawState {
        let mut s = DrawState {
            topology: d.topology,
            layout: self.layout(b, cap, d.layout, d.vs),
            ..DrawState::default()
        };
        for &(slot, id, stride, offset) in &d.vertex {
            let buffer = self.buffer(b, cap, id);
            if let (Some(at), Some(buffer)) = (s.vertex.get_mut(slot as usize), buffer) {
                *at = Some(VertexBinding {
                    buffer,
                    stride,
                    offset,
                });
            }
        }
        s.index = self.buffer(b, cap, d.index.0).map(|buffer| IndexBinding {
            buffer,
            format: d.index.1,
            offset: d.index.2,
        });
        s.vs = self.stage(
            b,
            cap,
            Stage::Vertex,
            d.vs,
            &d.vs_cb,
            &d.vs_srv,
            &d.vs_samplers,
        );
        s.ps = self.stage(
            b,
            cap,
            Stage::Pixel,
            d.ps,
            &d.ps_cb,
            &d.ps_srv,
            &d.ps_samplers,
        );
        s.blend = self.blend(b, cap, d.blend);
        s.blend_factor = d.blend_factor;
        s.sample_mask = d.sample_mask;
        s.depth = self.depth(b, cap, d.depth);
        s.stencil_ref = d.stencil_ref;
        s.raster = self.raster(b, cap, d.raster);
        s.viewport = d.viewport;
        s.scissor = d.scissor;
        for &(slot, id) in &d.rtv {
            let v = self.view(b, cap, id);
            if let Some(at) = s.targets.get_mut(slot as usize) {
                *at = v;
            }
        }
        s.depth_target = self.view(b, cap, d.dsv);
        s
    }

    /// Issues the selected commands of `cap` in order; `after` runs after
    /// each one with its index into the frame's commands.
    pub fn run<B: Backend>(
        &mut self,
        b: &mut B,
        cap: &Capture,
        cmds: &[usize],
        mut after: impl FnMut(&mut Self, &mut B, usize, &DrawState),
    ) {
        for &i in cmds {
            let Some(c) = cap.frame.cmds.get(i) else {
                continue;
            };
            let mut state = DrawState::default();
            match c {
                Cmd::Draw(d) => {
                    state = self.draw_state(b, cap, d);
                    b.draw(&state, draw_call(d));
                }
                Cmd::ClearTarget { view, color } => {
                    if let Some(v) = self.view(b, cap, *view) {
                        b.clear_target(v, *color);
                    }
                }
                Cmd::ClearDepth {
                    view,
                    flags,
                    depth,
                    stencil,
                } => {
                    if let Some(v) = self.view(b, cap, *view) {
                        b.clear_depth(v, *flags, *depth, *stencil);
                    }
                }
                Cmd::UpdateBuffer { id, offset, data } => {
                    if let Some(buf) = self.buffer(b, cap, *id) {
                        b.write_buffer(buf, *offset, data);
                    }
                }
                Cmd::CopyTexture { dst, src } => {
                    if let (Some(d), Some(s)) =
                        (self.texture(b, cap, *dst), self.texture(b, cap, *src))
                    {
                        b.copy_texture(d, s);
                    }
                }
            }
            after(self, b, i, &state);
        }
    }

    /// A draw state seen on the backend, in the capture's terms: ids are
    /// the captured objects the handles were made for (for shaders and
    /// states, the first of the captured ids with that bytecode or
    /// description), constant buffer contents are `cbs` (the bytes last
    /// written to each bound buffer). Handles the replay did not make come
    /// out as id 0.
    #[must_use]
    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // every field; slots are < 16
    pub fn captured_draw(
        &self,
        s: &DrawState,
        call: DrawCall,
        cbs: impl Fn(Buffer) -> Vec<u8>,
    ) -> ffx_gfx::Draw {
        fn id<H: std::hash::Hash + Eq>(m: &HashMap<H, Id>, h: Option<H>) -> Id {
            h.and_then(|h| m.get(&h).copied()).unwrap_or(0)
        }
        let stage_cbs = |st: &StageBindings| -> Vec<(u8, Vec<u8>)> {
            st.cbs
                .iter()
                .enumerate()
                .filter_map(|(i, c)| c.map(|c| (i as u8, cbs(c.buffer))))
                .collect()
        };
        let views = |list: &[Option<View>]| -> Vec<(u8, Id)> {
            list.iter()
                .enumerate()
                .filter_map(|(i, v)| v.map(|v| (i as u8, id(&self.back.views, Some(v)))))
                .collect()
        };
        let samplers = |list: &[Option<Sampler>]| -> Vec<(u8, Id)> {
            list.iter()
                .enumerate()
                .filter_map(|(i, v)| v.map(|v| (i as u8, id(&self.back.samplers, Some(v)))))
                .collect()
        };
        let (kind, count, instances, start, base, first_instance) = match call {
            DrawCall::Indexed { count, start, base } => {
                (ffx_gfx::DrawKind::Indexed, count, 1, start, base, 0)
            }
            DrawCall::Plain { count, start } => (ffx_gfx::DrawKind::Plain, count, 1, start, 0, 0),
            DrawCall::IndexedInstanced {
                count,
                instances,
                start,
                base,
                first_instance,
            } => (
                ffx_gfx::DrawKind::IndexedInstanced,
                count,
                instances,
                start,
                base,
                first_instance,
            ),
            DrawCall::Instanced {
                count,
                instances,
                start,
                first_instance,
            } => (
                ffx_gfx::DrawKind::Instanced,
                count,
                instances,
                start,
                0,
                first_instance,
            ),
        };
        ffx_gfx::Draw {
            kind,
            count,
            instances,
            start,
            base,
            first_instance,
            topology: s.topology,
            layout: id(&self.back.layouts, s.layout),
            vs: id(&self.back.shaders, s.vs.shader),
            ps: id(&self.back.shaders, s.ps.shader),
            vertex: s
                .vertex
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    v.map(|v| {
                        (
                            i as u8,
                            id(&self.back.buffers, Some(v.buffer)),
                            v.stride,
                            v.offset,
                        )
                    })
                })
                .collect(),
            index: s.index.map_or((0, 0, 0), |x| {
                (id(&self.back.buffers, Some(x.buffer)), x.format, x.offset)
            }),
            vs_cb: stage_cbs(&s.vs),
            ps_cb: stage_cbs(&s.ps),
            vs_srv: views(&s.vs.srvs),
            ps_srv: views(&s.ps.srvs),
            vs_samplers: samplers(&s.vs.samplers),
            ps_samplers: samplers(&s.ps.samplers),
            blend: id(&self.back.blends, s.blend),
            blend_factor: s.blend_factor,
            sample_mask: s.sample_mask,
            depth: id(&self.back.depths, s.depth),
            stencil_ref: s.stencil_ref,
            raster: id(&self.back.rasters, s.raster),
            viewport: s.viewport,
            scissor: s.scissor,
            rtv: views(&s.targets),
            dsv: id(&self.back.views, s.depth_target),
        }
    }
}

/// The call of a captured draw.
#[must_use]
pub fn draw_call(d: &ffx_gfx::Draw) -> DrawCall {
    match d.kind {
        ffx_gfx::DrawKind::Indexed => DrawCall::Indexed {
            count: d.count,
            start: d.start,
            base: d.base,
        },
        ffx_gfx::DrawKind::Plain => DrawCall::Plain {
            count: d.count,
            start: d.start,
        },
        ffx_gfx::DrawKind::IndexedInstanced => DrawCall::IndexedInstanced {
            count: d.count,
            instances: d.instances,
            start: d.start,
            base: d.base,
            first_instance: d.first_instance,
        },
        ffx_gfx::DrawKind::Instanced => DrawCall::Instanced {
            count: d.count,
            instances: d.instances,
            start: d.start,
            first_instance: d.first_instance,
        },
    }
}

/// Differences between two captured draws of one capture, field by field:
/// shaders by bytecode, layouts by elements, states by description
/// (padding aside), the rest by value and id. Empty when they are the
/// same draw. A stand-in for `ffx-drawdiff` until it lands.
#[must_use]
pub fn compare_draws(cap: &Capture, a: &ffx_gfx::Draw, b: &ffx_gfx::Draw) -> Vec<String> {
    let mut out = Vec::new();
    let shader = |id: Id| match cap.objects.get(&id) {
        Some(Msg::Shader { bytecode, .. }) => fnv1a(bytecode),
        _ => 0,
    };
    let layout = |id: Id| match cap.objects.get(&id) {
        Some(Msg::Layout { elements, .. }) => Some(elements.clone()),
        _ => None,
    };
    let state = |id: Id| match cap.objects.get(&id) {
        Some(Msg::State { kind, desc, .. }) => {
            crate::desc::without_padding(state_kind(*kind), desc)
        }
        _ => None,
    };
    let states = |list: &[(u8, Id)]| -> Vec<(u8, Option<Vec<u8>>)> {
        list.iter().map(|&(s, id)| (s, state(id))).collect()
    };
    macro_rules! same {
        ($name:literal, $x:expr, $y:expr) => {
            if $x != $y {
                out.push(format!("{}: {:?} / {:?}", $name, $x, $y));
            }
        };
    }
    same!("kind", a.kind, b.kind);
    same!(
        "call",
        (a.count, a.instances, a.start, a.base, a.first_instance),
        (b.count, b.instances, b.start, b.base, b.first_instance)
    );
    same!("topology", a.topology, b.topology);
    same!("layout", layout(a.layout), layout(b.layout));
    same!("vs", shader(a.vs), shader(b.vs));
    same!("ps", shader(a.ps), shader(b.ps));
    same!("vertex", a.vertex, b.vertex);
    same!("index", a.index, b.index);
    for (name, x, y) in [("vs_cb", &a.vs_cb, &b.vs_cb), ("ps_cb", &a.ps_cb, &b.ps_cb)] {
        // The replay's buffers are exactly the window the shader saw; a
        // window past the end of the game's buffer was cut short there,
        // and reads as zeros.
        let pad = |l: &[(u8, Vec<u8>)]| -> Vec<(u8, Vec<u8>)> {
            l.iter()
                .map(|(s, d)| {
                    let mut d = d.clone();
                    d.resize(d.len().next_multiple_of(16).max(16), 0);
                    (*s, d)
                })
                .collect()
        };
        if pad(x) != pad(y) {
            out.push(format!("{name}: contents differ"));
        }
    }
    same!("vs_srv", a.vs_srv, b.vs_srv);
    same!("ps_srv", a.ps_srv, b.ps_srv);
    same!(
        "vs_samplers",
        states(&a.vs_samplers),
        states(&b.vs_samplers)
    );
    same!(
        "ps_samplers",
        states(&a.ps_samplers),
        states(&b.ps_samplers)
    );
    same!("blend", state(a.blend), state(b.blend));
    same!(
        "blend_factor",
        a.blend_factor.map(f32::to_bits),
        b.blend_factor.map(f32::to_bits)
    );
    same!("sample_mask", a.sample_mask, b.sample_mask);
    same!("depth", state(a.depth), state(b.depth));
    same!("stencil_ref", a.stencil_ref, b.stencil_ref);
    same!("raster", state(a.raster), state(b.raster));
    same!(
        "viewport",
        a.viewport.map(f32::to_bits),
        b.viewport.map(f32::to_bits)
    );
    same!("scissor", a.scissor, b.scissor);
    same!("rtv", a.rtv, b.rtv);
    same!("dsv", a.dsv, b.dsv);
    out
}

/// A capture file of the replayed frame: the original's objects and
/// camera, then `cmds` as the frame (for `ffx-drawdiff` against the
/// original capture).
#[must_use]
pub fn write_capture(cap: &Capture, cmds: Vec<Cmd>) -> Vec<u8> {
    let mut out = ffx_gfx::CAPTURE_MAGIC.to_vec();
    for id in &cap.order {
        if let Some(m) = cap.objects.get(id) {
            m.write(&mut out);
        }
    }
    if let Some(w) = &cap.world {
        w.write(&mut out);
    }
    Msg::Frame(Frame {
        number: cap.frame.number,
        width: cap.frame.width,
        height: cap.frame.height,
        cmds,
    })
    .write(&mut out);
    out
}
