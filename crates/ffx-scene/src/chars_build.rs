//! The scene builder for the character world (scene `+0x7c`): `build(c,
//! records, frame)` gives the draws of its `Opaque` and `Transparent`
//! passes (character segments, weapon parts, the blob shadow) as the map
//! builder does ([`crate::build`]: the same [`Built`] of resources and
//! draws), from the frame's character-world records (the scene inputs'
//! character part, [`crate::chars`]) and the `chr/` clusters
//! (`docs/formats/scene-chars.md`, "The builder").
//!
//! Per pass, as `renderWorldCulledSorted` (`0x009f3c30`) and
//! `renderMeshInstance` (`0x005b9a90`) do for any world:
//!
//! 1. cull and key the record's bounds, one visibility job per instance
//!    list of the walk (every character instance is in a list of its
//!    own), sort each job, merge ([`crate::cull`], [`crate::sort`]);
//! 2. per instance, its segments in four buckets by
//!    `m_segmentRenderOrder`, a null material ending the instance, the
//!    pass remapping, the effect variant's render pass for the pass
//!    (`selectTechniquePass` `0x005ba690`), the hidden byte; one draw per
//!    shader pass;
//! 3. per draw: the pass's programs, states and input layout; the
//!    segment's static streams, with the cloth vertices as a dynamic
//!    stream where the program reads the cloth type (stream 5 of
//!    `phyrechrlitshader`); the material's textures at the
//!    `TextureQuality` rule and its samplers; the constant buffers through
//!    the location tables with the render thread's copy rule
//!    (`0x0059a650`, [`chars::cb_copy`]: `float4x4`/`float4x3` entries
//!    transposed, stopping at a `0xffffffff` first word), the per-node
//!    values from the joints ([`chars::node_buffer`]: `World`,
//!    `BoneTransforms`, `WorldInverse`, ...).
//!
//! The blob shadow (the ground-shadow cluster `+0x28c4`, a
//! `ClassDynamicMesh` the game makes from `ChrY0Shadow.cgfx.phyre`,
//! `PhyreScene_loadChrShaShader` `0x00664360`, `0x0069f9d0`) draws the
//! record's dynamic geometry lists: the four arrays as dynamic vertex
//! streams 0-3, the `u16` indices widened to `R32_UINT`.

// Draw building names its handles and buffers (b, c, d, s, ...).
#![allow(clippy::many_single_char_names)]

use std::collections::HashMap;
use std::ops::Range;

use ffx_draw::backend::{Blend, Buffer, DepthState, Raster, Sampler};
use ffx_draw::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, Stage, ViewKind,
};
use ffx_draw::state::{CbBinding, IndexBinding, StageBindings, VertexBinding};
use ffx_draw::{DrawCall, DrawState};
use ffx_gfx::scene_inputs::SceneInputs as Record;

use crate::build::{self, Built, Clusters, Error, MapDraw, Pass};
use crate::chars;
use crate::chars_model::{self, EffectContexts, InstanceInfo};
use crate::effect::{EffectVariant, LocTable, ShaderPass, Source};
use crate::frame::{External, Settings};
use crate::inputs;
use crate::map::{self, MapModel, SamplerState};
use crate::sort::{self, Entry};

fn err(what: impl Into<String>) -> Error {
    Error(what.into())
}

/// The cluster the blob shadow's material comes from
/// (`/FFX_Data/GameData/PS3Data/Shaders/GCM/ChrY0Shadow.cgfx.phyre`, the
/// D3D11 build's name for it).
pub const SHADOW_CLUSTER: &str = "ffx_data/gamedata/ps3data/shaders/d3d11/chry0shadow.fx.phyre";

/// The shader input the cloth vertices feed (`PRenderDataType` and
/// semantic index): `COLOR1` of `phyrechrlitshader`'s cloth variants,
/// render type `Color` index 1. `batchSetClothVertexBuffers`
/// (`0x006365f0`) uploads a cloth segment's vertices into its `Color`
/// block's next version in the dynamic ring, so the segment's static
/// `Color` block is never bound (`docs/formats/scene-chars.md`, "Cloth").
pub const CLOTH_RENDER_TYPE: (&str, u32) = ("Color", 1);

/// The blob shadow's material: `ChrY0Shadow`'s effect variant and the
/// sampler states of its `PMaterial`'s buffer.
#[derive(Debug, Clone)]
pub struct ShadowMaterial {
    pub variant: EffectVariant,
    pub contexts: EffectContexts,
    pub samplers: Vec<(u32, SamplerState)>,
}

