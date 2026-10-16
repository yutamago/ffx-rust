//! Runs original `FFX.exe` functions in an x86 emulator (Unicorn), so Rust
//! reimplementations can be checked against the real machine code without
//! launching the game.
//!
//! Two ways to start:
//! - [`Oracle::load`]: the image alone at its preferred base (`0x400000`),
//!   for functions whose inputs the test builds itself.
//! - [`Oracle::from_snapshot`]: the image relocated to where the game had
//!   it, its `.data` and every other region taken from a snapshot of the
//!   running game (`snapshot` harness command, [`ffx_bridge::snapshot`]).
//!   Pages are mapped when first touched. The Rust side of a comparison
//!   runs against a second oracle on the same snapshot through
//!   [`ffx_bridge::Mem`]; [`Oracle::diff`] then compares what both wrote.
//!
//! Imports are bound to stubs: a few CRT functions (`malloc`, `free`,
//! `memcpy`, ...) are implemented natively, and calling any other import
//! stops the emulation with [`Error::UnstubbedImport`].

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use ffx_bridge::snapshot::{BLOCK, Snapshot};
use goblin::pe::PE;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

const PAGE: u64 = 0x1000;
const PAGE_BYTES: usize = 0x1000;
const STACK_SIZE: u64 = 0x10_0000;
const HEAP_SIZE: u64 = 0x400_0000;
const STUB_SIZE: u64 = 0x100_0000;
/// Upper bound on instructions per call, against runaway loops.
const MAX_INSTRUCTIONS: usize = 200_000_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot parse the PE image: {0}")]
    Pe(#[from] goblin::error::Error),
    #[error("emulator error: {0:?}")]
    Emulator(unicorn_engine::uc_error),
    #[error("the emulated code called import {0}, which has no stub")]
    UnstubbedImport(String),
    #[error("invalid memory access at {0:#010x}")]
    BadAccess(u64),
    #[error("{0} not preserved across the call (the calling convention requires it)")]
    Clobbered(&'static str),
    #[error("the heap is exhausted")]
    OutOfMemory,
    #[error("no free range of {0:#x} bytes for the oracle's own memory")]
    NoRoom(u64),
}

impl From<unicorn_engine::uc_error> for Error {
    fn from(e: unicorn_engine::uc_error) -> Self {
        Self::Emulator(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// How a function takes its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// Arguments on the stack, the caller pops them.
    Cdecl,
    /// Arguments on the stack, the callee pops them.
    Stdcall,
    /// `this` in ECX, the rest on the stack, the callee pops them.
    Thiscall(u32),
    /// The first two in ECX and EDX, the rest on the stack, the callee pops them.
    Fastcall,
}

/// What a call left in the result registers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ret {
    pub eax: u32,
    pub edx: u32,
    /// `ST0` when the x87 stack is not empty (float and double results).
    pub st0: Option<f64>,
    /// `ST0`'s 80 bits (mantissa, then sign and exponent), to tell NaNs
    /// apart: [`Ret::st0`] makes every NaN the same.
    pub st0_raw: Option<[u8; 10]>,
}

#[derive(Clone, Copy)]
enum Stub {
    Malloc,
    Calloc,
    Free,
    Memcpy,
    Memmove,
    Memset,
    Missing(usize),
}

/// A snapshot whose blocks are unpacked on first use; shared by every
/// oracle made from it.
pub struct Pages {
    snapshot: Snapshot,
    unpacked: Vec<Vec<OnceLock<Vec<u8>>>>,
}

impl Pages {
    /// Wraps a snapshot.
    #[must_use]
    pub fn new(snapshot: Snapshot) -> Arc<Self> {
        let unpacked = snapshot
            .regions
            .iter()
            .map(|r| r.blocks.iter().map(|_| OnceLock::new()).collect())
            .collect();
        Arc::new(Self { snapshot, unpacked })
    }

    /// Reads a snapshot file.
    ///
    /// # Errors
    /// On I/O errors or a malformed file.
    pub fn read(path: impl AsRef<Path>) -> Result<Arc<Self>> {
        let file = std::fs::File::open(path)?;
        Ok(Self::new(Snapshot::read(std::io::BufReader::new(file))?))
    }

    /// The snapshot.
    #[must_use]
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// The snapshot block holding `addr`: its first address and bytes.
    fn chunk(&self, addr: u64) -> Option<(u64, &[u8])> {
        let a = u32::try_from(addr).ok()?;
        let i = self
            .snapshot
            .regions
            .partition_point(|r| r.base <= a)
            .checked_sub(1)?;
        let r = &self.snapshot.regions[i];
        if addr >= u64::from(r.base) + u64::from(r.size) {
            return None;
        }
        let k = usize::try_from((addr - u64::from(r.base)) / BLOCK as u64).ok()?;
        let packed = r.blocks.get(k)?;
        let data = self.unpacked[i][k].get_or_init(|| lz4_flex_unpack(packed));
        Some((u64::from(r.base) + (k * BLOCK) as u64, data.as_slice()))
    }
}

fn lz4_flex_unpack(block: &[u8]) -> Vec<u8> {
    ffx_bridge::snapshot::unpack(block).unwrap_or_default()
}

#[derive(Default)]
struct State {
    stubs: HashMap<u64, Stub>,
    missing: Vec<String>,
    heap_base: u64,
    heap_next: u64,
    stack_base: u64,
    sentinel: u64,
    fault: Option<Error>,
    pages: Option<Arc<Pages>>,
    /// Pages written since the last [`Oracle::clear_dirty`].
    dirty: BTreeSet<u32>,
    image_base: u32,
}

/// An emulated `FFX.exe` process.
pub struct Oracle {
    uc: Unicorn<'static, State>,
}

/// The x87 control word of FFX.exe's game thread (`fpucw`, docs/harness.md):
/// 53-bit precision, round to nearest, all exceptions masked.
pub const GAME_FPU_CONTROL: u32 = 0x027f;

impl Oracle {
    /// Loads `FFX.exe` (or any PE32 image) into a fresh emulator at its
    /// preferred base, the x87 set as on the game thread
    /// ([`GAME_FPU_CONTROL`]).
    ///
    /// # Errors
    /// Fails if the file cannot be read or parsed, or the emulator refuses a
    /// mapping.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::build(&bytes, None, None)
    }

    /// Loads `FFX.exe` where the snapshot's game had it, with the
    /// snapshot's memory behind it.
    ///
    /// # Errors
    /// As [`Oracle::load`]; also when the snapshot leaves no room for the
    /// oracle's own stack, heap and stubs.
    pub fn from_snapshot(exe: impl AsRef<Path>, pages: &Arc<Pages>) -> Result<Self> {
        let bytes = std::fs::read(exe)?;
        Self::build(&bytes, Some(pages.snapshot.image_base), Some(pages.clone()))
    }

    #[allow(clippy::too_many_lines)] // one setup, in order
    fn build(bytes: &[u8], at: Option<u32>, pages: Option<Arc<Pages>>) -> Result<Self> {
        let pe = PE::parse(bytes)?;
        let preferred = u32::try_from(pe.image_base)
            .map_err(|_| goblin::error::Error::Malformed("base".into()))?;
        let base = at.unwrap_or(preferred);
        let opt = pe
            .header
            .optional_header
            .ok_or(goblin::error::Error::Malformed("no optional header".into()))?;
        let image_size = u64::from(opt.windows_fields.size_of_image).next_multiple_of(PAGE);

        let mut image = vec![0u8; usize::try_from(image_size).expect("image size")];
        let headers = usize::try_from(opt.windows_fields.size_of_headers)
            .unwrap_or(0)
            .min(bytes.len());
        image[..headers].copy_from_slice(&bytes[..headers]);
        for section in &pe.sections {
            let raw = section.pointer_to_raw_data as usize;
            let len = (section.size_of_raw_data as usize).min(bytes.len().saturating_sub(raw));
            let va = section.virtual_address as usize;
            let len = len.min(image.len().saturating_sub(va));
            image[va..va + len].copy_from_slice(&bytes[raw..raw + len]);
        }
        if base != preferred {
            relocate(&mut image, &pe, base.wrapping_sub(preferred));
        }
        // The game's globals as they were: writable sections from the snapshot.
        if let Some(p) = &pages {
            for section in &pe.sections {
                if section.characteristics & 0x8000_0000 == 0 {
                    continue;
                }
                let va = u64::from(section.virtual_address);
                let len = u64::from(section.virtual_size.max(section.size_of_raw_data))
                    .next_multiple_of(PAGE);
                let mut a = u64::from(base) + va;
                let end = (a + len).min(u64::from(base) + image_size);
                while a < end {
                    if let Some((start, data)) = p.chunk(a) {
                        let off = usize::try_from(start - u64::from(base)).expect("in image");
                        let n = data.len().min(image.len() - off);
                        image[off..off + n].copy_from_slice(&data[..n]);
                        a = start + data.len() as u64;
                    } else {
                        a = (a + PAGE) & !(PAGE - 1);
                    }
                }
            }
        }

        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, State::default())?;
        uc.mem_map(u64::from(base), image_size, Prot::ALL)?;
        uc.mem_write(u64::from(base), &image)?;

        // The oracle's own memory, where the snapshot has nothing.
        let mut taken: Vec<(u64, u64)> = vec![(u64::from(base), u64::from(base) + image_size)];
        if let Some(p) = &pages {
            for r in &p.snapshot.regions {
                taken.push((u64::from(r.base), u64::from(r.base) + u64::from(r.size)));
            }
        }
        let stack_base = gap(&mut taken, STACK_SIZE)?;
        let heap_base = gap(&mut taken, HEAP_SIZE)?;
        let stub_base = gap(&mut taken, STUB_SIZE)?;
        uc.mem_map(stack_base, STACK_SIZE, Prot::READ | Prot::WRITE)?;
        uc.mem_map(heap_base, HEAP_SIZE, Prot::READ | Prot::WRITE)?;
        uc.mem_map(stub_base, STUB_SIZE, Prot::ALL)?;
        {
            let s = uc.get_data_mut();
            s.heap_base = heap_base;
            s.heap_next = heap_base;
            s.stack_base = stack_base;
            s.sentinel = stub_base + STUB_SIZE - 0x10;
            s.image_base = base;
            s.pages = pages;
        }

        // Bind every import to its own stub address, filled with `ret` so a
        // stray execution cannot fall through.
        let mut next_stub = stub_base;
        for import in &pe.imports {
            let stub = match import.name.as_ref() {
                "malloc" => Stub::Malloc,
                "calloc" => Stub::Calloc,
                "free" => Stub::Free,
                "memcpy" => Stub::Memcpy,
                "memmove" => Stub::Memmove,
                "memset" => Stub::Memset,
                other => {
                    let state = uc.get_data_mut();
                    state.missing.push(format!("{}!{other}", import.dll));
                    Stub::Missing(state.missing.len() - 1)
                }
            };
            uc.mem_write(next_stub, &[0xc3])?;
            uc.mem_write(
                u64::from(base) + import.offset as u64,
                &u32::try_from(next_stub).expect("32-bit").to_le_bytes(),
            )?;
            uc.get_data_mut().stubs.insert(next_stub, stub);
            next_stub += 0x10;
        }
        let sentinel = uc.get_data().sentinel;
        uc.mem_write(sentinel, &[0xc3])?;

        uc.add_code_hook(stub_base, stub_base + STUB_SIZE - 1, |uc, addr, _| {
            if addr == uc.get_data().sentinel {
                let _ = uc.emu_stop();
                return;
            }
            if let Some(stub) = uc.get_data().stubs.get(&addr).copied()
                && let Err(e) = run_stub(uc, stub)
            {
                uc.get_data_mut().fault = Some(e);
                let _ = uc.emu_stop();
            }
        })?;
        uc.add_mem_hook(
            HookType::MEM_INVALID,
            0,
            u64::MAX,
            |uc, kind: MemType, addr, _, _| {
                let unmapped = matches!(
                    kind,
                    MemType::READ_UNMAPPED | MemType::WRITE_UNMAPPED | MemType::FETCH_UNMAPPED
                );
                if unmapped && map_from_snapshot(uc, addr) {
                    return true;
                }
                uc.get_data_mut().fault = Some(Error::BadAccess(addr));
                false
            },
        )?;
        uc.add_mem_hook(
            HookType::MEM_WRITE,
            0,
            u64::MAX,
            |uc, _kind, addr, size, _| {
                let first = u32::try_from(addr >> 12).unwrap_or(u32::MAX);
                let last =
                    u32::try_from((addr + size as u64).saturating_sub(1) >> 12).unwrap_or(u32::MAX);
                let d = &mut uc.get_data_mut().dirty;
                for p in first..=last {
                    d.insert(p);
                }
                true
            },
        )?;
        let mut o = Self { uc };
        // The game thread's x87 control word (53-bit precision, docs/porting.md);
        // the emulator starts at 64-bit, where chained float code rounds
        // differently. A snapshot carries the one it was taken with.
        let cw =
            o.uc.get_data()
                .pages
                .as_ref()
                .map_or(GAME_FPU_CONTROL, |p| p.snapshot.fpu_control);
        o.set_fpu_control(cw)?;
        Ok(o)
    }

    /// Where the image is loaded.
    #[must_use]
    pub fn image_base(&self) -> u32 {
        self.uc.get_data().image_base
    }

    /// An `ffx.sym` address (preferred base) where the image really is.
    #[must_use]
    pub fn rebase(&self, sym: u32) -> u32 {
        sym.wrapping_sub(0x0040_0000)
            .wrapping_add(self.image_base())
    }

    /// Makes the function at `addr` (rebased) return `value` at once
    /// (`mov eax, value; ret`), for callees a test wants fixed (clocks).
    ///
    /// # Errors
    /// On emulator errors.
    pub fn stub_return(&mut self, addr: u32, value: u32) -> Result<()> {
        let mut code = vec![0xb8];
        code.extend_from_slice(&value.to_le_bytes());
        code.push(0xc3);
        self.uc.mem_write(u64::from(addr), &code)?;
        // Unicorn keeps translated code; drop what it had for these bytes.
        self.uc
            .ctl_remove_cache(u64::from(addr), u64::from(addr) + code.len() as u64)?;
        Ok(())
    }

    /// Sets the x87 control word (precision and rounding) for later calls.
    ///
    /// # Errors
    /// On emulator errors.
    pub fn set_fpu_control(&mut self, cw: u32) -> Result<()> {
        self.uc.reg_write(RegisterX86::FPCW, u64::from(cw))?;
        Ok(())
    }

    /// Frees every emulated allocation at once. Call between independent
    /// cases; earlier pointers become invalid.
    pub fn reset_heap(&mut self) {
        let s = self.uc.get_data_mut();
        s.heap_next = s.heap_base;
    }

    /// Allocates `len` zeroed bytes in emulated memory.
    ///
    /// # Errors
    /// Fails when the emulated heap is exhausted.
    pub fn alloc(&mut self, len: usize) -> Result<u32> {
        alloc(&mut self.uc, len as u64)
    }

    /// Copies `bytes` into newly allocated emulated memory.
    ///
    /// # Errors
    /// Fails when the emulated heap is exhausted.
    pub fn alloc_bytes(&mut self, bytes: &[u8]) -> Result<u32> {
        let addr = self.alloc(bytes.len())?;
        self.uc.mem_write(u64::from(addr), bytes)?;
        Ok(addr)
    }

    /// Maps the snapshot's pages for a range before the host touches it.
    fn touch(&mut self, addr: u32, len: usize) {
        let mut a = u64::from(addr) & !(PAGE - 1);
        let end = u64::from(addr) + len as u64;
        while a < end {
            if self.uc.mem_read_as_vec(a, 1).is_err() {
                map_from_snapshot(&mut self.uc, a);
            }
            a += PAGE;
        }
    }

    /// Reads emulated memory.
    ///
    /// # Errors
    /// Fails if the range is not mapped (nor in the snapshot).
    pub fn read(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        self.touch(addr, len);
        Ok(self.uc.mem_read_as_vec(u64::from(addr), len)?)
    }

    /// Writes emulated memory (and marks it dirty).
    ///
    /// # Errors
    /// Fails if the range is not mapped (nor in the snapshot).
    pub fn write(&mut self, addr: u32, bytes: &[u8]) -> Result<()> {
        self.touch(addr, bytes.len());
        self.uc.mem_write(u64::from(addr), bytes)?;
        let base = self.uc.get_data().image_base;
        if addr >= base && addr < base.saturating_add(0x0080_0000) && !bytes.is_empty() {
            // into the code: drop translations of it
            self.uc
                .ctl_remove_cache(u64::from(addr), u64::from(addr) + bytes.len() as u64)?;
        }
        if !bytes.is_empty() {
            let d = &mut self.uc.get_data_mut().dirty;
            for p in
                (addr >> 12)..=((addr + u32::try_from(bytes.len()).unwrap_or(u32::MAX) - 1) >> 12)
            {
                d.insert(p);
            }
        }
        Ok(())
    }

    /// Pages written since the last [`Oracle::clear_dirty`] (by emulated
    /// code or by [`Oracle::write`]), as page numbers (`address >> 12`).
    #[must_use]
    pub fn dirty_pages(&self) -> &BTreeSet<u32> {
        &self.uc.get_data().dirty
    }

    /// Forgets the written pages.
    pub fn clear_dirty(&mut self) {
        self.uc.get_data_mut().dirty.clear();
    }

    /// The oracle's own stack, which differs between runs by construction.
    #[must_use]
    pub fn stack_range(&self) -> std::ops::Range<u32> {
        let b = u32::try_from(self.uc.get_data().stack_base).expect("32-bit");
        b..b + u32::try_from(STACK_SIZE).expect("32-bit")
    }

    /// Bytes that differ between two oracles over the pages either wrote,
    /// outside both stacks: `(address, ours, theirs)`.
    ///
    /// # Errors
    /// On emulator errors.
    pub fn diff(&mut self, other: &mut Self) -> Result<Vec<(u32, u8, u8)>> {
        let pages: BTreeSet<u32> = self
            .dirty_pages()
            .union(other.dirty_pages())
            .copied()
            .collect();
        let (s1, s2) = (self.stack_range(), other.stack_range());
        let mut out = Vec::new();
        for p in pages {
            let a = p << 12;
            if s1.contains(&a) || s2.contains(&a) {
                continue;
            }
            let x = self.read(a, PAGE_BYTES)?;
            let y = other.read(a, PAGE_BYTES)?;
            for (i, (u, v)) in x.iter().zip(&y).enumerate() {
                if u != v {
                    out.push((a + u32::try_from(i).expect("page offset"), *u, *v));
                }
            }
        }
        Ok(out)
    }

    /// Calls a cdecl function at its address in the emulator (already
    /// rebased) and returns EAX. Checks that the callee-saved registers
    /// survive.
    ///
    /// # Errors
    /// Fails on emulator faults, invalid memory accesses, calls to unstubbed
    /// imports, or clobbered callee-saved registers.
    pub fn call_cdecl(&mut self, addr: u32, args: &[u32]) -> Result<u32> {
        Ok(self.call(addr, Conv::Cdecl, args)?.eax)
    }

    /// Calls a function at its address in the emulator (already rebased).
    ///
    /// # Errors
    /// As [`Oracle::call_cdecl`]; also when a callee-pops convention leaves
    /// the stack unbalanced.
    pub fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> Result<Ret> {
        const SAVED: [(RegisterX86, &str, u64); 4] = [
            (RegisterX86::EBX, "EBX", 0x1111_1111),
            (RegisterX86::ESI, "ESI", 0x2222_2222),
            (RegisterX86::EDI, "EDI", 0x3333_3333),
            (RegisterX86::EBP, "EBP", 0x4444_4444),
        ];
        let (stack_base, sentinel) = {
            let s = self.uc.get_data();
            (s.stack_base, s.sentinel)
        };
        let stack_args: &[u32] = match conv {
            Conv::Fastcall => {
                self.uc.reg_write(
                    RegisterX86::ECX,
                    u64::from(args.first().copied().unwrap_or(0)),
                )?;
                self.uc.reg_write(
                    RegisterX86::EDX,
                    u64::from(args.get(1).copied().unwrap_or(0)),
                )?;
                args.get(2..).unwrap_or(&[])
            }
            Conv::Thiscall(this) => {
                self.uc.reg_write(RegisterX86::ECX, u64::from(this))?;
                args
            }
            Conv::Cdecl | Conv::Stdcall => args,
        };
        let mut sp = stack_base + STACK_SIZE - 0x100;
        for &arg in stack_args.iter().rev() {
            sp -= 4;
            self.uc.mem_write(sp, &arg.to_le_bytes())?;
        }
        sp -= 4;
        self.uc
            .mem_write(sp, &u32::try_from(sentinel).expect("32-bit").to_le_bytes())?;
        self.uc.reg_write(RegisterX86::ESP, sp)?;
        for (reg, _, value) in SAVED {
            self.uc.reg_write(reg, value)?;
        }
        self.uc.get_data_mut().fault = None;

        let run = self.uc.emu_start(u64::from(addr), 0, 0, MAX_INSTRUCTIONS);
        if let Some(fault) = self.uc.get_data_mut().fault.take() {
            return Err(fault);
        }
        run?;
        for (reg, name, value) in SAVED {
            if self.uc.reg_read(reg)? != value {
                return Err(Error::Clobbered(name));
            }
        }
        let popped = match conv {
            Conv::Cdecl => 0,
            _ => 4 * stack_args.len() as u64,
        };
        if self.uc.reg_read(RegisterX86::ESP)? != sp + 4 + popped {
            return Err(Error::Clobbered("ESP"));
        }
        let eax = u32::try_from(self.uc.reg_read(RegisterX86::EAX)?).expect("32-bit register");
        let edx = u32::try_from(self.uc.reg_read(RegisterX86::EDX)?).expect("32-bit register");
        let st0_raw = self.st0_raw()?;
        Ok(Ret {
            eax,
            edx,
            st0: st0_raw.map(|r| f80_to_f64(&r)),
            st0_raw,
        })
    }

    /// `ST0`'s bytes, if the x87 stack holds anything.
    fn st0_raw(&mut self) -> Result<Option<[u8; 10]>> {
        let tag = self.uc.reg_read(RegisterX86::FPTAG)?;
        let sw = self.uc.reg_read(RegisterX86::FPSW)?;
        let top = (sw >> 11) & 7;
        // Two tag bits per physical register; 3 = empty.
        if (tag >> (2 * top)) & 3 == 3 {
            return Ok(None);
        }
        let raw = self.uc.reg_read_long(RegisterX86::ST0)?;
        let mut out = [0u8; 10];
        out.copy_from_slice(&raw[..10]);
        Ok(Some(out))
    }
}

