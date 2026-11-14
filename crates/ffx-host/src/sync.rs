//! The game's synchronisation objects on real host primitives
//! (docs/host.md, "Threads"): events, semaphores, mutexes, thread handles
//! and critical sections. Every game thread runs on its own host thread,
//! so a wait here blocks that host thread as the original's wait blocks
//! its thread.
//!
//! One lock and one condition variable cover every object: a change to
//! an object some thread waits on wakes all waiters, each re-checks its
//! own objects (a change nobody waits on wakes nobody: m5-perf1, the
//! game's critical sections and events woke every blocked thread at every
//! leave and reset, a third of the headless boot's CPU in futex calls). Waits also wake
//! every [`SLICE`] to see whether the run is ending
//! ([`Objects::stopping`]), so no host thread outlives the run blocked.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::exec::MAIN_THREAD;

/// How often a blocked wait looks at the stop flag.
pub const SLICE: Duration = Duration::from_millis(50);

/// `WAIT_OBJECT_0`.
pub const WAIT_OBJECT_0: u32 = 0;
/// `WAIT_TIMEOUT`.
pub const WAIT_TIMEOUT: u32 = 0x102;
/// `WAIT_FAILED`.
pub const WAIT_FAILED: u32 = u32::MAX;
/// `INFINITE`.
pub const INFINITE: u32 = u32::MAX;
/// `STILL_ACTIVE`, a running thread's exit code.
pub const STILL_ACTIVE: u32 = 259;

/// One kernel object.
#[derive(Debug, Clone)]
pub enum Obj {
    /// An event: manual reset or not, signalled or not.
    Event {
        /// Stays signalled after a wait.
        manual: bool,
        /// Signalled.
        set: bool,
    },
    /// A semaphore.
    Semaphore {
        /// The count.
        count: i32,
        /// The maximum.
        max: i32,
    },
    /// A mutex: its owner's thread id (0: free) and recursion count.
    Mutex {
        /// The owner.
        owner: u32,
        /// How often the owner holds it.
        count: u32,
    },
    /// A thread: signalled when it ends.
    Thread {
        /// The game's thread id.
        id: u32,
        /// `STILL_ACTIVE`, or the exit code.
        code: u32,
        /// Ended.
        done: bool,
    },
}

impl Obj {
    /// Whether a wait by `me` would be satisfied now.
    fn ready(&self, me: u32) -> bool {
        match *self {
            Self::Event { set, .. } => set,
            Self::Semaphore { count, .. } => count > 0,
            Self::Mutex { owner, .. } => owner == 0 || owner == me,
            Self::Thread { done, .. } => done,
        }
    }

    /// What a satisfied wait takes.
    fn consume(&mut self, me: u32) {
        match self {
            Self::Event { manual, set } => {
                if !*manual {
                    *set = false;
                }
            }
            Self::Semaphore { count, .. } => *count -= 1,
            Self::Mutex { owner, count } => {
                *owner = me;
                *count += 1;
            }
            Self::Thread { .. } => {}
        }
    }
}

/// A critical section: its owner's thread id (0: free) and recursion
/// count.
#[derive(Debug, Clone, Copy, Default)]
struct Section {
    owner: u32,
    count: u32,
}

#[derive(Debug, Default)]
struct Inner {
    objects: HashMap<u32, Obj>,
    /// Critical sections by the game address of their `CRITICAL_SECTION`.
    sections: HashMap<u32, Section>,
    /// Threads `TerminateThread` ended, by game thread id: the exit code.
    killed: HashMap<u32, u32>,
    /// Blocked waits per object handle ([`Objects::wait`]).
    waiting: HashMap<u32, u32>,
    /// Blocked `EnterCriticalSection`s per section.
    section_waiting: HashMap<u32, u32>,
    /// The thread that last signalled each event or semaphore
    /// (`SetEvent`, `ReleaseSemaphore`): the one a waiter waits for.
    signaller: HashMap<u32, u32>,
    /// Game threads that stopped on a failure (an unported address, a
    /// failed trap), in order ([`Objects::thread_failed`]).
    failed: Vec<Failure>,
    /// Game threads that ended (returned, `ExitThread`, failed, or
    /// terminated), by id.
    ended: std::collections::HashSet<u32>,
    /// What each blocked thread waits for now (for the hang report).
    blocked: HashMap<u32, Blocked>,
    /// The stop for the main thread's last wait, which can never end
    /// ([`Objects::take_dead`]).
    dead: Option<Failure>,
}

