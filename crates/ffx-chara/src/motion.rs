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

    /// The motion group table (`SG_RemoveMotionGroup`): 50 entries of 12
    /// bytes from `0x013011e0`, the group's address first.
    pub const MOTION_GROUP_TABLE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_11e0);

    /// `Ch_MgrpMove(new, old)` (`0x008370c0`, the Switch name): a motion
    /// group moved from `old` to `new`. `relocate_group` is
    /// `SG_RelocateGroup` (at `new`); then the group's table entry, the
    /// sequences of the group (data `+0` and group `+0x10` of each used
    /// [`SEQUENCES`] entry whose group is `old`) and every active
    /// character playing it (`+0x7e8`, through `Ch_MoveMgrp`) follow by
    /// `new - old`. Returns what the original leaves in EAX (its loops'
    /// ends: the character array's, else the sequence table's).
    pub fn ch_mgrp_move<M: Mem>(
        mem: &mut M,
        new: u32,
        old: u32,
        relocate_group: impl FnOnce(&mut M, u32),
    ) -> u32 {
        let d = new.wrapping_sub(old);
        relocate_group(mem, new);
        let table = MOTION_GROUP_TABLE.ptr(mem).addr;
        if let Some(e) = (0..50)
            .map(|i| table + 12 * i)
            .find(|&e| mem.read::<u32>(e) == old)
        {
            mem.write(e, new);
        }
        let seqs = SEQUENCES.ptr(mem).addr;
        for i in 0..100 {
            let e = seqs + 0x14 * i;
            let data: u32 = mem.read(e);
            if data != 0 && mem.read::<u32>(e + 0x10) == old {
                mem.write(e, data.wrapping_add(d));
                mem.write(e + 0x10, old.wrapping_add(d));
            }
        }
        let array = crate::handle::bridge::CHR_ARRAY;
        if array.get(mem) == 0 {
            return seqs + 0x14 * 100 + 0x10;
        }
        crate::handle::bridge::each_active(mem, |m, c| {
            if m.read::<u32>(c + 0x7e8) == old {
                crate::mapping::bridge::ch_move_mgrp(m, c, d);
            }
        });
        #[allow(clippy::cast_sign_loss)] // the original multiplies unsigned
        let n = crate::handle::bridge::CHR_COUNT.get(mem) as u32;
        array.get(mem).wrapping_add(n.wrapping_mul(0x880))
    }

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

    /// The active camera (`g_Camera`).
    pub const CAMERA: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0231_1430);
    /// `"c10"`, `2500.0`, `900.0` and `-0.785398` (`double`):
    /// `Ch_AnimCopy2`'s name and limits.
    pub const COPY2_NAME: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x00b5_9af8);
    /// `900.0`, the distance² limit.
    pub const COPY2_NEAR: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b5_9afc);
    /// `2500.0`, the distance² limit for `c10*` models.
    pub const COPY2_NEAR_C10: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b5_9b00);
    /// `-0.785398` (`double`), the camera angle limit.
    pub const COPY2_ANGLE: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_9b08);

    /// `Ch_AnimCopy2(chr)` (`0x00839140`): whether the character is close
    /// to the camera (distance² from `+0x694` to the camera's `+0x65c`
    /// under 900, 2500 for `c10` models) while the camera's angle
    /// (`+0x6b0`, or `+0x6d0` when its byte `+0x64d` is 0) is above
    /// -0.785398. Only `al` is the result.
    pub fn ch_anim_copy2(mem: &impl Mem, chr: u32) -> u8 {
        let cam = CAMERA.get(mem);
        let p: [f32; 3] = mem.read(chr + 0x694);
        let c: [f32; 3] = mem.read(cam + 0x65c);
        let d: [f32; 3] = std::array::from_fn(|i| ffx_ps2::vu0::sub(p[i], c[i]));
        let d2 = ffx_system::math::length_squared(d);
        let name: u32 = mem.read(chr + l::NAME);
        let s = crate::handle::bridge::c_string(mem, name);
        let near = if s.windows(3).any(|w| w == b"c10") {
            COPY2_NEAR_C10.get(mem)
        } else {
            COPY2_NEAR.get(mem)
        };
        let angle: f32 = if mem.read::<u8>(cam + 0x64d) != 0 {
            mem.read(cam + 0x6b0)
        } else {
            mem.read(cam + 0x6d0)
        };
        let near = f64::from(crate::x87::load(near));
        let close = near > f64::from(crate::x87::load(d2));
        u8::from(close && f64::from(crate::x87::load(angle)) > COPY2_ANGLE.get(mem))
    }

    /// `Ch_SeqGetFastMotion(chr)` (`0x00837860`): the first motion the
    /// sequence at `+0x714` plays: walks its codes (2: one byte, 3 and 6:
    /// three, 4: a jump through the sequence group `+0x70c`, table `+8` of
    /// `i16` offsets from `+0xc`) to the first 1, whose `i16` motion picks a
    /// 16-byte entry of the motion group's (`+0x710`) table `+0x10`; 0 for
    /// no sequence, an end (0, 5) or an unknown code (printed through the
    /// empty `dbgPrintf`).
    pub fn ch_seq_get_fast_motion(mem: &impl Mem, chr: u32) -> u32 {
        let mut p: u32 = mem.read(chr + 0x714);
        if p == 0 {
            return 0;
        }
        loop {
            match mem.read::<i8>(p) {
                1 => {
                    let m = read_i16u(mem, p + 1);
                    let g: u32 = mem.read(chr + 0x710);
                    let t: u32 = mem.read(g + 0x10);
                    return (m << 4).cast_unsigned().wrapping_add(t);
                }
                2 => p = p.wrapping_add(1),
                3 | 6 => p = p.wrapping_add(3),
                4 => {
                    let g: u32 = mem.read(chr + 0x70c);
                    let k = read_i16u(mem, p + 1);
                    let tab: u32 = mem.read(g + 8);
                    let off = i32::from(mem.read::<i16>(tab.wrapping_add((2 * k).cast_unsigned())));
                    p = off.cast_unsigned().wrapping_add(mem.read::<u32>(g + 0xc));
                }
                _ => return 0,
            }
        }
    }

    /// `ChN_ExistSystemMGRP(key, b)` (`0x00836850`): whether
    /// `SG_FindMotionGroup(0, b, key)` (passed in) finds one.
    pub fn chn_exist_system_mgrp<M: Mem>(
        mem: &mut M,
        key: u32,
        b: u32,
        find: impl FnOnce(&mut M, i32, i32, u32) -> u32,
    ) -> u32 {
        u32::from(find(mem, 0, b.cast_signed(), key) != 0)
    }

    /// `Cache_CheckYet(kind, key)` (`0x0083ffe0`, the renderer's): 2 while
    /// the resource is not read.
    pub const CACHE_CHECK_YET: u32 = 0x0083_ffe0;
    /// `Cache_ToTop`, `Cache_Use`, `Cache_Exist3` (`0x00840cf0`,
    /// `0x00840dd0`, `0x00840790`).
    pub const CACHE_TO_TOP: u32 = 0x0084_0cf0;
    pub const CACHE_USE: u32 = 0x0084_0dd0;
    pub const CACHE_EXIST3: u32 = 0x0084_0790;
    /// `Cache_Exist_ClothVertexBuff` (`0x00840850`).
    pub const CACHE_EXIST_CLOTH: u32 = 0x0084_0850;
    /// `yiAssert(cond, title, fmt, ...)` (`0x0090ae50`).
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    /// The motion registry's cache key (kind 2).
    pub const REG_MOTION_KEY: u32 = 0x0009_03e7;

    /// `Ch_DataReadSync(id)` (`0x0082a040`): `Cache_CheckYet(1, id)`; for
    /// 2 (not read) `yiAssert(0, "CH Data Read Sync", "%s does not
    /// read.(can't sync)", ch_create_name(id))`. Returns the state.
    pub fn ch_data_read_sync<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = call(mem, mem.rebase(CACHE_CHECK_YET), &[1, id]);
        if r == 2 {
            let name = crate::texanim::bridge::ch_create_name(mem, id);
            let (title, fmt) = (mem.rebase(0x00b5_96ec), mem.rebase(0x00b5_96cc));
            call(mem, mem.rebase(YI_ASSERT), &[0, title, fmt, name]);
        }
        r
    }

    /// `ChN_ReadSystemMGRPsync(a, b)` (`0x00836a50`): `Cache_CheckYet(2,
    /// b << 16 | a)`, 0 for 2 (not read yet).
    pub fn chn_read_system_mgrp_sync<M: Mem>(
        mem: &mut M,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = call(mem, mem.rebase(CACHE_CHECK_YET), &[2, (b << 16) | a]);
        if r == 2 { 0 } else { r }
    }

    /// `Ch_RegMotionToTop()` (`0x00837780`), `Ch_RegMotionUse()`
    /// (`0x00837790`): `Cache_ToTop`/`Cache_Use(2, 0x903e7)`, their
    /// result.
    pub fn ch_reg_motion<M: Mem>(
        mem: &mut M,
        f: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, mem.rebase(f), &[2, REG_MOTION_KEY])
    }

    /// `Ch_RegMotionGetExistMem()` (`0x00837730`): whether
    /// `Cache_Exist3(2, 0x903e7)` finds it.
    pub fn ch_reg_motion_get_exist_mem<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        u32::from(call(mem, mem.rebase(CACHE_EXIST3), &[2, REG_MOTION_KEY]) != 0)
    }

    /// `Ch_UpdateClothHideFlag(chr)` (`0x00831480`): when the cloth vertex
    /// buffer of the character's id (`u16` `+0`) is cached, the character
    /// has a cloth (`+0x834`) and flag bit 21 (`+0x194`), the cloth's
    /// `+8` = the sign-extended hide byte `+0x180`. Returns what the
    /// original leaves in EAX: that value, or the cache's answer.
    pub fn ch_update_cloth_hide_flag<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cloth: u32 = mem.read(chr + 0x834);
        let id = u32::from(mem.read::<u16>(chr));
        let r = call(mem, mem.rebase(CACHE_EXIST_CLOTH), &[1, id]);
        if cloth != 0 && r != 0 && mem.read::<u32>(chr + 0x194) & 0x20_0000 != 0 {
            let v = i32::from(mem.read::<i8>(chr + 0x180)).cast_unsigned();
            mem.write(cloth + 8, v);
            return v;
        }
        r
    }

    /// `SetField0_006f2f70(p, v)` (`0x006f2f70`): `*p = v`, the old value.
    pub const SET_FIELD0: u32 = 0x006f_2f70;

    /// `Ch_DrawClothClear(chr)` (`0x00830b90`, ours; `Ch_DrawMain`'s):
    /// when the cloth vertex buffer of the character's id is cached, the
    /// character has a cloth (`+0x834`), flag bit 21 (`+0x194`) and the
    /// renderer is attached ([`crate::sys::bridge::DRAW_DETACHED`] 0): for
    /// each of the cloth's meshes (count `+0x14`, lists at `+0x30`) and
    /// the buffer's (at `+4`), each entry `e` of the mesh list (count
    /// `+0`) whose buffer entry is set and whose `+0x4c` is set gets
    /// `SetField0_006f2f70(e + 0x48, 0)`. Returns what the original leaves
    /// in EAX.
    pub fn ch_draw_cloth_clear<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = u32::from(mem.read::<u16>(chr));
        let cloth: u32 = mem.read(chr + 0x834);
        let buf = call(mem, mem.rebase(CACHE_EXIST_CLOTH), &[1, id]);
        if cloth == 0
            || buf == 0
            || mem.read::<u32>(chr + 0x194) & 0x20_0000 == 0
            || crate::sys::bridge::DRAW_DETACHED.get(mem) != 0
            || mem.read::<u32>(cloth + 0x14) == 0
        {
            return buf;
        }
        let mut i = 0u32;
        while i < mem.read::<u32>(cloth + 0x14) {
            let list: u32 = mem.read(cloth + 0x30 + 4 * i);
            let other: u32 = mem.read(buf + 4 + 4 * i);
            if list != 0 && other != 0 {
                let mut j = 0u32;
                while j < mem.read::<u32>(list) {
                    let e: u32 = mem.read(list + 4 + 4 * j);
                    if e != 0
                        && mem.read::<u32>(other + 4 + 4 * j) != 0
                        && mem.read::<u32>(e + 0x4c) != 0
                    {
                        call(mem, mem.rebase(SET_FIELD0), &[e + 0x48, 0]);
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        buf.wrapping_add(4).wrapping_add(4 * i)
    }

    /// `AtelGetCurCtrlWork()` (`0x0086aff0`, the script module's).
    pub const CUR_CTRL_WORK: u32 = 0x0086_aff0;
    /// `g_BtlStatus`: not 0 in battle.
    pub const BATTLE_STATUS: ffx_bridge::Global<u8> = ffx_bridge::Global::new(0x0112_a8e0);

    /// `Ch_AnimCopyNear(chr)` (`0x00839200`, ours; `SG_ApplyBones`'s): 1
    /// outside battle, when the current script control work's bit 1 is
    /// clear and the positions `+0x684` and `+0x694` (x, y, z) are less
    /// than 2.0 apart on each axis (NaN: not); else 0. In AL.
    pub fn ch_anim_copy_near<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u8 {
        let w = call(mem, mem.rebase(CUR_CTRL_WORK), &[]);
        if BATTLE_STATUS.get(mem) != 0 || mem.read::<u8>(w) & 2 != 0 {
            return 0;
        }
        let near = (0..3).all(|k| {
            let a = Reg::load(mem.read(chr + 0x684 + 4 * k)).get();
            let b = Reg::load(mem.read(chr + 0x694 + 4 * k)).get();
            (a - b).abs() < 2.0
        });
        u8::from(near)
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
