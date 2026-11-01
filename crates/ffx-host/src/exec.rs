//! The mixed executor (docs/host.md, "Rust and Unicorn"): Rust ports from
//! the function table, everything else from `FFX.exe`'s `.text` (and the
//! player's C runtime DLLs) in Unicorn, on one arena.
//!
//! Unicorn shares the arena's host memory (`mem_map_ptr`). Every ported
//! address and every import stub holds an `int3` in the arena's copy of
//! the code; the interrupt stops the emulator and the executor runs the
//! port or the import in Rust, then resumes at a small return stub
//! (`ret n`, or `fld qword [cell]; ret n` for a float) so the caller sees
//! the original's convention. A port that calls an address goes through
//! [`Exec::call`]: the port when there is one, else Unicorn, its arguments
//! pushed below the current `esp` and a sentinel as the return address.
//! Nothing runs inside a Unicorn hook except bookkeeping, so the calls nest
//! as plain Rust recursion.
//!
//! Threads (docs/host.md, "Threads"): [`Shared`] is what every game thread
//! sees (the arena, the image, the table, the imports, the heap, the
//! files, the synchronisation objects); each game thread is one host
//! thread with its own [`Exec`]: its own Unicorn instance over the same
//! arena regions, its own stack, thread block and return stubs.
//!
//! Development only: the release runs no `FFX.exe` code (rewrite-plan, M5).

pub mod seh;
pub mod spot;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::{Duration, Instant};

use ffx_bridge::Mem as _;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

use crate::arena::{Arena, ArenaMem, PAGE};
use crate::dll::Dll;
use crate::image::Image;
use crate::imports::{Heap, ImportFn, Imports, ThreadCrt};
use crate::sync::Objects;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};
use crate::{Error, Result};
use ffx_fpu::x87::Op;

/// `int3`.
const INT3: u8 = 0xcc;
/// The x87 control word Windows starts threads with (53-bit precision, all
/// exceptions masked), as the game thread runs (docs/porting.md).
pub const FPU_CONTROL: u16 = 0x027f;
/// Each emulator's translation buffer (`Exec::on_thread`).
const TCG_BUFFER: u32 = 64 << 20;
/// The main thread's stack.
const STACK_SIZE: u32 = 0x0020_0000;
/// Every other thread's stack (the game asks for less; the emulated code
/// never sees the difference, and some threads nest ports deeply).
const THREAD_STACK_SIZE: u32 = 0x0010_0000;
/// The shared stub page: import stubs and data cells.
const STUB_SIZE: u32 = 0x0001_0000;
/// Where the trampolines start in the stub page (32 bytes each).
const TRAMPOLINES: u32 = 0xc000;
/// Where the executor's own regions start looking for room (above the
/// image).
const REGION_HINT: u32 = 0x0300_0000;
/// The `ret n` stubs: `n` stack words from 0 to this.
const MAX_RET_WORDS: u32 = 64;
/// The main thread's id (`GetCurrentThreadId`).
pub const MAIN_THREAD: u32 = 0x1004;
/// How often the watchdog looks at the stop flag and the budget.
const WATCH: Duration = Duration::from_millis(20);

// Unicorn's own stop request, which (unlike the rest of the API) may be
// called from another thread: its timeout thread does the same.
unsafe extern "C" {
    fn uc_emu_stop(uc: *mut std::ffi::c_void) -> i32;
}
/// A thread block: GDT, TEB, PEB, its return stubs page.
const THREAD_BLOCK: u32 = 4 * PAGE;
/// The TEB's TLS slots (`TlsGetValue`): `TlsSlots` at `0xe10`, 64 of them.
pub const TEB_TLS: u32 = 0xe10;

/// What stopped the emulator.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    /// An `int3` at this address (a port, an import stub or the sentinel).
    Int3(u32),
    /// Another interrupt.
    Interrupt(u32, u32),
    /// An access to unmapped memory.
    BadAccess(u32, bool),
}

/// Unicorn's side of the state (what the hooks touch).
#[derive(Debug, Default)]
struct State {
    event: Option<Event>,
    /// The profile (when [`Exec::cover`] is on).
    profile: Profile,
    last_tick: u64,
    /// The last basic blocks run (`FFX_HOST_TRACE=n`), oldest first.
    recent: std::collections::VecDeque<u32>,
    recent_cap: usize,
    /// Instructions run (when counting is on).
    instructions: u64,
    /// x87 instructions computed by `ffx_fpu` instead of the emulator.
    x87: u64,
}

/// Where emulated time went: basic blocks run, how often, and the clock
/// samples that landed in each (one per millisecond tick of
/// [`Exec::cover`]'s ticker, so a sample is about a millisecond).
#[derive(Debug, Default, Clone)]
pub struct Profile {
    /// Executions per basic block.
    pub blocks: HashMap<u32, u64>,
    /// Clock samples per basic block.
    pub samples: HashMap<u32, u64>,
}

impl Profile {
    /// Adds `other` in.
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.blocks {
            *self.blocks.entry(*k).or_default() += v;
        }
        for (k, v) in &other.samples {
            *self.samples.entry(*k).or_default() += v;
        }
    }
}

/// What the executor saw on the way, for the report.
#[derive(Debug, Default, Clone)]
pub struct Stats {
    /// Port entries by address (from Unicorn or from Rust).
    pub ports: BTreeMap<u32, u64>,
    /// Import calls by name.
    pub imports: BTreeMap<String, u64>,
    /// Unstubbed imports, in the order first called.
    pub missing_imports: Vec<String>,
    /// Calls from Rust into Unicorn.
    pub unicorn_calls: u64,
    /// Emulator starts (each resume counts).
    pub starts: u64,
    /// Time spent inside the emulator.
    pub emulated: Duration,
    /// Message boxes the game showed (caption: text).
    pub messages: Vec<String>,
    /// Milestones reached (import names), in order.
    pub milestones: Vec<String>,
    /// Instructions emulated (with [`Exec::count_instructions`]).
    pub instructions: u64,
    /// x87 instructions (`fsin`, `fpatan`, `fprem`, ...) computed exactly
    /// by `ffx_fpu` instead of Unicorn's own x87.
    pub x87: u64,
    /// Exceptions dispatched through the SEH chain ([`seh`]).
    pub exceptions: u64,
}

impl Stats {
    /// Adds `other` in.
    pub fn merge(&mut self, o: &Self) {
        for (k, v) in &o.ports {
            *self.ports.entry(*k).or_default() += v;
        }
        for (k, v) in &o.imports {
            *self.imports.entry(k.clone()).or_default() += v;
        }
        for m in &o.missing_imports {
            if !self.missing_imports.contains(m) {
                self.missing_imports.push(m.clone());
            }
        }
        self.unicorn_calls += o.unicorn_calls;
        self.starts += o.starts;
        self.emulated += o.emulated;
        self.messages.extend(o.messages.iter().cloned());
        self.milestones.extend(o.milestones.iter().cloned());
        self.instructions += o.instructions;
        self.x87 += o.x87;
        self.exceptions += o.exceptions;
    }
}

/// Why a run ended without an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// The called function returned.
    Returned,
    /// A milestone asked to stop ([`Exec::stop_at`]).
    Milestone(String),
    /// The wall-clock budget ran out at this `eip`.
    Budget(u32),
    /// An error (an unstubbed import, a fault) at this `eip`.
    Failed(String, u32),
    /// This thread called `ExitThread` (ends only this thread).
    ThreadExit(u32),
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Returned => write!(f, "returned"),
            Self::Milestone(m) => write!(f, "milestone: {m}"),
            Self::Budget(eip) => write!(f, "time budget used up at {eip:#010x}"),
            Self::Failed(why, eip) => write!(f, "{why} (eip {eip:#010x})"),
            Self::ThreadExit(code) => write!(f, "ExitThread({code:#x})"),
        }
    }
}

/// A game thread, as the report lists it.
#[derive(Debug, Clone)]
pub struct ThreadInfo {
    /// Its id.
    pub id: u32,
    /// Its handle.
    pub handle: u32,
    /// The thread routine.
    pub entry: u32,
    /// Its parameter.
    pub param: u32,
    /// Who started it (`CreateThread`, the thread pool).
    pub via: &'static str,
    /// Whether it ran yet (`CREATE_SUSPENDED` threads wait for
    /// `ResumeThread`).
    pub started: bool,
    /// How it ended (`None`: still running at the report).
    pub outcome: Option<String>,
    /// When it started, from the run's start.
    pub at: Duration,
    /// Its emulated time, starts and imports.
    pub stats: Option<Stats>,
    /// Its return addresses when it failed.
    pub trace: Vec<u32>,
}

/// Which side an import of `FFX.exe` or a DLL went to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// A Rust stand-in.
    Rust,
    /// The player's DLL, run in Unicorn.
    Dll,
    /// Nothing yet: stops the run when called.
    Missing,
}

/// A trap's Rust side: the executor, the trap's tag, the arguments.
pub type TrapFn = fn(&mut Exec, u32, &Args) -> Ret;

/// An `int3` the host placed at run time (a COM method's vtable slot,
/// `crate::com`): what runs there and how it returns.
#[derive(Debug, Clone, Copy)]
pub struct Trap {
    /// The Rust side.
    pub run: TrapFn,
    /// Handed to `run` (which method of which class).
    pub tag: u32,
    /// The convention.
    pub conv: Conv,
    /// Stack words.
    pub words: usize,
}

