//! The Rust map path (M4 round 2, `docs/draw.md`, "The map path"): the map
//! world's passes drawn through `ffx-draw` on the game's device, in place of
//! `PhyreEngine`'s.
//!
//! On with `FFX_RUST_MAP=1` (and the graphics layer, `FFX_GFX=1`; the
//! harness option `rust_map` and `tools/run-game.sh --rust-map` set both).
//!
//! Two threads. `PhyreScene_render` (`0x00668930`) runs on the main thread
//! and renders the map world (scene `+0xac`) through
//! `PApplication_renderWorld` (`0x0062be70`) for each pass ([`CALL_SITES`]:
//! its own copy of `PhyreScene_Draw3DWorld`'s (`0x0064fbb0`) sequence, and
//! `Draw3DWorld`'s). `renderWorld` makes no D3D11 calls: it culls and sorts
//! and fills render commands (72-byte entries, allocated from the
//! renderers' rings by `Phyre_PRendering_PRenderRingBufferSegmented_allocate`
//! `0x00613e10`). The render thread plays them a frame later
//! (`Phyre_PRendering_PRenderThread_processRenderCommands` `0x00615550`):
//! commands of type 1 and 2 are mesh instances, drawn by `0x006144f0` and
//! `0x00614610` (called at `0x006157cf` and `0x00615898`), in an order of
//! their own (a callback command queued between two `renderWorld` calls is
//! played after the whole scene).
//!
//! So the path ties the two together by the commands:
//!
//! 1. Main thread, update N: the map's `renderWorld` calls are pointed at
//!    [`render_map`] (call sites, not the function; any other world passes
//!    straight on: one site is shared with the characters world). It makes
//!    a tag for the pass (and, with a builder, the builder's job for it)
//!    and runs the original; every 72-byte command allocated meanwhile gets
//!    the tag ([`alloc`], an entry hook with a trampoline).
//! 2. Render thread, frame N: the two mesh-instance handler calls are
//!    pointed at [`mesh_command`]. While a tagged command is handled, its
//!    draws are taken from the proxy context's vtable: recorded (the
//!    context's bound state, `D3d11::import_bound`, the game's objects
//!    imported, and the constant buffer windows from the layer's copies),
//!    or, when the pass has a builder job, dropped. The state setters reach
//!    the device as usual, so `PhyreEngine`'s cache of what is bound stays
//!    true.
//! 3. The pass ends at the first draw that is not a tagged command's, at a
//!    command of another pass, at a clear, copy, dispatch, read-back or
//!    indirect draw, or at `Present`. There its draw list is issued through
//!    `ffx-draw`'s D3D11 backend between `begin` and `end` (which save and
//!    put back the context's state): the builder job's list, or the
//!    recorded one. A map or update of a resource a recorded draw reads
//!    issues the draws recorded so far first.
//!
//! The backend draws through the proxy context, so the layer sees the
//! issued draws as it saw `PhyreEngine`'s: captures (`gfxcapture`) record
//! them, and `drawdiff` compares them with an original capture.
//!
//! The `rustmap` harness command: `rustmap` (statistics), `rustmap on`,
//! `rustmap off` (from the next update).

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ffx_draw::backend::Buffer;
use ffx_draw::cache::Cache;
use ffx_draw::d3d11::{BoundCb, D3d11, Import};
use ffx_draw::desc::Stage;
use ffx_draw::pass::{Constants, Pass, PassDraw, issue};
use ffx_draw::state::{CbBinding, DrawCall};
use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VirtualAlloc, VirtualProtect,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

use super::{BIND_CONSTANT, Draw, proxy, state, vt};
use crate::log::log;

type Ptr = *mut c_void;

