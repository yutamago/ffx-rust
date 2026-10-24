//! Test harness: lets a test drive the running game over TCP.
//!
//! Enabled by `FFX_HARNESS=<port>` (listening on `127.0.0.1`). It then:
//!
//! - makes the keys bound to the PS2 buttons the test holds read as held
//!   (`Input_KeyHeld`, `0x00629d50`), so the game and its Flash menus see
//!   the buttons as if pressed on the keyboard;
//! - patches the swap chain's `Present` (from `D3D11CreateDeviceAndSwapChain`)
//!   to count frames and run commands on the game thread, between frames;
//! - adds keys the test holds to the `DirectInput` keyboard
//!   (`GetDeviceState`), for the PC-only menus that read keys directly;
//! - with `FFX_HARNESS_HOME=<Windows folder>`, answers the game's question
//!   for the Documents folder (`SHGetFolderPathW`, `CSIDL_PERSONAL`) with
//!   that folder, so settings and saves (autosaves too) stay in a sandbox;
//! - with `FFX_HARNESS_SEED=<n>`, seeds the random number generators with
//!   `n` instead of the clock (`Rng_ClockSeed`, `0x00798950`), so runs repeat;
//! - with `FFX_HARNESS_FAST=1`, runs frames back to back: the main loop's
//!   frame-limiter `Sleep` (`0x0062f3bb`) and the start-up loop's 33 ms
//!   `Time_Sleep` (`0x0066b345`, the logos) return at once and add their
//!   time to the game's clock (`Phyre_PTimerWin32_GetTime`, `0x0042fd80`), so the game still
//!   sees its usual frame rate; it also ends the start-up logos
//!   (`OpeningScreen.swf`, timed by Flash in real time) on their first frame;
//! - with `FFX_HARNESS_VIRTUAL_CLOCK=1` (and fast mode), the game's clock is
//!   its start time plus the skipped sleeps only, never real time, so every
//!   frame advances it by exactly one frame interval; with `lockstep on`
//!   frames then advance only as far as `wait` and `press` say (replays);
//! - gives the single-instance mutex (`CreateMutexW`, named
//!   `9B8F7468-6DE0-4C5A-9209-89D363F8FCC9` in `Game_Main`, `0x0062ee50`)
//!   a name of its own per harness port, so harness games run side by side
//!   (`docs/harness.md`, "Parallel games");
//! - with `FFX_HARNESS_BACKGROUND=1`, shows the window without activating
//!   it, keeps it at the bottom of the window stack and makes the keyboard
//!   a background device, so the game neither appears over other windows,
//!   takes the focus nor stops reading input without it; the real devices
//!   are then ignored, so typing in other windows does not reach the game.
//!
//! One command per line, one reply line per command (`ok …` or `err …`).
//! Addresses are `FFX.exe` addresses (rebased like the hooks) unless noted.
//!
//! | Command | Reply |
//! |---|---|
//! | `ping` | `ok pong` |
//! | `frame` | `ok <frames presented>` |
//! | `stats` | `ok frame <n> pad_reads <n> key_polls <n>` |
//! | `wait <n>` | after `n` more frames |
//! | `read <addr> <len>` | `ok <hex>` |
//! | `readp <addr> <offset> <len>` | reads at `*addr + offset` |
//! | `readabs <addr> <len>` | reads a process address |
//! | `write <addr> <hex>` | `ok` |
//! | `writep <addr> <offset> <hex>` | writes at `*addr + offset` |
//! | `pad [buttons]` | holds PS2 buttons, none to release |
//! | `press <buttons> <frames> [gap]` | holds buttons, releases, waits `gap` frames (default as long) |
//!
//! Buttons are names joined by `+` (`cross`, `circle`, `square`,
//! `triangle`, `up`, `down`, `left`, `right`, `start`, `select`, `l1`,
//! `r1`, `l2`, `r2`, `l3`, `r3`) or a hex mask (`0x20`). The pad word is the PS2 pad data with its two
//! bytes swapped: Circle `0x20`, Cross `0x40`, Start `0x800`, Up `0x1000`.
//!
//! | Command | Reply |
//! |---|---|
//! | `keys [dik…]` | holds DirectInput keys (hex scan codes), none to release |
//! | `key <dik[,dik…]> <frames> [gap]` | holds keys, releases, waits `gap` frames |
//! | `call <addr> [args…]` | `ok <eax>`: a cdecl call on the game thread |
//! | `shot <windows path>` | saves the back buffer as a 32-bit BMP |
//! | `loadsave <slot\|windows path>` | `ok <file> map <n> entrance <n> …`: starts the game from that save file without the title menu ([`crate::loadsave`]) |
//! | `events` | `ok <n>` then `n` lines: files opened since the last call |
//! | `gfxcapture <windows dir>` | `ok <n> draws`: the next frame's D3D11 calls (`gfx`, needs `FFX_GFX=1`) |
//! | `gfxstats` | `ok frames <n> draws <n>`: draws in the last frame |
//! | `overlay` | shows or hides the live renderer's overlay (as F10): `ok <shown>` |
//! | `overlayclick <x> <y>` | a left click in the overlay, in back buffer pixels |
//! | `overlaymouse <x> <y> [down\|up]` | moves the overlay's pointer there, pressing or releasing the left button |
//! | `overlaykey <vk hex> down\|up` | a key for the overlay (Windows virtual-key code) |
//! | `editor` | opens or closes the renderer's editor (as F11; shows the overlay) |
//! | `quit` | ends the process |

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};

use crate::log::log;

/// Set when the harness is on.
pub static ENABLED: AtomicBool = AtomicBool::new(false);
static FRAME: AtomicU32 = AtomicU32::new(0);
static CREATE_DEVICE_AND_SWAP_CHAIN: AtomicUsize = AtomicUsize::new(0);
static SHOW_WINDOW: AtomicUsize = AtomicUsize::new(0);
static XINPUT_GET_STATE: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_BOX_A: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_BOX_W: AtomicUsize = AtomicUsize::new(0);
static SH_GET_FOLDER_PATH: AtomicUsize = AtomicUsize::new(0);
static CREATE_MUTEX_W: AtomicUsize = AtomicUsize::new(0);
/// The single-instance mutex's name for this game (NUL-terminated).
static MUTEX_NAME: OnceLock<Vec<u16>> = OnceLock::new();
/// The sandbox Documents folder, NUL-terminated UTF-16.
static HOME: OnceLock<Vec<u16>> = OnceLock::new();
static PRESENT: AtomicUsize = AtomicUsize::new(0);
static BACKGROUND: AtomicBool = AtomicBool::new(false);
/// Fast mode: no frame limiter, no vsync.
static FAST: AtomicBool = AtomicBool::new(false);

/// PS2 buttons held by the test.
static BUTTONS: AtomicU32 = AtomicU32::new(0);
/// Bindings reported held for the test, for `stats`.
static PAD_READS: AtomicU32 = AtomicU32::new(0);
/// Trampoline to the original `Input_KeyHeld`.
pub static KEY_HELD_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The game's input object (`0x00ccb170`): its `+0x9c` counts the
/// (binding, pad bit) pairs at `*(+0xa4)`.
const INPUT: u32 = 0x00cc_b170;

/// Wraps `Input_KeyHeld` (`0x00629d50`, `thiscall(devices, binding,
/// device)`): for each pad bit the test holds, the first keyboard binding
/// (≥ 28) paired with it reads as held, as if its key were pressed. `Virtuos_InputManager_updateInput` (`0x00632a30`) asks this for every
/// binding when it builds the pad word, for the game and for the Flash
/// menus alike.
///
/// # Safety
/// Called by the game.
pub unsafe extern "thiscall" fn key_held(devices: *mut c_void, binding: u32, device: u32) -> u32 {
    // SAFETY: the trampoline runs the original with the same arguments.
    let held = unsafe {
        let original: unsafe extern "thiscall" fn(*mut c_void, u32, u32) -> u32 =
            std::mem::transmute(KEY_HELD_ORIGINAL.load(Ordering::Acquire));
        original(devices, binding, device)
    };
    let buttons = BUTTONS.load(Ordering::Acquire);
    if buttons == 0 || device != 0 || !ENABLED.load(Ordering::Relaxed) {
        return held;
    }
    let input = crate::rebase(INPUT);
    // SAFETY: the input object is a static of the game; its pair array is
    // set up before the first pad update.
    let bound = unsafe {
        let count = *((input + 0x9c) as *const u32) as usize;
        let pairs = *((input + 0xa4) as *const *const [u32; 2]);
        // For each held button, the first keyboard binding paired with it:
        // the key a player would press.
        !pairs.is_null()
            && binding >= 28
            && (0..32).any(|bit| {
                let bit = 1u32 << bit;
                buttons & bit != 0
                    && (0..count.min(256))
                        .map(|k| *pairs.add(k))
                        .find(|&[b, pair_bit]| b >= 28 && pair_bit == bit)
                        .is_some_and(|[b, _]| b == binding)
            })
    };
    if bound {
        PAD_READS.fetch_add(1, Ordering::Relaxed);
        (held & !0xff) | 1
    } else {
        held
    }
}

