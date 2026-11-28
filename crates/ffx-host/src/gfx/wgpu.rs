//! The portable back end of [`crate::gfx`]: the game's Direct3D 11 calls
//! drawn with wgpu (Vulkan, Metal, Direct3D 12), the game's own shaders
//! translated from DXBC to WGSL by `ffx-dxbc` (docs/host.md, "The wgpu
//! back end"; docs/m6.md).
//!
//! Direct3D 11 is an immediate API: every call takes effect in order,
//! resource writes between draws included. wgpu records command buffers
//! and its queue writes land before the next submission's commands. So
//! the back end records what the calls describe (`Cmd`) and turns the
//! list into one command buffer at `Present`, a read-back or `Flush`
//! (`Wgpu::flush`):
//!
//! - constant buffers live on the CPU (`Buf::shadow`); a draw copies the
//!   windows it binds into the frame's uniform arena and binds them at
//!   their offsets (dynamic offsets, so bind groups are cached);
//! - dynamic vertex and index buffers (`D3D11_USAGE_DYNAMIC`) likewise:
//!   a draw copies the bytes it reads into the frame's geometry arena;
//! - writes to other resources go straight to the queue when no recorded
//!   command uses the resource yet, else through the upload arena as a
//!   copy in the command list, in their place;
//! - the state objects become pipeline state; pipelines are cached by the
//!   interned descriptions, the formats and the vertex layout.
//!
//! What wgpu can't state is emulated, as in the proof of concept
//! (docs/m6.md): alpha to coverage on a single-sampled target is a
//! discard below 0.5 in the pixel shader; a depth texture read as a float
//! texture is read from a 32-bit float copy (`DepthToFloat`); a shader
//! resource that is also a target of the draw reads as unbound, as the
//! Direct3D 11 runtime unbinds it.
//!
//! The device draws into an offscreen back buffer of the game's size and
//! format; `Present` copies it onto the window's surface (a handle from
//! the platform: [`crate::platform::GfxWindow`]) when there is one. Without
//! a window it renders offscreen only (headless runs, tests, parity
//! captures on Windows).

// GPU sizes, offsets and coordinates move between u32, i32, u64, f32 and
// usize all the time; the values are small. One arm per call or object
// kind.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::struct_excessive_bools
)]

mod formats;

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use ::wgpu as w;
use ffx_dxbc::Container;
use ffx_dxbc::sm4::Dim;
use ffx_dxbc::wgsl::{self, TextureKind};
use ffx_gfx::{DrawKind, Element, StateKind, Subresource};

use crate::com::{E_FAIL, E_INVALIDARG, S_FALSE, S_OK};
use crate::com_methods::class;
use crate::gfx::{Backend, Create, Op, SwapChainDesc};
use crate::platform::GfxWindow;

/// Vertex buffer slots the back end follows (`D3D11_IA_VERTEX_INPUT_RESOURCE_SLOT_COUNT`
/// is 32; the game binds a few).
const VERTEX_SLOTS: usize = 16;
/// Shader resource and sampler slots per stage the back end follows.
const SRV_SLOTS: usize = 32;
const SAMPLER_SLOTS: usize = 16;
/// Constant buffer slots per stage (`D3D11_COMMONSHADER_CONSTANT_BUFFER_API_SLOT_COUNT`).
const CB_SLOTS: usize = 14;
/// The most frames the CPU runs ahead of the GPU without a window.
const FRAMES_AHEAD: usize = 2;
/// Bind groups kept before the cache starts over.
const BIND_GROUP_CACHE: usize = 8192;

/// A window handle for wgpu, from the platform's.
struct Handle(Arc<dyn GfxWindow>);

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("platform window")
    }
}

impl raw_window_handle::HasWindowHandle for Handle {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        self.0.window_handle()
    }
}

impl raw_window_handle::HasDisplayHandle for Handle {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        self.0.display_handle()
    }
}

/// A texture's description, whatever its dimensions.
#[derive(Debug, Clone, Copy)]
struct TexDesc {
    dims: u8,
    width: u32,
    height: u32,
    /// Array layers (1D, 2D) or depth (3D).
    layers: u32,
    mips: u32,
    format: u32,
    samples: u32,
    bind: u32,
    misc: u32,
}

impl TexDesc {
    fn new(dims: u8, d: &[u32]) -> Option<Self> {
        let g = |i: usize| d.get(i).copied().unwrap_or(0);
        let mut t = match dims {
            1 => Self {
                dims,
                width: g(0),
                height: 1,
                mips: g(1),
                layers: g(2),
                format: g(3),
                samples: 1,
                bind: g(5),
                misc: g(7),
            },
            2 => Self {
                dims,
                width: g(0),
                height: g(1),
                mips: g(2),
                layers: g(3),
                format: g(4),
                samples: g(5),
                bind: g(8),
                misc: g(10),
            },
            3 => Self {
                dims,
                width: g(0),
                height: g(1),
                layers: g(2),
                mips: g(3),
                format: g(4),
                samples: 1,
                bind: g(6),
                misc: g(8),
            },
            _ => return None,
        };
        t.width = t.width.max(1);
        t.height = t.height.max(1);
        t.layers = t.layers.max(1);
        t.samples = t.samples.max(1);
        if t.mips == 0 {
            // a full chain
            let big = t
                .width
                .max(t.height)
                .max(if dims == 3 { t.layers } else { 1 });
            t.mips = 32 - big.leading_zeros();
        }
        Some(t)
    }

    fn cube(&self) -> bool {
        self.misc & 0x4 != 0
    }

    fn mip_size(&self, mip: u32) -> (u32, u32) {
        ((self.width >> mip).max(1), (self.height >> mip).max(1))
    }
}

/// A buffer.
struct Buf {
    /// The GPU's (none for a constant buffer: its windows go into the
    /// uniform arena).
    buffer: Option<w::Buffer>,
    desc: [u32; 6],
    /// The contents as the game wrote them (constant, dynamic, index,
    /// staging and default buffers; immutable vertex data has none).
    shadow: Option<Vec<u8>>,
    /// Bumped by every write (the uniform arena's reuse key).
    version: u64,
    /// The flush that last recorded a command reading or writing it.
    epoch: u64,
}

impl Buf {
    fn dynamic(&self) -> bool {
        self.desc[1] == 2
    }
}

/// A texture.
struct Tex {
    texture: w::Texture,
    desc: TexDesc,
    format: w::TextureFormat,
    serial: u64,
    epoch: u64,
}

/// A view.
#[derive(Clone)]
struct ViewObj {
    class: usize,
    view: Arc<w::TextureView>,
    serial: u64,
    texture: w::Texture,
    tex_serial: u64,
    /// The view's format.
    format: w::TextureFormat,
    samples: u32,
    /// The size of the viewed mip.
    size: (u32, u32),
    dimension: w::TextureViewDimension,
    /// A depth-stencil texture's depth aspect.
    depth: bool,
    /// A depth-stencil texture's stencil aspect.
    stencil: bool,
    base_mip: u32,
}

struct ShaderObj {
    bytecode: Arc<[u8]>,
    hash: u64,
}

enum Obj {
    Buf(Buf),
    Tex(Tex),
    View(ViewObj),
    Shader(ShaderObj),
    Layout(Vec<Element>, u32),
    /// A state: kind, description, interned key.
    State(StateKind, Vec<u8>, u32),
    Query,
    /// An object the back end has no use for (an unordered access view, a
    /// buffer's shader resource view).
    Other,
}

/// What is bound on the immediate context.
#[derive(Clone)]
struct Bound {
    layout: u32,
    topology: u32,
    vertex: [(u32, u32, u32); VERTEX_SLOTS],
    index: (u32, u32, u32),
    vs: u32,
    ps: u32,
    gs: u32,
    /// Per stage (vertex, pixel): buffer, first constant, constants (0:
    /// the whole buffer).
    cb: [[(u32, u32, u32); CB_SLOTS]; 2],
    srv: [[u32; SRV_SLOTS]; 2],
    sampler: [[u32; SAMPLER_SLOTS]; 2],
    rtv: [u32; 8],
    dsv: u32,
    blend: u32,
    blend_factor: [f32; 4],
    sample_mask: u32,
    depth: u32,
    stencil_ref: u32,
    raster: u32,
    viewport: [f32; 6],
    scissor: [i32; 4],
}

impl Default for Bound {
    fn default() -> Self {
        Self {
            layout: 0,
            topology: 0,
            vertex: [(0, 0, 0); VERTEX_SLOTS],
            index: (0, 0, 0),
            vs: 0,
            ps: 0,
            gs: 0,
            cb: [[(0, 0, 0); CB_SLOTS]; 2],
            srv: [[0; SRV_SLOTS]; 2],
            sampler: [[0; SAMPLER_SLOTS]; 2],
            rtv: [0; 8],
            dsv: 0,
            blend: 0,
            blend_factor: [1.0; 4],
            sample_mask: u32::MAX,
            depth: 0,
            stencil_ref: 0,
            raster: 0,
            viewport: [0.0; 6],
            scissor: [0; 4],
        }
    }
}

/// A vertex and pixel shader pair, linked and translated.
struct Linked {
    serial: u64,
    vs: wgsl::Shader,
    ps: Option<wgsl::Shader>,
    vs_module: w::ShaderModule,
    ps_module: Option<w::ShaderModule>,
    /// The pixel shader with alpha to coverage as a test (made on first
    /// use).
    alpha_test: Mutex<Option<w::ShaderModule>>,
    layout: w::PipelineLayout,
    groups: [w::BindGroupLayout; 2],
    /// Whether the constant buffers are bound at dynamic offsets (bind
    /// groups cached) or each draw gets its own groups.
    dynamic: bool,
    /// The vertex shader reads `SV_VertexID` (its vertices can't be
    /// renumbered).
    vertex_id: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PipelineKey {
    linked: u64,
    layout: u32,
    strides: [u32; VERTEX_SLOTS],
    topology: u32,
    strip_index: Option<u32>,
    blend: u32,
    depth: u32,
    raster: u32,
    sample_mask: u32,
    colors: Vec<Option<w::TextureFormat>>,
    depth_format: Option<w::TextureFormat>,
    samples: u32,
}

/// Where a draw reads vertices or indices.
#[derive(Clone)]
enum Src {
    /// The frame's geometry arena, at this offset.
    Arena(u64),
    /// A buffer of the game's, at this offset.
    Buffer(w::Buffer, u64),
}

/// The views, samplers and constant windows of one bind group.
struct BindSpec {
    views: Vec<(u32, Arc<w::TextureView>, u64)>,
    samplers: Vec<(u32, Arc<w::Sampler>, u64)>,
    /// (slot, offset into the uniform arena, size).
    uniforms: Vec<(u32, u64, u64)>,
}

/// A draw's attachments.
#[derive(Clone)]
struct Targets {
    colors: Vec<Option<(Arc<w::TextureView>, u64)>>,
    depth: Option<(Arc<w::TextureView>, u64, bool)>,
}

impl Targets {
    fn key(&self) -> (Vec<u64>, u64) {
        (
            self.colors
                .iter()
                .map(|c| c.as_ref().map_or(0, |c| c.1))
                .collect(),
            self.depth.as_ref().map_or(0, |d| d.1),
        )
    }
}

/// The draw call itself.
#[derive(Clone)]
struct Call {
    indexed: Option<(Src, w::IndexFormat)>,
    range: std::ops::Range<u32>,
    base: i32,
    instances: std::ops::Range<u32>,
}

struct DrawCmd {
    pipeline: Arc<w::RenderPipeline>,
    linked: Arc<Linked>,
    targets: Targets,
    binds: [BindSpec; 2],
    vertex: Vec<(u32, Src)>,
    call: Call,
    viewport: [f32; 6],
    scissor: [u32; 4],
    blend_constant: [f32; 4],
    stencil_ref: u32,
}

/// A copy's end: a texture subresource and where in it.
struct TexAt {
    texture: w::Texture,
    mip: u32,
    origin: w::Origin3d,
    aspect: w::TextureAspect,
}

/// What the calls describe, in order.
enum Cmd {
    Draw(Box<DrawCmd>),
    ClearColor {
        view: Arc<w::TextureView>,
        color: [f32; 4],
    },
    ClearDepth {
        view: Arc<w::TextureView>,
        depth: Option<f32>,
        stencil: Option<u32>,
        has_depth: bool,
        has_stencil: bool,
    },
    UploadBuffer {
        at: u64,
        dst: w::Buffer,
        offset: u64,
        size: u64,
    },
    UploadTexture {
        at: u64,
        bytes_per_row: u32,
        rows: u32,
        dst: TexAt,
        extent: w::Extent3d,
    },
    CopyBuffer {
        src: w::Buffer,
        src_offset: u64,
        dst: w::Buffer,
        dst_offset: u64,
        size: u64,
    },
    CopyTexture {
        src: TexAt,
        dst: TexAt,
        extent: w::Extent3d,
    },
    Resolve {
        src: Arc<w::TextureView>,
        dst: Arc<w::TextureView>,
    },
    DepthToFloat {
        src: Arc<w::TextureView>,
        dst: Arc<w::TextureView>,
        multisampled: bool,
    },
}

/// A persistent arena buffer, written in full before each submission.
struct Arena {
    buffer: Option<w::Buffer>,
    serial: u64,
    usage: w::BufferUsages,
    data: Vec<u8>,
}

impl Arena {
    fn new(usage: w::BufferUsages) -> Self {
        Self {
            buffer: None,
            serial: 0,
            usage: usage | w::BufferUsages::COPY_DST,
            data: Vec::new(),
        }
    }

    /// Appends `bytes` at the next multiple of `align` (padded with zeros
    /// to `size`), returning the offset.
    fn put(&mut self, bytes: &[u8], align: usize, size: usize) -> u64 {
        let at = self.data.len().next_multiple_of(align.max(1));
        self.data.resize(at, 0);
        self.data
            .extend_from_slice(&bytes[..bytes.len().min(size.max(bytes.len()))]);
        self.data.resize(at + size.max(bytes.len()), 0);
        at as u64
    }