/// `Phyre_PFramework_PApplication_renderWorld`: thiscall on the
/// application, (world, camera, dir), `ret 0xc`.
const RENDER_WORLD: u32 = 0x0062_be70;
/// Every `call renderWorld` in `PhyreScene_Draw3DWorld` (`0x0064fbb0`)
/// and `PhyreScene_render` (`0x00668930`), `tools/refs.py 0x62be70`. The
/// field's map passes are `Draw3DWorld`'s second branch (scene `+0x10d8c`
/// is 1; opaque at `0x00650152`, transparent at `0x006502df`, from
/// `PhyreScene_render`'s call at `0x00669364`); the rest render other
/// worlds, or the map in passes that draw nothing on the field.
/// [`render_map`] passes every world but the map's straight on.
const CALL_SITES: [u32; 32] = [
    0x0064_fd18,
    0x0064_fd49,
    0x0064_fdaf,
    0x0064_fdf3,
    0x0064_fe35,
    0x0064_fe81,
    0x0064_fec5,
    0x0064_ff46,
    0x0064_ff74,
    0x0064_ffd4,
    0x0065_0018,
    0x0065_005a,
    0x0065_00a0,
    0x0065_00e4,
    0x0065_0152,
    0x0065_0183,
    0x0065_01df,
    0x0065_0223,
    0x0065_0299,
    0x0065_02df,
    0x0065_0323,
    0x0066_9419,
    0x0066_9643,
    0x0066_9667,
    0x0066_97bb,
    0x0066_97f8,
    0x0066_981f,
    0x0066_9b2f,
    0x0066_9b5d,
    0x0066_9b8b,
    0x0066_9d48,
    0x0066_9d76,
];
/// `Phyre_PRendering_PRenderRingBufferSegmented_allocate`: thiscall(ring,
/// size) -> pointer or 0, `ret 4`; a bump allocator per ring. Its first six
/// bytes (`push ebp; mov ebp, esp; mov eax, [ecx+8]`) move to the
/// trampoline.
const RING_ALLOCATE: u32 = 0x0061_3e10;
const RING_ALLOCATE_PROLOGUE: [u8; 6] = [0x55, 0x8b, 0xec, 0x8b, 0x41, 0x08];
/// A render command's size.
const COMMAND: u32 = 0x48;
/// `processRenderCommands`' calls of the mesh-instance handlers, cdecl
/// (render interface `0x00c94f00`, command + 8): type 1 (`0x006144f0`),
/// type 2 (`0x00614610`).
const HANDLER_SITES: [(u32, u32); 2] = [(0x0061_57cf, 0x0061_44f0), (0x0061_5898, 0x0061_4610)];
/// `g_PhyreScene`; the map world is at `+0xac`.
const PHYRE_SCENE: u32 = 0x00cc_b9d8;
const IMAGE_BASE: u32 = 0x0040_0000;

/// The mode is on (per update).
static ON: AtomicBool = AtomicBool::new(false);
/// Rebased: `renderWorld`, `g_PhyreScene`, the allocator's trampoline and
/// the two handlers.
static RENDER_WORLD_AT: AtomicUsize = AtomicUsize::new(0);
static SCENE_AT: AtomicUsize = AtomicUsize::new(0);
static ALLOCATE_AT: AtomicUsize = AtomicUsize::new(0);
static HANDLERS_AT: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];
/// While the backend issues: the recorder's entries pass every call on.
static ISSUING: AtomicBool = AtomicBool::new(false);
/// While a tagged command is handled (render thread).
static IN_MAP: AtomicBool = AtomicBool::new(false);
/// While the map's `renderWorld` runs (main thread): allocations are
/// tagged.
static TAGGING: AtomicBool = AtomicBool::new(false);
/// Main-thread pass count, for dropping tags never played.
static GENERATION: AtomicU32 = AtomicU32::new(0);

/// What a map pass is drawn for: the builder's inputs, on the main thread.
#[derive(Debug, Clone, Copy)]
pub struct PassInfo {
    pub pass: Pass,
    /// `renderWorld`'s direction (1 opaque, -1 transparent).
    pub dir: i32,
    /// `g_PhyreApplication`, the world (scene `+0xac`) and the camera
    /// (`*(scene + 0xdc) + 4`) `renderWorld` was called with.
    pub app: usize,
    pub world: usize,
    pub camera: usize,
}

/// The render-thread half of a built pass: given the backend (the game's
/// bound targets are `D3d11::import_bound`'s) and the cache, the pass's
/// draws.
pub type Job = Box<dyn FnOnce(&mut D3d11, &mut Cache) -> Vec<PassDraw> + Send>;

/// A draw list source for the map passes. Called on the main thread at
/// `renderWorld`'s place (update N) with the pass; returns the job that
/// makes the pass's draws on the render thread where `PhyreEngine`'s map
/// commands for that update play (frame N), or `None` to let the recorder
/// take `PhyreEngine`'s own draws.
pub type Builder = Box<dyn FnMut(&PassInfo) -> Option<Job> + Send>;

