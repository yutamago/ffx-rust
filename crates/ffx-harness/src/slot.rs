//! The game slot: only one FFX.exe runs at a time (the game refuses a
//! second one), so everything that starts the game queues for it here.
//!
//! The queue is a directory shared by every worktree,
//! `~/ffx-work/game-slot` (or `FFX_GAME_SLOT_DIR`):
//!
//! - `queue/<nanoseconds>-<pid>`: one ticket per waiter, served oldest
//!   first; a ticket whose process has died is removed by the next waiter;
//! - `lock`: held (an OS file lock) by the one process using the game, with
//!   a line saying who that is. The lock goes with the process, so a crashed
//!   holder frees the slot.
//!
//! [`Slot::acquire`] blocks until the slot is ours and says so on stderr
//! while it waits; dropping the [`Slot`] frees it. `game-slot status` shows
//! the holder and the queue, `game-slot run -- <command>` runs any command
//! in the slot.

use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The slot, held until dropped.
#[derive(Debug)]
pub struct Slot {
    _lock: File,
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

/// What the lock file says about its holder.
fn holder(lock: &Path) -> String {
    std::fs::read_to_string(lock)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

impl Slot {
    /// Queues for the game and returns once it is ours.
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
        let path = dir.join("lock");
        let start = Instant::now();
        let mut said = None;
        loop {
            let list = tickets(&queue);
            let ahead = list.iter().position(|(n, _)| *n == ticket).unwrap_or(0);
            if ahead == 0 {
                let mut lock = OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .read(true)
                    .write(true)
                    .open(&path)?;
                if lock.try_lock().is_ok() {
                    let _ = std::fs::remove_file(queue.join(&ticket));
                    lock.set_len(0)?;
                    lock.seek(SeekFrom::Start(0))?;
                    writeln!(
                        lock,
                        "pid {} since {}: {me}",
                        std::process::id(),
                        now.as_secs()
                    )?;
                    if said.is_some() {
                        eprintln!(
                            "game slot: ours after {:.0} s",
                            start.elapsed().as_secs_f64()
                        );
                    }
                    return Ok(Self { _lock: lock });
                }
            }
            if said != Some(ahead) {
                eprintln!(
                    "game slot: waiting ({ahead} ahead; in use by {})",
                    holder(&path)
                );
                said = Some(ahead);
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

/// The holder and the queue, as `game-slot status` prints them.
#[must_use]
pub fn status() -> String {
    let dir = dir();
    let path = dir.join("lock");
    let free = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_or(true, |f| f.try_lock().is_ok());
    let mut out = if free {
        "free\n".to_owned()
    } else {
        format!("in use: {}\n", holder(&path))
    };
    for (i, (name, _)) in tickets(&dir.join("queue")).iter().enumerate() {
        let mut who = String::new();
        let _ =
            File::open(dir.join("queue").join(name)).and_then(|mut f| f.read_to_string(&mut who));
        let _ = writeln!(out, "{}. {who}", i + 1);
    }
    out
}