/// A PS2 button mask from names joined by `+`, or a hex mask.
fn parse_buttons(s: &str) -> Result<u32, String> {
    if s.starts_with("0x") {
        return parse_u32(s);
    }
    s.split('+').try_fold(0, |mask, name| {
        let bit = match name {
            // The PS2 pad data bytes, swapped: data[3] low, data[2] high.
            "l2" => 0x1,
            "r2" => 0x2,
            "l1" => 0x4,
            "r1" => 0x8,
            "triangle" => 0x10,
            "circle" => 0x20,
            "cross" => 0x40,
            "square" => 0x80,
            "select" => 0x100,
            "l3" => 0x200,
            "r3" => 0x400,
            "start" => 0x800,
            "up" => 0x1000,
            "right" => 0x2000,
            "down" => 0x4000,
            "left" => 0x8000,
            other => return Err(format!("unknown button {other:?}")),
        };
        Ok(mask | bit)
    })
}

/// Trampoline to the original `Rng_ClockSeed`.
pub static CLOCK_SEED_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The fixed seed, when `FFX_HARNESS_SEED` is set.
static SEED: OnceLock<u32> = OnceLock::new();

/// Wraps `Rng_ClockSeed` (`0x00798950`): the XOR of the system time's bytes
/// plus 1, which `MsRandMize` (`0x00798890`) turns into the seeds of the
/// global generator and the 68 generators at `0x01135ee0`. With
/// `FFX_HARNESS_SEED` it returns that value instead.
///
/// # Safety
/// Called by the game.
#[must_use]
pub unsafe extern "C" fn clock_seed() -> u32 {
    if let Some(&seed) = SEED.get() {
        log(&format!("  harness: random seed {seed}"));
        return seed;
    }
    // SAFETY: the trampoline runs the original.
    unsafe {
        let original: unsafe extern "C" fn() -> u32 =
            std::mem::transmute(CLOCK_SEED_ORIGINAL.load(Ordering::Acquire));
        original()
    }
}

/// Trampoline to the original `Phyre_PTimerWin32_GetTime`.
pub static TIME_NOW_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The virtual clock is on (`FFX_HARNESS_VIRTUAL_CLOCK=1`).
static VIRTUAL: AtomicBool = AtomicBool::new(false);
/// The clock's first reading, as `f64` bits (0: not read yet).
static VIRTUAL_START: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Frames advance only up to [`ALLOWED`] (`lockstep on`).
static LOCKSTEP: AtomicBool = AtomicBool::new(false);
/// The last frame the game may present in lockstep.
static ALLOWED: AtomicU32 = AtomicU32::new(0);
/// Seconds the frame limiter skipped, as `f64` bits.
static SKIPPED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The PS2 vsync handler `createVsyncEmuThread` (`0x00679e60`) was given,
/// on the virtual clock: run from `Present` instead of a real-time thread.
static VSYNC_CALLBACK: AtomicUsize = AtomicUsize::new(0);
/// Vsync ticks run so far (1/59.94 s of game time each).
static VSYNC_TICKS: AtomicU64 = AtomicU64::new(0);

/// `createVsyncEmuThread(callback)` on the virtual clock: no thread (which
/// would call `callback` every 16 ms of real time); [`run_vsync`] calls it
/// from the game thread as game time passes.
unsafe extern "C" fn virtual_vsync_start(callback: usize) {
    VSYNC_TICKS.store(vsync_ticks_now(), Ordering::Release);
    VSYNC_CALLBACK.store(callback, Ordering::Release);
}

/// `endVsyncEmuThread` on the virtual clock.
unsafe extern "C" fn virtual_vsync_end() {
    VSYNC_CALLBACK.store(0, Ordering::Release);
}

/// `VsyncEmu_Wake` (`0x00679f80`) on the virtual clock: there is no thread
/// to wake.
unsafe extern "C" fn virtual_vsync_wake() {}

fn vsync_ticks_now() -> u64 {
    // SAFETY: the wrapped clock, as the game reads it.
    let t = unsafe { time_now() };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive seconds
    let ticks = (t * 59.94) as u64;
    ticks
}

/// Calls the vsync handler once per vsync of game time since the last call.
fn run_vsync() {
    let cb = VSYNC_CALLBACK.load(Ordering::Acquire);
    if cb == 0 {
        return;
    }
    let now = vsync_ticks_now();
    let mut done = VSYNC_TICKS.load(Ordering::Acquire);
    while done < now {
        // SAFETY: the game's own vsync handler, cdecl without arguments,
        // as its thread called it.
        unsafe {
            let f: unsafe extern "C" fn() = std::mem::transmute(cb);
            f();
        }
        done += 1;
    }
    VSYNC_TICKS.store(done, Ordering::Release);
}

/// `FileSys_SubmitRead` (`0x0061c220`) on the virtual clock: runs the read
/// and its decompression on the game thread at once, as `BigFileStream_readVFile`
/// (`0x0061bfe0`) does, instead of on the thread pool, so a load takes the
/// same number of frames every run.
///
/// # Safety
/// Called by the game with a read request.
unsafe extern "C" fn sync_submit_read(request: usize) {
    type Read = unsafe extern "system" fn(usize, usize, usize);
    type Inflate = unsafe extern "system" fn(usize, usize, usize, u32);
    // SAFETY: the game's own thread-pool callbacks, called as
    // BigFileStream_readVFile calls them.
    unsafe {
        let read: Read = std::mem::transmute(crate::rebase(0x0061_b8d0));
        let inflate: Inflate = std::mem::transmute(crate::rebase(0x0061_ba80));
        read(0, request, 0);
        inflate(0, request, 0, 0);
    }
}

/// The limiter's replacement `Sleep`, called through this pointer by the
/// patched `call [ptr]` at `0x0062f3bb`.
static FAST_SLEEP: unsafe extern "system" fn(u32) = fast_sleep;

/// Wraps `Phyre_PTimerWin32_GetTime` (`0x0042fd80`, seconds): adds the skipped time.
///
/// # Safety
/// Called by the game.
#[must_use]
pub unsafe extern "C" fn time_now() -> f64 {
    let skipped = f64::from_bits(SKIPPED.load(Ordering::Acquire));
    if VIRTUAL.load(Ordering::Acquire) {
        let start = VIRTUAL_START.load(Ordering::Acquire);
        if start != 0 {
            return f64::from_bits(start) + skipped;
        }
    }
    // SAFETY: the trampoline runs the original, which returns in st(0).
    let now = unsafe {
        let original: unsafe extern "C" fn() -> f64 =
            std::mem::transmute(TIME_NOW_ORIGINAL.load(Ordering::Acquire));
        original()
    };
    if VIRTUAL.load(Ordering::Acquire) {
        let _ =
            VIRTUAL_START.compare_exchange(0, now.to_bits(), Ordering::AcqRel, Ordering::Acquire);
        return f64::from_bits(VIRTUAL_START.load(Ordering::Acquire)) + skipped;
    }
    now + skipped
}

/// The frame limiter's `Sleep(ms)` in fast mode: no wait, the clock moves.
unsafe extern "system" fn fast_sleep(ms: u32) {
    // On the virtual clock nothing else moves time: a Sleep(0) in a wait
    // loop would spin for ever.
    let ms = if VIRTUAL.load(Ordering::Relaxed) {
        ms.max(1)
    } else {
        ms
    };
    let mut bits = SKIPPED.load(Ordering::Acquire);
    loop {
        let next = (f64::from_bits(bits) + f64::from(ms) / 1000.0).to_bits();
        match SKIPPED.compare_exchange(bits, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(b) => bits = b,
        }
    }
}

static SLEEP_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static SLEEP_LOG: Mutex<Vec<(usize, u32, u32)>> = Mutex::new(Vec::new());

