//! The external renderer: keeps a copy of the game's GPU resources from
//! the scene stream (`ffx-gfx`) and draws frames with wgpu, running the
//! game's own shaders through `ffx-dxbc`'s WGSL translation.

// GPU sizes, offsets and coordinates move between u32, i32, f32 and
// usize all the time; the values are small.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    clippy::items_after_statements
)]

pub mod enhanced;
pub mod families;
pub mod formats;
#[cfg(windows)]
pub mod live;
pub mod overlay;
pub mod png;
mod replay;
#[cfg(windows)]
pub mod share;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use ffx_gfx::{Id, Msg, StateKind, ViewKind};

pub use replay::Filter;

/// A mirrored texture.
pub struct Texture {
    /// FNV-1a of the top level's bytes as the game created it (0 without
    /// data): the key of material packs.
    pub hash: u64,
    pub texture: wgpu::Texture,
    pub desc: ffx_gfx::TextureDesc,
    pub format: wgpu::TextureFormat,
}

pub struct View {
    pub kind: ViewKind,
    pub resource: Id,
    pub desc: Option<[u32; 6]>,
}

pub struct Buffer {
    pub buffer: wgpu::Buffer,
    pub desc: ffx_gfx::BufferDesc,
    /// The contents, for constant buffers drawn from (none for vertex and
    /// index data, which lives on the GPU).
    pub data: Vec<u8>,
}

pub struct Shader {
    pub bytecode: Arc<[u8]>,
    pub hash: u64,
}

/// The renderer's device and its copy of the game's objects.
pub struct Renderer {
    /// The adapter and backend in use.
    pub adapter: String,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub textures: HashMap<Id, Texture>,
    pub views: HashMap<Id, View>,
    pub buffers: HashMap<Id, Buffer>,
    pub shaders: HashMap<Id, Shader>,
    pub layouts: HashMap<Id, Vec<ffx_gfx::Element>>,
    pub states: HashMap<Id, (StateKind, Vec<u8>)>,
    cache: replay::Cache,
    /// Messages that did not apply (unknown formats, …), for reports.
    pub warnings: Vec<String>,
}

