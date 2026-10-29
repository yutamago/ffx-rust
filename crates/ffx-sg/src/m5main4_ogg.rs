//! M5 stage 1, list `small`, round 4: libogg's bit packer as the video
//! player's Vorbis decoder links it (`oggpack_write`, its big-endian twin,
//! `oggpack_look`, `_adv`, `_read`), from the machine code.
//!
//! The buffer is five words: `+0` the end byte, `+4` the end bit, `+8` the
//! storage, `+0xc` the cursor into it, `+0x10` the storage size. The
//! storage grows through the allocator's `realloc` (`[0x00c8b244]`) and
//! is released through its `free` (`[0x00c8b240]`), both cdecl.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `libogg_mask`: `(1 << n) - 1` for n = 0..=32.
    pub const MASK: u32 = 0x00b8_f4b8;
    /// The allocator's `free` and `realloc` pointers.
    pub const FREE: u32 = 0x00c8_b240;
    pub const REALLOC: u32 = 0x00c8_b244;

    fn mask(h: &impl Host, bits: u32) -> u32 {
        h.read(at(h.rebase(MASK), bits.wrapping_mul(4)))
    }

    /// `x >> (n & 31)` and `x << (n & 31)`: the CPU's 32-bit shifts.
    fn shr(x: u32, n: u32) -> u32 {
        x >> (n & 31)
    }
    fn shl(x: u32, n: u32) -> u32 {
        x << (n & 31)
    }

    /// `shl r8, cl`: the byte shifted by `cl & 31` (0 from 8 on).
    fn shl8(x: u32, n: u32) -> u8 {
        let n = n & 31;
        if n >= 8 { 0 } else { byte(x << n) }
    }

    /// The low byte.
    #[allow(clippy::cast_possible_truncation)] // the byte stores
    fn byte(x: u32) -> u8 {
        x as u8
    }

    /// `x / 8`, toward zero (`cdq`, `and edx, 7`, `sar 3`).
    fn bytes_of(x: u32) -> u32 {
        (x.cast_signed() / 8).cast_unsigned()
    }

    /// The write's common head: `bits` over 32 or a failed grow end in
    /// [`fail`] (`None`); without storage it returns at once (`None`).
    fn grow(h: &mut impl Host, b: u32, bits: u32) -> Option<()> {
        if bits > 32 {
            fail(h, b);
            return None;
        }
        let storage: i32 = h.read(at(b, 0x10));
        if h.read::<i32>(b) >= storage.wrapping_sub(4) {
            if h.read::<u32>(at(b, 0xc)) == 0 {
                return None;
            }
            if storage > 0x7fff_feff {
                fail(h, b);
                return None;
            }
            let realloc: u32 = h.read(h.rebase(REALLOC));
            let old: u32 = h.read(at(b, 8));
            let p = h.call(
                realloc,
                Conv::Cdecl,
                &[old, storage.wrapping_add(0x100).cast_unsigned()],
            );
            if p == 0 {
                fail(h, b);
                return None;
            }
            let end: u32 = h.read(b);
            let n: u32 = h.read(at(b, 0x10));
            h.write(at(b, 0x10), n.wrapping_add(0x100));
            h.write(at(b, 8), p);
            h.write(at(b, 0xc), end.wrapping_add(p));
        }
        Some(())
    }

    /// The error path: the storage freed, the five words zeroed.
    fn fail(h: &mut impl Host, b: u32) {
        let p: u32 = h.read(at(b, 8));
        if p != 0 {
            let free: u32 = h.read(h.rebase(FREE));
            h.call(free, Conv::Cdecl, &[p]);
        }
        for o in [0, 4, 8, 0xc, 0x10] {
            h.write(at(b, o), 0u32);
        }
    }

    /// The write's common tail: the cursor past the whole bytes of
    /// `total` (end bit plus the bits written).
    fn advance(h: &mut impl Host, b: u32, total: u32) {
        let n = bytes_of(total);
        let end: u32 = h.read(b);
        h.write(b, end.wrapping_add(n));
        let ptr: u32 = h.read(at(b, 0xc));
        h.write(at(b, 0xc), ptr.wrapping_add(n));
        h.write(at(b, 4), total & 7);
    }

    /// `libogg_oggpack_write(b, value, bits)` (`0x0042e350`, cdecl): the
    /// low `bits` of `value` appended least significant bit first.
    pub fn write(h: &mut impl Host, b: u32, value: u32, bits: u32) {
        if grow(h, b, bits).is_none() {
            return;
        }
        let bit: u32 = h.read(at(b, 4));
        let ptr: u32 = h.read(at(b, 0xc));
        let v = value & mask(h, bits);
        let total = bits.wrapping_add(bit);
        let first: u8 = h.read(ptr);
        h.write(ptr, first | shl8(v, bit));
        let total_s = total.cast_signed();
        for (k, at_) in [(8i32, 1u32), (16, 2), (24, 3)] {
            if total_s < k {
                return advance(h, b, total);
            }
            let ptr: u32 = h.read(at(b, 0xc));
            h.write(
                at(ptr, at_),
                byte(shr(v, k.cast_unsigned().wrapping_sub(bit))),
            );
        }
        if total_s >= 32 {
            let ptr: u32 = h.read(at(b, 0xc));
            let last = if bit == 0 {
                0
            } else {
                byte(shr(v, 32u32.wrapping_sub(bit)))
            };
            h.write(at(ptr, 4), last);
        }
        advance(h, b, total);
    }

    /// `FUN_0042e480(b, value, bits)` (cdecl): libogg's `oggpackB_write`,
    /// the big-endian packer (the value's top bits first).
    pub fn write_b(h: &mut impl Host, b: u32, value: u32, bits: u32) {
        if grow(h, b, bits).is_none() {
            return;
        }
        let ptr: u32 = h.read(at(b, 0xc));
        let v = shl(mask(h, bits) & value, 32u32.wrapping_sub(bits));
        let bit: u32 = h.read(at(b, 4));
        let total = bits.wrapping_add(bit);
        let first: u8 = h.read(ptr);
        h.write(ptr, first | byte(shr(v, bit.wrapping_add(24))));
        let total_s = total.cast_signed();
        for (k, at_, by) in [(8i32, 1u32, 16u32), (16, 2, 8), (24, 3, 0)] {
            if total_s < k {
                return advance(h, b, total);
            }
            let bit: u32 = h.read(at(b, 4));
            let ptr: u32 = h.read(at(b, 0xc));
            h.write(at(ptr, at_), byte(shr(v, bit.wrapping_add(by))));
        }
        if total_s >= 32 {
            let bit: u32 = h.read(at(b, 4));
            let ptr: u32 = h.read(at(b, 0xc));
            let last = if bit == 0 {
                0
            } else {
                shl8(v, 8u32.wrapping_sub(bit))
            };
            h.write(at(ptr, 4), last);
        }
        advance(h, b, total);
    }

    /// The next `bits` (after the end bit `bit`), little-endian, from the
    /// cursor, before the mask.
    fn gather(h: &impl Host, ptr: u32, bit: u32, total: i32) -> u32 {
        let mut v = shr(u32::from(h.read::<u8>(ptr)), bit);
        for (k, i) in [(8i32, 1u32), (16, 2), (24, 3)] {
            if total <= k {
                return v;
            }
            v |= shl(
                u32::from(h.read::<u8>(at(ptr, i))),
                k.cast_unsigned().wrapping_sub(bit),
            );
        }
        if total > 32 && bit != 0 {
            v |= shl(u32::from(h.read::<u8>(at(ptr, 4))), 32u32.wrapping_sub(bit));
        }
        v
    }

    /// The bits `total` reaches past the end: `None` to read on, else the
    /// result (`-1` past the end, 0 for nothing to read).
    fn bounds(h: &impl Host, b: u32, total: i32) -> Option<u32> {
        let end: i32 = h.read(b);
        let storage: i32 = h.read(at(b, 0x10));
        if end < storage.wrapping_sub(4) {
            return None;
        }
        if end > storage.wrapping_sub(total.wrapping_add(7) >> 3) {
            return Some(u32::MAX);
        }
        if total == 0 { Some(0) } else { None }
    }

    /// `libogg_oggpack_look(b, bits)` (`0x0042e840`, fastcall): the next
    /// `bits` without moving; -1 past the end or for `bits` over 32.
    pub fn look(h: &impl Host, b: u32, bits: u32) -> u32 {
        if bits > 32 {
            return u32::MAX;
        }
        let m = mask(h, bits);
        let bit: u32 = h.read(at(b, 4));
        let total = bits.wrapping_add(bit).cast_signed();
        if let Some(r) = bounds(h, b, total) {
            return r;
        }
        let ptr: u32 = h.read(at(b, 0xc));
        gather(h, ptr, bit, total) & m
    }

    /// `libogg_oggpack_adv(b, bits)` (`0x0042ea30`, fastcall): skips
    /// `bits`; past the end the cursor is cleared, the end byte set to
    /// the storage size and the end bit to 1.
    pub fn adv(h: &mut impl Host, b: u32, bits: u32) {
        let end: i32 = h.read(b);
        let total = bits.wrapping_add(h.read::<u32>(at(b, 4)));
        let storage: i32 = h.read(at(b, 0x10));
        if end > storage.wrapping_sub(total.cast_signed().wrapping_add(7) >> 3) {
            past_end(h, b);
            return;
        }
        let n = bytes_of(total);
        let ptr: u32 = h.read(at(b, 0xc));
        h.write(at(b, 0xc), ptr.wrapping_add(n));
        h.write(at(b, 4), total & 7);
        h.write(b, n.wrapping_add(end.cast_unsigned()));
    }

    fn past_end(h: &mut impl Host, b: u32) {
        let storage: u32 = h.read(at(b, 0x10));
        h.write(at(b, 0xc), 0u32);
        h.write(b, storage);
        h.write(at(b, 4), 1u32);
    }

    /// `libogg_oggpack_read(b, bits)` (`0x0042eb10`, fastcall): the next
    /// `bits`, moving past them; -1 past the end or for `bits` over 32
    /// (the buffer then marked as past its end, as `adv` does).
    pub fn read(h: &mut impl Host, b: u32, bits: u32) -> u32 {
        if bits > 32 {
            past_end(h, b);
            return u32::MAX;
        }
        let m = mask(h, bits);
        let bit: u32 = h.read(at(b, 4));
        let total = bits.wrapping_add(bit);
        match bounds(h, b, total.cast_signed()) {
            Some(0) => return 0,
            Some(_) => {
                past_end(h, b);
                return u32::MAX;
            }
            None => {}
        }
        let ptr: u32 = h.read(at(b, 0xc));
        let v = gather(h, ptr, bit, total.cast_signed());
        let n = bytes_of(total);
        h.write(at(b, 0xc), ptr.wrapping_add(n));
        let end: u32 = h.read(b);
        h.write(b, end.wrapping_add(n));
        h.write(at(b, 4), total & 7);
        v & m
    }
}
