//! The in-game settings overlay: an egui panel the renderer draws into a
//! transparent image the hook lays over the game's frame. The game window
//! sends the pointer, wheel and keys while it is shown (F10 shows and
//! hides it); changes apply at once, "Save" writes them into the active
//! profile (`profiles`).

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use ffx_gfx::Input;

use crate::enhanced::Settings;
use crate::profiles::Profiles;

const PAINTER_WGSL: &str = r"
struct Screen { size: vec4<f32> };
@group(0) @binding(0) var<uniform> screen: Screen;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos.x / screen.size.x * 2.0 - 1.0, 1.0 - pos.y / screen.size.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    return out;
}

// Coordinates from 2 across: an opaque image (the scene, whose alpha is
// the game's glow mask), at the coordinates less 2.
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let opaque = in.uv.x >= 1.5;
    let t = textureSample(tex, samp, in.uv - vec2<f32>(select(0.0, 2.0, opaque), 0.0));
    return in.color * select(t, vec4<f32>(t.rgb, 1.0), opaque);
}
";

/// The egui texture of the sequence editor's picture.
const SCENE: egui::TextureId = egui::TextureId::User(2);

/// Frame statistics the overlay shows.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub frame_ms: f32,
    /// The renderer's CPU time for the frame.
    pub draw_ms: f32,
    /// The GPU's, per pass (a few frames old), and the whole; empty where
    /// the GPU cannot tell.
    pub gpu: Vec<(&'static str, f32)>,
    pub gpu_ms: f32,
    pub enhanced_items: usize,
    pub other_draws: usize,
}

struct Painter {
    pipeline: wgpu::RenderPipeline,
    screen_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// egui's textures (and ours, without a texture to write: previews).
    textures: HashMap<egui::TextureId, (Option<wgpu::Texture>, wgpu::BindGroup)>,
    target: Option<((u32, u32), wgpu::Texture, wgpu::TextureView)>,
    /// A part of the image on its way back from the GPU.
    pending: Option<Pending>,
    /// What the last painted image showed (a hash of its meshes) and the
    /// part of the screen it covered.
    last: Option<(u64, (u32, u32, u32, u32))>,
    /// Paint all of it next time (a texture changed, or the game has no
    /// image yet).
    pub dirty: bool,
}

/// A part of the overlay image being read back.
struct Pending {
    buffer: wgpu::Buffer,
    /// x, y, width, height.
    area: (u32, u32, u32, u32),
    pitch: u32,
    ready: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// A part of the overlay image: x, y, width, height and its pixels
/// (premultiplied RGBA8, rows packed).
pub type Update = (u32, u32, u32, u32, Vec<u8>);

impl Painter {
    /// Paint again next time though the meshes are the same (a texture
    /// they show changed).
    fn repaint(&mut self) {
        if let Some(l) = self.last.as_mut() {
            l.0 ^= 1;
        }
    }

