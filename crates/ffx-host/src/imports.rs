//! `FFX.exe`'s imports (docs/host.md, "Imports").
//!
//! Every IAT slot points at its own `int3` stub ([`crate::exec`]); the stub
//! runs the import's Rust side from [`TABLE`], or stops the run with
//! [`crate::Error::UnstubbedImport`] (the report names the first one).
//! Data imports (`_acmdln`, `_fmode`, ...) point at cells in the arena
//! instead.
//!
//! This round's Rust sides are the platform-free ones: the C runtime the
//! start-up code needs (on the arena heap) and deterministic stand-ins for
//! the Win32 calls on the way to the first window, which the Linux check
//! uses. On Windows the stand-ins for windows, files, `D3D11`, `XInput`, …
//! become forwarders to the real APIs (and FMOD, Iggy and the movies to
//! `rust_audio`, `ffx-swf` and `ffx-movie`); see docs/host.md.

use std::collections::HashMap;

use ffx_bridge::Mem;

use crate::Result;
use crate::arena::{Arena, PAGE};
use crate::exec::{Exec, Stop};
use crate::files;
use crate::table::{Args, Conv, Ret};

/// An import's Rust side.
pub type ImportFn = fn(&mut Exec, &Args) -> Ret;

/// One bound IAT slot.
#[derive(Debug, Clone)]
pub struct Bound {
    /// `DLL!name`, the DLL as the image writes it.
    pub name: String,
    /// The convention.
    pub conv: Conv,
    /// Stack words.
    pub words: usize,
    /// The Rust side, `None` when unstubbed.
    pub run: Option<ImportFn>,
}

/// The imports as bound.
#[derive(Debug, Default, Clone)]
pub struct Imports {
    bound: HashMap<usize, Bound>,
}

impl Imports {
    /// An empty binding.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds import `i` (`dll!name`) to its Rust side, if [`TABLE`] has one.
    pub fn bind(&mut self, i: usize, dll: &str, name: &str) {
        let full = format!("{dll}!{name}");
        let found = TABLE
            .iter()
            .find(|(d, n, ..)| d.eq_ignore_ascii_case(dll) && *n == name);
        let b = match found {
            Some(&(_, _, conv, words, run)) => Bound {
                name: full,
                conv,
                words,
                run: Some(run),
            },
            None => Bound {
                name: full,
                conv: Conv::Stdcall,
                words: 0,
                run: None,
            },
        };
        self.bound.insert(i, b);
    }

    /// Import `i`'s binding.
    #[must_use]
    pub fn bound(&self, i: usize) -> Option<&Bound> {
        self.bound.get(&i)
    }

    /// How many imports have a Rust side, and how many there are.
    #[must_use]
    pub fn coverage(&self) -> (usize, usize) {
        let stubbed = self.bound.values().filter(|b| b.run.is_some()).count();
        (stubbed, self.bound.len())
    }
}

/// The data imports: their cells' initial contents (`DLL!name`, value).
pub const DATA: [(&str, u32); 4] = [
    ("MSVCR110.dll!_fmode", 0),
    ("MSVCR110.dll!_commode", 0),
    ("MSVCR110.dll!_acmdln", 0), // filled with the command line's address
    ("MSVCR110.dll!_HUGE", 0),   // + 4: 0x7ff00000 (infinity)
];

/// The C runtime's state: the arena heap, `atexit` lists, `rand`.
#[derive(Debug)]
pub struct Crt {
    heap: (u32, u32),
    next: u32,
    free: HashMap<u32, Vec<u32>>,
    /// `_onexit` / `atexit` functions, in registration order.
    pub at_exit: Vec<u32>,
    /// `rand`'s state (MSVC: per thread; one thread here).
    pub rand: u32,
    /// The command line (`_acmdln`), an arena address.
    pub cmdline: u32,
    /// Fake handles handed out.
    next_handle: u32,
    last_error: u32,
}

/// The arena heap's size (grown by more regions later; one is enough to
/// reach the window).
const HEAP_SIZE: u32 = 0x2000_0000;
/// Every block's header: its size.
const HEADER: u32 = 16;

impl Crt {
    /// The heap's region in `arena`.
    ///
    /// # Errors
    /// When the arena has no room.
    pub fn new(arena: &Arena) -> Result<Self> {
        let at = arena.map_anywhere(0x1000_0000, HEAP_SIZE, "heap")?;
        Ok(Self {
            heap: (at, at + HEAP_SIZE),
            next: at,
            free: HashMap::new(),
            at_exit: Vec::new(),
            rand: 1,
            cmdline: 0,
            next_handle: 0x100,
            last_error: 0,
        })
    }

    /// `malloc`: a 16-byte aligned block, or 0.
    pub fn alloc(&mut self, mem: &mut impl Mem, size: u32) -> u32 {
        let rounded = size.max(1).next_multiple_of(16);
        if let Some(p) = self.free.get_mut(&rounded).and_then(Vec::pop) {
            return p;
        }
        let Some(end) = self.next.checked_add(HEADER + rounded) else {
            return 0;
        };
        if end > self.heap.1 {
            return 0;
        }
        let p = self.next + HEADER;
        mem.write(p - 4, rounded);
        self.next = end;
        p
    }