/// One map pass of one update.
struct Tag {
    pass: Pass,
    generation: u32,
    job: Mutex<Option<Job>>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Stats {
    passes: u64,
    built: u64,
    tagged: u64,
    commands: u64,
    recorded: u64,
    dropped: u64,
    issued: u64,
    flushes: u64,
    /// Clears, copies, indirect draws and the like that ended a pass.
    ended_by_call: u64,
    refused: u64,
    imports: u64,
}

struct MapPath {
    backend: D3d11,
    cache: Cache,
    pending: Vec<PassDraw>,
    /// Raw resources the pending draws read (vertex, index, shader
    /// resource views' resources).
    reads: HashSet<usize>,
    /// The pass whose commands are being played.
    current: Option<Arc<Tag>>,
    stats: Stats,
}

// SAFETY: the backend's COM objects are used only on the render thread
// (the handlers, the context's entries and Present run there); the mutex
// only orders the harness thread's reads of the statistics.
unsafe impl Send for MapPath {}

static PATH: Mutex<Option<MapPath>> = Mutex::new(None);
static BUILDER: Mutex<Option<Builder>> = Mutex::new(None);
/// The pass `renderWorld` is filling (main thread), and the commands
/// tagged so far, by address.
static FILLING: Mutex<Option<Arc<Tag>>> = Mutex::new(None);
static TAGS: Mutex<Option<HashMap<usize, Arc<Tag>>>> = Mutex::new(None);
static TAGGED: AtomicU32 = AtomicU32::new(0);

/// Sets (or clears) the builder that supplies the map passes' draws.
pub fn set_builder(b: Option<Builder>) {
    if let Ok(mut g) = BUILDER.lock() {
        *g = b;
    }
}

/// Writes `call target` (or `jmp`, `op` 0xe9) over five bytes of code.
///
/// # Safety
/// `at` must be five bytes of the image's code no thread runs now.
unsafe fn write_rel(at: usize, op: u8, target: usize) -> bool {
    let p = at as *mut u8;
    let mut old = 0;
    // SAFETY: forwarded from the caller.
    unsafe {
        if VirtualProtect(p.cast(), 5, PAGE_EXECUTE_READWRITE, &raw mut old) == 0 {
            return false;
        }
        *p = op;
        std::ptr::write_unaligned(p.add(1).cast::<u32>(), target.wrapping_sub(at + 5) as u32);
        VirtualProtect(p.cast(), 5, old, &raw mut old);
        FlushInstructionCache(GetCurrentProcess(), p.cast(), 5);
    }
    true
}

/// The target of the `call rel32` at `at`, if it is one.
///
/// # Safety
/// `at` must be readable code of the image.
unsafe fn call_target(at: usize) -> Option<usize> {
    let p = at as *const u8;
    // SAFETY: forwarded from the caller.
    unsafe {
        (*p == 0xe8).then(|| {
            (at + 5).wrapping_add(std::ptr::read_unaligned(p.add(1).cast::<u32>()) as usize)
        })
    }
}

/// Points the map world's `renderWorld` calls at [`render_map`], the ring
/// allocator at [`alloc`] and the mesh-instance handler calls at
/// [`mesh_command`], when `FFX_RUST_MAP=1` and the graphics layer is on.
/// All or nothing.
///
/// # Safety
/// `module` must be the main module, before other threads run game code.
pub unsafe fn install(module: usize) {
    if !std::env::var("FFX_RUST_MAP").is_ok_and(|v| v == "1") {
        return;
    }
    if !super::ENABLED.load(Ordering::Acquire) {
        log("  gfx: rust map: needs the graphics layer (FFX_GFX=1); off");
        return;
    }
    let rebase = |a: u32| module + (a - IMAGE_BASE) as usize;
    let render_world = rebase(RENDER_WORLD);
    let allocate = rebase(RING_ALLOCATE);
    // SAFETY: reads of the image's code, checked before anything is written.
    let fits = unsafe {
        CALL_SITES
            .iter()
            .all(|&s| call_target(rebase(s)) == Some(render_world))
            && HANDLER_SITES
                .iter()
                .all(|&(s, h)| call_target(rebase(s)) == Some(rebase(h)))
            && std::slice::from_raw_parts(allocate as *const u8, 6) == RING_ALLOCATE_PROLOGUE
    };
    if !fits {
        log("  gfx: rust map: the code is not as expected; off");
        return;
    }
    // The allocator's trampoline: its first six bytes, then a jump back.
    // SAFETY: a fresh allocation, written before use.
    let tramp = unsafe {
        let t = VirtualAlloc(
            std::ptr::null(),
            16,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        ) as usize;
        if t == 0 {
            return;
        }
        std::ptr::copy_nonoverlapping(RING_ALLOCATE_PROLOGUE.as_ptr(), t as *mut u8, 6);
        if !write_rel(t + 6, 0xe9, allocate + 6) {
            return;
        }
        t
    };
    RENDER_WORLD_AT.store(render_world, Ordering::Release);
    SCENE_AT.store(rebase(PHYRE_SCENE), Ordering::Release);
    ALLOCATE_AT.store(tramp, Ordering::Release);
    for (at, &(_, h)) in HANDLERS_AT.iter().zip(&HANDLER_SITES) {
        at.store(rebase(h), Ordering::Release);
    }
    if let Ok(mut t) = TAGS.lock() {
        *t = Some(HashMap::new());
    }
    let handlers = [
        mesh_command_1 as *const () as usize,
        mesh_command_2 as *const () as usize,
    ];
    // SAFETY: the sites were checked above; no other thread runs yet.
    let ok = unsafe {
        CALL_SITES
            .iter()
            .all(|&s| write_rel(rebase(s), 0xe8, render_map as *const () as usize))
            && HANDLER_SITES
                .iter()
                .zip(handlers)
                .all(|(&(s, _), f)| write_rel(rebase(s), 0xe8, f))
            && write_rel(allocate, 0xe9, alloc as *const () as usize)
    };
    ON.store(ok, Ordering::Release);
    log(&format!(
        "  gfx: rust map: {} ({} renderWorld calls, the ring allocator, 2 handler calls)",
        if ok { "on" } else { "patching failed" },
        CALL_SITES.len()
    ));
}

type RenderWorld = unsafe extern "thiscall" fn(usize, usize, usize, i32) -> u32;
type Allocate = unsafe extern "thiscall" fn(usize, u32) -> usize;
type Handler = unsafe extern "C" fn(usize, usize) -> u32;

/// Stands in for `renderWorld` at the map world's call sites (main
/// thread): the pass's tag, the builder's job, then the original with its
/// commands tagged. Never panics (it runs inside the game's frame).
unsafe extern "thiscall" fn render_map(app: usize, world: usize, camera: usize, dir: i32) -> u32 {
    // SAFETY: the original function's address, set before the patch.
    let original: RenderWorld =
        unsafe { std::mem::transmute(RENDER_WORLD_AT.load(Ordering::Acquire)) };
    // SAFETY: a pointer-sized global of the image.
    let scene = unsafe { *(SCENE_AT.load(Ordering::Acquire) as *const usize) };
    let map = scene != 0 && world == scene + 0xac;
    if !map || !ON.load(Ordering::Acquire) {
        // SAFETY: the original, same arguments.
        return unsafe { original(app, world, camera, dir) };
    }
    let info = PassInfo {
        pass: Pass::from_dir(dir),
        dir,
        app,
        world,
        camera,
    };
    let job = BUILDER
        .lock()
        .ok()
        .and_then(|mut b| b.as_mut().and_then(|f| f(&info)));
    let generation = GENERATION.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
    if generation.is_multiple_of(64)
        && let Ok(mut t) = TAGS.lock()
        && let Some(t) = t.as_mut()
    {
        // Tags of commands never played (culled on the render thread).
        t.retain(|_, tag| generation.wrapping_sub(tag.generation) < 64);
    }
    if let Ok(mut f) = FILLING.lock() {
        *f = Some(Arc::new(Tag {
            pass: info.pass,
            generation,
            job: Mutex::new(job),
        }));
    }
    TAGGING.store(true, Ordering::Release);
    // SAFETY: as above.
    let r = unsafe { original(app, world, camera, dir) };
    TAGGING.store(false, Ordering::Release);
    if let Ok(mut f) = FILLING.lock() {
        *f = None;
    }
    r
}

/// The ring allocator: commands allocated while the map's `renderWorld`
/// runs get its tag. Any thread.
unsafe extern "thiscall" fn alloc(ring: usize, size: u32) -> usize {
    // SAFETY: the trampoline of the original, same arguments.
    let p = unsafe {
        let f: Allocate = std::mem::transmute(ALLOCATE_AT.load(Ordering::Acquire));
        f(ring, size)
    };
    if p != 0 && size == COMMAND && TAGGING.load(Ordering::Acquire) {
        let tag = FILLING.lock().ok().and_then(|f| f.clone());
        if let Some(tag) = tag
            && let Ok(mut t) = TAGS.lock()
            && let Some(t) = t.as_mut()
        {
            t.insert(p, tag);
            TAGGED.fetch_add(1, Ordering::Relaxed);
        }
    }
    p
}

unsafe extern "C" fn mesh_command_1(iface: usize, command: usize) -> u32 {
    mesh_command(0, iface, command)
}

unsafe extern "C" fn mesh_command_2(iface: usize, command: usize) -> u32 {
    mesh_command(1, iface, command)
}

/// A mesh-instance command on the render thread: a tagged one's draws go
/// to its pass.
fn mesh_command(which: usize, iface: usize, command: usize) -> u32 {
    let handler = HANDLERS_AT
        .get(which)
        .map_or(0, |h| h.load(Ordering::Acquire));
    // SAFETY: the original handler, same arguments.
    let call = || unsafe {
        let f: Handler = std::mem::transmute(handler);
        f(iface, command)
    };
    let tag = TAGS
        .lock()
        .ok()
        .and_then(|mut t| t.as_mut().and_then(|t| t.remove(&command.wrapping_sub(8))));
    let Some(tag) = tag else {
        return call();
    };
    {
        let Ok(mut g) = PATH.lock() else {
            return call();
        };
        if !ready(&mut g) {
            return call();
        }
        let Some(p) = g.as_mut() else {
            return call();
        };
        if p.current.as_ref().is_some_and(|c| !Arc::ptr_eq(c, &tag)) {
            end_pass(p);
        }
        if p.current.is_none() {
            p.current = Some(tag);
            take_slots();
        }
        p.stats.commands += 1;
    }
    IN_MAP.store(true, Ordering::Release);
    let r = call();
    IN_MAP.store(false, Ordering::Release);
    r
}

/// Makes the backend on first use (render thread); false when there is
/// no device.
fn ready(g: &mut Option<MapPath>) -> bool {
    if g.is_some() {
        return true;
    }
    static TOLD: AtomicBool = AtomicBool::new(false);
    let fail = |why: &str| {
        if !TOLD.swap(true, Ordering::AcqRel) {
            log(&format!(
                "  gfx: rust map: no backend ({why}); PhyreEngine draws the map"
            ));
        }
        false
    };
    let device = super::DEVICE.load(Ordering::Acquire) as Ptr;
    let real = super::CONTEXT.load(Ordering::Acquire) as Ptr;
    if device.is_null() || real.is_null() {
        return fail("no device yet");
    }
    let Some(context) = proxy::proxy_of(real) else {
        return fail("no proxy context");
    };
    // SAFETY: the game's device and its immediate context's proxy; the
    // backend is used only on the render thread.
    let Some(backend) = (unsafe { D3d11::borrow(device, context) }) else {
        return fail("borrow failed");
    };
    *g = Some(MapPath {
        backend,
        cache: Cache::default(),
        pending: Vec::new(),
        reads: HashSet::new(),
        current: None,
        stats: Stats::default(),
    });
    true
}

/// Issues the recorded draws.
fn flush(p: &mut MapPath) {
    if p.pending.is_empty() {
        return;
    }
    ISSUING.store(true, Ordering::Release);
    let done = issue(&mut p.backend, &mut p.cache, &p.pending);
    ISSUING.store(false, Ordering::Release);
    p.stats.issued += done.draws as u64;
    p.stats.refused += done.refused as u64;
    p.pending.clear();
    p.reads.clear();
}

/// Ends the current pass: its builder job's draws, or what is left of the
/// recorded ones, issued; the context's entries given back.
fn end_pass(p: &mut MapPath) {
    let Some(tag) = p.current.take() else {
        return;
    };
    give_slots();
    let job = tag.job.lock().ok().and_then(|mut j| j.take());
    if let Some(job) = job {
        let draws = job(&mut p.backend, &mut p.cache);
        ISSUING.store(true, Ordering::Release);
        let done = issue(&mut p.backend, &mut p.cache, &draws);
        ISSUING.store(false, Ordering::Release);
        p.stats.built += 1;
        p.stats.issued += done.draws as u64;
        p.stats.refused += done.refused as u64;
    } else {
        flush(p);
    }
    p.stats.passes += 1;
    p.stats.tagged = u64::from(TAGGED.load(Ordering::Relaxed));
    p.stats.imports += p.backend.imports() as u64;
    p.backend.release_imports();
    if p.stats.passes >= 64 && p.stats.passes.is_power_of_two() {
        log(&format!(
            "  gfx: rust map: {} ({:?} pass ended)",
            stats_line(&p.stats),
            tag.pass
        ));
    }
}

fn stats_line(s: &Stats) -> String {
    format!(
        "passes {} built {} tagged {} commands {} recorded {} dropped {} issued {} flushes {} ended-by-call {} refused {} imports {}",
        s.passes,
        s.built,
        s.tagged,
        s.commands,
        s.recorded,
        s.dropped,
        s.issued,
        s.flushes,
        s.ended_by_call,
        s.refused,
        s.imports
    )
}

/// At `Present` (render thread): a pass still open ends.
pub fn on_present() {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    static FRAMES: AtomicU32 = AtomicU32::new(0);
    let frame = FRAMES.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut g) = PATH.lock() {
        if let Some(p) = g.as_mut() {
            end_pass(p);
        }
        if frame >= 256 && frame.is_power_of_two() {
            let s = g.as_ref().map(|p| p.stats).unwrap_or_default();
            let open = TAGS
                .lock()
                .ok()
                .and_then(|t| t.as_ref().map(HashMap::len))
                .unwrap_or(0);
            log(&format!(
                "  gfx: rust map: frame {frame}: {} map passes queued, {} commands tagged, {open} not played yet; {}",
                GENERATION.load(Ordering::Relaxed),
                TAGGED.load(Ordering::Relaxed),
                stats_line(&s)
            ));
        }
    }
}

