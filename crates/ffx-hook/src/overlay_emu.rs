//! `FFX_MAGIC_EMU=1`: the spell overlays run in Unicorn (`ffx-overlay`)
//! instead of natively, inside the game's own memory (identity mapping:
//! guest address = host address). The M5 gate for the 64-bit program,
//! which runs them the same way inside its arena
//! (docs/formats/magic.md, "Running the original overlays").
//!
//! - `FFX.exe`'s `LoadLibraryA`, `GetProcAddress` and `FreeLibrary` imports
//!   are wrapped. A `magic_*.dll` is read as data and laid out by
//!   `ffx-overlay` in memory reserved here; its `.text` pages stay
//!   reserved and inaccessible, so the game never runs them natively. The
//!   module handle is the image base; `GetProcAddress` answers the real
//!   export addresses.
//! - Any native jump into an overlay's `.text` (`DllMain` called here,
//!   `InitMagicPRX` from `MagicFile_Start`, the overlay table's entries,
//!   callbacks the overlay stored) faults; a vectored handler sends it to
//!   [`enter`], which runs the code in this thread's Unicorn from the
//!   caller's own stack (the driver runs 64 KiB below it).
//! - Overlay code leaving its `.text` (the `g_MagicImports` slots, any
//!   other game function) comes back through [`OverlayHost::call_out`],
//!   which calls the real address natively with the overlay's registers
//!   and arguments. `rand` and the libm imports go to the game's own
//!   `MSVCR110` the same way; the CRT start-up imports are `ffx-overlay`'s
//!   stand-ins.
//!
//! Cost is logged per overlay when it is unloaded (`ffx-hook.log`,
//! `overlay-emu:` lines): per function, calls, time in Unicorn and time in
//! the native call-outs it made.
//!
//! `FFX_MAGIC_RUST=1` (every DLL) or a comma list of DLL numbers
//! (`604,309`) answers the trapped calls of those DLLs with their Rust
//! ports (`ffx-magic`, docs/formats/magic.md "Porting the overlays")
//! where one binds at the address; anything without a port still runs in
//! Unicorn. The loader is the same (it implies `FFX_MAGIC_EMU`), so the
//! DLL's data stays where the original's would be. The unload report
//! marks each entry `rust` or `unicorn`; a DLL is fully ported when a
//! replay shows no `unicorn` entry for it.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ffx_overlay::{Config, Host, Overlay, Pe, Regs};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows_sys::Win32::System::Memory::{
    GetProcessHeap, HEAP_ZERO_MEMORY, HeapAlloc, HeapFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_NOACCESS, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::log::log;

/// `g_MagicImports` and its 744 slots (docs/formats/magic.md).
const MAGIC_IMPORTS: u32 = 0x00c6_4ce8;
const MAGIC_SLOTS: u32 = 744;
/// The stack the driver leaves to the overlay code above it.
const GAP: u32 = 0x1_0000;

static ON: AtomicBool = AtomicBool::new(false);
static LOAD_LIBRARY_A: AtomicUsize = AtomicUsize::new(0);
static GET_PROC_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static FREE_LIBRARY: AtomicUsize = AtomicUsize::new(0);

