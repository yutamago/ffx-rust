//! Vertical flipping. The textures are stored bottom row first (the PS3
//! version's convention, kept by the PC port); DDS files are top row first.
//! Flipping is its own inverse, so export and import use the same code.

use crate::texture::Format;

/// Flips one mip level of `height` pixel rows in place.
pub(crate) fn flip_level(format: Format, width: u32, height: u32, level: &mut [u8]) {
    match format {
        Format::Argb8 | Format::Rgba8 | Format::L8 => {
            let row = width as usize * format.bits() as usize / 8;
            let rows = height as usize;
            for y in 0..rows / 2 {
                let (top, bottom) = level.split_at_mut((rows - 1 - y) * row);
                top[y * row..(y + 1) * row].swap_with_slice(&mut bottom[..row]);
            }
        }
        Format::Dxt1 | Format::Dxt3 | Format::Dxt5 | Format::Bc4 | Format::Bc5 => {
            let block = format.bits() as usize * 2;
            let row = width.div_ceil(4).max(1) as usize * block;
            let block_rows = height.div_ceil(4).max(1) as usize;
            // Rows of blocks swap ends...
            for y in 0..block_rows / 2 {
                let (top, bottom) = level.split_at_mut((block_rows - 1 - y) * row);
                top[y * row..(y + 1) * row].swap_with_slice(&mut bottom[..row]);
            }
            // ...and inside each block the pixel rows that exist reverse.
            let valid = height.min(4) as usize;
            for b in level[..block_rows * row].chunks_exact_mut(block) {
                match format {
                    Format::Dxt1 => flip_color(&mut b[..8], valid),
                    Format::Dxt3 => {
                        reverse_units(&mut b[..8], 2, valid);
                        flip_color(&mut b[8..], valid);
                    }
                    Format::Dxt5 => {
                        flip_alpha5(&mut b[..8], valid);
                        flip_color(&mut b[8..], valid);
                    }
                    // One or two alpha-style channel blocks.
                    _ => {
                        for channel in b.as_chunks_mut::<8>().0 {
                            flip_alpha5(channel, valid);
                        }
                    }
                }
            }
        }
    }
}

/// Reverses the first `count` units of `size` bytes.
fn reverse_units(bytes: &mut [u8], size: usize, count: usize) {
    for i in 0..count / 2 {
        let (a, b) = (i * size, (count - 1 - i) * size);
        for k in 0..size {
            bytes.swap(a + k, b + k);
        }
    }
}

/// A DXT color block: two endpoints, then one index byte per pixel row.
fn flip_color(block: &mut [u8], valid: usize) {
    reverse_units(&mut block[4..8], 1, valid);
}

/// A DXT5 alpha block: two endpoints, then 4 rows of 12 bits of indices.
fn flip_alpha5(block: &mut [u8], valid: usize) {
    let bits = block[2..8]
        .iter()
        .rev()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
    let row = |r: usize| (bits >> (12 * r)) & 0xfff;
    let mut rows: Vec<u64> = (0..4).map(row).collect();
    rows[..valid].reverse();
    let out = rows
        .iter()
        .enumerate()
        .fold(0u64, |acc, (r, &v)| acc | (v << (12 * r)));
    for (i, b) in block[2..8].iter_mut().enumerate() {
        *b = (out >> (8 * i)).to_le_bytes()[0];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_are_involutions() {
        for format in Format::ALL {
            for (w, h) in [(8, 8), (16, 4), (8, 2), (4, 1), (12, 20)] {
                let size = format.level_size(w, h);
                let original: Vec<u8> = (0..size)
                    .map(|i| u8::try_from(i * 37 % 251).unwrap())
                    .collect();
                let mut data = original.clone();
                flip_level(format, w, h, &mut data);
                if h > 1 {
                    assert_ne!(data, original, "{format:?} {w}x{h} unchanged");
                }
                flip_level(format, w, h, &mut data);
                assert_eq!(data, original, "{format:?} {w}x{h}");
            }
        }
    }

    #[test]
    fn raw_rows_swap() {
        let mut d = vec![1, 2, 3, 4, 5, 6];
        flip_level(Format::L8, 2, 3, &mut d);
        assert_eq!(d, [5, 6, 3, 4, 1, 2]);
    }
}