impl ShadowMaterial {
    /// Reads `ChrY0Shadow`'s cluster.
    ///
    /// # Errors
    /// When it doesn't parse or has no material.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, Error> {
        let variant = EffectVariant::parse(bytes.clone()).map_err(|e| err(e.0))?;
        let contexts = EffectContexts::parse(bytes.clone()).map_err(|e| err(e.0))?;
        let g = crate::phyre::Graph::parse(bytes).map_err(|e| err(format!("shadow: {e}")))?;
        let m = *g
            .objects("PMaterial")
            .first()
            .ok_or_else(|| err("shadow cluster: no PMaterial"))?;
        let mut samplers = Vec::new();
        if let Some(b) = g.target_obj(m, "PMaterial", "m_parameterBuffer") {
            for f in &g.pointers[b.list] {
                if f.object() != b.index || f.member() & 0x8000_0000 == 0 {
                    continue;
                }
                let at = f.member() & 0x7fff_ffff;
                let list = f.target_list() as usize;
                if at >= 8 && f.user_fixup().is_none() && g.class_of(list) == "PSamplerState" {
                    samplers.push((at - 8, map::sampler_state(&g, g.obj(list, f.target()))));
                }
            }
        }
        Ok(Self {
            variant,
            contexts,
            samplers,
        })
    }
}

/// The static data of the character world: the character models (one
/// [`MapModel`] with every model appended), their instances' identifying
/// data, the effect variants and textures their materials import, and
/// the blob shadow's material.
#[derive(Debug, Clone)]
pub struct CharClusters {
    pub map: MapModel,
    /// Per model: its archive path and its instances in [`Self::map`].
    pub models: Vec<(String, Range<usize>)>,
    /// Per instance of [`Self::map`].
    pub info: Vec<InstanceInfo>,
    pub effects: HashMap<String, EffectVariant>,
    /// The effects' node contexts, by the same import ids.
    pub contexts: HashMap<String, EffectContexts>,
    pub textures: HashMap<String, ffx_phyre::Texture>,
    pub shadow: Option<ShadowMaterial>,
}

impl CharClusters {
    /// Loads the character models of `model_paths` and what they import,
    /// and the shadow material ([`SHADOW_CLUSTER`]) when `read` has it.
    ///
    /// # Errors
    /// When a model, an effect or a texture is missing or doesn't parse.
    pub fn load(
        read: impl Fn(&str) -> Option<Vec<u8>>,
        model_paths: &[String],
    ) -> Result<Self, Error> {
        let mut out = Self {
            map: MapModel {
                instances: Vec::new(),
                segments: Vec::new(),
                materials: Vec::new(),
                bytes: Vec::new(),
            },
            models: Vec::new(),
            info: Vec::new(),
            effects: HashMap::new(),
            contexts: HashMap::new(),
            textures: HashMap::new(),
            shadow: None,
        };
        for path in model_paths {
            // The map builder's loader reads a model and its imports.
            let c = Clusters::load(&read, path)?;
            let bytes = read(path).ok_or_else(|| err(format!("{path}: not found")))?;
            let mut info =
                chars_model::instance_infos(bytes).map_err(|e| err(format!("{path}: {e}")))?;
            chars_model::add_materials(&mut info, &c.map);
            if info.len() != c.map.instances.len() {
                return Err(err(format!("{path}: instance lists differ")));
            }
            let range = chars_model::append(&mut out.map, c.map);
            out.models.push((path.clone(), range));
            out.info.extend(info);
            out.effects.extend(c.effects);
            out.textures.extend(c.textures);
        }
        for effect in out.effects.keys() {
            if out.contexts.contains_key(effect) {
                continue;
            }
            let path = map::archive_path(effect);
            let bytes = read(&path).ok_or_else(|| err(format!("{path}: not found")))?;
            let cx = EffectContexts::parse(bytes).map_err(|e| err(format!("{path}: {e}")))?;
            out.contexts.insert(effect.clone(), cx);
        }
        if let Some(bytes) = read(SHADOW_CLUSTER) {
            out.shadow = Some(ShadowMaterial::parse(bytes)?);
        }
        Ok(out)
    }
}

/// What a record instance is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// An instance of [`CharClusters::map`]'s instance.
    Model(usize),
    /// The blob shadow's dynamic mesh.
    Shadow,
    /// Nothing loaded matches it (not drawn; noted).
    Unknown,
}

/// Whether a record instance belongs to a cluster with a
/// `ClassDynamicMesh` list (the ground-shadow cluster `+0x28c4`).
#[must_use]
pub fn is_dynamic(s: &Record, index: usize) -> bool {
    s.instances
        .get(index)
        .and_then(|i| s.clusters.get(i.cluster as usize))
        .is_some_and(|c| c.lists.iter().any(|(class, _)| class == "ClassDynamicMesh"))
}

/// The role of every record instance: the shadow, or the loaded file
/// instance whose mesh it carries ([`chars_model::matches`]) with the
/// best [`chars_model::material_score`] (the first among equals).
#[must_use]
pub fn resolve(c: &CharClusters, s: &Record) -> Vec<Role> {
    let cw = s.chars.as_deref();
    (0..s.instances.len())
        .map(|k| {
            if is_dynamic(s, k) {
                return Role::Shadow;
            }
            let (Some(i), Some(ci)) = (s.instances.get(k), cw.and_then(|w| w.instances.get(k)))
            else {
                return Role::Unknown;
            };
            let bounds = s
                .bounds
                .iter()
                .find(|b| b.instance == u32::try_from(k).ok());
            let mut best: Option<(usize, usize)> = None;
            for (f, info) in c.info.iter().enumerate() {
                if chars_model::matches(info, i, ci, bounds) {
                    let score = chars_model::material_score(info, s, i);
                    if best.is_none_or(|b| score > b.1) {
                        best = Some((f, score));
                    }
                }
            }
            best.map_or(Role::Unknown, |b| Role::Model(b.0))
        })
        .collect()
}

