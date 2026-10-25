//! The graphics layer: intercepts the game's Direct3D 11 device and
//! immediate context, keeps track of what it creates and binds, and can
//! capture one frame's calls to disk (`capture`) for study.
//!
//! On with `FFX_GFX=1` (or whenever the external renderer is on). The
//! hooks patch the shared vtables of the device and context the game gets
//! from `D3D11CreateDeviceAndSwapChain`, so the Iggy (Flash) renderer,
//! which uses the same device, goes through them too.

// Local `type F = …` aliases sit next to the calls that use them.
#![allow(clippy::items_after_statements)]

pub mod capture;
pub mod live;
pub mod map_build;
mod proxy;
pub mod replay;
pub mod scene;
pub mod vt;

use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use crate::log::log;

/// Calls COM method `index` of `object`.
macro_rules! com {
    ($object:expr, $index:expr, $ty:ty $(, $arg:expr)*) => {{
        let object: *mut c_void = $object;
        let vtable = *(object as *const *const usize);
        let f: $ty = std::mem::transmute(*vtable.add($index));
        f(object $(, $arg)*)
    }};
}
pub(crate) use com;

/// Set once the hooks are in.
pub static ENABLED: AtomicBool = AtomicBool::new(false);
static CREATE_DEVICE_AND_SWAP_CHAIN: AtomicUsize = AtomicUsize::new(0);
static CREATE_DEVICE: AtomicUsize = AtomicUsize::new(0);
pub(crate) static DEVICE: AtomicUsize = AtomicUsize::new(0);
pub(crate) static CONTEXT: AtomicUsize = AtomicUsize::new(0);
static FRAME: AtomicU32 = AtomicU32::new(0);
/// Microseconds spent in the layer's own work since the last report:
/// [draw hooks, unmap copies, update copies].
pub(crate) static SPENT: [AtomicU32; 3] = [const { AtomicU32::new(0) }; 3];
static DRAWS: AtomicU32 = AtomicU32::new(0);
static LAST_DRAWS: AtomicU32 = AtomicU32::new(0);

/// One hooked vtable and the methods it held before.
struct Table {
    vtable: AtomicUsize,
    originals: [AtomicUsize; SLOTS],
}

const SLOTS: usize = 140;
const TABLES: usize = 8;

/// Device and context vtables: an object's interfaces (`ID3D11Device`,
/// `ID3D11Device1`, …) can each have their own.
static DEVICE_TABLES: [Table; TABLES] = [const {
    Table {
        vtable: AtomicUsize::new(0),
        originals: [const { AtomicUsize::new(0) }; SLOTS],
    }
}; TABLES];
static PRESENT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);

fn original(tables: &[Table], this: *mut c_void, slot: usize) -> usize {
    // SAFETY: `this` is a live COM object, which starts with its vtable.
    let vtable = unsafe { *(this as *const usize) };
    tables
        .iter()
        .find(|t| t.vtable.load(Ordering::Acquire) == vtable)
        .or(tables.first())
        .map_or(0, |t| t.originals[slot].load(Ordering::Acquire))
}

fn device_original(this: *mut c_void, slot: usize) -> usize {
    original(&DEVICE_TABLES, this, slot)
}

/// The real object behind a context proxy, and its method `slot` (read
/// from its vtable now: the runtime swaps entries of the immediate
/// context's vtable as it goes).
fn forward(this: *mut c_void, slot: usize) -> (*mut c_void, usize) {
    let real = proxy::real(this);
    (real, real_method(real, slot))
}

/// Method `slot` of a real (not proxied) object.
pub(crate) fn real_method(object: *mut c_void, slot: usize) -> usize {
    // SAFETY: a live COM object starts with its vtable.
    unsafe { *(*(object as *const *const usize)).add(slot) }
}

