//! A minimal PNG writer (RGBA8), for replays and tests, and a reader of
//! 8-bit RGB and RGBA images (material packs).

fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}

fn chunk(out: &mut Vec<u8>, tag: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32::try_from(data.len()).expect("chunk size").to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&tag);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// Encodes `rgba` (rows top first) as a PNG.
#[must_use]
pub fn encode(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width * height * 4 + height) as usize);
    for row in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, *b"IHDR", &ihdr);
    chunk(
        &mut out,
        *b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(&mut out, *b"IEND", &[]);
    out
}

/// Decodes an 8-bit, non-interlaced RGB or RGBA PNG to RGBA8 (rows top
/// first); `None` for anything else.
#[must_use]
pub fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.get(..8)? != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let mut at = 8;
    let (mut width, mut height, mut channels) = (0u32, 0u32, 0usize);
    let mut data = Vec::new();
    while at + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[at..at + 4].try_into().ok()?) as usize;
        let tag = &bytes[at + 4..at + 8];
        let body = bytes.get(at + 8..at + 8 + len)?;
        match tag {
            b"IHDR" => {
                width = u32::from_be_bytes(body[0..4].try_into().ok()?);
                height = u32::from_be_bytes(body[4..8].try_into().ok()?);
                let (depth, kind, interlace) = (body[8], body[9], body[12]);
                channels = match (depth, kind, interlace) {
                    (8, 2, 0) => 3,
                    (8, 6, 0) => 4,
                    _ => return None,
                };
            }
            b"IDAT" => data.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    if channels == 0 || width == 0 || height == 0 {
        return None;
    }
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&data).ok()?;
    let stride = width as usize * channels;
    if raw.len() < (stride + 1) * height as usize {
        return None;
    }
    let mut pixels = vec![0u8; stride * height as usize];
    for y in 0..height as usize {
        let filter = raw[y * (stride + 1)];
        let line = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        for x in 0..stride {
            let a = if x >= channels {
                pixels[y * stride + x - channels]
            } else {
                0
            };
            let b = if y > 0 {
                pixels[(y - 1) * stride + x]
            } else {
                0
            };
            let c = if x >= channels && y > 0 {
                pixels[(y - 1) * stride + x - channels]
            } else {
                0
            };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
                4 => {
                    let p = i16::from(a) + i16::from(b) - i16::from(c);
                    let (pa, pb, pc) = (
                        (p - i16::from(a)).abs(),
                        (p - i16::from(b)).abs(),
                        (p - i16::from(c)).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return None,
            };
            pixels[y * stride + x] = line[x].wrapping_add(predicted);
        }
    }
    if channels == 4 {
        return Some((width, height, pixels));
    }
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for px in pixels.chunks_exact(3) {
        rgba.extend_from_slice(px);
        rgba.push(255);
    }
    Some((width, height, rgba))
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_what_it_encodes() {
        let rgba: Vec<u8> = (0..16 * 9 * 4).map(|i| (i * 7 % 251) as u8).collect();
        let png = super::encode(16, 9, &rgba);
        assert_eq!(super::decode(&png), Some((16, 9, rgba)));
    }
}
