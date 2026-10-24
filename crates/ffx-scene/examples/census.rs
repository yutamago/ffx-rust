//! The census of a capture's scene draws (`docs/formats/scene.md`):
//! `census <capture.ffxgfx> [archive path filter...]`, with `FFX_GAME_DIR`
//! set. The filters choose which clusters of `FFX_Data.vbf` are indexed
//! besides the shaders (default: the map's and the characters' folders of
//! the field-walk-1 frame).
use ffx_scene::{assets::AssetIndex, capture::Capture, census};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: census <capture.ffxgfx> [filter...]");
    let cap = Capture::read(&std::fs::read(path).expect("read")).expect("capture");
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    // `@file`: one filter per line.
    let mut filters: Vec<String> = args
        .flat_map(|a| match a.strip_prefix('@') {
            Some(f) => std::fs::read_to_string(f)
                .expect("filter file")
                .lines()
                .map(str::to_owned)
                .collect(),
            None => vec![a],
        })
        .collect();
    if filters.is_empty() {
        filters = vec!["/map/cdsp/cdsp00/".into(), "/ps3data/chr/".into()];
    }
    let mut index = AssetIndex::new(ffx_scene::assets::TQ_LOW_DROPPED_LEVELS);
    index
        .add_archive(&archive, |p| {
            ffx_scene::assets::is_shader_cluster(p)
                || filters.iter().any(|f| p.contains(f.as_str()))
        })
        .expect("index");
    // FFX_SCENE_MODS=<game>/mods: the loose-file mods the instance loaded.
    if let Ok(mods) = std::env::var("FFX_SCENE_MODS") {
        index.add_mods(std::path::Path::new(&mods)).expect("mods");
    }
    let draws = census::census(&cap, &index);
    print!("{}", census::report(&draws));
    println!("\n== culling ==");
    for line in census::cull_check(&cap, &draws, &index) {
        println!("{line}");
    }
    let (checked, bad, notes) = census::sort_check(&draws);
    println!("\n== sort rule: {checked} neighbouring instance pairs, {bad} out of order ==");
    for n in notes {
        println!("{n}");
    }
    println!("\n== derived matrices (draws, bit-exact, largest ULP) ==");
    for (name, (n, exact, u)) in census::matrix_checks(&cap) {
        println!("{name}\t{n}\t{exact}\t{u}");
    }
}
