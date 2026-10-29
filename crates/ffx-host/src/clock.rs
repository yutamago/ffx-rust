//! The virtual clock and the frame boundary (docs/host.md, "The clock"):
//! the rules the hook's harness applies with `FFX_HARNESS_FAST=1`,
//! `FFX_HARNESS_VIRTUAL_CLOCK=1` and lockstep (docs/harness.md, "Frames and
//! threads", "Load pacing", "Sound in lockstep"), here always on, so a run
//! advances the same way whatever the host's speed.
//!
//! - **The clock.** Game time is [`VIRTUAL_START`] plus the time skipped,
//!   never the host's: `Phyre_PTimerWin32_GetTime` (`0x0042fd80`) returns
//!   it, `QueryPerformanceCounter` counts it at 10 MHz, `GetTickCount`,
//!   `GetTickCount64` and `timeGetTime` in ms. Only two sleeps move it: the
//!   frame limiter's `Sleep` (`Game_Main`'s `call [Sleep]` at
//!   `0x0062f3bb`) steps to the next frame's time exactly, and the
//!   start-up loop's `SleepThreadInMillisecond(33)` (`0x0066b345`, the
//!   logos) by its milliseconds. Every other sleep sleeps for real
//!   (other threads go on) and leaves the clock alone.
//! - **The boundary.** `FFXApplication_animate` (`0x0042f520`, the update)
//!   is wrapped: its entry is frame boundary `n` (`n` updates done). There
//!   the replay script runs ([`crate::replay`]), the asset loader gets its
//!   turn, the PS2 vsync handler runs once per vsync of game time, and the
//!   `FmodManager` thread gets its tick; then the update runs.
//! - **The vsync thread** (`createVsyncEmuThread` `0x00679e60`) is not
//!   started: its handler runs at the boundary instead.
//! - **Reads** (`FileSys_SubmitRead` `0x0061c220`) run on the calling
//!   thread at once, as on the virtual clock in the hook.
//! - **The loader's turn** (`crate::clock::loader_wait`): the asset loader
//!   (`AsyncLoadManager_threadMain_00691360`) takes the render lock
//!   (`0x00c94ee4`) only in its turn, which the boundary gives while its
//!   request lists are not empty and waits out; the render thread's next
//!   take of the lock waits meanwhile. While an update runs the loader
//!   waits, unless the main thread sleeps inside it (`CdReadSync` waiting
//!   for a read) or the update takes longer than [`STALL`].
//! - **FMOD's turn**: `FmodManager_tick` (`0x006fa700`) runs once per
//!   boundary, when the main thread gives the turn, and serves the
//!   synchronous commands the main thread waits for in between.
//! - **The opening logos** (`OpeningScreen.swf`) end at their first
//!   frame, as in fast mode: the flip clears `g_PhyreSceneOpeningPlaying`
//!   (`0x00ccb9c2`).
//! - **Input**: `Input_KeyHeld` (`0x00629d50`) is wrapped so the buttons
//!   the script holds read as their first keyboard binding.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use ffx_bridge::Mem as _;

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Ret, Table};

/// Where the virtual clock starts, in seconds: the hook's (2^20 s).
pub const VIRTUAL_START: f64 = 1_048_576.0;
/// One frame of the limiter at two vsyncs a frame.
pub const FRAME: f64 = 2.0 * 0.016_683_333_333_333_335;
/// An update that runs this long (host time) frees the loader inside it
/// (the hook's 100 ms, scaled for the emulator).
const STALL: Duration = Duration::from_secs(3);
/// How long a turn is waited for before the boundary gives up.
const PATIENCE: Duration = Duration::from_secs(120);
/// How long the boundary waits for the render thread to let the lock go.
const LOCK_PATIENCE: Duration = Duration::from_secs(20);

