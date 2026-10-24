//! Object descriptions, field by field.
//!
//! The vocabulary is Direct3D 11's: formats are DXGI formats, blend
//! factors, comparison functions, filters and address modes keep their
//! `D3D11_*` numbers. The game's data and shaders are written against
//! them, so a later backend (Vulkan, wgpu) translates from these numbers
//! the way `ffx-render`'s `formats.rs` does today.
//!
//! The state descriptions read and write the exact bytes of the D3D11
//! structures (`from_d3d11` / `to_d3d11`), which is what the graphics
//! layer records (`capture.ffxgfx` keeps them raw). Padding bytes are not
//! fields: they read as nothing and write as zero.

/// `D3D11_BUFFER_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BufferDesc {
    pub size: u32,
    /// `D3D11_USAGE`: 0 default, 1 immutable, 2 dynamic, 3 staging.
    pub usage: u32,
    /// `D3D11_BIND_*` flags.
    pub bind: u32,
    /// `D3D11_CPU_ACCESS_*` flags.
    pub cpu_access: u32,
    pub misc: u32,
    pub stride: u32,
}

pub const USAGE_DEFAULT: u32 = 0;
pub const USAGE_IMMUTABLE: u32 = 1;
pub const USAGE_DYNAMIC: u32 = 2;
pub const USAGE_STAGING: u32 = 3;
pub const BIND_VERTEX: u32 = 0x1;
pub const BIND_INDEX: u32 = 0x2;
pub const BIND_CONSTANT: u32 = 0x4;
pub const BIND_SHADER_RESOURCE: u32 = 0x8;
pub const BIND_RENDER_TARGET: u32 = 0x20;
pub const BIND_DEPTH_STENCIL: u32 = 0x40;
pub const CPU_WRITE: u32 = 0x1_0000;
pub const CPU_READ: u32 = 0x2_0000;

impl BufferDesc {
    /// From the six words the graphics layer records.
    #[must_use]
    pub fn from_words(w: [u32; 6]) -> Self {
        Self {
            size: w[0],
            usage: w[1],
            bind: w[2],
            cpu_access: w[3],
            misc: w[4],
            stride: w[5],
        }
    }

    #[must_use]
    pub fn words(&self) -> [u32; 6] {
        [
            self.size,
            self.usage,
            self.bind,
            self.cpu_access,
            self.misc,
            self.stride,
        ]
    }
}

/// `D3D11_TEXTURE2D_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureDesc {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub array_size: u32,
    /// DXGI format.
    pub format: u32,
    pub samples: u32,
    pub quality: u32,
    pub usage: u32,
    pub bind: u32,
    pub cpu_access: u32,
    pub misc: u32,
}

impl TextureDesc {
    /// From the eleven words the graphics layer records.
    #[must_use]
    pub fn from_words(w: [u32; 11]) -> Self {
        Self {
            width: w[0],
            height: w[1],
            mips: w[2],
            array_size: w[3],
            format: w[4],
            samples: w[5],
            quality: w[6],
            usage: w[7],
            bind: w[8],
            cpu_access: w[9],
            misc: w[10],
        }
    }

    #[must_use]
    pub fn words(&self) -> [u32; 11] {
        [
            self.width,
            self.height,
            self.mips,
            self.array_size,
            self.format,
            self.samples,
            self.quality,
            self.usage,
            self.bind,
            self.cpu_access,
            self.misc,
        ]
    }
}

/// What a view is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ViewKind {
    ShaderResource,
    RenderTarget,
    DepthStencil,
}

/// A view description: format, dimension (`D3D11_*_DIMENSION` of the
/// view's kind) and the dimension's fields (the D3D11 union: e.g. most
/// detailed mip and mip count for a `TEXTURE2D` shader resource view).
/// A depth-stencil view's first field is its flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ViewDesc {
    pub format: u32,
    pub dimension: u32,
    pub fields: [u32; 4],
}

impl ViewDesc {
    /// From the six words the graphics layer records (render target views
    /// have five meaningful ones).
    #[must_use]
    pub fn from_words(w: [u32; 6]) -> Self {
        Self {
            format: w[0],
            dimension: w[1],
            fields: [w[2], w[3], w[4], w[5]],
        }
    }

