//! M5 stage 1, list `pc-ui`, batch 1: the input thread's controller
//! queues (`AsyncControllerTaskManager`: two `std::deque`s under a mutex),
//! the menus' texture unloads (`MenuTextureManager`), the Iggy menu
//! manager's teardown, the movie player's destructor, the text utility's
//! teardown, the debug GUI's hex/decimal parsers and a few singletons'
//! create/destroy.
//!
//! Layouts (from the originals' loads and stores):
//! - `AsyncControllerTaskManager` (`g_AsyncControllerTaskManager`,
//!   `0x00ccc860`): `+4` the run flag (0 = stop), `+0x10` a
//!   `std::deque<u32>` (the button words), `+0x24` a
//!   `std::deque<[f32; 4]>` (the sticks), `+0x3c` the thread's "stopped"
//!   flag, `+0x40` the mutex.
//! - MSVC's `std::deque` (`+0` the container proxy, `+4` the block map,
//!   `+8` the map size (a power of two), `+0xc` the first element's
//!   offset, `+0x10` the size). `u32` elements are four to a block,
//!   16-byte ones one to a block; blocks are 16 bytes.
//! - `PArray`-style buffers (`+0` capacity with bit 31 "not owned", `+4`
//!   the data) in the menu objects: freed when bit 31 is clear.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    // ---- imports (`ffx.sym` slots) ----
    /// `KERNEL32!WaitForSingleObject` (stdcall, 2 words).
    pub const WAIT_IAT: u32 = 0x00b0_c0f8;
    /// `KERNEL32!ReleaseMutex` (stdcall, 1 word).
    pub const RELEASE_MUTEX_IAT: u32 = 0x00b0_c1c0;
    /// `KERNEL32!CloseHandle` (stdcall, 1 word).
    pub const CLOSE_HANDLE_IAT: u32 = 0x00b0_c0dc;
    /// `MSVCR110!__iob_func` (cdecl).
    pub const IOB_FUNC_IAT: u32 = 0x00b0_c350;
    /// `MSVCR110!fprintf` (cdecl, varargs).
    pub const FPRINTF_IAT: u32 = 0x00b0_c38c;
    /// `MSVCR110!sscanf` (cdecl, varargs).
    pub const SSCANF_IAT: u32 = 0x00b0_c448;
    /// `MSVCR110!atoi` (cdecl).
    pub const ATOI_IAT: u32 = 0x00b0_c40c;
    /// `MSVCR110!memmove` (cdecl).
    pub const MEMMOVE_IAT: u32 = 0x00b0_c34c;
    /// `MSVCP110!std::_Xbad_alloc()` (cdecl; throws).
    pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;
    /// `MSVCP110!std::_Xlength_error(const char*)` (cdecl; throws).
    pub const XLENGTH_ERROR_IAT: u32 = 0x00b0_c29c;
    /// `"deque<T> too long"`.
    pub const DEQUE_TOO_LONG: u32 = 0x00b4_6bf0;

    // ---- globals ----
    /// `g_AsyncControllerOn` (`u32`, name ours): the controller queues are
    /// in use (both `AsyncControllerTaskManager` queue methods test it).
    pub const ASYNC_CONTROLLER_ON: u32 = 0x0133_c930;
    /// `g_AsyncControllerTaskManager` (name ours; its `sCreateInstance`
    /// `0x0068db00` stores it).
    pub const ASYNC_CONTROLLER: u32 = 0x00cc_c860;
    /// `g_TouchRecordInstance` (name ours): `0x006897d0` creates it.
    pub const TOUCH_RECORD: u32 = 0x00cc_c850;
    /// `g_ParrayInstance_00ccc858` (name ours): `0x0068ba20` creates it.
    pub const PARRAY16: u32 = 0x00cc_c858;
    /// `g_RemapButtonIcon` (`RemapButtonIcon_sDestoryInstance`).
    pub const REMAP_BUTTON_ICON: u32 = 0x00ce_63c4;
    /// `g_Singleton_00ce6dc0` (name ours): `0x006e76f0` deletes it.
    pub const SINGLETON_CE6DC0: u32 = 0x00ce_6dc0;
    /// `g_GDrawContext` (name ours): the Iggy `GDraw` context (`+4` the
    /// render command queue).
    pub const GDRAW_CONTEXT: u32 = 0x01a8_58c4;
    /// `g_GDrawContextRefs` (name ours): its reference count.
    pub const GDRAW_CONTEXT_REFS: u32 = 0x01a8_58c8;
    /// `g_IggyMenuManager`.
    pub const IGGY_MENU_MANAGER: u32 = 0x0198_4c6c;
    /// `g_GraphicUi`.
    pub const GRAPHIC_UI: u32 = 0x00cc_c81c;
    /// `g_PhyreScene`.
    pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
    /// `g_MenuTextureManager`.
    pub const MENU_TEXTURE_MANAGER: u32 = 0x00cc_c820;
    /// `g_TextUtilInstances` (name ours): 16 `TextUtil` pointers.
    pub const TEXT_UTILS: u32 = 0x00ce_8260;
    /// `"TextUtil %p not registered"`-style message `TextUtil_terminate`
    /// prints for an unknown instance.
    pub const TEXT_UTIL_UNKNOWN: u32 = 0x00b4_bebc;
    /// `g_GuWindowGlobalTrans` (name ours): `Gui_WindowSetGlobalTrans`.
    pub const GU_WINDOW_GLOBAL_TRANS: u32 = 0x0130_3730;
    /// `"%x"`.
    pub const FMT_HEX: u32 = 0x00b4_1f0c;
    /// The AS3 function name `0x00702a10` calls.
    pub const AS3_NAME_00702A10: u32 = 0x00b4_cec8;
    /// The Iggy menu `0x00702a10` calls (`g_SaveLoadMenu_00ceb248`).
    pub const IGGY_MENU_CEB248: u32 = 0x00ce_b248;

    // ---- callees ----
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    pub const INPUT_SET_INFO: u32 = 0x0063_1550;
    pub const INPUT_SWAP_STATE: u32 = 0x0063_2930;
    pub const INPUT_SET_VIBRATION: u32 = 0x0063_29c0;
    pub const COPY_ITER_U32: u32 = 0x0068_d5b0;
    pub const COPY_ITER_V4: u32 = 0x0068_d5d0;
    pub const GROW_MAP_U32: u32 = 0x0068_cf60;
    pub const GROW_MAP_V4: u32 = 0x0068_d0e0;
    pub const PUSH_U32: u32 = 0x0068_d9f0;
    pub const PUSH_V4: u32 = 0x0068_da80;
    pub const CLEAR_U32: u32 = 0x0068_d2d0;
    pub const CLEAR_V4: u32 = 0x0068_d330;
    pub const SLEEP_THREAD: u32 = 0x006f_2d10;
    pub const IGGY_CALL_AS3: u32 = 0x00a2_8ba0;
    pub const HEX_PARSE: u32 = 0x0084_8f60;
    pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
    pub const MKV_READER_DTOR: u32 = 0x00a3_e040;
    pub const FRAME_QUEUE_DTOR: u32 = 0x00a3_dd50;
    pub const TEXT_ASSET_RELEASE: u32 = 0x006f_5c40;
    pub const FUN_00A295A0: u32 = 0x00a2_95a0;
    pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
    pub const RELEASE_PCLUSTER: u32 = 0x0069_bdb0;
    pub const SET_UI_INSTANCE_UNLOADED: u32 = 0x0067_e4f0;
    pub const UNIFY_FILENAME: u32 = 0x0067_9820;
    pub const SET_TEXTURE_UNLOADED: u32 = 0x006a_3740;
    pub const SCENE_UNLOAD_MENU_TEXTURE: u32 = 0x0067_1120;
    pub const MENU_UNLOAD_TEXTURE: u32 = 0x0067_dbf0;
    pub const MEMSET_THUNK: u32 = 0x0094_964c;
    pub const SA_MENU_TEXTURE_NAME: u32 = 0x0077_4ae0;
    pub const FFX_LANG: u32 = 0x0064_10e0;
    pub const FFX_LANG_JMP: u32 = 0x008a_c2f0;
    pub const LOCALIZATION_MANAGER: u32 = 0x006d_b070;
    pub const UNLOAD_SA_MENU: u32 = 0x0067_f230;
    pub const UNLOAD_SA_LANG: u32 = 0x0067_f030;
    pub const UNLOAD_SA_NAMES: u32 = 0x0067_de60;
    pub const UNLOAD_GAME_MENU: u32 = 0x0067_ee70;
    pub const UNLOAD_BG_WATER: u32 = 0x0067_ecf0;
    pub const UNLOAD_DEFAULT_MENU: u32 = 0x0067_e9d0;
    pub const UNLOAD_DEFAULT_TEXTURES: u32 = 0x0068_57c0;
    pub const MENU_TREE_RELEASE: u32 = 0x006a_1230;
    pub const FUN_006A1550: u32 = 0x006a_1550;
    pub const MENU_MESHES_RELEASE: u32 = 0x0067_b680;
    pub const FUN_0067B850: u32 = 0x0067_b850;
    pub const MENU_UNLOAD_EXTRA: u32 = 0x0064_2380;
    pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;
    pub const GDRAW_RELEASE_QUEUE: u32 = 0x00a3_d240;
    pub const GDRAW_RELEASE: u32 = 0x00a3_d570;
    pub const RING_JOB_DTOR: u32 = 0x00a3_4ab0;
    pub const IGGY_MANAGER_DTOR: u32 = 0x00a2_5eb0;
    pub const QUEUE_CALLBACK: u32 = 0x005b_5880;
    pub const SYNC_RENDER: u32 = 0x005b_c3f0;
    /// The render-thread trampoline both `GDraw` commands go through.
    pub const GDRAW_TRAMPOLINE: u32 = 0x00a3_4b00;
    /// The `GDraw` context's flush command (`0x00a3d240`).
    pub const GDRAW_FLUSH_CMD: u32 = 0x00a3_7960;
    /// The end-of-frame command (`IggyMenuManager_EndDraw_00a362e0`).
    pub const GDRAW_END_CMD: u32 = 0x00a3_7fe0;
    pub const TOUCH_RECORD_RESET: u32 = 0x0068_9510;
    pub const PARRAY_RESET16: u32 = 0x0068_bc60;

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set_g(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn rd(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    fn delete(h: &mut impl Host, p: u32) {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }

    fn phyre_free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    // ---- the controller queues ----

    /// `AsyncControllerTaskManager_GetControllerInfo(this; &word, &sticks)`
    /// (`ret 8`). `frame` is the original's `ebp - 0x10` (four words,
    /// zeroed; the deque iterators go to `frame + 4`). Without
    /// [`ASYNC_CONTROLLER_ON`] or with the button queue empty: `*word = 0`
    /// and the sticks zero. Else under the mutex `+0x40`: the front of
    /// both queues (the button deque's `+0x10` size and `+0xc` offset
    /// stepped with their own `if` chains). `eax`: 0, or `ReleaseMutex`'s.
    pub fn get_controller_info(
        h: &mut impl Host,
        this: u32,
        word: u32,
        sticks: u32,
        frame: u32,
    ) -> u32 {
        for i in 0..4 {
            h.write(frame + 4 * i, 0u32);
        }
        if g(h, ASYNC_CONTROLLER_ON) == 0 {
            zero_info(h, word, sticks);
            return 0;
        }
        let mutex = rd(h, this + 0x40);
        import(h, WAIT_IAT, Conv::Stdcall, &[mutex, 0xffff_ffff]);
        if rd(h, this + 0x20) == 0 {
            zero_info(h, word, sticks);
            let mutex = rd(h, this + 0x40);
            return import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex]);
        }
        // the button word
        let it = h.call_sym(COPY_ITER_U32, Conv::Thiscall(this + 0x10), &[frame + 4]);
        let proxy = rd(h, it);
        let cont = if proxy == 0 { 0 } else { rd(h, proxy) };
        let off = rd(h, it + 8);
        let mask = rd(h, cont + 8).wrapping_sub(1);
        let map = rd(h, cont + 4);
        let block = rd(h, map.wrapping_add(((off >> 2) & mask).wrapping_mul(4)));
        let v = rd(h, block.wrapping_add((off & 3) * 4));
        h.write(word, v);
        let n = rd(h, this + 0x20);
        if n != 0 {
            h.write(this + 0x20, n - 1);
            if n == 1 {
                h.write(this + 0x1c, 0u32);
            } else {
                let o = rd(h, this + 0x1c);
                h.write(this + 0x1c, o.wrapping_add(1));
            }
        }
        // the sticks
        let it = h.call_sym(COPY_ITER_V4, Conv::Thiscall(this + 0x24), &[frame + 4]);
        let proxy = rd(h, it);
        let cont = if proxy == 0 { 0 } else { rd(h, proxy) };
        let mask = rd(h, cont + 8).wrapping_sub(1);
        let map = rd(h, cont + 4);
        let block = rd(h, map.wrapping_add((mask & rd(h, it + 8)).wrapping_mul(4)));
        for i in 0..4 {
            let x = rd(h, block.wrapping_add(4 * i));
            h.write(sticks + 4 * i, x);
        }
        let n = rd(h, this + 0x34);
        if n != 0 {
            h.write(this + 0x34, n - 1);
            if n == 1 {
                h.write(this + 0x30, 0u32);
            } else {
                let o = rd(h, this + 0x30);
                h.write(this + 0x30, o.wrapping_add(1));
            }
        }
        let mutex = rd(h, this + 0x40);
        import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex])
    }

    /// No input: the word and the four stick floats zero.
    fn zero_info(h: &mut impl Host, word: u32, sticks: u32) {
        h.write(word, 0u32);
        for i in 0..4 {
            h.write(sticks + 4 * i, 0u32);
        }
    }

    /// `AsyncControllerTaskManager_PushControllerInfo(this; word, s0, s1,
    /// s2, s3)` (`ret 0x14`). `word_at` is the address of the caller's
    /// `word` slot (the original hands its own argument slot to the
    /// push), `frame` the original's `ebp - 0x14` (the four stick words).
    /// Under [`ASYNC_CONTROLLER_ON`] and the mutex: both pushes. `eax`:
    /// `s3` without the flag, else `ReleaseMutex`'s.
    pub fn push_controller_info(
        h: &mut impl Host,
        this: u32,
        word_at: u32,
        sticks: [u32; 4],
        frame: u32,
    ) -> u32 {
        for (i, s) in (0u32..).zip(sticks) {
            h.write(frame + 4 * i, s);
        }
        if g(h, ASYNC_CONTROLLER_ON) == 0 {
            return sticks[3];
        }
        let mutex = rd(h, this + 0x40);
        import(h, WAIT_IAT, Conv::Stdcall, &[mutex, 0xffff_ffff]);
        h.call_sym(PUSH_U32, Conv::Thiscall(this + 0x10), &[word_at]);
        h.call_sym(PUSH_V4, Conv::Thiscall(this + 0x24), &[frame]);
        let mutex = rd(h, this + 0x40);
        import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex])
    }

    /// `AsyncControllerTaskManager_ClearControllerInfo(this)`: under
    /// [`ASYNC_CONTROLLER_ON`] and the mutex, both deques cleared.
    pub fn clear_controller_info(h: &mut impl Host, this: u32) {
        if g(h, ASYNC_CONTROLLER_ON) == 0 {
            return;
        }
        let mutex = rd(h, this + 0x40);
        import(h, WAIT_IAT, Conv::Stdcall, &[mutex, 0xffff_ffff]);
        h.call_sym(CLEAR_U32, Conv::Thiscall(this + 0x10), &[]);
        h.call_sym(CLEAR_V4, Conv::Thiscall(this + 0x24), &[]);
        let mutex = rd(h, this + 0x40);
        import(h, RELEASE_MUTEX_IAT, Conv::Stdcall, &[mutex]);
    }

    /// `0x0068d2d0` / `0x0068d330` (`std::deque::_Tidy`, thiscall on the
    /// deque): size and offset to 0 (when the size isn't), every block
    /// from the last deleted, then the map; map and map size to 0.
    pub fn deque_tidy(h: &mut impl Host, this: u32) {
        if rd(h, this + 0x10) != 0 {
            h.write(this + 0xc, 0u32);
            h.write(this + 0x10, 0u32);
        }
        let mut n = rd(h, this + 8);
        while n != 0 {
            n -= 1;
            let block = rd(h, rd(h, this + 4).wrapping_add(4 * n));
            if block != 0 {
                delete(h, block);
            }
        }
        let map = rd(h, this + 4);
        if map != 0 {
            delete(h, map);
        }
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
    }

    /// `std::deque::_Growmap(this; count)` (`ret 4`): `0x0068cf60` for the
    /// `u32` deque (`shift` 2: the offset's block is `off >> 2`),
    /// `0x0068d0e0` for the 16-byte one (`shift` 0). The map size doubles
    /// from `max(size, 1)` until it grows by at least `count` and is at
    /// least 8 (`_Xlength_error` past `0x0fffffff`); the new map
    /// (`operator new`, `_Xbad_alloc` on failure or past `0x3fffffff`
    /// words) gets the old blocks from the first one on at the same
    /// index, those before it after them (or, when there are more of
    /// those than new slots, the first ones after them and the rest at
    /// the start), and zeros elsewhere; the old map is deleted.
    pub fn grow_map(h: &mut impl Host, this: u32, count: u32, shift: u32) {
        let size = rd(h, this + 8);
        let mut new_size = if size == 0 { 1 } else { size };
        while new_size.wrapping_sub(size) < count || new_size < 8 {
            if 0x0fff_ffff - new_size < new_size {
                let msg = h.rebase(DEQUE_TOO_LONG);
                import(h, XLENGTH_ERROR_IAT, Conv::Cdecl, &[msg]);
                return;
            }
            new_size += new_size;
        }
        let inc = new_size.wrapping_sub(size);
        let first = rd(h, this + 0xc) >> shift;
        let total = size.wrapping_add(inc);
        let mut new_map = 0;
        if total != 0 {
            if total > 0x3fff_ffff {
                import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
            } else {
                new_map = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[total << 2]);
                if new_map == 0 {
                    import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
                }
            }
        }
        let map = rd(h, this + 4);
        let size = rd(h, this + 8);
        let src = map.wrapping_add(first.wrapping_mul(4));
        let tail = words_between(src, map.wrapping_add(size.wrapping_mul(4)));
        let at = import(
            h,
            MEMMOVE_IAT,
            Conv::Cdecl,
            &[new_map.wrapping_add(first.wrapping_mul(4)), src, tail],
        );
        let end = tail.wrapping_add(at);
        let mut zero_at;
        let zeros;
        if first <= inc {
            let head = words_between(map, map.wrapping_add(first.wrapping_mul(4)));
            let at = import(h, MEMMOVE_IAT, Conv::Cdecl, &[end, map, head]);
            let mut p = head.wrapping_add(at);
            for _ in 0..inc - first {
                h.write(p, 0u32);
                p = p.wrapping_add(4);
            }
            zero_at = new_map;
            zeros = first;
        } else {
            let moved = inc.wrapping_mul(4);
            import(h, MEMMOVE_IAT, Conv::Cdecl, &[end, map, moved & !3]);
            let map = rd(h, this + 4);
            let rest_src = moved.wrapping_add(map);
            let rest = words_between(rest_src, map.wrapping_add(first.wrapping_mul(4)));
            let at = import(h, MEMMOVE_IAT, Conv::Cdecl, &[new_map, rest_src, rest]);
            zero_at = rest.wrapping_add(at);
            zeros = inc;
        }
        for _ in 0..zeros {
            h.write(zero_at, 0u32);
            zero_at = zero_at.wrapping_add(4);
        }
        let map = rd(h, this + 4);
        if map != 0 {
            delete(h, map);
        }
        let s = rd(h, this + 8);
        h.write(this + 8, s.wrapping_add(inc));
        h.write(this + 4, new_map);
    }

    /// `((to - from) sar 2) * 4`: the bytes between two word pointers as
    /// the original computes them.
    fn words_between(from: u32, to: u32) -> u32 {
        (to.wrapping_sub(from).cast_signed() >> 2)
            .cast_unsigned()
            .wrapping_mul(4)
    }

    /// `0x0068d9f0` (`std::deque<u32>::push_back(this; &v)`, `ret 4`).
    pub fn push_u32(h: &mut impl Host, this: u32, v: u32) {
        let size = rd(h, this + 0x10);
        if rd(h, this + 0xc).wrapping_add(size).trailing_zeros() >= 2
            && rd(h, this + 8) <= size.wrapping_add(4) >> 2
        {
            h.call_sym(GROW_MAP_U32, Conv::Thiscall(this), &[1]);
        }
        let map_size = rd(h, this + 8);
        let off = rd(h, this + 0xc) & map_size.wrapping_mul(4).wrapping_sub(1);
        h.write(this + 0xc, off);
        let pos = rd(h, this + 0x10).wrapping_add(off);
        let slot = ((pos >> 2) & map_size.wrapping_sub(1)) * 4;
        let block = new_block(h, this, slot);
        let at = block.wrapping_add((pos & 3) * 4);
        if at != 0 {
            let x = rd(h, v);
            h.write(at, x);
        }
        let s = rd(h, this + 0x10);
        h.write(this + 0x10, s.wrapping_add(1));
    }

    /// `0x0068da80` (`std::deque<[f32; 4]>::push_back(this; &v)`, `ret 4`).
    pub fn push_v4(h: &mut impl Host, this: u32, v: u32) {
        if rd(h, this + 8) <= rd(h, this + 0x10).wrapping_add(1) {
            h.call_sym(GROW_MAP_V4, Conv::Thiscall(this), &[1]);
        }
        let mask = rd(h, this + 8).wrapping_sub(1);
        let off = rd(h, this + 0xc) & mask;
        h.write(this + 0xc, off);
        let pos = rd(h, this + 0x10).wrapping_add(off);
        let slot = (mask & pos).wrapping_mul(4);
        let block = new_block(h, this, slot);
        if block != 0 {
            for i in 0..4 {
                let x = rd(h, v + 4 * i);
                h.write(block + 4 * i, x);
            }
        }
        let s = rd(h, this + 0x10);
        h.write(this + 0x10, s.wrapping_add(1));
    }

    /// The block at map byte `slot`, allocated (16 bytes) when empty.
    fn new_block(h: &mut impl Host, this: u32, slot: u32) -> u32 {
        if rd(h, rd(h, this + 4).wrapping_add(slot)) == 0 {
            let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x10]);
            if p == 0 {
                import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
            }
            let at = rd(h, this + 4).wrapping_add(slot);
            h.write(at, p);
        }
        rd(h, rd(h, this + 4).wrapping_add(slot))
    }

    /// `0x0068db80` (`AsyncControllerTaskManager::sDestroyInstance`, from
    /// `graphicFinalize`): the run flag `+4` cleared, `SleepThread` until
    /// the thread says it stopped (`+0x3c`), the mutex closed, both
    /// deques torn down and the manager deleted.
    pub fn async_controller_destroy(h: &mut impl Host) {
        if g(h, ASYNC_CONTROLLER) == 0 {
            return;
        }
        let m = g(h, ASYNC_CONTROLLER);
        h.write(m + 4, 0u32);
        let mut m = g(h, ASYNC_CONTROLLER);
        while rd(h, m + 0x3c) == 0 {
            h.call_sym(SLEEP_THREAD, Conv::Cdecl, &[]);
            m = g(h, ASYNC_CONTROLLER);
        }
        let mutex = rd(h, m + 0x40);
        import(h, CLOSE_HANDLE_IAT, Conv::Stdcall, &[mutex]);
        h.call_sym(CLEAR_V4, Conv::Thiscall(m + 0x24), &[]);
        let p = rd(h, m + 0x24);
        delete(h, p);
        h.write(m + 0x24, 0u32);
        h.call_sym(CLEAR_U32, Conv::Thiscall(m + 0x10), &[]);
        let p = rd(h, m + 0x10);
        delete(h, p);
        h.write(m + 0x10, 0u32);
        delete(h, m);
        set_g(h, ASYNC_CONTROLLER, 0);
    }

    // ---- input forwarders ----

    /// `inputSetInputInfoToCurrentFrame()`: `setInputInfo` on the input
    /// manager (a tail jump); its `eax`.
    pub fn set_input_info_to_current_frame(h: &mut impl Host) -> u32 {
        let m = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(INPUT_SET_INFO, Conv::Thiscall(m), &[])
    }

    /// `0x00630df0(out, a, b, c)` -> `out`: the input manager's
    /// `SwapState(out, a, b, c)`.
    pub fn input_swap_state(h: &mut impl Host, out: u32, a: u32, b: u32, c: u32) -> u32 {
        let m = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(INPUT_SWAP_STATE, Conv::Thiscall(m), &[out, a, b, c]);
        out
    }

    /// `inputSetVibration(a, b, c)`: the input manager's
    /// `setVibration(a, b, c)` (`ret 0xc`); its `eax`.
    pub fn input_set_vibration(h: &mut impl Host, a: u32, b: u32, c: u32) -> u32 {
        let m = h.call_sym(INPUT_MANAGER, Conv::Cdecl, &[]);
        h.call_sym(INPUT_SET_VIBRATION, Conv::Thiscall(m), &[a, b, c])
    }

    /// `0x006897d0()`: when [`TOUCH_RECORD`] is unset, a new 0x160-byte
    /// record (`TouchRecord_Reset`, `+0x150..+0x15c` zero) or 0.
    pub fn touch_record_create(h: &mut impl Host) {
        if g(h, TOUCH_RECORD) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x160]);
        if p == 0 {
            set_g(h, TOUCH_RECORD, 0);
            return;
        }
        h.call_sym(TOUCH_RECORD_RESET, Conv::Thiscall(p), &[]);
        for off in [0x150, 0x154, 0x158, 0x15c] {
            h.write(p + off, 0u32);
        }
        set_g(h, TOUCH_RECORD, p);
    }

    /// `0x0068ba20()`: when [`PARRAY16`] is unset, a new 8-byte array
    /// (both words zero, `PArray_reset16`) or 0.
    pub fn parray16_create(h: &mut impl Host) {
        if g(h, PARRAY16) != 0 {
            return;
        }
        let mut p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        if p == 0 {
            p = 0;
        } else {
            h.write(p, 0u32);
            h.write(p + 4, 0u32);
            h.call_sym(PARRAY_RESET16, Conv::Thiscall(p), &[]);
        }
        set_g(h, PARRAY16, p);
    }

    /// `RemapButtonIcon_sDestoryInstance` and `0x006e76f0`: the singleton
    /// at `sym` deleted and cleared when set.
    pub fn delete_singleton(h: &mut impl Host, sym: u32) {
        let p = g(h, sym);
        if p != 0 {
            delete(h, p);
            set_g(h, sym, 0);
        }
    }

    /// `0x00702a10()`: `callIggyAS3External(g_SaveLoadMenu_00ceb248; 0,
    /// name, 0, 0, 1)`; its `eax`.
    pub fn save_menu_call_as3(h: &mut impl Host) -> u32 {
        let menu = g(h, IGGY_MENU_CEB248);
        let name = h.rebase(AS3_NAME_00702A10);
        h.call_sym(IGGY_CALL_AS3, Conv::Thiscall(menu), &[0, name, 0, 0, 1])
    }

    // ---- the debug GUI ----

    /// `Gui_WindowSetGlobalTrans(v)`: `v` clamped to `0..=0x80` (signed).
    pub fn gui_window_set_global_trans(h: &mut impl Host, v: u32) {
        // `clamp`: rustc 1.99 -O folds `if v > 0x80 { 0x80 } else { v.max(0) }`
        // to `v.max(0)` (a miscompile; seen on this function's test)
        let t = v.cast_signed().clamp(0, 0x80);
        set_g(h, GU_WINDOW_GLOBAL_TRANS, t.cast_unsigned());
    }

    /// Whether `c` is a hex digit for `0x00848f60` (signed compares: bytes
    /// past 0x7f are not).
    fn is_hex(c: u8) -> bool {
        c.is_ascii_hexdigit()
    }

    /// `0x00848f60(s, out)` -> the end of the digits: the hex digits at
    /// `s` copied to the 16-byte buffer `frame` (the original's
    /// `ebp - 0x14`), `sscanf(buf, "%x", out)`. Sixteen or more digits
    /// overrun the buffer: `report_rangecheckfailure` (it ends the
    /// process; the original wrote past its buffer first).
    pub fn hex_parse(h: &mut impl Host, s: u32, out: u32, frame: u32) -> u32 {
        let mut p = s;
        let mut n = 0u32;
        loop {
            let c: u8 = h.read(p);
            if !is_hex(c) {
                break;
            }
            if n < 16 {
                h.write(frame + n, c);
            }
            n += 1;
            p = p.wrapping_add(1);
        }
        if n >= 16 {
            h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
            return p;
        }
        h.write(frame + n, 0u8);
        let fmt = h.rebase(FMT_HEX);
        import(h, SSCANF_IAT, Conv::Cdecl, &[frame, fmt, out]);
        p
    }

    /// `0x00848fe0(s, out)` -> the end of the number: `$` and hex
    /// (`0x00848f60` on `s + 1`), else the decimal digits at `s` through
    /// `atoi` into `*out` (the buffer `frame` as in [`hex_parse`]).
    pub fn number_parse(h: &mut impl Host, s: u32, out: u32, frame: u32) -> u32 {
        let mut c: u8 = h.read(s);
        if c == b'$' {
            return h.call_sym(HEX_PARSE, Conv::Cdecl, &[s.wrapping_add(1), out]);
        }
        let mut p = s;
        let mut n = 0u32;
        if c.cast_signed() >= 0x30_i8 {
            while c.cast_signed() <= 0x39_i8 {
                if n < 16 {
                    h.write(frame + n, c);
                }
                p = p.wrapping_add(1);
                c = h.read(p);
                n += 1;
                if c.cast_signed() < 0x30_i8 {
                    break;
                }
            }
            if n >= 16 {
                h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[]);
                return p;
            }
        }
        h.write(frame + n, 0u8);
        let v = import(h, ATOI_IAT, Conv::Cdecl, &[frame]);
        h.write(out, v);
        p
    }

    // ---- teardown ----

    /// `PVideoPlaybackWin32_dtor_00a26f30(this)`: the reader `+0x118` and
    /// the frame queues `+0x80`, `+0x40`, `+0`; the last one's `eax`.
    pub fn video_playback_dtor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(MKV_READER_DTOR, Conv::Thiscall(this + 0x118), &[]);
        h.call_sym(FRAME_QUEUE_DTOR, Conv::Thiscall(this + 0x80), &[]);
        h.call_sym(FRAME_QUEUE_DTOR, Conv::Thiscall(this + 0x40), &[]);
        h.call_sym(FRAME_QUEUE_DTOR, Conv::Thiscall(this), &[])
    }

    /// `TextUtil_terminate(this)`: out of [`TEXT_UTILS`] (or a message to
    /// `stderr` when not there), the asset `+0xc` and the object `+0x10`
    /// released and freed, both clusters `+0`, `+4` released;
    /// `releasePCluster`'s `eax`.
    pub fn text_util_terminate(h: &mut impl Host, this: u32) -> u32 {
        let table = h.rebase(TEXT_UTILS);
        let mut found = None;
        for i in 0..16 {
            if rd(h, table + 4 * i) == this {
                found = Some(i);
                break;
            }
        }
        if let Some(i) = found {
            h.write(table + 4 * i, 0u32);
        } else {
            let iob = import(h, IOB_FUNC_IAT, Conv::Cdecl, &[]);
            let fmt = h.rebase(TEXT_UTIL_UNKNOWN);
            import(
                h,
                FPRINTF_IAT,
                Conv::Cdecl,
                &[iob.wrapping_add(0x40), fmt, this],
            );
        }
        let asset = rd(h, this + 0xc);
        if asset != 0 {
            h.call_sym(TEXT_ASSET_RELEASE, Conv::Thiscall(asset), &[]);
            phyre_free(h, asset);
            h.write(this + 0xc, 0u32);
        }
        let obj = rd(h, this + 0x10);
        if obj != 0 {
            h.call_sym(FUN_00A295A0, Conv::Thiscall(obj), &[]);
            phyre_free(h, obj);
            h.write(this + 0x10, 0u32);
        }
        let mut eax = 0;
        for i in 0..2 {
            let cluster = rd(h, this + 4 * i);
            let m = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
            eax = h.call_sym(RELEASE_PCLUSTER, Conv::Thiscall(m), &[cluster]);
        }
        eax
    }

    /// One menu texture unloaded by name (`name`, a C string): the UI's
    /// instance marked unloaded, the name unified into `buf` (256 bytes),
    /// the menu material's texture and the scene's texture unloaded;
    /// `PhyreScene_unloadMenuTexture`'s `eax`.
    pub fn unload_named(h: &mut impl Host, name: u32, buf: u32) -> u32 {
        let ui = g(h, GRAPHIC_UI);
        h.call_sym(SET_UI_INSTANCE_UNLOADED, Conv::Thiscall(ui), &[name]);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[name, buf, 0x100]);
        let ui = g(h, GRAPHIC_UI);
        let geo = rd(h, rd(h, ui + 0x2bd4));
        h.call_sym(SET_TEXTURE_UNLOADED, Conv::Thiscall(geo), &[buf]);
        let scene = g(h, PHYRE_SCENE);
        h.call_sym(SCENE_UNLOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[buf])
    }

    /// [`unload_named`] for the name in the global `sym`, when set.
    fn unload_slot(h: &mut impl Host, sym: u32, buf: u32) -> Option<u32> {
        let name = g(h, sym);
        (name != 0).then(|| unload_named(h, name, buf))
    }

    /// `MenuTextureManager_UnloadBGWaterTexture()`: the names in
    /// `0x00c59d68` and `0x00c59d64` ([`unload_named`]; `buf` the
    /// original's `ebp - 0x104`).
    pub fn unload_bg_water(h: &mut impl Host, buf: u32) {
        unload_slot(h, 0x00c5_9d68, buf);
        unload_slot(h, 0x00c5_9d64, buf);
    }

    /// `MenuTextureManager_UnloadGameMenuTexture()`: `0x00c59d78`,
    /// `0x00c59d7c`, `0x00c59d74`.
    pub fn unload_game_menu(h: &mut impl Host, buf: u32) {
        unload_slot(h, 0x00c5_9d78, buf);
        unload_slot(h, 0x00c5_9d7c, buf);
        unload_slot(h, 0x00c5_9d74, buf);
    }

    /// `0x0067de60()`: the twelve save-menu textures 5..=16
    /// (`getSa_MenuTextureName` into a cleared buffer, then
    /// [`unload_named`]). `frame` is the original's `ebp - 0x234`: the ids
    /// at `+0`, the unified name at `+0x30`, the name at `+0x130`.
    pub fn unload_sa_names(h: &mut impl Host, frame: u32) {
        for i in 0..12 {
            h.write(frame + 4 * i, 5 + i);
        }
        let name = frame + 0x130;
        let buf = frame + 0x30;
        for i in 0..12 {
            h.call_sym(MEMSET_THUNK, Conv::Cdecl, &[name, 0, 0x100]);
            let id = rd(h, frame + 4 * i);
            h.call_sym(SA_MENU_TEXTURE_NAME, Conv::Cdecl, &[name, id]);
            unload_named(h, name, buf);
        }
    }

    /// The four texture ids `0x0067f030` unloads for language `lang`
    /// (its jump table): `0x00c59dcc + 0x10 k`.
    #[must_use]
    pub fn sa_lang_group(lang: u32) -> u32 {
        let k = match lang {
            1..=5 => lang,
            9 => 6,
            10 => 7,
            _ => 0,
        };
        0x00c5_9dcc + 0x10 * k
    }

    /// `0x0067f030(this)` (thiscall): the language's four save-menu
    /// textures (`MenuUnloadTexture(this; id)`); the last one's `eax`.
    pub fn unload_sa_lang(h: &mut impl Host, this: u32) -> u32 {
        let lang = h.call_sym(FFX_LANG, Conv::Cdecl, &[]);
        let base = sa_lang_group(lang);
        let mut eax = 0;
        for i in 0..4 {
            let id = g(h, base + 4 * i);
            eax = h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[id]);
        }
        eax
    }

    /// `MenuTextureManager_UnloadSaMenuTexture(this)`: [`unload_sa_names`]
    /// (`0x0067de60`), the language's four textures for Japanese (0),
    /// Chinese (9) and Korean (10), the names in `0x025d16cc` and
    /// `0x025d16c8`, then `0x0067f030`; its `eax`.
    pub fn unload_sa_menu(h: &mut impl Host, this: u32, buf: u32) -> u32 {
        h.call_sym(UNLOAD_SA_NAMES, Conv::Cdecl, &[]);
        let ids: &[u32] = match h.call_sym(FFX_LANG_JMP, Conv::Cdecl, &[]) {
            0 => &[0x00c5_9938, 0x00c5_993c, 0x00c5_9878, 0x00c5_987c],
            9 => &[0x00c5_99c0, 0x00c5_99c4, 0x00c5_98f8, 0x00c5_98fc],
            10 => &[0x00c5_9980, 0x00c5_9984, 0x00c5_98b8, 0x00c5_98bc],
            _ => &[],
        };
        for &sym in ids {
            let id = g(h, sym);
            h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[id]);
        }
        unload_slot(h, 0x025d_16cc, buf);
        unload_slot(h, 0x025d_16c8, buf);
        h.call_sym(UNLOAD_SA_LANG, Conv::Thiscall(this), &[])
    }

    /// The nine scene texture names `UnLoadDefaultMenuTexture` unloads for
    /// language `lang` (its jump table).
    #[must_use]
    pub fn default_names(lang: u32) -> u32 {
        match lang {
            0 => 0x00c3_34f0,
            2 => 0x00c3_3580,
            3 => 0x00c3_355c,
            4 => 0x00c3_3538,
            5 => 0x00c3_35a4,
            9 => 0x00c3_35c8,
            10 => 0x00c3_35ec,
            _ => 0x00c3_3514,
        }
    }

    /// `MenuTextureManager_UnLoadDefaultMenuTexture(this)`: the
    /// language's nine scene textures, the game menu's and the water's,
    /// the names in `0x00c59d84`, `0x00c59d6c` and `this + 0x20c` (that
    /// one cleared), the save menu's, then
    /// `MenuTextureManager_unloadDefaultTextures_006857c0`; its `eax`.
    pub fn unload_default_menu(h: &mut impl Host, this: u32, buf: u32) -> u32 {
        let loc = h.call_sym(LOCALIZATION_MANAGER, Conv::Cdecl, &[]);
        let names = h.rebase(default_names(rd(h, loc + 4)));
        for i in 0..9 {
            let name = rd(h, names + 4 * i);
            let scene = g(h, PHYRE_SCENE);
            h.call_sym(SCENE_UNLOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[name]);
        }
        h.call_sym(UNLOAD_GAME_MENU, Conv::Cdecl, &[]);
        h.call_sym(UNLOAD_BG_WATER, Conv::Cdecl, &[]);
        unload_slot(h, 0x00c5_9d84, buf);
        unload_slot(h, 0x00c5_9d6c, buf);
        let name = rd(h, this + 0x20c);
        if name != 0 {
            unload_named(h, name, buf);
            h.write(this + 0x20c, 0u32);
        }
        h.call_sym(UNLOAD_SA_MENU, Conv::Thiscall(this), &[]);
        h.call_sym(UNLOAD_DEFAULT_TEXTURES, Conv::Cdecl, &[])
    }

    /// `MenuTextureManager_UnLoadExtraMenuTexture(this)`: four scene
    /// textures, `0x00c59568..` when `*this` is 0, else `0x00c595e8..`;
    /// the last one's `eax`.
    pub fn unload_extra_menu(h: &mut impl Host, this: u32) -> u32 {
        let ids = if rd(h, this) == 0 {
            [0x00c5_9568, 0x00c5_956c, 0x00c5_95a8, 0x00c5_95ac]
        } else {
            [0x00c5_95e8, 0x00c5_95ec, 0x00c5_9628, 0x00c5_962c]
        };
        let mut eax = 0;
        for sym in ids {
            let scene = g(h, PHYRE_SCENE);
            let id = g(h, sym);
            eax = h.call_sym(SCENE_UNLOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[id]);
        }
        eax
    }

    /// `0x0067b680(this)` (thiscall, the menu manager): when the mesh
    /// table `+0x15f0` has a first entry, its tree released, then every
    /// one of the 0x578 entries' objects (`0x006a1550`).
    pub fn menu_meshes_release(h: &mut impl Host, this: u32) {
        let table = this + 0x15f0;
        let first = rd(h, table);
        if first == 0 {
            return;
        }
        let tree = rd(h, first);
        if tree != 0 {
            h.call_sym(MENU_TREE_RELEASE, Conv::Thiscall(tree), &[]);
        }
        for k in 0..0x578 {
            let e = rd(h, table + 4 * k);
            if e != 0 {
                let obj = rd(h, e);
                if obj != 0 {
                    h.call_sym(FUN_006A1550, Conv::Thiscall(obj), &[]);
                }
            }
        }
    }

    /// An owned buffer (`+0` capacity, bit 31 "not owned"; `+4` data)
    /// freed when owned (and, with `null_check`, set), then both words
    /// zeroed (`data_first`: the data word first).
    fn free_buffer(h: &mut impl Host, at: u32, null_check: bool, data_first: bool) {
        let owned = rd(h, at) >> 31 == 0;
        let data = rd(h, at + 4);
        if owned && (!null_check || data != 0) {
            phyre_free(h, data);
        }
        if data_first {
            h.write(at + 4, 0u32);
            h.write(at, 0u32);
        } else {
            h.write(at, 0u32);
            h.write(at + 4, 0u32);
        }
    }

    /// `0x0067b230(this)` (thiscall, the menu manager's teardown): the
    /// meshes, two arrays `+0x5b80`, `+0x5b84`, `0x0067b850`, the default
    /// and extra menu textures, `+0x5b88` cleared and eight buffers.
    pub fn menu_manager_teardown(h: &mut impl Host, this: u32) {
        h.call_sym(MENU_MESHES_RELEASE, Conv::Thiscall(this), &[]);
        let p = rd(h, this + 0x5b80);
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
        let p = rd(h, this + 0x5b84);
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p]);
        h.call_sym(FUN_0067B850, Conv::Thiscall(this), &[]);
        let mgr = g(h, MENU_TEXTURE_MANAGER);
        h.call_sym(UNLOAD_DEFAULT_MENU, Conv::Thiscall(mgr), &[]);
        h.call_sym(MENU_UNLOAD_EXTRA, Conv::Cdecl, &[]);
        h.write(this + 0x5b88, 0u32);
        free_buffer(h, this + 0x5830, true, true);
        free_buffer(h, this + 0x5828, true, true);
        free_buffer(h, this + 0x4240, false, true);
        free_buffer(h, this + 0x4238, false, true);
        free_buffer(h, this + 0x2bd8, true, true);
        free_buffer(h, this + 0x2bd0, true, true);
        free_buffer(h, this + 0x15e8, false, true);
        free_buffer(h, this + 0x15e0, false, false);
    }

    // ---- the Iggy menu manager and GDraw ----

    /// `0x00a3d240()`: with a `GDraw` context and its queue, the flush
    /// command queued, the queue's vtable slot 2, then
    /// `PRenderer::syncRender` on it (a tail jump).
    pub fn gdraw_flush(h: &mut impl Host) {
        let ctx = g(h, GDRAW_CONTEXT);
        if ctx == 0 || rd(h, ctx + 4) == 0 {
            return;
        }
        let q = rd(h, ctx + 4);
        let tramp = h.rebase(GDRAW_TRAMPOLINE);
        let cmd = h.rebase(GDRAW_FLUSH_CMD);
        h.call_sym(QUEUE_CALLBACK, Conv::Thiscall(q), &[tramp, cmd]);
        let q = rd(h, ctx + 4);
        h.call_virtual(q, 2, &[]);
        let q = rd(h, ctx + 4);
        h.call_sym(SYNC_RENDER, Conv::Thiscall(q), &[]);
    }

    /// `0x00a3d570()` -> 0: the `GDraw` context's reference dropped; the
    /// last one destroys it (`PRingBufferedThreadPoolJob` destructor,
    /// `PhyreFree`).
    pub fn gdraw_release(h: &mut impl Host) -> u32 {
        let n = g(h, GDRAW_CONTEXT_REFS);
        if n == 1 {
            let p = g(h, GDRAW_CONTEXT);
            set_g(h, GDRAW_CONTEXT, 0);
            if p != 0 {
                h.call_sym(RING_JOB_DTOR, Conv::Thiscall(p), &[]);
                phyre_free(h, p);
                let n = g(h, GDRAW_CONTEXT_REFS);
                set_g(h, GDRAW_CONTEXT_REFS, n.wrapping_sub(1));
                return 0;
            }
        }
        set_g(h, GDRAW_CONTEXT_REFS, n.wrapping_sub(1));
        0
    }

    /// `IggyMenuManager_EndDraw_00a362e0()`: with a `GDraw` context and its
    /// queue, the end command queued. `eax`: 0 without a context, the
    /// context without a queue, else `queueCallback`'s.
    pub fn end_draw(h: &mut impl Host) -> u32 {
        let ctx = g(h, GDRAW_CONTEXT);
        if ctx == 0 {
            return 0;
        }
        let q = rd(h, ctx + 4);
        if q == 0 {
            return ctx;
        }
        let tramp = h.rebase(GDRAW_TRAMPOLINE);
        let cmd = h.rebase(GDRAW_END_CMD);
        h.call_sym(QUEUE_CALLBACK, Conv::Thiscall(q), &[tramp, cmd])
    }

    /// `0x00a25eb0(this)` (`IggyMenuManager`'s destructor body): every
    /// menu of the list (`+0xc`, `+8 & 0x7fffffff`) destroyed, deleted
    /// and cleared; the `GDraw` context flushed (when `+4` is set) and
    /// released, `+4` cleared; the buffers `+0x10` and `+8` freed.
    pub fn iggy_manager_dtor(h: &mut impl Host, this: u32) {
        let mut i = 0;
        while i < rd(h, this + 8) & 0x7fff_ffff {
            let at = rd(h, this + 0xc).wrapping_add(4 * i);
            let menu = rd(h, at);
            if menu != 0 {
                h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(menu), &[]);
                delete(h, menu);
                let at = rd(h, this + 0xc).wrapping_add(4 * i);
                h.write(at, 0u32);
            }
            i += 1;
        }
        if rd(h, this + 4) != 0 {
            h.call_sym(GDRAW_RELEASE_QUEUE, Conv::Cdecl, &[]);
        }
        h.call_sym(GDRAW_RELEASE, Conv::Cdecl, &[]);
        h.write(this + 4, 0u32);
        free_buffer(h, this + 0x10, true, true);
        let owned = rd(h, this + 8) >> 31 == 0;
        if owned {
            let data = rd(h, this + 0xc);
            phyre_free(h, data);
        }
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
    }

    /// `0x00a26270()`: [`IGGY_MENU_MANAGER`] destroyed (`0x00a25eb0`),
    /// deleted and cleared when set.
    pub fn iggy_manager_destroy(h: &mut impl Host) {
        let m = g(h, IGGY_MENU_MANAGER);
        if m != 0 {
            h.call_sym(IGGY_MANAGER_DTOR, Conv::Thiscall(m), &[]);
            delete(h, m);
            set_g(h, IGGY_MENU_MANAGER, 0);
        }
    }
}
