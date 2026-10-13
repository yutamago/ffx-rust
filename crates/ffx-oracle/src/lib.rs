//! Runs original `FFX.exe` functions in an x86 emulator (Unicorn), so Rust
//! reimplementations can be checked against the real machine code without
//! launching the game.
//!
//! The image is mapped at its preferred base (`0x400000`), so no relocation
//! is needed. Imports are bound to stubs: a few CRT functions (`malloc`,
//! `free`, `memcpy`, ...) are implemented natively, and calling any other
//! import stops the emulation with [`Error::UnstubbedImport`].

use std::collections::HashMap;
use std::path::Path;

use goblin::pe::PE;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

const PAGE: u64 = 0x1000;
const STACK_BASE: u64 = 0x1000_0000;
const STACK_SIZE: u64 = 0x10_0000;
const HEAP_BASE: u64 = 0x2000_0000;
const HEAP_SIZE: u64 = 0x400_0000;
const STUB_BASE: u64 = 0x3000_0000;
/// Return address pushed for the outermost call; reaching it ends the run.
const RETURN_SENTINEL: u64 = STUB_BASE + 0x00ff_fff0;
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
    #[error("{0} not preserved across the call (cdecl requires it)")]
    Clobbered(&'static str),
    #[error("the heap is exhausted")]
    OutOfMemory,
}

impl From<unicorn_engine::uc_error> for Error {
    fn from(e: unicorn_engine::uc_error) -> Self {
        Self::Emulator(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

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

#[derive(Default)]
struct State {
    stubs: HashMap<u64, Stub>,
    missing: Vec<String>,
    /// Bump allocator: next free heap address.
    heap_next: u64,
    fault: Option<Error>,
}

/// An emulated `FFX.exe` process image.
pub struct Oracle {
    uc: Unicorn<'static, State>,
}

impl Oracle {
    /// Loads `FFX.exe` (or any PE32 image) into a fresh emulator.
    ///
    /// # Errors
    /// Fails if the file cannot be read or parsed, or the emulator refuses a
    /// mapping.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        let pe = PE::parse(&bytes)?;
        let base = pe.image_base;
        let opt = pe
            .header
            .optional_header
            .ok_or(goblin::error::Error::Malformed("no optional header".into()))?;
        let image_size = u64::from(opt.windows_fields.size_of_image).next_multiple_of(PAGE);

        let mut uc = Unicorn::new_with_data(Arch::X86, Mode::MODE_32, State::default())?;
        uc.mem_map(base, image_size, Prot::ALL)?;
        let headers = usize::try_from(opt.windows_fields.size_of_headers)
            .unwrap_or(0)
            .min(bytes.len());
        uc.mem_write(base, &bytes[..headers])?;
        for section in &pe.sections {
            let raw = section.pointer_to_raw_data as usize;
            let len = (section.size_of_raw_data as usize).min(bytes.len().saturating_sub(raw));
            uc.mem_write(
                base + u64::from(section.virtual_address),
                &bytes[raw..raw + len],
            )?;
        }
        uc.mem_map(STACK_BASE, STACK_SIZE, Prot::READ | Prot::WRITE)?;
        uc.mem_map(HEAP_BASE, HEAP_SIZE, Prot::READ | Prot::WRITE)?;
        uc.mem_map(STUB_BASE, 0x100_0000, Prot::ALL)?;
        uc.get_data_mut().heap_next = HEAP_BASE;

        // Bind every import to its own stub address, filled with `ret` so a
        // stray execution cannot fall through.
        let mut next_stub = STUB_BASE;
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
                base + import.offset as u64,
                &u32::try_from(next_stub).expect("32-bit").to_le_bytes(),
            )?;
            uc.get_data_mut().stubs.insert(next_stub, stub);
            next_stub += 0x10;
        }
        uc.mem_write(RETURN_SENTINEL, &[0xc3])?;

        uc.add_code_hook(STUB_BASE, STUB_BASE + 0xff_ffff, |uc, addr, _| {
            if addr == RETURN_SENTINEL {
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
            |uc, _kind: MemType, addr, _, _| {
                uc.get_data_mut().fault = Some(Error::BadAccess(addr));
                false
            },
        )?;
        Ok(Self { uc })
    }

    /// Frees every emulated allocation at once. Call between independent
    /// cases; earlier pointers become invalid.
    pub fn reset_heap(&mut self) {
        self.uc.get_data_mut().heap_next = HEAP_BASE;
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

    /// Reads emulated memory.
    ///
    /// # Errors
    /// Fails if the range is not mapped.
    pub fn read(&self, addr: u32, len: usize) -> Result<Vec<u8>> {
        Ok(self.uc.mem_read_as_vec(u64::from(addr), len)?)
    }

    /// Writes emulated memory.
    ///
    /// # Errors
    /// Fails if the range is not mapped.
    pub fn write(&mut self, addr: u32, bytes: &[u8]) -> Result<()> {
        Ok(self.uc.mem_write(u64::from(addr), bytes)?)
    }

    /// Calls a cdecl function at its `FFX.exe` address and returns EAX.
    /// Checks that the callee-saved registers survive, as cdecl requires.
    ///
    /// # Errors
    /// Fails on emulator faults, invalid memory accesses, calls to unstubbed
    /// imports, or clobbered callee-saved registers.
    pub fn call_cdecl(&mut self, addr: u32, args: &[u32]) -> Result<u32> {
        const SAVED: [(RegisterX86, &str, u64); 4] = [
            (RegisterX86::EBX, "EBX", 0x1111_1111),
            (RegisterX86::ESI, "ESI", 0x2222_2222),
            (RegisterX86::EDI, "EDI", 0x3333_3333),
            (RegisterX86::EBP, "EBP", 0x4444_4444),
        ];
        let mut sp = STACK_BASE + STACK_SIZE - 0x100;
        for &arg in args.iter().rev() {
            sp -= 4;
            self.uc.mem_write(sp, &arg.to_le_bytes())?;
        }
        sp -= 4;
        self.uc.mem_write(
            sp,
            &u32::try_from(RETURN_SENTINEL)
                .expect("32-bit")
                .to_le_bytes(),
        )?;
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
        if self.uc.reg_read(RegisterX86::ESP)? != sp + 4 {
            return Err(Error::Clobbered("ESP"));
        }
        Ok(u32::try_from(self.uc.reg_read(RegisterX86::EAX)?).expect("32-bit register"))
    }
}

fn alloc(uc: &mut Unicorn<State>, len: u64) -> Result<u32> {
    let state = uc.get_data_mut();
    let addr = state.heap_next;
    let end = addr + len.max(1).next_multiple_of(16);
    if end > HEAP_BASE + HEAP_SIZE {
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
