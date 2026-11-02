//! Decoding the top mip level to upright RGBA8, and writing it as PNG.

use std::io::Write;

use flate2::Crc;
use flate2::write::ZlibEncoder;

use crate::texture::{Format, Texture};

/// RGB565 to RGB888.
fn rgb565(c: u16) -> [u8; 3] {
    let (r, g, b) = (c >> 11, (c >> 5) & 0x3f, c & 0x1f);
    #[allow(clippy::cast_possible_truncation)] // all at most 255
    [
        ((r << 3) | (r >> 2)) as u8,
        ((g << 2) | (g >> 4)) as u8,
        ((b << 3) | (b >> 2)) as u8,
    ]
}

#[allow(clippy::cast_possible_truncation)] // weighted averages of bytes
fn mix(a: [u8; 3], b: [u8; 3], wa: u16, wb: u16) -> [u8; 3] {
    std::array::from_fn(|k| ((u16::from(a[k]) * wa + u16::from(b[k]) * wb) / (wa + wb)) as u8)
}

/// One DXT color block: 16 RGBA pixels. `dxt1` enables the 3-color mode
/// with transparent black.
fn color_block(b: &[u8], dxt1: bool) -> [[u8; 4]; 16] {
    let (c0, c1) = (
        u16::from_le_bytes([b[0], b[1]]),
        u16::from_le_bytes([b[2], b[3]]),
    );
    let (p0, p1) = (rgb565(c0), rgb565(c1));
    let opaque = |c: [u8; 3]| [c[0], c[1], c[2], 255];
    let palette = if c0 > c1 || !dxt1 {
        [
            opaque(p0),
            opaque(p1),
            opaque(mix(p0, p1, 2, 1)),
            opaque(mix(p0, p1, 1, 2)),
        ]
    } else {
        [
            opaque(p0),
            opaque(p1),
            opaque(mix(p0, p1, 1, 1)),
            [0, 0, 0, 0],
        ]
    };
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    std::array::from_fn(|i| palette[(bits >> (2 * i) & 3) as usize])
}

/// A DXT5 alpha block: 16 alpha values.
fn alpha_block(b: &[u8]) -> [u8; 16] {
    let (a0, a1) = (u16::from(b[0]), u16::from(b[1]));
    #[allow(clippy::cast_possible_truncation)] // interpolations of bytes
    let palette: [u8; 8] = std::array::from_fn(|i| {
        let i = i as u16;
        match i {
            0 => a0 as u8,
            1 => a1 as u8,
            _ if a0 > a1 => (((8 - i) * a0 + (i - 1) * a1) / 7) as u8,
            2..=5 => (((6 - i) * a0 + (i - 1) * a1) / 5) as u8,
            6 => 0,
            _ => 255,
        }
    });
    let bits = b[2..8]
        .iter()
        .rev()
        .fold(0u64, |acc, &x| acc << 8 | u64::from(x));
    std::array::from_fn(|i| palette[(bits >> (3 * i) & 7) as usize])
}

impl Texture {
    /// The top mip level (the first face of cube maps) as RGBA8 rows, top
    /// row first.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let (w, h) = (self.width as usize, self.height as usize);
        let src = self.pixels();
        let mut out = vec![0u8; w * h * 4];
        // Stored rows are bottom first; `y` counts from the bottom.
        let mut put = |x: usize, y: usize, px: [u8; 4]| {
            if x < w && y < h {
                let o = ((h - 1 - y) * w + x) * 4;
                out[o..o + 4].copy_from_slice(&px);
            }
        };
        match self.format {
            Format::Dxt1 | Format::Dxt3 | Format::Dxt5 | Format::Bc4 | Format::Bc5 => {
                let block_len = self.format.bits() as usize * 2;
                let bw = w.div_ceil(4);
                for (n, b) in src[..self.format.level_size(self.width, self.height)]
                    .chunks(block_len)
                    .enumerate()
                {
                    let (bx, by) = (n % bw, n / bw);
                    let mut px = match self.format {
                        Format::Dxt1 => color_block(b, true),
                        // One channel as grey; two as red and green.
                        Format::Bc4 => alpha_block(b).map(|v| [v, v, v, 255]),
                        Format::Bc5 => {
                            let (r, g) = (alpha_block(&b[..8]), alpha_block(&b[8..]));
                            std::array::from_fn(|i| [r[i], g[i], 0, 255])
                        }
                        _ => color_block(&b[8..], false),
                    };
                    match self.format {
                        Format::Dxt3 => {
                            for (i, p) in px.iter_mut().enumerate() {
                                let a = b[i / 2] >> (4 * (i % 2)) & 0xf;
                                p[3] = a << 4 | a;
                            }
                        }
                        Format::Dxt5 => {
                            for (p, a) in px.iter_mut().zip(alpha_block(b)) {
                                p[3] = a;
                            }
                        }
                        _ => {}
                    }
                    for (i, p) in px.into_iter().enumerate() {
                        put(bx * 4 + i % 4, by * 4 + i / 4, p);
                    }
                }
            }
            Format::Argb8 => {
                for (i, p) in src[..w * h * 4].chunks(4).enumerate() {
                    put(i % w, i / w, [p[2], p[1], p[0], p[3]]);
                }
            }
            Format::Rgba8 => {
                for (i, p) in src[..w * h * 4].chunks(4).enumerate() {
                    put(i % w, i / w, [p[0], p[1], p[2], p[3]]);
                }
            }
            Format::L8 => {
                for (i, &l) in src[..w * h].iter().enumerate() {
                    put(i % w, i / w, [l, l, l, 255]);
                }
            }
        }
        out
    }

    /// The top mip level as an upright RGBA PNG.
    #[must_use]
    pub fn to_png(&self) -> Vec<u8> {
        png(self.width, self.height, &self.to_rgba())
    }
}

/// A minimal RGBA8 PNG.
#[must_use]
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let row = width as usize * 4;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for r in rgba.chunks(row) {
        raw.push(0);
        raw.extend(r);
    }
    let mut z = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(&raw).expect("in-memory write");
    let idat = z.finish().expect("in-memory write");
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend(
            u32::try_from(data.len())
                .expect("chunk under 4 GiB")
                .to_be_bytes(),
        );
        out.extend(kind);
        out.extend(data);
        let mut crc = Crc::new();
        crc.update(kind);
        crc.update(data);
        out.extend(crc.sum().to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend(width.to_be_bytes());
    ihdr.extend(height.to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &idat);
    chunk(b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_color_and_alpha_blocks() {
        // White and black endpoints; indices 0 (white) and 1 (black) alternate.
        let px = color_block(&[0xff, 0xff, 0, 0, 0x44, 0x44, 0x44, 0x44], true);
        assert_eq!(px[0], [255, 255, 255, 255]);
        assert_eq!(px[1], [0, 0, 0, 255]);
        // DXT1 3-color mode: index 3 is transparent black.
        let px = color_block(&[0, 0, 0xff, 0xff, 0xff, 0, 0, 0], true);
        assert_eq!(px[0], [0, 0, 0, 0]);
        // DXT5 alpha: endpoints 255 and 0, index 1 everywhere.
        let a = alpha_block(&[
            255,
            0,
            0b0100_1001,
            0b1001_0010,
            0b0010_0100,
            0b0100_1001,
            0b1001_0010,
            0b0010_0100,
        ]);
        assert!(a.iter().all(|&x| x == 0));
    }

    #[test]
    fn writes_a_png_signature_and_chunks() {
        let png = png(1, 1, &[1, 2, 3, 4]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}
