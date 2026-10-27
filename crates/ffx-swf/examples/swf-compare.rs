//! The pilot's comparison page: Iggy (the game's screenshots) against
//! Ruffle (`iggy-replay`'s images over the same background).
//!
//! ```text
//! swf-compare <shots dir> <ruffle dir> <out dir> <page.toml-like spec>
//! ```
//!
//! The spec (one line each, `#` comments):
//!
//! ```text
//! section <title>
//! shot <name> <background shot | -> <caption>
//! anim <name> <background> <shot>,<shot>,...
//! ```
//!
//! For each `shot`: `<shots>/<name>.bmp` (Iggy, over the scene), Ruffle's
//! `<ruffle>/<name>-ruffle.png` put over `<shots>/<background>.bmp` (the
//! scene before the screen opened; `-` for black), and their difference.
//! Writes PNGs and `index.html` into `<out>`; an `anim` writes an animated
//! PNG of the frames side by side (Iggy left, Ruffle right) and a frame
//! strip.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ffx_swf::compose::{difference, over_opaque};
use image::{RgbImage, RgbaImage};

fn load_rgb(path: &Path) -> Result<RgbImage> {
    Ok(image::open(path)
        .with_context(|| path.display().to_string())?
        .to_rgb8())
}

fn ruffle_over(shots: &Path, ruffle: &Path, name: &str, bg: &str) -> Result<RgbImage> {
    let top: RgbaImage = image::open(ruffle.join(format!("{name}-ruffle.png")))
        .with_context(|| format!("{name}-ruffle.png"))?
        .to_rgba8();
    let base = if bg == "-" {
        RgbImage::new(top.width(), top.height())
    } else {
        load_rgb(&shots.join(format!("{bg}.bmp")))?
    };
    Ok(over_opaque(&base, &top))
}

/// An animated PNG of `frames` (RGB, one size), `delay` 1/30 s each.
fn write_apng(path: &Path, frames: &[RgbImage], delay_num: u16, delay_den: u16) -> Result<()> {
    let first = frames.first().context("no frames")?;
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, first.width(), first.height());
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_animated(u32::try_from(frames.len())?, 0)?;
    enc.set_frame_delay(delay_num, delay_den)?;
    let mut w = enc.write_header()?;
    for f in frames {
        w.write_image_data(f.as_raw())?;
    }
    w.finish()?;
    Ok(())
}

