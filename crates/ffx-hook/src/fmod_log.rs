//! FMOD call log: every call `FFX.exe` makes into FMOD, with its arguments,
//! result and out-values (`docs/formats/audio-platform.md`).
//!
//! With `FFX_FMOD_LOG=<file>` (a Windows path; `tools/run-game.sh` passes it
//! through, `/p` in `WSLENV`) the hook points every `FFX.exe` import of
//! `fmod_event.dll` and `fmodex.dll` at a logging stub, and every entry of
//! `fmod_event.dll`'s own vtables that is a named export (the interfaces
//! the game calls virtually: `EventProjectI`, `EventGroupI`, ...). A stub
//! pushes its index and enters [`enter`]; for a call from `FFX.exe` (the
//! return address in the image; FMOD's calls among its own objects pass
//! straight through) the return address is swapped for [`after`], which
//! writes one line once the callee has returned (`ret n`: the words it
//! popped are its argument count) and goes back to the caller. Nested calls
//! (FMOD callbacks into the game that call FMOD again) keep a per-thread
//! stack.
//!
//! Nothing else changes: the stubs pass every register and stack word
//! through untouched, so the sound stays as set (`FFX_TEST_VOLUME=0` keeps
//! it off). Without the variable nothing is patched.
//!
//! The file (read by `crates/ffx-audio-census`):
//!
//! ```text
//! # fmodlog 1
//! # fn <index> <dll> <import|vtable@<slot>> <target> <decorated name>
//! # thread <tid> <start address: FFX.exe address or module+offset>
//! # update <n> <dsp clock after it> <result>   (with FFX_AUDIO_WAV: crate::audio_wav)
//! <seq> <ms> <frame> <tid> <caller> <index> <words> <arg,arg,...> <result> [o<i>=<word>]... [s<i>="<text>"]...
//! ```
//!
//! Words are hex; `caller` is the `FFX.exe` address the call returns to;
//! `frame` the harness frame count; `o<i>` the word argument `i` points at
//! after the call (when it is a readable pointer), `s<i>` the string it
//! pointed at before (when it is one).

use std::cell::RefCell;
use std::ffi::{CStr, c_void};
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    GetModuleFileNameA, GetModuleHandleA, GetModuleHandleExA, GetProcAddress,
};
use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEMORY_BASIC_INFORMATION, PAGE_EXECUTE, PAGE_GUARD, PAGE_NOACCESS, VirtualQuery,
};
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId};

use crate::log::log;

/// Stubs available (40 imports and the named vtable entries).
const STUBS: usize = 1024;
/// Stack words kept per call (the longest FMOD call the game makes takes 7).
const WORDS: usize = 8;

/// One patched function.
struct Entry {
    dll: &'static str,
    /// `import`, or `vtable@<slot address>`.
    site: String,
    name: String,
    original: usize,
}

static TABLE: OnceLock<Vec<Entry>> = OnceLock::new();
static ON: AtomicBool = AtomicBool::new(false);
static SEQ: AtomicU64 = AtomicU64::new(0);

struct Log {
    out: std::io::BufWriter<std::fs::File>,
    start: Instant,
    flushed: Instant,
    threads: Vec<u32>,
}

static LOG: Mutex<Option<Log>> = Mutex::new(None);

/// A call on its way: what [`after`] needs.
struct Pending {
    index: u32,
    ret: u32,
    /// Where the return address was (the caller's `esp` after its `call`).
    ret_slot: usize,
    args: [u32; WORDS],
    strings: [Option<String>; WORDS],
}

thread_local! {
    static PENDING: RefCell<Vec<Pending>> = const { RefCell::new(Vec::new()) };
}