/// Diagnostic `Sleep` (`FFX_HARNESS_TRACE_SLEEP=1`): logs callers.
#[allow(clippy::cast_possible_truncation)]
unsafe extern "system" fn traced_sleep(ms: u32) {
    let mut frames = [std::ptr::null_mut::<c_void>(); 6];
    // SAFETY: a buffer of four frame slots.
    let n = unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace(
            1,
            6,
            frames.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    let base = crate::GAME_BASE.load(Ordering::Relaxed);
    // The first return address inside FFX.exe.
    // The second return address inside FFX.exe (the first is the game's
    // Sleep wrapper).
    let caller = frames[..usize::from(n)]
        .iter()
        .map(|f| *f as usize)
        .filter(|&a| a >= base && a < base + crate::SIZE_OF_IMAGE as usize)
        .nth(1)
        .unwrap_or(0);
    if let Ok(mut log) = SLEEP_LOG.lock()
        && log.len() < 4000
    {
        log.push((caller, ms, FRAME.load(Ordering::Relaxed)));
    }
    // SAFETY: the original from the import table.
    unsafe {
        let f: unsafe extern "system" fn(u32) =
            std::mem::transmute(SLEEP_ORIGINAL.load(Ordering::Acquire));
        f(ms);
    }
}

/// The start-up loop's `Time_Sleep(33)` (`cdecl`) in fast mode.
unsafe extern "C" fn fast_sleep_cdecl(ms: u32) {
    // SAFETY: same work as the stdcall version.
    unsafe { fast_sleep(ms) };
}

/// Points the start-up loop's `call Time_Sleep` (`0x0066b345`, in
/// `Boot_Step` `0x0066b2e0`: 33 ms a frame during the logos) at
/// `fast_sleep_cdecl`.
///
/// # Safety
/// `module` is the main module; the loop does not run yet.
unsafe fn patch_boot_limiter(module: usize) -> bool {
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    let site = (module + (0x0066_b345 - crate::IMAGE_BASE as usize)) as *mut u8;
    let time_sleep = module + (0x006f_2d20 - crate::IMAGE_BASE as usize);
    // SAFETY: inside the mapped image; checked to be `e8 <rel32 Time_Sleep>`.
    unsafe {
        let current = std::slice::from_raw_parts(site, 5);
        let rel = i32::from_le_bytes(current[1..5].try_into().expect("4 bytes"));
        if current[0] != 0xe8 || (site as usize + 5).wrapping_add_signed(rel as isize) != time_sleep
        {
            return false;
        }
        let mut old = 0;
        if VirtualProtect(site.cast(), 5, PAGE_EXECUTE_READWRITE, &raw mut old) == 0 {
            return false;
        }
        let target = fast_sleep_cdecl as *const () as usize;
        let rel = (target.wrapping_sub(site as usize + 5) as u32).to_le_bytes();
        site.add(1).copy_from_nonoverlapping(rel.as_ptr(), 4);
        VirtualProtect(site.cast(), 5, old, &raw mut old);
    }
    true
}

/// Points the main loop's `call [Sleep]` (`0x0062f3bb`) at `fast_sleep`.
///
/// # Safety
/// `module` is the main module; no thread runs the main loop yet.
unsafe fn patch_frame_limiter(module: usize) -> bool {
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    let site = (module + (0x0062_f3bb - crate::IMAGE_BASE as usize)) as *mut u8;
    let iat_sleep = module + (0x00b0_c100 - crate::IMAGE_BASE as usize);
    // SAFETY: inside the mapped image; checked to be `ff 15 <&Sleep>`.
    unsafe {
        let current = std::slice::from_raw_parts(site, 6);
        if current[..2] != [0xff, 0x15]
            || u32::from_le_bytes(current[2..6].try_into().expect("4 bytes")) as usize != iat_sleep
        {
            return false;
        }
        let mut old = 0;
        if VirtualProtect(site.cast(), 6, PAGE_EXECUTE_READWRITE, &raw mut old) == 0 {
            return false;
        }
        let ptr = u32::try_from(&raw const FAST_SLEEP as usize)
            .expect("32-bit")
            .to_le_bytes();
        site.add(2).copy_from_nonoverlapping(ptr.as_ptr(), 4);
        VirtualProtect(site.cast(), 6, old, &raw mut old);
    }
    true
}

/// `DirectInput` keys held by the test, by scan code.
static KEYS: Mutex<[bool; 256]> = Mutex::new([false; 256]);
static CREATE_DEVICE: AtomicUsize = AtomicUsize::new(0);
static GET_DEVICE_STATE: AtomicUsize = AtomicUsize::new(0);
static SET_COOPERATIVE_LEVEL: AtomicUsize = AtomicUsize::new(0);
/// `GetDeviceState` calls for the keyboard, for `stats`.
static KEY_POLLS: AtomicU32 = AtomicU32::new(0);

fn set_keys(codes: &[u8]) {
    if let Ok(mut keys) = KEYS.lock() {
        *keys = [false; 256];
        for &c in codes {
            keys[usize::from(c)] = true;
        }
    }
}

/// Wraps the game's `IDirectInput8`: devices it creates get the harness's
/// `GetDeviceState` and `SetCooperativeLevel`.
///
/// # Safety
/// `direct_input` must be a live `IDirectInput8` object.
pub unsafe fn wrap_direct_input(direct_input: *mut c_void) {
    // The harness's keys, and the overlay of the live renderer, which must
    // keep the game from seeing what is typed into it.
    let renderer = std::env::var("FFX_RENDERER").is_ok_and(|v| !v.is_empty() && v != "0");
    if !(ENABLED.load(Ordering::Acquire) || renderer) || CREATE_DEVICE.load(Ordering::Acquire) != 0
    {
        return;
    }
    // SAFETY: a COM object starts with its vtable; CreateDevice is entry 3.
    if let Some(o) = unsafe {
        crate::iat::patch_vtable(
            *direct_input.cast::<*mut usize>(),
            3,
            create_device as *const () as usize,
        )
    } {
        CREATE_DEVICE.store(o, Ordering::Release);
        log("  harness: IDirectInput8::CreateDevice hooked");
    }
}

/// `IDirectInput8::CreateDevice`: patches the device vtable once.
unsafe extern "system" fn create_device(
    this: *mut c_void,
    guid: *const c_void,
    device: *mut *mut c_void,
    outer: *mut c_void,
) -> i32 {
    // SAFETY: the original from the vtable, same arguments.
    let hr = unsafe {
        let original: unsafe extern "system" fn(
            *mut c_void,
            *const c_void,
            *mut *mut c_void,
            *mut c_void,
        ) -> i32 = std::mem::transmute(CREATE_DEVICE.load(Ordering::Acquire));
        original(this, guid, device, outer)
    };
    if hr >= 0 && !device.is_null() && GET_DEVICE_STATE.load(Ordering::Acquire) == 0 {
        // SAFETY: on success `*device` is a live IDirectInputDevice8, whose
        // vtable has GetDeviceState at 9 and SetCooperativeLevel at 13.
        unsafe {
            let vtable = *(*device).cast::<*mut usize>();
            if let Some(o) =
                crate::iat::patch_vtable(vtable, 9, get_device_state as *const () as usize)
            {
                GET_DEVICE_STATE.store(o, Ordering::Release);
            }
            if let Some(o) =
                crate::iat::patch_vtable(vtable, 13, set_cooperative_level as *const () as usize)
            {
                SET_COOPERATIVE_LEVEL.store(o, Ordering::Release);
            }
        }
        log("  harness: IDirectInputDevice8 hooked");
    }
    hr
}

/// `SetCooperativeLevel`: background and non-exclusive in background mode.
unsafe extern "system" fn set_cooperative_level(
    this: *mut c_void,
    window: *mut c_void,
    mut flags: u32,
) -> i32 {
    const DISCL_NONEXCLUSIVE: u32 = 2;
    const DISCL_FOREGROUND: u32 = 4;
    const DISCL_BACKGROUND: u32 = 8;
    if BACKGROUND.load(Ordering::Relaxed) && flags & DISCL_FOREGROUND != 0 {
        flags = (flags & !(DISCL_FOREGROUND | 1)) | DISCL_BACKGROUND | DISCL_NONEXCLUSIVE;
    }
    // SAFETY: the original from the vtable.
    unsafe {
        let original: unsafe extern "system" fn(*mut c_void, *mut c_void, u32) -> i32 =
            std::mem::transmute(SET_COOPERATIVE_LEVEL.load(Ordering::Acquire));
        original(this, window, flags)
    }
}

/// `GetDeviceState`: the keyboard (256 bytes) gets the held keys added; a
/// keyboard that is not acquired reads as all keys up instead of failing
/// (in the harness only).
unsafe extern "system" fn get_device_state(this: *mut c_void, size: u32, data: *mut u8) -> i32 {
    // SAFETY: the original from the vtable, same arguments.
    let mut hr = unsafe {
        let original: unsafe extern "system" fn(*mut c_void, u32, *mut u8) -> i32 =
            std::mem::transmute(GET_DEVICE_STATE.load(Ordering::Acquire));
        original(this, size, data)
    };
    // In background mode the devices are background devices, so they would
    // report what the user types into other windows: their real state is
    // dropped and only the test's keys count.
    let background = BACKGROUND.load(Ordering::Relaxed);
    let isolate = background || crate::gfx::live::overlay_takes_keys();
    if (background || crate::gfx::live::overlay_visible()) && !data.is_null() && size != 256 {
        // SAFETY: the caller's buffer holds `size` bytes.
        unsafe { std::ptr::write_bytes(data, 0, size as usize) };
        hr = 0;
    }
    if size == 256 && !data.is_null() {
        // Outside the harness (the renderer's overlay only) the failure
        // stands: it is how the game knows to acquire the keyboard again
        // after the window was in the background.
        if hr < 0 && !ENABLED.load(Ordering::Relaxed) {
            return hr;
        }
        KEY_POLLS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller's buffer holds `size` bytes.
        let state = unsafe { std::slice::from_raw_parts_mut(data, 256) };
        if hr < 0 || isolate {
            state.fill(0);
            hr = 0;
        }
        if let Ok(keys) = KEYS.lock() {
            for (s, &k) in state.iter_mut().zip(keys.iter()) {
                if k {
                    *s = 0x80;
                }
            }
        }
    }
    hr
}

/// Work for the game thread.
enum Job {
    /// Reply at this frame.
    WaitUntil(u32),
    /// Release the pad at this frame, then wait until the second.
    Release(u32, u32),
    Call(usize, Vec<u32>),
    Shot(String),
    Snapshot(String),
    FpuControl,
    /// Start the game from a save (`loadsave`, [`crate::loadsave`]).
    LoadSave(String),
    /// Until the bytes at the range equal the value (under the mask), or
    /// the frame limit passes: `(range, value, mask, limit frame)`.
    WaitFor(TraceRange, Vec<u8>, Vec<u8>, u32),
}

static JOBS: Mutex<VecDeque<(Job, Sender<String>)>> = Mutex::new(VecDeque::new());

/// A memory range a trace hashes each frame: `addr:len` (an `ffx.sym`
/// address) or `*addr+off:len` (through the pointer stored at `addr`).
struct TraceRange {
    addr: u32,
    deref: bool,
    off: u32,
    len: usize,
}

struct Trace {
    out: std::io::BufWriter<std::fs::File>,
    ranges: Vec<TraceRange>,
}

static TRACE: Mutex<Option<Trace>> = Mutex::new(None);

fn parse_trace_range(spec: &str) -> Result<TraceRange, String> {
    let (at, len) = spec
        .split_once(':')
        .ok_or_else(|| format!("{spec}: want addr:len"))?;
    let len = parse_u32(len)? as usize;
    let (deref, at) = at.strip_prefix('*').map_or((false, at), |a| (true, a));
    let (addr, off) = match at.split_once('+') {
        Some((a, o)) => (parse_u32(a)?, parse_u32(o)?),
        None => (parse_u32(at)?, 0),
    };
    Ok(TraceRange {
        addr,
        deref,
        off,
        len,
    })
}

/// The bytes of a trace range, if readable.
fn range_bytes(r: &TraceRange) -> Option<Vec<u8>> {
    let at = crate::rebase(r.addr);
    let at = if r.deref {
        let p = read_memory(at, 4).ok()?;
        u32::from_le_bytes(p.try_into().ok()?) as usize + r.off as usize
    } else {
        at + r.off as usize
    };
    read_memory(at, r.len).ok()
}

/// Whether a range equals `value` under `mask` (bytes, little-endian).
fn range_matches(r: &TraceRange, value: &[u8], mask: &[u8]) -> bool {
    range_bytes(r).is_some_and(|b| {
        b.iter()
            .zip(value)
            .zip(mask)
            .all(|((x, v), m)| x & m == v & m)
    })
}

/// FNV-1a over the range, or `-` when it is not readable.
fn trace_hash(r: &TraceRange) -> String {
    let at = crate::rebase(r.addr);
    let at = if r.deref {
        match read_memory(at, 4) {
            Ok(p) => u32::from_le_bytes(p.try_into().expect("4 bytes")) as usize + r.off as usize,
            Err(_) => return "-".into(),
        }
    } else {
        at + r.off as usize
    };
    match read_memory(at, r.len) {
        Ok(bytes) => {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in bytes {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            format!("{h:016x}")
        }
        Err(_) => "-".into(),
    }
}

/// Writes this frame's line of an active trace.
fn trace_frame(frame: u32) {
    use std::io::Write as _;
    if let Ok(mut t) = TRACE.lock()
        && let Some(t) = t.as_mut()
    {
        let hashes: Vec<String> = t.ranges.iter().map(trace_hash).collect();
        let _ = writeln!(t.out, "{frame} {}", hashes.join(" "));
    }
}
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Records an event for the `events` command.
pub fn event(line: String) {
    if ENABLED.load(Ordering::Relaxed)
        && let Ok(mut e) = EVENTS.lock()
    {
        e.push(line);
    }
}

/// Replaces `XInputGetState` in background mode: no controller, so a real
/// one does not reach the game.
unsafe extern "system" fn xinput_get_state(_user: u32, _state: *mut c_void) -> u32 {
    const ERROR_DEVICE_NOT_CONNECTED: u32 = 1167;
    ERROR_DEVICE_NOT_CONNECTED
}

/// Replaces `SHGetFolderPathW`: the Documents folder is the sandbox.
unsafe extern "system" fn sh_get_folder_path(
    window: *mut c_void,
    csidl: i32,
    token: *mut c_void,
    flags: u32,
    out: *mut u16,
) -> i32 {
    const CSIDL_PERSONAL: i32 = 5;
    if csidl & 0xff == CSIDL_PERSONAL
        && !out.is_null()
        && let Some(home) = HOME.get()
    {
        // SAFETY: the caller passes a MAX_PATH buffer; the home path is
        // checked to fit when it is set.
        unsafe { std::ptr::copy_nonoverlapping(home.as_ptr(), out, home.len()) };
        return 0;
    }
    // SAFETY: the original from the import table, same arguments.
    unsafe {
        let original: unsafe extern "system" fn(
            *mut c_void,
            i32,
            *mut c_void,
            u32,
            *mut u16,
        ) -> i32 = std::mem::transmute(SH_GET_FOLDER_PATH.load(Ordering::Acquire));
        original(window, csidl, token, flags, out)
    }
}

/// The name `Game_Main` (`0x0062ee50`) gives its single-instance mutex; a
/// second game finding it ends with "Same program is already running.".
const INSTANCE_MUTEX: &str = "9B8F7468-6DE0-4C5A-9209-89D363F8FCC9";

/// Replaces `CreateMutexW`: the single-instance mutex gets this game's own
/// name ([`MUTEX_NAME`]), other mutexes are created as asked.
unsafe extern "system" fn create_mutex_w(
    attributes: *mut c_void,
    owner: i32,
    name: *const u16,
) -> *mut c_void {
    let mut name = name;
    if !name.is_null()
        && let Some(own) = MUTEX_NAME.get()
    {
        let wanted = INSTANCE_MUTEX.encode_utf16().chain([0]);
        // SAFETY: a NUL-terminated name from the caller; the comparison
        // stops at the first difference or the NUL.
        let same = wanted
            .enumerate()
            .all(|(i, c)| unsafe { *name.add(i) } == c);
        if same {
            name = own.as_ptr();
        }
    }
    // SAFETY: the original from the import table, same arguments.
    unsafe {
        let original: unsafe extern "system" fn(*mut c_void, i32, *const u16) -> *mut c_void =
            std::mem::transmute(CREATE_MUTEX_W.load(Ordering::Acquire));
        original(attributes, owner, name)
    }
}

/// Replaces `MessageBoxA` in background mode: logs the text and answers
/// OK, so no dialog appears on the user's desktop.
unsafe extern "system" fn message_box_a(
    _w: *mut c_void,
    text: *const u8,
    caption: *const u8,
    _t: u32,
) -> i32 {
    let s = |p: *const u8| {
        if p.is_null() {
            String::new()
        } else {
            // SAFETY: NUL-terminated, from the caller.
            unsafe { std::ffi::CStr::from_ptr(p.cast()) }
                .to_string_lossy()
                .into_owned()
        }
    };
    log(&format!(
        "  harness: message box suppressed: [{}] {}",
        s(caption),
        s(text)
    ));
    event(format!("messagebox {}", s(text)));
    1 // IDOK
}

/// Replaces `MessageBoxW` in background mode, like [`message_box_a`].
unsafe extern "system" fn message_box_w(
    _w: *mut c_void,
    text: *const u16,
    caption: *const u16,
    _t: u32,
) -> i32 {
    let s = |p: *const u16| {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: NUL-terminated, from the caller.
        let len = (0..4096).take_while(|&i| unsafe { *p.add(i) } != 0).count();
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, len) })
    };
    log(&format!(
        "  harness: message box suppressed: [{}] {}",
        s(caption),
        s(text)
    ));
    event(format!("messagebox {}", s(text)));
    1 // IDOK
}

