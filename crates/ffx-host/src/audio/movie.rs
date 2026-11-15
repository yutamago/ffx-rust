//! The movie's user stream (`System::createSound` with `FMOD_OPENUSER`,
//! `CREATESTREAM`): mixed from the game's PCM read callback at the
//! stream's rate, interpolated and ramped like every voice, as the hooked
//! game's Rust engine does (`crates/ffx-hook/src/rust_audio.rs`, "The
//! movie's user stream"): two chunks are read in `createSound` (before the
//! game sets the user data: silence), then one each time the play position
//! passes a multiple of [`CHUNK`] frames, in that update (the reads FMOD
//! makes in the movie-fmv log). Matched against FMOD's captures by the
//! audio gate's movie-fmv, sample for sample (docs/host.md, "FMOD").
//!
//! The read callback is game code (the movie player's, stdcall(sound,
//! data, bytes)): it runs through [`Exec::call_stdcall`] into a block of
//! the arena heap, outside the audio state's lock (it asks for the sound's
//! user data, `Sound::getUserData`, which takes that lock).

use std::sync::Arc;

use ffx_bridge::Mem as _;
use ffx_sound::mixer::{Voice, Wave};

use crate::exec::Exec;

/// Frames the read callback writes per call (`decodebuffersize`).
pub const CHUNK: usize = 0x1000;

/// The stream.
#[derive(Debug)]
pub struct Movie {
    pub sound: u32,
    /// The channel `playSound` gave it (`sound | 4`), 0 before.
    pub channel: u32,
    /// The game's `FMOD_SOUND_PCMREADCALLBACK`.
    read: u32,
    channels: u16,
    rate: u32,
    voice: Option<Voice>,
    /// Frames from `base` on (the ones played are dropped).
    pcm: Vec<i16>,
    base: u64,
    pub volume: f32,
    pub paused: bool,
    /// The stereo speaker mode (`Engine::stereo`).
    pub stereo: bool,
    /// Frames read and mixed so far (for the report).
    pub fetched: u64,
    pub mixed: u64,
}

fn empty(channels: u16, rate: u32) -> Arc<Wave> {
    Arc::new(Wave {
        pcm: Vec::new(),
        channels,
        rate,
        frames: 0,
        looped: None,
    })
}

impl Movie {
    /// The stream `createSound` describes: `FMOD_CREATESOUNDEXINFO` at
    /// `exinfo` (channels `+0xc`, rate `+0x10`, read callback `+0x2c`).
    pub fn new(e: &Exec, sound: u32, exinfo: u32) -> Self {
        let mem = e.mem();
        let (channels, rate, read) = if exinfo == 0 {
            (2, 48_000, 0)
        } else {
            (
                mem.read::<u32>(exinfo + 0xc),
                mem.read::<u32>(exinfo + 0x10),
                mem.read::<u32>(exinfo + 0x2c),
            )
        };
        Self {
            sound,
            channel: 0,
            read,
            channels: u16::try_from(channels).unwrap_or(2).clamp(1, 8),
            rate: if rate == 0 { 48_000 } else { rate },
            voice: None,
            pcm: Vec::new(),
            base: 0,
            volume: 1.0,
            paused: false,
            stereo: false,
            fetched: 0,
            mixed: 0,
        }
    }

    /// What a read needs: (callback, sound, bytes).
    pub fn request(&self) -> Option<(u32, u32, u32)> {
        let bytes = u32::try_from(CHUNK * usize::from(self.channels) * 2).ok()?;
        (self.read != 0).then_some((self.read, self.sound, bytes))
    }

    /// Appends one chunk the callback wrote.
    pub fn push(&mut self, samples: &[i16]) {
        self.pcm.extend_from_slice(samples);
        self.fetched += CHUNK as u64;
    }

    /// Mixes the stream into `block`; returns how many chunks to read
    /// after it (the play position passed that many multiples of
    /// [`CHUNK`]).
    pub fn mix(&mut self, block: &mut [f32]) -> usize {
        if self.channel == 0 || self.paused {
            return 0;
        }
        let ch = usize::from(self.channels);
        let mut v = self
            .voice
            .take()
            .unwrap_or_else(|| Voice::new(empty(self.channels, self.rate), false));
        let before = self.base + (v.pos >> 32);
        // drop what is behind the play position (keep a frame for the taps)
        let drop = (usize::try_from((v.pos >> 32).saturating_sub(1)).unwrap_or(0))
            .min(self.pcm.len() / ch);
        if drop > 0 {
            self.pcm.drain(..drop * ch);
            self.base += drop as u64;
            v.pos -= (drop as u64) << 32;
        }
        v.wave = Arc::new(Wave {
            frames: u32::try_from(self.pcm.len() / ch).unwrap_or(0),
            pcm: std::mem::take(&mut self.pcm),
            channels: self.channels,
            rate: self.rate,
            looped: None,
        });
        v.volume = self.volume;
        v.stereo = self.stereo;
        v.mix(block);
        v.ended = false;
        self.pcm = Arc::try_unwrap(std::mem::replace(
            &mut v.wave,
            empty(self.channels, self.rate),
        ))
        .map(|w| w.pcm)
        .unwrap_or_default();
        let after = self.base + (v.pos >> 32);
        self.mixed += after - before;
        self.voice = Some(v);
        let chunk = CHUNK as u64;
        usize::try_from(after / chunk - before / chunk).unwrap_or(0)
    }
}

