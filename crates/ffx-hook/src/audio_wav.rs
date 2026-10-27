//! FMOD's mixed output to a WAV file, deterministically
//! (`FFX_AUDIO_WAV=<file>`; `docs/harness.md`, "Audio capture").
//!
//! **The writer.** `FfxFmod_initEventSystem` (`0x00706820`, original or
//! its port) starts FMOD with `EventSystem::init(128, 0, NULL, 0)` through
//! `FFX.exe`'s import slot. The wrapper put into that slot first sets the
//! low-level `System` up, before its `init`:
//!
//! - output `FMOD_OUTPUTTYPE_WAVWRITER_NRT` (5 in FMOD Ex 4.44): no audio
//!   device is opened, nothing reaches the speakers, and the mixer runs
//!   only inside `System::update` (one DSP block per update), which
//!   `EventSystem::update` calls;
//! - the software format with `FMOD_SOUND_FORMAT_PCMFLOAT` (5): the WAV
//!   holds the mixer's own 32-bit floats, so a difference far below one
//!   16-bit step still shows, whatever `FFX_TEST_VOLUME` is;
//! - DSP blocks of `rate / 30` samples (1600 at 48 kHz): one block per
//!   FMOD tick (33.333 ms of game time), so the sound keeps the game's
//!   time and the voice streams end where they do in the game;
//! - `init` flags `| FMOD_INIT_STREAM_FROM_UPDATE` (1): no stream thread;
//!   streams (music, voices) are decoded inside `System::update` too;
//! - `extradriverdata` = the file name (the writer's file).
//!
//! **Lockstep.** The `FmodManager` thread (`FmodManager_tick` `0x006fa700`)
//! runs on its own timer: each wake-up runs the FMOD command queue
//! (`0x006fa7e0`) and, once 33.333 ms of game time have passed,
//! `FfxFmod_soundLoop`. Against the main thread that is a race: which
//! update a sound starts in, and so every sample after it, followed real
//! time. With the virtual clock, once the main thread has reached its
//! first frame boundary, the tick is wrapped ([`tick`]): the `FmodManager`
//! thread runs one tick only when the main thread gives it its turn, at
//! every frame boundary just before the update ([`boundary_turn`], from
//! the harness's `animate`), and the main thread waits for it. Between two
//! turns it only serves a synchronous command the main thread waits for
//! (`FmodManager_registCommandSync`, `0x006fa500`: done byte `+0x44`
//! cleared), by running that command's handler alone, as the queue run
//! does first. A turn that does not come for [`STALL`] (30 s) while the
//! main thread is inside an update (a wait for FMOD no one knows of) runs
//! one free tick, logged with every thread's stack (`free` in the marks
//! file): from there on the output is not repeatable.
//!
//! **Marks.** `<file>.marks` (text) says where things are in the WAV: the
//! format, and one line per harness `audiomark <name>`: the frame, the
//! mixer's sample clock (`System::getDSPClock`, read after every
//! `EventSystem::update`) and the update count. `tools/wav-compare.py`
//! compares two WAVs between two marks. The harness's `quit` ends the
//! process without closing FMOD, so the WAV header's sizes are never
//! filled in and the last block or so may be lost: compare between marks,
//! up to a mark some frames before the end.
//!
//! **Without a file.** A harness game on the virtual clock
//! (`FFX_HARNESS`, `FFX_HARNESS_FAST=1`, `FFX_HARNESS_VIRTUAL_CLOCK=1`:
//! every lockstep replay) gets the same set-up and the same turns with
//! `FMOD_OUTPUTTYPE_NOSOUND_NRT` (4) and no file, unless
//! `FFX_AUDIO_NRT=0`. With a real output device the mixer plays in real
//! time, so a voice stream ended after however many updates the machine
//! managed meanwhile: Wakka's line in event-besaid closed its subtitle at
//! event count 0x74 to 0x82 over six runs, original code included, and
//! with it moved his mouth (auto talk until the voice ends) and every frame
//! the script waits on a voice or a music file. Under the turns the voice
//! plays one block per update, and the subtitle closed at 0x6f in every
//! run (`docs/harness.md`, "Sound in lockstep").

use std::ffi::{CString, c_void};
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
use windows_sys::Win32::System::Threading::{ResetEvent, SetEvent, WaitForSingleObject};