fn side_by_side(a: &RgbImage, b: &RgbImage, scale: u32) -> RgbImage {
    let (w, h) = (a.width() / scale, a.height() / scale);
    let sa = image::imageops::resize(a, w, h, image::imageops::FilterType::Triangle);
    let sb = image::imageops::resize(b, w, h, image::imageops::FilterType::Triangle);
    let mut out = RgbImage::new(2 * w + 8, h);
    image::imageops::replace(&mut out, &sa, 0, 0);
    image::imageops::replace(&mut out, &sb, i64::from(w + 8), 0);
    out
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: swf-compare <shots dir> <ruffle dir> <out dir> <spec>";
    let shots = PathBuf::from(args.next().context(usage)?);
    let ruffle = PathBuf::from(args.next().context(usage)?);
    let out = PathBuf::from(args.next().context(usage)?);
    let spec = std::fs::read_to_string(args.next().context(usage)?)?;
    std::fs::create_dir_all(out.join("img"))?;
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>FFX M5 SWF pilot: Iggy against Ruffle</title>\n\
         <style>body{font:14px sans-serif;background:#222;color:#ddd;margin:16px}\
         h2{border-bottom:1px solid #555}figure{display:inline-block;margin:4px;vertical-align:top}\
         img{width:420px;cursor:zoom-in}img.big{width:1280px}figcaption{font-size:12px;color:#aaa}\
         .row{margin-bottom:18px}.stat{font-family:monospace;font-size:12px;color:#9c9}\
         .swap{position:relative;display:inline-block}.swap img.r{position:absolute;left:0;top:0;opacity:0}\
         .swap:hover img.r{opacity:1}</style>\n\
         <h1>FFX M5 SWF pilot: the game's Flash screens, Iggy against Ruffle</h1>\n\
         <p>Left: the original game (Iggy, a harness screenshot). Middle: Ruffle playing the same \
         <code>.swf</code> with the game's recorded Iggy calls (<code>crates/ffx-swf</code>, \
         <code>iggy-replay</code>), drawn over the scene the original showed before the screen opened. \
         Right: the difference (red, 4x; dimmed original underneath). Hover the Ruffle image's \
         \"flip\" copy to swap between the two. Click an image to enlarge it. Notes: <a href=notes.html>notes.html</a>.</p>\n",
    );
    let mut total = Vec::new();
    for line in spec.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (kind, rest) = line.split_once(' ').unwrap_or((line, ""));
        match kind {
            "section" => {
                let _ = writeln!(html, "<h2>{rest}</h2>");
            }
            "shot" => {
                let mut p = rest.splitn(3, ' ');
                let name = p.next().unwrap_or("");
                let bg = p.next().unwrap_or("-");
                let caption = p.next().unwrap_or("");
                let orig = match load_rgb(&shots.join(format!("{name}.bmp"))) {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("{name}: {e:#}");
                        continue;
                    }
                };
                let ruf = ruffle_over(&shots, &ruffle, name, bg)?;
                let (diff, d) = difference(&orig, &ruf);
                orig.save(out.join(format!("img/{name}-iggy.png")))?;
                ruf.save(out.join(format!("img/{name}-ruffle.png")))?;
                diff.save(out.join(format!("img/{name}-diff.png")))?;
                // basis points, then shown as percent
                let bp = |n: u64| n * 10_000 / d.pixels.max(1);
                let stat = format!(
                    "{}.{:02}% of pixels differ by more than 8, {}.{:02}% by more than 32; mean {}.{:03}; Ruffle minus Iggy per channel {:?}/1000",
                    bp(d.over8) / 100,
                    bp(d.over8) % 100,
                    bp(d.over32) / 100,
                    bp(d.over32) % 100,
                    d.mean_milli / 1000,
                    d.mean_milli % 1000,
                    d.bias_milli
                );
                println!("{name}: {stat}");
                total.push((name.to_owned(), stat.clone()));
                let _ = writeln!(
                    html,
                    "<div class=row><b>{name}</b> {caption}<br>\
                     <figure><img src=img/{name}-iggy.png><figcaption>Iggy (original)</figcaption></figure>\
                     <figure><img src=img/{name}-ruffle.png><figcaption>Ruffle</figcaption></figure>\
                     <figure><img src=img/{name}-diff.png><figcaption>difference</figcaption></figure>\
                     <figure><span class=swap><img src=img/{name}-iggy.png><img class=r src=img/{name}-ruffle.png></span>\
                     <figcaption>flip (hover: Ruffle)</figcaption></figure>\
                     <div class=stat>{stat}</div></div>"
                );
            }
            "crop" => {
                // crop <name> <background> <x> <y> <w> <h> <zoom> <caption>
                let p: Vec<&str> = rest.splitn(8, ' ').collect();
                if p.len() < 7 {
                    eprintln!("spec: short crop line {line}");
                    continue;
                }
                let (name, bg) = (p[0], p[1]);
                let n: Vec<u32> = p[2..7].iter().map(|v| v.parse().unwrap_or(1)).collect();
                let (x, y, w, h, zoom) = (n[0], n[1], n[2], n[3], n[4].max(1));
                let caption = p.get(7).copied().unwrap_or("");
                let orig = load_rgb(&shots.join(format!("{name}.bmp")))?;
                let ruf = ruffle_over(&shots, &ruffle, name, bg)?;
                let tag = format!("{name}-{x}-{y}");
                for (img, side) in [(&orig, "iggy"), (&ruf, "ruffle")] {
                    let c = image::imageops::crop_imm(img, x, y, w, h).to_image();
                    let z = image::imageops::resize(
                        &c,
                        w * zoom,
                        h * zoom,
                        image::imageops::FilterType::Nearest,
                    );
                    z.save(out.join(format!("img/crop-{tag}-{side}.png")))?;
                }
                let _ = writeln!(
                    html,
                    "<div class=row><b>{name}</b> {caption} ({w}x{h} at {x},{y}, {zoom}x, nearest)<br>\
                     <figure><img style=width:auto src=img/crop-{tag}-iggy.png><figcaption>Iggy</figcaption></figure>\
                     <figure><img style=width:auto src=img/crop-{tag}-ruffle.png><figcaption>Ruffle</figcaption></figure></div>"
                );
            }
            "anim" => {
                let mut p = rest.splitn(3, ' ');
                let name = p.next().unwrap_or("anim");
                let bg = p.next().unwrap_or("-");
                let frames: Vec<&str> = p.next().unwrap_or("").split(',').collect();
                let mut pairs = Vec::new();
                for f in &frames {
                    let orig = load_rgb(&shots.join(format!("{f}.bmp")))?;
                    let ruf = ruffle_over(&shots, &ruffle, f, bg)?;
                    pairs.push(side_by_side(&orig, &ruf, 2));
                }
                // every other update at 30 a second: 1/15 s per frame; slowed 3x
                write_apng(&out.join(format!("img/{name}.png")), &pairs, 1, 5)?;
                write_apng(&out.join(format!("img/{name}-realtime.png")), &pairs, 1, 15)?;
                // the strip: Iggy row over Ruffle row, quarter size
                let (w, h) = (pairs[0].width() / 2, pairs[0].height() / 2);
                let mut strip = RgbImage::new(w * u32::try_from(pairs.len())?, 2 * h);
                for (i, f) in frames.iter().enumerate() {
                    let orig = load_rgb(&shots.join(format!("{f}.bmp")))?;
                    let ruf = ruffle_over(&shots, &ruffle, f, bg)?;
                    let x = i64::from(w) * i64::try_from(i)?;
                    let so = image::imageops::resize(
                        &orig,
                        w - 4,
                        h,
                        image::imageops::FilterType::Triangle,
                    );
                    let sr = image::imageops::resize(
                        &ruf,
                        w - 4,
                        h,
                        image::imageops::FilterType::Triangle,
                    );
                    image::imageops::replace(&mut strip, &so, x, 0);
                    image::imageops::replace(&mut strip, &sr, x, i64::from(h));
                }
                strip.save(out.join(format!("img/{name}-strip.png")))?;
                let _ = writeln!(
                    html,
                    "<div class=row><b>{name}</b>: {} frames, every other update (Iggy left, Ruffle right)<br>\
                     <figure><img class=big src=img/{name}.png><figcaption>3x slower than the game</figcaption></figure>\
                     <figure><img class=big src=img/{name}-realtime.png><figcaption>at the game's speed</figcaption></figure>\
                     <figure><img class=big src=img/{name}-strip.png><figcaption>the frames: Iggy above, Ruffle below</figcaption></figure></div>",
                    frames.len()
                );
            }
            _ => eprintln!("spec: unknown line {line}"),
        }
    }
    html.push_str(
        "<script>document.querySelectorAll('img').forEach(i=>i.onclick=()=>i.classList.toggle('big'))</script>\n",
    );
    std::fs::write(out.join("index.html"), html)?;
    let mut tsv = String::new();
    for (n, s) in total {
        let _ = writeln!(tsv, "{n}\t{s}");
    }
    std::fs::write(out.join("stats.tsv"), tsv)?;
    Ok(())
}