/// Whether `FFX_MAGIC_EMU=1` turned the emulation on.
#[must_use]
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Wraps the three imports and adds the trap handler when
/// `FFX_MAGIC_EMU=1`.
///
/// # Safety
/// `base` must be the main module; called before game code runs.
pub unsafe fn install(base: usize) {
    let emu = std::env::var("FFX_MAGIC_EMU").is_ok_and(|v| v == "1");
    let rust = std::env::var("FFX_MAGIC_RUST")
        .ok()
        .filter(|v| !v.is_empty());
    if !emu && rust.is_none() {
        return;
    }
    if let Some(v) = rust {
        let sel = if v == "1" {
            RustSel::All
        } else {
            RustSel::List(v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        };
        log(&format!("overlay-emu: Rust overlays for {sel:?}"));
        let _ = RUST.set(sel);
    }
    // SAFETY: reserves 64 KiB of address space nothing else will get.
    let service = unsafe { VirtualAlloc(std::ptr::null(), 0x1_0000, MEM_RESERVE, PAGE_NOACCESS) };
    if service.is_null() {
        log("overlay-emu: no room for the service range; overlays run natively");
        return;
    }
    ffx_overlay::configure(Config {
        base: 0,
        service: service as u32,
        import_table: Some((crate::rebase(MAGIC_IMPORTS) as u32, MAGIC_SLOTS)),
    });
    // SAFETY: the caller's main module, before game code runs.
    unsafe {
        let patch = |name: &str, f: usize, keep: &AtomicUsize| match crate::iat::patch(
            base,
            "KERNEL32.dll",
            name,
            f,
        ) {
            Some(orig) => keep.store(orig, Ordering::Release),
            None => log(&format!("overlay-emu: FFX.exe imports no {name}")),
        };
        patch(
            "LoadLibraryA",
            load_library_a as *const () as usize,
            &LOAD_LIBRARY_A,
        );
        patch(
            "GetProcAddress",
            get_proc_address as *const () as usize,
            &GET_PROC_ADDRESS,
        );
        patch(
            "FreeLibrary",
            free_library as *const () as usize,
            &FREE_LIBRARY,
        );
        AddVectoredExceptionHandler(1, Some(trap));
    }
    ON.store(true, Ordering::Release);
    log(&format!(
        "overlay-emu: on (service {:#010x}); magic_*.dll run in Unicorn",
        service as usize
    ));
}

/// Which DLLs `FFX_MAGIC_RUST` hands to the ports.
#[derive(Debug)]
enum RustSel {
    All,
    List(Vec<u16>),
}

static RUST: std::sync::OnceLock<RustSel> = std::sync::OnceLock::new();

/// The loaded overlays as the ports see them, by base.
type Images = Vec<(u32, std::sync::Arc<ffx_magic::Image>)>;
static IMAGES: Mutex<Images> = Mutex::new(Vec::new());
/// The port bound at an address (`None`: none), per loaded overlay.
static BINDS: Mutex<Option<HashMap<u32, Option<usize>>>> = Mutex::new(None);
/// Calls answered in Rust, per entry address.
static RUST_CALLS: Mutex<Option<HashMap<u32, u64>>> = Mutex::new(None);

/// The image and port for a trapped call at `at`, when `FFX_MAGIC_RUST`
/// selects its DLL and a port binds there.
fn rust_port(at: u32) -> Option<(std::sync::Arc<ffx_magic::Image>, usize)> {
    let sel = RUST.get()?;
    let img = IMAGES
        .lock()
        .ok()?
        .iter()
        .find(|(b, i)| at >= *b && i.in_text(at))
        .map(|(_, i)| std::sync::Arc::clone(i))?;
    if let RustSel::List(l) = sel
        && !l.contains(&img.number)
    {
        return None;
    }
    let mut binds = BINDS.lock().ok()?;
    let i = *binds
        .get_or_insert_with(HashMap::new)
        .entry(at)
        .or_insert_with(|| ffx_magic::bind(&img, at));
    i.map(|i| (img, i))
}

/// The game for the ports: its own memory (identity) and native calls.
struct HookGame;

impl ffx_bridge::Mem for HookGame {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        ffx_bridge::RawMem { image: 0 }.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        ffx_bridge::RawMem { image: 0 }.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        0
    }
}

/// An x87 80-bit value from an `f64` (exact).
fn f64_to_f80(x: f64) -> [u8; 10] {
    let b = x.to_bits();
    let sign = ((b >> 63) as u16) << 15;
    let exp = ((b >> 52) & 0x7ff) as u16;
    let frac = b & ((1 << 52) - 1);
    let (e, m): (u16, u64) = match exp {
        0 if frac == 0 => (0, 0),
        0 => {
            // subnormal: the leading 1 moves to bit 63
            let shift = frac.leading_zeros() - 11;
            (15361 - shift as u16, frac << (shift + 11))
        }
        0x7ff => (0x7fff, (1 << 63) | (frac << 11)),
        _ => (exp - 1023 + 16383, (1 << 63) | (frac << 11)),
    };
    let mut out = [0u8; 10];
    out[..8].copy_from_slice(&m.to_le_bytes());
    out[8..].copy_from_slice(&(sign | e).to_le_bytes());
    out
}

/// An x87 80-bit value as an `f64` (the game's 53-bit results are exact).
fn f80_to_f64(b: &[u8; 10]) -> f64 {
    let m = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = i32::from(se & 0x7fff);
    if e == 0x7fff {
        return if m << 1 == 0 {
            sign * f64::INFINITY
        } else {
            f64::NAN
        };
    }
    #[allow(clippy::cast_precision_loss)] // exact for 53-bit mantissas
    let v = (m as f64) * 2f64.powi(e - 16383 - 63);
    sign * v
}

/// Calls `target` natively ([`native`]) with `args` on a stack of our
/// own, `st` pushed (ST0 first) and `xmm0`.
fn call_regs(target: u32, ecx: u32, args: &[u32], st: &[f64], xmm0: Option<f64>) -> Regs {
    let mut stack = [0u32; 1 + ARGS as usize];
    for (d, a) in stack[1..].iter_mut().zip(args) {
        *d = *a;
    }
    let mut regs = Regs::at(stack.as_mut_ptr() as u32);
    regs.ecx = ecx;
    regs.st_len = st.len().min(2) as u8;
    for (d, x) in regs.st.iter_mut().zip(st) {
        *d = f64_to_f80(*x);
    }
    if let Some(x) = xmm0 {
        regs.xmm0[..8].copy_from_slice(&x.to_le_bytes());
    }
    native(target, &mut regs);
    // `stack` lives until here: native read and wrote it
    std::hint::black_box(&mut stack);
    regs
}