    #[allow(clippy::too_many_lines)] // one pipeline description
    fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("overlay"),
            source: wgpu::ShaderSource::Wgsl(PAINTER_WGSL.into()),
        });
        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&screen_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Unorm8x4,
                            offset: 16,
                            shader_location: 2,
                        },
                    ],
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                // egui's colours are premultiplied.
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            screen_layout,
            texture_layout,
            sampler,
            textures: HashMap::new(),
            target: None,
            pending: None,
            last: None,
            dirty: true,
        }
    }

    /// Shows `view` as egui texture `id` (or drops it).
    fn set_view(
        &mut self,
        device: &wgpu::Device,
        id: egui::TextureId,
        view: Option<&wgpu::TextureView>,
    ) {
        self.repaint();
        let Some(view) = view else {
            self.textures.remove(&id);
            return;
        };
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.insert(id, (None, group));
    }

    fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        self.repaint();
        let egui::ImageData::Color(image) = &delta.image;
        let [w, h] = image.size;
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(egui::Color32::to_array)
            .collect();
        let size = wgpu::Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        };
        let origin = match delta.pos {
            Some([x, y]) => wgpu::Origin3d {
                x: x as u32,
                y: y as u32,
                z: 0,
            },
            None => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("egui"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.texture_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                self.textures.insert(id, (Some(texture), group));
                wgpu::Origin3d::ZERO
            }
        };
        if let Some((Some(texture), _)) = self.textures.get(&id) {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin,
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w as u32),
                    rows_per_image: Some(h as u32),
                },
                size,
            );
        }
    }

    /// Paints `primitives` into a transparent `size` image and reads it
    /// back (RGBA8, premultiplied alpha, rows packed).
    #[allow(clippy::too_many_lines)]
    fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        primitives: &[egui::ClippedPrimitive],
        ppp: f32,
    ) -> Result<Option<Update>> {
        use std::hash::{Hash, Hasher};
        // What came back from the GPU since (a frame late: no wait for the
        // frame's other work).
        let _ = device.poll(wgpu::PollType::Poll);
        let mut out = None;
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.ready.load(std::sync::atomic::Ordering::Acquire))
        {
            let p = self.pending.take().expect("checked");
            let (x, y, w, h) = p.area;
            {
                let data = p
                    .buffer
                    .slice(..)
                    .get_mapped_range()
                    .map_err(|e| anyhow!("{e:?}"))?;
                let mut pixels = Vec::with_capacity((w * h * 4) as usize);
                for r in 0..h as usize {
                    let at = r * p.pitch as usize;
                    pixels.extend_from_slice(&data[at..at + 4 * w as usize]);
                }
                out = Some((x, y, w, h, pixels));
            }
            p.buffer.unmap();
        }
        if self.pending.is_some() {
            return Ok(out);
        }
        if self.target.as_ref().is_none_or(|t| t.0 != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("overlay"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.target = Some((size, texture, view));
        }
        let (_, texture, view) = self.target.as_ref().expect("made above");
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut draws = Vec::new();
        for p in primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &p.primitive else {
                continue;
            };
            let base = (vertices.len() / 20) as u32;
            for v in &mesh.vertices {
                vertices.extend_from_slice(&(v.pos.x).to_le_bytes());
                vertices.extend_from_slice(&(v.pos.y).to_le_bytes());
                vertices.extend_from_slice(&v.uv.x.to_le_bytes());
                vertices.extend_from_slice(&v.uv.y.to_le_bytes());
                vertices.extend_from_slice(&v.color.to_array());
            }
            let first = indices.len() as u32;
            indices.extend(mesh.indices.iter().map(|i| i + base));
            draws.push((first..indices.len() as u32, mesh.texture_id, p.clip_rect));
        }
        // Unchanged: nothing to paint or send.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        size.hash(&mut hasher);
        vertices.hash(&mut hasher);
        indices.hash(&mut hasher);
        for (range, tex, clip) in &draws {
            range.hash(&mut hasher);
            tex.hash(&mut hasher);
            for f in [clip.min.x, clip.min.y, clip.max.x, clip.max.y] {
                f.to_bits().hash(&mut hasher);
            }
        }
        let hash = hasher.finish();
        if !self.dirty && self.last.is_some_and(|l| l.0 == hash) {
            return Ok(out);
        }
        // The part to send: what the panels cover now and covered before
        // (to clear it), all of it when the game has nothing yet.
        let pixel = |clip: &egui::Rect| {
            (
                (clip.min.x * ppp).clamp(0.0, size.0 as f32) as u32,
                (clip.min.y * ppp).clamp(0.0, size.1 as f32) as u32,
                (clip.max.x * ppp).ceil().clamp(0.0, size.0 as f32) as u32,
                (clip.max.y * ppp).ceil().clamp(0.0, size.1 as f32) as u32,
            )
        };
        let union = |a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)| {
            if a.2 <= a.0 || a.3 <= a.1 {
                b
            } else if b.2 <= b.0 || b.3 <= b.1 {
                a
            } else {
                (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3))
            }
        };
        let covered = draws
            .iter()
            .filter(|d| !d.0.is_empty())
            .map(|d| pixel(&d.2))
            .fold((0, 0, 0, 0), union);
        let area = if self.dirty {
            (0, 0, size.0, size.1)
        } else {
            union(covered, self.last.map_or((0, 0, 0, 0), |l| l.1))
        };
        self.last = Some((hash, covered));
        self.dirty = false;
        if area.2 <= area.0 || area.3 <= area.1 {
            return Ok(out);
        }
        let area = (area.0, area.1, area.2 - area.0, area.3 - area.1);
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (vertices.len() as u64).max(20).next_multiple_of(4),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vbuf, 0, &vertices);
        let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (indices.len() as u64 * 4).max(4),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ibytes: Vec<u8> = indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        queue.write_buffer(&ibuf, 0, &ibytes);
        let points = (size.0 as f32 / ppp, size.1 as f32 / ppp);
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut u = Vec::new();
        for f in [points.0, points.1, 0.0, 0.0] {
            u.extend_from_slice(&f32::to_le_bytes(f));
        }
        queue.write_buffer(&uniforms, 0, &u);
        let screen = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.screen_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pitch = (area.2 * 4).next_multiple_of(256);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(pitch * area.3),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &screen, &[]);
            rp.set_vertex_buffer(0, vbuf.slice(..));
            rp.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
            for (range, tex, clip) in draws {
                let Some((_, group)) = self.textures.get(&tex) else {
                    continue;
                };
                let x0 = (clip.min.x * ppp).clamp(0.0, size.0 as f32) as u32;
                let y0 = (clip.min.y * ppp).clamp(0.0, size.1 as f32) as u32;
                let x1 = (clip.max.x * ppp).clamp(0.0, size.0 as f32) as u32;
                let y1 = (clip.max.y * ppp).clamp(0.0, size.1 as f32) as u32;
                if x1 <= x0 || y1 <= y0 {
                    continue;
                }
                rp.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
                rp.set_bind_group(1, group, &[]);
                rp.draw_indexed(range, 0, 0..1);
            }
        }
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: area.0,
                    y: area.1,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(area.3),
                },
            },
            wgpu::Extent3d {
                width: area.2,
                height: area.3,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([enc.finish()]);
        let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = ready.clone();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            if r.is_ok() {
                flag.store(true, std::sync::atomic::Ordering::Release);
            }
        });
        self.pending = Some(Pending {
            buffer: readback,
            area,
            pitch,
            ready,
        });
        Ok(out)
    }
}

