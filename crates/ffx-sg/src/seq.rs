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
}
