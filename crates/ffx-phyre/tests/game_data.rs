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
    let mut not_rewritten: Vec<String> = Vec::new();
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
                let mut imported = model.clone();
                let read = imported
                    .import_glb(&glb)
                    .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
                assert!(!read.is_empty(), "{}: no segments read back", entry.path);
                match imported.write(&bytes) {
                    Ok(w) if w == bytes => {}
                    Ok(w) => not_rewritten.push(format!(
                        "{}: differs at {:?} (sizes {} / {})",
                        entry.path,
                        w.iter().zip(&bytes).position(|(a, b)| a != b),
                        w.len(),
                        bytes.len()
                    )),
                    Err(e) => not_rewritten.push(format!("{}: {e}", entry.path)),
                }
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
    assert!(
        not_rewritten.is_empty(),
        "{} models not written back unchanged: {:#?}",
        not_rewritten.len(),
        &not_rewritten[..not_rewritten.len().min(10)]
    );
}

/// Column-major 4×4 product `a * b`.
fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        let (col, row) = (i / 4, i % 4);
        (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum()
    })
}

/// World matrix of node `i`: its local matrix times its parents'.
fn resolve(
    i: usize,
    nodes: &[ffx_phyre::Node],
    world: &mut [Option<[f32; 16]>],
    depth: usize,
) -> [f32; 16] {
    if let Some(w) = world[i] {
        return w;
    }
    let local = nodes[i].local;
    let w = match nodes[i].parent.filter(|&p| p < nodes.len() && depth < 512) {
        Some(p) => mul(&resolve(p, nodes, world, depth + 1), &local),
        None => local,
    };
    world[i] = Some(w);
    w
}

/// Skeleton matrices are the inverse bind matrices: times the world matrix
/// of the node a bone names, they give the identity.
#[test]
fn skeleton_matrices_invert_the_bind_pose() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let (mut good, mut bad, mut unnamed) = (0usize, 0usize, 0usize);
    let mut worst: Vec<(f32, String)> = Vec::new();
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.contains("/chr/") && e.path.ends_with(".dae.phyre") && e.size > 0)
    {
        let Ok(model) = ffx_phyre::Model::parse(&archive.read(entry).unwrap()) else {
            continue;
        };
        // World matrices of the nodes.
        let mut world: Vec<Option<[f32; 16]>> = vec![None; model.nodes.len()];
        for inst in &model.instances {
            let Some(sk) = inst.skeleton.map(|s| &model.skeletons[s]) else {
                continue;
            };
            for &(s, _) in &inst.segments {
                for b in &model.segments[s].bones {
                    let name = &sk.names[b.hierarchy as usize];
                    let Some(n) = model.nodes.iter().position(|n| &n.name == name) else {
                        unnamed += 1;
                        continue;
                    };
                    let w = resolve(n, &model.nodes, &mut world, 0);
                    let p = mul(&sk.inverse_bind[b.skeleton as usize], &w);
                    let err = (0..16)
                        .map(|i| (p[i] - if i % 5 == 0 { 1.0 } else { 0.0 }).abs())
                        .fold(0f32, f32::max);
                    if err < 1e-2 {
                        good += 1;
                    } else {
                        bad += 1;
                        if worst.len() < 5 {
                            worst.push((err, format!("{} {name}", entry.path)));
                        }
                    }
                }
            }
        }
    }
    eprintln!("bind check: {good} good, {bad} bad, {unnamed} unnamed; worst: {worst:?}");
    // A few models (e.g. the summon s016) store a node pose that is not the
    // bind pose; everywhere else the skeleton matrices invert it.
    assert_eq!(unnamed, 0);
    assert!(good > 48_000 && bad * 50 < good, "{good} good, {bad} bad");
}

#[test]
fn grown_and_shrunk_segments_read_back() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let mut checked = 0;
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".dae.phyre") && e.size > 0)
        .step_by(10)
    {
        let bytes = archive.read(entry).unwrap();
        let Ok(mut model) = ffx_phyre::Model::parse(&bytes) else {
            continue;
        };
        // Segment 0 twice over (a second copy of every vertex and triangle);
        // the last segment cut to its first triangle.
        let seg = &mut model.segments[0];
        let n = seg.positions.len();
        if 2 * n > 0x1_0000 {
            continue;
        }
        let n16 = u16::try_from(n).unwrap();
        for list in [&mut seg.normals, &mut seg.positions] {
            list.extend_from_within(..);
        }
        seg.uvs.extend_from_within(..);
        seg.colors.extend_from_within(..);
        seg.joints.extend_from_within(..);
        seg.weights.extend_from_within(..);
        let more: Vec<u16> = seg.indices.iter().map(|i| i + n16).collect();
        seg.indices.extend(more);
        let last = model.segments.len() - 1;
        if last > 0 {
            let seg = &mut model.segments[last];
            seg.indices.truncate(3);
        }
        let written = model
            .write(&bytes)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        let back =
            ffx_phyre::Model::parse(&written).unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        for (a, b) in model.segments.iter().zip(&back.segments) {
            assert_eq!(a.positions, b.positions, "{}", entry.path);
            assert_eq!(a.normals, b.normals, "{}", entry.path);
            assert_eq!(a.uvs, b.uvs, "{}", entry.path);
            assert_eq!(a.joints, b.joints, "{}", entry.path);
            assert_eq!(a.weights, b.weights, "{}", entry.path);
            assert_eq!(a.indices, b.indices, "{}", entry.path);
        }
        checked += 1;
    }
    eprintln!("{checked} models edited and read back");
    assert!(checked > 100);
}
