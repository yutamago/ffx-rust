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

fn md5_of(path: &std::path::Path) -> [u8; 16] {
    use md5::{Digest, Md5};
    use std::io::Read;
    let mut f = std::fs::File::open(path).unwrap();
    let mut h = Md5::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break h.finalize().into();
        }
        h.update(&buf[..n]);
    }
}

/// An `io::Write` that only hashes.
struct HashWriter(md5::Md5);

impl std::io::Write for HashWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        md5::Digest::update(&mut self.0, buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn repack_is_identity(name: &str) {
    let Some(path) = archive_path(name) else {
        return;
    };
    let archive = Archive::open(&path).unwrap();
    let mut out = HashWriter(md5::Digest::new());
    ffx_vbf::pack(&archive, &std::collections::BTreeMap::new(), &mut out).unwrap();
    let packed: [u8; 16] = md5::Digest::finalize(out.0).into();
    assert_eq!(
        packed,
        md5_of(&path),
        "{name}: repacking without changes must reproduce it"
    );
}

#[test]
fn repacking_metamenu_reproduces_it() {
    repack_is_identity("metamenu.vbf");
}

/// Streams all ~20 GB of `FFX_Data.vbf` twice; run with `--ignored`.
#[test]
#[ignore = "reads the whole 20 GB archive twice"]
fn repacking_ffx_data_reproduces_it() {
    repack_is_identity("FFX_Data.vbf");
}
