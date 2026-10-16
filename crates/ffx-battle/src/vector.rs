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
}