/// Takes a free table for `vtable`, or `None` when it is hooked already
/// (or there is no room).
fn claim(tables: &'static [Table], vtable: usize) -> Option<&'static Table> {
    if tables
        .iter()
        .any(|t| t.vtable.load(Ordering::Acquire) == vtable)
    {
        return None;
    }
    tables.iter().find(|t| {
        t.vtable
            .compare_exchange(0, vtable, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    })
}

type Guid = [u8; 16];

pub(crate) const fn guid(d1: u32, d2: u16, d3: u16, d4: [u8; 8]) -> Guid {
    let a = d1.to_le_bytes();
    let b = d2.to_le_bytes();
    let c = d3.to_le_bytes();
    [
        a[0], a[1], a[2], a[3], b[0], b[1], c[0], c[1], d4[0], d4[1], d4[2], d4[3], d4[4], d4[5],
        d4[6], d4[7],
    ]
}

/// `ID3D11DeviceContext` to `ID3D11DeviceContext4`; from 1 on they have the
/// `…1` methods.
pub(crate) const CONTEXT_IIDS: [Guid; 5] = [
    guid(
        0xc0bf_a96c,
        0xe089,
        0x44fb,
        [0x8e, 0xaf, 0x26, 0xf8, 0x79, 0x61, 0x90, 0xda],
    ),
    guid(
        0xbb2c_6faa,
        0xb5fb,
        0x4082,
        [0x8e, 0x6b, 0x38, 0x8b, 0x8c, 0xfa, 0x90, 0xe1],
    ),
    guid(
        0x420d_5b32,
        0xb90c,
        0x4da4,
        [0xbe, 0xf0, 0x35, 0x9f, 0x6a, 0x24, 0xa8, 0x3a],
    ),
    guid(
        0xb4e3_c01d,
        0xe79e,
        0x4637,
        [0x91, 0xb2, 0x51, 0x0e, 0x9f, 0x4c, 0x9b, 0x8f],
    ),
    guid(
        0x9176_00da,
        0xf58c,
        0x4c33,
        [0x98, 0xd8, 0x3e, 0x15, 0xb3, 0x90, 0xfa, 0x24],
    ),
];
/// `ID3D11Device` to `ID3D11Device5`.
pub(crate) const DEVICE_IIDS: [Guid; 6] = [
    guid(
        0xdb6f_6ddb,
        0xac77,
        0x4e88,
        [0x82, 0x53, 0x81, 0x9d, 0xf9, 0xbb, 0xf1, 0x40],
    ),
    guid(
        0xa04b_fb29,
        0x08ef,
        0x43d6,
        [0xa4, 0x9c, 0xa9, 0xbd, 0xbd, 0xcb, 0xe6, 0x86],
    ),
    guid(
        0x9d06_dffa,
        0xd1e5,
        0x4d07,
        [0x83, 0xa8, 0x1b, 0xb1, 0x23, 0xf2, 0xf8, 0x41],
    ),
    guid(
        0xa05c_8c37,
        0xd2c6,
        0x4732,
        [0xb3, 0xa0, 0x9c, 0xe0, 0xb0, 0xdc, 0x9a, 0xe6],
    ),
    guid(
        0x8992_ab71,
        0x02e6,
        0x4b8d,
        [0xba, 0x48, 0xb0, 0x56, 0xdc, 0xda, 0x42, 0xc4],
    ),
    guid(
        0x8ffd_e202,
        0xa0e7,
        0x45df,
        [0x9e, 0x01, 0xe8, 0x37, 0x80, 0x1b, 0x5e, 0xa0],
    ),
];

/// Hooks the vtables of every interface of the device and the context.
unsafe fn adopt(device: *mut c_void, context: *mut c_void) {
    // Contexts are proxied (`proxy`), not hooked.
    type Qi = unsafe extern "system" fn(*mut c_void, *const u8, *mut *mut c_void) -> i32;
    // SAFETY: live COM objects; QueryInterface is slot 0, Release slot 2.
    unsafe {
        for (i, iid) in DEVICE_IIDS.iter().enumerate() {
            let mut out: *mut c_void = std::ptr::null_mut();
            let qi: Qi = std::mem::transmute(**device.cast::<*const usize>());
            if qi(device, iid.as_ptr(), &raw mut out) >= 0 && !out.is_null() {
                let vtable = *out.cast::<*mut usize>();
                if let Some(t) = claim(&DEVICE_TABLES, vtable as usize) {
                    hook_device(vtable, t);
                    log(&format!(
                        "  gfx: ID3D11Device{} vtable {:#x} hooked",
                        if i == 0 { String::new() } else { i.to_string() },
                        vtable as usize
                    ));
                }
                com!(
                    out,
                    vt::RELEASE,
                    unsafe extern "system" fn(*mut c_void) -> u32
                );
            }
        }
        let _ = context;
    }
}

/// What a buffer was created as, and for constant buffers their latest
/// contents.
#[derive(Debug, Clone, Default)]
pub struct Buffer {
    /// `D3D11_BUFFER_DESC`: width, usage, bind, CPU access, misc, stride.
    pub desc: [u32; 6],
    pub shadow: Option<Vec<u8>>,
    /// Bumped on every write the layer sees.
    pub version: u32,
    /// A scene draw for the live renderer reads it: keep its contents.
    pub wanted: bool,
    /// Where the last write map put it (dynamic buffers keep that memory
    /// mapped; the live link reads the bytes a draw uses from there).
    pub mapped_ptr: usize,
}

pub const BIND_VERTEX: u32 = 0x1;
pub const BIND_INDEX: u32 = 0x2;
pub const BIND_CONSTANT: u32 = 0x4;

#[derive(Debug, Clone)]
pub struct Texture {
    /// `D3D11_TEXTURE2D_DESC`.
    pub desc: [u32; 11],
    /// Bumped on every write from the CPU or a copy the layer sees.
    pub version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewKind {
    ShaderResource,
    RenderTarget,
    DepthStencil,
}

#[derive(Debug, Clone)]
pub struct View {
    pub kind: ViewKind,
    pub resource: usize,
    /// The view description, when the game gave one (format, dimension,
    /// then the dimension's fields).
    pub desc: Option<[u32; 6]>,
}

#[derive(Debug, Clone)]
pub struct Shader {
    /// `v`ertex, `p`ixel, `g`eometry or `c`ompute.
    pub kind: char,
    pub bytecode: Arc<[u8]>,
    pub hash: u64,
}

#[derive(Debug, Clone)]
pub struct Element {
    pub semantic: String,
    pub index: u32,
    pub format: u32,
    pub slot: u32,
    pub offset: u32,
    pub per_instance: bool,
    pub step: u32,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub elements: Vec<Element>,
    /// Hash of the vertex shader signature it was made for.
    pub signature: u64,
}

/// What is bound on the immediate context.
#[derive(Debug, Clone, Default)]
pub struct Bound {
    pub vs: usize,
    pub ps: usize,
    pub layout: usize,
    pub topology: u32,
    /// (buffer, stride, offset) per slot.
    pub vertex: [(usize, u32, u32); 8],
    /// (buffer, format, offset).
    pub index: (usize, u32, u32),
    pub vs_cb: [usize; 14],
    pub ps_cb: [usize; 14],
    /// (first constant, constants) per slot from the `…1` setters, in
    /// 16-byte units; (0, 0) is the whole buffer.
    pub vs_cb_range: [(u32, u32); 14],
    pub ps_cb_range: [(u32, u32); 14],
    pub vs_srv: [usize; 8],
    pub ps_srv: [usize; 16],
    pub ps_sampler: [usize; 16],
    pub vs_sampler: [usize; 4],
    pub rtv: [usize; 8],
    pub dsv: usize,
    pub blend: usize,
    pub blend_factor: [f32; 4],
    pub sample_mask: u32,
    pub depth: usize,
    pub stencil_ref: u32,
    pub raster: usize,
    pub viewport: [f32; 6],
    pub scissor: [i32; 4],
}

/// Everything the layer knows.
#[derive(Default)]
pub struct State {
    pub buffers: HashMap<usize, Buffer>,
    pub textures: HashMap<usize, Texture>,
    pub views: HashMap<usize, View>,
    pub shaders: HashMap<usize, Shader>,
    pub layouts: HashMap<usize, Layout>,
    /// Blend, depth-stencil, rasterizer and sampler descriptions, raw.
    pub states: HashMap<usize, (char, Vec<u8>)>,
    pub bound: Bound,
    /// Mapped resources: resource → (subresource, data pointer).
    mapped: HashMap<usize, (u32, usize)>,
    pub capture: Option<capture::Capture>,
    /// Addresses a new object took over since the live link last looked.
    pub reborn: Vec<usize>,
}

static STATE: OnceLock<Mutex<State>> = OnceLock::new();

impl State {
    /// Marks a texture (or buffer) as written.
    pub fn touch(&mut self, id: usize) {
        if let Some(t) = self.textures.get_mut(&id) {
            t.version = t.version.wrapping_add(1);
        } else if let Some(b) = self.buffers.get_mut(&id) {
            b.version = b.version.wrapping_add(1);
        }
    }

    /// Marks the resource behind a view as written.
    pub fn touch_view(&mut self, view: usize) {
        if let Some(r) = self.views.get(&view).map(|v| v.resource) {
            self.touch(r);
        }
    }

    /// Drops what was known about `id`: a new object at a freed address
    /// may be of another kind.
    fn forget(&mut self, id: usize) {
        self.reborn.push(id);
        self.buffers.remove(&id);
        self.textures.remove(&id);
        self.views.remove(&id);
        self.shaders.remove(&id);
        self.layouts.remove(&id);
        self.states.remove(&id);
    }
}

pub fn state() -> MutexGuard<'static, State> {
    STATE
        .get_or_init(|| Mutex::new(State::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// FNV-1a, for shader bytecode.
#[must_use]
pub fn hash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Turns the layer on when `FFX_GFX` is set.
///
/// # Safety
/// `module` must be the main module, before other threads run game code.
pub unsafe fn install(module: usize) {
    let wanted = |v: &str| std::env::var(v).is_ok_and(|v| !v.is_empty() && v != "0");
    if !wanted("FFX_GFX") && !wanted("FFX_RENDERER") {
        return;
    }
    // SAFETY: forwarded from the caller.
    match unsafe {
        crate::iat::patch(
            module,
            "d3d11.dll",
            "D3D11CreateDeviceAndSwapChain",
            create_device_and_swap_chain as *const () as usize,
        )
    } {
        Some(original) => {
            CREATE_DEVICE_AND_SWAP_CHAIN.store(original, Ordering::Release);
            ENABLED.store(true, Ordering::Release);
            log("  gfx: D3D11CreateDeviceAndSwapChain patched");
        }
        None => log("  gfx: D3D11CreateDeviceAndSwapChain not imported"),
    }
    // SAFETY: forwarded from the caller.
    if let Some(original) = unsafe {
        crate::iat::patch(
            module,
            "d3d11.dll",
            "D3D11CreateDevice",
            create_device as *const () as usize,
        )
    } {
        CREATE_DEVICE.store(original, Ordering::Release);
        log("  gfx: D3D11CreateDevice patched");
    }
    // SAFETY: forwarded from the caller.
    unsafe { scene::install(module) };
}

type CreateDeviceFn = unsafe extern "system" fn(
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *const u32,
    u32,
    u32,
    *mut *mut c_void,
    *mut u32,
    *mut *mut c_void,
) -> i32;

#[allow(clippy::too_many_arguments)] // the D3D11 signature
unsafe extern "system" fn create_device(
    adapter: *mut c_void,
    driver: u32,
    software: *mut c_void,
    flags: u32,
    levels: *const u32,
    level_count: u32,
    sdk: u32,
    device: *mut *mut c_void,
    level: *mut u32,
    context: *mut *mut c_void,
) -> i32 {
    // SAFETY: the address came from the import table.
    let original: CreateDeviceFn =
        unsafe { std::mem::transmute(CREATE_DEVICE.load(Ordering::Acquire)) };
    // SAFETY: forwarded unchanged.
    let hr = unsafe {
        original(
            adapter,
            driver,
            software,
            flags,
            levels,
            level_count,
            sdk,
            device,
            level,
            context,
        )
    };
    // SAFETY: on success the pointers were written.
    unsafe {
        let dev = if device.is_null() {
            std::ptr::null_mut()
        } else {
            *device
        };
        let ctx = if context.is_null() {
            std::ptr::null_mut()
        } else {
            *context
        };
        log(&format!(
            "  gfx: D3D11CreateDevice flags {flags:#x} -> {hr:#x} device {:#x} context {:#x}",
            dev as usize, ctx as usize
        ));
        if hr >= 0 && !dev.is_null() && !ctx.is_null() {
            adopt(dev, ctx);
            hook_context(proxy::vtable());
            *context = proxy::wrap(ctx);
        }
    }
    hr
}

type CreateDeviceAndSwapChainFn = unsafe extern "system" fn(
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *const u32,
    u32,
    u32,
    *const c_void,
    *mut *mut c_void,
    *mut *mut c_void,
    *mut u32,
    *mut *mut c_void,
) -> i32;

#[allow(clippy::too_many_arguments)] // the D3D11 signature
unsafe extern "system" fn create_device_and_swap_chain(
    adapter: *mut c_void,
    driver: u32,
    software: *mut c_void,
    flags: u32,
    levels: *const u32,
    level_count: u32,
    sdk: u32,
    desc: *const c_void,
    swap_chain: *mut *mut c_void,
    device: *mut *mut c_void,
    level: *mut u32,
    context: *mut *mut c_void,
) -> i32 {
    // SAFETY: the address came from the import table.
    let original: CreateDeviceAndSwapChainFn =
        unsafe { std::mem::transmute(CREATE_DEVICE_AND_SWAP_CHAIN.load(Ordering::Acquire)) };
    // SAFETY: forwarded unchanged.
    let hr = unsafe {
        original(
            adapter,
            driver,
            software,
            flags,
            levels,
            level_count,
            sdk,
            desc,
            swap_chain,
            device,
            level,
            context,
        )
    };
    if hr < 0 || device.is_null() || context.is_null() {
        return hr;
    }
    // SAFETY: on success the pointers were written; COM objects start with
    // their vtables.
    unsafe {
        let (dev, ctx) = (*device, *context);
        if dev.is_null() || ctx.is_null() || DEVICE.load(Ordering::Acquire) != 0 {
            return hr;
        }
        log(&format!(
            "  gfx: D3D11CreateDeviceAndSwapChain flags {flags:#x} device {:#x} context {:#x}",
            dev as usize, ctx as usize
        ));
        DEVICE.store(dev as usize, Ordering::Release);
        CONTEXT.store(ctx as usize, Ordering::Release);
        adopt(dev, ctx);
        hook_context(proxy::vtable());
        *context = proxy::wrap(ctx);
        live::start();
        if !desc.is_null() {
            // DXGI_SWAP_CHAIN_DESC: width, height first; the window at 44.
            let d = desc.cast::<u8>();
            let w = std::ptr::read_unaligned(d.cast::<u32>());
            let h = std::ptr::read_unaligned(d.add(4).cast::<u32>());
            let hwnd = std::ptr::read_unaligned(d.add(44).cast::<usize>());
            if live::on() {
                live::set_window(hwnd, w, h);
            }
        }
        if !swap_chain.is_null() && !(*swap_chain).is_null() {
            let vtable = *(*swap_chain).cast::<*mut usize>();
            if let Some(o) =
                crate::iat::patch_vtable(vtable, vt::PRESENT, present as *const () as usize)
            {
                PRESENT_ORIGINAL.store(o, Ordering::Release);
            }
        }
        log("  gfx: device, context and Present hooked");
    }
    hr
}

/// Every (vtable, slot, replacement) hooked, so that entries the runtime
/// writes back can be hooked again.
static HOOKED: Mutex<Vec<(usize, usize, usize)>> = Mutex::new(Vec::new());

unsafe fn hook(vtable: *mut usize, slot: usize, replacement: usize, table: &Table) {
    // SAFETY: forwarded from the caller.
    if let Some(o) = unsafe { crate::iat::patch_vtable(vtable, slot, replacement) } {
        table.originals[slot].store(o, Ordering::Release);
        if let Ok(mut h) = HOOKED.lock() {
            h.push((vtable as usize, slot, replacement));
        }
    }
}

/// Hooks again the entries something replaced since; returns how many.
fn rehook() -> usize {
    let Ok(list) = HOOKED.lock().map(|h| h.clone()) else {
        return 0;
    };
    let mut n = 0;
    for (vtable, slot, replacement) in list {
        // SAFETY: the vtable was hooked before, so it has the slot.
        let current = unsafe { *(vtable as *const usize).add(slot) };
        if current != replacement {
            let tables = &DEVICE_TABLES;
            if let Some(t) = tables
                .iter()
                .find(|t| t.vtable.load(Ordering::Acquire) == vtable)
            {
                // SAFETY: as above.
                if unsafe { crate::iat::patch_vtable(vtable as *mut usize, slot, replacement) }
                    .is_some()
                {
                    t.originals[slot].store(current, Ordering::Release);
                    n += 1;
                    static LOGGED: AtomicU32 = AtomicU32::new(0);
                    if LOGGED.fetch_add(1, Ordering::Relaxed) < 64 {
                        log(&format!(
                            "  gfx: vtable {vtable:#x} slot {slot} was replaced by {current:#x}; hooked again"
                        ));
                    }
                }
            }
        }
    }
    n
}

unsafe fn hook_device(vtable: *mut usize, o: &Table) {
    use vt::device as d;
    // SAFETY: the device vtable has all these slots.
    unsafe {
        hook(
            vtable,
            d::CREATE_BUFFER,
            create_buffer as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_TEXTURE_2D,
            create_texture_2d as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_SHADER_RESOURCE_VIEW,
            create_srv as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_RENDER_TARGET_VIEW,
            create_rtv as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_DEPTH_STENCIL_VIEW,
            create_dsv as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_INPUT_LAYOUT,
            create_input_layout as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_VERTEX_SHADER,
            create_vertex_shader as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_PIXEL_SHADER,
            create_pixel_shader as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_GEOMETRY_SHADER,
            create_geometry_shader as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_COMPUTE_SHADER,
            create_compute_shader as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_BLEND_STATE,
            create_blend_state as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_DEPTH_STENCIL_STATE,
            create_depth_state as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_RASTERIZER_STATE,
            create_raster_state as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_SAMPLER_STATE,
            create_sampler_state as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::GET_IMMEDIATE_CONTEXT,
            get_immediate_context as *const () as usize,
            o,
        );
        hook(
            vtable,
            d::CREATE_DEFERRED_CONTEXT,
            create_deferred_context as *const () as usize,
            o,
        );
    }
}

/// Fills the proxy vtable's hooked entries (the rest forward).
fn hook_context(vtable: *mut usize) {
    use vt::context as c;
    let entries = [
        (
            c::VS_SET_CONSTANT_BUFFERS_1,
            vs_set_cb1 as *const () as usize,
        ),
        (
            c::PS_SET_CONSTANT_BUFFERS_1,
            ps_set_cb1 as *const () as usize,
        ),
        (
            c::UPDATE_SUBRESOURCE_1,
            update_subresource1 as *const () as usize,
        ),
        (
            c::COPY_SUBRESOURCE_REGION_1,
            copy_subresource_region1 as *const () as usize,
        ),
        (c::VS_SET_CONSTANT_BUFFERS, vs_set_cb as *const () as usize),
        (c::PS_SET_CONSTANT_BUFFERS, ps_set_cb as *const () as usize),
        (c::PS_SET_SHADER_RESOURCES, ps_set_srv as *const () as usize),
        (c::VS_SET_SHADER_RESOURCES, vs_set_srv as *const () as usize),
        (c::PS_SET_SAMPLERS, ps_set_samplers as *const () as usize),
        (c::VS_SET_SAMPLERS, vs_set_samplers as *const () as usize),
        (c::PS_SET_SHADER, ps_set_shader as *const () as usize),
        (c::VS_SET_SHADER, vs_set_shader as *const () as usize),
        (c::GS_SET_SHADER, gs_set_shader as *const () as usize),
        (c::CS_SET_SHADER, cs_set_shader as *const () as usize),
        (c::DRAW_INDEXED, draw_indexed as *const () as usize),
        (c::DRAW, draw as *const () as usize),
        (
            c::DRAW_INDEXED_INSTANCED,
            draw_indexed_instanced as *const () as usize,
        ),
        (c::DRAW_INSTANCED, draw_instanced as *const () as usize),
        (c::DISPATCH, dispatch as *const () as usize),
        (c::MAP, map as *const () as usize),
        (c::UNMAP, unmap as *const () as usize),
        (
            c::IA_SET_INPUT_LAYOUT,
            ia_set_input_layout as *const () as usize,
        ),
        (
            c::IA_SET_VERTEX_BUFFERS,
            ia_set_vertex_buffers as *const () as usize,
        ),
        (
            c::IA_SET_INDEX_BUFFER,
            ia_set_index_buffer as *const () as usize,
        ),
        (
            c::IA_SET_PRIMITIVE_TOPOLOGY,
            ia_set_topology as *const () as usize,
        ),
        (
            c::OM_SET_RENDER_TARGETS,
            om_set_render_targets as *const () as usize,
        ),
        (
            c::OM_SET_RENDER_TARGETS_AND_UAVS,
            om_set_rt_uav as *const () as usize,
        ),
        (c::OM_SET_BLEND_STATE, om_set_blend as *const () as usize),
        (
            c::OM_SET_DEPTH_STENCIL_STATE,
            om_set_depth as *const () as usize,
        ),
        (c::RS_SET_STATE, rs_set_state as *const () as usize),
        (c::RS_SET_VIEWPORTS, rs_set_viewports as *const () as usize),
        (
            c::RS_SET_SCISSOR_RECTS,
            rs_set_scissors as *const () as usize,
        ),
        (
            c::COPY_SUBRESOURCE_REGION,
            copy_subresource_region as *const () as usize,
        ),
        (c::COPY_RESOURCE, copy_resource as *const () as usize),
        (
            c::UPDATE_SUBRESOURCE,
            update_subresource as *const () as usize,
        ),
        (c::CLEAR_RENDER_TARGET_VIEW, clear_rtv as *const () as usize),
        (c::CLEAR_DEPTH_STENCIL_VIEW, clear_dsv as *const () as usize),
        (c::GENERATE_MIPS, generate_mips as *const () as usize),
        (
            c::RESOLVE_SUBRESOURCE,
            resolve_subresource as *const () as usize,
        ),
    ];
    for (slot, f) in entries {
        debug_assert!(slot < proxy::SLOTS);
        // SAFETY: the proxy vtable is ours and has `proxy::SLOTS` entries.
        unsafe { *vtable.add(slot) = f };
    }
}

type Ptr = *mut c_void;

// ---- Device ----

unsafe extern "system" fn create_buffer(
    this: Ptr,
    desc: *const u32,
    data: *const usize,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, *const u32, *const usize, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments; `desc` is a D3D11_BUFFER_DESC
    // and `data` (if set) a D3D11_SUBRESOURCE_DATA.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, vt::device::CREATE_BUFFER));
        let hr = f(this, desc, data, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() && !desc.is_null() {
            let d: [u32; 6] = std::ptr::read_unaligned(desc.cast());
            let shadow = (d[2] & BIND_CONSTANT != 0).then(|| {
                let mut v = if data.is_null() || *data == 0 {
                    vec![0; d[0] as usize]
                } else {
                    std::slice::from_raw_parts(*data as *const u8, d[0] as usize).to_vec()
                };
                // Room past the end: the game's writes into a mapped
                // constant buffer may run over its size (harmless in the
                // driver's page-sized mapping).
                v.reserve_exact(4096);
                v
            });
            {
                let mut st = state();
                st.forget(*out as usize);
                st.buffers.insert(
                    *out as usize,
                    Buffer {
                        desc: d,
                        shadow,
                        version: 0,
                        wanted: false,
                        mapped_ptr: 0,
                    },
                );
            }
        }
        hr
    }
}

unsafe extern "system" fn create_texture_2d(
    this: Ptr,
    desc: *const u32,
    data: *const usize,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, *const u32, *const usize, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, vt::device::CREATE_TEXTURE_2D));
        let hr = f(this, desc, data, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() && !desc.is_null() {
            {
                let mut st = state();
                st.forget(*out as usize);
                st.textures.insert(
                    *out as usize,
                    Texture {
                        desc: std::ptr::read_unaligned(desc.cast()),
                        version: 0,
                    },
                );
            }
        }
        hr
    }
}

unsafe fn create_view(
    slot: usize,
    kind: ViewKind,
    this: Ptr,
    resource: Ptr,
    desc: *const u32,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, Ptr, *const u32, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments; view descriptions are at least
    // six u32s (format, dimension, a union of up to four).
    unsafe {
        let f: F = std::mem::transmute(device_original(this, slot));
        let hr = f(this, resource, desc, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() {
            let desc = (!desc.is_null()).then(|| {
                let mut d = [0u32; 6];
                // A render target view description is format, dimension,
                // then a union of three: five u32s. Shader resource and
                // depth-stencil views have six.
                let n = if kind == ViewKind::RenderTarget { 5 } else { 6 };
                std::ptr::copy_nonoverlapping(desc, d.as_mut_ptr(), n);
                d
            });
            {
                let mut st = state();
                st.forget(*out as usize);
                st.views.insert(
                    *out as usize,
                    View {
                        kind,
                        resource: resource as usize,
                        desc,
                    },
                );
            }
        }
        hr
    }
}

unsafe extern "system" fn create_srv(this: Ptr, r: Ptr, d: *const u32, out: *mut Ptr) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_view(
            vt::device::CREATE_SHADER_RESOURCE_VIEW,
            ViewKind::ShaderResource,
            this,
            r,
            d,
            out,
        )
    }
}

unsafe extern "system" fn create_rtv(this: Ptr, r: Ptr, d: *const u32, out: *mut Ptr) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_view(
            vt::device::CREATE_RENDER_TARGET_VIEW,
            ViewKind::RenderTarget,
            this,
            r,
            d,
            out,
        )
    }
}

