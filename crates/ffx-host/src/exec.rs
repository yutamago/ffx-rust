//! The mixed executor (docs/host.md, "Rust and Unicorn"): Rust ports from
//! the function table, everything else from `FFX.exe`'s `.text` in
//! Unicorn, on one arena.
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
//! Development only: the release runs no `FFX.exe` code (rewrite-plan, M5).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ffx_bridge::Mem as _;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

use crate::arena::{Arena, ArenaMem, PAGE};
use crate::image::Image;
use crate::imports::{ImportFn, Imports};
use crate::table::{Args, Conv, Ret, Table};
use crate::{Error, Result};

/// `int3`.
const INT3: u8 = 0xcc;
/// The x87 control word Windows starts threads with (53-bit precision, all
/// exceptions masked), as the game thread runs (docs/porting.md).
pub const FPU_CONTROL: u16 = 0x027f;
/// The main thread's stack.
const STACK_SIZE: u32 = 0x0020_0000;
/// The stub page: import stubs, return stubs, the sentinel.
const STUB_SIZE: u32 = 0x0001_0000;
/// Where the executor's own regions start looking for room (above the
/// image).
const REGION_HINT: u32 = 0x0300_0000;
/// The `ret n` stubs: `n` stack words from 0 to this.
const MAX_RET_WORDS: u32 = 64;

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
    /// Basic blocks run in `.text` (when coverage is on).
    blocks: BTreeSet<u32>,
    /// Instructions run (when counting is on).
    instructions: u64,
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
    /// Threads the game asked for (entry, parameter); not run yet.
    pub threads: Vec<(u32, u32)>,
    /// Message boxes the game showed (caption: text).
    pub messages: Vec<String>,
    /// Milestones reached (import names), in order.
    pub milestones: Vec<String>,
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
}

impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Returned => write!(f, "returned"),
            Self::Milestone(m) => write!(f, "milestone: {m}"),
            Self::Budget(eip) => write!(f, "time budget used up at {eip:#010x}"),
            Self::Failed(why, eip) => write!(f, "{why} (eip {eip:#010x})"),
        }
    }
}

/// The executor.
pub struct Exec {
    uc: Unicorn<'static, State>,
    arena: Arc<Arena>,
    image: Image,
    table: Table,
    imports: Imports,
    /// Import stub address → index into `imports`' bound list.
    stubs: HashMap<u32, usize>,
    sentinel: u32,
    ret_stubs: u32,
    ret_float_stubs: u32,
    float_cell: u32,
    stack: (u32, u32),
    deadline: Option<Instant>,
    stop_at: BTreeSet<String>,
    stopped: Option<Stop>,
    /// The counters.
    pub stats: Stats,
    /// The C runtime's state (heap, `atexit`, ...).
    pub crt: crate::imports::Crt,
    /// Lets unstubbed C runtime imports return 0 instead of stopping
    /// (they are still listed as missing).
    pub lenient: bool,
    /// Open files (the game folder).
    pub files: crate::files::Files,
}

