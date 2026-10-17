//! The game slots: a pool of `N` slots (`FFX_GAME_SLOTS`, default
//! [`DEFAULT_SLOTS`]), each one a game instance that runs beside the others
//! (`docs/harness.md`, "Parallel games"). Slot `n` selects the instance
//! folder (`tools/instance.sh n`), the harness port (`27100 + n`), the
//! sandboxed Documents folder and the output directory.
//!
//! The pool is a directory shared by every worktree,
//! `~/ffx-work/game-slot` (or `FFX_GAME_SLOT_DIR`):
//!
//! - `queue/<nanoseconds>-<pid>`: one ticket per waiter, served oldest
//!   first; a ticket whose process has died is removed by the next waiter;
//! - `lock-<n>`: held (an OS file lock) by the one process using slot `n`,
//!   with a line saying who that is. The lock goes with the process, so a
//!   crashed holder frees the slot;
//! - `lock`: held shared by every slot holder. A launcher that wants the
//!   game to itself (an older checkout of this crate, `tools/run-game.sh`
//!   without `--instance`) takes it exclusively and so waits for every slot.
//!
//! [`Slot::acquire`] blocks until a slot is ours and says so on stderr
//! while it waits; dropping the [`Slot`] frees it. `game-slot status` shows
//! the holders and the queue, `game-slot run -- <command>` runs any command
//! in a slot (`FFX_GAME_SLOT=<n>` tells it which).

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Slots when `FFX_GAME_SLOTS` is not set (measured in `docs/harness.md`).
pub const DEFAULT_SLOTS: u32 = 3;

/// A slot, held until dropped.
#[derive(Debug)]
pub struct Slot {
    number: u32,
    _lock: File,
    _shared: File,
}

/// The slot directory.
#[must_use]
pub fn dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FFX_GAME_SLOT_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME").map_or_else(std::env::temp_dir, PathBuf::from);
    home.join("ffx-work/game-slot")
}

/// How many slots the pool has: `FFX_GAME_SLOTS`, at least 1.
#[must_use]
pub fn count() -> u32 {
    std::env::var("FFX_GAME_SLOTS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(DEFAULT_SLOTS)
        .max(1)
}

/// The slot a parent process holds for this one (`FFX_GAME_SLOT_HELD=1`,
/// `FFX_GAME_SLOT=<n>`, set by `game-slot run`), or `None`.
#[must_use]
pub fn held() -> Option<u32> {
    std::env::var("FFX_GAME_SLOT_HELD")
        .is_ok_and(|v| v == "1")
        .then(|| {
            std::env::var("FFX_GAME_SLOT")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(0)
        })
}

/// Slot `n`'s lock file.
fn lock_path(dir: &Path, n: u32) -> PathBuf {
    dir.join(format!("lock-{n}"))
}

/// Who is asking: `FFX_GAME_SLOT_LABEL`, else the working directory (the
/// worktree) and the command.
fn label() -> String {
    if let Ok(l) = std::env::var("FFX_GAME_SLOT_LABEL") {
        return l;
    }
    let cwd = std::env::current_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    let args: Vec<String> = std::env::args().collect();
    format!("{cwd}: {}", args.join(" "))
}

fn alive(pid: u32) -> bool {
    // Waiters run under WSL/Linux; elsewhere assume alive.
    !Path::new("/proc").exists() || Path::new(&format!("/proc/{pid}")).exists()
}

/// The live tickets, oldest first; dead ones are removed.
fn tickets(queue: &Path) -> Vec<(String, u32)> {
    let mut list: Vec<(String, u32)> = std::fs::read_dir(queue)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let pid = name.rsplit('-').next()?.parse().ok()?;
            Some((name, pid))
        })
        .collect();
    list.retain(|(name, pid)| {
        let ok = alive(*pid);
        if !ok {
            let _ = std::fs::remove_file(queue.join(name));
        }
        ok
    });
    list.sort();
    list
}

