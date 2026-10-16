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
            let k = match kind {
                2 | 3 => 1,
                5 => 2,
                6 => 3,
                _ => 0,
            };
            VIBRATE_CURVES[k].ptr(mem).addr
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
        let record = ffx_kernel::bridge::rom(mem, table, id, 0);
        let strings = segment_strings(mem, table.get(mem), id & 0xfff);
        let at = record
            .wrapping_add(field)
            .wrapping_add(u32::from(hiragana) * 4);
        strings.wrapping_add(u32::from(mem.read::<u16>(at)))
    }

    /// Where the strings of `id`'s segment start, as `MsGetExcelData`
    /// stores them for its caller (the first segment's when none holds it).
    fn segment_strings(mem: &impl Mem, table: u32, id: u32) -> u32 {
        let count = i32::from(mem.read::<i16>(table));
        let mut seg = table.wrapping_add(8);
        let mut found = false;
        for _ in 0..count.max(0) {
            let first = u32::from(mem.read::<u16>(seg));
            let last = u32::from(mem.read::<u16>(seg.wrapping_add(2)));
            if first <= id && id <= last {
                found = true;
                break;
            }
            seg = seg.wrapping_add(12);
        }
        if !found {
            seg = table.wrapping_add(8);
        }
        let data = mem.read::<u32>(seg.wrapping_add(8));
        u32::from(mem.read::<u16>(seg.wrapping_add(6)))
            .wrapping_add(data)
            .wrapping_add(table)
    }

    /// `TkMsGetRomItem(id, &name)` (`0x008ab280`): an item's name.
    pub fn tk_ms_get_rom_item(mem: &mut impl Mem, id: u32, out: u32) {
        let name = kernel_text(mem, ffx_kernel::bridge::ITEM, id, 0);
        mem.write(out, name);
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
        let widths = FONT_WIDTHS.ptr(mem).addr;
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
            let w = if proportional == 0 {
                if c >= 0x100 { 13 } else { 9 }
            } else {
                i32::from(mem.read::<u8>(widths.wrapping_add(c))) + 2
            };
            total = total.wrapping_add(w << 4);
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
