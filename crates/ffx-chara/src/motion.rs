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

    use crate::x87::Reg;

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

    /// The motion found in the model block's lists for character `key`:
    /// the archive's (`+0x10c`: count `i32` `+4`, pointers from `+0x10`)
    /// or, with block bit `+0x70` 0, the block's own (`i16` count
    /// `+0x110`, pointers at `+0x114`); each pointed-to starts with the
    /// `i16` character. 0 when none.
    fn find_motion(mem: &impl Mem, b: u32, key: i32) -> u32 {
        if mem.read::<u8>(b + 0x70) & 1 == 0 {
            let t: u32 = mem.read(b + 0x10c);
            if t == 0 {
                return 0;
            }
            let mut i = 0i32;
            while i < mem.read::<i32>(t + 4) {
                let at = t.wrapping_add(0x10 + 4 * i.cast_unsigned());
                let p: u32 = mem.read(at);
                if i32::from(mem.read::<i16>(p)) == key {
                    return mem.read(at);
                }
                i += 1;
            }
        } else {
            let list: u32 = mem.read(b + 0x114);
            if list == 0 {
                return 0;
            }
            let n = i32::from(mem.read::<i16>(b + 0x110));
            for i in 0..n.max(0).cast_unsigned() {
                let p: u32 = mem.read(list.wrapping_add(4 * i));
                if i32::from(mem.read::<i16>(p)) == key {
                    return p;
                }
            }
        }
        0
    }

    /// `Ch_SetSequence(chr, id)` (`0x00837ac0`): starts motion `id`
    /// (character `<< 16 | number`). Another character's motion needs it
    /// in the model block's lists (kept at `+0x758`). The number is looked
    /// up in [`SEQUENCES`]' chain for the character (`u16` at `+0` of each
    /// entry's sorted list, 16 bytes per motion from `+0xc`, count `i16`
    /// `+8`). Returns 0 when it started; 1 when it was already playing,
    /// or not found (then `+0x736` 0 and `+0x728` 1, and `+0x7ed` 1 when
    /// the chain lacked it).
    pub fn ch_set_sequence(mem: &mut impl Mem, chr: u32, id: u32) -> u32 {
        #[allow(clippy::cast_possible_wrap)] // sar
        let key = ((id as i32) >> 16) & 0xffff;
        mem.write(chr + 0x758, 0u32);
        if i32::from(mem.read::<u16>(chr + l::ID)) != key {
            let b: u32 = mem.read(chr + l::BLK);
            let m = find_motion(mem, b, key);
            if m != 0 {
                mem.write(chr + 0x758, m);
            }
            if mem.read::<u32>(chr + 0x758) == 0 {
                mem.write(chr + 0x736, 0u8);
                mem.write(chr + 0x728, 1u16);
                return 1;
            }
        }
        if mem.read::<u32>(chr + l::MOTION_ID) == id && mem.read::<u32>(chr + l::MOTION_730) == 0 {
            return 1;
        }
        let d: u32 = mem.read(chr + l::DIRTY);
        mem.write(chr + l::DIRTY, d | 2);
        mem.write(chr + l::MOTION_730, 0u32);
        if mem.read::<u16>(chr + l::ID) & 0xf000 == 0
            && (mem.read::<u32>(chr + l::MOTION_ID) ^ id) & 0xffff_0000 != 0
        {
            mem.write(chr + l::NEXT_HOKAN, 1u16);
        }
        mem.write(chr + l::MOTION_ID, id);
        crate::cloth::bridge::ch_cloth_set_atari_jimen_check(mem, chr);
        let number = id & 0xffff;
        let mut s = SEQUENCE_HEAD.get(mem);
        while s != 0 && i32::from(mem.read::<i16>(s + 4)) != key {
            s = mem.read(s + 0xc);
        }
        while s != 0 {
            let d: u32 = mem.read(s);
            let n = i32::from(mem.read::<i16>(d + 8));
            let list: u32 = mem.read(d + 0xc);
            for i in 0..n.max(0).cast_unsigned() {
                if u32::from(mem.read::<u16>(list.wrapping_add(16 * i))) != number {
                    continue;
                }
                mem.write(chr + 0x710, d);
                let e = mem.read::<u32>(d + 0xc).wrapping_add(16 * i);
                mem.write(chr + 0x70c, e);
                let v: u32 = mem.read(e + 0xc);
                mem.write(chr + 0x714, v);
                mem.write(chr + 0x728, 0u32);
                mem.write(chr + l::MOTION_LOOP, 0xffffu16);
                let next = mem.read::<u16>(chr + l::NEXT_HOKAN);
                mem.write(chr + 0x737, 0xffu16);
                mem.write(chr + l::MOTION_SCRIPTED, 1u8);
                if next == 0 {
                    mem.write(chr + l::HOKAN, 4u16);
                } else {
                    mem.write(chr + l::HOKAN, next);
                    mem.write(chr + l::NEXT_HOKAN, 0u16);
                    let d: u32 = mem.read(chr + l::DIRTY);
                    mem.write(chr + l::DIRTY, d | 4);
                }
                let v: u32 = mem.read(s + 0x10);
                let f: u32 = mem.read(chr + l::FLAGS);
                mem.write(chr + l::FLAGS, f | 0x0100_0000);
                mem.write(chr + 0x7e8, v);
                mem.write(chr + 0x7e4, u32::MAX);
                mem.write(chr + 0x7ed, 0u16);
                mem.write(chr + 0x73e, 1u8);
                return 0;
            }
            s = mem.read(s + 8);
        }
        mem.write(chr + 0x7ed, 1u8);
        mem.write(chr + 0x736, 0u8);
        mem.write(chr + 0x728, 1u16);
        1
    }

    /// `0.5f` (`Chr_FracStep`'s threshold).
    pub const K_HALF_F32: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b9_2194);

    /// The fraction of `x` in `0..1` the way `Chr_FracStep` takes it:
    /// `x − ftol(x)` rounded to `f32`, plus 1 when negative.
    fn frac(mem: &impl Mem, x: f32) -> f32 {
        let i = ffx_bridge::x87::ftol(mem, f64::from(x));
        let fi = Reg::constant(f64::from(i.cast_signed())).store();
        let f = Reg::load(x).sub_m32(fi).store();
        if f < 0.0 {
            Reg::load(f).add_st(Reg::constant(1.0)).store()
        } else {
            f
        }
    }

    /// `Chr_FracStep(a, b, n)` (`0x00839f50`): `fa` = the fraction of `a`,
    /// `fb` = the fraction of `fa − b`; `fa` when `fb` is 0, else
    /// `fa − fb / n` when `fb < 0.5`, else `fa + (1 − fb) / n` (each step
    /// as the x87 rounds).
    pub fn chr_frac_step(mem: &impl Mem, a: f32, b: f32, n: i32) -> f32 {
        let fa = frac(mem, crate::x87::load(a));
        let t = Reg::load(fa).sub_m32(b).store();
        let fb = frac(mem, t);
        if f64::from(fb) == 0.0 {
            return fa;
        }
        let nf = Reg::constant(f64::from(n)).store();
        let half = f64::from(crate::x87::load(K_HALF_F32.get(mem)));
        if half > f64::from(fb) {
            let q = Reg::load(fb).div_st(Reg::load(nf));
            Reg::load(fa).sub_st(q).store()
        } else {
            let q = Reg::constant(1.0)
                .sub_st(Reg::load(fb))
                .div_st(Reg::load(nf));
            Reg::load(fa).add_st(q).store()
        }
    }

    /// `ReadU16Unaligned(p)` (`0x00838220`) as its callers take it: the
    /// two bytes sign-extended and or-ed (`b1 << 8 | b0`), then the low
    /// half sign-extended (so a negative low byte fills the high one).
    fn read_i16u(mem: &impl Mem, p: u32) -> i32 {
        let b: [u8; 2] = mem.read(p);
        let v = (i32::from(b[1].cast_signed()) << 8) | i32::from(b[0].cast_signed());
        #[allow(clippy::cast_possible_truncation)] // movsx of ax
        let r = i32::from(v as i16);
        r
    }

    /// `Ch_GetSeqFrame(chr)` (`0x00836af0`): the frames the motion
    /// sequence at `+0x714` adds up to, or -1 (no sequence, a bad step, an
    /// unknown code). Codes (first byte): 0 and 5 end; 1 a motion segment
    /// (motion `i16` +1 of the group `+0x710`, times `i16` +3, frames
    /// between the keys `i16` +5 and +7), 9 bytes; 2 skips a byte; 3 adds
    /// `i16` +1, 3 bytes; 4 jumps (`i16` +1 into the list at `+0x70c`);
    /// 6 skips 3 bytes.
    pub fn ch_get_seq_frame(mem: &impl Mem, chr: u32) -> i32 {
        let mut p: u32 = mem.read(chr + 0x714);
        if p == 0 {
            return -1;
        }
        let mut total = 0i32;
        loop {
            match mem.read::<i8>(p) {
                0 | 5 => return total,
                1 => {
                    let m = read_i16u(mem, p + 1).cast_unsigned() << 4;
                    let group: u32 = mem.read(chr + 0x710);
                    let seg = m.wrapping_add(mem.read(group + 0x10));
                    if seg == 0 {
                        return -1;
                    }
                    let k = read_i16u(mem, p + 3);
                    if k == 0 {
                        return -1;
                    }
                    let keys: u32 = mem.read(seg + 8);
                    let key = |i: i32| -> i32 {
                        i32::from(
                            mem.read::<i16>(keys.wrapping_add(i.cast_unsigned().wrapping_mul(2))),
                        )
                    };
                    let d = key(read_i16u(mem, p + 7)).wrapping_sub(key(read_i16u(mem, p + 5)));
                    total = total.wrapping_add(d.wrapping_mul(k));
                    p = p.wrapping_add(9);
                }
                2 => p = p.wrapping_add(1),
                3 => {
                    total = total.wrapping_add(read_i16u(mem, p + 1));
                    p = p.wrapping_add(3);
                }
                4 => {
                    let list: u32 = mem.read(chr + 0x70c);
                    let i = read_i16u(mem, p + 1).cast_unsigned();
                    let tbl: u32 = mem.read(list + 8);
                    let off = i32::from(mem.read::<i16>(tbl.wrapping_add(i.wrapping_mul(2))));
                    p = off.cast_unsigned().wrapping_add(mem.read(list + 0xc));
                }
                6 => p = p.wrapping_add(3),
                _ => return -1,
            }
        }
    }

    /// `Ch_MotionIsEnd(chr)` (`0x008371b0`): with a sequence (`+0x73e`),
    /// whether `Ch_GetSeqFrame` fails; else whether no loops are left.
    pub fn ch_motion_is_end(mem: &impl Mem, chr: u32) -> u32 {
        if mem.read::<u8>(chr + 0x73e) != 0 {
            u32::from(ch_get_seq_frame(mem, chr) == -1)
        } else {
            u32::from(mem.read::<u16>(chr + l::MOTION_LOOPS_LEFT) == 0)
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
