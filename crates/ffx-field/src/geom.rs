//! `rcBBox`, `rcVec3f`, `rcMat4f`: the field's small geometry helpers.
//!
//! Vectors are four floats (x, y, z, w), matrices 4 x 4 floats in rows
//! (the translation in the fourth row, `+0x30`). Float semantics follow
//! the original's x87 code ([`crate::x87`]).

use ffx_bridge::Global;

use crate::x87;

/// A bounding box (`rcBBoxInit`, 32 bytes): the minimum corner at `+0x0`,
/// the maximum at `+0x10`, both with w = 1.
pub mod bbox {
    /// `+0x0`: the minimum corner (x, y, z, w).
    pub const MIN: u32 = 0x0;
    /// `+0x10`: the maximum corner (x, y, z, w).
    pub const MAX: u32 = 0x10;
    /// An empty box: the minimum at `f32::MAX`, the maximum at `-f32::MAX`,
    /// so any point widens it (`rcBBoxSet`).
    pub const EMPTY: [u32; 8] = [
        0x7f7f_ffff,
        0x7f7f_ffff,
        0x7f7f_ffff,
        0x3f80_0000,
        0xff7f_ffff,
        0xff7f_ffff,
        0xff7f_ffff,
        0x3f80_0000,
    ];
}

/// The camera matrix (`g_CameraMatrix`, `MsGetCameraMatrix`).
pub const CAMERA_MATRIX: Global<[f32; 16]> = Global::new(0x0231_1440);
/// The screen matrix (`g_ScreenMatrix`, `MsGetScreenMatrix`).
pub const SCREEN_MATRIX: Global<[f32; 16]> = Global::new(0x0231_1480);
/// The background's world matrix (`rcbgSetWorldMatrix`).
pub const WORLD: Global<[f32; 16]> = Global::new(0x025d_59a0);
/// World through the camera (`rcbgUpdateMatrix`).
pub const VIEW_WORLD: Global<[f32; 16]> = Global::new(0x025d_5960);
/// The clip matrix (`rcbgSetClipMatrix`).
pub const CLIP: Global<[f32; 16]> = Global::new(0x025d_58e0);
/// [`VIEW_WORLD`] through the clip matrix (`rcbgUpdateMatrix`).
pub const CLIP_WORLD: Global<[f32; 16]> = Global::new(0x025d_58a0);
/// [`VIEW_WORLD`] through the screen matrix (`rcbgUpdateMatrix`).
pub const SCREEN_WORLD: Global<[f32; 16]> = Global::new(0x025d_5920);

/// Offset of a matrix's translation row.
pub const MAT_TRANSLATE: u32 = 0x30;

/// One axis of the box's centre: `(max + min) * 0.5` (`rcBBoxGetCenter`).
#[must_use]
#[allow(clippy::manual_midpoint)] // the original's sum, then product
pub fn center(min: u32, max: u32) -> u32 {
    x87::store((x87::load(max) + x87::load(min)) * 0.5)
}

/// `rcBBoxGetShortConvertScale`: the largest absolute coordinate of the
/// box's corners (at least 0; NaNs never win) over 4000, the scale that
/// brings the box into ±4000.
#[must_use]
pub fn short_convert_scale(corners: [f32; 8]) -> f32 {
    // the original's order: min.x, max.x, min.y, max.y, min.z, max.z
    let mut m = 0.0f32;
    for i in [0, 4, 1, 5, 2, 6] {
        let a = corners[i].abs();
        if a > m {
            m = a;
        }
    }
    f32::from_bits(x87::store(f64::from(m) / 4000.0))
}

/// One component of a cross product: `a * b - c * d`, rounded once.
#[must_use]
pub fn cross_term(a: u32, b: u32, c: u32, d: u32) -> u32 {
    x87::store(x87::load(a) * x87::load(b) - x87::load(c) * x87::load(d))
}

/// A vector's length, `sqrt((x² + y²) + z²)` at 53 bits, as `f32`
/// (`rcVec3fLength`, `KeVu0VecLen`).
#[must_use]
pub fn length3(v: [u32; 3]) -> u32 {
    let [x, y, z] = v.map(x87::load);
    x87::store((x * x + y * y + z * z).sqrt())
}

