//! The character world's draws against the scene inputs' character part
//! (`docs/formats/scene-chars.md`, "Check"): every draw of a character
//! model or the blob shadow is tied to an (instance, segment) of the
//! record, and its material and per-node constants (`BoneTransforms`,
//! `World`, `WorldView`, `WorldViewProjection`, the character lighting,
//! `ClothWorld`, `SgSpec_*`, ...), its cloth and shadow vertices and the
//! pass's culling and order come out of the record bit for bit.
//!
//! Needs `FFX_GAME_DIR` and `FFX_SCENE_CHARS` (capture files of
//! `tools/replays/draw-cdsp0000.txt` / `draw-bsil0000.txt` taken with the
//! character inputs, separated by `:`); skips when either is unset.

use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS};
use ffx_scene::chars::Tie;
use ffx_scene::chars_check;

/// `WorldInverse` and `WorldViewInverse` of some Besaid NPCs come out up
/// to this many ulp off (open item 1 of docs/formats/scene-chars.md);
/// everything else is exact.
const INVERSE_ULP: u32 = 4;

#[test]
fn the_allow_list_parses() {
    let rules = ffx_drawdiff::rules::parse("chars-allow.txt", include_str!("chars-allow.txt"))
        .expect("allow list");
    assert_eq!(rules.len(), 2);
}

#[test]
fn inputs_reproduce_the_character_draws() {
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
    for (file, bytes) in files.iter().zip(&captures) {
        let f = chars_check::check_capture(bytes, &index, &archive).expect("check");
        let report = chars_check::report(&f);
        eprintln!("{}:\n{report}", file.display());
        assert!(!f.draws.is_empty(), "no character draws");
        assert!(f.unchecked.is_empty(), "unchecked draws: {:?}", f.unchecked);
        for c in &f.draws {
            let d = &c.draw;
            assert_ne!(d.tie, Tie::None, "draw {} not tied", d.number);
            for x in &d.checked {
                let inverse = matches!(x.name.as_str(), "WorldInverse" | "WorldViewInverse");
                assert!(
                    x.exact || (inverse && x.max_ulp <= INVERSE_ULP),
                    "draw {} {} {}: {:?}, {} ulp",
                    d.number,
                    x.stage,
                    x.name,
                    x.first_diff,
                    x.max_ulp
                );
            }
            for v in &c.vertices {
                assert!(v.equal, "draw {}: {} differ", d.number, v.what);
            }
        }
        assert_eq!(f.order.len(), 2, "both passes");
        for o in &f.order {
            assert!(o.in_order, "pass {}: {o:?}", o.pass);
        }
        let (n, exact, nv, equal) = chars_check::totals(&f);
        eprintln!("{n} constants, {exact} exact; {nv} vertex comparisons, {equal} equal");
        assert!(n > 1000);
    }
}