fn key(vk: u32) -> Option<egui::Key> {
    use egui::Key as K;
    Some(match vk {
        0x08 => K::Backspace,
        0x09 => K::Tab,
        0x0d => K::Enter,
        0x1b => K::Escape,
        0x20 => K::Space,
        0x25 => K::ArrowLeft,
        0x26 => K::ArrowUp,
        0x27 => K::ArrowRight,
        0x28 => K::ArrowDown,
        0x2e => K::Delete,
        0x24 => K::Home,
        0x23 => K::End,
        _ => return None,
    })
}

/// What the profile row is doing.
#[derive(Default)]
enum ProfileEdit {
    #[default]
    None,
    /// Typing a new name for the active profile.
    Rename(String),
    /// Asking before deleting the active profile.
    Delete,
}

/// A request from the panel, carried out after it is drawn.
enum Action {
    Save,
    Revert,
    Defaults,
    Switch(String),
    New,
    Duplicate,
    Rename(String),
    Delete,
    Copy,
    Paste,
}

#[allow(clippy::struct_excessive_bools)] // independent states of the panel
pub struct Overlay {
    ctx: egui::Context,
    painter: Painter,
    pub visible: bool,
    pointer: egui::Pos2,
    events: Vec<egui::Event>,
    start: std::time::Instant,
    pub profiles: Profiles,
    edit: ProfileEdit,
    pub message: String,
    /// The panel has the keyboard (a text field has focus or the pointer is
    /// over it): the game does not get the keys.
    pub wants_keys: bool,
    /// Frame times since the shown figures were last updated.
    times: Vec<f32>,
    times_from: std::time::Instant,
    /// Frames per second, mean and longest frame time (ms) shown.
    shown: (f32, f32, f32),
    /// The frame figures shown, renewed with those (so the image stays the
    /// same in between and need not be sent).
    stats_shown: Option<Stats>,
    pub editor: crate::editor::Editor,
    /// The free camera.
    pub freecam: crate::freecam::FreeCam,
    /// The sequence editor's picture of a recorded frame is registered
    /// (drawn under everything).
    scene_shown: bool,
    /// Whether the picked texture's preview is registered.
    preview: bool,
}

