//! AMD `FidelityFX` Super Resolution 3 (the upscaler) through AMD's
//! `FidelityFX` API: `amd_fidelityfx_loader_dx12.dll` and
//! `amd_fidelityfx_upscaler_dx12.dll` from the `FidelityFX` SDK (MIT
//! licence, `LICENSE-FidelityFX.txt`), copied next to the renderer by
//! `tools/build-renderer.sh`. The structures below follow the SDK's C
//! headers (`ffx_api.h`, `ffx_api_types.h`, `ffx_upscale.h`,
//! `dx12/ffx_api_dx12.h`, v2.0.0).
//!
//! FSR records into a Direct3D 12 command list of its own, executed on
//! wgpu's queue between two wgpu submissions; wgpu is first told to put
//! the textures in the states the dispatch names (shader read for the
//! inputs, unordered access for the output).

#![allow(non_camel_case_types, non_snake_case, clippy::struct_field_names)] // the C API's names, for comparison

use std::ffi::c_void;

use anyhow::{Context, Result, anyhow, bail};
use windows::Win32::Graphics::Direct3D12::{
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_FENCE_FLAG_NONE, ID3D12CommandAllocator,
    ID3D12CommandList, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList,
};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::{Interface, PCSTR, PCWSTR};

type ffxContext = *mut c_void;
type ReturnCode = u32;

#[repr(C)]
struct Header {
    kind: u64,
    next: *mut Header,
}

#[repr(C)]
struct Dimensions {
    width: u32,
    height: u32,
}

#[repr(C)]
struct Coords {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ResourceDescription {
    kind: u32,
    format: u32,
    width: u32,
    height: u32,
    depth: u32,
    mip_count: u32,
    flags: u32,
    usage: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Resource {
    resource: *mut c_void,
    description: ResourceDescription,
    state: u32,
}

const CREATE_CONTEXT_UPSCALE: u64 = 0x0001_0000;
const DISPATCH_UPSCALE: u64 = 0x0001_0001;
const QUERY_JITTER_PHASE_COUNT: u64 = 0x0001_0004;
const QUERY_JITTER_OFFSET: u64 = 0x0001_0005;
const CREATE_BACKEND_DX12: u64 = 2;

/// `FfxApiCreateContextUpscaleFlags`: `FFX_UPSCALE_ENABLE_DEBUG_CHECKING`
/// is left off; colour is display-referred (`NON_LINEAR_COLORSPACE`).
const ENABLE_NON_LINEAR_COLORSPACE: u32 = 1 << 8;
/// `FfxApiDispatchFsrUpscaleFlags`.
const FLAG_NON_LINEAR_COLOR_SRGB: u32 = 1 << 1;

const TYPE_TEXTURE2D: u32 = 2;
const FORMAT_R8G8B8A8_UNORM: u32 = 10;
const FORMAT_R16G16_FLOAT: u32 = 18;
const FORMAT_R32_FLOAT: u32 = 28;
const USAGE_READ_ONLY: u32 = 0;
const USAGE_UAV: u32 = 1 << 1;
const USAGE_DEPTHTARGET: u32 = 1 << 2;
const STATE_UNORDERED_ACCESS: u32 = 1 << 1;
const STATE_PIXEL_COMPUTE_READ: u32 = (1 << 2) | (1 << 3);

#[repr(C)]
struct CreateContextUpscale {
    header: Header,
    flags: u32,
    max_render_size: Dimensions,
    max_upscale_size: Dimensions,
    message: Option<unsafe extern "C" fn(u32, *const u16)>,
}

#[repr(C)]
struct CreateBackendDx12 {
    header: Header,
    device: *mut c_void,
}

#[repr(C)]
struct DispatchUpscale {
    header: Header,
    command_list: *mut c_void,
    color: Resource,
    depth: Resource,
    motion_vectors: Resource,
    exposure: Resource,
    reactive: Resource,
    transparency_and_composition: Resource,
    output: Resource,
    jitter_offset: Coords,
    motion_vector_scale: Coords,
    render_size: Dimensions,
    upscale_size: Dimensions,
    enable_sharpening: bool,
    sharpness: f32,
    frame_time_delta: f32,
    pre_exposure: f32,
    reset: bool,
    camera_near: f32,
    camera_far: f32,
    camera_fov_angle_vertical: f32,
    view_space_to_meters_factor: f32,
    flags: u32,
}

#[repr(C)]
struct QueryJitterPhaseCount {
    header: Header,
    render_width: u32,
    display_width: u32,
    out_phase_count: *mut i32,
}

#[repr(C)]
struct QueryJitterOffset {
    header: Header,
    index: i32,
    phase_count: i32,
    out_x: *mut f32,
    out_y: *mut f32,
}

/// What `GetProcAddress` returns.
type Proc = unsafe extern "system" fn() -> isize;
type CreateContext =
    unsafe extern "C" fn(*mut ffxContext, *mut Header, *const c_void) -> ReturnCode;
type DestroyContext = unsafe extern "C" fn(*mut ffxContext, *const c_void) -> ReturnCode;
type Query = unsafe extern "C" fn(*mut ffxContext, *mut Header) -> ReturnCode;
type Dispatch = unsafe extern "C" fn(*mut ffxContext, *const Header) -> ReturnCode;

struct Api {
    create: CreateContext,
    destroy: DestroyContext,
    query: Query,
    dispatch: Dispatch,
}

/// FSR's messages, for the renderer's log.
static MESSAGES: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

unsafe extern "C" fn message(kind: u32, text: *const u16) {
    if text.is_null() {
        return;
    }
    // SAFETY: a NUL-terminated wide string from the runtime.
    let s = unsafe {
        let mut n = 0;
        while *text.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text, n))
    };
    if let Ok(mut m) = MESSAGES.lock()
        && m.len() < 32
    {
        m.push(format!(
            "{} {s}",
            if kind == 0 { "error" } else { "warning" }
        ));
    }
}

