//! `KERNEL32` stand-ins for threads, synchronisation and the C runtime
//! DLLs (docs/host.md, "Threads" and "The C runtime DLLs").
//!
//! - Threads: `CreateThread` runs the routine on a host thread of its own
//!   with its own Unicorn instance over the shared arena
//!   ([`Exec::spawn_thread`]); the thread pool's work items too.
//! - Synchronisation: events, semaphores, mutexes, thread handles and
//!   critical sections on real host primitives ([`crate::sync`]); the
//!   `Interlocked*` functions are host atomics on the arena's memory.
//! - What `MSVCR110.dll` and `MSVCP110.dll` ask of the system while they
//!   start and run: TLS slots in each thread's TEB, the code pages
//!   (Windows-1252, the "C" locale), the standard handles, the module
//!   handles of the DLLs.
//!
//! [`TABLE`] is looked up before [`crate::imports::TABLE`].

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use ffx_bridge::Mem;

use crate::exec::{Exec, Stop, TEB_TLS};
use crate::imports::{ImportFn, a, read_str, write_str};
use crate::sync::{INFINITE, Obj, SLICE, WAIT_FAILED};
use crate::table::{Args, Conv, Ret};

const K32: &str = "KERNEL32.dll";

// ---- threads -------------------------------------------------------------

/// `CreateThread(attr, stack, entry, param, flags, &id)`: the routine runs
/// on its own host thread (`CREATE_SUSPENDED`, 4: at `ResumeThread`).
fn create_thread(e: &mut Exec, args: &Args) -> Ret {
    let suspended = a(args, 4) & 4 != 0;
    let (handle, id) = e.spawn_thread(a(args, 2), a(args, 3), suspended, "CreateThread");
    if a(args, 5) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 5), id);
    }
    Ret::eax(handle)
}

fn resume_thread(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(e.resume_thread(a(args, 0)))
}

/// `ExitThread(code)`: ends this thread (the main thread: the run).
fn exit_thread(e: &mut Exec, args: &Args) -> Ret {
    e.end(Stop::ThreadExit(a(args, 0)));
    Ret::VOID
}

/// `GetExitCodeThread(h, &code)`.
fn exit_code_thread(e: &mut Exec, args: &Args) -> Ret {
    let Some(code) = e.shared().sync.thread_code(a(args, 0)) else {
        return Ret::eax(0);
    };
    let mut mem = e.mem();
    mem.write(a(args, 1), code);
    Ret::eax(1)
}

fn current_thread_id(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(e.id)
}

/// `CreateThreadpoolWork(callback, context, environment)`.
fn create_pool_work(e: &mut Exec, args: &Args) -> Ret {
    let h = e.new_handle();
    if let Ok(mut w) = e.shared().pool_work.lock() {
        w.insert(h, (a(args, 0), a(args, 1)));
    }
    Ret::eax(h)
}

/// `SubmitThreadpoolWork(work)`.
fn submit_pool_work(e: &mut Exec, args: &Args) -> Ret {
    e.submit_work(a(args, 0));
    Ret::VOID
}

// ---- synchronisation -----------------------------------------------------

/// `CreateEventA/W(attr, manual, initial, name)`.
fn create_event(e: &mut Exec, args: &Args) -> Ret {
    let h = e.new_handle();
    e.shared().sync.insert(
        h,
        Obj::Event {
            manual: a(args, 1) != 0,
            set: a(args, 2) != 0,
        },
    );
    Ret::eax(h)
}

fn set_event(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(e.shared().sync.set_event(a(args, 0), true)))
}

fn reset_event(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(u32::from(e.shared().sync.set_event(a(args, 0), false)))
}

/// `CreateSemaphoreA/W(attr, initial, maximum, name)`.
fn create_semaphore(e: &mut Exec, args: &Args) -> Ret {
    let h = e.new_handle();
    e.shared().sync.insert(
        h,
        Obj::Semaphore {
            count: a(args, 1).cast_signed(),
            max: a(args, 2).cast_signed(),
        },
    );
    Ret::eax(h)
}

/// `ReleaseSemaphore(h, count, &previous)`.
fn release_semaphore(e: &mut Exec, args: &Args) -> Ret {
    let prev = e
        .shared()
        .sync
        .release_semaphore(a(args, 0), a(args, 1).cast_signed());
    match prev {
        Some(prev) => {
            if a(args, 2) != 0 {
                let mut mem = e.mem();
                mem.write(a(args, 2), prev);
            }
            Ret::eax(1)
        }
        None => {
            e.crt.last_error = 298; // ERROR_TOO_MANY_POSTS
            Ret::eax(0)
        }
    }
}

