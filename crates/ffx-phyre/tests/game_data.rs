//! Converts every shipped texture to DDS and back. Needs `FFX_GAME_DIR`;
//! skipped otherwise.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ffx_phyre::{Error, Format, Kind, Texture};
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
        // The writer for new sizes and formats gives the same file for the
        // same image.
        let same = texture
            .with_any_dds(&dds)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        assert_eq!(same.bytes(), &bytes[..], "{}", entry.path);
        *kinds
            .entry(format!("{:?} {:?}", texture.kind, texture.format))
            .or_default() += 1;
    }
    let total: usize = kinds.values().sum();
    assert_eq!(total, 44_364);
    assert_eq!(big_endian, 30);
    eprintln!("{total} textures round-tripped through DDS: {kinds:?}");
}

/// Every 20th texture at twice its size in another format, with a full mip
/// chain and with one level: it parses back with that size, format and mip
/// count, and writing the original image into it gives the original file
/// again (so the fields the writer sets follow the shipped conventions for
/// one-level and full-chain textures, word 20 included).
#[test]
fn every_twentieth_texture_resizes_and_changes_format() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let mut done = 0;
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".dds.phyre") && e.size > 0)
        .step_by(20)
    {
        let Ok(texture) = Texture::parse(archive.read(entry).unwrap()) else {
            continue;
        };
        let (w, h) = (texture.width * 2, texture.height * 2);
        // The top level at twice the size (each pixel four times), on every
        // face.
        let doubled: Vec<u8> = texture
            .to_rgba()
            .chunks_exact(texture.width as usize * 4)
            .flat_map(|r| {
                let wide: Vec<u8> = r
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .flat_map(|p| [*p, *p].concat())
                    .collect();
                [wide.clone(), wide].concat()
            })
            .collect();
        let rgba = doubled.repeat(texture.kind.faces());
        let format = if texture.format == Format::Argb8 {
            Format::Rgba8
        } else {
            Format::Argb8
        };
        // One-level cube maps are refused (no shipped cube map has one
        // level); they get two.
        let one = if texture.kind == Kind::CubeMap { 2 } else { 1 };
        for levels in [ffx_phyre::full_mip_levels(w, h), one] {
            let big = texture
                .with_rgba(w, h, &rgba, format, levels)
                .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
            let back = Texture::parse(big.bytes().to_vec()).unwrap();
            assert_eq!(
                (
                    back.kind,
                    back.format,
                    back.width,
                    back.height,
                    back.mip_levels
                ),
                (texture.kind, format, w, h, levels),
                "{}",
                entry.path
            );
            assert!(back.to_rgba() == doubled, "{}", entry.path);
            let restored = big
                .with_any_dds(&texture.to_dds())
                .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
            if restored.bytes() != texture.bytes() {
                // Word 20 of the few tiny block textures whose chain is over
                // twice their top level: the writer leaves room for it. The
                // rest of the file is the original.
                let tiny = ffx_phyre::chain_size(
                    texture.format,
                    texture.width,
                    texture.height,
                    texture.mip_levels,
                ) > 2 * texture.format.level_size(texture.width, texture.height);
                assert!(tiny, "{} restored differs", entry.path);
                let mask = |b: &[u8]| [&b[..0x50], &b[0x54..]].concat();
                assert!(
                    mask(restored.bytes()) == mask(texture.bytes()),
                    "{} restored differs outside word 20",
                    entry.path
                );
            }
        }
        done += 1;
    }
    assert!(done > 2000, "{done}");
    eprintln!("{done} textures resized and restored");
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
        // Further sets: shifted, so that each set is told apart.
        for (k, set) in seg.extra_uvs.iter_mut().enumerate() {
            let shifted: Vec<[f32; 2]> = set
                .iter()
                .map(|&[u, v]| [u + 0.25 * f32::from(u8::try_from(k + 1).unwrap()), v])
                .collect();
            set.extend(shifted);
        }
        for set in &mut seg.extra_colors {
            set.extend_from_within(..);
        }
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
            assert_eq!(a.extra_uvs, b.extra_uvs, "{}", entry.path);
            assert_eq!(a.colors, b.colors, "{}", entry.path);
            assert_eq!(a.extra_colors, b.extra_colors, "{}", entry.path);
            assert_eq!(a.joints, b.joints, "{}", entry.path);
            assert_eq!(a.weights, b.weights, "{}", entry.path);
            assert_eq!(a.indices, b.indices, "{}", entry.path);
        }
        checked += 1;
    }
    eprintln!("{checked} models edited and read back");
    assert!(checked > 100);
}

