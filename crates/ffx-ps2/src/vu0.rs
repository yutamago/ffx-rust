//! The VU0 vector and matrix library the PC port rewrote in C
//! (`FFXVu0*`, the `sceVu0` replacements, `Matrix::copy`), with the game's
//! x87 rounding ([`ffx_bridge::x87`]).
//!
//! Vectors are four `f32`s, matrices sixteen, row-major (`ffx_system::math`
//! has the layout). Each function is followed as compiled: the order it
//! reads and writes memory (so overlapping arguments behave the same),
//! which operand of each operation it takes from memory (a signalling NaN
//! there loses to a quiet NaN in the register), and what it leaves in
//! `eax`, which the bridges return because Ghidra's "void" functions here
//! leave a value callers may read.

// a, b, t, u as in the formulas
#![allow(clippy::many_single_char_names)]

use ffx_bridge::x87::Reg;

fn ld(x: f32) -> Reg {
    Reg::load(x)
}

/// `FFXVu0AddVector` component: `a + [b]`.
#[must_use]
pub fn add(a: f32, b: f32) -> f32 {
    ld(a).add_m32(b).store()
}

/// `FFXVu0SubVector` component: `a - [b]`.
#[must_use]
pub fn sub(a: f32, b: f32) -> f32 {
    ld(a).sub_m32(b).store()
}

/// `FFXVu0MulVector` component: `a * [b]`.
#[must_use]
pub fn mul(a: f32, b: f32) -> f32 {
    ld(a).mul_m32(b).store()
}

/// `FFXVu0ScaleVector(dst, v, s)`: `v * s`, the last component with `v`'s
/// element as the memory operand.
#[must_use]
pub fn scale(v: [f32; 4], s: f32) -> [f32; 4] {
    let s = ld(s);
    [
        (ld(v[0]) * s).store(),
        (ld(v[1]) * s).store(),
        (ld(v[2]) * s).store(),
        s.mul_m32(v[3]).store(),
    ]
}

/// `FFXVu0DivVector(dst, v, s)`: zero when `s` is ±0, else `v` times the
/// reciprocal of `s` stored as an `f32`, as [`scale`] multiplies.
#[must_use]
pub fn div(v: [f32; 4], s: f32) -> [f32; 4] {
    if s == 0.0 {
        return [0.0; 4];
    }
    scale(v, (Reg::ONE / ld(s)).store())
}

/// `FFXVu0DivVectorXYZ` component `i` of `v / s`: the third with `v`'s
/// element as the memory operand (`fdivr`).
#[must_use]
pub fn div_xyz(i: u32, v: f32, s: f32) -> f32 {
    if i == 2 {
        ld(s).divr_m32(v).store()
    } else {
        (ld(v) / ld(s)).store()
    }
}

/// `FFXVu0InnerProduct(a, b)`: `(a1 b1 + a0 b0) + a2 b2`, `b` the memory
/// operands.
#[must_use]
pub fn inner(a: [f32; 3], b: [f32; 3]) -> f32 {
    (ld(a[1]).mul_m32(b[1]) + ld(a[0]).mul_m32(b[0]) + ld(a[2]).mul_m32(b[2])).store()
}

/// `FFXVu0OuterProduct(dst, a, b)`: `a × b`, `w` = 0, with the original's
/// memory operands.
#[must_use]
pub fn outer(a: [f32; 3], b: [f32; 3]) -> [f32; 4] {
    [
        (ld(a[1]).mul_m32(b[2]) - ld(b[1]).mul_m32(a[2])).store(),
        (ld(b[0]).mul_m32(a[2]) - ld(b[2]).mul_m32(a[0])).store(),
        (ld(b[1]).mul_m32(a[0]) - ld(a[1]).mul_m32(b[0])).store(),
        0.0,
    ]
}

/// `FFXVu0InterVector` component `i`: `b (1 - t) + a t`, `1 - t` stored as
/// an `f32` first (`u`); the last with `b`'s element as the memory operand.
#[must_use]
pub fn inter(i: u32, a: f32, b: f32, t: f32, u: f32) -> f32 {
    let (t, u) = (ld(t), ld(u));
    if i == 3 {
        (t * ld(a) + u.mul_m32(b)).store()
    } else {
        (ld(b) * u + ld(a) * t).store()
    }
}

/// `1 - t` as `FFXVu0InterVector` stores it.
#[must_use]
pub fn one_minus(t: f32) -> f32 {
    (Reg::ONE - ld(t)).store()
}

