//! The battle code's vector and matrix helpers (`docs/rewrite-plan.md`,
//! M2, "The PS2 emulation registers"): PS2 VU0 macro code the PC port runs
//! on the x87, through emulated VF registers in `.data` (16 bytes each,
//! x, y, z, w). They load their operands into those registers and leave
//! their intermediates there, so the registers are outputs too: the bridges
//! leave the same values in them.
//!
//! The arithmetic is the x87's at 53 bits ([`ffx_bridge::x87`]): `f64`,
//! rounded to `f32` where the original stores. Where it multiplies and adds
//! without a store between (`MsApplyMatrix`: `fld; fmul; fadd; fstp`), the
//! product is exact and only the sum is rounded, to 53 bits and then to
//! 24: `f32` arithmetic (`a * b + c`, two roundings to 24 bits) differs in
//! the last bit for some inputs, and a fused multiply-add (one rounding)
//! too. NaNs take the x87's operand (the larger significand), not SSE's.

use ffx_bridge::x87::{add, mul, store, sub};

/// A vector of four `f32` bit patterns.
pub type Vec4 = [u32; 4];

/// A 4×4 matrix, row by row.
pub type Mat4 = [Vec4; 4];

fn ld(bits: u32) -> f64 {
    ffx_bridge::x87::load(bits)
}

/// `v` times `s`, every component.
#[must_use]
pub fn scale(v: Vec4, s: f64) -> Vec4 {
    v.map(|x| store(mul(ld(x), s)))
}

/// `m · v` as `MsApplyMatrix` computes it: `v.x · m[0]`, then `+ v.y ·
/// m[1]` and so on, each sum stored as `f32`. Returns the result and the
/// accumulator after the third row (left in a register).
#[must_use]
pub fn apply(m: &Mat4, v: Vec4) -> (Vec4, Vec4) {
    let mut acc = m[0].map(|c| store(mul(ld(c), ld(v[0]))));
    for (row, &k) in m.iter().zip(&v).skip(1).take(2) {
        for (a, &c) in acc.iter_mut().zip(row) {
            *a = store(add(mul(ld(c), ld(k)), ld(*a)));
        }
    }
    let mut out = [0; 4];
    for ((o, &c), &a) in out.iter_mut().zip(&m[3]).zip(&acc) {
        *o = store(add(mul(ld(c), ld(v[3])), ld(a)));
    }
    (out, acc)
}