/// Map segments with several texture coordinate and color sets export
/// them all (`TEXCOORD_n`, `COLOR_n`) and read them back; set 0 is the first
/// stream in block order.
#[test]
fn every_stream_set_exports_and_reads_back() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let path = "ffx_data/gamedata/ps3data/map/znkd/znkd14/mdl/d3d11/znkd14.dae.phyre";
    let bytes = archive.read(archive.find(path).unwrap()).unwrap();
    let model = ffx_phyre::Model::parse(&bytes).unwrap();
    let (uv2, uv3, c2) = model.segments.iter().fold((0, 0, 0), |(a, b, c), s| {
        (
            a + usize::from(!s.extra_uvs.is_empty()),
            b + usize::from(s.extra_uvs.len() > 1),
            c + usize::from(!s.extra_colors.is_empty()),
        )
    });
    eprintln!("{path}: {uv2} segments with 2+ UV sets, {uv3} with 3, {c2} with 2+ color sets");
    assert!(uv2 > 0 && uv3 > 0 && c2 > 0);
    // Stream set numbers as stored (`m_streamSet`), and in the glTF extras.
    assert_eq!(
        model.segments[50].uv_sets,
        [1],
        "znkd14 segment 50: its only ST is set 1"
    );
    assert_eq!(model.segments[6].uv_sets, [1, 2]);
    assert_eq!(model.segments[144].uv_sets, [0, 1, 2]);
    let glb = model.to_glb();
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    let attrs: Vec<&serde_json::Value> = json["meshes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|m| m["primitives"].as_array().unwrap())
        .map(|p| &p["attributes"])
        .collect();
    assert!(attrs.iter().any(|a| !a["TEXCOORD_2"].is_null()));
    assert!(attrs.iter().any(|a| !a["COLOR_1"].is_null()));
    let prim_of = |seg: u64| {
        json["meshes"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|m| m["primitives"].as_array().unwrap())
            .find(|p| p["extras"]["ffx_segment"] == seg)
            .unwrap()
            .clone()
    };
    assert_eq!(prim_of(50)["extras"]["ffx_uv_sets"], serde_json::json!([1]));
    assert_eq!(
        prim_of(144)["extras"]["ffx_uv_sets"],
        serde_json::json!([0, 1, 2])
    );
    assert!(prim_of(0)["extras"].get("ffx_uv_sets").is_none());
    for (other, seg, uv_sets, color_sets) in [
        ("map/mihn/mihn02/mdl/d3d11/mihn02", 368, &[0u8, 2][..], None),
        (
            "map/swin/swin00/mdl/d3d11/swin00",
            79,
            &[0u8][..],
            Some(&[0u8, 2][..]),
        ),
    ] {
        let p = format!("ffx_data/gamedata/ps3data/{other}.dae.phyre");
        let m = ffx_phyre::Model::parse(&archive.read(archive.find(&p).unwrap()).unwrap()).unwrap();
        assert_eq!(m.segments[seg].uv_sets, uv_sets, "{p} segment {seg}");
        if let Some(c) = color_sets {
            assert_eq!(m.segments[seg].color_sets, c, "{p} segment {seg}");
        }
    }
    // Edited sets come back as edited; the rest stays as stored.
    let mut edited = model.clone();
    let s = edited
        .segments
        .iter()
        .position(|s| s.extra_uvs.len() > 1)
        .unwrap();
    for uv in &mut edited.segments[s].extra_uvs[1] {
        uv[0] += 0.5;
    }
    let mut back = model.clone();
    back.import_glb(&edited.to_glb()).unwrap();
    // The edited set within float noise (v is flipped twice), the others
    // bit for bit.
    let close = |a: &[[f32; 2]], b: &[[f32; 2]]| {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(x, y)| (x[0] - y[0]).abs() < 1e-6 && (x[1] - y[1]).abs() < 1e-6)
    };
    assert!(close(
        &back.segments[s].extra_uvs[1],
        &edited.segments[s].extra_uvs[1]
    ));
    assert_eq!(
        back.segments[s].extra_uvs[0],
        model.segments[s].extra_uvs[0]
    );
    assert_eq!(back.segments[s].uvs, model.segments[s].uvs);
    let written = back.write(&bytes).unwrap();
    let reread = ffx_phyre::Model::parse(&written).unwrap();
    assert_eq!(reread.segments[s].extra_uvs, back.segments[s].extra_uvs);
    assert_eq!(reread.segments[s].uvs, model.segments[s].uvs);
    assert_eq!(reread.segments[s].tangents, model.segments[s].tangents);
}

