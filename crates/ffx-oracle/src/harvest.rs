//! Real call states, harvested from the original code (docs/hooking.md,
//! "Harvested call states").
//!
//! [`Oracle::harvest`] runs a driver (an original top-level routine such as
//! `MsBattleProcess`) for some frames on a snapshot; a code hook at each
//! target function's entry saves what that call started from: the
//! registers, the x87 and SSE state, a window of the stack above `ESP` and
//! every page written since the oracle was made (or since the last
//! [`Oracle::clear_dirty`]). A state is a small delta over its snapshot.
//! [`Store`] keeps them as `<root>/<snapshot>/<addr>/<n>.state`, LZ4
//! compressed; [`harvested`] reads them back for tests, and [`Case::check`]
//! runs the original from the saved registers in one oracle and the Rust
//! port in another, then compares results, every byte written outside the
//! stacks, and the stack above the arguments (out-parameters in the
//! caller's frame). [`harvest_test!`](crate::harvest_test) makes that one
//! line per port.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use unicorn_engine::{RegisterX86, Unicorn};

use crate::{Conv, Error, Oracle, PAGE, PAGE_BYTES, Pages, Result, Ret, STACK_SIZE, State};

/// The first bytes of a state file; the dedupe key follows, then the LZ4
/// block (`lz4_flex`, size prepended) of the rest.
pub const STATE_MAGIC: &[u8; 8] = b"FFXHARV1";
/// How much stack above `ESP` a state keeps.
pub const STACK_WINDOW: u32 = 0x1_0000;
/// `Phyre_PTimerWin32_GetTime` (`ffx.sym`).
const GET_TIME: u32 = 0x0042_fd80;
/// Blocks kept in [`Harvest::trail`].
const TRAIL: usize = 24;
/// Stack words after the return address that go into the dedupe key.
const KEY_WORDS: u32 = 8;
/// Bytes behind each key word that points at mapped memory, also hashed.
const KEY_DEREF: usize = 64;

/// The function a harvest runs, `frames` times in a row.
#[derive(Debug, Clone)]
pub struct Driver {
    /// Its `ffx.sym` address.
    pub addr: u32,
    pub conv: Conv,
    pub args: Vec<u32>,
    pub frames: u32,
}

/// The integer registers at a target's first instruction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Regs {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    pub eflags: u32,
}

/// Where the harvesting oracle had its own memory; a replaying oracle must
/// have the same, since the saved pointers point there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Layout {
    pub image_base: u32,
    pub stack_base: u32,
    pub heap_base: u32,
    /// The oracle heap's next free address.
    pub heap_next: u32,
    /// The runtime DLL heap's next free address (0: no DLL).
    pub crt_heap_next: u32,
    /// The native `rand` state.
    pub rand_seed: u32,
}

/// One call as the original code made it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallState {
    /// The called function's `ffx.sym` address.
    pub target: u32,
    /// The driver frame it was reached in (from 0).
    pub frame: u32,
    /// Which hit of the target it was (from 0, counting duplicates).
    pub hit: u64,
    /// Hash of the target, `EAX`/`ECX`/`EDX`, the first stack words and the
    /// bytes they point at: equal keys are taken as the same call.
    pub key: u64,
    pub regs: Regs,
    pub fpcw: u32,
    pub fpsw: u32,
    pub fptag: u32,
    pub st: [[u8; 10]; 8],
    pub mxcsr: u32,
    pub xmm: [[u8; 16]; 8],
    pub layout: Layout,
    /// The stack from `ESP` up ([`STACK_WINDOW`] at most); `[0..4]` is the
    /// return address.
    pub stack: Vec<u8>,
    /// Every page written before the call, outside the oracle's stack:
    /// `(page number, 4 KiB)`.
    pub pages: Vec<(u32, Vec<u8>)>,
}

impl CallState {
    /// Stack argument `i` (the word at `ESP + 4 + 4i`), 0 beyond the window.
    #[must_use]
    pub fn arg(&self, i: u32) -> u32 {
        let at = 4 + 4 * i as usize;
        self.stack
            .get(at..at + 4)
            .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }

    /// Stack argument `i` as a float.
    #[must_use]
    pub fn arg_f32(&self, i: u32) -> f32 {
        f32::from_bits(self.arg(i))
    }

    /// `ECX` (`this` of a thiscall, first fastcall argument).
    #[must_use]
    pub fn ecx(&self) -> u32 {
        self.regs.ecx
    }

    /// `EDX` (second fastcall argument).
    #[must_use]
    pub fn edx(&self) -> u32 {
        self.regs.edx
    }

    /// `EAX` (some originals take an argument there).
    #[must_use]
    pub fn eax(&self) -> u32 {
        self.regs.eax
    }

