//! Continuation round 11 of list `h4-glue-other`, batch 4: the event
//! shapes' VFX render data.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicFontGetScreenWH(&w, &h)` (cdecl).
pub const SCREEN_WH: u32 = 0x0064_0db0;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const RENDER_DATA: u32 = 0x0064_53d0;
/// `calculateVFXSP_AlphaFunction(mode)` (cdecl).
pub const ALPHA_FUNCTION: u32 = 0x0071_2c30;
/// `graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices,
/// indices)` (cdecl) -> 0 when out of memory.
pub const GET_BUFFER: u32 = 0x0063_fdb0;
/// `FFXVu0MulMatrix(dst, a, b)` (cdecl).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `graphicVFXSetVisible(lo, hi, visible)` (cdecl).
pub const SET_VISIBLE: u32 = 0x0064_5710;
/// `g_pppFillMatrix` (`0x0230fca0`).
pub const FILL_MATRIX: u32 = 0x0230_fca0;
/// `g_AlphaScale` (`0x00c3a4d0`, f32).
pub const ALPHA_SCALE: u32 = 0x00c3_a4d0;
/// `f32` constants: the bounds' start (`0x00b17504` the minimum's,
/// `0x00b1f2d0` the maximum's).
pub const MIN_START: u32 = 0x00b1_7504;
/// See [`MIN_START`].
pub const MAX_START: u32 = 0x00b1_f2d0;
/// `f64` constants: 1/512 (`0x00b41cc0`), 416 (`0x00b41d30`), 15
/// (`0x00b922c0`), 1/256 (`0x00b50198`), 1/128 (`0x00b170b8`), 1/4096
/// (`0x00b50190`), 0.5 (`0x00b92220`), 0.85 (`0x00b501a8`).
pub const INV_512: u32 = 0x00b4_1cc0;
/// See [`INV_512`].
pub const HEIGHT_416: u32 = 0x00b4_1d30;
/// See [`INV_512`].
pub const FIFTEEN: u32 = 0x00b9_22c0;
/// See [`INV_512`].
pub const INV_256: u32 = 0x00b5_0198;
/// See [`INV_512`].
pub const INV_128: u32 = 0x00b1_70b8;
/// See [`INV_512`].
pub const INV_4096: u32 = 0x00b5_0190;
/// See [`INV_512`].
pub const HALF: u32 = 0x00b9_2220;
/// See [`INV_512`].
pub const MARGIN: u32 = 0x00b5_01a8;
/// Bytes of [`update_atel_shp`]'s frame, from the original's `ebp -
/// 0x94` up to its cookie: the screen height (`+0x14`) and width
/// (`+0x20`) the font call fills, the transformed bounds' maximum
/// (`+0x70`) and minimum (`+0x80`), each a vector with w 1.
pub const ATEL_FRAME: u32 = 0x90;

/// The six alpha buckets' instance offsets (ids 10000 apart).
const BUCKET_STEP: u32 = 10000;

/// The instance `(lo, hi)` plus `k` as the original's 64-bit add.
fn instance(lo: u32, hi: u32, k: u32) -> (u32, u32) {
    let id = (u64::from(hi) << 32 | u64::from(lo)).wrapping_add(u64::from(k));
    (id as u32, (id >> 32) as u32)
}

/// The bucket of an alpha function (0, 2, 0x10, 0x20, 0x100, 0x200 ->
/// 0..5), `None` for any other.
fn bucket(alpha: u32) -> Option<u32> {
    match alpha {
        0 => Some(0),
        2 => Some(1),
        0x10 => Some(2),
        0x20 => Some(3),
        0x100 => Some(4),
        0x200 => Some(5),
        _ => None,
    }
}