/// `Game_Main`'s limiter `call [Sleep]` returns here.
const LIMITER_RETURN: u32 = 0x0062_f3c1;
/// `PhyreScene_renderOpeningScreen`'s `call SleepThreadInMillisecond`
/// returns here.
const BOOT_SLEEP_RETURN: u32 = 0x0066_b34a;
/// `g_AppStartTime`, `g_FrameLimiterLast` (`f64`), the limiter's vsyncs a
/// frame (`u32`).
const APP_START_TIME: u32 = 0x00c9_0278;
const FRAME_LIMITER_LAST: u32 = 0x00cc_9d00;
const FRAME_VSYNCS: u32 = 0x00c3_0e88;
/// `g_PhyreSceneOpeningPlaying`.
const OPENING_PLAYING: u32 = 0x00cc_b9c2;
/// The render lock and its owner word.
const RENDER_LOCK: u32 = 0x00c9_4ee4;
const RENDER_LOCK_OWNER: u32 = RENDER_LOCK + 0x10;
/// `g_AsyncLoadManager`.
const ASYNC_LOAD_MANAGER: u32 = 0x00cc_c88c;
/// `g_FmodCommandHandlers`: 70 cdecl `handler(FfxFmod*, u32 args[4])`.
const FMOD_HANDLERS: u32 = 0x00c3_a368;
/// The game's input object: `+0x9c` counts the (binding, pad bit) pairs
/// at `*(+0xa4)`.
const INPUT: u32 = 0x00cc_b170;

/// What the clock and the boundary share across the game's threads.
#[derive(Debug, Default)]
pub struct Clock {
    /// Seconds skipped (`f64` bits).
    skipped: AtomicU64,
    /// Frame boundaries entered (updates begun).
    frame: AtomicU32,
    /// The main thread is at a boundary (between two updates).
    at_boundary: AtomicBool,
    /// Lockstep: from the main loop's first boundary on (before it, the
    /// start-up loop's loads run free, as before the replays' `lockstep
    /// on`).
    lockstep: AtomicBool,
    /// The main thread sleeps (inside an update).
    main_sleeping: AtomicBool,
    /// When the running update started (host time), for [`STALL`].
    update_started: Mutex<Option<Instant>>,
    /// The vsync handler and the vsyncs run so far.
    vsync: AtomicU32,
    vsync_ticks: AtomicU64,
    /// The render and loader threads.
    render: AtomicU32,
    loader: AtomicU32,
    /// The loader's turn, and the render thread waiting for its end.
    turn: AtomicBool,
    render_waiting: AtomicBool,
    loader_turns: AtomicU32,
    loader_freed: AtomicU32,
    /// `g_FmodManager`, the FMOD turns asked and done.
    fmod: AtomicU32,
    fmod_on: AtomicBool,
    fmod_asked: AtomicU64,
    fmod_done: AtomicU64,
    /// PS2 buttons the script holds.
    pub buttons: AtomicU32,
    /// Flips run, and the update each shows (the main thread's frame when
    /// the render thread ran it).
    flips: AtomicU32,
    /// The application object `FFXApplication_animate` runs on.
    pub app: AtomicU32,
    /// The replay script (`--replay`).
    pub script: Mutex<Option<crate::replay::Script>>,
    wake: Mutex<()>,
    changed: Condvar,
}

impl Clock {
    /// The game time now, in seconds.
    #[must_use]
    pub fn now(&self) -> f64 {
        VIRTUAL_START + f64::from_bits(self.skipped.load(Ordering::Acquire))
    }

    /// Moves the clock on by `seconds`.
    pub fn skip(&self, seconds: f64) {
        let mut bits = self.skipped.load(Ordering::Acquire);
        loop {
            let next = (f64::from_bits(bits) + seconds).to_bits();
            match self
                .skipped
                .compare_exchange(bits, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => break,
                Err(b) => bits = b,
            }
        }
    }

    /// Sets the time skipped (the script's `pin`).
    pub fn set_skipped(&self, seconds: f64) {
        self.skipped.store(seconds.to_bits(), Ordering::Release);
        self.vsync_ticks
            .store(self.vsync_ticks_now(), Ordering::Release);
    }

    /// Frame boundaries entered so far.
    #[must_use]
    pub fn frame(&self) -> u32 {
        self.frame.load(Ordering::Acquire)
    }

    /// Flips the render thread ran.
    #[must_use]
    pub fn flips(&self) -> u32 {
        self.flips.load(Ordering::Acquire)
    }

    /// The loader's turns given, and the times an update freed it.
    #[must_use]
    pub fn loader_turns(&self) -> (u32, u32) {
        (
            self.loader_turns.load(Ordering::Acquire),
            self.loader_freed.load(Ordering::Acquire),
        )
    }