    /// Serialises the state (the file format of [`Store`]).
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(0x200 + self.stack.len() + self.pages.len() * 0x1004);
        let w = |b: &mut Vec<u8>, v: u32| b.extend_from_slice(&v.to_le_bytes());
        w(&mut b, self.target);
        w(&mut b, self.frame);
        b.extend_from_slice(&self.hit.to_le_bytes());
        let r = &self.regs;
        for v in [
            r.eax, r.ecx, r.edx, r.ebx, r.esp, r.ebp, r.esi, r.edi, r.eflags,
        ] {
            w(&mut b, v);
        }
        for v in [self.fpcw, self.fpsw, self.fptag, self.mxcsr] {
            w(&mut b, v);
        }
        for s in &self.st {
            b.extend_from_slice(s);
        }
        for x in &self.xmm {
            b.extend_from_slice(x);
        }
        let l = &self.layout;
        for v in [
            l.image_base,
            l.stack_base,
            l.heap_base,
            l.heap_next,
            l.crt_heap_next,
            l.rand_seed,
        ] {
            w(&mut b, v);
        }
        w(
            &mut b,
            u32::try_from(self.stack.len()).expect("small stack"),
        );
        b.extend_from_slice(&self.stack);
        w(&mut b, u32::try_from(self.pages.len()).expect("page count"));
        for (p, data) in &self.pages {
            w(&mut b, *p);
            b.extend_from_slice(data);
        }
        b
    }

    /// Reads [`CallState::to_bytes`]'s format.
    ///
    /// # Errors
    /// When the bytes are cut short or malformed.
    pub fn from_bytes(bytes: &[u8], key: u64) -> Result<Self> {
        let bad = || Error::Io(std::io::Error::other("malformed harvested state"));
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8]> {
            let s = bytes.get(at..at + n).ok_or_else(bad)?;
            at += n;
            Ok(s)
        };
        let mut s = Self {
            key,
            ..Self::default()
        };
        macro_rules! word {
            () => {
                u32::from_le_bytes(take(4)?.try_into().expect("4 bytes"))
            };
        }
        s.target = word!();
        s.frame = word!();
        s.hit = u64::from_le_bytes(take(8)?.try_into().expect("8 bytes"));
        s.regs = Regs {
            eax: word!(),
            ecx: word!(),
            edx: word!(),
            ebx: word!(),
            esp: word!(),
            ebp: word!(),
            esi: word!(),
            edi: word!(),
            eflags: word!(),
        };
        s.fpcw = word!();
        s.fpsw = word!();
        s.fptag = word!();
        s.mxcsr = word!();
        for st in &mut s.st {
            st.copy_from_slice(take(10)?);
        }
        for x in &mut s.xmm {
            x.copy_from_slice(take(16)?);
        }
        s.layout = Layout {
            image_base: word!(),
            stack_base: word!(),
            heap_base: word!(),
            heap_next: word!(),
            crt_heap_next: word!(),
            rand_seed: word!(),
        };
        let n = word!() as usize;
        s.stack = take(n)?.to_vec();
        let pages = word!() as usize;
        for _ in 0..pages {
            let p = word!();
            s.pages.push((p, take(PAGE_BYTES)?.to_vec()));
        }
        Ok(s)
    }

    /// The compressed file bytes.
    #[must_use]
    pub fn to_file(&self) -> Vec<u8> {
        let mut out = STATE_MAGIC.to_vec();
        out.extend_from_slice(&self.key.to_le_bytes());
        out.extend_from_slice(&lz4_flex::compress_prepend_size(&self.to_bytes()));
        out
    }

    /// Reads a state file.
    ///
    /// # Errors
    /// On I/O errors or a malformed file.
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let key = file_key(&bytes)?;
        let body = lz4_flex::decompress_size_prepended(&bytes[16..])
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;
        Self::from_bytes(&body, key)
    }
}

/// The dedupe key in a state file's header.
fn file_key(bytes: &[u8]) -> Result<u64> {
    if bytes.len() < 16 || &bytes[..8] != STATE_MAGIC {
        return Err(Error::Io(std::io::Error::other("not a harvested state")));
    }
    Ok(u64::from_le_bytes(
        bytes[8..16].try_into().expect("8 bytes"),
    ))
}

/// What a harvest found.
#[derive(Debug, Default)]
pub struct Harvest {
    pub states: Vec<CallState>,
    /// Driver frames that returned.
    pub frames: u32,
    /// Why the driver stopped early, if it did (the states up to there
    /// stay valid).
    pub error: Option<Error>,
    /// Where it stopped: `EIP` and the word at `[ESP]` (often the return
    /// address of the call that failed).
    pub stopped_at: Option<(u32, u32)>,
    /// The last blocks run in the targets' span before the stop (rebased).
    pub trail: Vec<u32>,
    /// Entries seen per target (`ffx.sym` address), duplicates included.
    pub hits: HashMap<u32, u64>,
}

/// The hook's bookkeeping during a harvest.
#[derive(Default)]
pub(crate) struct Run {
    /// Rebased entry -> `ffx.sym` address.
    targets: HashMap<u64, u32>,
    max: usize,
    seen: HashMap<u32, HashSet<u64>>,
    hits: HashMap<u32, u64>,
    states: Vec<CallState>,
    frame: u32,
    /// The last blocks run in the targets' span, oldest first.
    trail: std::collections::VecDeque<u32>,
}