/// A call inside an open pass that may see or change what its draws use:
/// with `Some(resource)`, the recorded draws are issued first when they
/// read it; with `None` the pass ends here.
fn hazard(resource: Option<usize>) {
    if ISSUING.load(Ordering::Acquire) {
        return;
    }
    let Ok(mut g) = PATH.lock() else {
        return;
    };
    let Some(p) = g.as_mut() else {
        return;
    };
    match resource {
        None => {
            if p.current.is_some() {
                p.stats.ended_by_call += 1;
                end_pass(p);
            }
        }
        Some(r) => {
            if !p.pending.is_empty() && p.reads.contains(&r) {
                p.stats.flushes += 1;
                flush(p);
            }
        }
    }
}

/// The bytes of a constant buffer window from the layer's copy (`None`:
/// the layer has none for that buffer).
fn window(cb: BoundCb) -> Option<Vec<u8>> {
    let s = state();
    let b = s.buffers.get(&cb.buffer)?;
    if b.desc[2] & BIND_CONSTANT == 0 {
        return None;
    }
    let sh = b.shadow.as_ref()?;
    Some(if cb.count == 0 {
        sh.clone()
    } else {
        let from = (cb.first as usize * 16).min(sh.len());
        let to = (from + cb.count as usize * 16).min(sh.len());
        sh[from..to].to_vec()
    })
}