    /// Makes the buffer big enough and writes the data; `serials` hands
    /// out a new serial when the buffer is replaced.
    fn upload(&mut self, device: &w::Device, queue: &w::Queue, serials: &mut u64) {
        if self.data.is_empty() {
            return;
        }
        self.data.resize(self.data.len().next_multiple_of(4), 0);
        let need = self.data.len() as u64;
        if self.buffer.as_ref().is_none_or(|b| b.size() < need) {
            let size = need.next_power_of_two().max(1 << 16);
            self.buffer = Some(device.create_buffer(&w::BufferDescriptor {
                label: Some("ffx arena"),
                size,
                usage: self.usage,
                mapped_at_creation: false,
            }));
            *serials += 1;
            self.serial = *serials;
        }
        if let Some(b) = &self.buffer {
            queue.write_buffer(b, 0, &self.data);
        }
        self.data.clear();
    }
}

/// The device and what belongs to it.
struct Dev {
    adapter: w::Adapter,
    device: w::Device,
    queue: w::Queue,
    surface: Option<w::Surface<'static>>,
    config: Option<w::SurfaceConfiguration>,
    blit: Option<(
        w::TextureFormat,
        w::RenderPipeline,
        w::BindGroupLayout,
        w::Sampler,
    )>,
    info: String,
}

/// The wgpu back end.
pub struct Wgpu {
    window: Option<Arc<dyn GfxWindow>>,
    dev: Option<Dev>,
    objects: HashMap<u32, Obj>,
    bound: Bound,
    cmds: Vec<Cmd>,
    uniform: Arena,
    geometry: Arena,
    upload: Arena,
    /// Uniform windows already in this flush's arena: (buffer, version,
    /// first, count, size) → offset.
    uniform_memo: HashMap<(u32, u64, u32, u32, u64), u64>,
    epoch: u64,
    serials: u64,
    /// Interned state and layout descriptions → key (from 1).
    intern: HashMap<(u8, Vec<u8>), u32>,
    linked: HashMap<(u64, u64), Option<Arc<Linked>>>,
    pipelines: HashMap<PipelineKey, Option<Arc<w::RenderPipeline>>>,
    samplers: HashMap<(u32, bool), (Arc<w::Sampler>, u64)>,
    dummies: HashMap<(u8, u8), (Arc<w::TextureView>, u64)>,
    bind_groups: HashMap<BindKey, Arc<w::BindGroup>>,
    /// Float copies of depth textures read as float textures: texture
    /// serial → (copy, view, serial, stale).
    depth_copies: HashMap<u64, (w::Texture, Arc<w::TextureView>, u64, bool)>,
    depth_to_float: HashMap<bool, (w::RenderPipeline, w::BindGroupLayout)>,
    back_buffer: u32,
    back: Option<w::Texture>,
    /// The game's swap chain description.
    swap_desc: [u32; 15],
    sync: u32,
    queries: HashMap<u32, u32>,
    clock: u64,
    in_flight: VecDeque<w::SubmissionIndex>,
    /// Problems, counted (for the report: [`Wgpu::problems`]).
    failures: BTreeMap<String, usize>,
    errors: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BindKey {
    linked: u64,
    group: u8,
    views: Vec<u64>,
    samplers: Vec<u64>,
    arena: u64,
}

impl Wgpu {
    /// A back end that draws onto `window` (a platform's), or offscreen
    /// with none. The device is made at the game's
    /// `D3D11CreateDevice(AndSwapChain)`.
    #[must_use]
    pub fn new(window: Option<Arc<dyn GfxWindow>>) -> Self {
        Self {
            window,
            dev: None,
            objects: HashMap::new(),
            bound: Bound::default(),
            cmds: Vec::new(),
            uniform: Arena::new(w::BufferUsages::UNIFORM),
            geometry: Arena::new(w::BufferUsages::VERTEX | w::BufferUsages::INDEX),
            upload: Arena::new(w::BufferUsages::COPY_SRC),
            uniform_memo: HashMap::new(),
            epoch: 1,
            serials: 0,
            intern: HashMap::new(),
            linked: HashMap::new(),
            pipelines: HashMap::new(),
            samplers: HashMap::new(),
            dummies: HashMap::new(),
            bind_groups: HashMap::new(),
            depth_copies: HashMap::new(),
            depth_to_float: HashMap::new(),
            back_buffer: 0,
            back: None,
            swap_desc: [0; 15],
            sync: 1,
            queries: HashMap::new(),
            clock: 0,
            in_flight: VecDeque::new(),
            failures: BTreeMap::new(),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// The adapter and API in use (empty before the device).
    #[must_use]
    pub fn adapter(&self) -> &str {
        self.dev.as_ref().map_or("", |d| d.info.as_str())
    }

    /// What didn't draw as Direct3D 11 would, with counts: failed
    /// translations and pipelines, unsupported calls, GPU errors.
    #[must_use]
    pub fn problems(&self) -> Vec<(String, usize)> {
        let mut v: Vec<_> = self.failures.iter().map(|(k, n)| (k.clone(), *n)).collect();
        if let Ok(e) = self.errors.lock() {
            v.extend(e.iter().map(|e| (format!("GPU: {e}"), 1)));
        }
        v
    }

    fn fail(&mut self, what: impl Into<String>) {
        let what = what.into();
        let n = self.failures.entry(what.clone()).or_default();
        if *n == 0 {
            crate::log::note(&format!("gfx wgpu: {what}"));
        }
        *n += 1;
    }

    fn serial(&mut self) -> u64 {
        self.serials += 1;
        self.serials
    }

    fn intern(&mut self, kind: u8, desc: &[u8]) -> u32 {
        let next = self.intern.len() as u32 + 1;
        *self.intern.entry((kind, desc.to_vec())).or_insert(next)
    }

    /// Makes the device (and the surface on the platform's window).
    fn open(&mut self, desc: &SwapChainDesc) -> Result<(), String> {
        let backends = match std::env::var("FFX_HOST_WGPU_BACKEND").as_deref() {
            Ok("vulkan") => w::Backends::VULKAN,
            Ok("dx12") => w::Backends::DX12,
            Ok("metal") => w::Backends::METAL,
            Ok("gl") => w::Backends::GL,
            _ => w::Backends::PRIMARY,
        };
        let mut idesc = w::InstanceDescriptor {
            backends,
            ..w::InstanceDescriptor::new_without_display_handle()
        };
        if let Some(win) = &self.window {
            idesc = idesc.with_display_handle(Box::new(Handle(win.clone())));
        }
        let instance = w::Instance::new(idesc);
        let surface = match &self.window {
            Some(win) => {
                let (tx, rx) = std::sync::mpsc::channel();
                let (inst, handle) = (instance.clone(), win.clone());
                win.on_window_thread(Box::new(move || {
                    let s = inst.create_surface(w::SurfaceTarget::DisplayAndWindow(Box::new(
                        Handle(handle),
                    )));
                    let _ = tx.send(s.map_err(|e| e.to_string()));
                }));
                match rx.recv() {
                    Ok(Ok(s)) => Some(s),
                    Ok(Err(e)) => {
                        self.fail(format!("no surface on the window: {e}; offscreen"));
                        None
                    }
                    Err(_) => {
                        self.fail("no surface on the window (not made); offscreen");
                        None
                    }
                }
            }
            None => None,
        };
        let wanted_name = std::env::var("FFX_HOST_WGPU_ADAPTER").unwrap_or_default();
        let adapter = if wanted_name.is_empty() {
            pollster::block_on(instance.request_adapter(&w::RequestAdapterOptions {
                power_preference: w::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: surface.as_ref(),
                apply_limit_buckets: false,
            }))
            .map_err(|e| format!("no GPU adapter: {e}"))?
        } else {
            let wanted = wanted_name.to_lowercase();
            pollster::block_on(instance.enumerate_adapters(backends))
                .into_iter()
                .find(|a| a.get_info().name.to_lowercase().contains(&wanted))
                .ok_or_else(|| format!("no adapter named like {wanted_name}"))?
        };
        let ai = adapter.get_info();
        let want = w::Features::TEXTURE_COMPRESSION_BC
            | w::Features::DEPTH32FLOAT_STENCIL8
            | w::Features::FLOAT32_FILTERABLE
            | w::Features::FLOAT32_BLENDABLE
            | w::Features::DEPTH_CLIP_CONTROL
            | w::Features::POLYGON_MODE_LINE
            | w::Features::ADDRESS_MODE_CLAMP_TO_BORDER
            | w::Features::ADDRESS_MODE_CLAMP_TO_ZERO
            | w::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | w::Features::RG11B10UFLOAT_RENDERABLE
            | w::Features::TEXTURE_FORMAT_16BIT_NORM;
        let features = adapter.features() & want;
        for (f, what) in [
            (w::Features::TEXTURE_COMPRESSION_BC, "BC textures"),
            (
                w::Features::FLOAT32_FILTERABLE,
                "filtering 32-bit float textures",
            ),
        ] {
            if !features.contains(f) {
                self.fail(format!("the adapter lacks {what}"));
            }
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&w::DeviceDescriptor {
            label: Some("ffx-rust"),
            required_features: features,
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| format!("no GPU device: {e}"))?;
        let errors = self.errors.clone();
        device.on_uncaptured_error(Arc::new(move |e: w::Error| {
            if let Ok(mut v) = errors.lock()
                && v.len() < 64
            {
                let line = e.to_string();
                crate::log::note(&format!("gfx wgpu: GPU error: {line}"));
                v.push(line);
            }
        }));
        let info = format!("{} ({:?}, {})", ai.name, ai.backend, ai.driver);
        crate::log::note(&format!(
            "gfx wgpu: {info}; swap chain {}x{} format {}",
            desc.width(),
            desc.height(),
            desc.format()
        ));
        self.dev = Some(Dev {
            adapter,
            device,
            queue,
            surface,
            config: None,
            blit: None,
            info,
        });
        self.swap_desc = desc.0;
        Ok(())
    }

    fn device(&self) -> Option<&w::Device> {
        self.dev.as_ref().map(|d| &d.device)
    }

    fn format_supports(&self, format: w::TextureFormat, samples: u32) -> (bool, bool) {
        let Some(d) = &self.dev else {
            return (false, false);
        };
        let f = d.adapter.get_texture_format_features(format);
        (
            f.allowed_usages
                .contains(w::TextureUsages::RENDER_ATTACHMENT),
            samples <= 1 || f.flags.sample_count_supported(samples),
        )
    }

    // ---- objects -------------------------------------------------------

    fn make_buffer(&mut self, id: u32, desc: [u32; 6], init: Option<&[u8]>) -> u32 {
        let Some(device) = self.device().cloned() else {
            return E_FAIL;
        };
        let size = u64::from(desc[0].max(4)).next_multiple_of(4);
        let constant = desc[2] & 0x4 != 0;
        let buffer = (!constant).then(|| {
            device.create_buffer(&w::BufferDescriptor {
                label: None,
                size,
                usage: w::BufferUsages::VERTEX
                    | w::BufferUsages::INDEX
                    | w::BufferUsages::COPY_DST
                    | w::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        });
        // immutable vertex data is the one kind never written or read back
        let keep = !(desc[1] == 1 && desc[2] & 0x2 == 0 && !constant);
        let mut shadow = keep.then(|| vec![0u8; desc[0] as usize]);
        if let Some(data) = init {
            if let Some(s) = shadow.as_mut() {
                let n = data.len().min(s.len());
                s[..n].copy_from_slice(&data[..n]);
            }
            if let (Some(b), Some(q)) = (&buffer, self.dev.as_ref().map(|d| &d.queue)) {
                let mut padded = data[..data.len().min(size as usize)].to_vec();
                padded.resize(padded.len().next_multiple_of(4), 0);
                q.write_buffer(b, 0, &padded);
            }
        }
        self.objects.insert(
            id,
            Obj::Buf(Buf {
                buffer,
                desc,
                shadow,
                version: 0,
                epoch: 0,
            }),
        );
        S_OK
    }

    fn make_texture(
        &mut self,
        id: u32,
        dims: u8,
        words: &[u32],
        init: &[(u32, u32, Vec<u8>)],
    ) -> u32 {
        let Some(device) = self.device().cloned() else {
            return E_FAIL;
        };
        let Some(d) = TexDesc::new(dims, words) else {
            return E_INVALIDARG;
        };
        let depth = d.bind & 0x40 != 0;
        let format = match formats::texture_format(d.format, depth) {
            Some(f) => f,
            None => {
                self.fail(format!("texture format {} (drawn as RGBA8)", d.format));
                w::TextureFormat::Rgba8Unorm
            }
        };
        // FFX_HOST_WGPU_DEPTH32=1: 32-bit float depth for D24S8 (to measure
        // what the depth format changes)
        let format = if format == w::TextureFormat::Depth24PlusStencil8
            && std::env::var("FFX_HOST_WGPU_DEPTH32").is_ok_and(|v| v == "1")
            && device
                .features()
                .contains(w::Features::DEPTH32FLOAT_STENCIL8)
        {
            w::TextureFormat::Depth32FloatStencil8
        } else {
            format
        };
        if format.is_compressed()
            && !device
                .features()
                .contains(w::Features::TEXTURE_COMPRESSION_BC)
        {
            self.fail(format!("BC texture format {} without BC support", d.format));
        }
        let (renderable, mut samples_ok) = self.format_supports(format, d.samples);
        let mut samples = d.samples;
        if !samples_ok {
            self.fail(format!("{samples} samples of {format:?}: 4 instead"));
            samples = 4;
            samples_ok = self.format_supports(format, 4).1;
            if !samples_ok {
                samples = 1;
            }
        }
        let (_, bw) = formats::block(format);
        let size = w::Extent3d {
            width: d.width.next_multiple_of(bw),
            height: d.height.next_multiple_of(bw),
            depth_or_array_layers: if dims == 2 && d.cube() {
                d.layers.max(6)
            } else {
                d.layers
            },
        };
        let mut usage = w::TextureUsages::TEXTURE_BINDING
            | w::TextureUsages::COPY_DST
            | w::TextureUsages::COPY_SRC;
        if (renderable && !format.is_compressed()) || samples > 1 {
            usage |= w::TextureUsages::RENDER_ATTACHMENT;
        }
        let view_formats: Vec<w::TextureFormat> =
            [format.add_srgb_suffix(), format.remove_srgb_suffix()]
                .into_iter()
                .filter(|f| *f != format)
                .collect();
        let mips = if samples > 1 || dims == 1 { 1 } else { d.mips };
        let texture = device.create_texture(&w::TextureDescriptor {
            label: None,
            size,
            mip_level_count: mips,
            sample_count: samples,
            dimension: match dims {
                1 => w::TextureDimension::D1,
                3 => w::TextureDimension::D3,
                _ => w::TextureDimension::D2,
            },
            format,
            usage,
            view_formats: &view_formats,
        });
        let mut d = d;
        d.mips = mips;
        d.samples = samples;
        if samples == 1 && !format.is_depth_stencil_format() {
            for (i, (row, slice, data)) in init.iter().enumerate() {
                let i = i as u32;
                let (mip, layer) = if dims == 3 {
                    (i, 0)
                } else {
                    (i % mips, i / mips)
                };
                if mip >= mips {
                    continue;
                }
                self.write_texture_direct(
                    &texture, format, &d, mip, layer, None, data, *row, *slice,
                );
            }
        }
        let serial = self.serial();
        self.objects.insert(
            id,
            Obj::Tex(Tex {
                texture,
                desc: d,
                format,
                serial,
                epoch: 0,
            }),
        );
        S_OK
    }

    /// The region `bx` (or the whole subresource) of `(mip, layer)`:
    /// origin and extent in texels, the physical extent for compressed
    /// formats, and the rows of blocks.
    fn region(
        format: w::TextureFormat,
        d: &TexDesc,
        mip: u32,
        layer: u32,
        bx: Option<[u32; 6]>,
    ) -> (w::Origin3d, w::Extent3d, u32) {
        let (_, bw) = formats::block(format);
        let (mw, mh) = d.mip_size(mip);
        let md = if d.dims == 3 {
            (d.layers >> mip).max(1)
        } else {
            1
        };
        let (x0, y0, z0, x1, y1, z1) = match bx {
            Some(b) => (b[0], b[1], b[2], b[3].min(mw), b[4].min(mh), b[5].min(md)),
            None => (0, 0, 0, mw, mh, md),
        };
        let pw = mw.next_multiple_of(bw);
        let ph = mh.next_multiple_of(bw);
        let ex = (x1.saturating_sub(x0))
            .next_multiple_of(bw)
            .min(pw.saturating_sub(x0));
        let ey = (y1.saturating_sub(y0))
            .next_multiple_of(bw)
            .min(ph.saturating_sub(y0));
        let origin = w::Origin3d {
            x: x0,
            y: y0,
            z: if d.dims == 3 { z0 } else { layer },
        };
        let extent = w::Extent3d {
            width: ex,
            height: ey,
            depth_or_array_layers: if d.dims == 3 {
                z1.saturating_sub(z0).max(1)
            } else {
                1
            },
        };
        (origin, extent, ey / bw)
    }

    #[allow(clippy::too_many_arguments)] // the write's own
    fn write_texture_direct(
        &mut self,
        texture: &w::Texture,
        format: w::TextureFormat,
        d: &TexDesc,
        mip: u32,
        layer: u32,
        bx: Option<[u32; 6]>,
        data: &[u8],
        row: u32,
        slice: u32,
    ) {
        let (bytes, bw) = formats::block(format);
        let (origin, extent, rows) = Self::region(format, d, mip, layer, bx);
        let need_row = (extent.width / bw) * bytes;
        if extent.width == 0 || rows == 0 {
            return;
        }
        let row = if row == 0 { need_row } else { row };
        let depth = extent.depth_or_array_layers;
        let slice = if slice == 0 { row * rows } else { slice };
        let need = u64::from(slice) * u64::from(depth - 1)
            + u64::from(row) * u64::from(rows - 1)
            + u64::from(need_row);
        if row < need_row || (data.len() as u64) < need {
            self.fail(format!(
                "texture write short: {} of {need} bytes",
                data.len()
            ));
            return;
        }
        let Some(dev) = &self.dev else {
            return;
        };
        dev.queue.write_texture(
            w::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin,
                aspect: w::TextureAspect::All,
            },
            data,
            w::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(slice / row.max(1)),
            },
            extent,
        );
    }

    fn make_view(&mut self, id: u32, class: usize, resource: u32, desc: Option<&[u8]>) -> u32 {
        use w::TextureViewDimension as V;
        let Some(Obj::Tex(t)) = self.objects.get(&resource).map(|o| match o {
            Obj::Tex(t) => Obj::Tex(Tex {
                texture: t.texture.clone(),
                desc: t.desc,
                format: t.format,
                serial: t.serial,
                epoch: t.epoch,
            }),
            _ => Obj::Other,
        }) else {
            // a buffer's view (not used by the game's draws), an unknown
            // resource
            if matches!(self.objects.get(&resource), Some(Obj::Buf(_))) {
                self.objects.insert(id, Obj::Other);
                return S_OK;
            }
            return E_INVALIDARG;
        };
        let words: Vec<u32> = desc
            .unwrap_or(&[])
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        let g = |i: usize| words.get(i).copied().unwrap_or(0);
        let d = t.desc;
        let ds = t.format.is_depth_stencil_format();
        let has_stencil = t.format.has_stencil_aspect();
        let has_depth = t.format.has_depth_aspect();
        let view_format = if desc.is_some() && !ds {
            formats::texture_format(g(0), false)
                .filter(|f| f.remove_srgb_suffix() == t.format.remove_srgb_suffix())
                .unwrap_or(t.format)
        } else {
            t.format
        };
        let all_layers = if d.dims == 3 {
            1
        } else {
            t.texture.depth_or_array_layers()
        };
        let mips = t.texture.mip_level_count();
        let ms = t.texture.sample_count() > 1;
        let (dimension, base_mip, mip_count, base_layer, layers, aspect) = match class {
            class::SRV => {
                let aspect = if ds {
                    // R24_UNORM_X8 / R32_FLOAT_X8X24: depth; X24_G8 /
                    // X32_G8X24: stencil
                    if matches!(g(0), 47 | 22) && desc.is_some() && has_stencil {
                        w::TextureAspect::StencilOnly
                    } else if has_depth && has_stencil {
                        w::TextureAspect::DepthOnly
                    } else {
                        w::TextureAspect::All
                    }
                } else {
                    w::TextureAspect::All
                };
                let mip_range = |most: u32, count: u32| {
                    let base = most.min(mips - 1);
                    let n = if count == u32::MAX || count == 0 {
                        mips - base
                    } else {
                        count.min(mips - base)
                    };
                    (base, n.max(1))
                };
                if desc.is_none() {
                    let dim = match d.dims {
                        1 => V::D1,
                        3 => V::D3,
                        _ if d.cube() => V::Cube,
                        _ if all_layers > 1 => V::D2Array,
                        _ => V::D2,
                    };
                    (
                        dim,
                        0,
                        mips,
                        0,
                        if dim == V::D3 || dim == V::D1 {
                            1
                        } else {
                            all_layers
                        },
                        aspect,
                    )
                } else {
                    match g(1) {
                        2 => {
                            let (b, n) = mip_range(g(2), g(3));
                            (V::D1, b, n, 0, 1, aspect)
                        }
                        4 => {
                            let (b, n) = mip_range(g(2), g(3));
                            (V::D2, b, n, 0, 1, aspect)
                        }
                        3 | 5 => {
                            let (b, n) = mip_range(g(2), g(3));
                            let first = g(4).min(all_layers - 1);
                            (
                                V::D2Array,
                                b,
                                n,
                                first,
                                g(5).clamp(1, all_layers - first),
                                aspect,
                            )
                        }
                        6 => (V::D2, 0, 1, 0, 1, aspect),
                        7 => {
                            let first = g(2).min(all_layers - 1);
                            (
                                V::D2Array,
                                0,
                                1,
                                first,
                                g(3).clamp(1, all_layers - first),
                                aspect,
                            )
                        }
                        8 => {
                            let (b, n) = mip_range(g(2), g(3));
                            (V::D3, b, n, 0, 1, aspect)
                        }
                        9 => {
                            let (b, n) = mip_range(g(2), g(3));
                            (V::Cube, b, n, 0, 6, aspect)
                        }
                        10 => {
                            let (b, n) = mip_range(g(2), g(3));
                            let first = g(4).min(all_layers - 1);
                            (
                                V::CubeArray,
                                b,
                                n,
                                first,
                                (g(5).max(1) * 6).min(all_layers - first),
                                aspect,
                            )
                        }
                        _ => {
                            self.objects.insert(id, Obj::Other);
                            return S_OK;
                        }
                    }
                }
            }
            class::RTV | class::DSV => {
                let rtv = class == class::RTV;
                // (dimension code, mip, first layer)
                let (dim, mip, layer) = match (rtv, g(1)) {
                    (_, _) if desc.is_none() => (2, 0, 0),
                    (true, 4) | (false, 3) => (2, g(2), 0),
                    (true, 5) | (false, 4) => (2, g(2), g(3)),
                    (true, 6) | (false, 5) => (2, 0, 0),
                    (true, 7) | (false, 6) => (2, 0, g(2)),
                    (true, 8) => (3, g(2), g(3)),
                    (true, 2 | 3) | (false, 1 | 2) => (
                        2,
                        g(2),
                        if g(1) == 3 || (!rtv && g(1) == 2) {
                            g(3)
                        } else {
                            0
                        },
                    ),
                    _ => (2, 0, 0),
                };
                let mip = mip.min(mips - 1);
                if dim == 3 {
                    self.fail("render target view of a 3D texture slice");
                }
                (
                    if d.dims == 1 { V::D1 } else { V::D2 },
                    mip,
                    1,
                    layer.min(all_layers - 1),
                    1,
                    w::TextureAspect::All,
                )
            }
            _ => {
                self.objects.insert(id, Obj::Other);
                return S_OK;
            }
        };
        let dimension = if ms { V::D2 } else { dimension };
        let view = t.texture.create_view(&w::TextureViewDescriptor {
            label: None,
            // a depth-stencil view's format follows its aspect
            format: (!ds).then_some(view_format),
            dimension: Some(dimension),
            usage: None,
            aspect,
            base_mip_level: base_mip,
            mip_level_count: Some(mip_count),
            base_array_layer: base_layer,
            array_layer_count: Some(layers),
        });
        let (texture, tex_serial, samples, size) = (
            t.texture.clone(),
            t.serial,
            t.texture.sample_count(),
            d.mip_size(base_mip),
        );
        let serial = self.serial();
        self.objects.insert(
            id,
            Obj::View(ViewObj {
                class,
                view: Arc::new(view),
                serial,
                texture,
                tex_serial,
                format: view_format,
                samples,
                size,
                dimension,
                depth: ds && aspect != w::TextureAspect::StencilOnly && has_depth,
                stencil: aspect == w::TextureAspect::StencilOnly,
                base_mip,
            }),
        );
        S_OK
    }

    // ---- resource writes ------------------------------------------------

    /// The game's bytes `data` at `offset` of buffer `id`.
    fn write_buffer(&mut self, id: u32, offset: u32, data: &[u8]) {
        let epoch = self.epoch;
        let Some(Obj::Buf(b)) = self.objects.get_mut(&id) else {
            return;
        };
        b.version += 1;
        let off = offset as usize;
        if let Some(s) = b.shadow.as_mut() {
            let end = (off + data.len()).min(s.len());
            if off < end {
                s[off..end].copy_from_slice(&data[..end - off]);
            }
        }
        // constant and dynamic buffers are read from the shadow
        if b.buffer.is_none() || b.dynamic() {
            return;
        }
        let Some(buffer) = b.buffer.clone() else {
            return;
        };
        // 4-byte aligned, from the shadow when there is one
        let start = off & !3;
        let size = buffer.size() as usize;
        let end = (off + data.len()).next_multiple_of(4).min(size);
        if start >= end {
            return;
        }
        let bytes: Vec<u8> = match &b.shadow {
            Some(s) => {
                let mut v = s[start.min(s.len())..end.min(s.len())].to_vec();
                v.resize(end - start, 0);
                v
            }
            None => {
                let mut v = vec![0u8; off - start];
                v.extend_from_slice(data);
                v.resize(end - start, 0);
                v
            }
        };
        if b.epoch == epoch {
            let at = self.upload.put(&bytes, 4, bytes.len());
            b.epoch = epoch;
            self.cmds.push(Cmd::UploadBuffer {
                at,
                dst: buffer,
                offset: start as u64,
                size: bytes.len() as u64,
            });
        } else if let Some(dev) = &self.dev {
            dev.queue.write_buffer(&buffer, start as u64, &bytes);
        }
    }

    /// The game's bytes for subresource `sub` of texture `id` (`bx`: a
    /// region).
    fn write_texture(
        &mut self,
        id: u32,
        sub: u32,
        bx: Option<[u32; 6]>,
        data: &[u8],
        row: u32,
        slice: u32,
    ) {
        let epoch = self.epoch;
        let Some(Obj::Tex(t)) = self.objects.get_mut(&id) else {
            return;
        };
        let (texture, format, d) = (t.texture.clone(), t.format, t.desc);
        if d.samples > 1 || format.is_depth_stencil_format() {
            self.fail("a write to a multisampled or depth texture");
            return;
        }
        let mips = d.mips;
        let (mip, layer) = if d.dims == 3 {
            (sub, 0)
        } else {
            (sub % mips, sub / mips)
        };
        if mip >= mips {
            return;
        }
        if t.epoch != epoch {
            self.write_texture_direct(&texture, format, &d, mip, layer, bx, data, row, slice);
            return;
        }
        // in the command list: rows restaged at a 256-byte pitch
        let (bytes, bw) = formats::block(format);
        let (origin, extent, rows) = Self::region(format, &d, mip, layer, bx);
        let need_row = (extent.width / bw) * bytes;
        let row = if row == 0 { need_row } else { row };
        let depth = extent.depth_or_array_layers;
        let slice = if slice == 0 { row * rows } else { slice };
        if extent.width == 0 || rows == 0 {
            return;
        }
        let pitch = need_row.next_multiple_of(w::COPY_BYTES_PER_ROW_ALIGNMENT);
        let mut staged = vec![0u8; (pitch * rows * depth) as usize];
        for z in 0..depth {
            for y in 0..rows {
                let from = (z * slice + y * row) as usize;
                let to = ((z * rows + y) * pitch) as usize;
                let Some(src) = data.get(from..from + need_row as usize) else {
                    self.fail("texture write short");
                    return;
                };
                staged[to..to + need_row as usize].copy_from_slice(src);
            }
        }
        let at = self.upload.put(&staged, 256, staged.len());
        self.cmds.push(Cmd::UploadTexture {
            at,
            bytes_per_row: pitch,
            rows,
            dst: TexAt {
                texture,
                mip,
                origin,
                aspect: w::TextureAspect::All,
            },
            extent,
        });
    }

    fn touch_buffer(&mut self, id: u32) {
        let epoch = self.epoch;
        if let Some(Obj::Buf(b)) = self.objects.get_mut(&id) {
            b.epoch = epoch;
        }
    }

    fn touch_texture(&mut self, id: u32) {
        let epoch = self.epoch;
        if let Some(Obj::Tex(t)) = self.objects.get_mut(&id) {
            t.epoch = epoch;
        }
    }

    /// Marks the textures behind views as used by the command list.
    fn touch_view_texture(&mut self, serial: u64) {
        let epoch = self.epoch;
        for o in self.objects.values_mut() {
            if let Obj::Tex(t) = o
                && t.serial == serial
            {
                t.epoch = epoch;
                return;
            }
        }
    }

    // ---- copies ----------------------------------------------------------

    fn copy(&mut self, dst: u32, src: u32) {
        match (self.objects.get(&dst), self.objects.get(&src)) {
            (Some(Obj::Buf(d)), Some(Obj::Buf(s))) => {
                if let Some(ss) = s.shadow.clone()
                    && d.shadow.is_some()
                {
                    // the CPU copies follow (constant and dynamic buffers
                    // are read from them)
                    self.write_buffer(dst, 0, &ss);
                    return;
                }
                let (Some(db), Some(sb)) = (d.buffer.clone(), s.buffer.clone()) else {
                    return;
                };
                let size = db.size().min(sb.size());
                self.touch_buffer(dst);
                self.touch_buffer(src);
                self.cmds.push(Cmd::CopyBuffer {
                    src: sb,
                    src_offset: 0,
                    dst: db,
                    dst_offset: 0,
                    size,
                });
            }
            (Some(Obj::Tex(d)), Some(Obj::Tex(s))) => {
                if d.format.remove_srgb_suffix() != s.format.remove_srgb_suffix()
                    || d.desc.samples != s.desc.samples
                {
                    self.fail(format!("copy {:?} to {:?}", s.format, d.format));
                    return;
                }
                if d.format.is_depth_stencil_format() {
                    self.fail("a copy of a depth-stencil texture");
                    return;
                }
                let (dt, st, desc) = (d.texture.clone(), s.texture.clone(), d.desc);
                let mips = dt.mip_level_count().min(st.mip_level_count());
                let layers = dt.depth_or_array_layers().min(st.depth_or_array_layers());
                self.touch_texture(dst);
                self.touch_texture(src);
                for mip in 0..mips {
                    let ext = dt.size().mip_level_size(mip, dt.dimension());
                    let ext = w::Extent3d {
                        depth_or_array_layers: if desc.dims == 3 {
                            ext.depth_or_array_layers
                        } else {
                            layers
                        },
                        ..ext
                    };
                    self.cmds.push(Cmd::CopyTexture {
                        src: TexAt {
                            texture: st.clone(),
                            mip,
                            origin: w::Origin3d::ZERO,
                            aspect: w::TextureAspect::All,
                        },
                        dst: TexAt {
                            texture: dt.clone(),
                            mip,
                            origin: w::Origin3d::ZERO,
                            aspect: w::TextureAspect::All,
                        },
                        extent: ext,
                    });
                }
            }
            _ => {}
        }
    }

    fn copy_region(&mut self, a: [u32; 7], bx: Option<[u32; 6]>) {
        let [dst, dsub, x, y, z, src, ssub] = a;
        match (self.objects.get(&dst), self.objects.get(&src)) {
            (Some(Obj::Buf(d)), Some(Obj::Buf(s))) => {
                let (from, to) = bx.map_or((0, s.desc[0]), |b| (b[0], b[3]));
                let n = to.saturating_sub(from);
                if let Some(ss) = s.shadow.as_ref()
                    && d.shadow.is_some()
                {
                    let bytes =
                        ss[(from as usize).min(ss.len())..(to as usize).min(ss.len())].to_vec();
                    self.write_buffer(dst, x, &bytes);
                    return;
                }
                let (Some(db), Some(sb)) = (d.buffer.clone(), s.buffer.clone()) else {
                    return;
                };
                if from % 4 != 0 || x % 4 != 0 || n % 4 != 0 {
                    self.fail("an unaligned buffer region copy");
                    return;
                }
                self.touch_buffer(dst);
                self.touch_buffer(src);
                self.cmds.push(Cmd::CopyBuffer {
                    src: sb,
                    src_offset: u64::from(from),
                    dst: db,
                    dst_offset: u64::from(x),
                    size: u64::from(n),
                });
            }
            (Some(Obj::Tex(d)), Some(Obj::Tex(s))) => {
                if d.format.remove_srgb_suffix() != s.format.remove_srgb_suffix()
                    || d.format.is_depth_stencil_format()
                    || d.desc.samples > 1
                {
                    self.fail(format!("region copy {:?} to {:?}", s.format, d.format));
                    return;
                }
                let (sm, sl) = if s.desc.dims == 3 {
                    (ssub, 0)
                } else {
                    (ssub % s.desc.mips, ssub / s.desc.mips)
                };
                let (dm, dl) = if d.desc.dims == 3 {
                    (dsub, 0)
                } else {
                    (dsub % d.desc.mips, dsub / d.desc.mips)
                };
                let (sorigin, ext, _) = Self::region(s.format, &s.desc, sm, sl, bx);
                let (st, dt, ddesc, format) =
                    (s.texture.clone(), d.texture.clone(), d.desc, d.format);
                // clipped to the destination
                let (dw, dh) = ddesc.mip_size(dm);
                let (_, bw) = formats::block(format);
                let ext = w::Extent3d {
                    width: ext.width.min(dw.next_multiple_of(bw).saturating_sub(x)),
                    height: ext.height.min(dh.next_multiple_of(bw).saturating_sub(y)),
                    depth_or_array_layers: ext.depth_or_array_layers,
                };
                if ext.width == 0 || ext.height == 0 {
                    return;
                }
                self.touch_texture(dst);
                self.touch_texture(src);
                self.cmds.push(Cmd::CopyTexture {
                    src: TexAt {
                        texture: st,
                        mip: sm,
                        origin: sorigin,
                        aspect: w::TextureAspect::All,
                    },
                    dst: TexAt {
                        texture: dt,
                        mip: dm,
                        origin: w::Origin3d {
                            x,
                            y,
                            z: if ddesc.dims == 3 { z } else { dl },
                        },
                        aspect: w::TextureAspect::All,
                    },
                    extent: ext,
                });
            }
            _ => {}
        }
    }

    fn resolve(&mut self, a: [u32; 5]) {
        let (Some(Obj::Tex(d)), Some(Obj::Tex(s))) =
            (self.objects.get(&a[0]), self.objects.get(&a[2]))
        else {
            return;
        };
        if s.texture.sample_count() <= 1 || d.texture.sample_count() != 1 {
            self.fail("a resolve that isn't multisampled to single-sampled");
            return;
        }
        if d.format.remove_srgb_suffix() != s.format.remove_srgb_suffix()
            || !d
                .texture
                .usage()
                .contains(w::TextureUsages::RENDER_ATTACHMENT)
        {
            self.fail(format!("resolve {:?} to {:?}", s.format, d.format));
            return;
        }
        let dm = a[1] % d.desc.mips;
        let dl = a[1] / d.desc.mips;
        let view_format = formats::texture_format(a[4], false)
            .filter(|f| f.remove_srgb_suffix() == s.format.remove_srgb_suffix())
            .unwrap_or(s.format);
        let src = s.texture.create_view(&w::TextureViewDescriptor {
            format: Some(view_format),
            dimension: Some(w::TextureViewDimension::D2),
            base_array_layer: a[3],
            array_layer_count: Some(1),
            ..Default::default()
        });
        let dst = d.texture.create_view(&w::TextureViewDescriptor {
            format: Some(view_format),
            dimension: Some(w::TextureViewDimension::D2),
            base_mip_level: dm,
            mip_level_count: Some(1),
            base_array_layer: dl,
            array_layer_count: Some(1),
            ..Default::default()
        });
        self.touch_texture(a[0]);
        self.touch_texture(a[2]);
        self.cmds.push(Cmd::Resolve {
            src: Arc::new(src),
            dst: Arc::new(dst),
        });
    }

    // ---- draws -----------------------------------------------------------

    fn state(&self, id: u32, kind: StateKind) -> Option<(&[u8], u32)> {
        match self.objects.get(&id) {
            Some(Obj::State(k, d, key)) if *k == kind => Some((d.as_slice(), *key)),
            _ => None,
        }
    }

    fn linked(&mut self, vs: u32, ps: u32) -> Option<Arc<Linked>> {
        let Some(Obj::Shader(v)) = self.objects.get(&vs) else {
            return None;
        };
        let p = match self.objects.get(&ps) {
            Some(Obj::Shader(p)) => Some((p.hash, p.bytecode.clone())),
            _ => None,
        };
        let key = (v.hash, p.as_ref().map_or(0, |p| p.0));
        if let Some(l) = self.linked.get(&key) {
            return l.clone();
        }
        let vb = v.bytecode.clone();
        let serial = self.serial();
        let device = self.device()?.clone();
        let limits = device.limits();
        let result = (|| -> Result<Linked, String> {
            let vc = Container::parse(&vb).map_err(|e| e.to_string())?;
            let pc = match &p {
                Some((_, b)) => Some(Container::parse(b).map_err(|e| e.to_string())?),
                None => None,
            };
            let varyings = match &pc {
                Some(pc) => wgsl::link(&vc, pc),
                None => wgsl::own_varyings(&vc),
            }
            .map_err(|e| e.to_string())?;
            let vs = wgsl::translate(&vc, &varyings, 0).map_err(|e| e.to_string())?;
            let ps = match &pc {
                Some(pc) => Some(wgsl::translate(pc, &varyings, 1).map_err(|e| e.to_string())?),
                None => None,
            };
            let vertex_id = vc
                .inputs()
                .map_err(|e| e.to_string())?
                .iter()
                .any(|s| s.system == 6);
            let scope = device.push_error_scope(w::ErrorFilter::Validation);
            let vs_module = device.create_shader_module(w::ShaderModuleDescriptor {
                label: Some("vs"),
                source: w::ShaderSource::Wgsl(vs.wgsl.clone().into()),
            });
            let ps_module = ps.as_ref().map(|ps| {
                device.create_shader_module(w::ShaderModuleDescriptor {
                    label: Some("ps"),
                    source: w::ShaderSource::Wgsl(ps.wgsl.clone().into()),
                })
            });
            let buffers = vs.buffers.len() + ps.as_ref().map_or(0, |p| p.buffers.len());
            let dynamic = buffers as u32 <= limits.max_dynamic_uniform_buffers_per_pipeline_layout;
            let empty = wgsl::Shader {
                wgsl: String::new(),
                vertex: false,
                buffers: Vec::new(),
                textures: Vec::new(),
                samplers: Vec::new(),
                inputs: Vec::new(),
                targets: Vec::new(),
                writes_depth: false,
            };
            let g0 = group_layout(&device, &vs, w::ShaderStages::VERTEX, dynamic);
            let g1 = group_layout(
                &device,
                ps.as_ref().unwrap_or(&empty),
                w::ShaderStages::FRAGMENT,
                dynamic,
            );
            let layout = device.create_pipeline_layout(&w::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&g0), Some(&g1)],
                immediate_size: 0,
            });
            if let Some(e) = pollster::block_on(scope.pop()) {
                return Err(e.to_string());
            }
            Ok(Linked {
                serial,
                vs,
                ps,
                vs_module,
                ps_module,
                alpha_test: Mutex::new(None),
                layout,
                groups: [g0, g1],
                dynamic,
                vertex_id,
            })
        })();
        let linked = match result {
            Ok(l) => Some(Arc::new(l)),
            Err(e) => {
                self.fail(format!("shaders {:016x}/{:016x}: {e}", key.0, key.1));
                None
            }
        };
        self.linked.insert(key, linked.clone());
        linked
    }