/// Replaces `ShowWindow`: in background mode, shows without activating and
/// puts the window at the bottom of the window stack, so it neither takes
/// the focus nor appears over the windows on screen.
unsafe extern "system" fn show_window(window: *mut c_void, mut cmd: i32) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    const SW_SHOWNOACTIVATE: i32 = 4;
    const SW_SHOWMINNOACTIVE: i32 = 7;
    let background = BACKGROUND.load(Ordering::Relaxed) && cmd != 0;
    if background {
        cmd = if cmd == 2 || cmd == 6 || cmd == 7 {
            SW_SHOWMINNOACTIVE
        } else {
            SW_SHOWNOACTIVATE
        };
    }
    let original: unsafe extern "system" fn(*mut c_void, i32) -> i32 =
        // SAFETY: the address came from the import table.
        unsafe { std::mem::transmute(SHOW_WINDOW.load(Ordering::Acquire)) };
    // SAFETY: same arguments as the game's call.
    let was_visible = unsafe { original(window, cmd) };
    if background {
        // SAFETY: the game's own window handle; only the stacking order changes.
        unsafe {
            SetWindowPos(
                window,
                HWND_BOTTOM,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
    was_visible
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

/// Replaces `D3D11CreateDeviceAndSwapChain`: patches `Present` of the new
/// swap chain's vtable.
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
    if hr >= 0
        && !swap_chain.is_null()
        // SAFETY: on success the swap chain pointer was written.
        && let Some(sc) = Some(unsafe { *swap_chain }).filter(|p| !p.is_null())
        && PRESENT.load(Ordering::Acquire) == 0
    {
        // SAFETY: a COM object starts with its vtable; Present is entry 8.
        let original = unsafe {
            crate::iat::patch_vtable(*sc.cast::<*mut usize>(), 8, present as *const () as usize)
        };
        if let Some(o) = original {
            PRESENT.store(o, Ordering::Release);
            log("  harness: Present hooked");
        }
    }
    log(&format!(
        "  harness: D3D11CreateDeviceAndSwapChain -> {hr:#x}"
    ));
    hr
}

/// Replaces `IDXGISwapChain::Present`: runs due jobs, then presents.
/// Runs the jobs due at `frame`.
///
/// # Safety
/// Inside the swap chain's own `Present`, on the game thread.
unsafe fn run_due_jobs(frame: u32, swap_chain: *mut c_void) {
    let due: Vec<(Job, Sender<String>)> = JOBS.lock().map_or_else(
        |_| Vec::new(),
        |mut jobs| {
            let mut keep = VecDeque::new();
            let mut due = Vec::new();
            while let Some(job) = jobs.pop_front() {
                let ready = match job.0 {
                    Job::WaitUntil(f) | Job::Release(f, _) => frame >= f,
                    Job::Call(..)
                    | Job::Shot(_)
                    | Job::Snapshot(_)
                    | Job::FpuControl
                    | Job::LoadSave(_) => true,
                    Job::WaitFor(ref r, ref value, ref mask, limit) => {
                        frame >= limit || range_matches(r, value, mask)
                    }
                };
                if ready {
                    due.push(job);
                } else {
                    keep.push_back(job);
                }
            }
            *jobs = keep;
            due
        },
    );
    for (job, reply) in due {
        match job {
            Job::WaitUntil(_) => {
                let _ = reply.send(format!("ok {frame}"));
            }
            Job::Release(_, then) => {
                BUTTONS.store(0, Ordering::Release);
                set_keys(&[]);
                if let Ok(mut jobs) = JOBS.lock() {
                    jobs.push_back((Job::WaitUntil(then), reply));
                }
            }
            Job::Call(address, args) => {
                // SAFETY: the test asked for this call, on the game thread.
                let eax = unsafe { call(address, &args) };
                let _ = reply.send(format!("ok {eax:#x}"));
            }
            Job::Snapshot(path) => {
                let r = crate::snapshot::take(&path);
                let _ = reply.send(r.map_or_else(
                    |e| format!("err {e}"),
                    |(n, bytes)| format!("ok {n} regions {bytes} bytes"),
                ));
            }
            Job::WaitFor(r, value, mask, _) => {
                let hit = range_matches(&r, &value, &mask);
                // hold here, on the game thread, before another frame runs
                if LOCKSTEP.load(Ordering::Acquire) {
                    ALLOWED.store(frame, Ordering::Release);
                }
                let _ = reply.send(if hit {
                    format!("ok {frame}")
                } else {
                    format!("err timeout at {frame}")
                });
            }
            Job::FpuControl => {
                let _ = reply.send(format!("ok {:#06x}", crate::snapshot::fpu_control()));
            }
            Job::LoadSave(spec) => {
                let r = crate::loadsave::load(&spec);
                log(&format!("loadsave {spec} at frame {frame}: {r:?}"));
                let _ = reply.send(r.map_or_else(|e| format!("err {e}"), |s| format!("ok {s}")));
            }
            Job::Shot(path) => {
                // SAFETY: the swap chain is live inside its own Present.
                let r = unsafe { screenshot(swap_chain, &path) };
                let _ = reply
                    .send(r.map_or_else(|e| format!("err {e}"), |(w, h)| format!("ok {w} {h}")));
            }
        }
    }
}

unsafe extern "system" fn present(swap_chain: *mut c_void, interval: u32, flags: u32) -> i32 {
    let frame = FRAME.fetch_add(1, Ordering::AcqRel) + 1;
    trace_frame(frame);
    // Not a barrier on the asset loader (AsyncLoadManager): some of its
    // tasks need frames to pass, so holding the game thread until it is
    // idle deadlocks. Replays wait for game state instead (`waitfor`).
    if VIRTUAL.load(Ordering::Acquire) {
        run_vsync();
    }
    if FAST.load(Ordering::Relaxed) {
        skip_opening_screen();
    }
    // SAFETY: forwarded: inside the swap chain's own Present.
    unsafe { run_due_jobs(frame, swap_chain) };
    // Lockstep: hold the game here until the script allows more frames,
    // still answering jobs (calls, snapshots, shots) meanwhile.
    while LOCKSTEP.load(Ordering::Acquire) && frame >= ALLOWED.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(1));
        // SAFETY: as above.
        unsafe { run_due_jobs(frame, swap_chain) };
    }
    // SAFETY: the address came from the vtable.
    let original: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32 =
        unsafe { std::mem::transmute(PRESENT.load(Ordering::Acquire)) };
    // Fast mode presents without waiting for the vertical blank.
    let interval = if FAST.load(Ordering::Relaxed) {
        0
    } else {
        interval
    };
    // SAFETY: forwarded, the sync interval perhaps lowered.
    unsafe { original(swap_chain, interval, flags) }
}