/// What the character draws take from the rest of the frame.
#[derive(Debug, Clone)]
pub struct FrameInputs {
    pub settings: Settings,
    /// `D3D11_VIEWPORT` and scissor of the scene targets.
    pub viewport: [f32; 6],
    pub scissor: [i32; 4],
    /// The scene-wide parameter buffer (`PSceneWideParameters_getBuffer`)
    /// as the game keeps it: matrices in memory order (the copy rule
    /// transposes them).
    pub scene_wide: Vec<u8>,
    /// Per record instance: the lights of its node context, in order
    /// (the character lighting's three `PLight`s).
    pub lights: HashMap<usize, Vec<Light>>,
    /// Texture slots (record material, offset) the game has set though the
    /// cluster leaves them empty (the texture animation's blank layers):
    /// bound as the host's [`External::MaterialTexture`].
    pub material_textures: std::collections::HashSet<(usize, u32)>,
}

/// A light as the node context's shader parameters take it: the words of
/// its `m_direction` and `m_colorIntensity` (a parameter copies its
/// definition's size from them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Light {
    pub direction: [u32; 4],
    pub color: [u32; 4],
}

/// The lights a character instance's node context has: the character
/// lighting's three directional `PLight`s (`Phyre_CharacterLighting`,
/// `docs/formats/scene-chars.md`). Every captured character draw binds
/// the 3-light context's shader.
pub const CHAR_LIGHTS: u32 = 3;

/// Where a built draw's constant buffers take a light's value: stage
/// (vertex or not), byte offset and size in the stage's `$Globals`, the
/// light and whether it is the direction (else the colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightSlot {
    pub vertex: bool,
    pub target: u32,
    pub size: u32,
    pub light: u32,
    pub direction: bool,
}

/// A texture slot of a draw's material that the cluster leaves empty:
/// stage, register, record material and the slot's offset (the game may
/// set a texture there at run time: [`FrameInputs::material_textures`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSlot {
    pub vertex: bool,
    pub register: u32,
    pub material: usize,
    pub offset: u32,
}

/// What a built draw took from where, for inputs read back from a
/// capture and for the gate.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrawTrace {
    pub lights: Vec<LightSlot>,
    pub empty_textures: Vec<TextureSlot>,
    /// Byte ranges of a stage's `$Globals` that a location entry covers
    /// but the copy rule left unwritten (after the last bone of
    /// `BoneTransforms`, [`chars::cb_copy`]): stage (vertex or not) and
    /// range.
    pub unwritten: Vec<(bool, Range<u32>)>,
}

/// The node context's buffer of a shader (`m_parameterDefinitionsForPasses`,
/// `m_parameterBufferSize` bytes of the heap's fill): the combined
/// attenuation and spot coefficients (0 for `*Coeff0`, 1 for `*Coeff1`:
/// the directional lights' values on every captured draw) and the
/// lights' direction and colour.
fn shader_buffer(
    size: u32,
    params: &[chars_model::ShaderParam],
    lights: &[Light],
    notes: &mut Vec<String>,
) -> Vec<u8> {
    let mut buf = vec![0xcd_u8; size as usize];
    for p in params {
        let words: [u32; 4] = match p.kind {
            0xcf | 0xd1 => [0; 4],
            0xd0 | 0xd2 => [1f32.to_bits(); 4],
            0xcb | 0xc2 => match lights.get(p.light as usize) {
                Some(l) if p.kind == 0xcb => l.direction,
                Some(l) => l.color,
                None => {
                    notes.push(format!("{}: no light {}", p.name, p.light));
                    continue;
                }
            },
            k => {
                notes.push(format!("shader parameter {} of kind {k:#x} left", p.name));
                continue;
            }
        };
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let (o, n) = (p.offset as usize, (p.size as usize).min(16));
        if let Some(d) = buf.get_mut(o..o + n) {
            d.copy_from_slice(&bytes[..n]);
        }
    }
    buf
}