    /// `free`.
    pub fn release(&mut self, mem: &impl Mem, p: u32) {
        if p == 0 || p < self.heap.0 || p >= self.heap.1 {
            return;
        }
        let size: u32 = mem.read(p - 4);
        self.free.entry(size).or_default().push(p);
    }

    /// A block's usable size.
    #[must_use]
    pub fn size_of(&self, mem: &impl Mem, p: u32) -> u32 {
        if p < self.heap.0 || p >= self.heap.1 {
            return 0;
        }
        mem.read(p - 4)
    }

    /// A fresh fake handle.
    pub fn handle(&mut self) -> u32 {
        self.next_handle += 4;
        self.next_handle
    }
}

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn zero(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

fn one(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(1)
}

fn void(_: &mut Exec, _: &Args) -> Ret {
    Ret::VOID
}

fn first_arg(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(a(args, 0))
}

fn new_handle(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(e.crt.handle())
}

// ---- the C runtime -------------------------------------------------------

/// `_initterm(begin, end)`: calls each non-null function pointer.
fn initterm(e: &mut Exec, args: &Args) -> Ret {
    let (mut p, end) = (a(args, 0), a(args, 1));
    while p < end {
        let f: u32 = e.mem().read(p);
        if f != 0 && e.call_cdecl(f, &[]).is_err() {
            return Ret::VOID;
        }
        p += 4;
    }
    Ret::VOID
}

/// `_initterm_e(begin, end)`: as `_initterm`, stopping at the first
/// non-zero result, which it returns.
fn initterm_e(e: &mut Exec, args: &Args) -> Ret {
    let (mut p, end) = (a(args, 0), a(args, 1));
    while p < end {
        let f: u32 = e.mem().read(p);
        if f != 0 {
            match e.call_cdecl(f, &[]) {
                Ok(0) => {}
                Ok(r) => return Ret::eax(r),
                Err(_) => return Ret::eax(0),
            }
        }
        p += 4;
    }
    Ret::eax(0)
}

/// `__getmainargs(&argc, &argv, &envp, wildcard, &startinfo)`.
fn getmainargs(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let name = e.crt.alloc(&mut mem, 16);
    mem.write_bytes(name, b"FFX.exe\0");
    let vector = e.crt.alloc(&mut mem, 8);
    mem.write(vector, [name, 0]);
    let envp = e.crt.alloc(&mut mem, 4);
    mem.write(envp, 0u32);
    mem.write(a(args, 0), 1u32);
    mem.write(a(args, 1), vector);
    mem.write(a(args, 2), envp);
    Ret::eax(0)
}

/// `_onexit(f)` / `__dllonexit(f, ..)`: registers `f`, returns it.
fn onexit(e: &mut Exec, args: &Args) -> Ret {
    e.crt.at_exit.push(a(args, 0));
    Ret::eax(a(args, 0))
}

fn malloc(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    Ret::eax(e.crt.alloc(&mut mem, a(args, 0)))
}

fn calloc(e: &mut Exec, args: &Args) -> Ret {
    let Some(n) = a(args, 0).checked_mul(a(args, 1)) else {
        return Ret::eax(0);
    };
    let mut mem = e.mem();
    let p = e.crt.alloc(&mut mem, n);
    if p != 0 {
        mem.write_bytes(p, &vec![0; n as usize]);
    }
    Ret::eax(p)
}

fn free(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    e.crt.release(&mem, a(args, 0));
    Ret::VOID
}

/// `_recalloc(p, n, size)` / `realloc(p, size)` share this.
fn resize(e: &mut Exec, p: u32, size: u32, zero: bool) -> u32 {
    let mut mem = e.mem();
    let old = e.crt.size_of(&mem, p);
    let q = e.crt.alloc(&mut mem, size);
    if q == 0 {
        return 0;
    }
    let keep = old.min(size);
    let mut buf = vec![0; keep as usize];
    mem.read_bytes(p, &mut buf);
    mem.write_bytes(q, &buf);
    if zero && size > keep {
        mem.write_bytes(q + keep, &vec![0; (size - keep) as usize]);
    }
    e.crt.release(&mem, p);
    q
}

fn recalloc(e: &mut Exec, args: &Args) -> Ret {
    let size = a(args, 1).saturating_mul(a(args, 2));
    Ret::eax(resize(e, a(args, 0), size, true))
}

fn memset(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    #[allow(clippy::cast_possible_truncation)] // the byte, as memset takes it
    mem.write_bytes(a(args, 0), &vec![a(args, 1) as u8; a(args, 2) as usize]);
    Ret::eax(a(args, 0))
}

fn memcpy(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let mut buf = vec![0; a(args, 2) as usize];
    mem.read_bytes(a(args, 1), &mut buf);
    mem.write_bytes(a(args, 0), &buf);
    Ret::eax(a(args, 0))
}

fn strlen_at(mem: &impl Mem, mut p: u32) -> u32 {
    let start = p;
    while mem.read::<u8>(p) != 0 {
        p += 1;
    }
    p - start
}

fn strdup(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let n = strlen_at(&mem, a(args, 0)) + 1;
    let mut buf = vec![0; n as usize];
    mem.read_bytes(a(args, 0), &mut buf);
    let q = e.crt.alloc(&mut mem, n);
    mem.write_bytes(q, &buf);
    Ret::eax(q)
}

/// `rand()`: MSVC's generator (state × 214013 + 2531011, bits 16..30).
fn rand(e: &mut Exec, _: &Args) -> Ret {
    e.crt.rand = e.crt.rand.wrapping_mul(214_013).wrapping_add(2_531_011);
    Ret::eax((e.crt.rand >> 16) & 0x7fff)
}

fn srand(e: &mut Exec, args: &Args) -> Ret {
    e.crt.rand = a(args, 0);
    Ret::VOID
}

/// `exit`, `_exit`, `_amsg_exit`, `abort`, `ExitProcess`: the run ends.
fn exit(e: &mut Exec, args: &Args) -> Ret {
    e.end(Stop::Milestone(format!("exit({:#x})", a(args, 0))));
    Ret::VOID
}

// ---- KERNEL32 stand-ins (deterministic) ----------------------------------

fn get_last_error(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(e.crt.last_error)
}

fn set_last_error(e: &mut Exec, args: &Args) -> Ret {
    e.crt.last_error = a(args, 0);
    Ret::VOID
}

/// `GetSystemTimeAsFileTime(&ft)`: a fixed time.
fn system_time(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 0), 0x01d0_0000_0000_0000u64);
    Ret::VOID
}

