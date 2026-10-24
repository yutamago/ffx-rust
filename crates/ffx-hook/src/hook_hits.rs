//! Hook-hit mode: which installed hooks the game actually runs.
//!
//! With `FFX_HOOK_HITS=<file>` (a Windows path; `tools/run-game.sh` passes
//! it through, `/p` in `WSLENV`), every installed hook's patch site (the
//! `jmp rel32` [`crate::install`] wrote over the original's first byte) gets
//! a one-time `int3` in place of its `0xe9`. The first call through it
//! traps into the vectored handler in `log`, which puts the `0xe9` back,
//! appends `0x<address> <name>` to `<file>` and resumes at the `jmp`: the
//! shim then runs as usual.
//!
//! Cost: with the variable unset nothing is armed, so the shims, thunks and
//! patch sites are exactly the ones of a normal run (no flag, no branch).
//! With it set, each hook costs one exception and one append the first time
//! it runs, and nothing after that. Hits go straight to the file (one
//! `WriteFile` in append mode per hit, no user-space buffer), so a game that
//! crashes still leaves every hit it had. A call that reaches a port's Rust
//! code without going through its patch site (one module crate calling
//! another's core directly) is not counted: the mode answers which hooks the
//! game enters.

use std::fs::File;
use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// The armed sites: process address (sorted), `FFX.exe` address, name.
struct Armed {
    sites: Vec<(usize, u32, &'static str)>,
    fired: Vec<AtomicBool>,
    file: File,
    count: AtomicU32,
}

static ARMED: OnceLock<Armed> = OnceLock::new();

/// Arms every hook of [`crate::hooks`] whose site still holds the `jmp` to
/// its replacement (a disabled or failed hook is left alone). Call after
/// the hooks are installed.
///
/// # Safety
/// `base` must be the main module, with the hook table installed; called
/// before other threads run game code.
pub unsafe fn install(base: usize) {
    let Ok(path) = std::env::var("FFX_HOOK_HITS") else {
        return;
    };
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            crate::log::log(&format!("hook hits: cannot open {path}: {e}"));
            return;
        }
    };
    let mut sites = Vec::new();
    let mut skipped = 0;
    for hook in crate::hooks() {
        let site = base + (hook.address - crate::IMAGE_BASE) as usize;
        // SAFETY: inside the main module's image, which is mapped.
        let jmp = unsafe { std::ptr::read_unaligned(site as *const [u8; 5]) };
        let target =
            (site + 5).wrapping_add(u32::from_le_bytes([jmp[1], jmp[2], jmp[3], jmp[4]]) as usize);
        if jmp[0] != 0xe9 || target != hook.replacement {
            skipped += 1; // not installed (FFX_HOOK_DISABLE) or patched over
            continue;
        }
        let mut old = 0;
        // SAFETY: one byte of the mapped image; it stays writable so the
        // handler can put the byte back without another protection change.
        if unsafe { VirtualProtect(site as *const _, 1, PAGE_EXECUTE_READWRITE, &raw mut old) } == 0
        {
            skipped += 1;
            continue;
        }
        // SAFETY: just made writable; no other thread runs game code yet.
        unsafe { std::ptr::write_volatile(site as *mut u8, 0xcc) };
        sites.push((site, hook.address, hook.name));
    }
    sites.sort_unstable_by_key(|s| s.0);
    sites.dedup_by_key(|s| s.0);
    // SAFETY: flushing the current process's own code.
    unsafe { FlushInstructionCache(GetCurrentProcess(), std::ptr::null(), 0) };
    crate::log::log(&format!(
        "hook hits: {} hooks armed, {skipped} skipped (not installed); hits to {path}",
        sites.len()
    ));
    let fired = (0..sites.len()).map(|_| AtomicBool::new(false)).collect();
    let _ = ARMED.set(Armed {
        sites,
        fired,
        file,
        count: AtomicU32::new(0),
    });
}

/// A breakpoint at `at`: when it is an armed hook site, puts the `jmp` back
/// (the first time also records the hook) and returns true, the caller
/// resuming at `at`. Another thread that hit the same `int3` before the
/// byte came back resumes the same way.
pub fn hit(at: usize) -> bool {
    let Some(armed) = ARMED.get() else {
        return false;
    };
    let Ok(i) = armed.sites.binary_search_by_key(&at, |s| s.0) else {
        return false;
    };
    let (site, address, name) = armed.sites[i];
    // SAFETY: an armed site, made writable at install; 0xe9 is what was there.
    unsafe {
        std::ptr::write_volatile(site as *mut u8, 0xe9);
        FlushInstructionCache(GetCurrentProcess(), site as *const _, 1);
    }
    if !armed.fired[i].swap(true, Ordering::AcqRel) {
        armed.count.fetch_add(1, Ordering::Relaxed);
        // One write per line (append mode: lines from several threads
        // never interleave), formatted on this stack: no allocation.
        let mut line = [0u8; 192];
        let mut cur = std::io::Cursor::new(&mut line[..]);
        let n = if writeln!(cur, "0x{address:08x} {name}").is_ok() {
            cur.position()
        } else {
            cur.set_position(0);
            let _ = writeln!(cur, "0x{address:08x}");
            cur.position()
        };
        let n = usize::try_from(n).unwrap_or(0).min(line.len());
        let _ = (&armed.file).write_all(&line[..n]);
    }
    true
}

/// How many hooks have run so far.
pub fn fired() -> u32 {
    ARMED.get().map_or(0, |a| a.count.load(Ordering::Relaxed))
}
