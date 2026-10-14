//! Tests against the running game. They start `FFX.exe` on the Windows
//! desktop (in the background, without taking the focus), so they are
//! ignored by default. Each test shares one launch between several checks,
//! because starting and loading cost more than the checks themselves.
//! `tools/game-tests.sh` runs the ones that changed files concern:
//!
//! ```bash
//! tools/game-tests.sh            # tests for what changed since origin/main
//! tools/game-tests.sh --all      # all of them
//! ```
//!
//! Screenshots go to the system temp folder (`ffx-harness/`).

use std::path::PathBuf;

use ffx_harness::screens::{cursor_at, is_sphere_grid_choice};
use ffx_harness::{Game, Options};

fn game_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("FFX_GAME_DIR").expect("FFX_GAME_DIR"))
}

fn archive() -> ffx_vbf::Archive {
    ffx_vbf::Archive::open(game_dir().join("data/FFX_Data.vbf")).unwrap()
}

fn original(path: &str) -> Vec<u8> {
    let archive = archive();
    archive.read(archive.find(path).unwrap()).unwrap()
}

/// Saves a screenshot for a failing check, then panics.
fn fail(game: &mut Game, name: &str, why: &str) -> ! {
    let path = screenshot_path(name);
    if let Ok(shot) = game.screenshot() {
        let _ = shot.save_png(&path);
    }
    panic!("{why} (screenshot: {})", path.display());
}

fn screenshot_path(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join("ffx-harness")
        .join(format!("{name}.png"))
}

/// Writes mod files into `mods/<name>`, runs `test`, and removes the mod
/// again, also when the test fails.
fn with_mods<T>(
    name: &str,
    files: &[(&str, Vec<u8>)],
    test: impl FnOnce() -> T + std::panic::UnwindSafe,
) -> T {
    let dir = game_dir().join("mods").join(name);
    for (path, bytes) in files {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, bytes).unwrap();
    }
    let result = std::panic::catch_unwind(test);
    std::fs::remove_dir_all(&dir).unwrap();
    result.unwrap()
}

fn u32_le(bytes: Vec<u8>) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}

// --- Mods the tests make, each with the archive path it replaces ---

const ITEM: &str = "ffx_ps2/ffx/master/new_uspc/battle/kernel/item.bin";
const EVENT: &str = "ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0600/znkd0600.ebp";
const MODEL: &str = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
const MOTIONS: &str = "ffx_ps2/ffx/master/jppc/chr/pc/c001/mot/resident0.mgrp";

/// The item table with item 0 (Potion) at power 77 instead of 4.
fn kernel_mod() -> Vec<u8> {
    let mut table = ffx_kernel::Table::parse(&original(ITEM)).unwrap();
    table.record_mut(0).unwrap()[0x2a] = 77;
    table.to_bytes().unwrap()
}

/// The first scene's event script with `pushimm 0x1234; popv v1` in front
/// of worker 0's first entry, which runs when the event starts. The two
/// instructions grow the worker past its range, so its code moves to the
/// end of the block. Returns the file and v1's offset in the global area.
fn script_mod() -> (Vec<u8>, u32) {
    let container = ffx_script::Container::parse(original(EVENT)).unwrap();
    let (location, _) = container.script.variable(0, 1).unwrap();
    assert_eq!((location >> 25) & 7, 0, "v1 is a global");
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
    let bytes = container.replace_script(&script).unwrap().bytes().to_vec();
    (bytes, location & 0xff_ffff)
}

/// Tidus 30 % wider, with segment 0 twice over (a resized buffer).
fn model_mod() -> Vec<u8> {
    let original = original(MODEL);
    let mut model = ffx_phyre::Model::parse(&original).unwrap();
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
    modded
}

/// Tidus's resident motions with the root bone turned a quarter turn about
/// y in every clip (the file grows: zero channels become constants).
fn motion_mod() -> Vec<u8> {
    let mut group = ffx_motion::MotionGroup::parse(&original(MOTIONS)).unwrap();
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
    modded
}

