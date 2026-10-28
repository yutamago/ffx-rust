//! The software mixer that replaces FMOD Ex's (M5 stage 2; measured
//! behaviour in `docs/formats/audio-platform.md`, "FMOD Ex's mixer").
//!
//! Output: 48,000 Hz stereo `f32`, one block of 1,600 frames per FMOD tick
//! (what the game's FMOD writes under the harness's capture). Each voice is
//! one playing wave: 16-bit PCM from [`crate::ima`] times 1/32768,
//! resampled to the output rate by linear interpolation (`a + (b - a) *
//! frac`, the fraction of a 32.32 fixed-point position starting at 0); its
//! left and right gains move to a new value by a linear ramp over the
//! block's first [`RAMP`] frames (from 0 at the start), added after each
//! frame, and only for a change of at least [`CHANGE`]; below [`SILENT`] a
//! voice falls silent at once. The pan is ignored; mono waves play on the
//! left only. Loops run on whole IMA blocks. All measured against FMOD's
//! own mix and with `examples/fmod-probe.rs`; what is still off (float
//! rounding) is listed in `audio-platform.md`.

use std::sync::Arc;

/// Output rate.
pub const RATE: u32 = 48_000;
/// Frames per block (one FMOD tick, 33.333 ms).
pub const BLOCK: usize = 1600;
/// Frames of the gain ramp at a voice's start (measured: frame k of a new
/// voice has gain k/64 of its volume).
pub const RAMP: u32 = 64;
/// The smallest change of a voice's levels (left plus right) that FMOD
/// applies: 0.00064 (one ramp step of 1e-5).
pub const CHANGE: f32 = 0.000_64;
/// Below this volume FMOD mixes a voice silent (between the measured
/// 1.704e-5, silent, and 2.32e-5, not).
pub const SILENT: f32 = 2.0e-5;
/// 16-bit PCM to float.
const SCALE: f32 = 1.0 / 32768.0;

/// A decoded wave, shared by its voices.
#[derive(Debug)]
pub struct Wave {
    /// Interleaved 16-bit PCM.
    pub pcm: Vec<i16>,
    pub channels: u16,
    pub rate: u32,
    pub frames: u32,
    /// Loop start and end frame (inclusive), from the FSB loop chunk.
    pub looped: Option<(u32, u32)>,
}

impl Wave {
    fn frame(&self, i: u64) -> (f32, f32) {
        let Ok(i) = usize::try_from(i) else {
            return (0.0, 0.0);
        };
        if self.channels == 2 {
            match self.pcm.get(2 * i..2 * i + 2) {
                Some(s) => (f32::from(s[0]), f32::from(s[1])),
                None => (0.0, 0.0),
            }
        } else {
            let s = self.pcm.get(i).map_or(0.0, |&s| f32::from(s));
            (s, s)
        }
    }
}

/// One playing wave.
#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)] // four independent states of a voice
pub struct Voice {
    pub wave: Arc<Wave>,
    /// Loops between the wave's loop points; false plays it once.
    pub looping: bool,
    /// Position in source frames, 32.32 fixed point.
    pub pos: u64,
    /// Source frames per output frame, 32.32.
    pub step: u64,
    /// Linear gain (event volume x the game's volume).
    pub volume: f32,
    /// -1..1 (`Channel::setPan`).
    pub pan: f32,
    pub paused: bool,
    /// Output frames mixed so far.
    pub age: u32,
    /// The left and right gain the last frame used (0 at the start: the
    /// first block ramps up from silence).
    pub gain: [f32; 2],
    /// Past its first loop end (later passes are a whole number of blocks).
    pub wrapped: bool,
    /// Blocks mixed.
    pub blocks: u32,
    /// Ended: past its last frame.
    pub ended: bool,
}

impl Voice {
    #[must_use]
    pub fn new(wave: Arc<Wave>, looping: bool) -> Self {
        let step = (u64::from(wave.rate) << 32) / u64::from(RATE);
        Self {
            wave,
            looping,
            pos: 0,
            step,
            volume: 1.0,
            pan: 0.0,
            paused: false,
            age: 0,
            gain: [0.0; 2],
            blocks: 0,
            wrapped: false,
            ended: false,
        }
    }

    /// Left and right gains for the pan.
    fn pan_gains(&self) -> (f32, f32) {
        // measured with fmod-probe (audio-platform.md): the event system
        // ignores `Channel::setPan` (pans -1..1 change nothing); a mono wave
        // plays on the left only, a stereo wave on both sides at full gain
        if self.wave.channels == 1 {
            (1.0, 0.0)
        } else {
            (1.0, 1.0)
        }
    }

