//! Captures Direct3D 11 frames (`gfxcapture`, see `ffx-hook`'s `gfx`)
//! from a save, for studying how the game draws and as references
//! (`docs/renderer.md`, "Reference captures").
//!
//! `gfx_capture <out dir> [--hash] [--frames N] [entry] [battle]`: boots
//! into save `entry` (default 1, the first manual save), waits for the
//! field, captures into `<out dir>/field`; with `battle`, starts the first
//! battle of the Besaid field table entry 0 too and captures into
//! `<out dir>/battle`. `--hash` writes content hashes instead of contents
//! (a few MB instead of a few hundred), `--frames N` captures N frames in
//! a row. Keep captures out of the repository (`~/ffx-work/m4/captures`).
//! Pinned captures for comparing runs come from the `draw-field` replay
//! (`tools/replays/draw-field.txt`), not from here.
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

/// Bytes under `dir`.
fn size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .map(|e| {
                let p = e.path();
                if p.is_dir() {
                    size(&p)
                } else {
                    e.metadata().map_or(0, |m| m.len())
                }
            })
            .sum()
    })
}

fn capture(game: &mut ffx_harness::Game, dir: &Path, how: &str) -> ffx_harness::Result<()> {
    std::fs::create_dir_all(dir)?;
    game.screenshot()?.save_png(&dir.join("shot.png"))?;
    // FFX_CAPTURE_DRAWS="first last": render target 0 after each of those.
    let range = std::env::var("FFX_CAPTURE_DRAWS")
        .ok()
        .filter(|r| !r.trim().is_empty())
        .map_or_else(String::new, |r| format!(" draws {r}"));
    let start = std::time::Instant::now();
    let reply = game.command(&format!("gfxcapture {}{how}{range}", windows_path(dir)))?;
    println!(
        "{}: {reply} ({:.1} s, {:.1} MB)",
        dir.display(),
        start.elapsed().as_secs_f64(),
        f64::from(u32::try_from(size(dir) / 1000).unwrap_or(u32::MAX)) / 1e3
    );
    Ok(())
}

fn main() -> ffx_harness::Result<()> {
    let mut args = Vec::new();
    let mut how = String::new();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--hash" => how.push_str(" hash"),
            "--frames" => {
                how.push_str(" frames ");
                how.push_str(&it.next().expect("frames"));
            }
            _ => args.push(a),
        }
    }
    let mut args = args.into_iter();
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
            capture(&mut game, &out.join(format!("boot{m}s")), &how)?;
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
    capture(&mut game, &out.join("field"), &how)?;
    // FFX_CAPTURE_MORE=n: n more field captures, 15 frames apart.
    let more: u32 = std::env::var("FFX_CAPTURE_MORE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    for i in 0..more {
        game.wait(15)?;
        capture(&mut game, &out.join(format!("field{}", i + 2)), &how)?;
    }
    if battle {
        ffx_harness::screens::start_battle(&mut game, 3, 0, 0)?;
        game.wait(400)?;
        capture(&mut game, &out.join("battle"), &how)?;
    }
    game.quit()
}
