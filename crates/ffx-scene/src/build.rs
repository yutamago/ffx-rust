//! The scene builder for the map world (scene `+0xac`): `build(clusters,
//! inputs)` gives the map's draws of the opaque and the transparent pass
//! as `ffx-draw` [`DrawState`]s and [`DrawCall`]s, with the objects they
//! use in [`Resources`] (`docs/scene-builder.md`).
//!
//! Per pass, as `PApplication_renderWorld` (`0x0062be70`) →
//! `renderWorldCulledSorted` (`0x009f3c30`) does it:
//!
//! 1. cull and key every `PMeshInstanceBounds` in jobs of 400
//!    ([`crate::cull`], `PCheckVisibleBoundsJob_execute` `0x009f3160`),
//!    sort each job's entries and merge the jobs ([`crate::sort`]);
//! 2. draw each instance (`renderMeshInstance` `0x005b9a90`): its
//!    segments in four buckets by `m_segmentRenderOrder`, a null material
//!    ending the instance, the material's pass remapping, the effect
//!    variant's render pass for the pass (`selectTechniquePass`
//!    `0x005ba690`), the hidden byte; one draw per shader pass;
//! 3. per draw: the pass's programs, input layout and states, the
//!    segment's buffers, the material's textures and samplers, and the
//!    constant buffers filled through the pass's location tables
//!    (`allocateAndFillParameterBuffers` `0x005b5a50`).

// Draw building names its handles and buffers (b, c, d, s, ...).
#![allow(clippy::many_single_char_names)]

use std::collections::HashMap;

use ffx_draw::backend::{
    Blend, Buffer, DepthState, Layout, Raster, Sampler, Shader, Texture, View,
};
use ffx_draw::desc::{
    BlendDesc, BufferDesc, DepthStencilDesc, InputElement, RasterDesc, SamplerDesc, Stage,
    TextureDesc, ViewDesc, ViewKind,
};
use ffx_draw::state::{CbBinding, IndexBinding, StageBindings, VertexBinding};
use ffx_draw::{DrawCall, DrawState};

use crate::census::{mul, mul_f64};
use crate::effect::{EffectVariant, LocTable, Program, ShaderPass, Source};
use crate::frame::{Camera, External, Mat, SceneInputs, SlotTexture};
use crate::map::{self, MapModel};
use crate::sort::{self, Entry};

/// A build error: what was being built and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(what: impl Into<String>) -> Error {
    Error(what.into())
}

/// The static data of a map: its models (one per cluster of the world,
/// appended into one [`MapModel`]) and every effect variant and texture
/// their materials import.
#[derive(Debug, Clone)]
pub struct Clusters {
    pub map: MapModel,
    /// Per model: its archive path and its instances in [`Self::map`].
    pub models: Vec<(String, std::ops::Range<usize>)>,
    /// By import id (`PS3Data/Shaders/X.fx#MD5`).
    pub effects: HashMap<String, EffectVariant>,
    /// By import id (`PS3Data/map/.../tex/x.dds`).
    pub textures: HashMap<String, ffx_phyre::Texture>,
}

impl Clusters {
    /// Loads a map model and what it imports through `read` (archive path
    /// → bytes; the archive, or the mod folder first as the hook does).
    ///
    /// # Errors
    /// When the model, an effect or a texture is missing or doesn't parse.
    pub fn load(read: impl Fn(&str) -> Option<Vec<u8>>, model_path: &str) -> Result<Self, Error> {
        Self::load_models(read, &[model_path])
    }

    /// Loads several models (a world's clusters, in its order) into one.
    ///
    /// # Errors
    /// As [`Self::load`].
    pub fn load_models(
        read: impl Fn(&str) -> Option<Vec<u8>>,
        model_paths: &[&str],
    ) -> Result<Self, Error> {
        let mut out = Self {
            map: MapModel {
                instances: Vec::new(),
                segments: Vec::new(),
                materials: Vec::new(),
                bytes: Vec::new(),
            },
            models: Vec::new(),
            effects: HashMap::new(),
            textures: HashMap::new(),
        };
        for &path in model_paths {
            let bytes = read(path).ok_or_else(|| err(format!("{path}: not found")))?;
            let model = MapModel::parse(bytes).map_err(|e| err(format!("{path}: {e}")))?;
            let n = model.instances.len();
            let base = out.map.append(model);
            out.models.push((path.to_owned(), base..base + n));
        }
        for m in &out.map.materials {
            if !out.effects.contains_key(&m.effect) {
                let path = map::archive_path(&m.effect);
                let bytes = read(&path).ok_or_else(|| err(format!("{path}: not found")))?;
                let v = EffectVariant::parse(bytes).map_err(|e| err(format!("{path}: {e}")))?;
                out.effects.insert(m.effect.clone(), v);
            }
        }
        let imports: Vec<String> = out
            .map
            .materials
            .iter()
            .flat_map(|m| m.textures.iter().map(|t| t.import.clone()))
            .collect();
        out.add_textures(&read, &imports)?;
        Ok(out)
    }

