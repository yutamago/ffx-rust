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