/// Monster m068 (in the salvage ship's first formation) with 12345 HP
/// instead of 50.
fn monster_mod() -> (String, Vec<u8>) {
    let path = ffx_monster::archive_path(68);
    let mut monster = ffx_monster::MonsterFile::parse(original(&path)).unwrap();
    monster.import("[stats]\nhp = 12345\n").unwrap();
    (path, monster.bytes().to_vec())
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

// --- The tests ---

/// One launch: background input, a kernel mod in memory, and the way to a
/// new game's first PS2 menu, where the pad moves the cursor.
#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn start_up_input_and_kernel_mod() {
    let modded = kernel_mod();
    let files = [(ITEM, modded.clone())];
    with_mods("zz-harness-start-up", &files, move || {
        let mut game = Game::launch(&Options::default()).unwrap();
        // The keyboard is read without the focus.
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

        // g_KernelItem (0x0112a940) points at the mod's item table.
        game.wait_for_open(&format!("mod {ITEM}"), 30, 1200)
            .unwrap();
        let memory = game.read_through(0x0112_a940, 0, modded.len()).unwrap();
        assert_eq!(memory, modded, "the game holds the mod's item table");

        // New Game to the Sphere Grid choice, a PS2 menu.
        if let Err(e) = ffx_harness::screens::new_game(&mut game) {
            fail(&mut game, "new-game", &e.to_string());
        }
        if !is_sphere_grid_choice(&game.screenshot().unwrap()) {
            fail(
                &mut game,
                "sphere-grid",
                "no Sphere Grid choice after New Game",
            );
        }
        // Down moves the cursor from Standard to Expert.
        game.wait(30).unwrap();
        let before = game.screenshot().unwrap();
        assert_eq!((before.width, before.height), (1280, 720));
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
        // The pad word holds exactly the held button (Circle confirms, so
        // this comes last).
        game.command("pad circle").unwrap();
        game.wait(3).unwrap();
        let word = u32_le(game.read(0x00cc_b170, 4).unwrap());
        game.command("pad").unwrap();
        assert_eq!(word, 0x20, "pad word {word:#x}");
        game.quit().unwrap();
    });
}

/// One launch into the first scene (`znkd0600`) with three mods: a grown
/// event script, a resized model and edited motions (Tidus's). The script's
/// write is read back; the model and motions must be read from the mod and
/// the scene keep running.
#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn first_scene_with_script_model_and_motion_mods() {
    let (script, offset) = script_mod();
    let files = [
        (EVENT, script),
        (MODEL, model_mod()),
        (MOTIONS, motion_mod()),
    ];
    with_mods("zz-harness-first-scene", &files, move || {
        let mut game = Game::launch(&Options::default()).unwrap();
        let opened = ffx_harness::screens::first_scene(&mut game).unwrap();
        assert!(
            opened.starts_with("mod "),
            "the event came from the mod: {opened}"
        );
        game.wait(120).unwrap();
        // Script globals: *(*0x01326b28 + 0x2c) + offset; v1 is a byte.
        let base = u32_le(game.read_through(0x0132_6b28, 0x2c, 4).unwrap());
        let value = game.read_abs(base + offset, 1).unwrap()[0];
        assert_eq!(value, 0x34, "pushimm 0x1234; popv v1 (a byte)");
        let frames = game.frame().unwrap();
        game.wait(600).unwrap();
        assert!(
            game.frame().unwrap() >= frames + 600,
            "the scene keeps running"
        );
        game.events().unwrap();
        for path in [MODEL, MOTIONS] {
            let wanted = format!("mod {path}");
            assert!(
                game.opened().contains(&wanted),
                "the game did not read {path} from the mod"
            );
        }
        game.screenshot()
            .unwrap()
            .save_png(&screenshot_path("first-scene"))
            .unwrap();
        game.quit().unwrap();
    });
}

/// One launch from the user's save (the sandbox copy) into a battle with a
/// monster mod; the user's own files stay byte for byte the same.
#[test]
#[ignore = "starts FFX.exe on the Windows desktop"]
fn save_and_battle_with_monster_mod() {
    let before = user_game_files();
    let (path, modded) = monster_mod();
    let files = [(path.as_str(), modded)];
    let max_hp = with_mods("zz-harness-save", &files, || {
        let mut game = Game::launch(&Options::default()).unwrap();
        // Entry 1: the first manual save (entry 0 is the autosave).
        let event = ffx_harness::screens::load_save(&mut game, 1).unwrap();
        assert!(event.ends_with(".ebp"), "{event}");
        game.wait(120).unwrap();
        // The salvage ship's first formation: field 3 (cdsp00), group 0,
        // formation 0, with m068.
        let formation = ffx_harness::screens::start_battle(&mut game, 3, 0, 0).unwrap();
        assert!(formation.contains("cdsp00_00"), "{formation}");
        game.wait(300).unwrap();
        // Max HP (+0x594) of every actor; the enemies follow the party.
        let hp: Vec<u32> = (0..0x1f)
            .map(|i| u32_le(ffx_harness::screens::actor(&mut game, i, 0x594, 4).unwrap()))
            .collect();
        game.screenshot()
            .unwrap()
            .save_png(&screenshot_path("battle"))
            .unwrap();
        game.quit().unwrap();
        hp
    });
    assert!(
        max_hp.contains(&12345),
        "no actor with the mod's HP: {max_hp:?}"
    );
    assert_eq!(
        user_game_files(),
        before,
        "the user's settings and saves are unchanged"
    );
}

/// Three launches from the save: seed 1 twice gives the same generators,
/// seed 2 others.
#[test]
#[ignore = "starts FFX.exe on the Windows desktop three times"]
fn random_generators_repeat_with_a_seed() {
    // The 68 generators at 0x01135ee0 and the global one at 0x00c42200.
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