/// A constant buffer of `size` bytes (the heap's `0xcd` fill) filled
/// through a location table with the render thread's copy
/// ([`chars::cb_copy`]): the scene-wide, material, per-node and node
/// context (location group 4, the shader's own parameters) buffers.
/// Also gives the ranges an entry covers that its copy left unwritten.
fn fill_cb(
    size: u32,
    table: &LocTable,
    sources: [&[u8]; 4],
    notes: &mut Vec<String>,
) -> (Vec<u8>, Vec<Range<u32>>) {
    let mut cb = vec![0xcd_u8; size as usize];
    let mut unwritten = Vec::new();
    for (src, loc) in table.sourced() {
        if loc.target == u32::MAX {
            continue;
        }
        let from = match src {
            Source::SceneWide => sources[0],
            Source::Material => sources[1],
            Source::Node => sources[2],
            Source::Other => sources[3],
        };
        let (o, t, n) = (
            usize::from(loc.offset),
            loc.target as usize,
            loc.size as usize,
        );
        let Some(s) = from.get(o..o + n) else {
            notes.push(format!(
                "{src:?} parameter at {o:#x} ({n} bytes): outside its buffer"
            ));
            continue;
        };
        let w = chars::cb_copy(loc.kind, s);
        match cb.get_mut(t..t + w.len()) {
            Some(d) => d.copy_from_slice(&w),
            None => notes.push(format!("{src:?} parameter to {t:#x}: outside the cb")),
        }
        if w.len() < n {
            unwritten.push(build::handle(t + w.len())..build::handle(t + n));
        }
    }
    (cb, unwritten)
}

/// The inverse of [`chars::cb_copy`] on whole entries: a buffer whose
/// matrices are in the shaders' order (as inputs read back from captured
/// constant buffers have them) made the game's (memory order), for the
/// scene-wide entries of `tables` of kind `'1'` and `'0'`. Other bytes are
/// kept.
#[must_use]
pub fn scene_wide_to_memory<'a>(
    cb_order: &[u8],
    tables: impl IntoIterator<Item = &'a LocTable>,
) -> Vec<u8> {
    let mut out = cb_order.to_vec();
    let mut done = std::collections::HashSet::new();
    for table in tables {
        for (src, loc) in table.sourced() {
            if src != Source::SceneWide || !done.insert(loc.offset) {
                continue;
            }
            let (stride, order): (usize, fn(usize) -> usize) = match loc.kind {
                0x31 => (16, |i| (i % 4) * 4 + i / 4),
                0x30 => (12, |i| (i % 4) * 3 + i / 4),
                _ => continue,
            };
            let o = usize::from(loc.offset);
            for e in 0..loc.size as usize / (4 * stride) {
                let at = o + e * 4 * stride;
                let Some(src) = cb_order.get(at..at + 4 * stride) else {
                    break;
                };
                // cb word i came from memory word order(i).
                for i in 0..stride {
                    let m = at + 4 * order(i);
                    out[m..m + 4].copy_from_slice(&src[4 * i..4 * i + 4]);
                }
            }
        }
    }
    out
}

struct Builder<'a> {
    c: &'a CharClusters,
    frame: &'a FrameInputs,
    dropped: u32,
    out: Built,
    /// Per draw of `out`: what it took from where.
    traces: Vec<DrawTrace>,
}

fn dynamic_buffer(bind: u32, data: &[u8]) -> BufferDesc {
    BufferDesc {
        size: build::handle(data.len()),
        usage: ffx_draw::desc::USAGE_DYNAMIC,
        bind,
        cpu_access: ffx_draw::desc::CPU_WRITE,
        misc: 0,
        stride: 0,
    }
}

/// One draw's source: the record, the instance and segment in it, its
/// role.
#[derive(Clone, Copy)]
struct Src<'a> {
    s: &'a Record,
    pass: Pass,
    instance: usize,
    segment: usize,
    material: usize,
    role: Role,
}

