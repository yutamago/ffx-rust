//! Native stand-ins for the `KERNEL32` functions `MSVCR110.dll` imports,
//! enough to initialise it and run its exports on one emulated thread.
//!
//! Everything is deterministic: clocks and counters are constants, the
//! process has one thread (id 1), handles are small fixed numbers, file
//! output is discarded. The heap functions allocate from the oracle's CRT
//! heap (blocks are always zeroed, `HEAP_ZERO_MEMORY` or not), which
//! [`crate::Oracle::reset_heap`] leaves alone so the runtime's own blocks
//! (per-thread data, stdio buffers) survive. Code pages are Windows-1252
//! (`GetACP`), the "C" locale.
//!
//! A stub runs before the code at its address, which is `ret 4n` for a
//! function of `n` arguments (stdcall).

use unicorn_engine::{RegisterX86, Unicorn};

use crate::crt::{read, write};
use crate::{Error, Result, State};

/// A `KERNEL32` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Api {
    EncodePointer,
    GetLastError,
    SetLastError,
    GetCurrentThreadId,
    GetCurrentProcessId,
    GetCurrentThread,
    GetCurrentProcess,
    GetTickCount,
    QueryPerformanceCounter,
    GetSystemTimeAsFileTime,
    IsProcessorFeaturePresent,
    Zero,
    One,
    GetProcessHeap,
    HeapAlloc,
    HeapReAlloc,
    HeapSize,
    TlsAlloc,
    TlsGetValue,
    TlsSetValue,
    GetModuleHandle(bool),
    GetModuleHandleExW,
    GetModuleFileName(bool),
    GetCommandLine(bool),
    GetEnvironmentStringsW,
    GetStartupInfoW,
    GetStdHandle,
    GetFileType,
    WriteFile,
    GetAcp,
    GetOemCp,
    GetCpInfo,
    MultiByteToWideChar,
    WideCharToMultiByte,
    GetStringTypeW,
    LcMapStringW,
    CompareStringW,
    GetUserDefaultLcid,
    InterlockedIncrement,
    InterlockedDecrement,
    InterlockedExchange,
    InterlockedCompareExchange,
    InterlockedExchangeAdd,
    GetVersionExW,
    GetSystemInfo,
    InitializeSListHead,
    /// The runtime's last resorts: stop with [`Error::RuntimeAbort`].
    Abort(&'static str),
    GetThreadTimes,
    /// `CreateMutexW`, `CreateSemaphoreW`, `CreateEventW`: a new handle.
    NewHandle,
    /// `ReleaseSemaphore(handle, count, previous)`.
    ReleaseSemaphore,
}

