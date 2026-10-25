//! The character builder gate (`docs/formats/scene-chars.md`, "The
//! builder"): the character world's draws built from a cdsp0000 frame's
//! scene inputs and the `chr/` clusters replace the captured ones (scene
//! draws 100-116) and `ffx-drawdiff` finds 0 differences in the scene but
//! those `tests/chars-allow.txt` allows with a written cause.
//!
//! Needs `FFX_GAME_DIR` and `FFX_SCENE_CHARS` (the character gate's
//! captures, `:`-separated); skips when either is unset. Captures of other
//! scenes (bsil0000) are built and reported, not gated.

use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS};
use ffx_scene::build::Pass;
use ffx_scene::chars_model::ChrIndex;
use ffx_scene::frame::Settings;
use ffx_scene::{chars_check, chars_diff};

#[test]
fn the_built_character_draws_are_the_captured_ones() {
    let files: Vec<std::path::PathBuf> = std::env::var_os("FFX_SCENE_CHARS")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    let Some(game) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR unset: skipped");
        return;
    };
    if files.is_empty() {
        eprintln!("FFX_SCENE_CHARS unset: skipped");
        return;
    }
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let ini = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/sandbox/GameSetting.ini"),
    )
    .expect("sandbox GameSetting.ini");
    let settings = Settings::from_ini(&ini).expect("settings");
    let allow = ffx_drawdiff::rules::parse("chars-allow.txt", include_str!("chars-allow.txt"))
        .expect("allow list");
    let captures: Vec<Vec<u8>> = files
        .iter()
        .map(|f| std::fs::read(f).expect("capture"))
        .collect();
    let scenes: Vec<String> = captures
        .iter()
        .filter_map(|b| chars_check::scene_of(b))
        .collect();
    let mut index = AssetIndex::new(TQ_LOW_DROPPED_LEVELS);
    index
        .add_archive(&archive, |p| chars_check::wanted(p, &scenes))
        .expect("index");
    let models = chars_diff::chr_models(&archive);
    let chr = ChrIndex::new(
        |p| archive.find(p).and_then(|e| archive.read(e).ok()),
        models.iter().map(String::as_str),
    );
    let mut gated = 0;
    for (file, bytes) in files.iter().zip(&captures) {
        let path = file.display().to_string();
        let o = chars_diff::run(bytes, &archive, &index, &chr, settings).expect("build");
        let a = ffx_drawdiff::capture::Capture::parse(bytes).expect("A");
        let b = ffx_drawdiff::capture::Capture::from_msgs(&o.msgs);
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
        eprintln!(
            "{path}: models {:?}; notes {:?}\n{report}",
            o.models, o.built.notes
        );
        assert!(
            o.blend_problems.is_empty(),
            "{path}: {:?}",
            o.blend_problems
        );
        if chars_check::scene_of(bytes).as_deref() != Some("cdsp0000") {
            continue;
        }
        gated += 1;
        assert_eq!(o.opaque, (100..115).collect::<Vec<_>>(), "{path}");
        assert_eq!(o.transparent, vec![115, 116], "{path}");
        assert_eq!(o.built.pass(Pass::Opaque).count(), 15, "{path}");
        assert_eq!(o.built.pass(Pass::Transparent).count(), 2, "{path}");
        assert_eq!(cmp.frames.len(), 1, "{path}");
        let f = &cmp.frames[0];
        assert_eq!(f.a_items, f.b_items, "{path}: scene items");
        assert_eq!(cmp.unallowed(), 0, "{path}:\n{report}");
    }
    assert!(gated > 0, "no cdsp0000 capture in FFX_SCENE_CHARS");
}
