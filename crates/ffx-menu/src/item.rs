//! The item menu's control, `TkMenuCtrlItem` (`0x008c5450`, its menu
//! slot's ctrl): a state machine on the slot's `+0x1c` (0..0x38, jump
//! table `0x008c5dc8`) over the item list window (`g_TkMenuItemWindow`),
//! the important-items window (`+4`), the target window (`+8`), the
//! name entry window (`+0xc`), the gear window and the sub-command
//! windows (`TkMenuOpenSubCmdWindowBeside`).
//!
//! Some states pick the next one at once (the original loops on its
//! dispatch); the others store it and return. `eax` is the original's on
//! every path: the state, a window pointer (with its `+0x45` byte in `al`
//! where the original loads it into `al`), or the last callee's result.

/// The callees, by address.
pub mod callee {
    pub const GET_CONTROL_PAD: u32 = 0x008b_e410;
    pub const GET_CONTROL_PAD_REP: u32 = 0x008b_e470;
    pub const GET_CONTROL_PAD_TRG: u32 = 0x008b_e4b0;
    pub const GET_ROM_ITEM: u32 = 0x0079_09f0;
    pub const ITEM_TARGET_MASK: u32 = 0x008c_8a30;
    pub const FREE_WINDOW_GROUP: u32 = 0x008a_a410;
    pub const OPEN_TARGET_WINDOW: u32 = 0x008c_9240;
    pub const GET_PLAYER_FROM_INDEX2: u32 = 0x008a_9ad0;
    pub const GET_SUMMON_FROM_INDEX: u32 = 0x008a_9bc0;
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    pub const APPLY_HEAL: u32 = 0x008c_8c40;
    pub const USE_ON_TARGETS: u32 = 0x008c_8d50;
    pub const ITEM_REFRESH: u32 = 0x008c_8e60;
    pub const CLEAR_WINDOW: u32 = 0x008c_93a0;
    pub const SAVE_ITEM_TYPE_BUFFER: u32 = 0x0079_0530;
    pub const SAVE_ITEM_NUM_BUFFER: u32 = 0x0079_0510;
    pub const SORT_INVENTORY: u32 = 0x008c_8fe0;
    pub const PACK_GEAR: u32 = 0x008c_a120;
    pub const COUNT_GEAR_BY_OWNER: u32 = 0x008c_94e0;
    pub const SORT_GEAR_BY_OWNER: u32 = 0x008c_9fb0;
    pub const SORT_GEAR_BY_KIND: u32 = 0x008c_a1b0;
    pub const SORT_ALL_GEAR: u32 = 0x008c_9bf0;
    pub const SWAP_SAVE_GEAR: u32 = 0x007a_b9f0;
    pub const REFRESH_WEAPON_SHOP_SELL2: u32 = 0x008d_9560;
    pub const MAIN_ACTIVE_MENU: u32 = 0x008a_a100;
    pub const MAIN_GET_MENU_ACTIVE_MASK: u32 = 0x008a_a460;
    pub const CLOSE_WINDOW2: u32 = 0x008c_6b80;
    pub const SAVE_ITEM_USE: u32 = 0x0079_0550;
    pub const CLEAR_WINDOW2: u32 = 0x008c_93c0;
    pub const OPEN_NAME_ENTRY_WINDOW: u32 = 0x008c_4250;
    pub const ITEM_MES: u32 = 0x008d_df40;
    pub const ITEM_HELP: u32 = 0x008d_df20;
    pub const OPEN_SUB_CMD_WINDOW_BESIDE: u32 = 0x008e_46b0;
    pub const SUB_CMD_SET_VALUE: u32 = 0x008e_4a10;
    pub const SUB_CMD_SET_VALUE2: u32 = 0x008e_4a30;
    pub const RESTART_SUB_CMD_WINDOW1: u32 = 0x008e_49f0;
    pub const RESTART_SUB_CMD_WINDOW2: u32 = 0x008e_49d0;
    pub const OPEN_LIST_WINDOW: u32 = 0x008c_9140;
    pub const CLOSE_WINDOW3: u32 = 0x008c_6b40;
    pub const CLOSE_WINDOW4: u32 = 0x008c_6ba0;
    pub const OPEN_GEAR_WINDOW: u32 = 0x008c_9330;
    pub const CLOSE_WINDOW: u32 = 0x008c_6b60;
    pub const CLOSE_SUB_CMD_WINDOW1: u32 = 0x008e_3710;
    pub const MAIN_SLEEP_MENU: u32 = 0x008a_ad70;
    /// `MenuItem_UsableByte2`, stored as the list window's row filter.
    pub const USABLE_BYTE2: u32 = 0x008c_6a40;
}