/// `STUBS` stubs of 10 bytes (`push imm32; jmp rel32`), then the common
/// entry: `pushad`, [`enter`] with the saved registers (index at word 8,
/// return address 9, arguments from 10), the target it answers put over
/// the index, `popad`, `ret` into the target.
#[unsafe(naked)]
unsafe extern "C" fn stubs() {
    core::arch::naked_asm!(
        ".set ffx_fmodlog_i, 0",
        ".rept {n}",
        ".byte 0x68",
        ".long ffx_fmodlog_i",
        ".byte 0xe9",
        ".long 2f - . - 4",
        ".set ffx_fmodlog_i, ffx_fmodlog_i + 1",
        ".endr",
        "2:",
        "pushad",
        "push esp",
        "call {enter}",
        "add esp, 4",
        "mov [esp + 32], eax",
        "popad",
        "ret",
        n = const STUBS,
        enter = sym enter,
    )
}

/// Where a logged call returns: a slot for the real return address,
/// `pushad`, [`leave`], the address it answers put in the slot, `popad`,
/// `ret` to the caller with the callee's registers.
#[unsafe(naked)]
unsafe extern "C" fn after() {
    core::arch::naked_asm!(
        "push 0",
        "pushad",
        "push esp",
        "call {leave}",
        "add esp, 4",
        "mov [esp + 32], eax",
        "popad",
        "ret",
        leave = sym leave,
    )
}

fn in_game(address: u32) -> bool {
    let base = crate::GAME_BASE.load(Ordering::Relaxed);
    let a = address as usize;
    a >= base && a < base + crate::SIZE_OF_IMAGE as usize
}

fn game_address(address: u32) -> u32 {
    let base = crate::GAME_BASE.load(Ordering::Relaxed) as u32;
    address.wrapping_sub(base).wrapping_add(crate::IMAGE_BASE)
}

