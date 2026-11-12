//! The test harness inside `ffx-rust` (docs/harness.md, "Two processes"):
//! the protocol `ffx-hook`'s harness serves inside `FFX.exe`
//! (`crates/ffx-hook/src/harness.rs`), served from `ffx-host`, so
//! `ffx-harness` drives both programs the same way.
//!
//! With `FFX_HARNESS=<port>` a thread listens on `127.0.0.1:<port>`, one
//! command a line, one reply a line (`ok …` / `err …`), `TCP_NODELAY` and
//! each reply in one write, as the hook's `serve`. The environment the
//! hook reads means the same here:
//!
//! - `FFX_HARNESS_BACKGROUND=1`: the window is never activated and stays at
//!   the bottom of the window stack, the mouse is the test's
//!   ([`crate::win_forward`]);
//! - `FFX_HARNESS_HOME=<folder>`: the game's Documents folder (settings,
//!   saves), the program's `--documents`;
//! - `FFX_HARNESS_SEED=<n>`: `Rng_ClockSeed` (`0x00798950`) returns `n`.
//!
//! **Frames are updates**, as in the hook: frame `n` is the clock's frame
//! boundary after `n` updates (the entry of `FFXApplication_animate`,
//! [`crate::clock`]), on the main thread. There the trace samples, the
//! main thread's jobs run (`call`, `wait`, `press`'s release, `waitfor`,
//! `shotwhen`, `holdfor`, `loadsave`, `pin`), and lockstep holds the game
//! until the script allows more updates ([`boundary`]). The flips are
//! labelled with the update they show (`Phyre_PRendering_PRenderer_flip`
//! `0x005b6af0`, wrapped), and in lockstep the render thread holds the
//! flip of update `n` (`Renderer_PresentFlip`, [`before_present_flip`])
//! until the main thread has left boundary `n`, so a `shot` asked for there
//! is that image (`ok <w> <h> update <n>`).
//!
//! | Command | Here |
//! |---|---|
//! | `ping`, `stats`, `frame` | the clock's frame; `stats` also counts presents, flips queued and run |
//! | `wait`, `press`, `key`, `waitfor`, `shotwhen`, `holdfor`, `lockstep`, `call`, `callnext`, `pin`, `loadsave`, `trace`, `events` | as the hook's, at the clock's boundary |
//! | `devsave <n> [lockstep]` | as the hook's: developer save `n` through the original loader and the archive reader under `debugLoadForViewer` ([`crate::devsave`]) |
//! | `read`, `readp`, `readabs`, `write`, `writep` | arena memory (a game address is an offset into the arena) |
//! | `keys`, `key` | the `DirectInput` keyboard's state (`GetDeviceState`) |
//! | `pad`, `press` | the buttons the clock's `Input_KeyHeld` wrapper answers for |
//! | `shot <path>` | the back buffer of the update the main thread stands after, as a 32-bit BMP |
//! | `gfxcapture <dir>` | the layer's recording so far (`capture.ffxgfx`, `calls.txt`) |
//! | `notes` | the run's notes so far (`crate::log::note`: turns, failed threads), as `events` lists |
//! | `quit` | ends the process (after writing the `--profile` file: [`profile_at_quit`]) |
//!
//! `audiomark <name>` notes the frame and the mixer's sample clock in the
//! marks file of the `FFX_AUDIO_WAV` capture (`crate::audio::Audio::mark`;
//! docs/harness.md, "Audio capture"), as the hook's. Not served
//! (answered `err`): `snapshot`, `fpucw`, the
//! overlay and live-editor commands, `cursor`, `threads`, `stacks`.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ffx_bridge::Mem;

use crate::arena::ArenaMem;
use crate::exec::{Exec, Shared};
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

static BACKGROUND: OnceLock<bool> = OnceLock::new();
static PRESENTS: AtomicU32 = AtomicU32::new(0);
static KEY_POLLS: AtomicU32 = AtomicU32::new(0);
/// Keys the test holds (`DirectInput` scan codes).
static KEYS: Mutex<[bool; 256]> = Mutex::new([false; 256]);
static ON: AtomicBool = AtomicBool::new(false);

/// Lockstep: the main thread holds at boundary `n` while `n >= ALLOWED`.
static LOCKSTEP: AtomicBool = AtomicBool::new(false);
/// `lockstep on` asked: the next boundary takes hold.
static LOCKSTEP_REQUEST: AtomicBool = AtomicBool::new(false);
static ALLOWED: AtomicU32 = AtomicU32::new(0);
/// Boundaries entered and left, as the newest one's frame plus 1 (0: none).
static REACHED: AtomicU32 = AtomicU32::new(0);
static PASSED: AtomicU32 = AtomicU32::new(0);
/// `shotwhen` shots not yet taken: the presents hold as in lockstep.
static PINNED: AtomicU32 = AtomicU32::new(0);

/// The label of a present whose flip was not seen.
const NO_LABEL: u32 = u32::MAX;
/// Flips queued (main thread) and run (render thread), and the update each
/// queued flip shows, by flip number (the render ring has 16 slots).
/// Ctrl work 0's event frame count (`0x01325d94`) at the last frame
/// boundary, for [`seeded_job`] (the hook's `JOB_EVENTS`).
static JOB_EVENTS: AtomicU32 = AtomicU32::new(0);
/// Seeded jobs run (cloth, middle calc), for `stats`.
static SEEDED_JOBS: AtomicU32 = AtomicU32::new(0);
static FLIPS: AtomicU32 = AtomicU32::new(0);
static FLIPS_RUN: AtomicU32 = AtomicU32::new(0);
/// Flips whose `Renderer_PresentFlip` returned ([`after_present_flip`]).
static FLIPS_DONE: AtomicU32 = AtomicU32::new(0);
/// Times a cloth job waited for the render thread ([`cloth_order`]).
static CLOTH_WAITS: AtomicU32 = AtomicU32::new(0);
static FLIP_LABELS: [AtomicU32; 64] = [const { AtomicU32::new(NO_LABEL) }; 64];
/// The update the render thread's current flip shows.
static PRESENT_LABEL: AtomicU32 = AtomicU32::new(NO_LABEL);
/// How long a present waits for the main thread to reach its boundary
/// before it goes on (the hook's 5 s, for the emulator).
const PRESENT_PATIENCE: Duration = Duration::from_secs(60);

/// Work for the main thread at a frame boundary.
enum Job {
    /// Reply at this frame.
    WaitUntil(u32),
    /// Release the buttons and keys at the first frame, reply at the second.
    Release(u32, u32),
    Call(u32, Vec<u32>),
    Pin,
    /// `loadsave <save> [lockstep]`.
    LoadSave(String, bool),
    /// `devsave <n> [lockstep]` ([`crate::devsave`]).
    DevSave(i32, bool),
    /// Until the range matches or the limit frame.
    WaitFor(Range, Vec<u8>, Vec<u8>, u32),
    /// As `WaitFor`, then the image of that update; the first boundary it
    /// was checked at.
    ShotWhen(Range, Vec<u8>, Vec<u8>, u32, String, Option<u32>),
    /// In lockstep: no update until the range matches or the deadline.
    HoldFor(Range, Vec<u8>, Vec<u8>, Instant),
}

static JOBS: Mutex<VecDeque<(Job, Sender<String>)>> = Mutex::new(VecDeque::new());
/// `callnext`: at the start of the next update.
#[allow(clippy::type_complexity)] // one queue of plain tuples
static UPDATE_CALLS: Mutex<VecDeque<(u32, Vec<u32>, Sender<String>)>> = Mutex::new(VecDeque::new());

