//! M5 stage 1, list `phyre-core`, batch 7: `PGameplay`'s spline (a
//! cardinal spline over 16-byte control points).
//!
//! - the tangent at a point (half the neighbours' difference, normalised,
//!   scaled by `1 - tension`) and the cubic Hermite point between two
//!   control points;
//! - the segment evaluator (four control points from `0x0054b080`), its
//!   direction (the difference to a point 0.01 further on, normalised);
//! - the position and direction at a distance along the spline (the
//!   segment table at `+0x28`, the length at `+0x20`), the direction in
//!   world space and the position transformed by the world matrix;
//! - a vector's length.
//!
//! x87 at 53 bits: each function computes in f64 in the original's
//! order and rounds where the original stores (`fstp dword`); square
//! roots go through the CRT's `_CIsqrt`. Two of them leave values in
//! their own argument words (`0x00549a60`: `t + 0.01`; `0x0054b710` /
//! `0x0054b8f0`: `1 - fraction`); their callers are all ported and push
//! fresh words, so the ports keep them in locals.

/// The constants the originals read (doubles unless noted).
pub const HALF: u32 = 0x00b9_2220;
pub const THREE: u32 = 0x00b2_c5c8;
pub const MINUS_TWO: u32 = 0x00b2_c5d0;
pub const TWO: u32 = 0x00b9_2270;
pub const ONE: u32 = 0x00b9_2248;
pub const STEP: u32 = 0x00b1_f2b8;
/// `2^32` as a float (the unsigned `fild` fix-up).
pub const TWO_32: u32 = 0x00b1_1f14;
/// `_CIsqrt`'s import thunk (`ST0` in and out).
pub const CI_SQRT: u32 = 0x0094_97b8;
/// The spline's control point `0x0054b080(spline; out, i)` (thiscall,
/// 16 bytes out).
pub const CONTROL_POINT: u32 = 0x0054_b080;
/// `PMatrix4_FromMatrix3x4_00451080(out; m)` and
/// `PMatrix4_RowTimes_004c9c20(m; out, v)` (thiscall).
pub const FROM_3X4: u32 = 0x0045_1080;
pub const ROW_TIMES: u32 = 0x004c_9c20;

/// Frame bytes the evaluators need (the control points; the nested
/// evaluations' results after them).
pub const EVAL_FRAME: u32 = 0x40;
pub const DIRECTION_FRAME: u32 = EVAL_FRAME + 0x20;
pub const AT_FRAME: u32 = DIRECTION_FRAME + 0x20;
pub const WORLD_FRAME: u32 = AT_FRAME + 0x60;

/// What `0x0054b710` / `0x0054b8f0` evaluate per segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Along {
    /// The direction (`0x00549a60`).
    Direction,
    /// The position (`0x00549920`).
    Position,
}

