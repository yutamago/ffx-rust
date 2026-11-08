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
//! The C runtime is the game's own: `MSVCR110.dll` from beside `FFX.exe`
//! is mapped next to it, relocated, its `KERNEL32` imports bound to
//! deterministic single-threaded stand-ins (`win32`), and initialised with
//! `DllMain(DLL_PROCESS_ATTACH)` once per oracle ([`Runtime::Dll`]).
//! `FFX.exe`'s MSVCR110 imports call its exports, except `malloc`,
//! `calloc`, `free`, `memcpy`, `memmove`, `memset` and `_wassert`, kept
//! native. Two emulator gaps are closed for it: the x87 transcendental instructions, which the
//! emulator computes through host `double`s, are redone as the FPU does
//! them (`x87`, trapped through `int3` patched over them), the CPU model
//! has no SSE4.2, whose string instructions the emulator gets wrong, and
//! MXCSR starts as on Windows (`0x1f80`), so the runtime takes its SSE2
//! math paths as it does in the game. [`Runtime::Native`] (or
//! `FFX_ORACLE_RUNTIME=native`) uses native stubs instead (`crt`). Calling
//! an import with neither stops the emulation with
//! [`Error::UnstubbedImport`]; [`Oracle::missing_imports`] lists those.
//! [`Oracle::bind_import`] points an import at a test's own code or back
//! at its native stub.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use ffx_bridge::snapshot::{BLOCK, Snapshot};
use goblin::pe::PE;
use unicorn_engine::{Arch, HookType, MemType, Mode, Prot, RegisterX86, Unicorn};

pub mod harvest;
pub use harvest::{CallState, Case, Driver, OracleHost, Store, harvested};
mod win32;

use ffx_fpu::x87;

const PAGE: u64 = 0x1000;
const PAGE_BYTES: usize = 0x1000;
const STACK_SIZE: u64 = 0x10_0000;
const HEAP_SIZE: u64 = 0x400_0000;
const STUB_SIZE: u64 = 0x100_0000;
/// `UC_CPU_X86_CORE2DUO`.
const CPU_CORE2DUO: i32 = 2;
/// The runtime DLL's own heap (`HeapAlloc`).
const CRT_HEAP_SIZE: u64 = 0x100_0000;
/// MSVCR110 imports that stay native even with the DLL loaded: the heap is
/// the oracle's either way, and the copies are faster natively (the
/// runtime's `memcpy` also allows overlap, as the native one does).
/// `_wassert` stays native too, so a failed assertion stops the call with
/// [`Error::Assertion`] instead of the runtime's abort path.
const KEPT_NATIVE: [&str; 7] = [
    "malloc", "calloc", "free", "memcpy", "memmove", "memset", "_wassert",
];
/// Upper bound on instructions per call, against runaway loops (the
/// default; [`Oracle::set_max_instructions`] changes it per oracle).
pub const MAX_INSTRUCTIONS: usize = 200_000_000;

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
    #[error("no return after {1} instructions (a runaway loop?), at {0:#010x}")]
    Runaway(u64, usize),
    #[error("the heap is exhausted")]
    OutOfMemory,
    #[error("no free range of {0:#x} bytes for the oracle's own memory")]
    NoRoom(u64),
    #[error("interrupt {0} at {1:#010x}")]
    Interrupt(u32, u64),
    #[error("MSVCR110.dll did not initialise: {0}")]
    RuntimeInit(String),
    #[error("assertion failed in the emulated code: {message} ({file}:{line})")]
    Assertion {
        message: String,
        file: String,
        line: u32,
    },
    #[error("a harvested state does not fit this oracle's memory layout: {0}")]
    LayoutMismatch(String),
    #[error("the C runtime gave up: it called {0} (an invalid parameter, abort() or an exception)")]
    RuntimeAbort(&'static str),
    #[error("the function at {0:#010x} left no float in ST0")]
    NoFloat(u32),
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
    /// The first two in ECX and EDX, the rest on the stack, the caller pops
    /// them: a custom convention of MSVC's link-time code generation
    /// (`0x00401690`, `Settings_ValidateBindings`; the hook's
    /// `fastcall-caller-pops` shims).
    FastcallCallerPops,
}

/// What a call left in the result registers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ret {
    pub eax: u32,
    pub edx: u32,
    /// `ST0` when the x87 stack is not empty (float and double results),
    /// as `fst qword` stores it ([`f80_to_f64`]: NaN signs and payloads
    /// kept, so compare it bit for bit, [`same_float`]).
    pub st0: Option<f64>,
    /// `ST0`'s 80 bits (mantissa, then sign and exponent).
    pub st0_raw: Option<[u8; 10]>,
}

impl Ret {
    /// `ST0` as `fst dword` stores it ([`f80_to_f32`]: one rounding from
    /// the 80 bits, NaN payloads kept).
    #[must_use]
    pub fn st0_f32(&self) -> Option<f32> {
        self.st0_raw.map(f80_to_f32)
    }

    /// Whether the original's `ST0` is exactly a port's float result
    /// ([`same_float`]; false when the x87 stack is empty).
    #[must_use]
    pub fn st0_is(&self, port: f64) -> bool {
        self.st0.is_some_and(|st0| same_float(st0, port))
    }

    /// `ST0` as `fstp qword` stores it, NaN payloads kept (quieted): the
    /// same as [`Ret::st0`] ([`f80_to_f64`]), kept for the callers that
    /// name it; what a game-side `f64` call (`call_cdecl_f64` in
    /// `ffx-hook`) returns.
    #[must_use]
    pub fn st0_f64(&self) -> Option<f64> {
        self.st0
    }
}

