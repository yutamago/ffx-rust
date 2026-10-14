//! Boots the game into a save (`load_save [entry]`, default 1: the first
//! manual save after the autosave) and leaves it running.
fn main() -> ffx_harness::Result<()> {
    let entry = std::env::args()
        .nth(1)
        .map_or(1, |e| e.parse().expect("entry"));
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    println!("{}", ffx_harness::screens::load_save(&mut game, entry)?);
    game.detach();
    Ok(())
}