/// A game thread's stop on a failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The game thread.
    pub thread: u32,
    /// Why (`Stop::Failed`'s text: the unported address and its callers).
    pub why: String,
    /// Where (`Stop::Failed`'s address).
    pub eip: u32,
}

/// What a blocked thread waits for.
#[derive(Debug, Clone)]
enum Blocked {
    /// `WaitFor*Object*` on these handles.
    Objects(Vec<u32>),
    /// `EnterCriticalSection` on the section at this address.
    Section(u32),
}

impl Inner {
    /// Whether a wait blocks on `handle` now.
    fn awaited(&self, handle: u32) -> bool {
        self.waiting.get(&handle).is_some_and(|&n| n > 0)
    }

    /// The failure of game thread `id`, if it stopped on one.
    fn failure(&self, id: u32) -> Option<&Failure> {
        self.failed.iter().find(|f| f.thread == id)
    }

    /// The main thread's stop when its wait on `handle`, not satisfied
    /// now, can never end: its mutex's owner, or the thread that signalled
    /// it before, stopped on a failure. The failure's text and address
    /// first (the cause), then the wait.
    fn dead_handle(&self, handle: u32) -> Option<Failure> {
        let (who, what, kind) = match self.objects.get(&handle)? {
            Obj::Mutex { owner, .. } if *owner != 0 => (*owner, "holds", "mutex"),
            Obj::Event { .. } => (*self.signaller.get(&handle)?, "signals", "event"),
            Obj::Semaphore { .. } => (*self.signaller.get(&handle)?, "releases", "semaphore"),
            Obj::Mutex { .. } | Obj::Thread { .. } => return None,
        };
        let f = self.failure(who)?;
        Some(Failure {
            thread: who,
            why: format!(
                "{} on thread {who:#x}; the main thread waits for {kind} {handle:#x}, which that \
                 thread {what}",
                f.why
            ),
            eip: f.eip,
        })
    }

    /// What thread `id` waits for, in words.
    fn blocked_text(&self, id: u32) -> Option<String> {
        Some(match self.blocked.get(&id)? {
            Blocked::Objects(h) => {
                let list: Vec<String> = h
                    .iter()
                    .map(|h| {
                        let kind = match self.objects.get(h) {
                            Some(Obj::Event { .. }) => "event",
                            Some(Obj::Semaphore { .. }) => "semaphore",
                            Some(Obj::Mutex { .. }) => "mutex",
                            Some(Obj::Thread { .. }) => "thread",
                            None => "handle",
                        };
                        match self.signaller.get(h) {
                            Some(t) => format!("{kind} {h:#x} (last signalled by thread {t:#x})"),
                            None => format!("{kind} {h:#x}"),
                        }
                    })
                    .collect();
                format!("waits for {}", list.join(", "))
            }
            Blocked::Section(at) => match self.sections.get(at) {
                Some(s) if s.owner != 0 => format!(
                    "waits for critical section {at:#010x}, held by thread {:#x}{}",
                    s.owner,
                    if self.killed.contains_key(&s.owner) {
                        " (terminated)"
                    } else if self.ended.contains(&s.owner) {
                        " (ended)"
                    } else {
                        ""
                    }
                ),
                _ => format!("waits for critical section {at:#010x}"),
            },
        })
    }
}

/// A wait shorter than this is a poll: a hang through polls is the
/// watchdog's to find (`Shared::check_hang`), not the wait's.
const DEAD_MIN_MS: u32 = 1000;

/// Counts `key` in or out of a waiting map.
fn note_wait(map: &mut HashMap<u32, u32>, key: u32, on: bool) {
    if on {
        *map.entry(key).or_default() += 1;
    } else if let Some(n) = map.get_mut(&key) {
        *n -= 1;
        if *n == 0 {
            map.remove(&key);
        }
    }
}