unsafe extern "system" fn create_dsv(this: Ptr, r: Ptr, d: *const u32, out: *mut Ptr) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_view(
            vt::device::CREATE_DEPTH_STENCIL_VIEW,
            ViewKind::DepthStencil,
            this,
            r,
            d,
            out,
        )
    }
}

unsafe extern "system" fn create_input_layout(
    this: Ptr,
    elements: *const u32,
    count: u32,
    signature: *const u8,
    signature_len: usize,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, *const u32, u32, *const u8, usize, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments; elements are
    // D3D11_INPUT_ELEMENT_DESCs of seven u32-sized fields (x86).
    unsafe {
        let f: F = std::mem::transmute(device_original(this, vt::device::CREATE_INPUT_LAYOUT));
        let hr = f(this, elements, count, signature, signature_len, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() {
            let mut list = Vec::new();
            for i in 0..count as usize {
                let e = std::slice::from_raw_parts(elements.add(7 * i), 7);
                let name = if e[0] == 0 {
                    String::new()
                } else {
                    CStr::from_ptr(e[0] as *const _)
                        .to_string_lossy()
                        .into_owned()
                };
                list.push(Element {
                    semantic: name,
                    index: e[1],
                    format: e[2],
                    slot: e[3],
                    offset: e[4],
                    per_instance: e[5] == 1,
                    step: e[6],
                });
            }
            let sig = if signature.is_null() {
                0
            } else {
                hash(std::slice::from_raw_parts(signature, signature_len))
            };
            {
                let mut st = state();
                st.forget(*out as usize);
                st.layouts.insert(
                    *out as usize,
                    Layout {
                        elements: list,
                        signature: sig,
                    },
                );
            }
        }
        hr
    }
}

unsafe fn create_shader(
    slot: usize,
    kind: char,
    this: Ptr,
    code: *const u8,
    len: usize,
    linkage: Ptr,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, *const u8, usize, Ptr, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, slot));
        let hr = f(this, code, len, linkage, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() && !code.is_null() {
            let bytes: Arc<[u8]> = std::slice::from_raw_parts(code, len).into();
            let h = hash(&bytes);
            {
                let mut st = state();
                st.forget(*out as usize);
                st.shaders.insert(
                    *out as usize,
                    Shader {
                        kind,
                        bytecode: bytes,
                        hash: h,
                    },
                );
            }
        }
        hr
    }
}