/// `QueryPerformanceCounter(&n)` / `QueryPerformanceFrequency(&n)`.
fn perf_counter(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 0), 10_000_000u64);
    Ret::eax(1)
}

/// `GetModuleHandleW/A(name)`: the image for NULL, else 0.
fn module_handle(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(if a(args, 0) == 0 { e.image().base } else { 0 })
}

/// `GetStartupInfoW(&si)`: zeroed, `cb` set.
fn startup_info(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write_bytes(a(args, 0), &[0; 0x44]);
    mem.write(a(args, 0), 0x44u32);
    Ret::VOID
}

/// `HeapAlloc(heap, flags, size)`.
fn heap_alloc(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = e.crt.alloc(&mut mem, a(args, 2));
    if p != 0 && a(args, 1) & 8 != 0 {
        mem.write_bytes(p, &vec![0; a(args, 2) as usize]);
    }
    Ret::eax(p)
}

/// `HeapFree(heap, flags, p)`.
fn heap_free(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    e.crt.release(&mem, a(args, 2));
    Ret::eax(1)
}

/// `HeapSize(heap, flags, p)`.
fn heap_size(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    Ret::eax(e.crt.size_of(&mem, a(args, 2)))
}

/// `HeapReAlloc(heap, flags, p, size)`.
fn heap_realloc(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(resize(e, a(args, 2), a(args, 3), a(args, 1) & 8 != 0))
}

/// `VirtualAlloc(addr, size, type, protect)`: a fresh arena region.
fn virtual_alloc(e: &mut Exec, args: &Args) -> Ret {
    let size = a(args, 1).next_multiple_of(PAGE);
    Ret::eax(e.map_region(size, "VirtualAlloc").unwrap_or(0))
}

/// `CreateThread(attr, stack, entry, param, flags, &id)`: recorded, not
/// run yet (docs/host.md, "Threads").
fn create_thread(e: &mut Exec, args: &Args) -> Ret {
    e.stats.threads.push((a(args, 2), a(args, 3)));
    let id = e.crt.handle();
    if a(args, 5) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 5), id);
    }
    Ret::eax(id)
}

/// `GetSystemInfo(&info)`: one x86 processor, 4 KiB pages.
fn system_info(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = a(args, 0);
    mem.write_bytes(p, &[0; 36]);
    mem.write(p + 4, PAGE); // dwPageSize
    mem.write(p + 8, 0x0001_0000u32); // lpMinimumApplicationAddress
    mem.write(p + 12, 0x7ffe_ffffu32);
    mem.write(p + 16, 1u32); // dwActiveProcessorMask
    mem.write(p + 20, 4u32); // dwNumberOfProcessors
    mem.write(p + 24, 586u32); // dwProcessorType
    mem.write(p + 28, 0x0001_0000u32); // dwAllocationGranularity
    Ret::VOID
}

/// `_controlfp_s(&current, new, mask)`: the x87 control word as the game
/// thread has it (`0x027f`, in `_controlfp`'s bit layout: `0x0009001f`);
/// changes are not applied yet (the emulated x87 keeps its own).
fn controlfp_s(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 0) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 0), 0x0009_001fu32);
    }
    Ret::eax(0)
}

