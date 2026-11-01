//! The movie's user stream (`System::createSound` with `FMOD_OPENUSER`,
//! `CREATESTREAM`): mixed from the game's PCM read callback at the
//! stream's rate, interpolated and ramped like every voice, as the hooked
//! game's Rust engine does (`crates/ffx-hook/src/rust_audio.rs`, "The
//! movie's user stream"): two chunks are read in `createSound` (before the
//! game sets the user data: silence), then one each time the play position
//! passes a multiple of [`CHUNK`] frames, in that update. Not measured
//! against FMOD yet (the hook's isn't either).
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
