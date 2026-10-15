//! The enhanced renderer: draws the game's scenery and characters itself
//! (its own shaders, with the game's geometry, textures, transforms and
//! materials, found in each draw's constant buffers by the names the
//! shader reflection gives), lights them in linear HDR with a key light,
//! soft shadows and sky light on top of the game's baked vertex light,
//! and tone-maps into the game's scene target. Draws it does not handle
//! (water, painted backdrops, transparent effects, unknown shaders) go
//! through the game's own shaders afterwards, as in the original path.

#[cfg(windows)]
pub mod fsr;
pub mod glow;
pub mod lights;
pub mod materials;
pub mod math;
mod post;
mod settings;
#[cfg(test)]
mod shader_tests;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use ffx_gfx::{Cmd, Draw, Frame, Id, Msg, StateKind};

use crate::families::{Families, Family};
use crate::{Filter, Renderer, formats};
pub use settings::Settings;

const GBUFFER_WGSL: &str = include_str!("gbuffer.wgsl");
const LIGHTING_WGSL: &str = include_str!("lighting.wgsl");
const OUTPUT_WGSL: &str = include_str!("output.wgsl");
const AO_WGSL: &str = include_str!("ao.wgsl");
const MATERIAL_WGSL: &str = include_str!("material.wgsl");
const RESOLVE_WGSL: &str = include_str!("resolve.wgsl");
const RESOLVE_SIZE: usize = 16 + 64 + 16;
const VOLUMETRIC_WGSL: &str = include_str!("volumetric.wgsl");
const WATER_WGSL: &str = include_str!("water.wgsl");
const PROBE_WGSL: &str = include_str!("probe.wgsl");
const EFFECT_WGSL: &str = include_str!("effect.wgsl");
const TAA_WGSL: &str = include_str!("taa.wgsl");
/// The lighting pass's ray-traced shadow, for devices with ray queries …
const RT_SHADOW_WGSL: &str = "enable wgpu_ray_query;
@group(0) @binding(9) var scene_as: acceleration_structure;
// A ray toward a point of the key light's disc (different per pixel and
// frame: TAA averages the soft edge); 0 where scenery is in the way.
fn rt_shadow(p: vec3<f32>, n: vec3<f32>, l: vec3<f32>, px: vec2<f32>, size: f32, frame: f32) -> f32 {
    let h1 = fract(sin(dot(px + frame * 7.13, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    let h2 = fract(sin(dot(px + frame * 3.71, vec2<f32>(39.3468, 11.135))) * 24634.6345);
    let t = normalize(cross(l, select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(l.y) > 0.9)));
    let b = cross(l, t);
    let r = sqrt(h1) * size;
    let a = h2 * 6.2831853;
    let dir = normalize(l + (t * cos(a) + b * sin(a)) * r);
    var rq: ray_query;
    rayQueryInitialize(&rq, scene_as, RayDesc(RAY_FLAG_TERMINATE_ON_FIRST_HIT, 0xffu, 0.02, light.rt.w, p + n * 0.15 + l * 0.05, dir));
    rayQueryProceed(&rq);
    let hit = rayQueryGetCommittedIntersection(&rq);
    if (hit.kind != RAY_QUERY_INTERSECTION_NONE) {
        return 0.0;
    }
    return 1.0;
}
// Whether nothing of the scenery is within `far` of `o` along `dir`.
fn rt_visible(o: vec3<f32>, dir: vec3<f32>, far: f32, px: vec2<f32>) -> f32 {
    if (far <= 0.05) {
        return 1.0;
    }
    var rq: ray_query;
    rayQueryInitialize(&rq, scene_as, RayDesc(RAY_FLAG_TERMINATE_ON_FIRST_HIT, 0xffu, 0.02, far, o, dir));
    rayQueryProceed(&rq);
    if (rayQueryGetCommittedIntersection(&rq).kind != RAY_QUERY_INTERSECTION_NONE) {
        return 0.0;
    }
    return 1.0;
}
";
/// … and without.
const NO_RT_WGSL: &str = "fn rt_shadow(p: vec3<f32>, n: vec3<f32>, l: vec3<f32>, px: vec2<f32>, size: f32, frame: f32) -> f32 {
    return 1.0;
}
fn rt_visible(o: vec3<f32>, dir: vec3<f32>, far: f32, px: vec2<f32>) -> f32 {
    return 1.0;
}
";

/// A cached bottom-level structure: the buffers it was built from (to
/// notice when an id is reused) and the structure.
struct CachedBlas {
    vertex: wgpu::Buffer,
    index: Option<wgpu::Buffer>,
    blas: wgpu::Blas,
}

/// A structure made this frame, to build: the structure, its size, the
/// vertex source, first vertex and stride, the index source and first
/// index.
type FreshBlas = (
    wgpu::Blas,
    wgpu::BlasTriangleGeometrySizeDescriptor,
    Source,
    u32,
    u32,
    Option<(Source, u32)>,
);

/// What a bottom-level structure is built from: vertex buffer id, first
/// vertex, stride, vertex count, index buffer id, first index, index count,
/// 32-bit indices.
type BlasKey = (Id, u32, u32, u32, Id, u32, u32, bool);
const TAA_SIZE: usize = 64 + 64 + 32;
/// Ids of the effect light's own target in the renderer's mirror (above
/// any 32-bit process's user addresses, which the game's ids are).
const EFFECT_IDS: Id = 0xfff0_0000;
/// Light probes: at most this many per frame, each a cube of faces this
/// size.
const MAX_PROBES: usize = 2;
const PROBE_FACE: u32 = 32;
const PROBE_INFO_SIZE: usize = 64 * 6 * MAX_PROBES + 16 * MAX_PROBES + 32;
const PROBES_SIZE: usize = 16 + 16 * MAX_PROBES + 16 * 9 * MAX_PROBES;
const WATER_SIZE: usize = 64 + 16 * 5;
const WATER_SCENE_SIZE: usize = 16 * 12 + 16 * 9;
/// Vertex inputs the water shader reads, by location.
const WATER_INPUTS: [&str; 8] = [
    "POSITION0",
    "NORMAL0",
    "COLOR0",
    "TEXCOORD0",
    "TEXCOORD4",
    "TEXCOORD6",
    "TEXCOORD7",
    "TANGENT0",
];

/// A water draw for the enhanced water pass.
struct WaterItem {
    key: GKey,
    vertex: Vec<Source>,
    index: Option<(Source, wgpu::IndexFormat)>,
    draw: Box<Draw>,
    uniform: Vec<u8>,
    /// Views: colour 0, colour 1, normal 0, normal 1, cube; and the
    /// sampler.
    views: [Id; 5],
    sampler: Id,
}
const VOL_SIZE: usize = 64 + 16 * 12 + 16 * 8;
const AO_SIZE: usize = 64 * 2 + 32;

/// Variables of one shader's first constant buffer: name → (offset, size),
/// and struct members as `struct.member`.
#[derive(Debug, Default)]
struct Reflect {
    vars: HashMap<String, (u32, u32)>,
    used: std::collections::HashSet<String>,
    /// Texture and sampler slots by binding name.
    texture_slots: HashMap<String, u32>,
    sampler_slots: HashMap<String, u32>,
    inputs: Vec<String>,
}

impl Reflect {
    fn parse(bytecode: &[u8]) -> Option<Self> {
        let c = ffx_dxbc::Container::parse(bytecode).ok()?;
        let r = c.reflection().ok()??;
        let mut out = Self::default();
        if let Some(cb) = r.buffer_at(0) {
            for v in &cb.variables {
                out.vars.insert(v.name.clone(), (v.offset, v.size));
                if v.used {
                    out.used.insert(v.name.clone());
                }
                for m in &v.members {
                    out.vars
                        .insert(format!("{}.{}", v.name, m.name), (v.offset + m.offset, 16));
                }
            }
        }
        for b in &r.bindings {
            match b.kind {
                ffx_dxbc::BindKind::Texture => {
                    out.texture_slots.insert(b.name.clone(), b.slot);
                }
                ffx_dxbc::BindKind::Sampler => {
                    out.sampler_slots.insert(b.name.clone(), b.slot);
                }
                _ => {}
            }
        }
        out.inputs = c
            .inputs()
            .ok()?
            .iter()
            .map(|s| format!("{}{}", s.semantic.to_ascii_uppercase(), s.index))
            .collect();
        Some(out)
    }

    fn floats(&self, cb: &[u8], name: &str, n: usize) -> Option<Vec<f32>> {
        let (off, _) = *self.vars.get(name)?;
        let bytes = cb.get(off as usize..off as usize + 4 * n)?;
        Some(
            bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect(),
        )
    }

    fn rows(&self, cb: &[u8], name: &str) -> Option<math::Mat> {
        let f = self.floats(cb, name, 16)?;
        Some([
            [f[0], f[1], f[2], f[3]],
            [f[4], f[5], f[6], f[7]],
            [f[8], f[9], f[10], f[11]],
            [f[12], f[13], f[14], f[15]],
        ])
    }
}

/// Vertex inputs the G-buffer shader reads, by location.
const INPUTS: [&str; 7] = [
    "POSITION0",
    "NORMAL0",
    "COLOR0",
    "TEXCOORD0",
    "BLENDINDICES0",
    "BLENDWEIGHTS0",
    "COLOR1",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)] // independent shader switches
struct Variant {
    skinned: bool,
    cloth: bool,
    color: bool,
    uv: bool,
    alpha_test: bool,
    shadow: bool,
}