impl Overlay {
    /// The overlay; `file` is `ffx-render.toml`, whose settings are
    /// `settings`.
    #[must_use]
    pub fn new(device: &wgpu::Device, file: &std::path::Path, settings: &Settings) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        Self {
            ctx,
            painter: Painter::new(device),
            visible: false,
            pointer: egui::Pos2::ZERO,
            events: Vec::new(),
            start: std::time::Instant::now(),
            profiles: Profiles::new(file, settings),
            edit: ProfileEdit::None,
            message: String::new(),
            wants_keys: false,
            times: Vec::new(),
            times_from: std::time::Instant::now(),
            shown: (0.0, 0.0, 0.0),
            stats_shown: None,
            editor: crate::editor::Editor::new(
                file.parent().unwrap_or_else(|| std::path::Path::new(".")),
            ),
            freecam: crate::freecam::FreeCam::default(),
            scene_shown: false,
            preview: false,
        }
    }

    /// The picture of a recorded frame for the sequence editor, over the
    /// whole screen (none: the game's image shows).
    pub fn set_scene(&mut self, device: &wgpu::Device, view: Option<&wgpu::TextureView>) {
        self.scene_shown = view.is_some();
        self.painter.set_view(device, SCENE, view);
    }

    /// The preview of the texture the editor picked.
    pub fn set_preview(&mut self, device: &wgpu::Device, view: Option<&wgpu::TextureView>) {
        self.preview = view.is_some();
        self.painter.set_view(device, crate::editor::PREVIEW, view);
    }

    /// Takes the game window's input.
    pub fn feed(&mut self, events: &[Input]) {
        let free = !self.ctx.is_pointer_over_egui();
        for e in events {
            if self.freecam.on {
                self.freecam.feed(e, free);
            }
            match *e {
                Input::Toggle => {
                    self.visible = !self.visible;
                    self.wants_keys = false;
                }
                Input::Move(x, y) => {
                    self.pointer = egui::pos2(x, y);
                    self.events.push(egui::Event::PointerMoved(self.pointer));
                }
                Input::Button { button, pressed } => self.events.push(egui::Event::PointerButton {
                    pos: self.pointer,
                    button: match button {
                        1 => egui::PointerButton::Secondary,
                        2 => egui::PointerButton::Middle,
                        _ => egui::PointerButton::Primary,
                    },
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }),
                Input::Wheel(d) => self.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, d),
                    phase: egui::TouchPhase::Move,
                    modifiers: egui::Modifiers::NONE,
                }),
                // F11: the editor.
                Input::Key { vk: 0x7a, pressed } => {
                    if pressed {
                        self.editor.open = !self.editor.open;
                    }
                }
                Input::Key { vk, pressed } => {
                    if let Some(k) = key(vk) {
                        self.events.push(egui::Event::Key {
                            key: k,
                            physical_key: None,
                            pressed,
                            repeat: false,
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                }
                Input::Char(c) => {
                    if let Some(ch) = char::from_u32(c).filter(|c| !c.is_control()) {
                        self.events.push(egui::Event::Text(ch.to_string()));
                    }
                }
            }
        }
    }

    /// Counts a frame that took `ms` (shown or not, so the figures are
    /// ready when the panel opens).
    pub fn frame_time(&mut self, ms: f32) {
        if ms > 0.0 {
            self.times.push(ms);
        }
        if self.times_from.elapsed().as_secs_f32() >= 0.5 && !self.times.is_empty() {
            let mean = self.times.iter().sum::<f32>() / self.times.len() as f32;
            let worst = self.times.iter().copied().fold(0.0, f32::max);
            self.shown = (1000.0 / mean, mean, worst);
            self.stats_shown = None;
            self.times.clear();
            self.times_from = std::time::Instant::now();
        }
    }

    /// The overlay is hidden: the game drops its image, so all of it goes
    /// when it shows again.
    pub fn hidden(&mut self) {
        self.painter.dirty = true;
        self.painter.pending = None;
        // Hidden, the overlay cannot resume the game: it goes on.
        self.editor.paused = false;
        self.editor.playing = false;
    }

    /// Runs the panel over a `size` screen; returns the part of the image
    /// that changed (a frame late), if any.
    ///
    /// # Errors
    /// When painting fails.
    pub fn frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        settings: &mut Settings,
        stats: Stats,
        io: &mut crate::editor::Io,
    ) -> Result<Option<Update>> {
        let stats = self.stats_shown.get_or_insert(stats).clone();
        let ppp = (size.1 as f32 / 720.0).max(1.0);
        let points = egui::vec2(size.0 as f32 / ppp, size.1 as f32 / ppp);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
            time: Some(self.start.elapsed().as_secs_f64()),
            events: std::mem::take(&mut self.events)
                .into_iter()
                .map(|e| match e {
                    egui::Event::PointerMoved(p) => {
                        egui::Event::PointerMoved(egui::pos2(p.x / ppp, p.y / ppp))
                    }
                    egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers,
                    } => egui::Event::PointerButton {
                        pos: egui::pos2(pos.x / ppp, pos.y / ppp),
                        button,
                        pressed,
                        modifiers,
                    },
                    other => other,
                })
                .collect(),
            ..Default::default()
        };
        self.ctx.set_pixels_per_point(ppp);
        let mut actions = Vec::new();
        let head = Head {
            stats,
            fps: self.shown,
            message: &self.message,
            profiles: &self.profiles.names,
            active: &self.profiles.active,
            max_height: points.y - 24.0,
        };
        let edit = &mut self.edit;
        let saved = &self.profiles.saved;
        let editor = &mut self.editor;
        let free = &mut self.freecam;
        io.preview = self.preview;
        let scene_shown = self.scene_shown;
        let out = self.ctx.run_ui(input, |ui| {
            let ctx = ui.ctx().clone();
            if scene_shown {
                ctx.layer_painter(egui::LayerId::background()).image(
                    SCENE,
                    ctx.content_rect(),
                    egui::Rect::from_min_max(egui::pos2(2.0, 0.0), egui::pos2(3.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            // A click on the scene (not on a panel) is the editor's.
            let over = ctx.is_pointer_over_egui();
            let click = ctx
                .input(|i| {
                    i.pointer
                        .primary_clicked()
                        .then(|| i.pointer.interact_pos())
                        .flatten()
                })
                .filter(|_| !over && editor.picking())
                .map(|p| (p.x / points.x, p.y / points.y));
            editor.ui(ui, io, click);
            panel(
                &ctx,
                &mut Ed { s: settings, saved },
                &head,
                edit,
                &mut actions,
                &mut editor.open,
                free,
            );
        });
        // The free camera takes the keys (W, A, S, D, Q, E) from the game.
        self.wants_keys = self.ctx.egui_wants_keyboard_input()
            || self.ctx.is_pointer_over_egui()
            || self.freecam.on;
        for a in actions {
            self.act(a, settings);
        }
        for (id, deltas) in &out.textures_delta.set {
            for d in deltas {
                self.painter.set_texture(device, queue, *id, d);
            }
        }
        let primitives = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        let image = self
            .painter
            .paint(device, queue, size, &primitives, out.pixels_per_point)?;
        for id in &out.textures_delta.free {
            self.painter.textures.remove(id);
        }
        Ok(image)
    }

    fn act(&mut self, a: Action, s: &mut Settings) {
        // A profile picked by hand: scenes without their own go back to it.
        let picks = matches!(
            a,
            Action::Switch(_)
                | Action::New
                | Action::Duplicate
                | Action::Rename(_)
                | Action::Delete
                | Action::Paste
        );
        let p = &mut self.profiles;
        let result = match a {
            Action::Save => p.save(s).map(|()| format!("saved \"{}\"", p.active)),
            Action::Revert => {
                *s = p.saved.clone();
                Ok("back to the saved profile".into())
            }
            Action::Defaults => {
                *s = Settings::default();
                Ok("factory defaults (not saved)".into())
            }
            Action::Switch(name) => p.switch(&name, s).map(|()| format!("\"{name}\"")),
            Action::New => p.create("New profile", &Settings::default()).map(|n| {
                *s = Settings::default();
                format!("made \"{n}\" (factory defaults)")
            }),
            Action::Duplicate => {
                let want = format!("{} copy", p.active);
                p.create(&want, s).map(|n| format!("made \"{n}\""))
            }
            Action::Rename(name) => p.rename(&name).map(|n| format!("renamed to \"{n}\"")),
            Action::Delete => {
                let old = p.active.clone();
                p.delete(s).map(|()| format!("deleted \"{old}\""))
            }
            Action::Copy => {
                if crate::clipboard::set(&p.share_text()) {
                    Ok(format!("\"{}\" is on the clipboard", p.active))
                } else {
                    Err("no clipboard".into())
                }
            }
            Action::Paste => match crate::clipboard::get() {
                Some(text) => p.import(&text, s).map(|n| format!("imported \"{n}\"")),
                None => Err("nothing on the clipboard".into()),
            },
        };
        if picks && result.is_ok() {
            p.chose();
        }
        self.message = result.unwrap_or_else(|e| format!("failed: {e}"));
    }
}

/// The settings being edited and the profile's saved values (what a
/// setting's reset button goes back to).
struct Ed<'a> {
    s: &'a mut Settings,
    saved: &'a Settings,
}

/// A setting and its saved value, for the row helpers.
macro_rules! f {
    ($e:ident . $f:ident) => {
        (&mut $e.s.$f, &$e.saved.$f)
    };
}

/// A setting's row: the widget, then (when it differs from the profile)
/// a button that resets it.
fn row<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    (v, saved): (&mut T, &T),
    add: impl FnOnce(&mut egui::Ui, &mut T),
) {
    ui.horizontal(|ui| {
        add(ui, v);
        if v != saved
            && ui
                .small_button("↺")
                .on_hover_text("Back to the profile's value")
                .clicked()
        {
            *v = saved.clone();
        }
    });
}

