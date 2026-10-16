//! Boots into a save, waits, and writes a memory snapshot of the game for
//! `ffx-oracle` (`snapshot <windows path> [entry] [frames]`; default entry 1,
//! the first manual save, and 120 frames in the field). Prints the harness's
//! reply and the game thread's x87 control word.
fn main() -> ffx_harness::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: snapshot <windows path> [entry] [frames]");
    let entry = args.next().map_or(1, |e| e.parse().expect("entry"));
    let frames = args.next().map_or(120, |f| f.parse().expect("frames"));
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    println!("{}", ffx_harness::screens::load_save(&mut game, entry)?);
    game.wait(frames)?;
    println!("fpucw {}", game.command("fpucw")?);
    println!("frame {}", game.frame()?);
    println!("snapshot {}", game.command(&format!("snapshot {path}"))?);
    game.quit()
}
