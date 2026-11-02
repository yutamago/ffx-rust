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

/// Pixel formats: the five the shipped textures use, and three more the
/// game's loader accepts (`RGBA8`, `BC4`, `BC5`).
///
/// The cluster names its format in the user fixup data (`"DXT5"`), which
/// the loader looks up in the engine's table of 33 texture formats
/// (`PTextureFormatDesc_registerAll_004c1050`, records of `0x28` bytes from
/// `0x00ca2fcc`, filled by static constructors). A record holds the bits
/// per pixel (`+0xc`), whether it is a 4x4 block format (`+0x10` bit 0)
/// and the `DXGI_FORMAT` the D3D11 texture is made with (`+0x18`, read by
/// `PTexture2DD3D11_CreateDeviceTexture_0058fc20`). There is no BC6H, BC7
/// or sRGB format in that table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Dxt1,
    Dxt3,
    Dxt5,
    /// 32-bit, stored B, G, R, A in memory.
    Argb8,
    /// 8-bit luminance.
    L8,
    /// 32-bit, stored R, G, B, A in memory. No shipped texture uses it.
    Rgba8,
    /// One 8-byte alpha-style block per 4x4 pixels (one channel). No
    /// shipped texture uses it.
    Bc4,
    /// Two BC4 blocks per 4x4 pixels (two channels). No shipped texture
    /// uses it.
    Bc5,
}

impl Format {
    /// Every format this crate reads and writes.
    pub const ALL: [Self; 8] = [
        Self::Dxt1,
        Self::Dxt3,
        Self::Dxt5,
        Self::Argb8,
        Self::L8,
        Self::Rgba8,
        Self::Bc4,
        Self::Bc5,
    ];

    /// The format by its name in the cluster.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// The name the cluster stores (the engine's format name).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Dxt1 => "DXT1",
            Self::Dxt3 => "DXT3",
            Self::Dxt5 => "DXT5",
            Self::Argb8 => "ARGB8",
            Self::L8 => "L8",
            Self::Rgba8 => "RGBA8",
            Self::Bc4 => "BC4",
            Self::Bc5 => "BC5",
        }
    }

    /// The `DXGI_FORMAT` the game creates the texture with (the format
    /// record's `+0x18`).
    #[must_use]
    pub fn dxgi(self) -> u32 {
        match self {
            Self::Dxt1 => 71,  // BC1_UNORM
            Self::Dxt3 => 74,  // BC2_UNORM
            Self::Dxt5 => 77,  // BC3_UNORM
            Self::Argb8 => 87, // B8G8R8A8_UNORM
            Self::L8 => 61,    // R8_UNORM
            Self::Rgba8 => 28, // R8G8B8A8_UNORM
            Self::Bc4 => 80,   // BC4_UNORM
            Self::Bc5 => 83,   // BC5_UNORM
        }
    }

    /// Bits per pixel (the format record's `+0xc`).
    #[must_use]
    pub fn bits(self) -> u32 {
        match self {
            Self::Dxt1 | Self::Bc4 => 4,
            Self::Dxt3 | Self::Dxt5 | Self::Bc5 | Self::L8 => 8,
            Self::Argb8 | Self::Rgba8 => 32,
        }
    }

    /// Whether the format stores 4x4 blocks (the format record's `+0x10`
    /// bit 0).
    #[must_use]
    pub fn is_block(self) -> bool {
        matches!(
            self,
            Self::Dxt1 | Self::Dxt3 | Self::Dxt5 | Self::Bc4 | Self::Bc5
        )
    }

    /// Bytes of one mip level.
    #[must_use]
    pub fn level_size(self, width: u32, height: u32) -> usize {
        if self.is_block() {
            let blocks = |n: u32| n.div_ceil(4).max(1) as usize;
            blocks(width) * blocks(height) * 2 * self.bits() as usize
        } else {
            width as usize * height as usize * self.bits() as usize / 8
        }
    }

    pub(crate) fn dds(self) -> PixelFormat {
        match self {
            Self::Dxt1 => PixelFormat::FourCc(*b"DXT1"),
            Self::Dxt3 => PixelFormat::FourCc(*b"DXT3"),
            Self::Dxt5 => PixelFormat::FourCc(*b"DXT5"),
            Self::Argb8 => PixelFormat::Bgra8,
            Self::L8 => PixelFormat::L8,
            Self::Rgba8 => PixelFormat::Rgba8,
            Self::Bc4 => PixelFormat::FourCc(*b"ATI1"),
            Self::Bc5 => PixelFormat::FourCc(*b"ATI2"),
        }
    }

    pub(crate) fn from_dds(format: PixelFormat) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.dds() == format)
    }
}

