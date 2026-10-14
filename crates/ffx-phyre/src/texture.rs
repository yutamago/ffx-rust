//! Textures: `PTexture2D` and `PTextureCubeMap` clusters.
//!
//! The user fixup data starts with the class name and the format name
//! (`"PTexture2D\0DXT5\0"`). The texture's fields sit at fixed distances from
//! the end of the instance data:
//!
//! | Class | Max mip level | Width | Height |
//! |---|---|---|---|
//! | `PTexture2D` | end − `0x20` | end − `0x10` | end − `0x0c` |
//! | `PTextureCubeMap` | end − `0x14` | end − `0x04` (square) | |
//!
//! The pixel data is the full mip chain (largest first), per face for cube
//! maps, and fills the file from [`crate::Header::buffers`] to the end.

use crate::dds::{Dds, PixelFormat};
use crate::flip::flip_level;
use crate::{Error, Header, Result};

/// Pixel formats used by the shipped textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dxt1,
    Dxt3,
    Dxt5,
    /// 32-bit, stored B, G, R, A in memory.
    Argb8,
    /// 8-bit luminance.
    L8,
}

impl Format {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "DXT1" => Self::Dxt1,
            "DXT3" => Self::Dxt3,
            "DXT5" => Self::Dxt5,
            "ARGB8" => Self::Argb8,
            "L8" => Self::L8,
            _ => return None,
        })
    }

    /// Bytes of one mip level.
    #[must_use]
    pub fn level_size(self, width: u32, height: u32) -> usize {
        let blocks = |n: u32| n.div_ceil(4).max(1) as usize;
        match self {
            Self::Dxt1 => blocks(width) * blocks(height) * 8,
            Self::Dxt3 | Self::Dxt5 => blocks(width) * blocks(height) * 16,
            Self::Argb8 => width as usize * height as usize * 4,
            Self::L8 => width as usize * height as usize,
        }
    }

    pub(crate) fn dds(self) -> PixelFormat {
        match self {
            Self::Dxt1 => PixelFormat::FourCc(*b"DXT1"),
            Self::Dxt3 => PixelFormat::FourCc(*b"DXT3"),
            Self::Dxt5 => PixelFormat::FourCc(*b"DXT5"),
            Self::Argb8 => PixelFormat::Bgra8,
            Self::L8 => PixelFormat::L8,
        }
    }
}

/// 2D texture or cube map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Texture2D,
    CubeMap,
}

/// A parsed texture cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texture {
    bytes: Vec<u8>,
    pub kind: Kind,
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    data: usize,
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
        .ok_or(Error::Malformed("field outside the file"))
}

fn chain_size(format: Format, mut width: u32, mut height: u32, levels: u32) -> usize {
    let mut total = 0;
    for _ in 0..levels {
        total += format.level_size(width, height);
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    total
}

impl Texture {
    /// Parses a `*.dds.phyre` file.
    ///
    /// # Errors
    /// Fails on other cluster kinds, unknown formats, and files whose pixel
    /// data does not have the size the metadata implies.
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let header = Header::parse(&bytes)?;
        let strings_at = header.user_fixup_data();
        let strings = bytes
            .get(strings_at..strings_at + header.user_fixup_data_size as usize)
            .ok_or(Error::Malformed("user fixup data outside the file"))?;
        let mut names = strings
            .split(|&b| b == 0)
            .map(|s| String::from_utf8_lossy(s).into_owned());
        let class = names.next().unwrap_or_default();
        let format_name = names.next().unwrap_or_default();
        let format = Format::parse(&format_name)
            .ok_or_else(|| Error::Unsupported(format!("format {format_name:?}")))?;
        let end = header.instance_data_end();
        let field = |back: usize| {
            end.checked_sub(back)
                .ok_or(Error::Malformed("instance data too short"))
                .and_then(|o| u32_at(&bytes, o))
        };
        let (kind, width, height, max_mip) = match class.as_str() {
            "PTexture2D" => (Kind::Texture2D, field(0x10)?, field(0x0c)?, field(0x20)?),
            "PTextureCubeMap" => {
                let size = field(0x04)?;
                (Kind::CubeMap, size, size, field(0x14)?)
            }
            other => return Err(Error::Unsupported(format!("class {other:?}"))),
        };
        let mip_levels = max_mip + 1;
        let data = header.buffers();
        let faces = if kind == Kind::CubeMap { 6 } else { 1 };
        let expected = chain_size(format, width, height, mip_levels) * faces;
        if data + expected != bytes.len() {
            return Err(Error::Malformed(
                "pixel data does not match the texture's size",
            ));
        }
        Ok(Self {
            bytes,
            kind,
            format,
            width,
            height,
            mip_levels,
            data,
        })
    }

    /// The cluster's bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The pixel data: every mip level, per face.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.bytes[self.data..]
    }

    /// The pixel data turned upright (top row first), as DDS stores it.
    fn flipped(&self, pixels: &[u8]) -> Vec<u8> {
        let mut out = pixels.to_vec();
        let faces = if self.kind == Kind::CubeMap { 6 } else { 1 };
        let mut at = 0;
        for _ in 0..faces {
            let (mut w, mut h) = (self.width, self.height);
            for _ in 0..self.mip_levels {
                let len = self.format.level_size(w, h);
                flip_level(self.format, w, h, &mut out[at..at + len]);
                at += len;
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }
        }
        out
    }

    /// Converts to a DDS file, turned upright: the game stores rows bottom
    /// first, DDS top first.
    #[must_use]
    pub fn to_dds(&self) -> Vec<u8> {
        self.dds_header().write(&self.flipped(self.pixels()))
    }

    /// Converts to a DDS file with the pixel rows as stored (upside down).
    #[must_use]
    pub fn to_dds_raw(&self) -> Vec<u8> {
        self.dds_header().write(self.pixels())
    }

    fn dds_header(&self) -> Dds {
        Dds {
            width: self.width,
            height: self.height,
            mip_levels: self.mip_levels,
            cube: self.kind == Kind::CubeMap,
            format: self.format.dds(),
            top_level_size: self.format.level_size(self.width, self.height),
        }
    }

    /// Replaces the pixel data with an upright DDS (as [`Texture::to_dds`]
    /// writes) of the same format, size, mip count and kind. Everything else
    /// in the cluster stays as it is.
    ///
    /// # Errors
    /// Fails on DDS files that cannot be read or do not match.
    pub fn with_dds(&self, dds: &[u8]) -> Result<Self> {
        let (info, pixels) = Dds::read(dds)?;
        let mismatch = |what: String| Err(Error::Mismatch(what));
        if info.format != self.format.dds() {
            return mismatch(format!(
                "format {:?}, expected {:?}",
                info.format, self.format
            ));
        }
        if (info.width, info.height) != (self.width, self.height) {
            return mismatch(format!(
                "size {}x{}, expected {}x{}",
                info.width, info.height, self.width, self.height
            ));
        }
        if info.mip_levels != self.mip_levels {
            return mismatch(format!(
                "{} mip levels, expected {}",
                info.mip_levels, self.mip_levels
            ));
        }
        if info.cube != (self.kind == Kind::CubeMap) {
            return mismatch("cube map flag differs".into());
        }
        if pixels.len() != self.pixels().len() {
            return mismatch(format!(
                "{} bytes of pixel data, expected {}",
                pixels.len(),
                self.pixels().len()
            ));
        }
        let mut bytes = self.bytes.clone();
        bytes[self.data..].copy_from_slice(&self.flipped(pixels));
        Self::parse(bytes)
    }
}
