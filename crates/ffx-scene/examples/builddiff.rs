//! Builds the map world of a capture's frame and diffs it against the
//! captured map draws: `builddiff <capture.ffxgfx> [out.ffxgfx]
//! [--allow F] [--limit N]`, with `FFX_GAME_DIR` set (cdsp0000 only for
//! now). Prints the drawdiff report of the scene part and the builder's
//! notes; writes the rebuilt capture when asked.
use ffx_scene::build::{Clusters, build};
use ffx_scene::capture::Capture;
use ffx_scene::capture_inputs::CaptureInputs;
use ffx_scene::frame::{Settings, TextureQuality};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: builddiff <capture> [out] [--allow F] [--limit N]");
    let mut out = None;
    let mut allow = Vec::new();
    let mut limit = 12;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--allow" => {
                let f = args.next().expect("--allow F");
                let text = std::fs::read_to_string(&f).expect("allow file");
                allow.extend(ffx_drawdiff::rules::parse(&f, &text).expect("rules"));
            }
            "--limit" => limit = args.next().and_then(|n| n.parse().ok()).expect("--limit N"),
            _ => out = Some(a),
        }
    }
    let cap = Capture::read(&std::fs::read(&path).expect("read")).expect("capture");
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let read = |p: &str| archive.find(p).and_then(|e| archive.read(e).ok());
    let t = std::time::Instant::now();
    let clusters = Clusters::load(
        read,
        "ffx_data/gamedata/ps3data/map/cdsp/cdsp00/mdl/d3d11/cdsp00.dae.phyre",
    )
    .expect("clusters");
    eprintln!("clusters in {:.2} s", t.elapsed().as_secs_f64());
    let settings = Settings {
        texture_quality: TextureQuality::Low,
        anisotropy: 1.0,
    };
    let inputs = CaptureInputs::new(&cap, &clusters, settings, false).expect("inputs");
    eprintln!(
        "matched {} map draws; hidden instances {:?}",
        inputs.matched.len(),
        inputs
            .hidden
            .iter()
            .map(|&i| clusters.map.instances[i].name.clone())
            .collect::<Vec<_>>()
    );
    let t = std::time::Instant::now();
    let built = build(&clusters, &inputs).expect("build");
    eprintln!(
        "built {} draws in {:.3} s",
        built.draws.len(),
        t.elapsed().as_secs_f64()
    );
    for n in &built.notes {
        println!("note: {n}");
    }
    let replaced: Vec<usize> = inputs.matched.iter().map(|m| m.number).collect();
    let msgs =
        ffx_scene::emit::replace_map_draws(&cap, &built, &replaced, &|e| inputs.external_view(e))
            .expect("emit");
    let bytes = ffx_scene::emit::capture_bytes(&msgs);
    if let Some(o) = out {
        std::fs::write(&o, &bytes).expect("write");
    }
    let a = ffx_drawdiff::capture::Capture::read(std::path::Path::new(&path)).expect("A");
    let b = ffx_drawdiff::capture::Capture::parse(&bytes).expect("B");
    let opts = ffx_drawdiff::diff::Options {
        part: ffx_drawdiff::diff::Part::Scene,
        tolerances: Vec::new(),
        allow,
    };
    let cmp = ffx_drawdiff::diff::compare(&a, &b, &opts);
    let style = ffx_drawdiff::report::Style {
        limit,
        ..Default::default()
    };
    print!(
        "{}",
        ffx_drawdiff::report::render((&path, "built"), &a, &b, &cmp, &opts, style)
    );
}
