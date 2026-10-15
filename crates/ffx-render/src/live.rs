//! `ffx-render live <mapping> <game pid>`: the renderer the game's hook
//! starts. Reads the scene stream from the shared mapping (`ffx-gfx`'s
//! ring), draws each frame's scene and writes the colour, normal and depth
//! images back for the hook to put in the game's targets.

use anyhow::{Context, Result, bail};
use ffx_gfx::ring::Ring;
use windows_sys::Win32::Foundation::WAIT_TIMEOUT;
use windows_sys::Win32::System::Memory::{FILE_MAP_ALL_ACCESS, MapViewOfFile, OpenFileMappingW};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
};

use crate::{Filter, Renderer};

/// Sizes of the two rings, as the hook makes them.
const TO_RENDERER: usize = 256 << 20;
const TO_GAME: usize = 64 << 20;

/// Appends a line to `ffx-render.log` next to the executable.
pub fn log(line: &str) {
    use std::io::Write as _;
    let path = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("ffx-render.log")));
    if let Some(path) = path
        && let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

/// `ffx-render.toml`, or the defaults (written there when missing).
fn load_settings(path: &std::path::Path) -> crate::enhanced::Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
            log(&format!("{}: {e}", path.display()));
            crate::enhanced::Settings::default()
        }),
        Err(_) => {
            let s = crate::enhanced::Settings::default();
            if let Ok(text) = toml::to_string_pretty(&s) {
                let _ = std::fs::write(path, text);
            }
            s
        }
    }
}

