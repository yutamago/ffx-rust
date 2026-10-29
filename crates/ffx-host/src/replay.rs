//! A replay script (`tools/replays/*.txt`) played in-process at the frame
//! boundary (`ffx-rust --replay FILE`): the harness's commands that drive
//! the game, with its rules (docs/harness.md, "Snapshots and replays").
//! Every boundary is a lockstep hold: the script reads and decides there,
//! and the next update runs only when it is done with this one.
//!
//! | Command | Here |
//! |---|---|
//! | `waitfor <range> <value> [mask] [frames]` | until the bytes match (at most `frames` updates, default 600) |
//! | `shotwhen <range> <value> <mask> <frames> <path>` | as `waitfor`; notes the update the shot shows (no image: the null device) |
//! | `press <buttons> <frames> [gap]` | holds the buttons `frames` updates, then waits `gap` more |
//! | `wait <frames>` | `frames` updates |
//! | `pin` | the time parts of the harness's `pin` (docs/harness.md, "Pinned state") |
//! | `lockstep`, `trace`, `realtime`, `settings`, `audiomark`, `calllog` | nothing (always in lockstep; no trace file) |
//!
//! A range is `addr:len`, `addr+off:len` or `*addr+off:len` (through the
//! pointer at `addr`). A script that fails ends the run with the reason.

use std::sync::atomic::Ordering;

use ffx_bridge::Mem as _;

use crate::clock::{FRAME, VIRTUAL_START};
use crate::exec::{Exec, Stop};

/// The script and where it stands.
#[derive(Debug, Default)]
pub struct Script {
    lines: Vec<String>,
    pos: usize,
    /// The current command's limit or release frame, once started.
    started: Option<Started>,
    /// What each command answered: (line, reply).
    pub replies: Vec<(String, String)>,
    /// The shots: (path, update shown).
    pub shots: Vec<(String, u32)>,
    /// Ends the run when the script is done.
    pub stop_when_done: bool,
    done: bool,
}

#[derive(Debug, Clone, Copy)]
enum Started {
    /// Wait until this frame (`waitfor`/`shotwhen` limit, `wait`).
    Until(u32),
    /// Release the buttons at the first, go on at the second.
    Press(u32, u32),
}

impl Script {
    /// A script from its text.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let lines = text
            .lines()
            .map(|l| l.split('#').next().unwrap_or("").trim().to_owned())
            .filter(|l| !l.is_empty())
            .collect();
        Self {
            lines,
            ..Self::default()
        }
    }

    /// Whether every command ran.
    #[must_use]
    pub fn done(&self) -> bool {
        self.done
    }
}

/// A range of the harness's trace syntax.
#[derive(Debug, Clone, Copy)]
struct Range {
    addr: u32,
    deref: bool,
    off: u32,
    len: u32,
}

fn parse_u32(s: &str) -> Result<u32, String> {
    let r = s
        .strip_prefix("0x")
        .map_or_else(|| s.parse(), |h| u32::from_str_radix(h, 16));
    r.map_err(|e| format!("{s}: {e}"))
}

fn parse_range(spec: &str) -> Result<Range, String> {
    let (at, len) = spec
        .split_once(':')
        .ok_or_else(|| format!("{spec}: want addr:len"))?;
    let len = parse_u32(len)?;
    let (deref, at) = at.strip_prefix('*').map_or((false, at), |a| (true, a));
    let (addr, off) = match at.split_once('+') {
        Some((a, o)) => (parse_u32(a)?, parse_u32(o)?),
        None => (parse_u32(at)?, 0),
    };
    Ok(Range {
        addr,
        deref,
        off,
        len,
    })
}

