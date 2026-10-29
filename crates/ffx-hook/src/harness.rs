//! Test harness: lets a test drive the running game over TCP.
//!
//! Enabled by `FFX_HARNESS=<port>` (listening on `127.0.0.1`). It then:
//!
//! - makes the keys bound to the PS2 buttons the test holds read as held
//!   (`Input_KeyHeld`, `0x00629d50`), so the game and its Flash menus see
//!   the buttons as if pressed on the keyboard;
//! - wraps the game's update (`FFXApplication_animate`, `0x0042f520`) to
//!   count frames and, on the main thread between two updates, sample the
//!   trace and run commands (calls, waits, snapshots); lockstep holds the
//!   main thread there ([`animate`]);
//! - patches the swap chain's `Present` (from `D3D11CreateDeviceAndSwapChain`)
//!   to take screenshots on the render thread, of the image of a known
//!   update ([`flip`], [`present_flip`]: each flip's update);
//! - gives each cloth and middle-calc job (`Ch_CalcMiddle_Batch`: mouths, eye
//!   blinks) a seed of its own for the CRT `rand()` ([`seeded_job`]), so they repeat;
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
//!   a fixed start ([`VIRTUAL_START`]) plus the skipped sleeps only, never real time, so every
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
//!   are then ignored, so typing in other windows does not reach the game,
//!   and the mouse pointer the game reads (`GetCursorPos`) is far outside
//!   its window unless a test sets one (`cursor`), so the user's mouse
//!   does not either (`SetCursorPos` and `ClipCursor` do nothing then).
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
//! | `call <addr> [args…]` | `ok <eax>`: a cdecl call on the main thread, between two updates |
//! | `shot <windows path>` | `ok <w> <h> update <n>`: saves the back buffer as a 32-bit BMP, the image of update `n` (in lockstep the one the main thread is held after; see `docs/harness.md`, "Frames") |
//! | `shotwhen <range> <value hex> <mask hex> <frames> <windows path>` | `ok <w> <h> update <n>`: as `waitfor`, then the image of exactly the update `n` after which the bytes first matched, in real time too (`docs/harness.md`, boot) |
//! | `loadsave <slot\|windows path> [lockstep]` | `ok <file> map <n> entrance <n> …`: starts the game from that save file without the title menu ([`crate::loadsave`]); with `lockstep`, lockstep holds the main thread at the boundary the load was made at, so the map's load runs in lockstep from its first update (`docs/harness.md`, "Lockstep from the load") |
//! | `events` | `ok <n>` then `n` lines: files opened since the last call |
//! | `gfxcapture <windows dir> [hash] [frames N] [now] [async]` | `ok <n> draws`: the next frame's D3D11 calls (`gfx`, needs `FFX_GFX=1`); `now` in lockstep holds exactly the updates after the boundary the main thread is held at, and fails if a frame shows another (`docs/renderer.md`, "Reference captures") |
//! | `gfxreplay <capture.ffxgfx> [out dir]` | the capture's scene drawn by `ffx-draw` in place of the game's in the next frame, compared with the capture (`gfx::replay`, `docs/draw.md`) |
//! | `rustmap [on\|off]` | the Rust map path's statistics (`gfx::scene`, needs `FFX_RUST_MAP=1`), or turns it on or off from the next map pass |
//! | `gfxstats` | `ok frames <n> draws <n>`: draws in the last frame |
//! | `cursor <x> <y>` / `cursor off` | background mode: the mouse pointer the game reads, in client pixels, or none (far outside the window, the default) |
//! | `overlay` | shows or hides the live renderer's overlay (as F10): `ok <shown>` |
//! | `overlayclick <x> <y>` | a left click in the overlay, in back buffer pixels |
//! | `overlaymouse <x> <y> [down\|up]` | moves the overlay's pointer there, pressing or releasing the left button |
//! | `overlaykey <vk hex> down\|up` | a key for the overlay (Windows virtual-key code) |
//! | `editor` | opens or closes the renderer's editor (as F11; shows the overlay) |
//! | `audiomark <name>` | `ok frame <n> sample <n> updates <n> free <n> late <n>`: notes where the `FFX_AUDIO_WAV` capture stands in its marks file ([`crate::audio_wav`]; `ok off` without it; with no file on the virtual clock only the sample clock, no marks file) |
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

/// The frame count of the harness (0 when it is off).
pub fn frame() -> u32 {
    FRAME.load(Ordering::Relaxed)
}
static CREATE_DEVICE_AND_SWAP_CHAIN: AtomicUsize = AtomicUsize::new(0);
static SHOW_WINDOW: AtomicUsize = AtomicUsize::new(0);
static XINPUT_GET_STATE: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_BOX_A: AtomicUsize = AtomicUsize::new(0);
static MESSAGE_BOX_W: AtomicUsize = AtomicUsize::new(0);
static SH_GET_FOLDER_PATH: AtomicUsize = AtomicUsize::new(0);
static CREATE_MUTEX_W: AtomicUsize = AtomicUsize::new(0);
static GET_CURSOR_POS: AtomicUsize = AtomicUsize::new(0);
static SET_CURSOR_POS: AtomicUsize = AtomicUsize::new(0);
static CLIP_CURSOR: AtomicUsize = AtomicUsize::new(0);
/// The mouse pointer the game sees in background mode ([`get_cursor_pos`]):
/// [`NO_CURSOR`], or client coordinates set by `cursor <x> <y>`, `x` in
/// the high half.
static CURSOR: AtomicU64 = AtomicU64::new(NO_CURSOR);
const NO_CURSOR: u64 = u64::MAX;
/// `g_hWnd`, the game's window.
const G_HWND: u32 = 0x00cc_9ce8;
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
/// Where the virtual clock starts, in seconds. The real clock
/// (`Phyre_PTimerWin32_GetTime`: FILETIME / 1e7, about 1.34e10 s, where an
/// `f64` step is 1.9e-6 s) differed in every run, so the game's time
/// differences (the shaders' `scene.GlobalTime`) took other roundings:
/// three values 8 `f32` ulp apart in six draw captures
/// (`docs/drawdiff.md`, "Noise in cdsp0000"). A fixed power of two (about
/// 12 days; a step of 2.3e-10 s) gives every run the same times.
const VIRTUAL_START: f64 = 1_048_576.0;
/// Frames advance only up to [`ALLOWED`] (`lockstep on`).
static LOCKSTEP: AtomicBool = AtomicBool::new(false);
/// The last frame the game may present in lockstep.
static ALLOWED: AtomicU32 = AtomicU32::new(0);
/// `lockstep on` asked for while the game runs free: the main thread turns
/// it on at the next boundary it enters ([`animate`]) and holds there.
/// Set from the command thread at once, the hold fell on the boundary the
/// main thread was in when the command came (if it had not reached the
/// hold check yet) or on the next one: in about one run in twenty the
/// first, and what that boundary had run under (a free update, no loader
/// turn) changed battle-attack's FMOD capture from its first sample (two
/// of 42 runs, original code alike; docs/harness.md, "Audio capture").
static LOCKSTEP_REQUEST: AtomicBool = AtomicBool::new(false);
/// Seconds the frame limiter skipped, as `f64` bits.
static SKIPPED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The PS2 vsync handler `createVsyncEmuThread` (`0x00679e60`) was given,
/// on the virtual clock: run from `Present` instead of a real-time thread.
static VSYNC_CALLBACK: AtomicUsize = AtomicUsize::new(0);
/// Vsync ticks run so far (1/59.94 s of game time each).
static VSYNC_TICKS: AtomicU64 = AtomicU64::new(0);

/// `createVsyncEmuThread(callback)` on the virtual clock: no thread (which
/// would call `callback` every 16 ms of real time); [`run_vsync`] calls it
/// at the main thread's frame boundary as game time passes ([`animate`]; before the first update, from `Present`).
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
/// and its decompression on the calling thread at once, as `BigFileStream_readVFile`
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
        return VIRTUAL_START + skipped;
    }
    // SAFETY: the trampoline runs the original, which returns in st(0).
    let now = unsafe {
        let original: unsafe extern "C" fn() -> f64 =
            std::mem::transmute(TIME_NOW_ORIGINAL.load(Ordering::Acquire));
        original()
    };
    now + skipped
}

/// Moves the clock on by `seconds`.
fn skip(seconds: f64) {
    let mut bits = SKIPPED.load(Ordering::Acquire);
    loop {
        let next = (f64::from_bits(bits) + seconds).to_bits();
        match SKIPPED.compare_exchange(bits, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(b) => bits = b,
        }
    }
}