impl Oracle {
    /// Runs `driver` and saves the state of each call into `targets`
    /// (`ffx.sym` addresses), at most `max_per_target` distinct ones per
    /// target (by [`CallState::key`]). Pages written before the harvest
    /// (since the last [`Oracle::clear_dirty`]) are part of every state;
    /// the oracle keeps what the driver did afterwards.
    ///
    /// # Errors
    /// Only on emulator errors setting the hooks up; a failing driver ends
    /// the harvest with [`Harvest::error`] set.
    pub fn harvest(
        &mut self,
        driver: &Driver,
        targets: &[u32],
        max_per_target: usize,
    ) -> Result<Harvest> {
        let mut run = Run {
            max: max_per_target,
            ..Run::default()
        };
        for &t in targets {
            run.targets.insert(u64::from(self.rebase(t)), t);
        }
        let lo = run.targets.keys().copied().min().unwrap_or(0);
        let hi = run.targets.keys().copied().max().unwrap_or(0);
        let any = !run.targets.is_empty();
        self.uc.get_data_mut().harvest = Some(run);
        // One block hook over the targets' span: a call's target always
        // starts a translated block, and a block hook costs per block, not
        // per instruction (thousands of code hooks would slow translation).
        let mut hooks = Vec::new();
        if any {
            hooks.push(
                self.uc
                    .add_block_hook(lo, hi, |uc, addr, _| capture(uc, addr))?,
            );
        }
        // code translated before the hooks has no callback in it
        self.uc.ctl_flush_tb()?;
        let entry = self.rebase(driver.addr);
        let mut frames = 0;
        let mut error = None;
        let mut stopped_at = None;
        for f in 0..driver.frames {
            if let Some(r) = self.uc.get_data_mut().harvest.as_mut() {
                r.frame = f;
            }
            match self.call(entry, driver.conv, &driver.args) {
                Ok(_) => frames += 1,
                Err(e) => {
                    error = Some(e);
                    let eip = self.uc.reg_read(RegisterX86::EIP).unwrap_or(0);
                    let esp = self.uc.reg_read(RegisterX86::ESP).unwrap_or(0);
                    let top = self
                        .uc
                        .mem_read_as_vec(esp, 4)
                        .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
                    stopped_at = Some((u32::try_from(eip).unwrap_or(0), top));
                    break;
                }
            }
        }
        for h in hooks {
            self.uc.remove_hook(h)?;
        }
        self.uc.ctl_flush_tb()?;
        let run = self.uc.get_data_mut().harvest.take().unwrap_or_default();
        Ok(Harvest {
            states: run.states,
            frames,
            error,
            stopped_at,
            trail: run.trail.into_iter().collect(),
            hits: run.hits,
        })
    }

