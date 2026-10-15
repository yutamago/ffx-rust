//! The scene image shared with the game's device: instead of reading the
//! finished colour, normal and depth back to the CPU and sending the
//! pixels, the renderer draws them into Direct3D 12 textures created
//! shareable under a name, and the game opens them on its Direct3D 11
//! device. Three sets in turn, so the game's GPU can still read one while
//! the next is written. Names: `Local\ffx-gfx-<game pid>-<generation>-<set>-<plane>`
//! (planes: 0 colour, 1 normal, 2 depth); a new generation when the size
//! changes.

use anyhow::{Context, Result, anyhow};
use ffx_gfx::Id;
use windows::Win32::Foundation::{CloseHandle, GENERIC_ALL, HANDLE};
use windows::Win32::Graphics::Direct3D12::{
    D3D12_HEAP_FLAG_SHARED, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT, D3D12_RESOURCE_DESC,
    D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,
    D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS, D3D12_RESOURCE_STATE_COMMON,
    D3D12_TEXTURE_LAYOUT_UNKNOWN, ID3D12Resource,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R32_FLOAT, DXGI_SAMPLE_DESC,
};
use windows::core::PCWSTR;

use crate::Renderer;

const SETS: u32 = 3;

const SHARE_WGSL: &str = "
@group(0) @binding(0) var c_tex: texture_2d<f32>;
@group(0) @binding(1) var n_tex: texture_2d<f32>;
@group(0) @binding(2) var z_tex: texture_depth_2d;
struct VsOut { @builtin(position) pos: vec4<f32> };
@vertex
fn vs(@builtin(vertex_index) id: u32) -> VsOut {
    let uv = vec2<f32>(f32((id << 1u) & 2u), f32(id & 2u));
    var out: VsOut;
    out.pos = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    return out;
}
struct Out {
    @location(0) c: vec4<f32>,
    @location(1) n: vec4<f32>,
    @location(2) z: f32,
};
@fragment
fn fs(in: VsOut) -> Out {
    let p = vec2<i32>(in.pos.xy);
    var o: Out;
    o.c = textureLoad(c_tex, p, 0);
    o.n = textureLoad(n_tex, p, 0);
    o.z = textureLoad(z_tex, p, 0);
    return o;
}
";

/// What the game is told: where this frame's image is.
#[derive(Debug, Clone, Copy)]
pub struct Shared {
    pub width: u32,
    pub height: u32,
    pub generation: u32,
    pub set: u32,
}

