//! Asset parity on a capture of field-walk-1's frame (`docs/formats/scene.md`):
//! every static object its scene draws use is made from the archive (or,
//! for the characters, from the loose-file mods the harness instance
//! loaded), and the frame follows the sort and matrix rules.
//!
//! Needs `FFX_GAME_DIR` and `FFX_SCENE_CAPTURE` (a `capture.ffxgfx` of the
//! cdsp0000 frame, e.g. `~/ffx-work/m4/captures/fw1-a/capture.ffxgfx`);
//! skips when either is unset. `FFX_SCENE_MODS` names the instance's
//! `mods/` folder when the capture loaded mods; without it, objects of
//! the character draws may stay unexplained and are only listed.

use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS, is_shader_cluster};
use ffx_scene::capture::Capture;
use ffx_scene::census::{self, Verdict};

fn setup() -> Option<(Capture, AssetIndex, bool)> {
    let game = std::env::var_os("FFX_GAME_DIR")?;
    let Some(capture) = std::env::var_os("FFX_SCENE_CAPTURE") else {
        eprintln!("FFX_SCENE_CAPTURE unset: skipped");
        return None;
    };
    let cap = Capture::read(&std::fs::read(capture).expect("capture")).expect("capture");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let mut index = AssetIndex::new(TQ_LOW_DROPPED_LEVELS);
    // The map, the characters on the deck (Tidus and his face object) and
    // every shader.
    let folders = ["/map/cdsp/cdsp00/", "/chr/pc/c001/", "/chr/obj/f001/"];
    index
        .add_archive(&archive, |p| {
            is_shader_cluster(p) || folders.iter().any(|f| p.contains(f))
        })
        .expect("index");
    let mods = std::env::var_os("FFX_SCENE_MODS");
    if let Some(m) = &mods {
        index.add_mods(std::path::Path::new(m)).expect("mods");
    }
    Some((cap, index, mods.is_some()))
}

#[test]
fn every_static_object_of_the_frame_comes_from_the_data() {
    let Some((cap, index, mods)) = setup() else {
        return;
    };
    let draws = census::census(&cap, &index);
    assert_eq!(draws.len(), 128, "scene draws of the field-walk-1 frame");
    let mut unexplained = Vec::new();
    for d in &draws {
        let character = d.ps_family.starts_with("phyrechrlitshader");
        for b in &d.bound {
            let ok = !matches!(b.verdict, Verdict::Unexplained | Verdict::Texels(_));
            if !ok && (mods || !character) {
                unexplained.push(format!("draw {} {} {:?}", d.number, b.role, b.verdict));
            }
            if !ok && character && !mods {
                eprintln!(
                    "character draw {} {}: not in the archive (a mod?)",
                    d.number, b.role
                );
            }
        }
    }
    assert!(unexplained.is_empty(), "{unexplained:#?}");
}

#[test]
fn draw_order_and_derived_matrices_follow_the_rules() {
    let Some((cap, index, _)) = setup() else {
        return;
    };
    let draws = census::census(&cap, &index);
    let (checked, bad, notes) = census::sort_check(&draws);
    assert!(checked > 10, "{checked} pairs");
    assert_eq!(bad, 0, "{notes:#?}");
    let checks = census::matrix_checks(&cap);
    for rule in [
        "WorldView = View . World (reversed)",
        "WorldViewProjection = ViewProjection . World (reversed, f64 sums)",
        "ViewProjection = Projection . View (reversed)",
    ] {
        let (n, exact, _) = checks[rule];
        assert!(n > 100 && exact == n, "{rule}: {exact} of {n}");
    }
}