/// What every game thread shares.
pub struct Shared {
    arena: Arc<Arena>,
    image: Image,
    /// The player's C runtime DLLs, in load order.
    pub dlls: Vec<Dll>,
    table: Table,
    imports: Imports,
    /// Import stub address → index into `imports`' bound list.
    stubs: HashMap<u32, usize>,
    /// Every import of `FFX.exe` and the DLLs: `DLL!name` (`(from X)` for a
    /// DLL's), and where it went.
    pub routes: Vec<(String, Route)>,
    /// The C runtime's heap (also `HeapAlloc`'s).
    pub heap: Mutex<Heap>,
    /// Open files.
    pub files: Mutex<crate::files::Files>,
    /// Events, semaphores, mutexes, thread handles, critical sections.
    pub sync: Objects,
    /// `_onexit` / `atexit` functions, in registration order.
    pub at_exit: Mutex<Vec<u32>>,
    /// The command line (`GetCommandLineA`, `_acmdln`), an arena address.
    pub cmdline: u32,
    stop: Mutex<Option<Stop>>,
    stop_at: RwLock<BTreeSet<String>>,
    deadline: Mutex<Option<Instant>>,
    lenient: AtomicBool,
    cover: AtomicBool,
    count: AtomicBool,
    handles: AtomicU32,
    thread_ids: AtomicU32,
    tls_next: AtomicU32,
    tick: Arc<AtomicU64>,
    started: Instant,
    /// Every game thread asked for.
    pub threads: Mutex<Vec<ThreadInfo>>,
    /// The profile of every thread that ended.
    finished: Mutex<Profile>,
    /// Every live emulator (thread id, Unicorn handle), for the watchdog.
    emulators: Mutex<Vec<(u32, usize)>>,
    /// Every live thread's host entries, for the harness's `stacks`.
    pub spots: Mutex<Vec<Arc<spot::Spot>>>,
    /// Thread-pool work items: handle → (callback, context).
    pub pool_work: Mutex<HashMap<u32, (u32, u32)>>,
    running: AtomicU32,
    /// The x87 instructions `ffx_fpu` computes, at exact instruction
    /// starts (patched to `int3`; game addresses).
    x87: Arc<HashMap<u32, Op>>,
    /// Possible ones in modules of unknown builds (a code hook each).
    x87_watch: Vec<(u32, Op)>,
    /// Traps placed at run time ([`Shared::add_trap`]).
    traps: RwLock<HashMap<u32, Trap>>,
    /// The D3D11 device and DXGI (`crate::gfx`), once created.
    pub gfx: crate::gfx::Slot,
    /// Trampolines to the originals that ports wrap: address → trampoline
    /// ([`Exec::call_original`]).
    originals: HashMap<u32, u32>,
    /// The virtual clock, the frame boundary and the turns
    /// (`crate::clock`).
    pub clock: crate::clock::Clock,
    /// FMOD's imports on the Rust mixer (`crate::audio`).
    pub audio: crate::audio::Audio,
    /// Iggy's imports on `ffx-swf` (`crate::iggy`).
    pub iggy: crate::iggy::Iggy,
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shared")
            .field("image", &self.image.base)
            .field("dlls", &self.dlls.len())
            .field("ports", &self.table.len())
            .finish_non_exhaustive()
    }
}

/// FFX.exe's `MSVCR110` imports that keep their Rust side when the
/// player's DLLs are loaded: start-up and exit (the run's own), the heap
/// (the DLL's `malloc` ends on the same heap through `HeapAlloc`), the
/// copies and `rand` (per thread, MSVC's sequence). Everything else goes to
/// the DLL (docs/host.md, "The C runtime DLLs").
pub const RUST_WITH_DLLS: [&str; 26] = [
    "__set_app_type",
    "__setusermatherr",
    "__getmainargs",
    "_initterm",
    "_initterm_e",
    "_onexit",
    "__dllonexit",
    "__crtGetShowWindowMode",
    "_crt_debugger_hook",
    "__crtSetUnhandledExceptionFilter",
    "_amsg_exit",
    "exit",
    "_exit",
    "_cexit",
    "abort",
    "malloc",
    "calloc",
    "free",
    "_recalloc",
    "_calloc_crt",
    "memset",
    "memcpy",
    "memmove",
    "_strdup",
    "rand",
    "srand",
];

impl Shared {
    /// Lays out the shared stub page, binds every import of `image` and
    /// `dlls`, and marks the ports in `.text`.
    ///
    /// # Errors
    /// When a mapping fails.
    pub fn new(
        arena: Arc<Arena>,
        image: Image,
        dlls: Vec<Dll>,
        table: Table,
        files: crate::files::Files,
    ) -> Result<Arc<Self>> {
        let stubs_at = arena.map_anywhere(REGION_HINT, STUB_SIZE, "import stubs")?;
        let heap = Heap::new(&arena)?;
        let mut s = Self {
            arena,
            image,
            dlls,
            table,
            imports: Imports::new(),
            stubs: HashMap::new(),
            routes: Vec::new(),
            heap: Mutex::new(heap),
            files: Mutex::new(files),
            sync: Objects::default(),
            at_exit: Mutex::new(Vec::new()),
            cmdline: 0,
            stop: Mutex::new(None),
            stop_at: RwLock::new(BTreeSet::new()),
            deadline: Mutex::new(None),
            lenient: AtomicBool::new(false),
            cover: AtomicBool::new(false),
            count: AtomicBool::new(false),
            handles: AtomicU32::new(0x100),
            thread_ids: AtomicU32::new(MAIN_THREAD),
            tls_next: AtomicU32::new(0),
            tick: Arc::new(AtomicU64::new(0)),
            started: Instant::now(),
            threads: Mutex::new(Vec::new()),
            finished: Mutex::new(Profile::default()),
            emulators: Mutex::new(Vec::new()),
            spots: Mutex::new(Vec::new()),
            pool_work: Mutex::new(HashMap::new()),
            running: AtomicU32::new(0),
            x87: Arc::new(HashMap::new()),
            x87_watch: Vec::new(),
            traps: RwLock::new(HashMap::new()),
            gfx: crate::gfx::Slot::default(),
            originals: HashMap::new(),
            clock: crate::clock::Clock::default(),
            audio: crate::audio::Audio::default(),
            iggy: crate::iggy::Iggy::default(),
        };
        s.bind_imports(stubs_at);
        s.build_trampolines(stubs_at + TRAMPOLINES);
        for addr in s.table.addresses().collect::<Vec<_>>() {
            if s.image.is_code(addr) {
                s.arena.fill(addr, &[INT3]);
            }
        }
        s.mark_x87();
        let s = Arc::new(s);
        let weak = Arc::downgrade(&s);
        std::thread::Builder::new()
            .name("ffx-host watchdog".into())
            .spawn(move || watchdog(&weak))
            .map_err(Error::Io)?;
        Ok(s)
    }

    /// Copies each wrapped original's prologue (`Table::wrap`) into a
    /// trampoline that jumps on into the original's body, before the
    /// original's entry gets its `int3`. A prologue that doesn't match the
    /// image leaves that original without a trampoline (its wrapper's
    /// [`Exec::call_original`] fails).
    fn build_trampolines(&mut self, mut at: u32) {
        let mem = ArenaMem {
            arena: Arc::clone(&self.arena),
        };
        for (addr, prologue) in self.table.wrapped().collect::<Vec<_>>() {
            let mut now = vec![0u8; prologue.len()];
            ffx_bridge::Mem::read_bytes(&mem, addr, &mut now);
            if now != prologue || prologue.len() > 24 {
                crate::log::note(&format!(
                    "no trampoline for {addr:#010x}: its code is {now:02x?}, not {prologue:02x?}"
                ));
                continue;
            }
            let mut code = prologue.to_vec();
            let len = u32::try_from(prologue.len()).unwrap_or(0);
            let from = at + len + 5;
            code.push(0xe9);
            code.extend_from_slice(&(addr + len).wrapping_sub(from).to_le_bytes());
            self.arena.fill(at, &code);
            self.originals.insert(addr, at);
            at += 32;
        }
    }

    /// Finds the x87 instructions `ffx_fpu` computes in `FFX.exe` and the
    /// DLLs: exact sites get an `int3` (the interrupt hook computes them),
    /// a module of an unknown build a code hook on every possible one
    /// (docs/host.md, "x87").
    fn mark_x87(&mut self) {
        let mut exact = HashMap::new();
        let modules = std::iter::once((self.image.base, &self.image.x87))
            .chain(self.dlls.iter().map(|d| (d.base, &d.x87)));
        for (base, sites) in modules {
            for &(rva, op) in &sites.at {
                if sites.exact() {
                    exact.insert(base + rva, op);
                } else {
                    self.x87_watch.push((base + rva, op));
                }
            }
        }
        for &at in exact.keys() {
            self.arena.fill(at, &[INT3]);
        }
        self.x87 = Arc::new(exact);
    }

    /// Whether the run is ending or out of time.
    pub(crate) fn overdue(&self) -> bool {
        self.sync.stopping()
            || self
                .deadline
                .lock()
                .ok()
                .and_then(|d| *d)
                .is_some_and(|d| Instant::now() >= d)
    }

    /// Places a trap at `at`, which must hold an `int3` in a region the
    /// caller mapped: emulated code reaching it (or [`Exec::call`]) runs
    /// `trap.run`.
    pub fn add_trap(&self, at: u32, trap: Trap) {
        if let Ok(mut t) = self.traps.write() {
            t.insert(at, trap);
        }
    }

    /// The trap at `at`.
    #[must_use]
    pub fn trap(&self, at: u32) -> Option<Trap> {
        self.traps.read().ok().and_then(|t| t.get(&at).copied())
    }

    /// The loaded DLL called `name` (any case).
    #[must_use]
    pub fn dll(&self, name: &str) -> Option<&Dll> {
        self.dlls.iter().find(|d| d.name.eq_ignore_ascii_case(name))
    }

    /// Points every IAT slot of `FFX.exe` and the DLLs at its own `int3`
    /// stub (the Rust side), at a DLL's export, or (data imports) at a cell.
    fn bind_imports(&mut self, stubs_at: u32) {
        let mut at = stubs_at;
        let mut cells = stubs_at + 0x8000;
        let mut mem = ArenaMem {
            arena: Arc::clone(&self.arena),
        };
        let cmdline = cells;
        mem.write_bytes(cmdline, b"FFX.exe\0");
        cells += 0x10;
        self.cmdline = cmdline;
        let mut index = 0usize;
        let exe = self.image.imports.clone();
        for imp in &exe {
            let full = format!("{}!{}", imp.dll, imp.name);
            let dll = self.dll(&imp.dll).cloned();
            // a data import: the DLL's own variable, else a cell
            if let Some(&(_, v)) = crate::imports::DATA.iter().find(|(n, _)| *n == full) {
                let target = if let Some(x) = dll.as_ref().and_then(|d| d.exports.get(&imp.name)) {
                    self.routes.push((full, Route::Dll));
                    *x
                } else {
                    let v = if imp.name == "_acmdln" { cmdline } else { v };
                    ffx_bridge::Mem::write(&mut mem, cells, v);
                    if imp.name == "_HUGE" {
                        ffx_bridge::Mem::write(&mut mem, cells + 4, 0x7ff0_0000u32);
                    }
                    self.routes.push((full, Route::Rust));
                    cells += 0x10;
                    cells - 0x10
                };
                ffx_bridge::Mem::write(&mut mem, imp.slot, target);
                continue;
            }
            if let Some(d) = &dll
                && !RUST_WITH_DLLS.contains(&imp.name.as_str())
                && let Some(&x) = d.exports.get(&imp.name)
            {
                ffx_bridge::Mem::write(&mut mem, imp.slot, x);
                self.routes.push((full, Route::Dll));
                continue;
            }
            self.imports.bind(index, &imp.dll, &imp.name, None);
            let route = if self.imports.bound(index).is_some_and(|b| b.run.is_some()) {
                Route::Rust
            } else {
                Route::Missing
            };
            self.routes.push((full, route));
            self.arena.fill(at, &[INT3]);
            self.arena.fill(imp.slot, &at.to_le_bytes());
            self.stubs.insert(at, index);
            index += 1;
            at += 4;
        }
        let dlls = self.dlls.clone();
        for d in &dlls {
            for imp in &d.imports {
                let full = format!("{}!{} (from {})", imp.dll, imp.name, d.name);
                if let Some(x) = self
                    .dll(&imp.dll)
                    .and_then(|t| t.exports.get(&imp.name).copied())
                {
                    ffx_bridge::Mem::write(&mut mem, imp.slot, x);
                    self.routes.push((full, Route::Dll));
                    continue;
                }
                self.imports.bind(index, &imp.dll, &imp.name, Some(&d.name));
                let route = if self.imports.bound(index).is_some_and(|b| b.run.is_some()) {
                    Route::Rust
                } else {
                    Route::Missing
                };
                self.routes.push((full, route));
                self.arena.fill(at, &[INT3]);
                self.arena.fill(imp.slot, &at.to_le_bytes());
                self.stubs.insert(at, index);
                index += 1;
                at += 4;
            }
        }
    }

