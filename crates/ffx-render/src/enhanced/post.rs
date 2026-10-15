//! Camera effects on the HDR image before tone mapping: depth of field,
//! motion blur and bloom (`post.wgsl`, `focus.wgsl`).

use super::Settings;
use super::{
    FLOAT, UNFILTERED, attachment, fullscreen_entry, layout_entries, math, put_rows, put4, tex,
    uniform,
};

pub(super) const POST_WGSL: &str = include_str!("post.wgsl");
pub(super) const FOCUS_WGSL: &str = include_str!("focus.wgsl");
/// The uniform: inverse view-projection, last frame's, then dof, blur,
/// bloom and the camera's look-at point.
const SIZE: usize = 64 + 64 + 16 * 4;
/// Depth of field's tiles (pixels a side, as in `post.wgsl`).
const TILE: u32 = 16;
const TILE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
/// Bloom levels: half the image, then each half the one before.
pub(super) const LEVELS: usize = 6;
const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// What one frame's effects read.
pub(super) struct Inputs<'a> {
    pub hdr: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub motion: &'a wgpu::TextureView,
    pub size: (u32, u32),
    /// Unjittered, this frame and last.
    pub view_proj: math::Mat,
    pub prev_view_proj: math::Mat,
    pub frame: u32,
    /// What the camera looks at (world space), when the game says.
    pub look_at: Option<[f32; 3]>,
}

/// Where the image ended up.
pub(super) struct Output {
    /// The effects wrote the image into `Post::image` (else it is still in
    /// the input).
    pub moved: bool,
    /// Bloom to add, already divided by the level count: strength.
    pub bloom: Option<f32>,
}

struct Targets {
    size: (u32, u32),
    /// A second HDR image (the effects go back and forth).
    image: [wgpu::TextureView; 2],
    levels: Vec<wgpu::TextureView>,
    /// Depth of field's near blur per tile.
    tiles: wgpu::TextureView,
}

pub(super) struct Post {
    layout: wgpu::BindGroupLayout,
    focus_layout: wgpu::BindGroupLayout,
    dof: wgpu::RenderPipeline,
    dof_tiles: wgpu::RenderPipeline,
    motion: wgpu::RenderPipeline,
    #[cfg_attr(not(windows), allow(dead_code))]
    full_motion: wgpu::RenderPipeline,
    down_first: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    focus_pipeline: wgpu::ComputePipeline,
    focus: wgpu::Buffer,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    targets: Option<Targets>,
    /// Bound as the tiles where a pass does not read them.
    no_tiles: wgpu::TextureView,
    /// The image the last effect wrote.
    last: usize,
}