    /// FMOD turns done.
    #[must_use]
    pub fn fmod_turns(&self) -> u64 {
        self.fmod_done.load(Ordering::Acquire)
    }

    fn vsync_ticks_now(&self) -> u64 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive seconds
        let t = (self.now() * 59.94) as u64;
        t
    }

    fn notify(&self) {
        let _g = self.wake.lock();
        self.changed.notify_all();
    }

    /// Waits (host time, at most `limit`) until `done`, waking on every
    /// change and every millisecond; false on time out or when the run
    /// ends.
    fn wait_until(&self, e: &Exec, limit: Duration, mut done: impl FnMut() -> bool) -> bool {
        let start = Instant::now();
        let Ok(mut g) = self.wake.lock() else {
            return false;
        };
        loop {
            if done() {
                return true;
            }
            if start.elapsed() > limit || e.shared().overdue() {
                return false;
            }
            g = match self.changed.wait_timeout(g, Duration::from_millis(1)) {
                Ok((g, _)) => g,
                Err(_) => return false,
            };
        }
    }
}

fn clock(e: &Exec) -> &Clock {
    &e.shared().clock
}

fn caller(e: &Exec, args: &Args) -> u32 {
    if args.slots == 0 {
        0
    } else {
        e.mem().read::<u32>(args.slots - 4)
    }
}

// ---- the time imports --------------------------------------------------

/// `QueryPerformanceCounter(&n)`: the clock at 10 MHz.
pub fn perf_counter(e: &mut Exec, args: &Args) -> Ret {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive
    let n = (clock(e).now() * 1e7).round() as u64;
    e.mem().write(args.stack.first().copied().unwrap_or(0), n);
    Ret::eax(1)
}

/// `QueryPerformanceFrequency(&n)`: 10 MHz.
pub fn perf_frequency(e: &mut Exec, args: &Args) -> Ret {
    e.mem()
        .write(args.stack.first().copied().unwrap_or(0), 10_000_000u64);
    Ret::eax(1)
}

fn millis(e: &Exec) -> u64 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive
    let ms = (clock(e).now() * 1000.0) as u64;
    ms
}

/// `GetTickCount()` / `timeGetTime()`: the clock in ms.
pub fn tick_count(e: &mut Exec, _: &Args) -> Ret {
    #[allow(clippy::cast_possible_truncation)] // wraps as Windows' does
    Ret::eax(millis(e) as u32)
}

/// `GetTickCount64()`.
pub fn tick_count64(e: &mut Exec, _: &Args) -> Ret {
    Ret::u64(millis(e))
}

/// `Sleep(ms)` on the virtual clock: the frame limiter's steps the clock
/// to the next frame's time (returns true: done); any other is left to
/// the real sleep (false). Notes the main thread sleeping, for the
/// loader's turn.
pub fn sleep(e: &mut Exec, args: &Args) -> bool {
    let ms = args.stack.first().copied().unwrap_or(0);
    if caller(e, args) == LIMITER_RETURN {
        limiter_sleep(e, ms);
        return true;
    }
    if e.is_main() {
        clock(e).main_sleeping.store(true, Ordering::Release);
        clock(e).notify();
    }
    false
}

/// After a real sleep (the main thread is awake again).
pub fn woke(e: &Exec) {
    if e.is_main() {
        clock(e).main_sleeping.store(false, Ordering::Release);
    }
}

/// The limiter's `Sleep(ms)` (`Game_Main` `0x0062f2a0`..`0x0062f3bb`): the
/// clock moves to the frame's time (start + last frame + vsyncs x
/// 1/59.94 s) and a little past it (the limiter compares in 80 bits), as
/// the hook's `fast_sleep` does.
fn limiter_sleep(e: &Exec, ms: u32) {
    let mem = e.mem();
    let start: f64 = mem.read(APP_START_TIME);
    let last: f64 = mem.read(FRAME_LIMITER_LAST);
    let count: u32 = mem.read(FRAME_VSYNCS);
    let due = start + last + f64::from(count) * 0.016_683_333_333_333_335;
    let c = clock(e);
    let ahead = (due - c.now()).max(0.0) + 1e-9;
    if ahead < 1.0 {
        c.skip(ahead);
    } else {
        c.skip(f64::from(ms.max(1)) / 1000.0);
    }
}