/// `fistp qword` under round-toward-zero, low word: the truncation of
/// `x`, 0 for NaN or out of range (the integer indefinite's low half).
#[must_use]
pub fn chop(x: f64) -> u32 {
    if x.is_nan() || x.abs() >= 9_223_372_036_854_775_808.0 {
        0
    } else {
        (x.trunc() as i64) as u32
    }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn thiscall(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The float at `at` as `fld dword` loads it.
    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// The double at `at` (an image constant).
    fn ldd(h: &impl Host, sym: u32) -> f64 {
        let at = h.rebase(sym);
        let lo: u32 = h.read(at);
        let hi: u32 = h.read(at + 4);
        f64::from_bits(u64::from(hi) << 32 | u64::from(lo))
    }

    /// `x` through an `fstp dword`, as loaded back.
    fn r(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn st(h: &mut impl Host, at: u32, x: f64) {
        h.write(at, x87::store(x));
    }

    fn ld3(h: &impl Host, at: u32) -> [f64; 3] {
        [ld(h, at), ld(h, at + 4), ld(h, at + 8)]
    }

    fn st3(h: &mut impl Host, at: u32, v: [f64; 3]) {
        for (k, x) in v.into_iter().enumerate() {
            st(h, at + 4 * k as u32, x);
        }
    }

    /// `_CIsqrt(x)` stored as `f32`.
    fn sqrt(h: &mut impl Host, x: f64) -> f64 {
        let f = h.rebase(CI_SQRT);
        x87::load(h.call_intrinsic(f, x).to_bits())
    }

    /// The length of `d` as the originals compute it: each partial sum of
    /// squares stored as `f32`, then `_CIsqrt`.
    fn length(h: &mut impl Host, d: [f64; 3]) -> f64 {
        let s = r(x87::mul(d[0], d[0]));
        let s = r(x87::add(s, x87::mul(d[1], d[1])));
        let s = r(x87::add(s, x87::mul(d[2], d[2])));
        sqrt(h, s)
    }

    /// `b - a` per component, each stored as `f32`.
    fn diff(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        core::array::from_fn(|k| r(x87::sub(b[k], a[k])))
    }

    /// The tangent `0x00548980(out, &p0, &p1, &p2, tension)` -> `out`.
    pub fn tangent(h: &mut impl Host, out: u32, p0: u32, p1: u32, p2: u32, tension: u32) -> u32 {
        let v = tangent_of(h, ld3(h, p0), ld3(h, p1), ld3(h, p2), tension);
        st3(h, out, v);
        out
    }

    fn tangent_of(
        h: &mut impl Host,
        p0: [f64; 3],
        p1: [f64; 3],
        p2: [f64; 3],
        tension: u32,
    ) -> [f64; 3] {
        let a = diff(p1, p2);
        let b = diff(p0, p1);
        let half = ldd(h, HALF);
        let s: [f64; 3] = core::array::from_fn(|k| r(x87::mul(r(x87::add(b[k], a[k])), half)));
        let len = length(h, s);
        if len == 0.0 {
            return [0.0; 3];
        }
        let v: [f64; 3] = core::array::from_fn(|k| r(x87::div(s[k], len)));
        let w = r(x87::sub(1.0, x87::load(tension)));
        core::array::from_fn(|k| r(x87::mul(w, v[k])))
    }

    /// The Hermite point `0x005487d0(out, &p0, &p1, &p2, &p3, t,
    /// tension)` -> `out`: between `p1` and `p2`, with the tangents at
    /// `p1` and `p2` scaled by the segment's length.
    #[allow(clippy::too_many_arguments)] // the original's seven words
    pub fn hermite(
        h: &mut impl Host,
        out: u32,
        p0: u32,
        p1: u32,
        p2: u32,
        p3: u32,
        t: u32,
        tension: u32,
    ) -> u32 {
        let tf = x87::load(t);
        let b = r(x87::mul(tf, tf));
        let a = r(x87::mul(tf, b));
        let (q0, q1, q2, q3) = (ld3(h, p0), ld3(h, p1), ld3(h, p2), ld3(h, p3));
        let m1 = tangent_of(h, q0, q1, q2, tension);
        let m2 = tangent_of(h, q1, q2, q3, tension);
        let len = length(h, diff(q1, q2));
        let (three, minus_two) = (ldd(h, THREE), ldd(h, MINUS_TWO));
        let (two, one) = (ldd(h, TWO), ldd(h, ONE));
        let h3 = r(x87::mul(x87::sub(a, b), len));
        let three_b = x87::mul(three, b);
        let h2 = r(x87::add(x87::mul(a, minus_two), three_b));
        let h1 = r(x87::mul(len, x87::add(x87::sub(a, x87::mul(b, two)), tf)));
        let h0 = r(x87::add(x87::sub(x87::mul(a, two), three_b), one));
        let v: [f64; 3] = core::array::from_fn(|k| {
            let t0 = r(x87::mul(h0, q1[k]));
            let t1 = r(x87::mul(h1, m1[k]));
            let t2 = r(x87::mul(h2, q2[k]));
            let t3 = r(x87::mul(h3, m2[k]));
            r(x87::add(r(x87::add(r(x87::add(t0, t1)), t2)), t3))
        });
        st3(h, out, v);
        out
    }

    /// The segment evaluator `0x00549920(spline; out, i, t)` -> `out`:
    /// control points `i - 1 .. i + 2` (through `0x0054b080` into the
    /// frame at `frame`, [`EVAL_FRAME`] bytes), the Hermite point with
    /// the spline's tension (`+0x1c`).
    pub fn eval(h: &mut impl Host, this: u32, out: u32, i: u32, t: u32, frame: u32) -> u32 {
        for k in 0..4u32 {
            thiscall(
                h,
                CONTROL_POINT,
                this,
                &[frame + 16 * k, i.wrapping_sub(1).wrapping_add(k)],
            );
        }
        let tension = x87::store(ld(h, this + 0x1c));
        let t = x87::store(x87::load(t));
        hermite(
            h,
            out,
            frame,
            frame + 16,
            frame + 32,
            frame + 48,
            t,
            tension,
        )
    }

    /// The segment direction `0x00549a60(spline; out, i, t)` -> `out`: the
    /// normalised difference from the point at `t` to the one at `t +
    /// 0.01` ([`DIRECTION_FRAME`] bytes of frame).
    pub fn direction(h: &mut impl Host, this: u32, out: u32, i: u32, t: u32, frame: u32) -> u32 {
        let p = frame + EVAL_FRAME;
        let q = p + 0x10;
        eval(h, this, p, i, t, frame);
        let t2 = x87::store(x87::add(x87::load(t), ldd(h, STEP)));
        eval(h, this, q, i, t2, frame);
        let d = diff(ld3(h, p), ld3(h, q));
        let len = length(h, d);
        let inv = r(x87::div(1.0, len));
        let v: [f64; 3] = core::array::from_fn(|k| r(x87::mul(d[k], inv)));
        st3(h, out, v);
        out
    }

    /// One evaluation of `what`.
    fn eval_as(h: &mut impl Host, what: Along, this: u32, out: u32, i: u32, t: u32, frame: u32) {
        match what {
            Along::Direction => direction(h, this, out, i, t, frame),
            Along::Position => eval(h, this, out, i, t, frame),
        };
    }

    /// An unsigned word through `fild` and the 2^32 fix-up, stored as `f32`.
    fn unsigned(h: &impl Host, w: u32) -> f64 {
        let mut x = f64::from(w as i32);
        if (w as i32) < 0 {
            x = x87::add(x, ld(h, h.rebase(TWO_32)));
        }
        r(x)
    }

    /// `0x0054b710` / `0x0054b8f0(spline; out, distance)` -> `out`: the
    /// direction or position at `distance` along the spline: the segment
    /// table (`+0x28`, 28-byte entries: segment index, parameter) indexed
    /// by the distance's share of the length (`+0x20`) times the count
    /// (`+0x24`, less bit 31), the two neighbouring entries' results
    /// blended by the fraction ([`AT_FRAME`] bytes of frame).
    pub fn at(
        h: &mut impl Host,
        this: u32,
        out: u32,
        distance: u32,
        what: Along,
        frame: u32,
    ) -> u32 {
        let n = h.read::<u32>(this + 0x24) & 0x7fff_ffff;
        let length = ld(h, this + 0x20);
        let sub = frame + 0x20;
        if n == 0 {
            eval_as(h, what, this, out, 0, 0, sub);
            return out;
        }
        let q = r(x87::div(x87::load(distance), length));
        let x = r(x87::mul(q, unsigned(h, n)));
        let i = chop(x);
        let j = if i.wrapping_add(1) < n { i + 1 } else { i };
        let table: u32 = h.read(this + 0x28);
        let a = frame;
        let b = frame + 0x10;
        for (e, dst) in [(i, a), (j, b)] {
            let entry = table.wrapping_add(e.wrapping_mul(28));
            let seg: u32 = h.read(entry);
            let t = x87::store(ld(h, entry.wrapping_add(4)));
            eval_as(h, what, this, dst, seg, t, sub);
        }
        let fr = r(x87::sub(x, unsigned(h, i)));
        let bv = ld3(h, b);
        let av = ld3(h, a);
        let bfr: [f64; 3] = core::array::from_fn(|k| r(x87::mul(bv[k], fr)));
        let w = r(x87::sub(1.0, fr));
        let v: [f64; 3] = core::array::from_fn(|k| r(x87::add(r(x87::mul(w, av[k])), bfr[k])));
        st3(h, out, v);
        out
    }

    /// `0x0054b860(spline; out, distance)` -> `out`: the direction at
    /// `distance` turned by the world matrix (`[+0x14]` as a 4x4 in the
    /// frame; [`WORLD_FRAME`] bytes).
    pub fn world_direction(
        h: &mut impl Host,
        this: u32,
        out: u32,
        distance: u32,
        frame: u32,
    ) -> u32 {
        let m = frame;
        let v = frame + 0x40;
        let src: u32 = h.read(this + 0x14);
        thiscall(h, FROM_3X4, m, &[src]);
        at(h, this, v, distance, Along::Direction, frame + 0x50);
        let vv = ld3(h, v);
        let out3: [f64; 3] = core::array::from_fn(|c| {
            let c = c as u32;
            let s = x87::mul(ld(h, m + 4 * c), vv[0]);
            let s = x87::add(s, x87::mul(vv[1], ld(h, m + 0x10 + 4 * c)));
            r(x87::add(s, x87::mul(vv[2], ld(h, m + 0x20 + 4 * c))))
        });
        st3(h, out, out3);
        out
    }

    /// `0x0054ba40(spline; out, distance)` -> `out`: the position at
    /// `distance` as `(p, 1)` times the world matrix (`PMatrix4_RowTimes`
    /// on a copy of the 4x4 in the frame; [`WORLD_FRAME`] + 0x40 bytes).
    pub fn world_position(
        h: &mut impl Host,
        this: u32,
        out: u32,
        distance: u32,
        frame: u32,
    ) -> u32 {
        let m = frame;
        let copy = frame + 0x40;
        let p = frame + 0x80;
        let v = frame + 0x90;
        let res = frame + 0xa0;
        let src: u32 = h.read(this + 0x14);
        thiscall(h, FROM_3X4, m, &[src]);
        for w in 0..16 {
            let x: u32 = h.read(m + 4 * w);
            h.write(copy + 4 * w, x);
        }
        at(h, this, p, distance, Along::Position, frame + 0xb0);
        let pv = ld3(h, p);
        st3(h, v, pv);
        h.write(v + 12, 1.0f32.to_bits());
        let r4 = thiscall(h, ROW_TIMES, copy, &[res, v]);
        let rv = ld3(h, r4);
        st3(h, out, rv);
        out
    }

    /// `0x0054bd60(out, &v)` -> `out`: `[out]` = the length of `v`.
    pub fn length_of(h: &mut impl Host, out: u32, v: u32) -> u32 {
        let d = ld3(h, v);
        let len = length(h, d);
        st(h, out, len);
        out
    }
}
