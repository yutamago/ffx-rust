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
    let mut word = 0;
    for _ in 0..30 {
        game.wait(30).unwrap();
        game.command("pad circle").unwrap();
        game.wait(2).unwrap();
        word = u32::from_le_bytes(game.read(0x00cc_b170, 4).unwrap().try_into().unwrap());
        game.command("pad").unwrap();
        if word != 0 {
            break;
        }
    }
    assert_eq!(word, 0x20, "pad word {word:#x}");
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
    game.command("press down 3 20").unwrap();
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

/// Writes `bytes` as a mod file at `path` in `mods/<name>`, runs `test`,
/// and removes the mod again.
fn with_mod<T>(
    name: &str,
    path: &str,
    bytes: &[u8],
    test: impl FnOnce() -> T + std::panic::UnwindSafe,
) -> T {
    let dir = game_dir().join("mods").join(name);
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, bytes).unwrap();
    let result = std::panic::catch_unwind(test);
    std::fs::remove_dir_all(&dir).unwrap();
    result.unwrap()
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn a_grown_event_script_runs_in_the_game() {
    const EVENT: &str = "ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0600/znkd0600.ebp";
    let archive = ffx_vbf::Archive::open(game_dir().join("data/FFX_Data.vbf")).unwrap();
    let original = archive.read(archive.find(EVENT).unwrap()).unwrap();
    let container = ffx_script::Container::parse(original.clone()).unwrap();
    // Worker 0's first entry runs when the event starts. Two instructions
    // in front of it grow the worker past its range, so its code moves to
    // the end of the block. v1 is a byte in the global area at 0xa3c.
    let (location, _) = container.script.variable(0, 1).unwrap();
    assert_eq!((location >> 25) & 7, 0, "v1 is a global");
    let offset = location & 0xff_ffff;
    let text = ffx_script::disassemble(&container.script).unwrap();
    let edited = text.replacen(
        "worker 0 type 1\ne0:\n",
        "worker 0 type 1\ne0:\n    pushimm 4660\n    popv v1\n",
        1,
    );
    assert_ne!(edited, text);
    let script = ffx_script::assemble(&container.script, &edited).unwrap();
    assert!(
        script.workers()[0].range != container.script.workers()[0].range,
        "the edit grows worker 0"
    );
    let modded = container.replace_script(&script).unwrap().bytes().to_vec();

    let value = with_mod("zz-harness-script", EVENT, &modded, move || {
        let mut game = Game::launch(&Options::default()).unwrap();
        let opened = ffx_harness::screens::first_scene(&mut game).unwrap();
        assert!(
            opened.starts_with("mod "),
            "the event came from the mod: {opened}"
        );
        game.wait(120).unwrap();
        // Global variables: *(*0x01326b28 + 0x2c) + offset.
        let base = u32::from_le_bytes(
            game.read_through(0x0132_6b28, 0x2c, 4)
                .unwrap()
                .try_into()
                .unwrap(),
        );
        let value = game.read_abs(base + offset, 1).unwrap()[0];
        let frames = game.frame().unwrap();
        game.wait(300).unwrap();
        assert!(
            game.frame().unwrap() >= frames + 300,
            "the game keeps running"
        );
        game.quit().unwrap();
        value
    });
    assert_eq!(value, 0x34, "pushimm 0x1234; popv v1 (a byte)");
}