    /// Ends the whole run: every thread unwinds with [`Error::Stopped`].
    pub fn stop_all(&self, why: Stop) {
        if let Ok(mut s) = self.stop.lock()
            && s.is_none()
        {
            *s = Some(why);
        }
        self.sync.stop();
    }

    /// Why the whole run ended, if it did.
    #[must_use]
    pub fn stopped(&self) -> Option<Stop> {
        self.stop.lock().ok().and_then(|s| s.clone())
    }

    /// A fresh handle value.
    pub fn handle(&self) -> u32 {
        self.handles.fetch_add(4, Ordering::Relaxed) + 4
    }

    /// `TlsAlloc`: the next slot (64 of them).
    pub fn tls_alloc(&self) -> Option<u32> {
        let i = self.tls_next.fetch_add(1, Ordering::Relaxed);
        (i < 64).then_some(i)
    }

    /// Game threads still running.
    #[must_use]
    pub fn running_threads(&self) -> u32 {
        self.running.load(Ordering::Acquire)
    }

    /// The image.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// The arena.
    #[must_use]
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// The name of bound import `i` (`DLL!name`).
    #[must_use]
    pub fn import_name(&self, i: usize) -> Option<&str> {
        self.imports.bound(i).map(|b| b.name.as_str())
    }

    /// The function-table entry's name at `at`.
    #[must_use]
    pub fn port_name(&self, at: u32) -> Option<&'static str> {
        self.table.get(at).map(|e| e.name)
    }
}

/// Stops every emulator (at its next block) when the run ends or runs out
/// of time, so the threads see it; they resume or unwind themselves.
fn watchdog(shared: &std::sync::Weak<Shared>) {
    loop {
        std::thread::sleep(WATCH);
        let Some(s) = shared.upgrade() else { return };
        if s.overdue()
            && let Ok(list) = s.emulators.lock()
        {
            for &(_, h) in list.iter() {
                // SAFETY: a live handle: an `Exec` removes its own under this
                // lock before its emulator closes.
                unsafe {
                    uc_emu_stop(h as *mut std::ffi::c_void);
                }
            }
        }
    }
}

/// One game thread's executor.
pub struct Exec {
    uc: Unicorn<'static, State>,
    shared: Arc<Shared>,
    /// The game thread's id.
    pub id: u32,
    stack: (u32, u32),
    sentinel: u32,
    ret_stubs: u32,
    ret_float_stubs: u32,
    float_cell: u32,
    /// The thread environment block.
    pub teb: u32,
    generation: u64,
    mapped: BTreeSet<u32>,
    stopped: Option<Stop>,
    /// The counters.
    pub stats: Stats,
    /// Return addresses on the stack when this thread failed (innermost
    /// first), for the report.
    pub trace: Vec<u32>,
    /// The last basic blocks run before this thread failed, oldest first
    /// (with `FFX_HOST_TRACE=n`).
    pub recent: Vec<u32>,
    /// The C runtime's per-thread state (`errno`'s cousin `GetLastError`,
    /// `rand`).
    pub crt: ThreadCrt,
    /// Exceptions in flight ([`seh`]).
    seh: seh::State,
    /// The thread's scratch blocks in the arena (`ffx_bridge::route::Scratch`),
    /// allocated on first use.
    scratch: [u32; 3],
    /// The ports' stack frames (`StackFrame`) on the game stack: each
    /// frame's address and the `esp` below which it was carved.
    frames: Vec<(u32, u32)>,
    /// Nesting of [`ffx_bridge::route::Router::call_x87`] (its stubs).
    x87_depth: u32,
    /// How the trap that runs now returns, when its [`Trap`]'s convention
    /// can't say (an overlay port's pops or tail jump: [`TrapExit`]).
    pub(crate) trap_exit: Option<TrapExit>,
    /// How many of [`crate::library`]'s overlay counting hooks this
    /// thread's emulator has.
    pub(crate) overlay_hooks: usize,
    /// Its host entries, for the harness's `stacks`.
    spot: Arc<spot::Spot>,
}

/// A trap's own return ([`Exec::trap_exit`], set by its Rust side): the
/// argument words it popped, or a tail jump (the caller's stack as it was
/// at the entry, `eax` the target).
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrapExit {
    pub pops: u32,
    pub jump: Option<u32>,
}

impl Drop for Exec {
    fn drop(&mut self) {
        let id = self.id;
        if let Ok(mut l) = self.shared.emulators.lock() {
            l.retain(|&(t, _)| t != id);
        }
        if let Ok(mut l) = self.shared.spots.lock() {
            l.retain(|s| s.id != id);
        }
    }
}

