//! The Rust engine's mix on the default output device (WASAPI, shared
//! mode, through the `wasapi` crate, MIT), when `FFX_RUST_AUDIO=1` and no
//! `FFX_AUDIO_WAV` is set (`docs/formats/audio-platform.md`, "Output").
//!
//! The game clock drives the mix: each `EventSystem::update` hands one
//! block to [`push`], which only appends it to a queue and never waits. An
//! output thread of its own feeds the device from the queue whenever the
//! device asks. So the device's clock never paces the game and nothing the
//! game sees depends on it:
//!
//! - **underrun** (the device wants frames the game has not mixed yet, e.g.
//!   during a load): silence is written and counted;
//! - **overrun** (the game mixes faster than real time, e.g. the harness's
//!   fast mode): the queue keeps at most [`MAX_QUEUE`] frames and drops the
//!   oldest, counted.
//!
//! Playback starts once [`PREFILL`] frames are queued (100 ms) and again
//! after each underrun. The thread logs the device, its format and every
//! 10 s the frames written, underruns and overruns to the hook log.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::log::log;

/// Frames queued before playback (re)starts: three FMOD ticks.
const PREFILL: usize = 4800;
/// Frames the queue keeps at most (0.5 s); older ones are dropped.
const MAX_QUEUE: usize = 24_000;
const RATE: u32 = 48_000;

static QUEUE: Mutex<VecDeque<f32>> = Mutex::new(VecDeque::new());
static ON: AtomicBool = AtomicBool::new(false);
static WRITTEN: AtomicU64 = AtomicU64::new(0);
static UNDERRUNS: AtomicU64 = AtomicU64::new(0);
static DROPPED: AtomicU64 = AtomicU64::new(0);

/// The device output runs (the mix follows the game's real time then).
#[must_use]
pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

/// Hands one mixed block (interleaved stereo) to the device. Never waits
/// for the device.
pub fn push(block: &[f32]) {
    if !ON.load(Ordering::Acquire) {
        return;
    }
    let Ok(mut q) = QUEUE.lock() else { return };
    q.extend(block.iter().copied());
    let over = q.len().saturating_sub(2 * MAX_QUEUE);
    if over > 0 {
        q.drain(..over);
        DROPPED.fetch_add(over as u64 / 2, Ordering::Relaxed);
    }
}

/// Starts the output thread (`FFX_RUST_AUDIO_OUT=0` keeps it off; harness
/// games on the virtual clock keep it off too, as FMOD's capture set-up
/// does, unless `FFX_RUST_AUDIO_OUT=1`).
pub fn start() {
    let env = |n: &str| std::env::var(n).unwrap_or_default();
    let lockstep = !env("FFX_HARNESS").is_empty()
        && env("FFX_HARNESS_FAST") == "1"
        && env("FFX_HARNESS_VIRTUAL_CLOCK") == "1";
    match env("FFX_RUST_AUDIO_OUT").as_str() {
        "0" => return,
        "1" => {}
        _ if lockstep => return,
        _ => {}
    }
    ON.store(true, Ordering::Release);
    let spawned = std::thread::Builder::new()
        .name("ffx-rust-audio-out".into())
        .spawn(|| {
            if let Err(e) = run() {
                log(&format!("  rust audio out: stopped: {e}"));
            }
            ON.store(false, Ordering::Release);
        });
    if let Err(e) = spawned {
        log(&format!("  rust audio out: no thread: {e}"));
        ON.store(false, Ordering::Release);
    }
}

#[cfg(windows)]
fn run() -> Result<(), String> {
    use wasapi::{Direction, SampleType, StreamMode, WaveFormat};
    let e = |x: wasapi::WasapiError| x.to_string();
    let _ = wasapi::initialize_mta();
    let device = wasapi::DeviceEnumerator::new()
        .map_err(e)?
        .get_default_device(&Direction::Render)
        .map_err(e)?;
    let name = device.get_friendlyname().unwrap_or_default();
    let mut client = device.get_iaudioclient().map_err(e)?;
    let format = WaveFormat::new(32, 32, &SampleType::Float, RATE as usize, 2, None);
    let (period, _) = client.get_device_period().map_err(e)?;
    client
        .initialize_client(
            &format,
            &Direction::Render,
            &StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: period,
            },
        )
        .map_err(e)?;
    let event = client.set_get_eventhandle().map_err(e)?;
    let render = client.get_audiorenderclient().map_err(e)?;
    let size = client.get_buffer_size().map_err(e)?;
    log(&format!(
        "  rust audio out: device \"{name}\" opened, 48000 Hz stereo float (shared, converted by Windows), buffer {size} frames, period {period} x 100 ns"
    ));
    client.start_stream().map_err(e)?;
    let mut playing = false;
    let mut report = Instant::now();
    loop {
        let _ = event.wait_for_event(100);
        let space = client.get_available_space_in_frames().map_err(e)? as usize;
        let mut data = vec![0f32; space * 2];
        if let Ok(mut q) = QUEUE.lock() {
            if !playing && q.len() >= PREFILL * 2 {
                playing = true;
            }
            if playing {
                let n = (q.len() / 2).min(space);
                for (d, s) in data.iter_mut().zip(q.drain(..n * 2)) {
                    *d = s;
                }
                if n < space {
                    // underrun: silence for the rest, refill before playing on
                    UNDERRUNS.fetch_add(1, Ordering::Relaxed);
                    playing = false;
                }
                WRITTEN.fetch_add(n as u64, Ordering::Relaxed);
            }
        }
        let bytes: Vec<u8> = data.iter().flat_map(|x| x.to_le_bytes()).collect();
        render.write_to_device(space, &bytes, None).map_err(e)?;
        if report.elapsed() >= Duration::from_secs(10) {
            report = Instant::now();
            log(&format!(
                "  rust audio out: {} frames written, {} underruns, {} frames dropped",
                WRITTEN.load(Ordering::Relaxed),
                UNDERRUNS.load(Ordering::Relaxed),
                DROPPED.load(Ordering::Relaxed)
            ));
        }
        if !ON.load(Ordering::Acquire) {
            let _ = client.stop_stream();
            return Ok(());
        }
    }
}

#[cfg(not(windows))]
fn run() -> Result<(), String> {
    Err("no output device off Windows".into())
}
