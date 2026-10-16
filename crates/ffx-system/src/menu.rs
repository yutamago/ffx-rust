//! Small `TO` menu and text state: help modes, the ability map work area,
//! the item list of the ability map, pause, font width tables, the Al Bhed
//! table, key item names and help.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// `TOMenuGetFirstReadEndFlag`'s flag.
    pub const FIRST_READ_END: Global<i8> = Global::new(0x0134_0859);
    /// The help mode `TOMenuHelpSetExtMode` sets.
    pub const HELP_EXT_MODE: Global<u32> = Global::new(0x0186_a228);
    /// The second value `TOMenuHelpSetExtMode` sets.
    pub const HELP_EXT_ARG: Global<u32> = Global::new(0x00c5_a088);
    /// The ability map's work area.
    pub const ABILITYMAP_WORK: Global<u8> = Global::new(0x0164_0870);
    /// Entries in the ability map's item list.
    pub const AMAP_ITEM_LIST_MAX: Global<u32> = Global::new(0x0186_a384);
    /// The ability map's item list: 4-byte entries, type (`u16`) first.
    pub const AMAP_ITEMS: Global<u32> = Global::new(0x0159_7770);
    /// `g_PauseEntered`.
    pub const PAUSE_ENTERED: Global<u32> = Global::new(0x0134_084c);
    /// `SetInvokedFromSelectCursor`'s value.
    pub const INVOKED_FROM_SELECT_CURSOR: Global<u32> = Global::new(0x0187_16f0);
    /// `TOGetMessageWindowMode`'s value.
    pub const MESSAGE_WINDOW_MODE: Global<u32> = Global::new(0x0186_59bc);
    /// The FFX-to-Al Bhed character table.
    pub const ALBHED_TABLE: Global<u8> = Global::new(0x0187_63e8);
    /// The font width table pointers, one per 16-byte font record.
    pub const FONT_WIDTH_TABLES: Global<u32> = Global::new(0x0184_1da4);

    /// `TOMenuGetFirstReadEndFlag()` (`0x008a8e50`).
    pub fn first_read_end(mem: &impl Mem) -> i32 {
        i32::from(FIRST_READ_END.get(mem))
    }

    /// `TOMenuHelpSetExtMode(mode, arg)` (`0x008bef10`).
    pub fn help_set_ext_mode(mem: &mut impl Mem, mode: u32, arg: u32) {
        HELP_EXT_MODE.set(mem, mode);
        HELP_EXT_ARG.set(mem, arg);
    }

    /// `TOMenuGetAbilitymapWorkAdrs()` (`0x008bea30`).
    pub fn abilitymap_work(mem: &impl Mem) -> u32 {
        ABILITYMAP_WORK.ptr(mem).addr
    }

    /// `TOAmapGetItemListMax()` (`0x008c8c30`).
    pub fn amap_item_list_max(mem: &impl Mem) -> u32 {
        AMAP_ITEM_LIST_MAX.get(mem)
    }

    /// `TOAmapGetItemType(i)` (`0x008c6710`).
    pub fn amap_item_type(mem: &impl Mem, i: i32) -> u16 {
        mem.read(
            AMAP_ITEMS
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `IsInTkMenuPause()` (`0x008a8e10`).
    pub fn is_in_pause(mem: &impl Mem) -> u32 {
        PAUSE_ENTERED.get(mem)
    }

    /// `SetInvokedFromSelectCursor(v)` (`0x008e89c0`).
    pub fn set_invoked_from_select_cursor(mem: &mut impl Mem, v: u32) {
        INVOKED_FROM_SELECT_CURSOR.set(mem, v);
    }

    /// `TOGetMessageWindowMode()` (`0x008ac7b0`).
    pub fn message_window_mode(mem: &impl Mem) -> u32 {
        MESSAGE_WINDOW_MODE.get(mem)
    }

    /// `TOGetFFXToAlbhedTblAdrs()` (`0x00906b20`).
    pub fn albhed_table(mem: &impl Mem) -> u32 {
        ALBHED_TABLE.ptr(mem).addr
    }

    /// `TOGetFontWidthTblAdrs(font)` (`0x008ac3d0`).
    pub fn font_width_table(mem: &impl Mem, font: i32) -> u32 {
        mem.read(
            FONT_WIDTH_TABLES
                .ptr(mem)
                .addr
                .wrapping_add(font.cast_unsigned().wrapping_mul(16)),
        )
    }

    /// `TOFFXToSJIS(text, out)` (`0x008b6200`): the PC port's stub, an
    /// empty string.
    pub fn ffx_to_sjis(mem: &mut impl Mem, out: u32) {
        mem.write(out, 0u8);
    }

    /// `TOMenuGetGetImportantName(id)` (`0x008ddee0`, `field` 0) and
    /// `TOMenuGetGetImportantHelp(id)` (`0x008ddea0`, `field` 8): the key
    /// item's name or help text, given `MsGetSaveConfigHiragana()` and
    /// `MsImportantPtr(id)` as (record, strings): the strings plus the
    /// record's `u16` offset (the second one, `+4`, with hiragana on).
    pub fn important_text(
        mem: &impl Mem,
        field: u32,
        hiragana: u32,
        record: u32,
        strings: u32,
    ) -> u32 {
        let off = field + if hiragana != 0 { 4 } else { 0 };
        strings.wrapping_add(u32::from(mem.read::<u16>(record.wrapping_add(off))))
    }

    /// `0x008c8d10(chr, delta)`: character `chr`'s HP (`TkMenuGetHP`) plus
    /// `delta`, capped (signed) at its maximum (`TkMenuGetMaxHP`, asked
    /// again for the cap), set with `TkMenuSetHP`; what that returns. The
    /// three are the menu's, passed in.
    pub fn add_hp_capped<M: Mem>(
        mem: &mut M,
        chr: u32,
        delta: u32,
        get_hp: impl FnOnce(&mut M, u32) -> u32,
        mut get_max_hp: impl FnMut(&mut M, u32) -> u32,
        set_hp: impl FnOnce(&mut M, u32, u32) -> u32,
    ) -> u32 {
        let mut hp = get_hp(mem, chr).wrapping_add(delta);
        if hp.cast_signed() >= get_max_hp(mem, chr).cast_signed() {
            hp = get_max_hp(mem, chr);
        }
        set_hp(mem, chr, hp)
    }

    /// `TOMenuGetControlPadAnalogHist(pad, port, k, i)` (`0x008be450`): with
    /// the dummy controller on (`TkMenuGetDummyControlerFlag` 1) the dummy
    /// analog axis `k` ([`crate::pad::bridge::dummy_analog`]), else (a tail
    /// call) `TkGetControlerAnalogHist(pad, port, k, i)`.
    pub fn control_pad_analog_hist<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        dummy_flag: impl FnOnce(&mut M) -> u32,
        analog_hist: impl FnOnce(&mut M, [u32; 4]) -> u32,
    ) -> u32 {
        if dummy_flag(mem) == 1 {
            u32::from(crate::pad::bridge::dummy_analog(mem, args[2]))
        } else {
            analog_hist(mem, args)
        }
    }

    /// The PS2 product codes `0x00779ca0` picks the save directory name
    /// from, by `TkGetLang()`: 1 `SLUS-20312`, 2 `SCES-50491`, 3
    /// `SCES-50494`, 4 `SCES-50492`, 5 `SCES-50493`, 18 `SCES-50490`, any
    /// other `SLPS-25088` (Japanese).
    pub fn save_product_code(mem: &impl Mem, lang: i32) -> u32 {
        mem.rebase(match lang {
            1 => 0x00b5_39c8,
            2 => 0x00b5_39f8,
            3 => 0x00b5_3a28,
            4 => 0x00b5_39e0,
            5 => 0x00b5_3a10,
            18 => 0x00b5_3a40,
            _ => 0x00b5_3a58,
        })
    }

    /// The save-region flag `TOSwapInternationalEnvInit` sets (`0x00c59564`).
    pub const SAVE_REGION: Global<u32> = Global::new(0x00c5_9564);
    /// Four words it clears (`0x01841d28`).
    pub const INTERNATIONAL_ENV: Global<[u32; 4]> = Global::new(0x0184_1d28);

    /// `TOSwapInternationalEnvInit(english)` (`0x008b0920`):
    /// `MsSetSaveConfigEnglish(english)` (the battle module's, passed in),
    /// then [`SAVE_REGION`] 4 for `english` 1 and 0x12 otherwise, and
    /// [`INTERNATIONAL_ENV`] zeroed; the call's `eax`.
    pub fn swap_international_env_init<M: Mem>(
        mem: &mut M,
        english: u32,
        set_english: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let r = set_english(mem, english);
        SAVE_REGION.set(mem, if english == 1 { 4 } else { 0x12 });
        INTERNATIONAL_ENV.set(mem, [0; 4]);
        r
    }

    /// `TOGetFFXLangStr()` (`0x008ac300`): the two-letter code of the
    /// language `TOGetFFXLang()` returned (`lang`): 1 `us`, 2 `fr`, 3 `es`,
    /// 4 `de`, 5 `it`, 9 `kr`, 10 `ch`, 18 `uk`, any other `jp`; an image
    /// string.
    pub fn ffx_lang_str(mem: &impl Mem, lang: u32) -> u32 {
        mem.rebase(match lang {
            1 => 0x00b5_46b8,
            2 => 0x00b7_c908,
            3 => 0x00b7_c914,
            4 => 0x00b7_c90c,
            5 => 0x00b7_c910,
            9 => 0x00b7_c918,
            10 => 0x00b7_c91c,
            18 => 0x00b5_46bc,
            _ => 0x00b7_c900,
        })
    }

    /// The second save-region flag (`0x00c59978`).
    pub const SAVE_REGION2: Global<u32> = Global::new(0x00c5_9978);
    /// Two words `0x008b0970` clears (`0x01841d38`).
    pub const INTERNATIONAL_ENV2: Global<[u32; 2]> = Global::new(0x0184_1d38);

    /// `0x008b0970(english)`: [`swap_international_env_init`]'s twin:
    /// `MsSetSaveConfigEnglish(english)`, [`SAVE_REGION2`] 4 or 0x12,
    /// [`INTERNATIONAL_ENV2`] zeroed; the call's `eax`.
    pub fn swap_international_env_init2<M: Mem>(
        mem: &mut M,
        english: u32,
        set_english: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let r = set_english(mem, english);
        SAVE_REGION2.set(mem, if english == 1 { 4 } else { 0x12 });
        INTERNATIONAL_ENV2.set(mem, [0; 2]);
        r
    }

    /// `TOMenuAtelReq(work, entry, level)` (`0x008be370`): with the event
    /// script controller switched to 6 (`AtelChangeCurrentCtrl`), signal
    /// `entry` of `work` registered without duplicate tags
    /// (`AtelRegistSignalWithoutSameTag(0xffff, entry, 0, work, level)`),
    /// then the previous controller restored (`AtelSetCurrentCtrl`); what
    /// that returns. The three are the script module's, passed in.
    pub fn menu_atel_req<M: Mem>(
        mem: &mut M,
        args: [u32; 3],
        change_ctrl: impl FnOnce(&mut M, u32) -> u32,
        regist: impl FnOnce(&mut M, [u32; 5]) -> u32,
        set_ctrl: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let saved = change_ctrl(mem, 6);
        regist(mem, [0xffff, args[1], 0, args[0], args[2]]);
        set_ctrl(mem, saved)
    }
}