/// 2D texture or cube map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Texture2D,
    CubeMap,
}

impl Kind {
    /// 1, or 6 for a cube map.
    #[must_use]
    pub fn faces(self) -> usize {
        match self {
            Self::Texture2D => 1,
            Self::CubeMap => 6,
        }
    }

    /// Where the object starts, before the end of the instance data
    /// ([`Header::instance_data_end`]).
    fn object_back(self) -> usize {
        match self {
            Self::Texture2D => 0x2c,
            Self::CubeMap => 0x20,
        }
    }
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

/// Flips every level of every face (an involution: upright to stored and
/// back).
fn flip_chain(format: Format, kind: Kind, size: (u32, u32), levels: u32, pixels: &[u8]) -> Vec<u8> {
    let mut out = pixels.to_vec();
    let mut at = 0;
    for _ in 0..kind.faces() {
        let (mut w, mut h) = size;
        for _ in 0..levels {
            let len = format.level_size(w, h);
            flip_level(format, w, h, &mut out[at..at + len]);
            at += len;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
    }
    out
}

/// Bytes of a mip chain of one face.
#[must_use]
pub fn chain_size(format: Format, mut width: u32, mut height: u32, levels: u32) -> usize {
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
        let format = Format::from_name(&format_name)
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
        // A full chain at the largest side has 15 levels; past 32 the size
        // math below is meaningless.
        if max_mip >= 32 {
            return Err(Error::Malformed("mip count over 32"));
        }
        let mip_levels = max_mip + 1;
        let data = header.buffers();
        let expected = chain_size(format, width, height, mip_levels) * kind.faces();
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
        flip_chain(
            self.format,
            self.kind,
            (self.width, self.height),
            self.mip_levels,
            pixels,
        )
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

/// The largest side the game's device takes: `D3D11_REQ_TEXTURE2D_U_OR_V_DIMENSION`
/// at feature level `11_0` (`PRendererD3D11_createDeviceAndSwapChain_00595800`
/// asks for `11_0`, `10_1` or `10_0`; at `10_x` the limit is 8192).
pub const MAX_SIDE: u32 = 16384;

/// Mip levels of a full chain down to 1x1 (`GetMipCountForTextureSize`,
/// `0x004c2430`, plus one).
#[must_use]
pub fn full_mip_levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// `PTextureCommonBase.m_textureFlags` bit 1: the maximum mip level follows
/// the size (`PTexture2DBase::setDimensions`, `0x004c40c0`, sets
/// `m_maxMipLevel` from `GetMipCountForTextureSize` then). The shipped
/// textures with one level have it, the others not.
const FLAG_AUTO_MAX_MIP: u32 = 0x2;

impl Texture {
    /// Checks a new size and mip count before any size math on them
    /// ([`Texture::rebuilt`], [`Texture::with_any_dds`] and
    /// [`Texture::with_rgba`] call it first).
    ///
    /// # Errors
    /// Fails on sizes the game cannot create (0, over [`MAX_SIDE`], a cube
    /// map that is not square), more mip levels than the size has, and cube
    /// maps with one level: no shipped cube map has one, so the one-level
    /// flag convention (`m_textureFlags` bit 1) is untested for them.
    pub fn check_shape(&self, width: u32, height: u32, mip_levels: u32) -> Result<()> {
        let bad = |what: String| Err(Error::Mismatch(what));
        if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
            return bad(format!(
                "size {width}x{height}: sides must be 1 to {MAX_SIDE}"
            ));
        }
        if self.kind == Kind::CubeMap && width != height {
            return bad(format!("cube map {width}x{height} is not square"));
        }
        let full = full_mip_levels(width, height);
        if mip_levels == 0 || mip_levels > full {
            return bad(format!(
                "{mip_levels} mip levels: {width}x{height} has 1 to {full}"
            ));
        }
        if self.kind == Kind::CubeMap && mip_levels == 1 && full > 1 {
            return Err(Error::Unsupported(
                "a cube map with one level: no shipped cube map has one, give it 2 or more".into(),
            ));
        }
        Ok(())
    }

    /// A copy of this cluster holding another image: any format of
    /// [`Format`], size and mip count. `pixels` is upright (top row first,
    /// as DDS stores it): every level, largest first, per face for cube maps.
    ///
    /// The cluster's own fields are rewritten; nothing else in it refers to
    /// the size or format (`docs/formats/phyre.md`, "Changing size, format
    /// and mip count"):
    /// - `m_mipmapCount` (levels − 1), `m_maxMipLevel`, `m_textureFlags`
    ///   bit 1, `m_width`, `m_height` in the `PTexture2D`/`PTextureCubeMap`;
    /// - the format name in the user fixup data, with its record's length,
    ///   the later records' offsets and header word 13;
    /// - header word 20, the texture buffer size: the loader allocates twice
    ///   that for the levels it reads;
    /// - the pixel data at the end.
    ///
    /// With this texture's own format, size and mip count the result is
    /// [`Texture::with_dds`]'s.
    ///
    /// # Errors
    /// Fails on what [`Texture::check_shape`] refuses and on pixel data of
    /// another size.
    pub fn rebuilt(
        &self,
        format: Format,
        width: u32,
        height: u32,
        mip_levels: u32,
        pixels: &[u8],
    ) -> Result<Self> {
        self.check_shape(width, height, mip_levels)?;
        let full = full_mip_levels(width, height);
        let bad = |what: String| Err(Error::Mismatch(what));
        let faces = self.kind.faces();
        let chain = chain_size(format, width, height, mip_levels) * faces;
        if pixels.len() != chain {
            return bad(format!(
                "{} bytes of pixel data, expected {chain}",
                pixels.len()
            ));
        }
        let same = (format, width, height, mip_levels)
            == (self.format, self.width, self.height, self.mip_levels);
        let header = Header::parse(&self.bytes)?;
        let mut bytes = self.bytes[..self.data].to_vec();
        if !same {
            let obj = header.instance_data_end() - self.kind.object_back();
            let flags = u32::from_le_bytes(bytes[obj + 0x14..obj + 0x18].try_into().expect("4"));
            let (flags, max_mip) = if mip_levels == 1 {
                (flags | FLAG_AUTO_MAX_MIP, full - 1)
            } else {
                (flags & !FLAG_AUTO_MAX_MIP, mip_levels - 1)
            };
            put_u32(&mut bytes, obj + 0xc, mip_levels - 1);
            put_u32(&mut bytes, obj + 0x10, max_mip);
            put_u32(&mut bytes, obj + 0x14, flags);
            put_u32(&mut bytes, obj + 0x1c, width);
            if self.kind == Kind::Texture2D {
                put_u32(&mut bytes, obj + 0x20, height);
            }
            // The loader's buffer is twice this; the original sets the top
            // level (per face), enough for every chain but a few tiny
            // block textures.
            let top = format.level_size(width, height) * faces;
            let buffer = u32::try_from(top.max(chain.div_ceil(2)))
                .map_err(|_| Error::Unsupported("texture over 4 GiB".into()))?;
            put_u32(&mut bytes, 0x50, buffer);
            if format != self.format {
                bytes = rename_format(&bytes, &header, self.format, format)?;
            }
        }
        bytes.extend(flip_chain(
            format,
            self.kind,
            (width, height),
            mip_levels,
            pixels,
        ));
        Self::parse(bytes)
    }

    /// Like [`Texture::with_dds`], but the DDS may have any format of
    /// [`Format`], size and mip count ([`Texture::rebuilt`]). It must still
    /// be a cube map when this texture is one.
    ///
    /// # Errors
    /// Fails on DDS files that cannot be read and on what
    /// [`Texture::rebuilt`] refuses.
    pub fn with_any_dds(&self, dds: &[u8]) -> Result<Self> {
        let (info, pixels) = Dds::read(dds)?;
        if info.cube != (self.kind == Kind::CubeMap) {
            return Err(Error::Mismatch("cube map flag differs".into()));
        }
        let format = Format::from_dds(info.format)
            .ok_or_else(|| Error::Mismatch(format!("DDS format {:?}", info.format)))?;
        self.check_shape(info.width, info.height, info.mip_levels)?;
        let chain =
            chain_size(format, info.width, info.height, info.mip_levels) * self.kind.faces();
        // Trailing bytes some tools leave are dropped.
        let pixels = pixels.get(..chain).unwrap_or(pixels);
        self.rebuilt(format, info.width, info.height, info.mip_levels, pixels)
    }

    /// A copy of this cluster holding an upright RGBA8 image (`width` x
    /// `height`, top row first; six faces one after the other for a cube
    /// map), stored as `format` with `mip_levels` levels made by a 2x2 box
    /// filter.
    ///
    /// `format` must be uncompressed (`ARGB8`, `RGBA8` or `L8`, which keeps
    /// the red channel); for the block formats compress with a DDS tool
    /// and use [`Texture::with_any_dds`].
    ///
    /// # Errors
    /// Fails on block formats, `rgba` of another size and what
    /// [`Texture::rebuilt`] refuses.
    pub fn with_rgba(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        format: Format,
        mip_levels: u32,
    ) -> Result<Self> {
        if format.is_block() {
            return Err(Error::Unsupported(format!(
                "{} from RGBA: compress to DDS first",
                format.name()
            )));
        }
        self.check_shape(width, height, mip_levels)?;
        let face = width as usize * height as usize * 4;
        if rgba.len() != face * self.kind.faces() {
            return Err(Error::Mismatch(format!(
                "{} bytes of RGBA, expected {}",
                rgba.len(),
                face * self.kind.faces()
            )));
        }
        let mut pixels = Vec::new();
        for face in rgba.chunks_exact(face) {
            let (mut w, mut h) = (width, height);
            let mut level = face.to_vec();
            for k in 0..mip_levels {
                if k > 0 {
                    (level, w, h) = halve(&level, w, h);
                }
                for p in level.as_chunks::<4>().0 {
                    match format {
                        Format::Argb8 => pixels.extend([p[2], p[1], p[0], p[3]]),
                        Format::Rgba8 => pixels.extend_from_slice(p),
                        _ => pixels.push(p[0]),
                    }
                }
            }
        }
        self.rebuilt(format, width, height, mip_levels, &pixels)
    }
}

/// The next mip level of an RGBA8 image: the mean of each 2x2 (or 2x1)
/// block.
fn halve(src: &[u8], w: u32, h: u32) -> (Vec<u8>, u32, u32) {
    let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
    let (w, h) = (w as usize, h as usize);
    let mut out = Vec::with_capacity(nw as usize * nh as usize * 4);
    for y in 0..nh as usize {
        for x in 0..nw as usize {
            for c in 0..4 {
                let mut sum = 0u32;
                let mut n = 0u32;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = (2 * x + dx, 2 * y + dy);
                    if sx < w && sy < h {
                        sum += u32::from(src[(sy * w + sx) * 4 + c]);
                        n += 1;
                    }
                }
                out.push(u8::try_from((sum + n / 2) / n).expect("a mean of bytes"));
            }
        }
    }
    (out, nw, nh)
}

/// Swaps the format name in the user fixup data (the string the loader
/// resolves to the format record), fixing its record's length, the later
/// records' offsets and header word 13. Everything after the user fixup
/// data moves with it; the fixup tables refer to user fixups by index, so
/// they stay valid.
fn rename_format(bytes: &[u8], header: &Header, from: Format, to: Format) -> Result<Vec<u8>> {
    let data_at = header.user_fixup_data();
    let size = header.user_fixup_data_size as usize;
    let records_at = data_at + size;
    let count = header.user_fixup_count as usize;
    let word = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().expect("4 bytes")) as usize;
    let old = format!("{}\0", from.name());
    let record = (0..count)
        .map(|i| records_at + 12 * i)
        .find(|&r| {
            let (len, off) = (word(r + 4), word(r + 8));
            len == old.len()
                && bytes.get(data_at + off..data_at + off + len) == Some(old.as_bytes())
        })
        .ok_or(Error::Malformed("no user fixup names the texture format"))?;
    let (len, off) = (word(record + 4), word(record + 8));
    let new = format!("{}\0", to.name());
    let mut out = Vec::with_capacity(bytes.len() + new.len());
    out.extend_from_slice(&bytes[..data_at + off]);
    out.extend_from_slice(new.as_bytes());
    out.extend_from_slice(&bytes[data_at + off + len..]);
    // `v` (at least `len`) moved by the change in length.
    let grow = |v: usize| u32::try_from(v + new.len() - len).expect("a small cluster");
    let records_at = records_at + new.len() - len;
    let record = record + new.len() - len;
    for i in 0..count {
        let r = records_at + 12 * i;
        let o = word_at(&out, r + 8);
        if o > off {
            put_u32(&mut out, r + 8, grow(o));
        }
    }
    put_u32(
        &mut out,
        record + 4,
        u32::try_from(new.len()).expect("short name"),
    );
    put_u32(&mut out, 0x34, grow(size));
    Ok(out)
}