/// Geometry that grows past an unskinned instance's culling box grows the
/// box (`PMeshInstanceBounds`); other instances keep theirs.
#[test]
fn edited_geometry_grows_its_culling_box() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let mut checked = 0;
    for entry in archive
        .entries()
        .iter()
        .filter(|e| e.path.contains("/map/") && e.path.ends_with(".dae.phyre") && e.size > 0)
        .step_by(20)
    {
        let bytes = archive.read(entry).unwrap();
        let Ok(mut model) = ffx_phyre::Model::parse(&bytes) else {
            continue;
        };
        let Some((i, s)) = model.instances.iter().enumerate().find_map(|(i, inst)| {
            let &(s, _) = inst.segments.first()?;
            (inst.bounds.is_some() && inst.skeleton.is_none()).then_some((i, s))
        }) else {
            continue;
        };
        let old = model.instances[i].bounds.unwrap();
        let center = [0, 1, 2].map(|k| old.min[k] + old.size[k] / 2.0);
        for p in &mut model.segments[s].positions {
            for k in 0..3 {
                p[k] = center[k] + (p[k] - center[k]) * 3.0 + 1.0;
            }
        }
        let written = model.write(&bytes).unwrap();
        let back = ffx_phyre::Model::parse(&written).unwrap();
        let b = back.instances[i].bounds.unwrap();
        for p in &model.segments[s].positions {
            for k in 0..3 {
                assert!(
                    p[k] >= b.min[k] && p[k] <= b.min[k] + b.size[k] * 1.000_01 + 1e-4,
                    "{}: instance {i} box {b:?} misses {p:?}",
                    entry.path
                );
            }
        }
        for (j, (a, c)) in model.instances.iter().zip(&back.instances).enumerate() {
            if j != i && !a.segments.iter().any(|&(x, _)| x == s) {
                assert_eq!(a.bounds, c.bounds, "{}: instance {j}", entry.path);
            }
        }
        checked += 1;
    }
    eprintln!("{checked} map models with a grown instance");
    assert!(checked > 10);

    // An instance with a skeleton keeps its box even when its edited
    // segment has no joints (the skeleton alone decides).
    let path = "ffx_data/gamedata/ps3data/map/cdsp/cdsp00/mdl/d3d11/cdsp00.dae.phyre";
    let bytes = archive.read(archive.find(path).unwrap()).unwrap();
    let mut model = ffx_phyre::Model::parse(&bytes).unwrap();
    let i = model
        .instances
        .iter()
        .position(|inst| inst.skeleton.is_none() && inst.bounds.is_some())
        .unwrap();
    let s = model.instances[i].segments[0].0;
    assert_eq!(model.segments[s].joints, Vec::<[u8; 4]>::new());
    model.instances[i].skeleton = Some(0);
    for p in &mut model.segments[s].positions {
        *p = p.map(|v| v * 5.0 + 100.0);
    }
    let back = ffx_phyre::Model::parse(&model.write(&bytes).unwrap()).unwrap();
    assert_eq!(back.instances[i].bounds, model.instances[i].bounds);

    // A skinned instance's box is the game's to move: kept as it is.
    let path = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
    let bytes = archive.read(archive.find(path).unwrap()).unwrap();
    let mut model = ffx_phyre::Model::parse(&bytes).unwrap();
    let (i, inst) = model
        .instances
        .iter()
        .enumerate()
        .find(|(_, inst)| inst.skeleton.is_some() && inst.bounds.is_some())
        .unwrap();
    let s = inst.segments[0].0;
    for p in &mut model.segments[s].positions {
        *p = p.map(|v| v * 5.0 + 100.0);
    }
    let back = ffx_phyre::Model::parse(&model.write(&bytes).unwrap()).unwrap();
    assert_eq!(back.instances[i].bounds, model.instances[i].bounds);
}

