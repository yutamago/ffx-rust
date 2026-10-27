//! The asset loader's turn at the render lock, in lockstep
//! (`docs/harness.md`, "Load pacing").
//!
//! `AsyncLoadManager`'s thread (`AsyncLoadManager_threadMain_00691360`)
//! loads textures under the render lock (`0x00c94ee4`, a recursive
//! `PCriticalSectionSimple`: owner thread `+0x10`, depth `+0x14`, taken by
//! `Phyre_PMutex_lockInner_00431850` with 20 ms event waits). The
//! render thread holds that lock while it presents, and in lockstep the
//! harness holds it there until the main thread leaves its frame boundary.
//! The loader therefore only got the lock in the gaps between the render
//! thread's three holds of a frame, and how many textures it loaded per
//! frame followed how long the main thread's update took against the render
//! thread's wake-up from its hold (1 ms sleeps): with the original's update
//! (about 0.8 ms) the next frame's commands were usually queued before the
//! render thread woke, and the loader got the lock in about every other
//! frame; the Rust update (about 2.5 ms) left it free for two or three
//! textures a frame. Valefor's 26 textures took 21-43 frames in the
//! original and 9-18 with the Rust replacements (frames slowed to 250 ms,
//! `examples/load_pacing.rs`), and the summon entry 282 or 281 frames.
//!
//! In lockstep the loader now gets the lock at one point of each frame
//! only, the same for the original code and for Rust: when the main thread
//! is about to leave a frame boundary and the loader has work, the render
//! thread is let out of its present hold (the script is done with that
//! update: its screenshots are taken), the render thread's next take of the
//! lock waits, and the loader runs until it has nothing left to do (its
//! request lists are empty) with the lock free. Then the main thread and
//! the render thread go on. While the main thread runs an update the loader
//! waits before taking the lock, unless the main thread stays in one update
//! for [`STALL`] (it waits for the loader there, as during map loads), in
//! which case the loader goes on until that update ends. Outside lockstep
//! nothing changes.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::log::log;

/// The render lock (`PRenderScopeLock_enter_00463d50`'s object).
const RENDER_LOCK: u32 = 0x00c9_4ee4;
/// The lock's owner word (a thread id, 0 when free).
const RENDER_LOCK_OWNER: u32 = RENDER_LOCK + 0x10;
/// `g_AsyncLoadManager`: `+0x20` and `+0x2c` count its two request lists.
const ASYNC_LOAD_MANAGER: u32 = 0x00cc_c88c;
/// An update this long is one waiting for the loader.
const STALL: Duration = Duration::from_millis(100);
/// How long a turn waits for one side before it gives up (logged).
const PATIENCE: Duration = Duration::from_secs(5);

/// Trampoline to `Phyre_PMutex_lockInner_00431850`.
pub static LOCK_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// Trampoline to `AsyncLoadManager_threadMain_00691360`.
pub static LOADER_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
/// The loader thread and the render thread.
static LOADER_TID: AtomicU32 = AtomicU32::new(0);
static RENDER_TID: AtomicU32 = AtomicU32::new(0);
/// A turn runs: the loader may take the lock, the render thread may not.
static TURN: AtomicBool = AtomicBool::new(false);
/// The render thread waits for the turn to end.
static RENDER_WAITING: AtomicBool = AtomicBool::new(false);
/// The update (`PASSED` value) the loader was let go in after a stall.
static STALL_FREED: AtomicU32 = AtomicU32::new(u32::MAX);
/// Stalls so far (logged).
static STALLS: AtomicU32 = AtomicU32::new(0);

static WAKE: Mutex<()> = Mutex::new(());
static CHANGED: Condvar = Condvar::new();

fn current_thread() -> u32 {
    // SAFETY: plain Win32 call.
    unsafe { windows_sys::Win32::System::Threading::GetCurrentThreadId() }
}

fn notify() {
    let _g = WAKE.lock();
    CHANGED.notify_all();
}

