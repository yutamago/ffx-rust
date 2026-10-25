//! Dumps a model's materials: `material <model path filter> [n...]`.
#![allow(clippy::many_single_char_names)] // a dump tool's short names
use ffx_scene::phyre::Graph;

fn main() {
    let mut args = std::env::args().skip(1);
    let filter = args.next().expect("usage: material <filter> [n...]");
    let want: Vec<u32> = args.filter_map(|a| a.parse().ok()).collect();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let e = archive
        .entries()
        .iter()
        .find(|e| e.path.contains(filter.as_str()) && e.path.ends_with(".dae.phyre"))
        .expect("model");
    let g = Graph::parse(archive.read(e).expect("read")).expect("graph");
    let imports = g.imports();
    for (i, m) in (0u32..).zip(g.objects("PMaterial")) {
        if !want.is_empty() && !want.contains(&i) {
            continue;
        }
        let c = "PMaterial";
        let fx = g
            .ptr(m, c, "m_effectVariant", 0)
            .and_then(|t| g.import_index(t))
            .and_then(|k| imports.get(k));
        println!(
            "material {i} {:?} fx {fx:?} from {:?} to {:?} texanim {:?}",
            m,
            g.user_text(m, c, "m_remapFrom"),
            g.user_text(m, c, "m_remapTo"),
            g.string(m, c, "m_texAnimID"),
        );
        println!(
            "  raw ptrs {:?}",
            g.pointers[m.list]
                .iter()
                .filter(|f| f.object() == m.index)
                .map(|f| f.0)
                .collect::<Vec<_>>()
        );
        if let Some(b) = g.target_obj(m, c, "m_parameterBuffer") {
            println!(
                "  buffer list {} size {}",
                b.list,
                g.c.lists[b.list].object_size()
            );
            for f in &g.pointers[b.list] {
                let t = f
                    .user_fixup()
                    .map(|u| g.c.user_fixups[u as usize].data.clone());
                let extra = if g.class_of(f.target_list() as usize) == "PSamplerState" {
                    let o = g.obj(f.target_list() as usize, f.target());
                    format!(
                        " {:08x?}",
                        (0..9)
                            .map(|k| g.word_at(o.at + 4 * k).unwrap_or(0))
                            .collect::<Vec<_>>()
                    )
                } else {
                    String::new()
                };
                println!(
                    "    ptr {:x?} {}{t:x?}{extra}",
                    f.0,
                    g.class_of(f.target_list() as usize)
                );
            }
        }
    }
}
