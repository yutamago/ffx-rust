//! The in-game settings overlay: an egui panel the renderer draws into a
//! transparent image the hook lays over the game's frame. The game window
//! sends the pointer, wheel and keys while it is shown (F10 shows and
//! hides it); changes apply at once and "Save" writes `ffx-render.toml`.

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use ffx_gfx::Input;

use crate::enhanced::Settings;

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

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return in.color * textureSample(tex, samp, in.uv);
}
";

/// Frame statistics the overlay shows.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub frame_ms: f32,
    pub draw_ms: f32,
    pub enhanced_items: usize,
    pub other_draws: usize,
}

struct Painter {
    pipeline: wgpu::RenderPipeline,
    screen_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<egui::TextureId, (wgpu::Texture, wgpu::BindGroup)>,
    target: Option<((u32, u32), wgpu::Texture, wgpu::TextureView)>,
}

impl Painter {
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
        }
    }

    fn set_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: egui::TextureId,
        delta: &egui::epaint::ImageDelta,
    ) {
        let egui::ImageData::Color(image) = &delta.image;
        let [w, h] = image.size;
        let pixels: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
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
                self.textures.insert(id, (texture, group));
                wgpu::Origin3d::ZERO
            }
        };
        if let Some((texture, _)) = self.textures.get(&id) {
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
    ) -> Result<Vec<u8>> {
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
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut u = Vec::new();
        for f in [points.0, points.1, 0.0, 0.0] {
            u.extend_from_slice(&f32::to_le_bytes(f));
        }
        queue.write_buffer(&ubuf, 0, &u);
        let screen = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.screen_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        let pitch = (size.0 * 4).next_multiple_of(256);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(pitch * size.1),
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
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
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
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range().map_err(|e| anyhow!("{e:?}"))?;
        let mut out = Vec::with_capacity((size.0 * size.1 * 4) as usize);
        for y in 0..size.1 as usize {
            out.extend_from_slice(
                &data[y * pitch as usize..y * pitch as usize + 4 * size.0 as usize],
            );
        }
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

pub struct Overlay {
    ctx: egui::Context,
    painter: Painter,
    pub visible: bool,
    pointer: egui::Pos2,
    events: Vec<egui::Event>,
    start: std::time::Instant,
    /// Set by the panel: write the settings file.
    pub save: bool,
    /// Set by the panel: go back to the defaults.
    pub reset: bool,
    pub message: String,
}

impl Overlay {
    #[must_use]
    pub fn new(device: &wgpu::Device) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        Self {
            ctx,
            painter: Painter::new(device),
            visible: false,
            pointer: egui::Pos2::ZERO,
            events: Vec::new(),
            start: std::time::Instant::now(),
            save: false,
            reset: false,
            message: String::new(),
        }
    }

    /// Takes the game window's input.
    pub fn feed(&mut self, events: &[Input]) {
        for e in events {
            match *e {
                Input::Toggle => self.visible = !self.visible,
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

    /// Runs the panel over a `size` screen; returns the overlay image.
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
    ) -> Result<Vec<u8>> {
        let ppp = (size.1 as f32 / 720.0).max(1.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.0 as f32 / ppp, size.1 as f32 / ppp),
            )),
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
        let mut save = false;
        let mut reset = false;
        let message = self.message.clone();
        let out = self.ctx.run_ui(input, |ui| {
            panel(ui.ctx(), settings, stats, &mut save, &mut reset, &message);
        });
        self.save |= save;
        self.reset |= reset;
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
}

fn color3(ui: &mut egui::Ui, label: &str, c: &mut [f32; 3]) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_rgb(c);
    });
}

fn slider(ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>) {
    ui.add(egui::Slider::new(v, range).text(label));
}

