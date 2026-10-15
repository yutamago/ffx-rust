//! `idle_probe <out dir> <steps> <frames per step>`: boots to the title
//! menu and leaves it alone, a screenshot and the files opened at each step
//! (what the game shows when nobody presses anything).
use std::path::PathBuf;

fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("out dir"));
    let steps: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(60);
    let every: u32 = args.next().and_then(|a| a.parse().ok()).unwrap_or(300);
    std::fs::create_dir_all(&out)?;
    // FFX_LANG (tools/run-game.sh): the game's language.
    let options = ffx_harness::Options::default();
    let mut game = ffx_harness::Game::launch(&options)?;
    ffx_harness::screens::to_title(&mut game)?;
    println!("at the title, frame {}", game.frame()?);
    for k in 0..steps {
        game.wait(every)?;
        let opened = game.events()?;
        game.screenshot()?
            .save_png(&out.join(format!("{k:03}.png")))?;
        println!("{k:03} frame {} {}", game.frame()?, opened.join(" "));
    }
    Ok(())
}
