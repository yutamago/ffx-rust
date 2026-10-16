//! `KeVu0*`: vector helpers written for the PS2's VU0 macro mode, run on
//! the emulated VU0 registers (`docs/rewrite-plan.md`, "The PS2 emulation
//! registers").
//!
//! The PC port keeps the VF registers in `.data` from `0x00c8f790`, 16
//! bytes each (x, y, z, w). These functions load their operands into the
//! registers below, compute there and copy the result out, so the
//! registers are outputs too: a replacement leaves the same values in
//! them.

use ffx_bridge::Global;

use crate::x87;

/// The VU0 register at `0x00c8f7a0`: the first operand.
pub const VF_A: Global<[u32; 4]> = Global::new(0x00c8_f7a0);
/// The VU0 register at `0x00c8f7b0`: the second operand.
pub const VF_B: Global<[u32; 4]> = Global::new(0x00c8_f7b0);
/// The VU0 register at `0x00c8f7c0`: `KeVu0AddVecXYZ`'s result.
pub const VF_C: Global<[u32; 4]> = Global::new(0x00c8_f7c0);
/// The VU0 register at `0x00c8f7d0`: `KeVu0CopyVecXYZ`'s source.
pub const VF_D: Global<[u32; 4]> = Global::new(0x00c8_f7d0);
/// The three words `KeMdl_MkTkClut` stores (the third a float).
pub const TK_CLUT: Global<[u32; 3]> = Global::new(0x010d_5600);

/// `dst` with x, y, z from `src` (through the x87 stack), w kept.
#[must_use]
pub fn copy_xyz(dst: [u32; 4], src: [u32; 4]) -> [u32; 4] {
    [
        x87::copy(src[0]),
        x87::copy(src[1]),
        x87::copy(src[2]),
        dst[3],
    ]
}

/// `dst` with x, y, z = `a + b`, w kept.
#[must_use]
pub fn add_xyz(dst: [u32; 4], a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let s = |i: usize| x87::store(x87::load(a[i]) + x87::load(b[i]));
    [s(0), s(1), s(2), dst[3]]
}

/// `v` with x, y, z times `s`, w kept.
#[must_use]
pub fn scale_xyz(v: [u32; 4], s: u32) -> [u32; 4] {
    let m = |i: usize| x87::store(x87::load(v[i]) * x87::load(s));
    [m(0), m(1), m(2), v[3]]
}

/// `sqrt(x² + y²)` at 53 bits, as `f32` bits.
#[must_use]
pub fn length2(x: u32, y: u32) -> u32 {
    let (x, y) = (x87::load(x), x87::load(y));
    x87::store((x * x + y * y).sqrt())
}

/// The VU0 helpers where the game keeps their state, through
/// The emulated R5900's `r0` (`g_Ee_Gpr`).
pub const EE_GPR: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c8_f508);
/// The emulated VU0 R register (`g_Vu0_R`), `f32` bits.
pub const VU0_R: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c0_a000);
/// Non-zero once R is seeded (`g_Vu0_RSeeded`).
pub const VU0_R_SEEDED: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00c8_f728);
/// The scale `KeVu0RndI` applies before converting (an `f64` in `.rdata`).
pub const RND_SCALE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_1588);

