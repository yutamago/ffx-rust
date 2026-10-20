//! Quaternions, matrices and directions of the main module (`Sg_Quaternion*`,
//! `Sg_MatrixToQuaternion`, `Sg_CreateVectorFromHV`, `Sg_VectorToMatrix2`,
//! `sg_normalize`).
//!
//! Sines, cosines and arc cosines are the C runtime's (MSVCR110's `_CIsin`,
//! `_CIcos`, `_CIacos`, called with the angle in `ST0` and stored as `f32`
//! right after): the caller supplies them ([`Crt`]). Square roots are the
//! x87's `fsqrt`, which `_CIsqrt` is.

/// A C runtime function the originals call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crt {
    /// `_CIsin`.
    Sin,
    /// `_CIcos`.
    Cos,
    /// `_CIacos`.
    Acos,
    /// `_CIatan`.
    Atan,
}

/// The originals over game memory (`ffx_bridge::Mem`).
#[allow(clippy::many_single_char_names)] // quaternion and matrix components
pub mod bridge {
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, Mem};

    use super::Crt;
    use crate::scratch::{SCRATCH, Scratch};

    /// 1 (`f64` in `.rdata`), added before the square roots.
    pub const ONE: Global<f64> = Global::new(0x00b9_2248);
    /// 0.5 (`f64` in `.rdata`).
    pub const HALF: Global<f64> = Global::new(0x00b9_2220);
    /// `Sg_QuaternionSlerp`'s threshold for linear interpolation
    /// (`f64` 0.0001 in `.rdata`).
    pub const SLERP_EPSILON: Global<f64> = Global::new(0x00b5_aa38);
    /// π and 180 (`f64` in `.rdata`).
    pub const PI: Global<f64> = Global::new(0x00b5_82f0);
    /// See [`PI`].
    pub const DEG_180: Global<f64> = Global::new(0x00b4_8378);
    /// The up vector's y (`f32` -1 in `.rdata`, `Sg_VectorToMatrix2`).
    pub const UP_Y: Global<f32> = Global::new(0x00b9_2340);

    fn f(mem: &impl Mem, p: u32, k: u32) -> f32 {
        mem.read(p.wrapping_add(4 * k))
    }

    fn put(mem: &mut impl Mem, p: u32, k: u32, v: f32) {
        mem.write(p.wrapping_add(4 * k), v);
    }

    /// `sqrt` as `_CIsqrt` gives it (`fsqrt`, a NaN passed through).
    fn sqrt(x: Reg) -> Reg {
        if x.value().is_nan() {
            x
        } else {
            Reg::constant(x.value().sqrt())
        }
    }

    /// `Sg_QuaternionToMatrix(q, m)` (`0x00845450`): the rotation of the
    /// unit quaternion `(x, y, z, w)` as a 4×4 matrix (row vectors), each
    /// product stored as `f32` first, as the original does.
    pub fn quaternion_to_matrix(mem: &mut impl Mem, q: u32, m: u32) {
        let [x, y, z, w] = [0, 1, 2, 3].map(|k| f(mem, q, k));
        let twice = |v: f32| (Reg::load(v) + Reg::load(v)).store();
        let (x2, y2, z2) = (twice(x), twice(y), twice(z));
        let mul = |a: f32, b: f32| (Reg::load(a) * Reg::load(b)).store();
        let xx = mul(x, x2);
        let xy = mul(x, y2);
        let xz = mul(x, z2);
        let yy = mul(y, y2);
        let yz = mul(y, z2);
        let zz = mul(z, z2);
        let wx = mul(x2, w);
        let wy = Reg::load(y2).mul_m32(w).store();
        let wz = Reg::load(z2).mul_m32(w).store();
        let r = |v: f32| Reg::load(v);
        let one_minus = |a: Reg| (Reg::ONE - a).store();
        let out = [
            one_minus(r(zz).add_m32(yy)),
            (r(xy) + r(wz)).store(),
            (r(xz) - r(wy)).store(),
            0.0,
            (r(xy) - r(wz)).store(),
            one_minus(r(zz) + r(xx)),
            (r(wx) + r(yz)).store(),
            0.0,
            (r(wy) + r(xz)).store(),
            (r(yz) - r(wx)).store(),
            one_minus(r(xx) + r(yy)),
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        ];
        mem.write(m, out);
    }

    /// `Sg_MatrixToQuaternion(m, q)` (`0x00845170`): the rotation of `m`
    /// as a quaternion. With a positive trace, `w = sqrt(trace + 1) / 2`
    /// and the rest from the off-diagonal differences; otherwise from the
    /// largest diagonal element (a zero root leaves the scale 0).
    pub fn matrix_to_quaternion(mem: &mut impl Mem, m: u32, q: u32) {
        let one = Reg::constant(ONE.get(mem));
        let half = Reg::constant(HALF.get(mem));
        let trace = Reg::load(f(mem, m, 0))
            .add_m32(f(mem, m, 5))
            .add_m32(f(mem, m, 10))
            .store();
        if trace > 0.0 {
            let s = round_trip(sqrt(Reg::load(trace) + one).store());
            put(mem, q, 3, (Reg::load(s) * half).store());
            let t = Reg::load((half / Reg::load(s)).store());
            let a = (Reg::load(f(mem, m, 6)).sub_m32(f(mem, m, 9)) * t).store();
            put(mem, q, 0, a);
            let b = (Reg::load(f(mem, m, 8)).sub_m32(f(mem, m, 2)) * t).store();
            put(mem, q, 1, b);
            let c = (t * Reg::load(f(mem, m, 1)).sub_m32(f(mem, m, 4))).store();
            put(mem, q, 2, c);
            return;
        }
        let e: [f32; 16] = mem.read(m);
        let diag = |i: u32| e[(i * 5) as usize];
        let at = |r: u32, c: u32| e[(r * 4 + c) as usize];
        let mut i = 0u32;
        if diag(0) < diag(1) {
            i = 1;
        }
        if diag(i) < diag(2) {
            i = 2;
        }
        let next = [1u32, 2, 0];
        let j = next[i as usize];
        let k = next[j as usize];
        let d = Reg::load(diag(i)) - Reg::load(diag(k)).add_m32(diag(j));
        let s = round_trip(sqrt(d + one).store());
        let mut tmp = [0f32; 4];
        tmp[i as usize] = (Reg::load(s) * half).store();
        #[allow(clippy::float_cmp)] // the original's `fucomp` with 0
        let t = if s == 0.0 {
            s
        } else {
            (half / Reg::load(s)).store()
        };
        let t = Reg::load(t);
        tmp[3] = (Reg::load(at(j, k)).sub_m32(at(k, j)) * t).store();
        tmp[j as usize] = (Reg::load(at(j, i)).add_m32(at(i, j)) * t).store();
        tmp[k as usize] = (t * Reg::load(at(k, i)).add_m32(at(i, k))).store();
        mem.write(q, tmp.map(round_trip));
    }

    /// `Sg_QuaternionSlerp(a, b, t, out)` (`0x00845310`): spherical
    /// interpolation from `a` to `b` (negated when their dot product is
    /// negative), linear when they are within 0.0001 of each other;
    /// returns `out`.
    pub fn quaternion_slerp(
        mem: &mut impl Mem,
        a: u32,
        b: u32,
        t: f32,
        out: u32,
        crt: &mut impl FnMut(Crt, f64) -> f32,
    ) -> u32 {
        let qa: [f32; 4] = mem.read(a);
        let qb: [f32; 4] = mem.read(b);
        let mut dot = Reg::load(qa[0]).mul_m32(qb[0]);
        for k in 1..4 {
            dot = dot + Reg::load(qa[k]).mul_m32(qb[k]);
        }
        let mut dot = dot.store();
        let bb = if dot < 0.0 {
            dot = (-Reg::load(dot)).store();
            qb.map(|v| (-Reg::load(v)).store())
        } else {
            qb.map(round_trip)
        };
        let d = Reg::load(dot);
        let gap = Reg::ONE - d;
        let (s0, s1) = if gap.value() > SLERP_EPSILON.get(mem) {
            let om = round_trip(crt(Crt::Acos, d.value()));
            let so = round_trip(crt(Crt::Sin, f64::from(om)));
            let x0 = (Reg::ONE - Reg::load(t)).mul_m32(om);
            let s0 = Reg::load(crt(Crt::Sin, x0.value())).div_m32(so).store();
            let x1 = Reg::load(om).mul_m32(t);
            let s1 = Reg::load(crt(Crt::Sin, x1.value())).div_m32(so).store();
            (s0, s1)
        } else {
            ((Reg::ONE - Reg::load(t)).store(), round_trip(t))
        };
        let (s0, s1) = (Reg::load(s0), Reg::load(s1));
        let a0 = f(mem, a, 0);
        let r0 = Reg::load(a0) * s0 + s1.mul_m32(bb[0]);
        put(mem, out, 0, r0.store());
        let a1 = f(mem, a, 1);
        let r1 = Reg::load(bb[1]) * s1 + s0.mul_m32(a1);
        put(mem, out, 1, r1.store());
        let a2 = f(mem, a, 2);
        let r2 = Reg::load(a2) * s0 + Reg::load(bb[2]) * s1;
        put(mem, out, 2, r2.store());
        let a3 = f(mem, a, 3);
        let r3 = s1 * Reg::load(bb[3]) + s0.mul_m32(a3);
        put(mem, out, 3, r3.store());
        out
    }

    /// Degrees to radians as the originals compute them: `x * π / 180`,
    /// stored as `f32`.
    fn rad(mem: &impl Mem, x: f32) -> f32 {
        (Reg::load(x) * Reg::constant(PI.get(mem)) / Reg::constant(DEG_180.get(mem))).store()
    }

    /// `Sg_CreateVectorFromHV(out, h, v)` (`0x008450d0`): the direction at
    /// heading `h` and elevation `v` (degrees, each wrapped to ±π after
    /// conversion): `(sin h cos v, sin v, cos h cos v, 1)`.
    pub fn create_vector_from_hv(
        mem: &mut impl Mem,
        out: u32,
        h: f32,
        v: f32,
        crt: &mut impl FnMut(Crt, f64) -> f32,
    ) {
        let vw = crate::vec::bridge::angle_wrap(rad(mem, v));
        let sv = round_trip(crt(Crt::Sin, f64::from(vw)));
        put(mem, out, 1, round_trip(sv));
        let hw = crate::vec::bridge::angle_wrap(rad(mem, h));
        let cv = round_trip(crt(Crt::Cos, f64::from(vw)));
        let sh = round_trip(crt(Crt::Sin, f64::from(hw)));
        put(mem, out, 0, Reg::load(sh).mul_m32(cv).store());
        let ch = round_trip(crt(Crt::Cos, f64::from(hw)));
        put(mem, out, 2, Reg::load(ch).mul_m32(cv).store());
        put(mem, out, 3, 1.0);
    }

    /// `Sg_VectorToMatrix2(m, v)` (`0x00845560`): a unit matrix whose first
    /// three rows are a frame looking along `v`: `v` normalized, then
    /// `normalize(up × v) × v` normalized, then `up × v` normalized (up
    /// `(0, -1, 0)`); returns `FFXVu0UnitMatrix`'s result.
    pub fn vector_to_matrix2(mem: &mut impl Mem, m: u32, v: u32) -> u32 {
        use ffx_ps2::vu0::bridge as vu0;
        let up_y = UP_Y.get(mem);
        let src: [f32; 3] = mem.read(v);
        let mut s = Scratch::new(mem);
        // The original's locals: up at 0, the vector at 0x30, n at 0x20,
        // n1 at 0x10, n2 at 0x40.
        let (up, vec, n, n1, n2) = (
            SCRATCH,
            SCRATCH + 0x30,
            SCRATCH + 0x20,
            SCRATCH + 0x10,
            SCRATCH + 0x40,
        );
        s.write(up, [0.0f32, round_trip(up_y), 0.0]);
        let [x, y, z] = src.map(round_trip);
        s.write(vec, [x, y, z, 1.0]);
        vu0::normalize(&mut s, n, vec);
        crate::vec::bridge::cross(&mut s, vec, up, n);
        vu0::normalize(&mut s, n1, vec);
        crate::vec::bridge::cross(&mut s, vec, n1, n);
        vu0::normalize(&mut s, n2, vec);
        let rows: [[f32; 3]; 3] = [s.read(n), s.read(n2), s.read(n1)];
        let r = vu0::matrix_identity(&mut s, m);
        for (k, row) in (0u32..).zip(rows) {
            s.write(m.wrapping_add(16 * k), row.map(round_trip));
        }
        r
    }

    /// `sg_normalize(v, n)` (`0x00846710`): normalizes `n` (`i16`) 16-byte
    /// vectors in place (`FFXVu0Normalize`: `xyz` to length 1, `w` kept).
    pub fn normalize(mem: &mut impl Mem, v: u32, n: u32) {
        use ffx_ps2::vu0::bridge as vu0;
        let count = i32::from(crate::lo16(n).cast_signed());
        if count <= 0 {
            return;
        }
        let mut s = Scratch::new(mem);
        for i in 0..count.cast_unsigned() {
            let p = v.wrapping_add(16 * i);
            let w: [u32; 4] = s.read(p);
            s.write(SCRATCH + 0x10, w);
            vu0::normalize(&mut s, SCRATCH, SCRATCH + 0x10);
            let r: [u32; 4] = s.read(SCRATCH);
            s.write(p, r);
        }
    }
}