fn call_of(d: Draw) -> DrawCall {
    match d {
        Draw::Indexed { count, start, base } => DrawCall::Indexed { count, start, base },
        Draw::Plain { count, start } => DrawCall::Plain { count, start },
        Draw::IndexedInstanced {
            count,
            instances,
            start,
            base,
            first,
        } => DrawCall::IndexedInstanced {
            count,
            instances,
            start,
            base,
            first_instance: first,
        },
        Draw::Instanced {
            count,
            instances,
            start,
            first,
        } => DrawCall::Instanced {
            count,
            instances,
            start,
            first_instance: first,
        },
    }
}

/// The recorded form of the draw the game is making now: the context's
/// bound state with the game's objects imported, and each constant buffer
/// window as bytes (the layer's copy; the game's buffer itself when there
/// is none).
fn recorded(b: &mut D3d11, d: Draw) -> (PassDraw, Vec<usize>) {
    let now = b.import_bound();
    let mut state = now.state;
    let mut constants = Vec::new();
    for (stage, cbs) in [Stage::Vertex, Stage::Pixel].into_iter().zip(now.cbs) {
        for (slot, cb) in cbs.into_iter().enumerate() {
            let Some(cb) = cb else {
                continue;
            };
            if let Some(data) = window(cb) {
                constants.push(Constants {
                    stage,
                    slot: slot as u8,
                    data,
                });
                continue;
            }
            // SAFETY: bound on the context, so alive.
            let h = unsafe { b.import(cb.buffer as Ptr, Import::Buffer) };
            let bindings = if stage == Stage::Vertex {
                &mut state.vs
            } else {
                &mut state.ps
            };
            if let (Some(h), Some(at)) = (h, bindings.cbs.get_mut(slot)) {
                *at = Some(CbBinding {
                    buffer: Buffer(h),
                    first: cb.first,
                    count: cb.count,
                });
            }
        }
    }
    (
        PassDraw {
            state,
            call: call_of(d),
            constants,
            writes: Vec::new(),
        },
        now.reads,
    )
}