/// `GetSystemMetrics(index)`: a 1920 x 1080 primary screen.
fn system_metrics(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(match a(args, 0) {
        0 | 16 | 78 => 1920, // SM_CXSCREEN, SM_CXFULLSCREEN, SM_CXVIRTUALSCREEN
        1 | 17 | 79 => 1080,
        80 => 1, // SM_CMONITORS
        _ => 0,
    })
}

/// `GetModuleFileNameA/W(module, buf, size)`: `<game>\FFX.exe`.
fn module_file_name(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let name = format!("{}\\FFX.exe", files::GAME_DIR_WIN);
    let mut mem = e.mem();
    let n = write_str(&mut mem, a(args, 1), a(args, 2), &name, wide);
    Ret::eax(n)
}

fn module_file_name_a(e: &mut Exec, args: &Args) -> Ret {
    module_file_name(e, args, false)
}

fn module_file_name_w(e: &mut Exec, args: &Args) -> Ret {
    module_file_name(e, args, true)
}

/// Writes `s` (with its NUL, cut to `cap` characters) as bytes or UTF-16;
/// returns the characters written without the NUL.
fn write_str(mem: &mut impl Mem, at: u32, cap: u32, s: &str, wide: bool) -> u32 {
    if cap == 0 {
        return 0;
    }
    let units: Vec<u16> = if wide {
        s.encode_utf16().collect()
    } else {
        s.bytes().map(u16::from).collect()
    };
    let n = u32::try_from(units.len()).unwrap_or(u32::MAX).min(cap - 1);
    for (i, &u) in (0..n).zip(&units) {
        if wide {
            mem.write(at + 2 * i, u);
        } else {
            #[allow(clippy::cast_possible_truncation)] // built from bytes
            mem.write(at + i, u as u8);
        }
    }
    if wide {
        mem.write(at + 2 * n, 0u16);
    } else {
        mem.write(at + n, 0u8);
    }
    n
}

/// Reads a NUL-terminated string (bytes or UTF-16) at `at`.
fn read_str(mem: &impl Mem, at: u32, wide: bool) -> String {
    let mut out = Vec::new();
    let mut p = at;
    loop {
        let u = if wide {
            mem.read::<u16>(p)
        } else {
            u16::from(mem.read::<u8>(p))
        };
        if u == 0 || out.len() > 0x8000 {
            break;
        }
        out.push(u);
        p += if wide { 2 } else { 1 };
    }
    if wide {
        String::from_utf16_lossy(&out)
    } else {
        #[allow(clippy::cast_possible_truncation)] // bytes
        out.iter().map(|&u| char::from(u as u8)).collect()
    }
}

/// `GetEnvironmentVariableA/W(name, buf, size)`: none set (0,
/// `ERROR_ENVVAR_NOT_FOUND`).
fn env_var(e: &mut Exec, _: &Args) -> Ret {
    e.crt.last_error = 203;
    Ret::eax(0)
}

/// `GetCurrentDirectoryA(size, buf)`: the game folder.
fn current_dir(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    Ret::eax(write_str(
        &mut mem,
        a(args, 1),
        a(args, 0),
        files::GAME_DIR_WIN,
        false,
    ))
}

/// `GlobalMemoryStatusEx(&status)`: 16 GB, 8 free.
fn memory_status(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = a(args, 0);
    mem.write(p + 4, 50u32);
    mem.write(p + 8, 16u64 << 30);
    mem.write(p + 16, 8u64 << 30);
    mem.write(p + 24, 32u64 << 30);
    mem.write(p + 32, 24u64 << 30);
    mem.write(p + 40, 4u64 << 30);
    mem.write(p + 48, 2u64 << 30);
    mem.write(p + 56, 0u64);
    Ret::eax(1)
}

/// `GetVersionExA(&info)`: Windows 10 (6.2 as manifest-less programs see
/// it).
fn version_ex(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = a(args, 0);
    mem.write(p + 4, 6u32);
    mem.write(p + 8, 2u32);
    mem.write(p + 12, 9200u32);
    mem.write(p + 16, 2u32);
    Ret::eax(1)
}

/// `GetThreadLocale()`: en-US.
fn thread_locale(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0x409)
}

/// `CreateFileA/W(name, access, share, sa, disposition, flags, template)`.
fn create_file(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), wide);
    let write = a(args, 1) & 0x4000_0000 != 0;
    match e.files.open(&name, write, a(args, 4)) {
        Some(h) => {
            e.crt.last_error = 0;
            Ret::eax(h)
        }
        None => {
            e.crt.last_error = 2;
            Ret::eax(u32::MAX)
        }
    }
}

fn create_file_a(e: &mut Exec, args: &Args) -> Ret {
    create_file(e, args, false)
}

fn create_file_w(e: &mut Exec, args: &Args) -> Ret {
    create_file(e, args, true)
}