// ---- the ports and wrappers ----------------------------------------------

/// `Phyre_PTimerWin32_GetTime()` (`0x0042fd80`): the virtual clock, in
/// `ST0`.
fn get_time(e: &mut Exec, _: &Args) -> Ret {
    Ret::st0(clock(e).now())
}

/// `SleepThreadInMillisecond(ms)` (`0x006f2d20`, cdecl, returns 0): the
/// start-up loop's 33 ms (from `0x0066b345`) moves the clock instead of
/// sleeping; any other sleeps for real.
fn sleep_ms(e: &mut Exec, args: &Args) -> Ret {
    let ms = args.stack.first().copied().unwrap_or(0);
    if caller(e, args) == BOOT_SLEEP_RETURN {
        clock(e).skip(f64::from(ms.max(1)) / 1000.0);
        return Ret::eax(0);
    }
    let sleep = Args {
        stack: vec![ms],
        ..Args::default()
    };
    crate::win32::real_sleep(e, &sleep);
    Ret::eax(0)
}

/// `FFXApplication_animate(app)` (`0x0042f520`, thiscall, eax): the frame
/// boundary, then the update.
fn animate(e: &mut Exec, args: &Args) -> Ret {
    let c = clock(e);
    let frame = c.frame();
    c.app.store(args.ecx, Ordering::Release);
    c.lockstep.store(true, Ordering::Release);
    c.at_boundary.store(true, Ordering::Release);
    c.notify();
    crate::replay::boundary(e, frame);
    loader_turn(e, frame);
    run_vsync(e);
    fmod_turn(e);
    let c = clock(e);
    c.at_boundary.store(false, Ordering::Release);
    if let Ok(mut s) = c.update_started.lock() {
        *s = Some(Instant::now());
    }
    c.notify();
    let r = e.call_original(0x0042_f520, Conv::Thiscall, args);
    clock(e).frame.store(frame + 1, Ordering::Release);
    r.unwrap_or(Ret::eax(0))
}

/// `Renderer_PresentFlip(renderer)` (`0x00593760`, thiscall, bool): notes
/// the render thread; ends the opening logos on their first frame; before
/// the first update runs the vsyncs (no boundary yet).
fn present_flip(e: &mut Exec, args: &Args) -> Ret {
    let c = clock(e);
    c.render.store(e.id, Ordering::Release);
    let r = e.call_original(0x0059_3760, Conv::Thiscall, args);
    let c = clock(e);
    c.flips.fetch_add(1, Ordering::AcqRel);
    let mut mem = e.mem();
    if mem.read::<u8>(OPENING_PLAYING) != 0 {
        mem.write(OPENING_PLAYING, 0u8);
        crate::log::note(&format!(
            "the opening logos end at flip {} (fast mode)",
            c.flips()
        ));
    }
    if c.frame() == 0 {
        run_vsync(e);
    }
    r.unwrap_or(Ret::eax(0))
}

/// `createVsyncEmuThread(handler)` (`0x00679e60`, cdecl): no thread; the
/// handler runs at the boundary as game time passes.
fn vsync_start(e: &mut Exec, args: &Args) -> Ret {
    let c = clock(e);
    c.vsync_ticks.store(c.vsync_ticks_now(), Ordering::Release);
    c.vsync
        .store(args.stack.first().copied().unwrap_or(0), Ordering::Release);
    Ret::eax(1)
}

/// `endVsyncEmuThread()` (`0x00679f20`).
fn vsync_end(e: &mut Exec, _: &Args) -> Ret {
    clock(e).vsync.store(0, Ordering::Release);
    Ret::eax(0)
}

/// `VsyncEmu_Wake()` (`0x00679f80`): no thread to wake.
fn vsync_wake(_: &mut Exec, _: &Args) -> Ret {
    Ret::VOID
}

/// Calls the vsync handler once per vsync of game time since the last.
fn run_vsync(e: &mut Exec) {
    let c = clock(e);
    let cb = c.vsync.load(Ordering::Acquire);
    if cb == 0 {
        return;
    }
    let now = c.vsync_ticks_now();
    let mut done = c.vsync_ticks.load(Ordering::Acquire);
    while done < now {
        if e.call_cdecl(cb, &[]).is_err() {
            return;
        }
        done += 1;
    }
    clock(e).vsync_ticks.store(done, Ordering::Release);
}

