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

    /// Calls `f` on every active character, the array reread after each
    /// (as the `Ch_All*` loops do).
    pub fn each_active<M: Mem>(mem: &mut M, mut f: impl FnMut(&mut M, u32)) {
        let mut c = CHR_ARRAY.get(mem);
        loop {
            let end = slot(mem, CHR_COUNT.get(mem));
            if c == end {
                break;
            }
            if active(mem, c) {
                f(mem, c);
            }
            c = c.wrapping_add(l::SIZE);
        }
    }

    /// `Ch_AllReserveCalcNorm()` (`0x00832550`).
    pub fn ch_all_reserve_calc_norm(mem: &mut impl Mem) {
        each_active(mem, |m, c| {
            crate::parts::bridge::ch_reserve_calc_norm(m, c, l::F_RESERVE_CALC_NORM);
        });
    }

    /// `Ch_AllSetCalcNorm(on)` (`0x008325b0`).
    pub fn ch_all_set_calc_norm(mem: &mut impl Mem, on: u32) {
        each_active(mem, |m, c| {
            crate::parts::bridge::ch_set_calc_norm(m, c, on);
        });
    }

    /// `Ch_SysGetNbUse()` (`0x008285e0`): characters in use.
    pub fn ch_sys_get_nb_use(mem: &impl Mem) -> u32 {
        let end = slot(mem, CHR_COUNT.get(mem));
        let mut c = CHR_ARRAY.get(mem);
        let mut n = 0u32;
        while c != end {
            n += u32::from(active(mem, c));
            c = c.wrapping_add(l::SIZE);
        }
        n
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

    /// `Ch_HandleFromId(id)` (`0x008261f0`): the first active character
    /// whose `u16` id equals `id` (all 32 bits compared), or 0.
    pub fn ch_handle_from_id(mem: &impl Mem, id: u32) -> u32 {
        let end = slot(mem, CHR_COUNT.get(mem));
        let mut c = CHR_ARRAY.get(mem);
        while c != end {
            if active(mem, c) && u32::from(mem.read::<u16>(c + l::ID)) == id {
                return c;
            }
            c = c.wrapping_add(l::SIZE);
        }
        0
    }

    /// `Ch_UnbindChildren(chr)` (`0x00826150`): clears the bind parent of
    /// the characters bound to `chr`. The original walks `g_ChrCount`
    /// slots from `chr` itself, not from the array's start.
    pub fn ch_unbind_children(mem: &mut impl Mem, chr: u32) {
        if chr == 0 {
            return;
        }
        let mut c = chr;
        loop {
            #[allow(clippy::cast_sign_loss)] // multiplied unsigned
            let end = chr.wrapping_add((CHR_COUNT.get(mem) as u32).wrapping_mul(l::SIZE));
            if c == end {
                break;
            }
            if mem.read::<u32>(c + l::BIND_PARENT) == chr {
                mem.write(c + l::BIND_PARENT, 0u32);
            }
            c = c.wrapping_add(l::SIZE);
        }
    }

    /// `Ch_AllClearFlags24(…)` (`0x00832db0`): clears bits 24..26 of
    /// [`l::FLAGS`] of every active character.
    pub fn ch_all_clear_flags24(mem: &mut impl Mem) {
        each_active(mem, |m, c| {
            let w: u32 = m.read(c + l::FLAGS);
            m.write(c + l::FLAGS, w & 0xf8ff_ffff);
        });
    }

    /// `Ch_CompareDrawOrder(&a, &b)` (`0x008320f0`): a sort comparator:
    /// `(*a)+0x6d4 − (*b)+0x6d4`.
    pub fn ch_compare_draw_order(mem: &impl Mem, a: u32, b: u32) -> u32 {
        let (ca, cb): (u32, u32) = (mem.read(a), mem.read(b));
        mem.read::<u32>(ca + 0x6d4)
            .wrapping_sub(mem.read(cb + 0x6d4))
    }

    /// The debug line buffer the `chr.c` debug prints go to.
    pub const DEBUG_LINE: Global<u8> = Global::new(0x023c_4000);

    /// A C string at `p` (`"(null)"` for a null pointer, as the CRT's `%s`).
    pub(crate) fn c_string(mem: &impl Mem, p: u32) -> Vec<u8> {
        if p == 0 {
            return b"(null)".to_vec();
        }
        let mut out = Vec::new();
        let mut at = p;
        loop {
            let b: u8 = mem.read(at);
            if b == 0 {
                return out;
            }
            out.push(b);
            at = at.wrapping_add(1);
        }
    }

    /// Writes `s` and its NUL at `at` (what `sprintf` leaves).
    pub(crate) fn put_c_string(mem: &mut impl Mem, at: u32, s: &[u8]) {
        let mut v = s.to_vec();
        v.push(0);
        mem.write_bytes(at, &v);
    }

    /// `Ch_FunctionHistory(file, line)` (`0x00833770`): for each active
    /// character without models, `sprintf(debug line, "%s:%s:%d\n", name,
    /// file, line)`.
    pub fn ch_function_history(mem: &mut impl Mem, file: u32, line: u32) {
        each_active(mem, |m, c| {
            let header: u32 = m.read(c + l::MODEL_HEADER);
            if m.read::<u16>(header + l::HEADER_MODELS) != 0 {
                return;
            }
            let name: u32 = m.read(c + l::NAME);
            let mut s = c_string(m, name);
            s.push(b':');
            s.extend(c_string(m, file));
            s.extend(format!(":{}\n", line.cast_signed()).bytes());
            let at = DEBUG_LINE.ptr(m).addr;
            put_c_string(m, at, &s);
        });
    }

    // ---- round 8: setup and per-frame helpers that call other modules

    /// `user_malloc(len)` (`0x00687020`, the system module's).
    pub const USER_MALLOC: u32 = 0x0068_7020;
    /// `Sg_MemSetStr(p, name, n)` (`0x0083c580`).
    pub const MEM_SET_STR: u32 = 0x0083_c580;
    /// `Ch_FunctionHistoryClear()` (`0x00833760`).
    pub const FUNCTION_HISTORY_CLEAR: u32 = 0x0083_3760;
    /// `memclear(p, len)` (`0x0062f4e0`).
    pub const MEMCLEAR: u32 = 0x0062_f4e0;
    /// `Ch_ModelMapping(header)` (`0x00827590`).
    pub const MODEL_MAPPING: u32 = 0x0082_7590;
    /// `g_ChrAllocateErr`.
    pub const ALLOCATE_ERR: Global<u32> = Global::new(0x012f_bd98);

    /// `Ch_Allocate_InitAll(n)` (`0x00825670`): `n` characters
    /// (`user_malloc(n * 0x1100)`, twice what they use), each inactive with
    /// no name (`+4`) and no renderer instance (`+0x830`); the block named
    /// `"[CHR WORK]"`; the allocation error cleared; then
    /// `Ch_FunctionHistoryClear()`, whose result it returns.
    pub fn ch_allocate_init_all<M: Mem>(
        mem: &mut M,
        n: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        CHR_COUNT.set(mem, n);
        ALLOCATE_ERR.set(mem, 0);
        let arr = call(
            mem,
            mem.rebase(USER_MALLOC),
            &[n.cast_unsigned().wrapping_mul(0x1100)],
        );
        CHR_ARRAY.set(mem, arr);
        let mut i = 0;
        while i < CHR_COUNT.get(mem) {
            let c = CHR_ARRAY
                .get(mem)
                .wrapping_add(i.cast_unsigned().wrapping_mul(l::SIZE));
            mem.write(c + l::ACTIVE, 0u8);
            mem.write(c + 4, 0u32);
            mem.write(c + 0x830, 0u32);
            i += 1;
        }
        let arr = CHR_ARRAY.get(mem);
        call(
            mem,
            mem.rebase(MEM_SET_STR),
            &[arr, mem.rebase(0x00b5_94e0), u32::MAX],
        );
        call(mem, mem.rebase(FUNCTION_HISTORY_CLEAR), &[])
    }

    /// `Ch_BlkNew(header, key, x)` (`0x00825860`, ours): the first free
    /// model block ([`CHR_BLK`], 40 of `0x12c` bytes, free when `+0` is
    /// -1; none: address 0, as the original) cleared (`memclear`), `+0` =
    /// `header`, `+4` = `key`, `Ch_ModelMapping(key)`, `+0x14` = `x`,
    /// `+0x30` and `+0x34` = 1.0, `+0x38` = 0.1. Returns the block.
    pub fn ch_blk_new<M: Mem>(
        mem: &mut M,
        header: u32,
        key: u32,
        x: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = CHR_BLK.ptr(mem).addr;
        let b = (0..40)
            .map(|i| base + 0x12c * i)
            .find(|&e| mem.read::<u32>(e) == u32::MAX)
            .unwrap_or(0);
        call(mem, mem.rebase(MEMCLEAR), &[b, 0x12c]);
        mem.write(b, header);
        mem.write(b + 4, key);
        call(mem, mem.rebase(MODEL_MAPPING), &[key]);
        mem.write(b + 0x30, 1f32);
        mem.write(b + 0x34, 1f32);
        mem.write(b + 0x14, x);
        mem.write(b + 0x38, f32::from_bits(0x3dcc_cccd));
        b
    }

    /// `ReturnTableU32_00820580(i)` (`0x00820580`): the frame's work so far.
    pub const FRAME_WORK: u32 = 0x0082_0580;
    /// `Ch_ReserveCalcNorm2(chr)` (`0x00835730`).
    pub const RESERVE_CALC_NORM2: u32 = 0x0083_5730;

    /// `Ch_DrawReserveCalcNorm()` (`0x008336a0`, ours; `Ch_DrawMain`'s):
    /// the budget left this frame (`500 - ReturnTableU32_00820580(0)`)
    /// pays for `budget / 40` normal recalculations
    /// (`Ch_ReserveCalcNorm2`), handed round-robin from the cursor
    /// [`crate::sys::bridge::HISTORY_9E8`] to active characters with flag
    /// bits 1–2 (`+0x194`) and bit 6 (`+0x198`). A full turn with nothing
    /// to do clears the cursor. Returns what the original leaves in EAX.
    pub fn ch_draw_reserve_calc_norm<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cursor = crate::sys::bridge::HISTORY_9E8;
        let used = call(mem, mem.rebase(FRAME_WORK), &[0]);
        let budget = 500i32.wrapping_sub(used.cast_signed());
        if budget < 39 {
            return used;
        }
        let product = i64::from(budget) * 0x6666_6667;
        let mut k = budget / 40;
        let mut last = cursor.get(mem);
        let arr = CHR_ARRAY.get(mem);
        if last == 0 {
            last = arr;
            cursor.set(mem, last);
        }
        if k == 0 {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // the low half of imul
            return product as u32;
        }
        let mut c = last;
        let mut eax;
        loop {
            let end = CHR_ARRAY
                .get(mem)
                .wrapping_add(CHR_COUNT.get(mem).cast_unsigned().wrapping_mul(l::SIZE));
            eax = end;
            c = c.wrapping_add(l::SIZE);
            if c >= end {
                c = CHR_ARRAY.get(mem);
            }
            if c == last {
                cursor.set(mem, 0);
                return eax;
            }
            if mem.read::<u8>(c + l::ACTIVE) != 0
                && mem.read::<u8>(c + 0x194) & 6 != 0
                && mem.read::<u8>(c + 0x198) & 0x40 != 0
            {
                eax = call(mem, mem.rebase(RESERVE_CALC_NORM2), &[c]);
                last = c;
                cursor.set(mem, last);
                k -= 1;
            }
            if k == 0 {
                return eax;
            }
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
