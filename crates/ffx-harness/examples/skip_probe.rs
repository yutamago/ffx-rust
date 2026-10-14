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
        for _ in 0..3 {
            match candidate.strip_prefix("key:") {
                Some(code) => game.key(&[u8::from_str_radix(code, 16).expect("hex")], 8)?,
                None => game.press(&candidate, 8)?,
            }
            game.wait(30)?;
        }
        let title = screens::wait_for(&mut game, screens::is_title, 30, 50);
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
