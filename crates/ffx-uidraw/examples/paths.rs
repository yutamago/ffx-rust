//! Lists archive paths containing every given word: `paths <word>...`,
//! with `FFX_GAME_DIR` set (finding the clusters a UI draw samples).
fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    let game = std::env::var("FFX_GAME_DIR").expect("FFX_GAME_DIR");
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    for e in archive.entries() {
        if words.iter().all(|w| e.path.contains(w.as_str())) {
            println!("{} {}", e.size, e.path);
        }
    }
}
