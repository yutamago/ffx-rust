//! List `p5-core`, continuation round: `PMatrix4_lookAt`, the affine
//! inverse (`Vectormath_Transform3_inverse`) and `PMatrix4_Inverse`, the
//! wrapper that runs it on a 4x4 matrix.
//!
//! x87 at 53 bits (`ffx_bridge::x87`): every product and sum is computed
//! in `f64` in the original's order and rounded to `f32` where the
//! original stores (`fstp dword`); a value reloaded from such a store is
//! the `f32`. The statement order and operands were read off the machine
//! code with a symbolic tracer (`~/ffx-work/m4/port/p2c/x87sym.py`).

use ffx_bridge::Mem;
use ffx_bridge::x87::{add, div, load, mul, sqrt, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `PMatrix4_FromAxes_00450de0` (thiscall `(m; a, b, c, d)`).
pub const FROM_AXES: u32 = 0x0045_0de0;
/// `Vectormath_Transform3_inverse` (cdecl `(out, m)`).
pub const TRANSFORM3_INVERSE: u32 = 0x004c_c890;
/// `PMatrix4_SetAffine_004c9360` (thiscall `(out; m)`).
pub const SET_AFFINE: u32 = 0x004c_9360;

/// `fstp dword` of an `f64`: the `f32` value a reload gives.
fn f32s(x: f64) -> f64 {
    load(store(x))
}

/// `|v|^2` as the original sums it (`x*x` stored, `+ y*y` stored,
/// `+ z*z` stored), then `1 / sqrt` (each stored).
fn inverse_length(v: [f64; 3]) -> f64 {
    let s1 = f32s(mul(v[0], v[0]));
    let s2 = f32s(add(s1, mul(v[1], v[1])));
    let s3 = f32s(add(s2, mul(v[2], v[2])));
    let len = f32s(sqrt(s3));
    f32s(div(1.0, len))
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, FROM_AXES, Host, Mem, SET_AFFINE, TRANSFORM3_INVERSE, add, div, f32s, inverse_length,
        load, mul, store, sub,
    };

    fn vec3(m: &impl Mem, p: u32) -> [f64; 3] {
        [load(m.read(p)), load(m.read(p + 4)), load(m.read(p + 8))]
    }

    fn put3(m: &mut impl Mem, p: u32, v: [f64; 3]) {
        for (i, x) in (0u32..).zip(v) {
            m.write(p + 4 * i, store(x));
        }
    }

    /// `PMatrix4_lookAt(m; p, q, u)` (`0x00451c30`, thiscall, `ret 0xc`)
    /// -> `m`. `u` normalised (`n`), `f = (q - p)` normalised, `s =
    /// f x n` normalised, `t = s x f` normalised (each component a
    /// difference of two products, stored; each length as
    /// [`super::inverse_length`]); then `PMatrix4_FromAxes` into the
    /// frame's matrix with rows `s`, `t`, `-f` and the eye `p` (as
    /// loaded), and the matrix copied to `m` float by float.
    ///
    /// `frame` (0x94 bytes) is laid out as the original's (`ebp - 0x94`):
    /// `t` at 0, `-f` at 0x10, the matrix at 0x20, `s` at 0x70, `p` at
    /// 0x80.
    pub fn look_at(h: &mut impl Host, this: u32, p: u32, q: u32, u: u32, frame: u32) -> u32 {
        // n = u / |u| (the first and last products with the inverse in
        // the other order, as the original multiplies)
        let k = inverse_length(vec3(h, u));
        let uv = vec3(h, u);
        let n = [
            f32s(mul(uv[0], k)),
            f32s(mul(k, uv[1])),
            f32s(mul(k, uv[2])),
        ];
        let (pv, qv) = (vec3(h, p), vec3(h, q));
        let d = [
            f32s(sub(qv[0], pv[0])),
            f32s(sub(qv[1], pv[1])),
            f32s(sub(qv[2], pv[2])),
        ];
        let k = inverse_length(d);
        let f = [f32s(mul(d[0], k)), f32s(mul(d[1], k)), f32s(mul(k, d[2]))];
        let c = [
            f32s(sub(mul(f[1], n[2]), mul(f[2], n[1]))),
            f32s(sub(mul(f[2], n[0]), mul(n[2], f[0]))),
            f32s(sub(mul(n[1], f[0]), mul(f[1], n[0]))),
        ];
        let k = inverse_length(c);
        let s = [f32s(mul(k, c[0])), f32s(mul(k, c[1])), f32s(mul(k, c[2]))];
        let c = [
            f32s(sub(mul(s[1], f[2]), mul(s[2], f[1]))),
            f32s(sub(mul(s[2], f[0]), mul(f[2], s[0]))),
            f32s(sub(mul(f[1], s[0]), mul(s[1], f[0]))),
        ];
        let k = inverse_length(c);
        let t = [f32s(mul(c[0], k)), f32s(mul(c[1], k)), f32s(mul(k, c[2]))];
        put3(h, frame, t);
        // the fourth row: the eye `p` (read again, through fld/fstp)
        let pv = vec3(h, p);
        put3(h, frame + 0x80, pv);
        put3(h, frame + 0x10, [-f[0], -f[1], -f[2]]);
        put3(h, frame + 0x70, s);
        let mat = frame + 0x20;
        h.call_sym(
            FROM_AXES,
            Conv::Thiscall(mat),
            &[frame + 0x70, frame, frame + 0x10, frame + 0x80],
        );
        for i in 0..16 {
            let x: u32 = h.read(mat + 4 * i);
            h.write(this + 4 * i, store(load(x)));
        }
        this
    }

    /// `Vectormath_Transform3_inverse(out, m)` (`0x004cc890`, cdecl) ->
    /// `out`'s words 0-2, 4-6, 8-10, 12-14 (the rest untouched): the
    /// inverse of the affine matrix `m` (rows of 4 words, the 4th unused).
    /// Nine cofactors, each a difference of two products, stored;
    /// `det` summed in three stores; `1 / det` stored; each element a
    /// cofactor times it, stored; the translation `-(z * r2 + (y * r1 +
    /// x * r0))`, each product and sum stored. All of `m` is read before
    /// `out` is written. `eax`: `out` is not what the original leaves
    /// (its last load is `out`'s pointer): the shim returns it.
    pub fn transform3_inverse(m: &mut impl Mem, out: u32, src: u32) -> u32 {
        let x = |i: u32| load(m.read(src + 4 * i));
        let cof = |a: u32, b: u32, c: u32, d: u32| f32s(sub(mul(x(a), x(b)), mul(x(c), x(d))));
        let v1 = cof(10, 5, 6, 9);
        let v2 = cof(6, 8, 10, 4);
        let v3 = cof(9, 4, 8, 5);
        let v4 = cof(2, 9, 10, 1);
        let v5 = cof(10, 0, 2, 8);
        let v6 = cof(1, 8, 0, 9);
        let v7 = cof(6, 1, 2, 5);
        let v8 = cof(2, 4, 6, 0);
        let v9 = cof(5, 0, 4, 1);
        let d1 = f32s(mul(x(8), v7));
        let d2 = f32s(add(mul(x(9), v8), d1));
        let d3 = f32s(add(mul(x(10), v9), d2));
        let inv = f32s(div(1.0, d3));
        let rows = [
            [f32s(mul(v1, inv)), f32s(mul(v4, inv)), f32s(mul(v7, inv))],
            [f32s(mul(v2, inv)), f32s(mul(v5, inv)), f32s(mul(v8, inv))],
            [f32s(mul(v3, inv)), f32s(mul(v6, inv)), f32s(mul(inv, v9))],
        ];
        let mut tr = [0f64; 3];
        for (k, t) in tr.iter_mut().enumerate() {
            let a = f32s(mul(x(12), rows[0][k]));
            let b = f32s(mul(x(13), rows[1][k]));
            let s = f32s(add(b, a));
            let c = f32s(mul(x(14), rows[2][k]));
            *t = f32s(add(c, s));
        }
        for (r, row) in (0u32..).zip(rows) {
            put3(m, out + 0x10 * r, row);
        }
        put3(m, out + 0x30, [-tr[0], -tr[1], -tr[2]]);
        out
    }

    /// `PMatrix4_Inverse_004cc0c0(out, m)` (cdecl) -> `out`: `m`'s rows'
    /// first three floats (through `fld`/`fstp`) into a local 3x4 (the
    /// 4th words left as they were: the inverse never reads them),
    /// `Vectormath_Transform3_inverse` into a second local, then
    /// `PMatrix4_SetAffine(out; that local)`. `frame` (0x90 bytes) is
    /// laid out as the original's (`ebp - 0x90`): the inverse at 0, the
    /// copy at 0x40.
    pub fn inverse(h: &mut impl Host, out: u32, src: u32, frame: u32) -> u32 {
        let copy = frame + 0x40;
        for r in 0..4 {
            let v = vec3(h, src + 0x10 * r);
            put3(h, copy + 0x10 * r, v);
        }
        let inv = h.call_sym(TRANSFORM3_INVERSE, Conv::Cdecl, &[frame, copy]);
        h.call_sym(SET_AFFINE, Conv::Thiscall(out), &[inv]);
        out
    }
}