/// `CreateMutexA/W(attr, initial owner, name)`.
fn create_mutex(e: &mut Exec, args: &Args) -> Ret {
    let h = e.new_handle();
    let owned = a(args, 1) != 0;
    e.shared().sync.insert(
        h,
        Obj::Mutex {
            owner: if owned { e.id } else { 0 },
            count: u32::from(owned),
        },
    );
    Ret::eax(h)
}

fn release_mutex(e: &mut Exec, args: &Args) -> Ret {
    let id = e.id;
    Ret::eax(u32::from(e.shared().sync.release_mutex(a(args, 0), id)))
}

/// `WaitForSingleObject(h, ms)` / `WaitForSingleObjectEx(h, ms, alertable)`.
fn wait_single(e: &mut Exec, args: &Args) -> Ret {
    let id = e.id;
    let r = e.shared().sync.wait(id, &[a(args, 0)], false, a(args, 1));
    Ret::eax(r)
}

/// `WaitForMultipleObjects(n, handles, all, ms)`.
fn wait_multiple(e: &mut Exec, args: &Args) -> Ret {
    let n = a(args, 0).min(64);
    let mem = e.mem();
    let handles: Vec<u32> = (0..n).map(|i| mem.read(a(args, 1) + 4 * i)).collect();
    if handles.is_empty() {
        return Ret::eax(WAIT_FAILED);
    }
    let id = e.id;
    let r = e
        .shared()
        .sync
        .wait(id, &handles, a(args, 2) != 0, a(args, 3));
    Ret::eax(r)
}

/// `CloseHandle(h)`: files and objects.
fn close_handle(e: &mut Exec, args: &Args) -> Ret {
    e.files().close(a(args, 0));
    e.shared().sync.remove(a(args, 0));
    Ret::eax(1)
}

/// `Sleep(ms)`: a real sleep (other threads run meanwhile), cut short when
/// the run ends.
fn sleep(e: &mut Exec, args: &Args) -> Ret {
    let ms = a(args, 0);
    if ms == 0 {
        std::thread::yield_now();
        return Ret::VOID;
    }
    let mut left = if ms == INFINITE {
        Duration::MAX
    } else {
        Duration::from_millis(ms.into())
    };
    while !left.is_zero() && !e.shared().sync.stopping() {
        let step = left.min(SLICE);
        std::thread::sleep(step);
        left = left.saturating_sub(step);
    }
    Ret::VOID
}

fn sleep_ex(e: &mut Exec, args: &Args) -> Ret {
    sleep(e, args);
    Ret::eax(0)
}

fn switch_to_thread(_: &mut Exec, _: &Args) -> Ret {
    std::thread::yield_now();
    Ret::eax(1)
}

fn section_init(e: &mut Exec, args: &Args) -> Ret {
    e.shared().sync.section_init(a(args, 0));
    Ret::eax(1)
}

fn section_enter(e: &mut Exec, args: &Args) -> Ret {
    let id = e.id;
    if !e.shared().sync.section_enter(a(args, 0), id) {
        let why = e.shared().stopped().unwrap_or(Stop::Returned);
        e.end(why);
    }
    Ret::VOID
}

fn section_try(e: &mut Exec, args: &Args) -> Ret {
    let id = e.id;
    Ret::eax(u32::from(e.shared().sync.section_try(a(args, 0), id)))
}

fn section_leave(e: &mut Exec, args: &Args) -> Ret {
    let id = e.id;
    e.shared().sync.section_leave(a(args, 0), id);
    Ret::VOID
}

fn section_delete(e: &mut Exec, args: &Args) -> Ret {
    e.shared().sync.section_delete(a(args, 0));
    Ret::VOID
}

/// The arena word at `addr` as a host atomic (`None`: unmapped or
/// unaligned).
fn atomic(e: &Exec, addr: u32) -> Option<&AtomicU32> {
    if !addr.is_multiple_of(4) || !e.arena().is_mapped(addr, 4) {
        return None;
    }
    // SAFETY: a mapped, aligned word of the arena, which outlives `e`;
    // every thread's access to it goes through atomics or the emulator.
    Some(unsafe { AtomicU32::from_ptr(e.arena().ptr(addr).cast()) })
}

