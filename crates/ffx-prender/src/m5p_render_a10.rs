//! M5 stage 1, `PhyreEngine` list `render-a`, batch 9: `PPhysicsMesh`'s
//! vertex stream converter (`FUN_0049a520`): a stream of any vertex
//! element format widened to three floats per vertex. Reached only
//! through `PPhysicsMeshBullet`'s vtable (`0x00b39ee8`, slot 0
//! `FUN_005f4830` -> `PPhysicsMesh_decodeStream_0049bec0`).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_render_a3::MEMCPY;

/// The CRT's `memset(dst, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PGeometry_FormatSize_0055bc40(format)` (cdecl): the bytes per
/// vertex of a format, the source stride.
pub const FORMAT_STRIDE: u32 = 0x0055_bc40;

/// How a format's elements become floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elem {
    /// Raw words copied (`memcpy`, the source advancing by the stride
    /// only).
    Copy,
    /// An integer of `bytes` bytes (signed or not) made a float, then
    /// divided by (`Div`) or multiplied with (`Mul`) the f64 at an
    /// address, or left as it is (`Plain`).
    Int {
        bytes: u32,
        signed: bool,
        scale: Scale,
    },
    /// A half float (`PPhysicsMesh_halfToFloat`).
    Half,
    /// A packed 11/11/10 normal (`PPhysicsMesh_unpackNormal`), one word
    /// per vertex.
    Normal,
}

/// The scale of an [`Elem::Int`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Plain,
    Div(u32),
    Mul(u32),
}

/// The element kind of a converter case (the value of
/// `PPhysicsMesh_formatSize`, or 0x32 for format 0x32): the jump table
/// at `0x0049ae98` through the index bytes at `0x0049aed0`.
#[must_use]
pub fn elem(case: u32) -> Option<Elem> {
    use Scale::{Div, Mul, Plain};
    let int = |bytes, signed, scale| {
        Some(Elem::Int {
            bytes,
            signed,
            scale,
        })
    };
    match case {
        0 => Some(Elem::Copy),
        4 => Some(Elem::Half),
        8 => int(4, false, Plain),
        12 => int(2, false, Div(0x00b1_70d8)),
        16 => int(1, false, Div(0x00b1_70c0)),
        20 => int(2, false, Plain),
        24 => int(1, false, Plain),
        28 => int(4, true, Plain),
        32 => int(2, true, Mul(0x00b1_70b0)),
        36 => int(1, true, Mul(0x00b1_70b8)),
        40 => int(2, true, Plain),
        44 => int(1, true, Plain),
        0x32 => Some(Elem::Normal),
        _ => None,
    }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    /// `FUN_0049a520(out, src, format, count)` (cdecl) -> the end of `out`:
    /// nothing for a null `out`; else `out` cleared (`memset`, 12 bytes
    /// per vertex) and each vertex's first `n` floats written from the
    /// source (`n` = the stride over the element size, at most 3; the
    /// stride `Phyre_PGeometry_FormatSize(format)`). The source pointer
    /// moves by the bytes the elements read plus the stride per vertex
    /// (the original never rewinds it), by the stride for raw copies, by
    /// 4 for packed normals. Formats without a case (or no vertices)
    /// return `out` cleared.
    pub fn convert(h: &mut impl Host, out: u32, src: u32, format: u32, count: u32) -> u32 {
        let case = if format == 0x32 {
            0x32
        } else {
            crate::m5p_render_a4::bridge::format_size(format)
        };
        let stride = cdecl(h, FORMAT_STRIDE, &[format]);
        if out == 0 {
            return 0;
        }
        cdecl(h, MEMSET, &[out, 0, count.wrapping_mul(12)]);
        let Some(kind) = elem(case) else {
            return out;
        };
        if count == 0 {
            return out;
        }
        let mut o = out;
        let mut s = src;
        match kind {
            Elem::Copy => {
                let n = (stride >> 2).min(3);
                for _ in 0..count {
                    cdecl(h, MEMCPY, &[o, s, n << 2]);
                    s = s.wrapping_add(stride);
                    o = o.wrapping_add(12);
                }
            }
            Elem::Normal => {
                for _ in 0..count {
                    crate::m5p_render_a4::bridge::unpack_normal(h, s, o, o + 4, o + 8);
                    s = s.wrapping_add(4);
                    o = o.wrapping_add(12);
                }
            }
            Elem::Half => {
                let n = (stride >> 1).min(3);
                for _ in 0..count {
                    for k in 0..n {
                        let v: u16 = h.read(s);
                        let f = crate::m5p_render_a4::bridge::half_to_float(h, u32::from(v));
                        h.write(o + 4 * k, x87::store(x87::load(f.to_bits())));
                        s = s.wrapping_add(2);
                    }
                    s = s.wrapping_add(stride);
                    o = o.wrapping_add(12);
                }
            }
            Elem::Int {
                bytes,
                signed,
                scale,
            } => {
                let n = (stride / bytes).min(3);
                let k = match scale {
                    Scale::Plain => 0.0,
                    Scale::Div(at) | Scale::Mul(at) => {
                        let at = h.rebase(at);
                        f64::from_bits(h.read(at))
                    }
                };
                for _ in 0..count {
                    for i in 0..n {
                        let v = read_int(h, s, bytes, signed);
                        let bits = match scale {
                            Scale::Plain => x87::store(v),
                            Scale::Div(_) => x87::store(x87::div(v, k)),
                            Scale::Mul(_) => x87::store(x87::mul(v, k)),
                        };
                        h.write(o + 4 * i, bits);
                        s = s.wrapping_add(bytes);
                    }
                    s = s.wrapping_add(stride);
                    o = o.wrapping_add(12);
                }
            }
        }
        o
    }

    /// An element as `fild` loads it (an unsigned word gets 2^32 added
    /// when its top bit is set, exactly).
    fn read_int(h: &mut impl Host, at: u32, bytes: u32, signed: bool) -> f64 {
        match (bytes, signed) {
            (1, false) => f64::from(h.read::<u8>(at)),
            (1, true) => f64::from(h.read::<i8>(at)),
            (2, false) => f64::from(h.read::<u16>(at)),
            (2, true) => f64::from(h.read::<i16>(at)),
            (_, false) => f64::from(h.read::<u32>(at)),
            (_, true) => f64::from(h.read::<i32>(at)),
        }
    }
}