impl ffx_magic::Game for HookGame {
    fn call(&mut self, target: u32, ecx: u32, args: &[u32]) -> ffx_magic::Out {
        let r = call_regs(target, ecx, args, &[], None);
        ffx_magic::Out {
            eax: r.eax,
            edx: r.edx,
            st0: (r.st_len > 0).then(|| f80_to_f64(&r.st[0])),
        }
    }
    fn rand(&mut self) -> u32 {
        msvcr110(c"rand").map_or(0, |f| call_regs(f, 0, &[], &[], None).eax)
    }
    fn libm(&mut self, f: ffx_magic::Libm, x: f64) -> f64 {
        let name = match f {
            ffx_magic::Libm::Sin => c"_libm_sse2_sin_precise",
            ffx_magic::Libm::Cos => c"_libm_sse2_cos_precise",
            ffx_magic::Libm::Atan => c"_libm_sse2_atan_precise",
            ffx_magic::Libm::Acos => c"_libm_sse2_acos_precise",
        };
        msvcr110(name).map_or(0.0, |a| {
            let r = call_regs(a, 0, &[], &[], Some(x));
            let mut b = [0u8; 8];
            b.copy_from_slice(&r.xmm0[..8]);
            f64::from_le_bytes(b)
        })
    }
    fn atan2(&mut self, y: f64, x: f64) -> f64 {
        // ST0 = x, ST1 = y
        msvcr110(c"_CIatan2").map_or(0.0, |a| {
            let r = call_regs(a, 0, &[], &[x, y], None);
            f80_to_f64(&r.st[0])
        })
    }
}

/// Runs the port `i` of `img` for the trapped call in `fr`; never panics.
fn enter_rust(fr: &mut EnterFrame, img: &ffx_magic::Image, i: usize) {
    let Some(f) = ffx_magic::entry::<HookGame>(i) else {
        return;
    };
    let frame = ffx_magic::Frame {
        at: fr.target,
        esp: fr.s,
        eax: fr.eax,
        ecx: fr.ecx,
        edx: fr.edx,
    };
    let r = f(&mut HookGame, img, &frame);
    if let Ok(mut c) = RUST_CALLS.lock() {
        *c.get_or_insert_with(HashMap::new)
            .entry(fr.target)
            .or_default() += 1;
    }
    fr.eax = r.eax;
    fr.edx = r.edx;
    if let Some(t) = r.jump {
        // `jmp eax`: the thunk "returns" into the target with the
        // caller's stack as it was (its return address on top)
        fr.ret_slot = fr.s - 4;
        fr.ret = t;
        fr.has_st0 = 0;
        return;
    }
    fr.ret_slot = fr.s + r.pops;
    match r.st0 {
        Some(x) => {
            fr.has_st0 = 1;
            fr.st0 = f64_to_f80(x);
        }
        None => fr.has_st0 = 0,
    }
}

fn is_overlay_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase().replace('/', "\\");
    let file = p.rsplit('\\').next().unwrap_or(&p);
    p.contains("magicfiles\\") && file.starts_with("magic_") && file.ends_with(".dll")
}

/// `LoadLibraryA` for `FFX.exe`: overlays laid out here, the rest
/// forwarded.
unsafe extern "system" fn load_library_a(path: *const u8) -> usize {
    let orig = LOAD_LIBRARY_A.load(Ordering::Acquire);
    // SAFETY: the game passes a NUL-terminated path.
    let name = unsafe { CStr::from_ptr(path.cast()) }
        .to_string_lossy()
        .into_owned();
    if is_overlay_path(&name) {
        match load_overlay(&name) {
            Ok(base) => return base as usize,
            Err(e) => log(&format!("overlay-emu: {name}: {e}; loading it natively")),
        }
    }
    // SAFETY: the real import, same arguments.
    unsafe {
        let f: unsafe extern "system" fn(*const u8) -> usize = std::mem::transmute(orig);
        f(path)
    }
}

/// `GetProcAddress`: an emulated overlay's exports by name.
unsafe extern "system" fn get_proc_address(module: usize, name: *const u8) -> usize {
    if (name as usize) >> 16 != 0
        && let Some(ov) = ffx_overlay::by_base(module as u32)
    {
        // SAFETY: a NUL-terminated export name (not an ordinal).
        let n = unsafe { CStr::from_ptr(name.cast()) }.to_string_lossy();
        return ov.export(&n).unwrap_or(0) as usize;
    }
    let orig = GET_PROC_ADDRESS.load(Ordering::Acquire);
    // SAFETY: the real import, same arguments.
    unsafe {
        let f: unsafe extern "system" fn(usize, *const u8) -> usize = std::mem::transmute(orig);
        f(module, name)
    }
}

