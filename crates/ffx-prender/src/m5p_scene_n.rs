//! M5 stage 1, list `scene`, batch 14: the visible bounds job's plane
//! test of a point (`0x009f2890`) and a skeleton joint's world matrix
//! (`0x004f71e0`). Floats on the x87 at 53 bits, each store rounded to
//! `f32` ([`ffx_bridge::x87`]); frames as in [`crate::m5p_scene_f`].

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_f::FROM_MATRIX3X4;

/// `PMatrix4_MulThis_004c9da0` (thiscall `(a; out, b)`, `ret 8`): `out =
/// a . b`, returning `out`.
pub const MUL_THIS: u32 = 0x004c_9da0;

pub mod bridge {
    //! The port over game memory.

    use super::{Conv, FROM_MATRIX3X4, Host, MUL_THIS, Mem, x87};

    const EXTERNAL: u32 = 0x8000_0000;

    fn ld(m: &impl Mem, at: u32) -> f64 {
        let bits: u32 = m.read(at);
        x87::load(bits)
    }

    /// `0x009f2890` (cdecl `(planes, point)`): the four planes (columns of
    /// the 4x4 at `planes`) against `point` (x, y, z): for each plane `i`,
    /// `(m0[i] x + m1[i] y) + m2[i] z - m3[i]` (each product, sum and the
    /// difference rounded to `f32`), and whether that is below 0 (a NaN is
    /// not). The four answers as the bytes of `eax`, plane 0 lowest.
    pub fn planes_below(m: &impl Mem, planes: u32, point: u32) -> u32 {
        let x = x87::load(x87::store(ld(m, point)));
        let y = x87::load(x87::store(ld(m, point + 4)));
        let z = x87::load(x87::store(ld(m, point + 8)));
        let mut r = 0u32;
        for i in 0..4u32 {
            let p0 = x87::store(x87::mul(ld(m, planes + 4 * i), x));
            let p1 = x87::store(x87::mul(ld(m, planes + 0x10 + 4 * i), y));
            let p2 = x87::store(x87::mul(ld(m, planes + 0x20 + 4 * i), z));
            let s = x87::store(x87::add(x87::load(p0), x87::load(p1)));
            let s = x87::store(x87::add(x87::load(s), x87::load(p2)));
            let d = x87::store(x87::sub(x87::load(s), ld(m, planes + 0x30 + 4 * i)));
            let below = x87::load(d) < 0.0;
            r |= u32::from(below) << (8 * i);
        }
        r
    }

    /// `acc = m . acc` through the frame's scratch at `tmp`, read back from
    /// the product's returned address as `f32` copies.
    fn mul_into(h: &mut impl Host, m: u32, tmp: u32, acc: u32) {
        let r = h.call_sym(MUL_THIS, Conv::Thiscall(m), &[tmp, acc]);
        for i in 0..16 {
            let f: f32 = h.read(r.wrapping_add(4 * i));
            h.write(acc + 4 * i, x87::round_trip(f));
        }
    }

    /// `0x004f71e0` (thiscall `(out, joint)`, `ret 8`): the world matrix
    /// of a skeleton joint: the identity for a joint past the skeleton's
    /// count (`[this] + 0x18`); else its local matrix (`+0xc`, 64 bytes
    /// each) times each parent's (`[this] + 0x2c`, until a negative
    /// index), then the instance's world (`+4`, a 3x4) unless `+0x6c` bit
    /// 0 is set. The frame is the original's `ebp-0xcc` (51 words: the
    /// products' scratch at 0 and 0x40, the accumulator at 0x8c). `out`.
    pub fn joint_world(h: &mut impl Host, this: u32, out: u32, joint: u32, frame: u32) -> u32 {
        let skel: u32 = h.read(this);
        let parents: u32 = h.read(skel + 0x2c);
        let count: u32 = h.read(skel + 0x18);
        let mats: u32 = h.read(this + 0xc);
        if joint >= count & !EXTERNAL {
            for i in 0..16 {
                let v = if i % 5 == 0 { 1.0f32 } else { 0.0 };
                h.write(out + 4 * i, v);
            }
            return out;
        }
        let scratch = frame;
        let local = frame + 0x40;
        let acc = frame + 0x8c;
        let src = mats.wrapping_add(joint << 6);
        for i in 0..16 {
            let w: u32 = h.read(src + 4 * i);
            h.write(acc + 4 * i, w);
        }
        let mut p: u32 = h.read(parents.wrapping_add(joint.wrapping_mul(4)));
        while p & EXTERNAL == 0 {
            let m = mats.wrapping_add(p << 6);
            mul_into(h, m, local, acc);
            p = h.read(parents.wrapping_add(p.wrapping_mul(4)));
        }
        let flags: u32 = h.read(this + 0x6c);
        if flags & 1 == 0 {
            let world: u32 = h.read(this + 4);
            h.call_sym(FROM_MATRIX3X4, Conv::Thiscall(local), &[world]);
            mul_into(h, local, scratch, acc);
        }
        for i in 0..16 {
            let w: u32 = h.read(acc + 4 * i);
            h.write(out + 4 * i, w);
        }
        out
    }
}
