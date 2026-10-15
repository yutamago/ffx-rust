//! Drawing a frame of the scene stream: pipelines from the translated
//! shaders and the D3D11 states, bind groups from the inline constant
//! buffers and the mirrored views, render passes per set of targets.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use ffx_dxbc::wgsl::{self, TextureKind};
use ffx_dxbc::{Container, sm4::Dim};
use ffx_gfx::{Cmd, Draw, Frame, Id, StateKind, ViewKind};

use crate::{Renderer, formats};

/// Which commands of a frame to draw.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Stop after this many draws.
    pub until: Option<usize>,
    /// Draw only into these render targets (views); empty for all.
    pub targets: Vec<Id>,
}

/// A vertex and pixel shader pair, linked and translated.
struct Linked {
    vs: wgsl::Shader,
    ps: wgsl::Shader,
    vs_module: wgpu::ShaderModule,
    ps_module: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    groups: [wgpu::BindGroupLayout; 2],
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PipelineKey {
    vs: u64,
    ps: u64,
    layout: Id,
    strides: [u32; 8],
    topology: u32,
    strip_index: Option<u32>,
    blend: Id,
    depth: Id,
    raster: Id,
    colors: Vec<Option<wgpu::TextureFormat>>,
    depth_format: Option<wgpu::TextureFormat>,
    samples: u32,
}

#[derive(Default)]
pub struct Cache {
    linked: HashMap<(u64, u64), Option<Arc<Linked>>>,
    pipelines: HashMap<PipelineKey, Option<Arc<wgpu::RenderPipeline>>>,
    samplers: HashMap<(Id, bool), Arc<wgpu::Sampler>>,
    views: HashMap<(Id, bool), Arc<wgpu::TextureView>>,
    dummies: HashMap<(TextureKind, u8), Arc<wgpu::TextureView>>,
    default_sampler: Option<(Arc<wgpu::Sampler>, Arc<wgpu::Sampler>)>,
    /// Problems seen once (pipelines that failed and why).
    pub failures: HashMap<String, usize>,
}

fn dim_code(d: Dim) -> u8 {
    match d {
        Dim::TextureCube => 1,
        Dim::Texture3D => 2,
        _ => 0,
    }
}

fn view_dimension(d: Dim) -> wgpu::TextureViewDimension {
    match d {
        Dim::TextureCube => wgpu::TextureViewDimension::Cube,
        Dim::Texture3D => wgpu::TextureViewDimension::D3,
        Dim::Texture1D => wgpu::TextureViewDimension::D1,
        _ => wgpu::TextureViewDimension::D2,
    }
}

fn group_layout(
    device: &wgpu::Device,
    shader: &wgsl::Shader,
    visibility: wgpu::ShaderStages,
) -> wgpu::BindGroupLayout {
    let mut entries = Vec::new();
    for &(slot, size) in &shader.buffers {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: slot,
            visibility,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(u64::from(size)),
            },
            count: None,
        });
    }
    for t in &shader.textures {
        let sample_type = match t.kind {
            TextureKind::Float => wgpu::TextureSampleType::Float { filterable: true },
            TextureKind::Multisampled => wgpu::TextureSampleType::Float { filterable: false },
            TextureKind::Uint => wgpu::TextureSampleType::Uint,
            TextureKind::Sint => wgpu::TextureSampleType::Sint,
            TextureKind::Depth => wgpu::TextureSampleType::Depth,
        };
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: wgsl::TEXTURE_BINDING + t.slot,
            visibility,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: view_dimension(t.dim),
                multisampled: t.kind == TextureKind::Multisampled,
            },
            count: None,
        });
    }
    for &(slot, comparison) in &shader.samplers {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: wgsl::SAMPLER_BINDING + slot,
            visibility,
            ty: wgpu::BindingType::Sampler(if comparison {
                wgpu::SamplerBindingType::Comparison
            } else {
                wgpu::SamplerBindingType::Filtering
            }),
            count: None,
        });
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    })
}

/// `format` cut down to `room` bytes (0: no limit), keeping its type.
pub(crate) fn shrink_format(format: wgpu::VertexFormat, room: u32) -> wgpu::VertexFormat {
    use wgpu::VertexFormat as V;
    if room == 0 || format.size() <= u64::from(room) {
        return format;
    }
    match (format, room) {
        (V::Float32x4 | V::Float32x3, 8..) => V::Float32x2,
        (V::Float32x4 | V::Float32x3 | V::Float32x2, 4..) => V::Float32,
        (V::Uint32x4 | V::Uint32x3, 8..) => V::Uint32x2,
        (V::Uint32x4 | V::Uint32x3 | V::Uint32x2, 4..) => V::Uint32,
        (V::Float16x4, 4..) => V::Float16x2,
        (V::Unorm16x4, 4..) => V::Unorm16x2,
        _ => format,
    }
}