/// `FFXVu0InterVectorXYZ` component `i < 3`: `a t + b (1 - t)`, `1 - t`
/// kept in the register; the third with `b`'s element as the memory
/// operand.
#[must_use]
pub fn inter_xyz(i: u32, a: f32, b: f32, t: f32) -> f32 {
    let t = ld(t);
    let w = Reg::ONE - t;
    if i == 2 {
        (t * ld(a) + w.mul_m32(b)).store()
    } else {
        (ld(a) * t + ld(b) * w).store()
    }
}

/// The identity matrix (`0x006ece60`, `FFXVu0UnitMatrix`).
#[must_use]
pub fn identity() -> [f32; 16] {
    std::array::from_fn(|k| if k % 5 == 0 { 1.0 } else { 0.0 })
}

/// `FFXVu0RotTransPers` with `fixed` 0: a coordinate as the GS takes it,
/// 12.4 fixed point rounded (`x * 16 + 0.5` through `_ftol2_sse`), given
/// the two constants the original loads (16.0 and 0.5).
pub fn to_fixed(mem: &impl ffx_bridge::Mem, x: f32, k_scale: f64, k_round: f64) -> u32 {
    use ffx_bridge::x87::{add, mul};
    ffx_bridge::x87::ftol(mem, add(mul(ld(x).value(), k_scale), k_round))
}

/// A C runtime function of one float the library calls (`_CIcos`,
/// `_CIsin`): the caller supplies it (in the game, MSVCR110's own), taking
/// the angle as loaded and returning what the original stores (`fstp
/// dword`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trig {
    /// `_CIcos`.
    Cos,
    /// `_CIsin`.
    Sin,
}