/// A screenshot waiting for a present: the path, the first update it may
/// show, whether it must show exactly that one (`shotwhen`), the reply.
struct Shot {
    path: String,
    want: u32,
    pinned: bool,
    reply: Sender<String>,
}

static SHOTS: Mutex<Vec<Shot>> = Mutex::new(Vec::new());

/// A memory range of the trace syntax: `addr:len`, `addr+off:len`,
/// `*addr+off:len` (through the pointer at `addr`).
#[derive(Debug, Clone, Copy)]
struct Range {
    addr: u32,
    deref: bool,
    off: u32,
    len: u32,
}

struct Trace {
    out: std::io::BufWriter<std::fs::File>,
    ranges: Vec<Range>,
}

static TRACE: Mutex<Option<Trace>> = Mutex::new(None);
/// `open <file>` events (the archive paths the game opens), for `events`.
static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Background mode (`FFX_HARNESS_BACKGROUND=1`).
pub fn background() -> bool {
    *BACKGROUND.get_or_init(|| std::env::var("FFX_HARNESS_BACKGROUND").is_ok_and(|v| v == "1"))
}

/// Whether the harness listens.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The sandboxed Documents folder (`FFX_HARNESS_HOME`), if any.
#[must_use]
pub fn home() -> Option<std::path::PathBuf> {
    std::env::var_os("FFX_HARNESS_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
}

/// Starts the listener when `FFX_HARNESS` names a port.
///
/// # Errors
/// When the port can't be bound.
pub fn start(shared: &Arc<Shared>) -> std::io::Result<()> {
    let Some(port) = std::env::var("FFX_HARNESS")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return Ok(());
    };
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    ON.store(true, Ordering::Relaxed);
    let shared = Arc::clone(shared);
    std::thread::Builder::new()
        .name("ffx-host harness".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&shared);
                let _ = std::thread::Builder::new()
                    .name("ffx-host harness client".into())
                    .spawn(move || serve(&shared, stream));
            }
        })?;
    Ok(())
}

fn serve(shared: &Arc<Shared>, stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        let reply = command(shared, line.trim());
        if out.write_all(format!("{reply}\n").as_bytes()).is_err() {
            return;
        }
    }
}

/// `Present`s so far.
pub fn presents() -> u32 {
    PRESENTS.load(Ordering::Relaxed)
}

// ---- the table: seed, flips, opens ---------------------------------------

/// Registers the harness's wrappers (with the clock's, `crate::clock`):
/// the flip labels, the file-open events and, with `FFX_HARNESS_SEED`, the
/// random seed.
pub fn register(t: &mut Table) {
    let entry = |name, conv, words, port: fn(&mut Exec, &Args) -> Ret| Entry {
        name,
        conv,
        words,
        port: Port::Exec(port),
    };
    // push ebp; mov ebp, esp; sub esp, 8
    t.wrap(
        0x005b_6af0,
        entry("Phyre_PRendering_PRenderer_flip", Conv::Thiscall, 0, flip),
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
    );
    // push ebp; mov ebp, esp; sub esp, 8
    t.wrap(
        0x0060_7f40,
        entry(
            "Phyre_PSerialization_PStreamFileWin32_openFile",
            Conv::Thiscall,
            5,
            file_open,
        ),
        &[0x55, 0x8b, 0xec, 0x83, 0xec, 0x08],
    );
    if seed().is_some() {
        t.register(
            0x0079_8950,
            entry("Rng_ClockSeed", Conv::Cdecl, 0, clock_seed),
        );
    }
    // push ebp; mov ebp, esp; push ebx
    t.wrap(
        0x0082_e390,
        entry("Ch_Cloth_PerCharacter", Conv::Cdecl, 2, cloth_job),
        &[0x55, 0x8b, 0xec, 0x53],
    );
    // push ebp; mov ebp, esp; mov ecx, [ebp + 0x10]
    t.wrap(
        0x0083_3530,
        entry("Ch_CalcMiddle_Batch", Conv::Cdecl, 3, middle_job),
        &[0x55, 0x8b, 0xec, 0x8b, 0x4d, 0x10],
    );
    crate::devsave::register(t);
}

/// Runs one character job with this thread's `rand()` seeded for it and
/// gives the thread its own seed back after, as the hook's `seeded_job`
/// (docs/harness.md): the cloth's wind (`Sg_ClothStep` under
/// `Ch_Cloth_PerCharacter`) and the mouth and texture animations (under
/// `Ch_CalcMiddle_Batch`) draw from `rand()`, whose seed is per thread, and
/// the scheduler runs them on whichever job thread is free. The seed comes
/// from the kind, the first character's place in the character array
/// (`*0x023c44e4`, 0x880 bytes a record) and [`JOB_EVENTS`]. Only with the
/// harness, as in the hook; the two functions have no callers but the job
/// schedulers (`JobSchedule_Ch_Cloth_PerCharacter` and
/// `JobSchedule_Ch_CalcMiddle_Batch` push them), so wrapping them is the
/// hook's patch of those pushes.
fn seeded_job(e: &mut Exec, kind: u32, first: u32, f: u32, args: &Args) -> Ret {
    if !enabled() || first == 0 {
        return e.call_original(f, Conv::Cdecl, args).unwrap_or(Ret::eax(0));
    }
    let array = e.mem().read::<u32>(0x023c_44e4);
    let place = first.wrapping_sub(array) / 0x880;
    let events = JOB_EVENTS.load(Ordering::Acquire);
    let own = e.crt.rand;
    e.crt.rand = place.wrapping_mul(0x9e37_79b1) ^ events.wrapping_mul(0x85eb_ca6b) ^ kind;
    let r = e.call_original(f, Conv::Cdecl, args).unwrap_or(Ret::eax(0));
    e.crt.rand = own;
    SEEDED_JOBS.fetch_add(1, Ordering::Relaxed);
    r
}

/// `Ch_Cloth_PerCharacter(chars, count)` (`0x0082e390`, cdecl): `count`
/// character pointers at `chars`, seeded (kind 1).
fn cloth_job(e: &mut Exec, args: &Args) -> Ret {
    cloth_order(e);
    let (chars, count) = (crate::imports::a(args, 0), crate::imports::a(args, 1));
    let first = if chars != 0 && count != 0 {
        e.mem().read::<u32>(chars)
    } else {
        0
    };
    seeded_job(e, 1, first, 0x0082_e390, args)
}

/// The cloth order (docs/host.md, "The cloth race"; the hook's harness
/// does the same): in lockstep, a cloth job waits until the render thread
/// has presented every flip queued before it, so that the previous
/// update's draw has staged the cloth vertices (setting the slots' flags)
/// before this update's submit (`ClothInstancingDynamicMesh_submit`, after
/// the jobs) looks at them. On by default; `FFX_HARNESS_CLOTH_ORDER=0`
/// turns it off. At most 2 s of host time, then it goes on and notes it.
fn cloth_order(e: &Exec) {
    static ON: OnceLock<bool> = OnceLock::new();
    if !*ON.get_or_init(|| std::env::var("FFX_HARNESS_CLOTH_ORDER").map_or(true, |v| v != "0"))
        || !enabled()
        || !LOCKSTEP.load(Ordering::Acquire)
    {
        return;
    }
    let want = FLIPS.load(Ordering::Acquire);
    if FLIPS_DONE.load(Ordering::Acquire) >= want {
        return;
    }
    CLOTH_WAITS.fetch_add(1, Ordering::Relaxed);
    let start = Instant::now();
    while FLIPS_DONE.load(Ordering::Acquire) < want && !e.shared().overdue() {
        if start.elapsed() > Duration::from_secs(2) {
            crate::log::note(&format!(
                "harness: cloth job at frame {}: flip {want} not presented after 2 s",
                e.shared().clock.frame()
            ));
            return;
        }
        std::thread::sleep(Duration::from_micros(200));
    }
}

