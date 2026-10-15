//! The enhanced renderer: draws the game's scenery and characters itself
//! (its own shaders, with the game's geometry, textures, transforms and
//! materials, found in each draw's constant buffers by the names the
//! shader reflection gives), lights them in linear HDR with a key light,
//! soft shadows and sky light on top of the game's baked vertex light,
//! and tone-maps into the game's scene target. Draws it does not handle
//! (water, painted backdrops, transparent effects, unknown shaders) go
//! through the game's own shaders afterwards, as in the original path.

pub mod math;
mod settings;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use ffx_gfx::{Cmd, Draw, Frame, Id, StateKind};

use crate::families::{Families, Family};
use crate::{Filter, Renderer, formats};
pub use settings::Settings;

const GBUFFER_WGSL: &str = include_str!("gbuffer.wgsl");
const LIGHTING_WGSL: &str = include_str!("lighting.wgsl");
const OUTPUT_WGSL: &str = include_str!("output.wgsl");

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

    fn floats<'a>(&self, cb: &'a [u8], name: &str, n: usize) -> Option<Vec<f32>> {
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
struct Variant {
    skinned: bool,
    cloth: bool,
    color: bool,
    uv: bool,
    alpha_test: bool,
    shadow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GKey {
    variant: Variant,
    /// Per slot: stride, step per instance, attributes (location, format,
    /// offset).
    buffers: Vec<(u32, bool, Vec<(u32, wgpu::VertexFormat, u64)>)>,
    cull: Option<wgpu::Face>,
    front: wgpu::FrontFace,
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
    depth: wgpu::TextureView,
    hdr: wgpu::TextureView,
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
    output: Option<(
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
        Vec<wgpu::TextureFormat>,
        wgpu::TextureFormat,
    )>,
    targets: Option<Targets>,
    shadow: Option<(u32, wgpu::TextureView)>,
    white: wgpu::TextureView,
    black: wgpu::TextureView,
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

/// Item uniform: world (64), cloth (64), colour, params, ambient (48).
const ITEM_SIZE: usize = 176;
const CAMERA_SIZE: usize = 64 * 3 + 32;
const LIGHT_SIZE: usize = 64 * 2 + 16 * 15;
const POST_SIZE: usize = 16 + 64 + 16;

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
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        families: Families,
        settings: Settings,
    ) -> Self {
        use wgpu::ShaderStages as S;
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
                    (5, tex(wgpu::TextureSampleType::Depth)),
                    (
                        6,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    ),
                    (7, tex(UNFILTERED)),
                ],
                S::FRAGMENT,
            ),
        });
        let lighting = fullscreen(
            device,
            &lighting_layout,
            LIGHTING_WGSL,
            &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba16Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
        );
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
            output: None,
            targets: None,
            shadow: None,
            white: one_pixel(device, queue, [255; 4]),
            black: one_pixel(device, queue, [0, 0, 0, 255]),
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

    fn pipeline(&mut self, device: &wgpu::Device, key: &GKey) -> Option<Arc<wgpu::RenderPipeline>> {
        if let Some(p) = self.pipelines.get(key) {
            return p.clone();
        }
        let v = key.variant;
        let consts = format!(
            "diagnostic(off, derivative_uniformity);\nconst SKINNED: bool = {};\nconst CLOTH: bool = {};\nconst HAS_COLOR: bool = {};\nconst HAS_UV: bool = {};\nconst ALPHA_TEST: bool = {};\n",
            v.skinned, v.cloth, v.color, v.uv, v.alpha_test
        );
        // The vertex input struct: the attributes the draw has.
        let present: Vec<u32> = key
            .buffers
            .iter()
            .flat_map(|b| b.2.iter().map(|a| a.0))
            .collect();
        let decl = [
            ("pos", "vec3<f32>", "vec3<f32>(0.0)"),
            ("nrm", "vec3<f32>", "vec3<f32>(0.0, 1.0, 0.0)"),
            ("col", "vec4<f32>", "vec4<f32>(0.5)"),
            ("uv", "vec2<f32>", "vec2<f32>(0.0)"),
            ("bi", "vec4<u32>", "vec4<u32>(0u)"),
            ("bw", "vec4<f32>", "vec4<f32>(1.0, 0.0, 0.0, 0.0)"),
            ("cloth", "vec4<f32>", "vec4<f32>(0.0)"),
        ];
        let mut vsin = String::from("struct VsIn {\n    @builtin(vertex_index) vid: u32,\n");
        let mut access = String::new();
        for (loc, (name, ty, default)) in decl.iter().enumerate() {
            if present.contains(&(loc as u32)) {
                vsin.push_str(&format!("    @location({loc}) {name}: {ty},\n"));
                access.push_str(&format!(
                    "fn a_{name}(v: VsIn) -> {ty} {{ return v.{name}; }}\n"
                ));
            } else {
                access.push_str(&format!(
                    "fn a_{name}(v: VsIn) -> {ty} {{ return {default}; }}\n"
                ));
            }
        }
        vsin.push_str("};\n");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gbuffer"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{consts}{vsin}{access}{GBUFFER_WGSL}").into(),
            ),
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
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        self.targets = Some(Targets {
            size,
            albedo: make(wgpu::TextureFormat::Rgba8UnormSrgb, "albedo"),
            normal: make(wgpu::TextureFormat::Rgba16Float, "normal"),
            baked: make(wgpu::TextureFormat::Rgba16Float, "baked"),
            depth: make(wgpu::TextureFormat::Depth32Float, "depth"),
            hdr: make(wgpu::TextureFormat::Rgba16Float, "hdr"),
        });
    }

    fn shadow_map(&mut self, device: &wgpu::Device) -> wgpu::TextureView {
        let size = self.settings.shadow_size.clamp(256, 8192);
        if let Some((s, v)) = &self.shadow
            && *s == size
        {
            return v.clone();
        }
        let v = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow"),
                size: wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.shadow = Some((size, v.clone()));
        v
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
        let mut per_slot: Vec<(u32, bool, Vec<(u32, wgpu::VertexFormat, u64)>)> = Vec::new();
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
        let mut u = Vec::with_capacity(ITEM_SIZE);
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
        put4(&mut u, [ambient[0], ambient[1], ambient[2], 0.0]);
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
        Some(Item {
            key,
            vertex,
            index,
            draw: Box::new(d.clone()),
            uniform: u,
            bones: bones_data,
            albedo,
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
        self.note(format!("frame: camera at {eye:?}"));
        let device = r.device.clone();
        let queue = r.queue.clone();
        self.targets(&device, size);
        // Split the frame: enhanced items, and the rest for the game's
        // shaders (kept in order).
        let mut sent: HashMap<Id, (u32, Vec<u8>)> = HashMap::new();
        let mut geom: Vec<u8> = Vec::new();
        let mut items = Vec::new();
        let mut rest = Vec::new();
        for c in &frame.cmds {
            match c {
                Cmd::UpdateBuffer { id, offset, data } => {
                    sent.insert(*id, (*offset, data.clone()));
                    rest.push(c.clone());
                }
                Cmd::ClearTarget { .. } | Cmd::ClearDepth { .. } => {}
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
                        let mut cache: HashMap<(Id, u32), Source> = HashMap::new();
                        let sources = |id: Id, offset: u32| -> Option<Source> {
                            if let Some(s) = cache.get(&(id, offset)) {
                                return Some(*s);
                            }
                            let s = match crate::replay::sent_bytes(&sent, id, offset) {
                                Some(b) => Source::Arena(put(b)),
                                None => {
                                    r.buffers.get(&id)?;
                                    Source::Buffer(id, u64::from(offset))
                                }
                            };
                            cache.insert((id, offset), s);
                            Some(s)
                        };
                        self.item(r, d, family, time, sources)
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
        put4(&mut cam, [time, 0.0, 0.0, 0.0]);
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
        // Its shadow box around the view.
        let forward = math::normalize([view[2][0], view[2][1], view[2][2]]);
        let extent = self.settings.shadow_extent;
        let center = [
            eye[0] + forward[0] * extent * 0.6,
            eye[1] + forward[1] * extent * 0.6,
            eye[2] + forward[2] * extent * 0.6,
        ];
        let back = extent * 4.0;
        let light_eye = [
            center[0] - sun_dir[0] * back,
            center[1] - sun_dir[1] * back,
            center[2] - sun_dir[2] * back,
        ];
        let light_view = math::look_along(light_eye, sun_dir);
        let light_vp = math::mul(&math::ortho(extent, extent, 0.0, back * 2.0), &light_view);
        let mut shadow_cam = Vec::new();
        put_rows(&mut shadow_cam, &light_vp);
        put_rows(&mut shadow_cam, &light_view);
        put_rows(&mut shadow_cam, &math::identity());
        put4(
            &mut shadow_cam,
            [light_eye[0], light_eye[1], light_eye[2], 1.0],
        );
        put4(&mut shadow_cam, [time, 0.0, 0.0, 0.0]);
        let shadow_cam_at = alloc(&shadow_cam, CAMERA_SIZE);
        let mut item_at = Vec::with_capacity(items.len());
        for i in &items {
            let u = alloc(&i.uniform, ITEM_SIZE);
            let b = i.bones.as_ref().map(|b| alloc(b, 5120));
            item_at.push((u, b));
        }
        let zero_bones = alloc(&[0u8; 16], 5120);
        let s = &self.settings;
        let mut light = Vec::new();
        put_rows(&mut light, &math::transpose(&inv_vp));
        put_rows(&mut light, &light_vp);
        put4(&mut light, [eye[0], eye[1], eye[2], 1.0]);
        put4(
            &mut light,
            [sun_dir[0], sun_dir[1], sun_dir[2], sun_intensity],
        );
        put4(&mut light, [sun_color[0], sun_color[1], sun_color[2], 0.0]);
        put4(&mut light, [sky.0[0], sky.0[1], sky.0[2], sky.1]);
        put4(
            &mut light,
            [s.ground_color[0], s.ground_color[1], s.ground_color[2], 0.0],
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
                0.0,
            ],
        );
        put4(&mut light, [size.0 as f32, size.1 as f32, 0.0, 0.0]);
        put4(&mut light, view[2]);
        put4(&mut light, [fill.0[0], fill.0[1], fill.0[2], fill.2]);
        put4(&mut light, [fill.1[0], fill.1[1], fill.1[2], 0.0]);
        let light_at = alloc(&light, LIGHT_SIZE);
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
        put4(&mut post, [0.0, 0.0, 0.0, 0.0]);
        let post_at = alloc(&post, POST_SIZE);
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
                | wgpu::BufferUsages::COPY_DST,
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
        let shadow_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: binding(shadow_cam_at, CAMERA_SIZE),
            }],
        });
        self.note(format!(
            "{} items, {} other commands",
            items.len(),
            rest.len()
        ));
        // Per-item bind groups and pipelines.
        let mut prepared = Vec::new();
        for (i, (u, b)) in items.iter().zip(&item_at) {
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
                ],
            });
            prepared.push((pipe, shadow_pipe, group, i));
        }
        let shadow_view = self.shadow_map(&device);
        let t = self.targets.as_ref().expect("made above");
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("enhanced"),
        });
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let record = |rp: &mut wgpu::RenderPass, i: &Item| -> bool {
            for (slot, src) in i.vertex.iter().enumerate() {
                match *src {
                    Source::Arena(at) => rp.set_vertex_buffer(slot as u32, gbuf.slice(at..)),
                    Source::Buffer(id, off) => match r.buffers.get(&id) {
                        Some(b) if off < b.buffer.size() => {
                            rp.set_vertex_buffer(slot as u32, b.buffer.slice(off..))
                        }
                        _ => return false,
                    },
                }
            }
            let d = &i.draw;
            let instances = d.first_instance..d.first_instance + d.instances.max(1);
            match i.index {
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
        // Shadow map.
        if self.settings.shadows {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_view,
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
            rp.set_bind_group(0, &shadow_group, &[]);
            for (_, sp, group, i) in &prepared {
                if let Some(sp) = sp {
                    rp.set_pipeline(sp);
                    rp.set_bind_group(1, group, &[]);
                    record(&mut rp, i);
                }
            }
        }
        // G-buffer.
        {
            let clear = |v| attachment(v, wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT));
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gbuffer"),
                color_attachments: &[clear(&t.albedo), clear(&t.normal), clear(&t.baked)],
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
            rp.set_viewport(0.0, 0.0, size.0 as f32, size.1 as f32, 0.0, 1.0);
            rp.set_bind_group(0, &cam_group, &[]);
            for (p, _, group, i) in &prepared {
                rp.set_pipeline(p);
                rp.set_bind_group(1, group, &[]);
                record(&mut rp, i);
            }
        }
        // Lighting.
        {
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
                        resource: wgpu::BindingResource::TextureView(&shadow_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&self.compare),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(&self.white),
                    },
                ],
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
        // Output into the game's scene targets.
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
                label: Some("output"),
                entries: &layout_entries(
                    &[
                        (0, uniform(POST_SIZE as u64)),
                        (1, tex(UNFILTERED)),
                        (2, tex(UNFILTERED)),
                        (3, tex(wgpu::TextureSampleType::Depth)),
                        (4, tex(FLOAT)),
                        (
                            5,
                            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        ),
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
                OUTPUT_WGSL,
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
                    resource: binding(post_at, POST_SIZE),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&t.hdr),
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
                    resource: wgpu::BindingResource::TextureView(&self.black),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&self.linear),
                },
            ],
        });
        {
            let att = |v| attachment(v, wgpu::LoadOp::Load);
            let has_stencil = depth_format.has_stencil_aspect();
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("output"),
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
        if let Some(e) = pollster::block_on(scope.pop()) {
            self.note(format!("enhanced frame: {e}"));
        }
        // Everything else with the game's shaders, on top.
        let rest = Frame {
            number: frame.number,
            width: frame.width,
            height: frame.height,
            cmds: rest,
        };
        r.draw_frame(&rest, &Filter::default())
    }
}
