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

    /// `Ch_SetSequence(chr, seq)` (`0x00837ac0`).
    pub const SET_SEQUENCE: u32 = 0x0083_7ac0;
    /// `Ch_TextureSetAnim(chr, a, b)` (`0x0083d040`).
    pub const TEXTURE_SET_ANIM: u32 = 0x0083_d040;
    /// `i8` per slot: the system motion set `Ch_SetSysMotion` uses.
    pub const SYS_MOTION_SETS: ffx_bridge::Global<i8> = ffx_bridge::Global::new(0x00c4_9784);

    /// `Ch_SetSysMotion(chr, slot, motion)` (`0x00837d00`): starts the
    /// sequence for a system motion of the character's definition
    /// (`+0x1c4`). With bit 0 of the definition's `+0x70`: the sequence
    /// table of the slot's set ([`SYS_MOTION_SETS`], 8-byte entries from
    /// `+0x10` of `+0x6c`) indexed by `motion` (in slot 1, motion 0x10 on a
    /// character with flag bit 9 of `+0x198` first asks
    /// `Ch_TextureSetAnim(chr, 4, 1)`). Else the slot's list at `+0x108`
    /// (count `+8 + 8 * slot`, `u16` pairs (motion, sequence) at `+0xc`),
    /// the sequence key `definition +0 << 16 | sequence`. A miss in a slot
    /// other than 0 retries slot 0 (motion 0 in the table form). Returns
    /// what the original leaves in EAX.
    pub fn ch_set_sys_motion<M: Mem>(
        mem: &mut M,
        chr: u32,
        slot: u32,
        motion: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let def: u32 = mem.read(chr + 0x1c4);
        let set_of = |mem: &M, d: u32, s: u32| -> (u32, u32) {
            let tab: u32 = mem.read(d + 0x6c);
            let k = i32::from(mem.read::<i8>(SYS_MOTION_SETS.ptr(mem).addr.wrapping_add(s)));
            (
                tab,
                mem.read(tab.wrapping_add((8 * k).cast_unsigned()).wrapping_add(0x10)),
            )
        };
        if mem.read::<u8>(def + 0x70) & 1 != 0 {
            let (mut slot, mut motion) = (slot, motion);
            let (mut eax, mut set) = set_of(mem, def, slot);
            while set != 0 {
                if mem.read::<u32>(chr + 0x198) & 0x200 != 0 && slot == 1 && motion == 0x10 {
                    eax = call(mem, mem.rebase(TEXTURE_SET_ANIM), &[chr, 4, slot]);
                }
                let seq: u32 = mem.read(set.wrapping_add(4 * motion));
                if seq != 0 {
                    return call(mem, mem.rebase(SET_SEQUENCE), &[chr, seq]);
                }
                if slot == 0 {
                    return eax;
                }
                let d: u32 = mem.read(chr + 0x1c4);
                (eax, set) = set_of(mem, d, 0);
                slot = 0;
                motion = 0;
            }
            return eax;
        }
        let list: u32 = mem.read(def + 0x108);
        if list == 0 {
            return def;
        }
        let mut slot = slot;
        loop {
            let e = list.wrapping_add(8 * slot);
            let count: u32 = mem.read(e + 8);
            let pairs: u32 = mem.read(e + 0xc);
            let mut i = 0u32;
            while i < count {
                if u32::from(mem.read::<u16>(pairs.wrapping_add(4 * i))) == motion {
                    let s = u32::from(mem.read::<u16>(pairs.wrapping_add(4 * i + 2)));
                    let key = s | (mem.read::<u32>(def) << 16);
                    return call(mem, mem.rebase(SET_SEQUENCE), &[chr, key]);
                }
                i += 1;
            }
            if slot == 0 {
                return i;
            }
            slot = 0;
        }
    }

    /// `Sg_MemSetOwn(p, owner)` (`0x0083c550`), `Cache_Lock` (`0x00840960`).
    pub const MEM_SET_OWN: u32 = 0x0083_c550;
    pub const CACHE_LOCK: u32 = 0x0084_0960;

    /// `Ch_RegMotionEntryCache(p)` (`0x008376b0`): the battle idling
    /// motions at `p` into the cache: named `"[PC Battle Idling]"`, type 3,
    /// owner 6, entry (2, 0x903e7) used, locked and not yet read. Returns
    /// `Cache_Entry_NotYet`'s result.
    pub fn ch_reg_motion_entry_cache<M: Mem>(
        mem: &mut M,
        p: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let h = crate::handle::bridge::MEM_SET_STR;
        let k = REG_MOTION_KEY;
        call(mem, mem.rebase(h), &[p, mem.rebase(0x00b5_9a68), u32::MAX]);
        call(mem, mem.rebase(crate::sys::bridge::MEM_SET_TYPE), &[p, 3]);
        call(mem, mem.rebase(MEM_SET_OWN), &[p, 6]);
        call(mem, mem.rebase(crate::sys::bridge::CACHE_ENTRY), &[2, k, p]);
        call(mem, mem.rebase(CACHE_USE), &[2, k]);
        call(mem, mem.rebase(CACHE_LOCK), &[2, k]);
        call(
            mem,
            mem.rebase(crate::sys::bridge::CACHE_ENTRY_NOT_YET),
            &[2, k, 1],
        )
    }

    /// `Ch_RomReadChrCB(request)` (`0x0082a6c0`): the read finished:
    /// `Cache_Entry_NotYet(request +0x14, request +0xc, 0)`.
    pub fn ch_rom_read_chr_cb<M: Mem>(
        mem: &mut M,
        req: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (kind, key) = (mem.read(req + 0x14), mem.read(req + 0xc));
        call(
            mem,
            mem.rebase(crate::sys::bridge::CACHE_ENTRY_NOT_YET),
            &[kind, key, 0],
        )
    }

    /// `Ch_MotionCancel(chr)` (`0x00837180`): `Ch_SetSysMotion(chr, slot
    /// i8 +0x185, 0)`, its result.
    pub fn ch_motion_cancel<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slot = i32::from(mem.read::<i8>(chr + 0x185)).cast_unsigned();
        call(mem, mem.rebase(0x0083_7d00), &[chr, slot, 0])
    }

    /// `SG_StepChannels(obj, frames)` (`0x008394d0`, the main module's).
    pub const STEP_CHANNELS: u32 = 0x0083_94d0;

    /// `Ch_MotionSetupFcurve(chr, clip)` (`0x00839980`): sets up the
    /// character's channels (0x2c bytes each from `+0x748`) for a clip:
    /// frames `+0x7e0` = `i16` clip `+4`; events (`u16` `+6`, data at
    /// clip + `+0x10`) into `+0x7f0`/`+0x7f4` (0 without). Nine channels
    /// per bone, for the fewer of the clip's (`i16` `+2`) and the header's
    /// (`u16` `+10` of `+0x1c0`) bones, each typed by the next 2 bits of
    /// the 64-bit words at clip + `+8`: 0 the value 0, 1 the value 1, 2 a
    /// constant (the next `i16` of the data at clip + `+0xc`, 12-bit
    /// fixed), 3 a stream there (rotation for the first 3 of the 9; length
    /// `i16`, so the data moves by `len / 2` halfwords), linked in front of
    /// `+0x74c`. Then `SG_StepChannels(chr, 1)`, each streamed channel's
    /// previous value `+0x18` = its value `+0x16`, and `+0x7dc` = 1.
    /// Returns what the original leaves in EAX.
    pub fn ch_motion_setup_fcurve<M: Mem>(
        mem: &mut M,
        chr: u32,
        clip: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if clip == 0 {
            return 0;
        }
        let mut data = clip.wrapping_add(mem.read::<u32>(clip + 0xc));
        let mut words = clip.wrapping_add(mem.read::<u32>(clip + 8));
        let bones = i32::from(mem.read::<i16>(clip + 2));
        mem.write(chr + 0x7e0, i32::from(mem.read::<i16>(clip + 4)));
        let mut ch: u32 = mem.read(chr + 0x748);
        let events: u16 = mem.read(clip + 6);
        if events == 0 {
            mem.write(chr + 0x7f4, 0u32);
        } else {
            mem.write(chr + 0x7f0, events);
            mem.write(chr + 0x7f4, clip.wrapping_add(mem.read::<u32>(clip + 0x10)));
        }
        let hdr: u32 = mem.read(chr + 0x1c0);
        let have = i32::from(mem.read::<u16>(hdr + 0xa));
        let n = bones.min(have).wrapping_mul(9);
        let (mut bits, mut left, mut head) = (0u64, 0u32, 0u32);
        let mut i = 0i32;
        while i < n {
            if left == 0 {
                let lo: u32 = mem.read(words);
                let hi: u32 = mem.read(words + 4);
                bits = u64::from(lo) | (u64::from(hi) << 32);
                words = words.wrapping_add(8);
                left = 0x20;
            }
            #[allow(clippy::cast_possible_truncation)] // 2 bits
            let kind = (bits & 3) as u8;
            mem.write(ch + 4, kind);
            match kind {
                0 => mem.write(ch + 0x1c, 0f32),
                1 => mem.write(ch + 0x1c, 1f32),
                2 => {
                    let v = crate::fixed::fixed12_to_float(u32::from(mem.read::<u16>(data)));
                    data = data.wrapping_add(2);
                    mem.write(ch + 0x1c, v);
                }
                _ => {
                    mem.write(ch + 5, u8::from(i.cast_unsigned() % 9 < 3));
                    mem.write(ch + 8, data.wrapping_add(2));
                    mem.write(ch + 0xc, data.wrapping_add(2));
                    mem.write(ch + 6, 0u16);
                    mem.write(ch + 0x14, 0u32);
                    mem.write(ch, head);
                    let len = i32::from(mem.read::<i16>(data));
                    head = ch;
                    data = data.wrapping_add(((len / 2) * 2).cast_unsigned());
                }
            }
            bits >>= 2;
            mem.write(ch + 0x28, 1u8);
            left -= 1;
            ch = ch.wrapping_add(0x2c);
            i += 1;
        }
        mem.write(chr + 0x74c, head);
        let mut eax = call(mem, mem.rebase(STEP_CHANNELS), &[chr, 1]);
        let mut c: u32 = mem.read(chr + 0x74c);
        while c != 0 {
            let v: u16 = mem.read(c + 0x16);
            mem.write(c + 0x18, v);
            eax = (eax & 0xffff_0000) | u32::from(v);
            c = mem.read(c);
        }
        mem.write(chr + 0x7dc, 1u32);
        eax
    }

    /// `Ch_MotionSetupFcurve(chr, clip)` ([`ch_motion_setup_fcurve`]).
    pub const SETUP_FCURVE: u32 = 0x0083_9980;
    /// `g_ChrStrC001` (`"c001"`) and `g_GraphicMapId`.
    pub const STR_C001: u32 = 0x00b4_86bc;
    pub const GRAPHIC_MAP_ID: ffx_bridge::Global<i32> = ffx_bridge::Global::new(0x0133_0844);

    /// `Ch_SetMotionSeq(chr, clip, loops, start, end)` (`0x00839b80`):
    /// plays `clip` from `start` to `end` (negative: the clip's length,
    /// `i16` `+0` × 256; Tidus, `"c001"`, on map 0x7a playing 0x3000 to
    /// -0x100 stops at 0x2800): `+0x7e0` = 0x1e00, the clip's channels set
    /// up (`Ch_MotionSetupFcurve`), `+0x718` start, `+0x720` length,
    /// `+0x71c` end − 1, `+0x724` the pending loop count `+0x726` (or
    /// `loops` when none, -1), which is cleared; `+0x754` = 0x100, `+0x740`
    /// = start, `+0x752` = `+0x73a` (at least 1), flag bit 16 of `+0x194`
    /// cleared, `+0x728` = 1. Returns `+0x73a` (what EAX holds).
    pub fn ch_set_motion_seq<M: Mem>(
        mem: &mut M,
        chr: u32,
        clip: u32,
        loops: u32,
        start: i32,
        end: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let name: u32 = mem.read(chr + 4);
        let tidus = crate::handle::bridge::c_string(mem, name)
            == crate::handle::bridge::c_string(mem, mem.rebase(STR_C001));
        let mut end = end;
        if tidus && GRAPHIC_MAP_ID.get(mem) == 0x7a && start == 0x3000 && end == -0x100 {
            end = 0x2800;
        }
        mem.write(chr + 0x7e0, 0x1e00u32);
        call(mem, mem.rebase(SETUP_FCURVE), &[chr, clip]);
        mem.write(chr + 0x718, start);
        if end < 0 {
            end = i32::from(mem.read::<i16>(clip)) << 8;
        }
        mem.write(chr + 0x720, end.wrapping_sub(start));
        mem.write(chr + 0x71c, end.wrapping_sub(1));
        let pending: u16 = mem.read(chr + 0x726);
        #[allow(clippy::cast_possible_truncation)] // the low half, as stored
        let n = if pending == 0xffff {
            loops as u16
        } else {
            pending
        };
        mem.write(chr + 0x724, n);
        mem.write(chr + 0x726, 0xffffu16);
        mem.write(chr + 0x754, 0x100u16);
        let s: u32 = mem.read(chr + 0x718);
        mem.write(chr + 0x740, s);
        let k: u16 = mem.read(chr + 0x73a);
        mem.write(chr + 0x752, if k == 0 { 1 } else { k });
        let f: u32 = mem.read(chr + 0x194);
        mem.write(chr + 0x194, f & 0xfffe_ffff);
        mem.write(chr + 0x728, 1u16);
        u32::from(k)
    }

    /// `Ch_SetSysMotion(chr, slot, motion)` (`0x00837d00`),
    /// `Sg_AngleApproach(cur, target, step)` (`0x00835f80`, `ST0` out),
    /// `Angle_Wrap_00836720` (`ST0` out) and the water height
    /// (`f32`, `Sg_GetSuimen`).
    pub const SET_SYS_MOTION: u32 = 0x0083_7d00;
    pub const ANGLE_APPROACH: u32 = 0x0083_5f80;
    pub const ANGLE_WRAP: u32 = 0x0083_6720;
    pub const SUIMEN: u32 = 0x0130_09d8;

    /// The automatic motion step (`0x00835d30`, called for characters the
    /// script released, `Native_ChrReleaseMotion`): with flag bit 10 of
    /// `+0x194` the facing `+0x158` turns toward `+0x168` (step by speed
    /// `+0x154` against `+0x170`: faster, standing or walking); unless a
    /// motion is pending (`+0x736`), standing (`+0x154` 0) plays `+0x79c`,
    /// else `+0x7a8` above the water (y `+0x10` not above its height + 1)
    /// or `+0x7a4` below, as a sequence (low half from 0x1000) or system
    /// motion 2, and the run blend `+0x504` grows by 0.15 while below 1.
    /// The original's EAX is not a result.
    pub fn ch_auto_motion<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let fl = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        let bits = |mem: &M, a: u32| crate::x87::load(mem.read::<f32>(a)).to_bits();
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r32 = |x: f64| x as f32;
        if mem.read::<u32>(chr + 0x194) & 0x400 != 0 {
            let speed = fl(mem, chr + 0x154);
            let step = if fl(mem, chr + 0x170) < speed {
                0x00b5_99a8
            } else if speed == 0.0 {
                0x00b5_99a4
            } else {
                0x00b5_982c
            };
            let step = bits(mem, r(mem, step));
            let to = r32((fcall)(mem, r(mem, ANGLE_WRAP), &[bits(mem, chr + 0x168)]));
            let cur = r32((fcall)(mem, r(mem, ANGLE_WRAP), &[bits(mem, chr + 0x158)]));
            let v = (fcall)(
                mem,
                r(mem, ANGLE_APPROACH),
                &[cur.to_bits(), to.to_bits(), step],
            );
            mem.write(chr + 0x158, r32(v));
        }
        if mem.read::<u8>(chr + 0x736) != 0 {
            return;
        }
        let play = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, m: u32| {
            if m & 0xffff >= 0x1000 {
                call(mem, r(mem, SET_SEQUENCE), &[chr, m]);
            } else {
                call(mem, r(mem, SET_SYS_MOTION), &[chr, 2, m]);
            }
            mem.write(chr + 0x736, 0u8);
        };
        if fl(mem, chr + 0x154) == 0.0 {
            let m: u32 = mem.read(chr + 0x79c);
            play(mem, call, m);
            return;
        }
        let top = fl(mem, r(mem, SUIMEN)) + mem.read::<f64>(r(mem, 0x00b9_2248));
        let m: u32 = mem.read(
            chr + if top > fl(mem, chr + 0x10) {
                0x7a4
            } else {
                0x7a8
            },
        );
        play(mem, call, m);
        if 1.0 > fl(mem, chr + 0x504) {
            let v = r32(fl(mem, chr + 0x504) + mem.read::<f64>(r(mem, 0x00b5_99b0)));
            mem.write(chr + 0x504, v);
        }
    }

    /// `MsGetSaveEventAddress`'s thunk (`0x0088c830`), `Ch_ResetMotion`
    /// (`0x0082b080`), `Ch_SeqGetFastMotion` (`0x00837860`),
    /// `Ch_SetMotionSeq` (`0x00839b80`), `SG_DisposeMotionGroup` (`0x00836e20`),
    /// `SG_FindMotionGroup` (`0x00836f10`), `MsBattleCheck` (`0x00780c90`).
    pub const SAVE_EVENT_ADDRESS: u32 = 0x0088_c830;
    pub const RESET_MOTION: u32 = 0x0082_b080;
    pub const SEQ_GET_FAST_MOTION: u32 = 0x0083_7860;
    pub const SET_MOTION_SEQ: u32 = 0x0083_9b80;
    pub const DISPOSE_MOTION_GROUP: u32 = 0x0083_6e20;
    pub const FIND_MOTION_GROUP: u32 = 0x0083_6f10;
    pub const BATTLE_CHECK: u32 = 0x0078_0c90;
    /// The sequence `Ch_MotionSetLow2` leaves at `+0x714` and the two
    /// globals `0x008303a0` points at the save data's `+0xd2`.
    pub const LOW_SEQUENCE: u32 = 0x00c4_9789;
    pub const SAVE_D2_A: u32 = 0x022f_d6c0;
    pub const SAVE_D2_B: u32 = 0x023c_4450;
    /// The byte `SetByte_012fb8fd` sets.
    pub const BYTE_012FB8FD: u32 = 0x012f_b8fd;

    /// `Ch_MotionSetLow2(chr, motion, -, loops, start, end)` (`0x008373e0`):
    /// after `MsGetSaveEventAddress()` and `Ch_ResetMotion`, plays the
    /// sequence of `motion` (`Ch_SetSequence`; non-zero: -1) by its fast
    /// motion (`Ch_SeqGetFastMotion`, none: -1) as raw frames
    /// (`Ch_SetMotionSeq(chr, [+0xc], loops, start << 8, end << 8)`);
    /// `+0x736` = 1, `+0x714` = the fixed sequence, `+0x738` = 1. Returns 0.
    pub fn ch_motion_set_low2<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        chr_play_motion(mem, PlayFrames::Low2, args, call)
    }

    /// `ChN_DisposeSystemMGRP(a, b)` (`0x00836830`):
    /// `SG_DisposeMotionGroup(0, b, a)`'s result.
    pub fn chn_dispose_system_mgrp<M: Mem>(
        mem: &mut M,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, mem.rebase(DISPOSE_MOTION_GROUP), &[0, b, a])
    }

    /// `ChN_SystemMgrpLoaded()` (`0x00837710`): whether the system motion
    /// group (`SG_FindMotionGroup(2, 0, 0)`) is loaded, 1 or 0.
    pub fn chn_system_mgrp_loaded<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        u32::from(call(mem, mem.rebase(FIND_MOTION_GROUP), &[2, 0, 0]) != 0)
    }

    /// `Ch_ReadEventMgrp(x)` (`0x00837670`): `Ch_RomReadEventMGRPsync` of
    /// `AtelEventGetMotionIndex(0x0086c1c0(x))` (a tail call); its result.
    pub fn ch_read_event_mgrp<M: Mem>(
        mem: &mut M,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let v = call(mem, mem.rebase(0x0086_c1c0), &[x]);
        let i = call(mem, mem.rebase(0x0086_5890), &[v]);
        call(mem, mem.rebase(0x0082_a7f0), &[i])
    }

    /// `Ch_SaveMotionByte(chr)` (`0x008303a0`): points both globals at the
    /// save data's `+0xd2`; in battle, a character whose byte `+0x186` is
    /// neither 2 nor 3 gets that byte, else 0.
    pub fn ch_save_motion_byte<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let p = call(mem, mem.rebase(SAVE_EVENT_ADDRESS), &[]).wrapping_add(0xd2);
        mem.write(mem.rebase(SAVE_D2_A), p);
        mem.write(mem.rebase(SAVE_D2_B), p);
        if call(mem, mem.rebase(BATTLE_CHECK), &[]) == 0 {
            return 0;
        }
        if matches!(mem.read::<u8>(chr + 0x186), 2 | 3) {
            return 0;
        }
        u32::from(mem.read::<u8>(p))
    }

    /// `SetByte_012fb8fd(v)` (`0x00821b20`): the byte at [`BYTE_012FB8FD`].
    pub fn set_byte_012fb8fd<M: Mem>(mem: &mut M, v: u8) {
        mem.write(mem.rebase(BYTE_012FB8FD), v);
    }

    /// How a `Chr_PlayMotion*` variant takes its frames.
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum PlayFrames {
        /// `Ch_MotionSetLow2`: raw, after `MsGetSaveEventAddress()`.
        Low2,
        /// `Chr_PlayMotionInclEnd`: the end frame + 1 unless `loops` is 1
        /// or the end is not positive.
        InclEnd,
        /// `Chr_PlayMotionMarks`: indices into the fast motion's `i16`
        /// mark table (`+8`), the end as for `InclEnd`.
        Marks,
    }

    /// `Chr_PlayMotionInclEnd` (`0x008374c0`) and `Chr_PlayMotionMarks`
    /// (`0x00837350`), `(chr, motion, -, loops, start, end)`: as
    /// [`ch_motion_set_low2`] with their frames ([`PlayFrames`]).
    pub fn chr_play_motion<M: Mem>(
        mem: &mut M,
        how: PlayFrames,
        args: [u32; 6],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [chr, motion, _, loops, start, end] = args;
        if how == PlayFrames::Low2 {
            call(mem, mem.rebase(SAVE_EVENT_ADDRESS), &[]);
        }
        call(mem, mem.rebase(RESET_MOTION), &[chr]);
        if call(mem, mem.rebase(SET_SEQUENCE), &[chr, motion]) != 0 {
            return u32::MAX;
        }
        let fast = call(mem, mem.rebase(SEQ_GET_FAST_MOTION), &[chr]);
        if fast == 0 {
            return u32::MAX;
        }
        let (mut s, mut e) = (start.cast_signed(), end.cast_signed());
        if how == PlayFrames::Marks {
            let marks: u32 = mem.read(fast.wrapping_add(8));
            s = i32::from(mem.read::<i16>(marks.wrapping_add(start.wrapping_mul(2))));
            e = i32::from(mem.read::<i16>(marks.wrapping_add(end.wrapping_mul(2))));
        }
        if how != PlayFrames::Low2 && loops != 1 && e > 0 {
            e += 1;
        }
        let clip: u32 = mem.read(fast.wrapping_add(0xc));
        let (s, e) = ((s << 8).cast_unsigned(), (e << 8).cast_unsigned());
        call(mem, mem.rebase(SET_MOTION_SEQ), &[chr, clip, loops, s, e]);
        mem.write(chr + 0x736, 1u8);
        mem.write(chr + 0x714, mem.rebase(LOW_SEQUENCE));
        mem.write(chr + 0x738, 1u8);
        0
    }

    /// The loaded motion groups (`Ch_DisposeAllMGRP`): 50 entries of 12
    /// bytes, used when `+0` is not 0; `i16` kind `+4` (2 the system's),
    /// `i16` `+6`, key `+8`.
    pub const MGRP_TABLE: u32 = 0x0130_11e0;

    /// `Ch_DisposeMGRP(x)` (`0x00836ad0`): `SG_DisposeMotionGroup(1, x, 0)`.
    pub fn ch_dispose_mgrp<M: Mem>(
        mem: &mut M,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        call(mem, mem.rebase(DISPOSE_MOTION_GROUP), &[1, x, 0])
    }

    /// `Ch_DisposeAllMGRP()` (`0x00836a80`): disposes the first used group
    /// that is not the system's (`SG_DisposeMotionGroup(kind, +6, key)`)
    /// and starts over, until none is left. Returns what EAX holds (the
    /// table's end).
    pub fn ch_dispose_all_mgrp<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = mem.rebase(MGRP_TABLE);
        'again: loop {
            for k in 0..50u32 {
                let e = table + 12 * k;
                if mem.read::<u32>(e) != 0 && mem.read::<i16>(e + 4) != 2 {
                    let a = i32::from(mem.read::<i16>(e + 4)).cast_unsigned();
                    let b = i32::from(mem.read::<i16>(e + 6)).cast_unsigned();
                    let key: u32 = mem.read(e + 8);
                    call(mem, mem.rebase(DISPOSE_MOTION_GROUP), &[a, b, key]);
                    continue 'again;
                }
            }
            return table + 50 * 12 + 4;
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
