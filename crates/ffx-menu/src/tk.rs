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

/// The GS register packers (`Gs_Pack4` and copies): `(a, b, c, d)` as the
/// `u64` `a | b << 32 | c << 16 | d << 48` (the `or`s overlapping).
#[must_use]
pub const fn gs_pack4(a: u32, b: u32, c: u32, d: u32) -> u64 {
    (a as u64) | ((b as u64) << 32) | ((c as u64) << 16) | ((d as u64) << 48)
}

/// `Gs_Pack5(a, b, c, d, e)` and copies: [`gs_pack4`] with `e & 0x0ffffff0`
/// folded in above `c` and `d` (`<< 12`).
#[must_use]
#[allow(clippy::many_single_char_names)]
pub const fn gs_pack5(a: u32, b: u32, c: u32, d: u32, e: u32) -> u64 {
    let e = e & 0x0fff_fff0;
    gs_pack4(a, b, (e << 12) | c, (e >> 20) | d)
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

    /// The overdrive modes in list order (`TkMn2BuildItemList_008c2390`
    /// kind 1): 20 bit numbers of `TkMn2GetOverDriveType`'s mask.
    pub const MN2_OVERDRIVE_ORDER: Global<[u8; 20]> = Global::new(0x00c8_765c);

    /// `TkMn2BuildItemList_008c2390(kind, arg)` (`0x008c2390`): fills
    /// [`MENU_ITEM_LIST`] (cleared with [`MENU_ITEM_COUNTS`]) by `kind`,
    /// then [`MN2_LIST_MAX`] = entries written and [`MN2_NAME_MODE_MAX`] =
    /// entries shown (at least 1). Entries are `{u16 id, u8 kind, u8
    /// value}`:
    /// - 0 the current character's commands (`TkMenuGetCommandMax/Offset
    ///   (MsMenuStatusNum(0), 5, chr, 0)`, owned ones `0x101`; a missing one
    ///   with a menu panel leaves a gap unless `chr` is 0, 2 or 4), or for
    ///   `chr` 1 the aeons of `TkMenuSummonEnableMask` (8..15, `0x102`);
    ///   2 the same for the current aeon (never a gap);
    /// - 1 the overdrive modes of [`MN2_OVERDRIVE_ORDER`] in
    ///   `TkMn2GetOverDriveType(chr)` (`0x103`);
    /// - 3 the aeon grow list (`TkMn2GetSummonGrow`, 8-byte records: mask
    ///   byte, `u16` command, `u16` item, needed count): learnt 5, too few
    ///   items 7, allowed for the aeon (mask bit `aeon - 8`) or with key
    ///   item `0xa022` 4, else 6; value = record index; no items, no entry
    ///   (its `+3` keeps the `0xff` written first);
    /// - 5 the weapon customisations (`TkMn2GetKaizou`, same record shape,
    ///   mask 2 for armour `arg + 5`, else 1) against `arg`'s abilities
    ///   (`u16` at `+0xe`, `MsGetRomAbility` group `+0x69`, level `+0x6a`,
    ///   kind `+0x6b`): 0xb, 0xe too few items, 0xc same, 0xf higher or
    ///   blocked, 0x10 no empty slot (`MenuList_IsEmptySlot(arg)`); the
    ///   ones without items follow as `0xff11_0000` (listed, not shown);
    /// - 6/7 the inventory (`MsSaveItemTypeBuffer/NumBuffer`), each slot
    ///   in place, usable ones (always for 7) kind `0x12`, value 1;
    /// - `0x13`/`0x15` every weapon (`MsGetSaveWeaponList(-1, -1, 0)`):
    ///   kind 1, value 1 when flags `+3 & 0xd` are 0 and `+6` is `0xff`;
    ///   unowned or flag 2 zero (`0x13`) or the id with 0 (`0x15`); shown up
    ///   to the last owned one;
    /// - `0x14` the current character's weapons of kind `arg`, `0x101`, and
    ///   for `arg` 1 an empty `0x0100_0000` entry.
    ///
    /// `local` is 12 bytes of out slots (a count, then a discarded word).
    pub fn tk_mn2_build_item_list<M: Mem>(
        mem: &mut M,
        kind: u32,
        arg: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let out = local.wrapping_add(8);
        let f = fun(mem, 0x0079_0200);
        let commands = call(mem, f, &[0, out]);
        let f = fun(mem, 0x008a_9860);
        let player = call(mem, f, &[]);
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let counts = MENU_ITEM_COUNTS.ptr(mem).addr;
        mem.write(list, [0u32; 0x200]);
        mem.write(counts, [0u16; 0x200]);
        MN2_LIST_MAX.set(mem, 0);
        let mut e = list;
        let shown = match kind {
            0 | 2 => {
                let (who, mut whole) = if kind == 2 {
                    let f = fun(mem, 0x008a_9880);
                    (call(mem, f, &[]), true)
                } else {
                    (player, false)
                };
                if who == 1 {
                    let f = fun(mem, 0x008a_b1e0);
                    let mask = call(mem, f, &[]);
                    for c in 8..16 {
                        if mask & (1 << c) != 0 {
                            put(mem, &mut e, c, 2, 1);
                        }
                    }
                } else {
                    whole |= matches!(who, 0 | 2 | 4);
                    let f = fun(mem, 0x008a_8e20);
                    call(mem, f, &[]);
                    let f = fun(mem, 0x008d_2d80);
                    let max = call(mem, f, &[commands, 5, who, 0]).cast_signed();
                    let f = fun(mem, 0x008d_2dc0);
                    let off = call(mem, f, &[commands, 5, who, 0]);
                    let mut at = commands.wrapping_add(off.wrapping_mul(4)).wrapping_add(2);
                    for _ in 0..max.max(0) {
                        let id = u32::from(mem.read::<u16>(at));
                        let f = fun(mem, 0x0078_5020);
                        if call(mem, f, &[who, id]) != 0 {
                            put(mem, &mut e, id, 1, 1);
                        } else if !whole {
                            let f = fun(mem, 0x0078_50b0);
                            if call(mem, f, &[id]) != 0 {
                                e = e.wrapping_add(4);
                            }
                        }
                        at = at.wrapping_add(4);
                    }
                }
                e
            }
            1 => {
                let f = fun(mem, 0x008c_1bf0);
                let mask = call(mem, f, &[player]);
                let order = MN2_OVERDRIVE_ORDER.get(mem);
                for b in order {
                    if mask & (1 << (b & 0x1f)) != 0 {
                        put(mem, &mut e, u32::from(b), 3, 1);
                    }
                }
                e
            }
            3 => {
                build_grow_rows(mem, &mut e, call);
                e
            }
            5 => build_kaizou_rows(mem, arg, out, &mut e, call),
            6 | 7 => build_inventory_rows(mem, kind == 7, local, &mut e, call),
            0x13 | 0x15 => build_weapon_rows(mem, kind == 0x13, local, &mut e, call),
            0x14 => {
                let f = fun(mem, 0x008a_9860);
                let chr = call(mem, f, &[]);
                let f = fun(mem, 0x007a_bd10);
                let ids = call(mem, f, &[chr, arg, 0, local]);
                let get = fun(mem, 0x007a_bbd0);
                let mut k = 0u32;
                while k.cast_signed() < mem.read::<i32>(local) {
                    let id = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
                    let rec = call(mem, get, &[id, out]);
                    if mem.read::<u8>(rec.wrapping_add(2)) != 0 {
                        put(mem, &mut e, id, 1, 1);
                    }
                    k += 1;
                }
                if arg == 1 {
                    mem.write(e, 0x0100_0000u32);
                    e = e.wrapping_add(4);
                }
                e
            }
            _ => list,
        };
        MN2_LIST_MAX.set(mem, e.wrapping_sub(list) >> 2);
        let shown = shown.wrapping_sub(list) >> 2;
        MN2_NAME_MODE_MAX.set(mem, if shown == 0 { 1 } else { shown });
    }

    /// `TkMn2BuildItemList_008c2390` kinds 6 and 7: the inventory, each
    /// slot in place (`all`: kind 7, every item usable).
    fn build_inventory_rows<M: Mem>(
        mem: &mut M,
        all: bool,
        local: u32,
        e: &mut u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let out = local.wrapping_add(8);
        let f = fun(mem, 0x0079_0530);
        let ids = call(mem, f, &[local]);
        let f = fun(mem, 0x0079_0510);
        let nums = call(mem, f, &[local]);
        let mut k = 0u32;
        while k.cast_signed() < mem.read::<i32>(local) {
            let id = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
            let n: u8 = mem.read(nums.wrapping_add(k));
            let f = fun(mem, 0x008a_b280);
            let rec = call(mem, f, &[id, out]);
            if n != 0 {
                let usable = item_usable_by_call(mem, rec, call) || all;
                if usable {
                    mem.write(e.wrapping_add(2), 0x12u8);
                }
                #[allow(clippy::cast_possible_truncation)] // a u16 id
                mem.write(*e, id as u16);
                mem.write(e.wrapping_add(3), u8::from(usable));
            }
            *e = e.wrapping_add(4);
            k += 1;
        }
        *e
    }

    /// `TkMn2BuildItemList_008c2390` kinds `0x13` and `0x15`: every weapon
    /// (`zero`: kind `0x13`, unowned ones cleared); returns the end of the
    /// rows shown.
    fn build_weapon_rows<M: Mem>(
        mem: &mut M,
        zero: bool,
        local: u32,
        e: &mut u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let out = local.wrapping_add(8);
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let f = fun(mem, 0x007a_bd10);
        let ids = call(mem, f, &[u32::MAX, u32::MAX, 0, local]);
        let get = fun(mem, 0x007a_bbd0);
        let mut k = 0u32;
        while k.cast_signed() < mem.read::<i32>(local) {
            let id = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
            let rec = call(mem, get, &[id, out]);
            if id != 0 {
                let flags: u8 = mem.read(rec.wrapping_add(3));
                if mem.read::<u8>(rec.wrapping_add(2)) == 0 || flags & 2 != 0 {
                    if zero {
                        mem.write(*e, 0u32);
                    } else {
                        #[allow(clippy::cast_possible_truncation)] // a u16 id
                        mem.write(*e, id as u16);
                        mem.write(e.wrapping_add(2), 0u16);
                    }
                } else {
                    let plain = flags & 0xd == 0 && mem.read::<u8>(rec.wrapping_add(6)) == 0xff;
                    #[allow(clippy::cast_possible_truncation)] // a u16 id
                    mem.write(*e, id as u16);
                    mem.write(e.wrapping_add(3), u8::from(plain));
                    mem.write(e.wrapping_add(2), 1u8);
                }
            }
            *e = e.wrapping_add(4);
            k += 1;
        }
        // shown up to the last owned weapon (at least one row)
        let mut end_row = mem.read::<i32>(local).wrapping_sub(1);
        while end_row >= 0 {
            let at = ids.wrapping_add(2 * end_row.cast_unsigned());
            let id = u32::from(mem.read::<u16>(at));
            let rec = call(mem, get, &[id, out]);
            if mem.read::<u8>(rec.wrapping_add(2)) != 0 {
                end_row += 1;
                break;
            }
            end_row -= 1;
        }
        if end_row <= 0 {
            mem.write(list, 0u32);
            end_row = 1;
        }
        list.wrapping_add(end_row.cast_unsigned().wrapping_mul(4))
    }

    /// `TkMn2RefreshList(kind)` (`0x008c2360`):
    /// `TkMn2BuildItemList_008c2390(kind, 0)`.
    pub fn tk_mn2_refresh_list<M: Mem>(
        mem: &mut M,
        kind: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(0x008c_2390);
        call(mem, f, &[kind, 0]);
    }

    /// The weapon list's length (`MenuWeapon_BuildKindList`).
    pub const MENU_WEAPON_KIND_COUNT: Global<u32> = Global::new(0x0186_a620);

    /// `MenuWeapon_BuildKindList(_, kind)` (`0x008cfd20`): the current
    /// character's weapons of `kind` (`TkMn2BuildItemList_008c2390(0x14,
    /// kind)` through its thunk `0x008c2380`), their count kept in
    /// [`MENU_WEAPON_KIND_COUNT`] and returned.
    pub fn menu_weapon_build_kind_list<M: Mem>(
        mem: &mut M,
        kind: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008c_2380);
        call(mem, f, &[0x14, kind]);
        let f = mem.rebase(0x008c_1bb0);
        let n = call(mem, f, &[]);
        MENU_WEAPON_KIND_COUNT.set(mem, n);
        n
    }

    /// `MenuKaizou_BuildList(k)` (`0x008d6dd0`): the customisations for
    /// weapon `k` of the customise menu's list (`u16` ids at
    /// `[0x0186aa38]`): `TkMn2BuildItemList_008c2390(5,
    /// MsGetSaveWeapon(id, local))` through the thunk `0x008c2380`.
    pub fn menu_kaizou_build_list<M: Mem>(
        mem: &mut M,
        k: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let list: u32 = mem.read(mem.rebase(0x0186_aa38));
        let id = u32::from(mem.read::<u16>(list.wrapping_add(k.wrapping_mul(2))));
        let f = mem.rebase(0x007a_bbd0);
        let rec = call(mem, f, &[id, local]);
        let f = mem.rebase(0x008c_2380);
        call(mem, f, &[5, rec]);
    }

    /// `Pad_InitWorks()` (`0x00889340`): `Pad_InitWork(port, 0)` for ports
    /// 0 and 1.
    pub fn pad_init_works<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let f = mem.rebase(0x0088_9360);
        for port in 0..2 {
            call(mem, f, &[port, 0]);
        }
    }

    /// `MenuConfig_DrawRows()` (`0x008cbab0`): the menu background
    /// (`TODrawMenuBG`), then each of [`MENU_CONFIG_ROWS`] rows
    /// (`MenuConfig_DrawOptionRow(0, 0, row)`, `TkMn2DrawKickSyncPacket2()`
    /// after each); the last call's `eax`.
    pub fn menu_config_draw_rows<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008f_5c10);
        let mut r = call(mem, f, &[]);
        let mut row = 0u32;
        while row.cast_signed() < MENU_CONFIG_ROWS.get(mem) {
            let f = mem.rebase(0x008c_bae0);
            call(mem, f, &[0, 0, row]);
            let f = mem.rebase(0x008c_0cd0);
            r = call(mem, f, &[]);
            row += 1;
        }
        r
    }

    /// `BtlEnd_AddAP_008bdaf0(chr, ap, mark)` (`0x008bdaf0`): when `ap`
    /// on top of the AP saved (`BtlEnd_GetSaveNextAP`) reaches the next
    /// level (`BtlEnd_GetNextAP`), the level-up mark
    /// (`TOBtlEndSetFlagMark(chr, mark)`), `ap` cut to what is missing
    /// when the sphere level (`BtlEnd_GetSP`) is 98; then
    /// `0x008ba390(chr, ap)` and its `eax`.
    pub fn btl_end_add_ap<M: Mem>(
        mem: &mut M,
        chr: u32,
        ap: u32,
        mark: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let f = fun(mem, 0x008b_da00);
        let saved = call(mem, f, &[chr]);
        let f = fun(mem, 0x008b_da40);
        let next = call(mem, f, &[chr]).cast_signed();
        let mut ap = ap;
        if next <= saved.wrapping_add(ap).cast_signed() {
            let f = fun(mem, 0x008b_da50);
            if call(mem, f, &[chr]) == 0x62 {
                ap = next.cast_unsigned().wrapping_sub(saved);
            }
            // saved + ap still reaches next
            let f = fun(mem, 0x008b_e020);
            call(mem, f, &[chr, mark]);
        }
        let f = fun(mem, 0x008b_a390);
        call(mem, f, &[chr, ap])
    }

    /// The battle-end message about a celestial weapon
    /// (`TkBtlEndCheckLegendWeapon`).
    pub const BTL_END_LEGEND_TEXT: Global<u32> = Global::new(0x025c_c590);

    /// `TkBtlEndCheckLegendWeapon()` (`0x008ba5c0`): counts the six weapon
    /// spoil slots (`u16` at `0x02310f8e`) whose `MsGetSaveWeapon` record
    /// is in use (`+2`) with flags `+3 & 0xd`; for exactly one, its name
    /// (`MsGetSaveWeaponName`) as macro 7's value and text `0x401b`, else
    /// text `0x401c` (`TkBtlEndGetText`) into [`BTL_END_LEGEND_TEXT`];
    /// returns the count. `local` is `MsGetSaveWeapon`'s out word.
    pub fn tk_btl_end_check_legend_weapon<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let slots = fun(mem, 0x0231_0f8e);
        let (mut n, mut name) = (0u32, 0u32);
        for k in 0..6 {
            let id = u32::from(mem.read::<u16>(slots.wrapping_add(2 * k)));
            let f = fun(mem, 0x007a_bbd0);
            let rec = call(mem, f, &[id, local]);
            if mem.read::<u8>(rec.wrapping_add(2)) != 0
                && mem.read::<u8>(rec.wrapping_add(3)) & 0xd != 0
            {
                n += 1;
                let f = fun(mem, 0x007a_bdf0);
                name = call(mem, f, &[id]);
            }
        }
        let text = if n == 1 {
            let f = fun(mem, 0x008b_8960);
            call(mem, f, &[7, 0, 0]);
            let f = fun(mem, 0x008b_8980);
            call(mem, f, &[7, 0, name]);
            let f = fun(mem, 0x008b_8680);
            call(mem, f, &[7]);
            0x401b
        } else {
            0x401c
        };
        let f = fun(mem, 0x008b_da60);
        let t = call(mem, f, &[text]);
        BTL_END_LEGEND_TEXT.set(mem, t);
        n
    }

    /// One `{u16 id, u8 kind, u8 value}` list entry at `*e`, then the next.
    fn put(mem: &mut impl Mem, e: &mut u32, id: u32, kind: u8, value: u8) {
        #[allow(clippy::cast_possible_truncation)] // a u16 id
        mem.write(*e, id as u16);
        mem.write(e.wrapping_add(2), kind);
        mem.write(e.wrapping_add(3), value);
        *e = e.wrapping_add(4);
    }

    /// [`item_usable`] with `TkMenuSummonEnableMask` called (only when
    /// `+0x22` has bit 7 without bit 6).
    fn item_usable_by_call<M: Mem>(
        mem: &mut M,
        rec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        if mem.read::<u8>(rec.wrapping_add(0x1c)) & 1 == 0 {
            return false;
        }
        let f = mem.read::<u8>(rec.wrapping_add(0x22));
        if f & 0x80 == 0 || f & 0x40 != 0 {
            return true;
        }
        let mask = mem.rebase(0x008a_b1e0);
        call(mem, mask, &[]) != 0
    }

    /// `TkMn2BuildItemList_008c2390` kind 3: the current aeon's grow list.
    fn build_grow_rows<M: Mem>(
        mem: &mut M,
        e: &mut u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let f = fun(mem, 0x0079_0800);
        let important = call(mem, f, &[0xa022]);
        let f = fun(mem, 0x008a_9880);
        let summon = call(mem, f, &[]);
        let f = fun(mem, 0x008c_1c40);
        let max = call(mem, f, &[]).cast_signed();
        let f = fun(mem, 0x008c_1c20);
        let table = call(mem, f, &[]);
        #[allow(clippy::cast_possible_truncation)] // the byte test of `1 << n`
        let bit = (1u32 << (summon.wrapping_sub(8) & 0x1f)) as u8;
        for k in 0..max.max(0).cast_unsigned() {
            let rec = table.wrapping_add(8 * k);
            let id = u32::from(mem.read::<u16>(rec.wrapping_add(2)));
            mem.write(e.wrapping_add(3), 0xffu8);
            let f = fun(mem, 0x0078_5020);
            let row = if call(mem, f, &[summon, id]) == 0 {
                let item = u32::from(mem.read::<u16>(rec.wrapping_add(4)));
                let f = fun(mem, 0x0079_04b0);
                let n = call(mem, f, &[item]).cast_signed();
                if n == 0 {
                    None
                } else if n < i32::from(mem.read::<u8>(rec.wrapping_add(6))) {
                    Some(7)
                } else if mem.read::<u8>(rec) & bit != 0 || important != 0 {
                    Some(4)
                } else {
                    Some(6)
                }
            } else {
                Some(5)
            };
            if let Some(kind) = row {
                #[allow(clippy::cast_possible_truncation)] // the index byte
                put(mem, e, id, kind, k as u8);
            }
        }
    }

    /// `TkMn2BuildItemList_008c2390` kind 5: the customisations for the
    /// piece at `gear`; returns the end of the rows shown.
    fn build_kaizou_rows<M: Mem>(
        mem: &mut M,
        gear: u32,
        out: u32,
        e: &mut u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let f = fun(mem, 0x008c_1b70);
        let table = call(mem, f, &[]);
        let f = fun(mem, 0x008c_1b90);
        let max = call(mem, f, &[]).cast_signed();
        let mask: u8 = if mem.read::<u8>(gear.wrapping_add(5)) != 0 {
            2
        } else {
            1
        };
        // the piece's abilities: group, level, kind (0xff: any customisation
        // of kind 0xfe is blocked)
        let mut have = [[0u8; 3]; 4];
        let mut n_have = 0;
        let mut wild = false;
        for s in 0..4 {
            let a = u32::from(mem.read::<u16>(gear.wrapping_add(0xe + 2 * s)));
            if a == 0 || a == 0xff {
                continue;
            }
            let f = fun(mem, 0x0079_0970);
            let rec = call(mem, f, &[a, out]);
            let g: [u8; 3] = mem.read(rec.wrapping_add(0x69));
            wild |= g[2] == 0xff;
            have[n_have] = g;
            n_have += 1;
        }
        let mut missing = 0u32;
        for k in 0..max.max(0).cast_unsigned() {
            let row = table.wrapping_add(8 * k);
            if mem.read::<u8>(row) & mask == 0 {
                continue;
            }
            let ability = u32::from(mem.read::<u16>(row.wrapping_add(2)));
            let f = fun(mem, 0x0079_0970);
            let rec = call(mem, f, &[ability, out]);
            let item = u32::from(mem.read::<u16>(row.wrapping_add(4)));
            let f = fun(mem, 0x0079_04b0);
            let n = call(mem, f, &[item]).cast_signed();
            if n == 0 {
                missing += 1;
                continue;
            }
            let [group, level, kind]: [u8; 3] = mem.read(rec.wrapping_add(0x69));
            let mut row_kind = 0xbu8;
            for g in &have[..n_have] {
                if g[0] == group {
                    if g[1] > level {
                        row_kind = 0xf;
                    }
                    if g[1] == level {
                        row_kind = if g[2] == kind { 0xc } else { 0xf };
                    }
                }
                if wild && kind == 0xfe {
                    row_kind = 0xf;
                }
            }
            if row_kind == 0xb && i32::from(mem.read::<u8>(row.wrapping_add(6))) > n {
                row_kind = 0xe;
            }
            let f = fun(mem, 0x008b_f720);
            if call(mem, f, &[gear]) == 0 {
                row_kind = 0x10;
            }
            #[allow(clippy::cast_possible_truncation)] // the index byte
            put(mem, e, ability, row_kind, k as u8);
        }
        // the rows without items are listed, not shown
        let shown = *e;
        for _ in 0..missing {
            mem.write(*e, 0xff11_0000u32);
            *e = e.wrapping_add(4);
        }
        shown
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

    /// `TkMenuRefreshSubCmdHelp()` (`0x008e49b0`): the sub-command window's
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
    /// `g_TkMenuItemTarget`: the character the item menu targets, negative
    /// for none (`MenuItem_SetTarget`; the name-entry window writes it too).
    pub const MENU_ITEM_TARGET: Global<u32> = Global::new(0x0186_a258);
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
        if list == 4 || (list == 3 && MENU_ITEM_TARGET.get(mem).cast_signed() >= 0) {
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

    /// The battle results' gil count-down: the step, then the gil still to
    /// count (`g_TkBtlEndGil`; once named AP, but [`btl_end_step_gil`] pays
    /// each step with `MsPayGIL(-step)` and the total is result 19, the
    /// gil won, which `0x00857bf2` and `0x0085a9ec` pay the same way).
    pub const BTL_END_GIL: Global<[u32; 2]> = Global::new(0x0186_9f10);

    /// The battle-end counting step for a total `n` (gil or AP): `n / 189 | 1`,
    /// moved off a multiple of 5 by adding 2.
    #[must_use]
    pub const fn btl_end_count_step(n: u32) -> u32 {
        let s = (n / 0xbd) | 1;
        if s.is_multiple_of(5) {
            s.wrapping_add(2)
        } else {
            s
        }
    }

    /// `BtlEnd_InitGilStep()` (`0x008bdf10`, own name; was `BtlEnd_InitApStep`):
    /// the gil won (result 19) as the count-down total, and its step.
    pub fn btl_end_init_gil_step(mem: &mut impl Mem) {
        let gil = btl_end_get_result(mem, 19);
        BTL_END_GIL.set(mem, [btl_end_count_step(gil), gil]);
    }

    /// `BtlEnd_ApStep(chr)` (`0x008ba3c0`, own name): the step for the
    /// character's `MsGetNextAP`.
    pub fn btl_end_ap_step_for(mem: &mut impl Mem, chr: i32) -> u32 {
        btl_end_count_step(ffx_battle::menu::bridge::ms_get_next_ap(mem, chr).cast_unsigned())
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
    /// from `Sg_PacketGetPtr` (after the no-op `TkVU1SyncPath`); the
    /// pointer back (left in `eax`).
    pub fn tk_menu_draw_init_string(mem: &mut impl Mem) -> u32 {
        let p = ffx_sg::state::bridge::packet_get_ptr(mem);
        mem.write(mem.rebase(0x0187_152c), p);
        MENU_PACKET.set(mem, p);
        p
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

    /// `TkMenuMainFreeWindow(window)` (`0x008aa3f0`): reset, unused
    /// (`+0x40`), idle.
    pub fn tk_menu_main_free_window(mem: &mut impl Mem, w: u32) {
        menu_main_reset_window(mem, w);
        mem.write(w.wrapping_add(0x40), 0u8);
        mem.write(w.wrapping_add(0x28), 0u32);
    }

    /// The menu's 32 window records (`0x98` bytes each).
    pub const MENU_WINDOWS: Global<u8> = Global::new(0x0184_0900);

    /// `MenuMain_ResetAllWindows()` (`0x008aa470`, own name): every window
    /// record reset.
    pub fn menu_main_reset_all_windows(mem: &mut impl Mem) {
        let base = MENU_WINDOWS.ptr(mem).addr;
        for k in 0..0x20 {
            menu_main_reset_window(mem, base + k * 0x98);
        }
    }

    /// `TkMenuDrawKick()` (`0x008dce50`): the full-screen scissor, then
    /// `TkMenuDrawInitString`; the packet pointer back.
    pub fn tk_menu_draw_kick(mem: &mut impl Mem) -> u32 {
        ffx_system::pkt::bridge::draw_scissor_xywh(mem, 0, 0, 0x200, 0x1a0);
        tk_menu_draw_init_string(mem)
    }

    /// `Msg_CancelChoice(worker, slot)` (`0x00864150`, own name): after
    /// `AtelGetBasicWork(worker)` (for its cache), choice slot `slot` (0..3) of the current script context: its pending answer
    /// `+0x64` = -1, `+0x68` = 0, `+0x73` = 0; a pending one (not negative)
    /// first writes the default byte `+0x72` to its target `+0x6c`.
    pub fn msg_cancel_choice(mem: &mut impl Mem, worker: i32, slot: i32) {
        ffx_script::vm::bridge::atel_get_basic_work(mem, worker);
        let ctx = mem.read::<u32>(mem.rebase(0x0132_6b28));
        let s = ctx.wrapping_add(slot.clamp(0, 3).cast_unsigned() << 4);
        let old = mem.read::<i32>(s.wrapping_add(0x64));
        mem.write(s.wrapping_add(0x64), u32::MAX);
        mem.write(s.wrapping_add(0x68), 0u32);
        mem.write(s.wrapping_add(0x73), 0u8);
        if old >= 0 {
            let to = mem.read::<u32>(s.wrapping_add(0x6c));
            let v = mem.read::<u8>(s.wrapping_add(0x72));
            mem.write(to, v);
        }
    }

    /// `TkSetDebugFontBuffer(p)` (`0x0088fab0`): the debug font's current
    /// pointer; past its end (`+0xc`) the original only logs. Returns the
    /// end (left in `eax`).
    pub fn tk_set_debug_font_buffer(mem: &mut impl Mem, p: u32) -> u32 {
        let buf = DEBUG_FONT_BUFFER.get(mem);
        mem.write(buf, p);
        mem.read::<u32>(buf.wrapping_add(0xc))
    }

    /// The `+0x28`-float of an object (`Obj_GetFloat28`).
    pub fn obj_get_float28(mem: &impl Mem, p: u32) -> f32 {
        mem.read(p.wrapping_add(0x28))
    }

    /// The two-float table at `0x01325b98` (`Tk_GetScale`): entry
    /// `which != 0`.
    pub const SCALE_PAIR: Global<[f32; 2]> = Global::new(0x0132_5b98);

    /// `Tk_GetScale(which)` (`0x00872360`, own name).
    pub fn tk_get_scale(mem: &impl Mem, which: u32) -> f32 {
        SCALE_PAIR.get(mem)[usize::from(which != 0)]
    }

    /// `Tk_SetScale(which, v)` (`0x00872530`, own name): stored through
    /// `fld`/`fstp` (a signalling NaN comes out quiet).
    pub fn tk_set_scale(mem: &mut impl Mem, which: u32, v: f32) {
        let at = SCALE_PAIR.ptr(mem).addr + 4 * u32::from(which != 0);
        mem.write(at, ffx_bridge::x87::round_trip(v));
    }

    /// `Math_AngleFrom12Bits(a)` (`0x0088c0a0`) and `Math_AngleFrom15Bits`
    /// (`0x0088c0e0`, own names): the low `bits` of `a` as an angle (×2π
    /// from `0x00b39378`, ÷ the scale constant, each step rounded to `f32`
    /// as the original stores it), wrapped by `Math_WrapAngle`.
    pub fn math_angle_from_bits(mem: &impl Mem, a: u32, mask: u32, scale: u32) -> f32 {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let v = f64::from(a & mask) as f32;
        let tau = mem.read::<f64>(mem.rebase(0x00b3_9378));
        let div = mem.read::<f64>(mem.rebase(scale));
        #[allow(clippy::cast_possible_truncation)]
        let v = (f64::from(v) * tau) as f32;
        #[allow(clippy::cast_possible_truncation)]
        let v = (f64::from(v) / div) as f32;
        crate::math::wrap_angle(v)
    }

    // ---- round 7: menu sprite packets and the main option dispatch ----

    /// A GS `XYZ`/`UV` pair as the packet builders store it: `x` and `y`
    /// each sign-extended to 64 bits, `y` shifted 16 (the `or`s
    /// overlapping, as `cdq`/`shld` produce it).
    #[must_use]
    pub const fn gs_xy(x: i32, y: i32) -> u64 {
        (x as i64).cast_unsigned() | ((y as i64).cast_unsigned() << 16)
    }

    fn put_u64(mem: &mut impl Mem, at: u32, v: u64) {
        mem.write(at, v);
    }

    /// The header of a menu sprite (`0x8001`, `0x64000000`, the GIF
    /// register list `gif`, `0x156` or `0x1c9`). The original stores the
    /// list's address as a literal (no relocation), so it is stored as is.
    fn sprite_header(mem: &mut impl Mem, p: u32, first: u32, gif: u32, prim: u32) {
        mem.write(p, [first, 0x6400_0000, gif, 0, prim, 0]);
    }

    /// A menu font/sprite cell: `u8 u`, `u8 v`, `i8 dx`, `i8 dy`, `u16 w`,
    /// `u16 h`, `u16 sw`, `u16 sh` (12 bytes; `TkMenuDrawSprite`).
    fn cell(mem: &impl Mem, e: u32) -> ([i32; 2], [i32; 2], [i32; 4]) {
        let b = |o| i32::from(mem.read::<u8>(e + o));
        let s = |o| i32::from(mem.read::<i8>(e + o));
        let h = |o| i32::from(mem.read::<u16>(e + o));
        ([b(0), b(1)], [s(2), s(3)], [h(4), h(6), h(8), h(10)])
    }

    /// `MenuDraw_Sprites(p, _, x, y, entry, ox, oy, alpha)` (`0x008dc1b0`,
    /// own name): `MenuDraw_GifTag` (the GS word from `TOGetGsRegWord`,
    /// tint `alpha << 24 | 0x808080`) and one sprite per cell of table
    /// entry `entry` (`TkMenuGetTableEntry` of `0x00c5a900`): texture from
    /// (`u × 16 − 8`, not negative; `v × 16 + 8`), screen from the cell's
    /// offset back from `(x + 0x700, y + 0x730) × 16` (x mirrored). Returns
    /// the packet end.
    #[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
    pub fn menu_draw_sprites(mem: &mut impl Mem, args: [u32; 8], mirror: bool) -> u32 {
        sprites(mem, args, u8::from(mirror))
    }

    /// `MenuDraw_Sp3Body(p, _, x, y, entry, ox, oy, alpha)` (`0x008dbf20`,
    /// own name): the unmirrored form of [`menu_draw_sprites`]: texture
    /// from (`u × 16 + 8`, `v × 16 + 8`), screen from the cell's offset
    /// forward, the right edge cut where `u` passes `0xff0`.
    pub fn menu_draw_sprites_plain(mem: &mut impl Mem, args: [u32; 8]) -> u32 {
        sprites(mem, args, 2)
    }

    /// The three sprite bodies: 0 `0x008dc1b0`, 1 `0x008dc710`, 2
    /// `0x008dbf20`.
    #[allow(clippy::many_single_char_names)]
    fn sprites(mem: &mut impl Mem, args: [u32; 8], mode: u8) -> u32 {
        let mirror = mode == 1;
        let [p, _, x, y, entry, ox, oy, alpha] = args;
        let sx = x.wrapping_add(0x700) << 4;
        let sy = y.wrapping_add(0x730) << 4;
        let table = tk_menu_get_table_entry(mem, entry, mem.rebase(0x00c5_a900));
        let gs = ffx_system::pkt::bridge::GS_WORD_008FA7B0;
        #[allow(clippy::cast_possible_truncation)]
        let mut q = menu_draw_gif_tag(mem, p, gs as u32, 0, (alpha << 24) | 0x0080_8080);
        if table == 0 {
            return q;
        }
        let mut k = 0u32;
        while k < mem.read::<u32>(table) {
            let e = table + 4 + 12 * k;
            let ([u, v], [dx, dy], [w, h, sw, sh]) = cell(mem, e);
            let (sx, sy, ox, oy) = (
                sx.cast_signed(),
                sy.cast_signed(),
                ox.cast_signed(),
                oy.cast_signed(),
            );
            sprite_header(mem, q, 0x8001, 0x00f5_3530, 0x156);
            if mode == 2 {
                let u0 = u * 16 + 8;
                let v0 = v * 16 + 8;
                let mut u1 = w * 16 + u0;
                let v1 = h * 16 + v0;
                let xa = (dx * 16).wrapping_add(sx).wrapping_add(ox);
                let mut xb = (sw * 16).wrapping_add(xa);
                let ya = (dy * 16).wrapping_add(sy).wrapping_add(oy);
                let yb = (sh * 16).wrapping_add(ya);
                if u1 > 0xff0 {
                    xb = xb.wrapping_sub(u1 - 0xff0);
                    u1 = 0xff0;
                }
                put_u64(mem, q + 0x18, gs_xy(u0, v0));
                put_u64(mem, q + 0x20, gs_xy(xa, ya));
                put_u64(mem, q + 0x28, gs_xy(u1, v1));
                put_u64(mem, q + 0x30, gs_xy(xb, yb));
            } else if mirror {
                let u0 = u * 16 + 8;
                let v0 = (v * 16 - 8).max(0);
                let mut u1 = w * 16 + u0;
                let v1 = h * 16 - 8 + v0;
                let xa = dx * 16 + sx + ox;
                let mut xb = sw * 16 + xa;
                let ya = dy * 16 + sy + oy;
                let yb = sh * 16 + ya;
                if u1 > 0xff0 {
                    xb -= u1 - 0xff0;
                    u1 = 0xff0;
                }
                put_u64(mem, q + 0x18, gs_xy(u0, v0));
                put_u64(mem, q + 0x20, gs_xy(xa, yb));
                put_u64(mem, q + 0x28, gs_xy(u1, v1));
                put_u64(mem, q + 0x30, gs_xy(xb, ya));
            } else {
                let v0 = v * 16 + 8;
                let u0 = (u * 16 - 8).max(0);
                let u1 = w * 16 + u0;
                let v1 = h * 16 + v0;
                let xa = sx.wrapping_sub(dx * 16).wrapping_add(ox);
                let xb = xa.wrapping_sub(sw * 16);
                let ya = (dy * 16).wrapping_add(sy).wrapping_add(oy);
                let yb = (sh * 16).wrapping_add(ya);
                put_u64(mem, q + 0x18, gs_xy(u0, v0));
                put_u64(mem, q + 0x20, gs_xy(xa, ya));
                put_u64(mem, q + 0x28, gs_xy(u1, v1));
                put_u64(mem, q + 0x30, gs_xy(xb, yb));
            }
            q += 0x40;
            k += 1;
        }
        q
    }

    /// The three bar sprites (`MenuDraw_Bar`, `0x008dc360`;
    /// `MenuDraw_BarV`, `0x008dc480`; `MenuDraw_Bar2`, `0x008dc5a0`, own
    /// names): one sprite of cell `cell` at `(x + 0x700, y + 0x730) × 16`
    /// stretched to `n` pixels (`Bar`: down, ending at the cell width; `V`:
    /// vertical texture, along x; `Bar2`: `n` down and `m` across), nothing
    /// for `n <= 0`. Returns the packet end.
    #[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
    pub fn menu_draw_bar(
        mem: &mut impl Mem,
        kind: u8,
        p: u32,
        n: i32,
        m: i32,
        c: u32,
        x: i32,
        y: i32,
    ) -> u32 {
        if n <= 0 {
            return p;
        }
        let [b4, b5] = [4, 5].map(|o| i32::from(mem.read::<u8>(c + o)));
        let [h8, ha, hc, he] = [8, 0xa, 0xc, 0xe].map(|o| i32::from(mem.read::<u16>(c + o)));
        let (bx, by) = (x.wrapping_add(0x700), y.wrapping_add(0x730));
        sprite_header(mem, p, 0x8001, 0x00f5_3530, 0x156);
        match kind {
            0 | 2 => {
                let u0 = b4 * 16 + 8;
                let v = (ha + b5) * 16 - 8;
                let u1 = h8 * 16 + u0;
                let x1 = if kind == 0 {
                    hc.wrapping_add(bx) << 4
                } else {
                    bx.wrapping_add(m) << 4
                };
                put_u64(mem, p + 0x18, gs_xy(u0, v));
                put_u64(mem, p + 0x20, gs_xy(bx << 4, by << 4));
                put_u64(mem, p + 0x28, gs_xy(u1, v));
                put_u64(mem, p + 0x30, gs_xy(x1, by.wrapping_add(n) << 4));
            }
            _ => {
                let v0 = b5 * 16 + 8;
                let u = (hc + b4) * 16 - 8;
                let v1 = ha * 16 + v0;
                put_u64(mem, p + 0x18, gs_xy(u, v0));
                put_u64(mem, p + 0x20, gs_xy(bx << 4, by << 4));
                put_u64(mem, p + 0x28, gs_xy(u, v1));
                put_u64(
                    mem,
                    p + 0x30,
                    gs_xy(bx.wrapping_add(n) << 4, he.wrapping_add(by) << 4),
                );
            }
        }
        p + 0x40
    }

    /// `MenuDraw_Line(p, x, y, w, a, b)` (`0x008ca580`, own name): a
    /// horizontal line primitive (GIF list `0x00f51510`, `0x1c9`) from
    /// `(x, y)` to `(x + w, y)` with registers `a`, `b`; the packet end.
    #[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
    pub fn menu_draw_line(mem: &mut impl Mem, p: u32, args: [i32; 5]) -> u32 {
        let [x, y, w, a, b] = args;
        sprite_header(mem, p, 1, 0x00f5_1510, 0x1c9);
        mem.write(p + 0x18, [a.cast_unsigned(), 0]);
        put_u64(mem, p + 0x20, gs_xy(x, y));
        mem.write(p + 0x28, [b.cast_unsigned(), 0]);
        put_u64(mem, p + 0x30, gs_xy(x.wrapping_add(w), y));
        mem.write(p + 0x38, [0u32, 0]);
        p + 0x40
    }

    /// `MenuDraw_Strip(p, x, y, xs, n, dx, tag)` (`0x008ca610`) and
    /// `MenuDraw_StripR` (`0x008ca6f0`, own names): a GIF packet of `n`
    /// `XYZ2` points, point `k` at (`x ± xs[k] + dx`, `y + 16k`) (`xs`
    /// `u16`s; subtracted for `R`), padded to a quadword. Returns the end.
    #[allow(clippy::many_single_char_names, clippy::too_many_arguments)]
    pub fn menu_draw_strip(mem: &mut impl Mem, p: u32, args: [u32; 6], reverse: bool) -> u32 {
        let [x, y, xs, n, dx, tag] = args;
        mem.write(p, [1u32, 0x2400_0000, 0x10, 0, 0x1ca, 0, tag, 0]);
        let sign = if n.cast_signed() < 0 { u32::MAX } else { 0 };
        mem.write(p + 0x20, [n | 0x8000, sign | 0x1400_0000]);
        mem.write(p + 0x28, [5u32, 0]);
        let mut q = p + 0x30;
        let mut y = y;
        for k in 0..n.cast_signed().max(0).cast_unsigned() {
            let v = u32::from(mem.read::<u16>(xs.wrapping_add(2 * k)));
            let px = if reverse {
                x.wrapping_sub(v).wrapping_add(dx)
            } else {
                v.wrapping_add(dx.wrapping_add(x))
            };
            put_u64(mem, q, gs_xy(px.cast_signed(), y.cast_signed()));
            y = y.wrapping_add(0x10);
            q += 8;
        }
        if n & 1 != 0 {
            mem.write(q, [0u32, 0]);
            q += 8;
        }
        q
    }

    /// The main menu's last chosen option (`TkMenuMainOpenOption` resets
    /// it).
    pub const MENU_MAIN_CHOICE: Global<i32> = Global::new(0x0187_154c);

    /// `TkMenuMainOpenOption(i)` (`0x008e24c0`, own name): option `i`'s
    /// menu (`TkMenuMainGetOptionMenu`) activated and the main menu (slot 1)
    /// put to sleep; menus 0..20 map to slots 3, 2, 4, -, 6, 7, -, 9, 10,
    /// -, 15, ..., 21 (15), 20 (20). Returns the menu.
    pub fn tk_menu_main_open_option<M: Mem>(
        mem: &mut M,
        i: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        MENU_MAIN_CHOICE.set(mem, -1);
        let menu = tk_menu_main_get_option_menu(mem, i);
        let slot = match menu {
            0 => 3,
            1 => 2,
            2 => 4,
            4 => 6,
            5 => 7,
            7 => 9,
            8 => 10,
            10 => 15,
            15 => 21,
            20 => 20,
            _ => return menu,
        };
        tk_menu_main_active_menu(mem, slot, 0, call);
        tk_menu_main_sleep_menu(mem, 1, call);
        menu
    }
    /// The Gu selector's popup window (`Gu_SelectorOpen` stores it; the
    /// mouse handler hands it the clicks while it is set).
    pub const GU_POPUP_WINDOW: Global<u32> = Global::new(0x0130_3740);
    /// The gate for the script byte at context `+0x1e6` (`0x008655b0`).
    pub const ATEL_SCRIPT_BYTE_GATE: Global<u32> = Global::new(0x00c5_26d4);
    /// The debug draw toggle (button `0x800` on the debug pad flips it).
    pub const ATEL_DEBUG_DRAW: Global<u8> = Global::new(0x0132_8a78);
    /// The step `TkMenuDrawShadeRatio` adds each frame to the shade ratio.
    pub const MENU_SHADE_RATIO_STEP: Global<u32> = Global::new(0x0184_0880);
    /// The menu's screen x2 flag (`TkMenuStart` sets it).
    pub const MENU_SCREEN_X2: Global<u8> = Global::new(0x0184_0885);

    /// A setter of one global: `v` (its low byte for a byte global).
    /// Every one of these returns what it loaded: `eax` holds `v`, or for
    /// a byte, `al` over the rest of `v` — `v` either way.
    pub fn set_word(mem: &mut impl Mem, g: Global<u32>, v: u32) -> u32 {
        g.set(mem, v);
        v
    }

    /// The byte form of [`set_word`].
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_byte(mem: &mut impl Mem, g: Global<u8>, v: u32) -> u32 {
        g.set(mem, v as u8);
        v
    }

    /// `TkConfig_SetByteC(v)` (`0x0088ec20`, own name): config byte `+0xc`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tk_config_set_byte_c(mem: &mut impl Mem, v: u32) -> u32 {
        let at = FFX_CONFIG.ptr(mem).addr.wrapping_add(0xc);
        mem.write(at, v as u8);
        v
    }

    /// `TkGetDebugControler1Hist(i)` and friends (`0x00885150`..`0x008851f0`):
    /// `hist` field of frame `i` of pad `pad`, port 0, zero-extended.
    pub fn tk_get_debug_controler_hist(mem: &impl Mem, pad: i32, i: i32, field: u32) -> u32 {
        tk_get_controler_hist(mem, pad, 0, i, field) & 0xffff
    }

    /// `TkGetControlerIdHist(pad, port, i)` (`0x00888db0`): byte `+0` of
    /// frame `i`, loaded into `al` over the frame pointer.
    pub fn tk_get_controler_id_hist(mem: &impl Mem, pad: i32, port: i32, i: i32) -> u32 {
        let f = pad_get_frame(mem, pad, port, i);
        (f & 0xffff_ff00) | u32::from(mem.read::<u8>(f))
    }

    /// `TkGetControlerPressHist(pad, port, i)` (`0x00888df0`): byte `+1`.
    pub fn tk_get_controler_press_hist(mem: &impl Mem, pad: i32, port: i32, i: i32) -> u32 {
        u32::from(mem.read::<u8>(pad_get_frame(mem, pad, port, i).wrapping_add(1)))
    }

    /// `TkMenuMainFreeWindowGroup(group)` (`0x008aa410`): every used
    /// window record (`+0x40` == 1) of group `+0x36` freed.
    pub fn tk_menu_main_free_window_group(mem: &mut impl Mem, group: u32) {
        let base = MENU_WINDOWS.ptr(mem).addr;
        for k in 0..0x20u32 {
            let w = base + k * 0x98;
            if mem.read::<u8>(w + 0x40) == 1 && u32::from(mem.read::<u8>(w + 0x36)) == group {
                tk_menu_main_free_window(mem, w);
            }
        }
    }

    /// `Dma_TagSize(p, end)`: the quadword count between `p` and `end`
    /// minus one (the DMA tag's QWC), as the original's `u32` arithmetic.
    fn dma_qwc(p: u32, end: u32) -> u32 {
        (end.wrapping_sub(p).wrapping_add(0xf) >> 4).wrapping_sub(1)
    }

    /// `Dma_CloseCnt(p, end, flags)` (`0x00890a60`, own name): a `cnt`
    /// tag (`0x1000`) at `p` over `p..end`; the last word is the QWC with
    /// `flags` or-ed in. Returns `end`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn dma_close_cnt(mem: &mut impl Mem, p: u32, end: u32, flags: u32) -> u32 {
        let q = dma_qwc(p, end);
        mem.write(p, q as u16);
        mem.write(p + 2, 0x1000u16);
        mem.write(p + 4, [0u32, 0, q | flags]);
        end
    }

    /// `Dma_CloseCnt2(p, end, a, b)` (`0x00890aa0`, own name): a `cnt` tag
    /// with `a`, `b` as its last two words. Returns `end`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn dma_close_cnt2(mem: &mut impl Mem, p: u32, end: u32, a: u32, b: u32) -> u32 {
        mem.write(p, dma_qwc(p, end) as u16);
        mem.write(p + 2, 0x1000u16);
        mem.write(p + 4, [0u32, a, b]);
        end
    }

    /// `Dma_CloseNext(p, end, addr, a, b)` (`0x00890ae0`, own name): a
    /// `next` tag (`0x2000`) to `addr`, `a`, `b` after. Returns `end`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn dma_close_next(mem: &mut impl Mem, p: u32, end: u32, addr: u32, a: u32, b: u32) -> u32 {
        mem.write(p, dma_qwc(p, end) as u16);
        mem.write(p + 2, 0x2000u16);
        mem.write(p + 4, [addr, a, b]);
        end
    }

    /// `Dma_PutTag(p, id, addr)` (`0x00890910` `end`/`0x70`, `0x00890960`
    /// `cnt`/`0x10`, `0x008909b0` `ret`/`0x60`, `0x008908c0` and
    /// `0x00890d90` `call`/`0x50` with `addr`; own names): a one-quadword
    /// tag (QWC 1, byte 2 left as it is, id in byte 3) and a zero
    /// quadword. Returns `p + 0x20`.
    pub fn dma_put_tag(mem: &mut impl Mem, p: u32, id: u8, addr: u32) -> u32 {
        mem.write(p, 1u16);
        mem.write(p + 3, id);
        mem.write(p + 4, [addr, 0, 0, 0, 0, 0, 0]);
        p + 0x20
    }

    /// `MenuWin_StateSlide(w)` (`0x008e0200`, own name): state `+0x2c` 1,
    /// `+0x4c` 12 higher (a float, through x87), next handler `+0x114` from
    /// `+0x104`. Returns 0.
    pub fn menu_win_state_slide(mem: &mut impl Mem, w: u32) -> u32 {
        let y: f32 = mem.read(w + 0x4c);
        mem.write(w + 0x2c, 1u16);
        #[allow(clippy::cast_possible_truncation)]
        mem.write(w + 0x4c, (f64::from(y) - 12.0) as f32);
        let next: u32 = mem.read(w + 0x104);
        mem.write(w + 0x114, next);
        0
    }

    /// `MenuWin_StateOpen(w)` (`0x008e0240`, own name): state `+0x2c` 2,
    /// `+0xa0` from `+0xa8`, handlers `0x008e0230` (`+0x114`) and
    /// `0x008de970` (`+0x11c`). Returns 0.
    pub fn menu_win_state_open(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w + 0x2c, 2u16);
        let v: u32 = mem.read(w + 0xa8);
        mem.write(w + 0xa0, v);
        let (a, b) = (mem.rebase(0x008e_0230), mem.rebase(0x008d_e970));
        mem.write(w + 0x114, a);
        mem.write(w + 0x11c, b);
        0
    }

    /// `MenuWin_StateWaitText(w)` (`0x008e0280`, own name): state `+0x2e`
    /// 2, or 3 when the text at `+0x98` is empty; the handler `+0x120` is
    /// itself. Returns 0.
    pub fn menu_win_state_wait_text(mem: &mut impl Mem, w: u32) -> u32 {
        let text: u32 = mem.read(w + 0x98);
        let state: u16 = if mem.read::<u8>(text) == 0 { 3 } else { 2 };
        mem.write(w + 0x2e, state);
        let me = mem.rebase(0x008e_0280);
        mem.write(w + 0x120, me);
        0
    }

    /// `Yi_PutRef(p, a, b)` (`0x00916ad0`, own name): `{b, 0, a, 0}`.
    /// Returns `a` (left in `eax`).
    pub fn yi_put_ref(mem: &mut impl Mem, p: u32, a: u32, b: u32) -> u32 {
        mem.write(p, [b, 0, a, 0]);
        a
    }

    /// `TkDebugFont_SetWord4(p)` (`0x0088fb10`, own name): word `+4` of
    /// the current debug font buffer = `set_ptr32(p)` (`p`). Returns `p`.
    pub fn tk_debug_font_set_word4(mem: &mut impl Mem, p: u32) -> u32 {
        let buf = DEBUG_FONT_BUFFER.get(mem);
        mem.write(buf.wrapping_add(4), p);
        p
    }

    /// `TkGetDebugFontStringWidth(proportional, s)` (`0x0088edf0`):
    /// [`tk_get_str_width`] in 16-pixel cells (arithmetic shift).
    pub fn tk_get_debug_font_string_width(mem: &impl Mem, proportional: i32, s: u32) -> i32 {
        tk_get_str_width(mem, proportional, s) >> 4
    }
    /// The first value of scene `i`'s parameter `name` (`TOSeqGet*`'s
    /// lookup), 0 when it has none.
    fn seq_value(mem: &impl Mem, i: i32, name: u32) -> u32 {
        let h = crate::scene::bridge::tk_menu_get_param_handle(mem, i, name);
        if h == 0 {
            0
        } else {
            crate::scene::bridge::tk_menu_get_param_offset(mem, i, h.cast_signed(), 0)
        }
    }

    /// `TkMn2SeqGetColor(name)` (`0x008c2ad0`) and
    /// `TkMn2SeqGetColorCustom(i, name)` (`0x008c2af0`): `TOSeqGetColor`
    /// into a local, returned (scene 1 for the first).
    pub fn tk_mn2_seq_get_color(mem: &impl Mem, i: i32, name: u32) -> u32 {
        seq_value(mem, i, name)
    }

    /// `TkMn2SeqGetNum(name)` (`0x008c2b10`) and `TkMn2SeqGetNumCustom(i,
    /// name)` (`0x008c2b30`): `TOSeqGetNum`'s halfword, sign-extended.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tk_mn2_seq_get_num(mem: &impl Mem, i: i32, name: u32) -> i32 {
        i32::from(seq_value(mem, i, name) as i16)
    }

    /// `TkMn2SeqGetPosCustom(i, name, &pos)` (`0x008c2b70`): `TOSeqGetPoint`
    /// into two halfwords.
    pub fn tk_mn2_seq_get_pos_custom(mem: &mut impl Mem, i: i32, name: u32, out: u32) {
        ffx_system::scene::bridge::get_point(
            mem,
            i,
            name,
            [out, out.wrapping_add(2)],
            crate::scene::bridge::tk_menu_get_param_handle,
            crate::scene::bridge::tk_menu_get_param_offset,
        );
    }

    /// `TkMn2SeqGetRectCustom(i, name, &rect)` (`0x008c2bc0`):
    /// `TOSeqGetRect` into four halfwords.
    pub fn tk_mn2_seq_get_rect_custom(mem: &mut impl Mem, i: i32, name: u32, out: u32) {
        let outs = [0, 2, 4, 6].map(|o| out.wrapping_add(o));
        ffx_system::scene::bridge::get_rect(
            mem,
            i,
            name,
            outs,
            crate::scene::bridge::tk_menu_get_param_handle,
            crate::scene::bridge::tk_menu_get_param_offset,
        );
    }
    /// `TkMn2SeqStartScene(name)` (`0x008c2bf0`): `TOSeqStartScene(1,
    /// name)`.
    pub fn tk_mn2_seq_start_scene(mem: &mut impl Mem, name: u32) -> u32 {
        ffx_system::scene::bridge::start_scene(
            mem,
            1,
            name,
            false,
            crate::scene::bridge::tk_menu_get_scene_handle,
        )
    }

    /// `TkMn2SeqStopScene(name)` (`0x008c2c10`): `TOSeqStopScene(1, name)`.
    pub fn tk_mn2_seq_stop_scene(mem: &mut impl Mem, name: u32) -> u32 {
        ffx_system::scene::bridge::stop_scene(mem, 1, name)
    }

    /// `TkMn2SetMacroParam(n, b, v)` (`0x008c2c60`): window 0's macro `n`
    /// byte 1 = `b`, value = `v`. Returns `v`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tk_mn2_set_macro_param(mem: &mut impl Mem, n: i32, b: u32, v: u32) -> u32 {
        ffx_system::window::bridge::macro_set_byte1(mem, 0, n, b as u8);
        ffx_system::window::bridge::macro_set_value(mem, 0, n, v)
    }

    /// `TkGetDebugControlerAnalog(k)` (`0x00885190`): analog byte `k` of
    /// the debug pad's (1) current frame.
    pub fn tk_get_debug_controler_analog(mem: &impl Mem, k: u32) -> u32 {
        tk_get_controler_analog_hist(mem, 1, 0, k, 0)
    }

    /// `TkMenuMainAllocWindow()` (`0x008aa1a0`): the first unused window
    /// record (`+0x40` zero), reset (`MenuMain_ResetWindowRet`); 0 when
    /// all 32 are used.
    pub fn tk_menu_main_alloc_window(mem: &mut impl Mem) -> u32 {
        let base = MENU_WINDOWS.ptr(mem).addr;
        for k in 0..0x20u32 {
            let w = base + k * 0x98;
            if mem.read::<u8>(w + 0x40) == 0 {
                menu_main_reset_window(mem, w);
                return w;
            }
        }
        0
    }

    /// `TkMalloc(size)` (`0x0088e650`): `Sg_MemSetAlign(0x40)`, then
    /// `user_malloc` of `size` rounded up to 2 KiB (as the original's
    /// signed rounding).
    pub fn tk_malloc<M: Mem>(
        mem: &mut M,
        size: u32,
        malloc: &mut impl FnMut(&mut M, u32) -> u32,
    ) -> u32 {
        ffx_sg::state::bridge::mem_set_align(mem, 0x40);
        let a = size.wrapping_add(0x7ff);
        let bias = if a.cast_signed() < 0 { 0x7ff } else { 0 };
        let rounded = a.wrapping_add(bias) & 0xffff_f800;
        malloc(mem, rounded)
    }

    /// `TkMn2DumpPlate(plate)` (`0x008c18a0`): six `dbgPrintf`s (a PC
    /// stub); `eax` is left with the sign-extended halfword `+6`.
    pub fn tk_mn2_dump_plate(mem: &impl Mem, plate: u32) -> u32 {
        i32::from(mem.read::<i16>(plate.wrapping_add(6))).cast_unsigned()
    }

    /// `TkMn2GetMainPos()` (`0x008e2630`): the main menu's scene layout:
    /// `rect_noactive` (`0x023cc138`), `pos_noactive` (`0x023cc144`),
    /// `pos_nmblkline` (`0x0187168c`), the rectangle at `0x01871690`, and
    /// two colours (`0x023cc148`, `0x023cc140`). Returns the last colour.
    pub fn tk_mn2_get_main_pos(mem: &mut impl Mem) -> u32 {
        let r = |mem: &mut _, a: u32| Mem::rebase(mem, a);
        let (n0, d0) = (r(mem, 0x00b6_67ec), r(mem, 0x023c_c138));
        tk_mn2_seq_get_rect(mem, n0, d0);
        let (n1, d1) = (r(mem, 0x00b6_67fc), r(mem, 0x023c_c144));
        tk_mn2_seq_get_pos(mem, n1, d1);
        let (n2, d2) = (r(mem, 0x00b6_680c), r(mem, 0x0187_168c));
        tk_mn2_seq_get_pos(mem, n2, d2);
        let (n3, d3) = (r(mem, 0x00b6_681c), r(mem, 0x0187_1690));
        tk_mn2_seq_get_rect(mem, n3, d3);
        let n4 = r(mem, 0x00b6_682c);
        let c0 = tk_mn2_seq_get_color(mem, 1, n4);
        let d4 = r(mem, 0x023c_c148);
        mem.write(d4, c0);
        let n5 = r(mem, 0x00b6_683c);
        let c1 = tk_mn2_seq_get_color(mem, 1, n5);
        let d5 = r(mem, 0x023c_c140);
        mem.write(d5, c1);
        c1
    }

    /// `TkMn2GetCommonPlate(plate, rect, c1, c2, a, b)` (`0x008c1a60`) and
    /// `TkMn2GetCommonPlateCustom(i, plate, rect, c1, c2, a, b)`
    /// (`0x008c1aa0`): a menu plate from scene `i` (1 for the first): the
    /// rectangle named `rect`, colours `c1` (`+8`) and `c2` (`+0xc`), then
    /// `a`, `b` (`+0x10`, `+0x14`). Returns `b` (left in `eax`).
    pub fn tk_mn2_get_common_plate(
        mem: &mut impl Mem,
        i: i32,
        plate: u32,
        names: [u32; 3],
        a: u32,
        b: u32,
    ) -> u32 {
        tk_mn2_seq_get_rect_custom(mem, i, names[0], plate);
        let c1 = tk_mn2_seq_get_color(mem, i, names[1]);
        mem.write(plate + 8, c1);
        let c2 = tk_mn2_seq_get_color(mem, i, names[2]);
        mem.write(plate + 0xc, [c2, a, b]);
        b
    }

    /// `Script_PcOffset(ctx, worker)` (`0x008725e0`, own name): the
    /// worker's code pointer (`+0x18`) less the script base (`ctx+4`) and
    /// its `+0x30`.
    pub fn script_pc_offset(mem: &impl Mem, ctx: u32, worker: u32) -> u32 {
        let base: u32 = mem.read(ctx.wrapping_add(4));
        let pc: u32 = mem.read(worker.wrapping_add(0x18));
        let off: u32 = mem.read(base.wrapping_add(0x30));
        pc.wrapping_sub(off).wrapping_sub(base)
    }

    /// `Tk_NoteEternalCalm(scene)` (`0x0088ec90`, own name): on an Eternal
    /// Calm scene seen first, `0x018663d0` = 1 and `0x018663d4` = 0.
    /// Returns what the original leaves in `eax`: the check's result, or
    /// 0 after setting.
    pub fn tk_note_eternal_calm(mem: &mut impl Mem, scene: i32) -> u32 {
        let r = ffx_system::config::bridge::is_eternal_calm(mem, scene);
        let flag = mem.rebase(0x0186_63d0);
        if r != 0 && mem.read::<u8>(flag) == 0 {
            mem.write(flag, 1u8);
            mem.write(mem.rebase(0x0186_63d4), 0u32);
        }
        r
    }
    /// `MenuTuto_CloseWindow(k)` (`0x008d9b30` for 0, `0x008d9b60` for 1,
    /// own names): tutorial window `k` closed (`+0x41` = 1, `+0x10` = 0,
    /// `+0x3e` = 2) and forgotten, then its scene stopped (`scene18`, for
    /// the first always; `scene9`, for the second only after a close).
    /// Returns what the original leaves in `eax`: the stop's result, or 0.
    pub fn menu_tuto_close_window(mem: &mut impl Mem, k: usize) -> u32 {
        let w = TUTO_WINDOWS[k].get(mem);
        if w != 0 {
            mem.write(w + 0x41, 1u8);
            mem.write(w + 0x10, 0u32);
            mem.write(w + 0x3e, 2u8);
            TUTO_WINDOWS[k].set(mem, 0);
        } else if k == 1 {
            return 0;
        }
        let name = mem.rebase(if k == 0 { 0x00b6_6708 } else { 0x00b6_5a94 });
        tk_mn2_seq_stop_scene(mem, name)
    }

    /// The window openers' common start: a window from
    /// [`tk_menu_main_alloc_window`] (the original faults when none is
    /// free).
    fn open_window(mem: &mut impl Mem) -> u32 {
        tk_menu_main_alloc_window(mem)
    }

    fn put_code(mem: &mut impl Mem, at: u32, f: u32) {
        let f = mem.rebase(f);
        mem.write(at, f);
    }

    /// `MenuSave_OpenWindow2(arg)` (`0x008b4fb0`, own name): the save
    /// menu's second window (`0x01866258`): callbacks `+0x10` =
    /// `0x008b4920`, mode `+0x3e` 4, `+0x58` = `arg`; registered. Returns
    /// the window.
    pub fn menu_save_open_window2<M: Mem>(
        mem: &mut M,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        mem.write(w + 8, [0u32, 0]);
        put_code(mem, w + 0x10, 0x008b_4920);
        mem.write(w + 0x14, 0u32);
        mem.write(w + 0x1c, 0u32);
        mem.write(w + 0x3e, 4u8);
        SAVE_LOAD_WINDOW2.set(mem, w);
        mem.write(w + 0x58, arg);
        tk_menu_main_regist_window(mem, w, call);
        w
    }

    /// `MenuWorldMap_OpenWindow()` (`0x008e2d60`, own name): the world map
    /// window (`0x01871698`): callbacks `0x008e2b00`, `0x008e2b30`,
    /// `0x008e2b60`, mode 7, tint `0x80` ×3, `+0x64` 0, `+0x68` 6;
    /// registered. Returns the window.
    pub fn menu_world_map_open_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        put_code(mem, w + 8, 0x008e_2b00);
        put_code(mem, w + 0xc, 0x008e_2b30);
        put_code(mem, w + 0x10, 0x008e_2b60);
        mem.write(w + 0x14, 0u32);
        mem.write(w + 0x1c, 0u32);
        mem.write(w + 0x3e, 7u8);
        WORLD_MAP_WINDOW.set(mem, w);
        mem.write(w + 0x58, [0x80u32, 0x80, 0x80, 0, 6]);
        tk_menu_main_regist_window(mem, w, call);
        w
    }

    /// `TkMn2OpenSummonGrowWindow()` (`0x008ced30`): callbacks
    /// `0x008cd2e0`, `0x008cd460`, `0x008cdba0`, `0x008ceed0`; mode 2, rows
    /// from `TkMn2GetSummonGrowMax`, 9 visible, 3 columns, `+0x54` `0xaa`;
    /// registered, kept at `0x0186a5a8`, the status background set up.
    /// Returns the window.
    pub fn tk_mn2_open_summon_grow_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        put_code(mem, w + 8, 0x008c_d2e0);
        put_code(mem, w + 0xc, 0x008c_d460);
        put_code(mem, w + 0x10, 0x008c_dba0);
        put_code(mem, w + 0x14, 0x008c_eed0);
        mem.write(w + 0x3e, 2u8);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        mem.write(w + 0x42, 1u8);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_summon_grow_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x3a, 9u16);
        mem.write(w + 0x36, 3u16);
        mem.write(w + 0x54, 0xaau16);
        tk_menu_main_regist_window(mem, w, call);
        SUMMON_GROW_WINDOW.set(mem, w);
        tk_mn2_init_status_bg(mem);
        w
    }

    /// `MenuLimit_OpenWindow(arg)` (`0x008d0d50`, own name): the limit
    /// menu's window (`0x0186a714`): callbacks `MenuLimit_WindowIdle`,
    /// `0x008d0e90`, `0x008d16f0`, `0x008d1bb0`; `+0x3e` `0x201`, rows from
    /// `TkMn2GetListMax`, `+0x36` `0x103`, 7 visible, `+0x58` = `arg`.
    /// Returns what the registration returns (left in `eax`).
    pub fn menu_limit_open_window<M: Mem>(
        mem: &mut M,
        arg: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        put_code(mem, w + 0xc, 0x008d_0e90);
        put_code(mem, w + 0x10, 0x008d_16f0);
        put_code(mem, w + 0x14, 0x008d_1bb0);
        mem.write(w + 0x3e, 0x201u16);
        mem.write(w + 0x42, 1u8);
        mem.write(w + 0x58, arg);
        mem.write(w + 0x45, 0xffu8);
        mem.write(w + 0x5c, 1u32);
        put_code(mem, w + 8, 0x008d_0e80);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_list_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x36, 0x103u16);
        mem.write(w + 0x3a, 7u16);
        let r = tk_menu_main_regist_window(mem, w, call);
        LIMIT_WINDOW.set(mem, w);
        r
    }

    /// `TkMn2RefreshWeaponShopSell()` (`0x008d9550`, `kind` `0x13`) and
    /// `TkMn2RefreshWeaponShopSell2()` (`0x008d9560`, `0x15`):
    /// `TkMn2RefreshList(kind)`.
    pub fn tk_mn2_refresh_weapon_shop_sell<M: Mem>(
        mem: &mut M,
        kind: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(0x008c_2360);
        call(mem, f, &[kind]);
    }

    /// `MenuWShop_OpenSellWindow()` (`0x008da950`): the weapon shop's
    /// inventory window, kept in the second [`TUTO_WINDOWS`] slot
    /// (`0x0186ac38`): the list `TkMn2RefreshList(7)`, callbacks `+0xc`
    /// `0x008d9df0`, `+0x10` `0x008da470`, `+0x14` `0x008dab00`, `+0x1c`
    /// `MenuWShop_UsableByte`; `+0x3e` 3, `+0x36` 3, rows from
    /// `TkMn2GetListMax`, 10 visible; registered. Returns the window.
    pub fn menu_wshop_open_sell_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        let f = mem.rebase(0x008c_2360);
        call(mem, f, &[7]);
        put_code(mem, w + 0xc, 0x008d_9df0);
        put_code(mem, w + 0x10, 0x008d_a470);
        put_code(mem, w + 0x14, 0x008d_ab00);
        put_code(mem, w + 0x1c, 0x008d_9b00);
        mem.write(w + 0x3e, 3u8);
        mem.write(w + 0x36, 3u8);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        mem.write(w + 0x42, 1u8);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_list_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x3a, 10u16);
        mem.write(w + 0x45, 0u8);
        mem.write(w + 0x37, 0u8);
        TUTO_WINDOWS[1].set(mem, w);
        tk_menu_main_regist_window(mem, w, call);
        w
    }

    /// `MenuKaizou_OpenListWindow(k)` (`0x008d6ce0`): the customise menu's
    /// second window ([`KAIZOU_WINDOW2`]) over `MenuKaizou_BuildList(k)`:
    /// callbacks `MenuKaizou_WindowIdle`, `0x008d5810`,
    /// `DrawGearCustomizationMenu`, `0x008d6e20`, `+0x1c`
    /// `MenuKaizou_ItemUsable`; `+0x3e` `0x203`, `+0x58` = `k`, rows from
    /// `TkMn2GetNameModeMax` (the missing ones not shown), `+0x36` 3, 9
    /// visible. Returns the registration's `eax`.
    pub fn menu_kaizou_open_list_window<M: Mem>(
        mem: &mut M,
        k: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        let f = mem.rebase(0x008d_6dd0);
        call(mem, f, &[k]);
        put_code(mem, w + 8, 0x008d_56d0);
        put_code(mem, w + 0xc, 0x008d_5810);
        put_code(mem, w + 0x10, 0x008d_5f60);
        put_code(mem, w + 0x14, 0x008d_6e20);
        put_code(mem, w + 0x1c, 0x008d_5730);
        mem.write(w + 0x3e, 0x203u16);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        mem.write(w + 0x42, 1u8);
        mem.write(w + 0x58, k);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_name_mode_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x36, 3u16);
        mem.write(w + 0x3a, 9u16);
        let r = tk_menu_main_regist_window(mem, w, call);
        KAIZOU_WINDOW2.set(mem, w);
        r
    }

    /// `MenuEquip_OpenWeaponWindow(a, kind)` (`0x008cfc90`): the equipment
    /// menu's list ([`EQUIP_WINDOW`]) over `MenuWeapon_BuildKindList(a,
    /// kind)`: rows the `u16` of `g_TkMenuWeaponKindCount`, 5 visible;
    /// callbacks `MenuEquip_WindowIdle`, `TkMenuListWindowCtrl_008b44b0`,
    /// `0x008cf9f0`, `0x008cfd60`, `+0x1c` `0x008cf960`; `+0x3e` `0x201`,
    /// `+0x4a` `0x1d`, `+0x36` 3, `+0x4c` 50. Returns the registration's
    /// `eax`.
    pub fn menu_equip_open_weapon_window<M: Mem>(
        mem: &mut M,
        a: u32,
        kind: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        let f = mem.rebase(0x008c_fd20);
        call(mem, f, &[a, kind]);
        mem.write(w + 0x32, 0u32);
        #[allow(clippy::cast_possible_truncation)]
        let rows = MENU_WEAPON_KIND_COUNT.get(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x3a, 5u16);
        put_code(mem, w + 0xc, 0x008b_44b0);
        put_code(mem, w + 0x10, 0x008c_f9f0);
        put_code(mem, w + 0x14, 0x008c_fd60);
        put_code(mem, w + 0x1c, 0x008c_f960);
        mem.write(w + 0x3e, 0x201u16);
        mem.write(w + 0x48, 0x001d_0000u32);
        mem.write(w + 0x42, 1u8);
        put_code(mem, w + 8, 0x008c_f950);
        mem.write(w + 0x36, 3u16);
        mem.write(w + 0x4c, 0x32u16);
        let r = tk_menu_main_regist_window(mem, w, call);
        EQUIP_WINDOW.set(mem, w);
        r
    }

    /// The overdrive menu's saved scroll and cursor per character (`u16`
    /// in 4-byte entries, `MenuLimit_OpenModeWindow`).
    pub const LIMIT_SCROLLS: Global<u32> = Global::new(0x0186_a7a0);
    /// See [`LIMIT_SCROLLS`].
    pub const LIMIT_CURSORS: Global<u32> = Global::new(0x0186_a7e0);

    /// `MenuLimit_OpenModeWindow(chr)` (`0x008d0dd0`): the overdrive
    /// menu's mode list ([`LIMIT_WINDOW2`]) over `TkMn2RefreshList(1)`:
    /// callbacks `MenuLimit_WindowIdle`, `0x008d0e90`, `0x008d16f0`,
    /// `0x008d1bb0`, no `+0x1c`; `+0x3e` `0x201`, `+0x58` = `chr`, `+0x45`
    /// `0xff`, rows from `TkMn2GetListMax`, `+0x36` 3, 7 visible; scroll
    /// `+0x32`/`+0x34` and cursor `+0x48` from `chr`'s [`LIMIT_SCROLLS`]
    /// and [`LIMIT_CURSORS`] entries. Returns the registration's `eax`.
    pub fn menu_limit_open_mode_window<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        let f = mem.rebase(0x008c_2360);
        call(mem, f, &[1]);
        put_code(mem, w + 0xc, 0x008d_0e90);
        put_code(mem, w + 0x10, 0x008d_16f0);
        put_code(mem, w + 0x14, 0x008d_1bb0);
        mem.write(w + 0x1c, 0u32);
        mem.write(w + 0x3e, 0x201u16);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        mem.write(w + 0x42, 1u8);
        mem.write(w + 0x58, chr);
        mem.write(w + 0x45, 0xffu8);
        mem.write(w + 0x5c, 0u32);
        put_code(mem, w + 8, 0x008d_0e80);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_list_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x36, 3u16);
        mem.write(w + 0x3a, 7u16);
        let at = chr.wrapping_mul(4);
        let scroll: u16 = mem.read(LIMIT_SCROLLS.ptr(mem).addr.wrapping_add(at));
        mem.write(w + 0x32, scroll);
        mem.write(w + 0x34, scroll);
        let cursor: u16 = mem.read(LIMIT_CURSORS.ptr(mem).addr.wrapping_add(at));
        mem.write(w + 0x48, cursor);
        let r = tk_menu_main_regist_window(mem, w, call);
        LIMIT_WINDOW2.set(mem, w);
        r
    }

    /// `BtlEnd_CountApStep_008ba4e0(chr, mark)` (`0x008ba4e0`): one step
    /// of the battle-end AP count-up: at sphere level 99 or more
    /// (`BtlEnd_GetSP`) the rest (`SetTable_02310f20(chr, 0)`) is dropped
    /// and 0 returned; otherwise `BtlEnd_ApStep(chr)` AP (at most the rest,
    /// `BtlEnd_GetResult(chr)`) go to `BtlEnd_AddAP_008bdaf0(chr, step,
    /// mark)` and the rest shrinks. Returns 1 while AP were left over.
    pub fn btl_end_count_ap_step<M: Mem>(
        mem: &mut M,
        chr: u32,
        mark: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let f = fun(mem, 0x008b_a3c0);
        let mut step = call(mem, f, &[chr]);
        let f = fun(mem, 0x008b_da10);
        let rest = call(mem, f, &[chr]);
        let f = fun(mem, 0x008b_da50);
        let level = call(mem, f, &[chr]).cast_signed();
        let set = fun(mem, 0x008b_dfc0);
        if level >= 0x63 {
            call(mem, set, &[chr, 0]);
            return 0;
        }
        let mut more = 1;
        if step.cast_signed() >= rest.cast_signed() {
            step = rest;
            more = 0;
        }
        let f = fun(mem, 0x008b_daf0);
        call(mem, f, &[chr, step, mark]);
        call(mem, set, &[chr, rest.wrapping_sub(step)]);
        more
    }

    /// `MenuItem_OpenGearWindow()` (`0x008c9330`): the item menu's gear
    /// list ([`ITEM_WINDOW4`]) over `TkMn2RefreshWeaponShopSell2()`, gear
    /// mode ([`MENU_ITEM_GEAR_MODE`]) off: callbacks `+0xc` `0x008c7630`,
    /// `+0x10` `0x008c87e0`, `+0x14` `0x008c9400`; `+0x3e` 1, `+0x42` 2,
    /// rows from `TkMn2GetNameModeMax`, 9 visible, `+0x45` `0xff`.
    /// Returns the registration's `eax`.
    pub fn menu_item_open_gear_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = open_window(mem);
        let f = mem.rebase(0x008d_9560);
        call(mem, f, &[]);
        MENU_ITEM_GEAR_MODE.set(mem, 0);
        put_code(mem, w + 0xc, 0x008c_7630);
        put_code(mem, w + 0x10, 0x008c_87e0);
        put_code(mem, w + 0x14, 0x008c_9400);
        mem.write(w + 0x3e, 1u8);
        mem.write(w + 0x48, 0u16);
        mem.write(w + 0x32, 0u32);
        mem.write(w + 0x42, 2u8);
        #[allow(clippy::cast_possible_truncation)]
        let rows = tk_mn2_get_name_mode_max(mem) as u16;
        mem.write(w + 0x30, rows);
        mem.write(w + 0x3a, 9u16);
        mem.write(w + 0x45, 0xffu8);
        ITEM_WINDOW4.set(mem, w);
        tk_menu_main_regist_window(mem, w, call)
    }

    /// `MenuItem_PackGear()` (`0x008ca120`): over the shown entries
    /// (`TkMn2GetNameModeMax`) of [`MENU_ITEM_LIST`], each empty one
    /// (kind byte `+2` zero) takes the next filled one after it
    /// (`Menu_FindFlagged(list, from, n)`; the search goes on from there):
    /// `SwapSaveGear(id, its id)` and the kind bytes swapped; stops when
    /// none is left. Then `TkMn2RefreshWeaponShopSell2()` and its `eax`.
    pub fn menu_item_pack_gear<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let shown = tk_mn2_get_name_mode_max(mem).cast_signed();
        let mut from = 0i32;
        let mut k = 0i32;
        while k < shown {
            let entry = list.wrapping_add(k.cast_unsigned().wrapping_mul(4));
            if mem.read::<u8>(entry.wrapping_add(2)) == 0 {
                from = from.max(k + 1);
                let f = mem.rebase(0x008c_9b90);
                from = call(mem, f, &[list, from.cast_unsigned(), shown.cast_unsigned()])
                    .cast_signed();
                if from < 0 {
                    break;
                }
                let other = list.wrapping_add(from.cast_unsigned().wrapping_mul(4));
                let (id, other_id) = (mem.read::<u16>(entry), mem.read::<u16>(other));
                let f = mem.rebase(0x007a_b9f0);
                call(mem, f, &[u32::from(id), u32::from(other_id)]);
                let (kind, other_kind) = (
                    mem.read::<u8>(entry.wrapping_add(2)),
                    mem.read::<u8>(other.wrapping_add(2)),
                );
                mem.write(entry.wrapping_add(2), other_kind);
                mem.write(other.wrapping_add(2), kind);
            }
            k += 1;
        }
        let f = mem.rebase(0x008d_9560);
        call(mem, f, &[])
    }

    /// `MenuItem_DrawKeyCursor(_, x, y)` (`0x008c4b60`, caller
    /// `0x008c3d00`): unless `0x0186a270` is set (then 1), the command
    /// cursor `TkMn2DrawCrossCmdCursor(cx, cy, 0)` on the key cell
    /// [`MENU_ITEM_CELL`] (`15 row + col`, five keys a group): `cx =
    /// f32(remapX2(53) * f32(col % 5) + f32(x))`, `cy = f32(f32(row) *
    /// step + f32(y))` with `step = f32(remapY2(471) /
    /// f32(max(MenuItem_KeyRowCount(), 6)))`; groups 0..2 add
    /// `remapX2(684/1041/1398)` and `remapY2(423)`. Its `eax`.
    pub fn menu_item_draw_key_cursor<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, div, load, mul, store};
        let int = f64::from;
        let (xf, yf) = (store(int(x)), store(int(y)));
        let key_cell = MENU_ITEM_CELL.get(mem).cast_signed();
        let (row, col) = (key_cell / 15, key_cell % 15);
        let count = mem.rebase(0x008c_5180);
        let rows = if call(mem, count, &[]).cast_signed() > 6 {
            call(mem, count, &[]).cast_signed()
        } else {
            6
        };
        let ry = remap(mem, true, 471.0);
        let step = store(div(ry, load(store(int(rows)))));
        let (group, key) = (col / 5, col % 5);
        let rx = remap(mem, false, 53.0);
        let mut cx = store(add(mul(rx, load(store(int(key)))), load(xf)));
        let mut cy = store(add(mul(load(store(int(row))), load(step)), load(yf)));
        let left = match group {
            0 => Some(684.0),
            1 => Some(1041.0),
            2 => Some(1398.0),
            _ => None,
        };
        if let Some(left) = left {
            cx = store(add(remap(mem, false, left), load(cx)));
            cy = store(add(remap(mem, true, 423.0), load(cy)));
        }
        if mem.read::<u32>(mem.rebase(0x0186_a270)) != 0 {
            return 1;
        }
        let f = mem.rebase(0x008c_0580);
        call(mem, f, &[cx, cy, 0])
    }

    /// `BtlEnd_CountApSteps()` (`0x008ba550`): `BtlEnd_CountApStep_008ba4e0`
    /// for each entry `k` of [`BTL_END_LIST`] (its `i16` character at entry
    /// `+6`, `mark` = `k`); 1 while any had AP left.
    pub fn btl_end_count_ap_steps<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let first = BTL_END_LIST.ptr(mem).addr.wrapping_add(0xe);
        let mut any = 0;
        let mut k = 0u32;
        while k < u32::from(BTL_END_LIST.get(mem)) {
            let chr = i32::from(mem.read::<i16>(first.wrapping_add(14 * k)));
            let f = mem.rebase(0x008b_a4e0);
            any |= call(mem, f, &[chr.cast_unsigned(), k]);
            k += 1;
        }
        any
    }

    /// The sub-command window openers (`0x008e33d0`
    /// `TkMenuOpenSubCmdWindowMain`, `0x008e3470`, `0x008e3510`,
    /// `0x008e35b0`): kept in `0x023cc120`; 2 rows of 2, cursor 1, `+0x37`
    /// 1 (`b37`), mode 2 (3 for `items` 0), `+0x24` = `x`, callbacks
    /// `0x008e2e20`, `0x008e2f80`, `0x008e36b0`, `+0x58..` = `items`, `c`,
    /// `d`, `+0x90`/`+0x94` the variant's pair; registered. Returns the
    /// window.
    #[allow(clippy::too_many_arguments)]
    pub fn menu_sub_cmd_open<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        b37: bool,
        pair: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [x, items, c, d] = args;
        let w = open_window(mem);
        MN2_LIST_MAX2.set(mem, w);
        mem.write(w + 0x30, 2u16);
        mem.write(w + 0x3a, 2u16);
        mem.write(w + 0x48, 1u16);
        mem.write(w + 0x42, 1u8);
        if b37 {
            mem.write(w + 0x37, 1u8);
        }
        mem.write(w + 0x3e, u8::from(items == 0) + 2);
        mem.write(w + 0x24, x);
        mem.write(w + 8, 0u32);
        put_code(mem, w + 0xc, 0x008e_2e20);
        put_code(mem, w + 0x10, 0x008e_2f80);
        put_code(mem, w + 0x14, 0x008e_36b0);
        mem.write(w + 0x1c, 0u32);
        mem.write(w + 0x5c, c);
        mem.write(w + 0x58, items);
        mem.write(w + 0x60, d);
        mem.write(w + 0x90, pair);
        tk_menu_main_regist_window(mem, w, call);
        w
    }

    /// `TkMenuOpenSubCmdWindowMain_008e4790(id, texts, values, n, width,
    /// b44)` (`0x008e4790`; Fahrenheit's Switch name, shared with
    /// `0x008e33d0`): the one-row sub-command window. A window with `+0x1c`
    /// = `id`, `+0x4e` = `width` (`u16`), callbacks `+8` `0x008e36c0`, `+0xc`
    /// `0x008e3880`, `+0x10` `0x008e3cf0`, `+0x14` `0x008e4a60`, `+0x30` =
    /// `n` (`u16`), `+0x46` the word `0x1000`, `+0x42` 1, `+0x36` the word
    /// `0x103`, layer 2 (`+0x3e`), `+0x44` = `b44`. Entry `k < n` (signed)
    /// is `[texts[k], values[k], TkMn2StringWidth(texts[k])]` from `+0x58`,
    /// 12 bytes each. Then `+0x90` = 0 and the row's length (the widths
    /// plus `32 * (n - 1)`, wrapping) goes to `+0x3a`; a length above the
    /// width (signed) but below `0x1f1` sets the width to `0x1f0`. The gap
    /// `+0x3c`: `(width - length - 0x60) / (n - 1)` (signed) for `n > 1`,
    /// clamped as a `u16` to `0x20..=0x80`; 0x20 otherwise. Registered;
    /// returns the window. Its callers (`0x008e4660`, `0x008e46b0`,
    /// `0x008e48f0`, `0x008e4930`) keep it in `g_TkMenuSubCmdWindow`.
    pub fn tk_menu_open_sub_cmd_window_row<M: Mem>(
        mem: &mut M,
        args: [u32; 6],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [id, texts, values, n, width, b44] = args;
        let n = n.cast_signed();
        let w = open_window(mem);
        mem.write(w + 0x1c, id);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        mem.write(w + 0x4e, width as u16);
        put_code(mem, w + 8, 0x008e_36c0);
        put_code(mem, w + 0xc, 0x008e_3880);
        put_code(mem, w + 0x10, 0x008e_3cf0);
        put_code(mem, w + 0x14, 0x008e_4a60);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        mem.write(w + 0x30, n as u16);
        mem.write(w + 0x46, 0x1000u32);
        mem.write(w + 0x42, 1u8);
        mem.write(w + 0x36, 0x103u32);
        mem.write(w + 0x3e, 2u8);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        mem.write(w + 0x44, b44 as u8);
        let mut len = 0i32;
        for k in 0..n.max(0).cast_unsigned() {
            let text: u32 = mem.read(texts.wrapping_add(4 * k));
            let value: u32 = mem.read(values.wrapping_add(4 * k));
            let width = tk_mn2_string_width(mem, text, call);
            len = len.wrapping_add(width.cast_signed());
            let e = w.wrapping_add(0x58 + 0xc * k);
            mem.write(e, text);
            mem.write(e.wrapping_add(8), width);
            mem.write(e.wrapping_add(4), value);
        }
        let len = len.wrapping_add(n.wrapping_shl(5)).wrapping_sub(0x20);
        mem.write(w + 0x90, 0u32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        mem.write(w + 0x3a, len as u16);
        if i32::from(mem.read::<i16>(w + 0x4e)) < len && len < 0x1f1 {
            mem.write(w + 0x4e, 0x1f0u16);
        }
        let gap = if n > 1 {
            let h = i32::from(mem.read::<i16>(w + 0x4e));
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let q = (h.wrapping_sub(len).wrapping_sub(0x60) / (n - 1)) as u16;
            q.clamp(0x20, 0x80)
        } else {
            0x20
        };
        mem.write(w + 0x3c, gap);
        tk_menu_main_regist_window(mem, w, call);
        w
    }

    /// `TkMenuCtrlHdd(w)` (`0x008e2870`): the HDD menu's control step (the
    /// pad reads it starts with are unused): state `+0x1c` 0 closes the
    /// HDD menu (`MenuHdd_Close`) and moves to 1; in state 1, once
    /// `0x023cc130` is set, menu 20 sleeps and the main menu (1) wakes.
    pub fn tk_menu_ctrl_hdd<M: Mem>(
        mem: &mut M,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        match mem.read::<u32>(w + 0x1c) {
            0 => {
                menu_hdd_close(mem, call);
                mem.write(w + 0x1c, 1u32);
            }
            1 if mem.read::<u32>(mem.rebase(0x023c_c130)) != 0 => {
                tk_menu_main_sleep_menu(mem, 0x14, call);
                tk_menu_main_active_menu(mem, 1, 0, call);
            }
            _ => {}
        }
    }

    /// `Vibrate_BumpStream(k)` (`0x00875af0`, own name): vibration stream
    /// `k` (clamped to 0..=7, negative to 0): its `u16` counter
    /// (`0x01328170 + k * 0x24`) up by one and its byte `+2` cleared.
    /// Returns the stream's record (`0x01328150 + k * 0x24`, left in
    /// `eax`).
    pub fn vibrate_bump_stream(mem: &mut impl Mem, k: i32) -> u32 {
        let k = k.clamp(0, 7).cast_unsigned();
        let rec = mem.rebase(0x0132_8150) + k * 0x24;
        let n: u16 = mem.read(rec + 0x20);
        mem.write(rec + 0x20, n.wrapping_add(1));
        mem.write(rec + 0x22, 0u8);
        rec
    }

    /// `TkDebugFont_SwapBuffer()` (`0x00890780`, own name): the debug
    /// font's current buffer (`g_TkDebugFontBuffer`) flipped between the
    /// two at `0x0133c8c4`/`0x0133c8c8`; its write pointer (`+0`, `+4`)
    /// back to its start (`+8`), the position words cleared. Returns the
    /// start.
    pub fn tk_debug_font_swap_buffer(mem: &mut impl Mem) -> u32 {
        let pair: [u32; 2] = mem.read(mem.rebase(0x0133_c8c4));
        let cur = if DEBUG_FONT_BUFFER.get(mem) == pair[0] {
            pair[1]
        } else {
            pair[0]
        };
        DEBUG_FONT_BUFFER.set(mem, cur);
        let start: u32 = mem.read(cur + 8);
        mem.write(cur, [start, start]);
        mem.write(mem.rebase(0x0133_c8d0), [0u32; 3]);
        start
    }

    /// `Dma_PutTagEx(p, qwc, pce, id, irq, addr)` (`0x00890860`, and its
    /// copy `0x00890d30`; own names): a DMA tag with QWC `qwc` (1 when 0),
    /// byte 3 `((irq << 3 | id) << 2 | pce) << 2` (8-bit), `addr`, and two
    /// zero words; for `qwc` 0 a zero quadword follows. Returns the end.
    #[allow(clippy::cast_possible_truncation)]
    pub fn dma_put_tag_ex(mem: &mut impl Mem, p: u32, qwc: u32, bits: [u32; 3], addr: u32) -> u32 {
        let [pce, id, irq] = bits.map(|b| b as u8);
        mem.write(p, if qwc == 0 { 1 } else { qwc as u16 });
        let b = (((irq << 3) | id) << 2 | pce) << 2;
        mem.write(p + 3, b);
        mem.write(p + 4, [addr, 0, 0]);
        if qwc == 0 {
            mem.write(p + 0x10, [0u32; 4]);
            p + 0x20
        } else {
            p + 0x10
        }
    }

    /// `Vibrate_SetStream(entry, src)` (`0x00876230`, own name): a stream
    /// entry from a curve header `src` (step 0, length `(src[1] + 1) × 2`,
    /// bytes `src[0]` and `src[2]`, data from `src + 2`), or for null an
    /// idle entry (`+4` = `0x02000000`, `+8` 0, no data). Returns the data
    /// pointer (`set_ptr32`'s, left in `eax`).
    pub fn vibrate_set_stream(mem: &mut impl Mem, entry: u32, src: u32) -> u32 {
        if src == 0 {
            mem.write(entry + 4, 0x0200_0000u32);
            mem.write(entry + 8, 0u16);
            mem.write(entry, 0u32);
            return 0;
        }
        mem.write(entry + 4, 0u16);
        let n = u16::from(mem.read::<u8>(src + 1));
        mem.write(entry + 6, n.wrapping_add(1).wrapping_mul(2));
        let a: u8 = mem.read(src);
        mem.write(entry + 8, a);
        let data = src + 2;
        let b: u8 = mem.read(data);
        mem.write(entry + 9, b);
        mem.write(entry, data);
        data
    }

    /// `Vibrate_ResetAll()` (`0x00875e80`, own name): the eight streams'
    /// counters and both entries cleared, vibration on, every pad allowed
    /// (`0xff`). Returns `0xff`.
    pub fn vibrate_reset_all(mem: &mut impl Mem) -> u32 {
        let base = mem.rebase(0x0132_8150);
        for k in 0..8 {
            let rec = base + k * 0x24;
            mem.write(rec + 0x22, 0u8);
            mem.write(rec + 0x20, 0u16);
            for e in [rec, rec + 0x10] {
                mem.write(e + 4, 0u32);
                mem.write(e + 8, 0u16);
                mem.write(e + 0xa, 0u8);
            }
        }
        VIBRATE_OFF.set(mem, 0);
        mem.write(mem.rebase(0x0132_8274), 0xffu16);
        0xff
    }

    /// `TkEucToSjis(src, out)` (`0x008907e0`, own name): one character of
    /// the debug font's EUC text: 0 ends (0 written, 0 returned), ASCII
    /// passes (1), a two-byte EUC code becomes Shift-JIS (2).
    pub fn tk_euc_to_sjis(mem: &mut impl Mem, src: u32, out: u32) -> u32 {
        let c0: u8 = mem.read(src);
        let d0: u8 = mem.read(src + 1);
        if c0 == 0 {
            mem.write(out, 0u8);
            return 0;
        }
        if c0 < 0x80 {
            mem.write(out, c0);
            return 1;
        }
        let (c, d) = (c0 & 0x7f, d0 & 0x7f);
        let lead: u8 = if c < 0x5f { 0x70 } else { 0xb0 };
        let trail_base: u8 = if c & 1 == 0 {
            0x7e
        } else {
            0x1f + u8::from(d > 0x5f)
        };
        #[allow(clippy::cast_possible_truncation)]
        let first = ((u32::from(c) + 1) >> 1) as u8;
        mem.write(out, first.wrapping_add(lead));
        mem.write(out + 1, trail_base.wrapping_add(d));
        2
    }
    /// `TkNormalFVector(v)` (`0x0088c5a0`): `FFXVu0Normalize(v, v)`.
    pub fn tk_normal_f_vector(mem: &mut impl Mem, v: u32) {
        ffx_ps2::vu0::bridge::normalize(mem, v, v);
    }

    /// The C runtime's `strstr` on game strings: whether `needle` occurs
    /// in `hay` (an empty needle always does).
    fn c_contains(mem: &impl Mem, hay: u32, needle: u32) -> bool {
        let read = |p: u32| {
            let mut v = Vec::new();
            let mut q = p;
            loop {
                let c: u8 = mem.read(q);
                if c == 0 {
                    return v;
                }
                v.push(c);
                q = q.wrapping_add(1);
            }
        };
        let (h, n) = (read(hay), read(needle));
        n.is_empty() || h.windows(n.len()).any(|w| w == n.as_slice())
    }

    /// `hardCodeToFixTheWrongData(_, kind)` (`0x008e02f0`): the
    /// flag at `0x01871534` set when `kind` is 14 and one of the seven
    /// 16-byte records at `0x00c5b240` (a name, then an id at `+0xc`) has
    /// its name inside the string at `0x023cbc60` and its id equal to
    /// `0x01325ac0`; cleared otherwise. Returns what the last record leaves
    /// in `eax`: that global's value when its name matched, else 0.
    pub fn hard_code_to_fix_the_wrong_data(mem: &mut impl Mem, kind: i32) -> u32 {
        let flag = mem.rebase(0x0187_1534);
        mem.write(flag, 0u32);
        let hay = mem.rebase(0x023c_bc60);
        let table = mem.rebase(0x00c5_b240);
        let cur: u32 = mem.read(mem.rebase(0x0132_5ac0));
        let mut eax = 0;
        for k in 0..7 {
            let rec = table + 0x10 * k;
            eax = 0;
            if c_contains(mem, hay, rec) {
                eax = cur;
                if cur == mem.read::<u32>(rec + 0xc) && kind == 0xe {
                    mem.write(flag, 1u32);
                }
            }
        }
        eax
    }

    /// `TkMenuDrawRect(x, y, w, h, c0, c1)` (`0x008dd340`): on
    /// PC a renderer UI element (`graphicDrawUIElement(element, 0, 1, 0,
    /// 0)`), skipped while `graphicGetAbmap` is 1: corners (`x`, `y`) and
    /// (`x + w`, `y + h`) as floats, each with the bytes of its colour, depth
    /// 1000. `draw` gets the element's set fields as (offset, word); the
    /// original's other words are stack garbage. Returns `draw`'s result,
    /// or 1 when skipped.
    #[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
    pub fn tk_menu_draw_rect<M: Mem>(
        mem: &mut M,
        [x, y, w, h]: [i32; 4],
        colours: [u32; 2],
        abmap: &mut impl FnMut(&mut M) -> u32,
        draw: &mut impl FnMut(&mut M, &[(u32, u32)]) -> u32,
    ) -> u32 {
        if abmap(mem) == 1 {
            return 1;
        }
        let f = |v: i32| (v as f32).to_bits();
        let mut fields = vec![
            (0, f(x)),
            (4, f(y)),
            (0x20, f(x.wrapping_add(w))),
            (0x24, f(y.wrapping_add(h))),
            (0x80, 1000),
            (0x88, 0),
            (0x90, 0),
            (0x94, 0),
        ];
        for (base, c) in [(0x10, colours[0]), (0x30, colours[1])] {
            for k in 0..4 {
                fields.push((base + 4 * k, (c >> (8 * k)) & 0xff));
            }
        }
        draw(mem, &fields)
    }

    /// `TkMenuDrawSp3FaceA(x, y, entry)` (`0x008dd610`) and
    /// `TkMenuDrawSp3FaceAA(x, y, entry, alpha)` (`0x008dd6a0`): a face
    /// sprite (`MenuDraw_SpriteP1`, tint `alpha`) at `g_TkMenuPacket +
    /// 0x10`, then its frame as four [`tk_menu_draw_rect`] bars (grey
    /// `0x40808080`): top and bottom `0x5e × 2`, left and right `0xb ×
    /// 0x39`. Returns the last bar's result.
    pub fn tk_menu_draw_sp3_face<M: Mem>(
        mem: &mut M,
        [x, y, entry, alpha]: [u32; 4],
        abmap: &mut impl FnMut(&mut M) -> u32,
        draw: &mut impl FnMut(&mut M, &[(u32, u32)]) -> u32,
    ) -> u32 {
        let p = MENU_PACKET.get(mem).wrapping_add(0x10);
        MENU_PACKET.set(mem, p);
        let end = menu_draw_sprites_plain(mem, [p, 1, x, y, entry, 0, 0, alpha]);
        MENU_PACKET.set(mem, end);
        let (x, y) = (x.cast_signed(), y.cast_signed());
        let grey = [0x4080_8080; 2];
        tk_menu_draw_rect(mem, [x, y, 0x5e, 2], grey, abmap, draw);
        tk_menu_draw_rect(mem, [x, y.wrapping_add(0x3b), 0x5e, 2], grey, abmap, draw);
        let y2 = y.wrapping_add(2);
        tk_menu_draw_rect(mem, [x, y2, 0xb, 0x39], grey, abmap, draw);
        tk_menu_draw_rect(
            mem,
            [x.wrapping_add(0x53), y2, 0xb, 0x39],
            grey,
            abmap,
            draw,
        )
    }
    /// `TkMenuGetCommandName(id)` (`0x008dde50`, own name): the command's
    /// name string (`MsGetRomPlyCommand`'s strings plus the record's first
    /// halfword, or its third with hiragana on).
    pub fn tk_menu_get_command_name(mem: &impl Mem, id: u32) -> u32 {
        let k = u32::from(ffx_battle::save::bridge::ms_get_save_config_hiragana(mem) != 0);
        let table = ffx_kernel::bridge::COMMAND.get(mem);
        #[allow(clippy::cast_possible_wrap)] // 12 bits
        let (record, strings) = ffx_kernel::bridge::excel_data(mem, (id & 0xfff) as i32, table);
        strings.wrapping_add(u32::from(mem.read::<u16>(record.wrapping_add(4 * k))))
    }

    /// `Math_StoreMax(dst, src, v)` (`0x00889b00`, own name): `*dst` = `v`,
    /// or `*src` when `v` is less (a NaN keeps `v`); through x87.
    pub fn math_store_max(mem: &mut impl Mem, dst: u32, src: u32, v: f32) {
        let s: f32 = mem.read(src);
        let r = if v < s { s } else { v };
        mem.write(dst, ffx_bridge::x87::round_trip(r));
    }

    /// `Math_StoreMin(dst, src, _, _, _, v)` (`0x00889b40`, own name):
    /// `*dst` = `v`, or `*src` when `v` is greater.
    pub fn math_store_min(mem: &mut impl Mem, dst: u32, src: u32, v: f32) {
        let s: f32 = mem.read(src);
        let r = if v > s { s } else { v };
        mem.write(dst, ffx_bridge::x87::round_trip(r));
    }

    /// `TkDeci2Send(a, n)` (`0x00885470`, own name): with no request
    /// pending (`0x0132f581` and `0x0132f578` clear) one is queued: `n -
    /// 0x81` truncated to a multiple of 128 at `0x0132f57e`, `a |
    /// 0x20000000` at `0x0132f578`; 1. Busy: -1.
    #[allow(clippy::cast_possible_truncation)]
    pub fn tk_deci2_send(mem: &mut impl Mem, a: u32, n: i32) -> i32 {
        let busy = mem.read::<u8>(mem.rebase(0x0132_f581)) != 0
            || mem.read::<u32>(mem.rebase(0x0132_f578)) != 0;
        if busy {
            return -1;
        }
        let v = n.wrapping_sub(0x81);
        let v = v.wrapping_add(if v < 0 { 0x7f } else { 0 }) & !0x7f;
        mem.write(mem.rebase(0x0132_f57e), v.cast_unsigned() as u16);
        mem.write(mem.rebase(0x0132_f578), a | 0x2000_0000);
        1
    }

    /// `TkCh_SetPos(chr, x, y, z)` (`0x00872310`, own name): `Ch_SetPos3`
    /// with the floats passed through x87.
    pub fn tk_ch_set_pos(mem: &mut impl Mem, chr: u32, [x, y, z]: [f32; 3]) {
        let r = ffx_bridge::x87::round_trip;
        ffx_chara::chr::bridge::ch_set_pos3(mem, chr, r(x), r(y), r(z));
    }
    /// `Math_AngleTo12Bits(a)` (`0x0088bfe0`) and `Math_AngleTo15Bits(a)`
    /// (`0x0088c040`, own names): `a` wrapped (`Math_WrapAngle`), made
    /// non-negative by a turn, scaled by the constant at `scale` (4096 or
    /// 32768) over 2π, truncated (`_ftol2_sse`) and masked.
    pub fn math_angle_to_bits(mem: &impl Mem, a: f32, scale: u32, mask: u32) -> u32 {
        let tau = mem.read::<f64>(mem.rebase(0x00b3_9378));
        let k = mem.read::<f64>(mem.rebase(scale));
        let mut w = crate::math::wrap_angle(ffx_bridge::x87::round_trip(a));
        if w < 0.0 {
            #[allow(clippy::cast_possible_truncation)]
            {
                w = (f64::from(w) + tau) as f32;
            }
        }
        ffx_bridge::x87::ftol(mem, f64::from(w) * k / tau) & mask
    }

    /// `Math_AngleToDegrees(a)` (`0x0088c5c0`, own name): `a` wrapped,
    /// times `57.2957763671875`, then brought into [0, 360] by whole turns
    /// (float steps).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn math_angle_to_degrees(a: f32) -> f32 {
        let w = crate::math::wrap_angle(ffx_bridge::x87::round_trip(a));
        let mut d = (f64::from(w) * 57.295_776_367_187_5) as f32;
        while d > 360.0 {
            d = (f64::from(d) - 360.0) as f32;
        }
        while d < 0.0 {
            d = (f64::from(d) + 360.0) as f32;
        }
        d
    }

    /// `Math_AngleStepTo(a, target, step)` (`0x0088b6b0`, own name): `a +
    /// step` unless that would reach or pass `target` (or a NaN arises),
    /// then `target`; wrapped.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn math_angle_step_to(a: f32, target: f32, step: f32) -> f32 {
        let d1 = (f64::from(a) - f64::from(target)) as f32;
        let d2 = (f64::from(a) + f64::from(step) - f64::from(target)) as f32;
        let d = (f64::from(d1) * f64::from(d2)) as f32;
        let v = if d > 0.0 {
            (f64::from(a) + f64::from(step)) as f32
        } else {
            target
        };
        crate::math::wrap_angle(ffx_bridge::x87::round_trip(v))
    }

    /// `Vibrate_SetStreams(rec, a, b)` (`0x00876180`, own name): both
    /// entries of a stream record from curve headers ([`vibrate_set_stream`]
    /// at `rec` and `rec + 0x10`). Returns the second's data pointer.
    pub fn vibrate_set_streams(mem: &mut impl Mem, rec: u32, a: u32, b: u32) -> u32 {
        vibrate_set_stream(mem, rec, a);
        vibrate_set_stream(mem, rec + 0x10, b)
    }

    /// `Vibrate_StartLevel(k, e, from, to, len)` (`0x00876050`, own name):
    /// stream `k`'s (0..=7) entry `e` (0..=1) restarted as a plain level:
    /// no data, `+4` `0xfffe0000` then step 0 and length `len` (-1 for
    /// negative), level `from` (the entry's `+0xa` when negative) easing to
    /// `to` (both clamped to 0..=0x7f; `to` is `from` for a negative
    /// length); the stream's counter bumped and mode `+0x22` 2. Returns the
    /// length.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn vibrate_start_level(
        mem: &mut impl Mem,
        k: i32,
        e: i32,
        from: i32,
        to: i32,
        len: i32,
    ) -> i32 {
        let k = k.clamp(0, 7).cast_unsigned();
        let rec = mem.rebase(0x0132_8150) + k * 0x24;
        let n: u16 = mem.read(rec + 0x20);
        mem.write(rec + 0x20, n.wrapping_add(1));
        mem.write(rec + 0x22, 0u8);
        let entry = rec + e.clamp(0, 1).cast_unsigned() * 0x10;
        mem.write(entry, 0u32);
        mem.write(rec + 0x22, 2u8);
        mem.write(entry + 4, 0xfffe_0000u32);
        mem.write(entry + 8, 0u16);
        let from = if from < 0 {
            i32::from(mem.read::<u8>(entry + 0xa))
        } else {
            from
        };
        let from = from.clamp(0, 0x7f) as u8;
        let to = to.clamp(0, 0x7f) as u8;
        let len = if len < 0 { -1 } else { len };
        mem.write(entry + 4, 0u16);
        mem.write(entry + 6, len as u16);
        mem.write(entry + 8, from);
        mem.write(entry + 9, if (len as i16) < 0 { from } else { to });
        len
    }

    /// `Vibrate_CurveData(k, n, table)` (`0x00875d70`, own name): curve
    /// `k` (< 0x40) of `table`: its start (`table + table[k]`) plus 4 and
    /// its first `n` halfword lengths, when it has a curve and its
    /// halfword `n` is non-zero; 0 otherwise.
    pub fn vibrate_curve_data(mem: &impl Mem, k: u32, n: i32, table: u32) -> u32 {
        if k > 0x3f || table == 0 {
            return 0;
        }
        let off: u32 = mem.read(table.wrapping_add(4 * k));
        if off == 0 {
            return 0;
        }
        let base = off.wrapping_add(table);
        let at = |j: i32| base.wrapping_add((j.wrapping_mul(2)).cast_unsigned());
        if mem.read::<u16>(at(n)) == 0 {
            return 0;
        }
        let mut sum = 4u32;
        for j in 0..n.max(0) {
            sum = sum.wrapping_add(u32::from(mem.read::<u16>(at(j))));
        }
        sum.wrapping_add(base)
    }
    /// The CLUT allocator's state: VRAM base (`0x0193bb0c`), cursor
    /// (`+4`), the split (`+8`, `g_YiClutSplit`), CLUTs handed out (`+0xc`)
    /// and the sub-slot within a 0x800 block (`+0x10`).
    const CLUT_STATE: u32 = 0x0193_bb0c;

    /// `yiClutAlloc_init(mode)` (`0x00916e60`): mode 2 starts at
    /// `0xb4000`, 3 at `0x18000`, both with 32 CLUTs in the first area; 4 at
    /// `0x82000` with 16; other modes keep the addresses. Count and sub-slot
    /// cleared. Returns 0.
    pub fn yi_clut_alloc_init(mem: &mut impl Mem, mode: i32) -> u32 {
        let st = mem.rebase(CLUT_STATE);
        let setup = match mode {
            2 => Some((0xb_4000u32, 0x20u32)),
            3 => Some((0x1_8000, 0x20)),
            4 => Some((0x8_2000, 0x10)),
            _ => None,
        };
        if let Some((base, split)) = setup {
            mem.write(st, [base, base, split]);
        }
        mem.write(st + 0xc, [0u32, 0]);
        0
    }

    /// `yiClutAlloc_get256idx()` (`0x00916e50`): the CLUTs handed out.
    pub fn yi_clut_alloc_get256idx(mem: &impl Mem) -> u32 {
        mem.read(mem.rebase(CLUT_STATE + 0xc))
    }

    /// `yiClutAlloc_copy256(clut, src, k)` (`0x00916d40`): 256 colours
    /// (0x400 bytes) into CLUT `k` (`clut + 0x70 + k * 0x460`). Returns 0.
    pub fn yi_clut_alloc_copy256(mem: &mut impl Mem, clut: u32, src: u32, k: i32) -> u32 {
        let dst = clut
            .wrapping_add(0x70)
            .wrapping_add(k.wrapping_mul(0x460).cast_unsigned());
        for q in 0..0x100 {
            let v: u32 = mem.read(src.wrapping_add(4 * q));
            mem.write(dst.wrapping_add(4 * q), v);
        }
        0
    }

    /// `yiClutAlloc_copy16(clut, src, k, i)` (`0x00916c80`): 16 colours
    /// into slot `i` of CLUT `k`: the GS swizzle's two rows of 8
    /// (`+0x70` and `+0xb0` of `clut + (i % 2 + k * 0x23 + i / 2 * 4) *
    /// 0x20`, C division). Returns 0.
    pub fn yi_clut_alloc_copy16(mem: &mut impl Mem, clut: u32, src: u32, k: i32, i: i32) -> u32 {
        let idx = (i % 2)
            .wrapping_add(k.wrapping_mul(0x23))
            .wrapping_add((i / 2).wrapping_mul(4));
        let base = clut.wrapping_add(idx.wrapping_mul(0x20).cast_unsigned());
        for q in 0..16u32 {
            let v: u32 = mem.read(src.wrapping_add(4 * q));
            let off = if q < 8 {
                0x70 + 4 * q
            } else {
                0xb0 + 4 * (q - 8)
            };
            mem.write(base.wrapping_add(off), v);
        }
        0
    }
    /// One step of a menu layout loader: a scene parameter (scene 1)
    /// fetched into a global.
    #[derive(Clone, Copy)]
    enum SeqOp {
        /// `TkMn2SeqGetPos(name, out)`.
        Pos(u32, u32),
        /// `TkMn2SeqGetRect(name, out)`.
        Rect(u32, u32),
        /// `TkMn2SeqGetColor(name)` stored at `out`.
        Color(u32, u32),
        /// `TkMn2SeqGetNum(name)` stored at `out`.
        Num(u32, u32),
        /// `TkMn2GetCommonPlate(out, rect, c1, c2, a, b)`.
        Plate(u32, [u32; 3], u32, u32),
    }

    fn run_seq_ops(mem: &mut impl Mem, ops: &[SeqOp]) {
        for &op in ops {
            match op {
                SeqOp::Pos(name, out) => {
                    let (n, o) = (mem.rebase(name), mem.rebase(out));
                    tk_mn2_seq_get_pos(mem, n, o);
                }
                SeqOp::Rect(name, out) => {
                    let (n, o) = (mem.rebase(name), mem.rebase(out));
                    tk_mn2_seq_get_rect(mem, n, o);
                }
                SeqOp::Color(name, out) => {
                    let v = tk_mn2_seq_get_color(mem, 1, mem.rebase(name));
                    mem.write(mem.rebase(out), v);
                }
                SeqOp::Num(name, out) => {
                    let v = tk_mn2_seq_get_num(mem, 1, mem.rebase(name));
                    mem.write(mem.rebase(out), v);
                }
                SeqOp::Plate(out, names, a, b) => {
                    let o = mem.rebase(out);
                    let names = names.map(|n| mem.rebase(n));
                    tk_mn2_get_common_plate(mem, 1, o, names, a, b);
                }
            }
        }
    }

    /// The status screen's layout before `num_wpnplt`
    /// ([`tk_mn2_get_status_pos`]).
    const STATUS_LAYOUT_A: &[SeqOp] = &[
        SeqOp::Color(0x00b6_2884, 0x023c_c41c),
        SeqOp::Color(0x00b6_2890, 0x023c_c418),
        SeqOp::Rect(0x00b6_5f24, 0x0186_a83c),
        SeqOp::Pos(0x00b6_5f34, 0x0186_a844),
        SeqOp::Rect(0x00b6_5f44, 0x0186_a848),
        SeqOp::Rect(0x00b6_5f54, 0x023c_c3d8),
        SeqOp::Rect(0x00b6_5f64, 0x023c_c3d0),
        SeqOp::Rect(0x00b6_5f74, 0x0186_a864),
        SeqOp::Pos(0x00b6_5f84, 0x0186_a9f8),
        SeqOp::Rect(0x00b6_5f90, 0x0186_a9c0),
        SeqOp::Pos(0x00b6_5f98, 0x0186_a9c8),
        SeqOp::Color(0x00b6_5fa0, 0x0186_a9cc),
        SeqOp::Color(0x00b6_5fac, 0x0186_a9d0),
        SeqOp::Color(0x00b6_5fb8, 0x0186_a850),
        SeqOp::Color(0x00b6_5fc4, 0x0186_a854),
        SeqOp::Color(0x00b6_5fd4, 0x0186_a858),
        SeqOp::Color(0x00b6_5fe4, 0x0186_a85c),
        SeqOp::Color(0x00b6_5ff4, 0x0186_a860),
        SeqOp::Color(0x00b6_6004, 0x0186_a9bc),
        SeqOp::Pos(0x00b6_6010, 0x0186_a87c),
        SeqOp::Pos(0x00b6_601c, 0x0186_a880),
        SeqOp::Pos(0x00b6_6028, 0x023c_c420),
        SeqOp::Num(0x00b6_6034, 0x0186_a884),
        SeqOp::Plate(
            0x0186_a888,
            [0x00b6_6058, 0x00b6_604c, 0x00b6_6040],
            0xd8,
            0xd9,
        ),
        SeqOp::Plate(
            0x0186_a8a0,
            [0x00b6_607c, 0x00b6_6070, 0x00b6_6064],
            0xd8,
            0xd9,
        ),
        SeqOp::Plate(
            0x0186_a8b8,
            [0x00b6_60a0, 0x00b6_6094, 0x00b6_6088],
            0x109,
            0xd9,
        ),
        SeqOp::Plate(
            0x0186_a8d0,
            [0x00b6_60b8, 0x00b6_60ac, 0x00b6_60ac],
            0x109,
            0xd9,
        ),
        SeqOp::Plate(
            0x0186_a8e8,
            [0x00b6_5f44, 0x00b6_5fb8, 0x00b6_5fb8],
            0x0,
            0x0,
        ),
        SeqOp::Plate(
            0x0186_a900,
            [0x00b6_5f54, 0x00b6_5fc4, 0x00b6_5fd4],
            0xda,
            0xdb,
        ),
        SeqOp::Plate(
            0x0186_a918,
            [0x00b6_5f64, 0x00b6_5fe4, 0x00b6_5ff4],
            0x0,
            0x0,
        ),
        SeqOp::Plate(
            0x0186_a978,
            [0x00b6_5f74, 0x00b6_5fb8, 0x00b6_5fb8],
            0xde,
            0xdf,
        ),
        SeqOp::Plate(
            0x023c_c400,
            [0x00b6_60c4, 0x00b6_27e0, 0x00b6_27f0],
            0xd8,
            0xd9,
        ),
        SeqOp::Pos(0x00b6_60d0, 0x023c_c3e8),
        SeqOp::Pos(0x00b6_60dc, 0x023c_c3e4),
        SeqOp::Pos(0x00b6_5f34, 0x0186_a9b8),
    ];

    /// The status screen's layout after the weapon plates.
    const STATUS_LAYOUT_B: &[SeqOp] = &[
        SeqOp::Pos(0x00b6_60f4, 0x0186_a9a8),
        SeqOp::Pos(0x00b6_6104, 0x0186_a9ac),
        SeqOp::Pos(0x00b6_6110, 0x0186_a9b0),
        SeqOp::Pos(0x00b6_611c, 0x0186_a9b4),
        SeqOp::Pos(0x00b6_612c, 0x0186_a86c),
        SeqOp::Pos(0x00b6_6138, 0x0186_a870),
        SeqOp::Pos(0x00b6_6144, 0x0186_a874),
        SeqOp::Pos(0x00b6_6104, 0x0186_a878),
        SeqOp::Num(0x00b6_6150, 0x0186_a9e8),
        SeqOp::Num(0x00b6_615c, 0x0186_a9ec),
        SeqOp::Num(0x00b6_6168, 0x0186_a9f0),
        SeqOp::Pos(0x00b6_6174, 0x0186_a9f4),
        SeqOp::Rect(0x00b6_6180, 0x0186_a9d8),
        SeqOp::Pos(0x00b6_6190, 0x0186_a9e0),
        SeqOp::Pos(0x00b6_619c, 0x0186_a9e4),
        SeqOp::Pos(0x00b6_61a8, 0x023c_c3e0),
    ];

    /// The weapon plates and where `TkMn2GetStatusPos` copies them.
    const WEAPON_PLATE_COPIES: [(u32, u32); 4] = [
        (0x0186_a8e8, 0x0186_a930),
        (0x0186_a900, 0x0186_a948),
        (0x0186_a918, 0x0186_a960),
        (0x0186_a978, 0x0186_a990),
    ];

    /// `TkMn2GetStatusPos()` (`0x008d4920`): the status
    /// screen's layout from scene 1 (menu background colours, name and
    /// weapon plates, limit gauge, faces, ...); the four weapon plates are
    /// then copied (24 bytes each, [`WEAPON_PLATE_COPIES`]), the copies
    /// moved down by `num_wpnplt`
    /// (also kept at `0x0186a9d4`).
    pub fn tk_mn2_get_status_pos(mem: &mut impl Mem) {
        run_seq_ops(mem, STATUS_LAYOUT_A);
        let dy = tk_mn2_seq_get_num(mem, 1, mem.rebase(0x00b6_60e8)).cast_unsigned();
        mem.write(mem.rebase(0x0186_a9d4), dy);
        for (src, dst) in WEAPON_PLATE_COPIES {
            let (s, d) = (mem.rebase(src), mem.rebase(dst));
            let v: [u32; 6] = mem.read(s);
            mem.write(d, v);
        }
        for (src, dst) in WEAPON_PLATE_COPIES {
            let y: u16 = mem.read(mem.rebase(src + 2));
            #[allow(clippy::cast_possible_truncation)]
            mem.write(mem.rebase(dst + 2), y.wrapping_add(dy as u16));
        }
        run_seq_ops(mem, STATUS_LAYOUT_B);
    }
    /// `Vibrate_SetStreamsMode(rec, mode, a, b)` (`0x00876130`, own name):
    /// mode 1 sets both entries from curve headers ([`vibrate_set_streams`])
    /// and the record's mode `+0x22` to 1; other modes only clear the
    /// mode. Returns the data pointer, or the record.
    pub fn vibrate_set_streams_mode(
        mem: &mut impl Mem,
        rec: u32,
        mode: i32,
        a: u32,
        b: u32,
    ) -> u32 {
        if mode != 1 {
            mem.write(rec + 0x22, 0u8);
            return rec;
        }
        let r = vibrate_set_streams(mem, rec, a, b);
        mem.write(rec + 0x22, 1u8);
        r
    }

    /// `TkSetVibrateStream(k, curve, table)` (`0x00875fb0`): stream `k`
    /// (0..=7) restarted on curve `curve` of `table`: entry 0 from the
    /// header past its first part (when its second halfword is non-zero),
    /// entry 1 from the header at `+4` (when its first is); mode 1.
    /// Returns the second entry's data pointer.
    pub fn tk_set_vibrate_stream(mem: &mut impl Mem, k: i32, curve: u32, table: u32) -> u32 {
        let k = k.clamp(0, 7).cast_unsigned();
        let rec = mem.rebase(0x0132_8150) + k * 0x24;
        let n: u16 = mem.read(rec + 0x20);
        mem.write(rec + 0x20, n.wrapping_add(1));
        mem.write(rec + 0x22, 0u8);
        let base = if curve < 0x40 && table != 0 {
            match mem.read::<u32>(table.wrapping_add(4 * curve)) {
                0 => None,
                off => Some(off.wrapping_add(table)),
            }
        } else {
            None
        };
        let second = match base {
            Some(b) if mem.read::<u16>(b) != 0 => b.wrapping_add(4),
            _ => 0,
        };
        let first = match base {
            Some(b) if mem.read::<u16>(b.wrapping_add(2)) != 0 => u32::from(mem.read::<u16>(b))
                .wrapping_add(4)
                .wrapping_add(b),
            _ => 0,
        };
        let r = vibrate_set_streams(mem, rec, first, second);
        mem.write(rec + 0x22, 1u8);
        r
    }
    /// A byte global read zero-extended (`movzx eax, byte [g]`), as
    /// `TkGetSavedLang` (`0x0088c850`, `0x01330874`) does.
    pub fn get_byte(mem: &impl Mem, g: u32) -> u32 {
        u32::from(mem.read::<u8>(mem.rebase(g)))
    }

    /// `Ch_RequestCalcNormal()` (`0x00832540`, own name): byte
    /// `0x013009e4` = 1, the count of deferred normal recalculations the
    /// character draw takes one from.
    pub fn ch_request_calc_normal(mem: &mut impl Mem) {
        mem.write(mem.rebase(0x0130_09e4), 1u8);
    }

    /// `TkGetControlerBindAnalogIndex(pad, port, code)` (`0x00888cf0`): codes
    /// 4..=11 (the buttons after the four directions) through the pad's
    /// button map (`TkFindPadButton`) and back to a code by its position in
    /// `g_PadButtonCodes` (`0x00c54e28`) plus 4; 0 for an unmapped button;
    /// other codes unchanged.
    pub fn tk_get_controler_bind_analog_index(
        mem: &impl Mem,
        pad: i32,
        port: i32,
        code: i32,
    ) -> i32 {
        if !(4..0xc).contains(&code) {
            return code;
        }
        let b = tk_find_pad_button(mem, pad, port, code - 4);
        if b < 0 {
            return 0;
        }
        let codes = mem.rebase(0x00c5_4e28);
        let mut k = 0u32;
        while i32::from(mem.read::<i8>(codes + k)) != b {
            k += 1;
        }
        k.cast_signed() + 4
    }

    /// `Atel_FindWorkerByOwner(owner)` (`0x00876440`, own name): the
    /// first active worker (`+0xaa` 1) of the current control's
    /// `+0xc` workers whose `+0x9c` is `owner`: its `u16 +0x2e`; -1 if
    /// none.
    pub fn atel_find_worker_by_owner(mem: &mut impl Mem, owner: u32) -> i32 {
        let ctrl = ffx_script::vm::bridge::atel_get_cur_ctrl_work(mem);
        let n = i32::from(mem.read::<u16>(ctrl + 0xc));
        for k in 0..n {
            let w = ffx_script::vm::bridge::atel_get_basic_work(mem, k);
            if mem.read::<u8>(w + 0xaa) == 1 && mem.read::<u32>(w + 0x9c) == owner {
                return i32::from(mem.read::<u16>(w + 0x2e));
            }
        }
        -1
    }

    /// `TkInitSceneFromSave()` (`0x0088dd60`, own name): fade and
    /// `0x01330854` cleared; without a pending load (`0x00c33480` is -1)
    /// `0x01327108` = 1; otherwise the save's event block takes its stored
    /// map and entrance back (halfwords `+0xba` to `+0`, `+0xc2` to `+2`,
    /// bytes `+0xb8` to `+0xc` (then cleared) and `+0xce` to `+0xd`) and
    /// `0x0133084c` = 1. `0x01330858` cleared either way.
    pub fn tk_init_scene_from_save(mem: &mut impl Mem) {
        let g = |mem: &mut _, a: u32| Mem::rebase(mem, a);
        let at = g(mem, 0x00c3_3480);
        let pending: i32 = mem.read(at);
        let a = g(mem, 0x0133_0850);
        mem.write(a, [0u32, 0]);
        if pending == -1 {
            let x = g(mem, 0x0132_7108);
            mem.write(x, 1u32);
        } else {
            let e = ffx_battle::save::bridge::ms_get_save_event_address(mem);
            let b8: u16 = mem.read(e + 0xb8);
            let ba: u16 = mem.read(e + 0xba);
            mem.write(e + 0xb8, 0u16);
            let c2: u16 = mem.read(e + 0xc2);
            mem.write(e + 2, c2);
            let ce: u8 = mem.read(e + 0xce);
            mem.write(e, ba);
            #[allow(clippy::cast_possible_truncation)]
            mem.write(e + 0xc, b8 as u8);
            mem.write(e + 0xd, ce);
            let x = g(mem, 0x0133_084c);
            mem.write(x, 1u32);
        }
        let x = g(mem, 0x0133_0858);
        mem.write(x, 0u32);
    }
    /// `TkMenuDrawMenuWindowScissor(window, mode, t)` (`0x008dd220`): the
    /// scissor (`TODrawScissorXYWH(0, y, 0x1ff, h)`, through the
    /// `TkMenuDrawScissorRect` thunk) for a window's (`+0x14` y, `+0x1c`
    /// height) inside: mode 1 opens from the bottom and mode 2 from the
    /// top by `|t| / 4096` of the height (the other end for negative `t`),
    /// other modes the whole inside. Returns the scissor's result.
    pub fn tk_menu_draw_menu_window_scissor(mem: &mut impl Mem, w: u32, mode: i32, t: i32) -> u32 {
        let wy: i32 = mem.read(w + 0x14);
        let wh: i32 = mem.read(w + 0x1c);
        let part = || t.wrapping_abs().wrapping_mul(wh) / 0x1000;
        let (y, h) = match mode {
            1 => {
                let d = part();
                let y = if t < 0 {
                    wy
                } else {
                    wy.wrapping_sub(d).wrapping_add(3).wrapping_add(wh)
                };
                (y, d.wrapping_sub(6).max(0))
            }
            2 => {
                let d = part();
                let y = if t < 0 {
                    wy.wrapping_add(3).wrapping_add(d)
                } else {
                    wy.wrapping_add(3)
                };
                (y, wh.wrapping_sub(d).wrapping_sub(6).max(0))
            }
            _ => (wy.wrapping_add(3), wh.wrapping_sub(6)),
        };
        ffx_system::pkt::bridge::draw_scissor_xywh(mem, 0, y, 0x1ff, h)
    }

    /// `Vibrate_StepLevel(rec, entry, len)` (`0x00875c90`, own name): one
    /// frame of a stream entry: its step `+4` advanced; at `len` a curve
    /// stream (mode 1) moves to its next part ([`vibrate_start_stream`]
    /// from the data pointer, when set) and a level (mode 2) settles on its
    /// target. Returns the level now: the eased value between `+8` and
    /// `+9` over `+6` steps, or `+8` (also for a negative `len`, which
    /// changes nothing).
    pub fn vibrate_step_level(mem: &mut impl Mem, rec: u32, entry: u32, len: i32) -> u32 {
        if len < 0 {
            return u32::from(mem.read::<u8>(entry + 8));
        }
        let step = mem.read::<u16>(entry + 4).wrapping_add(1);
        mem.write(entry + 4, step);
        if i32::from(step.cast_signed()) >= len {
            match mem.read::<u8>(rec + 0x22) {
                1 => {
                    let data: u32 = mem.read(entry);
                    if data != 0 {
                        vibrate_start_stream(mem, entry, data);
                    }
                }
                2 => {
                    let to: u8 = mem.read(entry + 9);
                    mem.write(entry + 8, to);
                    mem.write(entry + 6, 0xfffeu16);
                }
                _ => {}
            }
        }
        let (from, to) = (mem.read::<u8>(entry + 8), mem.read::<u8>(entry + 9));
        let n = mem.read::<i16>(entry + 6);
        if from != to && n > 0 {
            let n = i32::from(n);
            let s = i32::from(mem.read::<i16>(entry + 4));
            let v = (n - s)
                .wrapping_mul(i32::from(from))
                .wrapping_add(i32::from(to).wrapping_mul(s));
            return (v / n).cast_unsigned();
        }
        u32::from(from)
    }
    /// `TkEucToSjisString(src, dst, n)` (`0x0088ed10`, own name): up to `n`
    /// bytes of EUC text converted ([`tk_euc_to_sjis`] a character at a
    /// time) and terminated. Returns where the source stopped, or 0 when
    /// the text ended first (or `src` is null).
    pub fn tk_euc_to_sjis_string(mem: &mut impl Mem, src: u32, dst: u32, n: i32) -> u32 {
        if src == 0 {
            return 0;
        }
        let (mut s, mut d, mut n) = (src, dst, n);
        loop {
            let k = tk_euc_to_sjis(mem, s, d);
            n = n.wrapping_sub(k.cast_signed());
            s = s.wrapping_add(k);
            d = d.wrapping_add(k);
            if n < 1 {
                break;
            }
            if k == 0 {
                return 0;
            }
        }
        mem.write(d, 0u8);
        s
    }
    /// `TkMenuDrawMenuWindowScissorWithLabel(window, mode, t)`
    /// (`0x008dd2d0`): [`tk_menu_draw_menu_window_scissor`] with the
    /// window's inside lowered 10 pixels for its label (`+0x14` and
    /// `+0x1c` changed for the call and put back).
    pub fn tk_menu_draw_menu_window_scissor_with_label(
        mem: &mut impl Mem,
        window: u32,
        mode: i32,
        t: i32,
    ) -> u32 {
        let height: u32 = mem.read(window + 0x1c);
        let top: u32 = mem.read(window + 0x14);
        mem.write(window + 0x14, top.wrapping_add(10));
        mem.write(window + 0x1c, height.wrapping_sub(10));
        let r = tk_menu_draw_menu_window_scissor(mem, window, mode, t);
        mem.write(window + 0x14, top);
        mem.write(window + 0x1c, height);
        r
    }

    /// `Vibrate_EntryLevel(rec, e)` (`0x00875c20`, own name): entry `e`'s
    /// level this frame, also kept in its `+0xa`: 0 outside modes 1 and
    /// 2, the start level for a negative length, 0 for a curve part of
    /// 0x200 or more, otherwise [`vibrate_step_level`] (whose whole result
    /// is returned).
    pub fn vibrate_entry_level(mem: &mut impl Mem, rec: u32, e: u32) -> u32 {
        let entry = rec.wrapping_add(e.wrapping_shl(4));
        let mode: u8 = mem.read(rec + 0x22);
        let len = i32::from(mem.read::<i16>(entry.wrapping_add(6)));
        let r = if mode.wrapping_sub(1) > 1 {
            0
        } else if len < 0 {
            u32::from(mem.read::<u8>(entry.wrapping_add(8)))
        } else if mode == 1 && len >= 0x200 {
            0
        } else {
            vibrate_step_level(mem, rec, entry, len)
        };
        #[allow(clippy::cast_possible_truncation)]
        mem.write(entry.wrapping_add(0xa), r as u8);
        r
    }
    /// The save file names' format (`0x023cc0a0`: the product code and
    /// `FF0906%02d`, set by `TOMenuMakeSaveFileName`'s caller) applied to
    /// `n`; `None` when it holds a conversion the formatter lacks.
    fn save_name(mem: &impl Mem, n: u32) -> Option<Vec<u8>> {
        let f = ffx_text::printf::c_string(mem, mem.rebase(0x023c_c0a0));
        ffx_text::printf::format(mem, &f, &mut || n)
    }

    /// The icon file name `010906%02d.ico` (`0x00c5b3fc`) for `n`.
    fn icon_name(mem: &impl Mem, n: u32) -> Vec<u8> {
        let f = ffx_text::printf::c_string(mem, mem.rebase(0x00c5_b3fc));
        ffx_text::printf::format(mem, &f, &mut || n).unwrap_or_default()
    }

    /// The memory-card paths the save menu builds into `out` for slot
    /// `n` (`S` the save name): `/S` (`MenuSave_PathDir`, `0x008b3ff0`),
    /// `/S/S` (`MenuSave_PathData`, `0x008b4060`), `/S/010906nn.ico`
    /// (`MenuSave_PathIcon`, `0x008b4110`) and `/S/icon.sys`
    /// (`MenuSave_PathIconSys`, `0x008b4230`); own names. The original
    /// builds them with `sprintf` and `strcat`s, ending with the text and
    /// its terminator. `false` (nothing written) when the save name's
    /// format is beyond [`ffx_text::printf`].
    pub fn menu_save_path(mem: &mut impl Mem, out: u32, n: u32, kind: u8) -> bool {
        let Some(name) = save_name(mem, n) else {
            return false;
        };
        let mut p = vec![b'/'];
        p.extend(&name);
        match kind {
            1 => {
                p.push(b'/');
                p.extend(&name);
            }
            2 => {
                p.push(b'/');
                p.extend(icon_name(mem, n));
            }
            3 => p.extend(b"/icon.sys"),
            _ => {}
        }
        p.push(0);
        mem.write_bytes(out, &p);
        true
    }

    /// `MenuSave_IconName(out, n)` (`0x008b41e0`, own name):
    /// `010906%02d.ico` for `n` copied to `out`.
    pub fn menu_save_icon_name(mem: &mut impl Mem, out: u32, n: u32) {
        let mut s = icon_name(mem, n);
        s.push(0);
        mem.write_bytes(out, &s);
    }
    /// `MenuKaizou_InitList(w)` (`0x008d55e0`, own name): the
    /// customise menu's weapon list (`MsGetSaveWeaponList(-1, -1, 0)` at
    /// `0x0186aa38`) and its length up to the last entry whose slot byte
    /// `+2` is set (`0x0186aa3c`); `+0x1c` cleared, the status background
    /// set up. (The original counts into its own argument slot; the port
    /// borrows `+0x1c`, which it clears anyway.)
    pub fn menu_kaizou_init_list(mem: &mut impl Mem, w: u32) {
        mem.write(w + 0x1c, 0u32);
        let list = ffx_battle::save::bridge::ms_get_save_weapon_list(mem, -1, -1, 0, w + 0x1c);
        let n: i32 = mem.read(w + 0x1c);
        mem.write(w + 0x1c, 0u32);
        mem.write(mem.rebase(0x0186_aa38), list);
        let mut k = n - 1;
        while k > 0 {
            let id = u32::from(mem.read::<u16>(list.wrapping_add((2 * k).cast_unsigned())));
            let slot = ffx_battle::save::bridge::ms_get_save_weapon(mem, id, 0);
            if mem.read::<u8>(slot + 2) != 0 {
                k += 1;
                break;
            }
            k -= 1;
        }
        mem.write(mem.rebase(0x0186_aa3c), k);
        tk_mn2_init_status_bg(mem);
    }

    /// `MenuSummon_GetStat(base, desc, max)` (`0x008ceca0`): a summon
    /// stat field of the record at `base`, by its descriptor (`u16` offsets
    /// `+0xc` for the maximum, `+0xe` for the current value; `+0x10` 4 for
    /// a `u32`, else a byte).
    pub fn menu_summon_get_stat(mem: &impl Mem, base: u32, desc: u32, max: u32) -> u32 {
        let off = u32::from(mem.read::<u16>(desc + if max != 0 { 0xc } else { 0xe }));
        if mem.read::<u32>(desc + 0x10) == 4 {
            mem.read(base.wrapping_add(off))
        } else {
            u32::from(mem.read::<u8>(base.wrapping_add(off)))
        }
    }

    /// `MenuSummon_SetStat(base, desc, max, v)` (`0x008cee70`): the store
    /// [`menu_summon_get_stat`] reads.
    pub fn menu_summon_set_stat(mem: &mut impl Mem, base: u32, desc: u32, max: u32, v: u32) {
        let off = u32::from(mem.read::<u16>(desc + if max != 0 { 0xc } else { 0xe }));
        if mem.read::<u32>(desc + 0x10) == 4 {
            mem.write(base.wrapping_add(off), v);
        } else {
            #[allow(clippy::cast_possible_truncation)] // the low byte, as stored
            mem.write(base.wrapping_add(off), v as u8);
        }
    }

    /// `Menu_FindByte(bytes, v, from, to)` (`0x008c9bc0`): the first index
    /// in `from..to` whose byte is `v`, else -1.
    pub fn menu_find_byte(mem: &impl Mem, bytes: u32, v: u32, from: i32, to: i32) -> i32 {
        (from..to)
            .find(|&i| u32::from(mem.read::<u8>(bytes.wrapping_add(i.cast_unsigned()))) == v)
            .unwrap_or(-1)
    }

    /// `Menu_FindFlagged(entries, from, to)` (`0x008c9b90`): the first index
    /// in `from..to` whose 4-byte entry has byte `+2` set, else -1.
    pub fn menu_find_flagged(mem: &impl Mem, entries: u32, from: i32, to: i32) -> i32 {
        (from..to)
            .find(|&i| {
                mem.read::<u8>(entries.wrapping_add(4u32.wrapping_mul(i.cast_unsigned())) + 2) != 0
            })
            .unwrap_or(-1)
    }

    /// `Menu_FadeStep(obj)` (`0x008e2b30`): `+0x64` += `+0x68`, at least 0
    /// and at most 0x80. Returns the value before the upper clamp (EAX).
    pub fn menu_fade_step(mem: &mut impl Mem, obj: u32) -> u32 {
        let v = mem
            .read::<i32>(obj + 0x64)
            .wrapping_add(mem.read(obj + 0x68))
            .max(0);
        mem.write(obj + 0x64, v.min(0x80));
        v.cast_unsigned()
    }

    /// The menu item key list `Menu_ParseItemKeys` fills (32 entries of 4
    /// bytes, `g_TkMenuItemKeyUse`).
    pub const ITEM_KEYS: u32 = 0x0186_a280;

    /// `Menu_ParseItemKeys(s)` (`0x008c5280`): splits a key string into
    /// the [`ITEM_KEYS`] entries: a byte, then (kind 4) a second byte, and
    /// for a value in 6..=0x2f one argument byte; the unused entries after
    /// the last are cleared (their first byte).
    #[allow(clippy::many_single_char_names)] // string, pointer, entry, bytes
    pub fn menu_parse_item_keys<M: Mem>(mem: &mut M, s: u32) {
        let table = mem.rebase(ITEM_KEYS);
        let mut p = s;
        let mut i = 0u32;
        let next = |mem: &M, p: &mut u32| {
            let b: u8 = mem.read(*p);
            *p = p.wrapping_add(1);
            b
        };
        let mut b = next(mem, &mut p);
        while b != 0 {
            let e = table.wrapping_add(4 * i);
            mem.write(e, b);
            if b == 4 {
                let k = next(mem, &mut p);
                mem.write(e + 1, k);
                if k.wrapping_sub(6) <= 0x29 {
                    let a = next(mem, &mut p);
                    mem.write(e + 2, a);
                    mem.write(e + 3, 0u8);
                } else {
                    mem.write(e + 2, 0u8);
                }
            } else if b.wrapping_sub(6) <= 0x29 {
                let a = next(mem, &mut p);
                mem.write(e + 1, a);
                mem.write(e + 2, 0u8);
            } else {
                mem.write(e + 1, 0u8);
            }
            b = next(mem, &mut p);
            i += 1;
        }
        while i < 0x20 {
            mem.write(table.wrapping_add(4 * i), 0u8);
            i += 1;
        }
    }

    /// `g_TkMenuSceneData` (pointers per scene).
    pub const MENU_SCENE_DATA: u32 = 0x022f_b4dc;

    /// `MenuScene_ResetGroup(scene, off, on)` (`0x00a7ba70`): the group at
    /// `off` in scene `scene`'s data (none for 0 or no data: 0) gets its
    /// list offset's top bits cleared; each of its 20-byte entries is shown
    /// (bit 31 set) or hidden (bits 30, 31 cleared) and reset (`+8` 0,
    /// `u16` `+0xc` -1, `u16` `+4` 0, `+0xe` 0). Returns the group.
    #[allow(clippy::many_single_char_names)] // group, value, entry, word, count
    pub fn menu_scene_reset_group(mem: &mut impl Mem, scene: u32, off: u32, on: u32) -> u32 {
        if off == 0 {
            return 0;
        }
        let at = mem
            .rebase(MENU_SCENE_DATA)
            .wrapping_add(4u32.wrapping_mul(scene));
        let data: u32 = mem.read(at);
        if data == 0 {
            return 0;
        }
        let g = data.wrapping_add(off);
        if g == 0 {
            return 0;
        }
        let v = mem.read::<u32>(g + 4) & 0x3fff_ffff;
        mem.write(g + 4, v);
        let list = (mem.read::<u32>(g + 4) & 0x3fff_ffff).wrapping_add(mem.read::<u32>(at));
        let n: i32 = mem.read(list);
        let mut e = list.wrapping_add(4);
        for _ in 0..n.max(0) {
            let w: u32 = mem.read(e);
            mem.write(
                e,
                if on == 0 {
                    w & 0x3fff_ffff
                } else {
                    w | 0x8000_0000
                },
            );
            mem.write(e + 8, 0u32);
            mem.write(e + 0xc, 0xffffu16);
            mem.write(e + 4, 0u16);
            mem.write(e + 0xe, 0u32);
            e = e.wrapping_add(0x14);
        }
        g
    }

    /// The party-select cursor (`i8`), the slot count (`i32`) and the slot
    /// pointers (`0x0186a44c`, each with `+0xc` 1 when selectable).
    pub const PARTY_CURSOR: u32 = 0x0186_a444;
    pub const PARTY_COUNT: u32 = 0x0186_a448;
    pub const PARTY_SLOTS: u32 = 0x0186_a44c;

    /// `MenuParty_CursorPrev()` (`0x008cba60`): the cursor moves back
    /// (wrapping to the last slot) to the previous selectable slot.
    pub fn menu_party_cursor_prev(mem: &mut impl Mem) {
        let (cur, n) = (
            mem.rebase(PARTY_CURSOR),
            mem.read::<i32>(mem.rebase(PARTY_COUNT)),
        );
        let slots: u32 = mem.read(mem.rebase(PARTY_SLOTS));
        let mut c: i8 = mem.read(cur);
        loop {
            c = c.wrapping_sub(1);
            mem.write(cur, c);
            if c < 0 {
                #[allow(clippy::cast_possible_truncation)] // the low byte
                {
                    c = n.wrapping_sub(1) as i8;
                }
                mem.write(cur, c);
            }
            let s: u32 =
                mem.read(slots.wrapping_add(4u32.wrapping_mul(i32::from(c).cast_unsigned())));
            if mem.read::<u32>(s.wrapping_add(0xc)) == 1 {
                return;
            }
        }
    }

    /// `MenuParty_CursorNext()` (`0x008cba20`): the cursor moves on
    /// (wrapping to 0) to the next selectable slot.
    pub fn menu_party_cursor_next(mem: &mut impl Mem) {
        let (cur, n) = (
            mem.rebase(PARTY_CURSOR),
            mem.read::<i32>(mem.rebase(PARTY_COUNT)),
        );
        let slots: u32 = mem.read(mem.rebase(PARTY_SLOTS));
        let mut c: i8 = mem.read(cur);
        loop {
            c = c.wrapping_add(1);
            if i32::from(c) >= n {
                c = 0;
            }
            let s: u32 =
                mem.read(slots.wrapping_add(4u32.wrapping_mul(i32::from(c).cast_unsigned())));
            if mem.read::<u32>(s.wrapping_add(0xc)) == 1 {
                break;
            }
        }
        mem.write(cur, c);
    }

    /// `Menu_Scale32(a, b)` (`0x008bf100`): `a + a × b / 32`, the division
    /// truncating toward 0.
    #[must_use]
    pub fn menu_scale32(a: i32, b: i32) -> i32 {
        a.wrapping_mul(b) / 32 + a
    }

    /// `Menu_EaseCounter(-, counter, a, b)` (`0x008bf7e0`): a non-zero
    /// counter moves 0x199 toward 0; reaching or passing 0 it becomes 0 and
    /// `*a` = `*b` (taken before).
    #[allow(clippy::many_single_char_names)] // the two targets, counter, value
    pub fn menu_ease_counter(mem: &mut impl Mem, counter: u32, a: u32, b: u32) {
        let c: i32 = mem.read(counter);
        let v: u32 = mem.read(b);
        if c > 0 {
            let n = c.wrapping_sub(0x199);
            mem.write(counter, n);
            if n <= 0 {
                mem.write(counter, 0u32);
                mem.write(a, v);
                mem.write(b, v);
            }
        } else if c != 0 {
            let n = c.wrapping_add(0x199);
            mem.write(counter, n);
            if n >= 0 {
                mem.write(counter, 0u32);
                mem.write(a, v);
                mem.write(b, v);
            }
        }
    }

    /// `Menu_CursorEase(summon, counter, a, b, last)` (`0x008bf750`, own
    /// name): `Menu_EaseCounter` for the party cursor (the summon one with
    /// `summon` 1): the counter moves 0x199 toward 0 and, reaching it,
    /// `*a` = `*b` = the current position; then a position other than
    /// `*last` (read first) restarts it at the step (`TkMenuGetSummonStep`
    /// / `TkMenuGetPlayerStep`) `<< 12`, with `*a` = the old position and
    /// `*b` = `*last` = the new one.
    #[allow(clippy::many_single_char_names, clippy::comparison_chain)]
    pub fn menu_cursor_ease<M: Mem>(
        mem: &mut M,
        summon: bool,
        [counter, a, b, last]: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(if summon { 0x008a_9890 } else { 0x008a_9870 });
        let pos = call(mem, f, &[]);
        let c: i32 = mem.read(counter);
        let old: u32 = mem.read(last);
        let done = if c > 0 {
            let n = c.wrapping_sub(0x199);
            mem.write(counter, n);
            n <= 0
        } else if c < 0 {
            let n = c.wrapping_add(0x199);
            mem.write(counter, n);
            n >= 0
        } else {
            false
        };
        if done {
            mem.write(counter, 0u32);
            mem.write(a, pos);
            mem.write(b, pos);
        }
        if old != pos {
            let f = mem.rebase(if summon { 0x008a_9bb0 } else { 0x008a_9840 });
            let step = call(mem, f, &[]);
            mem.write(counter, step << 12);
            mem.write(a, old);
            mem.write(b, pos);
            mem.write(last, pos);
        }
    }

    /// `TkSetEventJump(map, entrance)` (`0x0088ea60`): with `map` the
    /// pending value `0x00c33480`, only `0x01327108` = 1. Otherwise the
    /// save's event block (`MsGetSaveEventAddress()`): map 0x17 first
    /// destroys every effect (`graphicVFXDestroyAll`,
    /// `fiosDeleteCompletedOps`); a negative map takes the saved map
    /// (`+0xba`) and entrance (`+0xb8`, then cleared). The previous map
    /// (`+0`) and entrance (`+0xc`) move to `+2`/`+0xd` (for a negative
    /// map `+0xc2`/`+0xce` instead), the new ones in; then
    /// `g_TkEventJumpRequest` = 1.
    pub fn tk_set_event_jump<M: Mem>(
        mem: &mut M,
        map: i32,
        entrance: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let pending = mem.rebase(0x00c3_3480);
        if mem.read::<i32>(pending) == map {
            let at = mem.rebase(0x0132_7108);
            mem.write(at, 1u32);
            return;
        }
        let f = mem.rebase(0x0078_5240);
        let s = call(mem, f, &[]);
        let (m, e) = if map < 0 {
            let e: u16 = mem.read(s.wrapping_add(0xb8));
            let m: u16 = mem.read(s.wrapping_add(0xba));
            mem.write(s.wrapping_add(0xb8), 0u16);
            (m, e.to_le_bytes()[0])
        } else {
            if map == 0x17 {
                for f in [0x0064_5310, 0x0067_9560] {
                    let f = mem.rebase(f);
                    call(mem, f, &[]);
                }
            }
            let [lo, hi, ..] = map.to_le_bytes();
            (u16::from_le_bytes([lo, hi]), entrance.to_le_bytes()[0])
        };
        let (prev_map, prev_entrance) = if map < 0 {
            (
                mem.read::<u16>(s.wrapping_add(0xc2)),
                mem.read::<u8>(s.wrapping_add(0xce)),
            )
        } else {
            (mem.read::<u16>(s), mem.read::<u8>(s.wrapping_add(0xc)))
        };
        mem.write(s.wrapping_add(2), prev_map);
        mem.write(s.wrapping_add(0xd), prev_entrance);
        mem.write(s, m);
        mem.write(s.wrapping_add(0xc), e);
        let at = mem.rebase(0x0133_084c);
        mem.write(at, 1u32);
    }

    /// `MenuItem_CursorTable()` (`0x008c4ff0`, own name): one of four
    /// tables by language (`TOGetFFXLang` through `0x008ac2f0`: Japanese
    /// or not) and whether the item menu has a target
    /// (`0x0186a258` >= 0): `0x00c5a19c`/`0x00c5a1cc` (Japanese, with and
    /// without), `0x00c5a188`/`0x00c5a1e4`.
    pub fn menu_item_cursor_table<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_c2f0);
        let lang = call(mem, f, &[]);
        let target = MENU_ITEM_TARGET.get(mem).cast_signed() >= 0;
        mem.rebase(match (lang == 0, target) {
            (true, true) => 0x00c5_a19c,
            (true, false) => 0x00c5_a1cc,
            (false, true) => 0x00c5_a188,
            (false, false) => 0x00c5_a1e4,
        })
    }

    /// `MenuItem_CursorCount()` (`0x008c5030`, own name): the entries of
    /// [`menu_item_cursor_table`] (4 bytes each) before the first whose
    /// `u16` is 0.
    pub fn menu_item_cursor_count<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = menu_item_cursor_table(mem, call);
        let mut n = 0u32;
        while mem.read::<u16>(table.wrapping_add(4 * n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `MenuItem_CellTable()` (`0x008c4f80`, own name): with an item-menu
    /// target `t` (`0x0186a258` >= 0) entry `t` of `0x00c5a0d8` (Japanese,
    /// `MsGetSaveConfigEnglish` asked but unused) or `0x00c5a120`; without
    /// one `0x00c5a0d4` (Japanese, not English) or `0x00c5a168`.
    pub fn menu_item_cell_table<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let target = MENU_ITEM_TARGET.get(mem);
        let f = mem.rebase(0x008a_c2f0);
        let lang = call(mem, f, &[]);
        let english = if lang == 0 {
            let f = mem.rebase(0x0078_51d0);
            call(mem, f, &[]) != 0
        } else {
            true
        };
        if target.cast_signed() < 0 {
            return mem.rebase(if english { 0x00c5_a168 } else { 0x00c5_a0d4 });
        }
        let table = mem.rebase(if lang == 0 { 0x00c5_a0d8 } else { 0x00c5_a120 });
        table.wrapping_add(target.wrapping_mul(4))
    }

    /// `MenuShop_Price(id)` (`0x008db3b0`, own name): the first word of the
    /// item's record (`TkMn2GetExcelData(id & 0xfff, [0x0186ac28])`) times
    /// the shop ratio (`0x01326b6c`) / 100, truncated (32-bit product); at
    /// least 1.
    pub fn menu_shop_price<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let table = mem.read::<u32>(mem.rebase(0x0186_ac28));
        let f = mem.rebase(0x008c_1af0);
        let rec = call(mem, f, &[id & 0xfff, table]);
        let v: i32 = mem.read(rec);
        let f = mem.rebase(0x0086_c480);
        let ratio = call(mem, f, &[]).cast_signed();
        let p = ratio.wrapping_mul(v) / 100;
        if p == 0 { 1 } else { p }
    }

    /// `TkMenuMainRemoveMenu(slot)` (`0x008aab20`): the slot (clamped to
    /// `0..=24`) emptied, its window's close callback `+0x14` called, its
    /// bit cleared from the active mask, then `TOMenuClearControlPadBuff()`,
    /// `TOSetExeMessageWindowArea(0)` (its result returned) and
    /// `0x025d16c0` = 0.
    pub fn tk_menu_main_remove_menu<M: Mem>(
        mem: &mut M,
        slot: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slot = slot.clamp(0, 0x18);
        let at = slot_at(mem, slot);
        let w: u32 = mem.read(at);
        mem.write(at, 0u32);
        if w != 0 {
            let f = mem.read::<u32>(w.wrapping_add(0x14));
            if f != 0 {
                call(mem, f, &[w]);
            }
        }
        let m = MENU_MAIN_ACTIVE_MASK.get(mem) & !(1u32 << slot);
        MENU_MAIN_ACTIVE_MASK.set(mem, m);
        let f = mem.rebase(0x008b_e3d0);
        call(mem, f, &[]);
        let f = mem.rebase(0x008a_f5c0);
        let r = call(mem, f, &[0]);
        let at = mem.rebase(0x025d_16c0);
        mem.write(at, 0u32);
        r
    }

    /// `TkCh_BindToElem(kind, a, chr, b, keep)` (`0x008722b0`): kind 7 with
    /// a character and `keep` 0 first hides it (`Ch_ModelSetHide(chr, 1,
    /// 1)`); then `Ch_BindToElem(a, chr, b)`, whose result is returned.
    pub fn tk_ch_bind_to_elem<M: Mem>(
        mem: &mut M,
        [kind, a, chr, b, keep]: [u32; 5],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if kind == 7 && chr != 0 && keep == 0 {
            let f = mem.rebase(0x0082_aea0);
            call(mem, f, &[chr, 1, 1]);
        }
        let f = mem.rebase(0x0083_2630);
        call(mem, f, &[a, chr, b])
    }

    /// `MenuNameEntry_RegistWindow()` (`0x008c4450`, own name): a window
    /// from `TkMenuMainAllocWindow()` with the name-entry callbacks
    /// (`+8` `0x008c3540`, `+0xc` `0x008c3cf0`, `+0x10` `0x008c4030`,
    /// `+0x14` `0x008c44c0`, `+0x1c` `0x008c3550`) and `+0x3e` = 3,
    /// registered (`TkMenuMainRegistWindow`); returns it.
    pub fn menu_name_entry_regist_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_a1a0);
        let w = call(mem, f, &[]);
        for (o, f) in [
            (8u32, 0x008c_3540u32),
            (0xc, 0x008c_3cf0),
            (0x10, 0x008c_4030),
            (0x14, 0x008c_44c0),
            (0x1c, 0x008c_3550),
        ] {
            let f = mem.rebase(f);
            mem.write(w.wrapping_add(o), f);
        }
        mem.write(w.wrapping_add(0x3e), 3u8);
        let f = mem.rebase(0x008a_ab00);
        call(mem, f, &[w]);
        w
    }

    /// `MenuShop_SellPrice(id)` (`0x008db400`, own name): the item's record
    /// (`TkMn2GetExcelData(id & 0xfff, [0x0186ac28])`) first word through
    /// `AbmapQuarterOrOne(g_TkMenuWShopList, price)` (a quarter, or 1).
    pub fn menu_shop_sell_price<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table: u32 = mem.read(mem.rebase(0x0186_ac28));
        let excel = mem.rebase(0x008c_1af0);
        let rec = call(mem, excel, &[id & 0xfff, table]);
        let price: u32 = mem.read(rec);
        let list: u32 = mem.read(mem.rebase(0x0186_ac20));
        let quarter = mem.rebase(0x00a5_cee0);
        call(mem, quarter, &[list, price])
    }

    /// `TkMenuStopScene(scene, group)` (`0x00a7c8a0`): `MenuScene_ResetGroup
    /// (scene, group, 0)`; the entry it returns (if any) has the top two
    /// bits of its `+4` word cleared. Returns the entry.
    pub fn tk_menu_stop_scene<M: Mem>(
        mem: &mut M,
        scene: u32,
        group: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let reset = mem.rebase(0x00a7_ba70);
        let e = call(mem, reset, &[scene, group, 0]);
        if e != 0 {
            let v = mem.read::<u32>(e.wrapping_add(4)) & 0x3fff_ffff;
            mem.write(e.wrapping_add(4), v);
        }
        e
    }

    /// `MenuKaizou_WeaponByte4(w)` (`0x008d56f0`, own name): the weapon at
    /// the window's cursor (`i16 +0x48`) in the list at `[0x0186aa38]`
    /// (`MsGetSaveWeapon`, its out pointer `local`): -1 when its byte `+2`
    /// is 0, else its byte `+4`.
    pub fn menu_kaizou_weapon_byte4<M: Mem>(
        mem: &mut M,
        w: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k = i32::from(mem.read::<i16>(w.wrapping_add(0x48))).cast_unsigned();
        let list: u32 = mem.read(mem.rebase(0x0186_aa38));
        let id: u16 = mem.read(list.wrapping_add(k.wrapping_mul(2)));
        let get = mem.rebase(0x007a_bbd0);
        let weapon = call(mem, get, &[u32::from(id), local]);
        if mem.read::<u8>(weapon.wrapping_add(2)) == 0 {
            u32::MAX
        } else {
            u32::from(mem.read::<u8>(weapon.wrapping_add(4)))
        }
    }

    /// `MenuMain_BusyMask()` (`0x008aa610`, own name): the active main menus
    /// ([`MENU_MAIN_ACTIVE_MASK`]) with bit 31 for a CD read in progress
    /// (`TkCdReadSync(1)`); with none of those, bit 31 alone when the
    /// script's callback queue is busy (`AtelIsCallbackQue()` = 1).
    pub fn menu_main_busy_mask<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut mask = MENU_MAIN_ACTIVE_MASK.get(mem);
        let sync = mem.rebase(0x0088_d1c0);
        if call(mem, sync, &[1]) != 0 {
            mask |= 0x8000_0000;
        }
        if mask == 0 {
            let que = mem.rebase(0x0087_5180);
            if call(mem, que, &[]) == 1 {
                return 0x8000_0000;
            }
        }
        mask
    }

    /// `MenuWShop_OpenSubWindow4()` (`0x008d83e0`, own name): a window from
    /// `TkMenuMainAllocWindow()` with callbacks `+0xc` `0x008d7a30`, `+0x10`
    /// `0x008d7f50`, `+0x14` `0x008d8510`, layer 1, kept in `0x0186ab00`
    /// (what `MenuWShop_CloseSubWindow4` closes) and registered; returned.
    pub fn menu_wshop_open_sub_window4<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        for (o, f) in [
            (0xcu32, 0x008d_7a30u32),
            (0x10, 0x008d_7f50),
            (0x14, 0x008d_8510),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 1u8);
        let keep = mem.rebase(0x0186_ab00);
        mem.write(keep, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// The start of the four shop amount windows: a window from
    /// `TkMenuMainAllocWindow()`, then `TkMn2GetExcelData([id], [table])`
    /// (the shop's record; returned with the window).
    fn shop_amount_window_alloc<M: Mem>(
        mem: &mut M,
        id: u32,
        table: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> (u32, u32) {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        let id: u32 = mem.read(mem.rebase(id));
        let table: u32 = mem.read(mem.rebase(table));
        let excel = mem.rebase(0x008c_1af0);
        let record = call(mem, excel, &[id, table]);
        (w, record)
    }

    /// The common fields of the four shop amount windows: callbacks `+0xc`,
    /// `+0x10`, `+0x14` and layer 4 (`+0x3e`).
    fn shop_amount_window_handlers(mem: &mut impl Mem, w: u32, handlers: [u32; 3]) {
        for (o, f) in [0xcu32, 0x10, 0x14].into_iter().zip(handlers) {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 4u8);
    }

    /// The end of the four shop amount windows: `MsGetGIL()` at `+0x68`,
    /// `+0x48` (`u16`) 1, kept in `keep`, registered
    /// (`TkMenuMainRegistWindow`); the window is returned.
    fn shop_amount_window_finish<M: Mem>(
        mem: &mut M,
        w: u32,
        keep: Global<u32>,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let gil = mem.rebase(0x0078_4e80);
        let g = call(mem, gil, &[]);
        mem.write(w.wrapping_add(0x68), g);
        mem.write(w.wrapping_add(0x48), 1u16);
        keep.set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// The `u16` id of entry `i16 parent+0x48` in a list of `stride`-byte
    /// entries at `list`.
    fn shop_cursor_id(mem: &impl Mem, parent: u32, list: u32, stride: u32) -> u32 {
        let k = i32::from(mem.read::<i16>(parent.wrapping_add(0x48))).cast_unsigned();
        u32::from(mem.read::<u16>(list.wrapping_add(k.wrapping_mul(stride))))
    }

    /// `MenuWShop_OpenSubWindow2(parent)` (`0x008d81d0`, own name): the
    /// weapon shop's buy-amount window ([`WSHOP_WINDOWS`]`[2]`, which
    /// `MenuWShop_CloseSubWindow2` closes). `TkMn2GetExcelData([0x0186ab1c],
    /// [0x0186aba8])` is the shop record; the weapon is its `u16` at
    /// `+2 + 2 * parent+0x48` (signed index): `+0x5c`
    /// `MenuWShop_GetRecord(id)`, `+0x60` `MenuShop_WeaponPrice` of that,
    /// `+0x64` 1; callbacks `0x008d7690`, `0x008d7a40`, `0x008d84d0`, layer
    /// 4, the parent at `+0x58`, the gil at `+0x68`, `+0x48` 1; registered,
    /// returned.
    pub fn menu_wshop_open_sub_window2<M: Mem>(
        mem: &mut M,
        parent: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, record) = shop_amount_window_alloc(mem, 0x0186_ab1c, 0x0186_aba8, call);
        shop_amount_window_handlers(mem, w, [0x008d_7690, 0x008d_7a40, 0x008d_84d0]);
        mem.write(w.wrapping_add(0x58), parent);
        let id = shop_cursor_id(mem, parent, record.wrapping_add(2), 2);
        let get = mem.rebase(0x008d_9170);
        let entry = call(mem, get, &[id]);
        mem.write(w.wrapping_add(0x5c), entry);
        let price = mem.rebase(0x008d_9370);
        let p = call(mem, price, &[entry]);
        mem.write(w.wrapping_add(0x60), p);
        mem.write(w.wrapping_add(0x64), 1u32);
        shop_amount_window_finish(mem, w, WSHOP_WINDOWS[2], call)
    }

    /// `MenuWShop_OpenSubWindow3(parent, local)` (`0x008d82d0`, own name):
    /// the weapon shop's sell window ([`WSHOP_WINDOWS`]`[3]`, which
    /// `MenuWShop_CloseSubWindow3` closes). `TkMn2GetExcelData([0x0186ab1c],
    /// [0x0186aba8])` is called and dropped; the weapon id is the `u16` of
    /// [`MENU_ITEM_LIST`]`[parent+0x48]` (4-byte entries, signed index):
    /// `+0x5c` `MsGetSaveWeapon(id, local)` (`local`: the original's stack
    /// word), `+0x60` `MenuShop_WeaponSellPrice` of that, `+0x64` 1, `+0x6c`
    /// the id; callbacks `0x008d7860`, `0x008d7cc0`, `0x008d84f0`, layer 4,
    /// the parent at `+0x58`, the gil at `+0x68`, `+0x48` 1; registered,
    /// returned.
    pub fn menu_wshop_open_sub_window3<M: Mem>(
        mem: &mut M,
        parent: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, _) = shop_amount_window_alloc(mem, 0x0186_ab1c, 0x0186_aba8, call);
        let id = shop_cursor_id(mem, parent, MENU_ITEM_LIST.ptr(mem).addr, 4);
        shop_amount_window_handlers(mem, w, [0x008d_7860, 0x008d_7cc0, 0x008d_84f0]);
        mem.write(w.wrapping_add(0x58), parent);
        let get = mem.rebase(0x007a_bbd0);
        let weapon = call(mem, get, &[id, local]);
        mem.write(w.wrapping_add(0x5c), weapon);
        let price = mem.rebase(0x008d_9410);
        let p = call(mem, price, &[weapon]);
        mem.write(w.wrapping_add(0x60), p);
        mem.write(w.wrapping_add(0x64), 1u32);
        let gil = mem.rebase(0x0078_4e80);
        let g = call(mem, gil, &[]);
        mem.write(w.wrapping_add(0x68), g);
        mem.write(w.wrapping_add(0x6c), id);
        mem.write(w.wrapping_add(0x48), 1u16);
        WSHOP_WINDOWS[3].set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuShop_OpenBuyWindow(parent)` (`0x008da860`, own name): the item
    /// shop's buy-amount window ([`WSHOP_WINDOW`], which
    /// `TkMenuWShopCloseWindow` closes). `TkMn2GetExcelData(
    /// g_TkMenuWShopList, [0x0186ac2c])` is the shop record; the item id is
    /// its `u16` at `+2 + 2 * parent+0x48` (signed index): `+0x5c` the id,
    /// `+0x60` `MenuShop_Price(id)`, `+0x64` `MsGetSaveItemNum(id)`;
    /// callbacks `0x008d9bd0`, `0x008d9fe0`, `0x008daae0`, layer 4, `+0x36`
    /// 3, the parent at `+0x58`, the gil at `+0x68`, `+0x48` 1; registered,
    /// returned.
    pub fn menu_shop_open_buy_window<M: Mem>(
        mem: &mut M,
        parent: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (w, record) = shop_amount_window_alloc(mem, 0x0186_ac20, 0x0186_ac2c, call);
        shop_amount_window_handlers(mem, w, [0x008d_9bd0, 0x008d_9fe0, 0x008d_aae0]);
        mem.write(w.wrapping_add(0x36), 3u8);
        mem.write(w.wrapping_add(0x58), parent);
        let id = shop_cursor_id(mem, parent, record.wrapping_add(2), 2);
        mem.write(w.wrapping_add(0x5c), id);
        let price = mem.rebase(0x008d_b3b0);
        let p = call(mem, price, &[id]);
        mem.write(w.wrapping_add(0x60), p);
        let id: u32 = mem.read(w.wrapping_add(0x5c));
        let num = mem.rebase(0x0079_04b0);
        let n = call(mem, num, &[id]);
        mem.write(w.wrapping_add(0x64), n);
        shop_amount_window_finish(mem, w, WSHOP_WINDOW, call)
    }

    /// `MenuShop_OpenSellWindow(parent)` (`0x008daa00`, own name): the item
    /// shop's sell window ([`WSHOP_WINDOW2`], which `TkMenuWShopCloseWindow2`
    /// closes). `TkMn2GetExcelData(g_TkMenuWShopList, [0x0186ac2c])` is
    /// called and dropped; the item id is the `u16` of
    /// [`MENU_ITEM_LIST`]`[parent+0x48]` (the index read first, the entry
    /// read again before each use): `+0x5c` the id, `+0x60`
    /// `MenuShop_SellPrice(id)`, `+0x64` `MsGetSaveItemNum(id)`; callbacks
    /// `0x008d9e20`, `0x008da730`, `0x008dab20`, layer 4, `+0x36` 3, the
    /// parent at `+0x58`, the gil at `+0x68`, `+0x48` 1; registered,
    /// returned.
    pub fn menu_shop_open_sell_window<M: Mem>(
        mem: &mut M,
        parent: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let k = i32::from(mem.read::<i16>(parent.wrapping_add(0x48))).cast_unsigned();
        let (w, _) = shop_amount_window_alloc(mem, 0x0186_ac20, 0x0186_ac2c, call);
        shop_amount_window_handlers(mem, w, [0x008d_9e20, 0x008d_a730, 0x008d_ab20]);
        mem.write(w.wrapping_add(0x36), 3u8);
        mem.write(w.wrapping_add(0x58), parent);
        let entry = MENU_ITEM_LIST.ptr(mem).addr.wrapping_add(k.wrapping_mul(4));
        let id = |mem: &M| u32::from(mem.read::<u16>(entry));
        let item = id(mem);
        mem.write(w.wrapping_add(0x5c), item);
        let sell = mem.rebase(0x008d_b400);
        let item = id(mem);
        let price = call(mem, sell, &[item]);
        mem.write(w.wrapping_add(0x60), price);
        let num = mem.rebase(0x0079_04b0);
        let item = id(mem);
        let count = call(mem, num, &[item]);
        mem.write(w.wrapping_add(0x64), count);
        shop_amount_window_finish(mem, w, WSHOP_WINDOW2, call)
    }

    /// `MenuSummon_OpenWindow2()` (`0x008ceda0`, own name): the aeon menu's
    /// second window ([`SUMMON_WINDOW2`], closed by
    /// `TkMenuSummonCloseWindow2`). `TkMenuGetCurrentSummon()` first, then a
    /// window from `TkMenuMainAllocWindow()`: `+0x48` (`u16`) and `+0x32`
    /// 0, `+0x30` 10, `+0x3a` 9 (`u16`), callbacks `+8` `0x008cd310`,
    /// `+0xc` `0x008cd6d0`, `+0x10` `0x008ce750`, `+0x14` `0x008ceee0`,
    /// `+0x1c` `0x008cd420`, layer 2, `+0x42` 1, `+0x36` 3 (`u16`), `+0x45`
    /// 0, the fade step `0xaa` (`+0x54`); registered, kept,
    /// `TkMn2InitStatusBG()` and `MenuSummon_RefreshGrowMasks(aeon)`.
    /// Returns the window.
    pub fn menu_summon_open_window2<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let current = mem.rebase(0x008a_9880);
        let summon = call(mem, current, &[]);
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x48), 0u16);
        mem.write(at(0x32), 0u32);
        mem.write(at(0x30), 10u16);
        mem.write(at(0x3a), 9u16);
        for (o, f) in [
            (8u32, 0x008c_d310u32),
            (0xc, 0x008c_d6d0),
            (0x10, 0x008c_e750),
            (0x14, 0x008c_eee0),
        ] {
            let handler = mem.rebase(f);
            mem.write(at(o), handler);
        }
        mem.write(at(0x3e), 2u8);
        let handler = mem.rebase(0x008c_d420);
        mem.write(at(0x1c), handler);
        mem.write(at(0x42), 1u8);
        mem.write(at(0x36), 3u16);
        mem.write(at(0x45), 0u8);
        mem.write(at(0x54), 0xaau16);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        SUMMON_WINDOW2.set(mem, w);
        let bg = mem.rebase(0x008d_4ed0);
        call(mem, bg, &[]);
        let grow = mem.rebase(0x008c_d3b0);
        call(mem, grow, &[summon]);
        w
    }

    /// The item menu's equipment counts: `u16 [owner][kind]` for owners
    /// 0..7 (byte `+4` of a gear slot) and kind 0 (byte `+5` zero, weapons)
    /// or 1 (armour), filled by `MenuItem_CountGearByOwner`.
    pub const MENU_ITEM_GEAR_COUNTS: Global<[u16; 14]> = Global::new(0x0186_a3e4);

    /// `MenuItem_CountGearByOwner(local)` (`0x008c94e0`, own name): clears
    /// [`MENU_ITEM_GEAR_COUNTS`], then for each of the first
    /// `TkMn2GetNameModeMax()` entries (signed) of [`MENU_ITEM_LIST`] (`u16`
    /// ids, 4-byte entries) counts `MsGetSaveWeapon(id, local)` (`local`:
    /// the original's stack word) under its owner (byte `+4`, only below 7)
    /// and kind (byte `+5` nonzero → 1). The original leaves in `eax` the
    /// count when it is not positive, else from the last entry the counter
    /// index `owner * 2 + kind` (counted) or the slot's address (owner >= 7);
    /// that is returned.
    pub fn menu_item_count_gear_by_owner<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let max = mem.rebase(0x008c_1bc0);
        let n = call(mem, max, &[]).cast_signed();
        let counts = MENU_ITEM_GEAR_COUNTS.ptr(mem).addr;
        for i in 0..7u32 {
            mem.write(counts.wrapping_add(i * 4), 0u32);
        }
        let ids = MENU_ITEM_LIST.ptr(mem).addr;
        let get = mem.rebase(0x007a_bbd0);
        let mut last = n.cast_unsigned();
        for i in 0..n.max(0).cast_unsigned() {
            let id = u32::from(mem.read::<u16>(ids.wrapping_add(i.wrapping_mul(4))));
            let slot = call(mem, get, &[id, local]);
            let owner = u32::from(mem.read::<u8>(slot.wrapping_add(4)));
            last = slot;
            if owner < 7 {
                let kind = u32::from(mem.read::<u8>(slot.wrapping_add(5)) != 0);
                let k = kind + owner * 2;
                let at = counts.wrapping_add(k * 2);
                let c: u16 = mem.read(at);
                mem.write(at, c.wrapping_add(1));
                last = k;
            }
        }
        last
    }

    /// `TkMenuOpenSubCmdWindowRect(id, texts, values, n, b44, x, y, width,
    /// h)` (`0x008e4930`, own name; the last four are `f32`): the one-row
    /// sub-command window ([`tk_menu_open_sub_cmd_window_row`] with `id`,
    /// `texts`, `values`, `n`, `ftol2_sse(width)` and `b44`) kept in
    /// [`MENU_SUB_CMD_WINDOW`], then `+0x4a`, `+0x4c`, `+0x4e`, `+0x50`
    /// (`u16`) the four floats through `ftol2_sse` (each stored through the
    /// global, read again). Returns the global.
    pub fn tk_menu_open_sub_cmd_window_rect<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        rect: [f32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [id, texts, values, n, b44] = args;
        let width = ffx_bridge::x87::ftol(mem, f64::from(rect[2]));
        let w = tk_menu_open_sub_cmd_window_row(mem, [id, texts, values, n, width, b44], call);
        MENU_SUB_CMD_WINDOW.set(mem, w);
        for (o, v) in [0x4au32, 0x4c, 0x4e, 0x50].into_iter().zip(rect) {
            let [lo, hi, ..] = ffx_bridge::x87::ftol(mem, f64::from(v)).to_le_bytes();
            let w = MENU_SUB_CMD_WINDOW.get(mem);
            mem.write(w.wrapping_add(o), u16::from_le_bytes([lo, hi]));
        }
        MENU_SUB_CMD_WINDOW.get(mem)
    }

    /// `TkMenuOpenSubCmdWindowBeside(id, texts, values, n, b44)`
    /// (`0x008e46b0`, own name): the second sub-command window
    /// ([`MENU_SUB_CMD_WINDOW2`]) beside the open one
    /// ([`MENU_SUB_CMD_WINDOW`]). Its width: `0x165` with none open, else
    /// the open window's `i16 +0x4e` less its cursor entry's text width
    /// (`+0x60 + 12 * i16 +0x48`) less `0x18`; then
    /// [`tk_menu_open_sub_cmd_window_row`]. The new window (each field
    /// through the global): `+0x46` (`u16`) 0, layer 3, `+0x3f` 0, `+0x14`
    /// `0x008e4a90`, `+0x10` `0x008e4140`; `+0x4a` (`u16`) the open
    /// window's `+0x4e - new +0x4e + +0x4a` (`i16`s; 0x30 with none open),
    /// `+0x94` 1, `+0x4c` 0x25, `+0x90` -1. Returns the global.
    pub fn tk_menu_open_sub_cmd_window_beside<M: Mem>(
        mem: &mut M,
        args: [u32; 5],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [id, texts, values, n, b44] = args;
        let i16_at = |mem: &M, at: u32| i32::from(mem.read::<i16>(at));
        let open = MENU_SUB_CMD_WINDOW.get(mem);
        let width = if open == 0 {
            0x165
        } else {
            let k = i16_at(mem, open.wrapping_add(0x48))
                .wrapping_add(8)
                .wrapping_mul(12);
            let text: i32 = mem.read(open.wrapping_add(k.cast_unsigned()));
            i16_at(mem, open.wrapping_add(0x4e))
                .wrapping_sub(text)
                .wrapping_sub(0x18)
        };
        let w = tk_menu_open_sub_cmd_window_row(
            mem,
            [id, texts, values, n, width.cast_unsigned(), b44],
            call,
        );
        MENU_SUB_CMD_WINDOW2.set(mem, w);
        mem.write(w.wrapping_add(0x46), 0u16);
        let at = |mem: &M, o: u32| MENU_SUB_CMD_WINDOW2.get(mem).wrapping_add(o);
        let a = at(mem, 0x3e);
        mem.write(a, 3u8);
        let a = at(mem, 0x3f);
        mem.write(a, 0u8);
        for (o, f) in [(0x14u32, 0x008e_4a90u32), (0x10, 0x008e_4140)] {
            let handler = mem.rebase(f);
            let a = at(mem, o);
            mem.write(a, handler);
        }
        let open = MENU_SUB_CMD_WINDOW.get(mem);
        let w = MENU_SUB_CMD_WINDOW2.get(mem);
        let x = if open == 0 {
            0x30
        } else {
            i16_at(mem, open.wrapping_add(0x4e))
                .wrapping_sub(i16_at(mem, w.wrapping_add(0x4e)))
                .wrapping_add(i16_at(mem, open.wrapping_add(0x4a)))
        };
        mem.write(w.wrapping_add(0x94), 1u32);
        let [lo, hi, ..] = x.to_le_bytes();
        let a = at(mem, 0x4a);
        mem.write(a, u16::from_le_bytes([lo, hi]));
        let a = at(mem, 0x4c);
        mem.write(a, 0x25u16);
        let a = at(mem, 0x90);
        mem.write(a, u32::MAX);
        MENU_SUB_CMD_WINDOW2.get(mem)
    }

    /// `TkMsSetSaveParam(chr)` (`0x008c3070`, Fahrenheit's Switch name):
    /// `MsSetSaveParam(chr)`, then HP and MP each clamped to their maximum
    /// (signed; `TkMenuGetHP`/`TkMenuGetMaxHP`, `TkMenuSetHP`, the same for
    /// MP; the maximum is fetched again for the set). Returns what the
    /// original leaves in `eax`: `TkMenuSetMP`'s result after a clamp, else
    /// the maximum MP.
    pub fn tk_ms_set_save_param<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_60f0);
        call(mem, f, &[chr]);
        let mut last = 0;
        for (get, max, set) in [
            (0x008a_98c0u32, 0x008a_9990u32, 0x008a_ae90u32),
            (0x008a_9970, 0x008a_99b0, 0x008a_aef0),
        ] {
            let get = mem.rebase(get);
            let max = mem.rebase(max);
            let v = call(mem, get, &[chr]).cast_signed();
            last = call(mem, max, &[chr]);
            if v > last.cast_signed() {
                let m = call(mem, max, &[chr]);
                let set = mem.rebase(set);
                last = call(mem, set, &[chr, m]);
            }
        }
        last
    }

    /// `MenuKaizou_AddAbility(weapon, ability)` (`0x008d5680`, own name):
    /// the first of the weapon's `u8 +0xb` ability slots (`u16` from
    /// `+0xe`) that is 0 or `0xff` gets `ability` (`u16`); none when all are
    /// taken. Then `TkMsSetSaveParamAll()` (`0x008c30d0`, a jump to
    /// `MsSetSaveParamAll`; a tail jump: its result).
    pub fn menu_kaizou_add_ability<M: Mem>(
        mem: &mut M,
        weapon: u32,
        ability: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slots = u32::from(mem.read::<u8>(weapon.wrapping_add(0xb)));
        for k in 0..slots {
            let at = weapon.wrapping_add(0xe + 2 * k);
            let v: u16 = mem.read(at);
            if v == 0 || v == 0xff {
                let [lo, hi, ..] = ability.to_le_bytes();
                mem.write(at, u16::from_le_bytes([lo, hi]));
                break;
            }
        }
        let f = mem.rebase(0x008c_30d0);
        call(mem, f, &[])
    }

    /// `TkMsEventEffectRead(id)` (`0x00872470`, Fahrenheit's Switch name):
    /// `g_AtelOverlayStatus` (`u8 0x01326b80`) = 1, then a tail jump to
    /// `MsEventEffectRead(id)` (its result).
    pub fn tk_ms_event_effect_read<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let status = mem.rebase(0x0132_6b80);
        mem.write(status, 1u8);
        let f = mem.rebase(0x0078_8c60);
        call(mem, f, &[id])
    }

    /// `TkResetClip()` (`0x0088c710`, Fahrenheit's name): with `TkIsDebug()`
    /// nonzero, `TkCreateDECI2Header(0x01330700, 0x10c, 0xeffa, 0x45, 0x48)`
    /// and `u16 0x01330708` = 0. Returns what the original leaves in `eax`:
    /// `TkIsDebug()`'s 0, or the header call's result.
    pub fn tk_reset_clip<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_7d10);
        let debug = call(mem, f, &[]);
        if debug == 0 {
            return 0;
        }
        let header = mem.rebase(0x0133_0700);
        let f = mem.rebase(0x0088_5120);
        let r = call(mem, f, &[header, 0x10c, 0xeffa, 0x45, 0x48]);
        mem.write(mem.rebase(0x0133_0708), 0u16);
        r
    }

    /// `MenuMain_Reset()` (`0x008aa570`, own name): the 28 words from
    /// [`MENU_SLOTS`] (the slots, the active mask, the buffer index)
    /// cleared, `MenuMain_ResetAllWindows()`, `MenuHelp_SetRatio2(1)` (its
    /// result returned).
    pub fn menu_main_reset<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slots = MENU_SLOTS.ptr(mem).addr;
        for k in 0..28u32 {
            mem.write(slots.wrapping_add(4 * k), 0u32);
        }
        let f = mem.rebase(0x008a_a470);
        call(mem, f, &[]);
        let f = mem.rebase(0x008a_b270);
        call(mem, f, &[1])
    }

    /// `MenuItem_DrawRows(window, _, first, x, y)` (`0x008c99f0`, own name;
    /// `x`, `y` `f32`): nine `MenuItem_DrawRow(window, x', y', first + i)`,
    /// `x' = f32(graphicUiRemapX2(210) + x)` once, `y'` from `y` and after
    /// each row `f32(graphicUiRemapY2(77) + y')`. The original's `eax` is
    /// what the last remap leaves (no caller reads it).
    pub fn menu_item_draw_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        first: u32,
        [x, y]: [f32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| v as f32;
        let left = r(remap(mem, false, 210.0) + f64::from(x));
        let mut top = ffx_bridge::x87::round_trip(y);
        let row = mem.rebase(0x008c_9a70);
        for i in 0..9u32 {
            call(
                mem,
                row,
                &[window, left.to_bits(), top.to_bits(), first.wrapping_add(i)],
            );
            top = r(remap(mem, true, 77.0) + f64::from(top));
        }
    }

    /// `MenuNameEntry_Commit()` (`0x008c4e60`, own name): writes the typed
    /// name. The target ([`MENU_ITEM_TARGET`], signed): `>= 0` a
    /// character, `MsGetSaveChrName(t)` with `+0x13` =
    /// `MenuItem_KeysMatch(t)` and `+0x12` = `TOGetFFXLangJmp()`; else the
    /// save's `+0xec` (`TkGetSaveEventAddress`, a jump to
    /// `MsGetSaveEventAddress`). Each key entry of `g_TkMenuItemKeyUse`
    /// (4 bytes, up to a code 0) is copied as its code, then for codes
    /// `6..=0x2f` its byte `+1`, for code 4 its byte `+1` and, below 0x30,
    /// `+2`; for a character target each key first asks `TOGetFFXLangJmp()`
    /// (up to three times: 0, 10, 9) and calls `MsGetSaveConfigEnglish()` on
    /// a match. A 0 ends the name. Then the achievements:
    /// target 0x11 unlocks 0xf and 0x1a, 0xd 0xd, 0xe 0xe
    /// (`achievementUnlockAchievement`). Returns what the original leaves in
    /// `eax`: the last unlock's result, else the key count, else (no keys)
    /// the language or the save address.
    pub fn menu_name_entry_commit<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let target = |mem: &M| MENU_ITEM_TARGET.get(mem).cast_signed();
        let lang_f = mem.rebase(0x008a_c2f0);
        let t = target(mem);
        let (mut out, mut last) = if t >= 0 {
            let f = mem.rebase(0x0078_4fb0);
            let name = call(mem, f, &[t.cast_unsigned()]);
            let f = mem.rebase(0x008c_5390);
            let t = target(mem).cast_unsigned();
            let [m, ..] = call(mem, f, &[t]).to_le_bytes();
            mem.write(name.wrapping_add(0x13), m);
            let lang = call(mem, lang_f, &[]);
            mem.write(name.wrapping_add(0x12), lang.to_le_bytes()[0]);
            (name, lang)
        } else {
            let f = mem.rebase(0x0088_c830);
            let save = call(mem, f, &[]);
            (save.wrapping_add(0xec), save)
        };
        let keys = MENU_ITEM_KEY_USE.ptr(mem).addr;
        let english = mem.rebase(0x0078_51d0);
        let mut k = 0u32;
        let mut code: u8 = mem.read(keys);
        while code != 0 {
            let entry = keys.wrapping_add(4 * k);
            if target(mem) >= 0 {
                let mut matched = false;
                for want in [0u32, 10, 9] {
                    if call(mem, lang_f, &[]) == want {
                        matched = true;
                        break;
                    }
                }
                if matched {
                    call(mem, english, &[]);
                }
            }
            let mut put = |mem: &mut M, v: u8| {
                mem.write(out, v);
                out = out.wrapping_add(1);
            };
            put(mem, code);
            let a: u8 = mem.read(entry.wrapping_add(1));
            if code.wrapping_sub(6) <= 0x29 {
                put(mem, a);
            } else if code == 4 {
                put(mem, a);
                if a < 0x30 {
                    let b: u8 = mem.read(entry.wrapping_add(2));
                    put(mem, b);
                }
            }
            k += 1;
            last = k;
            code = mem.read(keys.wrapping_add(4 * k));
        }
        mem.write(out, 0u8);
        let unlock = mem.rebase(0x0082_2380);
        let ids: &[u32] = match target(mem) {
            0x11 => &[0xf, 0x1a],
            0xd => &[0xd],
            0xe => &[0xe],
            _ => &[],
        };
        for &id in ids {
            last = call(mem, unlock, &[id]);
        }
        last
    }

    /// `TkHideAttachedChrs(actor, hide)` (`0x00872380`, own name): for the
    /// actor's two attached characters (`+0xaf4`, ids `u16 +0xb04`), each
    /// whose bit (1, 2) of `u16 +0xb14` is set, id is not `0xff` and pointer
    /// not 0: `Ch_SetHideEvent(chr, hide)` and `Ch_SetShadowMode(chr, 0)`.
    /// The original leaves `actor` in `eax`.
    pub fn tk_hide_attached_chrs<M: Mem>(
        mem: &mut M,
        actor: u32,
        hide: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for k in 0..2u32 {
            let mask: u16 = mem.read(actor.wrapping_add(0xb14));
            if u32::from(mask) & (1 << k) == 0 {
                continue;
            }
            if mem.read::<u16>(actor.wrapping_add(0xb04 + 2 * k)) == 0xff {
                continue;
            }
            let at = actor.wrapping_add(0xaf4 + 4 * k);
            let chr: u32 = mem.read(at);
            if chr == 0 {
                continue;
            }
            let f = mem.rebase(0x0082_b2c0);
            call(mem, f, &[chr, hide]);
            let chr: u32 = mem.read(at);
            let f = mem.rebase(0x0082_b820);
            call(mem, f, &[chr, 0]);
        }
        actor
    }

    /// `MenuStatus_OpenWindow(a, index, b, c)` (`0x008d3140`, own name): a
    /// window from `TkMenuMainAllocWindow()` kept in [`STATUS_WINDOWS`]
    /// `[index]`, with the status window's callbacks (`+8` `0x008d2670`,
    /// `+0xc` `TkMn2WindowStepFade`, `+0x10` `0x008d2790`, `+0x14`
    /// `TkMenuWindowFadedOut`), layer `index + 3` (`+0x3e`), `c` at `+0x5c`,
    /// `a` at `+0x64`, `b` at `+0x58`, `index` at `+0x60`, the kind at
    /// `+0x68` (index 0..=4 → 1, 2, 3, 5, 4; else 0) and the fade step
    /// `0xaa` (`+0x54`); registered (`TkMenuMainRegistWindow`) and returned.
    pub fn menu_status_open_window<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let index = args[1];
        let kind: u32 = match index {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 5,
            4 => 4,
            _ => 0,
        };
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        let slot = STATUS_WINDOWS
            .ptr(mem)
            .addr
            .wrapping_add(index.wrapping_mul(4));
        mem.write(slot, w);
        for (o, f) in [
            (8u32, 0x008d_2670u32),
            (0xc, 0x008d_26d0),
            (0x10, 0x008d_2790),
            (0x14, 0x008d_3200),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), index.to_le_bytes()[0].wrapping_add(3));
        mem.write(w.wrapping_add(0x5c), args[3]);
        mem.write(w.wrapping_add(0x64), args[0]);
        mem.write(w.wrapping_add(0x58), args[2]);
        mem.write(w.wrapping_add(0x60), index);
        mem.write(w.wrapping_add(0x68), kind);
        mem.write(w.wrapping_add(0x54), 0xaau16);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuNameEntry_DeleteKey()` (`0x008c44d0`, own name; called twice by
    /// the name entry's control step `0x008c3580`): with language 10, 9 or
    /// 11 (`TOGetFFXLang` through `0x008ac2f0`, asked again for each test)
    /// and keys that spell the target's string ([`menu_item_keys_match`]
    /// of [`MENU_ITEM_TARGET`]), every leading non-empty entry of
    /// [`ITEM_KEYS`] gets its first byte cleared; otherwise the last of them
    /// does. The run of entries is not bounded, as the original.
    pub fn menu_name_entry_delete_key<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let lang = mem.rebase(0x008a_c2f0);
        let mut cjk = false;
        for l in [10, 9, 11] {
            if call(mem, lang, &[]) == l {
                cjk = true;
                break;
            }
        }
        let keys = mem.rebase(ITEM_KEYS);
        let entry = |k: u32| keys.wrapping_add(4 * k);
        let all = cjk && {
            let t = MENU_ITEM_TARGET.get(mem).cast_signed();
            menu_item_keys_match(mem, t, call) != 0
        };
        let mut n = 0u32;
        while mem.read::<u8>(entry(n)) != 0 {
            if all {
                mem.write(entry(n), 0u8);
            }
            n += 1;
        }
        if !all && n != 0 {
            mem.write(entry(n - 1), 0u8);
        }
    }

    /// `MenuItem_KeysMatch(t)` (`0x008c5390`, own name): whether the item
    /// keys [`menu_parse_item_keys`] left in [`ITEM_KEYS`] spell target `t`'s
    /// key string (`TOMenuGetNameMes` of the `u16` at `0x00c5a0d8 + 4t`):
    /// token by token, each 4-byte entry its bytes then a 0 (a value in
    /// 6..=0x2f with its argument byte; kind 4 with its byte, and for a byte
    /// up to 0x2f one more), until the string's end meets an entry starting
    /// with 0. 0 for a negative `t` or an empty string.
    pub fn menu_item_keys_match<M: Mem>(
        mem: &mut M,
        t: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if t < 0 {
            return 0;
        }
        let id: u16 = mem.read(
            mem.rebase(0x00c5_a0d8)
                .wrapping_add(t.cast_unsigned().wrapping_mul(4)),
        );
        let name_mes = mem.rebase(0x008b_edd0);
        let mut at = call(mem, name_mes, &[u32::from(id)]);
        if mem.read::<u8>(at) == 0 {
            return 0;
        }
        let mut key = mem.rebase(ITEM_KEYS);
        let mut next = |mem: &M| {
            let v = mem.read::<u8>(at);
            at = at.wrapping_add(1);
            v
        };
        loop {
            let b = next(mem);
            let entry: [u8; 4] = mem.read(key);
            // the entry's expected bytes, ending with its 0
            let (want, n) = if b == 0 {
                return u32::from(entry[0] == 0);
            } else if b.wrapping_sub(6) <= 0x29 {
                ([b, next(mem), 0, 0], 3)
            } else if b == 4 {
                let c = next(mem);
                if c <= 0x2f {
                    ([4, c, next(mem), 0], 4)
                } else {
                    ([4, c, 0, 0], 3)
                }
            } else {
                ([b, 0, 0, 0], 2)
            };
            if entry[..n] != want[..n] {
                return 0;
            }
            key = key.wrapping_add(4);
        }
    }

    /// `MenuItem_OpenListWindow(kind)` (`0x008c9140`, own name): a window from
    /// `TkMenuMainAllocWindow()` with the item list's callbacks (`+0xc`
    /// `0x008c6dd0`, `+0x10` `0x008c7750`, `+0x14` `0x008c93e0`), `kind` at
    /// `+0x43`, layer 1, `+0x42` = 1 and the cursor words cleared; then by
    /// kind: 0 (items) and 1 (key items) half the count (rounded up,
    /// `TOAmapGetItemListMax()` or `TkMenuGetImportantCounter()`) as rows
    /// (`+0x30`) and the count at `+0x94`, columns 3 or 0 (`+0x36`), 10 rows
    /// shown at `0x32001d`; 2 (the usable list) `MenuItem_UsableByte` as its
    /// check, 3 columns, 9 rows at `0x50001d`. Kinds 0 and 2 clear
    /// `0x0186a3ac`. Registered (`TkMenuMainRegistWindow`) and returned.
    pub fn menu_item_open_list_window<M: Mem>(
        mem: &mut M,
        kind: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x48), 0u16);
        mem.write(at(0x32), 0u32);
        mem.write(at(0x43), kind.to_le_bytes()[0]);
        for (o, f) in [
            (0xcu32, 0x008c_6dd0u32),
            (0x10, 0x008c_7750),
            (0x14, 0x008c_93e0),
        ] {
            let handler = mem.rebase(f);
            mem.write(at(o), handler);
        }
        mem.write(at(0x3e), 1u16);
        mem.write(at(0x42), 1u8);
        mem.write(at(0x58), 0u32);
        mem.write(at(0x5c), 0u32);
        let page = mem.rebase(0x0186_a3ac);
        let list = match kind {
            0 => {
                mem.write(page, 0u16);
                Some((0x008c_8c30, 3u8))
            }
            1 => {
                mem.write(at(0x42), 1u8);
                Some((0x008c_8a10, 0))
            }
            2 => {
                let usable = mem.rebase(0x008c_6a30);
                mem.write(at(0x1c), usable);
                mem.write(at(0x36), 3u8);
                mem.write(at(0x3a), 9u16);
                mem.write(at(0x4a), 0x0050_001du32);
                mem.write(at(0x42), 1u8);
                mem.write(page, 0u16);
                None
            }
            _ => None,
        };
        if let Some((count, columns)) = list {
            let count = mem.rebase(count);
            let n = call(mem, count, &[]).cast_signed();
            // the rows rounded up (toward zero for a negative count), as a word
            let rows = (n.wrapping_add(1) / 2).to_le_bytes();
            mem.write(at(0x30), u16::from_le_bytes([rows[0], rows[1]]));
            let n = call(mem, count, &[]);
            mem.write(at(0x94), n);
            mem.write(at(0x36), columns);
            mem.write(at(0x4a), 0x0032_001du32);
            mem.write(at(0x3a), 10u16);
        }
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuWShop_CanAfford(e)` (`0x008d85d0`, own name): 0 while
    /// `0x0186ab14` is 0, else whether [`menu_shop_weapon_price`]`(e)` is at
    /// most `MsGetGIL()` (signed).
    pub fn menu_wshop_can_afford<M: Mem>(
        mem: &mut M,
        e: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if mem.read::<u32>(mem.rebase(0x0186_ab14)) == 0 {
            return 0;
        }
        let price = menu_shop_weapon_price(mem, e, call);
        let gil = mem.rebase(0x0078_4e80);
        u32::from(price <= call(mem, gil, &[]).cast_signed())
    }

    /// `MenuShop_CanBuy(id)` (`0x008dab30`, own name): whether the party
    /// holds fewer than 99 (`MsGetSaveItemNum`, signed) and
    /// [`menu_shop_price`]`(id)` is at most `MsGetGIL()`.
    pub fn menu_shop_can_buy<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let price = menu_shop_price(mem, id, call);
        let held = mem.rebase(0x0079_04b0);
        if call(mem, held, &[id]).cast_signed() >= 99 {
            return 0;
        }
        let gil = mem.rebase(0x0078_4e80);
        u32::from(price <= call(mem, gil, &[]).cast_signed())
    }

    /// `MenuNameEntry_OpenCharWindow(chr)` (`0x008c43f0`, own name): a window
    /// from `TkMenuMainAllocWindow()` with the name entry's character
    /// callbacks (`+8` `0x008c3530`, `+0xc` `0x008c3ce0`, `+0x10`
    /// `0x008c3ec0`, `+0x14` `0x008c44b0`), layer 3, `chr` at `+0x58`;
    /// registered (`TkMenuMainRegistWindow`); then `chr` into `0x023cc3c4`,
    /// `0x023cc3c0`, `0x023cc3bc` and `0x023cc3c8` = 0. Returns the window.
    pub fn menu_name_entry_open_char_window<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        for (o, f) in [
            (8u32, 0x008c_3530u32),
            (0xc, 0x008c_3ce0),
            (0x10, 0x008c_3ec0),
            (0x14, 0x008c_44b0),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 3u8);
        mem.write(w.wrapping_add(0x58), chr);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        for a in [0x023c_c3c4, 0x023c_c3c0, 0x023c_c3bc] {
            let a = mem.rebase(a);
            mem.write(a, chr);
        }
        let a = mem.rebase(0x023c_c3c8);
        mem.write(a, 0u32);
        w
    }

    /// `MenuItem_TargetNeedsRecovery(kind, _, chr)` (`0x008c6a70`): kind 0
    /// whether HP is below the maximum, 1 MP, 2 either, 3 always (1); any
    /// other kind 0. The comparisons are signed (`MenuItem_Less`).
    pub fn menu_item_target_needs_recovery(mem: &impl Mem, kind: u32, chr: u32) -> u32 {
        let below = |now: u32, max: u32| {
            u32::from(super::less(
                tk_menu_get_ply(mem, chr, now).cast_signed(),
                tk_menu_get_ply(mem, chr, max).cast_signed(),
            ))
        };
        match kind {
            0 => below(0x1c, 0x24),
            1 => below(0x20, 0x28),
            2 => below(0x1c, 0x24) | below(0x20, 0x28),
            3 => 1,
            _ => 0,
        }
    }

    /// `Msg_PollClose(i)` (`0x00860aa0`, own name): 1 when message window
    /// `i`'s state (`AtelGetMesWinWork(i)` `+0x14`) is 0 or 4; in state 2
    /// with message status 3 (`TOGetMessageStatus(i)`), the window is
    /// closed (`Msg_CloseWindow(i)`) when the current control work's bit 2
    /// asks for it (`AtelGetCurCtrlWork()`, the bit cleared); 0 otherwise.
    pub fn msg_poll_close<M: Mem>(
        mem: &mut M,
        i: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0086_be90);
        let window = call(mem, f, &[i]);
        match mem.read::<u16>(window.wrapping_add(0x14)) {
            0 | 4 => return 1,
            2 => {}
            _ => return 0,
        }
        let f = mem.rebase(0x008b_7ca0);
        if call(mem, f, &[i]) != 3 {
            return 0;
        }
        let f = mem.rebase(0x0086_aff0);
        let ctrl = call(mem, f, &[]);
        if mem.read::<u8>(ctrl) & 4 != 0 {
            let close = mem.rebase(0x0086_40f0);
            call(mem, close, &[i]);
            let v = mem.read::<u8>(ctrl) & !4;
            mem.write(ctrl, v);
        }
        0
    }

    /// `MenuSummon_GrowInfo(stat, chr, cost, gain)` (`0x008cd320`, own
    /// name): for an aeon stat (`stat` < 10, signed; its descriptor at
    /// `0x00c5a608` + 0x1c × `stat`) the sphere cost (`u16 +4` of
    /// `MenuSummon_GrowEntry(stat)`) into `*cost` and, while the stat's
    /// maximum ([`menu_summon_get_stat`] of `chr`'s record) is below the
    /// descriptor's limit (`+0x14`, signed), the gain its function (`+0x18`,
    /// called with `chr` and that maximum) gives, clamped to 1..=99
    /// ([`crate::math::clamp_1_to_99`]), into `*gain`; returns 1 then.
    /// Otherwise `*cost` stays -1 (stat >= 10), `*gain` 0, and 0 returns.
    pub fn menu_summon_grow_info<M: Mem>(
        mem: &mut M,
        stat: i32,
        chr: u32,
        [cost, gain]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(cost, u32::MAX);
        mem.write(gain, 0u32);
        let ply = save::ms_get_save_player_ptr(mem, chr);
        if stat >= 10 {
            return 0;
        }
        let entry = mem.rebase(0x008c_ecf0);
        let entry = call(mem, entry, &[stat.cast_unsigned()]);
        let c = u32::from(mem.read::<u16>(entry.wrapping_add(4)));
        mem.write(cost, c);
        let desc = mem
            .rebase(0x00c5_a608)
            .wrapping_add(stat.cast_unsigned().wrapping_mul(0x1c));
        let max = menu_summon_get_stat(mem, ply, desc, 1);
        if max.cast_signed() >= mem.read::<i32>(desc.wrapping_add(0x14)) {
            mem.write(gain, 0u32);
            return 0;
        }
        let f: u32 = mem.read(desc.wrapping_add(0x18));
        let g = call(mem, f, &[chr, max]).cast_signed();
        mem.write(gain, crate::math::clamp_1_to_99(g));
        1
    }

    /// `MenuNameEntry_LoadLayout()` (`0x008c5050`, own name): the name
    /// entry's layout from the menu sequence: three plates
    /// (`TkMn2GetCommonPlate` into `0x0186a334`, `0x0186a34c`, `0x0186a364`,
    /// the first with 0xda/0xdb), three numbers (`TkMn2SeqGetNum`), a colour
    /// (`TkMn2SeqGetColor`), eight positions (`TkMn2SeqGetPos`) and a
    /// rectangle (`TkMn2SeqGetRect`), each name a string of the image. The
    /// last call's result is left in `eax`.
    pub fn menu_name_entry_load_layout<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let plate = r(mem, 0x008c_1a60);
        for (to, rect, c1, c2, a, b) in [
            (
                0x0186_a334,
                0x00b6_58d4,
                0x00b6_58c8,
                0x00b6_58c8,
                0xda,
                0xdb,
            ),
            (0x0186_a34c, 0x00b6_58f8, 0x00b6_58ec, 0x00b6_58e0, 0, 0),
            (0x0186_a364, 0x00b6_5910, 0x00b6_5904, 0x00b6_5904, 0, 0),
        ] {
            let args = [r(mem, to), r(mem, rect), r(mem, c1), r(mem, c2), a, b];
            call(mem, plate, &args);
        }
        for (f, name, to) in [
            (0x008c_2b10, 0x00b6_591c, 0x0186_a304),
            (0x008c_2b10, 0x00b6_5928, 0x0186_a31c),
            (0x008c_2b10, 0x00b6_5934, 0x0186_a320),
            (0x008c_2ad0, 0x00b6_5940, 0x0186_a314),
        ] {
            let (f, name, to) = (r(mem, f), r(mem, name), r(mem, to));
            let v = call(mem, f, &[name]);
            mem.write(to, v);
        }
        let pos = r(mem, 0x008c_2b50);
        for (name, to) in [
            (0x00b6_594c, 0x0186_a27c),
            (0x00b6_5958, 0x0186_a300),
            (0x00b6_5964, 0x0186_a310),
            (0x00b6_5970, 0x0186_a318),
            (0x00b6_597c, 0x0186_a324),
            (0x00b6_5988, 0x0186_a328),
            (0x00b6_5994, 0x0186_a32c),
            (0x00b6_59a0, 0x0186_a330),
        ] {
            let args = [r(mem, name), r(mem, to)];
            call(mem, pos, &args);
        }
        let rect = r(mem, 0x008c_2b90);
        let args = [r(mem, 0x00b6_59a8), r(mem, 0x0186_a308)];
        call(mem, rect, &args)
    }

    /// `MenuSummon_DrawFaceSlide(x, y, w, h, t, cur, next)` (`0x008d34a0`,
    /// own name): aeon `cur`'s face (`TkMenuGetSummonFromIndex`) drawn at
    /// `(x, y, w, h)` (`TkMn2DrawCrossCharFace`, while
    /// `TOMenuGetFirstReadEndFlag()`); with a slide `t` (nonzero) aeon
    /// `next`'s face too, `x` moved by `0x0186a9ec × t / 4096` (signed,
    /// toward zero; the sum at 53 bits truncated to an integer and back to
    /// a float) and faded (font shade alpha `(0x1000 - |t|) >> 5` between
    /// `TkMn2SaveFontShade` and `TkMn2RestoreFontShade`, whose result is
    /// returned). Without a slide `eax` is the face draw's result, or the
    /// flag's when not drawn.
    #[allow(clippy::many_single_char_names)] // the rectangle and the slide
    pub fn menu_summon_draw_face_slide<M: Mem>(
        mem: &mut M,
        [x, y, w, h]: [u32; 4],
        t: i32,
        [cur, next]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let summon = mem.rebase(0x008a_9bc0);
        let cur = call(mem, summon, &[cur]);
        let next = call(mem, summon, &[next]);
        draw_face_slide(mem, [x, y, w, h], t, [cur, next], call)
    }

    /// `MenuPlayer_DrawFaceSlide(x, y, w, h, t, cur, next)` (`0x008d32b0`,
    /// own name; caller `0x008d4ee0`): [`menu_summon_draw_face_slide`]
    /// for party members, the faces from `TkMenuGetPlayerFromIndex2`.
    #[allow(clippy::many_single_char_names)] // the rectangle and the slide
    pub fn menu_player_draw_face_slide<M: Mem>(
        mem: &mut M,
        xywh: [u32; 4],
        t: i32,
        [cur, next]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let player = mem.rebase(0x008a_9ad0);
        let cur = call(mem, player, &[cur]);
        let next = call(mem, player, &[next]);
        draw_face_slide(mem, xywh, t, [cur, next], call)
    }

    /// `MenuNameEntry_DrawCharFaceSlide(x, y, w, h, t, cur, next)`
    /// (`0x008d33b0`, own name; caller `0x008c3ec0` for a character
    /// target) and its byte-for-byte copy `MenuNameEntry_DrawSummonFaceSlide`
    /// (`0x008d35a0`, an aeon target): [`menu_summon_draw_face_slide`]
    /// with `cur` and `next` already faces (no `TkMenuGetSummonFromIndex`).
    #[allow(clippy::many_single_char_names)] // the rectangle and the slide
    pub fn menu_name_entry_draw_face_slide<M: Mem>(
        mem: &mut M,
        xywh: [u32; 4],
        t: i32,
        faces: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        draw_face_slide(mem, xywh, t, faces, call)
    }

    /// The face slide of [`menu_summon_draw_face_slide`] and
    /// [`menu_name_entry_draw_face_slide`], from the two faces on.
    #[allow(clippy::many_single_char_names)] // the rectangle and the slide
    fn draw_face_slide<M: Mem>(
        mem: &mut M,
        [x, y, w, h]: [u32; 4],
        t: i32,
        [cur, next]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87;
        let read_end = mem.rebase(0x008a_8e50);
        let face = mem.rebase(0x008c_0250);
        let mut drawn = call(mem, read_end, &[]);
        if drawn != 0 {
            drawn = call(mem, face, &[cur, x, y, w, h]);
        }
        if t == 0 {
            return drawn;
        }
        let step: i32 = mem.read(mem.rebase(0x0186_a9ec));
        let shift = step.wrapping_mul(t) / 0x1000;
        let moved = x87::add(x87::load(x), x87::load(x87::store(f64::from(shift))));
        let moved = x87::ftol(mem, moved).cast_signed();
        let save = mem.rebase(0x008c_2330);
        call(mem, save, &[]);
        let alpha = (0x1000 - t.wrapping_abs()) >> 5;
        let shade = mem.rebase(0x008c_2c30);
        call(mem, shade, &[0x80, 0x80, 0x80, alpha.cast_unsigned()]);
        if call(mem, read_end, &[]) != 0 {
            let moved = x87::store(f64::from(moved));
            call(mem, face, &[next, moved, y, w, h]);
        }
        let restore = mem.rebase(0x008c_2300);
        call(mem, restore, &[])
    }

    /// `MenuSummon_BuildCommandList(chr)` (`0x008d2b40`, own name): the aeon
    /// status window's commands. From the command lists of
    /// `MsMenuStatusNum(0, local)` (`TkMenuGetCommandMax` and
    /// `TkMenuGetCommandOffset` of `(list, 1, chr, group)`), the ids
    /// (`u16 +2` of each 4-byte entry) `chr` has (`MsGetSaveCommand`) are
    /// appended to the `u16` array at `0x0186aa04`: groups 1, 3, 4 and 5
    /// counted in `0x0186aa24`, then group 2 counted in `0x0186aa28` after
    /// those (each counter a `u16`, written after its entry). Group 4 stops
    /// at the first command `chr` has and appends the group's *first* id,
    /// whichever matched. `TOGetSaveGilAddress()` is asked (unused).
    /// Returns what the original leaves in `eax`: group 2's offset when it
    /// is empty, else its last id when stored, or 0.
    pub fn menu_summon_build_command_list<M: Mem>(
        mem: &mut M,
        chr: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let status_num = r(mem, 0x0079_0200);
        let list = call(mem, status_num, &[0, local]);
        let gil = r(mem, 0x008a_8e20);
        call(mem, gil, &[]);
        let (max, offset, has) = (
            r(mem, 0x008d_2d80),
            r(mem, 0x008d_2dc0),
            r(mem, 0x0078_5020),
        );
        let ids = r(mem, 0x0186_aa04);
        let (first, second) = (r(mem, 0x0186_aa24), r(mem, 0x0186_aa28));
        let mut eax = 0;
        for group in [1u32, 3, 4, 5, 2] {
            let n = call(mem, max, &[list, 1, chr, group]).cast_signed();
            let off = call(mem, offset, &[list, 1, chr, group]);
            eax = off;
            let count = if group == 2 { second } else { first };
            if group == 1 || group == 2 {
                mem.write(count, 0u16);
            }
            let entries = list.wrapping_add(off.wrapping_mul(4)).wrapping_add(2);
            for k in 0..n.max(0).cast_unsigned() {
                let id: u16 = mem.read(entries.wrapping_add(4 * k));
                eax = call(mem, has, &[chr, u32::from(id)]);
                if eax == 0 {
                    continue;
                }
                let id = if group == 4 { mem.read(entries) } else { id };
                let base = if group == 2 {
                    mem.read::<u16>(first)
                } else {
                    0
                };
                let k: u16 = mem.read(count);
                let slot = u32::from(k) + u32::from(base);
                mem.write(ids.wrapping_add(2 * slot), id);
                mem.write(count, k.wrapping_add(1));
                eax = u32::from(id);
                if group == 4 {
                    break;
                }
            }
        }
        eax
    }

    /// The base value of an equipment entry `e` (shared by
    /// `0x008d9370` and `0x008d9410`): the first words of its abilities'
    /// records (`u16` ids at `+0xe`, `byte +0xb` of them; 0 and 0xff empty)
    /// summed through `TkMn2GetExcelData(id & 0xfff, [0x0186aba4])`, then
    /// `AbmapScaledLevelValue(sum, slots, empty slots)`.
    fn weapon_value<M: Mem>(
        mem: &mut M,
        e: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let n = u32::from(mem.read::<u8>(e.wrapping_add(0xb)));
        let mut empty = n;
        let mut sum = 0u32;
        for k in 0..n {
            let id = u32::from(mem.read::<u16>(e.wrapping_add(0xe + 2 * k)));
            if id == 0xff || id == 0 {
                continue;
            }
            let table = mem.read::<u32>(mem.rebase(0x0186_aba4));
            let f = mem.rebase(0x008c_1af0);
            let rec = call(mem, f, &[id & 0xfff, table]);
            sum = sum.wrapping_add(mem.read::<u32>(rec));
            empty -= 1;
        }
        let f = mem.rebase(0x00a5_cbe0);
        call(mem, f, &[sum, n, empty])
    }

    /// `MenuShop_WeaponPrice(e)` (`0x008d9370`, own name): the entry's
    /// value (`weapon_value`) times the shop ratio (`0x01326b6c`) / 100,
    /// truncated (32-bit product); at least 1.
    pub fn menu_shop_weapon_price<M: Mem>(
        mem: &mut M,
        e: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let v = weapon_value(mem, e, call).cast_signed();
        let f = mem.rebase(0x0086_c480);
        let ratio = call(mem, f, &[]).cast_signed();
        let p = ratio.wrapping_mul(v) / 100;
        if p == 0 { 1 } else { p }
    }

    /// `MenuShop_WeaponSellPrice(e)` (`0x008d9410`, own name):
    /// `AbmapQuarterOrOne([0x0186ab1c], value)` (`weapon_value`), 0 for an
    /// entry with flag bits `+3 & 0xd`.
    pub fn menu_shop_weapon_sell_price<M: Mem>(
        mem: &mut M,
        entry: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let v = weapon_value(mem, entry, call);
        let x = mem.read::<u32>(mem.rebase(0x0186_ab1c));
        let f = mem.rebase(0x00a5_cf10);
        let r = call(mem, f, &[x, v]);
        if mem.read::<u8>(entry.wrapping_add(3)) & 0xd != 0 {
            0
        } else {
            r
        }
    }

    /// `MenuEquip_IsSelectable(id, any)` (`0x008d5750`, own name): the
    /// equipment slot from `MsGetSaveWeapon(id, &local)` (`local`, its
    /// name id, unused: `scratch` stands for it) is 0 when empty (`+2`) or
    /// flagged 2 (`+3`); else 1 with `any`, else 0 for flags 4 or 8, else
    /// `MenuList_IsEmptySlot(slot)`.
    pub fn menu_equip_is_selectable<M: Mem>(
        mem: &mut M,
        id: u32,
        any: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x007a_bbd0);
        let slot = call(mem, f, &[id, scratch]);
        if mem.read::<u8>(slot.wrapping_add(2)) == 0 {
            return 0;
        }
        let flags: u8 = mem.read(slot.wrapping_add(3));
        if flags & 2 != 0 {
            return 0;
        }
        if any != 0 {
            return 1;
        }
        if flags & 0xc != 0 {
            return 0;
        }
        let f = mem.rebase(0x008b_f720);
        call(mem, f, &[slot])
    }

    /// `TkMenuItemHealAmount(item, player, mp)` (`0x008c69a0`): what the
    /// item record (`TkMsGetRomItem(item, &local)`, `scratch` standing for
    /// the unused `local`) restores, by kind (`+0x28`) and power (`+0x2a`):
    /// 6 power * 50; 7 (the player's byte `+0x31` + power) / 2 * power; 8
    /// the max HP (or MP with `mp`) * power / 16, truncated; others 0.
    pub fn tk_menu_item_heal_amount<M: Mem>(
        mem: &mut M,
        [item, player, mp]: [u32; 3],
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let f = mem.rebase(0x008a_b280);
        let rec = call(mem, f, &[item, scratch]);
        let power = i32::from(mem.read::<u8>(rec.wrapping_add(0x2a)));
        match mem.read::<u8>(rec.wrapping_add(0x28)) {
            6 => power * 50,
            7 => {
                let f = mem.rebase(0x0078_5330);
                let p = call(mem, f, &[player]);
                let b = i32::from(mem.read::<u8>(p.wrapping_add(0x31)));
                i32::midpoint(b, power) * power // both 0..=255
            }
            8 => {
                let f = mem.rebase(if mp == 0 { 0x008a_9990 } else { 0x008a_99b0 });
                let max = call(mem, f, &[player]).cast_signed();
                max.wrapping_mul(power) / 16
            }
            _ => 0,
        }
    }

    /// The layout `DrawCrossMenuIcon*` share (`0x008e6700`, `0x008e67f0`,
    /// `0x008e6970`): `DrawCrossMenuIconXYWHRGBA(remapX2(180) + x,
    /// remapY2(7) + y, remapX2(38), remapY2(46), icon, 0x80 ×4)`, each
    /// remap a double (`ST0`) and every argument rounded to a float. The
    /// remaps run in the original's order (Y 46, X 38, Y 7, X 180).
    #[allow(clippy::many_single_char_names)]
    fn cross_icon<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        icon: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| (v as f32).to_bits();
        let h = r(remap(mem, true, 46.0));
        let w = r(remap(mem, false, 38.0));
        let top = r(remap(mem, true, 7.0) + f64::from(y));
        let left = r(remap(mem, false, 180.0) + f64::from(x));
        let f = mem.rebase(0x008e_6af0);
        call(mem, f, &[left, top, w, h, icon, 0x80, 0x80, 0x80, 0x80])
    }

    /// The name after the icon: `ToMakeBtlEasyFont(text, remapX2(240) + x,
    /// remapY2(8) + y, colour, 0.78, 1.0)`; its result.
    fn cross_text<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        text: u32,
        colour: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| (v as f32).to_bits();
        let top = r(remap(mem, true, 8.0) + f64::from(y));
        let left = r(remap(mem, false, 240.0) + f64::from(x));
        let scale = f32::from_bits(0x3f47_ae14).to_bits();
        let f = mem.rebase(0x0090_5ab0);
        call(mem, f, &[text, left, top, colour, scale, 1f32.to_bits()])
    }

    /// `DrawCrossMenuSummonPlateNoHPMPClut(chr, x, y, colour)` (`0x008e7110`;
    /// the aeon list's plate without HP/MP, caller `0x008ceb90`): the name
    /// `TOGetSaveChrName(chr)` drawn with `ToMakeBtlEasyEdgeFont(name, x',
    /// y', colour, 0.82, 1.0)`, `x'` = `remapX2(160) + x` and `y'` =
    /// `remapY2(10) + y`, the remaps in the
    /// original's order (Y, then X), each sum rounded to a float. Returns
    /// the font call's `eax` (the original's, unread by its caller).
    pub fn draw_cross_menu_summon_plate_no_hpmp<M: Mem>(
        mem: &mut M,
        chr: u32,
        [x, y]: [f32; 2],
        colour: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| (v as f32).to_bits();
        let top = r(remap(mem, true, 10.0) + f64::from(y));
        let left = r(remap(mem, false, 160.0) + f64::from(x));
        let f = mem.rebase(0x008a_c850);
        let name = call(mem, f, &[chr]);
        let scale = f32::from_bits(0x3f51_eb85).to_bits();
        let f = mem.rebase(0x0090_5930);
        call(mem, f, &[name, left, top, colour, scale, 1f32.to_bits()])
    }

    /// `DrawCrossMenuIconItemName(item, x, y, colour)` (`0x008e6700`): the
    /// item's icon (`MsGetRomItem(item, &text)` record byte `+0x14`) and
    /// its name (`+0` plus the `text` base the call writes; `scratch`
    /// stands for that local) at `(x, y)`. Returns `ToMakeBtlEasyFont`'s
    /// result.
    pub fn draw_cross_menu_icon_item_name<M: Mem>(
        mem: &mut M,
        item: u32,
        [x, y]: [f32; 2],
        colour: u32,
        scratch: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0079_09f0);
        let rec = call(mem, f, &[item, scratch]);
        let base: u32 = mem.read(scratch);
        let text = u32::from(mem.read::<u16>(rec)).wrapping_add(base);
        let icon = u32::from(mem.read::<u8>(rec.wrapping_add(0x14)));
        cross_icon(mem, [x, y], icon, remap, call);
        cross_text(mem, [x, y], text, colour, remap, call)
    }

    /// The weapon icon of an equipment slot `e`: `2 * +4 + 1 + +5` (as a
    /// byte); then the name, and with `+4` equal to `+6` the marker icon
    /// 0x31 over it.
    fn cross_weapon<M: Mem>(
        mem: &mut M,
        e: u32,
        text: u32,
        [x, y, colour]: [u32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = [f32::from_bits(x), f32::from_bits(y)];
        let kind: u8 = mem.read(e.wrapping_add(4));
        let icon = kind
            .wrapping_mul(2)
            .wrapping_add(1)
            .wrapping_add(mem.read::<u8>(e.wrapping_add(5)));
        cross_icon(mem, at, u32::from(icon), remap, call);
        cross_text(mem, at, text, colour, remap, call);
        if mem.read::<u8>(e.wrapping_add(4)) == mem.read::<u8>(e.wrapping_add(6)) {
            cross_icon(mem, at, 0x31, remap, call);
        }
    }

    /// `DrawCrossMenuIconWeaponName(id, x, y, colour)` (`0x008e67f0`): the
    /// weapon slot `MsGetSaveWeapon(id, &unused)` (`scratch` standing for
    /// the local) drawn with `MsGetSaveWeaponName(id)` (see `cross_weapon`).
    pub fn draw_cross_menu_icon_weapon_name<M: Mem>(
        mem: &mut M,
        id: u32,
        [x, y, colour]: [u32; 3],
        scratch: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(0x007a_bbd0);
        let e = call(mem, f, &[id, scratch]);
        let f = mem.rebase(0x007a_bdf0);
        let text = call(mem, f, &[id]);
        cross_weapon(mem, e, text, [x, y, colour], remap, call);
    }

    /// `DrawCrossMenuIconWeaponName2(e, x, y, colour)` (`0x008e6970`): an
    /// equipment record `e` drawn with `MsWeaponName(+0, +4,
    /// MsGetSaveConfigHiragana(0))` (see `cross_weapon`).
    pub fn draw_cross_menu_icon_weapon_name2<M: Mem>(
        mem: &mut M,
        e: u32,
        [x, y, colour]: [u32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(0x0078_51f0);
        let hira = call(mem, f, &[0]);
        let name = u32::from(mem.read::<u16>(e));
        let kind = u32::from(mem.read::<u8>(e.wrapping_add(4)));
        let f = mem.rebase(0x007a_0c50);
        let text = call(mem, f, &[name, kind, hira]);
        cross_weapon(mem, e, text, [x, y, colour], remap, call);
    }

    /// `TkMn2DrawGillPlate(x, y, w, h, gil)` (`0x008c0a20`): the plate's
    /// two halves (`TOMkpShapeXYWHUV(0xc8, …)`: the left at `x` with width
    /// `w - remapX2(207)`, UVs `(0.0176, 0.885, 0.303, 0.919)`; the right at
    /// `x + w - remapX2(207)` with width `remapX2(207)`, UVs `(0.303, 0.885,
    /// 0.506, 0.919)`), then the amount right-aligned:
    /// `ToGetBtlEasyDigitWidth2(gil, &width, 0.68, 1.0)` (`scratch` standing
    /// for `width`) and `ToMakeBtlEasyEdgeDigit2(gil, x + w - remapX2(95,
    /// or 110 for language 3) - width, remapY2(-2) + y, 0x25, 0.68, 1.0)`,
    /// whose result is returned. Sums are doubles, arguments floats; the
    /// remaps run in the original's order.
    #[allow(clippy::many_single_char_names, clippy::too_many_lines)]
    pub fn tk_mn2_draw_gill_plate<M: Mem>(
        mem: &mut M,
        [x, y, w, h]: [f32; 4],
        gil: u32,
        scratch: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| v as f32;
        // the arguments pass through fld/fstp (signalling NaNs quieted)
        let [x, y, w, h] = [x, y, w, h].map(ffx_bridge::x87::round_trip);
        let k = f32::from_bits;
        let (cap, v0, v1) = (k(0x434f_0000), k(0x3f62_8000), k(0x3f6b_4000));
        let rx = |mem: &mut M, remap: &mut dyn FnMut(&mut M, bool, f32) -> f64, v: f32| {
            r(remap(mem, false, v))
        };
        let shape = mem.rebase(0x0090_3bb0);
        // the left part
        let left_w = r(f64::from(w) - f64::from(rx(mem, remap, cap)));
        let b = f32::to_bits;
        call(
            mem,
            shape,
            &[
                0xc8,
                b(x),
                b(y),
                b(left_w),
                b(h),
                b(k(0x3c90_0000)),
                b(v0),
                b(k(0x3e9b_8000)),
                b(v1),
            ],
        );
        // the right part
        let right = f64::from(x) + f64::from(w);
        let cap_w = rx(mem, remap, cap);
        let right_x = r(right - f64::from(rx(mem, remap, cap)));
        call(
            mem,
            shape,
            &[
                0xc8,
                b(right_x),
                b(y),
                b(cap_w),
                b(h),
                b(k(0x3e9b_8000)),
                b(v0),
                b(k(0x3f01_8000)),
                b(v1),
            ],
        );
        // the amount
        let scale = k(0x3f2e_147b);
        let f = mem.rebase(0x0090_5230);
        call(mem, f, &[gil, scratch, b(scale), b(1.0)]);
        let width: f32 = mem.read(scratch);
        let f = mem.rebase(0x008a_c2f0);
        let lang = call(mem, f, &[]);
        let top = r(remap(mem, true, -2.0) + f64::from(y));
        let margin = k(if lang == 3 { 0x42dc_0000 } else { 0x42be_0000 });
        let digits_x = r(right - f64::from(rx(mem, remap, margin)) - f64::from(width));
        let f = mem.rebase(0x0090_5820);
        call(mem, f, &[gil, b(digits_x), b(top), 0x25, b(scale), b(1.0)])
    }

    /// `SndSepPlaySimple(sfx)` (`0x00886e80`) through `call`.
    fn list_sound<M: Mem>(
        mem: &mut M,
        sfx: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(0x0088_6e80);
        call(mem, f, &[sfx]);
    }

    /// The list window's scroll end (`0x008b46d4`/`0x008b4796`): the cursor
    /// (`+0x48`) moves with the top row (`+0x32`) to the target top
    /// (`+0x34`), the timer (`+0x46`) cleared, state 2.
    fn list_scroll_end(mem: &mut impl Mem, w: u32) {
        let target: u16 = mem.read(w.wrapping_add(0x34));
        mem.write(w.wrapping_add(0x46), 0u16);
        let top: u16 = mem.read(w.wrapping_add(0x32));
        let cursor: u16 = mem.read(w.wrapping_add(0x48));
        mem.write(
            w.wrapping_add(0x48),
            cursor.wrapping_add(target.wrapping_sub(top)),
        );
        mem.write(w.wrapping_add(0x32), target);
        mem.write(w.wrapping_add(0x28), 2u32);
    }

    /// State 2 of `TkMenuListWindowCtrl`: the pad. Returns the next state
    /// to run now, or `None` to return.
    #[allow(clippy::similar_names)] // pad, rep
    fn list_input<M: Mem>(
        mem: &mut M,
        w: u32,
        [pad, rep, trg]: [u16; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let at = |o: u32| w.wrapping_add(o);
        if trg & 0x20 != 0 {
            list_confirm(mem, w, call);
            return None;
        }
        if trg & 0x40 != 0 {
            list_sound(mem, 0x8000_0004, call);
            mem.write(at(0x28), 0x11u32);
            return None;
        }
        let count = i32::from(mem.read::<u16>(at(0x30)));
        let rows = i32::from(mem.read::<u16>(at(0x3a)));
        let cursor = mem.read::<i16>(at(0x48));
        if rep & 0x1000 != 0 && cursor > 0 {
            list_sound(mem, 0x8000_0001, call);
            let c = cursor - 1;
            mem.write(at(0x48), c);
            return (c < mem.read::<i16>(at(0x32))).then_some(7);
        }
        if rep & 0x4000 != 0 && i32::from(cursor) < count - 1 {
            list_sound(mem, 0x8000_0001, call);
            let c = cursor.wrapping_add(1);
            mem.write(at(0x48), c);
            let top = i32::from(mem.read::<i16>(at(0x32)));
            return (i32::from(c) >= top + rows).then_some(0xb);
        }
        let top = i32::from(mem.read::<i16>(at(0x32)));
        if rep & 1 != 0 && pad == 1 && cursor > 0 {
            let t = (top - rows).max(0);
            if t != top {
                mem.write(
                    at(0x34),
                    t.to_le_bytes()[..2]
                        .try_into()
                        .map_or(0, u16::from_le_bytes),
                );
                return Some(8);
            }
        }
        if rep & 2 != 0 && pad == 2 && i32::from(cursor) < count - 1 {
            let last = count - rows;
            let t = if top + rows < last {
                top + rows
            } else {
                last.max(0)
            };
            if t != top {
                mem.write(
                    at(0x34),
                    t.to_le_bytes()[..2]
                        .try_into()
                        .map_or(0, u16::from_le_bytes),
                );
                return Some(0xc);
            }
        }
        None
    }

    /// The list's confirm (`0x008b4835`): the check callback `+0x1c(w,
    /// cursor, picked)`, if any; refused (0) plays 0x80000003 and stays;
    /// otherwise state 0xf, with 0x80000001 unless this was the last pick
    /// (`+0x45 == +0x42 - 1`) of a window without the `+0x37` sound flag.
    fn list_confirm<M: Mem>(
        mem: &mut M,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| w.wrapping_add(o);
        let f: u32 = mem.read(at(0x1c));
        let mut picked = i32::from(mem.read::<i8>(at(0x45)));
        if f != 0 {
            let cursor = i32::from(mem.read::<i16>(at(0x48)));
            if call(mem, f, &[w, cursor.cast_unsigned(), picked.cast_unsigned()]) == 0 {
                list_sound(mem, 0x8000_0003, call);
                return;
            }
            picked = i32::from(mem.read::<i8>(at(0x45)));
        }
        let last = i32::from(mem.read::<u8>(at(0x42))) - 1;
        if picked != last || mem.read::<u8>(at(0x37)) != 0 {
            list_sound(mem, 0x8000_0001, call);
        }
        mem.write(at(0x28), 0xfu32);
    }

    /// `TkMenuListWindowCtrl(w)` (`0x008b44b0`): one frame of a list
    /// window's state machine (`+0x28`), with the pad read once
    /// (`TOMenuGetControlPad`, `…Rep`, `…Trg`). A state may hand on to the
    /// next in the same frame without storing it, as the original's loop
    /// does: 2 the pad (`list_input`); 7/8 start a scroll up a row / a page
    /// (timer `+0x46` 0x1000) and 11/12 down (0xf000); 9/10 and 13/14 run
    /// the timer by 0x199/0x111 to the scroll end, 9 and 13 cut short by
    /// the held direction; 15 stores a pick, 17 takes one back; 0 and 1
    /// return to 2.
    #[allow(clippy::too_many_lines)]
    pub fn tk_menu_list_window_ctrl<M: Mem>(
        mem: &mut M,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| w.wrapping_add(o);
        let read_pad = |mem: &mut M, f: u32, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f = mem.rebase(f);
            call(mem, f, &[]).to_le_bytes()[..2]
                .try_into()
                .map_or(0, u16::from_le_bytes)
        };
        let pad = read_pad(mem, 0x008b_e410, call);
        let rep = read_pad(mem, 0x008b_e470, call);
        let mut trg = read_pad(mem, 0x008b_e4b0, call);
        let mut state: u32 = mem.read(at(0x28));
        loop {
            let timer = |mem: &mut M, d: u16| -> i16 {
                let v = mem.read::<u16>(at(0x46)).wrapping_add(d);
                mem.write(at(0x46), v);
                i16::from_le_bytes(v.to_le_bytes())
            };
            state = match state {
                0 => {
                    mem.write(at(0x28), 2u32);
                    return;
                }
                1 => {
                    mem.write(at(0x45), 0u8);
                    mem.write(at(0x28), 2u32);
                    return;
                }
                2 => match list_input(mem, w, [pad, rep, trg], call) {
                    Some(s) => s,
                    None => return,
                },
                7 | 0xb => {
                    list_sound(mem, 0x8000_0001, call);
                    let top = mem.read::<u16>(at(0x32));
                    let top = if state == 7 {
                        top.wrapping_sub(1)
                    } else {
                        top.wrapping_add(1)
                    };
                    mem.write(at(0x32), top);
                    mem.write(at(0x34), top);
                    let (t, next) = if state == 7 {
                        (0x1000u16, 9)
                    } else {
                        (0xf000, 0xd)
                    };
                    mem.write(at(0x46), t);
                    trg = 0;
                    mem.write(at(0x28), next);
                    next
                }
                8 | 0xc => {
                    list_sound(mem, 0x8000_0001, call);
                    let (t, next) = if state == 8 {
                        (0x1000u16, 0xa)
                    } else {
                        (0xf000, 0xe)
                    };
                    mem.write(at(0x46), t);
                    trg = 0;
                    mem.write(at(0x28), next);
                    next
                }
                9 => {
                    if timer(mem, 0xfe67) > 0 && trg & 0x1000 == 0 {
                        return;
                    }
                    list_scroll_end(mem, w);
                    trg = 0;
                    2
                }
                0xd => {
                    if timer(mem, 0x199) < 0 && trg & 0x4000 == 0 {
                        return;
                    }
                    list_scroll_end(mem, w);
                    trg = 0;
                    2
                }
                0xa => {
                    if timer(mem, 0xfeef) > 0 {
                        return;
                    }
                    list_scroll_end(mem, w);
                    2
                }
                0xe => {
                    if timer(mem, 0x111) < 0 {
                        return;
                    }
                    list_scroll_end(mem, w);
                    2
                }
                0xf => {
                    let n = mem.read::<i8>(at(0x45));
                    let cursor: u16 = mem.read(at(0x48));
                    mem.write(at(0x2c).wrapping_add_signed(2 * i32::from(n)), cursor);
                    let n = n.wrapping_add(1);
                    mem.write(at(0x45), n);
                    if i32::from(n) < i32::from(mem.read::<u8>(at(0x42))) {
                        mem.write(at(0x28), 2u32);
                        return;
                    }
                    mem.write(at(0x28), 0x10u32);
                    return;
                }
                0x11 => {
                    let n = mem.read::<i8>(at(0x45)).wrapping_sub(1);
                    mem.write(at(0x45), n);
                    mem.write(at(0x28), if n >= 0 { 2u32 } else { 0x12 });
                    return;
                }
                _ => return,
            };
        }
    }

    /// `TkSetRgba_*(i, r, g, b, a)` (`0x0088fca0`, `0x00890320`,
    /// `0x008906d0`): entry `i` (clamped to `0..=last`) of the colour table
    /// at `table` = the four components clamped to 0..=255, packed
    /// `a << 24 | b << 16 | g << 8 | r`. Returns the packed colour.
    #[allow(clippy::many_single_char_names)]
    pub fn tk_set_rgba(mem: &mut impl Mem, table: u32, last: i32, i: i32, rgba: [i32; 4]) -> u32 {
        let i = i.clamp(0, last).cast_unsigned();
        let [r, g, b, a] = rgba.map(|c| c.clamp(0, 0xff).cast_unsigned());
        let v = (((a << 8 | b) << 8 | g) << 8) | r;
        mem.write(mem.rebase(table) + 4 * i, v);
        v
    }

    /// The name list `Menu_AddName` keeps (count `i32`, ten 256-byte names).
    pub const NAME_COUNT: u32 = 0x0187_16a0;
    pub const NAMES: u32 = 0x0187_5710;

    /// `Menu_AddName(s)` (`0x008e5200`): `s` (256 bytes) joins the
    /// [`NAMES`] list unless already there (0) or the list is full. Returns
    /// 0 when found, else `s` (what EAX holds).
    pub fn menu_add_name(mem: &mut impl Mem, s: u32) -> u32 {
        let n: i32 = mem.read(mem.rebase(NAME_COUNT));
        let names = mem.rebase(NAMES);
        let text = crate::tk::bridge::c_bytes(mem, s);
        for k in 0..n.max(0).cast_unsigned() {
            if crate::tk::bridge::c_bytes(mem, names + 0x100 * k) == text {
                return 0;
            }
        }
        if n < 10 {
            let at = names + 0x100 * n.cast_unsigned();
            for w in 0..0x40 {
                let v: u32 = mem.read(s.wrapping_add(4 * w));
                mem.write(at + 4 * w, v);
            }
            mem.write(mem.rebase(NAME_COUNT), n + 1);
        }
        s
    }

    /// `TkSyncV()` (`0x00887f00`): the three sound sync-V flags set
    /// (`0x00887ea0`, `0x00887eb0`, `0x00887ec0`), then
    /// `TkSndClearKickFlag`. Returns 0.
    pub fn tk_sync_v<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for f in [0x0088_7ea0, 0x0088_7eb0, 0x0088_7ec0, 0x0088_7ab0] {
            let f = mem.rebase(f);
            call(mem, f, &[]);
        }
        0
    }

    /// `TkMn2DrawInitPacket()` (`0x008c0be0`): `g_TkMn2DrawY` (and its
    /// copy `0x0186a244`) = `TOMenuGetPacketBuffAdrs()`, moved on by half of
    /// `TOMenuGetPacketBuffSize()` (rounded towards zero to 64 bytes) when
    /// the copy already holds that address; the four font shades 0x80.
    /// Returns the half size (left in `eax`).
    pub fn tk_mn2_draw_init_packet<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008b_edf0);
        let mut at = call(mem, f, &[]);
        let f = mem.rebase(0x008b_ee00);
        let size = call(mem, f, &[]).cast_signed();
        let half = (size / 2 / 64 * 64).cast_unsigned();
        let copy = mem.rebase(0x0186_a244);
        if mem.read::<u32>(copy) == at {
            at = at.wrapping_add(half);
        }
        MN2_DRAW_Y.set(mem, at);
        mem.write(copy, at);
        let shade = MN2_FONT_SHADE.ptr(mem).addr;
        for k in 0..4 {
            mem.write(shade.wrapping_add(8 * k), 0x80u32);
        }
        half
    }

    /// `TkMn2DrawKickPacketBody(clear)` (`0x008c0c50`): the menu packet's
    /// high-water marks first (unsigned): `0x0186a248` takes the bytes used
    /// (`g_TkMn2DrawY - 0x0186a244`) and `g_TkMn2Work` the cursor when
    /// larger. Then the full-screen scissor (`TkMn2DrawScissorClear`) when
    /// `clear` is set, a 16-byte step of the cursor (`MenuMn2_StepDrawY`),
    /// `TkVU1SyncPath()`, `dbgPrintf("TK:MN:Packet Over\n")` once the cursor
    /// reaches `0x02000000`, and [`tk_mn2_draw_init_packet`] for the next
    /// frame (a tail jump: its result).
    pub fn tk_mn2_draw_kick_packet_body<M: Mem>(
        mem: &mut M,
        clear: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let y = MN2_DRAW_Y.get(mem);
        let used = y.wrapping_sub(mem.read(mem.rebase(0x0186_a244)));
        let peak = mem.rebase(0x0186_a248);
        if mem.read::<u32>(peak) < used {
            mem.write(peak, used);
        }
        let work = mem.rebase(0x0186_a240);
        if mem.read::<u32>(work) < y {
            mem.write(work, y);
        }
        if clear != 0 {
            let f = mem.rebase(0x008c_1370);
            call(mem, f, &[]);
        }
        let y = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        MN2_DRAW_Y.set(mem, y);
        let f = mem.rebase(0x0088_ec60);
        call(mem, f, &[]);
        if MN2_DRAW_Y.get(mem) >= 0x0200_0000 {
            let (f, text) = (mem.rebase(0x0062_f500), mem.rebase(0x00b6_5768));
            call(mem, f, &[text]);
        }
        tk_mn2_draw_init_packet(mem, call)
    }

    /// `TkMn2DrawKickSyncPacket()` (`0x008c0cc0`) and
    /// `TkMn2DrawKickSyncPacket2()` (`0x008c0cd0`): [`tk_mn2_draw_kick_packet_body`]
    /// (with the scissor cleared for the first), then `TkVU1SyncPath()`,
    /// whose result is returned (a tail jump).
    pub fn tk_mn2_draw_kick_sync_packet<M: Mem>(
        mem: &mut M,
        clear: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        tk_mn2_draw_kick_packet_body(mem, clear, call);
        let f = mem.rebase(0x0088_ec60);
        call(mem, f, &[])
    }

    /// `MenuMn2_EmitPacket(a, b, kind)` (`0x008c11b0`, own name): the menu
    /// draw cursor (`g_TkMn2DrawY`) stepped by 16, then the packet builder
    /// `0x008f1b50(cursor, a, b, kind)` writes there and its end becomes
    /// the cursor (returned).
    pub fn menu_mn2_emit_packet<M: Mem>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let y = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        MN2_DRAW_Y.set(mem, y);
        let f = mem.rebase(0x008f_1b50);
        let end = call(mem, f, &[y, args[0], args[1], args[2]]);
        MN2_DRAW_Y.set(mem, end);
        end
    }

    /// `TkFree(p)` (`0x0088d520`): `user_free_thunk(p)`, then
    /// `Sg_MemCheck` on pools 0 and 1 (with the source file `0x00b5e968`
    /// and lines 480, 481). Returns the last check's result.
    pub fn tk_free<M: Mem>(
        mem: &mut M,
        p: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0068_7010);
        call(mem, f, &[p]);
        let file = mem.rebase(0x00b5_e968);
        let f = mem.rebase(0x0083_b7b0);
        call(mem, f, &[0, file, 0x1e0]);
        call(mem, f, &[1, file, 0x1e1])
    }

    /// `Math_YawPitchTo(yaw, pitch, from, to)` (`0x0088b4d0`):
    /// [`crate::math::yaw_pitch_to`] (`atan2` the C runtime's `_CIatan2`)
    /// stored at `*yaw`, then `*pitch`; `eax` is left holding `yaw`.
    pub fn math_yaw_pitch_to<M: Mem>(
        mem: &mut M,
        yaw: u32,
        pitch: u32,
        from: [f32; 3],
        to: [f32; 3],
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        let (y, p) = crate::math::yaw_pitch_to(from, to, |a, b| atan2(mem, a, b));
        mem.write(yaw, y.to_bits());
        mem.write(pitch, p.to_bits());
        yaw
    }

    /// `TkSndIsRead()` (`0x00886600`): whether any of the sound status
    /// words (`0x0132ff20`, `0x0132ff1c`, `0x0132ff10`, `0x0132ff24`) is
    /// set, read before [`tk_snd_update_main`] runs (and clears most of
    /// them).
    pub fn tk_snd_is_read<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut any = 0u32;
        for a in [0x0132_ff20, 0x0132_ff1c, 0x0132_ff10, 0x0132_ff24] {
            any |= mem.read::<u32>(mem.rebase(a));
        }
        tk_snd_update_main(mem, call);
        u32::from(any != 0)
    }

    /// `TkSetSyncVCallback(id, f)` (`0x00887ed0`): with the sync callbacks
    /// held off (`0x013301f0` = 0 through `0x00887e80`),
    /// `TkGetSyncVCallbackHandle(id)` and `TkSetSyncVCallbackPtr(id, f)`;
    /// then on again (its result returned).
    pub fn tk_set_sync_v_callback<M: Mem>(
        mem: &mut M,
        id: u32,
        f: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let enable = mem.rebase(0x0088_7e80);
        call(mem, enable, &[0]);
        let handle = mem.rebase(0x0088_8620);
        call(mem, handle, &[id]);
        let set = mem.rebase(0x0088_8790);
        call(mem, set, &[id, f]);
        call(mem, enable, &[1])
    }

    /// `TkSyncVEI()` (`0x00887f40`): the pending count `0x013301e0` down by
    /// one while positive (else set to 0), then the sync callbacks on
    /// (`0x00887e80(1)`, its result returned).
    pub fn tk_sync_v_ei<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = mem.rebase(0x0133_01e0);
        let n: i32 = mem.read(at);
        mem.write(at, if n > 0 { n - 1 } else { 0 });
        let enable = mem.rebase(0x0088_7e80);
        call(mem, enable, &[1])
    }

    /// `Math_FacingWithin(heading, tol, x0, z0, x1, z1)` (`0x0088b780`, own
    /// name): [`crate::math::facing_within`] as 1 or 0 (`atan2` the C
    /// runtime's `_CIatan2`).
    pub fn math_facing_within<M: Mem>(
        mem: &mut M,
        heading: f32,
        tol: f32,
        from: [f32; 2],
        to: [f32; 2],
        atan2: &mut impl FnMut(&mut M, f64, f64) -> f64,
    ) -> u32 {
        u32::from(crate::math::facing_within(
            heading,
            tol,
            from,
            to,
            |a, b| atan2(mem, a, b),
        ))
    }

    /// `Math_PolarToVector(out, r, yaw, pitch)` (`0x0088a3a0`, own name):
    /// [`crate::math::polar_to_vector`] into `out[1]`, `out[3]`, `out[0]`,
    /// `out[2]` in that order (`sin_cos` the C runtime's `_CIcos`/`_CIsin`).
    pub fn math_polar_to_vector<M: Mem>(
        mem: &mut M,
        out: u32,
        [r, yaw, pitch]: [f32; 3],
        sin_cos: &mut impl FnMut(&mut M, bool, f64) -> f64,
    ) {
        let v = crate::math::polar_to_vector(r, yaw, pitch, |c, x| sin_cos(mem, c, x));
        for k in [1u32, 3, 0, 2] {
            mem.write(out.wrapping_add(4 * k), v[k as usize].to_bits());
        }
    }

    /// `Math_SegmentsCross(a, b, c, d)` (`0x00889bb0`, own name):
    /// [`crate::math::segments_cross`] of the points' `x` (`+0`) and `z`
    /// (`+8`), each loaded as a float.
    pub fn math_segments_cross(mem: &impl Mem, points: [u32; 4]) -> i32 {
        let xz = |p: u32| {
            [0, 8].map(|o| ffx_bridge::x87::round_trip(f32::from_bits(mem.read(p.wrapping_add(o)))))
        };
        let [a, b, c, d] = points.map(xz);
        crate::math::segments_cross(a, b, c, d)
    }

    /// `TkSndUpdate()` (`0x00887b60`): `SndExecMusicClearQue(1, 0)` (every
    /// queued music/SE entry counted down), then [`tk_snd_update_main`],
    /// whose result it returns (a tail jump).
    pub fn tk_snd_update<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_6000);
        call(mem, f, &[1, 0]);
        tk_snd_update_main(mem, call)
    }

    /// `TkSndUpdate2()` (`0x00887b80`): with `TkGetSoundSyncVFlag()` set,
    /// `SndExecMusicClearQue(1, 0)` and [`tk_snd_update`] (so the queue
    /// counts down twice); otherwise the flag (0).
    pub fn tk_snd_update2<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_7dd0);
        let flag = call(mem, f, &[]);
        if flag == 0 {
            return flag;
        }
        let f = mem.rebase(0x0088_6000);
        call(mem, f, &[1, 0]);
        tk_snd_update(mem, call)
    }

    /// `TkSndUpdate3(last)` (`0x00887ba0`): the sync counter
    /// (`TkGetSyncVCounter()`); when `last` is nonzero and differs from it,
    /// the sound sync flag is set and [`tk_snd_update_main`] runs first.
    pub fn tk_snd_update3<M: Mem>(
        mem: &mut M,
        last: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_7e00);
        let counter = call(mem, f, &[]);
        if last != 0 && last != counter {
            let f = mem.rebase(0x0088_7ea0);
            call(mem, f, &[]);
            tk_snd_update_main(mem, call);
        }
        counter
    }

    /// `TkSndUpdateMain()` (`0x00887bd0`): with `TkGetSoundSyncVFlag()` set,
    /// `SndSqsIopFrame()`, the voice/shout/stream busy words and the status
    /// words `0x0132ff20`/`0x0132ff24` cleared, `SndStatusUpdate()`, and
    /// `TkClearSoundSyncVFlag()` (its result returned); otherwise 0.
    pub fn tk_snd_update_main<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_7dd0);
        if call(mem, f, &[]) == 0 {
            return 0;
        }
        let f = mem.rebase(0x0088_7310);
        call(mem, f, &[]);
        for a in [
            0x0132_ff24,
            0x0132_ff14,
            0x0132_ff18,
            0x0132_ff10,
            0x0132_ff20,
        ] {
            let a = mem.rebase(a);
            mem.write(a, 0u32);
        }
        let f = mem.rebase(0x0088_7700);
        call(mem, f, &[]);
        let f = mem.rebase(0x0088_7d30);
        call(mem, f, &[])
    }

    /// `TkMsImportantSet(n)` (`0x0088e790`): `MsImportantSet(n)`; the
    /// important items `0xa004..=0xa01d` also tail-call
    /// `TOSetCollectBit(n - 0xa004)`. Otherwise returns `n - 0xa004` (left
    /// in `eax`).
    pub fn tk_ms_important_set<M: Mem>(
        mem: &mut M,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0079_0930);
        call(mem, f, &[n]);
        let bit = n.wrapping_sub(0xa004);
        if bit > 0x19 {
            return bit;
        }
        let f = mem.rebase(0x0090_6b30);
        call(mem, f, &[bit])
    }

    /// `TkRefreshSoundMode()` (`0x008724f0`): the save's config byte
    /// (`*MsGetSaveMenu()`) picks `SndSetSoundMode` 0 (bit 7), 1 (bit 0) or
    /// 2, and its bit 4 `TkSetControlerActuatorMode(0, 0, bit)`, whose
    /// result is returned.
    pub fn tk_refresh_sound_mode<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let get = mem.rebase(0x0078_5290);
        let at = call(mem, get, &[]);
        let b: u8 = mem.read(at);
        let mode = if b & 0x80 != 0 {
            0
        } else if b & 1 != 0 {
            1
        } else {
            2
        };
        let f = mem.rebase(0x0088_7180);
        call(mem, f, &[mode]);
        let at = call(mem, get, &[]);
        let b: u8 = mem.read(at);
        let f = mem.rebase(0x0088_94d0);
        call(mem, f, &[0, 0, u32::from((b >> 4) & 1)])
    }

    /// `MenuSubCmd_CenterOpen()` (`0x008e4ab0`, own name):
    /// `MenuSubCmd_CenterX(g_TkMenuSubCmdWindow)` when a sub-command window
    /// is open (its result returned), else 0.
    pub fn menu_sub_cmd_center_open<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = MENU_SUB_CMD_WINDOW.get(mem);
        if w == 0 {
            return 0;
        }
        let f = mem.rebase(0x008e_4ad0);
        call(mem, f, &[w])
    }

    /// The bytes of a NUL-terminated string (without the NUL).
    pub fn c_bytes(mem: &impl Mem, mut p: u32) -> Vec<u8> {
        let mut out = vec![];
        loop {
            let b: u8 = mem.read(p);
            if b == 0 {
                return out;
            }
            out.push(b);
            p = p.wrapping_add(1);
        }
    }

    /// The menu's VRAM save buffer (`0x01340870`, 13 blocks of 64 KiB; the
    /// start of a 5 MiB area battle uses as `g_BtlEffectOverlayDefault`).
    pub const VRAM_SAVE: u32 = 0x0134_0870;

    /// The blocks of `MenuMain_RestoreVram`/`MenuMain_SaveVram`: `block(buf,
    /// vram, 0x10000)` for 13 blocks of 64 KiB, the buffer from
    /// [`VRAM_SAVE`] and the GS address from `0xbc000` on (`0x4000` apart).
    /// Returns the last block's result.
    fn vram_blocks<M: Mem>(
        mem: &mut M,
        block: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(block);
        let mut buf = mem.rebase(VRAM_SAVE);
        let mut vram = 0xb_c000u32;
        let mut r = 0;
        for _ in 0..13 {
            r = call(mem, f, &[buf, vram, 0x1_0000]);
            buf = buf.wrapping_add(0x1_0000);
            vram = vram.wrapping_add(0x4000);
        }
        r
    }

    /// `MenuMain_RestoreVram()` (`0x008aa690`, own name): the VRAM the menu
    /// saved on opening uploaded back (`MenuMain_LoadVramBlock`, see
    /// [`vram_blocks`]) when it closes (`0x008a9ca0`).
    pub fn menu_main_restore_vram<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        vram_blocks(mem, 0x008a_a710, call)
    }

    /// `MenuMain_SaveVram()` (`0x008aab90`, own name): 13 blocks of VRAM
    /// read into the save buffer (`MenuMain_StoreVramBlock`, see
    /// [`vram_blocks`]) when the menu opens (`0x008a9ca0`).
    pub fn menu_main_save_vram<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        vram_blocks(mem, 0x008a_ac10, call)
    }

    /// The ability menu's command id (`u16`, `0x0186a3a8`).
    pub const ABILITY_COMMAND: Global<u16> = Global::new(0x0186_a3a8);
    /// 1 when the ability menu's command takes one target (`0x0186a391`).
    pub const ABILITY_SINGLE: Global<u8> = Global::new(0x0186_a391);

    /// Heals every member of a list: `TkMenuAddHPCapped(chr,
    /// TkMenuItemHealAmount(command, chr, 0))` for `get(i)`, `i < count()`.
    /// Returns what `eax` holds after: the count, or the last heal's result.
    fn heal_list<M: Mem>(
        mem: &mut M,
        count: u32,
        get: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let max = mem.rebase(count);
        let n = call(mem, max, &[]);
        let mut last = n;
        let get = mem.rebase(get);
        let amount = mem.rebase(0x008c_69a0);
        let add = mem.rebase(0x008c_8d10);
        let mut i = 0u32;
        #[allow(clippy::cast_possible_wrap)]
        while (i as i32) < n as i32 {
            let chr = call(mem, get, &[i]);
            let cmd = u32::from(ABILITY_COMMAND.get(mem));
            let heal = call(mem, amount, &[cmd, chr, 0]);
            last = call(mem, add, &[chr, heal]);
            i += 1;
        }
        last
    }

    /// `MenuAbility_ApplyHeal(target, user)` (`0x008c6d00`): the ability
    /// menu's command ([`ABILITY_COMMAND`]) heals. One target
    /// ([`ABILITY_SINGLE`] = 1): `TkMenuAddHPCapped(target,
    /// Cmd_HealAmount(command, user))`, its result returned. Otherwise the
    /// command record (`MsGetRomPlyCommand(command, local)`) picks the
    /// lists by its `+0x22`: bit 7 every aeon (`TkMenuGetSummonListMax`,
    /// `TkMenuGetSummonFromIndex`), bit 6 every party member
    /// (`TkMenuGetPlayerListMax`, `TkMenuGetPlayerFromIndex2`), each healed
    /// by `TkMenuItemHealAmount(command, chr, 0)` (not `Cmd_HealAmount`'s
    /// amount, which is computed and dropped). Returns what the original
    /// leaves in `eax`: the record, a list's count or the last heal's
    /// result. `local` is the record lookup's 8-byte out slot.
    pub fn menu_ability_apply_heal<M: Mem>(
        mem: &mut M,
        target: u32,
        user: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let cmd = u32::from(ABILITY_COMMAND.get(mem));
        let f = mem.rebase(0x008c_6920);
        let amount = call(mem, f, &[cmd, user]);
        if ABILITY_SINGLE.get(mem) == 1 {
            let f = mem.rebase(0x008c_8d10);
            return call(mem, f, &[target, amount]);
        }
        let cmd = u32::from(ABILITY_COMMAND.get(mem));
        let f = mem.rebase(0x0079_0a90);
        let rec = call(mem, f, &[cmd, local]);
        let mut r = rec;
        if mem.read::<u8>(rec.wrapping_add(0x22)) & 0x80 != 0 {
            r = heal_list(mem, 0x008a_9bf0, 0x008a_9bc0, call);
        }
        if mem.read::<u8>(rec.wrapping_add(0x22)) & 0x40 != 0 {
            r = heal_list(mem, 0x008a_9b30, 0x008a_9ad0, call);
        }
        r
    }

    /// `TkMn2DrawCommonPlateL(plate)` (`0x008bf9b0`): the menu draw cursor
    /// ([`MN2_DRAW_Y`]) stepped by 16, then the two-part plate writer
    /// `0x008f0990(cursor, +0x10, +0x14, x, y, w, +8, +0xc)` (`x`, `y`, `w`
    /// the plate's `i16`s at 0, 2, 4) writes there and its end becomes the
    /// cursor (returned).
    pub fn tk_mn2_draw_common_plate_l<M: Mem>(
        mem: &mut M,
        plate: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let half = |mem: &mut M, o: u32| {
            #[allow(clippy::cast_sign_loss)]
            let v = i32::from(mem.read::<i16>(plate.wrapping_add(o))) as u32;
            v
        };
        let word = |mem: &mut M, o: u32| mem.read::<u32>(plate.wrapping_add(o));
        let y = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        let mut args = [
            y,
            0,
            word(mem, 0x14),
            half(mem, 0),
            half(mem, 2),
            half(mem, 4),
            word(mem, 8),
            word(mem, 0xc),
        ];
        MN2_DRAW_Y.set(mem, y);
        // `+0x10` is read after the cursor's store, as the original
        args[1] = word(mem, 0x10);
        let f = mem.rebase(0x008f_0990);
        let end = call(mem, f, &args);
        MN2_DRAW_Y.set(mem, end);
        end
    }

    /// A window from `TkMenuMainAllocWindow()` with the given `(offset,
    /// handler)` callbacks; returned unregistered.
    fn alloc_window<M: Mem>(
        mem: &mut M,
        handlers: &[(u32, u32)],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        for &(o, f) in handlers {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        w
    }

    /// `MenuWShop_OpenWindow3()` (`0x008da9c0`, own name): a window with
    /// callbacks `+0xc` `0x008d9e10`, `+0x10` `0x008da6b0`, `+0x14`
    /// `0x008dab10`, layer 2 (`+0x3e`), `+0x36` = 3, kept in
    /// [`WSHOP_WINDOW3`] (what `TkMenuWShopCloseWindow3` closes) and
    /// registered; returned. The shop's opener `0x008da7a0` calls it.
    pub fn menu_wshop_open_window3<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = alloc_window(
            mem,
            &[(0xc, 0x008d_9e10), (0x10, 0x008d_a6b0), (0x14, 0x008d_ab10)],
            call,
        );
        mem.write(w.wrapping_add(0x3e), 2u8);
        mem.write(w.wrapping_add(0x36), 3u8);
        WSHOP_WINDOW3.set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuStatus_OpenMainWindow()` (`0x008d3100`, own name): a window
    /// with callbacks `+0xc` `0x008d26c0`, `+0x10` `0x008d2710`, `+0x14`
    /// `0x008d31f0`, layer 2 (`+0x3e`), `+0x36` = 3, the fade step `0x155`
    /// (`+0x54`), registered and returned; the status menu's set-up
    /// (`0x008d1d80`) keeps it in `0x0186a838`.
    pub fn menu_status_open_main_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = alloc_window(
            mem,
            &[(0xc, 0x008d_26c0), (0x10, 0x008d_2710), (0x14, 0x008d_31f0)],
            call,
        );
        mem.write(w.wrapping_add(0x3e), 2u8);
        mem.write(w.wrapping_add(0x36), 3u8);
        mem.write(w.wrapping_add(0x54), 0x155u16);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuSummon_OpenWindow()` (`0x008cd1b0`, own name): the aeon menu's
    /// window ([`SUMMON_WINDOW`], closed by `TkMenuSummonCloseWindow`) with
    /// callbacks `+8` `0x008ccac0`, `+0xc` `0x008cced0`, `+0x10`
    /// `0x008cebe0`, `+0x14` `0x008cd230`, `+0x1c` `0x008ccad0` and the
    /// fade step `0x155` (`+0x54`), registered. Returns what
    /// `TkMenuMainRegistWindow` returns.
    pub fn menu_summon_open_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = alloc_window(
            mem,
            &[
                (8, 0x008c_cac0),
                (0xc, 0x008c_ced0),
                (0x10, 0x008c_ebe0),
                (0x14, 0x008c_d230),
                (0x1c, 0x008c_cad0),
            ],
            call,
        );
        SUMMON_WINDOW.set(mem, w);
        mem.write(w.wrapping_add(0x54), 0x155u16);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w])
    }

    /// The HDD-install flag (`u8`, `0x0133c913`; printed as `ffx_hddmode`).
    pub const HDD_MODE: Global<u8> = Global::new(0x0133_c913);

    /// `TkRefreshHDDMode()` (`0x0088e930`): the sound driver's wave and
    /// music info paths, `SqsInitWaveInfoHdd(get_hdd_name(0x2c))` and
    /// `SqsInitMusicInfoHdd(get_hdd_name(0x2d))`, or the empty string
    /// (`0x00b8fbeb`) for each when [`HDD_MODE`] is 0. Returns the music
    /// call's result.
    pub fn tk_refresh_hdd_mode<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut last = 0;
        for (name, init) in [(0x2cu32, 0x0081_d470u32), (0x2d, 0x0081_d3c0)] {
            let path = if HDD_MODE.get(mem) == 0 {
                mem.rebase(0x00b8_fbeb)
            } else {
                let f = mem.rebase(0x0077_7eb0);
                call(mem, f, &[name])
            };
            let f = mem.rebase(init);
            last = call(mem, f, &[path]);
        }
        last
    }

    /// `SsSetProjectBase(path)` (`0x00887a90`): a jump to
    /// `SqsSetProjectBase(path)`, whose result it returns.
    pub fn ss_set_project_base<M: Mem>(
        mem: &mut M,
        path: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0081_eab0);
        call(mem, f, &[path])
    }

    /// `TkMn2StringWidth(text)` (`0x008c2cb0`): the width half of
    /// `TOGetMessageSizePacked(text)` (its low 16 bits, zero-extended).
    pub fn tk_mn2_string_width<M: Mem>(
        mem: &mut M,
        text: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008b_7170);
        call(mem, f, &[text]) & 0xffff
    }

    /// `MenuItem_LoadTargetKeys(target)` (`0x008c5320`, own name): for
    /// `target >= 0`, the key string of the target's message
    /// (`TOMenuGetNameMes` of the `u16` id at `0x00c5a0d8 + 4 * target`,
    /// or `0x00c5a120` when `TOGetFFXLang()` is not 0) parsed into
    /// [`ITEM_KEYS`] ([`menu_parse_item_keys`], a tail jump). A negative
    /// target does nothing. Its `eax` is the parser's leftover (unread).
    pub fn menu_item_load_target_keys<M: Mem>(
        mem: &mut M,
        target: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if target < 0 {
            return;
        }
        let f = mem.rebase(0x008a_c2f0);
        let lang = call(mem, f, &[]);
        let table = mem.rebase(if lang == 0 { 0x00c5_a0d8 } else { 0x00c5_a120 });
        let id: u16 = mem.read(table.wrapping_add(target.cast_unsigned().wrapping_mul(4)));
        let f = mem.rebase(0x008b_edd0);
        let s = call(mem, f, &[u32::from(id)]);
        menu_parse_item_keys(mem, s);
    }

    /// The aeon stats whose growth the party can pay for (`u32` bit mask,
    /// `0x0186a604`).
    pub const SUMMON_AFFORD_MASK: Global<u32> = Global::new(0x0186_a604);
    /// The aeon stats that can grow and can be paid for (`u32` bit mask,
    /// `0x0186a600`).
    pub const SUMMON_GROW_MASK: Global<u32> = Global::new(0x0186_a600);

    /// `MenuSummon_RefreshGrowMasks(chr)` (`0x008cd3b0`, own name): both
    /// masks cleared, then for the 10 stats `MenuSummon_GrowInfo(stat, chr,
    /// &item, &count)`: when `count <= MsGetSaveItemNum(item)` (signed) the
    /// stat's bit joins [`SUMMON_AFFORD_MASK`], and [`SUMMON_GROW_MASK`]
    /// too when the info call returned non-zero. Returns the last item
    /// count. `local` is 8 bytes of out slots (item, count).
    pub fn menu_summon_refresh_grow_masks<M: Mem>(
        mem: &mut M,
        chr: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        SUMMON_GROW_MASK.set(mem, 0);
        SUMMON_AFFORD_MASK.set(mem, 0);
        let info = mem.rebase(0x008c_d320);
        let num = mem.rebase(0x0079_04b0);
        let (item, count) = (local, local.wrapping_add(4));
        let mut owned = 0;
        for stat in 0..10u32 {
            let bit = 1u32 << stat;
            let grows = call(mem, info, &[stat, chr, item, count]);
            let id: u32 = mem.read(item);
            owned = call(mem, num, &[id]);
            if mem.read::<i32>(count) <= owned.cast_signed() {
                let m = SUMMON_AFFORD_MASK.get(mem);
                SUMMON_AFFORD_MASK.set(mem, m | bit);
                if grows != 0 {
                    let m = SUMMON_GROW_MASK.get(mem);
                    SUMMON_GROW_MASK.set(mem, m | bit);
                }
            }
        }
        owned
    }

    /// `BtlEnd_GetSaveNextAP(chr)` (`0x008bda00`, own name): a jump to
    /// `MsGetSaveNextAP(chr)`; its caller is the battle results' AP code (`0x008bdaf0`).
    pub fn btl_end_get_save_next_ap<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_52b0);
        call(mem, f, &[chr])
    }

    /// `BtlEnd_AddGil(gil)` (`0x008ba3a0`, own name): a jump to
    /// `MsPayGIL(-gil)`; the battle results add their gil with it
    /// (`0x008ba590`).
    pub fn btl_end_add_gil<M: Mem>(
        mem: &mut M,
        gil: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_59a0);
        call(mem, f, &[gil.wrapping_neg()])
    }

    /// `TkMn2DrawKickPacket()` (`0x008c0c40`):
    /// [`tk_mn2_draw_kick_packet_body`] with the scissor clear; its result.
    pub fn tk_mn2_draw_kick_packet<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        tk_mn2_draw_kick_packet_body(mem, 1, call)
    }

    /// `TkSyncVDI()` (`0x00887f20`): the v-sync disable count (`0x013301e0`,
    /// which [`tk_sync_v_ei`] counts down) up by one, then
    /// `SetGlobal013301f0(0)`, whose result it returns.
    pub fn tk_sync_vdi<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = mem.rebase(0x0133_01e0);
        let n: u32 = mem.read(at);
        mem.write(at, n.wrapping_add(1));
        let f = mem.rebase(0x0088_7e80);
        call(mem, f, &[0])
    }

    /// `TkMenuRefreshPartyList()` (`0x008aae10`): `TOPlayerListInit(0)`, then
    /// `TOSummonListInit(0)`, whose result it returns.
    pub fn tk_menu_refresh_party_list<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_8f40);
        call(mem, f, &[0]);
        let f = mem.rebase(0x008a_9090);
        call(mem, f, &[0])
    }

    /// `MenuMain_RemoveAllMenus()` (`0x008aadd0`, own name):
    /// `TkMenuMainRemoveMenu(slot)` for the 25 slots; the last result. The
    /// menu's close (`0x008a9ca0`) calls it.
    pub fn menu_main_remove_all_menus<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_ab20);
        let mut last = 0;
        for slot in 0..0x19 {
            last = call(mem, f, &[slot]);
        }
        last
    }

    /// `TkChPatchM339(chr)` (`0x0082ba00`): `Ch_ModelSetHide(chr, 1, 0)`
    /// for a character; returns its result, or 0 (the null `chr`).
    pub fn tk_ch_patch_m339<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if chr == 0 {
            return 0;
        }
        let f = mem.rebase(0x0082_aea0);
        call(mem, f, &[chr, 1, 0])
    }

    /// `Pad_StickToByte(axis)` (`0x00889aa0`, own name): a stick axis
    /// (`f32`, -1..1) as the PS2 pad's byte, `0x80 - ftol(axis * -127.0)`
    /// (the product in x87 double precision, `_ftol2_sse`), in `al`; the
    /// rest of `eax` keeps the truncation's upper 24 bits.
    pub fn pad_stick_to_byte<M: Mem>(mem: &mut M, axis: u32) -> u32 {
        use ffx_bridge::x87;
        let t = x87::ftol(mem, x87::mul(x87::load(axis), -127.0));
        (t & 0xffff_ff00) | (0x80u32.wrapping_sub(t) & 0xff)
    }

    /// `TkMenuStartScene(scene, group)` (`0x00a7c880`): group `group` of the
    /// scene switched on (`MenuScene_ResetGroup(scene, group, 1)`); the
    /// entry it returns, when any, gets bits 30 and 31 of `+4`. Returns
    /// the entry.
    pub fn tk_menu_start_scene<M: Mem>(
        mem: &mut M,
        scene: u32,
        group: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x00a7_ba70);
        let e = call(mem, f, &[scene, group, 1]);
        if e != 0 {
            let at = e.wrapping_add(4);
            let v: u32 = mem.read(at);
            mem.write(at, v | 0xc000_0000);
        }
        e
    }

    /// `MenuSave_McWrite(fd, buf, size)` (`0x00888570`, own name): 0 while
    /// `TkGetTGS()` is set, else a jump to `sceMcWrite(fd, buf, size)`; the
    /// save menu writes its data with it (`0x008b15d0`).
    pub fn menu_save_mc_write<M: Mem>(
        mem: &mut M,
        args: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_d6d0);
        if call(mem, f, &[]) != 0 {
            return 0;
        }
        let f = mem.rebase(0x0062_fa10);
        call(mem, f, &args)
    }

    /// The help screen's memory in use (`i32`, `0x0112a09c`; `HelpMem_Get`
    /// at `0x00774c00` adds).
    pub const HELP_MEM_USED: Global<u32> = Global::new(0x0112_a09c);

    /// `HelpMem_Free(size)` (`0x00774aa0`, own name): `size` rounded up to
    /// 64 (signed) taken off [`HELP_MEM_USED`]; below 0 is
    /// `yiAssert(0, 0x00b5313c, 0x00b533f4)`. Returns `-rounded`, or the
    /// assertion's result.
    pub fn help_mem_free<M: Mem>(
        mem: &mut M,
        size: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let up = size.wrapping_add(0x3f);
        let neg = (up.wrapping_add((up >> 31) & 0x3f) >> 6)
            .wrapping_neg()
            .wrapping_shl(6)
            .cast_unsigned();
        let used = HELP_MEM_USED.get(mem).wrapping_add(neg);
        HELP_MEM_USED.set(mem, used);
        if used.cast_signed() >= 0 {
            return neg;
        }
        let f = mem.rebase(0x0090_ae50);
        let (file, msg) = (mem.rebase(0x00b5_313c), mem.rebase(0x00b5_33f4));
        call(mem, f, &[0, file, msg])
    }

    /// `InitSysModuleEncoding()` (`0x00646910`): the character tables of
    /// the game's language, `MallocAndReadGamecode(TOGetFFXLang())`, whose
    /// result it returns.
    pub fn init_sys_module_encoding<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_c2f0);
        let lang = call(mem, f, &[]);
        let f = mem.rebase(0x0064_6920);
        call(mem, f, &[lang])
    }

    /// `TkInitSyncV()` (`0x00887e30`): the v-sync counters
    /// (`0x01330220`, `g_TkSyncVCounter`, `g_TkTotalSyncVCounter`,
    /// `0x013301f0`, the disable count `0x013301e0`) cleared,
    /// `TkSetSyncVCallback(0, 0x00887f70)`, then the three sync flag
    /// setters `0x00887ea0`, `0x00887eb0` and `0x00887ec0` (a tail jump:
    /// its result).
    pub fn tk_init_sync_v<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for g in [
            0x0133_0220u32,
            0x0133_0200,
            0x0133_0210,
            0x0133_01f0,
            0x0133_01e0,
        ] {
            let at = mem.rebase(g);
            mem.write(at, 0u32);
        }
        let f = mem.rebase(0x0088_7ed0);
        let cb = mem.rebase(0x0088_7f70);
        call(mem, f, &[0, cb]);
        let mut last = 0;
        for g in [0x0088_7ea0u32, 0x0088_7eb0, 0x0088_7ec0] {
            let f = mem.rebase(g);
            last = call(mem, f, &[]);
        }
        last
    }

    /// The save copy the tutorial plays on (`0x01871524`, a pointer:
    /// `TkMenuCtrlTuto` copies `g_SaveData` to `0x0186ac60` and points it
    /// there).
    pub const TUTO_SAVE_COPY: Global<u32> = Global::new(0x0187_1524);

    /// `MenuTuto_RestoreNames()` (`0x008db7e0`, own name): the 18 character
    /// names (`MsGetSaveChrName(i)`, `i` 0..=17) copied back byte by byte,
    /// terminator included, from the same offsets in [`TUTO_SAVE_COPY`],
    /// then the save's byte `+0xd1` from the copy into
    /// `MsGetSaveEventAddress()`. Returns that byte in `al`, the rest of
    /// `eax` from the last name's address.
    pub fn menu_tuto_restore_names<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_c830);
        let save = call(mem, f, &[]);
        let copy0 = TUTO_SAVE_COPY.get(mem);
        let name = mem.rebase(0x0078_4fb0);
        let base = mem.rebase(0x0112_ca90);
        let mut dst = 0;
        for i in 0..=0x11u32 {
            dst = call(mem, name, &[i]);
            let mut src = TUTO_SAVE_COPY.get(mem).wrapping_sub(base).wrapping_add(dst);
            let delta = dst.wrapping_sub(src);
            loop {
                let b: u8 = mem.read(src);
                mem.write(src.wrapping_add(delta), b);
                src = src.wrapping_add(1);
                if b == 0 {
                    break;
                }
            }
        }
        let b: u8 = mem.read(copy0.wrapping_add(0xd1));
        mem.write(save.wrapping_add(0xd1), b);
        (dst & 0xffff_ff00) | u32::from(b)
    }

    /// `MenuWShop_OpenSelWindow()` (`0x008da8f0`, own name): a window kept
    /// in `TUTO_WINDOWS[0]` (`0x0186ac34`) with callbacks `+0xc`
    /// `0x008d9da0`, `+0x10` `0x008da050`, `+0x14` `0x008daaf0`; `+0x48`
    /// and `+0x32` 0, `+0x30` the list's count (`u16` `0x0186ac24`), layer
    /// 3 (`+0x3e`), `+0x34` `0x30000`, `+0x42` 1, `+0x3a` 10, `+0x45` -1;
    /// registered and returned. `TkMenuCtrlWShop` opens it.
    pub fn menu_wshop_open_sel_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        mem.write(w.wrapping_add(0x48), 0u16);
        mem.write(w.wrapping_add(0x32), 0u16);
        let count: u16 = mem.read(mem.rebase(0x0186_ac24));
        mem.write(w.wrapping_add(0x30), count);
        for (o, f) in [
            (0xcu32, 0x008d_9da0u32),
            (0x10, 0x008d_a050),
            (0x14, 0x008d_aaf0),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 3u8);
        mem.write(w.wrapping_add(0x34), 0x3_0000u32);
        mem.write(w.wrapping_add(0x42), 1u8);
        mem.write(w.wrapping_add(0x3a), 10u16);
        mem.write(w.wrapping_add(0x45), 0xffu8);
        TUTO_WINDOWS[0].set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `Pad_CheckPressMode(port, slot, ret, status)` (`0x00889510`, own
    /// name): with `scePadInfoPressMode(port, slot)` = 1, 7 when the pad's
    /// type (`status[1] & 0xf`) is not 9 and `scePadEnterPressMode(port,
    /// slot)` (`0x0062fb10`) is non-zero, else `ret`; otherwise the
    /// controller work's (`TkGetControlerWork(port, slot)`) `+0x82` cleared
    /// and `ret`. On PC both pad calls return 0.
    pub fn pad_check_press_mode<M: Mem>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [port, slot, ret, status] = args;
        let f = mem.rebase(0x0088_8ec0);
        let work = call(mem, f, &[port, slot]);
        let f = mem.rebase(0x0062_fb70);
        if call(mem, f, &[port, slot]) != 1 {
            mem.write(work.wrapping_add(0x82), 0u8);
            return ret;
        }
        let kind: u8 = mem.read(status.wrapping_add(1));
        if kind & 0xf == 9 {
            return ret;
        }
        let f = mem.rebase(0x0062_fb10);
        if call(mem, f, &[port, slot]) == 0 {
            return ret;
        }
        7
    }

    /// The menu's frame buffer save (`0x01410870`, 13 blocks of `0xc000`
    /// bytes).
    pub const FRAME_SAVE: u32 = 0x0141_0870;

    /// `MenuMain_SaveFrameBuffer()` (`0x008aac80`, own name): the shown
    /// buffer's index (`0x018408f4`, which `Sg_GetDBuffDC` fills) kept to
    /// its low bit, the frame base `(word +0x10 of entry index (40 bytes
    /// each) & 0x1ff) << 11`, then `MenuMain_StoreFrameBlock(buf, gs,
    /// 0xc000)` for 13 blocks from [`FRAME_SAVE`] and the base (`0xc000`
    /// and `0x4000` apart). The menu's opening (`0x008a9ca0`) calls it after
    /// [`menu_main_save_vram`]. Returns the last block's result.
    pub fn menu_main_save_frame_buffer<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let index = mem.rebase(0x0184_08f4);
        let f = mem.rebase(0x0082_0530);
        let dc = call(mem, f, &[index]);
        let i = mem.read::<u32>(index) & 1;
        mem.write(index, i);
        let word: u32 = mem.read(dc.wrapping_add(i * 40 + 0x10));
        let mut gs = (word & 0x1ff) << 11;
        let mut buf = mem.rebase(FRAME_SAVE);
        let f = mem.rebase(0x008a_acf0);
        let mut last = 0;
        for _ in 0..13 {
            last = call(mem, f, &[buf, gs, 0xc000]);
            buf = buf.wrapping_add(0xc000);
            gs = gs.wrapping_add(0x4000);
        }
        last
    }

    /// `TkRefreshHdd()` (`0x008724a0`): the HDD mode from the save's menu
    /// config (bit 13 of `*MsGetSaveMenu()`), cleared unless the HDD is
    /// ready (`get_hdd_word_518()` not 3 and `get_hdd_status()` = 1), kept
    /// in the FFX config's `+0xe` (`TkGetFFXCofingAdrs()`) and
    /// [`HDD_MODE`]; then [`tk_refresh_hdd_mode`] (a tail jump: its
    /// result).
    pub fn tk_refresh_hdd<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_5290);
        let menu = call(mem, f, &[]);
        let f = mem.rebase(0x0088_d5e0);
        let config = call(mem, f, &[]);
        let mut on = u8::from(mem.read::<u32>(menu) & 0x2000 != 0);
        let f = mem.rebase(0x0077_87b0);
        let ready = call(mem, f, &[]) != 3 && {
            let f = mem.rebase(0x0077_7e90);
            call(mem, f, &[]) == 1
        };
        if !ready {
            on = 0;
        }
        mem.write(config.wrapping_add(0xe), on);
        HDD_MODE.set(mem, on);
        tk_refresh_hdd_mode(mem, call)
    }

    /// `BtlEnd_StepGil()` (`0x008ba590`, own name): one step of the battle
    /// results' count-down ([`BTL_END_GIL`]: the step, the rest): the step,
    /// or the whole rest when not below it (unsigned), comes off the rest
    /// and is paid out as gil ([`btl_end_add_gil`], so the counted value
    /// is gil, not AP). Returns 1 while something remains, else 0.
    pub fn btl_end_step_gil<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [step, rest] = BTL_END_GIL.get(mem);
        let (take, more) = if step < rest { (step, 1) } else { (rest, 0) };
        let at = BTL_END_GIL.ptr(mem).addr.wrapping_add(4);
        mem.write(at, rest.wrapping_sub(take));
        let f = mem.rebase(0x008b_a3a0);
        call(mem, f, &[take]);
        more
    }

    /// `TkInitWork()` (`0x0088e570`, own name; `TkInit` calls it first):
    /// `0x01330878` and `0x0133087c` cleared, `TkInitAlarm()`,
    /// `TkInitControlerWork()`, [`tk_init_sync_v`], then the debug menu
    /// status 1, the encounter output, `g_SndStreamTable`, the HDD error
    /// and debug draw flags 0, and the save's `+0x2b` 0 and `+0x13` bit 1
    /// set, bit 2 cleared (`MsGetSaveEventAddress()`, whose result it
    /// returns).
    pub fn tk_init_work<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        for g in [0x0133_0878u32, 0x0133_087c] {
            let at = mem.rebase(g);
            mem.write(at, 0u32);
        }
        for f in [0x0088_c810u32, 0x0088_8f00] {
            let f = mem.rebase(f);
            call(mem, f, &[]);
        }
        let f = mem.rebase(0x0088_7e30);
        call(mem, f, &[]);
        let w = |mem: &mut M, g: u32, v: u8| {
            let at = mem.rebase(g);
            mem.write(at, v);
        };
        w(mem, 0x0133_0870, 1);
        w(mem, 0x0133_0871, 0);
        let at = mem.rebase(0x023c_bec4);
        mem.write(at, 0u32);
        w(mem, 0x0133_0873, 0);
        let f = mem.rebase(0x0078_5240);
        let save = call(mem, f, &[]);
        mem.write(save.wrapping_add(0x2b), 0u8);
        w(mem, 0x0133_0872, 0);
        let save = call(mem, f, &[]);
        let b: u8 = mem.read(save.wrapping_add(0x13));
        mem.write(save.wrapping_add(0x13), (b & 0xfb) | 2);
        save
    }

    /// `MenuWShop_OpenListWindow()` (`0x008d8260`, own name): the weapon
    /// shop's list window ([`WSHOP_WINDOWS`]`[0]`, which
    /// `MenuWShop_RestartListWindow` restarts): callbacks `+0xc`
    /// `0x008d77f0`, `+0x10` `0x008d7aa0`, `+0x14` `0x008d84e0`; `+0x48`
    /// (`u16`) and `+0x32` (`u32`) 0, `+0x30` the list's count (`u16`
    /// `0x0186ab10`), layer 3, `+0x42` 1, `+0x3a` 9, `+0x45` -1, `+0x37` 0,
    /// `+0x28` 0; registered and returned.
    pub fn menu_wshop_open_list_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        mem.write(w.wrapping_add(0x48), 0u16);
        mem.write(w.wrapping_add(0x32), 0u32);
        let count: u16 = mem.read(mem.rebase(0x0186_ab10));
        mem.write(w.wrapping_add(0x30), count);
        for (o, f) in [
            (0xcu32, 0x008d_77f0u32),
            (0x10, 0x008d_7aa0),
            (0x14, 0x008d_84e0),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 3u8);
        mem.write(w.wrapping_add(0x42), 1u8);
        mem.write(w.wrapping_add(0x3a), 9u16);
        mem.write(w.wrapping_add(0x45), 0xffu8);
        mem.write(w.wrapping_add(0x37), 0u8);
        mem.write(w.wrapping_add(0x28), 0u32);
        WSHOP_WINDOWS[0].set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        w
    }

    /// `MenuWShop_OpenSubWindow1()` (`0x008d8370`, own name): the weapon
    /// shop's second window ([`WSHOP_WINDOWS`]`[1]`): callbacks `+0xc`
    /// `0x008d79c0`, `+0x10` `0x008d7d20`, `+0x14` `0x008d8500`, `+0x1c`
    /// `0x008d75d0`; layer 3, `+0x48` (`u16`) and `+0x32` (`u32`) 0, `+0x42`
    /// 1, `+0x30` `TkMn2GetNameModeMax()` (`u16`), `+0x3a` 9, `+0x45` -1,
    /// `+0x28` 0, `+0x37` 0; registered, then `MenuWShop_InitCursor`
    /// (its result returned).
    pub fn menu_wshop_open_sub_window1<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        for (o, f) in [
            (0xcu32, 0x008d_79c0u32),
            (0x10, 0x008d_7d20),
            (0x14, 0x008d_8500),
            (0x1c, 0x008d_75d0),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 3u8);
        mem.write(w.wrapping_add(0x48), 0u16);
        mem.write(w.wrapping_add(0x32), 0u32);
        mem.write(w.wrapping_add(0x42), 1u8);
        let f = mem.rebase(0x008c_1bc0);
        let max = call(mem, f, &[]);
        let [lo, hi, ..] = max.to_le_bytes();
        mem.write(w.wrapping_add(0x30), u16::from_le_bytes([lo, hi]));
        mem.write(w.wrapping_add(0x3a), 9u16);
        mem.write(w.wrapping_add(0x45), 0xffu8);
        mem.write(w.wrapping_add(0x28), 0u32);
        mem.write(w.wrapping_add(0x37), 0u8);
        WSHOP_WINDOWS[1].set(mem, w);
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        let f = mem.rebase(0x008d_9580);
        call(mem, f, &[w])
    }

    /// `MenuSummon_OpenStatusWindow()` (`0x008cd140`): the aeon status
    /// window ([`SUMMON_STATUS_WINDOW`], which `MenuSummon_CloseStatusWindow`
    /// closes):
    /// callbacks `+0xc` `MenuSummon_StatusWindowCtrl`, `+0x10`
    /// `0x008cd080`, `+0x14` `0x008cd220`, `+0x45` 0, `+0x36` 3; then
    /// `0x023cc3c8` 0 and `TkMenuGetCurrentSummonPos()` three times into
    /// `0x023cc3c4`, `0x023cc3c0`, `0x023cc3bc`; registered;
    /// `MenuSummon_BuildCommandList(TkMenuGetCurrentSummon())`, and
    /// `TkMn2InitStatusBG()` (a tail jump: its result).
    pub fn menu_summon_open_status_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        SUMMON_STATUS_WINDOW.set(mem, w);
        for (o, f) in [
            (0xcu32, 0x008c_cb60u32),
            (0x10, 0x008c_d080),
            (0x14, 0x008c_d220),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x45), 0u8);
        mem.write(w.wrapping_add(0x36), 3u8);
        let at = mem.rebase(0x023c_c3c8);
        mem.write(at, 0u32);
        let pos = mem.rebase(0x008a_9890);
        for g in [0x023c_c3c4u32, 0x023c_c3c0, 0x023c_c3bc] {
            let v = call(mem, pos, &[]);
            let at = mem.rebase(g);
            mem.write(at, v);
        }
        let regist = mem.rebase(0x008a_ab00);
        call(mem, regist, &[w]);
        let f = mem.rebase(0x008a_9880);
        let chr = call(mem, f, &[]);
        let f = mem.rebase(0x008d_2b40);
        call(mem, f, &[chr]);
        let f = mem.rebase(0x008d_4ed0);
        call(mem, f, &[])
    }

    /// `BtlEnd_SelectedWeapon()` (`0x008bda80`, own name): the weapon under
    /// the battle results' cursor, `MsGetSaveWeapon(id, local)`: from the
    /// spoils (`u16` `0x02310f8e[[0x0186a1a4]]`) when the side flag
    /// (`0x0186a19c`, inverted while `0x0186a1a8` is set) is non-zero, else
    /// from the list at `[0x025cc580]` (`u16` at `[0x0186a190] +
    /// [0x0186a1a0]`). Returns the record. `local` is the 4-byte out slot.
    pub fn btl_end_selected_weapon<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = |mem: &mut M, a: u32| mem.read::<u32>(mem.rebase(a));
        let mut side = g(mem, 0x0186_a19c);
        if g(mem, 0x0186_a1a8) != 0 {
            side = u32::from(side == 0);
        }
        let at = if side == 0 {
            let i = g(mem, 0x0186_a190).wrapping_add(g(mem, 0x0186_a1a0));
            g(mem, 0x025c_c580).wrapping_add(i.wrapping_mul(2))
        } else {
            let i = g(mem, 0x0186_a1a4);
            mem.rebase(0x0231_0f8e).wrapping_add(i.wrapping_mul(2))
        };
        let id: u16 = mem.read(at);
        let f = mem.rebase(0x007a_bbd0);
        call(mem, f, &[u32::from(id), local])
    }

    /// `MenuKaizou_OpenWindow()` (`0x008d6d60`, own name): the customize
    /// menu's window ([`KAIZOU_WINDOW`], kept after registering): callbacks
    /// `+8` `0x008d56e0`, `+0xc` `UpdateGearCustomizationMenuState`,
    /// `+0x10` `0x008d6860`, `+0x14` `0x008d6e30`, `+0x1c` `0x008d57a0`;
    /// `+0x48` (`u16`) and `+0x32` (`u32`) 0, `+0x30` the list's count
    /// (`u16` `0x0186aa3c`), `+0x3e` `u16` `0x101`, `+0x42` 1, `+0x36`
    /// `u16` 3, `+0x3a` 9. Returns what `TkMenuMainRegistWindow` returns.
    pub fn menu_kaizou_open_window<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let alloc = mem.rebase(0x008a_a1a0);
        let w = call(mem, alloc, &[]);
        mem.write(w.wrapping_add(0x48), 0u16);
        mem.write(w.wrapping_add(0x32), 0u32);
        let count: u16 = mem.read(mem.rebase(0x0186_aa3c));
        mem.write(w.wrapping_add(0x30), count);
        for (o, f) in [
            (8u32, 0x008d_56e0u32),
            (0xc, 0x008d_5830),
            (0x10, 0x008d_6860),
            (0x14, 0x008d_6e30),
            (0x1c, 0x008d_57a0),
        ] {
            let handler = mem.rebase(f);
            mem.write(w.wrapping_add(o), handler);
        }
        mem.write(w.wrapping_add(0x3e), 0x101u16);
        mem.write(w.wrapping_add(0x42), 1u8);
        mem.write(w.wrapping_add(0x36), 3u16);
        mem.write(w.wrapping_add(0x3a), 9u16);
        let regist = mem.rebase(0x008a_ab00);
        let r = call(mem, regist, &[w]);
        KAIZOU_WINDOW.set(mem, w);
        r
    }

    /// `TkFindLegendWeapon_008c30e0(chr)` (`0x008c30e0`; the Switch inlines
    /// it into `TkSetLegendAbility`, its only caller here): the first of
    /// `chr`'s weapons (`MsGetSaveWeaponList(chr, -1, 0, &n)`) whose record
    /// (`MsGetSaveWeapon(id, &x)`) has flag `+3` bit 2 (the celestial
    /// weapon) and owner byte `+4` == `chr`, or 0; `dbgPrintf` "TK:Found
    /// Legend weapon(%d)" or "TK:not found ..." with `chr` either way.
    /// `local` is 8 bytes of out slots (`n`, then `x`).
    pub fn tk_find_legend_weapon<M: Mem>(
        mem: &mut M,
        chr: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_battle::save::bridge as save;
        let mut ids = save::ms_get_save_weapon_list(mem, chr.cast_signed(), -1, 0, local);
        let mut found = None;
        while mem.read::<i32>(local) > 0 {
            let id: u16 = mem.read(ids);
            let get = mem.rebase(0x007a_bbd0);
            let rec = call(mem, get, &[u32::from(id), local.wrapping_add(4)]);
            if mem.read::<u8>(rec.wrapping_add(3)) & 4 != 0
                && u32::from(mem.read::<u8>(rec.wrapping_add(4))) == chr
            {
                found = Some(rec);
                break;
            }
            let n: i32 = mem.read(local);
            mem.write(local, n.wrapping_sub(1));
            ids = ids.wrapping_add(2);
        }
        let text = if found.is_some() {
            0x00b6_57b4
        } else {
            0x00b6_57d0
        };
        let (f, text) = (mem.rebase(0x0062_f500), mem.rebase(text));
        call(mem, f, &[text, chr]);
        found.unwrap_or(0)
    }

    /// `BtlEnd_InParty(chr)` (`0x008b9ec0`, own name): 1 when `chr` is one
    /// of the three party slots `MsGetSavePartyMember(&a, &b, &c)` fills
    /// and not `0xff`, else 0. `local` is 12 bytes of out slots.
    pub fn btl_end_in_party<M: Mem>(
        mem: &mut M,
        chr: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_52f0);
        call(
            mem,
            f,
            &[local, local.wrapping_add(4), local.wrapping_add(8)],
        );
        for i in 0..3u32 {
            let slot: u32 = mem.read(local.wrapping_add(4 * i));
            if slot == chr && slot != 0xff {
                return 1;
            }
        }
        0
    }

    /// `MenuWShop_ClampSubWindow1()` (`0x008d8450`, own name): with the
    /// weapon shop's second window ([`WSHOP_WINDOWS`]`[1]`) open, its
    /// cursor (`i16 +0x48`) kept below the count (`u16 +0x30`), the scroll
    /// (`i16 +0x32`) brought to the cursor's page (`+0x3a` rows, at least
    /// 0), copied to `+0x34`; then `TkMenuRestartSelFileWindow(w)`, `+0x28`
    /// and `+0x45` cleared. Returns the window (re-read), or `None` when
    /// none is open (the original leaves `eax` alone).
    pub fn menu_wshop_clamp_sub_window1<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let w = WSHOP_WINDOWS[1].get(mem);
        if w == 0 {
            return None;
        }
        let count: u16 = mem.read(w.wrapping_add(0x30));
        if i32::from(mem.read::<i16>(w.wrapping_add(0x48))) >= i32::from(count) {
            mem.write(w.wrapping_add(0x48), count.wrapping_sub(1));
        }
        let cur: u16 = mem.read(w.wrapping_add(0x48));
        if cur.cast_signed() < mem.read::<i16>(w.wrapping_add(0x32)) {
            mem.write(w.wrapping_add(0x32), cur);
        }
        let rows: u16 = mem.read(w.wrapping_add(0x3a));
        let top = i32::from(mem.read::<i16>(w.wrapping_add(0x32))) + i32::from(rows);
        if i32::from(cur.cast_signed()) >= top {
            let v = cur.wrapping_sub(rows).wrapping_sub(1);
            mem.write(w.wrapping_add(0x32), v);
            if v.cast_signed() < 0 {
                mem.write(w.wrapping_add(0x32), 0u16);
            }
        }
        let scroll: u16 = mem.read(w.wrapping_add(0x32));
        mem.write(w.wrapping_add(0x34), scroll);
        let f = mem.rebase(0x008b_5530);
        call(mem, f, &[w]);
        let w = WSHOP_WINDOWS[1].get(mem);
        mem.write(w.wrapping_add(0x28), 0u32);
        mem.write(w.wrapping_add(0x45), 0u8);
        Some(w)
    }

    /// The save counter: one more for every save packed (`0x008b3e60`),
    /// stored in the header capped at `0x80000000`.
    pub const MENU_SAVE_COUNTER: Global<u32> = Global::new(0x0186_627c);

    /// Bytes of save data behind a packed save's `0x40`-byte header.
    pub const SAVE_DATA_SIZE: u32 = 0x68c0;

    /// The save file buffer (`0x40`-byte header, then the save data).
    pub const SAVE_FILE_BUFFER: Global<u32> = Global::new(0x0159_7f70);
    /// The save slot `TkMenuPreSave` packs and describes (set to 1 there).
    pub const MENU_SAVE_SLOT: Global<u32> = Global::new(0x0186_628c);
    /// Non-zero while a caller hands `TkMenuPreSave` an already packed
    /// save in [`SAVE_FILE_BUFFER`] and its location in [`MENU_PRE_SAVE_DIC`].
    pub const MENU_PRE_SAVE_PACKED: Global<u32> = Global::new(0x0186_6244);
    /// The save location (`AtelGetSaveDic`) a caller hands `TkMenuPreSave`.
    pub const MENU_PRE_SAVE_DIC: Global<u32> = Global::new(0x0186_624c);
    /// `g_SaveData_LoadResult` (`u8`), which `TkMenuPreSave` sets to 1.
    pub const SAVE_DATA_LOAD_RESULT: Global<u8> = Global::new(0x0159_7f74);

    /// The `TkMenuPreSave` frame (`local`, `0x16cc` bytes) laid out as the
    /// original's: each buffer's offset below the frame's end.
    mod pre_save {
        pub const LEN: u32 = 0x16cc;
        pub const TITLE: u32 = 0x44;
        pub const SUBTITLE: u32 = 0xc4;
        pub const TEXT_A: u32 = 0x4c4;
        pub const TEXT_B: u32 = 0x8c4;
        pub const TEXT_C: u32 = 0xec4;
        pub const LABEL: u32 = 0xcc4;
        pub const PLACE: u32 = 0xac4;
        pub const TIME: u32 = 0x10c4;
        pub const DETAIL: u32 = 0x12c4;
        pub const PLACE_UTF8: u32 = 0x14c4;
        pub const TIME_LABEL: u32 = 0x16c4;
    }

    /// `memset(at, 0, n)`.
    fn zero_bytes(mem: &mut impl Mem, at: u32, n: usize) {
        mem.write_bytes(at, &vec![0u8; n]);
    }

    /// The C string at `at` (without its zero).
    fn c_string(mem: &impl Mem, at: u32) -> Vec<u8> {
        let mut s = Vec::new();
        loop {
            let c: u8 = mem.read(at.wrapping_add(u32::try_from(s.len()).unwrap_or(u32::MAX)));
            if c == 0 {
                return s;
            }
            s.push(c);
        }
    }

    /// `strncpy(to, from, n)`: at most `n` bytes of the string, the rest of
    /// the `n` zero (no terminator when the string is `n` long or more).
    fn strncpy(mem: &mut impl Mem, to: u32, from: u32, n: u32) {
        let mut done = false;
        for k in 0..n {
            let c: u8 = if done {
                0
            } else {
                mem.read(from.wrapping_add(k))
            };
            done |= c == 0;
            mem.write(to.wrapping_add(k), c);
        }
    }

    /// The play time text, `sprintf("%003d:%02d:%02d")` of `seconds` as
    /// hours, minutes and seconds; from 1000 hours on `999:59:59`.
    #[must_use]
    pub fn pre_save_time(seconds: u32) -> String {
        let (mut h, mut m, mut s) = (seconds / 60 / 60, seconds / 60 % 60, seconds % 60);
        if h > 999 {
            (h, m, s) = (999, 59, 59);
        }
        format!("{h:03}:{m:02}:{s:02}")
    }

    /// `TkMenuPreSave()` (`0x008b50b0`): packs the save of slot
    /// [`MENU_SAVE_SLOT`] = 1 into [`SAVE_FILE_BUFFER`]
    /// (`MenuSave_PackSave(buffer, TkGetSaveEventAddress(slot), slot)`,
    /// play time = its result, location `AtelGetSaveDic()`) unless
    /// [`MENU_PRE_SAVE_PACKED`] (then the buffer's `+0x10` and
    /// [`MENU_PRE_SAVE_DIC`]); `MenuSave_BuildIconSys(slot, time, dic)`;
    /// [`SAVE_DATA_LOAD_RESULT`] = 1; then the PC save description:
    /// `SaveDataSetSaveDataInfo(title, subtitle, detail)` with title
    /// `SaveDataGetLoc(0x4f)` (64 bytes), subtitle `SaveDataGetLoc(0x50)`
    /// without its number tag, in UTF-8 (128), and detail `"%s %s\n%s %s"`
    /// of `SaveDataGetLoc(0x51)`, the location name
    /// (`AtelGetSaveDicName(dic or the buffer's u16 +0x18, 0)`),
    /// `SaveDataGetLoc(0x52)` and [`pre_save_time`]; returns
    /// `SaveDataUpdateIcon(dic)`. `local` is the `0x16cc`-byte frame.
    pub fn tk_menu_pre_save<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use pre_save as p;
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let at = |k: u32| local.wrapping_add(p::LEN).wrapping_sub(k);
        zero_bytes(mem, at(p::TITLE), 0x40);
        zero_bytes(mem, at(p::SUBTITLE), 0x80);
        zero_bytes(mem, at(p::DETAIL), 0x200);
        MENU_SAVE_SLOT.set(mem, 1);
        let buffer = SAVE_FILE_BUFFER.ptr(mem).addr;
        let (slot, time, dic) = if MENU_PRE_SAVE_PACKED.get(mem) == 0 {
            let f = fun(mem, 0x0086_c410);
            let dic = call(mem, f, &[]);
            let slot = MENU_SAVE_SLOT.get(mem);
            let f = fun(mem, 0x0088_c830);
            let event = call(mem, f, &[slot]);
            let f = fun(mem, 0x008b_3e60);
            let time = call(mem, f, &[buffer, event, slot]);
            (MENU_SAVE_SLOT.get(mem), time, dic)
        } else {
            let time = mem.read(buffer.wrapping_add(0x10));
            (1, time, MENU_PRE_SAVE_DIC.get(mem))
        };
        let f = fun(mem, 0x008b_42e0);
        call(mem, f, &[slot, time, dic]);
        SAVE_DATA_LOAD_RESULT.set(mem, 1);
        let get_loc = fun(mem, 0x0064_7f30);
        let to_utf8 = fun(mem, 0x0064_5f30);
        let (a, b, c) = (at(p::TEXT_A), at(p::TEXT_B), at(p::TEXT_C));
        zero_bytes(mem, a, 0x400);
        call(mem, get_loc, &[0x4f, a]);
        strncpy(mem, at(p::TITLE), a, 0x40);
        for t in [a, b, c] {
            zero_bytes(mem, t, 0x400);
        }
        call(mem, get_loc, &[0x50, a]);
        let f = fun(mem, 0x0064_8150);
        call(mem, f, &[a, b]);
        call(mem, to_utf8, &[b, c]);
        strncpy(mem, at(p::SUBTITLE), c, 0x80);
        let time_text = pre_save_time(time);
        zero_bytes(mem, b, 0x400);
        zero_bytes(mem, a, 0x400);
        for k in [p::LABEL, p::PLACE, p::PLACE_UTF8, p::TIME_LABEL, p::TIME] {
            zero_bytes(mem, at(k), 0x200);
        }
        call(mem, get_loc, &[0x51, b]);
        call(mem, to_utf8, &[b, a]);
        strncpy(mem, at(p::LABEL), a, 0x200);
        let id = if MENU_PRE_SAVE_PACKED.get(mem) == 0 {
            u32::from(mem.read::<u16>(buffer.wrapping_add(0x18)))
        } else {
            dic
        };
        let f = fun(mem, 0x0086_c430);
        let name = call(mem, f, &[id, 0]);
        strncpy(mem, at(p::PLACE), name, 0x200);
        call(mem, to_utf8, &[at(p::PLACE), at(p::PLACE_UTF8)]);
        call(mem, get_loc, &[0x52, b]);
        call(mem, to_utf8, &[b, a]);
        strncpy(mem, at(p::TIME_LABEL), a, 0x200);
        let mut time_bytes = time_text.into_bytes();
        time_bytes.push(0);
        mem.write_bytes(at(p::TIME), &time_bytes);
        let mut detail = c_string(mem, at(p::LABEL));
        detail.push(b' ');
        detail.extend(c_string(mem, at(p::PLACE_UTF8)));
        detail.push(b'\n');
        detail.extend(c_string(mem, at(p::TIME_LABEL)));
        detail.push(b' ');
        detail.extend(c_string(mem, at(p::TIME)));
        detail.push(0);
        mem.write_bytes(at(p::DETAIL), &detail);
        let f = fun(mem, 0x0064_86c0);
        call(mem, f, &[at(p::TITLE), at(p::SUBTITLE), at(p::DETAIL)]);
        let f = fun(mem, 0x0064_8920);
        call(mem, f, &[dic])
    }

    /// `MenuSave_PackSave(out, save, slot)` (`0x008b3e60`, own name; caller
    /// `TkMenuCtrlSaveLoad`): a save file at `out`, a `0x40`-byte header
    /// then [`SAVE_DATA_SIZE`] bytes copied from `save`
    /// (`TkSetEvSaveStatus` on the copy). The header: `+0` the counter
    /// ([`MENU_SAVE_COUNTER`] plus one, at most `0x80000000`); `+4` `slot`;
    /// `+5..+0xc` the party (`TkMenuGetPlayerFromIndex2`, padded with
    /// `0xff`; just the first character while the story point, `u16` data
    /// `+0xbec`, is 15 or less), then `+5` the data's byte `+0x3d58`;
    /// `+0xc` `TOGetFFXLang()`; past story point 15 `+6`/`+7` the bytes
    /// `+0x3d59`/`+0x3d5a`, and below 300 a 6 in `+5`/`+6` becomes 7;
    /// `+0x10` data `+0xbc`; `+0x14` `MsGetGIL()`; `+0x18` `AtelGetSaveDic()`;
    /// `+0x1c` `0x100`; `+0x1e` `AtelGetAlbhedRikku()`; `+0x1f`
    /// `MsGetSaveConfigEnglish()`; `+0x20` the name `MsGetSaveChrName(0)`;
    /// `+0x1a` and data `+0x64b4` the `TkCRC` of the data (taken with
    /// `+0x64b4` 0). Returns the header's `+0x10`.
    #[allow(clippy::cast_possible_truncation)] // byte and halfword results
    pub fn menu_save_pack_save<M: Mem>(
        mem: &mut M,
        out: u32,
        save: u32,
        slot: u8,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let data = out.wrapping_add(0x40);
        mem.write_bytes(out, &[0; 0x40]);
        let mut copy = vec![0u8; SAVE_DATA_SIZE as usize];
        mem.read_bytes(save, &mut copy);
        mem.write_bytes(data, &copy);
        tk_set_ev_save_status(mem, data);
        let counter = MENU_SAVE_COUNTER.get(mem).wrapping_add(1);
        MENU_SAVE_COUNTER.set(mem, counter);
        mem.write(out, counter.min(0x8000_0000));

        let story: u16 = mem.read(data.wrapping_add(0xbec));
        let past_start = story > 15;
        let early = story < 300;
        let mut party = MENU_PLAYER_LIST_MAX2.get(mem).cast_signed();
        if !past_start && party > 1 {
            party = 1;
        }
        let mut i = 0i32;
        while i < party {
            let chr = tk_menu_get_player_from_index2(mem, i);
            mem.write(out.wrapping_add(5).wrapping_add_signed(i), chr);
            i += 1;
        }
        // the rest of the seven party bytes, when fewer were written
        for k in i.max(0)..7 {
            mem.write(out.wrapping_add(5).wrapping_add_signed(k), 0xffu8);
        }

        let lang = call(mem, mem.rebase(0x008a_c2f0), &[]);
        mem.write(out.wrapping_add(0xc), lang as u8);
        let first: u8 = mem.read(data.wrapping_add(0x3d58));
        mem.write(out.wrapping_add(5), first);
        if past_start {
            let second: u8 = mem.read(data.wrapping_add(0x3d59));
            mem.write(out.wrapping_add(6), second);
            let third: u8 = mem.read(data.wrapping_add(0x3d5a));
            mem.write(out.wrapping_add(7), third);
            if early {
                for o in [5u32, 6] {
                    if mem.read::<u8>(out.wrapping_add(o)) == 6 {
                        mem.write(out.wrapping_add(o), 7u8);
                    }
                }
            }
        }
        mem.write(out.wrapping_add(4), slot);
        let v: u32 = mem.read(data.wrapping_add(0xbc));
        mem.write(out.wrapping_add(0x10), v);
        let gil = call(mem, mem.rebase(0x008a_98b0), &[]);
        mem.write(out.wrapping_add(0x14), gil);
        let dic = call(mem, mem.rebase(0x0086_c410), &[]);
        mem.write(out.wrapping_add(0x18), dic as u16);
        mem.write(out.wrapping_add(0x1c), 0x100u16);
        let rikku = call(mem, mem.rebase(0x0086_a7e0), &[]);
        mem.write(out.wrapping_add(0x1e), rikku as u8);
        let english = call(mem, mem.rebase(0x0078_51d0), &[]);
        mem.write(out.wrapping_add(0x1f), english as u8);
        let name = call(mem, mem.rebase(0x0078_4fb0), &[0]);
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(name.wrapping_add(k));
            mem.write(out.wrapping_add(0x20).wrapping_add(k), c);
            k = k.wrapping_add(1);
            if c == 0 {
                break;
            }
        }
        mem.write(data.wrapping_add(0x64b4), 0u32);
        #[allow(clippy::cast_possible_wrap)]
        let crc = tk_crc(mem, SAVE_DATA_SIZE as i32, data);
        mem.write(out.wrapping_add(0x1a), crc);
        mem.write(data.wrapping_add(0x64b4), u32::from(crc));
        mem.read(out.wrapping_add(0x10))
    }

    /// A rectangle placed at `(x, y)` plus remapped offsets: `[left, top,
    /// width, height]` as float words, the remaps in the original's order
    /// (`remapY2(h)`, `remapX2(w)`, `remapY2(dy) + y`, `remapX2(dx) + x`),
    /// each value rounded to a float where the original stores it.
    fn remapped_rect<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        [dx, dy, w, h]: [f32; 4],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
    ) -> [u32; 4] {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let round = |v: f64| (v as f32).to_bits();
        let height = round(remap(mem, true, h));
        let width = round(remap(mem, false, w));
        let top = round(remap(mem, true, dy) + f64::from(y));
        let left = round(remap(mem, false, dx) + f64::from(x));
        [left, top, width, height]
    }

    /// `TkMn2WeaponBuySellWindow(x, y, weapon, amount, owned)` (`0x008c2d00`,
    /// Fahrenheit's name): the weapon shop's amount window at `(x, y)`.
    /// `TODrawMenuPlateXYWHType(x, y, 560, 240, 10)` (sizes remapped), a
    /// `TODrawCrossBoxXYWHC2` gradient box (10, 10, 540 x 220, both colours
    /// `0x40ffffff`), a plate (10, 10, 540 x 60, type 5), the weapon's
    /// `DrawCrossMenuIconWeaponName2(weapon, x - 85, y + 10, 0)`, the sheet
    /// `0x008f9c20(10, 70, 540 x 160, 0)`, a plate (13, 73, 534 x 60, type
    /// 2), sheet cell 20 (`0x008f8bb0`, 65, 85, 430 x 36), and two
    /// `TkMn2DrawGillPlate`s (24, 137 and 24, 187; 520 x 32): `|amount|`
    /// and `owned - amount` (at most 999 999 999). Offsets and sizes go
    /// through the UI remaps as in [`remapped_rect`]. Returns the last
    /// call's `eax`.
    pub fn tk_mn2_weapon_buy_sell_window<M: Mem>(
        mem: &mut M,
        at: [f32; 2],
        weapon: u32,
        amount: i32,
        owned: i32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| (v as f32).to_bits();
        let [x, y] = at;
        let plate = mem.rebase(0x008f_5f70);
        let height = r(remap(mem, true, 240.0));
        let width = r(remap(mem, false, 560.0));
        call(mem, plate, &[x.to_bits(), y.to_bits(), width, height, 10]);

        let [left, top, width, height] = remapped_rect(mem, at, [10.0, 10.0, 540.0, 220.0], remap);
        let cross_box = mem.rebase(0x008f_4b20);
        call(
            mem,
            cross_box,
            &[left, top, width, height, 0x40ff_ffff, 0x40ff_ffff],
        );

        let [left, top, width, height] = remapped_rect(mem, at, [10.0, 10.0, 540.0, 60.0], remap);
        call(mem, plate, &[left, top, width, height, 5]);

        let top = r(remap(mem, true, 10.0) + f64::from(y));
        let left = r(f64::from(x) - remap(mem, false, 85.0));
        let name = mem.rebase(0x008e_6970);
        call(mem, name, &[weapon, left, top, 0]);

        let [left, top, width, height] = remapped_rect(mem, at, [10.0, 70.0, 540.0, 160.0], remap);
        let sheet = mem.rebase(0x008f_9c20);
        call(mem, sheet, &[left, top, width, height, 0]);

        let [left, top, width, height] = remapped_rect(mem, at, [13.0, 73.0, 534.0, 60.0], remap);
        call(mem, plate, &[left, top, width, height, 2]);

        let [left, top, width, height] = remapped_rect(mem, at, [65.0, 85.0, 430.0, 36.0], remap);
        let sheet_cell = mem.rebase(0x008f_8bb0);
        call(mem, sheet_cell, &[20, left, top, width, height]);

        let gil_plate = mem.rebase(0x008c_0a20);
        let count = amount.wrapping_abs().cast_unsigned();
        let [left, top, width, height] = remapped_rect(mem, at, [24.0, 137.0, 520.0, 32.0], remap);
        call(mem, gil_plate, &[left, top, width, height, count]);

        let left_over = owned.wrapping_sub(amount).min(999_999_999).cast_unsigned();
        let [left, top, width, height] = remapped_rect(mem, at, [24.0, 187.0, 520.0, 32.0], remap);
        call(mem, gil_plate, &[left, top, width, height, left_over])
    }

    /// `TkMn2BuySellWindow(x, y, item, price, owned_count, amount, gil)`
    /// (`0x008bf170`, Fahrenheit's name): the item shop's amount window at
    /// `(x, y)`. A plate (`TODrawMenuPlateXYWHType`, 560 x 316, type 10),
    /// a `TODrawCrossBoxXYWHC2` box (10, 10, 540 x 296, colours
    /// `0x40ffffff`), a plate (10, 10, 540 x 60, type 5), the item's
    /// `DrawCrossMenuIconItemName(item, x - 85, y + 10, 0)`, three
    /// `TOMkpShapeXYWHUV` shapes of sheet `0xc8` (112, 76 and 377, 76:
    /// 72 x 72, UVs 0.484/0.587/0.564/0.667; 244, 90: 73 x 44, UVs
    /// 0.718/0.0137/0.789/0.0566), `price` and `owned_count` as
    /// `ToMakeBtlEasyDigit2(n, x', y + 93, 0, 0.68, 0)` centred on 147 and
    /// 412 (`x'` = remapped `x` minus half of `ToGetBtlEasyDigitWidth2(n,
    /// &w, 0.68, 0)`'s width; `width` stands for the original's argument
    /// slot it writes), a plate (12, 148, 534 x 60, type 2), sheet cell 20
    /// (64, 160, 430 x 36), and two `TkMn2DrawGillPlate`s (22, 212 and 22,
    /// 263; 522 x 32): `|price * amount|` and `gil - price * amount` (at
    /// most 999 999 999). Offsets and sizes go through the UI remaps as in
    /// [`remapped_rect`]. Returns the last call's `eax`.
    pub fn tk_mn2_buy_sell_window<M: Mem>(
        mem: &mut M,
        at: [f32; 2],
        [item, price, owned_count, amount, gil]: [u32; 5],
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r = |v: f64| (v as f32).to_bits();
        let [x, y] = at;
        let total = price.cast_signed().wrapping_mul(amount.cast_signed());
        let plate = mem.rebase(0x008f_5f70);
        let height = r(remap(mem, true, 316.0));
        let w = r(remap(mem, false, 560.0));
        call(mem, plate, &[x.to_bits(), y.to_bits(), w, height, 10]);

        let [left, top, w, height] = remapped_rect(mem, at, [10.0, 10.0, 540.0, 296.0], remap);
        let cross_box = mem.rebase(0x008f_4b20);
        call(
            mem,
            cross_box,
            &[left, top, w, height, 0x40ff_ffff, 0x40ff_ffff],
        );

        let [left, top, w, height] = remapped_rect(mem, at, [10.0, 10.0, 540.0, 60.0], remap);
        call(mem, plate, &[left, top, w, height, 5]);

        let top = r(remap(mem, true, 10.0) + f64::from(y));
        let left = r(f64::from(x) - remap(mem, false, 85.0));
        let name = mem.rebase(0x008e_6700);
        call(mem, name, &[item, left, top, 0]);

        let shape = mem.rebase(0x0090_3bb0);
        let arrow_uv = [0.484_375f32, 0.586_914_06, 0.564_453_1, 0.666_992_2].map(f32::to_bits);
        for dx in [112.0, 377.0] {
            let rect = remapped_rect(mem, at, [dx, 76.0, 72.0, 72.0], remap);
            call(mem, shape, &[&[0xc8], &rect[..], &arrow_uv].concat());
        }
        let rect = remapped_rect(mem, at, [244.0, 90.0, 73.0, 44.0], remap);
        let uv = [0.717_773_44f32, 0.013_671_875, 0.789_062_5, 0.056_640_625].map(f32::to_bits);
        call(mem, shape, &[&[0xc8], &rect[..], &uv].concat());

        let digit_width = mem.rebase(0x0090_5230);
        let digits = mem.rebase(0x0090_5550);
        let scale = 0.68f32.to_bits();
        for (n, dx) in [(price, 147.0), (owned_count, 412.0)] {
            call(mem, digit_width, &[n, width, scale, 0]);
            let top = r(remap(mem, true, 93.0) + f64::from(y));
            let right = remap(mem, false, dx) + f64::from(x);
            let half = f64::from(mem.read::<f32>(width)) * 0.5;
            let left = r(right - half);
            call(mem, digits, &[n, left, top, 0, scale, 0]);
        }

        let [left, top, w, height] = remapped_rect(mem, at, [12.0, 148.0, 534.0, 60.0], remap);
        call(mem, plate, &[left, top, w, height, 2]);

        let [left, top, w, height] = remapped_rect(mem, at, [64.0, 160.0, 430.0, 36.0], remap);
        let sheet_cell = mem.rebase(0x008f_8bb0);
        call(mem, sheet_cell, &[20, left, top, w, height]);

        let gil_plate = mem.rebase(0x008c_0a20);
        let cost = total.wrapping_abs().cast_unsigned();
        let [left, top, w, height] = remapped_rect(mem, at, [22.0, 212.0, 522.0, 32.0], remap);
        call(mem, gil_plate, &[left, top, w, height, cost]);

        let left_over = gil
            .cast_signed()
            .wrapping_sub(total)
            .min(999_999_999)
            .cast_unsigned();
        let [left, top, w, height] = remapped_rect(mem, at, [22.0, 263.0, 522.0, 32.0], remap);
        call(mem, gil_plate, &[left, top, w, height, left_over])
    }

    /// The legend-ability tables of `TkSetLegendAbility`, by character
    /// (0..=17): four `u16` abilities per level, 8 bytes per level.
    pub const LEGEND_ABILITY_TABLES: [u32; 18] = [
        0x00c8_6d00,
        0x00c8_6d10,
        0x00c8_6d60,
        0x00c8_6d20,
        0x00c8_6d30,
        0x00c8_6d40,
        0x00c8_6d50,
        0x00c8_6d70,
        0x00c8_6d80,
        0x00c8_6d90,
        0x00c8_6da0,
        0x00c8_6db0,
        0x00c8_6dc0,
        0x00c8_6de0,
        0x00c8_6dd0,
        0x00c8_6df0,
        0x00c8_6e00,
        0x00c8_6e10,
    ];

    /// `TkSetLegendAbility(chr, level)` (`0x008c3170`, Fahrenheit's name):
    /// the four abilities of `chr`'s legend weapon (`TkFindLegendWeapon(chr)`,
    /// called for any `chr`) become those of [`LEGEND_ABILITY_TABLES`]`[chr]`
    /// at `level` (8 bytes per level, signed), the weapon's `u16` at
    /// `+0xe..+0x14`; then `MsSetSaveParamAll()` and 1. Returns 0 without a
    /// weapon or for `chr` above 17.
    pub fn tk_set_legend_ability<M: Mem>(
        mem: &mut M,
        chr: u32,
        level: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = usize::try_from(chr)
            .ok()
            .and_then(|i| LEGEND_ABILITY_TABLES.get(i))
            .map(|&t| mem.rebase(t).wrapping_add(level.wrapping_mul(8)));
        let find = mem.rebase(0x008c_30e0);
        let weapon = call(mem, find, &[chr]);
        let Some(abilities) = table else {
            return 0;
        };
        if weapon == 0 {
            return 0;
        }
        for k in 0..4u32 {
            let a: u16 = mem.read(abilities.wrapping_add(2 * k));
            mem.write(weapon.wrapping_add(0xe + 2 * k), a);
        }
        let save_all = mem.rebase(0x008c_30d0);
        call(mem, save_all, &[]);
        1
    }

    /// `Math_AngleStepMode(a, target, step, mode)` (`0x0088b610`, own name;
    /// caller `0x00869510`): mode 1 [`math_angle_step_to`] with `step`,
    /// mode 2 the same with `step * -1.0` (a float; a NaN keeps its sign),
    /// any other [`crate::math::angle_step`]; the result stored as a float
    /// and returned in `ST0`.
    #[must_use]
    pub fn math_angle_step_mode(a: f32, target: f32, step: f32, mode: i32) -> f32 {
        use ffx_bridge::x87::{load, mul, store};
        let result = match mode {
            1 => math_angle_step_to(a, target, step),
            2 => {
                let back = f32::from_bits(store(mul(load(step.to_bits()), -1.0)));
                math_angle_step_to(a, target, back)
            }
            _ => crate::math::angle_step(a, target, step),
        };
        f32::from_bits(store(load(result.to_bits())))
    }

    /// `toGetStatusWeaponMesPos()` (`0x008d54a0`, Fahrenheit's name;
    /// caller `0x008cf990`): where the status screen's weapon/armour
    /// captions end, `ftol(remapX2(1020) - w)` (at 53 bits, no float store
    /// between) with `w` the wider of the two captions'
    /// `ToGetBtlEasyFontWidth(TOMenuGetStatusMes(k), &w, 0, 0.78, 1.0)` for
    /// `k` 5 and 6 (the second when it is not narrower, or for a NaN).
    /// `widths` stands for the original's two locals the widths go to.
    pub fn to_get_status_weapon_mes_pos<M: Mem>(
        mem: &mut M,
        widths: [u32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let status_mes = mem.rebase(0x008b_ee70);
        let font_width = mem.rebase(0x0090_5290);
        let scale = 0.78f32.to_bits();
        let one = 1f32.to_bits();
        for (k, out) in [(5, widths[0]), (6, widths[1])] {
            let text = call(mem, status_mes, &[k, out, 0, scale, one]);
            call(mem, font_width, &[text, out, 0, scale, one]);
        }
        let first = ffx_bridge::x87::load(mem.read(widths[0]));
        let second = ffx_bridge::x87::load(mem.read(widths[1]));
        let wider = if second < first { first } else { second };
        let right = remap(mem, false, 1020.0);
        ffx_bridge::x87::ftol(mem, ffx_bridge::x87::sub(right, wider))
    }

    /// `Pad_ReadCurrDevice(a, b, work, pad)` (`0x008898a0`, own name;
    /// caller `0x00888f90`, the pad read): the PC input's buttons
    /// (`inputGetCurrDeviceButtonInfo`, `u16`) with opposite directions
    /// cancelling (`0xa000` both set drops them to `& 0x5fff`, then `0x5000`
    /// to `& 0xafff`), by the pad record's mode (byte `+1`, high nibble):
    /// 6 (analogue) fills `2 * low nibble - 3` bytes from `+3` with `0x80`
    /// (when more than 3), mode byte `0x42`, `work+0x98` (`u32`) 0; mode 2
    /// becomes `0x22`; except for mode 6 the buttons go to `work+0x9a` and
    /// `+0x98` (`u16`). `a` and `b` are unused. Returns `work` (left in
    /// `eax`).
    pub fn pad_read_curr_device<M: Mem>(
        mem: &mut M,
        work: u32,
        pad: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let input = mem.rebase(0x0063_0c80);
        #[allow(clippy::cast_possible_truncation)] // movzx of ax
        let mut buttons = call(mem, input, &[]) as u16;
        if buttons & 0xa000 == 0xa000 {
            buttons &= 0x5fff;
        }
        if buttons & 0x5000 == 0x5000 {
            buttons &= 0xafff;
        }
        let mode: u8 = mem.read(pad.wrapping_add(1));
        let stick_bytes = i32::from(mode & 0xf) * 2;
        match mode >> 4 {
            6 => {
                if stick_bytes > 3 {
                    for k in 0..(stick_bytes - 3).cast_unsigned() {
                        mem.write(pad.wrapping_add(3 + k), 0x80u8);
                    }
                }
                mem.write(pad.wrapping_add(1), 0x42u8);
                mem.write(work.wrapping_add(0x98), 0u32);
                return work;
            }
            2 => mem.write(pad.wrapping_add(1), 0x22u8),
            _ => {}
        }
        mem.write(work.wrapping_add(0x9a), buttons);
        mem.write(work.wrapping_add(0x98), buttons);
        work
    }

    /// `Pad_ResetWork(work)` (`0x00888970`, own name; callers `0x00888910`,
    /// `0x00889790`): byte `+0` `0xff`, `+4` and `+8` 0, `u16 +0x1c` 0,
    /// then (a tail call) `Pad_ResetAnalogAt0C(work)`, whose `eax` is
    /// returned.
    pub fn pad_reset_work<M: Mem>(
        mem: &mut M,
        work: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        mem.write(work, 0xffu8);
        mem.write(work.wrapping_add(4), 0u32);
        mem.write(work.wrapping_add(8), 0u32);
        mem.write(work.wrapping_add(0x1c), 0u16);
        let reset = mem.rebase(0x0088_9030);
        call(mem, reset, &[work])
    }

    /// `Pad_ClearAnalogWork(a, b)` (`0x008889f0`, own name; caller
    /// `0x00889360`): in `TkGetControlerWork(a, b)` the eight `u16` at
    /// `+0xa0` and the eight floats at `+0xb0` become 0 (interleaved).
    /// Returns the work `+ 0xd0` (the original's `eax` after its loop).
    pub fn pad_clear_analog_work<M: Mem>(
        mem: &mut M,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let get = mem.rebase(0x0088_8ec0);
        let work = call(mem, get, &[a, b]);
        for k in 0..8u32 {
            mem.write(work.wrapping_add(0xa0 + 2 * k), 0u16);
            mem.write(work.wrapping_add(0xb0 + 4 * k), 0u32);
        }
        work.wrapping_add(0xd0)
    }

    /// The menu text wrappers: `(address, text, help)`. Each original is
    /// `MsMenuGetText(text, id, MsGetSaveConfigHiragana())` (or
    /// `MsMenuGetHelp` for `help`) of its one argument; see
    /// [`menu_text_of`]. The `TOMenuGet*`, `TkMenu*` and `TkMn2*` families
    /// repeat the same texts.
    pub const MENU_TEXT_WRAPPERS: [(u32, u32, bool); 45] = [
        (0x0086_0780, 3, false),  // AtelMenuSaveMes_00860780
        (0x008b_ea50, 5, true),   // TOMenuGetArmsHelp
        (0x008b_ea70, 5, false),  // TOMenuGetArmsMes
        (0x008b_ea90, 6, true),   // TOMenuGetBuildHelp
        (0x008b_eab0, 6, false),  // TOMenuGetBuildMes
        (0x008b_ead0, 7, true),   // TOMenuGetConfigHelp
        (0x008b_eaf0, 7, false),  // TOMenuGetConfigMes
        (0x008b_ec90, 8, true),   // TOMenuGetItemHelp
        (0x008b_ecb0, 8, false),  // TOMenuGetItemMes
        (0x008b_ecd0, 9, true),   // TOMenuGetMainHelp
        (0x008b_ecf0, 9, false),  // TOMenuGetMainMes
        (0x008b_ed10, 1, true),   // TOMenuGetMenu1Help
        (0x008b_ed30, 1, false),  // TOMenuGetMenu1Mes
        (0x008b_ed50, 2, true),   // TOMenuGetMenu2Help
        (0x008b_ed70, 2, false),  // TOMenuGetMenu2Mes
        (0x008b_edb0, 10, true),  // TOMenuGetNameHelp
        (0x008b_edd0, 10, false), // TOMenuGetNameMes
        (0x008b_ee10, 3, true),   // TOMenuGetSaveHelp
        (0x008b_ee30, 3, false),  // TOMenuGetSaveMes
        (0x008b_ee50, 11, true),  // TOMenuGetStatusHelp
        (0x008b_ee70, 11, false), // TOMenuGetStatusMes
        (0x008b_ee90, 12, true),  // TOMenuGetSummonHelp
        (0x008b_eeb0, 12, false), // TOMenuGetSummonMes
        (0x008b_f150, 5, false),  // TkMn2ArmsMes
        (0x008d_c8f0, 5, true),   // TkMenuArmsHelp
        (0x008d_c910, 5, false),  // TkMenuArmsMes
        (0x008d_c930, 6, true),   // TkMenuBuildHelp
        (0x008d_c950, 6, false),  // TkMenuBuildMes
        (0x008d_c9b0, 7, false),  // TkMenuConfigMes
        (0x008d_df20, 8, true),   // TkMenuItemHelp
        (0x008d_df40, 8, false),  // TkMenuItemMes
        (0x008d_df60, 9, true),   // TkMenuMainHelp
        (0x008d_df80, 9, false),  // TkMenuMainMes
        (0x008d_dfa0, 1, true),   // TkMenuMenu1Help
        (0x008d_dfc0, 1, false),  // TkMenuMenu1Mes
        (0x008d_dfe0, 2, true),   // TkMenuMenu2Help
        (0x008d_e000, 2, false),  // TkMenuMenu2Mes
        (0x008d_e020, 10, true),  // TkMenuNameHelp
        (0x008d_e040, 10, false), // TkMenuNameMes
        (0x008d_e060, 3, true),   // TkMenuSaveHelp
        (0x008d_e080, 3, false),  // TkMenuSaveMes
        (0x008d_e120, 11, true),  // TkMenuStatusHelp
        (0x008d_e140, 11, false), // TkMenuStatusMes
        (0x008d_e160, 12, true),  // TkMenuSummonHelp
        (0x008d_e180, 12, false), // TkMenuSummonMes
    ];

    /// One of the [`MENU_TEXT_WRAPPERS`]: menu text `text`'s name (or, for
    /// `help`, help line) of record `id`, in kana when
    /// `MsGetSaveConfigHiragana()` says so.
    pub fn menu_text_of(mem: &impl Mem, text: u32, help: bool, id: u32) -> u32 {
        use ffx_battle::menu::bridge::{ms_menu_get_help, ms_menu_get_text};
        let kana = save::ms_get_save_config_hiragana(mem);
        if help {
            ms_menu_get_help(mem, text, id, kana)
        } else {
            ms_menu_get_text(mem, text, id, kana)
        }
    }

    /// `TkMenuDraw1612Width(text)` (`0x008dc9e0`, Fahrenheit's name): a
    /// tail jump to `TOGetTextWidth_008b8040(text)`; its result.
    pub fn tk_menu_draw1612_width<M: Mem>(
        mem: &mut M,
        text: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let width = mem.rebase(0x008b_8040);
        call(mem, width, &[text])
    }

    /// `TkMn2TransCharFace(summon)` (`0x008c2ce0`, Fahrenheit's name): the
    /// face texture upload, `TOMenuTransFaceSmnTex()` for a nonzero
    /// `summon`, else `TOMenuTransFacePlyTex()` (tail jumps); its result.
    pub fn tk_mn2_trans_char_face<M: Mem>(
        mem: &mut M,
        summon: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(if summon == 0 {
            0x0090_1100
        } else {
            0x0090_1130
        });
        call(mem, f, &[])
    }

    /// `MenuNameEntry_CommitDefault0b()` (`0x008c5370`, own name; callers
    /// `0x00787100`, `0x00787470`): the name entry's commit for character
    /// `0xb` without the menu: `MenuItem_SetTarget(0xb)`,
    /// `MenuItem_LoadTargetKeys(0xb)`, then (a tail jump)
    /// `MenuNameEntry_Commit()`, whose result is returned.
    pub fn menu_name_entry_commit_default0b<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let set_target = mem.rebase(0x008c_5440);
        call(mem, set_target, &[0xb]);
        let load_keys = mem.rebase(0x008c_5320);
        call(mem, load_keys, &[0xb]);
        let commit = mem.rebase(0x008c_4e60);
        call(mem, commit, &[])
    }

    /// `TkMn2DrawMesL(x, y, text, colour)` (`0x008c1060`, Fahrenheit's
    /// name): [`MN2_DRAW_Y`] moves `0x10` on and becomes the cursor of
    /// `TOAddPktBaseMesFontRGBA_008ed160(cursor, text, x, y, colour,
    /// shade...)` with the four [`MN2_FONT_SHADE`] words (from `+0x18` down
    /// to `+0`, 8 bytes apart), whose result is stored back and returned.
    pub fn tk_mn2_draw_mes_l<M: Mem>(
        mem: &mut M,
        [x, y, text, colour]: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let shade = MN2_FONT_SHADE.ptr(mem).addr;
        let [s0, s1, s2, s3] =
            [0x18u32, 0x10, 8, 0].map(|o| mem.read::<u32>(shade.wrapping_add(o)));
        let cursor = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        MN2_DRAW_Y.set(mem, cursor);
        let add = mem.rebase(0x008e_d160);
        let end = call(mem, add, &[cursor, text, x, y, colour, s0, s1, s2, s3]);
        MN2_DRAW_Y.set(mem, end);
        end
    }

    /// `debugSetRikkuName()` (`0x00860a60`, Fahrenheit's name; callers
    /// `0x00787100`, `0x00787470`, not debug-only): Rikku's default name,
    /// `MsMenuGetText(3, 0x202c, MsGetSaveConfigHiragana())`, copied with
    /// its NUL into `MsGetSaveChrName(6)`. Returns what the original leaves
    /// in `eax`: the name's address minus the text's.
    pub fn debug_set_rikku_name<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let text = menu_text_of(mem, 3, false, 0x202c);
        let chr_name = mem.rebase(0x0078_4fb0);
        let name = call(mem, chr_name, &[6]);
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(text.wrapping_add(k));
            mem.write(name.wrapping_add(k), c);
            k = k.wrapping_add(1);
            if c == 0 {
                break;
            }
        }
        name.wrapping_sub(text)
    }

    /// `MenuItem_RefreshHelp_008c7650(window)` (`0x008c7650`): the item list
    /// window's help line. The row is the window's list word at
    /// `+0x58 + 4 * k` (`k` = byte `+0x45` when not negative, at most
    /// `+0x42 - 1`) plus twice the `i16` at `+0x48`. With `+0x94` set, by
    /// kind `+0x43`: 0, an item (entry kind byte of [`MENU_ITEM_LIST`]
    /// nonzero) `TkMenuGetItemHelp(id)`; 1, a key item below
    /// `TkMenuImportantListMax()` with a nonzero id that
    /// `TkMsImportantGet(id)` owns, `TkMenuGetImportantHelp(id)`; 2, below
    /// `MenuItem_GetRows()`, kind 1 `TkMenuGetCommandHelp(id)`, kind 2
    /// `MsGetMenuChrInfo(id, 0)`; otherwise 0. When `+0x45` is not negative,
    /// `TkMenuSetHelpMessage(help)` (a tail jump), whose `eax` is returned;
    /// otherwise `None` (the original's `eax` is then whatever it last
    /// held, which no caller reads).
    pub fn menu_item_refresh_help<M: Mem>(
        mem: &mut M,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let at = |o: u32| window.wrapping_add(o);
        let selected = i32::from(mem.read::<i8>(at(0x45)));
        let lists = i32::from(mem.read::<u8>(at(0x42)));
        let k = if selected.max(0) >= lists {
            lists - 1
        } else {
            selected.max(0)
        };
        let base: u32 = mem.read(at(0x58).wrapping_add(k.cast_unsigned().wrapping_mul(4)));
        let cursor = i32::from(mem.read::<i16>(at(0x48))).cast_unsigned();
        let row = base.wrapping_add(cursor.wrapping_mul(2));
        let items = MENU_ITEM_LIST.ptr(mem).addr;
        let entry = items.wrapping_add(row.wrapping_mul(4));
        let id = |mem: &M| u32::from(mem.read::<u16>(entry));
        let entry_kind = |mem: &M| mem.read::<u8>(entry.wrapping_add(2));
        let mut help = 0;
        if mem.read::<u32>(at(0x94)) != 0 {
            match mem.read::<u8>(at(0x43)) {
                0 => {
                    if entry_kind(mem) != 0 {
                        let f = mem.rebase(0x008a_98f0);
                        help = call(mem, f, &[id(mem)]);
                    }
                }
                1 => {
                    let max = mem.rebase(0x008c_8a20);
                    let max = call(mem, max, &[]);
                    if row.cast_signed() < max.cast_signed() {
                        let list = MENU_IMPORTANT_LIST.ptr(mem).addr;
                        let key =
                            u32::from(mem.read::<u16>(list.wrapping_add(row.wrapping_mul(2))));
                        if key != 0 {
                            let owned = mem.rebase(0x008a_b2c0);
                            if call(mem, owned, &[key]) != 0 {
                                let f = mem.rebase(0x008d_dea0);
                                help = call(mem, f, &[key]);
                            }
                        }
                    }
                }
                2 => {
                    let rows = mem.rebase(0x008c_8a00);
                    let rows = call(mem, rows, &[]);
                    if row.cast_signed() < rows.cast_signed() {
                        match entry_kind(mem) {
                            1 => {
                                let f = mem.rebase(0x008d_de10);
                                help = call(mem, f, &[id(mem)]);
                            }
                            2 => {
                                let f = mem.rebase(0x0078_f7c0);
                                help = call(mem, f, &[id(mem), 0]);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if selected < 0 {
            return None;
        }
        let set = mem.rebase(0x008a_aeb0);
        Some(call(mem, set, &[help]))
    }

    /// `MenuSave_McSync(mode, cmd, result)` (`0x008884a0`, own name; caller
    /// `TkMenuCtrlSaveLoad`, three sites): a nonzero `mode` is passed on to
    /// the `sceMcSync` wrapper `0x008884e0(mode, cmd, result)` (a tail
    /// jump); mode 0 waits, calling it with mode 1 until it returns nonzero
    /// and adding 1 000 000 to the counter at `0x01330278`
    /// (`op_counter_add_00888890`) after every poll. Returns the wrapper's
    /// last result.
    pub fn menu_save_mc_sync<M: Mem>(
        mem: &mut M,
        [mode, cmd, result]: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let sync = mem.rebase(0x0088_84e0);
        if mode != 0 {
            return call(mem, sync, &[mode, cmd, result]);
        }
        let add = mem.rebase(0x0088_8890);
        loop {
            let done = call(mem, sync, &[1, cmd, result]);
            call(mem, add, &[1_000_000]);
            if done != 0 {
                return done;
            }
        }
    }

    /// The item shop amount windows' state machine (`MenuShop_BuyWindowCtrl`,
    /// `MenuShop_SellWindowCtrl`), by state `+0x28`: 0 becomes 1; 1 goes
    /// on as 2, 2 stores 3 and goes on as 3; in 3 a trigger `0x20` deals
    /// (state 4: `+0x45` 1, sound `0x8000000e`, `MsSaveItemUse(+0x5c, ±n)`,
    /// `MsPayGIL(±n * +0x60)`, minus when selling, state 5), `0x40` cancels
    /// (state 6: `+0x45` `0xff`, sound `0x80000004`, state 7); otherwise
    /// the repeat pad moves the amount `n` (`i16 +0x48`, from 1 to `most`,
    /// sound `0x80000001` on a move): `0x4000` −10 (at least 1), `0x1000`
    /// +10 (at most `most`), `0x8000` −1, `0x2000` +1. States 5, 7 and
    /// others do nothing.
    fn shop_amount_ctrl<M: Mem>(
        mem: &mut M,
        window: u32,
        [rep, trg]: [u32; 2],
        most: i32,
        sell: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| window.wrapping_add(o);
        let sign = |v: i32| if sell { v.wrapping_neg() } else { v };
        let sound = mem.rebase(0x0088_6e80);
        let amount = |mem: &M| mem.read::<i16>(at(0x48));
        let mut state = mem.read::<u32>(at(0x28));
        loop {
            match state {
                0 => {
                    mem.write(at(0x28), 1u32);
                    return;
                }
                1 => state = 2,
                2 | 3 => {
                    if state == 2 {
                        mem.write(at(0x28), 3u32);
                    }
                    if trg & 0x20 != 0 {
                        state = 4;
                    } else if trg & 0x40 != 0 {
                        state = 6;
                    } else {
                        let n = amount(mem);
                        if rep & 0x4000 != 0 {
                            if n > 1 {
                                call(mem, sound, &[0x8000_0001]);
                                let n = amount(mem).wrapping_sub(10);
                                mem.write(at(0x48), if n < 1 { 1 } else { n });
                            }
                        } else if rep & 0x1000 != 0 {
                            if i32::from(n) < most {
                                call(mem, sound, &[0x8000_0001]);
                                let n = amount(mem).wrapping_add(10);
                                #[allow(clippy::cast_possible_truncation)] // the low half
                                let n = if i32::from(n) > most { most as i16 } else { n };
                                mem.write(at(0x48), n);
                            }
                        } else if rep & 0x8000 != 0 {
                            if n > 1 {
                                call(mem, sound, &[0x8000_0001]);
                                let n = amount(mem).wrapping_sub(1);
                                mem.write(at(0x48), n);
                            }
                        } else if rep & 0x2000 != 0 && i32::from(n) < most {
                            call(mem, sound, &[0x8000_0001]);
                            let n = amount(mem).wrapping_add(1);
                            mem.write(at(0x48), n);
                        }
                        return;
                    }
                }
                4 => {
                    mem.write(at(0x45), 1u8);
                    call(mem, sound, &[0x8000_000e]);
                    let item: u32 = mem.read(at(0x5c));
                    let n = i32::from(amount(mem));
                    let item_use = mem.rebase(0x0079_0550);
                    call(mem, item_use, &[item, sign(n).cast_unsigned()]);
                    let n = i32::from(amount(mem));
                    let price = mem.read::<u32>(at(0x60)).cast_signed();
                    let pay = mem.rebase(0x0078_59a0);
                    call(mem, pay, &[sign(n.wrapping_mul(price)).cast_unsigned()]);
                    mem.write(at(0x28), 5u32);
                    return;
                }
                6 => {
                    mem.write(at(0x45), 0xffu8);
                    call(mem, sound, &[0x8000_0004]);
                    mem.write(at(0x28), 7u32);
                    return;
                }
                _ => return,
            }
        }
    }

    /// `TOMenuGetControlPad()` (dropped), then the repeat and trigger
    /// pads (`u16`).
    fn shop_pads<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> [u32; 2] {
        let pad = mem.rebase(0x008b_e410);
        call(mem, pad, &[]);
        let rep = mem.rebase(0x008b_e470);
        let rep = call(mem, rep, &[]) & 0xffff;
        let trg = mem.rebase(0x008b_e4b0);
        let trg = call(mem, trg, &[]) & 0xffff;
        [rep, trg]
    }

    /// `MenuShop_SellWindowCtrl(window)` (`0x008d9e20`, own name; the
    /// `+0xc` callback `MenuShop_OpenSellWindow` installs): the item shop
    /// sell window's input, [`shop_amount_ctrl`] selling, up to `+0x64`
    /// (the count owned). The caller never reads `eax`.
    pub fn menu_shop_sell_window_ctrl<M: Mem>(
        mem: &mut M,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let pads = shop_pads(mem, call);
        let most = mem.read::<u32>(window.wrapping_add(0x64)).cast_signed();
        shop_amount_ctrl(mem, window, pads, most, true, call);
    }

    /// `MenuShop_BuyWindowCtrl(window)` (`0x008d9bd0`, own name; the `+0xc`
    /// callback `MenuShop_OpenBuyWindow` installs): the item shop buy
    /// window's input, [`shop_amount_ctrl`] buying, up to what the gil
    /// pays for, `MsGetGIL() / +0x60` (signed), and at most 99 with the
    /// count owned (`+0x64`): above that `99 - owned`. A zero price (or
    /// `i32::MIN / -1`) faults the original; here it allows 0. The caller
    /// never reads `eax`.
    pub fn menu_shop_buy_window_ctrl<M: Mem>(
        mem: &mut M,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let price = mem.read::<u32>(window.wrapping_add(0x60)).cast_signed();
        let pads = shop_pads(mem, call);
        let gil = mem.rebase(0x0078_4e80);
        let gil = call(mem, gil, &[]).cast_signed();
        let mut most = gil.checked_div(price).unwrap_or(0);
        let owned = mem.read::<u32>(window.wrapping_add(0x64)).cast_signed();
        if owned.wrapping_add(most) > 99 {
            most = 99i32.wrapping_sub(owned);
        }
        shop_amount_ctrl(mem, window, pads, most, false, call);
    }

    /// The weapon shop's yes/no confirmation (`0x008d7690`/`0x008d7860`):
    /// `TkMenuOpenSubCmdWindow(TkMenuMainMes(mes), TkMenuMainMes(0x1c),
    /// TkMenuMainMes(0x1d))`, then on `TkMn2GetListMax2()` `+0x3e` 5,
    /// `+0x37` 0 and `+0x60 = ftol(graphicUiRemapY2(493) - 208)`.
    fn wshop_open_confirm<M: Mem>(
        mem: &mut M,
        mes: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let text = mem.rebase(0x008d_df80);
        let question = call(mem, text, &[mes]);
        let yes = call(mem, text, &[0x1c]);
        let no = call(mem, text, &[0x1d]);
        let open = mem.rebase(0x008e_33a0);
        call(mem, open, &[question, yes, no]);
        let list = mem.rebase(0x008e_3390);
        let w = call(mem, list, &[]);
        mem.write(w.wrapping_add(0x3e), 5u8);
        mem.write(w.wrapping_add(0x37), 0u8);
        let y = remap(mem, true, 493.0) - 208.0;
        let y = ffx_bridge::x87::ftol(mem, y);
        mem.write(w.wrapping_add(0x60), y);
    }

    /// The answer of the weapon shop's yes/no window (`TkMn2GetListMax2()`,
    /// `i8 +0x45`): `None` while it is open (0), otherwise the window is
    /// closed (`TOMenuCloseYesNoWindow`) and the next state is 4 for "yes"
    /// (the `+0x48` row 0) or 6 (cancel, or "no" with sound `0x80000001`).
    fn wshop_confirm_answer<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let list = mem.rebase(0x008e_3390);
        let w = call(mem, list, &[]);
        let answer = mem.read::<i8>(w.wrapping_add(0x45));
        let close = mem.rebase(0x008e_2de0);
        if answer < 0 {
            call(mem, close, &[]);
            return Some(6);
        }
        if answer == 0 {
            return None;
        }
        let row = mem.read::<i16>(w.wrapping_add(0x48));
        call(mem, close, &[]);
        if row == 0 {
            return Some(4);
        }
        let sound = mem.rebase(0x0088_6e80);
        call(mem, sound, &[0x8000_0001]);
        Some(6)
    }

    /// `TOMenuGetControlPad()`, `TOMenuGetControlPadRep()`,
    /// `TOMenuGetControlPadTrg()`, results dropped.
    fn wshop_pads<M: Mem>(mem: &mut M, call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32) {
        for f in [0x008b_e410, 0x008b_e470, 0x008b_e4b0] {
            let f = mem.rebase(f);
            call(mem, f, &[]);
        }
    }

    /// `MenuWShop_BuyConfirmCtrl(window)` (`0x008d7690`, own name): the
    /// weapon shop's "buy?" control. The pads and `MsGetGIL()` are read
    /// and dropped; then by state `+0x28`: 0 and 1 go on as 2 (0 stores 1
    /// first, overwritten by 2: left out); 2 opens the confirmation (text `0x1b`, see
    /// [`wshop_open_confirm`]), stores 3 and goes on as 3; 3 waits for the
    /// answer ([`wshop_confirm_answer`]) and goes on as 4 or 6; 4 buys:
    /// sound `0x8000000e`, `0x0186ab18 = 0x007ab910(+0x5c)` (the weapon
    /// given), `MsPayGIL(+0x60)`, `+0x45` 1, state 5; 6 cancels: `+0x45`
    /// `0xff`, state 7; 5, 7 and others do nothing. The caller never
    /// reads `eax`.
    pub fn menu_wshop_buy_confirm_ctrl<M: Mem>(
        mem: &mut M,
        window: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| window.wrapping_add(o);
        wshop_pads(mem, call);
        let gil = mem.rebase(0x0078_4e80);
        call(mem, gil, &[]);
        let mut state = mem.read::<u32>(at(0x28));
        loop {
            match state {
                // 0 also stores state 1, which state 2 overwrites at once
                0 | 1 => state = 2,
                2 | 3 => {
                    if state == 2 {
                        wshop_open_confirm(mem, 0x1b, remap, call);
                        mem.write(at(0x28), 3u32);
                    }
                    let Some(next) = wshop_confirm_answer(mem, call) else {
                        return;
                    };
                    state = next;
                }
                4 => {
                    let sound = mem.rebase(0x0088_6e80);
                    call(mem, sound, &[0x8000_000e]);
                    let give = mem.rebase(0x007a_b910);
                    let id = mem.read::<u32>(at(0x5c));
                    let got = call(mem, give, &[id]);
                    let price = mem.read::<u32>(at(0x60));
                    mem.write(mem.rebase(0x0186_ab18), got);
                    let pay = mem.rebase(0x0078_59a0);
                    call(mem, pay, &[price]);
                    mem.write(at(0x45), 1u8);
                    mem.write(at(0x28), 5u32);
                    return;
                }
                6 => {
                    mem.write(at(0x45), 0xffu8);
                    mem.write(at(0x28), 7u32);
                    return;
                }
                _ => return,
            }
        }
    }

    /// `MenuWShop_SellConfirmCtrl(window)` (`0x008d7860`, own name): the
    /// weapon shop's "sell?" control. The pads are read and dropped; then
    /// by state `+0x28`: 0 stores 1 (and stops); 1 goes on as 2; 2 copies
    /// the name of weapon `+0x6c` (`MsGetSaveWeapon(id, &name)`) to
    /// `0x0186ab20`, makes it macro 0 (`TkMn2SetMacroParam(0, 0, ..)`),
    /// opens the confirmation (text `0x29`, see [`wshop_open_confirm`]),
    /// stores 3 and goes on as 3; 3 waits for the answer
    /// ([`wshop_confirm_answer`]) and goes on as 4 or 6; 4 stores `+0x45` 1
    /// and state 5 (the sale is the caller's); 6 `+0x45` `0xff`, state 7;
    /// 5, 7 and others do nothing. `local` is a word for the name pointer
    /// (the original reuses its argument's slot, so an ID 0xff, which
    /// leaves it alone, copies the window's bytes). The caller never reads
    /// `eax`.
    pub fn menu_wshop_sell_confirm_ctrl<M: Mem>(
        mem: &mut M,
        window: u32,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| window.wrapping_add(o);
        wshop_pads(mem, call);
        let mut state = mem.read::<u32>(at(0x28));
        loop {
            match state {
                0 => {
                    mem.write(at(0x28), 1u32);
                    return;
                }
                1 => state = 2,
                2 | 3 => {
                    if state == 2 {
                        let weapon = mem.rebase(0x007a_bbd0);
                        let id = mem.read::<u32>(at(0x6c));
                        // the slot holds the window until the callee
                        // writes it (it does not for ID 0xff)
                        mem.write(local, window);
                        call(mem, weapon, &[id, local]);
                        let name = mem.read::<u32>(local);
                        let dst = mem.rebase(0x0186_ab20);
                        for i in 0.. {
                            let c = mem.read::<u8>(name.wrapping_add(i));
                            mem.write(dst.wrapping_add(i), c);
                            if c == 0 {
                                break;
                            }
                        }
                        let macro_param = mem.rebase(0x008c_2c60);
                        call(mem, macro_param, &[0, 0, dst]);
                        wshop_open_confirm(mem, 0x29, remap, call);
                        mem.write(at(0x28), 3u32);
                    }
                    let Some(next) = wshop_confirm_answer(mem, call) else {
                        return;
                    };
                    state = next;
                }
                4 => {
                    mem.write(at(0x45), 1u8);
                    mem.write(at(0x28), 5u32);
                    return;
                }
                6 => {
                    mem.write(at(0x45), 0xffu8);
                    mem.write(at(0x28), 7u32);
                    return;
                }
                _ => return,
            }
        }
    }

    /// `TkMn2DrawSelItemsHelp(window)` (`0x008d1a20`, Fahrenheit's name):
    /// the help line of the selected row `i16 +0x48` of
    /// `g_TkMenuItemList` (below `u16 +0x30`, signed). By the entry's kind
    /// (`u8 +2`, id `u16 +0`): 1, 4..=7 `TkMn2GetCommandHelp(id)`; 2
    /// `MsGetMenuChrInfo(id, 0)` (`TkMn2GetCommandHelp(0x30ff)` for id
    /// 15); 3 `TkMenuMenu1Help(id + 0x1033)`; 11..=16 `TkMn2GetAbilityHelp`;
    /// 18 `TkMn2GetItemHelp`; the id 0 of commands, abilities and items,
    /// other kinds and rows out of range give 0. Ends in
    /// `TkMenuSetHelpMessage(help)` (a tail jump; no caller reads `eax`).
    pub fn tk_mn2_draw_sel_items_help<M: Mem>(
        mem: &mut M,
        window: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let row = i32::from(mem.read::<i16>(window.wrapping_add(0x48)));
        let rows = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        let mut help = 0;
        if row < rows {
            let entry = MENU_ITEM_LIST
                .ptr(mem)
                .addr
                .wrapping_add(row.cast_unsigned().wrapping_mul(4));
            let id = u32::from(mem.read::<u16>(entry));
            let by_id = |mem: &mut M, f: u32, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
                if id == 0 {
                    0
                } else {
                    let f = mem.rebase(f);
                    call(mem, f, &[id])
                }
            };
            help = match mem.read::<u8>(entry.wrapping_add(2)) {
                1 | 4..=7 => by_id(mem, 0x008c_19e0, call),
                2 if id == 0xf => {
                    let f = mem.rebase(0x008c_19e0);
                    call(mem, f, &[0x30ff])
                }
                2 => {
                    let f = mem.rebase(0x0078_f7c0);
                    call(mem, f, &[id, 0])
                }
                3 => {
                    let f = mem.rebase(0x008d_dfa0);
                    call(mem, f, &[id + 0x1033])
                }
                11..=16 => by_id(mem, 0x008c_1960, call),
                18 => by_id(mem, 0x008c_1b30, call),
                _ => 0,
            };
        }
        let set = mem.rebase(0x008a_aeb0);
        call(mem, set, &[help]);
    }

    /// `HelpMem_Get(reset, size)` (`0x00774c00`, own name): a block of the
    /// help screen's work memory. [`HELP_MEM_USED`] is cleared when
    /// `reset` is 0; the block is `TOMenuGetHelpWorkAdrs()` plus that,
    /// and the use grows by `size` rounded up to 64 (signed). A
    /// `dbgPrintf` (a `ret` in this build) and, at or over
    /// `TOMenuGetHelpWorkSize()`, `yiAssert(0, 0x00b5313c, 0x00b533d0,
    /// used, size)`. Returns the block.
    pub fn help_mem_get<M: Mem>(
        mem: &mut M,
        reset: u32,
        size: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008b_ec80);
        let limit = call(mem, f, &[]);
        if reset == 0 {
            HELP_MEM_USED.set(mem, 0);
        }
        let f = mem.rebase(0x008b_ec70);
        let base = call(mem, f, &[]);
        let used = HELP_MEM_USED.get(mem);
        let block = used.wrapping_add(base);
        let up = size.wrapping_add(0x3f);
        let up = (up.wrapping_add((up >> 31) & 0x3f) & !0x3f).cast_unsigned();
        let used = used.wrapping_add(up);
        HELP_MEM_USED.set(mem, used);
        let f = mem.rebase(0x0062_f500);
        let fmt = mem.rebase(0x00b5_33b0);
        call(mem, f, &[fmt, block, used, limit]);
        let used = HELP_MEM_USED.get(mem);
        if used.cast_signed() >= limit.cast_signed() {
            let f = mem.rebase(0x0090_ae50);
            let (file, msg) = (mem.rebase(0x00b5_313c), mem.rebase(0x00b5_33d0));
            call(mem, f, &[0, file, msg, used, limit]);
        }
        block
    }

    /// `TkMenuExchangePlayerPos(a, b)` (`0x008a9720`, Fahrenheit's name):
    /// swaps the menu's party slots `a` and `b` ([`MENU_PLAYERS`]) and the
    /// save's party behind them. For `b` at or past
    /// [`MENU_PLAYER_PARTY_MASK`] (signed, the reserve) the character in
    /// `b` is looked up in `MsGetSaveOutParty(&n)`; found at `j`, the
    /// save's slot is `j + 3`, not found nothing is swapped. Otherwise
    /// the slot is `b` (`MsGetSaveInParty(&n)` is called, its list
    /// unused), nothing for a negative `b`. The swap is
    /// `MsExchangeSaveParty(players[a], slot)` and the two bytes; always
    /// `MenuParty_RefreshMask()` at the end (no caller reads `eax`).
    /// `local` is a word for the count (the original uses its argument
    /// slot).
    pub fn tk_menu_exchange_player_pos<M: Mem>(
        mem: &mut M,
        slot_a: i32,
        slot_b: i32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let players = MENU_PLAYERS.ptr(mem).addr;
        let at = |i: i32| players.wrapping_add(i.cast_unsigned());
        let first = u32::from(mem.read::<u8>(at(slot_a)));
        mem.write(local, slot_b);
        let slot = if slot_b >= MENU_PLAYER_PARTY_MASK.get(mem).cast_signed() {
            let second = mem.read::<u8>(at(slot_b));
            let f = mem.rebase(0x0078_52d0);
            let list = call(mem, f, &[local]);
            let n = mem.read::<u32>(local).cast_signed();
            (0..n)
                .find(|&j| mem.read::<u8>(list.wrapping_add(j.cast_unsigned())) == second)
                .map(|j| j + 3)
        } else {
            let f = mem.rebase(0x0078_5270);
            call(mem, f, &[local]);
            Some(slot_b).filter(|&b| b >= 0)
        };
        if let Some(slot) = slot {
            let f = mem.rebase(0x0078_4d00);
            call(mem, f, &[first, slot.cast_unsigned()]);
            let x = mem.read::<u8>(at(slot_a));
            let y = mem.read::<u8>(at(slot_b));
            mem.write(at(slot_a), y);
            mem.write(at(slot_b), x);
        }
        let f = mem.rebase(0x008a_ae30);
        call(mem, f, &[]);
    }

    /// `MenuItem_OpenTargetWindow(aeons, a, b)` (`0x008c9240`, own name):
    /// the item/ability target window, `TkMenuMainAllocWindow()` kept in
    /// [`ITEM_WINDOW2`]. Its width `+0x68` is the widest
    /// `TkMenuDraw1612Width(TOGetSaveChrName(..))` of the party
    /// (`aeons` 0: `TkMenuGetPlayerFromIndex2`) or the aeons
    /// (`TkMenuGetSummonFromIndex`), at least `0x77`. Handlers `+8`
    /// `0x008c6910`, `+0xc` `0x008c7430`, `+0x10` `0x008c80b0`, `+0x14`
    /// `0x008c93f0`, `+0x1c` `0x008c6a50`; `+0x3e` 2, `+0x58` `aeons`,
    /// `+0x5c` `a`, `+0x60` `b`, `u16 +0x54` `0x155`, `u16 +0x48` the
    /// cursor `u8 0x0186a3ac[aeons]`. Returns
    /// `TkMenuMainRegistWindow(window)`.
    pub fn menu_item_open_target_window<M: Mem>(
        mem: &mut M,
        aeons: u32,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_a1a0);
        let window = call(mem, f, &[]);
        ITEM_WINDOW2.set(mem, window);
        let (count, get) = if aeons == 0 {
            (0x008a_9b30, 0x008a_9ad0)
        } else {
            (0x008a_9bf0, 0x008a_9bc0)
        };
        let f = mem.rebase(count);
        let n = call(mem, f, &[]).cast_signed();
        let mut width = 0x77i32;
        for i in 0..n {
            let f = mem.rebase(get);
            let chr = call(mem, f, &[i.cast_unsigned()]);
            let f = mem.rebase(0x008a_c850);
            let name = call(mem, f, &[chr]);
            let f = mem.rebase(0x008d_c9e0);
            width = width.max(call(mem, f, &[name]).cast_signed());
        }
        let at = |o: u32| window.wrapping_add(o);
        for (o, f) in [
            (8, 0x008c_6910),
            (0xc, 0x008c_7430),
            (0x10, 0x008c_80b0),
            (0x14, 0x008c_93f0),
            (0x1c, 0x008c_6a50),
        ] {
            let handler = mem.rebase(f);
            mem.write(at(o), handler);
        }
        mem.write(at(0x3e), 2u8);
        mem.write(at(0x5c), a);
        mem.write(at(0x60), b);
        mem.write(at(0x58), aeons);
        mem.write(at(0x54), 0x155u16);
        let cursor = mem.rebase(0x0186_a3ac).wrapping_add(aeons);
        let cursor = u16::from(mem.read::<u8>(cursor));
        mem.write(at(0x48), cursor);
        mem.write(at(0x68), width);
        let f = mem.rebase(0x008a_ab00);
        call(mem, f, &[window])
    }

    /// `TkMn2ItemRequireWindow(x, y, item, need)` (`0x008c1c90`, Fahrenheit's
    /// name; `x`, `y` integers): the "items needed" box at `(x, y)`.
    /// `TODrawMenuPlateXYWHType(x, y, 740, 134, 5)` (sizes remapped), a
    /// divider shape (`-1`, 20, 52, 700 x 2, UV 1631/567..1829/570) and
    /// three sheet shapes (`200`): arrows at 180 and 490 (56, 75 x 75) and
    /// the slash at 337 (68, 73 x 44). For an `item >= 0`: `n =
    /// MsGetSaveItemNum(item)`, short = `n < need` (signed, also the
    /// colour), `DrawCrossMenuIconItemName(item, remapX2(0) + x, y -
    /// remapY2(1), short)`, `n` in digits (scale 0.68) centred on 218;
    /// then, short, `TkMenuMainMes(0x15)` in the small font (0.78)
    /// centred on 528 (at 74), else `n - need` in digits centred on 528.
    /// Centring subtracts half the width `ToGetBtlEasyDigitWidth2` /
    /// `ToGetBtlEasyFontWidth` write to `width` (a scratch word; the
    /// original's local) in double precision before the store. Returns
    /// what the original leaves in `eax`: `item` when negative, else the
    /// last draw call's result.
    pub fn tk_mn2_item_require_window<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        item: u32,
        need: i32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let round = |v: f64| (v as f32).to_bits();
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let at = [x as f32, y as f32];
        let plate = mem.rebase(0x008f_5f70);
        let height = round(remap(mem, true, 134.0));
        let width_px = round(remap(mem, false, 740.0));
        call(
            mem,
            plate,
            &[at[0].to_bits(), at[1].to_bits(), width_px, height, 5],
        );

        let shape = mem.rebase(0x0090_3bb0);
        let rect = remapped_rect(mem, at, [20.0, 52.0, 700.0, 2.0], remap);
        let uv = [1631.0f32, 567.0, 1829.0, 570.0].map(f32::to_bits);
        call(mem, shape, &[&[u32::MAX], &rect[..], &uv].concat());
        let arrow_uv = [0.486_328_13f32, 0.588_867_2, 0.559_570_3, 0.662_109_4].map(f32::to_bits);
        let slash_uv =
            [0.717_773_44f32, 0.013_671_875, 0.789_062_5, 0.056_640_625].map(f32::to_bits);
        for (rect, uv) in [
            ([180.0, 56.0, 75.0, 75.0], arrow_uv),
            ([337.0, 68.0, 73.0, 44.0], slash_uv),
            ([490.0, 56.0, 75.0, 75.0], arrow_uv),
        ] {
            let rect = remapped_rect(mem, at, rect, remap);
            call(mem, shape, &[&[200], &rect[..], &uv].concat());
        }
        if item.cast_signed() < 0 {
            return item;
        }

        let count = mem.rebase(0x0079_04b0);
        let owned = call(mem, count, &[item]).cast_signed();
        let short = u32::from(owned < need);
        let top = round(f64::from(at[1]) - remap(mem, true, 1.0));
        let left = round(remap(mem, false, 0.0) + f64::from(at[0]));
        let name = mem.rebase(0x008e_6700);
        call(mem, name, &[item, left, top, short]);

        let digit_width = mem.rebase(0x0090_5230);
        let digits = mem.rebase(0x0090_5550);
        let one = 1.0f32.to_bits();
        let scale = 0.68f32.to_bits();
        // `ToGetBtlEasy*Width` wrote `width`: centred on `remapX2(dx) + x`
        let centred =
            |mem: &mut M, dy: f32, dx: f32, remap: &mut dyn FnMut(&mut M, bool, f32) -> f64| {
                let top = round(remap(mem, true, dy) + f64::from(at[1]));
                let right = remap(mem, false, dx) + f64::from(at[0]);
                let half = f64::from(mem.read::<f32>(width)) * 0.5;
                [round(right - half), top]
            };
        call(
            mem,
            digit_width,
            &[owned.cast_unsigned(), width, scale, one],
        );
        let [left, top] = centred(mem, 75.0, 218.0, remap);
        call(
            mem,
            digits,
            &[owned.cast_unsigned(), left, top, short, scale, one],
        );
        let text = mem.rebase(0x008d_df80);
        if short == 1 {
            let small = 0.78f32.to_bits();
            let mes = call(mem, text, &[0x15]);
            let font_width = mem.rebase(0x0090_5290);
            call(mem, font_width, &[mes, width, short, small]);
            let [left, top] = centred(mem, 74.0, 528.0, remap);
            let mes = call(mem, text, &[0x15]);
            let font = mem.rebase(0x0090_5ab0);
            return call(mem, font, &[mes, left, top, short, small]);
        }
        let more = owned.wrapping_sub(need).cast_unsigned();
        call(mem, digit_width, &[more, width, scale, one]);
        let [left, top] = centred(mem, 75.0, 528.0, remap);
        call(mem, digits, &[more, left, top, short, scale, one])
    }

    /// `MenuSave_KindText(kind, base)` (`0x008b5560`, own name; caller
    /// `0x008b4b70`): `base + 0x20` for kind 1, else a
    /// `TkMenuSaveMes` text: `0x2019` for 2, `0x2014` for 0 and 4,
    /// `0x202a` for the rest.
    pub fn menu_save_kind_text<M: Mem>(
        mem: &mut M,
        kind: u32,
        base: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = match kind {
            1 => return base.wrapping_add(0x20),
            2 => 0x2019,
            0 | 4 => 0x2014,
            _ => 0x202a,
        };
        let f = mem.rebase(0x008d_e080);
        call(mem, f, &[id])
    }

    /// `MenuWShop_EquipBought(id)` (`0x008d9120`, own name; caller
    /// `0x008d6e40`): equips equipment `id` on its owner. Its slot
    /// (`MsGetSaveWeapon(id, &name)`, the name unused): owner `u8 +4`,
    /// armour flag `u8 +5`; `TkMenuGetArmor(owner)` or `TkMenuGetWeapon`
    /// (result unused), `MsEquipSaveWeapon(owner, armour, id)`, and
    /// `TkMsSetSaveParam(owner)`, whose result it returns. `local` is the
    /// name's word.
    pub fn menu_wshop_equip_bought<M: Mem>(
        mem: &mut M,
        id: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x007a_bbd0);
        let slot = call(mem, f, &[id, local]);
        let armour = u32::from(mem.read::<u8>(slot.wrapping_add(5)));
        let owner = u32::from(mem.read::<u8>(slot.wrapping_add(4)));
        let get = if armour == 0 {
            0x008a_9c70
        } else {
            0x008a_9820
        };
        let f = mem.rebase(get);
        call(mem, f, &[owner]);
        let f = mem.rebase(0x007a_b970);
        call(mem, f, &[owner, armour, id]);
        let f = mem.rebase(0x008c_3070);
        call(mem, f, &[owner])
    }

    /// `MenuDrawChrNameOrCommand(chr, x, y, colour)` (`0x008c0ec0`, own
    /// name; caller `0x008d2790`): for `chr` 100 `TkMn2DrawMesL(x, y,
    /// TkMn2GetCommandName(0x30ff), colour)`; otherwise
    /// [`MN2_DRAW_Y`] `+= 16` and becomes `TOAddPktChrNameIcon(it, chr,
    /// x, y, colour)`. Returns the callee's result.
    pub fn menu_draw_chr_name_or_command<M: Mem>(
        mem: &mut M,
        [chr, x, y, colour]: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if chr == 100 {
            let f = mem.rebase(0x008c_1a20);
            let text = call(mem, f, &[0x30ff]);
            let f = mem.rebase(0x008c_1060);
            return call(mem, f, &[x, y, text, colour]);
        }
        let cursor = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        MN2_DRAW_Y.set(mem, cursor);
        let f = mem.rebase(0x008e_ba80);
        let cursor = call(mem, f, &[cursor, chr, x, y, colour]);
        MN2_DRAW_Y.set(mem, cursor);
        cursor
    }

    /// `TkMenuDrawLayer(layer)` (`0x008a9690`, own name; caller
    /// `0x008aa290`): nothing when `0x0134085c` is 1 and `layer >= 6`
    /// (signed). Otherwise `TkVU1SyncPath()`, `TOMenuOpenPktBuffTmp()`,
    /// `TODrawScissorXYWH(0, 0, 512, 416)`, `TOSeqDrawLayer(1, layer)`,
    /// `TOMenuDrawKickTmp()`, `TkVU1SyncPath()`, then the draw callback
    /// `+0x10` of each of the 16 [`MENU_WINDOWS`] whose layer `u8 +0x3e`
    /// is `layer`. No caller reads `eax`.
    pub fn tk_menu_draw_layer<M: Mem>(
        mem: &mut M,
        layer: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u32>(mem.rebase(0x0134_085c)) == 1 && layer >= 6 {
            return;
        }
        let sync = mem.rebase(0x0088_ec60);
        call(mem, sync, &[]);
        let f = mem.rebase(0x008b_ef30);
        call(mem, f, &[]);
        let f = mem.rebase(0x008f_9230);
        call(mem, f, &[0, 0, 0x200, 0x1a0]);
        let f = mem.rebase(0x008a_e340);
        call(mem, f, &[1, layer.cast_unsigned()]);
        let f = mem.rebase(0x008b_ea20);
        call(mem, f, &[]);
        call(mem, sync, &[]);
        let windows = MENU_WINDOWS.ptr(mem).addr;
        for i in 0..16u32 {
            let w = windows.wrapping_add(i * 0x98);
            if u32::from(mem.read::<u8>(w.wrapping_add(0x3e))) != layer.cast_unsigned() {
                continue;
            }
            let draw = mem.read::<u32>(w.wrapping_add(0x10));
            if draw != 0 {
                call(mem, draw, &[w]);
            }
        }
    }

    /// `TkMenuCloseAll()` (`0x008aa200`, own name; caller `0x008a9ca0`):
    /// `TOMenuCloseMainWindows(0)`; each of the 25 [`MENU_SLOTS`] menus
    /// whose bit is clear in the word after [`MENU_MAIN_ACTIVE_MASK`]
    /// (`0x018408f0`, read each time) and whose `+0x18` or `+0x1a` byte is
    /// set gets its callback `+0`; then `TOMenuCloseMainWindows(1..=8)`
    /// and `TOSeqCtrl(1)`, whose result it returns.
    pub fn tk_menu_close_all<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let close = mem.rebase(0x008a_9230);
        call(mem, close, &[0]);
        let slots = MENU_SLOTS.ptr(mem).addr;
        let mask = MENU_MAIN_ACTIVE_MASK.ptr(mem).addr.wrapping_add(4);
        for i in 0..25u32 {
            let menu = mem.read::<u32>(slots.wrapping_add(i * 4));
            if menu == 0 || mem.read::<u32>(mask) & (1 << i) != 0 {
                continue;
            }
            if mem.read::<u8>(menu.wrapping_add(0x18)) == 0
                && mem.read::<u8>(menu.wrapping_add(0x1a)) == 0
            {
                continue;
            }
            let f = mem.read::<u32>(menu);
            if f != 0 {
                call(mem, f, &[menu]);
            }
        }
        for owner in 1..=8 {
            call(mem, close, &[owner]);
        }
        let f = mem.rebase(0x008a_e0f0);
        call(mem, f, &[1])
    }

    /// `MenuSaveLoad_SyncToggle()` (`0x008b4da0`, own name; caller
    /// `TkMenuCtrlSaveLoad`): unless the second toggle byte
    /// ([`MENU_SAVE_LOAD_TOGGLES`] `[1]`) is 2, returns 1. Else
    /// `MenuSave_McSync(1, 0, &result)`: 1 (done) sets the byte
    /// `0x01866268 + (toggle[0] != 0)` to 1 when `result` is 0 and the
    /// word `0x0186626c` is 2, else to 0; flips `toggle[0]` (to
    /// `toggle[0] == 0`), clears `toggle[1]` and returns 1. Other results
    /// return 1 when negative, else 0. `local` is `result`'s word.
    pub fn menu_save_load_sync_toggle<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let toggles = MENU_SAVE_LOAD_TOGGLES.ptr(mem).addr;
        let first = u32::from(mem.read::<u8>(toggles) != 0);
        if mem.read::<u8>(toggles.wrapping_add(1)) != 2 {
            return 1;
        }
        let f = mem.rebase(0x0088_84a0);
        let r = call(mem, f, &[1, 0, local]).cast_signed();
        if r != 1 {
            return u32::from(r < 0);
        }
        let done = mem.read::<u32>(local) == 0 && mem.read::<u32>(toggles.wrapping_add(2)) == 2;
        mem.write(toggles.wrapping_sub(2).wrapping_add(first), u8::from(done));
        let flipped = u8::from(mem.read::<u8>(toggles) == 0);
        mem.write(toggles, flipped);
        mem.write(toggles.wrapping_add(1), 0u8);
        1
    }

    /// `MenuSummon_GrowStat(slot, chr)` (`0x008cd240`, own name; caller
    /// `TkMenuCtrlSummon`): raises aeon `chr`'s stat `slot` (below 10,
    /// signed) with its sphere. `p = MsGetSavePlayerPtr(chr)`, `entry =
    /// MenuSummon_GrowEntry(slot)`, the descriptor `desc = 0x00c5a608 +
    /// 0x1c * slot`; the spheres used are `Math_Clamp1To99(gain(chr,
    /// MenuSummon_GetStat(p, desc, 1)))`, `gain` the function at `desc +
    /// 0x18`, taken with `MsSaveItemUse(u16 entry+4, -used)`; the stat
    /// becomes `MenuSummon_GetStat(p, desc, 0) + u8 entry+6`, at most
    /// `desc + 0x14` (unsigned), through `MenuSummon_SetStat(p, desc, 0,
    /// ..)`. Always ends with `TkMsSetSaveParam(chr)`, whose result it
    /// returns.
    pub fn menu_summon_grow_stat<M: Mem>(
        mem: &mut M,
        slot: i32,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0078_5330);
        let player = call(mem, f, &[chr]);
        if slot < 10 {
            let f = mem.rebase(0x008c_ecf0);
            let entry = call(mem, f, &[slot.cast_unsigned()]);
            let desc = mem
                .rebase(0x00c5_a608)
                .wrapping_add(slot.cast_unsigned().wrapping_mul(0x1c));
            let get = mem.rebase(0x008c_eca0);
            let current = call(mem, get, &[player, desc, 1]);
            let gain = mem.read::<u32>(desc.wrapping_add(0x18));
            let gain = call(mem, gain, &[chr, current]);
            let f = mem.rebase(0x008c_ed10);
            let used = call(mem, f, &[gain]);
            let item = u32::from(mem.read::<u16>(entry.wrapping_add(4)));
            let f = mem.rebase(0x0079_0550);
            call(mem, f, &[item, used.wrapping_neg()]);
            let stat = call(mem, get, &[player, desc, 0])
                .wrapping_add(u32::from(mem.read::<u8>(entry.wrapping_add(6))));
            let most = mem.read::<u32>(desc.wrapping_add(0x14));
            let f = mem.rebase(0x008c_ee70);
            call(mem, f, &[player, desc, 0, stat.min(most)]);
        }
        let f = mem.rebase(0x008c_3070);
        call(mem, f, &[chr])
    }

    /// `TkInitStartMap()` (`0x0088e880`, own name; caller `TkInit`): with
    /// `g_TkTGS` set and `g_GsDefDrawEnv2` clear, three `dbgPrintf`
    /// lines (a `ret` in this build) and `file_offset_get(&names)` on the
    /// three `.rdata` paths `0x00b5ea00`/`18`/`30` (`local`, 3 words);
    /// then `SetGlobal_01128ac4(0)`, `sa_cd_callback_init(0)`, the start
    /// map events (`g_TkStartMapEvent`, `g_TkLocalStartMapEvent`) both
    /// `0x17, 0`, and the word `0x023cbec0` 0. No caller reads `eax`.
    pub fn tk_init_start_map<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u8>(mem.rebase(0x0133_0832)) != 0 {
            let names = [0x00b5_ea00, 0x00b5_ea18, 0x00b5_ea30].map(|a| mem.rebase(a));
            for (k, name) in names.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)] // 3 words
                mem.write(local.wrapping_add(4 * k as u32), name);
            }
            if mem.read::<u8>(mem.rebase(0x00b5_ec4f)) == 0 {
                let print = mem.rebase(0x0062_f500);
                for (fmt, name) in [0x00b5_ea48, 0x00b5_ea58, 0x00b5_ea68]
                    .into_iter()
                    .zip(names)
                {
                    let fmt = mem.rebase(fmt);
                    call(mem, print, &[fmt, name]);
                }
                let f = mem.rebase(0x0076_db40);
                call(mem, f, &[local]);
            }
        }
        let f = mem.rebase(0x0076_cfc0);
        call(mem, f, &[0]);
        let f = mem.rebase(0x0076_f100);
        call(mem, f, &[0]);
        for (at, v) in [
            (0x0133_085c, 0x17u32),
            (0x0133_0860, 0),
            (0x0133_0864, 0x17),
            (0x0133_0868, 0),
            (0x023c_bec0, 0),
        ] {
            let at = mem.rebase(at);
            mem.write(at, v);
        }
    }

    /// `MenuSaveLoad_OpenWindow(mode)` (`0x008b5000`, own name; caller
    /// `TkMenuCtrlSaveLoad`): `TkMenuMainAllocWindow()` kept in
    /// `g_TkMenuSaveLoadWindow` (`0x01866254`); help
    /// `TkMenuSaveMes(0x2004)` (byte `0x0186628a` set) or `0x201f`;
    /// `u16 +0x48` 0, `u16 +0x3a` 5, handlers `+8` `0x008b3d10`, `+0xc`
    /// `0x008b44b0`, `+0x10` `0x008b4a50`, `+0x14` `0x008b55d0`, `+0x1c`
    /// `0x008b3730`, `+0x30` 99, `+0x42` 1, `+0x3e` 1, `+0x58` `mode`;
    /// registered, then `TOSeqStartScene(1, ..)` for the three scene
    /// names at `0x00b62fcc`/`d4`/`dc`. Returns the window.
    pub fn menu_save_load_open_window<M: Mem>(
        mem: &mut M,
        mode: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008a_a1a0);
        let window = call(mem, f, &[]);
        mem.write(mem.rebase(0x0186_6254), window);
        let id = if mem.read::<u8>(mem.rebase(0x0186_628a)) != 0 {
            0x2004
        } else {
            0x201f
        };
        let f = mem.rebase(0x008d_e080);
        let help = call(mem, f, &[id]);
        let f = mem.rebase(0x008a_aeb0);
        call(mem, f, &[help]);
        let at = |o: u32| window.wrapping_add(o);
        mem.write(at(0x48), 0u16);
        mem.write(at(0x3a), 5u16);
        for (o, f) in [
            (8, 0x008b_3d10),
            (0xc, 0x008b_44b0),
            (0x10, 0x008b_4a50),
            (0x14, 0x008b_55d0),
            (0x1c, 0x008b_3730),
        ] {
            let handler = mem.rebase(f);
            mem.write(at(o), handler);
        }
        mem.write(at(0x30), 0x63u32);
        mem.write(at(0x42), 1u8);
        mem.write(at(0x3e), 1u8);
        mem.write(at(0x58), mode);
        let f = mem.rebase(0x008a_ab00);
        call(mem, f, &[window]);
        let start = mem.rebase(0x008a_f0d0);
        for name in [0x00b6_2fcc, 0x00b6_2fd4, 0x00b6_2fdc] {
            let name = mem.rebase(name);
            call(mem, start, &[1, name]);
        }
        window
    }

    /// `MenuWShop_EquipLoss(id)` (`0x008d8520`, own name; caller
    /// `0x008d6e40`): what equipping `id` would cost its owner. Owner `u8
    /// +4` and armour flag `u8 +5` of `MsGetSaveWeapon(id, &name)`; with
    /// the HP and MP before (`TkMenuGetHP`/`TkMenuGetMP`) and the piece
    /// worn (`TkMenuGetArmor`/`TkMenuGetWeapon`), it equips `id`
    /// (`MsEquipSaveWeapon`, `TkMsSetSaveParam`), sets bit 0 when the HP
    /// drop and bit 1 when the MP drop (signed), equips the old piece
    /// back the same way and restores HP and MP (`TkMenuSetHP`,
    /// `TkMenuSetMP`). Returns the bits. `local` is the name's word.
    pub fn menu_wshop_equip_loss<M: Mem>(
        mem: &mut M,
        id: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x007a_bbd0);
        let slot = call(mem, f, &[id, local]);
        let owner = u32::from(mem.read::<u8>(slot.wrapping_add(4)));
        let armour = u32::from(mem.read::<u8>(slot.wrapping_add(5)));
        let (get_hp, get_mp) = (mem.rebase(0x008a_98c0), mem.rebase(0x008a_9970));
        let hp = call(mem, get_hp, &[owner]);
        let mp = call(mem, get_mp, &[owner]);
        let get = if armour == 0 {
            0x008a_9c70
        } else {
            0x008a_9820
        };
        let f = mem.rebase(get);
        let worn = call(mem, f, &[owner]);
        let (equip, refresh) = (mem.rebase(0x007a_b970), mem.rebase(0x008c_3070));
        call(mem, equip, &[owner, armour, id]);
        call(mem, refresh, &[owner]);
        let mut loss = u32::from(hp.cast_signed() > call(mem, get_hp, &[owner]).cast_signed());
        if mp.cast_signed() > call(mem, get_mp, &[owner]).cast_signed() {
            loss |= 2;
        }
        call(mem, equip, &[owner, armour, worn]);
        call(mem, refresh, &[owner]);
        let f = mem.rebase(0x008a_ae90);
        call(mem, f, &[owner, hp]);
        let f = mem.rebase(0x008a_aef0);
        call(mem, f, &[owner, mp]);
        loss
    }

    /// The weapon shop's buy/sell/exit command window:
    /// `TkMenuOpenSubCmdWindowRect(0, texts, helps, 3, 2, x, y, w, h)`
    /// with `TkMenuMainMes(0x16..=0x18)` and `TkMenuMainHelp(0x16..=0x18)`
    /// in `local` (6 words), `(x, y, w, h)` the remaps of (210, 156,
    /// 1180, 54), done Y 54, X 1180, Y 156, X 210. `between` runs after
    /// the texts, before the remaps. The window's result.
    fn wshop_open_command_window<M: Mem>(
        mem: &mut M,
        local: u32,
        between: impl FnOnce(&mut M),
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let round = |v: f64| (v as f32).to_bits();
        for (k, (f, id)) in [0x008d_df80u32, 0x008d_df60]
            .into_iter()
            .flat_map(|f| (0x16..=0x18u32).map(move |id| (f, id)))
            .enumerate()
        {
            let f = mem.rebase(f);
            let text = call(mem, f, &[id]);
            #[allow(clippy::cast_possible_truncation)] // 6 words
            mem.write(local.wrapping_add(4 * k as u32), text);
        }
        between(mem);
        let height = round(remap(mem, true, 54.0));
        let width = round(remap(mem, false, 1180.0));
        let top = round(remap(mem, true, 156.0));
        let left = round(remap(mem, false, 210.0));
        let f = mem.rebase(0x008e_4930);
        call(
            mem,
            f,
            &[
                0,
                local,
                local.wrapping_add(0xc),
                3,
                2,
                left,
                top,
                width,
                height,
            ],
        )
    }

    /// `MenuWShop_OpenCommandWindow()` (`0x008d8110`, own name; caller
    /// `0x008d6e40`): clears the weapon shop sub windows 0 and 1
    /// (`0x0186aaf0`, `0x0186aaf4`), opens the command window
    /// ([`wshop_open_command_window`]) and returns
    /// `MenuWShop_OpenSubWindow4()`.
    pub fn menu_wshop_open_command_window<M: Mem>(
        mem: &mut M,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let clear = |mem: &mut M| {
            for at in [0x0186_aaf0, 0x0186_aaf4] {
                let at = mem.rebase(at);
                mem.write(at, 0u32);
            }
        };
        wshop_open_command_window(mem, local, clear, remap, call);
        let f = mem.rebase(0x008d_83e0);
        call(mem, f, &[])
    }

    /// `MenuWShop_Start()` (`0x008da7a0`, own name; caller
    /// `TkMenuCtrlWShop`): `MenuWShop_OpenWindow3()`, the command window
    /// ([`wshop_open_command_window`]), then the tutorial windows
    /// (`0x0186ac34`, `0x0186ac38`) cleared. Returns the window's result.
    pub fn menu_wshop_start<M: Mem>(
        mem: &mut M,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x008d_a9c0);
        call(mem, f, &[]);
        let r = wshop_open_command_window(mem, local, |_| {}, remap, call);
        for at in [0x0186_ac34, 0x0186_ac38] {
            let at = mem.rebase(at);
            mem.write(at, 0u32);
        }
        r
    }

    /// `TkCdReadWithMallocWithMessage(dir, file)` (`0x0088d2c0`,
    /// Fahrenheit's name; caller `AtelEventSetUp`): reads a file into a
    /// new buffer. With the CD object `GetCD()`: `+0x10(dir)` selects the
    /// directory, the size is `+0x14(file)` or, for an Eternal Calm scene
    /// (`isEternalCalmScene(file / 18)`, signed: the
    /// scene of a file pair), `getEternalCalmSceneSize(file / 18)`. `Sg_MemSetAlign(0x40)`, the
    /// buffer `user_malloc(size rounded up to 0x800)` (signed), its first
    /// word 0, `FlushCache(0)`, the read `+0(file, buffer, 0, 0)`, the
    /// sync `+8(0)`, `FlushCache(0)`; four `AtelPs2DebugString` marks
    /// (`0x00b5e9c0`, `d0`, `e4`, `f0`) between. Returns the buffer.
    pub fn tk_cd_read_with_malloc_with_message<M: Mem>(
        mem: &mut M,
        dir: u32,
        file: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0076_d0d0);
        let cd = call(mem, f, &[]);
        let select = mem.read::<u32>(cd.wrapping_add(0x10));
        call(mem, select, &[dir]);
        let size_of = mem.read::<u32>(cd.wrapping_add(0x14));
        let mut size = call(mem, size_of, &[file.cast_unsigned()]);
        let scene = (file / 18).cast_unsigned();
        let f = mem.rebase(0x0067_ab50);
        if call(mem, f, &[scene]) != 0 {
            let f = mem.rebase(0x0067_aae0);
            size = call(mem, f, &[scene]);
        }
        let mark = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, at: u32| {
            let f = mem.rebase(0x0087_3c70);
            let text = mem.rebase(at);
            call(mem, f, &[text]);
        };
        mark(mem, call, 0x00b5_e9c0);
        let f = mem.rebase(0x0083_c4a0);
        call(mem, f, &[0x40]);
        let up = size.cast_signed().wrapping_add(0x7ff);
        let up = up.wrapping_add((up >> 31) & 0x7ff) & !0x7ff;
        let f = mem.rebase(0x0068_7020);
        let buffer = call(mem, f, &[up.cast_unsigned()]);
        mark(mem, call, 0x00b5_e9d0);
        mem.write(buffer, 0u32);
        let flush = mem.rebase(0x0062_f4c0);
        call(mem, flush, &[0]);
        mark(mem, call, 0x00b5_e9e4);
        let read = mem.read::<u32>(cd);
        call(mem, read, &[file.cast_unsigned(), buffer, 0, 0]);
        mark(mem, call, 0x00b5_e9f0);
        let sync = mem.read::<u32>(cd.wrapping_add(8));
        call(mem, sync, &[0]);
        call(mem, flush, &[0]);
        buffer
    }

    /// `MenuSave_DicText(out, _, _, index)` (`0x008b43e0`, own name;
    /// caller `0x008b42e0`): text `index` of the save screen's dictionary
    /// into `out` (Shift-JIS, through `local`, 0x40 bytes in the
    /// original). In Japanese (`TOGetFFXLangJmp()` and
    /// `MsGetSaveConfigEnglish()` both 0) the text is
    /// `0x0086c430(index, 0)`; otherwise entry `index` of
    /// `TOGetSaveDicAdrs(MsGetSaveConfigEnglish())`, at its `u16 +2`
    /// offset. `TOFFXToSJIS(text, local)`, then `out` = `local` (`out` is
    /// emptied first, so the append is a copy). No caller reads `eax`.
    pub fn menu_save_dic_text<M: Mem>(
        mem: &mut M,
        out: u32,
        index: i32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        mem.write(out, 0u8);
        let english = mem.rebase(0x0078_51d0);
        let f = mem.rebase(0x008a_c2f0);
        let japanese = call(mem, f, &[]) == 0 && call(mem, english, &[]) == 0;
        let text = if japanese {
            let lang = call(mem, english, &[]);
            let f = mem.rebase(0x0086_c430);
            call(mem, f, &[index.cast_unsigned(), lang])
        } else {
            let lang = call(mem, english, &[]);
            let f = mem.rebase(0x008b_7f60);
            let dic = call(mem, f, &[lang]);
            let entry = dic.wrapping_add(index.cast_unsigned().wrapping_mul(4));
            entry.wrapping_add(u32::from(mem.read::<u16>(entry.wrapping_add(2))))
        };
        let f = mem.rebase(0x008b_6200);
        call(mem, f, &[text, local]);
        let mut end = out;
        while mem.read::<u8>(end) != 0 {
            end = end.wrapping_add(1);
        }
        let mut i = 0u32;
        loop {
            let c = mem.read::<u8>(local.wrapping_add(i));
            mem.write(end.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `MenuItem_DrawCellCursor(_, x, y)` (`0x008c4a90`, own name; caller
    /// `0x008c4030`): with `0x0186a270` 1, the command cursor
    /// `TkMn2DrawCrossCmdCursor(f32(cx), f32(cy), 0)` at `cx =
    /// ftol(remapX2(265) + f32(x))` and `cy = ftol(f32(row) * step + top)`,
    /// `top = ftol(remapY2(508) + f32(y))`, `row` the `i32` at
    /// `0x0186a26c`, `step = f32(remapY2(420) / f32(MenuItem_CursorCount()))`.
    /// No caller reads `eax`.
    pub fn menu_item_draw_cell_cursor<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u32>(mem.rebase(0x0186_a270)) != 1 {
            return;
        }
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let float = |v: i32| v as f32;
        let left = remap(mem, false, 265.0) + f64::from(float(x));
        let left = ffx_bridge::x87::ftol(mem, left).cast_signed();
        let top = remap(mem, true, 508.0) + f64::from(float(y));
        let top = ffx_bridge::x87::ftol(mem, top).cast_signed();
        let height = remap(mem, true, 420.0);
        let f = mem.rebase(0x008c_5030);
        let count = call(mem, f, &[]).cast_signed();
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let step = (height / f64::from(float(count))) as f32;
        let row = mem.read::<u32>(mem.rebase(0x0186_a26c)).cast_signed();
        let at = f64::from(float(row)) * f64::from(step) + f64::from(top);
        let at = ffx_bridge::x87::ftol(mem, at).cast_signed();
        let f = mem.rebase(0x008c_0580);
        call(mem, f, &[float(left).to_bits(), float(at).to_bits(), 0]);
    }

    /// `MenuItem_UseOnTargets(target)` (`0x008c8d50`, own name; caller
    /// `TkMenuCtrlItem`): applies command [`ABILITY_COMMAND`]'s healing
    /// (`TkMenuItemHealAmount(cmd, who, 1)` then `MenuItem_AddMP(who,
    /// amount)`) to `target` when the command has one target
    /// (`0x0186a391` 1), else, by `TkMsGetRomItem(cmd, &n)`'s `+0x22`
    /// flags, to every aeon (`0x80`) and every party member (`0x40`). The
    /// target's amount is computed first in every case. No caller reads
    /// `eax`.
    pub fn menu_item_use_on_targets<M: Mem>(
        mem: &mut M,
        target: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let cmd = |mem: &M| u32::from(mem.read::<u16>(mem.rebase(0x0186_a3a8)));
        let heal = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, who: u32| {
            let f = mem.rebase(0x008c_69a0);
            let c = cmd(mem);
            let amount = call(mem, f, &[c, who, 1]);
            let f = mem.rebase(0x008c_8e20);
            call(mem, f, &[who, amount]);
        };
        // the target's amount comes first, used or not
        let f = mem.rebase(0x008c_69a0);
        let c = cmd(mem);
        let amount = call(mem, f, &[c, target, 1]);
        if mem.read::<u8>(mem.rebase(0x0186_a391)) == 1 {
            let f = mem.rebase(0x008c_8e20);
            call(mem, f, &[target, amount]);
            return;
        }
        let f = mem.rebase(0x008a_b280);
        let c = cmd(mem);
        let item = call(mem, f, &[c, local]);
        for (bit, count, get) in [
            (0x80u8, 0x008a_9bf0, 0x008a_9bc0),
            (0x40, 0x008a_9b30, 0x008a_9ad0),
        ] {
            if mem.read::<u8>(item.wrapping_add(0x22)) & bit == 0 {
                continue;
            }
            let f = mem.rebase(count);
            let n = call(mem, f, &[]).cast_signed();
            for i in 0..n {
                let f = mem.rebase(get);
                let who = call(mem, f, &[i.cast_unsigned()]);
                heal(mem, call, who);
            }
        }
    }

    /// `MenuWeapon_DrawRow(window, x, y, row, fixed)` (`0x008d0170`, own
    /// name; caller `0x008d0250`; `x`, `y` `f32`): unless `fixed`, `y`
    /// scrolls up by `remapY2(79) * i16 +0x46 / 4096` (double, stored as
    /// a float). A row in `0..u16 +0x30` gets a plate
    /// `TODrawMenuPlateXYWHType(x, y, remapX2(740), remapY2(60), 5)` and,
    /// for a nonzero `g_TkMenuItemList` id, `MsGetSaveWeapon(id, &name)`
    /// (unused) and `DrawCrossMenuIconWeaponName(id, x, y, 0)`. No caller
    /// reads `eax`.
    pub fn menu_weapon_draw_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        [row, fixed]: [i32; 2],
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let round = |v: f64| v as f32;
        // the floats pass through fld/fstp (an sNaN comes out quiet)
        let x = ffx_bridge::x87::round_trip(x);
        let mut y = ffx_bridge::x87::round_trip(y);
        if fixed == 0 {
            let line = remap(mem, true, 79.0);
            let scroll = f64::from(mem.read::<i16>(window.wrapping_add(0x46)));
            y = round(f64::from(y) - line * scroll * (1.0 / 4096.0));
        }
        let rows = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= rows {
            return;
        }
        let height = round(remap(mem, true, 60.0)).to_bits();
        let width = round(remap(mem, false, 740.0)).to_bits();
        let f = mem.rebase(0x008f_5f70);
        call(mem, f, &[x.to_bits(), y.to_bits(), width, height, 5]);
        let entry = MENU_ITEM_LIST
            .ptr(mem)
            .addr
            .wrapping_add(row.cast_unsigned() * 4);
        let id = u32::from(mem.read::<u16>(entry));
        if id == 0 {
            return;
        }
        let f = mem.rebase(0x007a_bbd0);
        call(mem, f, &[id, local]);
        let id = u32::from(mem.read::<u16>(entry));
        let f = mem.rebase(0x008e_67f0);
        call(mem, f, &[id, x.to_bits(), y.to_bits(), 0]);
    }

    /// `Pad_ActuatorIds(port, slot)` (`0x00888a70`, own name; caller
    /// `0x00889050`): the pad's small and large motor actuators, `(large
    /// << 8) | small`, `0xffff` without actuators
    /// (`scePadInfoAct(port, slot, -1, 0)` 0) or combinations. For each
    /// combination `c` (`scePadInfoComb(port, slot, -1, 0)` of them) the
    /// actuators `scePadInfoComb(port, slot, c, k)` (`k` below
    /// `scePadInfoComb(port, slot, c, -1)`) whose kind
    /// `scePadInfoAct(port, slot, a, 2)` is 1 (small) or 2 (large),
    /// starting from `0xff`; the first combination whose two values OR to
    /// something other than `0xff` is the answer, else the last one.
    pub fn pad_actuator_ids<M: Mem>(
        mem: &mut M,
        port: u32,
        slot: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let act = mem.rebase(0x0062_fb40);
        let comb = mem.rebase(0x0062_fb50);
        let (mut large, mut small) = (0xffu32, 0xffu32);
        if call(mem, act, &[port, slot, u32::MAX, 0]) == 0 {
            return 0xffff;
        }
        let combs = call(mem, comb, &[port, slot, u32::MAX, 0]).cast_signed();
        for c in 0..combs {
            let c = c.cast_unsigned();
            let n = call(mem, comb, &[port, slot, c, u32::MAX]).cast_signed();
            (large, small) = (0xff, 0xff);
            for k in 0..n {
                let a = call(mem, comb, &[port, slot, c, k.cast_unsigned()]);
                match call(mem, act, &[port, slot, a, 2]) {
                    1 => small = a,
                    2 => large = a,
                    _ => {}
                }
            }
            if large | small != 0xff {
                break;
            }
        }
        (large << 8) | small
    }
    /// `MenuItem_DrawChrSmallPlate(x, y, chr)` (`0x008c1260`, own name;
    /// caller `0x008c9570`): `TOMenuDrawChrSmallPlate_008f51e0(chr, x, y)`,
    /// the floats copied through `fld`/`fstp`; its result.
    pub fn menu_item_draw_chr_small_plate<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::round_trip;
        let f = mem.rebase(0x008f_51e0);
        call(
            mem,
            f,
            &[chr, round_trip(x).to_bits(), round_trip(y).to_bits()],
        )
    }

    /// `TkMenuConfigHelp(id)` (`0x008dc970`, Fahrenheit's name): menu text
    /// 7's help line of `id` (14's when `MsGetSaveConfigEnglish()`, which
    /// is 0 in this build), in kana when `MsGetSaveConfigHiragana()` says
    /// so; the help line.
    pub fn tk_menu_config_help<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let kana = save::ms_get_save_config_hiragana(mem);
        let english = mem.rebase(0x0078_51d0);
        let text = if call(mem, english, &[]) == 0 { 7 } else { 14 };
        ffx_battle::menu::bridge::ms_menu_get_help(mem, text, id, kana)
    }

    /// `MenuWeapon_DrawRows(window, fixed, first, _, y)` (`0x008d0250`,
    /// own name; caller `0x008cf9f0`): five [`menu_weapon_draw_row`]s,
    /// rows `first..first + 5` at `x = f32(remapX2(210))`, the first at
    /// `y = f32(remapY2(599) + f32(y))`, each next one
    /// `f32(remapY2(79) + y)` lower. No caller reads `eax`.
    pub fn menu_weapon_draw_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        [fixed, first, y]: [i32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rows = Rows {
            row: 0x008d_0170,
            x: 210.0,
            top: 599.0,
            step: 79.0,
            from: 0,
            to: 5,
        };
        draw_rows(mem, &rows, window, [fixed, first, y], remap, call);
    }

    /// `MenuKaizou_DrawRows(window, fixed, first, _, y)` (`0x008d6560`,
    /// own name; caller `0x008d5d50`): [`menu_weapon_draw_rows`] for the
    /// customise menu, eleven [`menu_kaizou_draw_row`]s, rows
    /// `first - 1..first + 10`, at `remapX2(209)`, from `remapY2(244) +
    /// y`, `remapY2(75)` apart. No caller reads `eax`.
    pub fn menu_kaizou_draw_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        [fixed, first, y]: [i32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rows = Rows {
            row: 0x008d_6760,
            x: 209.0,
            top: 244.0,
            step: 75.0,
            from: -1,
            to: 10,
        };
        draw_rows(mem, &rows, window, [fixed, first, y], remap, call);
    }

    /// A row list's layout: the row function, the remapped `x`, first
    /// `y` and step, and the row offsets `from..to` added to `first`.
    struct Rows {
        row: u32,
        x: f32,
        top: f32,
        step: f32,
        from: i32,
        to: i32,
    }

    /// The row loop of [`menu_weapon_draw_rows`] and
    /// [`menu_kaizou_draw_rows`]: `row(window, x, y, first + i, fixed)`,
    /// `y` stepped (as a float) after each row, the last included.
    fn draw_rows<M: Mem>(
        mem: &mut M,
        rows: &Rows,
        window: u32,
        [fixed, first, y]: [i32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87;
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let y = y as f32;
        let x = x87::store(remap(mem, false, rows.x));
        let mut y = x87::store(remap(mem, true, rows.top) + f64::from(y));
        let row = mem.rebase(rows.row);
        for i in rows.from..rows.to {
            let args = [
                window,
                x,
                y,
                first.wrapping_add(i).cast_unsigned(),
                fixed.cast_unsigned(),
            ];
            call(mem, row, &args);
            y = x87::store(remap(mem, true, rows.step) + x87::load(y));
        }
    }

    /// `MenuSave_BuildIconSys(n, b, index)` (`0x008b42e0`, own name;
    /// callers `TkMenuCtrlSaveLoad`, `TkMenuPreSave`): the PS2 `icon.sys`
    /// at `0x0159e870` (0x3c4 bytes) cleared, magic `PS2D` at `+0`, `u16`
    /// 0x20 at `+6` and 0x60 at `+0xc`; the title
    /// [`menu_save_dic_text`]`(local, n, b, index)` at `+0xc0`, then
    /// `MenuSave_IconName(local, n)` at `+0x104`, `+0x144` and `+0x184`
    /// (each copied up to its NUL, `local` 0x100 bytes in the original).
    /// No caller reads `eax`.
    pub fn menu_save_build_icon_sys<M: Mem>(
        mem: &mut M,
        [icon, second, index]: [u32; 3],
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let sys = mem.rebase(0x0159_e870);
        mem.write_bytes(sys, &[0; 0x3c4]);
        mem.write(sys.wrapping_add(6), 0x20u16);
        mem.write(sys, 0x4432_5350u32);
        mem.write(sys.wrapping_add(0xc), 0x60u32);
        let dic = mem.rebase(0x008b_43e0);
        call(mem, dic, &[local, icon, second, index]);
        let copy = |mem: &mut M, to: u32| {
            let mut i = 0u32;
            loop {
                let c = mem.read::<u8>(local.wrapping_add(i));
                mem.write(to.wrapping_add(i), c);
                if c == 0 {
                    break;
                }
                i = i.wrapping_add(1);
            }
        };
        copy(mem, sys.wrapping_add(0xc0));
        menu_save_icon_name(mem, local, icon);
        for at in [0x104, 0x144, 0x184] {
            copy(mem, sys.wrapping_add(at));
        }
    }

    /// `BtlEnd_StepEntry(i)` (`0x008bb690`, own name; caller
    /// `TkBtlEndCtrl`): one step of after-battle entry `i` (14 bytes from
    /// `0x01869f20`: `i16 +2` a timer, `i16 +4` the state, `i16 +6` the
    /// character). State 0 and 3 count the timer down, and below 0 go on
    /// to 1 (timer 0; state 0 then returns 0 at once) or 4; 1 and 4 count
    /// it up by 0xe3 and from 0x1000 go on to 2 or 5 (timer 0x1000); 2
    /// sets its timer to 0x1000 again. Then, with `BtlEnd_GetSP(chr)` at least 99,
    /// `SetTable_02310f20(chr, 0)`. Returns 1 in state 2 or 5, else 0.
    pub fn btl_end_step_entry<M: Mem>(
        mem: &mut M,
        i: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let e = mem.rebase(0x0186_9f22).wrapping_add(i.wrapping_mul(14));
        let mut done = 0;
        // the timer and the state as one dword
        let go = |mem: &mut M, state: u32, timer: u32| mem.write(e, (state << 16) | timer);
        let step = |mem: &mut M, by: i16| {
            let t = mem.read::<i16>(e).wrapping_add(by);
            mem.write(e, t);
            t
        };
        match mem.read::<i16>(e.wrapping_add(2)) {
            0 => {
                if step(mem, -1) < 0 {
                    go(mem, 1, 0);
                }
                return 0;
            }
            1 => {
                if step(mem, 0xe3) >= 0x1000 {
                    go(mem, 2, 0x1000);
                }
            }
            2 => {
                go(mem, 2, 0x1000);
                done = 1;
            }
            3 => {
                if step(mem, -1) < 0 {
                    go(mem, 4, 0);
                }
            }
            4 => {
                if step(mem, 0xe3) >= 0x1000 {
                    go(mem, 5, 0x1000);
                }
            }
            5 => done = 1,
            _ => {}
        }
        let chr = |mem: &M| i32::from(mem.read::<i16>(e.wrapping_add(4))).cast_unsigned();
        let sp = mem.rebase(0x008b_da50);
        let c = chr(mem);
        if call(mem, sp, &[c]).cast_signed() >= 0x63 {
            let set = mem.rebase(0x008b_dfc0);
            let c = chr(mem);
            call(mem, set, &[c, 0]);
        }
        done
    }
    /// `MenuScene_KeyEase(key, frame, frames)` (`0x00a7c6f0`, own name;
    /// caller `0x00a7b290`): how far a menu scene key is at `frame` of
    /// `frames`, returned in `ST0`. `t = f32(f32(frame) / f32(frames))`
    /// (1 for no frames); by the key's bits `0x7000`: 0x2000 ease out
    /// `1 - (1 - t)²`, 0x3000 ease in `t²`, 0x4000 `(cos(v) + 1) / 2` with
    /// `v = (1 - t) π` clamped to `[0, π]` (`cos` is `_CIcos`), else `t`;
    /// every step rounded to a float.
    pub fn menu_scene_key_ease<M: Mem>(
        mem: &mut M,
        key: u32,
        frame: i32,
        frames: i32,
        cos: &mut impl FnMut(&mut M, f64) -> f64,
    ) -> f32 {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let int = |v: i32| f64::from(v as f32);
        let round = |v: f64| load(store(v));
        let t = round(div(int(frame), int(frames)));
        if frames == 0 {
            return 1.0;
        }
        let t = match key & 0x7000 {
            0x2000 => {
                let u = round(sub(1.0, t));
                let sq = round(mul(u, u));
                round(sub(1.0, sq))
            }
            0x3000 => round(mul(t, t)),
            0x4000 => {
                // g_PiF64 holds the float pi, like g_PiF32
                let pi = f64::from(std::f32::consts::PI);
                let a = round(sub(1.0, t));
                let mut v = round(mul(a, pi));
                if v > pi {
                    v = pi;
                } else if v < 0.0 {
                    v = 0.0;
                }
                let c = round(cos(mem, v));
                let c = round(add(c, 1.0));
                round(mul(c, 0.5))
            }
            _ => t,
        };
        #[allow(clippy::cast_possible_truncation)] // a float already
        let t = t as f32;
        t
    }

    /// `MenuKaizou_DrawWeaponTitle(x, y, i)` (`0x008d6660`, own name;
    /// caller `DrawGearCustomizationMenu`): the customise menu's header
    /// plate `TODrawMenuPlateXYWHType(f32(x), f32(y), f32(remapX2(740)),
    /// f32(remapY2(60)), 5)` and on it the name of weapon `i` of the list
    /// at `[0x0186aa38]` (`MsGetSaveWeapon(id, &name)`), centred on
    /// `x + remapX2(370)`: `ToGetBtlEasyFontWidth(name, &width, 0, 0.78,
    /// 1)`, then `ToMakeBtlEasyFont(name, f32(remapX2(370) + x - width /
    /// 2), f32(remapY2(8) + y), 0, 0.78, 1)`. `local` holds the name and
    /// the width (two words). No caller reads `eax`.
    pub fn menu_kaizou_draw_weapon_title<M: Mem>(
        mem: &mut M,
        [x, y, i]: [i32; 3],
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let list: u32 = mem.read(mem.rebase(0x0186_aa38));
        let id = mem.read::<u16>(list.wrapping_add(i.cast_unsigned().wrapping_mul(2)));
        let weapon = mem.rebase(0x007a_bbd0);
        call(mem, weapon, &[u32::from(id), local]);
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let (x, y) = (store(f64::from(x as f32)), store(f64::from(y as f32)));
        let height = store(remap(mem, true, 60.0));
        let width = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[x, y, width, height, 5]);
        let (scale, one) = (0.78f32.to_bits(), 1.0f32.to_bits());
        let name: u32 = mem.read(local);
        let out = local.wrapping_add(4);
        let measure = mem.rebase(0x0090_5290);
        call(mem, measure, &[name, out, 0, scale, one]);
        let ty = store(add(remap(mem, true, 8.0), load(y)));
        let left = add(remap(mem, false, 370.0), load(x));
        let half = mul(load(mem.read(out)), 0.5);
        let tx = store(sub(left, half));
        let name: u32 = mem.read(local);
        let font = mem.rebase(0x0090_5ab0);
        call(mem, font, &[name, tx, ty, 0, scale, one]);
    }

    /// `MenuKaizou_DrawRow(window, x, y, row, fixed)` (`0x008d6760`, own
    /// name; caller `0x008d6560`): [`menu_weapon_draw_row`] for the
    /// customise menu: `y` scrolled by `remapY2(75)` lines (unless
    /// `fixed`), rows of the `u16` list at `[0x0186aa38]` (count `u16
    /// +0x30`), the name (`DrawCrossMenuIconWeaponName(id, x, y, colour)`,
    /// `colour` 1 when `MenuEquip_IsSelectable(id, 0)` is 0) only for a
    /// weapon whose `MsGetSaveWeapon` record has `+2` set and `+3` bit 1
    /// clear. No caller reads `eax`.
    pub fn menu_kaizou_draw_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        [row, fixed]: [i32; 2],
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{round_trip, store};
        let x = round_trip(x).to_bits();
        let mut y = round_trip(y).to_bits();
        if fixed == 0 {
            let line = remap(mem, true, 75.0);
            let scroll = f64::from(mem.read::<i16>(window.wrapping_add(0x46)));
            y = store(f64::from(f32::from_bits(y)) - line * scroll * (1.0 / 4096.0));
        }
        let rows = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= rows {
            return;
        }
        let height = store(remap(mem, true, 60.0));
        let width = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[x, y, width, height, 5]);
        let id = |mem: &M| {
            let list: u32 = mem.read(mem.rebase(0x0186_aa38));
            u32::from(mem.read::<u16>(list.wrapping_add(row.cast_unsigned() * 2)))
        };
        let weapon = mem.rebase(0x007a_bbd0);
        let w = id(mem);
        let rec = call(mem, weapon, &[w, local]);
        if mem.read::<u8>(rec.wrapping_add(2)) == 0 || mem.read::<u8>(rec.wrapping_add(3)) & 2 != 0
        {
            return;
        }
        let selectable = mem.rebase(0x008d_5750);
        let w = id(mem);
        let colour = u32::from(call(mem, selectable, &[w, 0]) == 0);
        let name = mem.rebase(0x008e_67f0);
        let w = id(mem);
        call(mem, name, &[w, x, y, colour]);
    }

    /// `Pad_RepeatButton(work, _, i, now, prev, slot)` (`0x00889980`, own
    /// name; caller `0x00889940`, for `i` 0-3): key repeat of button `i`
    /// (mask `u16 0x00c54e34[i]` over `now` and `prev`) in a pad work:
    /// counter `u16 +0xa0`, timer `f32 +0xb0` and last time `f32 +0xd0`,
    /// each at `[i + 4 slot]`. Pressed and held: the timer runs down by the
    /// time since last (`inputGetCurTime()`, `time`), and below 0 restarts
    /// at `i16 +0x96 / 30` (counter the same) and the button repeats.
    /// Newly pressed: timer `i16 +0x94 / 30`, and the button counts when
    /// no masked `prev` bit was released. The time is stored as last.
    /// Returns the button bits in `ax` (the rest of `eax` is `slot`'s
    /// high half, like the original's).
    pub fn pad_repeat_button<M: Mem>(
        mem: &mut M,
        work: u32,
        [i, now, prev, slot]: [u32; 4],
        time: &mut impl FnMut(&mut M) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store, sub};
        let t = store(time(mem));
        let mask = mem.read::<u16>(mem.rebase(0x00c5_4e34).wrapping_add(i.wrapping_mul(2)));
        let k = i.wrapping_add(slot.wrapping_mul(4));
        let counter = work.wrapping_add(0xa0).wrapping_add(k.wrapping_mul(2));
        let timer = work.wrapping_add(0xb0).wrapping_add(k.wrapping_mul(4));
        let last = work.wrapping_add(0xd0).wrapping_add(k.wrapping_mul(4));
        // the low halves (16-bit and, cmp)
        let low = |v: u32| u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]]);
        let prev = low(prev) & mask;
        let now = low(now) & mask;
        let mut left = ffx_bridge::x87::round_trip(f32::from_bits(mem.read(timer))).to_bits();
        let mut count = mem.read::<u16>(counter);
        let mut out = 0u16;
        // f32(f32(n) / 30), 1/30 a double
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let restart = |n: i16| {
            store(mul(
                f64::from(f32::from(n)),
                f64::from_bits(0x3fa1_1111_0000_0000),
            ))
        };
        if now != 0 {
            if now == prev {
                let since = sub(load(t), f64::from(f32::from_bits(mem.read(last))));
                left = store(sub(load(left), since));
                if 0.0 > load(left) {
                    let n = mem.read::<i16>(work.wrapping_add(0x96));
                    count = n.cast_unsigned();
                    out = now;
                    left = restart(n);
                }
            } else {
                let n = mem.read::<i16>(work.wrapping_add(0x94));
                count = n.cast_unsigned();
                left = restart(n);
                if (now ^ prev) & prev == 0 {
                    out = now;
                }
            }
        }
        mem.write(counter, count);
        mem.write(timer, left);
        mem.write(last, t);
        (slot & 0xffff_0000) | u32::from(out)
    }
    /// `Pad_RepeatButtons(work, b, now, prev, slot)` (`0x00889940`, own
    /// name): [`pad_repeat_button`] for buttons 0-3, their bits or'd in
    /// `ax` (the rest of `eax` the last call's).
    pub fn pad_repeat_buttons<M: Mem>(
        mem: &mut M,
        work: u32,
        [b, now, prev, slot]: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(0x0088_9980);
        let mut bits = 0;
        let mut eax = 0;
        for i in 0..4 {
            eax = call(mem, f, &[work, b, i, now, prev, slot]);
            bits |= eax & 0xffff;
        }
        (eax & 0xffff_0000) | bits
    }

    /// `Math_SemicircleTable(out, n, r)` (`0x008cac70`, own name; caller
    /// `TkBtlEndInit`): `n` `u16`s at `out`, entry `i` =
    /// `ftol(f32(f32(sin(f32(acos(x)))) * f32(r) * 16))` with `x =
    /// f32((2 f32(i) - d) / d)`, `d = f32(n) - 1`, clamped to `[-1, 1]`
    /// (NaN kept); then the first and the `n`-th entry are 0 (also for `n`
    /// below 1: `out[n - 1]`). `trig(sin, v)` is the C runtime's `_CIsin`
    /// or `_CIacos`. Returns `out`.
    #[allow(clippy::many_single_char_names)] // the formula's names
    pub fn math_semicircle_table<M: Mem>(
        mem: &mut M,
        out: u32,
        [count, radius]: [i32; 2],
        trig: &mut impl FnMut(&mut M, bool, f64) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{div, load, mul, store, sub};
        #[allow(clippy::cast_precision_loss)] // fild, fstp dword
        let int = |v: i32| f64::from(v as f32);
        let round = |v: f64| load(store(v));
        if count > 0 {
            let d = sub(int(count), 1.0);
            let radius = int(radius);
            for i in 0..count {
                let q = round(div(sub(mul(2.0, int(i)), d), d));
                // (a NaN stays a NaN)
                let x = q.clamp(-1.0, 1.0);
                let a = round(trig(mem, false, x));
                let s = round(trig(mem, true, a));
                let v = round(mul(mul(s, radius), 16.0));
                let v = ffx_bridge::x87::ftol(mem, v);
                #[allow(clippy::cast_possible_truncation)] // mov word
                mem.write(out.wrapping_add(i.cast_unsigned() * 2), v as u16);
            }
        }
        mem.write(out, 0u16);
        let last = count.wrapping_sub(1).cast_unsigned().wrapping_mul(2);
        mem.write(out.wrapping_add(last), 0u16);
        out
    }

    /// `MenuAbility_CommandTargetMask(cmd)` (`0x008c6bc0`, own name;
    /// caller `TkMenuCtrlAbility`): command `cmd` (kept at `0x0186a3a8`)
    /// from `MsGetRomPlyCommand(cmd, local)`: the recovery kind byte
    /// `0x0186a392` from `+0x23` (2 gives 1, 3 gives 2, else 0), the
    /// all-targets byte `0x0186a391` 0 when `+0x1a >> 2 & 3` is 1 or 2,
    /// else 1; then bit `i` of the `u16` at `0x0186a3a4` for each aeon
    /// (`+0x22` bit 7, `TkMenuGetSummonFromIndex`) and then, from bit 0
    /// again, each party member (bit 6, `TkMenuGetPlayerFromIndex2`) that
    /// `MenuItem_TargetNeedsRecovery(kind, all, chr)` accepts. Returns 0
    /// with party members, else 1 with aeons, else 0.
    pub fn menu_ability_command_target_mask<M: Mem>(
        mem: &mut M,
        cmd: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        target_mask(mem, cmd, local, false, call)
    }

    /// `MenuItem_ItemTargetMask(item)` (`0x008c8a30`, own name; caller
    /// `TkMenuCtrlItem`): [`menu_ability_command_target_mask`] for an item
    /// (`TkMsGetRomItem(item, local)`), whose recovery kind becomes 3 when
    /// `TOGetFFXLangJmp()` (always asked) is not 11 and `+0x22` has bit
    /// 4. Its `shl ax, cl` bit equals the ability version's rotated one in
    /// the low 16 bits.
    pub fn menu_item_item_target_mask<M: Mem>(
        mem: &mut M,
        item: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        target_mask(mem, item, local, true, call)
    }

    /// The body of [`menu_ability_command_target_mask`] and
    /// [`menu_item_item_target_mask`].
    fn target_mask<M: Mem>(
        mem: &mut M,
        cmd: u32,
        local: u32,
        item: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        #[allow(clippy::cast_possible_truncation)] // mov word
        mem.write(r(mem, 0x0186_a3a8), cmd as u16);
        let rom = r(mem, if item { 0x008a_b280 } else { 0x0079_0a90 });
        let rec = call(mem, rom, &[cmd, local]);
        let kind = match mem.read::<u8>(rec.wrapping_add(0x23)) {
            2 => 1u8,
            3 => 2,
            _ => 0,
        };
        mem.write(r(mem, 0x0186_a392), kind);
        if item {
            let lang = r(mem, 0x008a_c2f0);
            if call(mem, lang, &[]) != 11 && mem.read::<u8>(rec.wrapping_add(0x22)) & 0x10 != 0 {
                mem.write(r(mem, 0x0186_a392), 3u8);
            }
        }
        let targets = (mem.read::<u16>(rec.wrapping_add(0x1a)) >> 2) & 3;
        mem.write(r(mem, 0x0186_a391), u8::from(!matches!(targets, 1 | 2)));
        let mut mask = 0u32;
        let mut ret = 0;
        let groups = [
            (0x80u8, 0x008a_9bf0u32, 0x008a_9bc0u32),
            (0x40, 0x008a_9b30, 0x008a_9ad0),
        ];
        for (bit, max, get) in groups {
            if mem.read::<u8>(rec.wrapping_add(0x22)) & bit == 0 {
                continue;
            }
            let f = r(mem, max);
            let n = call(mem, f, &[]).cast_signed();
            let mut one = 1u32;
            for i in 0..n {
                let f = r(mem, get);
                let chr = call(mem, f, &[i.cast_unsigned()]);
                let kind = u32::from(mem.read::<u8>(r(mem, 0x0186_a392)));
                let all = u32::from(mem.read::<u8>(r(mem, 0x0186_a391)));
                let f = r(mem, 0x008c_6a70);
                if call(mem, f, &[kind, all, chr]) != 0 {
                    mask |= one;
                }
                one = one.rotate_left(1);
            }
            ret = u32::from(bit == 0x80);
        }
        #[allow(clippy::cast_possible_truncation)] // mov word
        mem.write(r(mem, 0x0186_a3a4), mask as u16);
        ret
    }

    /// `MenuItem_SortInventory()` (`0x008c8fe0`, own name; caller
    /// `TkMenuCtrlItem`): rebuilds the save's item list in the game's
    /// item order. All 256 entries of `MsSaveItemTypeBuffer(local)` become
    /// 0xff and of `MsSaveItemNumBuffer(local)` 0 (past their 0x70 slots,
    /// as in the original); the counts of the menu list
    /// (`TOAmapGetItemListMax()` entries of 4 bytes at `0x01597770`: `u16`
    /// id, count byte `+2`) indexed by `id & 0xfff` (ids above 0xff and
    /// zero counts skipped); then for each id of the order table at
    /// `0x00c87578` (`u16`s up to 0xffff) with a count, the id and the
    /// count's low byte appended. `TkMenuItemRefresh(4)`. The order table
    /// (0x2000-0x206f, never written) indexes only the 256 counts; an
    /// index past them (stack in the original) counts as 0 here. No caller
    /// reads `eax`.
    pub fn menu_item_sort_inventory<M: Mem>(
        mem: &mut M,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let f = r(mem, 0x0079_0530);
        let mut types = call(mem, f, &[local]);
        let f = r(mem, 0x0079_0510);
        let mut nums = call(mem, f, &[local]);
        for i in 0..0x100u32 {
            mem.write(types.wrapping_add(i * 2), 0xffu16);
            mem.write(nums.wrapping_add(i), 0u8);
        }
        let mut counts = [0u16; 0x100];
        let f = r(mem, 0x008c_8c30);
        let n = call(mem, f, &[]).cast_signed();
        let mut at = r(mem, 0x0159_7770);
        for _ in 0..n {
            let id = usize::from(mem.read::<u16>(at) & 0xfff);
            let count = mem.read::<u8>(at.wrapping_add(2));
            if count != 0 && id <= 0xff {
                counts[id] = u16::from(count);
            }
            at = at.wrapping_add(4);
        }
        let mut order = r(mem, 0x00c8_7578);
        loop {
            let id = mem.read::<u16>(order);
            if id == 0xffff {
                break;
            }
            let count = counts.get(usize::from(id & 0xfff)).copied().unwrap_or(0);
            if count != 0 {
                mem.write(types, id);
                types = types.wrapping_add(2);
                mem.write(nums, count.to_le_bytes()[0]);
                nums = nums.wrapping_add(1);
            }
            order = order.wrapping_add(2);
        }
        let f = r(mem, 0x008c_8e60);
        call(mem, f, &[4]);
    }
    /// `MenuItem_DrawRow(window, x, y, row)` (`0x008daf50`, own name;
    /// caller `0x008daea0`): for `row` within the `u16` count at `window +
    /// 0x30`, the plate `TODrawMenuPlateXYWHType(x, y, f32(remapX2(740)),
    /// f32(remapY2(60)), 4)`; for a menu list entry (4 bytes at
    /// `0x01597770`) whose byte `+3` is set, its count `MsGetSaveItemNum(id)`
    /// right-aligned at `x + remapX2(636)` (`ToGetBtlEasyDigitWidth2(n,
    /// &width, 0.68, 0)`, then `ToMakeBtlEasyDigit2(n, f32(remapX2(636) +
    /// x - width / 2), f32(remapY2(14) + y), 0, 0.68, 0)`) and the name
    /// `DrawCrossMenuIconItemName(id, x, y, 0)`. `local` holds the width.
    /// No caller reads `eax`.
    pub fn menu_item_draw_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        let rows = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= rows {
            return;
        }
        let (xb, yb) = (round_trip(x).to_bits(), round_trip(y).to_bits());
        let height = store(remap(mem, true, 60.0));
        let width = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[xb, yb, width, height, 4]);
        let entry = mem
            .rebase(0x0159_7770)
            .wrapping_add(row.cast_unsigned() * 4);
        if mem.read::<u8>(entry.wrapping_add(3)) == 0 {
            return;
        }
        let id = |mem: &M| u32::from(mem.read::<u16>(entry));
        let scale = 0.68f32.to_bits();
        let num = mem.rebase(0x0079_04b0);
        let i = id(mem);
        let n = call(mem, num, &[i]);
        let measure = mem.rebase(0x0090_5230);
        call(mem, measure, &[n, local, scale, 0]);
        let ty = store(add(remap(mem, true, 14.0), f64::from(y)));
        let left = add(remap(mem, false, 636.0), f64::from(x));
        let half = mul(load(mem.read(local)), 0.5);
        let tx = store(sub(left, half));
        let i = id(mem);
        let n = call(mem, num, &[i]);
        let digits = mem.rebase(0x0090_5550);
        call(mem, digits, &[n, tx, ty, 0, scale, 0]);
        let i = id(mem);
        let name = mem.rebase(0x008e_6700);
        call(mem, name, &[i, xb, yb, 0]);
    }

    /// `NameInputChrChange(chr)` (`0x008c33f0`, Fahrenheit's name): the
    /// name entry switches to character `chr` (at most 0x11, not the
    /// current `0x0186a258`; else nothing). With `w` the window at
    /// `[0x0186a278]`: `0x0186a274` the title `TOMenuGetNameMes(0x204e)`
    /// for an aeon (`chr` 8 up) in menu mode bit 19, else `0x200b`; `w +
    /// 0x5c` and `0x0186a258` = `chr`; `MsGetSaveConfigEnglish()` asked in
    /// Japanese; `0x0186a25c`, `0x0186a268`, `0x0186a260` cleared. Without
    /// a `w + 0x58` the character window opens (`w + 0x60` =
    /// `MenuNameEntry_OpenCharWindow(chr)`, `0x0186a270` 1, `0x0186a26c` 1
    /// in Japanese else 3, the face slide `0x023cc3c4` = the old character
    /// and `0x023cc3c8` 0x1000 when it is not below `chr`, else -0x1000);
    /// with one `w + 0x60`, `0x0186a270` and `0x0186a26c` are 0.
    /// `MenuNameEntry_LoadLayout()`, `TkMn2GetStatusPos()`, `w + 0x64` =
    /// `MenuNameEntry_RegistWindow()`, and the tail call
    /// `Menu_ParseItemKeys(TOGetSaveChrName(chr))`, whose result is
    /// returned (0 when nothing happens; the original leaves `eax`
    /// alone, and its caller does not read it).
    pub fn name_input_chr_change<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let w: u32 = mem.read(r(mem, 0x0186_a278));
        let target = r(mem, 0x0186_a258);
        if chr > 0x11 || chr == mem.read::<u32>(target) {
            return 0;
        }
        let open = mem.read::<u32>(w.wrapping_add(0x58));
        let title = r(mem, 0x0186_a274);
        mem.write(title, 0u32);
        let f = r(mem, 0x008a_b240);
        let mode = call(mem, f, &[]);
        let id = if mode & 0x8_0000 != 0 && chr >= 8 {
            0x204e
        } else {
            0x200b
        };
        let f = r(mem, 0x008b_edd0);
        let text = call(mem, f, &[id]);
        mem.write(title, text);
        let old = mem.read::<u32>(target);
        mem.write(w.wrapping_add(0x5c), chr);
        mem.write(target, chr);
        let lang = r(mem, 0x008a_c2f0);
        if call(mem, lang, &[]) == 0 {
            let f = r(mem, 0x0078_51d0);
            call(mem, f, &[]);
        }
        for a in [0x0186_a25c, 0x0186_a268, 0x0186_a260] {
            mem.write(r(mem, a), 0u32);
        }
        let (on, layout) = (r(mem, 0x0186_a270), r(mem, 0x0186_a26c));
        if open == 0 {
            let f = r(mem, 0x008c_43f0);
            let window = call(mem, f, &[chr]);
            mem.write(w.wrapping_add(0x60), window);
            mem.write(on, 1u32);
            let japanese = call(mem, lang, &[]) == 0;
            mem.write(layout, if japanese { 3u32 } else { 1 });
            mem.write(r(mem, 0x023c_c3c4), old);
            let slide: i32 = if old.cast_signed() >= chr.cast_signed() {
                0x1000
            } else {
                -0x1000
            };
            mem.write(r(mem, 0x023c_c3c8), slide);
        } else {
            mem.write(w.wrapping_add(0x60), 0u32);
            mem.write(on, 0u32);
            mem.write(layout, 0u32);
        }
        for a in [0x008c_5050, 0x008d_4920] {
            let f = r(mem, a);
            call(mem, f, &[]);
        }
        let f = r(mem, 0x008c_4450);
        let regist = call(mem, f, &[]);
        mem.write(w.wrapping_add(0x64), regist);
        let f = r(mem, 0x008a_c850);
        let name = call(mem, f, &[chr]);
        let f = r(mem, 0x008c_5280);
        call(mem, f, &[name])
    }
    /// `MenuItem_DrawRowPairs(window, _, pair, x, y)` (`0x008daea0`, own
    /// name; caller `0x008da470`; `x`, `y` `f32`): ten pairs of
    /// `MenuItem_DrawRow(window, x', y', row)`, rows `2 pair + k` for `k`
    /// in `0..20`, the left one at `x' = f32(graphicUiRemapX2(210) + x)`,
    /// the right one at `f32(graphicUiRemapX2(978) + x)` (both remapped
    /// again for every pair), `y'` from `y` and after each pair
    /// `f32(graphicUiRemapY2(77) + y')`. The original's `eax` is what the
    /// last remap leaves (no caller reads it).
    pub fn menu_item_draw_row_pairs<M: Mem>(
        mem: &mut M,
        window: u32,
        pair: u32,
        [x, y]: [f32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, round_trip, store};
        let draw = mem.rebase(0x008d_af50);
        let mut top = round_trip(y).to_bits();
        let mut row = pair.wrapping_mul(2);
        for _ in 0..10 {
            for (k, column) in [(0, 210.0), (1, 978.0)] {
                let left = store(add(remap(mem, false, column), load(x.to_bits())));
                call(mem, draw, &[window, left, top, row.wrapping_add(k)]);
            }
            top = store(add(remap(mem, true, 77.0), load(top)));
            row = row.wrapping_add(2);
        }
    }

    /// `Pad_UpdateFrame(a, b)` (`0x00889790`, own name; caller
    /// `TkScanControler`): with `work = TkGetControlerWork(a, b)`, the
    /// previous frame `TkGetControlerFrame(work, 0)` is read, the ring's
    /// newest index (`u8 +0x9c`) steps (`+1 & 3`) and the new frame `cur
    /// = TkGetControlerFrame(work, 0)` gets the bytes `+0x80`, `+0x82` at
    /// `+0`, `+1`. By the mode byte `+0x81` (read before the step): 1, the
    /// live buttons: `u16 +0x98` at `+2` and `+4`, `+0x9a` at `+0x1e`, the
    /// edges against the previous frame's `+2` at `+6` (pressed, `(old ^
    /// new) & new`) and `+8` (released), the 16 bytes from `work + 0x84`
    /// at `+0xc`, and `Pad_RepeatButtons` for slot 0 (`+0x98` against the
    /// old `+2`) into `+0xa` and slot 1 (`+0x9a` against the old `+0x1e`)
    /// into `+0x1c`; 2, the previous frame's 32 bytes copied (by words),
    /// then the edges `+6..+0xc` and `+0x1c` cleared. Without the byte
    /// `+0x9e`, `Pad_ResetWork(cur)`. Returns what the original leaves in
    /// `eax`: `Pad_ResetWork`'s result, else the slot-1 repeats (mode 1),
    /// 0 (mode 2) or the `u16 +0x9a`.
    pub fn pad_update_frame<M: Mem>(
        mem: &mut M,
        a: u32,
        b: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let get_work = mem.rebase(0x0088_8ec0);
        let get_frame = mem.rebase(0x0088_8b60);
        let work = call(mem, get_work, &[a, b]);
        let at = |o: u32| work.wrapping_add(o);
        let prev = call(mem, get_frame, &[work, 0]);
        let mode = mem.read::<u8>(at(0x81));
        let newest = mem.read::<u8>(at(0x9c)).wrapping_add(1) & 3;
        mem.write(at(0x9c), newest);
        let cur = call(mem, get_frame, &[work, 0]);
        let now = u32::from(mem.read::<u16>(at(0x98)));
        let now2 = u32::from(mem.read::<u16>(at(0x9a)));
        let v = mem.read::<u8>(at(0x80));
        mem.write(cur, v);
        let v = mem.read::<u8>(at(0x82));
        mem.write(cur.wrapping_add(1), v);
        let mut eax = now2;
        match mode {
            1 => {
                let repeat = mem.rebase(0x0088_9940);
                #[allow(clippy::cast_possible_truncation)] // u16 stores
                let half = |v: u32| v as u16;
                mem.write(cur.wrapping_add(0x1e), half(now2));
                mem.write(cur.wrapping_add(2), half(now));
                mem.write(cur.wrapping_add(4), half(now));
                let old = u32::from(mem.read::<u16>(prev.wrapping_add(2)));
                let old2 = u32::from(mem.read::<u16>(prev.wrapping_add(0x1e)));
                let changed = old ^ now;
                mem.write(cur.wrapping_add(6), half(changed & now));
                mem.write(cur.wrapping_add(8), half(changed & old));
                for k in 0..16 {
                    let v = mem.read::<u8>(at(0x84 + k));
                    mem.write(cur.wrapping_add(0xc + k), v);
                }
                let r = call(mem, repeat, &[work, cur, now, old, 0]);
                mem.write(cur.wrapping_add(0xa), half(r));
                eax = call(mem, repeat, &[work, cur, now2, old2, 1]);
                mem.write(cur.wrapping_add(0x1c), half(eax));
            }
            2 => {
                for k in 0..8 {
                    let v = mem.read::<u32>(prev.wrapping_add(4 * k));
                    mem.write(cur.wrapping_add(4 * k), v);
                }
                mem.write(cur.wrapping_add(6), 0u32);
                mem.write(cur.wrapping_add(0xa), 0u16);
                mem.write(cur.wrapping_add(0x1c), 0u16);
                eax = 0;
            }
            _ => {}
        }
        if mem.read::<u8>(at(0x9e)) == 0 {
            let reset = mem.rebase(0x0088_8970);
            eax = call(mem, reset, &[cur]);
        }
        eax
    }

    /// `TkMenuDrawAll()` (`0x008aa290`, own name; caller `0x008a9ca0`):
    /// nothing while the first read is not done (`i8 0x01340859` 0), the
    /// word `0x01871538` set and the map not 0x17e. Else
    /// `TkMenuDrawInitString()`, `TkMenuDrawScissorClear()`, the
    /// background `TODrawMenuBGC2(c0, c1)` unless `graphicGetAbmap()` is 1
    /// (the colours from the bytes `0x025d167c, 7e, 7d, 7f` and `0x025d1678,
    /// 7a, 79, 7b`, high byte first; both `0x80000000` in menu mode
    /// `0x0040....`), `TkMenuDrawKick()`, `TkVU1SyncPath()`,
    /// `TkMenuDrawLayer(0)`, the draw callback `+4(menu)` of each of the 25
    /// [`MENU_SLOTS`] menus whose bit is clear in `0x018408f0` (read each
    /// time) and whose `+0x19` or `+0x1b` byte is set, the main task's
    /// first word called, `TkMenuDrawLayer(1..=8)`, `TkMenuDrawKick()` and
    /// (a tail jump) `TkVU1SyncPath()`, whose `eax` is returned (0 on the
    /// early return, where the original leaves its caller's `eax`; the
    /// caller does not read it).
    pub fn tk_menu_draw_all<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        if mem.read::<u8>(r(mem, 0x0134_0859)) == 0
            && mem.read::<u32>(r(mem, 0x0187_1538)) != 0
            && mem.read::<u32>(r(mem, 0x0133_0844)) != 0x17e
        {
            return 0;
        }
        for f in [0x008d_ce30, 0x008d_d440] {
            let f = r(mem, f);
            call(mem, f, &[]);
        }
        let colour = |mem: &M, bytes: [u32; 4]| {
            bytes.iter().fold(0u32, |c, &a| {
                (c << 8) | u32::from(mem.read::<u8>(mem.rebase(a)))
            })
        };
        let mut c0 = colour(mem, [0x025d_167c, 0x025d_167e, 0x025d_167d, 0x025d_167f]);
        let mut c1 = colour(mem, [0x025d_1678, 0x025d_167a, 0x025d_1679, 0x025d_167b]);
        if MN2_MENU_MODE.get(mem) & 0xffff_0000 == 0x0040_0000 {
            (c0, c1) = (0x8000_0000, 0x8000_0000);
        }
        let f = r(mem, 0x0064_1060);
        if call(mem, f, &[]) != 1 {
            let f = r(mem, 0x008f_5c60);
            call(mem, f, &[c0, c1]);
        }
        let (kick, sync, layer) = (
            r(mem, 0x008d_ce50),
            r(mem, 0x0088_ec60),
            r(mem, 0x008a_9690),
        );
        call(mem, kick, &[]);
        call(mem, sync, &[]);
        call(mem, layer, &[0]);
        let slots = MENU_SLOTS.ptr(mem).addr;
        let mask = MENU_MAIN_ACTIVE_MASK.ptr(mem).addr.wrapping_add(4);
        for i in 0..25u32 {
            let menu = mem.read::<u32>(slots.wrapping_add(i * 4));
            if menu == 0 || mem.read::<u32>(mask) & (1 << i) != 0 {
                continue;
            }
            if mem.read::<u8>(menu.wrapping_add(0x19)) == 0
                && mem.read::<u8>(menu.wrapping_add(0x1b)) == 0
            {
                continue;
            }
            let f = mem.read::<u32>(menu.wrapping_add(4));
            if f != 0 {
                call(mem, f, &[menu]);
            }
        }
        let task = MENU_MAIN_TASK.get(mem);
        if task != 0 {
            let f = mem.read::<u32>(task);
            call(mem, f, &[]);
        }
        for l in 1..=8 {
            call(mem, layer, &[l]);
        }
        call(mem, kick, &[]);
        call(mem, sync, &[])
    }

    /// `MenuSelList_CursorX(window)` (`0x008d1bc0`, own name; caller
    /// `0x008d15a0`, which hands it to `TkMn2DrawCrossCursor`): the
    /// cursor's x on a two-column list, in `ST0` (`f32`). The column's
    /// left `base = f32(graphicUiRemapX2(970))` when bit 0 of `+0x48` (the
    /// cursor) is set, else `f32(graphicUiRemapX2(210))`. Over the `u16
    /// +0x30` entries of [`MENU_ITEM_LIST`] (count read each time) whose
    /// byte `+3` is set, by the byte `+2`: 1 returns `f32(remapX2(140) +
    /// base)` at once; 2 measures `AtelGetSummonNameforFinalBattleSelector
    /// (id)`, 3 `TkMenuMenu1Mes(id + 0x1033)`, with
    /// `ToGetBtlEasyFontWidth(text, &w, 0, 0.78, 1.0)` and keeps the
    /// largest `w` (only a larger one, so NaNs are skipped). Then
    /// `f32((remapX2(740) - widest) * 0.5 + base - remapX2(40))`. `local`
    /// is `w`'s word (the original uses its argument slot).
    pub fn menu_sel_list_cursor_x<M: Mem>(
        mem: &mut M,
        window: u32,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> f32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let column = if mem.read::<u8>(window.wrapping_add(0x48)) & 1 == 0 {
            210.0
        } else {
            970.0
        };
        let base = store(remap(mem, false, column));
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let count = |mem: &M| u32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        let width = mem.rebase(0x0090_5290);
        let mut widest = 0f32.to_bits();
        let mut i = 0u32;
        while i < count(mem) {
            let entry = list.wrapping_add(i * 4);
            if mem.read::<u8>(entry.wrapping_add(3)) != 0 {
                let id = u32::from(mem.read::<u16>(entry));
                let text = match mem.read::<u8>(entry.wrapping_add(2)) {
                    1 => {
                        let x = add(remap(mem, false, 140.0), load(base));
                        return f32::from_bits(store(x));
                    }
                    2 => Some((0x0087_3b50, id)),
                    3 => Some((0x008d_dfc0, id + 0x1033)),
                    _ => None,
                };
                if let Some((f, arg)) = text {
                    let f = mem.rebase(f);
                    let text = call(mem, f, &[arg]);
                    let scale = [0.78f32.to_bits(), 1f32.to_bits()];
                    call(mem, width, &[text, local, 0, scale[0], scale[1]]);
                    let w = mem.read::<u32>(local);
                    if load(w) > load(widest) {
                        widest = store(load(w));
                    }
                }
            }
            i += 1;
        }
        let span = sub(remap(mem, false, 740.0), load(widest));
        let left = add(mul(span, 0.5), load(base));
        f32::from_bits(store(sub(left, remap(mem, false, 40.0))))
    }

    /// `TkMn2DrawCrossSummonBattleCountPlate(x, y, chr)` (`0x008cd720`,
    /// Fahrenheit's name; caller `TkMn2DrawSummonPlateCheckHP`; `x`, `y`
    /// `f32`): with `p = MsGetSavePlayerPtr(chr)`, the plate
    /// `TOMkpShapeXYWHUVC2(-1, x, f32(remapY2(5) + y), f32(remapX2(424)),
    /// f32(remapY2(54)), 1515, 170, 1959, 228, 0x803c3e1a, 0x3c3e1a)` (the
    /// remaps in the order 54, 424, 5); the caption
    /// `TOMkpCrossExtMesFontLClut(0, TkMenuStatusMes(0x18), tx, ty, 0x53,
    /// 0.78, 1.0)` with `tx = f32(remapX2(300) + x)` and
    /// `ty = f32(remapY2(12) + y)` (y remapped first); the count
    /// `TkMn2SetMacroParam(4, 1, u8 p[0x3d])`; and the same line for
    /// `TkMenuStatusMes(0x19)` at `remapX2(86)` with flags 0, whose `eax`
    /// is returned.
    pub fn tk_mn2_draw_cross_summon_battle_count_plate<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        chr: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, round_trip, store};
        let get_player = mem.rebase(0x0078_5330);
        let player = call(mem, get_player, &[chr]);
        let height = store(remap(mem, true, 54.0));
        let width = store(remap(mem, false, 424.0));
        let top = store(add(remap(mem, true, 5.0), load(y.to_bits())));
        let uv = [1515f32, 170.0, 1959.0, 228.0].map(f32::to_bits);
        let xb = round_trip(x).to_bits();
        let shape = mem.rebase(0x0090_3ee0);
        #[rustfmt::skip]
        call(mem, shape, &[0xffff_ffff, xb, top, width, height, uv[0], uv[1], uv[2], uv[3], 0x803c_3e1a, 0x003c_3e1a]);
        let mes = mem.rebase(0x008d_e140);
        let font = mem.rebase(0x0090_16b0);
        let scale = [0.78f32.to_bits(), 1f32.to_bits()];
        let mut eax = 0;
        for (column, id, flags) in [(300.0, 0x18, 0x53), (86.0, 0x19, 0)] {
            if id == 0x19 {
                let count = u32::from(mem.read::<u8>(player.wrapping_add(0x3d)));
                let macro_param = mem.rebase(0x008c_2c60);
                call(mem, macro_param, &[4, 1, count]);
            }
            let ty = store(add(remap(mem, true, 12.0), load(y.to_bits())));
            let tx = store(add(remap(mem, false, column), load(x.to_bits())));
            let text = call(mem, mes, &[id]);
            eax = call(mem, font, &[0, text, tx, ty, flags, scale[0], scale[1]]);
        }
        eax
    }
    /// `MenuItem_SortGear_008c9c40(list, start, n)` (`0x008c9c40`, own
    /// name; caller `0x008c9bf0`, once per owner and gear kind): sorts the
    /// `n` entries of `list` from `start` (4-byte entries, `u16` id) by
    /// their abilities. Each entry gets a 32-byte bit set in `table`
    /// (`MsGetSaveWeapon(id, local)`, ability count `+0xb`, `u16`
    /// abilities from `+0xe`, `0xff` skipped: bit `31 - (a & 31)` of word
    /// `(a & 0xff) >> 5`). Then a selection sort: for each `i < n - 1` the
    /// best of `i..n` is the first one `MenuItem_CompareEntries` ranks
    /// above, ties going to more abilities, then to the first larger
    /// ability id; a new best is swapped in (`SwapSaveGear(id_i,
    /// id_best)`, `MenuItem_SwapEntries`). Every step is reported through
    /// `dbgPrintf` (a `ret` on PC). `table` is the original's 0x1900-byte
    /// frame buffer (200 entries; more overrun it in the original too),
    /// `local` the out word.
    #[allow(clippy::too_many_arguments)]
    pub fn menu_item_sort_gear<M: Mem>(
        mem: &mut M,
        list: u32,
        start: u32,
        n: i32,
        table: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let print = mem.rebase(0x0062_f500);
        let text = |mem: &M, id_a: u32| mem.rebase(id_a);
        let weapon = mem.rebase(0x007a_bbd0);
        let entry =
            |i: i32| list.wrapping_add(start.wrapping_add(i.cast_unsigned()).wrapping_mul(4));
        let mut i = 0i32;
        while i < n {
            let bits = table.wrapping_add(i.cast_unsigned().wrapping_mul(0x20));
            for k in 0..8 {
                mem.write(bits.wrapping_add(4 * k), 0u32);
            }
            let id = u32::from(mem.read::<u16>(entry(i)));
            let rec = call(mem, weapon, &[id, local]);
            let pos = start.wrapping_add(i.cast_unsigned());
            let fmt = text(mem, 0x00b6_5a10);
            call(mem, print, &[fmt, id, pos]);
            let mut k = 0u32;
            while k < u32::from(mem.read::<u8>(rec.wrapping_add(0xb))) {
                let ability = mem.read::<u16>(rec.wrapping_add(0xe + 2 * k));
                if ability != 0xff {
                    let fmt = text(mem, 0x00b6_5a24);
                    call(mem, print, &[fmt, u32::from(ability)]);
                    let low = u32::from(ability & 0xff);
                    let (word, bit) = (low >> 5, 0x1f - (low & 0x1f));
                    let at = bits.wrapping_add(4 * word);
                    let v = mem.read::<u32>(at) | 1 << bit;
                    mem.write(at, v);
                    let fmt = text(mem, 0x00b6_5a34);
                    call(mem, print, &[fmt, word]);
                    let fmt = text(mem, 0x00b6_5a40);
                    call(mem, print, &[fmt, bit]);
                }
                k += 1;
            }
            i += 1;
        }
        let end = n.wrapping_sub(1);
        if end <= 0 {
            return;
        }
        let mut i = 0i32;
        loop {
            let mut best = i;
            let mut other = i.wrapping_add(1);
            while other < n {
                let rank = menu_item_compare_entries(
                    mem,
                    table,
                    best.cast_unsigned(),
                    other.cast_unsigned(),
                );
                if rank < 0 {
                    let fmt = text(mem, 0x00b6_5a4c);
                    call(mem, print, &[fmt, best.cast_unsigned(), i.cast_unsigned()]);
                    best = other;
                } else if rank == 0 {
                    let id_a = u32::from(mem.read::<u16>(entry(best)));
                    let ra = call(mem, weapon, &[id_a, local]);
                    let id_b = u32::from(mem.read::<u16>(entry(other)));
                    let rb = call(mem, weapon, &[id_b, local]);
                    let ca = mem.read::<u8>(ra.wrapping_add(0xb));
                    let cb = mem.read::<u8>(rb.wrapping_add(0xb));
                    if ca < cb {
                        best = other;
                    } else if ca == cb {
                        for k in 0..u32::from(ca) {
                            let ab_a = mem.read::<u16>(ra.wrapping_add(0xe + 2 * k));
                            let ab_b = mem.read::<u16>(rb.wrapping_add(0xe + 2 * k));
                            if ab_a < ab_b {
                                best = other;
                                break;
                            }
                            if ab_a > ab_b {
                                break;
                            }
                        }
                    }
                }
                other = other.wrapping_add(1);
            }
            if best != i {
                let id_a = u32::from(mem.read::<u16>(entry(i)));
                let id_b = u32::from(mem.read::<u16>(entry(best)));
                let fmt = text(mem, 0x00b6_5a64);
                let (from, to) = (
                    start.wrapping_add(i.cast_unsigned()),
                    start.wrapping_add(best.cast_unsigned()),
                );
                call(mem, print, &[fmt, from, to]);
                let swap = mem.rebase(0x007a_b9f0);
                call(mem, swap, &[id_a, id_b]);
                menu_item_swap_entries(mem, table, i.cast_unsigned(), best.cast_unsigned());
            }
            i = i.wrapping_add(1);
            if i >= end {
                return;
            }
        }
    }

    /// A list row's backing: `TODrawMenuPlateXYWHType(x, y, X(740),
    /// Y(60), kind)` (`y` remapped first).
    fn list_row_plate<M: Mem>(
        mem: &mut M,
        [x, y]: [u32; 2],
        kind: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::store;
        let h = store(remap(mem, true, 60.0));
        let w = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[x, y, w, h, kind]);
    }

    /// A list row's count: `ToGetBtlEasyDigitWidth2(v, width, 0.68, 0)`,
    /// then `ToMakeBtlEasyEdgeDigit2(v, f32(X(636) + x - w * 0.5),
    /// f32(Y(14) + y), dim, 0.68, 0)`, `v` the byte at `value` (read for
    /// each call).
    fn list_row_count<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        value: u32,
        dim: u32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let scale = 0.68f32.to_bits();
        let v = u32::from(mem.read::<u8>(value));
        let measure = mem.rebase(0x0090_5230);
        call(mem, measure, &[v, width, scale, 0]);
        let ty = store(add(remap(mem, true, 14.0), load(y.to_bits())));
        let right = add(remap(mem, false, 636.0), load(x.to_bits()));
        let tx = store(sub(right, mul(load(mem.read(width)), 0.5)));
        let v = u32::from(mem.read::<u8>(value));
        let digits = mem.rebase(0x0090_5820);
        call(mem, digits, &[v, tx, ty, dim, scale, 0]);
    }

    /// `MenuItem_DrawListRow(window, x, y, row, mode)` (`0x008c7c30`, own
    /// name; caller `MenuItem_ListWindowDraw_008c7750`): one row of the
    /// item list window. Modes 0 and 2 add `2 * i16 +0x32` (the scroll) to
    /// `row`, mode 1 `2 * i16 +0x34`. By the window's list kind (`+0x43`):
    /// 0 items (`row` below `TOAmapGetItemListMax()`: a kind-4 plate, then
    /// for a count the count and `DrawCrossMenuIconItemName`, dimmed when
    /// the usable byte is 0), 1 key items (`row` below
    /// `TkMenuImportantListMax()`: the id from [`MENU_IMPORTANT_LIST`],
    /// which replaces `row` for the cursor test; when
    /// `TkMsImportantGet(id)`, a kind-7 plate and its name), 2 commands
    /// (`row` below `MenuItem_GetRows()`: text kind 1 is a kind-`0xe`
    /// plate, the command icon shape, its name and its MP cost byte `+0x25`
    /// of `MsGetRomPlyCommand(id, local)`; kind 2 is
    /// `MenuMn2_EmitPacket(ftol(x), ftol(y), 2)`). A negative or too large
    /// `row` returns at once. Then the blinking cursor
    /// `TkMn2DrawStatiCursor(X(170) + x, Y(5) + y, 0)` when the window is
    /// active (`+0x42` 2, `i8 +0x45` positive), its cursor row
    /// `+0x58 + 2 * i16 +0x2c` is `row` and bit 1 of `TkMenuGetTimer()` is
    /// set. `width` stands for the original's argument slot the digit
    /// width goes to, `local` for its command out word.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn menu_item_draw_list_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: u32,
        mode: u32,
        [width, local]: [u32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, ftol, load, round_trip, store, sub};
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let at = |o: u32| window.wrapping_add(o);
        let xy = [round_trip(x).to_bits(), round_trip(y).to_bits()];
        let (xl, yl) = (load(x.to_bits()), load(y.to_bits()));
        let scroll = match mode {
            0 | 2 => Some(0x32),
            1 => Some(0x34),
            _ => None,
        };
        let mut row = row;
        if let Some(o) = scroll {
            let s = i32::from(mem.read::<i16>(at(o))).cast_unsigned();
            row = row.wrapping_add(s.wrapping_mul(2));
        }
        let entry = MENU_ITEM_LIST
            .ptr(mem)
            .addr
            .wrapping_add(row.wrapping_mul(4));
        let in_range = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, f: u32| {
            let f = mem.rebase(f);
            row.cast_signed() >= 0 && row.cast_signed() < call(mem, f, &[]).cast_signed()
        };
        match mem.read::<u8>(at(0x43)) {
            0 => {
                if !in_range(mem, call, 0x008c_8c30) {
                    return;
                }
                let dim = u32::from(mem.read::<u8>(entry.wrapping_add(3)) == 0);
                list_row_plate(mem, xy, 4, remap, call);
                if mem.read::<u8>(entry.wrapping_add(2)) != 0 {
                    list_row_count(mem, [x, y], entry.wrapping_add(2), dim, width, remap, call);
                    let id = u32::from(mem.read::<u16>(entry));
                    let name = fun(mem, 0x008e_6700);
                    call(mem, name, &[id, xy[0], xy[1], dim]);
                }
            }
            1 => {
                if !in_range(mem, call, 0x008c_8a20) {
                    return;
                }
                let keys = MENU_IMPORTANT_LIST.ptr(mem).addr;
                row = u32::from(mem.read::<u16>(keys.wrapping_add(row.wrapping_mul(2))));
                let get = fun(mem, 0x008a_b2c0);
                if row != 0 && call(mem, get, &[row]) != 0 {
                    list_row_plate(mem, xy, 7, remap, call);
                    let name = fun(mem, 0x008f_50f0);
                    call(mem, name, &[row, xy[0], xy[1], 0]);
                }
            }
            2 => {
                if !in_range(mem, call, 0x008c_8a00) {
                    return;
                }
                let dim = u32::from(mem.read::<u8>(entry.wrapping_add(3)) == 0);
                match mem.read::<u8>(entry.wrapping_add(2)) {
                    1 => {
                        let id = u32::from(mem.read::<u16>(entry));
                        let rom = fun(mem, 0x0079_0a90);
                        let rec = call(mem, rom, &[id, local]);
                        list_row_plate(mem, xy, 0xe, remap, call);
                        let h = store(remap(mem, true, 68.0));
                        let w = store(remap(mem, false, 68.0));
                        let sy = store(sub(yl, remap(mem, true, 2.0)));
                        let sx = store(add(remap(mem, false, 602.0), xl));
                        // u0, v0, u1, v1: 0.486, 0.589, 0.560, 0.662
                        let uv = [0x3ef9_0000u32, 0x3f16_c000, 0x3f0f_4000, 0x3f29_8000];
                        let shape = fun(mem, 0x0090_3bb0);
                        call(mem, shape, &[200, sx, sy, w, h, uv[0], uv[1], uv[2], uv[3]]);
                        let id = u32::from(mem.read::<u16>(entry));
                        let name = fun(mem, 0x008f_5000);
                        call(mem, name, &[id, xy[0], xy[1], dim]);
                        let cost = rec.wrapping_add(0x25);
                        if mem.read::<u8>(cost) != 0 {
                            list_row_count(mem, [x, y], cost, dim, width, remap, call);
                        }
                    }
                    2 => {
                        let ty = ftol(mem, yl);
                        let tx = ftol(mem, xl);
                        let emit = fun(mem, 0x008c_11b0);
                        call(mem, emit, &[tx, ty, 2]);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        if mem.read::<u8>(at(0x42)) != 2 || mem.read::<i8>(at(0x45)) < 1 {
            return;
        }
        let cursor = mem.read::<u32>(at(0x58)).wrapping_add(
            i32::from(mem.read::<i16>(at(0x2c)))
                .cast_unsigned()
                .wrapping_mul(2),
        );
        if cursor != row {
            return;
        }
        let timer = fun(mem, 0x008a_9c60);
        if (call(mem, timer, &[]).cast_signed() / 2) & 1 == 0 {
            return;
        }
        let cy = store(add(remap(mem, true, 5.0), yl));
        let cx = store(add(remap(mem, false, 170.0), xl));
        let draw = fun(mem, 0x008c_13d0);
        call(mem, draw, &[cx, cy, 0]);
    }

    /// Set while the item menu lists gear (`TkMenuCtrlItem` after the gear
    /// sort), cleared by the other item lists.
    pub const MENU_ITEM_GEAR_MODE: Global<u8> = Global::new(0x0186_a393);

    /// `MenuItem_DrawGearInfo(window)` (`0x008c9570`, own name; caller
    /// `0x008c87e0`, the item menu's draw): outside gear mode
    /// ([`MENU_ITEM_GEAR_MODE`] 0) only `MenuItem_DrawChrSmallPlate(X(210),
    /// Y(231), -1)`. In gear mode, for the window's row `i16 +0x48` of
    /// [`MENU_ITEM_LIST`] (id `+0`, count byte `+2`): with an id and a
    /// count the record `r = MsGetSaveWeapon(id, local)` and its owner
    /// (byte `+4` when byte `+2` is set, else -1) get the small plate;
    /// then the owner's equipped piece (`TkMenuGetArmor`/`TkMenuGetWeapon`
    /// by byte `+5`, `0xff` none) in a plate (`X(1144), Y(295), X(740),
    /// Y(60)`, kind 2) with sheet cell `0x12` and either its stats
    /// (`0x008d8aa0`), a plate and its name, or status text `0x17` centred
    /// on `X(1514)` (scale 0.78); then the selected one: plate at
    /// `X(970), Y(659)`, cell `0x13` and `0x008d8aa0(X(970), Y(727), r)`.
    /// Every `X`/`Y` is `graphicUiRemapX2`/`Y2`, each y before its x, the
    /// last argument first. Ends with `TkMn2DrawKickSyncPacket2()`, whose
    /// `eax` it returns. `local` is the out word, `width` the text width's.
    pub fn menu_item_draw_gear_info<M: Mem>(
        mem: &mut M,
        window: u32,
        [local, width]: [u32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, mul, store, sub};
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let (small_plate, kick) = (fun(mem, 0x008c_1260), fun(mem, 0x008c_0cd0));
        let (plate, sheet, stats) = (
            fun(mem, 0x008f_5f70),
            fun(mem, 0x008f_8bb0),
            fun(mem, 0x008d_8aa0),
        );
        let weapon = fun(mem, 0x007a_bbd0);
        let mut owner = u32::MAX;
        if MENU_ITEM_GEAR_MODE.get(mem) == 0 {
            let y = store(remap(mem, true, 231.0));
            let x = store(remap(mem, false, 210.0));
            call(mem, small_plate, &[x, y, owner]);
            return call(mem, kick, &[]);
        }
        let row = i32::from(mem.read::<i16>(window.wrapping_add(0x48))).cast_unsigned();
        let entry = MENU_ITEM_LIST
            .ptr(mem)
            .addr
            .wrapping_add(row.wrapping_mul(4));
        let listed =
            |mem: &M| mem.read::<u16>(entry) != 0 && mem.read::<u8>(entry.wrapping_add(2)) != 0;
        let mut rec = 0;
        if listed(mem) {
            let id = u32::from(mem.read::<u16>(entry));
            rec = call(mem, weapon, &[id, local]);
            if mem.read::<u8>(rec.wrapping_add(2)) != 0 {
                owner = u32::from(mem.read::<u8>(rec.wrapping_add(4)));
            }
        }
        let y = store(remap(mem, true, 231.0));
        let x = store(remap(mem, false, 210.0));
        call(mem, small_plate, &[x, y, owner]);
        if !listed(mem) {
            return call(mem, kick, &[]);
        }
        let get = if mem.read::<u8>(rec.wrapping_add(5)) == 0 {
            fun(mem, 0x008a_9c70)
        } else {
            fun(mem, 0x008a_9820)
        };
        let equipped = call(mem, get, &[owner]);
        let worn = if equipped == 0xff {
            0
        } else {
            call(mem, weapon, &[equipped, local])
        };
        let [x, y, w, h] = status_rect(mem, remap, [1144.0, 295.0, 740.0, 60.0], None);
        call(mem, plate, &[x, y, w, h, 2]);
        let [x, y, w, h] = status_rect(mem, remap, [1299.0, 307.0, 430.0, 36.0], None);
        call(mem, sheet, &[0x12, x, y, w, h]);
        let scale = 0.78f32.to_bits();
        let one = 1f32.to_bits();
        if worn == 0 {
            let mes = fun(mem, 0x008b_ee70);
            let text = call(mem, mes, &[0x17, width, 0, scale, one]);
            let measure = fun(mem, 0x0090_5290);
            call(mem, measure, &[text, width, 0, scale, one]);
            let y = store(remap(mem, true, 243.0));
            let right = remap(mem, false, 1514.0);
            let x = store(sub(right, mul(load(mem.read(width)), 0.5)));
            let text = call(mem, mes, &[0x17, x, y, 0, scale, one]);
            let font = fun(mem, 0x0090_5ab0);
            call(mem, font, &[text, x, y, 0, scale, one]);
        } else {
            let y = store(remap(mem, true, 363.0));
            let x = store(remap(mem, false, 1144.0));
            call(mem, stats, &[x, y, worn]);
            let [x, y, w, h] = status_rect(mem, remap, [1164.0, 231.0, 700.0, 64.0], None);
            call(mem, plate, &[x, y, w, h, 9]);
            let y = store(remap(mem, true, 234.0));
            let x = store(remap(mem, false, 1164.0));
            let name = fun(mem, 0x008e_6970);
            call(mem, name, &[worn, x, y, 0]);
        }
        let [x, y, w, h] = status_rect(mem, remap, [970.0, 659.0, 740.0, 60.0], None);
        call(mem, plate, &[x, y, w, h, 2]);
        let [x, y, w, h] = status_rect(mem, remap, [1125.0, 671.0, 430.0, 36.0], None);
        call(mem, sheet, &[0x13, x, y, w, h]);
        let y = store(remap(mem, true, 727.0));
        let x = store(remap(mem, false, 970.0));
        call(mem, stats, &[x, y, rec]);
        call(mem, kick, &[])
    }

    /// Calls the VU0 helper `f` with the two pointers `args` and the four
    /// words at `vec` behind them (what the spline segments push; each
    /// helper reads the component it scales by).
    fn vu0_call<M: Mem>(
        mem: &mut M,
        f: u32,
        [a, b]: [u32; 2],
        vec: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let f = mem.rebase(f);
        let w: [u32; 4] = [0u32, 4, 8, 12].map(|o| mem.read(vec.wrapping_add(o)));
        call(mem, f, &[a, b, w[0], w[1], w[2], w[3]]);
    }

    /// The tangent tail of the spline segments: `dst = src * src`
    /// (`Vec4Scale4`), `dst.x += y`, `+= z` (`pppVu0AddY`/`AddZ`),
    /// `Q = vf00.w / sqrt(dst.x)` (`MsDivSqrt_007c13c0`), `0x00728ab0()`,
    /// `dst = src * Q` (`vfx_vec3_scale`), `dst.w = vf00.w`
    /// (`vfx_vec_w_add`), copied to `out`: `src` normalised.
    fn spline_unit<M: Mem>(
        mem: &mut M,
        out: u32,
        [dst, src]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::round_trip;
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let (dst, src) = (fun(mem, dst), fun(mem, src));
        let copy = |mem: &M, a: u32| round_trip(f32::from_bits(mem.read(a))).to_bits();
        vu0_call(mem, 0x007c_11c0, [dst, src], src, call);
        vu0_call(mem, 0x0074_b630, [dst, dst], dst, call);
        vu0_call(mem, 0x0074_b650, [dst, dst], dst, call);
        let (q_reg, vf00) = (fun(mem, 0x00c8_f788), fun(mem, 0x00c0_a004));
        let (vf00_w, dst_x) = (copy(mem, vf00.wrapping_add(0xc)), copy(mem, dst));
        let f = fun(mem, 0x007c_13c0);
        call(mem, f, &[q_reg, vf00_w, dst_x]);
        let f = fun(mem, 0x0072_8ab0);
        call(mem, f, &[]);
        let scale = copy(mem, q_reg);
        let f = fun(mem, 0x0075_bb00);
        call(mem, f, &[dst, src, scale]);
        vu0_call(mem, 0x0081_7cc0, [dst, vf00], vf00, call);
        let f = fun(mem, 0x0071_0e70);
        call(mem, f, &[dst, out]);
    }

    /// `TkMtCalcSpline3Segment_0088a500(out, index, t, points, position)`
    /// (`0x0088a500`, own name; caller `0x0088aa40`, `TkMtCalcSpline3`'s
    /// body for more than 3 points): the uniform cubic B-spline segment of
    /// the four 16-byte points from `points + 16 * index`. A nonzero
    /// `position` gives the point, weights `((1-t)^3/6, t^3/2 - t^2 + 2/3,
    /// -t^3/2 + t^2/2 + t/2 + 1/6, t^3/6)` (each an `f32`, `t/2` and `t^2`
    /// too), as `vf0x7f0` of `MulX`/`MaddaY`/`MaddaZ`/`MaddW`; 0 the
    /// derivative `(-t^2/2 + t - 1/2, 1.5t^2 - 2t, -1.5t^2 + t + 1/2,
    /// t^2/2)`, normalised. Through the VU0 registers at `0x00c8f790`..
    /// (coefficients `0x00c8f7e0`, points `0x00c8f7a0`.., result
    /// `0x00c8f7f0`, unit `0x00c8f800`). `local` is the coefficients'
    /// 16-byte frame copy.
    #[allow(clippy::too_many_arguments)]
    pub fn tk_mt_calc_spline3_segment<M: Mem>(
        mem: &mut M,
        out: u32,
        index: u32,
        t: f32,
        points: u32,
        position: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, div, load, mul, store, sub};
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let tl = load(t.to_bits());
        let weights = if position == 0 {
            [
                store(sub(add(mul(mul(tl, -0.5), tl), tl), 0.5)),
                store(sub(mul(mul(tl, 1.5), tl), mul(tl, 2.0))),
                store(add(add(mul(mul(tl, -1.5), tl), tl), 0.5)),
                store(mul(mul(tl, 0.5), tl)),
            ]
        } else {
            let a = load(store(sub(1.0, tl)));
            let w0 = store(div(mul(mul(a, a), a), 6.0));
            let tt = load(store(mul(tl, tl)));
            let half = load(store(div(tl, 2.0)));
            let cube = load(store(mul(tt, half)));
            let w1 = store(add(sub(cube, tt), f64::from(2.0f32 / 3.0)));
            let w2 = store(add(
                add(add(mul(cube, -1.0), div(tt, 2.0)), half),
                f64::from(1.0f32 / 6.0),
            ));
            [w0, w1, w2, store(div(cube, 3.0))]
        };
        for (k, w) in (0u32..).zip(weights) {
            mem.write(local.wrapping_add(4 * k), w);
        }
        let points = points.wrapping_add(index.wrapping_mul(16));
        let copy = fun(mem, 0x0071_0e30);
        let coef = fun(mem, 0x00c8_f7e0);
        call(mem, copy, &[coef, local]);
        let vf = fun(mem, 0x00c8_f7a0);
        for k in 0..4u32 {
            call(
                mem,
                copy,
                &[vf.wrapping_add(16 * k), points.wrapping_add(16 * k)],
            );
        }
        let (acc, result) = (fun(mem, 0x00c8_f790), fun(mem, 0x00c8_f7f0));
        vu0_call(mem, 0x0072_c6a0, [acc, vf], coef, call);
        vu0_call(mem, 0x0072_c590, [acc, vf.wrapping_add(0x10)], coef, call);
        vu0_call(mem, 0x0072_c5f0, [acc, vf.wrapping_add(0x20)], coef, call);
        vu0_call(
            mem,
            0x0072_c650,
            [result, vf.wrapping_add(0x30)],
            coef,
            call,
        );
        if position != 0 {
            let to = fun(mem, 0x0071_0e70);
            call(mem, to, &[result, out]);
            return;
        }
        spline_unit(mem, out, [0x00c8_f800, 0x00c8_f7f0], call);
    }

    /// `TkMtCalcSplineSegment_0088abd0(out, index, t, points, position)`
    /// (`0x0088abd0`, own name; caller `0x0088afd0`): the uniform quadratic
    /// B-spline segment of the three points from `points + 16 * index`,
    /// like [`tk_mt_calc_spline3_segment`]: weights `((1-t)^2/2, t(1-t) +
    /// 1/2, t^2/2)` for the point, `(t - 1, 1 - 2t, t)` for the tangent
    /// (normalised), into `0x00c8f7d0` (`MulX`, `MaddaY`,
    /// `Vu0MaddaScalar4` to `0x00c8f7e0`; unit `0x00c8f7f0`). Only three
    /// words of `local` are written: the fourth is copied as it stands
    /// (the original's never-written frame word).
    #[allow(clippy::too_many_arguments)]
    pub fn tk_mt_calc_spline_segment<M: Mem>(
        mem: &mut M,
        out: u32,
        index: u32,
        t: f32,
        points: u32,
        position: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let tl = load(t.to_bits());
        let weights = if position == 0 {
            [
                store(sub(tl, 1.0)),
                store(sub(1.0, mul(tl, 2.0))),
                round_trip(t).to_bits(),
            ]
        } else {
            let a = load(store(sub(1.0, tl)));
            let w0 = store(mul(mul(a, a), 0.5));
            let w2 = store(mul(mul(tl, tl), 0.5));
            [w0, store(add(mul(tl, a), 0.5)), w2]
        };
        for (k, w) in (0u32..).zip(weights) {
            mem.write(local.wrapping_add(4 * k), w);
        }
        let points = points.wrapping_add(index.wrapping_mul(16));
        let copy = fun(mem, 0x0071_0e30);
        let coef = fun(mem, 0x00c8_f7d0);
        call(mem, copy, &[coef, local]);
        let vf = fun(mem, 0x00c8_f7a0);
        for k in 0..3u32 {
            call(
                mem,
                copy,
                &[vf.wrapping_add(16 * k), points.wrapping_add(16 * k)],
            );
        }
        let (acc, result) = (fun(mem, 0x00c8_f790), fun(mem, 0x00c8_f7e0));
        vu0_call(mem, 0x0072_c6a0, [acc, vf], coef, call);
        vu0_call(mem, 0x0072_c590, [acc, vf.wrapping_add(0x10)], coef, call);
        vu0_call(
            mem,
            0x007c_1000,
            [result, vf.wrapping_add(0x20)],
            coef,
            call,
        );
        if position != 0 {
            let to = fun(mem, 0x0071_0e70);
            call(mem, to, &[result, out]);
            return;
        }
        spline_unit(mem, out, [0x00c8_f7f0, 0x00c8_f7e0], call);
    }

    /// The config menu row that shows the PC's own texts
    /// (`SaveDataGetLoc`) instead of `TkMenuConfigMes` ones.
    pub const MENU_CONFIG_LOC_ROW: Global<i32> = Global::new(0x0186_a438);
    /// The config menu's cursor row (`i8`).
    pub const MENU_CONFIG_CURSOR: Global<i8> = Global::new(0x0186_a444);
    /// The config menu's row count (its rows share 740 PS2 pixels).
    pub const MENU_CONFIG_ROWS: Global<i32> = Global::new(0x0186_a448);
    /// The help text the PC row's options leave (`SaveDataGetLoc`).
    pub const MENU_CONFIG_HELP_TEXT: Global<u8> = Global::new(0x0186_a4a0);

    /// `MenuConfig_DrawOptionRow(_, _, row)` (`0x008cbae0`, own name; caller
    /// `0x008cbab0`): one config menu row. With `step = f32(740 /
    /// f32(rows))` and `base = f32(row) * step` (kept at 53 bits) the row
    /// record `GetDerefU32_008cbaa0(row)` (option count `+4`, choice `+8`,
    /// enabled `+0xc`, title `u16 +0x18`, option texts `u16` every 4 bytes
    /// from `+0x1c`) is drawn: the frame `0x008cb530(X(210), Y(base +
    /// 220))`, the title at `X(470), Y(base + 228)` (scale 0.78), the
    /// backing shape, the cross cursor on the cursor row, then each option
    /// centred on `X(750) + k * X(420) + X(210)` (measured at scale 0.68,
    /// drawn at 0.78; dimmed when the row is disabled), the brush on the choice of an enabled row and,
    /// on the cursor row, the choice's help. The PC row
    /// ([`MENU_CONFIG_LOC_ROW`]) takes its texts from `SaveDataGetLoc`
    /// (title `0x1cd`, options `0x1d0, 0x1ce, ..`, help `0x1d1, 0x1cf,
    /// ..` into [`MENU_CONFIG_HELP_TEXT`]). `title`/`option` stand for the
    /// original's two 256-byte text buffers, `width` for its width word.
    #[allow(clippy::too_many_lines)]
    pub fn menu_config_draw_option_row<M: Mem>(
        mem: &mut M,
        row: u32,
        [title, option, width]: [u32; 3],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
        #[allow(clippy::cast_precision_loss)] // fild
        let int = |v: u32| f64::from(v.cast_signed());
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let (loc, font, measure, mes) = (
            fun(mem, 0x0064_12d0),
            fun(mem, 0x0090_5ab0),
            fun(mem, 0x0090_5290),
            fun(mem, 0x008d_c9b0),
        );
        let (big, small, one) = (0.78f32.to_bits(), 0.68f32.to_bits(), 1f32.to_bits());
        let rows = load(store(int(MENU_CONFIG_ROWS.get(mem).cast_unsigned())));
        let step = load(store(div(740.0, rows)));
        let callee = fun(mem, 0x008c_baa0);
        let rec = call(mem, callee, &[row]);
        let base = mul(load(store(int(row))), step);
        let frame_y = store(remap(mem, true, f32::from_bits(store(add(base, 220.0)))));
        let frame_x = store(remap(mem, false, 210.0));
        let callee = fun(mem, 0x008c_b530);
        call(mem, callee, &[frame_x, frame_y]);
        let pc_row = |mem: &M| row == MENU_CONFIG_LOC_ROW.get(mem).cast_unsigned();
        let title_y = store(add(base, 228.0));
        let text = if pc_row(mem) {
            call(mem, loc, &[0x1cd, title]);
            title
        } else {
            0
        };
        let ty = store(remap(mem, true, f32::from_bits(title_y)));
        let tx = store(remap(mem, false, 470.0));
        let text = if text == 0 {
            let id = u32::from(mem.read::<u16>(rec.wrapping_add(0x18)));
            call(mem, mes, &[id])
        } else {
            text
        };
        call(mem, font, &[text, tx, ty, 0, big, one]);
        let h = store(remap(mem, true, 54.0));
        let w = store(remap(mem, false, 554.0));
        let sy = store(remap(mem, true, f32::from_bits(store(add(base, 223.0)))));
        let sx = store(remap(mem, false, 654.0));
        let uv = [1515f32, 170.0, 1959.0, 228.0].map(f32::to_bits);
        let callee = fun(mem, 0x0090_3ee0);
        #[rustfmt::skip]
        call(mem, callee, &[u32::MAX, sx, sy, w, h, uv[0], uv[1], uv[2], uv[3], 0x4080_8080, 0x4080_8080]);
        let cursor_row = |mem: &M| row == i32::from(MENU_CONFIG_CURSOR.get(mem)).cast_unsigned();
        if cursor_row(mem) {
            let cy = store(remap(mem, true, f32::from_bits(store(add(base, 232.0)))));
            let cx = store(remap(mem, false, 440.0));
            let callee = fun(mem, 0x008c_0670);
            call(mem, callee, &[cx, cy, 0]);
        }
        let dim = u32::from(mem.read::<u32>(rec.wrapping_add(0xc)) == 0);
        let ry = load(store(remap(mem, true, f32::from_bits(title_y))));
        let (mut loc_id, mut help_id, mut entry) = (0x1d0u32, 0x1d1u32, rec.wrapping_add(0x1c));
        // the option id the help reads (the original's slot; only the
        // other rows write it)
        let mut id = 0;
        let mut k = 0u32;
        while k.cast_signed() < mem.read::<i32>(rec.wrapping_add(4)) {
            let text = if pc_row(mem) {
                call(mem, loc, &[loc_id, option]);
                option
            } else {
                id = u32::from(mem.read::<u16>(entry));
                0
            };
            let column = remap(mem, false, 420.0);
            let left = mul(column, load(store(int(k))));
            let xk = load(store(add(remap(mem, false, 750.0), left)));
            let text = if text == 0 {
                call(mem, mes, &[id])
            } else {
                text
            };
            call(mem, measure, &[text, width, dim, small, one]);
            let centre = add(remap(mem, false, 210.0), xk);
            let ox = store(sub(centre, mul(load(mem.read(width)), 0.5)));
            // measured at 0.68, drawn at 0.78
            call(mem, font, &[text, ox, store(ry), dim, big, one]);
            let rec_choice = mem.read::<u32>(rec.wrapping_add(8));
            if k == rec_choice && mem.read::<u32>(rec.wrapping_add(0xc)) != 0 {
                let bw = remap(mem, false, 300.0);
                let bw = ftol(mem, bw);
                let by = add(remap(mem, true, 38.0), ry);
                let by = ftol(mem, by);
                let bx = add(remap(mem, false, 60.0), xk);
                let bx = ftol(mem, bx);
                let callee = fun(mem, 0x008b_f830);
                call(mem, callee, &[row, bx, by, bw]);
            }
            if cursor_row(mem) && k == mem.read::<u32>(rec.wrapping_add(8)) {
                let set_help = fun(mem, 0x008a_aeb0);
                if pc_row(mem) {
                    let help = MENU_CONFIG_HELP_TEXT.ptr(mem).addr;
                    call(mem, loc, &[help_id, help]);
                    call(mem, set_help, &[help]);
                } else {
                    let callee = fun(mem, 0x008d_c970);
                    let help = call(mem, callee, &[id]);
                    call(mem, set_help, &[help]);
                }
            }
            loc_id = loc_id.wrapping_sub(2);
            help_id = help_id.wrapping_sub(2);
            entry = entry.wrapping_add(4);
            k += 1;
        }
    }

    /// The pad input of [`tk_mn2_ctrl_sel_items_window`]'s state 3: the
    /// next state, or `None` to return.
    #[allow(clippy::too_many_lines)]
    fn sel_items_input<M: Mem>(
        mem: &mut M,
        w: u32,
        [pad, rep, trg]: [u16; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<u32> {
        let at = |o: u32| w.wrapping_add(o);
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let (snd, draw, ratio) = (
            fun(mem, 0x0088_6e80),
            fun(mem, 0x008d_1a20),
            fun(mem, 0x008a_b270),
        );
        let half = (i32::from(mem.read::<u16>(at(0x30))) + 1) >> 1;
        let pair = i32::from(mem.read::<i16>(at(0x48))) >> 1;
        if trg & 0x20 != 0 {
            let pick: u32 = mem.read(at(0x1c));
            if pick != 0 {
                let cursor = i32::from(mem.read::<i16>(at(0x48))).cast_unsigned();
                let picked = i32::from(mem.read::<i8>(at(0x45))).cast_unsigned();
                let r = call(mem, pick, &[w, cursor, picked]).cast_signed();
                if r < 0 {
                    return None;
                }
                if r == 0 {
                    call(mem, snd, &[0x8000_0003]);
                    return None;
                }
            }
            if mem.read::<u8>(at(0x37)) != 0 {
                call(mem, snd, &[0x8000_0001]);
            }
            mem.write(at(0x28), 0xcu32);
            return None;
        }
        if trg & 0x40 != 0 {
            call(mem, snd, &[0x8000_0004]);
            mem.write(at(0x28), 0xeu32);
            return None;
        }
        if trg & 0x8000 != 0 {
            let cursor = mem.read::<u16>(at(0x48));
            if cursor & 1 != 0 {
                call(mem, snd, &[0x8000_0001]);
                mem.write(at(0x48), mem.read::<u16>(at(0x48)) & 0xfffe);
                call(mem, draw, &[w]);
            }
            return None;
        }
        if trg & 0x2000 != 0 {
            let cursor = mem.read::<u16>(at(0x48));
            if cursor & 1 != 0 {
                return None;
            }
            if i32::from(cursor.cast_signed()) < i32::from(mem.read::<u16>(at(0x30))) - 1 {
                mem.write(at(0x48), cursor | 1);
            } else {
                if pair == 0 {
                    return None;
                }
                mem.write(at(0x48), cursor.wrapping_sub(1));
            }
            call(mem, snd, &[0x8000_0001]);
            call(mem, draw, &[w]);
            return None;
        }
        let top = i32::from(mem.read::<i16>(at(0x32)));
        let visible = i32::from(mem.read::<u16>(at(0x3a)));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the 16-bit stores
        let scroll_to =
            |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, to: i32, next: u32| {
                if to == top {
                    return None;
                }
                mem.write(at(0x34), to as u16);
                call(mem, snd, &[0x8000_0001]);
                call(mem, ratio, &[0]);
                Some(next)
            };
        if rep & 1 != 0 && pad == 1 && pair > 0 {
            return scroll_to(mem, call, top.wrapping_sub(visible).max(0), 6);
        }
        if rep & 2 != 0 && pad == 2 && pair < half - 1 {
            let last = half - visible;
            let to = top + visible;
            return scroll_to(mem, call, if to < last { to } else { last.max(0) }, 10);
        }
        if rep & 0x1000 != 0 && pair > 0 {
            call(mem, snd, &[0x8000_0001]);
            mem.write(at(0x48), mem.read::<u16>(at(0x48)).wrapping_sub(2));
            call(mem, draw, &[w]);
            if pair > i32::from(mem.read::<i16>(at(0x32))) {
                return None;
            }
            call(mem, ratio, &[0]);
            return Some(4);
        }
        if rep & 0x4000 == 0 || pair >= half - 1 {
            return None;
        }
        call(mem, snd, &[0x8000_0001]);
        let cursor = mem.read::<u16>(at(0x48));
        let step = if i32::from(cursor.cast_signed()) < i32::from(mem.read::<u16>(at(0x30))) - 2 {
            2
        } else {
            1
        };
        mem.write(at(0x48), cursor.wrapping_add(step));
        call(mem, draw, &[w]);
        let bottom = i32::from(mem.read::<u16>(at(0x3a))) + i32::from(mem.read::<i16>(at(0x32)));
        if pair + 1 < bottom {
            return None;
        }
        call(mem, ratio, &[0]);
        Some(8)
    }

    /// `TkMn2CtrlSelItemsWindow(w)` (`0x008d0eb0`; Fahrenheit's name for
    /// the Switch function this matches; callers `0x008d0e90`,
    /// `0x008d9df0`): one frame of a two-column selection window's state
    /// machine (`+0x28`, jump table `0x008d13b4`), the pad read once.
    /// The cursor `i16 +0x48` counts entries (two per row: bit 0 is the
    /// column), `u16 +0x30` entries, `i16 +0x32` the top row, `u16 +0x3a`
    /// the visible rows, `+0x46` the scroll timer. 0/1 draw the help
    /// (`TkMn2DrawSelItemsHelp`) and go to 2; 2 waits for a pressed and
    /// repeated button with `MenuHelp_SetRatio2(1)`; 3 reads the pad:
    /// confirm (`0x20`, through the `+0x1c` callback when set) goes to `0xc`,
    /// cancel (`0x40`) to `0xe`, left/right (`0x8000`/`0x2000`) change
    /// the column, page up/down (`1`/`2` with the pad on 1/2) start a page
    /// scroll (6, 10: timer by 0x111), up/down (`0x1000`/`0x4000`) move a
    /// row and past the view start a row scroll (4, 8: timer by 0x333, cut
    /// short by the held direction); the scroll ends (5, 7, 9, 11) move
    /// the cursor with the top, clamp it to the list and go back to 3 in
    /// the same frame with no trigger. `0xc` stores a pick (`+0x2c` slots,
    /// count `i8 +0x45`), `0xe` takes one back.
    #[allow(clippy::too_many_lines)]
    pub fn tk_mn2_ctrl_sel_items_window<M: Mem>(
        mem: &mut M,
        w: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |o: u32| w.wrapping_add(o);
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let read_pad = |mem: &mut M, f: u32, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let f = mem.rebase(f);
            call(mem, f, &[]).to_le_bytes()[..2]
                .try_into()
                .map_or(0, u16::from_le_bytes)
        };
        let pad = read_pad(mem, 0x008b_e410, call);
        let rep = read_pad(mem, 0x008b_e470, call);
        let mut trg = read_pad(mem, 0x008b_e4b0, call);
        let (draw, ratio) = (fun(mem, 0x008d_1a20), fun(mem, 0x008a_b270));
        // a scroll's end: the cursor moves with the top
        let land = |mem: &mut M| {
            let to = mem.read::<u16>(at(0x34));
            mem.write(at(0x46), 0u16);
            let moved = to.wrapping_sub(mem.read::<u16>(at(0x32))).wrapping_mul(2);
            mem.write(at(0x48), mem.read::<u16>(at(0x48)).wrapping_add(moved));
            mem.write(at(0x32), to);
        };
        let clamp = |mem: &mut M| {
            let n = mem.read::<u16>(at(0x30));
            if i32::from(mem.read::<i16>(at(0x48))) >= i32::from(n) {
                mem.write(at(0x48), n.wrapping_sub(1));
            }
        };
        let timer = |mem: &mut M, d: u16| -> i16 {
            let v = mem.read::<u16>(at(0x46)).wrapping_add(d);
            mem.write(at(0x46), v);
            v.cast_signed()
        };
        let mut state: u32 = mem.read(at(0x28));
        loop {
            state = match state {
                0 | 1 => {
                    call(mem, draw, &[w]);
                    mem.write(at(0x28), 2u32);
                    return;
                }
                2 | 3 => {
                    if state == 2 {
                        call(mem, ratio, &[1]);
                        if trg & rep == 0 {
                            return;
                        }
                        mem.write(at(0x28), 3u32);
                    }
                    match sel_items_input(mem, w, [pad, rep, trg], call) {
                        Some(s) => s,
                        None => return,
                    }
                }
                4 | 5 => {
                    if state == 4 {
                        let top = mem.read::<u16>(at(0x32)).wrapping_sub(1);
                        mem.write(at(0x32), top);
                        mem.write(at(0x34), top);
                        mem.write(at(0x46), 0x1000u16);
                        trg = 0;
                        mem.write(at(0x28), 5u32);
                    }
                    if timer(mem, 0xfccd) > 0 && trg & 0x1000 == 0 {
                        return;
                    }
                    land(mem);
                    call(mem, draw, &[w]);
                    call(mem, ratio, &[1]);
                    trg = 0;
                    mem.write(at(0x28), 3u32);
                    3
                }
                6 | 7 => {
                    if state == 6 {
                        mem.write(at(0x46), 0x1000u16);
                        mem.write(at(0x28), 7u32);
                    }
                    if timer(mem, 0xfeef) > 0 {
                        return;
                    }
                    land(mem);
                    call(mem, draw, &[w]);
                    call(mem, ratio, &[1]);
                    trg = 0;
                    mem.write(at(0x28), 3u32);
                    3
                }
                8 | 9 => {
                    if state == 8 {
                        let top = mem.read::<u16>(at(0x32)).wrapping_add(1);
                        mem.write(at(0x32), top);
                        mem.write(at(0x34), top);
                        mem.write(at(0x46), 0xf000u16);
                        trg = 0;
                        mem.write(at(0x28), 9u32);
                    }
                    if timer(mem, 0x333) < 0 && trg & 0x4000 == 0 {
                        return;
                    }
                    land(mem);
                    call(mem, draw, &[w]);
                    clamp(mem);
                    call(mem, ratio, &[1]);
                    trg = 0;
                    mem.write(at(0x28), 3u32);
                    3
                }
                10 | 11 => {
                    if state == 10 {
                        mem.write(at(0x46), 0xf000u16);
                        mem.write(at(0x28), 0xbu32);
                    }
                    if timer(mem, 0x111) < 0 {
                        return;
                    }
                    land(mem);
                    clamp(mem);
                    call(mem, draw, &[w]);
                    trg = 0;
                    call(mem, ratio, &[1]);
                    mem.write(at(0x28), 3u32);
                    3
                }
                0xc => {
                    let n = mem.read::<i8>(at(0x45));
                    let cursor: u16 = mem.read(at(0x48));
                    mem.write(at(0x2c).wrapping_add_signed(2 * i32::from(n)), cursor);
                    mem.write(at(0x45), n.wrapping_add(1));
                    mem.write(at(0x28), 0xdu32);
                    return;
                }
                0xe => {
                    let n = mem.read::<i8>(at(0x45)).wrapping_sub(1);
                    mem.write(at(0x45), n);
                    mem.write(at(0x28), 0xfu32);
                    return;
                }
                _ => return,
            };
        }
    }

    /// The main command window (`TkMenuAppearMainCmdWindow`'s window).
    pub const MENU_MAIN_CMD_WINDOW: Global<u32> = Global::new(0x0187_1540);
    /// The widest option text of the main command window.
    pub const MENU_MAIN_CMD_WIDTH: Global<i32> = Global::new(0x0187_1550);
    /// Cleared by the single-option openings; the menu control waits for
    /// it before freeing the window groups.
    pub const MENU_MAIN_CMD_DONE: Global<u32> = Global::new(0x0187_1554);
    /// The main command window's slide (`0x3000` when opened sliding).
    pub const MENU_MAIN_CMD_SLIDE: Global<u32> = Global::new(0x0187_1558);
    /// Per row: a `u16` delay (`4 * row`) and a `u16` slide.
    pub const MENU_MAIN_ROW_DELAY: Global<[u16; 8]> = Global::new(0x0187_1668);
    /// See [`MENU_MAIN_ROW_DELAY`].
    pub const MENU_MAIN_ROW_SLIDE: Global<[u16; 8]> = Global::new(0x0187_1678);

    /// [`tk_menu_appear_main_cmd_window`]'s options for the other modes:
    /// the scenario mask, slots by `g_TkMenuMainOptionOrder`, compacted.
    fn main_cmd_options<M: Mem>(
        mem: &mut M,
        options: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let f = fun(mem, 0x0086_c470);
        let flag = call(mem, f, &[]).cast_signed();
        let from = |limit: i32| flag == 0 || flag >= limit;
        let mut mask = if from(0xd2) { 0x15 } else { 0x11 };
        mask |= 0x8000;
        if from(0x43) {
            mask |= 8;
        }
        if from(0x448) {
            mask |= 0x80;
        }
        mask |= 0x302;
        if from(0xd9) {
            let f = fun(mem, 0x0086_be50);
            if call(mem, f, &[]) & 0x40 == 0 {
                let f = fun(mem, 0x008a_9b50);
                if call(mem, f, &[]).cast_signed() > 1 {
                    mask |= 0x40;
                }
            }
        }
        let f = fun(mem, 0x008a_b1e0);
        if call(mem, f, &[]) != 0 && from(0xd0) {
            mask |= 0x20;
        }
        // the original's 0x100-byte frame buffer; the slots the game's
        // order table names are all below 32 (larger ones would land
        // on its saved registers)
        let mut slots = [[0u32; 2]; 32];
        let order = fun(mem, 0x00c5_b2d4);
        for i in 0..32u32 {
            if mask & 1 << i == 0 {
                continue;
            }
            let slot = usize::from(mem.read::<u8>(order.wrapping_add(i)));
            let kind = match i {
                1 => 1,
                2..=4 | 15 => 2,
                _ => 0,
            };
            if let Some(s) = slots.get_mut(slot) {
                *s = [i, 1 | kind << 16];
            }
        }
        let mut count = 0u32;
        for [menu, enabled] in slots {
            if enabled & 0xffff == 1 {
                let at = options.wrapping_add(8 * count);
                mem.write(at, menu);
                mem.write(at.wrapping_add(4), enabled);
                count += 1;
            }
        }
        MENU_MAIN_MAX_OPTIONS.set(mem, count);
    }

    /// `TkMenuAppearMainCmdWindow(slide, mode)` (`0x008e1c60`; Fahrenheit's
    /// name): the main menu's options and their window. Modes `0xc`-`0x10`
    /// and `0x18` give one option (menus `0xd`, `0xc`, `0xe`, `0xa`, `0xb`,
    /// `0x37`) and clear [`MENU_MAIN_CMD_DONE`]. Any other mode builds the
    /// option mask from `AtelGetScenarioFlag()` (0 counts as "after
    /// everything"): `0x302 | 0x8000` always, `0x11` before flag `0xd2`
    /// else `0x15`, `8` from `0x43`, `0x80` from `0x448`, `0x40` from
    /// `0xd9` when `AtelGetMenuMask()` lacks bit `0x40` and
    /// `TkMenuGetPlayerListMax2() > 1`, `0x20` from `0xd0` when
    /// `TkMenuSummonEnableMask()`; option bit `i` goes to slot
    /// `byte 0x00c5b2d4[i]` (menu `i`, enabled 1, kind 1 for menu 1, 2
    /// for 2-4 and 15, else 0), and the filled slots in order become
    /// [`MENU_MAIN_OPTIONS`]. Then the help message (`TkMenuSetHelpMessage(0)`
    /// and that of the cursor's option), the slide, the row timers, a
    /// window from `TkMenuMainAllocWindow()` (callbacks `+0xc`
    /// `0x008e2040`, `+0x10` `0x008e20f0`, kind `+0x36` 2, layer `+0x3e`
    /// 7) kept in [`MENU_MAIN_CMD_WINDOW`], the widest option text
    /// (`TkMenuDraw1612Width(TkMenuMainMes(menu))`), and the registration;
    /// returns `TkMenuMainRegistWindow`'s `eax`.
    pub fn tk_menu_appear_main_cmd_window<M: Mem>(
        mem: &mut M,
        slide: u32,
        mode: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let fun = |mem: &M, a: u32| mem.rebase(a);
        let cursor = fun(mem, 0x0187_1548);
        mem.write(cursor, 0u32);
        MENU_MAIN_CMD_WIDTH.set(mem, 0);
        MENU_MAIN_CHOICE.set(mem, -1);
        let options = MENU_MAIN_OPTIONS.ptr(mem).addr;
        let single = match mode {
            0xc => Some(0xd),
            0xd => Some(0xc),
            0xe => Some(0xe),
            0xf => Some(0xa),
            0x10 => Some(0xb),
            0x18 => Some(0x37),
            _ => None,
        };
        if let Some(menu) = single {
            mem.write(options, menu);
            mem.write(options.wrapping_add(4), 1u32);
            MENU_MAIN_MAX_OPTIONS.set(mem, 1);
            MENU_MAIN_CMD_DONE.set(mem, 0);
        } else {
            main_cmd_options(mem, options, call);
        }
        let set_help = fun(mem, 0x008a_aeb0);
        call(mem, set_help, &[0]);
        let get_menu = fun(mem, 0x008e_2270);
        let c: u32 = mem.read(cursor);
        let menu = call(mem, get_menu, &[c]);
        let f = fun(mem, 0x008d_df60);
        let help = call(mem, f, &[menu]);
        call(mem, set_help, &[help]);
        MENU_MAIN_CMD_SLIDE.set(mem, if slide == 0 { 0 } else { 0x3000 });
        let (delay, row_slide) = (
            MENU_MAIN_ROW_DELAY.ptr(mem).addr,
            MENU_MAIN_ROW_SLIDE.ptr(mem).addr,
        );
        for c in 0..8u16 {
            mem.write(delay.wrapping_add(2 * u32::from(c)), c * 4);
            mem.write(row_slide.wrapping_add(2 * u32::from(c)), 0u16);
        }
        let f = fun(mem, 0x008a_a1a0);
        let w = call(mem, f, &[]);
        MENU_MAIN_CMD_WINDOW.set(mem, w);
        let (ctrl, draw) = (fun(mem, 0x008e_2040), fun(mem, 0x008e_20f0));
        for (o, v) in [
            (8u32, 0),
            (0xc, ctrl),
            (0x10, draw),
            (0x14, 0),
            (0x18, 0),
            (0x1c, 0),
        ] {
            mem.write(w.wrapping_add(o), v);
        }
        mem.write(w.wrapping_add(0x36), 2u8);
        mem.write(w.wrapping_add(0x3e), 7u8);
        let max = fun(mem, 0x008e_2280);
        let (mes, width) = (fun(mem, 0x008d_df80), fun(mem, 0x008d_c9e0));
        let mut i = 0u32;
        while i.cast_signed() < call(mem, max, &[]).cast_signed() {
            let menu = call(mem, get_menu, &[i]);
            let text = call(mem, mes, &[menu]);
            let wd = call(mem, width, &[text]).cast_signed();
            if wd > MENU_MAIN_CMD_WIDTH.get(mem) {
                MENU_MAIN_CMD_WIDTH.set(mem, wd);
            }
            i += 1;
        }
        let f = fun(mem, 0x008a_ab00);
        call(mem, f, &[w])
    }

    /// `MenuItem_SortGearByOwner()` (`0x008c9fb0`, own name; caller
    /// `TkMenuCtrlItem`, which reads the result): orders the gear list by
    /// owner. With `n = TkMn2GetNameModeMax()`, `owners[i]` is the byte
    /// `+4` of `MsGetSaveWeapon(id_i, &local)` for the first `n` entries
    /// of [`MENU_ITEM_LIST`]. For owners 0..7 in turn the group of
    /// `weapons + armour` entries (`u16` pairs at `0x0186a3e4`) starting
    /// after the previous groups is walked; an entry of another owner is
    /// swapped (`SwapSaveGear(id, id_j)`, and the two owner bytes) with
    /// `j = Menu_FindByte(owners, owner, from, n)`, `from` first the
    /// group's end and after each swap `j + 1`. Returns 1, or 0 when a
    /// search finds nothing. `owners` is the 0x400-byte buffer (the
    /// original's frame; bytes past `n` are whatever it holds), `local`
    /// `MsGetSaveWeapon`'s out word.
    pub fn menu_item_sort_gear_by_owner<M: Mem>(
        mem: &mut M,
        owners: u32,
        local: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let rebase = |mem: &M, a: u32| mem.rebase(a);
        let list = MENU_ITEM_LIST.ptr(mem).addr;
        let id = |mem: &M, i: u32| u32::from(mem.read::<u16>(list.wrapping_add(i.wrapping_mul(4))));
        let name_max = rebase(mem, 0x008c_1bc0);
        let n = call(mem, name_max, &[]).cast_signed();
        let weapon = rebase(mem, 0x007a_bbd0);
        for i in 0..n.max(0).cast_unsigned() {
            let rec = call(mem, weapon, &[id(mem, i), local]);
            let owner = mem.read::<u8>(rec.wrapping_add(4));
            mem.write(owners.wrapping_add(i), owner);
        }
        let counts = rebase(mem, 0x0186_a3e4);
        let (find, swap) = (rebase(mem, 0x008c_9bc0), rebase(mem, 0x007a_b9f0));
        let mut base = 0u32;
        for owner in 0..7u32 {
            let pair = counts.wrapping_add(owner * 4);
            let count = u32::from(mem.read::<u16>(pair)) + u32::from(mem.read::<u16>(pair + 2));
            let end = base.wrapping_add(count);
            let mut searched = end.wrapping_sub(1);
            for k in 0..count {
                let slot = owners.wrapping_add(base).wrapping_add(k);
                let had = mem.read::<u8>(slot);
                if u32::from(had) == owner {
                    continue;
                }
                let from = searched.wrapping_add(1);
                let j = call(mem, find, &[owners, owner, from, n.cast_unsigned()]);
                if j.cast_signed() < 0 {
                    return 0;
                }
                let (mine, theirs) = (id(mem, base.wrapping_add(k)), id(mem, j));
                call(mem, swap, &[mine, theirs]);
                let other = owners.wrapping_add(j);
                let moved = mem.read::<u8>(other);
                mem.write(slot, moved);
                mem.write(other, had);
                searched = j;
            }
            base = end;
        }
        1
    }

    /// The plate and the price backing of a shop row (`MenuShop_DrawRow`,
    /// `MenuWShop_DrawBuyRow`, `MenuWShop_DrawSellRow`):
    /// `TODrawMenuPlateXYWHType(x, y, f32(remapX2(740)), f32(remapY2(60)),
    /// 5)`, then `TOMkpShapeXYWHUVC2(-1, f32(remapX2(460) + x),
    /// f32(remapY2(3) + y), f32(remapX2(250)), f32(remapY2(54)), 1515, 170,
    /// 1715, 228, 0x80808080, 0x808080)` (each y remapped before its x).
    fn shop_row_frame<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, round_trip, store};
        let (xb, yb) = (round_trip(x).to_bits(), round_trip(y).to_bits());
        let height = store(remap(mem, true, 60.0));
        let width = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[xb, yb, width, height, 5]);
        let height = store(remap(mem, true, 54.0));
        let width = store(remap(mem, false, 250.0));
        let top = store(add(remap(mem, true, 3.0), load(yb)));
        let left = store(add(remap(mem, false, 460.0), load(xb)));
        let uv = [1515f32, 170.0, 1715.0, 228.0].map(f32::to_bits);
        let shape = mem.rebase(0x0090_3ee0);
        #[rustfmt::skip]
        call(mem, shape, &[0xffff_ffff, left, top, width, height, uv[0], uv[1], uv[2], uv[3], 0x8080_8080, 0x0080_8080]);
    }

    /// A shop row's price: `ToMakeBtlEasyDigitRight(price, tx, ty, 0x25,
    /// 0.78, 0.0)` at `tx = f32(remapX2(686) + x)`, `ty = f32(remapY2(9) +
    /// y)` (y remapped first).
    fn shop_row_price<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        price: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, store};
        let ty = store(add(remap(mem, true, 9.0), load(y.to_bits())));
        let tx = store(add(remap(mem, false, 686.0), load(x.to_bits())));
        let digits = mem.rebase(0x0090_55c0);
        call(mem, digits, &[price, tx, ty, 0x25, 0.78f32.to_bits(), 0]);
    }

    /// A weapon shop row's name: `DrawCrossMenuIconWeaponName2(rec,
    /// f32(x - remapX2(50)), y, grey)`.
    fn wshop_row_name<M: Mem>(
        mem: &mut M,
        [x, y]: [f32; 2],
        rec: u32,
        grey: bool,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{load, round_trip, store, sub};
        let yb = round_trip(y).to_bits();
        let left = store(sub(load(x.to_bits()), remap(mem, false, 50.0)));
        let name = mem.rebase(0x008e_6970);
        call(mem, name, &[rec, left, yb, u32::from(grey)])
    }

    /// `MenuShop_DrawRow(window, x, y, row, ids)` (`0x008dac70`, own name;
    /// caller `0x008dabf0`; `x`, `y` `f32`): for `row` within the `u16`
    /// count at `window + 0x30`, item `id = u16 ids[1 + row]` with `price =
    /// MenuShop_Price(id)` (asked first): the shop row frame, the price,
    /// and `DrawCrossMenuIconItemName(id, x, y, !MenuShop_CanBuy(id))`,
    /// whose `eax` is returned. Out of range: the count when `row` is past
    /// it (the original's `eax`), 0 for a negative row (the original
    /// leaves its caller's `eax`; no caller reads it).
    pub fn menu_shop_draw_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::round_trip;
        let Some(id) = shop_row_id(mem, window, row, ids) else {
            return shop_row_out(mem, window, row);
        };
        let f = mem.rebase(0x008d_b3b0);
        let price = call(mem, f, &[id]);
        shop_row_frame(mem, [x, y], remap, call);
        shop_row_price(mem, [x, y], price, remap, call);
        let f = mem.rebase(0x008d_ab30);
        let grey = call(mem, f, &[id]) == 0;
        let (xb, yb) = (round_trip(x).to_bits(), round_trip(y).to_bits());
        let f = mem.rebase(0x008e_6700);
        call(mem, f, &[id, xb, yb, u32::from(grey)])
    }

    /// The `u16 ids[1 + row]` of a shop row in range (`0 <= row <` the
    /// `u16` at `window + 0x30`).
    fn shop_row_id(mem: &impl Mem, window: u32, row: i32, ids: u32) -> Option<u32> {
        let count = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= count {
            return None;
        }
        let at = ids.wrapping_add(2).wrapping_add(row.cast_unsigned() * 2);
        Some(u32::from(mem.read::<u16>(at)))
    }

    /// What a shop row out of range leaves in `eax`: the count when `row`
    /// is past it, 0 for a negative row.
    fn shop_row_out(mem: &impl Mem, window: u32, row: i32) -> u32 {
        if row < 0 {
            0
        } else {
            u32::from(mem.read::<u16>(window.wrapping_add(0x30)))
        }
    }

    /// `MenuWShop_DrawBuyRow(window, x, y, row, ids)` (`0x008d8cb0`, own
    /// name; caller `0x008d8c30`): as [`menu_shop_draw_row`] for a weapon
    /// shop entry: `rec = MenuWShop_GetRecord(id)`, `price =
    /// MenuShop_WeaponPrice(rec)`, the frame and price, then the name
    /// greyed when `MenuWShop_CanAfford(rec)` is 0. Returns the name
    /// call's `eax`.
    pub fn menu_wshop_draw_buy_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let Some(id) = shop_row_id(mem, window, row, ids) else {
            return shop_row_out(mem, window, row);
        };
        let f = mem.rebase(0x008d_9170);
        let rec = call(mem, f, &[id]);
        let f = mem.rebase(0x008d_9370);
        let price = call(mem, f, &[rec]);
        shop_row_frame(mem, [x, y], remap, call);
        shop_row_price(mem, [x, y], price, remap, call);
        let f = mem.rebase(0x008d_85d0);
        let grey = call(mem, f, &[rec]) == 0;
        wshop_row_name(mem, [x, y], rec, grey, remap, call)
    }

    /// `MenuWShop_DrawSellRow(window, x, y, row)` (`0x008d8f60`, own name;
    /// caller `0x008d8ee0`): for `row` in range, the shop row frame; then,
    /// unless the [`MENU_ITEM_LIST`] entry's id is 0 (returns 0), `rec =
    /// MsGetSaveWeapon(id, &local)`, its price
    /// `MenuShop_WeaponSellPrice(rec)` and the name, greyed when the
    /// entry's byte `+3` is 0. Returns the name call's `eax`; out of range
    /// as [`menu_shop_draw_row`].
    pub fn menu_wshop_draw_sell_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        local: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let count = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= count {
            return shop_row_out(mem, window, row);
        }
        shop_row_frame(mem, [x, y], remap, call);
        let entry = MENU_ITEM_LIST
            .ptr(mem)
            .addr
            .wrapping_add(row.cast_unsigned() * 4);
        let id = u32::from(mem.read::<u16>(entry));
        if id == 0 {
            return 0;
        }
        let f = mem.rebase(0x007a_bbd0);
        let rec = call(mem, f, &[id, local]);
        let f = mem.rebase(0x008d_9410);
        let price = call(mem, f, &[rec]);
        shop_row_price(mem, [x, y], price, remap, call);
        let grey = mem.read::<u8>(entry.wrapping_add(3)) == 0;
        wshop_row_name(mem, [x, y], rec, grey, remap, call)
    }

    /// `MenuSummon_DrawGrowRow(window, x, y, row, fixed)` (`0x008ce5c0`,
    /// own name; caller `0x008ce520`; `x`, `y` `f32`): an aeon stat row.
    /// Unless `fixed`, `y` scrolls first: `f32(y - remapY2(70) * i16
    /// window[0x46] / 4096)`. For `row` within the `u16` count at `+0x30`:
    /// the plate `TODrawMenuPlateXYWHType(x, y, f32(remapX2(740)),
    /// f32(remapY2(54)), 0xe)`; `grey` when bit `row` of
    /// `g_TkMenuSummonGrowMask` is clear; the name
    /// `ToMakeBtlEasyFont(TOMenuGetSummonMes(name), f32(remapX2(200) + x),
    /// f32(remapY2(6) + y), grey, 0.78, 1.0)` and the value
    /// `ToMakeBtlEasyDigitRight(MenuSummon_GetStat(
    /// MsGetSavePlayerPtr(TkMenuGetCurrentSummon()), desc, 1),
    /// f32(remapX2(650) + x), f32(remapY2(6) + y), grey, 0.68, 0.0)`, with
    /// `desc` the stat's 0x1c-byte record at `0x00c5a608` (`name` its word
    /// `+4`). Returns the digits' `eax`; out of range the count, or 0 for
    /// a negative row (the original's `eax` there is its caller's, or the
    /// scroll word when it scrolled; no caller reads it).
    pub fn menu_summon_draw_grow_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        fixed: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        let mut yb = y.to_bits();
        if fixed == 0 {
            let top = load(yb);
            let step = remap(mem, true, 70.0);
            let scroll = f64::from(mem.read::<i16>(window.wrapping_add(0x46)));
            yb = store(sub(top, mul(mul(step, scroll), 1.0 / 4096.0)));
        }
        let count = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= count {
            return shop_row_out(mem, window, row);
        }
        let xb = round_trip(x).to_bits();
        let yb = round_trip(f32::from_bits(yb)).to_bits();
        let height = store(remap(mem, true, 54.0));
        let width = store(remap(mem, false, 740.0));
        let plate = mem.rebase(0x008f_5f70);
        call(mem, plate, &[xb, yb, width, height, 0xe]);
        let mask = mem.read::<u32>(mem.rebase(0x0186_a600));
        let grey = u32::from(mask & (1u32 << (row.cast_unsigned() & 0x1f)) == 0);
        let desc = mem
            .rebase(0x00c5_a608)
            .wrapping_add(row.cast_unsigned() * 0x1c);
        let ty = store(add(remap(mem, true, 6.0), load(yb)));
        let tx = store(add(remap(mem, false, 200.0), load(xb)));
        let f = mem.rebase(0x008b_eeb0);
        let name = mem.read::<u32>(desc.wrapping_add(4));
        let text = call(mem, f, &[name]);
        let font = mem.rebase(0x0090_5ab0);
        call(
            mem,
            font,
            &[text, tx, ty, grey, 0.78f32.to_bits(), 1f32.to_bits()],
        );
        let f = mem.rebase(0x008a_9880);
        let summon = call(mem, f, &[]);
        let f = mem.rebase(0x0078_5330);
        let player = call(mem, f, &[summon]);
        let f = mem.rebase(0x008c_eca0);
        let value = call(mem, f, &[player, desc, 1]);
        let ty = store(add(remap(mem, true, 6.0), load(yb)));
        let tx = store(add(remap(mem, false, 650.0), load(xb)));
        let digits = mem.rebase(0x0090_55c0);
        call(mem, digits, &[value, tx, ty, grey, 0.68f32.to_bits(), 0])
    }
    /// `TkScanControler()` (`0x008893e0`, Fahrenheit's name; callers
    /// `Sg_MainLoop`, `0x00821e80`, `TOControlBtlWindow`): `SaveDataIsBusy()`
    /// through its thunk `0x00822560` (result unused), then for pads 0
    /// and 1 `Pad_UpdateFrame(pad, 0, TkGetControlerWork2(pad, 0))` (the
    /// third word is unused by the callee). Returns the last
    /// `Pad_UpdateFrame`'s `eax`.
    pub fn tk_scan_controler<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let busy = mem.rebase(0x0082_2560);
        call(mem, busy, &[]);
        let (work2, update) = (mem.rebase(0x0088_8e10), mem.rebase(0x0088_9790));
        let mut eax = 0;
        for pad in 0..2 {
            let w = call(mem, work2, &[pad, 0]);
            eax = call(mem, update, &[pad, 0, w]);
        }
        eax
    }

    /// Shop rows (`0x008d8c30`, `0x008d8ee0`: 9, `0x008dabf0`: 10; three
    /// copies that differ in `row` and the count): `x' =
    /// f32(graphicUiRemapX2(210) + x)` once, `y'` from `y` and after each
    /// row `f32(graphicUiRemapY2(77) + y')`, `row(window, x', y', first +
    /// i, ids)` for `i` in `0..rows`.
    #[allow(clippy::too_many_arguments)]
    fn shop_draw_rows<M: Mem>(
        mem: &mut M,
        [row, rows]: [u32; 2],
        window: u32,
        first: u32,
        [x, y]: [f32; 2],
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, round_trip, store};
        let row = mem.rebase(row);
        let left = store(add(remap(mem, false, 210.0), load(x.to_bits())));
        let mut top = round_trip(y).to_bits();
        for i in 0..rows {
            call(mem, row, &[window, left, top, first.wrapping_add(i), ids]);
            top = store(add(remap(mem, true, 77.0), load(top)));
        }
    }

    /// `MenuWShop_DrawBuyRows(window, _, first, x, y, ids)` (`0x008d8c30`,
    /// own name; `x`, `y` `f32`): [`shop_draw_rows`] of
    /// `MenuWShop_DrawBuyRow`. No caller reads `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn menu_wshop_draw_buy_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        first: u32,
        xy: [f32; 2],
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        shop_draw_rows(mem, [0x008d_8cb0, 9], window, first, xy, ids, remap, call);
    }

    /// `MenuWShop_DrawSellRows(window, _, first, x, y, ids)` (`0x008d8ee0`,
    /// own name): [`shop_draw_rows`] of `MenuWShop_DrawSellRow` (which
    /// ignores `ids`).
    #[allow(clippy::too_many_arguments)]
    pub fn menu_wshop_draw_sell_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        first: u32,
        xy: [f32; 2],
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        shop_draw_rows(mem, [0x008d_8f60, 9], window, first, xy, ids, remap, call);
    }

    /// `MenuShop_DrawRows(window, _, first, x, y, ids)` (`0x008dabf0`, own
    /// name): [`shop_draw_rows`] of `MenuShop_DrawRow`, ten of them.
    #[allow(clippy::too_many_arguments)]
    pub fn menu_shop_draw_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        first: u32,
        xy: [f32; 2],
        ids: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        shop_draw_rows(mem, [0x008d_ac70, 10], window, first, xy, ids, remap, call);
    }

    /// `MenuSummon_DrawGrowRows(window, fixed, first, x, y)` (`0x008ce520`,
    /// own name; caller `0x008ce750`; `x`, `y` integers): eleven
    /// `MenuSummon_DrawGrowRow(window, x', y', first + i, fixed)` for `i`
    /// in `-1..10`, `x' = f32(remapX2(210) + f32(x))`, `y'` from
    /// `f32(remapY2(300) + f32(y))` and after each row `f32(remapY2(70) +
    /// y')`. No caller reads `eax`.
    pub fn menu_summon_draw_grow_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        fixed: u32,
        first: u32,
        [x, y]: [i32; 2],
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, store};
        let int = |v: i32| load(store(f64::from(v)));
        let left = store(add(remap(mem, false, 210.0), int(x)));
        let mut top = store(add(remap(mem, true, 300.0), int(y)));
        let row = mem.rebase(0x008c_e5c0);
        for i in -1..10i32 {
            let r = first.wrapping_add(i.cast_unsigned());
            call(mem, row, &[window, left, top, r, fixed]);
            top = store(add(remap(mem, true, 70.0), load(top)));
        }
    }

    /// `MenuSelList_DrawCursor(window)` (`0x008d15a0`, own name; caller
    /// `0x008d1400`): `TkMn2DrawCrossCursor(x, y, 0)` with `x =
    /// MenuSelList_CursorX(window)` (asked last, through `cursor_x`) and
    /// `y = f32(remapY2(493) + remapY2(77) * f32(v))`, with
    /// `v = (i16 +0x48 >> 1) - i16 +0x32` (the cursor's row on screen).
    /// Returns the cursor call's `eax`.
    pub fn menu_sel_list_draw_cursor<M: Mem>(
        mem: &mut M,
        window: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        cursor_x: &mut impl FnMut(&mut M, u32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        let half = i32::from(mem.read::<i16>(window.wrapping_add(0x48))) >> 1;
        let step = remap(mem, true, 77.0);
        let v = half.wrapping_sub(i32::from(mem.read::<i16>(window.wrapping_add(0x32))));
        let offset = mul(step, load(store(f64::from(v))));
        let y = store(add(remap(mem, true, 493.0), offset));
        let x = store(cursor_x(mem, window));
        let f = mem.rebase(0x008c_0670);
        call(mem, f, &[x, y, 0])
    }
    /// The four remaps of a status plate in the original's order (`h`, `w`,
    /// `y`, `x`), each stored as `f32`; `dx`, when given, added to `x`
    /// before its store (`[x, y, w, h]` bits).
    fn status_rect<M: Mem>(
        mem: &mut M,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        [x, y, w, h]: [f32; 4],
        dx: Option<f64>,
    ) -> [u32; 4] {
        use ffx_bridge::x87::{add, store};
        let h = store(remap(mem, true, h));
        let w = store(remap(mem, false, w));
        let y = store(remap(mem, true, y));
        let x = remap(mem, false, x);
        [store(dx.map_or(x, |d| add(x, d))), y, w, h]
    }

    /// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, uv.., c, c)` with the four
    /// [`status_rect`] remaps of `xywh` (`x` moved by `dx`).
    #[allow(clippy::too_many_arguments)]
    fn status_shape<M: Mem>(
        mem: &mut M,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        shape: u32,
        xywh: [f32; 4],
        dx: f64,
        uv: [f32; 4],
        colour: u32,
    ) -> u32 {
        let [x, y, w, h] = status_rect(mem, remap, xywh, Some(dx));
        let [u0, v0, u1, v1] = uv.map(f32::to_bits);
        let draw = mem.rebase(0x0090_3ee0);
        call(
            mem,
            draw,
            &[shape, x, y, w, h, u0, v0, u1, v1, colour, colour],
        )
    }

    /// The status windows' grey plate texture rectangle (`TOMkpShapeXYWHUVC2`
    /// u0, v0, u1, v1) and colour.
    const STATUS_PLATE_UV: [f32; 4] = [1515.0, 170.0, 1959.0, 228.0];
    const STATUS_PLATE_COLOUR: u32 = 0x4080_8080;
    /// `ToGetBtlEasyFontWidth` / `ToMakeBtlEasyFont` scale pair (0.78, 1).
    const EASY_FONT_SCALE: [u32; 2] = [0x3f47_ae14, 0x3f80_0000];

    /// `TkMn2DrawOverDrive(x, y, chr)` (`0x008d3d70`, Fahrenheit's name;
    /// called through `MenuStatus_DrawPage`'s jump table, pages 0 and 3):
    /// the Overdrive panel at `dx = x - i16 0x0186a888` (`dy = y -
    /// i16 0x0186a88a`). For `chr < 8` (signed) a type-`0xd` plate and two
    /// shapes (-1 and 200); otherwise `MenuStatus_OffsetRect(dx, dy,
    /// plate, 0x0186a888, 1)` with the plate's word `+8` replaced by
    /// `0x0186a8ac`, drawn by `TkMn2DrawCommonPlateL(plate)`. Then shapes
    /// -1 and 400, `TkMn2DrawCrossLimitParam(x, y, w, h, TkMenuGetLimit(chr),
    /// TkMenuGetLimitMax(chr))` (max asked first) and, for `chr < 8`, the
    /// Overdrive mode's name `TkMenuMenu1Mes(TkMn2GetOverDriveId(chr) +
    /// 0x1033)` through `ToMakeBtlEasyFont`. Every remap pair runs `h`,
    /// `w`, `y`, `x`, `x` plus `f32(dx)`. `plate` is the original's 24-byte
    /// frame rectangle. Returns the last call's `eax`.
    pub fn tk_mn2_draw_over_drive<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        chr: i32,
        plate: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store};
        let rect = mem.rebase(0x0186_a888);
        let dx = x.wrapping_sub(i32::from(mem.read::<i16>(rect)));
        let dy = y.wrapping_sub(i32::from(mem.read::<i16>(rect.wrapping_add(2))));
        let xf = load(store(f64::from(dx)));
        if chr < 8 {
            let [px, py, pw, ph] = status_rect(mem, remap, [470.0, 282.0, 1240.0, 60.0], Some(xf));
            let draw_plate = mem.rebase(0x008f_5f70);
            call(mem, draw_plate, &[px, py, pw, ph, 0xd]);
            #[rustfmt::skip]
            status_shape(mem, remap, call, u32::MAX, [1036.0, 285.0, 421.0, 55.0], xf, STATUS_PLATE_UV, STATUS_PLATE_COLOUR);
            // texels of the 1024-wide sheet
            let uv = [364.0, 181.0, 758.0, 209.0].map(|v: f32| v / 1024.0);
            status_shape(
                mem,
                remap,
                call,
                200,
                [1078.0, 289.0, 394.0, 28.0],
                xf,
                uv,
                0x8080_8080,
            );
        } else {
            let offset_rect = mem.rebase(0x008d_3220);
            #[rustfmt::skip]
            call(mem, offset_rect, &[dx.cast_unsigned(), dy.cast_unsigned(), plate, rect, 1]);
            let w = mem.read::<u32>(mem.rebase(0x0186_a8ac));
            mem.write(plate.wrapping_add(8), w);
            let common = mem.rebase(0x008b_f9b0);
            call(mem, common, &[plate]);
        }
        let uv = [274.0, 61.0, 606.0, 93.0];
        status_shape(
            mem,
            remap,
            call,
            u32::MAX,
            [569.0, 290.0, 332.0, 32.0],
            xf,
            uv,
            0x8080_8080,
        );
        let uv = [64.0, 467.0, 343.0, 504.0].map(|v: f32| v / 1024.0);
        status_shape(
            mem,
            remap,
            call,
            400,
            [603.0, 294.0, 201.0, 24.0],
            xf,
            uv,
            0x8080_8080,
        );
        let chr_word = chr.cast_unsigned();
        let (limit_max, limit) = (mem.rebase(0x008a_9950), mem.rebase(0x008a_9930));
        let max = call(mem, limit_max, &[chr_word]);
        let lim = call(mem, limit, &[chr_word]);
        let [lx, ly, lw, lh] = status_rect(mem, remap, [598.0, 326.0, 450.0, 8.0], Some(xf));
        let cross = mem.rebase(0x008d_3690);
        let mut eax = call(mem, cross, &[lx, ly, lw, lh, lim, max]);
        if chr < 8 {
            let ty = store(remap(mem, true, 298.0));
            let tx = store(add(remap(mem, false, 1454.0), xf));
            let over_drive_id = mem.rebase(0x008c_1bd0);
            let id = call(mem, over_drive_id, &[chr_word]);
            let mes = mem.rebase(0x008d_dfc0);
            let text = call(mem, mes, &[id.wrapping_add(0x1033)]);
            let font = mem.rebase(0x0090_5ab0);
            let [s0, s1] = EASY_FONT_SCALE;
            eax = call(mem, font, &[text, tx, ty, 0, s0, s1]);
        }
        eax
    }

    /// `MenuStatus_DrawLayoutRow(x, y, chr)` (`0x008c11e0`, own name;
    /// `MenuStatus_DrawPage`'s page 2): when `chr`'s bit (`1 << (chr &
    /// 31)`) is set in `TOMenuGetYesNoWindow()`, `TkMn2DrawRect(i16
    /// 0x023cc144 + x, i16 0x023cc146 + y, 0x200, i16 0x023cc13e, u32
    /// 0x023cc148, u32 0x023cc140)`; then the draw cursor ([`MN2_DRAW_Y`])
    /// steps by 16 and becomes `TOMenuDrawChrLayoutPlate(cursor, chr, x,
    /// y)` (returned).
    pub fn menu_status_draw_layout_row<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let yes_no = mem.rebase(0x008a_9b60);
        let mask = call(mem, yes_no, &[]);
        if (1u32 << (chr & 31)) & mask != 0 {
            let at = |mem: &M, a: u32| mem.rebase(a);
            let half = |mem: &M, a: u32| i32::from(mem.read::<i16>(at(mem, a)));
            let left = half(mem, 0x023c_c144).wrapping_add(x).cast_unsigned();
            let top = half(mem, 0x023c_c146).wrapping_add(y).cast_unsigned();
            let kind = half(mem, 0x023c_c13e).cast_unsigned();
            let a = mem.read::<u32>(at(mem, 0x023c_c148));
            let b = mem.read::<u32>(at(mem, 0x023c_c140));
            let draw_rect = at(mem, 0x008c_1280);
            call(mem, draw_rect, &[left, top, 0x200, kind, a, b]);
        }
        let cursor = MN2_DRAW_Y.get(mem).wrapping_add(0x10);
        MN2_DRAW_Y.set(mem, cursor);
        let layout = mem.rebase(0x008f_0a70);
        let end = call(
            mem,
            layout,
            &[cursor, chr, x.cast_unsigned(), y.cast_unsigned()],
        );
        MN2_DRAW_Y.set(mem, end);
        end
    }

    /// `MenuStatus_DrawEquipPlate(chr, x)` (`0x008d4540`, own name;
    /// `MenuStatus_DrawPage`'s page 0): two type-`0xd` plates with their
    /// grey shapes (rows 352/355 and 422/425), the captions
    /// `TOMenuGetStatusMes(5)` and `(6)` right-aligned at `remapX2(1030)`
    /// (each asked once for `ToGetBtlEasyFontWidth` into `width` and again
    /// for `ToMakeBtlEasyFont`), then `DrawCrossMenuIconWeaponName` of
    /// `TkMenuGetWeapon(chr)` and of `TkMenuGetArmor(chr)`, or for armour
    /// `0xff` the caption `TOMenuGetStatusMes(0x17)`. All `x` coordinates
    /// add `f32(x)`. Returns the last call's `eax`.
    pub fn menu_status_draw_equip_plate<M: Mem>(
        mem: &mut M,
        chr: u32,
        x: i32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        let xf = load(store(f64::from(x)));
        let draw_plate = mem.rebase(0x008f_5f70);
        for (plate_y, shape_y) in [(352.0, 355.0), (422.0, 425.0)] {
            let [px, py, pw, ph] =
                status_rect(mem, remap, [470.0, plate_y, 1240.0, 60.0], Some(xf));
            call(mem, draw_plate, &[px, py, pw, ph, 0xd]);
            #[rustfmt::skip]
            status_shape(mem, remap, call, u32::MAX, [1036.0, shape_y, 421.0, 55.0], xf, STATUS_PLATE_UV, STATUS_PLATE_COLOUR);
        }
        let mes = mem.rebase(0x008b_ee70);
        let (font_width, font) = (mem.rebase(0x0090_5290), mem.rebase(0x0090_5ab0));
        let [s0, s1] = EASY_FONT_SCALE;
        for (id, row) in [(5, 361.0), (6, 431.0)] {
            let text = call(mem, mes, &[id]);
            call(mem, font_width, &[text, width, 0, s0, s1]);
            let w = mem.read::<u32>(width);
            let ty = store(remap(mem, true, row));
            let tx = store(sub(add(remap(mem, false, 1030.0), xf), load(w)));
            let text = call(mem, mes, &[id]);
            call(mem, font, &[text, tx, ty, 0, s0, s1]);
        }
        let icon_name = mem.rebase(0x008e_67f0);
        let weapon = mem.rebase(0x008a_9c70);
        let weapon = call(mem, weapon, &[chr]);
        let ty = store(remap(mem, true, 354.0));
        let tx = store(add(remap(mem, false, 1032.0), xf));
        call(mem, icon_name, &[weapon, tx, ty, 0]);
        let armour = mem.rebase(0x008a_9820);
        let armour = call(mem, armour, &[chr]);
        if armour == 0xff {
            let ty = store(remap(mem, true, 433.0));
            let tx = store(add(remap(mem, false, 1212.0), xf));
            let text = call(mem, mes, &[0x17]);
            call(mem, font, &[text, tx, ty, 0, s0, s1])
        } else {
            let ty = store(remap(mem, true, 423.0));
            let tx = store(add(remap(mem, false, 1032.0), xf));
            call(mem, icon_name, &[armour, tx, ty, 0])
        }
    }

    /// `MenuSummon_DrawCommandRows(_, x)` (`0x008d4280`, own name;
    /// `MenuStatus_DrawPage`'s page 1): with `n` the two command counts
    /// (`u16 0x0186aa24 + u16 0x0186aa28`, see
    /// [`menu_summon_build_command_list`]) and
    /// `step = f32(remapY2(f32(0x11d - 60 n)) / f32(n + 1))`, the rows
    /// `MenuSummon_DrawCommandRow(ftol(f32(x) + f32(remapX2(470))),
    /// ftol(top), i)` for `i` in `0..n`, `top` starting at
    /// `f32(remapY2(305) + step)` and moving by `f32(remapY2(60) + step)`
    /// (remaps in the order of these terms).
    pub fn menu_summon_draw_command_rows<M: Mem>(
        mem: &mut M,
        x: i32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, div, ftol, load, store};
        let count = |mem: &M, a: u32| u32::from(mem.read::<u16>(mem.rebase(a)));
        let n = count(mem, 0x0186_aa24) + count(mem, 0x0186_aa28);
        let n = n.cast_signed();
        let span = f32::from_bits(store(f64::from(0x11di32.wrapping_sub(n.wrapping_mul(60)))));
        let span = remap(mem, true, span);
        let step = store(div(span, load(store(f64::from(n + 1)))));
        let left = store(remap(mem, false, 470.0));
        let mut top = store(add(remap(mem, true, 305.0), load(step)));
        let advance = store(add(remap(mem, true, 60.0), load(step)));
        if n <= 0 {
            return;
        }
        let x = ftol(mem, add(load(store(f64::from(x))), load(left)));
        let row = mem.rebase(0x008d_4350);
        for i in 0..n {
            let y = ftol(mem, load(top));
            call(mem, row, &[x, y, i.cast_unsigned()]);
            top = store(add(load(top), load(advance)));
        }
    }

    /// `MenuSummon_DrawCommandRow(x, y, i)` (`0x008d4350`, own name; caller
    /// [`menu_summon_draw_command_rows`]): a type-`0xd` plate at `(f32(x),
    /// f32(y))` sized `remapX2(1220)` by `remapY2(60)`, its grey shape, the
    /// caption `TOMenuGetStatusMes(i < u16 0x0186aa24 ? 0x15 : 0x16)`
    /// (signed) right-aligned at `remapX2(586)` (its width through
    /// `width`), and the command's name `TkMn2GetCommandName(u16
    /// 0x0186aa04[i])` at `remapX2(746)` (asked after the remaps). Returns
    /// the name's `ToMakeBtlEasyFont` `eax`.
    pub fn menu_summon_draw_command_row<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        index: i32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        let (xb, yb) = (store(f64::from(x)), store(f64::from(y)));
        let (xf, yf) = (load(xb), load(yb));
        let plate_h = store(remap(mem, true, 60.0));
        let plate_w = store(remap(mem, false, 1220.0));
        let draw_plate = mem.rebase(0x008f_5f70);
        call(mem, draw_plate, &[xb, yb, plate_w, plate_h, 0xd]);
        let shape_h = store(remap(mem, true, 54.0));
        let shape_w = store(remap(mem, false, 554.0));
        let sy = store(add(remap(mem, true, 3.0), yf));
        let sx = store(add(remap(mem, false, 566.0), xf));
        let [u0, v0, u1, v1] = STATUS_PLATE_UV.map(f32::to_bits);
        let c = STATUS_PLATE_COLOUR;
        let shape = mem.rebase(0x0090_3ee0);
        call(
            mem,
            shape,
            &[u32::MAX, sx, sy, shape_w, shape_h, u0, v0, u1, v1, c, c],
        );
        let first = i32::from(mem.read::<u16>(mem.rebase(0x0186_aa24)));
        let mes = mem.rebase(0x008b_ee70);
        let text = call(mem, mes, &[if index < first { 0x15 } else { 0x16 }]);
        let [s0, s1] = EASY_FONT_SCALE;
        let font_width = mem.rebase(0x0090_5290);
        call(mem, font_width, &[text, width, 0, s0, s1]);
        let tw = mem.read::<u32>(width);
        let ty = store(add(remap(mem, true, 9.0), yf));
        let tx = store(sub(add(remap(mem, false, 586.0), xf), load(tw)));
        let font = mem.rebase(0x0090_5ab0);
        call(mem, font, &[text, tx, ty, 0, s0, s1]);
        let ty = store(add(remap(mem, true, 9.0), yf));
        let tx = store(add(remap(mem, false, 746.0), xf));
        let ids = mem.rebase(0x0186_aa04);
        let id = mem.read::<u16>(ids.wrapping_add(index.cast_unsigned().wrapping_mul(2)));
        let name = mem.rebase(0x008c_1a20);
        let name = call(mem, name, &[u32::from(id)]);
        call(mem, font, &[name, tx, ty, 0, s0, s1])
    }

    /// `MenuStatus_DrawPage(x, y, chr, page, a, b, c, _, t)` (`0x008d4ee0`,
    /// own name; caller `0x008d2710`, the status main window's draw).
    /// `ease = MenuMn2_Ease(t, 1) * 0x155 / 4096` (truncated);
    /// `MenuPlayer_DamageClass(chr)` is asked (unused). Pages 0 and 1 first
    /// draw the face slide for `cur = TkMenuGetCurrentPlayer()` or
    /// `TkMenuGetCurrentSummon()`: `TkMn2TransCharFace(page)`, then
    /// `MenuPlayer_DrawFaceSlide` or `MenuSummon_DrawFaceSlide(fx, fy,
    /// remapX2(200), remapY2(200), *a, *b, *c)` with
    /// `fx = f32(remapX2(236) + f32(x))` and `fy = f32(remapY2(282 or
    /// 347.5) + f32(y))` (remaps `h`, `w`, `y`, `x`). Then the original's
    /// jump table on the page:
    ///
    /// - 0: `MenuStatus_DrawEquipPlate(cur, ease)`, `TkMn2DrawOverDrive(i16
    ///   0x0186a888 + ease, i16 0x0186a88a, cur)` and
    ///   `TOMenuDrawChrStatusPlate(cur, f32(remapX2(210) + f32(ease)),
    ///   f32(remapY2(165)))`;
    /// - 1: `MenuSummon_DrawCommandRows(cur, ease)`;
    /// - 2: `MenuStatus_DrawLayoutRow(i16 0x0186a83c, i16 0x0186a83e, chr)`;
    /// - 3: `TkMn2DrawOverDrive(i16 0x0186a8a0 + ease, i16 0x0186a8a2,
    ///   TkMenuGetCurrentSummon())`.
    ///
    /// Every page (any above 3 too, unsigned) ends in
    /// `TkMn2DrawKickPacket()`, whose `eax` is returned.
    #[allow(clippy::too_many_arguments)]
    pub fn menu_status_draw_page<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        chr: u32,
        page: u32,
        faces: [u32; 3],
        time: i32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store};
        let at = |mem: &M, a: u32| mem.rebase(a);
        let ease_fn = at(mem, 0x008d_30c0);
        let ease = call(mem, ease_fn, &[time.cast_unsigned(), 1]).cast_signed();
        let ease = ease.wrapping_mul(0x155) / 4096;
        let damage_class = at(mem, 0x008a_99f0);
        call(mem, damage_class, &[chr]);
        let half = |mem: &M, a: u32| i32::from(mem.read::<i16>(mem.rebase(a))).cast_unsigned();
        let (player, summon) = (at(mem, 0x008a_9860), at(mem, 0x008a_9880));
        let cur = match page {
            0 | 1 => {
                let cur = call(mem, if page == 0 { player } else { summon }, &[]);
                let trans = at(mem, 0x008c_2ce0);
                call(mem, trans, &[page]);
                let [f0, f1, f2] = faces.map(|p| mem.read::<u32>(p));
                let h = store(remap(mem, true, 200.0));
                let w = store(remap(mem, false, 200.0));
                let row = if page == 0 { 282.0 } else { 347.5 };
                let fy = store(add(remap(mem, true, row), load(store(f64::from(y)))));
                let fx = store(add(remap(mem, false, 236.0), load(store(f64::from(x)))));
                let slide = at(mem, if page == 0 { 0x008d_32b0 } else { 0x008d_34a0 });
                call(mem, slide, &[fx, fy, w, h, f0, f1, f2]);
                cur
            }
            3 => call(mem, summon, &[]),
            _ => chr,
        };
        let ease_word = ease.cast_unsigned();
        let over_drive = at(mem, 0x008d_3d70);
        match page {
            0 => {
                let equip = at(mem, 0x008d_4540);
                call(mem, equip, &[cur, ease_word]);
                let ox = half(mem, 0x0186_a888).wrapping_add(ease_word);
                let oy = half(mem, 0x0186_a88a);
                call(mem, over_drive, &[ox, oy, cur]);
                let py = store(remap(mem, true, 165.0));
                let px = store(add(remap(mem, false, 210.0), load(store(f64::from(ease)))));
                let status_plate = at(mem, 0x008f_f490);
                call(mem, status_plate, &[cur, px, py]);
            }
            1 => {
                let rows = at(mem, 0x008d_4280);
                call(mem, rows, &[cur, ease_word]);
            }
            2 => {
                let (lx, ly) = (half(mem, 0x0186_a83c), half(mem, 0x0186_a83e));
                let layout = at(mem, 0x008c_11e0);
                call(mem, layout, &[lx, ly, cur]);
            }
            3 => {
                let ox = half(mem, 0x0186_a8a0).wrapping_add(ease_word);
                let oy = half(mem, 0x0186_a8a2);
                call(mem, over_drive, &[ox, oy, cur]);
            }
            _ => {}
        }
        let kick = at(mem, 0x008c_0c40);
        call(mem, kick, &[])
    }
    /// `MenuSelList_DrawRow(window, x, y, row, fixed)` (`0x008d1820`, own
    /// name; caller `0x008d1620`): one row of a selection list. Unless
    /// `fixed`, `y` moves up by `f32(i16 +0x46 * 32 / 4096)` (truncated).
    /// For `row` in `0..u16 +0x30` (signed) a type-`0xe` plate
    /// `(x, y, remapX2(740), remapY2(60))`, then for an enabled entry
    /// (byte `+3` of [`MENU_ITEM_LIST`] entry `row`) by its kind (byte
    /// `+2`): 1 the command icon and name
    /// `TODrawCrossMenuIconCommandName(id, x, y, 0)`; 2 the aeon name
    /// `AtelGetSummonNameforFinalBattleSelector(id)`, 3
    /// `TkMenuMenu1Mes(id + 0x1033)`, each centred: `ToMakeBtlEasyFont(text,
    /// f32((remapX2(740) - width) * 0.5 + x), f32(remapY2(8) + y), 0,
    /// 0.78, 1)` (y remapped first, the width through `width`).
    #[allow(clippy::too_many_arguments)]
    pub fn menu_sel_list_draw_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [f32; 2],
        row: i32,
        fixed: bool,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, round_trip, store, sub};
        let mut top = y.to_bits();
        if !fixed {
            let scroll = i32::from(mem.read::<i16>(window.wrapping_add(0x46))) * 32 / 4096;
            top = store(sub(load(top), load(store(f64::from(scroll)))));
        }
        let rows = i32::from(mem.read::<u16>(window.wrapping_add(0x30)));
        if row < 0 || row >= rows {
            return;
        }
        let left = round_trip(x).to_bits();
        let top = round_trip(f32::from_bits(top)).to_bits();
        let plate_h = store(remap(mem, true, 60.0));
        let plate_w = store(remap(mem, false, 740.0));
        let draw_plate = mem.rebase(0x008f_5f70);
        call(mem, draw_plate, &[left, top, plate_w, plate_h, 0xe]);
        let entry = MENU_ITEM_LIST
            .ptr(mem)
            .addr
            .wrapping_add(row.cast_unsigned() * 4);
        if mem.read::<u8>(entry.wrapping_add(3)) == 0 {
            return;
        }
        let id = u32::from(mem.read::<u16>(entry));
        let text = match mem.read::<u8>(entry.wrapping_add(2)) {
            1 => {
                let icon_name = mem.rebase(0x008f_5000);
                call(mem, icon_name, &[id, left, top, 0]);
                return;
            }
            2 => {
                let summon_name = mem.rebase(0x0087_3b50);
                call(mem, summon_name, &[id])
            }
            3 => {
                let mes = mem.rebase(0x008d_dfc0);
                call(mem, mes, &[id + 0x1033])
            }
            _ => return,
        };
        let [s0, s1] = EASY_FONT_SCALE;
        let font_width = mem.rebase(0x0090_5290);
        call(mem, font_width, &[text, width, 0, s0, s1]);
        let ty = store(add(remap(mem, true, 8.0), load(top)));
        let span = sub(remap(mem, false, 740.0), load(mem.read::<u32>(width)));
        let tx = store(add(mul(span, 0.5), load(left)));
        let font = mem.rebase(0x0090_5ab0);
        call(mem, font, &[text, tx, ty, 0, s0, s1]);
    }

    /// `TkMn2DrawStatusPlate(x, y, _, _, chr, mes, field)` (`0x008c1490`,
    /// Fahrenheit's name): a type-`0xe` plate at `(f32(x), f32(y))` sized
    /// `remapX2(740)` by `remapY2(60)`; shape 200 `TOMkpShapeXYWHUV(200,
    /// f32(remapX2(596) + x), f32(y - remapY2(8)), remapX2(80), remapY2(80),
    /// 498, 603, 573, 678 / 1024)`; the caption `TOMenuGetStatusMes(mes)`
    /// centred over `remapX2(740)` at `remapY2(8)` (asked once for the
    /// width and again to draw); and the stat byte
    /// `MsGetSavePlayerPtr(chr)[field]` as `ToMakeBtlEasyEdgeDigit2(v,
    /// f32(remapX2(637) + x - width / 2), f32(remapY2(15) + y), 0, 0.68,
    /// -0.0)` (width from `ToGetBtlEasyDigitWidth2(v, width, 0.68, 0)`).
    /// Remaps run `h`, `w`, `y`, `x`. Returns the digits' `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn tk_mn2_draw_status_plate<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        chr: u32,
        mes: u32,
        field: u32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let player = mem.rebase(0x0078_5330);
        let player = call(mem, player, &[chr]);
        let (xb, yb) = (store(f64::from(x)), store(f64::from(y)));
        let (xf, yf) = (load(xb), load(yb));
        let plate_h = store(remap(mem, true, 60.0));
        let plate_w = store(remap(mem, false, 740.0));
        let draw_plate = mem.rebase(0x008f_5f70);
        call(mem, draw_plate, &[xb, yb, plate_w, plate_h, 0xe]);
        let shape_h = store(remap(mem, true, 80.0));
        let shape_w = store(remap(mem, false, 80.0));
        let sy = store(sub(yf, remap(mem, true, 8.0)));
        let sx = store(add(remap(mem, false, 596.0), xf));
        let [u0, v0, u1, v1] = [498.0f32, 603.0, 573.0, 678.0].map(|v| (v / 1024.0).to_bits());
        let shape = mem.rebase(0x0090_3bb0);
        call(mem, shape, &[200, sx, sy, shape_w, shape_h, u0, v0, u1, v1]);
        let status_mes = mem.rebase(0x008b_ee70);
        let [s0, s1] = EASY_FONT_SCALE;
        let text = call(mem, status_mes, &[mes]);
        let font_width = mem.rebase(0x0090_5290);
        call(mem, font_width, &[text, width, 0, s0, s1]);
        let ty = store(add(remap(mem, true, 8.0), yf));
        let span = sub(remap(mem, false, 740.0), load(mem.read::<u32>(width)));
        let tx = store(add(mul(span, 0.5), xf));
        let text = call(mem, status_mes, &[mes]);
        let font = mem.rebase(0x0090_5ab0);
        call(mem, font, &[text, tx, ty, 0, s0, s1]);
        let stat = player.wrapping_add(field);
        let digit_scale = 0.68f32.to_bits();
        let value = u32::from(mem.read::<u8>(stat));
        let digit_width = mem.rebase(0x0090_5230);
        call(mem, digit_width, &[value, width, digit_scale, 0]);
        let dy = store(add(remap(mem, true, 15.0), yf));
        let right = add(remap(mem, false, 637.0), xf);
        let value = u32::from(mem.read::<u8>(stat));
        let dx = store(sub(right, mul(load(mem.read::<u32>(width)), 0.5)));
        let digits = mem.rebase(0x0090_5820);
        call(mem, digits, &[value, dx, dy, 0, digit_scale, 0x8000_0000])
    }

    /// `MenuMain_DrawOptionRow(i, x, _, _, fade)` (`0x008e22a0`, own name;
    /// caller `0x008e20f0`): main-menu option `i` sliding in with `fade`
    /// (0..0x1000). `left = f32(remapX2(1304) + x)`; `top =
    /// f32((f32(remapY2(194) + remapY2(75) * f32(i)) * f32(fade) +
    /// f32(0x1000 - fade) * 3) / 4096)`. Its bar is shape -3 at
    /// `(left, top, remapX2(476), remapY2(60))`, texels `(1118,
    /// 74.7 i, 1599, 67 + 74.7 i)` in colour `(fade / 32) << 24 |
    /// 0x808080` (both corners); its name `TkMenuMainMes(
    /// TkMenuMainGetOptionMenu(i))` (asked for the width and again)
    /// centred over `remapX2(476)` at `remapY2(7)`, through
    /// `ToMakeBtlEasyEdgeFontA(text, x, y, dim, 0.78, 1, fade / 32)` with
    /// `dim` 1 when `TkMenuMainGetOptionEnabled(i)` is 0 (also the width
    /// call's third word). For the chosen option (`0x01871548`) at full
    /// fade, `TkMn2DrawCrossCursor(f32(remapX2(115) + left),
    /// f32(remapY2(10) + top), 0)`. Returns the last call's `eax`.
    pub fn menu_main_draw_option_row<M: Mem>(
        mem: &mut M,
        index: i32,
        x: f32,
        fade: i32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let alpha = fade / 32;
        let left = store(add(remap(mem, false, 1304.0), load(x.to_bits())));
        let fi = load(store(f64::from(index)));
        let spaced = mul(remap(mem, true, 75.0), fi);
        let base = load(store(add(remap(mem, true, 194.0), spaced)));
        let shown = mul(base, load(store(f64::from(fade))));
        let hidden = mul(load(store(f64::from(0x1000i32.wrapping_sub(fade)))), 3.0);
        let top = store(mul(add(shown, hidden), 1.0 / 4096.0));
        let colour = alpha
            .cast_unsigned()
            .wrapping_shl(24)
            .wrapping_add(0x0080_8080);
        let band = mul(fi, f64::from(74.7f32));
        let v1 = store(add(67.0, band));
        let v0 = store(add(band, 0.0));
        let bar_h = store(remap(mem, true, 60.0));
        let bar_w = store(remap(mem, false, 476.0));
        let [u0, u1] = [1118f32, 1599.0].map(f32::to_bits);
        let shape = mem.rebase(0x0090_3ee0);
        #[rustfmt::skip]
        call(mem, shape, &[0xffff_fffd, left, top, bar_w, bar_h, u0, v0, u1, v1, colour, colour]);
        let index_word = index.cast_unsigned();
        let (option_menu, option_enabled) = (mem.rebase(0x008e_2270), mem.rebase(0x008e_2290));
        let menu = call(mem, option_menu, &[index_word]);
        let dim = u32::from(call(mem, option_enabled, &[index_word]) == 0);
        let main_mes = mem.rebase(0x008d_df80);
        let [s0, s1] = EASY_FONT_SCALE;
        let text = call(mem, main_mes, &[menu]);
        let font_width = mem.rebase(0x0090_5290);
        call(mem, font_width, &[text, width, dim, s0, s1]);
        let ty = store(add(remap(mem, true, 7.0), load(top)));
        let span = sub(remap(mem, false, 476.0), load(mem.read::<u32>(width)));
        let tx = store(add(mul(span, 0.5), load(left)));
        let text = call(mem, main_mes, &[menu]);
        let edge_font = mem.rebase(0x0090_5970);
        let alpha = alpha.cast_unsigned();
        let mut eax = call(mem, edge_font, &[text, tx, ty, dim, s0, s1, alpha]);
        let chosen = mem.read::<u32>(mem.rebase(0x0187_1548));
        if index_word == chosen && fade == 0x1000 {
            let cy = store(add(remap(mem, true, 10.0), load(top)));
            let cx = store(add(remap(mem, false, 115.0), load(left)));
            let cursor = mem.rebase(0x008c_0670);
            eax = call(mem, cursor, &[cx, cy, 0]);
        }
        eax
    }
    /// `Pad_UpdateAnalog(work, raw, buttons)` (`0x00889570`, own name;
    /// caller `0x00888f90`): a pad work record's analog state from a raw
    /// PS2 pad packet. `n = (raw[1] & 0xf) * 2 - 2` (0 counts as -1) words
    /// of analog data follow at `raw + 4`; after `Pad_ResetAnalog(work)` up
    /// to four of those bytes are copied to `+0x84` (and skipped), then
    /// `+0x84..+0x87` become `Pad_StickToByte` of
    /// `inputGetCurrDeviceStickInfo()`'s `f32`s 2, 3, 0, 1. The pressure
    /// bytes `+0x90`/`+0x91`: with data left, `Compute_008889e0(8..11,
    /// byte)` halved (truncated) for the buttons `0x2000` (`+ 0x80`),
    /// `0x8000` (`0x7f -`), `0x1000` (`0x7f -`), `0x4000` (`+ 0x80`), the
    /// next four bytes in that order; without, the fixed `0xbf`, `0x40`,
    /// `0x40`, `0xbf`. Finally the stick bits of `u16 +0x9a`: `+0x86 <
    /// 0x10` sets `0x8000` (clearing `0x2000`), `> 0xf0` sets `0x2000`
    /// (clearing `0xa000`); `+0x87 > 0xf0` sets `0x4000` (clearing
    /// `0x1000`), `< 0x10` sets `0x1000` (clearing `0x4000`).
    pub fn pad_update_analog<M: Mem>(
        mem: &mut M,
        work: u32,
        raw: u32,
        buttons: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let stick_info = mem.rebase(0x0063_0c90);
        let stick = call(mem, stick_info, &[]);
        let mut data = raw.wrapping_add(4);
        let mut left = i32::from(mem.read::<u8>(raw.wrapping_add(1)) & 0xf) * 2 - 2;
        if left == 0 {
            left = -1;
        }
        let reset = mem.rebase(0x0088_9000);
        call(mem, reset, &[work]);
        if left > 0 {
            for i in 0..4 {
                let b = mem.read::<u8>(data);
                mem.write(work.wrapping_add(0x84 + i), b);
                left -= 1;
                data = data.wrapping_add(1);
                if left <= 0 {
                    break;
                }
            }
        }
        let to_byte = mem.rebase(0x0088_9aa0);
        for (k, axis) in (0u32..).zip([2u32, 3, 0, 1]) {
            // fld/fstp: the axis passed as loaded
            let v = mem.read::<u32>(stick.wrapping_add(axis * 4));
            let v = ffx_bridge::x87::round_trip(f32::from_bits(v)).to_bits();
            let b = call(mem, to_byte, &[v]).to_le_bytes()[0];
            mem.write(work.wrapping_add(0x84 + k), b);
        }
        let pressure = mem.rebase(0x0088_89e0);
        // (button, byte offset, kind, flipped): flipped bytes are 0x7f - p/2
        let keys = [
            (0x2000, 0x90, 8, false),
            (0x8000, 0x90, 9, true),
            (0x1000, 0x91, 10, true),
            (0x4000, 0x91, 11, false),
        ];
        for (k, (bit, at, kind, flipped)) in (0u32..).zip(keys) {
            if buttons & bit == 0 {
                continue;
            }
            let v = if left > 0 {
                let p = u32::from(mem.read::<u8>(data.wrapping_add(k)));
                let half = call(mem, pressure, &[kind, p]).cast_signed() / 2;
                let half = half.to_le_bytes()[0];
                if flipped {
                    0x7fu8.wrapping_sub(half)
                } else {
                    half.wrapping_add(0x80)
                }
            } else if flipped {
                0x40
            } else {
                0xbf
            };
            mem.write(work.wrapping_add(at), v);
        }
        let flags = work.wrapping_add(0x9a);
        let x = mem.read::<u8>(work.wrapping_add(0x86));
        if x < 0x10 {
            let f = mem.read::<u16>(flags) & 0xdfff | 0x8000;
            mem.write(flags, f);
        } else if x > 0xf0 {
            let f = mem.read::<u16>(flags) & 0x5fff | 0x2000;
            mem.write(flags, f);
        }
        let y = mem.read::<u8>(work.wrapping_add(0x87));
        if y > 0xf0 {
            let f = mem.read::<u16>(flags) & 0xefff | 0x4000;
            mem.write(flags, f);
        } else if y < 0x10 {
            let f = mem.read::<u16>(flags) & 0xbfff | 0x1000;
            mem.write(flags, f);
        }
    }

    /// `MenuSaveLoad_DrawSlotRow(window, x, y, row, mode)` (`0x008b4b70`,
    /// own name; caller `0x008b4a50`, the save/load window's draw): one of
    /// the 99 save slots. `row` moves by the window's i16 `+0x32` (modes 0
    /// and 2) or `+0x34` (mode 1); mode 0 also moves `y` by `i16 +0x46 *
    /// -48 / 4096` (truncated). For `row` in `0..u16 +0x30` (signed), with
    /// `state = g_TkSaveSlots[row]` and `dim` (`state` not 0..2 or 4 when
    /// byte `0x0186628a` is 1, else `state != 1`): the frame
    /// `0x008f9b60(x, y, 0x7a, 0x27, 0x80372d22, 0x80524641)`, shapes
    /// `0x10f` at `(x, y)` and `0x10c` at `(x + 0x1bd, y)` around
    /// `TODrawBoxXYWHC2(x + 0x7a, y, 0x143, 0x27, 0x8048211d, 0x805c393c)`,
    /// the slot number `sprintf(digits, "%02d", row + 1)` through
    /// `0x00901f80(digits, x + 0x5a, y + 2, 0x25)`, and the kind text
    /// `MenuSave_KindText(state, TkMenuGetSaveFileInfo(row))`: in Japanese
    /// (`TOGetFFXLangJmp() == 0`) at `x + 0x92` (`0x00901d10(.., dim,
    /// state)` for a used slot, else `0x00901cb0(.., dim)`), otherwise at
    /// `x + 0x88` through `0x00901cb0`. A used slot (`state == 1`) adds
    /// its place name at `y + 0x11` (`AtelGetSaveDicName(u16 info+0x18,
    /// u8 info+0x1f)` through `0x00901d10(.., dim, u8 info+0x1f)` in
    /// Japanese, else with `MsGetSaveConfigEnglish()` as the second word
    /// through `0x00901cb0`) and its play time `TOMenuDrawTimer(info+0x10,
    /// f32(x + 0x1b3), f32(y + 5), 0)`. `digits` is the original's 8-byte
    /// frame buffer.
    pub fn menu_save_load_draw_slot_row<M: Mem>(
        mem: &mut M,
        window: u32,
        [x, y]: [i32; 2],
        row: i32,
        mode: u32,
        digits: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let half = |mem: &M, o: u32| i32::from(mem.read::<i16>(window.wrapping_add(o)));
        let (mut row, mut y) = (row, y);
        match mode {
            0 => {
                row = row.wrapping_add(half(mem, 0x32));
                y = y.wrapping_add(half(mem, 0x46) * -48 / 4096);
            }
            1 => row = row.wrapping_add(half(mem, 0x34)),
            2 => row = row.wrapping_add(half(mem, 0x32)),
            _ => {}
        }
        if row < 0 || row >= i32::from(mem.read::<u16>(window.wrapping_add(0x30))) {
            return;
        }
        let at = |mem: &M, a: u32| mem.rebase(a);
        let row_word = row.cast_unsigned();
        let state = mem.read::<u8>(at(mem, 0x0186_6290).wrapping_add(row_word));
        let dim = if mem.read::<u8>(at(mem, 0x0186_628a)) == 1 {
            !(state <= 2 || state == 4)
        } else {
            state != 1
        };
        let dim = u32::from(dim);
        let state = u32::from(state);
        let (xw, yw) = (x.cast_unsigned(), y.cast_unsigned());
        let frame = at(mem, 0x008f_9b60);
        call(mem, frame, &[xw, yw, 0x7a, 0x27, 0x8037_2d22, 0x8052_4641]);
        let shape = at(mem, 0x0090_3a10);
        call(mem, shape, &[0x10f, xw, yw]);
        let draw_box = at(mem, 0x008f_3db0);
        #[rustfmt::skip]
        call(mem, draw_box, &[xw.wrapping_add(0x7a), yw, 0x143, 0x27, 0x8048_211d, 0x805c_393c]);
        call(mem, shape, &[0x10c, xw.wrapping_add(0x1bd), yw]);
        let sprintf = mem.read::<u32>(at(mem, 0x00b0_c3dc));
        let format = at(mem, 0x00b4_1ec0);
        call(mem, sprintf, &[digits, format, row_word.wrapping_add(1)]);
        let number = at(mem, 0x0090_1f80);
        call(
            mem,
            number,
            &[digits, xw.wrapping_add(0x5a), yw.wrapping_add(2), 0x25],
        );
        let file_info = at(mem, 0x008b_4e40);
        let info = call(mem, file_info, &[row_word]);
        let kind_text = at(mem, 0x008b_5560);
        let text = call(mem, kind_text, &[state, info]);
        let lang = at(mem, 0x008a_c2f0);
        let (text_jp, text_en) = (at(mem, 0x0090_1d10), at(mem, 0x0090_1cb0));
        let (jp_x, en_x) = (xw.wrapping_add(0x92), xw.wrapping_add(0x88));
        if call(mem, lang, &[]) == 0 {
            if state == 1 {
                call(mem, text_jp, &[text, jp_x, yw, dim, state]);
            } else {
                call(mem, text_en, &[text, jp_x, yw, dim]);
            }
        } else {
            call(mem, text_en, &[text, en_x, yw, dim]);
        }
        if state != 1 {
            return;
        }
        let place = at(mem, 0x0086_c430);
        let line = yw.wrapping_add(0x11);
        if call(mem, lang, &[]) == 0 {
            let place_id = u32::from(mem.read::<u16>(info.wrapping_add(0x18)));
            let style = u32::from(mem.read::<u8>(info.wrapping_add(0x1f)));
            let name = call(mem, place, &[place_id, style]);
            call(mem, text_jp, &[name, jp_x, line, dim, style]);
        } else {
            let english = at(mem, 0x0078_51d0);
            let english = call(mem, english, &[]);
            let place_id = u32::from(mem.read::<u16>(info.wrapping_add(0x18)));
            let name = call(mem, place, &[place_id, english]);
            call(mem, text_en, &[name, en_x, line, dim]);
        }
        let to_f32 = |v: i32| ffx_bridge::x87::store(f64::from(v));
        let timer = at(mem, 0x008e_19f0);
        let frames = mem.read::<u32>(info.wrapping_add(0x10));
        let (tx, ty) = (to_f32(x.wrapping_add(0x1b3)), to_f32(y.wrapping_add(5)));
        call(mem, timer, &[frames, tx, ty, 0]);
    }
    /// `DrawCrossMenuSummonPlateClut(chr, x, y, alpha)` (`0x008e6e10`,
    /// Fahrenheit's name): a character's name and HP/MP plate.
    ///
    /// - The name: `ToMakeBtlEasyEdgeFont(TOGetSaveChrName(chr),
    ///   f32(remapX2(160) + x), f32(remapY2(10) + y), alpha, 0.82, 1)`.
    /// - Offsets `(wide, tall, shift)` are `(22, 1, 11)` when
    ///   `TOGetFFXLangJmp()` is 11, else 0.
    /// - HP: label shape 0x21 (`TOMkpShapeXYWHUV`) at `(remapX2(480) + x,
    ///   remapY2(8) + y)`, sized `remapX2(wide + 61.5)` by
    ///   `remapY2(tall + 23)`, texels `(356, 15, wide + 436, 41 + tall) / 1024`; then
    ///   `sprintf(text, "%5d/%5d", TkMenuGetHP(chr), TkMenuGetMaxHP(chr))`
    ///   (max asked first) through `TOMkpCrossEasyStrFontSClut(text,
    ///   remapX2(594) + x + shift, remapY2(9) + y, 0x25, 0.68, 0)`.
    /// - MP: shape 0x22 at `(remapX2(530) + x, remapY2(35) + y)`, sized
    ///   `remapX2(wide + 49)` by `remapY2(23)`, texels `(510, 15, wide + 570,
    ///   41) / 1024`; `"%4d/%4d"` of
    ///   the MP at `(remapX2(615) + x + shift, remapY2(37) + y)`.
    ///
    /// Each value is rounded to `f32`; remaps run `h`, `w`, `y`, `x`.
    /// `text` is the original's 32-byte frame buffer. Returns the last text
    /// call's `eax`.
    pub fn draw_cross_menu_summon_plate_clut<M: Mem>(
        mem: &mut M,
        chr: u32,
        [x, y]: [f32; 2],
        alpha: u32,
        text: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, mul, store};
        const TEXEL: f64 = 1.0 / 1024.0;
        let (xf, yf) = (load(x.to_bits()), load(y.to_bits()));
        let at = |mem: &M, a: u32| mem.rebase(a);
        let name_y = store(add(remap(mem, true, 10.0), yf));
        let name_x = store(add(remap(mem, false, 160.0), xf));
        let chr_name = at(mem, 0x008a_c850);
        let name = call(mem, chr_name, &[chr]);
        let edge_font = at(mem, 0x0090_5930);
        let name_scale = [0.82f32, 1.0].map(f32::to_bits);
        call(
            mem,
            edge_font,
            &[name, name_x, name_y, alpha, name_scale[0], name_scale[1]],
        );
        let lang = at(mem, 0x008a_c2f0);
        let (wide, tall, shift) = if call(mem, lang, &[]) == 11 {
            (22.0, 1.0, 11.0)
        } else {
            (0.0, 0.0, 0.0)
        };
        let f32_of = |v: f64| f32::from_bits(store(v));
        let shape = at(mem, 0x0090_3bb0);
        let sprintf = mem.read::<u32>(at(mem, 0x00b0_c3dc));
        let str_font = at(mem, 0x0090_1660);
        let digit_scale = 0.68f32.to_bits();
        // (shape, x, y, w, h, uv, value, max, format, text x, text y)
        let hp = (
            0x21,
            [480.0, 8.0],
            [f32_of(add(wide, 61.5)), f32_of(add(tall, 23.0))],
            [356.0f32, 15.0].map(|v| (v / 1024.0).to_bits()),
            [
                store(mul(TEXEL, add(wide, 436.0))),
                store(mul(add(41.0, tall), TEXEL)),
            ],
            [0x008a_98c0, 0x008a_9990, 0x00b6_684c],
            [594.0, 9.0],
        );
        let mp = (
            0x22,
            [530.0, 35.0],
            [f32_of(add(wide, 49.0)), 23.0],
            [510.0f32, 15.0].map(|v| (v / 1024.0).to_bits()),
            [
                store(mul(add(wide, 570.0), TEXEL)),
                (41.0f32 / 1024.0).to_bits(),
            ],
            [0x008a_9970, 0x008a_99b0, 0x00b6_6854],
            [615.0, 37.0],
        );
        let mut eax = 0;
        for (id, [sx, sy], [sw, sh], [u0, v0], [u1, v1], [value, max, format], [tx, ty]) in [hp, mp]
        {
            let h = store(remap(mem, true, sh));
            let w = store(remap(mem, false, sw));
            let top = store(add(remap(mem, true, sy), yf));
            let left = store(add(remap(mem, false, sx), xf));
            call(mem, shape, &[id, left, top, w, h, u0, v0, u1, v1]);
            let (value, max) = (at(mem, value), at(mem, max));
            let max = call(mem, max, &[chr]);
            let value = call(mem, value, &[chr]);
            call(mem, sprintf, &[text, at(mem, format), value, max]);
            let top = store(add(remap(mem, true, ty), yf));
            let left = store(add(add(remap(mem, false, tx), xf), shift));
            eax = call(mem, str_font, &[text, left, top, 0x25, digit_scale, 0]);
        }
        eax
    }

    /// `TkMn2DrawSummonPlateCheckHP(x, y, chr)` (`0x008ceb00`, Fahrenheit's
    /// name): a character's plate, with HP/MP while `TkMenuGetHP(chr) > 0`.
    ///
    /// The colour is `MenuPlayer_DamageClass(chr)`, or 1 when the open
    /// sub-command window's (`0x023cc128`) row `i16 +0x48` is marked 1 in
    /// the bytes at `0x0186a5b0` and `chr > 13` (signed). Alive:
    /// `DrawCrossMenuSummonPlateClut(chr, f32(x), f32(y), colour)`.
    /// Otherwise `DrawCrossMenuSummonPlateNoHPMPClut(chr, f32(x), f32(y),
    /// colour)` with colour `0x53` unless the window case set 1, then
    /// `TkMn2DrawCrossSummonBattleCountPlate(f32(remapX2(428) + f32(x)),
    /// f32(y), chr)`. Returns the last call's `eax`.
    pub fn tk_mn2_draw_summon_plate_check_hp<M: Mem>(
        mem: &mut M,
        [x, y]: [i32; 2],
        chr: i32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store};
        let at = |mem: &M, a: u32| mem.rebase(a);
        let id = chr.cast_unsigned();
        let hp = call(mem, at(mem, 0x008a_98c0), &[id]);
        let mut colour = call(mem, at(mem, 0x008a_99f0), &[id]);
        let window = mem.read::<u32>(at(mem, 0x023c_c128));
        let marked = window != 0 && {
            let row = i32::from(mem.read::<i16>(window.wrapping_add(0x48)));
            mem.read::<u8>(at(mem, 0x0186_a5b0).wrapping_add_signed(row)) == 1 && chr > 13
        };
        if marked {
            colour = 1;
        }
        let (fx, fy) = (store(f64::from(x)), store(f64::from(y)));
        if hp.cast_signed() > 0 {
            return call(mem, at(mem, 0x008e_6e10), &[id, fx, fy, colour]);
        }
        if !marked {
            colour = 0x53;
        }
        call(mem, at(mem, 0x008e_7110), &[id, fx, fy, colour]);
        let left = store(add(remap(mem, false, 428.0), load(fx)));
        call(mem, at(mem, 0x008c_d720), &[left, fy, id])
    }

    /// `Pad_ReadAndUpdate(port, slot, pad)` (`0x00888f90`, name ours): one
    /// pad's frame. With `work = TkGetControlerWork(port, slot)`:
    /// `Pad_ReadCurrDevice(port, slot, work, pad)`, `Pad_UpdateAnalog(work,
    /// pad, u16 work +0x98)`, and when byte `+0x9d` is set
    /// `inputSetVibration(port, byte +0xff, byte +0xfe)`. The original also
    /// calls `scePadRead_ps2`, which is `xor eax, eax; ret` on PC; left out.
    /// Its only caller (`0x00889050`) ignores `eax`.
    pub fn pad_read_and_update<M: Mem>(
        mem: &mut M,
        port: u32,
        slot: u32,
        pad: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let work = tk_get_controler_work(mem, port.cast_signed(), slot.cast_signed());
        let read = mem.rebase(0x0088_98a0);
        call(mem, read, &[port, slot, work, pad]);
        let buttons = u32::from(mem.read::<u16>(work.wrapping_add(0x98)));
        let analog = mem.rebase(0x0088_9570);
        call(mem, analog, &[work, pad, buttons]);
        if mem.read::<u8>(work.wrapping_add(0x9d)) != 0 {
            let large = u32::from(mem.read::<u8>(work.wrapping_add(0xff)));
            let small = u32::from(mem.read::<u8>(work.wrapping_add(0xfe)));
            let vibrate = mem.rebase(0x0063_0e20);
            call(mem, vibrate, &[port, large, small]);
        }
    }

    /// `MenuSelList_DrawRows(window, fixed, first, _, y)` (`0x008d1620`,
    /// name ours): nine two-column rows of a selection list from row
    /// `first * 2 - 2`. Row pairs start at `f32(remapY2(403) + f32(y))` and
    /// step by `remapY2(77)` (rounded to `f32` each time); the left column
    /// is at `remapX2(210)`, the right at `remapX2(970)`, each drawn by
    /// `MenuSelList_DrawRow(window, x, y, row, fixed)`. Its only caller
    /// (`0x008d1400`) ignores `eax`.
    pub fn menu_sel_list_draw_rows<M: Mem>(
        mem: &mut M,
        window: u32,
        fixed: u32,
        first: i32,
        y: i32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, load, store};
        let draw_row = mem.rebase(0x008d_1820);
        let mut top = store(add(remap(mem, true, 403.0), load(store(f64::from(y)))));
        let mut row = first.wrapping_mul(2).wrapping_sub(2);
        for _ in 0..9 {
            for (column, x) in [(0, 210.0), (1, 970.0)] {
                let left = store(remap(mem, false, x));
                let r = row.wrapping_add(column).cast_unsigned();
                call(mem, draw_row, &[window, left, top, r, fixed]);
            }
            top = store(add(remap(mem, true, 77.0), load(top)));
            row = row.wrapping_add(2);
        }
    }

    /// `TkBtlEndGetText(id)` (`0x008bda60`, Fahrenheit's name):
    /// `MsMenuGetBtlEndText(id, MsGetSaveConfigHiragana())` (`eax`).
    pub fn tk_btl_end_get_text<M: Mem>(
        mem: &mut M,
        id: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let hiragana = mem.rebase(0x0078_51f0);
        let hiragana = call(mem, hiragana, &[]);
        let text = mem.rebase(0x0078_fb60);
        call(mem, text, &[id, hiragana])
    }

    /// `TKMallocAndReadSPS2(index, page)` (`0x0088ce80`, Fahrenheit's
    /// name): loads a help `.sps2` file whole. The path is the language's
    /// master directory (`TOGetFFXLang()`: 0 `new_jppc`, 2 `new_frpc`, 3
    /// `new_sppc`, 4 `new_depc`, 5 `new_itpc`, 9 `new_krpc`, 10 `new_chpc`,
    /// anything else `new_uspc`, under `/ffx_ps2/ffx/master/`) followed by
    /// entry `index` of the file table at `0x00c58e44` (page 0) or the page
    /// table at `0x00c58e58`; `fiosUnifyFilename(path, unified, 0x100)` and
    /// `sprintf_s(path, 0xff, "%s", unified)` normalise it. A missing file
    /// is reported (`dbgPrintf`, a `ret` on PC) and gives 0. Otherwise the
    /// size is rounded up to 16, `Sg_MemSetAlign(0x40)`, and the buffer
    /// (`user_malloc` of the size rounded up to 0x800, signed) gets a zero
    /// first word, then `fiosRead` and `fiosClose`; returns the buffer.
    /// `path` and `unified` are the original's two 256-byte frame buffers.
    pub fn tk_malloc_and_read_sps2<M: Mem>(
        mem: &mut M,
        index: u32,
        page: u32,
        [path, unified]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let lang = call(mem, at(mem, 0x0064_10e0), &[]);
        let master = match lang {
            0 => 0x00b5_df10,
            2 => 0x00b5_e310,
            3 => 0x00b5_e210,
            4 => 0x00b5_e110,
            5 => 0x00b5_e410,
            9 => 0x00b5_e510,
            10 => 0x00b5_e610,
            _ => 0x00b5_e010,
        };
        let table = if page == 0 { 0x00c5_8e44 } else { 0x00c5_8e58 };
        let name = mem.read::<u32>(at(mem, table).wrapping_add(index.wrapping_mul(4)));
        // strcpy, then strcat
        let mut to = path;
        for from in [at(mem, master), name] {
            let mut from = from;
            loop {
                let c = mem.read::<u8>(from);
                mem.write(to, c);
                if c == 0 {
                    break;
                }
                from = from.wrapping_add(1);
                to = to.wrapping_add(1);
            }
        }
        call(mem, at(mem, 0x0067_9820), &[path, unified, 0x100]);
        let sprintf_s = mem.read::<u32>(at(mem, 0x00b0_c394));
        call(mem, sprintf_s, &[path, 0xff, at(mem, 0x00b0_e160), unified]);
        if call(mem, at(mem, 0x0067_96f0), &[path]) == 0 {
            call(mem, at(mem, 0x0062_f500), &[at(mem, 0x00b5_ec00), path]);
            return 0;
        }
        let file = call(mem, at(mem, 0x0067_9730), &[path, 1]);
        let size = call(mem, at(mem, 0x0067_9570), &[file]).wrapping_add(0xf) & !0xf;
        call(mem, at(mem, 0x0083_c4a0), &[0x40]);
        let padded = size.wrapping_add(0x7ff).cast_signed();
        let bias = if padded < 0 { 0x7ff } else { 0 };
        let alloc = padded.wrapping_add(bias).cast_unsigned() & !0x7ff;
        let buf = call(mem, at(mem, 0x0068_7020), &[alloc]);
        mem.write(buf, 0u32);
        call(mem, at(mem, 0x0067_9800), &[file, buf, size]);
        call(mem, at(mem, 0x0067_9510), &[file]);
        buf
    }

    /// Text code `07 p`: the pen's f32 x at `pen_x` moves right by `p -
    /// 0xe0` for `p` in `0xe1..=0xef` (the help and 16x12 text drawers).
    fn pen_step_x<M: Mem>(mem: &mut M, p: u32, pen_x: u32) {
        use ffx_bridge::x87::{add, load, store};
        let step = mem.read::<u8>(p + 1);
        if step > 0xe0 && step < 0xf0 {
            let step = store(f64::from(step - 0xe0));
            let moved = add(load(step), load(mem.read(pen_x)));
            mem.write(pen_x, store(moved));
        }
    }

    /// Text code `0a p`: the pen's flag byte gets low nibble `(p & 0xf) -
    /// 1` when that is nonzero and high nibble `(p & 0xf0) - 0x40` when
    /// that is at least `0x40`.
    fn pen_set_flags<M: Mem>(mem: &mut M, p: u32, pen_flags: u32) {
        let arg = mem.read::<u8>(p + 1);
        let mut f = mem.read::<u8>(pen_flags);
        if arg & 0xf != 0 {
            f = f & 0xf0 | ((arg & 0xf) - 1);
            mem.write(pen_flags, f);
        }
        if arg & 0xf0 >= 0x40 {
            f = f & 0xf | ((arg & 0xf0) - 0x40);
            mem.write(pen_flags, f);
        }
    }

    /// `DrawMesFontLClutTypeRGBA_ForHelp(key, text, x, y, flags, r, g, b,
    /// a)` (`0x008e75c0`, Fahrenheit's name): draws a help text through the
    /// pen at `0x025d09a0`.
    ///
    /// The pen: bytes `+0..+3` = `r, g, b, a` (low bytes), f32 `+4` x and
    /// `+8` y (from the ints), `+0xc..+0x17` zero, byte `+0x1b` `flags`, the
    /// rest of `+0x18` zero. The glyph record at `0x01841d50` (0x40 bytes)
    /// is cleared. `graphicFontGetScreenWH` fills two frame words nobody
    /// reads (`screen`). Then per code (`TOGetGroup`, jump table
    /// `0x008e788c`), stepping with `TOSkipCode`:
    ///
    /// - 1 (line break): x back to the start, `y += 16.0`.
    /// - 2 (control): `07 e1..ef` moves x by `byte - 0xe0`; `0a` sets the
    ///   flags' low nibble to `(p & 0xf) - 1` when that is nonzero and the
    ///   high nibble to `(p & 0xf0) - 0x40` when that is `>= 0x40`; `0b`
    ///   draws button icon `p`: `setButtonData(p, -1)`,
    ///   `TODrawScaleButtonIconRGBA(p, x, f32(y' + 3.0 +
    ///   AdjustButtonIconY(0.65, r, g, b, a)), 0.65, r, g, b, a)` with `y'` =
    ///   `y + 1` for icons `0x34..=0x37`, then `x += font space(0) + icon
    ///   width(p, 0.65)`. Every control code but `07` ends with
    ///   `resetButtonData()`.
    /// - 3 (macro): `TOAddPktChangeMacro_help(key, text, pen, 0.78, 1.0, 1)`
    ///   unless `key` is -1.
    /// - 4 (a character): `TOGetFontInfo`, then a shadow pass (draw type 1,
    ///   shadow type 0) and a plain pass (shadow type 1) of the 0.78 font,
    ///   draw type 0, and `TOAddFontWidthSpace` advances the pen.
    ///
    /// Its only caller (`0x0076f850`) ignores `eax`.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn draw_mes_font_lclut_type_rgba_for_help<M: Mem>(
        mem: &mut M,
        key: u32,
        text: u32,
        [left, top]: [i32; 2],
        flags: u32,
        rgba: [u32; 4],
        screen: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, load, store};
        const SCALE: u32 = 0x3f47_ae14; // 0.78
        const ICON_SCALE: u32 = 0x3f26_6666; // 0.65
        let at = |mem: &M, a: u32| mem.rebase(a);
        let (pen, glyph) = (at(mem, 0x025d_09a0), at(mem, 0x0184_1d50));
        let (pen_x, pen_y, pen_flags) = (pen + 4, pen + 8, pen + 0x1b);
        call(mem, at(mem, 0x0064_0db0), &[screen, screen + 4]);
        for k in 0..16 {
            mem.write(glyph + 4 * k, 0u32);
        }
        let colour = u32::from_le_bytes(rgba.map(|c| c.to_le_bytes()[0]));
        let start = store(f64::from(left));
        mem.write(pen, colour);
        mem.write(pen_x, start);
        mem.write(pen_y, store(f64::from(top)));
        for off in [0xc, 0x10, 0x14] {
            mem.write(pen + off, 0u32);
        }
        mem.write(pen + 0x18, (flags & 0xff) << 24);
        let group = at(mem, 0x008d_f610);
        let skip = at(mem, 0x008d_fd80);
        let mut p = text;
        loop {
            let code = mem.read::<u8>(p);
            if code == 0 {
                break;
            }
            match call(mem, group, &[u32::from(code)]) {
                1 => {
                    mem.write(pen_x, store(load(start)));
                    let down = add(load(mem.read(pen_y)), 16.0);
                    mem.write(pen_y, store(down));
                }
                2 if code == 7 => pen_step_x(mem, p, pen_x),
                2 => {
                    if code == 0xa {
                        pen_set_flags(mem, p, pen_flags);
                    }
                    if code == 0xb {
                        let icon = u32::from(mem.read::<u8>(p + 1));
                        let mut top = store(load(mem.read(pen_y)));
                        if (0x34..=0x37).contains(&icon) {
                            top = store(add(load(top), 1.0));
                        }
                        call(mem, at(mem, 0x008b_1350), &[icon, u32::MAX]);
                        let raise = fcall(
                            mem,
                            at(mem, 0x008e_5280),
                            &[ICON_SCALE, rgba[0], rgba[1], rgba[2], rgba[3]],
                        );
                        let top = store(add(raise, add(load(top), 3.0)));
                        let x = store(load(mem.read(pen_x)));
                        let draw = at(mem, 0x008f_8ee0);
                        call(
                            mem,
                            draw,
                            &[icon, x, top, ICON_SCALE, rgba[0], rgba[1], rgba[2], rgba[3]],
                        );
                        let width = fcall(mem, at(mem, 0x008f_a800), &[icon, ICON_SCALE]);
                        let space = fcall(mem, at(mem, 0x008f_a570), &[0]);
                        let moved = add(add(space, width), load(mem.read(pen_x)));
                        mem.write(pen_x, store(moved));
                    }
                    call(mem, at(mem, 0x008b_1330), &[]);
                }
                3 => {
                    if key != u32::MAX {
                        let macro_help = at(mem, 0x008e_9ca0);
                        call(mem, macro_help, &[key, p, pen, SCALE, 1.0f32.to_bits(), 1]);
                    }
                }
                4 => {
                    let (draw_type, shadow) = (at(mem, 0x0064_3af0), at(mem, 0x0064_0dd0));
                    call(mem, at(mem, 0x008b_7240), &[p, glyph]);
                    call(mem, draw_type, &[1]);
                    call(mem, shadow, &[0]);
                    call(mem, at(mem, 0x008f_bae0), &[pen, glyph, SCALE]);
                    call(mem, shadow, &[1]);
                    call(mem, at(mem, 0x008f_b940), &[pen, glyph, SCALE]);
                    call(mem, draw_type, &[0]);
                    call(mem, at(mem, 0x008e_8ed0), &[pen, glyph, SCALE]);
                }
                _ => {}
            }
            p = call(mem, skip, &[p]);
        }
    }

    /// `MenuItem_DrawKeyUseCells()` (`0x008c4770`, name ours): the key-use
    /// cells under the item menu. `MenuItem_CellTable()` gives the record
    /// whose u16 `+2` is the cell count `n`.
    ///
    /// - The plate: `TODrawMenuPlateXYWHType(remapX2(790), remapY2(280),
    ///   remapX2(740), remapY2(60), 2)` (remapped h, w, y, x); its x, y are
    ///   also kept as `i16` (`ftol`). The original also remaps 740 and 60
    ///   once more and drops the results (pure; left out).
    /// - The row is centred: `shift = (8 - n) * i32 0x0186a31c / 2` (toward
    ///   zero), first cell at `x = f32(remapX2(167) + x16 + f32(shift))`.
    /// - Names: from `y = f32(remapY2(12) + y16)`, the 4-byte texts at
    ///   `0x0186a280` until an empty one, each centred in its cell,
    ///   `f32((remapX2(42) - width) * 0.5 + x)` (`ToGetBtlEasyFontWidth` at
    ///   0.78), by `ToMakeBtlEasyFont`,
    ///   `x += remapX2(52)`.
    /// - Frames: `y = f32(remapY2(48) + y16)` (and `remapY2(50)`, dropped),
    ///   cell `i` gets `TkMn2DrawCrossRect(x, y, remapX2(42), remapY2(2), c,
    ///   c)`: colour `0x8000c6ff`, except for the cell to fill (empty and the
    ///   first or after a filled one, or the last when filled), which blinks
    ///   `0x80e1f8ff` while the fraction of `f32(graphicGetTime())` is above
    ///   0.5.
    ///
    /// `width` is the original's frame word for the text width. Its only
    /// caller (`0x008c3d00`) ignores `eax`.
    pub fn menu_item_draw_key_use_cells<M: Mem>(
        mem: &mut M,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, ftol, load, mul, store, sub};
        const SCALE: u32 = 0x3f47_ae14; // 0.78
        const ONE: u32 = 0x3f80_0000;
        let at = |mem: &M, a: u32| mem.rebase(a);
        let table = call(mem, at(mem, 0x008c_4f80), &[]);
        let count = |mem: &M| i32::from(mem.read::<u16>(table.wrapping_add(2)));
        // `mov si, ax`: the low half of the truncation, sign-extended later
        let low_half = |v: u32| {
            let [a, b, ..] = v.to_le_bytes();
            f64::from(i16::from_le_bytes([a, b]))
        };
        let base_x = remap(mem, false, 790.0);
        let x16 = low_half(ftol(mem, base_x));
        let base_y = remap(mem, true, 280.0);
        let y16 = low_half(ftol(mem, base_y));
        let plate_h = store(remap(mem, true, 60.0));
        let plate_w = store(remap(mem, false, 740.0));
        let plate = [store(base_x), store(base_y), plate_w, plate_h, 2];
        call(mem, at(mem, 0x008f_5f70), &plate);
        let scale = mem.read::<i32>(at(mem, 0x0186_a31c));
        let empty = 8i32.wrapping_sub(count(mem));
        let shift = load(store(f64::from(empty.wrapping_mul(scale) / 2)));
        let mut left = store(add(add(remap(mem, false, 167.0), x16), shift));
        let top = store(add(remap(mem, true, 12.0), y16));
        let names = at(mem, 0x0186_a280);
        let mut slot = 0;
        while slot < count(mem) {
            let text = names.wrapping_add(4 * slot.cast_unsigned());
            if mem.read::<u8>(text) == 0 {
                break;
            }
            call(mem, at(mem, 0x0090_5290), &[text, width, 0, SCALE, ONE]);
            let centred = mul(sub(remap(mem, false, 42.0), load(mem.read(width))), 0.5);
            let tx = store(add(centred, load(left)));
            call(mem, at(mem, 0x0090_5ab0), &[text, tx, top, 0, SCALE, ONE]);
            left = store(add(remap(mem, false, 52.0), load(left)));
            slot += 1;
        }
        left = store(add(add(remap(mem, false, 167.0), x16), shift));
        let top = store(add(remap(mem, true, 48.0), y16));
        let mut slot = 0;
        while slot < count(mem) {
            let text = names.wrapping_add(4 * slot.cast_unsigned());
            let filled = mem.read::<u8>(text) != 0;
            let next = if filled {
                slot == count(mem) - 1
            } else {
                slot == 0 || mem.read::<u8>(text - 4) != 0
            };
            let colour = if next {
                let time = store(fcall(mem, at(mem, 0x0064_1410), &[]));
                let whole = store(f64::from(ftol(mem, load(time)).cast_signed()));
                let fraction = load(store(sub(load(time), load(whole))));
                (fraction > 0.5).then_some(0x80e1_f8ff)
            } else {
                Some(0x8000_c6ff)
            };
            if let Some(colour) = colour {
                let rect_h = store(remap(mem, true, 2.0));
                let rect_w = store(remap(mem, false, 42.0));
                let rect = [left, top, rect_w, rect_h, colour, colour];
                call(mem, at(mem, 0x008c_0930), &rect);
            }
            left = store(add(remap(mem, false, 52.0), load(left)));
            slot += 1;
        }
    }

    /// `TkMn2MainStatusPlayerStatus(_, y, chr, t)` (`0x008d5180`,
    /// Fahrenheit's name): the stat plates of the status page and, for
    /// anyone but ids 14..=17, the plate under them.
    ///
    /// - `slide = MenuMn2_Ease(t, 1) * 0xd0 / 4096` (toward zero).
    /// - The stats `(mes, field)` at `0x00c5a790` (8-byte pairs, up to a
    ///   negative `mes`): from `x = ftol(remapX2(210))`, `y0 =
    ///   ftol(remapY2(552))`, the fifth starting a second column at
    ///   `remapX2(970)`, `remapY2(552)`. Each:
    ///   `TkMn2DrawStatusPlate(x, MenuHelp_Ratio(slide, i % 4) + y0 + y,
    ///   i16 0x023cc420, i16 0x023cc422, chr, mes, field)`, then `y0 =
    ///   ftol(remapY2(81) + f32(y0))` and `TkMn2DrawKickSyncPacket2()`.
    ///   `MsGetSavePlayerPtr(chr)` is asked before each entry and its
    ///   result dropped.
    /// - The plate: `w = f32(remapX2(80) + width)` of
    ///   `TOMenuGetStatusMes(0x1a)` at 0.78 (`ToGetCrossExtMesFontWidth`);
    ///   `right = remapX2(740)` unless `remapX2(660) < w`, then
    ///   `f32(remapX2(80) + w)`; `left = f32(remapX2(1806) - right)`;
    ///   `TODrawMenuPlateXYWHType(left, remapY2(925), right, remapY2(48),
    ///   2)`, shape `0xf3` at `(tx, remapY2(920))` sized `remapX2(64)` by
    ///   `remapY2(64)` with texels `(949, 234, 1023, 309) / 1024`, where
    ///   `tx = f32((right - w) * 0.5 + left)`, and the text at
    ///   `f32(remapX2(80) + tx)`, `remapY2(928)` (0.78).
    ///
    /// Returns `TkMn2DrawKickSyncPacket()`'s `eax`. `width` is the
    /// original's frame word for the text width.
    #[allow(clippy::too_many_lines)]
    pub fn tk_mn2_main_status_player_status<M: Mem>(
        mem: &mut M,
        y: i32,
        chr: i32,
        t: u32,
        width: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f64,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{add, ftol, load, mul, store, sub};
        const SCALE: u32 = 0x3f47_ae14; // 0.78
        const ONE: u32 = 0x3f80_0000;
        let at = |mem: &M, a: u32| mem.rebase(a);
        let id = chr.cast_unsigned();
        let eased = call(mem, at(mem, 0x008d_30c0), &[t, 1]).cast_signed();
        let slide = eased.wrapping_mul(0xd0) / 4096;
        let column = |mem: &mut M, remap: &mut dyn FnMut(&mut M, bool, f32) -> f64, x: f32| {
            let left = remap(mem, false, x);
            let left = ftol(mem, left);
            let top = remap(mem, true, 552.0);
            (left, ftol(mem, top).cast_signed())
        };
        let (mut left, mut top) = column(mem, remap, 210.0);
        let player = at(mem, 0x0078_5330);
        call(mem, player, &[id]);
        let stats = at(mem, 0x00c5_a790);
        let mut i = 0u32;
        loop {
            let mes = mem.read::<i32>(stats.wrapping_add(8 * i));
            if mes < 0 {
                break;
            }
            let field = mem.read::<u32>(stats.wrapping_add(8 * i + 4));
            if i == 4 {
                (left, top) = column(mem, remap, 970.0);
            }
            let ratio = call(mem, at(mem, 0x008b_f120), &[slide.cast_unsigned(), i & 3]);
            let row_y = ratio
                .wrapping_add(top.cast_unsigned())
                .wrapping_add(y.cast_unsigned());
            let sx = i32::from(mem.read::<i16>(at(mem, 0x023c_c420))).cast_unsigned();
            let sy = i32::from(mem.read::<i16>(at(mem, 0x023c_c422))).cast_unsigned();
            let plate = at(mem, 0x008c_1490);
            call(
                mem,
                plate,
                &[left, row_y, sx, sy, id, mes.cast_unsigned(), field],
            );
            let step = remap(mem, true, 81.0);
            top = ftol(mem, add(step, load(store(f64::from(top))))).cast_signed();
            i += 1;
            call(mem, at(mem, 0x008c_0cd0), &[]);
            call(mem, player, &[id]);
        }
        if !(14..=17).contains(&chr) {
            let get_mes = at(mem, 0x008b_ee70);
            let text = call(mem, get_mes, &[0x1a]);
            call(mem, at(mem, 0x0090_5320), &[0, text, width, SCALE, ONE]);
            let wide = store(add(remap(mem, false, 80.0), load(mem.read(width))));
            let limit = remap(mem, false, 660.0);
            // `fcomp; test ah, 5; jp`: 740 unless the limit is below (ordered)
            let right = if limit < load(wide) {
                store(add(remap(mem, false, 80.0), load(wide)))
            } else {
                store(remap(mem, false, 740.0))
            };
            let screen = remap(mem, false, 1806.0);
            let plate_x = store(sub(screen, load(right)));
            let text_x = store(add(mul(sub(load(right), load(wide)), 0.5), load(plate_x)));
            let h = store(remap(mem, true, 48.0));
            let plate_y = store(remap(mem, true, 925.0));
            call(mem, at(mem, 0x008f_5f70), &[plate_x, plate_y, right, h, 2]);
            let sh = store(remap(mem, true, 64.0));
            let sw = store(remap(mem, false, 64.0));
            let sy = store(remap(mem, true, 920.0));
            let uv = [949.0f32, 234.0, 1023.0, 309.0].map(|v| (v / 1024.0).to_bits());
            let shape = at(mem, 0x0090_3bb0);
            call(
                mem,
                shape,
                &[0xf3, text_x, sy, sw, sh, uv[0], uv[1], uv[2], uv[3]],
            );
            let ty = store(remap(mem, true, 928.0));
            let tx = store(add(remap(mem, false, 80.0), load(text_x)));
            let text = call(mem, get_mes, &[0x1a]);
            call(mem, at(mem, 0x0090_16b0), &[0, text, tx, ty, 0, SCALE, ONE]);
        }
        call(mem, at(mem, 0x008c_0cc0), &[])
    }

    /// `Pad_InitWork(port, slot)` (`0x00889360`, name ours): resets one
    /// pad's work record (`TkGetControlerWork(port, slot)`): u16 `+0x9d` =
    /// `0x100`, byte `+0x9f` = 1, `+0x94` = `0x5000c`, then
    /// `Pad_ResetMode(work)`, `op_slot_bytes_reset_008888e0(work)`,
    /// `Pad_ResetSlots_00888910(work)`, `Pad_ClearAnalogWork(port, slot)`,
    /// bytes `+0x9c` and `+0x83` = 0, and the identity button map
    /// (`TkSetPadButtonMap(port, slot, i, i)` for `i` in 0..16). Its only
    /// caller (`0x00889340`, ports 0 and 1) ignores `eax`.
    pub fn pad_init_work<M: Mem>(
        mem: &mut M,
        port: u32,
        slot: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let work = tk_get_controler_work(mem, port.cast_signed(), slot.cast_signed());
        mem.write(work.wrapping_add(0x9d), 0x100u16);
        mem.write(work.wrapping_add(0x9f), 1u8);
        mem.write(work.wrapping_add(0x94), 0x0005_000cu32);
        for f in [0x0088_8ee0, 0x0088_88e0, 0x0088_8910] {
            let f = mem.rebase(f);
            call(mem, f, &[work]);
        }
        let clear = mem.rebase(0x0088_89f0);
        call(mem, clear, &[port, slot]);
        mem.write(work.wrapping_add(0x9c), 0u8);
        mem.write(work.wrapping_add(0x83), 0u8);
        let map = mem.rebase(0x0088_94f0);
        for button in 0..16 {
            call(mem, map, &[port, slot, button, button]);
        }
    }

    /// `AtelEvent_FreeMovie()` (`0x00874130`, name ours): by the event
    /// save's `+0xe0` (`TkGetSaveEventAddress`, jump table `0x008741b8` on
    /// 1..=5): 1, 2, 3 and 5 keep everything; 4 only stops the movie
    /// (`movie_stop_prog`); anything else stops it, frees the buffers at
    /// `0x01327118`, `0x01327114` and `0x01327110` (`TkFree`, then 0) and
    /// clears `+0xe0`. Returns the `eax` the original leaves (`kind - 1`,
    /// the movie stop's, or the event save's address).
    pub fn atel_event_free_movie<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let event = mem.rebase(0x0088_c830);
        let save = call(mem, event, &[]);
        let kind = mem.read::<u32>(save.wrapping_add(0xe0));
        let stop = mem.rebase(0x0076_f090);
        match kind {
            1 | 2 | 3 | 5 => return kind - 1,
            4 => return call(mem, stop, &[]),
            _ => {}
        }
        call(mem, stop, &[]);
        for buffer in [0x0132_7118, 0x0132_7114, 0x0132_7110] {
            let at = mem.rebase(buffer);
            let p = mem.read::<u32>(at);
            if p != 0 {
                let free = mem.rebase(0x0088_d520);
                call(mem, free, &[p]);
                mem.write(at, 0u32);
            }
        }
        let save = call(mem, event, &[]);
        mem.write(save.wrapping_add(0xe0), 0u32);
        save
    }

    /// `MenuWShop_InitList()` (`0x008d94a0`, name ours): the weapon shop's
    /// stock. `0x0186ab14` = 1 when one of the owned weapons
    /// (`MsGetSaveWeaponList(-1, -1, 0, &count)`) has a zero byte `+2`
    /// (`MsGetSaveWeapon(id, &name)`), else 0. Then for the u16 ids from
    /// `+2` of `TkMn2GetExcelData(0x0186ab1c, 0x0186aba8)`, up to a zero or
    /// 16 of them: the record `MenuWShop_GetRecord(id)` gets byte `+6` =
    /// `0xff` and `MsSetWeaponName`; the count goes to `0x0186ab10`.
    /// `count` and `name` are the original's frame words. Returns the
    /// `eax` the original leaves.
    pub fn menu_wshop_init_list<M: Mem>(
        mem: &mut M,
        [count, name]: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let excel = [0x0186_ab1c, 0x0186_aba8].map(|g| mem.read::<u32>(at(mem, g)));
        let list = call(mem, at(mem, 0x008c_1af0), &excel);
        let max = u32::MAX;
        let owned = call(mem, at(mem, 0x007a_bd10), &[max, max, 0, count]);
        let unnamed = at(mem, 0x0186_ab14);
        mem.write(unnamed, 0u32);
        let mut i = 0;
        while i < mem.read::<i32>(count) {
            let id = u32::from(mem.read::<u16>(owned.wrapping_add(2 * i.cast_unsigned())));
            let weapon = call(mem, at(mem, 0x007a_bbd0), &[id, name]);
            if mem.read::<u8>(weapon.wrapping_add(2)) == 0 {
                mem.write(unnamed, 1u32);
                break;
            }
            i += 1;
        }
        let mut n = 0u32;
        let mut eax = 0;
        while n < 16 {
            let id = u32::from(mem.read::<u16>(list.wrapping_add(2 + 2 * n)));
            if id == 0 {
                eax = 0;
                break;
            }
            let record = call(mem, at(mem, 0x008d_9170), &[id]);
            mem.write(record.wrapping_add(6), 0xffu8);
            eax = call(mem, at(mem, 0x0079_93b0), &[record]);
            n += 1;
        }
        mem.write(at(mem, 0x0186_ab10), n);
        eax
    }

    /// `MenuItem_KeyRowCount()` (`0x008c5180`, name ours): rows of three in
    /// the current key layout. Japanese without the English setting: the
    /// u16 list `0x00c5a2f0[i32 0x0186a25c]`; languages 10, 9 and 11: with
    /// no item target (`0x0186a258` < 0) `PasswordGetTotalIndex()` rounded
    /// up, else the list `0x00c5a260`; English (1, or 0 with the English
    /// setting) `0x00c5a2c8`; others `0x00c5a288`. A list counts its
    /// entries up to a zero; the result is that `/ 3`. The language is
    /// asked again before each test (`TOGetFFXLangJmp`).
    pub fn menu_item_key_row_count<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let lang = at(mem, 0x008a_c2f0);
        let list = if call(mem, lang, &[]) == 0 {
            if call(mem, at(mem, 0x0078_51d0), &[]) == 0 {
                let mode = mem.read::<u32>(at(mem, 0x0186_a25c));
                mem.read::<u32>(at(mem, 0x00c5_a2f0).wrapping_add(mode.wrapping_mul(4)))
            } else {
                at(mem, 0x00c5_a2c8)
            }
        } else if call(mem, lang, &[]) == 10
            || call(mem, lang, &[]) == 9
            || call(mem, lang, &[]) == 11
        {
            if mem.read::<i32>(at(mem, 0x0186_a258)) < 0 {
                let total = at(mem, 0x0079_02d0);
                let up = call(mem, total, &[]).cast_signed() % 3 != 0;
                let rows = call(mem, total, &[]).cast_signed() / 3;
                return if up { rows + 1 } else { rows };
            }
            at(mem, 0x00c5_a260)
        } else if call(mem, lang, &[]) == 1 {
            at(mem, 0x00c5_a2c8)
        } else {
            at(mem, 0x00c5_a288)
        };
        let mut n = 0i32;
        while mem.read::<u16>(list.wrapping_add(2 * n.cast_unsigned())) != 0 {
            n += 1;
        }
        n / 3
    }

    /// `MenuItem_TypeKey()` (`0x008c4c90`, name ours): puts the character
    /// under the key cursor into the next key-use cell. The layout is
    /// [`menu_item_key_row_count`]'s, except that languages 9, 10 and 11
    /// without an item target take the password entries: the cursor
    /// `i32 0x0186a268` gives row `/ 5` and column `% 5` (toward zero). The
    /// row's text is `PasswordText_GetEntry(row)` there (nothing past
    /// `PasswordGetTotalIndex()` rows), else `TOMenuGetNameMes(u16
    /// list[row])`; the column skips that many characters (codes
    /// `0x06..=0x2f` are two bytes, `04` with a byte below `0x30` three).
    /// The cell is the first empty one of the 4-byte cells at `0x0186a280`,
    /// at most the table's u16 `+2` minus 1. A text that ends before the
    /// column empties the cell (and the next cell's first byte); otherwise
    /// the character's bytes go in, then a zero and an empty next cell.
    pub fn menu_item_type_key<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let at = |mem: &M, a: u32| mem.rebase(a);
        let table = call(mem, at(mem, 0x008c_4f80), &[]);
        let lang = at(mem, 0x008a_c2f0);
        let target = |mem: &M| mem.read::<i32>(at(mem, 0x0186_a258));
        let list = if call(mem, lang, &[]) == 0 {
            if call(mem, at(mem, 0x0078_51d0), &[]) == 0 {
                let mode = mem.read::<u32>(at(mem, 0x0186_a25c));
                mem.read::<u32>(at(mem, 0x00c5_a2f0).wrapping_add(mode.wrapping_mul(4)))
            } else {
                at(mem, 0x00c5_a2c8)
            }
        } else if call(mem, lang, &[]) == 10
            || call(mem, lang, &[]) == 9
            || call(mem, lang, &[]) == 11
        {
            if target(mem) < 0 {
                table
            } else {
                at(mem, 0x00c5_a260)
            }
        } else if call(mem, lang, &[]) == 1 {
            at(mem, 0x00c5_a2c8)
        } else {
            at(mem, 0x00c5_a288)
        };
        let cursor = mem.read::<i32>(at(mem, 0x0186_a268));
        let (row, mut column) = (cursor / 5, cursor % 5);
        let password =
            (call(mem, lang, &[]) == 10 || call(mem, lang, &[]) == 9 || call(mem, lang, &[]) == 11)
                && target(mem) < 0;
        let mut p = if password {
            let total = call(mem, at(mem, 0x0079_02d0), &[]).cast_signed();
            if row >= total {
                return;
            }
            call(mem, at(mem, 0x0079_02a0), &[row.cast_unsigned()])
        } else {
            let id = mem.read::<u16>(list.wrapping_add(row.cast_unsigned().wrapping_mul(2)));
            call(mem, at(mem, 0x008b_edd0), &[u32::from(id)])
        };
        while column > 0 {
            let c = mem.read::<u8>(p);
            p = p.wrapping_add(1);
            if c == 0 {
                break;
            }
            if c.wrapping_sub(6) <= 0x29 {
                p = p.wrapping_add(1);
            } else if c == 4 {
                let arg = mem.read::<u8>(p);
                p = p.wrapping_add(1);
                if arg < 0x30 {
                    p = p.wrapping_add(1);
                }
            }
            column -= 1;
        }
        let keys = at(mem, 0x0186_a280);
        let mut k = 0u32;
        while mem.read::<u8>(keys.wrapping_add(4 * k)) != 0 {
            k += 1;
        }
        let cells = i32::from(mem.read::<u16>(table.wrapping_add(2)));
        if k.cast_signed() >= cells {
            k = (cells - 1).cast_unsigned();
        }
        let key = keys.wrapping_add(k.wrapping_mul(4));
        if column > 0 {
            mem.write(key, 0u8);
            mem.write(key.wrapping_add(4), 0u8);
            return;
        }
        let c = mem.read::<u8>(p);
        mem.write(key, c);
        if c.wrapping_sub(6) <= 0x29 {
            mem.write(key.wrapping_add(1), mem.read::<u8>(p.wrapping_add(1)));
        } else if c == 4 {
            let arg = mem.read::<u8>(p.wrapping_add(1));
            mem.write(key.wrapping_add(1), arg);
            if arg < 0x30 {
                mem.write(key.wrapping_add(2), mem.read::<u8>(p.wrapping_add(2)));
                mem.write(key.wrapping_add(3), 0u8);
                mem.write(key.wrapping_add(4), 0u8);
                return;
            }
        } else {
            mem.write(key.wrapping_add(1), 0u8);
            mem.write(key.wrapping_add(4), 0u8);
            return;
        }
        mem.write(key.wrapping_add(2), 0u8);
        mem.write(key.wrapping_add(4), 0u8);
    }

    /// `TkMenuGetDispCommand(kind, sub, a, chr, out)` (`0x008d2e10`,
    /// Fahrenheit's name): the commands a menu shows, into the words at
    /// `out`. The list is `MsMenuStatusNum(0, &n)` from entry
    /// `TkMenuGetCommandOffset(list, kind, sub, a)`, `max =
    /// TkMenuGetCommandMax(..)` of them (4-byte entries, the u16 id at
    /// `+2`); `out[..max]` is cleared first.
    ///
    /// - `sub` 1: entry by entry, the id when that party member has joined
    ///   (`MsGetSavePlyJoin`; id 100 asks member 15), else 0. Returns
    ///   `max`.
    /// - `kind` 0: the abilities of the character's weapon and armour
    ///   (`MsGetSaveWeapon` u16 `+0xe..+0x14`), packed, skipping 0 and
    ///   `0xff`.
    /// - otherwise the commands `MsGetSaveCommand(chr, id)` has, packed;
    ///   for a party member (`chr < 8`) a missing one leaves a gap when
    ///   `MsGetSaveCommandMenuPanel(id)` is set and lowers the count when
    ///   it is not. (`sub` `0x40..=0x47` also asks `MsGetSavePlayerPtr`,
    ///   the others `TOGetSaveGilAddress`; both results dropped.)
    ///
    /// Returns the count for `chr < 8`, else the commands found. `scratch`
    /// is the original's two frame words.
    #[allow(clippy::too_many_arguments)]
    pub fn tk_menu_get_disp_command<M: Mem>(
        mem: &mut M,
        kind: u32,
        sub: u32,
        a: u32,
        chr: i32,
        out: u32,
        scratch: [u32; 2],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> i32 {
        let at = |mem: &M, f: u32| mem.rebase(f);
        let id = chr.cast_unsigned();
        let list = call(mem, at(mem, 0x0079_0200), &[0, scratch[0]]);
        let mut max = call(mem, at(mem, 0x008d_2d80), &[list, kind, sub, a]).cast_signed();
        let offset = call(mem, at(mem, 0x008d_2dc0), &[list, kind, sub, a]);
        let entries = list.wrapping_add(offset.wrapping_mul(4)).wrapping_add(2);
        let entry = |mem: &M, k: u32| u32::from(mem.read::<u16>(entries.wrapping_add(4 * k)));
        let count = max.max(0).cast_unsigned();
        for k in 0..count {
            mem.write(out.wrapping_add(4 * k), 0u32);
        }
        let mut out = out;
        if sub == 1 {
            let join = at(mem, 0x0078_5380);
            for k in 0..count {
                let command = entry(mem, k);
                let member = if command == 100 { 0xf } else { command };
                if call(mem, join, &[member]) != 0 {
                    mem.write(out, command);
                }
                out = out.wrapping_add(4);
            }
            return max;
        }
        let mut found = 0;
        if kind == 0 {
            let weapon = call(mem, at(mem, 0x008a_9c70), &[id]);
            let armour = call(mem, at(mem, 0x008a_9820), &[id]);
            for gear in [weapon, armour] {
                if gear == 0xff {
                    continue;
                }
                let record = call(mem, at(mem, 0x007a_bbd0), &[gear, scratch[1]]);
                for field in [0xe, 0x10, 0x12, 0x14] {
                    let ability = u32::from(mem.read::<u16>(record.wrapping_add(field)));
                    if ability != 0 && ability != 0xff {
                        mem.write(out, ability);
                        out = out.wrapping_add(4);
                    }
                }
            }
        } else {
            if sub.wrapping_sub(0x40) <= 7 {
                call(mem, at(mem, 0x0078_5330), &[id]);
            } else {
                call(mem, at(mem, 0x008a_8e20), &[]);
            }
            for k in 0..count {
                let command = entry(mem, k);
                if call(mem, at(mem, 0x0078_5020), &[id, command]) != 0 {
                    mem.write(out, command);
                    out = out.wrapping_add(4);
                    found += 1;
                } else if chr < 8 {
                    if call(mem, at(mem, 0x0078_50b0), &[command]) == 0 {
                        max -= 1;
                    } else {
                        out = out.wrapping_add(4);
                    }
                }
            }
        }
        if chr < 8 { max } else { found }
    }

    /// `MenuMes_AddPktFont1612(cursor, key, text, x, y, flags, edge, r, g,
    /// b, a)` (`0x008e4af0`, name ours): 16x12 text into the packet at
    /// `cursor` (`g_TOPktCursor` `0x018716cc`, returned when done) through
    /// the pen at `0x025d09a0`, like [`draw_mes_font_lclut_type_rgba_for_help`]
    /// with these differences:
    ///
    /// - The pen's byte `+0x1a` is `edge`, `+0x1b` `flags`.
    /// - English (`TOGetFFXLangJmp()` 1): with `y` 0x163 a leading
    ///   `48 73 78 82 7f` is skipped; at `(0xd6, 0xbb)` a text starting
    ///   with `h` starts at x 200.
    /// - Characters: with `edge` 1 the edge font (`0x00906260`) then the
    ///   face (`0x00906270`), else shadow (`0x00906290`) and face between
    ///   draw/shadow types; `y` 0x163 lowers each by 1 (Japanese) or 2.
    /// - `0b` icons: scale 0.73 at `f32(y + 4.5 + AdjustButtonIconY(..))`,
    ///   then `x += width + 1`; only `0b` calls `resetButtonData()`.
    /// - Macros: `TOAddPktChangeMacro(key, text, pen)` unless `key` is -1.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn menu_mes_add_pkt_font1612<M: Mem>(
        mem: &mut M,
        cursor: u32,
        key: u32,
        text: u32,
        [left, top]: [i32; 2],
        [flags, edge]: [u32; 2],
        rgba: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        const SCALE: u32 = 0x3f47_ae14; // 0.78
        const ICON_SCALE: u32 = 0x3f3a_e148; // 0.73
        const STAMP: i32 = 0x163;
        let at = |mem: &M, a: u32| mem.rebase(a);
        let (pen, glyph) = (at(mem, 0x025d_09a0), at(mem, 0x0184_1d50));
        let (pen_x, pen_y, pen_flags) = (pen + 4, pen + 8, pen + 0x1b);
        let packet = at(mem, 0x0187_16cc);
        for k in 0..16 {
            mem.write(glyph + 4 * k, 0u32);
        }
        for off in [0xc, 0x10, 0x14] {
            mem.write(pen + off, 0u32);
        }
        mem.write(packet, cursor);
        let start = store(f64::from(left));
        mem.write(pen, u32::from_le_bytes(rgba.map(|c| c.to_le_bytes()[0])));
        mem.write(pen_x, start);
        mem.write(pen_y, store(f64::from(top)));
        mem.write(pen + 0x18, ((flags & 0xff) << 24) | ((edge & 0xff) << 16));
        let lang = at(mem, 0x008a_c2f0);
        let mut p = text;
        if call(mem, lang, &[]) == 1 {
            let stamp = [0x48u8, 0x73, 0x78, 0x82, 0x7f];
            if top == STAMP && (0..5).all(|k| mem.read::<u8>(p + k) == stamp[k as usize]) {
                p += 5;
            }
            if left == 0xd6 && top == 0xbb && mem.read::<u8>(p) == b'h' {
                mem.write(pen_x, 200.0f32.to_bits());
            }
        }
        let group = at(mem, 0x008d_f610);
        let skip = at(mem, 0x008d_fd80);
        loop {
            let code = mem.read::<u8>(p);
            if code == 0 {
                break;
            }
            match call(mem, group, &[u32::from(code)]) {
                1 => {
                    mem.write(pen_x, store(load(start)));
                    let down = add(load(mem.read(pen_y)), 16.0);
                    mem.write(pen_y, store(down));
                }
                2 if code == 7 => pen_step_x(mem, p, pen_x),
                2 => {
                    if code == 0xa {
                        pen_set_flags(mem, p, pen_flags);
                    }
                    if code == 0xb {
                        let icon = u32::from(mem.read::<u8>(p + 1));
                        call(mem, at(mem, 0x008b_1350), &[icon, u32::MAX]);
                        let colour = [rgba[0], rgba[1], rgba[2], rgba[3]];
                        let adjust = at(mem, 0x008e_5280);
                        let raise = fcall(
                            mem,
                            adjust,
                            &[ICON_SCALE, colour[0], colour[1], colour[2], colour[3]],
                        );
                        let y = store(add(raise, add(load(mem.read(pen_y)), 4.5)));
                        let x = store(load(mem.read(pen_x)));
                        let draw = at(mem, 0x008f_8ee0);
                        call(
                            mem,
                            draw,
                            &[
                                icon, x, y, ICON_SCALE, colour[0], colour[1], colour[2], colour[3],
                            ],
                        );
                        let width = fcall(mem, at(mem, 0x008f_a800), &[icon, ICON_SCALE]);
                        let moved = add(add(width, 1.0), load(mem.read(pen_x)));
                        mem.write(pen_x, store(moved));
                        call(mem, at(mem, 0x008b_1330), &[]);
                    }
                }
                3 => {
                    if key != u32::MAX {
                        call(mem, at(mem, 0x008e_9b80), &[key, p, pen]);
                    }
                }
                4 => {
                    call(mem, at(mem, 0x008b_7240), &[p, glyph]);
                    if top == STAMP {
                        let rise = if call(mem, lang, &[]) == 0 { 1.0 } else { 2.0 };
                        mem.write(pen_y, store(add(load(mem.read(pen_y)), rise)));
                    }
                    let face = at(mem, 0x0090_6270);
                    if mem.read::<u8>(pen + 0x1a) == 1 {
                        call(mem, at(mem, 0x0090_6260), &[pen, glyph]);
                        call(mem, face, &[pen, glyph]);
                    } else {
                        let (draw_type, shadow) = (at(mem, 0x0064_3af0), at(mem, 0x0064_0dd0));
                        call(mem, draw_type, &[1]);
                        call(mem, shadow, &[0]);
                        call(mem, at(mem, 0x0090_6290), &[pen, glyph]);
                        call(mem, shadow, &[1]);
                        call(mem, face, &[pen, glyph]);
                        call(mem, draw_type, &[0]);
                    }
                    call(mem, at(mem, 0x008e_8ed0), &[pen, glyph, SCALE]);
                    if top == STAMP {
                        let rise = if call(mem, lang, &[]) == 0 { 1.0 } else { 2.0 };
                        mem.write(pen_y, store(sub(load(mem.read(pen_y)), rise)));
                    }
                }
                _ => {}
            }
            p = call(mem, skip, &[p]);
        }
        mem.read(packet)
    }

    /// `MenuMes_AddPktFont1612Icons(cursor, key, text, x, y, flags, edge,
    /// r, g, b, a)` (`0x008e4e60`, name ours): the sibling of
    /// [`menu_mes_add_pkt_font1612`] without the English stamp checks, and:
    ///
    /// - Characters: with `edge` 1 the edge font and the face, no rise;
    ///   else, with `y` 0x163, the pen goes up by 1 (Japanese) or 3 for the
    ///   shadow and face passes and back before `TOAddFontWidthSpace`.
    /// - `0b` icons at scale 0.73: Japanese at `f32(y + 4.0 +
    ///   AdjustButtonIconY(..))`; other languages at `f32(y' + 5.0)` with
    ///   `y' = f32(y - 2.5)` for icons `0x34..=0x37`, `0x20..=0x22`, `0x42`,
    ///   and `0x48`/`0x30` while `350.0 > y`, else `y' = y`. Then `x +=
    ///   width + 1`. Every control code but `07` ends with
    ///   `resetButtonData()`.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn menu_mes_add_pkt_font1612_icons<M: Mem>(
        mem: &mut M,
        cursor: u32,
        key: u32,
        text: u32,
        [left, top]: [i32; 2],
        [flags, edge]: [u32; 2],
        rgba: [u32; 4],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        use ffx_bridge::x87::{add, load, store, sub};
        const SCALE: u32 = 0x3f47_ae14; // 0.78
        const ICON_SCALE: u32 = 0x3f3a_e148; // 0.73
        const STAMP: i32 = 0x163;
        let at = |mem: &M, a: u32| mem.rebase(a);
        let (pen, glyph) = (at(mem, 0x025d_09a0), at(mem, 0x0184_1d50));
        let (pen_x, pen_y, pen_flags) = (pen + 4, pen + 8, pen + 0x1b);
        let packet = at(mem, 0x0187_16cc);
        for k in 0..16 {
            mem.write(glyph + 4 * k, 0u32);
        }
        for off in [0xc, 0x10, 0x14] {
            mem.write(pen + off, 0u32);
        }
        mem.write(packet, cursor);
        let start = store(f64::from(left));
        mem.write(pen, u32::from_le_bytes(rgba.map(|c| c.to_le_bytes()[0])));
        mem.write(pen_x, start);
        mem.write(pen_y, store(f64::from(top)));
        mem.write(pen + 0x18, ((flags & 0xff) << 24) | ((edge & 0xff) << 16));
        let lang = at(mem, 0x008a_c2f0);
        let group = at(mem, 0x008d_f610);
        let skip = at(mem, 0x008d_fd80);
        let mut p = text;
        loop {
            let code = mem.read::<u8>(p);
            if code == 0 {
                break;
            }
            match call(mem, group, &[u32::from(code)]) {
                1 => {
                    mem.write(pen_x, store(load(start)));
                    let down = add(load(mem.read(pen_y)), 16.0);
                    mem.write(pen_y, store(down));
                }
                2 if code == 7 => pen_step_x(mem, p, pen_x),
                2 => {
                    if code == 0xa {
                        pen_set_flags(mem, p, pen_flags);
                    }
                    if code == 0xb {
                        let icon = mem.read::<u8>(p + 1);
                        let id = u32::from(icon);
                        call(mem, at(mem, 0x008b_1350), &[id, u32::MAX]);
                        let colour = [rgba[0], rgba[1], rgba[2], rgba[3]];
                        let y = if call(mem, lang, &[]) == 0 {
                            let adjust = at(mem, 0x008e_5280);
                            let raise = fcall(
                                mem,
                                adjust,
                                &[ICON_SCALE, colour[0], colour[1], colour[2], colour[3]],
                            );
                            store(add(raise, add(load(mem.read(pen_y)), 4.0)))
                        } else {
                            let mut y = store(load(mem.read(pen_y)));
                            let low = (0x34..=0x37).contains(&icon)
                                || (0x20..=0x22).contains(&icon)
                                || (350.0 > load(y) && (icon == 0x48 || icon == 0x30))
                                || icon == 0x42;
                            if low {
                                y = store(sub(load(y), 2.5));
                            }
                            store(add(load(y), 5.0))
                        };
                        let x = store(load(mem.read(pen_x)));
                        let draw = at(mem, 0x008f_8ee0);
                        call(
                            mem,
                            draw,
                            &[
                                id, x, y, ICON_SCALE, colour[0], colour[1], colour[2], colour[3],
                            ],
                        );
                        let width = fcall(mem, at(mem, 0x008f_a800), &[id, ICON_SCALE]);
                        let moved = add(add(width, 1.0), load(mem.read(pen_x)));
                        mem.write(pen_x, store(moved));
                    }
                    call(mem, at(mem, 0x008b_1330), &[]);
                }
                3 => {
                    if key != u32::MAX {
                        call(mem, at(mem, 0x008e_9b80), &[key, p, pen]);
                    }
                }
                4 => {
                    call(mem, at(mem, 0x008b_7240), &[p, glyph]);
                    let face = at(mem, 0x0090_6270);
                    if mem.read::<u8>(pen + 0x1a) == 1 {
                        call(mem, at(mem, 0x0090_6260), &[pen, glyph]);
                        call(mem, face, &[pen, glyph]);
                    } else {
                        if top == STAMP {
                            let rise = if call(mem, lang, &[]) == 0 { 1.0 } else { 3.0 };
                            mem.write(pen_y, store(add(load(mem.read(pen_y)), rise)));
                        }
                        let (draw_type, shadow) = (at(mem, 0x0064_3af0), at(mem, 0x0064_0dd0));
                        call(mem, draw_type, &[1]);
                        call(mem, shadow, &[0]);
                        call(mem, at(mem, 0x0090_6290), &[pen, glyph]);
                        call(mem, shadow, &[1]);
                        call(mem, face, &[pen, glyph]);
                        call(mem, draw_type, &[0]);
                        if top == STAMP {
                            let rise = if call(mem, lang, &[]) == 0 { 1.0 } else { 3.0 };
                            mem.write(pen_y, store(sub(load(mem.read(pen_y)), rise)));
                        }
                    }
                    call(mem, at(mem, 0x008e_8ed0), &[pen, glyph, SCALE]);
                }
                _ => {}
            }
            p = call(mem, skip, &[p]);
        }
        mem.read(packet)
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