/// The readable, committed bytes from `address` to the end of its region
/// (0 when none).
pub(crate) fn readable(address: u32) -> usize {
    if address < 0x1_0000 {
        return 0;
    }
    // SAFETY: VirtualQuery only reads the address space's bookkeeping.
    unsafe {
        let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        if VirtualQuery(
            address as usize as *const c_void,
            &raw mut info,
            size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0
        {
            return 0;
        }
        if info.State != MEM_COMMIT
            || info.Protect & (PAGE_GUARD | PAGE_NOACCESS) != 0
            || info.Protect == PAGE_EXECUTE
        {
            return 0;
        }
        (info.BaseAddress as usize + info.RegionSize).saturating_sub(address as usize)
    }
}

/// The zero-terminated printable string at `address`, if it is one.
fn string_at(address: u32) -> Option<String> {
    let room = readable(address).min(160);
    if room < 2 {
        return None;
    }
    // SAFETY: `room` bytes from `address` are committed and readable.
    let bytes = unsafe { std::slice::from_raw_parts(address as usize as *const u8, room) };
    let end = bytes.iter().position(|&b| b == 0)?;
    let text = &bytes[..end];
    (end >= 2 && text.iter().all(|&b| (0x20..0x7f).contains(&b)))
        .then(|| String::from_utf8_lossy(text).into_owned())
}

/// The word at `address`, if it is readable.
fn word_at(address: u32) -> Option<u32> {
    // SAFETY: four readable bytes.
    (readable(address) >= 4).then(|| unsafe { (address as usize as *const u32).read_unaligned() })
}

/// Called by the stubs with the `pushad` frame; answers the target.
extern "C" fn enter(regs: *mut u32) -> usize {
    // SAFETY: the stub's frame: 8 saved registers, the index, the return
    // address and the caller's stack words.
    let (index, ret) = unsafe { (*regs.add(8), *regs.add(9)) };
    let Some(entry) = TABLE.get().and_then(|t| t.get(index as usize)) else {
        return 0;
    };
    if !ON.load(Ordering::Relaxed) || !in_game(ret) {
        return entry.original;
    }
    let mut args = [0; WORDS];
    for (i, a) in args.iter_mut().enumerate() {
        // SAFETY: the caller's stack words (at most a few past its
        // arguments, still its frame).
        *a = unsafe { *regs.add(10 + i) };
    }
    let strings = args.map(string_at);
    let pending = Pending {
        index,
        ret,
        // SAFETY: inside the frame.
        ret_slot: unsafe { regs.add(9) } as usize,
        args,
        strings,
    };
    let pushed = PENDING
        .try_with(|p| p.try_borrow_mut().map(|mut p| p.push(pending)).is_ok())
        .unwrap_or(false);
    if pushed {
        // SAFETY: the return slot of the frame.
        unsafe { *regs.add(9) = after as *const () as usize as u32 };
    }
    entry.original
}

/// Called by [`after`] with its `pushad` frame (word 3 the `esp` before it,
/// the return slot; word 7 the callee's `eax`); answers the caller's return
/// address.
extern "C" fn leave(regs: *mut u32) -> usize {
    // SAFETY: the frame of `after`.
    let (slot, result) = unsafe { (*regs.add(3) as usize, *regs.add(7)) };
    let Ok(Some(p)) = PENDING.try_with(|p| p.try_borrow_mut().ok().and_then(|mut p| p.pop()))
    else {
        // Cannot happen: every redirected return pushed its entry.
        return 0;
    };
    let words = slot.saturating_sub(p.ret_slot) / 4;
    write_call(&p, words, result);
    p.ret as usize
}

fn write_call(p: &Pending, words: usize, result: u32) {
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    // SAFETY: plain call.
    let tid = unsafe { GetCurrentThreadId() };
    let n = words.min(WORDS);
    let args: Vec<String> = p.args[..n].iter().map(|a| format!("{a:x}")).collect();
    let mut line = String::new();
    let Ok(mut log) = LOG.lock() else { return };
    let Some(log) = log.as_mut() else { return };
    if !log.threads.contains(&tid) {
        log.threads.push(tid);
        let _ = writeln!(log.out, "# thread {tid} {}", thread_start());
    }
    let ms = log.start.elapsed().as_secs_f64() * 1000.0;
    let _ = write!(
        line,
        "{seq} {ms:.3} {} {tid} {:08x} {} {words} {} {result:x}",
        crate::harness::frame(),
        game_address(p.ret),
        p.index,
        if args.is_empty() {
            "-".to_owned()
        } else {
            args.join(",")
        },
    );
    for (i, a) in p.args[..n].iter().enumerate() {
        if let Some(w) = word_at(*a) {
            let _ = write!(line, " o{i}={w:x}");
        }
    }
    for (i, s) in p.strings[..n].iter().enumerate() {
        if let Some(s) = s {
            let s = s.replace('\\', "\\\\").replace('"', "\\\"");
            let _ = write!(line, " s{i}=\"{s}\"");
        }
    }
    let _ = writeln!(log.out, "{line}");
    // `quit` ends the process without unwinding: keep the file current.
    if log.flushed.elapsed().as_millis() >= 1000 {
        let _ = log.out.flush();
        log.flushed = Instant::now();
    }
}

/// Writes `# <text>` into the log (when it is on), in order with the calls:
/// the audio capture's `EventSystem::update` wrapper notes each update and
/// the mixer's sample clock after it (`# update <n> <dsp clock>`), since an
/// update it makes is not a call from `FFX.exe` and so not logged.
pub fn note(text: &str) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let Ok(mut log) = LOG.lock() else { return };
    let Some(log) = log.as_mut() else { return };
    let _ = writeln!(log.out, "# {text}");
    // one flush per FMOD tick: `quit` then loses no update's calls
    let _ = log.out.flush();
    log.flushed = Instant::now();
}

/// This thread's start address, as an `FFX.exe` address or `module+offset`.
fn thread_start() -> String {
    type Query = unsafe extern "system" fn(isize, u32, *mut c_void, u32, *mut u32) -> i32;
    // SAFETY: plain Win32 calls with local buffers.
    unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        let Some(query) = GetProcAddress(ntdll, c"NtQueryInformationThread".as_ptr().cast()) else {
            return "?".into();
        };
        let query: Query = std::mem::transmute(query);
        let mut start: usize = 0;
        // ThreadQuerySetWin32StartAddress = 9
        query(
            GetCurrentThread() as isize,
            9,
            (&raw mut start).cast(),
            4,
            std::ptr::null_mut(),
        );
        if in_game(start as u32) {
            return format!("{:08x}", game_address(start as u32));
        }
        let mut module = std::ptr::null_mut();
        if GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            start as *const u8,
            &raw mut module,
        ) == 0
        {
            return format!("{start:#x}");
        }
        let mut name = [0u8; 260];
        let len = GetModuleFileNameA(module, name.as_mut_ptr(), 260) as usize;
        let path = String::from_utf8_lossy(&name[..len.min(260)]).into_owned();
        let file = path.rsplit('\\').next().unwrap_or(&path).to_owned();
        format!("{file}+{:#x}", start - module as usize)
    }
}

