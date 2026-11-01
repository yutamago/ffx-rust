//! The game's Direct3D 11 device and DXGI (docs/host.md, "D3D11 and
//! DXGI").
//!
//! `D3D11CreateDeviceAndSwapChain` makes the device, its immediate
//! context, the swap chain and (on demand) the DXGI factory, adapter,
//! output and device as COM objects in the arena ([`crate::com`]). Every
//! method call lands in [`method`], which reads the game's arguments from
//! the arena, keeps what the API would know (resource descriptions,
//! contents of mappable resources, the context's bound state), records
//! the call, and hands the decoded call to a [`Backend`]:
//!
//! - [`Null`]: no device (the Linux check): objects are plausible
//!   stand-ins, reads give back what was written;
//! - `D3d11` (Windows, `crate::d3d11`): the real device, every call
//!   forwarded with its arguments marshalled out of the arena.
//!
//! Whatever the back end, the layer records the stream the hook's capture
//! writes (`ffx_gfx`, `docs/draw.md` and `docs/drawdiff.md`): objects as
//! they are made, and per `Present` a frame of draws with everything they
//! had bound, clears, copies and buffer updates. Every call also goes into
//! a text log (`Gfx::log`), and calls the layer doesn't handle are listed
//! (`Gfx::unhandled`), as missing imports are.

// One arm per method as `d3d11.h` declares them (many answer `S_OK`), and
// the methods' argument words as `a[i]`, `this` first.
#![allow(clippy::match_same_arms, clippy::many_single_char_names)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};

use ffx_bridge::Mem as _;
use ffx_gfx::{Cmd, Element, Msg, StateKind, Subresource, ViewKind};

use crate::com::{
    self, Com, DXGI_ERROR_NOT_FOUND, E_INVALIDARG, E_NOINTERFACE, E_NOTIMPL, E_OUTOFMEMORY,
    E_POINTER, S_FALSE, S_OK,
};
use crate::com_methods::{CLASSES, Returns, class};
use crate::exec::{Exec, Stop};
use crate::table::{Args, Ret};

mod composite;

/// `D3D_FEATURE_LEVEL_11_0`.
pub const FEATURE_LEVEL_11_0: u32 = 0xb000;
/// `DI_NOTATTACHED` (`S_FALSE`: the device is not attached).
const DI_NOTATTACHED: u32 = S_FALSE;
/// The most lines the text log keeps (the counts go on).
const LOG_LINES: usize = 50_000;

/// The shared slot ([`crate::exec::Shared::gfx`]): the layer once the
/// device exists.
#[derive(Default)]
pub struct Slot {
    gfx: Mutex<Option<Gfx>>,
    config: Mutex<Config>,
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("gfx::Slot")
    }
}

impl Slot {
    /// The layer (`None` before the device).
    pub fn lock(&self) -> MutexGuard<'_, Option<Gfx>> {
        self.gfx
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Sets how the layer will run (before the device is made).
    pub fn configure(&self, config: Config) {
        if let Ok(mut c) = self.config.lock() {
            *c = config;
        }
    }

    /// How the layer runs.
    #[must_use]
    pub fn config(&self) -> Config {
        self.config.lock().map(|c| c.clone()).unwrap_or_default()
    }
}

/// Which back end a run uses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Choice {
    /// The real device on Windows, the null one elsewhere.
    #[default]
    Default,
    /// The recording null device everywhere.
    Null,
}

/// How the layer runs ([`crate::Options::gfx`]).
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// The back end.
    pub backend: Choice,
    /// End the run (a milestone) after this many `Present` calls.
    pub stop_after_presents: Option<u32>,
    /// Keep no recording (`capture.ffxgfx`): `ffx-rust` without
    /// `--gfx-out` or `FFX_HOST_GFX_RECORD=1`. The recording grows by every
    /// frame's draws (1.8 GB on the boot to the title, about 6 GB of the
    /// process in menu-grid's harness run).
    pub no_record: bool,
}

/// `DXGI_SWAP_CHAIN_DESC`, word by word as the game passes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwapChainDesc(pub [u32; 15]);

impl SwapChainDesc {
    /// The back buffer's width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.0[0]
    }
    /// The back buffer's height.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.0[1]
    }
    /// The back buffer's DXGI format.
    #[must_use]
    pub fn format(&self) -> u32 {
        self.0[4]
    }
    /// The window.
    #[must_use]
    pub fn window(&self) -> u32 {
        self.0[11]
    }
    /// The back buffer as a `D3D11_TEXTURE2D_DESC`.
    #[must_use]
    pub fn back_buffer(&self) -> [u32; 11] {
        let s = &self.0;
        // width, height, mips, array, format, samples, quality, usage
        // DEFAULT, bind RENDER_TARGET | SHADER_RESOURCE (DXGI_USAGE_*
        // render target output / shader input), CPU 0, misc 0
        let mut bind = 0;
        if s[9] & 0x20 != 0 {
            bind |= 0x20;
        }
        if s[9] & 0x10 != 0 {
            bind |= 0x08;
        }
        [s[0], s[1], 1, 1, s[4], s[7], s[8], 0, bind, 0, 0]
    }
}

/// An object to make, decoded from the game's arguments.
#[derive(Debug, Clone)]
pub enum Create<'a> {
    /// `CreateBuffer`.
    Buffer {
        /// `D3D11_BUFFER_DESC`.
        desc: [u32; 6],
        /// The initial contents.
        init: Option<&'a [u8]>,
    },
    /// `CreateTexture1D/2D/3D`.
    Texture {
        /// 1, 2 or 3.
        dims: u8,
        /// The description's words (`D3D11_TEXTURE*D_DESC`).
        desc: &'a [u32],
        /// The initial contents per subresource (row pitch, depth pitch,
        /// bytes).
        init: &'a [(u32, u32, Vec<u8>)],
    },
    /// `Create*View`.
    View {
        /// The view's class ([`class::SRV`], ...).
        class: usize,
        /// The resource.
        resource: u32,
        /// The description's bytes, when given.
        desc: Option<&'a [u8]>,
    },
    /// `Create*Shader`: `b'v'`, `b'p'`, `b'g'`, `b'h'`, `b'd'`, `b'c'`.
    Shader {
        /// The stage.
        stage: u8,
        /// The DXBC.
        bytecode: &'a [u8],
    },
    /// `CreateInputLayout`.
    Layout {
        /// The elements, raw (`D3D11_INPUT_ELEMENT_DESC` with the names
        /// read out).
        elements: &'a [Element],
        /// The vertex shader's DXBC it is checked against.
        bytecode: &'a [u8],
    },
    /// `Create*State`.
    State {
        /// Which.
        kind: StateKind,
        /// The description's bytes.
        desc: &'a [u8],
    },
    /// `CreateQuery` / `CreatePredicate`: `D3D11_QUERY_DESC`.
    Query([u32; 2]),
}

