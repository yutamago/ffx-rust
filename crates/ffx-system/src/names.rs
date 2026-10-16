//! Kernel names as the system layer looks them up: a record's name offset
//! in its table's strings.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_kernel::bridge::{ITEM, rom_strings};

    /// What an ID of 0 gets: an empty name (`.bss`, zero).
    pub const EMPTY_NAME: Global<u8> = Global::new(0x0112_a07c);

    /// `Kernel_GetItemName(id)` (`0x007767a0`): the item's name, at its
    /// segment's strings plus the record's first `u16`; [`EMPTY_NAME`] for
    /// 0.
    pub fn item_name(mem: &impl Mem, id: u32) -> u32 {
        if id == 0 {
            return EMPTY_NAME.ptr(mem).addr;
        }
        let (record, strings) = rom_strings(mem, ITEM, id);
        strings.wrapping_add(u32::from(mem.read::<u16>(record)))
    }

    /// The `ply_save` records (`MsGetSavePlayerPtr`'s, `ffx_battle::save`).
    pub const PLY_SAVE: Global<[u8; 0x94]> = Global::new(0x0113_205c);

    /// `0x00776f50(chr)`: a jump to `MsGetSavePlayerPtr`: the 0x94-byte
    /// `ply_save` record of `chr & 0xff`, null from 0x12 on. (`ffx-system`
    /// cannot depend on `ffx-battle`, which depends on it through
    /// `ffx-chara`.)
    pub fn save_player_ptr(mem: &impl Mem, chr: u32) -> u32 {
        let chr = chr & 0xff;
        if chr >= 0x12 {
            return 0;
        }
        PLY_SAVE.ptr(mem).addr + chr * 0x94
    }

    /// `0x00774720(id)`: [`EMPTY_NAME`] for 0, else the auto-ability's
    /// name from `TkMn2GetAbilityMes` (passed in, it lives in `ffx-menu`;
    /// a tail jump).
    pub fn ability_name<M: Mem>(
        mem: &mut M,
        id: u32,
        ability_mes: impl Fn(&mut M, u32) -> u32,
    ) -> u32 {
        if id == 0 {
            return EMPTY_NAME.ptr(mem).addr;
        }
        ability_mes(mem, id)
    }
}