/// `InterlockedExchangeAdd(&v, by)` and friends: the old value.
fn fetch_add(e: &Exec, addr: u32, by: u32) -> u32 {
    atomic(e, addr).map_or(0, |x| x.fetch_add(by, Ordering::SeqCst))
}

fn interlocked_inc(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(fetch_add(e, a(args, 0), 1).wrapping_add(1))
}

fn interlocked_dec(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(fetch_add(e, a(args, 0), u32::MAX).wrapping_sub(1))
}

fn interlocked_xadd(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(fetch_add(e, a(args, 0), a(args, 1)))
}

/// `InterlockedExchange(&v, new)`.
fn interlocked_xchg(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(atomic(e, a(args, 0)).map_or(0, |x| x.swap(a(args, 1), Ordering::SeqCst)))
}

/// `InterlockedCompareExchange(&v, new, cmp)`.
fn interlocked_cmpxchg(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(atomic(e, a(args, 0)).map_or(0, |x| {
        match x.compare_exchange(a(args, 2), a(args, 1), Ordering::SeqCst, Ordering::SeqCst) {
            Ok(v) | Err(v) => v,
        }
    }))
}

// ---- thread-local storage ------------------------------------------------

fn tls_alloc(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(e.shared().tls_alloc().unwrap_or(u32::MAX))
}

/// `TlsGetValue(i)`: the slot in this thread's TEB.
fn tls_get(e: &mut Exec, args: &Args) -> Ret {
    let i = a(args, 0);
    if i >= 64 {
        e.crt.last_error = 87;
        return Ret::eax(0);
    }
    e.crt.last_error = 0;
    let v: u32 = e.mem().read(e.teb + TEB_TLS + 4 * i);
    Ret::eax(v)
}

fn tls_set(e: &mut Exec, args: &Args) -> Ret {
    let i = a(args, 0);
    if i >= 64 {
        e.crt.last_error = 87;
        return Ret::eax(0);
    }
    let mut mem = e.mem();
    mem.write(e.teb + TEB_TLS + 4 * i, a(args, 1));
    Ret::eax(1)
}

// ---- modules, the command line, the console ------------------------------

/// The handle `GetModuleHandle("kernel32")` returns: never dereferenced
/// (`GetProcAddress` finds nothing in it, so the CRT takes its fallbacks).
const KERNEL32: u32 = 0x7700_0000;

fn module_by_name(e: &Exec, name: &str) -> u32 {
    let lower = name.to_ascii_lowercase();
    let file = lower.rsplit(['\\', '/']).next().unwrap_or(&lower);
    let stem = file.trim_end_matches(".dll");
    if stem == "kernel32" {
        return KERNEL32;
    }
    if file == "ffx.exe" {
        return e.image().base;
    }
    e.shared()
        .dlls
        .iter()
        .find(|d| d.name.to_ascii_lowercase().trim_end_matches(".dll") == stem)
        .map_or(0, |d| d.base)
}

/// `GetModuleHandleA/W(name)`: the image for NULL, the DLLs by name.
fn module_handle(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    if a(args, 0) == 0 {
        return Ret::eax(e.image().base);
    }
    let name = read_str(&e.mem(), a(args, 0), wide);
    let h = module_by_name(e, &name);
    if h == 0 {
        e.crt.last_error = 126; // ERROR_MOD_NOT_FOUND
    }
    Ret::eax(h)
}

fn module_handle_a(e: &mut Exec, args: &Args) -> Ret {
    module_handle(e, args, false)
}

fn module_handle_w(e: &mut Exec, args: &Args) -> Ret {
    module_handle(e, args, true)
}

/// `GetModuleHandleExW(flags, name or address, &module)`.
fn module_handle_ex(e: &mut Exec, args: &Args) -> Ret {
    let h = if a(args, 0) & 4 != 0 {
        // GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
        let at = a(args, 1);
        e.shared()
            .dlls
            .iter()
            .find(|d| d.contains(at))
            .map_or(e.image().base, |d| d.base)
    } else if a(args, 1) == 0 {
        e.image().base
    } else {
        let name = read_str(&e.mem(), a(args, 1), true);
        module_by_name(e, &name)
    };
    if a(args, 2) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 2), h);
    }
    Ret::eax(u32::from(h != 0))
}