/// A context call, decoded. Objects are their game handles; the back end
/// maps them to its own.
#[derive(Debug, Clone)]
pub enum Op<'a> {
    /// `IASetInputLayout`.
    InputLayout(u32),
    /// `IASetVertexBuffers(start, buffers, strides, offsets)`.
    VertexBuffers(u32, &'a [u32], &'a [u32], &'a [u32]),
    /// `IASetIndexBuffer(buffer, format, offset)`.
    IndexBuffer(u32, u32, u32),
    /// `IASetPrimitiveTopology`.
    Topology(u32),
    /// `xSSetShader(stage, shader)`.
    Shader(u8, u32),
    /// `xSSetConstantBuffers(1)(stage, start, buffers, first constants,
    /// constant counts)`.
    ConstantBuffers(u8, u32, &'a [u32], Option<(&'a [u32], &'a [u32])>),
    /// `xSSetShaderResources(stage, start, views)`.
    Resources(u8, u32, &'a [u32]),
    /// `xSSetSamplers(stage, start, samplers)`.
    Samplers(u8, u32, &'a [u32]),
    /// `OMSetRenderTargets(views, depth)`.
    Targets(&'a [u32], u32),
    /// `OMSetBlendState(state, factor, mask)`.
    Blend(u32, [f32; 4], u32),
    /// `OMSetDepthStencilState(state, reference)`.
    Depth(u32, u32),
    /// `RSSetState`.
    Raster(u32),
    /// `RSSetViewports`.
    Viewports(&'a [[f32; 6]]),
    /// `RSSetScissorRects`.
    Scissors(&'a [[i32; 4]]),
    /// One of the four draws.
    Draw(ffx_gfx::DrawKind, [u32; 5]),
    /// `ClearRenderTargetView`.
    ClearTarget(u32, [f32; 4]),
    /// `ClearDepthStencilView(view, flags, depth, stencil)`.
    ClearDepth(u32, u32, f32, u8),
    /// `CopyResource(dst, src)`.
    Copy(u32, u32),
    /// `ResolveSubresource(dst, dst sub, src, src sub, format)`: a
    /// multisampled target into a plain one (MSAA, settings-msaa).
    Resolve([u32; 5]),
    /// `CopySubresourceRegion(dst, sub, x, y, z, src, sub, box)`.
    CopyRegion([u32; 7], Option<[u32; 6]>),
    /// `UpdateSubresource(dst, sub, box, data, row pitch, depth pitch)`.
    Update(u32, u32, Option<[u32; 6]>, &'a [u8], u32, u32),
    /// The CPU's writes to a mapped subresource, at `Unmap` (resource,
    /// subresource, row pitch, depth pitch, bytes).
    Unmapped(u32, u32, u32, u32, &'a [u8]),
    /// `Begin(query)`.
    Begin(u32),
    /// `End(query)`.
    End(u32),
    /// `GenerateMips(view)`.
    GenerateMips(u32),
    /// `ClearState`.
    ClearState,
    /// `Flush`.
    Flush,
}

/// A back end: makes the objects and runs the calls.
pub trait Backend: Send {
    /// Its name, for the report.
    fn name(&self) -> &'static str;
    /// The device and the swap chain (`D3D11CreateDeviceAndSwapChain`):
    /// an `HRESULT`.
    fn create_device(&mut self, desc: &SwapChainDesc, flags: u32) -> u32;
    /// The swap chain's back buffer, as object `id`.
    fn back_buffer(&mut self, id: u32) -> u32;
    /// Makes object `id`: an `HRESULT`.
    fn create(&mut self, id: u32, what: &Create<'_>) -> u32;
    /// The game's last reference to `id` went.
    fn release(&mut self, id: u32);
    /// A context call.
    fn op(&mut self, op: &Op<'_>);
    /// Reads subresource `sub` of `id` (a mapped read: a staging
    /// resource) into `out` at `row` bytes a row; `false`: nothing to
    /// read (the layer keeps its copy).
    fn read(&mut self, id: u32, sub: u32, row: u32, out: &mut [u8]) -> bool;
    /// A query's data (`GetData`): `S_OK`, or `S_FALSE` not yet.
    fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32;
    /// `Present(sync, flags)`.
    fn present(&mut self, sync: u32, flags: u32) -> u32;
    /// The back buffer as it is now, BGRA rows (the harness's `shot`);
    /// `None` without a real one.
    fn back_buffer_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        None
    }
}

/// The recording null device: no GPU, every object a stand-in.
#[derive(Debug, Default)]
pub struct Null {
    queries: HashMap<u32, u32>,
    clock: u64,
}

impl Backend for Null {
    fn name(&self) -> &'static str {
        "null (recording)"
    }
    fn create_device(&mut self, _: &SwapChainDesc, _: u32) -> u32 {
        S_OK
    }
    fn back_buffer(&mut self, _: u32) -> u32 {
        S_OK
    }
    fn create(&mut self, id: u32, what: &Create<'_>) -> u32 {
        if let Create::Query(d) = what {
            self.queries.insert(id, d[0]);
        }
        S_OK
    }
    fn release(&mut self, _: u32) {}
    fn op(&mut self, _: &Op<'_>) {}
    fn read(&mut self, _: u32, _: u32, _: u32, _: &mut [u8]) -> bool {
        false
    }
    fn query_data(&mut self, id: u32, out: &mut [u8]) -> u32 {
        out.fill(0);
        match self.queries.get(&id) {
            // EVENT: done
            Some(0) => put(out, 0, &1u32.to_le_bytes()),
            // TIMESTAMP: a 1 GHz clock, a millisecond a query
            Some(2) => {
                self.clock += 1_000_000;
                put(out, 0, &self.clock.to_le_bytes());
            }
            // TIMESTAMP_DISJOINT: 1 GHz, not disjoint
            Some(3) => put(out, 0, &1_000_000_000u64.to_le_bytes()),
            _ => {}
        }
        S_OK
    }
    fn present(&mut self, _: u32, _: u32) -> u32 {
        S_OK
    }
}

fn put(out: &mut [u8], at: usize, b: &[u8]) {
    let n = b.len().min(out.len().saturating_sub(at));
    out[at..at + n].copy_from_slice(&b[..n]);
}

/// A buffer or texture.
#[derive(Debug, Clone)]
struct Resource {
    /// 0 buffer, 1–3 a texture's dimensions.
    dims: u8,
    /// The description's words.
    desc: Vec<u32>,
    /// Each subresource: offset into the copy, row pitch, depth pitch,
    /// bytes.
    subs: Vec<(u32, u32, u32, u32)>,
    /// The arena copy of the contents (`Map` hands it out), 0 until
    /// needed.
    copy: u32,
    /// The size of the copy.
    size: u32,
    /// What the stream last saw of a non-constant buffer (for the
    /// changed span of `UpdateBuffer`).
    seen: Vec<u8>,
}

impl Resource {
    fn is_constant_buffer(&self) -> bool {
        self.dims == 0 && self.desc[2] & 0x4 != 0
    }
}

/// A view: its class, resource and description words.
#[derive(Debug, Clone)]
struct View {
    resource: u32,
    desc: Vec<u8>,
}

/// What is bound on the immediate context (as the hook's layer keeps
/// it, `ffx-hook` `gfx::Bound`).
#[derive(Debug, Clone)]
struct Bound {
    vs: u32,
    ps: u32,
    gs: u32,
    layout: u32,
    topology: u32,
    vertex: [(u32, u32, u32); 32],
    index: (u32, u32, u32),
    vs_cb: [(u32, u32, u32); 14],
    ps_cb: [(u32, u32, u32); 14],
    vs_srv: [u32; 128],
    ps_srv: [u32; 128],
    vs_sampler: [u32; 16],
    ps_sampler: [u32; 16],
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
            vs: 0,
            ps: 0,
            gs: 0,
            layout: 0,
            topology: 0,
            vertex: [(0, 0, 0); 32],
            index: (0, 0, 0),
            vs_cb: [(0, 0, 0); 14],
            ps_cb: [(0, 0, 0); 14],
            vs_srv: [0; 128],
            ps_srv: [0; 128],
            vs_sampler: [0; 16],
            ps_sampler: [0; 16],
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

/// An open `Map`.
#[derive(Debug, Clone, Copy)]
struct Mapping {
    kind: u32,
}

/// The layer: the objects, the bound state, the stream and the log.
pub struct Gfx {
    com: Com,
    backend: Box<dyn Backend>,
    config: Config,
    /// The device.
    pub device: u32,
    /// The immediate context.
    pub context: u32,
    /// The swap chain.
    pub swap_chain: u32,
    back_buffer: u32,
    factory: u32,
    adapter: u32,
    output: u32,
    dxgi_device: u32,
    desc: SwapChainDesc,
    flags: u32,
    resources: HashMap<u32, Resource>,
    views: HashMap<u32, View>,
    /// Shaders, layouts, states, queries: their description bytes.
    descs: HashMap<u32, Vec<u8>>,
    maps: HashMap<(u32, u32), Mapping>,
    bound: Bound,
    cmds: Vec<Cmd>,
    /// `Present` calls so far.
    pub presents: u32,
    stream: Vec<u8>,
    log: Vec<String>,
    /// Calls per method (`Interface::Method`).
    pub counts: BTreeMap<String, u64>,
    /// Methods the layer doesn't handle, as first called.
    pub unhandled: BTreeSet<String>,
    /// Draws, clears and copies per frame, the frame being made last.
    pub frame_sizes: Vec<(u32, u32, u32)>,
    /// The objects of Iggy's composite ([`Gfx::composite`]).
    composite: composite::Objects,
}

impl std::fmt::Debug for Gfx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gfx")
            .field("backend", &self.backend.name())
            .field("presents", &self.presents)
            .finish_non_exhaustive()
    }
}

/// Reads `n` words from `at` (none when `at` is 0).
fn words(mem: &impl ffx_bridge::Mem, at: u32, n: u32) -> Vec<u32> {
    if at == 0 {
        return Vec::new();
    }
    (0..n.min(4096)).map(|i| mem.read(at + 4 * i)).collect()
}

/// Reads `n` bytes from `at`.
fn bytes(mem: &impl ffx_bridge::Mem, at: u32, n: u32) -> Vec<u8> {
    let mut v = vec![0u8; n as usize];
    if at != 0 {
        mem.read_bytes(at, &mut v);
    }
    v
}

/// A NUL-terminated string.
fn c_string(mem: &impl ffx_bridge::Mem, at: u32) -> String {
    let mut s = Vec::new();
    if at == 0 {
        return String::new();
    }
    for i in 0..256 {
        let b: u8 = mem.read(at + i);
        if b == 0 {
            break;
        }
        s.push(b);
    }
    String::from_utf8_lossy(&s).into_owned()
}

/// A `D3D11_BOX` (6 words) or none.
fn read_box(mem: &impl ffx_bridge::Mem, at: u32) -> Option<[u32; 6]> {
    (at != 0).then(|| mem.read(at))
}

/// Bytes per block and the block's width (1, or 4 for block-compressed
/// formats) of a DXGI format.
#[must_use]
pub fn format_block(format: u32) -> (u32, u32) {
    match format {
        1..=4 => (16, 1),
        5..=8 => (12, 1),
        9..=22 => (8, 1),
        23..=47 | 67..=69 | 87..=93 => (4, 1),
        48..=59 | 85 | 86 | 115 => (2, 1),
        60..=65 => (1, 1),
        70..=72 | 79..=81 => (8, 4),
        73..=78 | 82..=84 | 94..=99 => (16, 4),
        _ => (4, 1),
    }
}

/// Each subresource of a texture (`dims`, its description words): row
/// pitch, rows of blocks, depth, in D3D11's order (mip fastest, then
/// array slice).
fn texture_subs(dims: u8, d: &[u32]) -> Vec<(u32, u32, u32)> {
    let (w, h, depth, mips, array, format) = match dims {
        1 => (d[0], 1, 1, d[1], d[2], d[3]),
        2 => (d[0], d[1], 1, d[2], d[3], d[4]),
        _ => (d[0], d[1], d[2], d[3], 1, d[4]),
    };
    let full = 32 - w.max(h).max(depth).max(1).leading_zeros();
    let mips = if mips == 0 { full } else { mips.min(full) };
    let (block, bw) = format_block(format);
    let mut out = Vec::new();
    for _ in 0..array.max(1) {
        for m in 0..mips {
            let mw = (w >> m).max(1);
            let mh = (h >> m).max(1);
            let md = (depth >> m).max(1);
            let row = mw.div_ceil(bw) * block;
            let rows = mh.div_ceil(bw);
            out.push((row, rows, md));
        }
    }
    out
}

/// One argument word as a float.
fn f(w: u32) -> f32 {
    f32::from_bits(w)
}

impl Gfx {
    /// The layer over a new device and swap chain: makes the objects, asks
    /// the back end for its device.
    ///
    /// # Errors
    /// When the COM stub region can't be mapped.
    pub fn new(e: &Exec, config: Config, desc: SwapChainDesc, flags: u32) -> crate::Result<Self> {
        let backend: Box<dyn Backend> = match config.backend {
            Choice::Null => Box::new(Null::default()),
            Choice::Default => default_backend(),
        };
        let mut com = Com::new(e, method)?;
        let device = com.create(e, class::DEVICE);
        let context = com.create(e, class::CONTEXT);
        let swap_chain = com.create(e, class::SWAP_CHAIN);
        let back_buffer = com.create(e, class::TEXTURE2D);
        let mut g = Self {
            com,
            backend,
            config,
            device,
            context,
            swap_chain,
            back_buffer,
            factory: 0,
            adapter: 0,
            output: 0,
            dxgi_device: 0,
            desc,
            flags,
            resources: HashMap::new(),
            views: HashMap::new(),
            descs: HashMap::new(),
            maps: HashMap::new(),
            bound: Bound::default(),
            cmds: Vec::new(),
            presents: 0,
            stream: Vec::new(),
            log: Vec::new(),
            counts: BTreeMap::new(),
            unhandled: BTreeSet::new(),
            frame_sizes: vec![(0, 0, 0)],
            composite: composite::Objects::default(),
        };
        let bb = desc.back_buffer();
        g.resources.insert(
            back_buffer,
            Resource {
                dims: 2,
                desc: bb.to_vec(),
                subs: Vec::new(),
                copy: 0,
                size: 0,
                seen: Vec::new(),
            },
        );
        Ok(g)
    }

    /// The back end's name.
    #[must_use]
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }

    /// The recorded stream (`capture.ffxgfx`): every object message, then
    /// a frame per `Present`, and the frame in progress.
    #[must_use]
    pub fn stream(&self) -> Vec<u8> {
        let mut out = self.stream.clone();
        if !self.cmds.is_empty() {
            Msg::Frame(self.frame()).write(&mut out);
        }
        out
    }

    /// The back buffer as BGRA rows (`None` on the null device): the
    /// harness's screenshots between two `Present`s (`crate::harness`).
    pub fn back_buffer_image(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        self.backend.back_buffer_image()
    }

    /// The text log: one line per call (the first [`LOG_LINES`]).
    #[must_use]
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// Live objects per interface.
    #[must_use]
    pub fn census(&self) -> Vec<(&'static str, usize)> {
        self.com.census()
    }

    fn frame(&self) -> ffx_gfx::Frame {
        ffx_gfx::Frame {
            number: self.presents,
            width: self.desc.width(),
            height: self.desc.height(),
            cmds: self.cmds.clone(),
        }
    }

    fn emit(&mut self, m: &Msg) {
        if !self.config.no_record {
            m.write(&mut self.stream);
        }
    }

    /// Whether the layer keeps its recording ([`Config::no_record`]).
    #[must_use]
    pub fn records(&self) -> bool {
        !self.config.no_record
    }

    /// Hands `h` out through `out` with a reference (`GetDevice`,
    /// `GetImmediateContext`, ...).
    fn hand_out(&mut self, e: &Exec, h: u32, out: u32) -> u32 {
        if out == 0 {
            return E_POINTER;
        }
        if h != 0 {
            self.com.add_ref(h);
        }
        e.mem().write(out, h);
        if h == 0 { E_NOINTERFACE } else { S_OK }
    }

    fn factory(&mut self, e: &Exec) -> u32 {
        if self.factory == 0 {
            self.factory = self.com.create(e, class::FACTORY);
        }
        self.factory
    }

    fn adapter(&mut self, e: &Exec) -> u32 {
        if self.adapter == 0 {
            self.adapter = self.com.create(e, class::ADAPTER);
        }
        self.adapter
    }

    fn output(&mut self, e: &Exec) -> u32 {
        if self.output == 0 {
            self.output = self.com.create(e, class::OUTPUT);
        }
        self.output
    }

    fn dxgi_device(&mut self, e: &Exec) -> u32 {
        if self.dxgi_device == 0 {
            self.dxgi_device = self.com.create(e, class::DXGI_DEVICE);
        }
        self.dxgi_device
    }

    /// `QueryInterface(this, riid, ppv)`.
    fn query_interface(&mut self, e: &Exec, this: u32, class: usize, riid: u32, out: u32) -> u32 {
        let mem = e.mem();
        let iid = com::read_guid(&mem, riid);
        let name = com::iid_name(iid);
        let target = match name {
            Some(n) if com::implements(class, n) => this,
            Some("IDXGIDevice" | "IDXGIDevice1") if class == class::DEVICE => self.dxgi_device(e),
            Some("ID3D11Device" | "ID3D11Device1") if class == class::DXGI_DEVICE => self.device,
            _ => 0,
        };
        self.note(&format!(
            "  QueryInterface {} -> {}",
            name.map_or_else(|| com::iid_text(iid), str::to_string),
            if target == 0 { "E_NOINTERFACE" } else { "S_OK" }
        ));
        if out == 0 {
            return E_POINTER;
        }
        if target == 0 {
            e.mem().write(out, 0u32);
            return E_NOINTERFACE;
        }
        self.hand_out(e, target, out)
    }

    fn note(&mut self, line: &str) {
        if self.log.len() < LOG_LINES {
            self.log.push(line.to_string());
        }
    }

    /// Makes object `class` for a `Create*` call: the back end's
    /// `HRESULT`, the handle through `out`.
    fn make(&mut self, e: &Exec, class: usize, what: &Create<'_>, out: u32) -> (u32, u32) {
        if out == 0 {
            // a validation call (D3D11 returns S_FALSE without creating)
            return (S_FALSE, 0);
        }
        let h = self.com.create(e, class);
        if h == 0 {
            return (E_OUTOFMEMORY, 0);
        }
        let hr = self.backend.create(h, what);
        if hr >> 31 != 0 {
            e.mem().write(out, 0u32);
            return (hr, 0);
        }
        e.mem().write(out, h);
        (S_OK, h)
    }

    /// `CreateBuffer(this, desc, init, out)`.
    fn create_buffer(&mut self, e: &Exec, a: &[u32]) -> u32 {
        let mem = e.mem();
        if a[1] == 0 {
            return E_INVALIDARG;
        }
        let desc: [u32; 6] = mem.read(a[1]);
        let init = (a[2] != 0).then(|| bytes(&mem, mem.read(a[2]), desc[0]));
        let what = Create::Buffer {
            desc,
            init: init.as_deref(),
        };
        let (hr, h) = self.make(e, class::BUFFER, &what, a[3]);
        if h != 0 {
            let copy = e.alloc(desc[0].max(16));
            if let Some(b) = &init {
                e.mem().write_bytes(copy, b);
            }
            let cb = desc[2] & 0x4 != 0;
            self.resources.insert(
                h,
                Resource {
                    dims: 0,
                    desc: desc.to_vec(),
                    subs: vec![(0, desc[0], desc[0], desc[0])],
                    copy,
                    size: desc[0],
                    seen: if cb {
                        Vec::new()
                    } else {
                        init.clone().unwrap_or_default()
                    },
                },
            );
            self.emit(&Msg::Buffer {
                id: h,
                desc,
                data: init.unwrap_or_default(),
            });
        }
        hr
    }

    /// `CreateTexture{1,2,3}D(this, desc, init, out)`.
    fn create_texture(&mut self, e: &Exec, dims: u8, a: &[u32]) -> u32 {
        let mem = e.mem();
        if a[1] == 0 {
            return E_INVALIDARG;
        }
        let n = match dims {
            1 => 9,
            2 => 11,
            _ => 10,
        };
        let desc = words(&mem, a[1], n);
        let layout = texture_subs(dims, &desc);
        let mut init = Vec::new();
        if a[2] != 0 {
            for (i, &(row, rows, depth)) in layout.iter().enumerate() {
                let at = a[2] + 12 * u32::try_from(i).unwrap_or(0);
                let [p, pitch, slice]: [u32; 3] = mem.read(at);
                let pitch = if pitch == 0 { row } else { pitch };
                let slice = if slice == 0 { pitch * rows } else { slice };
                let size = slice * (depth - 1) + pitch * (rows - 1) + row;
                init.push((pitch, slice, bytes(&mem, p, size)));
            }
        }
        let class = match dims {
            1 => class::TEXTURE1D,
            2 => class::TEXTURE2D,
            _ => class::TEXTURE3D,
        };
        let what = Create::Texture {
            dims,
            desc: &desc,
            init: &init,
        };
        let (hr, h) = self.make(e, class, &what, a[3]);
        if h != 0 {
            let mut subs = Vec::new();
            let mut size = 0u32;
            for &(row, rows, depth) in &layout {
                let slice = row * rows;
                subs.push((size, row, slice, slice * depth));
                size = size.saturating_add(slice * depth);
            }
            if dims == 2 {
                self.emit(&Msg::Texture {
                    id: h,
                    desc: desc.clone().try_into().unwrap_or([0; 11]),
                    subresources: init
                        .iter()
                        .map(|(pitch, _, data)| Subresource {
                            row_pitch: *pitch,
                            data: data.clone(),
                        })
                        .collect(),
                });
            }
            let mut r = Resource {
                dims,
                desc,
                subs,
                copy: 0,
                size,
                seen: Vec::new(),
            };
            // the contents only when the CPU can map it
            let usage = match dims {
                1 => 4,
                2 => 7,
                _ => 5,
            };
            if r.desc.get(usage).is_some_and(|&u| u >= 2) {
                r.copy = e.alloc(size.max(16));
                let mut mem = e.mem();
                for (i, (pitch, slice, data)) in init.iter().enumerate() {
                    let (off, row, rslice, _) = r.subs[i];
                    copy_rows(
                        &mut mem,
                        r.copy + off,
                        row,
                        rslice,
                        data,
                        *pitch,
                        *slice,
                        layout[i],
                    );
                }
            }
            self.resources.insert(h, r);
        }
        hr
    }

    /// `Create{ShaderResource,RenderTarget,DepthStencil,UnorderedAccess}View(this, resource, desc, out)`.
    fn create_view(&mut self, e: &Exec, class: usize, a: &[u32]) -> u32 {
        let mem = e.mem();
        let n = match class {
            class::RTV => 20,
            _ => 24,
        };
        let desc = (a[2] != 0).then(|| bytes(&mem, a[2], n));
        let what = Create::View {
            class,
            resource: a[1],
            desc: desc.as_deref(),
        };
        let (hr, h) = self.make(e, class, &what, a[3]);
        if h != 0 {
            self.com.add_ref(a[1]);
            let kind = match class {
                class::RTV => Some(ViewKind::RenderTarget),
                class::DSV => Some(ViewKind::DepthStencil),
                class::SRV => Some(ViewKind::ShaderResource),
                _ => None,
            };
            let words_of = desc.as_ref().map(|d| {
                let mut w = [0u32; 6];
                for (i, c) in d.chunks(4).enumerate().take(6) {
                    w[i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                }
                w
            });
            if let Some(kind) = kind {
                self.emit(&Msg::View {
                    id: h,
                    kind,
                    resource: a[1],
                    desc: words_of,
                });
            }
            self.views.insert(
                h,
                View {
                    resource: a[1],
                    desc: desc.unwrap_or_default(),
                },
            );
        }
        hr
    }

    /// `Create*Shader(this, code, len, linkage, out)` (and the
    /// stream-output geometry shader: `out` last).
    fn create_shader(&mut self, e: &Exec, stage: u8, a: &[u32], out: u32) -> u32 {
        let mem = e.mem();
        let code = bytes(&mem, a[1], a[2]);
        let class = match stage {
            b'v' => class::VERTEX_SHADER,
            b'p' => class::PIXEL_SHADER,
            b'g' => class::GEOMETRY_SHADER,
            b'h' => class::HULL_SHADER,
            b'd' => class::DOMAIN_SHADER,
            _ => class::COMPUTE_SHADER,
        };
        let what = Create::Shader {
            stage,
            bytecode: &code,
        };
        let (hr, h) = self.make(e, class, &what, out);
        if h != 0 {
            if matches!(stage, b'v' | b'p') {
                self.emit(&Msg::Shader {
                    id: h,
                    kind: stage,
                    bytecode: code.clone(),
                });
            }
            self.descs.insert(h, code);
        }
        hr
    }

    /// `CreateInputLayout(this, elements, n, code, len, out)`.
    fn create_layout(&mut self, e: &Exec, a: &[u32]) -> u32 {
        let mem = e.mem();
        let elements: Vec<Element> = (0..a[2].min(32))
            .map(|i| {
                let w: [u32; 7] = mem.read(a[1] + 28 * i);
                Element {
                    semantic: c_string(&mem, w[0]),
                    index: w[1],
                    format: w[2],
                    slot: w[3],
                    offset: w[4],
                    per_instance: w[5] != 0,
                    step: w[6],
                }
            })
            .collect();
        let code = bytes(&mem, a[3], a[4]);
        let what = Create::Layout {
            elements: &elements,
            bytecode: &code,
        };
        let (hr, h) = self.make(e, class::INPUT_LAYOUT, &what, a[5]);
        if h != 0 {
            self.emit(&Msg::Layout { id: h, elements });
        }
        hr
    }

    /// `Create{Blend,DepthStencil,Rasterizer,Sampler}State(this, desc, out)`.
    fn create_state(&mut self, e: &Exec, kind: StateKind, a: &[u32]) -> u32 {
        let mem = e.mem();
        let (n, class) = match kind {
            StateKind::Blend => (264, class::BLEND),
            StateKind::Depth => (52, class::DEPTH),
            StateKind::Raster => (40, class::RASTER),
            StateKind::Sampler => (52, class::SAMPLER),
        };
        if a[1] == 0 {
            return E_INVALIDARG;
        }
        let desc = bytes(&mem, a[1], n);
        let what = Create::State { kind, desc: &desc };
        let (hr, h) = self.make(e, class, &what, a[2]);
        if h != 0 {
            self.emit(&Msg::State {
                id: h,
                kind,
                desc: desc.clone(),
            });
            self.descs.insert(h, desc);
        }
        hr
    }

    /// `CreateQuery/CreatePredicate(this, desc, out)`.
    fn create_query(&mut self, e: &Exec, a: &[u32]) -> u32 {
        let mem = e.mem();
        if a[1] == 0 {
            return E_INVALIDARG;
        }
        let d: [u32; 2] = mem.read(a[1]);
        let (hr, h) = self.make(e, class::QUERY, &Create::Query(d), a[2]);
        if h != 0 {
            self.descs
                .insert(h, d.iter().flat_map(|w| w.to_le_bytes()).collect());
        }
        hr
    }

    /// Writes a stored description (`GetDesc(this, out)`).
    fn get_desc(&self, e: &Exec, this: u32, out: u32) {
        if out == 0 {
            return;
        }
        let mut mem = e.mem();
        if let Some(r) = self.resources.get(&this) {
            for (i, w) in r.desc.iter().enumerate() {
                mem.write(out + 4 * u32::try_from(i).unwrap_or(0), *w);
            }
        } else if let Some(v) = self.views.get(&this) {
            mem.write_bytes(out, &v.desc);
        } else if let Some(d) = self.descs.get(&this) {
            mem.write_bytes(out, d);
        }
    }

    /// The window of constant buffer (`buffer`, first, count) a draw sees.
    fn window(&self, e: &Exec, (b, first, num): (u32, u32, u32)) -> Vec<u8> {
        let Some(r) = self.resources.get(&b) else {
            return Vec::new();
        };
        let (start, len) = if num == 0 {
            (0, r.size)
        } else {
            (first * 16, num * 16)
        };
        let mut v = vec![0u8; len as usize];
        if r.copy != 0 && start < r.size {
            let n = len.min(r.size - start);
            e.mem().read_bytes(r.copy + start, &mut v[..n as usize]);
        }
        v
    }

    fn draw(&mut self, e: &Exec, kind: ffx_gfx::DrawKind, p: [u32; 5]) {
        use ffx_gfx::DrawKind as K;
        let (count, instances, start, base, first) = match kind {
            K::Indexed => (p[0], 1, p[1], p[2], 0),
            K::Plain => (p[0], 1, p[1], 0, 0),
            K::IndexedInstanced => (p[0], p[1], p[2], p[3], p[4]),
            K::Instanced => (p[0], p[1], p[2], 0, p[3]),
        };
        let b = &self.bound;
        let cbs = |list: &[(u32, u32, u32); 14]| -> Vec<(u8, Vec<u8>)> {
            list.iter()
                .enumerate()
                .filter(|(_, c)| c.0 != 0)
                .map(|(i, c)| (u8::try_from(i).unwrap_or(0), self.window(e, *c)))
                .collect()
        };
        let slots = |list: &[u32]| -> Vec<(u8, u32)> {
            list.iter()
                .enumerate()
                .filter(|(_, v)| **v != 0)
                .map(|(i, v)| (u8::try_from(i).unwrap_or(0), *v))
                .collect()
        };
        let d = ffx_gfx::Draw {
            kind,
            count,
            instances,
            start,
            base: i32::from_ne_bytes(base.to_ne_bytes()),
            first_instance: first,
            topology: b.topology,
            layout: b.layout,
            vs: b.vs,
            ps: b.ps,
            vertex: b
                .vertex
                .iter()
                .enumerate()
                .filter(|(_, v)| v.0 != 0)
                .map(|(i, v)| (u8::try_from(i).unwrap_or(0), v.0, v.1, v.2))
                .collect(),
            index: b.index,
            vs_cb: cbs(&b.vs_cb),
            ps_cb: cbs(&b.ps_cb),
            vs_srv: slots(&b.vs_srv),
            ps_srv: slots(&b.ps_srv),
            vs_samplers: slots(&b.vs_sampler),
            ps_samplers: slots(&b.ps_sampler),
            blend: b.blend,
            blend_factor: b.blend_factor,
            sample_mask: b.sample_mask,
            depth: b.depth,
            stencil_ref: b.stencil_ref,
            raster: b.raster,
            viewport: b.viewport,
            scissor: b.scissor,
            rtv: slots(&b.rtv),
            dsv: b.dsv,
        };
        self.cmds.push(Cmd::Draw(Box::new(d)));
        if let Some(s) = self.frame_sizes.last_mut() {
            s.0 += 1;
        }
    }

    /// The stream's `UpdateBuffer` for a non-constant buffer whose
    /// contents changed: the changed span.
    fn buffer_changed(&mut self, e: &Exec, id: u32) {
        let Some(r) = self.resources.get_mut(&id) else {
            return;
        };
        if r.dims != 0 || r.is_constant_buffer() || r.copy == 0 {
            return;
        }
        let now = bytes(&e.mem(), r.copy, r.size);
        if r.seen.len() != now.len() {
            r.seen = vec![0; now.len()];
        }
        let first = now.iter().zip(&r.seen).position(|(a, b)| a != b);
        let last = now.iter().zip(&r.seen).rposition(|(a, b)| a != b);
        if let (Some(lo), Some(hi)) = (first, last) {
            let data = now[lo..=hi].to_vec();
            r.seen = now;
            self.cmds.push(Cmd::UpdateBuffer {
                id,
                offset: u32::try_from(lo).unwrap_or(0),
                data,
            });
        }
    }

    /// `Map(this, resource, sub, type, flags, out)`.
    fn map(&mut self, e: &Exec, a: &[u32]) -> u32 {
        let (id, sub, kind, out) = (a[1], a[2], a[3], a[5]);
        let Some(r) = self.resources.get_mut(&id) else {
            return E_INVALIDARG;
        };
        if r.copy == 0 {
            r.copy = e.alloc(r.size.max(16));
            if r.copy == 0 {
                return E_OUTOFMEMORY;
            }
        }
        let Some(&(off, row, slice, size)) = r.subs.get(sub as usize) else {
            return E_INVALIDARG;
        };
        let (copy, dims) = (r.copy, r.dims);
        // READ, READ_WRITE: what the GPU has
        if kind == 1 || kind == 3 {
            let mut buf = bytes(&e.mem(), copy + off, size);
            if self.backend.read(id, sub, row, &mut buf) {
                e.mem().write_bytes(copy + off, &buf);
            }
        }
        if out != 0 {
            let mut mem = e.mem();
            mem.write(out, copy + off);
            mem.write(out + 4, if dims == 0 { size } else { row });
            mem.write(out + 8, if dims == 0 { size } else { slice });
        }
        self.maps.insert((id, sub), Mapping { kind });
        S_OK
    }

    /// `Unmap(this, resource, sub)`.
    fn unmap(&mut self, e: &Exec, a: &[u32]) {
        let (id, sub) = (a[1], a[2]);
        let Some(m) = self.maps.remove(&(id, sub)) else {
            return;
        };
        if m.kind == 1 {
            return;
        }
        let Some(r) = self.resources.get(&id) else {
            return;
        };
        let Some(&(off, row, slice, size)) = r.subs.get(sub as usize) else {
            return;
        };
        let data = bytes(&e.mem(), r.copy + off, size);
        self.backend.op(&Op::Unmapped(id, sub, row, slice, &data));
        self.buffer_changed(e, id);
    }

    /// `UpdateSubresource(this, dst, sub, box, data, row, depth)`.
    fn update(&mut self, e: &Exec, a: &[u32]) {
        let mem = e.mem();
        let (id, sub, bx, src, pitch, depth) = (a[1], a[2], read_box(&mem, a[3]), a[4], a[5], a[6]);
        let Some(r) = self.resources.get(&id).cloned() else {
            return;
        };
        if r.dims == 0 {
            let (lo, hi) = bx.map_or((0, r.size), |b| (b[0], b[3].min(r.size)));
            let n = hi.saturating_sub(lo);
            let data = bytes(&mem, src, n);
            self.backend
                .op(&Op::Update(id, sub, bx, &data, pitch, depth));
            if r.copy != 0 {
                e.mem().write_bytes(r.copy + lo, &data);
            }
            self.buffer_changed(e, id);
            return;
        }
        let Some(&(off, row, slice, _)) = r.subs.get(sub as usize) else {
            return;
        };
        let layout = texture_subs(r.dims, &r.desc);
        let (_, rows, d) = layout.get(sub as usize).copied().unwrap_or((row, 1, 1));
        // a box covers part of the subresource: its rows of blocks
        let (block, bw) = format_block(match r.dims {
            1 => r.desc[3],
            _ => r.desc[4],
        });
        let (brow, brows, bd) = bx.map_or((row, rows, d), |b| {
            (
                b[3].saturating_sub(b[0]).div_ceil(bw) * block,
                b[4].saturating_sub(b[1]).div_ceil(bw),
                b[5].saturating_sub(b[2]).max(1),
            )
        });
        let pitch = if pitch == 0 { brow } else { pitch };
        let size = depth * (bd - 1) + pitch * brows.saturating_sub(1) + brow;
        let data = bytes(&mem, src, size);
        self.backend
            .op(&Op::Update(id, sub, bx, &data, pitch, depth));
        if r.copy != 0 && bx.is_none() {
            let mut m = e.mem();
            copy_rows(
                &mut m,
                r.copy + off,
                row,
                slice,
                &data,
                pitch,
                depth,
                (row, rows, d),
            );
        }
        self.emit(&Msg::UpdateTexture {
            id,
            subresource: sub,
            row_pitch: pitch,
            data,
        });
    }

    fn present(&mut self, e: &mut Exec, a: &[u32]) -> u32 {
        let backend = &mut self.backend;
        crate::harness::on_present(|| backend.back_buffer_image());
        let hr = self.backend.present(a[1], a[2]);
        let f = self.frame();
        self.emit(&Msg::Frame(f));
        self.cmds.clear();
        self.presents += 1;
        self.frame_sizes.push((0, 0, 0));
        if self
            .config
            .stop_after_presents
            .is_some_and(|n| self.presents >= n)
        {
            e.end(Stop::Milestone(format!("Present {}", self.presents)));
        }
        hr
    }

    /// One method call: what it returns.
    #[allow(clippy::too_many_lines)] // one arm per method
    fn call(
        &mut self,
        e: &mut Exec,
        class: usize,
        iface: &str,
        name: &str,
        a: &[u32],
    ) -> Option<u32> {
        let this = a[0];
        let mut mem = e.mem();
        let hr = match (iface, name) {
            ("IUnknown", "QueryInterface") => self.query_interface(e, this, class, a[1], a[2]),
            ("IUnknown", "AddRef") => self.com.add_ref(this),
            ("IUnknown", "Release") => {
                let n = self.com.release(this);
                if n == 0 {
                    self.backend.release(this);
                    if let Some(v) = self.views.get(&this) {
                        let r = v.resource;
                        self.com.release(r);
                    }
                    if self.resources.contains_key(&this)
                        || self.views.contains_key(&this)
                        || self.descs.contains_key(&this)
                    {
                        self.emit(&Msg::Release { id: this });
                    }
                }
                n
            }
            // ID3D11DeviceChild
            ("ID3D11DeviceChild", "GetDevice") => {
                let d = self.device;
                self.hand_out(e, d, a[1]);
                return None;
            }
            ("ID3D11DeviceChild" | "IDXGIObject", "SetPrivateData" | "SetPrivateDataInterface") => {
                S_OK
            }
            ("ID3D11DeviceChild" | "IDXGIObject" | "ID3D11Device", "GetPrivateData") => {
                DXGI_ERROR_NOT_FOUND
            }
            ("ID3D11Device", "SetPrivateData" | "SetPrivateDataInterface") => S_OK,
            // ID3D11Resource
            ("ID3D11Resource", "GetType") => {
                let t = match class {
                    class::BUFFER => 1,
                    class::TEXTURE1D => 2,
                    class::TEXTURE2D => 3,
                    _ => 4,
                };
                if a[1] != 0 {
                    mem.write(a[1], t);
                }
                return None;
            }
            ("ID3D11Resource", "SetEvictionPriority") => return None,
            ("ID3D11Resource", "GetEvictionPriority") => 0,
            ("ID3D11View", "GetResource") => {
                let r = self.views.get(&this).map_or(0, |v| v.resource);
                self.hand_out(e, r, a[1]);
                return None;
            }
            (_, "GetDesc") if class < class::SWAP_CHAIN => {
                self.get_desc(e, this, a[1]);
                return None;
            }
            // ID3D11Device
            ("ID3D11Device", "CreateBuffer") => self.create_buffer(e, a),
            ("ID3D11Device", "CreateTexture1D") => self.create_texture(e, 1, a),
            ("ID3D11Device", "CreateTexture2D") => self.create_texture(e, 2, a),
            ("ID3D11Device", "CreateTexture3D") => self.create_texture(e, 3, a),
            ("ID3D11Device", "CreateShaderResourceView") => self.create_view(e, class::SRV, a),
            ("ID3D11Device", "CreateRenderTargetView") => self.create_view(e, class::RTV, a),
            ("ID3D11Device", "CreateDepthStencilView") => self.create_view(e, class::DSV, a),
            ("ID3D11Device", "CreateUnorderedAccessView") => self.create_view(e, class::UAV, a),
            ("ID3D11Device", "CreateInputLayout") => self.create_layout(e, a),
            ("ID3D11Device", "CreateVertexShader") => self.create_shader(e, b'v', a, a[4]),
            ("ID3D11Device", "CreatePixelShader") => self.create_shader(e, b'p', a, a[4]),
            ("ID3D11Device", "CreateGeometryShader") => self.create_shader(e, b'g', a, a[4]),
            ("ID3D11Device", "CreateGeometryShaderWithStreamOutput") => {
                self.create_shader(e, b'g', a, a[9])
            }
            ("ID3D11Device", "CreateHullShader") => self.create_shader(e, b'h', a, a[4]),
            ("ID3D11Device", "CreateDomainShader") => self.create_shader(e, b'd', a, a[4]),
            ("ID3D11Device", "CreateComputeShader") => self.create_shader(e, b'c', a, a[4]),
            ("ID3D11Device", "CreateBlendState") => self.create_state(e, StateKind::Blend, a),
            ("ID3D11Device", "CreateDepthStencilState") => {
                self.create_state(e, StateKind::Depth, a)
            }
            ("ID3D11Device", "CreateRasterizerState") => self.create_state(e, StateKind::Raster, a),
            ("ID3D11Device", "CreateSamplerState") => self.create_state(e, StateKind::Sampler, a),
            ("ID3D11Device", "CreateQuery" | "CreatePredicate") => self.create_query(e, a),
            ("ID3D11Device", "CheckFormatSupport") => {
                // every usage but the video ones
                if a[2] != 0 {
                    mem.write(a[2], 0x07ff_ffffu32);
                }
                S_OK
            }
            ("ID3D11Device", "CheckMultisampleQualityLevels") => {
                if a[3] != 0 {
                    mem.write(a[3], u32::from(matches!(a[2], 1 | 2 | 4 | 8)));
                }
                S_OK
            }
            ("ID3D11Device", "CheckFeatureSupport") => {
                // nothing optional: zeros
                if a[2] != 0 {
                    mem.write_bytes(a[2], &vec![0u8; a[3].min(256) as usize]);
                }
                S_OK
            }
            ("ID3D11Device", "GetFeatureLevel") => FEATURE_LEVEL_11_0,
            ("ID3D11Device", "GetCreationFlags") => self.flags,
            ("ID3D11Device", "GetDeviceRemovedReason") => S_OK,
            ("ID3D11Device", "SetExceptionMode") => S_OK,
            ("ID3D11Device", "GetExceptionMode") => 0,
            ("ID3D11Device", "GetImmediateContext") => {
                let c = self.context;
                self.hand_out(e, c, a[1]);
                return None;
            }
            // ID3D11DeviceContext
            ("ID3D11DeviceContext", m) => return self.context_call(e, m, a),
            ("ID3D11DeviceContext1", m) => return self.context_call(e, m, a),
            // IDXGISwapChain
            ("IDXGIObject", "GetParent") => {
                let p = match class {
                    class::SWAP_CHAIN | class::ADAPTER => self.factory(e),
                    class::OUTPUT | class::DXGI_DEVICE => self.adapter(e),
                    _ => 0,
                };
                self.hand_out(e, p, a[2])
            }
            ("IDXGIDeviceSubObject", "GetDevice") => {
                let d = self.device;
                self.hand_out(e, d, a[2])
            }
            ("IDXGISwapChain", "Present") => self.present(e, a),
            ("IDXGISwapChain", "GetBuffer") => {
                if a[1] == 0 {
                    let b = self.back_buffer;
                    let hr = self.backend.back_buffer(b);
                    if hr == S_OK {
                        self.hand_out(e, b, a[3])
                    } else {
                        hr
                    }
                } else {
                    DXGI_ERROR_NOT_FOUND
                }
            }
            ("IDXGISwapChain", "SetFullscreenState") => S_OK,
            ("IDXGISwapChain", "GetFullscreenState") => {
                if a[1] != 0 {
                    mem.write(a[1], 0u32);
                }
                if a[2] != 0 {
                    mem.write(a[2], 0u32);
                }
                S_OK
            }
            ("IDXGISwapChain", "GetDesc") => {
                if a[1] != 0 {
                    mem.write(a[1], self.desc.0);
                }
                S_OK
            }
            ("IDXGISwapChain", "ResizeBuffers") => {
                // (count, width, height, format, flags); 0 keeps the size
                if a[2] != 0 {
                    self.desc.0[0] = a[2];
                }
                if a[3] != 0 {
                    self.desc.0[1] = a[3];
                }
                if a[4] != 0 {
                    self.desc.0[4] = a[4];
                }
                let bb = self.desc.back_buffer().to_vec();
                if let Some(r) = self.resources.get_mut(&self.back_buffer) {
                    r.desc = bb;
                }
                S_OK
            }
            ("IDXGISwapChain", "ResizeTarget") => S_OK,
            ("IDXGISwapChain", "GetContainingOutput") => {
                let o = self.output(e);
                self.hand_out(e, o, a[1])
            }
            ("IDXGISwapChain", "GetLastPresentCount") => {
                if a[1] != 0 {
                    mem.write(a[1], self.presents);
                }
                S_OK
            }
            // IDXGIFactory
            ("IDXGIFactory", "EnumAdapters") | ("IDXGIFactory1", "EnumAdapters1") => {
                if a[1] == 0 {
                    let ad = self.adapter(e);
                    self.hand_out(e, ad, a[2])
                } else {
                    DXGI_ERROR_NOT_FOUND
                }
            }
            ("IDXGIFactory", "MakeWindowAssociation") => S_OK,
            ("IDXGIFactory", "GetWindowAssociation") => {
                if a[1] != 0 {
                    mem.write(a[1], self.desc.window());
                }
                S_OK
            }
            ("IDXGIFactory1", "IsCurrent") => 1,
            // IDXGIAdapter
            ("IDXGIAdapter", "EnumOutputs") => {
                if a[1] == 0 {
                    let o = self.output(e);
                    self.hand_out(e, o, a[2])
                } else {
                    DXGI_ERROR_NOT_FOUND
                }
            }
            ("IDXGIAdapter", "GetDesc") | ("IDXGIAdapter1", "GetDesc1") => {
                adapter_desc(&mut mem, a[1], name == "GetDesc1");
                S_OK
            }
            // IDXGIOutput
            ("IDXGIOutput", "GetDesc") => {
                if a[1] != 0 {
                    // DeviceName[32] (WCHAR), DesktopCoordinates, attached,
                    // rotation IDENTITY, monitor
                    let mut d = [0u8; 100];
                    for (i, c) in "\\\\.\\DISPLAY1".encode_utf16().enumerate() {
                        d[2 * i..2 * i + 2].copy_from_slice(&c.to_le_bytes());
                    }
                    d[72..76].copy_from_slice(&1920u32.to_le_bytes());
                    d[76..80].copy_from_slice(&1080u32.to_le_bytes());
                    d[80..84].copy_from_slice(&1u32.to_le_bytes());
                    d[84..88].copy_from_slice(&1u32.to_le_bytes());
                    d[88..92].copy_from_slice(&0x0001_0001u32.to_le_bytes());
                    mem.write_bytes(a[1], &d[..92]);
                }
                S_OK
            }
            ("IDXGIOutput", "GetDisplayModeList") => {
                // (format, flags, &count, modes): one mode, 1920 x 1080 at 60
                if a[3] != 0 {
                    if a[4] != 0 {
                        let n: u32 = mem.read(a[3]);
                        if n >= 1 {
                            mem.write(a[4], [1920u32, 1080, 60, 1, a[1], 0, 0]);
                        }
                    }
                    mem.write(a[3], 1u32);
                }
                S_OK
            }
            ("IDXGIOutput", "FindClosestMatchingMode") => {
                if a[2] != 0 {
                    let format: u32 = if a[1] == 0 { 0 } else { mem.read(a[1] + 16) };
                    let format = if format == 0 { 28 } else { format };
                    mem.write(a[2], [1920u32, 1080, 60, 1, format, 0, 0]);
                }
                S_OK
            }
            ("IDXGIOutput", "WaitForVBlank") => S_OK,
            // IDXGIDevice
            ("IDXGIDevice", "GetAdapter") => {
                let ad = self.adapter(e);
                self.hand_out(e, ad, a[1])
            }
            ("IDXGIDevice", "SetGPUThreadPriority") => S_OK,
            ("IDXGIDevice1", "SetMaximumFrameLatency") => S_OK,
            ("IDXGIDevice1", "GetMaximumFrameLatency") => {
                if a[1] != 0 {
                    mem.write(a[1], 3u32);
                }
                S_OK
            }
            // DirectInput: a keyboard and a mouse (EnumDevices: in
            // `method`), untouched
            ("IDirectInput8A", "CreateDevice") => {
                if a[2] == 0 {
                    return Some(E_POINTER);
                }
                let iid = com::read_guid(&mem, a[1]);
                self.note(&format!("  DirectInput device {}", com::iid_text(iid)));
                let h = self.com.create(e, class::DINPUT_DEVICE);
                e.mem().write(a[2], h);
                S_OK
            }
            ("IDirectInput8A", "GetDeviceStatus") => DI_NOTATTACHED,
            ("IDirectInput8A", "Initialize") => S_OK,
            (
                "IDirectInputDevice8A",
                "SetDataFormat"
                | "SetCooperativeLevel"
                | "SetProperty"
                | "Acquire"
                | "Unacquire"
                | "SetEventNotification"
                | "Poll",
            ) => S_OK,
            ("IDirectInputDevice8A", "GetDeviceState") => {
                if a[2] != 0 {
                    let mut state = vec![0u8; a[1].min(4096) as usize];
                    if state.len() == 256 {
                        // the keyboard: the keys a harness test holds
                        crate::harness::keyboard_state(&mut state);
                    }
                    mem.write_bytes(a[2], &state);
                }
                S_OK
            }
            ("IDirectInputDevice8A", "GetDeviceData") => {
                if a[3] != 0 {
                    mem.write(a[3], 0u32);
                }
                S_OK
            }
            _ => {
                self.unhandled.insert(format!("{iface}::{name}"));
                return Some(E_NOTIMPL);
            }
        };
        Some(hr)
    }

    /// One immediate-context call.
    #[allow(clippy::too_many_lines)] // one arm per method
    fn context_call(&mut self, e: &mut Exec, name: &str, a: &[u32]) -> Option<u32> {
        let mem = e.mem();
        let stage = name.as_bytes()[0].to_ascii_lowercase();
        let list = |at: u32, n: u32| words(&mem, at, n);
        match name {
            "IASetInputLayout" => {
                self.bound.layout = a[1];
                self.backend.op(&Op::InputLayout(a[1]));
            }
            "IASetVertexBuffers" => {
                let (start, n) = (a[1], a[2].min(32));
                let (b, s, o) = (list(a[3], n), list(a[4], n), list(a[5], n));
                for i in 0..n as usize {
                    if let Some(slot) = self.bound.vertex.get_mut(start as usize + i) {
                        *slot = (
                            b.get(i).copied().unwrap_or(0),
                            s.get(i).copied().unwrap_or(0),
                            o.get(i).copied().unwrap_or(0),
                        );
                    }
                }
                self.backend.op(&Op::VertexBuffers(start, &b, &s, &o));
            }
            "IASetIndexBuffer" => {
                self.bound.index = (a[1], a[2], a[3]);
                self.backend.op(&Op::IndexBuffer(a[1], a[2], a[3]));
            }
            "IASetPrimitiveTopology" => {
                self.bound.topology = a[1];
                self.backend.op(&Op::Topology(a[1]));
            }
            "VSSetShader" | "PSSetShader" | "GSSetShader" | "HSSetShader" | "DSSetShader"
            | "CSSetShader" => {
                match stage {
                    b'v' => self.bound.vs = a[1],
                    b'p' => self.bound.ps = a[1],
                    b'g' => self.bound.gs = a[1],
                    _ => {}
                }
                self.backend.op(&Op::Shader(stage, a[1]));
            }
            "VSSetConstantBuffers"
            | "PSSetConstantBuffers"
            | "GSSetConstantBuffers"
            | "HSSetConstantBuffers"
            | "DSSetConstantBuffers"
            | "CSSetConstantBuffers"
            | "VSSetConstantBuffers1"
            | "PSSetConstantBuffers1"
            | "GSSetConstantBuffers1"
            | "HSSetConstantBuffers1"
            | "DSSetConstantBuffers1"
            | "CSSetConstantBuffers1" => {
                let (start, n) = (a[1], a[2].min(14));
                let b = list(a[3], n);
                let ranges = name.ends_with('1').then(|| (list(a[4], n), list(a[5], n)));
                let target = match stage {
                    b'v' => Some(&mut self.bound.vs_cb),
                    b'p' => Some(&mut self.bound.ps_cb),
                    _ => None,
                };
                if let Some(t) = target {
                    for i in 0..n as usize {
                        let (first, num) = ranges.as_ref().map_or((0, 0), |(f, c)| {
                            (
                                f.get(i).copied().unwrap_or(0),
                                c.get(i).copied().unwrap_or(0),
                            )
                        });
                        if let Some(slot) = t.get_mut(start as usize + i) {
                            *slot = (b.get(i).copied().unwrap_or(0), first, num);
                        }
                    }
                }
                let r = ranges.as_ref().map(|(f, c)| (f.as_slice(), c.as_slice()));
                self.backend.op(&Op::ConstantBuffers(stage, start, &b, r));
            }
            "VSSetShaderResources"
            | "PSSetShaderResources"
            | "GSSetShaderResources"
            | "HSSetShaderResources"
            | "DSSetShaderResources"
            | "CSSetShaderResources" => {
                let (start, n) = (a[1], a[2].min(128));
                let v = list(a[3], n);
                let target = match stage {
                    b'v' => Some(&mut self.bound.vs_srv),
                    b'p' => Some(&mut self.bound.ps_srv),
                    _ => None,
                };
                if let Some(t) = target {
                    for (i, x) in v.iter().enumerate() {
                        if let Some(slot) = t.get_mut(start as usize + i) {
                            *slot = *x;
                        }
                    }
                }
                self.backend.op(&Op::Resources(stage, start, &v));
            }
            "VSSetSamplers" | "PSSetSamplers" | "GSSetSamplers" | "HSSetSamplers"
            | "DSSetSamplers" | "CSSetSamplers" => {
                let (start, n) = (a[1], a[2].min(16));
                let v = list(a[3], n);
                let target = match stage {
                    b'v' => Some(&mut self.bound.vs_sampler),
                    b'p' => Some(&mut self.bound.ps_sampler),
                    _ => None,
                };
                if let Some(t) = target {
                    for (i, x) in v.iter().enumerate() {
                        if let Some(slot) = t.get_mut(start as usize + i) {
                            *slot = *x;
                        }
                    }
                }
                self.backend.op(&Op::Samplers(stage, start, &v));
            }
            "OMSetRenderTargets" | "OMSetRenderTargetsAndUnorderedAccessViews" => {
                // D3D11_KEEP_RENDER_TARGETS_AND_DEPTH_STENCIL: the UAVs only
                if a[1] != u32::MAX {
                    let v = list(a[2], a[1].min(8));
                    self.bound.rtv = [0; 8];
                    self.bound.rtv[..v.len()].copy_from_slice(&v);
                    self.bound.dsv = a[3];
                    self.backend.op(&Op::Targets(&v, a[3]));
                }
            }
            "OMSetBlendState" => {
                let factor = if a[2] == 0 {
                    [1.0; 4]
                } else {
                    mem.read::<[f32; 4]>(a[2])
                };
                self.bound.blend = a[1];
                self.bound.blend_factor = factor;
                self.bound.sample_mask = a[3];
                self.backend.op(&Op::Blend(a[1], factor, a[3]));
            }
            "OMSetDepthStencilState" => {
                self.bound.depth = a[1];
                self.bound.stencil_ref = a[2];
                self.backend.op(&Op::Depth(a[1], a[2]));
            }
            "RSSetState" => {
                self.bound.raster = a[1];
                self.backend.op(&Op::Raster(a[1]));
            }
            "RSSetViewports" => {
                let v: Vec<[f32; 6]> = (0..a[1].min(16)).map(|i| mem.read(a[2] + 24 * i)).collect();
                if let Some(first) = v.first() {
                    self.bound.viewport = *first;
                }
                self.backend.op(&Op::Viewports(&v));
            }
            "RSSetScissorRects" => {
                let v: Vec<[i32; 4]> = (0..a[1].min(16)).map(|i| mem.read(a[2] + 16 * i)).collect();
                if let Some(first) = v.first() {
                    self.bound.scissor = *first;
                }
                self.backend.op(&Op::Scissors(&v));
            }
            "Draw" | "DrawIndexed" | "DrawInstanced" | "DrawIndexedInstanced" => {
                use ffx_gfx::DrawKind as K;
                let kind = match name {
                    "Draw" => K::Plain,
                    "DrawIndexed" => K::Indexed,
                    "DrawInstanced" => K::Instanced,
                    _ => K::IndexedInstanced,
                };
                let mut p = [0u32; 5];
                for (i, w) in a[1..].iter().take(5).enumerate() {
                    p[i] = *w;
                }
                self.draw(e, kind, p);
                self.backend.op(&Op::Draw(kind, p));
            }
            "ClearRenderTargetView" => {
                let color = if a[2] == 0 {
                    [0.0; 4]
                } else {
                    mem.read::<[f32; 4]>(a[2])
                };
                self.cmds.push(Cmd::ClearTarget { view: a[1], color });
                self.count_clear();
                self.backend.op(&Op::ClearTarget(a[1], color));
            }
            "ClearDepthStencilView" => {
                let stencil = u8::try_from(a[4] & 0xff).unwrap_or(0);
                self.cmds.push(Cmd::ClearDepth {
                    view: a[1],
                    flags: a[2],
                    depth: f(a[3]),
                    stencil,
                });
                self.count_clear();
                self.backend
                    .op(&Op::ClearDepth(a[1], a[2], f(a[3]), stencil));
            }
            "CopyResource" => {
                self.copy_contents(e, a[1], a[2]);
                self.texture_copied(a[1], a[2]);
                self.backend.op(&Op::Copy(a[1], a[2]));
            }
            "CopySubresourceRegion" | "CopySubresourceRegion1" => {
                let bx = read_box(&mem, a[8]);
                self.texture_copied(a[1], a[6]);
                self.backend.op(&Op::CopyRegion(
                    [a[1], a[2], a[3], a[4], a[5], a[6], a[7]],
                    bx,
                ));
            }
            "UpdateSubresource" | "UpdateSubresource1" => self.update(e, a),
            "Map" => return Some(self.map(e, a)),
            "Unmap" => self.unmap(e, a),
            "Begin" => self.backend.op(&Op::Begin(a[1])),
            "End" => self.backend.op(&Op::End(a[1])),
            "GetData" => {
                // (query, data, size, flags)
                let mut out = vec![0u8; a[3].min(64) as usize];
                let hr = self.backend.query_data(a[1], &mut out);
                if a[2] != 0 {
                    e.mem().write_bytes(a[2], &out);
                }
                return Some(hr);
            }
            "GenerateMips" => self.backend.op(&Op::GenerateMips(a[1])),
            "ResolveSubresource" => {
                self.texture_copied(a[1], a[3]);
                self.backend
                    .op(&Op::Resolve([a[1], a[2], a[3], a[4], a[5]]));
            }
            "ClearState" => {
                self.bound = Bound::default();
                self.backend.op(&Op::ClearState);
            }
            "Flush" => self.backend.op(&Op::Flush),
            "OMGetRenderTargets" => {
                // (num, views, depth): the bound views, each with a
                // reference, as the API hands them out
                // (`IggyPlayer_tickDraw_00a3d9f0` asks when its device
                // record has none)
                let (num, views, depth) = (a[1].min(8), a[2], a[3]);
                let bound = (self.bound.rtv, self.bound.dsv);
                if views != 0 {
                    for i in 0..num {
                        let v = bound.0[i as usize];
                        if v != 0 {
                            self.com.add_ref(v);
                        }
                        e.mem().write(views + 4 * i, v);
                    }
                }
                if depth != 0 {
                    if bound.1 != 0 {
                        self.com.add_ref(bound.1);
                    }
                    e.mem().write(depth, bound.1);
                }
            }
            "GetType" | "GetContextFlags" => return Some(0),
            "SetPredication"
            | "SetResourceMinLOD"
            | "DiscardResource"
            | "DiscardView"
            | "SOSetTargets"
            | "Dispatch"
            | "DispatchIndirect"
            | "CSSetUnorderedAccessViews" => {}
            "GetResourceMinLOD" => return Some(0),
            m if m.contains("Get") => {
                // a getter the layer doesn't answer yet: listed
                self.unhandled.insert(format!("ID3D11DeviceContext::{m}"));
            }
            m => {
                self.unhandled.insert(format!("ID3D11DeviceContext::{m}"));
            }
        }
        None
    }

    fn count_clear(&mut self) {
        if let Some(s) = self.frame_sizes.last_mut() {
            s.1 += 1;
        }
    }

    /// A copy between textures goes into the stream (`CopyTexture`).
    fn texture_copied(&mut self, dst: u32, src: u32) {
        let tex = |id| self.resources.get(&id).is_some_and(|r| r.dims != 0);
        if tex(dst) && tex(src) {
            self.cmds.push(Cmd::CopyTexture { dst, src });
            if let Some(s) = self.frame_sizes.last_mut() {
                s.2 += 1;
            }
        }
    }

    /// `CopyResource` between two resources the layer keeps contents of.
    fn copy_contents(&mut self, e: &Exec, dst: u32, src: u32) {
        let (Some(d), Some(s)) = (self.resources.get(&dst), self.resources.get(&src)) else {
            return;
        };
        if d.copy != 0 && s.copy != 0 {
            let data = bytes(&e.mem(), s.copy, s.size.min(d.size));
            e.mem().write_bytes(d.copy, &data);
        }
    }
}

/// Copies one subresource's rows from `data` (at `pitch`/`slice`) into
/// the layer's copy (at `row`/`rslice`).
#[allow(clippy::too_many_arguments)] // two layouts
fn copy_rows(
    mem: &mut impl ffx_bridge::Mem,
    at: u32,
    row: u32,
    rslice: u32,
    data: &[u8],
    pitch: u32,
    slice: u32,
    (width, rows, depth): (u32, u32, u32),
) {
    let width = width.min(pitch) as usize;
    for z in 0..depth {
        for y in 0..rows {
            let src = (z * slice + y * pitch) as usize;
            if let Some(chunk) = data.get(src..src + width) {
                mem.write_bytes(at + z * rslice + y * row, chunk);
            }
        }
    }
}

/// `DXGI_ADAPTER_DESC(1)` of the null adapter.
fn adapter_desc(mem: &mut impl ffx_bridge::Mem, at: u32, one: bool) {
    if at == 0 {
        return;
    }
    // Description[128] (WCHAR), vendor, device, subsys, revision,
    // dedicated video, system, shared memory (SIZE_T), LUID, (flags)
    let mut d = vec![0u8; if one { 304 } else { 300 }];
    for (i, c) in "ffx-host null adapter".encode_utf16().enumerate() {
        d[2 * i..2 * i + 2].copy_from_slice(&c.to_le_bytes());
    }
    d[272..276].copy_from_slice(&0x1000_0000u32.to_le_bytes());
    d[280..284].copy_from_slice(&0x2000_0000u32.to_le_bytes());
    mem.write_bytes(at, &d);
}

#[cfg(windows)]
fn default_backend() -> Box<dyn Backend> {
    Box::new(crate::d3d11::D3d11::default())
}

#[cfg(not(windows))]
fn default_backend() -> Box<dyn Backend> {
    Box::new(Null::default())
}

/// Every COM method's trap ([`crate::com::Com::new`]): runs the call on
/// the layer, logs it, and returns as the method's declaration says.
pub fn method(e: &mut Exec, tag: u32, args: &Args) -> Ret {
    let Some((class, _, &(iface, name, _, returns))) = com::method(tag) else {
        return Ret::eax(E_NOTIMPL);
    };
    if (iface, name) == ("IDirectInput8A", "EnumDevices") {
        // calls back into the game, which creates the devices: outside
        // the layer's lock
        return crate::stand_ins::dinput_enum_devices(e, args);
    }
    let shared = Arc::clone(e.shared());
    let mut slot = shared.gfx.lock();
    let Some(g) = slot.as_mut() else {
        return Ret::eax(E_NOTIMPL);
    };
    let key = format!("{iface}::{name}");
    *g.counts.entry(key).or_default() += 1;
    let a: Vec<u32> = {
        let mut v = args.stack.clone();
        v.resize(v.len().max(12), 0);
        v
    };
    let r = g.call(e, class, iface, name, &a);
    if g.log.len() < LOG_LINES {
        let shown = &args.stack[1..];
        let line = match r {
            Some(v) => format!(
                "{}#{:x} {}::{name}({}) -> {v:#x}",
                g.presents,
                args.stack[0],
                CLASSES[class].1,
                hex_list(shown)
            ),
            None => format!(
                "{}#{:x} {}::{name}({})",
                g.presents,
                args.stack[0],
                CLASSES[class].1,
                hex_list(shown)
            ),
        };
        g.log.push(line);
    }
    match returns {
        Returns::Hresult | Returns::Word => Ret::eax(r.unwrap_or(S_OK)),
        Returns::Float => Ret::st0(f64::from(f(r.unwrap_or(0)))),
        Returns::Void => Ret::VOID,
    }
}

fn hex_list(w: &[u32]) -> String {
    w.iter()
        .map(|v| format!("{v:#x}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `D3D11CreateDeviceAndSwapChain(adapter, driver, software, flags,
/// levels, n, sdk, &desc, &swap chain, &device, &level, &context)`: the
/// layer's device and swap chain.
pub fn create_device_and_swap_chain(e: &mut Exec, args: &Args) -> Ret {
    let a = |i: usize| args.stack.get(i).copied().unwrap_or(0);
    if a(7) == 0 {
        return Ret::eax(E_INVALIDARG);
    }
    let desc = SwapChainDesc(e.mem().read(a(7)));
    create(e, Some(desc), a(3), [a(8), a(9), a(10), a(11)])
}

/// `DirectInput8Create(instance, version, riid, &out, outer)`: the
/// `DirectInput` object, on the layer's COM objects (made with the
/// device, which the game creates first).
pub fn direct_input8_create(e: &mut Exec, args: &Args) -> Ret {
    let out = args.stack.get(3).copied().unwrap_or(0);
    let shared = Arc::clone(e.shared());
    let mut slot = shared.gfx.lock();
    let Some(g) = slot.as_mut() else {
        e.stats
            .milestones
            .push("DirectInput8Create before the D3D11 device: refused".into());
        return Ret::eax(com::E_FAIL);
    };
    if out == 0 {
        return Ret::eax(E_POINTER);
    }
    let h = g.com.create(e, class::DINPUT);
    e.mem().write(out, h);
    g.note(&format!("DirectInput8Create -> {h:#x}"));
    Ret::eax(S_OK)
}

/// `D3D11CreateDevice(adapter, driver, software, flags, levels, n, sdk,
/// &device, &level, &context)` asked for a device: the layer's device
/// (and a swap chain nobody asked for, never handed out).
pub fn create_device(e: &mut Exec, args: &Args) -> Ret {
    let a = |i: usize| args.stack.get(i).copied().unwrap_or(0);
    create(e, None, a(3), [0, a(7), a(8), a(9)])
}

fn create(e: &mut Exec, desc: Option<SwapChainDesc>, flags: u32, out: [u32; 4]) -> Ret {
    let shared = Arc::clone(e.shared());
    let mut slot = shared.gfx.lock();
    let config = shared.gfx.config();
    if slot.is_some() {
        // a second device: not on the game's path
        e.stats.milestones.push("a second D3D11 device".into());
        return Ret::eax(com::E_FAIL);
    }
    let d = desc.unwrap_or(SwapChainDesc([
        1920, 1080, 60, 1, 28, 0, 0, 1, 0, 0x20, 1, 0, 1, 0, 0,
    ]));
    let mut g = match Gfx::new(e, config, d, flags) {
        Ok(g) => g,
        Err(err) => {
            e.end(Stop::Failed(format!("gfx: {err}"), 0));
            return Ret::eax(com::E_FAIL);
        }
    };
    let hr = g.backend.create_device(&d, flags);
    if hr >> 31 != 0 {
        return Ret::eax(hr);
    }
    let mut mem = e.mem();
    let handles = [g.swap_chain, g.device, 0, g.context];
    for (i, &p) in out.iter().enumerate() {
        if p == 0 {
            continue;
        }
        if i == 2 {
            mem.write(p, FEATURE_LEVEL_11_0);
        } else {
            mem.write(p, handles[i]);
        }
    }
    g.note(&format!(
        "D3D11CreateDeviceAndSwapChain flags {flags:#x} desc {:x?} -> device {:#x} context {:#x} swap chain {:#x} ({})",
        d.0,
        g.device,
        g.context,
        g.swap_chain,
        g.backend.name()
    ));
    e.stats.milestones.push(format!(
        "D3D11 device and swap chain ({}, {} x {})",
        g.backend.name(),
        d.width(),
        d.height()
    ));
    *slot = Some(g);
    Ret::eax(S_OK)
}

/// The layer's part of the run's report: the back end, the frames, the
/// calls per method, the methods not handled, the live objects.
#[must_use]
pub fn report(exec: &Exec) -> String {
    use std::fmt::Write as _;
    let slot = exec.shared().gfx.lock();
    let Some(g) = slot.as_ref() else {
        return "gfx: no device\n".into();
    };
    let mut out = String::new();
    let calls: u64 = g.counts.values().sum();
    let _ = writeln!(
        out,
        "gfx: {} back end, {} presents, {calls} calls, {} methods",
        g.backend_name(),
        g.presents,
        g.counts.len()
    );
    for (i, (d, c, k)) in g.frame_sizes.iter().enumerate() {
        if d + c + k > 0 {
            let _ = writeln!(
                out,
                "  frame {i}: {d} draws, {c} clears, {k} texture copies"
            );
        }
    }
    let _ = writeln!(out, "gfx calls:");
    for (m, n) in &g.counts {
        let _ = writeln!(out, "  {n:>8}  {m}");
    }
    let _ = writeln!(out, "gfx unhandled: {:?}", g.unhandled);
    let _ = writeln!(out, "gfx live objects: {:?}", g.census());
    out
}

/// Writes the layer's recording into `dir`: `capture.ffxgfx` (the
/// stream after `ffx_gfx::CAPTURE_MAGIC`, as the hook writes it) and
/// `calls.txt` (the text log).
///
/// # Errors
/// When a file can't be written.
pub fn write(exec: &Exec, dir: &std::path::Path) -> std::io::Result<()> {
    write_shared(exec.shared(), dir)
}

/// [`write`] from any thread (the harness's `gfxcapture`).
///
/// # Errors
/// When a file can't be written.
pub fn write_shared(shared: &crate::exec::Shared, dir: &std::path::Path) -> std::io::Result<()> {
    let slot = shared.gfx.lock();
    let Some(g) = slot.as_ref() else {
        return Ok(());
    };
    std::fs::create_dir_all(dir)?;
    let mut file = ffx_gfx::CAPTURE_MAGIC.to_vec();
    file.extend(g.stream());
    std::fs::write(dir.join("capture.ffxgfx"), file)?;
    let mut text = g.log().join("\n");
    text.push('\n');
    std::fs::write(dir.join("calls.txt"), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_subresources_follow_d3d11_order_and_block_sizes() {
        // 256 x 128 BC3, full chain, 2 slices
        let d = [256, 128, 0, 2, 77, 1, 0, 0, 8, 0, 0];
        let s = texture_subs(2, &d);
        assert_eq!(s.len(), 18);
        assert_eq!(s[0], (64 * 16, 32, 1));
        assert_eq!(s[8], (16, 1, 1)); // 1 x 1: one block
        assert_eq!(s[9], s[0]);
        // R8G8B8A8 64 x 64, one mip
        assert_eq!(
            texture_subs(2, &[64, 64, 1, 1, 28, 1, 0, 0, 8, 0, 0]),
            vec![(256, 64, 1)]
        );
    }

    #[test]
    fn the_back_buffer_is_the_swap_chains_description() {
        let d = SwapChainDesc([
            1280, 720, 60, 1, 0x1c, 0, 0, 1, 0, 0x20, 1, 0x10010, 1, 0, 2,
        ]);
        assert_eq!(
            d.back_buffer(),
            [1280, 720, 1, 1, 0x1c, 1, 0, 0, 0x20, 0, 0]
        );
        assert_eq!(d.window(), 0x10010);
    }
}
