//! The test harness inside `ffx-rust` (docs/harness.md, "Two processes"):
//! the protocol `ffx-hook`'s harness serves inside `FFX.exe`
//! (`crates/ffx-hook/src/harness.rs`), served from `ffx-host`, so
//! `ffx-harness` drives both programs the same way.
//!
//! With `FFX_HARNESS=<port>` a thread listens on `127.0.0.1:<port>`, one
//! command a line, one reply a line (`ok …` / `err …`), `TCP_NODELAY` and
//! each reply in one write, as the hook's `serve`. The environment the
//! hook reads means the same here:
//!
//! - `FFX_HARNESS_BACKGROUND=1`: the window is never activated and stays at
//!   the bottom of the window stack, the mouse is the test's
//!   ([`crate::win_forward`]);
//! - `FFX_HARNESS_HOME=<folder>`: the game's Documents folder (settings,
//!   saves), the program's `--documents`.
//!
//! What is served, and how it differs from the hook's:
//!
//! | Command | Here |
//! |---|---|
//! | `ping`, `stats`, `frame`, `wait <n>` | frames are `Present`s: the update boundary (`FFXApplication_animate`) has no hook in `ffx-host` yet, so `frame` and `wait` count images |
//! | `read`, `readp`, `readabs`, `write`, `writep` | arena memory (on i686 a game address is the process address) |
//! | `keys`, `key` | the `DirectInput` keyboard's state (`GetDeviceState`) |
//! | `pad`, `press` | the buttons' default keys held (`GameSetting.ini`'s bindings in the sandbox: Circle C, Cross X, Start B, …), where the hook answers `Input_KeyHeld` |
//! | `shot <path>` | the back buffer at the next `Present`, as a 32-bit BMP; `ok <w> <h> update <n>` with `n` the present |
//! | `gfxcapture <dir>` | the layer's recording so far (`capture.ffxgfx`, `calls.txt`), every frame from the first `Present` |
//! | `quit` | ends the process |
//!
//! Not yet (answered `err`): `call`, `waitfor`, `shotwhen`, `loadsave`,
//! `snapshot`, `lockstep`, `pin` and the rest, which need the frame
//! boundary and the virtual clock (docs/host.md, "What blocks the next
//! step").

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use ffx_bridge::Mem;

use crate::arena::ArenaMem;
use crate::exec::Shared;

static BACKGROUND: OnceLock<bool> = OnceLock::new();
static PRESENTS: AtomicU32 = AtomicU32::new(0);
static KEY_POLLS: AtomicU32 = AtomicU32::new(0);
static TICK: (Mutex<()>, Condvar) = (Mutex::new(()), Condvar::new());
/// Keys the test holds (`DirectInput` scan codes).
static KEYS: Mutex<[bool; 256]> = Mutex::new([false; 256]);
/// A `shot` waiting for the next `Present`: the path and the reply.
static SHOT: Mutex<Option<(String, Sender<String>)>> = Mutex::new(None);
static ON: AtomicBool = AtomicBool::new(false);

/// Background mode (`FFX_HARNESS_BACKGROUND=1`).
pub fn background() -> bool {
    *BACKGROUND.get_or_init(|| std::env::var("FFX_HARNESS_BACKGROUND").is_ok_and(|v| v == "1"))
}

/// Whether the harness listens.
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// The sandboxed Documents folder (`FFX_HARNESS_HOME`), if any.
#[must_use]
pub fn home() -> Option<std::path::PathBuf> {
    std::env::var_os("FFX_HARNESS_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
}

/// Starts the listener when `FFX_HARNESS` names a port.
///
/// # Errors
/// When the port can't be bound.
pub fn start(shared: &Arc<Shared>) -> std::io::Result<()> {
    let Some(port) = std::env::var("FFX_HARNESS")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
    else {
        return Ok(());
    };
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    ON.store(true, Ordering::Relaxed);
    let shared = Arc::clone(shared);
    std::thread::Builder::new()
        .name("ffx-host harness".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&shared);
                let _ = std::thread::Builder::new()
                    .name("ffx-host harness client".into())
                    .spawn(move || serve(&shared, stream));
            }
        })?;
    Ok(())
}

fn serve(shared: &Arc<Shared>, stream: TcpStream) {
    let _ = stream.set_nodelay(true);
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        let reply = command(shared, line.trim());
        if out.write_all(format!("{reply}\n").as_bytes()).is_err() {
            return;
        }
    }
}

/// `Present`s so far.
pub fn presents() -> u32 {
    PRESENTS.load(Ordering::Relaxed)
}