unsafe extern "system" fn create_vertex_shader(
    this: Ptr,
    code: *const u8,
    len: usize,
    linkage: Ptr,
    out: *mut Ptr,
) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_shader(
            vt::device::CREATE_VERTEX_SHADER,
            'v',
            this,
            code,
            len,
            linkage,
            out,
        )
    }
}

unsafe extern "system" fn create_pixel_shader(
    this: Ptr,
    code: *const u8,
    len: usize,
    linkage: Ptr,
    out: *mut Ptr,
) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_shader(
            vt::device::CREATE_PIXEL_SHADER,
            'p',
            this,
            code,
            len,
            linkage,
            out,
        )
    }
}

unsafe extern "system" fn create_geometry_shader(
    this: Ptr,
    code: *const u8,
    len: usize,
    linkage: Ptr,
    out: *mut Ptr,
) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_shader(
            vt::device::CREATE_GEOMETRY_SHADER,
            'g',
            this,
            code,
            len,
            linkage,
            out,
        )
    }
}

unsafe extern "system" fn create_compute_shader(
    this: Ptr,
    code: *const u8,
    len: usize,
    linkage: Ptr,
    out: *mut Ptr,
) -> i32 {
    // SAFETY: forwarded.
    unsafe {
        create_shader(
            vt::device::CREATE_COMPUTE_SHADER,
            'c',
            this,
            code,
            len,
            linkage,
            out,
        )
    }
}

