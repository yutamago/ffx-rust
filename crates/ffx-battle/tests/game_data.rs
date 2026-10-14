//! Checks the shipped encounter table and every formation file. Needs
//! `FFX_GAME_DIR`; skipped otherwise.

use std::path::PathBuf;

use ffx_battle::{ENCOUNTER_TABLE, EncounterTable, FormationFile, formation_path};
use ffx_vbf::Archive;

fn archive() -> Option<Archive> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap())
}

#[test]
fn the_encounter_table_round_trips_and_names_existing_files() {
    let Some(archive) = archive() else { return };
    let original = archive
        .read(archive.find(ENCOUNTER_TABLE).unwrap())
        .unwrap();
    let table = EncounterTable::parse(&original).unwrap();
    assert_eq!(table.fields.len(), 96);
    assert_eq!(table.write(), original);

    let mut copy = table.clone();
    copy.import(&table.export().unwrap()).unwrap();
    assert_eq!(copy.write(), original, "TOML round trip");

    // Every formation of every field names a shipped file.
    let mut formations = 0;
    for (i, field) in table.fields.iter().enumerate() {
        for (g, group) in table.blocks[field.block].iter().enumerate() {
            for f in 0..group.formations.len() {
                let name = table.formation_file(i, g, f).unwrap();
                assert!(archive.find(&formation_path(&name)).is_some(), "{name}");
                formations += 1;
            }
        }
    }
    let cdsp00 = table
        .fields
        .iter()
        .position(|f| f.name() == "cdsp00")
        .unwrap();
    assert_eq!(
        table.formation_file(cdsp00, 0, 0).as_deref(),
        Some("cdsp00_00")
    );
    eprintln!("{formations} formations, all present");
}

#[test]
fn every_formation_file_round_trips() {
    let Some(archive) = archive() else { return };
    let prefix = "ffx_ps2/ffx/master/jppc/battle/btl/";
    let mut files = 0;
    let mut with_formation = 0;
    for entry in archive.entries() {
        if !(entry.path.starts_with(prefix) && entry.path.ends_with(".bin")) {
            continue;
        }
        let original = archive.read(entry).unwrap();
        let mut file = FormationFile::parse(original.clone())
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        if file.formation().is_some() {
            let text = file.export().unwrap();
            file.import(&text).unwrap();
            assert_eq!(
                file.bytes(),
                &original[..],
                "{}: TOML round trip",
                entry.path
            );
            assert!(
                file.monsters()
                    .unwrap()
                    .iter()
                    .flatten()
                    .all(|id| id >> 12 == 1),
                "{}",
                entry.path
            );
            with_formation += 1;
        }
        files += 1;
    }
    assert_eq!(files, 863);
    eprintln!("{files} formation files, {with_formation} with a formation, round-tripped");
}