/// `ReadFile(h, buf, n, &read, overlapped)`.
fn read_file(e: &mut Exec, args: &Args) -> Ret {
    let Some(data) = e.files.read(a(args, 0), a(args, 2) as usize) else {
        return Ret::eax(0);
    };
    let mut mem = e.mem();
    mem.write_bytes(a(args, 1), &data);
    if a(args, 3) != 0 {
        mem.write(a(args, 3), u32::try_from(data.len()).unwrap_or(0));
    }
    Ret::eax(1)
}

/// `WriteFile(h, buf, n, &written, overlapped)`: discarded (no game file
/// is written by this build), reported as written.
fn write_file(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 3) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 3), a(args, 2));
    }
    Ret::eax(1)
}

/// `GetFileSizeEx(h, &size)`.
fn file_size_ex(e: &mut Exec, args: &Args) -> Ret {
    match e.files.size(a(args, 0)) {
        Some(n) => {
            let mut mem = e.mem();
            mem.write(a(args, 1), n);
            Ret::eax(1)
        }
        None => Ret::eax(0),
    }
}

/// `SetFilePointer(h, low, &high, method)`.
fn set_file_pointer(e: &mut Exec, args: &Args) -> Ret {
    let high = if a(args, 2) == 0 {
        i64::from(a(args, 1).cast_signed())
    } else {
        let hi: u32 = e.mem().read(a(args, 2));
        i64::from(hi) << 32 | i64::from(a(args, 1))
    };
    match e.files.seek(a(args, 0), high, a(args, 3)) {
        Some(pos) => {
            if a(args, 2) != 0 {
                let mut mem = e.mem();
                #[allow(clippy::cast_possible_truncation)] // the high half
                mem.write(a(args, 2), (pos >> 32) as u32);
            }
            #[allow(clippy::cast_possible_truncation)] // the low half
            Ret::eax(pos as u32)
        }
        None => Ret::eax(u32::MAX),
    }
}

/// `CloseHandle(h)`.
fn close_handle(e: &mut Exec, args: &Args) -> Ret {
    e.files.close(a(args, 0));
    Ret::eax(1)
}

/// `GetFileAttributesW(name)`.
fn file_attributes(e: &mut Exec, args: &Args) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), true);
    Ret::eax(e.files.attributes(&name))
}

/// `SHGetFolderPathW(hwnd, csidl, token, flags, path)`: a documents folder
/// outside the game folder (so nothing there opens yet).
fn folder_path(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    write_str(&mut mem, a(args, 4), 260, "C:\\FFXDocuments", true);
    Ret::eax(0)
}

/// `PathCombineA/W(out, dir, file)`.
fn path_combine(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let mut mem = e.mem();
    let dir = read_str(&mem, a(args, 1), wide);
    let file = read_str(&mem, a(args, 2), wide);
    let joined = if dir.is_empty() {
        file
    } else if file.is_empty() {
        dir
    } else {
        format!("{}\\{}", dir.trim_end_matches('\\'), file)
    };
    write_str(&mut mem, a(args, 0), 260, &joined, wide);
    Ret::eax(a(args, 0))
}

fn path_combine_a(e: &mut Exec, args: &Args) -> Ret {
    path_combine(e, args, false)
}

fn path_combine_w(e: &mut Exec, args: &Args) -> Ret {
    path_combine(e, args, true)
}

/// `PathRemoveFileSpecA/W(path)`: cuts at the last backslash.
fn path_remove_file_spec(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let mut mem = e.mem();
    let s = read_str(&mem, a(args, 0), wide);
    match s.rfind('\\') {
        Some(i) => {
            write_str(&mut mem, a(args, 0), 260, &s[..i], wide);
            Ret::eax(1)
        }
        None => Ret::eax(0),
    }
}

fn path_remove_file_spec_a(e: &mut Exec, args: &Args) -> Ret {
    path_remove_file_spec(e, args, false)
}

fn path_remove_file_spec_w(e: &mut Exec, args: &Args) -> Ret {
    path_remove_file_spec(e, args, true)
}

/// `GetCommandLineW()`: `FFX.exe`, in the arena.
fn command_line_w(e: &mut Exec, _: &Args) -> Ret {
    let mut mem = e.mem();
    let p = e.crt.alloc(&mut mem, 32);
    write_str(&mut mem, p, 16, "FFX.exe", true);
    Ret::eax(p)
}

/// `CommandLineToArgvW(line, &argc)`: one argument, the program.
fn command_line_to_argv(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = e.crt.alloc(&mut mem, 48);
    write_str(&mut mem, p + 8, 16, "FFX.exe", true);
    mem.write(p, [p + 8, 0]);
    mem.write(a(args, 1), 1u32);
    Ret::eax(p)
}

/// `MessageBoxW/A(hwnd, text, caption, type)`: logged, `IDOK`.
fn message_box(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let mem = e.mem();
    let text = read_str(&mem, a(args, 1), wide);
    let caption = read_str(&mem, a(args, 2), wide);
    e.stats.messages.push(format!("{caption}: {text}"));
    Ret::eax(1)
}