use crate::log::log;

/// `FMOD_OUTPUTTYPE_NOSOUND_NRT` (FMOD Ex 4.44).
const OUTPUT_NOSOUND_NRT: i32 = 4;
/// `FMOD_OUTPUTTYPE_WAVWRITER_NRT` (FMOD Ex 4.44).
const OUTPUT_WAVWRITER_NRT: i32 = 5;
/// `FMOD_SOUND_FORMAT_PCMFLOAT`.
const FORMAT_PCMFLOAT: i32 = 5;
/// `FMOD_INIT_STREAM_FROM_UPDATE`.
const INIT_STREAM_FROM_UPDATE: u32 = 1;
/// FMOD ticks per second of game time (`FmodManager_tick`'s 33.333 ms).
const TICKS_PER_SECOND: i32 = 30;
/// A turn not given for this long while the main thread is inside an
/// update: one free tick. Long map loads hold one update for seconds
/// (`CdReadSync` `0x0076c490` waiting for the read thread) without
/// needing FMOD; a free tick there runs the command queue mid-update and
/// moved everything after it by one block (2 s: one or two free ticks per
/// run, captures one block apart), so this is a last resort against a
/// deadlock only.
const STALL: Duration = Duration::from_secs(30);
/// How long the main thread waits for a turn to end before it gives up.
const PATIENCE: Duration = Duration::from_secs(20);

/// `FmodManager_tick`.
const TICK: u32 = 0x006f_a700;
/// `g_FmodCommandHandlers`: 70 cdecl `handler(FfxFmod*, u32 args[4])`.
const HANDLERS: u32 = 0x00c3_a368;

const INIT_NAME: &str = "?init@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@HIPAXI@Z";
const UPDATE_NAME: &str = "?update@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@XZ";

/// The capture is on (`FFX_AUDIO_WAV` set, the slots patched).
static ON: AtomicBool = AtomicBool::new(false);
/// The output type `EventSystem::init` sets: [`OUTPUT_WAVWRITER_NRT`]
/// with a file, [`OUTPUT_NOSOUND_NRT`] without.
static OUTPUT: AtomicU32 = AtomicU32::new(OUTPUT_WAVWRITER_NRT as u32);
/// The WAV file's name, for `EventSystem::init`.
static PATH: OnceLock<CString> = OnceLock::new();
/// The marks file.
static MARKS: Mutex<Option<std::fs::File>> = Mutex::new(None);
/// The original import targets.
static INIT_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static UPDATE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Trampoline to `FmodManager_tick`.
pub static TICK_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The low-level `System` (from `EventSystem::getSystemObject`).
static SYSTEM: AtomicUsize = AtomicUsize::new(0);
/// `g_FmodManager`, as the tick saw it.
static MANAGER: AtomicU32 = AtomicU32::new(0);
/// The mixer's sample clock after the last `EventSystem::update`.
static SAMPLE: AtomicU64 = AtomicU64::new(0);
/// `EventSystem::update` calls so far.
static UPDATES: AtomicU64 = AtomicU64::new(0);
/// Turns asked for and turns done.
static TURNS_ASKED: AtomicU64 = AtomicU64::new(0);
static TURNS_DONE: AtomicU64 = AtomicU64::new(0);
/// Turns are given (the tick waits for them).
static TURNS_ON: AtomicBool = AtomicBool::new(false);
/// Free ticks so far (each a point where the output stops repeating).
static FREE_TICKS: AtomicU32 = AtomicU32::new(0);
/// Turns the main thread gave up waiting for.
static LATE: AtomicU32 = AtomicU32::new(0);

static WAKE: Mutex<()> = Mutex::new(());
static DONE: Condvar = Condvar::new();

/// `GetProcAddress(dll, name)`, or 0.
fn export(dll: &std::ffi::CStr, name: &str) -> usize {
    let Ok(name) = CString::new(name) else {
        return 0;
    };
    // SAFETY: plain Win32 calls with NUL-terminated names.
    unsafe {
        let module = GetModuleHandleA(dll.as_ptr().cast());
        if module.is_null() {
            return 0;
        }
        GetProcAddress(module, name.as_ptr().cast()).map_or(0, |f| f as usize)
    }
}

