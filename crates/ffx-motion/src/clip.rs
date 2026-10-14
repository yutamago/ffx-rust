//! One motion clip: the animation of a skeleton's bones over a number of
//! frames (`SG_SetClip`, `0x00839980`; stepped by `SG_StepChannels`,
//! `0x008394d0`).
//!
//! ```text
//! +0x00  u16  frame count
//! +0x02  u16  bone count
//! +0x04  u16  frame rate × 256 (0x1e00: 30 frames a second)
//! +0x06  u16  event count
//! +0x08  u32  offset of the channel types (the header ends there: 0x10 or 0x18)
//! +0x0c  u32  offset of the channel values
//! +0x10  u32  offset of the events (headers of 0x18 bytes)
//! +0x14  u32  0
//! ```
//!
//! Every bone has nine channels: rotation x, y, z (1/4096 turn), translation
//! x, y, z (1/100 unit) and scale x, y, z (1/4096). Each channel's type takes
//! two bits, lowest first: 0 is 0, 1 is 4096, 2 is a constant `i16` from the
//! values, 3 is a key stream from the values (`i16` byte length including
//! itself, then the bytes). A stream holds one delta per frame, added to the
//! value (which starts at 0):
//!
//! - `0xxxxxxx`: a 7-bit signed delta;
//! - `11xxxxxx yyyyyyyy`: a 14-bit signed delta, `y` the high byte;
//! - `10nnnnnn`: the previous delta again, for this frame and `n` more.
//!
//! Events (8 bytes each) follow the values, 4-byte aligned.

use crate::{Error, Result};

/// One of a bone's nine channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    /// Type 0: always 0.
    Zero,
    /// Type 1: always 4096 (1.0).
    One,
    /// Type 2: a constant.
    Constant(i16),
    /// Type 3: one value per frame.
    Keys(Vec<i16>),
}

impl Channel {
    /// The channel's value at `frame` (the last key past the end).
    #[must_use]
    pub fn at(&self, frame: usize) -> i16 {
        match self {
            Self::Zero => 0,
            Self::One => 4096,
            Self::Constant(v) => *v,
            Self::Keys(k) => k.get(frame).or(k.last()).copied().unwrap_or(0),
        }
    }

    fn kind(&self) -> u8 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::Constant(_) => 2,
            Self::Keys(_) => 3,
        }
    }
}

/// A motion clip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub frames: u16,
    /// Frames per second × 256.
    pub rate: u16,
    /// Nine channels per bone.
    pub channels: Vec<Channel>,
    /// Events (four words each; their meaning is not known yet).
    pub events: Vec<[u16; 4]>,
    /// Header length, 0x10 or 0x18 (kept so the file is rebuilt exactly).
    pub header_len: u32,
    /// The header word at +0x14.
    pub word14: u32,
    /// The uncompressed format (one weapon clip, played by `0x00838750`):
    /// the bits of its `f32` frame rate.
    pub raw_rate: Option<u32>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(Error::Truncated(o))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Truncated(o))
}

/// Decodes `frames` keys from a stream; returns them and the bytes used.
fn decode_keys(stream: &[u8], frames: usize) -> Result<(Vec<i16>, usize)> {
    let mut keys = Vec::with_capacity(frames);
    let (mut value, mut delta, mut repeat, mut at) = (0i16, 0i16, 0u8, 0);
    for _ in 0..frames {
        if repeat == 0 {
            let b = *stream.get(at).ok_or(Error::Truncated(at))?;
            at += 1;
            if b & 0x80 == 0 {
                delta = i16::from((b << 1).cast_signed()) >> 1;
            } else if b & 0x40 == 0 {
                repeat = b & 0x3f;
            } else {
                let hi = *stream.get(at).ok_or(Error::Truncated(at))?;
                at += 1;
                let raw = (u16::from(hi) << 6 | u16::from(b & 0x3f)) << 2;
                delta = raw.cast_signed() >> 2;
            }
        } else {
            repeat -= 1;
        }
        value = value.wrapping_add(delta);
        keys.push(value);
    }
    Ok((keys, at))
}

