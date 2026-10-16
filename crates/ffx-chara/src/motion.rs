//! Motion interpolation helpers: float and angle lerps with a damped
//! weight, and the 12-bit angle lerp in turns (`AngleTurn_Lerp`). All
//! cdecl on floats, results in `st0`; each step rounds as the x87 does
//! ([`crate::x87::Reg`]).

use crate::fixed::{fixed8_to_float, fixed12_to_float};
use crate::x87::Reg;

/// 3.141592 and 6.283184 as the `double`s the original compares with.
const PI: f64 = f64::from_bits(0x4009_21fb_0000_0000);
const TWO_PI: f64 = f64::from_bits(0x4019_21fb_0000_0000);

/// `Chr_Lerp(a, b, t)` (`0x00839cb0`): `t × b + (1 − t) × a`.
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    let one_t = Reg::constant(1.0).sub_st(Reg::load(t)).mul_m32(a);
    Reg::load(t).mul_st(Reg::load(b)).add_st(one_t).store()
}

/// The weight `max(x − |a − b| × k, lo)`, the difference unrounded.
fn damped_weight(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let d = Reg::load(a).sub_st(Reg::load(b)).abs().mul_m32(k);
    let t = Reg::load(x).sub_st(d).store();
    if f64::from(crate::x87::load(lo)) > f64::from(t) {
        crate::x87::load(lo)
    } else {
        t
    }
}

/// `Chr_LerpDamped(a, b, x, lo, k)` (`0x00839d70`): [`lerp`] with the
/// weight `max(x − |a − b| × k, lo)`.
#[must_use]
pub fn lerp_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let t = damped_weight(a, b, x, lo, k);
    lerp(crate::x87::load(a), crate::x87::load(b), t)
}

/// `Chr_LerpAngleDamped(a, b, x, lo, k)` (`0x00839cd0`): as
/// [`lerp_damped`] on angles: `b` first moved by a turn toward `a` when
/// they are more than π apart.
#[must_use]
pub fn lerp_angle_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
    let a = crate::x87::load(a);
    let mut b = crate::x87::load(b);
    if PI < Reg::load(a).sub_st(Reg::load(b)).get() {
        b = Reg::load(b).add_st(Reg::constant(TWO_PI)).store();
    }
    if Reg::load(b).sub_st(Reg::load(a)).get() > PI {
        b = Reg::load(b).sub_st(Reg::constant(TWO_PI)).store();
    }
    let t = damped_weight(a, b, x, lo, k);
    lerp(a, b, t)
}

/// `AngleTurn_Lerp(a, b, t)` (`0x00839e80`): 12-bit angles `a` toward
/// `b` the short way, in turns (as floats), `t` in 8.8 fixed.
#[must_use]
pub fn angle_turn_lerp(a: u32, b: u32, t: u32) -> f32 {
    let fa = fixed12_to_float(a & 0xfff);
    let fb = fixed12_to_float(b & 0xfff);
    let ft = fixed8_to_float(t);
    let mut base = fa;
    let mut d = Reg::load(fb).sub_st(Reg::load(fa)).store();
    // not below 0.5, NaN included
    if d.partial_cmp(&0.5) != Some(std::cmp::Ordering::Less) {
        base = Reg::load(fa).add_st(Reg::constant(1.0)).store();
        d = Reg::load(fb).sub_st(Reg::load(base)).store();
    }
    if -0.5 >= d {
        let b1 = Reg::load(fb).add_st(Reg::constant(1.0)).store();
        d = Reg::load(b1).sub_st(Reg::load(base)).store();
    }
    Reg::load(d)
        .mul_st(Reg::load(ft))
        .add_st(Reg::load(base))
        .store()
}

/// `Chr_RangeClass(a, b, x, y)` (`0x00831330`): where `x` and `y` fall
/// in the range `a..b`, which wraps when `a > b`: 1 for an empty range
/// (`a == b`), 2 when `x` is in it, 4 when `y` is (and `x` is not), else
/// 0; for a wrapping range 3 and 5 instead of 2 and 4. Signed.
#[must_use]
pub const fn range_class(a: i32, b: i32, x: i32, y: i32) -> u32 {
    if a == b {
        1
    } else if a < b {
        if a <= x && x < b {
            2
        } else if a <= y && y < b {
            4
        } else {
            0
        }
    } else if a <= x || x < b {
        3
    } else if a <= y || y < b {
        5
    } else {
        0
    }
}

