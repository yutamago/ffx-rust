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
use crate::clock;
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
    /// The DLL that imports it (`None`: `FFX.exe`).
    pub from: Option<String>,
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

    /// Binds import `i` (`dll!name`, imported by `from` or by `FFX.exe`)
    /// to its Rust side, if [`TABLE`] has one.
    pub fn bind(&mut self, i: usize, dll: &str, name: &str, from: Option<&str>) {
        let from = from.map(str::to_string);
        let full = format!("{dll}!{name}");
        let found = platform_table()
            .iter()
            .chain(crate::win32::TABLE)
            .chain(TABLE)
            .chain(crate::crypt::TABLE)
            .chain(crate::stand_ins::TABLE)
            .chain(crate::iggy::TABLE)
            .chain(crate::audio::TABLE)
            .find(|(d, n, ..)| d.eq_ignore_ascii_case(dll) && *n == name);
        let b = match found {
            Some(&(_, _, conv, words, run)) => Bound {
                name: full,
                conv,
                words,
                run: Some(run),
                from,
            },
            None => match iggy_words(dll, name) {
                // Iggy (Flash UI): a null stand-in, until `ffx-swf` takes
                // its place; its exports carry their stack size (`_X@N`)
                Some(words) => Bound {
                    name: full,
                    conv: Conv::Stdcall,
                    words,
                    run: Some(iggy_null),
                    from,
                },
                None => Bound {
                    name: full,
                    conv: Conv::Stdcall,
                    words: 0,
                    run: None,
                    from,
                },
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

/// The imports that forward to the real system on this platform, bound
/// ahead of the stand-ins (on Windows `crate::win_forward`; none
/// elsewhere).
fn platform_table() -> &'static [(&'static str, &'static str, Conv, usize, ImportFn)] {
    #[cfg(windows)]
    {
        crate::win_forward::TABLE
    }
    #[cfg(not(windows))]
    {
        &[]
    }
}

/// The stack words of an `iggy_w32.dll` stdcall export (`_Name@N`).
fn iggy_words(dll: &str, name: &str) -> Option<usize> {
    if !dll.eq_ignore_ascii_case("iggy_w32.dll") {
        return None;
    }
    let (_, n) = name.strip_prefix('_')?.rsplit_once('@')?;
    n.parse::<usize>().ok().map(|n| n / 4)
}

/// Every Iggy call: nothing happens, 0 (no player, no value).
fn iggy_null(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

/// The data imports: their cells' initial contents (`DLL!name`, value).
pub const DATA: [(&str, u32); 4] = [
    ("MSVCR110.dll!_fmode", 0),
    ("MSVCR110.dll!_commode", 0),
    ("MSVCR110.dll!_acmdln", 0), // filled with the command line's address
    ("MSVCR110.dll!_HUGE", 0),   // + 4: 0x7ff00000 (infinity)
];

/// The heap behind `malloc` and `HeapAlloc`, shared by every thread
/// (behind [`crate::exec::Shared::heap`]'s lock).
#[derive(Debug)]
pub struct Heap {
    range: (u32, u32),
    next: u32,
    free: HashMap<u32, Vec<u32>>,
}

/// The C runtime's per-thread state.
#[derive(Debug, Clone, Copy)]
pub struct ThreadCrt {
    /// `rand`'s state (per thread, as MSVC keeps it; seed 1).
    pub rand: u32,
    /// `GetLastError`.
    pub last_error: u32,
}

impl Default for ThreadCrt {
    fn default() -> Self {
        Self {
            rand: 1,
            last_error: 0,
        }
    }
}

/// The arena heap's size (grown by more regions later; one is enough to
/// reach the window).
const HEAP_SIZE: u32 = 0x2000_0000;
/// Every block's header: its size.
const HEADER: u32 = 16;

impl Heap {
    /// The heap's region in `arena`.
    ///
    /// # Errors
    /// When the arena has no room.
    pub fn new(arena: &Arena) -> Result<Self> {
        let at = arena.map_anywhere(0x1000_0000, HEAP_SIZE, "heap")?;
        Ok(Self {
            range: (at, at + HEAP_SIZE),
            next: at,
            free: HashMap::new(),
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
        if end > self.range.1 {
            return 0;
        }
        let p = self.next + HEADER;
        mem.write(p - 4, rounded);
        self.next = end;
        p
    }

    /// `free`.
    pub fn release(&mut self, mem: &impl Mem, p: u32) {
        if p == 0 || p < self.range.0 || p >= self.range.1 {
            return;
        }
        let size: u32 = mem.read(p - 4);
        self.free.entry(size).or_default().push(p);
    }

    /// A block's usable size.
    #[must_use]
    pub fn size_of(&self, mem: &impl Mem, p: u32) -> u32 {
        if p < self.range.0 || p >= self.range.1 {
            return 0;
        }
        mem.read(p - 4)
    }
}

pub(crate) fn a(args: &Args, i: usize) -> u32 {
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
    Ret::eax(e.new_handle())
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
    let name = e.alloc(16);
    let mut mem = e.mem();
    mem.write_bytes(name, b"FFX.exe\0");
    let vector = e.alloc(8);
    mem.write(vector, [name, 0]);
    let envp = e.alloc(4);
    mem.write(envp, 0u32);
    mem.write(a(args, 0), 1u32);
    mem.write(a(args, 1), vector);
    mem.write(a(args, 2), envp);
    Ret::eax(0)
}

/// `_onexit(f)` / `__dllonexit(f, ..)`: registers `f`, returns it.
fn onexit(e: &mut Exec, args: &Args) -> Ret {
    if let Ok(mut l) = e.shared().at_exit.lock() {
        l.push(a(args, 0));
    }
    Ret::eax(a(args, 0))
}

fn malloc(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(e.alloc(a(args, 0)))
}

fn calloc(e: &mut Exec, args: &Args) -> Ret {
    let Some(n) = a(args, 0).checked_mul(a(args, 1)) else {
        return Ret::eax(0);
    };
    let p = e.alloc(n);
    let mut mem = e.mem();
    if p != 0 {
        mem.write_bytes(p, &vec![0; n as usize]);
    }
    Ret::eax(p)
}

fn free(e: &mut Exec, args: &Args) -> Ret {
    e.release(a(args, 0));
    Ret::VOID
}

/// `_recalloc(p, n, size)` / `realloc(p, size)` share this.
fn resize(e: &mut Exec, p: u32, size: u32, zero: bool) -> u32 {
    let mut mem = e.mem();
    let old = e.size_of(p);
    let q = e.alloc(size);
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
    e.release(p);
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
    let q = e.alloc(n);
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

/// `GetStartupInfoW(&si)`: zeroed, `cb` set.
fn startup_info(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write_bytes(a(args, 0), &[0; 0x44]);
    mem.write(a(args, 0), 0x44u32);
    Ret::VOID
}

/// `HeapAlloc(heap, flags, size)`.
fn heap_alloc(e: &mut Exec, args: &Args) -> Ret {
    let p = e.alloc(a(args, 2));
    let mut mem = e.mem();
    if p != 0 && a(args, 1) & 8 != 0 {
        mem.write_bytes(p, &vec![0; a(args, 2) as usize]);
    }
    Ret::eax(p)
}

/// `HeapFree(heap, flags, p)`.
fn heap_free(e: &mut Exec, args: &Args) -> Ret {
    e.release(a(args, 2));
    Ret::eax(1)
}

/// `HeapSize(heap, flags, p)`.
fn heap_size(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(e.size_of(a(args, 2)))
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

/// Writes `s` (with its NUL, cut to `cap` characters) as bytes or UTF-16;
/// returns the characters written without the NUL.
pub(crate) fn write_str(mem: &mut impl Mem, at: u32, cap: u32, s: &str, wide: bool) -> u32 {
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
pub(crate) fn read_str(mem: &impl Mem, at: u32, wide: bool) -> String {
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
    let opened = e.files().open(&name, write, a(args, 4));
    match opened {
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
    let data = e.files().read(a(args, 0), a(args, 2) as usize);
    let Some(data) = data else {
        return Ret::eax(0);
    };
    let mut mem = e.mem();
    mem.write_bytes(a(args, 1), &data);
    if a(args, 3) != 0 {
        mem.write(a(args, 3), u32::try_from(data.len()).unwrap_or(0));
    }
    Ret::eax(1)
}

/// `WriteFile(h, buf, n, &written, overlapped)`: documents files are
/// written; anything else (the console) is discarded, reported as
/// written.
fn write_file(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let mut data = vec![0; a(args, 2) as usize];
    mem.read_bytes(a(args, 1), &mut data);
    let n = e.files().write(a(args, 0), &data).unwrap_or(data.len());
    if a(args, 3) != 0 {
        mem.write(a(args, 3), u32::try_from(n).unwrap_or(0));
    }
    Ret::eax(1)
}

/// `CreateDirectoryW(name, sa)`: in the documents folder; elsewhere it
/// "exists".
fn create_directory(e: &mut Exec, args: &Args) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), true);
    e.files().create_dir(&name);
    Ret::eax(1)
}

/// `GetFileSizeEx(h, &size)`.
fn file_size_ex(e: &mut Exec, args: &Args) -> Ret {
    let size = e.files().size(a(args, 0));
    match size {
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
    let pos = e.files().seek(a(args, 0), high, a(args, 3));
    match pos {
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

/// `GetFileAttributesW(name)`.
fn file_attributes(e: &mut Exec, args: &Args) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), true);
    let attributes = e.files().attributes(&name);
    Ret::eax(attributes)
}

/// `SHGetFolderPathW(hwnd, csidl, token, flags, path)`: the documents
/// folder ([`files::DOCUMENTS_WIN`], a host folder of the caller's).
fn folder_path(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    write_str(&mut mem, a(args, 4), 260, files::DOCUMENTS_WIN, true);
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
    let p = e.alloc(32);
    let mut mem = e.mem();
    write_str(&mut mem, p, 16, "FFX.exe", true);
    Ret::eax(p)
}

/// `CommandLineToArgvW(line, &argc)`: one argument, the program.
fn command_line_to_argv(e: &mut Exec, args: &Args) -> Ret {
    let p = e.alloc(48);
    let mut mem = e.mem();
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

/// `wcstombs(dst, src, n)` / `mbstowcs(dst, src, n)` in the "C" locale
/// (code units below 0x100 map to themselves); returns the count without
/// the NUL, or `-1` for a unit the locale can't convert.
fn convert(e: &mut Exec, args: &Args, to_wide: bool) -> Ret {
    let mut mem = e.mem();
    let (dst, src, n) = (a(args, 0), a(args, 1), a(args, 2));
    let s = read_str(&mem, src, !to_wide);
    if !to_wide && s.chars().any(|c| u32::from(c) > 0xff) {
        return Ret::eax(u32::MAX);
    }
    let len = u32::try_from(s.chars().count()).unwrap_or(u32::MAX);
    if dst == 0 {
        return Ret::eax(len);
    }
    let k = len.min(n);
    for (i, c) in (0..k).zip(s.chars()) {
        if to_wide {
            #[allow(clippy::cast_possible_truncation)] // below 0x100
            mem.write(dst + 2 * i, u32::from(c) as u16);
        } else {
            #[allow(clippy::cast_possible_truncation)] // checked above
            mem.write(dst + i, u32::from(c) as u8);
        }
    }
    if k < n {
        if to_wide {
            mem.write(dst + 2 * k, 0u16);
        } else {
            mem.write(dst + k, 0u8);
        }
    }
    Ret::eax(k)
}

fn wcstombs(e: &mut Exec, args: &Args) -> Ret {
    convert(e, args, false)
}

fn mbstowcs(e: &mut Exec, args: &Args) -> Ret {
    convert(e, args, true)
}

/// MSVC's `printf` formatting of `fmt` with the variable arguments from
/// stack word `first` on (`%d %i %u %x %X %o %c %s %S %p %f %e %g %%`,
/// flags, width, precision, `h`/`l`/`ll`/`I64`).
#[allow(clippy::too_many_lines)] // one formatter, conversion by conversion
fn format(mem: &impl Mem, fmt: &str, args: &Args, first: u32) -> String {
    let mut out = String::new();
    let mut next = args.slots + 4 * first;
    let mut word = || {
        let w: u32 = mem.read(next);
        next += 4;
        w
    };
    let mut it = fmt.chars().peekable();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut flags = String::new();
        while let Some(&f) = it.peek().filter(|f| "-+ #0".contains(**f)) {
            flags.push(f);
            it.next();
        }
        let mut width = String::new();
        while let Some(&d) = it.peek().filter(|d| d.is_ascii_digit() || **d == '*') {
            if d == '*' {
                width = word().cast_signed().to_string();
            } else {
                width.push(d);
            }
            it.next();
        }
        let mut precision: Option<usize> = None;
        if it.peek() == Some(&'.') {
            it.next();
            let mut p = String::new();
            while let Some(&d) = it.peek().filter(|d| d.is_ascii_digit()) {
                p.push(d);
                it.next();
            }
            precision = Some(p.parse().unwrap_or(0));
        }
        let mut long = false;
        while let Some(&m) = it.peek().filter(|m| "hlI6432".contains(**m)) {
            if m == '4' || m == '6' {
                long = true;
            }
            if m == 'l' && it.clone().nth(1) == Some('l') {
                long = true;
            }
            it.next();
        }
        let Some(conv) = it.next() else { break };
        let body = match conv {
            'd' | 'i' => {
                if long {
                    let lo = word();
                    let hi = word();
                    ((u64::from(hi) << 32 | u64::from(lo)).cast_signed()).to_string()
                } else {
                    word().cast_signed().to_string()
                }
            }
            'u' | 'x' | 'X' | 'o' => {
                let v = if long {
                    let lo = word();
                    u64::from(word()) << 32 | u64::from(lo)
                } else {
                    u64::from(word())
                };
                match conv {
                    'u' => v.to_string(),
                    'x' => format!("{v:x}"),
                    'X' => format!("{v:X}"),
                    _ => format!("{v:o}"),
                }
            }
            'p' => format!("{:08X}", word()),
            'c' => char::from_u32(word() & 0xff)
                .map(String::from)
                .unwrap_or_default(),
            's' | 'S' => {
                let p = word();
                let s = if p == 0 {
                    "(null)".to_string()
                } else {
                    read_str(mem, p, conv == 'S')
                };
                match precision {
                    Some(n) => s.chars().take(n).collect(),
                    None => s,
                }
            }
            'f' | 'e' | 'g' | 'F' | 'E' | 'G' => {
                let lo = word();
                let v = f64::from_bits(u64::from(word()) << 32 | u64::from(lo));
                let p = precision.unwrap_or(6);
                match conv {
                    'e' | 'E' => format!("{v:.p$e}"),
                    'g' | 'G' => format!("{v}"),
                    _ => format!("{v:.p$}"),
                }
            }
            '%' => "%".to_string(),
            other => format!("%{other}"),
        };
        let w: usize = width.trim_start_matches('-').parse().unwrap_or(0);
        let pad = w.saturating_sub(body.chars().count());
        if flags.contains('-') || width.starts_with('-') {
            out.push_str(&body);
            out.extend(std::iter::repeat_n(' ', pad));
        } else if flags.contains('0') && conv != 's' {
            let (sign, digits) = body
                .strip_prefix('-')
                .map_or(("", body.as_str()), |d| ("-", d));
            out.push_str(sign);
            out.extend(std::iter::repeat_n('0', pad));
            out.push_str(digits);
        } else {
            out.extend(std::iter::repeat_n(' ', pad));
            out.push_str(&body);
        }
    }
    out
}

/// `sprintf(buf, fmt, ...)`.
fn sprintf(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let fmt = read_str(&mem, a(args, 1), false);
    let s = format(&mem, &fmt, args, 2);
    let n = write_str(&mut mem, a(args, 0), u32::MAX, &s, false);
    Ret::eax(n)
}

/// `sprintf_s(buf, size, fmt, ...)` / `_snprintf(buf, size, fmt, ...)`
/// (the cut differs: `_snprintf` doesn't terminate a full buffer, which
/// this doesn't model yet).
fn sprintf_s(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let fmt = read_str(&mem, a(args, 2), false);
    let s = format(&mem, &fmt, args, 3);
    let n = write_str(&mut mem, a(args, 0), a(args, 1), &s, false);
    Ret::eax(n)
}

/// `_wfopen` / `fopen`: no stdio files yet (NULL, as for a missing file).
fn fopen_missing(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), wide);
    e.files().log.push((format!("fopen {name}"), false));
    Ret::eax(0)
}

fn wfopen(e: &mut Exec, args: &Args) -> Ret {
    fopen_missing(e, args, true)
}

fn fopen(e: &mut Exec, args: &Args) -> Ret {
    fopen_missing(e, args, false)
}

/// The fake window handle (`CreateWindowExA`).
pub const HWND: u32 = 0x0001_0010;

/// `CreateWindowExA`: a milestone, recorded; the run goes on with a fake
/// window ([`HWND`]) that gets no messages (`--stop-at
/// USER32.dll!CreateWindowExA` stops here instead).
fn window_milestone(e: &mut Exec, _: &Args) -> Ret {
    e.stats
        .milestones
        .push("first window (CreateWindowExA)".into());
    Ret::eax(HWND)
}

/// `GetClientRect` / `GetWindowRect(hwnd, &rect)`: the window is the
/// 1920 x 1080 screen.
fn window_rect(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 1), [0u32, 0, 1920, 1080]);
    Ret::eax(1)
}

/// `PeekMessageA/W(&msg, hwnd, min, max, remove)`: no messages.
fn peek_message(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

/// `D3D11CreateDevice(adapter, driver, software, flags, levels, n, sdk,
/// &device, &level, &context)`. The renderer's start-up first asks only
/// for the feature level (no device, no context): `S_OK` and `11_0`
/// (`0xb000`, the highest of the default levels), recorded as a
/// milestone. Asked for a device, it is `crate::gfx`'s.
fn create_device(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 7) != 0 || a(args, 9) != 0 {
        return crate::gfx::create_device(e, args);
    }
    if a(args, 8) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 8), 0xb000u32);
    }
    e.stats
        .milestones
        .push("D3D11 feature level probe (11_0)".into());
    Ret::eax(0)
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
    (K32, "GetCurrentProcess", Conv::Stdcall, 0, |_, _| {
        Ret::eax(u32::MAX)
    }),
    (K32, "GetCurrentThread", Conv::Stdcall, 0, |_, _| {
        Ret::eax(u32::MAX - 1)
    }),
    // the virtual clock (crate::clock)
    (K32, "GetTickCount", Conv::Stdcall, 0, clock::tick_count),
    (K32, "GetTickCount64", Conv::Stdcall, 0, clock::tick_count64),
    (
        K32,
        "QueryPerformanceCounter",
        Conv::Stdcall,
        1,
        clock::perf_counter,
    ),
    (
        K32,
        "QueryPerformanceFrequency",
        Conv::Stdcall,
        1,
        clock::perf_frequency,
    ),
    (K32, "SetUnhandledExceptionFilter", Conv::Stdcall, 1, zero),
    (K32, "GetProcessHeap", Conv::Stdcall, 0, |_, _| {
        Ret::eax(0x0bad_0001)
    }),
    (K32, "HeapAlloc", Conv::Stdcall, 3, heap_alloc),
    (K32, "HeapFree", Conv::Stdcall, 3, heap_free),
    (K32, "HeapSize", Conv::Stdcall, 3, heap_size),
    (K32, "HeapReAlloc", Conv::Stdcall, 4, heap_realloc),
    (K32, "VirtualAlloc", Conv::Stdcall, 4, virtual_alloc),
    (K32, "VirtualFree", Conv::Stdcall, 3, one),
    (K32, "SetThreadPriority", Conv::Stdcall, 2, one),
    (K32, "GetSystemInfo", Conv::Stdcall, 1, system_info),
    (K32, "SetThreadExecutionState", Conv::Stdcall, 1, one),
    (K32, "ExitProcess", Conv::Stdcall, 1, exit),
    (K32, "GetStartupInfoW", Conv::Stdcall, 1, startup_info),
    (K32, "GetThreadLocale", Conv::Stdcall, 0, thread_locale),
    (K32, "SetThreadUILanguage", Conv::Stdcall, 1, first_arg),
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
    (K32, "CreateDirectoryW", Conv::Stdcall, 2, create_directory),
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
    (CRT, "wcstombs", Conv::Cdecl, 3, wcstombs),
    (CRT, "mbstowcs", Conv::Cdecl, 3, mbstowcs),
    (CRT, "sprintf", Conv::Cdecl, 2, sprintf),
    (CRT, "sprintf_s", Conv::Cdecl, 3, sprintf_s),
    (CRT, "_snprintf", Conv::Cdecl, 3, sprintf_s),
    (CRT, "_wfopen", Conv::Cdecl, 2, wfopen),
    (CRT, "fopen", Conv::Cdecl, 2, fopen),
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
    (
        K32,
        "LoadLibraryA",
        Conv::Stdcall,
        1,
        crate::library::load_library_a,
    ),
    (K32, "LoadLibraryW", Conv::Stdcall, 1, zero),
    (K32, "LoadLibraryExA", Conv::Stdcall, 3, zero),
    (
        K32,
        "GetProcAddress",
        Conv::Stdcall,
        2,
        crate::library::get_proc_address,
    ),
    (
        K32,
        "FreeLibrary",
        Conv::Stdcall,
        1,
        crate::library::free_library,
    ),
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
        create_device,
    ),
    (
        "d3d11.dll",
        "D3D11CreateDeviceAndSwapChain",
        Conv::Stdcall,
        12,
        crate::gfx::create_device_and_swap_chain,
    ),
    (
        "DINPUT8.dll",
        "DirectInput8Create",
        Conv::Stdcall,
        5,
        crate::gfx::direct_input8_create,
    ),
    // Steam, absent: `SteamAPI_Init` fails, so the game runs without
    // achievements and never asks for the interfaces (Steam_Init)
    (
        "steam_api.dll",
        "SteamAPI_RestartAppIfNecessary",
        Conv::Cdecl,
        1,
        zero,
    ),
    ("steam_api.dll", "SteamAPI_Init", Conv::Cdecl, 0, zero),
    (
        "steam_api.dll",
        "SteamAPI_RunCallbacks",
        Conv::Cdecl,
        0,
        void,
    ),
    ("steam_api.dll", "SteamAPI_Shutdown", Conv::Cdecl, 0, void),
    (
        "steam_api.dll",
        "SteamAPI_RegisterCallback",
        Conv::Cdecl,
        2,
        void,
    ),
    (
        "steam_api.dll",
        "SteamAPI_UnregisterCallback",
        Conv::Cdecl,
        1,
        void,
    ),
    // the window (a fake one, no messages)
    ("USER32.dll", "ShowWindow", Conv::Stdcall, 2, zero),
    ("USER32.dll", "SetWindowPos", Conv::Stdcall, 7, one),
    ("USER32.dll", "SetWindowTextA", Conv::Stdcall, 2, one),
    ("USER32.dll", "SetWindowLongA", Conv::Stdcall, 3, zero),
    ("USER32.dll", "GetWindowLongA", Conv::Stdcall, 2, zero),
    ("USER32.dll", "GetClientRect", Conv::Stdcall, 2, window_rect),
    ("USER32.dll", "GetWindowRect", Conv::Stdcall, 2, window_rect),
    ("USER32.dll", "ClientToScreen", Conv::Stdcall, 2, one),
    ("USER32.dll", "PeekMessageA", Conv::Stdcall, 5, peek_message),
    ("USER32.dll", "PeekMessageW", Conv::Stdcall, 5, peek_message),
    ("USER32.dll", "UpdateWindow", Conv::Stdcall, 1, one),
    ("USER32.dll", "SetCursorPos", Conv::Stdcall, 2, one),
    ("USER32.dll", "ClipCursor", Conv::Stdcall, 1, one),
    (
        "USER32.dll",
        "RegisterWindowMessageA",
        Conv::Stdcall,
        1,
        |_, _| Ret::eax(0xc100),
    ),
    (
        "USER32.dll",
        "RegisterDeviceNotificationA",
        Conv::Stdcall,
        3,
        new_handle,
    ),
    (
        "USER32.dll",
        "SetWindowsHookExA",
        Conv::Stdcall,
        4,
        new_handle,
    ),
    ("IMM32.dll", "ImmAssociateContext", Conv::Stdcall, 2, zero),
    // timers
    ("WINMM.dll", "timeBeginPeriod", Conv::Stdcall, 1, zero),
    ("WINMM.dll", "timeEndPeriod", Conv::Stdcall, 1, zero),
    (
        "WINMM.dll",
        "timeGetTime",
        Conv::Stdcall,
        0,
        clock::tick_count,
    ),
];
