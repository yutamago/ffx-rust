//! The mix on the default output device through cpal (ALSA on Linux, which
//! `PulseAudio` and `PipeWire` serve; Core Audio on macOS), with the WASAPI
//! output's rules (`ffx-hook/src/rust_audio_out.rs`; docs/host.md, "Native
//! platform", "FMOD"):
//!
//! - [`push`] appends a block (48,000 Hz interleaved stereo f32) to a queue
//!   and never waits; the queue keeps at most [`MAX_QUEUE`] frames (half a
//!   second) and drops the oldest;
//! - the device's callback takes frames from the queue; playback starts
//!   once [`PREFILL`] frames are queued and again after each underrun (the
//!   device wanted frames that weren't there: silence, counted);
//! - the same switches: `FFX_RUST_AUDIO_OUT=0` keeps it off, harness games
//!   in lockstep keep it off unless `FFX_RUST_AUDIO_OUT=1`.
//!
//! The device is asked for 48,000 Hz stereo f32; a device that has
//! another rate or channel count gets the mix resampled (linear) and
//! spread over its channels ([`Converter`]).

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Frames queued before playback (re)starts: three FMOD ticks.
pub const PREFILL: usize = 4800;
/// Frames the queue keeps at most (0.5 s).
pub const MAX_QUEUE: usize = 24_000;
/// The mix's rate.
pub const RATE: u32 = 48_000;

/// The queue between the mixer and the device, and its counters.
#[derive(Debug, Default)]
pub struct Queue {
    samples: VecDeque<f32>,
    playing: bool,
    /// Underruns so far.
    pub underruns: u64,
    /// Frames dropped (the queue was full).
    pub dropped: u64,
    /// Frames handed to the device.
    pub written: u64,
}

impl Queue {
    /// Appends a block (interleaved stereo), dropping the oldest frames
    /// past [`MAX_QUEUE`].
    pub fn push(&mut self, block: &[f32]) {
        self.samples.extend(block.iter().copied());
        let over = self.samples.len().saturating_sub(2 * MAX_QUEUE);
        if over > 0 {
            self.samples.drain(..over);
            self.dropped += over as u64 / 2;
        }
    }

    /// Frames queued.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    /// Fills `out` (interleaved stereo) for the device: queued frames once
    /// playing, silence before and after an underrun.
    pub fn pull(&mut self, out: &mut [f32]) {
        let want = out.len() / 2;
        if !self.playing && self.frames() >= PREFILL {
            self.playing = true;
        }
        let mut n = 0;
        if self.playing {
            n = self.frames().min(want);
            for (d, s) in out.iter_mut().zip(self.samples.drain(..2 * n)) {
                *d = s;
            }
            if n < want {
                self.underruns += 1;
                self.playing = false;
            }
            self.written += n as u64;
        }
        out[2 * n..].fill(0.0);
    }
}

/// The mix (48,000 Hz stereo) as a device of another rate or channel
/// count wants it: linear interpolation between the mix's frames, left
/// and right on the first two channels (a mono device gets their mean),
/// silence on the others.
#[derive(Debug)]
pub struct Converter {
    rate: u32,
    channels: usize,
    /// The mix's frames not used yet (stereo).
    pending: VecDeque<[f32; 2]>,
    /// The position between `pending[0]` and `pending[1]`, 0..1.
    pos: f64,
}

impl Converter {
    /// A converter to `rate` Hz, `channels` channels.
    #[must_use]
    pub fn new(rate: u32, channels: usize) -> Self {
        Self {
            rate,
            channels: channels.max(1),
            pending: VecDeque::new(),
            pos: 0.0,
        }
    }

