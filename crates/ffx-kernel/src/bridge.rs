//! The kernel lookups where the game keeps the tables, through
//! [`ffx_bridge::Mem`] (`docs/rewrite-plan.md`, M1). A loaded table is the
//! file as it is (`docs/formats/kernel.md`): `i16` segment count at `+0`,
//! 12-byte segments from `+8` (first ID, last ID, stride, string offset as
//! `u16`, data offset as `u32`). [`crate::Table`] reads the same container
//! from bytes; these return the game's pointers, for callers still in
//! `FFX.exe`.

use ffx_bridge::{Global, Mem, Ptr};

/// `command` (player commands).
pub const COMMAND: Global<u32> = Global::new(0x0112_a92c);
/// `monmagic1` (monster commands, first table).
pub const MON_MAGIC1: Global<u32> = Global::new(0x0112_a930);
/// `monmagic2` (monster commands, second table).
pub const MON_MAGIC2: Global<u32> = Global::new(0x0112_a934);
/// `item`.
pub const ITEM: Global<u32> = Global::new(0x0112_a940);
/// `a_ability` (auto-abilities).
pub const A_ABILITY: Global<u32> = Global::new(0x0112_a944);
/// Battle resource 0xb (`MsBattleInit`), read by `MsGetRomCTBbase`.
pub const CTB_BASE: Global<u32> = Global::new(0x0112_a94c);
/// Battle resource 6 (`MsBattleInit`), read by `MsGetRomSummonGrow`.
pub const SUMMON_GROW: Global<u32> = Global::new(0x0112_a960);
/// Battle resource 5 (`MsBattleInit`), read by `MsGetRomKaizou`.
pub const KAIZOU: Global<u32> = Global::new(0x0112_a964);

/// `MsCheckRange(x, lo, hi)` (`0x0079a0c0`): `x` clamped, signed, low
/// bound first.
#[must_use]
pub fn ms_check_range(x: i32, lo: i32, hi: i32) -> i32 {
    let x = if x < lo { lo } else { x };
    if x > hi { hi } else { x }
}

/// `MsGetExcelData(id, table, strings)` (`0x007ab870`): the record of `id`;
/// when `strings` is not null, also stores where its segment's strings
/// start there. An ID outside every segment gets the first segment's first
/// record (and strings).
pub fn ms_get_excel_data(mem: &mut impl Mem, id: i32, table: u32, strings: u32) -> u32 {
    let count = i32::from(mem.read::<i16>(table));
    let mut seg = table + 8;
    let mut found = false;
    for _ in 0..count.max(0) {
        let first = i32::from(mem.read::<u16>(seg));
        let last = i32::from(mem.read::<u16>(seg + 2));
        if first <= id && id <= last {
            found = true;
            break;
        }
        seg += 12;
    }
    if !found {
        seg = table + 8;
    }
    let data = mem.read::<u32>(seg + 8);
    if strings != 0 {
        let s = u32::from(mem.read::<u16>(seg + 6))
            .wrapping_add(data)
            .wrapping_add(table);
        mem.write(strings, s);
    }
    if !found {
        return data.wrapping_add(table);
    }
    let first = u32::from(mem.read::<u16>(seg));
    let stride = u32::from(mem.read::<u16>(seg + 4));
    #[allow(clippy::cast_sign_loss)] // id >= first here
    let index = (id as u32).wrapping_sub(first);
    stride
        .wrapping_mul(index)
        .wrapping_add(data)
        .wrapping_add(table)
}

/// `MsGetExcelDataNum(table, segment)` (`0x007ab8f0`): records in segment
/// `segment`.
pub fn ms_get_excel_data_num(mem: &impl Mem, table: u32, segment: i32) -> i32 {
    #[allow(clippy::cast_sign_loss)] // the original indexes unsigned
    let seg = table.wrapping_add((segment as u32).wrapping_mul(12));
    i32::from(mem.read::<u16>(seg + 10)) - i32::from(mem.read::<u16>(seg + 8)) + 1
}

/// A record by the 12-bit ID in the low bits of `id` (`MsGetRomItem` and
/// its siblings).
pub fn rom(mem: &mut impl Mem, table: Global<u32>, id: u32, strings: u32) -> u32 {
    let t = table.get(mem);
    #[allow(clippy::cast_possible_wrap)] // 12 bits
    ms_get_excel_data(mem, (id & 0xfff) as i32, t, strings)
}

/// `MsGetRomCTBbase(n)` (`0x00790990`): record `n - 1` (n clamped to
/// 1..=255) of the CTB base table.
pub fn ms_get_rom_ctb_base(mem: &mut impl Mem, n: i32) -> u32 {
    let t = CTB_BASE.get(mem);
    ms_get_excel_data(mem, ms_check_range(n, 1, 0xff) - 1, t, 0)
}

/// `MsGetRomCTBcount(n)` (`0x007909c0`): the first byte of that record.
pub fn ms_get_rom_ctb_count(mem: &mut impl Mem, n: i32) -> u8 {
    let r = ms_get_rom_ctb_base(mem, n);
    Ptr::<u8>::new(r).get(mem)
}

/// `MsGetRomKaizou(&count)` (`0x00790a10`) and `MsGetRomSummonGrow`
/// (`0x00790ab0`): the table's first record; the first segment's record
/// count goes to `count` when it is not null.
pub fn first_record(mem: &mut impl Mem, table: Global<u32>, count: u32) -> u32 {
    let t = table.get(mem);
    let n = ms_get_excel_data_num(mem, t, 0);
    let r = ms_get_excel_data(mem, 0, t, 0);
    if count != 0 {
        mem.write(count, n);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_bridge::VecMem;

    #[test]
    fn lookup_follows_the_segments() {
        // two segments: IDs 0..=2 stride 4 at +0x40, IDs 0x10..=0x11 stride 8 at +0x60
        let mut m = VecMem::new(0x1000, 0x100);
        m.write(0x1000u32, 2i16);
        for (i, (first, last, stride, strings, data)) in [
            (0u16, 2u16, 4u16, 0x30u16, 0x40u32),
            (0x10, 0x11, 8, 0x20, 0x60),
        ]
        .into_iter()
        .enumerate()
        {
            let s = 0x1008 + 12 * u32::try_from(i).unwrap();
            m.write(s, first);
            m.write(s + 2, last);
            m.write(s + 4, stride);
            m.write(s + 6, strings);
            m.write(s + 8, data);
        }
        assert_eq!(ms_get_excel_data(&mut m, 1, 0x1000, 0), 0x1044);
        assert_eq!(ms_get_excel_data(&mut m, 0x11, 0x1000, 0x10f0), 0x1068);
        assert_eq!(m.read::<u32>(0x10f0), 0x1080);
        // not found: the first segment's data
        assert_eq!(ms_get_excel_data(&mut m, 7, 0x1000, 0), 0x1040);
        assert_eq!(ms_get_excel_data_num(&m, 0x1000, 1), 2);
        assert_eq!(ms_check_range(0, 1, 0xff), 1);
        assert_eq!(ms_check_range(300, 1, 0xff), 0xff);
    }
}