/// What FSR said since the last call.
#[must_use]
pub fn messages() -> Vec<String> {
    MESSAGES
        .lock()
        .map(|mut m| std::mem::take(&mut *m))
        .unwrap_or_default()
}

/// A dispatch's inputs (wgpu textures at the render size) and settings.
pub struct Inputs<'a> {
    pub color: &'a wgpu::Texture,
    pub depth: &'a wgpu::Texture,
    /// Motion to last frame, in UV units (`Rg16Float`).
    pub motion: &'a wgpu::Texture,
    pub render: (u32, u32),
    pub display: (u32, u32),
    /// The jitter applied, in render pixels.
    pub jitter: (f32, f32),
    pub frame_ms: f32,
    pub reset: bool,
    pub near: f32,
    pub far: f32,
    pub fov_y: f32,
    pub sharpness: f32,
}

struct UpscaleContext {
    raw: ffxContext,
    display: (u32, u32),
}

pub struct Fsr {
    api: Api,
    device: ID3D12Device,
    context: Option<UpscaleContext>,
    output: Option<((u32, u32), wgpu::Texture, wgpu::TextureView)>,
    /// Command lists in turn, each free once the fence passes its value.
    lists: Vec<(ID3D12CommandAllocator, ID3D12GraphicsCommandList, u64)>,
    fence: ID3D12Fence,
    value: u64,
    next: usize,
}

impl Drop for Fsr {
    fn drop(&mut self) {
        if let Some(mut c) = self.context.take() {
            // SAFETY: our context, made by the same API.
            unsafe { (self.api.destroy)(&raw mut c.raw, std::ptr::null()) };
        }
    }
}

fn load(dir: &std::path::Path) -> Result<Api> {
    let path = dir.join("amd_fidelityfx_loader_dx12.dll");
    if !path.is_file() {
        bail!("no {}", path.display());
    }
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain([0])
        .collect();
    // SAFETY: loading AMD's DLL and looking up its documented exports, with
    // the signatures of `ffx_api.h`.
    unsafe {
        let module = LoadLibraryW(PCWSTR(wide.as_ptr())).context("loading the FSR DLL")?;
        let get = |name: &[u8]| {
            GetProcAddress(module, PCSTR(name.as_ptr())).ok_or_else(|| anyhow!("no export"))
        };
        Ok(Api {
            create: std::mem::transmute::<Proc, CreateContext>(get(b"ffxCreateContext\0")?),
            destroy: std::mem::transmute::<Proc, DestroyContext>(get(b"ffxDestroyContext\0")?),
            query: std::mem::transmute::<Proc, Query>(get(b"ffxQuery\0")?),
            dispatch: std::mem::transmute::<Proc, Dispatch>(get(b"ffxDispatch\0")?),
        })
    }
}

fn texture2d(t: &wgpu::Texture, format: u32, usage: u32, state: u32) -> Option<Resource> {
    // SAFETY: the texture's Direct3D 12 resource, used while it lives.
    let raw = unsafe {
        t.as_hal::<wgpu::hal::api::Dx12>()
            .map(|h| h.raw_resource().as_raw())
    }?;
    Some(Resource {
        resource: raw,
        description: ResourceDescription {
            kind: TYPE_TEXTURE2D,
            format,
            width: t.width(),
            height: t.height(),
            depth: 1,
            mip_count: 1,
            flags: 0,
            usage,
        },
        state,
    })
}