impl Renderer {
    /// A renderer on the default high-performance adapter.
    ///
    /// # Errors
    /// When there is no adapter or device with the needed features.
    pub fn new() -> Result<Self> {
        // Direct3D 12 on Windows (its textures can be shared with the
        // game's device); `FFX_RENDER_BACKEND=vulkan` picks Vulkan.
        let backends = match std::env::var("FFX_RENDER_BACKEND").as_deref() {
            Ok("vulkan") => wgpu::Backends::VULKAN,
            Ok("dx12") => wgpu::Backends::DX12,
            _ if cfg!(windows) => wgpu::Backends::DX12,
            _ => wgpu::Backends::PRIMARY,
        };
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .context("no GPU adapter")?;
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.backend);
        let wanted = wgpu::Features::TEXTURE_COMPRESSION_BC
            | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
            | wgpu::Features::FLOAT32_FILTERABLE
            | wgpu::Features::DUAL_SOURCE_BLENDING
            | wgpu::Features::DEPTH32FLOAT_STENCIL8;
        let features = adapter.features() & wanted;
        let limits = adapter.limits();
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ffx-render"),
            required_features: features,
            required_limits: limits,
            ..Default::default()
        }))
        .context("no GPU device")?;
        Ok(Self {
            adapter: adapter_name,
            device,
            queue,
            textures: HashMap::new(),
            views: HashMap::new(),
            buffers: HashMap::new(),
            shaders: HashMap::new(),
            layouts: HashMap::new(),
            states: HashMap::new(),
            cache: replay::Cache::default(),
            warnings: Vec::new(),
        })
    }

    /// Applies one message; frames are drawn with `filter`.
    ///
    /// # Errors
    /// When a frame cannot be drawn.
    pub fn apply(&mut self, msg: Msg, filter: &Filter) -> Result<()> {
        match msg {
            Msg::Buffer { id, desc, data } => self.create_buffer(id, desc, &data),
            Msg::Texture {
                id,
                desc,
                subresources,
            } => self.create_texture(id, desc, &subresources),
            Msg::View {
                id,
                kind,
                resource,
                desc,
            } => {
                self.views.insert(
                    id,
                    View {
                        kind,
                        resource,
                        desc,
                    },
                );
            }
            Msg::Shader { id, bytecode, .. } => {
                let hash = ffx_dxbc::hash(&bytecode);
                self.shaders.insert(
                    id,
                    Shader {
                        bytecode: bytecode.into(),
                        hash,
                    },
                );
            }
            Msg::Layout { id, elements } => {
                self.layouts.insert(id, elements);
            }
            Msg::State { id, kind, desc } => {
                self.states.insert(id, (kind, desc));
            }
            Msg::UpdateBuffer { id, offset, data } => self.update_buffer(id, offset, &data),
            Msg::UpdateTexture {
                id,
                subresource,
                row_pitch,
                data,
            } => self.update_texture(id, subresource, row_pitch, &data),
            Msg::Release { id } => {
                self.textures.remove(&id);
                self.buffers.remove(&id);
                self.views.remove(&id);
                self.shaders.remove(&id);
            }
            Msg::Frame(frame) => self.draw_frame(&frame, filter)?,
            // Overlay input is the live loop's.
            Msg::Input { .. } => {}
        }
        Ok(())
    }

    fn create_buffer(&mut self, id: Id, desc: ffx_gfx::BufferDesc, data: &[u8]) {
        let size = u64::from(desc[0].max(4)).next_multiple_of(4);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::INDEX
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Constant and index data stay on the CPU too: draws read windows
        // of the first, and the second tells which vertices a draw uses.
        let keep = desc[2] & 0x6 != 0;
        let mut kept = Vec::new();
        if !data.is_empty() {
            let mut padded = data.to_vec();
            padded.resize(size as usize, 0);
            self.queue.write_buffer(&buffer, 0, &padded);
            if keep {
                kept = data.to_vec();
            }
        }
        self.buffers.insert(
            id,
            Buffer {
                buffer,
                desc,
                data: kept,
            },
        );
    }

    fn update_buffer(&mut self, id: Id, offset: u32, data: &[u8]) {
        let Some(b) = self.buffers.get_mut(&id) else {
            return;
        };
        if b.desc[2] & 0x6 != 0 {
            let end = offset as usize + data.len();
            if b.data.len() < end {
                b.data.resize(end, 0);
            }
            b.data[offset as usize..end].copy_from_slice(data);
        }
        // write_buffer wants 4-byte multiples.
        let start = offset & !3;
        let mut padded = vec![0u8; (offset - start) as usize];
        padded.extend_from_slice(data);
        padded.resize(padded.len().next_multiple_of(4), 0);
        let limit = b.buffer.size();
        if u64::from(start) + padded.len() as u64 <= limit {
            self.queue
                .write_buffer(&b.buffer, u64::from(start), &padded);
        }
    }

    fn create_texture(
        &mut self,
        id: Id,
        desc: ffx_gfx::TextureDesc,
        subresources: &[ffx_gfx::Subresource],
    ) {
        let bind = desc[8];
        let depth = bind & 0x40 != 0;
        let Some(format) = formats::texture_format(desc[4], depth) else {
            self.warnings
                .push(format!("texture {id:#x}: format {}", desc[4]));
            return;
        };
        let (w, h) = (desc[0].max(1), desc[1].max(1));
        let (_, bw) = formats::block(format);
        let size = wgpu::Extent3d {
            width: w.next_multiple_of(bw),
            height: h.next_multiple_of(bw),
            depth_or_array_layers: desc[3].max(1),
        };
        let mips = desc[2].max(1);
        let mut usage = wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC;
        if bind & 0x60 != 0 && !format.is_compressed() {
            usage |= wgpu::TextureUsages::RENDER_ATTACHMENT;
        }
        // An sRGB view too, for sampling colours in linear space.
        let srgb = format.add_srgb_suffix();
        let view_formats: &[wgpu::TextureFormat] = if srgb == format {
            &[]
        } else {
            std::slice::from_ref(&srgb)
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: mips,
            sample_count: desc[5].max(1),
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats,
        });
        if desc[5] <= 1 && !format.is_depth_stencil_format() {
            for (i, s) in subresources.iter().enumerate() {
                let mip = i as u32 % mips;
                let layer = i as u32 / mips;
                self.write_subresource(&texture, format, (w, h), mip, layer, s.row_pitch, &s.data);
            }
        }
        self.textures.insert(
            id,
            Texture {
                hash: subresources.first().map_or(0, |s| fnv(&s.data)),
                texture,
                desc,
                format,
            },
        );
    }

    /// Reads `texture` (RGBA8, level 0, `width` × `height`) back.
    ///
    /// # Errors
    /// When the GPU fails.
    pub fn read_texture(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let pitch = (width * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read-back"),
            size: u64::from(pitch * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        enc.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range().map_err(|e| anyhow!("{e:?}"))?;
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            out.extend_from_slice(
                &data[y * pitch as usize..y * pitch as usize + 4 * width as usize],
            );
        }
        Ok(out)
    }

    #[allow(clippy::too_many_arguments)]
    fn write_subresource(
        &self,
        texture: &wgpu::Texture,
        format: wgpu::TextureFormat,
        (w, h): (u32, u32),
        mip: u32,
        layer: u32,
        row_pitch: u32,
        data: &[u8],
    ) {
        let (bytes, bw) = formats::block(format);
        let mw = (w >> mip).max(1);
        let mh = (h >> mip).max(1);
        let pw = mw.next_multiple_of(bw);
        let ph = mh.next_multiple_of(bw);
        let rows = ph / bw;
        let needed = (pw / bw) * bytes;
        if row_pitch < needed || data.len() < (row_pitch * (rows - 1) + needed) as usize {
            return;
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_pitch),
                rows_per_image: Some(rows),
            },
            wgpu::Extent3d {
                width: pw,
                height: ph,
                depth_or_array_layers: 1,
            },
        );
    }

    fn update_texture(&mut self, id: Id, subresource: u32, row_pitch: u32, data: &[u8]) {
        let Some(t) = self.textures.get(&id) else {
            return;
        };
        let mips = t.desc[2].max(1);
        let (texture, format, size) = (t.texture.clone(), t.format, (t.desc[0], t.desc[1]));
        self.write_subresource(
            &texture,
            format,
            size,
            subresource % mips,
            subresource / mips,
            row_pitch,
            data,
        );
    }

    /// Reads the scene targets back in one go: colour and normal as RGBA8,
    /// then depth as f32, rows packed.
    ///
    /// # Errors
    /// When a target is unknown or of another format.
    pub fn read_scene(&self, color: Id, normal: Id, depth: Id) -> Result<(u32, u32, Vec<u8>)> {
        let get = |id: Id| {
            self.textures
                .get(&id)
                .ok_or_else(|| anyhow!("no texture {id:#x}"))
        };
        let (c, n, z) = (get(color)?, get(normal)?, get(depth)?);
        let (w, h) = (c.desc[0], c.desc[1]);
        let pitch = (w * 4).next_multiple_of(256);
        let plane = u64::from(pitch * h);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene read-back"),
            size: plane * 3,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for (i, (t, aspect)) in [
            (c, wgpu::TextureAspect::All),
            (n, wgpu::TextureAspect::All),
            (z, wgpu::TextureAspect::DepthOnly),
        ]
        .into_iter()
        .enumerate()
        {
            enc.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &t.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: plane * i as u64,
                        bytes_per_row: Some(pitch),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range().map_err(|e| anyhow!("{e:?}"))?;
        let mut out = Vec::with_capacity((w * h * 12) as usize);
        for i in 0..3 {
            for y in 0..h as usize {
                let at = plane as usize * i + y * pitch as usize;
                out.extend_from_slice(&data[at..at + 4 * w as usize]);
            }
        }
        Ok((w, h, out))
    }

    /// Reads level `mip` of a texture back as raw blocks, rows packed.
    ///
    /// # Errors
    /// When the texture is unknown.
    pub fn read_raw(&self, id: Id, mip: u32) -> Result<Vec<u8>> {
        let t = self
            .textures
            .get(&id)
            .ok_or_else(|| anyhow!("no texture {id:#x}"))?;
        let (bytes, bw) = formats::block(t.format);
        let w = (t.desc[0] >> mip).max(1).next_multiple_of(bw);
        let h = (t.desc[1] >> mip).max(1).next_multiple_of(bw);
        let row = (w / bw) * bytes;
        let pitch = row.next_multiple_of(256);
        let rows = h / bw;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(pitch * rows),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &t.texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(rows),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range().map_err(|e| anyhow!("{e:?}"))?;
        let mut out = Vec::new();
        for y in 0..rows as usize {
            out.extend_from_slice(&data[y * pitch as usize..y * pitch as usize + row as usize]);
        }
        Ok(out)
    }

    /// Reads a texture's first level back as RGBA8 (formats it can read:
    /// 8-bit RGBA and BGRA).
    ///
    /// # Errors
    /// When the texture is unknown or not readable.
    pub fn read_rgba(&self, id: Id) -> Result<(u32, u32, Vec<u8>)> {
        let t = self
            .textures
            .get(&id)
            .ok_or_else(|| anyhow!("no texture {id:#x}"))?;
        let (w, h) = (t.desc[0], t.desc[1]);
        let bgra = matches!(
            t.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        if !bgra
            && !matches!(
                t.format,
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb
            )
        {
            return Err(anyhow!("cannot read format {:?}", t.format));
        }
        let pitch = (w * 4).next_multiple_of(256);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(pitch * h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        enc.copy_texture_to_buffer(
            t.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        let data = slice.get_mapped_range().map_err(|e| anyhow!("{e:?}"))?;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h as usize {
            let row = &data[y * pitch as usize..y * pitch as usize + 4 * w as usize];
            for px in row.as_chunks::<4>().0 {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                } else {
                    out.extend_from_slice(px);
                }
            }
        }
        Ok((w, h, out))
    }
}

/// FNV-1a, 64 bits.
#[must_use]
pub fn fnv(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}