/// Ends the start-up logos as their movie's `Finish` callback
/// (`0x00652400`) does: `graphicInitScene` (`0x006416c0`) shows
/// `OpeningScreen.swf` while the flag `0x00ccb9c2` is set and loads the game
/// meanwhile; the loading waits that follow the loop still run. Flash times
/// the movie in real time, so fast mode cannot shorten it otherwise.
fn skip_opening_screen() {
    let flag = crate::rebase(0x00cc_b9c2) as *mut u8;
    // SAFETY: a byte in the image's data; the game thread is the only other
    // user and is here, inside Present.
    unsafe {
        if flag.read_volatile() != 0 {
            flag.write_volatile(0);
        }
    }
}

/// A cdecl call with up to eight `u32` arguments.
pub(crate) unsafe fn call(address: usize, args: &[u32]) -> u32 {
    type F0 = unsafe extern "C" fn() -> u32;
    type F8 = unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32;
    let a = |i: usize| args.get(i).copied().unwrap_or(0);
    // SAFETY: cdecl callers clean the stack, so passing extra zero arguments
    // to a function that takes fewer is harmless.
    unsafe {
        if args.is_empty() {
            std::mem::transmute::<usize, F0>(address)()
        } else {
            std::mem::transmute::<usize, F8>(address)(
                a(0),
                a(1),
                a(2),
                a(3),
                a(4),
                a(5),
                a(6),
                a(7),
            )
        }
    }
}

/// Calls COM method `index` of `object`.
macro_rules! com {
    ($object:expr, $index:expr, $ty:ty $(, $arg:expr)*) => {{
        let object: *mut c_void = $object;
        let vtable = *(object as *const *const usize);
        let f: $ty = std::mem::transmute(*vtable.add($index));
        f(object $(, $arg)*)
    }};
}