fn f64_at(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

fn f32_at(h: &impl Host, at: u32) -> f64 {
    load(h.read(at))
}

/// An `i32` as the original's `fild` then `fstp dword`.
fn int_f32(v: i32) -> u32 {
    store(f64::from(v))
}

/// `updateVFXRenderDataAtelShp(lo, hi, shapes, obj, index)`
/// (`0x007196f0`, cdecl; `frame` see [`ATEL_FRAME`]) -> 1, or 0 when a
/// bucket's render data or buffer is missing. The shape `index` (record
/// at `shapes + [shapes + 0x10 + 8 index]`, i16) unless flagged (`+0` bit
/// 3): its primitives (`+4` relative, 8 bytes: RGBA and the alpha mode)
/// go to six instances by alpha function, each primitive a quad (four
/// vertices from the i16 corners at `+0x20`, scaled by 1/15 and the
/// screen height / 416; colours by 1/256 (alpha by `g_AlphaScale`) and the
/// object's tint; uvs by 1/4096), the vertices also widening a bounding
/// box. Then each instance gets the object's matrix, its screen offset
/// (half the screen, and on screens 720 high or more a shift that keeps
/// the box inside 85% of it) and is made visible.
#[allow(clippy::too_many_lines)] // the original's passes
pub fn update_atel_shp(
    h: &mut impl Host,
    frame: u32,
    lo: u32,
    hi: u32,
    shapes: u32,
    obj: u32,
    index: u32,
) -> u32 {
    let height_at = frame + 0x14;
    let width_at = frame + 0x20;
    let b_vec = frame + 0x70;
    let a_vec = frame + 0x80;
    // the bounds
    let lo_start = store(f32_at(h, h.rebase(MIN_START)));
    let hi_start = store(f32_at(h, h.rebase(MAX_START)));
    let (mut min, mut max) = ([lo_start; 3], [hi_start; 3]);
    let mut counters = [0u32; 6];
    let mut last_bucket = 0u32;
    h.call_sym(SCREEN_WH, Conv::Cdecl, &[width_at, height_at]);
    let width = h.read::<u32>(width_at).cast_signed();
    let height = h.read::<u32>(height_at).cast_signed();
    let sx = store(mul(load(int_f32(width)), f64_at(h, INV_512)));
    let sy = store(div(load(int_f32(height)), f64_at(h, HEIGHT_416)));
    let entry = shapes.wrapping_add(0x10 + 8 * (index & 0xffff));
    let record = shapes.wrapping_add(i32::from(h.read::<i16>(entry)).cast_unsigned());
    if h.read::<u8>(record) & 8 == 0 {
        let prims = record.wrapping_add(h.read(record + 4));
        // the counts: cleared, then 12 vertices and 6 indices per
        // primitive in its bucket's instance
        for k in 0..6 {
            let (l, hh) = instance(lo, hi, k * BUCKET_STEP);
            let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[l, hh]);
            if rd != 0 {
                let seg: u32 = h.read(rd + 0x94);
                h.write(seg + 8, 0u32);
                let seg: u32 = h.read(rd + 0x94);
                h.write(seg + 4, 0u32);
            }
        }
        let mut j = 0i32;
        while j < i32::from(h.read::<i16>(record + 2)) {
            let mode = u32::from(h.read::<u8>(prims.wrapping_add(4 + 8 * j.cast_unsigned())));
            let alpha = h.call_sym(ALPHA_FUNCTION, Conv::Cdecl, &[mode]);
            if let Some(b) = bucket(alpha) {
                let (l, hh) = instance(lo, hi, b * BUCKET_STEP);
                let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[l, hh]);
                if rd != 0 {
                    let seg: u32 = h.read(rd + 0x94);
                    let v: u32 = h.read(seg + 4);
                    h.write(seg + 4, v.wrapping_add(0xc));
                    let seg: u32 = h.read(rd + 0x94);
                    let v: u32 = h.read(seg + 8);
                    h.write(seg + 8, v.wrapping_add(6));
                }
            }
            j += 1;
        }
        for k in 0..6 {
            let (l, hh) = instance(lo, hi, k * BUCKET_STEP);
            let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[l, hh]);
            if rd != 0 {
                let seg: u32 = h.read(rd + 0x94);
                let (v, i): (u32, u32) = (h.read(seg + 4), h.read(seg + 8));
                if h.call_sym(GET_BUFFER, Conv::Cdecl, &[rd, 0, v, i]) == 0 {
                    return 0;
                }
            }
        }
        // the quads
        let fifteen = f64_at(h, FIFTEEN);
        let inv256 = f64_at(h, INV_256);
        let inv128 = f64_at(h, INV_128);
        let inv4096 = f64_at(h, INV_4096);
        let scale = load(h.read(h.rebase(ALPHA_SCALE)));
        let mut corners = record + 0x20;
        let mut prim = prims;
        let mut j = 0i32;
        while j < i32::from(h.read::<i16>(record + 2)) {
            let alpha = {
                let mode = u32::from(h.read::<u8>(prim + 4));
                h.call_sym(ALPHA_FUNCTION, Conv::Cdecl, &[mode])
            };
            let (b, (l, hh)) = match bucket(alpha) {
                Some(b) => (b, instance(lo, hi, b * BUCKET_STEP)),
                None => (last_bucket, (lo, hi)),
            };
            last_bucket = b;
            let q = counters[b as usize];
            let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[l, hh]);
            if rd == 0 {
                return 0;
            }
            let seg: u32 = h.read(rd + 0x94);
            // positions
            let syf = load(sy);
            let vbuf: u32 = h.read(seg + 0xc);
            let v = vbuf.wrapping_add(q.wrapping_mul(48));
            let xs = [0u32, 2, 0, 2];
            let ys = [1u32, 1, 3, 3];
            for n in 0..4u32 {
                let x = f64::from(h.read::<i16>(corners + 2 * xs[n as usize]));
                let y = f64::from(h.read::<i16>(corners + 2 * ys[n as usize]));
                let px = store(mul(div(x, fifteen), syf));
                let py = store(mul(div(y, fifteen), syf));
                h.write(v + 12 * n, px);
                h.write(v + 12 * n + 4, py);
                h.write(v + 12 * n + 8, 1f32.to_bits());
            }
            // the bounds, from the vertices as stored
            for n in 0..4u32 {
                for (axis, off) in [(0usize, 0u32), (1, 4), (2, 8)] {
                    let p: u32 = h.read(v + 12 * n + off);
                    if load(p) < load(min[axis]) {
                        min[axis] = store(load(p));
                    }
                }
                for (axis, off) in [(0usize, 0u32), (1, 4), (2, 8)] {
                    let p: u32 = h.read(v + 12 * n + off);
                    if load(p) > load(max[axis]) {
                        max[axis] = store(load(p));
                    }
                }
            }
            // colours
            let rgba: [u8; 4] = std::array::from_fn(|k| h.read(prim + k as u32));
            let mut r = store(mul(f64::from(rgba[0]), inv256));
            let mut g = store(mul(f64::from(rgba[1]), inv256));
            let mut bl = store(mul(inv256, f64::from(rgba[2])));
            let mut a = store(div(f64::from(rgba[3]), scale));
            if obj != 0 {
                let tint: [u8; 4] = std::array::from_fn(|k| h.read(obj + 4 + k as u32));
                r = store(mul(mul(f64::from(tint[0]), inv128), load(r)));
                g = store(mul(mul(f64::from(tint[1]), inv128), load(g)));
                bl = store(mul(mul(f64::from(tint[2]), inv128), load(bl)));
                a = store(mul(mul(inv128, f64::from(tint[3])), load(a)));
            } else {
                r = store(mul(1.0, load(r)));
                g = store(mul(1.0, load(g)));
                bl = store(mul(1.0, load(bl)));
                a = store(mul(1.0, load(a)));
            }
            let cbuf: u32 = h.read(seg + 0x14);
            let col = cbuf.wrapping_add(q.wrapping_mul(64));
            for n in 0..4u32 {
                for (k, v) in [r, g, bl, a].into_iter().enumerate() {
                    h.write(col + 16 * n + 4 * k as u32, v);
                }
            }
            // texture coordinates
            let raw: [i16; 4] = std::array::from_fn(|k| h.read(corners + 8 + 2 * k as u32));
            let u0 = store(mul(f64::from(raw[0]), inv4096));
            let v0 = store(mul(f64::from(raw[1]), inv4096));
            let u1 = store(mul(f64::from(raw[2]), inv4096));
            let v1 = store(mul(inv4096, f64::from(raw[3])));
            let top = sub(1.0, load(v0));
            let w0 = store(top);
            let x1 = store(add(load(u1), load(u0)));
            let y1 = store(sub(top, load(v1)));
            let ubuf: u32 = h.read(seg + 0x18);
            let t = ubuf.wrapping_add(q.wrapping_mul(32));
            for (k, v) in [u0, w0, x1, w0, u0, y1, x1, y1].into_iter().enumerate() {
                h.write(t + 4 * k as u32, v);
            }
            // indices
            let ibuf: u32 = h.read(seg + 0x1c);
            let i = ibuf.wrapping_add(q.wrapping_mul(12));
            let base = q.wrapping_mul(4);
            for (k, d) in [0u32, 2, 1, 2, 3, 1].into_iter().enumerate() {
                h.write(i + 2 * k as u32, base.wrapping_add(d) as u16);
            }
            counters[b as usize] = q.wrapping_add(1);
            corners += 0x10;
            prim += 8;
            j += 1;
        }
    }
    // the instances: the object's matrix, the screen offset, visible
    let half = f64_at(h, HALF);
    for k in 0..6 {
        let (l, hh) = instance(lo, hi, k * BUCKET_STEP);
        let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[l, hh]);
        if rd == 0 {
            continue;
        }
        if obj != 0 {
            let m: u32 = h.read(obj + 0x10);
            for w in 0..16 {
                let v: u32 = h.read(m + 4 * w);
                h.write(rd + 4 * w, v);
            }
            for off in [0xc, 0x1c, 0x2c] {
                h.write(rd + off, 0u32);
            }
            h.write(rd + 0x3c, 1f32.to_bits());
            let wf = load(int_f32(width));
            let x = store(sub(mul(f32_at(h, rd + 0x30), load(sx)), mul(wf, half)));
            h.write(rd + 0x30, x);
            let hf = load(int_f32(height));
            let y = store(sub(mul(f32_at(h, rd + 0x34), load(sy)), mul(half, hf)));
            h.write(rd + 0x34, y);
            let second: u32 = h.read(obj + 0x1c);
            if second == 0 {
                h.write(rd + 0x80, 0u32);
            } else {
                for w in 0..16 {
                    let v: u32 = h.read(second + 4 * w);
                    h.write(rd + 0x40 + 4 * w, v);
                }
                let fill = h.rebase(FILL_MATRIX);
                h.call_sym(MUL_MATRIX, Conv::Cdecl, &[rd + 0x40, fill, rd + 0x40]);
                h.write(rd + 0x80, 1u32);
            }
            let bounds = [min[0], min[1], min[2], max[0], max[1], max[2]];
            for (w, v) in bounds.into_iter().enumerate() {
                h.write(rd + 0x98 + 4 * w as u32, store(load(v)));
            }
        }
        if height >= 0x2d0 {
            shift_on_screen(h, rd, obj, width, height, sx, half, a_vec, b_vec);
        }
        h.call_sym(SET_VISIBLE, Conv::Cdecl, &[l, hh, 1]);
    }
    1
}