/// The originals on the characters.
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::layout as l;

    /// `Ch_MotionGetLoop(chr)` (`0x00839880`): the loops left, `i16` at
    /// `+0x724`, sign-extended.
    pub fn ch_motion_get_loop(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i16>(chr + l::MOTION_LOOPS_LEFT))
    }

    /// `Ch_MotionSetNull(chr)` (`0x00837540`): no motion: `+0x714`, the
    /// motion id and `+0x7e8` cleared.
    pub fn ch_motion_set_null(mem: &mut impl Mem, chr: u32) {
        mem.write(chr + 0x714, 0u32);
        mem.write(chr + l::MOTION_ID, 0u32);
        mem.write(chr + 0x7e8, 0u32);
    }

    /// `Ch_MotionSet737(chr, v)` (`0x00837570`): the byte at `+0x737`;
    /// returns 0.
    pub fn ch_motion_set_737(mem: &mut impl Mem, chr: u32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + 0x737, v as u8);
        0
    }

    /// `Ch_MotionLoopFrame(chr)` (`0x008392d0`): the frame (`i32`
    /// `+0x740`, 8.8 fixed, negative taken as 0) past the end (`+0x71c`)
    /// goes back by the loop length (`+0x720`) while loops are left
    /// (`i16` `+0x724`, 0 for endless), to the start (`+0x718`) when the
    /// length is 0, or stops on the last whole frame when the loops run
    /// out. Returns 1 when it stopped or jumped to the start, else 0.
    pub fn ch_motion_loop_frame(mem: &mut impl Mem, chr: u32) -> u32 {
        if mem.read::<i32>(chr + 0x740) < 0 {
            mem.write(chr + 0x740, 0u32);
        }
        let end: u32 = mem.read(chr + 0x71c);
        let mut frame: i32 = mem.read(chr + 0x740);
        let step = i32::from(mem.read::<i16>(chr + 0x756));
        let ahead = step.wrapping_add(0x1ff).wrapping_add(frame) & !0xff;
        let last = end.wrapping_add(1).cast_signed() & !0xff;
        if ahead > last && mem.read::<i16>(chr + l::MOTION_LOOPS_LEFT) == 1 {
            mem.write(chr + 0x740, end & !0xff);
            mem.write(chr + 0x728, 0u16);
            return 1;
        }
        while frame > end.cast_signed() {
            let loops: u16 = mem.read(chr + l::MOTION_LOOPS_LEFT);
            if loops == 0 {
                if mem.read::<u32>(chr + 0x720) == 0 {
                    let start: u32 = mem.read(chr + 0x718);
                    mem.write(chr + 0x740, start);
                    return 1;
                }
            } else {
                let left = loops.wrapping_sub(1);
                mem.write(chr + l::MOTION_LOOPS_LEFT, left);
                if left == 0 {
                    mem.write(chr + 0x740, end & !0xff);
                    mem.write(chr + 0x728, 0u16);
                    mem.write(chr + 0x754, 0u16);
                    return 1;
                }
            }
            let len: u32 = mem.read(chr + 0x720);
            let f: u32 = mem.read(chr + 0x740);
            mem.write(chr + 0x740, f.wrapping_sub(len));
            frame = mem.read(chr + 0x740);
        }
        0
    }

    /// The sequence table (`SG_AddSequence`): 100 entries of `0x14`
    /// bytes from `0x01300a08`: `+0` data, `u16` key `+4`, `u16` taken
    /// `+6`, next of the same key `+8`, next key `+0xc`.
    pub const SEQUENCES: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_0a08);
    /// The first sequence of the first key (`Ch_SeqLink`).
    pub const SEQUENCE_HEAD: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_11d8);

    /// `Ch_SeqLink()` (`0x00838170`): links the used sequences: one chain
    /// per key through `+8` (later ones marked taken), the chains'
    /// first entries through `+0xc` from [`SEQUENCE_HEAD`].
    pub fn ch_seq_link(mem: &mut impl Mem) {
        let base = SEQUENCES.ptr(mem).addr;
        let at = |i: u32| base + 0x14 * i;
        SEQUENCE_HEAD.set(mem, 0);
        for i in 0..100 {
            mem.write(at(i) + 6, 0u16);
            mem.write(at(i) + 8, 0u32);
        }
        let mut prev = 0u32;
        for i in 0..100 {
            let e = at(i);
            if mem.read::<u32>(e) == 0 || mem.read::<u16>(e + 6) != 0 {
                continue;
            }
            if SEQUENCE_HEAD.get(mem) == 0 {
                SEQUENCE_HEAD.set(mem, e);
            }
            mem.write(e + 0xc, 0u32);
            if prev != 0 {
                mem.write(prev + 0xc, e);
            }
            prev = e;
            let mut tail = e;
            for k in 0..100 {
                let o = at(k);
                if mem.read::<u32>(o) != 0
                    && mem.read::<u16>(o + 6) == 0
                    && o != e
                    && mem.read::<u16>(e + 4) == mem.read::<u16>(o + 4)
                {
                    mem.write(tail + 8, o);
                    mem.write(o + 6, 1u16);
                    mem.write(o + 8, 0u32);
                    tail = o;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerps() {
        assert_eq!(lerp(0.0, 2.0, 0.5), 1.0);
        assert_eq!(lerp_damped(0.0, 2.0, 0.5, 0.0, 0.0), 1.0);
        assert!((angle_turn_lerp(0, 0xf00, 0x100) - (-0.0625 + 1.0)).abs() < 1e-6);
    }
}