/// Calls the `__stdcall` FMOD method `name` of `fmodex.dll` with `this`
/// and `args` (all words); `None` when it is not exported.
fn system_call(name: &str, this: usize, args: &[usize]) -> Option<u32> {
    let f = export(c"fmodex.dll", name);
    if f == 0 {
        return None;
    }
    // SAFETY: an exported FMOD Ex `System` method (`QAG`: stdcall, `this`
    // first) called with the words its signature takes.
    unsafe {
        Some(match args {
            [] => {
                let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(f);
                f(this)
            }
            [w0] => {
                let f: unsafe extern "system" fn(usize, usize) -> u32 = std::mem::transmute(f);
                f(this, *w0)
            }
            [w0, w1] => {
                let f: unsafe extern "system" fn(usize, usize, usize) -> u32 =
                    std::mem::transmute(f);
                f(this, *w0, *w1)
            }
            [w0, w1, w2, w3, w4] => {
                let f: unsafe extern "system" fn(usize, usize, usize, usize, usize, usize) -> u32 =
                    std::mem::transmute(f);
                f(this, *w0, *w1, *w2, *w3, *w4)
            }
            [w0, w1, w2, w3, w4, w5] => {
                let f: unsafe extern "system" fn(
                    usize,
                    usize,
                    usize,
                    usize,
                    usize,
                    usize,
                    usize,
                ) -> u32 = std::mem::transmute(f);
                f(this, *w0, *w1, *w2, *w3, *w4, *w5)
            }
            _ => return None,
        })
    }
}

fn mark_line(line: &str) {
    if let Ok(mut m) = MARKS.lock()
        && let Some(f) = m.as_mut()
    {
        let _ = writeln!(f, "{line}");
        let _ = f.flush();
    }
}

/// The software format: `(rate, format, output channels, max input
/// channels, resampler, bits)`.
fn software_format(system: usize) -> Option<[i32; 6]> {
    let mut v = [0i32; 6];
    let base = v.as_mut_ptr() as usize;
    let p = |i: usize| base + 4 * i;
    let r = system_call(
        "?getSoftwareFormat@System@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAW4FMOD_SOUND_FORMAT@@00PAW4FMOD_DSP_RESAMPLER@@0@Z",
        system,
        &[p(0), p(1), p(2), p(3), p(4), p(5)],
    )?;
    (r == 0).then_some(v)
}

/// Replaces `FMOD::EventSystem::init(this, maxchannels, flags,
/// extradriverdata, eventflags)` in `FFX.exe`'s import slot: the WAV
/// writer's set-up (module docs), then the original.
unsafe extern "system" fn event_system_init(
    this: usize,
    channels: i32,
    flags: u32,
    _extra: usize,
    event_flags: u32,
) -> u32 {
    let mut system = 0usize;
    let get = export(
        c"fmod_event.dll",
        "?getSystemObject@EventSystem@FMOD@@QAG?AW4FMOD_RESULT@@PAPAVSystem@2@@Z",
    );
    if get != 0 {
        // SAFETY: `EventSystem::getSystemObject(this, System**)`, stdcall.
        unsafe {
            let f: unsafe extern "system" fn(usize, *mut usize) -> u32 = std::mem::transmute(get);
            f(this, &raw mut system);
        }
    }
    let path = PATH.get().map_or(std::ptr::null(), |p| p.as_ptr());
    if system != 0 {
        SYSTEM.store(system, Ordering::Release);
        let kind = OUTPUT.load(Ordering::Acquire);
        let output = system_call(
            "?setOutput@System@FMOD@@QAG?AW4FMOD_RESULT@@W4FMOD_OUTPUTTYPE@@@Z",
            system,
            &[kind as usize],
        );
        let before = software_format(system);
        let set_format = before.and_then(|[rate, _, out, max_in, resampler, _]| {
            #[allow(clippy::cast_sign_loss)] // FMOD's ints, passed as words
            system_call(
                "?setSoftwareFormat@System@FMOD@@QAG?AW4FMOD_RESULT@@HW4FMOD_SOUND_FORMAT@@HHW4FMOD_DSP_RESAMPLER@@@Z",
                system,
                &[
                    rate as usize,
                    FORMAT_PCMFLOAT as usize,
                    out as usize,
                    max_in as usize,
                    resampler as usize,
                ],
            )
        });
        let rate = before.map_or(48_000, |f| f[0]);
        #[allow(clippy::cast_sign_loss)] // a positive rate
        let block = (rate / TICKS_PER_SECOND) as usize;
        let set_block = system_call(
            "?setDSPBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@IH@Z",
            system,
            &[block, 4],
        );
        log(&format!(
            "  audio wav: setOutput({kind}) {output:?}, software format before {before:?}, \
             PCMFLOAT {set_format:?}, DSP block {block} {set_block:?}"
        ));
    } else {
        log("  audio wav: no System object; FMOD starts as usual");
    }
    // SAFETY: the original import with the game's arguments, the flags and
    // the writer's file name changed.
    let r = unsafe {
        let f: unsafe extern "system" fn(usize, i32, u32, *const i8, u32) -> u32 =
            std::mem::transmute(INIT_ORIGINAL.load(Ordering::Acquire));
        f(
            this,
            channels,
            flags | INIT_STREAM_FROM_UPDATE,
            path.cast(),
            event_flags,
        )
    };
    if system != 0 {
        let after = software_format(system);
        let mut length = 0u32;
        let mut count = 0i32;
        let buffers = system_call(
            "?getDSPBufferSize@System@FMOD@@QAG?AW4FMOD_RESULT@@PAIPAH@Z",
            system,
            &[(&raw mut length) as usize, (&raw mut count) as usize],
        );
        let mut output = 0i32;
        system_call(
            "?getOutput@System@FMOD@@QAG?AW4FMOD_RESULT@@PAW4FMOD_OUTPUTTYPE@@@Z",
            system,
            &[(&raw mut output) as usize],
        );
        log(&format!(
            "  audio wav: EventSystem::init {r:#x}; output {output}, software format {after:?}, \
             DSP buffers {length} x {count} ({buffers:?})"
        ));
        if let Some([rate, format, channels, _, _, bits]) = after {
            mark_line(&format!(
                "format rate {rate} format {format} channels {channels} bits {bits} block {length} output {output} init {r:#x}"
            ));
        }
    }
    r
}

