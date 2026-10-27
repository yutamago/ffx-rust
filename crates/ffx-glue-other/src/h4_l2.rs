//! Continuation round 10 of list `h4-glue-other`, batch 2: the guild
//! map's VFX render data update.

use ffx_bridge::x87::{load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::pass;
use crate::h4_i1::MUL_MATRIX;
use crate::h4_k1::{GET_BUFFER, SET_VISIBLE};
use crate::h4_k4::GET_RENDER_DATA;

/// The bounds' starts (`0x00b17504`, f32 max; `0x00b1f2d0`, its
/// negative).
pub const LOW_START: u32 = 0x00b1_7504;
/// See [`LOW_START`].
pub const HIGH_START: u32 = 0x00b1_f2d0;
/// The colour scales (f64): `0x00b170b8` for the tint bytes,
/// `0x00b50198` for the primitives' colour bytes.
pub const TINT_SCALE: u32 = 0x00b1_70b8;
/// See [`TINT_SCALE`].
pub const COLOUR_SCALE: u32 = 0x00b5_0198;
/// `g_GuideCameraInverse` (`0x0230fce0`).
pub const GUIDE_CAMERA_INVERSE: u32 = 0x0230_fce0;
/// The segment records' end bound (`0xca8`, 30 records of 0x6c bytes).
const SEGMENTS_END: u32 = 0xca8;

/// `fld qword` of the constant at `sym`.
fn constant(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

/// A segment's primitive count (`+2`, i16).
fn count(h: &impl Host, segment: u32) -> i32 {
    i32::from(h.read::<i16>(segment.wrapping_add(2)))
}

/// The word `off` bytes before the end of segment record `record` (its
/// buffers: `-0x60` positions, `-0x58` colours, `-0x50` indices).
fn entry(h: &impl Host, data: u32, record: u32, off: u32) -> u32 {
    let segments: u32 = h.read(data.wrapping_add(0x94));
    h.read(segments.wrapping_add(record).wrapping_sub(off))
}

/// `updateGuildMapRenderData(lo, hi, map, tint)` (`0x00717620`, cdecl) ->
/// 1, or 0 when the render data, a segment buffer is missing, a segment
/// has a type 1..7 or there are more than 29 segments. Halves the tint's
/// alpha (`+0x1b`). The map (`+0x10` i16 primitive count, segments at
/// `+4`, vertices (3 i16) at `+8`, offsets from the map) is read segment
/// by segment (16-byte header: `+1` type, `+2` i16 primitives; type 0's
/// 20-byte primitives follow: colour bytes `+0..+11`, vertex indices
/// u16 `+0xc..+0x10`): each type-0 segment gets its buffer
/// (`graphicDynGeoMemManagerGetBufferForSegment`, 3 per primitive) and
/// fills positions (f32 of the i16s), colours (byte * colour scale
/// times the tint byte * tint scale, as the original's x87 order) and
/// indices; the bounds grow over the positions. Then the render data gets
/// the tint's `+0` (`+0xb0`), the matrix (`[tint + 0x20]`) times
/// `g_GuideCameraInverse` with its last column (0, 0, 0, 1), flag 1 in
/// `+0xc8`, `+0x80` 0 and the bounds (`+0x98`..`+0xac`), and is made
/// visible.
#[allow(clippy::too_many_lines)] // the segments, then the render data
pub fn update_guild_map(h: &mut impl Host, lo: u32, hi: u32, map: u32, tint: u32) -> u32 {
    let low = pass(h.read(h.rebase(LOW_START)));
    let high = pass(h.read(h.rebase(HIGH_START)));
    let mut bounds = [low, low, low, high, high, high];
    let data = h.call_sym(GET_RENDER_DATA, Conv::Cdecl, &[lo, hi]);
    if data == 0 {
        return 0;
    }
    let alpha: u8 = h.read(tint.wrapping_add(0x1b));
    h.write(tint.wrapping_add(0x1b), alpha >> 1);
    let mut remaining = i32::from(h.read::<i16>(map.wrapping_add(0x10)));
    let mut segment = map.wrapping_add(h.read(map.wrapping_add(4)));
    let points = map.wrapping_add(h.read(map.wrapping_add(8)));
    let mut record = 0u32;
    let mut index = 0u32;
    while remaining > 0 {
        record += 0x6c;
        if record >= SEGMENTS_END {
            return 0;
        }
        let kind: i8 = h.read(segment.wrapping_add(1));
        let mut prim = segment.wrapping_add(0x10);
        if kind == 0 {
            let n3 = count(h, segment).wrapping_mul(3) as u32;
            if h.call_sym(GET_BUFFER, Conv::Cdecl, &[data, index, n3, n3]) == 0 {
                return 0;
            }
            let colour_scale = constant(h, COLOUR_SCALE);
            let tint_scale = constant(h, TINT_SCALE);
            let mut k = 0i32;
            let mut at = 0u32;
            while k < count(h, segment) {
                let vertices = entry(h, data, record, 0x60);
                let base = vertices.wrapping_add(at);
                for v in 0..3u32 {
                    let i: u16 = h.read(prim.wrapping_add(0xc + 2 * v));
                    let point = points.wrapping_add(u32::from(i) * 6);
                    for c in 0..3u32 {
                        let x = i32::from(h.read::<i16>(point.wrapping_add(2 * c)));
                        h.write(base.wrapping_add(12 * v + 4 * c), store(f64::from(x)));
                    }
                }
                for v in 0..3u32 {
                    let p = base.wrapping_add(12 * v);
                    for c in 0..3u32 {
                        let x = load(h.read(p.wrapping_add(4 * c)));
                        let c = c as usize;
                        // min: `x < low` (no NaN); max: `x > high`
                        if x < load(bounds[c]) {
                            bounds[c] = h.read(p.wrapping_add(4 * c as u32));
                        }
                        if x > load(bounds[3 + c]) {
                            bounds[3 + c] = h.read(p.wrapping_add(4 * c as u32));
                        }
                    }
                }
                let colours = entry(h, data, record, 0x58).wrapping_add(at / 0x24 * 0x30);
                let mut t = [0f64; 4];
                for (j, s) in t.iter_mut().enumerate() {
                    let b: u8 = h.read(tint.wrapping_add(0x18 + j as u32));
                    *s = load(store(mul(f64::from(b), tint_scale)));
                }
                for j in 0..12u32 {
                    let p = f64::from(h.read::<u8>(prim.wrapping_add(j)));
                    let s = t[(j % 4) as usize];
                    let v = if j % 4 == 3 {
                        mul(mul(p, s), colour_scale)
                    } else {
                        mul(mul(p, colour_scale), s)
                    };
                    h.write(colours.wrapping_add(4 * j), store(v));
                }
                let indices = entry(h, data, record, 0x50).wrapping_add(at / 0x24 * 6);
                let first = (k as u32).wrapping_mul(3) as u16;
                h.write(indices, first);
                h.write(indices.wrapping_add(2), first.wrapping_add(1));
                h.write(indices.wrapping_add(4), first.wrapping_add(2));
                k += 1;
                at += 0x24;
                prim = prim.wrapping_add(0x14);
            }
            let segments: u32 = h.read(data.wrapping_add(0x94));
            let n3 = count(h, segment).wrapping_mul(3);
            h.write(segments.wrapping_add(record).wrapping_sub(0x68), n3);
            let segments: u32 = h.read(data.wrapping_add(0x94));
            let n3 = count(h, segment).wrapping_mul(3);
            h.write(segments.wrapping_add(record).wrapping_sub(0x64), n3);
        } else if (1..=7).contains(&kind) {
            return 0;
        }
        remaining -= count(h, segment);
        index += 1;
        segment = prim;
    }
    let colour: u32 = h.read(tint);
    h.write(data.wrapping_add(0xb0), colour);
    let matrix: u32 = h.read(tint.wrapping_add(0x20));
    for k in 0..16u32 {
        let w: u32 = h.read(matrix.wrapping_add(4 * k));
        h.write(data.wrapping_add(4 * k), w);
    }
    let inverse = h.rebase(GUIDE_CAMERA_INVERSE);
    h.call_sym(MUL_MATRIX, Conv::Cdecl, &[data, inverse, data]);
    for off in [0xc, 0x1c, 0x2c] {
        h.write(data.wrapping_add(off), 0u32);
    }
    h.write(data.wrapping_add(0x3c), 1.0f32.to_bits());
    let flags: u32 = h.read(data.wrapping_add(0xc8));
    h.write(data.wrapping_add(0xc8), flags | 1);
    h.write(data.wrapping_add(0x80), 0u32);
    for (k, b) in bounds.into_iter().enumerate() {
        h.write(data.wrapping_add(0x98 + 4 * k as u32), pass(b));
    }
    h.call_sym(SET_VISIBLE, Conv::Cdecl, &[lo, hi, 1]);
    1
}