/// What a lock file says about its holder.
fn holder(lock: &Path) -> String {
    std::fs::read_to_string(lock)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn open_lock(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
}

/// Whether nobody holds the lock at `path`.
fn free(path: &Path) -> bool {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_or(true, |f| f.try_lock().is_ok())
}

impl Slot {
    /// Queues for a slot and returns once one is ours.
    ///
    /// # Errors
    /// Fails when the slot directory cannot be used.
    pub fn acquire() -> std::io::Result<Self> {
        let dir = dir();
        let queue = dir.join("queue");
        std::fs::create_dir_all(&queue)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let ticket = format!("{:020}-{}", now.as_nanos(), std::process::id());
        let me = label();
        std::fs::write(queue.join(&ticket), &me)?;
        let slots = count();
        let start = Instant::now();
        let mut said = None;
        loop {
            let list = tickets(&queue);
            let ahead = list.iter().position(|(n, _)| *n == ticket).unwrap_or(0);
            // The oldest `slots` waiters may take whatever slot is free.
            if ahead < slots as usize
                && let Some(slot) = Self::try_take(&dir, slots, &me, now)?
            {
                let _ = std::fs::remove_file(queue.join(&ticket));
                if said.is_some() {
                    eprintln!(
                        "game slot: slot {} ours after {:.0} s",
                        slot.number,
                        start.elapsed().as_secs_f64()
                    );
                }
                return Ok(slot);
            }
            if said != Some(ahead) {
                eprintln!(
                    "game slot: waiting ({ahead} ahead; {})",
                    in_use(&dir, slots).join("; ")
                );
                said = Some(ahead);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Takes the first free slot, with the shared pool lock.
    fn try_take(dir: &Path, slots: u32, me: &str, now: Duration) -> std::io::Result<Option<Self>> {
        let shared = open_lock(&dir.join("lock"))?;
        if shared.try_lock_shared().is_err() {
            // Someone has the game to themselves.
            return Ok(None);
        }
        for number in 0..slots {
            let mut lock = open_lock(&lock_path(dir, number))?;
            if lock.try_lock().is_ok() {
                lock.set_len(0)?;
                lock.seek(SeekFrom::Start(0))?;
                writeln!(
                    lock,
                    "pid {} since {}: {me}",
                    std::process::id(),
                    now.as_secs()
                )?;
                return Ok(Some(Self {
                    number,
                    _lock: lock,
                    _shared: shared,
                }));
            }
        }
        Ok(None)
    }

    /// The slot's number (its instance).
    #[must_use]
    pub fn number(&self) -> u32 {
        self.number
    }
}

/// `slot n: holder` for every slot in use, or what holds the whole game.
fn in_use(dir: &Path, slots: u32) -> Vec<String> {
    let mut out: Vec<String> = (0..slots.max(highest_lock(dir) + 1))
        .filter(|&n| !free(&lock_path(dir, n)))
        .map(|n| format!("slot {n}: {}", holder(&lock_path(dir, n))))
        .collect();
    let whole = dir.join("lock");
    if out.is_empty() && !free(&whole) {
        out.push(format!("the whole game: {}", holder(&whole)));
    }
    out
}

/// The highest slot number with a lock file (a pool once larger).
fn highest_lock(dir: &Path) -> u32 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_prefix("lock-")?
                .parse::<u32>()
                .ok()
        })
        .max()
        .unwrap_or(0)
}

/// The holders and the queue, as `game-slot status` prints them.
#[must_use]
pub fn status() -> String {
    let dir = dir();
    let slots = count();
    let busy = in_use(&dir, slots);
    let mut out = if busy.is_empty() {
        format!("free ({slots} slots)\n")
    } else {
        format!(
            "{} of {slots} slots in use\n{}\n",
            busy.iter().filter(|b| b.starts_with("slot ")).count(),
            busy.join("\n")
        )
    };
    for (i, (name, _)) in tickets(&dir.join("queue")).iter().enumerate() {
        let mut who = String::new();
        let _ =
            File::open(dir.join("queue").join(name)).and_then(|mut f| f.read_to_string(&mut who));
        let _ = writeln!(out, "{}. {who}", i + 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two holders get two slots; a third waits until one is dropped; a
    /// whole-game holder (exclusive `lock`) keeps everyone out.
    #[test]
    fn slots_are_shared_out_and_freed() {
        let dir = std::env::temp_dir().join(format!("ffx-slot-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let take = || Slot::try_take(&dir, 2, "test", Duration::ZERO).unwrap();
        // A child process spawned meanwhile by another test holds copies of
        // our lock files' descriptors until it execs: wait that out.
        let take_freed = || {
            (0..100)
                .find_map(|_| {
                    take().or_else(|| {
                        std::thread::sleep(Duration::from_millis(10));
                        None
                    })
                })
                .unwrap()
        };
        let a = take().unwrap();
        let b = take().unwrap();
        assert_eq!((a.number(), b.number()), (0, 1));
        assert!(take().is_none());
        drop(a);
        let c = take_freed();
        assert_eq!(c.number(), 0);
        drop((b, c));
        let whole = open_lock(&dir.join("lock")).unwrap();
        whole.lock().unwrap();
        assert!(take().is_none());
        drop(whole);
        drop(take_freed());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