/// `FreeLibrary`: `DllMain(DLL_PROCESS_DETACH)` in Unicorn, then the
/// overlay is forgotten and its memory released.
unsafe extern "system" fn free_library(module: usize) -> i32 {
    if let Some(ov) = ffx_overlay::by_base(module as u32) {
        // SAFETY: the overlay's entry point, run through the trap.
        let ok = unsafe { dll_main(&ov, 0) };
        ov.unload();
        if let Ok(mut i) = IMAGES.lock() {
            i.retain(|(b, _)| *b != ov.base());
        }
        if let Ok(mut b) = BINDS.lock()
            && let Some(m) = b.as_mut()
        {
            m.retain(|a, _| *a < ov.base() || *a >= ov.base() + ov.span());
        }
        report(&ov);
        // SAFETY: the reservation `load_overlay` made.
        unsafe { VirtualFree(ov.base() as *mut c_void, 0, MEM_RELEASE) };
        log(&format!(
            "overlay-emu: unloaded {} (DllMain detach {ok})",
            ov.name()
        ));
        return 1;
    }
    let orig = FREE_LIBRARY.load(Ordering::Acquire);
    // SAFETY: the real import, same argument.
    unsafe {
        let f: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(orig);
        f(module)
    }
}

/// `DllMain(base, reason, 0)`, natively: the jump into `.text` traps.
unsafe fn dll_main(ov: &Overlay, reason: u32) -> u32 {
    // SAFETY: a stdcall entry point of the overlay (run in Unicorn).
    unsafe {
        let f: unsafe extern "system" fn(u32, u32, u32) -> u32 =
            std::mem::transmute(ov.entry_point() as usize);
        f(ov.base(), reason, 0)
    }
}

fn load_overlay(path: &str) -> Result<u32, String> {
    let file = std::fs::read(path)
        .or_else(|_| {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let dir = exe.parent().ok_or("no exe directory")?;
            std::fs::read(dir.join(path.replace('\\', "/"))).map_err(|e| e.to_string())
        })
        .map_err(|e| format!("read: {e}"))?;
    let pe = Pe::parse(&file).map_err(|e| e.to_string())?;
    let span = pe.span();
    let (t0, t1) = pe.text_range().map_err(|e| e.to_string())?;
    // SAFETY: a fresh reservation, at the preferred base when it is free.
    let base = unsafe {
        let mut b = VirtualAlloc(
            pe.preferred_base as *const c_void,
            span as usize,
            MEM_RESERVE,
            PAGE_NOACCESS,
        );
        if b.is_null() {
            b = VirtualAlloc(std::ptr::null(), span as usize, MEM_RESERVE, PAGE_NOACCESS);
        }
        b as u32
    };
    if base == 0 {
        return Err(format!("no room for {span:#x} bytes"));
    }
    // headers and data writable; the code range stays inaccessible
    // SAFETY: inside the reservation just made.
    let ok = unsafe {
        !VirtualAlloc(
            base as *const c_void,
            t0 as usize,
            MEM_COMMIT,
            PAGE_READWRITE,
        )
        .is_null()
            && (t1 >= span
                || !VirtualAlloc(
                    (base + t1) as *const c_void,
                    (span - t1) as usize,
                    MEM_COMMIT,
                    PAGE_READWRITE,
                )
                .is_null())
    };
    if !ok {
        // SAFETY: the reservation just made.
        unsafe { VirtualFree(base as *mut c_void, 0, MEM_RELEASE) };
        return Err("commit failed".into());
    }
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path).to_owned();
    if RUST.get().is_some() {
        match ffx_magic::Image::new(&name, &file, base) {
            Ok(img) => {
                if let Ok(mut i) = IMAGES.lock() {
                    i.push((base, std::sync::Arc::new(img)));
                }
            }
            Err(e) => log(&format!("overlay-emu: {name}: no Rust view ({e})")),
        }
    }
    let ov = match Overlay::load(&name, &file, &pe, base) {
        Ok(ov) => ov,
        Err(e) => {
            // SAFETY: as above.
            unsafe { VirtualFree(base as *mut c_void, 0, MEM_RELEASE) };
            return Err(e.to_string());
        }
    };
    // SAFETY: the overlay's entry point, run through the trap.
    let r = unsafe { dll_main(&ov, 1) };
    if r == 0 {
        ov.unload();
        // SAFETY: as above.
        unsafe { VirtualFree(base as *mut c_void, 0, MEM_RELEASE) };
        return Err("DllMain failed".into());
    }
    log(&format!(
        "overlay-emu: loaded {name} at {base:#010x} (code {:#010x}..{:#010x})",
        base + t0,
        base + t1
    ));
    Ok(base)
}