    /// Replaces the game's clock (`Phyre_PTimerWin32_GetTime`, which calls
    /// a system DLL the oracle lacks) with one that starts at `start`
    /// seconds and advances `step` per call. Written through
    /// [`Oracle::write`], so harvested states carry it.
    ///
    /// # Errors
    /// On emulator errors.
    pub fn fake_clock(&mut self, start: f64, step: f64) -> Result<()> {
        let mut code = vec![0u8; 0x30];
        let at = self.alloc_code(&code)?;
        let (cell, inc) = (at + 0x20, at + 0x28);
        code.clear();
        code.extend_from_slice(&[0xdd, 0x05]); // fld qword [cell]
        code.extend_from_slice(&cell.to_le_bytes());
        code.extend_from_slice(&[0xdc, 0x05]); // fadd qword [inc]
        code.extend_from_slice(&inc.to_le_bytes());
        code.extend_from_slice(&[0xdd, 0x15]); // fst qword [cell]
        code.extend_from_slice(&cell.to_le_bytes());
        code.push(0xc3);
        code.resize(0x20, 0xcc);
        code.extend_from_slice(&start.to_le_bytes());
        code.extend_from_slice(&step.to_le_bytes());
        self.write(at, &code)?;
        let site = self.rebase(GET_TIME);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&at.wrapping_sub(site + 5).to_le_bytes());
        self.write(site, &jmp)?;
        Ok(())
    }

    /// The oracle's own memory layout now.
    #[must_use]
    pub fn layout(&self) -> Layout {
        let s = self.uc.get_data();
        let w = |v: u64| u32::try_from(v).unwrap_or(0);
        Layout {
            image_base: s.image_base,
            stack_base: w(s.stack_base),
            heap_base: w(s.heap_base),
            heap_next: w(s.heap_next),
            crt_heap_next: if s.dll_base == 0 { 0 } else { w(s.crt_heap.0) },
            rand_seed: s.rand_seed,
        }
    }

    /// Puts a harvested state into this oracle (made from the same snapshot
    /// with the same runtime): its pages, stack window, registers and heap
    /// positions; then forgets the written pages, so [`Oracle::diff`] sees
    /// only what follows.
    ///
    /// # Errors
    /// When this oracle's memory layout differs from the harvesting one's,
    /// or on emulator errors.
    pub fn apply_state(&mut self, s: &CallState) -> Result<()> {
        let mine = self.layout();
        let theirs = s.layout;
        if (mine.image_base, mine.stack_base, mine.heap_base)
            != (theirs.image_base, theirs.stack_base, theirs.heap_base)
            || (mine.crt_heap_next == 0) != (theirs.crt_heap_next == 0)
        {
            return Err(Error::LayoutMismatch(format!(
                "state {theirs:x?}, oracle {mine:x?}"
            )));
        }
        for (p, data) in &s.pages {
            let a = *p << 12;
            self.touch(a, PAGE_BYTES);
            self.uc
                .mem_write(u64::from(a), data)
                .map_err(|_| Error::BadAccess(u64::from(a)))?;
        }
        self.uc.mem_write(u64::from(s.regs.esp), &s.stack)?;
        {
            let st = self.uc.get_data_mut();
            st.heap_next = u64::from(theirs.heap_next);
            if theirs.crt_heap_next != 0 {
                st.crt_heap.0 = u64::from(theirs.crt_heap_next);
            }
            st.rand_seed = theirs.rand_seed;
            // calls from the port's side go below the saved frames
            st.call_top = Some(u64::from(s.regs.esp.saturating_sub(0x1000) & !0xf));
        }
        let r = &s.regs;
        for (reg, v) in [
            (RegisterX86::EAX, r.eax),
            (RegisterX86::ECX, r.ecx),
            (RegisterX86::EDX, r.edx),
            (RegisterX86::EBX, r.ebx),
            (RegisterX86::ESP, r.esp),
            (RegisterX86::EBP, r.ebp),
            (RegisterX86::ESI, r.esi),
            (RegisterX86::EDI, r.edi),
            (RegisterX86::EFLAGS, r.eflags),
            (RegisterX86::FPCW, s.fpcw),
            (RegisterX86::MXCSR, s.mxcsr),
        ] {
            self.uc.reg_write(reg, u64::from(v))?;
        }
        for (reg, x) in XMM.iter().zip(&s.xmm) {
            self.uc.reg_write_long(*reg, x)?;
        }
        // the x87 stack is empty at a call, as the conventions require
        // (and as Oracle::call leaves it)
        self.uc
            .reg_write(RegisterX86::FPSW, u64::from(s.fpsw & !0x3800))?;
        self.uc.reg_write(RegisterX86::FPTAG, 0xffff)?;
        self.uc.ctl_flush_tb()?;
        self.clear_dirty();
        Ok(())
    }

    /// Runs the harvested call's original from its saved registers (after
    /// [`Oracle::apply_state`]), returning to the oracle instead of the
    /// saved return address.
    ///
    /// # Errors
    /// As [`Oracle::call`]: faults, runaways, callee-saved registers or
    /// `ESP` not as the conventions require.
    pub fn run_state(&mut self, s: &CallState) -> Result<StateRet> {
        let sentinel = u32::try_from(self.uc.get_data().sentinel).expect("32-bit");
        let r = s.regs;
        self.uc
            .mem_write(u64::from(r.esp), &sentinel.to_le_bytes())?;
        self.uc.reg_write(RegisterX86::ESP, u64::from(r.esp))?;
        self.uc.get_data_mut().fault = None;
        self.uc.get_data_mut().returned = false;
        let entry = self.rebase(s.target);
        let run = self
            .uc
            .emu_start(u64::from(entry), 0, 0, crate::MAX_INSTRUCTIONS);
        if let Some(fault) = self.uc.get_data_mut().fault.take() {
            return Err(fault);
        }
        run?;
        if !self.uc.get_data().returned {
            return Err(Error::Runaway(self.uc.reg_read(RegisterX86::EIP)?));
        }
        for (reg, name, v) in [
            (RegisterX86::EBX, "EBX", r.ebx),
            (RegisterX86::ESI, "ESI", r.esi),
            (RegisterX86::EDI, "EDI", r.edi),
            (RegisterX86::EBP, "EBP", r.ebp),
        ] {
            if self.uc.reg_read(reg)? != u64::from(v) {
                return Err(Error::Clobbered(name));
            }
        }
        let esp = u32::try_from(self.uc.reg_read(RegisterX86::ESP)?).expect("32-bit");
        let popped = esp.wrapping_sub(r.esp.wrapping_add(4));
        if popped > 0x100 || popped % 4 != 0 {
            return Err(Error::Clobbered("ESP"));
        }
        let eax = u32::try_from(self.uc.reg_read(RegisterX86::EAX)?).expect("32-bit");
        let edx = u32::try_from(self.uc.reg_read(RegisterX86::EDX)?).expect("32-bit");
        let st0_raw = self.st0_raw()?;
        Ok(StateRet {
            ret: Ret {
                eax,
                edx,
                st0: st0_raw.map(|b| crate::f80_to_f64(&b)),
                st0_raw,
            },
            popped,
        })
    }
}

const XMM: [RegisterX86; 8] = [
    RegisterX86::XMM0,
    RegisterX86::XMM1,
    RegisterX86::XMM2,
    RegisterX86::XMM3,
    RegisterX86::XMM4,
    RegisterX86::XMM5,
    RegisterX86::XMM6,
    RegisterX86::XMM7,
];

