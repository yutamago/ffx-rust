//! The character-world draws of captures against their scene inputs
//! (`docs/formats/scene-chars.md`): `charcheck <capture.ffxgfx>...` with
//! `FFX_GAME_DIR` set.
use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS};
use ffx_scene::chars_check;

fn main() {
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let scenes: Vec<String> = paths
        .iter()
        .filter_map(|p| chars_check::scene_of(&std::fs::read(p).ok()?))
        .collect();
    let mut index = AssetIndex::new(TQ_LOW_DROPPED_LEVELS);
    index
        .add_archive(&archive, |p| chars_check::wanted(p, &scenes))
        .expect("index");
    for path in paths {
        let bytes = std::fs::read(&path).expect("read");
        match chars_check::check_capture(&bytes, &index, &archive) {
            Ok(f) => {
                println!("== {path} ==");
                print!("{}", chars_check::report(&f));
                println!("totals {:?}", chars_check::totals(&f));
            }
            Err(e) => println!("== {path}: {e}"),
        }
    }
}
