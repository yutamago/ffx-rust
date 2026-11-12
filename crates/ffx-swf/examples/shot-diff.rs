//! Ruffle's images (`iggy-replay`) against a replay's recorded screenshots,
//! counted as the replays count them (`docs/host.md`, "Iggy's screens
//! drawn"):
//!
//! ```text
//! shot-diff <shots dir> <ruffle dir> <background shot> [--out <dir>] <shot>...
//! ```
//!
//! Each `<ruffle>/<shot>-ruffle.png` (straight alpha) is put over
//! `<shots>/<background shot>.bmp` (the scene the menu covers; `-` for
//! black) and compared with `<shots>/<shot>.bmp`: one line per shot with the
//! pixels whose largest channel differs by more than 24 and by more than 8
//! levels, and the mean channel difference. `--out` keeps the composite and
//! the difference image of each.

use std::path::PathBuf;

use anyhow::{Context, Result};
use ffx_swf::compose::{difference, over_opaque};
use image::RgbImage;

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut out: Option<PathBuf> = None;
    if let Some(i) = args.iter().position(|a| a == "--out") {
        out = args.get(i + 1).map(PathBuf::from);
        args.drain(i..(i + 2).min(args.len()));
    }
    let mut it = args.into_iter();
    let usage = "shot-diff <shots> <ruffle> <background> [--out D] <shot>...";
    let shots = PathBuf::from(it.next().context(usage)?);
    let ruffle = PathBuf::from(it.next().context(usage)?);
    let bg = it.next().context(usage)?;
    if let Some(o) = &out {
        std::fs::create_dir_all(o)?;
    }
    println!("shot\tover24\tover8\tmean");
    for name in it {
        let orig = image::open(shots.join(format!("{name}.bmp")))
            .with_context(|| format!("{name}.bmp"))?
            .to_rgb8();
        let top = image::open(ruffle.join(format!("{name}-ruffle.png")))
            .with_context(|| format!("{name}-ruffle.png"))?
            .to_rgba8();
        let base = if bg == "-" {
            RgbImage::new(orig.width(), orig.height())
        } else {
            image::open(shots.join(format!("{bg}.bmp")))?.to_rgb8()
        };
        let comp = over_opaque(&base, &top);
        let (img, d) = difference(&orig, &comp);
        println!(
            "{name}\t{}\t{}\t{}.{:03}",
            d.over24,
            d.over8,
            d.mean_milli / 1000,
            d.mean_milli % 1000
        );
        if let Some(o) = &out {
            comp.save(o.join(format!("{name}-ruffle-over.png")))?;
            img.save(o.join(format!("{name}-diff.png")))?;
        }
    }
    Ok(())
}