/// The vectored handler: a jump into an emulated overlay's code (an
/// access violation at the instruction pointer itself) continues in
/// [`enter_thunk`] with the target pushed.
unsafe extern "system" fn trap(info: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: the system passes a valid record and context.
    unsafe {
        let rec = &*(*info).ExceptionRecord;
        if rec.ExceptionCode.cast_unsigned() != 0xc000_0005 {
            return 0;
        }
        if rec.NumberParameters >= 2
            && rec.ExceptionInformation[0] == 1
            && commit_code_buffer(rec.ExceptionInformation[1])
        {
            return -1;
        }
        let at = rec.ExceptionAddress as u32;
        if rec.NumberParameters < 2 || rec.ExceptionInformation[1] as u32 != at {
            return 0;
        }
        if ffx_overlay::text_owner(at).is_none() {
            return 0;
        }
        let ctx = &mut *(*info).ContextRecord;
        ctx.Esp -= 4;
        (ctx.Esp as *mut u32).write(at);
        ctx.Eip = enter_thunk as *const () as u32;
    }
    -1 // EXCEPTION_CONTINUE_EXECUTION
}

/// Unicorn reserves its 32 MiB translated-code buffer and commits 4 MiB
/// at a time from a vectored handler of its own, which asks for 4 MiB even
/// in the last 4 MiB of the reservation; that commit fails and the write
/// crashes the game once the buffer is nearly full (seen in
/// battle-overdrive). This handler, first in line, commits what is left
/// of the reservation instead: a write into reserved, uncommitted
/// read-write-execute memory, up to 4 MiB or the reservation's end.
unsafe fn commit_code_buffer(at: usize) -> bool {
    use windows_sys::Win32::System::Memory::{
        MEM_RESERVE, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE_READWRITE, VirtualQuery,
    };
    // SAFETY: an out-parameter of the right size.
    let mut m: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: queries an address, no access.
    let n = unsafe {
        VirtualQuery(
            at as *const c_void,
            &raw mut m,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        )
    };
    if n == 0 || m.State != MEM_RESERVE || m.AllocationProtect != PAGE_EXECUTE_READWRITE {
        return false;
    }
    let size = m.RegionSize.min(0x40_0000);
    // SAFETY: commits pages of a reservation that is already ours to use.
    !unsafe { VirtualAlloc(m.BaseAddress, size, MEM_COMMIT, PAGE_EXECUTE_READWRITE) }.is_null()
}

/// What [`enter_thunk`] saves and [`enter`] answers: `pushad`'s eight
/// words, the flags, the answer area, then the target and the caller's
/// return address (a copy, away from the caller's stack).
#[repr(C)]
struct EnterFrame {
    edi: u32,
    esi: u32,
    ebp: u32,
    /// `pushad`'s `esp` slot, which `popad` skips: the caller's `esp` at
    /// the return address (`S`).
    s: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
    eflags: u32,
    /// Where the return address goes: the caller's `esp` after the
    /// return, minus 4.
    ret_slot: u32,
    has_st0: u32,
    st0: [u8; 10],
    _pad: [u8; 2],
    fpu_cw: u16,
    _pad2: u16,
    mxcsr: u32,
    _pad3: [u32; 3],
    target: u32,
    ret: u32,
}

const _: () = assert!(std::mem::size_of::<EnterFrame>() == 84);

/// A native jump into overlay code: `[esp]` the target, `[esp + 4]` the
/// caller's return address `S`. Saves the registers below an answer area,
/// probes the 64 KiB below them page by page and copies the frame below
/// that: the overlay code runs on the caller's stack from `S` down, where
/// the first copy was, and [`enter`] runs below the second. Then returns
/// to the caller with the emulated registers, `esp` past what the callee
/// popped and a float result in ST0.
#[unsafe(naked)]
unsafe extern "C" fn enter_thunk() {
    core::arch::naked_asm!(
        "sub esp, 40",
        "pushfd",
        "pushad",
        "mov ebx, esp",
        "fnstcw word ptr [ebx + 56]",
        "stmxcsr dword ptr [ebx + 60]",
        "lea eax, [ebx + 80]",
        "mov [ebx + 12], eax",
        "lea eax, [ebx - {gap}]",
        "mov ecx, esp",
        "2:",
        "sub ecx, 0x1000",
        "test dword ptr [ecx], eax",
        "cmp ecx, eax",
        "ja 2b",
        "and eax, -16",
        "cld",
        "mov esi, ebx",
        "mov edi, eax",
        "mov ecx, 21",
        "rep movsd",
        "mov ebx, eax",
        "lea esp, [ebx - 12]",
        "push ebx",
        "call {enter}",
        "mov esp, ebx",
        "fldcw word ptr [ebx + 56]",
        "ldmxcsr dword ptr [ebx + 60]",
        "cmp dword ptr [ebx + 40], 0",
        "je 3f",
        "fld tbyte ptr [ebx + 44]",
        "3:",
        "mov eax, [ebx + 80]",
        "mov ecx, [ebx + 36]",
        "mov [ecx], eax",
        "mov [ebx + 76], ecx",
        "popad",
        "popfd",
        "lea esp, [esp + 40]",
        "mov esp, [esp]",
        "ret",
        gap = const GAP,
        enter = sym enter,
    )
}

thread_local! {
    /// Nesting of [`enter`] on this thread, for the cost accounting.
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    /// Time spent in call-outs of the current top-level entry.
    static NATIVE: Cell<i64> = const { Cell::new(0) };
}