unsafe fn create_state(
    slot: usize,
    kind: char,
    size: usize,
    this: Ptr,
    desc: *const u8,
    out: *mut Ptr,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, *const u8, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments; `size` is the description's.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, slot));
        let hr = f(this, desc, out);
        if hr >= 0 && !out.is_null() && !(*out).is_null() && !desc.is_null() {
            let bytes = std::slice::from_raw_parts(desc, size).to_vec();
            {
                let mut st = state();
                st.forget(*out as usize);
                st.states.insert(*out as usize, (kind, bytes));
            }
        }
        hr
    }
}

unsafe extern "system" fn create_blend_state(this: Ptr, d: *const u8, out: *mut Ptr) -> i32 {
    // D3D11_BLEND_DESC: two BOOLs and eight 32-byte render target entries.
    // SAFETY: forwarded.
    unsafe {
        create_state(
            vt::device::CREATE_BLEND_STATE,
            'b',
            8 + 8 * 32,
            this,
            d,
            out,
        )
    }
}

unsafe extern "system" fn create_depth_state(this: Ptr, d: *const u8, out: *mut Ptr) -> i32 {
    // D3D11_DEPTH_STENCIL_DESC: 52 bytes.
    // SAFETY: forwarded.
    unsafe {
        create_state(
            vt::device::CREATE_DEPTH_STENCIL_STATE,
            'd',
            52,
            this,
            d,
            out,
        )
    }
}

unsafe extern "system" fn create_raster_state(this: Ptr, d: *const u8, out: *mut Ptr) -> i32 {
    // D3D11_RASTERIZER_DESC: 40 bytes.
    // SAFETY: forwarded.
    unsafe { create_state(vt::device::CREATE_RASTERIZER_STATE, 'r', 40, this, d, out) }
}

unsafe extern "system" fn create_sampler_state(this: Ptr, d: *const u8, out: *mut Ptr) -> i32 {
    // D3D11_SAMPLER_DESC: 52 bytes.
    // SAFETY: forwarded.
    unsafe { create_state(vt::device::CREATE_SAMPLER_STATE, 's', 52, this, d, out) }
}

unsafe extern "system" fn get_immediate_context(this: Ptr, out: *mut Ptr) {
    type F = unsafe extern "system" fn(Ptr, *mut Ptr);
    // SAFETY: the original, same arguments; the game gets the proxy.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, vt::device::GET_IMMEDIATE_CONTEXT));
        f(this, out);
        if !out.is_null()
            && !(*out).is_null()
            && let Some(p) = proxy::proxy_of(*out)
        {
            *out = p;
        }
    }
}

unsafe extern "system" fn create_deferred_context(this: Ptr, flags: u32, out: *mut Ptr) -> i32 {
    type F = unsafe extern "system" fn(Ptr, u32, *mut Ptr) -> i32;
    // SAFETY: the original, same arguments.
    unsafe {
        let f: F = std::mem::transmute(device_original(this, vt::device::CREATE_DEFERRED_CONTEXT));
        let hr = f(this, flags, out);
        log(&format!("  gfx: CreateDeferredContext -> {hr:#x}"));
        hr
    }
}

// ---- Context ----

/// Copies `n` pointers from `src` into `dst[start..]`.
unsafe fn set_slots(dst: &mut [usize], start: u32, n: u32, src: *const usize) {
    for i in 0..n as usize {
        let at = start as usize + i;
        if at < dst.len() {
            // SAFETY: the caller passes `n` pointers (or null for none).
            dst[at] = if src.is_null() {
                0
            } else {
                unsafe { *src.add(i) }
            };
        }
    }
}

macro_rules! slots_hook {
    ($name:ident, $slot:expr, $field:ident) => {
        unsafe extern "system" fn $name(this: Ptr, start: u32, n: u32, list: *const usize) {
            type F = unsafe extern "system" fn(Ptr, u32, u32, *const usize);
            // SAFETY: the original, same arguments.
            unsafe {
                set_slots(&mut state().bound.$field, start, n, list);
                let (this, f) = forward(this, $slot);
                let f: F = std::mem::transmute(f);
                f(this, start, n, list);
            }
        }
    };
}

macro_rules! cb_hook {
    ($name:ident, $name1:ident, $slot:expr, $slot1:expr, $field:ident, $range:ident) => {
        unsafe extern "system" fn $name(this: Ptr, start: u32, n: u32, list: *const usize) {
            type F = unsafe extern "system" fn(Ptr, u32, u32, *const usize);
            // SAFETY: the original, same arguments.
            unsafe {
                {
                    let mut s = state();
                    set_slots(&mut s.bound.$field, start, n, list);
                    for i in start..(start + n).min(14) {
                        s.bound.$range[i as usize] = (0, 0);
                    }
                }
                let (this, f) = forward(this, $slot);
                let f: F = std::mem::transmute(f);
                f(this, start, n, list);
            }
        }

        unsafe extern "system" fn $name1(
            this: Ptr,
            start: u32,
            n: u32,
            list: *const usize,
            first: *const u32,
            count: *const u32,
        ) {
            type F = unsafe extern "system" fn(Ptr, u32, u32, *const usize, *const u32, *const u32);
            // SAFETY: the original, same arguments; the ranges have `n`
            // entries when set.
            unsafe {
                {
                    let mut s = state();
                    set_slots(&mut s.bound.$field, start, n, list);
                    for i in 0..n {
                        let at = (start + i) as usize;
                        if at < 14 {
                            s.bound.$range[at] = if first.is_null() || count.is_null() {
                                (0, 0)
                            } else {
                                (*first.add(i as usize), *count.add(i as usize))
                            };
                        }
                    }
                }
                let (this, f) = forward(this, $slot1);
                let f: F = std::mem::transmute(f);
                f(this, start, n, list, first, count);
            }
        }
    };
}

cb_hook!(
    vs_set_cb,
    vs_set_cb1,
    vt::context::VS_SET_CONSTANT_BUFFERS,
    vt::context::VS_SET_CONSTANT_BUFFERS_1,
    vs_cb,
    vs_cb_range
);
cb_hook!(
    ps_set_cb,
    ps_set_cb1,
    vt::context::PS_SET_CONSTANT_BUFFERS,
    vt::context::PS_SET_CONSTANT_BUFFERS_1,
    ps_cb,
    ps_cb_range
);
slots_hook!(ps_set_srv, vt::context::PS_SET_SHADER_RESOURCES, ps_srv);
slots_hook!(vs_set_srv, vt::context::VS_SET_SHADER_RESOURCES, vs_srv);
slots_hook!(ps_set_samplers, vt::context::PS_SET_SAMPLERS, ps_sampler);
slots_hook!(vs_set_samplers, vt::context::VS_SET_SAMPLERS, vs_sampler);

