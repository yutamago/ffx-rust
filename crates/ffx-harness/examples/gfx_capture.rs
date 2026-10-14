//! Captures one frame's Direct3D 11 calls (`gfxcapture`, see `ffx-hook`'s
//! `gfx`) from a save, for studying how the game draws.
//!
//! `gfx_capture <out dir> [entry] [battle]`: boots into save `entry`
//! (default 1, the first manual save), waits for the field, captures into
//! `<out dir>/field`; with `battle`, starts the first battle of the
//! Besaid field table entry 0 too and captures into `<out dir>/battle`.
use std::path::Path;
use std::process::Command;

fn windows_path(p: &Path) -> String {
    let out = Command::new("wslpath")
        .arg("-w")
        .arg(p)
        .output()
        .expect("wslpath");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn capture(game: &mut ffx_harness::Game, dir: &Path) -> ffx_harness::Result<()> {
    std::fs::create_dir_all(dir)?;
    game.screenshot()?.save_png(&dir.join("shot.png"))?;
    let reply = game.command(&format!("gfxcapture {}", windows_path(dir)))?;
    println!("{}: {reply}", dir.display());
    Ok(())
}

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("out dir"));
    let entry = args.next().map_or(1, |e| e.parse().expect("entry"));
    let battle = args.next().is_some_and(|a| a == "battle");
    let options = ffx_harness::Options {
        gfx: Some("1".into()),
        ..ffx_harness::Options::default()
    };
    let mut game = ffx_harness::Game::launch(&options)?;
    let event = ffx_harness::screens::load_save(&mut game, entry)?;
    println!("in {event}");
    game.wait(240)?;
    capture(&mut game, &out.join("field"))?;
    if battle {
        ffx_harness::screens::start_battle(&mut game, 0, 0, 0)?;
        game.wait(400)?;
        capture(&mut game, &out.join("battle"))?;
    }
    game.quit()
}