/// Encodes keys as a stream, the inverse of [`decode_keys`]. Like the
/// shipped data, a run of the previous delta (at most 64 frames) is never
/// directly followed by another run: the delta is written again first.
fn encode_keys(keys: &[i16]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let (mut value, mut delta, mut after_run) = (0i16, 0i16, false);
    let mut i = 0;
    while i < keys.len() {
        let d = keys[i].wrapping_sub(value);
        if d == delta && !after_run {
            let mut n = 1;
            while n < 64 && i + n < keys.len() && keys[i + n].wrapping_sub(keys[i + n - 1]) == delta
            {
                n += 1;
            }
            out.push(0x80 | u8::try_from(n - 1).expect("under 64"));
            value = keys[i + n - 1];
            i += n;
            after_run = true;
            continue;
        }
        if (-64..64).contains(&d) {
            out.push(d.to_le_bytes()[0] & 0x7f);
        } else if (-8192..8192).contains(&d) {
            let raw = d.cast_unsigned() & 0x3fff;
            out.push(0xc0 | (raw & 0x3f) as u8);
            out.push((raw >> 6) as u8);
        } else {
            return Err(Error::Unencodable(d));
        }
        delta = d;
        value = keys[i];
        after_run = false;
        i += 1;
    }
    Ok(out)
}

impl Clip {
    /// Number of bones.
    #[must_use]
    pub fn bones(&self) -> usize {
        self.channels.len() / 9
    }

    /// A bone's nine channel values at `frame`.
    #[must_use]
    pub fn bone_at(&self, bone: usize, frame: usize) -> [i16; 9] {
        std::array::from_fn(|k| self.channels[bone * 9 + k].at(frame))
    }

    /// Parses the clip at `at`. Returns it and the end of its values.
    ///
    /// # Errors
    /// Fails on truncated data.
    pub fn parse(b: &[u8], at: usize) -> Result<(Self, usize)> {
        let frames = u16_at(b, at)?;
        let bones = usize::from(u16_at(b, at + 2)?);
        if !matches!(u32_at(b, at + 8)?, 0x10 | 0x18) {
            return Self::parse_raw(b, at, frames, bones);
        }
        let rate = u16_at(b, at + 4)?;
        let event_count = usize::from(u16_at(b, at + 6)?);
        let header_len = u32_at(b, at + 8)?;
        let values = at + u32_at(b, at + 0xc)? as usize;
        let (events_at, word14) = if header_len >= 0x18 {
            (u32_at(b, at + 0x10)? as usize, u32_at(b, at + 0x14)?)
        } else {
            (0, 0)
        };
        let types = at + header_len as usize;
        let mut p = values;
        let mut channels = Vec::with_capacity(bones * 9);
        for i in 0..bones * 9 {
            let byte = *b
                .get(types + i / 4)
                .ok_or(Error::Truncated(types + i / 4))?;
            channels.push(match (byte >> (2 * (i % 4))) & 3 {
                0 => Channel::Zero,
                1 => Channel::One,
                2 => {
                    p += 2;
                    Channel::Constant(u16_at(b, p - 2)?.cast_signed())
                }
                _ => {
                    let len = usize::from(u16_at(b, p)?);
                    let stream = b.get(p + 2..p + len).ok_or(Error::Truncated(p))?;
                    let (keys, used) = decode_keys(stream, usize::from(frames))?;
                    if used + 2 > len {
                        return Err(Error::Malformed("key stream overruns its length"));
                    }
                    p += len;
                    Channel::Keys(keys)
                }
            });
        }
        let events = (0..event_count)
            .map(|e| {
                let o = at + events_at + 8 * e;
                Ok([
                    u16_at(b, o)?,
                    u16_at(b, o + 2)?,
                    u16_at(b, o + 4)?,
                    u16_at(b, o + 6)?,
                ])
            })
            .collect::<Result<Vec<_>>>()?;
        let end = if event_count > 0 {
            at + events_at + 8 * event_count
        } else {
            p
        };
        Ok((
            Self {
                frames,
                rate,
                channels,
                events,
                header_len,
                word14,
                raw_rate: None,
            },
            end,
        ))
    }

