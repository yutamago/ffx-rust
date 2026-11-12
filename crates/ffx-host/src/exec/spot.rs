//! Where each game thread is, for the harness's `stacks` command
//! (docs/harness.md, "Two processes"): the host entries it is inside
//! (ports, imports, traps, innermost last) with the `esp` each was entered
//! at. Another thread's Unicorn registers can't be read while it runs, but a
//! thread that waits always waits inside a host entry (an import, a turn of
//! the clock), so the innermost entry's `esp` is where its emulated stack
//! is; the return addresses are found from there.
//!
//! Written by the owning thread on every entry and leave (relaxed stores,
//! no lock); read by the harness thread at any time. A read can see an
//! entry being replaced: a sampling aid, as the hook's `stacks` is.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// Host entries kept per thread (deeper ones are counted, not kept).
pub const DEPTH: usize = 16;

/// What a host entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A port or wrapper of the function table (its address).
    Port,
    /// An import (its index in the bound list).
    Import,
    /// A trap placed at run time (its address).
    Trap,
}

/// One host entry as read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    /// What it is.
    pub kind: Kind,
    /// Its address or import index.
    pub what: u32,
    /// `esp` at the entry (at the caller's return address).
    pub esp: u32,
}

/// One thread's host entries.
#[derive(Debug)]
pub struct Spot {
    /// The game thread's id.
    pub id: u32,
    /// Its stack region (`start`, `end`).
    pub stack: (u32, u32),
    depth: AtomicU32,
    /// Each entry's kind and what (`kind << 32 | what`), and its `esp`.
    frames: [(AtomicU64, AtomicU32); DEPTH],
    /// `esp` at the last entry, kept after it is left (where a thread that
    /// runs emulated code was last seen).
    last_esp: AtomicU32,
    /// Inside `emu_start` (after the instance's set-up): only then may
    /// another thread stop its emulator (`TerminateThread`, the watchdog).
    pub emulating: AtomicBool,
}

impl Spot {
    /// A thread's spot.
    #[must_use]
    pub fn new(id: u32, stack: (u32, u32)) -> Self {
        Self {
            id,
            stack,
            depth: AtomicU32::new(0),
            frames: std::array::from_fn(|_| (AtomicU64::new(0), AtomicU32::new(0))),
            last_esp: AtomicU32::new(0),
            emulating: AtomicBool::new(false),
        }
    }

    /// The owning thread enters a host entry.
    pub fn enter(&self, kind: Kind, what: u32, esp: u32) {
        let d = self.depth.load(Ordering::Relaxed) as usize;
        if let Some(slot) = self.frames.get(d) {
            let k: u64 = match kind {
                Kind::Port => 1,
                Kind::Import => 2,
                Kind::Trap => 3,
            };
            slot.0.store((k << 32) | u64::from(what), Ordering::Relaxed);
            slot.1.store(esp, Ordering::Relaxed);
        }
        self.last_esp.store(esp, Ordering::Relaxed);
        self.depth
            .store(u32::try_from(d + 1).unwrap_or(u32::MAX), Ordering::Release);
    }

    /// The owning thread leaves its innermost host entry.
    pub fn leave(&self) {
        let d = self.depth.load(Ordering::Relaxed);
        self.depth.store(d.saturating_sub(1), Ordering::Release);
    }

    /// The entries now, outermost first, and how many more were too deep
    /// to keep.
    #[must_use]
    pub fn frames(&self) -> (Vec<Frame>, usize) {
        let d = self.depth.load(Ordering::Acquire) as usize;
        let kept = d.min(DEPTH);
        let frames = self.frames[..kept]
            .iter()
            .map(|(w, esp)| {
                let p = w.load(Ordering::Relaxed);
                let kind = match p >> 32 {
                    1 => Kind::Port,
                    2 => Kind::Import,
                    _ => Kind::Trap,
                };
                #[allow(clippy::cast_possible_truncation)] // the low half
                Frame {
                    kind,
                    what: p as u32,
                    esp: esp.load(Ordering::Relaxed),
                }
            })
            .collect();
        (frames, d - kept)
    }

    /// `esp` at the last entry.
    #[must_use]
    pub fn last_esp(&self) -> u32 {
        self.last_esp.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_nest_and_unpack() {
        let s = Spot::new(0x1004, (0x0300_0000, 0x0320_0000));
        s.enter(Kind::Port, 0x0042_f520, 0x031f_ff00);
        s.enter(Kind::Import, 17, 0x031f_fe00);
        let (f, more) = s.frames();
        assert_eq!(more, 0);
        assert_eq!(
            f,
            vec![
                Frame {
                    kind: Kind::Port,
                    what: 0x0042_f520,
                    esp: 0x031f_ff00
                },
                Frame {
                    kind: Kind::Import,
                    what: 17,
                    esp: 0x031f_fe00
                },
            ]
        );
        s.leave();
        assert_eq!(s.frames().0.len(), 1);
        assert_eq!(s.last_esp(), 0x031f_fe00);
        s.leave();
        s.leave();
        assert_eq!(s.frames().0, Vec::new());
    }

    #[test]
    fn too_deep_entries_are_counted() {
        let s = Spot::new(1, (0, 0x1000));
        for i in 0..u32::try_from(DEPTH).unwrap() + 3 {
            s.enter(Kind::Trap, i, 0x800 - i);
        }
        let (f, more) = s.frames();
        assert_eq!((f.len(), more), (DEPTH, 3));
        for _ in 0..3 {
            s.leave();
        }
        assert_eq!(s.frames(), (f, 0));
    }
}
