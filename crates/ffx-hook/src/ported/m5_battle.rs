//! M5 stage 1, list `battle` (`ffx_battle::m5_battle`): the battle's
//! debug helpers (full items and party, weak enemies, the scene picker,
//! the environment file), the PS2 CD layer the movie player keeps, and
//! a few jump thunks.
//!
//! Threads: the main thread (the battle, the debug menu, the movie
//! player's reads); the shims keep no state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_battle::m5_battle::bridge as mb;
use ffx_menu::m5m_m::bridge as mm;

/// Calls the game's cdecl code at a run-time address (`mb::Call`).
fn call<M>(_: &mut M, f: u32, args: &[u32]) -> u32 {
    super::call_cdecl(f, args)
}

ported! {
    // ---- jump thunks ----
    /// `FFXGetAsiaMode_007826a0()`: `jmp FFXGetAsiaMode`.
    FFXGetAsiaMode_007826a0 @ 0x0078_26a0 => extern "C" fn asia_mode_thunk() -> u32 {
        mb::jmp_thunk(&mut mem(), &mut call, mb::FFX_GET_ASIA_MODE, &[])
    }
    /// `MsEtEffectRead_00782740()`: `jmp MsEtEffectRead`.
    MsEtEffectRead_00782740 @ 0x0078_2740 => extern "C" fn et_effect_read_thunk() -> u32 {
        mb::jmp_thunk(&mut mem(), &mut call, mb::MS_ET_EFFECT_READ, &[])
    }
    /// `MsSetChrPos_thunk(i, pos)` (a magic DLL import).
    MsSetChrPos_thunk @ 0x0092_e980 => extern "C" fn set_chr_pos_thunk(i: u32, pos: u32) -> u32 {
        mb::jmp_thunk(&mut mem(), &mut call, mb::MS_SET_CHR_POS, &[i, pos])
    }
    /// `MsSetChrVisible_thunk(i, v)` (a magic DLL import).
    MsSetChrVisible_thunk @ 0x0092_e990 => extern "C" fn set_chr_visible_thunk(i: u32, v: u32) -> u32 {
        mb::jmp_thunk(&mut mem(), &mut call, mb::MS_SET_CHR_VISIBLE, &[i, v])
    }
    /// `MsEffectSetMotion_thunk(i, motion)` (a magic DLL import).
    MsEffectSetMotion_thunk @ 0x0092_eb10 => extern "C" fn effect_set_motion_thunk(i: u32, motion: u32) -> u32 {
        mb::jmp_thunk(&mut mem(), &mut call, mb::MS_EFFECT_SET_MOTION, &[i, motion])
    }

    // ---- the debug menu's helpers ----
    /// `MsDebugFullItem()`: its tail call's `eax`.
    MsDebugFullItem @ 0x0078_49d0 => extern "C" fn ms_debug_full_item() -> u32 {
        mb::ms_debug_full_item(&mut mem(), &mut call)
    }
    /// `MsEnvInit()`: the file name in a 0x84-byte frame buffer.
    MsEnvInit @ 0x0078_2770 => extern "C" fn ms_env_init() {
        let mut f = StackFrame::<0x21>::new();
        mb::ms_env_init(&mut mem(), &mut call, f.at());
    }
    /// `MsDebugFullMember()`: `MsSaveEncountCheck`'s `eax`.
    MsDebugFullMember @ 0x0078_4ac0 => extern "C" fn ms_debug_full_member() -> u32 {
        mb::ms_debug_full_member(&mut mem(), &mut call)
    }
    /// `MsDebugFullStats_00784b00()`.
    MsDebugFullStats_00784b00 @ 0x0078_4b00 => extern "C" fn ms_debug_full_stats() {
        mb::ms_debug_full_stats(&mut mem());
    }
    /// `MsDebugWeakAll_00784b80()`.
    MsDebugWeakAll_00784b80 @ 0x0078_4b80 => extern "C" fn ms_debug_weak_all() {
        mb::ms_debug_weak_all(&mut mem(), &mut call);
    }
    /// `MsDebugWeakMonsters_00784bc0(on)`.
    MsDebugWeakMonsters_00784bc0 @ 0x0078_4bc0 => extern "C" fn ms_debug_weak_monsters(on: u32) {
        mb::ms_debug_weak_monsters(&mut mem(), &mut call, on);
    }
    /// `MsDebugAllItemsOne_00784c20()`.
    MsDebugAllItemsOne_00784c20 @ 0x0078_4c20 => extern "C" fn ms_debug_all_items_one() {
        mb::ms_debug_all_items_one(&mut mem());
    }
    /// `MsDebugAbilityOne_00784c40()`.
    MsDebugAbilityOne_00784c40 @ 0x0078_4c40 => extern "C" fn ms_debug_ability_one() {
        mb::ms_debug_ability_one(&mut mem());
    }
    /// `MsDebugWeakParty_00784c80(on)`.
    MsDebugWeakParty_00784c80 @ 0x0078_4c80 => extern "C" fn ms_debug_weak_party(on: u32) {
        mb::ms_debug_weak_party(&mut mem(), &mut call, on);
    }
    /// `MsDebugClearItems_00784cd0()`.
    MsDebugClearItems_00784cd0 @ 0x0078_4cd0 => extern "C" fn ms_debug_clear_items() {
        mb::ms_debug_clear_items(&mut mem());
    }
    /// `MsCheckChrBtlPosAll()`.
    MsCheckChrBtlPosAll @ 0x0079_15f0 => extern "C" fn ms_check_chr_btl_pos_all() -> u32 {
        mb::ms_check_chr_btl_pos_all(&mut mem(), &mut call)
    }
    /// `MsSEPtransCheckWave()`.
    MsSEPtransCheckWave @ 0x007a_04d0 => extern "C" fn ms_sep_trans_check_wave() -> u32 {
        mb::ms_sep_trans_check_wave(&mem())
    }
    /// `MsPickGroundGroup_007abf40(group)`.
    MsPickGroundGroup_007abf40 @ 0x007a_bf40 => extern "C" fn ms_pick_ground_group(group: u32) {
        mb::ms_pick_ground_group(&mut mem(), &mut call, group);
    }
    /// `MsAtelFreeCameraID(kind, slot)`.
    MsAtelFreeCameraID @ 0x007b_a6d0 => extern "C" fn ms_atel_free_camera_id(kind: u32, slot: u32) {
        mb::ms_atel_free_camera_id(&mut mem(), &mut call, kind, slot);
    }
    /// `MsCameraGetAngle(yaw, pitch, len)`.
    MsCameraGetAngle @ 0x007b_d2d0 => extern "C" fn ms_camera_get_angle(yaw: u32, pitch: u32, len: u32) {
        mb::ms_camera_get_angle(&mut mem(), &mut call, yaw, pitch, len);
    }
    /// `MsDebugSceneWalk_007c5dd0()`.
    MsDebugSceneWalk_007c5dd0 @ 0x007c_5dd0 => extern "C" fn ms_debug_scene_walk() {
        mb::ms_debug_scene_walk(&mut mem(), &mut call);
    }
    /// `MsDebugSceneGroupBase_007c5f00()`: the encounter index.
    MsDebugSceneGroupBase_007c5f00 @ 0x007c_5f00 => extern "C" fn ms_debug_scene_group_base() -> u32 {
        mb::ms_debug_scene_group_base(&mut mem(), &mut call)
    }
    /// `MsDebugSetScene(field, group, in_group, scene_field, formation)`.
    MsDebugSetScene @ 0x007c_5f60 => extern "C" fn ms_debug_set_scene(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mb::ms_debug_set_scene(&mut mem(), &mut call, [a, b, c, d, e])
    }
    /// `MsGetBattleName(full, out)`: `out`.
    MsGetBattleName @ 0x007c_5fc0 => extern "C" fn ms_get_battle_name(full: u32, out: u32) -> u32 {
        mb::ms_get_battle_name(&mut mem(), &mut call, full, out)
    }
    /// `MsDebugNextMember_007c60a0(slot, step)`.
    MsDebugNextMember_007c60a0 @ 0x007c_60a0 => extern "C" fn ms_debug_next_member(slot: u32, step: u32) -> u32 {
        mb::ms_debug_next_member(&mut mem(), &mut call, slot, step)
    }
    /// `MsEnvWriteText_007ce9f0(writer, text)`: the chunk length in the
    /// `text` slot.
    MsEnvWriteText_007ce9f0 @ 0x007c_e9f0 => extern "C" fn ms_env_write_text(writer: u32, text: u32; slots: u32) {
        mb::ms_env_write_text(&mut mem(), &mut call, slots, writer, text);
    }
    /// `MsEnvFindKey_007cf140(table, key)`.
    MsEnvFindKey_007cf140 @ 0x007c_f140 => extern "C" fn ms_env_find_key(table: u32, key: u32) -> u32 {
        mb::ms_env_find_key(&mem(), table, key)
    }

    // ---- the CD layer and the password file ----
    /// `CdDiscWait_00776f60(ctx)`.
    CdDiscWait_00776f60 @ 0x0077_6f60 => extern "C" fn cd_disc_wait(ctx: u32) -> u32 {
        mb::cd_disc_wait(&mut mem(), &mut call, ctx)
    }
    /// `CdOpenCheck_sub(sync)`.
    CdOpenCheck_sub @ 0x0077_7000 => extern "C" fn cd_open_check_sub(sync: u32) -> u32 {
        mb::cd_open_check_sub(&mut mem(), &mut call, sync)
    }
    /// `CdReadSectors_00777080(sector, count, buf, mode, ctx)`: the
    /// advanced sector in the caller's first argument slot.
    CdReadSectors_00777080 @ 0x0077_7080 => extern "C" fn cd_read_sectors(sector: u32, count: u32, buf: u32, mode: u32, ctx: u32; slots: u32) -> u32 {
        mb::cd_read_sectors(&mut mem(), &mut call, slots, [sector, count, buf, mode, ctx])
    }
    /// `PasswordFileRead_00790430(name, dst)`: the size.
    PasswordFileRead_00790430 @ 0x0079_0430 => extern "C" fn password_file_read(name: u32, dst: u32) -> u32 {
        mb::password_file_read(&mut mem(), &mut call, name, dst)
    }

    // ---- batch 2: the debug status, files, the position script reader ----
    /// `MsBattleProcess_DebugStatus_00792f20()`: a 0x40-byte frame buffer.
    MsBattleProcess_DebugStatus_00792f20 @ 0x0079_2f20 => extern "C" fn ms_battle_debug_status() {
        let mut f = StackFrame::<0x10>::new();
        mb::ms_battle_debug_status(&mut mem(), &mut call, f.at());
    }
    /// `MsCheckFile(name)`: the rounded size, -1.
    MsCheckFile @ 0x007c_8a80 => extern "C" fn ms_check_file(name: u32) -> u32 {
        mb::ms_check_file(&mut mem(), &mut call, name)
    }
    /// `MsFileIn(name, buf, n)`.
    MsFileIn @ 0x007c_8b00 => extern "C" fn ms_file_in(name: u32, buf: u32, n: u32) {
        mb::ms_file_io(&mut mem(), &mut call, false, name, buf, n);
    }
    /// `MsFileOut(name, buf, n)`.
    MsFileOut @ 0x007c_8b50 => extern "C" fn ms_file_out(name: u32, buf: u32, n: u32) {
        mb::ms_file_io(&mut mem(), &mut call, true, name, buf, n);
    }
    /// `MsDebugWinAlloc_007c8bb0(slot)`: the record.
    MsDebugWinAlloc_007c8bb0 @ 0x007c_8bb0 => extern "C" fn ms_debug_win_alloc(slot: u32) -> u32 {
        mb::ms_debug_win_alloc(&mut mem(), slot)
    }
    /// `ms_malloc_007cf970(size)`: a 0x100-byte frame buffer.
    ms_malloc_007cf970 @ 0x007c_f970 => extern "C" fn ms_malloc(size: u32) -> u32 {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_malloc(&mut mem(), &mut call, f.at(), size)
    }
    /// `MsDebugBattlePosInit(out, a, b)`: a 0x100-byte frame buffer.
    MsDebugBattlePosInit @ 0x007c_90f0 => extern "C" fn ms_debug_battle_pos_init(out: u32, a: u32, b: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_debug_battle_pos_init(&mut mem(), &mut call, f.at(), [out, a, b]);
    }
    /// `ms_free(p)`: a 0x100-byte frame buffer.
    ms_free @ 0x007c_f8f0 => extern "C" fn ms_free(p: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_free(&mut mem(), &mut call, f.at(), p);
    }
    /// `MsBtlPosFree_007c9210()`.
    MsBtlPosFree_007c9210 @ 0x007c_9210 => extern "C" fn ms_btl_pos_free() {
        mb::ms_btl_pos_free(&mut mem(), &mut call);
    }
    /// `MsEnvWriteText_007cd680(writer, text)`: a copy of `0x007ce9f0`.
    MsEnvWriteText_007cd680 @ 0x007c_d680 => extern "C" fn ms_env_write_text_2(writer: u32, text: u32; slots: u32) {
        mb::ms_env_write_text(&mut mem(), &mut call, slots, writer, text);
    }
    /// `MsBtlPosCheckVersion_007cc1f0(name, version)`.
    MsBtlPosCheckVersion_007cc1f0 @ 0x007c_c1f0 => extern "C" fn ms_btl_pos_check_version(name: u32, version: u32) -> u32 {
        mb::ms_btl_pos_check_version(&mut mem(), &mut call, name, version)
    }
    /// `MsBtlPosCheckVersion2_007cc250(name, version, sub)`.
    MsBtlPosCheckVersion2_007cc250 @ 0x007c_c250 => extern "C" fn ms_btl_pos_check_version2(name: u32, version: u32, sub: u32) -> u32 {
        mb::ms_btl_pos_check_version2(&mut mem(), &mut call, name, version, sub)
    }
    /// `MsReadSkipSpace_007cf540(reader)`.
    MsReadSkipSpace_007cf540 @ 0x007c_f540 => extern "C" fn ms_read_skip_space(reader: u32) -> u32 {
        mb::ms_read_skip_space(&mut mem(), reader)
    }
    /// `MsReadToken_007cf180(reader, out, size)`: the quote flag in the
    /// `reader` slot.
    MsReadToken_007cf180 @ 0x007c_f180 => extern "C" fn ms_read_token(reader: u32, out: u32, size: u32; slots: u32) -> u32 {
        mb::ms_read_token(&mut mem(), &mut call, slots, reader, out, size)
    }
    /// `MsReadFloat_007cd360(reader, name, out)`: a 0x100-byte frame buffer.
    MsReadFloat_007cd360 @ 0x007c_d360 => extern "C" fn ms_read_float(reader: u32, name: u32, out: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_read_value(&mut mem(), &mut call, f.at(), true, [reader, name, out]);
    }
    /// `MsReadInt_007cd420(reader, name, out)`: a 0x100-byte frame buffer.
    MsReadInt_007cd420 @ 0x007c_d420 => extern "C" fn ms_read_int(reader: u32, name: u32, out: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_read_value(&mut mem(), &mut call, f.at(), false, [reader, name, out]);
    }
    /// `MsBtlPosAllocPoints_007cd4e0()`.
    MsBtlPosAllocPoints_007cd4e0 @ 0x007c_d4e0 => extern "C" fn ms_btl_pos_alloc_points() {
        mb::ms_btl_pos_alloc_points(&mut mem(), &mut call);
    }
    /// `MsKeywordLookup_007ceeb0(name, table)`.
    MsKeywordLookup_007ceeb0 @ 0x007c_eeb0 => extern "C" fn ms_keyword_lookup(name: u32, table: u32) -> u32 {
        mb::ms_keyword_lookup(&mem(), name, table)
    }
    /// `MsReadSkipLine_007cf380(reader, line)`.
    MsReadSkipLine_007cf380 @ 0x007c_f380 => extern "C" fn ms_read_skip_line(reader: u32, line: u32) -> u32 {
        mb::ms_read_skip_line(&mut mem(), reader, line)
    }
    /// `MsReadOpen_007cd5a0(reader, name)`.
    MsReadOpen_007cd5a0 @ 0x007c_d5a0 => extern "C" fn ms_read_open(reader: u32, name: u32) {
        mb::ms_read_open(&mut mem(), &mut call, reader, name);
    }
    /// `MsPathBaseName_007cefb0(path, ext)`.
    MsPathBaseName_007cefb0 @ 0x007c_efb0 => extern "C" fn ms_path_base_name(path: u32, ext: u32) -> u32 {
        mb::ms_path_base_name(&mut mem(), path, ext)
    }
    /// `MsStrEqualNoCase_007cf330(a, b)`.
    MsStrEqualNoCase_007cf330 @ 0x007c_f330 => extern "C" fn ms_str_equal_no_case(a: u32, b: u32) -> u32 {
        mb::ms_str_equal_no_case(&mem(), a, b)
    }
    /// `MsPathSetExt_007cf590(path, src, n, force)`: the extension index
    /// in the `path` slot.
    MsPathSetExt_007cf590 @ 0x007c_f590 => extern "C" fn ms_path_set_ext(path: u32, src: u32, n: u32, force: u32; slots: u32) -> u32 {
        mb::ms_path_set_ext(&mut mem(), &mut call, slots, [path, src, n, force])
    }
    /// `MsCameraKindChar_007cfaa0(kind)`: the letter in `al`.
    MsCameraKindChar_007cfaa0 @ 0x007c_faa0 => extern "C" fn ms_camera_kind_char(kind: u32) -> u32 {
        mb::ms_camera_kind_char(&mem(), kind)
    }
    /// `MsDebugSetCameraMode(side, mode)`.
    MsDebugSetCameraMode @ 0x007c_fb60 => extern "C" fn ms_debug_set_camera_mode(side: u32, mode: u32) {
        mb::ms_debug_set_camera_mode(&mut mem(), &mut call, side, mode);
    }
    /// `MsInfoWinSet_007cfba0(i, mark, pos)`.
    MsInfoWinSet_007cfba0 @ 0x007c_fba0 => extern "C" fn ms_info_win_set(i: u32, mark: u32, pos: u32) {
        mb::ms_info_win_set(&mut mem(), i, mark, pos);
    }
    /// `MsInfoWinsInit_007cfca0()`.
    MsInfoWinsInit_007cfca0 @ 0x007c_fca0 => extern "C" fn ms_info_wins_init() {
        mb::ms_info_wins_init(&mut mem(), &mut call);
    }

    // ---- batch 3: the camera and battle debug screens' helpers ----
    /// `MsEnvWriteText_007de700(writer, text)`: a copy of `0x007ce9f0`.
    MsEnvWriteText_007de700 @ 0x007d_e700 => extern "C" fn ms_env_write_text_3(writer: u32, text: u32; slots: u32) {
        mb::ms_env_write_text(&mut mem(), &mut call, slots, writer, text);
    }
    /// `MsBtlPosEditFree_007d7450()`: then `MsBtlPosFree_007c9210`.
    MsBtlPosEditFree_007d7450 @ 0x007d_7450 => extern "C" fn ms_btl_pos_edit_free() {
        mb::ms_btl_pos_edit_free(&mut mem(), &mut call);
    }
    /// `MsDebugIsTextMode_007d79e0(kind)`.
    MsDebugIsTextMode_007d79e0 @ 0x007d_79e0 => extern "C" fn ms_debug_is_text_mode(kind: u32) -> u32 {
        mb::ms_debug_is_text_mode(kind)
    }
    /// `MsDebugScenePath_007d70d0()`.
    MsDebugScenePath_007d70d0 @ 0x007d_70d0 => extern "C" fn ms_debug_scene_path() {
        mb::ms_debug_scene_path(&mut mem(), &mut call);
    }
    /// `MsDebugPlaceAll_007d73e0()`: the position in a 3-float frame.
    MsDebugPlaceAll_007d73e0 @ 0x007d_73e0 => extern "C" fn ms_debug_place_all() {
        let mut f = StackFrame::<3>::new();
        mb::ms_debug_place_all(&mut mem(), &mut call, f.at());
    }
    /// `MsDebugFindName_007d2f10(name, list)`.
    MsDebugFindName_007d2f10 @ 0x007d_2f10 => extern "C" fn ms_debug_find_name(name: u32, list: u32) -> u32 {
        mb::ms_debug_find_name(&mut mem(), &mut call, name, list)
    }
    /// `MsDebugRefFromCamera_007d36b0(cam)`.
    MsDebugRefFromCamera_007d36b0 @ 0x007d_36b0 => extern "C" fn ms_debug_ref_from_camera(cam: u32) {
        mb::ms_debug_ref_from_camera(&mut mem(), cam);
    }
    /// `MsDebugRefSet_007d3420(value, point[3], dir[3])`: the value in
    /// ST0, the clamped value in the `value` slot.
    MsDebugRefSet_007d3420 @ 0x007d_3420 => extern "C" fn ms_debug_ref_set(v: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32; slots: u32) -> f32 {
        f32::from_bits(mb::ms_debug_ref_set(&mut mem(), slots, [v, a, b, c, d, e, f]))
    }
    /// `MsDebugRefAdd_007d2f50(delta)`: the value in ST0.
    MsDebugRefAdd_007d2f50 @ 0x007d_2f50 => extern "C" fn ms_debug_ref_add(delta: u32) -> f32 {
        f32::from_bits(mb::ms_debug_ref_add(&mut mem(), delta))
    }
    /// `MsDebugRefRound_007d23d0()`.
    MsDebugRefRound_007d23d0 @ 0x007d_23d0 => extern "C" fn ms_debug_ref_round() {
        mb::ms_debug_ref_round(&mut mem());
    }
    /// `MsSetFileName(out, n)`.
    MsSetFileName @ 0x007d_ce80 => extern "C" fn ms_set_file_name(out: u32, n: u32) {
        mb::ms_set_file_name(&mut mem(), &mut call, out, n);
    }
    /// `MsGetPlySaveName(out, name)`.
    MsGetPlySaveName @ 0x007d_cec0 => extern "C" fn ms_get_ply_save_name(out: u32, name: u32) {
        mb::ms_get_ply_save_name(&mut mem(), &mut call, out, name);
    }
    /// `MsDebugActorCount_007d9ab0(id, kind)`.
    MsDebugActorCount_007d9ab0 @ 0x007d_9ab0 => extern "C" fn ms_debug_actor_count(id: u32, kind: u32) -> u32 {
        mb::ms_debug_actor_count(&mut mem(), &mut call, id, kind)
    }
    /// `MsDebugPathAfterBase_007d12a0(path, base)`.
    MsDebugPathAfterBase_007d12a0 @ 0x007d_12a0 => extern "C" fn ms_debug_path_after_base(path: u32, base: u32) -> u32 {
        mb::ms_debug_path_after_base(&mem(), path, base)
    }
    /// `MsDebugCameraCopy_007d1220()`.
    MsDebugCameraCopy_007d1220 @ 0x007d_1220 => extern "C" fn ms_debug_camera_copy() {
        mb::ms_debug_camera_copy(&mut mem(), &mut call);
    }
    /// `MsBtlPosParse_007cd0f0(reader)`: a 0x100-byte frame buffer.
    MsBtlPosParse_007cd0f0 @ 0x007c_d0f0 => extern "C" fn ms_btl_pos_parse(reader: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_btl_pos_parse(&mut mem(), &mut call, f.at(), reader);
    }
    /// `MsBtlPosLoad_007ca4a0(name)`: the reader in a 5-word frame.
    MsBtlPosLoad_007ca4a0 @ 0x007c_a4a0 => extern "C" fn ms_btl_pos_load(name: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        mb::ms_btl_pos_load(&mut mem(), &mut call, f.at(), name)
    }

    // ---- batch 4: the position screens, the monster motion file ----
    /// `MsBtlPosWriteAll_007c9bc0(out)`: `MsBtlPosWriteBin`'s `eax`.
    MsBtlPosWriteAll_007c9bc0 @ 0x007c_9bc0 => extern "C" fn ms_btl_pos_write_all(out: u32) -> u32 {
        mb::ms_btl_pos_write_all(&mut mem(), &mut call, out)
    }
    /// `MsDebugPosGroup_007d72e0()`: the group record or 0.
    MsDebugPosGroup_007d72e0 @ 0x007d_72e0 => extern "C" fn ms_debug_pos_group() -> u32 {
        mb::ms_debug_pos_group(&mut mem())
    }
    /// `MsDebugPosNextKind_007d92d0(step)`.
    MsDebugPosNextKind_007d92d0 @ 0x007d_92d0 => extern "C" fn ms_debug_pos_next_kind(step: u32) {
        mb::ms_debug_pos_next_kind(&mut mem(), step);
    }
    /// `MsDebugPosWindows_007d7a40(all, group, kind, start, r, g, b, mark)`:
    /// the next slot.
    MsDebugPosWindows_007d7a40 @ 0x007d_7a40 => extern "C" fn ms_debug_pos_windows(all: u32, group: u32, kind: u32, start: u32, r: u32, g: u32, b: u32, mark: u32) -> u32 {
        mb::ms_debug_pos_windows(&mut mem(), &mut call, [all, group, kind, start, r, g, b, mark])
    }
    /// `MsMonSave(name, monster, n)`: the writer and text in a frame
    /// (5 words, then 0x100 bytes).
    MsMonSave @ 0x007d_d480 => extern "C" fn ms_mon_save(name: u32, monster: u32, n: u32) -> u32 {
        let mut f = StackFrame::<0x45>::new();
        mb::ms_mon_save(&mut mem(), &mut call, f.at(), [name, monster, n])
    }
    /// `MsDebugPrintRules_007d10c0(y)`: a 0x100-byte frame buffer.
    MsDebugPrintRules_007d10c0 @ 0x007d_10c0 => extern "C" fn ms_debug_print_rules(y: u32) -> u32 {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_debug_print_rules(&mut mem(), &mut call, f.at(), y)
    }

    // ---- batch 5: actors by place ----
    /// `MsDebugWidthAt_007d93f0(a, b, c, index)`: the width in ST0.
    MsDebugWidthAt_007d93f0 @ 0x007d_93f0 => extern "C" fn ms_debug_width_at(a: u32, b: u32, c: u32, index: u32) -> f32 {
        f32::from_bits(mb::ms_debug_width_at(&mut mem(), &mut call, [a, b, c, index]))
    }
    /// `MsDebugDirAt_007d9470(a, b, c)`: the direction in ST0.
    MsDebugDirAt_007d9470 @ 0x007d_9470 => extern "C" fn ms_debug_dir_at(a: u32, b: u32, c: u32) -> f32 {
        f32::from_bits(mb::ms_debug_dir_at(&mut mem(), &mut call, [a, b, c]))
    }
    /// `MsDebugSetDirAt_007d9550(a, b, c, deg)`.
    MsDebugSetDirAt_007d9550 @ 0x007d_9550 => extern "C" fn ms_debug_set_dir_at(a: u32, b: u32, c: u32, deg: u32) {
        mb::ms_debug_set_dir_at(&mut mem(), &mut call, [a, b, c, deg]);
    }

    // ---- batch 6 ----
    /// `MsReadVec3_007cd170(reader, name, x, y, z)`: a 0x100-byte frame buffer.
    MsReadVec3_007cd170 @ 0x007c_d170 => extern "C" fn ms_read_vec3(reader: u32, name: u32, x: u32, y: u32, z: u32) {
        let mut f = StackFrame::<0x40>::new();
        mb::ms_read_vec3(&mut mem(), &mut call, f.at(), [reader, name, x, y, z]);
    }
    /// `MsDebugCameraKeep_007c8c50(step)`: `cam * 0xda0` in the `step`
    /// slot.
    MsDebugCameraKeep_007c8c50 @ 0x007c_8c50 => extern "C" fn ms_debug_camera_keep(step: u32; slots: u32) -> u32 {
        mb::ms_debug_camera_keep(&mut mem(), &mut call, slots, step)
    }

    // ---- batch 7 ----
    /// `MsPlySave(name)`: the writer and text in a frame (5 words, then
    /// 0x400 bytes).
    MsPlySave @ 0x007d_d5e0 => extern "C" fn ms_ply_save(name: u32) -> u32 {
        let mut f = StackFrame::<0x105>::new();
        mb::ms_ply_save(&mut mem(), &mut call, f.at(), name)
    }
    // ---- round 4 (menu-battle-script 2) (ffx_menu::m5m_m) ----
    /// `()`: the address `0x00b546a0`.
    BtlRdata_007829a0 @ 0x0078_29a0 => extern "C" fn btl_rdata_007829a0() -> u32 {
        mm::rdata_address(&mem(), 0x00b5_46a0)
    }
    /// `MsBtlReadFileDone_00782fe0()`.
    MsBtlReadFileDone_00782fe0 @ 0x0078_2fe0 => extern "C" fn ms_btl_read_file_done() {
        mm::btl_read_file_done(&mut mem());
    }
    /// `MsBtlReadAsyncTask_00784500()`.
    MsBtlReadAsyncTask_00784500 @ 0x0078_4500 => extern "C" fn ms_btl_read_async_task() -> u32 {
        mm::btl_read_async(&mut mem())
    }
    /// `()`: `Native_70a4`'s body.
    ChnSystemMgrpReset_0079cca0 @ 0x0079_cca0 => extern "C" fn chn_system_mgrp_reset() {
        mm::chn_system_mgrp_reset(&mut mem());
    }
}