const STS: [RegisterX86; 8] = [
    RegisterX86::ST0,
    RegisterX86::ST1,
    RegisterX86::ST2,
    RegisterX86::ST3,
    RegisterX86::ST4,
    RegisterX86::ST5,
    RegisterX86::ST6,
    RegisterX86::ST7,
];

/// An oracle as the bridges' [`ffx_bridge::Host`]: its memory, and its
/// emulator for the original callees (cdecl; a failing callee panics).
pub struct OracleHost<'a>(pub &'a mut Oracle);

impl ffx_bridge::Mem for OracleHost<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        ffx_bridge::Mem::read_bytes(&*self.0, addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        ffx_bridge::Mem::write_bytes(self.0, addr, data);
    }
    fn image_base(&self) -> u32 {
        ffx_bridge::Mem::image_base(&*self.0)
    }
}

impl ffx_bridge::Host for OracleHost<'_> {
    fn call(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.0
            .call(addr, Conv::Cdecl, args)
            .unwrap_or_else(|e| panic!("callee {addr:#x}: {e}"))
            .eax
    }

    fn callee_scratch(&mut self) -> Option<u32> {
        // the bottom of the emulated stack: no call reaches that deep, and
        // the diff leaves the stacks out
        Some(self.0.stack_range().start + 0x100)
    }
}

/// What a harvested call's original returned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateRet {
    pub ret: Ret,
    /// Bytes of arguments the callee popped (0 for cdecl).
    pub popped: u32,
}

/// The code hook at a target's entry.
fn capture(uc: &mut Unicorn<'_, State>, addr: u64) {
    if let Err(e) = try_capture(uc, addr) {
        uc.get_data_mut().fault = Some(e);
        let _ = uc.emu_stop();
    }
}

#[allow(clippy::too_many_lines)] // one capture, in order
fn try_capture(uc: &mut Unicorn<'_, State>, addr: u64) -> Result<()> {
    let Some(run) = uc.get_data_mut().harvest.as_mut() else {
        return Ok(());
    };
    if run.trail.len() == TRAIL {
        run.trail.pop_front();
    }
    run.trail.push_back(u32::try_from(addr).unwrap_or(0));
    let Some(&target) = run.targets.get(&addr) else {
        return Ok(());
    };
    let hit = *run.hits.get(&target).unwrap_or(&0);
    *run.hits.entry(target).or_default() += 1;
    if run.seen.get(&target).map_or(0, HashSet::len) >= run.max {
        return Ok(());
    }
    let frame = run.frame;
    let reg = |uc: &Unicorn<State>, r| -> Result<u32> {
        Ok(u32::try_from(uc.reg_read(r)? & 0xffff_ffff).expect("masked"))
    };
    let regs = Regs {
        eax: reg(uc, RegisterX86::EAX)?,
        ecx: reg(uc, RegisterX86::ECX)?,
        edx: reg(uc, RegisterX86::EDX)?,
        ebx: reg(uc, RegisterX86::EBX)?,
        esp: reg(uc, RegisterX86::ESP)?,
        ebp: reg(uc, RegisterX86::EBP)?,
        esi: reg(uc, RegisterX86::ESI)?,
        edi: reg(uc, RegisterX86::EDI)?,
        eflags: reg(uc, RegisterX86::EFLAGS)?,
    };
    // the key: who, register arguments, stack arguments and what they point at
    let mut h = DefaultHasher::new();
    (target, regs.eax, regs.ecx, regs.edx).hash(&mut h);
    let mut words: Vec<u32> = vec![regs.eax, regs.ecx, regs.edx];
    for i in 0..KEY_WORDS {
        let w = uc
            .mem_read_as_vec(u64::from(regs.esp) + 4 + 4 * u64::from(i), 4)
            .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
        w.hash(&mut h);
        words.push(w);
    }
    for w in words {
        if let Ok(b) = uc.mem_read_as_vec(u64::from(w), KEY_DEREF) {
            b.hash(&mut h);
        }
    }
    let key = h.finish();
    let run = uc
        .get_data_mut()
        .harvest
        .as_mut()
        .expect("still harvesting");
    if !run.seen.entry(target).or_default().insert(key) {
        return Ok(());
    }
    let s = uc.get_data();
    let stack_top = s.stack_base + STACK_SIZE;
    let stack_lo = s.stack_base;
    let esp = u64::from(regs.esp);
    let window = if (stack_lo..stack_top).contains(&esp) {
        (stack_top - esp).min(u64::from(STACK_WINDOW))
    } else {
        u64::from(STACK_WINDOW)
    };
    let stack = uc.mem_read_as_vec(esp, usize::try_from(window).expect("small"))?;
    let dirty: Vec<u32> = s
        .dirty
        .iter()
        .copied()
        .filter(|&p| !(stack_lo..stack_top).contains(&(u64::from(p) * PAGE)))
        .collect();
    // only pages that differ from what a fresh oracle on the snapshot
    // holds there: the snapshot's bytes, or zeros in the oracle's own heap
    let snap = s.pages.clone();
    let heap = s.heap_base..s.heap_base + crate::HEAP_SIZE;
    let mut pages = Vec::with_capacity(dirty.len());
    for p in dirty {
        let a = u64::from(p) * PAGE;
        let Ok(data) = uc.mem_read_as_vec(a, PAGE_BYTES) else {
            continue;
        };
        let base: Option<&[u8]> = snap.as_ref().and_then(|sp| {
            let (start, chunk) = sp.chunk(a)?;
            let off = usize::try_from(a - start).ok()?;
            chunk.get(off..off + PAGE_BYTES)
        });
        let same = match base {
            Some(b) => b == data.as_slice(),
            None => heap.contains(&a) && data.iter().all(|&x| x == 0),
        };
        if !same {
            pages.push((p, data));
        }
    }
    let mut st = [[0u8; 10]; 8];
    for (out, r) in st.iter_mut().zip(STS) {
        out.copy_from_slice(&uc.reg_read_long(r)?[..10]);
    }
    let mut xmm = [[0u8; 16]; 8];
    for (out, r) in xmm.iter_mut().zip(XMM) {
        out.copy_from_slice(&uc.reg_read_long(r)?[..16]);
    }
    let w = |v: u64| u32::try_from(v).unwrap_or(0);
    let layout = Layout {
        image_base: s.image_base,
        stack_base: w(s.stack_base),
        heap_base: w(s.heap_base),
        heap_next: w(s.heap_next),
        crt_heap_next: if s.dll_base == 0 { 0 } else { w(s.crt_heap.0) },
        rand_seed: s.rand_seed,
    };
    let state = CallState {
        target,
        frame,
        hit,
        key,
        regs,
        fpcw: reg(uc, RegisterX86::FPCW)?,
        fpsw: reg(uc, RegisterX86::FPSW)?,
        fptag: reg(uc, RegisterX86::FPTAG)?,
        st,
        mxcsr: reg(uc, RegisterX86::MXCSR)?,
        xmm,
        layout,
        stack,
        pages,
    };
    uc.get_data_mut()
        .harvest
        .as_mut()
        .expect("still harvesting")
        .states
        .push(state);
    Ok(())
}

