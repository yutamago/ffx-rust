//! The small Atel accessors of the VM's global state: one or two loads or
//! stores each, on the globals named in `symbols/ffx.sym`.

use ffx_bridge::{Global, Mem};

use super::{CONTEXT, CTRL_WORKS};
use crate::vm::{block, ctrl, through_x87};

/// `g_AtelMusicNumber`.
pub const MUSIC_NUMBER: Global<u32> = Global::new(0x0132_7130);
/// `g_AtelEffectFlags`: 32 flags.
pub const EFFECT_FLAGS: Global<u32> = Global::new(0x0132_6b70);
/// `g_AtelEffectParams`: four words.
pub const EFFECT_PARAMS: Global<u32> = Global::new(0x0132_6ce0);
/// `g_AtelSnd3dVolumeTotalRatio` (`f32`).
pub const SND3D_TOTAL_RATIO: Global<u32> = Global::new(0x0132_830c);
/// `g_AtelSnd3dVolumeDiffRatio` (`f32`).
pub const SND3D_DIFF_RATIO: Global<u32> = Global::new(0x0132_8310);
/// `g_AtelSnd3dWork`: 16 entries of `0x24` bytes.
pub const SND3D_WORK: Global<u8> = Global::new(0x0132_8318);
/// `g_AtelAlbhedDic`: two words.
pub const ALBHED_DIC: Global<u32> = Global::new(0x0132_5b90);
/// `g_AtelForceBattleMapId`.
pub const FORCE_BATTLE_MAP_ID: Global<u32> = Global::new(0x00c5_273c);
/// `g_AtelCallbackQue`: entries of `0x100` bytes, the first word non-zero
/// when queued.
pub const CALLBACK_QUE: Global<u32> = Global::new(0x0132_7f50);
/// `g_AtelBattleEndStatus`: `+0` status, `+4` keep-recover type, `+8`,
/// `+0xc`.
pub const BATTLE_END_STATUS: Global<u32> = Global::new(0x0132_6b48);
/// `g_AtelMotionRequest`: three words.
pub const MOTION_REQUEST: Global<u32> = Global::new(0x0132_6b5c);
/// `g_AtelShopRatio`.
pub const SHOP_RATIO: Global<u32> = Global::new(0x0132_6b6c);
/// `g_AtelMenuMask`.
pub const MENU_MASK: Global<u32> = Global::new(0x0132_6b74);
/// `g_AtelOverlayStatus` (`u8`).
pub const OVERLAY_STATUS: Global<u8> = Global::new(0x0132_6b80);
/// `g_AtelNotAuronPatchFlag` (`u8`).
pub const NOT_AURON_PATCH_FLAG: Global<u8> = Global::new(0x0132_6b81);
/// `g_AtelBattleEndMovieFadeout`: three words.
pub const BATTLE_END_MOVIE_FADEOUT: Global<u32> = Global::new(0x0132_6b84);
/// `g_AtelLoveParamDisable`.
pub const LOVE_PARAM_DISABLE: Global<u32> = Global::new(0x0132_6b90);
/// `g_AtelMCFPInWater`: four words.
pub const MCFP_IN_WATER: Global<u32> = Global::new(0x0132_6cf0);
/// `g_AtelMenuOtakaraFlag` (`i8`).
pub const MENU_OTAKARA_FLAG: Global<i8> = Global::new(0x0132_70f0);
/// `g_AtelMapIdPage`: `+0` state (2 set, 0 cleared), `+1`, `+2` the page,
/// `+3` the event mode (`0xff` reads as 1).
pub const MAP_ID_PAGE: Global<u8> = Global::new(0x0132_711c);
/// `g_AtelExpandFTCAdrs`.
pub const EXPAND_FTC_ADRS: Global<u32> = Global::new(0x0132_7120);
/// `g_AtelPauseFlag`: two words.
pub const PAUSE_FLAG: Global<u32> = Global::new(0x0132_7138);
/// `g_MegasAdding`.
pub const MEGAS_ADDING: Global<u32> = Global::new(0x0132_5ac8);

fn word(g: Global<u32>, mem: &impl Mem, i: u32) -> u32 {
    g.ptr(mem).at(i).addr
}

fn set_words(mem: &mut impl Mem, g: Global<u32>, values: &[u32]) {
    for (i, &v) in (0..).zip(values) {
        let at = word(g, mem, i);
        mem.write(at, v);
    }
}

/// `AtelGetMusicNumber()` (`0x00873b00`).
pub fn atel_get_music_number(mem: &impl Mem) -> u32 {
    MUSIC_NUMBER.get(mem)
}

