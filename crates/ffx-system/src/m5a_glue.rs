//! M5 `system-a` (round 3): the small platform glue left over from the
//! earlier rounds: the input manager's forwarders (`input*`), the
//! vibration call, the PS2 clock read, `fiosLseek`, the voice-volume and
//! voice-table forwarders, unwinding destructors of std lists and trees,
//! and the controller manager's constructor and teardown.

/// The originals over game memory and the original callees.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// `operator new` (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    /// `user_free_thunk` (cdecl).
    pub const USER_FREE_THUNK: u32 = 0x0068_7010;
    /// `Phyre_PhyreFree` (cdecl).
    pub const PHYRE_FREE: u32 = 0x0042_fb20;
    /// `Virtuos_InputManager_getInstance` (cdecl).
    pub const INPUT_MANAGER: u32 = 0x0063_1300;
    /// `Virtuos_InputManager_getButtonInfo` (thiscall).
    pub const INPUT_BUTTON_INFO: u32 = 0x0063_12e0;
    /// `Virtuos_InputManager_isReleaseButton` (thiscall, `ret 4`).
    pub const INPUT_IS_RELEASE: u32 = 0x0063_1380;
    /// `Virtuos_InputManager_isRepButton` (thiscall, `ret 4`).
    pub const INPUT_IS_REP: u32 = 0x0063_13a0;
    /// `AsyncControllerTaskManager_sGetInstance` (cdecl).
    pub const CONTROLLER_MANAGER: u32 = 0x0068_dc10;
    /// `AsyncControllerTaskManager_ClearControllerInfo` (thiscall).
    pub const CLEAR_CONTROLLER_INFO: u32 = 0x0068_cc90;
    /// `Phyre_PFramework_PApplication_GetApplication` (cdecl).
    pub const GET_APPLICATION: u32 = 0x0062_3b90;
    /// `FFXApplication_pollInputAsync` (thiscall).
    pub const POLL_INPUT_ASYNC: u32 = 0x0042_f790;
    /// `Phyre_PFramework_PInputDevicePad_setVibration` (thiscall, `ret 8`).
    pub const PAD_SET_VIBRATION: u32 = 0x0062_cb70;
    /// `0x00630730` (cdecl): the input manager's teardown after the
    /// device object.
    pub const INPUT_TEARDOWN: u32 = 0x0063_0730;
    /// The device object `0x00630b10` deletes.
    pub const INPUT_DEVICE: u32 = 0x00cc_b15c;
    /// `KERNEL32!GetSystemTime` (stdcall).
    pub const GET_SYSTEM_TIME_IAT: u32 = 0x00b0_c1bc;
    /// `MSVCR110!_vsnprintf_s` (cdecl).
    pub const VSNPRINTF_S_IAT: u32 = 0x00b0_c384;
    /// `MSVCP110!std::_Xbad_alloc` (cdecl, never returns).
    pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;
    /// `PhyreFIOSStreamFile_seek` (thiscall, `ret 8`).
    pub const STREAM_SEEK: u32 = 0x006f_9d80;
    /// `ListVector_dtor_00634220` (thiscall).
    pub const LIST_VECTOR_DTOR: u32 = 0x0063_4220;
    /// `std_Tree_erase_range_00636d60` (thiscall, `ret 0xc`).
    pub const TREE_ERASE_636D60: u32 = 0x0063_6d60;
    /// `0x006370d0` (thiscall, `ret 0xc`): the same erase for another tree.
    pub const TREE_ERASE_6370D0: u32 = 0x0063_70d0;
    /// `0x00635c40` (stdcall, `ret 8`): destroys a range of elements.
    pub const DESTROY_RANGE: u32 = 0x0063_5c40;
    /// `LocalizationManager_GetInstance` (cdecl).
    pub const LOC_MANAGER: u32 = 0x006d_b070;
    /// `LocalizationManager_getText` (thiscall, `ret 0xc`).
    pub const LOC_TEXT: u32 = 0x006d_b1f0;
    /// `FmodManager_registCommandAsync3` (thiscall, `ret 0xc`).
    pub const FMOD_COMMAND3: u32 = 0x006f_a2c0;
    /// The FMOD manager (`ecx` of the voice-volume calls).
    pub const FMOD_MANAGER: u32 = 0x00ce_9010;
    /// The voice table object `0x0067a9b0` / `0x0067a9d0` hand on.
    pub const VOICE_TABLE: u32 = 0x00ce_9014;
    /// `0x006fb380` (thiscall, `ret 0x10`).
    pub const VOICE_SELECT: u32 = 0x006f_b380;
    /// `0x006fb410` (thiscall, `ret 8`).
    pub const VOICE_FIND: u32 = 0x006f_b410;
    /// `SleepThreadInMillisecond` (cdecl).
    pub const SLEEP_MS: u32 = 0x006f_2d20;
    /// The controller manager's constructor callees (`0x00630f40`):
    /// `TouchRecord_sCreateInstance_006897d0`, `TouchManager_GetInstance`,
    /// `GuiTouch_reset_0068a660` (thiscall), `MouseState_createInstance_00a25990`,
    /// `PArray16_sCreateInstance_0068ba20`, `inputGetCurTime` (`ST0`).
    pub const TOUCH_RECORD_CREATE: u32 = 0x0068_97d0;
    pub const TOUCH_MANAGER: u32 = 0x0068_98c0;
    pub const GUI_TOUCH_RESET: u32 = 0x0068_a660;
    pub const MOUSE_STATE_CREATE: u32 = 0x00a2_5990;
    pub const ARRAY16_CREATE: u32 = 0x0068_ba20;
    pub const CUR_TIME: u32 = 0x0063_0c60;
    /// The teardown callees of `0x00633f70`: `0x00636970` (thiscall),
    /// `Phyre_PFreeList_dtor_PFreeList`, `Phyre_PRedBlackTree_clear`.
    pub const TEARDOWN_636970: u32 = 0x0063_6970;
    pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
    pub const TREE_CLEAR: u32 = 0x0057_4760;

    /// The targets of the jump thunks `code_006793b0`, `code_006793c0`,
    /// `code_006793d0`: `PhyreFIOS_LoadPrxIdle`, `PhyreFIOS_LoadPrxInProgress`,
    /// `Compare_006f9b70` (the PRX state is 3), all cdecl.
    pub const PRX_IDLE: u32 = 0x006f_9b80;
    pub const PRX_IN_PROGRESS: u32 = 0x006f_9ba0;
    pub const PRX_LOADED: u32 = 0x006f_9b70;

    /// A 5-byte `jmp target` (cdecl, no arguments): `target`'s `eax`.
    pub fn jump_thunk(h: &mut impl Host, target: u32) -> u32 {
        cdecl(h, target, &[])
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// The frame `0x0062f5b0` needs: a `SYSTEMTIME` at `ebp - 0x14`.
    pub const SYSTEM_TIME_FRAME: u32 = 0x14;

    /// `0x0062f5b0(out)` (cdecl): `GetSystemTime(ebp - 0x14)`, then the
    /// low bytes of the second, minute and hour into `out + 1..=3`, the
    /// day, month and year into `out + 5..=7` (the PS2 clock layout).
    /// Returns 0.
    pub fn system_clock(h: &mut impl Host, ebp: u32, out: u32) -> u32 {
        let st = ebp.wrapping_sub(SYSTEM_TIME_FRAME);
        import(h, GET_SYSTEM_TIME_IAT, Conv::Stdcall, &[st]);
        // SYSTEMTIME: year, month, day of week, day, hour, minute, second
        for (to, from) in [(1, 12), (2, 10), (3, 8), (5, 6), (6, 2), (7, 0)] {
            let b: u8 = h.read(st.wrapping_add(from));
            h.write(out.wrapping_add(to), b);
        }
        0
    }

    /// `fiosLseek(f, offset, whence)` (cdecl; also `sceLseek`, its jump
    /// thunk): `PhyreFIOSStreamFile_seek(f; offset, whence)`'s `eax`, 0
    /// for a null `f`; `edx` = 0.
    pub fn fios_lseek(h: &mut impl Host, f: u32, offset: u32, whence: u32) -> u64 {
        if f == 0 {
            return 0;
        }
        u64::from(h.call_sym(STREAM_SEEK, Conv::Thiscall(f), &[offset, whence]))
    }

    /// `0x00630b10()` (cdecl): the device object at `0x00ccb15c` taken
    /// (the word cleared) and, when set, its deleting destructor (vtable
    /// slot 0, flags 1); then a tail jump to `0x00630730`. Its `eax`.
    pub fn input_shutdown(h: &mut impl Host) -> u32 {
        let at = h.rebase(INPUT_DEVICE);
        let dev: u32 = h.read(at);
        h.write(at, 0u32);
        if dev != 0 {
            h.call_virtual(dev, 0, &[1]);
        }
        cdecl(h, INPUT_TEARDOWN, &[])
    }

    /// `0x00631230(this; keep)` (thiscall, `ret 4`): unless `keep`'s low
    /// byte is set, `user_free_thunk(+8)` and `+8`, `+4` cleared; `+0`
    /// cleared.
    pub fn buffer_reset(h: &mut impl Host, this: u32, keep: u32) {
        if keep.to_le_bytes()[0] == 0 {
            let p: u32 = h.read(this.wrapping_add(8));
            cdecl(h, USER_FREE_THUNK, &[p]);
            h.write(this.wrapping_add(8), 0u32);
            h.write(this.wrapping_add(4), 0u32);
        }
        h.write(this, 0u32);
    }

    /// `inputClear()` (cdecl): the input manager, then a tail jump to
    /// its bare-`ret` clear (`0x00631260`, not called: it does nothing).
    /// Returns the manager.
    pub fn input_clear(h: &mut impl Host) -> u32 {
        cdecl(h, INPUT_MANAGER, &[])
    }

    /// `inputClearAllInputInfo()` (cdecl): a tail jump to
    /// `ClearControllerInfo` of the controller manager; its `eax`.
    pub fn clear_all_input_info(h: &mut impl Host) -> u32 {
        let m = cdecl(h, CONTROLLER_MANAGER, &[]);
        h.call_sym(CLEAR_CONTROLLER_INFO, Conv::Thiscall(m), &[])
    }

    /// `inputGetCurrDeviceButtonInfo()` (cdecl): the input manager's
    /// button info (a tail jump).
    pub fn button_info(h: &mut impl Host) -> u32 {
        let m = cdecl(h, INPUT_MANAGER, &[]);
        h.call_sym(INPUT_BUTTON_INFO, Conv::Thiscall(m), &[])
    }

    /// `Input_AsyncPoll()` (cdecl): a tail jump to
    /// `FFXApplication_pollInputAsync` of the application; its `eax`.
    pub fn async_poll(h: &mut impl Host) -> u32 {
        let app = cdecl(h, GET_APPLICATION, &[]);
        h.call_sym(POLL_INPUT_ASYNC, Conv::Thiscall(app), &[])
    }

    /// `inputIsReleaseButton(b)` / `inputIsRepButton(b)` (cdecl): the
    /// input manager's answer (`method`), zero-extended.
    pub fn input_button(h: &mut impl Host, method: u32, b: u32) -> u32 {
        let m = cdecl(h, INPUT_MANAGER, &[]);
        h.call_sym(method, Conv::Thiscall(m), &[b]) & 0xff
    }

    /// `Virtuos_InputManager_setVibration(this; pad, small, large)`
    /// (thiscall, `ret 0xc`): only for a pad whose device is set (the
    /// application's `+0x2b8` table, word `pad * 4 + 0x38`): the device's
    /// `setVibration(large, small)` while vibration is on (`+0xa8`), else
    /// `setVibration(0, 0)`. Returns that call's `eax`, or the table
    /// address when the pad has no device.
    pub fn set_vibration(h: &mut impl Host, this: u32, pad: u32, small: u32, large: u32) -> u32 {
        let app = cdecl(h, GET_APPLICATION, &[]);
        let table: u32 = h.read(app.wrapping_add(0x2b8));
        let slot = pad.wrapping_mul(4).wrapping_add(0x38);
        if h.read::<u32>(slot.wrapping_add(table)) == 0 {
            return table;
        }
        let on = h.read::<u32>(this.wrapping_add(0xa8)) != 0;
        let app = cdecl(h, GET_APPLICATION, &[]);
        let table: u32 = h.read(app.wrapping_add(0x2b8));
        let dev: u32 = h.read(table.wrapping_add(slot));
        let args = if on { [large, small] } else { [0, 0] };
        h.call_sym(PAD_SET_VIBRATION, Conv::Thiscall(dev), &args)
    }

    /// `0x00631060(this)` (`ecx`; unwinding): `user_free_thunk(+8)` when
    /// set. Returns its `eax`, or 0.
    pub fn free_member8(h: &mut impl Host, this: u32) -> u32 {
        let p: u32 = h.read(this.wrapping_add(8));
        if p == 0 {
            return 0;
        }
        cdecl(h, USER_FREE_THUNK, &[p])
    }

    /// `0x00634200(this)` (`ecx`; unwinding): `Phyre_PhyreFree(+0xc)`
    /// unless the word's bit 0 is set (a tagged inline value).
    pub fn phyre_free_member_c(h: &mut impl Host, this: u32) {
        let p: u32 = h.read(this.wrapping_add(0xc));
        if p & 1 == 0 {
            cdecl(h, PHYRE_FREE, &[p]);
        }
    }

    /// `0x00634340(this)` (`ecx`; unwinding): `operator delete(+0)`.
    pub fn delete_member0(h: &mut impl Host, this: u32) {
        let p: u32 = h.read(this);
        cdecl(h, OPERATOR_DELETE, &[p]);
    }

    /// `0x00634360(this)`: a jump thunk to `ListVector_dtor_00634220`;
    /// its `eax`.
    pub fn list_vector_dtor_thunk(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(LIST_VECTOR_DTOR, Conv::Thiscall(this), &[])
    }

    /// `0x00634370(this)` (`ecx`; unwinding): a `std::list`'s destructor:
    /// the head (`+0`) linked to itself, the size (`+4`) 0, every node
    /// deleted (the head reread at each step), then the head.
    pub fn list_dtor(h: &mut impl Host, this: u32) {
        let head: u32 = h.read(this);
        let mut node: u32 = h.read(head);
        h.write(head, head);
        let head: u32 = h.read(this);
        h.write(head.wrapping_add(4), head);
        h.write(this.wrapping_add(4), 0u32);
        while node != h.read::<u32>(this) {
            let next: u32 = h.read(node);
            cdecl(h, OPERATOR_DELETE, &[node]);
            node = next;
        }
        let head: u32 = h.read(this);
        cdecl(h, OPERATOR_DELETE, &[head]);
    }

    /// The frame of `0x006343b0` / `0x006343e0`: the erase's result
    /// iterator at `ebp - 4` (first holding `this`, the pushed `ecx`).
    pub const TREE_DTOR_FRAME: u32 = 4;

    /// `0x006343b0` / `0x006343e0(this)` (`ecx`; unwinding): a `std` tree's
    /// destructor: `erase(this; &it, head->left, head)` (`erase` the
    /// tree's own, thiscall `ret 0xc`, its iterator out at `ebp - 4`),
    /// then `operator delete(head)`.
    pub fn tree_dtor(h: &mut impl Host, ebp: u32, erase: u32, this: u32) {
        let it = ebp.wrapping_sub(TREE_DTOR_FRAME);
        h.write(it, this);
        let head: u32 = h.read(this);
        let left: u32 = h.read(head);
        h.call_sym(erase, Conv::Thiscall(this), &[it, left, head]);
        let head: u32 = h.read(this);
        cdecl(h, OPERATOR_DELETE, &[head]);
    }

    /// `0x00634500(this)` (`ecx`; unwinding): a vector's destructor: when
    /// `+0` is set, `0x00635c40(first, last)` (stdcall), `operator
    /// delete(first)` and the three words cleared. Returns `delete`'s
    /// `eax`, or 0.
    pub fn vector_dtor(h: &mut impl Host, this: u32) -> u32 {
        let first: u32 = h.read(this);
        if first == 0 {
            return 0;
        }
        let last: u32 = h.read(this.wrapping_add(4));
        h.call_sym(DESTROY_RANGE, Conv::Stdcall, &[first, last]);
        let first: u32 = h.read(this);
        let r = cdecl(h, OPERATOR_DELETE, &[first]);
        for off in [0, 4, 8] {
            h.write(this.wrapping_add(off), 0u32);
        }
        r
    }

    /// `SaveDataGetLoc(id, out)` (cdecl): `LocalizationManager_getText(
    /// GetInstance(); id, out, 0)`'s `eax`.
    pub fn save_data_get_loc(h: &mut impl Host, id: u32, out: u32) -> u32 {
        let m = cdecl(h, LOC_MANAGER, &[]);
        h.call_sym(LOC_TEXT, Conv::Thiscall(m), &[id, out, 0])
    }

    /// `0x00630f40(this)` (thiscall; a static initialiser's constructor):
    /// `+0`, `+4`, `+0x9c..=+0xb4` zeroed, the floats `+0x8c`, `+0x94`,
    /// `+0x90` 0.0; the touch record created, the touch manager's
    /// `GuiTouch_reset`, the mouse state and the 16-entry array created;
    /// `+0x88` = `inputGetCurTime()`. Returns `this`.
    pub fn controller_ctor(h: &mut impl Host, this: u32) -> u32 {
        for off in [0u32, 4, 0x9c, 0xa0, 0xa4, 0xa8, 0xac, 0xb0, 0xb4] {
            h.write(this.wrapping_add(off), 0u32);
        }
        for off in [0x8c, 0x94, 0x90] {
            h.write(this.wrapping_add(off), 0f32);
        }
        cdecl(h, TOUCH_RECORD_CREATE, &[]);
        let touch = cdecl(h, TOUCH_MANAGER, &[]);
        h.call_sym(GUI_TOUCH_RESET, Conv::Thiscall(touch), &[]);
        cdecl(h, MOUSE_STATE_CREATE, &[]);
        cdecl(h, ARRAY16_CREATE, &[]);
        let t = h.call_sym_st0(CUR_TIME, Conv::Cdecl, &[]);
        h.write(this.wrapping_add(0x88), t);
        this
    }

    /// `0x00633f70(this)` (`ecx`; unwinding): `0x00636970(this)`, the
    /// free list at `+0x18` destroyed, `Phyre_PRedBlackTree_clear(this)`
    /// twice (the two base destructors).
    pub fn tree_set_dtor(h: &mut impl Host, this: u32) {
        h.call_sym(TEARDOWN_636970, Conv::Thiscall(this), &[]);
        h.call_sym(FREE_LIST_DTOR, Conv::Thiscall(this.wrapping_add(0x18)), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
        h.call_sym(TREE_CLEAR, Conv::Thiscall(this), &[]);
    }

    /// `0x00686bb0(next, prev)` (stdcall, `ret 8`): a `std::list` node of
    /// 0x24 bytes from `operator new` (`std::_Xbad_alloc` when it fails),
    /// `+0` = `next` and `+4` = `prev`, both the node itself when `next`
    /// is 0. Returns the node.
    pub fn buy_list_node(h: &mut impl Host, next: u32, prev: u32) -> u32 {
        let node = cdecl(h, OPERATOR_NEW, &[0x24]);
        if node == 0 {
            import(h, XBAD_ALLOC_IAT, Conv::Cdecl, &[]);
        }
        let (n, p) = if next == 0 {
            (node, node)
        } else {
            (next, prev)
        };
        h.write(node, n);
        if node.wrapping_add(4) != 0 {
            h.write(node.wrapping_add(4), p);
        }
        node
    }

    /// `vsnprintf1K_00687070(buffer, count, fmt, ...)` (cdecl):
    /// `_vsnprintf_s(buffer, 0x400, count, fmt, args)`'s `eax`; `args` is
    /// the address of the first variadic word (the caller's fourth slot).
    pub fn vsnprintf_1k(h: &mut impl Host, buffer: u32, count: u32, fmt: u32, args: u32) -> u32 {
        import(
            h,
            VSNPRINTF_S_IAT,
            Conv::Cdecl,
            &[buffer, 0x400, count, fmt, args],
        )
    }

    /// `iVoiceVolumeDirect(v)` (cdecl): `registCommandAsync3(fmod; 0xe,
    /// v, 1)`'s `eax`.
    pub fn voice_volume_direct(h: &mut impl Host, v: u32) -> u32 {
        let fmod: u32 = h.read(h.rebase(FMOD_MANAGER));
        h.call_sym(FMOD_COMMAND3, Conv::Thiscall(fmod), &[0xe, v, 1])
    }

    /// `iVoiceVolumeTranslate(v, t)` (cdecl): `registCommandAsync3(fmod;
    /// 0xd, v, t)`'s `eax`.
    pub fn voice_volume_translate(h: &mut impl Host, v: u32, t: u32) -> u32 {
        let fmod: u32 = h.read(h.rebase(FMOD_MANAGER));
        h.call_sym(FMOD_COMMAND3, Conv::Thiscall(fmod), &[0xd, v, t])
    }

    /// `0x0067a9b0(a, b, c, d)` (cdecl): `0x006fb380(voices; a, b, c,
    /// d)`'s `eax`.
    #[allow(clippy::many_single_char_names)] // the table's four key columns
    pub fn voice_select_forward(h: &mut impl Host, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let t: u32 = h.read(h.rebase(VOICE_TABLE));
        h.call_sym(VOICE_SELECT, Conv::Thiscall(t), &[a, b, c, d])
    }

    /// `0x0067a9d0(a, b)` (cdecl): `0x006fb410(voices; a, b)`'s `eax`.
    pub fn voice_find_forward(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let t: u32 = h.read(h.rebase(VOICE_TABLE));
        h.call_sym(VOICE_FIND, Conv::Thiscall(t), &[a, b])
    }

    /// Entry `i` of the column whose array `this + off` points at.
    fn col(h: &impl Host, this: u32, off: u32, i: u32) -> u32 {
        let base: u32 = h.read(this.wrapping_add(off));
        h.read(base.wrapping_add(i.wrapping_mul(4)))
    }

    /// Waits until the table is loaded (`+0x20`), `Sleep(5)` at a time.
    fn wait_loaded(h: &mut impl Host, this: u32) {
        while h.read::<u32>(this.wrapping_add(0x20)) == 0 {
            cdecl(h, SLEEP_MS, &[5]);
        }
    }

    /// `0x006fb380(this; id, b, c, d)` (thiscall, `ret 0x10`): nothing
    /// without a table (`+0x1c`); else, once loaded, the first of the
    /// `+0x10` (signed) entries whose columns `+0x28`, `+0x30`, `+0x34`,
    /// `+0x38` equal `id`'s low half, `b`, `c` and `d` selects its `+4`
    /// word into `+8`. Returns 0.
    #[allow(clippy::many_single_char_names)] // the table's four key columns
    pub fn voice_select(h: &mut impl Host, this: u32, id: u32, b: u32, c: u32, d: u32) -> u32 {
        if h.read::<u32>(this.wrapping_add(0x1c)) == 0 {
            return 0;
        }
        wait_loaded(h, this);
        let id = id & 0xffff;
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(this.wrapping_add(0x10)).cast_signed() {
            if col(h, this, 0x28, i) == id
                && col(h, this, 0x30, i) == b
                && col(h, this, 0x34, i) == c
                && col(h, this, 0x38, i) == d
            {
                let v = col(h, this, 4, i);
                h.write(this.wrapping_add(8), v);
                return 0;
            }
            i = i.wrapping_add(1);
        }
        0
    }

    /// `0x006fb410(this; a, b)` (thiscall, `ret 8`): 0 without a table
    /// (`+0x1c`); else, once loaded, the `+0x3c` word of the last of the
    /// `+0x14` (signed) entries whose columns `+0x2c` and `+0x24` equal
    /// `a` and `b`, or 0.
    pub fn voice_find(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        if h.read::<u32>(this.wrapping_add(0x1c)) == 0 {
            return 0;
        }
        wait_loaded(h, this);
        let mut found = 0;
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(this.wrapping_add(0x14)).cast_signed() {
            if col(h, this, 0x2c, i) == a && col(h, this, 0x24, i) == b {
                found = col(h, this, 0x3c, i);
            }
            i = i.wrapping_add(1);
        }
        found
    }
}