impl Fsr {
    /// FSR on `device` (Direct3D 12), from the DLLs in `dir`.
    ///
    /// # Errors
    /// When the DLLs are missing or the device is not Direct3D 12.
    pub fn new(device: &wgpu::Device, dir: &std::path::Path) -> Result<Self> {
        // SAFETY: wgpu's Direct3D 12 device, kept alive by wgpu.
        let raw = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
            .map(|h| h.raw_device().clone())
            .ok_or_else(|| anyhow!("FSR needs Direct3D 12"))?;
        let api = load(dir)?;
        // SAFETY: plain Direct3D 12 calls.
        let fence = unsafe { raw.CreateFence(0, D3D12_FENCE_FLAG_NONE)? };
        Ok(Self {
            api,
            device: raw,
            context: None,
            output: None,
            lists: Vec::new(),
            fence,
            value: 0,
            next: 0,
        })
    }

    /// The camera jitter for `frame` (render pixels), FSR's sequence for
    /// this ratio.
    #[must_use]
    pub fn jitter(&self, render: (u32, u32), display: (u32, u32), frame: u32) -> (f32, f32) {
        let mut phases = 0i32;
        let mut q = QueryJitterPhaseCount {
            header: Header {
                kind: QUERY_JITTER_PHASE_COUNT,
                next: std::ptr::null_mut(),
            },
            render_width: render.0,
            display_width: display.0,
            out_phase_count: &raw mut phases,
        };
        // Without a context yet the query is a global one (a null context).
        let mut raw = self
            .context
            .as_ref()
            .map_or(std::ptr::null_mut(), |c| c.raw);
        let ctx: *mut ffxContext = if raw.is_null() {
            std::ptr::null_mut()
        } else {
            &raw mut raw
        };
        // SAFETY: a query with live out pointers.
        unsafe { (self.api.query)(ctx, (&raw mut q).cast()) };
        let phases = phases.max(1);
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut o = QueryJitterOffset {
            header: Header {
                kind: QUERY_JITTER_OFFSET,
                next: std::ptr::null_mut(),
            },
            index: (frame % phases.cast_unsigned()).cast_signed(),
            phase_count: phases,
            out_x: &raw mut x,
            out_y: &raw mut y,
        };
        // SAFETY: as above.
        unsafe { (self.api.query)(ctx, (&raw mut o).cast()) };
        (x, y)
    }

    fn context(&mut self, display: (u32, u32)) -> Result<()> {
        if self.context.as_ref().is_some_and(|c| c.display == display) {
            return Ok(());
        }
        if let Some(mut c) = self.context.take() {
            self.wait_idle();
            // SAFETY: our context.
            unsafe { (self.api.destroy)(&raw mut c.raw, std::ptr::null()) };
        }
        let mut backend = CreateBackendDx12 {
            header: Header {
                kind: CREATE_BACKEND_DX12,
                next: std::ptr::null_mut(),
            },
            device: self.device.as_raw(),
        };
        let mut desc = CreateContextUpscale {
            header: Header {
                kind: CREATE_CONTEXT_UPSCALE,
                next: (&raw mut backend).cast(),
            },
            flags: ENABLE_NON_LINEAR_COLORSPACE,
            // Any render size up to the display's.
            max_render_size: Dimensions {
                width: display.0,
                height: display.1,
            },
            max_upscale_size: Dimensions {
                width: display.0,
                height: display.1,
            },
            message: Some(message),
        };
        let mut raw: ffxContext = std::ptr::null_mut();
        // SAFETY: the descriptions live for the call; the backend keeps the
        // device, which outlives the context (dropped first).
        let rc =
            unsafe { (self.api.create)(&raw mut raw, (&raw mut desc).cast(), std::ptr::null()) };
        if rc != 0 || raw.is_null() {
            bail!("FSR context: error {rc}");
        }
        self.context = Some(UpscaleContext { raw, display });
        Ok(())
    }

    fn wait_idle(&self) {
        // SAFETY: our fence.
        while unsafe { self.fence.GetCompletedValue() } < self.value {
            std::thread::yield_now();
        }
    }