    /// Loads texture clusters by import id (those the game sets into
    /// materials at run time: texture animation), skipping loaded ones.
    ///
    /// # Errors
    /// When a texture is missing or doesn't parse.
    pub fn add_textures(
        &mut self,
        read: impl Fn(&str) -> Option<Vec<u8>>,
        imports: &[String],
    ) -> Result<(), Error> {
        for import in imports {
            if self.textures.contains_key(import) {
                continue;
            }
            let path = map::archive_path(import);
            let bytes = read(&path).ok_or_else(|| err(format!("{path}: not found")))?;
            let tex = ffx_phyre::Texture::parse(bytes).map_err(|e| err(format!("{path}: {e}")))?;
            self.textures.insert(import.clone(), tex);
        }
        Ok(())
    }

    /// The model loaded from `path` (an archive path), by index.
    #[must_use]
    pub fn model_of(&self, path: &str) -> Option<usize> {
        self.models
            .iter()
            .position(|m| m.0.eq_ignore_ascii_case(path))
    }
}

/// A texture's description and its subresources (mips of slice 0, then
/// of slice 1, ...): row pitch and bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureData {
    pub desc: TextureDesc,
    pub subresources: Vec<(u32, Vec<u8>)>,
}

/// What a view shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewSource {
    Texture(Texture),
    /// The host's: a scene target or a texture the game sets at run time.
    External(External),
}

/// A view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewData {
    pub source: ViewSource,
    pub kind: ViewKind,
    pub desc: Option<ViewDesc>,
}

/// The objects the draws use; handles index these lists.
#[derive(Debug, Default, Clone)]
pub struct Resources {
    pub shaders: Vec<(Stage, Vec<u8>)>,
    pub layouts: Vec<(Vec<InputElement>, Shader)>,
    pub blends: Vec<BlendDesc>,
    pub depths: Vec<DepthStencilDesc>,
    pub rasters: Vec<RasterDesc>,
    pub samplers: Vec<SamplerDesc>,
    pub buffers: Vec<(BufferDesc, Vec<u8>)>,
    pub textures: Vec<TextureData>,
    pub views: Vec<ViewData>,
    shader_ix: HashMap<(Stage, Vec<u8>), Shader>,
    layout_ix: HashMap<(Vec<InputElement>, Shader), Layout>,
    static_ix: HashMap<(usize, usize, bool), Buffer>,
    /// Vertex buffers the host provides (`External::DynamicVertices`),
    /// by handle: in `buffers` with a dynamic description and no data.
    pub external_buffers: HashMap<u32, External>,
    texture_ix: HashMap<String, View>,
    external_ix: HashMap<(External, ViewKind), View>,
    external_buffer_ix: HashMap<External, Buffer>,
}