fn message_box_w(e: &mut Exec, args: &Args) -> Ret {
    message_box(e, args, true)
}

/// `_time64(&t)`: a fixed time.
fn time64(e: &mut Exec, args: &Args) -> Ret {
    let t = 1_700_000_000u64;
    if a(args, 0) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 0), t);
    }
    Ret::u64(t)
}

/// `InterlockedCompareExchange(&v, new, cmp)`.
fn interlocked_cmpxchg(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let old: u32 = mem.read(a(args, 0));
    if old == a(args, 2) {
        mem.write(a(args, 0), a(args, 1));
    }
    Ret::eax(old)
}

fn interlocked_add(e: &mut Exec, args: &Args, by: u32, ret_new: bool) -> Ret {
    let mut mem = e.mem();
    let old: u32 = mem.read(a(args, 0));
    let new = old.wrapping_add(by);
    mem.write(a(args, 0), new);
    Ret::eax(if ret_new { new } else { old })
}

fn interlocked_inc(e: &mut Exec, args: &Args) -> Ret {
    interlocked_add(e, args, 1, true)
}

fn interlocked_dec(e: &mut Exec, args: &Args) -> Ret {
    interlocked_add(e, args, u32::MAX, true)
}

fn interlocked_xadd(e: &mut Exec, args: &Args) -> Ret {
    let by = a(args, 1);
    interlocked_add(e, args, by, false)
}

/// `GetTickCount64()`.
fn tick_count64(_: &mut Exec, _: &Args) -> Ret {
    Ret::u64(1_000_000)
}

/// A milestone stand-in: the run stops when one of these is reached
/// unless the caller asked for more ([`Exec::stop_at`] covers the rest).
fn window_milestone(e: &mut Exec, _: &Args) -> Ret {
    e.end(Stop::Milestone("first window (CreateWindowExA)".into()));
    Ret::eax(0)
}

fn device_milestone(e: &mut Exec, _: &Args) -> Ret {
    e.end(Stop::Milestone("D3D11 device (D3D11CreateDevice*)".into()));
    Ret::eax(0x8000_4005)
}

const K32: &str = "KERNEL32.dll";
const CRT: &str = "MSVCR110.dll";