/// The globals, by address.
pub mod global {
    /// `g_TkMenuItemList`: 0x200 x `{u16 id, u8 count, u8 usable}`.
    pub const LIST: u32 = 0x0159_7770;
    /// `g_TkMenuItemGearWindow`.
    pub const GEAR_WINDOW: u32 = 0x0186_a37c;
    /// `u8`: how the picked item is used on its target (0 heal, 1 use,
    /// 2 both, 3 name entry).
    pub const USE_KIND: u32 = 0x0186_a392;
    /// `u8 g_TkMenuItemGearMode`.
    pub const GEAR_MODE: u32 = 0x0186_a393;
    /// `g_TkMenuItemWindow`.
    pub const WINDOW: u32 = 0x0186_a394;
    /// The important-items list window (`MenuItem_OpenListWindow(1)`).
    pub const WINDOW2: u32 = 0x0186_a398;
    /// The target window (`+0x58` 0: party rows, else aeons).
    pub const TARGET_WINDOW: u32 = 0x0186_a39c;
    /// The aeon name entry window.
    pub const NAME_WINDOW: u32 = 0x0186_a3a0;
    /// `u16 g_TkMenuAbilityCommand` (the item used, here).
    pub const COMMAND: u32 = 0x0186_a3a8;
    /// 0, 1 (the item has no target: `+0x22` bit 3) or 2 (the target
    /// window is open).
    pub const TARGET_MODE: u32 = 0x0186_a40c;
    /// `g_TkMenuSubCmdWindow2`.
    pub const SUB_CMD_WINDOW2: u32 = 0x023c_c124;
    /// `g_TkMenuSubCmdWindow`.
    pub const SUB_CMD_WINDOW: u32 = 0x023c_c128;
}

/// Words of the original's frame the port needs (`ebp-0x24..ebp-4`).
pub const LOCAL_WORDS: usize = 9;

