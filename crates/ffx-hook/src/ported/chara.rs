//! `ffx-chara`: finding characters and their accessors. All cdecl, the
//! character first; floats are `f32` on the stack and in `st0`.

use ffx_chara::chr::bridge as c;
use ffx_chara::handle::bridge as h;
use ffx_chara::layout as l;

use super::{mem, ported};

ported! {
    // ---- finding characters

    /// `Ch_BlkGetHandleFast(key)`: the model block whose first word is `key`.
    Ch_BlkGetHandleFast @ 0x0082_5ea0 => extern "C" fn ch_blk_get_handle_fast(key: u32) -> u32 {
        h::ch_blk_get_handle_fast(&mem(), key)
    }
    /// `Ch_CheckHandle(chr)`: 0 for an active character, else -1.
    Ch_CheckHandle @ 0x0082_61b0 => extern "C" fn ch_check_handle(chr: u32) -> i32 {
        h::ch_check_handle(&mem(), chr)
    }
    /// `Ch_FunctionHistoryInit()`.
    Ch_FunctionHistoryInit @ 0x0082_6a40 => extern "C" fn ch_function_history_init() {
        h::ch_function_history_init(&mut mem());
    }
    /// `Ch_GetBlkDirName(chr)`.
    Ch_GetBlkDirName @ 0x0082_6a60 => extern "C" fn ch_get_blk_dir_name(chr: u32) -> u32 {
        h::ch_get_blk_dir_name(&mem(), chr)
    }
    /// `Ch_GetChrRenderInsatanceFromOEF2Content(p)`.
    Ch_GetChrRenderInsatanceFromOEF2Content @ 0x0082_6af0 => extern "C" fn ch_get_chr_render_instance_from_oef2_content(p: u32) -> u32 {
        h::ch_get_chr_render_instance_from_oef2_content(&mem(), p)
    }
    /// `Ch_GetChrRenderInstance(chr)`.
    Ch_GetChrRenderInstance @ 0x0082_6b70 => extern "C" fn ch_get_chr_render_instance(chr: u32) -> u32 {
        h::ch_get_chr_render_instance(&mem(), chr)
    }
    /// `Ch_GetId(chr)`.
    Ch_GetId @ 0x0082_6b80 => extern "C" fn ch_get_id(chr: u32) -> u32 {
        h::ch_get_id(&mem(), chr)
    }
    /// `Ch_GetName(chr)`.
    Ch_GetName @ 0x0082_6b90 => extern "C" fn ch_get_name(chr: u32) -> u32 {
        h::ch_get_name(&mem(), chr)
    }
    /// `Ch_HandleGetNext()`: the next active character, or 0.
    Ch_HandleGetNext @ 0x0082_6d70 => extern "C" fn ch_handle_get_next() -> u32 {
        h::ch_handle_get_next(&mut mem())
    }
    /// `Ch_HandleRewind()`.
    Ch_HandleRewind @ 0x0082_6db0 => extern "C" fn ch_handle_rewind() {
        h::ch_handle_rewind(&mut mem());
    }

    // ---- getters

    /// `Ch_EffectGetEffectLevel(chr)`.
    Ch_EffectGetEffectLevel @ 0x0082_a9a0 => extern "C" fn ch_effect_get_effect_level(chr: u32) -> u32 {
        c::ch_effect_get_effect_level(&mem(), chr)
    }
    /// `Ch_EffectGetResidentPtr(chr)`.
    Ch_EffectGetResidentPtr @ 0x0082_a9b0 => extern "C" fn ch_effect_get_resident_ptr(chr: u32) -> u32 {
        c::ch_effect_get_resident_ptr(&mem(), chr)
    }
    /// `Ch_GetBodyHit(chr)`.
    Ch_GetBodyHit @ 0x0082_ab10 => extern "C" fn ch_get_body_hit(chr: u32) -> u32 {
        c::ch_get_body_hit(&mem(), chr)
    }
    /// `Ch_GetCamHeight(chr)`.
    Ch_GetCamHeight @ 0x0082_ab30 => extern "C" fn ch_get_cam_height(chr: u32) -> f32 {
        c::ch_get_cam_height(&mem(), chr)
    }
    /// `Ch_GetClip(chr)`.
    Ch_GetClip @ 0x0082_ab40 => extern "C" fn ch_get_clip(chr: u32) -> u32 {
        c::ch_get_clip(&mem(), chr)
    }
    /// `Ch_GetDataScale(chr)`.
    Ch_GetDataScale @ 0x0082_ab60 => extern "C" fn ch_get_data_scale(chr: u32) -> f32 {
        c::ch_get_data_scale(&mem(), chr)
    }
    /// `Ch_GetDir(chr)`.
    Ch_GetDir @ 0x0082_ab80 => extern "C" fn ch_get_dir(chr: u32) -> f32 {
        c::ch_get_dir(&mem(), chr)
    }
    /// `Ch_GetFieldMode(chr)`.
    Ch_GetFieldMode @ 0x0082_ab90 => extern "C" fn ch_get_field_mode(chr: u32) -> i32 {
        c::ch_get_field_mode(&mem(), chr)
    }
    /// `Ch_GetFixHeight(chr)`.
    Ch_GetFixHeight @ 0x0082_aba0 => extern "C" fn ch_get_fix_height(chr: u32) -> f32 {
        c::ch_get_fix_height(&mem(), chr)
    }
    /// `Ch_GetGrav(chr)`.
    Ch_GetGrav @ 0x0082_abb0 => extern "C" fn ch_get_grav(chr: u32) -> f32 {
        c::ch_get_grav(&mem(), chr)
    }
    /// `Ch_GetGravMode(chr)`.
    Ch_GetGravMode @ 0x0082_abc0 => extern "C" fn ch_get_grav_mode(chr: u32) -> i32 {
        c::ch_get_grav_mode(&mem(), chr)
    }
    /// `Ch_GetHide(chr)`.
    Ch_GetHide @ 0x0082_abd0 => extern "C" fn ch_get_hide(chr: u32) -> i32 {
        c::ch_get_hide(&mem(), chr)
    }
    /// `Ch_GetHideCalc(chr)`.
    Ch_GetHideCalc @ 0x0082_abe0 => extern "C" fn ch_get_hide_calc(chr: u32) -> u32 {
        c::ch_get_hide_calc(&mem(), chr)
    }
    /// `Ch_GetHideEvent(chr)`.
    Ch_GetHideEvent @ 0x0082_ac20 => extern "C" fn ch_get_hide_event(chr: u32) -> u32 {
        c::ch_get_hide_event(&mem(), chr)
    }
    /// `Ch_GetJimen(chr)`.
    Ch_GetJimen @ 0x0082_ac40 => extern "C" fn ch_get_jimen(chr: u32) -> f32 {
        c::ch_get_jimen(&mem(), chr)
    }
    /// `Ch_GetMotionSpeed(chr)`.
    Ch_GetMotionSpeed @ 0x0082_ac50 => extern "C" fn ch_get_motion_speed(chr: u32) -> i32 {
        c::ch_get_motion_speed(&mem(), chr)
    }
    /// `Ch_GetOrgMatrix(chr)`.
    Ch_GetOrgMatrix @ 0x0082_ac60 => extern "C" fn ch_get_org_matrix(chr: u32) -> u32 {
        c::ch_get_org_matrix(&mem(), chr)
    }
    /// `Ch_GetOverlapHit(chr)`.
    Ch_GetOverlapHit @ 0x0082_ac70 => extern "C" fn ch_get_overlap_hit(chr: u32) -> u32 {
        c::ch_get_overlap_hit(&mem(), chr)
    }
    /// `Ch_GetPos(chr, &x, &y, &z)`.
    Ch_GetPos @ 0x0082_ac90 => extern "C" fn ch_get_pos(chr: u32, x: u32, y: u32, z: u32) {
        c::ch_get_pos(&mut mem(), chr, x, y, z);
    }
    /// `Ch_GetResHide(chr)`.
    Ch_GetResHide @ 0x0082_acd0 => extern "C" fn ch_get_res_hide(chr: u32) -> i32 {
        c::ch_get_hide(&mem(), chr)
    }
    /// `Ch_GetRot(chr)`: wrapped to ±π.
    Ch_GetRot @ 0x0082_ace0 => extern "C" fn ch_get_rot(chr: u32) -> f32 {
        c::ch_get_rot(&mem(), chr)
    }
    /// `Ch_GetScale(chr, &x, &y, &z)`.
    Ch_GetScale @ 0x0082_ad00 => extern "C" fn ch_get_scale(chr: u32, x: u32, y: u32, z: u32) {
        c::ch_get_scale(&mut mem(), chr, x, y, z);
    }
    /// `Ch_GetScaleX(chr)`.
    Ch_GetScaleX @ 0x0082_ad30 => extern "C" fn ch_get_scale_x(chr: u32) -> f32 {
        c::ch_get_scale_axis(&mem(), chr, 0)
    }
    /// `Ch_GetScaleY(chr)`.
    Ch_GetScaleY @ 0x0082_ad40 => extern "C" fn ch_get_scale_y(chr: u32) -> f32 {
        c::ch_get_scale_axis(&mem(), chr, 1)
    }
    /// `Ch_GetScaleZ(chr)`.
    Ch_GetScaleZ @ 0x0082_ad50 => extern "C" fn ch_get_scale_z(chr: u32) -> f32 {
        c::ch_get_scale_axis(&mem(), chr, 2)
    }
    /// `Ch_GetSp(chr)`.
    Ch_GetSp @ 0x0082_ad60 => extern "C" fn ch_get_sp(chr: u32) -> f32 {
        c::ch_get_sp(&mem(), chr)
    }
    /// `Ch_GetThickness(chr)`.
    Ch_GetThickness @ 0x0082_ad70 => extern "C" fn ch_get_thickness(chr: u32) -> f32 {
        c::ch_get_thickness(&mem(), chr)
    }
    /// `Ch_GetThicknessBtl(chr)`.
    Ch_GetThicknessBtl @ 0x0082_ad90 => extern "C" fn ch_get_thickness_btl(chr: u32) -> f32 {
        c::ch_get_thickness_btl(&mem(), chr)
    }
    /// `Ch_GetThicknessBtlCam(chr)`.
    Ch_GetThicknessBtlCam @ 0x0082_adb0 => extern "C" fn ch_get_thickness_btl_cam(chr: u32) -> f32 {
        c::ch_get_thickness_btl_cam(&mem(), chr)
    }
    /// `Ch_IdGetPNum(chr)`.
    Ch_IdGetPNum @ 0x0082_adc0 => extern "C" fn ch_id_get_p_num(chr: u32) -> i32 {
        c::ch_id_get_p_num(&mem(), chr)
    }
    /// `Ch_ModelGetHide(chr, model)`.
    Ch_ModelGetHide @ 0x0082_add0 => extern "C" fn ch_model_get_hide(chr: u32, model: u32) -> i32 {
        c::ch_model_get_hide(&mem(), chr, model)
    }
    /// `Ch_ModelGetNromPtr(chr, model)`.
    Ch_ModelGetNromPtr @ 0x0082_adf0 => extern "C" fn ch_model_get_nrom_ptr(chr: u32, model: u32) -> u32 {
        c::ch_model_get_nrom_ptr(&mem(), chr, model)
    }
    /// `Ch_ModelGetVtexPtr(chr, model)`.
    Ch_ModelGetVtexPtr @ 0x0082_ae10 => extern "C" fn ch_model_get_vtex_ptr(chr: u32, model: u32) -> u32 {
        c::ch_model_get_vtex_ptr(&mem(), chr, model)
    }
    /// `Ch_TextureGetAnimId(chr)`.
    Ch_TextureGetAnimId @ 0x0082_b960 => extern "C" fn ch_texture_get_anim_id(chr: u32) -> u32 {
        c::ch_texture_get_anim_id(&mem(), chr)
    }

    // ---- flags

    /// `Ch_EnBodyHit(chr, on)` -> old "no body hit" bit.
    Ch_EnBodyHit @ 0x0082_a9f0 => extern "C" fn ch_en_body_hit(chr: u32, on: u32) -> u32 {
        c::ch_en_body_hit(&mut mem(), chr, on)
    }
    /// `Ch_EnClip(chr, on)` -> old bit.
    Ch_EnClip @ 0x0082_aa30 => extern "C" fn ch_en_clip(chr: u32, on: u32) -> u32 {
        c::ch_en_clip(&mut mem(), chr, on)
    }
    /// `Ch_EnMapID(chr, on)`, then `Ch_IdEnableShade`.
    Ch_EnMapID @ 0x0082_aa70 => extern "C" fn ch_en_map_id(chr: u32, on: u32) -> u32 {
        c::ch_en_map_id(&mut mem(), chr, on)
    }
    /// `Ch_EnRotHokan(chr, on)` -> old bit.
    Ch_EnRotHokan @ 0x0082_aae0 => extern "C" fn ch_en_rot_hokan(chr: u32, on: u32) -> u32 {
        c::ch_en_rot_hokan(&mut mem(), chr, on)
    }
    /// `Ch_SetChocobo(chr, on)`.
    Ch_SetChocobo @ 0x0082_b130 => extern "C" fn ch_set_chocobo(chr: u32, on: u32) -> u32 {
        c::ch_set_chocobo(&mut mem(), chr, on)
    }
    /// `Ch_SetHideBattle(chr, on)` -> whether it was hidden so.
    Ch_SetHideBattle @ 0x0082_b260 => extern "C" fn ch_set_hide_battle(chr: u32, on: u32) -> u32 {
        c::ch_set_hide_battle(&mut mem(), chr, on)
    }
    /// `Ch_SetHideEffect(chr, on)` -> whether it was hidden so.
    Ch_SetHideEffect @ 0x0082_b2a0 => extern "C" fn ch_set_hide_effect(chr: u32, on: u32) -> u32 {
        c::ch_set_hide_effect(&mut mem(), chr, on)
    }
    /// `Ch_SetOverlapHit(chr, on)`.
    Ch_SetOverlapHit @ 0x0082_b420 => extern "C" fn ch_set_overlap_hit(chr: u32, on: u32) -> u32 {
        c::ch_set_overlap_hit(&mut mem(), chr, on)
    }
    /// `Ch_Vu1SetClipmode(chr, on)`.
    Ch_Vu1SetClipmode @ 0x0082_b970 => extern "C" fn ch_vu1_set_clipmode(chr: u32, on: u32) -> u32 {
        c::ch_vu1_set_clipmode(&mut mem(), chr, on)
    }
    /// `Chr_FadeSet(fade, target, frames)`.
    Chr_FadeSet @ 0x0082_b990 => extern "C" fn chr_fade_set(fade: u32, target: f32, frames: i32) {
        c::chr_fade_set(&mut mem(), fade, target, frames);
    }
    /// `Chr_SetHideFlags(chr, on, mask)` -> whether any was set.
    Chr_SetHideFlags @ 0x0082_ba30 => extern "C" fn chr_set_hide_flags(chr: u32, on: u32, mask: u32) -> u32 {
        c::chr_set_hide_flags(&mut mem(), chr, on, mask)
    }
    /// `Chr_WrapAngle(a)`: to ±π by steps of 2π.
    Chr_WrapAngle @ 0x0082_ba70 => extern "C" fn chr_wrap_angle(a: f32) -> f32 {
        ffx_chara::chr::wrap_angle(a)
    }
    /// `Ch_IdEnableShade(chr, on)`.
    Ch_IdEnableShade @ 0x0083_40f0 => extern "C" fn ch_id_enable_shade(chr: u32, on: u32) -> u32 {
        c::ch_id_enable_shade(&mut mem(), chr, on)
    }

    // ---- setters

    /// `Ch_AddGrav(chr, g)`.
    Ch_AddGrav @ 0x0082_a890 => extern "C" fn ch_add_grav(chr: u32, g: f32) {
        c::ch_add_grav(&mut mem(), chr, g);
    }
    /// `Ch_EffectSetEffectLevel(chr, level)`.
    Ch_EffectSetEffectLevel @ 0x0082_a9d0 => extern "C" fn ch_effect_set_effect_level(chr: u32, level: u32) {
        c::ch_effect_set_effect_level(&mut mem(), chr, level);
    }
    /// `Ch_MotionSetHokan(chr, frames)`.
    Ch_MotionSetHokan @ 0x0082_afb0 => extern "C" fn ch_motion_set_hokan(chr: u32, frames: u32) {
        c::ch_motion_set_hokan(&mut mem(), chr, frames);
    }
    /// `Ch_MotionSetMoves(chr, slot, motion)`.
    Ch_MotionSetMoves @ 0x0082_afe0 => extern "C" fn ch_motion_set_moves(chr: u32, slot: u32, motion: u32) {
        c::ch_motion_set_moves(&mut mem(), chr, slot, motion);
    }
    /// `Ch_MotionSetNextHokan(chr, frames)`.
    Ch_MotionSetNextHokan @ 0x0082_b010 => extern "C" fn ch_motion_set_next_hokan(chr: u32, frames: i32) {
        c::ch_motion_set_next_hokan(&mut mem(), chr, frames);
    }
    /// `Ch_MoveSetChgRun(chr, speed)` -> old.
    Ch_MoveSetChgRun @ 0x0082_b040 => extern "C" fn ch_move_set_chg_run(chr: u32, speed: f32) -> f32 {
        c::ch_move_set_chg_run(&mut mem(), chr, speed)
    }
    /// `Ch_ResetMotion(chr)`.
    Ch_ResetMotion @ 0x0082_b080 => extern "C" fn ch_reset_motion(chr: u32) {
        c::ch_reset_motion(&mut mem(), chr);
    }
    /// `Ch_ResetPos(chr)`.
    Ch_ResetPos @ 0x0082_b0b0 => extern "C" fn ch_reset_pos(chr: u32) {
        c::ch_reset_pos(&mut mem(), chr);
    }
    /// `Ch_SetBattleChrNum(chr, n)`.
    Ch_SetBattleChrNum @ 0x0082_b0d0 => extern "C" fn ch_set_battle_chr_num(chr: u32, n: u32) {
        c::ch_set_battle_chr_num(&mut mem(), chr, n);
    }
    /// `Ch_SetCamHeight(chr, h)`.
    Ch_SetCamHeight @ 0x0082_b0f0 => extern "C" fn ch_set_cam_height(chr: u32, h: f32) {
        c::ch_set_cam_height(&mut mem(), chr, h);
    }
    /// `Ch_SetDir(chr, dir)`.
    Ch_SetDir @ 0x0082_b190 => extern "C" fn ch_set_dir(chr: u32, dir: f32) {
        c::ch_set_f32(&mut mem(), chr, l::DIR, dir);
    }
    /// `Ch_SetFieldMode(chr, mode)`.
    Ch_SetFieldMode @ 0x0082_b1e0 => extern "C" fn ch_set_field_mode(chr: u32, mode: u32) {
        c::ch_set_u8(&mut mem(), chr, l::FIELD_MODE, mode);
    }
    /// `Ch_SetFixHeight(chr, h)`.
    Ch_SetFixHeight @ 0x0082_b200 => extern "C" fn ch_set_fix_height(chr: u32, h: f32) {
        c::ch_set_fix_height(&mut mem(), chr, h);
    }
    /// `Ch_SetGravMode(chr, mode)`.
    Ch_SetGravMode @ 0x0082_b240 => extern "C" fn ch_set_grav_mode(chr: u32, mode: u32) {
        c::ch_set_u8(&mut mem(), chr, l::GRAV_MODE, mode);
    }
    /// `Ch_SetHideCalc(chr, on)`.
    Ch_SetHideCalc @ 0x0082_b280 => extern "C" fn ch_set_hide_calc(chr: u32, on: u32) {
        c::ch_set_u8(&mut mem(), chr, l::HIDE_CALC, on);
    }
    /// `Ch_SetHitHeight(chr, h)`.
    Ch_SetHitHeight @ 0x0082_b320 => extern "C" fn ch_set_hit_height(chr: u32, h: f32) {
        c::ch_set_f32(&mut mem(), chr, l::HIT_HEIGHT, h);
    }
    /// `Ch_SetHiju(chr, v)`.
    Ch_SetHiju @ 0x0082_b340 => extern "C" fn ch_set_hiju(chr: u32, v: f32) {
        c::ch_set_f32(&mut mem(), chr, l::HIJU, v);
    }
    /// `Ch_SetJimen(chr, y)`.
    Ch_SetJimen @ 0x0082_b360 => extern "C" fn ch_set_jimen(chr: u32, y: f32) {
        c::ch_set_f32(&mut mem(), chr, l::JIMEN, y);
    }
    /// `Ch_SetMotionSpeed(chr, speed)`.
    Ch_SetMotionSpeed @ 0x0082_b380 => extern "C" fn ch_set_motion_speed(chr: u32, speed: u32) {
        c::ch_set_motion_speed(&mut mem(), chr, speed);
    }
    /// `Ch_SetMotionType(chr, kind)`.
    Ch_SetMotionType @ 0x0082_b3a0 => extern "C" fn ch_set_motion_type(chr: u32, kind: u32) {
        c::ch_set_u8(&mut mem(), chr, l::MOTION_TYPE, kind);
    }
    /// `Ch_SetOffset(chr, x, y, z)`.
    Ch_SetOffset @ 0x0082_b3c0 => extern "C" fn ch_set_offset(chr: u32, x: f32, y: f32, z: f32) {
        c::ch_set_offset(&mut mem(), chr, x, y, z);
    }
    /// `Ch_SetOrgMatrix(chr, matrix)`.
    Ch_SetOrgMatrix @ 0x0082_b3e0 => extern "C" fn ch_set_org_matrix(chr: u32, matrix: u32) {
        c::ch_set_org_matrix(&mut mem(), chr, matrix);
    }
    /// `Ch_SetOrgMatrixScale(chr, s)`.
    Ch_SetOrgMatrixScale @ 0x0082_b400 => extern "C" fn ch_set_org_matrix_scale(chr: u32, s: f32) {
        c::ch_set_f32(&mut mem(), chr, l::ORG_MATRIX_SCALE, s);
    }
    /// `Ch_SetPos(chr, x, z)`.
    Ch_SetPos @ 0x0082_b440 => extern "C" fn ch_set_pos(chr: u32, x: f32, z: f32) {
        c::ch_set_pos(&mut mem(), chr, x, z);
    }
    /// `Ch_SetPos3(chr, x, y, z)`.
    Ch_SetPos3 @ 0x0082_b480 => extern "C" fn ch_set_pos3(chr: u32, x: f32, y: f32, z: f32) {
        c::ch_set_pos3(&mut mem(), chr, x, y, z);
    }
    /// `Ch_SetScale(chr, axes, x, y, z)`.
    Ch_SetScale @ 0x0082_b560 => extern "C" fn ch_set_scale(chr: u32, axes: u32, x: f32, y: f32, z: f32) {
        c::ch_set_scale(&mut mem(), chr, axes, x, y, z);
    }
    /// `Ch_SetScaleAll(chr, s)`.
    Ch_SetScaleAll @ 0x0082_b590 => extern "C" fn ch_set_scale_all(chr: u32, s: f32) {
        c::ch_set_scale_all(&mut mem(), chr, s);
    }
    /// `Ch_SetShade(chr, v, frames)`.
    Ch_SetShade @ 0x0082_b5b0 => extern "C" fn ch_set_shade(chr: u32, v: f32, frames: i32) {
        c::ch_set_fade(&mut mem(), chr, l::FADE_SHADE, v, frames);
    }
    /// `Ch_SetShadeColEffect(chr, r, g, b)`.
    Ch_SetShadeColEffect @ 0x0082_b6e0 => extern "C" fn ch_set_shade_col_effect(chr: u32, r: f32, g: f32, b: f32) {
        c::ch_set_shade_col_effect(&mut mem(), chr, r, g, b);
    }
    /// `Ch_SetShadowMode(chr, mode)` -> old.
    Ch_SetShadowMode @ 0x0082_b820 => extern "C" fn ch_set_shadow_mode(chr: u32, mode: u32) -> i32 {
        c::ch_set_shadow_mode(&mut mem(), chr, mode)
    }
    /// `Ch_SetSp(chr, sp)`.
    Ch_SetSp @ 0x0082_b840 => extern "C" fn ch_set_sp(chr: u32, sp: f32) {
        c::ch_set_f32(&mut mem(), chr, l::SP, sp);
    }
    /// `Ch_SetSpecularAlpha(chr, v, frames)`.
    Ch_SetSpecularAlpha @ 0x0082_b860 => extern "C" fn ch_set_specular_alpha(chr: u32, v: f32, frames: i32) {
        c::ch_set_fade(&mut mem(), chr, l::FADE_SPECULAR, v, frames);
    }
    /// `Ch_SetThickness(chr, t)` -> old.
    Ch_SetThickness @ 0x0082_b880 => extern "C" fn ch_set_thickness(chr: u32, t: f32) -> f32 {
        c::ch_set_thickness(&mut mem(), chr, t)
    }
    /// `Ch_SetThicknessBtlCam(chr, t)`.
    Ch_SetThicknessBtlCam @ 0x0082_b8c0 => extern "C" fn ch_set_thickness_btl_cam(chr: u32, t: f32) {
        c::ch_set_thickness_btl_cam(&mut mem(), chr, t);
    }
    /// `Ch_SetTransparent(chr, alpha, frames)`.
    Ch_SetTransparent @ 0x0082_b900 => extern "C" fn ch_set_transparent(chr: u32, alpha: f32, frames: i32) {
        c::ch_set_fade(&mut mem(), chr, l::FADE_ALPHA, alpha, frames);
    }
    /// `Ch_SetVelY(chr, v)`.
    Ch_SetVelY @ 0x0082_b920 => extern "C" fn ch_set_vel_y(chr: u32, v: f32) {
        c::ch_set_vel_y(&mut mem(), chr, v);
    }
    /// `Ch_SetWeight(chr, w)` -> old.
    Ch_SetWeight @ 0x0082_b940 => extern "C" fn ch_set_weight(chr: u32, w: f32) -> f32 {
        c::ch_set_weight(&mut mem(), chr, w)
    }
}
