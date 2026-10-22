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
    /// [`crate::sys::bridge::CALC_NORM_CURSOR`] to active characters with flag
    /// bits 1–2 (`+0x194`) and bit 6 (`+0x198`). A full turn with nothing
    /// to do clears the cursor. Returns what the original leaves in EAX.
    pub fn ch_draw_reserve_calc_norm<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cursor = crate::sys::bridge::CALC_NORM_CURSOR;
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

    /// `Ch_CbArcMapping(blk, data)` (`0x00825ee0`), `ch_create_chrname(id)`
    /// (`0x00838060`), the `sprintf` import slot, `memcpy`'s thunk
    /// (`0x00949256`), the debug line (`0x023c4000`) and its format
    /// `"[%s]EffectData"` (`0x00b59320`).
    pub const CB_ARC_MAPPING: u32 = 0x0082_5ee0;
    pub const CREATE_CHRNAME: u32 = 0x0083_8060;
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    pub const MEMCPY_THUNK: u32 = 0x0094_9256;
    pub const EFFECT_DATA_FORMAT: u32 = 0x00b5_9320;

    /// `Ch_BlkAllocate_00825760(id, data)` (`0x00825760`): the first free
    /// model block (none: 0, which the original then clears anyway) is
    /// cleared, marked resident (bit 0 of `+0x70`), takes the id and maps
    /// the archive (`Ch_CbArcMapping`). Its effect data (`+0x120`, to the
    /// end of `data`'s heap block) is copied into its own block (`+0x124`,
    /// size `+0x128`, named `"[<name>]EffectData"`, type 5). Returns the
    /// block. (`dbgPrintf` calls are left out: it is empty.)
    pub fn ch_blk_allocate<M: Mem>(
        mem: &mut M,
        id: u32,
        data: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let base = CHR_BLK.ptr(mem).addr;
        let blk = (0..40u32)
            .map(|k| base + 0x12c * k)
            .find(|&b| mem.read::<u32>(b) == u32::MAX)
            .unwrap_or(0);
        call(mem, mem.rebase(MEMCLEAR), &[blk, 0x12c]);
        let f: u8 = mem.read(blk.wrapping_add(0x70));
        mem.write(blk.wrapping_add(0x70), f | 1);
        mem.write(blk, id);
        call(mem, mem.rebase(CB_ARC_MAPPING), &[blk, data]);
        let effect: u32 = mem.read(blk.wrapping_add(0x120));
        if effect == 0 {
            return blk;
        }
        let size = call(mem, mem.rebase(crate::chr::bridge::MEM_GET_SIZE), &[data]);
        let size = size.wrapping_add(data.wrapping_sub(effect));
        mem.write(blk.wrapping_add(0x128), size);
        call(mem, mem.rebase(CREATE_CHRNAME), &[id, size]);
        let p = call(mem, mem.rebase(USER_MALLOC), &[size]);
        mem.write(blk.wrapping_add(0x124), p);
        let name = call(mem, mem.rebase(CREATE_CHRNAME), &[id]);
        let line = DEBUG_LINE.ptr(mem).addr;
        let sprintf: u32 = mem.read(mem.rebase(SPRINTF_SLOT));
        call(mem, sprintf, &[line, mem.rebase(EFFECT_DATA_FORMAT), name]);
        let p: u32 = mem.read(blk.wrapping_add(0x124));
        call(mem, mem.rebase(MEM_SET_STR), &[p, line, u32::MAX]);
        call(mem, mem.rebase(crate::sys::bridge::MEM_SET_TYPE), &[p, 5]);
        let (to, from, n) = (
            mem.read::<u32>(blk.wrapping_add(0x124)),
            mem.read::<u32>(blk.wrapping_add(0x120)),
            mem.read::<u32>(blk.wrapping_add(0x128)),
        );
        call(mem, mem.rebase(MEMCPY_THUNK), &[to, from, n]);
        blk
    }

    // ---- battle round 35: Ch_Allocate

    /// `Sg_MemSetCheck(on)` (main, `0x0083c4d0`).
    pub const MEM_SET_CHECK: u32 = 0x0083_c4d0;
    /// `Ch_BlkGetHandle(key)` (`0x00825a40`; [`ch_blk_get_handle`], called
    /// through the game: it needs the shim's frame).
    pub const BLK_GET_HANDLE: u32 = 0x0082_5a40;
    /// `dbgPrintf(fmt, ..)` (`0x0062f500`).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `"SG:Ch_Allocate %s:%s\n"`.
    pub const ALLOCATE_FORMAT: u32 = 0x00b5_94f4;
    /// `Ch_CalcElement(chr)` (`0x00832760`; through the game: its scratch
    /// and float callbacks are the shim's).
    pub const CALC_ELEMENT: u32 = 0x0083_2760;
    /// `Ch_NeckInit(chr)` (`0x00834e60`; through the game, as
    /// [`CALC_ELEMENT`]).
    pub const NECK_INIT: u32 = 0x0083_4e60;
    /// `Cache_SetLock(kind, key, on)` (render, `0x00840ac0`).
    pub const CACHE_SET_LOCK: u32 = 0x0084_0ac0;
    /// `Cache_Exist_RenderData(kind, key)` (render, `0x00840890`).
    pub const CACHE_EXIST_RENDER_DATA: u32 = 0x0084_0890;
    /// `Cache_Exist_ShadowMeshData(kind, key)` (render, `0x008408d0`).
    pub const CACHE_EXIST_SHADOW_MESH_DATA: u32 = 0x0084_08d0;
    /// `SG_InitBones(chr)` (main, `0x008277f0`).
    pub const SG_INIT_BONES: u32 = 0x0082_77f0;
    /// `op_et_monster_init(chr, key)` (vfx, `0x007fc780`).
    pub const OP_ET_MONSTER_INIT: u32 = 0x007f_c780;
    /// `graphicCharacterRegistToInstantiate(slot, data, chr, done)`
    /// (render, `0x0063d370`).
    pub const REGIST_TO_INSTANTIATE: u32 = 0x0063_d370;
    /// `Ch_OnCharacterInstanceLoaded` (`0x00827d10`), the render
    /// instance's callback.
    pub const ON_INSTANCE_LOADED: u32 = 0x0082_7d10;
    /// `MagicFile_GetMagicId()` (system, `0x009da3b0`).
    pub const MAGIC_GET_ID: u32 = 0x009d_a3b0;
    /// `g_FFXIsDebug`.
    pub const IS_DEBUG: Global<u8> = Global::new(0x0133_c910);
    /// The new character's clip distance (`f32` 50).
    pub const ALLOCATE_CLIP_Z: Global<u32> = Global::new(0x00b1_f2c8);
    /// -1.0 (`f32`): the heights, thicknesses and run change left unset.
    pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);
    /// The new character's shadow height (`f32` 0.2).
    pub const ALLOCATE_SHADOW_HEIGHT: Global<u32> = Global::new(0x00b4_12c4);
    /// The ids that get flag 2 (`i16` pairs, 0xc4 bytes).
    pub const FLAG2_IDS: Global<u8> = Global::new(0x00c4_94d8);
    /// The ids' two floats at `+0x814`/`+0x818`: 13 entries of 12 bytes
    /// (id, two floats); the first's floats are the default, its id unused.
    pub const ID_FLOATS: Global<u8> = Global::new(0x00c4_95b0);

    /// The magic file whose Rikku (`0x3004`) gets `+0x87c` set.
    const MAGIC_RIKKU_ID: u32 = 0x113;

    /// Whether `id` gets flag `0x200` at [`l::FLAGS2`]: `0x10a6`,
    /// `0x10b2`-`0x10b4`, `0x111e`, `0x1121`-`0x1123`, `0x3004`,
    /// `0x3009`-`0x300b` (the original's two-level switch).
    fn allocate_flag_200(id: u32) -> bool {
        matches!(
            id,
            0x10a6 | 0x10b2..=0x10b4 | 0x111e | 0x1121..=0x1123 | 0x3004 | 0x3009..=0x300b
        )
    }

    /// `Ch_Allocate(key)` (`0x00824f90`): a new character in the first free
    /// slot (`+2` clear; none gives a null one, which the original writes
    /// through as well) for model block `Ch_BlkGetHandle(key)`. A missing
    /// block returns 0 when [`ALLOCATE_ERR`] was set (clearing it), else
    /// takes block 1 (`+0xf8` set). With `g_FFXIsDebug` clear it logs
    /// `"SG:Ch_Allocate %s:%s"` (the block's `+0x91`, `+0x71`). The slot is
    /// cleared, made active, given the block (`Ch_CbMapping`) and its
    /// defaults (the `Ch_Set*` series with the original's arguments:
    /// scale the block's `+0x34`, clip 50, heights -1, shadow height 0.2,
    /// moves 0-2 and 0x10-0x13 unset), set up between `Cache_SetLock(1,
    /// key, 1)` and `(.., 0)` with [`l::FLAGS2`] bit `0x400` held
    /// (`SG_InitBones`, textures, `op_et_monster_init`, effect level 1).
    /// The shadow mode follows `id >> 12` (0-3: 1, 4-6: 0, 15: 3 and flag
    /// `0x40`), then mouth, elements, option positions, neck and cloth;
    /// the id tables give flag 2 ([`FLAG2_IDS`]) and the two floats
    /// ([`ID_FLOATS`], the last match), ids `0x1088`, `0x1089`, `0x108c`
    /// flag `0x80`, [`allocate_flag_200`] flag `0x200`; a character with
    /// flag `0x200000` takes its rest pose (`+0x83c` 1, else 0); the render
    /// and shadow instances are registered (`+0x830` with
    /// [`ON_INSTANCE_LOADED`], `+0x838` when a shadow mesh exists), and
    /// `+0x87c` is set for Rikku in magic file `0x113`. Returns the
    /// character. Unported and other modules' callees go through `call`.
    #[allow(clippy::too_many_lines)]
    pub fn ch_allocate<M: Mem>(
        mem: &mut M,
        key: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::chr::bridge as c;
        use crate::parts::bridge as p;
        use ffx_bridge::x87::round_trip;
        let game = |mem: &M, f: u32| mem.rebase(f);
        let copy = |bits: u32| round_trip(f32::from_bits(bits));
        call(mem, game(mem, MEM_SET_CHECK), &[0]);
        let mut chr = 0;
        let mut i = 0;
        while i < CHR_COUNT.get(mem) {
            let at = slot(mem, i);
            if !active(mem, at) {
                chr = at;
                break;
            }
            i += 1;
        }
        let mut blk = call(mem, game(mem, BLK_GET_HANDLE), &[key]);
        if blk == 0 {
            if ALLOCATE_ERR.get(mem) != 0 {
                ALLOCATE_ERR.set(mem, 0);
                return 0;
            }
            blk = call(mem, game(mem, BLK_GET_HANDLE), &[1]);
            mem.write(blk.wrapping_add(0xf8), 1u8);
        }
        ALLOCATE_ERR.set(mem, 0);
        if IS_DEBUG.get(mem) == 0 {
            let f = game(mem, ALLOCATE_FORMAT);
            let names = [blk.wrapping_add(0x91), blk.wrapping_add(0x71)];
            call(mem, game(mem, DBG_PRINTF), &[f, names[0], names[1]]);
        }
        mem.write_bytes(chr, &[0; l::SIZE as usize]);
        mem.write(chr.wrapping_add(l::ACTIVE), 1u8);
        mem.write(chr.wrapping_add(l::BLK), blk);
        crate::mapping::bridge::ch_cb_mapping(mem, chr, blk);
        for o in [0x7e4, l::BIND_SELF, l::BIND_PARENT_ID] {
            mem.write(chr.wrapping_add(o), u32::MAX);
        }
        c::ch_set_pos3(mem, chr, 0.0, 0.0, 0.0);
        c::ch_set_u8(mem, chr, l::GRAV_MODE, 1);
        c::ch_set_fade(mem, chr, l::FADE_SHADE, 1.0, 0);
        c::ch_set_shade_col(mem, chr, [1.0; 3], 0, call);
        c::ch_set_shade_col_effect(mem, chr, 1.0, 1.0, 1.0);
        c::ch_set_shade_id(mem, chr, 1.0, 0, call);
        c::ch_set_fade(mem, chr, l::FADE_ALPHA, 1.0, 0);
        c::ch_set_fade(mem, chr, l::FADE_SPECULAR, 1.0, 0);
        c::ch_set_motion_speed(mem, chr, 0x100);
        let z = copy(ALLOCATE_CLIP_Z.get(mem));
        c::ch_set_clip_z(mem, chr, z, call);
        c::ch_set_weight(mem, chr, 1.0);
        let scale = copy(mem.read(blk.wrapping_add(0x34)));
        c::ch_set_scale_all(mem, chr, scale);
        let unset = |mem: &M| copy(MINUS_ONE.get(mem));
        let v = unset(mem);
        c::ch_set_cam_height(mem, chr, v);
        let v = unset(mem);
        c::ch_set_fix_height(mem, chr, v);
        let v = unset(mem);
        c::ch_set_thickness(mem, chr, v);
        let v = unset(mem);
        c::ch_set_thickness_btl_cam(mem, chr, v);
        let v = copy(ALLOCATE_SHADOW_HEIGHT.get(mem));
        p::ch_shadow_set_height(mem, chr, v);
        p::ch_set_keep_fps(mem, chr, 1);
        let unset_move = u32::MAX;
        let moves = [0, 1, 2].map(|k| (k, k));
        let unset_moves = [0x10, 0x11, 0x12, 0x13].map(|k| (k, unset_move));
        for (slot, motion) in moves.into_iter().chain(unset_moves) {
            c::ch_motion_set_moves(mem, chr, slot, motion);
        }
        c::ch_motion_set_next_hokan(mem, chr, 1);
        c::ch_en_clip(mem, chr, 1);
        let v = unset(mem);
        c::ch_move_set_chg_run(mem, chr, v);
        c::ch_en_rot_hokan(mem, chr, 1);
        c::ch_id_enable_shade(mem, chr, 1);
        crate::id::bridge::ch_id_set_def_pid(mem, chr);
        p::ch_draw_set_cull(mem, chr, 1);
        p::ch_bind_set_scale(mem, chr, 1.0);
        c::ch_set_overlap_hit(mem, chr, 1);
        p::ch_cloth_set_enable(mem, chr, 1);
        p::ch_cloth_enable_grav(mem, chr, 1);
        c::ch_vu1_set_clipmode(mem, chr, 0);
        p::ch_neck_set_speed(mem, chr, 0.0);
        crate::texture::bridge::ch_texture_anim_set_enable(mem, chr, 1);
        c::ch_set_f32(mem, chr, l::ORG_MATRIX_SCALE, 1.0);
        call(mem, game(mem, CACHE_SET_LOCK), &[1, key, 1]);
        let flags2 = chr.wrapping_add(l::FLAGS2);
        let v: u32 = mem.read(flags2);
        mem.write(flags2, v | 0x400);
        call(mem, game(mem, SG_INIT_BONES), &[chr]);
        crate::texture::bridge::ch_texture_setup(mem, chr, call);
        crate::texanim::bridge::ch_texture_set_clut_base(mem, chr, u32::MAX, u32::MAX);
        crate::texanim::bridge::ch_texture_set_image_base(mem, chr, u32::MAX, u32::MAX);
        call(mem, game(mem, OP_ET_MONSTER_INIT), &[chr, key]);
        c::ch_effect_set_effect_level(mem, chr, 1);
        call(mem, game(mem, CACHE_SET_LOCK), &[1, key, 0]);
        let v: u32 = mem.read(flags2);
        mem.write(flags2, v & !0x400);
        let flags = chr.wrapping_add(l::FLAGS);
        let set = |mem: &mut M, at: u32, bit: u32| {
            let v: u32 = mem.read(at);
            mem.write(at, v | bit);
        };
        match u32::from(mem.read::<u16>(chr)) >> 12 {
            0..=3 => {
                c::ch_set_shadow_mode(mem, chr, 1);
            }
            4..=6 => {
                c::ch_set_shadow_mode(mem, chr, 0);
            }
            0xf => {
                c::ch_set_shadow_mode(mem, chr, 3);
                set(mem, flags, 0x40);
            }
            _ => {}
        }
        p::ch_mouth_init(mem, chr);
        call(mem, game(mem, CALC_ELEMENT), &[chr]);
        crate::attach::bridge::ch_optpos_calc(mem, chr);
        call(mem, game(mem, NECK_INIT), &[chr]);
        crate::cloth::bridge::ch_cloth_reset(mem, chr);
        p::ch_cloth_mesh_mapping_offset(mem, chr);
        p::ch_cloth_set_fast(mem, chr);
        let id = u32::from(mem.read::<u16>(chr));
        let ids = FLAG2_IDS.ptr(mem).addr;
        for k in 0..0xc4 / 2 {
            let v: i16 = mem.read(ids.wrapping_add(2 * k));
            if id == i32::from(v).cast_unsigned() {
                set(mem, flags, 2);
            }
        }
        let table = ID_FLOATS.ptr(mem).addr;
        let floats = |mem: &mut M, e: u32| {
            for k in 0..2 {
                let v = copy(mem.read(table.wrapping_add(e * 12 + 4 + 4 * k)));
                mem.write(chr.wrapping_add(0x814 + 4 * k), v.to_bits());
            }
        };
        floats(mem, 0);
        for e in 1..=12 {
            if id == mem.read::<u32>(table.wrapping_add(e * 12)) {
                floats(mem, e);
            }
        }
        if matches!(id, 0x1088 | 0x1089 | 0x108c) {
            set(mem, flags2, 0x80);
        }
        if allocate_flag_200(id) {
            set(mem, flags2, 0x200);
        }
        let rest = if mem.read::<u32>(flags) & 0x20_0000 == 0 {
            0u32
        } else {
            crate::cloth::bridge::ch_get_rest_pose_matrices(mem, chr, call);
            1
        };
        mem.write(chr.wrapping_add(0x83c), rest);
        let shadow_slot = chr.wrapping_add(0x838);
        mem.write(chr.wrapping_add(0x870), 1u16);
        mem.write(shadow_slot, 0u32);
        let render = call(mem, game(mem, CACHE_EXIST_RENDER_DATA), &[1, key]);
        let shadow = call(mem, game(mem, CACHE_EXIST_SHADOW_MESH_DATA), &[1, key]);
        let regist = game(mem, REGIST_TO_INSTANTIATE);
        let done = game(mem, ON_INSTANCE_LOADED);
        call(
            mem,
            regist,
            &[chr.wrapping_add(l::RENDER_INSTANCE), render, chr, done],
        );
        if shadow != 0 {
            call(mem, regist, &[shadow_slot, shadow, chr, 0]);
        }
        let magic = call(mem, game(mem, MAGIC_GET_ID), &[]);
        let rikku = magic == MAGIC_RIKKU_ID && mem.read::<u16>(chr) == 0x3004;
        mem.write(chr.wrapping_add(0x87c), u8::from(rikku));
        chr
    }

    // ---- battle round 36: Ch_BlkGetHandle

    /// `Sg_PcRead(path)` (main, `0x0083ae30`): a PC file's data, or 0.
    pub const PC_READ: u32 = 0x0083_ae30;
    /// `Sg_MemSetAlign(align)` (main, `0x0083c4a0`).
    pub const MEM_SET_ALIGN: u32 = 0x0083_c4a0;
    /// `Chb_AttachMotionTBL_OLD(blk)` (`0x00837fa0`, below), called
    /// through the game.
    pub const ATTACH_MOTION_TBL_OLD: u32 = 0x0083_7fa0;
    /// Bytes of scratch [`chb_attach_motion_tbl_old`] needs: its path
    /// (`[ebp-0x104]`, 0x100 bytes).
    pub const ATTACH_MOTION_TBL_SCRATCH: u32 = 0x100;

    /// `Chb_AttachMotionTBL_OLD(blk)` (`0x00837fa0`; only the host-PC
    /// viewer path of `Ch_BlkGetHandle` calls it): reads
    /// `"<dir>/inf/sysmot.tbl"` and `"<dir>/inf/relation.tbl"` (`dir` the
    /// block's `+0xb8`; `Sg_PcRead`, path in `scratch`) into `+0x108` and
    /// `+0x10c`. A table that was read is relocated: its first word, the
    /// address it was built for, becomes its own, and when that moved it
    /// the system table's words `+0xc`, `+0x14`, `+0x1c`, `+0x24` move by
    /// the same amount, the relation table's `[+4]` words from `+0x10`
    /// (the count read again at each step, compared signed). No caller
    /// reads `eax`.
    pub fn chb_attach_motion_tbl_old<M: Mem>(
        mem: &mut M,
        blk: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let read = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, file: &[u8]| {
            let dir = c_string(mem, blk.wrapping_add(0xb8));
            put_c_string(mem, scratch, &[dir.as_slice(), file].concat());
            call(mem, mem.rebase(PC_READ), &[scratch])
        };
        let t = read(mem, call, b"/inf/sysmot.tbl");
        mem.write(blk.wrapping_add(0x108), t);
        if t != 0 {
            let d = t.wrapping_sub(mem.read(t));
            mem.write(t, t);
            if d != 0 {
                for o in [0xc, 0x14, 0x1c, 0x24] {
                    let w: u32 = mem.read(t.wrapping_add(o));
                    mem.write(t.wrapping_add(o), w.wrapping_add(d));
                }
            }
        }
        let t = read(mem, call, b"/inf/relation.tbl");
        mem.write(blk.wrapping_add(0x10c), t);
        if t == 0 {
            return;
        }
        let d = t.wrapping_sub(mem.read(t));
        mem.write(t, t);
        if d == 0 {
            return;
        }
        let mut i = 0i32;
        while i < mem.read::<i32>(t.wrapping_add(4)) {
            let at = t.wrapping_add(0x10).wrapping_add(4 * i.cast_unsigned());
            let w: u32 = mem.read(at);
            mem.write(at, w.wrapping_add(d));
            i += 1;
        }
    }
    /// `Ch_DataRead(id)` (`0x00829ef0`; through the game: its scratch is
    /// the shim's).
    pub const DATA_READ: u32 = 0x0082_9ef0;
    /// `Cache_Exist3(kind, id)`, `Cache_CheckYet(kind, id)`,
    /// `Cache_Use(kind, id)` (render).
    pub const CACHE_EXIST3: u32 = 0x0084_0790;
    pub const CACHE_CHECK_YET: u32 = 0x0083_ffe0;
    pub const CACHE_USE: u32 = 0x0084_0dd0;
    /// `"Viewer"`: a viewer block's tag (`+0xb4`).
    pub const VIEWER_TAG: u32 = 0x00b5_9368;
    /// `""`: a game block's tag without `g_FFXIsDebug`.
    pub const EMPTY_TAG: u32 = 0x00b8_fbeb;
    /// `"chrdata %x is not exist\n"`.
    pub const NOT_EXIST_FORMAT: u32 = 0x00b5_93c4;
    /// The viewer models (key `0xfnnn`): 12-byte entries, the name at
    /// `+0`, the directory at `+8`.
    pub const VIEWER_MODELS: Global<u32> = Global::new(0x012f_bca8);
    /// The debug model lists by kind (`key >> 12`): their entries
    /// (0x80 bytes, the number first, the tag at `+0x30`) and counts.
    pub const DEBUG_MODELS: Global<u32> = Global::new(0x012f_bc64);
    pub const DEBUG_MODEL_COUNTS: Global<u32> = Global::new(0x012f_bc84);
    /// The bytes [`ch_blk_get_handle`] needs at `frame`: the path
    /// (`0x100`) and the directory after it (`0x40`), as the original's
    /// two frame buffers lie.
    pub const BLK_GET_HANDLE_FRAME: u32 = 0x140;

    /// `strcpy(dst, src)` as the original's byte loops.
    fn copy_c_string(mem: &mut impl Mem, dst: u32, src: u32) {
        let s = c_string(mem, src);
        put_c_string(mem, dst, &s);
    }

    /// `Ch_BlkGetHandle(key)` (`0x00825a40`; caller `Ch_Allocate`): the
    /// model block of `key` ([`ch_blk_get_handle_fast`]), else a new one:
    ///
    /// - A viewer key (`0xfnnn`, [`VIEWER_MODELS`] entry `nnn`) reads
    ///   `/ffx/proj2/chr/prot/<dir>/<name>` from the PC (`Sg_PcRead`):
    ///   `mdl/<name>.chr` gives an archive block
    ///   ([`ch_blk_allocate`], `+0x70` bit 1), with `tex/<name>.tex`
    ///   (aligned to 0x80, named, [`crate::mapping::bridge::ch_tex_data_mapping`]) at `+0x14`
    ///   and bit 2 when it exists, the data named `"[dir:name]VIEWCHRDATA"`;
    ///   else `mdl/<name>.mdl` and its texture make a block
    ///   ([`ch_blk_new`], `Chb_AttachMotionTBL_OLD`, `"[dir:name]MDLDATA"`);
    ///   neither: 0. Both take the tag `"Viewer"` (`+0xb4`), the path
    ///   (`+0xb8`), name (`+0x71`) and directory (`+0x91`).
    /// - Any other: with `g_FFXIsDebug` its tag is the debug list entry of
    ///   number `key & 0xfff` for kind `key >> 12` ([`DEBUG_MODELS`], the
    ///   last match, `+0x30`; none: `dbgPrintf("chrdata %x is not
    ///   exist")` and 0), else `""`. The render cache's data
    ///   (`Cache_Exist3(1, key)`; missing: `Ch_DataRead`, `Ch_ReadSync`,
    ///   asked again, still missing: 0), waited for when
    ///   `Cache_CheckYet` is 1, used (`Cache_Use`), makes an archive block
    ///   with the tag, the path `/ffx/proj2/chr/data/<section>/<name>`
    ///   (debug; else empty), `ch_create_name` and `ch_create_section`.
    ///
    /// Returns the block (EAX). The strings are written at `frame`
    /// ([`BLK_GET_HANDLE_FRAME`] bytes), the original's frame buffers.
    #[allow(clippy::too_many_lines)]
    pub fn ch_blk_get_handle<M: Mem>(
        mem: &mut M,
        key: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::texanim::bridge::{ch_create_name, ch_create_section};
        let blk = ch_blk_get_handle_fast(mem, key);
        if blk != 0 {
            return blk;
        }
        let (path, dir) = (frame, frame.wrapping_add(0x100));
        let game = |mem: &M, f: u32| mem.rebase(f);
        let set_str = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, p: u32| {
            let f = game(mem, MEM_SET_STR);
            call(mem, f, &[p, path, u32::MAX]);
        };
        let read = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f = game(mem, PC_READ);
            call(mem, f, &[path])
        };
        let align = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f = game(mem, MEM_SET_ALIGN);
            call(mem, f, &[0x80]);
        };
        let names = |mem: &mut M, blk: u32, name: u32, section: u32| {
            copy_c_string(mem, blk.wrapping_add(0xb8), dir);
            copy_c_string(mem, blk.wrapping_add(0x71), name);
            copy_c_string(mem, blk.wrapping_add(0x91), section);
        };
        if key & 0xf000 == 0xf000 {
            let entry = VIEWER_MODELS.ptr(mem).addr.wrapping_add((key & 0xfff) * 12);
            let name: u32 = mem.read(entry);
            let section: u32 = mem.read(entry.wrapping_add(8));
            let (n, sec) = (c_string(mem, name), c_string(mem, section));
            let d = [b"/ffx/proj2/chr/prot/".as_slice(), &sec, b"/", &n].concat();
            put_c_string(mem, dir, &d);
            let file =
                |kind: &[u8], ext: &[u8]| [d.as_slice(), b"/", kind, b"/", &n, b".", ext].concat();
            put_c_string(mem, path, &file(b"mdl", b"chr"));
            let data = read(mem, call);
            if data != 0 {
                let blk = ch_blk_allocate(mem, key, data, call);
                let f: u8 = mem.read(blk.wrapping_add(0x70));
                mem.write(blk.wrapping_add(0x70), f | 2);
                let tag = game(mem, VIEWER_TAG);
                mem.write(blk.wrapping_add(0xb4), tag);
                put_c_string(mem, path, &file(b"tex", b"tex"));
                align(mem, call);
                let tex = read(mem, call);
                if tex != 0 {
                    set_str(mem, call, tex);
                    crate::mapping::bridge::ch_tex_data_mapping(mem, tex);
                    let f: u8 = mem.read(blk.wrapping_add(0x70));
                    mem.write(blk.wrapping_add(0x70), f | 4);
                    mem.write(blk.wrapping_add(0x14), tex);
                }
                names(mem, blk, name, section);
                let (sec, n) = (
                    c_string(mem, blk.wrapping_add(0x91)),
                    c_string(mem, blk.wrapping_add(0x71)),
                );
                let label = [b"[".as_slice(), &sec, b":", &n, b"]VIEWCHRDATA"].concat();
                put_c_string(mem, path, &label);
                set_str(mem, call, data);
                return blk;
            }
            put_c_string(mem, path, &file(b"mdl", b"mdl"));
            let data = read(mem, call);
            if data == 0 {
                return 0;
            }
            set_str(mem, call, data);
            put_c_string(mem, path, &file(b"tex", b"tex"));
            align(mem, call);
            let tex = read(mem, call);
            set_str(mem, call, tex);
            crate::mapping::bridge::ch_tex_data_mapping(mem, tex);
            let blk = ch_blk_new(mem, key, data, tex, call);
            let tag = game(mem, VIEWER_TAG);
            mem.write(blk.wrapping_add(0xb4), tag);
            names(mem, blk, name, section);
            let f = game(mem, ATTACH_MOTION_TBL_OLD);
            call(mem, f, &[blk]);
            let label = [b"[".as_slice(), &sec, b":", &n, b"]MDLDATA"].concat();
            put_c_string(mem, path, &label);
            set_str(mem, call, data);
            return blk;
        }
        let debug = IS_DEBUG.get(mem) != 0;
        let tag = if debug {
            let kind = (key.cast_signed() >> 12).cast_unsigned();
            let count = DEBUG_MODEL_COUNTS.ptr(mem).at(kind).get(mem).cast_signed();
            let mut found = 0;
            if count > 0 {
                let mut e = DEBUG_MODELS.ptr(mem).at(kind).get(mem);
                for _ in 0..count {
                    if mem.read::<u32>(e) == key & 0xfff {
                        found = e;
                    }
                    e = e.wrapping_add(0x80);
                }
            }
            if found == 0 {
                let f = game(mem, DBG_PRINTF);
                let fmt = game(mem, NOT_EXIST_FORMAT);
                call(mem, f, &[fmt, key]);
                return 0;
            }
            found.wrapping_add(0x30)
        } else {
            game(mem, EMPTY_TAG)
        };
        let name = ch_create_name(mem, key);
        let section = ch_create_section(mem, key);
        let exist = game(mem, CACHE_EXIST3);
        let mut data = call(mem, exist, &[1, key]);
        if data == 0 {
            let f = game(mem, DATA_READ);
            call(mem, f, &[key]);
            crate::sys::bridge::ch_read_sync(mem, call);
            data = call(mem, exist, &[1, key]);
            if data == 0 {
                return 0;
            }
        }
        let f = game(mem, CACHE_CHECK_YET);
        if call(mem, f, &[1, key]) == 1 {
            crate::sys::bridge::ch_read_sync(mem, call);
        }
        let f = game(mem, CACHE_USE);
        call(mem, f, &[1, key]);
        let blk = ch_blk_allocate(mem, key, data, call);
        mem.write(blk.wrapping_add(0xb4), tag);
        if debug {
            let (sec, n) = (c_string(mem, section), c_string(mem, name));
            let d = [b"/ffx/proj2/chr/data/".as_slice(), &sec, b"/", &n].concat();
            put_c_string(mem, dir, &d);
            copy_c_string(mem, blk.wrapping_add(0xb8), dir);
        } else {
            mem.write(blk.wrapping_add(0xb8), 0u8);
        }
        let name = ch_create_name(mem, key);
        copy_c_string(mem, blk.wrapping_add(0x71), name);
        let section = ch_create_section(mem, key);
        copy_c_string(mem, blk.wrapping_add(0x91), section);
        blk
    }

    /// `"SG:Ch_DataDispose:%s -result= "`, `"-1\n"` (dbgPrintf) and the
    /// in-use assertion's place and text (`yiAssert`).
    pub const DATA_DISPOSE_FORMATS: [u32; 4] = [0x00b5_93fc, 0x00b5_941c, 0x00b5_9450, 0x00b5_9420];
    /// `yiAssert(cond, place, fmt, ..)` (`0x0090ae50`).
    pub const YI_ASSERT: u32 = 0x0090_ae50;
    /// `Sg_CacheDisposeChr(id)` (main, `0x00840e60`).
    pub const CACHE_DISPOSE_CHR: u32 = 0x0084_0e60;
    /// `Ch_BlkDispose(blk)` (`0x008258d0`).
    pub const BLK_DISPOSE: u32 = 0x0082_58d0;

    /// `ChN_DataDispose(key, check)` (`0x00824f00`): logs the key's name
    /// (`ch_create_chrname`); a character of that id still in use
    /// (`Ch_HandleFromId`) gives -1 (logged, and with `check` set an
    /// assertion naming it); else the model block (`Ch_BlkGetHandleFast`),
    /// when there is one, is disposed (`Ch_BlkDispose`) and its id's cache
    /// entry too (`Sg_CacheDisposeChr`); 0.
    pub fn ch_n_data_dispose<M: Mem>(
        mem: &mut M,
        key: u32,
        check: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let game = |mem: &M, f: u32| mem.rebase(f);
        let [result, minus, place, text] = DATA_DISPOSE_FORMATS;
        let name = crate::texanim::bridge::ch_create_chrname(mem, key);
        call(mem, game(mem, DBG_PRINTF), &[game(mem, result), name]);
        if ch_handle_from_id(mem, key) != 0 {
            call(mem, game(mem, DBG_PRINTF), &[game(mem, minus)]);
            if check != 0 {
                let name = crate::texanim::bridge::ch_create_chrname(mem, key);
                let args = [0, game(mem, place), game(mem, text), name];
                call(mem, game(mem, YI_ASSERT), &args);
            }
            return u32::MAX;
        }
        let blk = ch_blk_get_handle_fast(mem, key);
        if blk != 0 {
            let id: u32 = mem.read(blk);
            crate::chr::bridge::ch_blk_dispose(mem, blk, call);
            call(mem, game(mem, CACHE_DISPOSE_CHR), &[id]);
        }
        0
    }

    /// `"SG:Ch_Dispose:%s\n"`.
    pub const DISPOSE_FORMAT: u32 = 0x00b5_950c;
    /// `MagicFile_GetHandle()` (system, `0x009da390`).
    pub const MAGIC_GET_HANDLE: u32 = 0x009d_a390;
    /// `JobScheduleWait_Ch_Cloth_PerCharacter()` (`0x00a44b30`).
    pub const CLOTH_JOBS_WAIT: u32 = 0x00a4_4b30;
    /// `op_et_monster_free(chr)` (vfx, `0x007fc650`).
    pub const OP_ET_MONSTER_FREE: u32 = 0x007f_c650;
    /// `graphicCharacterInstanceSetVisible(instance, on)` (render,
    /// `0x0063c390`).
    pub const INSTANCE_SET_VISIBLE: u32 = 0x0063_c390;
    /// `Sg_GetPlayer()` (main, `0x0082d860`).
    pub const SG_GET_PLAYER: u32 = 0x0082_d860;
    /// `Sg_SetPlayer(chr)` (main, `0x0082dad0`).
    pub const SG_SET_PLAYER: u32 = 0x0082_dad0;
    /// `user_free_thunk(p)` (`0x00687010`).
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `graphicCharacterInstanceDestroy(instance, chr)` (render,
    /// `0x0063a030`).
    pub const INSTANCE_DESTROY: u32 = 0x0063_a030;
    /// `graphicCharacterInstanceDestroyClothVertexBuffers(buffers)`
    /// (render, `0x0063a160`).
    pub const INSTANCE_DESTROY_CLOTH_BUFFERS: u32 = 0x0063_a160;

    /// `Ch_Dispose(chr)` (`0x008266f0`): an active character (else
    /// `Ch_CheckHandle`'s -1 back) is logged, unbinds its children, waits
    /// for the cloth jobs in magic file `0x113`, gives back its CLUTs, its
    /// monster effects (unless its block's `+0x70` bit 1), goes inactive
    /// and invisible, stops being the player and Tidus, destroys its cloth
    /// meshes, frees each model's three buffers (`+0x1bc`, 0x38 bytes
    /// each, the header's `+6` count, re-read) and the `+0x874` table
    /// (each non-null of the header's `+8` entries, then itself), its
    /// texture animation (`+0x6fc` set: `+0x700`), `+0x328`, `+0x1bc`,
    /// `+0x748`, `+0x6dc`, `+0x32c`, sets the memory-move trigger, destroys
    /// its render and shadow instances (`+0x830`, `+0x838`, cleared) and
    /// cloth vertex buffers (`+0x834`, cleared after). Returns the last
    /// call's EAX.
    pub fn ch_dispose<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let game = |mem: &M, f: u32| mem.rebase(f);
        let at = |o: u32| chr.wrapping_add(o);
        let check = ch_check_handle(mem, chr);
        if check != 0 {
            return check.cast_unsigned();
        }
        let name: u32 = mem.read(at(4));
        call(
            mem,
            game(mem, DBG_PRINTF),
            &[game(mem, DISPOSE_FORMAT), name],
        );
        ch_unbind_children(mem, chr);
        if call(mem, game(mem, MAGIC_GET_HANDLE), &[]) != u32::MAX
            && call(mem, game(mem, MAGIC_GET_ID), &[]) == MAGIC_RIKKU_ID
        {
            call(mem, game(mem, CLOTH_JOBS_WAIT), &[]);
        }
        crate::texture::bridge::ch_clut_dispose(mem, chr, call);
        let blk: u32 = mem.read(at(l::BLK));
        if mem.read::<u8>(blk.wrapping_add(0x70)) & 2 == 0 {
            call(mem, game(mem, OP_ET_MONSTER_FREE), &[chr]);
        }
        mem.write(at(l::ACTIVE), 0u8);
        let instance: u32 = mem.read(at(l::RENDER_INSTANCE));
        call(mem, game(mem, INSTANCE_SET_VISIBLE), &[instance, 0]);
        let tidus = crate::mapping::bridge::TIDUS;
        if call(mem, game(mem, SG_GET_PLAYER), &[]) == chr || tidus.get(mem) == chr {
            call(mem, game(mem, SG_SET_PLAYER), &[0]);
            tidus.set(mem, 0);
        }
        let header: u32 = mem.read(at(l::MODEL_HEADER));
        crate::cloth::bridge::ch_cloth_mesh_destroy(mem, chr, call);
        let free = game(mem, USER_FREE);
        let count = |mem: &M, o: u32| u32::from(mem.read::<u16>(header.wrapping_add(o)));
        let mut i = 0;
        while i < count(mem, 6) {
            let model = mem
                .read::<u32>(at(l::MODELS))
                .wrapping_add(i * l::MODEL_SIZE);
            for o in [4, 8, 0xc] {
                let p: u32 = mem.read(model.wrapping_add(o));
                call(mem, free, &[p]);
            }
            i += 1;
        }
        let table: u32 = mem.read(at(0x874));
        if table != 0 {
            let mut i = 0;
            while i < count(mem, 8) {
                let p: u32 = mem.read(mem.read::<u32>(at(0x874)).wrapping_add(4 * i));
                if p != 0 {
                    call(mem, free, &[p]);
                }
                i += 1;
            }
            let table: u32 = mem.read(at(0x874));
            call(mem, free, &[table]);
        }
        if mem.read::<u32>(at(0x6fc)) != 0 {
            let id: u32 = mem.read(at(0x700));
            crate::texanim::bridge::chr_texanim_del(mem, id.cast_signed(), call);
        }
        for o in [0x328, l::MODELS, 0x748, 0x6dc, 0x32c] {
            let p: u32 = mem.read(at(o));
            call(mem, free, &[p]);
        }
        crate::sys::bridge::ch_mem_move_set_trigger(mem);
        for o in [l::RENDER_INSTANCE, 0x838] {
            let instance: u32 = mem.read(at(o));
            call(mem, game(mem, INSTANCE_DESTROY), &[instance, chr]);
        }
        let buffers: u32 = mem.read(at(0x834));
        mem.write(at(l::RENDER_INSTANCE), 0u32);
        mem.write(at(0x838), 0u32);
        let r = call(mem, game(mem, INSTANCE_DESTROY_CLOTH_BUFFERS), &[buffers]);
        mem.write(at(0x834), 0u32);
        r
    }

    /// `Ch_Dispose(chr)` (`0x008266f0`), called through the game.
    pub const DISPOSE: u32 = 0x0082_66f0;
    /// `Ch_Allocate(key)` (`0x00824f90`), called through the game.
    pub const ALLOCATE: u32 = 0x0082_4f90;

    /// `Ch_DisposeAll()` (`0x008268e0`): with an array, `Ch_Dispose` on
    /// each active character (the array and its count read again after
    /// each call, as the walk's end), the array freed (`user_free`) and
    /// both cleared. Nothing without an array. No caller reads `eax`.
    pub fn ch_dispose_all<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let end = |mem: &M| {
            let n = CHR_COUNT.get(mem).cast_unsigned();
            CHR_ARRAY.get(mem).wrapping_add(n.wrapping_mul(l::SIZE))
        };
        let array = CHR_ARRAY.get(mem);
        if array == 0 {
            return;
        }
        let mut c = array;
        while c != end(mem) {
            if mem.read::<u8>(c.wrapping_add(l::ACTIVE)) != 0 {
                call(mem, mem.rebase(DISPOSE), &[c]);
            }
            c = c.wrapping_add(l::SIZE);
        }
        let array = CHR_ARRAY.get(mem);
        call(mem, mem.rebase(USER_FREE), &[array]);
        CHR_ARRAY.set(mem, 0);
        CHR_COUNT.set(mem, 0);
    }

    /// `Ch_Allocate2(key, v)` (`0x00825650`): `Ch_Allocate(key)` with `v`
    /// in the new character's `+0x190` (what a bound child copies); returns
    /// the character.
    pub fn ch_allocate2<M: Mem>(
        mem: &mut M,
        key: u32,
        v: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let chr = call(mem, mem.rebase(ALLOCATE), &[key]);
        mem.write(chr.wrapping_add(0x190), v);
        chr
    }

    /// `Ch_RomDataReadInit()` (`0x0082a5f0`), `Ch_DrawInitAll()`
    /// (`0x0082ed30`), `sg_getdbname(kind)` (`0x00829c90`),
    /// `Sg_PcReadD(path)` (`0x0083ae50`), `Sg_PcReadGetLen()`
    /// (`0x0083ae70`), `dbgPrintf` (`0x0062f500`), `Ch_DisposeAll`
    /// (`0x008268e0`), `Sg_InitLight()` (`0x008360b0`), `Sg_InitCamera()`
    /// (`0x0083f780`): through the game.
    pub const ROM_DATA_READ_INIT: u32 = 0x0082_a5f0;
    pub const DRAW_INIT_ALL: u32 = 0x0082_ed30;
    pub const GET_DB_NAME: u32 = 0x0082_9c90;
    pub const PC_READ_D: u32 = 0x0083_ae50;
    pub const PC_READ_GET_LEN: u32 = 0x0083_ae70;
    pub const DISPOSE_ALL: u32 = 0x0082_68e0;
    pub const INIT_LIGHT: u32 = 0x0083_60b0;
    pub const INIT_CAMERA: u32 = 0x0083_f780;
    /// `"/ffx/proj2/chr/common/%s.tbl"` and `"sizeof(CHR)=%x\n"`.
    pub const DEBUG_TBL_FORMAT: u32 = 0x00b5_95a4;
    pub const SIZEOF_FORMAT: u32 = 0x00b5_95c4;
    /// `u32` cleared by `Ch_InitAll` (after the seven debug model counts).
    pub const DEBUG_MODEL_COUNT_8: Global<u32> = Global::new(0x012f_bca4);
    /// Bytes of scratch [`ch_init_all`] needs: the path (`[ebp-0x68]`).
    pub const INIT_ALL_SCRATCH: u32 = 0x68;

    /// `Ch_InitAll()` (`0x00826ea0`; caller `Sg_MainInit`): the 40 model
    /// blocks cleared (`memclear`, 12000 bytes) and marked free (`-1`
    /// first words), `Ch_RomDataReadInit`, no character array, the eighth
    /// debug count cleared, `Ch_DrawInitAll`. Then for the seven model
    /// kinds, with `g_FFXIsDebug` (read each time) the debug model list
    /// `/ffx/proj2/chr/common/<sg_getdbname(kind)>.tbl` read from the PC
    /// (`sprintf` into `scratch`, `Sg_PcReadD`) and its length / 0x80 as
    /// the count, else both 0. Then `dbgPrintf("sizeof(CHR)=%x\n",
    /// 0x880)`, `Ch_DisposeAll`, `Sg_InitLight`, `Sg_InitCamera`. No caller
    /// reads `eax`.
    pub fn ch_init_all<M: Mem>(
        mem: &mut M,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let g = |mem: &M, f: u32| mem.rebase(f);
        let blocks = CHR_BLK.ptr(mem).addr;
        call(mem, g(mem, MEMCLEAR), &[blocks, 12000]);
        // the original walks up to g_GuScreenSize, right after the blocks
        let end = g(mem, 0x023c_bbe0);
        let mut b = blocks;
        while b.cast_signed() < end.cast_signed() {
            mem.write(b, u32::MAX);
            b = b.wrapping_add(l::BLK_SIZE);
        }
        call(mem, g(mem, ROM_DATA_READ_INIT), &[]);
        CHR_ARRAY.set(mem, 0);
        CHR_COUNT.set(mem, 0);
        DEBUG_MODEL_COUNT_8.set(mem, 0);
        call(mem, g(mem, DRAW_INIT_ALL), &[]);
        let models = DEBUG_MODELS.ptr(mem).addr;
        let counts = DEBUG_MODEL_COUNTS.ptr(mem).addr;
        let sprintf: u32 = mem.read(g(mem, SPRINTF_SLOT));
        for kind in 0..7u32 {
            if IS_DEBUG.get(mem) == 0 {
                mem.write(models + 4 * kind, 0u32);
                mem.write(counts + 4 * kind, 0u32);
                continue;
            }
            let name = call(mem, g(mem, GET_DB_NAME), &[kind]);
            call(mem, sprintf, &[scratch, g(mem, DEBUG_TBL_FORMAT), name]);
            let list = call(mem, g(mem, PC_READ_D), &[scratch]);
            mem.write(models + 4 * kind, list);
            let len = call(mem, g(mem, PC_READ_GET_LEN), &[]);
            mem.write(counts + 4 * kind, len >> 7);
        }
        call(mem, g(mem, DBG_PRINTF), &[g(mem, SIZEOF_FORMAT), l::SIZE]);
        call(mem, g(mem, DISPOSE_ALL), &[]);
        call(mem, g(mem, INIT_LIGHT), &[]);
        call(mem, g(mem, INIT_CAMERA), &[]);
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