/// The harvested states on disk: `<root>/<snapshot>/<addr>/<n>.state`,
/// `<root>/<snapshot>/source` holding the snapshot's path.
#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    /// `FFX_HARVEST`, else `~/ffx-work/harvest`.
    #[must_use]
    pub fn from_env() -> Self {
        let root = std::env::var_os("FFX_HARVEST").map_or_else(
            || {
                let home = std::env::var_os("HOME").unwrap_or_default();
                PathBuf::from(home).join("ffx-work/harvest")
            },
            PathBuf::from,
        );
        Self { root }
    }

    /// A store at `root`.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory of one target's states from one snapshot.
    #[must_use]
    pub fn dir(&self, snapshot: &str, target: u32) -> PathBuf {
        self.root.join(snapshot).join(format!("{target:08x}"))
    }

    /// The dedupe keys already stored for a target.
    #[must_use]
    pub fn keys(&self, snapshot: &str, target: u32) -> HashSet<u64> {
        state_files(&self.dir(snapshot, target))
            .into_iter()
            .filter_map(|f| {
                use std::io::Read;
                let mut head = [0u8; 16];
                std::fs::File::open(f).ok()?.read_exact(&mut head).ok()?;
                file_key(&head).ok()
            })
            .collect()
    }

    /// Saves a state unless one with its key is there; returns the new
    /// file.
    ///
    /// # Errors
    /// On I/O errors.
    pub fn save(&self, snapshot: &str, source: &Path, s: &CallState) -> Result<Option<PathBuf>> {
        let snap_dir = self.root.join(snapshot);
        std::fs::create_dir_all(&snap_dir)?;
        let src = snap_dir.join("source");
        if !src.exists() {
            std::fs::write(&src, source.to_string_lossy().as_bytes())?;
        }
        let dir = self.dir(snapshot, s.target);
        std::fs::create_dir_all(&dir)?;
        if self.keys(snapshot, s.target).contains(&s.key) {
            return Ok(None);
        }
        let n = state_files(&dir).len();
        #[allow(clippy::maybe_infinite_iter)] // some name is free
        let path = (n..)
            .map(|i| dir.join(format!("{i}.state")))
            .find(|p| !p.exists())
            .expect("a free name");
        let tmp = dir.join(format!(".{n}.tmp"));
        std::fs::write(&tmp, s.to_file())?;
        std::fs::rename(&tmp, &path)?;
        Ok(Some(path))
    }

    /// Every stored state of `target`: `(snapshot name, snapshot path,
    /// state file)`, by snapshot then number.
    #[must_use]
    pub fn cases(&self, target: u32) -> Vec<(String, PathBuf, PathBuf)> {
        let Ok(dirs) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut snaps: Vec<PathBuf> = dirs.filter_map(|d| Some(d.ok()?.path())).collect();
        snaps.sort();
        let mut out = Vec::new();
        for snap in snaps {
            let Some(name) = snap.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            let source = std::fs::read_to_string(snap.join("source"))
                .map_or_else(|_| default_snapshot(&name), |s| PathBuf::from(s.trim()));
            for f in state_files(&snap.join(format!("{target:08x}"))) {
                out.push((name.clone(), source.clone(), f));
            }
        }
        out
    }
}