/// The layer's `Present` (before the back end presents): counts it and
/// takes a waiting screenshot from `image` (the back buffer as BGRA rows,
/// `None` without one).
pub fn on_present(image: impl FnOnce() -> Option<(u32, u32, Vec<u8>)>) {
    let n = PRESENTS.fetch_add(1, Ordering::Relaxed) + 1;
    if enabled() {
        let pending = SHOT.lock().ok().and_then(|mut s| s.take());
        if let Some((path, reply)) = pending {
            let r = match image() {
                Some((w, h, bgra)) => match std::fs::write(&path, bmp(w, h, &bgra)) {
                    Ok(()) => format!("ok {w} {h} update {n}"),
                    Err(e) => format!("err {path}: {e}"),
                },
                None => "err no back buffer to read (the null device)".into(),
            };
            let _ = reply.send(r);
        }
    }
    let _l = TICK.0.lock();
    TICK.1.notify_all();
}

/// The `DirectInput` keyboard's state (`GetDeviceState`, 256 bytes): the
/// keys the test holds read `0x80`.
pub fn keyboard_state(out: &mut [u8]) {
    out.fill(0);
    KEY_POLLS.fetch_add(1, Ordering::Relaxed);
    if let Ok(k) = KEYS.lock() {
        for (o, &held) in out.iter_mut().zip(k.iter()) {
            if held {
                *o = 0x80;
            }
        }
    }
}

/// A 32-bit bottom-up BMP of `w` x `h` BGRA rows.
fn bmp(w: u32, h: u32, bgra: &[u8]) -> Vec<u8> {
    let size = w * h * 4;
    let mut f = Vec::with_capacity(54 + size as usize);
    f.extend(b"BM");
    f.extend((54 + size).to_le_bytes());
    f.extend(0u32.to_le_bytes());
    f.extend(54u32.to_le_bytes());
    f.extend(40u32.to_le_bytes());
    f.extend(w.to_le_bytes());
    f.extend(h.to_le_bytes());
    f.extend(1u16.to_le_bytes());
    f.extend(32u16.to_le_bytes());
    f.extend([0u8; 24]);
    let row = (w * 4) as usize;
    for y in (0..h as usize).rev() {
        f.extend(bgra.get(y * row..(y + 1) * row).unwrap_or(&[]));
    }
    f
}

fn wait_presents(n: u32) {
    let target = presents().saturating_add(n);
    let mut l = TICK
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    while presents() < target {
        l = match TICK.1.wait_timeout(l, Duration::from_millis(200)) {
            Ok((l, _)) => l,
            Err(e) => e.into_inner().0,
        };
    }
}

fn set_keys(codes: &[u8]) {
    if let Ok(mut k) = KEYS.lock() {
        *k = [false; 256];
        for &c in codes {
            k[usize::from(c)] = true;
        }
    }
}

/// The default key of each PS2 button (`GameSetting.ini`'s
/// `ffxKeyBinding` defaults, docs/harness.md "Input facts"), as
/// `DirectInput` scan codes.
const BUTTON_KEYS: [(u32, u8); 14] = [
    (0x1, 0xc9),    // L2: Page Up
    (0x2, 0xd1),    // R2: Page Down
    (0x4, 0x1f),    // L1: S
    (0x8, 0x20),    // R1: D
    (0x10, 0x2f),   // Triangle: V
    (0x20, 0x2e),   // Circle: C
    (0x40, 0x2d),   // Cross: X
    (0x80, 0x2c),   // Square: Z
    (0x100, 0x31),  // Select: N
    (0x800, 0x30),  // Start: B
    (0x1000, 0xc8), // Up
    (0x2000, 0xcd), // Right
    (0x4000, 0xd0), // Down
    (0x8000, 0xcb), // Left
];

fn buttons(s: &str) -> Result<u32, String> {
    let mut mask = 0;
    for b in s.split('+').filter(|b| !b.is_empty()) {
        mask |= match b {
            "l2" => 0x1,
            "r2" => 0x2,
            "l1" => 0x4,
            "r1" => 0x8,
            "triangle" => 0x10,
            "circle" => 0x20,
            "cross" => 0x40,
            "square" => 0x80,
            "select" => 0x100,
            "start" => 0x800,
            "up" => 0x1000,
            "right" => 0x2000,
            "down" => 0x4000,
            "left" => 0x8000,
            other => parse(other)?,
        };
    }
    Ok(mask)
}

fn button_keys(mask: u32) -> Vec<u8> {
    BUTTON_KEYS
        .iter()
        .filter(|(b, _)| mask & b != 0)
        .map(|&(_, k)| k)
        .collect()
}

fn parse(s: &str) -> Result<u32, String> {
    let r = match s.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16),
        None => s.parse(),
    };
    r.map_err(|_| format!("bad number {s}"))
}

fn hex(b: &[u8]) -> String {
    use std::fmt::Write as _;
    b.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

fn unhex(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err("odd hex".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "bad hex".to_string()))
        .collect()
}

fn command(shared: &Arc<Shared>, line: &str) -> String {
    match run(shared, line) {
        Ok(r) => r,
        Err(e) => format!("err {e}"),
    }
}