/// `AtelRegistMusicNumber(n)` (`0x00873d00`).
pub fn atel_regist_music_number(mem: &mut impl Mem, n: u32) {
    MUSIC_NUMBER.set(mem, n);
}

/// `AtelGetEffectFlag(bit)` (`0x0086b030`): 1 when set (the bit number
/// taken mod 32, as `shl` does).
pub fn atel_get_effect_flag(mem: &impl Mem, bit: u32) -> u32 {
    u32::from(EFFECT_FLAGS.get(mem) & (1 << (bit & 31)) != 0)
}

/// `AtelSetEffectFlag(bit)` (`0x0086fa40`).
pub fn atel_set_effect_flag(mem: &mut impl Mem, bit: u32) {
    let f = EFFECT_FLAGS.get(mem) | 1 << (bit & 31);
    EFFECT_FLAGS.set(mem, f);
}

/// `AtelAllClearEffectFlag()` (`0x00860fd0`).
pub fn atel_all_clear_effect_flag(mem: &mut impl Mem) {
    EFFECT_FLAGS.set(mem, 0);
}

/// `AtelGetEffectParam(i)` (`0x0086b050`).
pub fn atel_get_effect_param(mem: &impl Mem, i: u32) -> u32 {
    mem.read(word(EFFECT_PARAMS, mem, i))
}

/// `AtelClearEffectParam()` (`0x00863880`).
pub fn atel_clear_effect_param(mem: &mut impl Mem) {
    set_words(mem, EFFECT_PARAMS, &[0; 4]);
}

/// `AtelSetSnd3dVolumeDiffRatio(ratio)` (`0x00877450`), the `f32` as bits
/// (through the x87).
pub fn atel_set_snd3d_volume_diff_ratio(mem: &mut impl Mem, bits: u32) {
    SND3D_DIFF_RATIO.set(mem, through_x87(bits));
}

/// `AtelSetSnd3dVolumeTotalRatio(ratio)` (`0x00877460`).
pub fn atel_set_snd3d_volume_total_ratio(mem: &mut impl Mem, bits: u32) {
    SND3D_TOTAL_RATIO.set(mem, through_x87(bits));
}

/// `AtelClearSnd3dWorkAll()` (`0x00877330`): clears the 16 entries'
/// `+0`, `+4` and `+0x20` (`u16`), then both ratios.
pub fn atel_clear_snd3d_work_all(mem: &mut impl Mem) {
    let base = SND3D_WORK.ptr(mem).addr;
    for i in 0..16 {
        let e = base + i * 0x24;
        mem.write(e, 0u32);
        mem.write(e + 4, 0u32);
        mem.write(e + 0x20, 0u16);
    }
    SND3D_TOTAL_RATIO.set(mem, 0);
    SND3D_DIFF_RATIO.set(mem, 0);
}

/// `AtelStrcat(dst, src)` (`0x00871950`): appends byte by byte (overlaps
/// behave as the original's loop). Returns its `EAX`: just past the
/// copied terminator.
pub fn atel_strcat(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
    let mut d = dst;
    while mem.read::<u8>(d) != 0 {
        d = d.wrapping_add(1);
    }
    let mut s = src;
    loop {
        let c = mem.read::<u8>(s);
        mem.write(d, c);
        s = s.wrapping_add(1);
        d = d.wrapping_add(1);
        if c == 0 {
            return d;
        }
    }
}

/// `AtelStrcpy(dst, src)` (`0x00871980`): `dst[0] = 0`, then
/// [`atel_strcat`].
pub fn atel_strcpy(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
    mem.write(dst, 0u8);
    atel_strcat(mem, dst, src)
}

/// `AtelRegistAlbhedDic(a, b)` (`0x0086e820`).
pub fn atel_regist_albhed_dic(mem: &mut impl Mem, a: u32, b: u32) {
    set_words(mem, ALBHED_DIC, &[a, b]);
}

/// `AtelSetForceBattleMapId(id)` (`0x00870130`).
pub fn atel_set_force_battle_map_id(mem: &mut impl Mem, id: u32) {
    FORCE_BATTLE_MAP_ID.set(mem, id);
}

/// `AtelSetVibrateStreamExt(v)` (`0x008715e0`): the current ctrl work's.
pub fn atel_set_vibrate_stream_ext(mem: &mut impl Mem, v: u32) {
    let c = CONTEXT.get(mem);
    mem.write(c + ctrl::VIBRATE_STREAM_EXT, v);
}

