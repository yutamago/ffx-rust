//! Minimal DDS reading and writing: the classic 124-byte header, for the
//! formats the game uses.

/// The DDS pixel formats we write and accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    FourCc([u8; 4]),
    /// 32-bit with masks R `0x00ff0000`, G `0x0000ff00`, B `0x000000ff`,
    /// A `0xff000000` (bytes B, G, R, A).
    Bgra8,
    /// 8-bit luminance.
    L8,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DdsError {
    #[error("not a DDS file")]
    NotDds,
    #[error("unsupported DDS pixel format")]
    UnsupportedFormat,
}

/// The parts of a DDS header the game's textures need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dds {
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub cube: bool,
    pub format: PixelFormat,
    /// Bytes of the top mip level (`dwPitchOrLinearSize` for compressed
    /// formats).
    pub top_level_size: usize,
}

const DDSD_CAPS: u32 = 0x1;
const DDSD_HEIGHT: u32 = 0x2;
const DDSD_WIDTH: u32 = 0x4;
const DDSD_PITCH: u32 = 0x8;
const DDSD_PIXELFORMAT: u32 = 0x1000;
const DDSD_MIPMAPCOUNT: u32 = 0x2_0000;
const DDSD_LINEARSIZE: u32 = 0x8_0000;
const DDPF_ALPHAPIXELS: u32 = 0x1;
const DDPF_FOURCC: u32 = 0x4;
const DDPF_RGB: u32 = 0x40;
const DDPF_LUMINANCE: u32 = 0x2_0000;
const DDSCAPS_COMPLEX: u32 = 0x8;
const DDSCAPS_TEXTURE: u32 = 0x1000;
const DDSCAPS_MIPMAP: u32 = 0x40_0000;
const DDSCAPS2_CUBEMAP_ALL: u32 = 0xfe00;

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes"))
}

impl Dds {
    /// Writes a DDS file: header, then `pixels` as they are.
    #[must_use]
    pub fn write(&self, pixels: &[u8]) -> Vec<u8> {
        let compressed = matches!(self.format, PixelFormat::FourCc(_));
        let mut flags = DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PIXELFORMAT | DDSD_MIPMAPCOUNT;
        flags |= if compressed {
            DDSD_LINEARSIZE
        } else {
            DDSD_PITCH
        };
        let pitch = match self.format {
            PixelFormat::FourCc(_) => u32::try_from(self.top_level_size).unwrap_or(u32::MAX),
            PixelFormat::Bgra8 => self.width * 4,
            PixelFormat::L8 => self.width,
        };
        let (pf_flags, four_cc, bits, masks) = match self.format {
            PixelFormat::FourCc(cc) => (DDPF_FOURCC, cc, 0, [0; 4]),
            PixelFormat::Bgra8 => (
                DDPF_RGB | DDPF_ALPHAPIXELS,
                [0; 4],
                32,
                [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000],
            ),
            PixelFormat::L8 => (DDPF_LUMINANCE, [0; 4], 8, [0xff, 0, 0, 0]),
        };
        let mut caps = DDSCAPS_TEXTURE;
        if self.mip_levels > 1 {
            caps |= DDSCAPS_MIPMAP | DDSCAPS_COMPLEX;
        }
        if self.cube {
            caps |= DDSCAPS_COMPLEX;
        }
        let mut out = Vec::with_capacity(128 + pixels.len());
        out.extend(b"DDS ");
        for v in [
            124,
            flags,
            self.height,
            self.width,
            pitch,
            0,
            self.mip_levels,
        ] {
            out.extend(u32::to_le_bytes(v));
        }
        out.extend([0u8; 44]);
        out.extend(32u32.to_le_bytes());
        out.extend(pf_flags.to_le_bytes());
        out.extend(four_cc);
        out.extend(u32::to_le_bytes(bits));
        for m in masks {
            out.extend(u32::to_le_bytes(m));
        }
        out.extend(caps.to_le_bytes());
        out.extend(u32::to_le_bytes(if self.cube {
            DDSCAPS2_CUBEMAP_ALL
        } else {
            0
        }));
        out.extend([0u8; 12]);
        debug_assert_eq!(out.len(), 128);
        out.extend(pixels);
        out
    }

    /// Reads a DDS file's header; returns it with the pixel data.
    ///
    /// # Errors
    /// Fails on files that are not DDS or use another pixel format.
    pub fn read(bytes: &[u8]) -> Result<(Self, &[u8]), DdsError> {
        if bytes.len() < 128 || &bytes[..4] != b"DDS " || u32_at(bytes, 4) != 124 {
            return Err(DdsError::NotDds);
        }
        let height = u32_at(bytes, 12);
        let width = u32_at(bytes, 16);
        let mip_levels = u32_at(bytes, 28).max(1);
        let pf_flags = u32_at(bytes, 80);
        let four_cc: [u8; 4] = bytes[84..88].try_into().expect("4 bytes");
        let bits = u32_at(bytes, 88);
        let masks = [
            u32_at(bytes, 92),
            u32_at(bytes, 96),
            u32_at(bytes, 100),
            u32_at(bytes, 104),
        ];
        let format = if pf_flags & DDPF_FOURCC != 0 {
            match &four_cc {
                b"DXT1" | b"DXT3" | b"DXT5" => PixelFormat::FourCc(four_cc),
                _ => return Err(DdsError::UnsupportedFormat),
            }
        } else if bits == 32 && masks == [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000] {
            PixelFormat::Bgra8
        } else if bits == 8 && pf_flags & DDPF_LUMINANCE != 0 {
            PixelFormat::L8
        } else {
            return Err(DdsError::UnsupportedFormat);
        };
        let cube = u32_at(bytes, 112) & 0x200 != 0;
        let top_level_size = u32_at(bytes, 20) as usize;
        Ok((
            Self {
                width,
                height,
                mip_levels,
                cube,
                format,
                top_level_size,
            },
            &bytes[128..],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_reads_back() {
        for format in [
            PixelFormat::FourCc(*b"DXT5"),
            PixelFormat::Bgra8,
            PixelFormat::L8,
        ] {
            let dds = Dds {
                width: 8,
                height: 4,
                mip_levels: 3,
                cube: false,
                format,
                top_level_size: 32,
            };
            let file = dds.write(&[7; 10]);
            let (back, pixels) = Dds::read(&file).unwrap();
            assert_eq!(
                (
                    back.width,
                    back.height,
                    back.mip_levels,
                    back.cube,
                    back.format
                ),
                (8, 4, 3, false, format)
            );
            assert_eq!(pixels, &[7; 10]);
        }
        assert_eq!(Dds::read(b"nope").unwrap_err(), DdsError::NotDds);
    }
}