fn check(ui: &mut egui::Ui, v: (&mut bool, &bool), label: &str) {
    row(ui, v, |ui, v| {
        ui.checkbox(v, label);
    });
}

fn color3(ui: &mut egui::Ui, v: (&mut [f32; 3], &[f32; 3]), label: &str) {
    row(ui, v, |ui, c| {
        ui.label(label);
        ui.color_edit_button_rgb(c);
    });
}

fn slider(
    ui: &mut egui::Ui,
    v: (&mut f32, &f32),
    label: &str,
    range: std::ops::RangeInclusive<f32>,
) {
    row(ui, v, |ui, v| {
        ui.add(egui::Slider::new(v, range).text(label));
    });
}

fn log_slider(
    ui: &mut egui::Ui,
    v: (&mut f32, &f32),
    label: &str,
    range: std::ops::RangeInclusive<f32>,
) {
    row(ui, v, |ui, v| {
        ui.add(egui::Slider::new(v, range).logarithmic(true).text(label));
    });
}

fn step_slider(
    ui: &mut egui::Ui,
    v: (&mut u32, &u32),
    label: &str,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) {
    row(ui, v, |ui, v| {
        let mut x = *v as f32;
        ui.add(
            egui::Slider::new(&mut x, range)
                .step_by(f64::from(step))
                .text(label),
        );
        *v = x as u32;
    });
}

fn choice<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    v: (&mut T, &T),
    label: &str,
    options: &[(T, &str)],
) {
    row(ui, v, |ui, v| {
        ui.label(label);
        for (o, name) in options {
            ui.selectable_value(v, o.clone(), *name);
        }
    });
}

/// What the panel shows besides the settings.
struct Head<'a> {
    stats: Stats,
    fps: (f32, f32, f32),
    message: &'a str,
    profiles: &'a [String],
    active: &'a str,
    max_height: f32,
}