/// Copies the swap chain's back buffer to a BMP file.
#[allow(clippy::too_many_lines)] // one COM sequence
unsafe fn screenshot(swap_chain: *mut c_void, path: &str) -> Result<(u32, u32), String> {
    const IID_TEXTURE2D: [u8; 16] = [
        0xf2, 0xaa, 0x15, 0x6f, 0x08, 0xd2, 0x89, 0x4e, 0x9a, 0xb4, 0x48, 0x95, 0x35, 0xd3, 0x4f,
        0x9c,
    ];
    // SAFETY: COM calls on live D3D11 objects, with the documented vtable
    // slots (IDXGISwapChain::GetBuffer 9; ID3D11DeviceChild::GetDevice 3;
    // ID3D11Texture2D::GetDesc 10; ID3D11Device::CreateTexture2D 5,
    // GetImmediateContext 40; ID3D11DeviceContext::Map 14, Unmap 15,
    // CopyResource 47; IUnknown::Release 2).
    unsafe {
        let mut back: *mut c_void = std::ptr::null_mut();
        let hr = com!(
            swap_chain,
            9,
            unsafe extern "system" fn(*mut c_void, u32, *const u8, *mut *mut c_void) -> i32,
            0,
            IID_TEXTURE2D.as_ptr(),
            &raw mut back
        );
        if hr < 0 || back.is_null() {
            return Err(format!("GetBuffer {hr:#x}"));
        }
        let mut desc = [0u32; 11];
        com!(
            back,
            10,
            unsafe extern "system" fn(*mut c_void, *mut u32),
            desc.as_mut_ptr()
        );
        let mut device: *mut c_void = std::ptr::null_mut();
        com!(
            back,
            3,
            unsafe extern "system" fn(*mut c_void, *mut *mut c_void),
            &raw mut device
        );
        let mut context: *mut c_void = std::ptr::null_mut();
        com!(
            device,
            40,
            unsafe extern "system" fn(*mut c_void, *mut *mut c_void),
            &raw mut context
        );
        let (width, height, format, samples) = (desc[0], desc[1], desc[4], desc[5]);
        let release = |o: *mut c_void| {
            if !o.is_null() {
                com!(o, 2, unsafe extern "system" fn(*mut c_void) -> u32);
            }
        };
        let done = |r: Result<(u32, u32), String>| {
            release(context);
            release(device);
            release(back);
            r
        };
        if samples != 1 {
            return done(Err(format!("multisampled back buffer ({samples})")));
        }
        // A staging copy the CPU can read.
        let mut staging_desc = desc;
        staging_desc[2] = 1; // mip levels
        staging_desc[3] = 1; // array size
        staging_desc[7] = 3; // D3D11_USAGE_STAGING
        staging_desc[8] = 0; // no bind flags
        staging_desc[9] = 0x2_0000; // D3D11_CPU_ACCESS_READ
        staging_desc[10] = 0;
        let mut staging: *mut c_void = std::ptr::null_mut();
        let hr = com!(
            device,
            5,
            unsafe extern "system" fn(
                *mut c_void,
                *const u32,
                *const c_void,
                *mut *mut c_void,
            ) -> i32,
            staging_desc.as_ptr(),
            std::ptr::null(),
            &raw mut staging
        );
        if hr < 0 {
            return done(Err(format!("CreateTexture2D {hr:#x}")));
        }
        com!(
            context,
            47,
            unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void),
            staging,
            back
        );
        let mut mapped = [0usize; 3]; // pData, RowPitch, DepthPitch
        let hr = com!(
            context,
            14,
            unsafe extern "system" fn(*mut c_void, *mut c_void, u32, u32, u32, *mut usize) -> i32,
            staging,
            0,
            1,
            0,
            mapped.as_mut_ptr()
        );
        if hr < 0 {
            release(staging);
            return done(Err(format!("Map {hr:#x}")));
        }
        // R8G8B8A8 (28, 29) or B8G8R8A8 (87, 91).
        let swap = matches!(format, 28 | 29);
        let known = swap || matches!(format, 87 | 91);
        let (data, pitch) = (mapped[0] as *const u8, mapped[1]);
        let mut bmp = Vec::with_capacity(54 + 4 * (width * height) as usize);
        let image = 4 * width * height;
        bmp.extend(b"BM");
        bmp.extend((54 + image).to_le_bytes());
        bmp.extend([0u8; 4]);
        bmp.extend(54u32.to_le_bytes());
        bmp.extend(40u32.to_le_bytes());
        bmp.extend(width.to_le_bytes());
        bmp.extend(height.to_le_bytes()); // bottom-up rows
        bmp.extend(1u16.to_le_bytes());
        bmp.extend(32u16.to_le_bytes());
        bmp.extend([0u8; 4]);
        bmp.extend(image.to_le_bytes());
        bmp.extend([0u8; 16]);
        for y in (0..height as usize).rev() {
            let row = std::slice::from_raw_parts(data.add(y * pitch), 4 * width as usize);
            for px in row.as_chunks::<4>().0 {
                if swap {
                    bmp.extend([px[2], px[1], px[0], 255]);
                } else {
                    bmp.extend([px[0], px[1], px[2], 255]);
                }
            }
        }
        com!(
            context,
            15,
            unsafe extern "system" fn(*mut c_void, *mut c_void, u32),
            staging,
            0
        );
        release(staging);
        if !known {
            return done(Err(format!("back buffer format {format}")));
        }
        done(
            std::fs::write(path, bmp)
                .map(|()| (width, height))
                .map_err(|e| e.to_string()),
        )
    }
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let t = s.trim_start_matches("0x");
    if s.starts_with("0x") {
        u32::from_str_radix(t, 16)
    } else {
        t.parse()
    }
    .map_err(|e| format!("{s}: {e}"))
}

/// A scan code, hex with or without `0x`.
fn parse_u8(s: &str) -> Result<u8, String> {
    u8::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| format!("{s}: {e}"))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Reads process memory, failing instead of faulting on unmapped pages.
/// `len` bytes of this process at `address` (an error where they are not
/// readable).
///
/// # Errors
/// When the memory cannot be read.
pub fn read_memory(address: usize, len: usize) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut out = vec![0u8; len];
    let mut read = 0;
    // SAFETY: ReadProcessMemory validates the source range itself.
    let ok = unsafe {
        ReadProcessMemory(
            GetCurrentProcess(),
            address as *const c_void,
            out.as_mut_ptr().cast(),
            len,
            &raw mut read,
        )
    };
    if ok == 0 || read != len {
        return Err(format!("cannot read {len} bytes at {address:#x}"));
    }
    Ok(out)
}

/// Call log (`FFX_HARNESS_CALL_LOG=<windows path>`): Rust replacements can
/// note each call (frame, thread, caller, a value), to find where two runs
/// that should agree part ways.
static CALL_LOG: Mutex<Option<std::io::BufWriter<std::fs::File>>> = Mutex::new(None);
static CALL_LOG_ON: AtomicBool = AtomicBool::new(false);

/// Notes a call in the call log, if there is one.
pub fn log_call(tag: &str, value: u32) {
    use std::io::Write as _;
    if !CALL_LOG_ON.load(Ordering::Relaxed) {
        return;
    }
    let mut frames = [std::ptr::null_mut::<c_void>(); 4];
    // SAFETY: a buffer of four frame slots.
    let n = unsafe {
        windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace(
            2,
            4,
            frames.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    let base = crate::GAME_BASE.load(Ordering::Relaxed);
    let callers: Vec<String> = frames[..usize::from(n)]
        .iter()
        .map(|f| *f as usize)
        .filter(|&a| a >= base && a < base + crate::SIZE_OF_IMAGE as usize)
        .map(|a| format!("{:08x}", a - base + crate::IMAGE_BASE as usize))
        .collect();
    // SAFETY: plain call.
    let thread = unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() };
    if let Ok(mut log) = CALL_LOG.lock()
        && let Some(w) = log.as_mut()
    {
        let _ = writeln!(
            w,
            "{} {thread} {tag} {value:#x} {}",
            FRAME.load(Ordering::Relaxed),
            callers.join(",")
        );
    }
}

/// Every thread of the game: `id:start` with the start address as an
/// `ffx.sym` address when it is in `FFX.exe`, else as `module+offset`.
fn list_threads() -> String {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenThread, THREAD_QUERY_INFORMATION,
    };
    type Query = unsafe extern "system" fn(isize, u32, *mut c_void, u32, *mut u32) -> i32;
    // SAFETY: plain Win32 calls; every pointer is local.
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        let Some(query) = GetProcAddress(ntdll, c"NtQueryInformationThread".as_ptr().cast()) else {
            return "err no NtQueryInformationThread".into();
        };
        let query: Query = std::mem::transmute(query);
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        let pid = GetCurrentProcessId();
        let mut te: THREADENTRY32 = std::mem::zeroed();
        te.dwSize = u32::try_from(size_of::<THREADENTRY32>()).unwrap_or(0);
        let base = crate::GAME_BASE.load(Ordering::Relaxed);
        let mut out = Vec::new();
        let mut ok = Thread32First(snap, &raw mut te) != 0;
        while ok {
            if te.th32OwnerProcessID == pid {
                let h = OpenThread(THREAD_QUERY_INFORMATION, 0, te.th32ThreadID);
                let mut start: usize = 0;
                // ThreadQuerySetWin32StartAddress = 9
                query(
                    h as isize,
                    9,
                    (&raw mut start).cast(),
                    4,
                    std::ptr::null_mut(),
                );
                CloseHandle(h);
                let at = if start >= base && start < base + crate::SIZE_OF_IMAGE as usize {
                    format!("{:08x}", start - base + crate::IMAGE_BASE as usize)
                } else {
                    format!("{start:#x}")
                };
                out.push(format!("{}:{at}", te.th32ThreadID));
            }
            ok = Thread32Next(snap, &raw mut te) != 0;
        }
        CloseHandle(snap);
        format!("ok {}", out.join(" "))
    }
}

