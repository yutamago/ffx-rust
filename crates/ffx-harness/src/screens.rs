//! Recognising screens and getting to them. Screens are told apart by the
//! mean colours of fixed rectangles of the 1280×720 frame.

use crate::{Error, Game, Image, Result};

/// The title menu: its "New Game" bar is blue, and the opening scene
/// behind it is gone.
#[must_use]
pub fn is_title(shot: &Image) -> bool {
    let [r, g, b] = shot.mean(830, 365, 400, 30);
    b > 90.0 && b > r + 40.0 && b > g + 10.0
}

/// The Sphere Grid choice after New Game: black around a purple box.
#[must_use]
pub fn is_sphere_grid_choice(shot: &Image) -> bool {
    let [r, g, b] = shot.mean(460, 330, 360, 120);
    let corner = shot.mean(0, 600, 300, 100);
    corner.iter().all(|&c| c < 10.0) && b > 60.0 && r > 30.0 && b > g + 20.0
}

/// Brightness of the cursor area left of the Sphere Grid option whose text
/// is centred at `y` (Standard 398, Expert 427).
#[must_use]
pub fn cursor_at(shot: &Image, y: usize) -> f32 {
    shot.mean(476, y - 12, 50, 24).iter().sum()
}

/// Polls every `step` frames, at most `tries` times, until `screen` shows.
///
/// # Errors
/// Fails when it does not.
pub fn wait_for(
    game: &mut Game,
    screen: fn(&Image) -> bool,
    step: u32,
    tries: u32,
) -> Result<Image> {
    for _ in 0..tries {
        let shot = game.screenshot()?;
        if screen(&shot) {
            return Ok(shot);
        }
        game.wait(step)?;
    }
    Err(Error::Harness("the screen did not appear".into()))
}

/// From boot to the title menu: waits for the opening (the title event
/// opens `titl00` when it starts), then taps Start (B) every second until
/// the title shows. Taps are a few frames: menus repeat held directions.
///
/// # Errors
/// Fails when the title does not appear.
pub fn to_title(game: &mut Game) -> Result<Image> {
    game.wait_for_open("titl00", 30, 3000)?;
    for _ in 0..40 {
        let shot = game.screenshot()?;
        if is_title(&shot) {
            return Ok(shot);
        }
        game.command("press start 3 60")?;
    }
    wait_for(game, is_title, 60, 60)
}

/// From boot to the Sphere Grid choice of a new game.
///
/// # Errors
/// Fails when a screen does not appear.
pub fn new_game(game: &mut Game) -> Result<Image> {
    to_title(game)?;
    // The Flash title menu reads keys: Up to New Game, Enter.
    game.command("key c8 3 20")?;
    game.command("key 1c 3 20")?;
    wait_for(game, is_sphere_grid_choice, 30, 20)
}
