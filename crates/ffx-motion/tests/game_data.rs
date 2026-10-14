//! Checks every shipped skeleton and motion group. Needs `FFX_GAME_DIR`;
//! skipped otherwise.

use std::path::PathBuf;

use ffx_motion::{MotionGroup, Skeleton};
use ffx_vbf::Archive;

fn archive() -> Option<Archive> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(Archive::open(PathBuf::from(dir).join("data/FFX_Data.vbf")).unwrap())
}

#[test]
fn every_motion_group_rebuilds_byte_for_byte() {
    let Some(archive) = archive() else { return };
    let (mut files, mut clips, mut exact) = (0, 0, 0);
    let mut first_diff = Vec::new();
    for entry in archive.entries() {
        if !entry.path.ends_with(".mgrp") {
            continue;
        }
        let bytes = archive.read(entry).unwrap();
        let group = MotionGroup::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        files += 1;
        clips += group.sequences.iter().map(|s| s.clips.len()).sum::<usize>();
        let rebuilt = group.write().unwrap();
        assert_eq!(
            MotionGroup::parse(&rebuilt).unwrap(),
            group,
            "{}: rebuilt group parses differently",
            entry.path
        );
        assert_eq!(
            MotionGroup::from_json(&group.to_json()).unwrap(),
            group,
            "{}: JSON round trip",
            entry.path
        );
        if rebuilt == bytes {
            exact += 1;
        } else if first_diff.len() < 10 {
            let at = rebuilt
                .iter()
                .zip(&bytes)
                .position(|(a, b)| a != b)
                .unwrap_or(rebuilt.len().min(bytes.len()));
            first_diff.push(format!(
                "{} at {at:#x} (sizes {:#x} / {:#x})",
                entry.path,
                rebuilt.len(),
                bytes.len()
            ));
        }
    }
    eprintln!("{files} groups, {clips} clips, {exact} rebuilt exactly");
    assert!(first_diff.is_empty(), "{first_diff:#?}");
    assert!(files > 3000 && clips > 20_000);
}

#[test]
fn every_skeleton_parses() {
    let Some(archive) = archive() else { return };
    let (mut files, mut boned) = (0, 0);
    for entry in archive.entries() {
        if !entry.path.ends_with(".chr") {
            continue;
        }
        let bytes = archive.read(entry).unwrap();
        let skeleton = Skeleton::from_chr(&bytes).unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        files += 1;
        boned += usize::from(!skeleton.bones.is_empty());
    }
    eprintln!("{files} .chr files, {boned} with bones");
    assert!(boned > 800, "{boned}");
}