/// A vertex buffer slot: stride (or slot), step per instance, attributes
/// (location, format, offset).
type SlotLayout = (u32, bool, Vec<(u32, wgpu::VertexFormat, u64)>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GKey {
    variant: Variant,
    /// Per slot: stride, step per instance, attributes (location, format,
    /// offset).
    buffers: Vec<SlotLayout>,
    cull: Option<wgpu::Face>,
    front: wgpu::FrontFace,
}

/// What a pick in the editor found.
#[derive(Debug, Clone, PartialEq)]
pub struct Picked {
    /// The draw's colour texture hash (0: none): its material's key.
    pub texture: u64,
    /// Its colour texture's view, for a preview.
    pub view: Option<Id>,
    pub character: bool,
    /// The surface point under the pick (world space), and the way it
    /// faces.
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// Nothing enhanced was there (sky, backdrop).
    pub empty: bool,
}

/// Reads what the G-buffer has at `at` (0–1 of the screen): which draw,
/// and the surface there (from the depth around it). Waits for the GPU (a
/// click in the editor, not every frame).
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the frame's pieces it reads; two copies and the surface
fn pick_at(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    t: &Targets,
    size: (u32, u32),
    at: (f32, f32),
    items: &[Item],
    inv_vp: &math::Mat,
    r: &Renderer,
) -> Option<Picked> {
    let px = (
        ((at.0 * size.0 as f32) as u32).clamp(1, size.0.saturating_sub(2)),
        ((at.1 * size.1 as f32) as u32).clamp(1, size.1.saturating_sub(2)),
    );
    // Depth copies must be whole: the id texel, then all the depth.
    let pitch = (size.0 * 4).next_multiple_of(256);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pick"),
        size: 256 + u64::from(pitch) * u64::from(size.1),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &t.id_texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: px.0,
                y: px.1,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &t.depth_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::DepthOnly,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 256,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(size.1),
            },
        },
        wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([enc.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
    let data = slice.get_mapped_range().ok()?;
    let word = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().expect("4"));
    let id = word(0) & 0x7fff_ffff;
    // The 3×3 pixels around it.
    let depth = |x: usize, y: usize| {
        let (qx, qy) = (px.0 as usize + x - 1, px.1 as usize + y - 1);
        f32::from_bits(word(256 + qy * pitch as usize + qx * 4))
    };
    let point = |dx: f32, dy: f32, z: f32| {
        let ndc = [
            (px.0 as f32 + 0.5 + dx) / size.0 as f32 * 2.0 - 1.0,
            1.0 - (px.1 as f32 + 0.5 + dy) / size.1 as f32 * 2.0,
        ];
        let h: [f32; 4] = std::array::from_fn(|k| {
            inv_vp[k][0] * ndc[0] + inv_vp[k][1] * ndc[1] + inv_vp[k][2] * z + inv_vp[k][3]
        });
        [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
    };
    let z = depth(1, 1);
    let p = point(0.0, 0.0, z);
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let mut normal = math::normalize(math::cross(
        sub(point(1.0, 0.0, depth(2, 1)), point(-1.0, 0.0, depth(0, 1))),
        sub(point(0.0, 1.0, depth(1, 2)), point(0.0, -1.0, depth(1, 0))),
    ));
    // Facing the camera.
    let eye = point(0.0, 0.0, 0.0);
    if math::dot(normal, sub(eye, p)) < 0.0 {
        normal = normal.map(|v| -v);
    }
    let item = (id > 0).then(|| items.get(id as usize - 1)).flatten();
    let view = item
        .and_then(|i| i.albedo.map(|a| a.0))
        .filter(|v| r.views.contains_key(v));
    Some(Picked {
        texture: item.map_or(0, |i| i.texture),
        view,
        character: item.is_some_and(|i| i.centre.is_some()),
        position: p,
        normal,
        empty: z >= 1.0 || item.is_none(),
    })
}

/// One draw for the enhanced passes.
struct Item {
    key: GKey,
    /// Vertex buffers per slot of `key.buffers`, then the index buffer.
    vertex: Vec<Source>,
    index: Option<(Source, wgpu::IndexFormat)>,
    draw: Box<Draw>,
    uniform: Vec<u8>,
    bones: Option<Vec<u8>>,
    albedo: Option<(Id, Id)>,
    /// A character's centre (for its light probe); scenery has none.
    centre: Option<[f32; 3]>,
    /// What it draws (shaders, buffers, range): finds it again next frame.
    motion: u64,
    /// Its colour texture's hash (0: none): the key of its material.
    texture: u64,
}

/// The effect light's targets and passes, for one scene image size and
/// format.
struct EffectLight {
    size: (u32, u32),
    format: wgpu::TextureFormat,
    /// The effects' target in the mirror: texture and view ids.
    ids: (Id, Id),
    /// A copy of the scene image (read while it is written).
    copy: (wgpu::Texture, wgpu::TextureView),
    small: [wgpu::TextureView; 2],
    down: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    apply: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// The light probes' G-buffer layers (six per probe) and the buffer of
/// their spherical harmonics.
struct ProbeTargets {
    /// Per layer: albedo, normal, baked light, depth.
    layers: Vec<[wgpu::TextureView; 4]>,
    /// The layers as arrays, for the projection pass: albedo, baked, depth.
    arrays: [wgpu::TextureView; 3],
    /// Where the G-buffer pipelines write motion, material and ids (not
    /// read for probes).
    motion: wgpu::TextureView,
    material: wgpu::TextureView,
    id: wgpu::TextureView,
    buffer: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

#[derive(Clone, Copy)]
enum Source {
    Buffer(Id, u64),
    Arena(u64),
}

/// Render targets of the enhanced passes, at the frame's size.
struct Targets {
    size: (u32, u32),
    albedo: wgpu::TextureView,
    normal: wgpu::TextureView,
    baked: wgpu::TextureView,
    /// Motion vectors (see `gbuffer.wgsl`).
    motion: wgpu::TextureView,
    /// Metallic, specular, emission, skin.
    material: wgpu::TextureView,
    /// Which draw is at each pixel (its id + 1), for the editor's picking.
    id: wgpu::TextureView,
    id_texture: wgpu::Texture,
    depth: wgpu::TextureView,
    hdr: wgpu::TextureView,
    ao: [wgpu::TextureView; 2],
    ldr: wgpu::TextureView,
    #[cfg_attr(not(windows), allow(dead_code))] // FSR's input
    ldr_texture: wgpu::Texture,
    /// TAA's history, in turn written and read.
    history: [wgpu::TextureView; 2],
    /// The HDR image and depth as textures (for copies), and their copies
    /// the water reads.
    hdr_texture: wgpu::Texture,
    depth_texture: wgpu::Texture,
    hdr_copy: (wgpu::Texture, wgpu::TextureView),
    depth_copy: (wgpu::Texture, wgpu::TextureView),
    /// Volumetric light, half size.
    vol: wgpu::TextureView,
}

pub struct Enhanced {
    pub settings: Settings,
    families: Families,
    reflect: HashMap<u64, Option<Arc<Reflect>>>,
    pipelines: HashMap<GKey, Option<Arc<wgpu::RenderPipeline>>>,
    camera_layout: wgpu::BindGroupLayout,
    item_layout: wgpu::BindGroupLayout,
    gbuffer_layout: wgpu::PipelineLayout,
    lighting: (wgpu::RenderPipeline, wgpu::BindGroupLayout),
    probes: ProbeTargets,
    /// The local lights of this frame (`lights::pack`).
    light_buffer: wgpu::Buffer,
    /// Local lights from the editor (scene and global), this frame.
    pub lights: Vec<lights::Light>,
    /// The editor's changes to the key light and to the glow lights.
    pub sun: lights::Sun,
    pub glow_edits: Vec<lights::GlowEdit>,
    /// Materials from the editor, by colour texture hash.
    pub material_edits: HashMap<u64, materials::Material>,
    /// The texture the editor has picked: its draws are outlined.
    pub selected: Option<u64>,
    /// A pick the editor asks for: where on the screen (0–1).
    pub pick: Option<(f32, f32)>,
    /// What the last pick found.
    pub picked: Option<Picked>,
    /// Lights found in the game's glowing effects.
    pub glow: glow::Glow,
    /// The device traces rays; the scenery's structures by draw.
    rt: bool,
    blas_cache: HashMap<BlasKey, CachedBlas>,
    /// The top-level structure, rebuilt each frame, remade when it needs
    /// more room.
    tlas: Option<(u32, wgpu::Tlas)>,
    effect: Option<EffectLight>,
    effect_generation: u32,
    /// Water: its layouts, pipeline layout and pipelines by geometry.
    water_layouts: (
        wgpu::BindGroupLayout,
        wgpu::BindGroupLayout,
        wgpu::PipelineLayout,
    ),
    water_pipelines: HashMap<GKey, Option<Arc<wgpu::RenderPipeline>>>,
    /// Volumetric light and its layout.
    volumetric: (wgpu::RenderPipeline, wgpu::BindGroupLayout),
    /// Frames drawn (for per-frame noise).
    frame_count: u32,
    /// Tone mapping into the intermediate image, and its layout.
    tone: (wgpu::RenderPipeline, wgpu::BindGroupLayout),
    taa: (wgpu::RenderPipeline, wgpu::BindGroupLayout),
    /// Depth of field, motion blur, bloom.
    post: post::Post,
    /// Last frame's view-projection without jitter and eye (motion
    /// vectors).
    last_view_proj: Option<(math::Mat, [f32; 3])>,
    /// Last frame's camera (view-projection, eye) and image size, and
    /// which history it wrote.
    taa_last: Option<(math::Mat, [f32; 3], (u32, u32))>,
    taa_index: usize,
    /// Per draw (`Item::motion`): last frame's World rows and bones.
    motion: HashMap<u64, (Vec<u8>, Option<Vec<u8>>)>,
    /// Material map generation (first level, then each smaller one).
    material_gen: (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
    ),
    /// Generated material maps by colour texture: (version, view).
    materials: HashMap<Id, (u32, wgpu::TextureView)>,
    /// Material packs: `<hash>.png` per colour texture (rg normal, b
    /// roughness, a cavity), used instead of the generated maps.
    pub material_dir: Option<std::path::PathBuf>,
    /// AMD FSR 3 (antialiasing 3), loaded on first use from `fsr_dir`.
    #[cfg(windows)]
    fsr: Option<fsr::Fsr>,
    #[cfg_attr(not(windows), allow(dead_code))]
    fsr_tried: bool,
    pub fsr_dir: Option<std::path::PathBuf>,
    /// FSR's motion input (`Rg16Float` at the render size).
    #[cfg_attr(not(windows), allow(dead_code))]
    fsr_motion: Option<((u32, u32), wgpu::Texture, wgpu::TextureView)>,
    last_render: Option<std::time::Instant>,
    /// Images the editor puts in place of textures (`Material::texture`).
    pub texture_dir: Option<std::path::PathBuf>,
    replacements: HashMap<String, Option<wgpu::TextureView>>,
    /// Where to write each generated material map and its colour texture
    /// (`<hash>.png`, `<hash>_albedo.png`), for editing into a pack.
    pub export_dir: Option<std::path::PathBuf>,
    exported: std::collections::HashSet<u64>,
    flat_material: wgpu::TextureView,
    /// Ambient occlusion and its blur, and their layout.
    ao: (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
    ),
    output: Option<(
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
        Vec<wgpu::TextureFormat>,
        wgpu::TextureFormat,
    )>,
    targets: Option<Targets>,
    /// Shadow cascades: size, the array for sampling, one view per layer.
    /// Size; the cascades with every caster and with those rays do not
    /// cover; the six layers (three each).
    shadow: Option<(
        u32,
        wgpu::TextureView,
        wgpu::TextureView,
        Vec<wgpu::TextureView>,
    )>,
    white: wgpu::TextureView,
    black: wgpu::TextureView,
    cube: wgpu::TextureView,
    linear: wgpu::Sampler,
    compare: wgpu::Sampler,
    /// Problems seen (once each), for the log.
    pub notes: HashMap<String, usize>,
    /// The map's lights as last seen in a character draw.
    map_lights: Option<MapLights>,
    /// Draws in the last frame: enhanced, and with the game's shaders.
    pub last_items: usize,
    pub last_other: usize,
}

/// The game's per-map character lighting, in linear terms: key and fill
/// direction (the way the light travels) and colour, and ambient.
#[derive(Debug, Clone, Copy)]
struct MapLights {
    key: ([f32; 3], [f32; 3]),
    fill: ([f32; 3], [f32; 3]),
    ambient: [f32; 3],
}

/// The game's light values are gamma-space, with 0.5 as full.
fn linear(c: [f32; 3]) -> [f32; 3] {
    c.map(|x| (x * 2.0).max(0.0).powf(2.2))
}

fn fullscreen(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &str,
    targets: &[Option<wgpu::ColorTargetState>],
    depth: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: depth,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn fullscreen_entry(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &str,
    entry: &str,
    targets: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn layout_entries(
    entries: &[(u32, wgpu::BindingType)],
    visibility: wgpu::ShaderStages,
) -> Vec<wgpu::BindGroupLayoutEntry> {
    entries
        .iter()
        .map(|&(binding, ty)| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty,
            count: None,
        })
        .collect()
}

fn uniform(min: u64) -> wgpu::BindingType {
    wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: wgpu::BufferSize::new(min),
    }
}

fn tex(sample_type: wgpu::TextureSampleType) -> wgpu::BindingType {
    wgpu::BindingType::Texture {
        sample_type,
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    }
}

const FLOAT: wgpu::TextureSampleType = wgpu::TextureSampleType::Float { filterable: true };
const UNFILTERED: wgpu::TextureSampleType = wgpu::TextureSampleType::Float { filterable: false };

/// Item uniform as `item()` makes it: world (64), cloth (64), colour,
/// params, ambient, scale, extra, look, tint (112); the frame adds last
/// frame's World.
const ITEM_BASE: usize = 240;
const ITEM_SIZE: usize = ITEM_BASE + 64;
const CAMERA_SIZE: usize = 64 * 5 + 32;
const LIGHT_SIZE: usize = 64 + 16 * 12 + 16 * 17;
const POST_SIZE: usize = 16 + 64 + 16;

/// The `VsIn` struct of a generated shader (the inputs `present`, by
/// location in `decl`) and its accessors `a_<name>`, which give the
/// declared default for inputs the draw lacks.
fn vertex_input(decl: &[(&str, &str, &str)], present: &[u32]) -> (String, String) {
    use std::fmt::Write as _;
    let mut vsin = String::from("struct VsIn {\n    @builtin(vertex_index) vid: u32,\n");
    let mut access = String::new();
    for (loc, (name, ty, default)) in decl.iter().enumerate() {
        if present.contains(&(loc as u32)) {
            let _ = writeln!(vsin, "    @location({loc}) {name}: {ty},");
            let _ = writeln!(
                access,
                "fn a_{name}(v: VsIn) -> {ty} {{ return v.{name}; }}"
            );
        } else {
            let _ = writeln!(
                access,
                "fn a_{name}(v: VsIn) -> {ty} {{ return {default}; }}"
            );
        }
    }
    vsin.push_str("};\n");
    (vsin, access)
}

#[allow(clippy::unnecessary_wraps)] // the colour attachment lists take options
fn attachment(
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        },
    })
}

fn put_rows(out: &mut Vec<u8>, m: &math::Mat) {
    for row in m {
        for v in row {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

fn put4(out: &mut Vec<u8>, v: [f32; 4]) {
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
}

/// The G-buffer shader of a variant, for a draw with the attributes at
/// `present` (`INPUTS` locations).
fn gbuffer_source(v: Variant, present: &[u32]) -> String {
    let consts = format!(
        "diagnostic(off, derivative_uniformity);\nconst SKINNED: bool = {};\nconst CLOTH: bool = {};\nconst HAS_COLOR: bool = {};\nconst HAS_UV: bool = {};\nconst ALPHA_TEST: bool = {};\n",
        v.skinned, v.cloth, v.color, v.uv, v.alpha_test
    );
    let decl = [
        ("pos", "vec3<f32>", "vec3<f32>(0.0)"),
        ("nrm", "vec3<f32>", "vec3<f32>(0.0, 1.0, 0.0)"),
        ("col", "vec4<f32>", "vec4<f32>(0.5)"),
        ("uv", "vec2<f32>", "vec2<f32>(0.0)"),
        ("bi", "vec4<u32>", "vec4<u32>(0u)"),
        ("bw", "vec4<f32>", "vec4<f32>(1.0, 0.0, 0.0, 0.0)"),
        ("cloth", "vec4<f32>", "vec4<f32>(0.0)"),
    ];
    let (vsin, access) = vertex_input(&decl, present);
    format!("{consts}{vsin}{access}{GBUFFER_WGSL}")
}

/// The water shader for a draw with the attributes at `present`.
fn water_source(present: &[u32]) -> String {
    let decl = [
        ("pos", "vec3<f32>", "vec3<f32>(0.0)"),
        ("nrm", "vec3<f32>", "vec3<f32>(0.0, 1.0, 0.0)"),
        ("col", "vec4<f32>", "vec4<f32>(0.5)"),
        ("uv0", "vec2<f32>", "vec2<f32>(0.0)"),
        ("uv1", "vec2<f32>", "vec2<f32>(0.0)"),
        ("uv2", "vec2<f32>", "vec2<f32>(0.0)"),
        ("uv3", "vec2<f32>", "vec2<f32>(0.0)"),
        ("tan", "vec3<f32>", "vec3<f32>(1.0, 0.0, 0.0)"),
    ];
    let (vsin, access) = vertex_input(&decl, present);
    format!("{vsin}{access}{WATER_WGSL}")
}

/// A draw's identity across frames: its shaders, buffers and range.
fn motion_key(d: &Draw) -> u64 {
    let words = [d.vs, d.ps, d.index.0, d.index.2, d.start, d.count]
        .into_iter()
        .chain([d.base.cast_unsigned(), d.first_instance])
        .chain(d.vertex.iter().flat_map(|v| [v.1, v.3]));
    let bytes: Vec<u8> = words.flat_map(u32::to_le_bytes).collect();
    crate::fnv(&bytes)
}

/// The light probes' layers, buffer and projection pass.
#[allow(clippy::too_many_lines)] // the targets, the views, the pass
fn probe_targets(device: &wgpu::Device) -> ProbeTargets {
    let count = 6 * MAX_PROBES as u32;
    let make = |format: wgpu::TextureFormat, label: &str| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: PROBE_FACE,
                height: PROBE_FACE,
                depth_or_array_layers: count,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
    };
    let textures = [
        make(wgpu::TextureFormat::Rgba8UnormSrgb, "probe albedo"),
        make(wgpu::TextureFormat::Rgba16Float, "probe normal"),
        make(wgpu::TextureFormat::Rgba16Float, "probe baked"),
        make(wgpu::TextureFormat::Depth32Float, "probe depth"),
    ];
    let layer = |t: &wgpu::Texture, l: u32| {
        t.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: l,
            array_layer_count: Some(1),
            ..Default::default()
        })
    };
    let layers = (0..count)
        .map(|l| {
            [
                layer(&textures[0], l),
                layer(&textures[1], l),
                layer(&textures[2], l),
                layer(&textures[3], l),
            ]
        })
        .collect();
    let scratch = |format, label| {
        make(format, label).create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2),
            array_layer_count: Some(1),
            ..Default::default()
        })
    };
    let motion = scratch(wgpu::TextureFormat::Rgba16Float, "probe motion");
    let material = scratch(wgpu::TextureFormat::Rgba8Unorm, "probe material");
    let id = scratch(wgpu::TextureFormat::R32Uint, "probe ids");
    let array = |t: &wgpu::Texture| {
        t.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
    };
    let arrays = [
        array(&textures[0]),
        array(&textures[2]),
        array(&textures[3]),
    ];
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probes"),
        size: PROBES_SIZE as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let array_tex = |sample_type| wgpu::BindingType::Texture {
        sample_type,
        view_dimension: wgpu::TextureViewDimension::D2Array,
        multisampled: false,
    };
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("probe"),
        entries: &layout_entries(
            &[
                (0, uniform(PROBE_INFO_SIZE as u64)),
                (1, array_tex(UNFILTERED)),
                (2, array_tex(UNFILTERED)),
                (3, array_tex(wgpu::TextureSampleType::Depth)),
                (
                    4,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(PROBES_SIZE as u64),
                    },
                ),
            ],
            wgpu::ShaderStages::COMPUTE,
        ),
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("probe"),
        source: wgpu::ShaderSource::Wgsl(PROBE_WGSL.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("probe"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("probe"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            }),
        ),
        module: &module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    ProbeTargets {
        layers,
        arrays,
        motion,
        material,
        id,
        buffer,
        pipeline,
        layout,
    }
}

/// The six faces of a light probe at `centre`: view-projections with the
/// handedness and conventions of the game's camera (so culling agrees).
fn probe_faces(centre: [f32; 3], view: &math::Mat, proj: &math::Mat) -> Vec<math::Mat> {
    let det = math::dot(
        [view[0][0], view[0][1], view[0][2]],
        math::cross(
            [view[1][0], view[1][1], view[1][2]],
            [view[2][0], view[2][1], view[2][2]],
        ),
    );
    let w = proj[3][2].signum();
    let k = det.signum() * w;
    let (near, far) = (0.5f32, 3000.0f32);
    let p = [
        [proj[0][0].signum(), 0.0, 0.0, 0.0],
        [0.0, proj[1][1].signum(), 0.0, 0.0],
        [0.0, 0.0, w * far / (far - near), -far * near / (far - near)],
        [0.0, 0.0, w, 0.0],
    ];
    let faces: [([f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
    ];
    faces
        .iter()
        .map(|&(f, u)| {
            let c = math::cross(u, f);
            let r = [c[0] * k, c[1] * k, c[2] * k];
            let z = [f[0] * w, f[1] * w, f[2] * w];
            let v = [
                [r[0], r[1], r[2], -math::dot(r, centre)],
                [u[0], u[1], u[2], -math::dot(u, centre)],
                [z[0], z[1], z[2], -math::dot(z, centre)],
                [0.0, 0.0, 0.0, 1.0],
            ];
            math::mul(&p, &v)
        })
        .collect()
}

/// What a scenery draw's bottom-level structure is built from: the key,
/// the vertex source, first vertex and stride, the index source and first
/// index, and the vertex count; `None` for draws rays cannot use (skinned,
/// cut out, instanced, positions not first in their vertex as three
/// floats, offsets not whole vertices).
#[allow(clippy::type_complexity)]
fn blas_input(
    r: &Renderer,
    i: &Item,
    arena: u64,
) -> Option<(BlasKey, Source, u32, u32, Option<(Source, u32)>, u32)> {
    let v = i.key.variant;
    if v.skinned
        || v.cloth
        || v.alpha_test
        || i.centre.is_some()
        || i.draw.topology != 4
        || i.draw.instances > 1
    {
        return None;
    }
    let slot = i.key.buffers.iter().position(|(_, inst, attrs)| {
        !inst
            && attrs
                .iter()
                .any(|&(loc, f, off)| loc == 0 && f == wgpu::VertexFormat::Float32x3 && off == 0)
    })?;
    let stride = i.key.buffers[slot].0;
    if stride < 12 || !stride.is_multiple_of(4) {
        return None;
    }
    let src = *i.vertex.get(slot)?;
    let (id, at, size) = match src {
        Source::Buffer(id, off) => (id, off, r.buffers.get(&id)?.buffer.size()),
        Source::Arena(at) => (Id::MAX, at, arena),
    };
    if at % u64::from(stride) != 0 {
        return None;
    }
    let base = u32::try_from(at / u64::from(stride)).ok()?;
    let d = &i.draw;
    match i.index {
        Some((isrc, format)) => {
            let isize = if format == wgpu::IndexFormat::Uint32 {
                4
            } else {
                2
            };
            let (iid, ioff) = match isrc {
                Source::Buffer(id, off) => (id, off),
                Source::Arena(at) => (Id::MAX, at),
            };
            if ioff % isize != 0 || d.base < 0 {
                return None;
            }
            let first_index = u32::try_from(ioff / isize).ok()? + d.start;
            let first_vertex = base + d.base as u32;
            let vertex_count =
                u32::try_from((size / u64::from(stride)).saturating_sub(u64::from(first_vertex)))
                    .ok()?;
            if vertex_count == 0 {
                return None;
            }
            let key = (
                id,
                first_vertex,
                stride,
                vertex_count,
                iid,
                first_index,
                d.count,
                isize == 4,
            );
            Some((
                key,
                src,
                first_vertex,
                stride,
                Some((isrc, first_index)),
                vertex_count,
            ))
        }
        None => {
            let first_vertex = base + d.start;
            let key = (id, first_vertex, stride, d.count, Id::MAX, 0, 0, false);
            Some((key, src, first_vertex, stride, None, d.count))
        }
    }
}

/// A one-pixel cube map, for water without an environment map.
fn one_cube(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 6,
    };
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("cube fallback"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        t.as_image_copy(),
        &rgba.repeat(6),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        size,
    );
    t.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    })
}