    /// Fills `out` (the device's interleaved frames), taking mix frames
    /// from `queue` as it needs them.
    #[allow(
        clippy::cast_possible_truncation, // samples, f64 to f32
        clippy::cast_precision_loss,      // a device buffer's frames
        clippy::cast_sign_loss,           // a frame count, positive
        clippy::many_single_char_names    // the interpolation's terms
    )]
    pub fn fill(&mut self, queue: &mut Queue, out: &mut [f32]) {
        let frames = out.len() / self.channels;
        let step = f64::from(RATE) / f64::from(self.rate);
        // the mix frames this call reaches, plus one to interpolate to
        let need = ((self.pos + frames as f64 * step).ceil() as usize + 2)
            .saturating_sub(self.pending.len());
        let mut buf = vec![0f32; 2 * need];
        queue.pull(&mut buf);
        self.pending.extend(buf.as_chunks::<2>().0.iter().copied());
        for frame in out.chunks_exact_mut(self.channels) {
            let a = self.pending.front().copied().unwrap_or_default();
            let b = self.pending.get(1).copied().unwrap_or(a);
            let t = self.pos as f32;
            let l = a[0] + (b[0] - a[0]) * t;
            let r = a[1] + (b[1] - a[1]) * t;
            if self.channels == 1 {
                frame[0] = f32::midpoint(l, r);
            } else {
                frame[0] = l;
                frame[1] = r;
                frame[2..].fill(0.0);
            }
            self.pos += step;
            while self.pos >= 1.0 && !self.pending.is_empty() {
                self.pending.pop_front();
                self.pos -= 1.0;
            }
        }
    }
}

static QUEUE: Mutex<Option<Queue>> = Mutex::new(None);
static ON: AtomicBool = AtomicBool::new(false);
static UNDERRUNS: AtomicU64 = AtomicU64::new(0);

/// Whether the device output runs.
#[must_use]
pub fn on() -> bool {
    ON.load(Ordering::Acquire)
}

/// Frames queued for the device.
#[must_use]
pub fn queued() -> usize {
    QUEUE
        .lock()
        .ok()
        .and_then(|q| q.as_ref().map(Queue::frames))
        .unwrap_or(0)
}

/// Underruns so far.
#[must_use]
pub fn underruns() -> u64 {
    UNDERRUNS.load(Ordering::Relaxed)
}

/// One mixed block for the device; never waits.
pub fn push(block: &[f32]) {
    if !on() {
        return;
    }
    if let Ok(mut q) = QUEUE.lock()
        && let Some(q) = q.as_mut()
    {
        q.push(block);
    }
}

/// Starts the output (the module's doc for the switches). The stream
/// lives on a thread of its own (`ffx-rust-audio-out`): cpal's streams
/// are not `Send` everywhere.
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
    if let Ok(mut q) = QUEUE.lock() {
        *q = Some(Queue::default());
    }
    ON.store(true, Ordering::Release);
    let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
    let spawned = std::thread::Builder::new()
        .name("ffx-rust-audio-out".into())
        .spawn(move || match open() {
            Ok((stream, what)) => {
                let _ = tx.send(Ok(what));
                // the stream plays while it lives: for the process
                let _stream = stream;
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        });
    let result = match spawned {
        Ok(_) => rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap_or_else(|_| Err("the device did not open in 5 s".into())),
        Err(e) => Err(format!("no thread: {e}")),
    };
    match result {
        Ok(what) => crate::log::note(&format!("rust audio out: {what}")),
        Err(e) => {
            crate::log::note(&format!("rust audio out: stopped: {e}"));
            ON.store(false, Ordering::Release);
        }
    }
}

