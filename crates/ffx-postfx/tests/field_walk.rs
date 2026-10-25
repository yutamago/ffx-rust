//! The post-processing gate on field-walk-1's frame (`docs/formats/postfx.md`):
//! every draw outside the scene is classified, in `PhyreScene_render`'s
//! order, every constant of the post-process draws follows its source, and
//! two runs differ only inside the scene (its documented noise).
//!
//! Needs `FFX_GAME_DIR`; the captures are `FFX_POSTFX_CAPTURE` and
//! `FFX_POSTFX_CAPTURE_B` (default `~/ffx-work/m4/captures/fw1-a` and
//! `fw1-b`). Skips when the game or capture A is missing.

use std::path::{Path, PathBuf};

use ffx_postfx::census::{self, Class, Pass};
use ffx_postfx::{camera, shaders::ShaderNames};

fn capture_path(var: &str, default: &str) -> Option<PathBuf> {
    let p = std::env::var_os(var).map(PathBuf::from).or_else(|| {
        std::env::var_os("HOME").map(|h| Path::new(&h).join("ffx-work/m4/captures").join(default))
    })?;
    p.exists().then_some(p)
}

struct Setup {
    a: ffx_drawdiff::capture::Capture,
    b: Option<ffx_drawdiff::capture::Capture>,
    shaders: ShaderNames,
    cameras: Vec<camera::Camera>,
}

fn setup() -> Option<Setup> {
    let Some(game) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR unset: skipped");
        return None;
    };
    let Some(a) = capture_path("FFX_POSTFX_CAPTURE", "fw1-a") else {
        eprintln!("no capture A (FFX_POSTFX_CAPTURE): skipped");
        return None;
    };
    let archive =
        ffx_vbf::Archive::open(Path::new(&game).join("data/FFX_Data.vbf")).expect("archive");
    let shaders = ShaderNames::from_archive(&archive).expect("shaders");
    let cameras = camera::map_cameras(&archive, "/map/cdsp/cdsp00/").expect("cameras");
    let a = ffx_drawdiff::capture::Capture::read(&a).expect("capture A");
    let b = capture_path("FFX_POSTFX_CAPTURE_B", "fw1-b")
        .map(|p| ffx_drawdiff::capture::Capture::read(&p).expect("capture B"));
    Some(Setup {
        a,
        b,
        shaders,
        cameras,
    })
}

fn run(s: &Setup) -> census::Census {
    let inputs = census::Inputs {
        shaders: &s.shaders,
        cameras: &s.cameras,
        // tools/sandbox/GameSetting.ini
        brightness: 50,
    };
    census::census(&s.a.frames[0], &inputs)
}

#[test]
fn every_draw_outside_the_scene_is_classified_in_order() {
    let Some(s) = setup() else {
        return;
    };
    let c = run(&s);
    let unclassified: Vec<usize> = c
        .draws
        .iter()
        .filter(|d| d.class == Class::Unclassified)
        .map(|d| d.number)
        .collect();
    assert!(
        unclassified.is_empty(),
        "unclassified draws {unclassified:?}"
    );
    assert!(
        c.out_of_order.is_empty(),
        "out of order {:?}",
        c.out_of_order
    );
    let totals = c.totals();
    assert_eq!(
        totals,
        vec![
            ("scene".to_owned(), 128),
            ("post".to_owned(), 18),
            ("effects".to_owned(), 35),
            ("ui".to_owned(), 16),
        ],
        "{}",
        c.report()
    );
    // The chain as docs/formats/postfx.md lists it.
    let post: Vec<String> = c
        .draws
        .iter()
        .filter_map(|d| match d.class {
            Class::Post(p) => Some(p.name()),
            _ => None,
        })
        .collect();
    assert_eq!(post.first().map(String::as_str), Some("glow map"));
    assert_eq!(post.last().map(String::as_str), Some("composite"));
    let last = c.draws.last().expect("draws");
    assert_eq!(last.class, Class::Post(Pass::Composite));
    assert!(last.target.contains("(undeclared)"), "{}", last.target);
}

#[test]
fn every_post_process_constant_follows_its_source() {
    let Some(s) = setup() else {
        return;
    };
    let c = run(&s);
    assert_eq!(c.camera_candidates, 1, "one GlobalCamera by its m_bgColor");
    let bad = c.bad_constants();
    assert!(bad.is_empty(), "{bad:#?}");
    let checked: usize = c.draws.iter().map(|d| d.constants.len()).sum();
    assert_eq!(checked, 36, "used constants of the 18 post-process draws");
}

#[test]
fn two_runs_differ_only_inside_the_scene() {
    let Some(s) = setup() else {
        return;
    };
    let Some(b) = &s.b else {
        eprintln!("no capture B (FFX_POSTFX_CAPTURE_B): skipped");
        return;
    };
    let cmp = ffx_drawdiff::diff::compare(&s.a, b, &ffx_drawdiff::diff::Options::default());
    let frame = &s.a.frames[0];
    let outside: Vec<String> = cmp.frames[0]
        .diffs
        .iter()
        .filter(|d| !d.a.is_some_and(|i| frame.items[i].scene))
        .map(|d| format!("{:?} {} {}", d.a, d.class, d.field))
        .collect();
    assert!(outside.is_empty(), "{outside:#?}");
}
