//! Which input skips the opening? For each candidate (PS2 buttons, or
//! `key:<scan code>`), boots the game, presses it during the opening, and
//! prints how many frames the title menu took.
//!
//! `cargo run --release -p ffx-harness --example skip_probe -- circle start key:1c`
use ffx_harness::{Game, Options, screens};

fn main() -> ffx_harness::Result<()> {
    for candidate in std::env::args().skip(1) {
        let mut game = Game::launch(&Options::default())?;
        game.wait_for_open("titl00", 30, 3000)?;
        game.wait(120)?;
        let start = game.frame()?;
        // Hold it for a second every two seconds until the title shows.
        let mut title = Err(ffx_harness::Error::Harness("no title".into()));
        for _ in 0..25 {
            match candidate.strip_prefix("key:") {
                Some(code) => game.key(&[u8::from_str_radix(code, 16).expect("hex")], 60)?,
                None => game.press(&candidate, 60)?,
            }
            if screens::is_title(&game.screenshot()?) {
                title = Ok(());
                break;
            }
        }
        let frames = game.frame()? - start;
        println!(
            "{candidate}: {}",
            if title.is_ok() {
                format!("title after {frames} frames")
            } else {
                format!("no title after {frames} frames")
            }
        );
        game.quit()?;
    }
    Ok(())
}
