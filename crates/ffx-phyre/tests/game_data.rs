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

#[test]
fn models_parse_or_are_refused() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let (mut ok, mut refused, mut other) = (0, 0, 0);
    let mut triangles = 0usize;
    let (mut segments, mut with_material, mut with_base_color) = (0, 0, 0);
    let mut samplers: BTreeMap<String, usize> = BTreeMap::new();
    let mut missing: Vec<String> = Vec::new();
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".dae.phyre") && e.size > 0)
    {
        let bytes = archive.read(entry).unwrap();
        match ffx_phyre::Model::parse(&bytes) {
            Ok(model) => {
                for s in &model.segments {
                    assert!(
                        s.positions.iter().flatten().all(|f| f.is_finite()),
                        "{}",
                        entry.path
                    );
                    triangles += s.indices.len() / 3;
                    segments += 1;
                    let material = s.material.map(|m| &model.materials[m]);
                    with_material += usize::from(material.is_some());
                    with_base_color += usize::from(material.and_then(|m| m.base_color()).is_some());
                }
                for t in model.materials.iter().flat_map(|m| &m.textures) {
                    *samplers.entry(t.sampler.clone()).or_default() += 1;
                    if archive.find(&t.archive_path()).is_none() {
                        missing.push(format!("{} -> {}", entry.path, t.path));
                    }
                }
                let glb = model.to_glb();
                assert_eq!(&glb[..4], b"glTF");
                ok += 1;
            }
            Err(Error::Unsupported(why)) => {
                refused += 1;
                *reasons.entry(why).or_default() += 1;
            }
            Err(e) => {
                other += 1;
                *reasons.entry(format!("{e}")).or_default() += 1;
            }
        }
    }
    eprintln!(
        "models: {ok} exported ({triangles} triangles), {refused} refused, {other} not models"
    );
    eprintln!("refused: {reasons:#?}");
    eprintln!(
        "segments: {segments}, with a material {with_material}, with a base color texture {with_base_color}"
    );
    eprintln!("samplers: {samplers:?}");
    assert!(
        missing.is_empty(),
        "texture paths not in the archive: {missing:?}"
    );
    assert_eq!((ok, refused, other), (1266, 48, 0), "{reasons:#?}");
}