impl ffx_bridge::Mem for Oracle {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        // The trait reads through &self; mapping from the snapshot needs
        // &mut, so a page the emulator has not touched yet is mapped by a
        // throwaway write-free path: read what is mapped, else the snapshot.
        if let Ok(v) = self.uc.mem_read_as_vec(u64::from(addr), out.len()) {
            out.copy_from_slice(&v);
            return;
        }
        for (i, b) in out.iter_mut().enumerate() {
            let a = u64::from(addr) + i as u64;
            *b = self
                .uc
                .mem_read_as_vec(a, 1)
                .ok()
                .and_then(|v| v.first().copied())
                .or_else(|| {
                    let p = self.uc.get_data().pages.as_ref()?;
                    let (start, data) = p.chunk(a)?;
                    data.get(usize::try_from(a - start).ok()?).copied()
                })
                .unwrap_or_else(|| panic!("read of unmapped game memory at {a:#x}"));
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.write(addr, data)
            .unwrap_or_else(|e| panic!("write of {} bytes at {addr:#x}: {e}", data.len()));
    }
    fn image_base(&self) -> u32 {
        self.uc.get_data().image_base
    }
}

/// Applies the image's base relocations for a load `delta` away.
fn relocate(image: &mut [u8], pe: &PE, delta: u32) {
    let Some(dir) = pe
        .header
        .optional_header
        .and_then(|o| o.data_directories.get_base_relocation_table().copied())
    else {
        return;
    };
    let mut at = dir.virtual_address as usize;
    let end = at + dir.size as usize;
    let rd = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().expect("4 bytes"));
    while at + 8 <= end {
        let page = rd(image, at) as usize;
        let size = rd(image, at + 4) as usize;
        if size < 8 {
            break;
        }
        for k in 0..(size - 8) / 2 {
            let e = u16::from_le_bytes([image[at + 8 + 2 * k], image[at + 9 + 2 * k]]);
            if e >> 12 == 3 {
                let p = page + usize::from(e & 0xfff);
                let v = rd(image, p).wrapping_add(delta);
                image[p..p + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        at += size;
    }
}

/// The lowest free range of `size` bytes from `0x1000_0000` (or else from
/// `0x10000`) that overlaps nothing in `taken`; marks it taken.
fn gap(taken: &mut Vec<(u64, u64)>, size: u64) -> Result<u64> {
    taken.sort_unstable();
    for start_at in [0x1000_0000u64, 0x1_0000] {
        let mut cand = start_at;
        for &(s, e) in taken.iter() {
            if e <= cand {
                continue;
            }
            if s >= cand + size {
                break;
            }
            cand = e.next_multiple_of(0x1_0000);
        }
        if cand + size <= 0xffff_0000 {
            taken.push((cand, cand + size));
            return Ok(cand);
        }
    }
    Err(Error::NoRoom(size))
}

/// Maps the snapshot chunk holding `addr`; false when the snapshot has none.
fn map_from_snapshot(uc: &mut Unicorn<State>, addr: u64) -> bool {
    let Some(pages) = uc.get_data().pages.clone() else {
        return false;
    };
    let Some((start, data)) = pages.chunk(addr) else {
        return false;
    };
    let len = (data.len() as u64).next_multiple_of(PAGE);
    if uc.mem_map(start, len, Prot::ALL).is_err() {
        // Partly mapped already: map page by page.
        let page = addr & !(PAGE - 1);
        if uc.mem_map(page, PAGE, Prot::ALL).is_err() {
            return false;
        }
        let off = usize::try_from(page - start).expect("in chunk");
        let n = PAGE_BYTES.min(data.len().saturating_sub(off));
        return uc.mem_write(page, &data[off..off + n]).is_ok();
    }
    uc.mem_write(start, data).is_ok()
}

/// An x87 80-bit extended value (little-endian bytes) as f64.
fn f80_to_f64(b: &[u8]) -> f64 {
    if b.len() < 10 {
        return f64::NAN;
    }
    let mant = u64::from_le_bytes(b[..8].try_into().expect("8 bytes"));
    let se = u16::from_le_bytes([b[8], b[9]]);
    let sign = if se & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from(se & 0x7fff);
    if exp == 0 && mant == 0 {
        return 0.0 * sign;
    }
    if exp == 0x7fff {
        return if mant << 1 == 0 {
            sign * f64::INFINITY
        } else {
            f64::NAN
        };
    }
    #[allow(clippy::cast_precision_loss)] // f64 holds what the game rounds to
    let m = mant as f64 / (1u64 << 63) as f64;
    sign * m * 2f64.powi(exp - 16383)
}

fn alloc(uc: &mut Unicorn<State>, len: u64) -> Result<u32> {
    let state = uc.get_data_mut();
    let addr = state.heap_next;
    let end = addr + len.max(1).next_multiple_of(16);
    if end > state.heap_base + HEAP_SIZE {
        return Err(Error::OutOfMemory);
    }
    state.heap_next = end;
    uc.mem_write(
        addr,
        &vec![0; usize::try_from(end - addr).expect("heap size")],
    )?;
    Ok(u32::try_from(addr).expect("32-bit heap"))
}

/// Implements one import natively. Stubs are entered by `call`, so the
/// return address is at `[esp]` and the arguments follow it; the stub's own
/// `ret` instruction then returns to the caller.
fn run_stub(uc: &mut Unicorn<State>, stub: Stub) -> Result<()> {
    let arg = |uc: &Unicorn<State>, i: u64| -> Result<u32> {
        let esp = uc.reg_read(RegisterX86::ESP)?;
        let b = uc.mem_read_as_vec(esp + 4 + 4 * i, 4)?;
        Ok(u32::from_le_bytes(b.try_into().expect("4 bytes")))
    };
    let ret = match stub {
        Stub::Malloc => alloc(uc, u64::from(arg(uc, 0)?))?,
        Stub::Calloc => alloc(uc, u64::from(arg(uc, 0)?) * u64::from(arg(uc, 1)?))?,
        Stub::Free => 0,
        Stub::Memcpy | Stub::Memmove => {
            let (dst, src, len) = (arg(uc, 0)?, arg(uc, 1)?, arg(uc, 2)?);
            let data = uc.mem_read_as_vec(u64::from(src), len as usize)?;
            uc.mem_write(u64::from(dst), &data)?;
            dst
        }
        Stub::Memset => {
            let (dst, value, len) = (arg(uc, 0)?, arg(uc, 1)?, arg(uc, 2)?);
            #[allow(clippy::cast_possible_truncation)] // memset stores the low byte
            uc.mem_write(u64::from(dst), &vec![value as u8; len as usize])?;
            dst
        }
        Stub::Missing(i) => return Err(Error::UnstubbedImport(uc.get_data().missing[i].clone())),
    };
    uc.reg_write(RegisterX86::EAX, u64::from(ret))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f80_values() {
        // 1.0: exponent 0x3fff, mantissa with the explicit integer bit.
        let one = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x3f];
        assert!((f80_to_f64(&one) - 1.0).abs() < f64::EPSILON);
        let minus_half = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xfe, 0xbf];
        assert!((f80_to_f64(&minus_half) + 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn gaps_avoid_taken_ranges() {
        let mut t = vec![(0x1000_0000, 0x1100_0000), (0x1180_0000, 0x1200_0000)];
        assert_eq!(gap(&mut t, 0x80_0000).unwrap(), 0x1100_0000);
        assert_eq!(gap(&mut t, 0x10_0000).unwrap(), 0x1200_0000);
    }
}