/// `FileSys_SubmitRead(request)` (`0x0061c220`, cdecl): the read and its
/// decompression on the calling thread at once
/// (`BigFileStream_fileTaskRead`, `BigFileStream_fileTaskUncompress`).
fn submit_read(e: &mut Exec, args: &Args) -> Ret {
    let request = args.stack.first().copied().unwrap_or(0);
    if e.call_stdcall(0x0061_b8d0, &[0, request, 0]).is_ok() {
        let _ = e.call_stdcall(0x0061_ba80, &[0, request, 0, 0]);
    }
    Ret::VOID
}

/// `AsyncLoadManager_threadMain_00691360(arg)` (cdecl): notes the
/// loader's thread, then runs it.
fn loader_thread(e: &mut Exec, args: &Args) -> Ret {
    clock(e).loader.store(e.id, Ordering::Release);
    e.call_original(0x0069_1360, Conv::Cdecl, args)
        .unwrap_or(Ret::eax(0))
}

/// `Phyre_PMutex_lockInner_00431850(this; &owner)` (thiscall): the
/// loader's turn rule before a take of the render lock.
fn lock_inner(e: &mut Exec, args: &Args) -> Ret {
    before_take(e, args.ecx, args.stack.first().copied().unwrap_or(0));
    e.call_original(0x0043_1850, Conv::Thiscall, args)
        .unwrap_or(Ret::eax(0))
}

fn before_take(e: &Exec, this: u32, owner: u32) {
    let c = clock(e);
    if this != RENDER_LOCK || !c.lockstep.load(Ordering::Acquire) {
        return;
    }
    if e.mem().read::<u32>(owner) == e.id {
        return;
    }
    if e.id == c.loader.load(Ordering::Acquire) {
        loader_wait(e);
    } else if e.id == c.render.load(Ordering::Acquire) && c.turn.load(Ordering::Acquire) {
        c.render_waiting.store(true, Ordering::Release);
        c.notify();
        c.wait_until(e, PATIENCE, || !c.turn.load(Ordering::Acquire));
        c.render_waiting.store(false, Ordering::Release);
    }
}

/// The loader before a take of the render lock: waits for its turn,
/// unless the main thread sleeps inside an update or the update takes
/// longer than [`STALL`].
fn loader_wait(e: &Exec) {
    let c = clock(e);
    let mut freed = false;
    c.wait_until(e, Duration::MAX, || {
        if c.turn.load(Ordering::Acquire) {
            return true;
        }
        if c.at_boundary.load(Ordering::Acquire) {
            return false;
        }
        let long = c
            .update_started
            .lock()
            .ok()
            .and_then(|s| *s)
            .is_some_and(|s| s.elapsed() > STALL);
        if c.main_sleeping.load(Ordering::Acquire) || long {
            freed = true;
            return true;
        }
        false
    });
    if freed {
        let n = c.loader_freed.fetch_add(1, Ordering::AcqRel);
        if n < 20 {
            crate::log::note(&format!(
                "the loader freed inside update {} (the main thread sleeps or stalls)",
                c.frame()
            ));
        }
    }
}

fn loader_busy(e: &Exec) -> bool {
    let mem = e.mem();
    let m: u32 = mem.read(ASYNC_LOAD_MANAGER);
    m != 0 && (mem.read::<u32>(m + 0x20) != 0 || mem.read::<u32>(m + 0x2c) != 0)
}

/// At the boundary: when the loader has work, its turn, waited out.
fn loader_turn(e: &Exec, frame: u32) {
    let c = clock(e);
    let loader = c.loader.load(Ordering::Acquire);
    if loader == 0 || !loader_busy(e) {
        return;
    }
    c.loader_turns.fetch_add(1, Ordering::AcqRel);
    c.turn.store(true, Ordering::Release);
    c.notify();
    let render = c.render.load(Ordering::Acquire);
    let owner = || e.mem().read::<u32>(RENDER_LOCK_OWNER);
    let free = c.wait_until(e, LOCK_PATIENCE, || {
        owner() != render || render == 0 || c.render_waiting.load(Ordering::Acquire)
    });
    let drained = free && c.wait_until(e, PATIENCE, || !loader_busy(e) && owner() != loader);
    if !drained {
        crate::log::note(&format!(
            "loader turn at frame {frame}: {}",
            if free {
                "the loader did not finish"
            } else {
                "the render thread kept the lock"
            }
        ));
    }
    c.turn.store(false, Ordering::Release);
    c.notify();
}