/// A draw while a pass is open (render thread). A tagged command's draw is
/// recorded (or dropped, when a builder job draws the pass); any other ends
/// the pass first. True when the game's draw is not to be made.
fn on_draw(d: Draw) -> bool {
    if ISSUING.load(Ordering::Acquire) {
        return false;
    }
    let Ok(mut g) = PATH.lock() else {
        return false;
    };
    let Some(p) = g.as_mut() else {
        return false;
    };
    if !IN_MAP.load(Ordering::Acquire) {
        end_pass(p);
        return false;
    }
    let built = p
        .current
        .as_ref()
        .is_some_and(|t| t.job.lock().is_ok_and(|j| j.is_some()));
    if built {
        p.stats.dropped += 1;
        return true;
    }
    let (draw, reads) = recorded(&mut p.backend, d);
    p.pending.push(draw);
    p.reads.extend(reads);
    p.stats.recorded += 1;
    true
}
// ---- The recorder's proxy entries ----

/// Proxy vtable slots the recorder takes while `PhyreEngine`'s map pass
/// runs, and what they held.
const SLOTS: [usize; 21] = [
    vt::context::DRAW_INDEXED,
    vt::context::DRAW,
    vt::context::DRAW_INDEXED_INSTANCED,
    vt::context::DRAW_INSTANCED,
    vt::context::MAP,
    vt::context::UPDATE_SUBRESOURCE,
    vt::context::UPDATE_SUBRESOURCE_1,
    vt::context::COPY_RESOURCE,
    vt::context::COPY_SUBRESOURCE_REGION,
    vt::context::COPY_SUBRESOURCE_REGION_1,
    vt::context::CLEAR_RENDER_TARGET_VIEW,
    vt::context::CLEAR_DEPTH_STENCIL_VIEW,
    vt::context::DISPATCH,
    vt::context::GENERATE_MIPS,
    vt::context::RESOLVE_SUBRESOURCE,
    DRAW_AUTO,
    DRAW_INDEXED_INSTANCED_INDIRECT,
    DRAW_INSTANCED_INDIRECT,
    DISPATCH_INDIRECT,
    CLEAR_UAV_UINT,
    CLEAR_UAV_FLOAT,
];
// ID3D11DeviceContext slots the layer does not hook.
const DRAW_AUTO: usize = 38;
const DRAW_INDEXED_INSTANCED_INDIRECT: usize = 39;
const DRAW_INSTANCED_INDIRECT: usize = 40;
const DISPATCH_INDIRECT: usize = 42;
const CLEAR_UAV_UINT: usize = 51;
const CLEAR_UAV_FLOAT: usize = 52;