impl std::fmt::Debug for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exec")
            .field("thread", &self.id)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl Exec {
    /// The main thread's executor over `shared`.
    ///
    /// # Errors
    /// When a mapping fails.
    pub fn new(shared: Arc<Shared>) -> Result<Self> {
        Self::on_thread(shared, MAIN_THREAD, STACK_SIZE)
    }

    /// An executor for game thread `id`: its own Unicorn instance over the
    /// arena, its own stack, thread block and return stubs.
    ///
    /// # Errors
    /// When a mapping fails.
    pub fn on_thread(shared: Arc<Shared>, id: u32, stack_size: u32) -> Result<Self> {
        let arena = Arc::clone(&shared.arena);
        let stack_base = arena.map_anywhere(REGION_HINT, stack_size, &format!("stack {id:#x}"))?;
        let block = arena.map_anywhere(REGION_HINT, THREAD_BLOCK, &format!("thread {id:#x}"))?;
        let stubs_at = block + 3 * PAGE;

        // Windows: the translation buffers commit through our handler
        // (crate::tcg_commit)
        #[cfg(windows)]
        crate::tcg_commit::install();
        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, State::default())?;
        // The translation buffer: QEMU's default is 1 GB per instance on a
        // 64-bit host, one instance per game thread (33 on the way to the
        // opening screen); on Windows an endpoint-security hook crashed
        // reserving them (docs/host.md, "Windows"). 64 MB each.
        // SAFETY: a plain control call on our own fresh instance, before
        // anything initialises it.
        unsafe {
            unicorn_engine::uc_ctl(
                uc.get_handle(),
                unicorn_engine::ControlType(
                    unicorn_engine::ControlType::TCG_BUFFER_SIZE.0
                        | unicorn_engine::ControlType::IO_WRITE.0,
                ),
                TCG_BUFFER,
            )
        }
        .and(Ok(()))?;
        // A Core 2: no SSE4.2, whose string instructions Unicorn gets wrong
        // (ffx-oracle, the same choice).
        uc.ctl_set_cpu_model(2)?;

        let mut exec = Self {
            uc,
            shared,
            id,
            stack: (stack_base, stack_base + stack_size),
            sentinel: stubs_at,
            ret_stubs: stubs_at + 0x10,
            ret_float_stubs: stubs_at + 0x10 + 4 * (MAX_RET_WORDS + 1),
            float_cell: stubs_at + 0xf00,
            teb: block + PAGE,
            generation: u64::MAX,
            mapped: BTreeSet::new(),
            stopped: None,
            stats: Stats::default(),
            trace: Vec::new(),
            recent: Vec::new(),
            crt: ThreadCrt::default(),
            seh: seh::State::default(),
            scratch: [0; 3],
            frames: Vec::new(),
            x87_depth: 0,
            trap_exit: None,
            overlay_hooks: 0,
            spot: Arc::new(spot::Spot::new(id, (stack_base, stack_base + stack_size))),
        };
        if let Ok(mut l) = exec.shared.emulators.lock() {
            l.push((id, exec.uc.get_handle() as usize));
        }
        if let Ok(mut l) = exec.shared.spots.lock() {
            l.push(Arc::clone(&exec.spot));
        }
        exec.write_stubs(stubs_at);
        exec.sync_regions()?;
        exec.thread_block(block)?;
        exec.add_hooks()?;
        if exec.shared.cover.load(Ordering::Relaxed) {
            exec.add_profile_hook()?;
        }
        if exec.shared.count.load(Ordering::Relaxed) {
            exec.add_count_hook()?;
        }
        let recent = std::env::var("FFX_HOST_TRACE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        if recent > 0 {
            exec.uc.get_data_mut().recent_cap = recent;
            exec.uc.add_block_hook(1, 0, |uc, addr, _| {
                let d = uc.get_data_mut();
                if d.recent.len() == d.recent_cap {
                    d.recent.pop_front();
                }
                #[allow(clippy::cast_possible_truncation)] // a 32-bit address
                d.recent.push_back(addr as u32);
            })?;
        }
        let top = exec.stack.1 - 0x100;
        exec.uc.reg_write(RegisterX86::ESP, u64::from(top))?;
        exec.uc
            .reg_write(RegisterX86::FPCW, u64::from(FPU_CONTROL))?;
        Ok(exec)
    }

    /// What every thread shares.
    #[must_use]
    pub fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }

    /// Whether this is the main thread.
    #[must_use]
    pub fn is_main(&self) -> bool {
        self.id == MAIN_THREAD
    }

    /// The arena.
    #[must_use]
    pub fn arena(&self) -> &Arc<Arena> {
        &self.shared.arena
    }

    /// The arena as [`ffx_bridge::Mem`].
    #[must_use]
    pub fn mem(&self) -> ArenaMem {
        ArenaMem {
            arena: Arc::clone(&self.shared.arena),
        }
    }

    /// The image.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.shared.image
    }

    /// The function table.
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.shared.table
    }

    /// The open files.
    pub fn files(&self) -> MutexGuard<'_, crate::files::Files> {
        self.shared
            .files
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// `malloc` on the shared heap: a 16-byte aligned block, or 0.
    #[must_use]
    pub fn alloc(&self, size: u32) -> u32 {
        let mut mem = self.mem();
        self.heap().alloc(&mut mem, size)
    }

    /// `free` on the shared heap.
    pub fn release(&self, p: u32) {
        let mem = self.mem();
        self.heap().release(&mem, p);
    }

    /// A heap block's usable size.
    #[must_use]
    pub fn size_of(&self, p: u32) -> u32 {
        let mem = self.mem();
        self.heap().size_of(&mem, p)
    }

    fn heap(&self) -> MutexGuard<'_, Heap> {
        self.shared
            .heap
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// A fresh handle value.
    #[must_use]
    pub fn new_handle(&self) -> u32 {
        self.shared.handle()
    }

    /// This thread's profile so far (with [`Exec::cover`] on).
    #[must_use]
    pub fn profile(&self) -> &Profile {
        &self.uc.get_data().profile
    }

    /// Every thread's profile: this one's and those of the threads that
    /// ended.
    #[must_use]
    pub fn all_profile(&self) -> Profile {
        let mut p = self
            .shared
            .finished
            .lock()
            .map(|f| f.clone())
            .unwrap_or_default();
        p.merge(self.profile());
        p
    }

    /// Profiles every basic block run, by count and clock samples (slower;
    /// threads started later profile too).
    ///
    /// # Errors
    /// When Unicorn refuses the hook.
    pub fn cover(&mut self) -> Result<()> {
        if !self.shared.cover.swap(true, Ordering::Relaxed) {
            let tick = Arc::downgrade(&self.shared.tick);
            std::thread::Builder::new()
                .name("ffx-host ticker".into())
                .spawn(move || {
                    while let Some(t) = tick.upgrade() {
                        t.fetch_add(1, Ordering::Relaxed);
                        drop(t);
                        std::thread::sleep(Duration::from_millis(1));
                    }
                })
                .map_err(Error::Io)?;
        }
        self.add_profile_hook()
    }

    fn add_profile_hook(&mut self) -> Result<()> {
        let tick = Arc::clone(&self.shared.tick);
        self.uc.add_block_hook(1, 0, move |uc, addr, _| {
            let t = tick.load(Ordering::Relaxed);
            let d = uc.get_data_mut();
            #[allow(clippy::cast_possible_truncation)] // a 32-bit address
            let addr = addr as u32;
            *d.profile.blocks.entry(addr).or_default() += 1;
            if t != d.last_tick {
                d.last_tick = t;
                *d.profile.samples.entry(addr).or_default() += 1;
            }
        })?;
        Ok(())
    }

    /// Counts every emulated instruction (much slower; for measuring the
    /// executor's speed against a run without it).
    ///
    /// # Errors
    /// When Unicorn refuses the hook.
    pub fn count_instructions(&mut self) -> Result<()> {
        self.shared.count.store(true, Ordering::Relaxed);
        self.add_count_hook()
    }

    fn add_count_hook(&mut self) -> Result<()> {
        self.uc.add_code_hook(1, 0, |uc, _, _| {
            uc.get_data_mut().instructions += 1;
        })?;
        Ok(())
    }

    /// Instructions emulated so far on this thread (with
    /// [`Exec::count_instructions`]).
    #[must_use]
    pub fn instructions(&self) -> u64 {
        self.uc.get_data().instructions
    }

    /// Stops the run when the import `name` (`USER32.dll!CreateWindowExW`)
    /// is first called: a milestone.
    pub fn stop_at(&mut self, name: &str) {
        if let Ok(mut s) = self.shared.stop_at.write() {
            s.insert(name.to_string());
        }
    }

    /// Ends the run after `budget` of wall-clock time.
    pub fn set_budget(&mut self, budget: Duration) {
        if let Ok(mut d) = self.shared.deadline.lock() {
            *d = Some(Instant::now() + budget);
        }
    }

    /// Lets unstubbed C runtime imports return 0 instead of stopping
    /// (they are still listed as missing).
    pub fn set_lenient(&mut self, on: bool) {
        self.shared.lenient.store(on, Ordering::Relaxed);
    }

    /// Ends the run (from a port or an import): every level of this thread
    /// unwinds with [`Error::Stopped`]. On the main thread, and for a
    /// milestone, an exit or the budget on any thread, the whole run ends.
    pub fn end(&mut self, why: Stop) {
        if self.stopped.is_none() {
            if let Stop::Milestone(m) = &why {
                self.stats.milestones.push(m.clone());
            }
            if matches!(why, Stop::Failed(..)) || (self.is_main() && matches!(why, Stop::Budget(_)))
            {
                self.trace = self.backtrace();
                self.recent = self.uc.get_data().recent.iter().copied().collect();
            }
            // a failed game thread ends alone and the run goes on: noted
            // now (the harness's `notes`), not only in the final report
            if matches!(why, Stop::Failed(..)) {
                let calls: Vec<String> = self
                    .trace
                    .iter()
                    .take(8)
                    .map(|a| format!("{a:#010x}"))
                    .collect();
                crate::log::note(&format!(
                    "thread {:#x} failed: {why}; returns {}",
                    self.id,
                    calls.join(" ")
                ));
            }
            let global = self.is_main() || matches!(why, Stop::Milestone(_) | Stop::Budget(_));
            if global {
                self.shared.stop_all(why.clone());
            }
            self.stopped = Some(why);
        }
    }

    /// Return addresses on this thread's stack, innermost first: words
    /// that point into `FFX.exe`'s or a DLL's code just after a `call`.
    #[must_use]
    pub fn backtrace(&self) -> Vec<u32> {
        let Ok(esp) = self.reg(RegisterX86::ESP) else {
            return Vec::new();
        };
        let mem = self.mem();
        let code =
            |a: u32| self.shared.image.is_code(a) || self.shared.dlls.iter().any(|d| d.contains(a));
        let after_call = |a: u32| {
            let b = |k: u32| mem.read::<u8>(a.wrapping_sub(k));
            b(5) == 0xe8
                || (b(2) == 0xff && (b(1) & 0x38) == 0x10)
                || (b(3) == 0xff && (b(2) & 0x38) == 0x10)
                || (b(6) == 0xff && (b(5) & 0x38) == 0x10)
        };
        let mut out = Vec::new();
        let mut p = esp;
        while p < self.stack.1 && out.len() < 24 {
            let w: u32 = mem.read(p);
            if code(w) && after_call(w) {
                out.push(w);
            }
            p += 4;
        }
        out
    }

    /// Whether this thread is to unwind (its own stop, or the run's).
    fn halted(&mut self) -> bool {
        if self.stopped.is_some() || self.seh.unwinding.is_some() {
            return true;
        }
        if self.shared.sync.stopping() {
            self.stopped = Some(self.shared.stopped().unwrap_or(Stop::Returned));
            // where the main thread was when another thread ended the run
            // (the budget while it waited: the report says on what)
            if self.is_main() && self.trace.is_empty() {
                self.trace = self.backtrace();
            }
            return true;
        }
        false
    }

    /// Why this thread stopped early, if it did.
    #[must_use]
    pub fn stopped(&self) -> Option<&Stop> {
        self.stopped.as_ref()
    }

    /// Calls the function at `addr` the way a port calls an original: the
    /// port when the table has one, the import's Rust side for an import
    /// stub, else the original in Unicorn.
    ///
    /// # Errors
    /// A fault in the emulated code, an unstubbed import, or a stop
    /// ([`Exec::stopped`]) while it ran.
    pub fn call(&mut self, addr: u32, conv: Conv, args: &Args) -> Result<Ret> {
        if self.halted() {
            return Err(Error::Stopped);
        }
        let r = if let Some(entry) = self.shared.table.get(addr).copied() {
            *self.stats.ports.entry(addr).or_default() += 1;
            let ret = if args.slots == 0 && matches!(entry.port, Port::Routed(_)) {
                self.run_port_from_rust(entry, args)?
            } else {
                self.run_port(entry, args)
            };
            if self.halted() {
                Err(Error::Stopped)
            } else {
                Ok(ret)
            }
        } else if let Some(&i) = self.shared.stubs.get(&addr) {
            let raise = self
                .shared
                .imports
                .bound(i)
                .is_some_and(|b| b.name.ends_with("!RaiseException"));
            if raise {
                // the dispatch runs in the executor's loop
                self.call_unicorn(addr, conv, args)
            } else if args.slots == 0 && !args.stack.is_empty() {
                // from Rust: the words on the game stack too, for the
                // imports that read past their declared words through
                // `slots` (`wsprintfW`'s varargs: a port's save file name
                // came out "000")
                self.on_game_stack(args, |e, a| e.run_import(i, a))
                    .and_then(|r| r)
            } else {
                self.run_import(i, args)
            }
        } else if let Some(t) = self.shared.trap(addr) {
            let ret = (t.run)(self, t.tag, args);
            if self.halted() {
                Err(Error::Stopped)
            } else {
                Ok(ret)
            }
        } else {
            self.call_unicorn(addr, conv, args)
        };
        // A failure anywhere ends the thread (the main thread: the run):
        // callers (ports, `_initterm`) can't all carry a `Result`, so every
        // level unwinds with `Stopped`.
        match r {
            Err(e) if !matches!(e, Error::Stopped) => {
                let eip = self.reg(RegisterX86::EIP).unwrap_or(0);
                self.end(Stop::Failed(e.to_string(), eip));
                Err(Error::Stopped)
            }
            r => r,
        }
    }

    /// Calls the original a wrapping port replaced (`Table::wrap`): the
    /// port the wrapper displaced when there was one (a `ported!` entry),
    /// else the original in Unicorn, through its trampoline.
    ///
    /// # Errors
    /// As [`Exec::call`]; an original without a trampoline is an
    /// [`Error::Interrupt`] at its address.
    pub fn call_original(&mut self, addr: u32, conv: Conv, args: &Args) -> Result<Ret> {
        if let Some(entry) = self.shared.table.wrapped_port(addr).copied() {
            *self.stats.ports.entry(addr).or_default() += 1;
            let ret = if args.slots == 0 && matches!(entry.port, Port::Routed(_)) {
                self.run_port_from_rust(entry, args)?
            } else {
                self.run_port(entry, args)
            };
            return if self.halted() {
                Err(Error::Stopped)
            } else {
                Ok(ret)
            };
        }
        let Some(&t) = self.shared.originals.get(&addr) else {
            self.end(Stop::Failed(
                format!("no trampoline for {addr:#010x}"),
                addr,
            ));
            return Err(Error::Stopped);
        };
        self.call(t, conv, args)
    }

    /// [`Exec::call`] for a cdecl function of word arguments, returning
    /// `eax`.
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn call_cdecl(&mut self, addr: u32, words: &[u32]) -> Result<u32> {
        let args = Args {
            stack: words.to_vec(),
            ..Args::default()
        };
        Ok(self.call(addr, Conv::Cdecl, &args)?.eax)
    }

    /// [`Exec::call`] for a stdcall function of word arguments, returning
    /// `eax`.
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn call_stdcall(&mut self, addr: u32, words: &[u32]) -> Result<u32> {
        let args = Args {
            stack: words.to_vec(),
            ..Args::default()
        };
        Ok(self.call(addr, Conv::Stdcall, &args)?.eax)
    }

    fn call_unicorn(&mut self, addr: u32, conv: Conv, args: &Args) -> Result<Ret> {
        self.stats.unicorn_calls += 1;
        let saved = self.save_regs()?;
        // below the current frame (a port runs between an original's call
        // and its return; its caller's words sit above esp)
        let esp = u32::try_from(self.uc.reg_read(RegisterX86::ESP)?).unwrap_or(0);
        let mut sp = (esp - 0x40) & !0xf;
        if sp < self.stack.0 + 0x1000 {
            return Err(Error::StackOverflow(addr));
        }
        let mut mem = self.mem();
        for &w in args.stack.iter().rev() {
            sp -= 4;
            ffx_bridge::Mem::write(&mut mem, sp, w);
        }
        sp -= 4;
        ffx_bridge::Mem::write(&mut mem, sp, self.sentinel);
        self.uc.reg_write(RegisterX86::ESP, u64::from(sp))?;
        if conv.register_args() >= 1 {
            self.uc.reg_write(RegisterX86::ECX, u64::from(args.ecx))?;
        }
        if conv.register_args() >= 2 {
            self.uc.reg_write(RegisterX86::EDX, u64::from(args.edx))?;
        }
        // the x87 stack is empty at a call
        let sw = self.uc.reg_read(RegisterX86::FPSW)?;
        self.uc.reg_write(RegisterX86::FPSW, sw & !0x3800)?;
        self.uc.reg_write(RegisterX86::FPTAG, 0xffff)?;
        // the esp the sentinel return leaves
        let pops = if conv.callee_pops() {
            4 * u32::try_from(args.stack.len()).unwrap_or(0)
        } else {
            0
        };
        let expect = sp + 4 + pops;
        self.seh.levels.push(expect);
        let run = self.run(addr, expect);
        self.seh.levels.pop();
        let ret = Ret {
            eax: self.reg(RegisterX86::EAX)?,
            edx: Some(self.reg(RegisterX86::EDX)?),
            st0: self.st0()?,
            sets_eax: true,
        };
        self.restore_regs(&saved)?;
        run?;
        Ok(ret)
    }

    /// Runs the game from its entry point (the CRT start-up, the static
    /// initialisers, `WinMain`) until it returns or stops; then ends every
    /// other thread.
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn run_entry(&mut self) -> Result<Stop> {
        let entry = self.shared.image.entry;
        let r = match self.call(entry, Conv::Stdcall, &Args::default()) {
            Ok(_) => Ok(Stop::Returned),
            Err(Error::Stopped) => Ok(self.stopped.clone().unwrap_or(Stop::Returned)),
            Err(e) => Err(e),
        };
        if let Ok(stop) = &r {
            self.shared.stop_all(stop.clone());
        }
        r
    }

    /// `DllMain(base, reason, 0)` of every loaded DLL, in load order
    /// (`DLL_PROCESS_ATTACH` 1 on the main thread, `DLL_THREAD_ATTACH` 2 on
    /// a new one).
    ///
    /// # Errors
    /// As [`Exec::call`]; a `DllMain` returning FALSE stops the run.
    pub fn attach_dlls(&mut self, reason: u32) -> Result<()> {
        let dlls: Vec<(String, u32, u32)> = self
            .shared
            .dlls
            .iter()
            .map(|d| (d.name.clone(), d.base, d.entry))
            .collect();
        for (name, base, entry) in dlls {
            let ok = self.call_stdcall(entry, &[base, reason, 0])?;
            if ok == 0 && reason == 1 {
                self.end(Stop::Failed(
                    format!("{name}: DllMain returned FALSE"),
                    entry,
                ));
                return Err(Error::Stopped);
            }
        }
        Ok(())
    }

    /// Waits up to `limit` for the game threads to end (after the run
    /// stopped, they unwind within a slice).
    pub fn join_threads(&self, limit: Duration) {
        let until = Instant::now() + limit;
        while self.shared.running_threads() > 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Emulates from `pc` until the sentinel returns with `esp` at
    /// `expect`, taking ports, imports and exceptions on the way.
    ///
    /// A sentinel return at an outer level's `esp` (a C++ catch continued
    /// in a frame older than a Rust port, [`seh`]) unwinds every Rust
    /// level to that one, which then returns as if its own call had.
    fn run(&mut self, mut pc: u32, expect: u32) -> Result<()> {
        loop {
            if self.halted() {
                return Err(Error::Stopped);
            }
            self.sync_regions()?;
            self.uc.get_data_mut().event = None;
            if self.shared.overdue() {
                if !self.shared.sync.stopping() {
                    self.end(Stop::Budget(pc));
                }
                self.halted();
                return Err(Error::Stopped);
            }
            self.stats.starts += 1;
            let t = Instant::now();
            let r = self.uc.emu_start(u64::from(pc), 0, 0, 0);
            self.stats.emulated += t.elapsed();
            let event = self.uc.get_data_mut().event.take();
            match event {
                Some(Event::Int3(at)) if at == self.sentinel => {
                    let esp = self.reg(RegisterX86::ESP)?;
                    if self.outer_level(esp, expect) {
                        self.start_unwinding(esp)?;
                        return Err(Error::Stopped);
                    }
                    return Ok(());
                }
                Some(Event::Int3(at)) if at == self.sentinel + seh::STUB_RETURN => {
                    pc = self.handler_returned()?;
                }
                Some(Event::Int3(at)) => {
                    let next = if let Some(&i) = self.shared.stubs.get(&at) {
                        self.enter_import(i, at)
                    } else if self.shared.table.get(at).is_some() {
                        self.enter_port(at)
                    } else if let Some(t) = self.shared.trap(at) {
                        self.enter_trap(t, at)
                    } else {
                        // a stray int3: EXCEPTION_BREAKPOINT at it
                        self.uc.reg_write(RegisterX86::EIP, u64::from(at))?;
                        self.raise_fault(seh::BREAKPOINT, at, &[0])
                    };
                    pc = match next {
                        Ok(pc) => pc,
                        Err(Error::Stopped) if self.unwound_to(expect)? => return Ok(()),
                        Err(e) => return Err(e),
                    };
                }
                Some(Event::Interrupt(n, at)) => {
                    let code = match n {
                        0 => seh::DIVIDE_BY_ZERO,
                        6 => seh::ILLEGAL_INSTRUCTION,
                        _ => return Err(Error::Interrupt(n, at)),
                    };
                    pc = self.raise_fault(code, at, &[])?;
                }
                Some(Event::BadAccess(at, write)) => {
                    let eip = self.reg(RegisterX86::EIP)?;
                    let kind = if write {
                        1
                    } else if at == eip {
                        8
                    } else {
                        0
                    };
                    pc = self.raise_fault(seh::ACCESS_VIOLATION, eip, &[kind, at])?;
                }
                None => {
                    r?;
                    let eip = self.reg(RegisterX86::EIP)?;
                    if self.shared.overdue() {
                        // the watchdog stopped it: the loop's check ends it
                        pc = eip;
                        continue;
                    }
                    return Err(Error::Ended(eip));
                }
            }
        }
    }

    /// Maps into this thread's emulator every arena region it hasn't seen
    /// (another thread's `VirtualAlloc`, stack or DLL).
    pub(crate) fn sync_regions(&mut self) -> Result<()> {
        crate::library::sync_hooks(self)?;
        let g = self.shared.arena.generation();
        if g == self.generation {
            return Ok(());
        }
        for r in self.shared.arena.regions() {
            if self.mapped.contains(&r.start) {
                continue;
            }
            // SAFETY: the region is committed host memory owned by the
            // arena, which outlives the emulator (the `Shared` this `Exec`
            // holds keeps the arena; the emulator is dropped first).
            unsafe {
                self.uc.mem_map_ptr(
                    u64::from(r.start),
                    u64::from(r.size),
                    Prot::ALL,
                    self.shared.arena.ptr(r.start).cast(),
                )?;
            }
            self.mapped.insert(r.start);
        }
        self.generation = g;
        Ok(())
    }

    /// The caller's arguments at an entry from Unicorn: `esp` at the
    /// return address.
    fn entry_args(&mut self, words: usize) -> Result<Args> {
        let esp = self.reg(RegisterX86::ESP)?;
        let mem = self.mem();
        let stack = (0..u32::try_from(words).unwrap_or(0))
            .map(|i| ffx_bridge::Mem::read::<u32>(&mem, esp + 4 + 4 * i))
            .collect();
        Ok(Args {
            ecx: self.reg(RegisterX86::ECX)?,
            edx: self.reg(RegisterX86::EDX)?,
            stack,
            slots: esp + 4,
        })
    }

    /// Runs the port at `at` for Unicorn code that called it; returns where
    /// to resume (a return stub).
    fn enter_port(&mut self, at: u32) -> Result<u32> {
        let entry = *self.shared.table.get(at).ok_or(Error::Interrupt(3, at))?;
        *self.stats.ports.entry(at).or_default() += 1;
        let args = self.entry_args(entry.words)?;
        let saved = self.save_regs()?;
        self.spot.enter(spot::Kind::Port, at, args.slots - 4);
        let ret = self.run_port(entry, &args);
        self.spot.leave();
        self.restore_regs(&saved)?;
        if self.halted() {
            return Err(Error::Stopped);
        }
        let pops = if entry.conv.callee_pops() {
            entry.words
        } else {
            0
        };
        self.finish(ret, pops)
    }

    /// Runs a port: a routed one (`ffx-host-ports`) with this executor as
    /// the thread's `ffx_bridge::route` router, so its calls into the game
    /// and its memory come here. Frames it left on the game stack are
    /// released.
    ///
    /// A routed port can't see a stop: its calls into the game just return
    /// 0, and one that polls (a wait loop around a call) would spin
    /// forever. So once the thread is halted the router unwinds the port
    /// ([`Halted`]), which ends here; the caller then sees the stop as for
    /// any other call.
    fn run_port(&mut self, entry: Entry, args: &Args) -> Ret {
        let watched = watched(entry.name);
        if watched {
            self.watch_note(entry.name, "enter");
        }
        let ret = self.run_port_inner(entry, args);
        if watched {
            self.watch_note(entry.name, "leave");
        }
        ret
    }

    /// A `FFX_HOST_WATCH` note: the port, the thread, the clock's frame
    /// and the host time since the first note.
    fn watch_note(&self, name: &str, what: &str) {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        static NOTES: AtomicU32 = AtomicU32::new(0);
        if NOTES.fetch_add(1, Ordering::Relaxed) >= WATCH_NOTES {
            return;
        }
        let t = START.get_or_init(Instant::now).elapsed();
        crate::log::note(&format!(
            "watch {:.3} ms thread {:#x} frame {} {what} {name}",
            t.as_secs_f64() * 1000.0,
            self.id,
            self.shared.clock.frame()
        ));
    }

    fn run_port_inner(&mut self, entry: Entry, args: &Args) -> Ret {
        let frames = self.frames.len();
        let esp = self.uc.reg_read(RegisterX86::ESP).ok();
        let ret = match entry.port {
            Port::Exec(f) => f(self, args),
            Port::Routed(f) => {
                let me: &mut dyn ffx_bridge::route::Router = self;
                // SAFETY: the executor outlives the guard, and while it is
                // set the port reaches the executor only through it.
                let router = unsafe { ffx_bridge::route::enter(std::ptr::NonNull::from(me)) };
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(args)));
                drop(router);
                match r {
                    Ok(ret) => ret,
                    Err(p) if p.is::<Halted>() => Ret::VOID,
                    Err(p) => {
                        let why = p
                            .downcast_ref::<&str>()
                            .map(ToString::to_string)
                            .or_else(|| p.downcast_ref::<String>().cloned())
                            .unwrap_or_default();
                        let at = self.reg(RegisterX86::EIP).unwrap_or(0);
                        self.end(Stop::Failed(
                            format!("port {} panicked: {why}", entry.name),
                            at,
                        ));
                        Ret::VOID
                    }
                }
            }
        };
        if self.frames.len() > frames {
            self.frames.truncate(frames);
            if let Some(esp) = esp {
                let _ = self.uc.reg_write(RegisterX86::ESP, esp);
            }
        }
        ret
    }

    /// [`Exec::run_port`] for a call from Rust (`args.slots` 0): the words
    /// are written to the game stack below the current frame first, as a
    /// caller in the game would have pushed them, so a shim that reads its
    /// caller's argument slots (a varargs `printf` reads its `va_list`
    /// there) finds them in the arena.
    fn run_port_from_rust(&mut self, entry: Entry, args: &Args) -> Result<Ret> {
        self.on_game_stack(args, |e, a| e.run_port(entry, a))
    }

    /// Runs `run` with `args`' words on the game stack below the current
    /// frame, a return-address word under them, and `slots` pointing at
    /// them, as a call from the game would leave them; `esp` is restored
    /// after. For callees that read their words through `slots` when Rust
    /// calls them (a port's varargs, `wsprintfW`'s).
    pub(crate) fn on_game_stack<R>(
        &mut self,
        args: &Args,
        run: impl FnOnce(&mut Self, &Args) -> R,
    ) -> Result<R> {
        let esp = self.reg(RegisterX86::ESP)?;
        let words = u32::try_from(args.stack.len()).unwrap_or(u32::MAX);
        let top = (esp - 0x40) & !0xf;
        let Some(base) = top.checked_sub(4 * words) else {
            return Err(Error::StackOverflow(0));
        };
        if base < self.stack.0 + 0x1000 {
            return Err(Error::StackOverflow(base));
        }
        let mut mem = self.mem();
        for (i, &w) in (0u32..).zip(&args.stack) {
            ffx_bridge::Mem::write(&mut mem, base + 4 * i, w);
        }
        // the return address's word, as for a call from the game
        ffx_bridge::Mem::write(&mut mem, base - 4, self.sentinel);
        self.uc.reg_write(RegisterX86::ESP, u64::from(base - 4))?;
        let mut on_stack = args.clone();
        on_stack.slots = base;
        let ret = run(self, &on_stack);
        self.uc.reg_write(RegisterX86::ESP, u64::from(esp))?;
        Ok(ret)
    }

    /// Pops `ST0` off Unicorn's x87 stack (what the caller's `fstp` does
    /// with a float result).
    pub(crate) fn pop_st0(&mut self) -> Result<()> {
        let sw = self.uc.reg_read(RegisterX86::FPSW)?;
        let top = (sw >> 11) & 7;
        let tag = self.uc.reg_read(RegisterX86::FPTAG)?;
        self.uc
            .reg_write(RegisterX86::FPTAG, tag | (3 << (2 * top)))?;
        let sw = (sw & !0x3800) | (((top + 1) & 7) << 11);
        self.uc.reg_write(RegisterX86::FPSW, sw)?;
        Ok(())
    }

    /// Runs trap `t` for Unicorn code that called it.
    fn enter_trap(&mut self, t: Trap, at: u32) -> Result<u32> {
        let args = self.entry_args(t.words)?;
        let saved = self.save_regs()?;
        self.trap_exit = None;
        self.spot.enter(spot::Kind::Trap, at, args.slots - 4);
        let ret = (t.run)(self, t.tag, &args);
        self.spot.leave();
        let exit = self.trap_exit.take();
        self.restore_regs(&saved)?;
        if self.halted() {
            return Err(Error::Stopped);
        }
        let pops = match exit {
            Some(TrapExit {
                jump: Some(target), ..
            }) => {
                // `jmp target` with the caller's stack as at the entry
                self.finish(ret, 0)?;
                return Ok(target);
            }
            Some(x) => (x.pops / 4) as usize,
            None if t.conv.callee_pops() => t.words,
            None => 0,
        };
        self.finish(ret, pops)
    }

    /// Counts the basic blocks this thread's emulator starts at the
    /// addresses `counts` holds, inside `begin..end` (an overlay's code:
    /// its entries that run in Unicorn, [`crate::library`]).
    pub(crate) fn add_entry_counter(
        &mut self,
        begin: u32,
        end: u32,
        counts: Arc<HashMap<u32, AtomicU64>>,
    ) -> Result<()> {
        self.uc
            .add_block_hook(u64::from(begin), u64::from(end) - 1, move |_, addr, _| {
                #[allow(clippy::cast_possible_truncation)] // a 32-bit address
                if let Some(c) = counts.get(&(addr as u32)) {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })?;
        Ok(())
    }

    /// Whether `addr` is an import's stub (its Rust side or DLL runs).
    pub(crate) fn is_import_stub(&self, addr: u32) -> bool {
        self.shared.stubs.contains_key(&addr)
    }

    /// `esp` of the emulator now.
    pub(crate) fn esp(&self) -> Result<u32> {
        self.reg(RegisterX86::ESP)
    }

    /// Sets the emulator's `esp`.
    pub(crate) fn set_esp(&mut self, esp: u32) -> Result<()> {
        self.uc.reg_write(RegisterX86::ESP, u64::from(esp))?;
        Ok(())
    }

    /// The bottom of this thread's stack (a frame must stay above it, with
    /// a page to spare).
    pub(crate) fn stack_floor(&self) -> u32 {
        self.stack.0 + 0x1000
    }

    /// Sets the low 64 bits of `xmm0` (an overlay port's SSE result).
    pub(crate) fn set_xmm0(&mut self, low: u64) -> Result<()> {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&low.to_le_bytes());
        self.uc.reg_write_long(RegisterX86::XMM0, &b)?;
        Ok(())
    }

    /// Calls `addr` (an SSE maths function: `MSVCR110`'s
    /// `_libm_sse2_*_precise`, `xmm0` in and out) with `x` in `xmm0`;
    /// its `xmm0` result. NaN when the call fails or nests too deep.
    pub(crate) fn call_sse(&mut self, addr: u32, x: f64) -> f64 {
        use ffx_bridge::Mem as _;
        if self.x87_depth >= X87_LEVELS {
            return f64::NAN;
        }
        // movsd xmm0, [cell]; mov eax, addr; call eax; movsd [cell], xmm0; ret
        let at = self.sentinel + X87_STUBS + X87_STUB * self.x87_depth;
        let cell = at + 0x40;
        let mut mem = self.mem();
        mem.write(cell, x);
        let mut code = vec![0xf2, 0x0f, 0x10, 0x05];
        code.extend_from_slice(&cell.to_le_bytes());
        code.push(0xb8);
        code.extend_from_slice(&addr.to_le_bytes());
        code.extend_from_slice(&[0xff, 0xd0, 0xf2, 0x0f, 0x11, 0x05]);
        code.extend_from_slice(&cell.to_le_bytes());
        code.push(0xc3);
        self.write_code(at, &code);
        self.x87_depth += 1;
        let r = Exec::call(self, at, Conv::Cdecl, &Args::default());
        self.x87_depth -= 1;
        if r.is_err() {
            return f64::NAN;
        }
        mem.read::<f64>(cell)
    }

    /// Runs import `i` (its stub at `stub`) for Unicorn code that called
    /// it.
    fn enter_import(&mut self, i: usize, stub: u32) -> Result<u32> {
        let words = self.shared.imports.bound(i).map_or(0, |b| b.words);
        let args = self.entry_args(words)?;
        let saved = self.save_regs()?;
        self.spot.enter(
            spot::Kind::Import,
            u32::try_from(i).unwrap_or(u32::MAX),
            args.slots - 4,
        );
        let ret = self.run_import(i, &args);
        self.spot.leave();
        self.restore_regs(&saved)?;
        let ret = ret?;
        if self.seh.pending.is_some() {
            // RaiseException: the dispatch goes on from here
            return self.raise_pending(stub);
        }
        let pops = match self.shared.imports.bound(i) {
            Some(b) if b.conv.callee_pops() => b.words,
            _ => 0,
        };
        self.finish(ret, pops)
    }

    fn run_import(&mut self, i: usize, args: &Args) -> Result<Ret> {
        let Some(b) = self.shared.imports.bound(i).cloned() else {
            return Err(Error::Interrupt(3, 0));
        };
        *self.stats.imports.entry(b.name.clone()).or_default() += 1;
        let stop = self
            .shared
            .stop_at
            .read()
            .is_ok_and(|s| s.contains(&b.name));
        if stop {
            self.end(Stop::Milestone(b.name.clone()));
            return Err(Error::Stopped);
        }
        match b.run {
            Some(f) => {
                let ret = f(self, args);
                if self.halted() {
                    return Err(Error::Stopped);
                }
                Ok(ret)
            }
            None => {
                let shown = match &b.from {
                    Some(d) => format!("{} (from {d})", b.name),
                    None => b.name.clone(),
                };
                if !self.stats.missing_imports.contains(&shown) {
                    self.stats.missing_imports.push(shown.clone());
                }
                // lenient: a C runtime function (cdecl, so nothing to pop)
                // returns 0 and the run goes on, to see further
                let cdecl = b.name.starts_with("MSVCR110.dll!")
                    && !b.name.contains('?')
                    && !b.name.contains("_CI");
                if self.shared.lenient.load(Ordering::Relaxed) && cdecl {
                    return Ok(Ret::eax(0));
                }
                let caller = if args.slots == 0 {
                    0
                } else {
                    self.mem().read::<u32>(args.slots - 4)
                };
                Err(Error::UnstubbedImport(format!(
                    "{shown} (called from {caller:#010x})"
                )))
            }
        }
    }

    /// Applies a port's or import's result and returns the stub that pops
    /// `pops` words (and loads `ST0` for a float).
    fn finish(&mut self, ret: Ret, pops: usize) -> Result<u32> {
        if ret.sets_eax {
            self.uc.reg_write(RegisterX86::EAX, u64::from(ret.eax))?;
        }
        if let Some(edx) = ret.edx {
            self.uc.reg_write(RegisterX86::EDX, u64::from(edx))?;
        }
        let n = u32::try_from(pops).unwrap_or(u32::MAX).min(MAX_RET_WORDS);
        Ok(match ret.st0 {
            Some(v) => {
                let mut mem = self.mem();
                ffx_bridge::Mem::write(&mut mem, self.float_cell, v);
                self.ret_float_stubs + 12 * n
            }
            None => self.ret_stubs + 4 * n,
        })
    }

    pub(crate) fn reg(&self, r: RegisterX86) -> Result<u32> {
        #[allow(clippy::cast_possible_truncation)] // 32-bit registers
        Ok(self.uc.reg_read(r)? as u32)
    }

    fn save_regs(&self) -> Result<[u64; 8]> {
        let mut out = [0; 8];
        for (o, r) in out.iter_mut().zip(SAVED_REGS) {
            *o = self.uc.reg_read(r)?;
        }
        Ok(out)
    }

    fn restore_regs(&mut self, saved: &[u64; 8]) -> Result<()> {
        for (&v, r) in saved.iter().zip(SAVED_REGS) {
            self.uc.reg_write(r, v)?;
        }
        Ok(())
    }

    /// `ST0` as a double, if the x87 stack holds anything.
    fn st0(&self) -> Result<Option<f64>> {
        let tag = self.uc.reg_read(RegisterX86::FPTAG)?;
        let sw = self.uc.reg_read(RegisterX86::FPSW)?;
        let top = (sw >> 11) & 7;
        if (tag >> (2 * top)) & 3 == 3 {
            return Ok(None);
        }
        let raw = self.uc.reg_read_long(RegisterX86::ST0)?;
        Ok(Some(f80_to_f64(&raw[..10])))
    }

    /// The sentinel, the `ret n` and `fld; ret n` stubs (this thread's own:
    /// the float cell is per thread).
    fn write_stubs(&mut self, at: u32) {
        let arena = Arc::clone(&self.shared.arena);
        arena.fill(at, &[INT3]);
        for n in 0..=MAX_RET_WORDS {
            let mut code = [0x90u8; 4];
            if n == 0 {
                code[0] = 0xc3;
            } else {
                code[0] = 0xc2;
                #[allow(clippy::cast_possible_truncation)] // at most 256
                code[1..3].copy_from_slice(&((4 * n) as u16).to_le_bytes());
            }
            arena.fill(self.ret_stubs + 4 * n, &code);
            // fld qword ptr [cell]; ret n
            let mut f = [0x90u8; 12];
            f[0] = 0xdd;
            f[1] = 0x05;
            f[2..6].copy_from_slice(&self.float_cell.to_le_bytes());
            f[6..10].copy_from_slice(&code);
            arena.fill(self.ret_float_stubs + 12 * n, &f);
        }
        self.write_seh_stubs();
    }

    /// The thread's TEB at `at + PAGE` (fs), a GDT at `at`, an empty PEB
    /// at `at + 2 * PAGE`: what MSVC's start-up code and SEH frames read
    /// (`fs:[0]`, the stack limits, `fs:[0x18]`, `fs:[0x30]`), and the TLS
    /// slots at `TEB + 0xe10`.
    fn thread_block(&mut self, at: u32) -> Result<()> {
        let teb = at + PAGE;
        let peb = at + 2 * PAGE;
        let mut mem = self.mem();
        let w = |m: &mut ArenaMem, a: u32, v: u32| ffx_bridge::Mem::write(m, a, v);
        w(&mut mem, teb, u32::MAX);
        w(&mut mem, teb + 4, self.stack.1);
        w(&mut mem, teb + 8, self.stack.0);
        w(&mut mem, teb + 0x18, teb);
        w(&mut mem, teb + 0x20, 0x1000); // ClientId: process
        w(&mut mem, teb + 0x24, self.id);
        w(&mut mem, teb + 0x30, peb);
        let descriptor = |base: u64, limit: u64, access: u64, flags: u64| -> u64 {
            (limit & 0xffff)
                | ((base & 0xff_ffff) << 16)
                | (access << 40)
                | (((limit >> 16) & 0xf) << 48)
                | (flags << 52)
                | ((base >> 24) << 56)
        };
        let table = [
            0,
            descriptor(u64::from(teb), 0xfff, 0xf3, 0x4),
            descriptor(0, 0xf_ffff, 0x9b, 0xc),
            descriptor(0, 0xf_ffff, 0x93, 0xc),
        ];
        for (i, d) in (0u32..).zip(table) {
            ffx_bridge::Mem::write(&mut mem, at + 8 * i, d);
        }
        let mut gdtr = [0u8; 24];
        gdtr[8..16].copy_from_slice(&u64::from(at).to_le_bytes());
        gdtr[16..20].copy_from_slice(&0x1fu32.to_le_bytes());
        self.uc.reg_write_long(RegisterX86::GDTR, &gdtr)?;
        let cr0 = self.uc.reg_read(RegisterX86::CR0)?;
        self.uc.reg_write(RegisterX86::CR0, cr0 | 1)?;
        let cr4 = self.uc.reg_read(RegisterX86::CR4)?;
        self.uc.reg_write(RegisterX86::CR4, cr4 | 0x600)?;
        self.uc.reg_write(RegisterX86::MXCSR, 0x1f80)?;
        self.uc.reg_write(RegisterX86::CS, 0x10)?;
        for seg in [RegisterX86::SS, RegisterX86::DS, RegisterX86::ES] {
            self.uc.reg_write(seg, 0x18)?;
        }
        self.uc.reg_write(RegisterX86::FS, 0x0b)?;
        Ok(())
    }

    fn add_hooks(&mut self) -> Result<()> {
        let x87 = Arc::clone(&self.shared.x87);
        self.uc.add_intr_hook(move |uc, intno| {
            #[allow(clippy::cast_possible_truncation)] // 32-bit eip
            let eip = uc.reg_read(RegisterX86::EIP).unwrap_or(0) as u32;
            // an x87 instruction `ffx_fpu` computes: done here, the
            // emulator goes on after it
            if intno == 3
                && let Some(&op) = x87.get(&eip.wrapping_sub(1))
                && ffx_fpu::unicorn::step(uc, u64::from(eip) - 1, op).is_ok()
            {
                uc.get_data_mut().x87 += 1;
                return;
            }
            let ev = if intno == 3 {
                Event::Int3(eip.wrapping_sub(1))
            } else {
                Event::Interrupt(intno, eip)
            };
            uc.get_data_mut().event = Some(ev);
            let _ = uc.emu_stop();
        })?;
        self.uc.add_mem_hook(
            HookType::MEM_INVALID,
            0,
            u64::MAX,
            |uc, kind: MemType, addr, _, _| {
                let write = matches!(kind, MemType::WRITE_UNMAPPED | MemType::WRITE_PROT);
                #[allow(clippy::cast_possible_truncation)] // 32-bit address
                let ev = Event::BadAccess(addr as u32, write);
                uc.get_data_mut().event = Some(ev);
                false
            },
        )?;
        for &(at, op) in &self.shared.x87_watch {
            let at = u64::from(at);
            self.uc.add_code_hook(at, at, move |uc, addr, _| {
                if ffx_fpu::unicorn::step(uc, addr, op).is_ok() {
                    uc.get_data_mut().x87 += 1;
                }
            })?;
        }
        Ok(())
    }

    /// x87 instructions computed by `ffx_fpu` on this thread so far.
    #[must_use]
    pub fn x87_steps(&self) -> u64 {
        self.uc.get_data().x87
    }

    /// Maps a region the game asked for at run time (`VirtualAlloc`) into
    /// the arena; every thread's emulator maps it before it next runs.
    ///
    /// # Errors
    /// When the arena or Unicorn refuse it.
    pub fn map_region(&mut self, size: u32, name: &str) -> Result<u32> {
        let at = self.shared.arena.map_anywhere(REGION_HINT, size, name)?;
        self.sync_regions()?;
        Ok(at)
    }

    /// A heap block holding `code`, runnable by Unicorn (tests and
    /// trampolines).
    ///
    /// # Errors
    /// When the heap is full.
    pub fn alloc_code(&mut self, code: &[u8]) -> Result<u32> {
        let len = u32::try_from(code.len()).unwrap_or(u32::MAX);
        let at = self.alloc(len);
        if at == 0 {
            return Err(Error::Map(0, len, "heap full".into()));
        }
        self.write_code(at, code);
        Ok(at)
    }

    /// Writes `code` at `at` and drops Unicorn's translations of it.
    pub fn write_code(&mut self, at: u32, code: &[u8]) {
        let mut mem = self.mem();
        mem.write_bytes(at, code);
        let end = u64::from(at) + code.len() as u64;
        let _ = self.uc.ctl_remove_cache(u64::from(at), end);
    }

    /// The import table (for the report).
    #[must_use]
    pub fn imports(&self) -> &Imports {
        &self.shared.imports
    }

    /// Runs an import's Rust side directly (tests).
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn call_import(&mut self, f: ImportFn, args: &Args) -> Result<Ret> {
        Ok(f(self, args))
    }

    /// Starts game thread `entry(param)` on its own host thread
    /// (`CreateThread`, `_beginthreadex` through the DLL, the thread
    /// pool); with `suspended` it waits for [`Exec::resume_thread`].
    /// Returns the handle and the id.
    pub fn spawn_thread(
        &mut self,
        entry: u32,
        param: u32,
        suspended: bool,
        via: &'static str,
    ) -> (u32, u32) {
        let shared = Arc::clone(&self.shared);
        let handle = shared.handle();
        let id = shared.thread_ids.fetch_add(4, Ordering::Relaxed) + 4;
        shared.sync.insert(
            handle,
            crate::sync::Obj::Thread {
                id,
                code: crate::sync::STILL_ACTIVE,
                done: false,
            },
        );
        if let Ok(mut t) = shared.threads.lock() {
            t.push(ThreadInfo {
                id,
                handle,
                entry,
                param,
                via,
                started: false,
                outcome: None,
                at: shared.started.elapsed(),
                stats: None,
                trace: Vec::new(),
            });
        }
        if !suspended {
            start_thread(&shared, handle);
        }
        (handle, id)
    }

    /// `ResumeThread`: starts a thread created suspended; the previous
    /// suspend count (1 if it was waiting, else 0).
    pub fn resume_thread(&mut self, handle: u32) -> u32 {
        let waiting = self.shared.threads.lock().is_ok_and(|t| {
            t.iter()
                .any(|i| i.handle == handle && !i.started && i.outcome.is_none())
        });
        if waiting {
            start_thread(&self.shared, handle);
            1
        } else {
            0
        }
    }

    /// The thread pool's `SubmitThreadpoolWork(work)`: runs the work's
    /// callback `(instance, context, work)` on a thread of its own.
    pub fn submit_work(&mut self, work: u32) {
        let item = self
            .shared
            .pool_work
            .lock()
            .ok()
            .and_then(|w| w.get(&work).copied());
        if let Some((callback, context)) = item {
            // a trampoline: callback(0, context, work); ret 4
            let code = pool_trampoline(callback, context, work);
            if let Ok(at) = self.alloc_code(&code) {
                self.spawn_thread(at, 0, false, "SubmitThreadpoolWork");
            }
        }
    }
}