// ---- FMOD's turn -------------------------------------------------------------

/// `FmodManager_tick(mgr)` (`0x006fa700`, thiscall): from the first
/// boundary on, runs once per turn the main thread gives, serving the
/// synchronous commands in between.
fn fmod_tick(e: &mut Exec, args: &Args) -> Ret {
    let manager = args.ecx;
    let c = clock(e);
    c.fmod.store(manager, Ordering::Release);
    if c.frame() > 0 {
        c.fmod_on.store(true, Ordering::Release);
        fmod_wait_for_turn(e, manager);
    }
    let on = clock(e).fmod_on.load(Ordering::Acquire);
    if on {
        // the tick's own wait (less than a period since the last
        // soundLoop) returns at once
        let ev = e.mem().read::<u32>(manager + 0x18);
        e.shared().sync.set_event(ev, true);
    }
    let r = e.call_original(0x006f_a700, Conv::Thiscall, args);
    let c = clock(e);
    if on {
        c.fmod_done
            .store(c.fmod_asked.load(Ordering::Acquire), Ordering::Release);
        c.notify();
    }
    r.unwrap_or(Ret::eax(0))
}

fn fmod_wait_for_turn(e: &mut Exec, manager: u32) {
    loop {
        let c = clock(e);
        if c.fmod_asked.load(Ordering::Acquire) > c.fmod_done.load(Ordering::Acquire)
            || e.shared().overdue()
        {
            return;
        }
        let mem = e.mem();
        // the thread is asked to stop (`FmodManager_threadMain`'s loop)
        if mem.read::<u8>(manager + 0x4c) == 0 {
            return;
        }
        if mem.read::<u8>(manager + 0x44) == 0 {
            serve_sync(e, manager);
        }
        let ev = e.mem().read::<u32>(manager + 0x18);
        let id = e.id;
        e.shared().sync.wait(id, &[ev], false, 50);
        e.shared().sync.set_event(ev, false);
    }
}

/// Runs the pending synchronous command alone, as the queue run
/// (`0x006fa7e0`) does first: `handler[+0x30](+8, +0x34)` into `+0x48`,
/// `+0x44` = 1, `SetEvent(+0x1c)`.
fn serve_sync(e: &mut Exec, manager: u32) {
    let mem = e.mem();
    let command: u32 = mem.read(manager + 0x30);
    if command >= 70 {
        return;
    }
    let fmod: u32 = mem.read(manager + 8);
    let handler: u32 = mem.read(FMOD_HANDLERS + 4 * command);
    let Ok(result) = e.call_cdecl(handler, &[fmod, manager + 0x34]) else {
        return;
    };
    let mut mem = e.mem();
    mem.write(manager + 0x48, result);
    mem.write(manager + 0x44, 1u8);
    let ev: u32 = mem.read(manager + 0x1c);
    e.shared().sync.set_event(ev, true);
}

/// At the boundary: the `FmodManager` thread's tick, waited for.
fn fmod_turn(e: &Exec) {
    let c = clock(e);
    let manager = c.fmod.load(Ordering::Acquire);
    if !c.fmod_on.load(Ordering::Acquire) || manager == 0 {
        return;
    }
    let asked = c.fmod_asked.fetch_add(1, Ordering::AcqRel) + 1;
    let ev = e.mem().read::<u32>(manager + 0x18);
    e.shared().sync.set_event(ev, true);
    if !c.wait_until(e, PATIENCE, || c.fmod_done.load(Ordering::Acquire) >= asked) {
        crate::log::note(&format!("the FMOD turn of frame {} did not end", c.frame()));
    }
}

// ---- input ---------------------------------------------------------------