impl Builder<'_> {
    /// The visible instances of a pass, in draw order: per instance list
    /// of the walk a job (runs of 400), culled with the camera's `+0x70`
    /// matrix, keyed on the view's z row, sorted, merged.
    fn visible(s: &Record, opaque: bool) -> Option<Vec<usize>> {
        let cull_vp = inputs::transpose(&inputs::cull_view_projection(s)?);
        let view = inputs::view(s)?;
        let mut jobs: Vec<Vec<usize>> = Vec::new();
        let mut last = None;
        for (k, b) in s.bounds.iter().enumerate() {
            match jobs.last_mut() {
                Some(j) if last == Some(b.list) && j.len() < 400 => j.push(k),
                _ => jobs.push(vec![k]),
            }
            last = Some(b.list);
        }
        let mut runs = Vec::new();
        for job in jobs {
            let mut entries = Vec::new();
            for k in job {
                let b = &s.bounds[k];
                let Some(inst) = b.instance else { continue };
                let world = b.world.as_ref().map(|m| inputs::world(Some(m)));
                let m = crate::cull::cull_matrix(&cull_vp, world.as_ref());
                if !crate::cull::box_visible(b.min, b.size, &m) {
                    continue;
                }
                let centre = crate::cull::centre(b.min, b.size, world.as_ref());
                let bias = inputs::depth_bias(s, u32::try_from(k).unwrap_or(u32::MAX));
                entries.push(Entry {
                    instance: inst as usize,
                    key: crate::cull::depth_key(&view, opaque, centre, bias),
                    render_order: b.render_order,
                });
            }
            sort::sort(&mut entries);
            if !entries.is_empty() {
                runs.push(entries);
            }
        }
        Some(sort::merge(&runs))
    }

    fn instance(&mut self, s: &Record, pass: Pass, index: usize, role: Role) -> Result<(), Error> {
        let Some(inst) = s.instances.get(index) else {
            return Ok(());
        };
        let c = self.c;
        for bucket in 0..4 {
            for (k, seg) in inst.segments.iter().enumerate() {
                if seg.render_order != bucket {
                    continue;
                }
                // A null material ends the instance.
                let Some(material) = inst.materials.get(seg.material as usize).copied().flatten()
                else {
                    return Ok(());
                };
                let live = s
                    .materials
                    .get(material as usize)
                    .ok_or_else(|| err(format!("record material {material} missing")))?;
                let (variant, contexts, file_material, lights) = match role {
                    Role::Model(fi) => {
                        let segment = c.map.instances[fi].segments[k];
                        let m = c.map.material(fi, segment).ok_or_else(|| {
                            err(format!("file instance {fi} segment {k}: no material"))
                        })?;
                        let effect = &c.map.materials[m].effect;
                        let v = c
                            .effects
                            .get(effect)
                            .ok_or_else(|| err(format!("effect {effect} not loaded")))?;
                        let cx = c
                            .contexts
                            .get(effect)
                            .ok_or_else(|| err(format!("effect {effect}: no contexts")))?;
                        (v, cx, Some(m), CHAR_LIGHTS)
                    }
                    Role::Shadow => match &c.shadow {
                        Some(sh) => (&sh.variant, &sh.contexts, None, 0),
                        None => {
                            self.note("the shadow material is not loaded: shadow not drawn");
                            return Ok(());
                        }
                    },
                    Role::Unknown => return Ok(()),
                };
                let pass_name = s.pass.as_str();
                let remap = |x: &str| (!x.is_empty()).then(|| x.to_owned());
                let (from, to) = (remap(&live.remap_from), remap(&live.remap_to));
                if pass_name == "Opaque"
                    && from.as_deref() == Some("Transparent")
                    && to.as_deref() != Some("Transparent")
                {
                    continue;
                }
                let pass_type = if from.as_deref() == Some(pass_name) {
                    to.as_deref().unwrap_or(pass_name)
                } else {
                    pass_name
                };
                let Some(rp) = variant.passes.iter().position(|p| p.pass_type == pass_type) else {
                    continue;
                };
                // The node context's shader (`0x004ef440`).
                let context = contexts.context(lights);
                let Some(shader) = variant.passes[rp].shaders.get(context) else {
                    continue;
                };
                let (size, params) = contexts
                    .shaders
                    .get(rp)
                    .and_then(|s| s.get(context))
                    .map_or((0, &[][..]), |(n, p)| (*n, p.as_slice()));
                if seg.hidden != 0 {
                    continue;
                }
                let src = Src {
                    s,
                    pass,
                    instance: index,
                    segment: k,
                    material: material as usize,
                    role,
                };
                for sp in &shader.passes {
                    self.draw(src, variant, file_material, (size, params), sp)?;
                }
            }
        }
        Ok(())
    }

    fn note(&mut self, n: impl Into<String>) {
        let n = n.into();
        if !self.out.notes.contains(&n) {
            self.out.notes.push(n);
        }
    }

    /// The input layout and vertex bindings of a draw.
    #[allow(clippy::too_many_lines)] // the model and the shadow streams
    fn vertex(
        &mut self,
        src: Src<'_>,
        sp: &ShaderPass,
        state: &mut DrawState,
    ) -> Result<u32, Error> {
        let c = self.c;
        let res = &mut self.out.resources;
        let vs = res.shader(Stage::Vertex, &sp.vs.code);
        let cw = src.s.chars.as_deref();
        let mut elements = Vec::new();
        let count = match src.role {
            Role::Model(fi) => {
                let segment = c.map.instances[fi].segments[src.segment];
                let seg = &c.map.segments[segment];
                let mut notes = Vec::new();
                let inputs = build::vertex_inputs(&c.map, fi, segment, &sp.vs, &mut notes);
                let cloth = cw.and_then(|w| {
                    w.cloth.iter().find(|x| {
                        x.instance as usize == src.instance && x.segment as usize == src.segment
                    })
                });
                for ((e, block), input) in inputs.into_iter().zip(&sp.vs.inputs) {
                    let is_cloth = input.render_type == CLOTH_RENDER_TYPE.0
                        && input.index == CLOTH_RENDER_TYPE.1;
                    let binding = match (block, cloth) {
                        (_, Some(x)) if is_cloth => {
                            let desc = dynamic_buffer(ffx_draw::desc::BIND_VERTEX, &x.vertices);
                            Some(VertexBinding {
                                buffer: res.buffer(desc, x.vertices.clone()),
                                stride: 16,
                                offset: 0,
                            })
                        }
                        (Some(b), _) => Some(VertexBinding {
                            buffer: res.static_buffer(&c.map, segment, Some(b)),
                            stride: build::handle(seg.blocks[b].stride),
                            offset: 0,
                        }),
                        (None, _) => {
                            notes.push(format!(
                                "segment {segment}: no {} stream for {}{}",
                                input.render_type, input.semantic, input.index
                            ));
                            None
                        }
                    };
                    if let (Some(v), Some(slot)) = (binding, state.vertex.get_mut(e.slot as usize))
                    {
                        *slot = Some(v);
                    }
                    elements.push(e);
                }
                for n in notes {
                    self.note(n);
                }
                let res = &mut self.out.resources;
                state.index = Some(IndexBinding {
                    buffer: res.static_buffer(&c.map, segment, None),
                    format: 57,
                    offset: 0,
                });
                seg.index_count
            }
            Role::Shadow => {
                let g = cw
                    .and_then(|w| w.dyn_geo.as_ref())
                    .ok_or_else(|| err("shadow: no dynamic geometry in the record"))?;
                for input in &sp.vs.inputs {
                    let k = input.slot as usize;
                    let data = g
                        .arrays
                        .get(k)
                        .ok_or_else(|| err(format!("shadow: no dynamic array for slot {k}")))?;
                    let stride = [12, 12, 16, 8][k];
                    let desc = dynamic_buffer(ffx_draw::desc::BIND_VERTEX, data);
                    let buffer = res.buffer(desc, data.clone());
                    if let Some(slot) = state.vertex.get_mut(k) {
                        *slot = Some(VertexBinding {
                            buffer,
                            stride,
                            offset: 0,
                        });
                    }
                    elements.push(InputElement {
                        semantic: input.semantic.clone(),
                        index: input.index,
                        format: input.format,
                        slot: input.slot,
                        offset: 0,
                        per_instance: false,
                        step: 0,
                    });
                }
                // The lists' u16 indices, widened into the index ring.
                let wide: Vec<u8> = g
                    .index_bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .flat_map(|w| u32::from(u16::from_le_bytes(*w)).to_le_bytes())
                    .collect();
                let desc = dynamic_buffer(ffx_draw::desc::BIND_INDEX, &wide);
                state.index = Some(IndexBinding {
                    buffer: res.buffer(desc, wide),
                    format: 42,
                    offset: 0,
                });
                g.indices
            }
            Role::Unknown => 0,
        };
        let res = &mut self.out.resources;
        state.layout = Some(res.layout(elements, vs));
        Ok(count)
    }

    #[allow(clippy::too_many_lines)] // one draw's every binding
    fn draw(
        &mut self,
        src: Src<'_>,
        variant: &EffectVariant,
        file_material: Option<usize>,
        context: (u32, &[chars_model::ShaderParam]),
        sp: &ShaderPass,
    ) -> Result<(), Error> {
        let mut state = DrawState {
            topology: 4,
            sample_mask: u32::MAX,
            stencil_ref: u32::from(sp.state.stencil_ref),
            viewport: self.frame.viewport,
            scissor: self.frame.scissor,
            ..DrawState::default()
        };
        if let Role::Model(fi) = src.role {
            let seg = &self.c.map.segments[self.c.map.instances[fi].segments[src.segment]];
            if seg.primitive != 2 {
                return Err(err(format!("primitive type {}", seg.primitive)));
            }
        }
        let count = self.vertex(src, sp, &mut state)?;
        let res = &mut self.out.resources;
        let vs = res.shader(Stage::Vertex, &sp.vs.code);
        let ps = res.shader(Stage::Pixel, &sp.ps.code);
        let blend = BlendDesc::from_d3d11(&sp.state.blend).map_err(|e| err(e.to_string()))?;
        state.blend = Some(Blend(build::intern(&mut res.blends, &blend)));
        let depth =
            DepthStencilDesc::from_d3d11(&sp.state.depth).map_err(|e| err(e.to_string()))?;
        state.depth = Some(DepthState(build::intern(&mut res.depths, &depth)));
        let raster = RasterDesc::from_d3d11(&sp.state.raster).map_err(|e| err(e.to_string()))?;
        state.raster = Some(Raster(build::intern(&mut res.rasters, &raster)));
        state.targets[0] = Some(res.external(External::Color, ViewKind::RenderTarget));
        state.targets[1] = Some(res.external(External::Normal, ViewKind::RenderTarget));
        state.depth_target = Some(res.external(External::Depth, ViewKind::DepthStencil));

        // Constant buffers.
        let node_in = chars::node_inputs(src.s, src.instance, src.segment).ok_or_else(|| {
            err(format!(
                "instance {} segment {}: no node inputs",
                src.instance, src.segment
            ))
        })?;
        let (node, node_notes) = chars::node_buffer(variant, &node_in);
        for n in node_notes {
            self.note(n);
        }
        let material = &src.s.materials[src.material].parameters;
        let mut notes = Vec::new();
        let lights = self
            .frame
            .lights
            .get(&src.instance)
            .map_or(&[][..], Vec::as_slice);
        let context_buffer = shader_buffer(context.0, context.1, lights, &mut notes);
        let mut trace = DrawTrace::default();
        let mut stages = [StageBindings::default(), StageBindings::default()];
        for (vertex, (b, (prog, params, textures, shader))) in
            [true, false].into_iter().zip(stages.iter_mut().zip([
                (&sp.vs, &sp.vs_params, &sp.vs_textures, vs),
                (&sp.ps, &sp.ps_params, &sp.ps_textures, ps),
            ]))
        {
            b.shader = Some(shader);
            if prog.global_cb != u32::MAX {
                for (s, loc) in params.sourced() {
                    let p = context.1.iter().find(|p| p.offset == u32::from(loc.offset));
                    if let (Source::Other, Some(p), true) = (s, p, loc.target != u32::MAX)
                        && matches!(p.kind, 0xc2 | 0xcb)
                    {
                        trace.lights.push(LightSlot {
                            vertex,
                            target: loc.target,
                            size: loc.size.min(p.size),
                            light: p.light,
                            direction: p.kind == 0xcb,
                        });
                    }
                }
                let (data, unwritten) = fill_cb(
                    prog.cb_size,
                    params,
                    [&self.frame.scene_wide, material, &node, &context_buffer],
                    &mut notes,
                );
                trace
                    .unwritten
                    .extend(unwritten.into_iter().map(|r| (vertex, r)));
                let desc = BufferDesc {
                    size: prog.cb_size,
                    usage: ffx_draw::desc::USAGE_DYNAMIC,
                    bind: ffx_draw::desc::BIND_CONSTANT,
                    cpu_access: ffx_draw::desc::CPU_WRITE,
                    misc: 0,
                    stride: 0,
                };
                let buffer: Buffer = self.out.resources.buffer(desc, data);
                if let Some(slot) = b.cbs.get_mut(prog.global_cb as usize) {
                    *slot = Some(CbBinding {
                        buffer,
                        first: 0,
                        count: 0,
                    });
                }
            }
            let slots = self.bind_textures(textures, variant, file_material, src.material, b)?;
            trace
                .empty_textures
                .extend(slots.into_iter().map(|(register, offset)| TextureSlot {
                    vertex,
                    register,
                    material: src.material,
                    offset,
                }));
        }
        for n in notes {
            self.note(n);
        }
        let [vsb, psb] = stages;
        state.vs = vsb;
        state.ps = psb;
        self.traces.push(trace);
        self.out.draws.push(MapDraw {
            pass: src.pass,
            instance: src.instance,
            segment: src.segment,
            material: src.material,
            state,
            call: DrawCall::Indexed {
                count,
                start: 0,
                base: 0,
            },
            // Bound as constant buffers in the resources (above).
            constants: Vec::new(),
        });
        Ok(())
    }

    /// Textures and samplers through a texture location table: a
    /// material sampler (0x42, 0x44) puts the file material's texture in
    /// register `target`; a slot the file leaves empty binds the host's
    /// [`External::MaterialTexture`] of the record material when the game
    /// set one ([`FrameInputs::material_textures`]; the shadow's always),
    /// else nothing; a sampler state (0x47) its state in sampler
    /// `target`. Gives the empty slots (register, offset).
    fn bind_textures(
        &mut self,
        table: &LocTable,
        variant: &EffectVariant,
        file_material: Option<usize>,
        record_material: usize,
        b: &mut StageBindings,
    ) -> Result<Vec<(u32, u32)>, Error> {
        let mut empty = Vec::new();
        let c = self.c;
        let mat = file_material.map(|m| &c.map.materials[m]);
        let samplers: &[(u32, SamplerState)] = match (mat, &c.shadow) {
            (Some(m), _) => &m.samplers,
            (None, Some(sh)) => &sh.samplers,
            (None, None) => &[],
        };
        for (src, loc) in table.sourced() {
            if loc.target == u32::MAX {
                continue;
            }
            let reg = loc.target as usize;
            if src != Source::Material {
                self.note(format!(
                    "{src:?} texture slot at {:#x} used (register {reg}): not supported",
                    loc.offset
                ));
                continue;
            }
            let Some(def) = variant.tweakable_at(loc.offset) else {
                self.note(format!("no tweakable at {:#x}", loc.offset));
                continue;
            };
            let offset = u32::from(loc.offset);
            match def.kind {
                0x42 | 0x44 => {
                    let file = mat.and_then(|m| m.textures.iter().find(|t| t.offset == offset));
                    let set = mat.is_none()
                        || self
                            .frame
                            .material_textures
                            .contains(&(record_material, offset));
                    let view = match file {
                        Some(t) => {
                            let tex = c
                                .textures
                                .get(&t.import)
                                .ok_or_else(|| err(format!("texture {} not loaded", t.import)))?;
                            self.out.resources.texture(&t.import, tex, self.dropped)?
                        }
                        None => {
                            empty.push((loc.target, offset));
                            if !set {
                                // Nothing in the slot: nothing bound.
                                continue;
                            }
                            self.out.resources.external(
                                External::MaterialTexture {
                                    material: record_material,
                                    offset,
                                },
                                ViewKind::ShaderResource,
                            )
                        }
                    };
                    if let Some(slot) = b.srvs.get_mut(reg) {
                        *slot = Some(view);
                    }
                }
                0x47 => {
                    let Some((_, s)) = samplers.iter().find(|s| s.0 == offset) else {
                        self.note(format!(
                            "no sampler state at {offset:#x} ({}) of material {record_material}",
                            def.name
                        ));
                        continue;
                    };
                    let desc = build::sampler_desc(s, self.frame.settings.anisotropy);
                    let h = Sampler(build::intern(&mut self.out.resources.samplers, &desc));
                    if let Some(slot) = b.samplers.get_mut(reg) {
                        *slot = Some(h);
                    }
                }
                k => self.note(format!("texture table entry {} of kind {k:#x}", def.name)),
            }
        }
        Ok(empty)
    }
}