/// `GetModuleFileNameA/W(module, buf, size)`: `<game>\FFX.exe`, or the
/// DLL's own path.
fn module_file_name(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let dll = e
        .shared()
        .dlls
        .iter()
        .find(|d| d.base == a(args, 0))
        .map(|d| d.name.clone());
    let name = format!(
        "{}\\{}",
        crate::files::GAME_DIR_WIN,
        dll.as_deref().unwrap_or("FFX.exe")
    );
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

fn command_line_a(e: &mut Exec, _: &Args) -> Ret {
    Ret::eax(e.shared().cmdline)
}

/// `GetEnvironmentStringsW()`: an empty block.
fn environment_strings(e: &mut Exec, _: &Args) -> Ret {
    let p = e.alloc(4);
    let mut mem = e.mem();
    mem.write(p, 0u32);
    Ret::eax(p)
}

/// The standard handles (`GetStdHandle(-10/-11/-12)`).
const STD_HANDLES: [u32; 3] = [0x10, 0x14, 0x18];

fn std_handle(_: &mut Exec, args: &Args) -> Ret {
    Ret::eax(match a(args, 0) {
        0xffff_fff6 => STD_HANDLES[0],
        0xffff_fff5 => STD_HANDLES[1],
        0xffff_fff4 => STD_HANDLES[2],
        _ => u32::MAX,
    })
}

/// `GetFileType(h)`: the standard handles are a console
/// (`FILE_TYPE_CHAR`), open files disks; anything else unknown.
fn file_type(e: &mut Exec, args: &Args) -> Ret {
    let h = a(args, 0);
    if STD_HANDLES.contains(&h) {
        return Ret::eax(2);
    }
    let open = e.files().size(h).is_some();
    Ret::eax(u32::from(open))
}

/// `SetFilePointerEx(h, distance (2 words), &new, method)`.
fn set_file_pointer_ex(e: &mut Exec, args: &Args) -> Ret {
    let to = (u64::from(a(args, 2)) << 32 | u64::from(a(args, 1))).cast_signed();
    let pos = e.files().seek(a(args, 0), to, a(args, 4));
    match pos {
        Some(p) => {
            if a(args, 3) != 0 {
                let mut mem = e.mem();
                mem.write(a(args, 3), p);
            }
            Ret::eax(1)
        }
        None => Ret::eax(0),
    }
}

// ---- code pages and locales (Windows-1252, the "C" locale) ---------------

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

/// `len` bytes at `p`, or up to and with the NUL for -1.
fn bytes_at(mem: &impl Mem, p: u32, len: u32) -> Vec<u8> {
    if len != u32::MAX {
        let mut v = vec![0; len as usize];
        mem.read_bytes(p, &mut v);
        return v;
    }
    let mut v = Vec::new();
    loop {
        let b: u8 = mem.read(p + u32::try_from(v.len()).unwrap_or(0));
        v.push(b);
        if b == 0 || v.len() > 0x10_0000 {
            return v;
        }
    }
}

/// `len` UTF-16 units at `p`, or up to and with the NUL for -1.
fn units_at(mem: &impl Mem, p: u32, len: u32) -> Vec<u16> {
    let mut v = Vec::new();
    loop {
        if len != u32::MAX && v.len() >= len as usize {
            return v;
        }
        let w: u16 = mem.read(p + 2 * u32::try_from(v.len()).unwrap_or(0));
        v.push(w);
        if (len == u32::MAX && w == 0) || v.len() > 0x10_0000 {
            return v;
        }
    }
}

/// `MultiByteToWideChar(cp, flags, src, srclen, dst, dstlen)`.
fn mb_to_wide(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let out: Vec<u16> = bytes_at(&mem, a(args, 2), a(args, 3))
        .into_iter()
        .map(to_wide)
        .collect();
    let n = u32::try_from(out.len()).unwrap_or(0);
    if a(args, 5) == 0 {
        return Ret::eax(n);
    }
    if n > a(args, 5) {
        e.crt.last_error = 122; // ERROR_INSUFFICIENT_BUFFER
        return Ret::eax(0);
    }
    for (i, w) in (0..).zip(out) {
        mem.write(a(args, 4) + 2 * i, w);
    }
    Ret::eax(n)
}

/// `WideCharToMultiByte(cp, flags, src, srclen, dst, dstlen, default,
/// &used default)`.
fn wide_to_mb(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let mut used = false;
    let out: Vec<u8> = units_at(&mem, a(args, 2), a(args, 3))
        .into_iter()
        .map(|w| {
            from_wide(w).unwrap_or_else(|| {
                used = true;
                b'?'
            })
        })
        .collect();
    if a(args, 7) != 0 {
        mem.write(a(args, 7), u32::from(used));
    }
    let n = u32::try_from(out.len()).unwrap_or(0);
    if a(args, 5) == 0 {
        return Ret::eax(n);
    }
    if n > a(args, 5) {
        e.crt.last_error = 122;
        return Ret::eax(0);
    }
    mem.write_bytes(a(args, 4), &out);
    Ret::eax(n)
}

/// `CT_CTYPE1` bits of a UTF-16 unit, as Windows classifies Latin-1.
fn ctype1(w: u16) -> u16 {
    let Some(c) = char::from_u32(u32::from(w)) else {
        return 0;
    };
    let mut t = 0;
    if c.is_uppercase() {
        t |= 0x1 | 0x100;
    }
    if c.is_lowercase() {
        t |= 0x2 | 0x100;
    }
    if c.is_alphabetic() {
        t |= 0x100;
    }
    if c.is_ascii_digit() {
        t |= 0x4;
    }
    if c.is_ascii_hexdigit() {
        t |= 0x80;
    }
    if c.is_whitespace() || c == '\u{a0}' {
        t |= 0x8;
    }
    if c == ' ' || c == '\t' || c == '\u{a0}' {
        t |= 0x40;
    }
    if c.is_control() {
        t |= 0x20;
    }
    if !c.is_alphanumeric() && !c.is_whitespace() && !c.is_control() && c != '\u{a0}' {
        t |= 0x10;
    }
    t
}

/// `GetStringTypeW(info type, src, len, types)`.
fn string_type_w(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let src = units_at(&mem, a(args, 1), a(args, 2));
    for (i, w) in (0..).zip(src) {
        let t = if a(args, 0) == 1 { ctype1(w) } else { 0 };
        mem.write(a(args, 3) + 2 * i, t);
    }
    Ret::eax(1)
}

/// `LCMapStringW(lcid, flags, src, srclen, dst, dstlen)`: lower or upper
/// case (`LCMAP_LOWERCASE` 0x100, `LCMAP_UPPERCASE` 0x200), one unit to
/// one unit.
fn lc_map_w(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let mut src = units_at(&mem, a(args, 2), a(args, 3));
    for w in &mut src {
        let Some(c) = char::from_u32(u32::from(*w)) else {
            continue;
        };
        let m = if a(args, 1) & 0x100 != 0 {
            c.to_lowercase().next()
        } else if a(args, 1) & 0x200 != 0 {
            c.to_uppercase().next()
        } else {
            Some(c)
        };
        if let Some(m) = m
            && let Ok(m) = u16::try_from(u32::from(m))
            && (u32::from(m) < 0x100) == (u32::from(c) < 0x100)
        {
            *w = m;
        }
    }
    let n = u32::try_from(src.len()).unwrap_or(0);
    if a(args, 5) != 0 {
        for (i, w) in (0..n.min(a(args, 5))).zip(src) {
            mem.write(a(args, 4) + 2 * i, w);
        }
    }
    Ret::eax(n)
}

/// `CompareStringW(lcid, flags, s1, n1, s2, n2)`: ordinal; 1, 2, 3 for
/// less, equal, greater.
fn compare_string_w(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let trim = |v: Vec<u16>| -> Vec<u16> { v.into_iter().take_while(|&w| w != 0).collect() };
    let x = trim(units_at(&mem, a(args, 2), a(args, 3)));
    let y = trim(units_at(&mem, a(args, 4), a(args, 5)));
    Ret::eax(match x.cmp(&y) {
        std::cmp::Ordering::Less => 1,
        std::cmp::Ordering::Equal => 2,
        std::cmp::Ordering::Greater => 3,
    })
}

/// `GetCPInfo(cp, &info)`: one byte per character, `?` as the default, no
/// lead bytes.
fn cp_info(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let mut info = [0u8; 20];
    info[0] = 1;
    info[4] = b'?';
    mem.write_bytes(a(args, 1), &info);
    Ret::eax(1)
}

/// `GetVersionExW(&info)`: as `GetVersionExA` (6.2, manifest-less).
fn version_ex_w(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    let p = a(args, 0);
    for (off, v) in [(4, 6u32), (8, 2), (12, 9200), (16, 2)] {
        mem.write(p + off, v);
    }
    Ret::eax(1)
}

fn slist_init(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 0), 0u64);
    Ret::VOID
}