/// Waits until `done` or `limit` passes (rechecked at least every ms).
fn wait_until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    let Ok(mut g) = WAKE.lock() else {
        return false;
    };
    loop {
        if done() {
            return true;
        }
        if start.elapsed() > limit {
            return false;
        }
        g = match CHANGED.wait_timeout(g, Duration::from_millis(1)) {
            Ok((g, _)) => g,
            Err(_) => return false,
        };
    }
}

/// The render thread, as seen in `Present`.
pub fn note_render_thread() {
    RENDER_TID.store(current_thread(), Ordering::Relaxed);
}

/// Wraps the loader thread's body: notes which thread it is.
///
/// # Safety
/// Called as the thread's body, with whatever it is given.
pub unsafe extern "C" fn loader_thread(arg: usize) -> u32 {
    LOADER_TID.store(current_thread(), Ordering::Release);
    // SAFETY: the trampoline runs the original body.
    unsafe {
        let f: unsafe extern "C" fn(usize) -> u32 =
            std::mem::transmute(LOADER_ORIGINAL.load(Ordering::Acquire));
        f(arg)
    }
}

/// Wraps `Phyre_PMutex_lockInner_00431850(this; &owner)`
/// when the port (`ported/p1_load.rs`) is left out (`FFX_HOOK_DISABLE`,
/// `FFX_HOOK_ONLY`, original-code runs): [`before_take`], then the original
/// through its trampoline ([`install_fallback`]).
///
/// # Safety
/// Called by the game with a lock and its owner word.
pub unsafe extern "thiscall" fn lock_take(this: u32, owner: u32) -> u32 {
    before_take(this, owner);
    // SAFETY: the original through its trampoline.
    unsafe {
        let f: unsafe extern "thiscall" fn(u32, u32) -> u32 =
            std::mem::transmute(LOCK_ORIGINAL.load(Ordering::Acquire));
        f(this, owner)
    }
}

/// The turn rule at the top of `Phyre_PMutex_lockInner_00431850(this;
/// &owner)`, before the lock is taken: in lockstep the loader's and the
/// render thread's top-level takes of the render lock wait for their turn.
/// Its port calls this first, and so does [`lock_take`] around the
/// original. Anything else (another lock, outside lockstep, a thread that
/// already owns it) passes at once.
pub fn before_take(this: u32, owner: u32) {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    let render_lock = crate::rebase(RENDER_LOCK) as u32;
    if this != render_lock || !crate::harness::lockstep() {
        return;
    }
    let tid = current_thread();
    // SAFETY: the lock's owner word; only this thread writes its own id
    // there.
    let held = unsafe { std::ptr::read_volatile(owner as usize as *const u32) } == tid;
    if held {
        return;
    }
    if tid == LOADER_TID.load(Ordering::Acquire) {
        loader_wait();
    } else if tid == RENDER_TID.load(Ordering::Relaxed) && TURN.load(Ordering::Acquire) {
        RENDER_WAITING.store(true, Ordering::Release);
        notify();
        wait_until(PATIENCE, || !TURN.load(Ordering::Acquire));
        RENDER_WAITING.store(false, Ordering::Release);
    }
}

/// `push ebp; mov ebp, esp; push ecx; push ebx; push esi; push edi`: whole
/// instructions without relative operands, copied into the trampoline.
const LOCK_PROLOGUE: &[u8] = &[0x55, 0x8b, 0xec, 0x51, 0x53, 0x56, 0x57];

