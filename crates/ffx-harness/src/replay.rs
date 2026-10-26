//! Replays: a scripted session played the same way twice, once on the
//! original code and once with the Rust replacements, each recording a
//! per-frame trace of memory hashes; the two traces must match
//! (`docs/rewrite-plan.md`, replay parity).
//!
//! A script is one command per line (`#` starts a comment):
//!
//! | Line | Does |
//! |---|---|
//! | `load N [script]` | boots and loads save entry `N` of the title's list (`screens::load_save`) |
//! | `load direct SAVE [script] [lockstep]` | boots into save slot `SAVE` (`ffx_<SAVE>`) or a Windows path, without the title menu (`screens::load_save_direct`); with `lockstep`, lockstep from the load's boundary on (`screens::load_save_direct_lockstep`) |
//! | `battle FIELD GROUP FORMATION` | starts a battle (`screens::start_battle`) |
//! | `trace RANGE...` | starts tracing these ranges (`addr:len`, `*addr+off:len`; see the `trace` harness command) |
//! | `realtime` | read before launch ([`realtime`]): the game runs at its own frame rate, not in fast mode; skipped here |
//! | `settings NAME` | read before launch ([`settings`]): `tools/sandbox/NAME.ini` merged over the sandbox's `GameSetting.ini`; skipped here |
//! | anything else | sent to the harness as it is (`press circle 4`, `wait 60`, ...) |
//!
//! `$OUT` in a line is the trace's folder (a Windows path), so a run's
//! screenshots (`shot $OUT\field-walk-1.bmp`) land next to its trace and
//! runs side by side do not overwrite each other's.
//!
//! The game runs seeded (seed 1) and in fast mode, as [`crate::Options`]
//! defaults to, so the same inputs land on the same frames.

use crate::{Game, Result, screens};

/// Plays `script` on a running game, tracing to `trace` (a Windows path)
/// once the script says `trace`. Returns the harness replies, one per line.
///
/// # Errors
/// When a command fails.
pub fn play(game: &mut Game, script: &str, trace: &str) -> Result<Vec<String>> {
    let mut replies = Vec::new();
    let out = trace.rsplit_once(['\\', '/']).map_or(".", |(dir, _)| dir);
    for raw in script.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        let line = &line.replace("$OUT", out);
        let line = line.as_str();
        if line.is_empty() {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let reply = match words[0] {
            "realtime" | "settings" => continue,
            "load" => {
                // `load <entry> [script]` or `load direct <save> [script]`:
                // fails when another scene starts (a lost key press loads a
                // different save).
                let (opened, want) = if words.get(1) == Some(&"direct") {
                    let save = words.get(2).copied().unwrap_or("1");
                    // `load direct SAVE [script] lockstep`: lockstep from
                    // the load on (screens::load_save_direct_lockstep)
                    let lockstep = words.len() > 3 && words.last() == Some(&"lockstep");
                    let want = words.get(3).filter(|w| **w != "lockstep");
                    let opened = if lockstep {
                        screens::load_save_direct_lockstep(game, save)?
                    } else {
                        screens::load_save_direct(game, save)?
                    };
                    (opened, want)
                } else {
                    (
                        screens::load_save(
                            game,
                            words.get(1).map_or(Ok(1), |w| w.parse()).unwrap_or(1),
                        )?,
                        words.get(2),
                    )
                };
                if let Some(want) = want
                    && !opened.contains(want)
                {
                    return Err(crate::Error::Harness(format!(
                        "load: {opened} started, not {want}"
                    )));
                }
                opened
            }
            "battle" => {
                let n = |i: usize| {
                    words
                        .get(i)
                        .and_then(|w| w.parse::<u16>().ok())
                        .unwrap_or(0)
                };
                #[allow(clippy::cast_possible_truncation)] // group and formation are bytes
                screens::start_battle(game, n(1), n(2) as u8, n(3) as u8)?
            }
            "trace" => game.command(&format!("trace {trace} {}", words[1..].join(" ")))?,
            // a timed-out wait says which line and what the bytes were
            "waitfor" | "shotwhen" => game.command(line).map_err(|e| match e {
                crate::Error::Harness(why) => {
                    let now = words
                        .get(1)
                        .and_then(|r| r.split_once(':'))
                        .filter(|(at, _)| !at.contains(['*', '+']))
                        .and_then(|(at, len)| game.command(&format!("read {at} {len}")).ok())
                        .map_or_else(String::new, |b| format!(" (the range holds {b})"));
                    crate::Error::Harness(format!("{line}: {why}{now}"))
                }
                e => e,
            })?,
            _ => game.command(line)?,
        };
        replies.push(format!("{line} -> {reply}"));
    }
    game.command("trace off")?;
    game.command("calllog off")?;
    Ok(replies)
}

/// Whether `script` asks to run at the game's own frame rate (a
/// `realtime` line): no fast mode, no virtual clock, no lockstep. The
/// opening (map `0x15d`) needs it: its scene keeps time with the music
/// stream, which plays in real time, so in fast mode the same frame shows
/// another moment of it in every run (`tools/replays/boot.txt`).
#[must_use]
pub fn realtime(script: &str) -> bool {
    script
        .lines()
        .any(|l| l.split('#').next().unwrap_or("").trim() == "realtime")
}

/// The settings file a script asks for (a `settings NAME` line): the game
/// starts with `tools/sandbox/NAME.ini` merged over the sandbox's
/// `GameSetting.ini` ([`crate::Options::settings`]). The last such line
/// wins.
#[must_use]
pub fn settings(script: &str) -> Option<String> {
    script
        .lines()
        .filter_map(|l| {
            let mut words = l.split('#').next().unwrap_or("").split_whitespace();
            if words.next() == Some("settings") {
                words.next().map(str::to_owned)
            } else {
                None
            }
        })
        .next_back()
}

/// The first frame where two traces differ: `(frame, range index)`, or
/// `None` when they agree on every frame both have.
#[must_use]
pub fn first_difference(a: &str, b: &str) -> Option<(u32, usize)> {
    for (la, lb) in a.lines().zip(b.lines()) {
        let (fa, ha) = la.split_once(' ').unwrap_or((la, ""));
        let (_, hb) = lb.split_once(' ').unwrap_or((lb, ""));
        if ha != hb {
            let frame = fa.parse().unwrap_or(0);
            let index = ha
                .split(' ')
                .zip(hb.split(' '))
                .position(|(x, y)| x != y)
                .unwrap_or(0);
            return Some((frame, index));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_line() {
        assert_eq!(settings("load direct 1\nwait 3\n"), None);
        assert_eq!(
            settings("# settings x\nsettings hdao # HDAO on\nwait 3\n").as_deref(),
            Some("hdao")
        );
    }

    #[test]
    fn differences() {
        let a = "10 aa bb\n11 aa bc\n";
        assert_eq!(first_difference(a, a), None);
        assert_eq!(first_difference(a, "10 aa bb\n11 aa bd\n"), Some((11, 1)));
    }
}
