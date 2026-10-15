//! Direct3D 11 formats and state descriptions in wgpu terms.

use wgpu::{
    AddressMode, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
    CompareFunction, DepthBiasState, Face as CullMode, FilterMode, FrontFace, MipmapFilterMode,
    PrimitiveTopology, SamplerBorderColor, StencilFaceState, StencilOperation, TextureFormat,
    VertexFormat,
};

/// The wgpu format of a DXGI texture or view format. Typeless formats
/// resolve to their usual typed form; `depth` picks depth formats for the
/// depth-stencil ones.
#[must_use]
pub fn texture_format(dxgi: u32, depth: bool) -> Option<TextureFormat> {
    use TextureFormat as F;
    Some(match dxgi {
        2 => F::Rgba32Float,
        3 => F::Rgba32Uint,
        4 => F::Rgba32Sint,
        10 => F::Rgba16Float,
        11 => F::Rgba16Unorm,
        12 => F::Rgba16Uint,
        13 => F::Rgba16Snorm,
        14 => F::Rgba16Sint,
        16 => F::Rg32Float,
        17 => F::Rg32Uint,
        18 => F::Rg32Sint,
        // D32S8X24, and D24S8 as 32-bit float depth: the renderer reads
        // depth back, which wgpu only allows for float depth.
        19 | 20 | 44..=47 => F::Depth32FloatStencil8,
        24 | 23 => F::Rgb10a2Unorm,
        25 => F::Rgb10a2Uint,
        26 => F::Rg11b10Ufloat,
        27 | 28 => F::Rgba8Unorm,
        29 => F::Rgba8UnormSrgb,
        30 => F::Rgba8Uint,
        31 => F::Rgba8Snorm,
        32 => F::Rgba8Sint,
        34 => F::Rg16Float,
        35 | 33 => F::Rg16Unorm,
        36 => F::Rg16Uint,
        37 => F::Rg16Snorm,
        38 => F::Rg16Sint,
        39 if depth => F::Depth32Float,
        39 | 41 => F::R32Float,
        40 => F::Depth32Float,
        42 => F::R32Uint,
        43 => F::R32Sint,
        49 | 48 => F::Rg8Unorm,
        50 => F::Rg8Uint,
        51 => F::Rg8Snorm,
        52 => F::Rg8Sint,
        54 if depth => F::Depth16Unorm,
        53 | 54 => F::R16Float,
        55 => F::Depth16Unorm,
        56 => F::R16Unorm,
        57 => F::R16Uint,
        58 => F::R16Snorm,
        59 => F::R16Sint,
        60 | 61 | 65 => F::R8Unorm,
        62 => F::R8Uint,
        63 => F::R8Snorm,
        64 => F::R8Sint,
        70 | 71 => F::Bc1RgbaUnorm,
        72 => F::Bc1RgbaUnormSrgb,
        73 | 74 => F::Bc2RgbaUnorm,
        75 => F::Bc2RgbaUnormSrgb,
        76 | 77 => F::Bc3RgbaUnorm,
        78 => F::Bc3RgbaUnormSrgb,
        79 | 80 => F::Bc4RUnorm,
        81 => F::Bc4RSnorm,
        82 | 83 => F::Bc5RgUnorm,
        84 => F::Bc5RgSnorm,
        87 | 88 | 90 => F::Bgra8Unorm,
        91 | 93 => F::Bgra8UnormSrgb,
        94 | 95 => F::Bc6hRgbUfloat,
        96 => F::Bc6hRgbFloat,
        97 | 98 => F::Bc7RgbaUnorm,
        99 => F::Bc7RgbaUnormSrgb,
        _ => return None,
    })
}

/// Bytes per 4×4 block (or per texel for uncompressed formats) and the
/// block size.
#[must_use]
pub fn block(format: TextureFormat) -> (u32, u32) {
    let (w, _) = format.block_dimensions();
    let bytes = format
        .block_copy_size(Some(wgpu::TextureAspect::All))
        .or_else(|| format.block_copy_size(Some(wgpu::TextureAspect::DepthOnly)))
        .unwrap_or(4);
    (bytes, w)
}

