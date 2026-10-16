//! FSB5 sound banks (`sound_pc/**/*.fsb`): the samples FMOD plays for the
//! game's music, voice and sound effects (`docs/formats/sound.md`).
//!
//! Every bank of the game is FSB5 version 1 with IMA ADPCM samples (codec
//! 7, [`crate::ima`]).

/// Errors reading a bank.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not an FSB5 bank")]
    Magic,
    #[error("FSB5 version {0} (only 1 is known)")]
    Version(u32),
    #[error("truncated bank")]
    Truncated,
}

/// Sample rates by the 4-bit index in a sample header.
const RATES: [u32; 11] = [
    4000, 8000, 11000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 96000,
];

/// One sample of a bank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// Its name in the bank's name table (the game's sound number).
    pub name: String,
    /// Samples per second.
    pub rate: u32,
    /// 1 or 2.
    pub channels: u16,
    /// Sample frames.
    pub frames: u32,
    /// Byte range of its data within the bank.
    pub data: std::ops::Range<usize>,
    /// Loop start and end (frames), from a loop chunk.
    pub looped: Option<(u32, u32)>,
}

/// A bank: its codec and samples.
#[derive(Debug, Clone)]
pub struct Bank {
    /// FMOD codec number (7: IMA ADPCM).
    pub codec: u32,
    /// The samples, in bank order.
    pub samples: Vec<Sample>,
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Truncated)
}

impl Bank {
    /// Reads a bank. The header is 0x3c bytes: magic `FSB5`, version,
    /// sample count, sample header size, name table size, data size,
    /// codec, then flags and a hash; sample headers, the name table and the
    /// data follow in that order.
    ///
    /// # Errors
    /// When the bytes are not a version 1 FSB5 bank.
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        if b.get(..4) != Some(b"FSB5") {
            return Err(Error::Magic);
        }
        let version = u32_at(b, 4)?;
        if version != 1 {
            return Err(Error::Version(version));
        }
        let count = u32_at(b, 8)? as usize;
        let headers = u32_at(b, 12)? as usize;
        let names = u32_at(b, 16)? as usize;
        let data_size = u32_at(b, 20)? as usize;
        let codec = u32_at(b, 24)?;
        let names_at = 0x3c + headers;
        let data_at = names_at + names;
        if b.len() < data_at + data_size {
            return Err(Error::Truncated);
        }

        // Each header: 64 bits (has chunks: 1, rate index: 4, stereo: 1,
        // data offset / 16: 28, frames: 30), then chunks of a 32-bit word
        // (more follow: 1, size: 24, type: 7) and `size` bytes.
        let mut at = 0x3c;
        let mut raw = Vec::with_capacity(count);
        for _ in 0..count {
            let lo = u64::from(u32_at(b, at)?);
            let hi = u64::from(u32_at(b, at + 4)?);
            let v = lo | hi << 32;
            at += 8;
            let mut more = v & 1 != 0;
            let rate = RATES
                .get(((v >> 1) & 0xf) as usize)
                .copied()
                .unwrap_or(44100);
            let channels = if (v >> 5) & 1 != 0 { 2 } else { 1 };
            let offset =
                usize::try_from(((v >> 6) & 0x0fff_ffff) * 16).map_err(|_| Error::Truncated)?;
            let frames = u32::try_from(v >> 34).map_err(|_| Error::Truncated)?;
            let mut looped = None;
            let mut rate = rate;
            let mut channels = channels;
            while more {
                let c = u32_at(b, at)?;
                at += 4;
                more = c & 1 != 0;
                let size = ((c >> 1) & 0x00ff_ffff) as usize;
                let kind = c >> 25;
                let body = b.get(at..at + size).ok_or(Error::Truncated)?;
                match kind {
                    1 if size >= 1 => channels = u16::from(body[0]),
                    2 if size >= 4 => rate = u32_at(body, 0)?,
                    3 if size >= 8 => looped = Some((u32_at(body, 0)?, u32_at(body, 4)?)),
                    _ => {}
                }
                at += size;
            }
            raw.push((rate, channels, offset, frames, looped));
        }

        let mut samples = Vec::with_capacity(count);
        for (i, &(rate, channels, offset, frames, looped)) in raw.iter().enumerate() {
            let end = raw.get(i + 1).map_or(data_size, |r| r.2);
            let name = if names >= 4 * count {
                let off = u32_at(b, names_at + 4 * i)? as usize;
                let s = b.get(names_at + off..data_at).unwrap_or(&[]);
                String::from_utf8_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())])
                    .into_owned()
            } else {
                i.to_string()
            };
            samples.push(Sample {
                name,
                rate,
                channels,
                frames,
                data: data_at + offset..data_at + end.max(offset),
                looped,
            });
        }
        Ok(Self { codec, samples })
    }
}
