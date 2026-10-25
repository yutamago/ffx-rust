//! The builder from the live snapshot alone, against a capture's map
//! draws: `wirediff <capture.ffxgfx>... [--allow F]... [--limit N]
//! [--out F]`, with `FFX_GAME_DIR` set. Every frame of each capture that
//! has map `renderWorld` records is built from its records
//! (`ffx_scene::live`) and diffed (`ffx_scene::gate::run`); prints the
//! drawdiff report of the scene part and the notes. `--out` writes the
//! last rebuilt frame as a capture.
use ffx_scene::capture::Capture;
use ffx_scene::frame::Settings;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut paths = Vec::new();
    let mut allow = Vec::new();
    let mut limit = 12;
    let mut out = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--allow" => {
                let f = args.next().expect("--allow F");
                let text = std::fs::read_to_string(&f).expect("allow file");
                allow.extend(ffx_drawdiff::rules::parse(&f, &text).expect("rules"));
            }
            "--limit" => limit = args.next().and_then(|n| n.parse().ok()).expect("--limit N"),
            "--out" => out = args.next(),
            _ => paths.push(a),
        }
    }
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let read = |p: &str| archive.find(p).and_then(|e| archive.read(e).ok());
    let ini = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/sandbox/GameSetting.ini"),
    )
    .expect("sandbox GameSetting.ini");
    let settings = Settings::from_ini(&ini).expect("settings");
    let mut bad = 0;
    for path in &paths {
        let bytes = std::fs::read(path).expect("read");
        let records = ffx_scene::inputs::read_wired(&bytes).expect("records");
        for cap in Capture::read_frames(&bytes).expect("capture") {
            let frame: Vec<_> = records
                .iter()
                .filter(|(s, _)| s.frame == cap.frame.number)
                .cloned()
                .collect();
            if frame.is_empty() {
                continue;
            }
            let label = format!("{path} frame {}", cap.frame.number);
            let t = std::time::Instant::now();
            match ffx_scene::gate::run(&cap, &frame, &read, settings, allow.clone(), limit, &label)
            {
                Ok(r) => {
                    println!(
                        "{label}: wire {} captured {:?} built {:?} items {}/{} unallowed {} ({:.2} s)",
                        r.wired,
                        r.captured,
                        r.built,
                        r.a_items,
                        r.b_items,
                        r.unallowed,
                        t.elapsed().as_secs_f64()
                    );
                    for n in &r.notes {
                        println!("  note: {n}");
                    }
                    print!("{}", r.report);
                    bad += r.unallowed;
                    if let Some(o) = &out {
                        std::fs::write(o, ffx_scene::emit::capture_bytes(&r.msgs)).expect("write");
                    }
                }
                Err(e) => {
                    println!("{label}: {e}");
                    bad += 1;
                }
            }
        }
    }
    std::process::exit(i32::from(bad != 0));
}