impl std::fmt::Debug for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Exec")
            .field("image", &self.image.base)
            .field("ports", &self.table.len())
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl Exec {
    /// Builds the executor on `arena`, which already holds `image`: maps
    /// the arena's regions into Unicorn, the stack, the thread block and
    /// the stubs, binds the imports and marks the ports.
    ///
    /// # Errors
    /// When a mapping fails.
    pub fn new(arena: Arc<Arena>, image: Image, table: Table, imports: Imports) -> Result<Self> {
        let stack_base = arena.map_anywhere(REGION_HINT, STACK_SIZE, "main stack")?;
        let stubs_at = arena.map_anywhere(REGION_HINT, STUB_SIZE, "stubs")?;
        let teb_at = arena.map_anywhere(REGION_HINT, 4 * PAGE, "thread block")?;
        let heap = crate::imports::Crt::new(&arena)?;

        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, State::default())?;
        // A Core 2: no SSE4.2, whose string instructions Unicorn gets wrong
        // (ffx-oracle, the same choice).
        uc.ctl_set_cpu_model(2)?;

        let mut exec = Self {
            uc,
            arena,
            image,
            table,
            imports,
            stubs: HashMap::new(),
            sentinel: stubs_at,
            ret_stubs: stubs_at + 0x10,
            ret_float_stubs: stubs_at + 0x10 + 4 * (MAX_RET_WORDS + 1),
            float_cell: stubs_at + 0x800,
            stack: (stack_base, stack_base + STACK_SIZE),
            deadline: None,
            stop_at: BTreeSet::new(),
            stopped: None,
            stats: Stats::default(),
            crt: heap,
            files: crate::files::Files::default(),
            lenient: false,
        };
        exec.write_stubs(stubs_at);
        exec.bind_imports(stubs_at + 0x1000);
        for addr in exec.table.addresses().collect::<Vec<_>>() {
            if exec.image.is_code(addr) {
                exec.arena.fill(addr, &[INT3]);
            }
        }
        exec.map_into_unicorn()?;
        exec.thread_block(teb_at)?;
        exec.add_hooks()?;
        let top = exec.stack.1 - 0x100;
        exec.uc.reg_write(RegisterX86::ESP, u64::from(top))?;
        exec.uc
            .reg_write(RegisterX86::FPCW, u64::from(FPU_CONTROL))?;
        Ok(exec)
    }

    /// The arena.
    #[must_use]
    pub fn arena(&self) -> &Arc<Arena> {
        &self.arena
    }

    /// The arena as [`ffx_bridge::Mem`].
    #[must_use]
    pub fn mem(&self) -> ArenaMem {
        ArenaMem {
            arena: Arc::clone(&self.arena),
        }
    }

    /// The image.
    #[must_use]
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// The function table.
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.table
    }

    /// The basic blocks run in `.text` so far (with [`Exec::cover`] on).
    #[must_use]
    pub fn blocks(&self) -> &BTreeSet<u32> {
        &self.uc.get_data().blocks
    }

    /// Records every basic block run in `.text` (slower).
    ///
    /// # Errors
    /// When Unicorn refuses the hook.
    pub fn cover(&mut self) -> Result<()> {
        let Some(text) = self.image.text().cloned() else {
            return Ok(());
        };
        let (lo, hi) = (u64::from(text.start), u64::from(text.start + text.size));
        self.uc.add_block_hook(lo, hi - 1, |uc, addr, _| {
            #[allow(clippy::cast_possible_truncation)] // a 32-bit address
            uc.get_data_mut().blocks.insert(addr as u32);
        })?;
        Ok(())
    }

    /// Counts every emulated instruction (much slower; for measuring the
    /// executor's speed against a run without it).
    ///
    /// # Errors
    /// When Unicorn refuses the hook.
    pub fn count_instructions(&mut self) -> Result<()> {
        self.uc.add_code_hook(0, u64::from(u32::MAX), |uc, _, _| {
            uc.get_data_mut().instructions += 1;
        })?;
        Ok(())
    }

    /// Instructions emulated so far (with [`Exec::count_instructions`]).
    #[must_use]
    pub fn instructions(&self) -> u64 {
        self.uc.get_data().instructions
    }

    /// Stops the run when the import `name` (`USER32.dll!CreateWindowExW`)
    /// is first called: a milestone.
    pub fn stop_at(&mut self, name: &str) {
        self.stop_at.insert(name.to_string());
    }

    /// Ends the run after `budget` of wall-clock time.
    pub fn set_budget(&mut self, budget: Duration) {
        self.deadline = Some(Instant::now() + budget);
    }

    /// Ends the run (from a port or an import): every level unwinds with
    /// [`Error::Stopped`].
    pub fn end(&mut self, why: Stop) {
        if self.stopped.is_none() {
            if let Stop::Milestone(m) = &why {
                self.stats.milestones.push(m.clone());
            }
            self.stopped = Some(why);
        }
    }

    /// Why the last run stopped early, if it did.
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
        if self.stopped.is_some() {
            return Err(Error::Stopped);
        }
        let r = if let Some(entry) = self.table.get(addr).copied() {
            *self.stats.ports.entry(addr).or_default() += 1;
            let ret = (entry.port)(self, args);
            if self.stopped.is_some() {
                Err(Error::Stopped)
            } else {
                Ok(ret)
            }
        } else if let Some(&i) = self.stubs.get(&addr) {
            self.run_import(i, args)
        } else {
            self.call_unicorn(addr, conv, args)
        };
        // A failure anywhere ends the whole run: callers (ports, `_initterm`)
        // can't all carry a `Result`, so every level unwinds with `Stopped`.
        match r {
            Err(e) if !matches!(e, Error::Stopped) => {
                let eip = self.reg(RegisterX86::EIP).unwrap_or(0);
                self.end(Stop::Failed(e.to_string(), eip));
                Err(Error::Stopped)
            }
            r => r,
        }
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
        let run = self.run(addr);
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
    /// initialisers, `WinMain`) until it returns or stops.
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn run_entry(&mut self) -> Result<Stop> {
        let entry = self.image.entry;
        match self.call(entry, Conv::Stdcall, &Args::default()) {
            Ok(_) => Ok(Stop::Returned),
            Err(Error::Stopped) => Ok(self.stopped.clone().unwrap_or(Stop::Returned)),
            Err(e) => Err(e),
        }
    }

    /// Emulates from `pc` until the sentinel returns, taking ports and
    /// imports on the way.
    fn run(&mut self, mut pc: u32) -> Result<()> {
        loop {
            self.uc.get_data_mut().event = None;
            let timeout = match self.deadline {
                Some(d) => {
                    let left = d.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        self.stopped = Some(Stop::Budget(pc));
                        return Err(Error::Stopped);
                    }
                    u64::try_from(left.as_micros()).unwrap_or(u64::MAX).max(1)
                }
                None => 0,
            };
            self.stats.starts += 1;
            let t = Instant::now();
            let r = self.uc.emu_start(u64::from(pc), 0, timeout, 0);
            self.stats.emulated += t.elapsed();
            let event = self.uc.get_data_mut().event.take();
            match event {
                Some(Event::Int3(at)) if at == self.sentinel => return Ok(()),
                Some(Event::Int3(at)) => {
                    pc = if let Some(&i) = self.stubs.get(&at) {
                        self.enter_import(i)?
                    } else if self.table.get(at).is_some() {
                        self.enter_port(at)?
                    } else {
                        return Err(Error::Interrupt(3, at));
                    };
                }
                Some(Event::Interrupt(n, at)) => return Err(Error::Interrupt(n, at)),
                Some(Event::BadAccess(at, write)) => {
                    let eip = self.reg(RegisterX86::EIP)?;
                    return Err(Error::BadAccess {
                        addr: at,
                        eip,
                        write,
                    });
                }
                None => {
                    r?;
                    let eip = self.reg(RegisterX86::EIP)?;
                    if self.deadline.is_some_and(|d| Instant::now() >= d) {
                        self.stopped = Some(Stop::Budget(eip));
                        return Err(Error::Stopped);
                    }
                    return Err(Error::Ended(eip));
                }
            }
        }
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
        let entry = *self.table.get(at).ok_or(Error::Interrupt(3, at))?;
        *self.stats.ports.entry(at).or_default() += 1;
        let args = self.entry_args(entry.words)?;
        let saved = self.save_regs()?;
        let ret = (entry.port)(self, &args);
        self.restore_regs(&saved)?;
        if let Some(stop) = &self.stopped {
            let _ = stop;
            return Err(Error::Stopped);
        }
        let pops = if entry.conv.callee_pops() {
            entry.words
        } else {
            0
        };
        self.finish(ret, pops)
    }

    /// Runs import `i` for Unicorn code that called it.
    fn enter_import(&mut self, i: usize) -> Result<u32> {
        let words = self.imports.bound(i).map_or(0, |b| b.words);
        let args = self.entry_args(words)?;
        let saved = self.save_regs()?;
        let ret = self.run_import(i, &args);
        self.restore_regs(&saved)?;
        let ret = ret?;
        let pops = match self.imports.bound(i) {
            Some(b) if b.conv.callee_pops() => b.words,
            _ => 0,
        };
        self.finish(ret, pops)
    }

    fn run_import(&mut self, i: usize, args: &Args) -> Result<Ret> {
        let Some(b) = self.imports.bound(i).cloned() else {
            return Err(Error::Interrupt(3, 0));
        };
        *self.stats.imports.entry(b.name.clone()).or_default() += 1;
        if self.stop_at.contains(&b.name) {
            self.stats.milestones.push(b.name.clone());
            self.stopped = Some(Stop::Milestone(b.name.clone()));
            return Err(Error::Stopped);
        }
        match b.run {
            Some(f) => {
                let ret = f(self, args);
                if self.stopped.is_some() {
                    return Err(Error::Stopped);
                }
                Ok(ret)
            }
            None => {
                if !self.stats.missing_imports.contains(&b.name) {
                    self.stats.missing_imports.push(b.name.clone());
                }
                // lenient: a C runtime function (cdecl, so nothing to pop)
                // returns 0 and the run goes on, to see further
                let cdecl = b.name.starts_with("MSVCR110.dll!")
                    && !b.name.contains('?')
                    && !b.name.contains("_CI");
                if self.lenient && cdecl {
                    return Ok(Ret::eax(0));
                }
                let caller = if args.slots == 0 {
                    0
                } else {
                    self.mem().read::<u32>(args.slots - 4)
                };
                Err(Error::UnstubbedImport(format!(
                    "{} (called from {caller:#010x})",
                    b.name
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

    fn reg(&self, r: RegisterX86) -> Result<u32> {
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

    /// The sentinel, the `ret n` and `fld; ret n` stubs.
    fn write_stubs(&mut self, at: u32) {
        self.arena.fill(at, &[INT3]);
        for n in 0..=MAX_RET_WORDS {
            let mut code = [0x90u8; 4];
            if n == 0 {
                code[0] = 0xc3;
            } else {
                code[0] = 0xc2;
                #[allow(clippy::cast_possible_truncation)] // at most 256
                code[1..3].copy_from_slice(&((4 * n) as u16).to_le_bytes());
            }
            self.arena.fill(self.ret_stubs + 4 * n, &code);
            // fld qword ptr [cell]; ret n
            let mut f = [0x90u8; 12];
            f[0] = 0xdd;
            f[1] = 0x05;
            f[2..6].copy_from_slice(&self.float_cell.to_le_bytes());
            f[6..10].copy_from_slice(&code);
            self.arena.fill(self.ret_float_stubs + 12 * n, &f);
        }
    }

    /// Points every IAT slot at its own `int3` stub, and each data import
    /// at a cell of its own.
    fn bind_imports(&mut self, mut at: u32) {
        let imports = self.image.imports.clone();
        let mut cells = at + 0x8000;
        let mut mem = self.mem();
        let cmdline = cells;
        mem.write_bytes(cmdline, b"FFX.exe\0");
        cells += 0x10;
        self.crt.cmdline = cmdline;
        for (i, imp) in imports.iter().enumerate() {
            let full = format!("{}!{}", imp.dll, imp.name);
            if let Some(&(_, v)) = crate::imports::DATA.iter().find(|(n, _)| *n == full) {
                let v = if imp.name == "_acmdln" { cmdline } else { v };
                ffx_bridge::Mem::write(&mut mem, cells, v);
                if imp.name == "_HUGE" {
                    ffx_bridge::Mem::write(&mut mem, cells + 4, 0x7ff0_0000u32);
                }
                ffx_bridge::Mem::write(&mut mem, imp.slot, cells);
                cells += 0x10;
                continue;
            }
            self.imports.bind(i, &imp.dll, &imp.name);
            self.arena.fill(at, &[INT3]);
            self.arena.fill(imp.slot, &at.to_le_bytes());
            self.stubs.insert(at, i);
            at += 4;
        }
    }

    /// Every arena region into Unicorn, on the same host memory.
    fn map_into_unicorn(&mut self) -> Result<()> {
        for r in self.arena.regions() {
            // SAFETY: the region is committed host memory owned by the
            // arena, which outlives the emulator (`Exec` holds both, the
            // emulator dropped first).
            unsafe {
                self.uc.mem_map_ptr(
                    u64::from(r.start),
                    u64::from(r.size),
                    Prot::ALL,
                    self.arena.ptr(r.start).cast(),
                )?;
            }
        }
        Ok(())
    }

    /// Maps a region the game asked for at run time (heap growth,
    /// `VirtualAlloc`) into the arena and Unicorn.
    ///
    /// # Errors
    /// When the arena or Unicorn refuse it.
    pub fn map_region(&mut self, size: u32, name: &str) -> Result<u32> {
        let at = self.arena.map_anywhere(REGION_HINT, size, name)?;
        let size = size.next_multiple_of(PAGE);
        // SAFETY: as in `map_into_unicorn`.
        unsafe {
            self.uc.mem_map_ptr(
                u64::from(at),
                u64::from(size),
                Prot::ALL,
                self.arena.ptr(at).cast(),
            )?;
        }
        Ok(at)
    }

    /// The main thread's TEB at `at + PAGE` (fs), a GDT at `at`, an empty
    /// PEB at `at + 2 * PAGE`: what MSVC's start-up code and SEH frames
    /// read (`fs:[0]`, the stack limits, `fs:[0x18]`, `fs:[0x30]`).
    fn thread_block(&mut self, at: u32) -> Result<()> {
        let teb = at + PAGE;
        let peb = at + 2 * PAGE;
        let mut mem = self.mem();
        let w = |m: &mut ArenaMem, a: u32, v: u32| ffx_bridge::Mem::write(m, a, v);
        w(&mut mem, teb, u32::MAX);
        w(&mut mem, teb + 4, self.stack.1);
        w(&mut mem, teb + 8, self.stack.0);
        w(&mut mem, teb + 0x18, teb);
        w(&mut mem, teb + 0x24, 1);
        w(&mut mem, teb + 0x2c, at + 3 * PAGE); // TLS slots
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
        self.uc.add_intr_hook(|uc, intno| {
            #[allow(clippy::cast_possible_truncation)] // 32-bit eip
            let eip = uc.reg_read(RegisterX86::EIP).unwrap_or(0) as u32;
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
        Ok(())
    }

    /// A heap block holding `code`, runnable by Unicorn (tests and
    /// trampolines).
    ///
    /// # Errors
    /// When the heap is full.
    pub fn alloc_code(&mut self, code: &[u8]) -> Result<u32> {
        let mut mem = self.mem();
        let len = u32::try_from(code.len()).unwrap_or(u32::MAX);
        let at = self.crt.alloc(&mut mem, len);
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
        &self.imports
    }

    /// Runs an import's Rust side directly (tests).
    ///
    /// # Errors
    /// As [`Exec::call`].
    pub fn call_import(&mut self, f: ImportFn, args: &Args) -> Result<Ret> {
        Ok(f(self, args))
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
