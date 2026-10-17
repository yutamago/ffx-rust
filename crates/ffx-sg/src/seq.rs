//! Character sequences (`SG_*`): relocating a loaded sequence's pointers
//! and stepping its animation channels.
//!
//! A channel (a linked list from an object's `+0x74c`, next at `+0`)
//! holds a byte stream at `+0xc`, a wait count `u16` at `+6`, the
//! per-frame delta `i16` at `+0x14`, the value `u16` at `+0x16` and the
//! previous value at `+0x18`. Each frame the value moves by the delta;
//! when no wait is pending the next stream byte is read first:
//!
//! - `0xxxxxxx`: a new delta, 7-bit signed;
//! - `10nnnnnn`: wait `n` frames (the delta stays);
//! - `11llllll hhhhhhhh`: a new delta, 14-bit signed (`h` high, `l` low).

#![allow(clippy::many_single_char_names)] // counts, pointers

/// The delta of a one-byte code (`0xxxxxxx`): bit 6 is the sign.
#[must_use]
pub fn delta7(b: u8) -> u16 {
    i16::from((b << 1).cast_signed() >> 1).cast_unsigned()
}

/// The delta of a two-byte code: `hi` (sign-extended) above the six
/// bits of `lo`, as 14-bit signed.
#[must_use]
pub fn delta14(lo: u8, hi: u8) -> u16 {
    let v = (i16::from(hi.cast_signed()).cast_unsigned() << 6) | u16::from(lo & 0x3f);
    ((v << 2).cast_signed() >> 2).cast_unsigned()
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{delta7, delta14};

    /// `SG_RelocateSequence(seq, base)` (`0x00836fc0`): adds `base` to the
    /// two table pointers (`+0xc`, `+0x10`) and to both pointers (`+8`,
    /// `+0xc`) of each 16-byte entry of the tables (`i16` counts at `+8`
    /// and `+0xa`; a table with no entries gets a null pointer). The
    /// count is read again for each entry, as in the original. Returns
    /// `seq`.
    pub fn relocate_sequence(mem: &mut impl Mem, seq: u32, base: u32) -> u32 {
        if seq == 0 {
            return seq;
        }
        for table in [0xcu32, 0x10] {
            let at = seq.wrapping_add(table);
            let p: u32 = mem.read(at);
            mem.write(at, p.wrapping_add(base));
        }
        for (count, table) in [(8u32, 0xcu32), (0xa, 0x10)] {
            let n: i16 = mem.read(seq.wrapping_add(count));
            if n == 0 {
                mem.write(seq.wrapping_add(table), 0u32);
                continue;
            }
            let mut i: i32 = 0;
            if i32::from(n) <= 0 {
                continue;
            }
            loop {
                let t: u32 = mem.read(seq.wrapping_add(table));
                #[allow(clippy::cast_sign_loss)] // i is from 0
                let e = t.wrapping_add(16 * i as u32);
                for off in [8u32, 0xc] {
                    let p: u32 = mem.read(e.wrapping_add(off));
                    mem.write(e.wrapping_add(off), p.wrapping_add(base));
                }
                i += 1;
                let n: i16 = mem.read(seq.wrapping_add(count));
                if i >= i32::from(n) {
                    break;
                }
            }
        }
        seq
    }

    /// `SG_StepChannels(obj, frames)` (`0x008394d0`): each channel of the
    /// object steps `frames` times (none when not above 0). Returns 0 (the
    /// end of the list).
    pub fn step_channels(mem: &mut impl Mem, obj: u32, frames: i32) -> u32 {
        let mut ch: u32 = mem.read(obj.wrapping_add(0x74c));
        while ch != 0 {
            if frames > 0 {
                let mut wait: u16 = mem.read(ch.wrapping_add(6));
                let mut delta: u16 = mem.read(ch.wrapping_add(0x14));
                let mut value: u16 = mem.read(ch.wrapping_add(0x16));
                for _ in 0..frames {
                    if wait == 0 {
                        let p: u32 = mem.read(ch.wrapping_add(0xc));
                        let b: u8 = mem.read(p);
                        mem.write(ch.wrapping_add(0xc), p.wrapping_add(1));
                        if b & 0x80 == 0 {
                            delta = delta7(b);
                            mem.write(ch.wrapping_add(0x14), delta);
                        } else if b & 0x40 != 0 {
                            let hi: u8 = mem.read(p.wrapping_add(1));
                            mem.write(ch.wrapping_add(0xc), p.wrapping_add(2));
                            delta = delta14(b, hi);
                            mem.write(ch.wrapping_add(0x14), delta);
                        } else {
                            wait = u16::from(b & 0x3f);
                            mem.write(ch.wrapping_add(6), wait);
                        }
                    } else {
                        wait -= 1;
                        mem.write(ch.wrapping_add(6), wait);
                    }
                    mem.write(ch.wrapping_add(0x18), value);
                    value = value.wrapping_add(delta);
                    mem.write(ch.wrapping_add(0x16), value);
                }
            }
            ch = mem.read(ch);
        }
        0
    }

    /// `SG_RelocateGroup(group)` (`0x00836f70`): a motion group moved since
    /// it was built (its first word, its own address then, differs):
    /// the sequence table (`+0xc`) and every sequence in it (20 bytes each,
    /// `i32` count at `+4`, read again for each) move by the difference.
    pub fn relocate_group(mem: &mut impl Mem, group: u32) {
        if group == 0 {
            return;
        }
        let delta = group.wrapping_sub(mem.read::<u32>(group));
        if delta == 0 {
            return;
        }
        let t: u32 = mem.read(group.wrapping_add(0xc));
        mem.write(group.wrapping_add(0xc), t.wrapping_add(delta));
        mem.write(group, group);
        let mut i = 0i32;
        while i < mem.read::<i32>(group.wrapping_add(4)) {
            let t: u32 = mem.read(group.wrapping_add(0xc));
            relocate_sequence(mem, t.wrapping_add(20 * i.cast_unsigned()), delta);
            i += 1;
        }
    }

    /// `SG_RewindChannels(obj)` (`0x008398f0`): every channel back to its
    /// stream's start (`+8`), wait and delta 0, value 0 (the old value kept
    /// at `+0x1a`), one step, the previous value set to the kept one; then
    /// the object's `+0x7dc` to 1.
    pub fn rewind_channels(mem: &mut impl Mem, obj: u32) {
        let first: u32 = mem.read(obj.wrapping_add(0x74c));
        let mut ch = first;
        while ch != 0 {
            let start: u32 = mem.read(ch.wrapping_add(8));
            mem.write(ch.wrapping_add(0xc), start);
            let v: u16 = mem.read(ch.wrapping_add(0x16));
            mem.write(ch.wrapping_add(0x1a), v);
            mem.write(ch.wrapping_add(6), 0u16);
            mem.write(ch.wrapping_add(0x14), 0u32);
            ch = mem.read(ch);
        }
        step_channels(mem, obj, 1);
        let mut ch: u32 = mem.read(obj.wrapping_add(0x74c));
        while ch != 0 {
            let v: u16 = mem.read(ch.wrapping_add(0x1a));
            mem.write(ch.wrapping_add(0x18), v);
            ch = mem.read(ch);
        }
        mem.write(obj.wrapping_add(0x7dc), 1u32);
    }

    /// The loaded motion groups: 50 entries of `{group, i16, i16, key}`.
    pub const MOTION_GROUPS: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_11e0);
    /// `g_ChrSequences`: 100 entries of 20 bytes, the sequence first.
    pub const SEQUENCES: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0130_0a08);

    /// `SG_FindMotionGroup(a, b, key)` (`0x00836f10`): the first loaded
    /// group with those two numbers and key, else 0.
    pub fn find_motion_group(mem: &impl Mem, a: i32, b: i32, key: u32) -> u32 {
        let base = MOTION_GROUPS.ptr(mem).addr;
        for k in 0..50 {
            let e = base.wrapping_add(12 * k);
            let group: u32 = mem.read(e);
            if group != 0
                && i32::from(mem.read::<i16>(e.wrapping_add(4))) == a
                && i32::from(mem.read::<i16>(e.wrapping_add(6))) == b
                && mem.read::<u32>(e.wrapping_add(8)) == key
            {
                return group;
            }
        }
        0
    }

    /// `SG_RemoveMotionGroup(group)` (`0x00836da0`): clears its entry.
    pub fn remove_motion_group(mem: &mut impl Mem, group: u32) {
        if group == 0 {
            return;
        }
        let base = MOTION_GROUPS.ptr(mem).addr;
        for k in 0..50 {
            let e = base.wrapping_add(12 * k);
            if mem.read::<u32>(e) == group {
                mem.write(e, 0u32);
                return;
            }
        }
    }

    /// `SG_RemoveSequence(seq)` (`0x00836de0`): clears its entry (a null
    /// `seq` finds the first free one) and links the table again
    /// (`Ch_SeqLink`).
    pub fn remove_sequence(mem: &mut impl Mem, seq: u32) {
        let base = SEQUENCES.ptr(mem).addr;
        for k in 0..100 {
            let e = base.wrapping_add(20 * k);
            if mem.read::<u32>(e) == seq {
                mem.write(e, 0u32);
                ffx_chara::motion::bridge::ch_seq_link(mem);
                return;
            }
        }
    }

    /// `user_free_thunk` and `Cache_Unuse`, called as the game's (with
    /// `Ch_MemMoveSetTrigger`).
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const CACHE_UNUSE: u32 = 0x0084_0d70;
    /// `Ch_MemMoveSetTrigger`.
    pub const MEM_MOVE_SET_TRIGGER: u32 = 0x0082_7250;
    /// `g_ChrCount`, `g_ChrArray` (`0x880` bytes each).
    pub const CHR_COUNT: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x023c_44e0);
    pub const CHR_ARRAY: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x023c_44e4);

    /// `SG_DisposeMotionGroup(a, b, key)` (`0x00836e20`): the loaded group
    /// (none: returns 0) loses its sequences (`+4` of them, 20 bytes each
    /// at `+0xc`) and its entry. A character group (`a` 0) or the system
    /// group (`a` 2) goes back to the cache (`Cache_Unuse(2, b << 16 |
    /// key)`, `0x903e7` for the system one); any other is freed (`user_free`)
    /// and the memory move triggered. Every active character still on it
    /// gets no motion (`Ch_MotionSetNull`). Returns the end of the
    /// character array (`eax` after the loop).
    pub fn dispose_motion_group(h: &mut impl ffx_bridge::Host, a: i32, b: i32, key: u32) -> u32 {
        let group = find_motion_group(h, a, b, key);
        if group == 0 {
            return 0;
        }
        let mut k = 0u32;
        while k.cast_signed() < h.read::<i32>(group.wrapping_add(4)) {
            let seqs: u32 = h.read(group.wrapping_add(0xc));
            remove_sequence(h, seqs.wrapping_add(20 * k));
            k += 1;
        }
        remove_motion_group(h, group);
        let cached = match a {
            0 => Some((b.cast_unsigned() << 16) | key),
            2 => Some(0x0009_03e7),
            _ => None,
        };
        if let Some(id) = cached {
            let at = h.rebase(CACHE_UNUSE);
            h.call(at, &[2, id]);
        } else {
            let at = h.rebase(USER_FREE);
            h.call(at, &[group]);
            let at = h.rebase(MEM_MOVE_SET_TRIGGER);
            h.call(at, &[]);
        }
        let end = |h: &mut _| {
            let n = CHR_COUNT.get(h);
            CHR_ARRAY.get(h).wrapping_add(n.wrapping_mul(0x880))
        };
        let mut p = CHR_ARRAY.get(h);
        let mut stop = end(h);
        while p != stop {
            if h.read::<u8>(p.wrapping_add(2)) != 0 && h.read::<u32>(p.wrapping_add(0x7e8)) == group
            {
                ffx_chara::motion::bridge::ch_motion_set_null(h, p);
            }
            p = p.wrapping_add(0x880);
            stop = end(h);
        }
        stop
    }

    /// `SG_ApplyBone`'s constants (`f64` 2, pi as `f32` in an `f64`, 4096)
    /// and the damping ones (`f32` 0.7 and 10).
    pub const TWO: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_2270);
    pub const PI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_82f0);
    pub const UNITS: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_15c8);
    pub const DAMP_X: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b2_607c);
    pub const DAMP_K: ffx_bridge::Global<f32> = ffx_bridge::Global::new(0x00b9_2318);

    /// `SG_ApplyBone(_, chan, bone, scale)` (`0x00838280`) and, `damped`,
    /// `SG_ApplyBoneDamped` (`0x00838470`). Nine channels of `0x2c` bytes
    /// (value at `+0x20`, changed flag at `+0x28`, cleared when read):
    /// rotations as `Angle_Wrap(v * 2 * pi)` into the bone's targets
    /// `+0x114..` (and at once its current `+0xe4..`), translations as `v *
    /// scale * 4096` into `+0x124..` (and `+0xf4..`), scales copied to
    /// `+0x134..` and `+0x104..`. Damped, the current rotations and
    /// translations then move toward the targets
    /// (`Chr_LerpAngleDamped(target, cur, 0.7, 0, 10)`, `Chr_LerpDamped(target,
    /// cur, 1, 0, 1)`) instead of jumping.
    pub fn apply_bone(mem: &mut impl Mem, chan: u32, bone: u32, scale: f32, damped: bool) {
        use ffx_bridge::x87;
        let f = |b: u32| f32::from_bits(b);
        let (two, pi, units) = (TWO.get(mem), PI.get(mem), UNITS.get(mem));
        let scale = x87::load(scale.to_bits());
        for k in 0..9u32 {
            let c = chan.wrapping_add(0x2c * k);
            if mem.read::<u8>(c.wrapping_add(0x28)) == 0 {
                continue;
            }
            mem.write(c.wrapping_add(0x28), 0u8);
            let v = x87::load(mem.read::<u32>(c.wrapping_add(0x20)));
            let (r, slot) = match k {
                0..3 => {
                    let a = f(x87::store(x87::mul(x87::mul(v, two), pi)));
                    (crate::vec::bridge::angle_wrap(a), k)
                }
                3..6 => (f(x87::store(x87::mul(x87::mul(v, scale), units))), k + 1),
                _ => (f(x87::store(v)), k + 2),
            };
            mem.write(bone.wrapping_add(0x114 + 4 * slot), r);
            if !damped || k >= 6 {
                mem.write(bone.wrapping_add(0xe4 + 4 * slot), r);
            }
        }
        if !damped {
            return;
        }
        let (dx, dk) = (DAMP_X.get(mem), DAMP_K.get(mem));
        let rt = x87::round_trip;
        for slot in [0u32, 1, 2, 4, 5, 6] {
            let target = rt(mem.read(bone.wrapping_add(0x114 + 4 * slot)));
            let cur = rt(mem.read(bone.wrapping_add(0xe4 + 4 * slot)));
            let r = if slot < 3 {
                ffx_chara::motion::lerp_angle_damped(target, cur, rt(dx), 0.0, rt(dk))
            } else {
                ffx_chara::motion::lerp_damped(target, cur, 1.0, 0.0, 1.0)
            };
            mem.write(bone.wrapping_add(0xe4 + 4 * slot), r);
        }
    }

    /// `yiAssert`, called as the game's.
    pub const YI_ASSERT: u32 = 0x0090_ae50;

    /// `SG_AddSequence(seq, v)` (`0x00836d30`): the first free entry of
    /// `g_ChrSequences` takes `seq`, its `u16` at `+4` and `v` (at `+0x10`);
    /// a full table asserts (`yiAssert(0, "SG:AddMseq", "nb mseq err")`).
    /// Either way the table is linked again (`Ch_SeqLink`, a tail call).
    pub fn add_sequence(h: &mut impl ffx_bridge::Host, seq: u32, v: u32) {
        let base = SEQUENCES.ptr(h).addr;
        let free = (0..100u32)
            .map(|k| base.wrapping_add(20 * k))
            .find(|&e| h.read::<u32>(e) == 0);
        if let Some(e) = free {
            let n: u16 = h.read(seq.wrapping_add(4));
            h.write(e.wrapping_add(4), n);
            h.write(e, seq);
            h.write(e.wrapping_add(0x10), v);
        } else {
            let at = h.rebase(YI_ASSERT);
            let file = h.rebase(0x00b5_9a24);
            let text = h.rebase(0x00b5_9a18);
            h.call(at, &[0, file, text]);
        }
        ffx_chara::motion::bridge::ch_seq_link(h);
    }

    /// `SG_ApplyBoneSel(a, chan, bone, scale)` (`0x00838240`):
    /// [`apply_bone`], damped when the bone's byte `+0x144` is set (`scale`
    /// passed on through `fld`/`fstp`).
    pub fn apply_bone_sel(mem: &mut impl Mem, chan: u32, bone: u32, scale: f32) {
        let damped = mem.read::<u8>(bone.wrapping_add(0x144)) != 0;
        apply_bone(mem, chan, bone, ffx_bridge::x87::round_trip(scale), damped);
    }

    /// `SG_SampleChannels(obj)` (`0x008393c0`): the key streams stepped to
    /// the motion time and their values interpolated. The time `+0x740`
    /// (8.8 frames, kept in `+0x744`) rounded up past the frame
    /// (`(t + 0x1ff) >> 8`) is clamped to the end (`(+0x71c + 1) >> 8`)
    /// for a motion that stops (`u16` `+0x724` 1), else wrapped back by the
    /// loop length (`+0x720 >> 8`). Going back rewinds the channels
    /// (`SG_RewindChannels`); then they step to that frame
    /// (`SG_StepChannels`, the frame count `+0x7dc`). Each channel of the
    /// list `+0x74c` then gets at `+0x1c` its key (`Fixed12_ToFloat` of
    /// `+0x16`) or, between frames (`t & 0xff`), the blend with the key
    /// before (`+0x18`): `AngleTurn_Lerp` for an angle (byte `+5`), else
    /// `prev + Fixed8_ToFloat(t & 0xff) * (key - prev)`.
    pub fn sample_channels(mem: &mut impl Mem, obj: u32) {
        use ffx_bridge::x87;
        use ffx_chara::fixed::{fixed8_to_float, fixed12_to_float};
        let at = |o: u32| obj.wrapping_add(o);
        let t: u32 = mem.read(at(0x740));
        let end = mem.read::<i32>(at(0x71c)).wrapping_add(1) >> 8;
        mem.write(at(0x744), t);
        let mut want = (t.wrapping_add(0x1ff)).cast_signed() >> 8;
        if want > end {
            if mem.read::<u16>(at(0x724)) == 1 {
                want = end;
            } else {
                want = want.wrapping_sub(mem.read::<i32>(at(0x720)) >> 8);
            }
        }
        let mut d = want.wrapping_sub(mem.read::<i32>(at(0x7dc)));
        if d < 0 {
            rewind_channels(mem, obj);
            d = want.wrapping_sub(1);
        }
        if want > mem.read::<i32>(at(0x7dc)) {
            step_channels(mem, obj, d);
            let n: i32 = mem.read(at(0x7dc));
            mem.write(at(0x7dc), n.wrapping_add(d));
        }
        let f = t & 0xff;
        let mut ch: u32 = mem.read(at(0x74c));
        while ch != 0 {
            let key: u16 = mem.read(ch.wrapping_add(0x16));
            let v = if f == 0 {
                fixed12_to_float(u32::from(key))
            } else {
                let prev: u16 = mem.read(ch.wrapping_add(0x18));
                if mem.read::<u8>(ch.wrapping_add(5)) != 0 {
                    let s = |x: u16| i32::from(x.cast_signed()).cast_unsigned();
                    ffx_chara::motion::angle_turn_lerp(s(prev), s(key), f)
                } else {
                    let a = x87::load(fixed12_to_float(u32::from(prev)).to_bits());
                    let b = x87::load(fixed12_to_float(u32::from(key)).to_bits());
                    let diff = x87::sub(b, a);
                    let w = x87::load(fixed8_to_float(f).to_bits());
                    f32::from_bits(x87::store(x87::add(x87::mul(w, diff), a)))
                }
            };
            mem.write(ch.wrapping_add(0x1c), v);
            ch = mem.read(ch);
        }
    }

    /// One linear step toward the target over `n` frames: `cur +=
    /// (target - cur) / n` unless the difference (stored as `f32`) is 0;
    /// a NaN difference steps.
    fn step_toward(mem: &mut impl Mem, target: u32, cur: u32, n: i32) {
        use ffx_bridge::x87;
        let t = x87::load(mem.read(target));
        let c = x87::load(mem.read(cur));
        let d = x87::load(x87::store(x87::sub(t, c)));
        if d == 0.0 {
            return;
        }
        let nf = x87::load(x87::store(f64::from(n)));
        let r = x87::add(x87::div(d, nf), c);
        mem.write(cur, f32::from_bits(x87::store(r)));
    }

    /// `SG_BlendToTarget(obj, n)` (`0x008395b0`): one of `n` steps from the
    /// current pose to the target. A channel motion (`+0x196` bit 0) steps
    /// each channel of `+0x74c` (`+0x20` toward `+0x1c`: `Chr_FracStep` for
    /// an angle, byte `+5`, else linearly) and marks it changed (`+0x28`).
    /// Otherwise each of the model's bones (`u16` `+0xa` of `+0x1c0`;
    /// `0x18c` bytes each at `+0x748`) steps its three rotations
    /// (`Chr_FracStep`) and six other values linearly, marking its nine
    /// channels changed; with `n` 1, `+0x194` gets bit 16.
    pub fn blend_to_target(mem: &mut impl Mem, obj: u32, n: i32) {
        use ffx_bridge::x87::round_trip as rt;
        use ffx_chara::motion::bridge::chr_frac_step;
        let frac = |mem: &mut _, cur: u32, target: u32| {
            let (c, t) = (rt(Mem::read(mem, cur)), rt(Mem::read(mem, target)));
            let r = chr_frac_step(mem, c, t, n);
            Mem::write(mem, cur, r);
        };
        if mem.read::<u8>(obj.wrapping_add(0x196)) & 1 != 0 {
            let mut ch: u32 = mem.read(obj.wrapping_add(0x74c));
            while ch != 0 {
                if mem.read::<u8>(ch.wrapping_add(5)) == 0 {
                    step_toward(mem, ch.wrapping_add(0x1c), ch.wrapping_add(0x20), n);
                } else {
                    frac(mem, ch.wrapping_add(0x20), ch.wrapping_add(0x1c));
                }
                mem.write(ch.wrapping_add(0x28), 1u8);
                ch = mem.read(ch);
            }
            return;
        }
        let count = |mem: &_| {
            let model: u32 = Mem::read(mem, obj.wrapping_add(0x1c0));
            i32::from(Mem::read::<u16>(mem, model.wrapping_add(0xa)))
        };
        let mut b = mem.read::<u32>(obj.wrapping_add(0x748)).wrapping_add(0x20);
        let mut i = 0;
        while i < count(mem) {
            for o in [0u32, 0x2c, 0x58] {
                frac(mem, b.wrapping_add(o), b.wrapping_add(o).wrapping_sub(4));
            }
            for o in [0x80u32, 0xac, 0xd8, 0x104, 0x130, 0x15c] {
                step_toward(mem, b.wrapping_add(o), b.wrapping_add(o + 4), n);
            }
            for o in [0x168u32, 0x13c, 0x110, 0xe4, 0xb8, 0x8c, 0x60, 0x34, 0x8] {
                mem.write(b.wrapping_add(o), 1u8);
            }
            i += 1;
            b = b.wrapping_add(0x18c);
        }
        if n == 1 {
            let f: u32 = mem.read(obj.wrapping_add(0x194));
            mem.write(obj.wrapping_add(0x194), f | 0x1_0000);
        }
    }

    /// The nine `i16` channel slots of a bone's raw pose (`0x160`-byte
    /// bones at `obj+0x328`).
    const RAW_SLOTS: [u32; 9] = [
        0x146, 0x148, 0x14a, 0x14e, 0x150, 0x152, 0x156, 0x158, 0x15a,
    ];
    /// 1/4096 (`f64`).
    pub const RAW_SCALE_UNIT: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b5_0190);

    /// Copies `n` words from `bones + from` to `bones + to`.
    fn copy_words(mem: &mut impl Mem, bones: u32, from: u32, to: u32, n: u32) {
        for k in 0..n {
            let w: u32 = mem.read(bones.wrapping_add(from + 4 * k));
            mem.write(bones.wrapping_add(to + 4 * k), w);
        }
    }

    /// `SG_ApplyRawClip(obj, clip, t, t2)` (`0x00838750`): an uncompressed
    /// clip (`i16` frames per channel at `stride = clip[0]`, `clip[1]`
    /// bones, the data at `+0x10`, nine channels per bone) sampled at
    /// frame `t >> 8` and blended toward frame `t2 >> 8` by `t & 0xff`
    /// (`Angle12_Lerp` for the three rotations, `(b - a) * f >> 8` the
    /// rest) into the raw pose of each bone (through the bone map
    /// `+0x758` when there is one, `0xffff` skipping). Then, by `+0x7ec`, a
    /// few bones' raw values are copied to their neighbours (1: bones 24 to
    /// 25 and 26; 2: 42 to 43; 3: 42 to 43 and 44), and every bone's
    /// float pose follows: rotations `SG_RawToRad`, translations times the
    /// character's scale (`+0x38` of `+0x1c4`), scales / 4096.
    pub fn apply_raw_clip(mem: &mut impl Mem, obj: u32, clip: u32, t: u32, t2: u32) {
        if clip == 0 {
            return;
        }
        let stride = i32::from(mem.read::<i16>(clip)).cast_unsigned();
        let bones_in_clip = i32::from(mem.read::<i16>(clip.wrapping_add(2)));
        let data = clip.wrapping_add(0x10);
        let mut cur = data.wrapping_add(((t.cast_signed() >> 8) * 2).cast_unsigned());
        let next = data.wrapping_add(((t2.cast_signed() >> 8) * 2).cast_unsigned());
        let delta = next.wrapping_sub(cur);
        let model: u32 = mem.read(obj.wrapping_add(0x1c0));
        let count = i32::from(mem.read::<u16>(model.wrapping_add(0xa)));
        let n = bones_in_clip.min(count);
        let f = t & 0xff;
        let mut off = 0u32;
        let mut map_at = 8u32;
        for _ in 0..n.max(0) {
            let map: u32 = mem.read(obj.wrapping_add(0x758));
            let bone = if map == 0 {
                Some(mem.read::<u32>(obj.wrapping_add(0x328)).wrapping_add(off))
            } else {
                let idx = u32::from(mem.read::<u16>(map.wrapping_add(map_at)));
                (idx != 0xffff)
                    .then(|| (idx * 0x160).wrapping_add(mem.read::<u32>(obj.wrapping_add(0x328))))
            };
            if let Some(bone) = bone {
                let mut p = cur;
                for (k, slot) in (0u32..).zip(RAW_SLOTS) {
                    let a = i32::from(mem.read::<i16>(p));
                    let v = if f == 0 {
                        a.cast_unsigned()
                    } else {
                        let b = i32::from(mem.read::<i16>(p.wrapping_add(delta)));
                        if k < 3 {
                            ffx_chara::fixed::angle12_lerp(a, b, f.cast_signed()).cast_unsigned()
                        } else {
                            ((b - a).wrapping_mul(f.cast_signed()) >> 8)
                                .wrapping_add(a)
                                .cast_unsigned()
                        }
                    };
                    mem.write(bone.wrapping_add(slot), crate::lo16(v));
                    p = p.wrapping_add(stride.wrapping_mul(2));
                }
            }
            cur = cur.wrapping_add(stride.wrapping_mul(18));
            off = off.wrapping_add(0x160);
            map_at += 2;
        }
        let bones = |mem: &_| Mem::read::<u32>(mem, obj.wrapping_add(0x328));
        match mem.read::<u8>(obj.wrapping_add(0x7ec)) {
            1 => {
                let b = bones(mem);
                copy_words(mem, b, 0x2246, 0x23a6, 6);
                let to = bones(mem).wrapping_add(0x23c0).wrapping_sub(b);
                copy_words(mem, b, 0x2246, to.wrapping_add(0x146), 6);
            }
            2 => {
                let b = bones(mem);
                copy_words(mem, b, 0x3b06, 0x3c66, 6);
            }
            3 => {
                let b = bones(mem);
                copy_words(mem, b, 0x3b06, 0x3c66, 6);
                let to = bones(mem).wrapping_add(0x3c80).wrapping_sub(b);
                copy_words(mem, b, 0x3b06, to.wrapping_add(0x146), 6);
            }
            _ => {}
        }
        raw_pose_to_float(mem, obj);
    }

    /// `SG_ApplyRawClip`'s last part: every bone's raw pose as floats.
    fn raw_pose_to_float(mem: &mut impl Mem, obj: u32) {
        use ffx_bridge::x87;
        let bones = |mem: &_| Mem::read::<u32>(mem, obj.wrapping_add(0x328));
        let scale_at: u32 = mem.read(obj.wrapping_add(0x1c4));
        let scale = x87::round_trip(mem.read(scale_at.wrapping_add(0x38)));
        let unit = RAW_SCALE_UNIT.get(mem);
        let mut e = 0u32;
        let mut i = 0i32;
        let model_count = |mem: &_| {
            let m: u32 = Mem::read(mem, obj.wrapping_add(0x1c0));
            i32::from(Mem::read::<u16>(mem, m.wrapping_add(0xa)))
        };
        while i < model_count(mem) {
            let raw = |mem: &_, slot: u32| {
                let v = i32::from(Mem::read::<i16>(
                    mem,
                    bones(mem).wrapping_add(e).wrapping_add(slot),
                ));
                x87::load(x87::store(f64::from(v)))
            };
            for (k, slot) in (0u32..).zip([0x146u32, 0x148, 0x14a]) {
                let x = f32::from_bits(x87::store(raw(mem, slot)));
                let r = crate::vec::bridge::raw_to_rad(mem, x);
                let at = bones(mem).wrapping_add(e).wrapping_add(0xe4 + 4 * k);
                mem.write(at, r);
            }
            let sc = x87::load(scale.to_bits());
            for (slot, out) in [(0x14eu32, 0xf4u32), (0x150, 0xf8), (0x152, 0xfc)] {
                let v = f32::from_bits(x87::store(x87::mul(raw(mem, slot), sc)));
                let at = bones(mem).wrapping_add(e).wrapping_add(out);
                mem.write(at, v);
            }
            for (slot, out) in [(0x156u32, 0x104u32), (0x158, 0x108), (0x15a, 0x10c)] {
                let v = f32::from_bits(x87::store(x87::mul(raw(mem, slot), unit)));
                let at = bones(mem).wrapping_add(e).wrapping_add(out);
                mem.write(at, v);
            }
            i += 1;
            e = e.wrapping_add(0x160);
        }
    }

    /// `SG_AddMotionGroup(group, a, b, key)` (`0x00836c90`): a loaded
    /// group takes the first free motion-group entry (`group`, the `i16`
    /// numbers, `key`); a full table (50) asserts (`yiAssert(0, "SG:Add
    /// MGRP", "nb mgrp err")`). The group is relocated
    /// ([`relocate_group`]) and each of its sequences added
    /// ([`add_sequence`] with the group). A null group does nothing.
    pub fn add_motion_group(h: &mut impl ffx_bridge::Host, group: u32, a: u32, b: u32, key: u32) {
        if group == 0 {
            return;
        }
        let base = MOTION_GROUPS.ptr(h).addr;
        let free = (0..50u32)
            .map(|k| base.wrapping_add(12 * k))
            .find(|&e| h.read::<u32>(e) == 0);
        if let Some(e) = free {
            h.write(e.wrapping_add(4), crate::lo16(a));
            h.write(e.wrapping_add(6), crate::lo16(b));
            h.write(e, group);
            h.write(e.wrapping_add(8), key);
        } else {
            let at = h.rebase(YI_ASSERT);
            let file = h.rebase(0x00b5_9a3c);
            let text = h.rebase(0x00b5_9a30);
            h.call(at, &[0, file, text]);
        }
        relocate_group(h, group);
        let mut k = 0i32;
        while k < h.read::<i32>(group.wrapping_add(4)) {
            let seqs: u32 = h.read(group.wrapping_add(0xc));
            add_sequence(h, seqs.wrapping_add(20 * k.cast_unsigned()), group);
            k += 1;
        }
    }

    /// `Ch_AnimCopyNear`, `Ch_AnimCopy2` (results in `al`), called as the
    /// game's.
    pub const CH_ANIM_COPY_NEAR: u32 = 0x0083_9200;
    pub const CH_ANIM_COPY2: u32 = 0x0083_9140;
    /// `getScenerioFlag()` (`g_SaveScenario` as `u16`), called as the
    /// game's.
    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    /// `g_GraphicMapId`.
    pub const GRAPHIC_MAP_ID: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0133_0844);
    /// 0.001 (`f64`).
    pub const MILLI: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b4_7c68);

    /// `SG_ApplyBones(chr)` (`0x00838f70`): every bone of the model takes
    /// its channels ([`apply_bone`]; through the bone map `+0x758`, whose
    /// `u16` `+2` bounds the count and `0xffff` skips) at the character's
    /// scale (`+0x38` of `+0x1c4`) / 1000. How: damped when `+0x871` is
    /// set; for a character with `+0x870` set and `+0x820` bit 2 clear,
    /// near the camera (`Ch_AnimCopyNear`) damped when `Ch_AnimCopy2` says
    /// so and by the bone's own byte otherwise (`SG_ApplyBoneSel`), except
    /// on a few maps (`0x170`, `0x11d`; `0x50` for kind 4, `0x137` for kind
    /// `0x5045`, `100` for kinds other than 0, `0x3c` before scenario
    /// `0x9a`), which apply directly.
    pub fn apply_bones(h: &mut impl ffx_bridge::Host, chr: u32) {
        use ffx_bridge::x87;
        let at = |o: u32| chr.wrapping_add(o);
        let mut chan: u32 = h.read(at(0x748));
        let def: u32 = h.read(at(0x1c4));
        let base = x87::load(h.read(def.wrapping_add(0x38)));
        let flag = h.rebase(GET_SCENERIO_FLAG);
        h.call(flag, &[]);
        let scale = f32::from_bits(x87::store(x87::mul(base, MILLI.get(h))));
        let map: u32 = h.read(at(0x758));
        let map_count = if map == 0 {
            0
        } else {
            i32::from(h.read::<u16>(map.wrapping_add(2)))
        };
        let mode = if h.read::<u8>(at(0x871)) != 0 {
            1
        } else if h.read::<u8>(at(0x870)) == 0 || h.read::<u8>(at(0x820)) & 4 != 0 {
            0
        } else {
            let near = h.rebase(CH_ANIM_COPY_NEAR);
            let mode = if crate::lo8(h.call(near, &[chr])) == 0 {
                0
            } else {
                let copy2 = h.rebase(CH_ANIM_COPY2);
                if crate::lo8(h.call(copy2, &[chr])) == 0 {
                    2
                } else {
                    1
                }
            };
            let kind: u16 = h.read(chr);
            let direct = match GRAPHIC_MAP_ID.get(h) {
                0x170 | 0x11d => true,
                0x50 => kind == 4,
                0x137 => kind == 0x5045,
                0x64 => kind != 0,
                0x3c => crate::lo16(h.call(flag, &[])) < 0x9a,
                _ => false,
            };
            if direct { 0 } else { mode }
        };
        let count = |h: &_| {
            let model: u32 = Mem::read(h, chr.wrapping_add(0x1c0));
            i32::from(Mem::read::<u16>(h, model.wrapping_add(0xa)))
        };
        let mut k = 0i32;
        while k < count(h) {
            let map: u32 = h.read(at(0x758));
            let bones: u32 = h.read(at(0x328));
            let bone = if map == 0 {
                Some(bones.wrapping_add(0x160 * k.cast_unsigned()))
            } else {
                if k >= map_count {
                    return;
                }
                let idx = u32::from(h.read::<u16>(map.wrapping_add(8 + 2 * k.cast_unsigned())));
                (idx != 0xffff).then(|| (idx * 0x160).wrapping_add(bones))
            };
            if let Some(bone) = bone {
                match mode {
                    1 => apply_bone(h, chan, bone, scale, true),
                    2 => apply_bone_sel(h, chan, bone, scale),
                    _ => apply_bone(h, chan, bone, scale, false),
                }
            }
            k += 1;
            chan = chan.wrapping_add(0x18c);
        }
    }

    /// `Ch_SetMotionSeq(chr, clip, loops, start, end)` and `dbgPrintf`,
    /// called as the game's.
    pub const CH_SET_MOTION_SEQ: u32 = 0x0083_9b80;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// "unknown motion sequence code %x\n".
    pub const UNKNOWN_CODE_TEXT: u32 = 0x00b5_9aa4;

    /// `SG_SeqStep(chr)` (`0x00837900`): one opcode of the character's
    /// motion script (`+0x714`; `u16` operands read as
    /// `ReadU16Unaligned`, sign-extended bytes). 0 ends it (2), 5 holds it
    /// (3); 1 plays a clip (`Ch_SetMotionSeq`: the clip `[+0x710]+0x10` table
    /// entry's `+0xc`, loops, start and end marks from its `+8` table, the
    /// end one frame further unless either loop operand is 1); 2 waits for
    /// `+0x728` to be 0; 3 waits a count (`+0x72a`); 4 jumps (`+0x70c`'s
    /// `+8` table, from its `+0xc`); 6 waits until `+0x737` reaches the
    /// operand. Waiting returns 1 with the script unmoved, a done step 0;
    /// an unknown code is printed (`dbgPrintf`) and returns -1.
    pub fn seq_step(h: &mut impl ffx_bridge::Host, chr: u32) -> u32 {
        use crate::state::bridge::read_u16_unaligned as ru;
        let at = |o: u32| chr.wrapping_add(o);
        let p: u32 = h.read(at(0x714));
        let op = i32::from(h.read::<i8>(p));
        let i16v = |v: i32| i32::from(crate::lo16(v.cast_unsigned()).cast_signed());
        match op {
            0 | 5 => {
                h.write(at(0x714), p);
                if op == 0 { 2 } else { 3 }
            }
            1 => {
                let clip_i = i16v(ru(h, p.wrapping_add(1)));
                let table: u32 = h.read(h.read::<u32>(at(0x710)).wrapping_add(0x10));
                let entry = table.wrapping_add((clip_i << 4).cast_unsigned());
                let marks: u32 = h.read(entry.wrapping_add(8));
                let mark = |h: &_, i: i32| {
                    i32::from(Mem::read::<i16>(
                        h,
                        marks.wrapping_add((2 * i).cast_unsigned()),
                    ))
                };
                let start = mark(h, i16v(ru(h, p.wrapping_add(5))));
                let mut end = mark(h, i16v(ru(h, p.wrapping_add(7))));
                let loops = i16v(ru(h, p.wrapping_add(3)));
                let last = crate::lo16(ru(h, p.wrapping_add(7)).cast_unsigned());
                if last != 1 && loops != 1 {
                    end += 1;
                }
                let clip: u32 = h.read(entry.wrapping_add(0xc));
                let f = h.rebase(CH_SET_MOTION_SEQ);
                h.call(
                    f,
                    &[
                        chr,
                        clip,
                        loops.cast_unsigned(),
                        (start << 8).cast_unsigned(),
                        (end << 8).cast_unsigned(),
                    ],
                );
                h.write(at(0x714), p.wrapping_add(9));
                h.write(at(0x73e), 0u8);
                0
            }
            2 => {
                if h.read::<u16>(at(0x728)) == 0 {
                    h.write(at(0x714), p.wrapping_add(1));
                    0
                } else {
                    h.write(at(0x714), p);
                    1
                }
            }
            3 => {
                if h.read::<u16>(at(0x72a)) == 0 {
                    let v = crate::lo16(ru(h, p.wrapping_add(1)).cast_unsigned());
                    h.write(at(0x72a), v);
                }
                let n = h.read::<u16>(at(0x72a)).wrapping_sub(1);
                h.write(at(0x72a), n);
                let done = n == 0;
                h.write(at(0x714), if done { p.wrapping_add(3) } else { p });
                u32::from(!done)
            }
            4 => {
                let jumps: u32 = h.read(at(0x70c));
                let i = i16v(ru(h, p.wrapping_add(1)));
                let offs: u32 = h.read(jumps.wrapping_add(8));
                let d = i32::from(h.read::<i16>(offs.wrapping_add((2 * i).cast_unsigned())));
                let base: u32 = h.read(jumps.wrapping_add(0xc));
                h.write(at(0x714), d.cast_unsigned().wrapping_add(base));
                0
            }
            6 => {
                let v = crate::lo16(ru(h, p.wrapping_add(1)).cast_unsigned()).cast_signed();
                let cur = i16::from(h.read::<i8>(at(0x737)));
                if cur < v {
                    h.write(at(0x714), p);
                    1
                } else {
                    h.write(at(0x714), p.wrapping_add(3));
                    0
                }
            }
            _ => {
                let f = h.rebase(DBG_PRINTF);
                let text = h.rebase(UNKNOWN_CODE_TEXT);
                h.call(f, &[text, op.cast_unsigned()]);
                h.write(at(0x714), p);
                u32::MAX
            }
        }
    }

    /// `SG_InitBones`'s callees, called as the game's: `user_malloc`, the
    /// CRT `sprintf` (through its import slot), `Sg_MemSetStr`,
    /// `Sg_MemSetType`, `memclear`, `Ch_ElemInit`, `Ch_OptposGetElement`,
    /// `Chr_ModelEntry`, `Ch_ModelCreatePacket`.
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    pub const SG_MEM_SET_STR: u32 = 0x0083_c580;
    pub const SG_MEM_SET_TYPE: u32 = 0x0083_c590;
    pub const MEMCLEAR: u32 = 0x0062_f4e0;
    pub const CH_ELEM_INIT: u32 = 0x0082_8b60;
    pub const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
    pub const CHR_MODEL_ENTRY: u32 = 0x0082_8be0;
    pub const CH_MODEL_CREATE_PACKET: u32 = 0x0082_7280;
    /// The block names' formats: "%s:Elem", "%s:Fcv", "%s:Mdl",
    /// "%s:Collision".
    pub const NAME_ELEM: u32 = 0x00b5_92c0;
    pub const NAME_FCV: u32 = 0x00b5_92c8;
    pub const NAME_MDL: u32 = 0x00b5_92d0;
    pub const NAME_COLLISION: u32 = 0x00b5_92d8;
    /// `SG_InitBones`' constants (`f64` 100, 180, 1000; pi is [`PI`]).
    pub const HUNDRED: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_22e0);
    pub const ONE_EIGHTY: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b4_8378);
    pub const THOUSAND: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b0_dba0);

    /// One named, typed, cleared block (`user_malloc(size)` stored at
    /// `chr + field`, named `fmt` with the character's name, type 1, then
    /// `clear` bytes cleared). The name goes through the host's callee
    /// scratch, as the original's stack buffer did.
    fn named_block(h: &mut impl ffx_bridge::Host, chr: u32, field: u32, size: u32, fmt: u32, clear: u32) {
        let at = h.rebase(USER_MALLOC);
        let p = h.call(at, &[size]);
        h.write(chr.wrapping_add(field), p);
        let buf = h.callee_scratch().unwrap_or(0);
        let slot = h.rebase(SPRINTF_SLOT);
        let sprintf: u32 = h.read(slot);
        let name: u32 = h.read(chr.wrapping_add(4));
        let fmt = h.rebase(fmt);
        h.call(sprintf, &[buf, fmt, name]);
        let p: u32 = h.read(chr.wrapping_add(field));
        let at = h.rebase(SG_MEM_SET_STR);
        h.call(at, &[p, buf, u32::MAX]);
        let p: u32 = h.read(chr.wrapping_add(field));
        let at = h.rebase(SG_MEM_SET_TYPE);
        h.call(at, &[p, 1]);
        let p: u32 = h.read(chr.wrapping_add(field));
        let at = h.rebase(MEMCLEAR);
        h.call(at, &[p, clear]);
    }

    /// `SG_InitBones(chr)` (`0x008277f0`): a character's bones from its
    /// model (`+0x1c0`). Blocks: `u16` `+0xa` bones of `0x160` ("%s:Elem",
    /// `+0x328`) and of `0x18c` ("%s:Fcv", `+0x748`), `+6` models of `0x38`
    /// ("%s:Mdl", `+0x1bc`), and with `+0x2c` collisions `0x60` each
    /// ("%s:Collision", `+0x32c`; only `0x38` each cleared). Each bone and
    /// the root element (`+0x1c8`, its `+4` -1) are initialised
    /// (`Ch_ElemInit`). The bind pose from the model's bone table (`+0x1c`,
    /// 20 bytes each): the parent (itself meaning the root element), the
    /// rotations in hundredths of a degree (`Angle_Wrap(v / 100 * pi /
    /// 180)`), the translations / 1000 and the scales `>> 12` (rounding
    /// to 0). When option points 7 and 8 (`Ch_OptposGetElement`) have a
    /// common ancestor, that ancestor and its own ancestors lose their
    /// damping byte (`+0x144`), as do bones whose parent has none. Each
    /// model entry gets `Chr_ModelEntry(k, model+0x10, model+4)` and 1.0 at
    /// `+0x14`, `+0x194` bit 21 telling whether its `+0x18` is set; then
    /// `Ch_ModelCreatePacket`.
    pub fn init_bones(h: &mut impl ffx_bridge::Host, chr: u32) {
        use ffx_bridge::x87;
        let model: u32 = h.read(chr.wrapping_add(0x1c0));
        let count = |h: &_, o: u32| u32::from(Mem::read::<u16>(h, model.wrapping_add(o)));
        let n = count(h, 0xa);
        named_block(h, chr, 0x328, n * 0x160, NAME_ELEM, count(h, 0xa) * 0x160);
        named_block(h, chr, 0x748, count(h, 0xa) * 0x18c, NAME_FCV, count(h, 0xa) * 0x18c);
        named_block(h, chr, 0x1bc, count(h, 6) * 0x38, NAME_MDL, count(h, 6) * 0x38);
        let coll = count(h, 0x2c);
        if coll != 0 {
            named_block(h, chr, 0x32c, coll * 0x60, NAME_COLLISION, count(h, 0x2c) * 0x38);
        }
        let elem = h.rebase(CH_ELEM_INIT);
        let mut k = 0;
        while k < count(h, 0xa) {
            let bones: u32 = h.read(chr.wrapping_add(0x328));
            h.call(elem, &[bones.wrapping_add(0x160 * k)]);
            k += 1;
        }
        h.call(elem, &[chr.wrapping_add(0x1c8)]);
        h.write(chr.wrapping_add(0x1cc), u32::MAX);
        let (c100, pi, c180, c1000) = (HUNDRED.get(h), PI.get(h), ONE_EIGHTY.get(h), THOUSAND.get(h));
        let st = |x: f64| f32::from_bits(x87::store(x));
        let mut k = 0;
        while k < count(h, 0xa) {
            let table: u32 = h.read(model.wrapping_add(0x1c));
            let e = table.wrapping_add(20 * k);
            let bones: u32 = h.read(chr.wrapping_add(0x328));
            let bone = bones.wrapping_add(0x160 * k);
            h.write(bone.wrapping_add(4), k);
            let parent = u32::from(h.read::<u16>(e));
            let p = if parent == k {
                chr.wrapping_add(0x1c8)
            } else {
                (parent * 0x160).wrapping_add(h.read::<u32>(chr.wrapping_add(0x328)))
            };
            h.write(bone, p);
            let v = |h: &_, i: u32| f64::from(Mem::read::<i16>(h, e.wrapping_add(2 * i)));
            for i in 0..3u32 {
                let a = x87::load(x87::store(v(h, 1 + i)));
                let r = st(x87::div(x87::mul(x87::div(a, c100), pi), c180));
                h.write(bone.wrapping_add(0xe4 + 4 * i), crate::vec::bridge::angle_wrap(r));
            }
            for i in 0..3u32 {
                h.write(bone.wrapping_add(0xf4 + 4 * i), st(x87::div(v(h, 4 + i), c1000)));
            }
            for i in 0..3u32 {
                #[allow(clippy::cast_possible_truncation)] // an i16
                let s = (v(h, 7 + i) as i32) / 4096;
                h.write(bone.wrapping_add(0x104 + 4 * i), st(f64::from(s)));
            }
            k += 1;
        }
        let opt = h.rebase(CH_OPTPOS_GET_ELEMENT);
        let a = h.call(opt, &[chr, 7, 0]);
        let b = h.call(opt, &[chr, 8, 0]);
        if a != 0 && b != 0 {
            let bones: u32 = h.read(chr.wrapping_add(0x328));
            let mut x = bones.wrapping_add(a.wrapping_mul(0x160));
            let mut y = bones.wrapping_add(b.wrapping_mul(0x160));
            while x != y {
                x = h.read(x);
                y = h.read(y);
            }
            if x != 0 {
                h.write(x.wrapping_add(0x144), 0u8);
                let mut c = bones;
                let mut k = 0;
                while k < u32::from(h.read::<u16>(h.read::<u32>(chr.wrapping_add(0x1c0)).wrapping_add(0xa))) {
                    let parent: u32 = h.read(c);
                    if h.read::<u8>(parent.wrapping_add(0x144)) == 0 {
                        h.write(c.wrapping_add(0x144), 0u8);
                    }
                    k += 1;
                    c = c.wrapping_add(0x160);
                }
                while x != 0 {
                    h.write(x.wrapping_add(0x144), 0u8);
                    x = h.read(x);
                }
            }
        }
        let entry = h.rebase(CHR_MODEL_ENTRY);
        let mut k = 0;
        while k < u32::from(h.read::<u16>(h.read::<u32>(chr.wrapping_add(0x1c0)).wrapping_add(6))) {
            let base: u32 = h.read(model.wrapping_add(0x10));
            let version = u32::from(h.read::<u16>(model.wrapping_add(4)));
            let mdl: u32 = h.read(chr.wrapping_add(0x1bc));
            let r = h.call(entry, &[k, base, version]);
            let e = mdl.wrapping_add(0x38 * k);
            h.write(e.wrapping_add(0x14), 1.0f32);
            h.write(e, r);
            let f: u32 = h.read(chr.wrapping_add(0x194));
            let bit = u32::from(h.read::<u16>(r.wrapping_add(0x18)) != 0) << 21;
            h.write(chr.wrapping_add(0x194), ((bit | f) ^ f) & 0x20_0000 ^ f);
            k += 1;
        }
        let at = h.rebase(CH_MODEL_CREATE_PACKET);
        h.call(at, &[chr]);
    }
}
