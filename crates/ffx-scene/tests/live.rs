//! The builder gate from the live snapshot (`docs/scene-builder.md`,
//! "Inputs from the snapshot"): every frame of each capture that has map
//! `renderWorld` records is built from those records alone
//! (`ffx_scene::live`) and diffed against its captured map draws
//! (`ffx_scene::gate::run`): 0 differences but the allow lists
//! (`tests/builder-allow.txt` and the map's
//! `crates/ffx-drawdiff/allow/<scene>.txt`).
//!
//! Needs `FFX_GAME_DIR` and `FFX_SCENE_WIRE` (`capture.ffxgfx` files
//! separated by `:`, e.g. `~/ffx-work/m4/captures/r2a-a/fw-101/capture.ffxgfx`
//! and `~/ffx-work/m4/captures/wire-bsil0000-a/bsil0000-101/capture.ffxgfx`);
//! skips without them.

use ffx_scene::capture::Capture;
use ffx_scene::frame::Settings;

fn allow_for(scene: &str) -> Vec<ffx_drawdiff::rules::Rule> {
    let mut rules =
        ffx_drawdiff::rules::parse("builder-allow.txt", include_str!("builder-allow.txt"))
            .expect("allow list");
    let map = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ffx-drawdiff/allow")
        .join(format!("{scene}.txt"));
    if let Ok(text) = std::fs::read_to_string(&map) {
        rules.extend(ffx_drawdiff::rules::parse(&map.to_string_lossy(), &text).expect("rules"));
    }
    rules
}

#[test]
fn the_snapshot_alone_builds_the_captured_map_draws() {
    let (Some(game), Some(files)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SCENE_WIRE"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SCENE_WIRE unset: skipped");
        return;
    };
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let read = |p: &str| archive.find(p).and_then(|e| archive.read(e).ok());
    let ini = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/sandbox/GameSetting.ini"),
    )
    .expect("sandbox GameSetting.ini");
    let settings = Settings::from_ini(&ini).expect("settings");
    let mut frames = 0;
    for path in std::env::split_paths(&files) {
        let bytes = std::fs::read(&path).expect("capture");
        let records = ffx_scene::inputs::read_wired(&bytes).expect("records");
        for cap in Capture::read_frames(&bytes).expect("capture") {
            let frame: Vec<_> = records
                .iter()
                .filter(|(s, _)| s.frame == cap.frame.number)
                .cloned()
                .collect();
            if frame.is_empty() {
                continue;
            }
            let scene = match &cap.world {
                Some(ffx_gfx::Msg::World { scene, .. }) => scene.clone(),
                _ => String::new(),
            };
            let label = format!("{} frame {}", path.display(), cap.frame.number);
            let r =
                ffx_scene::gate::run(&cap, &frame, &read, settings, allow_for(&scene), 12, &label)
                    .unwrap_or_else(|e| panic!("{label}: {e}"));
            eprintln!(
                "{label}: wire {} captured {:?} built {:?} unallowed {}",
                r.wired, r.captured, r.built, r.unallowed
            );
            assert_eq!(r.captured, r.built, "{label}: map draws per pass");
            assert_eq!(r.a_items, r.b_items, "{label}: scene items");
            assert_eq!(r.unallowed, 0, "{label}:\n{}", r.report);
            frames += 1;
        }
    }
    assert!(frames > 0, "no capture frame has map renderWorld records");
}