/// The stub for a `KERNEL32` import and its argument count.
// grouped by meaning, so some groups share a result
#[allow(clippy::enum_glob_use, clippy::match_same_arms)]
pub(crate) fn lookup(name: &str) -> Option<(Api, u8)> {
    use Api::*;
    Some(match name {
        "EncodePointer" | "DecodePointer" => (EncodePointer, 1),
        "GetLastError" => (GetLastError, 0),
        "SetLastError" => (SetLastError, 1),
        "GetCurrentThreadId" => (GetCurrentThreadId, 0),
        "GetCurrentProcessId" => (GetCurrentProcessId, 0),
        "GetCurrentThread" => (GetCurrentThread, 0),
        "GetCurrentProcess" => (GetCurrentProcess, 0),
        "GetTickCount" => (GetTickCount, 0),
        "QueryPerformanceCounter" => (QueryPerformanceCounter, 1),
        "GetSystemTimeAsFileTime" => (GetSystemTimeAsFileTime, 1),
        "IsProcessorFeaturePresent" => (IsProcessorFeaturePresent, 1),
        // nothing to do, or "no": the result is 0
        "IsDebuggerPresent" | "SwitchToThread" => (Zero, 0),
        "SetUnhandledExceptionFilter"
        | "SetErrorMode"
        | "OutputDebugStringA"
        | "OutputDebugStringW"
        | "Sleep"
        | "LoadLibraryW"
        | "GetConsoleCP" => (Zero, 1),
        "GetProcAddress" | "GetConsoleMode" => (Zero, 2),
        "LoadLibraryExW" | "LoadLibraryExA" => (Zero, 3),
        "HeapQueryInformation" => (Zero, 5),
        // single-threaded success
        "EnterCriticalSection"
        | "LeaveCriticalSection"
        | "DeleteCriticalSection"
        | "TryEnterCriticalSection"
        | "FreeEnvironmentStringsW"
        | "FreeLibrary"
        | "IsValidCodePage"
        | "TlsFree"
        | "CloseHandle"
        | "FlushFileBuffers" => (One, 1),
        "InitializeCriticalSectionAndSpinCount" | "HeapCompact" | "IsValidLocale" => (One, 2),
        "HeapFree" | "HeapValidate" => (One, 3),
        "GetProcessHeap" => (GetProcessHeap, 0),
        "HeapAlloc" => (HeapAlloc, 3),
        "HeapReAlloc" => (HeapReAlloc, 4),
        "HeapSize" => (HeapSize, 3),
        "TlsAlloc" => (TlsAlloc, 0),
        "TlsGetValue" => (TlsGetValue, 1),
        "TlsSetValue" => (TlsSetValue, 2),
        "GetModuleHandleW" => (GetModuleHandle(true), 1),
        "GetModuleHandleA" => (GetModuleHandle(false), 1),
        "GetModuleHandleExW" => (GetModuleHandleExW, 3),
        "GetModuleFileNameW" => (GetModuleFileName(true), 3),
        "GetModuleFileNameA" => (GetModuleFileName(false), 3),
        "GetCommandLineW" => (GetCommandLine(true), 0),
        "GetCommandLineA" => (GetCommandLine(false), 0),
        "GetEnvironmentStringsW" => (GetEnvironmentStringsW, 0),
        "GetStartupInfoW" => (GetStartupInfoW, 1),
        "GetStdHandle" => (GetStdHandle, 1),
        "GetFileType" => (GetFileType, 1),
        "WriteFile" => (WriteFile, 5),
        "GetACP" => (GetAcp, 0),
        "GetOEMCP" => (GetOemCp, 0),
        "GetCPInfo" => (GetCpInfo, 2),
        "MultiByteToWideChar" => (MultiByteToWideChar, 6),
        "WideCharToMultiByte" => (WideCharToMultiByte, 8),
        "GetStringTypeW" => (GetStringTypeW, 4),
        "LCMapStringW" => (LcMapStringW, 6),
        "CompareStringW" => (CompareStringW, 6),
        "GetUserDefaultLCID" => (GetUserDefaultLcid, 0),
        "InterlockedIncrement" => (InterlockedIncrement, 1),
        "InterlockedDecrement" => (InterlockedDecrement, 1),
        "InterlockedExchange" => (InterlockedExchange, 2),
        "InterlockedCompareExchange" => (InterlockedCompareExchange, 3),
        "InterlockedExchangeAdd" => (InterlockedExchangeAdd, 2),
        "GetVersionExW" => (GetVersionExW, 1),
        "GetSystemInfo" => (GetSystemInfo, 1),
        "InitializeSListHead" => (InitializeSListHead, 1),
        "GetThreadTimes" => (GetThreadTimes, 5),
        // threads and their objects: one thread, so every wait is satisfied
        // at once (WAIT_OBJECT_0) and every release succeeds
        "WaitForSingleObject" | "SleepEx" => (Zero, 2),
        "WaitForSingleObjectEx" => (Zero, 3),
        "ReleaseMutex" | "SetEvent" | "ResetEvent" => (One, 1),
        "ReleaseSemaphore" => (ReleaseSemaphore, 3),
        "CreateMutexW" | "CreateMutexA" => (NewHandle, 3),
        "CreateSemaphoreW" | "CreateSemaphoreA" => (NewHandle, 4),
        "CreateEventW" | "CreateEventA" => (NewHandle, 4),
        "UnhandledExceptionFilter" => (Abort("UnhandledExceptionFilter"), 1),
        "RaiseException" => (Abort("RaiseException"), 4),
        "TerminateProcess" => (Abort("TerminateProcess"), 2),
        "ExitProcess" => (Abort("ExitProcess"), 1),
        "DebugBreak" => (Abort("DebugBreak"), 0),
        _ => return None,
    })
}