static PREVIOUS: [AtomicUsize; SLOTS.len()] = [const { AtomicUsize::new(0) }; SLOTS.len()];

fn ours() -> [usize; SLOTS.len()] {
    [
        draw_indexed as *const () as usize,
        draw as *const () as usize,
        draw_indexed_instanced as *const () as usize,
        draw_instanced as *const () as usize,
        map as *const () as usize,
        update_subresource as *const () as usize,
        update_subresource1 as *const () as usize,
        copy_resource as *const () as usize,
        copy_subresource_region as *const () as usize,
        copy_subresource_region1 as *const () as usize,
        clear_rtv as *const () as usize,
        clear_dsv as *const () as usize,
        dispatch as *const () as usize,
        generate_mips as *const () as usize,
        resolve_subresource as *const () as usize,
        draw_auto as *const () as usize,
        draw_indexed_instanced_indirect as *const () as usize,
        draw_instanced_indirect as *const () as usize,
        dispatch_indirect as *const () as usize,
        clear_uav_uint as *const () as usize,
        clear_uav_float as *const () as usize,
    ]
}

fn take_slots() {
    let table = proxy::vtable();
    for (i, (&slot, f)) in SLOTS.iter().zip(ours()).enumerate() {
        // SAFETY: our own proxy vtable (`proxy::SLOTS` entries); an aligned
        // word store is atomic on x86, and only the render thread calls
        // through it.
        unsafe {
            let at = table.add(slot);
            let now = std::ptr::read_volatile(at);
            if now != f {
                PREVIOUS[i].store(now, Ordering::Release);
                std::ptr::write_volatile(at, f);
            }
        }
    }
}

fn give_slots() {
    let table = proxy::vtable();
    for (i, (&slot, f)) in SLOTS.iter().zip(ours()).enumerate() {
        let prev = PREVIOUS[i].load(Ordering::Acquire);
        // SAFETY: as in `take_slots`.
        unsafe {
            let at = table.add(slot);
            if prev != 0 && std::ptr::read_volatile(at) == f {
                std::ptr::write_volatile(at, prev);
            }
        }
    }
}

fn previous(i: usize) -> usize {
    PREVIOUS.get(i).map_or(0, |p| p.load(Ordering::Acquire))
}

/// A clear, copy, dispatch, read-back or indirect draw: the open pass ends
/// before it.
fn unrecorded() {
    hazard(None);
}

unsafe extern "system" fn draw_indexed(this: Ptr, count: u32, start: u32, base: i32) {
    if on_draw(Draw::Indexed { count, start, base }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, i32);
    // SAFETY: the entry this slot held, same arguments.
    unsafe {
        let f: F = std::mem::transmute(previous(0));
        f(this, count, start, base);
    }
}

