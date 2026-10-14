//! Times the stages from launch to the Sphere Grid choice.
use std::time::Instant;
fn main() -> ffx_harness::Result<()> {
    let t = Instant::now();
    let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
    println!(
        "{:5.1}s launched, frame {}",
        t.elapsed().as_secs_f32(),
        game.frame()?
    );
    game.wait_for_open("titl00", 10, 30000)?;
    println!(
        "{:5.1}s title event, frame {}",
        t.elapsed().as_secs_f32(),
        game.frame()?
    );
    ffx_harness::screens::to_title(&mut game)?;
    println!(
        "{:5.1}s title menu, frame {}",
        t.elapsed().as_secs_f32(),
        game.frame()?
    );
    game.command("key c8 3 20")?;
    game.command("key 1c 3 20")?;
    ffx_harness::screens::wait_for(
        &mut game,
        ffx_harness::screens::is_sphere_grid_choice,
        30,
        40,
    )?;
    println!(
        "{:5.1}s sphere grid, frame {}",
        t.elapsed().as_secs_f32(),
        game.frame()?
    );
    game.quit()
}