    /// The output (display size, storage), made when needed.
    fn output(
        &mut self,
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> (wgpu::Texture, wgpu::TextureView) {
        if self.output.as_ref().is_none_or(|o| o.0 != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("fsr output"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.output = Some((size, texture, view));
        }
        let o = self.output.as_ref().expect("made");
        (o.1.clone(), o.2.clone())
    }

    /// Puts the inputs and the output in the states the dispatch names, at
    /// the end of `enc` (submit it before [`Fsr::dispatch`]).
    pub fn prepare(&mut self, device: &wgpu::Device, enc: &mut wgpu::CommandEncoder, i: &Inputs) {
        let (output, _) = self.output(device, i.display);
        let read = wgpu::TextureUses::RESOURCE;
        enc.transition_resources(
            std::iter::empty(),
            [
                wgpu::TextureTransition {
                    texture: i.color,
                    selector: None,
                    state: read,
                },
                wgpu::TextureTransition {
                    texture: i.depth,
                    selector: None,
                    state: read,
                },
                wgpu::TextureTransition {
                    texture: i.motion,
                    selector: None,
                    state: read,
                },
                wgpu::TextureTransition {
                    texture: &output,
                    selector: None,
                    state: wgpu::TextureUses::STORAGE_READ_WRITE,
                },
            ]
            .into_iter(),
        );
    }

    /// Upscales into the output (after `prepare`'s encoder was submitted);
    /// returns its view.
    ///
    /// # Errors
    /// When FSR or Direct3D 12 fail.
    #[allow(clippy::too_many_lines)] // the description, a free command list, the submission
    pub fn dispatch(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        i: &Inputs,
    ) -> Result<wgpu::TextureView> {
        self.context(i.display)?;
        let (output, view) = self.output(device, i.display);
        let (Some(color), Some(depth), Some(motion), Some(out)) = (
            texture2d(
                i.color,
                FORMAT_R8G8B8A8_UNORM,
                USAGE_READ_ONLY,
                STATE_PIXEL_COMPUTE_READ,
            ),
            texture2d(
                i.depth,
                FORMAT_R32_FLOAT,
                USAGE_DEPTHTARGET,
                STATE_PIXEL_COMPUTE_READ,
            ),
            texture2d(
                i.motion,
                FORMAT_R16G16_FLOAT,
                USAGE_READ_ONLY,
                STATE_PIXEL_COMPUTE_READ,
            ),
            texture2d(
                &output,
                FORMAT_R8G8B8A8_UNORM,
                USAGE_UAV,
                STATE_UNORDERED_ACCESS,
            ),
        ) else {
            bail!("not Direct3D 12 textures");
        };
        // A command list whose last use the GPU has finished.
        if self.lists.len() < 3 {
            // SAFETY: plain Direct3D 12 calls.
            let (allocator, list) = unsafe {
                let a: ID3D12CommandAllocator = self
                    .device
                    .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?;
                let l: ID3D12GraphicsCommandList =
                    self.device
                        .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &a, None)?;
                l.Close()?;
                (a, l)
            };
            self.lists.push((allocator, list, 0));
        }
        let k = self.next % self.lists.len();
        self.next += 1;
        let (allocator, list, used) = &mut self.lists[k];
        // SAFETY: our fence.
        while unsafe { self.fence.GetCompletedValue() } < *used {
            std::thread::yield_now();
        }
        let none = Resource {
            resource: std::ptr::null_mut(),
            description: color.description,
            state: STATE_PIXEL_COMPUTE_READ,
        };
        let desc = DispatchUpscale {
            header: Header {
                kind: DISPATCH_UPSCALE,
                next: std::ptr::null_mut(),
            },
            command_list: list.as_raw(),
            color,
            depth,
            motion_vectors: motion,
            exposure: none,
            reactive: none,
            transparency_and_composition: none,
            output: out,
            jitter_offset: Coords {
                x: i.jitter.0,
                y: i.jitter.1,
            },
            // UV units to render pixels.
            motion_vector_scale: Coords {
                x: i.render.0 as f32,
                y: i.render.1 as f32,
            },
            render_size: Dimensions {
                width: i.render.0,
                height: i.render.1,
            },
            upscale_size: Dimensions {
                width: i.display.0,
                height: i.display.1,
            },
            enable_sharpening: i.sharpness > 0.0,
            sharpness: i.sharpness.clamp(0.0, 1.0),
            frame_time_delta: i.frame_ms.clamp(1.0, 100.0),
            pre_exposure: 1.0,
            reset: i.reset,
            camera_near: i.near,
            camera_far: i.far,
            camera_fov_angle_vertical: i.fov_y,
            // A character is about 20 units tall: 10 units a metre.
            view_space_to_meters_factor: 0.1,
            flags: FLAG_NON_LINEAR_COLOR_SRGB,
        };
        let mut ctx = self.context.as_ref().expect("made").raw;
        // SAFETY: our allocator and list (the GPU is done with them), the
        // description's resources live; the list then runs on wgpu's queue
        // after the submissions that wrote the inputs.
        unsafe {
            allocator.Reset()?;
            list.Reset(&*allocator, None)?;
            let rc = (self.api.dispatch)(&raw mut ctx, (&raw const desc).cast());
            list.Close()?;
            if rc != 0 {
                bail!("FSR dispatch: error {rc}");
            }
            let q = queue
                .as_hal::<wgpu::hal::api::Dx12>()
                .ok_or_else(|| anyhow!("not a Direct3D 12 queue"))?;
            let lists = [Some(list.cast::<ID3D12CommandList>()?)];
            q.as_raw().ExecuteCommandLists(&lists);
            self.value += 1;
            q.as_raw().Signal(&self.fence, self.value)?;
        }
        *used = self.value;
        Ok(view)
    }
}
