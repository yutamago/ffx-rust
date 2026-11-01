//! The run's log: accesses outside the arena's regions, counted and the
//! first few kept (a port that reads past a mapping gets zeros, as
//! [`crate::arena::ArenaMem`] promises, and the report names it).

use std::sync::Mutex;

/// A bad access: address, length, whether a write.
pub type Access = (u32, usize, bool);

/// The count and the first bad accesses.
static BAD: Mutex<(u64, Vec<Access>)> = Mutex::new((0, Vec::new()));

/// Records an access outside the mapped regions.
pub fn bad_access(addr: u32, len: usize, write: bool) {
    let first = if let Ok(mut b) = BAD.lock() {
        b.0 += 1;
        let first = b.1.len() < 16;
        if first {
            b.1.push((addr, len, write));
        }
        first
    } else {
        false
    };
    // the first ones as notes too (the harness's `notes`), outside the lock
    if first {
        note(&format!(
            "bad access: {} of {len} bytes at {addr:#010x} (outside the arena's regions)",
            if write { "write" } else { "read" }
        ));
    }
}

/// How many bad accesses, and the first ones.
#[must_use]
pub fn bad_accesses() -> (u64, Vec<Access>) {
    BAD.lock().map(|b| b.clone()).unwrap_or_default()
}

/// Notes of the run (the clock's turns, the libraries' events), for the
/// report: the first [`NOTES`] kept.
static NOTE: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// How many notes are kept.
const NOTES: usize = 400;

/// Adds a note (also to stderr with `FFX_HOST_NOTES=1`).
pub fn note(line: &str) {
    if std::env::var_os("FFX_HOST_NOTES").is_some() {
        eprintln!("{line}");
    }
    if let Ok(mut n) = NOTE.lock()
        && n.len() < NOTES
    {
        n.push(line.to_owned());
    }
}

/// The hook's log line, as a note: what `crates/ffx-hook` code compiled
/// into this crate (`audio::device`, the hook's `rust_audio_out.rs`) logs.
pub fn log(line: &str) {
    note(line.trim_start());
}

/// The notes so far.
#[must_use]
pub fn notes() -> Vec<String> {
    NOTE.lock().map(|n| n.clone()).unwrap_or_default()
}
