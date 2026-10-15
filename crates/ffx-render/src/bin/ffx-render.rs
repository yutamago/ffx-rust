//! `ffx-render replay <capture.ffxgfx> <out dir> [--until N] [--target ID]
//! [--enhanced] [--game DIR] [--settings FILE] [--edits FILE] [--set key=value] [--pick x,y [--outline]] [--materials DIR]
//! [--export-materials DIR]`: draws a captured frame and writes every
//! 8-bit colour render target it drew into as `<out dir>/<view id>.png`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("replay") => replay(&args[1..]),
        #[cfg(windows)]
        Some("live") => {
            let name = args.get(1).context("mapping name")?;
            let pid = args.get(2).context("game pid")?.parse()?;
            ffx_render::live::run(name, pid)
        }
        _ => bail!("usage: ffx-render replay <capture.ffxgfx> <out dir> [--until N] [--target ID]"),
    }
}

/// Settings with `key=value` overrides (TOML values).
/// The settings of a settings file (`--settings`), then each `--set` over
/// them.
fn settings(file: Option<&str>, sets: &[String]) -> ffx_render::enhanced::Settings {
    let mut table: toml::Table = file.map_or_else(toml::Table::new, |t| {
        toml::from_str(t).unwrap_or_else(|e| {
            eprintln!("bad --settings: {e}");
            toml::Table::new()
        })
    });
    for s in sets {
        match toml::from_str::<toml::Table>(s) {
            Ok(t) => table.extend(t),
            Err(e) => eprintln!("bad --set {s}: {e}"),
        }
    }
    toml::Value::Table(table).try_into().unwrap_or_else(|e| {
        eprintln!("bad settings: {e}");
        ffx_render::enhanced::Settings::default()
    })
}

fn parse_id(s: &str) -> Result<u32> {
    let s = s.trim_start_matches("0x");
    u32::from_str_radix(s, 16).context("hex id")
}

