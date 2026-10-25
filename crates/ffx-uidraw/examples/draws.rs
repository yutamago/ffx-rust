//! Lists a capture's draws with their shaders' origins:
//! `draws <capture> [archive path filter...]`, with `FFX_GAME_DIR` set.
//! One line per draw: item number, scene flag, target role, kind, count,
//! vertex and pixel shader origin, textures (archive file and level, or
//! the texture's size and format).
use ffx_uidraw::frame::Capture;
use ffx_uidraw::index::Index;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: draws <capture> [filter...]");
    let filters: Vec<String> = args.collect();
    let cap = Capture::read(std::path::Path::new(&path)).expect("capture");
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let game = std::path::Path::new(&game);
    let archive = ffx_vbf::Archive::open(game.join("data/FFX_Data.vbf")).expect("archive");
    let mut index = Index::default();
    index
        .add_archive(&archive, |p| filters.iter().any(|f| p.contains(f.as_str())))
        .expect("index");
    index.add_image(
        "FFX.exe",
        &std::fs::read(game.join("FFX.exe")).expect("FFX.exe"),
    );
    println!("scene {}", cap.scene);
    for (f, frame) in cap.frames.iter().enumerate() {
        println!("frame {f}: {} draws", frame.len());
        for d in frame {
            let tex: Vec<String> = d
                .ps_textures
                .iter()
                .map(|&(s, id)| {
                    let Some(t) = cap.textures.get(&id) else {
                        return format!("t{s}=?");
                    };
                    let src = t
                        .content()
                        .and_then(|k| index.textures.get(&k))
                        .and_then(|l| l.first())
                        .map_or_else(
                            || {
                                format!(
                                    "{}x{}f{}{}",
                                    t.width(),
                                    t.height(),
                                    t.format(),
                                    if t.target { "rt" } else { "" }
                                )
                            },
                            |s| format!("{}@{}", s.path, s.level),
                        );
                    format!("t{s}={src}")
                })
                .collect();
            println!(
                "{:04} {} rt{} {:?} {} vs {} ps {} {}",
                d.item,
                if d.scene { "S" } else { "-" },
                d.target.map_or(-1, |t| i64::from(t.1)),
                d.draw.kind,
                d.draw.count,
                index.vertex_shader(d.vs, d.ps),
                index.shader(d.ps),
                tex.join(" ")
            );
        }
    }
}