macro_rules! shader_hook {
    ($name:ident, $slot:expr, $set:expr) => {
        unsafe extern "system" fn $name(this: Ptr, shader: Ptr, classes: *const Ptr, n: u32) {
            type F = unsafe extern "system" fn(Ptr, Ptr, *const Ptr, u32);
            // SAFETY: the original, same arguments.
            unsafe {
                $set(&mut state(), shader as usize);
                let (this, f) = forward(this, $slot);
                let f: F = std::mem::transmute(f);
                f(this, shader, classes, n);
            }
        }
    };
}

shader_hook!(
    vs_set_shader,
    vt::context::VS_SET_SHADER,
    |s: &mut State, p| s.bound.vs = p
);
shader_hook!(
    ps_set_shader,
    vt::context::PS_SET_SHADER,
    |s: &mut State, p| s.bound.ps = p
);
shader_hook!(
    gs_set_shader,
    vt::context::GS_SET_SHADER,
    |s: &mut State, p: usize| if p != 0 {
        capture::note(s, &format!("gs_set_shader {p:#x}"));
    }
);
shader_hook!(
    cs_set_shader,
    vt::context::CS_SET_SHADER,
    |s: &mut State, p: usize| if p != 0 {
        capture::note(s, &format!("cs_set_shader {p:#x}"));
    }
);

/// A draw call's parameters.
#[derive(Debug, Clone, Copy)]
pub enum Draw {
    Indexed {
        count: u32,
        start: u32,
        base: i32,
    },
    Plain {
        count: u32,
        start: u32,
    },
    IndexedInstanced {
        count: u32,
        instances: u32,
        start: u32,
        base: i32,
        first: u32,
    },
    Instanced {
        count: u32,
        instances: u32,
        start: u32,
        first: u32,
    },
}

/// Records a draw; true when the game's own draw is to be skipped (the
/// live renderer draws it).
fn on_draw(d: Draw) -> bool {
    let t = std::time::Instant::now();
    let skip = on_draw_inner(d);
    SPENT[0].fetch_add(t.elapsed().as_micros() as u32, Ordering::Relaxed);
    skip
}

fn on_draw_inner(d: Draw) -> bool {
    DRAWS.fetch_add(1, Ordering::Relaxed);
    let mut s = state();
    if s.capture.is_some() {
        capture::draw(&mut s, d);
    }
    if live::on() {
        return live::draw(&mut s, d);
    }
    false
    // Drawing into a texture does not count as a write the renderer needs
    // to see again: it draws the scene targets itself.
}

/// For the game's afterimage pass when it is turned off: no blending for
/// this draw. Returns the blend state to put back.
fn unblended(this: Ptr) -> Option<(usize, [f32; 4], u32)> {
    let (blend, factor, mask) = {
        let s = state();
        if !live::afterimage_off(&s) {
            return None;
        }
        (s.bound.blend, s.bound.blend_factor, s.bound.sample_mask)
    };
    // SAFETY: the context's own method; a null state is the default (no
    // blending).
    unsafe { set_blend(this, 0, &factor, mask) };
    Some((blend, factor, mask))
}

/// Puts back what `unblended` changed.
fn reblend(this: Ptr, saved: Option<(usize, [f32; 4], u32)>) {
    if let Some((blend, factor, mask)) = saved {
        // SAFETY: the game's own state, as it set it.
        unsafe { set_blend(this, blend, &factor, mask) };
    }
}

/// `OMSetBlendState` past the hook (the game's view of what is bound stays).
unsafe fn set_blend(this: Ptr, blend: usize, factor: &[f32; 4], mask: u32) {
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32, u32);
    // SAFETY: the original method, its documented arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::OM_SET_BLEND_STATE);
        let f: F = std::mem::transmute(f);
        f(this, blend as Ptr, factor.as_ptr(), mask);
    }
}

unsafe extern "system" fn draw_indexed(this: Ptr, count: u32, start: u32, base: i32) {
    if on_draw(Draw::Indexed { count, start, base }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, i32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::DRAW_INDEXED);
        let f: F = std::mem::transmute(f);
        let saved = unblended(this);
        f(this, count, start, base);
        reblend(this, saved);
        capture::after_draw(&mut state());
    }
}

unsafe extern "system" fn draw(this: Ptr, count: u32, start: u32) {
    if on_draw(Draw::Plain { count, start }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::DRAW);
        let f: F = std::mem::transmute(f);
        let saved = unblended(this);
        f(this, count, start);
        reblend(this, saved);
        capture::after_draw(&mut state());
    }
}

unsafe extern "system" fn draw_indexed_instanced(
    this: Ptr,
    count: u32,
    instances: u32,
    start: u32,
    base: i32,
    first: u32,
) {
    let skip = on_draw(Draw::IndexedInstanced {
        count,
        instances,
        start,
        base,
        first,
    });
    if skip {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, i32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::DRAW_INDEXED_INSTANCED);
        let f: F = std::mem::transmute(f);
        let saved = unblended(this);
        f(this, count, instances, start, base, first);
        reblend(this, saved);
        capture::after_draw(&mut state());
    }
}

unsafe extern "system" fn draw_instanced(
    this: Ptr,
    count: u32,
    instances: u32,
    start: u32,
    first: u32,
) {
    let skip = on_draw(Draw::Instanced {
        count,
        instances,
        start,
        first,
    });
    if skip {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::DRAW_INSTANCED);
        let f: F = std::mem::transmute(f);
        let saved = unblended(this);
        f(this, count, instances, start, first);
        reblend(this, saved);
        capture::after_draw(&mut state());
    }
}

unsafe extern "system" fn dispatch(this: Ptr, x: u32, y: u32, z: u32) {
    capture::note(&mut state(), &format!("dispatch {x} {y} {z}"));
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::DISPATCH);
        let f: F = std::mem::transmute(f);
        f(this, x, y, z);
    }
}

unsafe extern "system" fn map(
    this: Ptr,
    resource: Ptr,
    sub: u32,
    kind: u32,
    flags: u32,
    out: *mut usize,
) -> i32 {
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, *mut usize) -> i32;
    // SAFETY: the original, same arguments; `out` is a
    // D3D11_MAPPED_SUBRESOURCE whose first field is the data pointer.
    unsafe {
        let (this, f) = forward(this, vt::context::MAP);
        let f: F = std::mem::transmute(f);
        let hr = f(this, resource, sub, kind, flags, out);
        // Writes only (read maps are the layer's own and the game's
        // read-backs).
        if hr >= 0 && !out.is_null() && kind != 1 {
            let mut s = state();
            if let Some(b) = s.buffers.get_mut(&(resource as usize)) {
                b.mapped_ptr = *out;
                let real = *out;
                // Constant buffers: the game writes into the layer's copy
                // (cached memory, cheap to read), which goes to the
                // driver's memory at Unmap.
                if let Some(sh) = b.shadow.as_mut()
                    && b.desc[2] & BIND_CONSTANT != 0
                    && redirect()
                {
                    *out = sh.as_mut_ptr() as usize;
                }
                s.mapped.insert(resource as usize, (sub, real));
            } else if s.textures.contains_key(&(resource as usize)) {
                s.mapped.insert(resource as usize, (sub, *out));
            }
        }
        hr
    }
}

unsafe extern "system" fn unmap(this: Ptr, resource: Ptr, sub: u32) {
    let t = std::time::Instant::now();
    {
        let mut s = state();
        if let Some((_, data)) = s.mapped.remove(&(resource as usize)) {
            let capturing = s.capture.is_some();
            if let Some(t) = s.textures.get_mut(&(resource as usize)) {
                t.version = t.version.wrapping_add(1);
            }
            if let Some(b) = s.buffers.get_mut(&(resource as usize)) {
                b.version = b.version.wrapping_add(1);
                let width = b.desc[0] as usize;
                let constant = b.desc[2] & BIND_CONSTANT != 0;
                let geometry = b.desc[2] & (BIND_VERTEX | BIND_INDEX) != 0;
                if constant && !redirect() {
                    // SAFETY: as below.
                    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, width) };
                    if let Some(sh) = &mut b.shadow {
                        sh.copy_from_slice(bytes);
                    }
                } else if constant {
                    if let Some(sh) = &b.shadow {
                        // SAFETY: the driver's mapping is `width` bytes and
                        // still mapped until the original Unmap below.
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                sh.as_ptr(),
                                data as *mut u8,
                                width.min(sh.len()),
                            );
                        }
                    }
                } else if capturing && geometry {
                    // Captures read the whole buffer (slow: write-combined
                    // memory), for study only.
                    // SAFETY: as above.
                    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, width) };
                    match &mut b.shadow {
                        Some(sh) => sh.copy_from_slice(bytes),
                        None => b.shadow = Some(bytes.to_vec()),
                    }
                }
            }
        }
    }
    SPENT[1].fetch_add(t.elapsed().as_micros() as u32, Ordering::Relaxed);
    type F = unsafe extern "system" fn(Ptr, Ptr, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::UNMAP);
        let f: F = std::mem::transmute(f);
        f(this, resource, sub);
    }
}