fn default_snapshot(name: &str) -> PathBuf {
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(format!("ffx-work/snapshots/{name}.snap"))
}

/// `<n>.state` files in numeric order.
fn state_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<(u64, PathBuf)> = rd
        .filter_map(|e| {
            let p = e.ok()?.path();
            let n = p
                .file_name()?
                .to_str()?
                .strip_suffix(".state")?
                .parse()
                .ok()?;
            Some((n, p))
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, p)| p).collect()
}

/// Snapshots read once per process.
fn pages_for(path: &Path) -> Result<Arc<Pages>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<Pages>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(p) = cache.lock().expect("cache").get(path) {
        return Ok(p.clone());
    }
    let p = Pages::read(path)?;
    cache
        .lock()
        .expect("cache")
        .insert(path.to_path_buf(), p.clone());
    Ok(p)
}

/// One stored state with what replays it.
pub struct Case {
    /// `<snapshot>/<addr>/<n>`, for messages.
    pub name: String,
    pub exe: PathBuf,
    pub pages: Arc<Pages>,
    pub state: CallState,
}

/// The stored states of one target, for tests: `None` (with a message on
/// stderr) when `FFX_GAME_DIR` is unset or the store has none. Snapshots a
/// state needs but that are missing are skipped with a message.
#[must_use]
pub fn harvested(addr: u32) -> Option<Vec<Case>> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    let store = Store::from_env();
    let mut out = Vec::new();
    let limit = std::env::var("FFX_HARVEST_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(usize::MAX);
    let mut per_snapshot: HashMap<String, usize> = HashMap::new();
    for (snap, source, file) in store.cases(addr) {
        let n = per_snapshot.entry(snap.clone()).or_default();
        if *n >= limit {
            continue;
        }
        if !source.is_file() {
            eprintln!(
                "{}: snapshot missing; skipping its states",
                source.display()
            );
            continue;
        }
        let pages = match pages_for(&source) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}: {e}; skipping its states", source.display());
                continue;
            }
        };
        let state = CallState::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let stem = file
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
        out.push(Case {
            name: format!("{snap}/{addr:08x}/{stem}"),
            exe: exe.clone(),
            pages,
            state,
        });
        *n += 1;
    }
    if out.is_empty() {
        eprintln!(
            "no harvested states of {addr:#010x} in {}; skipping",
            store.root.display()
        );
        assert!(
            std::env::var_os("FFX_HARVEST_REQUIRE").is_none(),
            "FFX_HARVEST_REQUIRE is set and {addr:#010x} has no harvested states"
        );
        return None;
    }
    Some(out)
}

/// What a port returned, checked against the original's registers.
pub trait PortResult {
    /// # Errors
    /// A message saying what differs.
    fn compare(self, original: &StateRet) -> std::result::Result<(), String>;
    /// Whether this result is a float (else the original must leave the
    /// x87 stack empty).
    fn is_float(&self) -> bool {
        false
    }
}

impl PortResult for () {
    fn compare(self, _: &StateRet) -> std::result::Result<(), String> {
        Ok(())
    }
}

impl PortResult for u32 {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        if o.ret.eax == self {
            Ok(())
        } else {
            Err(format!("eax {:#x}, port {self:#x}", o.ret.eax))
        }
    }
}

impl PortResult for i32 {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        self.cast_unsigned().compare(o)
    }
}

impl PortResult for bool {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        u32::from(self).compare(o)
    }
}

/// An integer result compared under a mask (`Masked(v, 0xff)`: `AL`).
#[derive(Debug, Clone, Copy)]
pub struct Masked(pub u32, pub u32);

impl PortResult for Masked {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        if o.ret.eax & self.1 == self.0 & self.1 {
            Ok(())
        } else {
            Err(format!(
                "eax {:#x}, port {:#x} (mask {:#x})",
                o.ret.eax, self.0, self.1
            ))
        }
    }
}

/// `EDX:EAX`.
impl PortResult for u64 {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        let got = u64::from(o.ret.edx) << 32 | u64::from(o.ret.eax);
        if got == self {
            Ok(())
        } else {
            Err(format!("edx:eax {got:#x}, port {self:#x}"))
        }
    }
}

impl PortResult for f64 {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        let Some(st0) = o.ret.st0 else {
            return Err(format!("no float in ST0, port {self}"));
        };
        // bit for bit: NaN signs and payloads too (`Ret::st0` keeps them)
        if crate::same_float(st0, self) {
            Ok(())
        } else {
            Err(format!(
                "ST0 {st0} ({:#018x}), port {self} ({:#018x})",
                st0.to_bits(),
                self.to_bits()
            ))
        }
    }
    fn is_float(&self) -> bool {
        true
    }
}

impl PortResult for f32 {
    fn compare(self, o: &StateRet) -> std::result::Result<(), String> {
        f64::from(self).compare(o)
    }
    fn is_float(&self) -> bool {
        true
    }
}