/// The writer's rules for further texture coordinate and color sets, on
/// the map segments that have them (layouts with `Color` first, `ST:1 ST:2`,
/// `Color:2`, three `ST` sets): a further set left empty keeps its stored
/// values, or takes set 0's when the segment is resized; a resized segment
/// gets each set's tangents from that set; an import of a glTF without a
/// further `TEXCOORD_n` changes nothing.
#[test]
#[allow(clippy::too_many_lines, clippy::float_cmp)] // one pass per rule; fallback tangents are bit for bit equal
fn further_sets_follow_the_writers_rules() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let (mut resized, mut flipped) = (0, 0);
    for name in [
        "znkd/znkd14/mdl/d3d11/znkd14",
        "swin/swin00/mdl/d3d11/swin00",
        "bika/bika04/mdl/d3d11/bika04",
        "mihn/mihn02/mdl/d3d11/mihn02",
    ] {
        let path = format!("ffx_data/gamedata/ps3data/map/{name}.dae.phyre");
        let bytes = archive.read(archive.find(&path).unwrap()).unwrap();
        let model = ffx_phyre::Model::parse(&bytes).unwrap();
        let multi: Vec<usize> = (0..model.segments.len())
            .filter(|&s| {
                let seg = &model.segments[s];
                (!seg.extra_uvs.is_empty() || !seg.extra_colors.is_empty())
                    && 2 * seg.positions.len() <= 0x1_0000
            })
            .collect();
        assert!(!multi.is_empty(), "{path}");

        // An emptied further set, vertex count unchanged: the file is unchanged.
        let mut m = model.clone();
        for &s in &multi {
            for set in &mut m.segments[s].extra_uvs {
                set.clear();
            }
            for set in &mut m.segments[s].extra_colors {
                set.clear();
            }
        }
        assert!(
            m.write(&bytes).unwrap() == bytes,
            "{path}: emptied sets changed the file"
        );

        // Resized (every vertex twice), further sets emptied: they take set 0.
        // Resized with set 1 = set 0 mirrored in u: its tangents point the
        // other way.
        let grow = |seg: &mut ffx_phyre::Segment| {
            let n = u16::try_from(seg.positions.len()).unwrap();
            seg.positions.extend_from_within(..);
            seg.normals.extend_from_within(..);
            seg.uvs.extend_from_within(..);
            seg.colors.extend_from_within(..);
            let more: Vec<u16> = seg.indices.iter().map(|i| i + n).collect();
            seg.indices.extend(more);
        };
        let (mut empty, mut mirrored) = (model.clone(), model.clone());
        for &s in &multi {
            grow(&mut empty.segments[s]);
            empty.segments[s].extra_uvs.iter_mut().for_each(Vec::clear);
            empty.segments[s]
                .extra_colors
                .iter_mut()
                .for_each(Vec::clear);
            let seg = &mut mirrored.segments[s];
            grow(seg);
            let first = seg.uvs.clone();
            for set in &mut seg.extra_uvs {
                *set = first.iter().map(|&[u, v]| [-u, v]).collect();
            }
            let colors = seg.colors.clone();
            for set in &mut seg.extra_colors {
                set.clone_from(&colors);
            }
        }
        let back = ffx_phyre::Model::parse(&empty.write(&bytes).unwrap()).unwrap();
        let back_m = ffx_phyre::Model::parse(&mirrored.write(&bytes).unwrap()).unwrap();
        for &s in &multi {
            let (b, e) = (&back.segments[s], &empty.segments[s]);
            assert_eq!(b.positions, e.positions, "{path} segment {s}");
            for set in &b.extra_uvs {
                assert_eq!(
                    set, &e.uvs,
                    "{path} segment {s}: an empty set takes set 0's"
                );
            }
            for set in &b.extra_colors {
                assert_eq!(set, &e.colors, "{path} segment {s}");
            }
            let (b, m) = (&back_m.segments[s], &mirrored.segments[s]);
            assert_eq!(b.extra_uvs, m.extra_uvs, "{path} segment {s}");
            assert_eq!(b.extra_colors, m.extra_colors, "{path} segment {s}");
            assert_eq!(b.uv_sets, model.segments[s].uv_sets, "{path} segment {s}");
            if !b.tangents.is_empty() {
                for t in &b.extra_tangents {
                    let opposite = t
                        .iter()
                        .zip(&b.tangents)
                        .filter(|(x, y)| x.iter().zip(y.iter()).all(|(a, b)| (a + b).abs() < 1e-4))
                        .count();
                    // Where the UVs give no direction (degenerate triangles),
                    // both sets take the same fallback tangent.
                    let same = t.iter().zip(&b.tangents).filter(|(x, y)| x == y).count();
                    assert_eq!(
                        opposite + same,
                        t.len(),
                        "{path} segment {s}: {opposite} opposite and {same} fallback tangents of {}",
                        t.len()
                    );
                    flipped += opposite;
                }
            }
            resized += 1;
        }

        // A glTF without the further TEXCOORD_n and COLOR_n: nothing changes.
        let mut glb = Glb::parse(&model.to_glb());
        for mesh in glb.json["meshes"].as_array_mut().unwrap() {
            for prim in mesh["primitives"].as_array_mut().unwrap() {
                let attrs = prim["attributes"].as_object_mut().unwrap();
                attrs.retain(|k, _| {
                    k == "TEXCOORD_0"
                        || k == "COLOR_0"
                        || !(k.starts_with("TEXCOORD_") || k.starts_with("COLOR_"))
                });
            }
        }
        let mut imported = model.clone();
        imported.import_glb(&glb.bytes()).unwrap();
        assert!(
            imported.write(&bytes).unwrap() == bytes,
            "{path}: an import without further sets changed the file"
        );
    }
    eprintln!(
        "{resized} multi-set segments resized and read back, {flipped} tangents of mirrored sets reversed"
    );
    assert!(resized > 10 && flipped > 10_000);
}