/// `GetThreadTimes(thread, &creation, &exit, &kernel, &user)`: created at
/// the fixed clock, no time spent.
fn thread_times(e: &mut Exec, args: &Args) -> Ret {
    let mut mem = e.mem();
    mem.write(a(args, 1), 0x01d0_0000_0000_0000u64);
    for i in 2..5 {
        mem.write(a(args, i), 0u64);
    }
    Ret::eax(1)
}

/// `RaiseException(code, flags, n, args)`: no SEH dispatch yet; a C++
/// `throw` (`0xe06d7363`) ends the thread with its code.
fn raise_exception(e: &mut Exec, args: &Args) -> Ret {
    e.end(Stop::Failed(
        format!(
            "RaiseException({:#010x}): no exception dispatch yet",
            a(args, 0)
        ),
        0,
    ));
    Ret::VOID
}

fn give_up(e: &mut Exec, _: &Args) -> Ret {
    e.end(Stop::Failed(
        "the C runtime gave up (TerminateProcess, UnhandledExceptionFilter or DebugBreak)".into(),
        0,
    ));
    Ret::eax(0)
}

fn zero(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

fn one(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(1)
}

/// Every stand-in here: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    // threads
    (K32, "CreateThread", Conv::Stdcall, 6, create_thread),
    (K32, "ResumeThread", Conv::Stdcall, 1, resume_thread),
    (K32, "SuspendThread", Conv::Stdcall, 1, zero),
    (K32, "ExitThread", Conv::Stdcall, 1, exit_thread),
    (K32, "GetExitCodeThread", Conv::Stdcall, 2, exit_code_thread),
    (
        K32,
        "GetCurrentThreadId",
        Conv::Stdcall,
        0,
        current_thread_id,
    ),
    (
        K32,
        "CreateThreadpoolWork",
        Conv::Stdcall,
        3,
        create_pool_work,
    ),
    (
        K32,
        "SubmitThreadpoolWork",
        Conv::Stdcall,
        1,
        submit_pool_work,
    ),
    // synchronisation
    (K32, "CreateEventA", Conv::Stdcall, 4, create_event),
    (K32, "CreateEventW", Conv::Stdcall, 4, create_event),
    (K32, "SetEvent", Conv::Stdcall, 1, set_event),
    (K32, "ResetEvent", Conv::Stdcall, 1, reset_event),
    (K32, "CreateSemaphoreA", Conv::Stdcall, 4, create_semaphore),
    (K32, "CreateSemaphoreW", Conv::Stdcall, 4, create_semaphore),
    (K32, "ReleaseSemaphore", Conv::Stdcall, 3, release_semaphore),
    (K32, "CreateMutexA", Conv::Stdcall, 3, create_mutex),
    (K32, "CreateMutexW", Conv::Stdcall, 3, create_mutex),
    (K32, "ReleaseMutex", Conv::Stdcall, 1, release_mutex),
    (K32, "WaitForSingleObject", Conv::Stdcall, 2, wait_single),
    (K32, "WaitForSingleObjectEx", Conv::Stdcall, 3, wait_single),
    (
        K32,
        "WaitForMultipleObjects",
        Conv::Stdcall,
        4,
        wait_multiple,
    ),
    (K32, "CloseHandle", Conv::Stdcall, 1, close_handle),
    (K32, "Sleep", Conv::Stdcall, 1, sleep),
    (K32, "SleepEx", Conv::Stdcall, 2, sleep_ex),
    (K32, "SwitchToThread", Conv::Stdcall, 0, switch_to_thread),
    (
        K32,
        "InitializeCriticalSection",
        Conv::Stdcall,
        1,
        section_init,
    ),
    (
        K32,
        "InitializeCriticalSectionEx",
        Conv::Stdcall,
        3,
        section_init,
    ),
    (
        K32,
        "InitializeCriticalSectionAndSpinCount",
        Conv::Stdcall,
        2,
        section_init,
    ),
    (K32, "EnterCriticalSection", Conv::Stdcall, 1, section_enter),
    (
        K32,
        "TryEnterCriticalSection",
        Conv::Stdcall,
        1,
        section_try,
    ),
    (K32, "LeaveCriticalSection", Conv::Stdcall, 1, section_leave),
    (
        K32,
        "DeleteCriticalSection",
        Conv::Stdcall,
        1,
        section_delete,
    ),
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
        "InterlockedExchange",
        Conv::Stdcall,
        2,
        interlocked_xchg,
    ),
    (
        K32,
        "InterlockedCompareExchange",
        Conv::Stdcall,
        3,
        interlocked_cmpxchg,
    ),
    // thread-local storage
    (K32, "TlsAlloc", Conv::Stdcall, 0, tls_alloc),
    (K32, "TlsFree", Conv::Stdcall, 1, one),
    (K32, "TlsGetValue", Conv::Stdcall, 1, tls_get),
    (K32, "TlsSetValue", Conv::Stdcall, 2, tls_set),
    // modules, the command line, the console
    (K32, "GetModuleHandleA", Conv::Stdcall, 1, module_handle_a),
    (K32, "GetModuleHandleW", Conv::Stdcall, 1, module_handle_w),
    (
        K32,
        "GetModuleHandleExW",
        Conv::Stdcall,
        3,
        module_handle_ex,
    ),
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
    (K32, "GetCommandLineA", Conv::Stdcall, 0, command_line_a),
    (
        K32,
        "GetEnvironmentStringsW",
        Conv::Stdcall,
        0,
        environment_strings,
    ),
    (K32, "FreeEnvironmentStringsW", Conv::Stdcall, 1, one),
    (K32, "GetStdHandle", Conv::Stdcall, 1, std_handle),
    (K32, "GetFileType", Conv::Stdcall, 1, file_type),
    (
        K32,
        "SetFilePointerEx",
        Conv::Stdcall,
        5,
        set_file_pointer_ex,
    ),
    (K32, "FlushFileBuffers", Conv::Stdcall, 1, one),
    (K32, "GetConsoleCP", Conv::Stdcall, 0, zero),
    (K32, "GetConsoleMode", Conv::Stdcall, 2, zero),
    (K32, "SetErrorMode", Conv::Stdcall, 1, zero),
    (K32, "HeapValidate", Conv::Stdcall, 3, one),
    // PF_COMPARE_EXCHANGE_DOUBLE, MMX, XMMI, RDTSC, XMMI64 (SSE2), as on
    // the machines the game runs on; not FASTFAIL (23), so the C runtime's
    // invalid-parameter path reports instead of `int 0x29`
    (
        K32,
        "IsProcessorFeaturePresent",
        Conv::Stdcall,
        1,
        |_, args| Ret::eax(u32::from(matches!(a(args, 0), 2 | 3 | 6 | 8 | 10))),
    ),
    (K32, "DisableThreadLibraryCalls", Conv::Stdcall, 1, one),
    (K32, "HeapQueryInformation", Conv::Stdcall, 5, zero),
    (K32, "VirtualQuery", Conv::Stdcall, 3, zero),
    (K32, "GetThreadTimes", Conv::Stdcall, 5, thread_times),
    (K32, "InitializeSListHead", Conv::Stdcall, 1, slist_init),
    (K32, "GetVersionExW", Conv::Stdcall, 1, version_ex_w),
    // code pages and locales
    (K32, "GetACP", Conv::Stdcall, 0, |_, _| Ret::eax(1252)),
    (K32, "GetOEMCP", Conv::Stdcall, 0, |_, _| Ret::eax(437)),
    (K32, "GetCPInfo", Conv::Stdcall, 2, cp_info),
    (K32, "IsValidCodePage", Conv::Stdcall, 1, one),
    (K32, "MultiByteToWideChar", Conv::Stdcall, 6, mb_to_wide),
    (K32, "WideCharToMultiByte", Conv::Stdcall, 8, wide_to_mb),
    (K32, "GetStringTypeW", Conv::Stdcall, 4, string_type_w),
    (K32, "LCMapStringW", Conv::Stdcall, 6, lc_map_w),
    (K32, "CompareStringW", Conv::Stdcall, 6, compare_string_w),
    (K32, "GetUserDefaultLCID", Conv::Stdcall, 0, |_, _| {
        Ret::eax(0x409)
    }),
    (K32, "IsValidLocale", Conv::Stdcall, 2, one),
    // the runtime's last resorts
    (K32, "RaiseException", Conv::Stdcall, 4, raise_exception),
    (K32, "UnhandledExceptionFilter", Conv::Stdcall, 1, give_up),
    (K32, "TerminateProcess", Conv::Stdcall, 2, give_up),
    (K32, "DebugBreak", Conv::Stdcall, 0, give_up),
];
