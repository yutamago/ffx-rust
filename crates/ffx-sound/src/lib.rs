//! Music, sound effects and voice (the PS2 `Snd` module, the FMOD layer).
//!
//! The `sound` module of `docs/rewrite-plan.md`: the sound data
//! ([`fsb`] banks of [`ima`] ADPCM samples) and the ported game-side sound
//! logic, a core and a `bridge` per original (`docs/porting.md`): the PS2
//! sound API ([`snd`]), the FMOD command queue's entry points ([`fmod`])
//! and the FMOD thread's handlers and objects ([`ffxfmod`]).

pub mod ffxfmod;
pub mod fmod;
pub mod fsb;
pub mod ima;
pub mod snd;

/// A WAV file of 16-bit PCM.
#[must_use]
pub fn wav(pcm: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data = u32::try_from(pcm.len() * 2).unwrap_or(u32::MAX);
    let mut w = Vec::with_capacity(44 + pcm.len() * 2);
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + data).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    w.extend_from_slice(&(channels * 2).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data.to_le_bytes());
    for s in pcm {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}
