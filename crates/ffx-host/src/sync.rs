//! The game's synchronisation objects on real host primitives
//! (docs/host.md, "Threads"): events, semaphores, mutexes, thread handles
//! and critical sections. Every game thread runs on its own host thread,
//! so a wait here blocks that host thread as the original's wait blocks
//! its thread.
//!
//! One lock and one condition variable cover every object: a change
//! wakes all waiters, each re-checks its own objects. Waits also wake
//! every [`SLICE`] to see whether the run is ending
//! ([`Objects::stopping`]), so no host thread outlives the run blocked.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

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
}

/// Every object, shared by the game's threads.
#[derive(Debug, Default)]
pub struct Objects {
    inner: Mutex<Inner>,
    changed: Condvar,
    stopping: AtomicBool,
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

    /// `SetEvent` / `ResetEvent`; false for a handle that is no event.
    pub fn set_event(&self, handle: u32, to: bool) -> bool {
        let mut g = self.lock();
        let ok = match g.objects.get_mut(&handle) {
            Some(Obj::Event { set, .. }) => {
                *set = to;
                true
            }
            _ => false,
        };
        drop(g);
        self.changed.notify_all();
        ok
    }

    /// `ReleaseSemaphore`: the previous count, or `None` (not a semaphore,
    /// or past its maximum).
    pub fn release_semaphore(&self, handle: u32, by: i32) -> Option<i32> {
        let mut g = self.lock();
        let r = match g.objects.get_mut(&handle) {
            Some(Obj::Semaphore { count, max }) if *count + by <= *max => {
                let prev = *count;
                *count += by;
                Some(prev)
            }
            _ => None,
        };
        drop(g);
        self.changed.notify_all();
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
        drop(g);
        self.changed.notify_all();
        ok
    }

    /// A thread ended with `code`.
    pub fn thread_done(&self, handle: u32, exit: u32) {
        let mut g = self.lock();
        if let Some(Obj::Thread { code, done, .. }) = g.objects.get_mut(&handle) {
            *code = exit;
            *done = true;
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
                return WAIT_OBJECT_0 + u32::try_from(i).unwrap_or(0);
            }
            if self.stopping() {
                return WAIT_TIMEOUT;
            }
            let slice = match deadline {
                Some(d) => {
                    let left = d.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return WAIT_TIMEOUT;
                    }
                    left.min(SLICE)
                }
                None => SLICE,
            };
            g = self
                .changed
                .wait_timeout(g, slice)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
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
    /// before it got the section.
    pub fn section_enter(&self, at: u32, me: u32) -> bool {
        let mut g = self.lock();
        loop {
            let s = g.sections.entry(at).or_default();
            if s.owner == 0 || s.owner == me {
                s.owner = me;
                s.count += 1;
                return true;
            }
            if self.stopping() {
                return false;
            }
            g = self
                .changed
                .wait_timeout(g, SLICE)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
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
                drop(g);
                self.changed.notify_all();
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
        assert!(o.set_event(8, true));
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
        assert_eq!(o.release_semaphore(4, 2), Some(0));
        assert_eq!(o.release_semaphore(4, 1), None);
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
}
