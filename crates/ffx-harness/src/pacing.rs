//! Load pacing: how many frames a magic file's textures take to load, with
//! every frame slowed down, so that the count follows the game's own
//! per-frame pacing and not how fast the machine is (`docs/harness.md`,
//! "Load pacing").
//!
//! The script is `tools/replays/battle-overdrive.txt` up to Yuna's summon
//! command (Valefor, magic `0x253`). From there the game advances one
//! update at a time in lockstep, and the main thread is held at each frame
//! boundary for [`Options::slow`] before the next update: every thread
//! that runs in real time (the asset loader) has finished whatever it can
//! do without the main thread. The count is the number of updates from the
//! frame the magic ID is set (the load request) to the frame the battle
//! effect read (`g_BtlEffectReadClear`, counted up by `Sg_EffectReadPoll`
//! each frame) is back at -1 (load done).

use std::time::Duration;

use crate::{Error, Game, Result, replay};

/// The replay the script comes from.
pub const SCRIPT: &str = "tools/replays/battle-overdrive.txt";
/// The line the prefix stops before: the summon's magic ID.
const STOP: &str = "waitfor 0x00c64ca0:4 53020000";
/// `g_MagicFile_Handle` - 0xc: the magic ID being loaded or played.
const MAGIC_ID: u32 = 0x00c6_4ca0;
/// Valefor's entry.
const VALEFOR: u32 = 0x253;
/// `g_BtlEffectReadClear`: -1 when no battle effect read runs.
const EFFECT_READ: u32 = 0x00c4_21e8;
/// `g_AsyncLoadManager`: `+0x20` and `+0x2c` count the two request lists.
const ASYNC_LOAD_MANAGER: u32 = 0x00cc_c88c;
/// `g_PhyreScene`: `+0x58` VFX texture slots, `+0x60` their states.
const PHYRE_SCENE: u32 = 0x00cc_b9d8;

/// How to measure.
#[derive(Debug, Clone)]
pub struct Options {
    /// How long the main thread is held at each frame boundary.
    pub slow: Duration,
    /// At most this many frames from the request to the end of the load.
    pub limit: u32,
    /// Also log every thread's stack (the harness's `stacks`) at each
    /// frame the loader is busy.
    pub stacks: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            slow: Duration::from_millis(250),
            limit: 400,
            stacks: false,
        }
    }
}

/// One measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pacing {
    /// The frame boundary at which the magic ID was first set.
    pub request: u32,
    /// Updates from the request until the effect read ended.
    pub frames: u32,
    /// Updates from the request during which the loader's request lists
    /// were not empty (`AsyncLoadManager_isAllRequestDone` false).
    pub loader_busy: u32,
    /// One line per frame from the request: the frame, the effect read,
    /// whether the loader is busy, and the scene's VFX texture slots
    /// (`g_PhyreScene` `+0x58`) with how many of them are ready (`+0x60`
    /// entries 1).
    pub log: Vec<String>,
}

/// The battle-overdrive script up to Yuna's summon command (no shots, no
/// trace).
///
/// # Errors
/// When the script cannot be read or has no summon line.
pub fn prefix(repo: &std::path::Path) -> Result<String> {
    let script = std::fs::read_to_string(repo.join(SCRIPT))?;
    let mut out = String::new();
    for line in script.lines() {
        let code = line.split('#').next().unwrap_or("").trim();
        if code.starts_with(STOP) {
            return Ok(out);
        }
        if code.starts_with("shot") || code.starts_with("trace") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    Err(Error::Harness(format!("{SCRIPT} has no `{STOP}` line")))
}

fn word(bytes: &[u8]) -> Result<u32> {
    bytes
        .get(..4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| Error::Harness("short read".into()))
}

/// Plays the prefix on a launched game and measures the summon's load.
///
/// # Errors
/// When a command fails, the prefix fails, or the load does not end within
/// [`Options::limit`] frames.
pub fn measure(game: &mut Game, repo: &std::path::Path, options: &Options) -> Result<Pacing> {
    let script = prefix(repo)?;
    replay::play(game, &script, "pacing.trace")?;
    let mut request = None;
    let mut started = false;
    let mut loader_busy = 0;
    let mut log = Vec::new();
    let mut step_ms = 0u128;
    for _ in 0..=options.limit + 600 {
        std::thread::sleep(options.slow);
        let frame = game.frame()?;
        let magic = word(&game.read(MAGIC_ID, 4)?)?;
        let read = word(&game.read(EFFECT_READ, 4)?)?;
        let lists = game.read_through(ASYNC_LOAD_MANAGER, 0x20, 0x10)?;
        let busy = word(&lists)? != 0 || word(&lists[0xc..])? != 0;
        if request.is_none() && magic == VALEFOR {
            request = Some(frame);
        }
        if let Some(at) = request {
            if busy {
                loader_busy += 1;
            }
            let slots = word(&game.read_through(PHYRE_SCENE, 0x58, 4)?)?;
            let states = word(&game.read_through(PHYRE_SCENE, 0x60, 4)?)?;
            let ready = if slots > 0 && slots < 0x400 && states != 0 {
                game.read_abs(states, slots as usize * 4)?
                    .chunks(4)
                    .filter(|c| c == &[1, 0, 0, 0])
                    .count()
            } else {
                0
            };
            log.push(format!(
                "{frame} read {read:08x} busy {} slots {slots} ready {ready} step {step_ms} ms",
                u8::from(busy)
            ));
            if options.stacks && busy {
                // the render lock (0x00c94ee4): owner thread, depth
                let lock = game.read(0x00c9_4ef4, 8)?;
                log.push(format!(
                    "{frame} renderlock owner {} depth {}",
                    word(&lock)?,
                    word(&lock[4..])?
                ));
                log.push(format!("{frame} stacks {}", game.command("stacks")?));
            }
            if read != u32::MAX {
                started = true;
            } else if started {
                return Ok(Pacing {
                    request: at,
                    frames: frame - at,
                    loader_busy,
                    log,
                });
            }
            if frame - at > options.limit {
                break;
            }
        }
        let t = std::time::Instant::now();
        game.wait(1)?;
        step_ms = t.elapsed().as_millis();
    }
    Err(Error::Harness(match request {
        None => "the summon's magic ID was never set".into(),
        Some(at) => format!("the load requested at frame {at} did not end"),
    }))
}