fn word_at(b: &[u8], o: usize) -> usize {
    u32::from_le_bytes(b[o..o + 4].try_into().expect("4 bytes")) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up 2D texture cluster with the shipped layout's parts that
    /// matter here: no namespace or lists, a `PTexture2D` ending the
    /// instance data, the two user fixups (`PTexture2D`, the format) and
    /// the pixels.
    fn synthetic(format: Format, width: u32, height: u32, levels: u32) -> Texture {
        let mut object = vec![0u8; 0x2c];
        put_u32(&mut object, 0xc, levels - 1);
        put_u32(&mut object, 0x10, levels - 1);
        put_u32(&mut object, 0x1c, width);
        put_u32(&mut object, 0x20, height);
        let class = b"PTexture2D\0";
        let name = format!("{}\0", format.name());
        let strings = [&class[..], name.as_bytes()].concat();
        let mut header = [0u32; 21];
        header[0] = crate::MAGIC;
        header[1] = 0x54;
        header[3] = u32::from_le_bytes(*b"11XD");
        header[12] = 2;
        header[13] = u32::try_from(strings.len()).unwrap();
        header[14] = 0x2c;
        header[20] = u32::try_from(format.level_size(width, height)).unwrap();
        let mut bytes: Vec<u8> = header.iter().flat_map(|w| w.to_le_bytes()).collect();
        bytes.extend(object);
        bytes.extend(&strings);
        for (kind, len, off) in [(8u32, class.len(), 0), (2, name.len(), class.len())] {
            for v in [
                kind,
                u32::try_from(len).unwrap(),
                u32::try_from(off).unwrap(),
            ] {
                bytes.extend(v.to_le_bytes());
            }
        }
        let chain = chain_size(format, width, height, levels);
        bytes.extend((0..chain).map(|i| u8::try_from(i % 251).unwrap()));
        Texture::parse(bytes).unwrap()
    }

    fn rgba(w: u32, h: u32) -> Vec<u8> {
        (0..w * h * 4)
            .map(|i| u8::try_from(i * 7 % 256).unwrap())
            .collect()
    }

    #[test]
    fn rebuilt_with_the_same_image_is_the_same_file() {
        let t = synthetic(Format::Dxt5, 64, 32, 7);
        let same = t
            .rebuilt(t.format, 64, 32, 7, &t.flipped(t.pixels()))
            .unwrap();
        assert_eq!(same.bytes(), t.bytes());
        assert_eq!(t.with_any_dds(&t.to_dds()).unwrap().bytes(), t.bytes());
    }

    #[test]
    fn resized_textures_parse_back() {
        let t = synthetic(Format::Dxt1, 64, 32, 7);
        for format in Format::ALL {
            for (w, h, levels) in [(256, 128, 9), (256, 128, 1), (100, 30, 3), (4096, 16, 13)] {
                let pixels = vec![0x5a; chain_size(format, w, h, levels)];
                let r = t.rebuilt(format, w, h, levels, &pixels).unwrap();
                assert_eq!(
                    (r.format, r.width, r.height, r.mip_levels),
                    (format, w, h, levels),
                    "{format:?} {w}x{h}"
                );
                assert_eq!(r.pixels().len(), pixels.len());
                // And back: the original file again.
                let back = r
                    .rebuilt(t.format, 64, 32, 7, &t.flipped(t.pixels()))
                    .unwrap();
                assert_eq!(back.bytes(), t.bytes(), "{format:?} {w}x{h} back");
            }
        }
    }

    #[test]
    fn single_level_textures_follow_the_shipped_convention() {
        let t = synthetic(Format::Dxt5, 64, 64, 7);
        let one = t
            .rebuilt(
                Format::Dxt5,
                512,
                256,
                1,
                &vec![0; chain_size(Format::Dxt5, 512, 256, 1)],
            )
            .unwrap();
        let end = Header::parse(one.bytes()).unwrap().instance_data_end();
        let obj = end - 0x2c;
        let w = |o| word_at(one.bytes(), obj + o);
        assert_eq!(
            (w(0xc), w(0x10), w(0x14)),
            (0, 9, FLAG_AUTO_MAX_MIP as usize)
        );
        assert_eq!(word_at(one.bytes(), 0x50), 512 * 256);
    }

    #[test]
    fn rgba_images_get_mips_and_their_format() {
        let t = synthetic(Format::Dxt1, 8, 8, 4);
        let image = rgba(32, 16);
        let r = t.with_rgba(32, 16, &image, Format::Argb8, 6).unwrap();
        assert_eq!(
            (r.format, r.width, r.height, r.mip_levels),
            (Format::Argb8, 32, 16, 6)
        );
        assert_eq!(r.to_rgba(), image);
        let r = t.with_rgba(32, 16, &image, Format::Rgba8, 1).unwrap();
        assert_eq!(r.to_rgba(), image);
        assert!(t.with_rgba(32, 16, &image, Format::Dxt5, 1).is_err());
    }

    #[test]
    fn impossible_textures_are_refused() {
        let t = synthetic(Format::Dxt1, 8, 8, 4);
        let px = |w, h, l| vec![0; chain_size(Format::Dxt1, w, h, l)];
        assert!(t.rebuilt(Format::Dxt1, 0, 8, 1, &[]).is_err());
        assert!(
            t.rebuilt(Format::Dxt1, 32768, 8, 1, &px(32768, 8, 1))
                .is_err()
        );
        assert!(t.rebuilt(Format::Dxt1, 16, 16, 6, &px(16, 16, 6)).is_err());
        assert!(t.rebuilt(Format::Dxt1, 16, 16, 5, &px(16, 16, 4)).is_err());
        assert!(t.rebuilt(Format::Dxt1, 16, 16, 5, &px(16, 16, 5)).is_ok());
    }

    /// Absurd sizes and mip counts fail before any size math or mip loop
    /// on them (no overflow in debug builds, no huge allocation).
    #[test]
    fn absurd_inputs_fail_before_size_math() {
        let t = synthetic(Format::Dxt1, 8, 8, 4);
        let mut dds = t.to_dds();
        for (w, h, levels) in [
            (u32::MAX, u32::MAX, 1),
            (8, 8, u32::MAX),
            (u32::MAX, 1, u32::MAX),
        ] {
            dds[12..16].copy_from_slice(&h.to_le_bytes());
            dds[16..20].copy_from_slice(&w.to_le_bytes());
            dds[28..32].copy_from_slice(&levels.to_le_bytes());
            assert!(t.with_any_dds(&dds).is_err(), "{w}x{h} {levels}");
            assert!(
                t.with_rgba(w, h, &[], Format::Argb8, levels).is_err(),
                "{w}x{h} {levels}"
            );
        }
        let mut bad = t.bytes().to_vec();
        let end = Header::parse(&bad).unwrap().instance_data_end();
        put_u32(&mut bad, end - 0x20, u32::MAX);
        assert!(Texture::parse(bad).is_err());
    }

    #[test]
    fn one_level_cube_maps_are_refused() {
        let cube = Texture {
            kind: Kind::CubeMap,
            ..synthetic(Format::Dxt1, 8, 8, 4)
        };
        assert!(matches!(
            cube.check_shape(64, 64, 1),
            Err(Error::Unsupported(_))
        ));
        assert!(cube.check_shape(64, 64, 2).is_ok());
        assert!(cube.check_shape(1, 1, 1).is_ok());
        assert!(cube.check_shape(64, 32, 7).is_err());
    }

    #[test]
    fn formats_match_the_engine_table() {
        for f in Format::ALL {
            assert_eq!(Format::from_name(f.name()), Some(f));
            assert_eq!(Format::from_dds(f.dds()), Some(f));
        }
        assert_eq!(Format::Bc5.level_size(8, 8), 64);
        assert_eq!(Format::Bc4.level_size(2, 2), 8);
        assert_eq!(Format::Rgba8.level_size(3, 3), 36);
    }
}