/// Every import with a Rust side: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    // C runtime start-up
    (CRT, "__set_app_type", Conv::Cdecl, 1, void),
    (CRT, "__setusermatherr", Conv::Cdecl, 1, void),
    (CRT, "_configthreadlocale", Conv::Cdecl, 1, zero),
    (CRT, "__getmainargs", Conv::Cdecl, 5, getmainargs),
    (CRT, "_initterm", Conv::Cdecl, 2, initterm),
    (CRT, "_initterm_e", Conv::Cdecl, 2, initterm_e),
    (CRT, "_onexit", Conv::Cdecl, 1, onexit),
    (CRT, "__dllonexit", Conv::Cdecl, 3, onexit),
    (CRT, "_lock", Conv::Cdecl, 1, void),
    (CRT, "_unlock", Conv::Cdecl, 1, void),
    (CRT, "_calloc_crt", Conv::Cdecl, 2, calloc),
    (CRT, "_ismbblead", Conv::Cdecl, 1, zero),
    (CRT, "__crtGetShowWindowMode", Conv::Cdecl, 0, |_, _| {
        Ret::eax(10)
    }),
    (CRT, "_crt_debugger_hook", Conv::Cdecl, 1, void),
    (CRT, "_fpreset", Conv::Cdecl, 0, void),
    (CRT, "_amsg_exit", Conv::Cdecl, 1, exit),
    (CRT, "exit", Conv::Cdecl, 1, exit),
    (CRT, "_exit", Conv::Cdecl, 1, exit),
    (CRT, "_cexit", Conv::Cdecl, 0, void),
    (CRT, "abort", Conv::Cdecl, 0, exit),
    // memory and strings
    (CRT, "malloc", Conv::Cdecl, 1, malloc),
    (CRT, "calloc", Conv::Cdecl, 2, calloc),
    (CRT, "free", Conv::Cdecl, 1, free),
    (CRT, "_recalloc", Conv::Cdecl, 3, recalloc),
    (CRT, "memset", Conv::Cdecl, 3, memset),
    (CRT, "memcpy", Conv::Cdecl, 3, memcpy),
    (CRT, "memmove", Conv::Cdecl, 3, memcpy),
    (CRT, "_strdup", Conv::Cdecl, 1, strdup),
    (CRT, "rand", Conv::Cdecl, 0, rand),
    (CRT, "srand", Conv::Cdecl, 1, srand),
    // KERNEL32
    (K32, "GetLastError", Conv::Stdcall, 0, get_last_error),
    (K32, "SetLastError", Conv::Stdcall, 1, set_last_error),
    (K32, "EncodePointer", Conv::Stdcall, 1, first_arg),
    (K32, "DecodePointer", Conv::Stdcall, 1, first_arg),
    (K32, "IsProcessorFeaturePresent", Conv::Stdcall, 1, one),
    (K32, "IsDebuggerPresent", Conv::Stdcall, 0, zero),
    (
        K32,
        "GetSystemTimeAsFileTime",
        Conv::Stdcall,
        1,
        system_time,
    ),
    (K32, "GetCurrentProcessId", Conv::Stdcall, 0, |_, _| {
        Ret::eax(0x1000)
    }),
    (K32, "GetCurrentThreadId", Conv::Stdcall, 0, |_, _| {
        Ret::eax(0x1004)
    }),
    (K32, "GetCurrentProcess", Conv::Stdcall, 0, |_, _| {
        Ret::eax(u32::MAX)
    }),
    (K32, "GetCurrentThread", Conv::Stdcall, 0, |_, _| {
        Ret::eax(u32::MAX - 1)
    }),
    (K32, "GetTickCount", Conv::Stdcall, 0, |_, _| {
        Ret::eax(1_000_000)
    }),
    (K32, "GetTickCount64", Conv::Stdcall, 0, tick_count64),
    (
        K32,
        "QueryPerformanceCounter",
        Conv::Stdcall,
        1,
        perf_counter,
    ),
    (
        K32,
        "QueryPerformanceFrequency",
        Conv::Stdcall,
        1,
        perf_counter,
    ),
    (K32, "SetUnhandledExceptionFilter", Conv::Stdcall, 1, zero),
    (K32, "GetModuleHandleW", Conv::Stdcall, 1, module_handle),
    (K32, "GetModuleHandleA", Conv::Stdcall, 1, module_handle),
    (K32, "GetProcessHeap", Conv::Stdcall, 0, |_, _| {
        Ret::eax(0x0bad_0001)
    }),
    (K32, "HeapAlloc", Conv::Stdcall, 3, heap_alloc),
    (K32, "HeapFree", Conv::Stdcall, 3, heap_free),
    (K32, "HeapSize", Conv::Stdcall, 3, heap_size),
    (K32, "HeapReAlloc", Conv::Stdcall, 4, heap_realloc),
    (K32, "VirtualAlloc", Conv::Stdcall, 4, virtual_alloc),
    (K32, "VirtualFree", Conv::Stdcall, 3, one),
    (K32, "InitializeCriticalSection", Conv::Stdcall, 1, void),
    (K32, "InitializeCriticalSectionEx", Conv::Stdcall, 3, one),
    (K32, "EnterCriticalSection", Conv::Stdcall, 1, void),
    (K32, "LeaveCriticalSection", Conv::Stdcall, 1, void),
    (K32, "DeleteCriticalSection", Conv::Stdcall, 1, void),
    (
        K32,
        "InterlockedIncrement",
        Conv::Stdcall,
        1,
        interlocked_inc,
    ),
    (
        K32,
        "InterlockedDecrement",
        Conv::Stdcall,
        1,
        interlocked_dec,
    ),
    (
        K32,
        "InterlockedExchangeAdd",
        Conv::Stdcall,
        2,
        interlocked_xadd,
    ),
    (
        K32,
        "InterlockedCompareExchange",
        Conv::Stdcall,
        3,
        interlocked_cmpxchg,
    ),
    (K32, "CreateEventA", Conv::Stdcall, 4, new_handle),
    (K32, "CreateEventW", Conv::Stdcall, 4, new_handle),
    (K32, "CreateMutexW", Conv::Stdcall, 3, new_handle),
    (K32, "CreateSemaphoreA", Conv::Stdcall, 4, new_handle),
    (K32, "CreateSemaphoreW", Conv::Stdcall, 4, new_handle),
    (K32, "SetEvent", Conv::Stdcall, 1, one),
    (K32, "ResetEvent", Conv::Stdcall, 1, one),
    (K32, "ReleaseSemaphore", Conv::Stdcall, 3, one),
    (K32, "ReleaseMutex", Conv::Stdcall, 1, one),
    (K32, "CloseHandle", Conv::Stdcall, 1, close_handle),
    (K32, "WaitForSingleObject", Conv::Stdcall, 2, zero),
    (K32, "WaitForSingleObjectEx", Conv::Stdcall, 3, zero),
    (K32, "CreateThread", Conv::Stdcall, 6, create_thread),
    (K32, "SetThreadPriority", Conv::Stdcall, 2, one),
    (K32, "ResumeThread", Conv::Stdcall, 1, zero),
    (K32, "Sleep", Conv::Stdcall, 1, void),
    (K32, "SleepEx", Conv::Stdcall, 2, zero),
    (K32, "GetSystemInfo", Conv::Stdcall, 1, system_info),
    (K32, "SetThreadExecutionState", Conv::Stdcall, 1, one),
    (K32, "ExitProcess", Conv::Stdcall, 1, exit),
    (K32, "GetStartupInfoW", Conv::Stdcall, 1, startup_info),
    (K32, "GetThreadLocale", Conv::Stdcall, 0, thread_locale),
    (K32, "SetThreadUILanguage", Conv::Stdcall, 1, first_arg),
    (
        K32,
        "GetModuleFileNameA",
        Conv::Stdcall,
        3,
        module_file_name_a,
    ),
    (
        K32,
        "GetModuleFileNameW",
        Conv::Stdcall,
        3,
        module_file_name_w,
    ),
    (K32, "GetEnvironmentVariableA", Conv::Stdcall, 3, env_var),
    (K32, "GetEnvironmentVariableW", Conv::Stdcall, 3, env_var),
    (K32, "GetCurrentDirectoryA", Conv::Stdcall, 2, current_dir),
    (K32, "SetCurrentDirectoryA", Conv::Stdcall, 1, one),
    (K32, "GlobalMemoryStatusEx", Conv::Stdcall, 1, memory_status),
    (K32, "GetVersionExA", Conv::Stdcall, 1, version_ex),
    (K32, "CreateFileA", Conv::Stdcall, 7, create_file_a),
    (K32, "CreateFileW", Conv::Stdcall, 7, create_file_w),
    (K32, "ReadFile", Conv::Stdcall, 5, read_file),
    (K32, "WriteFile", Conv::Stdcall, 5, write_file),
    (K32, "GetFileSizeEx", Conv::Stdcall, 2, file_size_ex),
    (K32, "SetFilePointer", Conv::Stdcall, 4, set_file_pointer),
    (K32, "GetFileAttributesW", Conv::Stdcall, 1, file_attributes),
    (K32, "CreateDirectoryW", Conv::Stdcall, 2, one),
    (K32, "LocalFree", Conv::Stdcall, 1, zero),
    (K32, "OutputDebugStringA", Conv::Stdcall, 1, void),
    (CRT, "_controlfp_s", Conv::Cdecl, 3, controlfp_s),
    (
        CRT,
        "__crtSetUnhandledExceptionFilter",
        Conv::Cdecl,
        1,
        zero,
    ),
    (CRT, "_time64", Conv::Cdecl, 1, time64),
    (
        "SHELL32.dll",
        "SHGetFolderPathW",
        Conv::Stdcall,
        5,
        folder_path,
    ),
    (
        "SHELL32.dll",
        "CommandLineToArgvW",
        Conv::Stdcall,
        2,
        command_line_to_argv,
    ),
    (
        "SHLWAPI.dll",
        "PathCombineA",
        Conv::Stdcall,
        3,
        path_combine_a,
    ),
    (
        "SHLWAPI.dll",
        "PathCombineW",
        Conv::Stdcall,
        3,
        path_combine_w,
    ),
    (
        "SHLWAPI.dll",
        "PathRemoveFileSpecA",
        Conv::Stdcall,
        1,
        path_remove_file_spec_a,
    ),
    (
        "SHLWAPI.dll",
        "PathRemoveFileSpecW",
        Conv::Stdcall,
        1,
        path_remove_file_spec_w,
    ),
    (K32, "GetCommandLineW", Conv::Stdcall, 0, command_line_w),
    (K32, "LoadLibraryA", Conv::Stdcall, 1, zero),
    (K32, "LoadLibraryW", Conv::Stdcall, 1, zero),
    (K32, "LoadLibraryExA", Conv::Stdcall, 3, zero),
    (K32, "GetProcAddress", Conv::Stdcall, 2, zero),
    (K32, "FreeLibrary", Conv::Stdcall, 1, one),
    ("USER32.dll", "MessageBoxW", Conv::Stdcall, 4, message_box_w),
    ("USER32.dll", "LoadIconA", Conv::Stdcall, 2, new_handle),
    ("USER32.dll", "LoadCursorA", Conv::Stdcall, 2, new_handle),
    ("USER32.dll", "RegisterClassA", Conv::Stdcall, 1, |_, _| {
        Ret::eax(0xc001)
    }),
    ("USER32.dll", "AdjustWindowRect", Conv::Stdcall, 3, one),
    ("USER32.dll", "ShowCursor", Conv::Stdcall, 1, zero),
    ("ole32.dll", "CoInitialize", Conv::Stdcall, 1, zero),
    (
        "USER32.dll",
        "GetSystemMetrics",
        Conv::Stdcall,
        1,
        system_metrics,
    ),
    // milestones
    (
        "USER32.dll",
        "CreateWindowExA",
        Conv::Stdcall,
        12,
        window_milestone,
    ),
    (
        "d3d11.dll",
        "D3D11CreateDevice",
        Conv::Stdcall,
        10,
        device_milestone,
    ),
    (
        "d3d11.dll",
        "D3D11CreateDeviceAndSwapChain",
        Conv::Stdcall,
        12,
        device_milestone,
    ),
    // timers
    ("WINMM.dll", "timeBeginPeriod", Conv::Stdcall, 1, zero),
    ("WINMM.dll", "timeEndPeriod", Conv::Stdcall, 1, zero),
    ("WINMM.dll", "timeGetTime", Conv::Stdcall, 0, |_, _| {
        Ret::eax(1_000_000)
    }),
];
