//! Scratch: what the scene draws of a capture bind and what matches.
use ffx_scene::{assets::AssetIndex, capture::Capture};

fn main() {
    let mut args = std::env::args().skip(1);
    let cap = Capture::read(&std::fs::read(args.next().expect("capture")).expect("read")).unwrap();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive =
        ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf")).unwrap();
    let prefixes: Vec<String> = args.collect();
    let index = AssetIndex::build(&archive, |p| {
        p.contains("/shaders/d3d11/") || prefixes.iter().any(|x| p.contains(x.as_str()))
    })
    .unwrap();
    eprintln!("read {} failed {}", index.read, index.failed.len());
    for (p, why) in &index.failed {
        eprintln!("failed {p}: {why}");
    }
    // FFX_EXPLORE_DUMP=dir: every unmatched static scene buffer as dir/<id>.bin
    if let Ok(dir) = std::env::var("FFX_EXPLORE_DUMP") {
        for (_, d) in cap.draws() {
            for id in d.vertex.iter().map(|v| v.1).chain([d.index.0]) {
                if let Some((desc, data)) = cap.buffer(id)
                    && desc[1] != 2
                    && index.get(ffx_scene::key::buffer(data)).is_empty()
                {
                    let _ = std::fs::write(format!("{dir}/{id:x}.bin"), data);
                }
            }
        }
    }
    for (i, d) in cap.draws() {
        let scene = Capture::scene_targets(d).is_some();
        let sh = |id| cap.shader(id).map(ffx_scene::key::shader).unwrap_or(0);
        let fam = |id| {
            index
                .shader_cluster
                .get(&sh(id))
                .cloned()
                .unwrap_or_else(|| "?".into())
        };
        let mut line = format!(
            "{i} scene={scene} n={} vs={} ps={}",
            d.count,
            fam(d.vs),
            fam(d.ps)
        );
        for v in &d.vertex {
            let (desc, data) = cap.buffer(v.1).unwrap();
            let k = ffx_scene::key::buffer(data);
            let m = index.get(k);
            line += &format!(
                " vb{}[u{} {}B s{}]={}",
                v.0,
                desc[1],
                desc[0],
                v.2,
                m.first().map_or("-".to_string(), |s| format!("{s:?}")
                    .chars()
                    .take(90)
                    .collect())
            );
        }
        if d.index.0 != 0 {
            let (desc, data) = cap.buffer(d.index.0).unwrap();
            let m = index.get(ffx_scene::key::buffer(data));
            line += &format!(" ib[u{} {}B]={}", desc[1], desc[0], m.len());
        }
        if let Some(v) = d.vertex.first()
            && let Some((_, data)) = cap.buffer(v.1)
            && let Some(ffx_scene::assets::Source::Vertices { model, segment, .. }) =
                index.get(ffx_scene::key::buffer(data)).first()
            && let Some(info) = index.segments.get(&(model.clone(), *segment))
        {
            line += &format!(" seg{segment} inst={:?} skinned={}", info.instances, info.skinned);
            if let Some(m) = &info.material {
                for t in &m.textures {
                    line += &format!(" {}={}", t.sampler, t.archive_path());
                }
            }
        }
        for s in &d.ps_srv {
            match cap.view_texture(s.1) {
                Some(t) => {
                    let m = t.key().map(|k| index.get(k).len());
                    line += &format!(
                        " t{}[{}x{} f{} m{}]={:?}",
                        s.0, t.desc[0], t.desc[1], t.desc[4], t.desc[2], m
                    );
                }
                None => line += &format!(" t{}=?", s.0),
            }
        }
        println!("{line}");
    }
}
