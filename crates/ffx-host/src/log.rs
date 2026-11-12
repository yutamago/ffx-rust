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

/// The count as one line, `bad_accesses: N` (with the first ones when
/// there are any): the report's and the harness's `quit`'s, which
/// `tools/replays/run-all.sh` reads from `ffx-rust.log` (a run with any
/// fails: M5 stage 4's check, docs/harness.md).
#[must_use]
pub fn bad_accesses_line() -> String {
    let (n, first) = bad_accesses();
    if n == 0 {
        return "bad_accesses: 0".into();
    }
    let first: Vec<String> = first
        .iter()
        .map(|&(addr, len, write)| {
            format!(
                "{} {len} at {addr:#010x}",
                if write { "write" } else { "read" }
            )
        })
        .collect();
    format!(
        "bad_accesses: {n} (outside the arena's regions; first: {})",
        first.join(", ")
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bad_access_line_counts_and_names_the_first() {
        // the counter is process-wide: other tests may have added to it
        let before = bad_accesses().0;
        bad_access(0x7fff_0000, 4, true);
        let line = bad_accesses_line();
        let n: u64 = line
            .strip_prefix("bad_accesses: ")
            .and_then(|r| r.split_whitespace().next())
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        assert_eq!(n, before + 1, "{line}");
        if before < 16 {
            assert!(line.contains("write 4 at 0x7fff0000"), "{line}");
        }
    }
}
