//! Converts every shipped texture to DDS and back. Needs `FFX_GAME_DIR`;
//! skipped otherwise.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ffx_phyre::{Error, Texture};
use ffx_vbf::Archive;

#[test]
fn every_texture_round_trips_through_dds() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut big_endian = 0;
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".dds.phyre") && e.size > 0)
    {
        let bytes = archive.read(entry).unwrap();
        let texture = match Texture::parse(bytes.clone()) {
            Ok(t) => t,
            // PS3 leftovers with a big-endian header.
            Err(Error::NotPhyre(magic)) if &magic == b"PHYR" => {
                big_endian += 1;
                continue;
            }
            Err(e) => panic!("{}: {e}", entry.path),
        };
        let dds = texture.to_dds();
        let back = texture
            .with_dds(&dds)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        assert_eq!(back.bytes(), &bytes[..], "{}", entry.path);
        *kinds
            .entry(format!("{:?} {:?}", texture.kind, texture.format))
            .or_default() += 1;
    }
    let total: usize = kinds.values().sum();
    assert_eq!(total, 44_364);
    assert_eq!(big_endian, 30);
    eprintln!("{total} textures round-tripped through DDS: {kinds:?}");
}