/// `MsNearRag(a, b)`: `a` moved to the angle nearest it that is `b`
/// modulo `two_pi`, within (-`pi`, `pi`] of it: `d = b − a` reduced
/// by whole turns (`ftol` truncates `d / two_pi`, as the caller's
/// `_ftol2_sse` does), made positive, then `a + d`, or `a + d − two_pi`
/// from `pi` on. The constants are the game's (2π and π rounded to `f32`).
#[must_use]
pub fn near_rag(a: u32, b: u32, two_pi: f64, pi: f64, ftol: impl FnOnce(f64) -> u32) -> u32 {
    let a = ld(a);
    let d = ld(store(sub(ld(b), a)));
    let turns = ftol(ffx_bridge::x87::div(d, two_pi)).cast_signed();
    #[allow(clippy::cast_precision_loss)] // fild, then fstp dword
    let nf = f64::from(turns as f32);
    let mut r = ld(store(sub(d, mul(nf, two_pi))));
    if r < 0.0 {
        r = ld(store(add(r, two_pi)));
    }
    if r < pi {
        store(add(a, r))
    } else {
        store(add(r, sub(a, two_pi)))
    }
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{Mat4, Vec4, apply, near_rag, scale};

    /// VU0 registers (`0x00c8f788` the scalar Q, then 16-byte VF registers
    /// from `0x00c8f790`) these functions use.
    pub const VU_Q: Global<u32> = Global::new(0x00c8_f788);
    /// `0x00c8f790`: `MsApplyMatrix`'s accumulator.
    pub const VF_ACC: Global<Vec4> = Global::new(0x00c8_f790);
    /// `0x00c8f7a0`, `0x00c8f7b0`: `MsMulVector`'s and `MsDivVector`'s
    /// operands.
    pub const VF_A: Global<Vec4> = Global::new(0x00c8_f7a0);
    pub const VF_B: Global<Vec4> = Global::new(0x00c8_f7b0);
    /// `0x00c8f7d0`-`0x00c8f80c`: a matrix (or `MsAddPoint`'s operands and
    /// result from `0x00c8f7d0`, `0x00c8f7e0`, `0x00c8f7f0`).
    pub const VF_M: Global<Mat4> = Global::new(0x00c8_f7d0);
    /// `0x00c8f810`: `MsApplyMatrix`'s vector.
    pub const VF_V: Global<Vec4> = Global::new(0x00c8_f810);
    /// `0x00c8f850`: `MsApplyMatrix`'s result.
    pub const VF_OUT: Global<Vec4> = Global::new(0x00c8_f850);
    /// `vf00` (`g_Vu0_Vf00`): (0, 0, 0, 1).
    pub const VF00: Global<Vec4> = Global::new(0x00c0_a004);
    /// The angle constants `MsNearRag` reads: 2π and π, as `f64` of their
    /// `f32` roundings.
    pub const TWO_PI: Global<f64> = Global::new(0x00b3_9378);
    pub const PI: Global<f64> = Global::new(0x00b9_2288);
    /// -1.0, `MsSetNormalUpVector`'s y.
    pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);

    /// `MsMulVector(v, s, b1, b2, b3)` (`0x007c2e10`): `v *= s`; the
    /// arguments go to [`VF_B`] as the VU code had them.
    pub fn ms_mul_vector(mem: &mut impl Mem, v: u32, args: Vec4) {
        let a: Vec4 = mem.read(v);
        VF_B.set(mem, args);
        let r = scale(a, ffx_bridge::x87::load(args[0]));
        VF_A.set(mem, r);
        mem.write(v, r);
    }

    /// `MsDivVector(v, s, b1, b2, b3)` (`0x007c2520`): `v *= 1 / s`
    /// (`vf00.w / s`, stored as `f32` in Q first).
    pub fn ms_div_vector(mem: &mut impl Mem, v: u32, args: Vec4) {
        use ffx_bridge::x87::{div, load, store};
        let a: Vec4 = mem.read(v);
        VF_B.set(mem, args);
        let q = store(div(load(VF00.get(mem)[3]), load(args[0])));
        VU_Q.set(mem, q);
        let r = scale(a, load(q));
        VF_A.set(mem, r);
        mem.write(v, r);
    }

    /// `MsAddPoint(out, a, b)` (`0x007c1480`): `out.xyz = a.xyz + b.xyz`;
    /// `out.w` is whatever the result register's w held.
    pub fn ms_add_point(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        use ffx_bridge::x87::{add, load, store};
        let va: Vec4 = mem.read(a);
        let vb: Vec4 = mem.read(b);
        let mut m = VF_M.get(mem);
        m[0] = va;
        m[1] = vb;
        for i in 0..3 {
            m[2][i] = store(add(load(va[i]), load(vb[i])));
        }
        VF_M.set(mem, m);
        mem.write(out, m[2]);
    }

    /// `MsSetNormalUpVector(v)` (`0x007c0ab0`): (0, -1, 0, 1), the up
    /// vector of a world whose y points down; nothing for null.
    pub fn ms_set_normal_up_vector(mem: &mut impl Mem, v: u32) {
        if v != 0 {
            let up = MINUS_ONE.get(mem);
            mem.write(v, [0, up, 0, 1.0f32.to_bits()]);
        }
    }

    /// `MsApplyMatrix(out, m, v)` (`0x007c1550`): `out = m · v`
    /// ([`apply`]), through the matrix registers.
    pub fn ms_apply_matrix(mem: &mut impl Mem, out: u32, m: u32, v: u32) {
        let mat: Mat4 = mem.read(m);
        VF_M.set(mem, mat);
        let vec: Vec4 = mem.read(v);
        VF_V.set(mem, vec);
        let (r, acc) = apply(&mat, vec);
        VF_ACC.set(mem, acc);
        VF_OUT.set(mem, r);
        mem.write(out, r);
    }

    /// `MsNearRag(a, b)` (`0x007c04c0`): [`near_rag`], an `f32` (on the x87
    /// stack).
    pub fn ms_near_rag(mem: &impl Mem, a: u32, b: u32) -> f32 {
        let r = near_rag(a, b, TWO_PI.get(mem), PI.get(mem), |x| {
            ffx_bridge::x87::ftol(mem, x)
        });
        f32::from_bits(r)
    }

    /// `0x00c8f7e0`, `0x00c8f7f0`, `0x00c8f800`: the registers after
    /// [`VF_M`]'s first row, as some helpers use them one by one.
    fn vf(mem: &impl Mem, i: u32) -> Vec4 {
        mem.read(VF_M.ptr(mem).addr + 16 * i)
    }

    fn set_vf(mem: &mut impl Mem, i: u32, v: Vec4) {
        let at = VF_M.ptr(mem).addr + 16 * i;
        mem.write(at, v);
    }

    /// `MsMulMatrix(out, a, b)` (`0x007c2770`): `out = b · a`, row `i`
    /// being `b[i]` applied to `a` as `MsApplyMatrix` does; `a` in
    /// [`VF_M`], `b` in the four registers from [`VF_V`], the rows in the
    /// four from [`VF_OUT`], the accumulator left from the last row.
    pub fn ms_mul_matrix(mem: &mut impl Mem, out: u32, a: u32, b: u32) {
        let ma: Mat4 = mem.read(a);
        let mb: Mat4 = mem.read(b);
        VF_M.set(mem, ma);
        let rows = mb.map(|row| apply(&ma, row));
        let at = VF_V.ptr(mem).addr;
        mem.write(at, mb);
        VF_ACC.set(mem, rows[3].1);
        let r = rows.map(|(r, _)| r);
        let at = VF_OUT.ptr(mem).addr;
        mem.write(at, r);
        mem.write(out, r);
    }

    /// `MsMiddlePoint(out, p, q, s, t, c, d, e)` (`0x007c25e0`): the point
    /// `(t·p + s·q) / (s + t)` between `p` and `q` (x, y, z; `out.w` is
    /// `p.w`). The divisor goes through `vf00.w / (s + t)` in Q; the float
    /// arguments are left in two registers as the VU code had them.
    pub fn ms_middle_point(mem: &mut impl Mem, out: u32, p: u32, q: u32, args: [u32; 5]) {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let [s, t] = [load(args[0]), load(args[1])];
        let sum = store(add(s, t));
        let mut vp: Vec4 = mem.read(p);
        let mut vq: Vec4 = mem.read(q);
        let at = VF_V.ptr(mem).addr;
        mem.write(at, sum);
        let recip = div(load(VF00.get(mem)[3]), load(sum));
        set_vf(mem, 2, [args[0], args[1], args[2], args[3]]);
        set_vf(mem, 3, [args[1], args[2], args[3], args[4]]);
        let qv = store(recip);
        VU_Q.set(mem, qv);
        vp[0] = store(mul(load(vp[0]), t));
        vp[1] = store(mul(load(vp[1]), t));
        vp[2] = store(mul(t, load(vp[2])));
        vq[0] = store(mul(load(vq[0]), s));
        vq[1] = store(mul(load(vq[1]), s));
        vq[2] = store(mul(s, load(vq[2])));
        for i in 0..3 {
            vp[i] = store(add(load(vq[i]), load(vp[i])));
        }
        let qf = load(qv);
        vp[0] = store(mul(load(vp[0]), qf));
        vp[1] = store(mul(load(vp[1]), qf));
        vp[2] = store(mul(qf, load(vp[2])));
        set_vf(mem, 0, vp);
        set_vf(mem, 1, vq);
        mem.write(out, vp);
    }

    /// The squares of `v`'s components in the second register, summed (x,
    /// then z, with y when `with_y`) into its x, the magnitude stored as
    /// `f32`. The registers as the VU code leaves them.
    fn square_sum(mem: &mut impl Mem, v: Vec4, with_y: bool) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        set_vf(mem, 0, v);
        let mut e = vf(mem, 1);
        e[0] = store(mul(load(v[0]), load(v[0])));
        if with_y {
            e[1] = store(mul(load(v[1]), load(v[1])));
        }
        e[2] = store(mul(load(v[2]), load(v[2])));
        if with_y {
            e[0] = store(add(load(e[1]), load(e[0])));
        }
        e[0] = store(add(load(e[2]), load(e[0])));
        set_vf(mem, 1, e);
        e[0] & 0x7fff_ffff
    }

    /// `vf00 + q` into the second register, its x returned (`MsVectorLen`'s
    /// tail).
    fn length_out(mem: &mut impl Mem, q: u32) -> f32 {
        use ffx_bridge::x87::{add, load, store};
        VU_Q.set(mem, q);
        let z = VF00.get(mem);
        let qf = load(q);
        let e = [
            store(add(load(z[0]), qf)),
            store(add(load(z[1]), qf)),
            store(add(load(z[2]), qf)),
            store(add(qf, load(z[3]))),
        ];
        set_vf(mem, 1, e);
        f32::from_bits(e[0])
    }

    /// `sqrt` as the game's `_CIsqrt` does it: the x87's `fsqrt` at 53
    /// bits (IEEE-exact, as `f64::sqrt`), stored as `f32`.
    fn sqrt(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits).sqrt())
    }

    /// `MsVectorLen(v)` (`0x007c4280`): `|v.xyz|`, through Q (an `f32`).
    pub fn ms_vector_len(mem: &mut impl Mem, v: u32) -> f32 {
        let vv: Vec4 = mem.read(v);
        let m = square_sum(mem, vv, true);
        length_out(mem, sqrt(m))
    }

    /// `MsVectorLen2D(v)` (`0x007c43a0`): `|v.xz|`.
    pub fn ms_vector_len_2d(mem: &mut impl Mem, v: u32) -> f32 {
        let vv: Vec4 = mem.read(v);
        let m = square_sum(mem, vv, false);
        length_out(mem, sqrt(m))
    }

    /// `MsVectorNormal(out, v)` (`0x007c44a0`): `v.xyz` scaled to length 1
    /// (by `vf00.w / |v|`, or 0 for a zero vector, in Q); `out.w = v.w`.
    pub fn ms_vector_normal(mem: &mut impl Mem, out: u32, v: u32) {
        use ffx_bridge::x87::{div, load, mul, store};
        let vv: Vec4 = mem.read(v);
        let m = square_sum(mem, vv, true);
        let q = if f32::from_bits(m) == 0.0 {
            0
        } else {
            store(div(load(VF00.get(mem)[3]), load(sqrt(m))))
        };
        VU_Q.set(mem, q);
        let qf = load(q);
        let mut d = vv;
        d[0] = store(mul(load(d[0]), qf));
        d[1] = store(mul(load(d[1]), qf));
        d[2] = store(mul(qf, load(d[2])));
        set_vf(mem, 0, d);
        mem.write(out, d);
    }
    /// `MsVectorGetSide(ax, az, bx, bz)` (`0x007c4180`): -1 when `b` lies
    /// to one side of `a` (the y of `a × b` negative: `ax·bz` above
    /// `az·bx`), else 0 (NaN too). The operands `(ax, 0, az, ?)` and
    /// `(bx, 0, bz, ?)` go to the first two matrix registers (their w is
    /// whatever the original's stack held: left as it was here), the
    /// products to [`VF_ACC`], the cross product to the third register.
    pub fn ms_vector_get_side(mem: &mut impl Mem, f: [u32; 4]) -> i32 {
        use ffx_bridge::x87::{load, mul, store, sub};
        let [ax, az, bx, bz] = f.map(load);
        let mut m = VF_M.get(mem);
        m[0] = [store(ax), 0, store(az), m[0][3]];
        m[1] = [store(bx), 0, store(bz), m[1][3]];
        let mut acc = VF_ACC.get(mem);
        acc[0] = store(mul(bz, 0.0));
        acc[1] = store(mul(az, bx));
        acc[2] = store(mul(ax, 0.0));
        m[2][0] = store(sub(load(acc[0]), mul(az, 0.0)));
        let y = store(sub(load(acc[1]), mul(ax, bz)));
        m[2][1] = y;
        m[2][2] = store(sub(load(acc[2]), mul(bx, 0.0)));
        VF_ACC.set(mem, acc);
        VF_M.set(mem, m);
        if 0.0 > f32::from_bits(y) { -1 } else { 0 }
    }
}