fn now() -> i64 {
    let mut t = 0i64;
    // SAFETY: an out-parameter.
    unsafe { QueryPerformanceCounter(&raw mut t) };
    t
}

#[derive(Default, Clone, Copy)]
struct Cost {
    calls: u64,
    total: i64,
    native: i64,
    call_outs: u64,
}

/// Per overlay function: calls, time, native time.
static COSTS: Mutex<Option<HashMap<u32, Cost>>> = Mutex::new(None);
static CALL_OUTS: Mutex<Option<HashMap<u32, u64>>> = Mutex::new(None);

thread_local! {
    static OUTS: RefCell<u64> = const { RefCell::new(0) };
}

/// Runs the trapped call in Unicorn; never panics (the game called it).
extern "C" fn enter(f: *mut EnterFrame) {
    // SAFETY: the thunk's frame.
    let fr = unsafe { &mut *f };
    let at = fr.s;
    let regs = Regs {
        eax: fr.eax,
        ecx: fr.ecx,
        edx: fr.edx,
        ebx: fr.ebx,
        esp: at,
        ebp: fr.ebp,
        esi: fr.esi,
        edi: fr.edi,
        eflags: fr.eflags,
        fpu_cw: fr.fpu_cw,
        mxcsr: fr.mxcsr,
        ..Regs::default()
    };
    if let Some((img, i)) = rust_port(fr.target) {
        enter_rust(fr, &img, i);
        return;
    }
    let depth = DEPTH.with(Cell::get);
    DEPTH.with(|d| d.set(depth + 1));
    let (t0, outs0) = (now(), OUTS.with(|o| *o.borrow()));
    if depth == 0 {
        NATIVE.with(|n| n.set(0));
    }
    let r = ffx_overlay::call(&mut OverlayHost, fr.target, &regs);
    DEPTH.with(|d| d.set(depth));
    if depth == 0 {
        let total = now() - t0;
        let native = NATIVE.with(Cell::get);
        let outs = OUTS.with(|o| *o.borrow()) - outs0;
        if let Ok(mut c) = COSTS.lock() {
            let e = c
                .get_or_insert_with(HashMap::new)
                .entry(fr.target)
                .or_default();
            e.calls += 1;
            e.total += total;
            e.native += native;
            e.call_outs += outs;
        }
    }
    match r {
        Ok(out) => {
            fr.eax = out.eax;
            fr.ecx = out.ecx;
            fr.edx = out.edx;
            fr.ebx = out.ebx;
            fr.ebp = out.ebp;
            fr.esi = out.esi;
            fr.edi = out.edi;
            fr.eflags = (fr.eflags & !0x8d5) | (out.eflags & 0x8d5);
            fr.ret_slot = out.esp - 4;
            fr.fpu_cw = out.fpu_cw;
            fr.mxcsr = out.mxcsr;
            fr.has_st0 = u32::from(out.st_len > 0);
            fr.st0 = out.st[0];
        }
        Err(e) => {
            log(&format!(
                "overlay-emu: call {:#010x} from {:#010x} failed: {e}",
                fr.target, fr.ret
            ));
            fr.eax = 0;
            fr.ret_slot = at;
            fr.has_st0 = 0;
        }
    }
}

/// [`call_native`]'s frame (offsets are the asm's).
#[repr(C)]
struct OutFrame {
    target: u32,           // 0
    eax: u32,              // 4
    ecx: u32,              // 8
    edx: u32,              // 12
    _ebx: u32,             // 16
    esi: u32,              // 20
    edi: u32,              // 24
    ebp: u32,              // 28
    fpu_cw: u16,           // 32
    _pad: u16,             // 34
    mxcsr: u32,            // 36
    st_in: u32,            // 40
    st: [[u8; 10]; 2],     // 44
    xmm0: [u8; 16],        // 64
    xmm1: [u8; 16],        // 80
    args: [u32; 32],       // 96
    saved_esp: u32,        // 224
    esp_after: u32,        // 228
    sw_before: u16,        // 232
    sw_after: u16,         // 234
    st_out: [u8; 10],      // 236
    _pad2: u16,            // 246
    has_st_out: u32,       // 248
    args_after: [u32; 32], // 252
}

const _: () = assert!(std::mem::size_of::<OutFrame>() == 380);
const ARGS: u32 = 32;

