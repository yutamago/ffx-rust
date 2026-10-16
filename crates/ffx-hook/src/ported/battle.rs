//! `ffx-battle`: random numbers, the save block, actors, the battle camera,
//! battle globals and helpers, damage.

use ffx_battle::{actor::bridge as actor, camera::bridge as camera, command::bridge as cmd};
use ffx_battle::{body::bridge as body, menu::bridge as menu, vector::bridge as vector};
use ffx_battle::{
    cue::bridge as cue, flow::bridge as flow, pos::bridge as pos, turn::bridge as turn,
};
use ffx_battle::{damage::bridge as dmg, state::bridge as state, util::bridge as util};
use ffx_battle::{minor::bridge as minor, misc::bridge as misc, save::bridge as save};

use super::{call0, mem, ported};

/// `TOGetFFXLang` (system), which `DmgCalc_Bribe` asks.
const TO_GET_FFX_LANG: u32 = 0x0064_10e0;
/// `TOBtlCloseSignWindow` (system), which the message cue calls.
const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;
/// `achievementUnlockAchievement(n)` (main), which `Battle_Steal` calls.
const ACHIEVEMENT_UNLOCK: u32 = 0x0082_2380;

/// Calls an original cdecl function of one argument by its `ffx.sym`
/// address.
fn call1(sym: u32, a: u32) -> u32 {
    // SAFETY: the address is a cdecl function of this build taking one u32.
    unsafe {
        let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(crate::rebase(sym));
        f(a)
    }
}

/// Calls an original cdecl function of four arguments by its `ffx.sym`
/// address.
#[allow(clippy::many_single_char_names)]
fn call4(sym: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
    // SAFETY: the address is a cdecl function of this build taking four u32s.
    unsafe {
        let f: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 =
            std::mem::transmute(crate::rebase(sym));
        f(a, b, c, d)
    }
}

/// `MsGetSummonerPos` (still the original: its `_CIcos`/`_CIsin` are next).
const MS_GET_SUMMONER_POS: u32 = 0x007a_c2b0;

