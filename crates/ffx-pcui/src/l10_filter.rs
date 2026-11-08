//! M5 stage 1, round 10 (`tail10`, list `pc-ui`): the `GDraw` blur's
//! one-axis set-up `0x00a36590` (cdecl, ten words: `(axis, api, target,
//! obj, size, scale, rect, out, keep, c)`), called twice per pass by
//! `GDraw_filterPasses_00a36490`.
//!
//! From the blur size it builds up to nine taps (16-byte records in its
//! frame at `ebp - 0x94`: the offset along `axis` (word `axis`), 0 across
//! (word `1 - axis`), the weight (`+8`), 0 (`+0xc`)), widens `rect` by the
//! taps' reach (clamped to 0 and to the target's size), and draws:
//! - up to nine taps (`k = ceil((size - 1) / 2)`, `n = k + 1` from 2 to
//!   9): one box pass (`0x00a37010`) with offsets two texels apart
//!   (`(2i + 1.5 - n) * scale`), the last at `(n - 1) * scale`, the first
//!   at `(1 - n + 1 / (frac + 1)) * scale`, `frac = 1 - (2k + 1 - size) /
//!   2` weighting the edge taps;
//! - more: a downsampling axis pass (`0x00a37170`) by `p = ceil((n - 10) /
//!   71 * 7 + 2)` (at most nine taps, the scale times `p / 9` above), then
//!   a second axis pass over the `2n / p` texels left (at most 18, the
//!   scale times `(r - 1) / 17` above), odd and even counts placing the
//!   taps half a texel apart differently.
//!
//! After each pass the object's texture (`obj + 0x1c`) becomes the pass's
//! result, the old one given back through the API's `+0x48(texture, 0,
//! c)` unless it is `keep` or the result itself; `out` gets `rect` grown
//! by one texel, clamped. x87 arithmetic in the original's order
//! (`ffx_bridge::x87`), `ceil` the CRT's (import `0x00b0c480`).

use ffx_graphic::host::{Conv, Host};

/// `ceil`'s import slot (cdecl, a double; `ST0`).
pub const CEIL_SLOT: u32 = 0x00b0_c480;
/// `GDraw_filterBoxPass_00a37010` (cdecl, eight words).
pub const BOX_PASS: u32 = 0x00a3_7010;
/// `GDraw_filterAxisPass_00a37170` (cdecl, twelve words).
pub const AXIS_PASS: u32 = 0x00a3_7170;

/// The tap records' bytes (`ebp - 0x94` up to the cookie).
pub const TAPS: u32 = 0x90;

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::cast_possible_truncation)] // the original's quantities (k, n, p, r, ...); double halves, indices 0..4
pub mod bridge {
    use super::{AXIS_PASS, BOX_PASS, CEIL_SLOT, Conv, Host};
    use ffx_bridge::x87;

    /// `fild` of an int then `fstp m32`: the int as an `f32`.
    fn int_f(i: i32) -> f64 {
        f64::from(f32::from_bits(x87::store(f64::from(i))))
    }

    /// `fstp m32` then `fld`.
    fn r32(x: f64) -> f64 {
        f64::from(f32::from_bits(x87::store(x)))
    }

    /// The CRT's `ceil` of `x` (pushed as a double).
    fn ceil(h: &mut impl Host, x: f64) -> f64 {
        let f: u32 = h.read(h.rebase(CEIL_SLOT));
        let b = x.to_bits();
        h.call_st0_f64(f, Conv::Cdecl, &[b as u32, (b >> 32) as u32])
    }

    /// `v <= 0 ? 0 : v` (`setle; dec; and`).
    fn clamp0(v: u32) -> u32 {
        if v.cast_signed() <= 0 { 0 } else { v }
    }

    /// `v < lim ? v : lim`, signed.
    fn min_s(v: u32, lim: u32) -> u32 {
        if v.cast_signed() < lim.cast_signed() {
            v
        } else {
            lim
        }
    }

    /// One tap record `i` of the frame at `taps`: `pos` along `axis`, 0
    /// across, the weight.
    fn tap(h: &mut impl Host, taps: u32, axis: u32, i: u32, pos: u32, weight: u32) {
        let rec = taps.wrapping_add(16 * i);
        h.write(rec.wrapping_add(4 * axis), pos);
        h.write(rec.wrapping_add(4 * (1u32.wrapping_sub(axis))), 0_u32);
        h.write(rec.wrapping_add(0xc), 0_u32);
        h.write(rec.wrapping_add(8), weight);
    }