    #[must_use]
    pub fn words(&self) -> [u32; 6] {
        [
            self.format,
            self.dimension,
            self.fields[0],
            self.fields[1],
            self.fields[2],
            self.fields[3],
        ]
    }
}

/// Shader stages the scene uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stage {
    Vertex,
    Pixel,
    Geometry,
}

/// `D3D11_INPUT_ELEMENT_DESC`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputElement {
    pub semantic: String,
    pub index: u32,
    /// DXGI format.
    pub format: u32,
    pub slot: u32,
    pub offset: u32,
    pub per_instance: bool,
    pub step: u32,
}

/// One render target's part of `D3D11_BLEND_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TargetBlend {
    pub enable: bool,
    /// `D3D11_BLEND`.
    pub src: u32,
    pub dst: u32,
    /// `D3D11_BLEND_OP`.
    pub op: u32,
    pub src_alpha: u32,
    pub dst_alpha: u32,
    pub op_alpha: u32,
    /// `D3D11_COLOR_WRITE_ENABLE_*` bits.
    pub write_mask: u8,
}

/// `D3D11_BLEND_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlendDesc {
    pub alpha_to_coverage: bool,
    pub independent: bool,
    pub targets: [TargetBlend; 8],
}

/// One face's part of `D3D11_DEPTH_STENCIL_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StencilFace {
    /// `D3D11_STENCIL_OP`.
    pub fail: u32,
    pub depth_fail: u32,
    pub pass: u32,
    /// `D3D11_COMPARISON_FUNC`.
    pub func: u32,
}

/// `D3D11_DEPTH_STENCIL_DESC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DepthStencilDesc {
    pub depth_enable: bool,
    /// `D3D11_DEPTH_WRITE_MASK`: 0 none, 1 all.
    pub depth_write: u32,
    pub depth_func: u32,
    pub stencil_enable: bool,
    pub stencil_read_mask: u8,
    pub stencil_write_mask: u8,
    pub front: StencilFace,
    pub back: StencilFace,
}

/// `D3D11_RASTERIZER_DESC`. Floats are kept as their bits, so that
/// descriptions compare and hash exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[allow(clippy::struct_excessive_bools)] // the D3D11 structure's
pub struct RasterDesc {
    /// `D3D11_FILL_MODE`: 2 wireframe, 3 solid.
    pub fill: u32,
    /// `D3D11_CULL_MODE`: 1 none, 2 front, 3 back.
    pub cull: u32,
    pub front_ccw: bool,
    pub depth_bias: i32,
    pub depth_bias_clamp: u32,
    pub slope_scaled_depth_bias: u32,
    pub depth_clip: bool,
    pub scissor: bool,
    pub multisample: bool,
    pub antialiased_line: bool,
}

/// `D3D11_SAMPLER_DESC`. Floats are kept as their bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SamplerDesc {
    /// `D3D11_FILTER`.
    pub filter: u32,
    /// `D3D11_TEXTURE_ADDRESS_MODE` for u, v, w.
    pub address: [u32; 3],
    pub mip_lod_bias: u32,
    pub max_anisotropy: u32,
    pub comparison: u32,
    pub border: [u32; 4],
    pub min_lod: u32,
    pub max_lod: u32,
}

/// A description whose D3D11 bytes were too short.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDesc {
    pub want: usize,
    pub got: usize,
}

impl std::fmt::Display for ShortDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "description of {} bytes, {} expected",
            self.got, self.want
        )
    }
}

impl std::error::Error for ShortDesc {}

fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn need(b: &[u8], want: usize) -> Result<(), ShortDesc> {
    if b.len() < want {
        Err(ShortDesc { want, got: b.len() })
    } else {
        Ok(())
    }
}

