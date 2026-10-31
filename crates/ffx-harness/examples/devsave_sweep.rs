//! One developer save's coverage session (`tools/devsave-sweep.sh`;
//! `docs/harness.md`, "Developer saves"):
//!
//! ```text
//! devsave_sweep <n> <report file> [snapshot <windows path>]
//! ```
//!
//! Boots the game (seed 1, fast mode, virtual clock; `FFX_HOOK_DISABLE`
//! and the coverage variables come from the environment), loads developer
//! save `n` from the title by the developers' loader (`load devsave`),
//! lets the scene run a fixed number of updates, opens and closes the main
//! menu and, where it is safe, starts one battle of the save's area and
//! lets it run. With `snapshot`, it writes a memory snapshot after the
//! scene's updates instead and stops there (no menu, no battle).
//!
//! The report is `key<TAB>value` lines: `load`, `map`, `scenario`,
//! `script`, `control` (whether the player got it: the scene's confirm
//! presses), `menu`, `battle` (`started <file>`, or `skipped: <why>`),
//! `end` (`ok`, or `crashed: <error>` when the harness stopped answering).
//!
//! A battle is safe (the Debug Revival mod's checks before its own battle
//! starts, `battle.md` "start encounter N" and `maps.md` "Effects") when no
//! battle runs (`0x0112a8e0`), the current ctrl work holds the control
//! (`*g_ScriptContext` bit 0x80), no map change is pending
//! (`g_TkEventJumpRequest` `0x0133084c`), no event effect plays (`0x01133364`)
//! and no magic file is loaded or waiting to unload (`0x00c64cac`,
//! `0x00c64cb0` -1). Its area is the battle field (the field table
//! `*0x0112a9c8`, count `0x0112a9d0`, 14 bytes an entry: the map number
//! at `+0`) whose map number is the one a random encounter looks up
//! (`TkGetNowEventJumpMapNo`, `g_SaveData +4`, which the event's own map
//! id at `+0` is not), else the area's first field (the event name's
//! first four letters: from the agency `kami0100`, `kami00`, as
//! battle-ifrit does); group 0, formation 0 (`screens::start_battle`). An
//! area without encounters gets no battle.

use std::fmt::Write as _;

use ffx_harness::{Error, Game, Options, Result, screens};

/// Updates the scene runs after the load before anything else.
const SCENE_UPDATES: u32 = 900;
/// Presses of confirm (64 updates each) to reach the control at most.
const CONTROL_PRESSES: u32 = 45;

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

/// The battle field of the map number `map`, else the first field of the
/// area (the event name's first four letters, `maca0100` -> `maca00`):
/// `(index, name)`.
fn field_of(game: &mut Game, map: u16, area: &str) -> Result<Option<(u16, String)>> {
    let table = u32_at(&game.read(0x0112_a9c8, 4)?, 0);
    let count = u16_at(&game.read(0x0112_a9d0, 2)?, 0);
    if table == 0 {
        return Ok(None);
    }
    let raw = game.command(&format!("readabs {table:#x} {}", usize::from(count) * 14))?;
    let bytes: Vec<u8> = (0..raw.len() / 2)
        .filter_map(|i| u8::from_str_radix(&raw[2 * i..2 * i + 2], 16).ok())
        .collect();
    let fields: Vec<(u16, u16, String)> = (0..count)
        .filter_map(|i| {
            let o = usize::from(i) * 14;
            let name = bytes.get(o + 6..o + 14)?;
            let end = name.iter().position(|&b| b == 0).unwrap_or(8);
            Some((
                i,
                u16_at(&bytes, o),
                String::from_utf8_lossy(&name[..end]).into_owned(),
            ))
        })
        .collect();
    let exact = fields.iter().find(|f| f.1 == map);
    let near = fields
        .iter()
        .find(|f| area.len() == 4 && f.2.starts_with(area));
    Ok(exact.or(near).map(|f| (f.0, f.2.clone())))
}

/// Why a battle may not start now, or `None` when it may.
fn battle_blocked(game: &mut Game) -> Result<Option<String>> {
    if game.read(0x0112_a8e0, 1)? != [0] {
        return Ok(Some("a battle runs".into()));
    }
    let context = u32_at(&game.read(0x0132_6b28, 4)?, 0);
    if context == 0 {
        return Ok(Some("no event runs".into()));
    }
    let flags = game.command("readp 0x01326b28 0 1")?;
    if u8::from_str_radix(&flags, 16).unwrap_or(0) & 0x80 == 0 {
        return Ok(Some("the player has no control".into()));
    }
    if game.read(0x0133_084c, 4)? != [0; 4] {
        return Ok(Some("a map change is pending".into()));
    }
    if game.read(0x0113_3364, 1)? != [0] {
        return Ok(Some("an event effect plays".into()));
    }
    if game.read(0x00c6_4cac, 4)? != [0xff; 4] || game.read(0x00c6_4cb0, 4)? != [0xff; 4] {
        return Ok(Some("a magic file is loaded".into()));
    }
    Ok(None)
}