    /// `rect` widened by `dx`, `dy` and clamped to `[0, target)`.
    fn widen(h: &mut impl Host, rect: u32, target: u32, dx: u32, dy: u32) {
        let (tw, th): (u32, u32) = (h.read(target), h.read(target.wrapping_add(4)));
        let x0: u32 = h.read(rect);
        h.write(rect, clamp0(x0.wrapping_sub(dx)));
        let y0: u32 = h.read(rect.wrapping_add(4));
        h.write(rect.wrapping_add(4), clamp0(y0.wrapping_sub(dy)));
        let x1: u32 = h.read(rect.wrapping_add(8));
        h.write(rect.wrapping_add(8), min_s(x1.wrapping_add(dx), tw));
        let y1: u32 = h.read(rect.wrapping_add(12));
        h.write(rect.wrapping_add(12), min_s(y1.wrapping_add(dy), th));
    }

    /// After a pass: the object's texture becomes `r` (the old one given
    /// back unless `keep` or `r`), `out` is `rect` grown by a texel;
    /// returns the `eax` the original leaves (`out`'s last word).
    fn finish(h: &mut impl Host, a: &Args, r: u32) -> u32 {
        let old: u32 = h.read(a.obj.wrapping_add(0x1c));
        if old != a.keep && old != r {
            let give_back: u32 = h.read(a.api.wrapping_add(0x48));
            h.call(give_back, Conv::Stdcall, &[old, 0, a.c]);
        }
        h.write(a.obj.wrapping_add(0x1c), r);
        let (tw, th): (u32, u32) = (h.read(a.target), h.read(a.target.wrapping_add(4)));
        let v: [u32; 4] = std::array::from_fn(|i| h.read(a.rect.wrapping_add(4 * i as u32)));
        h.write(a.out, clamp0(v[0].wrapping_sub(1)));
        h.write(a.out.wrapping_add(4), clamp0(v[1].wrapping_sub(1)));
        h.write(a.out.wrapping_add(8), min_s(v[2].wrapping_add(1), tw));
        let last = min_s(v[3].wrapping_add(1), th);
        h.write(a.out.wrapping_add(12), last);
        last
    }

    /// The original's ten arguments.
    struct Args {
        api: u32,
        target: u32,
        obj: u32,
        rect: u32,
        out: u32,
        keep: u32,
        c: u32,
    }

