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
    let mut settings_stamp = None;
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
    enhanced.export_dir = std::env::var_os("FFX_RENDER_EXPORT_MATERIALS")
        .map(std::path::PathBuf::from)
        .filter(|d| std::fs::create_dir_all(d).is_ok());
    let filter = Filter::default();
    let mut idle_from = std::time::Instant::now();
    let mut overlay = crate::overlay::Overlay::new(&r.device);
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
                "frame {} ({} commands)",
                frame.number,
                frame.cmds.len()
            ));
            for (f, n) in r.failures() {
                log(&format!("  {n} x {f}"));
            }
            for (f, n) in &enhanced.notes {
                if !f.starts_with("frame:") && !f.contains(" items, ") {
                    log(&format!("  enhanced: {n} x {f}"));
                }
            }
        }
        // Settings edits apply at once.
        if frames.is_multiple_of(30) {
            let stamp = std::fs::metadata(&settings_path)
                .and_then(|m| m.modified())
                .ok();
            if stamp != settings_stamp {
                settings_stamp = stamp;
                enhanced.settings = load_settings(&settings_path);
                log(&format!("settings: {:?}", enhanced.settings));
            }
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
        // The overlay, over the whole window.
        let overlay_image = if overlay.visible && screen.0 > 0 {
            let stats = crate::overlay::Stats {
                frame_ms: frame_time.as_secs_f32() * 1000.0,
                draw_ms: t_draw.as_secs_f32() * 1000.0,
                enhanced_items: enhanced.last_items,
                other_draws: enhanced.last_other,
            };
            match overlay.frame(&r.device, &r.queue, screen, &mut enhanced.settings, stats) {
                Ok(img) => Some(img),
                Err(e) => {
                    log(&format!("overlay: {e}"));
                    None
                }
            }
        } else {
            None
        };
        if overlay.save {
            overlay.save = false;
            overlay.message = match toml::to_string_pretty(&enhanced.settings) {
                Ok(text) => match std::fs::write(&settings_path, text) {
                    Ok(()) => "saved".into(),
                    Err(e) => format!("not saved: {e}"),
                },
                Err(e) => format!("not saved: {e}"),
            };
            settings_stamp = std::fs::metadata(&settings_path)
                .and_then(|m| m.modified())
                .ok();
        }
        if overlay.reset {
            overlay.reset = false;
            enhanced.settings = crate::enhanced::Settings::default();
            overlay.message = "defaults (not saved)".into();
        }
        // Answer: frame, scene size, flags (1: overlay follows, 2: the scene
        // is in shared textures, 4: with a fence, 8: the game's afterimage
        // off), the scene's colour, normal and depth (or the shared
        // generation and set), then the overlay's size and pixels.
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
            | if enhanced.settings.afterimage { 0 } else { 8 };
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&data);
        if let Some(img) = &overlay_image {
            out.extend_from_slice(&screen.0.to_le_bytes());
            out.extend_from_slice(&screen.1.to_le_bytes());
            out.extend_from_slice(img);
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
        last_frame = std::time::Instant::now();
        idle_from = std::time::Instant::now();
    }
}