/// `AtelCheckCallbackQue(i)` (`0x00875000`): whether entry `i` is queued.
pub fn atel_check_callback_que(mem: &impl Mem, i: u32) -> u32 {
    let at = CALLBACK_QUE.ptr(mem).addr.wrapping_add(i << 8);
    u32::from(mem.read::<u32>(at) != 0)
}

/// `AtelIsCallbackQue()` (`0x00875180`): whether entry 0 is queued.
pub fn atel_is_callback_que(mem: &impl Mem) -> u32 {
    u32::from(CALLBACK_QUE.get(mem) != 0)
}

/// `AtelSetEncountBattleEndStatus(a, b, c)` (`0x0086fc20`): `+0`, `+8`,
/// `+0xc`.
pub fn atel_set_encount_battle_end_status(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
    BATTLE_END_STATUS.set(mem, a);
    let at = word(BATTLE_END_STATUS, mem, 2);
    mem.write(at, b);
    mem.write(at + 4, c);
}

/// `AtelInitBattleEndStatus()` (`0x0086cff0`).
pub fn atel_init_battle_end_status(mem: &mut impl Mem) {
    BATTLE_END_STATUS.set(mem, 0x0001_0001);
}

/// `AtelSetBattleEndKeepRecoverType(v)` (`0x0086f3a0`): `+4`, its low
/// byte cleared.
pub fn atel_set_battle_end_keep_recover_type(mem: &mut impl Mem, v: u32) {
    let at = word(BATTLE_END_STATUS, mem, 1);
    mem.write(at, v & 0xffff_ff00);
}

/// `AtelGetStatusWinFadeOut()` (`0x0086c4d0`): 1 when bit 16 of the
/// battle end status is clear.
pub fn atel_get_status_win_fade_out(mem: &impl Mem) -> u32 {
    !(BATTLE_END_STATUS.get(mem) >> 16) & 1
}

/// `AtelSetMCFPInWater(a, b, c, d)` (`0x00870280`).
pub fn atel_set_mcfp_in_water(mem: &mut impl Mem, a: u32, b: u32, c: u32, d: u32) {
    set_words(mem, MCFP_IN_WATER, &[a, b, c, d]);
}

/// `AtelSetMenuExecStatus(bank)` (`0x008702e0`): the message window bank.
pub fn atel_set_menu_exec_status(mem: &mut impl Mem, bank: u8) {
    super::MENU_EXEC_STATUS.set(mem, bank.cast_signed());
}

/// `AtelSetShopRatio(r)` (`0x00870f60`).
pub fn atel_set_shop_ratio(mem: &mut impl Mem, r: u32) {
    SHOP_RATIO.set(mem, r);
}

/// `AtelEventGetMotionIndex(a, b)` (`0x00865890`): `a * 18 + b + 3`.
#[must_use]
pub const fn atel_event_get_motion_index(a: u32, b: u32) -> u32 {
    a.wrapping_mul(18).wrapping_add(b).wrapping_add(3)
}

/// `AtelSetMapIdPage(page)` (`0x00873e60`).
pub fn atel_set_map_id_page(mem: &mut impl Mem, page: u8) {
    let p = MAP_ID_PAGE.ptr(mem);
    p.set(mem, 2);
    p.at(2).set(mem, page);
}

/// `AtelClearPreBgPage()` (`0x00872b90`).
pub fn atel_clear_pre_bg_page(mem: &mut impl Mem) {
    let p = MAP_ID_PAGE.ptr(mem);
    p.set(mem, 0);
    p.at(1).set(mem, 0xff);
    p.at(2).set(mem, 0xff);
}

/// `AtelGetEventMode()` (`0x00873aa0`): `0xff` reads as 1 (a tail jump to
/// a `return 1`).
pub fn atel_get_event_mode(mem: &impl Mem) -> u32 {
    match MAP_ID_PAGE.ptr(mem).at(3).get(mem) {
        0xff => 1,
        m => u32::from(m),
    }
}

/// `AtelClearPauseFlag()` (`0x00872b70`).
pub fn atel_clear_pause_flag(mem: &mut impl Mem) {
    set_words(mem, PAUSE_FLAG, &[0, 0]);
}

/// `AtelRegistBattleEndMovieFadeout(a, b, c)` (`0x0086e840`).
pub fn atel_regist_battle_end_movie_fadeout(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
    set_words(mem, BATTLE_END_MOVIE_FADEOUT, &[a, b, c]);
}