unsafe extern "system" fn draw(this: Ptr, count: u32, start: u32) {
    if on_draw(Draw::Plain { count, start }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(1));
        f(this, count, start);
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
    if on_draw(Draw::IndexedInstanced {
        count,
        instances,
        start,
        base,
        first,
    }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, i32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(2));
        f(this, count, instances, start, base, first);
    }
}

unsafe extern "system" fn draw_instanced(
    this: Ptr,
    count: u32,
    instances: u32,
    start: u32,
    first: u32,
) {
    if on_draw(Draw::Instanced {
        count,
        instances,
        start,
        first,
    }) {
        return;
    }
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(3));
        f(this, count, instances, start, first);
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
    // D3D11_MAP: 1 read, 2 write, 3 read-write, 4 write-discard, 5
    // write-no-overwrite (never touches what a draw issued uses).
    match kind {
        1 | 3 => hazard(None),
        2 | 4 => hazard(Some(resource as usize)),
        _ => {}
    }
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, *mut usize) -> i32;
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(4));
        f(this, resource, sub, kind, flags, out)
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
    hazard(Some(dst as usize));
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(5));
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
    hazard(Some(dst as usize));
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, *const u32, *const u8, u32, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(6));
        f(this, dst, sub, region, data, row, depth, flags);
    }
}

unsafe extern "system" fn copy_resource(this: Ptr, dst: Ptr, src: Ptr) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, Ptr);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(7));
        f(this, dst, src);
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
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, u32, Ptr, u32, *const u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(8));
        f(this, dst, dst_sub, x, y, z, src, src_sub, region);
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
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, u32, u32, u32, Ptr, u32, *const u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(9));
        f(this, dst, dst_sub, x, y, z, src, src_sub, region, flags);
    }
}

unsafe extern "system" fn clear_rtv(this: Ptr, view: Ptr, color: *const f32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(10));
        f(this, view, color);
    }
}

unsafe extern "system" fn clear_dsv(this: Ptr, view: Ptr, flags: u32, depth: f32, stencil: u8) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, f32, u8);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(11));
        f(this, view, flags, depth, stencil);
    }
}

unsafe extern "system" fn dispatch(this: Ptr, x: u32, y: u32, z: u32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, u32, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(12));
        f(this, x, y, z);
    }
}

unsafe extern "system" fn generate_mips(this: Ptr, srv: Ptr) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(13));
        f(this, srv);
    }
}

unsafe extern "system" fn resolve_subresource(
    this: Ptr,
    dst: Ptr,
    dst_sub: u32,
    src: Ptr,
    src_sub: u32,
    format: u32,
) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32, Ptr, u32, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(14));
        f(this, dst, dst_sub, src, src_sub, format);
    }
}

unsafe extern "system" fn draw_auto(this: Ptr) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(15));
        f(this);
    }
}

unsafe extern "system" fn draw_indexed_instanced_indirect(this: Ptr, args: Ptr, offset: u32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(16));
        f(this, args, offset);
    }
}

unsafe extern "system" fn draw_instanced_indirect(this: Ptr, args: Ptr, offset: u32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(17));
        f(this, args, offset);
    }
}

unsafe extern "system" fn dispatch_indirect(this: Ptr, args: Ptr, offset: u32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(18));
        f(this, args, offset);
    }
}

unsafe extern "system" fn clear_uav_uint(this: Ptr, uav: Ptr, values: *const u32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, *const u32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(19));
        f(this, uav, values);
    }
}

unsafe extern "system" fn clear_uav_float(this: Ptr, uav: Ptr, values: *const f32) {
    unrecorded();
    type F = unsafe extern "system" fn(Ptr, Ptr, *const f32);
    // SAFETY: as above.
    unsafe {
        let f: F = std::mem::transmute(previous(20));
        f(this, uav, values);
    }
}

/// `rustmap [on|off]`: the mode's statistics, or turns it on or off from
/// the next map pass.
pub fn request(args: &str) -> String {
    if RENDER_WORLD_AT.load(Ordering::Acquire) == 0 {
        return "err the rust map path is not installed (FFX_RUST_MAP=1 with FFX_GFX=1)".into();
    }
    match args.trim() {
        "on" => ON.store(true, Ordering::Release),
        "off" => ON.store(false, Ordering::Release),
        "" => {}
        other => return format!("err usage: rustmap [on|off] ({other}?)"),
    }
    let s = PATH
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|p| p.stats))
        .unwrap_or_default();
    format!(
        "ok {} {}",
        if ON.load(Ordering::Acquire) {
            "on"
        } else {
            "off"
        },
        stats_line(&s)
    )
}