/// `push work; push context; push 0; call callback; ret 4`: a thread
/// routine (stdcall, one parameter) that runs a thread-pool callback.
fn pool_trampoline(callback: u32, context: u32, work: u32) -> Vec<u8> {
    let mut c = Vec::new();
    for v in [work, context, 0] {
        c.push(0x68);
        c.extend_from_slice(&v.to_le_bytes());
    }
    // mov eax, callback; call eax; ret 4
    c.push(0xb8);
    c.extend_from_slice(&callback.to_le_bytes());
    c.extend_from_slice(&[0xff, 0xd0, 0xc2, 0x04, 0x00]);
    c
}

/// Runs the thread behind `handle` on a new host thread.
fn start_thread(shared: &Arc<Shared>, handle: u32) {
    let Some((id, entry, param)) = shared.threads.lock().ok().and_then(|mut t| {
        let i = t.iter_mut().find(|i| i.handle == handle)?;
        i.started = true;
        i.at = shared.started.elapsed();
        Some((i.id, i.entry, i.param))
    }) else {
        return;
    };
    let shared2 = Arc::clone(shared);
    shared.running.fetch_add(1, Ordering::AcqRel);
    let spawned = std::thread::Builder::new()
        .name(format!("game thread {id:#x}"))
        .stack_size(64 << 20)
        .spawn(move || thread_main(&shared2, handle, id, entry, param));
    if spawned.is_err() {
        shared.running.fetch_sub(1, Ordering::AcqRel);
        finish_thread(shared, handle, "host thread refused".into(), None, None);
        shared.sync.thread_done(handle, 0);
    }
}