/// The axis `FFXVu0RotMatrixX`/`Y`/`Z` turn about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// `FFXVu0RotMatrixX` (`cdc::Matrix::RotX`).
    X,
    /// `FFXVu0RotMatrixY` (`cdc::Matrix::RotY`).
    Y,
    /// `FFXVu0RotMatrixZ` (`cdc::Matrix::RotZ`).
    Z,
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The scale `FFXVu0RotTransPers` applies (an `f64` 16.0 in `.rdata`).
    pub const FIXED_SCALE: Global<f64> = Global::new(0x00b3_13a0);
    /// The rounding it adds (an `f64` 0.5 in `.rdata`).
    pub const FIXED_ROUND: Global<f64> = Global::new(0x00b9_2220);

    /// What `FFXVu0ClipScreen` returns (the chara screen code reads and
    /// writes it too).
    pub const CLIP_SCREEN: Global<[f32; 4]> = Global::new(0x0130_0978);

    /// `FFXVu0ClipScreen()` (`0x00835b90`): the address of [`CLIP_SCREEN`].
    pub fn clip_screen(mem: &impl Mem) -> u32 {
        CLIP_SCREEN.ptr(mem).addr
    }

    fn at(p: u32, i: u32) -> u32 {
        p.wrapping_add(4 * i)
    }

    fn vec4(mem: &impl Mem, p: u32) -> [f32; 4] {
        mem.read(p)
    }

    /// Writes `r` to `dst` and returns its last word (what the originals
    /// leave in `eax` after copying a result from their locals).
    fn put4(mem: &mut impl Mem, dst: u32, r: [f32; 4]) -> u32 {
        mem.write(dst, r);
        r[3].to_bits()
    }

    /// `FFXVu0CopyVector(dst, src)` (`0x006ed370`): word by word; returns
    /// the last word.
    pub fn copy_vector(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        let mut w = 0;
        for i in 0..4 {
            w = mem.read::<u32>(at(src, i));
            mem.write(at(dst, i), w);
        }
        w
    }

    /// `FFXVu0CopyVectorXYZ(dst, src)` (`0x0062fca0`): x, y, z through the
    /// x87; returns `dst`.
    pub fn copy_vector_xyz(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        for i in 0..3 {
            let v = ffx_bridge::x87::round_trip(mem.read(at(src, i)));
            mem.write(at(dst, i), v);
        }
        dst
    }

    /// `FFXVu0AddVector(dst, a, b)` (`0x006ed290`) and its siblings: the
    /// four results computed before any is written.
    fn elementwise(mem: &mut impl Mem, dst: u32, a: u32, b: u32, f: fn(f32, f32) -> f32) -> u32 {
        let (x, y) = (vec4(mem, a), vec4(mem, b));
        put4(mem, dst, std::array::from_fn(|i| f(x[i], y[i])))
    }

    /// `FFXVu0AddVector(dst, a, b)` (`0x006ed290`).
    pub fn add_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32) -> u32 {
        elementwise(mem, dst, a, b, super::add)
    }

    /// `FFXVu0SubVector(dst, a, b)` (`0x006ed940`).
    pub fn sub_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32) -> u32 {
        elementwise(mem, dst, a, b, super::sub)
    }

    /// `FFXVu0MulVector(dst, a, b)` (`0x006ed5d0`).
    pub fn mul_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32) -> u32 {
        elementwise(mem, dst, a, b, super::mul)
    }

    /// `FFXVu0ScaleVector(dst, v, s)` (`0x006ed8f0`).
    pub fn scale_vector(mem: &mut impl Mem, dst: u32, v: u32, s: f32) -> u32 {
        let r = super::scale(vec4(mem, v), s);
        put4(mem, dst, r)
    }

    /// `FFXVu0DivVector(dst, v, s)` (`0x006ed3a0`): returns `dst` for a zero
    /// divisor, else the last word.
    pub fn div_vector(mem: &mut impl Mem, dst: u32, v: u32, s: f32) -> u32 {
        if s == 0.0 {
            mem.write(dst, [0.0f32; 4]);
            return dst;
        }
        let r = super::div(vec4(mem, v), s);
        put4(mem, dst, r)
    }

    /// `FFXVu0DivVectorXYZ(dst, v, s)` (`0x006ed420`): each component
    /// written before the next is read; returns `dst`.
    pub fn div_vector_xyz(mem: &mut impl Mem, dst: u32, v: u32, s: f32) -> u32 {
        for i in 0..3 {
            let r = super::div_xyz(i, mem.read(at(v, i)), s);
            mem.write(at(dst, i), r);
        }
        dst
    }

    /// `FFXVu0InnerProduct(a, b)` (`0x0062fce0`).
    pub fn inner_product(mem: &impl Mem, a: u32, b: u32) -> f32 {
        super::inner(mem.read(a), mem.read(b))
    }

    /// `FFXVu0OuterProduct(dst, a, b)` (`0x006ed720`): returns 0 (the last
    /// word, `w`).
    pub fn outer_product(mem: &mut impl Mem, dst: u32, a: u32, b: u32) -> u32 {
        let r = super::outer(mem.read(a), mem.read(b));
        put4(mem, dst, r)
    }

    /// `FFXVu0InterVector(dst, a, b, t)` (`0x0062fd10`): each component
    /// written before the next is read; returns `b`.
    pub fn inter_vector(mem: &mut impl Mem, dst: u32, a: u32, b: u32, t: f32) -> u32 {
        let u = super::one_minus(t);
        for i in 0..4 {
            let r = super::inter(i, mem.read(at(a, i)), mem.read(at(b, i)), t, u);
            mem.write(at(dst, i), r);
        }
        b
    }

    /// `FFXVu0InterVectorXYZ(dst, a, b, t)` (`0x0062fd70`): `w` copied from
    /// `a`; returns `b`.
    pub fn inter_vector_xyz(mem: &mut impl Mem, dst: u32, a: u32, b: u32, t: f32) -> u32 {
        for i in 0..3 {
            let r = super::inter_xyz(i, mem.read(at(a, i)), mem.read(at(b, i)), t);
            mem.write(at(dst, i), r);
        }
        let w = ffx_bridge::x87::round_trip(mem.read(at(a, 3)));
        mem.write(at(dst, 3), w);
        b
    }

    /// `FFXVu0Normalize(dst, src)` (`0x006ed6f0`): `src` copied word by
    /// word, then `cdc_Vector_SafeNormalize3(dst)`: `xyz` to length 1, `w`
    /// kept, or all zero for a vector too short (or not finite).
    pub fn normalize(mem: &mut impl Mem, dst: u32, src: u32) {
        copy_vector(mem, dst, src);
        ffx_system::math::bridge::safe_normalize3(mem, dst);
    }

    /// The identity (`0x006ece60`, `this` in ECX): returns 1.0's bits.
    pub fn matrix_identity(mem: &mut impl Mem, m: u32) -> u32 {
        mem.write(m, super::identity());
        1f32.to_bits()
    }

    /// `Matrix::copy(src)` (`0x006ecc40`, thiscall): word by word; returns
    /// `this`.
    pub fn matrix_copy(mem: &mut impl Mem, this: u32, src: u32) -> u32 {
        for i in 0..16 {
            let w: u32 = mem.read(at(src, i));
            mem.write(at(this, i), w);
        }
        this
    }

    /// `FFXVu0RotMatrixX`/`Y`/`Z(dst, src, angle)` (`0x006ed780`,
    /// `0x006ed7c0`, `0x006ed800`): `src` copied to `dst`
    /// (`Matrix::copy`), then `cos(angle)` and `sin(angle)` (in that
    /// order, through `trig`) turn it (`cdc::Matrix::RotX`/`RotY`/`RotZ`).
    /// Returns `dst`.
    pub fn rot_matrix(
        mem: &mut impl Mem,
        axis: super::Axis,
        dst: u32,
        src: u32,
        angle: f32,
        trig: &mut impl FnMut(super::Trig, f32) -> f32,
    ) -> u32 {
        use ffx_system::math::bridge::{rotate, rotate_y};
        matrix_copy(mem, dst, src);
        let c = trig(super::Trig::Cos, angle);
        let s = trig(super::Trig::Sin, angle);
        match axis {
            super::Axis::X => rotate(mem, dst, 1, s, c),
            super::Axis::Y => rotate_y(mem, dst, s, c),
            super::Axis::Z => rotate(mem, dst, 0, s, c),
        }
    }

    /// `FFXVu0ApplyMatrix(dst, m, v)` (`0x006ed2e0`): `cdc_Mul4x4` into a
    /// local, then copied; returns the last word.
    pub fn apply_matrix(mem: &mut impl Mem, dst: u32, m: u32, v: u32) -> u32 {
        let x: [f32; 4] = mem.read(v);
        let r: [f32; 4] = std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            let i = i as u32;
            let col = [0, 4, 8, 12].map(|row| mem.read::<f32>(at(m, row + i)));
            ffx_system::math::mul4x4_component(i, x, col)
        });
        put4(mem, dst, r)
    }

    /// `FFXVu0RotTransPers(dst, m, v, keep)` (`0x006ed840`): `v`
    /// transformed (`cdc_Mul3x4`), as floats when `keep` is set (returns
    /// `dst`), else in 12.4 fixed point (returns the last).
    pub fn rot_trans_pers(mem: &mut impl Mem, dst: u32, m: u32, v: u32, keep: u32) -> u32 {
        let x: [f32; 3] = mem.read(v);
        let mut t = [0.0f32; 4];
        for (i, slot) in (0..3).zip(t.iter_mut()) {
            let col = [0, 4, 8, 12].map(|row| mem.read::<f32>(at(m, row + i)));
            *slot = ffx_system::math::mul3x4_component(i, x, col);
        }
        if keep != 0 {
            let r = t.map(ffx_bridge::x87::round_trip);
            mem.write(dst, r);
            return dst;
        }
        let (k_scale, k_round) = (FIXED_SCALE.get(mem), FIXED_ROUND.get(mem));
        let mut last = 0;
        for (i, &c) in (0..4).zip(&t) {
            last = super::to_fixed(mem, c, k_scale, k_round);
            mem.write(at(dst, i), last);
        }
        last
    }

    /// `FFXVu0LightColorMatrix(dst, c0, c1, c2, ambient)` (`0x006ed4f0`):
    /// the four vectors as rows, copied whole; returns `dst`.
    pub fn light_color_matrix(mem: &mut impl Mem, dst: u32, rows: [u32; 4]) -> u32 {
        let r: [[u32; 4]; 4] = rows.map(|p| mem.read(p));
        mem.write(dst, r);
        dst
    }

    /// `FFXVu0NormalLightMatrix(dst, l0, l1, l2)` (`0x006ed620`): the three
    /// vectors as rows, the third twice, copied whole; returns its `w`.
    pub fn normal_light_matrix(mem: &mut impl Mem, dst: u32, rows: [u32; 3]) -> u32 {
        let r: [[u32; 4]; 3] = rows.map(|p| mem.read(p));
        mem.write(dst, [r[0], r[1], r[2], r[2]]);
        r[2][3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_cases() {
        assert_eq!(div([2.0, 4.0, 6.0, 8.0], 2.0), [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(div([2.0; 4], -0.0), [0.0; 4]);
        assert_eq!(
            outer([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            [0.0, 0.0, 1.0, 0.0]
        );
        assert_eq!(inner([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]), 32.0);
        assert_eq!(inter_xyz(0, 2.0, 4.0, 0.25), 3.5);
        assert_eq!(identity()[10], 1.0);
    }
}
