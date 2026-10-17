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

    /// `TkMenuGetImportantName(id)` (`0x008ddee0`, `field` 0) and
    /// `TkMenuGetImportantHelp(id)` (`0x008ddea0`, `field` 8): the key
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

    /// The magic file's texture-loaded flag (`0x01940ab0`).
    pub const MAGIC_TEXTURE_LOADED: Global<u32> = Global::new(0x0194_0ab0);
    /// The magic file (`0x00c64ca0`): its magic id first, its handle at +0xc (-1 for none).
    pub const MAGIC_FILE: Global<[u32; 4]> = Global::new(0x00c6_4ca0);

    /// `IsLoadTexture()` (`0x009da280`): with a magic file but its texture
    /// not loaded yet, `graphicMagicVFXLoadTexture(magic id)` (passed in)
    /// and the flag set; 1, or 0 without a magic file.
    pub fn is_load_texture<M: Mem>(mem: &mut M, load: impl FnOnce(&mut M, u32)) -> u32 {
        if MAGIC_TEXTURE_LOADED.get(mem) == 0 {
            let f = MAGIC_FILE.get(mem);
            if f[3] == u32::MAX {
                return 0;
            }
            load(mem, f[0]);
            MAGIC_TEXTURE_LOADED.set(mem, 1);
        }
        1
    }

    /// The calls of [`magic_file_start`] and [`magic_file_set_magic_id`].
    pub trait MagicFileCalls<M> {
        /// `printf(fmt, arg)` (the CRT's; debug output).
        fn printf(&mut self, mem: &mut M, fmt: u32, arg: Option<u32>);
        /// `graphicSetGameOverNotify(on)`.
        fn game_over_notify(&mut self, mem: &mut M, on: u32);
        /// The loaded magic module's entry (`*0x022fb6a0`) with the import
        /// table (`g_MagicImports`, `0x00c64ce8`).
        fn init_magic(&mut self, mem: &mut M, imports: u32);
        /// `graphicSetChrFogFade(on)`.
        fn fog_fade(&mut self, mem: &mut M, on: u32);
        /// `resetZanmatoSepList()`.
        fn reset_zanmato(&mut self, mem: &mut M);
    }

    /// `MagicFile_SetMagicId(id)` (`0x009da7a0`): 0x2c7 becomes 0x296; the
    /// id stored (`g_MagicFile_MagicId`) and printed when no magic file is
    /// open (handle -1), else only a different message printed.
    pub fn magic_file_set_magic_id<M: Mem>(
        mem: &mut M,
        id: u32,
        calls: &mut impl MagicFileCalls<M>,
    ) {
        let id = if id == 0x2c7 { 0x296 } else { id };
        if MAGIC_FILE.get(mem)[3] == u32::MAX {
            let at = MAGIC_FILE.ptr(mem).addr;
            mem.write(at, id);
            let fmt = mem.rebase(0x00b7_06bc);
            calls.printf(mem, fmt, Some(id));
        } else {
            let fmt = mem.rebase(0x00b7_06e0);
            calls.printf(mem, fmt, Some(id));
        }
    }

    /// `MagicFile_Start()` (`0x009da7f0`): 0 (after a message) without an
    /// open magic file; else `graphicSetGameOverNotify(id == 0x29e)`, the
    /// module's entry called with the import table, the id printed,
    /// `graphicSetChrFogFade(1)` for 0xa8 and `resetZanmatoSepList()` for
    /// 0x1b2 and 0x262 (Zanmato); 1.
    pub fn magic_file_start<M: Mem>(mem: &mut M, calls: &mut impl MagicFileCalls<M>) -> u32 {
        if MAGIC_FILE.get(mem)[3] == u32::MAX {
            let fmt = mem.rebase(0x00b7_0614);
            calls.printf(mem, fmt, None);
            return 0;
        }
        let id = MAGIC_FILE.get(mem)[0];
        calls.game_over_notify(mem, u32::from(id == 0x29e));
        let imports = mem.rebase(0x00c6_4ce8);
        calls.init_magic(mem, imports);
        let id = MAGIC_FILE.get(mem)[0];
        let fmt = mem.rebase(0x00b7_0648);
        calls.printf(mem, fmt, Some(id));
        let id = MAGIC_FILE.get(mem)[0];
        if id == 0xa8 {
            calls.fog_fade(mem, 1);
        }
        let id = MAGIC_FILE.get(mem)[0];
        if id == 0x1b2 || id == 0x262 {
            calls.reset_zanmato(mem);
        }
        1
    }

    /// `MagicFile_Cleanup()` (`0x009da2d0`): the magic path queued for
    /// deletion (`graphicMagicSetToBeDeletePath(graphicMagicGetPath())`) and
    /// reset (`graphicMagicSetPath(0x00b8fbeb)`); the id kept as the
    /// previous one (`+8` and `+4`), the handle in `+0x10`, both -1.
    pub fn magic_file_cleanup<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let path = call(mem, 0x0064_2030, &[]);
        call(mem, 0x0064_2070, &[path]);
        let empty = mem.rebase(0x00b8_fbeb);
        call(mem, 0x0064_2050, &[empty]);
        let f = MAGIC_FILE.ptr(mem).addr;
        let id: u32 = mem.read(f);
        mem.write(f.wrapping_add(8), id);
        mem.write(f.wrapping_add(4), id);
        let handle: u32 = mem.read(f.wrapping_add(0xc));
        mem.write(f, u32::MAX);
        mem.write(f.wrapping_add(0x10), handle);
        mem.write(f.wrapping_add(0xc), u32::MAX);
    }

    /// `MagicFile_NeedRestoreCamera()` (`0x009da700`): 1 when `0x01940abc`
    /// is set or the previous magic (`0x00c64ca8`) is 0xa5 or 0x133, or 0x152
    /// in battle scene 0x018b0046 (`MsGetBattleScene`, through `call`).
    pub fn magic_file_need_restore_camera<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(mem.rebase(0x0194_0abc)) != 0 {
            return 1;
        }
        let prev: u32 = mem.read(MAGIC_FILE.ptr(mem).addr.wrapping_add(8));
        if prev == 0xa5 || prev == 0x133 {
            return 1;
        }
        if prev == 0x152 && call(mem, 0x0078_2970, &[]) == 0x018b_0046 {
            return 1;
        }
        0
    }

    /// `MagicFile_RestoreScreenSetup()` (`0x009da780`):
    /// `graphicCameraSetFOVy(0x022fb600)`.
    pub fn magic_file_restore_screen_setup<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = mem.rebase(0x022f_b600);
        call(mem, 0x0063_95e0, &[at])
    }

    /// `MagicFile_isPrxPreloaded(id)` (`0x009dae60`): 1 when
    /// `MagicFile_FindPreloaded(id, &entry)` (through `call`, the entry in
    /// the scratch word `tmp`) succeeds with a non-null entry.
    pub fn magic_file_is_prx_preloaded<M: Mem>(
        mem: &mut M,
        id: u32,
        tmp: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        u32::from(call(mem, 0x009d_ad70, &[id, tmp]) != 0 && mem.read::<u32>(tmp) != 0)
    }
}
