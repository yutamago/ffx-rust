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

use ffx_bridge::Mem as _;

use crate::exec::Exec;

/// What a frontend can ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Load developer save `n` (1..=110, the developers' own saves in the
    /// player's archive: `crate::devsave`) and start its map.
    DevSave(i32),
    /// Start a battle as a random encounter does (`MsBattleEncountExe`'s
    /// request, as the harness's `start_battle`): the battle field's record
    /// (`field`, e.g. 3 `cdsp00`), its group and formation. Answered once
    /// the battle runs (`g_BtlStatus` nonzero), or after 600 updates
    /// without it; the request is raised again every 30 updates while the
    /// field drops it (`MsBattleExeClear` while a map loads).
    Battle {
        /// The battle field's record (`g_BtlFieldIndex`).
        field: u16,
        /// The group in the field.
        group: u8,
        /// The formation in the group.
        formation: u8,
    },
}

/// `g_BtlFieldIndex` (`u16`), then the group and the formation.
const BTL_FIELD_INDEX: u32 = 0x0112_c256;
/// `ResetEncountExe(full)`.
const RESET_ENCOUNT_EXE: u32 = 0x0078_0ff0;
/// `g_BtlExe`: the battle request flag.
const BTL_EXE: u32 = 0x0112_a8e2;
/// `g_BtlStatus`: nonzero in battle.
const BTL_STATUS: u32 = 0x0112_a8e0;
/// Updates a battle request waits for its battle.
const BATTLE_WAIT: u32 = 600;
/// Updates between two raises of a dropped battle request.
const BATTLE_AGAIN: u32 = 30;

/// A battle asked for and not started yet.
struct Battle {
    field: u16,
    group: u8,
    formation: u8,
    reply: Reply,
    /// The update it was asked at, and last raised at.
    since: u32,
    raised: u32,
    times: u32,
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
    /// A battle request waiting for its battle.
    battle: Option<Battle>,
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

    /// The waiting battle, taken out.
    fn take_battle(&self) -> Option<Battle> {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .battle
            .take()
    }

    /// Puts a waiting battle back (it keeps the queue looked at).
    fn keep_battle(&self, b: Battle) {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        q.battle = Some(b);
        self.waiting.store(true, Ordering::Release);
    }

    fn take(&self) -> Vec<(Request, Reply)> {
        let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        self.waiting.store(q.battle.is_some(), Ordering::Release);
        q.items.drain(..).collect()
    }

    /// The run ended (`Exec::run_entry` returns): what waits is answered
    /// `Err`, and so is every later request.
    pub fn close(&self) {
        let (left, battle) = {
            let mut q = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
            q.closed = true;
            self.waiting.store(false, Ordering::Release);
            (q.items.drain(..).collect::<Vec<_>>(), q.battle.take())
        };
        for (_, reply) in left {
            reply(Err(ENDED.into()));
        }
        if let Some(b) = battle {
            (b.reply)(Err(ENDED.into()));
        }
    }
}

/// The boundary before update `frame` (the main thread, `crate::clock`).
pub(crate) fn boundary(e: &mut Exec, frame: u32) {
    if !e.shared().requests.waiting.load(Ordering::Acquire) {
        return;
    }
    if let Some(b) = e.shared().requests.take_battle() {
        battle_step(e, frame, b);
    }
    for (request, reply) in e.shared().requests.take() {
        // a request that stopped the game: the rest are not run on it
        if e.stopped().is_some() {
            reply(Err(ENDED.into()));
            continue;
        }
        match request {
            Request::DevSave(n) => {
                let r = crate::devsave::load(e, n);
                crate::log::note(&format!("requests: DevSave({n}) at frame {frame}: {r:?}"));
                reply(r);
            }
            Request::Battle {
                field,
                group,
                formation,
            } => {
                if let Some(old) = e.shared().requests.take_battle() {
                    crate::log::note("requests: a waiting battle replaced");
                    (old.reply)(Err("another battle was asked for".into()));
                }
                // a battle runs: the request would only rewrite its field
                if e.mem().read::<u8>(BTL_STATUS) != 0 {
                    crate::log::note("requests: battle refused: a battle is running");
                    reply(Err("a battle is running".into()));
                    continue;
                }
                if let Err(err) = ask_battle(e, field, group, formation) {
                    crate::log::note(&format!("requests: battle: ResetEncountExe: {err}"));
                    reply(Err(format!("ResetEncountExe: {err}")));
                    continue;
                }
                crate::log::note(&format!(
                    "requests: Battle({field}, {group}, {formation}) asked at frame {frame}"
                ));
                e.shared().requests.keep_battle(Battle {
                    field,
                    group,
                    formation,
                    reply,
                    since: frame,
                    raised: frame,
                    times: 1,
                });
            }
        }
    }
}

