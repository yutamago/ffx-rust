//! Prints the platform import census (`docs/formats/platform.md`,
//! "Appendix: import census") as Markdown.
//!
//! ```bash
//! FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo run --release -p ffx-platform --example census
//! ```

use std::path::Path;

fn main() -> Result<(), String> {
    let game = std::env::var_os("FFX_GAME_DIR").ok_or("FFX_GAME_DIR unset")?;
    let exe = std::fs::read(Path::new(&game).join("FFX.exe")).map_err(|e| e.to_string())?;
    let mut image = ffx_platform::Image::parse(exe)?;
    let sym = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../symbols/ffx.sym");
    let names = ffx_platform::parse_sym(&std::fs::read_to_string(sym).map_err(|e| e.to_string())?);
    image.add_starts(names.keys().copied());
    print!("{}", ffx_platform::census_markdown(&image, &names));
    Ok(())
}
