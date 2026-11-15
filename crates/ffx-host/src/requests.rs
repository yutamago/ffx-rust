//! A frontend's requests to its game (docs/state-api.md, "Requests"):
//! queued from any thread (`Shared::requests`), served by the game's main
//! thread at the next frame boundary (`FFXApplication_animate`'s entry,
//! `crate::clock`), between two updates, where the harness serves its own
//! jobs. Each gets its reply through a callback.
//!
//! A run with no requests is unchanged: the queue is only looked at when
//! one is waiting.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::exec::Exec;

/// What a frontend can ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Load developer save `n` (1..=110, the developers' own saves in the
    /// player's archive: `crate::devsave`) and start its map.
    DevSave(i32),
}

/// A request's reply: what it did, or why not.
pub type Reply = Box<dyn FnOnce(Result<String, String>) + Send>;

/// A game's waiting requests.
#[derive(Default)]
pub struct Requests {
    queue: Mutex<VecDeque<(Request, Reply)>>,
    waiting: AtomicBool,
}

impl std::fmt::Debug for Requests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Requests")
            .field("waiting", &self.waiting.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Requests {
    /// Queues `request`; `reply` gets the outcome on the game's main thread
    /// at the next boundary (keep it short).
    pub fn push(&self, request: Request, reply: Reply) {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        q.push_back((request, reply));
        self.waiting.store(true, Ordering::Release);
    }

    fn take(&self) -> Vec<(Request, Reply)> {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        self.waiting.store(false, Ordering::Release);
        q.drain(..).collect()
    }
}

/// The boundary before update `frame` (the main thread, `crate::clock`).
pub(crate) fn boundary(e: &mut Exec, frame: u32) {
    if !e.shared().requests.waiting.load(Ordering::Acquire) {
        return;
    }
    for (request, reply) in e.shared().requests.take() {
        let r = match &request {
            Request::DevSave(n) => crate::devsave::load(e, *n),
        };
        crate::log::note(&format!("requests: {request:?} at frame {frame}: {r:?}"));
        reply(r);
    }
}
