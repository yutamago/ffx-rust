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

    /// `0x00776f50(chr)`: a jump to `MsGetSavePlayerPtr` (the
    /// `ply_save` record of `chr`).
    pub fn save_player_ptr(mem: &impl Mem, chr: u32) -> u32 {
        ffx_battle::save::bridge::ms_get_save_player_ptr(mem, chr)
    }
}