/// The wgpu vertex format of a DXGI input element format.
#[must_use]
pub fn vertex_format(dxgi: u32) -> Option<VertexFormat> {
    use VertexFormat as V;
    Some(match dxgi {
        2 => V::Float32x4,
        3 => V::Uint32x4,
        4 => V::Sint32x4,
        6 => V::Float32x3,
        7 => V::Uint32x3,
        8 => V::Sint32x3,
        10 => V::Float16x4,
        11 => V::Unorm16x4,
        12 => V::Uint16x4,
        13 => V::Snorm16x4,
        14 => V::Sint16x4,
        16 => V::Float32x2,
        17 => V::Uint32x2,
        18 => V::Sint32x2,
        28 => V::Unorm8x4,
        30 => V::Uint8x4,
        31 => V::Snorm8x4,
        32 => V::Sint8x4,
        34 => V::Float16x2,
        35 => V::Unorm16x2,
        36 => V::Uint16x2,
        37 => V::Snorm16x2,
        38 => V::Sint16x2,
        41 => V::Float32,
        42 => V::Uint32,
        43 => V::Sint32,
        87 => V::Unorm8x4Bgra,
        _ => return None,
    })
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn f32_at(b: &[u8], at: usize) -> f32 {
    f32::from_bits(u32_at(b, at))
}

fn blend_factor(v: u32) -> BlendFactor {
    use BlendFactor as B;
    match v {
        1 => B::Zero,
        3 => B::Src,
        4 => B::OneMinusSrc,
        5 => B::SrcAlpha,
        6 => B::OneMinusSrcAlpha,
        7 => B::DstAlpha,
        8 => B::OneMinusDstAlpha,
        9 => B::Dst,
        10 => B::OneMinusDst,
        11 => B::SrcAlphaSaturated,
        14 => B::Constant,
        15 => B::OneMinusConstant,
        16 => B::Src1,
        17 => B::OneMinusSrc1,
        18 => B::Src1Alpha,
        19 => B::OneMinusSrc1Alpha,
        _ => B::One,
    }
}

/// D3D11 blend factors for alpha use the alpha form of colour factors.
fn alpha_factor(v: u32) -> BlendFactor {
    match v {
        3 => BlendFactor::SrcAlpha,
        4 => BlendFactor::OneMinusSrcAlpha,
        9 => BlendFactor::DstAlpha,
        10 => BlendFactor::OneMinusDstAlpha,
        16 => BlendFactor::Src1Alpha,
        17 => BlendFactor::OneMinusSrc1Alpha,
        _ => blend_factor(v),
    }
}

fn blend_op(v: u32) -> BlendOperation {
    match v {
        2 => BlendOperation::Subtract,
        3 => BlendOperation::ReverseSubtract,
        4 => BlendOperation::Min,
        5 => BlendOperation::Max,
        _ => BlendOperation::Add,
    }
}

/// The blend of render target `slot` from a `D3D11_BLEND_DESC`, and its
/// write mask. `None` desc is the default (no blending, all channels).
#[must_use]
pub fn blend(desc: Option<&[u8]>, slot: usize) -> (Option<BlendState>, ColorWrites) {
    let Some(d) = desc else {
        return (None, ColorWrites::ALL);
    };
    let independent = u32_at(d, 4) != 0;
    let at = 8 + 32 * if independent { slot } else { 0 };
    let mask = ColorWrites::from_bits_truncate(d.get(at + 28).copied().unwrap_or(0xf).into());
    if u32_at(d, at) == 0 {
        return (None, mask);
    }
    let state = BlendState {
        color: BlendComponent {
            src_factor: blend_factor(u32_at(d, at + 4)),
            dst_factor: blend_factor(u32_at(d, at + 8)),
            operation: blend_op(u32_at(d, at + 12)),
        },
        alpha: BlendComponent {
            src_factor: alpha_factor(u32_at(d, at + 16)),
            dst_factor: alpha_factor(u32_at(d, at + 20)),
            operation: blend_op(u32_at(d, at + 24)),
        },
    };
    // Min and max ignore the factors in D3D; wgpu wants them One.
    let fix = |mut c: BlendComponent| {
        if matches!(c.operation, BlendOperation::Min | BlendOperation::Max) {
            c.src_factor = BlendFactor::One;
            c.dst_factor = BlendFactor::One;
        }
        c
    };
    (
        Some(BlendState {
            color: fix(state.color),
            alpha: fix(state.alpha),
        }),
        mask,
    )
}

/// Whether the blend uses alpha to coverage.
#[must_use]
pub fn alpha_to_coverage(desc: Option<&[u8]>) -> bool {
    desc.is_some_and(|d| u32_at(d, 0) != 0)
}

#[must_use]
pub fn compare(v: u32) -> CompareFunction {
    use CompareFunction as C;
    match v {
        1 => C::Never,
        2 => C::Less,
        3 => C::Equal,
        4 => C::LessEqual,
        5 => C::Greater,
        6 => C::NotEqual,
        7 => C::GreaterEqual,
        _ => C::Always,
    }
}

fn stencil_op(v: u32) -> StencilOperation {
    use StencilOperation as S;
    match v {
        2 => S::Zero,
        3 => S::Replace,
        4 => S::IncrementClamp,
        5 => S::DecrementClamp,
        6 => S::Invert,
        7 => S::IncrementWrap,
        8 => S::DecrementWrap,
        _ => S::Keep,
    }
}

fn stencil_face(d: &[u8], at: usize) -> StencilFaceState {
    StencilFaceState {
        fail_op: stencil_op(u32_at(d, at)),
        depth_fail_op: stencil_op(u32_at(d, at + 4)),
        pass_op: stencil_op(u32_at(d, at + 8)),
        compare: compare(u32_at(d, at + 12)),
    }
}

/// A `D3D11_DEPTH_STENCIL_DESC` (or the default for `None`) for a target
/// of `format`.
#[must_use]
pub fn depth_stencil(
    desc: Option<&[u8]>,
    format: TextureFormat,
    bias: DepthBiasState,
) -> wgpu::DepthStencilState {
    let default = [1u32, 1, 2, 0, 0xffff, 1, 1, 1, 8, 1, 1, 1, 8];
    let mut owned = Vec::new();
    let d = desc.unwrap_or_else(|| {
        for v in default {
            owned.extend_from_slice(&v.to_le_bytes());
        }
        // Read and write masks are bytes.
        owned[16] = 0xff;
        owned[17] = 0xff;
        owned[18] = 0;
        owned[19] = 0;
        &owned
    });
    let depth_on = u32_at(d, 0) != 0;
    let stencil_on = u32_at(d, 12) != 0 && format.has_stencil_aspect();
    wgpu::DepthStencilState {
        format,
        depth_write_enabled: Some(depth_on && u32_at(d, 4) != 0),
        depth_compare: Some(if depth_on {
            compare(u32_at(d, 8))
        } else {
            CompareFunction::Always
        }),
        stencil: if stencil_on {
            wgpu::StencilState {
                front: stencil_face(d, 20),
                back: stencil_face(d, 36),
                read_mask: u32::from(d[16]),
                write_mask: u32::from(d[17]),
            }
        } else {
            wgpu::StencilState::default()
        },
        bias,
    }
}

/// Culling, winding, depth bias and scissor from a
/// `D3D11_RASTERIZER_DESC` (default: back faces culled, clockwise front).
#[must_use]
pub fn raster(desc: Option<&[u8]>) -> (Option<CullMode>, FrontFace, DepthBiasState, bool) {
    let Some(d) = desc else {
        return (
            Some(CullMode::Back),
            FrontFace::Cw,
            DepthBiasState::default(),
            false,
        );
    };
    let cull = match u32_at(d, 4) {
        2 => Some(CullMode::Front),
        3 => Some(CullMode::Back),
        _ => None,
    };
    let front = if u32_at(d, 8) != 0 {
        FrontFace::Ccw
    } else {
        FrontFace::Cw
    };
    let bias = DepthBiasState {
        constant: u32_at(d, 12).cast_signed(),
        slope_scale: f32_at(d, 20),
        clamp: f32_at(d, 16),
    };
    (cull, front, bias, u32_at(d, 28) != 0)
}

#[must_use]
pub fn topology(v: u32) -> Option<PrimitiveTopology> {
    Some(match v {
        1 => PrimitiveTopology::PointList,
        2 => PrimitiveTopology::LineList,
        3 => PrimitiveTopology::LineStrip,
        4 => PrimitiveTopology::TriangleList,
        5 => PrimitiveTopology::TriangleStrip,
        _ => return None,
    })
}

fn address(v: u32) -> AddressMode {
    match v {
        // Mirror once (5) has no wgpu twin.
        2 | 5 => AddressMode::MirrorRepeat,
        3 => AddressMode::ClampToEdge,
        4 => AddressMode::ClampToBorder,
        _ => AddressMode::Repeat,
    }
}

/// A `D3D11_SAMPLER_DESC`. `filterable` false forces nearest filtering
/// (for textures wgpu cannot filter).
#[must_use]
pub fn sampler(desc: &[u8], filterable: bool) -> wgpu::SamplerDescriptor<'static> {
    let filter = u32_at(desc, 0);
    let comparison = filter & 0x80 != 0;
    let anisotropic = filter & 0x40 != 0;
    let linear = |bit: u32| filterable && (anisotropic || filter & bit != 0);
    let border = [
        f32_at(desc, 28),
        f32_at(desc, 32),
        f32_at(desc, 36),
        f32_at(desc, 40),
    ];
    let border_color = match border {
        [0.0, 0.0, 0.0, 0.0] => SamplerBorderColor::TransparentBlack,
        [1.0, 1.0, 1.0, 1.0] => SamplerBorderColor::OpaqueWhite,
        _ => SamplerBorderColor::OpaqueBlack,
    };
    let max_lod = f32_at(desc, 48);
    let mag = if linear(0x4) {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    };
    let min = if linear(0x10) {
        FilterMode::Linear
    } else {
        FilterMode::Nearest
    };
    let mip = if linear(0x1) {
        MipmapFilterMode::Linear
    } else {
        MipmapFilterMode::Nearest
    };
    let aniso = if anisotropic && filterable {
        u32_at(desc, 20).clamp(1, 16) as u16
    } else {
        1
    };
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: address(u32_at(desc, 4)),
        address_mode_v: address(u32_at(desc, 8)),
        address_mode_w: address(u32_at(desc, 12)),
        mag_filter: mag,
        min_filter: min,
        mipmap_filter: mip,
        lod_min_clamp: f32_at(desc, 44).max(0.0),
        lod_max_clamp: if max_lod > 1000.0 {
            32.0
        } else {
            max_lod.max(0.0)
        },
        compare: comparison.then(|| compare(u32_at(desc, 24))),
        anisotropy_clamp: if mag == FilterMode::Linear
            && min == FilterMode::Linear
            && mip == MipmapFilterMode::Linear
        {
            aniso
        } else {
            1
        },
        border_color: Some(border_color),
    }
}