#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // the options, then one replay
fn replay(args: &[String]) -> Result<()> {
    let capture = PathBuf::from(args.first().context("capture file")?);
    let out = PathBuf::from(args.get(1).context("out dir")?);
    let mut filter = ffx_render::Filter::default();
    let mut dump = Vec::new();
    let mut enhanced = false;
    // Lights from the frame's glowing effects (found as live, then the
    // frame drawn again with them).
    let mut glow = false;
    // A pick as the editor makes it (0–1 of the screen), after the frame.
    let mut pick: Option<(f32, f32)> = None;
    // With a pick: the frame again with the picked texture outlined.
    let mut outline = false;
    let mut game: Option<PathBuf> = None;
    let mut sets: Vec<String> = Vec::new();
    let mut file: Option<String> = None;
    // An editor's edits file: the captured scene's lights and materials.
    let mut edits: Option<ffx_render::edits::Edits> = None;
    let mut materials: Option<PathBuf> = None;
    let mut export: Option<PathBuf> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--until" => {
                filter.until = Some(args.get(i + 1).context("N")?.parse()?);
                i += 1;
            }
            "--enhanced" => enhanced = true,
            "--glow" => glow = true,
            "--outline" => outline = true,
            "--pick" => {
                let v = args.get(i + 1).context("x,y")?;
                let (x, y) = v.split_once(',').context("x,y")?;
                pick = Some((x.parse()?, y.parse()?));
                i += 1;
            }
            "--game" => {
                game = Some(PathBuf::from(args.get(i + 1).context("DIR")?));
                i += 1;
            }
            "--materials" => {
                materials = Some(PathBuf::from(args.get(i + 1).context("DIR")?));
                i += 1;
            }
            "--export-materials" => {
                let dir = PathBuf::from(args.get(i + 1).context("DIR")?);
                std::fs::create_dir_all(&dir)?;
                export = Some(dir);
                i += 1;
            }
            "--edits" => {
                let f = PathBuf::from(args.get(i + 1).context("FILE")?);
                let (loaded, err) = ffx_render::edits::Edits::load(&f);
                if let Some(e) = err {
                    bail!("{e}");
                }
                edits = Some(loaded);
                i += 1;
            }
            "--settings" => {
                let f = args.get(i + 1).context("FILE")?;
                file = Some(std::fs::read_to_string(f).with_context(|| f.clone())?);
                i += 1;
            }
            "--set" => {
                sets.push(args.get(i + 1).context("key=value")?.clone());
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
    println!("{}", r.adapter);
    let mut e = enhanced.then(|| {
        let dir = game.clone().or_else(ffx_render::families::game_dir);
        let cache = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("ffx-render-shaders.tsv")))
            .unwrap_or_else(|| PathBuf::from("ffx-render-shaders.tsv"));
        let families = ffx_render::families::Families::load(dir.as_deref(), &cache);
        println!("{} shaders with families", families.len());
        let mut e = ffx_render::enhanced::Enhanced::new(
            &r.device,
            &r.queue,
            families,
            settings(file.as_deref(), &sets),
        );
        e.material_dir.clone_from(&materials);
        e.fsr_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        e.export_dir.clone_from(&export);
        e
    });
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
        if let ffx_gfx::Msg::World { scene, camera } = &m {
            println!("world: {scene:?}, camera {camera:?}");
            if let (Some(e), Some(ed)) = (e.as_mut(), edits.as_ref()) {
                let eff = ed.effective(scene);
                e.rig = eff.rig;
                e.glow_edits = eff.glows;
                e.material_edits = eff.materials;
            }
        }
        match (&mut e, m) {
            (Some(e), ffx_gfx::Msg::Frame(f)) => {
                e.pick = pick;
                e.render_until(&mut r, &f, filter.until)?;
                if pick.is_some() {
                    println!("picked: {:?}", e.picked);
                    if outline {
                        e.selected = e.picked.as_ref().map(|p| p.texture);
                        e.render_until(&mut r, &f, filter.until)?;
                    }
                }
                if glow {
                    let fx = ffx_render::enhanced::Enhanced::glowing_effects(&r, &f);
                    println!("glowing effects: {} commands", fx.cmds.len());
                    e.effects(&mut r, &fx)?;
                    r.device.poll(wgpu::PollType::wait_indefinitely())?;
                    e.effects(&mut r, &fx)?;
                    println!("glow: {}", e.glow.report);
                    for g in &e.glow.found {
                        println!("  light {:?}", g.light);
                    }
                    e.render_until(&mut r, &f, filter.until)?;
                    for (k, id) in e.glow.target_ids().into_iter().enumerate() {
                        if let Ok((w, h, rgba)) = r.read_rgba(id) {
                            std::fs::create_dir_all(&out)?;
                            std::fs::write(
                                out.join(format!("glow_{k}.png")),
                                ffx_render::png::encode(w, h, &rgba),
                            )?;
                        }
                    }
                }
            }
            (_, m) => r.apply(m, &filter)?,
        }
    }
    if let Some(e) = &mut e {
        println!(
            "enhanced: {} items, {} draws with the game's shaders; cascades and probe faces: {} culled, {} drawn",
            e.last_items, e.last_other, e.culled.0, e.culled.1
        );
        r.device.poll(wgpu::PollType::wait_indefinitely())?;
        if let Some(t) = e.timer.as_mut() {
            t.poll(&r.device);
            let passes: Vec<String> = t
                .last
                .iter()
                .map(|(n, ms)| format!("{n} {ms:.2}"))
                .collect();
            println!("GPU {:.2} ms: {}", t.total, passes.join(", "));
        }
        for (n, c) in &e.notes {
            println!("note: {c} x {n}");
        }
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
    for e in ffx_render::take_gpu_errors() {
        println!("GPU error: {e}");
    }
    Ok(())
}
