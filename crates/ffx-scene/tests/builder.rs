//! The builder gate (`docs/scene-builder.md`): the scene builder's map
//! draws of a captured cdsp0000 frame against the captured map draws
//! (scene draws 0-99 and 117-127), through `ffx-drawdiff`: 0 differences
//! in every class but those `tests/builder-allow.txt` allows with a
//! written cause.
//!
//! Needs `FFX_GAME_DIR` and `FFX_SCENE_CAPTURES` (`capture.ffxgfx` files
//! of the field-walk-1 frame separated by `:`, e.g.
//! `~/ffx-work/m4/captures/fw1-a/capture.ffxgfx:~/ffx-work/m4/captures/fw3-b/capture.ffxgfx`)
//! or the census gate's `FFX_SCENE_CAPTURE` (one); skips without them. The texture quality comes from the test
//! sandbox's `GameSetting.ini` (`tools/sandbox`), which the round 1
//! captures share (`TQ_LOW`, MSAA off).

use ffx_scene::build::{Clusters, Pass, build};
use ffx_scene::capture::Capture;
use ffx_scene::capture_inputs::CaptureInputs;
use ffx_scene::frame::Settings;

const MODEL: &str = "ffx_data/gamedata/ps3data/map/cdsp/cdsp00/mdl/d3d11/cdsp00.dae.phyre";

fn setup() -> Option<(Vec<String>, Clusters, Settings)> {
    let game = std::env::var_os("FFX_GAME_DIR")?;
    let Ok(captures) =
        std::env::var("FFX_SCENE_CAPTURES").or_else(|_| std::env::var("FFX_SCENE_CAPTURE"))
    else {
        eprintln!("FFX_SCENE_CAPTURES and FFX_SCENE_CAPTURE unset: skipped");
        return None;
    };
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let clusters = Clusters::load(
        |p| archive.find(p).and_then(|e| archive.read(e).ok()),
        MODEL,
    )
    .expect("clusters");
    let ini = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/sandbox/GameSetting.ini"),
    )
    .expect("sandbox GameSetting.ini");
    let settings = Settings::from_ini(&ini).expect("settings");
    Some((
        captures.split(':').map(str::to_owned).collect(),
        clusters,
        settings,
    ))
}

#[test]
fn the_builders_map_draws_are_the_captured_ones() {
    let Some((captures, clusters, settings)) = setup() else {
        return;
    };
    let allow_text = include_str!("builder-allow.txt");
    let allow = ffx_drawdiff::rules::parse("builder-allow.txt", allow_text).expect("allow list");
    for path in captures {
        let bytes = std::fs::read(&path).expect("capture");
        let cap = Capture::read(&bytes).expect("capture");
        let inputs = CaptureInputs::new(&cap, &clusters, settings, false).expect("inputs");
        // The census's map draws: 100 opaque, 11 transparent.
        let numbers: Vec<usize> = inputs.matched.iter().map(|m| m.number).collect();
        let want: Vec<usize> = (0..100).chain(117..128).collect();
        assert_eq!(numbers, want, "{path}: the captured map draws");
        let built = build(&clusters, &inputs).expect("build");
        assert_eq!(built.pass(Pass::Opaque).count(), 100, "{path}");
        assert_eq!(built.pass(Pass::Transparent).count(), 11, "{path}");
        let (opaque, transparent) = inputs.pass_numbers();
        assert_eq!(opaque, (0..100).collect::<Vec<_>>(), "{path}");
        assert_eq!(transparent, (117..128).collect::<Vec<_>>(), "{path}");
        let (opaque, transparent) =
            ffx_scene::gate::pass_cmds(&ffx_scene::gate::map_draws(&cap, &clusters));
        let msgs =
            ffx_scene::emit::replace_map_draws(&cap, &built, &opaque, &transparent).expect("emit");
        let a = ffx_drawdiff::capture::Capture::parse(&bytes).expect("A");
        let b = ffx_drawdiff::capture::Capture::from_msgs(&msgs);
        let opts = ffx_drawdiff::diff::Options {
            part: ffx_drawdiff::diff::Part::Scene,
            tolerances: Vec::new(),
            allow: allow.clone(),
        };
        let cmp = ffx_drawdiff::diff::compare(&a, &b, &opts);
        let report = ffx_drawdiff::report::render(
            (&path, "built"),
            &a,
            &b,
            &cmp,
            &opts,
            ffx_drawdiff::report::Style::default(),
        );
        eprintln!("{report}");
        assert_eq!(cmp.frames.len(), 1, "{path}");
        let f = &cmp.frames[0];
        assert_eq!(f.a_items, f.b_items, "{path}: scene items");
        assert_eq!(cmp.unallowed(), 0, "{path}:\n{report}");
    }
}

#[test]
fn every_texture_quality_builds() {
    let Some((captures, clusters, mut settings)) = setup() else {
        return;
    };
    let cap = Capture::read(&std::fs::read(&captures[0]).expect("capture")).expect("capture");
    let mut widest = Vec::new();
    for q in [
        ffx_scene::frame::TextureQuality::High,
        ffx_scene::frame::TextureQuality::Medium,
        ffx_scene::frame::TextureQuality::Low,
    ] {
        settings.texture_quality = q;
        let inputs = CaptureInputs::new(&cap, &clusters, settings, false).expect("inputs");
        let built = build(&clusters, &inputs).expect("build");
        let w = built.resources.textures.iter().map(|t| t.desc.width).max();
        widest.push(w.unwrap_or(0));
    }
    // cdsp00's largest map texture is 2048 wide: 2048, 1024, 512.
    assert_eq!(widest, vec![2048, 1024, 512]);
}