/// Reads a `u32` of a mapped image.
///
/// # Safety
/// `at` must be readable.
unsafe fn read(at: usize) -> usize {
    // SAFETY: forwarded.
    unsafe { (at as *const u32).read_unaligned() as usize }
}

/// The names `module` imports from `dll`, with their import table slots.
///
/// # Safety
/// `module` must be a mapped PE32 image.
unsafe fn imports(module: usize, dll: &str) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    // SAFETY: the headers and import directory of a mapped image.
    unsafe {
        let nt = module + read(module + 0x3c);
        let dir = read(nt + 0x80);
        if dir == 0 {
            return out;
        }
        let mut desc = module + dir;
        loop {
            let (names, name, thunks) = (read(desc), read(desc + 12), read(desc + 16));
            if name == 0 {
                break;
            }
            let dll_name = CStr::from_ptr((module + name) as *const _).to_string_lossy();
            if dll_name.eq_ignore_ascii_case(dll) {
                let names = if names == 0 { thunks } else { names };
                let mut i = 0;
                loop {
                    let entry = read(module + names + 4 * i);
                    if entry == 0 {
                        break;
                    }
                    if entry & 0x8000_0000 == 0 {
                        let f = CStr::from_ptr((module + entry + 2) as *const _);
                        out.push((f.to_string_lossy().into_owned(), module + thunks + 4 * i));
                    }
                    i += 1;
                }
            }
            desc += 20;
        }
    }
    out
}

/// `module`'s named exports of FMOD's implementation classes
/// (`?method@<Class>I@FMOD@@...`): address -> name.
///
/// # Safety
/// `module` must be a mapped PE32 image.
unsafe fn class_exports(module: usize) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    // SAFETY: the export directory of a mapped image.
    unsafe {
        let nt = module + read(module + 0x3c);
        let dir = read(nt + 0x78);
        if dir == 0 {
            return out;
        }
        let e = module + dir;
        let (functions, names, ordinals) = (read(e + 0x1c), read(e + 0x20), read(e + 0x24));
        for i in 0..read(e + 0x18) {
            let name = CStr::from_ptr((module + read(module + names + 4 * i)) as *const _)
                .to_string_lossy()
                .into_owned();
            let class = name
                .strip_prefix('?')
                .and_then(|n| n.split('@').nth(1))
                .unwrap_or("");
            if !name.contains("@FMOD@@") || !class.ends_with('I') {
                continue;
            }
            let ordinal = (module + ordinals + 2 * i) as *const u16;
            let rva = read(module + functions + 4 * usize::from(ordinal.read_unaligned()));
            out.push((module + rva, name));
        }
    }
    out.sort();
    out
}