    /// The uncompressed format: `u16` frames, `u16` bones, `f32` frame
    /// rate, 8 zero bytes, then per bone and channel one `i16` per frame.
    fn parse_raw(b: &[u8], at: usize, frames: u16, bones: usize) -> Result<(Self, usize)> {
        let rate_bits = u32_at(b, at + 4)?;
        let n = usize::from(frames);
        let data = at + 0x10;
        let channels = (0..bones * 9)
            .map(|c| {
                (0..n)
                    .map(|f| Ok(u16_at(b, data + 2 * (c * n + f))?.cast_signed()))
                    .collect::<Result<Vec<_>>>()
                    .map(Channel::Keys)
            })
            .collect::<Result<Vec<_>>>()?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a frame rate
        let rate = (f32::from_bits(rate_bits) * 256.0) as u16;
        Ok((
            Self {
                frames,
                rate,
                channels,
                events: Vec::new(),
                header_len: 0x10,
                word14: 0,
                raw_rate: Some(rate_bits),
            },
            data + 2 * bones * 9 * n,
        ))
    }

    /// Appends the clip to `out` (which must be 8-byte aligned).
    ///
    /// # Errors
    /// Fails when a delta does not fit 14 bits or a stream exceeds 32 KiB.
    pub fn write(&self, out: &mut Vec<u8>) -> Result<()> {
        let at = out.len();
        let bones = self.bones();
        if let Some(rate) = self.raw_rate {
            out.extend(self.frames.to_le_bytes());
            out.extend(u16::try_from(bones).expect("bone count").to_le_bytes());
            out.extend(rate.to_le_bytes());
            out.extend([0; 8]);
            for c in &self.channels {
                for f in 0..usize::from(self.frames) {
                    out.extend(c.at(f).to_le_bytes());
                }
            }
            return Ok(());
        }
        let types_len = (bones * 9).div_ceil(4);
        let values_at = (self.header_len as usize + types_len).next_multiple_of(2);
        let mut types = vec![0u8; types_len];
        let mut values = Vec::new();
        for (i, c) in self.channels.iter().enumerate() {
            types[i / 4] |= c.kind() << (2 * (i % 4));
            match c {
                Channel::Constant(v) => values.extend(v.to_le_bytes()),
                Channel::Keys(k) => {
                    let mut stream = encode_keys(k)?;
                    if stream.len() % 2 == 1 {
                        stream.push(0);
                    }
                    let len = u16::try_from(stream.len() + 2)
                        .map_err(|_| Error::Malformed("key stream over 64 KiB"))?;
                    values.extend(len.to_le_bytes());
                    values.extend(stream);
                }
                Channel::Zero | Channel::One => {}
            }
        }
        let events_at = (values_at + values.len()).next_multiple_of(4);
        out.extend(self.frames.to_le_bytes());
        out.extend(u16::try_from(bones).expect("bone count").to_le_bytes());
        out.extend(self.rate.to_le_bytes());
        out.extend(
            u16::try_from(self.events.len())
                .expect("event count")
                .to_le_bytes(),
        );
        out.extend(self.header_len.to_le_bytes());
        out.extend(u32::try_from(values_at).expect("small").to_le_bytes());
        if self.header_len >= 0x18 {
            let events = if self.events.is_empty() { 0 } else { events_at };
            out.extend(u32::try_from(events).expect("small").to_le_bytes());
            out.extend(self.word14.to_le_bytes());
        }
        out.extend(types);
        out.resize(at + values_at, crate::PAD);
        out.extend(values);
        if !self.events.is_empty() {
            out.resize(at + events_at, crate::PAD);
            for e in &self.events {
                for w in e {
                    out.extend(w.to_le_bytes());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        let keys: Vec<i16> = vec![1000, 1000, 1000, 1001, 1002, 1003, 1003, -5000, 3000, 3063];
        let stream = encode_keys(&keys).unwrap();
        assert_eq!(
            decode_keys(&stream, keys.len()).unwrap(),
            (keys, stream.len())
        );
    }

    #[test]
    fn decodes_each_byte_form() {
        // Absolute 0x123 (14-bit), then +3 (7-bit), then +3 for 1 + 2 frames.
        let stream = [0xc0 | 0x23, 0x04, 0x03, 0x82];
        let (keys, used) = decode_keys(&stream, 5).unwrap();
        assert_eq!(keys, [0x123, 0x126, 0x129, 0x12c, 0x12f]);
        assert_eq!(used, 4);
    }
}