unsafe extern "system" fn ia_set_input_layout(this: Ptr, layout: Ptr) {
    seen("IASetInputLayout", this);
    state().bound.layout = layout as usize;
    type F = unsafe extern "system" fn(Ptr, Ptr);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::IA_SET_INPUT_LAYOUT);
        let f: F = std::mem::transmute(f);
        f(this, layout);
    }
}

unsafe extern "system" fn ia_set_vertex_buffers(
    this: Ptr,
    start: u32,
    n: u32,
    buffers: *const usize,
    strides: *const u32,
    offsets: *const u32,
) {
    {
        let mut s = state();
        for i in 0..n as usize {
            let at = start as usize + i;
            if at < 8 {
                // SAFETY: the caller passes `n` entries in each array.
                s.bound.vertex[at] = unsafe {
                    (
                        if buffers.is_null() {
                            0
                        } else {
                            *buffers.add(i)
                        },
                        if strides.is_null() {
                            0
                        } else {
                            *strides.add(i)
                        },
                        if offsets.is_null() {
                            0
                        } else {
                            *offsets.add(i)
                        },
                    )
                };
            }
        }
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, *const usize, *const u32, *const u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::IA_SET_VERTEX_BUFFERS);
        let f: F = std::mem::transmute(f);
        f(this, start, n, buffers, strides, offsets);
    }
}

unsafe extern "system" fn ia_set_index_buffer(this: Ptr, buffer: Ptr, format: u32, offset: u32) {
    state().bound.index = (buffer as usize, format, offset);
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::IA_SET_INDEX_BUFFER);
        let f: F = std::mem::transmute(f);
        f(this, buffer, format, offset);
    }
}

unsafe extern "system" fn ia_set_topology(this: Ptr, topology: u32) {
    state().bound.topology = topology;
    type F = unsafe extern "system" fn(Ptr, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::IA_SET_PRIMITIVE_TOPOLOGY);
        let f: F = std::mem::transmute(f);
        f(this, topology);
    }
}

/// Logs each new object a hook is called on (diagnostics).
fn seen(what: &str, this: Ptr) {
    static SEEN: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());
    // SAFETY: a live COM object.
    let vtable = unsafe { *(this as *const usize) };
    if let Ok(mut s) = SEEN.lock()
        && !s.contains(&(this as usize, vtable))
        && s.len() < 32
    {
        s.push((this as usize, vtable));
        log(&format!(
            "  gfx: {what} on {:#x} vtable {vtable:#x}",
            this as usize
        ));
    }
}

unsafe extern "system" fn om_set_render_targets(this: Ptr, n: u32, rtvs: *const usize, dsv: Ptr) {
    seen("OMSetRenderTargets", this);
    {
        let mut s = state();
        let mut list = [0usize; 8];
        // SAFETY: the caller passes `n` views.
        unsafe { set_slots(&mut list, 0, n, rtvs) };
        if s.capture.is_some() {
            capture::render_targets(&mut s, list, dsv as usize);
        }
        if live::on() {
            live::targets(&mut s, list, dsv as usize);
        }
        s.bound.rtv = list;
        s.bound.dsv = dsv as usize;
    }
    type F = unsafe extern "system" fn(Ptr, u32, *const usize, Ptr);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::OM_SET_RENDER_TARGETS);
        let f: F = std::mem::transmute(f);
        f(this, n, rtvs, dsv);
    }
}

unsafe extern "system" fn om_set_rt_uav(
    this: Ptr,
    n: u32,
    rtvs: *const usize,
    dsv: Ptr,
    uav_start: u32,
    uavs: u32,
    uav_list: *const usize,
    counts: *const u32,
) {
    capture::note(
        &mut state(),
        &format!("om_set_render_targets_and_uavs {n} uavs {uavs}"),
    );
    type F =
        unsafe extern "system" fn(Ptr, u32, *const usize, Ptr, u32, u32, *const usize, *const u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::OM_SET_RENDER_TARGETS_AND_UAVS);
        let f: F = std::mem::transmute(f);
        f(this, n, rtvs, dsv, uav_start, uavs, uav_list, counts);
    }
}

unsafe extern "system" fn om_set_blend(this: Ptr, blend: Ptr, factor: *const f32, mask: u32) {
    {
        let mut s = state();
        s.bound.blend = blend as usize;
        s.bound.sample_mask = mask;
        if !factor.is_null() {
            // SAFETY: four floats.
            s.bound.blend_factor = unsafe { std::ptr::read_unaligned(factor.cast()) };
        }
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::OM_SET_BLEND_STATE);
        let f: F = std::mem::transmute(f);
        f(this, blend, factor, mask);
    }
}

unsafe extern "system" fn om_set_depth(this: Ptr, depth: Ptr, stencil_ref: u32) {
    {
        let mut s = state();
        s.bound.depth = depth as usize;
        s.bound.stencil_ref = stencil_ref;
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::OM_SET_DEPTH_STENCIL_STATE);
        let f: F = std::mem::transmute(f);
        f(this, depth, stencil_ref);
    }
}

unsafe extern "system" fn rs_set_state(this: Ptr, raster: Ptr) {
    state().bound.raster = raster as usize;
    type F = unsafe extern "system" fn(Ptr, Ptr);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::RS_SET_STATE);
        let f: F = std::mem::transmute(f);
        f(this, raster);
    }
}

unsafe extern "system" fn rs_set_viewports(this: Ptr, n: u32, viewports: *const f32) {
    if n > 0 && !viewports.is_null() {
        // SAFETY: at least one D3D11_VIEWPORT (six floats).
        state().bound.viewport = unsafe { std::ptr::read_unaligned(viewports.cast()) };
    }
    type F = unsafe extern "system" fn(Ptr, u32, *const f32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::RS_SET_VIEWPORTS);
        let f: F = std::mem::transmute(f);
        f(this, n, viewports);
    }
}

unsafe extern "system" fn rs_set_scissors(this: Ptr, n: u32, rects: *const i32) {
    if n > 0 && !rects.is_null() {
        // SAFETY: at least one RECT (four i32).
        state().bound.scissor = unsafe { std::ptr::read_unaligned(rects.cast()) };
    }
    type F = unsafe extern "system" fn(Ptr, u32, *const i32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::RS_SET_SCISSOR_RECTS);
        let f: F = std::mem::transmute(f);
        f(this, n, rects);
    }
}

#[allow(clippy::too_many_arguments)] // the D3D11 signature
unsafe extern "system" fn copy_subresource_region(
    this: Ptr,
    dst: Ptr,
    dst_sub: u32,
    x: u32,
    y: u32,
    z: u32,
    src: Ptr,
    src_sub: u32,
    region: *const u32,
) {
    {
        let mut s = state();
        s.touch(dst as usize);
        capture::note(
            &mut s,
            &format!(
                "copy_region {:#x}[{dst_sub}] @{x},{y},{z} <- {:#x}[{src_sub}]",
                dst as usize, src as usize
            ),
        );
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, u32, Ptr, u32, *const u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::COPY_SUBRESOURCE_REGION);
        let f: F = std::mem::transmute(f);
        f(this, dst, dst_sub, x, y, z, src, src_sub, region);
    }
}