#[allow(clippy::too_many_lines)] // one row of buttons and its two modes
fn profile_row(
    ui: &mut egui::Ui,
    e: &Ed,
    head: &Head,
    edit: &mut ProfileEdit,
    actions: &mut Vec<Action>,
) {
    let modified = e.s != e.saved;
    ui.horizontal(|ui| {
        ui.label("Profile");
        let shown = if modified {
            format!("{} (changed)", head.active)
        } else {
            head.active.to_owned()
        };
        egui::ComboBox::from_id_salt("profile")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for name in head.profiles {
                    if ui.selectable_label(name == head.active, name).clicked()
                        && name != head.active
                    {
                        actions.push(Action::Switch(name.clone()));
                    }
                }
            });
        if ui
            .add_enabled(modified, egui::Button::new("Save"))
            .on_hover_text("Write these settings into the profile")
            .clicked()
        {
            actions.push(Action::Save);
        }
        if ui
            .add_enabled(modified, egui::Button::new("Revert"))
            .on_hover_text("Every setting back to the profile's")
            .clicked()
        {
            actions.push(Action::Revert);
        }
    });
    match edit {
        ProfileEdit::None => {
            ui.horizontal(|ui| {
                if ui
                    .button("New")
                    .on_hover_text("A profile with the factory defaults")
                    .clicked()
                {
                    actions.push(Action::New);
                }
                if ui
                    .button("Duplicate")
                    .on_hover_text("A copy of these settings")
                    .clicked()
                {
                    actions.push(Action::Duplicate);
                }
                if ui.button("Rename").clicked() {
                    *edit = ProfileEdit::Rename(head.active.to_owned());
                }
                if ui
                    .add_enabled(head.profiles.len() > 1, egui::Button::new("Delete"))
                    .clicked()
                {
                    *edit = ProfileEdit::Delete;
                }
                if ui
                    .button("Copy")
                    .on_hover_text("The saved profile to the clipboard, to share")
                    .clicked()
                {
                    actions.push(Action::Copy);
                }
                if ui
                    .button("Paste")
                    .on_hover_text("A profile from the clipboard, as a new profile")
                    .clicked()
                {
                    actions.push(Action::Paste);
                }
            });
        }
        ProfileEdit::Rename(name) => {
            let mut done = None;
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(name).desired_width(160.0));
                r.request_focus();
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    done = Some(true);
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    done = Some(false);
                }
            });
            if let Some(ok) = done {
                if ok {
                    actions.push(Action::Rename(name.clone()));
                }
                *edit = ProfileEdit::None;
            }
        }
        ProfileEdit::Delete => {
            ui.horizontal(|ui| {
                ui.label(format!("Delete \"{}\"?", head.active));
                if ui.button("Delete").clicked() {
                    actions.push(Action::Delete);
                    *edit = ProfileEdit::None;
                }
                if ui.button("Keep").clicked() {
                    *edit = ProfileEdit::None;
                }
            });
        }
    }
}

#[allow(clippy::too_many_lines)]
fn panel(
    ctx: &egui::Context,
    e: &mut Ed,
    head: &Head,
    edit: &mut ProfileEdit,
    actions: &mut Vec<Action>,
    editor: &mut bool,
    free: &mut crate::freecam::FreeCam,
) {
    egui::Window::new("FFX Renderer")
        .default_pos(egui::pos2(16.0, 16.0))
        .default_width(360.0)
        .resizable([true, false])
        .show(ctx, |ui| {
            // Fixed widths, so the panel does not jump as the figures change.
            let (fps, mean, worst) = head.fps;
            ui.label(
                egui::RichText::new(format!(
                    "{fps:4.0} fps {mean:5.1} ms, longest {worst:5.1} ms"
                ))
                .monospace(),
            );
            let gpu = if head.stats.gpu.is_empty() {
                "  ?".to_owned()
            } else {
                format!("{:5.1}", head.stats.gpu_ms)
            };
            ui.label(
                egui::RichText::new(format!(
                    "CPU {:5.1} ms, GPU {gpu} ms, {:4} enhanced, {:4} original",
                    head.stats.draw_ms, head.stats.enhanced_items, head.stats.other_draws
                ))
                .monospace(),
            );
            if !head.stats.gpu.is_empty() {
                egui::CollapsingHeader::new("GPU time per pass")
                    .id_salt("gpu passes")
                    .show(ui, |ui| {
                        egui::Grid::new("gpu grid").num_columns(2).show(ui, |ui| {
                            for (name, ms) in &head.stats.gpu {
                                ui.label(*name);
                                ui.label(egui::RichText::new(format!("{ms:5.1} ms")).monospace());
                                ui.end_row();
                            }
                        });
                    });
            }
            ui.horizontal(|ui| {
                ui.label("F10 hides the overlay.");
                ui.toggle_value(editor, "Editor (F11)");
                let mut on = free.on;
                if ui
                    .toggle_value(&mut on, "Free camera")
                    .on_hover_text(
                        "Right-drag looks, W/A/S/D move, Q/E down and up, Shift faster, \
                         the wheel sets the speed, middle-drag pans. The game gets no keys \
                         meanwhile; it draws for this camera.",
                    )
                    .changed()
                {
                    if on {
                        free.start_asked = true;
                    } else {
                        free.on = false;
                    }
                }
            });
            if free.on {
                ui.horizontal(|ui| {
                    ui.label(format!("speed {:.0}/s", free.speed));
                    ui.add(
                        egui::Slider::new(&mut free.zoom, 0.25..=4.0)
                            .logarithmic(true)
                            .text("zoom"),
                    );
                });
            }
            ui.label("Keys reach the game unless the pointer is over a panel.");
            ui.separator();
            profile_row(ui, e, head, edit, actions);
            ui.separator();
            // What does not fit on the screen scrolls.
            let used = ui.min_rect().height();
            egui::ScrollArea::vertical()
                .max_height((head.max_height - used - 72.0).max(120.0))
                .auto_shrink([false, true])
                .show(ui, |ui| settings_ui(ui, e));
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button("Factory defaults")
                    .on_hover_text("Every setting to the renderer's defaults (not saved)")
                    .clicked()
                {
                    actions.push(Action::Defaults);
                }
                ui.label(head.message);
            });
        });
}