/// Calls `frame.target` natively with the overlay's registers, its 32
/// argument words copied onto this stack, its x87 inputs pushed and its
/// control words; records the registers, the stack after the return, a
/// pushed float result and the argument words as the callee left them.
/// `ebx` holds the frame (callee-saved), so the callee sees it instead of
/// the overlay's `ebx`; no standard convention passes anything there.
#[unsafe(naked)]
unsafe extern "C" fn call_native(frame: *mut OutFrame) {
    core::arch::naked_asm!(
        "push ebp",
        "push ebx",
        "push esi",
        "push edi",
        "mov ebx, [esp + 20]",
        "mov [ebx + 224], esp",
        "cld",
        "sub esp, {bytes}",
        "mov edi, esp",
        "lea esi, [ebx + 96]",
        "mov ecx, {words}",
        "rep movsd",
        "fldcw word ptr [ebx + 32]",
        "ldmxcsr dword ptr [ebx + 36]",
        "movdqu xmm0, xmmword ptr [ebx + 64]",
        "movdqu xmm1, xmmword ptr [ebx + 80]",
        "fnstsw ax",
        "mov [ebx + 232], ax",
        "cmp dword ptr [ebx + 40], 2",
        "jb 2f",
        "fld tbyte ptr [ebx + 54]",
        "2:",
        "cmp dword ptr [ebx + 40], 1",
        "jb 3f",
        "fld tbyte ptr [ebx + 44]",
        "3:",
        "mov eax, [ebx + 4]",
        "mov ecx, [ebx + 8]",
        "mov edx, [ebx + 12]",
        "mov esi, [ebx + 20]",
        "mov edi, [ebx + 24]",
        "mov ebp, [ebx + 28]",
        "call dword ptr [ebx]",
        "mov [ebx + 4], eax",
        "mov [ebx + 8], ecx",
        "mov [ebx + 12], edx",
        "mov [ebx + 20], esi",
        "mov [ebx + 24], edi",
        "mov [ebx + 28], ebp",
        "mov [ebx + 228], esp",
        "movdqu xmmword ptr [ebx + 64], xmm0",
        "movdqu xmmword ptr [ebx + 80], xmm1",
        "fnstcw word ptr [ebx + 32]",
        "stmxcsr dword ptr [ebx + 36]",
        "fnstsw ax",
        "mov [ebx + 234], ax",
        "mov cx, [ebx + 232]",
        "xor cx, ax",
        "test cx, 0x3800",
        "jz 4f",
        "fstp tbyte ptr [ebx + 236]",
        "mov dword ptr [ebx + 248], 1",
        "4:",
        "cld",
        "mov esi, [ebx + 224]",
        "sub esi, {bytes}",
        "lea edi, [ebx + 252]",
        "mov ecx, {words}",
        "rep movsd",
        "mov esp, [ebx + 224]",
        "pop edi",
        "pop esi",
        "pop ebx",
        "pop ebp",
        "ret",
        bytes = const 4 * ARGS,
        words = const ARGS,
    )
}

/// Calls `target` natively for overlay code ([`call_native`]).
fn native(target: u32, regs: &mut Regs) {
    let mut f = OutFrame {
        target,
        eax: regs.eax,
        ecx: regs.ecx,
        edx: regs.edx,
        _ebx: 0,
        esi: regs.esi,
        edi: regs.edi,
        ebp: regs.ebp,
        fpu_cw: regs.fpu_cw,
        _pad: 0,
        mxcsr: regs.mxcsr,
        st_in: u32::from(regs.st_len.min(2)),
        st: regs.st,
        xmm0: regs.xmm0,
        xmm1: regs.xmm1,
        args: [0; 32],
        saved_esp: 0,
        esp_after: 0,
        sw_before: 0,
        sw_after: 0,
        st_out: [0; 10],
        _pad2: 0,
        has_st_out: 0,
        args_after: [0; 32],
    };
    let at = regs.esp + 4;
    for (i, a) in (0u32..).zip(f.args.iter_mut()) {
        // SAFETY: the overlay's stack in the game's memory (identity):
        // its arguments and the frames above them.
        *a = unsafe { ((at + 4 * i) as *const u32).read_unaligned() };
    }
    // SAFETY: the frame matches the asm's layout.
    unsafe { call_native(&raw mut f) };
    // what the callee changed in its argument words goes back
    for i in 0..ARGS as usize {
        if f.args_after[i] != f.args[i] {
            // SAFETY: as above.
            unsafe { ((at + 4 * i as u32) as *mut u32).write_unaligned(f.args_after[i]) };
        }
    }
    let pops = f.esp_after.wrapping_sub(f.saved_esp.wrapping_sub(4 * ARGS));
    regs.eax = f.eax;
    regs.ecx = f.ecx;
    regs.edx = f.edx;
    regs.esi = f.esi;
    regs.edi = f.edi;
    regs.ebp = f.ebp;
    regs.esp = at + pops;
    regs.fpu_cw = f.fpu_cw;
    regs.mxcsr = f.mxcsr;
    regs.xmm0 = f.xmm0;
    regs.xmm1 = f.xmm1;
    regs.st_len = u8::from(f.has_st_out != 0);
    regs.st[0] = f.st_out;
}

/// The game's side of the emulated overlays.
struct OverlayHost;