/// The session after the load; writes what happened into `report`.
fn session(
    game: &mut Game,
    report: &mut String,
    script: &str,
    snapshot: Option<&str>,
) -> Result<()> {
    game.wait(SCENE_UPDATES)?;
    let save = game.read(0x0112_ca90, 0xd)?;
    let map = u16_at(&save, 0);
    let map_no = u16_at(&save, 4);
    let _ = writeln!(
        report,
        "map_after\t{map} entrance {} map number {map_no}",
        save[0xc]
    );
    if let Some(path) = snapshot {
        let _ = writeln!(
            report,
            "snapshot\t{}",
            game.command(&format!("snapshot {path}"))?
        );
        let _ = writeln!(report, "frame\t{}", game.frame()?);
        return Ok(());
    }
    // Many saves start in a scene: confirm (messages, choices' defaults)
    // once a second until the player has the control (ctrl work 0 `+0`
    // bit 1, which `AtelUcOn` sets), for at most CONTROL_PRESSES presses.
    let mut presses = 0;
    while game.read(0x0132_5ba0, 1)?[0] & 2 == 0 && presses < CONTROL_PRESSES {
        game.command("press circle 4 60")?;
        presses += 1;
    }
    let _ = writeln!(
        report,
        "control\t{} after {presses} presses, frame {}",
        if game.read(0x0132_5ba0, 1)?[0] & 2 == 0 {
            "none"
        } else {
            "yes"
        },
        game.frame()?
    );
    // the main menu: triangle opens it, cross closes it
    game.command("press triangle 8 90")?;
    let mask = game.read(0x0184_08ec, 4)?;
    for _ in 0..4 {
        game.command("press cross 4 40")?;
    }
    game.wait(60)?;
    let _ = writeln!(
        report,
        "menu\tactive mask while open {:#010x}, after {:#010x}",
        u32_at(&mask, 0),
        u32_at(&game.read(0x0184_08ec, 4)?, 0)
    );
    // the event's name: `.../bjyt0200.ebp` -> area `bjyt`
    let area: String = script
        .rsplit('/')
        .next()
        .unwrap_or("")
        .chars()
        .take(4)
        .collect();
    let battle = match (battle_blocked(game)?, field_of(game, map_no, &area)?) {
        (Some(why), _) => format!("skipped: {why}"),
        (None, None) => format!("skipped: map number {map_no} (area {area}) has no battle field"),
        (None, Some((field, name))) => match screens::start_battle(game, field, 0, 0) {
            Ok(file) => {
                let began = game.command("waitfor 0x0112a8e0:1 01 ff 4000");
                // confirm through the commands: attacks, then the second
                // and third command lines (skills or magic, items)
                for _ in 0..6 {
                    game.command("press circle 4 200")?;
                }
                for downs in [1, 2, 3] {
                    for _ in 0..downs {
                        game.command("press down 4 15")?;
                    }
                    for _ in 0..4 {
                        game.command("press circle 4 200")?;
                    }
                }
                for _ in 0..10 {
                    game.command("press circle 4 200")?;
                }
                game.wait(600)?;
                format!(
                    "started field {field} {name} {file} ({}); phase after {:02x?}",
                    began.unwrap_or_else(|e| format!("never under way: {e}")),
                    game.read(0x0112_a8e0, 1)?
                )
            }
            Err(Error::Harness(e)) => format!("refused: field {field}: {e}"),
            Err(e) => return Err(e),
        },
    };
    let _ = writeln!(report, "battle\t{battle}");
    let _ = writeln!(report, "frame\t{}", game.frame()?);
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n: u32 = args
        .first()
        .and_then(|a| a.parse().ok())
        .expect("usage: devsave_sweep <n> <report> [snapshot <windows path>]");
    let out = std::path::PathBuf::from(args.get(1).expect("report file"));
    let snapshot = (args.get(2).map(String::as_str) == Some("snapshot"))
        .then(|| args.get(3).expect("snapshot path").as_str());
    let options = Options {
        virtual_clock: true,
        copy_user_data: false,
        ..Options::default()
    };
    let mut report = format!("save\t{n}\n");
    let mut game = Game::launch(&options)?;
    let end = match screens::load_dev_save(&mut game, n, false) {
        Ok(script) => {
            let _ = writeln!(report, "script\t{script}");
            session(&mut game, &mut report, &script, snapshot)
        }
        Err(e) => Err(e),
    };
    let _ = writeln!(
        report,
        "end\t{}",
        end.as_ref()
            .map_or_else(|e| format!("failed: {e}"), |()| "ok".into())
    );
    let log = game.new_log();
    for line in log.lines().filter(|l| {
        l.contains("devsave")
            || l.contains("overlay-emu")
            || l.contains(" unicorn: ")
            || l.contains(" rust: ")
            || l.contains("FAULT")
            || l.contains("PANIC")
            || l.contains("messagebox")
    }) {
        let _ = writeln!(report, "log\t{line}");
    }
    std::fs::write(&out, &report)?;
    print!("{report}");
    let _ = game.quit();
    end
}