fn put(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

impl BlendDesc {
    /// The size of `D3D11_BLEND_DESC`.
    pub const SIZE: usize = 8 + 8 * 32;

    /// # Errors
    /// When `b` is shorter than the structure.
    pub fn from_d3d11(b: &[u8]) -> Result<Self, ShortDesc> {
        need(b, Self::SIZE)?;
        let mut targets = [TargetBlend::default(); 8];
        for (i, t) in targets.iter_mut().enumerate() {
            let at = 8 + 32 * i;
            *t = TargetBlend {
                enable: word(b, at) != 0,
                src: word(b, at + 4),
                dst: word(b, at + 8),
                op: word(b, at + 12),
                src_alpha: word(b, at + 16),
                dst_alpha: word(b, at + 20),
                op_alpha: word(b, at + 24),
                write_mask: b[at + 28],
            };
        }
        Ok(Self {
            alpha_to_coverage: word(b, 0) != 0,
            independent: word(b, 4) != 0,
            targets,
        })
    }

    #[must_use]
    pub fn to_d3d11(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        put(&mut out, u32::from(self.alpha_to_coverage));
        put(&mut out, u32::from(self.independent));
        for t in &self.targets {
            put(&mut out, u32::from(t.enable));
            for v in [t.src, t.dst, t.op, t.src_alpha, t.dst_alpha, t.op_alpha] {
                put(&mut out, v);
            }
            out.extend_from_slice(&[t.write_mask, 0, 0, 0]);
        }
        out
    }
}

impl DepthStencilDesc {
    /// The size of `D3D11_DEPTH_STENCIL_DESC`.
    pub const SIZE: usize = 52;

    /// # Errors
    /// When `b` is shorter than the structure.
    pub fn from_d3d11(b: &[u8]) -> Result<Self, ShortDesc> {
        need(b, Self::SIZE)?;
        let face = |at: usize| StencilFace {
            fail: word(b, at),
            depth_fail: word(b, at + 4),
            pass: word(b, at + 8),
            func: word(b, at + 12),
        };
        Ok(Self {
            depth_enable: word(b, 0) != 0,
            depth_write: word(b, 4),
            depth_func: word(b, 8),
            stencil_enable: word(b, 12) != 0,
            stencil_read_mask: b[16],
            stencil_write_mask: b[17],
            front: face(20),
            back: face(36),
        })
    }

    #[must_use]
    pub fn to_d3d11(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        put(&mut out, u32::from(self.depth_enable));
        put(&mut out, self.depth_write);
        put(&mut out, self.depth_func);
        put(&mut out, u32::from(self.stencil_enable));
        out.extend_from_slice(&[self.stencil_read_mask, self.stencil_write_mask, 0, 0]);
        for f in [self.front, self.back] {
            for v in [f.fail, f.depth_fail, f.pass, f.func] {
                put(&mut out, v);
            }
        }
        out
    }
}

impl RasterDesc {
    /// The size of `D3D11_RASTERIZER_DESC`.
    pub const SIZE: usize = 40;

    /// # Errors
    /// When `b` is shorter than the structure.
    pub fn from_d3d11(b: &[u8]) -> Result<Self, ShortDesc> {
        need(b, Self::SIZE)?;
        Ok(Self {
            fill: word(b, 0),
            cull: word(b, 4),
            front_ccw: word(b, 8) != 0,
            depth_bias: word(b, 12).cast_signed(),
            depth_bias_clamp: word(b, 16),
            slope_scaled_depth_bias: word(b, 20),
            depth_clip: word(b, 24) != 0,
            scissor: word(b, 28) != 0,
            multisample: word(b, 32) != 0,
            antialiased_line: word(b, 36) != 0,
        })
    }

    #[must_use]
    pub fn to_d3d11(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        for v in [
            self.fill,
            self.cull,
            u32::from(self.front_ccw),
            self.depth_bias.cast_unsigned(),
            self.depth_bias_clamp,
            self.slope_scaled_depth_bias,
            u32::from(self.depth_clip),
            u32::from(self.scissor),
            u32::from(self.multisample),
            u32::from(self.antialiased_line),
        ] {
            put(&mut out, v);
        }
        out
    }
}

impl SamplerDesc {
    /// The size of `D3D11_SAMPLER_DESC`.
    pub const SIZE: usize = 52;

    /// # Errors
    /// When `b` is shorter than the structure.
    pub fn from_d3d11(b: &[u8]) -> Result<Self, ShortDesc> {
        need(b, Self::SIZE)?;
        Ok(Self {
            filter: word(b, 0),
            address: [word(b, 4), word(b, 8), word(b, 12)],
            mip_lod_bias: word(b, 16),
            max_anisotropy: word(b, 20),
            comparison: word(b, 24),
            border: [word(b, 28), word(b, 32), word(b, 36), word(b, 40)],
            min_lod: word(b, 44),
            max_lod: word(b, 48),
        })
    }

    #[must_use]
    pub fn to_d3d11(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        put(&mut out, self.filter);
        for v in self.address {
            put(&mut out, v);
        }
        put(&mut out, self.mip_lod_bias);
        put(&mut out, self.max_anisotropy);
        put(&mut out, self.comparison);
        for v in self.border {
            put(&mut out, v);
        }
        put(&mut out, self.min_lod);
        put(&mut out, self.max_lod);
        out
    }
}

/// The bytes of a D3D11 state structure without its padding (what
/// `to_d3d11` gives back from `from_d3d11`), for comparing recorded raw
/// descriptions.
#[must_use]
pub fn without_padding(kind: StateKind, raw: &[u8]) -> Option<Vec<u8>> {
    match kind {
        StateKind::Blend => BlendDesc::from_d3d11(raw).ok().map(|d| d.to_d3d11()),
        StateKind::Depth => DepthStencilDesc::from_d3d11(raw).ok().map(|d| d.to_d3d11()),
        StateKind::Raster => RasterDesc::from_d3d11(raw).ok().map(|d| d.to_d3d11()),
        StateKind::Sampler => SamplerDesc::from_d3d11(raw).ok().map(|d| d.to_d3d11()),
    }
}

/// The four kinds of state object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
    Blend,
    Depth,
    Raster,
    Sampler,
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)] // test bytes
mod tests {
    use super::*;

