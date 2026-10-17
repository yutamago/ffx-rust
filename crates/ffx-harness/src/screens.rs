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
    // An earlier `wait_for_open` (for another file) may have read the
    // event already: the opening can start before or after that file.
    if !game.opened().iter().any(|e| e.contains("titl00")) {
        game.wait_for_open("titl00", 30, 3000)?;
    }
    // Fast mode ends the logos at once (the hook clears their movie's
    // flag); without it they last a while in real time, so keep tapping
    // for up to two minutes.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    while std::time::Instant::now() < deadline {
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

/// From boot into the first scene of a new game (event `znkd0600`, the
/// Zanarkand ruins): Standard Sphere Grid, Yes, Arranged soundtrack.
/// Returns the event script's open event.
///
/// # Errors
/// Fails when a screen or the event does not appear.
pub fn first_scene(game: &mut Game) -> Result<String> {
    new_game(game)?;
    game.events()?;
    for _ in 0..4 {
        game.command("press circle 3 40")?;
    }
    game.wait_for_open("znkd0600.ebp", 30, 1200)
}

/// The save list: a pale blue-violet screen with the "Select save data"
/// help bar.
#[must_use]
pub fn is_save_list(shot: &Image) -> bool {
    let [r, g, b] = shot.mean(200, 400, 880, 250);
    b > 120.0 && r > 70.0 && b > r + 20.0 && b > g + 20.0
}

/// From boot into the game from save entry `entry` of the load list (0 is
/// the first; with an autosave, it is entry 0 and the manual saves follow).
/// The saves are the sandbox copies (`Options::home`). Returns the event
/// script the save starts in.
///
/// # Errors
/// Fails when a screen or the event does not appear.
pub fn load_save(game: &mut Game, entry: u32) -> Result<String> {
    to_title(game)?;
    // The title's cursor starts on Load; the Flash menus read keys.
    game.command("key 1c 3 30")?;
    wait_for(game, is_save_list, 30, 20)?;
    // The list is drawn a little before it takes keys; a Down sent at once
    // was sometimes lost and the autosave loaded instead.
    game.wait(30)?;
    for _ in 0..entry {
        game.command("key d0 3 20")?;
    }
    game.events()?;
    // Pick the entry, then Yes.
    game.command("key 1c 3 40")?;
    game.command("key 1c 3 40")?;
    game.wait_for_open(".ebp", 30, 1200)
}

/// From boot into the game from a save file, without the title menu: once
/// the title scene is set up, the `loadsave` harness command runs what the
/// Load menu runs once a save is picked (`docs/formats/save.md`). `save` is
/// a slot (the file `ffx_<slot>` of the sandbox saves, `Options::home`) or a
/// Windows path to a save file, which is only read. Returns the event
/// script the save starts in.
///
/// The title event's script (`test20.ebp`) opens while the boot's jump to
/// the title (map `0x17`) is still pending (`g_TkEventJumpRequest`,
/// `0x0133084c`); a load sent then is undone: the scene step enters the
/// title map (`titl00`) and the title script's set-up starts a new game's
/// state. So this waits for `titl00`'s map to open, the request to clear
/// and 30 frames for the script's set-up (about 70 frames from boot).
///
/// # Errors
/// Fails when the save does not load or its event does not open.
pub fn load_save_direct(game: &mut Game, save: &str) -> Result<String> {
    const TITLE_MAP: &str = "titl00/bin/mapout.vpa";
    game.events()?;
    if !game.opened().iter().any(|e| e.contains(TITLE_MAP)) {
        game.wait_for_open(TITLE_MAP, 2, 3000)?;
    }
    game.command("waitfor 0x0133084c:4 00000000 ffffffff 600")?;
    game.wait(30)?;
    let mut tries = 0;
    let reply = loop {
        match game.command(&format!("loadsave {save}")) {
            Ok(r) => break r,
            // The save data manager is made a little after the first event.
            Err(Error::Harness(e)) if e.contains("no save data manager") && tries < 100 => {
                tries += 1;
                game.wait(5)?;
            }
            Err(e) => return Err(e),
        }
    };
    let opened = game.wait_for_open(".ebp", 5, 1200)?;
    if opened.contains("test20.ebp") {
        return Err(Error::Harness(format!(
            "loadsave {save} ({reply}): the title event opened again"
        )));
    }
    Ok(opened)
}

/// Starts a battle from the field, the way a random encounter does
/// (`MsBattleEncountExe`, `0x00780d10`): the battle field (an index into the field
/// table at `*0x0112a9c8`, 14 bytes an entry with the map name at +6), the
/// encounter group and the formation in it go to `0x0112c256`, `…58` and
/// `…59`; the encounter counters are reset (`0x00780ff0`) and the request
/// flag `0x0112a8e2` is raised. `MsBattleProcess` (`0x00790c10`) then loads
/// the battle on the next frame. Waits until the formation's file opens.
///
/// # Errors
/// Fails when the battle does not load.
pub fn start_battle(game: &mut Game, field: u16, group: u8, formation: u8) -> Result<String> {
    game.events()?;
    game.command(&format!(
        "write 0x112c256 {:02x}{:02x}",
        field & 0xff,
        field >> 8
    ))?;
    game.command(&format!("write 0x112c258 {group:02x}{formation:02x}"))?;
    game.command("call 0x780ff0 1")?;
    game.command("write 0x112a8e2 01")?;
    game.wait_for_open("battle/btl/", 10, 600)
}

/// The battle actor `index` (`MsGetChr`, `0x00794020`): actors below
/// 0x1f are `0xf90` bytes from `*0x011334d4`.
///
/// # Errors
/// See [`Game::read_abs`].
pub fn actor(game: &mut Game, index: u32, offset: u32, len: usize) -> Result<Vec<u8>> {
    let base = u32::from_le_bytes(
        game.read(0x0113_34d4, 4)?
            .try_into()
            .map_err(|_| Error::Harness("pointer".into()))?,
    );
    game.read_abs(base + index * 0xf90 + offset, len)
}