/// Runs the read callback once: `(callback, sound, bytes)` into an arena
/// block, the samples back.
pub fn read(e: &mut Exec, (callback, sound, bytes): (u32, u32, u32)) -> Vec<i16> {
    let buf = e.alloc(bytes);
    if buf == 0 {
        return Vec::new();
    }
    let mut zero = vec![0u8; bytes as usize];
    e.mem().write_bytes(buf, &zero);
    let _ = e.call_stdcall(callback, &[sound, buf, bytes]);
    e.mem().read_bytes(buf, &mut zero);
    e.release(buf);
    zero.as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_sound::mixer::BLOCK;

    /// A stereo 48 kHz stream as the movie player's `createSound` makes it,
    /// with the two chunks `createSound` reads (`first`: their samples).
    fn stream(first: i16) -> Movie {
        let mut m = Movie {
            sound: 0x1_2501,
            channel: 0,
            read: 0x00a2_7ee0,
            channels: 2,
            rate: 48_000,
            voice: None,
            pcm: Vec::new(),
            base: 0,
            volume: 1.0,
            paused: false,
            stereo: false,
            fetched: 0,
            mixed: 0,
        };
        m.push(&vec![first; CHUNK * 2]);
        m.push(&vec![first; CHUNK * 2]);
        m
    }

    fn block() -> Vec<f32> {
        vec![0.0; BLOCK * 2]
    }

    #[test]
    fn the_stream_mixes_only_once_played_and_not_while_paused() {
        let mut m = stream(1000);
        assert_eq!(m.fetched, 2 * CHUNK as u64);
        assert_eq!(m.request(), Some((0x00a2_7ee0, 0x1_2501, 0x4000)));
        // before playSound: no channel, nothing mixed, nothing read
        let mut b = block();
        assert_eq!(m.mix(&mut b), 0);
        assert!(b.iter().all(|&x| x == 0.0));
        assert_eq!(m.mixed, 0);
        // paused (the game pauses it right after playSound)
        m.channel = m.sound | 4;
        m.paused = true;
        assert_eq!(m.mix(&mut b), 0);
        assert_eq!(m.mixed, 0);
        assert!(b.iter().all(|&x| x == 0.0));
        // resumed: one block of the stream's frames, the samples heard
        m.paused = false;
        m.mix(&mut b);
        assert_eq!(m.mixed, BLOCK as u64);
        assert!(b.iter().any(|&x| x != 0.0));
        // paused again: the position stands
        m.paused = true;
        let mut b = block();
        m.mix(&mut b);
        assert_eq!(m.mixed, BLOCK as u64);
        assert!(b.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn a_chunk_is_read_each_time_the_play_position_passes_a_multiple_of_one() {
        // FMOD's reads in the movie-fmv log (docs/formats/audio-platform.md,
        // "Residue"): for a start in update 350, updates 352, 355, 357, 360
        let mut m = stream(0);
        m.channel = m.sound | 4;
        let mut reads = Vec::new();
        for update in 350..361 {
            let n = m.mix(&mut block());
            for _ in 0..n {
                m.push(&vec![0; CHUNK * 2]);
                reads.push(update);
            }
        }
        assert_eq!(reads, [352, 355, 357, 360]);
        assert_eq!(m.mixed, 11 * BLOCK as u64);
        assert_eq!(m.fetched, 6 * CHUNK as u64);
    }

    #[test]
    fn the_samples_read_are_the_samples_played_in_order() {
        // a ramp over two chunks: the mix follows it, the frames played are
        // dropped, and what is kept stays aligned to the play position
        let mut m = stream(0);
        m.pcm.clear();
        m.fetched = 0;
        let ramp: Vec<i16> = (0..CHUNK * 2)
            .flat_map(|i| {
                let v = i16::try_from(i % 8000).unwrap();
                [v, -v]
            })
            .collect();
        m.push(&ramp);
        m.channel = m.sound | 4;
        let mut last = Vec::new();
        for _ in 0..4 {
            let mut b = block();
            m.mix(&mut b);
            last = b;
        }
        // the 4th block holds frames 4800..6400: left rises, right mirrors it
        let (l, r): (Vec<f32>, Vec<f32>) = last.chunks(2).map(|f| (f[0], f[1])).unzip();
        assert!(l.windows(2).skip(64).all(|w| w[1] > w[0]), "left rises");
        assert!(l.iter().zip(&r).skip(64).all(|(a, b)| (a + b).abs() < 1e-6));
        assert_eq!(m.base + m.pcm.len() as u64 / 2, 2 * CHUNK as u64);
        // dropped at the next mix: all but one frame before its start
        assert_eq!(m.base, 3 * BLOCK as u64 - 1);
    }
}
