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
    // FFX_CAPTURE_DRAWS="first last": render target 0 after each of those.
    let range = std::env::var("FFX_CAPTURE_DRAWS").unwrap_or_default();
    let reply = game.command(&format!("gfxcapture {} {range}", windows_path(dir)))?;
    println!("{}: {reply}", dir.display());
    Ok(())
}

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().expect("out dir"));
    let first = args.next();
    // `boot <frames>`: the opening, that many frames after start-up.
    if first.as_deref() == Some("boot") {
        // Seconds after start-up, comma-separated: a capture at each (the
        // boot screens and the opening run in real time).
        let marks: Vec<u64> = args
            .next()
            .unwrap_or_else(|| "40".into())
            .split(',')
            .map(|f| f.parse().expect("seconds"))
            .collect();
        let start = std::time::Instant::now();
        let options = ffx_harness::Options {
            gfx: Some("1".into()),
            ..ffx_harness::Options::default()
        };
        let mut game = ffx_harness::Game::launch(&options)?;
        for m in marks {
            while start.elapsed().as_secs() < m {
                game.wait(30)?;
            }
            capture(&mut game, &out.join(format!("boot{m}s")))?;
        }
        return game.quit();
    }
    let entry = first.map_or(1, |e| e.parse().expect("entry"));
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
    // FFX_CAPTURE_MORE=n: n more field captures, 15 frames apart.
    let more: u32 = std::env::var("FFX_CAPTURE_MORE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for i in 0..more {
        game.wait(15)?;
        capture(&mut game, &out.join(format!("field{}", i + 2)))?;
    }
    if battle {
        ffx_harness::screens::start_battle(&mut game, 3, 0, 0)?;
        game.wait(400)?;
        capture(&mut game, &out.join("battle"))?;
    }
    game.quit()
}
