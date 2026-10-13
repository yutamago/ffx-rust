//! Checks every shipped monster file. Needs `FFX_GAME_DIR`; skipped otherwise.

use std::path::PathBuf;

use ffx_kernel::Table;
use ffx_monster::{MonsterFile, archive_path};
use ffx_vbf::Archive;

#[test]
fn every_monster_file_round_trips_and_matches_the_kernel() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let read = |path: &str| archive.read(archive.find(path).unwrap()).unwrap();
    let kernel: Vec<Table> = (1..=3)
        .map(|n| {
            Table::parse(&read(&format!(
                "ffx_ps2/ffx/master/new_uspc/battle/kernel/monster{n}.bin"
            )))
            .unwrap()
        })
        .collect();

    let mut files = 0;
    let mut matched = 0;
    for id in 0..=999u16 {
        let Some(entry) = archive.find(&archive_path(id)) else {
            continue;
        };
        let original = archive.read(entry).unwrap();
        let file = MonsterFile::parse(original.clone()).unwrap_or_else(|e| panic!("m{id:03}: {e}"));

        let mut copy = file.clone();
        copy.import(&file.export().unwrap()).unwrap();
        assert_eq!(copy.bytes(), &original[..], "m{id:03}: TOML round trip");

        if let Some(record) = kernel.iter().find_map(|t| t.record(id)) {
            assert_eq!(
                &file.stats().unwrap()[20..],
                &record[20..],
                "m{id:03}: stats differ from the kernel"
            );
            matched += 1;
        }
        files += 1;
    }
    assert_eq!(files, 361);
    eprintln!("{files} monster files round-tripped, {matched} match their kernel record");
}
