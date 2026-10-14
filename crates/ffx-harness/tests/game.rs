//! Tests against the running game. They start `FFX.exe` on the Windows
//! desktop (in the background, without taking the focus), so they are
//! ignored by default:
//!
//! ```bash
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test -p ffx-harness --test game -- --ignored --test-threads 1
//! ```
//!
//! Screenshots of failures go to the system temp folder (`ffx-harness/`).

use std::path::PathBuf;

use ffx_harness::screens::{cursor_at, is_sphere_grid_choice};
use ffx_harness::{Game, Options};

fn game_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR"))
}

/// Saves a screenshot for a failing check, then panics.
fn fail(game: &mut Game, name: &str, why: &str) -> ! {
    let path = std::env::temp_dir()
        .join("ffx-harness")
        .join(format!("{name}.png"));
    if let Ok(shot) = game.screenshot() {
        let _ = shot.save_png(&path);
    }
    panic!("{why} (screenshot: {})", path.display());
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn runs_in_the_background_and_takes_input() {
    let mut game = Game::launch(&Options::default()).unwrap();
    let start = game.frame().unwrap();
    game.wait(120).unwrap();
    assert!(game.frame().unwrap() >= start + 120);
    let stats = game.command("stats").unwrap();
    let polls: u32 = stats
        .split_whitespace()
        .skip_while(|w| *w != "key_polls")
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    assert!(
        polls > 60,
        "the keyboard is not read in the background: {stats}"
    );
    // Once the game reads the pad (after the logos), the pad word holds
    // exactly the held button.
    game.command("pad cross").unwrap();
    let mut word = 0;
    for _ in 0..30 {
        game.wait(30).unwrap();
        word = u32::from_le_bytes(game.read(0x00cc_b170, 4).unwrap().try_into().unwrap());
        if word != 0 {
            break;
        }
    }
    game.command("pad").unwrap();
    assert_eq!(word, 0x4000, "pad word {word:#x}");
    let shot = game.screenshot().unwrap();
    assert_eq!((shot.width, shot.height), (1280, 720));
    game.quit().unwrap();
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn a_kernel_mod_reaches_game_memory() {
    const ITEM: &str = "ffx_ps2/ffx/master/new_uspc/battle/kernel/item.bin";
    let dir = game_dir();
    let archive = ffx_vbf::Archive::open(dir.join("data/FFX_Data.vbf")).unwrap();
    let original = archive.read(archive.find(ITEM).unwrap()).unwrap();
    let mut table = ffx_kernel::Table::parse(&original).unwrap();
    // Item 0 (Potion): power 4 → 77.
    table.record_mut(0).unwrap()[0x2a] = 77;
    let modded = table.to_bytes().unwrap();
    let mod_dir = dir.join("mods/zz-harness-kernel");
    let file = mod_dir.join(ITEM);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, &modded).unwrap();

    let result = std::panic::catch_unwind(|| {
        let mut game = Game::launch(&Options::default()).unwrap();
        let opened = game.wait_for_open(&format!("mod {ITEM}"), 30, 1200);
        // g_KernelItem (0x0112a940) points at the loaded table.
        let memory = game.read_through(0x0112_a940, 0, modded.len());
        game.quit().unwrap();
        opened.unwrap();
        memory.unwrap()
    });
    std::fs::remove_dir_all(&mod_dir).unwrap();
    let memory = result.unwrap();
    assert_eq!(memory, modded, "the game holds the mod's item table");
    assert_ne!(memory, original);
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop and plays the opening"]
fn new_game_reaches_the_sphere_grid_choice() {
    let mut game = Game::launch(&Options::default()).unwrap();
    if let Err(e) = ffx_harness::screens::new_game(&mut game) {
        fail(&mut game, "new-game", &e.to_string());
    }
    let shot = game.screenshot().unwrap();
    if !is_sphere_grid_choice(&shot) {
        fail(
            &mut game,
            "sphere-grid",
            "no Sphere Grid choice after New Game",
        );
    }
    // A PS2 menu: the pad moves the cursor from Standard to Expert.
    game.wait(30).unwrap();
    let before = game.screenshot().unwrap();
    game.press("down", 6).unwrap();
    game.wait(10).unwrap();
    let after = game.screenshot().unwrap();
    let (standard, expert) = (398, 427);
    if !(cursor_at(&before, standard) > cursor_at(&before, expert) + 60.0
        && cursor_at(&after, expert) > cursor_at(&after, standard) + 60.0)
    {
        fail(
            &mut game,
            "cursor",
            "Down did not move the cursor to Expert",
        );
    }
    game.quit().unwrap();
}
