//! Boots into a save, waits, and writes a memory snapshot of the game for
//! `ffx-oracle`: `snapshot <windows path> [entry] [frames] [battle FIELD
//! GROUP FORMATION]`. Defaults: entry 1 (the first manual save), 120 frames
//! in the field; with `battle`, the battle is started first and the frames
//! are counted from there. Prints the harness's replies and the game
//! thread's x87 control word.
fn main() -> ffx_harness::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .expect("usage: snapshot <windows path> [entry] [frames] [battle F G N]");
    let entry = args.get(1).map_or(1, |e| e.parse().expect("entry"));
    let frames = args.get(2).map_or(120, |f| f.parse().expect("frames"));
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    println!("{}", ffx_harness::screens::load_save(&mut game, entry)?);
    if args.get(3).map(String::as_str) == Some("battle") {
        let n = |i: usize| args.get(i).map_or(0, |v| v.parse::<u16>().expect("number"));
        #[allow(clippy::cast_possible_truncation)] // group and formation are bytes
        let opened = ffx_harness::screens::start_battle(&mut game, n(4), n(5) as u8, n(6) as u8)?;
        println!("{opened}");
    }
    game.wait(frames)?;
    println!("fpucw {}", game.command("fpucw")?);
    println!("frame {}", game.frame()?);
    println!("snapshot {}", game.command(&format!("snapshot {path}"))?);
    game.quit()
}