#[allow(clippy::too_many_lines)] // one arm per command
fn run(shared: &Arc<Shared>, line: &str) -> Result<String, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let arg = |i: usize| words.get(i).copied().ok_or("missing argument");
    let mut mem = ArenaMem {
        arena: Arc::clone(shared.arena()),
    };
    let read = |mem: &ArenaMem, at: u32, len: u32| -> Result<String, String> {
        if len > 1 << 20 {
            return Err("too long".into());
        }
        let mut b = vec![0u8; len as usize];
        if !shared.arena().is_mapped(at, b.len()) {
            return Err(format!("{at:#x} is not mapped"));
        }
        mem.read_bytes(at, &mut b);
        Ok(format!("ok {}", hex(&b)))
    };
    match words.first().copied().unwrap_or("") {
        "ping" => Ok("ok pong".into()),
        "frame" => Ok(format!("ok {}", presents())),
        "stats" => Ok(format!(
            "ok frame {} pad_reads 0 key_polls {} presents {} flips {} flips_run {} seeded_jobs 0",
            presents(),
            KEY_POLLS.load(Ordering::Relaxed),
            presents(),
            presents(),
            presents()
        )),
        "wait" => {
            wait_presents(parse(arg(1)?)?);
            Ok(format!("ok {}", presents()))
        }
        "read" | "readabs" => read(&mem, parse(arg(1)?)?, parse(arg(2)?)?),
        "readp" => {
            let base: u32 = mem.read(parse(arg(1)?)?);
            read(&mem, base.wrapping_add(parse(arg(2)?)?), parse(arg(3)?)?)
        }
        "write" | "writep" => {
            let (at, data) = if words[0] == "write" {
                (parse(arg(1)?)?, unhex(arg(2)?)?)
            } else {
                let base: u32 = mem.read(parse(arg(1)?)?);
                (base.wrapping_add(parse(arg(2)?)?), unhex(arg(3)?)?)
            };
            if !shared.arena().is_mapped(at, data.len()) {
                return Err(format!("{at:#x} is not mapped"));
            }
            mem.write_bytes(at, &data);
            Ok("ok".into())
        }
        "keys" => {
            let codes = words[1..]
                .iter()
                .map(|w| u8::from_str_radix(w.trim_start_matches("0x"), 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "bad key")?;
            set_keys(&codes);
            Ok("ok".into())
        }
        "key" | "press" => {
            let codes = if words[0] == "key" {
                arg(1)?
                    .split(',')
                    .map(|w| u8::from_str_radix(w.trim_start_matches("0x"), 16))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| "bad key")?
            } else {
                button_keys(buttons(arg(1)?)?)
            };
            let hold = parse(arg(2)?)?;
            let gap = words.get(3).map_or(Ok(hold), |w| parse(w))?;
            set_keys(&codes);
            wait_presents(hold);
            set_keys(&[]);
            wait_presents(gap);
            Ok(format!("ok {}", presents()))
        }
        "pad" => {
            let mask = words.get(1).map_or(Ok(0), |b| buttons(b))?;
            set_keys(&button_keys(mask));
            Ok("ok".into())
        }
        "shot" => {
            let path = words[1..].join(" ");
            if path.is_empty() {
                return Err("shot <path>".into());
            }
            let (tx, rx) = channel();
            if let Ok(mut s) = SHOT.lock() {
                *s = Some((path, tx));
            }
            rx.recv_timeout(Duration::from_secs(60))
                .map_err(|_| "no Present within 60 s".to_string())
        }
        "gfxcapture" => {
            let dir = arg(1)?;
            crate::gfx::write_shared(shared, std::path::Path::new(dir))
                .map_err(|e| e.to_string())?;
            let frames = shared.gfx.lock().as_ref().map_or(0, |g| g.presents);
            Ok(format!("ok {frames} frames"))
        }
        "quit" => {
            quit();
            Ok("ok".into())
        }
        // the replay player's closing lines: nothing to end
        "trace" | "calllog" if words.get(1) == Some(&"off") => Ok("ok".into()),
        other => Err(format!("{other}: not served by ffx-rust yet")),
    }
}

/// Ends the process at once (the hook's `quit` does the same: the game's
/// own shutdown from the harness thread can deadlock).
fn quit() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
        // SAFETY: ends this process.
        unsafe {
            TerminateProcess(GetCurrentProcess(), 0);
        }
    }
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_map_to_their_default_keys() {
        assert_eq!(button_keys(buttons("start").unwrap()), vec![0x30]);
        assert_eq!(button_keys(buttons("up+circle").unwrap()), vec![0x2e, 0xc8]);
        assert_eq!(buttons("0x40").unwrap(), 0x40);
    }

    #[test]
    fn a_shot_is_a_bottom_up_bmp() {
        let px = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let b = bmp(1, 2, &px);
        assert_eq!(&b[..2], b"BM");
        assert_eq!(b.len(), 54 + 8);
        // the last row first
        assert_eq!(&b[54..58], &[5, 6, 7, 8]);
    }
}