/// On screens 720 high or more, with the object's matrix unscaled (`+0`
/// and `+0x14` both 1): the render data's bounds transformed by its
/// matrix, and its offset (`+0x30`, `+0x34`) moved so they stay inside
/// 85% of the screen.
#[allow(clippy::too_many_arguments)]
fn shift_on_screen(
    h: &mut impl Host,
    rd: u32,
    obj: u32,
    width: i32,
    height: i32,
    sx: u32,
    half: f64,
    a_vec: u32,
    b_vec: u32,
) {
    let m: u32 = h.read(obj + 0x10);
    // the original's fucom: equal to 1 exactly, NaN not
    #[allow(clippy::float_cmp)]
    let unscaled = f32_at(h, m) == 1.0 && f32_at(h, m + 0x14) == 1.0;
    if !unscaled {
        return;
    }
    let margin = f64_at(h, MARGIN);
    let top = store(mul(mul(load(int_f32(height.wrapping_neg())), margin), half));
    let bottom = store(mul(mul(load(int_f32(height)), margin), half));
    let sxf = load(sx);
    let left = store(mul(
        mul(mul(load(int_f32(width.wrapping_neg())), sxf), margin),
        half,
    ));
    let right = store(mul(mul(mul(sxf, load(int_f32(width))), margin), half));
    for (w, off) in [0u32, 4, 8].into_iter().enumerate() {
        let v: u32 = h.read(rd + 0x98 + off);
        h.write(a_vec + 4 * w as u32, v);
        let v: u32 = h.read(rd + 0xa4 + off);
        h.write(b_vec + 4 * w as u32, v);
    }
    h.write(b_vec + 0xc, 1f32.to_bits());
    h.write(a_vec + 0xc, 1f32.to_bits());
    h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[a_vec, rd, a_vec]);
    h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[b_vec, rd, b_vec]);
    let ay = f32_at(h, a_vec + 4);
    let by = f32_at(h, b_vec + 4);
    let t = load(top);
    if t > ay {
        let v = store(add(sub(t, ay), f32_at(h, rd + 0x34)));
        h.write(rd + 0x34, v);
    } else if load(bottom) < by {
        let v = store(sub(f32_at(h, rd + 0x34), sub(by, load(bottom))));
        h.write(rd + 0x34, v);
    }
    let ax = f32_at(h, a_vec);
    let bx = f32_at(h, b_vec);
    let l = load(left);
    if l > ax {
        let v = store(add(sub(l, ax), f32_at(h, rd + 0x30)));
        h.write(rd + 0x30, v);
    } else if load(right) < bx {
        let v = store(sub(f32_at(h, rd + 0x30), sub(bx, load(right))));
        h.write(rd + 0x30, v);
    }
}
