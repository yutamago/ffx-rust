//! Pieces of the serialized-object fixup reader (`Fixup_*`): LEB128 counts
//! and the object slot setters.

/// The low word `__allshl` (`0x00949910`) gives for `x << count`, with
/// `count` taken as its low byte: 0 from 32 up.
#[must_use]
pub const fn allshl_low(x: u32, count: u32) -> u32 {
    let c = count & 0xff;
    if c < 32 { x << c } else { 0 }
}

/// `Fixup_ReadLeb128`'s value: 7 bits per byte, low first, while the top
/// bit is set; `byte(i)` gives byte `i`. Returns the value and the bytes
/// read. Shifts past 31 bits lose their bits, and the shift count wraps at
/// 256 like the original's (`cl`).
pub fn leb128(mut byte: impl FnMut(u32) -> u8) -> (u32, u32) {
    let mut v = 0;
    let mut n = 0;
    loop {
        let b = byte(n);
        v |= allshl_low(u32::from(b & 0x7f), n.wrapping_mul(7));
        n += 1;
        if b & 0x80 == 0 {
            return (v, n);
        }
    }
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// `Fixup_ReadLeb128(&cursor)` (`0x0060ac00`): reads at the cursor and
    /// moves it past the bytes read (one store per byte, as the original);
    /// 0 for a null cursor, which stays.
    pub fn read_leb128(mem: &mut impl Mem, cursor: u32) -> u32 {
        let mut p: u32 = mem.read(cursor);
        if p == 0 {
            return 0;
        }
        let mut v = 0;
        let mut n = 0u32;
        loop {
            let b: u8 = mem.read(p);
            p = p.wrapping_add(1);
            mem.write(cursor, p);
            v |= super::allshl_low(u32::from(b & 0x7f), n.wrapping_mul(7));
            n += 1;
            if b & 0x80 == 0 {
                return v;
            }
        }
    }

    /// `Fixup_WriteLeb128(value, &cursor)` (`0x0060aba0`): `value` as
    /// LEB128 at the cursor (7 bits per byte, low first, the top bit on all
    /// but the last; one byte for 0), the cursor moved past each byte as it
    /// is written. Nothing for a null cursor.
    pub fn write_leb128(mem: &mut impl Mem, value: u32, cursor: u32) {
        let p: u32 = mem.read(cursor);
        if p == 0 {
            return;
        }
        let mut v = value;
        loop {
            let low = (v & 0x7f) as u8;
            v >>= 7;
            let at: u32 = mem.read(cursor);
            mem.write(at, if v == 0 { low } else { low | 0x80 });
            mem.write(cursor, at.wrapping_add(1));
            if v == 0 {
                return;
            }
        }
    }

    /// `Fixup_SetObject(record, index)` (`0x0060ae40`) and
    /// `Fixup_SetArrayObject` (`0x0060ae30`): `record + 4 = index`.
    pub fn set_object(mem: &mut impl Mem, record: u32, index: u32) {
        mem.write(record.wrapping_add(4), index);
    }

    /// `Fixup_ReadRecord(&cursor, flags)` (`0x0060e730`, `thiscall` on the
    /// record, `ret 8`): LEB128 words at the cursor into the record's
    /// words: 6 (minus one, -1 for 0) unless flag 0x10, and when that word
    /// was 0 or skipped, 2, then 3 unless 0x20 and 4 unless 0x40; always 5
    /// unless 0x08. Returns what `eax` ends as (the last word read, or word
    /// 6's value).
    pub fn read_record(mem: &mut impl Mem, record: u32, cursor: u32, flags: u32) -> u32 {
        let mut eax = 0;
        let mut rest = true;
        if flags & 0x10 == 0 {
            let v = read_leb128(mem, cursor);
            eax = v.wrapping_sub(1) | if v == 0 { u32::MAX } else { 0 };
            mem.write(record.wrapping_add(0x18), eax);
            rest = v == 0;
        }
        if rest {
            for (word, skip) in [(8, 0), (0xc, 0x20), (0x10, 0x40)] {
                if flags & skip == 0 {
                    eax = read_leb128(mem, cursor);
                    mem.write(record.wrapping_add(word), eax);
                }
            }
        }
        if flags & 0x08 == 0 {
            eax = read_leb128(mem, cursor);
            mem.write(record.wrapping_add(0x14), eax);
        }
        eax
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leb128_values() {
        let at = |s: &'static [u8]| move |i: u32| s[i as usize];
        assert_eq!(leb128(at(&[0x05])), (5, 1));
        assert_eq!(leb128(at(&[0x80, 0x01])), (0x80, 2));
        assert_eq!(
            leb128(at(&[0xff, 0xff, 0xff, 0xff, 0x7f])),
            (0xffff_ffff, 5)
        );
        assert_eq!(allshl_low(1, 7 * 37), 1 << 3);
    }
}