/// The geometry where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{CAMERA_MATRIX, CLIP, CLIP_WORLD, SCREEN_MATRIX, SCREEN_WORLD, VIEW_WORLD, WORLD};
    use super::{MAT_TRANSLATE, bbox, center, cross_term, length3, short_convert_scale};
    use crate::x87;

    fn vec3(mem: &impl Mem, v: u32) -> [u32; 3] {
        mem.read(v)
    }

    /// Component `i` of the vector at `v`, as bits.
    fn at(mem: &impl Mem, v: u32, i: u32) -> u32 {
        mem.read(v + 4 * i)
    }

    /// `rcBBoxInit(box)` (`0x009214e0`): empty.
    pub fn rc_bbox_init(mem: &mut impl Mem, b: u32) {
        mem.write(b, bbox::EMPTY);
    }

    /// `rcBBoxGetCenter(box, out)` (`0x009213d0`): writes out's x, y, z.
    pub fn rc_bbox_get_center(mem: &mut impl Mem, b: u32, out: u32) {
        // per axis, read then written, as the original (out may alias)
        for i in 0..3 {
            let c = center(
                mem.read(b + bbox::MIN + 4 * i),
                mem.read(b + bbox::MAX + 4 * i),
            );
            mem.write(out + 4 * i, c);
        }
    }

    /// `rcBBoxGetShortConvertScale(box)` (`0x00921410`).
    pub fn rc_bbox_get_short_convert_scale(mem: &impl Mem, b: u32) -> f32 {
        short_convert_scale(mem.read::<[u32; 8]>(b).map(f32::from_bits))
    }

    /// `rcVec3fMul(out, a, b)` (`0x009212c0`): the cross product a × b.
    /// Each component is written before the next is read, as in the
    /// original (`out` may be `a` or `b`).
    pub fn rc_vec3f_mul(mem: &mut impl Mem, out: u32, lhs: u32, rhs: u32) {
        let (a, b) = (lhs, rhs);
        let cx = cross_term(at(mem, a, 1), at(mem, b, 2), at(mem, a, 2), at(mem, b, 1));
        mem.write(out, cx);
        let cy = cross_term(at(mem, a, 2), at(mem, b, 0), at(mem, a, 0), at(mem, b, 2));
        mem.write(out + 4, cy);
        let cz = cross_term(at(mem, b, 1), at(mem, a, 0), at(mem, b, 0), at(mem, a, 1));
        mem.write(out + 8, cz);
    }

    /// `rcVec3fDot(a, b)` (`0x00921220`): `(a.y·b.y + a.x·b.x) + a.z·b.z`
    /// at 53 bits, as `f32`.
    pub fn rc_vec3f_dot(mem: &impl Mem, a: u32, b: u32) -> f32 {
        let [ax, ay, az] = vec3(mem, a).map(x87::load);
        let [bx, by, bz] = vec3(mem, b).map(x87::load);
        f32::from_bits(x87::store(ay * by + ax * bx + az * bz))
    }

    /// `rcBBoxSet(box, points, n)` (`0x00921530`): the box around `n`
    /// points (16 bytes apart); comparisons are strict, NaNs never win.
    pub fn rc_bbox_set(mem: &mut impl Mem, b: u32, points: u32, n: i32) {
        rc_bbox_init(mem, b);
        let mut p = points;
        for _ in 0..n.max(0) {
            for i in 0..3 {
                let v = f32::from_bits(mem.read(p + 4 * i));
                let lo = b + bbox::MIN + 4 * i;
                if v < f32::from_bits(mem.read(lo)) {
                    mem.write(lo, x87::copy(v.to_bits()));
                }
                let hi = b + bbox::MAX + 4 * i;
                if v > f32::from_bits(mem.read(hi)) {
                    mem.write(hi, x87::copy(v.to_bits()));
                }
            }
            p = p.wrapping_add(16);
        }
    }

    /// `rcCalcPersMatrix(dst, m)` (`0x00920020`): `m` through the camera
    /// matrix, then the screen matrix (`cdc_Mul4x4_007059b0` twice).
    pub fn rc_calc_pers_matrix(mem: &mut impl Mem, dst: u32, m: u32) {
        let camera = CAMERA_MATRIX.ptr(mem).addr;
        let screen = SCREEN_MATRIX.ptr(mem).addr;
        ffx_system::math::bridge::mul4x4(mem, dst, camera, m);
        ffx_system::math::bridge::mul4x4(mem, dst, screen, dst);
    }

    /// `rcbgUpdateMatrix()` (`0x0092b160`): the background's view, clip and
    /// screen matrices from its world matrix.
    pub fn rcbg_update_matrix(mem: &mut impl Mem) {
        let [world, view, clip, clip_world, screen_world] =
            [WORLD, VIEW_WORLD, CLIP, CLIP_WORLD, SCREEN_WORLD].map(|g| g.ptr(mem).addr);
        let camera = CAMERA_MATRIX.ptr(mem).addr;
        let screen = SCREEN_MATRIX.ptr(mem).addr;
        ffx_system::math::bridge::mul4x4(mem, view, camera, world);
        ffx_system::math::bridge::mul4x4(mem, clip_world, clip, view);
        ffx_system::math::bridge::mul4x4(mem, screen_world, screen, view);
    }

    /// `rcVec3fApplyMatrix(out, v, m)` (`0x00921300`): `v` (w = 1) through
    /// `m`, each sum kept on the x87 stack: `((m[1,k]·y + x·m[0,k]) +
    /// m[2,k]·z) + m[3,k]` (`x` and `y` terms swap places for k > 0);
    /// each component stored before the next is read.
    pub fn rc_vec3f_apply_matrix(mem: &mut impl Mem, out: u32, v: u32, matrix: u32) {
        use ffx_bridge::x87::{add, load, mul, store};
        for k in 0..3 {
            let [vx, vy, vz] = vec3(mem, v).map(load);
            let e = |row: u32| load(mem.read(matrix + 4 * (4 * row + k)));
            let r = add(add(add(mul(e(1), vy), mul(vx, e(0))), mul(e(2), vz)), e(3));
            mem.write(out + 4 * k, store(r));
        }
    }

    /// `a + (b - a) * t` per component, stored one at a time
    /// (`rcVec3fLerp`, `yiLerpVec4`, `yiLerpVec2`).
    fn lerp(mem: &mut impl Mem, out: u32, a: u32, b: u32, t: u32, n: u32) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let t = load(t);
        for i in 0..n {
            let va = load(mem.read(a + 4 * i));
            let vb = load(mem.read(b + 4 * i));
            mem.write(out + 4 * i, store(add(mul(sub(vb, va), t), va)));
        }
    }

    /// `rcVec3fLerp(out, a, b, t)` (`0x00921280`), `t` as bits.
    pub fn rc_vec3f_lerp(mem: &mut impl Mem, out: u32, a: u32, b: u32, t: u32) {
        lerp(mem, out, a, b, t, 3);
    }

    /// `yiLerpVec4(t, a, b, out)` (`0x009148d0`).
    pub fn yi_lerp_vec4(mem: &mut impl Mem, t: u32, a: u32, b: u32, out: u32) {
        lerp(mem, out, a, b, t, 4);
    }

    /// `yiLerpVec2(t, a, b, out)` (`0x009148a0`).
    pub fn yi_lerp_vec2(mem: &mut impl Mem, t: u32, a: u32, b: u32, out: u32) {
        lerp(mem, out, a, b, t, 2);
    }

    /// `rcVec3fLength(v)` (`0x00921250`).
    pub fn rc_vec3f_length(mem: &impl Mem, v: u32) -> f32 {
        f32::from_bits(length3(vec3(mem, v)))
    }

    /// `rcVec3fNormalize(v)` (`0x00921360`): scales x, y, z by the `f32`
    /// reciprocal of the length; a vector of length 0 (or NaN) becomes
    /// (0, 0, 0, 0), w included.
    pub fn rc_vec3f_normalize(mem: &mut impl Mem, v: u32) {
        let len = rc_vec3f_length(mem, v);
        if len > 0.0 {
            let inv = x87::load(x87::store(1.0 / f64::from(len)));
            for i in 0..3 {
                let x = x87::load(mem.read(v + 4 * i));
                mem.write(v + 4 * i, x87::store(inv * x));
            }
        } else {
            mem.write(v, [0u32; 4]);
        }
    }

    /// `rcMat4fSetTranslate(m, x, y, z)` (`0x00925850`): the translation row
    /// (w kept). Floats as bits: they pass through the x87 stack.
    pub fn rc_mat4f_set_translate(mem: &mut impl Mem, m: u32, x: u32, y: u32, z: u32) {
        mem.write(m + MAT_TRANSLATE, [x, y, z].map(x87::copy));
    }

    /// `rcMat4fSetTranslateVec3f(m, v)` (`0x00925870`).
    pub fn rc_mat4f_set_translate_vec3f(mem: &mut impl Mem, matrix: u32, v: u32) {
        let [x, y, z] = vec3(mem, v).map(x87::copy);
        rc_mat4f_set_translate(mem, matrix, x, y, z);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_takes_the_largest_coordinate() {
        let c = [1.0, -8000.0, 2.0, 1.0, 3.0, 4.0, f32::NAN, 1.0];
        assert_eq!(short_convert_scale(c), 2.0);
        assert_eq!(short_convert_scale([0.0; 8]), 0.0);
    }

    #[test]
    fn cross_and_length() {
        let b = |x: f32| x.to_bits();
        assert_eq!(cross_term(b(2.0), b(3.0), b(1.0), b(4.0)), b(2.0));
        assert_eq!(length3([b(3.0), b(4.0), b(12.0)]), b(13.0));
    }
}