/// Builds the character world's draws for one frame from its `Opaque`
/// and `Transparent` records (`renderWorld` of world `+0x7c`, with the
/// character part).
///
/// # Errors
/// On a record without the camera, a missing effect or texture, or data
/// the builder doesn't handle (each named).
pub fn build(c: &CharClusters, records: [&Record; 2], frame: &FrameInputs) -> Result<Built, Error> {
    build_traced(c, records, frame).map(|b| b.0)
}

/// [`build`], with per draw where its constant buffers take the lights
/// ([`DrawTrace`]).
///
/// # Errors
/// As [`build`].
pub fn build_traced(
    c: &CharClusters,
    records: [&Record; 2],
    frame: &FrameInputs,
) -> Result<(Built, Vec<DrawTrace>), Error> {
    let mut b = Builder {
        c,
        frame,
        dropped: frame.settings.texture_quality.dropped_levels(),
        out: Built::default(),
        traces: Vec::new(),
    };
    for (s, pass) in records.into_iter().zip([Pass::Opaque, Pass::Transparent]) {
        if s.chars.is_none() {
            return Err(err(format!("pass {}: no character part", s.pass)));
        }
        let roles = resolve(c, s);
        for (k, r) in roles.iter().enumerate() {
            if *r == Role::Unknown {
                b.note(format!(
                    "pass {}: record instance {k} matches no loaded model",
                    s.pass
                ));
            }
        }
        let order = Builder::visible(s, pass == Pass::Opaque)
            .ok_or_else(|| err(format!("pass {}: no camera", s.pass)))?;
        for i in order {
            let role = roles.get(i).copied().unwrap_or(Role::Unknown);
            b.instance(s, pass, i, role)?;
        }
    }
    Ok((b.out, b.traces))
}

