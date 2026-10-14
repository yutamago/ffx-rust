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
const MESSAGES: &str = "ffx_ps2/ffx/master/new_uspc/event/obj_ps3/zn/znkd0600/znkd0600.bin";
const MODEL: &str = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
const ASSETS: &str = "ffx_data/gamedata/ps3data/chr/pc/c001/c001.ahwin32";
const YUNA_TEXTURE: &str = "PS3Data/chr/pc/c002/tex/c002.dds";
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

/// Tidus 30 % wider, with segment 0 twice over (a resized buffer) and a
/// new segment beside him on a new skeleton matrix and a new material with
/// Yuna's texture (new objects and lists, fixups re-encoded).
fn model_mod() -> Vec<u8> {
    let shipped = original(MODEL);
    // A new segment: a copy of the body (segment 5) bound rigidly to the
    // head joint (hierarchy 35) through a new skeleton matrix, a copy of
    // the head's (skeleton matrix 1).
    let head = ffx_phyre::Model::parse(&shipped).unwrap().skeletons[0].inverse_bind[1];
    let mut edit = ffx_phyre::edit::ClusterEdit::parse(&shipped).unwrap();
    let matrix = edit.add_skeleton_matrix(5, 35, &head).unwrap();
    let new = edit.add_segment(5).unwrap();
    edit.set_palette(new, &[(35, matrix)]).unwrap();
    // Its own material, a copy of the body's with Yuna's texture.
    let (material, index) = edit
        .add_material(edit.segment_material(new).unwrap(), new)
        .unwrap();
    edit.set_texture(material, "TextureSampler", YUNA_TEXTURE)
        .unwrap();
    edit.set_segment_material(new, index).unwrap();
    let original = edit.write().unwrap();
    let mut model = ffx_phyre::Model::parse(&original).unwrap();
    assert_eq!(model.segments.len(), 14);
    let copy = &mut model.segments[new as usize];
    copy.joints.fill([0; 4]);
    copy.weights.fill([1.0, 0.0, 0.0, 0.0]);
    for p in &mut copy.positions {
        p[0] += 8.0;
    }
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
    assert!(modded.len() > shipped.len());
    modded
}