fn hex_bytes(t: &str) -> Result<Vec<u8>, String> {
    (0..t.len() / 2)
        .map(|i| u8::from_str_radix(&t[2 * i..2 * i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

fn read_range(e: &Exec, r: Range) -> Vec<u8> {
    let mem = e.mem();
    let at = if r.deref {
        mem.read::<u32>(r.addr).wrapping_add(r.off)
    } else {
        r.addr + r.off
    };
    let mut out = vec![0u8; r.len as usize];
    mem.read_bytes(at, &mut out);
    out
}

fn matches(bytes: &[u8], value: &[u8], mask: &[u8]) -> bool {
    bytes
        .iter()
        .zip(value)
        .zip(mask)
        .all(|((b, v), m)| b & m == v & m)
}

/// A PS2 button mask from names joined by `+`, or a hex mask.
fn parse_buttons(s: &str) -> Result<u32, String> {
    if s.starts_with("0x") {
        return parse_u32(s);
    }
    s.split('+').try_fold(0, |mask, name| {
        let bit = match name {
            "l2" => 0x1,
            "r2" => 0x2,
            "l1" => 0x4,
            "r1" => 0x8,
            "triangle" => 0x10,
            "circle" => 0x20,
            "cross" => 0x40,
            "square" => 0x80,
            "select" => 0x100,
            "l3" => 0x200,
            "r3" => 0x400,
            "start" => 0x800,
            "up" => 0x1000,
            "right" => 0x2000,
            "down" => 0x4000,
            "left" => 0x8000,
            other => return Err(format!("unknown button {other:?}")),
        };
        Ok(mask | bit)
    })
}

/// What one step of a command did.
enum Step {
    /// Done with this reply: on to the next command at this boundary.
    Next(String),
    /// Not yet: the next boundary.
    Wait,
}

/// The script's turn at frame boundary `frame` (the main thread, between
/// two updates).
pub fn boundary(e: &mut Exec, frame: u32) {
    loop {
        let shared = std::sync::Arc::clone(e.shared());
        let line = {
            let Ok(s) = shared.clock.script.lock() else {
                return;
            };
            let Some(s) = s.as_ref() else { return };
            if s.done {
                return;
            }
            s.lines.get(s.pos).cloned()
        };
        let Some(line) = line else {
            finish(e, frame, None);
            return;
        };
        match step(e, frame, &line) {
            Ok(Step::Wait) => return,
            Ok(Step::Next(reply)) => {
                crate::log::note(&format!("replay, frame {frame}: {line} -> {reply}"));
                if let Ok(mut s) = e.shared().clock.script.lock()
                    && let Some(s) = s.as_mut()
                {
                    s.replies.push((line, reply));
                    s.pos += 1;
                    s.started = None;
                }
            }
            Err(why) => {
                finish(e, frame, Some(format!("{line}: {why}")));
                return;
            }
        }
    }
}

fn finish(e: &mut Exec, frame: u32, failed: Option<String>) {
    let stop = {
        let Ok(mut s) = e.shared().clock.script.lock() else {
            return;
        };
        let Some(s) = s.as_mut() else { return };
        s.done = true;
        s.stop_when_done
    };
    let what = failed.map_or_else(
        || format!("replay done at frame {frame}"),
        |why| format!("replay failed at frame {frame}: {why}"),
    );
    crate::log::note(&what);
    if stop {
        e.end(Stop::Milestone(what));
    }
}

fn started(e: &Exec) -> Option<Started> {
    e.shared()
        .clock
        .script
        .lock()
        .ok()
        .and_then(|s| s.as_ref().and_then(|s| s.started))
}

fn set_started(e: &Exec, st: Started) {
    if let Ok(mut s) = e.shared().clock.script.lock()
        && let Some(s) = s.as_mut()
    {
        s.started = Some(st);
    }
}

fn step(e: &mut Exec, frame: u32, line: &str) -> Result<Step, String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let arg = |i: usize| words.get(i).copied().ok_or("missing argument");
    match words[0] {
        "waitfor" | "shotwhen" => {
            let shot = words[0] == "shotwhen";
            let r = parse_range(arg(1)?)?;
            let value = hex_bytes(arg(2)?)?;
            let mask = words
                .get(3)
                .map_or_else(|| Ok(vec![0xff; value.len()]), |m| hex_bytes(m))?;
            if value.len() != r.len as usize || mask.len() != r.len as usize {
                return Err("value and mask must be as long as the range".into());
            }
            let frames = words.get(4).map_or(Ok(600), |w| parse_u32(w))?;
            let first = match started(e) {
                Some(Started::Until(limit)) if frame > limit => {
                    return Err(format!(
                        "timed out (the range holds {:02x?})",
                        read_range(e, r)
                    ));
                }
                Some(_) => false,
                None => {
                    set_started(e, Started::Until(frame + frames));
                    true
                }
            };
            if !matches(&read_range(e, r), &value, &mask) {
                return Ok(Step::Wait);
            }
            if shot {
                if first {
                    return Err("matched at its first check (its image may be gone)".into());
                }
                let path = words.get(5..).map(|w| w.join(" ")).unwrap_or_default();
                // the image of the update after which the bytes matched
                let shown = frame.saturating_sub(1);
                if let Ok(mut s) = e.shared().clock.script.lock()
                    && let Some(s) = s.as_mut()
                {
                    s.shots.push((path, shown));
                }
                return Ok(Step::Next(format!("ok update {shown}")));
            }
            Ok(Step::Next(format!("ok frame {frame}")))
        }
        "wait" => {
            let until = match started(e) {
                Some(Started::Until(u)) => u,
                _ => {
                    let u = frame + parse_u32(arg(1)?)?;
                    set_started(e, Started::Until(u));
                    u
                }
            };
            if frame >= until {
                Ok(Step::Next(format!("ok frame {frame}")))
            } else {
                Ok(Step::Wait)
            }
        }
        "press" => {
            let (release, until) = if let Some(Started::Press(r, u)) = started(e) {
                (r, u)
            } else {
                let buttons = parse_buttons(arg(1)?)?;
                let hold = parse_u32(arg(2)?)?;
                let gap = words.get(3).map_or(Ok(hold), |w| parse_u32(w))?;
                e.shared().clock.buttons.store(buttons, Ordering::Release);
                set_started(e, Started::Press(frame + hold, frame + hold + gap));
                (frame + hold, frame + hold + gap)
            };
            if frame >= release {
                e.shared().clock.buttons.store(0, Ordering::Release);
            }
            if frame >= until {
                Ok(Step::Next(format!("ok frame {frame}")))
            } else {
                Ok(Step::Wait)
            }
        }
        "pin" => Ok(Step::Next(format!("ok {}", pin(e)))),
        "lockstep" | "trace" | "realtime" | "settings" | "audiomark" | "calllog" => {
            Ok(Step::Next("ok (nothing to do here)".into()))
        }
        other => Err(format!("{other}: not supported in-process")),
    }
}

/// `Sg_FrameSync`'s carries (`f32`).
const FRAME_SYNC_CARRY_60: u32 = 0x012f_b7f8;
const FRAME_SYNC_CARRY_30: u32 = 0x012f_b7fc;
/// `g_Vu0_R`, `g_Vu0_RSeeded`, and the pinned `R`.
const VU0_R: u32 = 0x00c0_a000;
const VU0_R_SEEDED: u32 = 0x00c8_f728;
const VU0_R_PINNED: u32 = 0x3f80_0000 | 0x0035_a5c3;
/// The title's water frame and draw count.
const TITLE_WATER_FRAME: u32 = 0x00cc_ba09;
const TITLE_WATER_DRAWS: u32 = 0x00cc_ba0a;
/// `g_AppStartTime`, `g_FrameLimiterLast`, `g_AtelPlayTimeLast`.
const APP_START_TIME: u32 = 0x00c9_0278;
const FRAME_LIMITER_LAST: u32 = 0x00cc_9d00;
const ATEL_PLAY_TIME_LAST: u32 = 0x0132_5b84;
/// The clock and the game time the pin sets.
const PINNED_CLOCK: f64 = 4096.0;
const PINNED_GAME_TIME: f64 = 64.0;
/// `rand`'s seed in the CRT's per-thread data.
const HOLDRAND: u32 = 0x14;

/// The harness's `pin` (`crates/ffx-hook/src/harness.rs`,
/// `pin_carried_state`): the time, the shaders' time, `Sg_FrameSync`'s
/// carries, `g_Vu0_R`, the title's water and the main thread's CRT seed.
fn pin(e: &mut Exec) -> String {
    let mut pinned = vec!["frame sync", "vu0 R", "title water"];
    let app = e.shared().clock.app.load(Ordering::Acquire);
    {
        let mut mem = e.mem();
        mem.write(FRAME_SYNC_CARRY_60, 0f32);
        mem.write(FRAME_SYNC_CARRY_30, 0f32);
        mem.write(VU0_R, VU0_R_PINNED);
        mem.write(VU0_R_SEEDED, 1u32);
        mem.write(TITLE_WATER_FRAME, 0u8);
        mem.write(TITLE_WATER_DRAWS, 0u8);
        if app != 0 {
            mem.write(app + 0x28, 0f32);
            let now = VIRTUAL_START + PINNED_CLOCK;
            let start = now - PINNED_GAME_TIME;
            e.shared().clock.set_skipped(PINNED_CLOCK);
            mem.write(APP_START_TIME, start);
            mem.write(FRAME_LIMITER_LAST, now - start);
            #[allow(clippy::cast_possible_truncation)] // the game's f32s
            let game_time = (now - start) as f32;
            mem.write(app + 0x20, game_time);
            mem.write(app + 0x3b0, 0f32);
            mem.write(app + 0x120, 0f32);
            #[allow(clippy::cast_possible_truncation)]
            let next = (now - start + FRAME) as f32;
            mem.write(app + 0x24, next - game_time);
            mem.write(ATEL_PLAY_TIME_LAST, game_time);
            pinned.push("time");
            pinned.push("global time");
        }
    }
    let getptd = e
        .shared()
        .dll("MSVCR110.dll")
        .and_then(|d| d.exports.get("_getptd").copied());
    if let Some(f) = getptd
        && let Ok(ptd) = e.call_cdecl(f, &[])
        && ptd != 0
    {
        e.mem().write(ptd + HOLDRAND, 1u32);
        pinned.push("main CRT seed");
    }
    pinned.join(", ")
}