/// The stub's code: `ret 4n`, or `ret` without arguments.
pub(crate) fn code(nargs: u8) -> Vec<u8> {
    if nargs == 0 {
        vec![0xc3]
    } else {
        vec![0xc2, 4 * nargs, 0]
    }
}

/// The pseudo handle `GetProcessHeap` returns.
const PROCESS_HEAP: u32 = 0x0050_0000;
/// The handle `GetModuleHandle("kernel32")` returns: not dereferenced,
/// since `GetProcAddress` finds nothing in it.
const KERNEL32: u32 = 0x7700_0000;
/// `FILE_TYPE_CHAR`: the standard handles look like a console.
const FILE_TYPE_CHAR: u32 = 2;
/// The `GetStdHandle` results for input, output and error.
const STD_HANDLES: [u32; 3] = [0x10, 0x14, 0x18];
/// The fixed clock: 2013-12-26 00:00:00 UTC (the remaster's year), in
/// 100 ns units since 1601.
const FILETIME: u64 = 0x01cf_01cd_6b78_4000;

/// Windows-1252's 0x80..0xa0 (0 where undefined).
const CP1252_HIGH: [u16; 32] = [
    0x20ac, 0, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160, 0x2039,
    0x0152, 0, 0x017d, 0, 0, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014, 0x02dc,
    0x2122, 0x0161, 0x203a, 0x0153, 0, 0x017e, 0x0178,
];

fn to_wide(b: u8) -> u16 {
    match b {
        0x80..=0x9f => match CP1252_HIGH[usize::from(b - 0x80)] {
            0 => u16::from(b),
            w => w,
        },
        _ => u16::from(b),
    }
}

fn from_wide(w: u16) -> Option<u8> {
    match w {
        0..=0x7f | 0xa0..=0xff => u8::try_from(w).ok(),
        _ => CP1252_HIGH
            .iter()
            .position(|&c| c == w && c != 0)
            .and_then(|i| u8::try_from(0x80 + i).ok())
            .or_else(|| u8::try_from(w).ok().filter(|b| (0x80..0xa0).contains(b))),
    }
}

/// `CT_CTYPE1` bits of a UTF-16 unit, as Windows classifies Latin-1.
fn ctype1(w: u16) -> u16 {
    const UPPER: u16 = 0x1;
    const LOWER: u16 = 0x2;
    const DIGIT: u16 = 0x4;
    const SPACE: u16 = 0x8;
    const PUNCT: u16 = 0x10;
    const CNTRL: u16 = 0x20;
    const BLANK: u16 = 0x40;
    const XDIGIT: u16 = 0x80;
    const ALPHA: u16 = 0x100;
    let Some(c) = char::from_u32(u32::from(w)) else {
        return 0;
    };
    let mut t = 0;
    if c.is_uppercase() {
        t |= UPPER | ALPHA;
    }
    if c.is_lowercase() {
        t |= LOWER | ALPHA;
    }
    if c.is_alphabetic() {
        t |= ALPHA;
    }
    if c.is_ascii_digit() {
        t |= DIGIT;
    }
    if c.is_ascii_hexdigit() {
        t |= XDIGIT;
    }
    if c.is_whitespace() || c == '\u{a0}' {
        t |= SPACE;
    }
    if c == ' ' || c == '\t' || c == '\u{a0}' {
        t |= BLANK;
    }
    if c.is_control() {
        t |= CNTRL;
    }
    if !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control() && c != '\u{a0}' {
        t |= PUNCT;
    }
    t
}