/// Every model, and every 20th other cluster, rewritten through
/// `ClusterEdit` (fixups re-encoded) unpacks to the same objects, arrays,
/// fixups and buffers, and models parse to the same geometry.
#[test]
fn clusters_rewrite_through_cluster_edit() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let (mut models, mut others, mut skipped) = (0, 0, 0);
    for (k, entry) in archive
        .entries()
        .iter()
        .filter(|e| e.path.ends_with(".phyre") && e.size > 0)
        .enumerate()
    {
        let is_model = entry.path.ends_with(".dae.phyre");
        if !is_model && k % 20 != 0 {
            continue;
        }
        let bytes = archive.read(entry).unwrap();
        let Ok(edit) = ffx_phyre::edit::ClusterEdit::parse(&bytes) else {
            skipped += 1;
            continue;
        };
        let written = edit
            .write()
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        let again = ffx_phyre::edit::ClusterEdit::parse(&written)
            .unwrap_or_else(|e| panic!("{}: rewritten: {e}", entry.path));
        for (a, b) in again.lists.iter().zip(&edit.lists) {
            assert_eq!(a.objects, b.objects, "{} objects", entry.path);
            assert_eq!(
                a.arrays,
                b.arrays,
                "{} arrays {} {} objs {}",
                entry.path,
                a.arrays.len(),
                b.arrays.len(),
                a.objects.len()
            );
            assert_eq!(
                a.pointer_fixups, b.pointer_fixups,
                "{} pointers",
                entry.path
            );
            assert_eq!(
                a.array_fixups, b.array_fixups,
                "{} array fixups",
                entry.path
            );
            assert_eq!(
                a.pointer_array_fixups, b.pointer_array_fixups,
                "{}",
                entry.path
            );
        }
        assert_eq!(again.lists, edit.lists, "{}", entry.path);
        assert_eq!(again.user_records, edit.user_records, "{}", entry.path);
        assert_eq!(
            again.header_class_fixups, edit.header_class_fixups,
            "{}",
            entry.path
        );
        // Header classes come in list order; the parameter buffer lists'
        // are the last ones, each counting the buffer's tweakable
        // parameter definitions.
        let buffers: Vec<&ffx_phyre::edit::EditList> = edit
            .lists
            .iter()
            .filter(|l| edit.cluster.classes[l.class].name == "PParameterBuffer")
            .collect();
        if is_model {
            let before = edit
                .header_class_counts
                .len()
                .checked_sub(buffers.len())
                .unwrap_or_else(|| panic!("{}: fewer header classes than buffers", entry.path));
            for (l, &n) in buffers.iter().zip(&edit.header_class_counts[before..]) {
                let defs = u32::from_le_bytes(l.objects[8..12].try_into().unwrap());
                assert_eq!((l.count(), defs), (1, n), "{}", entry.path);
            }
        }
        assert_eq!(
            edit.header_class_counts.iter().sum::<u32>() as usize,
            edit.header_class_fixups.len(),
            "{}",
            entry.path
        );
        assert_eq!(again.buffers, edit.buffers, "{}", entry.path);
        if is_model && let Ok(model) = ffx_phyre::Model::parse(&bytes) {
            let back = ffx_phyre::Model::parse(&written).unwrap();
            assert_eq!(back.segments, model.segments, "{}", entry.path);
            models += 1;
        } else {
            others += 1;
        }
    }
    eprintln!("{models} models and {others} other clusters rewritten, {skipped} not parsed");
}

