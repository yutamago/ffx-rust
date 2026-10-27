//! IMA ADPCM as the game's banks store it (the Xbox layout): a mono block
//! is 0x24 bytes, a 4-byte header (predictor `i16`, step index `u8`, one
//! unused byte) and 32 bytes of 4-bit codes (low nibble first), 64 samples:
//! the predictor itself, then the first 63 codes (the 64th code is never
//! output; measured against FMOD's mix, where a block whose header differs
//! from the previous block's last code shows it at the block's first frame,
//! `docs/formats/audio-platform.md`, "FMOD Ex's mixer"); a stereo block is
//! 0x48 bytes: both channels' headers, then their codes in alternating
//! 4-byte words. The step and index tables are the IMA standard's. (A
//! stereo bank decoded with whole blocks per channel instead is noise: mean
//! step between samples 84 and 983 against 23 this way.)

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// Bytes of one channel's block.
pub const BLOCK: usize = 0x24;
/// Samples one block gives.
pub const BLOCK_SAMPLES: usize = 64;

/// One 4-bit code applied to the predictor and step index.
fn step(code: u8, predictor: &mut i32, index: &mut i32) -> i16 {
    let s = STEPS[usize::try_from(*index).unwrap_or(0)];
    let mut diff = s >> 3;
    if code & 4 != 0 {
        diff += s;
    }
    if code & 2 != 0 {
        diff += s >> 1;
    }
    if code & 1 != 0 {
        diff += s >> 2;
    }
    if code & 8 != 0 {
        *predictor -= diff;
    } else {
        *predictor += diff;
    }
    *predictor = (*predictor).clamp(-32768, 32767);
    *index = (*index + INDEX[usize::from(code)]).clamp(0, 88);
    i16::try_from(*predictor).unwrap_or(0)
}

/// One channel's state through a block.
struct Channel {
    predictor: i32,
    index: i32,
}

impl Channel {
    fn new(header: &[u8]) -> Self {
        Self {
            predictor: i32::from(i16::from_le_bytes([header[0], header[1]])),
            index: i32::from(header[2]).clamp(0, 88),
        }
    }

    /// The header's predictor: the block's first sample.
    fn sample(&self) -> i16 {
        i16::try_from(self.predictor).unwrap_or(0)
    }

    fn codes(&mut self, bytes: &[u8], out: &mut Vec<i16>) {
        for &byte in bytes {
            out.push(step(byte & 0xf, &mut self.predictor, &mut self.index));
            out.push(step(byte >> 4, &mut self.predictor, &mut self.index));
        }
    }
}

/// Decodes `frames` sample frames of `channels` (1 or 2) channels to
/// interleaved 16-bit PCM.
#[must_use]
pub fn decode(data: &[u8], channels: u16, frames: u32) -> Vec<i16> {
    let frames = frames as usize;
    if channels == 2 {
        let (mut l, mut r) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        for b in data.as_chunks::<{ 2 * BLOCK }>().0 {
            let (mut cl, mut cr) = (Channel::new(&b[0..4]), Channel::new(&b[4..8]));
            let start = l.len();
            l.push(cl.sample());
            r.push(cr.sample());
            for w in b[8..].as_chunks::<8>().0 {
                cl.codes(&w[..4], &mut l);
                cr.codes(&w[4..], &mut r);
            }
            // the block's 64th code is never output
            l.truncate(start + BLOCK_SAMPLES);
            r.truncate(start + BLOCK_SAMPLES);
        }
        let n = frames.min(l.len()).min(r.len());
        return l[..n]
            .iter()
            .zip(&r[..n])
            .flat_map(|(&a, &b)| [a, b])
            .collect();
    }
    let mut out = Vec::with_capacity(frames);
    for b in data.as_chunks::<BLOCK>().0 {
        let start = out.len();
        let mut c = Channel::new(&b[..4]);
        out.push(c.sample());
        c.codes(&b[4..], &mut out);
        out.truncate(start + BLOCK_SAMPLES);
    }
    out.truncate(frames);
    out
}