/// Plays into the first scene with a mod file in place and checks that the
/// game read it and keeps running for ten seconds. Saves a screenshot.
fn first_scene_with(name: &str, path: &'static str, bytes: &[u8]) {
    with_mod(name, path, bytes, move || {
        let mut game = Game::launch(&Options::default()).unwrap();
        ffx_harness::screens::first_scene(&mut game).unwrap();
        let frames = game.frame().unwrap();
        game.wait(600).unwrap();
        game.events().unwrap();
        let wanted = format!("mod {path}");
        assert!(
            game.opened().contains(&wanted),
            "the game did not read {path} from the mod"
        );
        assert!(
            game.frame().unwrap() >= frames + 600,
            "the game keeps running"
        );
        let shot = game.screenshot().unwrap();
        let file = std::env::temp_dir()
            .join("ffx-harness")
            .join(format!("{name}.png"));
        shot.save_png(&file).unwrap();
        eprintln!("screenshot: {}", file.display());
        game.quit().unwrap();
    });
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn a_resized_model_loads_in_the_game() {
    const MODEL: &str = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
    let archive = ffx_vbf::Archive::open(game_dir().join("data/FFX_Data.vbf")).unwrap();
    let original = archive.read(archive.find(MODEL).unwrap()).unwrap();
    let mut model = ffx_phyre::Model::parse(&original).unwrap();
    // Every vertex 30 % wider; segment 0 twice over (a resized buffer).
    for seg in &mut model.segments {
        for p in &mut seg.positions {
            p[0] *= 1.3;
        }
    }
    let seg = &mut model.segments[0];
    let n = u16::try_from(seg.positions.len()).unwrap();
    seg.positions.extend_from_within(..);
    seg.normals.extend_from_within(..);
    seg.uvs.extend_from_within(..);
    seg.colors.extend_from_within(..);
    seg.joints.extend_from_within(..);
    seg.weights.extend_from_within(..);
    let more: Vec<u16> = seg.indices.iter().map(|i| i + n).collect();
    seg.indices.extend(more);
    let modded = model.write(&original).unwrap();
    assert!(modded.len() > original.len());
    first_scene_with("zz-harness-model", MODEL, &modded);
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn an_edited_motion_group_loads_in_the_game() {
    const MOTIONS: &str = "ffx_ps2/ffx/master/jppc/chr/pc/c001/mot/resident0.mgrp";
    let archive = ffx_vbf::Archive::open(game_dir().join("data/FFX_Data.vbf")).unwrap();
    let original = archive.read(archive.find(MOTIONS).unwrap()).unwrap();
    let mut group = ffx_motion::MotionGroup::parse(&original).unwrap();
    // Every clip turns Tidus's root bone a quarter turn about y.
    for clip in group.sequences.iter_mut().flat_map(|s| &mut s.clips) {
        let ry = &mut clip.clip.channels[1];
        *ry = match std::mem::replace(ry, ffx_motion::Channel::Zero) {
            ffx_motion::Channel::Keys(k) => {
                ffx_motion::Channel::Keys(k.into_iter().map(|v| v + 1024).collect())
            }
            other => ffx_motion::Channel::Constant(other.at(0) + 1024),
        };
    }
    let modded = group.write().unwrap();
    assert_eq!(ffx_motion::MotionGroup::parse(&modded).unwrap(), group);
    first_scene_with("zz-harness-motion", MOTIONS, &modded);
}

/// The user's own settings and saves, by path.
fn user_game_files() -> Vec<(PathBuf, Vec<u8>)> {
    let root = ffx_harness::user_documents()
        .unwrap()
        .join("Square Enix/FINAL FANTASY X&X-2 HD Remaster");
    let mut out = Vec::new();
    let mut dirs = vec![root];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                out.push((path, bytes));
            }
        }
    }
    out.sort();
    out
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn a_save_loads_from_the_sandbox_and_the_users_files_stay_untouched() {
    let before = user_game_files();
    let mut game = Game::launch(&Options::default()).unwrap();
    // Entry 1: the first manual save (entry 0 is the autosave).
    let event = ffx_harness::screens::load_save(&mut game, 1).unwrap();
    assert!(event.ends_with(".ebp"), "{event}");
    let frames = game.frame().unwrap();
    game.wait(300).unwrap();
    assert!(game.frame().unwrap() >= frames + 300);
    let shot = game.screenshot().unwrap();
    let file = std::env::temp_dir().join("ffx-harness/loaded-save.png");
    shot.save_png(&file).unwrap();
    eprintln!("{event}; screenshot: {}", file.display());
    game.quit().unwrap();
    assert_eq!(
        user_game_files(),
        before,
        "the user's settings and saves are unchanged"
    );
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop three times"]
fn the_random_generators_repeat_with_a_seed() {
    // The 68 generators at 0x01135ee0 and the global one at 0x00c42200,
    // once the save has loaded.
    let state = |seed: u32| {
        let options = Options {
            seed: Some(seed),
            ..Options::default()
        };
        let mut game = Game::launch(&options).unwrap();
        ffx_harness::screens::load_save(&mut game, 1).unwrap();
        game.wait(120).unwrap();
        let mut state = game.read(0x0113_5ee0, 68 * 4).unwrap();
        state.extend(game.read(0x00c4_2200, 4).unwrap());
        game.quit().unwrap();
        state
    };
    let (a, b, c) = (state(1), state(1), state(2));
    assert_eq!(a, b, "the same seed gives the same generators");
    assert_ne!(a, c, "another seed gives other generators");
}

#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn a_monster_mod_reaches_the_battle() {
    // The salvage ship's first formation (field 3, cdsp00; group 0,
    // formation 0) has m068. Its HP becomes 12345.
    let path = ffx_monster::archive_path(68);
    let archive = ffx_vbf::Archive::open(game_dir().join("data/FFX_Data.vbf")).unwrap();
    let mut monster =
        ffx_monster::MonsterFile::parse(archive.read(archive.find(&path).unwrap()).unwrap())
            .unwrap();
    monster.import("[stats]\nhp = 12345\n").unwrap();
    let modded = monster.bytes().to_vec();
    let path: &'static str = Box::leak(path.into_boxed_str());
    let max_hp = with_mod("zz-harness-monster", path, &modded, move || {
        let mut game = Game::launch(&Options::default()).unwrap();
        ffx_harness::screens::load_save(&mut game, 1).unwrap();
        game.wait(120).unwrap();
        let formation = ffx_harness::screens::start_battle(&mut game, 3, 0, 0).unwrap();
        assert!(formation.contains("cdsp00_00"), "{formation}");
        game.wait(300).unwrap();
        // Max HP (+0x594) of every actor; the enemies follow the party.
        let hp: Vec<u32> = (0..0x1f)
            .map(|i| {
                let b = ffx_harness::screens::actor(&mut game, i, 0x594, 4).unwrap();
                u32::from_le_bytes(b.try_into().unwrap())
            })
            .collect();
        let shot = game.screenshot().unwrap();
        shot.save_png(&std::env::temp_dir().join("ffx-harness/monster-battle.png"))
            .unwrap();
        game.quit().unwrap();
        hp
    });
    assert!(
        max_hp.contains(&12345),
        "no actor with the mod's HP: {max_hp:?}"
    );
}