fn handle(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn intern<T: PartialEq + Clone>(list: &mut Vec<T>, v: &T) -> u32 {
    if let Some(i) = list.iter().position(|x| x == v) {
        return handle(i);
    }
    list.push(v.clone());
    handle(list.len() - 1)
}

impl Resources {
    fn shader(&mut self, stage: Stage, code: &[u8]) -> Shader {
        let key = (stage, code.to_vec());
        if let Some(&s) = self.shader_ix.get(&key) {
            return s;
        }
        self.shaders.push(key.clone());
        let s = Shader(handle(self.shaders.len() - 1));
        self.shader_ix.insert(key, s);
        s
    }

    fn layout(&mut self, elements: Vec<InputElement>, vs: Shader) -> Layout {
        let key = (elements, vs);
        if let Some(&l) = self.layout_ix.get(&key) {
            return l;
        }
        self.layouts.push(key.clone());
        let l = Layout(handle(self.layouts.len() - 1));
        self.layout_ix.insert(key, l);
        l
    }

    fn buffer(&mut self, desc: BufferDesc, data: Vec<u8>) -> Buffer {
        self.buffers.push((desc, data));
        Buffer(handle(self.buffers.len() - 1))
    }

    /// The game's static buffer of a segment's block (`block` index) or
    /// its indices (`None`): `D3D11_USAGE_IMMUTABLE`, bind flags 9 or 10.
    fn static_buffer(&mut self, map: &MapModel, segment: usize, block: Option<usize>) -> Buffer {
        let key = (segment, block.unwrap_or(0), block.is_none());
        if let Some(&b) = self.static_ix.get(&key) {
            return b;
        }
        let seg = &map.segments[segment];
        let range = match block {
            Some(k) => seg.blocks[k].bytes.clone(),
            None => seg.indices.clone(),
        };
        let data = map.bytes.get(range).unwrap_or_default().to_vec();
        let w = crate::key::static_buffer_desc(data.len(), block.is_none());
        let b = self.buffer(BufferDesc::from_words(w), data);
        self.static_ix.insert(key, b);
        b
    }

    /// A vertex buffer the host fills (the dynamic ring's vertices of a
    /// morphing mesh): a dynamic description of the block's size.
    fn external_buffer(&mut self, e: External, len: usize) -> Buffer {
        if let Some(&b) = self.external_buffer_ix.get(&e) {
            return b;
        }
        let desc = BufferDesc {
            size: handle(len),
            usage: ffx_draw::desc::USAGE_DYNAMIC,
            bind: 1,
            cpu_access: ffx_draw::desc::CPU_WRITE,
            misc: 0,
            stride: 0,
        };
        let b = self.buffer(desc, Vec::new());
        self.external_buffers.insert(b.0, e);
        self.external_buffer_ix.insert(e, b);
        b
    }

    fn external(&mut self, e: External, kind: ViewKind) -> View {
        if let Some(&v) = self.external_ix.get(&(e, kind)) {
            return v;
        }
        self.views.push(ViewData {
            source: ViewSource::External(e),
            kind,
            desc: None,
        });
        let v = View(handle(self.views.len() - 1));
        self.external_ix.insert((e, kind), v);
        v
    }

    /// The shader resource view of a texture cluster as the game makes
    /// it: the file's chain from mip `dropped` on (never past the last),
    /// DXT1/3/5 as BC1/2/3, cube maps as 6 slices; a `TEXTURE2D` or
    /// `TEXTURECUBE` view of every level.
    fn texture(
        &mut self,
        import: &str,
        t: &ffx_phyre::Texture,
        dropped: u32,
    ) -> Result<View, Error> {
        if let Some(&v) = self.texture_ix.get(import) {
            return Ok(v);
        }
        let cube = t.kind == ffx_phyre::Kind::CubeMap;
        let faces: u32 = if cube { 6 } else { 1 };
        let first = crate::assets::texture_start_level(t, dropped);
        let format = crate::key::dxgi_format(t.format);
        let dims = |l: u32| ((t.width >> l).max(1), (t.height >> l).max(1));
        let size = |l: u32| {
            let (w, h) = dims(l);
            t.format.level_size(w, h)
        };
        let chain: usize = (0..t.mip_levels).map(size).sum();
        let mips = t.mip_levels - first;
        let mut subresources = Vec::new();
        if !cube {
            subresources = upload_subresources(t, first, format)
                .ok_or_else(|| err(format!("{import}: format {format}")))?;
        }
        for face in 0..if cube { faces as usize } else { 0 } {
            let mut at = face * chain + (0..first).map(size).sum::<usize>();
            for l in first..t.mip_levels {
                let len = size(l);
                let data = t
                    .pixels()
                    .get(at..at + len)
                    .ok_or_else(|| err(format!("{import}: level {l} outside the file")))?
                    .to_vec();
                let row = crate::key::row_bytes(format, dims(l).0)
                    .ok_or_else(|| err(format!("{import}: format {format}")))?;
                subresources.push((handle(row), data));
                at += len;
            }
        }
        let (w, h) = dims(first);
        let desc = TextureDesc {
            width: w,
            height: h,
            mips,
            array_size: faces,
            format,
            samples: 1,
            quality: 0,
            usage: 0,
            bind: ffx_draw::desc::BIND_SHADER_RESOURCE,
            cpu_access: 0,
            misc: if cube { 4 } else { 0 },
        };
        self.textures.push(TextureData { desc, subresources });
        let tex = Texture(handle(self.textures.len() - 1));
        self.views.push(ViewData {
            source: ViewSource::Texture(tex),
            kind: ViewKind::ShaderResource,
            desc: Some(ViewDesc {
                format,
                dimension: if cube { 9 } else { 4 },
                fields: [0, mips, 0, 0],
            }),
        });
        let v = View(handle(self.views.len() - 1));
        self.texture_ix.insert(import.to_owned(), v);
        Ok(v)
    }
}

/// The subresources a 2D texture cluster gets on the device, as the game
/// loads and uploads it:
///
/// - `Phyre_PBinary_loadClusterTextures` (`0x00465ab0`, `0x00466160..`)
///   reads every level of the file into one buffer, the levels below
///   `first` all at its start (each overwriting the last), the others in
///   turn;
/// - the upload (`0x0058fc20`, `0x0058fe1f..`) gives level `k` of the
///   texture the data at the sum of the earlier levels' sizes, a level's
///   size `bpp * w * h / 8` with `w` and `h` halved per level without a
///   floor: a side that reaches 0 counts as the top level's (both
///   rounded up to 4 for block formats), and the row pitch from the same
///   width. Device levels whose side went to 0 therefore read past the
///   levels read, into what the dropped levels left there (round 2 wire:
///   cdsp0000's 16x128, 32x128 and 128x32 textures, whose last one or two
///   levels are bytes of the file's level 1). Bytes past the buffer are
///   taken as 0.
///
/// Packed rows (row pitch = row bytes), as `ffx-drawdiff` keys them.
fn upload_subresources(
    t: &ffx_phyre::Texture,
    first: u32,
    format: u32,
) -> Option<Vec<(u32, Vec<u8>)>> {
    let dims = |l: u32| ((t.width >> l).max(1), (t.height >> l).max(1));
    let size = |l: u32| {
        let (w, h) = dims(l);
        t.format.level_size(w, h)
    };
    let chain: usize = (0..t.mip_levels).map(size).sum();
    let pixels = t.pixels();
    // The loader's buffer.
    let mut buf = vec![0u8; chain];
    let mut pos = 0;
    let mut at = 0;
    for l in 0..t.mip_levels {
        let len = size(l);
        let src = pixels.get(at..at + len)?;
        buf.get_mut(pos..pos + len)?.copy_from_slice(src);
        at += len;
        if l >= first {
            pos += len;
        }
    }
    // The upload's level layout.
    let (bpp, block) = match t.format {
        ffx_phyre::Format::Dxt1 => (4, true),
        ffx_phyre::Format::Dxt3 | ffx_phyre::Format::Dxt5 => (8, true),
        ffx_phyre::Format::Argb8 => (32, false),
        ffx_phyre::Format::L8 => (8, false),
    };
    let round = |x: usize| if block { (x + 3) & !3 } else { x };
    let (top_w, top_h) = (t.width >> first, t.height >> first);
    let (mut w, mut h) = (top_w as usize, top_h as usize);
    let mut off = 0usize;
    let mut out = Vec::new();
    for k in 0..t.mip_levels - first {
        let uw = round(if w == 0 { top_w as usize } else { w });
        let uh = round(if h == 0 { top_h as usize } else { h });
        let pitch = bpp * uw * if block { 4 } else { 1 } / 8;
        let (dw, dh) = ((top_w >> k).max(1), (top_h >> k).max(1));
        let row = crate::key::row_bytes(format, dw)? as usize;
        let rows = if block { dh.div_ceil(4) } else { dh } as usize;
        let mut data = Vec::with_capacity(row * rows);
        for r in 0..rows {
            let s = off + r * pitch;
            for i in s..s + row {
                data.push(buf.get(i).copied().unwrap_or(0));
            }
        }
        out.push((handle(row), data));
        off += bpp * uw * uh / 8;
        w >>= 1;
        h >>= 1;
    }
    Some(out)
}

/// Which of the two map passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pass {
    Opaque,
    Transparent,
}