ported! {
    /// `rnd`: the global random generator.
    rnd @ 0x0079_89a0 => extern "C" fn rnd() -> u32 {
        crate::harness::log_call("rnd", 0);
        ffx_battle::rng::bridge::rnd(&mut mem())
    }
    /// `brnd(index)`: one of the 68 indexed generators.
    brnd @ 0x0079_88f0 => extern "C" fn brnd(index: u32) -> u32 {
        crate::harness::log_call("brnd", index);
        ffx_battle::rng::bridge::brnd(&mut mem(), index)
    }
    /// `MsRandMize(n)`: seeds every generator from `Rng_ClockSeed`.
    MsRandMize @ 0x0079_8890 => extern "C" fn ms_rand_mize(n: u32) {
        let clock = call0(0x0079_8950);
        ffx_battle::rng::bridge::ms_rand_mize(&mut mem(), clock, n);
    }

    // The save block.
    /// `MsGetSaveCommand(chr, cmd)`: command learnt.
    MsGetSaveCommand @ 0x0078_5020 => extern "C" fn ms_get_save_command(chr: i32, cmd: u32) -> u32 {
        save::ms_get_save_command(&mut mem(), chr, cmd)
    }
    /// `MsGetSaveCommandUse(chr, cmd)`: command used.
    MsGetSaveCommandUse @ 0x0078_5140 => extern "C" fn ms_get_save_command_use(chr: i32, cmd: u32) -> u32 {
        save::ms_get_save_command_use(&mut mem(), chr, cmd)
    }
    /// `MsGetSaveEventAddress`: `&g_SaveData`.
    MsGetSaveEventAddress @ 0x0078_5240 => extern "C" fn ms_get_save_event_address() -> u32 {
        save::ms_get_save_event_address(&mem())
    }
    /// `MsGetSaveMenu`: the menu settings word.
    MsGetSaveMenu @ 0x0078_5290 => extern "C" fn ms_get_save_menu() -> u32 {
        save::ms_get_save_menu(&mem())
    }
    /// `MsGetSaveConfigCursorSave`.
    MsGetSaveConfigCursorSave @ 0x0078_51c0 => extern "C" fn ms_get_save_config_cursor_save() -> u32 {
        save::ms_get_save_config_cursor_save(&mem())
    }
    /// `MsGetSaveConfigHiragana`.
    MsGetSaveConfigHiragana @ 0x0078_51f0 => extern "C" fn ms_get_save_config_hiragana() -> u32 {
        save::ms_get_save_config_hiragana(&mem())
    }
    /// `MsGetSavePlayerPtr(chr)`: a `ply_save` record.
    MsGetSavePlayerPtr @ 0x0078_5330 => extern "C" fn ms_get_save_player_ptr(chr: u32) -> u32 {
        save::ms_get_save_player_ptr(&mem(), chr)
    }
    /// `MsGetSavePlyJoin(chr)`.
    MsGetSavePlyJoin @ 0x0078_5380 => extern "C" fn ms_get_save_ply_join(chr: u32) -> u32 {
        save::ms_get_save_ply_join(&mem(), chr)
    }
    /// `MsGetSavePlyJoined(chr)`.
    MsGetSavePlyJoined @ 0x0078_53a0 => extern "C" fn ms_get_save_ply_joined(chr: u32) -> u32 {
        save::ms_get_save_ply_joined(&mem(), chr)
    }
    /// `MsGetSaveSP(chr)`.
    MsGetSaveSP @ 0x0078_53e0 => extern "C" fn ms_get_save_sp(chr: u32) -> u32 {
        save::ms_get_save_sp(&mem(), chr)
    }
    /// `getScenerioFlag`: the story progress (16 bits in AX).
    getScenerioFlag @ 0x0078_7360 => extern "C" fn get_scenerio_flag() -> u16 {
        save::get_scenerio_flag(&mem())
    }
    /// `getInterValefor` (8 bits in AL).
    getInterValefor @ 0x0078_7340 => extern "C" fn get_inter_valefor() -> u8 {
        save::get_inter_valefor(&mem())
    }
    /// `getDefaultChrName(chr)`.
    getDefaultChrName @ 0x0078_7320 => extern "C" fn get_default_chr_name(chr: u32) -> u32 {
        save::get_default_chr_name(&mem(), chr)
    }
    /// `MsGetSaveItemNum(item)`.
    MsGetSaveItemNum @ 0x0079_04b0 => extern "C" fn ms_get_save_item_num(item: u32) -> u32 {
        save::ms_get_save_item_num(&mut mem(), item)
    }
    /// `MsSetUseItem(item)`.
    MsSetUseItem @ 0x0079_0790 => extern "C" fn ms_set_use_item(item: u32) {
        save::ms_set_use_item(&mut mem(), item);
    }
    /// `MsSaveItemTypeBuffer(&count)`.
    MsSaveItemTypeBuffer @ 0x0079_0530 => extern "C" fn ms_save_item_type_buffer(count: u32) -> u32 {
        save::ms_save_item_type_buffer(&mut mem(), count)
    }
    /// `MsSaveItemNumBuffer(&count)`.
    MsSaveItemNumBuffer @ 0x0079_0510 => extern "C" fn ms_save_item_num_buffer(count: u32) -> u32 {
        save::ms_save_item_num_buffer(&mut mem(), count)
    }
    /// `MsGetSaveInParty(&count)`.
    MsGetSaveInParty @ 0x0078_5270 => extern "C" fn ms_get_save_in_party(count: u32) -> u32 {
        save::ms_get_save_in_party(&mut mem(), count)
    }
    /// `MsGetSaveOutParty(&count)`.
    MsGetSaveOutParty @ 0x0078_52d0 => extern "C" fn ms_get_save_out_party(count: u32) -> u32 {
        save::ms_get_save_out_party(&mut mem(), count)
    }
    /// `MsGetSavePartyMember(&a, &b, &c)`.
    MsGetSavePartyMember @ 0x0078_52f0 => extern "C" fn ms_get_save_party_member(a: u32, b: u32, c: u32) {
        save::ms_get_save_party_member(&mut mem(), a, b, c);
    }
    /// `MsSetSaveParty`.
    MsSetSaveParty @ 0x0078_6930 => extern "C" fn ms_set_save_party() {
        save::ms_set_save_party(&mut mem());
    }
    /// `MsGetGIL`.
    MsGetGIL @ 0x0078_4e80 => extern "C" fn ms_get_gil() -> u32 {
        save::ms_get_gil(&mem())
    }
    /// `MsDebugFullGill`.
    MsDebugFullGill @ 0x0078_49c0 => extern "C" fn ms_debug_full_gill() {
        save::ms_debug_full_gill(&mut mem());
    }
    /// `MsDebugAddGill(n)`.
    MsDebugAddGill @ 0x0078_4830 => extern "C" fn ms_debug_add_gill(n: i32) {
        save::ms_debug_add_gill(&mut mem(), n);
    }
    /// `MsGetSaveBattleCount`.
    MsGetSaveBattleCount @ 0x0078_4f50 => extern "C" fn ms_get_save_battle_count() -> u32 {
        save::ms_get_save_battle_count(&mem())
    }
    /// `MsGetMonsterCapture(id)`.
    MsGetMonsterCapture @ 0x0079_0af0 => extern "C" fn ms_get_monster_capture(id: u32) -> u32 {
        save::ms_get_monster_capture(&mem(), id)
    }
    /// `AddMonsterCapture(id, n)`.
    AddMonsterCapture @ 0x0079_0b90 => extern "C" fn add_monster_capture(id: u32, n: i32) -> i32 {
        save::add_monster_capture(&mut mem(), id, n)
    }
    /// `MsImportantGet(id)`: key item held.
    MsImportantGet @ 0x0079_0800 => extern "C" fn ms_important_get(id: u32) -> u32 {
        save::ms_important_get(&mem(), id)
    }
    /// `MsImportantSet(id)`.
    MsImportantSet @ 0x0079_0930 => extern "C" fn ms_important_set(id: u32) {
        save::ms_important_set(&mut mem(), id);
    }
    /// `MsImportantPtr(id, strings)`: the key item's kernel record.
    MsImportantPtr @ 0x0079_08a0 => extern "C" fn ms_important_ptr(id: u32, strings: u32) -> u32 {
        save::ms_important_ptr(&mut mem(), id, strings)
    }
    /// `MsSetItemCheck(id, table)`.
    MsSetItemCheck @ 0x0079_0670 => extern "C" fn ms_set_item_check(id: u32, table: u32) {
        save::ms_set_item_check(&mut mem(), id, table);
    }
    /// `MsSetMonsterCheck(id, table)`.
    MsSetMonsterCheck @ 0x0079_0be0 => extern "C" fn ms_set_monster_check(id: u32, table: u32) {
        save::ms_set_monster_check(&mut mem(), id, table);
    }
    /// `MsGetSaveAbilityMap`.
    MsGetSaveAbilityMap @ 0x0078_4f40 => extern "C" fn ms_get_save_ability_map() -> u32 {
        save::ms_get_save_ability_map(&mem())
    }
    /// `MsGetSaveWalkMap`.
    MsGetSaveWalkMap @ 0x0078_5400 => extern "C" fn ms_get_save_walk_map() -> u32 {
        save::ms_get_save_walk_map(&mem())
    }
    /// `MsGetSaveEventFlag`.
    MsGetSaveEventFlag @ 0x0078_5250 => extern "C" fn ms_get_save_event_flag() -> u32 {
        save::ms_get_save_event_flag(&mem())
    }
    /// `MsGetItemCheck`.
    MsGetItemCheck @ 0x0079_04a0 => extern "C" fn ms_get_item_check() -> u32 {
        save::ms_get_item_check(&mem())
    }
    /// `MsGetSaveItemUse`.
    MsGetSaveItemUse @ 0x0079_0500 => extern "C" fn ms_get_save_item_use() -> u32 {
        save::ms_get_save_item_use(&mem())
    }
    /// `MsGetSaveMonsterMeet`.
    MsGetSaveMonsterMeet @ 0x0079_0b20 => extern "C" fn ms_get_save_monster_meet() -> u32 {
        save::ms_get_save_monster_meet(&mem())
    }
    /// `MsGetSaveMonsterDefeat`.
    MsGetSaveMonsterDefeat @ 0x0079_0b10 => extern "C" fn ms_get_save_monster_defeat() -> u32 {
        save::ms_get_save_monster_defeat(&mem())
    }
    /// `MsGetSaveWeaponList(character, kind, unequipped, &count)`.
    MsGetSaveWeaponList @ 0x007a_bd10 => extern "C" fn ms_get_save_weapon_list(character: i32, kind: i32, unequipped: i32, count: u32) -> u32 {
        save::ms_get_save_weapon_list(&mut mem(), character, kind, unequipped, count)
    }
    /// `MsGetSaveWeaponAmount(&free)`.
    MsGetSaveWeaponAmount @ 0x007a_bc60 => extern "C" fn ms_get_save_weapon_amount(free: u32) -> u32 {
        save::ms_get_save_weapon_amount(&mut mem(), free)
    }
    /// `MsCalcWeaponCap(equipment)`.
    MsCalcWeaponCap @ 0x0079_8eb0 => extern "C" fn ms_calc_weapon_cap(equipment: u32) {
        save::ms_calc_weapon_cap(&mut mem(), equipment);
    }
    /// `MsGetSaveChrMan(chr)`.
    MsGetSaveChrMan @ 0x0078_4f90 => extern "C" fn ms_get_save_chr_man(chr: u32) -> u32 {
        save::ms_get_save_chr_man(&mut mem(), chr)
    }
    /// `MsGetSaveChrFemale(chr)`.
    MsGetSaveChrFemale @ 0x0078_4f60 => extern "C" fn ms_get_save_chr_female(chr: u32) -> u32 {
        save::ms_get_save_chr_female(&mut mem(), chr)
    }
    /// `MsGetSaveChrSummon(chr)`.
    MsGetSaveChrSummon @ 0x0078_4ff0 => extern "C" fn ms_get_save_chr_summon(chr: u32) -> u32 {
        save::ms_get_save_chr_summon(&mut mem(), chr)
    }
    /// `MsAddLoveParam(chr, n)`.
    MsAddLoveParam @ 0x0078_45e0 => extern "C" fn ms_add_love_param(chr: i32, n: i32) -> u32 {
        save::ms_add_love_param(&mut mem(), chr, n)
    }
    /// `MsGetYoujinbouCom`: Yojimbo's command.
    MsGetYoujinbouCom @ 0x007b_2b20 => extern "C" fn ms_get_youjinbou_com() -> u32 {
        save::ms_get_youjinbou_com(&mem())
    }

    // Actors.
    /// `MsGetChr(i)`: the actor of index `i`.
    MsGetChr @ 0x0079_4020 => extern "C" fn ms_get_chr(i: u32) -> u32 {
        actor::ms_get_chr(&mem(), i)
    }
    /// `MsGetChrTop`.
    MsGetChrTop @ 0x0079_5970 => extern "C" fn ms_get_chr_top() -> u32 {
        actor::ms_get_chr_top(&mem())
    }
    /// `MsGetMon(i)`.
    MsGetMon @ 0x0079_5aa0 => extern "C" fn ms_get_mon(i: u32) -> u32 {
        actor::ms_get_mon(&mem(), i)
    }
    /// `MsGetWeapon(i, second)`.
    MsGetWeapon @ 0x0079_5b10 => extern "C" fn ms_get_weapon(i: u32, second: i32) -> u32 {
        actor::ms_get_weapon(&mem(), i, second)
    }
    /// `MsBtlChrNumCheck(i)`.
    MsBtlChrNumCheck @ 0x0079_3650 => extern "C" fn ms_btl_chr_num_check(i: u32) -> u32 {
        actor::ms_btl_chr_num_check(i)
    }
    /// `MsGetRamChrMonster(i)`.
    MsGetRamChrMonster @ 0x0079_aef0 => extern "C" fn ms_get_ram_chr_monster(i: u32) -> u32 {
        actor::ms_get_ram_chr_monster(i)
    }
    /// `MsCalcWeakLevel(hp, max)`.
    MsCalcWeakLevel @ 0x0078_bf00 => extern "C" fn ms_calc_weak_level(hp: i32, max: i32) -> u32 {
        actor::ms_calc_weak_level(hp, max)
    }
    /// `MsAutoCommandStatusCheck(actor, need_allowed, no_400, no_800)`.
    MsAutoCommandStatusCheck @ 0x007b_24a0 => extern "C" fn ms_auto_command_status_check(p: u32, need_allowed: i32, no_400: i32, no_800: i32) -> u32 {
        actor::ms_auto_command_status_check(&mem(), p, need_allowed, no_400, no_800)
    }
    /// `MsCheckSetMotion(actor, mode)`.
    MsCheckSetMotion @ 0x007a_f320 => extern "C" fn ms_check_set_motion(p: u32, mode: i32) -> u32 {
        actor::ms_check_set_motion(&mem(), p, mode)
    }
    /// `MsPushBtlPos(actor)`.
    MsPushBtlPos @ 0x007a_c650 => extern "C" fn ms_push_btl_pos(p: u32) {
        actor::ms_push_btl_pos(&mut mem(), p);
    }
    /// `MsPopBtlPos(actor)`.
    MsPopBtlPos @ 0x007a_c600 => extern "C" fn ms_pop_btl_pos(p: u32) {
        actor::ms_pop_btl_pos(&mut mem(), p);
    }
    /// `MsSetAutoStatus(actor)`.
    MsSetAutoStatus @ 0x0079_b290 => extern "C" fn ms_set_auto_status(p: u32) {
        actor::ms_set_auto_status(&mut mem(), p);
    }
    /// `MsResetAutoStatus(actor)`.
    MsResetAutoStatus @ 0x0079_b1a0 => extern "C" fn ms_reset_auto_status(p: u32) {
        actor::ms_reset_auto_status(&mut mem(), p);
    }
    /// `MsSetEternalRelife(actor)`.
    MsSetEternalRelife @ 0x0078_deb0 => extern "C" fn ms_set_eternal_relife(p: u32) {
        actor::ms_set_eternal_relife(&mut mem(), p);
    }
    /// `MsMotionSpeedReset(actor)`.
    MsMotionSpeedReset @ 0x007a_b730 => extern "C" fn ms_motion_speed_reset(p: u32) {
        actor::ms_motion_speed_reset(&mut mem(), p);
    }
    /// `MsTerminateMotion(actor)`.
    MsTerminateMotion @ 0x007a_b6c0 => extern "C" fn ms_terminate_motion(p: u32) {
        actor::ms_terminate_motion(&mut mem(), p);
    }
    /// `msSetEndDir(actor)`.
    msSetEndDir @ 0x0079_3260 => extern "C" fn ms_set_end_dir(p: u32) {
        actor::ms_set_end_dir(&mut mem(), p);
    }
    /// `msCheckSameTarget(chr, actor, targets)`.
    msCheckSameTarget @ 0x0079_6d70 => extern "C" fn ms_check_same_target(chr: u32, p: u32, targets: u32) -> i32 {
        actor::ms_check_same_target(&mem(), chr, p, targets)
    }
    /// `MsBtlChrInfoFree(actor)`.
    MsBtlChrInfoFree @ 0x0079_3570 => extern "C" fn ms_btl_chr_info_free(p: u32) {
        actor::ms_btl_chr_info_free(&mut mem(), p);
    }

    // Battle globals.
    /// `MsBattleGetStatus`.
    MsBattleGetStatus @ 0x0078_16f0 => extern "C" fn ms_battle_get_status() -> i32 {
        state::ms_battle_get_status(&mem())
    }
    /// `MsGetBattleScene`: field and group of the battle.
    MsGetBattleScene @ 0x0078_2970 => extern "C" fn ms_get_battle_scene() -> u32 {
        state::ms_get_battle_scene(&mem())
    }
    /// `MsBattleExeClear`.
    MsBattleExeClear @ 0x0078_1530 => extern "C" fn ms_battle_exe_clear() -> i32 {
        state::ms_battle_exe_clear(&mut mem())
    }
    /// `MsEventEffectGetStat`.
    MsEventEffectGetStat @ 0x0078_8bd0 => extern "C" fn ms_event_effect_get_stat() -> i32 {
        state::ms_event_effect_get_stat(&mem())
    }
    /// `MsBtlReadCueLevel`.
    MsBtlReadCueLevel @ 0x007a_d180 => extern "C" fn ms_btl_read_cue_level() -> u32 {
        state::ms_btl_read_cue_level(&mem())
    }
    /// `MsGetVoiceEventFlag`.
    MsGetVoiceEventFlag @ 0x0079_86a0 => extern "C" fn ms_get_voice_event_flag() -> u32 {
        state::ms_get_voice_event_flag(&mem())
    }
    /// `MsMemCheckInit`.
    MsMemCheckInit @ 0x0079_9ee0 => extern "C" fn ms_mem_check_init() {
        state::ms_mem_check_init(&mut mem());
    }
    /// `MsMemCheckRegist(name, size)`.
    MsMemCheckRegist @ 0x0079_9f00 => extern "C" fn ms_mem_check_regist(_name: u32, size: i32) {
        state::ms_mem_check_regist(&mut mem(), size);
    }
    /// `MsAtelRequestInit`.
    MsAtelRequestInit @ 0x0079_7ff0 => extern "C" fn ms_atel_request_init() {
        state::ms_atel_request_init(&mut mem());
    }
    /// `MsAtelRequestAdd(a, b, c, d)`: the low halves of the four.
    MsAtelRequestAdd @ 0x0079_7b70 => extern "C" fn ms_atel_request_add(a: u32, b: u32, c: u32, d: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original stores 16 bits
        state::ms_atel_request_add(&mut mem(), [a as u16, b as u16, c as u16, d as u16]);
    }
    /// `MsBattleWater`: under water.
    MsBattleWater @ 0x0078_21e0 => extern "C" fn ms_battle_water() -> i32 {
        state::ms_battle_water(&mem())
    }
    /// `MsBattleGetMapCircleOn`.
    MsBattleGetMapCircleOn @ 0x0078_16a0 => extern "C" fn ms_battle_get_map_circle_on() -> i32 {
        state::ms_battle_get_map_circle_on(&mem())
    }
    /// `MsGetEncountType`.
    MsGetEncountType @ 0x0078_2a10 => extern "C" fn ms_get_encount_type() -> u32 {
        state::ms_get_encount_type(&mem())
    }
    /// `MsStoneEffectCheck`.
    MsStoneEffectCheck @ 0x007a_2670 => extern "C" fn ms_stone_effect_check() -> i32 {
        state::ms_stone_effect_check(&mem())
    }
    /// `MsBattleCursorType`.
    MsBattleCursorType @ 0x0078_0ce0 => extern "C" fn ms_battle_cursor_type() -> i32 {
        state::ms_battle_cursor_type(&mem())
    }
    /// `MsBattleGetChrDisposeFlag`: always 1.
    MsBattleGetChrDisposeFlag @ 0x0078_1630 => extern "C" fn ms_battle_get_chr_dispose_flag() -> u32 {
        state::ms_battle_get_chr_dispose_flag()
    }
    /// `MsGetInputCue`.
    MsGetInputCue @ 0x007b_22e0 => extern "C" fn ms_get_input_cue() -> u32 {
        state::ms_get_input_cue(&mem())
    }
    /// `MsGetCommandCue`.
    MsGetCommandCue @ 0x007b_0a10 => extern "C" fn ms_get_command_cue() -> u32 {
        state::ms_get_command_cue(&mem())
    }
    /// `MsCheckCommandCue(actor, command)`.
    MsCheckCommandCue @ 0x007b_0720 => extern "C" fn ms_check_command_cue(actor: u32, command: u32) -> u32 {
        state::ms_check_command_cue(&mem(), actor, command)
    }
    /// `MsCheckCommandCue2(actor, kind)`.
    MsCheckCommandCue2 @ 0x007b_0770 => extern "C" fn ms_check_command_cue2(actor: u32, kind: u32) -> u32 {
        state::ms_check_command_cue2(&mem(), actor, kind)
    }
    /// `MsOrderInit`.
    MsOrderInit @ 0x0078_d4b0 => extern "C" fn ms_order_init() {
        state::ms_order_init(&mut mem());
    }
    /// `MsOrderRegist(chr)`: the low byte.
    MsOrderRegist @ 0x0078_d4c0 => extern "C" fn ms_order_regist(chr: u32) {
        state::ms_order_regist(&mut mem(), chr.to_le_bytes()[0]);
    }
    /// `MsMuteBattleSubTitle(on)`.
    MsMuteBattleSubTitle @ 0x0078_5990 => extern "C" fn ms_mute_battle_sub_title(on: u32) {
        state::ms_mute_battle_sub_title(&mut mem(), on);
    }
    /// `MsEventEffectInit`.
    MsEventEffectInit @ 0x0078_8be0 => extern "C" fn ms_event_effect_init() {
        state::ms_event_effect_init(&mut mem());
    }
    /// `MsSEPtransAmount`.
    MsSEPtransAmount @ 0x007a_04b0 => extern "C" fn ms_sep_trans_amount() -> u32 {
        state::ms_sep_trans_amount(&mem())
    }
    /// `MsSEPtransCheck`.
    MsSEPtransCheck @ 0x007a_04c0 => extern "C" fn ms_sep_trans_check() -> i32 {
        state::ms_sep_trans_check(&mem())
    }
    /// `MsDebugGetFieldNum`.
    MsDebugGetFieldNum @ 0x007c_5f50 => extern "C" fn ms_debug_get_field_num() -> u32 {
        state::ms_debug_get_field_num(&mem())
    }
    /// `MsCheckStat(i)`.
    MsCheckStat @ 0x0079_e310 => extern "C" fn ms_check_stat(i: i32) -> u32 {
        state::ms_check_stat(&mem(), i)
    }
    /// `MsCheckStat2(i)`.
    MsCheckStat2 @ 0x0079_e320 => extern "C" fn ms_check_stat2(i: i32) -> u32 {
        state::ms_check_stat2(&mem(), i)
    }

    // Helpers.
    /// `MsStructClear(p, n)`.
    MsStructClear @ 0x0079_f230 => extern "C" fn ms_struct_clear(p: u32, n: i32) {
        util::ms_struct_clear(&mut mem(), p, n);
    }
    /// `MsStructFill(p, n, v)`.
    MsStructFill @ 0x0079_f260 => extern "C" fn ms_struct_fill(p: u32, n: i32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the original fills 16 bits
        util::ms_struct_fill(&mut mem(), p, n, v as u16);
    }
    /// `MsStrCopy(src, dst, max)`.
    MsStrCopy @ 0x007c_f6c0 => extern "C" fn ms_str_copy(src: u32, dst: u32, max: i32) -> i32 {
        util::ms_str_copy(&mut mem(), src, dst, max)
    }
    /// `MsGetEff2Chr(x)`.
    MsGetEff2Chr @ 0x0078_8de0 => extern "C" fn ms_get_eff2_chr(x: u32) -> u32 {
        util::ms_get_eff2_chr(x)
    }
    /// `MsCheckMegas(n)`.
    MsCheckMegas @ 0x007a_da40 => extern "C" fn ms_check_megas(n: i32) -> i32 {
        util::ms_check_megas(n)
    }
    /// `MsSetHashValue(list)`.
    MsSetHashValue @ 0x007c_f640 => extern "C" fn ms_set_hash_value(list: u32) {
        util::ms_set_hash_value(&mut mem(), list);
    }
    /// `MsGetEffID(group, a, b)`.
    MsGetEffID @ 0x0078_8df0 => extern "C" fn ms_get_eff_id(group: u32, a: u32, b: u32) -> u32 {
        util::ms_get_eff_id(group, a, b)
    }
    /// `MsAutoRepeatInit(p, first, next)`.
    MsAutoRepeatInit @ 0x007c_89a0 => extern "C" fn ms_auto_repeat_init(p: u32, first: i32, next: i32) {
        util::ms_auto_repeat_init(&mut mem(), p, first, next);
    }
    /// `MsAutoRepeatCheck(p, input)`.
    MsAutoRepeatCheck @ 0x007c_8950 => extern "C" fn ms_auto_repeat_check(p: u32, input: i32) -> i32 {
        util::ms_auto_repeat_check(&mut mem(), p, input)
    }

    // The battle camera.
    /// `MsCameraGetNum(cam)`.
    MsCameraGetNum @ 0x007b_d340 => extern "C" fn ms_camera_get_num(cam: u32) -> u32 {
        camera::ms_camera_get_num(&mem(), cam)
    }
    /// `MsCameraIDcheck(id)`.
    MsCameraIDcheck @ 0x007b_d560 => extern "C" fn ms_camera_id_check(id: u32) -> i32 {
        camera::ms_camera_id_check(&mem(), id)
    }
    /// `MsCameraGetPosNow(side, out)`.
    MsCameraGetPosNow @ 0x007b_d470 => extern "C" fn ms_camera_get_pos_now(side: u32, out: u32) {
        camera::ms_camera_get_pos_now(&mut mem(), side, out);
    }
    /// `MsCameraGetScrDpt`: a float, on the x87 stack.
    MsCameraGetScrDpt @ 0x007b_d510 => extern "C" fn ms_camera_get_scr_dpt() -> f32 {
        camera::ms_camera_get_scr_dpt(&mem())
    }
    /// `MsCameraPriority(id, bonus)`: the low byte of `bonus`.
    MsCameraPriority @ 0x007b_e0a0 => extern "C" fn ms_camera_priority(id: u32, bonus: u32) {
        camera::ms_camera_priority(&mut mem(), id, bonus.to_le_bytes()[0]);
    }
    /// `MsCameraPriorityMask(on)`.
    MsCameraPriorityMask @ 0x007b_e100 => extern "C" fn ms_camera_priority_mask(on: u32) {
        camera::ms_camera_priority_mask(&mut mem(), on);
    }
    /// `MsScreenResetShake(screen)`.
    MsScreenResetShake @ 0x007c_56c0 => extern "C" fn ms_screen_reset_shake(screen: u32) {
        camera::ms_screen_reset_shake(&mut mem(), screen);
    }
    /// `MsGetScreenMatrix`.
    MsGetScreenMatrix @ 0x007b_bd70 => extern "C" fn ms_get_screen_matrix() -> u32 {
        camera::ms_get_screen_matrix(&mem())
    }
    /// `MsGetCameraMatrix`.
    MsGetCameraMatrix @ 0x007c_04b0 => extern "C" fn ms_get_camera_matrix() -> u32 {
        camera::ms_get_camera_matrix(&mem())
    }

    // Damage, hits and statuses. DmgCalc and DmgCalc_InflictStatus_Extra
    // keep values in ECX or EDX across some of their callees
    // (DmgCalc_ApplyZombieDrainInteraction, DmgCalc_Alchemy,
    // DmgCalc_ShatterFail, DmgCalc_NoDamageDeath, DmgCalc_HitKind,
    // DmgCalc_IsDistill: their compiler knew those leave them alone); with
    // both callers replaced too, nothing relies on that any more.
    /// `MsGetRamChrSummon(chr)`.
    MsGetRamChrSummon @ 0x0079_af60 => extern "C" fn ms_get_ram_chr_summon(chr: u32) -> u32 {
        dmg::ms_get_ram_chr_summon(&mem(), chr)
    }
    /// `MsGetRndChr(chr, kind)`: the generator for an actor's rolls.
    MsGetRndChr @ 0x0078_d210 => extern "C" fn ms_get_rnd_chr(chr: u32, kind: i32) -> u32 {
        dmg::ms_get_rnd_chr(&mem(), chr, kind)
    }
    /// `MsCalcDamageCommand(user, target, command, formula, power, status, stat, vary, &def, &mdef, fallback)`.
    MsCalcDamageCommand @ 0x0078_9bf0 => extern "C" fn ms_calc_damage_command(user: u32, target: u32, command: u32, formula: i32, power: i32, status: u32, stat: i32, vary: i32, def: u32, mdef: u32, fallback: i32) -> i32 {
        dmg::ms_calc_damage_command(&mut mem(), user, target, command, formula, power, status, stat, vary, def, mdef, fallback)
    }
    /// `MsCalcEscape(chr)`.
    MsCalcEscape @ 0x0078_a780 => extern "C" fn ms_calc_escape(chr: u32) -> u32 {
        dmg::ms_calc_escape(&mut mem(), chr)
    }
    /// `DmgCalc_StatBoostAbility(user_id, target_id, command, damage)`.
    DmgCalc_StatBoostAbility @ 0x0078_91e0 => extern "C" fn dmg_calc_stat_boost_ability(user_id: u32, target_id: u32, command: u32, damage: i32) -> i32 {
        dmg::dmg_calc_stat_boost_ability(&mem(), user_id, target_id, command, damage)
    }
    /// `DmgCalc_PowerMagicBreak(user, command, damage)`.
    DmgCalc_PowerMagicBreak @ 0x0078_9290 => extern "C" fn dmg_calc_power_magic_break(user: u32, command: u32, damage: i32) -> i32 {
        dmg::dmg_calc_power_magic_break(&mem(), user, command, damage)
    }
    /// `DmgCalc_Crit(user, target, command, &flags, damage)`.
    DmgCalc_Crit @ 0x0078_9690 => extern "C" fn dmg_calc_crit(user: u32, target: u32, command: u32, flags: u32, damage: i32) -> i32 {
        dmg::dmg_calc_crit(&mut mem(), user, target, command, flags, damage)
    }
    /// `DmgCalc_Elem(target, command, elements, damage)`.
    DmgCalc_Elem @ 0x0078_a360 => extern "C" fn dmg_calc_elem(target: u32, _command: u32, elements: u32, damage: i32) -> i32 {
        dmg::dmg_calc_elem(&mem(), target, elements, damage)
    }
    /// `DmgCalc_Defend(target, command, &flags, info, damage)`.
    DmgCalc_Defend @ 0x0078_a7d0 => extern "C" fn dmg_calc_defend(target: u32, command: u32, flags: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_defend(&mut mem(), target, command, flags, info, damage)
    }
    /// `DmgCalc_Armored(user, target, command, info, &armored, damage)`.
    DmgCalc_Armored @ 0x0078_a830 => extern "C" fn dmg_calc_armored(user: u32, target: u32, command: u32, info: u32, armored: u32, damage: i32) -> i32 {
        dmg::dmg_calc_armored(&mut mem(), user, target, command, info, armored, damage)
    }
    /// `DmgCalc_CheckHit(user, target, command, info, counter)`: 0 hit, 1 miss, 2 none.
    DmgCalc_CheckHit @ 0x0078_a890 => extern "C" fn dmg_calc_check_hit(user: u32, target: u32, command: u32, info: u32, counter: i32) -> u32 {
        dmg::dmg_calc_check_hit(&mut mem(), user, target, command, info, counter)
    }
    /// `DmgCalc_DmgImmunity(target, command, &what, mask, &nulled, damage)`.
    DmgCalc_DmgImmunity @ 0x0078_aac0 => extern "C" fn dmg_calc_dmg_immunity(target: u32, command: u32, what: u32, mask: u32, nulled: u32, damage: i32) -> i32 {
        dmg::dmg_calc_dmg_immunity(&mut mem(), target, command, what, mask, nulled, damage)
    }
    /// `DmgCalc_Protect(command, &flags, &hit, info, damage)`.
    DmgCalc_Protect @ 0x0078_ad40 => extern "C" fn dmg_calc_protect(command: u32, flags: u32, hit: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_protect(&mut mem(), command, flags, hit, info, damage)
    }
    /// `DmgCalc_GravImmunity(target, formula, &what, mask, &nulled, damage)`.
    DmgCalc_GravImmunity @ 0x0078_ad80 => extern "C" fn dmg_calc_grav_immunity(target: u32, formula: i32, what: u32, mask: u32, nulled: u32, damage: i32) -> i32 {
        dmg::dmg_calc_grav_immunity(&mut mem(), target, formula, what, mask, nulled, damage)
    }
    /// `DmgCalc_Shell(command, &flags, &hit, info, damage)`.
    DmgCalc_Shell @ 0x0078_adc0 => extern "C" fn dmg_calc_shell(command: u32, flags: u32, hit: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_shell(&mut mem(), command, flags, hit, info, damage)
    }
    /// `DmgCalc_Threaten(target, info, &what, mask, flags, counts, damage)`.
    DmgCalc_Threaten @ 0x0078_bd50 => extern "C" fn dmg_calc_threaten(_target: u32, info: u32, what: u32, mask: u32, flags: u32, counts: u32, damage: u32) -> u32 {
        dmg::dmg_calc_threaten(&mut mem(), info, what, mask, flags, counts, damage)
    }
    /// `DmgCalc_ReelBonus(user, damage)`: Wakka's reels.
    DmgCalc_ReelBonus @ 0x0078_bd90 => extern "C" fn dmg_calc_reel_bonus(user: u32, damage: i32) -> u32 {
        dmg::dmg_calc_reel_bonus(&mem(), user, damage)
    }
    /// `DmgCalc_MarkDamaged(user_id, user, command, damage, always)`.
    DmgCalc_MarkDamaged @ 0x0078_bf50 => extern "C" fn dmg_calc_mark_damaged(_user_id: u32, user: u32, command: u32, damage: u32, always: i32) -> u32 {
        dmg::dmg_calc_mark_damaged(&mut mem(), user, command, damage, always)
    }
    /// `DmgCalc_CheckNuls(command, elements, info)`.
    DmgCalc_CheckNuls @ 0x0078_bfb0 => extern "C" fn dmg_calc_check_nuls(_command: u32, elements: u32, info: u32) -> i32 {
        dmg::dmg_calc_check_nuls(&mut mem(), elements, info)
    }
    /// `DmgCalc_Berserk(user, damage)`.
    DmgCalc_Berserk @ 0x0078_c0b0 => extern "C" fn dmg_calc_berserk(user: u32, damage: i32) -> i32 {
        dmg::dmg_calc_berserk(&mem(), user, damage)
    }
    /// `DmgCalc_QueueRemoveStatusOnHit(target_id, target, command, removed, counts, info)`.
    DmgCalc_QueueRemoveStatusOnHit @ 0x0078_c0f0 => extern "C" fn dmg_calc_queue_remove_status_on_hit(_target_id: u32, target: u32, command: u32, _removed: u32, counts: u32, info: u32) {
        dmg::dmg_calc_queue_remove_status_on_hit(&mut mem(), target, command, counts, info);
    }
    /// `MsAdjustCTB(chr, ctb, haste, slow)`.
    MsAdjustCTB @ 0x0078_c150 => extern "C" fn ms_adjust_ctb(_chr: u32, ctb: i32, haste: i32, slow: i32) -> i32 {
        dmg::ms_adjust_ctb(ctb, haste, slow)
    }
    /// `DmgCalc_DelayImmunity(target, damage, counts, _, flags, delayed)`.
    DmgCalc_DelayImmunity @ 0x0078_c180 => extern "C" fn dmg_calc_delay_immunity(target: u32, damage: u32, counts: u32, _unused: u32, flags: u32, delayed: i32) -> u32 {
        dmg::dmg_calc_delay_immunity(&mut mem(), target, damage, counts, flags, delayed)
    }
    /// `DmgCalc_CheckCounter(apply, user_id, user, target_id, target, command)`.
    DmgCalc_CheckCounter @ 0x0078_c1d0 => extern "C" fn dmg_calc_check_counter(apply: i32, user_id: i32, user: u32, target_id: i32, target: u32, command: u32) -> i32 {
        dmg::dmg_calc_check_counter(&mut mem(), apply, user_id, user, target_id, target, command)
    }
    /// `DmgCalc_ShieldBoost(target, &flags, damage)`.
    DmgCalc_ShieldBoost @ 0x0078_c530 => extern "C" fn dmg_calc_shield_boost(target: u32, flags: u32, damage: i32) -> i32 {
        dmg::dmg_calc_shield_boost(&mut mem(), target, flags, damage)
    }
    /// `DmgCalc_MagicBooster(user, command, command_id, &damage)`.
    DmgCalc_MagicBooster @ 0x0078_c580 => extern "C" fn dmg_calc_magic_booster(user: u32, command: u32, command_id: i32, damage: u32) -> i32 {
        dmg::dmg_calc_magic_booster(&mut mem(), user, command, command_id, damage)
    }
    /// `MsGetNextCTB(chr, rank, haste, slow)`.
    MsGetNextCTB @ 0x0078_d1d0 => extern "C" fn ms_get_next_ctb(chr: u32, rank: i32, haste: i32, slow: i32) -> i32 {
        dmg::ms_get_next_ctb(&mut mem(), chr, rank, haste, slow)
    }
    /// `DebugCanTakeDamage(chr)`.
    DebugCanTakeDamage @ 0x0078_d3a0 => extern "C" fn debug_can_take_damage(chr: u32) -> u32 {
        dmg::debug_can_take_damage(&mem(), chr)
    }
    /// `DmgCalc_ApplyStatUpBuffs(user, target, command)`.
    DmgCalc_ApplyStatUpBuffs @ 0x0078_d730 => extern "C" fn dmg_calc_apply_stat_up_buffs(_user: u32, target: u32, command: u32) {
        dmg::dmg_calc_apply_stat_up_buffs(&mut mem(), target, command);
    }
    /// `SetUpDamageInfo(info, parent, target)`.
    SetUpDamageInfo @ 0x0078_dae0 => extern "C" fn set_up_damage_info(info: u32, parent: u32, target: u32) {
        dmg::set_up_damage_info(&mut mem(), info, parent, target);
    }
    /// `DmgCalc_Delay(target, command, flags, damage)`.
    DmgCalc_Delay @ 0x0078_e0f0 => extern "C" fn dmg_calc_delay(target: u32, command: u32, flags: u32, damage: u32) -> u32 {
        dmg::dmg_calc_delay(&mut mem(), target, command, flags, damage)
    }
    /// `MsCheckStatCount(i)`.
    MsCheckStatCount @ 0x0079_e330 => extern "C" fn ms_check_stat_count(i: i32) -> u32 {
        dmg::ms_check_stat_count(&mem(), i)
    }
    /// `MsCheckWrongStatus(command)`.
    MsCheckWrongStatus @ 0x0079_e340 => extern "C" fn ms_check_wrong_status(command: u32) -> i32 {
        dmg::ms_check_wrong_status(&mem(), command)
    }

    /// `DmgCalc_ApplyZombieDrainInteraction(user, command, info, damage)`.
    DmgCalc_ApplyZombieDrainInteraction @ 0x0078_a200 => extern "C" fn dmg_calc_apply_zombie_drain_interaction(user: u32, command: u32, info: u32, damage: i32) -> i32 {
        dmg::dmg_calc_apply_zombie_drain_interaction(&mem(), user, command, info, damage)
    }
    /// `DmgCalc_ShatterFail(target, info, status, &what, counts, damage)`.
    DmgCalc_ShatterFail @ 0x0078_ba30 => extern "C" fn dmg_calc_shatter_fail(_target: u32, info: u32, status: u32, what: u32, counts: u32, damage: u32) {
        dmg::dmg_calc_shatter_fail(&mut mem(), info, status, what, counts, damage);
    }
    /// `DmgCalc_NoDamageDeath(target, info, status, flags, damage)`.
    DmgCalc_NoDamageDeath @ 0x0078_c480 => extern "C" fn dmg_calc_no_damage_death(_target: u32, info: u32, status: u32, flags: u32, damage: u32) -> u32 {
        dmg::dmg_calc_no_damage_death(&mut mem(), info, status, flags, damage)
    }
    /// `DmgCalc_Alchemy(user, command, command_id, formula, damage)`.
    DmgCalc_Alchemy @ 0x0078_c5f0 => extern "C" fn dmg_calc_alchemy(user: u32, command: u32, command_id: u32, formula: i32, damage: i32) -> i32 {
        dmg::dmg_calc_alchemy(&mem(), user, command, command_id, formula, damage)
    }
    /// `DmgCalc_HitKind(flags, counts)`.
    DmgCalc_HitKind @ 0x0078_c640 => extern "C" fn dmg_calc_hit_kind(flags: i32, counts: u32) -> u32 {
        dmg::dmg_calc_hit_kind(&mem(), flags, counts)
    }
    /// `DmgCalc_IsDistill(status2)`.
    DmgCalc_IsDistill @ 0x0078_c670 => extern "C" fn dmg_calc_is_distill(status2: u32) -> i32 {
        dmg::dmg_calc_is_distill(status2)
    }
    /// `DmgCalc_InflictStatus(user_id, user, target_id, target, command, inflicted, removed, info, flags, damage, &delayed)`.
    DmgCalc_InflictStatus @ 0x0078_ae00 => extern "C" fn dmg_calc_inflict_status(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, inflicted: u32, removed: u32, info: u32, flags: u32, damage: u32, delayed: u32) -> u32 {
        dmg::dmg_calc_inflict_status(&mut mem(), user_id, user, target_id, target, command, inflicted, removed, info, flags, damage, delayed)
    }
    /// `DmgCalc_InflictStatus_Extra(user_id, user, target_id, target, command, inflicted, removed, info, ret, damage, status2)`.
    DmgCalc_InflictStatus_Extra @ 0x0078_b4e0 => extern "C" fn dmg_calc_inflict_status_extra(user_id: u32, user: u32, _target_id: u32, target: u32, command: u32, inflicted: u32, removed: u32, info: u32, ret: u32, _damage: u32, status2: u32) -> u32 {
        dmg::dmg_calc_inflict_status_extra(&mut mem(), user_id, user, target, command, inflicted, removed, info, ret, status2)
    }
    /// `DmgCalc_ResultType(user_id, target_id, class, armored, def, mdef, damage, inflicted, removed, command, status, asleep, what, _, nulls)`.
    DmgCalc_ResultType @ 0x0078_c270 => extern "C" fn dmg_calc_result_type(user_id: u32, target_id: u32, class: i32, armored: i32, def: i32, mdef: i32, damage: u32, inflicted: u32, removed: u32, _command: u32, status: u32, asleep: i32, what: u32, _what2: u32, nulls: u32) -> u32 {
        dmg::dmg_calc_result_type(&mut mem(), user_id, target_id, class, armored, def, mdef, damage, inflicted, removed, status, asleep, what, nulls)
    }
    /// `DmgCalc_Bribe(target_id, target, command, counts, info)`; the language from `TOGetFFXLang`.
    DmgCalc_Bribe @ 0x0078_bdd0 => extern "C" fn dmg_calc_bribe(target_id: u32, target: u32, command: u32, counts: u32, info: u32) -> u32 {
        dmg::dmg_calc_bribe(&mut mem(), target_id, target, command, counts, info, |_| call0(TO_GET_FFX_LANG))
    }
    /// `DmgCalc(user_id, user, target_id, target, command, command_id, info, &damage, &hits, &misses, &changed)`: returns the HP damage.
    DmgCalc @ 0x0078_e630 => extern "C" fn dmg_calc(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, command_id: u32, info: u32, damage: u32, hits: u32, misses: u32, changed: u32) -> i32 {
        let out = dmg::DmgCalcOut { damage, hits, misses, changed };
        dmg::dmg_calc(&mut mem(), user_id, user, target_id, target, command, command_id, info, out, |_| call0(TO_GET_FFX_LANG))
    }

    /// `MsCalcDamage(user_id, user, target_id, target, command, command_id, results, &damage, &hits, &misses, hits_n)`.
    MsCalcDamage @ 0x0078_9740 => extern "C" fn ms_calc_damage(user_id: u32, user: u32, target_id: u32, target: u32, command: u32, command_id: u32, results: u32, damage: u32, hits: u32, misses: u32, hits_n: i32) -> i32 {
        let out = dmg::DmgCalcOut { damage, hits, misses, changed: 0 };
        dmg::ms_calc_damage(&mut mem(), user_id, user, target_id, target, command, command_id, results, out, hits_n, |_| call0(TO_GET_FFX_LANG))
    }
    /// `MsCalcDrainResult(user_id, target, command, results, damage, none)`.
    MsCalcDrainResult @ 0x0078_a240 => extern "C" fn ms_calc_drain_result(user_id: u32, target: u32, command: u32, results: u32, damage: u32, none: i32) {
        dmg::ms_calc_drain_result(&mut mem(), user_id, target, command, results, damage, none);
    }

    // Commands.
    /// `MsGetCommand(chr, _, stop, list, &id)`: a command's kernel record.
    MsGetCommand @ 0x0078_ce50 => extern "C" fn ms_get_command(_chr: u32, _unused: u32, stop: u32, list: u32, id: u32) -> u32 {
        cmd::ms_get_command(&mut mem(), stop, list, id)
    }
    /// `MsGetCommandMP(chr, command)`.
    MsGetCommandMP @ 0x0078_cf70 => extern "C" fn ms_get_command_mp(chr: u32, command: u32) -> i32 {
        cmd::ms_get_command_mp(&mut mem(), chr, command)
    }
    /// `MsCheckUseCommand(chr, command, extra)`: its MP cost, or -1.
    MsCheckUseCommand @ 0x0078_c690 => extern "C" fn ms_check_use_command(chr: u32, command: u32, extra: i32) -> i32 {
        cmd::ms_check_use_command(&mut mem(), chr, command, extra)
    }
    /// `MsCalcMagic(cue, k)`.
    MsCalcMagic @ 0x0078_ab20 => extern "C" fn ms_calc_magic(cue: u32, k: u32) -> i32 {
        cmd::ms_calc_magic(&mut mem(), cue, k)
    }
    /// `MsCalcCostCTB(cue)`: the CTB rank.
    MsCalcCostCTB @ 0x0078_95b0 => extern "C" fn ms_calc_cost_ctb(cue: u32) -> u32 {
        cmd::ms_calc_cost_ctb(&mut mem(), cue)
    }
    /// `MsCalcDamageCTB(cue, ctb, rank, haste, slow)`.
    MsCalcDamageCTB @ 0x0078_98d0 => extern "C" fn ms_calc_damage_ctb(cue: u32, ctb: u32, rank: i32, haste: u32, slow: u32) {
        cmd::ms_calc_damage_ctb(&mut mem(), cue, ctb, rank, haste, slow);
    }

    // Actor getters.
    /// `MsGetRamChrHP(i)`.
    MsGetRamChrHP @ 0x0079_add0 => extern "C" fn ms_get_ram_chr_hp(i: u32) -> u32 {
        actor::ms_get_ram_chr_hp(&mem(), i)
    }
    /// `MsGetRamChrHP_0079adf0(i)`.
    MsGetRamChrHP_0079adf0 @ 0x0079_adf0 => extern "C" fn ms_get_ram_chr_hp_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_hp_max(&mem(), i)
    }
    /// `MsGetRamChrMP(i)`.
    MsGetRamChrMP @ 0x0079_ae50 => extern "C" fn ms_get_ram_chr_mp(i: u32) -> u32 {
        actor::ms_get_ram_chr_mp(&mem(), i)
    }
    /// `MsGetRamChrMPmax(i)`.
    MsGetRamChrMPmax @ 0x0079_ae70 => extern "C" fn ms_get_ram_chr_mp_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_mp_max(&mem(), i)
    }
    /// `MsGetRamChrMag(i)`.
    MsGetRamChrMag @ 0x0079_ae90 => extern "C" fn ms_get_ram_chr_mag(i: u32) -> u32 {
        actor::ms_get_ram_chr_mag(&mem(), i)
    }
    /// `MsGetRamChrName(i)`.
    MsGetRamChrName @ 0x0079_af10 => extern "C" fn ms_get_ram_chr_name(i: u32) -> u32 {
        actor::ms_get_ram_chr_name(&mem(), i)
    }
    /// `MsGetRamChrIconNumber(i)`.
    MsGetRamChrIconNumber @ 0x0079_ae10 => extern "C" fn ms_get_ram_chr_icon_number(i: u32) -> u32 {
        actor::ms_get_ram_chr_icon_number(&mem(), i)
    }
    /// `MsGetRamChrIconType(i)`.
    MsGetRamChrIconType @ 0x0079_ae30 => extern "C" fn ms_get_ram_chr_icon_type(i: u32) -> u32 {
        actor::ms_get_ram_chr_icon_type(&mem(), i)
    }
    /// `MsGetRamChrMegas(i)`.
    MsGetRamChrMegas @ 0x0079_aed0 => extern "C" fn ms_get_ram_chr_megas(i: u32) -> u32 {
        actor::ms_get_ram_chr_megas(&mem(), i)
    }
    /// `MsGetRamChrDisableZan(i)`.
    MsGetRamChrDisableZan @ 0x0079_ad90 => extern "C" fn ms_get_ram_chr_disable_zan(i: u32) -> u32 {
        actor::ms_get_ram_chr_disable_zan(&mem(), i)
    }
    /// `MsGetRamChrCTBmax(i)`.
    MsGetRamChrCTBmax @ 0x0079_ad10 => extern "C" fn ms_get_ram_chr_ctb_max(i: u32) -> u32 {
        actor::ms_get_ram_chr_ctb_max(&mem(), i)
    }
    /// `MsGetChrStatDeath(i)`.
    MsGetChrStatDeath @ 0x0079_5780 => extern "C" fn ms_get_chr_stat_death(i: u32) -> u32 {
        actor::ms_get_chr_stat_death(&mem(), i)
    }
    /// `MsGetChrStatStone(i)`.
    MsGetChrStatStone @ 0x0079_5800 => extern "C" fn ms_get_chr_stat_stone(i: u32) -> u32 {
        actor::ms_get_chr_stat_stone(&mem(), i)
    }
    /// `MsGetChrStatWeak(i)`.
    MsGetChrStatWeak @ 0x0079_5840 => extern "C" fn ms_get_chr_stat_weak(i: u32) -> u32 {
        actor::ms_get_chr_stat_weak(&mem(), i)
    }
    /// `MsGetChrStatZombie(i)`.
    MsGetChrStatZombie @ 0x0079_5880 => extern "C" fn ms_get_chr_stat_zombie(i: u32) -> u32 {
        actor::ms_get_chr_stat_zombie(&mem(), i)
    }
    /// `MsGetChrStatPoison(i)`.
    MsGetChrStatPoison @ 0x0079_57c0 => extern "C" fn ms_get_chr_stat_poison(i: u32) -> u32 {
        actor::ms_get_chr_stat_poison(&mem(), i)
    }
    /// `MsGetChrLimitGauge(i)`.
    MsGetChrLimitGauge @ 0x0079_5550 => extern "C" fn ms_get_chr_limit_gauge(i: u32) -> u32 {
        actor::ms_get_chr_limit_gauge(&mem(), i)
    }
    /// `MsGetChrLimitGaugeMax(i)`.
    MsGetChrLimitGaugeMax @ 0x0079_5590 => extern "C" fn ms_get_chr_limit_gauge_max(i: u32) -> u32 {
        actor::ms_get_chr_limit_gauge_max(&mem(), i)
    }
    /// `MsGetChrHandle(i)`.
    MsGetChrHandle @ 0x0079_5080 => extern "C" fn ms_get_chr_handle(i: u32) -> u32 {
        actor::ms_get_chr_handle(&mem(), i)
    }
    /// `MsGetChrGroup(i)`.
    MsGetChrGroup @ 0x0079_5040 => extern "C" fn ms_get_chr_group(i: u32) -> u32 {
        actor::ms_get_chr_group(&mem(), i)
    }
    /// `MsGetChrCursorFlag(i)`.
    MsGetChrCursorFlag @ 0x0079_4ca0 => extern "C" fn ms_get_chr_cursor_flag(i: u32) -> u32 {
        actor::ms_get_chr_cursor_flag(&mem(), i)
    }
    /// `MsGetChrCommand(i)`.
    MsGetChrCommand @ 0x007b_09b0 => extern "C" fn ms_get_chr_command(i: u32) -> u32 {
        actor::ms_get_chr_command(&mem(), i)
    }
    /// `MsGetChrCommand2(i)`.
    MsGetChrCommand2 @ 0x007b_09e0 => extern "C" fn ms_get_chr_command2(i: u32) -> u32 {
        actor::ms_get_chr_command2(&mem(), i)
    }
    /// `MsBattleCheck`.
    MsBattleCheck @ 0x0078_0c90 => extern "C" fn ms_battle_check() -> u32 {
        state::ms_battle_check(&mem())
    }
    /// `MsBattleChrStart`.
    MsBattleChrStart @ 0x0078_0cb0 => extern "C" fn ms_battle_chr_start() -> u32 {
        state::ms_battle_chr_start(&mem())
    }
    /// `MsBattleGetNum`.
    MsBattleGetNum @ 0x0078_16b0 => extern "C" fn ms_battle_get_num() -> u32 {
        state::ms_battle_get_num(&mem())
    }

    // Vectors and matrices (VU0 code on the x87).
    /// `MsMulVector(v, s, b1, b2, b3)`: `v *= s` (floats as bits).
    MsMulVector @ 0x007c_2e10 => extern "C" fn ms_mul_vector(v: u32, s: u32, b1: u32, b2: u32, b3: u32) {
        vector::ms_mul_vector(&mut mem(), v, [s, b1, b2, b3]);
    }
    /// `MsDivVector(v, s, b1, b2, b3)`: `v *= 1 / s`.
    MsDivVector @ 0x007c_2520 => extern "C" fn ms_div_vector(v: u32, s: u32, b1: u32, b2: u32, b3: u32) {
        vector::ms_div_vector(&mut mem(), v, [s, b1, b2, b3]);
    }
    /// `MsAddPoint(out, a, b)`.
    MsAddPoint @ 0x007c_1480 => extern "C" fn ms_add_point(out: u32, a: u32, b: u32) {
        vector::ms_add_point(&mut mem(), out, a, b);
    }
    /// `MsSetNormalUpVector(v)`.
    MsSetNormalUpVector @ 0x007c_0ab0 => extern "C" fn ms_set_normal_up_vector(v: u32) {
        vector::ms_set_normal_up_vector(&mut mem(), v);
    }
    /// `MsApplyMatrix(out, m, v)`.
    MsApplyMatrix @ 0x007c_1550 => extern "C" fn ms_apply_matrix(out: u32, m: u32, v: u32) {
        vector::ms_apply_matrix(&mut mem(), out, m, v);
    }
    /// `MsNearRag(a, b)`: an angle, on the x87 stack.
    MsNearRag @ 0x007c_04c0 => extern "C" fn ms_near_rag(a: u32, b: u32) -> f32 {
        vector::ms_near_rag(&mem(), a, b)
    }

    // Battle messages.
    /// `MsMenuCloseTitleWindow(keep)`.
    MsMenuCloseTitleWindow @ 0x0078_fa30 => extern "C" fn ms_menu_close_title_window(keep: i32) {
        state::ms_menu_close_title_window(&mut mem(), keep, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        });
    }
    /// `MsMessageCueRegist(kind, actor, message, b, c)`.
    MsMessageCueRegist @ 0x0079_cfd0 => extern "C" fn ms_message_cue_regist(kind: u32, actor: u32, message: u32, b: u32, c: u32) -> u32 {
        let extra = [b.to_le_bytes()[0], c.to_le_bytes()[0]];
        state::ms_message_cue_regist(&mut mem(), kind.to_le_bytes()[0], actor, message, extra, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        })
    }
    /// `MsMonsterCapture(chr, monster)`.
    MsMonsterCapture @ 0x0079_0b30 => extern "C" fn ms_monster_capture(chr: u32, monster: u32) -> u32 {
        state::ms_monster_capture(&mut mem(), chr, monster, |_| {
            call0(TO_BTL_CLOSE_SIGN_WINDOW);
        })
    }

    /// `MsMulMatrix(out, a, b)`.
    MsMulMatrix @ 0x007c_2770 => extern "C" fn ms_mul_matrix(out: u32, a: u32, b: u32) {
        vector::ms_mul_matrix(&mut mem(), out, a, b);
    }
    /// `MsMiddlePoint(out, p, q, s, t, c, d, e)` (floats as bits).
    MsMiddlePoint @ 0x007c_25e0 => extern "C" fn ms_middle_point(out: u32, p: u32, q: u32, s: u32, t: u32, c: u32, d: u32, e: u32) {
        vector::ms_middle_point(&mut mem(), out, p, q, [s, t, c, d, e]);
    }
    /// `MsVectorLen(v)`: on the x87 stack.
    MsVectorLen @ 0x007c_4280 => extern "C" fn ms_vector_len(v: u32) -> f32 {
        vector::ms_vector_len(&mut mem(), v)
    }
    /// `MsVectorLen2D(v)`: on the x87 stack.
    MsVectorLen2D @ 0x007c_43a0 => extern "C" fn ms_vector_len_2d(v: u32) -> f32 {
        vector::ms_vector_len_2d(&mut mem(), v)
    }
    /// `MsVectorNormal(out, v)`.
    MsVectorNormal @ 0x007c_44a0 => extern "C" fn ms_vector_normal(out: u32, v: u32) {
        vector::ms_vector_normal(&mut mem(), out, v);
    }

    // Battle bookkeeping (ffx_battle::misc).
    /// `MsComBufInit`.
    MsComBufInit @ 0x0079_d290 => extern "C" fn ms_com_buf_init() {
        misc::ms_com_buf_init(&mut mem());
    }
    /// `MsEtEffectReadInit`.
    MsEtEffectReadInit @ 0x0079_e730 => extern "C" fn ms_et_effect_read_init() {
        misc::ms_et_effect_read_init(&mut mem());
    }
    /// `MsNumberInit`.
    MsNumberInit @ 0x0079_f9f0 => extern "C" fn ms_number_init() {
        misc::ms_number_init(&mut mem());
    }
    /// `MsActionInit`.
    MsActionInit @ 0x007a_ce90 => extern "C" fn ms_action_init() {
        misc::ms_action_init(&mut mem());
    }
    /// `MsEffectCameraReset`.
    MsEffectCameraReset @ 0x007b_bb00 => extern "C" fn ms_effect_camera_reset() {
        misc::ms_effect_camera_reset(&mut mem());
    }
    /// `MsBattleGetFontAddress`.
    MsBattleGetFontAddress @ 0x0078_1680 => extern "C" fn ms_battle_get_font_address() -> u32 {
        misc::ms_battle_get_font_address(&mem())
    }
    /// `MsRegMotionInit`.
    MsRegMotionInit @ 0x0079_ccd0 => extern "C" fn ms_reg_motion_init() {
        misc::ms_reg_motion_init(&mut mem());
    }
    /// `MsBtlStartAP`.
    MsBtlStartAP @ 0x0079_8e90 => extern "C" fn ms_btl_start_ap() {
        misc::ms_btl_start_ap(&mut mem());
    }
    /// `MsBtlInitItemMem`.
    MsBtlInitItemMem @ 0x0079_a000 => extern "C" fn ms_btl_init_item_mem() {
        misc::ms_btl_init_item_mem(&mut mem());
    }
    /// `MsBtlInitItemMem_007a0cd0`.
    MsBtlInitItemMem_007a0cd0 @ 0x007a_0cd0 => extern "C" fn ms_btl_init_item_mem2() {
        misc::ms_btl_init_item_mem2(&mut mem());
    }
    /// `MsMessageCueInit`.
    MsMessageCueInit @ 0x007a_2730 => extern "C" fn ms_message_cue_init() {
        misc::ms_message_cue_init(&mut mem());
    }
    /// `MsSplineInit`.
    MsSplineInit @ 0x007b_aaa0 => extern "C" fn ms_spline_init() {
        misc::ms_spline_init(&mut mem());
    }
    /// `MsBattleSpecialCameraFree`.
    MsBattleSpecialCameraFree @ 0x0079_dd90 => extern "C" fn ms_battle_special_camera_free() {
        misc::ms_battle_special_camera_free(&mut mem());
    }
    /// `MsBtlReadStatusComplete`.
    MsBtlReadStatusComplete @ 0x007a_d550 => extern "C" fn ms_btl_read_status_complete() {
        misc::ms_btl_read_status_complete(&mut mem());
    }
    /// `MsScreenFocusReset`.
    MsScreenFocusReset @ 0x007b_bd80 => extern "C" fn ms_screen_focus_reset() {
        misc::ms_screen_focus_reset(&mut mem());
    }
    /// `MsBtlListInit`.
    MsBtlListInit @ 0x0079_d250 => extern "C" fn ms_btl_list_init() {
        misc::ms_btl_list_init(&mut mem());
    }
    /// `MsBtlReadInit`.
    MsBtlReadInit @ 0x0078_3000 => extern "C" fn ms_btl_read_init() {
        misc::ms_btl_read_init(&mut mem());
    }
    /// `MsMoveTargetPos`.
    MsMoveTargetPos @ 0x007a_b160 => extern "C" fn ms_move_target_pos() -> u32 {
        misc::ms_move_target_pos(&mem())
    }
    /// `FinishReadPhyreBattleMap`.
    FinishReadPhyreBattleMap @ 0x0078_2fd0 => extern "C" fn finish_read_phyre_battle_map() {
        misc::finish_read_phyre_battle_map(&mut mem());
    }
    /// `MsDebugCameraSleepInit`.
    MsDebugCameraSleepInit @ 0x0079_2200 => extern "C" fn ms_debug_camera_sleep_init() {
        misc::ms_debug_camera_sleep_init(&mut mem());
    }
    /// `MsResetDirMove(p)`.
    MsResetDirMove @ 0x007a_b170 => extern "C" fn ms_reset_dir_move(p: u32) {
        misc::ms_reset_dir_move(&mut mem(), p);
    }
    /// `MsPaymentProcess(p)`.
    MsPaymentProcess @ 0x007a_fa60 => extern "C" fn ms_payment_process(p: u32) -> u32 {
        misc::ms_payment_process(&mem(), p)
    }
    /// `MsMotionSpeedSetNormal(p)`.
    MsMotionSpeedSetNormal @ 0x007a_a260 => extern "C" fn ms_motion_speed_set_normal(p: u32) {
        misc::ms_motion_speed_set_normal(&mut mem(), p);
    }
    /// `msMoveReset(p)`.
    msMoveReset @ 0x007a_a280 => extern "C" fn ms_move_reset(p: u32) {
        misc::ms_move_reset(&mut mem(), p);
    }
    /// `MsSetMenuCommand(p)`.
    MsSetMenuCommand @ 0x0079_2e50 => extern "C" fn ms_set_menu_command(p: u32) {
        misc::ms_set_menu_command(&mut mem(), p);
    }
    /// `MsSummonLimitSave(p)`.
    MsSummonLimitSave @ 0x007b_06e0 => extern "C" fn ms_summon_limit_save(p: u32) {
        misc::ms_summon_limit_save(&mut mem(), p);
    }
    /// `MsSummonLimitLoad(p)`.
    MsSummonLimitLoad @ 0x007b_06b0 => extern "C" fn ms_summon_limit_load(p: u32) {
        misc::ms_summon_limit_load(&mut mem(), p);
    }
    /// `MsEffectGetBufferAddress(p)`.
    MsEffectGetBufferAddress @ 0x007b_7eb0 => extern "C" fn ms_effect_get_buffer_address(p: u32) -> u32 {
        misc::ms_effect_get_buffer_address(&mut mem(), p)
    }
    /// `MsEffectGetLastMagicNum2(p)`.
    MsEffectGetLastMagicNum2 @ 0x0078_7bf0 => extern "C" fn ms_effect_get_last_magic_num2(p: u32) -> u32 {
        misc::ms_effect_get_last_magic_num2(&mem(), p)
    }
    /// `MsMoveSplineSet(p)`.
    MsMoveSplineSet @ 0x007a_b140 => extern "C" fn ms_move_spline_set(p: u32) {
        misc::ms_move_spline_set(&mut mem(), p);
    }
    /// `MsSaveUcID(id)`.
    MsSaveUcID @ 0x007d_42e0 => extern "C" fn ms_save_uc_id(id: u32) {
        misc::ms_save_uc_id(&mut mem(), id.to_le_bytes()[..2].try_into().map_or(0, u16::from_le_bytes));
    }
    /// `MsScreenInit(w, h)`.
    MsScreenInit @ 0x007c_0540 => extern "C" fn ms_screen_init(w: u32, h: u32) {
        misc::ms_screen_init(&mut mem(), w, h);
    }
    /// `MsBattleSpecialCameraPause(camera)`.
    MsBattleSpecialCameraPause @ 0x0079_ddb0 => extern "C" fn ms_battle_special_camera_pause(camera: u32) {
        misc::ms_battle_special_camera_pause(&mut mem(), camera.to_le_bytes()[0]);
    }
    /// `MsScreenOffset(x, y)` (floats as bits).
    MsScreenOffset @ 0x007b_c0c0 => extern "C" fn ms_screen_offset(x: u32, y: u32) {
        misc::ms_screen_offset(&mut mem(), x, y);
    }
    /// `MsScreenResetOffset(x, y)`.
    MsScreenResetOffset @ 0x007b_c720 => extern "C" fn ms_screen_reset_offset(x: u32, y: u32) {
        misc::ms_screen_reset_offset(&mut mem(), x, y);
    }
    /// `MsEffectSetCameraBufFloat(i, f)`.
    MsEffectSetCameraBufFloat @ 0x007b_7f50 => extern "C" fn ms_effect_set_camera_buf_float(i: u32, f: u32) {
        misc::ms_effect_set_camera_buf_float(&mut mem(), i, f);
    }

    /// `MsGetChrBit(chr, selector, all, any)`: a set of actors as bits.
    MsGetChrBit @ 0x0079_4330 => extern "C" fn ms_get_chr_bit(chr: u32, selector: u32, all: i32, any: i32) -> u32 {
        actor::ms_get_chr_bit(&mem(), chr, selector, all, any)
    }
    /// `MsGetChrWidth(i)`: on the x87 stack.
    MsGetChrWidth @ 0x0079_5a20 => extern "C" fn ms_get_chr_width(i: u32) -> f32 {
        actor::ms_get_chr_width(&mem(), i)
    }
    /// `MsGetChrScale(i, out)`.
    MsGetChrScale @ 0x0079_5720 => extern "C" fn ms_get_chr_scale(i: u32, out: u32) -> i32 {
        actor::ms_get_chr_scale(&mut mem(), i, out)
    }
    /// `MsGetChrDist(i)`.
    MsGetChrDist @ 0x0079_4e00 => extern "C" fn ms_get_chr_dist(i: u32) -> u32 {
        actor::ms_get_chr_dist(&mem(), i)
    }
    /// `MsGetChrBtlDir(i)`: on the x87 stack.
    MsGetChrBtlDir @ 0x0079_48a0 => extern "C" fn ms_get_chr_btl_dir(i: u32) -> f32 {
        actor::ms_get_chr_btl_dir(&mem(), i)
    }
    /// `MsGetChrGroundPos(i, &x, &y)`: EAX as the original leaves it.
    MsGetChrGroundPos @ 0x0079_4ff0 => extern "C" fn ms_get_chr_ground_pos(i: u32, x: u32, y: u32) -> u32 {
        actor::ms_get_chr_ground_pos(&mut mem(), i, x, y)
    }
    /// `MsGetChrTargetPos(i, &x, &y)`: EAX as the original leaves it.
    MsGetChrTargetPos @ 0x0079_5920 => extern "C" fn ms_get_chr_target_pos(i: u32, x: u32, y: u32) -> u32 {
        actor::ms_get_chr_target_pos(&mut mem(), i, x, y)
    }
    /// `MsGetChrEventFlag(i, event)`.
    MsGetChrEventFlag @ 0x0079_4f20 => extern "C" fn ms_get_chr_event_flag(i: u32, event: u32) -> u32 {
        actor::ms_get_chr_event_flag(&mem(), i, event)
    }
    /// `MsGetChrMotionSignal(i)`.
    MsGetChrMotionSignal @ 0x0079_5660 => extern "C" fn ms_get_chr_motion_signal(i: u32) -> u32 {
        actor::ms_get_chr_motion_signal(&mem(), i)
    }
    /// `MsGetChrModelID(i)`.
    MsGetChrModelID @ 0x0079_5610 => extern "C" fn ms_get_chr_model_id(i: u32) -> u32 {
        actor::ms_get_chr_model_id(&mem(), i)
    }
    /// `MsGetChrMotionSpeed(i)`.
    MsGetChrMotionSpeed @ 0x0079_56a0 => extern "C" fn ms_get_chr_motion_speed(i: u32) -> u32 {
        actor::ms_get_chr_motion_speed(&mem(), i)
    }
    /// `MsGetWeaponHandle(i, second)`.
    MsGetWeaponHandle @ 0x0079_5b40 => extern "C" fn ms_get_weapon_handle(i: u32, second: i32) -> u32 {
        actor::ms_get_weapon_handle(&mem(), i, second)
    }
    /// `MsGetWeaponSpecial(id, k)`.
    MsGetWeaponSpecial @ 0x0079_5bb0 => extern "C" fn ms_get_weapon_special(id: u32, k: u32) -> u32 {
        actor::ms_get_weapon_special(&mem(), id, k)
    }
    /// `MsCameraGetID(cam)`.
    MsCameraGetID @ 0x007b_d300 => extern "C" fn ms_camera_get_id(cam: u32) -> u32 {
        camera::ms_camera_get_id(&mem(), cam)
    }

    // Items and steals.
    /// `MsSetItemWindow`: the battle item lists.
    MsSetItemWindow @ 0x0079_06a0 => extern "C" fn ms_set_item_window() {
        save::ms_set_item_window(&mut mem());
    }
    /// `MsSaveItemUse(item, n)`.
    MsSaveItemUse @ 0x0079_0550 => extern "C" fn ms_save_item_use(item: u32, n: i32) -> i32 {
        save::ms_save_item_use(&mut mem(), item, n)
    }
    /// `Battle_Steal(user, target, spoils, info, fail)`.
    Battle_Steal @ 0x0078_b760 => extern "C" fn battle_steal(user: u32, target: u32, spoils: u32, info: u32, fail: i32) -> u32 {
        save::battle_steal(&mut mem(), user, target, spoils, info, fail, |_| call0(TO_GET_FFX_LANG), |_, n| {
            call1(ACHIEVEMENT_UNLOCK, n);
        })
    }
    /// `Battle_StealGil(user, target, spoils, info, fail)`: Mug's gil.
    Battle_StealGil @ 0x0078_b920 => extern "C" fn battle_steal_gil(_user: u32, target: u32, spoils: u32, info: u32, fail: i32) -> u32 {
        save::battle_steal_gil(&mut mem(), target, spoils, info, fail)
    }
    /// `MsGetActionBuffer(actor)`.
    MsGetActionBuffer @ 0x0078_d040 => extern "C" fn ms_get_action_buffer(actor: u32) -> u32 {
        save::ms_get_action_buffer(&mut mem(), actor)
    }

    /// `MsGetCommandFar(chr, command)`.
    MsGetCommandFar @ 0x0079_9690 => extern "C" fn ms_get_command_far(chr: i32, command: u32) -> u32 {
        actor::ms_get_command_far(&mem(), chr, command)
    }
    /// `MsGetCursorTarget(chr, selector, command, event)`.
    MsGetCursorTarget @ 0x0079_9820 => extern "C" fn ms_get_cursor_target(chr: u32, selector: u32, command: u32, event: u32) -> u32 {
        actor::ms_get_cursor_target(&mem(), chr, selector, command, event)
    }
    /// `MsGetGuardChr(user, target)`.
    MsGetGuardChr @ 0x0078_eef0 => extern "C" fn ms_get_guard_chr(user: u32, target: u32) -> u32 {
        actor::ms_get_guard_chr(&mut mem(), user, target)
    }
    /// `MsGetChrVigor(i)`.
    MsGetChrVigor @ 0x0078_cce0 => extern "C" fn ms_get_chr_vigor(i: u32) -> u32 {
        actor::ms_get_chr_vigor(&mem(), i)
    }
    /// `MsSetDirThetaMode(actor)`.
    MsSetDirThetaMode @ 0x0079_6940 => extern "C" fn ms_set_dir_theta_mode(actor: u32) -> u32 {
        minor::ms_set_dir_theta_mode(&mut mem(), actor)
    }
    /// `MsBtlListField(i)`.
    MsBtlListField @ 0x0079_d190 => extern "C" fn ms_btl_list_field(i: i32) -> u32 {
        minor::ms_btl_list_field(&mem(), i)
    }
    /// `MsBtlListFieldNum(map)`.
    MsBtlListFieldNum @ 0x0079_d1c0 => extern "C" fn ms_btl_list_field_num(map: i32) -> i32 {
        minor::ms_btl_list_field_num(&mem(), map)
    }
    /// `ResetEncountExe(all)`.
    ResetEncountExe @ 0x0078_0ff0 => extern "C" fn reset_encount_exe(all: u32) {
        minor::reset_encount_exe(&mut mem(), all);
    }
    /// `MsEffectCameraCall(camera, mode)`.
    MsEffectCameraCall @ 0x0078_7740 => extern "C" fn ms_effect_camera_call(camera: i32, mode: u32) -> u32 {
        minor::ms_effect_camera_call(&mut mem(), camera, mode)
    }
    /// `resetZanmatoSepList()`.
    resetZanmatoSepList @ 0x007a_8f20 => extern "C" fn reset_zanmato_sep_list() -> u32 {
        minor::reset_zanmato_sep_list(&mut mem())
    }
    /// `MsSetMotBinAddress(actor, i)`.
    MsSetMotBinAddress @ 0x0079_cd80 => extern "C" fn ms_set_mot_bin_address(actor: u32, i: i32) -> u32 {
        minor::ms_set_mot_bin_address(&mut mem(), actor, i)
    }
    /// `MsAtelRegist(manager, name, script)`.
    MsAtelRegist @ 0x0079_7a40 => extern "C" fn ms_atel_regist(manager: i32, _name: u32, script: u32) -> i32 {
        minor::ms_atel_regist(&mut mem(), manager, script)
    }
    /// `MsSEPtransInit()`.
    MsSEPtransInit @ 0x007a_0500 => extern "C" fn ms_sep_trans_init() -> u32 {
        minor::ms_sep_trans_init(&mut mem())
    }
    /// `MsAtelCamInit()`.
    MsAtelCamInit @ 0x0079_7500 => extern "C" fn ms_atel_cam_init() {
        minor::ms_atel_cam_init(&mut mem());
    }
    /// `Mon_FindHandler(file, i, &k)`.
    Mon_FindHandler @ 0x0079_7410 => extern "C" fn mon_find_handler(file: u32, i: i32, k: u32) -> u32 {
        minor::mon_find_handler(&mut mem(), file, i, k)
    }
    /// `MsMotBinFrame(_, actor, motion, frame)`.
    MsMotBinFrame @ 0x0079_cb40 => extern "C" fn ms_mot_bin_frame(_x: u32, actor: u32, motion: u32, frame: u32) -> u32 {
        minor::ms_mot_bin_frame(&mem(), actor, motion, frame)
    }
    /// `MsSEP3DfadeOutAll(time)`.
    MsSEP3DfadeOutAll @ 0x007a_08e0 => extern "C" fn ms_sep3d_fade_out_all(time: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        minor::ms_sep3d_fade_out_all(&mut mem(), time as u16)
    }
    /// `MsWeaponReadSet(actor, slot, weapon)`.
    MsWeaponReadSet @ 0x007a_d940 => extern "C" fn ms_weapon_read_set(actor: u32, slot: u32, weapon: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        minor::ms_weapon_read_set(&mut mem(), actor, slot as u8, weapon);
    }
    /// `MsEffectCameraBufInit()`.
    MsEffectCameraBufInit @ 0x0078_76e0 => extern "C" fn ms_effect_camera_buf_init() {
        minor::ms_effect_camera_buf_init(&mut mem());
    }
    /// `MsInitSaveWeaponEquipFlag()`.
    MsInitSaveWeaponEquipFlag @ 0x007a_be30 => extern "C" fn ms_init_save_weapon_equip_flag() -> u32 {
        minor::ms_init_save_weapon_equip_flag(&mut mem())
    }
    /// `MsStoneRegist(chr)`.
    MsStoneRegist @ 0x007a_2780 => extern "C" fn ms_stone_regist(chr: u32) -> u32 {
        minor::ms_stone_regist(&mut mem(), chr)
    }
    /// `MsSetSaveCommandUse(chr, command, on)`.
    MsSetSaveCommandUse @ 0x0078_5e00 => extern "C" fn ms_set_save_command_use(chr: i32, command: u32, on: u32) -> u32 {
        minor::ms_set_save_command_use(&mut mem(), chr, command, on)
    }
    /// `MsAtelCameraInit()`.
    MsAtelCameraInit @ 0x007b_a650 => extern "C" fn ms_atel_camera_init() {
        minor::ms_atel_camera_init(&mut mem());
    }
    /// `MsNumberCalcProcess()`.
    MsNumberCalcProcess @ 0x0079_f5e0 => extern "C" fn ms_number_calc_process() -> u32 {
        minor::ms_number_calc_process(&mut mem())
    }
    /// `MsGetNowSelectedCamraLevel()`.
    MsGetNowSelectedCamraLevel @ 0x007b_bc30 => extern "C" fn ms_get_now_selected_camra_level() -> i32 {
        camera::ms_get_now_selected_camra_level(&mem())
    }
    /// `MsCameraFree(id)`.
    MsCameraFree @ 0x007b_d000 => extern "C" fn ms_camera_free(id: u32) -> u32 {
        camera::ms_camera_free(&mut mem(), id)
    }
    /// `MsEffectCameraInit()`.
    MsEffectCameraInit @ 0x007b_ba70 => extern "C" fn ms_effect_camera_init() -> u32 {
        camera::ms_effect_camera_init(&mut mem())
    }
    /// `MsEffectCameraSet(look_at, eye, depth, roll)` (floats as bits).
    MsEffectCameraSet @ 0x007b_bb10 => extern "C" fn ms_effect_camera_set(look_at: u32, eye: u32, depth: u32, roll: u32) -> u32 {
        camera::ms_effect_camera_set(&mut mem(), look_at, eye, depth, roll)
    }
    /// `MsSetScreen(i, a, b, c, d, e, near, far, z0, z1)` (floats as bits).
    MsSetScreen @ 0x007b_c7f0 => extern "C" fn ms_set_screen(i: i32, a: u32, b: u32, c: u32, d: u32, e: u32, near: u32, far: u32, z0: u32, z1: u32) {
        camera::ms_set_screen(&mut mem(), i, [a, b, c, d, e, near, far, z0, z1]);
    }
    /// `MsVectorGetSide(ax, az, bx, bz)` (floats as bits).
    MsVectorGetSide @ 0x007c_4180 => extern "C" fn ms_vector_get_side(ax: u32, az: u32, bx: u32, bz: u32) -> i32 {
        vector::ms_vector_get_side(&mut mem(), [ax, az, bx, bz])
    }
    /// `Battle_FirstSelectedActor(chr, selector, all, any)`.
    Battle_FirstSelectedActor @ 0x0079_47f0 => extern "C" fn battle_first_selected_actor(chr: u32, selector: u32, all: i32, any: i32) -> i32 {
        body::battle_first_selected_actor(&mem(), chr, selector, all, any)
    }
    /// `msSetChrBodyHit(actor, part, on)`.
    msSetChrBodyHit @ 0x0079_6f20 => extern "C" fn ms_set_chr_body_hit(actor: u32, part: i32, on: i32) -> u32 {
        body::ms_set_chr_body_hit(&mut mem(), actor, part, on)
    }
    /// `MsGetChrHeight(i)`: on the x87 stack.
    MsGetChrHeight @ 0x0079_50f0 => extern "C" fn ms_get_chr_height(i: u32) -> f32 {
        body::ms_get_chr_height(&mem(), i)
    }
    /// `MsGetChrCamHeight(i)`: on the x87 stack.
    MsGetChrCamHeight @ 0x0079_4a30 => extern "C" fn ms_get_chr_cam_height(i: u32) -> f32 {
        body::ms_get_chr_cam_height(&mem(), i)
    }
    /// `MsGetChrCamWidth(i)`: on the x87 stack.
    MsGetChrCamWidth @ 0x0079_4a70 => extern "C" fn ms_get_chr_cam_width(i: u32) -> f32 {
        body::ms_get_chr_cam_width(&mem(), i)
    }
    /// `MsGetChrDir(i, &dir)`.
    MsGetChrDir @ 0x0079_4d20 => extern "C" fn ms_get_chr_dir(i: u32, dir: u32) -> i32 {
        body::ms_get_chr_dir(&mut mem(), i, dir)
    }
    /// `MsGetChrGround(i)`: on the x87 stack.
    MsGetChrGround @ 0x0079_4fb0 => extern "C" fn ms_get_chr_ground(i: u32) -> f32 {
        body::ms_get_chr_ground(&mem(), i)
    }
    /// `MsSetChrPos(i, pos)`.
    MsSetChrPos @ 0x0079_6150 => extern "C" fn ms_set_chr_pos(i: u32, pos: u32) -> i32 {
        body::ms_set_chr_pos(&mut mem(), i, pos)
    }
    /// `MsSetChrWeight(i, how)`.
    MsSetChrWeight @ 0x0079_68e0 => extern "C" fn ms_set_chr_weight(i: u32, how: i32) {
        body::ms_set_chr_weight(&mut mem(), i, how);
    }
    /// `msSetChrWeight(actor, w)` (a float as bits).
    msSetChrWeight @ 0x0079_72c0 => extern "C" fn ms_set_chr_weight2(actor: u32, w: u32) {
        body::ms_set_chr_weight2(&mut mem(), actor, w);
    }
    /// `MsSetChrMoveFlag(i, v)`.
    MsSetChrMoveFlag @ 0x0079_6110 => extern "C" fn ms_set_chr_move_flag(i: u32, v: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        body::ms_set_chr_move_flag(&mut mem(), i, v as u8)
    }
    /// `MsSetChrMotionLevel(i, v)`.
    MsSetChrMotionLevel @ 0x0079_60d0 => extern "C" fn ms_set_chr_motion_level(i: u32, v: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        body::ms_set_chr_motion_level(&mut mem(), i, v as u8)
    }
    /// `MsSetChrSpecular(i, v, frames)` (v a float as bits).
    MsSetChrSpecular @ 0x0079_6360 => extern "C" fn ms_set_chr_specular(i: u32, v: u32, frames: i32) -> u32 {
        body::ms_set_chr_specular(&mut mem(), i, v, frames)
    }
    /// `MsCheckChrMotionAll(mode)`.
    MsCheckChrMotionAll @ 0x0079_1630 => extern "C" fn ms_check_chr_motion_all(mode: i32) -> u32 {
        body::ms_check_chr_motion_all(&mut mem(), mode)
    }
    /// `MsEffectResetMotionDisable(actor)`.
    MsEffectResetMotionDisable @ 0x0078_8270 => extern "C" fn ms_effect_reset_motion_disable(actor: u32) -> u32 {
        minor::ms_effect_reset_motion_disable(&mut mem(), actor)
    }
    /// `MsGetNextAP(chr)`.
    MsGetNextAP @ 0x0078_4e90 => extern "C" fn ms_get_next_ap(chr: i32) -> i32 {
        menu::ms_get_next_ap(&mut mem(), chr)
    }
    /// `MsGetSaveChrName(chr)`.
    MsGetSaveChrName @ 0x0078_4fb0 => extern "C" fn ms_get_save_chr_name(chr: u32) -> u32 {
        menu::ms_get_save_chr_name(&mem(), chr)
    }
    /// `MsMenuStatusNum(id, strings)`.
    MsMenuStatusNum @ 0x0079_0200 => extern "C" fn ms_menu_status_num(id: i32, strings: u32) -> u32 {
        menu::ms_menu_status_num(&mut mem(), id, strings)
    }
    /// `MsWeaponName(id, k, second, &icon)`.
    MsWeaponName @ 0x007a_0c50 => extern "C" fn ms_weapon_name(id: u32, k: u32, second: u32, icon: u32) -> u32 {
        menu::ms_weapon_name(&mut mem(), id, k, second, icon)
    }
    /// `MsMenuGetChrSkipCommand(i)`.
    MsMenuGetChrSkipCommand @ 0x0078_fb80 => extern "C" fn ms_menu_get_chr_skip_command(i: u32) -> u32 {
        menu::ms_menu_get_chr_skip_command(&mem(), i)
    }
    /// `MsGetCheckEnglish()`: the language from `TkGetLang` (menu).
    MsGetCheckEnglish @ 0x0078_4e60 => extern "C" fn ms_get_check_english() -> i32 {
        menu::ms_get_check_english(ffx_menu::tk::bridge::tk_get_lang(&mem()))
    }
    /// `MsGetBattleEndStatus()`.
    MsGetBattleEndStatus @ 0x0079_28d0 => extern "C" fn ms_get_battle_end_status() -> u32 {
        menu::ms_get_battle_end_status(&mem())
    }
    /// `MsCameraSetRoll(id, roll)` (a float as bits).
    MsCameraSetRoll @ 0x007b_fa80 => extern "C" fn ms_camera_set_roll(id: u32, roll: u32) {
        camera::ms_camera_set_roll(&mut mem(), id, roll);
    }
    /// `MsCameraSetScrDpt(id, depth)` (a float as bits).
    MsCameraSetScrDpt @ 0x007b_fbd0 => extern "C" fn ms_camera_set_scr_dpt(id: u32, depth: u32) {
        camera::ms_camera_set_scr_dpt(&mut mem(), id, depth);
    }
    /// `MsDebugGetCameraMode(side)`.
    MsDebugGetCameraMode @ 0x007c_fb20 => extern "C" fn ms_debug_get_camera_mode(side: i32) -> u32 {
        camera::ms_debug_get_camera_mode(&mem(), side)
    }
    /// `MsCopyCameraMatrix(out)`.
    MsCopyCameraMatrix @ 0x007c_0070 => extern "C" fn ms_copy_camera_matrix(out: u32) -> u32 {
        camera::ms_copy_camera_matrix(&mut mem(), out)
    }
    /// `MsCopyScreenMatrix(out)`.
    MsCopyScreenMatrix @ 0x007b_ba50 => extern "C" fn ms_copy_screen_matrix(out: u32) -> u32 {
        camera::ms_copy_screen_matrix(&mut mem(), out)
    }
    /// `MsSubPoint(out, a, b)`.
    MsSubPoint @ 0x007c_40b0 => extern "C" fn ms_sub_point(out: u32, a: u32, b: u32) {
        vector::ms_sub_point(&mut mem(), out, a, b);
    }
    /// `MsPointLen(a, b)`: on the x87 stack.
    MsPointLen @ 0x007c_2ec0 => extern "C" fn ms_point_len(a: u32, b: u32) -> f32 {
        vector::ms_point_len(&mut mem(), a, b)
    }
    /// `MsPointLen2D(a, b)`: on the x87 stack.
    MsPointLen2D @ 0x007c_3030 => extern "C" fn ms_point_len_2d(a: u32, b: u32) -> f32 {
        vector::ms_point_len_2d(&mut mem(), a, b)
    }
    /// `Battle_ChrMoveFlags(actor)`.
    Battle_ChrMoveFlags @ 0x007b_4a70 => extern "C" fn battle_chr_move_flags(actor: u32) -> u32 {
        body::battle_chr_move_flags(&mem(), actor)
    }
    /// `MsMenuCountStatus(i)`.
    MsMenuCountStatus @ 0x0078_fa80 => extern "C" fn ms_menu_count_status(i: u32) -> u32 {
        minor::ms_menu_count_status(&mem(), i)
    }
    /// `MsMenuDefCountStatus(i)`.
    MsMenuDefCountStatus @ 0x0078_faa0 => extern "C" fn ms_menu_def_count_status(i: u32) -> u32 {
        minor::ms_menu_def_count_status(&mem(), i)
    }
    /// `MsMenuHalfElement(i)`.
    MsMenuHalfElement @ 0x0078_fe40 => extern "C" fn ms_menu_half_element(i: u32) -> u32 {
        minor::ms_menu_half_element(&mem(), i)
    }
    /// `MsMenuBitStatus(i)`.
    MsMenuBitStatus @ 0x0078_f950 => extern "C" fn ms_menu_bit_status(i: u32) -> u32 {
        minor::ms_menu_bit_status(&mem(), i)
    }
    /// `MsGetSummonerNum(i)`.
    MsGetSummonerNum @ 0x0078_8e10 => extern "C" fn ms_get_summoner_num(i: u32) -> u32 {
        minor::ms_get_summoner_num(&mem(), i)
    }
    /// `MsMenuWeakElement(i)`.
    MsMenuWeakElement @ 0x007a_9a20 => extern "C" fn ms_menu_weak_element(i: u32) -> u32 {
        minor::ms_menu_weak_element(&mem(), i)
    }
    /// `MsMenuCheckChrLive(i)`.
    MsMenuCheckChrLive @ 0x0078_f970 => extern "C" fn ms_menu_check_chr_live(i: u32) -> u32 {
        minor::ms_menu_check_chr_live(&mem(), i)
    }
    /// `MsGetSummonEnergy(i)`.
    MsGetSummonEnergy @ 0x007b_2b00 => extern "C" fn ms_get_summon_energy(i: u32) -> u32 {
        minor::ms_get_summon_energy(&mem(), i)
    }
    /// `MsEffectGetTargetFlag(i)`.
    MsEffectGetTargetFlag @ 0x0078_7c10 => extern "C" fn ms_effect_get_target_flag(i: u32) -> i32 {
        minor::ms_effect_get_target_flag(&mem(), i)
    }
    /// `MsSetMotionAttackWait(i)`.
    MsSetMotionAttackWait @ 0x007a_b500 => extern "C" fn ms_set_motion_attack_wait(i: u32) -> u32 {
        minor::ms_set_motion_attack_wait(&mut mem(), i)
    }
    /// `MsSetMotionReturn(i)`.
    MsSetMotionReturn @ 0x007a_b520 => extern "C" fn ms_set_motion_return(i: u32) -> u32 {
        minor::ms_set_motion_return(&mut mem(), i)
    }
    /// `MsDefensePosOff(actor)`.
    MsDefensePosOff @ 0x007a_f880 => extern "C" fn ms_defense_pos_off(actor: u32) -> u32 {
        minor::ms_defense_pos_off(&mut mem(), actor)
    }
    /// `MsPushBtlPosAll()`.
    MsPushBtlPosAll @ 0x007a_c680 => extern "C" fn ms_push_btl_pos_all() {
        minor::ms_push_btl_pos_all(&mut mem());
    }
    /// `MsInitParamData()`.
    MsInitParamData @ 0x0078_5960 => extern "C" fn ms_init_param_data() {
        minor::ms_init_param_data(&mut mem());
    }
    /// `MsInitMimic()`.
    MsInitMimic @ 0x0079_9ab0 => extern "C" fn ms_init_mimic() {
        minor::ms_init_mimic(&mut mem());
    }
    /// `MsInitChrAbilityMap()`.
    MsInitChrAbilityMap @ 0x0079_8820 => extern "C" fn ms_init_chr_ability_map() {
        minor::ms_init_chr_ability_map(&mut mem());
    }
    /// `MsBattleMemInit()`.
    MsBattleMemInit @ 0x0078_1d00 => extern "C" fn ms_battle_mem_init() {
        minor::ms_battle_mem_init(&mut mem());
    }
    /// `frnd()`: a float in [1, 2) on the x87 stack.
    frnd @ 0x0079_8930 => extern "C" fn frnd() -> f32 {
        minor::frnd(&mut mem())
    }
    /// `MsGetStatusThreat(i)`.
    MsGetStatusThreat @ 0x0079_afc0 => extern "C" fn ms_get_status_threat(i: u32) -> u32 {
        minor::ms_get_status_threat(&mem(), i)
    }
    /// `MsCheckChrHitMotionAll()`.
    MsCheckChrHitMotionAll @ 0x007a_f2b0 => extern "C" fn ms_check_chr_hit_motion_all() -> i32 {
        minor::ms_check_chr_hit_motion_all(&mem())
    }
    /// `MsSetPartyRelifeTime()`.
    MsSetPartyRelifeTime @ 0x0078_e0a0 => extern "C" fn ms_set_party_relife_time() {
        minor::ms_set_party_relife_time(&mut mem());
    }
    /// `MsCheckPlyBtlPosAll()`.
    MsCheckPlyBtlPosAll @ 0x0079_2170 => extern "C" fn ms_check_ply_btl_pos_all() -> u32 {
        minor::ms_check_ply_btl_pos_all(&mut mem())
    }
    /// `MsCommandStartChrStatusAll()`.
    MsCommandStartChrStatusAll @ 0x0079_3ef0 => extern "C" fn ms_command_start_chr_status_all() {
        minor::ms_command_start_chr_status_all(&mut mem());
    }
    /// `MsEffectCheck()`.
    MsEffectCheck @ 0x0078_7810 => extern "C" fn ms_effect_check() -> u32 {
        minor::ms_effect_check(&mem())
    }
    /// `MsEffectMotionLevel(id)`.
    MsEffectMotionLevel @ 0x0078_7ca0 => extern "C" fn ms_effect_motion_level(id: u32) -> i32 {
        minor::ms_effect_motion_level(&mem(), id)
    }
    /// `MsMenuStatusNumSize()`.
    MsMenuStatusNumSize @ 0x0079_0220 => extern "C" fn ms_menu_status_num_size() -> i32 {
        minor::ms_menu_status_num_size(&mem())
    }
    /// `MsProvOff(chr)`.
    MsProvOff @ 0x0078_d4e0 => extern "C" fn ms_prov_off(chr: u32) {
        minor::ms_prov_off(&mut mem(), chr);
    }
    /// `MsSaveEncountCheck()`.
    MsSaveEncountCheck @ 0x0078_5b10 => extern "C" fn ms_save_encount_check() -> u32 {
        minor::ms_save_encount_check(&mut mem())
    }
    /// `MsImportantName(id)`.
    MsImportantName @ 0x0079_0860 => extern "C" fn ms_important_name(id: u32) -> u32 {
        minor::ms_important_name(&mem(), id)
    }
    /// `MsSetSaveConfigEnglish(on)`: the language from `TkGetLang` (menu).
    MsSetSaveConfigEnglish @ 0x0078_5ed0 => extern "C" fn ms_set_save_config_english(on: u32) {
        let m = &mut mem();
        let lang = ffx_menu::tk::bridge::tk_get_lang(m);
        minor::ms_set_save_config_english(m, on, lang);
    }
    /// `MsBtlListEncount(i)`.
    MsBtlListEncount @ 0x0079_d170 => extern "C" fn ms_btl_list_encount(i: i32) -> u32 {
        minor::ms_btl_list_encount(&mem(), i)
    }
    /// `MsCheckInputCueExe(chr)`.
    MsCheckInputCueExe @ 0x007b_1fd0 => extern "C" fn ms_check_input_cue_exe(chr: u32) -> u32 {
        cue::ms_check_input_cue_exe(&mem(), chr)
    }
    /// `MsCheckCommandCueExe(chr)`.
    MsCheckCommandCueExe @ 0x007b_07d0 => extern "C" fn ms_check_command_cue_exe(chr: u32) -> u32 {
        cue::ms_check_command_cue_exe(&mem(), chr)
    }
    /// `MsCheckInputCuePos(chr)`.
    MsCheckInputCuePos @ 0x007b_2020 => extern "C" fn ms_check_input_cue_pos(chr: u32) {
        cue::ms_check_input_cue_pos(&mut mem(), chr);
    }
    /// `MsCheckTurnInputCue(chr)`.
    MsCheckTurnInputCue @ 0x007b_2060 => extern "C" fn ms_check_turn_input_cue(chr: u32) -> u32 {
        cue::ms_check_turn_input_cue(&mem(), chr)
    }
    /// `MsExeInputCue(chr, on)`.
    MsExeInputCue @ 0x007b_2290 => extern "C" fn ms_exe_input_cue(chr: u32, on: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        cue::ms_exe_input_cue(&mut mem(), chr, on as u8)
    }
    /// `MsExeComandCue(chr, on)`.
    MsExeComandCue @ 0x007b_0960 => extern "C" fn ms_exe_comand_cue(chr: u32, on: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        cue::ms_exe_comand_cue(&mut mem(), chr, on as u8)
    }
    /// `MsRegistInputCue(chr, prio, turn, flag)`.
    MsRegistInputCue @ 0x007b_2430 => extern "C" fn ms_regist_input_cue(chr: u32, prio: u32, turn: u32, flag: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_regist_input_cue(&mut mem(), chr, prio as u8, turn as u8, flag as u8)
    }
    /// `MsInsertInputCue(chr, prio, turn, flag, after, on)`.
    MsInsertInputCue @ 0x007b_2300 => extern "C" fn ms_insert_input_cue(chr: u32, prio: u32, turn: u32, flag: u32, after: u32, on: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_insert_input_cue(&mut mem(), chr, prio, turn as u8, flag, after, on as u8)
    }
    /// `MsDeleteCommandCue(chr, slot, keep)`.
    MsDeleteCommandCue @ 0x007b_0860 => extern "C" fn ms_delete_command_cue(chr: u32, slot: u32, keep: u32) -> i32 {
        cue::ms_delete_command_cue(&mut mem(), chr, slot, keep)
    }
    /// `MsGetDamageResult(chr, target, &damage, &kind, &more)`.
    MsGetDamageResult @ 0x0079_a920 => extern "C" fn ms_get_damage_result(chr: u32, target: u32, damage: u32, kind: u32, more: u32) -> i32 {
        cue::ms_get_damage_result(&mut mem(), chr, target, damage, kind, more)
    }
    /// `MsGetDamageInfo(chr, target, &a, &b, &c)`.
    MsGetDamageInfo @ 0x0079_a680 => extern "C" fn ms_get_damage_info(chr: u32, target: u32, a: u32, b: u32, c: u32) -> i32 {
        cue::ms_get_damage_info(&mut mem(), chr, target, [a, b, c])
    }
    /// `MsGetTargetList(chr, &n)`.
    MsGetTargetList @ 0x007b_7090 => extern "C" fn ms_get_target_list(chr: u32, n: u32) -> u32 {
        cue::ms_get_target_list(&mut mem(), chr, n)
    }
    /// `MsSetFirstCTB(mode)`.
    MsSetFirstCTB @ 0x0078_ded0 => extern "C" fn ms_set_first_ctb(mode: i32) -> i32 {
        turn::ms_set_first_ctb(&mut mem(), mode)
    }
    /// `MsUseItem(chr, command, n)`.
    MsUseItem @ 0x007b_0c20 => extern "C" fn ms_use_item(chr: u32, command: u32, n: i32) -> i32 {
        turn::ms_use_item(&mut mem(), chr, command, n)
    }
    /// `MsCheckStatus()`.
    MsCheckStatus @ 0x0079_a0e0 => extern "C" fn ms_check_status() {
        turn::ms_check_status(&mut mem());
    }
    /// `MsGetDamageStatus(chr, target, &added, &on, &added2, &removed, &off, &removed2)`.
    MsGetDamageStatus @ 0x0079_aa50 => extern "C" fn ms_get_damage_status(chr: u32, target: u32, p0: u32, p1: u32, p2: u32, p3: u32, p4: u32, p5: u32) -> u32 {
        turn::ms_get_damage_status(&mut mem(), chr, target, [p0, p1, p2, p3, p4, p5])
    }
    /// `MsApUp(chr, actor, ap, ret)`.
    MsApUp @ 0x0079_8a00 => extern "C" fn ms_ap_up(chr: u32, actor: u32, ap: u32, ret: u32) -> u32 {
        turn::ms_ap_up(&mut mem(), chr, actor, ap, ret)
    }
    /// `BattleRewards_AddItem(item, n, spoils)`.
    BattleRewards_AddItem @ 0x0079_8ac0 => extern "C" fn battle_rewards_add_item(item: u32, n: i32, spoils: u32) {
        turn::battle_rewards_add_item(&mut mem(), item, n, spoils);
    }
    /// `MsStatusEffectClear(i)`: `op_et_eff_off_signal` (vfx) for each.
    MsStatusEffectClear @ 0x0079_f110 => extern "C" fn ms_status_effect_clear(i: u32) -> u32 {
        flow::ms_status_effect_clear(&mut mem(), i, ffx_vfx::et::bridge::op_et_eff_off_signal)
    }
    /// `MsResetBindEffect(i)`: `op_et_bindeff_off_signal` (vfx).
    MsResetBindEffect @ 0x0078_8e60 => extern "C" fn ms_reset_bind_effect(i: u32) {
        flow::ms_reset_bind_effect(&mut mem(), i, ffx_vfx::et::bridge::op_et_bindeff_off_signal);
    }
    /// `MsResetAfterImage(actor)`: `op_et_weapon_afterimage_fade` (vfx).
    MsResetAfterImage @ 0x0078_8e30 => extern "C" fn ms_reset_after_image(actor: u32) {
        flow::ms_reset_after_image(&mut mem(), actor, ffx_vfx::et::bridge::op_et_weapon_afterimage_fade);
    }
    /// `MsEffectCheckMotion(i)`.
    MsEffectCheckMotion @ 0x0078_7840 => extern "C" fn ms_effect_check_motion(i: u32) -> i32 {
        flow::ms_effect_check_motion(&mut mem(), i)
    }
    /// `MsCheckStartEffect(chr, set)`.
    MsCheckStartEffect @ 0x007a_92c0 => extern "C" fn ms_check_start_effect(chr: u32, set: u32) -> u32 {
        flow::ms_check_start_effect(&mut mem(), chr, set)
    }
    /// `MsSetHeadTarget(chr, set)`.
    MsSetHeadTarget @ 0x007b_1e80 => extern "C" fn ms_set_head_target(chr: u32, set: u32) -> u32 {
        flow::ms_set_head_target(&mut mem(), chr, set)
    }
    /// `MsSaveAddInfoData(chr, kind)`.
    MsSaveAddInfoData @ 0x0078_5a00 => extern "C" fn ms_save_add_info_data(chr: i32, kind: u32) -> i32 {
        flow::ms_save_add_info_data(&mut mem(), chr, kind)
    }
    /// `MsLimitStatusProcess(_, actor, flags)`.
    MsLimitStatusProcess @ 0x0078_d270 => extern "C" fn ms_limit_status_process(_x: u32, actor: u32, flags: i32) -> u32 {
        flow::ms_limit_status_process(&mut mem(), actor, flags)
    }
    /// `MsMenuCheckLook()`.
    MsMenuCheckLook @ 0x0078_f9e0 => extern "C" fn ms_menu_check_look() -> i32 {
        flow::ms_menu_check_look(&mem())
    }
    /// `MsEffectCheckTarget(set)`.
    MsEffectCheckTarget @ 0x0078_7900 => extern "C" fn ms_effect_check_target(set: u32) -> u32 {
        flow::ms_effect_check_target(&mem(), set)
    }
    /// `MsReturnParty()`.
    MsReturnParty @ 0x007a_eec0 => extern "C" fn ms_return_party() {
        flow::ms_return_party(&mut mem());
    }
    /// `MsSetCameraMatrix(on, screen, camera, look_at, eye, roll)` (roll a float as bits).
    MsSetCameraMatrix @ 0x007c_06d0 => extern "C" fn ms_set_camera_matrix(on: u32, screen: u32, cam: u32, look_at: u32, eye: u32, roll: u32) -> u32 {
        camera::ms_set_camera_matrix(&mut mem(), on, screen, cam, look_at, eye, roll)
    }
    /// `MsSetChrImageDamageAttack`.
    MsSetChrImageDamageAttack @ 0x0078_d790 => extern "C" fn ms_set_chr_image_damage_attack(actor: u32) -> i32 {
        flow::ms_set_chr_image_damage_attack(&mut mem(), actor)
    }
    /// `MsSetChrHeight`.
    MsSetChrHeight @ 0x007a_b300 => extern "C" fn ms_set_chr_height(i: u32) {
        flow::ms_set_chr_height(&mut mem(), i);
    }
    /// `MsGetScreenData`.
    MsGetScreenData @ 0x007b_bd00 => extern "C" fn ms_get_screen_data(a: u32, b: u32, c: u32, d: u32) -> u32 {
        flow::ms_get_screen_data(&mut mem(), [a, b, c, d])
    }
    /// `MsBtlPosInit`.
    MsBtlPosInit @ 0x007c_93d0 => extern "C" fn ms_btl_pos_init() {
        flow::ms_btl_pos_init(&mut mem());
    }
    /// `MsAtelRegistInit`.
    MsAtelRegistInit @ 0x0079_7ae0 => extern "C" fn ms_atel_regist_init() {
        flow::ms_atel_regist_init(&mut mem());
    }
    /// `MsSetChrMotionInfo`.
    MsSetChrMotionInfo @ 0x007b_4a90 => extern "C" fn ms_set_chr_motion_info(i: u32, k: u32, value: u32) {
        flow::ms_set_chr_motion_info(&mut mem(), i, k, value);
    }
    /// `MsExchangeParty`.
    MsExchangeParty @ 0x007a_de10 => extern "C" fn ms_exchange_party() {
        flow::ms_exchange_party(&mut mem());
    }
    /// `MsChrSEfoot`.
    MsChrSEfoot @ 0x0079_fd20 => extern "C" fn ms_chr_se_foot(i: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        flow::ms_chr_se_foot(&mut mem(), i, v as u8);
    }
    /// `MsSEP3Dinit`.
    MsSEP3Dinit @ 0x007a_0910 => extern "C" fn ms_sep3d_init() {
        flow::ms_sep3d_init(&mut mem());
    }
    /// `MsEffectLimitSync`.
    MsEffectLimitSync @ 0x007a_fde0 => extern "C" fn ms_effect_limit_sync(i: u32) -> i32 {
        flow::ms_effect_limit_sync(&mem(), i)
    }
    /// `MsMenuInitText`.
    MsMenuInitText @ 0x0078_fe60 => extern "C" fn ms_menu_init_text() {
        flow::ms_menu_init_text(&mut mem());
    }
    /// `MsAddYoujinbouCom`.
    MsAddYoujinbouCom @ 0x007b_2950 => extern "C" fn ms_add_youjinbou_com(n: i32) -> i32 {
        flow::ms_add_youjinbou_com(&mut mem(), n)
    }
    /// `MsMagicEffectSet`.
    MsMagicEffectSet @ 0x0079_e890 => extern "C" fn ms_magic_effect_set(chr: u32, actor: u32) {
        flow::ms_magic_effect_set(&mut mem(), chr, actor);
    }
    /// `MsMenuCheckChrLook`.
    MsMenuCheckChrLook @ 0x0078_f990 => extern "C" fn ms_menu_check_chr_look(i: u32) -> u32 {
        flow::ms_menu_check_chr_look(&mem(), i)
    }
    /// `MsSetChrMoveFlagAll`.
    MsSetChrMoveFlagAll @ 0x0078_8e90 => extern "C" fn ms_set_chr_move_flag_all(on: u32) {
        flow::ms_set_chr_move_flag_all(&mut mem(), on);
    }
    /// `MsGetFirstAttack`.
    MsGetFirstAttack @ 0x0078_d090 => extern "C" fn ms_get_first_attack(chance: i32) -> u32 {
        flow::ms_get_first_attack(&mut mem(), chance)
    }
    /// `MsDefenseProcess`.
    MsDefenseProcess @ 0x007a_f8a0 => extern "C" fn ms_defense_process() -> u32 {
        flow::ms_defense_process(&mut mem())
    }
    /// `MsBtlChrInit`.
    MsBtlChrInit @ 0x0079_35b0 => extern "C" fn ms_btl_chr_init() {
        flow::ms_btl_chr_init(&mut mem());
    }
    /// `MsLimitTidusLearn`.
    MsLimitTidusLearn @ 0x007b_0cd0 => extern "C" fn ms_limit_tidus_learn(chr: i32) -> u32 {
        flow::ms_limit_tidus_learn(&mut mem(), chr)
    }
    /// `MsBattleCameraFree`.
    MsBattleCameraFree @ 0x007b_a950 => extern "C" fn ms_battle_camera_free() {
        flow::ms_battle_camera_free(&mut mem());
    }
    /// `MsBindOffCamera(w)`: `AtelGetCameraWorkAdrs` (script) for the worker.
    MsBindOffCamera @ 0x007b_a9c0 => extern "C" fn ms_bind_off_camera(w: u32) {
        flow::ms_bind_off_camera(&mut mem(), w, |_, w| ffx_script::vm::bridge::atel_get_camera_work_adrs(w));
    }
    /// `MsCheckStatusBeforeActionDamage`.
    MsCheckStatusBeforeActionDamage @ 0x007a_f770 => extern "C" fn ms_check_status_before_action_damage(actor: u32) -> u32 {
        flow::ms_check_status_before_action_damage(&mem(), actor)
    }
    /// `MsLimitCommandProcess(chr, actor, command, k)`.
    MsLimitCommandProcess @ 0x007b_03e0 => extern "C" fn ms_limit_command_process(_chr: u32, actor: u32, command: u32, k: u32) -> u32 {
        flow::ms_limit_command_process(&mut mem(), actor, command, k)
    }
    /// `MsSetLimitParty(chr, actor)`.
    MsSetLimitParty @ 0x007b_1aa0 => extern "C" fn ms_set_limit_party(chr: u32, actor: u32) {
        flow::ms_set_limit_party(&mut mem(), chr, actor);
    }
    /// `MsSetChrTransparent(i, alpha, frames)` (alpha a float as bits).
    MsSetChrTransparent @ 0x0079_6480 => extern "C" fn ms_set_chr_transparent(i: u32, alpha: u32, frames: i32) -> u32 {
        flow::ms_set_chr_transparent(&mut mem(), i, alpha, frames)
    }
    /// `MsEffectSetMotionFlag(i)`.
    MsEffectSetMotionFlag @ 0x0078_83f0 => extern "C" fn ms_effect_set_motion_flag(i: u32) {
        flow::ms_effect_set_motion_flag(&mut mem(), i);
    }
    /// `MsGetTargetRandomList2(i, &n)`.
    MsGetTargetRandomList2 @ 0x007b_7280 => extern "C" fn ms_get_target_random_list2(i: u32, n: u32) -> u32 {
        flow::ms_get_target_random_list2(&mut mem(), i, n)
    }
    /// `MsAtelCamFree()`: `AtelRemoveCtrlWork` (script).
    MsAtelCamFree @ 0x0079_74e0 => extern "C" fn ms_atel_cam_free() {
        flow::ms_atel_cam_free(&mut mem(), ffx_script::vm::bridge::atel_remove_ctrl_work);
    }
    /// `MsGetMagicEffNum(chr, id, command)`: `TkGetLang` (menu).
    MsGetMagicEffNum @ 0x0079_d2f0 => extern "C" fn ms_get_magic_eff_num(chr: i32, id: u32, command: u32) -> u32 {
        flow::ms_get_magic_eff_num(&mut mem(), chr, id, command, |m| ffx_menu::tk::bridge::tk_get_lang(m))
    }
    /// `MsBtlGetPos(write, actor, group, kind, index, vec)`: kinds 12-14
    /// call the original `MsGetSummonerPos`.
    MsBtlGetPos @ 0x007a_bfe0 => extern "C" fn ms_btl_get_pos(write: u32, actor: u32, group: i32, kind: i32, index: i32, vec: u32) -> i32 {
        pos::ms_btl_get_pos(&mut mem(), write, actor, group, kind, index, vec, |_, actor, index, k, vec| {
            call4(MS_GET_SUMMONER_POS, actor, index.cast_unsigned(), k.cast_unsigned(), vec);
        })
    }
    /// `MsRegistCommandCue(chr, command, turn, kind, flag)`.
    MsRegistCommandCue @ 0x007b_0b90 => extern "C" fn ms_regist_command_cue(chr: u32, command: u32, turn: u32, kind: u32, flag: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_regist_command_cue(&mut mem(), chr, command, turn as u8, kind as u8, flag as u8)
    }
    /// `MsInsertCommandCue(chr, command, turn, kind, flag, on, after)`.
    MsInsertCommandCue @ 0x007b_0a30 => extern "C" fn ms_insert_command_cue(chr: u32, command: u32, turn: u32, kind: u32, flag: u32, on: u32, after: u32) -> i32 {
        #[allow(clippy::cast_possible_truncation)] // byte arguments
        cue::ms_insert_command_cue(&mut mem(), chr, command, turn as u8, kind, flag as u8, on as u8, after)
    }
    /// `MsCleanCommandCue(chr, keep)`.
    MsCleanCommandCue @ 0x007b_0830 => extern "C" fn ms_clean_command_cue(chr: u32, keep: u32) -> i32 {
        cue::ms_clean_command_cue(&mut mem(), chr, keep)
    }
    /// `MsBtlListGroup(field, group)`.
    MsBtlListGroup @ 0x0079_d210 => extern "C" fn ms_btl_list_group(field: i32, group: i32) -> u32 {
        flow::ms_btl_list_group(&mem(), field, group)
    }
    /// `MsGetChrEscapePos(i, vec)`.
    MsGetChrEscapePos @ 0x0079_4ec0 => extern "C" fn ms_get_chr_escape_pos(i: u32, vec: u32) -> i32 {
        flow::ms_get_chr_escape_pos(&mut mem(), i, vec, |_, actor, index, k, vec| {
            call4(MS_GET_SUMMONER_POS, actor, index.cast_unsigned(), k.cast_unsigned(), vec);
        })
    }
    /// `MsUseChrMpLimit(i)`.
    MsUseChrMpLimit @ 0x0078_e5a0 => extern "C" fn ms_use_chr_mp_limit(i: u32) {
        flow::ms_use_chr_mp_limit(&mut mem(), i);
    }
    /// `MsBtlGetTerm()`.
    MsBtlGetTerm @ 0x0079_8b70 => extern "C" fn ms_btl_get_term() {
        flow::ms_btl_get_term(&mut mem());
    }
    /// `MsClearStatus(i, keep)`.
    MsClearStatus @ 0x0079_a190 => extern "C" fn ms_clear_status(i: u32, keep: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        flow::ms_clear_status(&mut mem(), i, keep as u16);
    }
    /// `MsMoveCheckFlag(actor)`.
    MsMoveCheckFlag @ 0x007a_aae0 => extern "C" fn ms_move_check_flag(actor: u32) -> u32 {
        flow::ms_move_check_flag(&mem(), actor)
    }
    /// `MsTestCheckMotion(i)`: `MsEffectCheckMotion`.
    MsTestCheckMotion @ 0x0079_0260 => extern "C" fn ms_test_check_motion(i: u32) -> i32 {
        flow::ms_effect_check_motion(&mut mem(), i)
    }
    /// `MsLimitUp(chr, actor, n)`.
    MsLimitUp @ 0x007b_1590 => extern "C" fn ms_limit_up(chr: u32, actor: u32, n: i32) -> i32 {
        turn::ms_limit_up(&mut mem(), chr, actor, n)
    }
    /// `MsCameraGet(owner, cam, kind)`.
    MsCameraGet @ 0x007b_d110 => extern "C" fn ms_camera_get(owner: u32, cam: u32, kind: u32) -> u32 {
        camera::ms_camera_get(&mut mem(), owner, cam, kind)
    }
}
