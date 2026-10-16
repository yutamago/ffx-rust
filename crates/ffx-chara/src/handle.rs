//! Finding characters: the character array (`g_ChrArray`, `g_ChrCount`
//! elements of [`layout::SIZE`] bytes), its iteration cursor, and the model
//! blocks (`g_ChrBlk`, [`layout::BLKS`] of [`layout::BLK_SIZE`] bytes).
//!
//! The core walks slots by index; [`bridge`] maps them to the game's
//! addresses. A handle is the address of an element, as the originals
//! return it.

use crate::layout;

/// Where the walk of `Ch_HandleGetNext` stops: the next active slot at or
/// after `cursor`, and the cursor after it. `None` leaves the cursor as it
/// was (the original stores it only when it steps).
#[must_use]
pub fn next_active(cursor: i32, count: i32, active: impl Fn(i32) -> bool) -> (Option<i32>, i32) {
    let mut c = cursor;
    while c < count {
        let slot = c;
        c += 1;
        if active(slot) {
            return (Some(slot), c);
        }
    }
    (None, c)
}

/// The originals, on the arrays where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{layout as l, next_active};

    /// Characters in [`CHR_ARRAY`].
    pub const CHR_COUNT: Global<i32> = Global::new(0x023c_44e0);
    /// The characters (a heap pointer).
    pub const CHR_ARRAY: Global<u32> = Global::new(0x023c_44e4);
    /// `Ch_HandleGetNext`'s cursor (a slot index).
    pub const HANDLE_CURSOR: Global<i32> = Global::new(0x012f_bd9c);
    /// The model blocks.
    pub const CHR_BLK: Global<u32> = Global::new(0x023c_8d00);
    /// `Ch_FunctionHistoryInit`'s count.
    pub const FUNCTION_HISTORY_COUNT: Global<u32> = Global::new(0x012f_bda4);
    /// `Ch_FunctionHistoryInit`'s entries (16 bytes each, up to
    /// `0x012fc598`).
    pub const FUNCTION_HISTORY: Global<u8> = Global::new(0x012f_bda8);
    /// Entries in [`FUNCTION_HISTORY`].
    pub const FUNCTION_HISTORY_LEN: u32 = (0x012f_c598 - 0x012f_bda8) / 16;

    fn slot(mem: &impl Mem, index: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)] // the original multiplies unsigned
        let i = index as u32;
        CHR_ARRAY.get(mem).wrapping_add(i.wrapping_mul(l::SIZE))
    }

    fn active(mem: &impl Mem, chr: u32) -> bool {
        mem.read::<u8>(chr + l::ACTIVE) != 0
    }

    /// `Ch_CheckHandle(chr)` (`0x008261b0`): 0 when `chr` is an active
    /// character, else -1.
    pub fn ch_check_handle(mem: &impl Mem, chr: u32) -> i32 {
        if chr != 0 {
            let end = slot(mem, CHR_COUNT.get(mem));
            let mut c = CHR_ARRAY.get(mem);
            while c != end {
                if active(mem, c) && c == chr {
                    return 0;
                }
                c = c.wrapping_add(l::SIZE);
            }
        }
        -1
    }

    /// `Ch_HandleGetNext()` (`0x00826d70`): the next active character from
    /// the cursor, or 0 at the end.
    pub fn ch_handle_get_next(mem: &mut impl Mem) -> u32 {
        let cursor = HANDLE_CURSOR.get(mem);
        let count = CHR_COUNT.get(mem);
        let (found, after) = next_active(cursor, count, |i| active(mem, slot(mem, i)));
        if after != cursor {
            HANDLE_CURSOR.set(mem, after);
        }
        found.map_or(0, |i| slot(mem, i))
    }

    /// `Ch_HandleRewind()` (`0x00826db0`).
    pub fn ch_handle_rewind(mem: &mut impl Mem) {
        HANDLE_CURSOR.set(mem, 0);
    }

    /// `Ch_BlkGetHandleFast(key)` (`0x00825ea0`): the model block whose
    /// first word is `key`, or 0.
    pub fn ch_blk_get_handle_fast(mem: &impl Mem, key: u32) -> u32 {
        let first = CHR_BLK.ptr(mem).addr;
        (0..l::BLKS)
            .map(|i| first + i * l::BLK_SIZE)
            .find(|&b| mem.read::<u32>(b) == key)
            .unwrap_or(0)
    }

    /// `Ch_GetChrRenderInsatanceFromOEF2Content(p)` (`0x00826af0`): the
    /// render instance of the first active character (without
    /// [`l::F2_NO_OEF2`]) whose resident data holds `p`; -1 if that
    /// character has none, 0 if no character holds it.
    pub fn ch_get_chr_render_instance_from_oef2_content(mem: &impl Mem, p: u32) -> u32 {
        let count = CHR_COUNT.get(mem);
        for i in 0..count.max(0) {
            let c = slot(mem, i);
            if !active(mem, c) || mem.read::<u32>(c + l::FLAGS2) & l::F2_NO_OEF2 != 0 {
                continue;
            }
            let blk: u32 = mem.read(c + l::BLK);
            let start: u32 = mem.read(blk + l::BLK_RESIDENT);
            let size: u32 = mem.read(blk + l::BLK_RESIDENT_SIZE);
            if start <= p && p < size.wrapping_add(start) {
                let ri: u32 = mem.read(c + l::RENDER_INSTANCE);
                return if ri == 0 { u32::MAX } else { ri };
            }
        }
        0
    }

    /// `Ch_GetChrRenderInstance(chr)` (`0x00826b70`).
    pub fn ch_get_chr_render_instance(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::RENDER_INSTANCE)
    }

    /// `Ch_GetId(chr)` (`0x00826b80`), zero-extended.
    pub fn ch_get_id(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(mem.read::<u16>(chr + l::ID))
    }

    /// `Ch_GetName(chr)` (`0x00826b90`).
    pub fn ch_get_name(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::NAME)
    }

    /// `Ch_GetBlkDirName(chr)` (`0x00826a60`): the address of the model
    /// block's directory name.
    pub fn ch_get_blk_dir_name(mem: &impl Mem, chr: u32) -> u32 {
        mem.read::<u32>(chr + l::BLK).wrapping_add(l::BLK_DIR_NAME)
    }

    /// `Ch_TextureAnimGetHandle(id)` (`0x00835ed0`): the first active
    /// character with texture animation `id`, or 0.
    pub fn ch_texture_anim_get_handle(mem: &impl Mem, id: u32) -> u32 {
        let end = slot(mem, CHR_COUNT.get(mem));
        let mut c = CHR_ARRAY.get(mem);
        while c != end {
            if active(mem, c) && mem.read::<u32>(c + l::TEXTURE_ANIM_ID) == id {
                return c;
            }
            c = c.wrapping_add(l::SIZE);
        }
        0
    }

    /// Cloth file loads (`Ch_IsLoadingClothFile`): 50 of `0x110` bytes,
    /// state byte at `+0` (1 = loading), key at `+4`.
    pub const CLOTH_LOADS: Global<u8> = Global::new(0x012f_c6a0);
    /// Entries in [`CLOTH_LOADS`] (to `0x012ffbc0`).
    pub const CLOTH_LOADS_LEN: u32 = (0x012f_fbc0 - 0x012f_c6a0) / 0x110;

    /// `Ch_IsLoadingClothFile(key)` (`0x0082a100`): 1 while a load of
    /// `key` is in progress.
    pub fn ch_is_loading_cloth_file(mem: &impl Mem, key: u32) -> u32 {
        let first = CLOTH_LOADS.ptr(mem).addr;
        u32::from((0..CLOTH_LOADS_LEN).any(|i| {
            let e = first + i * 0x110;
            mem.read::<u32>(e + 4) == key && mem.read::<u8>(e) == 1
        }))
    }

    /// `Ch_FunctionHistoryInit()` (`0x00826a40`): count 0, each entry's
    /// first byte 0.
    pub fn ch_function_history_init(mem: &mut impl Mem) {
        FUNCTION_HISTORY_COUNT.set(mem, 0);
        let first = FUNCTION_HISTORY.ptr(mem).addr;
        for i in 0..FUNCTION_HISTORY_LEN {
            mem.write(first + 16 * i, 0u8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_walk_skips_inactive_slots() {
        let on = [false, true, false, true];
        let a = |i: i32| on[usize::try_from(i).unwrap()];
        assert_eq!(next_active(0, 4, a), (Some(1), 2));
        assert_eq!(next_active(2, 4, a), (Some(3), 4));
        assert_eq!(next_active(4, 4, a), (None, 4));
        assert_eq!(next_active(5, 4, a), (None, 5));
    }
}
