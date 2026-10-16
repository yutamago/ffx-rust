//! Boots the game to the title menu, prints how long it took, and ends it.
fn main() -> ffx_harness::Result<()> {
    let start = std::time::Instant::now();
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    let launched = start.elapsed();
    ffx_harness::screens::to_title(&mut game)?;
    println!(
        "launched in {:.1} s, at the title after {:.1} s (frame {})",
        launched.as_secs_f64(),
        start.elapsed().as_secs_f64(),
        game.frame()?
    );
    Ok(())
}