unsafe extern "system" fn copy_resource(this: Ptr, dst: Ptr, src: Ptr) {
    {
        let mut s = state();
        s.touch(dst as usize);
        capture::note(
            &mut s,
            &format!("copy {:#x} <- {:#x}", dst as usize, src as usize),
        );
        capture::copy(&mut s, dst as usize, src as usize);
        if live::on() {
            live::copy(&mut s, dst as usize, src as usize);
        }
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, Ptr);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::COPY_RESOURCE);
        let f: F = std::mem::transmute(f);
        f(this, dst, src);
    }
}

fn note_update(dst: Ptr, sub: u32, region: *const u32, data: *const u8) {
    let mut s = state();
    let capturing = s.capture.is_some();
    if let Some(b) = s.buffers.get_mut(&(dst as usize)) {
        b.version = b.version.wrapping_add(1);
        let width = b.desc[0] as usize;
        let (from, to) = if region.is_null() {
            (0, width)
        } else {
            // SAFETY: a D3D11_BOX: left, top, front, right, bottom, back.
            unsafe { (*region as usize, *region.add(3) as usize) }
        };
        if !data.is_null() && from < to && to <= width {
            let geometry = b.desc[2] & (BIND_VERTEX | BIND_INDEX) != 0;
            if b.shadow.is_none() && (capturing || (live::on() && geometry)) {
                // The rest of the buffer as it is now, once.
                // SAFETY: on the render thread, inside a context call.
                let now = unsafe { capture::read_buffer(dst as usize, b.desc[0]) };
                b.shadow = Some(now.unwrap_or_else(|_| vec![0; width]));
            }
            if let Some(sh) = &mut b.shadow {
                // SAFETY: the caller passes `to - from` bytes.
                sh[from..to]
                    .copy_from_slice(unsafe { std::slice::from_raw_parts(data, to - from) });
            }
        }
    } else {
        s.touch(dst as usize);
        if capturing {
            capture::note(
                &mut s,
                &format!("update_texture {:#x}[{sub}]", dst as usize),
            );
        }
    }
}

unsafe extern "system" fn update_subresource(
    this: Ptr,
    dst: Ptr,
    sub: u32,
    region: *const u32,
    data: *const u8,
    row: u32,
    depth: u32,
) {
    note_update(dst, sub, region, data);
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::UPDATE_SUBRESOURCE);
        let f: F = std::mem::transmute(f);
        f(this, dst, sub, region, data, row, depth);
    }
}

#[allow(clippy::too_many_arguments)] // the D3D11 signature
unsafe extern "system" fn update_subresource1(
    this: Ptr,
    dst: Ptr,
    sub: u32,
    region: *const u32,
    data: *const u8,
    row: u32,
    depth: u32,
    flags: u32,
) {
    note_update(dst, sub, region, data);
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::UPDATE_SUBRESOURCE_1);
        let f: F = std::mem::transmute(f);
        f(this, dst, sub, region, data, row, depth, flags);
    }
}

#[allow(clippy::too_many_arguments)] // the D3D11 signature
unsafe extern "system" fn copy_subresource_region1(
    this: Ptr,
    dst: Ptr,
    dst_sub: u32,
    x: u32,
    y: u32,
    z: u32,
    src: Ptr,
    src_sub: u32,
    region: *const u32,
    flags: u32,
) {
    {
        let mut s = state();
        s.touch(dst as usize);
        capture::note(
            &mut s,
            &format!(
                "copy_region1 {:#x}[{dst_sub}] <- {:#x}[{src_sub}]",
                dst as usize, src as usize
            ),
        );
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, u32, Ptr, u32, *const u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::COPY_SUBRESOURCE_REGION_1);
        let f: F = std::mem::transmute(f);
        f(this, dst, dst_sub, x, y, z, src, src_sub, region, flags);
    }
}

unsafe extern "system" fn clear_rtv(this: Ptr, rtv: Ptr, color: *const f32) {
    if !color.is_null() {
        // SAFETY: four floats.
        let c: [f32; 4] = unsafe { std::ptr::read_unaligned(color.cast()) };
        let mut s = state();
        capture::note(&mut s, &format!("clear_rtv {:#x} {c:?}", rtv as usize));
        capture::clear_target(&mut s, rtv as usize, c);
        if live::on() {
            live::clear(ffx_gfx::Cmd::ClearTarget {
                view: rtv as u32,
                color: c,
            });
        }
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::CLEAR_RENDER_TARGET_VIEW);
        let f: F = std::mem::transmute(f);
        f(this, rtv, color);
    }
}

unsafe extern "system" fn clear_dsv(this: Ptr, dsv: Ptr, flags: u32, depth: f32, stencil: u8) {
    {
        let mut s = state();
        capture::note(
            &mut s,
            &format!(
                "clear_dsv {:#x} flags {flags} depth {depth} stencil {stencil}",
                dsv as usize
            ),
        );
        capture::clear_depth(&mut s, dsv as usize, flags, depth, stencil);
        if live::on() {
            live::clear(ffx_gfx::Cmd::ClearDepth {
                view: dsv as u32,
                flags,
                depth,
                stencil,
            });
        }
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, f32, u8);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::CLEAR_DEPTH_STENCIL_VIEW);
        let f: F = std::mem::transmute(f);
        f(this, dsv, flags, depth, stencil);
    }
}

unsafe extern "system" fn generate_mips(this: Ptr, srv: Ptr) {
    capture::note(&mut state(), &format!("generate_mips {:#x}", srv as usize));
    type F = unsafe extern "system" fn(Ptr, Ptr);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::GENERATE_MIPS);
        let f: F = std::mem::transmute(f);
        f(this, srv);
    }
}

unsafe extern "system" fn resolve_subresource(
    this: Ptr,
    dst: Ptr,
    dsub: u32,
    src: Ptr,
    ssub: u32,
    format: u32,
) {
    capture::note(
        &mut state(),
        &format!(
            "resolve {:#x} <- {:#x} format {format}",
            dst as usize, src as usize
        ),
    );
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, Ptr, u32, u32);
    // SAFETY: the original, same arguments.
    unsafe {
        let (this, f) = forward(this, vt::context::RESOLVE_SUBRESOURCE);
        let f: F = std::mem::transmute(f);
        f(this, dst, dsub, src, ssub, format);
    }
}

// ---- Present ----

unsafe extern "system" fn present(swap_chain: Ptr, interval: u32, flags: u32) -> i32 {
    scene::on_present();
    FRAME.fetch_add(1, Ordering::AcqRel);
    LAST_DRAWS.store(DRAWS.swap(0, Ordering::AcqRel), Ordering::Release);
    rehook();
    if live::on() {
        // SAFETY: on the render thread, inside Present.
        unsafe { live::present(&mut state(), swap_chain) };
    }
    // SAFETY: on the game thread, inside Present.
    unsafe { capture::on_present(swap_chain) };
    type F = unsafe extern "system" fn(Ptr, u32, u32) -> i32;
    let t = std::time::Instant::now();
    // SAFETY: the original, same arguments.
    let hr = unsafe {
        let f: F = std::mem::transmute(PRESENT_ORIGINAL.load(Ordering::Acquire));
        f(swap_chain, interval, flags)
    };
    if live::on() {
        static LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
        let frame = FRAME.load(Ordering::Relaxed);
        if let Ok(mut last) = LAST.lock() {
            if frame.is_multiple_of(120)
                && let Some(l) = *last
            {
                let spent: Vec<u32> = SPENT
                    .iter()
                    .map(|a| a.swap(0, Ordering::Relaxed) / 120)
                    .collect();
                log(&format!(
                    "  gfx: frame {frame}: present took {:?} (interval {interval}), {:?} since the last; per frame: draws {} us, unmaps {} us",
                    t.elapsed(),
                    l.elapsed(),
                    spent[0],
                    spent[1]
                ));
            }
            *last = Some(std::time::Instant::now());
        }
    }
    hr
}

/// Whether constant buffer maps go to the layer's copy (`FFX_GFX_REDIRECT=0`
/// turns it off, for finding problems).
fn redirect() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("FFX_GFX_REDIRECT").map_or(true, |v| v != "0"))
}

/// Frames presented and draws in the last one.
#[must_use]
pub fn stats() -> (u32, u32) {
    (
        FRAME.load(Ordering::Acquire),
        LAST_DRAWS.load(Ordering::Acquire),
    )
}

pub use capture::request as capture;
