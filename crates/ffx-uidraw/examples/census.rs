//! The UI census of captures (`docs/formats/ui-draw.md`):
//! `census <capture>... [--constants]`, with `FFX_GAME_DIR` set. Per
//! capture: one line per UI draw (family, target, textures, quads, states),
//! the families of every draw, and with `--constants` the used constants
//! of each distinct UI shader pair (first draw, then which words vary).
use std::collections::BTreeMap;

use ffx_uidraw::census;
use ffx_uidraw::frame::Capture;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let show_constants = args.iter().any(|a| a == "--constants");
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let index = ffx_uidraw::ui_index(std::path::Path::new(&game)).expect("index");
    for path in args.iter().filter(|a| !a.starts_with("--")) {
        let cap = Capture::read(std::path::Path::new(path)).expect("capture");
        println!("== {path} (scene {})", cap.scene);
        for frame in &cap.frames {
            let (families, ui) = census::census(&cap, &index, frame);
            print!("{}", census::report(&families, &ui));
            if !show_constants {
                continue;
            }
            let mut pairs: BTreeMap<(String, String), Vec<&census::UiDraw>> = BTreeMap::new();
            for u in &ui {
                pairs
                    .entry((u.vs.clone(), u.ps.clone()))
                    .or_default()
                    .push(u);
            }
            for ((vs, ps), draws) in pairs {
                println!("\nconstants of vs {vs} ps {ps} ({} draws):", draws.len());
                for (n, (name, vals)) in draws[0].constants.iter().enumerate() {
                    let varies = draws.iter().any(|d| {
                        d.constants.get(n).is_none_or(|c| {
                            c.1.iter()
                                .map(|f| f.to_bits())
                                .ne(vals.iter().map(|f| f.to_bits()))
                        })
                    });
                    println!(
                        "  {name}{} = {vals:?}",
                        if varies { " (varies)" } else { "" }
                    );
                }
            }
        }
    }
}