/// A `.glb` split into its JSON and binary chunk, and put back together.
struct Glb {
    json: serde_json::Value,
    bin: Vec<u8>,
}

impl Glb {
    fn parse(glb: &[u8]) -> Self {
        let word = |o: usize| u32::from_le_bytes(glb[o..o + 4].try_into().unwrap()) as usize;
        let json_len = word(12);
        let json = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        let at = 20 + json_len;
        let bin = glb[at + 8..at + 8 + word(at)].to_vec();
        Self { json, bin }
    }

    /// Appends data with an accessor; returns the accessor index.
    fn accessor(&mut self, data: &[u8], count: usize, ty: &str, component: u32) -> usize {
        while !self.bin.len().is_multiple_of(4) {
            self.bin.push(0);
        }
        let views = self.json["bufferViews"].as_array_mut().unwrap();
        views.push(serde_json::json!({"buffer": 0, "byteOffset": self.bin.len(), "byteLength": data.len()}));
        let view = views.len() - 1;
        self.bin.extend_from_slice(data);
        let accessors = self.json["accessors"].as_array_mut().unwrap();
        let mut a = serde_json::json!({"bufferView": view, "count": count, "type": ty, "componentType": component});
        if ty == "VEC3" && component == 5126 {
            // glTF wants bounds on positions.
            let f: Vec<f32> = data
                .chunks(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for v in f.chunks(3) {
                for k in 0..3 {
                    lo[k] = lo[k].min(v[k]);
                    hi[k] = hi[k].max(v[k]);
                }
            }
            a["min"] = serde_json::json!(lo);
            a["max"] = serde_json::json!(hi);
        }
        accessors.push(a);
        accessors.len() - 1
    }

    fn bytes(&self) -> Vec<u8> {
        let mut json = serde_json::to_vec(&self.json).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        self.json["buffers"][0]["byteLength"].as_u64().unwrap();
        let mut j = self.json.clone();
        j["buffers"][0]["byteLength"] = serde_json::json!(self.bin.len());
        json = serde_json::to_vec(&j).unwrap();
        while !json.len().is_multiple_of(4) {
            json.push(b' ');
        }
        let mut out = b"glTF".to_vec();
        out.extend(2u32.to_le_bytes());
        out.extend(
            u32::try_from(12 + 8 + json.len() + 8 + self.bin.len())
                .unwrap()
                .to_le_bytes(),
        );
        out.extend(u32::try_from(json.len()).unwrap().to_le_bytes());
        out.extend(b"JSON");
        out.extend(json);
        out.extend(u32::try_from(self.bin.len()).unwrap().to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(&self.bin);
        out
    }
}

/// Tidus with what a modeling tool adds: a new primitive (his head, moved
/// and weighted to a joint his mesh has no skeleton matrix for), and his
/// head segment weighted to a joint outside its one-bone palette.
#[test]
#[allow(clippy::too_many_lines, clippy::cast_possible_truncation)] // one scenario, small indices
fn new_segments_and_joints_import() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let path = "ffx_data/gamedata/ps3data/chr/pc/c001/mdl/d3d11/c001.dae.phyre";
    let original = archive.read(archive.find(path).unwrap()).unwrap();
    let model = ffx_phyre::Model::parse(&original).unwrap();
    let skeleton = &model.skeletons[0];
    // Joints by skeleton matrix: a hierarchy joint no palette uses.
    let used: Vec<u16> = model
        .segments
        .iter()
        .flat_map(|s| s.bones.iter().map(|b| b.hierarchy))
        .collect();
    let unbound = (0..skeleton.names.len())
        .map(|h| u16::try_from(h).unwrap())
        .find(|h| {
            !used.contains(h)
                && model
                    .nodes
                    .iter()
                    .any(|n| n.name == skeleton.names[usize::from(*h)])
        })
        .expect("an unbound joint");
    let head = &model.segments[0];
    let other = model.segments[5].bones[3].hierarchy;
    assert!(!head.bones.iter().any(|b| b.hierarchy == other));

    let mut glb = Glb::parse(&model.to_glb());
    let (mi, pi) = glb.json["meshes"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .find_map(|(mi, m)| {
            m["primitives"]
                .as_array()?
                .iter()
                .position(|p| p["extras"]["ffx_segment"] == 0)
                .map(|pi| (mi, pi))
        })
        .unwrap();
    // The skin's joint numbers of the two joints.
    let node_of = |name: &str| {
        glb.json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["name"] == name)
            .unwrap()
    };
    let skin = glb.json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["mesh"] == mi && n.get("skin").is_some())
        .and_then(|n| n["skin"].as_u64())
        .unwrap() as usize;
    let joint = |h: u16| {
        let node = node_of(&skeleton.names[usize::from(h)]);
        glb.json["skins"][skin]["joints"]
            .as_array()
            .unwrap()
            .iter()
            .position(|j| j == node)
            .unwrap()
    };
    let (j_other, j_unbound) = (joint(other), joint(unbound));
    let n = head.positions.len();
    let weights: Vec<u8> = (0..n)
        .flat_map(|_| [1f32, 0.0, 0.0, 0.0])
        .flat_map(f32::to_le_bytes)
        .collect();
    let joints_to = |j: usize| -> Vec<u8> {
        (0..n)
            .flat_map(|_| [u8::try_from(j).unwrap(), 0, 0, 0])
            .collect()
    };
    // Head: its first vertex now follows `other`.
    let mut head_joints = joints_to(joint(head.bones[0].hierarchy));
    head_joints[0] = u8::try_from(j_other).unwrap();
    let a_joints = glb.accessor(&head_joints, n, "VEC4", 5121);
    let a_weights = glb.accessor(&weights, n, "VEC4", 5126);
    let prim = &mut glb.json["meshes"][mi]["primitives"][pi];
    prim["attributes"]["JOINTS_0"] = serde_json::json!(a_joints);
    prim["attributes"]["WEIGHTS_0"] = serde_json::json!(a_weights);
    let mut new_prim = prim.clone();
    // The new primitive: the head moved by (4, 0, 0) in glTF space, on `unbound`.
    let positions = new_prim["attributes"]["POSITION"].as_u64().unwrap() as usize;
    let view = &glb.json["bufferViews"][glb.json["accessors"][positions]["bufferView"]
        .as_u64()
        .unwrap() as usize];
    let at = view["byteOffset"].as_u64().unwrap_or(0) as usize;
    let moved: Vec<u8> = glb.bin[at..at + 12 * n]
        .chunks(4)
        .enumerate()
        .flat_map(|(i, b)| {
            let v = f32::from_le_bytes(b.try_into().unwrap()) + if i % 3 == 0 { 4.0 } else { 0.0 };
            v.to_le_bytes()
        })
        .collect();
    let a_pos = glb.accessor(&moved, n, "VEC3", 5126);
    let a_joints = glb.accessor(&joints_to(j_unbound), n, "VEC4", 5121);
    new_prim["attributes"]["POSITION"] = serde_json::json!(a_pos);
    new_prim["attributes"]["JOINTS_0"] = serde_json::json!(a_joints);
    new_prim.as_object_mut().unwrap().remove("extras");
    // A new glTF material: the head's, with Yuna's texture.
    let materials = glb.json["materials"].as_array_mut().unwrap();
    let mut material = materials[new_prim["material"].as_u64().unwrap() as usize].clone();
    material["extras"]["ffx_textures"]["TextureSampler"] =
        serde_json::json!("PS3Data/chr/pc/c002/tex/c002.dds");
    materials.push(material);
    new_prim["material"] = serde_json::json!(materials.len() - 1);
    glb.json["meshes"][mi]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(new_prim);

