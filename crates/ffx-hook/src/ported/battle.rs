//! `ffx-battle`: random numbers, the save block, actors, the battle camera,
//! battle globals and helpers.

use ffx_battle::{actor::bridge as actor, camera::bridge as camera, save::bridge as save};
use ffx_battle::{state::bridge as state, util::bridge as util};

use super::{call0, mem, ported};

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
}