    /// Mixes one block into `out` (interleaved stereo).
    #[allow(clippy::many_single_char_names)] // a, b, l, r, w: the interpolation
    pub fn mix(&mut self, out: &mut [f32]) {
        if self.paused || self.ended {
            return;
        }
        let (pl, pr) = self.pan_gains();
        let mut target = [self.volume * pl, self.volume * pr];
        // measured with fmod-probe: a new gain is taken only when the
        // changes of the voice's levels (left plus right) come to at least
        // CHANGE; otherwise the old gain stays and the next block compares
        // again (mono 0.00063 a tick skips every other block, 0.00064 does
        // not; stereo 0.000315 and 0.00032 a side)
        let change = (target[0] - self.gain[0]).abs() + (target[1] - self.gain[1]).abs();
        if change < CHANGE {
            // measured with fmod-probe: a change too small to take, to a
            // volume below SILENT, silences the voice at once (no ramp;
            // 1.704e-5 silent, 2.32e-5 not); it keeps its place. A larger
            // change ramps as usual, to 0 too (battle-attack's music fade)
            if self.volume < SILENT {
                self.gain = [0.0; 2];
            }
            target = self.gain;
        }
        self.blocks = self.blocks.wrapping_add(1);
        let mut ramp_left = 0u32;
        let mut delta = [0.0f32; 2];
        #[allow(clippy::float_cmp)] // exact: a ramp only for a changed gain
        if target != self.gain {
            ramp_left = RAMP;
            #[allow(clippy::cast_precision_loss)] // 64
            let n = RAMP as f32;
            delta = [
                (target[0] - self.gain[0]) / n,
                (target[1] - self.gain[1]) / n,
            ];
        }
        let w = &*self.wave;
        let end = u64::from(w.frames);
        let mut loop_points = if self.looping {
            w.looped
                // measured with fmod-probe: FMOD loops on whole IMA blocks
                // (64 frames). The first pass ends at the end point rounded
                // up; every pass after it starts at the start point rounded
                // up and lasts the loop's length rounded up: 50027_0's
                // 119648..=737699 plays 0..737728, then 119680..737792
                .map(|(s, e)| {
                    let block = crate::ima::BLOCK_SAMPLES as u64;
                    let (s, e) = (u64::from(s), u64::from(e) + 1);
                    let start = s.div_ceil(block) * block;
                    let later = (start + (e - s).div_ceil(block) * block).min(end);
                    (start, e.div_ceil(block) * block, later)
                })
                .map(|(start, first, later)| (start, first.min(end), later))
                .or(Some((0, end, end)))
                .map(|(start, first, later)| {
                    (start, if self.wrapped { later } else { first }, later)
                })
        } else {
            None
        };
        for f in out.as_chunks_mut::<2>().0 {
            let i = self.pos >> 32;
            // measured bit for bit (fmod-probe, a 44.1 kHz stream alone): the
            // fraction is the top 31 bits converted to f32 (nearest, ties to
            // even), and the taps are weighted a x (1 - f) + b x f
            #[allow(clippy::cast_precision_loss)] // 31 fraction bits into f32, as measured
            let frac = ((self.pos & 0xffff_ffff) >> 1) as f32 / 2_147_483_648.0;
            let next = match loop_points {
                Some((ls, le, _)) if i + 1 >= le => ls,
                _ => i + 1,
            };
            let (a, b) = (w.frame(i), w.frame(next));
            let l = (a.0 * (1.0 - frac) + b.0 * frac) * SCALE;
            let r = (a.1 * (1.0 - frac) + b.1 * frac) * SCALE;
            // measured (fmod-probe, three voices): while the gain ramps, the
            // product is not rounded before it is added (the sum is rounded
            // to f32 once); at a steady gain both are f32 operations
            #[allow(clippy::cast_possible_truncation)] // the f32 mix buffer
            if ramp_left > 0 {
                f[0] = (f64::from(f[0]) + f64::from(l) * f64::from(self.gain[0])) as f32;
                f[1] = (f64::from(f[1]) + f64::from(r) * f64::from(self.gain[1])) as f32;
            } else {
                f[0] += l * self.gain[0];
                f[1] += r * self.gain[1];
            }
            if ramp_left > 0 {
                self.gain[0] += delta[0];
                self.gain[1] += delta[1];
                ramp_left -= 1;
                if ramp_left == 0 {
                    self.gain = target;
                }
            }
            self.age = self.age.saturating_add(1);
            self.pos += self.step;
            let i = self.pos >> 32;
            match loop_points {
                Some((ls, le, later)) if i >= le => {
                    self.pos -= (le - ls) << 32;
                    self.wrapped = true;
                    loop_points = Some((ls, later, later));
                }
                None if i >= end => {
                    self.ended = true;
                    break;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first frames FMOD mixed of `1_24_the_summoning` (44,100 Hz
    /// stereo, event volume 0.35481339, the game's 0.05) in menu-browse:
    /// its first non-zero samples are right 1 and left 5, both -1.
    #[test]
    fn start_ramp_and_interpolation_match_fmod() {
        let mut pcm = vec![0i16; 2 * 64];
        pcm[3] = -1; // frame 1, right
        pcm[10] = -1; // frame 5, left
        let wave = Arc::new(Wave {
            pcm,
            channels: 2,
            rate: 44_100,
            frames: 64,
            looped: None,
        });
        let mut v = Voice::new(wave, false);
        v.volume = 0.354_813_4 * 0.05;
        let mut out = vec![0.0f32; 2 * BLOCK];
        v.mix(&mut out);
        // (frame, channel, the capture's float)
        for (k, c, want) in [
            (1, 1, -7.772_083e-9f32),
            (2, 1, -2.749_309e-9),
            (5, 0, -2.511_387_5e-8),
            (6, 0, -2.474_377_5e-8),
        ] {
            assert_eq!(out[2 * k + c].to_bits(), want.to_bits(), "frame {k}");
        }
        assert_eq!(out[0].to_bits(), 0);
    }
}
