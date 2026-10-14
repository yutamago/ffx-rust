//! Drives the app's commands against the game: every editor reads, and
//! edits land in a temporary mod (removed afterwards). Needs
//! `FFX_GAME_DIR`; skipped otherwise.

use ffx_studio::Studio;
use serde_json::{Value, json};

#[test]
#[allow(clippy::too_many_lines)] // one walk through every editor
fn every_editor_reads_and_saves_into_a_mod() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let config = std::env::temp_dir().join("ffx-studio-test/config.json");
    let _ = std::fs::remove_file(&config);
    let mut studio = Studio::new(config, None);
    let mut call = |cmd: &str, args: Value| -> Value {
        studio
            .call(cmd, &args)
            .unwrap_or_else(|e| panic!("{cmd}: {e:#}"))
    };
    let status = call("set_game_dir", json!({"path": dir.to_str().unwrap()}));
    assert_eq!(status["game_ok"], true);
    let status = call("mod_create", json!({"name": "zz studio test"}));
    let id = status["active_mod"].as_str().unwrap().to_owned();
    let mod_dir = std::path::PathBuf::from(&dir).join("mods").join(&id);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Items: the Potion.
        let rows = call("table_rows", json!({"table": "item"}));
        assert_eq!(rows["rows"][0]["label"], "Potion");
        let rec = call("table_record", json!({"table": "item", "id": 0}));
        assert_eq!(rec["values"]["power"], 4);
        let saved = call(
            "table_save",
            json!({"table": "item", "id": 0, "values": {"name": "Big Potion", "power": 9}}),
        );
        assert_eq!(saved["record"]["values"]["name"], "Big Potion");
        assert_eq!(
            call("table_rows", json!({"table": "item"}))["rows"][0]["label"],
            "Big Potion"
        );
        // Every table reads.
        for e in call("editors", json!({})).as_array().unwrap() {
            if e["kind"] == "table" {
                let table = e["table"].as_str().unwrap();
                let rows = call("table_rows", json!({"table": table}));
                let first = rows["rows"][0]["id"].clone();
                call("table_record", json!({"table": table, "id": first}));
            }
        }
        // Treasure shows what a chest gives.
        let takara = call("table_rows", json!({"table": "takara"}));
        assert!(
            takara["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["label"].as_str().unwrap().contains("Potion"))
        );
        // Monsters: m068's HP and name.
        let rec = call("monster_record", json!({"id": 68}));
        assert_eq!(rec["values"]["name"], "Piranha");
        let saved = call(
            "monster_save",
            json!({"id": 68, "values": {"stats.hp": 777, "name": "Big Piranha"}}),
        );
        assert_eq!(saved["record"]["values"]["stats.hp"], 777);
        assert_eq!(saved["record"]["values"]["name"], "Big Piranha");
        // Encounters: the salvage ship's group and first formation.
        let field = call("encounter_field", json!({"field": 3}));
        assert_eq!(field["groups"][0]["formations"][0]["file"], "cdsp00_00");
        let field = call(
            "encounter_save_group",
            json!({"field": 3, "group": 0, "rate": 70, "weights": [3, 1]}),
        );
        assert_eq!(field["groups"][0]["rate"], 70);
        assert_eq!(field["groups"][0]["formations"][0]["chance"], 75.0);
        let field = call(
            "formation_save",
            json!({"field": 3, "file": "cdsp00_00", "monsters": [0x1044, 0x1046, 0x1046]}),
        );
        assert_eq!(
            field["groups"][0]["formations"][0]["monsters"][2]["id"],
            0x1046
        );
        // Dialogue: the opening's first line.
        let list = call("dialogue_list", json!({}));
        assert!(
            list["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == "znkd0600")
        );
        let d = call("dialogue_get", json!({"name": "znkd0600"}));
        assert!(
            d["messages"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Listen to my story")
        );
        let d = call(
            "dialogue_save",
            json!({"name": "znkd0600", "messages": [{"index": 0, "text": "\n\n{0e:40}Listen to my edited story.{0e:41}"}]}),
        );
        assert!(
            d["messages"][0]["text"]
                .as_str()
                .unwrap()
                .contains("edited")
        );
        // Scripts disassemble with native names.
        let s = call(
            "script_get",
            json!({"path": "ffx_ps2/ffx/master/jppc/event/obj/zn/znkd0600/znkd0600.ebp"}),
        );
        assert!(s["source"].as_str().unwrap().contains("Native_"));
        // Files and models list.
        let dirs = call("file_list", json!({"prefix": ""}))["dirs"]
            .as_array()
            .unwrap()
            .len();
        let models = call("model_list", json!({"filter": "c001"}))["rows"]
            .as_array()
            .unwrap()
            .len();
        assert!(dirs > 0 && models > 0, "{dirs} folders, {models} models");
        // The mod holds what was saved.
        let files = call("mod_files", json!({"id": id}));
        let files: Vec<&str> = files
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect();
        for want in [
            "item.bin",
            "m068.bin",
            "monster1.bin",
            "btl.bin",
            "cdsp00_00.bin",
            "znkd0600.bin",
        ] {
            assert!(
                files.iter().any(|f| f.ends_with(want)),
                "{want} in {files:?}"
            );
        }
        // Switching the mod off is written for the hook.
        call("mod_update", json!({"id": id, "enabled": false}));
        let meta = std::fs::read_to_string(mod_dir.join("mod.toml")).unwrap();
        assert!(meta.contains("enabled = false"), "{meta}");
    }));
    std::fs::remove_dir_all(&mod_dir).unwrap();
    result.unwrap();
}
