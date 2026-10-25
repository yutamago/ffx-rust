//! The post-processing census of a capture (`docs/formats/postfx.md`):
//! `census <capture dir or .ffxgfx> [map folder] [B capture]`, with
//! `FFX_GAME_DIR` set. The map folder (default `/map/cdsp/cdsp00/`, the
//! field-walk-1 frame's) is where the `GlobalCamera` is looked for; with
//! a second capture, the two are compared item by item and every
//! difference outside the scene is listed.
use std::path::Path;

use ffx_postfx::{camera, census, shaders::ShaderNames};

fn brightness(repo: &Path) -> i32 {
    let ini =
        std::fs::read_to_string(repo.join("tools/sandbox/GameSetting.ini")).unwrap_or_default();
    ini.lines()
        .find_map(|l| l.strip_prefix("Brightness=")?.trim().parse().ok())
        .unwrap_or(50)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: census <capture> [map folder] [B capture]");
    let map = args.next().unwrap_or_else(|| "/map/cdsp/cdsp00/".into());
    let other = args.next();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive =
        ffx_vbf::Archive::open(Path::new(&game).join("data/FFX_Data.vbf")).expect("archive");
    let shaders = ShaderNames::from_archive(&archive).expect("shaders");
    let cameras = camera::map_cameras(&archive, &map).expect("cameras");
    let a = ffx_drawdiff::capture::Capture::read(Path::new(&path)).expect("capture");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let inputs = census::Inputs {
        shaders: &shaders,
        cameras: &cameras,
        brightness: brightness(&repo),
    };
    let c = census::census(&a.frames[0], &inputs);
    print!("{}", c.report());
    let bad = c.bad_constants();
    println!("\n== constants without a matching source: {} ==", bad.len());
    for b in bad {
        println!("{b}");
    }
    if let Some(b) = other {
        let b = ffx_drawdiff::capture::Capture::read(Path::new(&b)).expect("B capture");
        let cmp = ffx_drawdiff::diff::compare(&a, &b, &ffx_drawdiff::diff::Options::default());
        let frame = &a.frames[0];
        let mut scene = 0;
        let mut outside = Vec::new();
        for d in &cmp.frames[0].diffs {
            let in_scene = d.a.is_some_and(|i| frame.items[i].scene);
            if in_scene {
                scene += 1;
            } else {
                outside.push(format!("{:?} {} {}", d.a, d.class, d.field));
            }
        }
        println!(
            "\n== A vs B: {scene} differences in scene items, {} outside ==",
            outside.len()
        );
        for o in outside {
            println!("{o}");
        }
    }
}
