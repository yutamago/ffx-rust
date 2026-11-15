//! Observers of a game between two updates (docs/state-api.md): at each
//! frame boundary (`FFXApplication_animate`'s entry, `crate::clock`) the
//! game's main thread reads an [`ffx_state::Snapshot`] and hands it to the
//! game's observers ([`Observers`], `Shared::observers`): a frontend's
//! (gd-ffx's Godot side) or a test's. Nothing is read while a game has no
//! observer.
//!
//! Observers only get the snapshot: they cannot write the game's memory or
//! call into the game, so the game runs the same with or without them
//! (replays, saves). They run on the game's main thread, outside any lock
//! of this module: they must return quickly (hand the snapshot on, don't
//! wait); one may add or remove observers.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

use crate::exec::{Exec, Shared};

/// An observer.
pub type Observer = Arc<dyn Fn(&Arc<ffx_state::Snapshot>) + Send + Sync>;

/// An observer's registration, to remove it ([`Observers::remove`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObserverId(u64);

/// A game's observers.
#[derive(Default)]
pub struct Observers {
    list: RwLock<Vec<(ObserverId, Observer)>>,
    any: AtomicBool,
    next: AtomicU64,
}

impl std::fmt::Debug for Observers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.list.read().map_or(0, |l| l.len());
        f.debug_struct("Observers")
            .field("count", &n)
            .finish_non_exhaustive()
    }
}

impl Observers {
    /// Registers `f`: from the next boundary on it gets every snapshot of
    /// this game, until [`Observers::remove`].
    pub fn add(&self, f: Observer) -> ObserverId {
        let id = ObserverId(self.next.fetch_add(1, Ordering::Relaxed));
        let mut l = self.list.write().unwrap_or_else(PoisonError::into_inner);
        l.push((id, f));
        self.any.store(true, Ordering::Release);
        id
    }

    /// Removes the observer `id` (nothing when it is gone already).
    pub fn remove(&self, id: ObserverId) {
        let mut l = self.list.write().unwrap_or_else(PoisonError::into_inner);
        l.retain(|(i, _)| *i != id);
        self.any.store(!l.is_empty(), Ordering::Release);
    }

    /// Whether any observer is registered.
    #[must_use]
    pub fn any(&self) -> bool {
        self.any.load(Ordering::Acquire)
    }

    /// The observers now (called outside the lock).
    fn current(&self) -> Vec<Observer> {
        self.list
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(_, f)| Arc::clone(f))
            .collect()
    }
}

/// The snapshot now, read from `shared`'s arena: `update` is the updates
/// run so far, `game_time` the clock's seconds since its start.
#[must_use]
pub fn snapshot(shared: &Shared) -> ffx_state::Snapshot {
    let arena = shared.arena();
    let reader = |at: u32, out: &mut [u8]| {
        if !arena.is_mapped(at, out.len()) {
            return false;
        }
        arena.read_bytes(at, out);
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
    let observers = &e.shared().observers;
    if !observers.any() {
        return;
    }
    let s = Arc::new(snapshot(e.shared()));
    for f in observers.current() {
        f(&s);
    }
}