/// Every pointer-sized slot of `module`'s data sections that holds one of
/// `exports`, in runs of at least three code pointers (vtables): slot ->
/// export name.
///
/// # Safety
/// `module` must be a mapped PE32 image.
unsafe fn vtable_slots(module: usize, exports: &[(usize, String)]) -> Vec<(usize, String)> {
    const EXECUTE: usize = 0x2000_0000;
    const INITIALIZED: usize = 0x40;
    let mut out = Vec::new();
    // SAFETY: the section table of a mapped image.
    unsafe {
        let nt = module + read(module + 0x3c);
        let sections = usize::from(((nt + 6) as *const u16).read_unaligned());
        let optional = usize::from(((nt + 0x14) as *const u16).read_unaligned());
        let table = nt + 0x18 + optional;
        let mut code = Vec::new();
        let mut data = Vec::new();
        for s in 0..sections {
            let h = table + 40 * s;
            let (size, rva, flags) = (read(h + 8), read(h + 12), read(h + 36));
            if flags & EXECUTE != 0 {
                code.push((module + rva, module + rva + size));
            } else if flags & INITIALIZED != 0 {
                data.push((module + rva, module + rva + size));
            }
        }
        let is_code = |a: usize| code.iter().any(|&(s, e)| a >= s && a < e);
        let name_of = |a: usize| {
            exports
                .binary_search_by_key(&a, |(addr, _)| *addr)
                .ok()
                .map(|i| exports[i].1.clone())
        };
        for (start, end) in data {
            let mut at = start;
            while at + 4 <= end {
                // a run of code pointers
                let mut run = at;
                while run + 4 <= end && is_code(read(run)) {
                    run += 4;
                }
                if run - at >= 12 {
                    let mut slot = at;
                    while slot < run {
                        if let Some(name) = name_of(read(slot)) {
                            out.push((slot, name));
                        }
                        slot += 4;
                    }
                }
                at = run.max(at + 4);
            }
        }
    }
    out
}

/// Installs the stubs when `FFX_FMOD_LOG` is set.
///
/// # Safety
/// `base` must be the main module; called before other threads run game
/// code (the loader has bound the imports already).
pub unsafe fn install(base: usize) {
    let Ok(path) = std::env::var("FFX_FMOD_LOG") else {
        return;
    };
    let file = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            log(&format!("fmod log: cannot create {path}: {e}"));
            return;
        }
    };
    let mut entries = Vec::new();
    let mut patches: Vec<(usize, bool)> = Vec::new();
    for dll in ["fmod_event.dll", "fmodex.dll"] {
        // SAFETY: forwarded from the caller.
        for (name, slot) in unsafe { imports(base, dll) } {
            entries.push(Entry {
                dll,
                site: "import".into(),
                name,
                // SAFETY: a bound import slot.
                original: unsafe { read(slot) },
            });
            patches.push((slot, true));
        }
    }
    // SAFETY: plain call.
    let event = unsafe { GetModuleHandleA(c"fmod_event.dll".as_ptr().cast()) } as usize;
    if event != 0 {
        // SAFETY: a loaded DLL is a mapped image.
        let exports = unsafe { class_exports(event) };
        // SAFETY: as above.
        for (slot, name) in unsafe { vtable_slots(event, &exports) } {
            entries.push(Entry {
                dll: "fmod_event.dll",
                site: format!("vtable@{:x}+{:x}", event, slot - event),
                name,
                // SAFETY: a slot inside the image.
                original: unsafe { read(slot) },
            });
            patches.push((slot, false));
        }
    }
    entries.truncate(STUBS);
    patches.truncate(STUBS);
    let mut header = String::from("# fmodlog 1\n");
    for (i, e) in entries.iter().enumerate() {
        let _ = writeln!(
            header,
            "# fn {i} {} {} {:x} {}",
            e.dll, e.site, e.original, e.name
        );
    }
    let mut out = std::io::BufWriter::with_capacity(1 << 20, file);
    let _ = out.write_all(header.as_bytes());
    let _ = out.flush();
    let count = entries.len();
    if TABLE.set(entries).is_err() {
        return;
    }
    if let Ok(mut l) = LOG.lock() {
        *l = Some(Log {
            out,
            start: Instant::now(),
            flushed: Instant::now(),
            threads: Vec::new(),
        });
    }
    ON.store(true, Ordering::Release);
    let stubs = stubs as *const () as usize;
    let mut done = 0;
    for (i, (slot, _import)) in patches.iter().enumerate() {
        // SAFETY: an import slot or a vtable slot found above; no other
        // thread runs yet.
        if unsafe { crate::iat::patch_vtable(*slot as *mut usize, 0, stubs + 10 * i) }.is_some() {
            done += 1;
        }
    }
    log(&format!(
        "  fmod log: {done} of {count} FMOD entries logged to {path}"
    ));
}
