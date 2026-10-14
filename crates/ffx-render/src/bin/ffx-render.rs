//! `ffx-render replay <capture.ffxgfx> <out dir> [--until N] [--target ID]`:
//! draws a captured frame and writes every 8-bit colour render target it
//! drew into as `<out dir>/<view id>.png`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("replay") => replay(&args[1..]),
        _ => bail!("usage: ffx-render replay <capture.ffxgfx> <out dir> [--until N] [--target ID]"),
    }
}

fn parse_id(s: &str) -> Result<u32> {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).context("hex id")
}

fn replay(args: &[String]) -> Result<()> {
    let capture = PathBuf::from(args.first().context("capture file")?);
    let out = PathBuf::from(args.get(1).context("out dir")?);
    let mut filter = ffx_render::Filter::default();
    let mut dump = Vec::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--until" => {
                filter.until = Some(args.get(i + 1).context("N")?.parse()?);
                i += 1;
            }
            "--dump-tex" => {
                dump.push(parse_id(args.get(i + 1).context("ID")?)?);
                i += 1;
            }
            "--target" => {
                filter
                    .targets
                    .push(parse_id(args.get(i + 1).context("ID")?)?);
                i += 1;
            }
            other => bail!("unknown option {other}"),
        }
        i += 1;
    }
    let bytes = std::fs::read(&capture).with_context(|| capture.display().to_string())?;
    let body = bytes
        .strip_prefix(&ffx_gfx::CAPTURE_MAGIC[..])
        .context("not a capture file")?;
    let msgs = ffx_gfx::read_all(body)?;
    let mut r = ffx_render::Renderer::new()?;
    let mut targets = BTreeSet::new();
    let start = std::time::Instant::now();
    for m in msgs {
        if let ffx_gfx::Msg::Frame(f) = &m {
            for c in &f.cmds {
                if let ffx_gfx::Cmd::Draw(d) = c {
                    for &(_, v) in &d.rtv {
                        targets.insert(v);
                    }
                }
            }
        }
        r.apply(m, &filter)?;
    }
    println!("drawn in {:?}", start.elapsed());
    std::fs::create_dir_all(&out)?;
    for view in targets {
        let Some(resource) = r.views.get(&view).map(|v| v.resource) else {
            continue;
        };
        match r.read_rgba(resource) {
            Ok((w, h, rgba)) => {
                let path = out.join(format!("{view:x}.png"));
                std::fs::write(&path, ffx_render::png::encode(w, h, &rgba))?;
                println!("{view:#x}: {w}x{h} -> {}", path.display());
            }
            Err(e) => println!("{view:#x}: {e}"),
        }
    }
    for id in dump {
        for mip in 0..4 {
            if let Ok(raw) = r.read_raw(id, mip) {
                std::fs::write(out.join(format!("tex_{id:x}_{mip}.raw")), raw)?;
            }
        }
    }
    for (f, n) in r.failures() {
        println!("{n:4} x {f}");
    }
    for w in &r.warnings {
        println!("warning: {w}");
    }
    Ok(())
}