#[allow(clippy::too_many_lines)]
fn panel(
    ctx: &egui::Context,
    s: &mut Settings,
    stats: Stats,
    save: &mut bool,
    reset: &mut bool,
    message: &str,
) {
    egui::Window::new("FFX Renderer")
        .default_pos(egui::pos2(16.0, 16.0))
        .default_width(330.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(format!(
                "{:.1} ms per frame, drawing {:.1} ms; {} enhanced, {} original draws",
                stats.frame_ms, stats.draw_ms, stats.enhanced_items, stats.other_draws
            ));
            ui.label("F10 hides this panel.");
            ui.separator();
            ui.checkbox(&mut s.enabled, "Enhanced lighting (off: the original look)");
            egui::CollapsingHeader::new("Light")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut s.map_lights, "Use the map's own lights");
                    if s.map_lights {
                        slider(ui, "key light", &mut s.key_scale, 0.0..=3.0);
                        slider(ui, "fill light", &mut s.fill_scale, 0.0..=2.0);
                        slider(ui, "sky light", &mut s.sky_scale, 0.0..=3.0);
                    } else {
                        slider(ui, "sun", &mut s.sun_intensity, 0.0..=10.0);
                        color3(ui, "sun colour", &mut s.sun_color);
                        ui.horizontal(|ui| {
                            ui.label("sun direction");
                            ui.add(
                                egui::DragValue::new(&mut s.sun_direction[0])
                                    .speed(0.01)
                                    .range(-1.0..=1.0),
                            );
                            ui.add(
                                egui::DragValue::new(&mut s.sun_direction[1])
                                    .speed(0.01)
                                    .range(-1.0..=1.0),
                            );
                            ui.add(
                                egui::DragValue::new(&mut s.sun_direction[2])
                                    .speed(0.01)
                                    .range(-1.0..=1.0),
                            );
                        });
                        slider(ui, "sky", &mut s.sky_intensity, 0.0..=5.0);
                        color3(ui, "sky colour", &mut s.sky_color);
                        color3(ui, "ground colour", &mut s.ground_color);
                    }
                    slider(ui, "baked light", &mut s.baked_light, 0.0..=2.0);
                    slider(ui, "key light on scenery", &mut s.scenery_key, 0.0..=1.0);
                    slider(ui, "scenery specular", &mut s.scenery_specular, 0.0..=2.0);
                    slider(ui, "character ambient", &mut s.character_ambient, 0.0..=4.0);
                    ui.checkbox(&mut s.probes, "Characters lit by the scenery around them");
                    slider(ui, "probe light", &mut s.probe_strength, 0.0..=3.0);
                    slider(ui, "probe share", &mut s.probe_mix, 0.0..=1.0);
                    slider(ui, "skin scattering", &mut s.skin, 0.0..=1.0);
                    slider(
                        ui,
                        "character falloff (soft–physical)",
                        &mut s.character_falloff,
                        0.0..=1.0,
                    );
                    ui.checkbox(
                        &mut s.material_maps,
                        "Surface detail from the textures (normal, roughness)",
                    );
                    slider(ui, "surface detail", &mut s.normal_strength, 0.0..=4.0);
                    slider(ui, "on characters", &mut s.character_detail, 0.0..=2.0);
                    slider(ui, "roughness", &mut s.roughness, 0.05..=1.0);
                    slider(ui, "specular", &mut s.specular, 0.0..=2.0);
                });
            egui::CollapsingHeader::new("Shadows").show(ui, |ui| {
                ui.checkbox(&mut s.shadows, "Real shadows");
                ui.checkbox(&mut s.fake_shadows, "The game's blob shadows too");
                let mut size = s.shadow_size as f32;
                ui.add(
                    egui::Slider::new(&mut size, 512.0..=8192.0)
                        .step_by(512.0)
                        .text("map size"),
                );
                s.shadow_size = size as u32;
                slider(ui, "area (units)", &mut s.shadow_extent, 10.0..=400.0);
                slider(ui, "softness", &mut s.shadow_softness, 0.0..=8.0);
            });
            egui::CollapsingHeader::new("Effects").show(ui, |ui| {
                ui.checkbox(
                    &mut s.effect_light,
                    "Fire and magic light their surroundings",
                );
                slider(ui, "strength", &mut s.effect_light_strength, 0.0..=10.0);
            });
            egui::CollapsingHeader::new("Water").show(ui, |ui| {
                ui.checkbox(&mut s.water, "Enhanced water");
                slider(ui, "reflection", &mut s.water_reflection, 0.0..=2.0);
                slider(ui, "refraction", &mut s.water_refraction, 0.0..=0.2);
                ui.add(
                    egui::Slider::new(&mut s.water_absorption, 0.0..=0.3)
                        .logarithmic(true)
                        .text("absorption"),
                );
                slider(ui, "environment", &mut s.water_environment, 0.0..=2.0);
                color3(ui, "deep colour", &mut s.water_deep);
            });
            egui::CollapsingHeader::new("Volumetric light").show(ui, |ui| {
                ui.checkbox(&mut s.volumetrics, "Light shafts");
                ui.add(
                    egui::Slider::new(&mut s.fog_density, 0.0..=0.02)
                        .logarithmic(true)
                        .text("density"),
                );
                slider(ui, "forward scattering", &mut s.fog_anisotropy, 0.0..=0.95);
                slider(
                    ui,
                    "shaft brightness",
                    &mut s.volumetric_strength,
                    0.0..=1.0,
                );
                slider(ui, "haze", &mut s.haze, 0.0..=2.0);
                let mut steps = s.volumetric_steps as f32;
                ui.add(
                    egui::Slider::new(&mut steps, 8.0..=64.0)
                        .step_by(4.0)
                        .text("steps"),
                );
                s.volumetric_steps = steps as u32;
                slider(ui, "distance", &mut s.volumetric_distance, 100.0..=3000.0);
            });
            egui::CollapsingHeader::new("Ambient occlusion").show(ui, |ui| {
                ui.checkbox(&mut s.ao, "On");
                slider(ui, "radius", &mut s.ao_radius, 0.5..=20.0);
                slider(ui, "strength", &mut s.ao_strength, 0.0..=2.0);
                slider(ui, "on baked light", &mut s.ao_on_baked, 0.0..=1.0);
            });
            egui::CollapsingHeader::new("Image").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("tone curve");
                    ui.selectable_value(&mut s.tone_curve, 0, "game");
                    ui.selectable_value(&mut s.tone_curve, 1, "ACES");
                    ui.selectable_value(&mut s.tone_curve, 2, "filmic");
                });
                slider(ui, "exposure", &mut s.exposure, 0.1..=4.0);
                slider(ui, "saturation", &mut s.saturation, 0.0..=2.0);
                slider(ui, "contrast", &mut s.contrast, 0.5..=1.5);
                ui.checkbox(&mut s.fog, "The game's fog");
                ui.horizontal(|ui| {
                    ui.label("anti-aliasing");
                    ui.selectable_value(&mut s.antialiasing, 0, "off");
                    ui.selectable_value(&mut s.antialiasing, 1, "FXAA");
                });
                ui.add(
                    egui::Slider::new(&mut s.render_scale, 0.5..=2.0)
                        .step_by(0.25)
                        .text("render scale (above 1: supersampling)"),
                );
            });
            egui::CollapsingHeader::new("Debug").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (v, name) in [
                        (0, "image"),
                        (1, "shadow"),
                        (2, "occlusion"),
                        (3, "normals"),
                        (4, "albedo"),
                        (5, "baked light"),
                        (6, "cascades"),
                        (7, "skin"),
                        (8, "probe light"),
                    ] {
                        ui.selectable_value(&mut s.debug_view, v, name);
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    *save = true;
                }
                if ui.button("Defaults").clicked() {
                    *reset = true;
                }
                ui.label(message);
            });
        });
}