/// `MSVCR110.dll`'s export `name` (the game's own runtime).
fn msvcr110(name: &CStr) -> Option<u32> {
    static CACHE: Mutex<Option<HashMap<String, u32>>> = Mutex::new(None);
    let mut c = CACHE.lock().ok()?;
    let m = c.get_or_insert_with(HashMap::new);
    let key = name.to_string_lossy().into_owned();
    if let Some(&a) = m.get(&key) {
        return Some(a);
    }
    // SAFETY: looks up a loaded module and an export by name.
    let a = unsafe {
        let h = GetModuleHandleA(c"MSVCR110.dll".as_ptr().cast());
        if h.is_null() {
            return None;
        }
        GetProcAddress(h, name.as_ptr().cast())? as usize as u32
    };
    m.insert(key, a);
    Some(a)
}

impl Host for OverlayHost {
    fn call_out(&mut self, target: u32, _slot: Option<u32>, regs: &mut Regs) -> Result<(), String> {
        OUTS.with(|o| *o.borrow_mut() += 1);
        let t0 = now();
        native(target, regs);
        if DEPTH.with(Cell::get) == 1 {
            NATIVE.with(|n| n.set(n.get() + now() - t0));
        }
        if let Ok(mut c) = CALL_OUTS.lock() {
            *c.get_or_insert_with(HashMap::new)
                .entry(target)
                .or_default() += 1;
        }
        Ok(())
    }

    fn import(&mut self, name: &str, regs: &mut Regs) -> Option<Result<(), String>> {
        let export: &CStr = match name {
            "rand" => c"rand",
            "_CIatan2" => c"_CIatan2",
            "_libm_sse2_atan_precise" => c"_libm_sse2_atan_precise",
            "_libm_sse2_sin_precise" => c"_libm_sse2_sin_precise",
            "_libm_sse2_cos_precise" => c"_libm_sse2_cos_precise",
            "_libm_sse2_acos_precise" => c"_libm_sse2_acos_precise",
            _ => return None,
        };
        Some(match msvcr110(export) {
            Some(f) => {
                native(f, regs);
                Ok(())
            }
            None => Err(format!("MSVCR110.dll has no {name}")),
        })
    }

    fn alloc(&mut self, size: u32) -> Option<u32> {
        // SAFETY: the process heap, zeroed.
        let p = unsafe { HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, size.max(1) as usize) };
        (!p.is_null()).then_some(p as u32)
    }

    fn free(&mut self, addr: u32) {
        // SAFETY: a block `alloc` returned.
        unsafe { HeapFree(GetProcessHeap(), 0, addr as *const c_void) };
    }
}

/// Logs an overlay's cost when it is unloaded and clears the counters.
fn report(ov: &Overlay) {
    let mut freq = 0i64;
    // SAFETY: an out-parameter.
    unsafe { QueryPerformanceFrequency(&raw mut freq) };
    let us = |t: i64| t * 1_000_000 / freq.max(1);
    let (b, e) = (ov.base(), ov.base() + ov.span());
    let mut lines = Vec::new();
    if let Ok(mut c) = COSTS.lock()
        && let Some(m) = c.as_mut()
    {
        let mut rows: Vec<(u32, Cost)> = m
            .iter()
            .filter(|(a, _)| **a >= b && **a < e)
            .map(|(a, c)| (*a, *c))
            .collect();
        rows.sort_by_key(|r| -r.1.total);
        let (mut total, mut native) = (0, 0);
        for (a, c) in &rows {
            total += c.total;
            native += c.native;
            lines.push(format!(
                "  +{:#07x} unicorn: {} calls, {} us in all ({} us/call), {} us native call-outs ({} of them)",
                a - b,
                c.calls,
                us(c.total),
                us(c.total) / c.calls.max(1).cast_signed(),
                us(c.native),
                c.call_outs
            ));
        }
        lines.insert(
            0,
            format!(
                "overlay-emu: {} cost: {} us in overlay calls, {} us of it in native call-outs, {} us emulating",
                ov.name(),
                us(total),
                us(native),
                us(total - native)
            ),
        );
        m.retain(|a, _| *a < b || *a >= e);
    }
    if let Ok(mut c) = RUST_CALLS.lock()
        && let Some(m) = c.as_mut()
    {
        let mut rows: Vec<(u32, u64)> = m
            .iter()
            .filter(|(a, _)| **a >= b && **a < e)
            .map(|(a, n)| (*a, *n))
            .collect();
        rows.sort_unstable();
        for (a, n) in &rows {
            lines.push(format!("  +{:#07x} rust: {n} calls", a - b));
        }
        m.retain(|a, _| *a < b || *a >= e);
    }
    if let Ok(mut c) = CALL_OUTS.lock()
        && let Some(m) = c.take()
    {
        let mut rows: Vec<(u32, u64)> = m.into_iter().collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.1));
        let top: Vec<String> = rows
            .iter()
            .take(12)
            .map(|(a, n)| format!("{a:#010x} x{n}"))
            .collect();
        lines.push(format!("  call-outs by target: {}", top.join(", ")));
    }
    log(&lines.join("\n"));
}
