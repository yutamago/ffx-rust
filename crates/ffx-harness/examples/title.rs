//! Boots the game to the title menu and leaves it running.
fn main() -> ffx_harness::Result<()> {
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    ffx_harness::screens::to_title(&mut game)?;
    println!("at the title, frame {}", game.frame()?);
    game.detach();
    Ok(())
}