fn texture(
    device: &wgpu::Device,
    (w, h): (u32, u32),
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Post {
    #[allow(clippy::too_many_lines)] // layouts, then each pipeline
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::ShaderStages as S;
        let storage = |read_only| wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(16),
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &layout_entries(
                &[
                    (0, uniform(SIZE as u64)),
                    (1, tex(FLOAT)),
                    (2, tex(wgpu::TextureSampleType::Depth)),
                    (3, tex(UNFILTERED)),
                    (
                        4,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                    (5, storage(true)),
                    (6, tex(UNFILTERED)),
                ],
                S::FRAGMENT,
            ),
        });
        let focus_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("focus"),
            entries: &layout_entries(
                &[
                    (0, uniform(SIZE as u64)),
                    (1, tex(wgpu::TextureSampleType::Depth)),
                    (2, storage(false)),
                ],
                S::COMPUTE,
            ),
        });
        let plain = [Some(wgpu::ColorTargetState {
            format: HDR,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let tile_target = [Some(wgpu::ColorTargetState {
            format: TILE_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let add = [Some(wgpu::ColorTargetState {
            format: HDR,
            blend: Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("focus"),
            source: wgpu::ShaderSource::Wgsl(FOCUS_WGSL.into()),
        });
        let focus_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("focus"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&focus_layout)],
                    immediate_size: 0,
                }),
            ),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            dof: fullscreen_entry(device, &layout, POST_WGSL, "fs_dof", &plain),
            dof_tiles: fullscreen_entry(device, &layout, POST_WGSL, "fs_dof_tiles", &tile_target),
            motion: fullscreen_entry(device, &layout, POST_WGSL, "fs_motion", &plain),
            full_motion: fullscreen_entry(
                device,
                &layout,
                POST_WGSL,
                "fs_full_motion",
                &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg16Float,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            ),
            down_first: fullscreen_entry(device, &layout, POST_WGSL, "fs_down_first", &plain),
            down: fullscreen_entry(device, &layout, POST_WGSL, "fs_down", &plain),
            up: fullscreen_entry(device, &layout, POST_WGSL, "fs_up", &add),
            layout,
            focus_layout,
            focus_pipeline,
            focus: buffer("focus", 16, wgpu::BufferUsages::STORAGE),
            uniform: buffer(
                "post effects",
                SIZE as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("post"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            targets: None,
            no_tiles: texture(device, (1, 1), TILE_FORMAT, "no tiles"),
            last: 0,
        }
    }

    fn targets(&mut self, device: &wgpu::Device, size: (u32, u32)) -> &Targets {
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            let make = |at: (u32, u32), label: &str| texture(device, at, HDR, label);
            let mut levels = Vec::with_capacity(LEVELS);
            let mut at = size;
            for _ in 0..LEVELS {
                at = ((at.0 / 2).max(1), (at.1 / 2).max(1));
                levels.push(make(at, "bloom"));
            }
            self.targets = Some(Targets {
                size,
                image: [make(size, "post image"), make(size, "post image")],
                levels,
                tiles: texture(
                    device,
                    (size.0.div_ceil(TILE), size.1.div_ceil(TILE)),
                    TILE_FORMAT,
                    "dof tiles",
                ),
            });
        }
        self.targets.as_ref().expect("made")
    }

    fn group(
        &self,
        device: &wgpu::Device,
        src: &wgpu::TextureView,
        tiles: Option<&wgpu::TextureView>,
        i: &Inputs,
    ) -> wgpu::BindGroup {
        let entries = [
            self.uniform.as_entire_binding(),
            wgpu::BindingResource::TextureView(src),
            wgpu::BindingResource::TextureView(i.depth),
            wgpu::BindingResource::TextureView(i.motion),
            wgpu::BindingResource::Sampler(&self.sampler),
            self.focus.as_entire_binding(),
            wgpu::BindingResource::TextureView(tiles.unwrap_or(&self.no_tiles)),
        ];
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &entries
                .into_iter()
                .enumerate()
                .map(|(k, resource)| wgpu::BindGroupEntry {
                    binding: k as u32,
                    resource,
                })
                .collect::<Vec<_>>(),
        })
    }

    /// Runs the effects the settings ask for, in order: depth of field,
    /// motion blur, bloom.
    #[allow(clippy::cast_precision_loss, clippy::too_many_lines)] // image sizes and frame numbers; one pass after the other
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        s: &Settings,
        i: &Inputs,
    ) -> Output {
        let size = i.size;
        // Radii are set for a 1080-line image.
        let px = size.1 as f32 / 1080.0;
        let mut u = Vec::with_capacity(SIZE);
        put_rows(&mut u, &math::transpose(&math::invert(&i.view_proj)));
        put_rows(&mut u, &i.prev_view_proj);
        put4(
            &mut u,
            [
                s.dof_strength,
                s.dof_radius * px,
                s.dof_speed.clamp(0.01, 1.0),
                s.dof_focus,
            ],
        );
        put4(
            &mut u,
            [
                s.motion_blur_shutter,
                s.motion_blur_length * px,
                if s.motion_blur_camera { 1.0 } else { 0.0 },
                (i.frame % 1024) as f32,
            ],
        );
        put4(
            &mut u,
            [s.bloom_threshold, s.bloom_knee, s.bloom_radius, 0.0],
        );
        put4(&mut u, look_at(s, i));
        debug_assert_eq!(u.len(), SIZE, "post effects uniform layout");
        queue.write_buffer(&self.uniform, 0, &u);
        let t = self.targets(device, size);
        let image = t.image.clone();
        let levels = t.levels.clone();
        let tiles = t.tiles.clone();
        let mut out = Output {
            moved: false,
            bloom: None,
        };
        let pass = |enc: &mut wgpu::CommandEncoder,
                    label: &str,
                    pipeline: &wgpu::RenderPipeline,
                    group: &wgpu::BindGroup,
                    dst: &wgpu::TextureView,
                    load: wgpu::LoadOp<wgpu::Color>| {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[attachment(dst, load)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, group, &[]);
            rp.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        // The image the next effect reads.
        let mut at: Option<usize> = None;
        if s.depth_of_field && s.dof_strength > 0.0 {
            let focus = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.focus_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(i.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.focus.as_entire_binding(),
                    },
                ],
            });
            {
                let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("focus"),
                    timestamp_writes: None,
                });
                cp.set_pipeline(&self.focus_pipeline);
                cp.set_bind_group(0, &focus, &[]);
                cp.dispatch_workgroups(1, 1, 1);
            }
            let group = self.group(device, i.hdr, None, i);
            pass(enc, "dof tiles", &self.dof_tiles, &group, &tiles, clear);
            let group = self.group(device, i.hdr, Some(&tiles), i);
            pass(enc, "depth of field", &self.dof, &group, &image[0], clear);
            at = Some(0);
        }
        if s.motion_blur && s.motion_blur_shutter > 0.0 {
            let src = at.map_or(i.hdr, |k| &image[k]);
            let dst = at.map_or(0, |k| 1 - k);
            let group = self.group(device, src, None, i);
            pass(enc, "motion blur", &self.motion, &group, &image[dst], clear);
            at = Some(dst);
        }
        if s.bloom && s.bloom_strength > 0.0 {
            let src = at.map_or(i.hdr, |k| &image[k]);
            let group = self.group(device, src, None, i);
            pass(enc, "bloom", &self.down_first, &group, &levels[0], clear);
            for k in 1..LEVELS {
                let group = self.group(device, &levels[k - 1], None, i);
                pass(enc, "bloom", &self.down, &group, &levels[k], clear);
            }
            for k in (1..LEVELS).rev() {
                let group = self.group(device, &levels[k], None, i);
                pass(
                    enc,
                    "bloom",
                    &self.up,
                    &group,
                    &levels[k - 1],
                    wgpu::LoadOp::Load,
                );
            }
            out.bloom = Some(s.bloom_strength / LEVELS as f32);
        }
        if let Some(k) = at {
            self.last = k;
            out.moved = true;
        }
        out
    }

    #[cfg_attr(not(windows), allow(dead_code))] // for FSR
    /// The motion everywhere (the camera's where the G-buffer drew
    /// nothing) into `dst` (`Rg16Float`, UV units), for FSR; after `run`
    /// (its uniform).
    pub fn full_motion(
        &self,
        device: &wgpu::Device,
        enc: &mut wgpu::CommandEncoder,
        i: &Inputs,
        dst: &wgpu::TextureView,
    ) {
        let group = self.group(device, i.hdr, None, i);
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("full motion"),
            color_attachments: &[attachment(dst, wgpu::LoadOp::Clear(wgpu::Color::BLACK))],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        rp.set_pipeline(&self.full_motion);
        rp.set_bind_group(0, &group, &[]);
        rp.draw(0..3, 0..1);
    }

    /// The image the effects wrote, when `Output::moved`.
    pub fn image(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.image[self.last])
    }

    /// The bloom to add (half size), when `Output::bloom`.
    pub fn bloom(&self) -> Option<&wgpu::TextureView> {
        self.targets.as_ref().map(|t| &t.levels[0])
    }
}

/// The look-at point for the focus pass: NDC, view depth, 1 when the
/// settings follow it and it is on screen in front of the camera.
fn look_at(s: &Settings, i: &Inputs) -> [f32; 4] {
    let Some(p) = i.look_at.filter(|_| s.dof_follow_camera) else {
        return [0.0; 4];
    };
    let m = &i.view_proj;
    let clip: [f32; 4] =
        std::array::from_fn(|r| m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2] + m[r][3]);
    if clip[3] <= 0.01 {
        return [0.0; 4];
    }
    let (x, y) = (clip[0] / clip[3], clip[1] / clip[3]);
    if x.abs() > 1.0 || y.abs() > 1.0 {
        return [0.0; 4];
    }
    [x, y, clip[3], 1.0]
}