/// Replaces `FMOD::EventSystem::update(this)` in `FFX.exe`'s import slot:
/// the original, then the mixer's sample clock is noted.
unsafe extern "system" fn event_system_update(this: usize) -> u32 {
    // SAFETY: the original import with the game's argument.
    let r = unsafe {
        let f: unsafe extern "system" fn(usize) -> u32 =
            std::mem::transmute(UPDATE_ORIGINAL.load(Ordering::Acquire));
        f(this)
    };
    let system = SYSTEM.load(Ordering::Acquire);
    if system != 0 {
        let (mut hi, mut lo) = (0u32, 0u32);
        if system_call(
            "?getDSPClock@System@FMOD@@QAG?AW4FMOD_RESULT@@PAI0@Z",
            system,
            &[(&raw mut hi) as usize, (&raw mut lo) as usize],
        ) == Some(0)
        {
            SAMPLE.store(u64::from(hi) << 32 | u64::from(lo), Ordering::Release);
        }
    }
    let n = UPDATES.fetch_add(1, Ordering::AcqRel) + 1;
    crate::fmod_log::note(&format!(
        "update {n} {} {r:x}",
        SAMPLE.load(Ordering::Acquire)
    ));
    r
}

fn manager_word<T: Copy>(manager: u32, offset: u32) -> T {
    // SAFETY: a field of `g_FmodManager` (a live 0x50-byte object while its
    // thread runs), read racily as the game's own threads do.
    unsafe { std::ptr::read_volatile((manager + offset) as usize as *const T) }
}

fn event(manager: u32) -> *mut c_void {
    manager_word::<u32>(manager, 0x18) as usize as *mut c_void
}

/// Runs the pending synchronous command alone, as the queue run
/// (`0x006fa7e0`) does first: `handler[+0x30](+8, +0x34)` into `+0x48`,
/// `+0x44` = 1, `SetEvent(+0x1c)`.
fn serve_sync(manager: u32) {
    let command: u32 = manager_word(manager, 0x30);
    if command >= 70 {
        return;
    }
    let fmod: u32 = manager_word(manager, 8);
    // SAFETY: the game's handler table and its cdecl handlers, called on
    // the FMOD thread with the manager's FfxFmod and argument block, as
    // the queue run calls them; the fields written are the ones it writes.
    unsafe {
        let handler =
            std::ptr::read_volatile((crate::rebase(HANDLERS) + 4 * command as usize) as *const u32);
        let f: unsafe extern "C" fn(u32, u32) -> u32 = std::mem::transmute(handler as usize);
        let result = f(fmod, manager + 0x34);
        std::ptr::write_volatile((manager + 0x48) as usize as *mut u32, result);
        std::ptr::write_volatile((manager + 0x44) as usize as *mut u8, 1);
        SetEvent(manager_word::<u32>(manager, 0x1c) as usize as *mut c_void);
    }
}