/// Writes a random encounter's request (as `start_battle` in
/// ffx-harness's screens.rs): the field, group and formation,
/// `ResetEncountExe(1)`, the request flag (not raised when the call fails;
/// the field, group and formation stay written then, as in the harness).
fn ask_battle(e: &mut Exec, field: u16, group: u8, formation: u8) -> crate::Result<()> {
    {
        let mut mem = e.mem();
        mem.write(BTL_FIELD_INDEX, field);
        mem.write(BTL_FIELD_INDEX + 2, group);
        mem.write(BTL_FIELD_INDEX + 3, formation);
    }
    e.call_cdecl(RESET_ENCOUNT_EXE, &[1])?;
    e.mem().write(BTL_EXE, 1u8);
    Ok(())
}

/// A waiting battle at the boundary: answered when it runs or has waited
/// too long, raised again while the field drops the request.
fn battle_step(e: &mut Exec, frame: u32, mut b: Battle) {
    if e.stopped().is_some() {
        (b.reply)(Err(ENDED.into()));
        return;
    }
    let (status, flag) = {
        let mem = e.mem();
        (mem.read::<u8>(BTL_STATUS), mem.read::<u8>(BTL_EXE))
    };
    let done = if status != 0 {
        Some(Ok(format!(
            "battle field {} group {} formation {} started at frame {frame} (asked {} times)",
            b.field, b.group, b.formation, b.times
        )))
    } else if frame.wrapping_sub(b.since) > BATTLE_WAIT {
        Some(Err(format!(
            "no battle after {BATTLE_WAIT} updates (asked {} times)",
            b.times
        )))
    } else if flag == 0 && frame.wrapping_sub(b.raised) >= BATTLE_AGAIN {
        match ask_battle(e, b.field, b.group, b.formation) {
            Ok(()) => {
                b.raised = frame;
                b.times += 1;
                None
            }
            Err(err) => Some(Err(format!("ResetEncountExe: {err}"))),
        }
    } else {
        None
    };
    if let Some(r) = done {
        crate::log::note(&format!("requests: battle at frame {frame}: {r:?}"));
        (b.reply)(r);
        return;
    }
    e.shared().requests.keep_battle(b);
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

    fn battle(reply: Reply) -> Battle {
        Battle {
            field: 3,
            group: 0,
            formation: 0,
            reply,
            since: 0,
            raised: 0,
            times: 1,
        }
    }

    #[test]
    fn a_waiting_battle_keeps_the_queue_looked_at_and_ends_with_the_run() {
        let q = Requests::default();
        let (got, reply) = recorder();
        q.keep_battle(battle(reply()));
        assert!(q.waiting.load(Ordering::Acquire));
        // taking the batch leaves the flag up while the battle waits
        let _ = q.take();
        assert!(q.waiting.load(Ordering::Acquire));
        q.close();
        assert_eq!(*got.lock().unwrap(), vec![Err(ENDED.to_string())]);
        assert!(q.take_battle().is_none());
        assert!(!q.waiting.load(Ordering::Acquire));
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
