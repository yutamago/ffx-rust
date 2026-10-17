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

    /// `user_free_thunk` and `Cache_Unuse`, still called as the game's.
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const CACHE_UNUSE: u32 = 0x0084_0d70;
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
            ffx_chara::sys::bridge::ch_mem_move_set_trigger(h);
        }
        let end = |h: &mut _| {
            let n = CHR_COUNT.get(h);
            CHR_ARRAY.get(h).wrapping_add(n.wrapping_mul(0x880))
        };
        let mut p = CHR_ARRAY.get(h);
        let mut stop = end(h);
        while p != stop {
            if h.read::<u8>(p.wrapping_add(2)) != 0 && h.read::<u32>(p.wrapping_add(0x7e8)) == group {
                ffx_chara::motion::bridge::ch_motion_set_null(h, p);
            }
            p = p.wrapping_add(0x880);
            stop = end(h);
        }
        stop
    }
}