/// Wraps `FmodManager_tick(mgr)` (`0x006fa700`, fastcall), on the
/// `FmodManager` thread: with turns on, waits for the main thread's turn
/// (serving synchronous commands meanwhile), then runs the original once.
unsafe extern "fastcall" fn tick(manager: u32) {
    MANAGER.store(manager, Ordering::Release);
    if crate::harness::reached() > 0 && crate::harness::virtual_clock() {
        TURNS_ON.store(true, Ordering::Release);
        wait_for_turn(manager);
    }
    if TURNS_ON.load(Ordering::Acquire) {
        // the turn's wait path (less than a period since the last
        // soundLoop) must not sleep: the event is set, so it returns at
        // once
        // SAFETY: the manager's event handle.
        unsafe { SetEvent(event(manager)) };
    }
    // SAFETY: the original through its trampoline, with the same manager.
    unsafe {
        let f: unsafe extern "fastcall" fn(u32) =
            std::mem::transmute(TICK_ORIGINAL.load(Ordering::Acquire));
        f(manager);
    }
    if TURNS_ON.load(Ordering::Acquire) {
        TURNS_DONE.store(TURNS_ASKED.load(Ordering::Acquire), Ordering::Release);
        let _g = WAKE.lock();
        DONE.notify_all();
    }
}

/// The `FmodManager` thread until its next turn.
fn wait_for_turn(manager: u32) {
    let mut seen = crate::harness::passed();
    let mut since = Instant::now();
    loop {
        if TURNS_ASKED.load(Ordering::Acquire) > TURNS_DONE.load(Ordering::Acquire) {
            return;
        }
        // the thread is asked to stop (`FmodManager_threadMain`'s loop)
        if manager_word::<u8>(manager, 0x4c) == 0 {
            return;
        }
        if manager_word::<u8>(manager, 0x44) == 0 {
            serve_sync(manager);
        }
        let passed = crate::harness::passed();
        let held = crate::harness::lockstep() && crate::harness::at_boundary();
        if passed != seen || held {
            seen = passed;
            since = Instant::now();
        } else if since.elapsed() > STALL {
            let n = FREE_TICKS.fetch_add(1, Ordering::AcqRel) + 1;
            let frame = crate::harness::frame();
            mark_line(&format!(
                "free frame {frame} sample {} updates {}",
                SAMPLE.load(Ordering::Acquire),
                UPDATES.load(Ordering::Acquire)
            ));
            if n <= 10 {
                log(&format!(
                    "  audio wav: no turn for {STALL:?} inside update {frame}: a free FMOD tick \
                     (the output stops repeating here); the threads:\n{}",
                    crate::harness::thread_stacks()
                ));
            }
            return;
        }
        // SAFETY: the manager's event: set by the main thread for a turn
        // and by a synchronous command.
        unsafe {
            WaitForSingleObject(event(manager), 50);
            ResetEvent(event(manager));
        }
    }
}

/// At the main thread's frame boundary, just before the update (the
/// harness's `animate`): the `FmodManager` thread's turn, waited for.
pub fn boundary_turn() {
    if !TURNS_ON.load(Ordering::Acquire) {
        return;
    }
    let manager = MANAGER.load(Ordering::Acquire);
    if manager == 0 {
        return;
    }
    let asked = TURNS_ASKED.fetch_add(1, Ordering::AcqRel) + 1;
    // SAFETY: the manager's event handle.
    unsafe { SetEvent(event(manager)) };
    let start = Instant::now();
    let Ok(mut g) = WAKE.lock() else {
        return;
    };
    while TURNS_DONE.load(Ordering::Acquire) < asked {
        if start.elapsed() > PATIENCE {
            if LATE.fetch_add(1, Ordering::AcqRel) < 10 {
                log(&format!(
                    "  audio wav: the FMOD turn of frame {} did not end in {PATIENCE:?}",
                    crate::harness::frame()
                ));
            }
            mark_line(&format!("late frame {}", crate::harness::frame()));
            return;
        }
        g = match DONE.wait_timeout(g, Duration::from_millis(5)) {
            Ok((g, _)) => g,
            Err(_) => return,
        };
    }
}

