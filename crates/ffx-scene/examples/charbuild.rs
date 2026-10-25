//! The character builder against captures: `charbuild <capture.ffxgfx>...
//! [--allow <rules>] [--out <dir>]`, with `FFX_GAME_DIR` set. Builds each
//! capture's character world from its records and the `chr/` models,
//! swaps the captured character draws for the built ones and prints the
//! `ffx-drawdiff` report of the scene (`docs/formats/scene-chars.md`,
//! "The builder"); `--out` writes the rebuilt captures.
use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS};
use ffx_scene::build::Pass;
use ffx_scene::chars_model::ChrIndex;
use ffx_scene::frame::Settings;
use ffx_scene::{chars_check, chars_diff};

#[allow(clippy::too_many_lines)] // a dev tool's one report
fn main() {
    let mut files = Vec::new();
    let mut allow = Vec::new();
    let mut out_dir = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--allow" => {
                let p = args.next().expect("--allow <file>");
                let text = std::fs::read_to_string(&p).expect("allow file");
                allow = ffx_drawdiff::rules::parse(&p, &text).expect("rules");
            }
            "--out" => out_dir = Some(std::path::PathBuf::from(args.next().expect("--out <dir>"))),
            _ => files.push(a),
        }
    }
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let ini = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/sandbox/GameSetting.ini"),
    )
    .expect("sandbox GameSetting.ini");
    let settings = Settings::from_ini(&ini).expect("settings");
    let captures: Vec<Vec<u8>> = files
        .iter()
        .map(|f| std::fs::read(f).expect("capture"))
        .collect();
    let scenes: Vec<String> = captures
        .iter()
        .filter_map(|b| chars_check::scene_of(b))
        .collect();
    let t = std::time::Instant::now();
    let mut index = AssetIndex::new(TQ_LOW_DROPPED_LEVELS);
    index
        .add_archive(&archive, |p| chars_check::wanted(p, &scenes))
        .expect("index");
    let read = |p: &str| archive.find(p).and_then(|e| archive.read(e).ok());
    let models = chars_diff::chr_models(&archive);
    let chr = ChrIndex::new(read, models.iter().map(String::as_str));
    eprintln!(
        "indexes: {:?} ({} chr models)",
        t.elapsed(),
        chr.models.len()
    );
    for (file, bytes) in files.iter().zip(&captures) {
        let t = std::time::Instant::now();
        let o = match chars_diff::run(bytes, &archive, &index, &chr, settings) {
            Ok(o) => o,
            Err(e) => {
                println!("== {file}: {e}");
                continue;
            }
        };
        println!(
            "== {file}: built {} opaque, {} transparent in {:?}; captured {:?} / {:?}",
            o.built.pass(Pass::Opaque).count(),
            o.built.pass(Pass::Transparent).count(),
            t.elapsed(),
            o.opaque,
            o.transparent
        );
        println!("models: {:?}", o.models);
        for d in &o.built.draws {
            println!(
                "  {:?} instance {} segment {} material {}",
                d.pass, d.instance, d.segment, d.material
            );
        }
        for n in &o.built.notes {
            println!("note: {n}");
        }
        let mut lights: Vec<_> = o.lights.iter().collect();
        lights.sort_by_key(|l| *l.0);
        for (i, l) in lights {
            println!("lights of instance {i} (from the capture): {l:x?}");
        }
        println!(
            "run-time material textures (from the capture): {:?}",
            o.material_textures
        );
        println!(
            "words after bone lists taken from the capture: {}; blend index problems: {:?}",
            o.unwritten_words, o.blend_problems
        );
        let a = ffx_drawdiff::capture::Capture::parse(bytes).expect("A");
        let b = ffx_drawdiff::capture::Capture::from_msgs(&o.msgs);
        let opts = ffx_drawdiff::diff::Options {
            part: ffx_drawdiff::diff::Part::Scene,
            tolerances: Vec::new(),
            allow: allow.clone(),
        };
        let cmp = ffx_drawdiff::diff::compare(&a, &b, &opts);
        println!(
            "{}",
            ffx_drawdiff::report::render(
                (file, "built"),
                &a,
                &b,
                &cmp,
                &opts,
                ffx_drawdiff::report::Style::default(),
            )
        );
        println!("unallowed: {}", cmp.unallowed());
        if let Some(dir) = &out_dir {
            std::fs::create_dir_all(dir).expect("out dir");
            let name = std::path::Path::new(file)
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .map_or_else(|| "capture".into(), |n| n.to_string_lossy().into_owned());
            let path = dir.join(format!("{name}.built.ffxgfx"));
            std::fs::write(&path, ffx_scene::emit::capture_bytes(&o.msgs)).expect("write");
            println!("wrote {}", path.display());
        }
    }
}