/// After the hook table is installed: when the port of
/// `Phyre_PMutex_lockInner_00431850` was left out, wraps the original with
/// [`lock_take`] instead, so original-code runs keep the turn rule (one hook
/// at the address either way: the port's own table entry, with the wrapper
/// in place of the port). Only with the harness.
///
/// # Safety
/// `base` is the main module; no other thread runs game code yet.
pub unsafe fn install_fallback(
    base: usize,
    hooks: &[crate::Hook],
    results: &[(&'static str, crate::Outcome)],
) {
    const NAME: &str = "Phyre_PMutex_lockInner_00431850";
    if !crate::harness::ENABLED.load(Ordering::Acquire) {
        return;
    }
    let Some(port) = hooks.iter().find(|h| h.name == NAME) else {
        return;
    };
    let installed = results
        .iter()
        .any(|(n, o)| *n == NAME && *o == crate::Outcome::Installed);
    if installed {
        return;
    }
    let wrapper = crate::Hook {
        prologue: LOCK_PROLOGUE,
        replacement: lock_take as *const () as usize,
        original: Some(&LOCK_ORIGINAL),
        steal: 0,
        ..*port
    };
    // SAFETY: forwarded from the caller; the port left the site untouched.
    let r = unsafe { crate::install(base, &[wrapper], &[]) };
    log(&format!(
        "  harness: loader turn around the original {NAME} (its port is off): {r:?}"
    ));
}

/// The loader before a top-level take: during a turn it goes on; while the
/// main thread runs an update it waits for the next turn, unless that
/// update stalls.
fn loader_wait() {
    let mut update = crate::harness::passed();
    let mut since = Instant::now();
    wait_until(Duration::MAX, || {
        if TURN.load(Ordering::Acquire) || !crate::harness::lockstep() {
            return true;
        }
        let now = crate::harness::passed();
        if STALL_FREED.load(Ordering::Acquire) == now && !crate::harness::at_boundary() {
            return true;
        }
        if now != update || crate::harness::at_boundary() {
            update = now;
            since = Instant::now();
        } else if since.elapsed() > STALL {
            STALL_FREED.store(now, Ordering::Release);
            let n = STALLS.fetch_add(1, Ordering::AcqRel);
            // a map load's update (the main thread in `CdReadSync`
            // `0x0076c490`, waiting for the read thread) is the usual one
            if n < 8 {
                log(&format!(
                    "  harness: loader freed in update {now} (stall {n})"
                ));
            }
            return true;
        }
        false
    });
}

/// Whether the loader has requests queued or running.
fn loader_busy() -> bool {
    // SAFETY: the manager's pointer and its two list counts, words of the
    // game's data (read racily: they are only compared with 0).
    unsafe {
        let manager = std::ptr::read_volatile(crate::rebase(ASYNC_LOAD_MANAGER) as *const u32);
        if manager == 0 {
            return false;
        }
        let at = |off: u32| std::ptr::read_volatile((manager + off) as usize as *const u32);
        at(0x20) != 0 || at(0x2c) != 0
    }
}

fn lock_owner() -> u32 {
    // SAFETY: the lock's owner word in the image's data.
    unsafe { std::ptr::read_volatile(crate::rebase(RENDER_LOCK_OWNER) as *const u32) }
}

/// At the main thread's frame boundary `frame`, as it is about to leave
/// (the script is done with this update): the loader's turn, when it has
/// work. `release_present` lets the render thread out of its present hold.
pub fn boundary_turn(frame: u32, release_present: impl FnOnce()) {
    if LOADER_TID.load(Ordering::Acquire) == 0 || !loader_busy() {
        return;
    }
    TURN.store(true, Ordering::Release);
    release_present();
    notify();
    let render = RENDER_TID.load(Ordering::Relaxed);
    // the render thread finishes its flip and lets the lock go, then waits
    // at its next take (or idles: the main thread queues nothing now)
    let freed = wait_until(PATIENCE, || {
        let owner = lock_owner();
        owner != render || RENDER_WAITING.load(Ordering::Acquire)
    });
    let drained = freed
        && wait_until(PATIENCE, || {
            !loader_busy() && lock_owner() != LOADER_TID.load(Ordering::Acquire)
        });
    if !drained {
        log(&format!(
            "  harness: loader turn at frame {frame}: {}",
            if freed {
                "the loader did not finish"
            } else {
                "the render thread kept the lock"
            }
        ));
    }
    TURN.store(false, Ordering::Release);
    notify();
}
