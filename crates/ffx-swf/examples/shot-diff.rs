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
//! the difference image of each; `--crop x,y,w,h` (with `--out`) also
//! writes `<shot>-crop.png`: that rectangle of the original, Ruffle's
//! composite and the difference, one above the other, at 3x.

use std::path::PathBuf;

use anyhow::{Context, Result};
use ffx_swf::compose::{difference, over_opaque};
use image::RgbImage;

#[allow(clippy::many_single_char_names)]
fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut out: Option<PathBuf> = None;
    if let Some(i) = args.iter().position(|a| a == "--out") {
        out = args.get(i + 1).map(PathBuf::from);
        args.drain(i..(i + 2).min(args.len()));
    }
    let mut crop: Option<[u32; 4]> = None;
    if let Some(i) = args.iter().position(|a| a == "--crop") {
        let v: Vec<u32> = args
            .get(i + 1)
            .map(|s| s.split(',').filter_map(|n| n.parse().ok()).collect())
            .unwrap_or_default();
        crop = <[u32; 4]>::try_from(v).ok();
        args.drain(i..(i + 2).min(args.len()));
    }
    let mut it = args.into_iter();
    let usage = "shot-diff <shots> <ruffle> <background> [--out D [--crop x,y,w,h]] <shot>...";
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
            comp.save(o.join(format!("{name}-ruffle-over.bmp")))?;
            img.save(o.join(format!("{name}-diff.png")))?;
            if let Some([x, y, w, h]) = crop {
                const K: u32 = 3;
                let mut strip = RgbImage::new(w * K, h * 3 * K);
                for (row, src) in [&orig, &comp, &img].into_iter().enumerate() {
                    let part = image::imageops::crop_imm(src, x, y, w, h).to_image();
                    let big =
                        image::imageops::resize(&part, w * K, h * K, image::imageops::Nearest);
                    let top = i64::from(h * K) * i64::try_from(row)?;
                    image::imageops::replace(&mut strip, &big, 0, top);
                }
                strip.save(o.join(format!("{name}-crop.png")))?;
                println!(
                    "  crop: best shift of Ruffle's {}",
                    best_shift(&orig, &comp, crop)
                );
            }
        }
    }
    Ok(())
}

/// The whole-pixel shift (-3..3 each way) of `b` that makes the crop most
/// like `a` (sum of absolute channel differences), with the sums unshifted
/// and shifted: a misplaced row or glyph shows as a non-zero shift.
#[allow(clippy::many_single_char_names)]
fn best_shift(a: &RgbImage, b: &RgbImage, crop: Option<[u32; 4]>) -> String {
    let Some([x, y, w, h]) = crop else {
        return String::new();
    };
    let sad = |dx: i64, dy: i64| -> u64 {
        let mut s = 0u64;
        for j in y..y + h {
            for i in x..x + w {
                let (bi, bj) = (i64::from(i) - dx, i64::from(j) - dy);
                let (Ok(bi), Ok(bj)) = (u32::try_from(bi), u32::try_from(bj)) else {
                    continue;
                };
                if bi >= b.width() || bj >= b.height() {
                    continue;
                }
                let (pa, pb) = (a.get_pixel(i, j), b.get_pixel(bi, bj));
                s += (0..3)
                    .map(|c| u64::from(pa[c].abs_diff(pb[c])))
                    .sum::<u64>();
            }
        }
        s
    };
    let base = sad(0, 0);
    let mut best = (base, 0, 0);
    for dy in -3..=3 {
        for dx in -3..=3 {
            let s = sad(dx, dy);
            if s < best.0 {
                best = (s, dx, dy);
            }
        }
    }
    format!("({}, {}): {} -> {}", best.1, best.2, base, best.0)
}
