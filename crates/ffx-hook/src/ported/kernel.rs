//! `ffx-kernel`: kernel table lookups.

use super::{mem, ported};

ported! {
    /// `MsGetExcelData(id, table, strings)`: a kernel record.
    MsGetExcelData @ 0x007a_b870 => extern "C" fn ms_get_excel_data(id: i32, table: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::ms_get_excel_data(&mut mem(), id, table, strings)
    }
    /// `MsGetExcelDataNum(table, segment)`.
    MsGetExcelDataNum @ 0x007a_b8f0 => extern "C" fn ms_get_excel_data_num(table: u32, segment: i32) -> i32 {
        ffx_kernel::bridge::ms_get_excel_data_num(&mem(), table, segment)
    }
    /// `MsCheckRange(x, lo, hi)`.
    MsCheckRange @ 0x0079_a0c0 => extern "C" fn ms_check_range(x: i32, lo: i32, hi: i32) -> i32 {
        ffx_kernel::bridge::ms_check_range(x, lo, hi)
    }
    /// `MsGetRomAbility(id, strings)`.
    MsGetRomAbility @ 0x0079_0970 => extern "C" fn ms_get_rom_ability(id: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::A_ABILITY, id, strings)
    }
    /// `MsGetRomItem(id, strings)`.
    MsGetRomItem @ 0x0079_09f0 => extern "C" fn ms_get_rom_item(id: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::ITEM, id, strings)
    }
    /// The first monster command table (`monmagic1`).
    MsGetRomMonCommand1 @ 0x0079_0a50 => extern "C" fn ms_get_rom_mon_command1(id: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::MON_MAGIC1, id, strings)
    }
    /// `MsGetRomMonCommand2(id, strings)` (`monmagic2`).
    MsGetRomMonCommand2 @ 0x0079_0a70 => extern "C" fn ms_get_rom_mon_command2(id: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::MON_MAGIC2, id, strings)
    }
    /// `MsGetRomPlyCommand(id, strings)`.
    MsGetRomPlyCommand @ 0x0079_0a90 => extern "C" fn ms_get_rom_ply_command(id: u32, strings: u32) -> u32 {
        ffx_kernel::bridge::rom(&mut mem(), ffx_kernel::bridge::COMMAND, id, strings)
    }
    /// `MsGetRomCTBbase(n)`.
    MsGetRomCTBbase @ 0x0079_0990 => extern "C" fn ms_get_rom_ctb_base(n: i32) -> u32 {
        ffx_kernel::bridge::ms_get_rom_ctb_base(&mut mem(), n)
    }
    /// `MsGetRomCTBcount(n)`.
    MsGetRomCTBcount @ 0x0079_09c0 => extern "C" fn ms_get_rom_ctb_count(n: i32) -> u8 {
        ffx_kernel::bridge::ms_get_rom_ctb_count(&mut mem(), n)
    }
    /// `MsGetRomKaizou(&count)`.
    MsGetRomKaizou @ 0x0079_0a10 => extern "C" fn ms_get_rom_kaizou(count: u32) -> u32 {
        ffx_kernel::bridge::first_record(&mut mem(), ffx_kernel::bridge::KAIZOU, count)
    }
    /// `MsGetRomSummonGrow(&count)`.
    MsGetRomSummonGrow @ 0x0079_0ab0 => extern "C" fn ms_get_rom_summon_grow(count: u32) -> u32 {
        ffx_kernel::bridge::first_record(&mut mem(), ffx_kernel::bridge::SUMMON_GROW, count)
    }
}