/// `Ch_CalcMiddle_Batch(chars, &pass, &count)` (`0x00833530`, cdecl):
/// `*count` records from `chars`, seeded (kind 2, the pass above it).
fn middle_job(e: &mut Exec, args: &Args) -> Ret {
    let (chars, pass) = (crate::imports::a(args, 0), crate::imports::a(args, 1));
    let kind = if pass == 0 {
        2
    } else {
        2 | (e.mem().read::<u32>(pass) << 8)
    };
    seeded_job(e, kind, chars, 0x0083_3530, args)
}

fn seed() -> Option<u32> {
    std::env::var("FFX_HARNESS_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
}

/// `Rng_ClockSeed()` (`0x00798950`, cdecl): `FFX_HARNESS_SEED`, as the
/// hook's `clock_seed`.
fn clock_seed(_: &mut Exec, _: &Args) -> Ret {
    let s = seed().unwrap_or(1);
    crate::log::note(&format!("harness: random seed {s}"));
    Ret::eax(s)
}

/// `Phyre_PRendering_PRenderer_flip(renderer)` (`0x005b6af0`, thiscall,
/// 0 on success): notes the update the queued flip shows (the clock's
/// frame: the updates done).
fn flip(e: &mut Exec, args: &Args) -> Ret {
    let r = e
        .call_original(0x005b_6af0, Conv::Thiscall, args)
        .unwrap_or(Ret::eax(1));
    if r.eax == 0 {
        let n = FLIPS.load(Ordering::Acquire);
        FLIP_LABELS[n as usize % FLIP_LABELS.len()]
            .store(e.shared().clock.frame(), Ordering::Release);
        FLIPS.store(n + 1, Ordering::Release);
    }
    r
}

/// The scene: the last event file (`.ebp`) the game opened, as the hook's
/// `world` notes it (the gate captures' `World` message).
static SCENE: Mutex<String> = Mutex::new(String::new());

/// Notes the scene an opened file names (`<map>.ebp`).
fn note_scene(path: &str) {
    let name = path.rsplit('/').next().unwrap_or(path);
    if let Some(stem) = name.strip_suffix(".ebp")
        && let Ok(mut s) = SCENE.lock()
        && *s != stem
    {
        stem.clone_into(&mut s);
    }
}

/// The scene ([`note_scene`]; empty before the first map).
#[must_use]
pub fn scene() -> String {
    SCENE.lock().map(|s| s.clone()).unwrap_or_default()
}

/// The update the render thread's present shows (the harness's flip
/// label), `None` when unknown.
#[must_use]
pub fn present_label() -> Option<u32> {
    let l = PRESENT_LABEL.load(Ordering::Acquire);
    (l != NO_LABEL).then_some(l)
}

/// `G_FILE_SYS`: the archive file system (`+0x3c`: the root's length).
const FILE_SYS: u32 = 0x00cc_9c48;

/// `Phyre_PSerialization_PStreamFileWin32_openFile(file; path, mode, …)`
/// (`0x00607f40`, thiscall, 5 words): a read open is an `open <path>` event
/// (the path below the archive root, `\` as `/`, lower case), as the
/// hook's `mods::open_mod` records it.
fn file_open(e: &mut Exec, args: &Args) -> Ret {
    let path = args.stack.first().copied().unwrap_or(0);
    let mode = args.stack.get(1).copied().unwrap_or(0);
    if enabled() && path != 0 && mode & 0xff != 0 {
        let mem = e.mem();
        let fs: u32 = mem.read(FILE_SYS);
        if fs != 0 {
            let root = mem.read::<u32>(fs + 0x3c) as usize;
            let mut bytes = Vec::new();
            for i in 0..1024 {
                let b: u8 = mem.read(path + i);
                if b == 0 {
                    break;
                }
                bytes.push(b);
            }
            let key: String = bytes
                .iter()
                .map(|&b| {
                    if b == b'\\' {
                        '/'
                    } else {
                        char::from(b.to_ascii_lowercase())
                    }
                })
                .collect();
            note_scene(&key);
            if let Some(k) = key.get(root..)
                && let Ok(mut ev) = EVENTS.lock()
            {
                ev.push(format!("open {k}"));
            }
        }
    }
    e.call_original(0x0060_7f40, Conv::Thiscall, args)
        .unwrap_or(Ret::eax(1))
}

// ---- the frame boundary ----------------------------------------------------

/// The main thread at frame boundary `frame` (`crate::clock`, before the
/// loader's turn): `lockstep on` takes hold, the trace samples, the jobs
/// run, and lockstep holds until the script allows the next update.
pub fn boundary(e: &mut Exec, frame: u32) {
    if !enabled() {
        return;
    }
    if LOCKSTEP_REQUEST.swap(false, Ordering::AcqRel) {
        ALLOWED.store(frame, Ordering::Release);
        LOCKSTEP.store(true, Ordering::Release);
    }
    REACHED.store(frame + 1, Ordering::Release);
    trace_frame(e, frame);
    run_main_jobs(e, frame);
    while LOCKSTEP.load(Ordering::Acquire)
        && frame >= ALLOWED.load(Ordering::Acquire)
        && !e.shared().overdue()
        && e.stopped().is_none()
    {
        std::thread::sleep(Duration::from_millis(1));
        run_main_jobs(e, frame);
    }
    PASSED.store(frame + 1, Ordering::Release);
    JOB_EVENTS.store(e.mem().read::<u32>(0x0132_5d94), Ordering::Release);
}

/// The `callnext` calls, at the start of the update (after the boundary's
/// turns).
pub fn update_calls(e: &mut Exec) {
    if !enabled() {
        return;
    }
    let due: Vec<_> = UPDATE_CALLS
        .lock()
        .map_or_else(|_| Vec::new(), |mut c| c.drain(..).collect());
    for (address, args, reply) in due {
        let r = e.call_cdecl(address, &args);
        let _ = reply.send(r.map_or_else(|e| format!("err {e}"), |eax| format!("ok {eax:#x}")));
    }
}

#[allow(clippy::too_many_lines)] // one arm per job kind
fn run_main_jobs(e: &mut Exec, frame: u32) {
    let due: Vec<(Job, Sender<String>)> = JOBS.lock().map_or_else(
        |_| Vec::new(),
        |mut jobs| {
            let mut keep = VecDeque::new();
            let mut due = Vec::new();
            while let Some(mut job) = jobs.pop_front() {
                let ready = match job.0 {
                    Job::WaitUntil(f) | Job::Release(f, _) => frame >= f,
                    Job::Call(..) | Job::Pin | Job::LoadSave(..) | Job::DevSave(..) => true,
                    Job::WaitFor(r, ref value, ref mask, limit) => {
                        frame >= limit || range_matches(e, r, value, mask)
                    }
                    Job::ShotWhen(r, ref value, ref mask, limit, _, ref mut first) => {
                        first.get_or_insert(frame);
                        frame >= limit || range_matches(e, r, value, mask)
                    }
                    Job::HoldFor(r, ref value, ref mask, deadline) => {
                        Instant::now() >= deadline || range_matches(e, r, value, mask)
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
                e.shared().clock.buttons.store(0, Ordering::Release);
                set_keys(&[]);
                if let Ok(mut jobs) = JOBS.lock() {
                    jobs.push_back((Job::WaitUntil(then), reply));
                }
            }
            Job::Call(address, args) => {
                let r = e.call_cdecl(address, &args);
                let _ =
                    reply.send(r.map_or_else(|e| format!("err {e}"), |eax| format!("ok {eax:#x}")));
            }
            Job::Pin => {
                let _ = reply.send(format!("ok {}", crate::replay::pin(e)));
            }
            Job::LoadSave(spec, lockstep) => {
                let r = load_save(e, &spec);
                crate::log::note(&format!("harness: loadsave {spec} at frame {frame}: {r:?}"));
                if lockstep && r.is_ok() {
                    LOCKSTEP_REQUEST.store(false, Ordering::Release);
                    ALLOWED.store(frame, Ordering::Release);
                    LOCKSTEP.store(true, Ordering::Release);
                }
                let _ = reply.send(r.map_or_else(|e| format!("err {e}"), |s| format!("ok {s}")));
            }
            Job::DevSave(n, lockstep) => {
                let r = crate::devsave::load(e, n);
                crate::log::note(&format!("harness: devsave {n} at frame {frame}: {r:?}"));
                // as for loadsave: the map's load in lockstep from here
                if lockstep && r.is_ok() {
                    LOCKSTEP_REQUEST.store(false, Ordering::Release);
                    ALLOWED.store(frame, Ordering::Release);
                    LOCKSTEP.store(true, Ordering::Release);
                }
                let _ = reply.send(r.map_or_else(|e| format!("err {e}"), |s| format!("ok {s}")));
            }
            Job::WaitFor(r, value, mask, _) => {
                let hit = range_matches(e, r, &value, &mask);
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
                let hit = range_matches(e, r, &value, &mask);
                let lockstep = LOCKSTEP.load(Ordering::Acquire);
                if lockstep {
                    ALLOWED.store(frame, Ordering::Release);
                }
                if !hit {
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                    let _ = reply.send(format!("err timeout at {frame}"));
                } else if !lockstep && first == Some(frame) {
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                    let _ = reply.send(format!(
                        "err matched at {frame}, the first frame it was checked: wait for an \
                         earlier state first"
                    ));
                } else if let Ok(mut shots) = SHOTS.lock() {
                    // its present waits for this boundary to end (PINNED)
                    shots.push(Shot {
                        path,
                        want: frame,
                        pinned: true,
                        reply,
                    });
                } else {
                    PINNED.fetch_sub(1, Ordering::AcqRel);
                }
            }
            Job::HoldFor(r, value, mask, _) => {
                let _ = reply.send(if range_matches(e, r, &value, &mask) {
                    format!("ok {frame}")
                } else {
                    format!("err timeout at {frame}")
                });
            }
        }
    }
}

/// Queues a main-thread job and waits for its reply (or the run's end).
fn on_game_thread(shared: &Shared, job: Job) -> String {
    if presents() == 0 {
        return "err the game has not presented a frame yet".into();
    }
    let (tx, rx) = channel();
    if let Ok(mut jobs) = JOBS.lock() {
        jobs.push_back((job, tx));
    }
    wait_reply(shared, &rx)
}

fn wait_reply(shared: &Shared, rx: &std::sync::mpsc::Receiver<String>) -> String {
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(r) => return r,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return "err no reply".into(),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(stop) = shared.stopped() {
                    return format!("err the run ended: {stop}");
                }
            }
        }
    }
}

