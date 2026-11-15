//! Observers of the game between two updates (docs/host.md, "The
//! game-state API"): at each frame boundary (`FFXApplication_animate`'s
//! entry, `crate::clock`) the main thread reads an [`ffx_state::Snapshot`]
//! and hands it to every observer, a frontend's (gd-ffx's Godot side) or a
//! test's. Nothing is read while no observer is registered.
//!
//! Observers only get the snapshot: they cannot write the game's memory or
//! call into the game, so the game runs the same with or without them
//! (replays, saves). They run on the game's main thread: they must return
//! quickly (hand the snapshot on, don't wait).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use ffx_bridge::Mem as _;

use crate::exec::{Exec, Shared};

/// An observer.
pub type Observer = Box<dyn Fn(&Arc<ffx_state::Snapshot>) + Send + Sync>;

static OBSERVERS: RwLock<Vec<Observer>> = RwLock::new(Vec::new());
static ANY: AtomicBool = AtomicBool::new(false);

/// Registers `f`: from the next boundary on it gets every snapshot (one
/// game per process: the observers are the process's).
pub fn observe(f: Observer) {
    if let Ok(mut o) = OBSERVERS.write() {
        o.push(f);
        ANY.store(true, Ordering::Release);
    }
}

/// The snapshot now, read from `shared`'s arena: `update` is the updates
/// run so far, `game_time` the clock's seconds since its start.
#[must_use]
pub fn snapshot(shared: &Shared) -> ffx_state::Snapshot {
    let arena = shared.arena();
    let mem = crate::arena::ArenaMem {
        arena: Arc::clone(arena),
    };
    let reader = |at: u32, out: &mut [u8]| {
        if !arena.is_mapped(at, out.len()) {
            return false;
        }
        mem.read_bytes(at, out);
        true
    };
    ffx_state::read(
        &reader,
        shared.clock.frame(),
        shared.clock.now() - crate::clock::VIRTUAL_START,
    )
}

/// The boundary before update `frame` (the main thread, `crate::clock`).
pub(crate) fn boundary(e: &Exec, _frame: u32) {
    if !ANY.load(Ordering::Acquire) {
        return;
    }
    let s = Arc::new(snapshot(e.shared()));
    if let Ok(o) = OBSERVERS.read() {
        for f in o.iter() {
            f(&s);
        }
    }
}
