//! M5 stage 1, list `battle` (`ffx_battle::m5_battle`): the battle's
//! debug helpers (full items and party, weak enemies, the scene picker,
//! the environment file), the PS2 CD layer the movie player keeps, and
//! a few jump thunks.
//!
//! Threads: the main thread (the battle, the debug menu, the movie
//! player's reads); the shims keep no state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_battle::m5_battle::bridge as mb;

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
}