/// Tidus's asset list with Yuna's texture, which the model mod imports.
fn assets_mod() -> Vec<u8> {
    ffx_phyre::assets::add_files(
        &original(ASSETS),
        &[ffx_phyre::assets::texture_entry(YUNA_TEXTURE)],
    )
    .unwrap()
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

/// The first scene's English dialogue with its first line ("Listen to my
/// story.") changed.
fn messages_mod() -> Vec<u8> {
    let archive = archive();
    let read = |path: &str| archive.read(archive.find(path).unwrap()).unwrap();
    let language = ffx_text::Language::English;
    let charset = ffx_text::Charset::from_table(&read(&language.charset_path()), language).unwrap();
    let mut table = ffx_text::messages::MessageTable::parse(&read(MESSAGES)).unwrap();
    table
        .import(
            "[[message]]\nindex = 0\ntext = \"\"\"\n\n{0e:40}Listen to my modded story.{0e:41}\"\"\"\n",
            &charset,
        )
        .unwrap();
    table.write().unwrap()
}

/// Monster m068 (in the salvage ship's first formation) with 12345 HP
/// instead of 50.
fn monster_mod() -> (String, Vec<u8>) {
    let path = ffx_monster::archive_path(68);
    let mut monster = ffx_monster::MonsterFile::parse(original(&path)).unwrap();
    monster.import("[stats]\nhp = 12345\n").unwrap();
    (path, monster.bytes().to_vec())
}

/// The salvage ship's first formation (`cdsp00_00`: m068, m069) with
/// m068, m070, m070.
fn formation_mod() -> (String, Vec<u8>) {
    let path = ffx_battle::formation_path("cdsp00_00");
    let mut file = ffx_battle::FormationFile::parse(original(&path)).unwrap();
    file.import("[formation]\nmonsters = [\"m068\", \"m070\", \"m070\"]\n")
        .unwrap();
    (path, file.bytes().to_vec())
}

/// The encounter table with the salvage ship's group (field 3, cdsp00) at
/// rate 61 and weights 3 and 1 instead of 60, 1 and 1. Also returns where
/// the group starts among the blocks.
fn encounter_mod() -> (Vec<u8>, u32) {
    let mut table =
        ffx_battle::EncounterTable::parse(&original(ffx_battle::ENCOUNTER_TABLE)).unwrap();
    let block = table.fields[3].block;
    let group = &mut table.blocks[block][0];
    assert_eq!((group.rate, group.formations.len()), (60, 2));
    group.rate = 61;
    group.formations[0].weight = 3;
    let at = u32::try_from(table.block_offset(block) + 2).unwrap();
    (table.write(), at)
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
    // A switched-off mod (mod.toml `enabled = false`) that would otherwise
    // win: it sorts after the other and gives the Potion power 99.
    let mut off = ffx_kernel::Table::parse(&original(ITEM)).unwrap();
    off.record_mut(0).unwrap()[0x2a] = 99;
    let off_files = [
        ("mod.toml", b"name = \"off\"\nenabled = false\n".to_vec()),
        (ITEM, off.to_bytes().unwrap()),
    ];
    with_mods("zz-harness-start-up-off", &off_files, || {
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
            assert_eq!(
                memory, modded,
                "the game holds the mod's item table, not the switched-off mod's"
            );

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
    let messages = messages_mod();
    let files = [
        (EVENT, script),
        (MESSAGES, messages.clone()),
        (MODEL, model_mod()),
        (ASSETS, assets_mod()),
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
        // The event's message table: *(*0x01326b28 + 0x34).
        let table = u32_le(game.read_through(0x0132_6b28, 0x34, 4).unwrap());
        assert_eq!(
            game.read_abs(table, messages.len()).unwrap(),
            messages,
            "the dialogue came from the mod"
        );
        let frames = game.frame().unwrap();
        game.wait(600).unwrap();
        assert!(
            game.frame().unwrap() >= frames + 600,
            "the scene keeps running"
        );
        game.events().unwrap();
        for path in [MESSAGES, MODEL, ASSETS, MOTIONS] {
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
fn save_and_battle_with_monster_and_formation_mods() {
    let before = user_game_files();
    let (monster_path, monster) = monster_mod();
    let (formation_path, formation) = formation_mod();
    let (encounters, group_at) = encounter_mod();
    let files = [
        (monster_path.as_str(), monster),
        (formation_path.as_str(), formation),
        (ffx_battle::ENCOUNTER_TABLE, encounters),
    ];
    let (group, ids, max_hp) = with_mods("zz-harness-save", &files, || {
        let mut game = Game::launch(&Options::default()).unwrap();
        // Entry 1: the first manual save (entry 0 is the autosave).
        let event = ffx_harness::screens::load_save(&mut game, 1).unwrap();
        assert!(event.ends_with(".ebp"), "{event}");
        game.wait(120).unwrap();
        // The game's copy of the encounter table (*0x0112a9cc: the blocks).
        let group = game.read_through(0x0112_a9cc, group_at, 9).unwrap();
        // The salvage ship's first formation: field 3 (cdsp00), group 0,
        // formation 0.
        let formation = ffx_harness::screens::start_battle(&mut game, 3, 0, 0).unwrap();
        assert!(formation.contains("cdsp00_00"), "{formation}");
        game.wait(300).unwrap();
        // Monster id (+0x0e) and max HP (+0x594) of every actor; the enemies
        // follow the party.
        let actors = |game: &mut Game, offset, len| -> Vec<Vec<u8>> {
            (0..0x1f)
                .map(|i| ffx_harness::screens::actor(game, i, offset, len).unwrap())
                .collect()
        };
        let ids: Vec<u16> = actors(&mut game, 0x0e, 2)
            .into_iter()
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let hp: Vec<u32> = actors(&mut game, 0x594, 4)
            .into_iter()
            .map(u32_le)
            .collect();
        // The kernel's equipment layout (weapon.bin, shop_arms.bin) in the
        // inventory: what Tidus (0) and Rikku (6) wear sets their battle
        // actors' weapon formula, power and crit bonus.
        for character in [0u32, 6] {
            let worn = game.read(0x0113_205c + character * 0x94 + 0x2d, 2).unwrap();
            let slot = |game: &mut Game, i: u8| {
                game.read(0x0113_0f2c + u32::from(i) * 0x16, 0x16).unwrap()
            };
            let (weapon, armor) = (slot(&mut game, worn[0]), slot(&mut game, worn[1]));
            for (eq, armor_flag) in [(&weapon, 0), (&armor, 1)] {
                assert_eq!(
                    (eq[2] != 0, u32::from(eq[4]), eq[5], u32::from(eq[6])),
                    (true, character, armor_flag, character),
                    "character {character}: in use, character, armor, equipped_by of {eq:02x?}"
                );
            }
            let actor = |game: &mut Game, offset| {
                ffx_harness::screens::actor(game, character, offset, 1).unwrap()[0]
            };
            assert_eq!(
                (
                    actor(&mut game, 0x5c1),
                    actor(&mut game, 0x5c7),
                    actor(&mut game, 0x5d8)
                ),
                (weapon[8], weapon[9], weapon[10] + armor[10]),
                "character {character}: actor formula, power, crit bonus from {weapon:02x?} {armor:02x?}"
            );
        }
        game.screenshot()
            .unwrap()
            .save_png(&screenshot_path("battle"))
            .unwrap();
        game.quit().unwrap();
        (group, ids, hp)
    });
    // n, music, rate, total, then (id, weights): entering the map copied
    // the default weights (low nibble) into the current ones (high nibble).
    assert_eq!(
        group,
        [2, 0x02, 0x04, 61, 4, 0, 0x33, 1, 0x11],
        "the encounter group in memory"
    );
    let monsters: Vec<u16> = ids.iter().copied().filter(|id| id >> 12 == 1).collect();
    assert_eq!(
        monsters,
        [0x1044, 0x1046, 0x1046],
        "the formation's monsters: {ids:x?}"
    );
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