fn u32_at(uc: &mut Unicorn<State>, a: u32) -> Result<u32> {
    let b = read(uc, a, 4)?;
    Ok(u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn put_u32(uc: &mut Unicorn<State>, a: u32, v: u32) -> Result<()> {
    write(uc, a, &v.to_le_bytes())
}

/// The UTF-16 string at `p`, `len` units or up to the NUL (-1).
fn wide(uc: &mut Unicorn<State>, p: u32, len: u32) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    let mut a = p;
    loop {
        if len != u32::MAX && out.len() >= len as usize {
            return Ok(out);
        }
        let b = read(uc, a, 2)?;
        let w = u16::from_le_bytes([b[0], b[1]]);
        out.push(w);
        if len == u32::MAX && w == 0 {
            return Ok(out);
        }
        a = a.wrapping_add(2);
    }
}

fn bytes(uc: &mut Unicorn<State>, p: u32, len: u32) -> Result<Vec<u8>> {
    if len != u32::MAX {
        return read(uc, p, len as usize);
    }
    let mut s = crate::crt::cstr(uc, p)?;
    s.push(0);
    Ok(s)
}

/// Allocates `len` zeroed bytes on the CRT heap (the oracle's heap when
/// the runtime DLL is not loaded).
pub(crate) fn heap_alloc(uc: &mut Unicorn<State>, len: u32) -> Result<u32> {
    if uc.get_data().crt_heap.1 == 0 {
        return crate::alloc(uc, u64::from(len));
    }
    let s = uc.get_data_mut();
    let addr = s.crt_heap.0;
    let end = addr + u64::from(len.max(1)).next_multiple_of(16);
    if end > s.crt_heap.1 {
        return Err(Error::OutOfMemory);
    }
    s.crt_heap.0 = end;
    let a = u32::try_from(addr).expect("32-bit heap");
    s.heap_sizes.insert(a, len);
    write(uc, a, &vec![0; usize::try_from(end - addr).expect("size")])?;
    Ok(a)
}

/// A string the oracle hands out (command line, module name), allocated
/// once on the CRT heap.
fn constant(uc: &mut Unicorn<State>, key: &'static str, data: &[u8]) -> Result<u32> {
    if let Some(&p) = uc.get_data().constants.get(key) {
        return Ok(p);
    }
    let p = heap_alloc(uc, u32::try_from(data.len()).expect("small"))?;
    write(uc, p, data)?;
    uc.get_data_mut().constants.insert(key, p);
    Ok(p)
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain([0])
        .flat_map(u16::to_le_bytes)
        .collect()
}

const EXE_PATH: &str = "C:\\FFX\\FFX.exe";
const DLL_PATH: &str = "C:\\FFX\\MSVCR110.dll";

/// Runs one `KERNEL32` function; returns EAX.
#[allow(clippy::too_many_lines, clippy::match_same_arms)] // one match over the functions
pub(crate) fn run(uc: &mut Unicorn<State>, api: Api) -> Result<u32> {
    let esp = u32::try_from(uc.reg_read(RegisterX86::ESP)?).expect("32-bit");
    let mut a = [0u32; 8];
    for (k, v) in (0u32..).zip(a.iter_mut()) {
        *v = read(uc, esp + 4 + 4 * k, 4)
            .map_or(0, |b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
    }
    Ok(match api {
        Api::EncodePointer => a[0],
        Api::GetLastError => uc.get_data().last_error,
        Api::SetLastError => {
            uc.get_data_mut().last_error = a[0];
            0
        }
        Api::GetCurrentThreadId => 1,
        Api::GetCurrentProcessId => 0x10,
        Api::GetCurrentThread => 0xffff_fffe,
        Api::GetCurrentProcess => 0xffff_ffff,
        Api::GetTickCount => 0x0010_0000,
        Api::QueryPerformanceCounter => {
            write(uc, a[0], &0x0123_4567_89abu64.to_le_bytes())?;
            1
        }
        Api::GetSystemTimeAsFileTime => {
            write(uc, a[0], &FILETIME.to_le_bytes())?;
            0
        }
        // PF_COMPARE_EXCHANGE_DOUBLE, MMX, XMMI, RDTSC, XMMI64 (SSE2), as
        // on the machines the game runs on; not FASTFAIL (23).
        Api::IsProcessorFeaturePresent => u32::from(matches!(a[0], 2 | 3 | 6 | 8 | 10)),
        Api::Zero => 0,
        Api::One => 1,
        Api::GetProcessHeap => PROCESS_HEAP,
        Api::HeapAlloc => heap_alloc(uc, a[2])?,
        Api::HeapReAlloc => {
            let old = uc.get_data().heap_sizes.get(&a[2]).copied().unwrap_or(0);
            let p = heap_alloc(uc, a[3])?;
            let keep = read(uc, a[2], old.min(a[3]) as usize)?;
            write(uc, p, &keep)?;
            p
        }
        Api::HeapSize => uc
            .get_data()
            .heap_sizes
            .get(&a[2])
            .copied()
            .unwrap_or(u32::MAX),
        Api::TlsAlloc => {
            let s = uc.get_data_mut();
            s.tls.push(0);
            u32::try_from(s.tls.len() - 1).expect("few slots")
        }
        Api::TlsGetValue => {
            let s = uc.get_data_mut();
            s.last_error = 0;
            s.tls.get(a[0] as usize).copied().unwrap_or(0)
        }
        Api::TlsSetValue => match uc.get_data_mut().tls.get_mut(a[0] as usize) {
            Some(v) => {
                *v = a[1];
                1
            }
            None => 0,
        },
        Api::GetModuleHandle(w) => {
            if a[0] == 0 {
                uc.get_data().image_base
            } else {
                let name: String = if w {
                    String::from_utf16_lossy(&wide(uc, a[0], u32::MAX)?)
                } else {
                    String::from_utf8_lossy(&crate::crt::cstr(uc, a[0])?).into_owned()
                };
                let name = name.trim_end_matches('\0').to_ascii_lowercase();
                if name.starts_with("kernel32") {
                    KERNEL32
                } else if name.starts_with("msvcr110") {
                    uc.get_data().dll_base
                } else {
                    0
                }
            }
        }
        Api::GetModuleHandleExW => {
            // (flags, name or address, out handle): only ever asked about
            // the runtime itself, to pin it.
            let h = uc.get_data().dll_base;
            if a[2] != 0 {
                put_u32(uc, a[2], h)?;
            }
            1
        }
        Api::GetModuleFileName(w) => {
            let path = if a[0] == uc.get_data().dll_base {
                DLL_PATH
            } else {
                EXE_PATH
            };
            let n = path.len().min(a[2].saturating_sub(1) as usize);
            if w {
                write(uc, a[1], &utf16z(&path[..n]))?;
            } else {
                let mut v = path.as_bytes()[..n].to_vec();
                v.push(0);
                write(uc, a[1], &v)?;
            }
            u32::try_from(n).expect("short")
        }
        Api::GetCommandLine(w) => {
            let line = format!("\"{EXE_PATH}\"");
            if w {
                constant(uc, "cmdline-w", &utf16z(&line))?
            } else {
                let mut v = line.into_bytes();
                v.push(0);
                constant(uc, "cmdline-a", &v)?
            }
        }
        Api::GetEnvironmentStringsW => constant(uc, "environment", &[0, 0, 0, 0])?,
        Api::GetStartupInfoW => {
            let mut info = vec![0u8; 0x44];
            info[0] = 0x44;
            write(uc, a[0], &info)?;
            0
        }
        Api::GetStdHandle => match a[0] {
            0xffff_fff6 => STD_HANDLES[0],
            0xffff_fff5 => STD_HANDLES[1],
            0xffff_fff4 => STD_HANDLES[2],
            _ => 0xffff_ffff,
        },
        Api::GetFileType => {
            if STD_HANDLES.contains(&a[0]) {
                FILE_TYPE_CHAR
            } else {
                0
            }
        }
        Api::WriteFile => {
            // (handle, data, length, written, overlapped): discarded
            if a[3] != 0 {
                put_u32(uc, a[3], a[2])?;
            }
            1
        }
        Api::GetAcp => 1252,
        Api::GetOemCp => 437,
        Api::GetCpInfo => {
            // MaxCharSize 1, DefaultChar "?", no lead bytes
            let mut info = vec![0u8; 20];
            info[0] = 1;
            info[4] = b'?';
            write(uc, a[1], &info)?;
            1
        }
        Api::MultiByteToWideChar => {
            // (cp, flags, src, srclen, dst, dstlen)
            let src = bytes(uc, a[2], a[3])?;
            let out: Vec<u16> = src.iter().map(|&b| to_wide(b)).collect();
            let n = u32::try_from(out.len()).expect("short");
            if a[5] == 0 {
                n
            } else if n > a[5] {
                uc.get_data_mut().last_error = 122; // ERROR_INSUFFICIENT_BUFFER
                0
            } else {
                let b: Vec<u8> = out.iter().flat_map(|w| w.to_le_bytes()).collect();
                write(uc, a[4], &b)?;
                n
            }
        }
        Api::WideCharToMultiByte => {
            // (cp, flags, src, srclen, dst, dstlen, default, used default)
            let src = wide(uc, a[2], a[3])?;
            let mut used = false;
            let out: Vec<u8> = src
                .iter()
                .map(|&w| {
                    from_wide(w).unwrap_or_else(|| {
                        used = true;
                        b'?'
                    })
                })
                .collect();
            if a[7] != 0 {
                put_u32(uc, a[7], u32::from(used))?;
            }
            let n = u32::try_from(out.len()).expect("short");
            if a[5] == 0 {
                n
            } else if n > a[5] {
                uc.get_data_mut().last_error = 122;
                0
            } else {
                write(uc, a[4], &out)?;
                n
            }
        }
        Api::GetStringTypeW => {
            // (info type, src, len, types)
            let src = wide(uc, a[1], a[2])?;
            let types: Vec<u8> = src
                .iter()
                .map(|&w| if a[0] == 1 { ctype1(w) } else { 0 })
                .flat_map(u16::to_le_bytes)
                .collect();
            write(uc, a[3], &types)?;
            1
        }
        Api::LcMapStringW => {
            // (lcid, flags, src, srclen, dst, dstlen)
            let mut src = wide(uc, a[2], a[3])?;
            for w in &mut src {
                let Some(c) = char::from_u32(u32::from(*w)) else {
                    continue;
                };
                let m = if a[1] & 0x100 != 0 {
                    c.to_lowercase().next()
                } else if a[1] & 0x200 != 0 {
                    c.to_uppercase().next()
                } else {
                    Some(c)
                };
                // only mappings that stay one unit, as in Latin-1
                if let Some(m) = m
                    && let Ok(m) = u16::try_from(u32::from(m))
                    && (u32::from(m) < 0x100) == (u32::from(c) < 0x100)
                {
                    *w = m;
                }
            }
            let n = u32::try_from(src.len()).expect("short");
            if a[5] != 0 {
                let b: Vec<u8> = src.iter().flat_map(|w| w.to_le_bytes()).collect();
                write(uc, a[4], &b[..(2 * n.min(a[5])) as usize])?;
            }
            n
        }
        Api::CompareStringW => {
            // (lcid, flags, s1, n1, s2, n2): ordinal, 1/2/3 for </=/>
            let x = wide(uc, a[2], a[3])?;
            let y = wide(uc, a[4], a[5])?;
            let trim =
                |v: Vec<u16>| -> Vec<u16> { v.into_iter().take_while(|&w| w != 0).collect() };
            match trim(x).cmp(&trim(y)) {
                std::cmp::Ordering::Less => 1,
                std::cmp::Ordering::Equal => 2,
                std::cmp::Ordering::Greater => 3,
            }
        }
        Api::GetUserDefaultLcid => 0x0409,
        Api::InterlockedIncrement | Api::InterlockedDecrement => {
            let v = u32_at(uc, a[0])?;
            let v = if api == Api::InterlockedIncrement {
                v.wrapping_add(1)
            } else {
                v.wrapping_sub(1)
            };
            put_u32(uc, a[0], v)?;
            v
        }
        Api::InterlockedExchange => {
            let v = u32_at(uc, a[0])?;
            put_u32(uc, a[0], a[1])?;
            v
        }
        Api::InterlockedCompareExchange => {
            let v = u32_at(uc, a[0])?;
            if v == a[2] {
                put_u32(uc, a[0], a[1])?;
            }
            v
        }
        Api::InterlockedExchangeAdd => {
            let v = u32_at(uc, a[0])?;
            put_u32(uc, a[0], v.wrapping_add(a[1]))?;
            v
        }
        Api::GetVersionExW => {
            // OSVERSIONINFOW: Windows 7 SP1 (6.1.7601), NT platform
            for (off, v) in [(4, 6), (8, 1), (12, 7601), (16, 2)] {
                put_u32(uc, a[0] + off, v)?;
            }
            1
        }
        Api::GetSystemInfo => {
            let mut info = vec![0u8; 36];
            info[4..8].copy_from_slice(&0x1000u32.to_le_bytes()); // page size
            info[8..12].copy_from_slice(&0x1_0000u32.to_le_bytes()); // min address
            info[12..16].copy_from_slice(&0x7ffe_ffffu32.to_le_bytes()); // max address
            info[16..20].copy_from_slice(&1u32.to_le_bytes()); // processor mask
            info[20..24].copy_from_slice(&1u32.to_le_bytes()); // processors
            info[24..28].copy_from_slice(&586u32.to_le_bytes()); // type
            info[28..32].copy_from_slice(&0x1_0000u32.to_le_bytes()); // granularity
            info[32..34].copy_from_slice(&6u16.to_le_bytes()); // level
            write(uc, a[0], &info)?;
            0
        }
        Api::GetThreadTimes => {
            // (thread, creation, exit, kernel, user): created at the fixed
            // clock, no time spent
            write(uc, a[1], &FILETIME.to_le_bytes())?;
            for &p in &a[2..5] {
                write(uc, p, &[0; 8])?;
            }
            1
        }
        Api::Abort(name) => return Err(Error::RuntimeAbort(name)),
        Api::NewHandle => {
            let s = uc.get_data_mut();
            s.handles += 1;
            0x100 + 4 * s.handles
        }
        Api::ReleaseSemaphore => {
            if a[2] != 0 {
                put_u32(uc, a[2], 0)?;
            }
            1
        }
        Api::InitializeSListHead => {
            write(uc, a[0], &[0; 8])?;
            0
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_pages() {
        assert_eq!(to_wide(0x80), 0x20ac);
        assert_eq!(from_wide(0x20ac), Some(0x80));
        assert_eq!(to_wide(b'a'), u16::from(b'a'));
        assert_eq!(from_wide(0xe9), Some(0xe9));
        assert_eq!(from_wide(0x3042), None);
    }

    #[test]
    fn classes() {
        assert_eq!(ctype1(u16::from(b'A')), 0x181);
        assert_eq!(ctype1(u16::from(b'7')), 0x84);
        assert_eq!(ctype1(u16::from(b' ')), 0x48);
        assert_eq!(ctype1(u16::from(b'!')), 0x10);
        assert_eq!(ctype1(0xe9), 0x102);
    }
}