// ---- presents and screenshots ------------------------------------------------

/// `Renderer_PresentFlip`'s wrapper (`crate::clock`, render thread), before
/// the original: which update this flip shows, and in lockstep (or with a
/// `shotwhen` pending) the hold until the main thread has left that
/// update's boundary, taking the screenshots asked for meanwhile.
pub fn before_present_flip(shared: &Shared) {
    let n = FLIPS_RUN.fetch_add(1, Ordering::AcqRel);
    let label = if n < FLIPS.load(Ordering::Acquire) {
        FLIP_LABELS[n as usize % FLIP_LABELS.len()].load(Ordering::Acquire)
    } else {
        NO_LABEL
    };
    PRESENT_LABEL.store(label, Ordering::Release);
    if !enabled() || label == NO_LABEL {
        return;
    }
    let start = Instant::now();
    // a gate capture's frame is taken at its Present, inside the flip:
    // the update it wants is not held (the hook's capture takes it in
    // Present, before the hold there), else the script's `gfxcapture
    // result` at boundary `label` would wait for it forever
    let captured = shared
        .gfx
        .lock()
        .as_ref()
        .is_some_and(|g| g.gate_wants() == Some(label));
    let hold =
        || !captured && (LOCKSTEP.load(Ordering::Acquire) || PINNED.load(Ordering::Acquire) > 0);
    let mut waited = false;
    while hold()
        && PASSED.load(Ordering::Acquire) <= label
        && (REACHED.load(Ordering::Acquire) > label || start.elapsed() < PRESENT_PATIENCE)
        && !shared.overdue()
    {
        waited = true;
        if has_shots(label) {
            let mut g = shared.gfx.lock();
            if let Some(g) = g.as_mut() {
                take_shots(label, || g.back_buffer_image());
            }
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if waited && LOCKSTEP.load(Ordering::Acquire) && REACHED.load(Ordering::Acquire) <= label {
        crate::log::note(&format!(
            "harness: present of update {label}: the main thread did not reach its boundary"
        ));
    }
}

/// After `Renderer_PresentFlip`'s original.
pub fn after_present_flip() {
    PRESENT_LABEL.store(NO_LABEL, Ordering::Release);
    FLIPS_DONE.fetch_add(1, Ordering::AcqRel);
}

fn has_shots(label: u32) -> bool {
    SHOTS
        .lock()
        .is_ok_and(|s| s.iter().any(|s| label == NO_LABEL || label >= s.want))
}

/// Takes the screenshots the image of update `label` satisfies.
fn take_shots(label: u32, mut image: impl FnMut() -> Option<(u32, u32, Vec<u8>)>) {
    let due: Vec<Shot> = SHOTS.lock().map_or_else(
        |_| Vec::new(),
        |mut s| {
            let (due, keep) = s
                .drain(..)
                .partition(|s| label == NO_LABEL || label >= s.want);
            *s = keep;
            due
        },
    );
    for shot in due {
        if shot.pinned {
            PINNED.fetch_sub(1, Ordering::AcqRel);
            if label != shot.want {
                let shown = if label == NO_LABEL {
                    "an unknown update".to_owned()
                } else {
                    format!("update {label}")
                };
                let _ = shot.reply.send(format!(
                    "err update {}'s image was missed (this one shows {shown})",
                    shot.want
                ));
                continue;
            }
        }
        let r = match image() {
            Some((w, h, bgra)) => match std::fs::write(&shot.path, bmp(w, h, &bgra)) {
                Ok(()) if label == NO_LABEL => format!("ok {w} {h}"),
                Ok(()) => format!("ok {w} {h} update {label}"),
                Err(e) => format!("err {}: {e}", shot.path),
            },
            None => "err no back buffer to read (the null device)".into(),
        };
        let _ = shot.reply.send(r);
    }
}

/// The layer's `Present` (before the back end presents; the gfx lock held):
/// counts it and takes the screenshots waiting for this image.
pub fn on_present(image: impl FnMut() -> Option<(u32, u32, Vec<u8>)>) {
    PRESENTS.fetch_add(1, Ordering::Relaxed);
    if enabled() {
        let label = PRESENT_LABEL.load(Ordering::Acquire);
        if has_shots(label) {
            take_shots(label, image);
        }
    }
}

/// The `DirectInput` keyboard's state (`GetDeviceState`, 256 bytes): the
/// keys the test holds read `0x80`.
pub fn keyboard_state(out: &mut [u8]) {
    out.fill(0);
    KEY_POLLS.fetch_add(1, Ordering::Relaxed);
    if let Ok(k) = KEYS.lock() {
        for (o, &held) in out.iter_mut().zip(k.iter()) {
            if held {
                *o = 0x80;
            }
        }
    }
}

/// A 32-bit bottom-up BMP of `w` x `h` BGRA rows.
fn bmp(w: u32, h: u32, bgra: &[u8]) -> Vec<u8> {
    let size = w * h * 4;
    let mut f = Vec::with_capacity(54 + size as usize);
    f.extend(b"BM");
    f.extend((54 + size).to_le_bytes());
    f.extend(0u32.to_le_bytes());
    f.extend(54u32.to_le_bytes());
    f.extend(40u32.to_le_bytes());
    f.extend(w.to_le_bytes());
    f.extend(h.to_le_bytes());
    f.extend(1u16.to_le_bytes());
    f.extend(32u16.to_le_bytes());
    f.extend([0u8; 24]);
    let row = (w * 4) as usize;
    for y in (0..h as usize).rev() {
        f.extend(bgra.get(y * row..(y + 1) * row).unwrap_or(&[]));
    }
    f
}

fn set_keys(codes: &[u8]) {
    if let Ok(mut k) = KEYS.lock() {
        *k = [false; 256];
        for &c in codes {
            k[usize::from(c)] = true;
        }
    }
}

// ---- memory ranges and the trace ---------------------------------------------

fn parse_range(spec: &str) -> Result<Range, String> {
    let (at, len) = spec
        .split_once(':')
        .ok_or_else(|| format!("{spec}: want addr:len"))?;
    let len = parse(len)?;
    let (deref, at) = at.strip_prefix('*').map_or((false, at), |a| (true, a));
    let (addr, off) = match at.split_once('+') {
        Some((a, o)) => (parse(a)?, parse(o)?),
        None => (parse(at)?, 0),
    };
    Ok(Range {
        addr,
        deref,
        off,
        len,
    })
}

/// The bytes of a range, if mapped.
fn range_bytes(shared: &Shared, r: Range) -> Option<Vec<u8>> {
    let mem = ArenaMem {
        arena: Arc::clone(shared.arena()),
    };
    let at = if r.deref {
        if !shared.arena().is_mapped(r.addr, 4) {
            return None;
        }
        mem.read::<u32>(r.addr).wrapping_add(r.off)
    } else {
        r.addr.wrapping_add(r.off)
    };
    if !shared.arena().is_mapped(at, r.len as usize) {
        return None;
    }
    let mut b = vec![0u8; r.len as usize];
    mem.read_bytes(at, &mut b);
    Some(b)
}

fn range_matches(e: &Exec, r: Range, value: &[u8], mask: &[u8]) -> bool {
    range_bytes(e.shared(), r).is_some_and(|b| {
        b.iter()
            .zip(value)
            .zip(mask)
            .all(|((x, v), m)| x & m == v & m)
    })
}

/// FNV-1a over the range, or `-` when it is not mapped (the hook's
/// `trace_hash`).
fn trace_hash(shared: &Shared, r: Range) -> String {
    range_bytes(shared, r).map_or_else(
        || "-".into(),
        |bytes| {
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in bytes {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            format!("{h:016x}")
        },
    )
}

fn trace_frame(e: &Exec, frame: u32) {
    if let Ok(mut t) = TRACE.lock()
        && let Some(t) = t.as_mut()
    {
        let hashes: Vec<String> = t
            .ranges
            .iter()
            .map(|&r| trace_hash(e.shared(), r))
            .collect();
        let _ = writeln!(t.out, "{frame} {}", hashes.join(" "));
    }
}

// ---- loadsave ------------------------------------------------------------------

const SAVE_DATA_MANAGER: u32 = 0x00ce_81f4;
const SAVE_FOLDER: u32 = 0x00ce_6ee0;
const SAVE_FOLDER_SET: u32 = 0x00ce_72e0;
const SAVE_FILE_PREFIX: u32 = 0x00c3_577c;
const SAVE_SIZE: u32 = 0x6900;
const SCRIPT_CONTEXT: u32 = 0x0132_6b28;
const CTRL_WORKS: u32 = 0x0132_5ba0;
const SET_UP_DEFAULT_SAVE_FOLDER: u32 = 0x006f_0330;
const SAVE_DATA_CHECK_CRC: u32 = 0x0064_7d70;
const SAVE_DATA_SAVE_LOAD_SUCCEED: u32 = 0x0064_8540;
const SAVE_DATA_SET_DIALOG_STATE: u32 = 0x0064_8620;
const SAVE_DATA_IS_ALBHED_LOADED: u32 = 0x0064_8020;
const SAVE_DATA_MARK_ALBHED: u32 = 0x0064_7e40;
const SAVE_DATA_RESET_TO_IDLE: u32 = 0x0064_8530;
const SAVE_DATA_SET_SCREEN_STATE: u32 = 0x0064_86e0;
const ATEL_CLEAR_FORCE_PLACE_ID: u32 = 0x0086_38b0;
const TK_MENU_JUMP_TO_LOADED_SCENE: u32 = 0x008b_4ec0;
const SET_CTRL0_FLAG_210: u32 = 0x0086_f3d0;
const ATEL_SET_EVENT_MODE: u32 = 0x0087_3e40;
const NATIVE_MUSIC_FADE_OUT: u32 = 0x0085_e290;
const NATIVE_FADE_OUT: u32 = 0x00a7_7da0;
const NATIVE_01C2: u32 = 0x0085_a570;
const NATIVE_SET_MAP_FADE_FRAMES: u32 = 0x0085_96b0;
const NATIVE_SET_MAP_FADE_TYPE: u32 = 0x0085_9590;
const NATIVE_MAP_JUMP: u32 = 0x0085_8460;

fn read_wide(e: &Exec, at: u32) -> Result<String, String> {
    let mem = e.mem();
    let mut units = Vec::new();
    for i in 0..1024 {
        let u: u16 = mem.read(at + 2 * i);
        if u == 0 {
            return Ok(String::from_utf16_lossy(&units));
        }
        units.push(u);
    }
    Err(format!("no string end at {at:#x}"))
}

/// Calls a script native (`(worker, unused, stack)`) with integer
/// arguments, through a stack in the arena laid out as
/// `AtelPopStackInteger` (`0x0086df00`) reads it: the count, the values
/// from `+4`, a type byte per value from `+0x54` (1 = integer).
fn native(e: &mut Exec, address: u32, args: &[i32]) -> Result<u32, String> {
    let stack = e.alloc(0x80);
    if stack == 0 {
        return Err("no memory for a native's stack".into());
    }
    let n = args.len().min(16);
    {
        let mut mem = e.mem();
        mem.write_bytes(stack, &[0; 0x80]);
        #[allow(clippy::cast_possible_truncation)] // at most 16
        mem.write(stack, n as u32);
        for (i, &a) in (0u32..).zip(args.iter().take(n)) {
            mem.write(stack + 4 + 4 * i, a.cast_unsigned());
            mem.write(stack + 0x54 + i, 1u8);
        }
    }
    let r = e.call_cdecl(address, &[0, 0, stack]);
    e.release(stack);
    r.map_err(|e| e.to_string())
}

fn call(e: &mut Exec, address: u32, args: &[u32]) -> Result<u32, String> {
    e.call_cdecl(address, args).map_err(|e| e.to_string())
}

/// The title script's part around its Load menu once a save is picked
/// (the hook's `loadsave::leave_title`): the title leaves as for a load
/// (`loadsave`, `devsave`).
pub(crate) fn leave_title(e: &mut Exec) -> Result<(), String> {
    call(e, SET_CTRL0_FLAG_210, &[1])?;
    call(e, ATEL_SET_EVENT_MODE, &[2])?;
    native(e, NATIVE_MUSIC_FADE_OUT, &[60])?;
    native(e, NATIVE_FADE_OUT, &[1])?;
    native(e, NATIVE_01C2, &[])?;
    native(e, NATIVE_SET_MAP_FADE_FRAMES, &[0])?;
    native(e, NATIVE_SET_MAP_FADE_TYPE, &[0])?;
    Ok(())
}

/// `loadsave <slot | path>` (the hook's `crate::loadsave`, docs/harness.md
/// "Snapshots and replays"): what the title's Load menu runs once a save
/// is picked, at the boundary.
fn load_save(e: &mut Exec, spec: &str) -> Result<String, String> {
    let path = if let Ok(slot) = spec.parse::<u32>() {
        if slot >= 200 {
            return Err(format!("slot {slot}: the game lists 0..199"));
        }
        call(e, SET_UP_DEFAULT_SAVE_FOLDER, &[])?;
        if e.mem().read::<u32>(SAVE_FOLDER_SET) == 0 {
            return Err("the save folder could not be set up".into());
        }
        let folder = read_wide(e, SAVE_FOLDER)?;
        let prefix_at: u32 = e.mem().read(SAVE_FILE_PREFIX);
        let prefix = read_wide(e, prefix_at)?;
        format!("{folder}{prefix}{slot:03}")
    } else {
        spec.to_owned()
    };
    // the game's path, else a path of the host's own
    let host = e
        .files()
        .resolve(&path)
        .map_or_else(|| std::path::PathBuf::from(&path), |(p, _)| p);
    let bytes = std::fs::read(&host).map_err(|err| format!("{path}: {err}"))?;
    let manager: u32 = e.mem().read(SAVE_DATA_MANAGER);
    if manager == 0 {
        return Err("no save data manager yet".into());
    }
    let buffer: u32 = e.mem().read(manager + 0x350);
    let size: u32 = e.mem().read(manager + 0x354);
    if size != SAVE_SIZE || bytes.len() != size as usize {
        return Err(format!(
            "{path}: {} bytes, the game reads {size:#x}",
            bytes.len()
        ));
    }
    if !e.arena().is_mapped(buffer, size as usize) {
        return Err(format!("the save buffer {buffer:#x} is not mapped"));
    }
    {
        let mut mem = e.mem();
        mem.write_bytes(buffer, &bytes);
        // SaveDataManager_doLoad: albhed result -1, state 3 (loading)
        mem.write(manager + 0xc, u32::MAX);
        mem.write(manager + 8, 3u32);
    }
    if call(e, SAVE_DATA_CHECK_CRC, &[])? != 1 {
        let mut mem = e.mem();
        mem.write_bytes(buffer, &vec![0; size as usize]);
        mem.write(manager + 8, 0u32);
        return Err(format!("{path}: CRC mismatch"));
    }
    call(e, SAVE_DATA_SAVE_LOAD_SUCCEED, &[3])?;
    call(e, SAVE_DATA_SET_DIALOG_STATE, &[0])?;
    if call(e, SAVE_DATA_IS_ALBHED_LOADED, &[])? == u32::MAX {
        call(e, SAVE_DATA_MARK_ALBHED, &[])?;
    }
    call(e, SAVE_DATA_RESET_TO_IDLE, &[])?;
    call(e, ATEL_CLEAR_FORCE_PLACE_ID, &[])?;
    call(e, TK_MENU_JUMP_TO_LOADED_SCENE, &[])?;
    call(e, SAVE_DATA_SET_SCREEN_STATE, &[0])?;
    leave_title(e)?;
    // the jump needs the running event to hold control (ctrl byte 0 bit
    // 7): the title script does, from ctrl work 0
    let (old, forced) = {
        let mut mem = e.mem();
        let old: u32 = mem.read(SCRIPT_CONTEXT);
        mem.write(SCRIPT_CONTEXT, CTRL_WORKS);
        let flags: u8 = mem.read(CTRL_WORKS);
        mem.write(CTRL_WORKS, flags | 0x80);
        (old, flags & 0x80 == 0)
    };
    native(e, NATIVE_MAP_JUMP, &[-1, 0])?;
    let mut mem = e.mem();
    mem.write(SCRIPT_CONTEXT, old);
    let map: u16 = mem.read(0x0112_ca90);
    let entrance: u8 = mem.read(0x0112_ca9c);
    Ok(format!(
        "{path} map {map} entrance {entrance} context {old:#x}{}",
        if forced { " (control taken)" } else { "" }
    ))
}

// ---- commands ---------------------------------------------------------------

/// The default key of each PS2 button (`GameSetting.ini`'s
/// `ffxKeyBinding` defaults, docs/harness.md "Input facts"), as
/// `DirectInput` scan codes: `key`'s names.
fn buttons(s: &str) -> Result<u32, String> {
    let mut mask = 0;
    for b in s.split('+').filter(|b| !b.is_empty()) {
        mask |= match b {
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
            other => parse(other)?,
        };
    }
    Ok(mask)
}

fn parse(s: &str) -> Result<u32, String> {
    let r = match s.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16),
        None => s.parse(),
    };
    r.map_err(|_| format!("bad number {s}"))
}

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "bad hex".to_string()))
        .collect()
}