/// Runs a job on the game thread and waits for its reply.
fn on_game_thread(job: Job) -> String {
    if PRESENT.load(Ordering::Acquire) == 0 {
        return "err the game has not presented a frame yet".into();
    }
    let (tx, rx) = channel();
    if let Ok(mut jobs) = JOBS.lock() {
        jobs.push_back((job, tx));
    }
    rx.recv().unwrap_or_else(|_| "err no reply".into())
}

#[allow(clippy::too_many_lines)] // one arm per command
fn command(line: &str) -> Result<String, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let arg = |i: usize| {
        words
            .get(i)
            .copied()
            .ok_or_else(|| format!("{}: missing argument {i}", words[0]))
    };
    let now = || FRAME.load(Ordering::Acquire);
    Ok(match words.first().copied().unwrap_or("") {
        "ping" => "ok pong".into(),
        "frame" => format!("ok {}", now()),
        "stats" => format!(
            "ok frame {} pad_reads {} key_polls {}",
            now(),
            PAD_READS.load(Ordering::Relaxed),
            KEY_POLLS.load(Ordering::Relaxed)
        ),
        "keys" => {
            let codes = words[1..]
                .iter()
                .map(|w| parse_u8(w))
                .collect::<Result<Vec<_>, _>>()?;
            set_keys(&codes);
            "ok".into()
        }
        "key" => {
            let codes = arg(1)?
                .split(',')
                .map(parse_u8)
                .collect::<Result<Vec<_>, _>>()?;
            let hold = parse_u32(arg(2)?)?;
            let gap = words.get(3).map_or(Ok(hold), |w| parse_u32(w))?;
            set_keys(&codes);
            let release = now() + hold;
            ALLOWED.fetch_max(release + gap, Ordering::AcqRel);
            on_game_thread(Job::Release(release, release + gap))
        }
        "wait" => {
            let until = now() + parse_u32(arg(1)?)?;
            ALLOWED.fetch_max(until, Ordering::AcqRel);
            on_game_thread(Job::WaitUntil(until))
        }
        "waitfor" => {
            // waitfor <range> <value hex> [mask hex] [frames]: advances (in
            // lockstep, frame by frame) until the bytes match
            let r = parse_trace_range(arg(1)?)?;
            let hexbytes = |t: &str| -> Result<Vec<u8>, String> {
                (0..t.len() / 2)
                    .map(|i| {
                        u8::from_str_radix(&t[2 * i..2 * i + 2], 16).map_err(|e| e.to_string())
                    })
                    .collect()
            };
            let value = hexbytes(arg(2)?)?;
            let mask = words
                .get(3)
                .map_or_else(|| Ok(vec![0xff; value.len()]), |m| hexbytes(m))?;
            let limit = now() + words.get(4).map_or(Ok(600), |w| parse_u32(w))?;
            if value.len() != r.len || mask.len() != r.len {
                return Err("value and mask must be as long as the range".into());
            }
            // let lockstep run up to the limit; the job answers as soon as
            // the bytes match, and the script's next command holds again
            ALLOWED.fetch_max(limit, Ordering::AcqRel);
            on_game_thread(Job::WaitFor(r, value, mask, limit))
        }
        "lockstep" => {
            let on = arg(1)? == "on";
            ALLOWED.store(now(), Ordering::Release);
            LOCKSTEP.store(on, Ordering::Release);
            format!("ok {}", now())
        }
        "read" => {
            let at = crate::rebase(parse_u32(arg(1)?)?);
            format!(
                "ok {}",
                hex(&read_memory(at, parse_u32(arg(2)?)? as usize)?)
            )
        }
        "readabs" => {
            let at = parse_u32(arg(1)?)? as usize;
            format!(
                "ok {}",
                hex(&read_memory(at, parse_u32(arg(2)?)? as usize)?)
            )
        }
        "readp" => {
            let ptr = read_memory(crate::rebase(parse_u32(arg(1)?)?), 4)?;
            let base = u32::from_le_bytes(ptr.try_into().expect("4 bytes")) as usize;
            let at = base + parse_u32(arg(2)?)? as usize;
            format!(
                "ok {}",
                hex(&read_memory(at, parse_u32(arg(3)?)? as usize)?)
            )
        }
        "write" | "writep" => {
            // writep <addr> <offset> <hex>: at *addr + offset, like readp
            let at = crate::rebase(parse_u32(arg(1)?)?);
            let (at, text) = if words[0] == "writep" {
                let ptr = read_memory(at, 4)?;
                let base = u32::from_le_bytes(ptr.try_into().expect("4 bytes")) as usize;
                (base + parse_u32(arg(2)?)? as usize, arg(3)?)
            } else {
                (at, arg(2)?)
            };
            let bytes = (0..text.len() / 2)
                .map(|i| u8::from_str_radix(&text[2 * i..2 * i + 2], 16).map_err(|e| e.to_string()))
                .collect::<Result<Vec<u8>, String>>()?;
            read_memory(at, bytes.len())?;
            // SAFETY: the range was just read successfully; the test owns
            // the consequences of the write.
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), at as *mut u8, bytes.len()) };
            "ok".into()
        }
        "pad" => {
            let buttons = words.get(1).map_or(Ok(0), |w| parse_buttons(w))?;
            BUTTONS.store(buttons, Ordering::Release);
            "ok".into()
        }
        "press" => {
            let buttons = parse_buttons(arg(1)?)?;
            let hold = parse_u32(arg(2)?)?;
            let gap = words.get(3).map_or(Ok(hold), |w| parse_u32(w))?;
            BUTTONS.store(buttons, Ordering::Release);
            let release = now() + hold;
            ALLOWED.fetch_max(release + gap, Ordering::AcqRel);
            on_game_thread(Job::Release(release, release + gap))
        }
        "call" => {
            let address = crate::rebase(parse_u32(arg(1)?)?);
            let args = words[2..]
                .iter()
                .map(|w| parse_u32(w))
                .collect::<Result<Vec<_>, _>>()?;
            if args.len() > 8 {
                return Err("at most 8 arguments".into());
            }
            on_game_thread(Job::Call(address, args))
        }
        "shot" => on_game_thread(Job::Shot(words[1..].join(" "))),
        "snapshot" => on_game_thread(Job::Snapshot(words[1..].join(" "))),
        "fpucw" => on_game_thread(Job::FpuControl),
        "loadsave" => on_game_thread(Job::LoadSave(words[1..].join(" "))),
        "threads" => list_threads(),
        "calllog" => {
            // calllog off: flush and close
            if let Ok(mut log) = CALL_LOG.lock()
                && let Some(mut w) = log.take()
            {
                use std::io::Write as _;
                let _ = w.flush();
            }
            CALL_LOG_ON.store(false, Ordering::Release);
            "ok".into()
        }
        "trace" => {
            // trace off | trace <windows path> <range>...
            let mut t = TRACE
                .lock()
                .map_err(|_| "trace lock poisoned".to_string())?;
            if arg(1)? == "off" {
                if let Some(mut old) = t.take() {
                    use std::io::Write as _;
                    old.out.flush().map_err(|e| e.to_string())?;
                }
                "ok".into()
            } else {
                let ranges = words[2..]
                    .iter()
                    .map(|w| parse_trace_range(w))
                    .collect::<Result<Vec<_>, _>>()?;
                let file = std::fs::File::create(arg(1)?).map_err(|e| e.to_string())?;
                *t = Some(Trace {
                    out: std::io::BufWriter::new(file),
                    ranges,
                });
                "ok".into()
            }
        }
        "gfxcapture" => crate::gfx::capture(&words[1..].join(" ")),
        "overlay" => format!("ok {}", crate::gfx::live::toggle_overlay()),
        "editor" => {
            crate::gfx::live::toggle_editor();
            "ok".into()
        }
        "overlaykey" => {
            let vk = u32::from_str_radix(arg(1)?.trim_start_matches("0x"), 16)
                .map_err(|e| format!("{e}"))?;
            crate::gfx::live::overlay_key(vk, arg(2)? == "down");
            "ok".into()
        }
        "overlaymouse" => {
            let x: f32 = arg(1)?.parse().map_err(|e| format!("{e}"))?;
            let y: f32 = arg(2)?.parse().map_err(|e| format!("{e}"))?;
            let button = match words.get(3).copied() {
                Some("down") => Some(true),
                Some("up") => Some(false),
                _ => None,
            };
            crate::gfx::live::mouse(x, y, button);
            "ok".into()
        }
        "overlayclick" => {
            let x: f32 = arg(1)?.parse().map_err(|e| format!("{e}"))?;
            let y: f32 = arg(2)?.parse().map_err(|e| format!("{e}"))?;
            crate::gfx::live::click(x, y);
            "ok".into()
        }
        "gfxstats" => {
            let (frames, draws) = crate::gfx::stats();
            let (sent, images, misses) = crate::gfx::live::stats();
            format!(
                "ok frames {frames} draws {draws} live sent {sent} images {images} misses {misses}"
            )
        }
        "events" => {
            let events = EVENTS
                .lock()
                .map(|mut e| std::mem::take(&mut *e))
                .unwrap_or_default();
            let mut out = format!("ok {}", events.len());
            for e in events {
                out.push('\n');
                out.push_str(&e);
            }
            out
        }
        "sleeps" => {
            let log = SLEEP_LOG
                .lock()
                .map(|mut l| std::mem::take(&mut *l))
                .unwrap_or_default();
            let base = crate::GAME_BASE.load(Ordering::Acquire);
            let mut counts: std::collections::BTreeMap<(usize, u32), (u32, u32, u32)> =
                std::collections::BTreeMap::new();
            for (caller, ms, frame) in log {
                let e = counts
                    .entry((caller.wrapping_sub(base) + crate::IMAGE_BASE as usize, ms))
                    .or_insert((0, frame, frame));
                e.0 += 1;
                e.2 = frame;
            }
            let mut out = String::from("ok");
            for ((c, ms), (n, f0, f1)) in counts {
                use std::fmt::Write as _;
                let _ = write!(out, " {c:#x}:{ms}ms x{n} frames {f0}-{f1};");
            }
            out
        }
        "quit" => {
            log("  harness: quit");
            // Not process::exit: running the game's shutdown from this
            // thread can deadlock against its own threads.
            // SAFETY: ends this process at once.
            unsafe {
                windows_sys::Win32::System::Threading::TerminateProcess(
                    windows_sys::Win32::System::Threading::GetCurrentProcess(),
                    0,
                );
            }
            "ok".into()
        }
        other => return Err(format!("unknown command {other:?}")),
    })
}