pub struct Share {
    pid: u32,
    generation: u32,
    size: (u32, u32),
    /// Per set: colour, normal, depth.
    sets: Vec<[wgpu::TextureView; 3]>,
    handles: Vec<HANDLE>,
    next: u32,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl Drop for Share {
    fn drop(&mut self) {
        for h in self.handles.drain(..) {
            // SAFETY: handles this owns.
            let _ = unsafe { CloseHandle(h) };
        }
    }
}

impl Share {
    /// # Errors
    /// When the device is not Direct3D 12.
    pub fn new(device: &wgpu::Device, pid: u32) -> Result<Self> {
        // SAFETY: only checks the backend.
        if unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }.is_none() {
            return Err(anyhow!("not a Direct3D 12 device"));
        }
        let tex = |sample_type| wgpu::BindingType::Texture {
            sample_type,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("share"),
            entries: &[
                entry(0, tex(wgpu::TextureSampleType::Float { filterable: false })),
                entry(1, tex(wgpu::TextureSampleType::Float { filterable: false })),
                entry(2, tex(wgpu::TextureSampleType::Depth)),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("share"),
            source: wgpu::ShaderSource::Wgsl(SHARE_WGSL.into()),
        });
        let target = |format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("share"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("share"),
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                }),
            ),
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
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    target(wgpu::TextureFormat::Rgba8Unorm),
                    target(wgpu::TextureFormat::Rgba8Unorm),
                    target(wgpu::TextureFormat::R32Float),
                ],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            pid,
            generation: 0,
            size: (0, 0),
            sets: Vec::new(),
            handles: Vec::new(),
            next: 0,
            pipeline,
            layout,
        })
    }

    /// Makes the shared textures for `size` (a new generation).
    fn make(&mut self, device: &wgpu::Device, size: (u32, u32)) -> Result<()> {
        for h in self.handles.drain(..) {
            // SAFETY: handles this owns.
            let _ = unsafe { CloseHandle(h) };
        }
        self.sets.clear();
        self.generation += 1;
        self.size = size;
        // SAFETY: checked to be Direct3D 12 in `new`.
        let hal = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
            .ok_or_else(|| anyhow!("not a Direct3D 12 device"))?;
        let raw = hal.raw_device().clone();
        drop(hal);
        for set in 0..SETS {
            let mut views = Vec::new();
            for (plane, dxgi, format) in [
                (
                    0,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    wgpu::TextureFormat::Rgba8Unorm,
                ),
                (
                    1,
                    DXGI_FORMAT_R8G8B8A8_UNORM,
                    wgpu::TextureFormat::Rgba8Unorm,
                ),
                (2, DXGI_FORMAT_R32_FLOAT, wgpu::TextureFormat::R32Float),
            ] {
                let name = format!(
                    "Local\\ffx-gfx-{}-{}-{set}-{plane}",
                    self.pid, self.generation
                );
                let (resource, handle) = create_shared(&raw, size, dxgi, &name)
                    .with_context(|| format!("shared texture {name}"))?;
                self.handles.push(handle);
                let extent = wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                };
                // SAFETY: a texture of this device, made as described.
                let texture = unsafe {
                    let hal_texture = wgpu::hal::dx12::Device::texture_from_raw(
                        resource,
                        format,
                        wgpu::TextureDimension::D2,
                        extent,
                        1,
                        1,
                    );
                    device.create_texture_from_hal::<wgpu::hal::api::Dx12>(
                        hal_texture,
                        &wgpu::TextureDescriptor {
                            label: Some("shared scene"),
                            size: extent,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                        wgpu::TextureUses::UNINITIALIZED,
                    )
                };
                views.push(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            }
            let [c, n, z]: [wgpu::TextureView; 3] =
                views.try_into().map_err(|_| anyhow!("three planes"))?;
            self.sets.push([c, n, z]);
        }
        Ok(())
    }

    /// Draws the scene targets (colour, normal and depth texture ids in
    /// the mirror) into the next shared set and waits for the GPU.
    ///
    /// # Errors
    /// When the textures cannot be made or the GPU fails.
    pub fn write(&mut self, r: &Renderer, color: Id, normal: Id, depth: Id) -> Result<Shared> {
        let get = |id: Id| {
            r.textures
                .get(&id)
                .ok_or_else(|| anyhow!("no texture {id:#x}"))
        };
        let (c, n, z) = (get(color)?, get(normal)?, get(depth)?);
        let size = (c.desc[0], c.desc[1]);
        if size != self.size {
            self.make(&r.device, size)?;
        }
        let set = self.next;
        self.next = (self.next + 1) % SETS;
        let view = |t: &wgpu::Texture, aspect| {
            t.create_view(&wgpu::TextureViewDescriptor {
                aspect,
                ..Default::default()
            })
        };
        let cv = view(&c.texture, wgpu::TextureAspect::All);
        let nv = view(&n.texture, wgpu::TextureAspect::All);
        let zv = view(&z.texture, wgpu::TextureAspect::DepthOnly);
        let group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&cv),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&nv),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&zv),
                },
            ],
        });
        let targets = &self.sets[set as usize];
        let mut enc = r
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("share"),
            });
        {
            let att = |v| {
                Some(wgpu::RenderPassColorAttachment {
                    view: v,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })
            };
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("share"),
                color_attachments: &[att(&targets[0]), att(&targets[1]), att(&targets[2])],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &group, &[]);
            rp.draw(0..3, 0..1);
        }
        r.queue.submit([enc.finish()]);
        // The game reads the textures once told: they must be written.
        r.device.poll(wgpu::PollType::wait_indefinitely())?;
        Ok(Shared {
            width: size.0,
            height: size.1,
            generation: self.generation,
            set,
        })
    }
}

/// A committed texture on `device`, shareable under `name`.
fn create_shared(
    device: &windows::Win32::Graphics::Direct3D12::ID3D12Device,
    size: (u32, u32),
    format: DXGI_FORMAT,
    name: &str,
) -> Result<(ID3D12Resource, HANDLE)> {
    let heap = D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        ..Default::default()
    };
    let desc = D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
        Alignment: 0,
        Width: u64::from(size.0),
        Height: size.1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: format,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
        Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
            | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
    };
    let mut resource: Option<ID3D12Resource> = None;
    let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
    // SAFETY: plain Direct3D 12 calls with valid descriptions.
    unsafe {
        device.CreateCommittedResource(
            &raw const heap,
            D3D12_HEAP_FLAG_SHARED,
            &raw const desc,
            D3D12_RESOURCE_STATE_COMMON,
            None,
            &raw mut resource,
        )?;
        let resource = resource.ok_or_else(|| anyhow!("no resource"))?;
        let handle =
            device.CreateSharedHandle(&resource, None, GENERIC_ALL.0, PCWSTR(wide.as_ptr()))?;
        Ok((resource, handle))
    }
}
