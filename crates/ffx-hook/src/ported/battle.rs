//! `ffx-battle`: random numbers, the save block, actors, the battle camera,
//! battle globals and helpers, damage.

use ffx_battle::{actor::bridge as actor, camera::bridge as camera, command::bridge as cmd};
use ffx_battle::{damage::bridge as dmg, state::bridge as state, util::bridge as util};
use ffx_battle::{save::bridge as save, vector::bridge as vector};

use super::{call0, mem, ported};

/// `TOGetFFXLang` (system), which `DmgCalc_Bribe` asks.
const TO_GET_FFX_LANG: u32 = 0x0064_10e0;
/// `TOBtlCloseSignWindow` (system), which the message cue calls.
const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;

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
}