    let imported = ffx_phyre::import_into_cluster(&original, &glb.bytes()).unwrap();
    assert_eq!(imported.new_textures, ["PS3Data/chr/pc/c002/tex/c002.dds"]);
    let (bytes, done) = (imported.bytes, imported.segments);
    let result = ffx_phyre::Model::parse(&bytes).unwrap();
    assert_eq!(result.segments.len(), model.segments.len() + 1);
    let new = done
        .iter()
        .copied()
        .find(|&s| s == 13)
        .expect("new segment 13 written");
    let s = &result.segments[new];
    assert_eq!(s.positions.len(), n);
    assert!(
        (s.positions[0][0] - head.positions[0][0]).abs() > 3.0,
        "moved"
    );
    assert_eq!(s.bones.len(), 1);
    assert_eq!(s.bones[0].hierarchy, unbound);
    let material = &result.materials[s.material.unwrap()];
    assert!(
        material
            .textures
            .iter()
            .any(|t| t.sampler == "TextureSampler" && t.path.ends_with("c002.dds")),
        "{:?}",
        material.textures
    );
    assert_eq!(
        result.segments[0]
            .material
            .map(|m| &result.materials[m].textures),
        head.material.map(|m| &model.materials[m].textures)
    );
    let skel = &result.skeletons[0];
    assert_eq!(skel.inverse_bind.len(), skeleton.inverse_bind.len() + 1);
    assert_eq!(
        usize::from(s.bones[0].skeleton),
        skeleton.inverse_bind.len()
    );
    // The head kept its geometry and gained `other` in its palette.
    let h = &result.segments[0];
    assert_eq!(h.positions, head.positions);
    assert!(h.bones.iter().any(|b| b.hierarchy == other));
    assert_eq!(h.bones[0], head.bones[0]);
    // Everything else is as it was.
    for k in 1..13 {
        assert_eq!(
            result.segments[k].positions, model.segments[k].positions,
            "segment {k}"
        );
    }
    eprintln!(
        "new segment {new}, joint {} bound as skeleton matrix {}",
        skeleton.names[usize::from(unbound)],
        s.bones[0].skeleton
    );
}

/// A new primitive beside an unskinned map part becomes a new segment of
/// its instance (no skeleton involved).
#[test]
fn a_new_part_joins_an_unskinned_instance() {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return;
    };
    let archive = Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap();
    let path = "ffx_data/gamedata/ps3data/map/cdsp/cdsp00/mdl/d3d11/cdsp00.dae.phyre";
    let bytes = archive.read(archive.find(path).unwrap()).unwrap();
    let model = ffx_phyre::Model::parse(&bytes).unwrap();
    let mut glb = Glb::parse(&model.to_glb());
    // A copy of the first mesh's first primitive, without its segment tag.
    let mut prim = glb.json["meshes"][0]["primitives"][0].clone();
    prim["extras"] = serde_json::json!({});
    glb.json["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(prim);
    let imported = ffx_phyre::import_into_cluster(&bytes, &glb.bytes()).unwrap();
    let back = ffx_phyre::Model::parse(&imported.bytes).unwrap();
    assert_eq!(back.segments.len(), model.segments.len() + 1);
    assert_eq!(imported.new_textures, Vec::<String>::new());
}
