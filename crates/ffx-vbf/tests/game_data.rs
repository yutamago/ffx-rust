//! Tests against the real archives. They need a game install, located through
//! `FFX_GAME_DIR` (the folder containing `FFX.exe`), and are skipped when it
//! is unset.

use std::path::PathBuf;

use ffx_vbf::Archive;

fn archive_path(name: &str) -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("data").join(name))
}

fn check_all(archive: &Archive) {
    let mut next_offset = None;
    for entry in archive.entries() {
        // Entries are stored in data order, not digest order. Empty files
        // carry a junk offset, so they are skipped.
        if entry.size == 0 {
            assert_eq!(archive.read(entry).unwrap(), Vec::<u8>::new());
            continue;
        }
        if let Some(expected) = next_offset {
            assert!(
                entry.offset >= expected,
                "{} is stored before the previous file",
                entry.path
            );
        }
        assert_eq!(archive.find(&entry.path), Some(entry));
        let data = archive.read(entry).unwrap();
        assert_eq!(data.len() as u64, entry.size, "{}", entry.path);
        next_offset = Some(entry.offset);
    }
}

#[test]
fn reads_every_file_in_metamenu() {
    let Some(path) = archive_path("metamenu.vbf") else {
        return;
    };
    let archive = Archive::open(path).unwrap();
    assert_eq!(archive.entries().len(), 150);
    check_all(&archive);

    let png = archive
        .find("metamenu/ps3data/menucredit/credits_1280.png")
        .unwrap();
    assert!(archive.read(png).unwrap().starts_with(b"\x89PNG\r\n\x1a\n"));
}

/// Decompresses all ~20 GB of `FFX_Data.vbf`; run with `--ignored`.
#[test]
#[ignore = "reads the whole 20 GB archive"]
fn reads_every_file_in_ffx_data() {
    let Some(path) = archive_path("FFX_Data.vbf") else {
        return;
    };
    let archive = Archive::open(path).unwrap();
    assert_eq!(archive.entries().len(), 71_979);
    check_all(&archive);
}