    /// `0x00a36590(axis, api, target, obj, size, scale, rect, out, keep,
    /// c)` with its tap records at `taps` ([`super::TAPS`] bytes, the
    /// original's `ebp - 0x94`); returns the `eax` it leaves.
    #[allow(clippy::too_many_lines)] // one original
    pub fn axis_setup(h: &mut impl Host, taps: u32, args: [u32; 10]) -> u32 {
        let [axis, api, target, obj, size, scale, rect, out, keep, c] = args;
        let a = Args {
            api,
            target,
            obj,
            rect,
            out,
            keep,
            c,
        };
        let size = x87::load(size);
        let scale = x87::load(scale);
        let half = r32(x87::mul(x87::sub(size, 1.0), 0.5));
        let up = r32(ceil(h, half));
        let k = x87::ftol(h, up).cast_signed();
        let width = k.wrapping_mul(2).wrapping_add(1);
        let n = (width.wrapping_sub(1) >> 1).wrapping_add(1);
        let wf = int_f(width);
        let d = x87::mul(x87::sub(wf, size), 0.5);
        let frac = r32(x87::sub(1.0, d));
        let inv = r32(x87::div(1.0, size));
        if h.read::<u32>(obj.wrapping_add(0x1c)) == 0 || n <= 1 {
            return obj;
        }
        if n <= 9 {
            // the box pass
            let nf = int_f(n);
            let c0 = x87::sub(1.5, nf);
            let w2 = x87::store(x87::add(inv, inv));
            for i in 0..n {
                let pos = x87::store(x87::mul(x87::add(int_f(2 * i), c0), scale));
                tap(h, taps, axis, i.cast_unsigned(), pos, w2);
            }
            let last = taps.wrapping_add(16 * (n - 1).cast_unsigned() + 4 * axis);
            h.write(last, x87::store(x87::mul(int_f(n - 1), scale)));
            let edge = taps.wrapping_add(16 * (n - 1).cast_unsigned() + 8);
            h.write(edge, x87::store(x87::mul(frac, inv)));
            let fr1 = x87::add(frac, 1.0);
            let s = x87::add(x87::sub(1.0, nf), x87::div(1.0, fr1));
            h.write(taps.wrapping_add(4 * axis), x87::store(x87::mul(s, scale)));
            h.write(taps.wrapping_add(8), x87::store(x87::mul(inv, fr1)));
            let reach = (n - 1).cast_unsigned();
            let (dx, dy) = if axis == 0 { (reach, 0) } else { (0, reach) };
            widen(h, rect, target, dx, dy);
            let r = h.call_sym(
                BOX_PASS,
                Conv::Cdecl,
                &[api, target, obj, n.cast_unsigned(), taps, rect, out, c],
            );
            return finish(h, &a, r);
        }

        // the downsampling pass
        let t = r32(x87::add(x87::mul(x87::div(int_f(n - 10), 71.0), 7.0), 2.0));
        let up = r32(ceil(h, t));
        let p = x87::ftol(h, up).cast_signed();
        let reach_n = if p < 2 { 2 } else { p };
        let (mut taps1, mut sc) = (p, x87::load(x87::store(scale)));
        if p > 9 {
            sc = r32(x87::mul(scale, int_f(p / 9)));
            taps1 = 9;
        }
        let inv2 = r32(x87::div(1.0, int_f(taps1.wrapping_mul(2))));
        if taps1 > 0 {
            let w = x87::store(x87::mul(inv2, 2.0));
            let mut j = 1i32.wrapping_sub(taps1);
            for i in 0..taps1 {
                let pos = x87::store(x87::mul(x87::add(int_f(j), 0.5), sc));
                tap(h, taps, axis, i.cast_unsigned(), pos, w);
                j = j.wrapping_add(2);
            }
        }
        // taps1 is 2 or more here (n >= 10); the original's idiv would fault on 0
        let r = n.wrapping_mul(2).checked_div(taps1).unwrap_or(0);
        let e = ((r.wrapping_add(1) >> 1) + 1)
            .wrapping_mul(taps1)
            .wrapping_add(1)
            .cast_unsigned();
        let (dx, dy) = if axis == 0 { (e, 0) } else { (0, e) };
        widen(h, rect, target, dx, dy);
        let (tz, tw): (u32, u32) = (
            h.read(target.wrapping_add(8)),
            h.read(target.wrapping_add(12)),
        );
        let res = h.call_sym(
            AXIS_PASS,
            Conv::Cdecl,
            &[
                api,
                target,
                obj,
                taps1.cast_unsigned(),
                taps,
                rect,
                axis,
                reach_n.cast_unsigned(),
                tz,
                tw,
                out,
                c,
            ],
        );
        finish(h, &a, res);
        if h.read::<u32>(obj.wrapping_add(0x1c)) == 0 {
            return obj;
        }

        // the second axis pass over what is left
        let (left, sc2) = if r > 18 {
            let v = x87::div(x87::mul(int_f(r.wrapping_sub(1)), scale), 17.0);
            (18, r32(v))
        } else {
            (r, scale)
        };
        let w1 = r32(x87::div(1.0, int_f(left)));
        let half_n;
        if left & 1 != 0 {
            half_n = left.wrapping_add(1) >> 1;
            if half_n > 0 {
                let c1 = x87::sub(1.0, int_f(half_n));
                let c5 = x87::add(c1, 0.5);
                let w2 = x87::store(x87::mul(w1, 2.0));
                for i in 0..half_n {
                    let pos = x87::store(x87::mul(x87::add(int_f(2 * i), c5), sc2));
                    tap(h, taps, axis, i.cast_unsigned(), pos, w2);
                }
                let last = taps.wrapping_add(16 * (half_n - 1).cast_unsigned());
                let pos = x87::store(x87::mul(x87::add(c1, int_f(2 * half_n - 2)), sc2));
                h.write(last.wrapping_add(4 * axis), pos);
                h.write(last.wrapping_add(8), x87::store(w1));
            }
        } else {
            half_n = left >> 1;
            if half_n > 0 {
                let c1 = x87::sub(1.0, int_f(half_n));
                let w2 = x87::store(x87::mul(w1, 2.0));
                for i in 0..half_n {
                    let pos = x87::store(x87::mul(x87::add(int_f(2 * i), c1), sc2));
                    tap(h, taps, axis, i.cast_unsigned(), pos, w2);
                }
            }
        }
        let (tz, tw): (u32, u32) = (
            h.read(target.wrapping_add(8)),
            h.read(target.wrapping_add(12)),
        );
        let reach = reach_n.cast_unsigned();
        let (a8, a9) = match axis {
            1 => (tz, tw.wrapping_mul(reach)),
            0 => (tz.wrapping_mul(reach), tw),
            _ => (tz, tw),
        };
        let res = h.call_sym(
            AXIS_PASS,
            Conv::Cdecl,
            &[
                api,
                target,
                obj,
                half_n.cast_unsigned(),
                taps,
                rect,
                axis,
                1,
                a8,
                a9,
                out,
                c,
            ],
        );
        finish(h, &a, res)
    }
}