/// One built draw, with where it comes from.
#[derive(Debug, Clone)]
pub struct MapDraw {
    pub pass: Pass,
    pub instance: usize,
    pub segment: usize,
    pub material: usize,
    pub state: DrawState,
    pub call: DrawCall,
}

/// The builder's output.
#[derive(Debug, Clone, Default)]
pub struct Built {
    pub resources: Resources,
    /// The opaque pass's draws, then the transparent pass's.
    pub draws: Vec<MapDraw>,
    /// What the builder could not do exactly (each a cause).
    pub notes: Vec<String>,
}

impl Built {
    /// The draws of a pass.
    pub fn pass(&self, pass: Pass) -> impl Iterator<Item = &MapDraw> {
        self.draws.iter().filter(move |d| d.pass == pass)
    }
}

/// `PSamplerStateD3D11`'s description (`0x00595bf0`): the filter from the
/// min and mag filters and the compare flag (`m_flags` bit 6,
/// `0x0058aa10`), `0x55` (anisotropic) when the anisotropy setting is
/// above 1; the wrap modes (`0x0058ab10`: 1 wrap, 3 border, 4 mirror,
/// else clamp); the LOD bias; the anisotropy truncated (the setting's,
/// rounded, when above 1); `LESS_EQUAL` with the compare flag, else
/// `NEVER`; the border bytes times the `f64` `1/255` at `0x00b34258`;
/// the LOD range of `CD3D11_SAMPLER_DESC(D3D11_DEFAULT)`.
#[must_use]
pub fn sampler_desc(s: &map::SamplerState, anisotropy: f32) -> SamplerDesc {
    let compare = (s.flags >> 6) & 1 != 0;
    let mag = u32::from(s.mag_filter != 0);
    let mut filter = match s.min_filter {
        0 | 2 => mag * 4,
        1 | 3 => mag * 4 + 0x10,
        4 => mag * 4 + 1,
        5 => mag * 4 + 0x11,
        _ => 0,
    };
    let aniso = anisotropy > 1.0;
    if aniso {
        filter = 0x55;
    }
    if compare {
        filter |= 0x80;
    }
    let wrap = |w: u8| match w {
        1 => 1,
        3 => 4,
        4 => 2,
        _ => 3,
    };
    // `fistp` under a chopping control word; out of range or NaN gives
    // the integer indefinite, whose low word is 0.
    let chop = |x: f32| -> u32 {
        if x.is_finite() && x.abs() < 9.2e18 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // the low word of the fistp qword
            {
                (x.trunc() as i64) as u32
            }
        } else {
            0
        }
    };
    let max_anisotropy = if aniso {
        chop(anisotropy.round_ties_even())
    } else {
        chop(s.max_anisotropy)
    };
    let scale = f64::from_bits(0x3f70_1010_2000_0000);
    #[allow(clippy::cast_possible_truncation)] // fstp dword
    let border = s.border.map(|b| ((f64::from(b) * scale) as f32).to_bits());
    SamplerDesc {
        filter,
        address: s.wrap.map(wrap),
        mip_lod_bias: s.lod_bias.to_bits(),
        max_anisotropy,
        comparison: if compare { 4 } else { 1 },
        border,
        min_lod: (-f32::MAX).to_bits(),
        max_lod: f32::MAX.to_bits(),
    }
}

/// The identity, for an instance without a world matrix.
const IDENTITY: Mat = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// The per-node parameter buffer of a variant (`0x005b7fe0`): each
/// untweakable definition's value at its offset, matrices in memory order
/// (the transpose of the shaders' order; [`fill_cb`] transposes them).
/// `World` (semantic 0x82) is the instance's world, `WorldView` (0x83)
/// `View . World` with f32 sums, `WorldViewProjection` (0x84)
/// `ViewProjection . World` with f64 sums rounded once
/// (`docs/formats/scene.md`, "Constants"). Others are left 0 and noted.
fn node_buffer(
    v: &EffectVariant,
    camera: &Camera,
    world: &Mat,
    notes: &mut Vec<String>,
) -> Vec<u8> {
    let mut buf = vec![0u8; usize::from(v.untweakable_size).max(16)];
    for d in &v.untweakable {
        let m = match d.kind {
            0x82 => *world,
            0x83 => mul(&camera.view, world),
            0x84 => mul_f64(&camera.view_projection, world),
            k => {
                let note = format!("per-node semantic {k:#x} ({}) left 0", d.name);
                if !notes.contains(&note) {
                    notes.push(note);
                }
                continue;
            }
        };
        let m = crate::inputs::transpose(&m);
        let at = d.offset as usize;
        for (k, x) in m.iter().enumerate() {
            if let Some(dst) = buf.get_mut(at + 4 * k..at + 4 * k + 4) {
                dst.copy_from_slice(&x.to_le_bytes());
            }
        }
    }
    buf
}