fn thread_main(shared: &Arc<Shared>, handle: u32, id: u32, entry: u32, param: u32) {
    let mut ex = match Exec::on_thread(Arc::clone(shared), id, THREAD_STACK_SIZE) {
        Ok(e) => e,
        Err(e) => {
            finish_thread(shared, handle, format!("no executor: {e}"), None, None);
            shared.sync.thread_done(handle, 0);
            shared.running.fetch_sub(1, Ordering::AcqRel);
            return;
        }
    };
    let _ = ex.attach_dlls(2);
    let r = ex.call_stdcall(entry, &[param]);
    let (code, outcome) = match (r, ex.stopped()) {
        (Ok(code), _) => (code, format!("returned {code:#x}")),
        (Err(_), Some(Stop::ThreadExit(code))) => (*code, format!("ExitThread({code:#x})")),
        (Err(_), Some(s)) => (0, s.to_string()),
        (Err(e), None) => (0, e.to_string()),
    };
    ex.stats.instructions = ex.instructions();
    ex.stats.x87 = ex.x87_steps();
    ex.stats.exceptions = ex.exceptions_raised();
    let stats = ex.stats.clone();
    let profile = ex.profile().clone();
    let trace = std::mem::take(&mut ex.trace);
    drop(ex);
    // the record first: a waiter on the handle reads it as soon as the
    // handle is signalled
    if let Ok(mut t) = shared.threads.lock()
        && let Some(i) = t.iter_mut().find(|i| i.handle == handle)
    {
        i.trace = trace;
    }
    finish_thread(shared, handle, outcome, Some(stats), Some(profile));
    shared.sync.thread_done(handle, code);
    shared.running.fetch_sub(1, Ordering::AcqRel);
}