/// A draw ready to record: its pipeline and bind groups.
struct Prepared {
    pipeline: Arc<wgpu::RenderPipeline>,
    groups: [wgpu::BindGroup; 2],
    draw: Box<Draw>,
    scissor: bool,
    /// Where each vertex slot and the index buffer read from: offsets in
    /// the frame's arena for buffers the frame sent bytes of.
    vertex: Vec<(u8, Source)>,
    index: Option<Source>,
}

/// A buffer binding: one of the mirrored buffers, or the frame's arena.
#[derive(Clone, Copy)]
enum Source {
    Buffer(Id, u64),
    Arena(u64),
}

/// The smallest and largest index of `count` indices from `start`.
fn index_bounds(
    data: &[u8],
    format: u32,
    offset: usize,
    start: u32,
    count: u32,
) -> Option<(u32, u32)> {
    let size = if format == 42 { 4 } else { 2 };
    let from = offset + start as usize * size;
    let bytes = data.get(from..from + count as usize * size)?;
    let mut lo = u32::MAX;
    let mut hi = 0;
    for c in bytes.chunks_exact(size) {
        let i = if size == 4 {
            u32::from_le_bytes([c[0], c[1], c[2], c[3]])
        } else {
            u32::from(u16::from_le_bytes([c[0], c[1]]))
        };
        lo = lo.min(i);
        hi = hi.max(i);
    }
    (lo <= hi).then_some((lo, hi))
}

/// The bytes a draw reads from vertex slot `slot` of a buffer the frame
/// sent bytes of: from the binding's offset to its last vertex.
pub(crate) fn draw_vertex_bytes<'a>(
    r: &'a Renderer,
    sent: &'a HashMap<Id, (u32, Vec<u8>)>,
    d: &Draw,
    slot: u8,
) -> Option<&'a [u8]> {
    let &(_, id, stride, offset) = d.vertex.iter().find(|v| v.0 == slot)?;
    let bytes = sent_bytes(sent, id, offset)?;
    let indexed = matches!(
        d.kind,
        ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced
    );
    let per_instance = r.layouts.get(&d.layout).is_some_and(|l| {
        l.iter()
            .any(|e| e.per_instance && e.slot == u32::from(slot))
    });
    let last = if per_instance {
        Some(u64::from(d.first_instance + d.instances.max(1)))
    } else if indexed {
        let (ib, fmt, ioff) = d.index;
        let bounds = match sent_bytes(sent, ib, ioff) {
            Some(data) => index_bounds(data, fmt, 0, d.start, d.count),
            None => r
                .buffers
                .get(&ib)
                .and_then(|b| index_bounds(&b.data, fmt, ioff as usize, d.start, d.count)),
        };
        bounds.map(|(_, hi)| (i64::from(hi) + i64::from(d.base)).max(0) as u64 + 1)
    } else {
        Some(u64::from(d.start + d.count))
    };
    let end = last
        .map_or(bytes.len(), |n| (n * u64::from(stride.max(1))) as usize)
        .min(bytes.len());
    bytes.get(..end)
}

/// The bytes a draw reads from an index buffer the frame sent bytes of.
pub(crate) fn draw_index_bytes<'a>(
    sent: &'a HashMap<Id, (u32, Vec<u8>)>,
    d: &Draw,
) -> Option<&'a [u8]> {
    let (ib, fmt, ioff) = d.index;
    let bytes = sent_bytes(sent, ib, ioff)?;
    let size = if fmt == 42 { 4 } else { 2 };
    bytes.get(..((d.start + d.count) as usize * size).min(bytes.len()))
}

/// The bytes the frame sent for a buffer, as seen from `offset` on.
pub(crate) fn sent_bytes(sent: &HashMap<Id, (u32, Vec<u8>)>, id: Id, offset: u32) -> Option<&[u8]> {
    let (at, data) = sent.get(&id)?;
    let skip = offset.checked_sub(*at)? as usize;
    data.get(skip..)
}

/// Consecutive commands on the same targets.
enum Pass {
    Clear {
        view: Id,
        color: Option<[f32; 4]>,
        depth: Option<(u32, f32, u8)>,
    },
    Draws {
        colors: Vec<Option<Id>>,
        depth: Option<Id>,
        draws: Vec<Prepared>,
    },
    Copy {
        dst: Id,
        src: Id,
    },
}