/// Every object, shared by the game's threads.
#[derive(Debug, Default)]
pub struct Objects {
    inner: Mutex<Inner>,
    changed: Condvar,
    stopping: AtomicBool,
    /// Some thread was terminated ([`Objects::killed`]'s fast path).
    any_killed: AtomicBool,
    /// Some game thread stopped on a failure (the dead-wait check's fast
    /// path).
    any_failed: AtomicBool,
}

impl Objects {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Ends every wait (the run is over): they return `WAIT_TIMEOUT` /
    /// leave sections unowned, and the threads unwind.
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        self.changed.notify_all();
    }

    /// Whether the run is ending.
    #[must_use]
    pub fn stopping(&self) -> bool {
        self.stopping.load(Ordering::Acquire)
    }

    /// `TerminateThread` of game thread `id` by another thread: its waits
    /// end (as at [`Objects::stop`]) and its executor unwinds at its next
    /// block or call ([`Objects::killed`]), with exit code `code`.
    pub fn kill(&self, id: u32, code: u32) {
        // the flag first: a waiter that sees itself in the map must find it
        // set ([`Objects::killed`]'s fast path)
        self.any_killed.store(true, Ordering::Release);
        self.lock().killed.entry(id).or_insert(code);
        self.changed.notify_all();
    }

    /// A game thread stopped on a failure: a wait of the main thread on
    /// what it signals, or a section it holds, can never end
    /// ([`Objects::wait`], [`Objects::section_enter`]).
    pub fn thread_failed(&self, f: Failure) {
        let mut g = self.lock();
        if g.failure(f.thread).is_none() {
            g.failed.push(f);
        }
        self.any_failed.store(true, Ordering::Release);
        drop(g);
        self.changed.notify_all();
    }

    /// The first game thread that stopped on a failure.
    #[must_use]
    pub fn first_failure(&self) -> Option<Failure> {
        if !self.any_failed.load(Ordering::Acquire) {
            return None;
        }
        self.lock().failed.first().cloned()
    }

    /// The stop for the main thread's wait that just ended as one that
    /// never could (`WAIT_FAILED` from [`Objects::wait`], false from
    /// [`Objects::section_enter`]): the failed thread's failure, then the
    /// wait. Taken once.
    pub fn take_dead(&self) -> Option<Failure> {
        self.lock().dead.take()
    }

    /// What game thread `id` is blocked on now, in words (`None` when it
    /// is not in a wait or a critical section).
    #[must_use]
    pub fn blocked(&self, id: u32) -> Option<String> {
        self.lock().blocked_text(id)
    }

    /// Whether any thread was terminated.
    #[must_use]
    pub fn any_killed(&self) -> bool {
        self.any_killed.load(Ordering::Acquire)
    }

    /// Whether game thread `id` was terminated: its exit code.
    #[must_use]
    pub fn killed(&self, id: u32) -> Option<u32> {
        if !self.any_killed() {
            return None;
        }
        self.lock().killed.get(&id).copied()
    }

    /// The game thread id behind thread handle `handle`, and whether it
    /// ended.
    #[must_use]
    pub fn thread_of(&self, handle: u32) -> Option<(u32, bool)> {
        match self.lock().objects.get(&handle) {
            Some(Obj::Thread { id, done, .. }) => Some((*id, *done)),
            _ => None,
        }
    }

    /// Adds `obj` under `handle`.
    pub fn insert(&self, handle: u32, obj: Obj) {
        self.lock().objects.insert(handle, obj);
    }

    /// Whether `handle` is one of these objects.
    #[must_use]
    pub fn contains(&self, handle: u32) -> bool {
        self.lock().objects.contains_key(&handle)
    }

    /// Drops `handle` (`CloseHandle`).
    pub fn remove(&self, handle: u32) {
        self.lock().objects.remove(&handle);
    }

    /// `SetEvent` / `ResetEvent` by game thread `me`; false for a handle
    /// that is no event.
    pub fn set_event(&self, handle: u32, to: bool, me: u32) -> bool {
        let mut g = self.lock();
        let ok = match g.objects.get_mut(&handle) {
            Some(Obj::Event { set, .. }) => {
                *set = to;
                true
            }
            _ => false,
        };
        if ok && to {
            g.signaller.insert(handle, me);
        }
        // a reset satisfies no wait
        let wake = to && g.awaited(handle);
        drop(g);
        if wake {
            self.changed.notify_all();
        }
        ok
    }

    /// `ReleaseSemaphore` by game thread `me`: the previous count, or
    /// `None` (not a semaphore, or past its maximum).
    pub fn release_semaphore(&self, handle: u32, by: i32, me: u32) -> Option<i32> {
        let mut g = self.lock();
        let r = match g.objects.get_mut(&handle) {
            Some(Obj::Semaphore { count, max }) if *count + by <= *max => {
                let prev = *count;
                *count += by;
                Some(prev)
            }
            _ => None,
        };
        if r.is_some() {
            g.signaller.insert(handle, me);
        }
        let wake = g.awaited(handle);
        drop(g);
        if wake {
            self.changed.notify_all();
        }
        r
    }

    /// `ReleaseMutex` by `me`.
    pub fn release_mutex(&self, handle: u32, me: u32) -> bool {
        let mut g = self.lock();
        let ok = match g.objects.get_mut(&handle) {
            Some(Obj::Mutex { owner, count }) if *owner == me && *count > 0 => {
                *count -= 1;
                if *count == 0 {
                    *owner = 0;
                }
                true
            }
            _ => false,
        };
        let wake = g.awaited(handle);
        drop(g);
        if wake {
            self.changed.notify_all();
        }
        ok
    }

    /// A thread ended with `code`.
    pub fn thread_done(&self, handle: u32, exit: u32) {
        let mut g = self.lock();
        if let Some(Obj::Thread { id, code, done }) = g.objects.get_mut(&handle) {
            *code = exit;
            *done = true;
            let id = *id;
            g.ended.insert(id);
        }
        drop(g);
        self.changed.notify_all();
    }

    /// `GetExitCodeThread`.
    #[must_use]
    pub fn thread_code(&self, handle: u32) -> Option<u32> {
        match self.lock().objects.get(&handle) {
            Some(Obj::Thread { code, .. }) => Some(*code),
            _ => None,
        }
    }

    /// `WaitForSingleObject` / `WaitForMultipleObjects` by thread `me`:
    /// waits until one (or, with `all`, every) handle is signalled, or
    /// `ms` milliseconds pass, or the run ends. Handles that are no object
    /// here (files, pseudo handles) count as signalled.
    pub fn wait(&self, me: u32, handles: &[u32], all: bool, ms: u32) -> u32 {
        let deadline = (ms != INFINITE).then(|| Instant::now() + Duration::from_millis(ms.into()));
        let mut g = self.lock();
        // whether `blocked` holds this wait (taken out on every return)
        let mut noted = false;
        loop {
            let ready: Vec<bool> = handles
                .iter()
                .map(|h| g.objects.get(h).is_none_or(|o| o.ready(me)))
                .collect();
            let hit = if all {
                ready.iter().all(|&r| r).then_some(0)
            } else {
                ready.iter().position(|&r| r)
            };
            if let Some(i) = hit {
                let take: Vec<u32> = if all {
                    handles.to_vec()
                } else {
                    vec![handles[i]]
                };
                for h in take {
                    if let Some(o) = g.objects.get_mut(&h) {
                        o.consume(me);
                    }
                }
                if noted {
                    g.blocked.remove(&me);
                }
                return WAIT_OBJECT_0 + u32::try_from(i).unwrap_or(0);
            }
            if self.stopping() || g.killed.contains_key(&me) {
                g.blocked.remove(&me);
                return WAIT_TIMEOUT;
            }
            // the main thread waiting for what a failed thread would have
            // signalled: the wait never ends (`all`: one such handle is
            // enough; else every handle not yet signalled must be one)
            if me == MAIN_THREAD && ms >= DEAD_MIN_MS && self.any_failed.load(Ordering::Acquire) {
                let dead: Vec<Option<Failure>> = handles
                    .iter()
                    .zip(&ready)
                    .filter(|(_, r)| !**r)
                    .map(|(h, _)| g.dead_handle(*h))
                    .collect();
                let hit = if all {
                    dead.iter().flatten().next().cloned()
                } else if dead.iter().all(Option::is_some) {
                    dead.first().cloned().flatten()
                } else {
                    None
                };
                if hit.is_some() {
                    g.dead = hit;
                    g.blocked.remove(&me);
                    return WAIT_FAILED;
                }
            }
            if !noted {
                g.blocked.insert(me, Blocked::Objects(handles.to_vec()));
                noted = true;
            }
            let slice = match deadline {
                Some(d) => {
                    let left = d.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        g.blocked.remove(&me);
                        return WAIT_TIMEOUT;
                    }
                    left.min(SLICE)
                }
                None => SLICE,
            };
            for &h in handles {
                note_wait(&mut g.waiting, h, true);
            }
            g = self
                .changed
                .wait_timeout(g, slice)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
            for &h in handles {
                note_wait(&mut g.waiting, h, false);
            }
        }
    }

    /// `InitializeCriticalSection*`.
    pub fn section_init(&self, at: u32) {
        self.lock().sections.insert(at, Section::default());
    }

    /// `DeleteCriticalSection`.
    pub fn section_delete(&self, at: u32) {
        self.lock().sections.remove(&at);
    }

    /// `EnterCriticalSection` by `me` (blocks); false when the run ended
    /// before it got the section, or (the main thread) when its owner
    /// stopped on a failure, so it is never left ([`Objects::take_dead`]).
    pub fn section_enter(&self, at: u32, me: u32) -> bool {
        let mut g = self.lock();
        let mut noted = false;
        loop {
            let s = g.sections.entry(at).or_default();
            if s.owner == 0 || s.owner == me {
                s.owner = me;
                s.count += 1;
                if noted {
                    g.blocked.remove(&me);
                }
                return true;
            }
            let owner = s.owner;
            if self.stopping() || g.killed.contains_key(&me) {
                g.blocked.remove(&me);
                return false;
            }
            if me == MAIN_THREAD
                && self.any_failed.load(Ordering::Acquire)
                && let Some(f) = g.failure(owner)
            {
                g.dead = Some(Failure {
                    thread: owner,
                    why: format!(
                        "{} on thread {owner:#x}; the main thread waits for critical section \
                         {at:#010x}, which that thread holds",
                        f.why
                    ),
                    eip: f.eip,
                });
                g.blocked.remove(&me);
                return false;
            }
            // a terminated thread never leaves what it holds (it unwinds
            // at its next call, `LeaveCriticalSection` included), nor one
            // that ended without leaving it
            if me == MAIN_THREAD && (g.ended.contains(&owner) || g.killed.contains_key(&owner)) {
                let how = if g.killed.contains_key(&owner) {
                    "was terminated (TerminateThread)"
                } else {
                    "ended"
                };
                g.dead = Some(Failure {
                    thread: owner,
                    why: format!(
                        "thread {owner:#x} {how} holding critical section {at:#010x}; the main \
                         thread waits for it"
                    ),
                    eip: at,
                });
                g.blocked.remove(&me);
                return false;
            }
            if !noted {
                g.blocked.insert(me, Blocked::Section(at));
                noted = true;
            }
            note_wait(&mut g.section_waiting, at, true);
            g = self
                .changed
                .wait_timeout(g, SLICE)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
            note_wait(&mut g.section_waiting, at, false);
        }
    }

    /// `TryEnterCriticalSection` by `me`.
    pub fn section_try(&self, at: u32, me: u32) -> bool {
        let mut g = self.lock();
        let s = g.sections.entry(at).or_default();
        if s.owner == 0 || s.owner == me {
            s.owner = me;
            s.count += 1;
            true
        } else {
            false
        }
    }

    /// `LeaveCriticalSection` by `me`.
    pub fn section_leave(&self, at: u32, me: u32) {
        let mut g = self.lock();
        if let Some(s) = g.sections.get_mut(&at)
            && s.owner == me
            && s.count > 0
        {
            s.count -= 1;
            if s.count == 0 {
                s.owner = 0;
                if g.section_waiting.contains_key(&at) {
                    drop(g);
                    self.changed.notify_all();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn auto_reset_event_wakes_one_waiter_per_set() {
        let o = Arc::new(Objects::default());
        o.insert(
            8,
            Obj::Event {
                manual: false,
                set: false,
            },
        );
        let o2 = Arc::clone(&o);
        let t = std::thread::spawn(move || o2.wait(2, &[8], false, INFINITE));
        std::thread::sleep(Duration::from_millis(20));
        assert!(o.set_event(8, true, 1));
        assert_eq!(t.join().unwrap(), WAIT_OBJECT_0);
        // consumed: the next wait times out
        assert_eq!(o.wait(1, &[8], false, 10), WAIT_TIMEOUT);
    }

    #[test]
    fn critical_section_excludes_other_threads_and_recurses() {
        let o = Arc::new(Objects::default());
        o.section_init(0x100);
        assert!(o.section_enter(0x100, 1));
        assert!(o.section_enter(0x100, 1));
        assert!(!o.section_try(0x100, 2));
        let o2 = Arc::clone(&o);
        let t = std::thread::spawn(move || o2.section_enter(0x100, 2));
        o.section_leave(0x100, 1);
        std::thread::sleep(Duration::from_millis(20));
        assert!(!t.is_finished());
        o.section_leave(0x100, 1);
        assert!(t.join().unwrap());
    }

    #[test]
    fn semaphore_counts_and_stop_ends_waits() {
        let o = Arc::new(Objects::default());
        o.insert(4, Obj::Semaphore { count: 1, max: 2 });
        assert_eq!(o.wait(1, &[4], false, 0), WAIT_OBJECT_0);
        assert_eq!(o.wait(1, &[4], false, 0), WAIT_TIMEOUT);
        assert_eq!(o.release_semaphore(4, 2, 1), Some(0));
        assert_eq!(o.release_semaphore(4, 1, 1), None);
        let o2 = Arc::clone(&o);
        o.insert(
            12,
            Obj::Event {
                manual: true,
                set: false,
            },
        );
        let t = std::thread::spawn(move || o2.wait(3, &[12], false, INFINITE));
        o.stop();
        assert_eq!(t.join().unwrap(), WAIT_TIMEOUT);
    }

    #[test]
    fn kill_ends_only_that_threads_waits() {
        let o = Arc::new(Objects::default());
        o.insert(
            16,
            Obj::Event {
                manual: true,
                set: false,
            },
        );
        o.section_init(0x200);
        assert!(o.section_enter(0x200, 9));
        assert_eq!(o.killed(5), None);
        let (o2, o3) = (Arc::clone(&o), Arc::clone(&o));
        let waiter = std::thread::spawn(move || o2.wait(5, &[16], false, INFINITE));
        let entering = std::thread::spawn(move || o3.section_enter(0x200, 5));
        let o4 = Arc::clone(&o);
        let other = std::thread::spawn(move || o4.wait(6, &[16], false, 300));
        std::thread::sleep(Duration::from_millis(20));
        o.kill(5, 0x77);
        assert_eq!(waiter.join().unwrap(), WAIT_TIMEOUT);
        assert!(!entering.join().unwrap());
        assert_eq!(o.killed(5), Some(0x77));
        assert_eq!(o.killed(6), None);
        // thread 6 waits its full time out; the section stays thread 9's
        assert_eq!(other.join().unwrap(), WAIT_TIMEOUT);
        assert!(!o.section_try(0x200, 6));
    }

    fn event(o: &Objects, h: u32) {
        o.insert(
            h,
            Obj::Event {
                manual: false,
                set: false,
            },
        );
    }

    fn failure(thread: u32) -> Failure {
        Failure {
            thread,
            why: "unported address 0x0077ba70 (x) called from y".into(),
            eip: 0x0077_ba70,
        }
    }

    #[test]
    fn the_main_threads_wait_for_a_failed_threads_event_ends_naming_both() {
        let o = Arc::new(Objects::default());
        event(&o, 20);
        event(&o, 24);
        // thread 7 signalled 20 once (consumed), thread 8 signals 24
        assert!(o.set_event(20, true, 7));
        assert_eq!(o.wait(MAIN_THREAD, &[20], false, 0), WAIT_OBJECT_0);
        assert!(o.set_event(24, true, 8));
        assert_eq!(o.wait(MAIN_THREAD, &[24], false, 0), WAIT_OBJECT_0);
        // the failure comes while the main thread waits
        let o2 = Arc::clone(&o);
        let main = std::thread::spawn(move || o2.wait(MAIN_THREAD, &[20], false, INFINITE));
        std::thread::sleep(Duration::from_millis(30));
        assert!(!main.is_finished());
        o.thread_failed(failure(7));
        assert_eq!(main.join().unwrap(), WAIT_FAILED);
        let dead = o.take_dead().expect("the dead wait's stop");
        assert_eq!((dead.thread, dead.eip), (7, 0x0077_ba70));
        assert!(
            dead.why
                .starts_with("unported address 0x0077ba70 (x) called from y on thread 0x7; ")
                && dead
                    .why
                    .contains("waits for event 0x14, which that thread signals"),
            "{}",
            dead.why
        );
        assert!(o.take_dead().is_none(), "taken once");
        // another thread's wait on it is only a wait (it times out)
        assert_eq!(o.wait(3, &[20], false, 1000), WAIT_TIMEOUT);
        // a poll is not judged; nor an any-wait with a live signaller
        assert_eq!(o.wait(MAIN_THREAD, &[20], false, 100), WAIT_TIMEOUT);
        assert_eq!(
            o.wait(MAIN_THREAD, &[20, 24], false, 1000),
            WAIT_TIMEOUT,
            "thread 8 can still signal 24"
        );
        assert!(o.take_dead().is_none());
        // a wait for all: the dead handle is enough
        assert_eq!(o.wait(MAIN_THREAD, &[24, 20], true, INFINITE), WAIT_FAILED);
        assert_eq!(o.take_dead().map(|d| d.thread), Some(7));
        assert_eq!(o.first_failure(), Some(failure(7)));
    }

    #[test]
    fn the_main_thread_entering_a_failed_threads_section_ends() {
        let o = Arc::new(Objects::default());
        o.section_init(0x300);
        assert!(o.section_enter(0x300, 9));
        o.thread_failed(failure(9));
        // another thread just waits (until the run stops)
        let o2 = Arc::clone(&o);
        let other = std::thread::spawn(move || o2.section_enter(0x300, 5));
        assert!(!o.section_enter(0x300, MAIN_THREAD));
        let dead = o.take_dead().expect("the dead wait's stop");
        assert!(
            dead.why
                .contains("waits for critical section 0x00000300, which that thread holds"),
            "{}",
            dead.why
        );
        std::thread::sleep(Duration::from_millis(30));
        assert!(!other.is_finished());
        assert!(
            o.blocked(5)
                .is_some_and(|b| b.contains("held by thread 0x9"))
        );
        o.stop();
        assert!(!other.join().unwrap());
        assert!(o.blocked(5).is_none());
    }

    #[test]
    fn the_main_thread_entering_a_section_a_terminated_thread_holds_ends() {
        let o = Objects::default();
        o.section_init(0x400);
        o.insert(
            40,
            Obj::Thread {
                id: 11,
                code: STILL_ACTIVE,
                done: false,
            },
        );
        assert!(o.section_enter(0x400, 11));
        o.kill(11, 0);
        assert!(!o.section_enter(0x400, MAIN_THREAD));
        let dead = o.take_dead().expect("the dead wait's stop");
        assert_eq!(
            dead.why,
            "thread 0xb was terminated (TerminateThread) holding critical section 0x00000400; \
             the main thread waits for it"
        );
        // one that ended holding it
        o.section_init(0x404);
        o.insert(
            44,
            Obj::Thread {
                id: 12,
                code: STILL_ACTIVE,
                done: false,
            },
        );
        assert!(o.section_enter(0x404, 12));
        o.thread_done(44, 0);
        assert!(!o.section_enter(0x404, MAIN_THREAD));
        assert!(
            o.take_dead()
                .is_some_and(|d| d.why.starts_with("thread 0xc ended holding"))
        );
    }
}