fn finish_thread(
    shared: &Shared,
    handle: u32,
    outcome: String,
    stats: Option<Stats>,
    profile: Option<Profile>,
) {
    if let Some(p) = profile
        && let Ok(mut f) = shared.finished.lock()
    {
        f.merge(&p);
    }
    if let Ok(mut t) = shared.threads.lock()
        && let Some(i) = t.iter_mut().find(|i| i.handle == handle)
    {
        i.outcome = Some(outcome);
        i.stats = stats;
    }
}

/// The registers a port's caller keeps across the call (the hook's thunk
/// keeps `eax`, `ecx` and `edx` too, unless the result goes there).
const SAVED_REGS: [RegisterX86; 8] = [
    RegisterX86::EAX,
    RegisterX86::ECX,
    RegisterX86::EDX,
    RegisterX86::EBX,
    RegisterX86::ESI,
    RegisterX86::EDI,
    RegisterX86::EBP,
    RegisterX86::ESP,
];

/// An 80-bit x87 value as a double (rounded to nearest even; the game runs
/// at 53 bits, so its values are exact).
#[must_use]
pub fn f80_to_f64(b: &[u8]) -> f64 {
    let mant = u64::from_le_bytes(b[..8].try_into().unwrap_or([0; 8]));
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from(se & 0x7fff);
    if exp == 0x7fff {
        return if mant << 1 == 0 {
            sign * f64::INFINITY
        } else {
            f64::NAN
        };
    }
    if mant == 0 {
        return sign * 0.0;
    }
    // mant * 2^(exp - 16383 - 63), in two steps to stay in range
    #[allow(clippy::cast_precision_loss)] // rounds to 53 bits, as fst does
    let m = mant as f64;
    sign * m * 2f64.powi((exp - 16383 - 63).clamp(-1100, 1100))
}

