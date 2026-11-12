//! The game's movies' AS3 interface (`ffx_swf::abc`):
//!
//! ```text
//! swf-abc [--vbf <FFX_Data.vbf>] [<movie.swf> [<Class> <method>]]
//! ```
//!
//! Without a movie: for each `.swf` of the player's `FFX_Data.vbf`
//! (`FFX_GAME_DIR/data/FFX_Data.vbf` by default) the functions it offers the
//! game (`ExternalInterface.addCallback`, with their argument counts:
//! `name/n`, or `name/required-most`) and the ones it calls in the game (`ExternalInterface.call`).
//! With a movie name: the same for that movie and every class method's
//! signature. With a class and a method: that method's instructions.

use std::path::PathBuf;

use anyhow::{Context, Result};
use ffx_swf::abc::{Movie, blocks};

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut vbf =
        std::env::var_os("FFX_GAME_DIR").map(|d| PathBuf::from(d).join("data/FFX_Data.vbf"));
    if let Some(i) = args.iter().position(|a| a == "--vbf") {
        vbf = args.get(i + 1).map(PathBuf::from);
        args.drain(i..(i + 2).min(args.len()));
    }
    let vbf = vbf.context("no FFX_Data.vbf (set FFX_GAME_DIR or pass --vbf)")?;
    let movies = ffx_swf::flash_movies(&vbf)?;
    let pick = args.first().map(|s| s.to_ascii_lowercase());
    for (name, data) in &movies {
        if pick.as_ref().is_some_and(|p| p != name) {
            continue;
        }
        if let (Some(class), Some(method)) = (args.get(1), args.get(2)) {
            for abc in blocks(data)? {
                print!("{}", abc.disassemble(class, method));
            }
            continue;
        }
        let movie = Movie::from_swf(data)?;
        println!(
            "{name} ({} bytes, {} methods)",
            data.len(),
            movie.methods.len()
        );
        let offered: Vec<String> = movie
            .callbacks
            .iter()
            .map(|c| match movie.arity(c) {
                Some(a) if a.most == Some(a.required) => format!("{c}/{}", a.required),
                Some(a) => format!(
                    "{c}/{}-{}",
                    a.required,
                    a.most.map_or_else(|| "...".to_owned(), |m| m.to_string())
                ),
                None => c.clone(),
            })
            .collect();
        println!("  offers: {}", offered.join(" "));
        let calls: Vec<&str> = movie.calls.iter().map(String::as_str).collect();
        println!("  calls:  {}", calls.join(" "));
        if pick.is_some() {
            for m in &movie.methods {
                println!(
                    "  {}/{}({}) required {}{}",
                    m.owner,
                    m.name,
                    m.types.join(", "),
                    m.required,
                    if m.rest { " ...rest" } else { "" }
                );
            }
        }
    }
    Ok(())
}