    #[allow(clippy::too_many_arguments)] // the pipeline's inputs
    fn pipeline(
        &mut self,
        linked: &Arc<Linked>,
        strides: [u32; VERTEX_SLOTS],
        colors: &[Option<w::TextureFormat>],
        depth_format: Option<w::TextureFormat>,
        samples: u32,
        indexed: bool,
    ) -> Option<Arc<w::RenderPipeline>> {
        let b = self.bound.clone();
        let blend = self
            .state(b.blend, StateKind::Blend)
            .map(|(d, k)| (d.to_vec(), k));
        let depth = self
            .state(b.depth, StateKind::Depth)
            .map(|(d, k)| (d.to_vec(), k));
        let raster = self
            .state(b.raster, StateKind::Raster)
            .map(|(d, k)| (d.to_vec(), k));
        let (elements, layout_key) = match self.objects.get(&b.layout) {
            Some(Obj::Layout(e, k)) => (e.clone(), *k),
            _ => (Vec::new(), 0),
        };
        let topology_code = b.topology;
        let Some(topology) = formats::topology(topology_code) else {
            self.fail(format!("topology {topology_code}"));
            return None;
        };
        let strip = matches!(
            topology,
            w::PrimitiveTopology::TriangleStrip | w::PrimitiveTopology::LineStrip
        ) && indexed;
        let key = PipelineKey {
            linked: linked.serial,
            layout: layout_key,
            strides,
            topology: topology_code,
            strip_index: strip.then_some(b.index.1),
            blend: blend.as_ref().map_or(0, |b| b.1),
            depth: depth.as_ref().map_or(0, |b| b.1),
            raster: raster.as_ref().map_or(0, |b| b.1),
            sample_mask: b.sample_mask,
            colors: colors.to_vec(),
            depth_format,
            samples,
        };
        if let Some(p) = self.pipelines.get(&key) {
            return p.clone();
        }
        let device = self.device()?.clone();
        let limits = device.limits();
        let features = device.features();
        // vertex buffers: the attributes the shader reads, per slot
        let mut offsets = [0u32; VERTEX_SLOTS];
        let mut placed = Vec::new();
        for e in &elements {
            let slot = (e.slot as usize).min(VERTEX_SLOTS - 1);
            let offset = if e.offset == u32::MAX {
                offsets[slot]
            } else {
                e.offset
            };
            match formats::vertex_format(e.format) {
                Some(f) => {
                    offsets[slot] = offset + f.size() as u32;
                    placed.push((e, offset, f));
                }
                None => self.fail(format!("vertex format {}", e.format)),
            }
        }
        let mut per_slot: Vec<(u32, bool, Vec<w::VertexAttribute>)> = Vec::new();
        let mut missing = Vec::new();
        for input in &linked.vs.inputs {
            let Some((e, offset, format)) = placed.iter().find(|(e, _, _)| {
                e.semantic.eq_ignore_ascii_case(&input.semantic) && e.index == input.index
            }) else {
                missing.push(format!("{}{}", input.semantic, input.index));
                continue;
            };
            let i = match per_slot.iter().position(|p| p.0 == e.slot) {
                Some(i) => i,
                None => {
                    per_slot.push((e.slot, e.per_instance, Vec::new()));
                    per_slot.len() - 1
                }
            };
            // Direct3D lets an element run past the stride; wgpu doesn't:
            // keep the components that fit
            let stride = strides.get(e.slot as usize).copied().unwrap_or(0);
            let format = shrink_format(*format, stride.saturating_sub(*offset));
            per_slot[i].2.push(w::VertexAttribute {
                format,
                offset: u64::from(*offset),
                shader_location: input.location,
            });
        }
        let fail = |this: &mut Self, what: String| {
            this.fail(what);
            this.pipelines.insert(key.clone(), None);
            None
        };
        if !missing.is_empty() {
            return fail(
                self,
                format!(
                    "vertex inputs missing from the layout: {}",
                    missing.join(",")
                ),
            );
        }
        per_slot.sort_by_key(|p| p.0);
        let slots = per_slot.iter().map(|p| p.0 + 1).max().unwrap_or(0);
        if slots > limits.max_vertex_buffers {
            return fail(
                self,
                format!(
                    "vertex slot {slots} past the adapter's {}",
                    limits.max_vertex_buffers
                ),
            );
        }
        let buffers: Vec<Option<w::VertexBufferLayout>> = (0..slots)
            .map(|slot| {
                let p = per_slot.iter().find(|p| p.0 == slot);
                Some(w::VertexBufferLayout {
                    array_stride: u64::from(strides.get(slot as usize).copied().unwrap_or(0)),
                    step_mode: if p.is_some_and(|p| p.1) {
                        w::VertexStepMode::Instance
                    } else {
                        w::VertexStepMode::Vertex
                    },
                    attributes: p.map_or(&[][..], |p| &p.2),
                })
            })
            .collect();
        let bdesc = blend.as_ref().map(|b| b.0.as_slice());
        let written: Vec<u32> = linked
            .ps
            .as_ref()
            .map_or(Vec::new(), |p| p.targets.iter().map(|t| t.0).collect());
        let targets: Vec<Option<w::ColorTargetState>> = colors
            .iter()
            .enumerate()
            .map(|(i, f)| {
                f.map(|format| {
                    let (blend, mask) = formats::blend(bdesc, i);
                    let float = matches!(
                        format.sample_type(None, None),
                        Some(w::TextureSampleType::Float { .. })
                    );
                    w::ColorTargetState {
                        format,
                        blend: blend.filter(|_| float),
                        write_mask: if written.contains(&(i as u32)) {
                            mask
                        } else {
                            w::ColorWrites::empty()
                        },
                    }
                })
            })
            .collect();
        let rdesc = raster.as_ref().map(|r| r.0.as_slice());
        let (cull, front, mut bias, _) = formats::raster(rdesc);
        let (wire, depth_clip) = formats::raster_extra(rdesc);
        if !matches!(
            topology,
            w::PrimitiveTopology::TriangleList | w::PrimitiveTopology::TriangleStrip
        ) {
            bias = w::DepthBiasState::default();
        }
        let depth_stencil = depth_format
            .map(|f| formats::depth_stencil(depth.as_ref().map(|d| d.0.as_slice()), f, bias));
        let a2c = formats::alpha_to_coverage(bdesc);
        let fragment_module = match (&linked.ps_module, samples == 1 && a2c) {
            (None, _) => None,
            (Some(m), false) => Some(m.clone()),
            (Some(_), true) => {
                // Alpha to coverage on a single-sampled target: Direct3D 11
                // covers the one sample when the output alpha is at least
                // 0.5 (the backgrounds' cut-outs, kami0100's book;
                // docs/m6.md); wgpu allows it only with multisampling.
                let mut cached = linked.alpha_test.lock().ok()?;
                if cached.is_none() {
                    let source = linked.ps.as_ref()?.wgsl.replacen(
                        "    run();\n    var out: Out;",
                        "    run();\n    if (bitcast<vec4<f32>>(o0).w < 0.5) { discard; }\n    var out: Out;",
                        1,
                    );
                    *cached = Some(device.create_shader_module(w::ShaderModuleDescriptor {
                        label: Some("ps, alpha to coverage as a test"),
                        source: w::ShaderSource::Wgsl(source.into()),
                    }));
                }
                cached.clone()
            }
        };
        let scope = device.push_error_scope(w::ErrorFilter::Validation);
        let pipeline = device.create_render_pipeline(&w::RenderPipelineDescriptor {
            label: None,
            layout: Some(&linked.layout),
            vertex: w::VertexState {
                module: &linked.vs_module,
                entry_point: Some("main"),
                compilation_options: w::PipelineCompilationOptions::default(),
                buffers: &buffers,
            },
            primitive: w::PrimitiveState {
                topology,
                strip_index_format: strip.then_some(if b.index.1 == 42 {
                    w::IndexFormat::Uint32
                } else {
                    w::IndexFormat::Uint16
                }),
                front_face: front,
                cull_mode: cull,
                unclipped_depth: !depth_clip && features.contains(w::Features::DEPTH_CLIP_CONTROL),
                polygon_mode: if wire && features.contains(w::Features::POLYGON_MODE_LINE) {
                    w::PolygonMode::Line
                } else {
                    w::PolygonMode::Fill
                },
                conservative: false,
            },
            depth_stencil,
            multisample: w::MultisampleState {
                count: samples,
                mask: u64::from(b.sample_mask),
                alpha_to_coverage_enabled: samples > 1 && a2c,
            },
            fragment: fragment_module.as_ref().map(|m| w::FragmentState {
                module: m,
                entry_point: Some("main"),
                compilation_options: w::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        let result = match pollster::block_on(scope.pop()) {
            Some(e) => {
                self.fail(format!("pipeline: {e}"));
                None
            }
            None => Some(Arc::new(pipeline)),
        };
        self.pipelines.insert(key, result.clone());
        result
    }

    fn sampler(&mut self, id: u32, comparison: bool) -> Option<(Arc<w::Sampler>, u64)> {
        let (desc, key) = match self.state(id, StateKind::Sampler) {
            Some((d, k)) => (Some(d.to_vec()), k),
            None => (None, 0),
        };
        if let Some(s) = self.samplers.get(&(key, comparison)) {
            return Some(s.clone());
        }
        let device = self.device()?.clone();
        let sd = match &desc {
            Some(d) => {
                if formats::sampler_lod_bias(d) != 0.0 {
                    self.fail("a sampler's MipLODBias (wgpu has none)");
                }
                let mut sd = formats::sampler(d, true);
                if sd.border_color == Some(w::SamplerBorderColor::TransparentBlack)
                    && !device
                        .features()
                        .contains(w::Features::ADDRESS_MODE_CLAMP_TO_ZERO)
                {
                    sd.border_color = Some(w::SamplerBorderColor::OpaqueBlack);
                }
                sd
            }
            // the default sampler: linear, clamped
            None => w::SamplerDescriptor {
                mag_filter: w::FilterMode::Linear,
                min_filter: w::FilterMode::Linear,
                mipmap_filter: w::MipmapFilterMode::Linear,
                ..Default::default()
            },
        };
        let mut sd = sd;
        if comparison && sd.compare.is_none() {
            sd.compare = Some(w::CompareFunction::LessEqual);
        }
        if !comparison {
            sd.compare = None;
        }
        let uses_border = [sd.address_mode_u, sd.address_mode_v, sd.address_mode_w]
            .contains(&w::AddressMode::ClampToBorder);
        if uses_border
            && !device
                .features()
                .contains(w::Features::ADDRESS_MODE_CLAMP_TO_BORDER)
        {
            for m in [
                &mut sd.address_mode_u,
                &mut sd.address_mode_v,
                &mut sd.address_mode_w,
            ] {
                if *m == w::AddressMode::ClampToBorder {
                    *m = w::AddressMode::ClampToEdge;
                }
            }
        }
        if !uses_border {
            sd.border_color = None;
        }
        let s = Arc::new(device.create_sampler(&sd));
        let serial = self.serial();
        self.samplers.insert((key, comparison), (s.clone(), serial));
        Some((s, serial))
    }

    fn dummy(&mut self, kind: TextureKind, dim: Dim) -> Option<(Arc<w::TextureView>, u64)> {
        let kc = match kind {
            TextureKind::Float => 0,
            TextureKind::Uint => 1,
            TextureKind::Sint => 2,
            TextureKind::Depth => 3,
            TextureKind::Multisampled => 4,
        };
        let dc = match dim {
            Dim::TextureCube => 1,
            Dim::Texture3D => 2,
            Dim::Texture1D => 3,
            _ => 0,
        };
        if let Some(v) = self.dummies.get(&(kc, dc)) {
            return Some(v.clone());
        }
        let device = self.device()?.clone();
        let format = match kind {
            TextureKind::Uint => w::TextureFormat::Rgba8Uint,
            TextureKind::Sint => w::TextureFormat::Rgba8Sint,
            TextureKind::Depth => w::TextureFormat::Depth32Float,
            _ => w::TextureFormat::Rgba8Unorm,
        };
        let layers = if dim == Dim::TextureCube { 6 } else { 1 };
        let samples = if kind == TextureKind::Multisampled {
            4
        } else {
            1
        };
        let dimension = match dim {
            Dim::Texture3D => w::TextureDimension::D3,
            Dim::Texture1D => w::TextureDimension::D1,
            _ => w::TextureDimension::D2,
        };
        let mut usage = w::TextureUsages::TEXTURE_BINDING;
        if samples > 1 {
            usage |= w::TextureUsages::RENDER_ATTACHMENT;
        }
        let texture = device.create_texture(&w::TextureDescriptor {
            label: Some("unbound"),
            size: w::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension,
            format,
            usage,
            view_formats: &[],
        });
        let view = Arc::new(texture.create_view(&w::TextureViewDescriptor {
            dimension: Some(view_dimension(dim)),
            ..Default::default()
        }));
        let serial = self.serial();
        self.dummies.insert((kc, dc), (view.clone(), serial));
        Some((view, serial))
    }

    /// The float copy of depth texture `v` for reading as a float texture,
    /// refreshed (a command) when the depth changed since.
    fn depth_float(&mut self, v: &ViewObj) -> Option<(Arc<w::TextureView>, u64)> {
        let ms = v.samples > 1;
        if let Some((_, view, serial, stale)) = self.depth_copies.get_mut(&v.tex_serial) {
            let (view, serial) = (view.clone(), *serial);
            if *stale {
                *stale = false;
                self.cmds.push(Cmd::DepthToFloat {
                    src: v.view.clone(),
                    dst: view.clone(),
                    multisampled: ms,
                });
            }
            return Some((view, serial));
        }
        let device = self.device()?.clone();
        let (w0, h0) = (v.texture.width(), v.texture.height());
        let texture = device.create_texture(&w::TextureDescriptor {
            label: Some("depth as float"),
            size: w::Extent3d {
                width: w0,
                height: h0,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: w::TextureDimension::D2,
            format: w::TextureFormat::R32Float,
            usage: w::TextureUsages::TEXTURE_BINDING | w::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = Arc::new(texture.create_view(&w::TextureViewDescriptor::default()));
        let serial = self.serial();
        self.depth_copies
            .insert(v.tex_serial, (texture, view.clone(), serial, false));
        self.cmds.push(Cmd::DepthToFloat {
            src: v.view.clone(),
            dst: view.clone(),
            multisampled: ms,
        });
        Some((view, serial))
    }

    /// The view a draw binds for shader resource `v` in slot `t`: `v`'s
    /// own, its depth's float copy, or why it can't be bound (the slot
    /// then reads as unbound).
    fn pick(
        &mut self,
        v: &ViewObj,
        t: &wgsl::TextureBinding,
    ) -> Result<(Arc<w::TextureView>, u64), String> {
        use w::TextureSampleType as S;
        let dim_ok = match t.dim {
            Dim::TextureCube => v.dimension == w::TextureViewDimension::Cube,
            Dim::Texture3D => v.dimension == w::TextureViewDimension::D3,
            Dim::Texture1D => v.dimension == w::TextureViewDimension::D1,
            _ => v.dimension == w::TextureViewDimension::D2,
        };
        let aspect = if v.stencil {
            w::TextureAspect::StencilOnly
        } else if v.depth {
            w::TextureAspect::DepthOnly
        } else {
            w::TextureAspect::All
        };
        let ms = v.samples > 1;
        let why = || {
            format!(
                "texture {:?} {:?} in a {:?} {:?} slot",
                v.format, v.dimension, t.kind, t.dim
            )
        };
        if !dim_ok || ms != (t.kind == TextureKind::Multisampled) {
            return Err(why());
        }
        match (t.kind, v.format.sample_type(Some(aspect), None)) {
            (TextureKind::Depth, Some(S::Depth))
            | (TextureKind::Float | TextureKind::Multisampled, Some(S::Float { .. }))
            | (TextureKind::Uint, Some(S::Uint))
            | (TextureKind::Sint, Some(S::Sint)) => Ok((v.view.clone(), v.serial)),
            (TextureKind::Float, Some(S::Depth))
                if v.dimension == w::TextureViewDimension::D2 && v.base_mip == 0 =>
            {
                self.depth_float(v).ok_or_else(why)
            }
            _ => Err(why()),
        }
    }

    /// Bytes of buffer `id` from `from`, `len` long (zeros past its end).
    fn shadow_bytes(&self, id: u32, from: usize, len: usize) -> Option<Vec<u8>> {
        let Some(Obj::Buf(b)) = self.objects.get(&id) else {
            return None;
        };
        let s = b.shadow.as_ref()?;
        let mut v = s
            .get(from.min(s.len())..(from + len).min(s.len()))?
            .to_vec();
        v.resize(len, 0);
        Some(v)
    }

    /// The smallest and largest index a draw reads.
    fn index_bounds(&self, start: u32, count: u32) -> Option<(u32, u32)> {
        let (ib, format, offset) = self.bound.index;
        let size = if format == 42 { 4 } else { 2 };
        let bytes = self.shadow_bytes(
            ib,
            offset as usize + start as usize * size,
            count as usize * size,
        )?;
        let restart = if size == 4 { u32::MAX } else { 0xffff };
        let strip = matches!(self.bound.topology, 3 | 5);
        let mut lo = u32::MAX;
        let mut hi = 0;
        for c in bytes.chunks_exact(size) {
            let i = if size == 4 {
                u32::from_le_bytes([c[0], c[1], c[2], c[3]])
            } else {
                u32::from(u16::from_le_bytes([c[0], c[1]]))
            };
            if strip && i == restart {
                continue;
            }
            lo = lo.min(i);
            hi = hi.max(i);
        }
        (lo <= hi).then_some((lo, hi))
    }

    fn view(&self, id: u32) -> Option<&ViewObj> {
        match self.objects.get(&id) {
            Some(Obj::View(v)) => Some(v),
            _ => None,
        }
    }

    /// Records a draw with everything bound now.
    fn draw(&mut self, kind: DrawKind, p: [u32; 5]) {
        if self.dev.is_none() {
            return;
        }
        let b = self.bound.clone();
        if b.gs != 0 {
            self.fail("a geometry shader (not drawn)");
            return;
        }
        // (count, instances, start, base vertex, first instance)
        let (count, instances, start, base, first_instance, indexed) = match kind {
            DrawKind::Plain => (p[0], 1, p[1], 0, 0, false),
            DrawKind::Indexed => (p[0], 1, p[1], p[2] as i32, 0, true),
            DrawKind::Instanced => (p[0], p[1], p[2], 0, p[3], false),
            DrawKind::IndexedInstanced => (p[0], p[1], p[2], p[3] as i32, p[4], true),
        };
        if count == 0 || instances == 0 {
            return;
        }
        // the targets
        let n = b.rtv.iter().rposition(|&v| v != 0).map_or(0, |i| i + 1);
        let mut colors = Vec::new();
        let mut color_formats = Vec::new();
        let mut size = None;
        let mut samples = None;
        let mut attached = Vec::new();
        for &id in &b.rtv[..n] {
            match self.view(id).filter(|v| v.class == class::RTV) {
                Some(v)
                    if v.texture
                        .usage()
                        .contains(w::TextureUsages::RENDER_ATTACHMENT) =>
                {
                    colors.push(Some((v.view.clone(), v.serial)));
                    color_formats.push(Some(v.format));
                    size.get_or_insert(v.size);
                    samples.get_or_insert(v.samples);
                    attached.push(v.tex_serial);
                }
                _ => {
                    colors.push(None);
                    color_formats.push(None);
                }
            }
        }
        let depth = self.view(b.dsv).filter(|v| v.class == class::DSV).map(|v| {
            (
                v.view.clone(),
                v.serial,
                v.format.has_stencil_aspect(),
                v.format,
                v.size,
                v.samples,
                v.tex_serial,
            )
        });
        // a depth target of another size than the colour targets: Direct3D
        // 11 draws nothing then (measured: a headless run's 1280 x 720 depth
        // buffer with its 1920 x 1080 back buffer left D3D11's frames
        // black), and wgpu can't bind them together
        if let (Some(d), Some(s)) = (&depth, size)
            && d.4 != s
        {
            self.fail(format!(
                "a {}x{} depth target with {}x{} colour targets (not drawn, as in D3D11)",
                d.4.0, d.4.1, s.0, s.1
            ));
            return;
        }
        if let Some(d) = &depth {
            size.get_or_insert(d.4);
            samples.get_or_insert(d.5);
            attached.push(d.6);
        }
        let (Some(size), Some(samples)) = (size, samples) else {
            // nothing to draw into
            return;
        };
        let Some(linked) = self.linked(b.vs, b.ps) else {
            return;
        };
        let mut strides = [0u32; VERTEX_SLOTS];
        for (s, v) in strides.iter_mut().zip(&b.vertex) {
            *s = v.1;
        }
        // only the slots the layout uses count for the pipeline
        let used: Vec<u32> = match self.objects.get(&b.layout) {
            Some(Obj::Layout(e, _)) => e.iter().map(|e| e.slot).collect(),
            _ => Vec::new(),
        };
        for (i, s) in strides.iter_mut().enumerate() {
            if !used.contains(&(i as u32)) {
                *s = 0;
            }
        }
        let Some(pipeline) = self.pipeline(
            &linked,
            strides,
            &color_formats,
            depth.as_ref().map(|d| d.3),
            samples,
            indexed,
        ) else {
            return;
        };
        let align = self
            .device()
            .map_or(256, |d| {
                d.limits().min_uniform_buffer_offset_alignment as usize
            })
            .max(16);
        let mut binds = Vec::with_capacity(2);
        for g in 0..2 {
            let shader = if g == 0 {
                Some(&linked.vs)
            } else {
                linked.ps.as_ref()
            };
            let Some(shader) = shader else {
                binds.push(BindSpec {
                    views: Vec::new(),
                    samplers: Vec::new(),
                    uniforms: Vec::new(),
                });
                continue;
            };
            let mut uniforms = Vec::new();
            let mut sorted = shader.buffers.clone();
            sorted.sort_by_key(|b| b.0);
            for &(slot, declared) in &sorted {
                let size = u64::from(declared.max(16)).next_multiple_of(16);
                let (id, first, count) = b.cb[g].get(slot as usize).copied().unwrap_or((0, 0, 0));
                let version = match self.objects.get(&id) {
                    Some(Obj::Buf(buf)) => buf.version,
                    _ => 0,
                };
                let memo = (id, version, first, count, size);
                let at = if let Some(&at) = self.uniform_memo.get(&memo) {
                    at
                } else {
                    let from = first as usize * 16;
                    let bytes = self
                        .shadow_bytes(id, from, size as usize)
                        .unwrap_or_default();
                    let at = self.uniform.put(&bytes, align, size as usize);
                    self.uniform_memo.insert(memo, at);
                    at
                };
                uniforms.push((slot, at, size));
            }
            let mut views = Vec::new();
            let textures = shader.textures.clone();
            for t in &textures {
                let id = b.srv[g].get(t.slot as usize).copied().unwrap_or(0);
                let chosen = match self.view(id).filter(|v| v.class == class::SRV).cloned() {
                    // read and written by the same draw: Direct3D 11 unbinds it
                    Some(v) if attached.contains(&v.tex_serial) => None,
                    Some(v) => {
                        self.touch_view_texture(v.tex_serial);
                        match self.pick(&v, t) {
                            Ok(x) => Some(x),
                            Err(why) => {
                                self.fail(why);
                                None
                            }
                        }
                    }
                    None => None,
                };
                let Some((view, serial)) = chosen.or_else(|| self.dummy(t.kind, t.dim)) else {
                    return;
                };
                views.push((t.slot, view, serial));
            }
            let mut sampler_list = Vec::new();
            for &(slot, cmp) in &shader.samplers.clone() {
                let id = b.sampler[g].get(slot as usize).copied().unwrap_or(0);
                let Some((s, serial)) = self.sampler(id, cmp) else {
                    return;
                };
                sampler_list.push((slot, s, serial));
            }
            binds.push(BindSpec {
                views,
                samplers: sampler_list,
                uniforms,
            });
        }
        // vertices and indices
        let index_format = if b.index.1 == 42 {
            w::IndexFormat::Uint32
        } else {
            w::IndexFormat::Uint16
        };
        let index_size: u32 = if b.index.1 == 42 { 4 } else { 2 };
        let elements: Vec<Element> = match self.objects.get(&b.layout) {
            Some(Obj::Layout(e, _)) => e.clone(),
            _ => Vec::new(),
        };
        let per_instance = |slot: u32| elements.iter().any(|e| e.slot == slot && e.per_instance);
        let any_dynamic_vertex = elements.iter().any(|e| {
            let id = b.vertex.get(e.slot as usize).map_or(0, |v| v.0);
            matches!(self.objects.get(&id), Some(Obj::Buf(buf)) if buf.dynamic())
        });
        // the vertices the draw reads (lo..=hi), when a buffer is copied
        let (lo, hi) = if indexed {
            match self.index_bounds(start, count) {
                Some((l, h)) => (
                    (i64::from(l) + i64::from(base)).max(0) as u32,
                    (i64::from(h) + i64::from(base)).max(0) as u32,
                ),
                None => (0, 0),
            }
        } else {
            (start, start + count - 1)
        };
        let lo = if linked.vertex_id || !any_dynamic_vertex {
            0
        } else {
            lo
        };
        let mut vertex = Vec::new();
        let mut slots: Vec<u32> = elements.iter().map(|e| e.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        for slot in slots {
            let Some(&(id, stride, offset)) = b.vertex.get(slot as usize) else {
                continue;
            };
            let Some(Obj::Buf(buf)) = self.objects.get(&id) else {
                // nothing bound: Direct3D 11 reads zeros
                let at = self.geometry.put(&[], 4, 256);
                vertex.push((slot, Src::Arena(at)));
                continue;
            };
            if buf.dynamic() || offset % 4 != 0 {
                let (from, len) = if per_instance(slot) {
                    (
                        offset as usize,
                        ((first_instance + instances) * stride.max(4)) as usize,
                    )
                } else if stride == 0 {
                    (offset as usize, 64)
                } else {
                    (
                        offset as usize + (lo * stride) as usize,
                        ((hi.saturating_sub(lo) + 1) * stride) as usize,
                    )
                };
                let bytes = self
                    .shadow_bytes(id, from, len)
                    .unwrap_or_else(|| vec![0; len]);
                let at = self.geometry.put(&bytes, 4, bytes.len().max(4));
                vertex.push((slot, Src::Arena(at)));
            } else if let Some(buffer) = buf.buffer.clone() {
                if u64::from(offset) >= buffer.size() {
                    let at = self.geometry.put(&[], 4, 256);
                    vertex.push((slot, Src::Arena(at)));
                } else {
                    vertex.push((slot, Src::Buffer(buffer, u64::from(offset))));
                    self.touch_buffer(id);
                }
            }
        }
        // a copied vertex range starts at `lo`
        let (range, base) = if indexed {
            (start..start + count, base - lo as i32)
        } else {
            (start - lo..start - lo + count, 0)
        };
        let index = if indexed {
            let (ib, _, ioff) = b.index;
            match self.objects.get(&ib) {
                Some(Obj::Buf(Buf {
                    buffer: Some(buffer),
                    desc,
                    ..
                })) if desc[1] != 2 && ioff % index_size == 0 => {
                    let buffer = buffer.clone();
                    self.touch_buffer(ib);
                    Some((Src::Buffer(buffer, u64::from(ioff)), index_format))
                }
                Some(Obj::Buf(_)) => {
                    let len = ((start + count) * index_size) as usize;
                    let bytes = self
                        .shadow_bytes(ib, ioff as usize, len)
                        .unwrap_or_else(|| vec![0; len]);
                    let at = self.geometry.put(&bytes, 4, bytes.len());
                    Some((Src::Arena(at), index_format))
                }
                _ => {
                    self.fail("an indexed draw without an index buffer");
                    return;
                }
            }
        } else {
            None
        };
        // the depth target changes: its float copy is stale
        if let Some(d) = &depth
            && let Some(c) = self.depth_copies.get_mut(&d.6)
        {
            c.3 = true;
        }
        for s in attached.clone() {
            self.touch_view_texture(s);
        }
        let (_, _, _, scissor_on) =
            formats::raster(self.state(b.raster, StateKind::Raster).map(|s| s.0));
        let (tw, th) = size;
        let scissor = if scissor_on {
            let c = |v: i32, m: u32| v.clamp(0, m as i32) as u32;
            let (x0, y0) = (c(b.scissor[0], tw), c(b.scissor[1], th));
            let (x1, y1) = (c(b.scissor[2], tw), c(b.scissor[3], th));
            [x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)]
        } else {
            [0, 0, tw, th]
        };
        let binds: [BindSpec; 2] = match binds.try_into() {
            Ok(b) => b,
            Err(_) => return,
        };
        self.cmds.push(Cmd::Draw(Box::new(DrawCmd {
            pipeline,
            linked,
            targets: Targets {
                colors,
                depth: depth.map(|d| (d.0, d.1, d.2)),
            },
            binds,
            vertex,
            call: Call {
                indexed: index,
                range,
                base,
                instances: first_instance..first_instance + instances,
            },
            viewport: b.viewport,
            scissor,
            blend_constant: b.blend_factor,
            stencil_ref: b.stencil_ref,
        })));
    }

    fn clear_target(&mut self, id: u32, color: [f32; 4]) {
        let Some(v) = self.view(id).filter(|v| v.class == class::RTV) else {
            return;
        };
        if !v
            .texture
            .usage()
            .contains(w::TextureUsages::RENDER_ATTACHMENT)
        {
            return;
        }
        let (view, s) = (v.view.clone(), v.tex_serial);
        self.touch_view_texture(s);
        self.cmds.push(Cmd::ClearColor { view, color });
    }

    fn clear_depth(&mut self, id: u32, flags: u32, depth: f32, stencil: u8) {
        let Some(v) = self.view(id).filter(|v| v.class == class::DSV) else {
            return;
        };
        let (view, s, f) = (v.view.clone(), v.tex_serial, v.format);
        self.touch_view_texture(s);
        if let Some(c) = self.depth_copies.get_mut(&s) {
            c.3 = true;
        }
        self.cmds.push(Cmd::ClearDepth {
            view,
            depth: (flags & 1 != 0).then_some(depth.clamp(0.0, 1.0)),
            stencil: (flags & 2 != 0).then_some(u32::from(stencil)),
            has_depth: f.has_depth_aspect(),
            has_stencil: f.has_stencil_aspect(),
        });
    }

    // ---- submission ------------------------------------------------------

    fn bind_group(
        &mut self,
        linked: &Linked,
        g: usize,
        spec: &BindSpec,
    ) -> Option<Arc<w::BindGroup>> {
        let uniform = self.uniform.buffer.clone();
        let key = linked.dynamic.then(|| BindKey {
            linked: linked.serial,
            group: g as u8,
            views: spec.views.iter().map(|v| v.2).collect(),
            samplers: spec.samplers.iter().map(|s| s.2).collect(),
            arena: self.uniform.serial,
        });
        if let Some(k) = &key
            && let Some(bg) = self.bind_groups.get(k)
        {
            return Some(bg.clone());
        }
        let device = self.device()?.clone();
        let mut entries = Vec::new();
        for &(slot, at, size) in &spec.uniforms {
            entries.push(w::BindGroupEntry {
                binding: slot,
                resource: w::BindingResource::Buffer(w::BufferBinding {
                    buffer: uniform.as_ref()?,
                    offset: if linked.dynamic { 0 } else { at },
                    size: w::BufferSize::new(size),
                }),
            });
        }
        for (slot, v, _) in &spec.views {
            entries.push(w::BindGroupEntry {
                binding: wgsl::TEXTURE_BINDING + slot,
                resource: w::BindingResource::TextureView(v),
            });
        }
        for (slot, s, _) in &spec.samplers {
            entries.push(w::BindGroupEntry {
                binding: wgsl::SAMPLER_BINDING + slot,
                resource: w::BindingResource::Sampler(s),
            });
        }
        let bg = Arc::new(device.create_bind_group(&w::BindGroupDescriptor {
            label: None,
            layout: &linked.groups[g],
            entries: &entries,
        }));
        if let Some(k) = key {
            if self.bind_groups.len() >= BIND_GROUP_CACHE {
                self.bind_groups.clear();
            }
            self.bind_groups.insert(k, bg.clone());
        }
        Some(bg)
    }

    fn depth_to_float_pipeline(
        &mut self,
        multisampled: bool,
    ) -> Option<(w::RenderPipeline, w::BindGroupLayout)> {
        if let Some(p) = self.depth_to_float.get(&multisampled) {
            return Some(p.clone());
        }
        let device = self.device()?.clone();
        let (ty, load) = if multisampled {
            (
                "texture_depth_multisampled_2d",
                "textureLoad(t, vec2<i32>(p.xy), 0)",
            )
        } else {
            ("texture_depth_2d", "textureLoad(t, vec2<i32>(p.xy), 0)")
        };
        let source = format!(
            "@group(0) @binding(0) var t: {ty};\n\
             @vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{\n\
             let x = f32((i << 1u) & 2u); let y = f32(i & 2u);\n\
             return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0); }}\n\
             @fragment fn fs(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {{\n\
             return vec4<f32>({load}, 0.0, 0.0, 1.0); }}\n"
        );
        let module = device.create_shader_module(w::ShaderModuleDescriptor {
            label: Some("depth to float"),
            source: w::ShaderSource::Wgsl(source.into()),
        });
        let bgl = device.create_bind_group_layout(&w::BindGroupLayoutDescriptor {
            label: None,
            entries: &[w::BindGroupLayoutEntry {
                binding: 0,
                visibility: w::ShaderStages::FRAGMENT,
                ty: w::BindingType::Texture {
                    sample_type: w::TextureSampleType::Depth,
                    view_dimension: w::TextureViewDimension::D2,
                    multisampled,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&w::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&w::RenderPipelineDescriptor {
            label: Some("depth to float"),
            layout: Some(&layout),
            vertex: w::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: w::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: w::PrimitiveState::default(),
            depth_stencil: None,
            multisample: w::MultisampleState::default(),
            fragment: Some(w::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: w::PipelineCompilationOptions::default(),
                targets: &[Some(w::ColorTargetState {
                    format: w::TextureFormat::R32Float,
                    blend: None,
                    write_mask: w::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.depth_to_float
            .insert(multisampled, (pipeline.clone(), bgl.clone()));
        Some((pipeline, bgl))
    }

    /// Turns the recorded commands into one command buffer and submits it.
    fn flush(&mut self) -> Option<w::SubmissionIndex> {
        let dev = self.dev.as_ref()?;
        let (device, queue) = (dev.device.clone(), dev.queue.clone());
        self.uniform.upload(&device, &queue, &mut self.serials);
        self.geometry.upload(&device, &queue, &mut self.serials);
        self.upload.upload(&device, &queue, &mut self.serials);
        self.uniform_memo.clear();
        let cmds = std::mem::take(&mut self.cmds);
        self.epoch += 1;
        if cmds.is_empty() {
            return Some(queue.submit([]));
        }
        let geometry = self.geometry.buffer.clone();
        let upload = self.upload.buffer.clone();
        let mut enc = device.create_command_encoder(&w::CommandEncoderDescriptor {
            label: Some("ffx frame"),
        });
        let mut pass: Option<(w::RenderPass<'static>, (Vec<u64>, u64))> = None;
        for cmd in cmds {
            if let Cmd::Draw(d) = &cmd {
                let key = d.targets.key();
                if pass.as_ref().is_none_or(|p| p.1 != key) {
                    // the previous pass ends before the next begins
                    drop(pass.take());
                    let colors: Vec<Option<w::RenderPassColorAttachment>> = d
                        .targets
                        .colors
                        .iter()
                        .map(|c| {
                            c.as_ref().map(|(v, _)| w::RenderPassColorAttachment {
                                view: v,
                                depth_slice: None,
                                resolve_target: None,
                                ops: w::Operations {
                                    load: w::LoadOp::Load,
                                    store: w::StoreOp::Store,
                                },
                            })
                        })
                        .collect();
                    let rp = enc.begin_render_pass(&w::RenderPassDescriptor {
                        label: None,
                        color_attachments: &colors,
                        depth_stencil_attachment: d.targets.depth.as_ref().map(
                            |(v, _, stencil)| w::RenderPassDepthStencilAttachment {
                                view: v,
                                depth_ops: Some(w::Operations {
                                    load: w::LoadOp::Load,
                                    store: w::StoreOp::Store,
                                }),
                                stencil_ops: stencil.then_some(w::Operations {
                                    load: w::LoadOp::Load,
                                    store: w::StoreOp::Store,
                                }),
                            },
                        ),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    pass = Some((rp.forget_lifetime(), key));
                }
                let groups = [
                    self.bind_group(&d.linked, 0, &d.binds[0]),
                    self.bind_group(&d.linked, 1, &d.binds[1]),
                ];
                let (Some((rp, _)), [Some(g0), Some(g1)]) = (pass.as_mut(), groups) else {
                    continue;
                };
                let offsets = |s: &BindSpec| -> Vec<u32> {
                    if d.linked.dynamic {
                        s.uniforms.iter().map(|u| u.1 as u32).collect()
                    } else {
                        Vec::new()
                    }
                };
                rp.set_pipeline(&d.pipeline);
                rp.set_bind_group(0, &*g0, &offsets(&d.binds[0]));
                rp.set_bind_group(1, &*g1, &offsets(&d.binds[1]));
                let max = device.limits().max_texture_dimension_2d as f32;
                let vp = d.viewport;
                rp.set_viewport(
                    vp[0],
                    vp[1],
                    vp[2].clamp(0.0, max),
                    vp[3].clamp(0.0, max),
                    vp[4].clamp(0.0, 1.0),
                    vp[5].clamp(0.0, 1.0),
                );
                rp.set_scissor_rect(d.scissor[0], d.scissor[1], d.scissor[2], d.scissor[3]);
                rp.set_blend_constant(w::Color {
                    r: f64::from(d.blend_constant[0]),
                    g: f64::from(d.blend_constant[1]),
                    b: f64::from(d.blend_constant[2]),
                    a: f64::from(d.blend_constant[3]),
                });
                rp.set_stencil_reference(d.stencil_ref);
                let mut ok = true;
                for (slot, src) in &d.vertex {
                    match src {
                        Src::Arena(at) => match &geometry {
                            Some(g) => rp.set_vertex_buffer(*slot, g.slice(*at..)),
                            None => ok = false,
                        },
                        Src::Buffer(b, off) => rp.set_vertex_buffer(*slot, b.slice(*off..)),
                    }
                }
                if !ok {
                    continue;
                }
                match &d.call.indexed {
                    Some((src, format)) => {
                        match src {
                            Src::Arena(at) => match &geometry {
                                Some(g) => rp.set_index_buffer(g.slice(*at..), *format),
                                None => continue,
                            },
                            Src::Buffer(b, off) => rp.set_index_buffer(b.slice(*off..), *format),
                        }
                        rp.draw_indexed(
                            d.call.range.clone(),
                            d.call.base,
                            d.call.instances.clone(),
                        );
                    }
                    None => rp.draw(d.call.range.clone(), d.call.instances.clone()),
                }
                continue;
            }
            pass = None;
            match cmd {
                Cmd::Draw(_) => {}
                Cmd::ClearColor { view, color } => {
                    let _ = enc.begin_render_pass(&w::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(w::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: w::Operations {
                                load: w::LoadOp::Clear(w::Color {
                                    r: f64::from(color[0]),
                                    g: f64::from(color[1]),
                                    b: f64::from(color[2]),
                                    a: f64::from(color[3]),
                                }),
                                store: w::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                }
                Cmd::ClearDepth {
                    view,
                    depth,
                    stencil,
                    has_depth,
                    has_stencil,
                } => {
                    let _ = enc.begin_render_pass(&w::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[],
                        depth_stencil_attachment: Some(w::RenderPassDepthStencilAttachment {
                            view: &view,
                            depth_ops: has_depth.then_some(w::Operations {
                                load: depth.map_or(w::LoadOp::Load, w::LoadOp::Clear),
                                store: w::StoreOp::Store,
                            }),
                            stencil_ops: has_stencil.then_some(w::Operations {
                                load: stencil.map_or(w::LoadOp::Load, w::LoadOp::Clear),
                                store: w::StoreOp::Store,
                            }),
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                }
                Cmd::UploadBuffer {
                    at,
                    dst,
                    offset,
                    size,
                } => {
                    if let Some(u) = &upload {
                        enc.copy_buffer_to_buffer(u, at, &dst, offset, size);
                    }
                }
                Cmd::UploadTexture {
                    at,
                    bytes_per_row,
                    rows,
                    dst,
                    extent,
                } => {
                    if let Some(u) = &upload {
                        enc.copy_buffer_to_texture(
                            w::TexelCopyBufferInfo {
                                buffer: u,
                                layout: w::TexelCopyBufferLayout {
                                    offset: at,
                                    bytes_per_row: Some(bytes_per_row),
                                    rows_per_image: Some(rows),
                                },
                            },
                            w::TexelCopyTextureInfo {
                                texture: &dst.texture,
                                mip_level: dst.mip,
                                origin: dst.origin,
                                aspect: dst.aspect,
                            },
                            extent,
                        );
                    }
                }
                Cmd::CopyBuffer {
                    src,
                    src_offset,
                    dst,
                    dst_offset,
                    size,
                } => enc.copy_buffer_to_buffer(&src, src_offset, &dst, dst_offset, size),
                Cmd::CopyTexture { src, dst, extent } => enc.copy_texture_to_texture(
                    w::TexelCopyTextureInfo {
                        texture: &src.texture,
                        mip_level: src.mip,
                        origin: src.origin,
                        aspect: src.aspect,
                    },
                    w::TexelCopyTextureInfo {
                        texture: &dst.texture,
                        mip_level: dst.mip,
                        origin: dst.origin,
                        aspect: dst.aspect,
                    },
                    extent,
                ),
                Cmd::Resolve { src, dst } => {
                    let _ = enc.begin_render_pass(&w::RenderPassDescriptor {
                        label: Some("resolve"),
                        color_attachments: &[Some(w::RenderPassColorAttachment {
                            view: &src,
                            depth_slice: None,
                            resolve_target: Some(&dst),
                            ops: w::Operations {
                                load: w::LoadOp::Load,
                                store: w::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                }
                Cmd::DepthToFloat {
                    src,
                    dst,
                    multisampled,
                } => {
                    let Some((pipeline, bgl)) = self.depth_to_float_pipeline(multisampled) else {
                        continue;
                    };
                    let bg = device.create_bind_group(&w::BindGroupDescriptor {
                        label: None,
                        layout: &bgl,
                        entries: &[w::BindGroupEntry {
                            binding: 0,
                            resource: w::BindingResource::TextureView(&src),
                        }],
                    });
                    let mut rp = enc.begin_render_pass(&w::RenderPassDescriptor {
                        label: Some("depth to float"),
                        color_attachments: &[Some(w::RenderPassColorAttachment {
                            view: &dst,
                            depth_slice: None,
                            resolve_target: None,
                            ops: w::Operations {
                                load: w::LoadOp::Load,
                                store: w::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    rp.set_pipeline(&pipeline);
                    rp.set_bind_group(0, &bg, &[]);
                    rp.draw(0..3, 0..1);
                }
            }
        }
        drop(pass);
        Some(queue.submit([enc.finish()]))
    }

    /// Waits for the GPU and reads `extent` of `(mip, layer)` of `texture`
    /// back, rows packed (`row` bytes each, block rows for compressed
    /// formats).
    fn read_back(
        &mut self,
        texture: &w::Texture,
        format: w::TextureFormat,
        mip: u32,
        layer: u32,
    ) -> Option<(u32, u32, Vec<u8>)> {
        self.flush();
        let dev = self.dev.as_ref()?;
        let (bytes, bw) = formats::block(format);
        let ext = texture.size().mip_level_size(mip, texture.dimension());
        let (wd, ht) = (
            ext.width.next_multiple_of(bw),
            ext.height.next_multiple_of(bw),
        );
        let row = (wd / bw) * bytes;
        let rows = ht / bw;
        let depth = if texture.dimension() == w::TextureDimension::D3 {
            ext.depth_or_array_layers
        } else {
            1
        };
        let pitch = row.next_multiple_of(w::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = dev.device.create_buffer(&w::BufferDescriptor {
            label: Some("read-back"),
            size: u64::from(pitch * rows * depth).max(4),
            usage: w::BufferUsages::COPY_DST | w::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = dev
            .device
            .create_command_encoder(&w::CommandEncoderDescriptor::default());
        enc.copy_texture_to_buffer(
            w::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin: w::Origin3d {
                    x: 0,
                    y: 0,
                    z: if depth > 1 { 0 } else { layer },
                },
                aspect: w::TextureAspect::All,
            },
            w::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: w::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pitch),
                    rows_per_image: Some(rows),
                },
            },
            w::Extent3d {
                width: wd,
                height: ht,
                depth_or_array_layers: depth,
            },
        );
        dev.queue.submit([enc.finish()]);
        let data = map_read(&dev.device, &buffer)?;
        let mut out = Vec::with_capacity((row * rows * depth) as usize);
        for y in 0..(rows * depth) as usize {
            let at = y * pitch as usize;
            out.extend_from_slice(data.get(at..at + row as usize)?);
        }
        Some((row, rows * depth, out))
    }

    fn read_buffer(&mut self, buffer: &w::Buffer) -> Option<Vec<u8>> {
        self.flush();
        let dev = self.dev.as_ref()?;
        let rb = dev.device.create_buffer(&w::BufferDescriptor {
            label: Some("read-back"),
            size: buffer.size(),
            usage: w::BufferUsages::COPY_DST | w::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = dev
            .device
            .create_command_encoder(&w::CommandEncoderDescriptor::default());
        enc.copy_buffer_to_buffer(buffer, 0, &rb, 0, buffer.size());
        dev.queue.submit([enc.finish()]);
        map_read(&dev.device, &rb)
    }

    /// The window's surface, configured for the window's size and the
    /// game's sync interval.
    fn configure(&mut self) -> bool {
        let Some(win) = self.window.clone() else {
            return false;
        };
        let sync = self.sync;
        let Some(dev) = self.dev.as_mut() else {
            return false;
        };
        let Some(surface) = &dev.surface else {
            return false;
        };
        let (ww, wh) = win.size();
        if ww == 0 || wh == 0 {
            return false;
        }
        let caps = surface.get_capabilities(&dev.adapter);
        let mode = if sync > 0 {
            w::PresentMode::Fifo
        } else if caps.present_modes.contains(&w::PresentMode::Immediate) {
            w::PresentMode::Immediate
        } else if caps.present_modes.contains(&w::PresentMode::Mailbox) {
            w::PresentMode::Mailbox
        } else {
            w::PresentMode::Fifo
        };
        let want = dev
            .config
            .as_ref()
            .is_none_or(|c| c.width != ww || c.height != wh || c.present_mode != mode);
        if !want {
            return true;
        }
        let Some(&format) = caps
            .formats
            .iter()
            .find(|f| {
                matches!(
                    f,
                    w::TextureFormat::Bgra8Unorm | w::TextureFormat::Rgba8Unorm
                )
            })
            .or_else(|| caps.formats.first())
        else {
            return false;
        };
        let config = w::SurfaceConfiguration {
            usage: w::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: w::SurfaceColorSpace::Auto,
            width: ww,
            height: wh,
            present_mode: mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(w::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };
        surface.configure(&dev.device, &config);
        dev.config = Some(config);
        if dev.blit.as_ref().is_none_or(|b| b.0 != format) {
            dev.blit = Some(make_blit(&dev.device, format));
        }
        true
    }

    /// Shows the back buffer on the window.
    fn show(&mut self) {
        if !self.configure() {
            return;
        }
        let Some(back) = self.back.clone() else {
            return;
        };
        let Some(dev) = self.dev.as_ref() else {
            return;
        };
        let (Some(surface), Some((_, pipeline, bgl, sampler))) = (&dev.surface, &dev.blit) else {
            return;
        };
        let frame = match surface.get_current_texture() {
            w::CurrentSurfaceTexture::Success(t) | w::CurrentSurfaceTexture::Suboptimal(t) => t,
            w::CurrentSurfaceTexture::Outdated | w::CurrentSurfaceTexture::Lost => {
                if let Some(d) = self.dev.as_mut() {
                    d.config = None;
                }
                return;
            }
            _ => return,
        };
        let src = back.create_view(&w::TextureViewDescriptor {
            format: Some(back.format().remove_srgb_suffix()),
            ..Default::default()
        });
        let dst = frame
            .texture
            .create_view(&w::TextureViewDescriptor::default());
        let bg = dev.device.create_bind_group(&w::BindGroupDescriptor {
            label: None,
            layout: bgl,
            entries: &[
                w::BindGroupEntry {
                    binding: 0,
                    resource: w::BindingResource::TextureView(&src),
                },
                w::BindGroupEntry {
                    binding: 1,
                    resource: w::BindingResource::Sampler(sampler),
                },
            ],
        });
        let mut enc = dev
            .device
            .create_command_encoder(&w::CommandEncoderDescriptor {
                label: Some("present"),
            });
        {
            let mut rp = enc.begin_render_pass(&w::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(w::RenderPassColorAttachment {
                    view: &dst,
                    depth_slice: None,
                    resolve_target: None,
                    ops: w::Operations {
                        load: w::LoadOp::Clear(w::Color::BLACK),
                        store: w::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, &bg, &[]);
            rp.draw(0..3, 0..1);
        }
        dev.queue.submit([enc.finish()]);
        dev.queue.present(frame);
    }
}

/// Maps `buffer` for reading and copies it out.
fn map_read(device: &w::Device, buffer: &w::Buffer) -> Option<Vec<u8>> {
    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(w::MapMode::Read, move |r| {
        let _ = tx.send(r.is_ok());
    });
    device.poll(w::PollType::wait_indefinitely()).ok()?;
    if !rx.recv().unwrap_or(false) {
        return None;
    }
    let data = slice.get_mapped_range().ok()?.to_vec();
    buffer.unmap();
    Some(data)
}

fn view_dimension(d: Dim) -> w::TextureViewDimension {
    match d {
        Dim::TextureCube => w::TextureViewDimension::Cube,
        Dim::Texture3D => w::TextureViewDimension::D3,
        Dim::Texture1D => w::TextureViewDimension::D1,
        _ => w::TextureViewDimension::D2,
    }
}

fn group_layout(
    device: &w::Device,
    shader: &wgsl::Shader,
    visibility: w::ShaderStages,
    dynamic: bool,
) -> w::BindGroupLayout {
    let mut entries = Vec::new();
    for &(slot, size) in &shader.buffers {
        entries.push(w::BindGroupLayoutEntry {
            binding: slot,
            visibility,
            ty: w::BindingType::Buffer {
                ty: w::BufferBindingType::Uniform,
                has_dynamic_offset: dynamic,
                min_binding_size: w::BufferSize::new(u64::from(size.max(16)).next_multiple_of(16)),
            },
            count: None,
        });
    }
    for t in &shader.textures {
        let sample_type = match t.kind {
            TextureKind::Float => w::TextureSampleType::Float { filterable: true },
            TextureKind::Multisampled => w::TextureSampleType::Float { filterable: false },
            TextureKind::Uint => w::TextureSampleType::Uint,
            TextureKind::Sint => w::TextureSampleType::Sint,
            TextureKind::Depth => w::TextureSampleType::Depth,
        };
        entries.push(w::BindGroupLayoutEntry {
            binding: wgsl::TEXTURE_BINDING + t.slot,
            visibility,
            ty: w::BindingType::Texture {
                sample_type,
                view_dimension: view_dimension(t.dim),
                multisampled: t.kind == TextureKind::Multisampled,
            },
            count: None,
        });
    }
    for &(slot, comparison) in &shader.samplers {
        entries.push(w::BindGroupLayoutEntry {
            binding: wgsl::SAMPLER_BINDING + slot,
            visibility,
            ty: w::BindingType::Sampler(if comparison {
                w::SamplerBindingType::Comparison
            } else {
                w::SamplerBindingType::Filtering
            }),
            count: None,
        });
    }
    device.create_bind_group_layout(&w::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    })
}

/// `format` cut down to `room` bytes (0: no limit), keeping its type.
fn shrink_format(format: w::VertexFormat, room: u32) -> w::VertexFormat {
    use w::VertexFormat as V;
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

/// The present pass: the back buffer stretched over the surface, linear.
fn make_blit(
    device: &w::Device,
    format: w::TextureFormat,
) -> (
    w::TextureFormat,
    w::RenderPipeline,
    w::BindGroupLayout,
    w::Sampler,
) {
    let module = device.create_shader_module(w::ShaderModuleDescriptor {
        label: Some("present"),
        source: w::ShaderSource::Wgsl(
            "@group(0) @binding(0) var t: texture_2d<f32>;\n\
             @group(0) @binding(1) var s: sampler;\n\
             struct V { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32> }\n\
             @vertex fn vs(@builtin(vertex_index) i: u32) -> V {\n\
             let x = f32((i << 1u) & 2u); let y = f32(i & 2u);\n\
             var o: V; o.p = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0); o.uv = vec2<f32>(x, y); return o; }\n\
             @fragment fn fs(v: V) -> @location(0) vec4<f32> {\n\
             return vec4<f32>(textureSampleLevel(t, s, v.uv, 0.0).rgb, 1.0); }\n"
                .into(),
        ),
    });
    let bgl = device.create_bind_group_layout(&w::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            w::BindGroupLayoutEntry {
                binding: 0,
                visibility: w::ShaderStages::FRAGMENT,
                ty: w::BindingType::Texture {
                    sample_type: w::TextureSampleType::Float { filterable: true },
                    view_dimension: w::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            w::BindGroupLayoutEntry {
                binding: 1,
                visibility: w::ShaderStages::FRAGMENT,
                ty: w::BindingType::Sampler(w::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let layout = device.create_pipeline_layout(&w::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&w::RenderPipelineDescriptor {
        label: Some("present"),
        layout: Some(&layout),
        vertex: w::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: w::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: w::PrimitiveState::default(),
        depth_stencil: None,
        multisample: w::MultisampleState::default(),
        fragment: Some(w::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: w::PipelineCompilationOptions::default(),
            targets: &[Some(w::ColorTargetState {
                format,
                blend: None,
                write_mask: w::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let sampler = device.create_sampler(&w::SamplerDescriptor {
        mag_filter: w::FilterMode::Linear,
        min_filter: w::FilterMode::Linear,
        ..Default::default()
    });
    (format, pipeline, bgl, sampler)
}

fn put(out: &mut [u8], at: usize, b: &[u8]) {
    let n = b.len().min(out.len().saturating_sub(at));
    out[at..at + n].copy_from_slice(&b[..n]);
}

impl Backend for Wgpu {
    fn name(&self) -> &'static str {
        "wgpu"
    }

    fn create_device(&mut self, desc: &SwapChainDesc, _flags: u32) -> u32 {
        match self.open(desc) {
            Ok(()) => S_OK,
            Err(e) => {
                self.fail(e);
                // DXGI_ERROR_UNSUPPORTED
                0x887a_0004
            }
        }
    }

    fn back_buffer(&mut self, id: u32) -> u32 {
        if self.objects.contains_key(&id) {
            return S_OK;
        }
        self.back_buffer = id;
        // the swap chain's description as a texture: render target and
        // shader resource, the game's size (1920 x 1080 when it gives
        // none, as the D3D11 back end's offscreen buffer)
        let sc = SwapChainDesc(self.swap_desc);
        let mut d = sc.back_buffer();
        if d[0] == 0 || d[1] == 0 {
            let (ww, wh) = self.window.as_ref().map_or((1920, 1080), |w| w.size());
            d[0] = if ww == 0 { 1920 } else { ww };
            d[1] = if wh == 0 { 1080 } else { wh };
        }
        if d[4] == 0 {
            d[4] = 28;
        }
        d[8] |= 0x28;
        let r = self.make_texture(id, 2, &d, &[]);
        if let Some(Obj::Tex(t)) = self.objects.get(&id) {
            self.back = Some(t.texture.clone());
        }
        r
    }

    fn create(&mut self, id: u32, what: &Create<'_>) -> u32 {
        if self.dev.is_none() {
            return E_FAIL;
        }
        match what {
            Create::Buffer { desc, init } => self.make_buffer(id, *desc, *init),
            Create::Texture { dims, desc, init } => self.make_texture(id, *dims, desc, init),
            Create::View {
                class,
                resource,
                desc,
            } => self.make_view(id, *class, *resource, *desc),
            Create::Shader { stage, bytecode } => {
                if !matches!(stage, b'v' | b'p') {
                    self.fail(format!("a {} shader", *stage as char));
                }
                self.objects.insert(
                    id,
                    Obj::Shader(ShaderObj {
                        bytecode: Arc::from(*bytecode),
                        hash: ffx_dxbc::hash(bytecode),
                    }),
                );
                S_OK
            }
            Create::Layout { elements, .. } => {
                let key = self.intern(4, format!("{elements:?}").as_bytes());
                self.objects.insert(id, Obj::Layout(elements.to_vec(), key));
                S_OK
            }
            Create::State { kind, desc } => {
                let k = match kind {
                    StateKind::Blend => 0,
                    StateKind::Depth => 1,
                    StateKind::Raster => 2,
                    StateKind::Sampler => 3,
                };
                let key = self.intern(k, desc);
                self.objects
                    .insert(id, Obj::State(*kind, desc.to_vec(), key));
                S_OK
            }
            Create::Query(q) => {
                self.queries.insert(id, q[0]);
                self.objects.insert(id, Obj::Query);
                S_OK
            }
        }
    }

    fn release(&mut self, id: u32) {
        if let Some(Obj::Tex(t)) = self.objects.remove(&id) {
            self.depth_copies.remove(&t.serial);
        }
        self.queries.remove(&id);
    }

    fn op(&mut self, op: &Op<'_>) {
        let b = &mut self.bound;
        let stage = |s: u8| match s {
            b'v' => Some(0usize),
            b'p' => Some(1),
            _ => None,
        };
        match op {
            Op::InputLayout(l) => b.layout = *l,
            Op::VertexBuffers(start, bufs, strides, offsets) => {
                for (i, &id) in bufs.iter().enumerate() {
                    if let Some(slot) = b.vertex.get_mut(*start as usize + i) {
                        *slot = (
                            id,
                            strides.get(i).copied().unwrap_or(0),
                            offsets.get(i).copied().unwrap_or(0),
                        );
                    }
                }
            }
            Op::IndexBuffer(id, f, o) => b.index = (*id, *f, *o),
            Op::Topology(t) => b.topology = *t,
            Op::Shader(s, id) => match s {
                b'v' => b.vs = *id,
                b'p' => b.ps = *id,
                b'g' => b.gs = *id,
                _ => {}
            },
            Op::ConstantBuffers(s, start, bufs, ranges) => {
                if let Some(g) = stage(*s) {
                    for (i, &id) in bufs.iter().enumerate() {
                        let range = ranges.map_or((0, 0), |(f, n)| {
                            (
                                f.get(i).copied().unwrap_or(0),
                                n.get(i).copied().unwrap_or(0),
                            )
                        });
                        if let Some(slot) = b.cb[g].get_mut(*start as usize + i) {
                            *slot = (id, range.0, range.1);
                        }
                    }
                }
            }
            Op::Resources(s, start, views) => {
                if let Some(g) = stage(*s) {
                    for (i, &id) in views.iter().enumerate() {
                        if let Some(slot) = b.srv[g].get_mut(*start as usize + i) {
                            *slot = id;
                        }
                    }
                }
            }
            Op::Samplers(s, start, samplers) => {
                if let Some(g) = stage(*s) {
                    for (i, &id) in samplers.iter().enumerate() {
                        if let Some(slot) = b.sampler[g].get_mut(*start as usize + i) {
                            *slot = id;
                        }
                    }
                }
            }
            Op::Targets(views, dsv) => {
                b.rtv = [0; 8];
                for (slot, &v) in b.rtv.iter_mut().zip(views.iter()) {
                    *slot = v;
                }
                b.dsv = *dsv;
            }
            Op::Blend(s, f, m) => {
                b.blend = *s;
                b.blend_factor = *f;
                b.sample_mask = *m;
            }
            Op::Depth(s, r) => {
                b.depth = *s;
                b.stencil_ref = *r;
            }
            Op::Raster(s) => b.raster = *s,
            Op::Viewports(v) => {
                if let Some(v) = v.first() {
                    b.viewport = *v;
                }
            }
            Op::Scissors(v) => {
                if let Some(v) = v.first() {
                    b.scissor = *v;
                }
            }
            Op::Draw(kind, p) => self.draw(*kind, *p),
            Op::ClearTarget(v, c) => self.clear_target(*v, *c),
            Op::ClearDepth(v, f, d, s) => self.clear_depth(*v, *f, *d, *s),
            Op::Copy(d, s) => self.copy(*d, *s),
            Op::Resolve(a) => self.resolve(*a),
            Op::CopyRegion(a, bx) => self.copy_region(*a, *bx),
            Op::Update(id, sub, bx, data, row, depth) => match self.objects.get(id) {
                Some(Obj::Buf(_)) => {
                    let off = bx.map_or(0, |b| b[0]);
                    self.write_buffer(*id, off, data);
                }
                Some(Obj::Tex(_)) => self.write_texture(*id, *sub, *bx, data, *row, *depth),
                _ => {}
            },
            Op::Unmapped(id, sub, row, depth, data, spans) => match self.objects.get(id) {
                Some(Obj::Buf(_)) => match spans {
                    Some(spans) => {
                        for &(off, n) in *spans {
                            let (o, n) = (off as usize, n as usize);
                            if let Some(bytes) = data.get(o..o + n) {
                                let bytes = bytes.to_vec();
                                self.write_buffer(*id, off, &bytes);
                            }
                        }
                    }
                    None => self.write_buffer(*id, 0, data),
                },
                Some(Obj::Tex(_)) => self.write_texture(*id, *sub, None, data, *row, *depth),
                _ => {}
            },
            Op::Begin(_) | Op::End(_) => {}
            Op::GenerateMips(_) => self.fail("GenerateMips (not drawn)"),
            Op::ClearState => self.bound = Bound::default(),
            Op::Flush => {
                self.flush();
            }
        }
    }

    fn read(&mut self, id: u32, sub: u32, row: u32, out: &mut [u8]) -> bool {
        match self.objects.get(&id) {
            Some(Obj::Buf(b)) => {
                if let Some(s) = &b.shadow
                    && b.buffer.is_none()
                {
                    put(out, 0, s);
                    return true;
                }
                let Some(buffer) = b.buffer.clone() else {
                    return false;
                };
                match self.read_buffer(&buffer) {
                    Some(data) => {
                        put(out, 0, &data);
                        true
                    }
                    None => false,
                }
            }
            Some(Obj::Tex(t)) => {
                if t.desc.samples > 1 || t.format.is_depth_stencil_format() {
                    return false;
                }
                let (texture, format, d) = (t.texture.clone(), t.format, t.desc);
                let (mip, layer) = if d.dims == 3 {
                    (sub, 0)
                } else {
                    (sub % d.mips, sub / d.mips)
                };
                let Some((tight, rows, data)) = self.read_back(&texture, format, mip, layer) else {
                    return false;
                };
                let row = if row == 0 { tight } else { row } as usize;
                for y in 0..rows as usize {
                    let src = &data[y * tight as usize..(y + 1) * tight as usize];
                    put(out, y * row, &src[..src.len().min(row)]);
                }
                true
            }
            _ => false,
        }
    }

    fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32 {
        out.fill(0);
        match self.queries.get(&id) {
            // EVENT: done (the commands before it are submitted)
            Some(0) => {
                self.flush();
                put(out, 0, &1u32.to_le_bytes());
            }
            // TIMESTAMP: a 1 GHz clock, a millisecond a query
            Some(2) => {
                self.clock += 1_000_000;
                put(out, 0, &self.clock.to_le_bytes());
            }
            // TIMESTAMP_DISJOINT: 1 GHz, not disjoint
            Some(3) => put(out, 0, &1_000_000_000u64.to_le_bytes()),
            Some(_) => {}
            None => return E_FAIL,
        }
        let _ = S_FALSE;
        S_OK
    }

    fn present(&mut self, sync: u32, _flags: u32) -> u32 {
        self.sync = sync;
        let Some(index) = self.flush() else {
            return S_OK;
        };
        self.show();
        self.in_flight.push_back(index);
        if self.in_flight.len() > FRAMES_AHEAD
            && let (Some(old), Some(dev)) = (self.in_flight.pop_front(), self.dev.as_ref())
        {
            let _ = dev.device.poll(w::PollType::Wait {
                submission_index: Some(old),
                timeout: None,
            });
        }
        if let Some(dev) = self.dev.as_ref() {
            let _ = dev.device.poll(w::PollType::Poll);
        }
        S_OK
    }

    fn back_buffer_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        let back = self.back.clone()?;
        let format = back.format();
        let (row, rows, mut data) = self.read_back(&back, format, 0, 0)?;
        let wd = back.width();
        let mut out = Vec::with_capacity((wd * 4 * rows) as usize);
        for y in 0..rows as usize {
            out.extend_from_slice(&data[y * row as usize..y * row as usize + (wd * 4) as usize]);
        }
        data = out;
        if matches!(
            format,
            w::TextureFormat::Rgba8Unorm | w::TextureFormat::Rgba8UnormSrgb
        ) {
            for px in data.as_chunks_mut::<4>().0 {
                px.swap(0, 2);
            }
        }
        Some((wd, back.height(), data))
    }

    fn read_texture(&mut self, id: u32, desc: &[u32; 11]) -> Option<Vec<Subresource>> {
        let Some(Obj::Tex(t)) = self.objects.get(&id) else {
            return None;
        };
        if desc[5] > 1 || t.desc.samples > 1 || t.format.is_depth_stencil_format() {
            return Some(Vec::new());
        }
        let (texture, format) = (t.texture.clone(), t.format);
        let (mips, slices) = (
            desc[2].max(1).min(texture.mip_level_count()),
            desc[3].max(1),
        );
        let mut out = Vec::new();
        for slice in 0..slices {
            for mip in 0..mips {
                let Some((row, _, data)) = self.read_back(&texture, format, mip, slice) else {
                    return Some(out);
                };
                out.push(Subresource {
                    row_pitch: row,
                    data,
                });
            }
        }
        Some(out)
    }

    fn object(&self, id: u32) -> Option<usize> {
        // Direct3D 11 hands out one state object for the descriptions it
        // counts as the same; the interned key does the same here
        match self.objects.get(&id) {
            Some(Obj::State(_, _, key)) => Some(*key as usize),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A device on any adapter (lavapipe on a machine without a GPU);
    /// `None` skips the test where there is none.
    fn device() -> Option<Wgpu> {
        let mut g = Wgpu::new(None);
        let mut desc = [0u32; 15];
        desc[0] = 16;
        desc[1] = 8;
        desc[4] = 28;
        desc[7] = 1;
        desc[9] = 0x20;
        (g.create_device(&SwapChainDesc(desc), 0) == S_OK).then_some(g)
    }

    #[test]
    fn a_cleared_back_buffer_reads_back_as_bgra() {
        let Some(mut g) = device() else {
            eprintln!("no wgpu adapter: skipped");
            return;
        };
        assert_eq!(g.back_buffer(1), S_OK);
        let rtv = Create::View {
            class: class::RTV,
            resource: 1,
            desc: None,
        };
        assert_eq!(g.create(2, &rtv), S_OK);
        g.op(&Op::ClearTarget(2, [1.0, 0.0, 0.0, 1.0]));
        let (w, h, px) = g.back_buffer_image().expect("an image");
        assert_eq!((w, h), (16, 8));
        assert_eq!(&px[..4], &[0, 0, 255, 255]);
        assert!(g.problems().is_empty(), "{:?}", g.problems());
    }

    #[test]
    fn writes_between_copies_keep_their_order() {
        let Some(mut g) = device() else {
            eprintln!("no wgpu adapter: skipped");
            return;
        };
        // two 4 x 4 RGBA8 textures; a copy of the first, then a write to
        // it: the copy must see the bytes from before the write
        let desc = [4, 4, 1, 1, 28, 1, 0, 0, 0x8, 0, 0];
        let init = [(16, 0, vec![7u8; 64])];
        assert_eq!(
            g.create(
                10,
                &Create::Texture {
                    dims: 2,
                    desc: &desc,
                    init: &init
                }
            ),
            S_OK
        );
        assert_eq!(
            g.create(
                11,
                &Create::Texture {
                    dims: 2,
                    desc: &desc,
                    init: &[]
                }
            ),
            S_OK
        );
        g.op(&Op::Copy(11, 10));
        g.op(&Op::Update(10, 0, None, &[9u8; 64], 16, 0));
        let mut out = vec![0u8; 64];
        assert!(g.read(11, 0, 16, &mut out));
        assert!(out.iter().all(|&b| b == 7), "{out:?}");
        assert!(g.read(10, 0, 16, &mut out));
        assert!(out.iter().all(|&b| b == 9), "{out:?}");
    }
}
