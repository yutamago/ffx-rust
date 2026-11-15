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

/// A request's reply: what it did, or why not. Called on the game's main
/// thread; must not panic.
pub type Reply = Box<dyn FnOnce(Result<String, String>) + Send>;

/// A game's waiting requests.
#[derive(Default)]
pub struct Requests {
    queue: Mutex<Queue>,
    waiting: AtomicBool,
}

#[derive(Default)]
struct Queue {
    items: VecDeque<(Request, Reply)>,
    /// The game's run ended ([`Requests::close`]): nothing is served any
    /// more.
    closed: bool,
}

impl std::fmt::Debug for Requests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Requests")
            .field("waiting", &self.waiting.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// What a request gets once the run has ended.
const ENDED: &str = "the run ended";

impl Requests {
    /// Queues `request`; `reply` gets the outcome on the game's main thread
    /// at the next boundary (keep it short). After the run ended it is
    /// answered `Err` at once, on the caller's thread.
    pub fn push(&self, request: Request, reply: Reply) {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        if q.closed {
            drop(q);
            reply(Err(ENDED.into()));
            return;
        }
        q.items.push_back((request, reply));
        self.waiting.store(true, Ordering::Release);
    }

    fn take(&self) -> Vec<(Request, Reply)> {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        self.waiting.store(false, Ordering::Release);
        q.items.drain(..).collect()
    }

    /// The run ended (`Exec::run_entry` returns): what waits is answered
    /// `Err`, and so is every later request.
    pub fn close(&self) {
        let left = {
            let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            q.closed = true;
            self.waiting.store(false, Ordering::Release);
            q.items.drain(..).collect::<Vec<_>>()
        };
        for (_, reply) in left {
            reply(Err(ENDED.into()));
        }
    }
}

/// The boundary before update `frame` (the main thread, `crate::clock`).
pub(crate) fn boundary(e: &mut Exec, frame: u32) {
    if !e.shared().requests.waiting.load(Ordering::Acquire) {
        return;
    }
    for (request, reply) in e.shared().requests.take() {
        // a request that stopped the game: the rest are not run on it
        if e.stopped().is_some() {
            reply(Err(ENDED.into()));
            continue;
        }
        let r = match &request {
            Request::DevSave(n) => crate::devsave::load(e, *n),
        };
        crate::log::note(&format!("requests: {request:?} at frame {frame}: {r:?}"));
        reply(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    type Got = Arc<Mutex<Vec<Result<String, String>>>>;

    fn recorder() -> (Got, impl Fn() -> Reply) {
        let got: Got = Arc::default();
        let g = Arc::clone(&got);
        (got, move || {
            let g = Arc::clone(&g);
            Box::new(move |r| g.lock().unwrap().push(r)) as Reply
        })
    }

    #[test]
    fn the_end_of_the_run_answers_what_waits_and_what_comes() {
        let q = Requests::default();
        let (got, reply) = recorder();
        q.push(Request::DevSave(7), reply());
        assert!(q.waiting.load(Ordering::Acquire));
        assert_eq!(got.lock().unwrap().len(), 0);
        // a reply that asks again while the queue closes is answered too
        let q = Arc::new(q);
        let again = Arc::clone(&q);
        let (got2, reply2) = recorder();
        let r2 = reply2();
        q.push(
            Request::DevSave(9),
            Box::new(move |_| again.push(Request::DevSave(10), r2)),
        );
        q.close();
        assert_eq!(*got2.lock().unwrap(), vec![Err(ENDED.to_string())]);
        assert!(!q.waiting.load(Ordering::Acquire));
        q.push(Request::DevSave(8), reply());
        let got = got.lock().unwrap();
        assert_eq!(*got, vec![Err(ENDED.to_string()), Err(ENDED.to_string())]);
    }
}
