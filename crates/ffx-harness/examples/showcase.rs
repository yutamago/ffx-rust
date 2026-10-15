//! `showcase <out dir> <scene>`: screenshots of the external renderer
//! against the original, the same frame both ways. Scenes: `deck` (save 1,
//! also with the overlay open), `battle:<field>` (a battle of that field
//! table entry, started from save 1), `boot:<seconds>` (the opening, that
//! long after start-up). The original look is the renderer with its
//! enhanced path off (`enabled = false`: the game's own shaders, as the
//! game draws them); settings go through `ffx-render.toml`, which the
//! renderer reloads as it changes.
use std::path::{Path, PathBuf};

fn settings_file() -> PathBuf {
    let game = std::env::var_os("FFX_GAME_DIR").map_or_else(
        || {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("steam-games/FFX-X2-HD")
        },
        PathBuf::from,
    );
    game.join("ffx-render.toml")
}

/// The original look, or the renderer's defaults.
fn set_enhanced(on: bool) -> std::io::Result<()> {
    let path = settings_file();
    if on {
        // Without the file the renderer writes and uses its defaults.
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    } else {
        std::fs::write(&path, "enabled = false\n")
    }
}

fn shot(game: &mut ffx_harness::Game, out: &Path, name: &str) -> ffx_harness::Result<()> {
    game.screenshot()?
        .save_png(&out.join(format!("{name}.png")))?;
    println!("{name}: {}", game.command("gfxstats")?);
    Ok(())
}

/// The frame with the original look, then enhanced (the renderer reloads
/// its settings within 30 frames; TAA settles in a few more). Short, so a
/// moving camera moves little.
fn both(game: &mut ffx_harness::Game, out: &Path, name: &str) -> ffx_harness::Result<()> {
    set_enhanced(false)?;
    game.wait(40)?;
    shot(game, out, &format!("{name}-original"))?;
    set_enhanced(true)?;
    game.wait(40)?;
    shot(game, out, &format!("{name}-enhanced"))
}

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    let scene = args.next().unwrap_or_else(|| "deck".into());
    std::fs::create_dir_all(&out)?;
    set_enhanced(true)?;
    let options = ffx_harness::Options {
        renderer: true,
        ..ffx_harness::Options::default()
    };
    let start = std::time::Instant::now();
    let mut game = ffx_harness::Game::launch(&options)?;
    if let Some(secs) = scene.strip_prefix("boot:") {
        let secs: u64 = secs.parse().expect("seconds");
        while start.elapsed().as_secs() < secs {
            game.wait(30)?;
        }
        both(&mut game, &out, &format!("boot{secs}"))?;
        return game.quit();
    }
    println!("in {}", ffx_harness::screens::load_save(&mut game, 1)?);
    game.wait(240)?;
    if let Some(field) = scene.strip_prefix("battle:") {
        let field: u16 = field.parse().expect("field");
        println!(
            "{}",
            ffx_harness::screens::start_battle(&mut game, field, 0, 0)?
        );
        // The battle's opening runs a while: wait until there is a picture.
        let start = std::time::Instant::now();
        loop {
            game.wait(60)?;
            let shot = game.screenshot()?;
            let m = shot.mean(0, 0, shot.width, shot.height);
            if m.iter().sum::<f32>() > 30.0 || start.elapsed().as_secs() > 60 {
                break;
            }
        }
        game.wait(240)?;
        both(&mut game, &out, &format!("battle{field}"))?;
    } else {
        both(&mut game, &out, "deck")?;
        game.command("overlay")?;
        game.wait(30)?;
        shot(&mut game, &out, "deck-overlay")?;
        game.command("overlay")?;
    }
    game.quit()
}