fn command(shared: &Arc<Shared>, line: &str) -> String {
    match run(shared, line) {
        Ok(r) => r,
        Err(e) => format!("err {e}"),
    }
}

/// `value mask` of `waitfor`/`shotwhen`/`holdfor`, as long as the range.
fn value_mask(words: &[&str], r: Range, mask_needed: bool) -> Result<(Vec<u8>, Vec<u8>), String> {
    let value = unhex(words.get(2).ok_or("missing value")?)?;
    let mask = match words.get(3) {
        Some(m) => unhex(m)?,
        None if mask_needed => return Err("missing mask".into()),
        None => vec![0xff; value.len()],
    };
    if value.len() != r.len as usize || mask.len() != r.len as usize {
        return Err("value and mask must be as long as the range".into());
    }
    Ok((value, mask))
}

#[allow(clippy::too_many_lines)] // one arm per command
fn run(shared: &Arc<Shared>, line: &str) -> Result<String, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let arg = |i: usize| words.get(i).copied().ok_or("missing argument");
    let mut mem = ArenaMem {
        arena: Arc::clone(shared.arena()),
    };
    let now = || shared.clock.frame();
    let read = |mem: &ArenaMem, at: u32, len: u32| -> Result<String, String> {
        if len > 1 << 20 {
            return Err("too long".into());
        }
        let mut b = vec![0u8; len as usize];
        if !shared.arena().is_mapped(at, b.len()) {
            return Err(format!("{at:#x} is not mapped"));
        }
        mem.read_bytes(at, &mut b);
        Ok(format!("ok {}", hex(&b)))
    };
    match words.first().copied().unwrap_or("") {
        "ping" => Ok("ok pong".into()),
        "frame" => Ok(format!("ok {}", now())),
        "stats" => Ok(format!(
            "ok frame {} pad_reads 0 key_polls {} presents {} flips {} flips_run {} seeded_jobs {} cloth_waits {}",
            now(),
            KEY_POLLS.load(Ordering::Relaxed),
            presents(),
            FLIPS.load(Ordering::Relaxed),
            FLIPS_RUN.load(Ordering::Relaxed),
            SEEDED_JOBS.load(Ordering::Relaxed),
            CLOTH_WAITS.load(Ordering::Relaxed),
        )),
        "wait" => {
            let until = now() + parse(arg(1)?)?;
            ALLOWED.fetch_max(until, Ordering::AcqRel);
            Ok(on_game_thread(shared, Job::WaitUntil(until)))
        }
        "waitfor" => {
            let r = parse_range(arg(1)?)?;
            let (value, mask) = value_mask(&words, r, false)?;
            let limit = now() + words.get(4).map_or(Ok(600), |w| parse(w))?;
            ALLOWED.fetch_max(limit, Ordering::AcqRel);
            Ok(on_game_thread(shared, Job::WaitFor(r, value, mask, limit)))
        }
        "holdfor" => {
            if !LOCKSTEP.load(Ordering::Acquire) {
                return Err("holdfor needs lockstep".into());
            }
            let r = parse_range(arg(1)?)?;
            let (value, mask) = value_mask(&words, r, true)?;
            let deadline = Instant::now() + Duration::from_millis(u64::from(parse(arg(4)?)?));
            Ok(on_game_thread(
                shared,
                Job::HoldFor(r, value, mask, deadline),
            ))
        }
        "shotwhen" => {
            let r = parse_range(arg(1)?)?;
            let (value, mask) = value_mask(&words, r, true)?;
            let limit = now() + parse(arg(4)?)?;
            let path = words.get(5..).map(|w| w.join(" ")).unwrap_or_default();
            if path.is_empty() {
                return Err("shotwhen: missing the path".into());
            }
            if presents() == 0 {
                return Err("the game has not presented a frame yet".into());
            }
            ALLOWED.fetch_max(limit, Ordering::AcqRel);
            PINNED.fetch_add(1, Ordering::AcqRel);
            Ok(on_game_thread(
                shared,
                Job::ShotWhen(r, value, mask, limit, path, None),
            ))
        }
        "lockstep" => {
            let on = arg(1)? == "on";
            if on && !LOCKSTEP.load(Ordering::Acquire) {
                // the main thread takes hold at its next boundary; one that
                // does not get there in time is held at once
                LOCKSTEP_REQUEST.store(true, Ordering::Release);
                let start = Instant::now();
                while LOCKSTEP_REQUEST.load(Ordering::Acquire)
                    && start.elapsed() < Duration::from_secs(60)
                    && shared.stopped().is_none()
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                if LOCKSTEP_REQUEST.swap(false, Ordering::AcqRel) {
                    ALLOWED.store(now(), Ordering::Release);
                    LOCKSTEP.store(true, Ordering::Release);
                }
                return Ok(format!("ok {}", ALLOWED.load(Ordering::Acquire)));
            }
            ALLOWED.store(now(), Ordering::Release);
            LOCKSTEP.store(on, Ordering::Release);
            Ok(format!("ok {}", now()))
        }
        "read" | "readabs" => read(&mem, parse(arg(1)?)?, parse(arg(2)?)?),
        "readp" => {
            // readp <addr> <off>... <len>: *addr + off; with more offsets
            // each one before the last is followed too
            if words.len() < 4 {
                return Err("readp <addr> <off>... <len>".into());
            }
            let word = |at: u32| -> Result<u32, String> {
                if shared.arena().is_mapped(at, 4) {
                    Ok(mem.read(at))
                } else {
                    Err(format!("{at:#x} is not mapped"))
                }
            };
            let mut at = word(parse(arg(1)?)?)?;
            let offs = &words[2..words.len() - 1];
            for (i, off) in offs.iter().enumerate() {
                at = at.wrapping_add(parse(off)?);
                if i + 1 < offs.len() {
                    at = word(at)?;
                }
            }
            read(&mem, at, parse(words[words.len() - 1])?)
        }
        "write" | "writep" => {
            let (at, data) = if words[0] == "write" {
                (parse(arg(1)?)?, unhex(arg(2)?)?)
            } else {
                let base: u32 = mem.read(parse(arg(1)?)?);
                (base.wrapping_add(parse(arg(2)?)?), unhex(arg(3)?)?)
            };
            if !shared.arena().is_mapped(at, data.len()) {
                return Err(format!("{at:#x} is not mapped"));
            }
            mem.write_bytes(at, &data);
            Ok("ok".into())
        }
        "keys" => {
            let codes = words[1..]
                .iter()
                .map(|w| u8::from_str_radix(w.trim_start_matches("0x"), 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "bad key")?;
            set_keys(&codes);
            Ok("ok".into())
        }
        "key" => {
            let codes = arg(1)?
                .split(',')
                .map(|w| u8::from_str_radix(w.trim_start_matches("0x"), 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "bad key")?;
            let hold = parse(arg(2)?)?;
            let gap = words.get(3).map_or(Ok(hold), |w| parse(w))?;
            set_keys(&codes);
            let release = now() + hold;
            ALLOWED.fetch_max(release + gap, Ordering::AcqRel);
            Ok(on_game_thread(shared, Job::Release(release, release + gap)))
        }
        "pad" => {
            let mask = words.get(1).map_or(Ok(0), |b| buttons(b))?;
            shared.clock.buttons.store(mask, Ordering::Release);
            Ok("ok".into())
        }
        "press" => {
            let mask = buttons(arg(1)?)?;
            let hold = parse(arg(2)?)?;
            let gap = words.get(3).map_or(Ok(hold), |w| parse(w))?;
            shared.clock.buttons.store(mask, Ordering::Release);
            let release = now() + hold;
            ALLOWED.fetch_max(release + gap, Ordering::AcqRel);
            Ok(on_game_thread(shared, Job::Release(release, release + gap)))
        }
        "call" | "callnext" => {
            let address = parse(arg(1)?)?;
            let args = words[2..]
                .iter()
                .map(|w| parse(w))
                .collect::<Result<Vec<_>, _>>()?;
            if args.len() > 8 {
                return Err("at most 8 arguments".into());
            }
            if words[0] == "call" {
                return Ok(on_game_thread(shared, Job::Call(address, args)));
            }
            if presents() == 0 {
                return Err("the game has not presented a frame yet".into());
            }
            let (tx, rx) = channel();
            if let Ok(mut calls) = UPDATE_CALLS.lock() {
                calls.push_back((address, args, tx));
            }
            ALLOWED.fetch_max(now() + 1, Ordering::AcqRel);
            Ok(wait_reply(shared, &rx))
        }
        "pin" => Ok(on_game_thread(shared, Job::Pin)),
        "loadsave" => {
            let lockstep = words.len() > 2 && words.last() == Some(&"lockstep");
            let spec = words[1..words.len() - usize::from(lockstep)].join(" ");
            Ok(on_game_thread(shared, Job::LoadSave(spec, lockstep)))
        }
        "devsave" => {
            let n = arg(1)?
                .parse::<i32>()
                .map_err(|e| format!("developer save number: {e}"))?;
            Ok(on_game_thread(
                shared,
                Job::DevSave(n, words.get(2) == Some(&"lockstep")),
            ))
        }
        "shot" => {
            let path = words[1..].join(" ");
            if path.is_empty() {
                return Err("shot <path>".into());
            }
            if presents() == 0 {
                return Err("the game has not presented a frame yet".into());
            }
            let (tx, rx) = channel();
            if let Ok(mut s) = SHOTS.lock() {
                s.push(Shot {
                    path,
                    want: now(),
                    pinned: false,
                    reply: tx,
                });
            }
            Ok(wait_reply(shared, &rx))
        }
        "trace" => {
            let mut t = TRACE.lock().map_err(|_| "trace lock poisoned")?;
            if arg(1)? == "off" {
                if let Some(mut old) = t.take() {
                    old.out.flush().map_err(|e| e.to_string())?;
                }
                return Ok("ok".into());
            }
            let ranges = words[2..]
                .iter()
                .map(|w| parse_range(w))
                .collect::<Result<Vec<_>, _>>()?;
            let file = std::fs::File::create(arg(1)?).map_err(|e| e.to_string())?;
            *t = Some(Trace {
                out: std::io::BufWriter::new(file),
                ranges,
            });
            Ok("ok".into())
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
            Ok(out)
        }
        // every game thread's emulated call stack (crate::stacks)
        "stacks" => Ok(format!("ok {}", crate::stacks::stacks(shared))),
        "notes" => {
            // the run's notes so far (the clock's turns, failed threads, …)
            let notes = crate::log::notes();
            let mut out = format!("ok {}", notes.len());
            for n in notes {
                out.push('\n');
                out.push_str(&n.replace('\n', " "));
            }
            Ok(out)
        }
        // a scene-gate capture (docs/drawdiff.md, "Scene gates"):
        // `gfxcapture <dir> [hash] [frames N] [now] [async]`, `gfxcapture
        // result` (crate::gfx::gate)
        "gfxcapture" if crate::gfx::gate::Options::wanted(&words[1..]) => {
            Ok(gate_capture(shared, &words[1..], now()))
        }
        "gfxcapture" => {
            let dir = arg(1)?;
            if shared.gfx.lock().as_ref().is_some_and(|g| !g.records()) {
                return Err("no recording (start ffx-rust with FFX_HOST_GFX_RECORD=1)".into());
            }
            crate::gfx::write_shared(shared, std::path::Path::new(dir))
                .map_err(|e| e.to_string())?;
            let frames = shared.gfx.lock().as_ref().map_or(0, |g| g.presents);
            Ok(format!("ok {frames} frames"))
        }
        "quit" => {
            quit(shared);
            Ok("ok".into())
        }
        // the replay player's closing line: no call log here
        "calllog" if words.get(1) == Some(&"off") => Ok("ok".into()),
        // where the mix stands (docs/harness.md, "Audio capture"): the
        // marks file beside the `FFX_AUDIO_WAV` capture gets the line
        "audiomark" => {
            let (turns, late) = shared.clock.fmod_turn_state();
            Ok(shared.audio.mark(arg(1)?, now(), turns, late))
        }
        other => Err(format!("{other}: not served by ffx-rust yet")),
    }
}

/// The reply of a detached gate capture (`gfxcapture ... async`), for
/// `gfxcapture result`.
static GATE_RESULT: Mutex<Option<std::sync::mpsc::Receiver<String>>> = Mutex::new(None);

/// `gfxcapture <dir> [hash] [frames N] [now] [async]` or `gfxcapture
/// result`, at boundary `frame`: captures the presents of updates
/// `frame + 1`.. (`crate::gfx::gate`), as the hook's capture armed after
/// the present of update `frame`.
fn gate_capture(shared: &Shared, words: &[&str], frame: u32) -> String {
    if words.first() == Some(&"result") {
        let Some(rx) = GATE_RESULT.lock().ok().and_then(|mut r| r.take()) else {
            return "err no capture is running".into();
        };
        return wait_reply(shared, &rx);
    }
    let opts = match crate::gfx::gate::Options::parse(words) {
        Ok(o) => o,
        Err(e) => return format!("err {e}"),
    };
    let detach = opts.detach;
    let (tx, rx) = channel();
    let mem = ArenaMem {
        arena: Arc::clone(shared.arena()),
    };
    let armed = match shared.gfx.lock().as_mut() {
        Some(g) => g.gate_arm(&mem, opts, frame + 1, tx),
        None => Err("no device yet".into()),
    };
    if let Err(e) = armed {
        return format!("err {e}");
    }
    if detach {
        if let Ok(mut r) = GATE_RESULT.lock() {
            *r = Some(rx);
        }
        return format!("ok armed at boundary {frame}");
    }
    wait_reply(shared, &rx)
}

/// Where `quit` writes the profile, and the `ffx.sym` it names
/// `FFX.exe`'s functions by ([`profile_at_quit`]).
static PROFILE: OnceLock<(std::path::PathBuf, std::path::PathBuf)> = OnceLock::new();

/// Has `quit` write the run's profile (`ffx-rust --profile FILE` under the
/// harness: the process ends there, before the program's own report) to
/// `path`, the `FFX.exe` functions named from the `ffx.sym` at `sym`.
pub fn profile_at_quit(path: std::path::PathBuf, sym: std::path::PathBuf) {
    let _ = PROFILE.set((path, sym));
}

/// Writes the profile [`profile_at_quit`] asked for: every Unicorn function
/// of every thread, running or ended (`crate::report::functions_tsv`),
/// through a temporary file, so a reader never sees half of it.
fn write_profile(shared: &Shared) {
    let Some((path, sym)) = PROFILE.get() else {
        return;
    };
    let syms = crate::report::Symbols::read(sym);
    let profile = shared.profile();
    let functions = crate::report::functions(shared, &profile, &syms);
    let entries = crate::report::entries_path(path);
    let _ = std::fs::write(
        &entries,
        crate::report::entries_tsv(shared, &profile, &syms),
    );
    let tmp = path.with_extension("part");
    let r = std::fs::write(&tmp, crate::report::functions_tsv(&functions))
        .and_then(|()| std::fs::rename(&tmp, path));
    match r {
        Ok(()) => eprintln!(
            "profile: {} functions in Unicorn, {}",
            functions.len(),
            path.display()
        ),
        Err(e) => eprintln!("profile {}: {e}", path.display()),
    }
}

/// Ends the process at once (the hook's `quit` does the same: the game's
/// own shutdown from the harness thread can deadlock).
fn quit(shared: &Shared) {
    // the overlays' Rust/Unicorn call counts (the report is never written)
    for line in crate::library::summary() {
        eprintln!("{line}");
    }
    write_profile(shared);
    // run-all.sh fails a run with any (docs/harness.md)
    eprintln!("{}", crate::log::bad_accesses_line());
    eprintln!(
        "harness: {} seeded jobs, {} cloth waits",
        SEEDED_JOBS.load(Ordering::Relaxed),
        CLOTH_WAITS.load(Ordering::Relaxed)
    );
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        // SAFETY: ends this process.
        unsafe {
            TerminateProcess(GetCurrentProcess(), 0);
        }
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_names_and_masks() {
        assert_eq!(buttons("start").unwrap(), 0x800);
        assert_eq!(buttons("up+circle").unwrap(), 0x1020);
        assert_eq!(buttons("0x40").unwrap(), 0x40);
        assert!(buttons("jump").is_err());
    }

    #[test]
    fn ranges_parse_as_the_hook_reads_them() {
        let r = parse_range("*0x023c44e4+0x4d4:16").unwrap();
        assert_eq!(
            (r.addr, r.deref, r.off, r.len),
            (0x023c_44e4, true, 0x4d4, 16)
        );
        let r = parse_range("0x01325d94:4").unwrap();
        assert_eq!((r.addr, r.deref, r.off, r.len), (0x0132_5d94, false, 0, 4));
        assert!(parse_range("0x10").is_err());
    }

    #[test]
    fn a_shot_is_a_bottom_up_bmp() {
        let px = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let b = bmp(1, 2, &px);
        assert_eq!(&b[..2], b"BM");
        assert_eq!(b.len(), 54 + 8);
        // the last row first
        assert_eq!(&b[54..58], &[5, 6, 7, 8]);
    }
}