/// Whether a location entry holds `float4x4`s (data type `0x31`, which
/// the entry's kind repeats).
#[must_use]
pub fn is_matrix(loc: &crate::effect::Loc) -> bool {
    loc.kind == 0x31 && loc.size.is_multiple_of(64) && loc.size > 0
}

/// A 4x4 of floats transposed, as bytes: a matrix as the game keeps it
/// (`PMatrix4`, column-major) and as the shaders get it, and back.
#[must_use]
pub fn transpose_bytes(b: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; b.len()];
    for i in 0..16 {
        let j = (i % 4) * 4 + i / 4;
        if let (Some(d), Some(s)) = (out.get_mut(4 * i..4 * i + 4), b.get(4 * j..4 * j + 4)) {
            d.copy_from_slice(s);
        }
    }
    out
}

/// Copies a `float4x4` entry as the render thread's constant buffer copy
/// does (`0x0059a650`, m4-chars, `docs/formats/scene-chars.md`): each
/// 4x4 transposed, stopping at one whose first word is `0xffffffff` (the
/// end of a bone list); what it doesn't reach keeps the buffer's fill.
pub fn copy_matrices(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.as_chunks::<64>().0.iter().zip(dst.as_chunks_mut::<64>().0) {
        if s[..4] == [0xff; 4] {
            break;
        }
        d.copy_from_slice(&transpose_bytes(s));
    }
}

/// A constant buffer of `size` bytes filled through a location table:
/// each entry's `size` bytes from its source buffer at its offset go to
/// its target offset (entries the program doesn't use have target
/// `u32::MAX`), `float4x4` entries of every source transposed
/// ([`copy_matrices`]): the shadow cascade `PhyreScene_RenderShadow`
/// writes into the material as a `PMatrix4` is its transpose in every
/// captured map draw (round 2 wire); the node and scene-wide buffers hold
/// their matrices in memory order too.
fn fill_cb(size: u32, table: &LocTable, sources: [&[u8]; 3], notes: &mut Vec<String>) -> Vec<u8> {
    let mut cb = vec![0xcd_u8; size as usize];
    for (src, loc) in table.sourced() {
        if loc.target == u32::MAX {
            continue;
        }
        let from = match src {
            Source::SceneWide => sources[0],
            Source::Material => sources[1],
            Source::Node => sources[2],
            Source::Other => {
                notes.push(format!("location group 4 entry at {:#x}", loc.offset));
                continue;
            }
        };
        let (o, t, n) = (
            usize::from(loc.offset),
            loc.target as usize,
            loc.size as usize,
        );
        match (from.get(o..o + n), cb.get_mut(t..t + n)) {
            (Some(s), Some(d)) if is_matrix(loc) => copy_matrices(s, d),
            (Some(s), Some(d)) => d.copy_from_slice(s),
            _ => notes.push(format!(
                "{src:?} parameter at {o:#x} ({n} bytes) to {t:#x}: outside a buffer"
            )),
        }
    }
    cb
}

struct Builder<'a, I: SceneInputs> {
    c: &'a Clusters,
    inputs: &'a I,
    camera: Camera,
    dropped: u32,
    anisotropy: f32,
    scene_wide: Vec<u8>,
    materials: HashMap<usize, Vec<u8>>,
    out: Built,
}

/// The input layout and vertex bindings of a segment for a vertex
/// program. Each shader input (`PStreamInputDescD3D11`: semantic, render
/// type, semantic index, format, slot) reads one of the segment's streams
/// of its render type: when the instance's segment context binds the type
/// and semantic index (`PMeshInstanceSegmentStreamBinding`: the water's
/// `TEXCOORD4/6/7`, a second UV set), the one whose `m_streamSet` is the
/// binding's `m_inputSet`; else the first. The element's offset is the stream's in its block.
/// Blocks are bound at offset 0 with their stride.
fn vertex_inputs(
    map: &MapModel,
    instance: usize,
    segment: usize,
    vs: &Program,
    notes: &mut Vec<String>,
) -> Vec<(InputElement, Option<usize>)> {
    let seg = &map.segments[segment];
    let inst = &map.instances[instance];
    let bindings = inst
        .segments
        .iter()
        .position(|&s| s == segment)
        .and_then(|k| inst.contexts.get(k))
        .map_or(&[][..], Vec::as_slice);
    let mut out = Vec::new();
    for input in &vs.inputs {
        // The binding's input set names the stream by its `m_streamSet`.
        let set = bindings
            .iter()
            .find(|b| b.render_type == input.render_type && u32::from(b.index) == input.index)
            .map(|b| b.input_set);
        let mut streams = seg
            .blocks
            .iter()
            .enumerate()
            .flat_map(|(b, block)| block.streams.iter().map(move |s| (b, s)))
            .filter(|(_, s)| s.render_type == input.render_type);
        let found = match set {
            Some(k) => streams.find(|(_, s)| s.stream_set == k),
            None => streams.next(),
        }
        .map(|(b, s)| (b, s.offset));
        let set = set.map_or_else(|| "first".to_owned(), |k| format!("set {k}"));
        if found.is_none() {
            notes.push(format!(
                "segment {segment}: no {} stream ({set}) for {}{}",
                input.render_type, input.semantic, input.index
            ));
        }
        out.push((
            InputElement {
                semantic: input.semantic.clone(),
                index: input.index,
                format: input.format,
                slot: input.slot,
                offset: found.map_or(0, |(_, o)| handle(o)),
                per_instance: false,
                step: 0,
            },
            found.map(|(b, _)| b),
        ));
    }
    out
}

