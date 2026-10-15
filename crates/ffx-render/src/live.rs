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

/// Runs until the game exits.
///
/// # Errors
/// When the mapping cannot be opened or the device made.
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
    log("device ready");
    let mut frames = 0u32;
    let filter = Filter::default();
    let mut idle_from = std::time::Instant::now();
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
        let ffx_gfx::Msg::Frame(frame) = msg else {
            r.apply(msg, &filter)?;
            idle_from = std::time::Instant::now();
            continue;
        };
        let targets = frame.cmds.iter().find_map(|c| match c {
            ffx_gfx::Cmd::Draw(d) if d.rtv.len() >= 2 && d.dsv != 0 => {
                Some((d.rtv[0].1, d.rtv[1].1, d.dsv))
            }
            _ => None,
        });
        frames += 1;
        if frames == 1 || frames % 600 == 0 {
            log(&format!(
                "frame {} ({} commands)",
                frame.number,
                frame.cmds.len()
            ));
            for (f, n) in r.failures() {
                log(&format!("  {n} x {f}"));
            }
        }
        let t0 = std::time::Instant::now();
        let t_parsed = t0 - t_head;
        let drawn = r.draw_frame(&frame, &filter);
        let t_draw = t0.elapsed();
        if let Err(e) = drawn {
            log(&format!("frame {}: {e}", frame.number));
        }
        let Some((c, n, z)) = targets else { continue };
        let resource = |v: ffx_gfx::Id| r.views.get(&v).map(|v| v.resource);
        let (Some(c), Some(n), Some(z)) = (resource(c), resource(n), resource(z)) else {
            continue;
        };
        let t1 = std::time::Instant::now();
        let scene = r.read_scene(c, n, z);
        let t_read = t1.elapsed();
        match scene {
            Ok((w, h, mut data)) => {
                // FFX_RENDER_TEST=invert: the colour inverted, to see
                // whose image is on screen.
                if std::env::var("FFX_RENDER_TEST").is_ok_and(|v| v == "invert") {
                    for px in data[..(w * h * 4) as usize].as_chunks_mut::<4>().0 {
                        px[0] = 255 - px[0];
                        px[1] = 255 - px[1];
                        px[2] = 255 - px[2];
                    }
                }
                let mut out = Vec::with_capacity(12 + data.len());
                out.extend_from_slice(&frame.number.to_le_bytes());
                out.extend_from_slice(&w.to_le_bytes());
                out.extend_from_slice(&h.to_le_bytes());
                out.extend_from_slice(&data);
                to_game.write(&out, || std::thread::yield_now());
                if frames % 120 == 0 {
                    log(&format!(
                        "frame {}: received+parsed {t_parsed:?}, draw {t_draw:?}, read back {t_read:?}, total {:?}, idle before {idle:?}",
                        frame.number,
                        t_head.elapsed()
                    ));
                }
            }
            Err(e) => log(&format!("frame {}: {e}", frame.number)),
        }
        idle_from = std::time::Instant::now();
    }
}