/// The emulated VU0 register (`vf31`, 16 bytes) the `rcCopyQword_*`
/// copies go through.
pub const VU0_COPY_REG: u32 = 0x00c8_f980;
/// The emulated VU0 register the `VFX_ScaleVec3` callers pass their
/// scale through (`0x00c8f970`).
pub const VU0_SCALE: u32 = 0x00c8_f970;
/// The emulated VU0 `vf00` (`0x00c0a004`).
pub const VU0_VF00: u32 = 0x00c0_a004;
/// 2π as the `f32` value widened (`.rdata` double, `0x00b39378`).
pub const TWO_PI: u32 = 0x00b3_9378;
/// `_CIfmod`'s import thunk (`0x009498e0`): `fmod(ST1, ST0)` into ST0.
pub const CI_FMOD: u32 = 0x0094_98e0;
/// `_CIacos`'s import thunk (`0x009498da`).
pub const CI_ACOS: u32 = 0x0094_98da;
/// `_CIsin`'s import thunk (`0x009498c8`).
pub const CI_SIN: u32 = 0x0094_98c8;
/// `_CIcos`'s import thunk (`0x009498c2`).
pub const CI_COS: u32 = 0x0094_98c2;
/// 100.0 in `.rdata` (`0x00b304f8`), the matrix row scale.
pub const SCALE_100: u32 = 0x00b3_04f8;
/// 10.0 in `.rdata` (`0x00b92318`).
pub const SCALE_10: u32 = 0x00b9_2318;
/// The field scale of effect vectors (`.rdata` float, `0x00b67e24`).
pub const FIELD_SCALE: u32 = 0x00b6_7e24;
/// 1024.0 in `.rdata` (`0x00b51ba8`, a double).
pub const FIELD_Y_OFFSET: u32 = 0x00b5_1ba8;

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{TK_CLUT, VF_A, VF_B, VF_C, VF_D, add_xyz, copy_xyz, length2, scale_xyz};
    use crate::{geom, x87};
    use ffx_bridge::x87::{Reg, ftol};

    /// `KeVu0CopyVecXYZ(dst, src)` (`0x0075ede0`): x, y, z of `src` into
    /// `dst`.
    pub fn ke_vu0_copy_vec_xyz(mem: &mut impl Mem, dst: u32, src: u32) {
        let d = mem.read::<[u32; 4]>(dst);
        VF_A.set(mem, d);
        let s = mem.read::<[u32; 4]>(src);
        VF_D.set(mem, s);
        let r = copy_xyz(d, s);
        VF_A.set(mem, r);
        mem.write(dst, r);
    }

    /// `KeVu0AddVecXYZ(dst, a, b)` (`0x0075eca0`): `dst.xyz = a.xyz + b.xyz`.
    pub fn ke_vu0_add_vec_xyz(mem: &mut impl Mem, dst: u32, a: u32, b: u32) {
        let va = mem.read::<[u32; 4]>(a);
        VF_A.set(mem, va);
        let vb = mem.read::<[u32; 4]>(b);
        VF_B.set(mem, vb);
        let d = mem.read::<[u32; 4]>(dst);
        let r = add_xyz(d, va, vb);
        VF_C.set(mem, r);
        mem.write(dst, r);
    }

    /// `KeVu0ScaleVecXYZ(dst, src, s)` (`0x0075fb50`), `s` a vector passed
    /// by value: `dst.xyz = src.xyz * s.x`, `dst.w = src.w`.
    pub fn ke_vu0_scale_vec_xyz(mem: &mut impl Mem, dst: u32, src: u32, s: [u32; 4]) {
        VF_A.set(mem, s);
        let v = mem.read::<[u32; 4]>(src);
        let r = scale_xyz(v, s[0]);
        VF_B.set(mem, r);
        mem.write(dst, r);
    }

    /// `KeVu0VecLen(v)` (`0x0075fda0`): the length of x, y, z.
    pub fn ke_vu0_vec_len(mem: &impl Mem, v: u32) -> f32 {
        f32::from_bits(geom::length3(mem.read(v)))
    }

    /// `KeVu0VecLenXY(v)` (`0x0075fde0`): the length of x, y.
    pub fn ke_vu0_vec_len_xy(mem: &impl Mem, v: u32) -> f32 {
        let [x, y] = mem.read::<[u32; 2]>(v);
        f32::from_bits(length2(x, y))
    }

    /// `KeVu0Sqrt(x)` (`0x0075fbf0`), `x` as bits.
    #[must_use]
    pub fn ke_vu0_sqrt(x: u32) -> f32 {
        f32::from_bits(x87::store(x87::load(x).sqrt()))
    }

    /// `KeMdl_MkTkClut(a, b, c)` (`0x00767530`): stores the three (`c` a
    /// float, through the x87 stack).
    pub fn ke_mdl_mk_tk_clut(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
        TK_CLUT.set(mem, [a, b, x87::copy(c)]);
    }

    /// `vrnext` on the emulated VU0 R register: the low 22 fraction bits
    /// shifted up with bit 4 of `R ^ (R >> 18)` in, exponent of 1.0.
    #[must_use]
    pub const fn vrnext(r: u32) -> u32 {
        (((r >> 18) ^ r) >> 4 & 1) | ((r & 0x003f_ffff) | 0x1fc0_0000).wrapping_shl(1)
    }

    /// One half of `KeVu0RndI`: `rN = r0 - 1` into the move register at
    /// `gpr` and `vf` (its high words from `src`), R ^= the low 23 bits of
    /// that word read as a float, `srand(R & 0x7fffff)`, then `vf.x` the
    /// value as a float. Returns the R-derived term: 1.0 until R is
    /// seeded, else the next R as a float.
    fn rnd_half<M: Mem>(
        mem: &mut M,
        gpr: u32,
        vf: u32,
        src: u32,
        srand: &mut impl FnMut(&mut M, u32),
    ) -> f32 {
        let (gpr, vf, src) = (mem.rebase(gpr), mem.rebase(vf), mem.rebase(src));
        let r0 = super::EE_GPR.get(mem);
        let a = r0.wrapping_sub(1);
        let sign = (a.cast_signed() >> 31).cast_unsigned();
        let r = super::VU0_R.ptr(mem).addr;
        let old = mem.read::<u32>(r);
        mem.write(gpr, a);
        mem.write(vf, a);
        let hi = mem.read::<[u32; 2]>(src);
        mem.write(vf.wrapping_add(8), hi);
        let bits = x87::copy(a);
        let nr = old ^ (bits & 0x007f_ffff);
        mem.write(r, nr);
        mem.write(gpr.wrapping_add(4), sign);
        mem.write(vf.wrapping_add(4), sign);
        srand(mem, nr & 0x007f_ffff);
        let v = mem.read::<i32>(vf);
        mem.write(vf, x87::store(f64::from(v)));
        if super::VU0_R_SEEDED.get(mem) == 0 {
            return 1.0;
        }
        let cur = mem.read::<u32>(r);
        let next = vrnext(cur);
        mem.write(r, next);
        f32::from_bits(next)
    }

    /// `KeVu0RndI()` (`0x0075f920`): two VU0-style random steps through
    /// the emulated registers (each `srand`s the CRT, `srand`), leaving
    /// `vf.x + R` at `0x00c8f7a0` and `0x00c8f970`; returns the second
    /// times an `f64` in `.rdata` (`0x00b51588`) through `_ftol2_sse`.
    pub fn ke_vu0_rnd_i<M: Mem>(mem: &mut M, mut srand: impl FnMut(&mut M, u32)) -> u32 {
        let t1 = rnd_half(mem, 0x00c8_f588, 0x00c8_f7b0, 0x00c8_f590, &mut srand);
        let x = mem.read::<f32>(mem.rebase(0x00c8_f7b0));
        let s1 = Reg::load(x).add_m32(t1).store();
        mem.write(mem.rebase(0x00c8_f7a0), s1);
        let t2 = rnd_half(mem, 0x00c8_f528, 0x00c8_f980, 0x00c8_f530, &mut srand);
        let y = mem.read::<f32>(mem.rebase(0x00c8_f980));
        let s2 = Reg::load(t2).add_m32(y).store();
        mem.write(mem.rebase(0x00c8_f970), s2);
        let k = super::RND_SCALE.get(mem);
        let v = Reg::load(f32::from_bits(x87::copy(s2.to_bits()))) * Reg::constant(k);
        ftol(mem, v.value())
    }

    /// The quadword copies through the emulated VU0 register at
    /// [`super::VU0_COPY_REG`] (`lqc2`/`sqc2`: `VFX_Copy16From` the source
    /// into it, `VFX_Copy16To` it to the destination), thirteen copies of
    /// `(dst, src)` in the map module (`rcCopyQword_*`). Returns the last
    /// word.
    pub fn rc_copy_qword(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        for i in 0..4u32 {
            let w = mem.read::<u32>(src.wrapping_add(4 * i));
            mem.write(vf + 4 * i, w);
        }
        let mut w = 0;
        for i in 0..4u32 {
            w = mem.read::<u32>(vf + 4 * i);
            mem.write(dst.wrapping_add(4 * i), w);
        }
        w
    }

    /// `rcbgScaleVector(dst, v, s)` (`0x009200a0`) and its twin
    /// `rcbgModelScaleVector` (`0x00927b00`): `v` through
    /// [`super::VU0_COPY_REG`], its x, y, z times `s` (`VFX_ScaleVec3`,
    /// z with the element as the memory operand), the quadword out to `dst`;
    /// returns the last word copied (`v`'s w). The originals also copy their
    /// argument area from `s` on (16 bytes, three of them the caller's stack)
    /// to `0x00c8f970`; only `s` is written there.
    pub fn rcbg_scale_vector(mem: &mut impl Mem, dst: u32, v: u32, s: u32) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        for i in 0..4u32 {
            let w = mem.read::<u32>(v.wrapping_add(4 * i));
            mem.write(vf + 4 * i, w);
        }
        mem.write(mem.rebase(super::VU0_SCALE), s);
        let k = Reg::load(f32::from_bits(s));
        for i in 0..2u32 {
            let e: f32 = mem.read(vf + 4 * i);
            mem.write(vf + 4 * i, (Reg::load(e) * k).store());
        }
        let z: f32 = mem.read(vf + 8);
        mem.write(vf + 8, k.mul_m32(z).store());
        let mut w = 0;
        for i in 0..4u32 {
            w = mem.read::<u32>(vf + 4 * i);
            mem.write(dst.wrapping_add(4 * i), w);
        }
        w
    }

    /// `rcbgModelVectorToInt(dst, v)` (`0x00927b60`): `v` through
    /// [`super::VU0_COPY_REG`], each of its four floats truncated to an
    /// integer (`0x00808b00`: `_ftol2_sse` per element, the source read
    /// first), the quadword out to `dst`; returns the last word.
    pub fn rcbg_model_vector_to_int(mem: &mut impl Mem, dst: u32, v: u32) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        let src = mem.read::<[f32; 4]>(v);
        for (i, e) in (0u32..).zip(src) {
            mem.write(vf + 4 * i, e);
        }
        for (i, e) in (0u32..).zip(src) {
            let n = ftol(mem, Reg::load(e).value());
            mem.write(vf + 4 * i, n);
        }
        let mut w = 0;
        for i in 0..4u32 {
            w = mem.read::<u32>(vf + 4 * i);
            mem.write(dst.wrapping_add(4 * i), w);
        }
        w
    }

    /// `rcbgScaleVector4_*(dst, v, s)` (`0x0091cb70`, `0x00925040`): as
    /// [`rcbg_scale_vector`], all four elements times `s`
    /// (`pppVu0MulX_007500d0`: w with the element as the memory operand).
    pub fn rcbg_scale_vector4(mem: &mut impl Mem, dst: u32, v: u32, s: u32) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        for i in 0..4u32 {
            let w = mem.read::<u32>(v.wrapping_add(4 * i));
            mem.write(vf + 4 * i, w);
        }
        mem.write(mem.rebase(super::VU0_SCALE), s);
        let k = Reg::load(f32::from_bits(s));
        for i in 0..3u32 {
            let e: f32 = mem.read(vf + 4 * i);
            mem.write(vf + 4 * i, (Reg::load(e) * k).store());
        }
        let w: f32 = mem.read(vf + 0xc);
        mem.write(vf + 0xc, k.mul_m32(w).store());
        let mut last = 0;
        for i in 0..4u32 {
            last = mem.read::<u32>(vf + 4 * i);
            mem.write(dst.wrapping_add(4 * i), last);
        }
        last
    }

    /// `rcVectorToFixed_*(dst, v)` (`0x009273e0`, `0x00927ac0`): `v`
    /// through [`super::VU0_COPY_REG`] as 12.4 fixed point
    /// (`rcVec4fToFixed` in place), out to `dst`; returns the last word.
    pub fn rc_vector_to_fixed(mem: &mut impl Mem, dst: u32, v: u32) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        for i in 0..4u32 {
            let w = mem.read::<u32>(v.wrapping_add(4 * i));
            mem.write(vf + 4 * i, w);
        }
        crate::model::bridge::rc_vec4f_to_fixed(mem, vf, vf);
        let mut last = 0;
        for i in 0..4u32 {
            last = mem.read::<u32>(vf + 4 * i);
            mem.write(dst.wrapping_add(4 * i), last);
        }
        last
    }

    /// The three rows of a matrix scaled in place by the `.rdata` float at
    /// `scale` ([`rcbg_scale_vector`] each): `rcbgScaleMatrix100`
    /// (`0x0091ffd0`) and `rcefScaleMatrix100` (`0x0092e630`) with 100.0
    /// (`0x00b304f8`).
    pub fn rcbg_scale_rows(mem: &mut impl Mem, m: u32, scale: u32) {
        let s = crate::x87::copy(mem.read::<u32>(mem.rebase(scale)));
        for r in 0..3u32 {
            rcbg_scale_vector(mem, m + 0x10 * r, m + 0x10 * r, s);
        }
    }

    /// `rcefVecToField(v)` (`0x0092d7e0`): `v` scaled by the `.rdata`
    /// float at `0x00b67e24` ([`rcbg_scale_vector`]), then 1024 off its y.
    pub fn rcef_vec_to_field(mem: &mut impl Mem, v: u32) {
        let s = crate::x87::copy(mem.read::<u32>(mem.rebase(super::FIELD_SCALE)));
        rcbg_scale_vector(mem, v, v, s);
        let y: f32 = mem.read(v + 4);
        let k = Reg::constant(mem.read::<f64>(mem.rebase(super::FIELD_Y_OFFSET)));
        mem.write(v + 4, (Reg::load(y) - k).store());
    }

    /// `rcuiScaleVector(dst, v, s, a, b, c)` (`0x00930f30`): an inlined
    /// `VFX_ScaleVec3` through [`super::VU0_COPY_REG`]: `v`'s x, y, z
    /// times `s` (z with the element as the memory operand), its w kept;
    /// `s, a, b, c` to [`super::VU0_SCALE`]; out to `dst`. Returns `v`'s w.
    pub fn rcui_scale_vector(mem: &mut impl Mem, dst: u32, v: u32, s: u32, rest: [u32; 3]) -> u32 {
        let vf = mem.rebase(super::VU0_COPY_REG);
        let e = mem.read::<[f32; 4]>(v);
        let k = Reg::load(f32::from_bits(s));
        let x = (Reg::load(e[0]) * k).store();
        let y = (Reg::load(e[1]) * k).store();
        let z = k.mul_m32(e[2]).store();
        mem.write(vf, [x, y, z, e[3]]);
        let sc = mem.rebase(super::VU0_SCALE);
        mem.write(sc, [s, rest[0], rest[1], rest[2]]);
        mem.write(dst, [x, y, z, e[3]]);
        e[3].to_bits()
    }

    /// `rcefGetVf00(dst)` (`0x00925020`): the emulated VU0 `vf00`
    /// (`0x00c0a004`, `VFX_Copy16To`) to `dst`; returns its w.
    pub fn rcef_get_vf00(mem: &mut impl Mem, dst: u32) -> u32 {
        let v = mem.read::<[u32; 4]>(mem.rebase(super::VU0_VF00));
        mem.write(dst, v);
        v[3]
    }

    /// `rcefAngleWrap(a)` (`0x0092d810`): an angle into `0..=2π` (the
    /// `.rdata` double at `0x00b39378`): negative ones as `2π - fmod(-a,
    /// 2π)`, ones past 2π as `fmod(a, 2π)` (`_CIfmod`, ST1 and ST0, through
    /// `fcall`), both stored as `f32`; the rest (NaN too) unchanged.
    pub fn rcef_angle_wrap<M: Mem>(
        mem: &mut M,
        a: u32,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) -> f32 {
        let x = Reg::load(f32::from_bits(a));
        let two_pi = mem.read::<f64>(mem.rebase(super::TWO_PI));
        let f = mem.rebase(super::CI_FMOD);
        if x.value() < 0.0 {
            let r = fcall(mem, f, crate::camera::FArgs::St01((-x).value(), two_pi));
            return (Reg::constant(two_pi) - Reg::constant(r)).store();
        }
        if two_pi < x.value() {
            let r = fcall(mem, f, crate::camera::FArgs::St01(x.value(), two_pi));
            return Reg::constant(r).store();
        }
        f32::from_bits(crate::x87::copy(a))
    }

    /// `rcefVecToAngle(v)` (`0x0092ec20`): `acos(-v.z)` (`_CIacos`, through
    /// `fcall`) stored as `f32`, `2π` minus it when `v.x > 0`.
    pub fn rcef_vec_to_angle<M: Mem>(
        mem: &mut M,
        v: u32,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) -> f32 {
        let z = Reg::load(mem.read::<f32>(v + 8));
        let f = mem.rebase(super::CI_ACOS);
        let a = Reg::constant(fcall(mem, f, crate::camera::FArgs::St0((-z).value()))).store();
        let x = mem.read::<f32>(v);
        if 0.0 < x {
            let two_pi = Reg::constant(mem.read::<f64>(mem.rebase(super::TWO_PI)));
            return (two_pi - Reg::load(a)).store();
        }
        a
    }

    /// `rcefAngleToDir(out, a)` (`0x0092ded0`): `(-sin a, 0, -cos a, 0)`
    /// (`_CIsin`, `_CIcos` through `fcall`, each rounded to `f32` first).
    pub fn rcef_angle_to_dir<M: Mem>(
        mem: &mut M,
        out: u32,
        a: u32,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) {
        let x = Reg::load(f32::from_bits(a)).value();
        let f = mem.rebase(super::CI_SIN);
        let s = Reg::constant(fcall(mem, f, crate::camera::FArgs::St0(x))).store();
        mem.write(out, (-Reg::load(s)).store());
        mem.write(out + 4, 0f32);
        let f = mem.rebase(super::CI_COS);
        let c = Reg::constant(fcall(mem, f, crate::camera::FArgs::St0(x))).store();
        mem.write(out + 8, (-Reg::load(c)).store());
        mem.write(out + 0xc, 0f32);
    }

    // ---- round 12 ----

    /// `KeVu0ScaleVecXY(dst, src, s)` (`0x0075fab0`): `dst.x` set to `s`
    /// first (the VU0 register A takes `dst` with it), then
    /// `dst = (src.x * s, s * src.y, src.z, src.w)` through register B.
    /// Returns `src.w`.
    pub fn ke_vu0_scale_vec_xy(mem: &mut impl Mem, dst: u32, src: u32, s: u32) -> u32 {
        mem.write(dst, x87::copy(s));
        let a = mem.read::<[u32; 4]>(dst);
        VF_A.set(mem, a);
        let mut b = mem.read::<[u32; 4]>(src);
        VF_B.set(mem, b);
        b[0] = x87::store(x87::load(b[0]) * x87::load(a[0]));
        b[1] = x87::store(x87::load(a[0]) * x87::load(b[1]));
        VF_B.set(mem, b);
        mem.write(dst, b);
        b[3]
    }

    /// `KeVu0SubVecXYZ(dst, a, b)` (`0x0075fcd0`): `dst.xyz = a.xyz -
    /// b.xyz`, `dst.w` kept, through registers A, B and C; restores
    /// `vf00` before the store. Returns `dst.w`.
    pub fn ke_vu0_sub_vec_xyz(mem: &mut impl Mem, dst: u32, a: u32, b: u32) -> u32 {
        let va = mem.read::<[u32; 4]>(a);
        VF_A.set(mem, va);
        let vb = mem.read::<[u32; 4]>(b);
        VF_B.set(mem, vb);
        let d = mem.read::<[u32; 4]>(dst);
        let s = |i: usize| x87::store(x87::load(va[i]) - x87::load(vb[i]));
        let r = [s(0), s(1), s(2), d[3]];
        VF_C.set(mem, r);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let r = VF_C.get(mem);
        mem.write(dst, r);
        r[3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w_is_kept() {
        let b = |x: f32| x.to_bits();
        let d = [b(9.0), b(9.0), b(9.0), b(7.0)];
        let s = [b(1.0), b(2.0), b(3.0), b(4.0)];
        assert_eq!(copy_xyz(d, s), [b(1.0), b(2.0), b(3.0), b(7.0)]);
        assert_eq!(add_xyz(d, s, s), [b(2.0), b(4.0), b(6.0), b(7.0)]);
        assert_eq!(scale_xyz(s, b(2.0)), [b(2.0), b(4.0), b(6.0), b(4.0)]);
        assert_eq!(length2(b(3.0), b(4.0)), b(5.0));
    }
}