/// [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{callee as c, global as g};

    /// The original's frame (`local` is its `ebp-0x24`): two message
    /// pairs at `-0x24`/`-0x1c`, `MsGetRomItem`'s out words at `-0x20`
    /// and `-0x18`, the save buffers' count at `-8`.
    const HELPS: u32 = 0;
    const ROM_OUT_B: u32 = 4;
    const TEXTS: u32 = 8;
    const ROM_OUT_A: u32 = 0xc;
    const COUNT: u32 = 0x1c;

    fn go<M, C>(mem: &mut M, call: &mut C, addr: u32, args: &[u32]) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let a = mem.rebase(addr);
        call(mem, a, args)
    }

    fn gl<M: Mem>(mem: &M, at: u32) -> u32 {
        mem.read(mem.rebase(at))
    }

    fn put<M: Mem>(mem: &mut M, at: u32, v: u32) {
        let a = mem.rebase(at);
        mem.write(a, v);
    }

    /// `al` replaced by `b` (the original's `mov al, [..]`).
    fn with_al(eax: u32, b: i8) -> u32 {
        (eax & 0xffff_ff00) | u32::from(b.cast_unsigned())
    }

    /// The list entry under window `w`'s cursor: `+0x58 + 2 * i16 +0x48`.
    fn cursor_entry<M: Mem>(mem: &M, w: u32) -> u32 {
        let row = i32::from(mem.read::<i16>(w.wrapping_add(0x48)));
        mem.read::<u32>(w.wrapping_add(0x58))
            .wrapping_add(row.cast_unsigned().wrapping_mul(2))
    }

    /// The item id of list entry `i`.
    fn item_id<M: Mem>(mem: &M, i: u32) -> u32 {
        u32::from(mem.read::<u16>(mem.rebase(g::LIST).wrapping_add(i.wrapping_mul(4))))
    }

    /// A word of `i16` at `p` (sign-extended).
    fn i16_at<M: Mem>(mem: &M, p: u32) -> u32 {
        i32::from(mem.read::<i16>(p)).cast_unsigned()
    }

    /// States 21 and 30: the pair of messages `a`, `b` in a sub command
    /// window beside the list (texts and helps in the frame, in the order
    /// `b`, `a`), values -409 and 409; `MenuSubCmd_SetValue2`'s `eax`.
    fn open_pair<M, C>(mem: &mut M, call: &mut C, local: u32, a: u32, b: u32) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let v = go(mem, call, c::ITEM_MES, &[a]);
        mem.write(local + TEXTS + 4, v);
        let v = go(mem, call, c::ITEM_MES, &[b]);
        mem.write(local + TEXTS, v);
        let v = go(mem, call, c::ITEM_HELP, &[a]);
        mem.write(local + HELPS + 4, v);
        let v = go(mem, call, c::ITEM_HELP, &[b]);
        mem.write(local + HELPS, v);
        go(
            mem,
            call,
            c::OPEN_SUB_CMD_WINDOW_BESIDE,
            &[0, local + TEXTS, local + HELPS, 2, 2],
        );
        go(mem, call, c::SUB_CMD_SET_VALUE, &[0xffff_fe67]);
        go(mem, call, c::SUB_CMD_SET_VALUE2, &[0x199])
    }

    /// `TkMenuCtrlItem(m)` (`0x008c5450`): one step of the item menu (see
    /// the module). `local`: the original's frame, [`super::LOCAL_WORDS`]
    /// words from its `ebp-0x24` (`MsGetRomItem`'s out words, the sub
    /// command texts, the save buffers' count). Returns the original's
    /// `eax`.
    ///
    /// The original keeps `g_TkMenuItemWindow` in `edx` across
    /// `TkMenuItemClearWindow` (which leaves `edx` alone): states 2 and
    /// 26 set up the window as it was before that call, which is the one
    /// the call cleared.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one state machine
    pub fn tk_menu_ctrl_item<M, C>(mem: &mut M, m: u32, local: u32, call: &mut C) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        go(mem, call, c::GET_CONTROL_PAD, &[]);
        go(mem, call, c::GET_CONTROL_PAD_REP, &[]);
        go(mem, call, c::GET_CONTROL_PAD_TRG, &[]);
        let state_at = m.wrapping_add(0x1c);
        let set = |mem: &mut M, v: u32| mem.write(state_at, v);
        let mut state = mem.read::<u32>(state_at);
        if state > 0x38 {
            return state;
        }
        // `eax` on entry to a state: the dispatched state, or the last
        // result of the state that falls into it
        let mut eax = state;
        loop {
            // the original's `edx`
            let w = gl(mem, g::WINDOW);
            let next = match state {
                0 => return stay(mem, set, 1, state),
                1 => return stay(mem, set, 2, state),
                2 => {
                    put(mem, g::TARGET_MODE, 0);
                    go(mem, call, c::CLEAR_WINDOW, &[]);
                    mem.write(w.wrapping_add(0x42), 1u8);
                    let filter = mem.rebase(c::USABLE_BYTE2);
                    mem.write(w.wrapping_add(0x1c), filter);
                    let r = go(mem, call, c::ITEM_REFRESH, &[0]);
                    set(mem, 3);
                    return r;
                }
                3 => return stay(mem, set, 4, state),
                4 => return stay(mem, set, 6, state),
                5 => {
                    put(mem, g::TARGET_MODE, 0);
                    let r = go(mem, call, c::CLEAR_WINDOW, &[]);
                    set(mem, 6);
                    return r;
                }
                6 => {
                    let mut b = mem.read::<i8>(w.wrapping_add(0x45));
                    put(mem, g::TARGET_MODE, 0);
                    if b < 0 {
                        set(mem, 0x13);
                        b = mem.read::<i8>(w.wrapping_add(0x45));
                    }
                    if b <= 0 {
                        return state;
                    }
                    set(mem, 0xb);
                    0xb
                }
                7 | 8 => {
                    if state == 7 {
                        go(mem, call, c::ITEM_REFRESH, &[0]);
                        go(mem, call, c::MAIN_ACTIVE_MENU, &[0x17, 0]);
                        set(mem, 8);
                    }
                    let mask = go(mem, call, c::MAIN_GET_MENU_ACTIVE_MASK, &[]);
                    if mask & 0x0080_0000 == 0 {
                        set(mem, 9);
                    }
                    return mask;
                }
                9 => {
                    set(mem, 0xa);
                    return stay(mem, set, 5, state);
                }
                10 | 16 => return stay(mem, set, 5, state),
                11 | 12 => {
                    let i = cursor_entry(mem, w);
                    let out = local + if state == 11 { ROM_OUT_A } else { ROM_OUT_B };
                    let id = item_id(mem, i);
                    let rom = go(mem, call, c::GET_ROM_ITEM, &[id, out]);
                    if mem.read::<u8>(rom.wrapping_add(0x22)) & 8 != 0 {
                        put(mem, g::TARGET_MODE, 1);
                        7
                    } else if state == 11 {
                        let id = item_id(mem, i);
                        let mask = go(mem, call, c::ITEM_TARGET_MASK, &[id]);
                        let w = gl(mem, g::WINDOW);
                        let kind = if mem.read::<u32>(w.wrapping_add(0x58)) == 0 {
                            2
                        } else {
                            1
                        };
                        put(mem, g::TARGET_MODE, 2);
                        go(mem, call, c::FREE_WINDOW_GROUP, &[0]);
                        let use_kind = u32::from(mem.read::<u8>(mem.rebase(g::USE_KIND)));
                        let r = go(mem, call, c::OPEN_TARGET_WINDOW, &[mask, use_kind, kind]);
                        set(mem, 0xd);
                        return r;
                    } else {
                        let id = item_id(mem, i);
                        let r = go(mem, call, c::ITEM_TARGET_MASK, &[id]);
                        set(mem, 0xd);
                        return r;
                    }
                }
                13 => {
                    let t = gl(mem, g::TARGET_WINDOW);
                    let b = mem.read::<i8>(t.wrapping_add(0x45));
                    if b < 0 {
                        put(mem, g::TARGET_MODE, 0);
                        let r = go(mem, call, c::CLOSE_WINDOW2, &[]);
                        set(mem, 5);
                        return r;
                    }
                    if b > 0 {
                        set(mem, 0xe);
                    }
                    return with_al(t, b);
                }
                14 => {
                    let t = gl(mem, g::TARGET_WINDOW);
                    let row = i16_at(mem, t.wrapping_add(0x48));
                    let who = if mem.read::<u32>(t.wrapping_add(0x58)) == 0 {
                        go(mem, call, c::GET_PLAYER_FROM_INDEX2, &[row])
                    } else {
                        go(mem, call, c::GET_SUMMON_FROM_INDEX, &[row])
                    };
                    match mem.read::<u8>(mem.rebase(g::USE_KIND)) {
                        0 => {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0062]);
                            go(mem, call, c::APPLY_HEAL, &[who]);
                            0xf
                        }
                        1 => {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0062]);
                            go(mem, call, c::USE_ON_TARGETS, &[who]);
                            0xf
                        }
                        2 => {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0062]);
                            go(mem, call, c::APPLY_HEAL, &[who]);
                            go(mem, call, c::USE_ON_TARGETS, &[who]);
                            0xf
                        }
                        3 => {
                            go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                            set(mem, 0x11);
                            0x11
                        }
                        _ => 0xf,
                    }
                }
                15 => {
                    let id = u32::from(mem.read::<u16>(mem.rebase(g::COMMAND)));
                    go(mem, call, c::SAVE_ITEM_USE, &[id, u32::MAX]);
                    go(mem, call, c::ITEM_REFRESH, &[0]);
                    let w = gl(mem, g::WINDOW);
                    let i = cursor_entry(mem, w);
                    let at = mem.rebase(g::LIST + 2).wrapping_add(i.wrapping_mul(4));
                    if mem.read::<u8>(at) == 0 {
                        let r = go(mem, call, c::CLOSE_WINDOW2, &[]);
                        set(mem, 5);
                        return r;
                    }
                    let r = go(mem, call, c::CLEAR_WINDOW2, &[]);
                    set(mem, 0xc);
                    return r;
                }
                17 => {
                    let t = gl(mem, g::TARGET_WINDOW);
                    let row = i16_at(mem, t.wrapping_add(0x48));
                    let aeon = go(mem, call, c::GET_SUMMON_FROM_INDEX, &[row]);
                    let r = go(mem, call, c::OPEN_NAME_ENTRY_WINDOW, &[0, aeon]);
                    set(mem, 0x12);
                    put(mem, g::NAME_WINDOW, r);
                    return r;
                }
                18 => {
                    let e = gl(mem, g::NAME_WINDOW);
                    if mem.read::<u16>(e.wrapping_add(0x30)) != 0 {
                        mem.write(e.wrapping_add(0x41), 1u8);
                        set(mem, 0xf);
                    }
                    return e;
                }
                19 => return stay(mem, set, 0x14, state),
                20 => {
                    go(mem, call, c::ITEM_REFRESH, &[4]);
                    set(mem, 0x34);
                    0x34
                }
                21 => {
                    let r = open_pair(mem, call, local, 3, 4);
                    set(mem, 0x16);
                    return r;
                }
                22 => return stay(mem, set, 0x17, state),
                23 | 24 => {
                    if state == 24 {
                        go(mem, call, c::RESTART_SUB_CMD_WINDOW1, &[]);
                    }
                    let r = go(mem, call, c::ITEM_REFRESH, &[4]);
                    set(mem, 0x19);
                    return r;
                }
                25 | 34 => {
                    let s = gl(mem, g::SUB_CMD_WINDOW2);
                    let b = mem.read::<i8>(s.wrapping_add(0x45));
                    if b < 0 {
                        set(mem, if state == 25 { 0x32 } else { 0x23 });
                        return s;
                    }
                    if b == 0 {
                        return s;
                    }
                    match (i16_at(mem, s.wrapping_add(0x48)), state) {
                        (0, 25) => 0x1a,
                        (1, 25) => 0x1c,
                        (0, _) => 0x26,
                        (1, _) => 0x25,
                        (v, _) => return v.wrapping_sub(1),
                    }
                }
                26 | 27 => {
                    let mut w = w;
                    if state == 26 {
                        go(mem, call, c::CLEAR_WINDOW, &[]);
                        mem.write(w.wrapping_add(0x42), 2u8);
                        mem.write(w.wrapping_add(0x1c), 0u32);
                        eax = go(mem, call, c::ITEM_REFRESH, &[4]);
                        w = gl(mem, g::WINDOW);
                        set(mem, 0x1b);
                    }
                    let b = mem.read::<i8>(w.wrapping_add(0x45));
                    if b < 0 {
                        set(mem, 0x18);
                        return with_al(eax, b);
                    }
                    if i32::from(b) < i32::from(mem.read::<u8>(w.wrapping_add(0x42))) {
                        return i32::from(b).cast_unsigned();
                    }
                    swap_rows(mem, call, local + COUNT);
                    0x1a
                }
                28 => {
                    go(mem, call, c::SORT_INVENTORY, &[]);
                    set(mem, 0x1d);
                    set(mem, 0x18);
                    0x18
                }
                29 => {
                    set(mem, 0x18);
                    0x18
                }
                30 => {
                    let r = open_pair(mem, call, local, 0xc, 0xd);
                    set(mem, 0x1f);
                    return r;
                }
                31 => return stay(mem, set, 0x20, state),
                32 | 33 => {
                    if state == 33 {
                        go(mem, call, c::RESTART_SUB_CMD_WINDOW1, &[]);
                    }
                    let r = go(mem, call, c::REFRESH_WEAPON_SHOP_SELL2, &[]);
                    set(mem, 0x22);
                    return r;
                }
                35 => {
                    go(mem, call, c::SUB_CMD_SET_VALUE2, &[0xffff_fe67]);
                    go(mem, call, c::SUB_CMD_SET_VALUE, &[0x199]);
                    go(mem, call, c::CLOSE_SUB_CMD_WINDOW1, &[]);
                    let r = go(mem, call, c::REFRESH_WEAPON_SHOP_SELL2, &[]);
                    set(mem, 0x24);
                    set(mem, 0x34);
                    return r;
                }
                36 | 49 | 51 => return stay(mem, set, 0x34, state),
                37 => {
                    go(mem, call, c::PACK_GEAR, &[]);
                    go(mem, call, c::COUNT_GEAR_BY_OWNER, &[]);
                    if go(mem, call, c::SORT_GEAR_BY_OWNER, &[]) != 0 {
                        go(mem, call, c::SORT_GEAR_BY_KIND, &[]);
                        go(mem, call, c::SORT_ALL_GEAR, &[]);
                    }
                    0x21
                }
                38 | 39 => {
                    if state == 38 {
                        let gw = gl(mem, g::GEAR_WINDOW);
                        let at = mem.rebase(g::GEAR_MODE);
                        mem.write(at, 1u8);
                        mem.write(gw.wrapping_add(0x45), 0u8);
                        mem.write(gw.wrapping_add(0x28), 0u32);
                        mem.write(gw.wrapping_add(0x37), 0u8);
                        set(mem, 0x27);
                    }
                    let gw = gl(mem, g::GEAR_WINDOW);
                    if mem.read::<i8>(gw.wrapping_add(0x45)) < 0 {
                        set(mem, 0x28);
                    }
                    if mem.read::<i8>(gw.wrapping_add(0x45)) < 2 {
                        return gw;
                    }
                    0x2a
                }
                40 | 41 => {
                    if state == 40 {
                        let gw = gl(mem, g::GEAR_WINDOW);
                        mem.write(gw.wrapping_add(0x45), 0xffu8);
                        mem.write(gw.wrapping_add(0x28), 0u32);
                        set(mem, 0x29);
                    }
                    set(mem, 0x21);
                    let at = mem.rebase(g::GEAR_MODE);
                    mem.write(at, 0u8);
                    return state;
                }
                42 => {
                    let gw = gl(mem, g::GEAR_WINDOW);
                    let a = item_id(mem, i16_at(mem, gw.wrapping_add(0x48)));
                    let b = item_id(mem, i16_at(mem, gw.wrapping_add(0x2c)));
                    if a == b {
                        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0003]);
                    } else {
                        go(mem, call, c::SND_SEP_PLAY_SIMPLE, &[0x8000_0001]);
                        go(mem, call, c::SWAP_SAVE_GEAR, &[a, b]);
                    }
                    go(mem, call, c::REFRESH_WEAPON_SHOP_SELL2, &[]);
                    set(mem, 0x2b);
                    0x26
                }
                43 => 0x26,
                44 => {
                    let w2 = gl(mem, g::WINDOW2);
                    mem.write(w2.wrapping_add(0x45), 0u8);
                    set(mem, 0x2d);
                    return w2;
                }
                45 => return stay(mem, set, 0x2e, state),
                46 => return stay(mem, set, 0x2f, state),
                47 => {
                    let w2 = gl(mem, g::WINDOW2);
                    if mem.read::<u8>(w2.wrapping_add(0x45)) != 0 {
                        set(mem, 0x30);
                    }
                    return w2;
                }
                48 => {
                    let w2 = gl(mem, g::WINDOW2);
                    mem.write(w2.wrapping_add(0x28), 0u32);
                    mem.write(w2.wrapping_add(0x45), 0xffu8);
                    set(mem, 0x31);
                    return w2;
                }
                50 => {
                    go(mem, call, c::SUB_CMD_SET_VALUE2, &[0xffff_fe67]);
                    go(mem, call, c::SUB_CMD_SET_VALUE, &[0x199]);
                    go(mem, call, c::CLOSE_SUB_CMD_WINDOW1, &[]);
                    let r = go(mem, call, c::ITEM_REFRESH, &[4]);
                    set(mem, 0x33);
                    set(mem, 0x34);
                    return r;
                }
                52 => {
                    go(mem, call, c::RESTART_SUB_CMD_WINDOW2, &[]);
                    let r = go(mem, call, c::SUB_CMD_SET_VALUE, &[0x199]);
                    set(mem, 0x35);
                    return r;
                }
                53 => return sub_cmd_pick(mem, call, w, set),
                _ => {
                    // 54..56: back to the main menu
                    if state == 54 {
                        set(mem, 0x37);
                    }
                    if state <= 55 {
                        set(mem, 0x38);
                    }
                    go(mem, call, c::MAIN_SLEEP_MENU, &[3]);
                    return go(mem, call, c::MAIN_ACTIVE_MENU, &[1, 0]);
                }
            };
            state = next;
            eax = next;
        }
    }

    /// A state that only stores the next one: `eax` stays the state.
    fn stay<M: Mem>(mem: &mut M, set: impl Fn(&mut M, u32), next: u32, state: u32) -> u32 {
        set(mem, next);
        state
    }

    /// State 27's swap: the item list rows at the window's two cursors
    /// (`+0x58 + 2 * i16 +0x2c` and `+0x5c + 2 * i16 +0x2e`) traded in the
    /// save's type (`u16`) and count (`u8`) buffers, the list refreshed and
    /// `+0x58 = +0x5c`. `count`: the buffers' out word.
    fn swap_rows<M, C>(mem: &mut M, call: &mut C, count: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let types = go(mem, call, c::SAVE_ITEM_TYPE_BUFFER, &[count]);
        let nums = go(mem, call, c::SAVE_ITEM_NUM_BUFFER, &[count]);
        let w = gl(mem, g::WINDOW);
        let row = |mem: &M, base: u32, cur: u32| {
            mem.read::<u32>(w.wrapping_add(base))
                .wrapping_add(i16_at(mem, w.wrapping_add(cur)).wrapping_mul(2))
        };
        let a = row(mem, 0x58, 0x2c);
        let b = row(mem, 0x5c, 0x2e);
        let num_a = mem.read::<u8>(a.wrapping_add(nums));
        let type_a = mem.read::<u16>(types.wrapping_add(a.wrapping_mul(2)));
        let num_b = mem.read::<u8>(b.wrapping_add(nums));
        let type_b = mem.read::<u16>(types.wrapping_add(b.wrapping_mul(2)));
        mem.write(types.wrapping_add(a.wrapping_mul(2)), type_b);
        mem.write(a.wrapping_add(nums), num_b);
        mem.write(types.wrapping_add(b.wrapping_mul(2)), type_a);
        mem.write(b.wrapping_add(nums), num_a);
        go(mem, call, c::ITEM_REFRESH, &[4]);
        let w = gl(mem, g::WINDOW);
        let v = mem.read::<u32>(w.wrapping_add(0x5c));
        mem.write(w.wrapping_add(0x58), v);
    }

    /// State 53: the sub-command window's row shows its list (0/1 the
    /// item list, opened if missing; 2 the gear list; 3 the important
    /// items) and closes the others; then a cancel stores 0x36 and a pick
    /// of row 0..3 stores 2, 0x15, 0x1e or 0x2c. `w`: the item window
    /// (the original's `edx`). `eax`: the window (no pick) or the row.
    fn sub_cmd_pick<M, C>(mem: &mut M, call: &mut C, w: u32, set: impl Fn(&mut M, u32)) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let s = gl(mem, g::SUB_CMD_WINDOW);
        match i16_at(mem, s.wrapping_add(0x48)) {
            0 | 1 => {
                if w == 0 {
                    go(mem, call, c::ITEM_REFRESH, &[4]);
                    let nw = go(mem, call, c::OPEN_LIST_WINDOW, &[0]);
                    put(mem, g::WINDOW, nw);
                    mem.write(nw.wrapping_add(0x45), 0xffu8);
                }
                go(mem, call, c::CLOSE_WINDOW3, &[]);
                go(mem, call, c::CLOSE_WINDOW4, &[]);
            }
            2 => {
                if gl(mem, g::GEAR_WINDOW) == 0 {
                    go(mem, call, c::OPEN_GEAR_WINDOW, &[]);
                }
                go(mem, call, c::CLOSE_WINDOW, &[]);
                go(mem, call, c::CLOSE_WINDOW3, &[]);
            }
            3 => {
                if gl(mem, g::WINDOW2) == 0 {
                    let nw = go(mem, call, c::OPEN_LIST_WINDOW, &[1]);
                    put(mem, g::WINDOW2, nw);
                    mem.write(nw.wrapping_add(0x45), 0xffu8);
                }
                go(mem, call, c::CLOSE_WINDOW4, &[]);
                go(mem, call, c::CLOSE_WINDOW, &[]);
            }
            _ => {}
        }
        let s = gl(mem, g::SUB_CMD_WINDOW);
        if mem.read::<i8>(s.wrapping_add(0x45)) < 0 {
            set(mem, 0x36);
        }
        let s = gl(mem, g::SUB_CMD_WINDOW);
        if mem.read::<i8>(s.wrapping_add(0x45)) <= 0 {
            return s;
        }
        let v = i16_at(mem, s.wrapping_add(0x48));
        match v {
            0 => set(mem, 2),
            1 => set(mem, 0x15),
            2 => set(mem, 0x1e),
            3 => set(mem, 0x2c),
            _ => {}
        }
        v
    }
}