#[allow(clippy::too_many_lines)]
fn settings_ui(ui: &mut egui::Ui, e: &mut Ed) {
    check(
        ui,
        f!(e.enabled),
        "Enhanced lighting (off: the original look)",
    );
    ui.label("Lights and exposure are in the editor (F11), per scene.");
    egui::CollapsingHeader::new("Characters").show(ui, |ui| {
        check(
            ui,
            f!(e.probes),
            "Light probes: characters, and the bounce on dynamic scenery",
        );
        slider(ui, f!(e.probe_mix), "probe share", 0.0..=1.0);
        slider(ui, f!(e.skin), "skin scattering", 0.0..=1.0);
        slider(
            ui,
            f!(e.skin_materials),
            "skin found by colour (faces)",
            0.0..=1.0,
        );
        slider(ui, f!(e.skin_roughness), "skin roughness", 0.05..=1.0);
        slider(ui, f!(e.skin_detail), "surface detail on skin", 0.0..=1.0);
        slider(
            ui,
            f!(e.character_falloff),
            "character falloff (soft–physical)",
            0.0..=1.0,
        );
        check(
            ui,
            f!(e.material_maps),
            "Surface detail from the textures (normal, roughness)",
        );
        slider(ui, f!(e.normal_strength), "surface detail", 0.0..=4.0);
        slider(ui, f!(e.character_detail), "on characters", 0.0..=2.0);
        slider(ui, f!(e.roughness), "roughness", 0.05..=1.0);
        slider(ui, f!(e.specular), "specular", 0.0..=2.0);
    });
    egui::CollapsingHeader::new("Shadows").show(ui, |ui| {
        check(
            ui,
            f!(e.ray_traced_shadows),
            "Ray-traced from the scenery (if the GPU can)",
        );
        slider(ui, f!(e.ray_distance), "ray distance", 10.0..=1000.0);
        slider(ui, f!(e.character_shadows), "character shadows", 0.0..=1.0);
        check(ui, f!(e.shadows), "Real shadows");
        check(ui, f!(e.fake_shadows), "The game's blob shadows too");
        step_slider(ui, f!(e.shadow_size), "map size", 512.0..=8192.0, 512.0);
        slider(ui, f!(e.shadow_extent), "area (units)", 10.0..=400.0);
        slider(ui, f!(e.shadow_softness), "softness", 0.0..=8.0);
    });
    egui::CollapsingHeader::new("Effects").show(ui, |ui| {
        check(
            ui,
            f!(e.effect_light),
            "Fire and magic light their surroundings",
        );
        slider(ui, f!(e.effect_light_strength), "strength", 0.0..=10.0);
        check(
            ui,
            f!(e.glow_lights),
            "Glowing things (save spheres, lamps) are lights",
        );
        slider(ui, f!(e.glow_strength), "their brightness", 0.0..=5.0);
    });
    egui::CollapsingHeader::new("Water").show(ui, |ui| {
        check(ui, f!(e.water), "Enhanced water");
        slider(ui, f!(e.water_reflection), "reflection", 0.0..=2.0);
        slider(ui, f!(e.water_refraction), "refraction", 0.0..=0.2);
        log_slider(ui, f!(e.water_absorption), "absorption", 0.0..=0.3);
        slider(ui, f!(e.water_environment), "environment", 0.0..=2.0);
        color3(ui, f!(e.water_deep), "deep colour");
    });
    egui::CollapsingHeader::new("Volumetric light").show(ui, |ui| {
        check(ui, f!(e.volumetrics), "Light shafts");
        log_slider(ui, f!(e.fog_density), "density", 0.0..=0.02);
        slider(ui, f!(e.fog_anisotropy), "forward scattering", 0.0..=0.95);
        slider(ui, f!(e.volumetric_strength), "shaft brightness", 0.0..=1.0);
        slider(ui, f!(e.haze), "haze", 0.0..=2.0);
        step_slider(ui, f!(e.volumetric_steps), "steps", 8.0..=64.0, 4.0);
        slider(ui, f!(e.volumetric_distance), "distance", 100.0..=3000.0);
    });
    egui::CollapsingHeader::new("Ambient occlusion").show(ui, |ui| {
        check(ui, f!(e.ao), "On");
        slider(ui, f!(e.ao_radius), "radius", 0.5..=20.0);
        slider(ui, f!(e.ao_strength), "strength", 0.0..=2.0);
    });
    egui::CollapsingHeader::new("Image").show(ui, |ui| {
        choice(
            ui,
            f!(e.tone_curve),
            "tone curve",
            &[(0, "game"), (1, "ACES"), (2, "filmic")],
        );
        slider(ui, f!(e.saturation), "saturation", 0.0..=2.0);
        slider(ui, f!(e.contrast), "contrast", 0.5..=1.5);
        check(ui, f!(e.fog), "The game's fog");
        check(
            ui,
            f!(e.afterimage),
            "The game's afterimage (a trail behind what moves)",
        );
    });
    egui::CollapsingHeader::new("Anti-aliasing").show(ui, |ui| {
        choice(
            ui,
            f!(e.antialiasing),
            "method",
            &[
                (0, "off"),
                (1, "FXAA"),
                (2, "temporal (TAA)"),
                (3, "AMD FSR 3"),
            ],
        );
        if e.s.antialiasing == 2 {
            slider(
                ui,
                f!(e.taa_blend),
                "new frame weight (lower: smoother)",
                0.02..=0.5,
            );
            slider(
                ui,
                f!(e.taa_clamp),
                "history tolerance (lower: less ghosting)",
                0.5..=3.0,
            );
        }
        if e.s.antialiasing == 3 {
            choice(
                ui,
                f!(e.fsr_mode),
                "FSR",
                &[
                    (0, "native"),
                    (1, "quality"),
                    (2, "balanced"),
                    (3, "performance"),
                    (4, "ultra perf."),
                ],
            );
            slider(ui, f!(e.fsr_sharpness), "FSR sharpening", 0.0..=1.0);
        }
        slider(ui, f!(e.sharpen), "sharpening", 0.0..=1.5);
        // FSR sets the render size by its mode.
        row(ui, f!(e.render_scale), |ui, v| {
            ui.add(
                egui::Slider::new(v, 0.5..=2.0)
                    .step_by(0.25)
                    .text("render scale (above 1: supersampling)"),
            );
        });
    });
    egui::CollapsingHeader::new("Camera effects").show(ui, |ui| {
        check(ui, f!(e.bloom), "Bloom");
        if e.s.bloom {
            slider(ui, f!(e.bloom_strength), "strength", 0.0..=2.0);
            slider(ui, f!(e.bloom_threshold), "threshold", 0.0..=4.0);
            slider(ui, f!(e.bloom_knee), "soft start", 0.0..=2.0);
            slider(ui, f!(e.bloom_radius), "spread", 0.5..=3.0);
        }
        ui.separator();
        check(ui, f!(e.depth_of_field), "Depth of field");
        if e.s.depth_of_field {
            slider(ui, f!(e.dof_strength), "blur", 0.0..=2.0);
            slider(ui, f!(e.dof_radius), "largest blur (px)", 1.0..=24.0);
            slider(
                ui,
                f!(e.dof_focus),
                "focus distance (0: automatic)",
                0.0..=500.0,
            );
            check(
                ui,
                f!(e.dof_follow_camera),
                "Automatic: on what the camera looks at (else the centre)",
            );
            slider(ui, f!(e.dof_speed), "focus speed", 0.01..=1.0);
        }
        ui.separator();
        check(ui, f!(e.motion_blur), "Motion blur");
        if e.s.motion_blur {
            slider(ui, f!(e.motion_blur_shutter), "shutter", 0.0..=1.0);
            slider(
                ui,
                f!(e.motion_blur_length),
                "longest blur (px)",
                4.0..=96.0,
            );
            check(ui, f!(e.motion_blur_camera), "From camera motion too");
        }
    });
    egui::CollapsingHeader::new("Debug").show(ui, |ui| {
        row(ui, f!(e.debug_view), |ui, v| {
            ui.horizontal_wrapped(|ui| {
                for (n, name) in [
                    (0, "image"),
                    (1, "shadow"),
                    (2, "occlusion"),
                    (3, "normals"),
                    (4, "albedo"),
                    (5, "baked light"),
                    (6, "cascades"),
                    (7, "skin"),
                    (8, "probe light"),
                    (9, "local lights"),
                    (10, "material"),
                ] {
                    ui.selectable_value(v, n, name);
                }
            });
        });
    });
}
