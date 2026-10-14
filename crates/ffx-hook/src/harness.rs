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
//!   time to the game's clock (`Time_Now`, `0x0042fd80`), so the game still
//!   sees its usual frame rate;
//! - with `FFX_HARNESS_BACKGROUND=1`, shows the window without activating
//!   it and makes the keyboard a background device, so the game neither
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
//! | `events` | `ok <n>` then `n` lines: files opened since the last call |
//! | `quit` | ends the process |

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
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
/// (≥ 28) paired with it reads as held, as if its key were pressed. `Input_UpdatePad` (`0x00632a30`) asks this for every
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
/// plus 1, which `Rng_Seed` (`0x00798890`) turns into the seeds of the
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

/// Trampoline to the original `Time_Now`.
pub static TIME_NOW_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Seconds the frame limiter skipped, as `f64` bits.
static SKIPPED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// The limiter's replacement `Sleep`, called through this pointer by the
/// patched `call [ptr]` at `0x0062f3bb`.
static FAST_SLEEP: unsafe extern "system" fn(u32) = fast_sleep;

/// Wraps `Time_Now` (`0x0042fd80`, seconds): adds the skipped time.
///
/// # Safety
/// Called by the game.
#[must_use]
pub unsafe extern "C" fn time_now() -> f64 {
    // SAFETY: the trampoline runs the original, which returns in st(0).
    let now = unsafe {
        let original: unsafe extern "C" fn() -> f64 =
            std::mem::transmute(TIME_NOW_ORIGINAL.load(Ordering::Acquire));
        original()
    };
    now + f64::from_bits(SKIPPED.load(Ordering::Acquire))
}

/// The frame limiter's `Sleep(ms)` in fast mode: no wait, the clock moves.
unsafe extern "system" fn fast_sleep(ms: u32) {
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
    if !ENABLED.load(Ordering::Acquire) || CREATE_DEVICE.load(Ordering::Acquire) != 0 {
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
/// keyboard that is not acquired reads as all keys up instead of failing.
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
    let isolate = BACKGROUND.load(Ordering::Relaxed);
    if isolate && !data.is_null() && size != 256 {
        // SAFETY: the caller's buffer holds `size` bytes.
        unsafe { std::ptr::write_bytes(data, 0, size as usize) };
        hr = 0;
    }
    if size == 256 && !data.is_null() {
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
}

static JOBS: Mutex<VecDeque<(Job, Sender<String>)>> = Mutex::new(VecDeque::new());
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

/// Replaces `ShowWindow`: shows without activating in background mode.
unsafe extern "system" fn show_window(window: *mut c_void, mut cmd: i32) -> i32 {
    const SW_SHOWNOACTIVATE: i32 = 4;
    const SW_SHOWMINNOACTIVE: i32 = 7;
    if BACKGROUND.load(Ordering::Relaxed) && cmd != 0 {
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
    unsafe { original(window, cmd) }
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
unsafe extern "system" fn present(swap_chain: *mut c_void, interval: u32, flags: u32) -> i32 {
    let frame = FRAME.fetch_add(1, Ordering::AcqRel) + 1;
    let due: Vec<(Job, Sender<String>)> = JOBS.lock().map_or_else(
        |_| Vec::new(),
        |mut jobs| {
            let mut keep = VecDeque::new();
            let mut due = Vec::new();
            while let Some(job) = jobs.pop_front() {
                let ready = match job.0 {
                    Job::WaitUntil(f) | Job::Release(f, _) => frame >= f,
                    Job::Call(..) | Job::Shot(_) => true,
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
            Job::Shot(path) => {
                // SAFETY: the swap chain is live inside its own Present.
                let r = unsafe { screenshot(swap_chain, &path) };
                let _ = reply
                    .send(r.map_or_else(|e| format!("err {e}"), |(w, h)| format!("ok {w} {h}")));
            }
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

/// A cdecl call with up to eight `u32` arguments.
unsafe fn call(address: usize, args: &[u32]) -> u32 {
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
fn read_memory(address: usize, len: usize) -> Result<Vec<u8>, String> {
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
            on_game_thread(Job::Release(release, release + gap))
        }
        "wait" => on_game_thread(Job::WaitUntil(now() + parse_u32(arg(1)?)?)),
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
        "write" => {
            let at = crate::rebase(parse_u32(arg(1)?)?);
            let text = arg(2)?;
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
    // SAFETY: forwarded from the caller.
    unsafe {
        for (dll, function, replacement, slot) in [
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