/// Runs until the game exits.
///
/// # Errors
/// When the mapping cannot be opened or the device made.
#[allow(clippy::too_many_lines)] // the loop: a message, the frame, the answer
pub fn run(name: &str, pid: u32) -> Result<()> {
    let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
    // SAFETY: plain Win32 calls; the view lives as long as this process.
    let (to_renderer, to_game, game) = unsafe {
        let mapping = OpenFileMappingW(FILE_MAP_ALL_ACCESS, 0, wide.as_ptr());
        if mapping.is_null() {
            bail!("no mapping {name}");
        }
        let view = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, TO_RENDERER + TO_GAME);
        if view.Value.is_null() {
            bail!("cannot map {name}");
        }
        let base = view.Value.cast::<u8>();
        let game = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        (Ring::open(base), Ring::open(base.add(TO_RENDERER)), game)
    };
    let alive = || {
        // SAFETY: our handle to the game.
        game.is_null() || unsafe { WaitForSingleObject(game, 0) } == WAIT_TIMEOUT
    };
    let wait = || {
        if !alive() {
            return false;
        }
        std::thread::yield_now();
        true
    };
    log(&format!("ffx-render live: {name}, game {pid}"));
    let mut r = match Renderer::new() {
        Ok(r) => r,
        Err(e) => {
            log(&format!("no renderer: {e:#}"));
            return Err(e).context("renderer");
        }
    };
    log(&format!("device ready: {}", r.adapter));
    let mut frames = 0u32;
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_default();
    let families = crate::families::Families::load(
        crate::families::game_dir().as_deref(),
        &exe_dir.join("ffx-render-shaders.tsv"),
    );
    log(&format!("{} shaders with families", families.len()));
    let settings_path = exe_dir.join("ffx-render.toml");
    let mut enhanced = crate::enhanced::Enhanced::new(
        &r.device,
        &r.queue,
        families,
        load_settings(&settings_path),
    );
    // Material packs next to the renderer; FFX_RENDER_EXPORT_MATERIALS=DIR
    // writes the generated maps there.
    let materials = exe_dir.join("materials");
    enhanced.material_dir = materials.is_dir().then_some(materials);
    enhanced.texture_dir = Some(exe_dir.join("textures"));
    enhanced.fsr_dir = Some(exe_dir.clone());
    enhanced.export_dir = std::env::var_os("FFX_RENDER_EXPORT_MATERIALS")
        .map(std::path::PathBuf::from)
        .filter(|d| std::fs::create_dir_all(d).is_ok());
    let filter = Filter::default();
    let mut idle_from = std::time::Instant::now();
    let mut overlay = crate::overlay::Overlay::new(&r.device, &settings_path, &enhanced.settings);
    let mut screen = (0u32, 0u32);
    // The scene image goes back through shared textures where it can.
    let mut share = if std::env::var("FFX_GFX_SHARE").is_ok_and(|v| v == "0") {
        None
    } else {
        match crate::share::Share::new(&r.device, pid) {
            Ok(s) => Some(s),
            Err(e) => {
                log(&format!("no shared textures: {e:#}"));
                None
            }
        }
    };
    let mut last_frame = std::time::Instant::now();
    let mut effect_msgs = 0u32;
    let mut frame_time = std::time::Duration::ZERO;
    loop {
        let mut head = [0u8; 8];
        if !to_renderer.read(&mut head, wait) {
            return Ok(());
        }
        let t_head = std::time::Instant::now();
        let idle = t_head - idle_from;
        let tag = u32::from_le_bytes(head[0..4].try_into().expect("4"));
        let len = u32::from_le_bytes(head[4..8].try_into().expect("4")) as usize;
        let mut payload = vec![0u8; len];
        if !to_renderer.read(&mut payload, wait) {
            return Ok(());
        }
        let msg = match ffx_gfx::Msg::read(tag, &payload) {
            Ok(m) => m,
            Err(e) => {
                log(&format!("bad message {tag}: {e}"));
                return Err(e.into());
            }
        };
        let frame = match msg {
            ffx_gfx::Msg::Frame(frame) => frame,
            ffx_gfx::Msg::Effects(effects) => {
                effect_msgs += 1;
                if effect_msgs % 600 == 1 {
                    let draws = effects
                        .cmds
                        .iter()
                        .filter(|c| matches!(c, ffx_gfx::Cmd::Draw(_)))
                        .count();
                    log(&format!(
                        "effects {}: {} commands, {draws} draws",
                        effects.number,
                        effects.cmds.len()
                    ));
                }
                if let Err(e) = enhanced.effects(&mut r, &effects) {
                    log(&format!("effects {}: {e}", effects.number));
                }
                idle_from = std::time::Instant::now();
                continue;
            }
            ffx_gfx::Msg::Input {
                width,
                height,
                events,
            } => {
                overlay.feed(&events);
                screen = (width, height);
                idle_from = std::time::Instant::now();
                continue;
            }
            msg => {
                r.apply(msg, &filter)?;
                idle_from = std::time::Instant::now();
                continue;
            }
        };
        let targets = frame.cmds.iter().find_map(|c| match c {
            ffx_gfx::Cmd::Draw(d) if d.rtv.len() >= 2 && d.dsv != 0 => {
                Some((d.rtv[0].1, d.rtv[1].1, d.dsv))
            }
            _ => None,
        });
        frames += 1;
        if frames == 1 || frames.is_multiple_of(600) {
            log(&format!(
                "frame {} ({} commands), {:?}",
                frame.number,
                frame.cmds.len(),
                r.world
            ));
            log(&format!("  glow: {}", enhanced.glow.report));
            for w in r.warnings.drain(..) {
                log(&format!("  warning: {w}"));
            }
            for e in crate::take_gpu_errors() {
                log(&format!("  GPU error: {e}"));
            }
            for m in crate::enhanced::fsr::messages() {
                log(&format!("  FSR: {m}"));
            }
            // FFX_RENDER_GLOW_DUMP=DIR: the glow targets as PNGs.
            if let Some(dir) = std::env::var_os("FFX_RENDER_GLOW_DUMP") {
                let dir = std::path::PathBuf::from(dir);
                for (k, id) in enhanced.glow.target_ids().into_iter().enumerate() {
                    if let Ok((w, h, rgba)) = r.read_rgba(id) {
                        let _ = std::fs::write(
                            dir.join(format!("glow_{}_{k}.png", frame.number)),
                            crate::png::encode(w, h, &rgba),
                        );
                    }
                }
            }
            for f in &enhanced.glow.found {
                log(&format!(
                    "  glow light at {:?}, colour {:?}, intensity {:.2}, range {:.0}",
                    f.light.position, f.light.color, f.light.intensity, f.light.range
                ));
            }
            for (f, n) in r.failures() {
                log(&format!("  {n} x {f}"));
            }
            for (f, n) in &enhanced.notes {
                if !f.starts_with("frame:") && !f.contains(" items, ") {
                    log(&format!("  enhanced: {n} x {f}"));
                }
            }
        }
        // Edits of the settings file apply at once.
        if frames.is_multiple_of(30) && overlay.profiles.poll(&mut enhanced.settings) {
            log(&format!("settings: {:?}", enhanced.settings));
        }
        // The profile the scene asks for.
        let asked = overlay
            .editor
            .edits
            .profile_for(&r.world.scene)
            .map(str::to_owned);
        if let Some(m) =
            overlay
                .profiles
                .follow(&r.world.scene, asked.as_deref(), &mut enhanced.settings)
        {
            log(&format!("{}: {m}", r.world.scene));
            overlay.message = m;
        }
        // The editor's changes for this scene.
        if overlay.editor.changed(&r.world.scene) {
            let e = overlay.editor.edits.effective(&r.world.scene);
            enhanced.lights = e.lights;
            enhanced.sun = e.sun;
            enhanced.glow_edits = e.glows;
            enhanced.material_edits = e.materials;
            enhanced.reload_replacements();
        }
        let t0 = std::time::Instant::now();
        let t_parsed = t0 - t_head;
        if targets.is_some()
            && let Err(e) = enhanced.render(&mut r, &frame)
        {
            log(&format!("frame {}: {e}", frame.number));
        }
        let t_draw = t0.elapsed();
        let resource = |v: ffx_gfx::Id| r.views.get(&v).map(|v| v.resource);
        let t1 = std::time::Instant::now();
        let ids = targets.and_then(|(c, n, z)| Some((resource(c)?, resource(n)?, resource(z)?)));
        let mut shared = None;
        if let (Some(s), Some((c, n, z))) = (share.as_mut(), ids) {
            match s.write(&r, c, n, z) {
                Ok(x) => shared = Some(x),
                Err(e) => {
                    log(&format!(
                        "frame {}: shared textures: {e:#}; reading back",
                        frame.number
                    ));
                    share = None;
                }
            }
        }
        let scene = match (ids, shared) {
            (Some((c, n, z)), None) => match r.read_scene(c, n, z) {
                Ok(x) => Some(x),
                Err(e) => {
                    log(&format!("frame {}: {e}", frame.number));
                    None
                }
            },
            _ => None,
        };
        let t_handoff = t1.elapsed();
        if !overlay.visible {
            enhanced.selected = None;
            overlay.hidden();
        }
        // The overlay, over the whole window.
        let overlay_image = if overlay.visible && screen.0 > 0 {
            let stats = crate::overlay::Stats {
                frame_ms: frame_time.as_secs_f32() * 1000.0,
                draw_ms: t_draw.as_secs_f32() * 1000.0,
                gpu: enhanced
                    .timer
                    .as_ref()
                    .map(|t| t.last.clone())
                    .unwrap_or_default(),
                gpu_ms: enhanced.timer.as_ref().map_or(0.0, |t| t.total),
                enhanced_items: enhanced.last_items,
                other_draws: enhanced.last_other,
            };
            let picked = enhanced.picked.take();
            if let Some(p) = &picked {
                let view = p.view.and_then(|v| r.texture_view(v, false));
                overlay.set_preview(&r.device, view.as_deref());
            }
            let glows: Vec<crate::enhanced::lights::Light> =
                enhanced.glow.lights().cloned().collect();
            let scene = r.world.scene.clone();
            let profiles = overlay.profiles.names.clone();
            let mut io = crate::editor::Io {
                scene: &scene,
                profiles: &profiles,
                camera: r.world.camera,
                glows: &glows,
                picked,
                pick: None,
                selected: enhanced.selected,
                preview: false,
            };
            let img = match overlay.frame(
                &r.device,
                &r.queue,
                screen,
                &mut enhanced.settings,
                stats,
                &mut io,
            ) {
                Ok(img) => img,
                Err(e) => {
                    log(&format!("overlay: {e}"));
                    None
                }
            };
            if io.pick.is_some() {
                enhanced.pick = io.pick;
            }
            enhanced.selected = io.selected;
            img
        } else {
            None
        };
        // Answer: frame, scene size, flags (1: overlay follows, 2: the scene
        // is in shared textures, 4: with a fence, 8: the game's afterimage
        // off, 16: the overlay takes the keyboard, 32: send the glowing
        // effects drawn after the scene), the scene's colour, normal and depth (or the shared
        // generation and set), then the screen's size, the part of the
        // overlay that changed (x, y, width, height) and its pixels.
        let (w, h, mut data) = match shared {
            Some(s) => {
                let mut d = s.generation.to_le_bytes().to_vec();
                d.extend_from_slice(&s.set.to_le_bytes());
                if let Some((handle, value)) = s.fence {
                    d.extend_from_slice(&handle.to_le_bytes());
                    d.extend_from_slice(&value.to_le_bytes());
                }
                (s.width, s.height, d)
            }
            None => scene.unwrap_or((0, 0, Vec::new())),
        };
        if std::env::var("FFX_RENDER_TEST").is_ok_and(|v| v == "invert")
            && w > 0
            && shared.is_none()
        {
            for px in data[..(w * h * 4) as usize].as_chunks_mut::<4>().0 {
                px[0] = 255 - px[0];
                px[1] = 255 - px[1];
                px[2] = 255 - px[2];
            }
        }
        let mut out = Vec::with_capacity(16 + data.len());
        out.extend_from_slice(&frame.number.to_le_bytes());
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        let flags = u32::from(overlay_image.is_some())
            | if shared.is_some() { 2 } else { 0 }
            | if shared.is_some_and(|s| s.fence.is_some()) {
                4
            } else {
                0
            }
            | if enhanced.settings.afterimage { 0 } else { 8 }
            | if overlay.visible && overlay.wants_keys {
                16
            } else {
                0
            }
            | if enhanced.settings.enabled && enhanced.settings.glow_lights {
                32
            } else {
                0
            };
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&data);
        if let Some((x, y, w, h, pixels)) = &overlay_image {
            for v in [screen.0, screen.1, *x, *y, *w, *h] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(pixels);
        }
        to_game.write(&out, std::thread::yield_now);
        if frames.is_multiple_of(120) {
            log(&format!(
                "frame {}: received+parsed {t_parsed:?}, draw {t_draw:?}, handed over {t_handoff:?}, total {:?}, idle before {idle:?}",
                frame.number,
                t_head.elapsed()
            ));
        }
        frame_time = last_frame.elapsed();
        overlay.frame_time(frame_time.as_secs_f32() * 1000.0);
        last_frame = std::time::Instant::now();
        idle_from = std::time::Instant::now();
    }
}