fn one_pixel(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> wgpu::TextureView {
    let t = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        t.as_image_copy(),
        &rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    t.create_view(&wgpu::TextureViewDescriptor::default())
}

impl Enhanced {
    #[must_use]
    #[allow(clippy::too_many_lines)] // every pass's layout and pipeline
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        families: Families,
        settings: Settings,
    ) -> Self {
        use wgpu::ShaderStages as S;
        let rt = device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera"),
            entries: &layout_entries(&[(0, uniform(CAMERA_SIZE as u64))], S::VERTEX_FRAGMENT),
        });
        let item_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("item"),
            entries: &layout_entries(
                &[
                    (0, uniform(ITEM_SIZE as u64)),
                    (1, uniform(5120)),
                    (2, tex(FLOAT)),
                    (
                        3,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                    (4, tex(FLOAT)),
                    (5, uniform(5120)),
                ],
                S::VERTEX_FRAGMENT,
            ),
        });
        let gbuffer_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&item_layout)],
            immediate_size: 0,
        });
        let lighting_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting"),
            entries: &layout_entries(
                &[
                    (0, uniform(LIGHT_SIZE as u64)),
                    (1, tex(UNFILTERED)),
                    (2, tex(UNFILTERED)),
                    (3, tex(UNFILTERED)),
                    (4, tex(wgpu::TextureSampleType::Depth)),
                    (
                        5,
                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                    ),
                    (
                        6,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    ),
                    (7, tex(UNFILTERED)),
                    (
                        8,
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(PROBES_SIZE as u64),
                        },
                    ),
                ]
                .into_iter()
                .chain(rt.then_some((
                    9,
                    wgpu::BindingType::AccelerationStructure {
                        vertex_return: false,
                    },
                )))
                .chain([
                    (
                        10,
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(lights::SIZE as u64),
                        },
                    ),
                    (11, tex(UNFILTERED)),
                ])
                .collect::<Vec<_>>(),
                S::FRAGMENT,
            ),
        });
        let lighting_source = format!(
            "{}{LIGHTING_WGSL}",
            if rt { RT_SHADOW_WGSL } else { NO_RT_WGSL }
        );
        let lighting = fullscreen(
            device,
            &lighting_layout,
            &lighting_source,
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
        );
        let ao_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ao"),
            entries: &layout_entries(
                &[
                    (0, uniform(AO_SIZE as u64)),
                    (1, tex(wgpu::TextureSampleType::Depth)),
                    (2, tex(UNFILTERED)),
                    (3, tex(UNFILTERED)),
                ],
                S::FRAGMENT,
            ),
        });
        let r8 = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::R8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let ao_pass = fullscreen(device, &ao_layout, AO_WGSL, &r8, None);
        let ao_blur = fullscreen_entry(device, &ao_layout, AO_WGSL, "fs_blur", &r8);
        let gen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material"),
            entries: &layout_entries(
                &[
                    (0, uniform(32)),
                    (1, tex(FLOAT)),
                    (
                        2,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                ],
                S::FRAGMENT,
            ),
        });
        let rgba8 = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let material_gen = (
            fullscreen_entry(device, &gen_layout, MATERIAL_WGSL, "fs", &rgba8),
            fullscreen_entry(device, &gen_layout, MATERIAL_WGSL, "fs_down", &rgba8),
            gen_layout,
        );
        let tone_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tone"),
            entries: &layout_entries(
                &[
                    (0, uniform(POST_SIZE as u64)),
                    (1, tex(UNFILTERED)),
                    (2, tex(UNFILTERED)),
                    (3, tex(wgpu::TextureSampleType::Depth)),
                    (4, tex(FLOAT)),
                    (6, tex(FLOAT)),
                    (
                        5,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                ],
                S::FRAGMENT,
            ),
        });
        let tone = (
            fullscreen(device, &tone_layout, OUTPUT_WGSL, &rgba8, None),
            tone_layout,
        );
        let taa_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa"),
            entries: &layout_entries(
                &[
                    (0, uniform(TAA_SIZE as u64)),
                    (1, tex(UNFILTERED)),
                    (2, tex(FLOAT)),
                    (3, tex(wgpu::TextureSampleType::Depth)),
                    (
                        4,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                    (5, tex(UNFILTERED)),
                ],
                S::FRAGMENT,
            ),
        });
        let taa = (
            fullscreen(
                device,
                &taa_layout,
                TAA_WGSL,
                &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                None,
            ),
            taa_layout,
        );
        let vol_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("volumetric"),
            entries: &layout_entries(
                &[
                    (0, uniform(VOL_SIZE as u64)),
                    (1, tex(wgpu::TextureSampleType::Depth)),
                    (
                        2,
                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                    ),
                    (
                        3,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    ),
                ],
                S::FRAGMENT,
            ),
        });
        let rgba16 = [Some(wgpu::ColorTargetState {
            format: wgpu::TextureFormat::Rgba16Float,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let volumetric = (
            fullscreen(device, &vol_layout, VOLUMETRIC_WGSL, &rgba16, None),
            vol_layout,
        );
        let cube_tex = wgpu::BindingType::Texture {
            sample_type: FLOAT,
            view_dimension: wgpu::TextureViewDimension::Cube,
            multisampled: false,
        };
        let water_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water"),
            entries: &layout_entries(
                &[
                    (0, uniform(WATER_SIZE as u64)),
                    (1, tex(FLOAT)),
                    (2, tex(FLOAT)),
                    (3, tex(FLOAT)),
                    (4, tex(FLOAT)),
                    (5, cube_tex),
                    (
                        6,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                ],
                S::VERTEX_FRAGMENT,
            ),
        });
        let water_scene_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("water scene"),
                entries: &layout_entries(
                    &[
                        (0, uniform(WATER_SCENE_SIZE as u64)),
                        (1, tex(FLOAT)),
                        (2, tex(wgpu::TextureSampleType::Depth)),
                        (
                            3,
                            wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Depth,
                                view_dimension: wgpu::TextureViewDimension::D2Array,
                                multisampled: false,
                            },
                        ),
                        (
                            4,
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        ),
                        (
                            5,
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        ),
                    ],
                    S::FRAGMENT,
                ),
            });
        let water_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("water"),
                bind_group_layouts: &[
                    Some(&camera_layout),
                    Some(&water_layout),
                    Some(&water_scene_layout),
                ],
                immediate_size: 0,
            });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let compare = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Self {
            settings,
            families,
            reflect: HashMap::new(),
            pipelines: HashMap::new(),
            camera_layout,
            item_layout,
            gbuffer_layout,
            lighting: (lighting, lighting_layout),
            ao: (ao_pass, ao_blur, ao_layout),
            material_gen,
            tone,
            volumetric,
            water_layouts: (water_layout, water_scene_layout, water_pipeline_layout),
            water_pipelines: HashMap::new(),
            probes: probe_targets(device),
            light_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("local lights"),
                size: lights::SIZE as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            lights: Vec::new(),
            sun: lights::Sun::default(),
            glow_edits: Vec::new(),
            material_edits: HashMap::new(),
            selected: None,
            pick: None,
            picked: None,
            glow: glow::Glow::new(device),
            rt,
            blas_cache: HashMap::new(),
            tlas: None,
            taa,
            post: post::Post::new(device),
            last_view_proj: None,
            taa_last: None,
            taa_index: 0,
            motion: HashMap::new(),
            effect: None,
            effect_generation: 0,
            frame_count: 0,
            materials: HashMap::new(),
            material_dir: None,
            texture_dir: None,
            #[cfg(windows)]
            fsr: None,
            fsr_tried: false,
            fsr_dir: None,
            fsr_motion: None,
            last_render: None,
            replacements: HashMap::new(),
            export_dir: None,
            exported: std::collections::HashSet::new(),
            flat_material: one_pixel(device, queue, [128, 128, 179, 255]),
            output: None,
            targets: None,
            shadow: None,
            white: one_pixel(device, queue, [255; 4]),
            black: one_pixel(device, queue, [0, 0, 0, 255]),
            cube: one_cube(device, queue, [40, 48, 56, 255]),
            linear,
            compare,
            notes: HashMap::new(),
            map_lights: None,
            last_items: 0,
            last_other: 0,
        }
    }

    /// Reads the map's lights from a character draw's pixel shader
    /// (`Light0`, `Light1`: direction towards the light, then colour;
    /// ambient from the scene).
    fn read_map_lights(&mut self, r: &Renderer, d: &Draw) -> Option<MapLights> {
        let ps = self.reflect(r, d.ps)?;
        let cb = &d.ps_cb.iter().find(|c| c.0 == 0)?.1;
        let light = |name: &str| -> Option<([f32; 3], [f32; 3])> {
            let (off, _) = *ps.vars.get(name)?;
            let f = |at: u32| {
                cb.get(at as usize..at as usize + 12).map(|b| {
                    let v: Vec<f32> = b
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|x| f32::from_le_bytes(*x))
                        .collect();
                    [v[0], v[1], v[2]]
                })
            };
            let dir = f(off)?;
            let color = f(off + 16)?;
            Some(([-dir[0], -dir[1], -dir[2]], linear(color)))
        };
        let key = light("Light0")?;
        let fill = light("Light1").unwrap_or(([0.0, -1.0, 0.0], [0.0; 3]));
        let ambient = ps
            .floats(cb, "CharacterAmbientLight", 3)
            .or_else(|| ps.floats(cb, "scene.GlobalAmbientColor", 3))
            .map_or([0.1; 3], |a| linear([a[0], a[1], a[2]]));
        Some(MapLights { key, fill, ambient })
    }

    fn note(&mut self, what: String) {
        *self.notes.entry(what).or_default() += 1;
    }

    fn reflect(&mut self, r: &Renderer, id: Id) -> Option<Arc<Reflect>> {
        let s = r.shaders.get(&id)?;
        if let Some(x) = self.reflect.get(&s.hash) {
            return x.clone();
        }
        let x = Reflect::parse(&s.bytecode).map(Arc::new);
        self.reflect.insert(s.hash, x.clone());
        x
    }

    fn family(&self, r: &Renderer, d: &Draw) -> Family {
        let f = |id: Id| {
            r.shaders
                .get(&id)
                .map_or(Family::Other, |s| self.families.get(s.hash))
        };
        match f(d.vs) {
            Family::Other => f(d.ps),
            x => x,
        }
    }

    #[allow(clippy::too_many_lines)] // one pipeline description
    fn pipeline(&mut self, device: &wgpu::Device, key: &GKey) -> Option<Arc<wgpu::RenderPipeline>> {
        if let Some(p) = self.pipelines.get(key) {
            return p.clone();
        }
        let v = key.variant;
        // The vertex input struct: the attributes the draw has.
        let present: Vec<u32> = key
            .buffers
            .iter()
            .flat_map(|b| b.2.iter().map(|a| a.0))
            .collect();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gbuffer"),
            source: wgpu::ShaderSource::Wgsl(gbuffer_source(v, &present).into()),
        });
        let attrs: Vec<Vec<wgpu::VertexAttribute>> = key
            .buffers
            .iter()
            .map(|(_, _, a)| {
                a.iter()
                    .map(|&(location, format, offset)| wgpu::VertexAttribute {
                        format,
                        offset,
                        shader_location: location,
                    })
                    .collect()
            })
            .collect();
        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = key
            .buffers
            .iter()
            .zip(&attrs)
            .map(|((stride, inst, _), a)| {
                Some(wgpu::VertexBufferLayout {
                    array_stride: u64::from(*stride),
                    step_mode: if *inst {
                        wgpu::VertexStepMode::Instance
                    } else {
                        wgpu::VertexStepMode::Vertex
                    },
                    attributes: a,
                })
            })
            .collect();
        let gtargets = [
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R32Uint,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            }),
        ];
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(if v.shadow { "shadow" } else { "gbuffer" }),
            layout: Some(&self.gbuffer_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: key.front,
                cull_mode: if v.shadow { None } else { key.cull },
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: if v.shadow {
                    wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    }
                } else {
                    wgpu::DepthBiasState::default()
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(if v.shadow { "fs_shadow" } else { "fs" }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: if v.shadow { &[] } else { &gtargets },
            }),
            multiview_mask: None,
            cache: None,
        });
        let result = match pollster::block_on(scope.pop()) {
            Some(e) => {
                self.note(format!("gbuffer pipeline {v:?}: {e}"));
                None
            }
            None => Some(Arc::new(pipeline)),
        };
        self.pipelines.insert(key.clone(), result.clone());
        result
    }

    fn targets(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let make = |format: wgpu::TextureFormat, label: &str| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size.0,
                        height: size.1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let plain = |format: wgpu::TextureFormat, label: &str| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let v = t.create_view(&wgpu::TextureViewDescriptor::default());
            (t, v)
        };
        let hdr_texture = plain(wgpu::TextureFormat::Rgba16Float, "hdr");
        let id_texture = plain(wgpu::TextureFormat::R32Uint, "draw ids");
        let ldr_texture = plain(wgpu::TextureFormat::Rgba8Unorm, "ldr");
        let depth_texture = plain(wgpu::TextureFormat::Depth32Float, "depth");
        self.targets = Some(Targets {
            size,
            albedo: make(wgpu::TextureFormat::Rgba8UnormSrgb, "albedo"),
            normal: make(wgpu::TextureFormat::Rgba16Float, "normal"),
            baked: make(wgpu::TextureFormat::Rgba16Float, "baked"),
            motion: make(wgpu::TextureFormat::Rgba16Float, "motion"),
            material: make(wgpu::TextureFormat::Rgba8Unorm, "material"),
            id: id_texture.1,
            id_texture: id_texture.0,
            depth: depth_texture.1,
            hdr: hdr_texture.1,
            ao: [
                make(wgpu::TextureFormat::R8Unorm, "ao"),
                make(wgpu::TextureFormat::R8Unorm, "ao blur"),
            ],
            ldr: ldr_texture.1,
            ldr_texture: ldr_texture.0,
            history: [
                make(wgpu::TextureFormat::Rgba16Float, "taa history"),
                make(wgpu::TextureFormat::Rgba16Float, "taa history"),
            ],
            hdr_texture: hdr_texture.0,
            depth_texture: depth_texture.0,
            hdr_copy: plain(wgpu::TextureFormat::Rgba16Float, "hdr copy"),
            depth_copy: plain(wgpu::TextureFormat::Depth32Float, "depth copy"),
            vol: device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("volumetric"),
                    size: wgpu::Extent3d {
                        width: (size.0 / 2).max(1),
                        height: (size.1 / 2).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
        });
    }

    fn shadow_map(
        &mut self,
        device: &wgpu::Device,
    ) -> (wgpu::TextureView, wgpu::TextureView, Vec<wgpu::TextureView>) {
        let size = self.settings.shadow_size.clamp(256, 8192);
        if let Some((s, a, b, l)) = &self.shadow
            && *s == size
        {
            return (a.clone(), b.clone(), l.clone());
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let array = |base| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_array_layer: base,
                array_layer_count: Some(3),
                ..Default::default()
            })
        };
        let (all, rest) = (array(0), array(3));
        let layers: Vec<wgpu::TextureView> = (0..6)
            .map(|l| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: l,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        self.shadow = Some((size, all.clone(), rest.clone(), layers.clone()));
        (all, rest, layers)
    }

    /// The material map of colour texture `resource` (made on first use,
    /// again when the texture changes).
    /// An image from the `textures` folder (PNG), with mip levels, as a
    /// draw's colour texture; `None` when it cannot be read (noted once).
    fn replacement(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        file: &str,
    ) -> Option<wgpu::TextureView> {
        if let Some(v) = self.replacements.get(file) {
            return v.clone();
        }
        let path = self.texture_dir.as_ref()?.join(file);
        let image = std::fs::read(&path)
            .ok()
            .and_then(|b| crate::png::decode(&b));
        let view = image.map(|(w, h, rgba)| {
            let mips = 32 - w.max(h).leading_zeros();
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("replacement"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let (mut lw, mut lh, mut level) = (w, h, rgba);
            for mip in 0..mips {
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &level,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(lw * 4),
                        rows_per_image: Some(lh),
                    },
                    wgpu::Extent3d {
                        width: lw,
                        height: lh,
                        depth_or_array_layers: 1,
                    },
                );
                // The next level: each pixel the mean of the 2×2 under it.
                let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
                let mut next = vec![0u8; (nw * nh * 4) as usize];
                for y in 0..nh {
                    for x in 0..nw {
                        for c in 0..4 {
                            let at = |px: u32, py: u32| {
                                u32::from(
                                    level
                                        [((py.min(lh - 1) * lw + px.min(lw - 1)) * 4 + c) as usize],
                                )
                            };
                            let sum = at(2 * x, 2 * y)
                                + at(2 * x + 1, 2 * y)
                                + at(2 * x, 2 * y + 1)
                                + at(2 * x + 1, 2 * y + 1);
                            next[((y * nw + x) * 4 + c) as usize] = (sum / 4) as u8;
                        }
                    }
                }
                (lw, lh, level) = (nw, nh, next);
            }
            texture.create_view(&wgpu::TextureViewDescriptor::default())
        });
        if view.is_none() {
            self.note(format!("replacement {} unreadable", path.display()));
        }
        self.replacements.insert(file.to_owned(), view.clone());
        view
    }

    /// Forgets loaded replacement images (the files may have changed).
    pub fn reload_replacements(&mut self) {
        self.replacements.clear();
    }

    #[allow(clippy::too_many_lines)] // the pack or the generator, the mips, the export
    fn material(&mut self, r: &Renderer, resource: Id) -> Option<wgpu::TextureView> {
        let t = r.textures.get(&resource)?;
        let version = t.desc[7];
        if let Some((v, view)) = self.materials.get(&resource)
            && *v == version
        {
            return Some(view.clone());
        }
        if t.format.is_depth_stencil_format() || t.desc[5] > 1 {
            return None;
        }
        let device = &r.device;
        // A pack's map for this texture, if there is one.
        let pack = self
            .material_dir
            .as_ref()
            .filter(|_| t.hash != 0)
            .and_then(|d| std::fs::read(d.join(format!("{:016x}.png", t.hash))).ok())
            .and_then(|b| crate::png::decode(&b));
        let (w, h) = pack
            .as_ref()
            .map_or((t.desc[0].max(1), t.desc[1].max(1)), |p| (p.0, p.1));
        let mips = if pack.is_some() {
            32 - w.max(h).leading_zeros()
        } else {
            t.desc[2].max(1).min(32 - w.max(h).leading_zeros())
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("material"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if let Some((pw, ph, pixels)) = &pack {
            r.queue.write_texture(
                texture.as_image_copy(),
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pw * 4),
                    rows_per_image: Some(*ph),
                },
                wgpu::Extent3d {
                    width: *pw,
                    height: *ph,
                    depth_or_array_layers: 1,
                },
            );
        }
        let s = &self.settings;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("material"),
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 256 * u64::from(mips),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let first = u32::from(pack.is_some());
        for level in first..mips {
            let mut u = Vec::new();
            put4(&mut u, [2.0, s.roughness, 0.25, 0.0]);
            put4(
                &mut u,
                [
                    if level == 0 { 0.0 } else { (level - 1) as f32 },
                    0.0,
                    0.0,
                    0.0,
                ],
            );
            r.queue.write_buffer(&ubuf, 256 * u64::from(level), &u);
            let source = if level == 0 {
                t.texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(t.format.add_srgb_suffix()),
                    ..Default::default()
                })
            } else {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level - 1,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            };
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.material_gen.2,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &ubuf,
                            offset: 256 * u64::from(level),
                            size: wgpu::BufferSize::new(32),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                ],
            });
            let target = texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("material"),
                color_attachments: &[attachment(&target, wgpu::LoadOp::Clear(wgpu::Color::BLACK))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(if level == 0 {
                &self.material_gen.0
            } else {
                &self.material_gen.1
            });
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        r.queue.submit([enc.finish()]);
        if let Some(dir) = &self.export_dir
            && pack.is_none()
            && t.hash != 0
            && self.exported.insert(t.hash)
        {
            let name = format!("{:016x}", t.hash);
            if let Ok(m) = r.read_texture(&texture, w, h) {
                let _ = std::fs::write(
                    dir.join(format!("{name}.png")),
                    crate::png::encode(w, h, &m),
                );
            }
            if let Ok((aw, ah, a)) = r.read_rgba(resource) {
                let _ = std::fs::write(
                    dir.join(format!("{name}_albedo.png")),
                    crate::png::encode(aw, ah, &a),
                );
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.materials.insert(resource, (version, view.clone()));
        Some(view)
    }

    /// The water pipeline for a geometry layout.
    #[allow(clippy::too_many_lines)] // one pipeline description
    fn water_pipeline(
        &mut self,
        device: &wgpu::Device,
        key: &GKey,
    ) -> Option<Arc<wgpu::RenderPipeline>> {
        if let Some(p) = self.water_pipelines.get(key) {
            return p.clone();
        }
        let present: Vec<u32> = key
            .buffers
            .iter()
            .flat_map(|b| b.2.iter().map(|a| a.0))
            .collect();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water"),
            source: wgpu::ShaderSource::Wgsl(water_source(&present).into()),
        });
        let attrs: Vec<Vec<wgpu::VertexAttribute>> = key
            .buffers
            .iter()
            .map(|(_, _, a)| {
                a.iter()
                    .map(|&(location, format, offset)| wgpu::VertexAttribute {
                        format,
                        offset,
                        shader_location: location,
                    })
                    .collect()
            })
            .collect();
        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = key
            .buffers
            .iter()
            .zip(&attrs)
            .map(|((stride, inst, _), a)| {
                Some(wgpu::VertexBufferLayout {
                    array_stride: u64::from(*stride),
                    step_mode: if *inst {
                        wgpu::VertexStepMode::Instance
                    } else {
                        wgpu::VertexStepMode::Vertex
                    },
                    attributes: a,
                })
            })
            .collect();
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water"),
            layout: Some(&self.water_layouts.2),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: key.front,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let result = match pollster::block_on(scope.pop()) {
            Some(e) => {
                self.note(format!("water pipeline: {e}"));
                None
            }
            None => Some(Arc::new(pipeline)),
        };
        self.water_pipelines.insert(key.clone(), result.clone());
        result
    }

    /// Builds the enhanced water of a water draw.
    #[allow(clippy::too_many_lines)] // one walk through the draw's inputs
    fn water_item(
        &mut self,
        r: &Renderer,
        d: &Draw,
        time: f32,
        mut sources: impl FnMut(Id, u32) -> Option<Source>,
    ) -> Option<WaterItem> {
        if d.topology != 4 {
            return None;
        }
        let vs = self.reflect(r, d.vs)?;
        let ps = self.reflect(r, d.ps)?;
        let vcb = d
            .vs_cb
            .iter()
            .find(|c| c.0 == 0)
            .map_or(&[][..], |c| &c.1[..]);
        let pcb = d
            .ps_cb
            .iter()
            .find(|c| c.0 == 0)
            .map_or(&[][..], |c| &c.1[..]);
        let elements = r.layouts.get(&d.layout)?.clone();
        let mut per_slot: Vec<SlotLayout> = Vec::new();
        let mut offsets = [0u32; 8];
        let mut have_pos = false;
        for e in &elements {
            let name = format!("{}{}", e.semantic.to_ascii_uppercase(), e.index);
            let slot = e.slot.min(7) as usize;
            let offset = if e.offset == u32::MAX {
                offsets[slot]
            } else {
                e.offset
            };
            let Some(format) = formats::vertex_format(e.format) else {
                continue;
            };
            offsets[slot] = offset + format.size() as u32;
            let Some(location) = WATER_INPUTS.iter().position(|i| *i == name) else {
                continue;
            };
            if !vs.inputs.contains(&name) {
                continue;
            }
            have_pos |= location == 0;
            let stride = d
                .vertex
                .iter()
                .find(|v| u32::from(v.0) == e.slot)
                .map_or(0, |v| v.2);
            let format = crate::replay::shrink_format(format, stride.saturating_sub(offset));
            let entry = match per_slot.iter_mut().position(|p| p.0 == e.slot) {
                Some(i) => &mut per_slot[i],
                None => {
                    per_slot.push((e.slot, e.per_instance, Vec::new()));
                    per_slot.last_mut().expect("pushed")
                }
            };
            entry.2.push((location as u32, format, u64::from(offset)));
        }
        if !have_pos {
            return None;
        }
        per_slot.sort_by_key(|p| p.0);
        let (_, front, _, _) = formats::raster(r.state(d.raster, StateKind::Raster));
        let key = GKey {
            variant: Variant::default(),
            buffers: per_slot
                .iter()
                .map(|(s, i, a)| {
                    (
                        d.vertex
                            .iter()
                            .find(|v| u32::from(v.0) == *s)
                            .map_or(0, |v| v.2),
                        *i,
                        a.clone(),
                    )
                })
                .collect(),
            cull: None,
            front,
        };
        let vertex: Vec<Source> = per_slot
            .iter()
            .map(|(s, _, _)| {
                let v = d.vertex.iter().find(|v| u32::from(v.0) == *s)?;
                sources(v.1, v.3)
            })
            .collect::<Option<_>>()?;
        let index = match d.kind {
            ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced => Some((
                sources(d.index.0, d.index.2)?,
                if d.index.1 == 42 {
                    wgpu::IndexFormat::Uint32
                } else {
                    wgpu::IndexFormat::Uint16
                },
            )),
            _ => None,
        };
        let f = |name: &str| vs.floats(vcb, name, 1).map_or(0.0, |v| v[0]);
        let scroll = |x: &str, y: &str| [(f(x) * time).fract(), (f(y) * time).fract()];
        let t0 = scroll("ScrollSpeed_tex0_X", "ScrollSpeed_tex0_Y");
        let t1 = scroll("ScrollSpeed_tex1_X", "ScrollSpeed_tex1_Y");
        let n0 = scroll("ScrollSpeed_nml1_X", "ScrollSpeed_nml1_Y");
        let n1 = scroll("ScrollSpeed_nml2_X", "ScrollSpeed_nml2_Y");
        let mut u = Vec::with_capacity(WATER_SIZE);
        put_rows(
            &mut u,
            &vs.rows(vcb, "World").unwrap_or_else(math::identity),
        );
        put4(&mut u, [t0[0], t0[1], t1[0], t1[1]]);
        put4(&mut u, [n0[0], n0[1], n1[0], n1[1]]);
        let color = ps
            .floats(pcb, "MaterialColour", 4)
            .unwrap_or_else(|| vec![1.0; 4]);
        put4(&mut u, [color[0], color[1], color[2], color[3]]);
        let spec = ps
            .floats(pcb, "SpecularColor", 3)
            .unwrap_or_else(|| vec![1.0; 3]);
        let blend = ps
            .floats(pcb, "MultiTexBlendFactor", 1)
            .map_or(0.5, |v| v[0]);
        put4(&mut u, [spec[0], spec[1], spec[2], blend]);
        let blended = formats::blend(r.state(d.blend, StateKind::Blend), 0)
            .0
            .is_some();
        put4(&mut u, [if blended { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0]);
        let view = |name: &str| -> Id {
            ps.texture_slots
                .get(name)
                .and_then(|t| d.ps_srv.iter().find(|x| u32::from(x.0) == *t))
                .map_or(0, |x| x.1)
        };
        let views = [
            view("TextureSampler"),
            view("TextureSampler1"),
            view("NormalMapSampler"),
            view("NormalMap2Sampler"),
            view("CubeMapSampler"),
        ];
        let sampler = ps
            .sampler_slots
            .get("TextureSamplerState")
            .and_then(|s| d.ps_samplers.iter().find(|x| u32::from(x.0) == *s))
            .map_or(0, |x| x.1);
        assert_eq!(u.len(), WATER_SIZE, "water uniform layout");
        Some(WaterItem {
            key,
            vertex,
            index,
            draw: Box::new(d.clone()),
            uniform: u,
            views,
            sampler,
        })
    }

    /// Builds the G-buffer item of a draw, or `None` when the enhanced
    /// passes cannot draw it (it then goes through the game's shader).
    #[allow(clippy::too_many_lines)]
    fn item(
        &mut self,
        r: &Renderer,
        d: &Draw,
        family: Family,
        time: f32,
        mut sources: impl FnMut(Id, u32) -> Option<Source>,
    ) -> Option<Item> {
        if !matches!(
            family,
            Family::Scenery | Family::Character | Family::UvAnimation | Family::Ice
        ) {
            return None;
        }
        if d.topology != 4 {
            return None;
        }
        // Blended draws keep the game's shading.
        let blend = r.state(d.blend, StateKind::Blend).map(<[u8]>::to_vec);
        if formats::blend(blend.as_deref(), 0).0.is_some() {
            return None;
        }
        let vs = self.reflect(r, d.vs)?;
        let ps = self.reflect(r, d.ps)?;
        let vcb = d
            .vs_cb
            .iter()
            .find(|c| c.0 == 0)
            .map_or(&[][..], |c| &c.1[..]);
        let pcb = d
            .ps_cb
            .iter()
            .find(|c| c.0 == 0)
            .map_or(&[][..], |c| &c.1[..]);
        let elements = r.layouts.get(&d.layout)?.clone();
        // Attributes by semantic.
        let mut per_slot: Vec<SlotLayout> = Vec::new();
        let mut have = [false; 7];
        let mut offsets = [0u32; 8];
        for e in &elements {
            let name = format!("{}{}", e.semantic.to_ascii_uppercase(), e.index);
            let slot = e.slot.min(7) as usize;
            let offset = if e.offset == u32::MAX {
                offsets[slot]
            } else {
                e.offset
            };
            let Some(format) = formats::vertex_format(e.format) else {
                continue;
            };
            offsets[slot] = offset + format.size() as u32;
            let Some(location) = INPUTS.iter().position(|i| *i == name) else {
                continue;
            };
            // Only what the game's own shader reads.
            if !vs.inputs.contains(&name) {
                continue;
            }
            let stride = d
                .vertex
                .iter()
                .find(|v| u32::from(v.0) == e.slot)
                .map_or(0, |v| v.2);
            let format = crate::replay::shrink_format(format, stride.saturating_sub(offset));
            have[location] = true;
            let entry = match per_slot.iter_mut().position(|p| p.0 == e.slot) {
                Some(i) => &mut per_slot[i],
                None => {
                    per_slot.push((e.slot, e.per_instance, Vec::new()));
                    per_slot.last_mut().expect("pushed")
                }
            };
            entry.2.push((location as u32, format, u64::from(offset)));
        }
        if !have[0] || !have[1] {
            return None;
        }
        let bones = vs.vars.contains_key("BoneTransforms") && have[4] && have[5];
        let variant = Variant {
            skinned: bones,
            cloth: bones && have[6] && vs.vars.contains_key("ClothWorld"),
            color: have[2],
            uv: have[3] && ps.texture_slots.contains_key("TextureSampler"),
            alpha_test: ps.used.contains("AlphaThreshold"),
            shadow: false,
        };
        if !variant.cloth {
            // Without the cloth path, COLOR1 is not read.
            for p in &mut per_slot {
                p.2.retain(|a| a.0 != 6);
            }
            per_slot.retain(|p| !p.2.is_empty());
        }
        per_slot.sort_by_key(|p| p.0);
        let (cull, front, _, _) = formats::raster(r.state(d.raster, StateKind::Raster));
        let key = GKey {
            variant,
            buffers: per_slot
                .iter()
                .map(|(s, i, a)| {
                    (
                        d.vertex
                            .iter()
                            .find(|v| u32::from(v.0) == *s)
                            .map_or(0, |v| v.2),
                        *i,
                        a.clone(),
                    )
                })
                .collect(),
            cull,
            front,
        };
        let vertex: Vec<Source> = per_slot
            .iter()
            .map(|(s, _, _)| {
                let v = d.vertex.iter().find(|v| u32::from(v.0) == *s)?;
                sources(v.1, v.3)
            })
            .collect::<Option<_>>()?;
        let index = match d.kind {
            ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced => Some((
                sources(d.index.0, d.index.2)?,
                if d.index.1 == 42 {
                    wgpu::IndexFormat::Uint32
                } else {
                    wgpu::IndexFormat::Uint16
                },
            )),
            _ => None,
        };
        // The uniform.
        let mut u = Vec::with_capacity(ITEM_BASE);
        put_rows(
            &mut u,
            &vs.rows(vcb, "World").unwrap_or_else(math::identity),
        );
        put_rows(
            &mut u,
            &vs.rows(vcb, "ClothWorld").unwrap_or_else(math::identity),
        );
        let color = ps
            .floats(pcb, "MaterialColour", 4)
            .or_else(|| vs.floats(vcb, "MaterialColour", 4))
            .unwrap_or_else(|| vec![1.0; 4]);
        put4(&mut u, [color[0], color[1], color[2], color[3]]);
        let threshold = ps.floats(pcb, "AlphaThreshold", 1).map_or(0.5, |v| v[0]);
        let scroll = if family == Family::UvAnimation {
            let x = vs.floats(vcb, "ScrollSpeed_X", 1).map_or(0.0, |v| v[0]);
            let y = vs.floats(vcb, "ScrollSpeed_Y", 1).map_or(0.0, |v| v[0]);
            [(x * time).fract(), (y * time).fract()]
        } else {
            [0.0, 0.0]
        };
        let material = if family == Family::Character {
            1.0
        } else {
            0.0
        };
        put4(&mut u, [threshold, scroll[0], scroll[1], material]);
        let ambient = vs
            .floats(vcb, "CharacterAmbientLight", 3)
            .map_or([0.5, 0.5, 0.5], |a| [a[0], a[1], a[2]]);
        let ambient = if family == Family::Character {
            let s = self.settings.character_ambient;
            [ambient[0] * s, ambient[1] * s, ambient[2] * s]
        } else {
            ambient
        };
        let fg = ps.floats(pcb, "fgEff", 3).unwrap_or_else(|| vec![1.0; 3]);
        let scale = ps
            .floats(pcb, "clrScaleFactor", 4)
            .unwrap_or_else(|| vec![1.0; 4]);
        let fog = (fg[0] + fg[1] + fg[2]) / 3.0 * (scale[0] + scale[1] + scale[2]) / 3.0;
        put4(&mut u, [ambient[0], ambient[1], ambient[2], fog]);
        put4(&mut u, [scale[0], scale[1], scale[2], scale[3]]);
        let detail = if family == Family::Character {
            self.settings.character_detail
        } else {
            1.0
        };
        let texture = variant
            .uv
            .then(|| ps.texture_slots["TextureSampler"])
            .and_then(|t| d.ps_srv.iter().find(|x| u32::from(x.0) == t))
            .and_then(|x| r.views.get(&x.1))
            .and_then(|v| r.textures.get(&v.resource))
            .map_or(0, |t| t.hash);
        let m = materials::resolve(
            self.material_edits.get(&texture),
            family == Family::Character,
            &self.settings,
        );
        put4(
            &mut u,
            [detail * m.detail, m.roughness, m.metallic, m.specular],
        );
        put4(
            &mut u,
            [m.emission, m.skin, m.skin_roughness, m.skin_detail],
        );
        // w: the draw's id, set when the frame is put together.
        put4(&mut u, [m.tint[0], m.tint[1], m.tint[2], 0.0]);
        let bones_data = variant.skinned.then(|| {
            let (off, _) = vs.vars["BoneTransforms"];
            let mut b = vcb
                .get(off as usize..)
                .map_or_else(Vec::new, |s| s[..s.len().min(5120)].to_vec());
            b.resize(5120, 0);
            b
        });
        let albedo = variant.uv.then(|| {
            let t = ps.texture_slots["TextureSampler"];
            let s = ps
                .sampler_slots
                .get("TextureSamplerState")
                .copied()
                .unwrap_or(t);
            let view = d
                .ps_srv
                .iter()
                .find(|x| u32::from(x.0) == t)
                .map_or(0, |x| x.1);
            let samp = d
                .ps_samplers
                .iter()
                .find(|x| u32::from(x.0) == s)
                .map_or(0, |x| x.1);
            (view, samp)
        });
        let centre = (family == Family::Character).then(|| {
            let world = vs.rows(vcb, "World").unwrap_or_else(math::identity);
            let mut at = [world[0][3], world[1][3], world[2][3]];
            if let Some(b) = &bones_data {
                // The mean of the bones' positions.
                let f = |o: usize| f32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"));
                let mut sum = [0.0f32; 3];
                let mut n = 0.0f32;
                for bone in 0..b.len() / 64 {
                    let o = bone * 64;
                    if f(o).abs() + f(o + 4).abs() + f(o + 8).abs() < 1e-4 {
                        continue;
                    }
                    for (r, s) in sum.iter_mut().enumerate() {
                        *s += f(o + r * 16 + 12);
                    }
                    n += 1.0;
                }
                if n > 0.0 {
                    at = sum.map(|s| s / n);
                }
            }
            at
        });
        debug_assert_eq!(u.len(), ITEM_BASE, "item uniform layout");
        Some(Item {
            key,
            vertex,
            index,
            draw: Box::new(d.clone()),
            uniform: u,
            bones: bones_data,
            albedo,
            centre,
            motion: motion_key(d),
            texture,
        })
    }

    /// Draws `frame` the enhanced way into its scene targets.
    ///
    /// # Errors
    /// When the GPU reports an error.
    #[allow(clippy::too_many_lines)]
    pub fn render(&mut self, r: &mut Renderer, frame: &Frame) -> Result<()> {
        self.render_until(r, frame, None)
    }

    /// Like [`Enhanced::render`], stopping after `until` draws.
    ///
    /// # Errors
    /// When the GPU reports an error.
    #[allow(clippy::too_many_lines)] // the frame's passes, in order
    pub fn render_until(
        &mut self,
        r: &mut Renderer,
        frame: &Frame,
        until: Option<usize>,
    ) -> Result<()> {
        let mut seen = 0usize;
        let cut: Vec<Cmd> = frame
            .cmds
            .iter()
            .take_while(|c| {
                if matches!(c, Cmd::Draw(_)) {
                    seen += 1;
                }
                until.is_none_or(|u| seen <= u)
            })
            .cloned()
            .collect();
        let frame = &Frame {
            number: frame.number,
            width: frame.width,
            height: frame.height,
            cmds: cut,
        };
        // The scene targets and camera come from the first scene draw.
        let Some(first) = frame.cmds.iter().find_map(|c| match c {
            Cmd::Draw(d) if d.rtv.len() >= 2 && d.dsv != 0 => Some(d.clone()),
            _ => None,
        }) else {
            self.note("no scene draw".into());
            return r.draw_frame(frame, &Filter::default());
        };
        let (color_view, normal_view, depth_view) = (first.rtv[0].1, first.rtv[1].1, first.dsv);
        let Some(size) = r
            .views
            .get(&color_view)
            .and_then(|v| r.textures.get(&v.resource))
            .map(|t| (t.desc[0], t.desc[1]))
        else {
            self.note("no scene target".into());
            return r.draw_frame(frame, &Filter::default());
        };
        // Camera: the scene struct of the first draw whose shader has it.
        let mut camera = None;
        for c in &frame.cmds {
            if let Cmd::Draw(d) = c
                && let Some(vs) = self.reflect(r, d.vs)
                && let Some(cb) = d.vs_cb.iter().find(|c| c.0 == 0)
                && let Some(vp) = vs.rows(&cb.1, "scene.ViewProjection")
            {
                let view = vs.rows(&cb.1, "scene.View").unwrap_or_else(math::identity);
                let eye = vs
                    .floats(&cb.1, "scene.EyePosition", 3)
                    .unwrap_or_else(|| vec![0.0; 3]);
                let fog = vs
                    .floats(&cb.1, "scene.FogColor", 3)
                    .unwrap_or_else(|| vec![0.0; 3]);
                let fogp = vs
                    .floats(&cb.1, "scene.FogNear", 4)
                    .unwrap_or_else(|| vec![0.0, 1e9, 1.0, 0.0]);
                let time = vs
                    .floats(&cb.1, "scene.GlobalTime", 1)
                    .map_or(0.0, |t| t[0]);
                camera = Some((
                    vp,
                    view,
                    [eye[0], eye[1], eye[2]],
                    [fog[0], fog[1], fog[2]],
                    [fogp[0], fogp[1], fogp[2], fogp[3]],
                    time,
                ));
                break;
            }
        }
        let Some((view_proj, view, eye, fog_color, fog_params, time)) = camera else {
            self.note("no camera".into());
            return r.draw_frame(frame, &Filter::default());
        };
        // TAA: a sub-pixel jitter of the camera each frame (Halton 2, 3).
        // FSR: its own jitter, and the render size its mode asks.
        let fsr_on = self.settings.antialiasing == 3 && self.fsr_ready(&r.device);
        let taa = self.settings.antialiasing == 2 || (self.settings.antialiasing == 3 && !fsr_on);
        let scale = if fsr_on {
            1.0 / [1.0, 1.5, 1.7, 2.0, 3.0][usize::from(self.settings.fsr_mode.min(4))]
        } else {
            self.settings.render_scale.clamp(0.5, 3.0)
        };
        let internal = (
            ((size.0 as f32 * scale).round() as u32).max(1),
            ((size.1 as f32 * scale).round() as u32).max(1),
        );
        let fsr_jitter = self.fsr_jitter(fsr_on, internal, size);
        let unjittered = view_proj;
        let view_proj = if let Some((ox, oy)) = fsr_jitter {
            let mut j = math::identity();
            j[0][3] = 2.0 * ox / internal.0 as f32;
            j[1][3] = -2.0 * oy / internal.1 as f32;
            math::mul(&j, &view_proj)
        } else if taa {
            let halton = |mut i: u32, base: u32| {
                let (mut f, mut r) = (1.0f32, 0.0f32);
                while i > 0 {
                    f /= base as f32;
                    r += f * (i % base) as f32;
                    i /= base;
                }
                r
            };
            let k = self.frame_count % 8 + 1;
            let jx = (halton(k, 2) - 0.5) * 2.0 / internal.0 as f32;
            let jy = (halton(k, 3) - 0.5) * 2.0 / internal.1 as f32;
            let mut j = math::identity();
            j[0][3] = jx;
            j[1][3] = jy;
            math::mul(&j, &view_proj)
        } else {
            view_proj
        };
        self.note(format!("frame: camera at {eye:?}"));
        let device = r.device.clone();
        let queue = r.queue.clone();
        // The enhanced passes run at the render scale; the resolve brings
        // the image to the game's size.
        self.targets(&device, internal);
        // Split the frame: enhanced items, and the rest for the game's
        // shaders (kept in order).
        let mut sent = crate::replay::Sent::new();
        let mut geom: Vec<u8> = Vec::new();
        let mut items = Vec::new();
        let mut waters = Vec::new();
        let mut rest = Vec::new();
        for c in &frame.cmds {
            match c {
                Cmd::UpdateBuffer { id, offset, data } => {
                    crate::replay::note_sent(&mut sent, *id, *offset, data);
                    rest.push(c.clone());
                }
                Cmd::ClearTarget { .. } | Cmd::ClearDepth { .. } => {}
                // Copies run with the game's draws, after the enhanced scene.
                Cmd::CopyTexture { .. } => rest.push(c.clone()),
                Cmd::Draw(d) => {
                    let family = self.family(r, d);
                    if family == Family::Character
                        && let Some(m) = self.read_map_lights(r, d)
                    {
                        self.map_lights = Some(m);
                    }
                    if family == Family::FakeShadow && !self.settings.fake_shadows {
                        continue;
                    }
                    let in_scene = d.rtv.first().map(|x| x.1) == Some(color_view);
                    let item = if in_scene && self.settings.enabled {
                        let mut put = |bytes: &[u8]| {
                            let at = geom.len().next_multiple_of(4);
                            geom.resize(at, 0);
                            geom.extend_from_slice(bytes);
                            at as u64
                        };
                        let sources = |id: Id, offset: u32| -> Option<Source> {
                            let bytes = if id == d.index.0 && offset == d.index.2 {
                                crate::replay::draw_index_bytes(&sent, d)
                            } else {
                                let slot = d.vertex.iter().find(|v| v.1 == id && v.3 == offset)?.0;
                                crate::replay::draw_vertex_bytes(r, &sent, d, slot)
                            };
                            Some(match bytes {
                                Some(b) => Source::Arena(put(b)),
                                None => {
                                    r.buffers.get(&id)?;
                                    Source::Buffer(id, u64::from(offset))
                                }
                            })
                        };
                        if family == Family::Water {
                            if self.settings.water
                                && let Some(w) = self.water_item(r, d, time, sources)
                            {
                                waters.push(w);
                                continue;
                            }
                            None
                        } else {
                            self.item(r, d, family, time, sources)
                        }
                    } else {
                        None
                    };
                    match item {
                        Some(i) => items.push(i),
                        None => rest.push(c.clone()),
                    }
                }
            }
        }
        self.last_items = items.len() + waters.len();
        self.last_other = rest.iter().filter(|c| matches!(c, Cmd::Draw(_))).count();
        // Uniforms: camera, shadow camera, then items.
        let align =
            u64::from(device.limits().min_uniform_buffer_offset_alignment).max(256) as usize;
        let mut arena: Vec<u8> = Vec::new();
        let mut alloc = |bytes: &[u8], min: usize| -> u64 {
            let at = arena.len().next_multiple_of(align);
            arena.resize(at, 0);
            arena.extend_from_slice(bytes);
            arena.resize(at + bytes.len().max(min), 0);
            at as u64
        };
        let inv_vp = math::invert(&view_proj);
        let mut cam = Vec::new();
        put_rows(&mut cam, &view_proj);
        put_rows(&mut cam, &view);
        put_rows(&mut cam, &math::transpose(&inv_vp));
        put4(&mut cam, [eye[0], eye[1], eye[2], 1.0]);
        put4(
            &mut cam,
            [
                time,
                self.settings.normal_strength,
                if self.settings.material_maps {
                    1.0
                } else {
                    0.0
                },
                self.settings.roughness,
            ],
        );
        // After a camera cut nothing moved: the image did not come from the
        // last one.
        let mut camera_cut = false;
        let prev_view_proj = match self.last_view_proj.replace((unjittered, eye)) {
            Some((vp, e)) if (0..3).map(|k| (e[k] - eye[k]).powi(2)).sum::<f32>() < 40.0 * 40.0 => {
                vp
            }
            _ => {
                camera_cut = true;
                unjittered
            }
        };
        put_rows(&mut cam, &unjittered);
        put_rows(&mut cam, &prev_view_proj);
        assert_eq!(cam.len(), CAMERA_SIZE, "camera uniform layout");
        let cam_at = alloc(&cam, CAMERA_SIZE);
        // The key light: the map's, or the settings'.
        let (sun_dir, sun_color, sun_intensity, fill, sky) =
            match (self.settings.map_lights, self.map_lights) {
                (true, Some(m)) => (
                    math::normalize(m.key.0),
                    m.key.1,
                    self.settings.key_scale,
                    (
                        math::normalize(m.fill.0),
                        m.fill.1,
                        self.settings.fill_scale,
                    ),
                    (m.ambient, self.settings.sky_scale),
                ),
                _ => (
                    math::normalize(self.settings.sun_direction),
                    self.settings.sun_color,
                    self.settings.sun_intensity,
                    ([0.0, -1.0, 0.0], [0.0; 3], 0.0),
                    (self.settings.sky_color, self.settings.sky_intensity),
                ),
            };
        // The editor's sun.
        let sun_dir = self.sun.direction.map_or(sun_dir, math::normalize);
        let sun_color = self.sun.color.unwrap_or(sun_color);
        let sun_intensity = sun_intensity * self.sun.scale.unwrap_or(1.0);
        // Three shadow cascades along the view, each a box around a sphere
        // holding its slice of the view (stable as the camera turns),
        // snapped to whole texels (stable as it moves).
        let forward = math::normalize([view[2][0], view[2][1], view[2][2]]);
        let near_extent = self.settings.shadow_extent.max(1.0);
        let splits = [near_extent, near_extent * 3.5, near_extent * 12.0];
        let shadow_size = self.settings.shadow_size.clamp(256, 8192) as f32;
        // Tangents of the half field of view, from the projection.
        let proj = math::mul(&view_proj, &math::invert(&view));
        let (tx, ty) = (
            1.0 / proj[0][0].abs().max(1e-3),
            1.0 / proj[1][1].abs().max(1e-3),
        );
        let rotation = math::look_along([0.0; 3], sun_dir);
        let mut cascades = Vec::new();
        let mut from = 0.0f32;
        for &to in &splits {
            let mid = f32::midpoint(from, to);
            let center = [
                eye[0] + forward[0] * mid,
                eye[1] + forward[1] * mid,
                eye[2] + forward[2] * mid,
            ];
            let corner = |d: f32| {
                let dx = d * tx;
                let dy = d * ty;
                ((d - mid) * (d - mid) + dx * dx + dy * dy).sqrt()
            };
            let radius = corner(from).max(corner(to)).ceil();
            let texel = 2.0 * radius / shadow_size;
            let c = [
                math::dot([rotation[0][0], rotation[0][1], rotation[0][2]], center),
                math::dot([rotation[1][0], rotation[1][1], rotation[1][2]], center),
                math::dot([rotation[2][0], rotation[2][1], rotation[2][2]], center),
            ];
            let snapped = [
                (c[0] / texel).floor() * texel,
                (c[1] / texel).floor() * texel,
            ];
            let depth = radius * 4.0;
            let mut lv = rotation;
            lv[0][3] = -snapped[0];
            lv[1][3] = -snapped[1];
            lv[2][3] = -(c[2] - depth);
            cascades.push(math::mul(
                &math::ortho(radius, radius, 0.0, depth * 2.0),
                &lv,
            ));
            from = to;
        }
        let extent = near_extent;
        let cascade_at: Vec<u64> = cascades
            .iter()
            .map(|vp| {
                let mut c = Vec::new();
                put_rows(&mut c, vp);
                put_rows(&mut c, &math::identity());
                put_rows(&mut c, &math::identity());
                put4(&mut c, [eye[0], eye[1], eye[2], 1.0]);
                put4(&mut c, [time, 0.0, 0.0, 0.0]);
                alloc(&c, CAMERA_SIZE)
            })
            .collect();
        let shadow_cam = Vec::<u8>::new();
        let _ = shadow_cam;
        // Each draw's transforms last frame, for its motion vectors: draws
        // are matched by what they draw (the same mesh drawn twice in a
        // frame by its turn).
        let mut motion = HashMap::with_capacity(items.len());
        let mut item_at = Vec::with_capacity(items.len());
        for i in &items {
            let mut key = i.motion;
            while motion.contains_key(&key) {
                key = key.wrapping_mul(0x0100_0000_01b3).wrapping_add(1);
            }
            let world = i.uniform[..64].to_vec();
            let (prev_world, prev_bones) = match self.motion.get(&key) {
                Some((w, b)) => (w.as_slice(), b.as_ref()),
                None => (world.as_slice(), None),
            };
            let mut u = i.uniform.clone();
            // The id for picking: the item's place in the frame, + 1; the
            // editor's pick is highlighted.
            let lit = self.selected.is_some_and(|s| s == i.texture && s != 0);
            let id = (item_at.len() + 1 + if lit { 1 << 23 } else { 0 }) as f32;
            u[ITEM_BASE - 4..ITEM_BASE].copy_from_slice(&id.to_le_bytes());
            u.extend_from_slice(prev_world);
            let at = alloc(&u, ITEM_SIZE);
            let b = i.bones.as_ref().map(|b| alloc(b, 5120));
            let pb = prev_bones.map(|b| alloc(b, 5120));
            item_at.push((at, b, pb));
            motion.insert(key, (world, i.bones.clone()));
        }
        self.motion = motion;
        let zero_bones = alloc(&[0u8; 16], 5120);
        let s = &self.settings;
        let mut light = Vec::new();
        put_rows(&mut light, &math::transpose(&inv_vp));
        for c in &cascades {
            put_rows(&mut light, c);
        }
        put4(&mut light, [eye[0], eye[1], eye[2], 1.0]);
        put4(
            &mut light,
            [sun_dir[0], sun_dir[1], sun_dir[2], sun_intensity],
        );
        put4(
            &mut light,
            [
                sun_color[0],
                sun_color[1],
                sun_color[2],
                (1.0 - s.baked_light.clamp(0.0, 1.0)) * s.scenery_sun.max(0.0),
            ],
        );
        put4(&mut light, [sky.0[0], sky.0[1], sky.0[2], sky.1]);
        put4(
            &mut light,
            [
                s.ground_color[0],
                s.ground_color[1],
                s.ground_color[2],
                s.scenery_sky,
            ],
        );
        let fog_lin = fog_color.map(|c| (c * 2.0).max(0.0).powf(2.2));
        put4(
            &mut light,
            [fog_lin[0], fog_lin[1], fog_lin[2], fog_params[3]],
        );
        put4(
            &mut light,
            [
                fog_params[0],
                fog_params[1],
                fog_params[2],
                if s.fog { 1.0 } else { 0.0 },
            ],
        );
        put4(
            &mut light,
            [
                s.baked_light,
                s.roughness,
                s.specular,
                if s.shadows { 1.0 } else { 0.0 },
            ],
        );
        let shadow_size = s.shadow_size.clamp(256, 8192);
        put4(
            &mut light,
            [
                1.0 / shadow_size as f32,
                s.shadow_softness,
                extent * 0.002,
                if s.ao { 1.0 } else { 0.0 },
            ],
        );
        put4(
            &mut light,
            [
                internal.0 as f32,
                internal.1 as f32,
                s.character_shadows,
                1.0 - s.baked_light.clamp(0.0, 1.0),
            ],
        );
        put4(&mut light, view[2]);
        put4(&mut light, [fill.0[0], fill.0[1], fill.0[2], fill.2]);
        put4(
            &mut light,
            [fill.1[0], fill.1[1], fill.1[2], s.scenery_bounce.max(0.0)],
        );
        put4(
            &mut light,
            [
                s.scenery_key,
                s.scenery_specular,
                s.ao_on_baked,
                s.character_falloff,
            ],
        );
        put4(
            &mut light,
            [splits[0], splits[1], splits[2], f32::from(s.debug_view)],
        );
        put4(
            &mut light,
            [
                s.probe_strength,
                s.probe_mix,
                s.skin,
                if s.probes { 1.0 } else { 0.0 },
            ],
        );
        let traced_on = self.rt && s.ray_traced_shadows && s.shadows;
        put4(
            &mut light,
            [
                if traced_on { 1.0 } else { 0.0 },
                s.sun_size.max(0.0),
                (self.frame_count % 64) as f32,
                s.ray_distance.max(1.0),
            ],
        );
        assert_eq!(light.len(), LIGHT_SIZE, "light uniform layout");
        let light_at = alloc(&light, LIGHT_SIZE);
        // Light probes at the characters nearest the camera (one per group
        // of characters standing together).
        let mut groups: Vec<([f32; 3], f32)> = Vec::new();
        if s.probes {
            for c in items.iter().filter_map(|i| i.centre) {
                match groups.iter_mut().find(|g| {
                    let m = g.0.map(|x| x / g.1);
                    (0..3).map(|k| (m[k] - c[k]).powi(2)).sum::<f32>() < 30.0 * 30.0
                }) {
                    Some(g) => {
                        for (sum, x) in g.0.iter_mut().zip(c) {
                            *sum += x;
                        }
                        g.1 += 1.0;
                    }
                    None => groups.push((c, 1.0)),
                }
            }
        }
        let mut centres: Vec<[f32; 3]> = groups.iter().map(|g| g.0.map(|x| x / g.1)).collect();
        let dist = |c: &[f32; 3]| (0..3).map(|k| (c[k] - eye[k]).powi(2)).sum::<f32>();
        centres.sort_by(|a, b| dist(a).total_cmp(&dist(b)));
        centres.truncate(MAX_PROBES);
        let mut info = Vec::new();
        let mut face_at = Vec::new();
        for slot in 0..MAX_PROBES {
            let faces = centres.get(slot).map_or_else(
                || vec![math::identity(); 6],
                |c| probe_faces(*c, &view, &proj),
            );
            for f in &faces {
                put_rows(&mut info, &math::transpose(&math::invert(f)));
                if slot < centres.len() {
                    let mut c = Vec::new();
                    put_rows(&mut c, f);
                    put_rows(&mut c, &math::identity());
                    put_rows(&mut c, &math::transpose(&math::invert(f)));
                    let e = centres[slot];
                    put4(&mut c, [e[0], e[1], e[2], 1.0]);
                    put4(
                        &mut c,
                        [
                            time,
                            s.normal_strength,
                            if s.material_maps { 1.0 } else { 0.0 },
                            s.roughness,
                        ],
                    );
                    face_at.push(alloc(&c, CAMERA_SIZE));
                }
            }
        }
        for slot in 0..MAX_PROBES {
            let c = centres.get(slot).copied().unwrap_or([0.0; 3]);
            put4(&mut info, [c[0], c[1], c[2], 1.0]);
        }
        put4(
            &mut info,
            [sky.0[0] * sky.1, sky.0[1] * sky.1, sky.0[2] * sky.1, 0.0],
        );
        put4(
            &mut info,
            // The scenery as the art lights it (what it sends back), also
            // where the baked light is turned down for dynamic light.
            [
                centres.len() as f32,
                s.baked_light.max(1.0),
                PROBE_FACE as f32,
                0.0,
            ],
        );
        assert_eq!(info.len(), PROBE_INFO_SIZE, "probe info uniform layout");
        let info_at = alloc(&info, PROBE_INFO_SIZE);
        let mut count = Vec::new();
        put4(&mut count, [centres.len() as f32, 0.0, 0.0, 0.0]);
        queue.write_buffer(&self.probes.buffer, 0, &count);
        let ao_at: Vec<u64> = [[0.0f32, 0.0], [1.0, 0.0], [0.0, 1.0]]
            .iter()
            .map(|dir| {
                let mut a = Vec::new();
                put_rows(&mut a, &math::transpose(&inv_vp));
                put_rows(&mut a, &view_proj);
                put4(
                    &mut a,
                    [
                        s.ao_radius,
                        s.ao_strength,
                        internal.0 as f32,
                        internal.1 as f32,
                    ],
                );
                put4(&mut a, [dir[0], dir[1], 0.0, 0.0]);
                alloc(&a, AO_SIZE)
            })
            .collect();
        let mut post = Vec::new();
        put4(
            &mut post,
            [
                s.exposure,
                f32::from(s.tone_curve),
                s.saturation,
                s.contrast,
            ],
        );
        put_rows(&mut post, &view);
        put4(
            &mut post,
            [
                if s.bloom {
                    s.bloom_strength / post::LEVELS as f32
                } else {
                    0.0
                },
                if s.volumetrics { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
        );
        assert_eq!(post.len(), POST_SIZE, "post uniform layout");
        let post_at = alloc(&post, POST_SIZE);
        self.frame_count = self.frame_count.wrapping_add(1);
        let mut vol = Vec::new();
        put_rows(&mut vol, &math::transpose(&inv_vp));
        for c in &cascades {
            put_rows(&mut vol, c);
        }
        put4(&mut vol, [eye[0], eye[1], eye[2], 1.0]);
        put4(
            &mut vol,
            [
                sun_dir[0],
                sun_dir[1],
                sun_dir[2],
                sun_intensity * s.volumetric_strength,
            ],
        );
        put4(&mut vol, [sun_color[0], sun_color[1], sun_color[2], 0.0]);
        put4(
            &mut vol,
            [
                s.fog_density,
                s.fog_anisotropy.clamp(0.0, 0.95),
                s.volumetric_steps.clamp(4, 128) as f32,
                s.volumetric_distance,
            ],
        );
        put4(
            &mut vol,
            [
                (internal.0 / 2).max(1) as f32,
                (internal.1 / 2).max(1) as f32,
                s.haze,
                (self.frame_count % 64) as f32,
            ],
        );
        put4(
            &mut vol,
            [sky.0[0] * sky.1, sky.0[1] * sky.1, sky.0[2] * sky.1, 0.0],
        );
        put4(&mut vol, view[2]);
        put4(&mut vol, [splits[0], splits[1], splits[2], 0.0]);
        assert_eq!(vol.len(), VOL_SIZE, "volumetric uniform layout");
        let vol_at = alloc(&vol, VOL_SIZE);
        let mut ws = Vec::new();
        for c in &cascades {
            put_rows(&mut ws, c);
        }
        put4(&mut ws, [sun_dir[0], sun_dir[1], sun_dir[2], sun_intensity]);
        put4(&mut ws, [sun_color[0], sun_color[1], sun_color[2], 0.0]);
        put4(&mut ws, [fog_lin[0], fog_lin[1], fog_lin[2], fog_params[3]]);
        put4(
            &mut ws,
            [
                fog_params[0],
                fog_params[1],
                fog_params[2],
                if s.fog { 1.0 } else { 0.0 },
            ],
        );
        put4(&mut ws, view[2]);
        put4(&mut ws, [splits[0], splits[1], splits[2], 0.0]);
        put4(
            &mut ws,
            [
                s.water_reflection,
                s.water_refraction,
                s.water_absorption,
                s.water_environment,
            ],
        );
        put4(
            &mut ws,
            [
                s.water_deep[0],
                s.water_deep[1],
                s.water_deep[2],
                if s.shadows { 1.0 } else { 0.0 },
            ],
        );
        put4(&mut ws, [internal.0 as f32, internal.1 as f32, 0.0, 0.0]);
        assert_eq!(ws.len(), WATER_SCENE_SIZE, "water scene uniform layout");
        let water_scene_at = alloc(&ws, WATER_SCENE_SIZE);
        let water_at: Vec<u64> = waters
            .iter()
            .map(|w| alloc(&w.uniform, WATER_SIZE))
            .collect();
        let mut resolve = Vec::new();
        put4(
            &mut resolve,
            [
                scale,
                if s.antialiasing == 1 { 1.0 } else { 0.0 },
                size.0 as f32,
                size.1 as f32,
            ],
        );
        put_rows(&mut resolve, &view);
        // The scene colour's clear alpha, for the pixels nothing covers.
        let clear_alpha = frame
            .cmds
            .iter()
            .find_map(|c| match c {
                Cmd::ClearTarget { view, color } if *view == color_view => Some(color[3]),
                _ => None,
            })
            .unwrap_or(0.0);
        put4(
            &mut resolve,
            [
                clear_alpha,
                s.sharpen.clamp(0.0, 2.0),
                if self.selected.is_some() { 1.0 } else { 0.0 },
                0.0,
            ],
        );
        assert_eq!(resolve.len(), RESOLVE_SIZE, "resolve uniform layout");
        let resolve_at = alloc(&resolve, RESOLVE_SIZE);
        // TAA: history is good when the image size is the same and the
        // camera did not cut.
        let history_ok = taa
            && self.taa_last.is_some_and(|(_, e, sz)| {
                sz == internal && (0..3).map(|k| (e[k] - eye[k]).powi(2)).sum::<f32>() < 40.0 * 40.0
            });
        let mut ta = Vec::new();
        put_rows(&mut ta, &math::transpose(&math::invert(&unjittered)));
        put_rows(&mut ta, &self.taa_last.map_or(unjittered, |(vp, _, _)| vp));
        put4(
            &mut ta,
            [
                s.taa_blend.clamp(0.02, 1.0),
                if history_ok { 1.0 } else { 0.0 },
                internal.0 as f32,
                internal.1 as f32,
            ],
        );
        put4(&mut ta, [s.taa_clamp.clamp(0.25, 4.0), 0.0, 0.0, 0.0]);
        assert_eq!(ta.len(), TAA_SIZE, "taa uniform layout");
        let taa_at = alloc(&ta, TAA_SIZE);
        let (taa_read, taa_write) = (self.taa_index, 1 - self.taa_index);
        if taa {
            self.taa_index = taa_write;
            self.taa_last = Some((unjittered, eye, internal));
        } else {
            self.taa_last = None;
        }
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("enhanced uniforms"),
            size: (arena.len() as u64).max(256).next_multiple_of(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        arena.resize(arena.len().next_multiple_of(4), 0);
        queue.write_buffer(&ubuf, 0, &arena);
        let gbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("enhanced geometry"),
            size: (geom.len() as u64).max(4).next_multiple_of(4),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_DST
                | if self.rt {
                    wgpu::BufferUsages::BLAS_INPUT
                } else {
                    wgpu::BufferUsages::empty()
                },
            mapped_at_creation: false,
        });
        geom.resize(geom.len().next_multiple_of(4), 0);
        if !geom.is_empty() {
            queue.write_buffer(&gbuf, 0, &geom);
        }
        let binding = |at: u64, size: usize| {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &ubuf,
                offset: at,
                size: wgpu::BufferSize::new(size as u64),
            })
        };
        let cam_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: binding(cam_at, CAMERA_SIZE),
            }],
        });
        let shadow_groups: Vec<wgpu::BindGroup> = cascade_at
            .iter()
            .map(|&at| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.camera_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(at, CAMERA_SIZE),
                    }],
                })
            })
            .collect();
        // Per-item bind groups and pipelines.
        let mut prepared = Vec::new();
        for (i, (u, b, pb)) in items.iter().zip(&item_at) {
            let Some(pipe) = self.pipeline(&device, &i.key) else {
                continue;
            };
            let shadow_key = GKey {
                variant: Variant {
                    shadow: true,
                    ..i.key.variant
                },
                ..i.key.clone()
            };
            let shadow_pipe = self.pipeline(&device, &shadow_key);
            let material = i
                .albedo
                .and_then(|(v, _)| r.views.get(&v).map(|vv| vv.resource))
                .and_then(|res| self.material(r, res))
                .unwrap_or_else(|| self.flat_material.clone());
            let (view, samp) = match i.albedo {
                Some((v, s)) => {
                    let view = r
                        .views
                        .get(&v)
                        .and_then(|vv| r.textures.get(&vv.resource))
                        .map(|t| {
                            t.texture.create_view(&wgpu::TextureViewDescriptor {
                                format: Some(t.format.add_srgb_suffix()),
                                ..Default::default()
                            })
                        });
                    (view, r.sampler(s, false))
                }
                None => (None, r.sampler(0, false)),
            };
            let view = view.unwrap_or_else(|| self.white.clone());
            // The editor's image in place of the texture.
            let image = self
                .material_edits
                .get(&i.texture)
                .and_then(|m| m.texture.clone());
            let view = image
                .and_then(|f| self.replacement(&device, &queue, &f))
                .unwrap_or(view);
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.item_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(*u, ITEM_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: binding(b.unwrap_or(zero_bones), 5120),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&samp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&material),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: binding(pb.or(*b).unwrap_or(zero_bones), 5120),
                    },
                ],
            });
            prepared.push((pipe, shadow_pipe, group, i));
        }
        let mut water_prepared = Vec::new();
        for (w, at) in waters.iter().zip(&water_at) {
            let Some(pipe) = self.water_pipeline(&device, &w.key) else {
                continue;
            };
            let flat = |r: &mut Renderer, id: Id| -> wgpu::TextureView {
                r.texture_view(id, false)
                    .map_or_else(|| self.white.clone(), |v| (*v).clone())
            };
            let views = [
                flat(r, w.views[0]),
                flat(r, w.views[1]),
                flat(r, w.views[2]),
                flat(r, w.views[3]),
            ];
            let is_cube = r
                .views
                .get(&w.views[4])
                .and_then(|v| v.desc)
                .is_some_and(|d| d[1] == 9);
            let cube = if is_cube {
                r.texture_view(w.views[4], false)
                    .map_or_else(|| self.cube.clone(), |v| (*v).clone())
            } else {
                self.cube.clone()
            };
            let samp = r.sampler(w.sampler, false);
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.water_layouts.0,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(*at, WATER_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&views[2]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&views[3]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&cube),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&samp),
                    },
                ],
            });
            water_prepared.push((pipe, group, w));
        }
        let (shadow_view, shadow_rest, shadow_layers) = self.shadow_map(&device);
        // Ray tracing: the scenery's draws into bottom-level structures
        // (kept while their buffers are), all of them into this frame's top
        // level.
        let mut traced = vec![false; prepared.len()];
        let mut instances: Vec<(BlasKey, Option<usize>, [f32; 12])> = Vec::new();
        // New structures: the structure, its size, vertex source, first
        // vertex, stride, index source and first index.
        let mut fresh: Vec<FreshBlas> = Vec::new();
        if traced_on {
            for (k, (_, _, _, i)) in prepared.iter().enumerate() {
                let Some(found) = blas_input(r, i, gbuf.size()) else {
                    continue;
                };
                let (key, vsrc, first_vertex, stride, isrc, vertex_count) = found;
                let mut transform = [0.0f32; 12];
                for (n, v) in transform.iter_mut().enumerate() {
                    *v = f32::from_le_bytes(i.uniform[n * 4..n * 4 + 4].try_into().expect("4"));
                }
                let current = |s: Source| match s {
                    Source::Buffer(id, _) => r.buffers.get(&id).map(|b| b.buffer.clone()),
                    Source::Arena(_) => None,
                };
                let cached = matches!(vsrc, Source::Buffer(..))
                    && isrc.is_none_or(|(s, _)| matches!(s, Source::Buffer(..)))
                    && self.blas_cache.get(&key).is_some_and(|c| {
                        current(vsrc).as_ref() == Some(&c.vertex)
                            && isrc.and_then(|(s, _)| current(s)) == c.index
                    });
                traced[k] = true;
                if cached {
                    instances.push((key, None, transform));
                    continue;
                }
                let size = wgpu::BlasTriangleGeometrySizeDescriptor {
                    vertex_format: wgpu::VertexFormat::Float32x3,
                    vertex_count,
                    index_format: i.index.map(|(_, f)| f),
                    index_count: i.index.map(|_| i.draw.count),
                    flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
                };
                let blas = device.create_blas(
                    &wgpu::CreateBlasDescriptor {
                        label: Some("scenery"),
                        flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                    },
                    wgpu::BlasGeometrySizeDescriptors::Triangles {
                        descriptors: vec![size.clone()],
                    },
                );
                instances.push((key, Some(fresh.len()), transform));
                fresh.push((blas, size, vsrc, first_vertex, stride, isrc));
            }
        }
        let t = self.targets.as_ref().expect("made above");
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("enhanced"),
        });
        let needed = (instances.len() as u32).max(1);
        if self.rt && self.tlas.as_ref().is_none_or(|t| t.0 < needed) {
            let capacity = needed.next_power_of_two().max(64);
            self.tlas = Some((
                capacity,
                device.create_tlas(&wgpu::CreateTlasDescriptor {
                    label: Some("scene"),
                    max_instances: capacity,
                    flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                    update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                }),
            ));
        }
        let mut tlas_slot = self.tlas.take();
        let tlas = tlas_slot.as_mut().map(|(capacity, tlas)| {
            let buffer_of = |s: Source| match s {
                Source::Buffer(id, _) => r.buffers.get(&id).map(|b| &b.buffer),
                Source::Arena(_) => Some(&gbuf),
            };
            let entries: Vec<wgpu::BlasBuildEntry> = fresh
                .iter()
                .filter_map(|(blas, size, vsrc, first_vertex, stride, isrc)| {
                    Some(wgpu::BlasBuildEntry {
                        blas,
                        geometry: wgpu::BlasGeometries::TriangleGeometries(vec![
                            wgpu::BlasTriangleGeometry {
                                size,
                                vertex_buffer: buffer_of(*vsrc)?,
                                first_vertex: *first_vertex,
                                vertex_stride: u64::from(*stride),
                                index_buffer: match isrc {
                                    Some((s, _)) => Some(buffer_of(*s)?),
                                    None => None,
                                },
                                first_index: isrc.map(|(_, f)| f),
                                transform_buffer: None,
                                transform_buffer_offset: None,
                            },
                        ]),
                    })
                })
                .collect();
            if let Some(all) = tlas.get_mut_slice(0..*capacity as usize) {
                for slot in all {
                    *slot = None;
                }
            }
            for (n, (key, made, transform)) in instances.iter().enumerate() {
                let blas = match made {
                    Some(m) => &fresh[*m].0,
                    None => &self.blas_cache[key].blas,
                };
                if let Some(slot) = tlas.get_mut_single(n) {
                    *slot = Some(wgpu::TlasInstance::new(blas, *transform, 0, 0xff));
                }
            }
            enc.build_acceleration_structures(entries.iter(), std::iter::once(&*tlas));
            &*tlas
        });
        // Keep the new static structures for later frames.
        for (key, made, _) in &instances {
            let Some(m) = made else { continue };
            let (blas, _, vsrc, _, _, isrc) = &fresh[*m];
            let get = |s: &Source| match s {
                Source::Buffer(id, _) => r.buffers.get(id).map(|b| b.buffer.clone()),
                Source::Arena(_) => None,
            };
            if let Some(vertex) = get(vsrc)
                && isrc.as_ref().is_none_or(|(s, _)| get(s).is_some())
            {
                self.blas_cache.insert(
                    *key,
                    CachedBlas {
                        vertex,
                        index: isrc.as_ref().and_then(|(s, _)| get(s)),
                        blas: blas.clone(),
                    },
                );
            }
        }
        if self.blas_cache.len() > 4096 {
            self.blas_cache.clear();
        }
        if !fresh.is_empty() {
            let tris: u64 = fresh
                .iter()
                .map(|f| u64::from(f.1.index_count.unwrap_or(f.1.vertex_count)) / 3)
                .sum();
            let verts: u64 = fresh.iter().map(|f| u64::from(f.1.vertex_count)).sum();
            *self
                .notes
                .entry(format!(
                    "rays: {} new structures ({tris} triangles, {verts} vertices), {} instances",
                    fresh.len(),
                    instances.len()
                ))
                .or_default() += 1;
        }
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let record_parts = |rp: &mut wgpu::RenderPass,
                            vertex: &[Source],
                            index: Option<(Source, wgpu::IndexFormat)>,
                            d: &Draw|
         -> bool {
            for (slot, src) in vertex.iter().enumerate() {
                match *src {
                    Source::Arena(at) => rp.set_vertex_buffer(slot as u32, gbuf.slice(at..)),
                    Source::Buffer(id, off) => match r.buffers.get(&id) {
                        Some(b) if off < b.buffer.size() => {
                            rp.set_vertex_buffer(slot as u32, b.buffer.slice(off..));
                        }
                        _ => return false,
                    },
                }
            }
            let instances = d.first_instance..d.first_instance + d.instances.max(1);
            match index {
                Some((src, format)) => {
                    match src {
                        Source::Arena(at) => rp.set_index_buffer(gbuf.slice(at..), format),
                        Source::Buffer(id, off) => match r.buffers.get(&id) {
                            Some(b) => rp.set_index_buffer(b.buffer.slice(off..), format),
                            None => return false,
                        },
                    }
                    rp.draw_indexed(d.start..d.start + d.count, d.base, instances);
                }
                None => rp.draw(d.start..d.start + d.count, instances),
            }
            true
        };
        let record = |rp: &mut wgpu::RenderPass, i: &Item| -> bool {
            record_parts(rp, &i.vertex, i.index, &i.draw)
        };
        // Shadow cascades.
        if self.settings.shadows {
            let sets = if traced_on { 6 } else { 3 };
            for (n, layer) in shadow_layers.iter().enumerate().take(sets) {
                let group = &shadow_groups[n % 3];
                let rest_only = n >= 3;
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shadow"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: layer,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rp.set_bind_group(0, group, &[]);
                for (k, (_, sp, igroup, i)) in prepared.iter().enumerate() {
                    if rest_only && traced[k] {
                        continue;
                    }
                    if let Some(sp) = sp {
                        rp.set_pipeline(sp);
                        rp.set_bind_group(1, igroup, &[]);
                        record(&mut rp, i);
                    }
                }
            }
        }
        // G-buffer.
        {
            let clear = |v| attachment(v, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gbuffer"),
                color_attachments: &[
                    clear(&t.albedo),
                    clear(&t.normal),
                    clear(&t.baked),
                    clear(&t.motion),
                    clear(&t.material),
                    clear(&t.id),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_viewport(0.0, 0.0, internal.0 as f32, internal.1 as f32, 0.0, 1.0);
            rp.set_bind_group(0, &cam_group, &[]);
            for (p, _, group, i) in &prepared {
                rp.set_pipeline(p);
                rp.set_bind_group(1, group, &[]);
                record(&mut rp, i);
            }
        }
        // Light probes: the scenery around each, then its harmonics.
        if !centres.is_empty() {
            for (layer, at) in self.probes.layers.iter().zip(&face_at) {
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.camera_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(*at, CAMERA_SIZE),
                    }],
                });
                let clear = |v| attachment(v, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("probe"),
                    color_attachments: &[
                        clear(&layer[0]),
                        clear(&layer[1]),
                        clear(&layer[2]),
                        clear(&self.probes.motion),
                        clear(&self.probes.material),
                        clear(&self.probes.id),
                    ],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &layer[3],
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rp.set_bind_group(0, &group, &[]);
                for (p, _, igroup, i) in &prepared {
                    if i.centre.is_none() {
                        rp.set_pipeline(p);
                        rp.set_bind_group(1, igroup, &[]);
                        record(&mut rp, i);
                    }
                }
            }
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.probes.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(info_at, PROBE_INFO_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.probes.arrays[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.probes.arrays[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&self.probes.arrays[2]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: self.probes.buffer.as_entire_binding(),
                    },
                ],
            });
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("probe harmonics"),
                timestamp_writes: None,
            });
            cp.set_pipeline(&self.probes.pipeline);
            cp.set_bind_group(0, &group, &[]);
            cp.dispatch_workgroups(centres.len() as u32, 1, 1);
        }
        // Ambient occlusion, then its blur across and down.
        if self.settings.ao {
            let passes: [(
                &wgpu::RenderPipeline,
                u64,
                &wgpu::TextureView,
                &wgpu::TextureView,
            ); 3] = [
                (&self.ao.0, ao_at[0], &self.white, &t.ao[0]),
                (&self.ao.1, ao_at[1], &t.ao[0], &t.ao[1]),
                (&self.ao.1, ao_at[2], &t.ao[1], &t.ao[0]),
            ];
            for (pipeline, at, src, dst) in passes {
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.ao.2,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: binding(at, AO_SIZE),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&t.depth),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&t.normal),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(src),
                        },
                    ],
                });
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ao"),
                    color_attachments: &[attachment(dst, wgpu::LoadOp::Clear(wgpu::Color::WHITE))],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                rp.set_pipeline(pipeline);
                rp.set_bind_group(0, &group, &[]);
                rp.draw(0..3, 0..1);
            }
        }
        // Lighting.
        {
            let glows: Vec<lights::Light> = if self.settings.glow_lights {
                self.glow
                    .lights()
                    .map(|l| {
                        lights::glow_edit(&self.glow_edits, l.position)
                            .map_or_else(|| l.clone(), |e| e.apply(l))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            queue.write_buffer(
                &self.light_buffer,
                0,
                &lights::pack(self.lights.iter().chain(&glows), traced_on),
            );
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.lighting.1,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(light_at, LIGHT_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&t.albedo),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&t.normal),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&t.baked),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&t.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(if traced_on {
                            &shadow_rest
                        } else {
                            &shadow_view
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&self.compare),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(if self.settings.ao {
                            &t.ao[0]
                        } else {
                            &self.white
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: self.probes.buffer.as_entire_binding(),
                    },
                ]
                .into_iter()
                .chain(tlas.as_ref().map(|tl| wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::AccelerationStructure(tl),
                }))
                .chain([
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: self.light_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 11,
                        resource: wgpu::BindingResource::TextureView(&t.material),
                    },
                ])
                .collect::<Vec<_>>(),
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lighting"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.hdr,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.lighting.0);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        // Water, over the lit scene: it reads copies of the image and depth.
        if !water_prepared.is_empty() {
            let extent = wgpu::Extent3d {
                width: internal.0,
                height: internal.1,
                depth_or_array_layers: 1,
            };
            enc.copy_texture_to_texture(
                t.hdr_texture.as_image_copy(),
                t.hdr_copy.0.as_image_copy(),
                extent,
            );
            enc.copy_texture_to_texture(
                t.depth_texture.as_image_copy(),
                t.depth_copy.0.as_image_copy(),
                extent,
            );
            let scene_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.water_layouts.1,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(water_scene_at, WATER_SCENE_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&t.hdr_copy.1),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&t.depth_copy.1),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&shadow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.compare),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                ],
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &[attachment(&t.hdr, wgpu::LoadOp::Load)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_viewport(0.0, 0.0, internal.0 as f32, internal.1 as f32, 0.0, 1.0);
            rp.set_bind_group(0, &cam_group, &[]);
            rp.set_bind_group(2, &scene_group, &[]);
            for (pipe, group, w) in &water_prepared {
                rp.set_pipeline(pipe);
                rp.set_bind_group(1, group, &[]);
                record_parts(&mut rp, &w.vertex, w.index, &w.draw);
            }
        }
        // Volumetric light.
        if self.settings.volumetrics {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.volumetric.1,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(vol_at, VOL_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&t.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&shadow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.compare),
                    },
                ],
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("volumetric"),
                color_attachments: &[attachment(&t.vol, wgpu::LoadOp::Clear(wgpu::Color::BLACK))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.volumetric.0);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        // Depth of field, motion blur, bloom.
        let fx = self.post.run(
            &device,
            &queue,
            &mut enc,
            &self.settings,
            &post::Inputs {
                hdr: &t.hdr,
                depth: &t.depth,
                motion: &t.motion,
                size: internal,
                view_proj: unjittered,
                prev_view_proj,
                frame: self.frame_count,
                look_at: r.world.camera.map(|(_, target)| target),
            },
        );
        let hdr = match self.post.image() {
            Some(v) if fx.moved => v.clone(),
            _ => t.hdr.clone(),
        };
        let bloom = match self.post.bloom() {
            Some(v) if fx.bloom.is_some() => v.clone(),
            _ => self.black.clone(),
        };
        // Tone mapping into the intermediate image.
        {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.tone.1,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(post_at, POST_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&hdr),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&t.normal),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&t.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&bloom),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&t.vol),
                    },
                ],
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tone"),
                color_attachments: &[attachment(&t.ldr, wgpu::LoadOp::Clear(wgpu::Color::BLACK))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.tone.0);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        if taa {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.taa.1,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: binding(taa_at, TAA_SIZE),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&t.ldr),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&t.history[taa_read]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&t.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&t.motion),
                    },
                ],
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("taa"),
                color_attachments: &[attachment(
                    &t.history[taa_write],
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                )],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.taa.0);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        // FSR: the motion everywhere, then its own command list between
        // this frame's two submissions.
        #[cfg(windows)]
        let fsr_view = match (fsr_on, self.fsr.as_mut()) {
            (true, Some(fsr)) => {
                if self.fsr_motion.as_ref().is_none_or(|m| m.0 != internal) {
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("fsr motion"),
                        size: wgpu::Extent3d {
                            width: internal.0,
                            height: internal.1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rg16Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    self.fsr_motion = Some((internal, texture, view));
                }
                let (_, motion_texture, motion_view) = self.fsr_motion.as_ref().expect("made");
                self.post.full_motion(
                    &device,
                    &mut enc,
                    &post::Inputs {
                        hdr: &t.hdr,
                        depth: &t.depth,
                        motion: &t.motion,
                        size: internal,
                        view_proj: unjittered,
                        prev_view_proj,
                        frame: self.frame_count,
                        look_at: None,
                    },
                    motion_view,
                );
                let proj = math::mul(&unjittered, &math::invert(&view));
                let (a, b) = (proj[2][2], proj[2][3]);
                let (near, far) = if proj[3][2] < 0.0 {
                    (b / a, b / (1.0 + a))
                } else {
                    (-b / a, b / (1.0 - a))
                };
                let (near, far) = (
                    near.abs().clamp(0.01, 1000.0),
                    far.abs().max(near.abs() + 1.0),
                );
                let now = std::time::Instant::now();
                let frame_ms = self
                    .last_render
                    .map_or(16.7, |t| (now - t).as_secs_f32() * 1000.0);
                let inputs = fsr::Inputs {
                    color: &t.ldr_texture,
                    depth: &t.depth_texture,
                    motion: motion_texture,
                    render: internal,
                    display: size,
                    jitter: fsr_jitter.unwrap_or((0.0, 0.0)),
                    frame_ms,
                    reset: camera_cut,
                    near,
                    far,
                    fov_y: 2.0 * (1.0 / proj[1][1].abs().max(1e-3)).atan(),
                    sharpness: self.settings.fsr_sharpness,
                };
                fsr.prepare(&device, &mut enc, &inputs);
                queue.submit([std::mem::replace(
                    &mut enc,
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("after fsr"),
                    }),
                )
                .finish()]);
                match fsr.dispatch(&device, &queue, &inputs) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        self.notes
                            .entry(format!("FSR: {e}"))
                            .and_modify(|n| *n += 1)
                            .or_insert(1);
                        None
                    }
                }
            }
            _ => None,
        };
        #[cfg(not(windows))]
        let fsr_view: Option<wgpu::TextureView> = {
            let _ = camera_cut;
            None
        };
        self.last_render = Some(std::time::Instant::now());
        let resolved = match &fsr_view {
            Some(v) => v,
            None if taa => &t.history[taa_write],
            None => &t.ldr,
        };
        // Resolve into the game's scene targets.
        let (Some(cv), Some(nv), Some(dv)) = (
            r.texture_view(color_view, false),
            r.texture_view(normal_view, false),
            r.texture_view(depth_view, false),
        ) else {
            self.note("scene targets missing".into());
            queue.submit([enc.finish()]);
            return Ok(());
        };
        let formats_now: Vec<wgpu::TextureFormat> = [color_view, normal_view]
            .iter()
            .filter_map(|v| r.target_format(*v).map(|f| f.0))
            .collect();
        let depth_format = r
            .target_format(depth_view)
            .map_or(wgpu::TextureFormat::Depth32FloatStencil8, |f| f.0);
        if self
            .output
            .as_ref()
            .is_none_or(|o| o.2 != formats_now || o.3 != depth_format)
        {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("resolve"),
                entries: &layout_entries(
                    &[
                        (0, uniform(RESOLVE_SIZE as u64)),
                        (1, tex(FLOAT)),
                        (
                            2,
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        ),
                        (3, tex(UNFILTERED)),
                        (4, tex(wgpu::TextureSampleType::Depth)),
                        (5, tex(wgpu::TextureSampleType::Uint)),
                    ],
                    wgpu::ShaderStages::FRAGMENT,
                ),
            });
            let targets: Vec<Option<wgpu::ColorTargetState>> = formats_now
                .iter()
                .map(|f| {
                    Some(wgpu::ColorTargetState {
                        format: *f,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                })
                .collect();
            let pipeline = fullscreen(
                &device,
                &layout,
                RESOLVE_WGSL,
                &targets,
                Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
            );
            self.output = Some((pipeline, layout, formats_now.clone(), depth_format));
        }
        let (pipeline, layout, _, _) = self.output.as_ref().expect("made above");
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: binding(resolve_at, RESOLVE_SIZE),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(resolved),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.linear),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&t.normal),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&t.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&t.id),
                },
            ],
        });
        {
            let att = |v| attachment(v, wgpu::LoadOp::Load);
            let has_stencil = depth_format.has_stencil_aspect();
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("resolve"),
                color_attachments: &[att(&cv), att(&nv)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: has_stencil.then_some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
        self.tlas = tlas_slot;
        if let Some(e) = pollster::block_on(scope.pop()) {
            self.note(format!("enhanced frame: {e}"));
        }
        if let Some(at) = self.pick.take()
            && let Some(t) = self.targets.as_ref()
        {
            self.picked = pick_at(&device, &queue, t, internal, at, &items, &inv_vp, r);
        }
        // Everything else with the game's shaders, on top.
        let rest = Frame {
            number: frame.number,
            width: frame.width,
            height: frame.height,
            cmds: rest,
        };
        if self.settings.effect_light && self.settings.enabled {
            self.effect_light(r, &rest, color_view)?;
        }
        r.draw_frame(&rest, &Filter::default())
    }

    /// Whether FSR can run (loaded on first ask; why not is noted once).
    #[cfg_attr(not(windows), allow(clippy::unused_self))] // FSR is Windows-only
    fn fsr_ready(&mut self, device: &wgpu::Device) -> bool {
        #[cfg(windows)]
        {
            if !self.fsr_tried {
                self.fsr_tried = true;
                let dir = self.fsr_dir.clone().unwrap_or_default();
                match fsr::Fsr::new(device, &dir) {
                    Ok(f) => self.fsr = Some(f),
                    Err(e) => self.note(format!("no FSR: {e:#}")),
                }
            }
            self.fsr.is_some()
        }
        #[cfg(not(windows))]
        {
            let _ = device;
            false
        }
    }

    /// FSR's jitter for this frame (render pixels), when it runs.
    #[cfg_attr(not(windows), allow(clippy::unused_self))] // FSR is Windows-only
    fn fsr_jitter(&self, on: bool, render: (u32, u32), display: (u32, u32)) -> Option<(f32, f32)> {
        #[cfg(windows)]
        {
            self.fsr
                .as_ref()
                .filter(|_| on)
                .map(|f| f.jitter(render, display, self.frame_count))
        }
        #[cfg(not(windows))]
        {
            let _ = (on, render, display);
            None
        }
    }

    /// The glowing effects the game drew over the last frame
    /// (`Msg::Effects`): lights are found in them.
    ///
    /// # Errors
    /// When they cannot be drawn.
    pub fn effects(&mut self, r: &mut Renderer, frame: &Frame) -> Result<()> {
        self.glow.scene(&r.world.scene);
        let _ = r.device.poll(wgpu::PollType::Poll);
        let Some((view_proj, _)) = self.last_view_proj else {
            return Ok(());
        };
        if !(self.settings.enabled && self.settings.glow_lights) {
            return Ok(());
        }
        self.glow
            .effects(r, frame, &view_proj, self.settings.glow_strength)
    }

    /// The glowing effects of a captured frame, as the hook picks them
    /// live (`Msg::Effects`): after the scene, into its colour against its
    /// depth, adding light (for replays).
    #[must_use]
    pub fn glowing_effects(r: &Renderer, frame: &Frame) -> Frame {
        let scene = frame.cmds.iter().find_map(|c| match c {
            Cmd::Draw(d) if d.rtv.len() >= 2 && d.dsv != 0 => Some((d.rtv[0].1, d.dsv)),
            _ => None,
        });
        let mut cmds = Vec::new();
        let mut after = false;
        for c in &frame.cmds {
            match c {
                Cmd::UpdateBuffer { .. } if after => cmds.push(c.clone()),
                Cmd::Draw(d) => {
                    if d.rtv.len() >= 2 {
                        continue;
                    }
                    after = true;
                    let glows = formats::blend(r.state(d.blend, StateKind::Blend), 0)
                        .0
                        .is_some_and(|b| {
                            b.color.dst_factor == wgpu::BlendFactor::One
                                || b.color.src_factor == wgpu::BlendFactor::One
                        });
                    let tested = r
                        .state(d.depth, StateKind::Depth)
                        .is_none_or(|b| b.get(..4).is_some_and(|w| w != [0; 4]));
                    if Some((d.rtv.first().map_or(0, |x| x.1), d.dsv)) == scene && glows && tested {
                        cmds.push(c.clone());
                    }
                }
                _ => {}
            }
        }
        Frame {
            number: frame.number,
            width: frame.width,
            height: frame.height,
            cmds,
        }
    }

    /// Whether a draw adds light: blended with the target, adding.
    fn additive(r: &Renderer, d: &Draw) -> bool {
        let (blend, _) = formats::blend(r.state(d.blend, StateKind::Blend), 0);
        blend.is_some_and(|b| {
            b.color.operation == wgpu::BlendOperation::Add
                && b.color.dst_factor == wgpu::BlendFactor::One
        })
    }

    /// Light from the frame's effects onto the scene image, before the
    /// effects themselves are drawn over it.
    #[allow(clippy::too_many_lines)] // targets, the effects' draws, then four passes
    fn effect_light(&mut self, r: &mut Renderer, rest: &Frame, color_view: Id) -> Result<()> {
        let emissive: Vec<&Draw> = rest
            .cmds
            .iter()
            .filter_map(|c| match c {
                Cmd::Draw(d)
                    if d.rtv.first().map(|x| x.1) == Some(color_view) && Self::additive(r, d) =>
                {
                    Some(&**d)
                }
                _ => None,
            })
            .collect();
        if emissive.is_empty() {
            return Ok(());
        }
        let Some((desc, format)) = r
            .views
            .get(&color_view)
            .and_then(|v| r.textures.get(&v.resource))
            .map(|t| (t.desc, t.format))
        else {
            return Ok(());
        };
        let size = (desc[0], desc[1]);
        let device = r.device.clone();
        let queue = r.queue.clone();
        if self
            .effect
            .as_ref()
            .is_none_or(|e| e.size != size || e.format != format)
        {
            self.effect_generation += 1;
            let ids = (
                EFFECT_IDS + self.effect_generation * 2,
                EFFECT_IDS + self.effect_generation * 2 + 1,
            );
            if let Some(old) = &self.effect {
                r.apply(Msg::Release { id: old.ids.0 }, &Filter::default())?;
                r.apply(Msg::Release { id: old.ids.1 }, &Filter::default())?;
            }
            let mut d = desc;
            d[2] = 1;
            d[3] = 1;
            d[5] = 1;
            d[6] = 0;
            d[7] = 0;
            d[8] = 0x28;
            d[9] = 0;
            d[10] = 0;
            r.apply(
                Msg::Texture {
                    id: ids.0,
                    desc: d,
                    subresources: Vec::new(),
                },
                &Filter::default(),
            )?;
            r.apply(
                Msg::View {
                    id: ids.1,
                    kind: ffx_gfx::ViewKind::RenderTarget,
                    resource: ids.0,
                    desc: None,
                },
                &Filter::default(),
            )?;
            let copy = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("effect light scene copy"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let copy_view = copy.create_view(&wgpu::TextureViewDescriptor::default());
            let small = || {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("effect light"),
                        size: wgpu::Extent3d {
                            width: (size.0 / 8).max(1),
                            height: (size.1 / 8).max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba16Float,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("effect light"),
                entries: &layout_entries(
                    &[
                        (0, uniform(16)),
                        (1, tex(FLOAT)),
                        (2, tex(FLOAT)),
                        (3, tex(UNFILTERED)),
                        (4, tex(wgpu::TextureSampleType::Depth)),
                        (
                            5,
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        ),
                    ],
                    wgpu::ShaderStages::FRAGMENT,
                ),
            });
            let target = |format| {
                [Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })]
            };
            let hdr = target(wgpu::TextureFormat::Rgba16Float);
            self.effect = Some(EffectLight {
                size,
                format,
                ids,
                copy: (copy, copy_view),
                small: [small(), small()],
                down: fullscreen_entry(&device, &layout, EFFECT_WGSL, "fs_down", &hdr),
                blur: fullscreen_entry(&device, &layout, EFFECT_WGSL, "fs_blur", &hdr),
                apply: fullscreen_entry(&device, &layout, EFFECT_WGSL, "fs_apply", &target(format)),
                layout,
            });
        }
        let e = self.effect.as_ref().expect("made above");
        // The effects alone, onto black, against the scene's depth.
        let mut cmds = vec![Cmd::ClearTarget {
            view: e.ids.1,
            color: [0.0; 4],
        }];
        for c in &rest.cmds {
            match c {
                Cmd::UpdateBuffer { .. } => cmds.push(c.clone()),
                Cmd::Draw(d) if emissive.iter().any(|x| std::ptr::eq(*x, &raw const **d)) => {
                    let mut d = d.clone();
                    d.rtv = vec![(0, e.ids.1)];
                    cmds.push(Cmd::Draw(d));
                }
                _ => {}
            }
        }
        let ids = e.ids;
        r.draw_frame(
            &Frame {
                number: rest.number,
                width: rest.width,
                height: rest.height,
                cmds,
            },
            &Filter::default(),
        )?;
        let (Some(scene), Some(fx_view)) = (
            r.views
                .get(&color_view)
                .and_then(|v| r.textures.get(&v.resource))
                .map(|t| t.texture.clone()),
            r.textures.get(&ids.0).map(|t| {
                t.texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
            }),
        ) else {
            return Ok(());
        };
        let Some(target) = r.texture_view(color_view, false) else {
            return Ok(());
        };
        let e = self.effect.as_ref().expect("made above");
        let t = self.targets.as_ref().expect("made by render");
        let strength = self.settings.effect_light_strength;
        let params: [[f32; 4]; 6] = [
            [strength, 0.0, 0.0, 0.0],
            [strength, 1.5, 0.0, 0.0],
            [strength, 0.0, 1.5, 0.0],
            [strength, 4.0, 0.0, 0.0],
            [strength, 0.0, 4.0, 0.0],
            [strength, 0.0, 0.0, 0.0],
        ];
        let mut bytes = Vec::new();
        for p in &params {
            put4(&mut bytes, *p);
            bytes.resize(bytes.len().next_multiple_of(256), 0);
        }
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effect light"),
            size: bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ubuf, 0, &bytes);
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("effect light"),
        });
        enc.copy_texture_to_texture(
            scene.as_image_copy(),
            e.copy.0.as_image_copy(),
            wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
        );
        let passes: [(
            &wgpu::RenderPipeline,
            &wgpu::TextureView,
            &wgpu::TextureView,
            &wgpu::TextureView,
        ); 6] = [
            (&e.down, &fx_view, &self.black, &e.small[0]),
            (&e.blur, &e.small[0], &self.black, &e.small[1]),
            (&e.blur, &e.small[1], &self.black, &e.small[0]),
            (&e.blur, &e.small[0], &self.black, &e.small[1]),
            (&e.blur, &e.small[1], &self.black, &e.small[0]),
            (&e.apply, &e.copy.1, &e.small[0], &target),
        ];
        for (k, (pipeline, a, b, out)) in passes.into_iter().enumerate() {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &e.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &ubuf,
                            offset: k as u64 * 256,
                            size: wgpu::BufferSize::new(16),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&t.albedo),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&t.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                ],
            });
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("effect light"),
                color_attachments: &[attachment(out, wgpu::LoadOp::Clear(wgpu::Color::BLACK))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
        Ok(())
    }
}