/// Opens the default device: the stream and what it is.
fn open() -> Result<(cpal::Stream, String), String> {
    use cpal::traits::{DeviceTrait as _, HostTrait as _, StreamTrait as _};
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no output device")?;
    let name = device
        .description()
        .map(|d| d.name().to_owned())
        .unwrap_or_default();
    // 48 kHz stereo f32 when the device has it, else its default
    let config = device
        .supported_output_configs()
        .map_err(|e| e.to_string())?
        .find(|c| {
            c.channels() == 2
                && c.sample_format() == cpal::SampleFormat::F32
                && (c.min_sample_rate()..=c.max_sample_rate()).contains(&RATE)
        })
        .map(|c| c.with_sample_rate(RATE));
    let config = match config {
        Some(c) => c,
        None => device.default_output_config().map_err(|e| e.to_string())?,
    };
    let format = config.sample_format();
    let cfg: cpal::StreamConfig = config.into();
    let (rate, channels) = (cfg.sample_rate, usize::from(cfg.channels));
    let mut conv = Converter::new(rate, channels);
    let direct = rate == RATE && channels == 2;
    let mut fill = move |out: &mut [f32]| {
        if let Ok(mut q) = QUEUE.lock()
            && let Some(q) = q.as_mut()
        {
            if direct {
                q.pull(out);
            } else {
                conv.fill(q, out);
            }
            UNDERRUNS.store(q.underruns, Ordering::Relaxed);
        } else {
            out.fill(0.0);
        }
    };
    let err = |e: cpal::Error| crate::log::note(&format!("rust audio out: {e}"));
    let stream = match format {
        cpal::SampleFormat::F32 => {
            device.build_output_stream(cfg, move |out: &mut [f32], _| fill(out), err, None)
        }
        cpal::SampleFormat::I16 => {
            let mut buf = Vec::new();
            device.build_output_stream(
                cfg,
                move |out: &mut [i16], _| {
                    buf.resize(out.len(), 0.0);
                    fill(&mut buf);
                    for (o, s) in out.iter_mut().zip(&buf) {
                        #[allow(clippy::cast_possible_truncation)] // clamped
                        {
                            *o = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                        }
                    }
                },
                err,
                None,
            )
        }
        other => return Err(format!("sample format {other} not handled")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((
        stream,
        format!(
            "device \"{name}\" opened, {rate} Hz, {channels} channels, {format}{}",
            if direct {
                ""
            } else {
                " (converted from 48000 Hz stereo)"
            }
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queue_prefills_counts_underruns_and_drops_the_oldest() {
        let mut q = Queue::default();
        let mut out = vec![1.0f32; 2 * 1000];
        // nothing yet: silence, no underrun counted (not playing)
        q.pull(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
        assert_eq!(q.underruns, 0);
        // below the prefill: still silence
        q.push(&vec![0.5; 2 * (PREFILL - 1)]);
        q.pull(&mut out);
        assert!(out.iter().all(|&s| s == 0.0));
        q.push(&[0.25, 0.25]);
        q.pull(&mut out);
        assert_eq!((out[0], q.frames()), (0.5, PREFILL - 1000));
        // drain past the end: the rest is silence, one underrun, refill
        let mut big = vec![1.0f32; 2 * PREFILL];
        q.pull(&mut big);
        assert_eq!(big[2 * (PREFILL - 1000) - 2], 0.25);
        assert_eq!(big[2 * (PREFILL - 1000)], 0.0);
        assert_eq!((q.underruns, q.written), (1, PREFILL as u64));
        // overrun: at most half a second kept
        q.push(&vec![0.0; 2 * (MAX_QUEUE + 100)]);
        assert_eq!((q.frames(), q.dropped), (MAX_QUEUE, 100));
    }

    #[test]
    fn conversion_to_another_rate_and_channel_count() {
        let mut q = Queue::default();
        // a ramp: frame i is (i, -i) / 10000
        let ramp: Vec<f32> = (0..PREFILL + 200)
            .flat_map(|i| {
                #[allow(clippy::cast_precision_loss)]
                let v = i as f32 / 10_000.0;
                [v, -v]
            })
            .collect();
        q.push(&ramp);
        // 24 kHz, 4 channels: every second mix frame, left/right on 0/1
        let mut c = Converter::new(24_000, 4);
        let mut out = vec![9.0f32; 4 * 10];
        c.fill(&mut q, &mut out);
        for (n, f) in out.as_chunks::<4>().0.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let want = (2 * n) as f32 / 10_000.0;
            assert!(
                (f[0] - want).abs() < 1e-6 && (f[1] + want).abs() < 1e-6,
                "{n}: {f:?}"
            );
            assert_eq!(&f[2..], &[0.0, 0.0]);
        }
        // 96 kHz mono: halfway frames interpolated, the channels' mean (0)
        let mut c = Converter::new(96_000, 1);
        let mut out = vec![9.0f32; 4];
        c.fill(&mut q, &mut out);
        assert!(out.iter().all(|s| s.abs() < 1e-6));
    }
}
