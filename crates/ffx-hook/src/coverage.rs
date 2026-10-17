//! Coverage mode: which candidate functions run at all.
//!
//! With `FFX_COVERAGE=<file>` (a Windows path; `tools/run-game.sh` passes
//! it through), `<file>` lists candidates, one `<address> <first byte>` per
//! line in hex (`tools/coverage.py candidates` writes it from
//! `symbols/classes.tsv`). Each candidate whose live first byte is still the
//! expected one (so no hook or patch owns it) gets a one-time breakpoint
//! (`int3`). The first time one fires, the vectored handler in `log`
//! restores the byte, appends the address to `<file>.hits` and resumes the
//! function as if nothing happened. Nothing else changes: the game runs
//! its own code, at full speed once every candidate it reaches has fired.

use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// The armed candidates: process address (sorted), original byte, fired.
struct Armed {
    sites: Vec<(usize, u8)>,
    fired: Vec<AtomicBool>,
    hits: std::path::PathBuf,
    count: AtomicU32,
    base: usize,
}

static ARMED: OnceLock<Armed> = OnceLock::new();

/// Reads the candidate list and arms every candidate that still holds its
/// expected first byte. Call after every other patch is in place.
///
/// # Safety
/// `base` must be the main module; called before other threads run game code.
pub unsafe fn install(base: usize) {
    let Ok(path) = std::env::var("FFX_COVERAGE") else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        crate::log::log(&format!("coverage: cannot read {path}"));
        return;
    };
    let mut sites = Vec::new();
    let (mut skipped, mut bad) = (0, 0);
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let (Some(a), Some(b)) = (words.next(), words.next()) else {
            continue;
        };
        let (Ok(a), Ok(b)) = (
            u32::from_str_radix(a.trim_start_matches("0x"), 16),
            u8::from_str_radix(b.trim_start_matches("0x"), 16),
        ) else {
            bad += 1;
            continue;
        };
        if !(crate::IMAGE_BASE..crate::IMAGE_BASE + crate::SIZE_OF_IMAGE).contains(&a) {
            bad += 1;
            continue;
        }
        let at = base + (a - crate::IMAGE_BASE) as usize;
        // SAFETY: inside the main module's image, which is mapped.
        let live = unsafe { std::ptr::read_volatile(at as *const u8) };
        if live != b {
            skipped += 1; // a hook or patch owns this byte
            continue;
        }
        let mut old = 0;
        // SAFETY: one byte of the mapped image; it stays writable so the
        // handler can put the byte back without another protection change.
        if unsafe { VirtualProtect(at as *const _, 1, PAGE_EXECUTE_READWRITE, &raw mut old) } == 0 {
            skipped += 1;
            continue;
        }
        // SAFETY: just made writable; no other thread runs game code yet.
        unsafe { std::ptr::write_volatile(at as *mut u8, 0xcc) };
        sites.push((at, b));
    }
    sites.sort_unstable();
    // SAFETY: flushing the current process's own code.
    unsafe { FlushInstructionCache(GetCurrentProcess(), std::ptr::null(), 0) };
    crate::log::log(&format!(
        "coverage: {} candidates armed, {skipped} skipped (patched), {bad} bad lines; hits to {path}.hits",
        sites.len()
    ));
    let fired = (0..sites.len()).map(|_| AtomicBool::new(false)).collect();
    let _ = ARMED.set(Armed {
        sites,
        fired,
        hits: format!("{path}.hits").into(),
        count: AtomicU32::new(0),
        base,
    });
}

/// A breakpoint at `at`: when it is an armed candidate, restores its byte
/// (the first time also records it) and returns true, the caller resuming
/// at `at`. Another thread that hit the same `int3` before the byte came
/// back resumes the same way.
pub fn hit(at: usize) -> bool {
    let Some(armed) = ARMED.get() else {
        return false;
    };
    let Ok(i) = armed.sites.binary_search_by_key(&at, |s| s.0) else {
        return false;
    };
    let (site, byte) = armed.sites[i];
    // SAFETY: an armed site, made writable at install.
    unsafe {
        std::ptr::write_volatile(site as *mut u8, byte);
        FlushInstructionCache(GetCurrentProcess(), site as *const _, 1);
    }
    if !armed.fired[i].swap(true, Ordering::AcqRel) {
        armed.count.fetch_add(1, Ordering::Relaxed);
        let image = (site - armed.base) as u32 + crate::IMAGE_BASE;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&armed.hits)
        {
            let _ = writeln!(f, "0x{image:08x}");
        }
    }
    true
}

/// How many candidates fired so far.
pub fn fired() -> u32 {
    ARMED.get().map_or(0, |a| a.count.load(Ordering::Relaxed))
}