impl Renderer {
    fn linked(&mut self, vs: Id, ps: Id) -> Option<Arc<Linked>> {
        let (vsh, psh) = (self.shaders.get(&vs)?, self.shaders.get(&ps)?);
        let key = (vsh.hash, psh.hash);
        if let Some(l) = self.cache.linked.get(&key) {
            return l.clone();
        }
        let (vb, pb) = (vsh.bytecode.clone(), psh.bytecode.clone());
        let result = (|| -> Result<Linked> {
            let vc = Container::parse(&vb)?;
            let pc = Container::parse(&pb)?;
            let varyings = wgsl::link(&vc, &pc)?;
            let vs = wgsl::translate(&vc, &varyings, 0)?;
            let ps = wgsl::translate(&pc, &varyings, 1)?;
            let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let vs_module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("vs"),
                    source: wgpu::ShaderSource::Wgsl(vs.wgsl.clone().into()),
                });
            let ps_module = self
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("ps"),
                    source: wgpu::ShaderSource::Wgsl(ps.wgsl.clone().into()),
                });
            let g0 = group_layout(&self.device, &vs, wgpu::ShaderStages::VERTEX);
            let g1 = group_layout(&self.device, &ps, wgpu::ShaderStages::FRAGMENT);
            let layout = self
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&g0), Some(&g1)],
                    immediate_size: 0,
                });
            if let Some(e) = pollster::block_on(scope.pop()) {
                return Err(anyhow!("{e}"));
            }
            Ok(Linked {
                vs,
                ps,
                vs_module,
                ps_module,
                layout,
                groups: [g0, g1],
            })
        })();
        let linked = match result {
            Ok(l) => Some(Arc::new(l)),
            Err(e) => {
                *self
                    .cache
                    .failures
                    .entry(format!("link {:016x}/{:016x}: {e}", key.0, key.1))
                    .or_default() += 1;
                None
            }
        };
        self.cache.linked.insert(key, linked.clone());
        linked
    }

    pub(crate) fn state(&self, id: Id, kind: StateKind) -> Option<&[u8]> {
        self.states
            .get(&id)
            .filter(|(k, _)| *k == kind)
            .map(|(_, d)| d.as_slice())
    }

    /// Whether the view's texture can be drawn into.
    fn can_target(&self, view: Id) -> bool {
        self.views
            .get(&view)
            .and_then(|v| self.textures.get(&v.resource))
            .is_some_and(|t| {
                t.texture
                    .usage()
                    .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            })
    }

    /// The format and sample count behind a render target or depth view.
    pub(crate) fn target_format(&self, view: Id) -> Option<(wgpu::TextureFormat, u32)> {
        let v = self.views.get(&view)?;
        let t = self.textures.get(&v.resource)?;
        Some((t.format, t.desc[5].max(1)))
    }

    #[allow(clippy::too_many_lines)] // one pipeline description
    fn pipeline(&mut self, d: &Draw, linked: &Arc<Linked>) -> Option<Arc<wgpu::RenderPipeline>> {
        let mut strides = [0u32; 8];
        for &(slot, _, stride, _) in &d.vertex {
            if let Some(s) = strides.get_mut(slot as usize) {
                *s = stride;
            }
        }
        let colors: Vec<Option<wgpu::TextureFormat>> = {
            let n = d.rtv.iter().map(|r| r.0 as usize + 1).max().unwrap_or(0);
            (0..n)
                .map(|i| {
                    d.rtv
                        .iter()
                        .find(|r| r.0 as usize == i)
                        .and_then(|r| self.target_format(r.1))
                        .map(|f| f.0)
                })
                .collect()
        };
        let depth_format = (d.dsv != 0).then(|| self.target_format(d.dsv)).flatten();
        let samples = d
            .rtv
            .first()
            .and_then(|r| self.target_format(r.1))
            .or(depth_format)
            .map_or(1, |f| f.1);
        let topology = formats::topology(d.topology)?;
        let strip = matches!(
            topology,
            wgpu::PrimitiveTopology::TriangleStrip | wgpu::PrimitiveTopology::LineStrip
        ) && matches!(
            d.kind,
            ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced
        );
        let key = PipelineKey {
            vs: self.shaders.get(&d.vs)?.hash,
            ps: self.shaders.get(&d.ps)?.hash,
            layout: d.layout,
            strides,
            topology: d.topology,
            strip_index: strip.then_some(d.index.1),
            blend: d.blend,
            depth: d.depth,
            raster: d.raster,
            colors: colors.clone(),
            depth_format: depth_format.map(|f| f.0),
            samples,
        };
        if let Some(p) = self.cache.pipelines.get(&key) {
            return p.clone();
        }
        let elements = self.layouts.get(&d.layout).cloned().unwrap_or_default();
        // Vertex buffers: attributes the shader reads, per slot.
        let mut per_slot: Vec<(u32, bool, Vec<wgpu::VertexAttribute>)> = Vec::new();
        let mut missing = Vec::new();
        let mut offsets = [0u32; 8];
        let mut placed = Vec::new();
        for e in &elements {
            let slot = e.slot as usize;
            let offset = if e.offset == u32::MAX {
                offsets[slot.min(7)]
            } else {
                e.offset
            };
            if let Some(f) = formats::vertex_format(e.format) {
                offsets[slot.min(7)] = offset + f.size() as u32;
                placed.push((e, offset, f));
            }
        }
        for input in &linked.vs.inputs {
            let Some((e, offset, format)) = placed.iter().find(|(e, _, _)| {
                e.semantic.eq_ignore_ascii_case(&input.semantic) && e.index == input.index
            }) else {
                missing.push(format!("{}{}", input.semantic, input.index));
                continue;
            };
            let entry = match per_slot.iter_mut().position(|p| p.0 == e.slot) {
                Some(i) => &mut per_slot[i],
                None => {
                    per_slot.push((e.slot, e.per_instance, Vec::new()));
                    per_slot.last_mut().expect("just pushed")
                }
            };
            // D3D lets an element run past the stride (reading into the
            // next vertex); wgpu does not: keep the components that fit.
            let stride = strides.get(e.slot as usize).copied().unwrap_or(0);
            let format = shrink_format(*format, stride.saturating_sub(*offset));
            entry.2.push(wgpu::VertexAttribute {
                format,
                offset: u64::from(*offset),
                shader_location: input.location,
            });
        }
        if !missing.is_empty() {
            *self
                .cache
                .failures
                .entry(format!("vertex inputs missing: {}", missing.join(",")))
                .or_default() += 1;
            self.cache.pipelines.insert(key, None);
            return None;
        }
        per_slot.sort_by_key(|p| p.0);
        let max_slot = per_slot.iter().map(|p| p.0 + 1).max().unwrap_or(0);
        let buffers: Vec<Option<wgpu::VertexBufferLayout>> = (0..max_slot)
            .map(|slot| {
                let p = per_slot.iter().find(|p| p.0 == slot);
                Some(wgpu::VertexBufferLayout {
                    array_stride: u64::from(strides.get(slot as usize).copied().unwrap_or(0)),
                    step_mode: if p.is_some_and(|p| p.1) {
                        wgpu::VertexStepMode::Instance
                    } else {
                        wgpu::VertexStepMode::Vertex
                    },
                    attributes: p.map_or(&[][..], |p| &p.2),
                })
            })
            .collect();
        let blend_desc = self.state(d.blend, StateKind::Blend).map(<[u8]>::to_vec);
        let written: Vec<u32> = linked.ps.targets.iter().map(|t| t.0).collect();
        let targets: Vec<Option<wgpu::ColorTargetState>> = colors
            .iter()
            .enumerate()
            .map(|(i, f)| {
                f.map(|format| {
                    let (blend, mask) = formats::blend(blend_desc.as_deref(), i);
                    let blend = blend.filter(|_| {
                        matches!(
                            format.sample_type(None, None),
                            Some(wgpu::TextureSampleType::Float { .. })
                        )
                    });
                    wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: if written.contains(&(i as u32)) {
                            mask
                        } else {
                            wgpu::ColorWrites::empty()
                        },
                    }
                })
            })
            .collect();
        let (cull, front, bias, _) = formats::raster(self.state(d.raster, StateKind::Raster));
        let depth_stencil = depth_format
            .map(|(f, _)| formats::depth_stencil(self.state(d.depth, StateKind::Depth), f, bias));
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&linked.layout),
                vertex: wgpu::VertexState {
                    module: &linked.vs_module,
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology,
                    strip_index_format: strip.then_some(if d.index.1 == 42 {
                        wgpu::IndexFormat::Uint32
                    } else {
                        wgpu::IndexFormat::Uint16
                    }),
                    front_face: front,
                    cull_mode: cull,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil,
                multisample: wgpu::MultisampleState {
                    count: samples,
                    mask: !0,
                    alpha_to_coverage_enabled: samples > 1
                        && formats::alpha_to_coverage(blend_desc.as_deref()),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &linked.ps_module,
                    entry_point: Some("main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &targets,
                }),
                multiview_mask: None,
                cache: None,
            });
        let result = match pollster::block_on(scope.pop()) {
            Some(e) => {
                *self
                    .cache
                    .failures
                    .entry(format!("pipeline: {e}"))
                    .or_default() += 1;
                None
            }
            None => Some(Arc::new(pipeline)),
        };
        self.cache.pipelines.insert(key, result.clone());
        result
    }

    pub(crate) fn texture_view(
        &mut self,
        view: Id,
        depth_aspect: bool,
    ) -> Option<Arc<wgpu::TextureView>> {
        if let Some(v) = self.cache.views.get(&(view, depth_aspect)) {
            return Some(v.clone());
        }
        let v = self.views.get(&view)?;
        let t = self.textures.get(&v.resource)?;
        if v.kind != ViewKind::ShaderResource
            && !t
                .texture
                .usage()
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            *self
                .cache
                .failures
                .entry(format!(
                    "view {view:#x}: texture {:#x} {:?} cannot be a target",
                    v.resource, t.desc
                ))
                .or_default() += 1;
            return None;
        }
        let mips = t.desc[2].max(1);
        let layers = t.desc[3].max(1);
        let desc = v.desc.unwrap_or([t.desc[4], 4, 0, mips, 0, layers]);
        let (base_mip, mip_count, dimension, base_layer, layer_count) = match v.kind {
            ViewKind::ShaderResource => {
                let count = if desc[3] == u32::MAX {
                    mips - desc[2].min(mips - 1)
                } else {
                    desc[3].min(mips - desc[2].min(mips - 1))
                };
                let dim = match desc[1] {
                    5 => wgpu::TextureViewDimension::D2Array,
                    9 => wgpu::TextureViewDimension::Cube,
                    10 => wgpu::TextureViewDimension::CubeArray,
                    _ => wgpu::TextureViewDimension::D2,
                };
                let (bl, lc) = match desc[1] {
                    5 => (desc[4], Some(desc[5].max(1))),
                    9 => (0, Some(6)),
                    _ => (0, Some(1)),
                };
                (desc[2].min(mips - 1), Some(count.max(1)), dim, bl, lc)
            }
            _ => (
                desc[2].min(mips - 1),
                Some(1),
                wgpu::TextureViewDimension::D2,
                0,
                Some(1),
            ),
        };
        let aspect = if t.format.is_depth_stencil_format() {
            if depth_aspect || v.kind == ViewKind::ShaderResource {
                wgpu::TextureAspect::DepthOnly
            } else {
                wgpu::TextureAspect::All
            }
        } else {
            wgpu::TextureAspect::All
        };
        let view_obj = t.texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: None,
            dimension: Some(dimension),
            usage: None,
            aspect,
            base_mip_level: base_mip,
            mip_level_count: mip_count,
            base_array_layer: base_layer,
            array_layer_count: layer_count,
        });
        let arc = Arc::new(view_obj);
        self.cache.views.insert((view, depth_aspect), arc.clone());
        Some(arc)
    }

    fn dummy(&mut self, kind: TextureKind, dim: Dim) -> Arc<wgpu::TextureView> {
        let key = (kind, dim_code(dim));
        if let Some(v) = self.cache.dummies.get(&key) {
            return v.clone();
        }
        let format = match kind {
            TextureKind::Uint => wgpu::TextureFormat::Rgba8Uint,
            TextureKind::Sint => wgpu::TextureFormat::Rgba8Sint,
            TextureKind::Depth => wgpu::TextureFormat::Depth32Float,
            _ => wgpu::TextureFormat::Rgba8Unorm,
        };
        let layers = if dim == Dim::TextureCube { 6 } else { 1 };
        let samples = if kind == TextureKind::Multisampled {
            4
        } else {
            1
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("dummy"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(view_dimension(dim)),
            ..Default::default()
        }));
        self.cache.dummies.insert(key, view.clone());
        view
    }

    pub(crate) fn sampler(&mut self, id: Id, comparison: bool) -> Arc<wgpu::Sampler> {
        if let Some(s) = self.cache.samplers.get(&(id, comparison)) {
            return s.clone();
        }
        let desc = self.state(id, StateKind::Sampler).map(<[u8]>::to_vec);
        let sampler = match desc {
            Some(d) => {
                let mut sd = formats::sampler(&d, true);
                if comparison && sd.compare.is_none() {
                    sd.compare = Some(wgpu::CompareFunction::LessEqual);
                }
                if !comparison {
                    sd.compare = None;
                }
                Arc::new(self.device.create_sampler(&sd))
            }
            None => {
                let pair = self.cache.default_sampler.get_or_insert_with(|| {
                    (
                        Arc::new(self.device.create_sampler(&wgpu::SamplerDescriptor {
                            mag_filter: wgpu::FilterMode::Linear,
                            min_filter: wgpu::FilterMode::Linear,
                            ..Default::default()
                        })),
                        Arc::new(self.device.create_sampler(&wgpu::SamplerDescriptor {
                            compare: Some(wgpu::CompareFunction::LessEqual),
                            ..Default::default()
                        })),
                    )
                });
                if comparison {
                    pair.1.clone()
                } else {
                    pair.0.clone()
                }
            }
        };
        self.cache
            .samplers
            .insert((id, comparison), sampler.clone());
        sampler
    }

    /// Bind group `group` of a draw: constant buffer windows from `arena`.
    fn bind_group(
        &mut self,
        linked: &Linked,
        group: usize,
        d: &Draw,
        arena: &wgpu::Buffer,
        offsets: &HashMap<(usize, u8), (u64, u64)>,
    ) -> wgpu::BindGroup {
        let shader = if group == 0 { &linked.vs } else { &linked.ps };
        let (srvs, samplers) = if group == 0 {
            (&d.vs_srv, &d.vs_samplers)
        } else {
            (&d.ps_srv, &d.ps_samplers)
        };
        let mut views = Vec::new();
        for t in &shader.textures {
            let id = srvs.iter().find(|s| u32::from(s.0) == t.slot).map(|s| s.1);
            let view = id
                .and_then(|id| self.texture_view(id, t.kind == TextureKind::Depth))
                .unwrap_or_else(|| self.dummy(t.kind, t.dim));
            views.push((t.slot, view));
        }
        let mut samps = Vec::new();
        for &(slot, cmp) in &shader.samplers {
            let id = samplers
                .iter()
                .find(|s| u32::from(s.0) == slot)
                .map_or(0, |s| s.1);
            samps.push((slot, self.sampler(id, cmp)));
        }
        let mut entries = Vec::new();
        for &(slot, _) in &shader.buffers {
            let (offset, size) = offsets[&(group, slot as u8)];
            entries.push(wgpu::BindGroupEntry {
                binding: slot,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: arena,
                    offset,
                    size: wgpu::BufferSize::new(size),
                }),
            });
        }
        for (slot, v) in &views {
            entries.push(wgpu::BindGroupEntry {
                binding: wgsl::TEXTURE_BINDING + slot,
                resource: wgpu::BindingResource::TextureView(v),
            });
        }
        for (slot, s) in &samps {
            entries.push(wgpu::BindGroupEntry {
                binding: wgsl::SAMPLER_BINDING + slot,
                resource: wgpu::BindingResource::Sampler(s),
            });
        }
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &linked.groups[group],
            entries: &entries,
        })
    }

    /// Draws a frame.
    ///
    /// # Errors
    /// When the GPU reports an error for the frame.
    #[allow(clippy::too_many_lines)] // batching, uploads, passes
    pub fn draw_frame(&mut self, frame: &Frame, filter: &Filter) -> Result<()> {
        // Dynamic buffer updates apply in order; collect draws per pass.
        let mut passes: Vec<Pass> = Vec::new();
        let mut uniform = Vec::new();
        type Pending = (
            usize,
            Box<Draw>,
            Arc<Linked>,
            Arc<wgpu::RenderPipeline>,
            HashMap<(usize, u8), (u64, u64)>,
            Vec<(u8, Source)>,
            Option<Source>,
        );
        let mut pending: Vec<Pending> = Vec::new();
        let mut draws_seen = 0usize;
        let mut sent: HashMap<Id, (u32, Vec<u8>)> = HashMap::new();
        let mut geom: Vec<u8> = Vec::new();
        let align = u64::from(self.device.limits().min_uniform_buffer_offset_alignment).max(16);
        for cmd in &frame.cmds {
            match cmd {
                Cmd::UpdateBuffer { id, offset, data } => {
                    // Kept for the draws that follow: the queue's writes
                    // would all land before any of this frame's draws.
                    sent.insert(*id, (*offset, data.clone()));
                }
                Cmd::CopyTexture { dst, src } => passes.push(Pass::Copy {
                    dst: *dst,
                    src: *src,
                }),
                Cmd::ClearTarget { view, color } => {
                    if !self.views.contains_key(view) {
                        continue;
                    }
                    passes.push(Pass::Clear {
                        view: *view,
                        color: Some(*color),
                        depth: None,
                    });
                }
                Cmd::ClearDepth {
                    view,
                    flags,
                    depth,
                    stencil,
                } => {
                    if !self.views.contains_key(view) {
                        continue;
                    }
                    passes.push(Pass::Clear {
                        view: *view,
                        color: None,
                        depth: Some((*flags, *depth, *stencil)),
                    });
                }
                Cmd::Draw(d) => {
                    draws_seen += 1;
                    if filter.until.is_some_and(|u| draws_seen > u) {
                        break;
                    }
                    if !filter.targets.is_empty()
                        && !d.rtv.iter().any(|r| filter.targets.contains(&r.1))
                    {
                        continue;
                    }
                    // Every target must be one of ours.
                    if d.rtv.iter().any(|r| !self.can_target(r.1))
                        || (d.dsv != 0 && !self.can_target(d.dsv))
                        || d.rtv.iter().any(|r| self.target_format(r.1).is_none())
                        || (d.dsv != 0 && self.target_format(d.dsv).is_none())
                        || d.rtv.is_empty() && d.dsv == 0
                    {
                        continue;
                    }
                    let Some(linked) = self.linked(d.vs, d.ps) else {
                        continue;
                    };
                    let Some(pipeline) = self.pipeline(d, &linked) else {
                        continue;
                    };
                    // Constant buffer windows into the arena.
                    let mut offsets = HashMap::new();
                    for (group, shader, cbs) in
                        [(0usize, &linked.vs, &d.vs_cb), (1, &linked.ps, &d.ps_cb)]
                    {
                        for &(slot, size) in &shader.buffers {
                            let data = cbs
                                .iter()
                                .find(|c| u32::from(c.0) == slot)
                                .map_or(&[][..], |c| &c.1[..]);
                            let start = (uniform.len() as u64).next_multiple_of(align);
                            uniform.resize(start as usize, 0);
                            let len = u64::from(size).max(data.len() as u64).next_multiple_of(16);
                            uniform.extend_from_slice(data);
                            uniform.resize((start + len) as usize, 0);
                            offsets.insert((group, slot as u8), (start, len));
                        }
                    }
                    let colors: Vec<Option<Id>> = {
                        let n = d.rtv.iter().map(|r| r.0 as usize + 1).max().unwrap_or(0);
                        (0..n)
                            .map(|i| d.rtv.iter().find(|r| r.0 as usize == i).map(|r| r.1))
                            .collect()
                    };
                    let depth = (d.dsv != 0).then_some(d.dsv);
                    let same = matches!(passes.last(), Some(Pass::Draws { colors: c, depth: z, .. }) if *c == colors && *z == depth);
                    if !same {
                        passes.push(Pass::Draws {
                            colors,
                            depth,
                            draws: Vec::new(),
                        });
                    }
                    let mut put = |bytes: &[u8], align: usize| {
                        let at = geom.len().next_multiple_of(align);
                        geom.resize(at, 0);
                        geom.extend_from_slice(bytes);
                        at as u64
                    };
                    let vertex: Vec<(u8, Source)> = d
                        .vertex
                        .iter()
                        .map(|&(slot, id, _, offset)| {
                            let src = match draw_vertex_bytes(self, &sent, d, slot) {
                                Some(bytes) => Source::Arena(put(bytes, 4)),
                                None => Source::Buffer(id, u64::from(offset)),
                            };
                            (slot, src)
                        })
                        .collect();
                    let index = matches!(
                        d.kind,
                        ffx_gfx::DrawKind::Indexed | ffx_gfx::DrawKind::IndexedInstanced
                    )
                    .then(|| match sent_bytes(&sent, d.index.0, d.index.2) {
                        Some(bytes) => Source::Arena(put(bytes, 4)),
                        None => Source::Buffer(d.index.0, u64::from(d.index.2)),
                    });
                    pending.push((
                        passes.len() - 1,
                        d.clone(),
                        linked,
                        pipeline,
                        offsets,
                        vertex,
                        index,
                    ));
                }
            }
        }
        let arena = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("constants"),
            size: (uniform.len() as u64).max(256).next_multiple_of(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !uniform.is_empty() {
            uniform.resize(uniform.len().next_multiple_of(4), 0);
            self.queue.write_buffer(&arena, 0, &uniform);
        }
        // Bind groups, in draw order, into their passes.
        let geometry = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame geometry"),
            size: (geom.len() as u64).max(4).next_multiple_of(4),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !geom.is_empty() {
            geom.resize(geom.len().next_multiple_of(4), 0);
            self.queue.write_buffer(&geometry, 0, &geom);
        }
        for (pass, d, linked, pipeline, offsets, vertex, index) in pending {
            let g0 = self.bind_group(&linked, 0, &d, &arena, &offsets);
            let g1 = self.bind_group(&linked, 1, &d, &arena, &offsets);
            let scissor = formats::raster(self.state(d.raster, StateKind::Raster)).3;
            if let Some(Pass::Draws { draws, .. }) = passes.get_mut(pass) {
                draws.push(Prepared {
                    pipeline,
                    groups: [g0, g1],
                    draw: d,
                    scissor,
                    vertex,
                    index,
                });
            }
        }

        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        for pass in &passes {
            match pass {
                Pass::Copy { dst, src } => {
                    let (Some(d), Some(sr)) = (self.textures.get(dst), self.textures.get(src))
                    else {
                        continue;
                    };
                    let compatible = d.format.remove_srgb_suffix()
                        == sr.format.remove_srgb_suffix()
                        && d.desc[0] == sr.desc[0]
                        && d.desc[1] == sr.desc[1]
                        && d.desc[5] == sr.desc[5];
                    if compatible {
                        enc.copy_texture_to_texture(
                            sr.texture.as_image_copy(),
                            d.texture.as_image_copy(),
                            wgpu::Extent3d {
                                width: d.desc[0],
                                height: d.desc[1],
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                }
                Pass::Clear { view, color, depth } => {
                    let Some(v) = self.texture_view(*view, false) else {
                        continue;
                    };
                    if let Some(c) = color {
                        let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &v,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: f64::from(c[0]),
                                        g: f64::from(c[1]),
                                        b: f64::from(c[2]),
                                        a: f64::from(c[3]),
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                    } else if let Some((flags, z, s)) = depth {
                        let has_stencil = self
                            .target_format(*view)
                            .is_some_and(|f| f.0.has_stencil_aspect());
                        let _ = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: None,
                            color_attachments: &[],
                            depth_stencil_attachment: Some(
                                wgpu::RenderPassDepthStencilAttachment {
                                    view: &v,
                                    depth_ops: Some(wgpu::Operations {
                                        load: if flags & 1 != 0 {
                                            wgpu::LoadOp::Clear(*z)
                                        } else {
                                            wgpu::LoadOp::Load
                                        },
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: has_stencil.then_some(wgpu::Operations {
                                        load: if flags & 2 != 0 {
                                            wgpu::LoadOp::Clear(u32::from(*s))
                                        } else {
                                            wgpu::LoadOp::Load
                                        },
                                        store: wgpu::StoreOp::Store,
                                    }),
                                },
                            ),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                            multiview_mask: None,
                        });
                    }
                }
                Pass::Draws {
                    colors,
                    depth,
                    draws,
                } => {
                    if draws.is_empty() {
                        continue;
                    }
                    let color_views: Vec<Option<Arc<wgpu::TextureView>>> = colors
                        .iter()
                        .map(|c| c.and_then(|c| self.texture_view(c, false)))
                        .collect();
                    let depth_view = depth.and_then(|z| self.texture_view(z, false));
                    let (tw, th) = colors
                        .iter()
                        .flatten()
                        .chain(depth.iter())
                        .find_map(|v| {
                            let view = self.views.get(v)?;
                            let t = self.textures.get(&view.resource)?;
                            let mip = view.desc.map_or(0, |d| d[2]);
                            Some(((t.desc[0] >> mip).max(1), (t.desc[1] >> mip).max(1)))
                        })
                        .unwrap_or((1, 1));
                    let has_stencil = depth
                        .and_then(|z| self.target_format(z))
                        .is_some_and(|f| f.0.has_stencil_aspect());
                    let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = color_views
                        .iter()
                        .map(|v| {
                            v.as_ref().map(|v| wgpu::RenderPassColorAttachment {
                                view: v,
                                depth_slice: None,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })
                        })
                        .collect();
                    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &attachments,
                        depth_stencil_attachment: depth_view.as_ref().map(|v| {
                            wgpu::RenderPassDepthStencilAttachment {
                                view: v,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: has_stencil.then_some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                            }
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    for p in draws {
                        let d = &p.draw;
                        let vp = d.viewport;
                        let x = vp[0].clamp(0.0, tw as f32);
                        let y = vp[1].clamp(0.0, th as f32);
                        let w = vp[2].min(tw as f32 - x).max(1.0);
                        let h = vp[3].min(th as f32 - y).max(1.0);
                        rp.set_viewport(x, y, w, h, vp[4].clamp(0.0, 1.0), vp[5].clamp(0.0, 1.0));
                        if p.scissor {
                            let sx = d.scissor[0].clamp(0, tw as i32) as u32;
                            let sy = d.scissor[1].clamp(0, th as i32) as u32;
                            let ex = d.scissor[2].clamp(0, tw as i32) as u32;
                            let ey = d.scissor[3].clamp(0, th as i32) as u32;
                            rp.set_scissor_rect(
                                sx,
                                sy,
                                ex.saturating_sub(sx),
                                ey.saturating_sub(sy),
                            );
                        } else {
                            rp.set_scissor_rect(0, 0, tw, th);
                        }
                        rp.set_pipeline(&p.pipeline);
                        rp.set_bind_group(0, &p.groups[0], &[]);
                        rp.set_bind_group(1, &p.groups[1], &[]);
                        rp.set_blend_constant(wgpu::Color {
                            r: f64::from(d.blend_factor[0]),
                            g: f64::from(d.blend_factor[1]),
                            b: f64::from(d.blend_factor[2]),
                            a: f64::from(d.blend_factor[3]),
                        });
                        rp.set_stencil_reference(d.stencil_ref);
                        let mut ok = true;
                        for &(slot, src) in &p.vertex {
                            match src {
                                Source::Arena(at) => {
                                    rp.set_vertex_buffer(u32::from(slot), geometry.slice(at..));
                                }
                                Source::Buffer(id, offset) => match self.buffers.get(&id) {
                                    Some(b) if offset < b.buffer.size() => {
                                        rp.set_vertex_buffer(
                                            u32::from(slot),
                                            b.buffer.slice(offset..),
                                        );
                                    }
                                    _ => ok = false,
                                },
                            }
                        }
                        if !ok {
                            continue;
                        }
                        let instances = d.first_instance..d.first_instance + d.instances.max(1);
                        let format = if d.index.1 == 42 {
                            wgpu::IndexFormat::Uint32
                        } else {
                            wgpu::IndexFormat::Uint16
                        };
                        match p.index {
                            Some(Source::Arena(at)) => {
                                rp.set_index_buffer(geometry.slice(at..), format);
                                rp.draw_indexed(d.start..d.start + d.count, d.base, instances);
                            }
                            Some(Source::Buffer(id, offset)) => {
                                let Some(b) = self.buffers.get(&id) else {
                                    continue;
                                };
                                rp.set_index_buffer(b.buffer.slice(offset..), format);
                                rp.draw_indexed(d.start..d.start + d.count, d.base, instances);
                            }
                            None => rp.draw(d.start..d.start + d.count, instances),
                        }
                    }
                }
            }
        }
        self.queue.submit([enc.finish()]);
        if let Some(e) = pollster::block_on(scope.pop()) {
            return Err(anyhow!("frame: {e}"));
        }
        Ok(())
    }

    /// Problems seen while drawing (failed links and pipelines), with
    /// counts.
    #[must_use]
    pub fn failures(&self) -> Vec<(String, usize)> {
        let mut v: Vec<_> = self
            .cache
            .failures
            .iter()
            .map(|(k, n)| (k.clone(), *n))
            .collect();
        v.sort();
        v
    }
}