/// `AtelInitMotionRequest()` (`0x0086d4c0`): `+8` = 0, `+0` and `+4` = -1
/// (in that order).
pub fn atel_init_motion_request(mem: &mut impl Mem) {
    let at = word(MOTION_REQUEST, mem, 2);
    mem.write(at, 0u32);
    set_words(mem, MOTION_REQUEST, &[u32::MAX, u32::MAX]);
}

/// `getMegasAdding()` (`0x00860a90`).
pub fn get_megas_adding(mem: &impl Mem) -> u32 {
    MEGAS_ADDING.get(mem)
}

/// `AtelCheckMenuOtakaraFlag()` (`0x00872b50`), sign-extended.
pub fn atel_check_menu_otakara_flag(mem: &impl Mem) -> u32 {
    i32::from(MENU_OTAKARA_FLAG.get(mem)).cast_unsigned()
}

/// `AtelGetMenuMask()` (`0x0086be50`).
pub fn atel_get_menu_mask(mem: &impl Mem) -> u32 {
    MENU_MASK.get(mem)
}

/// `AtelClearAllMenuMask()` (`0x00863620`).
pub fn atel_clear_all_menu_mask(mem: &mut impl Mem) {
    MENU_MASK.set(mem, 0);
}

/// `AtelGetExpandFTCAdrs()` (`0x00873ac0`).
pub fn atel_get_expand_ftc_adrs(mem: &impl Mem) -> u32 {
    EXPAND_FTC_ADRS.get(mem)
}

/// `AtelClearNotAuronPatchFlag()` (`0x00863f90`).
pub fn atel_clear_not_auron_patch_flag(mem: &mut impl Mem) {
    NOT_AURON_PATCH_FLAG.set(mem, 0);
}

/// `AtelClearLoveParamDisable()` (`0x00863ae0`).
pub fn atel_clear_love_param_disable(mem: &mut impl Mem) {
    LOVE_PARAM_DISABLE.set(mem, 0);
}

/// `AtelSetOverlayStatus(s)` (`0x00870900`).
pub fn atel_set_overlay_status(mem: &mut impl Mem, s: u8) {
    OVERLAY_STATUS.set(mem, s);
}

/// `AtelEventGetNowCtrlTopId()` (`0x008658b0`): ctrl work 0's player
/// worker, -1 for none.
pub fn atel_event_get_now_ctrl_top_id(mem: &impl Mem) -> u32 {
    match mem.read::<u16>(CTRL_WORKS.ptr(mem).addr + ctrl::TOP_ID) {
        0xffff => u32::MAX,
        id => u32::from(id),
    }
}

/// `AtelGetCurrentActorTotal()` (`0x0086b000`): the current ctrl work's
/// actors, 0 when it is not in use.
pub fn atel_get_current_actor_total(mem: &impl Mem) -> u32 {
    let c = CONTEXT.get(mem);
    if mem.read::<u8>(c + ctrl::FLAGS) & 1 == 0 {
        return 0;
    }
    u32::from(mem.read::<u16>(c + ctrl::ACTORS))
}

/// `AtelGetCtrlWorkStatus(i)` (`0x0086afc0`): whether ctrl work `i` is in
/// use.
pub fn atel_get_ctrl_work_status(mem: &impl Mem, i: u32) -> u32 {
    let c = super::atel_get_ctrl_work(mem, i);
    u32::from(mem.read::<u8>(c + ctrl::FLAGS) & 1)
}

/// `AtelGetActorExecspaceID(worker)` (`0x0086a420`): the low nibble of
/// worker `+0xae`.
pub fn atel_get_actor_execspace_id(mem: &impl Mem, w: u32) -> u32 {
    u32::from(mem.read::<u8>(w + 0xae) & 0xf)
}

/// `AtelGetGideMapActorNumber()` (`0x0086b710`): the minimap marker count
/// in the event block's expansion header (`+0x18` the offset of a table
/// whose first word it is), 0 when ctrl work 0 is free or anything is
/// missing.
pub fn atel_get_gide_map_actor_number(mem: &impl Mem) -> u32 {
    let c = CTRL_WORKS.ptr(mem).addr;
    if mem.read::<u8>(c + ctrl::FLAGS) & 1 == 0 {
        return 0;
    }
    let b = mem.read::<u32>(c + ctrl::BLOCK);
    if b == 0 {
        return 0;
    }
    let off = mem.read::<u32>(b + block::EXPAND);
    let e = off.wrapping_add(b);
    if off == 0 || e == 0 {
        return 0;
    }
    let t = mem.read::<u32>(e + 0x18);
    if t == 0 {
        return 0;
    }
    mem.read(t.wrapping_add(b))
}