/// The `Opaque` and `Transparent` character-world records of a frame
/// (not the `post*` calls).
#[must_use]
pub fn pass_records(all: &[Record], frame: u32) -> Option<[&Record; 2]> {
    let recs = chars::records(all, frame);
    let find = |name: &str| recs.iter().copied().find(|s| s.pass == name);
    Some([find("Opaque")?, find("Transparent")?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Loc;

    #[test]
    fn matrices_go_back_to_memory_order() {
        let table = LocTable {
            starts: [0, 1, 1, 1],
            counts: [1, 0, 0, 0],
            entries: vec![Loc {
                offset: 0,
                target: 0,
                size: 64,
                kind: 0x31,
            }],
        };
        let memory: Vec<u8> = (0..16u32)
            .flat_map(|i| f32::from(u16::try_from(i).unwrap_or(0)).to_le_bytes())
            .collect();
        let cb = chars::cb_copy(0x31, &memory);
        assert_ne!(cb, memory);
        assert_eq!(scene_wide_to_memory(&cb, [&table]), memory);
        // and the fill copies it back
        let mut notes = Vec::new();
        let (out, unwritten) = fill_cb(64, &table, [&memory, &[], &[], &[]], &mut notes);
        assert_eq!(out, cb);
        assert!(notes.is_empty() && unwritten.is_empty());
    }

    #[test]
    fn a_bone_list_stops_at_the_marker() {
        let table = LocTable {
            starts: [0, 0, 0, 1],
            counts: [0, 0, 1, 0],
            entries: vec![Loc {
                offset: 0,
                target: 0,
                size: 128,
                kind: 0x31,
            }],
        };
        let mut node = vec![0u8; 128];
        node[64..68].copy_from_slice(&[0xff; 4]);
        let mut notes = Vec::new();
        let (out, unwritten) = fill_cb(128, &table, [&[], &[], &node, &[]], &mut notes);
        // the second matrix keeps the fill
        assert_eq!(&out[..64], &[0u8; 64][..]);
        assert_eq!(&out[64..], &[0xcd_u8; 64][..]);
        assert_eq!(unwritten, vec![64..128]);
    }
}