/// The frame limiter's `Sleep(ms)` in fast mode: no wait, the clock moves.
///
/// On the virtual clock it moves to the next frame's time exactly. The
/// limiter (`Game_Main` `0x0062f2a0`..`0x0062f3bb`) sleeps the remaining
/// time chopped to whole ms (`fistp` under control word `| 0xc00`), then
/// 0 ms (made 1 here: nothing else moves this clock), so the clock stood
/// 0.3-1 ms past the frame's time in a pattern that depends on how many
/// frames have run; time differences such as the shaders'
/// `scene.GlobalTime` then came out 1 ms apart between runs whose loads
/// took other frame counts (`docs/drawdiff.md`, "Noise in cdsp0000"). The
/// frame's time is the limiter's: the start (`0x00c90278`) plus the last
/// frame's (`0x00cc9d00`) plus `DAT_00c30e88` x 0.016683333333333335 s.
unsafe extern "system" fn fast_sleep(ms: u32) {
    if !VIRTUAL.load(Ordering::Relaxed) {
        skip(f64::from(ms) / 1000.0);
        return;
    }
    // SAFETY: the limiter's own globals in the image's data, written only
    // by this (the main) thread.
    let (start, last, count) = unsafe {
        (
            *(crate::rebase(0x00c9_0278) as *const f64),
            *(crate::rebase(0x00cc_9d00) as *const f64),
            *(crate::rebase(0x00c3_0e88) as *const u32),
        )
    };
    let due = start + last + f64::from(count) * 0.016_683_333_333_333_335;
    let now = VIRTUAL_START + f64::from_bits(SKIPPED.load(Ordering::Acquire));
    // The limiter compares in 80 bits, so step a little past the frame's
    // time (1e-9 s: below an f32 step of the game's seconds); a clock
    // already there (the comparison still came out short) moves the same.
    let ahead = (due - now).max(0.0) + 1e-9;
    if ahead < 1.0 {
        skip(ahead);
    } else {
        // Not the frame's wait (the globals are not set up yet).
        skip(f64::from(ms.max(1)) / 1000.0);
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
    // On the virtual clock nothing else moves time: a Sleep(0) in a wait
    // loop would spin for ever.
    let ms = if VIRTUAL.load(Ordering::Relaxed) {
        ms.max(1)
    } else {
        ms
    };
    skip(f64::from(ms) / 1000.0);
}

/// Set once the start-up loop's sleep is patched ([`patch_boot_limiter`]).
static BOOT_SLEEP_PATCHED: AtomicBool = AtomicBool::new(false);

/// Where the Rust `PhyreScene_renderOpeningScreen` sends its 33 ms sleep
/// in fast mode: the same `fast_sleep_cdecl` the original's patched call
/// (`0x0066b345`) reaches, or `None` (sleep for real) when fast mode is off.
/// Without this the port slept in real time on the logos and left the
/// clock where it was (2026-10-26, replay-flakes).
#[must_use]
pub fn boot_sleep() -> Option<u32> {
    BOOT_SLEEP_PATCHED
        .load(Ordering::Acquire)
        .then_some(fast_sleep_cdecl as *const () as usize as u32)
}

/// Points the start-up loop's `call Time_Sleep` (`0x0066b345`, in
/// `PhyreScene_renderOpeningScreen` `0x0066b2e0`: 33 ms a frame during the
/// logos) at `fast_sleep_cdecl`; its Rust port asks [`boot_sleep`].
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
    BOOT_SLEEP_PATCHED.store(true, Ordering::Release);
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

/// MSVCR110's `_getptd`: the calling thread's CRT data, whose `+0x14` is
/// `rand`'s seed (`rand` `0x100a492a`: `_getptd`, `[eax+0x14] * 0x343fd +
/// 0x269ec3`; `srand` stores its argument there).
static GETPTD: AtomicUsize = AtomicUsize::new(0);
/// `rand`'s seed in the CRT's per-thread data.
const HOLDRAND: usize = 0x14;
/// Ctrl work 0's event frame count (`0x01325d94`) at the last frame
/// boundary, for [`seeded_job`].
static JOB_EVENTS: AtomicU32 = AtomicU32::new(0);
/// `Sg_FrameSync`'s (`0x00821e80`) carries, `f32`: the 60 Hz remainder
/// (`(carry + dt) x 60` is the update's due count) and the 30 Hz one
/// (`(carry + dt) x 30`, whole steps are the update's logic ticks).
const FRAME_SYNC_CARRY_60: u32 = 0x012f_b7f8;
const FRAME_SYNC_CARRY_30: u32 = 0x012f_b7fc;
/// `g_Vu0_R` (the emulated VU0 `R`, `f32` bits) and `g_Vu0_RSeeded`.
const VU0_R: u32 = 0x00c0_a000;
const VU0_R_SEEDED: u32 = 0x00c8_f728;
/// What [`pin_carried_state`] sets `g_Vu0_R` to: 1.0 with a mantissa that
/// has bits all over (the generator's state is `R`'s 23 mantissa bits).
const VU0_R_PINNED: u32 = 0x3f80_0000 | 0x0035_a5c3;
/// `g_AppStartTime` (`f64`), `g_FrameLimiterLast` (`f64`, the last frame's
/// time less the start) and `g_AtelPlayTimeLast` (`f32`, the game time the
/// play time was last counted from).
const APP_START_TIME: u32 = 0x00c9_0278;
const FRAME_LIMITER_LAST: u32 = 0x00cc_9d00;
const ATEL_PLAY_TIME_LAST: u32 = 0x0132_5b84;
/// The virtual clock (past [`VIRTUAL_START`]) and the game time
/// (`FFXApplication_getAppTime`: clock - start - paused) that
/// [`pin_carried_state`] sets: whole seconds, so the sums are exact, and
/// past what boot, title and load take.
const PINNED_CLOCK: f64 = 4096.0;
const PINNED_GAME_TIME: f64 = 64.0;
/// `FFXApplication`'s shader time (`f32`, `+0x3b0`): the scene-wide
/// parameter `GlobalTime`. `FFXApplication_updateGlobalTime_0042fa00`
/// (from `FFXApplication_animate`) adds each update's step to it, wraps it
/// to 0 at 2400 s and copies it to `+0x120`, where
/// `PSceneWideParametersKey_initialize` (`0x004fe940`, through the
/// renderer's context `app +0x50`, its `+0xd0`) takes it for the shaders.
/// A map load sets it to 0 (`0x0065d230`), so it carried the load
/// update's step, whose last bits follow the boot's length.
const APP_GLOBAL_TIME: usize = 0x3b0;
const APP_GLOBAL_TIME_COPY: usize = 0x120;
/// The title's water animation (the band at the top of the title,
/// `Water_00..69.dds`, 1024x256 BC3): `PhyreScene_drawMainMenuWaveEffect`
/// (`0x0065b5b0`, from `Sg_MainLoop` while `g_YnTest20Seen` is set) draws
/// frame `g_TitleWaterFrame` (`u8`) and steps it, mod 70, and counts
/// `g_TitleWaterDraws` (`u8`) up. Nothing else writes them, so a later
/// title showed the frame the free-running first visit left.
const TITLE_WATER_FRAME: u32 = 0x00cc_ba09;
const TITLE_WATER_DRAWS: u32 = 0x00cc_ba0a;
/// One frame of the limiter at two vsyncs a frame (`g_FrameVsyncs`), for
/// the step of the update the pin happens in.
const PINNED_FRAME: f64 = 2.0 * 0.016_683_333_333_333_335;
/// The application object `FFXApplication_animate` runs on (its `this`),
/// for [`pin_carried_state`]: `+0x20` the update's game time (`f32`),
/// `+0x24` its step, `+0x28` the paused time (`f32`), `+0x120` a copy of
/// `+0x20` (`Phyre_PFramework_PApplication_frame` `0x00627940`).
static APP: AtomicUsize = AtomicUsize::new(0);

/// The `pin` command, on the main thread between two updates: sets what
/// the frames run before `lockstep on` leave behind to fixed values, so
/// that a replay plays the same from there however many frames boot,
/// title and load took (docs/harness.md, "Pinned state"):
///
/// - the time. `Phyre_PFramework_PApplication_frame` keeps the game time
///   as an `f32` (`app +0x20`: clock - start - paused) and the step as the
///   difference of two such `f32`s (`+0x24`), so the step's last bits
///   depended on how far the clock had got, and `Sg_FrameSync` adds the
///   steps up (below). The virtual clock moves to [`PINNED_CLOCK`],
///   `g_AppStartTime` so that the game time is [`PINNED_GAME_TIME`], and
///   the paused time to 0; `g_FrameLimiterLast` (relative to the start),
///   `app +0x20`, the running update's step (`+0x24`, one frame) and the
///   play-time stamp follow, so every later step is one frame and the
///   limiter's next frame is one interval on;
/// - the shaders' `GlobalTime` ([`APP_GLOBAL_TIME`]): 0, as a map load
///   leaves it, so that it no longer carries the load update's step;
/// - `Sg_FrameSync`'s carries. The virtual clock's frame is two NTSC
///   fields (2 / 59.94 s), a little over the 1/30 s logic tick, so the
///   30 Hz carry grows by about 0.1% of a tick each update and about one
///   update in a thousand runs two logic ticks. Where that double tick
///   fell depended on the frames since start-up: battle-magic's effect
///   ran a tick ahead in some runs (twice the `g_Vu0_R` draws in one
///   update), original code and Rust alike;
/// - `g_Vu0_R` (and its seeded flag): the effects' generator (`op_random`,
///   the ppp rand programs), which the field's effects draw from before;
/// - the main thread's CRT `rand()` seed (`op_loop_ffx_after` draws from
///   it every update): 1, as at start-up;
/// - the title's water frame and its draw count ([`TITLE_WATER_FRAME`]):
///   0, as at start-up. The replays that go back to the title (a game
///   over) visit it once before, running free.
///
/// Returns what was pinned. The game's own generators (`0x00c42200`, the
/// 68 at `0x01135ee0`) need nothing: loading a save seeds them (`--seed`).
///
/// # Safety
/// On the main thread, between two updates (in `FFXApplication_animate`).
unsafe fn pin_carried_state() -> String {
    let mut pinned = vec!["frame sync", "vu0 R", "title water"];
    // SAFETY: words of the image's data and of the application object,
    // which only the main thread writes; `_getptd` returns this thread's
    // CRT data.
    unsafe {
        *(crate::rebase(FRAME_SYNC_CARRY_60) as *mut f32) = 0.0;
        *(crate::rebase(FRAME_SYNC_CARRY_30) as *mut f32) = 0.0;
        *(crate::rebase(VU0_R) as *mut u32) = VU0_R_PINNED;
        *(crate::rebase(VU0_R_SEEDED) as *mut u32) = 1;
        *(crate::rebase(TITLE_WATER_FRAME) as *mut u8) = 0;
        *(crate::rebase(TITLE_WATER_DRAWS) as *mut u8) = 0;
        let app = APP.load(Ordering::Acquire);
        if VIRTUAL.load(Ordering::Acquire) && app != 0 {
            // the paused time (summed while the game held its clock, which
            // varies with the load) goes into the start, so that the game
            // time's sums are exact
            *((app + 0x28) as *mut f32) = 0.0;
            let paused = 0.0;
            let now = VIRTUAL_START + PINNED_CLOCK;
            let start = now - PINNED_GAME_TIME;
            SKIPPED.store(PINNED_CLOCK.to_bits(), Ordering::Release);
            // the emulated vsyncs go on from the new time, not the jump
            VSYNC_TICKS.store(vsync_ticks_now(), Ordering::Release);
            *(crate::rebase(APP_START_TIME) as *mut f64) = start;
            // the running update's frame started now: the limiter's next
            // frame is due one interval on
            *(crate::rebase(FRAME_LIMITER_LAST) as *mut f64) = now - start;
            #[allow(clippy::cast_possible_truncation)] // the game's f32s
            let game_time = (now - start - paused) as f32;
            *((app + 0x20) as *mut f32) = game_time;
            // the shader time starts again from 0, as at a map load; its
            // copy (which held it, not the game time, between updates) too
            *((app + APP_GLOBAL_TIME) as *mut f32) = 0.0;
            *((app + APP_GLOBAL_TIME_COPY) as *mut f32) = 0.0;
            // the running update's step was taken from the old clock: one
            // frame, as the next updates' (two f32 game times apart)
            #[allow(clippy::cast_possible_truncation)]
            let next = (now - start + PINNED_FRAME) as f32;
            *((app + 0x24) as *mut f32) = next - game_time;
            *(crate::rebase(ATEL_PLAY_TIME_LAST) as *mut f32) = game_time;
            pinned.push("time");
            pinned.push("global time");
        }
        let getptd = GETPTD.load(Ordering::Acquire);
        if getptd != 0 {
            let ptd = std::mem::transmute::<usize, unsafe extern "C" fn() -> usize>(getptd)();
            ((ptd + HOLDRAND) as *mut u32).write(1);
            pinned.push("main CRT seed");
        }
    }
    pinned.join(", ")
}

/// The `pin` job's reply ([`pin_carried_state`]); only [`run_main_jobs`]
/// calls it, on the main thread between two updates.
fn pin_reply() -> String {
    // SAFETY: see above.
    format!("ok {}", unsafe { pin_carried_state() })
}

/// Seeded jobs run (cloth, middle calc), for `stats`.
static SEEDED_JOBS: AtomicU32 = AtomicU32::new(0);

/// Runs one character job with the CRT `rand()` seeded for it, and gives
/// the worker its own seed back after. Two job kinds the scheduler runs on
/// whichever `JobWorker` thread is free draw from `rand()`, whose seed is
/// per thread, so each run drew other numbers: the cloth's wind
/// (`Sg_ClothStep` `0x00822de0`, under `Ch_Cloth_PerCharacter`
/// `0x0082e390`) and the mouth and the texture animations, the eyes'
/// blinks among them (`Ch_Mouth`, `chr_texanim_main` `0x0077c8d0`, under
/// `Ch_CalcMiddle_Batch` `0x00833530`). The seed comes from the job's kind,
/// its first character's place in the character array (`*0x023c44e4`,
/// 0x880 bytes a record), and ctrl work 0's event frame count (which the
/// replays pin their screenshots to) as it stood at the last frame boundary
/// ([`JOB_EVENTS`]: the main thread counts it on during the update, beside
/// the jobs). The game itself is unchanged: the wrappers exist only with
/// the harness, on the original code and the Rust replacements alike.
///
/// # Safety
/// On a job thread, with `first` a character record (or 0).
unsafe fn seeded_job(kind: u32, first: u32, job: impl FnOnce() -> u32) -> u32 {
    type GetPtd = unsafe extern "C" fn() -> usize;
    let getptd = GETPTD.load(Ordering::Acquire);
    if getptd == 0 || first == 0 {
        return job();
    }
    // SAFETY: the array global is the image's; `_getptd` returns this
    // thread's CRT data, created if needed.
    unsafe {
        let array = *(crate::rebase(0x023c_44e4) as *const u32);
        let place = first.wrapping_sub(array) / 0x880;
        let events = JOB_EVENTS.load(Ordering::Acquire);
        let ptd = std::mem::transmute::<usize, GetPtd>(getptd)();
        let seed = (ptd + HOLDRAND) as *mut u32;
        let own = seed.read();
        seed.write(place.wrapping_mul(0x9e37_79b1) ^ events.wrapping_mul(0x85eb_ca6b) ^ kind);
        let r = job();
        seed.write(own);
        SEEDED_JOBS.fetch_add(1, Ordering::Relaxed);
        r
    }
}

/// The job `JobSchedule_Ch_Cloth_PerCharacter` (`0x00a44b40`) queues in
/// place of `Ch_Cloth_PerCharacter` (`0x0082e390`, `cdecl(chars, count)`:
/// `count` character pointers at `chars`), seeded ([`seeded_job`]).
unsafe extern "C" fn cloth_job(chars: u32, count: u32) -> u32 {
    type Job = unsafe extern "C" fn(u32, u32) -> u32;
    // SAFETY: the job's own arguments, passed on to the job.
    unsafe {
        let job: Job = std::mem::transmute(crate::rebase(0x0082_e390));
        let first = if chars != 0 && count != 0 {
            *(chars as *const u32)
        } else {
            0
        };
        seeded_job(1, first, || job(chars, count))
    }
}

/// The job `JobSchedule_Ch_CalcMiddle_Batch` (`0x00a44b90`) queues in
/// place of `Ch_CalcMiddle_Batch` (`0x00833530`, `cdecl(chars, &pass,
/// &count)`: `*count` records from `chars` on), seeded ([`seeded_job`];
/// the pass in the kind).
unsafe extern "C" fn middle_job(chars: u32, pass: u32, count: u32) -> u32 {
    type Job = unsafe extern "C" fn(u32, u32, u32) -> u32;
    // SAFETY: the job's own arguments, passed on to the job; `pass` points
    // at the caller's word.
    unsafe {
        let job: Job = std::mem::transmute(crate::rebase(0x0083_3530));
        let kind = if pass == 0 {
            2
        } else {
            2 | (*(pass as *const u32) << 8)
        };
        seeded_job(kind, chars, || job(chars, pass, count))
    }
}

/// Points the `push <job>` at `site` (`68 imm32`, an `FFX.exe` address)
/// at `replacement` when it pushes `job`.
///
/// # Safety
/// `module` is the main module; the job is not queued yet.
unsafe fn patch_job_push(module: usize, site: u32, job: u32, replacement: usize) -> bool {
    use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
    let site = (module + (site - crate::IMAGE_BASE) as usize) as *mut u8;
    let job = module + (job - crate::IMAGE_BASE) as usize;
    // SAFETY: inside the mapped image; checked to be `68 <imm32 job>`.
    unsafe {
        let current = std::slice::from_raw_parts(site, 5);
        if current[0] != 0x68
            || u32::from_le_bytes(current[1..5].try_into().unwrap_or_default()) as usize != job
        {
            return false;
        }
        let mut old = 0;
        if VirtualProtect(site.cast(), 5, PAGE_EXECUTE_READWRITE, &raw mut old) == 0 {
            return false;
        }
        let target = u32::try_from(replacement).unwrap_or_default().to_le_bytes();
        site.add(1).copy_from_nonoverlapping(target.as_ptr(), 4);
        VirtualProtect(site.cast(), 5, old, &raw mut old);
    }
    true
}

/// Finds MSVCR110's `_getptd` and points the two job schedulers'
/// `push <job>` at the seeded wrappers: `JobSchedule_Ch_Cloth_PerCharacter`
/// at `0x00a44b51`, `JobSchedule_Ch_CalcMiddle_Batch` at `0x00a44ba4`.
///
/// # Safety
/// `module` is the main module; no character is drawn yet.
unsafe fn patch_seeded_jobs(module: usize) -> bool {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    // SAFETY: plain Win32 lookups; the patches as documented.
    unsafe {
        let crt = GetModuleHandleA(c"MSVCR110.dll".as_ptr().cast());
        let Some(getptd) = GetProcAddress(crt, c"_getptd".as_ptr().cast()) else {
            return false;
        };
        GETPTD.store(getptd as usize, Ordering::Release);
        patch_job_push(
            module,
            0x00a4_4b51,
            0x0082_e390,
            cloth_job as *const () as usize,
        ) & patch_job_push(
            module,
            0x00a4_4ba4,
            0x0083_3530,
            middle_job as *const () as usize,
        )
    }
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

/// Work for the main thread at a frame boundary ([`animate`]), or for the
/// render thread in `Present` (screenshots).
enum Job {
    /// Reply at this frame.
    WaitUntil(u32),
    /// Release the pad at this frame, then wait until the second.
    Release(u32, u32),
    Call(usize, Vec<u32>),
    /// The first presented image of update `.1` or a later one (render
    /// thread); `.2`: pinned (a `shotwhen`'s): exactly update `.1`, counted
    /// in [`PINNED`].
    Shot(String, u32, bool),
    Snapshot(String),
    FpuControl,
    /// Pin the state the frames before lockstep leave behind (`pin`,
    /// [`pin_carried_state`]).
    Pin,
    /// Start the game from a save (`loadsave`, [`crate::loadsave`]); `.1`:
    /// hold the main thread in lockstep at this boundary once the load is
    /// made (`loadsave <save> lockstep`).
    LoadSave(String, bool),
    /// Until the bytes at the range equal the value (under the mask), or
    /// the frame limit passes: `(range, value, mask, limit frame)`.
    WaitFor(TraceRange, Vec<u8>, Vec<u8>, u32),
    /// As `WaitFor`, then the image of the update the bytes first matched
    /// after (`shotwhen`): `(range, value, mask, limit frame, path, the
    /// first boundary it was checked at)`.
    ShotWhen(TraceRange, Vec<u8>, Vec<u8>, u32, String, Option<u32>),
    /// In lockstep: hold the main thread at this boundary (no update runs)
    /// until the bytes at the range equal the value (under the mask), or
    /// the real-time deadline passes (`holdfor`): `(range, value, mask,
    /// deadline)`. For state other threads make at their own speed (a
    /// movie's prebuffer), so that the update it is reached at does not
    /// follow real time.
    HoldFor(TraceRange, Vec<u8>, Vec<u8>, std::time::Instant),
}

/// `shotwhen` jobs and their pinned shots not yet taken. While there are
/// any, the present of update `n` waits until the main thread has left
/// boundary `n` (as in lockstep), so a shot the main thread asks for there
/// cannot miss that image.
static PINNED: AtomicU32 = AtomicU32::new(0);

/// The main thread's jobs, run at frame boundaries.
static JOBS: Mutex<VecDeque<(Job, Sender<String>)>> = Mutex::new(VecDeque::new());
/// `callnext` calls, run by the main thread at the start of its next
/// update ([`run_update_calls`]): `(address, args, reply)`.
#[allow(clippy::type_complexity)] // one queue of plain tuples
static UPDATE_CALLS: Mutex<VecDeque<(usize, Vec<u32>, Sender<String>)>> =
    Mutex::new(VecDeque::new());

/// Runs the `callnext` calls (main thread, in `animate` after the boundary
/// is left and the FMOD turn is done, just before the update). A `call`
/// job runs while the main thread is held at its boundary, where in
/// lockstep the render thread holds the render lock in `Present` until the
/// boundary is left: a called function that takes that lock (a movie's
/// close unbinds its textures: `graphicDestroyFmv`) waited for it forever.
fn run_update_calls() {
    let due: Vec<_> = UPDATE_CALLS
        .lock()
        .map_or_else(|_| Vec::new(), |mut calls| calls.drain(..).collect());
    for (address, args, reply) in due {
        // SAFETY: the test asked for this call, on the main thread at the
        // start of an update.
        let eax = unsafe { call(address, &args) };
        let _ = reply.send(format!("ok {eax:#x}"));
    }
}
/// The render thread's jobs, run in `Present`.
static RENDER_JOBS: Mutex<VecDeque<(Job, Sender<String>)>> = Mutex::new(VecDeque::new());

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

/// Replaces `GetCursorPos` in background mode. The game's only caller,
/// the mouse device's update (`Phyre_PFramework_PInputDeviceMouse_update`,
/// `0x0062d6f0`), turns the desktop position into a window position, and
/// the PC menu (Iggy) moves its cursor to the entry under it: the user's
/// own mouse, resting over a background game's window, chose menu entries
/// and broke iggy-menus' key sequence (`docs/harness.md`, "The mouse in
/// background games"). The game sees a pointer far outside its window, or
/// the test's (`cursor <x> <y>`).
unsafe extern "system" fn get_cursor_pos(point: *mut windows_sys::Win32::Foundation::POINT) -> i32 {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    if point.is_null() {
        return 0;
    }
    let cursor = CURSOR.load(Ordering::Acquire);
    let mut at = POINT {
        x: -32000,
        y: -32000,
    };
    if cursor != NO_CURSOR {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // packed i32 halves
        let client = POINT {
            x: (cursor >> 32) as u32 as i32,
            y: cursor as u32 as i32,
        };
        // SAFETY: g_hWnd is a word of the game's image; the window it
        // names (or 0 before it exists) is only read.
        unsafe {
            let hwnd = *(crate::rebase(G_HWND) as *const *mut c_void);
            let mut p = client;
            if !hwnd.is_null() && ClientToScreen(hwnd, &raw mut p) != 0 {
                at = p;
            }
        }
    }
    // SAFETY: the caller's POINT.
    unsafe { *point = at };
    1
}

/// Replaces `SetCursorPos` in background mode (the debug free camera
/// centres the pointer): the user's mouse stays where it is.
unsafe extern "system" fn set_cursor_pos(_x: i32, _y: i32) -> i32 {
    1
}

/// Replaces `ClipCursor` in background mode (the mouse device clips the
/// pointer to the window in full screen): the user's mouse stays free.
unsafe extern "system" fn clip_cursor(_rect: *const c_void) -> i32 {
    1
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

/// Runs the main thread's jobs due at the frame boundary `frame` (the
/// number of updates finished).
#[allow(clippy::too_many_lines)] // one arm per job kind
fn run_main_jobs(frame: u32) {
    let due: Vec<(Job, Sender<String>)> = JOBS.lock().map_or_else(
        |_| Vec::new(),
        |mut jobs| {
            let mut keep = VecDeque::new();
            let mut due = Vec::new();
            while let Some(mut job) = jobs.pop_front() {
                let ready = match job.0 {
                    Job::WaitUntil(f) | Job::Release(f, _) => frame >= f,
                    Job::Call(..)
                    | Job::Shot(..)
                    | Job::Snapshot(_)
                    | Job::FpuControl
                    | Job::Pin
                    | Job::LoadSave(..) => true,
                    Job::WaitFor(ref r, ref value, ref mask, limit) => {
                        frame >= limit || range_matches(r, value, mask)
                    }
                    Job::ShotWhen(ref r, ref value, ref mask, limit, _, ref mut first) => {
                        first.get_or_insert(frame);
                        frame >= limit || range_matches(r, value, mask)
                    }
                    Job::HoldFor(ref r, ref value, ref mask, deadline) => {
                        std::time::Instant::now() >= deadline || range_matches(r, value, mask)
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
                // SAFETY: the test asked for this call, on the main thread
                // between two updates.
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
                // hold here, before the next update runs
                if LOCKSTEP.load(Ordering::Acquire) {
                    ALLOWED.store(frame, Ordering::Release);
                }
                let _ = reply.send(if hit {
                    format!("ok {frame}")
                } else {
                    format!("err timeout at {frame}")
                });
            }
            Job::ShotWhen(r, value, mask, _, path, first) => {
                let hit = range_matches(&r, &value, &mask);
                let lockstep = LOCKSTEP.load(Ordering::Acquire);
                if lockstep {
                    ALLOWED.store(frame, Ordering::Release);
                }
                if !hit {
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                    let _ = reply.send(format!("err timeout at {frame}"));
                } else if !lockstep && first == Some(frame) {
                    // the present of this update may have run before the
                    // command came (no hold was asked for then)
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                    let _ = reply.send(format!(
                        "err matched at {frame}, the first frame it was checked: wait for an \
                         earlier state first"
                    ));
                } else if let Ok(mut jobs) = RENDER_JOBS.lock() {
                    // its present waits for this boundary to end (PINNED)
                    jobs.push_back((Job::Shot(path, frame, true), reply));
                } else {
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                }
            }
            Job::HoldFor(r, value, mask, _) => {
                let _ = reply.send(if range_matches(&r, &value, &mask) {
                    format!("ok {frame}")
                } else {
                    format!("err timeout at {frame}")
                });
            }
            Job::FpuControl => {
                let _ = reply.send(format!("ok {:#06x}", crate::snapshot::fpu_control()));
            }
            Job::Pin => drop(reply.send(pin_reply())),
            Job::LoadSave(spec, lockstep) => {
                let r = crate::loadsave::load(&spec);
                log(&format!("loadsave {spec} at frame {frame}: {r:?}"));
                // `loadsave <save> lockstep`: hold here, at the boundary the
                // load was made at, so the map's load runs in lockstep from
                // its first update. A `lockstep on` sent after the event
                // opened took hold 0 to 9 updates into the load, as fast as
                // the reply came back, and what the map's start-up ran free
                // moved later state (docs/harness.md, "Lockstep from the
                // load").
                if lockstep && r.is_ok() {
                    LOCKSTEP_REQUEST.store(false, Ordering::Release);
                    ALLOWED.store(frame, Ordering::Release);
                    LOCKSTEP.store(true, Ordering::Release);
                }
                let _ = reply.send(r.map_or_else(|e| format!("err {e}"), |s| format!("ok {s}")));
            }
            Job::Shot(..) => {
                let _ = reply.send("err a shot is a render-thread job".into());
            }
        }
    }
}

/// Runs the render thread's jobs (screenshots) that the image being
/// presented satisfies: it shows update `label` ([`NO_LABEL`]: unknown).
///
/// # Safety
/// Inside the swap chain's own `Present`, on the render thread.
unsafe fn run_render_jobs(label: u32, swap_chain: *mut c_void) {
    let due: Vec<(Job, Sender<String>)> = RENDER_JOBS.lock().map_or_else(
        |_| Vec::new(),
        |mut jobs| {
            let (due, keep): (VecDeque<_>, VecDeque<_>) = jobs.drain(..).partition(|j| match j.0 {
                Job::Shot(_, want, _) => label == NO_LABEL || label >= want,
                _ => true,
            });
            *jobs = keep;
            due.into_iter().collect()
        },
    );
    for (job, reply) in due {
        if let Job::Shot(path, want, pinned) = job {
            if pinned {
                PINNED.fetch_sub(1, Ordering::AcqRel);
                if label != want {
                    let shown = if label == NO_LABEL {
                        "an unknown update".to_owned()
                    } else {
                        format!("update {label}")
                    };
                    let _ = reply.send(format!(
                        "err update {want}'s image was missed (this one shows {shown})"
                    ));
                    continue;
                }
            }
            // SAFETY: the swap chain is live inside its own Present.
            let r = unsafe { screenshot(swap_chain, &path) };
            if r.is_ok() {
                crate::iggy_log::note_shot(&path, (label != NO_LABEL).then_some(label));
            }
            let _ = reply.send(r.map_or_else(
                |e| format!("err {e}"),
                |(w, h)| {
                    if label == NO_LABEL {
                        format!("ok {w} {h}")
                    } else {
                        format!("ok {w} {h} update {label}")
                    }
                },
            ));
        } else {
            let _ = reply.send("err not a render-thread job".into());
        }
    }
}

/// The label of a present whose flip the harness did not see.
const NO_LABEL: u32 = u32::MAX;
/// Frame boundaries the main thread has entered: the newest one's label
/// plus 1 (0: none yet).
static REACHED: AtomicU32 = AtomicU32::new(0);
/// Frame boundaries the main thread has left, as [`REACHED`].
static PASSED: AtomicU32 = AtomicU32::new(0);

/// Lockstep is on.
pub(crate) fn lockstep() -> bool {
    LOCKSTEP.load(Ordering::Acquire)
}

/// Frame boundaries left, as [`PASSED`].
pub(crate) fn passed() -> u32 {
    PASSED.load(Ordering::Acquire)
}

/// Frame boundaries entered, as [`REACHED`] (0: the main loop has not run
/// an update yet).
pub(crate) fn reached() -> u32 {
    REACHED.load(Ordering::Acquire)
}

/// The virtual clock is on.
pub(crate) fn virtual_clock() -> bool {
    VIRTUAL.load(Ordering::Acquire)
}

/// The present holds end as if the main thread had left this many
/// boundaries: the loader's turn lets the render thread go before the main
/// thread leaves ([`crate::loader_turn`]).
static PRESENT_RELEASED: AtomicU32 = AtomicU32::new(0);

/// The main thread stands at a frame boundary (entered, not left).
pub(crate) fn at_boundary() -> bool {
    REACHED.load(Ordering::Acquire) > PASSED.load(Ordering::Acquire)
}

/// Trampoline to the original `FFXApplication_animate`.
pub static ANIMATE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Trampoline to the original `Phyre_PRendering_PRenderer_flip`.
pub static FLIP_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Trampoline to the original `Renderer_PresentFlip`.
pub static PRESENT_FLIP_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Flips the main thread queued.
static FLIPS: AtomicU32 = AtomicU32::new(0);
/// Flip commands the render thread ran.
static FLIPS_RUN: AtomicU32 = AtomicU32::new(0);
/// The update each queued flip shows ([`FRAME`] when it was queued), by
/// flip number modulo the ring's size. The render thread runs at most 16
/// command buffers behind (`Phyre_PRendering_PRenderThread_threadEntry`'s
/// ring of 16 slots).
static FLIP_LABELS: [AtomicU32; 64] = [const { AtomicU32::new(NO_LABEL) }; 64];
/// The update the render thread's current flip shows (render thread only).
static PRESENT_LABEL: AtomicU32 = AtomicU32::new(NO_LABEL);
/// Presents so far, for `stats`.
static PRESENTS: AtomicU32 = AtomicU32::new(0);

/// The updates run so far: on the main thread after an update, the label
/// its flip gets (`gfx::capture` tags scene inputs with it).
pub fn update_label() -> u32 {
    FRAME.load(Ordering::Acquire)
}

/// The label of the update the render thread's current flip shows (render
/// thread, inside `Present`), when known.
pub fn present_label() -> Option<u32> {
    let l = PRESENT_LABEL.load(Ordering::Acquire);
    (l != NO_LABEL).then_some(l)
}
/// The update whose flip the render thread runs next (the image its
/// current commands draw into), when the main thread has queued it; for
/// the Iggy call log ([`crate::iggy_log`]).
pub fn render_label() -> Option<u32> {
    let run = FLIPS_RUN.load(Ordering::Acquire);
    (run < FLIPS.load(Ordering::Acquire))
        .then(|| FLIP_LABELS[run as usize % FLIP_LABELS.len()].load(Ordering::Acquire))
        .filter(|&l| l != NO_LABEL)
}

/// A present without a known label was seen after the main loop started.
static UNLABELLED: AtomicBool = AtomicBool::new(false);
/// The virtual vsync handler runs on one thread at a time.
static VSYNC_LOCK: Mutex<()> = Mutex::new(());

/// Wraps `FFXApplication_animate` (`0x0042f520`, `thiscall(app) -> int`),
/// the game's update: `Phyre_PFramework_PApplication_frame` (`0x00627940`)
/// calls it once a frame on the main thread, before the frame's render and
/// flip; it runs `updateFFX` (`0x00822840`: `Sg_MainLoop` `0x00820ae0`,
/// with `Sg_FrameSync` `0x00821e80` inside) unless the PC menu is up. Its
/// entry is the frame boundary: every update before it has finished, the
/// next has not begun, and the previous frame's flip is queued. The trace
/// samples here, the main thread's jobs run here, and lockstep holds the
/// game here.
///
/// # Safety
/// Called by the game, on the main thread.
pub unsafe extern "thiscall" fn animate(app: *mut c_void) -> u32 {
    let frame = FRAME.load(Ordering::Acquire);
    APP.store(app as usize, Ordering::Release);
    // `lockstep on` takes hold here, at the first boundary entered after
    // the command, before anything of this boundary has run
    if LOCKSTEP_REQUEST.swap(false, Ordering::AcqRel) {
        ALLOWED.store(frame, Ordering::Release);
        LOCKSTEP.store(true, Ordering::Release);
    }
    REACHED.store(frame + 1, Ordering::Release);
    note_clock_step(frame);
    trace_frame(frame);
    run_main_jobs(frame);
    // Lockstep: hold the game here until the script allows more updates,
    // still answering jobs (calls, snapshots, waits) meanwhile.
    while LOCKSTEP.load(Ordering::Acquire) && frame >= ALLOWED.load(Ordering::Acquire) {
        std::thread::sleep(std::time::Duration::from_millis(1));
        run_main_jobs(frame);
    }
    // The asset loader's turn at the render lock is decided here, while
    // the render thread still holds it (crate::loader_turn).
    if LOCKSTEP.load(Ordering::Acquire) {
        crate::loader_turn::boundary_turn(frame, || {
            PRESENT_RELEASED.store(frame + 1, Ordering::Release);
        });
    }
    PASSED.store(frame + 1, Ordering::Release);
    // SAFETY: a word of the image's data (ctrl work 0's event frame count),
    // which only this thread writes.
    JOB_EVENTS.store(
        unsafe { *(crate::rebase(0x0132_5d94) as *const u32) },
        Ordering::Release,
    );
    if VIRTUAL.load(Ordering::Acquire)
        && let Ok(_one) = VSYNC_LOCK.lock()
    {
        run_vsync();
    }
    // FFX_AUDIO_WAV: the FmodManager thread's tick for this boundary, at
    // this frame's game time (crate::audio_wav).
    crate::audio_wav::boundary_turn();
    run_update_calls();
    // The port of `FFXApplication::animate` (M5 `main`, ported/m5_main.rs)
    // runs here, behind its own name: this wrapper owns the address.
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let r = if crate::ported::ffx_application_animate_on() {
        crate::ported::ffx_application_animate(app as usize as u32)
    } else {
        // SAFETY: the trampoline runs the original with the same `this`.
        unsafe {
            let original: unsafe extern "thiscall" fn(*mut c_void) -> u32 =
                std::mem::transmute(ANIMATE_ORIGINAL.load(Ordering::Acquire));
            original(app)
        }
    };
    FRAME.store(frame + 1, Ordering::Release);
    r
}

/// The virtual clock at the last frame boundary (`f64` bits of
/// [`SKIPPED`]), for [`note_clock_step`].
static CLOCK_AT_BOUNDARY: AtomicU64 = AtomicU64::new(0);
/// Updates [`note_clock_step`] has logged.
static CLOCK_STEPS_LOGGED: AtomicU32 = AtomicU32::new(0);

/// Logs an update in lockstep over which the virtual clock moved by more
/// than one frame (`harness: the clock moved N s in update M`): only the
/// frame limiter should move it once the main loop runs, so anything more
/// is time some wait inside the update added (the first 40).
fn note_clock_step(frame: u32) {
    let now = SKIPPED.load(Ordering::Acquire);
    let before = CLOCK_AT_BOUNDARY.swap(now, Ordering::AcqRel);
    let step = f64::from_bits(now) - f64::from_bits(before);
    if VIRTUAL.load(Ordering::Acquire)
        && LOCKSTEP.load(Ordering::Acquire)
        && frame > 0
        && step > 1.5 * PINNED_FRAME
        && CLOCK_STEPS_LOGGED.fetch_add(1, Ordering::AcqRel) < 40
    {
        log(&format!(
            "  harness: the clock moved {step:.6} s in update {}",
            frame - 1
        ));
    }
}

/// Wraps `Phyre_PRendering_PRenderer_flip` (`0x005b6af0`,
/// `thiscall(renderer) -> error`): on success it has written the flip
/// command (`0x16`) into the render ring, which the render thread runs once
/// the caller kicks it. Notes which update the flip shows.
///
/// # Safety
/// Called by the game, on the main thread.
pub unsafe extern "thiscall" fn flip(renderer: *mut c_void) -> u32 {
    // SAFETY: the trampoline runs the original with the same `this`.
    let r = unsafe {
        let original: unsafe extern "thiscall" fn(*mut c_void) -> u32 =
            std::mem::transmute(FLIP_ORIGINAL.load(Ordering::Acquire));
        original(renderer)
    };
    if r == 0 {
        let n = FLIPS.load(Ordering::Acquire);
        FLIP_LABELS[n as usize % FLIP_LABELS.len()]
            .store(FRAME.load(Ordering::Acquire), Ordering::Release);
        if VIRTUAL.load(Ordering::Acquire) {
            // SAFETY: the wrapped clock, as the game reads it.
            let now = unsafe { time_now() };
            FLIP_TIMES[n as usize % FLIP_TIMES.len()].store(now.to_bits(), Ordering::Release);
        }
        FLIPS.store(n + 1, Ordering::Release);
    }
    r
}

/// The virtual clock when each flip was queued (`f64` bits), by flip
/// number as [`FLIP_LABELS`]: the game time of the update the flip shows.
static FLIP_TIMES: [AtomicU64; 64] = [const { AtomicU64::new(0) }; 64];
/// The original `IggyPlayerReadyToTick` (`iggy_w32.dll`, stdcall(player)).
static IGGY_READY_TO_TICK: AtomicUsize = AtomicUsize::new(0);
/// `IggyPlayerDebugSetTime` (`iggy_w32.dll`, stdcall(player, f64 seconds)).
static IGGY_SET_TIME: AtomicUsize = AtomicUsize::new(0);
/// A render-thread Iggy tick came before its flip was queued.
static IGGY_UNTIMED: AtomicBool = AtomicBool::new(false);

/// Iggy's player fields (`iggy_w32.dll` of 2015-02-04, the game's copy):
/// the time `IggyPlayerDebugSetTime` sets (`f64`, 0: read the clock), and
/// the time of the last tick check (`f64`), from which the next check adds
/// the elapsed time to the tick accumulator (`+0x28`, compared with the
/// period, `1 / fps` at `+0x21e0`).
const IGGY_SET_TIME_FIELD: usize = 0x21d0;
const IGGY_LAST_TIME_FIELD: usize = 0x21d8;

/// The game's `IggyPlayerReadyToTick` import on the virtual clock (render
/// thread, `0x00a3d9f0`: `while ReadyToTick: TickRS`). Iggy times its
/// Flash frames by the wall clock (`QueryPerformanceCounter`, read in the
/// tick check), so how many Flash frames a page's slide or fade had run at
/// a given update followed real time: iggy-menus' Key bindings page slid
/// one Flash frame less in some runs (98,378 pixels; docs/harness.md,
/// "Coverage replays"). Here the player's time is the game time of the
/// update this draw belongs to (the flip it precedes, [`FLIP_TIMES`]), set
/// through Iggy's own `IggyPlayerDebugSetTime`, which the tick check's
/// time reads honour. A player's first check takes that time as its last
/// one too, so the real time Iggy read when the player started never
/// enters an elapsed time.
///
/// # Safety
/// Called by the game with an Iggy player (or null).
unsafe extern "system" fn iggy_ready_to_tick(player: usize) -> i32 {
    note_iggy_player(player);
    let set_time = IGGY_SET_TIME.load(Ordering::Acquire);
    if player != 0 && set_time != 0 {
        let run = FLIPS_RUN.load(Ordering::Acquire);
        let now = if run < FLIPS.load(Ordering::Acquire) {
            f64::from_bits(FLIP_TIMES[run as usize % FLIP_TIMES.len()].load(Ordering::Acquire))
        } else {
            if !IGGY_UNTIMED.swap(true, Ordering::AcqRel) {
                log("  harness: an Iggy tick before its flip was queued (the clock now)");
            }
            // SAFETY: the wrapped clock.
            unsafe { time_now() }
        };
        // SAFETY: the player Iggy made, with the fields above; this runs
        // under the game's Iggy critical section (0x01a858cc).
        unsafe {
            let set = (player + IGGY_SET_TIME_FIELD) as *mut f64;
            if set.read_unaligned() == 0.0 {
                ((player + IGGY_LAST_TIME_FIELD) as *mut f64).write_unaligned(now);
            }
            let f: unsafe extern "system" fn(usize, f64) = std::mem::transmute(set_time);
            f(player, now);
        }
    }
    // SAFETY: the original import, as the game calls it.
    unsafe {
        let f: unsafe extern "system" fn(usize) -> i32 =
            std::mem::transmute(IGGY_READY_TO_TICK.load(Ordering::Acquire));
        f(player)
    }
}

/// The Iggy players the tick check has seen, in order (0: free).
static IGGY_PLAYERS: [AtomicUsize; 32] = [const { AtomicUsize::new(0) }; 32];

fn note_iggy_player(player: usize) {
    if player == 0 {
        return;
    }
    for slot in &IGGY_PLAYERS {
        match slot.compare_exchange(0, player, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(seen) if seen == player => return,
            Err(_) => {}
        }
    }
}

/// `iggyplayers`: each player the tick check has seen, with its total
/// time (`+0x20`), tick accumulator (`+0x28`) and last check (`+0x21d8`),
/// `f64`s. A player Iggy has freed since reads as whatever is there now.
fn iggy_players() -> String {
    use std::fmt::Write as _;
    let mut out = String::from("ok");
    for slot in &IGGY_PLAYERS {
        let p = slot.load(Ordering::Acquire);
        if p == 0 {
            break;
        }
        let f = |off: usize| {
            read_memory(p + off, 8)
                .ok()
                .and_then(|b| b.try_into().ok())
                .map_or(f64::NAN, f64::from_le_bytes)
        };
        let _ = write!(
            out,
            " {p:#x}: total {:e} acc {:e} last {:e};",
            f(0x20),
            f(0x28),
            f(IGGY_LAST_TIME_FIELD)
        );
    }
    out
}

/// Puts Iggy's tick check on the virtual clock ([`iggy_ready_to_tick`]).
///
/// # Safety
/// `module` is `FFX.exe`, at start-up (no Iggy player yet).
unsafe fn patch_iggy_clock(module: usize) -> bool {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    // SAFETY: plain lookups, and the import patched before the game's
    // first Iggy player exists.
    unsafe {
        let iggy = GetModuleHandleA(c"iggy_w32.dll".as_ptr().cast());
        if iggy.is_null() {
            return false;
        }
        let Some(set_time) = GetProcAddress(iggy, c"_IggyPlayerDebugSetTime@12".as_ptr().cast())
        else {
            return false;
        };
        IGGY_SET_TIME.store(set_time as usize, Ordering::Release);
        match crate::iat::patch(
            module,
            "iggy_w32.dll",
            "_IggyPlayerReadyToTick@4",
            iggy_ready_to_tick as *const () as usize,
        ) {
            Some(original) => {
                IGGY_READY_TO_TICK.store(original, Ordering::Release);
                true
            }
            None => false,
        }
    }
}

/// Wraps `Renderer_PresentFlip` (`0x00593760`, `thiscall(renderer) ->
/// bool`): the render thread's flip command, from
/// `Phyre_PRendering_PRenderThread_processRenderCommands`; it presents
/// through the swap chain object at `+0x74` (its `+0x20`, which calls
/// `IDXGISwapChain::Present`). Tells [`present`] which update it shows.
///
/// # Safety
/// Called by the game, on the render thread.
pub unsafe extern "thiscall" fn present_flip(renderer: *mut c_void) -> u32 {
    let n = FLIPS_RUN.fetch_add(1, Ordering::AcqRel);
    let label = if n < FLIPS.load(Ordering::Acquire) {
        FLIP_LABELS[n as usize % FLIP_LABELS.len()].load(Ordering::Acquire)
    } else {
        NO_LABEL
    };
    PRESENT_LABEL.store(label, Ordering::Release);
    // SAFETY: the trampoline runs the original with the same `this`.
    let r = unsafe {
        let original: unsafe extern "thiscall" fn(*mut c_void) -> u32 =
            std::mem::transmute(PRESENT_FLIP_ORIGINAL.load(Ordering::Acquire));
        original(renderer)
    };
    PRESENT_LABEL.store(NO_LABEL, Ordering::Release);
    r
}

/// Replaces `IDXGISwapChain::Present`, on Phyre's render thread
/// (`Phyre_PRendering_PRenderThread_threadEntry`, `0x00616010`): takes the
/// screenshots this image satisfies, and in lockstep holds the image of
/// update `n` until the main thread has left frame boundary `n`, so a
/// screenshot asked for there shows the update the trace saw.
unsafe extern "system" fn present(swap_chain: *mut c_void, interval: u32, flags: u32) -> i32 {
    PRESENTS.fetch_add(1, Ordering::AcqRel);
    crate::loader_turn::note_render_thread();
    let label = PRESENT_LABEL.load(Ordering::Acquire);
    let started = REACHED.load(Ordering::Acquire) != 0;
    if label == NO_LABEL && started && !UNLABELLED.swap(true, Ordering::AcqRel) {
        log("  harness: a present without a flip label (screenshots may show another update)");
    }
    // Before the main loop's first update (the start-up logos) the virtual
    // vsync has no frame boundary to run at.
    if VIRTUAL.load(Ordering::Acquire)
        && !started
        && let Ok(_one) = VSYNC_LOCK.try_lock()
    {
        run_vsync();
    }
    if FAST.load(Ordering::Relaxed) {
        skip_opening_screen();
    }
    // SAFETY: forwarded: inside the swap chain's own Present.
    unsafe { run_render_jobs(label, swap_chain) };
    if label != NO_LABEL {
        // The main thread reaches boundary `label` right after queueing
        // this flip. One that does not within seconds would be waiting for
        // this thread (no known path does), so the hold ends then.
        let mut waited = 0u32;
        let hold = || LOCKSTEP.load(Ordering::Acquire) || PINNED.load(Ordering::Acquire) > 0;
        while hold()
            && PASSED
                .load(Ordering::Acquire)
                .max(PRESENT_RELEASED.load(Ordering::Acquire))
                <= label
            && (REACHED.load(Ordering::Acquire) > label || waited < 5000)
        {
            std::thread::sleep(std::time::Duration::from_millis(1));
            waited += 1;
            // SAFETY: as above.
            unsafe { run_render_jobs(label, swap_chain) };
        }
        if waited > 0 {
            // a pinned shot the main thread queued just before it left
            // the boundary
            // SAFETY: as above.
            unsafe { run_render_jobs(label, swap_chain) };
        }
        if LOCKSTEP.load(Ordering::Acquire) && REACHED.load(Ordering::Acquire) <= label {
            log(&format!(
                "  harness: present of update {label}: the main thread did not reach its boundary"
            ));
        }
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
    // SAFETY: a byte in the image's data, which the start-up loop polls on
    // the main thread; a byte write cannot tear.
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

/// `stacks`: for every thread but this one, its `eip` and the return
/// addresses into `FFX.exe` found on its stack (words that follow a
/// `call`), innermost first: `tid:eip:a,b,c ...`. A sampling aid for
/// finding what a thread waits on. Each thread is suspended only while its
/// registers and stack are copied (no allocation then: it may hold the
/// heap's lock).
#[allow(clippy::cast_possible_truncation)] // 32-bit process
pub(crate) fn thread_stacks() -> String {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        CONTEXT, GetThreadContext, ReadProcessMemory,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId, OpenThread, ResumeThread,
        SuspendThread, THREAD_GET_CONTEXT, THREAD_SUSPEND_RESUME,
    };
    const CONTEXT_CONTROL: u32 = 0x0001_0001;
    const WORDS: usize = 0x1000;
    let base = crate::GAME_BASE.load(Ordering::Relaxed);
    let end = base + crate::SIZE_OF_IMAGE as usize;
    let image = |a: usize| a - base + crate::IMAGE_BASE as usize;
    let mut ids = Vec::new();
    // SAFETY: plain Win32 calls on this process's threads.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        let (pid, me) = (GetCurrentProcessId(), GetCurrentThreadId());
        let mut te: THREADENTRY32 = std::mem::zeroed();
        te.dwSize = u32::try_from(size_of::<THREADENTRY32>()).unwrap_or(0);
        let mut ok = Thread32First(snap, &raw mut te) != 0;
        while ok {
            if te.th32OwnerProcessID == pid && te.th32ThreadID != me {
                ids.push(te.th32ThreadID);
            }
            ok = Thread32Next(snap, &raw mut te) != 0;
        }
        CloseHandle(snap);
    }
    let mut stack = vec![0u32; WORDS];
    let mut out = Vec::new();
    for id in ids {
        let mut eip = 0usize;
        let mut copied = 0usize;
        // SAFETY: the thread is suspended while its context and stack are
        // read; ReadProcessMemory fails instead of faulting.
        unsafe {
            let h = OpenThread(THREAD_GET_CONTEXT | THREAD_SUSPEND_RESUME, 0, id);
            if h.is_null() {
                continue;
            }
            if SuspendThread(h) != u32::MAX {
                let mut ctx: CONTEXT = std::mem::zeroed();
                ctx.ContextFlags = CONTEXT_CONTROL;
                if GetThreadContext(h, &raw mut ctx) != 0 {
                    eip = ctx.Eip as usize;
                    let mut n = WORDS * 4;
                    // read as much as the stack's pages allow
                    while n >= 0x100 {
                        let mut got = 0usize;
                        if ReadProcessMemory(
                            GetCurrentProcess(),
                            ctx.Esp as usize as *const c_void,
                            stack.as_mut_ptr().cast(),
                            n,
                            &raw mut got,
                        ) != 0
                        {
                            copied = got / 4;
                            break;
                        }
                        n /= 2;
                    }
                }
                ResumeThread(h);
            }
            CloseHandle(h);
        }
        let mut frames = Vec::new();
        for &w in &stack[..copied] {
            let a = w as usize;
            if a < base + 6 || a >= end {
                continue;
            }
            // SAFETY: inside the image, which is mapped and readable.
            let before = unsafe { std::slice::from_raw_parts((a - 6) as *const u8, 6) };
            let call = before[1] == 0xe8
                || (before[4] == 0xff && before[5] & 0x38 == 0x10)
                || (before[3] == 0xff && before[4] & 0x38 == 0x10)
                || (before[0] == 0xff && before[1] & 0x38 == 0x10);
            if call {
                frames.push(format!("{:08x}", image(a)));
                if frames.len() >= 24 {
                    break;
                }
            }
        }
        let at = if eip >= base && eip < end {
            format!("{:08x}", image(eip))
        } else {
            format!("{eip:#x}")
        };
        out.push(format!("{id}:{at}:{}", frames.join(",")));
    }
    format!("ok {}", out.join(" "))
}

/// Runs a job on the main thread at a frame boundary, or a screenshot on
/// the render thread, and waits for its reply.
fn on_game_thread(job: Job) -> String {
    if PRESENT.load(Ordering::Acquire) == 0 {
        return "err the game has not presented a frame yet".into();
    }
    let (tx, rx) = channel();
    let queue = if matches!(job, Job::Shot(..)) {
        &RENDER_JOBS
    } else {
        &JOBS
    };
    if let Ok(mut jobs) = queue.lock() {
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
            "ok frame {} pad_reads {} key_polls {} presents {} flips {} flips_run {} seeded_jobs {}",
            now(),
            PAD_READS.load(Ordering::Relaxed),
            KEY_POLLS.load(Ordering::Relaxed),
            PRESENTS.load(Ordering::Relaxed),
            FLIPS.load(Ordering::Relaxed),
            FLIPS_RUN.load(Ordering::Relaxed),
            SEEDED_JOBS.load(Ordering::Relaxed)
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
        "holdfor" => {
            // holdfor <range> <value hex> <mask hex> <ms>: in lockstep only,
            // holds the game at the boundary it stands at until the bytes
            // match (other threads' work: a movie's prebuffer), at most
            // <ms> of real time; no update runs meanwhile
            if !LOCKSTEP.load(Ordering::Acquire) {
                return Err("holdfor needs lockstep".into());
            }
            let r = parse_trace_range(arg(1)?)?;
            let hexbytes = |t: &str| -> Result<Vec<u8>, String> {
                (0..t.len() / 2)
                    .map(|i| {
                        u8::from_str_radix(&t[2 * i..2 * i + 2], 16).map_err(|e| e.to_string())
                    })
                    .collect()
            };
            let value = hexbytes(arg(2)?)?;
            let mask = hexbytes(arg(3)?)?;
            let ms = parse_u32(arg(4)?)?;
            if value.len() != r.len || mask.len() != r.len {
                return Err("value and mask must be as long as the range".into());
            }
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_millis(u64::from(ms));
            on_game_thread(Job::HoldFor(r, value, mask, deadline))
        }
        "shotwhen" => {
            // shotwhen <range> <value hex> <mask hex> <frames> <path>: as
            // waitfor, then the image of the update the bytes first matched
            // after, exactly (in real time too: its present waits for the
            // boundary)
            let r = parse_trace_range(arg(1)?)?;
            let hexbytes = |t: &str| -> Result<Vec<u8>, String> {
                (0..t.len() / 2)
                    .map(|i| {
                        u8::from_str_radix(&t[2 * i..2 * i + 2], 16).map_err(|e| e.to_string())
                    })
                    .collect()
            };
            let value = hexbytes(arg(2)?)?;
            let mask = hexbytes(arg(3)?)?;
            let limit = now() + parse_u32(arg(4)?)?;
            let path = words.get(5..).map(|w| w.join(" ")).unwrap_or_default();
            if path.is_empty() {
                return Err("shotwhen: missing the path".into());
            }
            if value.len() != r.len || mask.len() != r.len {
                return Err("value and mask must be as long as the range".into());
            }
            if PRESENT.load(Ordering::Acquire) == 0 {
                return Err("the game has not presented a frame yet".into());
            }
            ALLOWED.fetch_max(limit, Ordering::AcqRel);
            PINNED.fetch_add(1, Ordering::AcqRel);
            on_game_thread(Job::ShotWhen(r, value, mask, limit, path, None))
        }
        "lockstep" => {
            let on = arg(1)? == "on";
            if on && !LOCKSTEP.load(Ordering::Acquire) {
                // the main thread takes hold at its next boundary
                // (LOCKSTEP_REQUEST); a main loop that does not get there
                // in time (stuck in one update) is held at once, as before.
                // Before the main loop's first update (the start-up loop:
                // logos, the first loads) too: held at once there, every
                // start-up present waited 5 s for a boundary the main
                // thread had not reached (update 0), and a boot in
                // lockstep never got to its first update (replay-flakes3).
                LOCKSTEP_REQUEST.store(true, Ordering::Release);
                let start = std::time::Instant::now();
                while LOCKSTEP_REQUEST.load(Ordering::Acquire)
                    && start.elapsed() < std::time::Duration::from_secs(60)
                {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                if LOCKSTEP_REQUEST.swap(false, Ordering::AcqRel) {
                    ALLOWED.store(now(), Ordering::Release);
                    LOCKSTEP.store(true, Ordering::Release);
                }
                return Ok(format!("ok {}", ALLOWED.load(Ordering::Acquire)));
            }
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
            // readp <addr> <off>... <len>: *addr + off; with more offsets
            // each one before the last is followed too (*(*addr + off1) + off2)
            if words.len() < 4 {
                return Err("readp <addr> <off>... <len>".into());
            }
            let word = |at: usize| -> Result<usize, String> {
                let p = read_memory(at, 4)?;
                Ok(u32::from_le_bytes(p.try_into().expect("4 bytes")) as usize)
            };
            let mut at = word(crate::rebase(parse_u32(arg(1)?)?))?;
            let offs = &words[2..words.len() - 1];
            for (i, off) in offs.iter().enumerate() {
                at += parse_u32(off)? as usize;
                if i + 1 < offs.len() {
                    at = word(at)?;
                }
            }
            let len = parse_u32(words[words.len() - 1])? as usize;
            format!("ok {}", hex(&read_memory(at, len)?))
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
        "callnext" => {
            // callnext <address> [args]: as `call`, but at the start of the
            // next update (the main thread has left its boundary, the
            // render thread its present hold), as the game's own code
            // would call it; in lockstep that one update runs
            let address = crate::rebase(parse_u32(arg(1)?)?);
            let args = words[2..]
                .iter()
                .map(|w| parse_u32(w))
                .collect::<Result<Vec<_>, _>>()?;
            if args.len() > 8 {
                return Err("at most 8 arguments".into());
            }
            if PRESENT.load(Ordering::Acquire) == 0 {
                return Err("the game has not presented a frame yet".into());
            }
            let (tx, rx) = channel();
            if let Ok(mut calls) = UPDATE_CALLS.lock() {
                calls.push_back((address, args, tx));
            }
            ALLOWED.fetch_max(now() + 1, Ordering::AcqRel);
            rx.recv().unwrap_or_else(|_| "err no reply".into())
        }
        // the image of the update the main thread stands after (in
        // lockstep: the boundary it is held at), or a later one
        "shot" => on_game_thread(Job::Shot(words[1..].join(" "), now(), false)),
        "snapshot" => on_game_thread(Job::Snapshot(words[1..].join(" "))),
        "fpucw" => on_game_thread(Job::FpuControl),
        "pin" => on_game_thread(Job::Pin),
        // where the FFX_AUDIO_WAV capture stands (crate::audio_wav)
        "audiomark" => crate::audio_wav::mark(arg(1)?),
        "loadsave" => {
            let lockstep = words.len() > 2 && words.last() == Some(&"lockstep");
            let spec = &words[1..words.len() - usize::from(lockstep)];
            on_game_thread(Job::LoadSave(spec.join(" "), lockstep))
        }
        "threads" => list_threads(),
        "iggyplayers" => iggy_players(),
        "stacks" => thread_stacks(),
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
        "gfxreplay" => crate::gfx::replay::request(&words[1..].join(" ")),
        "rustmap" => crate::gfx::scene::request(&words[1..].join(" ")),
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
        "cursor" => {
            // cursor <x> <y> | cursor off: the pointer the game sees in
            // background mode ([`get_cursor_pos`]), in client pixels
            if !BACKGROUND.load(Ordering::Relaxed) {
                return Err("cursor: background mode only".into());
            }
            if arg(1)? == "off" {
                CURSOR.store(NO_CURSOR, Ordering::Release);
            } else {
                let x: i32 = arg(1)?.parse().map_err(|e| format!("{e}"))?;
                let y: i32 = arg(2)?.parse().map_err(|e| format!("{e}"))?;
                #[allow(clippy::cast_sign_loss)] // packed i32 halves
                let packed = (u64::from(x as u32) << 32) | u64::from(y as u32);
                CURSOR.store(packed, Ordering::Release);
            }
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

/// `FFXApplication::animate`'s port (M5 `main`, `ported/m5_main.rs`)
/// without the harness: the plain replacement at the address the
/// harness's [`animate`] wrapper takes otherwise.
///
/// # Safety
/// Called by the game, on the main thread.
unsafe extern "thiscall" fn animate_port(app: *mut c_void) -> u32 {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    crate::ported::ffx_application_animate(app as usize as u32)
}

/// Installs [`animate_port`] when the harness is off and the port is on.
///
/// # Safety
/// `module` must be the main module, before other threads run game code.
unsafe fn install_animate_port(module: usize) {
    if !crate::ported::ffx_application_animate_on() {
        return;
    }
    let hooks = [crate::Hook {
        name: "FFXApplication_animate_port",
        address: 0x0042_f520,
        // cmp byte [abs32], 0 (relocated: only its opcode is checked)
        prologue: &[0x80, 0x3d],
        replacement: animate_port as *const () as usize,
        original: None,
        steal: 0,
    }];
    // SAFETY: forwarded from the caller.
    let r = unsafe { crate::install(module, &hooks, &[]) };
    log(&format!("  FFXApplication::animate port {r:?}"));
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
        // SAFETY: forwarded from the caller.
        unsafe { install_animate_port(module) };
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
                "  harness: virtual clock from {VIRTUAL_START} s: synchronous reads, the vsync \
                 handler at the main thread's frame boundary {r:?}"
            ));
            // SAFETY: forwarded from the caller (start-up).
            let iggy = unsafe { patch_iggy_clock(module) };
            log(&format!(
                "  harness: Iggy's tick check on the virtual clock: {}",
                if iggy { "on" } else { "not available" }
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
    // The frame boundary on the main thread, and which update each present
    // shows (docs/harness.md, "Frames"). Trampolines, so `--hook-disable
    // replacements` keeps them: the original code runs under the same
    // harness.
    let frame_hooks = [
        crate::Hook {
            name: "FFXApplication_animate",
            address: 0x0042_f520,
            // cmp byte [abs32], 0 (relocated: only its opcode is checked;
            // the 7 bytes are copied)
            prologue: &[0x80, 0x3d],
            replacement: animate as *const () as usize,
            original: Some(&ANIMATE_ORIGINAL),
            steal: 7,
        },
        crate::Hook {
            name: "Phyre_PRendering_PRenderer_flip",
            address: 0x005b_6af0,
            // push ebp; mov ebp, esp; sub esp, 8
            prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
            replacement: flip as *const () as usize,
            original: Some(&FLIP_ORIGINAL),
            steal: 0,
        },
        crate::Hook {
            name: "Renderer_PresentFlip",
            address: 0x0059_3760,
            // push esi; mov esi, ecx; mov edx, [esi + 0x74]
            prologue: &[0x56, 0x8b, 0xf1, 0x8b, 0x56, 0x74],
            replacement: present_flip as *const () as usize,
            original: Some(&PRESENT_FLIP_ORIGINAL),
            steal: 0,
        },
    ];
    // SAFETY: the game has not started (we are in its start-up).
    let r = unsafe { crate::install(module, &frame_hooks, &[]) };
    log(&format!("  harness: frame boundary hooks {r:?}"));
    // The asset loader's turn at the render lock in lockstep
    // (crate::loader_turn; docs/harness.md, "Load pacing"). A trampoline:
    // the original code runs under the same harness. The lock side is the
    // port of Phyre_PMutex_lockInner_00431850 (ported/p1_load.rs), or
    // loader_turn::install_fallback's wrapper when that port is left out.
    let loader_hooks = [crate::Hook {
        name: "AsyncLoadManager_threadMain_00691360",
        address: 0x0069_1360,
        // push ebp; mov ebp, esp; mov eax, [abs32] (relocated: only its
        // opcode is checked; 8 bytes are copied)
        prologue: &[0x55, 0x8b, 0xec, 0xa1],
        replacement: crate::loader_turn::loader_thread as *const () as usize,
        original: Some(&crate::loader_turn::LOADER_ORIGINAL),
        steal: 8,
    }];
    // SAFETY: the game has not started (the loader thread is made later).
    let r = unsafe { crate::install(module, &loader_hooks, &[]) };
    log(&format!("  harness: loader turn hooks {r:?}"));
    // SAFETY: forwarded from the caller.
    let seeded = unsafe { patch_seeded_jobs(module) };
    log(&format!(
        "  harness: cloth and middle-calc jobs seeded per character and event frame: {}",
        if seeded {
            "on"
        } else {
            "not available (unexpected code)"
        }
    ));
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
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "USER32.dll",
            "GetCursorPos",
            get_cursor_pos as *const () as usize,
            &GET_CURSOR_POS,
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "USER32.dll",
            "SetCursorPos",
            set_cursor_pos as *const () as usize,
            &SET_CURSOR_POS,
        )))
        .chain(BACKGROUND.load(Ordering::Relaxed).then_some((
            "USER32.dll",
            "ClipCursor",
            clip_cursor as *const () as usize,
            &CLIP_CURSOR,
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