impl<I: SceneInputs> Builder<'_, I> {
    fn material_buffer(&mut self, m: usize) -> &[u8] {
        let inputs = self.inputs;
        self.materials
            .entry(m)
            .or_insert_with(|| inputs.material_buffer(m))
    }

    /// The visible instances of one pass, in draw order.
    fn visible(&self, opaque: bool) -> Vec<usize> {
        let mut runs = Vec::new();
        for job in self.inputs.jobs() {
            let mut entries = Vec::new();
            for &i in &job {
                let Some(b) = self.c.map.instances.get(i).and_then(|x| x.bounds) else {
                    continue;
                };
                if !self.inputs.instance_shown(i) {
                    continue;
                }
                let world = self.inputs.world(i);
                let m = crate::cull::cull_matrix(&self.camera.cull, world.as_ref());
                if !crate::cull::box_visible(b.min, b.size, &m) {
                    continue;
                }
                let p = crate::cull::centre(b.min, b.size, world.as_ref());
                let key =
                    crate::cull::depth_key(&self.camera.view, opaque, p, self.inputs.depth_bias(i));
                entries.push(Entry {
                    instance: i,
                    key,
                    render_order: self.inputs.render_order(i).unwrap_or(b.render_order),
                });
            }
            sort::sort(&mut entries);
            if !entries.is_empty() {
                runs.push(entries);
            }
        }
        sort::merge(&runs)
    }

    fn instance(&mut self, pass: Pass, pass_name: &str, instance: usize) -> Result<(), Error> {
        let c = self.c;
        let inst = &c.map.instances[instance];
        for bucket in 0..4 {
            for &segment in &inst.segments {
                let seg = &c.map.segments[segment];
                if seg.render_order != bucket {
                    continue;
                }
                // A null material ends the instance.
                let Some(material) = c.map.material(instance, segment) else {
                    return Ok(());
                };
                let mat = &c.map.materials[material];
                if pass_name == "Opaque"
                    && mat.remap_from.as_deref() == Some("Transparent")
                    && mat.remap_to.as_deref() != Some("Transparent")
                {
                    continue;
                }
                let variant = c
                    .effects
                    .get(&mat.effect)
                    .ok_or_else(|| err(format!("effect {} not loaded", mat.effect)))?;
                let pass_type = if mat.remap_from.as_deref() == Some(pass_name) {
                    mat.remap_to.as_deref().unwrap_or(pass_name)
                } else {
                    pass_name
                };
                let Some(render_pass) = variant.pass(pass_type) else {
                    continue;
                };
                // The shader is the render pass's context variant for the
                // node (`0x004ef440`: a search over the effect's context
                // switches, `NUM_LIGHTS`, ...), not ported: the first
                // shader, which every map draw of cdsp0000 uses.
                let Some(shader) = render_pass.shaders.first() else {
                    continue;
                };
                if self.inputs.segment_hidden(segment) {
                    continue;
                }
                for sp in &shader.passes {
                    self.draw(pass, instance, segment, material, variant, sp)?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // one draw's every binding
    fn draw(
        &mut self,
        pass: Pass,
        instance: usize,
        segment: usize,
        material: usize,
        variant: &EffectVariant,
        sp: &ShaderPass,
    ) -> Result<(), Error> {
        let c = self.c;
        let seg = &c.map.segments[segment];
        let mut state = DrawState {
            topology: match seg.primitive {
                2 => 4,
                p => return Err(err(format!("segment {segment}: primitive type {p}"))),
            },
            sample_mask: u32::MAX,
            stencil_ref: u32::from(sp.state.stencil_ref),
            viewport: self.inputs.viewport(),
            scissor: self.inputs.scissor(),
            ..DrawState::default()
        };
        let res = &mut self.out.resources;
        let vs = res.shader(Stage::Vertex, &sp.vs.code);
        let ps = res.shader(Stage::Pixel, &sp.ps.code);
        let inputs = vertex_inputs(&c.map, instance, segment, &sp.vs, &mut self.out.notes);
        let mut elements = Vec::new();
        let dynamic = c.map.instances[instance].dynamic;
        for (e, block) in inputs {
            if let Some(b) = block {
                let blk = &seg.blocks[b];
                let buffer = if dynamic && blk.streams.iter().any(|s| s.render_type == "Vertex") {
                    res.external_buffer(
                        External::DynamicVertices {
                            instance,
                            segment,
                            block: b,
                        },
                        blk.stride * blk.count,
                    )
                } else {
                    res.static_buffer(&c.map, segment, Some(b))
                };
                if let Some(slot) = state.vertex.get_mut(e.slot as usize) {
                    *slot = Some(VertexBinding {
                        buffer,
                        stride: handle(seg.blocks[b].stride),
                        offset: 0,
                    });
                }
            }
            elements.push(e);
        }
        state.layout = Some(res.layout(elements, vs));
        state.index = Some(IndexBinding {
            buffer: res.static_buffer(&c.map, segment, None),
            format: 57,
            offset: 0,
        });
        let blend = BlendDesc::from_d3d11(&sp.state.blend).map_err(|e| err(e.to_string()))?;
        state.blend = Some(Blend(intern(&mut res.blends, &blend)));
        let depth =
            DepthStencilDesc::from_d3d11(&sp.state.depth).map_err(|e| err(e.to_string()))?;
        state.depth = Some(DepthState(intern(&mut res.depths, &depth)));
        let raster = RasterDesc::from_d3d11(&sp.state.raster).map_err(|e| err(e.to_string()))?;
        state.raster = Some(Raster(intern(&mut res.rasters, &raster)));
        state.targets[0] = Some(res.external(External::Color, ViewKind::RenderTarget));
        state.targets[1] = Some(res.external(External::Normal, ViewKind::RenderTarget));
        state.depth_target = Some(res.external(External::Depth, ViewKind::DepthStencil));

        // Constant buffers.
        let world = self.inputs.world(instance).unwrap_or(IDENTITY);
        let node = node_buffer(variant, &self.camera, &world, &mut self.out.notes);
        let material_bytes = self.material_buffer(material).to_vec();
        let scene = std::mem::take(&mut self.scene_wide);
        let mut stages = [StageBindings::default(), StageBindings::default()];
        for (b, (prog, params, textures, shader)) in stages.iter_mut().zip([
            (&sp.vs, &sp.vs_params, &sp.vs_textures, vs),
            (&sp.ps, &sp.ps_params, &sp.ps_textures, ps),
        ]) {
            b.shader = Some(shader);
            if prog.global_cb != u32::MAX {
                let data = fill_cb(
                    prog.cb_size,
                    params,
                    [&scene, &material_bytes, &node],
                    &mut self.out.notes,
                );
                let desc = BufferDesc {
                    size: prog.cb_size,
                    usage: ffx_draw::desc::USAGE_DYNAMIC,
                    bind: ffx_draw::desc::BIND_CONSTANT,
                    cpu_access: ffx_draw::desc::CPU_WRITE,
                    misc: 0,
                    stride: 0,
                };
                let buffer = self.out.resources.buffer(desc, data);
                if let Some(slot) = b.cbs.get_mut(prog.global_cb as usize) {
                    *slot = Some(CbBinding {
                        buffer,
                        first: 0,
                        count: 0,
                    });
                }
            }
            self.bind_textures(textures, variant, material, b)?;
        }
        self.scene_wide = scene;
        let [vsb, psb] = stages;
        state.vs = vsb;
        state.ps = psb;
        self.out.draws.push(MapDraw {
            pass,
            instance,
            segment,
            material,
            state,
            call: DrawCall::Indexed {
                count: seg.index_count,
                start: 0,
                base: 0,
            },
        });
        Ok(())
    }

    /// Textures and samplers through a texture location table: a
    /// material sampler (0x42) puts its texture in register `target`, a
    /// sampler state (0x47) its state in sampler `target`.
    fn bind_textures(
        &mut self,
        table: &LocTable,
        variant: &EffectVariant,
        material: usize,
        b: &mut StageBindings,
    ) -> Result<(), Error> {
        let mat = &self.c.map.materials[material];
        for (src, loc) in table.sourced() {
            if loc.target == u32::MAX {
                continue;
            }
            let reg = loc.target as usize;
            if src != Source::Material {
                self.out.notes.push(format!(
                    "{src:?} texture slot at {:#x} used (register {reg}): not supported",
                    loc.offset
                ));
                continue;
            }
            let Some(def) = variant.tweakable_at(loc.offset) else {
                self.out
                    .notes
                    .push(format!("no tweakable at {:#x}", loc.offset));
                continue;
            };
            match def.kind {
                0x42 | 0x44 => {
                    let offset = u32::from(loc.offset);
                    let file = mat
                        .textures
                        .iter()
                        .find(|t| t.offset == offset)
                        .map(|t| t.import.as_str());
                    let view = match self.inputs.material_texture(material, offset, file) {
                        SlotTexture::Import(import) => {
                            let tex = self
                                .c
                                .textures
                                .get(&import)
                                .ok_or_else(|| err(format!("texture {import} not loaded")))?;
                            self.out.resources.texture(&import, tex, self.dropped)?
                        }
                        SlotTexture::Runtime => self.out.resources.external(
                            External::MaterialTexture { material, offset },
                            ViewKind::ShaderResource,
                        ),
                        // No texture in the slot: nothing bound.
                        SlotTexture::Empty => continue,
                    };
                    if let Some(slot) = b.srvs.get_mut(reg) {
                        *slot = Some(view);
                    }
                }
                0x47 => {
                    let Some((_, s)) = mat.samplers.iter().find(|s| s.0 == u32::from(loc.offset))
                    else {
                        self.out.notes.push(format!(
                            "material {material}: no sampler state at {:#x} ({})",
                            loc.offset, def.name
                        ));
                        continue;
                    };
                    let desc = sampler_desc(s, self.anisotropy);
                    let h = Sampler(intern(&mut self.out.resources.samplers, &desc));
                    if let Some(slot) = b.samplers.get_mut(reg) {
                        *slot = Some(h);
                    }
                }
                k => self
                    .out
                    .notes
                    .push(format!("texture table entry {} of kind {k:#x}", def.name)),
            }
        }
        Ok(())
    }
}

/// Builds the map world's draws for one frame.
///
/// # Errors
/// On a missing effect or texture, or data the builder doesn't handle
/// (each named).
pub fn build(c: &Clusters, inputs: &impl SceneInputs) -> Result<Built, Error> {
    let settings = inputs.settings();
    let dropped = settings.texture_quality.dropped_levels();
    let mut b = Builder {
        c,
        inputs,
        camera: inputs.camera(),
        dropped,
        anisotropy: settings.anisotropy,
        scene_wide: inputs.scene_wide(),
        materials: HashMap::new(),
        out: Built::default(),
    };
    let (opaque, transparent) = inputs.pass_names();
    for (pass, name, is_opaque) in [
        (Pass::Opaque, opaque, true),
        (Pass::Transparent, transparent, false),
    ] {
        for i in b.visible(is_opaque) {
            b.instance(pass, &name, i)?;
        }
    }
    let mut seen = std::collections::HashSet::new();
    b.out.notes.retain(|n| seen.insert(n.clone()));
    Ok(b.out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Loc;

    fn state(min: u8, mag: u8, wrap: [u8; 3], aniso: f32, flags: u32) -> map::SamplerState {
        map::SamplerState {
            min_filter: min,
            mag_filter: mag,
            wrap,
            lod_bias: 0.0,
            max_anisotropy: aniso,
            border: [0, 0, 0, 255],
            base_level: 0,
            max_level: 0x400,
            flags,
        }
    }

    #[test]
    fn sampler_descriptions_as_the_game_makes_them() {
        // cdsp0000's TextureSamplerState: linear, wrap, anisotropy 0
        let d = sampler_desc(&state(5, 1, [1, 1, 1], 0.0, 0), 1.0);
        assert_eq!(
            (d.filter, d.address, d.max_anisotropy, d.comparison),
            (0x15, [1, 1, 1], 0, 1)
        );
        assert_eq!(d.border[3], 1f32.to_bits());
        assert_eq!(
            (d.min_lod, d.max_lod),
            ((-f32::MAX).to_bits(), f32::MAX.to_bits())
        );
        // the water's: clamp (wrap 0 and 2 are clamp), anisotropy 1
        let d = sampler_desc(&state(5, 1, [0, 2, 3], 1.0, 0), 1.0);
        assert_eq!((d.address, d.max_anisotropy), ([3, 3, 4], 1));
        // the compare flag (bit 6): a comparison filter and LESS_EQUAL
        let d = sampler_desc(&state(1, 1, [0; 3], 1.0, 0x40), 1.0);
        assert_eq!((d.filter, d.comparison), (0x94, 4));
        // an anisotropy setting above 1 makes every sampler anisotropic
        let d = sampler_desc(&state(0, 0, [4; 3], 1.0, 0), 8.0);
        assert_eq!((d.filter, d.address, d.max_anisotropy), (0x55, [2; 3], 8));
    }

    #[test]
    fn constant_buffers_through_a_location_table() {
        let table = LocTable {
            starts: [0, 1, 2, 0],
            counts: [1, 1, 1, 0],
            entries: vec![
                Loc {
                    offset: 4,
                    target: 0,
                    size: 4,
                    kind: 0,
                },
                Loc {
                    offset: 8,
                    target: 4,
                    size: 4,
                    kind: 0,
                },
                Loc {
                    offset: 0,
                    target: u32::MAX,
                    size: 4,
                    kind: 0,
                },
            ],
        };
        let scene = [0, 0, 0, 0, 1, 2, 3, 4];
        let material = [0, 0, 0, 0, 0, 0, 0, 0, 5, 6, 7, 8];
        let mut notes = Vec::new();
        let cb = fill_cb(16, &table, [&scene, &material, &[]], &mut notes);
        // unused bytes keep the 0xcd fill of the game's heap
        assert_eq!(
            cb,
            [
                1, 2, 3, 4, 5, 6, 7, 8, 0xcd, 0xcd, 0xcd, 0xcd, 0xcd, 0xcd, 0xcd, 0xcd
            ]
        );
        assert_eq!(notes, Vec::<String>::new());
    }

    #[test]
    fn matrices_go_in_transposed_up_to_the_end_mark() {
        // Two float4x4s: the first 0..16, the second starts 0xffffffff.
        let mut src: Vec<u8> = (0u32..16).flat_map(u32::to_le_bytes).collect();
        src.extend([0xff; 4]);
        src.extend([0u8; 60]);
        let mut dst = vec![0xcd_u8; 128];
        copy_matrices(&src, &mut dst);
        let word = |i: usize| u32::from_le_bytes(dst[4 * i..4 * i + 4].try_into().expect("word"));
        // Row-major for column vectors: element (r, c) from word c * 4 + r.
        assert_eq!([word(0), word(1), word(4), word(7)], [0, 4, 1, 13]);
        assert!(dst[64..].iter().all(|&b| b == 0xcd));
        assert!(is_matrix(&Loc {
            offset: 0,
            target: 0,
            size: 128,
            kind: 0x31
        }));
    }
}
