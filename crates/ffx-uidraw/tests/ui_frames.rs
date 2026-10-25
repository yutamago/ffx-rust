//! The gate of the UI census (`docs/formats/ui-draw.md`): every UI draw of
//! the captured frames is traced. Needs `FFX_GAME_DIR` and
//! `FFX_UIDRAW_CAPTURES` (capture folders or `capture.ffxgfx` files,
//! separated by `;`, e.g. the five of `tools/replays/draw-ui-menu.txt` and
//! `draw-ui-dialogue.txt` under `~/ffx-work/m4/captures/ui-*-a/`); skips
//! when either is unset.

use ffx_uidraw::census::{self, Atlas, Family};
use ffx_uidraw::frame::Capture;

#[test]
fn every_ui_draw_of_the_captured_frames_is_traced() {
    let Some(game) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR unset: skipped");
        return;
    };
    let Ok(list) = std::env::var("FFX_UIDRAW_CAPTURES") else {
        eprintln!("FFX_UIDRAW_CAPTURES unset: skipped");
        return;
    };
    let index = ffx_uidraw::ui_index(std::path::Path::new(&game)).expect("index");
    let mut ui_draws = 0;
    let mut problems = Vec::new();
    for path in list.split(';').filter(|p| !p.trim().is_empty()) {
        let cap = Capture::read(std::path::Path::new(path.trim())).expect("capture");
        assert!(!cap.frames.is_empty(), "{path}: no frame");
        for frame in &cap.frames {
            let (families, ui) = census::census(&cap, &index, frame);
            for (d, f) in frame.iter().zip(&families) {
                if *f == Family::Unknown {
                    problems.push(format!("{path} #{}: a shader of unknown origin", d.item));
                }
            }
            for u in &ui {
                ui_draws += 1;
                if let Err(e) = census::traced(u) {
                    problems.push(format!("{path} #{}: {e}", u.item));
                }
                // The Tk draws sample only archive textures (the frame
                // buffer's copy for the NA shader).
                let tk = !matches!(u.family, Family::Iggy | Family::MenuFrameBuffer);
                for (slot, a) in &u.textures {
                    if tk && !matches!(a, Atlas::Archive { .. }) {
                        problems.push(format!("{path} #{}: t{slot} is {a}", u.item));
                    }
                }
            }
        }
    }
    eprintln!("{ui_draws} UI draws, {} problems", problems.len());
    assert!(ui_draws > 0, "no UI draw in the captures");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn families_by_shader_origin() {
    use census::family;
    assert_eq!(family(true, "x", "menudefaultshader.fx"), Family::Scene);
    assert_eq!(
        family(false, "menudefaultshadernt.fx", "menudefaultshadernt.fx"),
        Family::MenuPlain
    );
    assert_eq!(
        family(false, "abmapshadercone.fx", "optimizedfontshader.fx"),
        Family::Font
    );
    assert_eq!(
        family(false, "FFX.exe@0x00c84a98", "FFX.exe@0x00c6e0d8"),
        Family::Iggy
    );
    assert_eq!(family(false, "FFX.exe@0x00c84f60", "none"), Family::Iggy);
    assert_eq!(family(false, "copybuffer.fx", "none"), Family::Unknown);
    assert_eq!(family(false, "?", "?"), Family::Unknown);
    assert_eq!(
        family(false, "copybuffer.fx", "copybuffer.fx"),
        Family::Other
    );
    assert_eq!(
        family(false, "vfxshapeshader.fx", "vfxshapeshader.fx"),
        Family::Effect
    );
}
