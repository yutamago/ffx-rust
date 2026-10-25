//! The gate on real captures (`docs/formats/effects-draw.md`, "Gate"):
//! every effect draw of every captured frame traced to its geometry, its
//! list place and its maker, drawn with the shader `vfxShader_fx.txt`
//! gives its flags, and every list entry's draws present.
//!
//! `FFX_FXDRAW_CAPTURES` names folders (`:`-separated) holding
//! `<name>/capture.ffxgfx` with `<name>.snap` beside it (what
//! `tools/replays/draw-fx-*.txt` write); with `FFX_GAME_DIR` set too. Skips
//! when either is unset.

use std::path::{Path, PathBuf};

use ffx_fxdraw::capture::Capture;
use ffx_fxdraw::census;
use ffx_fxdraw::game::{Overlay, State};
use ffx_fxdraw::mem::Mem;
use ffx_fxdraw::shaders::Families;

fn pairs(dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in entries.flatten() {
        let p = e.path();
        let snap = p.with_extension("snap");
        if p.join("capture.ffxgfx").is_file() && snap.is_file() {
            out.push((p, snap));
        }
    }
    out.sort();
    out
}

#[test]
fn every_effect_draw_is_traced() {
    let (Some(dirs), Some(game)) = (
        std::env::var_os("FFX_FXDRAW_CAPTURES"),
        std::env::var_os("FFX_GAME_DIR"),
    ) else {
        eprintln!("skipped: FFX_FXDRAW_CAPTURES or FFX_GAME_DIR unset");
        return;
    };
    let fam = Families::from_game(Path::new(&game)).expect("archive");
    assert!(!fam.vsp.is_empty(), "vfxShader_fx.txt read");
    let mut frames = 0;
    let mut effects = 0;
    let mut failures = Vec::new();
    for dir in std::env::split_paths(&dirs) {
        for (cap, snap) in pairs(&dir) {
            let c = Capture::read(&cap).expect("capture");
            let m = Mem::read(&snap).expect("snapshot");
            let overlay = Overlay::from_game(&m, Path::new(&game));
            let r = census::census(&c, 0, &m, &fam, overlay.as_ref());
            frames += 1;
            effects += r.count("vfx");
            let state = State::read(&m).unwrap_or_default();
            if !r.traced() || !r.missing.is_empty() || !r.orphans.is_empty() {
                failures.push(format!("{}:\n{}", cap.display(), r.report(&state, &m)));
            }
        }
    }
    eprintln!("{frames} frames, {effects} effect draws traced");
    assert!(frames > 0, "no capture pairs found");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