#[derive(Clone, Copy)]
enum Stub {
    Malloc,
    Calloc,
    Free,
    Memcpy,
    Memmove,
    Memset,
    Crt(crt::Crt),
    /// A `KERNEL32` function the runtime DLL calls.
    Win32(win32::Win32),
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
    /// Imports with nothing to run: `DLL!name`.
    missing: Vec<String>,
    /// Imports with no native stub (`Stub::Missing` indexes this), whether
    /// or not the runtime DLL has them.
    unstubbed: Vec<String>,
    heap_base: u64,
    heap_next: u64,
    stack_base: u64,
    sentinel: u64,
    /// The call returned to the sentinel (else the instruction budget ran
    /// out or a stub stopped it).
    returned: bool,
    fault: Option<Error>,
    pages: Option<Arc<Pages>>,
    /// Pages written since the last [`Oracle::clear_dirty`].
    dirty: BTreeSet<u32>,
    image_base: u32,
    /// The CRT's `rand` state (`srand`; 1 at start).
    rand_seed: u32,
    /// What each import is bound to (stub or runtime export), by name.
    imports: HashMap<String, u64>,
    /// The native stub of every import that has one, by name.
    natives: HashMap<String, u64>,
    /// The import address table slot of every import, by name.
    slots: HashMap<String, u64>,
    /// The runtime DLL's exports, by name.
    exports: HashMap<String, u32>,
    /// Where `MSVCR110.dll` is loaded (0: not loaded).
    dll_base: u32,
    /// The CRT heap (`HeapAlloc`): next free address and end. Not reset by
    /// [`Oracle::reset_heap`].
    crt_heap: (u64, u64),
    /// Block sizes for `HeapSize`/`HeapReAlloc`, by address (both heaps).
    heap_sizes: HashMap<u32, u32>,
    /// Thread-local slots (`TlsAlloc`).
    tls: Vec<u32>,
    /// The `KERNEL32` stand-ins' handles, last error and semaphores.
    win32: win32::Objects,
    /// The runtime DLL's private memory, left out of [`Oracle::diff`].
    private: Vec<std::ops::Range<u64>>,
    /// Free space for [`Oracle::alloc_code`], in the stub area.
    code_next: u64,
    /// Strings handed out once (command line, environment).
    constants: HashMap<&'static str, u32>,
    /// The targets and states of a running [`Oracle::harvest`].
    harvest: Option<harvest::Run>,
    /// Where [`Oracle::call`] puts its arguments, when not at the top of
    /// the stack (below a harvested call's frame, [`Oracle::apply_state`]).
    call_top: Option<u64>,
    /// Blocks run in the `FFX_ORACLE_COVER` ranges (`ffx.sym` addresses).
    cover: BTreeSet<u32>,
    /// [`Oracle::call`]'s instruction budget.
    max_instructions: usize,
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
        Self::load_with(path, Runtime::from_env())
    }

    /// [`Oracle::load`] with a choice of C runtime.
    ///
    /// # Errors
    /// As [`Oracle::load`]; also when the runtime DLL fails to initialise.
    pub fn load_with(path: impl AsRef<Path>, runtime: Runtime) -> Result<Self> {
        let bytes = std::fs::read(path.as_ref())?;
        let dll = runtime.dll_beside(path.as_ref());
        Self::build(&bytes, None, None, dll.as_deref())
    }

    /// Loads `FFX.exe` where the snapshot's game had it, with the
    /// snapshot's memory behind it.
    ///
    /// # Errors
    /// As [`Oracle::load`]; also when the snapshot leaves no room for the
    /// oracle's own stack, heap and stubs.
    pub fn from_snapshot(exe: impl AsRef<Path>, pages: &Arc<Pages>) -> Result<Self> {
        Self::from_snapshot_with(exe, pages, Runtime::from_env())
    }

    /// [`Oracle::from_snapshot`] with a choice of C runtime.
    ///
    /// # Errors
    /// As [`Oracle::from_snapshot`]; also when the runtime DLL fails to
    /// initialise.
    pub fn from_snapshot_with(
        exe: impl AsRef<Path>,
        pages: &Arc<Pages>,
        runtime: Runtime,
    ) -> Result<Self> {
        let bytes = std::fs::read(exe.as_ref())?;
        let dll = runtime.dll_beside(exe.as_ref());
        Self::build(
            &bytes,
            Some(pages.snapshot.image_base),
            Some(pages.clone()),
            dll.as_deref(),
        )
    }

    #[allow(clippy::too_many_lines)] // one setup, in order
    fn build(
        bytes: &[u8],
        at: Option<u32>,
        pages: Option<Arc<Pages>>,
        dll: Option<&[u8]>,
    ) -> Result<Self> {
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

        let mut uc = Unicorn::new_with_data(
            Arch::X86,
            Mode::MODE_32,
            State {
                rand_seed: 1,
                max_instructions: MAX_INSTRUCTIONS,
                ..State::default()
            },
        )?;
        // A Core 2: SSE2 and SSE3, no SSE4.2. MSVCR110 picks its string
        // routines by CPUID, and the emulator's SSE4.2 string instructions
        // (`pcmpistri`) give wrong answers (its strstr found nothing);
        // the SSE2 routines compute the same functions.
        uc.ctl_set_cpu_model(CPU_CORE2DUO)?;
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
        let teb_base = gap(&mut taken, 2 * PAGE)?;
        uc.mem_map(teb_base, 2 * PAGE, Prot::READ | Prot::WRITE)?;
        thread_block(&mut uc, teb_base, stack_base)?;
        {
            let s = uc.get_data_mut();
            s.heap_base = heap_base;
            s.heap_next = heap_base;
            s.stack_base = stack_base;
            s.sentinel = stub_base + STUB_SIZE - 0x10;
            s.image_base = base;
            s.pages = pages;
        }
        // The game's C runtime, its imports bound to `win32` stubs.
        let mut next_stub = stub_base;
        let runtime = match dll {
            Some(dll) => {
                let crt_heap = gap(&mut taken, CRT_HEAP_SIZE)?;
                uc.mem_map(crt_heap, CRT_HEAP_SIZE, Prot::READ | Prot::WRITE)?;
                uc.get_data_mut().crt_heap = (crt_heap, crt_heap + CRT_HEAP_SIZE);
                Some(load_runtime(&mut uc, dll, &mut taken, &mut next_stub)?)
            }
            None => None,
        };

        // Bind every import to its own stub address, filled with `ret` so a
        // stray execution cannot fall through; MSVCR110 imports to the
        // runtime's exports when it is loaded, except the few kept native.
        for import in &pe.imports {
            let name = import.name.as_ref();
            let export = runtime.as_ref().and_then(|r| {
                (import.dll.eq_ignore_ascii_case("msvcr110.dll") && !KEPT_NATIVE.contains(&name))
                    .then(|| r.exports.get(name).copied())
                    .flatten()
            });
            let stub = match name {
                "malloc" => Some(Stub::Malloc),
                "calloc" => Some(Stub::Calloc),
                "free" => Some(Stub::Free),
                "memcpy" => Some(Stub::Memcpy),
                "memmove" => Some(Stub::Memmove),
                "memset" => Some(Stub::Memset),
                other => crt::lookup(other).map(Stub::Crt),
            };
            // FFX.exe's own KERNEL32 calls: the runtime's stand-ins
            let win = stub
                .is_none()
                .then(|| win32::lookup(import.dll, name))
                .flatten();
            let stub = stub.or(win.map(|(api, _)| Stub::Win32(api)));
            let stub = stub.unwrap_or_else(|| {
                let state = uc.get_data_mut();
                state.unstubbed.push(format!("{}!{name}", import.dll));
                if export.is_none() {
                    state.missing.push(format!("{}!{name}", import.dll));
                }
                Stub::Missing(state.unstubbed.len() - 1)
            });
            let code = match (stub, win) {
                (Stub::Crt(c), _) => crt::code(c, u32::try_from(next_stub).expect("32-bit")),
                (_, Some((_, n))) => win32::code(n),
                _ => vec![0xc3],
            };
            uc.mem_write(next_stub, &code)?;
            let bound = export.map_or(next_stub, u64::from);
            uc.mem_write(
                u64::from(base) + import.offset as u64,
                &u32::try_from(bound).expect("32-bit").to_le_bytes(),
            )?;
            let state = uc.get_data_mut();
            state.stubs.insert(next_stub, stub);
            if !matches!(stub, Stub::Missing(_)) {
                state.natives.insert(name.to_string(), next_stub);
            }
            state.imports.insert(name.to_string(), bound);
            state
                .slots
                .insert(name.to_string(), u64::from(base) + import.offset as u64);
            next_stub += 0x10;
        }
        let sentinel = uc.get_data().sentinel;
        uc.mem_write(sentinel, &[0xc3])?;
        uc.get_data_mut().code_next = next_stub.max(stub_base + STUB_SIZE / 2);

        uc.add_code_hook(stub_base, stub_base + STUB_SIZE - 1, |uc, addr, _| {
            if addr == uc.get_data().sentinel {
                uc.get_data_mut().returned = true;
                let _ = uc.emu_stop();
                return;
            }
            if let Some(stub) = uc.get_data().stubs.get(&addr).copied()
                && let Err(e) = run_stub(uc, stub, addr)
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
        if let Some(r) = runtime.as_ref().filter(|r| r.x87_exact) {
            // int3 at each, one interrupt hook for all
            let sites: HashMap<u64, x87::Op> = r.x87_sites.iter().copied().collect();
            for &at in sites.keys() {
                uc.mem_write(at, &[0xcc])?;
            }
            uc.add_intr_hook(move |uc, intno| {
                let at = uc
                    .reg_read(RegisterX86::EIP)
                    .map_or(0, |e| e.wrapping_sub(1));
                let r = match sites.get(&at) {
                    Some(&op) if intno == 3 => x87_hook(uc, at, op),
                    _ => Err(Error::Interrupt(intno, at)),
                };
                if let Err(e) = r {
                    uc.get_data_mut().fault = Some(e);
                    let _ = uc.emu_stop();
                }
            })?;
        } else if let Some(r) = &runtime {
            for &(at, op) in &r.x87_sites {
                uc.add_code_hook(at, at, move |uc, addr, _| {
                    if let Err(e) = x87_hook(uc, addr, op) {
                        uc.get_data_mut().fault = Some(e);
                        let _ = uc.emu_stop();
                    }
                })?;
            }
        }
        if let Some(ranges) = cover_ranges() {
            for (lo, len) in ranges {
                let at = u64::from(lo.wrapping_sub(0x0040_0000).wrapping_add(base));
                uc.add_block_hook(at, at + u64::from(len.max(1)) - 1, move |uc, addr, _| {
                    let s = uc.get_data_mut();
                    let sym = u32::try_from(addr)
                        .unwrap_or(0)
                        .wrapping_sub(s.image_base)
                        .wrapping_add(0x0040_0000);
                    s.cover.insert(sym);
                })?;
            }
        }
        let mut o = Self { uc };
        // The game thread's x87 control word (53-bit precision, docs/porting.md);
        // the emulator starts at 64-bit, where chained float code rounds
        // differently. A snapshot carries the one it was taken with.
        let cw =
            o.uc.get_data()
                .pages
                .as_ref()
                .map_or(GAME_FPU_CONTROL, |p| p.snapshot.fpu_control);
        if let Some(r) = runtime {
            o.uc.get_data_mut().exports.clone_from(&r.exports);
            // DllMain(module, DLL_PROCESS_ATTACH, 0) on the game thread's x87
            // setting; its work is not part of any later comparison.
            o.set_fpu_control(GAME_FPU_CONTROL)?;
            let ok = o
                .call(r.entry, Conv::Stdcall, &[r.base, 1, 0])
                .map_err(|e| Error::RuntimeInit(e.to_string()))?;
            if ok.eax == 0 {
                return Err(Error::RuntimeInit("DllMain returned FALSE".into()));
            }
            // An invalid-parameter handler that returns: a call the runtime
            // rejects (sprintf_s into too small a buffer, ...) takes the
            // CRT's error path (an empty string, -1, errno) instead of
            // ending the process. The game installs none and would end;
            // tests can still compare what the original does up to there.
            let handler = o.alloc_code(&[0xc3])?;
            if let Some(set) = o.runtime_export("_set_invalid_parameter_handler") {
                o.call(set, Conv::Cdecl, &[handler])
                    .map_err(|e| Error::RuntimeInit(e.to_string()))?;
            }
            // The runtime's own state (its image, and what it allocated to
            // start: per-thread data with errno, locale, stdio tables) is
            // left out of comparisons; later blocks start on a fresh page.
            let s = o.uc.get_data_mut();
            s.crt_heap.0 = s.crt_heap.0.next_multiple_of(PAGE);
            let heap_start = s.crt_heap.1 - CRT_HEAP_SIZE;
            s.private = vec![
                u64::from(r.base)..u64::from(r.base) + r.size,
                heap_start..s.crt_heap.0,
            ];
            o.clear_dirty();
        }
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

    /// Replaces the cdecl function at `addr` (rebased) by Rust, for running
    /// an original caller against what a hooked callee does (a shim and
    /// its bridge). When the emulated code reaches `addr`, `f` runs on the
    /// process as the callee sees it ([`StandIn`]: the return address at
    /// [`StandIn::esp`], the arguments above it) and its result goes to
    /// the 8-byte `cell`; the bytes written at `addr` then return it as
    /// `returns` says (`mov eax, [cell]` or `fld dword [cell]`) and `ret`.
    /// `cell` is the test's own memory, so the same in both oracles of a
    /// pair keeps their diff clean. A failed memory access in `f` stops
    /// the call with that error.
    ///
    /// # Errors
    /// On emulator errors.
    pub fn stand_in(
        &mut self,
        addr: u32,
        returns: Returns,
        cell: u32,
        mut f: impl FnMut(&mut StandIn<'_, '_>) -> u64 + 'static,
    ) -> Result<()> {
        let mut code = match returns {
            Returns::Nothing => vec![],
            Returns::Eax => vec![0xa1],
            Returns::St0F32 => vec![0xd9, 0x05],
            Returns::St0F64 => vec![0xdd, 0x05],
        };
        if returns != Returns::Nothing {
            code.extend(cell.to_le_bytes());
        }
        code.push(0xc3);
        self.write(addr, &code)?;
        self.uc
            .add_code_hook(u64::from(addr), u64::from(addr), move |uc, _, _| {
                let mut s = StandIn { uc, fault: None };
                let v = f(&mut s);
                let fault = s.fault.take();
                let r = match fault {
                    Some(e) => Err(e),
                    None => crt::write(uc, cell, &v.to_le_bytes()),
                };
                if let Err(e) = r {
                    uc.get_data_mut().fault = Some(e);
                    let _ = uc.emu_stop();
                }
            })?;
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
        let heap = s.heap_base..s.heap_base + HEAP_SIZE;
        s.heap_sizes.retain(|&a, _| !heap.contains(&u64::from(a)));
    }

    /// Allocates `len` zeroed bytes in emulated memory.
    ///
    /// # Errors
    /// Fails when the emulated heap is exhausted.
    pub fn alloc(&mut self, len: usize) -> Result<u32> {
        alloc(&mut self.uc, len as u64)
    }

    /// Maps zeroed read-write memory at a fixed place (whole pages around
    /// `addr..addr + len`), for tests of code that writes through a null
    /// or made-up pointer (the null page itself, say). Pages already
    /// mapped, or in the snapshot, are left as they are.
    ///
    /// # Errors
    /// On emulator errors.
    pub fn map_zeroed(&mut self, addr: u32, len: u32) -> Result<()> {
        let end = (u64::from(addr) + u64::from(len)).next_multiple_of(PAGE);
        map_unknown(&mut self.uc, u64::from(addr) & !(PAGE - 1), end)
    }

    /// Copies machine code into executable emulated memory (a test's own
    /// thunks); it stays until the oracle is dropped.
    ///
    /// # Errors
    /// When the code area is full.
    pub fn alloc_code(&mut self, code: &[u8]) -> Result<u32> {
        let s = self.uc.get_data_mut();
        let at = s.code_next;
        let end = at + (code.len() as u64).next_multiple_of(16);
        if end > s.sentinel {
            return Err(Error::OutOfMemory);
        }
        s.code_next = end;
        self.uc.mem_write(at, code)?;
        Ok(u32::try_from(at).expect("32-bit"))
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
        if !bytes.is_empty() {
            // drop any translation of the written bytes: tests write code
            // anywhere (stubs in .reloc, the stub area), and a stale
            // translation keeps running the old bytes
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

    /// The imports with no stub, as `DLL!name` (calling one fails with
    /// [`Error::UnstubbedImport`]).
    #[must_use]
    pub fn missing_imports(&self) -> &[String] {
        &self.uc.get_data().missing
    }

    /// What an import is bound to: the runtime DLL's export, or a native
    /// stub (call it with [`Oracle::call`]).
    #[must_use]
    pub fn import_stub(&self, name: &str) -> Option<u32> {
        self.uc
            .get_data()
            .imports
            .get(name)
            .map(|&a| u32::try_from(a).expect("32-bit stub"))
    }

    /// The native stub of an import, if it has one, whatever the import is
    /// bound to.
    #[must_use]
    pub fn native_stub(&self, name: &str) -> Option<u32> {
        self.uc
            .get_data()
            .natives
            .get(name)
            .map(|&a| u32::try_from(a).expect("32-bit stub"))
    }

    /// An export of the loaded runtime DLL (`MSVCR110.dll`), if loaded.
    #[must_use]
    pub fn runtime_export(&self, name: &str) -> Option<u32> {
        self.uc.get_data().exports.get(name).copied()
    }

    /// Where the runtime DLL is loaded, if it is.
    #[must_use]
    pub fn runtime_base(&self) -> Option<u32> {
        Some(self.uc.get_data().dll_base).filter(|&b| b != 0)
    }

    /// Loads the game's C++ runtime, `MSVCP110.dll` from beside `exe`
    /// (the `FFX.exe` this oracle was made from), for tests of originals
    /// that run the standard library's streams and locales (`docs/porting.md`,
    /// "The C++ runtime"). Needs the C runtime loaded ([`Runtime::Dll`]).
    ///
    /// The DLL is mapped into a free range and relocated; its `MSVCR110`
    /// imports are bound to the loaded runtime's exports (all of them,
    /// `malloc` and `free` included: what it allocates lives on the C
    /// runtime's heap and is freed there), its `KERNEL32` imports to the
    /// `win32` stand-ins (or missing stubs that stop the emulation).
    /// `DllMain(module, DLL_PROCESS_ATTACH, 0)` runs on the game thread's
    /// x87 setting, then `FFX.exe`'s `MSVCP110` imports are bound to the
    /// DLL's exports (functions, vtables and objects alike). The DLL's
    /// image and the whole C runtime heap are left out of [`Oracle::diff`]:
    /// locale reference counts and facets change in the original's run of a
    /// pair only. Its x87 instructions run as the emulator has them (the
    /// integer and string paths use none).
    ///
    /// # Errors
    /// When the C runtime is not loaded, the DLL is missing or cannot be
    /// parsed, there is no room for it, or its `DllMain` fails.
    pub fn load_cpp_runtime(&mut self, exe: impl AsRef<Path>) -> Result<()> {
        let exe = exe.as_ref();
        if self.runtime_base().is_none() {
            return Err(Error::RuntimeInit(
                "MSVCP110.dll needs MSVCR110.dll loaded".into(),
            ));
        }
        let dir = exe.parent().unwrap_or_else(|| Path::new("."));
        let bytes = ["MSVCP110.dll", "msvcp110.dll"]
            .iter()
            .find_map(|n| std::fs::read(dir.join(n)).ok())
            .ok_or_else(|| Error::RuntimeInit("no MSVCP110.dll beside FFX.exe".into()))?;
        let pe = PE::parse(&bytes)?;
        let mut image = lay_out(&bytes, &pe)?;
        let mut taken: Vec<(u64, u64)> = self
            .uc
            .mem_regions()?
            .iter()
            .map(|r| (r.begin, r.end + 1))
            .collect();
        if let Some(p) = &self.uc.get_data().pages {
            for r in &p.snapshot.regions {
                taken.push((u64::from(r.base), u64::from(r.base) + u64::from(r.size)));
            }
        }
        let base = gap(&mut taken, image.len() as u64)?;
        let base32 = u32::try_from(base).expect("32-bit");
        let preferred = u32::try_from(pe.image_base)
            .map_err(|_| goblin::error::Error::Malformed("base".into()))?;
        relocate(&mut image, &pe, base32.wrapping_sub(preferred));
        self.uc.mem_map(base, image.len() as u64, Prot::ALL)?;
        self.uc.mem_write(base, &image)?;
        for import in &pe.imports {
            let name = import.name.as_ref();
            let export = import
                .dll
                .eq_ignore_ascii_case("msvcr110.dll")
                .then(|| self.uc.get_data().exports.get(name).copied())
                .flatten();
            let bound = match export {
                Some(at) => at,
                None => {
                    let (stub, code) = if let Some((api, n)) = win32::lookup(import.dll, name) {
                        (Stub::Win32(api), win32::code(n))
                    } else {
                        let state = self.uc.get_data_mut();
                        state
                            .unstubbed
                            .push(format!("{}!{name} (MSVCP110)", import.dll));
                        (Stub::Missing(state.unstubbed.len() - 1), vec![0xc3])
                    };
                    let at = self.alloc_code(&code)?;
                    self.uc.get_data_mut().stubs.insert(u64::from(at), stub);
                    at
                }
            };
            self.uc
                .mem_write(base + import.offset as u64, &bound.to_le_bytes())?;
        }
        let exports: HashMap<&str, u32> = pe
            .exports
            .iter()
            .filter(|e| e.reexport.is_none())
            .filter_map(|e| Some((e.name?, base32 + u32::try_from(e.rva).ok()?)))
            .collect();
        let entry = base32
            + pe.header
                .optional_header
                .map_or(0, |o| o.standard_fields.address_of_entry_point);
        let cw = u32::try_from(self.uc.reg_read(RegisterX86::FPCW)?).unwrap_or(GAME_FPU_CONTROL);
        self.set_fpu_control(GAME_FPU_CONTROL)?;
        let ok = self
            .call(entry, Conv::Stdcall, &[base32, 1, 0])
            .map_err(|e| Error::RuntimeInit(format!("MSVCP110: {e}")))?;
        if ok.eax == 0 {
            return Err(Error::RuntimeInit("MSVCP110 DllMain returned FALSE".into()));
        }
        self.set_fpu_control(cw)?;
        // FFX.exe's MSVCP110 imports to the DLL's exports
        let ffx = std::fs::read(exe)?;
        let fpe = PE::parse(&ffx)?;
        let image_base = u64::from(self.image_base());
        for import in &fpe.imports {
            if !import.dll.eq_ignore_ascii_case("msvcp110.dll") {
                continue;
            }
            let name = import.name.as_ref();
            if let Some(&at) = exports.get(name) {
                self.uc
                    .mem_write(image_base + import.offset as u64, &at.to_le_bytes())?;
                self.uc
                    .get_data_mut()
                    .imports
                    .insert(name.to_string(), u64::from(at));
            }
        }
        let s = self.uc.get_data_mut();
        let crt_heap = s.crt_heap.1 - CRT_HEAP_SIZE..s.crt_heap.1;
        s.private.push(base..base + image.len() as u64);
        s.private.push(crt_heap);
        self.clear_dirty();
        Ok(())
    }

    /// Leaves `range` out of [`Oracle::diff`] (a test's own scratch: a bump
    /// heap the original's allocations go to, say).
    pub fn add_private(&mut self, range: std::ops::Range<u32>) {
        self.uc
            .get_data_mut()
            .private
            .push(u64::from(range.start)..u64::from(range.end));
    }

    /// Rebinds an import's slot in `FFX.exe`'s import table to `target`
    /// (a test's own code), or to its native stub when `target` is `None`.
    /// Returns the previous binding.
    ///
    /// # Errors
    /// When the import is unknown or has no native stub.
    pub fn bind_import(&mut self, name: &str, target: Option<u32>) -> Result<u32> {
        let s = self.uc.get_data();
        let slot = *s
            .slots
            .get(name)
            .ok_or_else(|| Error::UnstubbedImport(name.to_string()))?;
        let to = match target {
            Some(t) => t,
            None => self
                .native_stub(name)
                .ok_or_else(|| Error::UnstubbedImport(name.to_string()))?,
        };
        let old = self.uc.mem_read_as_vec(slot, 4)?;
        self.uc.mem_write(slot, &to.to_le_bytes())?;
        self.uc
            .get_data_mut()
            .imports
            .insert(name.to_string(), u64::from(to));
        Ok(u32::from_le_bytes(old.try_into().expect("4 bytes")))
    }

    /// Sets the instruction budget of each [`Oracle::call`] (default
    /// [`MAX_INSTRUCTIONS`]): a call that has not returned by then fails
    /// with [`Error::Runaway`]. A test whose original calls are all short
    /// but some spin forever (waits on a thread the emulator does not run)
    /// lowers it so each spin costs less; a call that runs out of it fails
    /// as a spin would, so the budget must stay far above the longest call
    /// that returns.
    pub fn set_max_instructions(&mut self, n: usize) {
        self.uc.get_data_mut().max_instructions = n;
    }

    /// Where [`Oracle::call`] pushes its arguments (`None`: the top of the
    /// oracle's stack, less `0x100`), for a port of an original whose
    /// frame callees see (pointers to its locals in their logs, in objects
    /// they keep): the port's oracle gets its frame at the original's
    /// frame address and its calls below it, so both pass the same
    /// pointers (`tests/phyre_render.rs`).
    pub fn set_call_top(&mut self, top: Option<u32>) {
        self.uc.get_data_mut().call_top = top.map(u64::from);
    }

    /// The instruction budget of each [`Oracle::call`].
    #[must_use]
    pub fn max_instructions(&self) -> usize {
        self.uc.get_data().max_instructions
    }

    /// The oracle's own stack, which differs between runs by construction.
    #[must_use]
    pub fn stack_range(&self) -> std::ops::Range<u32> {
        let b = u32::try_from(self.uc.get_data().stack_base).expect("32-bit");
        b..b + u32::try_from(STACK_SIZE).expect("32-bit")
    }

    /// Bytes that differ between two oracles over the pages either wrote,
    /// outside both stacks and the runtime DLL's own state (its image and
    /// the blocks it allocated to start): `(address, ours, theirs)`.
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
        let private: Vec<_> = self
            .uc
            .get_data()
            .private
            .iter()
            .chain(&other.uc.get_data().private)
            .cloned()
            .collect();
        let mut out = Vec::new();
        for p in pages {
            let a = p << 12;
            if s1.contains(&a)
                || s2.contains(&a)
                || private.iter().any(|r| r.contains(&u64::from(a)))
            {
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

    /// Calls a cdecl function that returns a float in `ST0` and returns it
    /// as `fst qword` stores it, NaN sign and payload kept ([`Ret::st0`]):
    /// the callback to hand a port for an original float function.
    ///
    /// # Errors
    /// As [`Oracle::call_cdecl`]; [`Error::NoFloat`] when the x87 stack is
    /// empty at the return.
    pub fn call_f64(&mut self, addr: u32, args: &[u32]) -> Result<f64> {
        self.call(addr, Conv::Cdecl, args)?
            .st0
            .ok_or(Error::NoFloat(addr))
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
        let (top, sentinel) = {
            let s = self.uc.get_data();
            (
                s.call_top.unwrap_or(s.stack_base + STACK_SIZE - 0x100),
                s.sentinel,
            )
        };
        let stack_args: &[u32] = match conv {
            Conv::Fastcall | Conv::FastcallCallerPops => {
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
        let mut sp = top;
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
        // The x87 stack is empty at a call, as the calling conventions
        // require: drop what earlier calls returned in ST0.
        let sw = self.uc.reg_read(RegisterX86::FPSW)?;
        self.uc.reg_write(RegisterX86::FPSW, sw & !0x3800)?;
        self.uc.reg_write(RegisterX86::FPTAG, 0xffff)?;
        self.uc.get_data_mut().fault = None;
        self.uc.get_data_mut().returned = false;

        let budget = self.uc.get_data().max_instructions;
        let run = self.uc.emu_start(u64::from(addr), 0, 0, budget);
        if let Some(fault) = self.uc.get_data_mut().fault.take() {
            return Err(fault);
        }
        run?;
        if !self.uc.get_data().returned {
            return Err(Error::Runaway(self.uc.reg_read(RegisterX86::EIP)?, budget));
        }
        for (reg, name, value) in SAVED {
            if self.uc.reg_read(reg)? != value {
                return Err(Error::Clobbered(name));
            }
        }
        let popped = match conv {
            Conv::Cdecl | Conv::FastcallCallerPops => 0,
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

/// `FFX_ORACLE_COVER`: comma-separated `addr+len` ranges (`ffx.sym`
/// addresses, hexadecimal) whose run blocks every oracle appends to the
/// file `FFX_ORACLE_COVER_OUT` when dropped, one address per line: rough
/// branch reach of a test (docs/hooking.md).
fn cover_ranges() -> Option<Vec<(u32, u32)>> {
    let spec = std::env::var("FFX_ORACLE_COVER").ok()?;
    let num = |t: &str| {
        let t = t.trim().replace('_', "");
        let t = t.strip_prefix("0x").unwrap_or(&t);
        u32::from_str_radix(t, 16).ok()
    };
    Some(
        spec.split(',')
            .filter_map(|r| {
                let (a, n) = r.split_once('+')?;
                Some((num(a)?, num(n)?))
            })
            .collect(),
    )
}

impl Drop for Oracle {
    fn drop(&mut self) {
        let cover = &self.uc.get_data().cover;
        if cover.is_empty() {
            return;
        }
        if let Some(out) = std::env::var_os("FFX_ORACLE_COVER_OUT") {
            use std::io::Write;
            let text = cover.iter().fold(String::new(), |mut t, a| {
                use std::fmt::Write as _;
                let _ = writeln!(t, "{a:#010x}");
                t
            });
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(out)
            {
                let _ = f.write_all(text.as_bytes());
            }
        }
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

/// How a [`Oracle::stand_in`] returns its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Returns {
    /// Nothing (the cell is written but not read).
    Nothing,
    /// The low word in `eax`.
    Eax,
    /// The low word as an `f32` in ST0.
    St0F32,
    /// The value as an `f64` in ST0.
    St0F64,
}

/// The emulated process as a Rust stand-in sees it ([`Oracle::stand_in`]):
/// memory as a [`ffx_bridge::Mem`] and the callee's stack.
pub struct StandIn<'a, 'u> {
    uc: &'a mut Unicorn<'u, State>,
    fault: Option<Error>,
}

impl StandIn<'_, '_> {
    /// `esp` on entry: the return address is at `[esp]`.
    #[must_use]
    pub fn esp(&self) -> u32 {
        u32::try_from(self.uc.reg_read(RegisterX86::ESP).unwrap_or(0)).unwrap_or(0)
    }

    /// The address of stack argument word `i` (`esp + 4 + 4i`).
    #[must_use]
    pub fn slot(&self, i: u32) -> u32 {
        self.esp().wrapping_add(4 + 4 * i)
    }

    /// Stack argument word `i`.
    #[must_use]
    pub fn arg(&self, i: u32) -> u32 {
        ffx_bridge::Mem::read(self, self.slot(i))
    }
}

impl ffx_bridge::Mem for StandIn<'_, '_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
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
                .unwrap_or(0);
        }
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        if let Err(e) = crt::write(self.uc, addr, data) {
            self.fault.get_or_insert(e);
        }
    }
    fn image_base(&self) -> u32 {
        self.uc.get_data().image_base
    }
}

/// Which C runtime the emulated code calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Runtime {
    /// The game's own `MSVCR110.dll` from beside `FFX.exe`, loaded into
    /// the emulator and initialised, its `KERNEL32` imports bound to
    /// native stand-ins (`win32`). `FFX.exe`'s MSVCR110 imports call its
    /// exports, except the heap and copy functions and `_wassert`. Without
    /// the file this is [`Runtime::Native`].
    #[default]
    Dll,
    /// Native stubs only (`crt`): faster to start, re-implemented
    /// semantics. `FFX_ORACLE_RUNTIME=native` makes it the default.
    Native,
}

impl Runtime {
    /// [`Runtime::Native`] when `FFX_ORACLE_RUNTIME` is `native`, else
    /// [`Runtime::Dll`].
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var("FFX_ORACLE_RUNTIME") {
            Ok(v) if v.eq_ignore_ascii_case("native") => Self::Native,
            _ => Self::Dll,
        }
    }

    /// The runtime DLL's bytes, for [`Runtime::Dll`] when it is beside the
    /// executable.
    fn dll_beside(self, exe: &Path) -> Option<Vec<u8>> {
        if self == Self::Native {
            return None;
        }
        let dir = exe.parent()?;
        ["MSVCR110.dll", "msvcr110.dll"]
            .iter()
            .find_map(|n| std::fs::read(dir.join(n)).ok())
    }
}

/// The loaded runtime DLL.
struct LoadedDll {
    base: u32,
    size: u64,
    entry: u32,
    exports: HashMap<String, u32>,
    /// Where the x87 instructions [`x87`] computes are, and whether those
    /// are exactly the instruction starts.
    x87_sites: Vec<(u64, x87::Op)>,
    x87_exact: bool,
}

/// The sections of a PE file laid out as in memory (unrelocated).
fn lay_out(bytes: &[u8], pe: &PE) -> Result<Vec<u8>> {
    let opt = pe
        .header
        .optional_header
        .ok_or(goblin::error::Error::Malformed("no optional header".into()))?;
    let size = u64::from(opt.windows_fields.size_of_image).next_multiple_of(PAGE);
    let mut image = vec![0u8; usize::try_from(size).expect("image size")];
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
    Ok(image)
}

/// Maps `MSVCR110.dll` into a free range, relocated, its imports bound to
/// `win32` stubs (or missing ones that stop the emulation, named
/// `KERNEL32.dll!X (MSVCR110)`).
fn load_runtime(
    uc: &mut Unicorn<'static, State>,
    bytes: &[u8],
    taken: &mut Vec<(u64, u64)>,
    next_stub: &mut u64,
) -> Result<LoadedDll> {
    let pe = PE::parse(bytes)?;
    let mut image = lay_out(bytes, &pe)?;
    let base = gap(taken, image.len() as u64)?;
    let base32 = u32::try_from(base).expect("32-bit");
    let preferred =
        u32::try_from(pe.image_base).map_err(|_| goblin::error::Error::Malformed("base".into()))?;
    relocate(&mut image, &pe, base32.wrapping_sub(preferred));
    uc.mem_map(base, image.len() as u64, Prot::ALL)?;
    uc.mem_write(base, &image)?;
    uc.get_data_mut().dll_base = base32;
    for import in &pe.imports {
        let name = import.name.as_ref();
        let (stub, code) = if let Some((api, n)) = win32::lookup(import.dll, name) {
            (Stub::Win32(api), win32::code(n))
        } else {
            let state = uc.get_data_mut();
            state
                .unstubbed
                .push(format!("{}!{name} (MSVCR110)", import.dll));
            (Stub::Missing(state.unstubbed.len() - 1), vec![0xc3])
        };
        uc.mem_write(*next_stub, &code)?;
        uc.mem_write(
            base + import.offset as u64,
            &u32::try_from(*next_stub).expect("32-bit").to_le_bytes(),
        )?;
        uc.get_data_mut().stubs.insert(*next_stub, stub);
        *next_stub += 0x10;
    }
    let exports = pe
        .exports
        .iter()
        .filter(|e| e.reexport.is_none())
        .filter_map(|e| Some((e.name?.to_string(), base32 + u32::try_from(e.rva).ok()?)))
        .collect();
    let entry = base32
        + pe.header
            .optional_header
            .map_or(0, |o| o.standard_fields.address_of_entry_point);
    let code: Vec<(u32, u32)> = pe
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
        .map(|s| (s.virtual_address, s.virtual_size))
        .collect();
    let found = ffx_fpu::sites(
        &image,
        pe.header.coff_header.time_date_stamp,
        pe.header
            .optional_header
            .map_or(0, |o| o.windows_fields.check_sum),
        &code,
    );
    let x87_exact = found.exact();
    let x87_sites = found
        .at
        .iter()
        .map(|&(rva, op)| (base + u64::from(rva), op))
        .collect();
    Ok(LoadedDll {
        base: base32,
        size: image.len() as u64,
        entry,
        exports,
        x87_sites,
        x87_exact,
    })
}

/// Runs one of the x87 instructions [`x87`] computes in place of the
/// emulator's own version, then continues after it.
fn x87_hook(uc: &mut Unicorn<State>, addr: u64, op: x87::Op) -> Result<()> {
    Ok(ffx_fpu::unicorn::step(uc, addr, op)?)
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
/// A thread environment block where `fs` points, as on Windows: code with
/// structured exception handling links its frames at `fs:[0]` (the chain
/// starts empty, -1), and `fs:[4]`/`fs:[8]` are the stack's top and bottom,
/// `fs:[0x18]` the block itself. Page one holds a descriptor table whose
/// entry 1 gives `fs` that base (Unicorn ignores a 32-bit `FS_BASE` write and
/// starts with protection off, where a selector is not looked up), page two
/// the block.
fn thread_block(uc: &mut Unicorn<'static, State>, at: u64, stack_base: u64) -> Result<()> {
    let teb = at + PAGE;
    let word = |v: u64| u32::try_from(v).expect("below 4 GiB").to_le_bytes();
    uc.mem_write(teb, &u32::MAX.to_le_bytes())?;
    uc.mem_write(teb + 4, &word(stack_base + STACK_SIZE))?;
    uc.mem_write(teb + 8, &word(stack_base))?;
    uc.mem_write(teb + 0x18, &word(teb))?;
    uc.mem_write(teb + 0x24, &1u32.to_le_bytes())?;
    // entry 1: data at the block, 4 KiB, ring 3 (fs); 2: flat 32-bit code;
    // 3: flat 32-bit data (ss, ds, es). With protection on, a segment's
    // cached flags decide the operand and stack size, so all of them need
    // proper 32-bit entries, not only fs.
    let descriptor = |base: u64, limit: u64, access: u64, flags: u64| -> u64 {
        (limit & 0xffff)
            | ((base & 0xff_ffff) << 16)
            | (access << 40)
            | (((limit >> 16) & 0xf) << 48)
            | (flags << 52)
            | ((base >> 24) << 56)
    };
    let teb_base = u64::from(u32::try_from(teb).expect("below 4 GiB"));
    let table = [
        0,
        descriptor(teb_base, 0xfff, 0xf3, 0x4),
        descriptor(0, 0xf_ffff, 0x9b, 0xc),
        descriptor(0, 0xf_ffff, 0x93, 0xc),
    ];
    for (i, d) in (0u64..).zip(table) {
        uc.mem_write(at + 8 * i, &d.to_le_bytes())?;
    }
    // uc_x86_mmr: selector u16, (pad), base u64, limit u32, flags u32
    let mut gdtr = [0u8; 24];
    gdtr[8..16].copy_from_slice(&at.to_le_bytes());
    gdtr[16..20].copy_from_slice(&0x1fu32.to_le_bytes());
    uc.reg_write_long(RegisterX86::GDTR, &gdtr)?;
    let cr0 = uc.reg_read(RegisterX86::CR0)?;
    uc.reg_write(RegisterX86::CR0, cr0 | 1)?;
    // SSE on, as an OS enables it (OSFXSR, OSXMMEXCPT)
    let cr4 = uc.reg_read(RegisterX86::CR4)?;
    uc.reg_write(RegisterX86::CR4, cr4 | 0x600)?;
    // MXCSR as Windows starts a thread: all exceptions masked, round to
    // nearest (the emulator starts at 0, which made MSVCR110 leave its
    // SSE2 math for the x87 paths)
    uc.reg_write(RegisterX86::MXCSR, 0x1f80)?;
    uc.reg_write(RegisterX86::CS, 0x10)?;
    for seg in [RegisterX86::SS, RegisterX86::DS, RegisterX86::ES] {
        uc.reg_write(seg, 0x18)?;
    }
    uc.reg_write(RegisterX86::FS, 0x0b)?; // entry 1, ring 3
    Ok(())
}

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

/// Maps zeroed read-write pages over `from..end` (page-aligned) where
/// neither the oracle nor the snapshot has memory.
fn map_unknown(uc: &mut Unicorn<State>, from: u64, end: u64) -> Result<()> {
    let pages = uc.get_data().pages.clone();
    let mut page = from;
    while page < end {
        let known = uc.mem_read_as_vec(page, 1).is_ok()
            || pages.as_ref().is_some_and(|p| p.chunk(page).is_some());
        if !known {
            uc.mem_map(page, PAGE, Prot::READ | Prot::WRITE)?;
        }
        page += PAGE;
    }
    Ok(())
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

/// An x87 80-bit extended value (little-endian bytes) as `fst qword`
/// stores it, NaN payloads kept: a NaN keeps its sign and the top 52
/// fraction bits, quieted (a signalling NaN gets the quiet bit, as on the
/// hardware); invalid encodings (unnormals, pseudo-NaNs, pseudo-infinities)
/// store the default NaN (`0xfff8_0000_0000_0000`); finite values round to
/// nearest even, overflowing to an infinity and underflowing to
/// subnormals or zero (the game's control word `0x027f`: exceptions
/// masked, round to nearest). Fewer than 10 bytes: a NaN.
#[must_use]
pub fn f80_to_f64(b: &[u8]) -> f64 {
    let Ok(raw) = <[u8; 10]>::try_from(b.get(..10).unwrap_or_default()) else {
        return f64::NAN;
    };
    f64::from_bits(f80_store(raw, 52, 11))
}

/// An x87 80-bit extended value as `fst dword` stores it: the f32
/// counterpart of [`f80_to_f64`] (a NaN keeps its sign and top 23 fraction
/// bits, quieted; finite values round to nearest even once, from the full
/// 64-bit mantissa).
#[must_use]
pub fn f80_to_f32(raw: [u8; 10]) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // 32 bits by construction
    f32::from_bits(f80_store(raw, 23, 8) as u32)
}

/// Whether a float the original left in `ST0` and a port's result are the
/// same bits (NaN payloads and signs included): the comparison every ST0
/// check should make. `port` as f64; an f32 result converts exactly with
/// `f64::from`, payload included.
#[must_use]
pub fn same_float(st0: f64, port: f64) -> bool {
    st0.to_bits() == port.to_bits()
}

/// Fails the test when a bridge hands a host call `n` words where the
/// game's host passes at most `max` (`ffx_bridge::Host`, "Call width"):
/// `what` names the host call (`"Host::call"`), the message the address and
/// the width. Every test host checks its calls with this before running the
/// callee, so a call the game refuses (or cuts short) can't pass a parity
/// test.
///
/// # Panics
/// When `n > max`.
pub fn check_width(what: &str, addr: u32, n: usize, max: usize) {
    assert!(
        n <= max,
        "{what}({addr:#x}) with {n} words: the game's host passes at most {max} \
         (ffx_bridge::Host, \"Call width\"); use a wider call"
    );
}

/// `raw` stored into an IEEE binary format with `frac` fraction bits and
/// `ebits` exponent bits, as the x87's `fst` does at round to nearest with
/// its exceptions masked. The result's bits, right-aligned.
fn f80_store(raw: [u8; 10], frac: u32, ebits: u32) -> u64 {
    let mant = u64::from_le_bytes(raw[..8].try_into().expect("8 bytes"));
    let se = u16::from_le_bytes([raw[8], raw[9]]);
    let exp = i32::from(se & 0x7fff);
    let sign = u64::from(se >> 15) << (frac + ebits);
    let emax = (1u64 << ebits) - 1;
    let quiet = 1u64 << (frac - 1);
    let indefinite = (1u64 << (frac + ebits)) | emax << frac | quiet;
    let integer = mant >> 63 != 0;
    if exp == 0x7fff {
        if !integer {
            return indefinite; // pseudo-NaN or pseudo-infinity
        }
        if mant << 1 == 0 {
            return sign | emax << frac;
        }
        return sign | emax << frac | quiet | (mant << 1) >> (64 - frac);
    }
    if exp != 0 && !integer {
        return indefinite; // unnormal
    }
    if mant == 0 {
        return sign;
    }
    // value = m * 2^(e - 63) with m's top bit set (denormals and
    // pseudo-denormals have the exponent of 1)
    let lz = mant.leading_zeros();
    let m = u128::from(mant << lz);
    let bias = (1i32 << (ebits - 1)) - 1;
    let mut be = exp.max(1) - 16383 - lz.cast_signed() + bias;
    let mut shift = 63 - frac.cast_signed();
    if be <= 0 {
        shift += 1 - be;
        be = 0;
    }
    if shift >= 65 {
        return sign; // below half the smallest subnormal
    }
    #[allow(clippy::cast_sign_loss)] // 11..=64
    let shift = shift as u32;
    let mut kept = m >> shift;
    let rem = m & ((1u128 << shift) - 1);
    let half = 1u128 << (shift - 1);
    if rem > half || (rem == half && kept & 1 != 0) {
        kept += 1;
    }
    #[allow(clippy::cast_possible_truncation)] // at most frac + 2 bits
    let mut kept = kept as u64;
    if be == 0 {
        // subnormal; a carry into bit `frac` makes the smallest normal
        return sign | kept;
    }
    if kept >> (frac + 1) != 0 {
        kept >>= 1;
        be += 1;
    }
    #[allow(clippy::cast_sign_loss)] // positive here
    let be = be as u64;
    if be >= emax {
        return sign | emax << frac;
    }
    sign | be << frac | (kept & ((1u64 << frac) - 1))
}

fn alloc(uc: &mut Unicorn<State>, len: u64) -> Result<u32> {
    let state = uc.get_data_mut();
    let addr = state.heap_next;
    let end = addr + len.max(1).next_multiple_of(16);
    if end > state.heap_base + HEAP_SIZE {
        return Err(Error::OutOfMemory);
    }
    state.heap_next = end;
    let a32 = u32::try_from(addr).expect("32-bit heap");
    state
        .heap_sizes
        .insert(a32, u32::try_from(len).unwrap_or(u32::MAX));
    uc.mem_write(
        addr,
        &vec![0; usize::try_from(end - addr).expect("heap size")],
    )?;
    Ok(u32::try_from(addr).expect("32-bit heap"))
}

/// Implements one import natively. Stubs are entered by `call`, so the
/// return address is at `[esp]` and the arguments follow it; the stub's own
/// `ret` instruction then returns to the caller.
fn run_stub(uc: &mut Unicorn<State>, stub: Stub, addr: u64) -> Result<()> {
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
            let data = crt::read(uc, src, len as usize)?;
            crt::write(uc, dst, &data)?;
            dst
        }
        Stub::Memset => {
            let (dst, value, len) = (arg(uc, 0)?, arg(uc, 1)?, arg(uc, 2)?);
            #[allow(clippy::cast_possible_truncation)] // memset stores the low byte
            crt::write(uc, dst, &vec![value as u8; len as usize])?;
            dst
        }
        Stub::Crt(c) => crt::run(uc, c, u32::try_from(addr).expect("32-bit stub"))?,
        Stub::Win32(api) => win32::run(uc, api)?,
        Stub::Missing(i) => return Err(Error::UnstubbedImport(uc.get_data().unstubbed[i].clone())),
    };
    uc.reg_write(RegisterX86::EAX, u64::from(ret))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_up_to_the_game_hosts_limit_passes() {
        check_width(
            "Host::call",
            0x0088_b4d0,
            ffx_bridge::CALL_WORDS,
            ffx_bridge::CALL_WORDS,
        );
    }

    #[test]
    #[should_panic(
        expected = "Host::call(0x88b4d0) with 8 words: the game's host passes at most 6"
    )]
    fn width_past_the_game_hosts_limit_fails() {
        check_width("Host::call", 0x0088_b4d0, 8, ffx_bridge::CALL_WORDS);
    }

    #[test]
    fn f80_values() {
        // 1.0: exponent 0x3fff, mantissa with the explicit integer bit.
        let one = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x3f];
        assert!((f80_to_f64(&one) - 1.0).abs() < f64::EPSILON);
        let minus_half = [0, 0, 0, 0, 0, 0, 0, 0x80, 0xfe, 0xbf];
        assert!((f80_to_f64(&minus_half) + 0.5).abs() < f64::EPSILON);
    }

    /// What this CPU's `fld tbyte; fstp qword` and `fstp dword` store.
    #[cfg(target_arch = "x86_64")]
    fn host_store(raw: [u8; 10]) -> (u64, u32) {
        let (mut d, mut f) = (0u64, 0u32);
        // SAFETY: loads 10 readable bytes and stores 8 and 4 into locals;
        // the x87 stack is left as found
        unsafe {
            std::arch::asm!(
                "fld tbyte ptr [{r}]",
                "fst qword ptr [{d}]",
                "fstp dword ptr [{f}]",
                r = in(reg) raw.as_ptr(),
                d = in(reg) &raw mut d,
                f = in(reg) &raw mut f,
                options(nostack),
            );
        }
        (d, f)
    }

    /// Bit patterns that reach every branch: random words with the
    /// exponent at the edges of both formats and every special encoding.
    fn f80_patterns() -> Vec<[u8; 10]> {
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let exps = [
            0,
            1,
            0x3f80 - 150,
            0x3f80 - 126,
            0x3f80 - 100,
            0x3fff,
            0x407f,
            0x4080,
            0x3c00 - 1075,
            0x3c00 - 1022,
            0x3c00 - 1000,
            0x43ff,
            0x4400,
            0x7ffe,
            0x7fff,
        ];
        let mut out = Vec::new();
        for i in 0..200_000u64 {
            let r = next();
            let mut mant = next();
            match i % 4 {
                0 => {}
                1 => mant &= !0x7ff,           // fits 53 bits
                2 => mant &= !((1 << 40) - 1), // fits 24 bits
                _ => mant |= 1 << 63,
            }
            let e = if i % 3 == 0 {
                #[allow(clippy::cast_possible_truncation)]
                let e = (r % 0x8000) as u16;
                e
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let k = (r % exps.len() as u64) as usize;
                #[allow(clippy::cast_possible_truncation)]
                let d = ((r >> 32) % 5) as u16;
                (exps[k] + d).saturating_sub(2).min(0x7fff)
            };
            let se = e | u16::from(r >> 63 != 0) << 15;
            let mut b = [0u8; 10];
            b[..8].copy_from_slice(&mant.to_le_bytes());
            b[8..].copy_from_slice(&se.to_le_bytes());
            out.push(b);
        }
        out
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn f80_stores_match_the_host_fpu() {
        for raw in f80_patterns() {
            let (d, f) = host_store(raw);
            assert_eq!(f80_to_f64(&raw).to_bits(), d, "f64 of {raw:02x?}");
            assert_eq!(f80_to_f32(raw).to_bits(), f, "f32 of {raw:02x?}");
        }
    }

    #[test]
    fn f80_nans_keep_sign_and_payload() {
        // a negative quiet NaN with payload bits high and low
        let mant: u64 = 0xc123_4567_89ab_cdef;
        let mut b = [0u8; 10];
        b[..8].copy_from_slice(&mant.to_le_bytes());
        b[8..].copy_from_slice(&0xffffu16.to_le_bytes());
        assert_eq!(f80_to_f64(&b).to_bits(), 0xfff8_2468_acf1_3579);
        assert_eq!(f80_to_f32(b).to_bits(), 0xffc1_2345);
        // a signalling one is quieted
        b[7] = 0x81;
        assert_eq!(f80_to_f32(b).to_bits(), 0xffc1_2345);
        // as `fld dword` leaves an f32 NaN: an f32 port result matches
        b[..5].fill(0);
        let r = Ret {
            eax: 0,
            edx: 0,
            st0: Some(f80_to_f64(&b)),
            st0_raw: Some(b),
        };
        // (a constant `f64::from(f32 NaN)` may fold to the default NaN in
        // release builds: Rust leaves NaN payloads of conversions open)
        assert!(r.st0_is(f64::from_bits(0xfff8_2468_a000_0000)));
        assert_eq!(r.st0_f32().map(f32::to_bits), Some(0xffc1_2345));
        assert!(!r.st0_is(f64::NAN));
    }

    #[test]
    fn gaps_avoid_taken_ranges() {
        let mut t = vec![(0x1000_0000, 0x1100_0000), (0x1180_0000, 0x1200_0000)];
        assert_eq!(gap(&mut t, 0x80_0000).unwrap(), 0x1100_0000);
        assert_eq!(gap(&mut t, 0x10_0000).unwrap(), 0x1200_0000);
    }
}

/// Native stand-ins for the C runtime and Win32 imports `FFX.exe` calls
/// (MSVCR110 semantics, single-threaded, the "C" locale).
///
/// A stub runs before the code at its address: a plain `ret` for cdecl, `ret
/// n` for stdcall. Stubs that return a `double` in `ST0` store it in the
/// 8 bytes after their code (`stub + 8`), and their code is `fld qword
/// [stub + 8]; ret`. `_CIsqrt` and `_CIfmod` are x87 code themselves
/// (`fsqrt`, an `fprem` loop): exact by construction.
// short names follow the C library's own (s, d, e, p, n, ...)
#[allow(clippy::many_single_char_names)]
mod crt {
    use super::{Error, PAGE, Result, State, alloc, map_from_snapshot};
    use unicorn_engine::{RegisterX86, Unicorn};

    /// One CRT or Win32 import.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) enum Crt {
        Strdup,
        Stricmp,
        Strnicmp,
        Strncmp,
        Strncpy,
        Strncat,
        Strchr,
        Strrchr,
        Strstr,
        Strpbrk,
        Strspn,
        Strlwr,
        Tolower,
        Toupper,
        Atoi,
        Strtol,
        Strtoul,
        Atof,
        Strtod,
        Memchr,
        Abs,
        /// `is*(c)`: the `_ctype` mask bits it tests.
        Ctype(u16),
        Sprintf,
        Snprintf,
        Vsprintf,
        Vsnprintf,
        SprintfS,
        VsnprintfS,
        StrcpyS,
        StrcatS,
        StrncpyS,
        MemcpyS,
        Finite,
        Isnan,
        Floor,
        Ceil,
        /// `_CIsqrt`: its code is `fsqrt; ret`.
        CiSqrt,
        /// `_CIfmod`: its code is an `fprem` loop.
        CiFmod,
        Rand,
        Srand,
        /// `InitializeCriticalSection*`, `Enter`/`Leave`/`DeleteCriticalSection`:
        /// nothing to do on one thread; stdcall with this many arguments,
        /// returning this.
        Lock(u8, u32),
        GetCurrentThreadId,
        /// `_wassert(message, file, line)`: stops with [`Error::Assertion`].
        Wassert,
    }

    /// `_ctype` bits.
    const UPPER: u16 = 0x1;
    const LOWER: u16 = 0x2;
    const DIGIT: u16 = 0x4;
    const SPACE: u16 = 0x8;
    const PUNCT: u16 = 0x10;
    const CONTROL: u16 = 0x20;
    const BLANK: u16 = 0x40;
    const HEX: u16 = 0x80;
    const ALPHA: u16 = 0x100;

    /// The stub for an import name, if there is one.
    pub(super) fn lookup(name: &str) -> Option<Crt> {
        Some(match name {
            "_strdup" | "strdup" => Crt::Strdup,
            "_stricmp" => Crt::Stricmp,
            "_strnicmp" => Crt::Strnicmp,
            "strncmp" => Crt::Strncmp,
            "strncpy" => Crt::Strncpy,
            "strncat" => Crt::Strncat,
            "strchr" => Crt::Strchr,
            "strrchr" => Crt::Strrchr,
            "strstr" => Crt::Strstr,
            "strpbrk" => Crt::Strpbrk,
            "strspn" => Crt::Strspn,
            "_strlwr" => Crt::Strlwr,
            "tolower" => Crt::Tolower,
            "toupper" => Crt::Toupper,
            "atoi" | "atol" => Crt::Atoi,
            "strtol" => Crt::Strtol,
            "strtoul" => Crt::Strtoul,
            "atof" => Crt::Atof,
            "strtod" => Crt::Strtod,
            "memchr" => Crt::Memchr,
            "abs" => Crt::Abs,
            "isalpha" => Crt::Ctype(ALPHA | UPPER | LOWER),
            "isupper" => Crt::Ctype(UPPER),
            "islower" => Crt::Ctype(LOWER),
            "isdigit" => Crt::Ctype(DIGIT),
            "isxdigit" => Crt::Ctype(HEX),
            "isspace" => Crt::Ctype(SPACE),
            "ispunct" => Crt::Ctype(PUNCT),
            "isalnum" => Crt::Ctype(ALPHA | UPPER | LOWER | DIGIT),
            "isgraph" => Crt::Ctype(PUNCT | ALPHA | UPPER | LOWER | DIGIT),
            "iscntrl" => Crt::Ctype(CONTROL),
            "sprintf" => Crt::Sprintf,
            "_snprintf" => Crt::Snprintf,
            "vsprintf" => Crt::Vsprintf,
            "_vsnprintf" => Crt::Vsnprintf,
            "sprintf_s" => Crt::SprintfS,
            "_vsnprintf_s" => Crt::VsnprintfS,
            "strcpy_s" => Crt::StrcpyS,
            "strcat_s" => Crt::StrcatS,
            "strncpy_s" => Crt::StrncpyS,
            "memcpy_s" | "memmove_s" => Crt::MemcpyS,
            "_finite" => Crt::Finite,
            "_isnan" => Crt::Isnan,
            "floor" => Crt::Floor,
            "ceil" => Crt::Ceil,
            "_CIsqrt" => Crt::CiSqrt,
            "_CIfmod" => Crt::CiFmod,
            "rand" => Crt::Rand,
            "srand" => Crt::Srand,
            "InitializeCriticalSection"
            | "EnterCriticalSection"
            | "LeaveCriticalSection"
            | "DeleteCriticalSection" => Crt::Lock(1, 0),
            "TryEnterCriticalSection" => Crt::Lock(1, 1),
            "InitializeCriticalSectionAndSpinCount" => Crt::Lock(2, 1),
            "InitializeCriticalSectionEx" => Crt::Lock(3, 1),
            "GetCurrentThreadId" => Crt::GetCurrentThreadId,
            "_wassert" => Crt::Wassert,
            _ => return None,
        })
    }

    /// The code at the stub's address.
    pub(super) fn code(crt: Crt, stub: u32) -> Vec<u8> {
        match crt {
            Crt::Atof | Crt::Strtod | Crt::Floor | Crt::Ceil => {
                // fld qword [stub + 8]; ret
                let mut c = vec![0xdd, 0x05];
                c.extend((stub + 8).to_le_bytes());
                c.push(0xc3);
                c
            }
            // fsqrt; ret
            Crt::CiSqrt => vec![0xd9, 0xfa, 0xc3],
            // fxch; 1: fprem; fnstsw ax; sahf; jp 1b; fstp st(1); ret
            Crt::CiFmod => vec![
                0xd9, 0xc9, 0xd9, 0xf8, 0xdf, 0xe0, 0x9e, 0x7a, 0xf9, 0xdd, 0xd9, 0xc3,
            ],
            Crt::Lock(n, _) => vec![0xc2, 4 * n, 0],
            _ => vec![0xc3],
        }
    }

    // ---- emulated memory

    fn ensure(uc: &mut Unicorn<State>, addr: u32, len: usize) {
        let mut a = u64::from(addr) & !(PAGE - 1);
        let end = u64::from(addr) + len as u64;
        while a < end {
            if uc.mem_read_as_vec(a, 1).is_err() {
                map_from_snapshot(uc, a);
            }
            a += PAGE;
        }
    }

    pub(super) fn read(uc: &mut Unicorn<State>, addr: u32, len: usize) -> Result<Vec<u8>> {
        ensure(uc, addr, len);
        uc.mem_read_as_vec(u64::from(addr), len)
            .map_err(|_| Error::BadAccess(u64::from(addr)))
    }

    /// Writes and marks the pages dirty, as emulated stores do.
    pub(super) fn write(uc: &mut Unicorn<State>, addr: u32, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        ensure(uc, addr, bytes.len());
        uc.mem_write(u64::from(addr), bytes)
            .map_err(|_| Error::BadAccess(u64::from(addr)))?;
        let last = addr.wrapping_add(u32::try_from(bytes.len() - 1).unwrap_or(u32::MAX));
        let d = &mut uc.get_data_mut().dirty;
        for p in (addr >> 12)..=(last >> 12) {
            d.insert(p);
        }
        Ok(())
    }

    fn u32_at(uc: &mut Unicorn<State>, addr: u32) -> Result<u32> {
        let b = read(uc, addr, 4)?;
        Ok(u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }

    /// The NUL-terminated string at `p` (without the NUL).
    pub(super) fn cstr(uc: &mut Unicorn<State>, p: u32) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut a = p;
        loop {
            let b = read(uc, a, 1)?[0];
            if b == 0 {
                return Ok(out);
            }
            out.push(b);
            a = a.wrapping_add(1);
        }
    }

    /// At most `n` bytes of the string at `p`.
    fn cstr_n(uc: &mut Unicorn<State>, p: u32, n: u32) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut a = p;
        for _ in 0..n {
            let b = read(uc, a, 1)?[0];
            if b == 0 {
                break;
            }
            out.push(b);
            a = a.wrapping_add(1);
        }
        Ok(out)
    }

    fn put_cstr(uc: &mut Unicorn<State>, at: u32, s: &[u8]) -> Result<()> {
        let mut v = s.to_vec();
        v.push(0);
        write(uc, at, &v)
    }

    // ---- character classes ("C" locale)

    fn ctype(c: u8) -> u16 {
        match c {
            // the "C" table has no separate alpha bit for letters, so
            // isalpha('a') is 2 (checked against MSVCR110.dll)
            b'A'..=b'F' => UPPER | HEX,
            b'G'..=b'Z' => UPPER,
            b'a'..=b'f' => LOWER | HEX,
            b'g'..=b'z' => LOWER,
            b'0'..=b'9' => DIGIT | HEX,
            b' ' => SPACE | BLANK,
            0x09..=0x0d => SPACE | CONTROL,
            0x00..=0x1f | 0x7f => CONTROL,
            0x21..=0x7e => PUNCT,
            _ => 0,
        }
    }

    fn lower(c: u8) -> u8 {
        c.to_ascii_lowercase()
    }

    /// `_stricmp`/`_strnicmp`: compares lowercased bytes.
    fn icmp(a: &[u8], b: &[u8]) -> i32 {
        for i in 0.. {
            let (x, y) = (
                a.get(i).copied().map_or(0, lower),
                b.get(i).copied().map_or(0, lower),
            );
            if x != y || x == 0 {
                return i32::from(x) - i32::from(y);
            }
        }
        unreachable!()
    }

    // ---- numbers

    /// `strtol`/`strtoul`: (value, bytes used, overflowed).
    fn strtol(s: &[u8], base: u32, signed: bool) -> (u32, usize) {
        let mut i = 0;
        while i < s.len() && ctype(s[i]) & SPACE != 0 {
            i += 1;
        }
        let mut neg = false;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            neg = s[i] == b'-';
            i += 1;
        }
        let mut base = base;
        let start = i;
        // a `0x` prefix commits to hex: with no digit after it nothing is
        // converted (MSVCR110; glibc would take the `0`)
        if (base == 0 || base == 16) && i + 1 < s.len() && s[i] == b'0' && (s[i + 1] | 0x20) == b'x'
        {
            i += 2;
            base = 16;
        } else if base == 0 {
            base = if s.get(i) == Some(&b'0') { 8 } else { 10 };
        }
        let mut v: u64 = 0;
        let mut over = false;
        let digits_start = i;
        while let Some(d) = s.get(i).and_then(|&c| char::from(c).to_digit(base)) {
            v = v * u64::from(base) + u64::from(d);
            if v > u64::from(u32::MAX) {
                over = true;
                v = u64::from(u32::MAX);
            }
            i += 1;
        }
        if i == digits_start {
            // no digits: nothing converted, the end is the start
            let _ = start;
            return (0, 0);
        }
        #[allow(clippy::cast_possible_truncation)] // clamped
        let r = if signed {
            let limit = if neg { 0x8000_0000u64 } else { 0x7fff_ffff };
            if over || v > limit {
                if neg { 0x8000_0000 } else { 0x7fff_ffff }
            } else if neg {
                (v as u32).wrapping_neg()
            } else {
                v as u32
            }
        } else if over {
            u32::MAX
        } else if neg {
            (v as u32).wrapping_neg()
        } else {
            v as u32
        };
        (r, i)
    }

    /// `strtod`/`atof`: (value, bytes used). Decimal and exponent forms (no
    /// hex), correctly rounded. MSVCR110 agrees on everything tried but a
    /// value just above half the smallest subnormal, which it flushes to 0.
    fn strtod(s: &[u8]) -> (f64, usize) {
        let mut i = 0;
        while i < s.len() && ctype(s[i]) & SPACE != 0 {
            i += 1;
        }
        let start = i;
        if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
            i += 1;
        }
        let mut digits = 0;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        if i < s.len() && s[i] == b'.' {
            i += 1;
            while i < s.len() && s[i].is_ascii_digit() {
                i += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return (0.0, 0);
        }
        if i < s.len() && (s[i] | 0x20) == b'e' {
            let mut j = i + 1;
            if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
                j += 1;
            }
            if j < s.len() && s[j].is_ascii_digit() {
                while j < s.len() && s[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        let text = std::str::from_utf8(&s[start..i]).unwrap_or("0");
        let text = text.strip_suffix('.').unwrap_or(text);
        (text.parse().unwrap_or(0.0), i)
    }

    // ---- printf

    /// Where the variable arguments come from.
    struct Args {
        at: u32,
    }

    impl Args {
        fn u32(&mut self, uc: &mut Unicorn<State>) -> Result<u32> {
            let v = u32_at(uc, self.at)?;
            self.at = self.at.wrapping_add(4);
            Ok(v)
        }
        fn u64(&mut self, uc: &mut Unicorn<State>) -> Result<u64> {
            let lo = self.u32(uc)?;
            let hi = self.u32(uc)?;
            Ok(u64::from(hi) << 32 | u64::from(lo))
        }
    }

    /// Rounds the ASCII digit string to `n` digits the CRT's way (`_fptostr`):
    /// up when the next digit is `'5'` or more, carrying through `'9'`;
    /// shorter strings are padded with `'0'`. Returns whether a carry made
    /// a new leading digit.
    fn round_digits(d: &mut Vec<u8>, n: usize) -> bool {
        if d.len() <= n {
            d.resize(n, b'0');
            return false;
        }
        let up = d[n] >= b'5';
        d.truncate(n);
        if !up {
            return false;
        }
        let mut i = n;
        loop {
            if i == 0 {
                d.insert(0, b'1');
                d.truncate(n.max(1));
                return true;
            }
            i -= 1;
            if d[i] == b'9' {
                d[i] = b'0';
            } else {
                d[i] += 1;
                return false;
            }
        }
    }

    /// A double's decimal digits as MSVCR110 sees them: 17 significant
    /// digits, correctly rounded, and the decimal exponent of the first.
    /// Infinities and NaNs become the CRT's `1#INF`, `1#QNAN`, `1#SNAN`
    /// and `1#IND` "digits" (exponent 0), which then round like digits
    /// (`%.2f` of infinity is `1.#J`).
    fn digits17(x: f64) -> (Vec<u8>, i32) {
        if x.is_infinite() {
            return (b"1#INF".to_vec(), 0);
        }
        if x.is_nan() {
            let b = x.to_bits();
            let quiet = b & (1 << 51) != 0;
            let indefinite = b == 0xfff8_0000_0000_0000;
            let s: &[u8] = if indefinite {
                b"1#IND"
            } else if quiet {
                b"1#QNAN"
            } else {
                b"1#SNAN"
            };
            return (s.to_vec(), 0);
        }
        if x == 0.0 {
            return (vec![b'0'; 17], 0);
        }
        let s = format!("{:.16e}", x.abs());
        let (m, e) = s.split_once('e').expect("exponent");
        let d: Vec<u8> = m.bytes().filter(u8::is_ascii_digit).collect();
        (d, e.parse().expect("exponent digits"))
    }

    /// `%f`: [-]ddd.ddd with `prec` decimals.
    fn fmt_f(x: f64, prec: usize, alt: bool) -> Vec<u8> {
        let (mut d, mut e) = digits17(x);
        let special = !x.is_finite();
        let mut out = Vec::new();
        let int_digits = if special { 1 } else { e + 1 };
        let keep = int_digits + i32::try_from(prec).unwrap_or(i32::MAX);
        if keep < 0 {
            d = vec![b'0'];
            e = -1;
        } else {
            #[allow(clippy::cast_sign_loss)] // checked above
            if round_digits(&mut d, keep as usize) {
                e += 1;
            }
        }
        let int_digits = if special { 1 } else { e + 1 };
        if int_digits <= 0 {
            out.push(b'0');
            if prec > 0 || alt {
                out.push(b'.');
            }
            #[allow(clippy::cast_sign_loss)]
            let zeros = (-int_digits) as usize;
            for k in 0..prec {
                out.push(if k < zeros {
                    b'0'
                } else {
                    *d.get(k - zeros).unwrap_or(&b'0')
                });
            }
        } else {
            #[allow(clippy::cast_sign_loss)]
            let n = int_digits as usize;
            for k in 0..n {
                out.push(*d.get(k).unwrap_or(&b'0'));
            }
            if prec > 0 || alt {
                out.push(b'.');
            }
            for k in 0..prec {
                out.push(*d.get(n + k).unwrap_or(&b'0'));
            }
        }
        out
    }

    /// `%e`: d.ddde+XXX (three exponent digits).
    fn fmt_e(x: f64, prec: usize, alt: bool, upper: bool) -> Vec<u8> {
        let (mut d, mut e) = digits17(x);
        if round_digits(&mut d, prec + 1) {
            e += 1;
        }
        let mut out = vec![d[0]];
        if prec > 0 || alt {
            out.push(b'.');
        }
        out.extend(&d[1..=prec]);
        out.push(if upper { b'E' } else { b'e' });
        out.extend(format!("{}{:03}", if e < 0 { '-' } else { '+' }, e.abs()).bytes());
        out
    }

    /// `%g`: `%e` when the exponent is below -4 or at least the precision,
    /// else `%f`; trailing zeros dropped unless `#`.
    fn fmt_g(x: f64, prec: usize, alt: bool, upper: bool) -> Vec<u8> {
        let p = prec.max(1);
        let (mut d, mut e) = digits17(x);
        if x == 0.0 {
            // the CRT puts zero's decimal point before its digit
            e = -1;
        } else if round_digits(&mut d, p) {
            e += 1;
        }
        let mut out = if e < -4 || e >= i32::try_from(p).unwrap_or(i32::MAX) {
            fmt_e(x, p - 1, alt, upper)
        } else {
            #[allow(clippy::cast_sign_loss)]
            let decimals = (i32::try_from(p).unwrap_or(0) - 1 - e).max(0) as usize;
            fmt_f(x, decimals, alt)
        };
        if !alt {
            let epos = out.iter().position(|&c| c == b'e' || c == b'E');
            let (mut mant, exp) = match epos {
                Some(i) => (out[..i].to_vec(), out[i..].to_vec()),
                None => (out.clone(), Vec::new()),
            };
            if mant.contains(&b'.') {
                while mant.last() == Some(&b'0') {
                    mant.pop();
                }
                if mant.last() == Some(&b'.') {
                    mant.pop();
                }
            }
            mant.extend(exp);
            out = mant;
        }
        out
    }

    /// Formats `fmt` with arguments read from `args`, as MSVCR110's
    /// `sprintf` family: flags `- + space # 0`, width and precision
    /// (`*` too), sizes `h l ll I64 L`, conversions `d i u o x X c s p e E
    /// f g G %`. `%s` of a null pointer is `(null)`. `%f`, `%e` and `%g`
    /// use at most 17 significant digits (correctly rounded), then zeros;
    /// infinities and NaNs print as `1.#INF`, `1.#QNAN`, `-1.#IND` digit
    /// strings.
    #[allow(clippy::too_many_lines)]
    fn format(uc: &mut Unicorn<State>, fmt: &[u8], args: &mut Args) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < fmt.len() {
            let c = fmt[i];
            i += 1;
            if c != b'%' {
                out.push(c);
                continue;
            }
            let (mut left, mut plus, mut space, mut alt, mut zero) =
                (false, false, false, false, false);
            while i < fmt.len() {
                match fmt[i] {
                    b'-' => left = true,
                    b'+' => plus = true,
                    b' ' => space = true,
                    b'#' => alt = true,
                    b'0' => zero = true,
                    _ => break,
                }
                i += 1;
            }
            let mut width: usize = 0;
            if fmt.get(i) == Some(&b'*') {
                let w = args.u32(uc)?.cast_signed();
                if w < 0 {
                    left = true;
                }
                width = w.unsigned_abs() as usize;
                i += 1;
            } else {
                while let Some(d) = fmt.get(i).filter(|c| c.is_ascii_digit()) {
                    width = width * 10 + usize::from(d - b'0');
                    i += 1;
                }
            }
            let mut prec: Option<usize> = None;
            if fmt.get(i) == Some(&b'.') {
                i += 1;
                if fmt.get(i) == Some(&b'*') {
                    let p = args.u32(uc)?.cast_signed();
                    prec = (p >= 0).then(|| p.unsigned_abs() as usize);
                    i += 1;
                } else {
                    let mut p = 0;
                    while let Some(d) = fmt.get(i).filter(|c| c.is_ascii_digit()) {
                        p = p * 10 + usize::from(d - b'0');
                        i += 1;
                    }
                    prec = Some(p);
                }
            }
            let mut size = 0u8; // 0 int, b'h', b'l', b'q' (64-bit), b'L'
            loop {
                match fmt.get(i) {
                    Some(b'h') => size = b'h',
                    Some(b'l') => size = if size == b'l' { b'q' } else { b'l' },
                    Some(b'L') => size = b'L',
                    Some(b'I') if fmt.get(i + 1..i + 3) == Some(b"64") => {
                        size = b'q';
                        i += 2;
                    }
                    Some(b'I') if fmt.get(i + 1..i + 3) == Some(b"32") => {
                        i += 2;
                    }
                    Some(b'I') => {}
                    _ => break,
                }
                i += 1;
            }
            let Some(&conv) = fmt.get(i) else { break };
            i += 1;
            let mut sign: Vec<u8> = Vec::new();
            let mut prefix: Vec<u8> = Vec::new();
            let mut body: Vec<u8>;
            let mut numeric_zero_pad = zero && !left;
            match conv {
                b'%' => {
                    out.push(b'%');
                    continue;
                }
                b'c' => {
                    #[allow(clippy::cast_possible_truncation)] // the low byte
                    let v = args.u32(uc)? as u8;
                    body = vec![v];
                    numeric_zero_pad = zero && !left;
                }
                b's' => {
                    let p = args.u32(uc)?;
                    body = if p == 0 {
                        b"(null)".to_vec()
                    } else if let Some(n) = prec {
                        cstr_n(uc, p, u32::try_from(n).unwrap_or(u32::MAX))?
                    } else {
                        cstr(uc, p)?
                    };
                    if let Some(n) = prec {
                        body.truncate(n);
                    }
                    // `0` pads strings with zeros too
                    numeric_zero_pad = zero && !left;
                }
                b'd' | b'i' | b'u' | b'o' | b'x' | b'X' | b'p' => {
                    let raw: u64 = if size == b'q' {
                        args.u64(uc)?
                    } else {
                        u64::from(args.u32(uc)?)
                    };
                    let signed = matches!(conv, b'd' | b'i');
                    let (neg, mag): (bool, u64) = if size == b'q' {
                        if signed && raw.cast_signed() < 0 {
                            (true, raw.cast_signed().unsigned_abs())
                        } else {
                            (false, raw)
                        }
                    } else {
                        #[allow(clippy::cast_possible_truncation)] // 32-bit
                        let v = raw as u32;
                        #[allow(clippy::cast_possible_truncation)]
                        let v = if size == b'h' {
                            if signed {
                                i32::from((v as u16).cast_signed()).cast_unsigned()
                            } else {
                                u32::from(v as u16)
                            }
                        } else {
                            v
                        };
                        if signed && v.cast_signed() < 0 {
                            (true, u64::from(v.cast_signed().unsigned_abs()))
                        } else {
                            (false, u64::from(v))
                        }
                    };
                    body = match conv {
                        b'o' => format!("{mag:o}"),
                        b'x' => format!("{mag:x}"),
                        b'X' => format!("{mag:X}"),
                        b'p' => format!("{mag:08X}"),
                        _ => format!("{mag}"),
                    }
                    .into_bytes();
                    if let Some(p) = prec {
                        if p == 0 && mag == 0 {
                            body.clear();
                        }
                        while body.len() < p {
                            body.insert(0, b'0');
                        }
                        numeric_zero_pad = false;
                    }
                    if signed {
                        if neg {
                            sign.push(b'-');
                        } else if plus {
                            sign.push(b'+');
                        } else if space {
                            sign.push(b' ');
                        }
                    }
                    if alt && mag != 0 {
                        match conv {
                            b'x' => prefix.extend(b"0x"),
                            b'X' => prefix.extend(b"0X"),
                            b'o' if body.first() != Some(&b'0') => body.insert(0, b'0'),
                            _ => {}
                        }
                    }
                }
                b'e' | b'E' | b'f' | b'g' | b'G' => {
                    let x = f64::from_bits(args.u64(uc)?);
                    let p = prec.unwrap_or(6);
                    body = match conv {
                        b'f' => fmt_f(x, p, alt),
                        b'e' | b'E' => fmt_e(x, p, alt, conv == b'E'),
                        _ => fmt_g(x, p, alt, conv == b'G'),
                    };
                    if x.is_sign_negative() {
                        sign.push(b'-');
                    } else if plus {
                        sign.push(b'+');
                    } else if space {
                        sign.push(b' ');
                    }
                }
                other => {
                    // unknown: printed as is
                    out.push(other);
                    continue;
                }
            }
            let len = sign.len() + prefix.len() + body.len();
            let pad = width.saturating_sub(len);
            if left {
                out.extend(&sign);
                out.extend(&prefix);
                out.extend(&body);
                out.extend(std::iter::repeat_n(b' ', pad));
            } else if numeric_zero_pad {
                out.extend(&sign);
                out.extend(&prefix);
                out.extend(std::iter::repeat_n(b'0', pad));
                out.extend(&body);
            } else {
                out.extend(std::iter::repeat_n(b' ', pad));
                out.extend(&sign);
                out.extend(&prefix);
                out.extend(&body);
            }
        }
        Ok(out)
    }

    /// `_snprintf`-style output into `count` bytes: the text, a NUL when
    /// there is room; -1 when it did not fit.
    fn put_counted(uc: &mut Unicorn<State>, buf: u32, count: u32, s: &[u8]) -> Result<u32> {
        let n = count as usize;
        match s.len().cmp(&n) {
            std::cmp::Ordering::Less => {
                put_cstr(uc, buf, s)?;
                Ok(u32::try_from(s.len()).expect("length"))
            }
            std::cmp::Ordering::Equal => {
                write(uc, buf, s)?;
                Ok(count)
            }
            std::cmp::Ordering::Greater => {
                write(uc, buf, &s[..n])?;
                Ok(u32::MAX)
            }
        }
    }

    fn put_double(uc: &mut Unicorn<State>, stub: u32, v: f64) -> Result<()> {
        // the stub's own slot: not a game page, so not marked dirty
        uc.mem_write(u64::from(stub) + 8, &v.to_le_bytes())
            .map_err(|_| Error::BadAccess(u64::from(stub)))
    }

    /// Runs one stub; returns EAX.
    #[allow(clippy::too_many_lines)]
    pub(super) fn run(uc: &mut Unicorn<State>, crt: Crt, stub: u32) -> Result<u32> {
        let esp = u32::try_from(uc.reg_read(RegisterX86::ESP)?).expect("32-bit");
        let mut a = [0u32; 6];
        for (k, v) in (0u32..).zip(a.iter_mut()) {
            *v = u32_at(uc, esp + 4 + 4 * k)?;
        }
        Ok(match crt {
            Crt::Strdup => {
                let s = cstr(uc, a[0])?;
                let p = alloc(uc, s.len() as u64 + 1)?;
                put_cstr(uc, p, &s)?;
                p
            }
            Crt::Stricmp => {
                let (a, b) = (cstr(uc, a[0])?, cstr(uc, a[1])?);
                icmp(&a, &b).cast_unsigned()
            }
            Crt::Strnicmp => {
                let n = a[2];
                let (a, b) = (cstr_n(uc, a[0], n)?, cstr_n(uc, a[1], n)?);
                icmp(&a, &b).cast_unsigned()
            }
            Crt::Strncmp => {
                let n = a[2];
                let (a, b) = (cstr_n(uc, a[0], n)?, cstr_n(uc, a[1], n)?);
                let mut r = 0i32;
                for k in 0..=a.len().max(b.len()) {
                    if k >= n as usize {
                        break;
                    }
                    let (x, y) = (
                        a.get(k).copied().unwrap_or(0),
                        b.get(k).copied().unwrap_or(0),
                    );
                    if x != y {
                        r = if x < y { -1 } else { 1 };
                        break;
                    }
                    if x == 0 {
                        break;
                    }
                }
                r.cast_unsigned()
            }
            Crt::Strncpy => {
                let (dst, src, n) = (a[0], a[1], a[2]);
                let mut s = cstr_n(uc, src, n)?;
                s.resize(n as usize, 0);
                write(uc, dst, &s)?;
                dst
            }
            Crt::Strncat => {
                let (dst, src, n) = (a[0], a[1], a[2]);
                let d = cstr(uc, dst)?;
                let s = cstr_n(uc, src, n)?;
                put_cstr(uc, dst + u32::try_from(d.len()).expect("length"), &s)?;
                dst
            }
            Crt::Strchr | Crt::Strrchr => {
                let (p, c) = (a[0], a[1]);
                #[allow(clippy::cast_possible_truncation)] // char
                let c = c as u8;
                let mut s = cstr(uc, p)?;
                s.push(0);
                let at = if crt == Crt::Strchr {
                    s.iter().position(|&b| b == c)
                } else {
                    s.iter().rposition(|&b| b == c)
                };
                at.map_or(0, |k| p + u32::try_from(k).expect("offset"))
            }
            Crt::Strstr => {
                let (p, q) = (a[0], a[1]);
                let (s, n) = (cstr(uc, p)?, cstr(uc, q)?);
                if n.is_empty() {
                    p
                } else {
                    s.windows(n.len())
                        .position(|w| w == n.as_slice())
                        .map_or(0, |k| p + u32::try_from(k).expect("offset"))
                }
            }
            Crt::Strpbrk => {
                let (p, q) = (a[0], a[1]);
                let (s, set) = (cstr(uc, p)?, cstr(uc, q)?);
                s.iter()
                    .position(|b| set.contains(b))
                    .map_or(0, |k| p + u32::try_from(k).expect("offset"))
            }
            Crt::Strspn => {
                let (s, set) = (cstr(uc, a[0])?, cstr(uc, a[1])?);
                u32::try_from(s.iter().take_while(|b| set.contains(b)).count()).expect("length")
            }
            Crt::Strlwr => {
                let p = a[0];
                let s: Vec<u8> = cstr(uc, p)?.into_iter().map(lower).collect();
                write(uc, p, &s)?;
                p
            }
            Crt::Tolower | Crt::Toupper => {
                let c = a[0];
                if c < 0x80 {
                    #[allow(clippy::cast_possible_truncation)] // ASCII
                    let b = c as u8;
                    u32::from(if crt == Crt::Tolower {
                        b.to_ascii_lowercase()
                    } else {
                        b.to_ascii_uppercase()
                    })
                } else {
                    c
                }
            }
            Crt::Atoi => {
                let s = cstr(uc, a[0])?;
                strtol(&s, 10, true).0
            }
            Crt::Strtol | Crt::Strtoul => {
                let (p, end, base) = (a[0], a[1], a[2]);
                let s = cstr(uc, p)?;
                let (v, used) = strtol(&s, base, crt == Crt::Strtol);
                if end != 0 {
                    let e = p + u32::try_from(used).expect("offset");
                    write(uc, end, &e.to_le_bytes())?;
                }
                v
            }
            Crt::Atof | Crt::Strtod => {
                let p = a[0];
                let s = cstr(uc, p)?;
                let (v, used) = strtod(&s);
                if crt == Crt::Strtod {
                    let end = a[1];
                    if end != 0 {
                        let e = p + u32::try_from(used).expect("offset");
                        write(uc, end, &e.to_le_bytes())?;
                    }
                }
                put_double(uc, stub, v)?;
                0
            }
            Crt::Memchr => {
                let (p, c, n) = (a[0], a[1], a[2]);
                #[allow(clippy::cast_possible_truncation)] // char
                let c = c as u8;
                let b = read(uc, p, n as usize)?;
                b.iter()
                    .position(|&x| x == c)
                    .map_or(0, |k| p + u32::try_from(k).expect("offset"))
            }
            Crt::Abs => a[0].cast_signed().wrapping_abs().cast_unsigned(),
            Crt::Ctype(mask) => {
                let c = a[0];
                #[allow(clippy::cast_possible_truncation)] // checked
                let bits = if c < 0x80 { ctype(c as u8) } else { 0 };
                u32::from(bits & mask)
            }
            Crt::Sprintf => {
                let (buf, fmt) = (a[0], a[1]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: esp + 12 })?;
                put_cstr(uc, buf, &s)?;
                u32::try_from(s.len()).expect("length")
            }
            Crt::Vsprintf => {
                let (buf, fmt, va) = (a[0], a[1], a[2]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: va })?;
                put_cstr(uc, buf, &s)?;
                u32::try_from(s.len()).expect("length")
            }
            Crt::Snprintf => {
                let (buf, count, fmt) = (a[0], a[1], a[2]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: esp + 16 })?;
                put_counted(uc, buf, count, &s)?
            }
            Crt::Vsnprintf => {
                let (buf, count, fmt, va) = (a[0], a[1], a[2], a[3]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: va })?;
                put_counted(uc, buf, count, &s)?
            }
            Crt::SprintfS => {
                let (buf, size, fmt) = (a[0], a[1], a[2]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: esp + 16 })?;
                if s.len() >= size as usize {
                    // the invalid-parameter path: what fits (size - 1 bytes
                    // and a NUL) is written, then the first byte cleared;
                    // -1 (as MSVCR110.dll does, with a returning handler)
                    if size > 0 {
                        put_cstr(uc, buf, &s[..size as usize - 1])?;
                        write(uc, buf, &[0])?;
                    }
                    u32::MAX
                } else {
                    put_cstr(uc, buf, &s)?;
                    u32::try_from(s.len()).expect("length")
                }
            }
            Crt::VsnprintfS => {
                let (buf, size, count, fmt, va) = (a[0], a[1], a[2], a[3], a[4]);
                let f = cstr(uc, fmt)?;
                let s = format(uc, &f, &mut Args { at: va })?;
                // _TRUNCATE (-1) or a count below the size: truncate, -1
                let room = if count == u32::MAX {
                    size.saturating_sub(1)
                } else {
                    count.min(size.saturating_sub(1))
                };
                if s.len() <= room as usize {
                    put_cstr(uc, buf, &s)?;
                    u32::try_from(s.len()).expect("length")
                } else {
                    put_cstr(uc, buf, &s[..room as usize])?;
                    u32::MAX
                }
            }
            Crt::StrcpyS | Crt::StrcatS | Crt::StrncpyS => {
                let (dst, size, src) = (a[0], a[1], a[2]);
                let mut s = if crt == Crt::StrncpyS {
                    let n = a[3];
                    cstr_n(uc, src, n)?
                } else {
                    cstr(uc, src)?
                };
                if crt == Crt::StrcatS {
                    let mut d = cstr(uc, dst)?;
                    d.extend(s);
                    s = d;
                }
                if s.len() >= size as usize {
                    if size > 0 {
                        write(uc, dst, &[0])?;
                    }
                    34 // ERANGE
                } else {
                    put_cstr(uc, dst, &s)?;
                    0
                }
            }
            Crt::MemcpyS => {
                let (dst, size, src, n) = (a[0], a[1], a[2], a[3]);
                if n > size {
                    34
                } else {
                    let b = read(uc, src, n as usize)?;
                    write(uc, dst, &b)?;
                    0
                }
            }
            Crt::Finite => {
                u32::from(f64::from_bits(u64::from(a[1]) << 32 | u64::from(a[0])).is_finite())
            }
            Crt::Isnan => {
                u32::from(f64::from_bits(u64::from(a[1]) << 32 | u64::from(a[0])).is_nan())
            }
            Crt::Floor | Crt::Ceil => {
                let x = f64::from_bits(u64::from(a[1]) << 32 | u64::from(a[0]));
                let v = if crt == Crt::Floor {
                    x.floor()
                } else {
                    x.ceil()
                };
                put_double(uc, stub, v)?;
                0
            }
            Crt::CiSqrt | Crt::CiFmod => {
                // x87 code: EAX as it was
                u32::try_from(uc.reg_read(RegisterX86::EAX)?).expect("32-bit")
            }
            Crt::Rand => {
                let st = &mut uc.get_data_mut().rand_seed;
                *st = st.wrapping_mul(214_013).wrapping_add(2_531_011);
                (*st >> 16) & 0x7fff
            }
            Crt::Srand => {
                let seed = a[0];
                uc.get_data_mut().rand_seed = seed;
                0
            }
            Crt::Lock(_, r) => r,
            Crt::GetCurrentThreadId => 1,
            Crt::Wassert => {
                let mut text = |p: u32| -> Result<String> {
                    let mut w = Vec::new();
                    for k in 0..4096u32 {
                        let b = read(uc, p.wrapping_add(2 * k), 2)?;
                        match u16::from_le_bytes([b[0], b[1]]) {
                            0 => break,
                            c => w.push(c),
                        }
                    }
                    Ok(String::from_utf16_lossy(&w))
                };
                return Err(Error::Assertion {
                    message: text(a[0])?,
                    file: text(a[1])?,
                    line: a[2],
                });
            }
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn digit_rounding() {
            let mut d = b"1#INF".to_vec();
            assert!(!round_digits(&mut d, 3));
            assert_eq!(d, b"1#J");
            let mut d = b"999".to_vec();
            assert!(round_digits(&mut d, 2));
            assert_eq!(d, b"10");
        }

        #[test]
        fn floats_as_msvc() {
            assert_eq!(fmt_f(1.5, 2, false), b"1.50");
            assert_eq!(fmt_f(0.125, 2, false), b"0.13");
            assert_eq!(fmt_f(f64::INFINITY, 6, false), b"1.#INF00");
            assert_eq!(fmt_f(f64::INFINITY, 2, false), b"1.#J");
            assert_eq!(fmt_f(0.1, 20, false), b"0.10000000000000001000");
            assert_eq!(fmt_f(0.5, 0, false), b"1");
            assert_eq!(fmt_f(9.5, 0, false), b"10");
            assert_eq!(fmt_g(f64::INFINITY, 6, false, false), b"1.#INF");
            assert_eq!(fmt_e(12345.678, 3, false, false), b"1.235e+004");
            assert_eq!(fmt_g(100_000.0, 6, false, false), b"100000");
            assert_eq!(fmt_g(1_000_000.0, 6, false, false), b"1e+006");
            assert_eq!(fmt_g(0.0001, 6, false, false), b"0.0001");
            assert_eq!(fmt_g(0.00001, 6, false, false), b"1e-005");
            assert_eq!(fmt_f(1e20, 0, false), b"100000000000000000000");
            assert_eq!(
                fmt_f(123_456_789_012_345_678_901.0, 1, false),
                b"123456789012345680000.0"
            );
        }

        #[test]
        fn numbers() {
            assert_eq!(strtol(b"  -0x1f", 0, true), (0xffff_ffe1, 7));
            assert_eq!(strtol(b"077", 0, true), (63, 3));
            assert_eq!(strtol(b"99999999999", 10, true), (0x7fff_ffff, 11));
            assert_eq!(strtol(b"x", 10, true), (0, 0));
            assert_eq!(strtol(b"0xg", 0, true), (0, 0));
            assert_eq!(strtol(b"-99999999999", 10, true), (0x8000_0000, 12));
            assert_eq!(strtod(b" 1.5e3x"), (1500.0, 6));
            assert_eq!(strtod(b"2."), (2.0, 2));
        }

        #[test]
        fn classes() {
            assert_eq!(ctype(b'A') & (ALPHA | UPPER | LOWER), 1);
            assert_eq!(ctype(b'7') & DIGIT, DIGIT);
            assert_eq!(icmp(b"ABC", b"abd"), -1);
        }
    }
}
