//! Boots the game to the Sphere Grid choice of a new game and leaves it
//! running, for exploring with `tools/harness.py`.
fn main() -> ffx_harness::Result<()> {
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    ffx_harness::screens::new_game(&mut game)?;
    println!("at the Sphere Grid choice, frame {}", game.frame()?);
    game.detach();
    Ok(())
}
