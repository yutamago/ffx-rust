//! `overlay_probe <out dir> <step>...`: loads save 1 (`FFX_PROBE_SAVE`: another)
//! with the renderer and
//! runs the steps, to try the overlay. Steps: `shot:<name>` (a PNG in the
//! out dir), `wait:<frames>`, `mouse:<x>,<y>[,down|up]` (the pointer
//! there, the left button pressed or released), `click:<x>,<y>` (a left click in the
//! overlay, back buffer pixels), `overlay` (F10), `editor` (F11),
//! `cmd:<harness command>`,
//! `set:<key>=<value>` (a line of `ffx-render.toml` in the game folder,
//! which the renderer rereads; the file is put back at the end).
use std::path::PathBuf;

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    std::fs::create_dir_all(&out)?;
    let options = ffx_harness::Options {
        renderer: true,
        ..ffx_harness::Options::default()
    };
    let game_dir = std::env::var_os("FFX_GAME_DIR").map_or_else(
        || {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("steam-games/FFX-X2-HD")
        },
        PathBuf::from,
    );
    let toml_path = game_dir.join("ffx-render.toml");
    let original = std::fs::read_to_string(&toml_path).ok();
    let mut game = ffx_harness::Game::launch(&options)?;
    // FFX_PROBE_SAVE: another save entry (default 1, the first manual).
    let entry = std::env::var("FFX_PROBE_SAVE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    println!("in {}", ffx_harness::screens::load_save(&mut game, entry)?);
    game.wait(240)?;
    for step in args {
        let (what, arg) = step.split_once(':').unwrap_or((step.as_str(), ""));
        match what {
            "shot" => game
                .screenshot()?
                .save_png(&out.join(format!("{arg}.png")))?,
            "wait" => game.wait(arg.parse().unwrap_or(30))?,
            "click" => {
                let (x, y) = arg.split_once(',').expect("x,y");
                println!("{}", game.command(&format!("overlayclick {x} {y}"))?);
                game.wait(10)?;
            }
            // mouse:x,y[,down|up]: the pointer there (a drag: down, moves,
            // up, with waits between).
            "mouse" => {
                let parts: Vec<&str> = arg.split(',').collect();
                let button = parts.get(2).copied().unwrap_or("");
                println!(
                    "{}",
                    game.command(&format!("overlaymouse {} {} {button}", parts[0], parts[1]))?
                );
                game.wait(3)?;
            }
            "overlay" => {
                println!("{}", game.command("overlay")?);
                game.wait(10)?;
            }
            "editor" => {
                println!("{}", game.command("editor")?);
                game.wait(10)?;
            }
            "cmd" => println!("{}", game.command(arg)?),
            "set" => {
                let (key, _) = arg.split_once('=').expect("key=value");
                let text = std::fs::read_to_string(&toml_path).unwrap_or_default();
                let mut lines: Vec<String> = text
                    .lines()
                    .filter(|l| l.split('=').next().map(str::trim) != Some(key.trim()))
                    .map(str::to_owned)
                    .collect();
                // Plain keys go before the first table.
                let at = lines
                    .iter()
                    .position(|l| l.starts_with('['))
                    .unwrap_or(lines.len());
                lines.insert(at, arg.to_owned());
                std::fs::write(&toml_path, lines.join("\n") + "\n")?;
                game.wait(45)?;
            }
            _ => eprintln!("unknown step {step}"),
        }
    }
    let result = game.quit();
    if let Some(text) = original {
        std::fs::write(&toml_path, text)?;
    }
    result
}