/// `Input_KeyHeld(devices; binding, device)` (`0x00629d50`, thiscall):
/// for each button the script holds, the first keyboard binding (≥ 28)
/// paired with it reads as held.
fn key_held(e: &mut Exec, args: &Args) -> Ret {
    let held = e
        .call_original(0x0062_9d50, Conv::Thiscall, args)
        .map_or(0, |r| r.eax);
    let buttons = clock(e).buttons.load(Ordering::Acquire);
    let binding = args.stack.first().copied().unwrap_or(0);
    let device = args.stack.get(1).copied().unwrap_or(0);
    if buttons == 0 || device != 0 || binding < 28 {
        return Ret::eax(held);
    }
    let mem = e.mem();
    let count = mem.read::<u32>(INPUT + 0x9c).min(256);
    let pairs: u32 = mem.read(INPUT + 0xa4);
    if pairs == 0 {
        return Ret::eax(held);
    }
    let pair = |k: u32| -> (u32, u32) { (mem.read(pairs + 8 * k), mem.read(pairs + 8 * k + 4)) };
    let bound = (0..32).any(|bit| {
        let bit = 1u32 << bit;
        buttons & bit != 0
            && (0..count)
                .map(pair)
                .find(|&(b, p)| b >= 28 && p == bit)
                .is_some_and(|(b, _)| b == binding)
    });
    Ret::eax(if bound { (held & !0xff) | 1 } else { held })
}

/// Registers the clock's ports and wrappers.
pub fn register(t: &mut Table) {
    let entry = |name, conv, words, port| Entry {
        name,
        conv,
        words,
        port,
    };
    t.register(
        0x0042_fd80,
        entry("Phyre_PTimerWin32_GetTime", Conv::Cdecl, 0, get_time),
    );
    t.register(
        0x006f_2d20,
        entry("SleepThreadInMillisecond", Conv::Cdecl, 1, sleep_ms),
    );
    t.register(
        0x0061_c220,
        entry("FileSys_SubmitRead", Conv::Cdecl, 1, submit_read),
    );
    t.register(
        0x0067_9e60,
        entry("createVsyncEmuThread", Conv::Cdecl, 1, vsync_start),
    );
    t.register(
        0x0067_9f20,
        entry("endVsyncEmuThread", Conv::Cdecl, 0, vsync_end),
    );
    t.register(
        0x0067_9f80,
        entry("VsyncEmu_Wake", Conv::Cdecl, 0, vsync_wake),
    );
    // cmp byte [0x00cc9cf5], 0; push esi; mov esi, ecx
    t.wrap(
        0x0042_f520,
        entry("FFXApplication_animate", Conv::Thiscall, 0, animate),
        &[0x80, 0x3d, 0xf5, 0x9c, 0xcc, 0x00, 0x00, 0x56, 0x8b, 0xf1],
    );
    // push esi; mov esi, ecx; mov edx, [esi + 0x74]
    t.wrap(
        0x0059_3760,
        entry("Renderer_PresentFlip", Conv::Thiscall, 0, present_flip),
        &[0x56, 0x8b, 0xf1, 0x8b, 0x56, 0x74],
    );
    // push ebp; mov ebp, esp; mov eax, [g_AsyncLoadManager]
    t.wrap(
        0x0069_1360,
        entry(
            "AsyncLoadManager_threadMain_00691360",
            Conv::Cdecl,
            1,
            loader_thread,
        ),
        &[0x55, 0x8b, 0xec, 0xa1, 0x8c, 0xc8, 0xcc, 0x00],
    );
    // push ebp; mov ebp, esp; push ecx; push ebx; push esi; push edi
    t.wrap(
        0x0043_1850,
        entry(
            "Phyre_PMutex_lockInner_00431850",
            Conv::Thiscall,
            1,
            lock_inner,
        ),
        &[0x55, 0x8b, 0xec, 0x51, 0x53, 0x56, 0x57],
    );
    // push ebp; mov ebp, esp; sub esp, 8
    t.wrap(
        0x006f_a700,
        entry("FmodManager_tick", Conv::Thiscall, 0, fmod_tick),
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
    );
    // push ebp; mov ebp, esp; mov eax, [ecx + 0x2b8]
    t.wrap(
        0x0062_9d50,
        entry("Input_KeyHeld", Conv::Thiscall, 2, key_held),
        &[0x55, 0x8b, 0xec, 0x8b, 0x81, 0xb8, 0x02, 0x00, 0x00],
    );
}
