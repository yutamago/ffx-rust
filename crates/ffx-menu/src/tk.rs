//! The PS2 `Tk` toolkit: menu state, pad mapping, vibration, the save CRC,
//! menu command lists and small GS packets.

/// `Pad_MapButtons`: each set bit `i` of `buttons` becomes bit `row[i]`
/// (shifted on 16 bits, so a target of 16 or more drops the button).
#[must_use]
pub fn map_buttons(row: [u8; 16], buttons: u16) -> u16 {
    let mut out = 0u16;
    for (i, &to) in row.iter().enumerate() {
        if buttons & (1 << i) != 0 {
            #[allow(clippy::cast_possible_truncation)] // the 16-bit shift
            let bit = (1u32 << (to & 0x1f)) as u16;
            out |= bit;
        }
    }
    out
}

/// `TkSetVibrateMask`: pad `pad`'s bit set or cleared (the shift is on 16
/// bits, so pads 16 and up change nothing); a negative pad sets all eight
/// (`0xff`) or none.
#[must_use]
pub fn vibrate_mask(mask: u16, pad: i32, on: bool) -> u16 {
    if pad < 0 {
        return if on { 0xff } else { 0 };
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the 16-bit shift
    let bit = (1u32 << (pad as u32 & 0x1f)) as u16;
    if on { mask | bit } else { mask & !bit }
}

/// The CRC table `TkCRC` builds: CRC-16 (polynomial `0x1021`) of each byte,
/// except entry 255, which stays 0 (its loop stops at 254).
#[must_use]
pub fn crc_table() -> [u16; 256] {
    let mut t = [0u16; 256];
    for (i, e) in t.iter_mut().enumerate().take(0xff) {
        #[allow(clippy::cast_possible_truncation)]
        let mut v = (i as u32) << 8;
        for _ in 0..8 {
            v = if v & 0x8000 != 0 {
                (v << 1) ^ 0x1021
            } else {
                v << 1
            };
        }
        #[allow(clippy::cast_possible_truncation)]
        {
            *e = v as u16;
        }
    }
    t
}

/// `TkCRC`: CRC-16/CCITT-FALSE (initial `0xffff`), inverted, but with the
/// table's entry 255 zero: bytes that index it give another value than the
/// standard CRC.
#[must_use]
pub fn crc(data: &[u8]) -> u16 {
    let t = crc_table();
    let mut c = 0xffffu16;
    for &b in data {
        c = (c << 8) ^ t[usize::from((c >> 8) as u8 ^ b)];
    }
    !c
}

/// `MenuMn2_Plus80(v)` (`0x008c22f0`, own name): `v + 0x80`.
#[must_use]
pub const fn plus_80(v: u32) -> u32 {
    v.wrapping_add(0x80)
}

/// `MenuItem_Less(a, b)` (`0x008c6a60`, own name): `a < b`, signed.
#[must_use]
pub const fn less(a: i32, b: i32) -> bool {
    a < b
}

/// `BtlEnd_RowX(i, n)` (`0x008ba370`, own name): `(7 - n) × 44 / 2 + i ×
/// 44 + 76`, the division truncating toward 0.
#[must_use]
pub const fn btl_end_row_x(i: i32, n: i32) -> i32 {
    (7i32.wrapping_sub(n)).wrapping_mul(0x2c) / 2 + i.wrapping_mul(0x2c).wrapping_add(0x4c)
}

/// `MenuHelp_Ratio(a, b)` (`0x008bf120`, own name): `(b + 2) × a × 2 / 4`,
/// truncating toward 0.
#[must_use]
pub const fn menu_help_ratio(a: i32, b: i32) -> i32 {
    b.wrapping_add(2).wrapping_mul(a).wrapping_mul(2) / 4
}

/// A menu command list: 4-byte records (`u8` a, `u8` b, two more bytes)
/// ending at a record whose first byte is `0xff`. `TkMenuGetCommandOffset`:
/// the index of the first record matching `(a, b)`, 0 when none does (or
/// the list is disabled).
pub fn command_offset(records: impl Iterator<Item = (u8, u8)>, a: u32, b: u32) -> i32 {
    for (i, (x, y)) in records.enumerate() {
        if u32::from(x) == a && u32::from(y) == b {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            return i as i32;
        }
    }
    0
}

/// `TkMenuGetCommandMax`: how many records match `(a, b)`.
pub fn command_count(records: impl Iterator<Item = (u8, u8)>, a: u32, b: u32) -> i32 {
    let mut n = 0;
    for (x, y) in records {
        if u32::from(x) == a && u32::from(y) == b {
            n += 1;
        }
    }
    n
}

/// `TkMenuDrawEnv`'s packet after its 16-byte header (left as it was): a
/// GIF tag for four A+D registers and `CLAMP_1`, `TEST_1`, `ALPHA_1` and
/// `PABE`.
pub const DRAW_ENV: [u32; 20] = [
    0x8004,
    0x1000_0000,
    0xe,
    0, // GIF tag: 4 loops, end of packet, A+D
    5,
    0,
    8,
    0, // CLAMP_1
    0x3_000d,
    0,
    0x47,
    0, // TEST_1
    0x44,
    0,
    0x42,
    0, // ALPHA_1
    0,
    0,
    0x49,
    0, // PABE
];

/// The originals, on the game's globals (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use ffx_battle::save::bridge as save;
    use ffx_field::yi::bridge as yi;

    use super::{DRAW_ENV, command_count, command_offset, crc, map_buttons, vibrate_mask};

    /// The game language (`FFXSetLang`, `TkGetLang`; `i8`).
    pub const LANG: Global<i8> = Global::new(0x0133_0830);
    /// `TkGetTGS`'s byte.
    pub const TGS: Global<u8> = Global::new(0x0133_0832);
    /// The debug font's position, ×16 (`TkSetDebugFontLocate`).
    pub const DEBUG_FONT_POS: Global<[i32; 2]> = Global::new(0x0133_c8d4);
    /// The menu's party (`u8` character ids).
    pub const MENU_PLAYERS: Global<u8> = Global::new(0x0184_1c14);
    /// `TkMenuGetPlayerListMax` without the ignore-join flag.
    pub const MENU_PLAYER_LIST_MAX: Global<u32> = Global::new(0x0184_1c1c);
    /// `TkMenuGetPlayerListMax2`.
    pub const MENU_PLAYER_LIST_MAX2: Global<u32> = Global::new(0x0184_1c20);
    /// The index of the menu's current character in [`MENU_PLAYERS`].
    pub const MENU_CURRENT_PLAYER_POS: Global<u32> = Global::new(0x0184_1c28);
    /// The menu's aeons (`u8` ids).
    pub const MENU_SUMMONS: Global<u8> = Global::new(0x0184_1c38);
    /// `TkMenuGetSummonListMax`.
    pub const MENU_SUMMON_LIST_MAX: Global<u32> = Global::new(0x0184_1c50);
    /// The index of the menu's current aeon in [`MENU_SUMMONS`].
    pub const MENU_CURRENT_SUMMON_POS: Global<u32> = Global::new(0x0184_1c54);
    /// `TkMenuSummonEnableMask`.
    pub const MENU_SUMMON_ENABLE_MASK: Global<u32> = Global::new(0x0184_1c5c);
    /// `TkMn2SetMainFrameRatioDiff`.
    pub const MN2_MAIN_FRAME_RATIO_DIFF: Global<u32> = Global::new(0x0184_1c64);
    /// `TkMn2GetMainFrameRatio`.
    pub const MN2_MAIN_FRAME_RATIO: Global<u32> = Global::new(0x0184_1c68);
    /// The help message shown, the one before it, and its timer.
    pub const MENU_HELP_MESSAGE: Global<[i32; 3]> = Global::new(0x0184_1c00);
    /// `setButtonData`'s two words.
    pub const BUTTON_DATA: Global<[u32; 2]> = Global::new(0x00c5_955c);
    /// `TkMn2SetFontShade`'s four values, stored downwards from `0x023cc470`.
    pub const MN2_FONT_SHADE: Global<u32> = Global::new(0x023c_c458);
    /// The pad button maps, 16 bytes per row (`Pad_MapButtons`).
    pub const PAD_BUTTON_MAP: Global<[u8; 16]> = Global::new(0x0133_0488);
    /// The FFX configuration block (`TkGetFFXCofingAdrs`).
    pub const FFX_CONFIG: Global<u8> = Global::new(0x0133_8880);
    /// The open sub-command window.
    pub const MENU_SUB_CMD_WINDOW: Global<u32> = Global::new(0x023c_c128);
    /// Vibration: the two volumes, the "off" byte, and the pad mask.
    pub const VIBRATE_VOLUME: Global<[u8; 2]> = Global::new(0x0132_8270);
    /// Nonzero: vibration off (`TkGetVibrateVolume` reports 0).
    pub const VIBRATE_OFF: Global<u8> = Global::new(0x0132_8272);
    /// Pads allowed to vibrate (`TkSetVibrateMask`).
    pub const VIBRATE_MASK: Global<u16> = Global::new(0x0132_8274);
    /// `TkMn2GetMenuMode`.
    pub const MN2_MENU_MODE: Global<u32> = Global::new(0x0184_08f8);
    /// `TkMenuMainGetMenuActiveMask`.
    pub const MENU_MAIN_ACTIVE_MASK: Global<u32> = Global::new(0x0184_08ec);
    /// `TkMenuMainSetIgnoreJoinFlag`.
    pub const MENU_MAIN_IGNORE_JOIN: Global<u32> = Global::new(0x0184_0870);
    /// `TkMenuGetDrawShadeRatio`.
    pub const MENU_DRAW_SHADE_RATIO: Global<u32> = Global::new(0x0184_087c);
    /// The dummy controller (`TkMenuFlushDummyControler`): buttons at
    /// `+0`, pending at `+4`, then current/pending byte pairs.
    pub const MENU_DUMMY_CONTROLER: Global<u32> = Global::new(0x0186_a420);
    /// `TkMenuGetDummyControlerFlag`.
    pub const MENU_DUMMY_CONTROLER_FLAG: Global<u8> = Global::new(0x0186_a430);
    /// `TkMenuGetImportantCounter`.
    pub const MENU_IMPORTANT_COUNTER: Global<u32> = Global::new(0x0186_a380);
    /// `TkMenuImportantListMax`.
    pub const MENU_IMPORTANT_LIST_MAX: Global<u32> = Global::new(0x0186_a388);
    /// The main menu's options, 8 bytes each (`u32` menu, `i16` enabled).
    pub const MENU_MAIN_OPTIONS: Global<u32> = Global::new(0x0187_1568);
    /// `TkMenuMainGetMaxOptions`.
    pub const MENU_MAIN_MAX_OPTIONS: Global<u32> = Global::new(0x0187_1688);
    /// The menu's packet pointer (`TkMenuDrawEnv` appends there).
    pub const MENU_PACKET: Global<u32> = Global::new(0x0187_1530);
    /// `TkSndSetStandbyVoice`.
    pub const SND_STANDBY_VOICE: Global<u32> = Global::new(0x0132_ff1c);
    /// `TkSndReportStatus` (nonzero: busy).
    pub const SND_STATUS: Global<u32> = Global::new(0x0132_ff24);
    /// `TkSndClearKickFlag`.
    pub const SND_KICK_FLAG: Global<u8> = Global::new(0x0133_0094);
    /// `TkClearSoundSyncVFlag`.
    pub const SOUND_SYNC_V_FLAG: Global<u32> = Global::new(0x0133_01e4);
    /// `TkGetTotalSyncVCounter`.
    pub const TOTAL_SYNC_V_COUNTER: Global<u32> = Global::new(0x0133_0210);
    /// The background thread function (`TkRegistBGThreadFunc`).
    pub const BG_THREAD_FUNC: Global<u32> = Global::new(0x0133_0254);
    /// Sync-V callback handles, three.
    pub const SYNC_V_CALLBACK_HANDLES: Global<u32> = Global::new(0x0133_025c);
    /// Sync-V callback pointers, three.
    pub const SYNC_V_CALLBACK_PTRS: Global<u32> = Global::new(0x0133_026c);
    /// `TkInitControlerWork`'s two words.
    pub const CONTROLER_WORK: Global<[u32; 2]> = Global::new(0x0133_0280);
    /// `TkGetSyncH`'s table.
    pub const SYNC_H: Global<u32> = Global::new(0x0133_080c);
    /// The fade-out, twice (`TkSetFadeOut`).
    pub const FADE_OUT: Global<[u32; 2]> = Global::new(0x0133_0850);
    /// `TkGetEncountCounterEventStart`.
    pub const ENCOUNT_COUNTER_EVENT_START: Global<u32> = Global::new(0x0133_086c);
    /// `TkSetDebugMenuStatus` (0 or 1).
    pub const DEBUG_MENU_STATUS: Global<u8> = Global::new(0x0133_0870);
    /// `TkGetBattleMusicSize` (its address).
    pub const BATTLE_MUSIC_SIZE: Global<u32> = Global::new(0x0133_0880);
    /// `TkGetBattleMusicBuffer`.
    pub const BATTLE_MUSIC_BUFFER: Global<u32> = Global::new(0x0133_c8c0);
    /// `TkMenuSaveLoadActive` clears it.
    pub const MENU_SAVE_LOAD_ACTIVE: Global<u32> = Global::new(0x0186_63c4);
    /// Vibration per pad, 8 records of `0x24` bytes: two channels of 16
    /// (`+0` pointer, `+4` `u32`, `+8` `u16`, `+0xa` `u8`), then `+0x22` on.
    pub const VIBRATE_PADS: Global<u8> = Global::new(0x0132_8150);
    /// Bytes per pad in [`VIBRATE_PADS`].
    pub const VIBRATE_PAD: u32 = 0x24;
    /// The last change of character: 1 next, `0xff` previous.
    pub const MENU_PLAYER_STEP: Global<u8> = Global::new(0x0184_0886);
    /// How many summon growth records lead with byte `+7` zero
    /// (`SetSummonGrowMax`, once).
    pub const MENU_SUMMON_GROW_MAX: Global<u32> = Global::new(0x0186_a23c);

    /// Originals that read one global (`get`), store their argument in one
    /// (`set`), or store a constant (`clear`).
    macro_rules! globals {
        ($($doc:literal $f:ident: $kind:ident $g:ident $(= $v:expr)?;)*) => {$(
            globals!(@ $doc $f $kind $g $($v)?);
        )*};
        (@ $doc:literal $f:ident get $g:ident) => {
            #[doc = $doc]
            pub fn $f(mem: &impl Mem) -> u32 {
                $g.get(mem).into()
            }
        };
        (@ $doc:literal $f:ident set $g:ident) => {
            #[doc = $doc]
            pub fn $f(mem: &mut impl Mem, v: u32) {
                $g.set(mem, v);
            }
        };
        (@ $doc:literal $f:ident clear $g:ident $v:expr) => {
            #[doc = $doc]
            pub fn $f(mem: &mut impl Mem) {
                $g.set(mem, $v);
            }
        };
    }

    globals! {
        "`TkMenuGetCurrentPlayerPos()` (`0x008a9870`)." tk_menu_get_current_player_pos: get MENU_CURRENT_PLAYER_POS;
        "`TkMenuSetCurrentPlayerPos(i)` (`0x008aae80`)." tk_menu_set_current_player_pos: set MENU_CURRENT_PLAYER_POS;
        "`TkMenuGetPlayerListMax2()` (`0x008a9b50`)." tk_menu_get_player_list_max2: get MENU_PLAYER_LIST_MAX2;
        "`TkMenuGetSummonListMax()` (`0x008a9bf0`)." tk_menu_get_summon_list_max: get MENU_SUMMON_LIST_MAX;
        "`TkMenuSummonEnableMask()` (`0x008ab1e0`)." tk_menu_summon_enable_mask: get MENU_SUMMON_ENABLE_MASK;
        "`TkMn2SetMainFrameRatioDiff(v)` (`0x008ab260`)." tk_mn2_set_main_frame_ratio_diff: set MN2_MAIN_FRAME_RATIO_DIFF;
        "`TkMn2GetMainFrameRatio()` (`0x008ab230`)." tk_mn2_get_main_frame_ratio: get MN2_MAIN_FRAME_RATIO;
        "`TkMn2GetMenuMode()` (`0x008ab240`)." tk_mn2_get_menu_mode: get MN2_MENU_MODE;
        "`TkMenuMainGetMenuActiveMask()` (`0x008aa460`)." tk_menu_main_get_menu_active_mask: get MENU_MAIN_ACTIVE_MASK;
        "`TkMenuMainSetIgnoreJoinFlag(v)` (`0x008aad60`)." tk_menu_main_set_ignore_join_flag: set MENU_MAIN_IGNORE_JOIN;
        "`TkMenuGetDrawShadeRatio()` (`0x008a98a0`)." tk_menu_get_draw_shade_ratio: get MENU_DRAW_SHADE_RATIO;
        "`TkMenuDrawShadeRatioDiff(v)` (`0x008a9670`): sets the ratio." tk_menu_draw_shade_ratio_diff: set MENU_DRAW_SHADE_RATIO;
        "`TkMenuGetDummyControlerFlag()` (`0x008ca510`)." tk_menu_get_dummy_controler_flag: get MENU_DUMMY_CONTROLER_FLAG;
        "`TkMenuGetImportantCounter()` (`0x008c8a10`)." tk_menu_get_important_counter: get MENU_IMPORTANT_COUNTER;
        "`TkMenuImportantListMax()` (`0x008c8a20`)." tk_menu_important_list_max: get MENU_IMPORTANT_LIST_MAX;
        "`TkMenuMainGetMaxOptions()` (`0x008e2280`)." tk_menu_main_get_max_options: get MENU_MAIN_MAX_OPTIONS;
        "`TkSndSetStandbyVoice(v)` (`0x00887b50`)." tk_snd_set_standby_voice: set SND_STANDBY_VOICE;
        "`TkGetEncountCounterEventStart()` (`0x0088d5c0`)." tk_get_encount_counter_event_start: get ENCOUNT_COUNTER_EVENT_START;
        "`TkGetBattleMusicBuffer()` (`0x0088d580`)." tk_get_battle_music_buffer: get BATTLE_MUSIC_BUFFER;
        "`TkSndClearKickFlag()` (`0x00887ab0`)." tk_snd_clear_kick_flag: clear SND_KICK_FLAG = 0;
        "`TkClearSoundSyncVFlag()` (`0x00887d30`)." tk_clear_sound_sync_v_flag: clear SOUND_SYNC_V_FLAG = 0;
        "`TkMenuSaveLoadActive()` (`0x008b55c0`): clears the flag." tk_menu_save_load_active: clear MENU_SAVE_LOAD_ACTIVE = 0;
        "`TkInitControlerWork()` (`0x00888f00`)." tk_init_controler_work: clear CONTROLER_WORK = [0; 2];
    }

    /// `TkMenuSetDummyControlerFlag(v)` (`0x008ca570`): the low byte.
    pub fn tk_menu_set_dummy_controler_flag(mem: &mut impl Mem, v: u8) {
        MENU_DUMMY_CONTROLER_FLAG.set(mem, v);
    }

    /// `TkSndReportStatus()` (`0x00887b30`): whether the sound status is
    /// nonzero.
    pub fn tk_snd_report_status(mem: &impl Mem) -> bool {
        SND_STATUS.get(mem) != 0
    }

    /// `TkGetLastSyncVCounter()` (`0x00887dc0`) and `TkGetSoundSyncVFlag()`
    /// (`0x00887dd0`): always 1 on the PC.
    #[must_use]
    pub const fn tk_get_last_sync_v_counter() -> u32 {
        1
    }

    /// `TkGetLang()` (`0x00887d00`), sign-extended.
    pub fn tk_get_lang(mem: &impl Mem) -> i32 {
        i32::from(LANG.get(mem))
    }

    /// `FFXSetLang(lang)` (`0x00887d20`).
    pub fn ffx_set_lang(mem: &mut impl Mem, lang: i8) {
        LANG.set(mem, lang);
    }

    /// `TkGetTGS()` (`0x0088d6d0`).
    pub fn tk_get_tgs(mem: &impl Mem) -> u8 {
        TGS.get(mem)
    }

    /// `TkSetDebugFontLocate(x, y)` (`0x0088faf0`).
    pub fn tk_set_debug_font_locate(mem: &mut impl Mem, x: i32, y: i32) {
        DEBUG_FONT_POS.set(mem, [x.wrapping_shl(4), y.wrapping_shl(4)]);
    }

    /// `TkMenuGetCurrentPlayer()` (`0x008a9860`): the current character's id.
    pub fn tk_menu_get_current_player(mem: &impl Mem) -> u8 {
        let i = MENU_CURRENT_PLAYER_POS.get(mem);
        mem.read(MENU_PLAYERS.ptr(mem).addr.wrapping_add(i))
    }

    /// `TkMenuGetCurrentSummon()` (`0x008a9880`).
    pub fn tk_menu_get_current_summon(mem: &impl Mem) -> u8 {
        let i = MENU_CURRENT_SUMMON_POS.get(mem);
        mem.read(MENU_SUMMONS.ptr(mem).addr.wrapping_add(i))
    }

    /// `TkMenuSetHelpMessage(id)` (`0x008aaeb0`): a new id keeps the old
    /// one and restarts the timer.
    pub fn tk_menu_set_help_message(mem: &mut impl Mem, id: i32) {
        let [cur, _, _] = MENU_HELP_MESSAGE.get(mem);
        if id != cur {
            let p = MENU_HELP_MESSAGE.ptr(mem).addr;
            mem.write(p + 4, cur);
            mem.write(p, id);
            mem.write(p + 8, 0i32);
        }
    }

    /// `resetButtonData()` (`0x008b1330`).
    pub fn reset_button_data(mem: &mut impl Mem) {
        BUTTON_DATA.set(mem, [u32::MAX; 2]);
    }

    /// `setButtonData(a, b)` (`0x008b1350`).
    pub fn set_button_data(mem: &mut impl Mem, a: u32, b: u32) {
        BUTTON_DATA.set(mem, [a, b]);
    }

    /// `TkMn2SetFontShade(s0, s1, s2, s3)` (`0x008c2c30`): `s0` at
    /// `0x023cc470` down to `s3` at `0x023cc458`, 8 bytes apart.
    pub fn tk_mn2_set_font_shade(mem: &mut impl Mem, s0: u32, s1: u32, s2: u32, s3: u32) {
        let base = MN2_FONT_SHADE.ptr(mem).addr;
        for (o, v) in [(0x18, s0), (0x10, s1), (8, s2), (0, s3)] {
            mem.write(base + o, v);
        }
    }

    /// `Pad_MapButtons(a, b, buttons)` (`0x00888c40`): through row `a + b`.
    pub fn pad_map_buttons(mem: &impl Mem, a: i32, b: i32, buttons: u16) -> u16 {
        #[allow(clippy::cast_sign_loss)]
        let row = PAD_BUTTON_MAP.ptr(mem).at(a.wrapping_add(b) as u32);
        map_buttons(row.get(mem), buttons)
    }

    /// `TkGetFFXCofingAdrs()` (`0x0088d5e0`).
    pub fn tk_get_ffx_cofing_adrs(mem: &impl Mem) -> u32 {
        FFX_CONFIG.ptr(mem).addr
    }

    /// `TkGetBattleMusicSize()` (`0x0088d570`): where the size is.
    pub fn tk_get_battle_music_size(mem: &impl Mem) -> u32 {
        BATTLE_MUSIC_SIZE.ptr(mem).addr
    }

    /// `TkSetVibrateMask(pad, on)` (`0x00875f60`): see [`vibrate_mask`].
    pub fn tk_set_vibrate_mask(mem: &mut impl Mem, pad: i32, on: i32) {
        let m = vibrate_mask(VIBRATE_MASK.get(mem), pad, on != 0);
        VIBRATE_MASK.set(mem, m);
    }

    /// `TkGetVibrateVolume(&a, &b)` (`0x00875e10`): 0 and 0 when vibration
    /// is off.
    pub fn tk_get_vibrate_volume(mem: &mut impl Mem, a: u32, b: u32) {
        let [x, y] = if VIBRATE_OFF.get(mem) == 0 {
            VIBRATE_VOLUME.get(mem)
        } else {
            [0, 0]
        };
        mem.write(a, u32::from(x));
        mem.write(b, u32::from(y));
    }

    /// `TkSetDebugMenuStatus(on)` (`0x0088ea30`).
    pub fn tk_set_debug_menu_status(mem: &mut impl Mem, on: i32) {
        DEBUG_MENU_STATUS.set(mem, u8::from(on != 0));
    }

    /// `TkSetFadeOut(v)` (`0x0088eb50`).
    pub fn tk_set_fade_out(mem: &mut impl Mem, v: u32) {
        FADE_OUT.set(mem, [v, v]);
    }

    /// `TkGetSyncH(i)` (`0x0088c800`), unchecked.
    pub fn tk_get_sync_h(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        SYNC_H.ptr(mem).at(i as u32).get(mem)
    }

    /// `TkGetSyncVCallbackHandle(i)` (`0x00888620`): `i` clamped to 0..=2.
    pub fn tk_get_sync_v_callback_handle(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        SYNC_V_CALLBACK_HANDLES
            .ptr(mem)
            .at(i.clamp(0, 2) as u32)
            .get(mem)
    }

    /// `TkSetSyncVCallbackPtr(i, f)` (`0x00888790`): `i` below 0 sets the
    /// first; 3 and up set nothing.
    pub fn tk_set_sync_v_callback_ptr(mem: &mut impl Mem, i: i32, f: u32) {
        if i >= 3 {
            return;
        }
        #[allow(clippy::cast_sign_loss)]
        let p = SYNC_V_CALLBACK_PTRS.ptr(mem).at(i.max(0) as u32);
        p.set(mem, f);
    }

    /// `TkRegistBGThreadFunc(f)` (`0x00888740`): returns the old one.
    pub fn tk_regist_bg_thread_func(mem: &mut impl Mem, f: u32) -> u32 {
        let old = BG_THREAD_FUNC.get(mem);
        BG_THREAD_FUNC.set(mem, f);
        old
    }

    /// `TkGetTotalSyncVCounter()` (`0x00887e10`): a 64-bit result, high
    /// half 0.
    pub fn tk_get_total_sync_v_counter(mem: &impl Mem) -> u64 {
        u64::from(TOTAL_SYNC_V_COUNTER.get(mem))
    }

    /// `TkMenuMainGetOptionMenu(i)` (`0x008e2270`), unchecked.
    pub fn tk_menu_main_get_option_menu(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        let o = (i as u32).wrapping_mul(8);
        mem.read(MENU_MAIN_OPTIONS.ptr(mem).addr.wrapping_add(o))
    }

    /// `TkMenuMainGetOptionEnabled(i)` (`0x008e2290`), sign-extended.
    pub fn tk_menu_main_get_option_enabled(mem: &impl Mem, i: i32) -> i32 {
        #[allow(clippy::cast_sign_loss)]
        let o = (i as u32).wrapping_mul(8).wrapping_add(4);
        i32::from(mem.read::<i16>(MENU_MAIN_OPTIONS.ptr(mem).addr.wrapping_add(o)))
    }

    /// `TkMenuRestartSelFileWindow(window)` (`0x008b5530`).
    pub fn tk_menu_restart_sel_file_window(mem: &mut impl Mem, window: u32) {
        if window != 0 {
            mem.write(window.wrapping_add(0x28), 1u32);
        }
    }

    /// `TkMenuMainClearSelectWindow(window)` (`0x008aa1e0`).
    pub fn tk_menu_main_clear_select_window(mem: &mut impl Mem, window: u32) {
        if window != 0 {
            mem.write(window.wrapping_add(0x45), 0u8);
            mem.write(window.wrapping_add(0x28), 0u32);
        }
    }

    /// `TkMenuRestartSubCmdWindow2()` (`0x008e49d0`).
    pub fn tk_menu_restart_sub_cmd_window2(mem: &mut impl Mem) {
        let w = MENU_SUB_CMD_WINDOW.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x45), 0u8);
            mem.write(w.wrapping_add(0x28), 1u32);
        }
    }

    /// `TkMenuCloseSubCmdWindow()` (`0x008e36f0`).
    pub fn tk_menu_close_sub_cmd_window(mem: &mut impl Mem) {
        let w = MENU_SUB_CMD_WINDOW.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x41), 1u8);
            MENU_SUB_CMD_WINDOW.set(mem, 0);
        }
    }

    /// `TkSetEvSaveStatus(p)` (`0x0088ea50`).
    pub fn tk_set_ev_save_status(mem: &mut impl Mem, p: u32) {
        mem.write(p.wrapping_add(0x2a), 1u8);
    }

    /// `TkMenuFlushDummyControler()` (`0x008ca490`): the pending input
    /// becomes current; pending goes back to released buttons and centred
    /// sticks (`0x80`).
    pub fn tk_menu_flush_dummy_controler(mem: &mut impl Mem) {
        let p = MENU_DUMMY_CONTROLER.ptr(mem).addr;
        let pending = mem.read::<u32>(p + 4);
        mem.write(p, pending);
        for o in [8, 0xa, 0xc] {
            let v = mem.read::<u8>(p + o + 1);
            mem.write(p + o, v);
        }
        let last = mem.read::<u8>(p + 0xf);
        mem.write(p + 4, 0u32);
        for o in [9, 0xb, 0xd] {
            mem.write(p + o, 0x80u8);
        }
        mem.write(p + 0xe, last);
        mem.write(p + 0xf, 0x80u8);
    }

    /// `TkCreateDECI2Header(out, a, b, c, d)` (`0x00885120`): an 8-byte
    /// header `{u16 a, u16 0, u16 b, u8 c, u8 d}`.
    pub fn tk_create_deci2_header(mem: &mut impl Mem, out: u32, a: u16, b: u16, c: u8, d: u8) {
        let [a0, a1] = a.to_le_bytes();
        let [b0, b1] = b.to_le_bytes();
        mem.write(out, [a0, a1, 0, 0, b0, b1, c, d]);
    }

    /// `TkMenuGetCommandOffset(list, enabled, a, b)` (`0x008d2dc0`).
    pub fn tk_menu_get_command_offset(
        mem: &impl Mem,
        list: u32,
        enabled: i32,
        a: u32,
        b: u32,
    ) -> i32 {
        if enabled == 0 {
            return 0;
        }
        command_offset(records(mem, list), a, b)
    }

    /// `TkMenuGetCommandMax(list, enabled, a, b)` (`0x008d2d80`): 8 when
    /// the list is disabled.
    pub fn tk_menu_get_command_max(mem: &impl Mem, list: u32, enabled: i32, a: u32, b: u32) -> i32 {
        if enabled == 0 {
            return 8;
        }
        command_count(records(mem, list), a, b)
    }

    /// The `(a, b)` of each record until the `0xff` one.
    fn records(mem: &impl Mem, list: u32) -> impl Iterator<Item = (u8, u8)> + '_ {
        let mut p = list;
        std::iter::from_fn(move || {
            let a = mem.read::<u8>(p);
            if a == 0xff {
                return None;
            }
            let b = mem.read::<u8>(p.wrapping_add(1));
            p = p.wrapping_add(4);
            Some((a, b))
        })
    }

    /// `TkCRC(len, data)` (`0x008b1450`): see [`crc`]; nothing read when
    /// `len` is 0 or less.
    pub fn tk_crc(mem: &impl Mem, len: i32, data: u32) -> u16 {
        #[allow(clippy::cast_sign_loss)]
        let mut v = vec![0u8; len.max(0) as usize];
        mem.read_bytes(data, &mut v);
        crc(&v)
    }

    /// `TkMenuDrawEnv()` (`0x008dcca0`): [`DRAW_ENV`] at the menu packet
    /// pointer `+0x10`; the pointer moves `0x60` on.
    pub fn tk_menu_draw_env(mem: &mut impl Mem) {
        let p = MENU_PACKET.get(mem);
        mem.write(p.wrapping_add(0x10), DRAW_ENV);
        MENU_PACKET.set(mem, p.wrapping_add(0x60));
    }

    /// `TkMn2GetSummonGrow()` (`0x008c1c20`): `MsGetRomSummonGrow` with a
    /// count it drops; returns the record.
    pub fn tk_mn2_get_summon_grow(mem: &mut impl Mem) -> u32 {
        ffx_kernel::bridge::first_record(mem, ffx_kernel::bridge::SUMMON_GROW, 0)
    }

    /// `SetSummonGrowMax()` (`0x008c1910`): when still 0, counts the summon
    /// growth records (8 bytes each) up to the first whose byte `+7` is
    /// set, storing the count after each one.
    pub fn set_summon_grow_max(mem: &mut impl Mem) {
        if MENU_SUMMON_GROW_MAX.get(mem) != 0 {
            return;
        }
        let record = tk_mn2_get_summon_grow(mem);
        let table = ffx_kernel::bridge::SUMMON_GROW.get(mem);
        #[allow(clippy::cast_sign_loss)] // counted down to 0 as unsigned
        let mut left = ffx_kernel::bridge::ms_get_excel_data_num(mem, table, 0) as u32;
        let mut p = record.wrapping_add(7);
        while left != 0 {
            if mem.read::<u8>(p) != 0 {
                return;
            }
            let n = MENU_SUMMON_GROW_MAX.get(mem).wrapping_add(1);
            MENU_SUMMON_GROW_MAX.set(mem, n);
            p = p.wrapping_add(8);
            left -= 1;
        }
    }

    /// Entry `i` of the byte list at `list` of `max` entries, clamped to it
    /// (an empty list reads the byte before it).
    fn from_index(mem: &impl Mem, list: Global<u8>, i: i32, max: u32) -> u8 {
        let i = if i < 0 {
            0
        } else {
            #[allow(clippy::cast_possible_wrap)]
            let end = (max as i32).wrapping_sub(1);
            i.min(end)
        };
        #[allow(clippy::cast_sign_loss)]
        mem.read(list.ptr(mem).addr.wrapping_add(i as u32))
    }

    /// The next entry of a list of `max`, back to the first after the last;
    /// `step` notes the direction (1).
    fn step_next(mem: &mut impl Mem, pos: Global<u32>, max: u32, step: Global<u8>) {
        #[allow(clippy::cast_possible_wrap)]
        let next = (pos.get(mem) as i32).wrapping_add(1);
        step.set(mem, 1);
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        pos.set(mem, if next >= max as i32 { 0 } else { next as u32 });
    }

    /// The previous entry, to the last before the first; `step` gets `0xff`.
    fn step_prev(mem: &mut impl Mem, pos: Global<u32>, max: u32, step: Global<u8>) {
        #[allow(clippy::cast_possible_wrap)]
        let prev = (pos.get(mem) as i32).wrapping_sub(1);
        #[allow(clippy::cast_sign_loss)]
        let p = if prev < 0 {
            max.wrapping_sub(1)
        } else {
            prev as u32
        };
        pos.set(mem, p);
        step.set(mem, 0xff);
    }

    /// Sound voice states: read, VAG read, shout read, standby (`u32`
    /// each; `TkSndIsReadVoice`, `TkSndIsReadVagVoice`,
    /// `TkSndIsReadShoutVoice`, `TkSndIsStandbyVoice` report nonzero).
    pub const SND_VOICE_FLAGS: Global<u32> = Global::new(0x0132_ff10);

    /// Whether sound voice flag `i` (see [`SND_VOICE_FLAGS`]) is set.
    pub fn tk_snd_flag(mem: &impl Mem, i: u32) -> u32 {
        u32::from(SND_VOICE_FLAGS.ptr(mem).at(i).get(mem) != 0)
    }

    /// The GS `CSR` copy (`TkGetGsRegisterCSR`).
    pub const GS_CSR: Global<u32> = Global::new(0x0133_01f4);
    /// The sync-V counter (`TkGetSyncVCounter`; counted with the total).
    pub const SYNC_V_COUNTER: Global<u32> = Global::new(0x0133_0200);
    /// The encounter output flag (`u8`; `TkGetOutputEncount`,
    /// `TkSetOutputEncount`).
    pub const OUTPUT_ENCOUNT: Global<u8> = Global::new(0x0133_0871);
    /// `TkMenuGetPlayerPartyMask`.
    pub const MENU_PLAYER_PARTY_MASK: Global<u32> = Global::new(0x0184_1c24);
    /// `TkMenuGetTimer` (`u8`).
    pub const MENU_TIMER: Global<u8> = Global::new(0x0184_0884);
    /// `TkMenuGetHelpRatio2`.
    pub const MENU_HELP_RATIO2: Global<u32> = Global::new(0x0184_0878);
    /// `TkMn2GetListMax2`.
    pub const MN2_LIST_MAX2: Global<u32> = Global::new(0x023c_c120);

    /// `TkMenuSetDummyControlerPending(buttons)` (`0x008ca560`): the
    /// dummy controller's pending buttons (`+4`).
    pub fn tk_menu_set_dummy_controler_pending(mem: &mut impl Mem, buttons: u32) {
        let p = MENU_DUMMY_CONTROLER.ptr(mem).addr;
        mem.write(p + 4, buttons);
    }

    /// The last change of aeon: 1 next, `0xff` previous.
    pub const MENU_SUMMON_STEP: Global<u8> = Global::new(0x0184_0887);

    /// `TkMenuGetSummonFromIndex(i)` (`0x008a9bc0`).
    pub fn tk_menu_get_summon_from_index(mem: &impl Mem, i: i32) -> u8 {
        let max = MENU_SUMMON_LIST_MAX.get(mem);
        from_index(mem, MENU_SUMMONS, i, max)
    }

    /// `TkMenuSetNextSummon()` (`0x008aaf70`).
    pub fn tk_menu_set_next_summon(mem: &mut impl Mem) {
        let max = MENU_SUMMON_LIST_MAX.get(mem);
        step_next(mem, MENU_CURRENT_SUMMON_POS, max, MENU_SUMMON_STEP);
    }

    /// `TkMenuSetPrevSummon()` (`0x008aafe0`).
    pub fn tk_menu_set_prev_summon(mem: &mut impl Mem) {
        let max = MENU_SUMMON_LIST_MAX.get(mem);
        step_prev(mem, MENU_CURRENT_SUMMON_POS, max, MENU_SUMMON_STEP);
    }

    /// `TkMenuSetNextPlayer()` (`0x008aaf10`): over
    /// [`tk_menu_get_player_list_max`].
    pub fn tk_menu_set_next_player(mem: &mut impl Mem) {
        let max = tk_menu_get_player_list_max(mem);
        step_next(mem, MENU_CURRENT_PLAYER_POS, max, MENU_PLAYER_STEP);
    }

    /// `TkMenuSetPrevPlayer()` (`0x008aafa0`).
    pub fn tk_menu_set_prev_player(mem: &mut impl Mem) {
        let max = tk_menu_get_player_list_max(mem);
        step_prev(mem, MENU_CURRENT_PLAYER_POS, max, MENU_PLAYER_STEP);
    }

    /// `TkMenuGetPlayerStep()` (`0x008a9840`), sign-extended.
    pub fn tk_menu_get_player_step(mem: &impl Mem) -> i32 {
        i32::from(MENU_PLAYER_STEP.get(mem).cast_signed())
    }

    /// `TkMenuGetSummonStep()` (`0x008a9bb0`), sign-extended.
    pub fn tk_menu_get_summon_step(mem: &impl Mem) -> i32 {
        i32::from(MENU_SUMMON_STEP.get(mem).cast_signed())
    }

    /// `TkMenuMainGetIgnoreJoinFlag()` (`0x008aa450`).
    pub fn tk_menu_main_get_ignore_join_flag(mem: &impl Mem) -> u32 {
        MENU_MAIN_IGNORE_JOIN.get(mem)
    }

    /// `TkMenuGetPlayerListMax()` (`0x008a9b30`): the second count while
    /// the ignore-join flag is set, else the first.
    pub fn tk_menu_get_player_list_max(mem: &impl Mem) -> u32 {
        if tk_menu_main_get_ignore_join_flag(mem) != 0 {
            MENU_PLAYER_LIST_MAX2.get(mem)
        } else {
            MENU_PLAYER_LIST_MAX.get(mem)
        }
    }

    /// `TkMenuGetPlayerFromIndex2(i)` (`0x008a9ad0`): the character at `i`
    /// of [`tk_menu_get_player_list_max`].
    pub fn tk_menu_get_player_from_index(mem: &impl Mem, i: i32) -> u8 {
        let max = tk_menu_get_player_list_max(mem);
        from_index(mem, MENU_PLAYERS, i, max)
    }

    /// `TkMenuGetPlayerFromIndex2_008a9b00(i)` (`0x008a9b00`): the same over
    /// `TkMenuGetPlayerListMax2`.
    pub fn tk_menu_get_player_from_index2(mem: &impl Mem, i: i32) -> u8 {
        let max = MENU_PLAYER_LIST_MAX2.get(mem);
        from_index(mem, MENU_PLAYERS, i, max)
    }

    /// `TkMenuSetNextPlayer2()` (`0x008aaf40`): the next character, back
    /// to the first after the last.
    pub fn tk_menu_set_next_player2(mem: &mut impl Mem) {
        let max = MENU_PLAYER_LIST_MAX2.get(mem);
        step_next(mem, MENU_CURRENT_PLAYER_POS, max, MENU_PLAYER_STEP);
    }

    /// `TkMenuSetPrevPlayer2()` (`0x008aafc0`): the previous character,
    /// to the last before the first.
    pub fn tk_menu_set_prev_player2(mem: &mut impl Mem) {
        let max = MENU_PLAYER_LIST_MAX2.get(mem);
        step_prev(mem, MENU_CURRENT_PLAYER_POS, max, MENU_PLAYER_STEP);
    }

    /// One pad's vibration off: its on byte and both channels' state.
    fn stop_vibrate(mem: &mut impl Mem, pad: i32) {
        #[allow(clippy::cast_sign_loss)]
        let p = VIBRATE_PADS
            .ptr(mem)
            .addr
            .wrapping_add(pad.clamp(0, 7) as u32 * VIBRATE_PAD);
        mem.write(p + 0x22, 0u8);
        for c in [p, p + 0x10] {
            mem.write(c + 4, 0u32);
            mem.write(c + 8, 0u16);
            mem.write(c + 0xa, 0u8);
        }
    }

    /// `TkStopVibrate(pad)` (`0x00876290`), `pad` clamped to 0..=7.
    pub fn tk_stop_vibrate(mem: &mut impl Mem, pad: i32) {
        stop_vibrate(mem, pad);
    }

    /// `TkStopVibrateAll()` (`0x008762f0`).
    pub fn tk_stop_vibrate_all(mem: &mut impl Mem) {
        for pad in 0..8 {
            stop_vibrate(mem, pad);
        }
    }

    /// `TkGetVibrateOn(pad)` (`0x00875de0`): a pad's on byte, `pad`
    /// clamped to 0..=7.
    pub fn tk_get_vibrate_on(mem: &impl Mem, pad: i32) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        let p = VIBRATE_PADS
            .ptr(mem)
            .addr
            .wrapping_add(pad.clamp(0, 7) as u32 * VIBRATE_PAD);
        mem.read(p + 0x22)
    }

    /// The default vibration curves `TkGetVibrateCurve` picks by kind.
    pub const VIBRATE_CURVES: [Global<u16>; 4] = [
        Global::new(0x00c5_2ad8),
        Global::new(0x00c5_2ae8),
        Global::new(0x00c5_2af8),
        Global::new(0x00c5_2b08),
    ];

    /// `TkGetVibrateCurve(kind, i, curve)` (`0x008763e0`): entry `i` of
    /// `curve`, or without one of the default for `kind` (2 and 3 the
    /// second, 5 the third, 6 the fourth, others the first).
    pub fn tk_get_vibrate_curve(mem: &impl Mem, kind: u32, i: i32, curve: u32) -> u16 {
        let curve = if curve == 0 {
            tk_get_vibrate_default_curve(mem, kind)
        } else {
            curve
        };
        #[allow(clippy::cast_sign_loss)]
        mem.read(curve.wrapping_add((i as u32).wrapping_mul(2)))
    }

    /// `TkMsGetRomItem(id, &out)` and its kin read a kernel record's text:
    /// the `u16` offset at `field` (or the next one, `+2` words on, in
    /// hiragana) from the segment's strings.
    fn kernel_text(mem: &mut impl Mem, table: Global<u32>, id: u32, field: u32) -> u32 {
        let hiragana = save::ms_get_save_config_hiragana(mem) != 0;
        let (record, strings) = ffx_kernel::bridge::rom_strings(mem, table, id);
        let at = record
            .wrapping_add(field)
            .wrapping_add(u32::from(hiragana) * 4);
        strings.wrapping_add(u32::from(mem.read::<u16>(at)))
    }

    /// `TkMsGetRomItem(id, &name)` (`0x008ab280`): an item's name, and
    /// its record (left in `eax` from `MsGetRomItem`; the item menus read
    /// its flags at `+0x1c`).
    pub fn tk_ms_get_rom_item(mem: &mut impl Mem, id: u32, out: u32) -> u32 {
        let name = kernel_text(mem, ffx_kernel::bridge::ITEM, id, 0);
        mem.write(out, name);
        ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::ITEM, id, 0)
    }

    /// `TkMenuGetItemHelp(id)` (`0x008a98f0`) and `TkMn2GetItemHelp`
    /// (`0x008c1b30`): an item's help text.
    pub fn tk_get_item_help(mem: &mut impl Mem, id: u32) -> u32 {
        kernel_text(mem, ffx_kernel::bridge::ITEM, id, 8)
    }

    /// `TkMn2GetCommandName(id)` (`0x008c1a20`): a command's name.
    pub fn tk_mn2_get_command_name(mem: &mut impl Mem, id: u32) -> u32 {
        kernel_text(mem, ffx_kernel::bridge::COMMAND, id, 0)
    }

    /// `TkMn2GetAbilityMes(id)` (`0x008c19a0`): an auto-ability's name
    /// (`MsGetRomAbility`, text offset `+0`, `+4` in hiragana).
    pub fn tk_mn2_get_ability_mes(mem: &mut impl Mem, id: u32) -> u32 {
        kernel_text(mem, ffx_kernel::bridge::A_ABILITY, id, 0)
    }

    /// `TkMn2GetAbilityHelp(id)` (`0x008c1960`): an auto-ability's help
    /// text (`+8`, `+0xc` in hiragana).
    pub fn tk_mn2_get_ability_help(mem: &mut impl Mem, id: u32) -> u32 {
        kernel_text(mem, ffx_kernel::bridge::A_ABILITY, id, 8)
    }

    /// `TkMenuGetCommandHelp(id)` (`0x008dde10`) and `TkMn2GetCommandHelp`
    /// (`0x008c19e0`): a command's help text.
    pub fn tk_get_command_help(mem: &mut impl Mem, id: u32) -> u32 {
        kernel_text(mem, ffx_kernel::bridge::COMMAND, id, 8)
    }

    /// The save block's event fields (`MsGetSaveEventAddress`).
    fn event(mem: &impl Mem) -> u32 {
        save::ms_get_save_event_address(mem)
    }

    /// `TkGetNowEventJumpId()` (`0x0088d660`).
    pub fn tk_get_now_event_jump_id(mem: &impl Mem) -> u16 {
        mem.read(event(mem))
    }

    /// `TkGetNowEventJumpMapNo()` (`0x0088d690`).
    pub fn tk_get_now_event_jump_map_no(mem: &impl Mem) -> u16 {
        mem.read(event(mem) + 4)
    }

    /// `TkGetNowEventJumpMapIdNo()` (`0x0088d680`).
    pub fn tk_get_now_event_jump_map_id_no(mem: &impl Mem) -> u16 {
        mem.read(event(mem) + 8)
    }

    /// `TkGetNowEventJumpIndex()` (`0x0088d670`).
    pub fn tk_get_now_event_jump_index(mem: &impl Mem) -> u8 {
        mem.read(event(mem) + 0xc)
    }

    /// `TkSetEventJumpMapNo(map, id)` (`0x0088eaf0`): the current map and
    /// entrance become the previous ones (or the saved ones at `+0xc6`,
    /// `+0xca` when `+0x2a` is set, which it clears).
    pub fn tk_set_event_jump_map_no(mem: &mut impl Mem, map: u16, id: u16) {
        let e = event(mem);
        let restore = mem.read::<u8>(e + 0x2a) != 0;
        let (prev_map, prev_id) = if restore { (0xc6, 0xca) } else { (4, 8) };
        let v = mem.read::<u16>(e + prev_map);
        mem.write(e + 6, v);
        let v = mem.read::<u16>(e + prev_id);
        mem.write(e + 0xa, v);
        mem.write(e + 4, map);
        mem.write(e + 0x2a, 0u8);
        mem.write(e + 8, id);
    }

    /// `TkMenuGetHP(chr)` (`0x008a98c0`) and its siblings: a `u32` of the
    /// character's `ply_save` record (`+0x1c` HP, `+0x20` MP, `+0x24` max
    /// HP, `+0x28` max MP).
    pub fn tk_menu_get_ply(mem: &impl Mem, chr: u32, field: u32) -> u32 {
        let ply = save::ms_get_save_player_ptr(mem, chr);
        mem.read(ply.wrapping_add(field))
    }

    /// `TkCheckPlayVoice()` (`0x00872340`): menu setting bit 8.
    pub fn tk_check_play_voice(mem: &impl Mem) -> u32 {
        (mem.read::<u32>(save::ms_get_save_menu(mem)) >> 8) & 1
    }

    /// `TkMsImportantGet(id)` (`0x008ab2c0`): `MsImportantGet`.
    pub fn tk_ms_important_get(mem: &impl Mem, id: u32) -> u32 {
        save::ms_important_get(mem, id)
    }

    /// `TkMn2GetSummonGrowMax()` (`0x008c1c40`).
    pub fn tk_mn2_get_summon_grow_max(mem: &mut impl Mem) -> u32 {
        set_summon_grow_max(mem);
        MENU_SUMMON_GROW_MAX.get(mem)
    }

    /// `TkCalcImportantListMax()` (`0x008c6790`): key items `0xa000` on
    /// until one whose byte `+0x10` is `0xff`; stored and returned.
    pub fn tk_calc_important_list_max(mem: &mut impl Mem) -> u32 {
        let mut n = 0u32;
        loop {
            let r = save::ms_important_ptr(mem, 0xa000u32.wrapping_add(n), 0);
            if mem.read::<u8>(r.wrapping_add(0x10)) == 0xff {
                break;
            }
            n = n.wrapping_add(1);
        }
        MENU_IMPORTANT_LIST_MAX.set(mem, n);
        n
    }

    /// `TkChSetBattleChrNum(work)` (`0x00872160`): `Ch_SetBattleChrNum` on
    /// the worker's character (`+0x9c`) with its number (`+0x40`, -1 as
    /// `0xff`).
    pub fn tk_ch_set_battle_chr_num(mem: &mut impl Mem, work: u32) {
        let chr = mem.read::<u32>(work.wrapping_add(0x9c));
        if chr == 0 {
            return;
        }
        let n = mem.read::<u32>(work.wrapping_add(0x40));
        let n = if n == u32::MAX { 0xff } else { n };
        ffx_chara::chr::bridge::ch_set_battle_chr_num(mem, chr, n);
    }

    /// `TkMn2SaveFontShade()` (`0x008c2330`): the four shades (stored
    /// downwards from `0x023cc470`, 8 bytes apart) copied to the words
    /// after them.
    pub fn tk_mn2_save_font_shade(mem: &mut impl Mem) {
        let base = MN2_FONT_SHADE.ptr(mem).addr;
        for o in [0x18, 0x10, 8, 0] {
            let v = mem.read::<u32>(base + o);
            mem.write(base + o + 4, v);
        }
    }

    /// `TkMn2RestoreFontShade()` (`0x008c2300`): back from those copies.
    pub fn tk_mn2_restore_font_shade(mem: &mut impl Mem) {
        let base = MN2_FONT_SHADE.ptr(mem).addr;
        for o in [0x18, 0x10, 8, 0] {
            let v = mem.read::<u32>(base + o + 4);
            mem.write(base + o, v);
        }
    }

    /// The status screen background's fade step (`i16`).
    pub const MN2_STATUS_BG_STEP: Global<i16> = Global::new(0x0186_a9fc);
    /// The status screen background's fade, 0..=0x1000 (`i16`).
    pub const MN2_STATUS_BG: Global<i16> = Global::new(0x0186_aa00);

    /// `TkMn2InitStatusBG()` (`0x008d4ed0`).
    pub fn tk_mn2_init_status_bg(mem: &mut impl Mem) {
        MN2_STATUS_BG_STEP.set(mem, 0);
        MN2_STATUS_BG.set(mem, 0);
    }

    /// `TMn2SetStatusBGDiff(step)` (`0x008d2660`).
    pub fn tmn2_set_status_bg_diff(mem: &mut impl Mem, step: i16) {
        MN2_STATUS_BG_STEP.set(mem, step);
    }

    /// `TkMn2StepStatusBG()` (`0x008d3280`): the fade moved by the step
    /// (16-bit), kept in 0..=0x1000.
    pub fn tk_mn2_step_status_bg(mem: &mut impl Mem) {
        let v = MN2_STATUS_BG
            .get(mem)
            .wrapping_add(MN2_STATUS_BG_STEP.get(mem))
            .clamp(0, 0x1000);
        MN2_STATUS_BG.set(mem, v);
    }

    /// `TkMn2GetStatusBG()` (`0x008d4910`), sign-extended.
    pub fn tk_mn2_get_status_bg(mem: &impl Mem) -> i32 {
        i32::from(MN2_STATUS_BG.get(mem))
    }

    /// The font width table `TkGetStrWidth` reads (a byte per character
    /// code, two-byte codes too).
    pub const FONT_WIDTHS: Global<u8> = Global::new(0x023c_bf6c);

    /// `TkGetStrWidth(proportional, s)` (`0x0088ee70`): the width of a
    /// string of one- and two-byte (lead `0x80` up) codes in 1/16: each
    /// character its table width + 2 when `proportional`, else 9 (one
    /// byte) or 13 (two).
    pub fn tk_get_str_width(mem: &impl Mem, proportional: i32, s: u32) -> i32 {
        let mut p = s;
        let mut total = 0i32;
        loop {
            let mut c = u32::from(mem.read::<u8>(p));
            p = p.wrapping_add(1);
            if c == 0 {
                return total;
            }
            if c >= 0x80 {
                c = (c << 8) + u32::from(mem.read::<u8>(p));
                p = p.wrapping_add(1);
            }
            total = total.wrapping_add(tk_get_char_width(mem, proportional, c) << 4);
        }
    }

    /// `TkMenuGetTableEntry(i, table)` (`0x008dce10`): a table of a `u32`
    /// count and that many offsets from its start; entry `i`, or 0 past
    /// the end.
    pub fn tk_menu_get_table_entry(mem: &impl Mem, i: u32, table: u32) -> u32 {
        if i >= mem.read::<u32>(table) {
            return 0;
        }
        let at = table.wrapping_add(4).wrapping_add(i.wrapping_mul(4));
        mem.read::<u32>(at).wrapping_add(table)
    }

    /// `TkGetControlerWork(a, b)` (`0x00888ec0`): the 0x100-byte work of
    /// pad `a + b`.
    pub fn tk_get_controler_work(mem: &impl Mem, a: i32, b: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        let i = a.wrapping_add(b) as u32;
        mem.rebase(0x0133_0288).wrapping_add(i.wrapping_mul(0x100))
    }

    /// `TkGetControlerWork2(a, b)` (`0x00888e10`): the 0x20-byte record of
    /// pad `a + b` after the button maps.
    pub fn tk_get_controler_work2(mem: &impl Mem, a: i32, b: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        let i = a.wrapping_add(b) as u32;
        mem.rebase(0x0133_04c0).wrapping_add(i.wrapping_mul(0x20))
    }

    /// `TkGetCharWidth(proportional, c)` (`0x00890a00`): one character's
    /// width as [`tk_get_str_width`] counts it (before the ×16).
    pub fn tk_get_char_width(mem: &impl Mem, proportional: i32, c: u32) -> i32 {
        if proportional == 0 {
            if c.cast_signed() >= 0x100 { 13 } else { 9 }
        } else {
            i32::from(mem.read::<u8>(FONT_WIDTHS.ptr(mem).addr.wrapping_add(c))) + 2
        }
    }

    /// `TkGetVibrateDefaultCurve(kind)` (`0x00876390`): 2 and 3 the second
    /// default curve, 5 the third, 6 the fourth, others the first.
    pub fn tk_get_vibrate_default_curve(mem: &impl Mem, kind: u32) -> u32 {
        let k = match kind {
            2 | 3 => 1,
            5 => 2,
            6 => 3,
            _ => 0,
        };
        VIBRATE_CURVES[k].ptr(mem).addr
    }

    /// `TkGetVibratePad(pad)` (`0x00875e50`): pad `pad`'s record, clamped
    /// to 0..=7.
    pub fn tk_get_vibrate_pad(mem: &impl Mem, pad: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        VIBRATE_PADS
            .ptr(mem)
            .addr
            .wrapping_add(pad.clamp(0, 7) as u32 * VIBRATE_PAD)
    }

    /// `TkGetSyncVCallbackPtr(i)` (`0x00888650`): `i` clamped to 0..=2.
    pub fn tk_get_sync_v_callback_ptr(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        SYNC_V_CALLBACK_PTRS
            .ptr(mem)
            .at(i.clamp(0, 2) as u32)
            .get(mem)
    }

    /// `TkSetPadButtonMap(a, b, i, to)` (`0x008894f0`): entry `i` of row
    /// `a + b` of the button maps.
    pub fn tk_set_pad_button_map(mem: &mut impl Mem, a: i32, b: i32, i: i32, to: u8) {
        #[allow(clippy::cast_sign_loss)]
        let at = PAD_BUTTON_MAP
            .ptr(mem)
            .addr
            .wrapping_add((a.wrapping_add(b) as u32).wrapping_mul(16))
            .wrapping_add(i as u32);
        mem.write(at, to);
    }

    /// The buttons `TkFindPadButton` looks up (`0x00c54e28`, bytes).
    pub const PAD_BUTTON_CODES: Global<u8> = Global::new(0x00c5_4e28);

    /// `TkFindPadButton(a, b, k)` (`0x00889ac0`): where in row `a + b` of
    /// the button maps the code `PAD_BUTTON_CODES[k]` is, -1 if nowhere.
    pub fn tk_find_pad_button(mem: &impl Mem, a: i32, b: i32, k: i32) -> i32 {
        #[allow(clippy::cast_sign_loss)]
        let code = mem.read::<u8>(PAD_BUTTON_CODES.ptr(mem).addr.wrapping_add(k as u32));
        #[allow(clippy::cast_sign_loss)]
        let row: [u8; 16] = PAD_BUTTON_MAP
            .ptr(mem)
            .at(a.wrapping_add(b) as u32)
            .get(mem);
        row.iter()
            .position(|&c| c == code)
            .map_or(-1, |i| i32::try_from(i).unwrap_or(-1))
    }

    /// `TkSetControlerActuatorMode(a, b, mode)` (`0x008894d0`): byte
    /// `+0x9d` of pad `a + b`'s work; returns the work (left in `eax`).
    pub fn tk_set_controler_actuator_mode(mem: &mut impl Mem, a: i32, b: i32, mode: u8) -> u32 {
        let work = tk_get_controler_work(mem, a, b);
        mem.write(work.wrapping_add(0x9d), mode);
        work
    }

    /// `TkUpdateControlerActuator(work)` (`0x00889410`): with the actuator
    /// mode on (`+0x9d`), the two motor bytes `+0xf8`/`+0xf9` are `+0xfe`
    /// and `+0xff` shifted right by `+0xf6`/`+0xf7`; off, both 0.
    pub fn tk_update_controler_actuator(mem: &mut impl Mem, work: u32) {
        let at = |o: u32| work.wrapping_add(o);
        if mem.read::<u8>(at(0x9d)) == 0 {
            mem.write(at(0xf8), 0u16);
            return;
        }
        for k in 0..2 {
            let shift = u32::from(mem.read::<u8>(at(0xf6 + k))) & 0x1f;
            let v = u32::from(mem.read::<u8>(at(0xfe + k))) >> shift;
            #[allow(clippy::cast_possible_truncation)]
            mem.write(at(0xf8 + k), v as u8);
        }
    }

    /// `TkGetControlerFrame(work, i)` (`0x00888b60`): input frame `i` after
    /// the newest (`+0x9c`) of a pad's 4 × `0x20` ring at `work`.
    pub fn tk_get_controler_frame(mem: &impl Mem, work: u32, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        let slot = u32::from(mem.read::<u8>(work.wrapping_add(0x9c))).wrapping_add(i as u32) & 3;
        work.wrapping_add(slot * 0x20)
    }

    /// The second debug font position, ×16 (`TkSetDebugFontLocate2`).
    pub const DEBUG_FONT_POS2: Global<[i32; 2]> = Global::new(0x0133_c8dc);

    /// `TkSetDebugFontLocate2(x, y)` (`0x008904e0`).
    pub fn tk_set_debug_font_locate2(mem: &mut impl Mem, x: i32, y: i32) {
        DEBUG_FONT_POS2.set(mem, [x.wrapping_shl(4), y.wrapping_shl(4)]);
    }

    /// `TkMenuClearDummyControler()` (`0x008ca540`): its buttons, pending
    /// buttons and flag.
    pub fn tk_menu_clear_dummy_controler(mem: &mut impl Mem) {
        let p = MENU_DUMMY_CONTROLER.ptr(mem).addr;
        mem.write(p, [0u32, 0]);
        MENU_DUMMY_CONTROLER_FLAG.set(mem, 0);
    }

    /// `TkSetDebugDrawFlag(on)`'s byte.
    pub const DEBUG_DRAW_FLAG: Global<u8> = Global::new(0x0133_0872);
    /// `TkIsDeci2Send`'s byte.
    pub const DECI2_SEND: Global<i8> = Global::new(0x0132_f580);

    /// `TkParseLast2Digits(s)` (`0x00888280`): the last two characters of a
    /// string as a decimal number (signed bytes, no checks).
    pub fn tk_parse_last_2_digits(mem: &impl Mem, s: u32) -> i32 {
        let mut end = s;
        while mem.read::<u8>(end) != 0 {
            end = end.wrapping_add(1);
        }
        let c = |k: u32| i32::from(mem.read::<i8>(end.wrapping_sub(k)));
        (c(2) * 10).wrapping_add(c(1)) - 0x210
    }

    /// `TkBackupEventJump(event)` (`0x0088e800`): the first six `u16` of the
    /// event block to `+0xc0`, bytes `+0xc` and `+0xd` widened to `+0xcc`
    /// and `+0xce` (what `TkSetEventJumpMapNo` and the restores read back).
    pub fn tk_backup_event_jump(mem: &mut impl Mem, event: u32) {
        let at = |o: u32| event.wrapping_add(o);
        for o in (0..0xc).step_by(2) {
            let v = mem.read::<u16>(at(o));
            mem.write(at(0xc0 + o), v);
        }
        for o in [0xc, 0xd] {
            let v = u16::from(mem.read::<u8>(at(o)));
            mem.write(at(0xc0 + 2 * o - 0xc), v);
        }
    }

    /// `TkRestoreEventJumpPrev(event)` (`0x0088e7c0`): the previous map and
    /// entrance (`+6`, `+0xa`) from the backup.
    pub fn tk_restore_event_jump_prev(mem: &mut impl Mem, event: u32) {
        let at = |o: u32| event.wrapping_add(o);
        let v = mem.read::<u16>(at(0xc6));
        mem.write(at(6), v);
        let v = mem.read::<u16>(at(0xca));
        mem.write(at(0xa), v);
    }

    /// `TkRestoreEventJumpMapIndex(event)` (`0x0088e7e0`): `+2` and the
    /// byte `+0xd` from the backup.
    pub fn tk_restore_event_jump_map_index(mem: &mut impl Mem, event: u32) {
        let at = |o: u32| event.wrapping_add(o);
        let v = mem.read::<u16>(at(0xc2));
        mem.write(at(2), v);
        let v = mem.read::<u8>(at(0xce));
        mem.write(at(0xd), v);
    }

    /// `TkMenuGetSaveFileInfo(i)` (`0x008b4e40`): the `0x40`-byte record of
    /// save file `i`, clamped to the 99 of them.
    pub fn tk_menu_get_save_file_info(mem: &impl Mem, i: i32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        mem.rebase(0x0159_ec70)
            .wrapping_add(i.clamp(0, 0x62) as u32 * 0x40)
    }

    /// `TkMn2SeqGetPos(name, &pos)` (`0x008c2b50`): `TOSeqGetPoint` in the
    /// menu's scene data 1, into two `i16`.
    pub fn tk_mn2_seq_get_pos(mem: &mut impl Mem, name: u32, out: u32) {
        ffx_system::scene::bridge::get_point(
            mem,
            1,
            name,
            [out, out.wrapping_add(2)],
            crate::scene::bridge::tk_menu_get_param_handle,
            crate::scene::bridge::tk_menu_get_param_offset,
        );
    }

    /// `TkMn2SeqGetRect(name, &rect)` (`0x008c2b90`): `TOSeqGetRect`, into
    /// four `i16`.
    pub fn tk_mn2_seq_get_rect(mem: &mut impl Mem, name: u32, out: u32) {
        let outs = [0, 2, 4, 6].map(|o| out.wrapping_add(o));
        ffx_system::scene::bridge::get_rect(
            mem,
            1,
            name,
            outs,
            crate::scene::bridge::tk_menu_get_param_handle,
            crate::scene::bridge::tk_menu_get_param_offset,
        );
    }

    /// A menu window: `+0x10` cleared on some closes, `+0x28` its state (0
    /// idle, 1 restart, 6 select), `+0x2c` and `+0x48` the cursor, `+0x41`
    /// the close request, `+0x45` busy, `+0x52`/`+0x54` fade and step.
    ///
    /// The `Close` helpers: request the close, optionally zero `+0x10`, and
    /// forget the window; they return what was there (left in `eax`).
    pub fn close_window(
        mem: &mut impl Mem,
        global: Global<u32>,
        zero_10: bool,
        forget: bool,
    ) -> u32 {
        let w = global.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x41), 1u8);
            if zero_10 {
                mem.write(w.wrapping_add(0x10), 0u32);
            }
            if forget {
                global.set(mem, 0);
            }
        }
        w
    }

    /// The `Clear` helpers: busy off and state idle; the window back.
    pub fn clear_window(mem: &mut impl Mem, global: Global<u32>) -> u32 {
        let w = global.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x45), 0u8);
            mem.write(w.wrapping_add(0x28), 0u32);
        }
        w
    }

    /// `TkMenuRestartWindow(window)` (`0x008d1b90`): state 1, busy off.
    pub fn tk_menu_restart_window(mem: &mut impl Mem, window: u32) -> u32 {
        if window != 0 {
            mem.write(window.wrapping_add(0x28), 1u32);
            mem.write(window.wrapping_add(0x45), 0u8);
        }
        window
    }

    /// The save/load menu's window (`TkMenuCtrlSaveLoad`).
    pub const SAVE_LOAD_WINDOW: Global<u32> = Global::new(0x0186_6254);
    /// The save/load menu's second window.
    pub const SAVE_LOAD_WINDOW2: Global<u32> = Global::new(0x0186_6258);
    /// The item menu's fourth window.
    pub const ITEM_WINDOW4: Global<u32> = Global::new(0x0186_a37c);
    /// The item menu's window (`TkMenuCtrlItem`).
    pub const ITEM_WINDOW: Global<u32> = Global::new(0x0186_a394);
    /// The item menu's third window.
    pub const ITEM_WINDOW3: Global<u32> = Global::new(0x0186_a398);
    /// The item menu's second window.
    pub const ITEM_WINDOW2: Global<u32> = Global::new(0x0186_a39c);
    /// The aeon menu's window (`TkMenuCtrlSummon`).
    pub const SUMMON_WINDOW: Global<u32> = Global::new(0x0186_a5a0);
    /// The aeon growth window (`TkMn2OpenSummonGrowWindow`).
    pub const SUMMON_GROW_WINDOW: Global<u32> = Global::new(0x0186_a5a8);
    /// The aeon menu's second window.
    pub const SUMMON_WINDOW2: Global<u32> = Global::new(0x0186_a5ac);
    /// The equipment menu's window (`TkMenuCtrlEquip`).
    pub const EQUIP_WINDOW: Global<u32> = Global::new(0x0186_a61c);
    /// The overdrive menu's window (`TkMenuCtrlLimit`).
    pub const LIMIT_WINDOW: Global<u32> = Global::new(0x0186_a714);
    /// The overdrive menu's second window.
    pub const LIMIT_WINDOW2: Global<u32> = Global::new(0x0186_a718);
    /// The customize menu's window (`TkMenuCtrlKaizou`).
    pub const KAIZOU_WINDOW: Global<u32> = Global::new(0x0186_aa30);
    /// The customize menu's second window.
    pub const KAIZOU_WINDOW2: Global<u32> = Global::new(0x0186_aa34);
    /// The weapon shop's third window.
    pub const WSHOP_WINDOW3: Global<u32> = Global::new(0x0186_ac30);
    /// The weapon shop's window (`TkMenuCtrlWShop`).
    pub const WSHOP_WINDOW: Global<u32> = Global::new(0x0186_ac3c);
    /// The weapon shop's second window.
    pub const WSHOP_WINDOW2: Global<u32> = Global::new(0x0186_ac40);
    /// The second sub-command window (`TkMenuCloseSubCmdWindow1`).
    pub const MENU_SUB_CMD_WINDOW1: Global<u32> = Global::new(0x023c_c124);

    /// `TkMenuRestartSubCmdWindow1()` (`0x008e49f0`).
    pub fn tk_menu_restart_sub_cmd_window1(mem: &mut impl Mem) -> u32 {
        let w = MENU_SUB_CMD_WINDOW1.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x45), 0u8);
            mem.write(w.wrapping_add(0x28), 1u32);
        }
        w
    }

    /// `TkMenuSetSubCmdWindowCursor(i)` (`0x008e3b80`): busy, the cursor at
    /// `i` (`+0x2c`, `+0x48`), state 6.
    pub fn tk_menu_set_sub_cmd_window_cursor(mem: &mut impl Mem, i: u16) -> u32 {
        let w = MENU_SUB_CMD_WINDOW.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x45), 1u8);
            mem.write(w.wrapping_add(0x2c), i);
            mem.write(w.wrapping_add(0x48), i);
            mem.write(w.wrapping_add(0x28), 6u32);
        }
        w
    }

    /// `TkMenuWindowStepFade(window)` (`0x008ab1f0`): `+0x52 += +0x54`
    /// (16-bit), kept in 0..=0x1000; returns `0x1000` (left in `eax`).
    pub fn tk_menu_window_step_fade(mem: &mut impl Mem, window: u32) -> u32 {
        let at = |o: u32| window.wrapping_add(o);
        let v = mem
            .read::<i16>(at(0x52))
            .wrapping_add(mem.read::<i16>(at(0x54)))
            .clamp(0, 0x1000);
        mem.write(at(0x52), v);
        0x1000
    }

    /// `TkMn2WindowStepFade(window)` (`0x008d26d0`): the same when the step
    /// is not 0, the result at most 0x1000 then at least 0; returns the
    /// window.
    pub fn tk_mn2_window_step_fade(mem: &mut impl Mem, window: u32) -> u32 {
        let at = |o: u32| window.wrapping_add(o);
        let step = mem.read::<i16>(at(0x54));
        if step != 0 {
            let v = mem
                .read::<i16>(at(0x52))
                .wrapping_add(step)
                .clamp(0, 0x1000);
            mem.write(at(0x52), v);
        }
        window
    }

    /// `TkMenuWindowFadedOut(window)` (`0x008d3200`): whether `+0x52` is 0.
    pub fn tk_menu_window_faded_out(mem: &impl Mem, window: u32) -> bool {
        mem.read::<i16>(window.wrapping_add(0x52)) == 0
    }

    /// The menu's item list (`TkMenuItemRefresh`): `0x200` entries
    /// `{u16 id, u8 count, u8 usable}`.
    pub const MENU_ITEM_LIST: Global<u32> = Global::new(0x0159_7770);
    /// Item counts by the id's low byte, `0x200` `u16` (cleared in full).
    pub const MENU_ITEM_COUNTS: Global<u16> = Global::new(0x0159_7370);
    /// Entries in [`MENU_ITEM_LIST`].
    pub const MENU_ITEM_LIST_COUNT: Global<u32> = Global::new(0x0186_a384);
    /// The first usable item's inventory slot (`i16`, -1 for none).
    pub const MENU_ITEM_FIRST_USABLE: Global<i16> = Global::new(0x0186_a3b0);

    /// `TkMenuItemRefresh(mode)` (`0x008c8e60`): the item list from the
    /// inventory. Modes 0 and 4 list every slot (empty ones leave their
    /// entry zero) and mark the usable items (kernel flags `+0x1c` bit 0;
    /// `+0x22` bit 7 needs bit 6 or an aeon); mode 4 counts every item as
    /// usable for the first-usable slot. Mode 6 lists only the items with
    /// an ability map id (`+0x5d` not `0xff`). Other modes list nothing.
    pub fn tk_menu_item_refresh(mem: &mut impl Mem, mode: i32) {
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let counts = MENU_ITEM_COUNTS.ptr(mem).addr;
        mem.write(list, [0u32; 0x200]);
        mem.write(counts, [0u16; 0x200]);
        let ids = ffx_battle::save::bridge::ITEM_IDS.ptr(mem);
        let nums = ffx_battle::save::bridge::ITEM_COUNTS.ptr(mem);
        MENU_ITEM_LIST_COUNT.set(mem, 0);
        MENU_ITEM_FIRST_USABLE.set(mem, -1);
        let mut entry = list;
        for slot in 0..ffx_battle::save::ITEM_SLOTS {
            let id = ids.at(slot).get(mem);
            let n = nums.at(slot).get(mem);
            let rec = ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::ITEM, u32::from(id), 0);
            let usable = match mode {
                0 | 4 => {
                    if n == 0 {
                        entry = entry.wrapping_add(4);
                        continue;
                    }
                    let usable = mode == 4 || item_usable(mem, rec);
                    if usable && MENU_ITEM_FIRST_USABLE.get(mem) < 0 {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                        MENU_ITEM_FIRST_USABLE.set(mem, slot as i16);
                    }
                    usable
                }
                6 if n != 0 && mem.read::<u8>(rec.wrapping_add(0x5d)) != 0xff => false,
                _ => continue,
            };
            mem.write(entry, id);
            mem.write(entry.wrapping_add(2), n);
            mem.write(entry.wrapping_add(3), u8::from(usable));
            mem.write(counts.wrapping_add(2 * u32::from(id & 0xff)), u16::from(n));
            entry = entry.wrapping_add(4);
        }
        MENU_ITEM_LIST_COUNT.set(mem, entry.wrapping_sub(list) >> 2);
    }

    /// An item's menu use: kernel flag `+0x1c` bit 0; with `+0x22` bit 7
    /// also bit 6 or an aeon (`TkMenuSummonEnableMask`).
    fn item_usable(mem: &impl Mem, rec: u32) -> bool {
        if mem.read::<u8>(rec.wrapping_add(0x1c)) & 1 == 0 {
            return false;
        }
        let f = mem.read::<u8>(rec.wrapping_add(0x22));
        f & 0x80 == 0 || f & 0x40 != 0 || MENU_SUMMON_ENABLE_MASK.get(mem) != 0
    }

    /// The key items by their list slot (`TkMenuItemImportantRefresh`), `u16`
    /// ids.
    pub const MENU_IMPORTANT_LIST: Global<u16> = Global::new(0x0159_6f70);

    /// `TkMenuItemImportantRefresh()` (`0x008c8ba0`): the key items owned,
    /// each at its list slot (kernel byte `+0x13`); the counter becomes the
    /// highest slot + 1.
    pub fn tk_menu_item_important_refresh(mem: &mut impl Mem) {
        let n = MENU_IMPORTANT_LIST_MAX.get(mem).cast_signed();
        let list = MENU_IMPORTANT_LIST.ptr(mem);
        for k in 0..n.max(0).cast_unsigned() {
            list.at(k).set(mem, 0);
        }
        MENU_IMPORTANT_COUNTER.set(mem, 0);
        for k in 0..n.max(0).cast_unsigned() {
            let id = 0xa000u32.wrapping_add(k);
            if save::ms_important_get(mem, id) == 0 {
                continue;
            }
            let rec = save::ms_important_ptr(mem, id, 0);
            let slot = u32::from(mem.read::<u8>(rec.wrapping_add(0x13)));
            #[allow(clippy::cast_possible_truncation)]
            list.at(slot).set(mem, id as u16);
            let top = slot + 1;
            if MENU_IMPORTANT_COUNTER.get(mem).cast_signed() < top.cast_signed() {
                MENU_IMPORTANT_COUNTER.set(mem, top);
            }
        }
    }

    /// `TkGetAbilityMapIdFromItemId(id)` (`0x008c67e0`): an item's sphere
    /// grid ability map id (kernel byte `+0x5d`).
    pub fn tk_get_ability_map_id_from_item_id(mem: &mut impl Mem, id: u32) -> u8 {
        let rec = ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::ITEM, id, 0);
        mem.read(rec.wrapping_add(0x5d))
    }

    /// `Cmd_HealAmount(command, chr)` (`0x008c6920`): what a healing
    /// command restores by its formula (`+0x28`) and power (`+0x2a`): 6
    /// `power × 50`, 7 `(stat + power) / 2 × power` with the character's
    /// stat `+0x31`, 8 `max HP × power / 16`; 0 otherwise.
    pub fn cmd_heal_amount(mem: &mut impl Mem, command: u32, chr: u32) -> i32 {
        let rec = ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::COMMAND, command, 0);
        let power = i32::from(mem.read::<u8>(rec.wrapping_add(0x2a)));
        match mem.read::<u8>(rec.wrapping_add(0x28)) {
            6 => power * 0x32,
            7 => {
                let ply = save::ms_get_save_player_ptr(mem, chr);
                let stat = i32::from(mem.read::<u8>(ply.wrapping_add(0x31)));
                #[allow(clippy::manual_midpoint)] // the original divides the sum
                ((stat + power) / 2).wrapping_mul(power)
            }
            8 => {
                tk_menu_get_ply(mem, chr, 0x24)
                    .cast_signed()
                    .wrapping_mul(power)
                    / 16
            }
            _ => 0,
        }
    }

    // ---- round 4: Tk library state, party fields, menu slots, list
    // callbacks, pad work ----

    /// The last sound kick id (`u8`, next to [`SND_KICK_FLAG`]).
    pub const SND_LAST_KICK_ID: Global<u8> = Global::new(0x0133_0095);
    /// The foot-step kick stay value (`TkSndSetFootKickStay`).
    pub const SND_FOOT_KICK_STAY: Global<u32> = Global::new(0x0132_ff4c);
    /// The third sync-V flag (`TkGetSoundSyncVFlag3`).
    pub const SOUND_SYNC_V_FLAG3: Global<u32> = Global::new(0x0133_01ec);
    /// The memory card debug lock (`TkMcSetDebugLock`).
    pub const MC_DEBUG_LOCK: Global<u32> = Global::new(0x0133_0238);
    /// The memory card load positions, one `u32` per slot.
    pub const MC_LOAD_FILE_POS: Global<u32> = Global::new(0x0133_023c);
    /// The common vibration curve (`TkGetCommonVibrate` returns its address).
    pub const COMMON_VIBRATE: Global<u8> = Global::new(0x0133_88c0);
    /// The HDD error flag (`u8`).
    pub const HDD_ERROR_FLAG: Global<u8> = Global::new(0x0133_0873);
    /// The start map's event id and index (`TkSetStartMapIndex`).
    pub const START_MAP_EVENT: Global<[u32; 2]> = Global::new(0x0133_085c);
    /// The local start map's event id and index (`TkSetLocalStartMapIndex`).
    pub const LOCAL_START_MAP_EVENT: Global<[u32; 2]> = Global::new(0x0133_0864);
    /// The menu slots: 25 window pointers (`TkMenuMainRegistMenu`).
    pub const MENU_SLOTS: Global<u32> = Global::new(0x0184_0888);
    /// The help window's second ratio step (`TkMenuSetHelpRatioDiff2`).
    pub const MENU_HELP_RATIO_DIFF2: Global<u32> = Global::new(0x0184_1c10);
    /// The aeon total list (`u8` ids, `TkMenuGetSummonTotalFromIndex`).
    pub const MENU_SUMMON_TOTALS: Global<u8> = Global::new(0x0184_1c44);
    /// Entries in [`MENU_SUMMON_TOTALS`].
    pub const MENU_SUMMON_TOTAL_MAX: Global<i32> = Global::new(0x0184_1c58);
    /// The Mn2 list count (`TkMn2GetListMax`).
    pub const MN2_LIST_MAX: Global<u32> = Global::new(0x0186_a24c);
    /// The Mn2 name-mode count (`TkMn2GetNameModeMax`).
    pub const MN2_NAME_MODE_MAX: Global<u32> = Global::new(0x0186_a250);
    /// The aeon menu's status window (closed with the status background).
    pub const SUMMON_STATUS_WINDOW: Global<u32> = Global::new(0x0186_a5a4);
    /// The aeon menu's count that picks the window state on restart.
    pub const SUMMON_RESTART_COUNT: Global<i32> = Global::new(0x0186_a5fc);
    /// The status menu's windows, by index (`+0x54` fade step on close).
    pub const STATUS_WINDOWS: Global<u32> = Global::new(0x0186_a820);
    /// The weapon shop's windows (`TkMenuCtrlWShop`): five more.
    pub const WSHOP_WINDOWS: [Global<u32>; 5] = [
        Global::new(0x0186_aaf0),
        Global::new(0x0186_aaf4),
        Global::new(0x0186_aaf8),
        Global::new(0x0186_aafc),
        Global::new(0x0186_ab00),
    ];
    /// The tutorial menu's windows.
    pub const TUTO_WINDOWS: [Global<u32>; 2] = [Global::new(0x0186_ac34), Global::new(0x0186_ac38)];
    /// The world map's window (`TkMenuCtrlWorldMap`).
    pub const WORLD_MAP_WINDOW: Global<u32> = Global::new(0x0187_1698);
    /// The second sub-command window (`0x023cc124`, beside
    /// [`MENU_SUB_CMD_WINDOW`]).
    pub const MENU_SUB_CMD_WINDOW2: Global<u32> = Global::new(0x023c_c124);

    globals! {
        "`TkSndGetLastKickId()` (`0x00887ad0`)." tk_snd_get_last_kick_id: get SND_LAST_KICK_ID;
        "`TkSndSetFootKickStay(v)` (`0x00887b40`)." tk_snd_set_foot_kick_stay: set SND_FOOT_KICK_STAY;
        "`TkClearSoundSyncVFlag3()` (`0x00887d50`)." tk_clear_sound_sync_v_flag3: clear SOUND_SYNC_V_FLAG3 = 0;
        "`TkGetSoundSyncVFlag3()` (`0x00887df0`)." tk_get_sound_sync_v_flag3: get SOUND_SYNC_V_FLAG3;
        "`TkMcSetDebugLock(v)` (`0x00888420`)." tk_mc_set_debug_lock: set MC_DEBUG_LOCK;
        "`TkGetDebugMenuStatus()` (`0x0088d5b0`)." tk_get_debug_menu_status: get DEBUG_MENU_STATUS;
        "`TkGetHddErrorFlag()` (`0x0088d5f0`)." tk_get_hdd_error_flag: get HDD_ERROR_FLAG;
        "`TkMenuSetHelpRatioDiff2(v)` (`0x008aaee0`)." tk_menu_set_help_ratio_diff2: set MENU_HELP_RATIO_DIFF2;
        "`TkMn2GetListMax()` (`0x008c1bb0`)." tk_mn2_get_list_max: get MN2_LIST_MAX;
        "`TkMn2GetNameModeMax()` (`0x008c1bc0`)." tk_mn2_get_name_mode_max: get MN2_NAME_MODE_MAX;
        "`TkMenuSetHelpRatio2()` (`0x008ab270`): the ratio to 1 (own name)." tk_menu_set_help_ratio2: clear MENU_HELP_RATIO2 = 1;
    }

    /// `TkMcGetLoadFilePos(slot)` (`0x00888300`).
    pub fn tk_mc_get_load_file_pos(mem: &impl Mem, slot: u32) -> u32 {
        mem.read(
            MC_LOAD_FILE_POS
                .ptr(mem)
                .addr
                .wrapping_add(slot.wrapping_mul(4)),
        )
    }

    /// `TkMcSetLoadFilePos(slot, pos)` (`0x00888480`).
    pub fn tk_mc_set_load_file_pos(mem: &mut impl Mem, slot: u32, pos: u32) {
        let at = MC_LOAD_FILE_POS
            .ptr(mem)
            .addr
            .wrapping_add(slot.wrapping_mul(4));
        mem.write(at, pos);
    }

    /// `TkGetCommonVibrate()` (`0x0088d590`): the common curve's address.
    pub fn tk_get_common_vibrate(mem: &impl Mem) -> u32 {
        COMMON_VIBRATE.ptr(mem).addr
    }

    /// `TkGetEventNewGame()` (`0x0088d5d0`): save byte `+0x2b`.
    pub fn tk_get_event_new_game(mem: &impl Mem) -> u8 {
        mem.read(event(mem) + 0x2b)
    }

    /// `TkSetEventNewGame(v)` (`0x0088eb40`): save byte `+0x2b`; returns
    /// the save block (left in `eax`).
    pub fn tk_set_event_new_game(mem: &mut impl Mem, v: u8) -> u32 {
        let e = event(mem);
        mem.write(e + 0x2b, v);
        e
    }

    /// `TkGetLastEventJumpId()` (`0x0088d600`): the previous event id
    /// (`+2`, beside [`tk_get_now_event_jump_id`]'s `+0`).
    pub fn tk_get_last_event_jump_id(mem: &impl Mem) -> u16 {
        mem.read(event(mem) + 2)
    }

    /// `TkGetLastEventJumpIndex()` (`0x0088d610`): `+0xd`.
    pub fn tk_get_last_event_jump_index(mem: &impl Mem) -> u8 {
        mem.read(event(mem) + 0xd)
    }

    /// `TkGetLastEventJumpMapIdNo()` (`0x0088d620`): `+0xa`.
    pub fn tk_get_last_event_jump_map_id_no(mem: &impl Mem) -> u16 {
        mem.read(event(mem) + 0xa)
    }

    /// `TkGetLastEventJumpMapNo()` (`0x0088d630`): `+6`.
    pub fn tk_get_last_event_jump_map_no(mem: &impl Mem) -> u16 {
        mem.read(event(mem) + 6)
    }

    /// `TkSetMapGroupDrawStatus(group, on)` (`0x0088eba0`): the field
    /// draw mask's bit `group & 31` set or cleared.
    pub fn tk_set_map_group_draw_status(mem: &mut impl Mem, group: u32, on: u32) {
        let mask = yi::yi_get_draw_mask(mem);
        let bit = 1u32 << (group & 0x1f);
        yi::yi_set_draw_mask(mem, if on != 0 { mask | bit } else { mask & !bit });
    }

    /// `ply_save` byte `field` of character `chr` (`TkMenuGetArmor` and
    /// kin: `+0x2d` weapon, `+0x2e` armour, `+0x38` overdrive mode, `+0x39`
    /// overdrive gauge, `+0x3a` its maximum).
    pub fn tk_menu_get_ply_byte(mem: &impl Mem, chr: u32, field: u32) -> u8 {
        let ply = save::ms_get_save_player_ptr(mem, chr);
        mem.read(ply.wrapping_add(field))
    }

    /// `TkMenuSetHP(chr, hp)` (`0x008aae90`): `+0x1c`; a nonzero HP clears
    /// the KO byte `+0x3d`. Returns the record (left in `eax`).
    pub fn tk_menu_set_hp(mem: &mut impl Mem, chr: u32, hp: u32) -> u32 {
        let ply = save::ms_get_save_player_ptr(mem, chr);
        mem.write(ply.wrapping_add(0x1c), hp);
        if hp != 0 {
            mem.write(ply.wrapping_add(0x3d), 0u8);
        }
        ply
    }

    /// `TkMenuSetMP(chr, mp)` (`0x008aaef0`) and the other `ply_save`
    /// setters: a `u32` field; the record back (left in `eax`).
    pub fn tk_menu_set_ply(mem: &mut impl Mem, chr: u32, field: u32, v: u32) -> u32 {
        let ply = save::ms_get_save_player_ptr(mem, chr);
        mem.write(ply.wrapping_add(field), v);
        ply
    }

    /// `TkMenuSetSP(chr, sp)` (`0x008ab000`) and `TkMn2SetOverDriveId`
    /// (`0x008c2c90`, `+0x38`): a byte field; the record back.
    pub fn tk_menu_set_ply_byte(mem: &mut impl Mem, chr: u32, field: u32, v: u8) -> u32 {
        let ply = save::ms_get_save_player_ptr(mem, chr);
        mem.write(ply.wrapping_add(field), v);
        ply
    }

    /// `TkMenuGetTime()` (`0x008a9c50`): the play time, save `+0xbc`.
    pub fn tk_menu_get_time(mem: &impl Mem) -> u32 {
        mem.read(event(mem) + 0xbc)
    }

    /// `TkMenuGetSummonTotalFromIndex(i)` (`0x008a9c00`): entry `i` of the
    /// aeon total list: entry 0 for a negative `i`, the last (`max - 1`,
    /// even -1 for an empty list) past the end.
    pub fn tk_menu_get_summon_total_from_index(mem: &impl Mem, i: i32) -> u8 {
        let last = MENU_SUMMON_TOTAL_MAX.get(mem).wrapping_sub(1);
        let i = if i < 0 {
            0
        } else if i > last {
            last
        } else {
            i
        };
        mem.read(
            MENU_SUMMON_TOTALS
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned()),
        )
    }

    /// The menu slot for `slot`, clamped to `0..=24`.
    fn slot_at(mem: &impl Mem, slot: i32) -> u32 {
        MENU_SLOTS.ptr(mem).addr + 4 * slot.clamp(0, 0x18).cast_unsigned()
    }

    /// `TkMenuMainRegistMenu(slot, window)` (`0x008aaab0`): stores the
    /// window in its slot, clears its active flags (`+0x18`) and timer
    /// (`+0x1c`) and calls its init callback `+8` with the menu mode.
    /// Returns what the callback returns, 0 without one.
    pub fn tk_menu_main_regist_menu<M: Mem>(
        mem: &mut M,
        slot: i32,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = slot_at(mem, slot);
        mem.write(at, window);
        let f = mem.read::<u32>(window.wrapping_add(8));
        mem.write(window.wrapping_add(0x18), 0u16);
        mem.write(window.wrapping_add(0x1c), 0u32);
        if f == 0 {
            return 0;
        }
        let mode = MN2_MENU_MODE.get(mem);
        call(mem, f, &[window, mode])
    }

    /// `TkMenuMainRegistWindow(window)` (`0x008aab00`): marks it used
    /// (`+0x40`), state idle, and tail-calls its callback `+8`; returns
    /// what that returns, or the window without one.
    pub fn tk_menu_main_regist_window<M: Mem>(
        mem: &mut M,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.read::<u32>(window.wrapping_add(8));
        mem.write(window.wrapping_add(0x40), 1u8);
        mem.write(window.wrapping_add(0x28), 0u32);
        if f == 0 {
            return window;
        }
        call(mem, f, &[window])
    }

    /// `TkMenuMainSleepMenu(slot)` (`0x008aad70`): an active menu (`+0x18`
    /// is 1) gets its sleep callback `+0x10`; then its flags are cleared and
    /// its bit leaves the active mask.
    pub fn tk_menu_main_sleep_menu<M: Mem>(
        mem: &mut M,
        slot: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let slot = slot.clamp(0, 0x18);
        let w = mem.read::<u32>(slot_at(mem, slot));
        if w == 0 {
            return;
        }
        if mem.read::<u8>(w.wrapping_add(0x18)) == 1 {
            let f = mem.read::<u32>(w.wrapping_add(0x10));
            if f != 0 {
                call(mem, f, &[w]);
            }
        }
        mem.write(w.wrapping_add(0x18), 0u16);
        let m = MENU_MAIN_ACTIVE_MASK.get(mem) & !(1u32 << slot);
        MENU_MAIN_ACTIVE_MASK.set(mem, m);
    }

    /// `TkMenuMainActiveMenu(slot, arg)` (`0x008aa100`): slot 1 becomes
    /// 0x16 in menu mode `0x80xxxx`; an inactive menu gets its activate
    /// callback `+0xc` (with `arg`) and its start callback `+0`; then
    /// `+0x18` = `0x0101` and its bit joins the active mask.
    pub fn tk_menu_main_active_menu<M: Mem>(
        mem: &mut M,
        slot: i32,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let slot = if slot == 1 && MN2_MENU_MODE.get(mem) & 0xffff_0000 == 0x80_0000 {
            0x16
        } else {
            slot.clamp(0, 0x18)
        };
        let w = mem.read::<u32>(slot_at(mem, slot));
        if w == 0 {
            return;
        }
        if mem.read::<u8>(w.wrapping_add(0x18)) == 0 {
            let f = mem.read::<u32>(w.wrapping_add(0xc));
            if f != 0 {
                call(mem, f, &[w, arg]);
            }
            let f = mem.read::<u32>(w);
            if f != 0 {
                call(mem, f, &[w]);
            }
        }
        mem.write(w.wrapping_add(0x18), 0x0101u16);
        let m = MENU_MAIN_ACTIVE_MASK.get(mem) | (1u32 << slot);
        MENU_MAIN_ACTIVE_MASK.set(mem, m);
    }

    /// The restart wrappers (`MenuSummon_RestartGrowWindow` and kin):
    /// `TkMenuRestartSelFileWindow` on the window in `global` when there is
    /// one; the global's value back (left in `eax`).
    pub fn restart_global_window(mem: &mut impl Mem, global: Global<u32>) -> u32 {
        let w = global.get(mem);
        if w != 0 {
            tk_menu_restart_sel_file_window(mem, w);
        }
        w
    }

    /// `MenuTuto_RestartWindow2()` (`0x008daac0`): `TkMenuRestartWindow`
    /// on the tutorial's second window.
    pub fn menu_tuto_restart_window2(mem: &mut impl Mem) -> u32 {
        let w = TUTO_WINDOWS[1].get(mem);
        if w != 0 {
            tk_menu_restart_window(mem, w);
        }
        w
    }

    /// `MenuWShop_RestartListWindow()` (`0x008d8420`): restarts the shop's
    /// first window, then sets it idle and not busy; the window back.
    pub fn menu_wshop_restart_list_window(mem: &mut impl Mem) -> u32 {
        let w = WSHOP_WINDOWS[0].get(mem);
        if w == 0 {
            return 0;
        }
        tk_menu_restart_sel_file_window(mem, w);
        let w = WSHOP_WINDOWS[0].get(mem);
        mem.write(w.wrapping_add(0x28), 0u32);
        mem.write(w.wrapping_add(0x45), 0u8);
        w
    }

    /// `MenuSummon_CloseStatusWindow()` (`0x008ccb10`): resets the status
    /// background, then requests the close and forgets the window.
    pub fn menu_summon_close_status_window(mem: &mut impl Mem) {
        if SUMMON_STATUS_WINDOW.get(mem) == 0 {
            return;
        }
        tk_mn2_init_status_bg(mem);
        let w = SUMMON_STATUS_WINDOW.get(mem);
        SUMMON_STATUS_WINDOW.set(mem, 0);
        mem.write(w.wrapping_add(0x41), 1u8);
    }

    /// `MenuSummon_RestartWindow()` (`0x008cd1f0`): the aeon window not
    /// busy, state 0 (count up to 1) or 2, then the status background
    /// reset.
    pub fn menu_summon_restart_window(mem: &mut impl Mem) {
        let w = SUMMON_WINDOW.get(mem);
        if w == 0 {
            return;
        }
        let state = if SUMMON_RESTART_COUNT.get(mem) <= 1 {
            0u32
        } else {
            2
        };
        mem.write(w.wrapping_add(0x45), 0u8);
        mem.write(w.wrapping_add(0x28), state);
        tk_mn2_init_status_bg(mem);
    }

    /// `MenuStatus_CloseWindow(i)` (`0x008d2680`): close request for status
    /// window `i`, fade step `-0xaa`, forget it; the global's value back.
    pub fn menu_status_close_window(mem: &mut impl Mem, i: u32) -> u32 {
        let at = STATUS_WINDOWS.ptr(mem).addr.wrapping_add(i.wrapping_mul(4));
        let w = mem.read::<u32>(at);
        if w == 0 {
            return 0;
        }
        mem.write(w.wrapping_add(0x41), 1u8);
        let w = mem.read::<u32>(at);
        mem.write(w.wrapping_add(0x54), 0xff56u16);
        mem.write(at, 0u32);
        w
    }

    /// `MenuSubCmd_RefreshHelp()` (`0x008e49b0`): the sub-command window's
    /// help message for its cursor (`+0x5c + 12 × cursor`).
    pub fn menu_sub_cmd_refresh_help(mem: &mut impl Mem) {
        let w = MENU_SUB_CMD_WINDOW.get(mem);
        if w == 0 {
            return;
        }
        let cursor = i32::from(mem.read::<i16>(w.wrapping_add(0x48))).cast_unsigned();
        let id = mem.read::<i32>(w.wrapping_add(0x5c).wrapping_add(cursor.wrapping_mul(0xc)));
        tk_menu_set_help_message(mem, id);
    }

    /// `MenuSubCmd_SetValue(v)` (`0x008e4a10`) and `MenuSubCmd_SetValue2`
    /// (`0x008e4a30`): `u16` `+0x38` of a sub-command window, if any.
    pub fn menu_sub_cmd_set_value(mem: &mut impl Mem, global: Global<u32>, v: u16) {
        let w = global.get(mem);
        if w != 0 {
            mem.write(w.wrapping_add(0x38), v);
        }
    }

    /// `MenuSummon_GrowEntry(i)` (`0x008cecf0`): `TkMn2GetSummonGrow() + 8
    /// × i + 8 × TkMn2GetSummonGrowMax()`.
    pub fn menu_summon_grow_entry(mem: &mut impl Mem, i: u32) -> u32 {
        let grow = tk_mn2_get_summon_grow(mem);
        let max = MENU_SUMMON_GROW_MAX.get(mem);
        grow.wrapping_add(i.wrapping_mul(8))
            .wrapping_add(max.wrapping_mul(8))
    }

    /// `MenuItem_IsUsable(list, i)` (`0x008c9120`): items (list 3 is the
    /// key items: usable when the counter is nonzero) are usable.
    pub fn menu_item_is_usable(mem: &impl Mem, list: i32) -> u32 {
        if list == 3 {
            u32::from(tk_menu_get_important_counter(mem) != 0)
        } else {
            1
        }
    }

    /// `MenuPlayer_DamageClass(chr)` (`0x008a99f0`, own name): 1 for a
    /// character outside the party (`MsGetSavePlyJoin` 0) while the
    /// ignore-join flag is clear, or one who left (`Joined` 1); otherwise 2
    /// when `MsCalcWeakLevel(hp, max HP)` is positive, else 0.
    /// `MenuPlayer_DamageClass2` (`0x008a9a70`) skips the first test.
    pub fn menu_player_damage_class(mem: &impl Mem, chr: u32, check_ignore: bool) -> u32 {
        let hp = tk_menu_get_ply(mem, chr, 0x1c).cast_signed();
        let max = tk_menu_get_ply(mem, chr, 0x24).cast_signed();
        let join = save::ms_get_save_ply_join(mem, chr);
        if check_ignore && join == 0 && MENU_MAIN_IGNORE_JOIN.get(mem) == 0 {
            return 1;
        }
        if join == 0 && save::ms_get_save_ply_joined(mem, chr) == 1 {
            return 1;
        }
        if ffx_battle::actor::bridge::ms_calc_weak_level(hp, max).cast_signed() >= 1 {
            2
        } else {
            0
        }
    }

    /// `TkGetControlerActuatorMode(pad, port)` (`0x00888ba0`): byte
    /// `+0x9d` of the controller work.
    pub fn pad_get_work_byte_9d(mem: &impl Mem, pad: i32, port: i32) -> u8 {
        mem.read(tk_get_controler_work(mem, pad, port).wrapping_add(0x9d))
    }

    /// `TkGetControlerAnalog(pad, port, i)` (`0x00888c00`): analog byte
    /// `+0x84 + i`. The original loads it into `al` over `i`, so the upper
    /// bytes of `eax` are `i`'s.
    pub fn tk_get_controler_analog(mem: &impl Mem, pad: i32, port: i32, i: u32) -> u32 {
        let b = mem.read::<u8>(
            tk_get_controler_work(mem, pad, port)
                .wrapping_add(0x84)
                .wrapping_add(i),
        );
        (i & !0xff) | u32::from(b)
    }

    /// `TkGetControlerId(pad, port)` (`0x00888d90`): byte `+0x80`, loaded
    /// into `al` over the work pointer (whose upper bytes stay in `eax`).
    pub fn tk_get_controler_id(mem: &impl Mem, pad: i32, port: i32) -> u32 {
        let w = tk_get_controler_work(mem, pad, port);
        (w & !0xff) | u32::from(mem.read::<u8>(w.wrapping_add(0x80)))
    }

    /// `TkGetControlerPress(pad, port)` (`0x00888dd0`): byte `+0x82`.
    pub fn tk_get_controler_press(mem: &impl Mem, pad: i32, port: i32) -> u8 {
        mem.read(tk_get_controler_work(mem, pad, port).wrapping_add(0x82))
    }

    /// `Pad_SetWorkByte(pad, port, field, v)`: the actuator bytes `+0xfe`
    /// (`Pad_SetActuatorFE`, `0x008894b0`) and `+0xff`
    /// (`Pad_SetActuatorFF`, `0x00889490`); the work pointer back.
    pub fn pad_set_work_byte(mem: &mut impl Mem, pad: i32, port: i32, field: u32, v: u8) -> u32 {
        let w = tk_get_controler_work(mem, pad, port);
        mem.write(w.wrapping_add(field), v);
        w
    }

    /// `Pad_ResetAnalog(work)` (`0x00889000`, own name): the two analog
    /// quads at `+0x84` and `+0x90` centred (`0x80`), the words between
    /// cleared. `Pad_ResetAnalogAt0C` (`0x00889030`) does the same at
    /// `+0xc`..`+0x1b`. Returns `work`.
    pub fn pad_reset_analog(mem: &mut impl Mem, work: u32, base: u32) -> u32 {
        let at = work.wrapping_add(base);
        mem.write(at, 0x8080_8080u32);
        mem.write(at.wrapping_add(4), 0u32);
        mem.write(at.wrapping_add(8), 0u32);
        mem.write(at.wrapping_add(0xc), 0x8080_8080u32);
        work
    }

    /// `Pad_SetActuatorPair(work, a, b)` (`0x00889460`, own name): bytes
    /// `+0xf0`, `+0xf1` and the `u16` `+0xf8` cleared.
    pub fn pad_set_actuator_pair(mem: &mut impl Mem, work: u32, a: u8, b: u8) {
        mem.write(work.wrapping_add(0xf0), a);
        mem.write(work.wrapping_add(0xf1), b);
        mem.write(work.wrapping_add(0xf8), 0u16);
    }

    /// `Pad_ResetMode(work)` (`0x00888ee0`, own name): `u16` `+0xf6` =
    /// `0x0808`; returns `work`.
    pub fn pad_reset_mode(mem: &mut impl Mem, work: u32) -> u32 {
        mem.write(work.wrapping_add(0xf6), 0x0808u16);
        work
    }

    fn item_entry(mem: &impl Mem, i: u32) -> u32 {
        MENU_ITEM_LIST.ptr(mem).addr.wrapping_add(i.wrapping_mul(4))
    }

    /// The item lists' `usable` callbacks (`MenuItem_UsableByte`,
    /// `0x008c6a30`, `0x008c6a40`, `0x008d9b00`): entry `i`'s byte `+3`.
    pub fn menu_item_usable_byte(mem: &impl Mem, i: u32) -> u8 {
        mem.read(item_entry(mem, i).wrapping_add(3))
    }

    /// `MenuKaizou_ItemUsable(list, i)` (`0x008d5730`): byte `+3` is not
    /// `0xff`.
    pub fn menu_kaizou_item_usable(mem: &impl Mem, i: u32) -> bool {
        menu_item_usable_byte(mem, i) != 0xff
    }

    /// `MenuWShop_ItemPresent(list, i)` (`0x008d75d0`): entry `i`'s id
    /// is nonzero.
    pub fn menu_wshop_item_present(mem: &impl Mem, i: u32) -> bool {
        mem.read::<u16>(item_entry(mem, i)) != 0
    }

    /// The window-state callbacks that set `+0x28` (state) to 0
    /// (`MenuEquip_WindowIdle` and kin); the window back.
    pub fn window_idle(mem: &mut impl Mem, window: u32) -> u32 {
        mem.write(window.wrapping_add(0x28), 0u32);
        window
    }

    /// `MenuSubCmd_CursorAtTop(window)` (`0x008e4a90`): the `i16` at
    /// `+0x46` is not positive.
    pub fn menu_sub_cmd_cursor_at_top(mem: &impl Mem, window: u32) -> bool {
        mem.read::<i16>(window.wrapping_add(0x46)) < 1
    }

    /// `MenuSubCmd_CenterX(window)` (`0x008e4ad0`): `+0x4a` = (512 - width
    /// `+0x4e`) / 2, truncated toward 0; the whole quotient back (left in
    /// `eax`).
    pub fn menu_sub_cmd_center_x(mem: &mut impl Mem, window: u32) -> i32 {
        let w = i32::from(mem.read::<i16>(window.wrapping_add(0x4e)));
        let x = (0x200 - w) / 2;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // the original stores AX
        mem.write(window.wrapping_add(0x4a), x as u16);
        x
    }

    /// `MenuList_IsEmptySlot(entry)` (`0x008bf720`): 1 when the `u16`
    /// selected by byte `+0xb` among those from `+0xc` is 0 or `0xff`.
    pub fn menu_list_is_empty_slot(mem: &impl Mem, entry: u32) -> u32 {
        let k = u32::from(mem.read::<u8>(entry.wrapping_add(0xb)));
        let v = mem.read::<u16>(entry.wrapping_add(0xc).wrapping_add(2 * k));
        u32::from(v == 0xff || v == 0)
    }

    /// `MenuMain_OptionY(i)` (`0x008e2260`): option `i`'s `i16` at `+2` of
    /// its 8-byte entry after [`MENU_MAIN_OPTIONS`]` + 4`.
    pub fn menu_main_option_y(mem: &impl Mem, i: u32) -> i32 {
        let at = MENU_MAIN_OPTIONS.ptr(mem).addr + 6;
        i32::from(mem.read::<i16>(at.wrapping_add(i.wrapping_mul(8))))
    }

    /// The tutorial's chosen entry and whether one was chosen.
    pub const TUTO_CHOICE: Global<i32> = Global::new(0x0187_1528);
    /// Set once the tutorial has a choice.
    pub const TUTO_CHOSEN: Global<u32> = Global::new(0x025d_16bc);

    /// `MenuTuto_GetChoice()` (`0x008db840`): the choice, -1 without one.
    pub fn menu_tuto_get_choice(mem: &impl Mem) -> i32 {
        if TUTO_CHOSEN.get(mem) != 0 {
            TUTO_CHOICE.get(mem)
        } else {
            -1
        }
    }

    /// `MenuTuto_SetChoice(window, i)` (`0x008db7a0`): the choice and its
    /// flag; returns `i` (left in `eax`).
    pub fn menu_tuto_set_choice(mem: &mut impl Mem, i: i32) -> i32 {
        TUTO_CHOSEN.set(mem, 1);
        TUTO_CHOICE.set(mem, i);
        i
    }

    /// The item menu's cursor cell (`MenuItem_SetCell`).
    pub const MENU_ITEM_CELL: Global<u32> = Global::new(0x0186_a268);
    /// The item menu's selected list (`MenuItem_SetList`).
    pub const MENU_ITEM_SELECTED_LIST: Global<u32> = Global::new(0x0186_a258);
    /// The Mn2 draw position (`MenuMn2_StepDrawY` adds 16).
    pub const MN2_DRAW_Y: Global<u32> = Global::new(0x0186_a238);

    /// `MenuItem_SetCell(x, y)` (`0x008c5260`): `y × 15 + x`.
    pub fn menu_item_set_cell(mem: &mut impl Mem, x: u32, y: u32) {
        MENU_ITEM_CELL.set(mem, y.wrapping_mul(15).wrapping_add(x));
    }

    /// The battle-end results: 18 words (`TkBtlEndGetResult`), the last
    /// one at `+0x4c`.
    pub const BTL_END_RESULTS: Global<u32> = Global::new(0x0231_0f20);

    /// The item menu's list rows (`MenuItem_GetRows`).
    pub const MENU_ITEM_ROWS: Global<u32> = Global::new(0x0186_a38c);
    /// The equipment menu's alternate mode (toggled in `TkMenuCtrlEquip`).
    pub const MENU_EQUIP_ALT_MODE: Global<u32> = Global::new(0x0186_a624);
    /// Two save/load menu toggles (`u8`).
    pub const MENU_SAVE_LOAD_TOGGLES: Global<[u8; 2]> = Global::new(0x0186_626a);
    /// A battle-end flag (`BtlEnd_SetFlag` sets it, the end clears it).
    pub const BTL_END_FLAG: Global<u32> = Global::new(0x0186_9fbc);
    /// The pause request (`Pause_Update` runs while it is set).
    pub const PAUSE_REQUEST: Global<u32> = Global::new(0x0134_0850);

    /// `BtlEnd_GetResult(i)` (`0x008bda10`, own name).
    pub fn btl_end_get_result(mem: &impl Mem, i: u32) -> u32 {
        mem.read(
            BTL_END_RESULTS
                .ptr(mem)
                .addr
                .wrapping_add(i.wrapping_mul(4)),
        )
    }

    // ---- round 5: memory card wrappers, Mn2 kernel lookups, config
    // callbacks, vibration streams ----

    /// The memory card port and the one the last call used.
    pub const MC_PORT: Global<[u32; 2]> = Global::new(0x0133_0230);

    /// The `TkMc*` wrappers (`TkMcOpen` and kin): most copy the port to
    /// the last-used one; then on the PC every `sceMc*` stub returns 0, as
    /// does the TGS path, so the result is always 0.
    pub fn tk_mc_call(mem: &mut impl Mem, sets_port: bool) -> u32 {
        if sets_port {
            let [port, _] = MC_PORT.get(mem);
            MC_PORT.set(mem, [port, port]);
        }
        0
    }

    /// `TkMcGetInfo(port, ...)` (`0x008882c0`): its port becomes the last
    /// used one; 0 (the `sceMcGetInfo` stub).
    pub fn tk_mc_get_info(mem: &mut impl Mem, port: u32) -> u32 {
        let [p, _] = MC_PORT.get(mem);
        MC_PORT.set(mem, [p, port]);
        0
    }

    /// The debug font buffer pointer (`TkGetDebugFontBuffer` reads its
    /// first word).
    pub const DEBUG_FONT_BUFFER: Global<u32> = Global::new(0x0133_c8cc);

    /// `TkGetDebugFontBuffer()` (`0x0088edc0`): `get_ptr32_func` of the
    /// buffer's first word.
    pub fn tk_get_debug_font_buffer(mem: &impl Mem) -> u32 {
        crate::get_ptr32_func(mem.read(DEBUG_FONT_BUFFER.get(mem)))
    }

    /// `TkMn2GetExcelData(id, table)` (`0x008c1af0`): `MsGetExcelData`'s
    /// record (the strings go to a local it drops).
    pub fn tk_mn2_get_excel_data(mem: &impl Mem, id: i32, table: u32) -> u32 {
        ffx_kernel::bridge::excel_data(mem, id, table).0
    }

    /// `TkMn2GetKaizou()` (`0x008c1b70`): `MsGetRomKaizou`'s first record.
    pub fn tk_mn2_get_kaizou(mem: &mut impl Mem) -> u32 {
        ffx_kernel::bridge::first_record(mem, ffx_kernel::bridge::KAIZOU, 0)
    }

    /// `TkMn2GetKaizouMax()` (`0x008c1b90`): the customize table's record
    /// count (first segment).
    pub fn tk_mn2_get_kaizou_max(mem: &impl Mem) -> i32 {
        let t = ffx_kernel::bridge::KAIZOU.get(mem);
        ffx_kernel::bridge::ms_get_excel_data_num(mem, t, 0)
    }

    /// Two Mn2 words `TkMn2InitWork` clears.
    pub const MN2_WORK: [Global<u32>; 2] = [Global::new(0x0186_a240), Global::new(0x0186_a248)];

    /// `TkMn2InitWork()` (`0x008c1c50`): the Mn2 counters to 0, the
    /// name-mode count to 1.
    pub fn tk_mn2_init_work(mem: &mut impl Mem) {
        MN2_WORK[0].set(mem, 0);
        MN2_WORK[1].set(mem, 0);
        MENU_SUMMON_GROW_MAX.set(mem, 0);
        MN2_LIST_MAX.set(mem, 0);
        MN2_NAME_MODE_MAX.set(mem, 1);
    }

    /// `MenuMn2_StepDrawY80(...)` (`0x008c0dd0`, own name): the draw y
    /// becomes `MenuMn2_Plus80(y + 16)`; returns it.
    pub fn menu_mn2_step_draw_y80(mem: &mut impl Mem) -> u32 {
        let v = super::plus_80(MN2_DRAW_Y.get(mem).wrapping_add(0x10));
        MN2_DRAW_Y.set(mem, v);
        v
    }

    /// `MenuItem_AddMP(chr, n)` (`0x008c8e20`, own name): MP + `n`, capped
    /// at the maximum (signed); the record back (from `TkMenuSetMP`).
    pub fn menu_item_add_mp(mem: &mut impl Mem, chr: u32, n: u32) -> u32 {
        let mp = tk_menu_get_ply(mem, chr, 0x20).wrapping_add(n);
        let max = tk_menu_get_ply(mem, chr, 0x28);
        let mp = if mp.cast_signed() < max.cast_signed() {
            mp
        } else {
            max
        };
        tk_menu_set_ply(mem, chr, 0x20, mp)
    }

    /// The party members' bit mask (`MenuParty_RefreshMask`).
    pub const MENU_PARTY_BITS: Global<u32> = Global::new(0x0184_1c2c);

    /// `MenuParty_RefreshMask()` (`0x008aae30`, own name): one bit per
    /// party slot (`TkMenuGetPlayerPartyMask` of them) that holds a
    /// character, by its id.
    pub fn menu_party_refresh_mask(mem: &mut impl Mem) {
        MENU_PARTY_BITS.set(mem, 0);
        let n = MENU_PLAYER_PARTY_MASK.get(mem).cast_signed();
        let base = MENU_PLAYERS.ptr(mem).addr;
        let mut bits = 0u32;
        for i in 0..n.max(0).cast_unsigned() {
            let c = mem.read::<u8>(base.wrapping_add(i));
            if c != 0xff {
                bits |= 1 << (c & 0x1f);
            }
        }
        MENU_PARTY_BITS.set(mem, bits);
    }

    /// `TkRefreshActuator()` (`0x00872480`): pad 0's
    /// actuator mode from the menu setting bit 4.
    pub fn pad_refresh_actuator_mode(mem: &mut impl Mem) -> u32 {
        let on = (mem.read::<u8>(save::ms_get_save_menu(mem)) >> 4) & 1;
        tk_set_controler_actuator_mode(mem, 0, 0, on)
    }

    /// `Pad_GetFrame(pad, port, i)` (`0x00888b80`, own name):
    /// `TkGetControlerFrame(TkGetControlerWork(pad, port), i)`.
    pub fn pad_get_frame(mem: &impl Mem, pad: i32, port: i32, i: i32) -> u32 {
        tk_get_controler_frame(mem, tk_get_controler_work(mem, pad, port), i)
    }

    fn menu_word(mem: &impl Mem) -> u32 {
        mem.read(save::ms_get_save_menu(mem))
    }

    /// The config menu's read callbacks (`MenuConfig_GetBit1` and kin):
    /// option `+8` = menu setting bit `bit`, inverted when `invert`; the
    /// option back.
    pub fn menu_config_get_bit(mem: &mut impl Mem, option: u32, bit: u32, invert: bool) -> u32 {
        let v = ((menu_word(mem) >> bit) & 1) ^ u32::from(invert);
        mem.write(option.wrapping_add(8), v);
        option
    }

    /// The config menu's write callbacks (`MenuConfig_SetBit5` and kin):
    /// the menu setting bits `mask` set when option `+8` is 0 (`set_on_0`)
    /// or nonzero, cleared otherwise; the setting's address back.
    pub fn menu_config_set_bits(mem: &mut impl Mem, option: u32, mask: u32, set_on_0: bool) -> u32 {
        let at = save::ms_get_save_menu(mem);
        let v = mem.read::<u32>(at);
        let set = (mem.read::<u32>(option.wrapping_add(8)) == 0) == set_on_0;
        mem.write(at, if set { v | mask } else { v & !mask });
        at
    }

    /// `MenuConfig_SetBit9(option)` (`0x008cadb0`): option `+0xc` = bit 8,
    /// then bit 9 set when `+8` is 0, cleared otherwise.
    pub fn menu_config_set_bit9(mem: &mut impl Mem, option: u32) -> u32 {
        let bit8 = (menu_word(mem) >> 8) & 1;
        mem.write(option.wrapping_add(0xc), bit8);
        menu_config_set_bits(mem, option, 0x200, true)
    }

    /// `MenuConfig_GetBit5(option)` (`0x008cade0`): `+8` = !bit 5, `+0xc`
    /// = bit 8; returns 0 or -1 by bit 8 (left in `eax`).
    pub fn menu_config_get_bit5(mem: &mut impl Mem, option: u32) -> u32 {
        let m = menu_word(mem);
        mem.write(option.wrapping_add(0xc), 1u32);
        mem.write(option.wrapping_add(8), (!m >> 5) & 1);
        let keep = if m & 0x100 != 0 { u32::MAX } else { 0 };
        let v = mem.read::<u32>(option.wrapping_add(0xc)) & keep;
        mem.write(option.wrapping_add(0xc), v);
        keep
    }

    /// `MenuConfig_GetBit8(option)` (`0x008cb490`): `+0xc` = 1, `+8` =
    /// !bit 8; returns `+8`'s value.
    pub fn menu_config_get_bit8(mem: &mut impl Mem, option: u32) -> u32 {
        mem.write(option.wrapping_add(0xc), 1u32);
        let v = (!menu_word(mem) >> 8) & 1;
        mem.write(option.wrapping_add(8), v);
        v
    }

    /// `MenuConfig_GetBit9(option)` (`0x008cb500`): `+0xc` = bit 8, `+8` =
    /// !bit 9; returns `+8`'s value.
    pub fn menu_config_get_bit9(mem: &mut impl Mem, option: u32) -> u32 {
        let m = menu_word(mem);
        mem.write(option.wrapping_add(0xc), (m >> 8) & 1);
        let v = (!menu_word(mem) >> 9) & 1;
        mem.write(option.wrapping_add(8), v);
        v
    }

    /// `MenuConfig_GetEnglish(option)` (`0x008cafd0`): `+8` =
    /// `MsGetSaveConfigEnglish() != 1`; that stub returns 0 on the PC, so
    /// 1. Returns the value.
    pub fn menu_config_get_english(mem: &mut impl Mem, option: u32) -> u32 {
        mem.write(option.wrapping_add(8), 1u32);
        1
    }

    /// `MenuConfig_SetEnglish(option)` (`0x008caff0`):
    /// `MsSetSaveConfigEnglish(+8 == 0)` with the game language.
    pub fn menu_config_set_english(mem: &mut impl Mem, option: u32) {
        let on = u32::from(mem.read::<u32>(option.wrapping_add(8)) == 0);
        let lang = i32::from(LANG.get(mem));
        ffx_battle::minor::bridge::ms_set_save_config_english(mem, on, lang);
    }

    /// `MenuConfig_GetSaveFlag(option)` (`0x008cb090`): `+8` = save byte
    /// `+0x5ec` bit 0; the option back.
    pub fn menu_config_get_save_flag(mem: &mut impl Mem, option: u32) -> u32 {
        let v = u32::from(mem.read::<u8>(event(mem) + 0x5ec) & 1);
        mem.write(option.wrapping_add(8), v);
        option
    }

    /// `Vibrate_ResetStream(stream)` (`0x00875f30`, own name): `+4` = 0,
    /// `u16` `+8` = 0, `+0xa` = 0; returns 0.
    pub fn vibrate_reset_stream(mem: &mut impl Mem, stream: u32) -> u32 {
        mem.write(stream.wrapping_add(4), 0u32);
        mem.write(stream.wrapping_add(8), 0u16);
        mem.write(stream.wrapping_add(0xa), 0u8);
        0
    }

    /// `Vibrate_ResetPad(pad)` (`0x00875ef0`, own name): state `+0x22` and
    /// `u16` `+0x20` cleared, then both 16-byte streams reset; returns 0.
    pub fn vibrate_reset_pad(mem: &mut impl Mem, pad: u32) -> u32 {
        mem.write(pad.wrapping_add(0x22), 0u8);
        mem.write(pad.wrapping_add(0x20), 0u16);
        vibrate_reset_stream(mem, pad);
        vibrate_reset_stream(mem, pad.wrapping_add(0x10))
    }

    /// `Vibrate_HoldStream(stream)` (`0x00876160`, own name): `+4` =
    /// `0xfffe0000`, `u16` `+8` = 0; returns `is_ptr32_null` of its
    /// pointer.
    pub fn vibrate_hold_stream(mem: &mut impl Mem, stream: u32) -> u32 {
        let p = mem.read::<u32>(stream);
        mem.write(stream.wrapping_add(4), 0xfffe_0000u32);
        mem.write(stream.wrapping_add(8), 0u16);
        u32::from(crate::is_ptr32_null(p))
    }

    /// `Vibrate_StartStream(stream, curve)` (`0x00876350`, own name): from
    /// the curve header (`u8` level, `u8` length, `u8` flag): `u16` `+4` =
    /// 0, `+6` = (length + 1) × 2, `+8` level, `+9` flag; the stream's
    /// pointer = the data after the first two bytes, returned.
    pub fn vibrate_start_stream(mem: &mut impl Mem, stream: u32, curve: u32) -> u32 {
        let data = curve.wrapping_add(2);
        mem.write(stream.wrapping_add(4), 0u16);
        let len = mem.read::<u8>(curve.wrapping_add(1));
        mem.write(
            stream.wrapping_add(6),
            u16::from(len).wrapping_add(1).wrapping_mul(2),
        );
        let level = mem.read::<u8>(curve);
        mem.write(stream.wrapping_add(8), level);
        let flag = mem.read::<u8>(data);
        mem.write(stream.wrapping_add(9), flag);
        mem.write(stream, data);
        data
    }

    /// `Vibrate_CheckDone(pad)` (`0x00875d30`, own name): state 1 ends when
    /// both streams' `+6` are `0x200`, state 2 when both are `0xfffe`;
    /// returns `pad`.
    pub fn vibrate_check_done(mem: &mut impl Mem, pad: u32) -> u32 {
        let want = match mem.read::<u8>(pad.wrapping_add(0x22)) {
            1 => 0x200u16,
            2 => 0xfffe,
            _ => return pad,
        };
        if mem.read::<u16>(pad.wrapping_add(6)) == want
            && mem.read::<u16>(pad.wrapping_add(0x16)) == want
        {
            mem.write(pad.wrapping_add(0x22), 0u8);
        }
        pad
    }

    /// The item menu's key-item-use flag (`MenuItem_IsUsable2`).
    pub const MENU_ITEM_KEY_USE: Global<u8> = Global::new(0x0186_a280);

    /// `MenuItem_IsUsable2(window, list)` (`0x008c3550`, own name): list 4,
    /// or list 3 with a target, is usable when the key-item flag is set;
    /// other lists always.
    pub fn menu_item_is_usable2(mem: &impl Mem, list: i32) -> u32 {
        if list == 4 || (list == 3 && MENU_ITEM_SELECTED_LIST.get(mem).cast_signed() >= 0) {
            u32::from(MENU_ITEM_KEY_USE.get(mem) != 0)
        } else {
            1
        }
    }

    /// The equipment menu's chosen item id (`0xff` for none).
    pub const MENU_EQUIP_ITEM: Global<u32> = Global::new(0x0186_a630);

    /// `MenuEquip_PickItem()` (`0x008d0430`, own name): the id of the item
    /// list entry under the equipment window's cursor (`i16` `+0x48`), or
    /// `0xff` when empty; returns the id read.
    pub fn menu_equip_pick_item(mem: &mut impl Mem) -> u32 {
        let w = EQUIP_WINDOW.get(mem);
        let i = i32::from(mem.read::<i16>(w.wrapping_add(0x48))).cast_unsigned();
        let id = u32::from(
            mem.read::<u16>(MENU_ITEM_LIST.ptr(mem).addr.wrapping_add(i.wrapping_mul(4))),
        );
        MENU_EQUIP_ITEM.set(mem, if id == 0 { 0xff } else { id });
        id
    }

    /// `MenuSaveLoad_Reset(v)` (`0x008b4e70`, own name): two flags and two
    /// words of the save/load menu cleared, the second window's `+0x58` =
    /// `v`; that window back.
    pub fn menu_save_load_reset(mem: &mut impl Mem, v: u32) -> u32 {
        let w = SAVE_LOAD_WINDOW2.get(mem);
        mem.write(mem.rebase(0x0186_628b), 0u8);
        mem.write(mem.rebase(0x0186_62f3), 0u8);
        mem.write(mem.rebase(0x0186_627c), 0u32);
        mem.write(mem.rebase(0x0186_6280), 0u32);
        mem.write(w.wrapping_add(0x58), v);
        w
    }

    /// The main menu's task (a pointer to a code pointer;
    /// `MenuMain_GetTask`).
    pub const MENU_MAIN_TASK: Global<u32> = Global::new(0x0184_0874);

    /// The sine table `Math_SinLookup` reads (`u16` entries).
    pub const SIN_LOOKUP: Global<u16> = Global::new(0x023c_bee0);

    /// `Math_SinLookup(x)` (`0x0088e730`, own name): entry `x / 64`
    /// (truncating toward 0); the original loads it into `ax`, so the upper
    /// half of `eax` is the index's.
    pub fn math_sin_lookup(mem: &impl Mem, x: i32) -> u32 {
        let i = x.wrapping_add((x >> 31) & 0x3f) >> 6;
        let at = SIN_LOOKUP
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(2));
        (i.cast_unsigned() & 0xffff_0000) | u32::from(mem.read::<u16>(at))
    }

    /// `Math_CosLookup(x)` (`0x0088e750`, own name): `Math_SinLookup` of
    /// `0x1000 - x`.
    pub fn math_cos_lookup(mem: &impl Mem, x: i32) -> u32 {
        math_sin_lookup(mem, 0x1000i32.wrapping_sub(x))
    }

    /// `Table_PairWord(table, i, second)` (`0x008725c0`, own name): word 0
    /// or 1 of the 8-byte entry `i`.
    pub fn table_pair_word(mem: &impl Mem, table: u32, i: u32, second: bool) -> u32 {
        mem.read(
            table
                .wrapping_add(i.wrapping_mul(8))
                .wrapping_add(if second { 4 } else { 0 }),
        )
    }

    /// `Ref_Deref(ref)` (`0x008725a0`, own name): the word at `ref[1]` past
    /// the base pointer `+0x20` of the object `ref[0]`.
    pub fn ref_deref(mem: &impl Mem, r: u32) -> u32 {
        let obj = mem.read::<u32>(r);
        let base = mem.read::<u32>(obj.wrapping_add(0x20));
        let off = mem.read::<u32>(r.wrapping_add(4));
        mem.read(base.wrapping_add(off))
    }

    /// `MenuStatus_ResetEntry(entry)` (`0x008dea20`, own name): `u16` `+0`
    /// and `+0x2c` cleared; returns 0.
    pub fn menu_status_reset_entry(mem: &mut impl Mem, entry: u32) -> u32 {
        mem.write(entry, 0u16);
        mem.write(entry.wrapping_add(0x2c), 0u32);
        0
    }

    /// The `+0x1c` clearing window callbacks (`MenuHdd_WindowReset` and
    /// kin); the window back.
    pub fn window_clear_1c(mem: &mut impl Mem, window: u32) -> u32 {
        mem.write(window.wrapping_add(0x1c), 0u32);
        window
    }

    // ---- round 6: pad history, menu list logic, save/load slots, battle
    // end, status layout, scissor wrappers ----

    /// A pad history `u16` (`TkGetControlerHist` and kin): field `field`
    /// of input frame `i`, loaded into `ax` over the frame pointer.
    pub fn tk_get_controler_hist(mem: &impl Mem, pad: i32, port: i32, i: i32, field: u32) -> u32 {
        let f = pad_get_frame(mem, pad, port, i);
        (f & 0xffff_0000) | u32::from(mem.read::<u16>(f.wrapping_add(field)))
    }

    /// `TkGetControlerAnalogHist(pad, port, k, i)` (`0x00888c20`): analog
    /// byte `k` (`+0xc + k`) of frame `i`, loaded into `al` over `k`.
    pub fn tk_get_controler_analog_hist(
        mem: &impl Mem,
        pad: i32,
        port: i32,
        k: u32,
        i: i32,
    ) -> u32 {
        let f = pad_get_frame(mem, pad, port, i);
        (k & !0xff) | u32::from(mem.read::<u8>(f.wrapping_add(0xc).wrapping_add(k)))
    }

    /// `TkGetControlerActuatorNumber(pad, port)` (`0x00888bc0`): for a
    /// pad of type 5 (`+0x9f`), how many of its two actuator modes
    /// (`+0xf6`, `+0xf7`) are not 8; 0 otherwise.
    pub fn tk_get_controler_actuator_number(mem: &impl Mem, pad: i32, port: i32) -> u32 {
        let w = tk_get_controler_work(mem, pad, port);
        if mem.read::<u8>(w.wrapping_add(0x9f)) != 5 {
            return 0;
        }
        u32::from(mem.read::<u8>(w.wrapping_add(0xf6)) != 8)
            + u32::from(mem.read::<u8>(w.wrapping_add(0xf7)) != 8)
    }

    /// `MenuAbility_RefreshList(chr)` (`0x008c6800`, own name): the item
    /// list from the menu status commands (`MsMenuStatusNum(0, 0)`, 4-byte
    /// records to `0xff`): each `@ A D E` record with byte 1 zero whose
    /// command the character has learnt becomes an entry (id, 1, usable:
    /// command flag `+0x22` bit 6 and MP cost `+0x25` within the
    /// character's MP); the first usable index and the row count are kept.
    /// Returns the rows.
    pub fn menu_ability_refresh_list(mem: &mut impl Mem, chr: u32) -> i32 {
        let mut rec = ffx_battle::menu::bridge::ms_menu_status_num(mem, 0, 0);
        MENU_ITEM_FIRST_USABLE.set(mem, -1);
        let mp = tk_menu_get_ply(mem, chr, 0x20).cast_signed();
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        for k in 0..0x200 {
            mem.write(list + 4 * k, 0u32);
        }
        let mut rows = 0i32;
        let mut at = list;
        loop {
            let kind = mem.read::<u8>(rec);
            if kind == 0xff {
                break;
            }
            if mem.read::<u8>(rec + 1) == 0 && matches!(kind, 0x40 | 0x41 | 0x44 | 0x45) {
                let id = mem.read::<u16>(rec + 2);
                if save::ms_get_save_command(mem, chr.cast_signed(), u32::from(id)) != 0 {
                    mem.write(at, id);
                    mem.write(at + 2, 1u8);
                    let cmd =
                        ffx_kernel::bridge::rom(mem, ffx_kernel::bridge::COMMAND, u32::from(id), 0);
                    let usable = mem.read::<u8>(cmd + 0x22) & 0x40 != 0
                        && i32::from(mem.read::<u8>(cmd + 0x25)) <= mp;
                    mem.write(at + 3, u8::from(usable));
                    if usable && MENU_ITEM_FIRST_USABLE.get(mem) < 0 {
                        #[allow(clippy::cast_possible_truncation)] // the original stores DI
                        MENU_ITEM_FIRST_USABLE.set(mem, rows as i16);
                    }
                    at += 4;
                    rows += 1;
                }
            }
            rec += 4;
        }
        MENU_ITEM_ROWS.set(mem, rows.cast_unsigned());
        rows
    }

    /// `MenuMain_ResetWindow(window)` (`0x008aa4b0`, own name): a menu
    /// window back to its defaults (callbacks and state cleared, ids
    /// `+0x2c`/`+0x2e` = -1, `+0x34` = `0x01000000`, `+0x3e` = `0x0101`).
    pub fn menu_main_reset_window(mem: &mut impl Mem, w: u32) {
        let w32 = |mem: &mut _, o: u32, v: u32| Mem::write(mem, w.wrapping_add(o), v);
        w32(mem, 0x46, 0);
        w32(mem, 0x38, 0);
        w32(mem, 0x24, 0);
        w32(mem, 0x28, 0);
        for o in (0..0x20).step_by(4) {
            w32(mem, o, 0);
        }
        w32(mem, 0x34, 0x0100_0000);
        mem.write(w.wrapping_add(0x2c), 0xffffu16);
        mem.write(w.wrapping_add(0x2e), 0xffffu16);
        w32(mem, 0x30, 0);
        w32(mem, 0x3e, 0x101);
        for o in [0x42, 0x4a, 0x4e, 0x52] {
            w32(mem, o, 0);
        }
        for o in (0x58..0x98).step_by(4) {
            w32(mem, o, 0);
        }
    }

    /// `MenuItem_CompareEntries(list, a, b)` (`0x008c9410`, own name): 32-
    /// byte entries `a` and `b` compared as 8 unsigned words: 1, -1 or 0.
    pub fn menu_item_compare_entries(mem: &impl Mem, list: u32, a: u32, b: u32) -> i32 {
        let (pa, pb) = (list.wrapping_add(a << 5), list.wrapping_add(b << 5));
        for k in 0..8 {
            let x = mem.read::<u32>(pa.wrapping_add(4 * k));
            let y = mem.read::<u32>(pb.wrapping_add(4 * k));
            match x.cmp(&y) {
                core::cmp::Ordering::Greater => return 1,
                core::cmp::Ordering::Less => return -1,
                core::cmp::Ordering::Equal => {}
            }
        }
        0
    }

    /// `MenuItem_SwapEntries(list, a, b)` (`0x008c9460`, own name): the two
    /// 32-byte entries swapped word by word.
    pub fn menu_item_swap_entries(mem: &mut impl Mem, list: u32, a: u32, b: u32) {
        let (pa, pb) = (list.wrapping_add(a << 5), list.wrapping_add(b << 5));
        for k in 0..8 {
            let x = mem.read::<u32>(pa.wrapping_add(4 * k));
            let y = mem.read::<u32>(pb.wrapping_add(4 * k));
            mem.write(pa.wrapping_add(4 * k), y);
            mem.write(pb.wrapping_add(4 * k), x);
        }
    }

    /// `MenuWShop_InitCursor(window)` (`0x008d9580`, own name): the cursor
    /// on the first usable item of the `+0x30` entries (0 when none), the
    /// scroll `+0x32` centred on it within `+0x3a` visible rows and clamped,
    /// copied to `+0x34`.
    pub fn menu_wshop_init_cursor(mem: &mut impl Mem, w: u32) {
        let n = u32::from(mem.read::<u16>(w.wrapping_add(0x30)));
        let mut i = 0u32;
        while i < n && mem.read::<u8>(item_entry(mem, i).wrapping_add(3)) == 0 {
            i += 1;
        }
        let visible = mem.read::<u16>(w.wrapping_add(0x3a));
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        let cursor = if i == n { 0 } else { i as u16 };
        mem.write(w.wrapping_add(0x48), cursor);
        let scroll = cursor.wrapping_sub(visible >> 1);
        mem.write(w.wrapping_add(0x32), scroll);
        if i32::from(scroll.cast_signed()) >= n.cast_signed().wrapping_sub(i32::from(visible)) {
            #[allow(clippy::cast_possible_truncation)]
            mem.write(w.wrapping_add(0x32), (n as u16).wrapping_sub(visible));
        }
        if mem.read::<i16>(w.wrapping_add(0x32)) < 0 {
            mem.write(w.wrapping_add(0x32), 0u16);
        }
        let v = mem.read::<u16>(w.wrapping_add(0x32));
        mem.write(w.wrapping_add(0x34), v);
    }

    /// The save/load menu's 99 slot states (`u8`: 0 free, 1 used, 3
    /// marked) and the any-used flag.
    pub const SAVE_SLOTS: Global<u8> = Global::new(0x0186_6290);
    /// Set when [`SAVE_SLOTS`] has a used slot (`MenuSaveLoad_MarkSlots`).
    pub const SAVE_SLOTS_USED: Global<u32> = Global::new(0x0186_6278);

    /// `MenuSaveLoad_MarkSlots(free, need)` (`0x008b54c0`, own name): marks
    /// (3) cleared; then each free slot is marked while `free` is below
    /// `need`, otherwise `need` is taken from `free`; any used slot sets
    /// the flag.
    pub fn menu_save_load_mark_slots(mem: &mut impl Mem, free: i32, need: i32) {
        let base = SAVE_SLOTS.ptr(mem).addr;
        SAVE_SLOTS_USED.set(mem, 0);
        for k in 0..99 {
            let s = mem.read::<u8>(base + k);
            if s == 0 || s == 3 {
                mem.write(base + k, 0u8);
            }
        }
        let mut free = free;
        for k in 0..99 {
            match mem.read::<u8>(base + k) {
                0 if free < need => mem.write(base + k, 3u8),
                0 => free = free.wrapping_sub(need),
                1 => SAVE_SLOTS_USED.set(mem, 1),
                _ => {}
            }
        }
    }

    /// The weapon shop's list kind, table and slot count.
    pub const WSHOP_LIST: Global<[u32; 4]> = Global::new(0x0186_ac20);

    /// `MenuWShop_CountSlots()` (`0x008db440`, own name): of the up to 16
    /// `u16` ids from `+2` of the shop record (`TkMn2GetExcelData`), the
    /// number before the first 0 or `0xff`.
    pub fn menu_wshop_count_slots(mem: &mut impl Mem) {
        let [id, _, _, table] = WSHOP_LIST.get(mem);
        let rec = tk_mn2_get_excel_data(mem, id.cast_signed(), table);
        let mut n = 0u32;
        while n < 16 {
            let v = mem.read::<u16>(rec.wrapping_add(2 + 2 * n));
            if v == 0xff || v == 0 {
                break;
            }
            n += 1;
        }
        let at = WSHOP_LIST.ptr(mem).addr + 4;
        mem.write(at, n);
    }

    /// The battle-end AP total and its counting step.
    pub const BTL_END_AP: Global<[u32; 2]> = Global::new(0x0186_9f10);

    /// The battle-end counting step for `ap`: `ap / 189 | 1`, moved off a
    /// multiple of 5 by adding 2.
    #[must_use]
    pub const fn btl_end_ap_step(ap: u32) -> u32 {
        let s = (ap / 0xbd) | 1;
        if s.is_multiple_of(5) { s + 2 } else { s }
    }

    /// `BtlEnd_InitApStep()` (`0x008bdf10`, own name): the AP total from
    /// result 19 and its step.
    pub fn btl_end_init_ap_step(mem: &mut impl Mem) {
        let ap = btl_end_get_result(mem, 19);
        BTL_END_AP.set(mem, [btl_end_ap_step(ap), ap]);
    }

    /// `BtlEnd_ApStep(chr)` (`0x008ba3c0`, own name): the step for the
    /// character's `MsGetNextAP`.
    pub fn btl_end_ap_step_for(mem: &mut impl Mem, chr: i32) -> u32 {
        btl_end_ap_step(ffx_battle::menu::bridge::ms_get_next_ap(mem, chr).cast_unsigned())
    }

    /// The battle-end list: count `u8` and up to three 14-byte entries.
    pub const BTL_END_LIST: Global<u8> = Global::new(0x0186_9f18);

    /// `BtlEnd_CompactList()` (`0x008ba400`, own name): entries whose byte
    /// `+9` is set move to the front (swapped, their `u16` `+2` staying in
    /// place), at most three.
    #[allow(clippy::many_single_char_names)]
    pub fn btl_end_compact_list(mem: &mut impl Mem) {
        let n = i32::from(BTL_END_LIST.get(mem));
        let base = BTL_END_LIST.ptr(mem).addr + 8;
        let mut dest = base;
        for k in 0..n.max(0).cast_unsigned() {
            if dest + 2 > base + 0x1e {
                return;
            }
            let e = base + 0xe * k;
            if mem.read::<u8>(e + 9) != 0 {
                let d = [
                    mem.read::<u32>(dest),
                    mem.read::<u32>(dest + 4),
                    mem.read::<u32>(dest + 8),
                ];
                let d3 = mem.read::<u16>(dest + 0xc);
                let v = mem.read::<u32>(e);
                mem.write(dest, v);
                let v = mem.read::<u32>(e + 4);
                mem.write(dest + 4, v);
                let v = mem.read::<u32>(e + 8);
                mem.write(dest + 8, v);
                let v = mem.read::<u16>(e + 0xc);
                mem.write(dest + 0xc, v);
                mem.write(e, d[0]);
                mem.write(e + 4, d[1]);
                mem.write(e + 8, d[2]);
                mem.write(e + 0xc, d3);
                let a = mem.read::<u16>(e + 2);
                let b = mem.read::<u16>(dest + 2);
                mem.write(dest + 2, a);
                mem.write(e + 2, b);
                dest += 0xe;
            }
        }
    }

    /// `MenuMn2_Ease(t, dir)` (`0x008d30c0`, own name): the low half of
    /// `Math_CosLookup(t)`, or for a negative `dir` `0x1000` minus that of
    /// `0x1000 - t`.
    pub fn menu_mn2_ease(mem: &impl Mem, t: i32, dir: i32) -> u32 {
        if dir < 0 {
            0x1000u32.wrapping_sub(math_cos_lookup(mem, 0x1000i32.wrapping_sub(t)) & 0xffff)
        } else {
            math_cos_lookup(mem, t) & 0xffff
        }
    }

    /// The status menu's rectangle size override (`MenuStatus_OffsetRect`).
    pub const STATUS_RECT_SIZE: Global<[u16; 2]> = Global::new(0x0186_a8a4);

    /// `MenuStatus_OffsetRect(dx, dy, out, rect, sized)` (`0x008d3220`, own
    /// name): the 24-byte rectangle copied, moved by (`dx`, `dy`), and with
    /// `sized` given the override size.
    pub fn menu_status_offset_rect(
        mem: &mut impl Mem,
        dx: u16,
        dy: u16,
        out: u32,
        rect: u32,
        sized: bool,
    ) {
        for k in 0..6 {
            let v = mem.read::<u32>(rect.wrapping_add(4 * k));
            mem.write(out.wrapping_add(4 * k), v);
        }
        let v = mem.read::<u16>(out).wrapping_add(dx);
        mem.write(out, v);
        let v = mem.read::<u16>(out.wrapping_add(2)).wrapping_add(dy);
        mem.write(out.wrapping_add(2), v);
        if sized {
            let [w, h] = STATUS_RECT_SIZE.get(mem);
            mem.write(out.wrapping_add(4), w);
            mem.write(out.wrapping_add(6), h);
        }
    }

    /// The status menu's origin (`u16` x, y).
    pub const STATUS_ORIGIN: Global<[u16; 2]> = Global::new(0x0186_a86c);

    /// `MenuStatus_GetPos(out, alt)` (`0x008d4e30`) and `MenuStatus_GetPos2`
    /// (`0x008d4e80`, own names): one of two `u16` positions (`a` or `b`
    /// with `alt`) plus the status origin.
    pub fn menu_status_get_pos(mem: &mut impl Mem, out: u32, alt: bool, pos: u32, alt_pos: u32) {
        let p = mem.rebase(if alt { alt_pos } else { pos });
        let [ox, oy] = STATUS_ORIGIN.get(mem);
        let x = mem.read::<u16>(p).wrapping_add(ox);
        let y = mem.read::<u16>(p + 2).wrapping_add(oy);
        mem.write(out, x);
        mem.write(out.wrapping_add(2), y);
    }

    /// `MenuHdd_Close()` (`0x008e2930`, own name): the HDD menu's state
    /// cleared, then the main task's callback `+8` (if any task) with the
    /// close routine `0x008e2910`.
    pub fn menu_hdd_close<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let at = mem.rebase(0x023c_c130);
        mem.write(at, 0u32);
        let task = MENU_MAIN_TASK.get(mem);
        if task != 0 {
            let f = mem.read::<u32>(task + 8);
            let close = mem.rebase(0x008e_2910);
            call(mem, f, &[close, 0, 0]);
        }
    }

    /// `MenuSaveLoad_BuildCrcTable(table)` (`0x008b1370`, own name): `u16`
    /// CRC-16 (`0x1021`) entries of `i << 9`, the top bit of `i` folded in
    /// first, eight steps in all (the `TkCRC` table's variant); like that
    /// one it stops at 254, leaving entry 255 untouched.
    pub fn menu_save_load_build_crc_table(mem: &mut impl Mem, table: u32) {
        for i in 0u32..0xff {
            let mut v = if i & 0x80 == 0 {
                i << 9
            } else {
                (i << 9) ^ 0x1021
            };
            for _ in 0..7 {
                v = if v & 0x8000 == 0 {
                    v << 1
                } else {
                    (v << 1) ^ 0x1021
                };
            }
            #[allow(clippy::cast_possible_truncation)]
            mem.write(table.wrapping_add(2 * i), v as u16);
        }
    }

    /// `MenuDraw_GifTag(packet, a, b, c)` (`0x008de0a0`, own name): a
    /// 64-byte GIF header (`a`, `b`, `c` at `+0x30`); the end back.
    pub fn menu_draw_gif_tag(mem: &mut impl Mem, p: u32, a: u32, b: u32, c: u32) -> u32 {
        let words = [
            1u32,
            0x1000_0000,
            0xe,
            0,
            1,
            0,
            0x14,
            0,
            0x8001,
            0x2400_0000,
            0x16,
            0,
            a,
            b,
            c,
            0,
        ];
        mem.write(p, words);
        p.wrapping_add(0x40)
    }

    /// `TkMenuDrawInitString()` (`0x008dce30`): both menu packet pointers
    /// from `Sg_PacketGetPtr` (after the no-op `TkVU1SyncPath`).
    pub fn tk_menu_draw_init_string(mem: &mut impl Mem) {
        let p = ffx_sg::state::bridge::packet_get_ptr(mem);
        mem.write(mem.rebase(0x0187_152c), p);
        MENU_PACKET.set(mem, p);
    }

    /// `TkMn2DrawMenuWindowScissor(y, h, mode, ratio)` (`0x008c0f70`): a
    /// full-width scissor; mode 1 shrinks it to `|ratio| × h / 4096` (minus
    /// 2, not negative) from the bottom (top for a negative ratio), mode 2
    /// cuts that amount off; returns the scissor's bottom.
    pub fn tk_mn2_draw_menu_window_scissor(
        mem: &mut impl Mem,
        y: i32,
        h: i32,
        mode: i32,
        ratio: i32,
    ) -> u32 {
        let part = |h: i32| {
            let v = ratio.wrapping_abs().wrapping_mul(h);
            v.wrapping_add((v >> 31) & 0xfff) >> 12
        };
        let (y, h) = match mode {
            1 => {
                let q = part(h);
                let y = if ratio < 0 {
                    y
                } else {
                    y.wrapping_sub(q).wrapping_add(h)
                };
                let hh = q.wrapping_sub(2);
                (y, if hh < 0 { 0 } else { hh })
            }
            2 => {
                let q = part(h);
                let y = if ratio < 0 {
                    y.wrapping_add(2).wrapping_add(q)
                } else {
                    y
                };
                let hh = h.wrapping_sub(q);
                (y, if hh < 0 { 0 } else { hh })
            }
            _ => (y, h),
        };
        ffx_system::pkt::bridge::draw_scissor_xywh(mem, 0, y, 0x200, h.wrapping_sub(1))
    }

    /// The weapon shop's table (`MenuWShop_GetRecord`).
    pub const WSHOP_TABLE: Global<u32> = Global::new(0x0186_abac);

    /// `MenuWShop_GetRecord(id)` (`0x008d9170`, own name): the record of
    /// the 12-bit id in the shop table.
    pub fn menu_wshop_get_record(mem: &impl Mem, id: u32) -> u32 {
        let t = WSHOP_TABLE.get(mem);
        #[allow(clippy::cast_possible_wrap)]
        tk_mn2_get_excel_data(mem, (id & 0xfff) as i32, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_differs_from_ccitt_only_through_entry_255() {
        let t = crc_table();
        assert_eq!(t[1], 0x1021);
        assert_eq!(t[0xff], 0);
        // CRC-16/CCITT-FALSE("123456789") = 0x29b1 never indexes entry 255
        assert_eq!(!crc(b"123456789"), 0x29b1);
    }

    #[test]
    fn buttons_and_vibration() {
        let mut row = [0u8; 16];
        for (i, r) in row.iter_mut().enumerate() {
            *r = 15 - u8::try_from(i).unwrap();
        }
        row[3] = 20; // out of the 16 bits
        assert_eq!(map_buttons(row, 0b1001), 0x8000);
        assert_eq!(vibrate_mask(0, 3, true), 8);
        assert_eq!(vibrate_mask(0xff, 3, false), 0xf7);
        assert_eq!(vibrate_mask(5, -1, true), 0xff);
        assert_eq!(vibrate_mask(5, 16, true), 5);
        let recs = [(1u8, 2u8), (3, 4), (1, 2)];
        assert_eq!(command_offset(recs.into_iter(), 3, 4), 1);
        assert_eq!(command_offset(recs.into_iter(), 9, 9), 0);
        assert_eq!(command_count(recs.into_iter(), 1, 2), 2);
    }
}