/// `audiomark <name>`: notes where the WAV stands (in lockstep, while the
/// main thread is held at a boundary, the `FmodManager` thread waits for
/// its turn, so this is the same point in every run).
pub fn mark(name: &str) -> String {
    if !ON.load(Ordering::Acquire) {
        return "ok off".into();
    }
    let (frame, sample, updates) = (
        crate::harness::frame(),
        SAMPLE.load(Ordering::Acquire),
        UPDATES.load(Ordering::Acquire),
    );
    let free = FREE_TICKS.load(Ordering::Acquire);
    let late = LATE.load(Ordering::Acquire);
    mark_line(&format!(
        "mark {name} frame {frame} sample {sample} updates {updates} free {free} late {late} turns {}",
        TURNS_ON.load(Ordering::Acquire)
    ));
    format!("ok frame {frame} sample {sample} updates {updates} free {free} late {late}")
}

/// Installs the capture when `FFX_AUDIO_WAV` names a file (a Windows path).
///
/// # Safety
/// `base` must be the main module; called before other threads run game
/// code, after the other import patches (`fmod_log`).
pub unsafe fn install(base: usize) {
    let path = std::env::var("FFX_AUDIO_WAV").unwrap_or_default();
    if path.is_empty() {
        // no file: the same mixer and turns without sound on the virtual
        // clock (module docs, "Without a file")
        let env = |name: &str| std::env::var(name).unwrap_or_default();
        let lockstep_game = !env("FFX_HARNESS").is_empty()
            && env("FFX_HARNESS_FAST") == "1"
            && env("FFX_HARNESS_VIRTUAL_CLOCK") == "1";
        if !lockstep_game || env("FFX_AUDIO_NRT") == "0" {
            return;
        }
        OUTPUT.store(OUTPUT_NOSOUND_NRT as u32, Ordering::Release);
    } else {
        let Ok(c_path) = CString::new(path.clone()) else {
            return;
        };
        match std::fs::File::create(format!("{path}.marks")) {
            Ok(mut f) => {
                let _ = writeln!(f, "# audiomarks 1 {path}");
                if let Ok(mut m) = MARKS.lock() {
                    *m = Some(f);
                }
            }
            Err(e) => log(&format!("  audio wav: cannot create {path}.marks: {e}")),
        }
        let _ = PATH.set(c_path);
    }
    // SAFETY: forwarded from the caller.
    let (init, update) = unsafe {
        (
            crate::iat::patch(
                base,
                "fmod_event.dll",
                INIT_NAME,
                event_system_init as *const () as usize,
            ),
            crate::iat::patch(
                base,
                "fmod_event.dll",
                UPDATE_NAME,
                event_system_update as *const () as usize,
            ),
        )
    };
    let (Some(init), Some(update)) = (init, update) else {
        log("  audio wav: EventSystem::init/update not imported; off");
        return;
    };
    INIT_ORIGINAL.store(init, Ordering::Release);
    UPDATE_ORIGINAL.store(update, Ordering::Release);
    let hooks = [crate::Hook {
        name: "FmodManager_tick",
        address: TICK,
        // push ebp; mov ebp, esp; sub esp, 8
        prologue: &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
        replacement: tick as *const () as usize,
        original: Some(&TICK_ORIGINAL),
        steal: 0,
    }];
    // SAFETY: forwarded from the caller (the FMOD thread is made later).
    let r = unsafe { crate::install(base, &hooks, &[]) };
    ON.store(true, Ordering::Release);
    if path.is_empty() {
        log(&format!(
            "  audio wav: no file, FMOD output NOSOUND_NRT on the virtual clock; FmodManager_tick turns {r:?}"
        ));
    } else {
        log(&format!(
            "  audio wav: FMOD output to {path} (WAVWRITER_NRT, PCMFLOAT); FmodManager_tick turns {r:?}"
        ));
    }
}