    fn bytes(n: usize, seed: u8) -> Vec<u8> {
        (0..n)
            .map(|i| (i as u8).wrapping_mul(37).wrapping_add(seed))
            .collect()
    }

    #[test]
    fn states_round_trip_but_padding() {
        // Booleans as 0/1, padding zero: then the bytes come back as given.
        let mut b = bytes(BlendDesc::SIZE, 3);
        b[0..4].copy_from_slice(&1u32.to_le_bytes());
        b[4..8].copy_from_slice(&0u32.to_le_bytes());
        for i in 0..8 {
            let at = 8 + 32 * i;
            b[at..at + 4].copy_from_slice(&u32::from(i % 2 == 0).to_le_bytes());
            b[at + 29..at + 32].fill(0);
        }
        assert_eq!(BlendDesc::from_d3d11(&b).unwrap().to_d3d11(), b);

        let mut d = bytes(DepthStencilDesc::SIZE, 9);
        d[0..4].copy_from_slice(&1u32.to_le_bytes());
        d[12..16].copy_from_slice(&0u32.to_le_bytes());
        d[18..20].fill(0);
        assert_eq!(DepthStencilDesc::from_d3d11(&d).unwrap().to_d3d11(), d);

        let mut r = bytes(RasterDesc::SIZE, 1);
        for at in [8, 24, 28, 32, 36] {
            r[at..at + 4].copy_from_slice(&u32::from(at % 8 == 0).to_le_bytes());
        }
        assert_eq!(RasterDesc::from_d3d11(&r).unwrap().to_d3d11(), r);

        let s = bytes(SamplerDesc::SIZE, 5);
        assert_eq!(SamplerDesc::from_d3d11(&s).unwrap().to_d3d11(), s);
    }

    #[test]
    fn padding_is_not_a_field() {
        let mut b = vec![0u8; BlendDesc::SIZE];
        b[8 + 28] = 0xf;
        b[8 + 29] = 0xaa; // padding
        let d = BlendDesc::from_d3d11(&b).unwrap();
        assert_eq!(d.targets[0].write_mask, 0xf);
        assert_eq!(d.to_d3d11()[8 + 29], 0);
        assert!(BlendDesc::from_d3d11(&b[..10]).is_err());
    }

    #[test]
    fn words_round_trip() {
        let w = [1, 2, 3, 4, 5, 6];
        assert_eq!(BufferDesc::from_words(w).words(), w);
        assert_eq!(ViewDesc::from_words(w).words(), w);
        let t = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        assert_eq!(TextureDesc::from_words(t).words(), t);
    }
}