/// Where [`ffx_bridge::route::Router::call_x87`] writes its stubs in the
/// thread's stub page (after the `fld; ret n` stubs, before the float cell
/// at 0xf00 and the SEH stubs), one per nesting level.
const X87_STUBS: u32 = 0x480;
/// Bytes per x87 stub.
const X87_STUB: u32 = 0x80;
/// Nesting levels of x87 stubs.
const X87_LEVELS: u32 = 8;

/// How many `FFX_HOST_WATCH` notes a run keeps.
const WATCH_NOTES: u32 = 20_000;

/// Whether `FFX_HOST_WATCH` (a comma list of `ffx.sym` names) names this
/// port: its entries and returns are noted with the thread, the frame and
/// the time ([`Exec::watch_note`]), to see which threads run what in
/// which order (the cloth jobs against the cloth copies, docs/host.md).
fn watched(name: &str) -> bool {
    static WATCH: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let w = WATCH.get_or_init(|| {
        std::env::var("FFX_HOST_WATCH")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    });
    !w.is_empty() && w.iter().any(|n| n == name)
}

/// The payload [`Exec::run_port`] catches: the thread was halted while a
/// routed port ran.
pub(crate) struct Halted;

/// Unwinds the routed port that called into a halted thread
/// ([`Exec::run_port`]).
pub(crate) fn halted() -> ! {
    std::panic::resume_unwind(Box::new(Halted))
}

/// The executor serves the ported shims' calls and memory
/// (`ffx_bridge::route`): calls through [`Exec::call`] (the table, the
/// imports, Unicorn), memory from the arena, scratch and frames inside it.
impl ffx_bridge::route::Router for Exec {
    fn call(
        &mut self,
        addr: u32,
        conv: Conv,
        args: &Args,
        float: ffx_bridge::route::Float,
    ) -> ffx_bridge::route::Raw {
        use ffx_bridge::route::{Float, Raw};
        let emulated = self.shared.table.get(addr).is_none()
            && !self.shared.stubs.contains_key(&addr)
            && self.shared.trap(addr).is_none();
        let Ok(ret) = Exec::call(self, addr, conv, args) else {
            halted()
        };
        let st0 = match (float, ret.st0) {
            (Float::None, _) => 0.0,
            (_, None) => f64::NAN,
            (Float::F64, Some(v)) => v,
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            (Float::F32, Some(v)) => f64::from(v as f32),
        };
        if emulated && float != Float::None && ret.st0.is_some() {
            // the original left it on Unicorn's x87 stack; the caller's
            // fstp pops it
            let _ = self.pop_st0();
        }
        Raw {
            eax: ret.eax,
            edx: ret.edx.unwrap_or(0),
            st0,
        }
    }

    fn call_x87(&mut self, addr: u32, st: &[f64], float: ffx_bridge::route::Float) -> f64 {
        use ffx_bridge::Mem as _;
        use ffx_bridge::route::Float;
        if self.x87_depth >= X87_LEVELS || st.len() > 7 {
            return f64::NAN;
        }
        // fld qword [cell] for each operand; mov eax, addr; call eax;
        // fstp dword/qword [out]; ret
        let at = self.sentinel + X87_STUBS + X87_STUB * self.x87_depth;
        let cells = at + 0x40;
        let out = cells + 0x38;
        let mut code = Vec::new();
        let mut mem = self.mem();
        for (i, &v) in (0u32..).zip(st) {
            mem.write(cells + 8 * i, v);
            code.extend_from_slice(&[0xdd, 0x05]);
            code.extend_from_slice(&(cells + 8 * i).to_le_bytes());
        }
        code.push(0xb8);
        code.extend_from_slice(&addr.to_le_bytes());
        code.extend_from_slice(&[0xff, 0xd0]);
        match float {
            Float::F32 => code.extend_from_slice(&[0xd9, 0x1d]),
            Float::F64 | Float::None => code.extend_from_slice(&[0xdd, 0x1d]),
        }
        code.extend_from_slice(&out.to_le_bytes());
        code.push(0xc3);
        self.write_code(at, &code);
        self.x87_depth += 1;
        let r = Exec::call(self, at, Conv::Cdecl, &Args::default());
        self.x87_depth -= 1;
        if r.is_err() {
            halted();
        }
        match float {
            Float::F32 => f64::from(mem.read::<f32>(out)),
            Float::F64 | Float::None => mem.read::<f64>(out),
        }
    }

    fn read_bytes(&mut self, addr: u32, out: &mut [u8]) {
        ffx_bridge::Mem::read_bytes(&self.mem(), addr, out);
    }

    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        ffx_bridge::Mem::write_bytes(&mut self.mem(), addr, data);
    }

    fn scratch(&mut self, slot: ffx_bridge::route::Scratch) -> u32 {
        let i = slot as usize;
        if self.scratch[i] == 0 {
            let at = self.alloc(ffx_bridge::SCRATCH_LEN);
            if at != 0 {
                let mut mem = self.mem();
                ffx_bridge::Mem::write_bytes(&mut mem, at, &[0; ffx_bridge::SCRATCH_LEN as usize]);
            }
            self.scratch[i] = at;
        }
        self.scratch[i]
    }

    fn frame_alloc(&mut self, bytes: u32) -> u32 {
        let Ok(esp) = self.reg(RegisterX86::ESP) else {
            return 0;
        };
        let at = esp.wrapping_sub(bytes) & !0xf;
        if at < self.stack.0 + 0x1000 || at > esp {
            return 0;
        }
        let mut mem = self.mem();
        ffx_bridge::Mem::write_bytes(&mut mem, at, &vec![0; (esp - at) as usize]);
        if self.uc.reg_write(RegisterX86::ESP, u64::from(at)).is_err() {
            return 0;
        }
        self.frames.push((at, esp));
        at
    }

    fn frame_free(&mut self, at: u32) {
        if at == 0 {
            return;
        }
        while let Some((top, esp)) = self.frames.pop() {
            let _ = self.uc.reg_write(RegisterX86::ESP, u64::from(esp));
            if top == at {
                break;
            }
        }
    }
}