/// No result check at all (the original's registers are garbage).
#[derive(Debug, Clone, Copy)]
pub struct Unchecked;

impl PortResult for Unchecked {
    fn compare(self, _: &StateRet) -> std::result::Result<(), String> {
        Ok(())
    }
    fn is_float(&self) -> bool {
        true
    }
}

impl Case {
    /// Two oracles on the case's snapshot with its state applied.
    ///
    /// # Errors
    /// When the oracles cannot be made or the state does not fit them.
    pub fn oracles(&self) -> Result<(Oracle, Oracle)> {
        let mut a = Oracle::from_snapshot(&self.exe, &self.pages)?;
        let mut b = Oracle::from_snapshot(&self.exe, &self.pages)?;
        a.apply_state(&self.state)?;
        b.apply_state(&self.state)?;
        Ok((a, b))
    }

    /// Runs the original in one oracle and `port` in the other from this
    /// state; compares the results, every byte written outside the stacks
    /// and the stack above the `args` argument words (the caller's frame).
    ///
    /// # Errors
    /// A message saying what differs, or why the original did not run.
    pub fn compare<R: PortResult>(
        &self,
        args: u32,
        port: impl FnOnce(&mut Oracle, &CallState) -> R,
    ) -> std::result::Result<(), String> {
        let (mut orig, mut ours) = self.oracles().map_err(|e| format!("setup: {e}"))?;
        let want = orig
            .run_state(&self.state)
            .map_err(|e| format!("original: {e}"))?;
        let got = port(&mut ours, &self.state);
        if want.ret.st0.is_some() && !got.is_float() {
            return Err(format!(
                "the original returns {:?} in ST0, the port no float",
                want.ret.st0
            ));
        }
        got.compare(&want)?;
        let differ = orig.diff(&mut ours).map_err(|e| e.to_string())?;
        if !differ.is_empty() {
            let show: Vec<String> = differ
                .iter()
                .take(16)
                .map(|(at, x, y)| format!("{at:#010x}: {x:02x} vs {y:02x}"))
                .collect();
            return Err(format!(
                "{} bytes differ (original vs port): {}",
                differ.len(),
                show.join(", ")
            ));
        }
        let esp = self.state.regs.esp;
        let from = 4 + 4 * args;
        let len = u32::try_from(self.state.stack.len()).expect("small");
        if from < len {
            let theirs = orig
                .read(esp + from, (len - from) as usize)
                .map_err(|e| e.to_string())?;
            let mine = ours
                .read(esp + from, (len - from) as usize)
                .map_err(|e| e.to_string())?;
            if let Some(i) = theirs.iter().zip(&mine).position(|(u, v)| u != v) {
                return Err(format!(
                    "caller's stack differs at esp+{:#x}: {:02x} vs {:02x}",
                    from as usize + i,
                    theirs[i],
                    mine[i]
                ));
            }
        }
        Ok(())
    }

    /// [`Case::compare`], panicking with the case name on a difference.
    pub fn check<R: PortResult>(&self, args: u32, port: impl FnOnce(&mut Oracle, &CallState) -> R) {
        if let Err(e) = self.compare(args, port) {
            panic!("{}: {e}", self.name);
        }
    }
}

/// Checks a port against every harvested state of `addr` (see
/// [`harvest_test!`](crate::harvest_test)); skips cleanly when there are none.
pub fn run_harvested<R: PortResult>(
    addr: u32,
    args: u32,
    port: impl Fn(&mut Oracle, &CallState) -> R,
) {
    let Some(cases) = harvested(addr) else { return };
    let mut failures = Vec::new();
    for c in &cases {
        if let Err(e) = c.compare(args, &port) {
            failures.push(format!("{}: {e}", c.name));
        }
    }
    eprintln!(
        "{addr:#010x}: {} harvested states, {} failed",
        cases.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A parity test over every harvested state of one function:
///
/// ```ignore
/// harvest_test!(rnd_harvested, 0x0079_89a0, args = 0, |m, _| bridge::rnd(m));
/// harvest_test!(brnd_harvested, 0x0079_88f0, args = 1, |m, s| bridge::brnd(m, s.arg(0)));
/// ```
///
/// `args` is the number of stack argument words (the caller's frame above
/// them is compared too). The closure gets the port's oracle (the state
/// already applied) and the [`CallState`](crate::harvest::CallState); it
/// returns `()`, `u32`, `i32`, `bool`, `u64` (`EDX:EAX`), `f32`/`f64`
/// (`ST0`), [`Masked`](crate::harvest::Masked) or
/// [`Unchecked`](crate::harvest::Unchecked). The test skips (passes with a
/// message) without `FFX_GAME_DIR` or harvested states, unless
/// `FFX_HARVEST_REQUIRE` is set; `FFX_HARVEST_LIMIT` caps the states per
/// snapshot.
#[macro_export]
macro_rules! harvest_test {
    ($name:ident, $addr:expr, args = $args:expr, $port:expr $(,)?) => {
        #[test]
        fn $name() {
            $crate::harvest::run_harvested($addr, $args, $port);
        }
    };
}