fn serve(port: u16) {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            log(&format!("  harness: cannot listen on port {port}: {e}"));
            return;
        }
    };
    log(&format!("  harness: listening on 127.0.0.1:{port}"));
    for stream in listener.incoming().flatten() {
        let Ok(mut out) = stream.try_clone() else {
            continue;
        };
        for line in BufReader::new(stream).lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let reply = command(&line).unwrap_or_else(|e| format!("err {e}"));
            if writeln!(out, "{reply}").is_err() {
                break;
            }
        }
    }
}

/// Turns the harness on when `FFX_HARNESS` names a port.
///
/// # Safety
/// `module` must be the main module, before other threads run game code.
#[allow(clippy::too_many_lines)] // one patch after the other
pub unsafe fn install(module: usize) {
    static ONCE: OnceLock<()> = OnceLock::new();
    let Some(port) = std::env::var("FFX_HARNESS")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return;
    };
    if ONCE.set(()).is_err() {
        return;
    }
    ENABLED.store(true, Ordering::Release);
    BACKGROUND.store(
        std::env::var("FFX_HARNESS_BACKGROUND").is_ok_and(|v| v == "1"),
        Ordering::Release,
    );
    if std::env::var("FFX_HARNESS_FAST").is_ok_and(|v| v == "1") {
        FAST.store(true, Ordering::Release);
        VIRTUAL.store(
            std::env::var("FFX_HARNESS_VIRTUAL_CLOCK").is_ok_and(|v| v == "1"),
            Ordering::Release,
        );
        if VIRTUAL.load(Ordering::Acquire) {
            let hook = |name, address, prologue, replacement| crate::Hook {
                name,
                address,
                prologue,
                replacement,
                original: None,
                steal: 0,
            };
            let hooks = [
                // push ebp; mov ebp, esp; push esi
                hook(
                    "FileSys_SubmitRead",
                    0x0061_c220,
                    &[0x55, 0x8b, 0xec, 0x56][..],
                    sync_submit_read as *const () as usize,
                ),
                // push ebp; mov ebp, esp; sub esp, 0x68
                hook(
                    "createVsyncEmuThread",
                    0x0067_9e60,
                    &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x68][..],
                    virtual_vsync_start as *const () as usize,
                ),
                hook(
                    "endVsyncEmuThread",
                    0x0067_9f20,
                    &[][..],
                    virtual_vsync_end as *const () as usize,
                ),
                hook(
                    "VsyncEmu_Wake",
                    0x0067_9f80,
                    &[][..],
                    virtual_vsync_wake as *const () as usize,
                ),
            ];
            // SAFETY: the game has not started loading yet (we are in its
            // start-up, before the main loop).
            let r = unsafe { crate::install(module, &hooks, &[]) };
            log(&format!(
                "  harness: virtual clock: synchronous reads, vsync on the game thread {r:?}"
            ));
        }
        // SAFETY: forwarded from the caller.
        let patched = unsafe { patch_frame_limiter(module) && patch_boot_limiter(module) };
        log(&format!(
            "  harness: fast mode {}",
            if patched {
                "on"
            } else {
                "not available (unexpected code)"
            }
        ));
    }
    if let Ok(path) = std::env::var("FFX_HARNESS_CALL_LOG")
        && let Ok(f) = std::fs::File::create(&path)
        && let Ok(mut log) = CALL_LOG.lock()
    {
        *log = Some(std::io::BufWriter::new(f));
        CALL_LOG_ON.store(true, Ordering::Release);
    }
    if let Some(seed) = std::env::var("FFX_HARNESS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        let _ = SEED.set(seed);
    }
    if let Ok(home) = std::env::var("FFX_HARNESS_HOME") {
        let wide: Vec<u16> = home
            .trim_end_matches('\\')
            .encode_utf16()
            .chain([0])
            .collect();
        // MAX_PATH, with room for the game's own subfolders.
        if wide.len() < 160 {
            log(&format!("  harness: Documents folder is {home}"));
            let _ = HOME.set(wide);
        } else {
            log("  harness: FFX_HARNESS_HOME is too long; ignored");
        }
    }
    let _ = MUTEX_NAME.set(
        format!("{INSTANCE_MUTEX}-harness-{port}")
            .encode_utf16()
            .chain([0])
            .collect(),
    );
    // SAFETY: forwarded from the caller.
    unsafe {
        for (dll, function, replacement, slot) in [
            (
                "KERNEL32.dll",
                "CreateMutexW",
                create_mutex_w as *const () as usize,
                &CREATE_MUTEX_W,
            ),
            (
                "d3d11.dll",
                "D3D11CreateDeviceAndSwapChain",
                create_device_and_swap_chain as *const () as usize,
                &CREATE_DEVICE_AND_SWAP_CHAIN,
            ),
            (
                "USER32.dll",
                "ShowWindow",
                show_window as *const () as usize,
                &SHOW_WINDOW,
            ),
        ]
        .into_iter()
        .chain(HOME.get().map(|_| {
            (
                "SHELL32.dll",
                "SHGetFolderPathW",
                sh_get_folder_path as *const () as usize,
                &SH_GET_FOLDER_PATH,
            )
        }))
        .chain(std::env::var("FFX_HARNESS_TRACE_SLEEP").is_ok().then_some((
            "KERNEL32.dll",
            "Sleep",
            traced_sleep as *const () as usize,
            &SLEEP_ORIGINAL,
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "USER32.dll",
            "MessageBoxA",
            message_box_a as *const () as usize,
            &MESSAGE_BOX_A,
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "USER32.dll",
            "MessageBoxW",
            message_box_w as *const () as usize,
            &MESSAGE_BOX_W,
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "XINPUT9_1_0.dll",
            "XInputGetState",
            xinput_get_state as *const () as usize,
            &XINPUT_GET_STATE,
        ))) {
            match crate::iat::patch(module, dll, function, replacement) {
                Some(original) => {
                    slot.store(original, Ordering::Release);
                    log(&format!("  harness: {function} patched"));
                }
                None => log(&format!("  harness: {function} not imported")),
            }
        }
    }
    std::thread::spawn(move || serve(port));
}
