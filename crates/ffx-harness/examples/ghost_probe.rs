//! `ghost_probe <out dir> none|original|enhanced [setting lines]`: loads
//! save 1, walks left then right, and takes screenshots while walking, to
//! look for double images. `none`: the hook without the renderer;
//! `original`: the renderer with its enhanced path off; `enhanced`: its
//! defaults. Further arguments are `ffx-render.toml` lines (`key = value`).
use std::path::PathBuf;

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    let mode = args.next().unwrap_or_else(|| "original".into());
    let extra: Vec<String> = args.collect();
    std::fs::create_dir_all(&out)?;
    let game_dir = std::env::var_os("FFX_GAME_DIR").map_or_else(
        || {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("steam-games/FFX-X2-HD")
        },
        PathBuf::from,
    );
    let mut toml = String::new();
    if mode == "original" {
        toml.push_str("enabled = false\n");
    }
    for line in &extra {
        toml.push_str(line);
        toml.push('\n');
    }
    std::fs::write(game_dir.join("ffx-render.toml"), toml)?;
    let options = ffx_harness::Options {
        renderer: mode != "none",
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    println!("in {}", ffx_harness::screens::load_save(&mut game, 1)?);
    game.wait(240)?;
    let shot = |game: &mut ffx_harness::Game, name: &str| -> ffx_harness::Result<()> {
        game.screenshot()?
            .save_png(&out.join(format!("{name}.png")))?;
        Ok(())
    };
    shot(&mut game, "still")?;
    for dir in ["left", "right"] {
        game.command(&format!("pad {dir}"))?;
        game.wait(40)?;
        shot(&mut game, &format!("{dir}-a"))?;
        game.wait(3)?;
        shot(&mut game, &format!("{dir}-b"))?;
        game.command("pad")?;
        game.wait(60)?;
    }
    println!("{}", game.command("gfxstats").unwrap_or_default());
    game.quit()
}
