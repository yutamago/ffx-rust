//! `ffx-chara`: finding characters and their accessors. All cdecl, the
//! character first; floats are `f32` on the stack and in `st0`.

use ffx_chara::attach::bridge as at;
use ffx_chara::chr::bridge as c;
use ffx_chara::cloth::bridge as cl;
use ffx_chara::handle::bridge as h;
use ffx_chara::id::bridge as id;
use ffx_chara::layout as l;
use ffx_chara::mapping::{self as mp, bridge as mb};
use ffx_chara::matrix::bridge as mx;
use ffx_chara::motion as mo;
use ffx_chara::parts::bridge as p;
use ffx_chara::sys::bridge as sy;
use ffx_chara::texanim::bridge as ta;
use ffx_chara::texture::bridge as tx;

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

    // ---- cloth and position
    /// `Ch_ClothEnableGrav(chr, on)`.
    Ch_ClothEnableGrav @ 0x0082_45f0 => extern "C" fn ch_cloth_enable_grav(chr: u32, on: u32) -> u32 {
        p::ch_cloth_enable_grav(&mut mem(), chr, on)
    }
    /// `Ch_ClothGetParaEdit()`.
    Ch_ClothGetParaEdit @ 0x0082_4610 => extern "C" fn ch_cloth_get_para_edit() -> i32 {
        sy::get_i8(&mem(), sy::CLOTH_PARA_EDIT)
    }
    /// `Ch_ClothMeshMappingOffset(chr)`.
    Ch_ClothMeshMappingOffset @ 0x0082_4620 => extern "C" fn ch_cloth_mesh_mapping_offset(chr: u32) {
        p::ch_cloth_mesh_mapping_offset(&mut mem(), chr);
    }
    /// `Ch_ClothSetFast(chr)`: settle frames by id.
    Ch_ClothSetFast @ 0x0082_4920 => extern "C" fn ch_cloth_set_fast(chr: u32) {
        p::ch_cloth_set_fast(&mut mem(), chr);
    }
    /// `Ch_ClothSetParaEdit(v)`.
    Ch_ClothSetParaEdit @ 0x0082_4a30 => extern "C" fn ch_cloth_set_para_edit(v: u32) {
        sy::set_i8(&mut mem(), sy::CLOTH_PARA_EDIT, v);
    }
    /// `Ch_ClothSetXZ(v)`.
    Ch_ClothSetXZ @ 0x0082_4a40 => extern "C" fn ch_cloth_set_xz(v: f32) {
        sy::set_f32(&mut mem(), sy::CLOTH_XZ, v);
    }
    /// `Ch_ClothSetY(v)`.
    Ch_ClothSetY @ 0x0082_4a50 => extern "C" fn ch_cloth_set_y(v: f32) {
        sy::set_f32(&mut mem(), sy::CLOTH_Y, v);
    }
    /// `Ch_MemMoveSetTrigger()`.
    Ch_MemMoveSetTrigger @ 0x0082_7250 => extern "C" fn ch_mem_move_set_trigger() {
        sy::ch_mem_move_set_trigger(&mut mem());
    }
    /// `Ch_WorkSwap(chr)`: the copy's address.
    Ch_WorkSwap @ 0x0082_8b30 => extern "C" fn ch_work_swap(chr: u32) -> u32 {
        p::ch_work_swap(&mut mem(), chr)
    }
    /// `Ch_IsLoadingClothFile(key)`.
    Ch_IsLoadingClothFile @ 0x0082_a100 => extern "C" fn ch_is_loading_cloth_file(key: u32) -> u32 {
        h::ch_is_loading_cloth_file(&mem(), key)
    }
    /// `Ch_AddY(chr, dy)`.
    Ch_AddY @ 0x0082_a8b0 => extern "C" fn ch_add_y(chr: u32, dy: f32) {
        p::ch_add_y(&mut mem(), chr, dy);
    }
    /// `Ch_AllSetThicknessEvent(t)`.
    Ch_AllSetThicknessEvent @ 0x0082_a8d0 => extern "C" fn ch_all_set_thickness_event(t: f32) {
        p::ch_all_set_thickness_event(&mut mem(), t);
    }
    /// `Ch_ClothSetEnable(chr, on)`.
    Ch_ClothSetEnable @ 0x0082_a930 => extern "C" fn ch_cloth_set_enable(chr: u32, on: u32) -> u32 {
        p::ch_cloth_set_enable(&mut mem(), chr, on)
    }
    /// `Ch_DisableBodyHit(chr, off)`.
    Ch_DisableBodyHit @ 0x0082_a950 => extern "C" fn ch_disable_body_hit(chr: u32, off: u32) -> u32 {
        p::ch_disable_body_hit(&mut mem(), chr, off)
    }
    /// `Ch_DisableID(chr, off)`.
    Ch_DisableID @ 0x0082_a980 => extern "C" fn ch_disable_id(chr: u32, off: u32) -> u32 {
        p::ch_disable_id(&mut mem(), chr, off)
    }

    // ---- drawing
    /// `Ch_DrawCalcShade(chr, &r, &g, &b)`.
    Ch_DrawCalcShade @ 0x0082_eaf0 => extern "C" fn ch_draw_calc_shade(chr: u32, r: u32, g: u32, b: u32) {
        p::ch_draw_calc_shade(&mut mem(), chr, r, g, b);
    }
    /// `Ch_DrawEnEffect(chr, on)`.
    Ch_DrawEnEffect @ 0x0082_eb60 => extern "C" fn ch_draw_en_effect(chr: u32, on: u32) -> u32 {
        p::ch_draw_en_effect(&mut mem(), chr, on)
    }
    /// `Ch_DrawGetDisableSpec()`.
    Ch_DrawGetDisableSpec @ 0x0082_eb80 => extern "C" fn ch_draw_get_disable_spec() -> i32 {
        sy::get_i8(&mem(), sy::DRAW_DISABLE_SPEC)
    }
    /// `Ch_DrawGetFourceSpec()`.
    Ch_DrawGetFourceSpec @ 0x0082_eb90 => extern "C" fn ch_draw_get_fource_spec() -> i32 {
        sy::get_i8(&mem(), sy::DRAW_FOURCE_SPEC)
    }
    /// `Ch_DrawGetModelAlpha(chr, model)`.
    Ch_DrawGetModelAlpha @ 0x0082_eba0 => extern "C" fn ch_draw_get_model_alpha(chr: u32, model: u32) -> f32 {
        p::ch_draw_get_model_alpha(&mem(), chr, model)
    }
    /// `Ch_DrawGetParaEdit()`.
    Ch_DrawGetParaEdit @ 0x0082_ebe0 => extern "C" fn ch_draw_get_para_edit() -> i32 {
        sy::get_i8(&mem(), sy::DRAW_PARA_EDIT)
    }
    /// `Ch_DrawGetShadowColor(&r, &g, &b)`.
    Ch_DrawGetShadowColor @ 0x0082_ebf0 => extern "C" fn ch_draw_get_shadow_color(r: u32, g: u32, b: u32) {
        sy::ch_draw_get_shadow_color(&mut mem(), [r, g, b]);
    }
    /// `Ch_DrawGetShadowDir(&a, &b)`.
    Ch_DrawGetShadowDir @ 0x0082_ec30 => extern "C" fn ch_draw_get_shadow_dir(a: u32, b: u32) {
        sy::ch_draw_get_shadow_dir(&mut mem(), [a, b]);
    }
    /// `Ch_DrawSetCull(chr, on)`.
    Ch_DrawSetCull @ 0x0083_00b0 => extern "C" fn ch_draw_set_cull(chr: u32, on: u32) -> u32 {
        p::ch_draw_set_cull(&mut mem(), chr, on)
    }
    /// `Ch_DrawSetDisableSpec(v)`.
    Ch_DrawSetDisableSpec @ 0x0083_00d0 => extern "C" fn ch_draw_set_disable_spec(v: u32) {
        sy::set_i8(&mut mem(), sy::DRAW_DISABLE_SPEC, v);
    }
    /// `Ch_DrawSetFourceSpec(v)`.
    Ch_DrawSetFourceSpec @ 0x0083_00e0 => extern "C" fn ch_draw_set_fource_spec(v: u32) {
        sy::set_i8(&mut mem(), sy::DRAW_FOURCE_SPEC, v);
    }
    /// `Ch_DrawSetParaEdit(v)`.
    Ch_DrawSetParaEdit @ 0x0083_00f0 => extern "C" fn ch_draw_set_para_edit(v: u32) {
        sy::set_i8(&mut mem(), sy::DRAW_PARA_EDIT, v);
    }
    /// `Ch_DrawSetShadowColor(r, g, b)`.
    Ch_DrawSetShadowColor @ 0x0083_0100 => extern "C" fn ch_draw_set_shadow_color(r: u32, g: u32, b: u32) {
        sy::ch_draw_set_shadow_color(&mut mem(), [r, g, b]);
    }
    /// `Ch_DrawSetShadowDir(a, b)`.
    Ch_DrawSetShadowDir @ 0x0083_0120 => extern "C" fn ch_draw_set_shadow_dir(a: u32, b: u32) {
        sy::ch_draw_set_shadow_dir(&mut mem(), [a, b]);
    }
    /// `Ch_DrawSetShadowMode(mode)`.
    Ch_DrawSetShadowMode @ 0x0083_0140 => extern "C" fn ch_draw_set_shadow_mode(mode: u32) {
        sy::ch_draw_set_shadow_mode(&mut mem(), mode);
    }
    /// `Ch_EnableGetTexture()`.
    Ch_EnableGetTexture @ 0x0083_0320 => extern "C" fn ch_enable_get_texture() -> i32 {
        sy::get_i8(&mem(), sy::ENABLE_TEXTURE)
    }

    // ---- models, lights, shadow, textures
    /// `Ch_GetNbElem(chr)`.
    Ch_GetNbElem @ 0x0083_03f0 => extern "C" fn ch_get_nb_elem(chr: u32) -> u32 {
        p::ch_get_nb_elem(&mem(), chr)
    }
    /// `Ch_GetNbModel(chr)`.
    Ch_GetNbModel @ 0x0083_0410 => extern "C" fn ch_get_nb_model(chr: u32) -> u32 {
        p::ch_get_nb_model(&mem(), chr)
    }
    /// `Ch_GetShadowDirH(chr)`.
    Ch_GetShadowDirH @ 0x0083_0440 => extern "C" fn ch_get_shadow_dir_h(chr: u32) -> u32 {
        p::ch_get_shadow_dir_h(&mem(), chr)
    }
    /// `Ch_PLightGetEnable(chr)`.
    Ch_PLightGetEnable @ 0x0083_0ad0 => extern "C" fn ch_plight_get_enable(chr: u32) -> i32 {
        p::ch_plight_get_enable(&mem(), chr)
    }
    /// `Ch_PLightSetEnable(chr, bits)`.
    Ch_PLightSetEnable @ 0x0083_0ae0 => extern "C" fn ch_plight_set_enable(chr: u32, bits: u32) {
        p::ch_plight_set_enable(&mut mem(), chr, bits);
    }
    /// `Ch_PLightSetEnableMask(chr, bits, mask)`.
    Ch_PLightSetEnableMask @ 0x0083_0b00 => extern "C" fn ch_plight_set_enable_mask(chr: u32, bits: u32, mask: u32) {
        p::ch_plight_set_enable_mask(&mut mem(), chr, bits, mask);
    }
    /// `Ch_SetShadowDirH(chr, dir)`.
    Ch_SetShadowDirH @ 0x0083_0c90 => extern "C" fn ch_set_shadow_dir_h(chr: u32, dir: u32) {
        p::ch_set_shadow_dir_h(&mut mem(), chr, dir);
    }
    /// `Ch_ShadowSetHeight(chr, h)`.
    Ch_ShadowSetHeight @ 0x0083_0cb0 => extern "C" fn ch_shadow_set_height(chr: u32, h: f32) {
        p::ch_shadow_set_height(&mut mem(), chr, h);
    }
    /// `Ch_TextureAnimSetEnable(chr, on)`.
    Ch_TextureAnimSetEnable @ 0x0083_0d10 => extern "C" fn ch_texture_anim_set_enable(chr: u32, on: u32) -> u32 {
        tx::ch_texture_anim_set_enable(&mut mem(), chr, on)
    }
    /// `Ch_TextureGetClutAddr(chr, i)`.
    Ch_TextureGetClutAddr @ 0x0083_0d30 => extern "C" fn ch_texture_get_clut_addr(chr: u32, i: i32) -> u32 {
        tx::ch_texture_get_clut_addr(&mem(), chr, i)
    }
    /// `Ch_TextureGetClutBase(chr, i)`.
    Ch_TextureGetClutBase @ 0x0083_0d60 => extern "C" fn ch_texture_get_clut_base(chr: u32, i: i32) -> u32 {
        tx::ch_texture_get_clut_base(&mem(), chr, i)
    }
    /// `Ch_TextureGetImageAddr(chr, i)`.
    Ch_TextureGetImageAddr @ 0x0083_0d90 => extern "C" fn ch_texture_get_image_addr(chr: u32, i: i32) -> u32 {
        tx::ch_texture_get_image_addr(&mem(), chr, i)
    }
    /// `Ch_TextureGetImageBase(chr, i)`.
    Ch_TextureGetImageBase @ 0x0083_0dc0 => extern "C" fn ch_texture_get_image_base(chr: u32, i: i32) -> u32 {
        tx::ch_texture_get_image_base(&mem(), chr, i)
    }

    // ---- binding and elements
    /// `Ch_BindSetScale(chr, s)`.
    Ch_BindSetScale @ 0x0083_2610 => extern "C" fn ch_bind_set_scale(chr: u32, s: f32) {
        p::ch_bind_set_scale(&mut mem(), chr, s);
    }
    /// `Ch_BindToElemDirect(chr, parent, elem)`.
    Ch_BindToElemDirect @ 0x0083_2680 => extern "C" fn ch_bind_to_elem_direct(chr: u32, parent: u32, elem: u32) {
        p::ch_bind_to_elem_direct(&mut mem(), chr, parent, elem);
    }
    /// `Ch_BindToElemDirectOfs(chr, parent, elem, x, y, z)`.
    Ch_BindToElemDirectOfs @ 0x0083_26e0 => extern "C" fn ch_bind_to_elem_direct_ofs(chr: u32, parent: u32, elem: u32, x: f32, y: f32, z: f32) {
        p::ch_bind_to_elem_direct_ofs(&mut mem(), chr, parent, elem, [x, y, z]);
    }
    /// `Ch_ElemGetParent(chr, elem)`.
    Ch_ElemGetParent @ 0x0083_3cf0 => extern "C" fn ch_elem_get_parent(chr: u32, elem: u32) -> u32 {
        p::ch_elem_get_parent(&mem(), chr, elem)
    }
    /// `Ch_ElemSetQuatMatrix(chr, elem, matrix, weight)`.
    Ch_ElemSetQuatMatrix @ 0x0083_3df0 => extern "C" fn ch_elem_set_quat_matrix(chr: u32, elem: u32, matrix: u32, w: f32) {
        p::ch_elem_set_quat_matrix(&mut mem(), chr, elem, matrix, w);
    }
    /// `Ch_ElemSetReplaceMatrix(chr, elem, matrix)`.
    Ch_ElemSetReplaceMatrix @ 0x0083_3e30 => extern "C" fn ch_elem_set_replace_matrix(chr: u32, elem: u32, matrix: u32) {
        p::ch_elem_set_replace_matrix(&mut mem(), chr, elem, matrix);
    }
    /// `Ch_RelElemMapping(list, delta)`.
    Ch_RelElemMapping @ 0x0083_77a0 => extern "C" fn ch_rel_elem_mapping(list: u32, delta: u32) {
        p::ch_rel_elem_mapping(&mut mem(), list, delta);
    }

    // ---- normals, motion, mouth and neck
    /// `Ch_GetCalcNorm(chr)`.
    Ch_GetCalcNorm @ 0x0083_3f30 => extern "C" fn ch_get_calc_norm(chr: u32) -> u32 {
        p::ch_get_calc_norm(&mem(), chr)
    }
    /// `Ch_GetDumpMotEv()`.
    Ch_GetDumpMotEv @ 0x0083_3f50 => extern "C" fn ch_get_dump_mot_ev() -> i32 {
        sy::get_i8(&mem(), sy::DUMP_MOT_EV)
    }
    /// `Ch_GetFuka(i)`.
    Ch_GetFuka @ 0x0083_3f60 => extern "C" fn ch_get_fuka(i: u32) -> u32 {
        sy::ch_get_fuka(&mem(), i)
    }
    /// `Ch_GetMotionType(chr)`.
    Ch_GetMotionType @ 0x0083_3f70 => extern "C" fn ch_get_motion_type(chr: u32) -> i32 {
        p::ch_get_motion_type(&mem(), chr)
    }
    /// `Ch_GetVel(chr, out)`.
    Ch_GetVel @ 0x0083_4000 => extern "C" fn ch_get_vel(chr: u32, out: u32) {
        p::ch_get_vel(&mut mem(), chr, out);
    }
    /// `Ch_GetVel2(chr, out)`.
    Ch_GetVel2 @ 0x0083_4030 => extern "C" fn ch_get_vel2(chr: u32, out: u32) {
        p::ch_get_vel2(&mut mem(), chr, out);
    }
    /// `Ch_GuiGetCalcNormal()`.
    Ch_GuiGetCalcNormal @ 0x0083_4060 => extern "C" fn ch_gui_get_calc_normal() -> u32 {
        sy::ch_gui_get_calc_normal(&mem())
    }
    /// `Ch_GuiSetCalcNormAll(on)`.
    Ch_GuiSetCalcNormAll @ 0x0083_4090 => extern "C" fn ch_gui_set_calc_norm_all(on: u32) {
        sy::ch_gui_set_calc_norm_all(&mut mem(), on);
    }
    /// `Ch_GuiSetMouthTh(v)`.
    Ch_GuiSetMouthTh @ 0x0083_40a0 => extern "C" fn ch_gui_set_mouth_th(v: i32) -> i32 {
        sy::ch_gui_set_mouth_th(&mut mem(), v)
    }
    /// `Ch_GuiSetTalk(a, b)`.
    Ch_GuiSetTalk @ 0x0083_40d0 => extern "C" fn ch_gui_set_talk(a: u32, b: u32) {
        sy::ch_gui_set_talk(&mut mem(), a, b);
    }
    /// `Ch_MouthGetAutoTalk(chr)`.
    Ch_MouthGetAutoTalk @ 0x0083_4740 => extern "C" fn ch_mouth_get_auto_talk(chr: u32) -> i32 {
        p::ch_mouth_get_auto_talk(&mem(), chr)
    }
    /// `Ch_MouthInit(chr)`.
    Ch_MouthInit @ 0x0083_4750 => extern "C" fn ch_mouth_init(chr: u32) {
        p::ch_mouth_init(&mut mem(), chr);
    }
    /// `Ch_MouthSetAutoTalk(chr, on)`.
    Ch_MouthSetAutoTalk @ 0x0083_4770 => extern "C" fn ch_mouth_set_auto_talk(chr: u32, on: u32) {
        p::ch_mouth_set_auto_talk(&mut mem(), chr, on);
    }
    /// `Ch_NeckSetLimit(chr, a, b)`.
    Ch_NeckSetLimit @ 0x0083_4fd0 => extern "C" fn ch_neck_set_limit(chr: u32, a: f32, b: f32) {
        p::ch_neck_set_limit(&mut mem(), chr, a, b);
    }
    /// `Ch_NeckSetSpeed(chr, speed)`.
    Ch_NeckSetSpeed @ 0x0083_5020 => extern "C" fn ch_neck_set_speed(chr: u32, speed: f32) {
        p::ch_neck_set_speed(&mut mem(), chr, speed);
    }
    /// `Ch_OptposGetBottom(chr, out)`.
    Ch_OptposGetBottom @ 0x0083_5600 => extern "C" fn ch_optpos_get_bottom(chr: u32, out: u32) {
        p::ch_optpos_copy(&mut mem(), chr, l::OPTPOS_BOTTOM, out);
    }
    /// `Ch_OptposGetCameraRef(chr, out)`.
    Ch_OptposGetCameraRef @ 0x0083_5630 => extern "C" fn ch_optpos_get_camera_ref(chr: u32, out: u32) {
        p::ch_optpos_copy(&mut mem(), chr, l::OPTPOS_CAMERA_REF, out);
    }
    /// `Ch_OptposGetExist(chr, i)`.
    Ch_OptposGetExist @ 0x0083_5680 => extern "C" fn ch_optpos_get_exist(chr: u32, i: u32) -> u32 {
        p::ch_optpos_get_exist(&mem(), chr, i)
    }
    /// `Ch_OptposGetTop(chr, out)`.
    Ch_OptposGetTop @ 0x0083_56e0 => extern "C" fn ch_optpos_get_top(chr: u32, out: u32) {
        p::ch_optpos_copy(&mut mem(), chr, l::OPTPOS_TOP, out);
    }
    /// `Ch_ReserveCalcNorm(chr)`.
    Ch_ReserveCalcNorm @ 0x0083_5710 => extern "C" fn ch_reserve_calc_norm(chr: u32) {
        p::ch_reserve_calc_norm(&mut mem(), chr, l::F_RESERVE_CALC_NORM);
    }
    /// `Ch_ReserveCalcNorm2(chr)`.
    Ch_ReserveCalcNorm2 @ 0x0083_5730 => extern "C" fn ch_reserve_calc_norm2(chr: u32) {
        p::ch_reserve_calc_norm(&mut mem(), chr, l::F_RESERVE_CALC_NORM2);
    }
    /// `Ch_SetCalcNorm(chr, on)` -> old bit.
    Ch_SetCalcNorm @ 0x0083_5750 => extern "C" fn ch_set_calc_norm(chr: u32, on: u32) -> u32 {
        p::ch_set_calc_norm(&mut mem(), chr, on)
    }
    /// `Ch_SetDumpMotEv(v)`.
    Ch_SetDumpMotEv @ 0x0083_57c0 => extern "C" fn ch_set_dump_mot_ev(v: u32) {
        sy::set_i8(&mut mem(), sy::DUMP_MOT_EV, v);
    }
    /// `Ch_SetFuka(i, v)`.
    Ch_SetFuka @ 0x0083_57d0 => extern "C" fn ch_set_fuka(i: u32, v: u32) {
        sy::ch_set_fuka(&mut mem(), i, v);
    }
    /// `Ch_SoundSetId(chr, id)`.
    Ch_SoundSetId @ 0x0083_5b50 => extern "C" fn ch_sound_set_id(chr: u32, id: u32) {
        p::ch_sound_set_id(&mut mem(), chr, id);
    }
    /// `Ch_SysGetScreenMatrixPtr()`.
    Ch_SysGetScreenMatrixPtr @ 0x0083_5b70 => extern "C" fn ch_sys_get_screen_matrix_ptr() -> u32 {
        sy::ch_sys_get_screen_matrix_ptr(&mem())
    }
    /// `Ch_SysGetCameraMatrixPtr()`.
    Ch_SysGetCameraMatrixPtr @ 0x0083_5ba0 => extern "C" fn ch_sys_get_camera_matrix_ptr() -> u32 {
        sy::ch_sys_get_camera_matrix_ptr(&mem())
    }
    /// `Ch_SysSetScreenMatrix(m)`.
    Ch_SysSetScreenMatrix @ 0x0083_5e90 => extern "C" fn ch_sys_set_screen_matrix(m: u32) {
        sy::set_matrix(&mut mem(), sy::SCREEN_MATRIX, m);
    }
    /// `Ch_SysSetCameraMatrix(m)`.
    Ch_SysSetCameraMatrix @ 0x0083_5eb0 => extern "C" fn ch_sys_set_camera_matrix(m: u32) {
        sy::set_matrix(&mut mem(), sy::CAMERA_MATRIX, m);
    }
    /// `Ch_TextureAnimGetHandle(id)`.
    Ch_TextureAnimGetHandle @ 0x0083_5ed0 => extern "C" fn ch_texture_anim_get_handle(id: u32) -> u32 {
        h::ch_texture_anim_get_handle(&mem(), id)
    }
    /// `Ch_MotionGetId(chr)`.
    Ch_MotionGetId @ 0x0083_71a0 => extern "C" fn ch_motion_get_id(chr: u32) -> u32 {
        p::ch_motion_get_id(&mem(), chr)
    }
    /// `Ch_GetKeepFps(chr)`.
    Ch_GetKeepFps @ 0x0083_92a0 => extern "C" fn ch_get_keep_fps(chr: u32) -> u32 {
        p::ch_get_keep_fps(&mem(), chr)
    }
    /// `Ch_GetWaitMotion(chr)`.
    Ch_GetWaitMotion @ 0x0083_92c0 => extern "C" fn ch_get_wait_motion(chr: u32) -> i32 {
        p::ch_get_wait_motion(&mem(), chr)
    }
    /// `Ch_MotionGetFrame(chr)`.
    Ch_MotionGetFrame @ 0x0083_9860 => extern "C" fn ch_motion_get_frame(chr: u32) -> u32 {
        p::ch_motion_get_frame(&mem(), chr)
    }
    /// `Ch_MotionGetHalf()`.
    Ch_MotionGetHalf @ 0x0083_9870 => extern "C" fn ch_motion_get_half() -> u32 {
        sy::ch_motion_get_half(&mem())
    }
    /// `Ch_MotionSetHalf(v)`.
    Ch_MotionSetHalf @ 0x0083_9950 => extern "C" fn ch_motion_set_half(v: u32) {
        sy::ch_motion_set_half(&mut mem(), v);
    }
    /// `Ch_MotionSetLoop(chr, n)`.
    Ch_MotionSetLoop @ 0x0083_9960 => extern "C" fn ch_motion_set_loop(chr: u32, n: u32) {
        p::ch_motion_set_loop(&mut mem(), chr, n);
    }
    /// `Ch_SetKeepFps(chr, on)` -> old bit.
    Ch_SetKeepFps @ 0x0083_9b50 => extern "C" fn ch_set_keep_fps(chr: u32, on: u32) -> u32 {
        p::ch_set_keep_fps(&mut mem(), chr, on)
    }
    /// `Fixed12_ToFloat(v)`: 4.12 fixed to float.
    Fixed12_ToFloat @ 0x0083_9dd0 => extern "C" fn fixed12_to_float(v: u32) -> f32 {
        ffx_chara::fixed::fixed12_to_float(v)
    }
    /// `Fixed8_ToFloat(v)`: 8.8 fixed to float.
    Fixed8_ToFloat @ 0x0083_9e00 => extern "C" fn fixed8_to_float(v: u32) -> f32 {
        ffx_chara::fixed::fixed8_to_float(v)
    }
    /// `Angle12_Lerp(a, b, t)`: 12-bit angles the short way.
    Angle12_Lerp @ 0x0083_9e30 => extern "C" fn angle12_lerp(a: i32, b: i32, t: i32) -> i32 {
        ffx_chara::fixed::angle12_lerp(a, b, t)
    }

    // ---- cluts and texture data
    /// `Ch_ClutEnable(chr, on)`.
    Ch_ClutEnable @ 0x0083_c940 => extern "C" fn ch_clut_enable(chr: u32, on: u32) {
        tx::ch_clut_enable(&mut mem(), chr, on);
    }
    /// `Ch_ClutGetNb(chr)`.
    Ch_ClutGetNb @ 0x0083_c9a0 => extern "C" fn ch_clut_get_nb(chr: u32) -> i32 {
        tx::ch_clut_get_nb(&mem(), chr)
    }
    /// `Ch_ClutGetOrgPtr(chr, i)`.
    Ch_ClutGetOrgPtr @ 0x0083_c9b0 => extern "C" fn ch_clut_get_org_ptr(chr: u32, i: u32) -> u32 {
        tx::ch_clut_get_org_ptr(&mem(), chr, i)
    }
    /// `Ch_ClutGetPtr(chr, i)`.
    Ch_ClutGetPtr @ 0x0083_c9d0 => extern "C" fn ch_clut_get_ptr(chr: u32, i: u32) -> u32 {
        tx::ch_clut_get_ptr(&mem(), chr, i)
    }
    /// `Ch_ClutGetSize(chr, i)`.
    Ch_ClutGetSize @ 0x0083_c9f0 => extern "C" fn ch_clut_get_size(chr: u32, i: u32) -> i32 {
        tx::ch_clut_get_size(&mem(), chr, i)
    }
    /// `Ch_GetLightValue(chr)`.
    Ch_GetLightValue @ 0x0083_cb90 => extern "C" fn ch_get_light_value(chr: u32) -> u32 {
        tx::ch_get_light_value(chr)
    }
    /// `Ch_TexGetClut(chr, i, &out)`.
    Ch_TexGetClut @ 0x0083_cd20 => extern "C" fn ch_tex_get_clut(chr: u32, i: u32, out: u32) -> u32 {
        tx::ch_tex_get_clut(&mut mem(), chr, i, out)
    }
    /// `Ch_TexGetImage(chr, i, &w, &h)`.
    Ch_TexGetImage @ 0x0083_cd50 => extern "C" fn ch_tex_get_image(chr: u32, i: u32, w: u32, h: u32) -> u32 {
        tx::ch_tex_get_image(&mut mem(), chr, i, w, h)
    }
    /// `Ch_TexGetNbClut(chr)`.
    Ch_TexGetNbClut @ 0x0083_cd90 => extern "C" fn ch_tex_get_nb_clut(chr: u32) -> u32 {
        tx::ch_tex_get_nb_clut(&mem(), chr)
    }
    /// `Ch_TexGetNbImage(chr)`.
    Ch_TexGetNbImage @ 0x0083_cdb0 => extern "C" fn ch_tex_get_nb_image(chr: u32) -> u32 {
        tx::ch_tex_get_nb_image(&mem(), chr)
    }

    // ---- map ids
    /// `Ch_IdGetDictionary(chr)`.
    Ch_IdGetDictionary @ 0x0083_d7a0 => extern "C" fn ch_id_get_dictionary(chr: u32) -> u32 {
        id::ch_id_get_field(&mem(), chr, 11)
    }
    /// `Ch_IdGetEffect(chr)`.
    Ch_IdGetEffect @ 0x0083_d7c0 => extern "C" fn ch_id_get_effect(chr: u32) -> u32 {
        id::ch_id_get_field(&mem(), chr, 9)
    }
    /// `Ch_IdGetEffectTable(chr)`.
    Ch_IdGetEffectTable @ 0x0083_d7e0 => extern "C" fn ch_id_get_effect_table(chr: u32) -> u32 {
        id::ch_id_get_effect_table(&mem(), chr)
    }
    /// `Ch_IdGetEncount(chr)`.
    Ch_IdGetEncount @ 0x0083_d820 => extern "C" fn ch_id_get_encount(chr: u32) -> u32 {
        id::ch_id_get_field(&mem(), chr, 7)
    }
    /// `Ch_IdGetMap(chr)`.
    Ch_IdGetMap @ 0x0083_d840 => extern "C" fn ch_id_get_map(chr: u32) -> u32 {
        id::ch_id_get_map(&mem(), chr)
    }
    /// `Ch_IdGetMapB(chr)`.
    Ch_IdGetMapB @ 0x0083_d860 => extern "C" fn ch_id_get_map_b(chr: u32) -> u32 {
        id::ch_id_get_map_b(&mem(), chr)
    }
    /// `Ch_IdGetSound(chr)`.
    Ch_IdGetSound @ 0x0083_d900 => extern "C" fn ch_id_get_sound(chr: u32) -> u32 {
        id::ch_id_get_field(&mem(), chr, 15)
    }
    /// `Ch_IdGetWater(chr)`.
    Ch_IdGetWater @ 0x0083_d920 => extern "C" fn ch_id_get_water(chr: u32) -> u32 {
        id::ch_id_get_field(&mem(), chr, 13)
    }
    /// `Ch_IdSetDefPid(chr)`.
    Ch_IdSetDefPid @ 0x0083_d940 => extern "C" fn ch_id_set_def_pid(chr: u32) {
        id::ch_id_set_def_pid(&mut mem(), chr);
    }

    // ---- effect packets
    /// `Ch_EffectPacketGetSize()`.
    Ch_EffectPacketGetSize @ 0x0084_5700 => extern "C" fn ch_effect_packet_get_size() -> u32 {
        sy::ch_effect_packet_get_size(&mem())
    }
    /// `Ch_EffectPacketStoreEND()`.
    Ch_EffectPacketStoreEND @ 0x0084_57d0 => extern "C" fn ch_effect_packet_store_end() {
        sy::ch_effect_packet_store_end(&mut mem());
    }
    /// `Ch_EffectPacketStoreMAT(v)`.
    Ch_EffectPacketStoreMAT @ 0x0084_5800 => extern "C" fn ch_effect_packet_store_mat(v: u32) {
        sy::set_u16(&mut mem(), sy::PACKET_MAT, v);
    }
    /// `Ch_EffectPacketStorePRG(v)`.
    Ch_EffectPacketStorePRG @ 0x0084_5810 => extern "C" fn ch_effect_packet_store_prg(v: u32) {
        sy::set_u16(&mut mem(), sy::PACKET_PRG, v);
    }

    // ---- motion lerps
    /// `Chr_Lerp(a, b, t)`.
    Chr_Lerp @ 0x0083_9cb0 => extern "C" fn chr_lerp(a: f32, b: f32, t: f32) -> f32 {
        mo::lerp(a, b, t)
    }
    /// `Chr_LerpAngleDamped(a, b, x, lo, k)`.
    Chr_LerpAngleDamped @ 0x0083_9cd0 => extern "C" fn chr_lerp_angle_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
        mo::lerp_angle_damped(a, b, x, lo, k)
    }
    /// `Chr_LerpDamped(a, b, x, lo, k)`.
    Chr_LerpDamped @ 0x0083_9d70 => extern "C" fn chr_lerp_damped(a: f32, b: f32, x: f32, lo: f32, k: f32) -> f32 {
        mo::lerp_damped(a, b, x, lo, k)
    }
    /// `AngleTurn_Lerp(a, b, t)`.
    AngleTurn_Lerp @ 0x0083_9e80 => extern "C" fn angle_turn_lerp(a: u32, b: u32, t: u32) -> f32 {
        mo::angle_turn_lerp(a, b, t)
    }
    /// `Ch_MotionGetLoop(chr)`.
    Ch_MotionGetLoop @ 0x0083_9880 => extern "C" fn ch_motion_get_loop(chr: u32) -> i32 {
        mo::bridge::ch_motion_get_loop(&mem(), chr)
    }

    // ---- attachment points
    /// `Ch_BindToElem(chr, parent, id)`.
    Ch_BindToElem @ 0x0083_2630 => extern "C" fn ch_bind_to_elem(chr: u32, parent: u32, id: u32) -> u32 {
        at::ch_bind_to_elem(&mut mem(), chr, parent, id)
    }
    /// `Ch_ElemGetAttr(chr, id, &elem, &offset)`.
    Ch_ElemGetAttr @ 0x0083_3a70 => extern "C" fn ch_elem_get_attr(chr: u32, id: u32, elem: u32, out: u32) -> u32 {
        at::ch_elem_get_attr(&mut mem(), chr, id, elem, out)
    }
    /// `Ch_ElemGetDirect(chr, id)`.
    Ch_ElemGetDirect @ 0x0083_3bf0 => extern "C" fn ch_elem_get_direct(chr: u32, id: u32) -> u32 {
        at::ch_elem_get_direct(&mem(), chr, id)
    }
    /// `Ch_OptposGet(chr, i, out)`.
    Ch_OptposGet @ 0x0083_54f0 => extern "C" fn ch_optpos_get(chr: u32, i: u32, out: u32) {
        at::ch_optpos_get(&mut mem(), chr, i, out);
    }
    /// `Ch_OptposGetBase(chr, out)`.
    Ch_OptposGetBase @ 0x0083_55e0 => extern "C" fn ch_optpos_get_base(chr: u32, out: u32) {
        at::ch_optpos_get_base(&mut mem(), chr, out);
    }
    /// `Ch_OptposGetElement(chr, id, &offset)`.
    Ch_OptposGetElement @ 0x0083_5660 => extern "C" fn ch_optpos_get_element(chr: u32, id: u32, out: u32) -> u32 {
        at::ch_optpos_get_element(&mut mem(), chr, id, out)
    }

    // ---- cloth
    /// `Ch_ClothReset(chr)`.
    Ch_ClothReset @ 0x0082_4650 => extern "C" fn ch_cloth_reset(chr: u32) {
        cl::ch_cloth_reset(&mut mem(), chr);
    }
    /// `Ch_ClothReset2(chr)`.
    Ch_ClothReset2 @ 0x0082_4670 => extern "C" fn ch_cloth_reset2(chr: u32) {
        cl::ch_cloth_reset2(&mut mem(), chr);
    }
    /// `Ch_ClothSetAtariJimenCheck(chr)`.
    Ch_ClothSetAtariJimenCheck @ 0x0082_4800 => extern "C" fn ch_cloth_set_atari_jimen_check(chr: u32) {
        cl::ch_cloth_set_atari_jimen_check(&mut mem(), chr);
    }
    /// `Ch_ShouldDisableClothForMotion(chr, model, part)`.
    Ch_ShouldDisableClothForMotion @ 0x0082_4a60 => extern "C" fn ch_should_disable_cloth_for_motion(chr: u32, a: u32, b: u32) -> u32 {
        cl::ch_should_disable_cloth_for_motion(&mem(), chr, a, b)
    }
    /// `Ch_UseLargeCollisionRad(chr, model, part)`.
    Ch_UseLargeCollisionRad @ 0x0082_4b40 => extern "C" fn ch_use_large_collision_rad(chr: u32, a: u32, b: u32) -> f32 {
        cl::ch_use_large_collision_rad(&mem(), chr, a, b)
    }
    /// `Ch_UseAnchorVertex(chr, model, v)`.
    Ch_UseAnchorVertex @ 0x0083_16d0 => extern "C" fn ch_use_anchor_vertex(chr: u32, model: u32, v: u32) -> u32 {
        cl::ch_use_anchor_vertex(&mem(), chr, model, v)
    }
    /// `Ch_ClothResetAll()`.
    Ch_ClothResetAll @ 0x0083_3800 => extern "C" fn ch_cloth_reset_all() {
        cl::ch_cloth_reset_all(&mut mem());
    }
    /// `Ch_ClothResetRelation(parent)`.
    Ch_ClothResetRelation @ 0x0083_3860 => extern "C" fn ch_cloth_reset_relation(parent: u32) {
        cl::ch_cloth_reset_relation(&mut mem(), parent);
    }

    // ---- every character, normals
    /// `Ch_SysGetNbUse()`.
    Ch_SysGetNbUse @ 0x0082_85e0 => extern "C" fn ch_sys_get_nb_use() -> u32 {
        h::ch_sys_get_nb_use(&mem())
    }
    /// `Ch_AllReserveCalcNorm()`.
    Ch_AllReserveCalcNorm @ 0x0083_2550 => extern "C" fn ch_all_reserve_calc_norm() {
        h::ch_all_reserve_calc_norm(&mut mem());
    }
    /// `Ch_AllSetCalcNorm(on)`.
    Ch_AllSetCalcNorm @ 0x0083_25b0 => extern "C" fn ch_all_set_calc_norm(on: u32) {
        h::ch_all_set_calc_norm(&mut mem(), on);
    }
    /// `Ch_SetCalcNorm2(chr, on)`.
    Ch_SetCalcNorm2_00835780 @ 0x0083_5780 => extern "C" fn ch_set_calc_norm2(chr: u32, on: u32) -> u32 {
        p::ch_set_calc_norm2(&mut mem(), chr, on)
    }

    // ---- textures
    /// `Ch_GetBigImageSize(chr, &w, &h, rows)`.
    Ch_GetBigImageSize @ 0x0083_3e60 => extern "C" fn ch_get_big_image_size(chr: u32, w: u32, h: u32, rows: u32) {
        tx::ch_get_big_image_size(&mut mem(), chr, w, h, rows);
    }
    /// `Ch_ClutDisposeCheck()`.
    Ch_ClutDisposeCheck @ 0x0083_c910 => extern "C" fn ch_clut_dispose_check() -> u32 {
        tx::ch_clut_dispose_check(&mem())
    }
    /// `Ch_TextureSendSpecular(out)`.
    Ch_TextureSendSpecular @ 0x0083_cf80 => extern "C" fn ch_texture_send_specular(out: u32) -> u32 {
        tx::ch_texture_send_specular(&mut mem(), out)
    }

    // ---- mapping loaded data
    /// `Ch_ArcMapping(arc)`.
    Ch_ArcMapping @ 0x0082_56f0 => extern "C" fn ch_arc_mapping(arc: u32) {
        mb::ch_arc_mapping(&mut mem(), arc);
    }
    /// `Ch_CbArcMapping(blk, arc)`.
    Ch_CbArcMapping @ 0x0082_5ee0 => extern "C" fn ch_cb_arc_mapping(blk: u32, arc: u32) {
        mb::ch_cb_arc_mapping(&mut mem(), blk, arc);
    }
    /// `Ch_CbMapping(chr, blk)`.
    Ch_CbMapping @ 0x0082_6070 => extern "C" fn ch_cb_mapping(chr: u32, blk: u32) {
        mb::ch_cb_mapping(&mut mem(), chr, blk);
    }
    /// `Ch_ElemRemapping(chr, delta)`.
    Ch_ElemRemapping @ 0x0082_6980 => extern "C" fn ch_elem_remapping(chr: u32, d: u32) {
        mb::ch_elem_remapping(&mut mem(), chr, d);
    }
    /// `Ch_ModelMapping(header)`.
    Ch_ModelMapping @ 0x0082_7590 => extern "C" fn ch_model_mapping(header: u32) {
        mb::ch_model_mapping(&mut mem(), header);
    }
    /// `Chr_ModelEntry(i, base, version)`.
    Chr_ModelEntry @ 0x0082_8be0 => extern "C" fn chr_model_entry(i: u32, base: u32, version: u32) -> u32 {
        mp::model_entry(i, base, version)
    }
    /// `Ch_MoveMgrp(chr, delta)`.
    Ch_MoveMgrp @ 0x0083_7590 => extern "C" fn ch_move_mgrp(chr: u32, d: u32) {
        mb::ch_move_mgrp(&mut mem(), chr, d);
    }
    /// `Ch_MotionRemappingFcurve(chr, delta)`.
    Ch_MotionRemappingFcurve @ 0x0083_9890 => extern "C" fn ch_motion_remapping_fcurve(chr: u32, d: u32) {
        mb::ch_motion_remapping_fcurve(&mut mem(), chr, d);
    }
    /// `Ch_ClutMapping(chr, copy)`.
    Ch_ClutMapping @ 0x0083_ca20 => extern "C" fn ch_clut_mapping(chr: u32, copy: u32) {
        mb::ch_clut_mapping(&mut mem(), chr, copy);
    }
    /// `Ch_TexDataMapping(tex)`.
    Ch_TexDataMapping @ 0x0083_cba0 => extern "C" fn ch_tex_data_mapping(tex: u32) {
        mb::ch_tex_data_mapping(&mut mem(), tex);
    }

    // ---- settings
    /// `Ch_SetAllocateErr()`.
    Ch_SetAllocateErr @ 0x0082_85d0 => extern "C" fn ch_set_allocate_err() {
        sy::ch_set_allocate_err(&mut mem());
    }
    /// `Ch_SetDrawIcon(on)`.
    Ch_SetDrawIcon @ 0x0083_0c70 => extern "C" fn ch_set_draw_icon(on: u32) {
        sy::set_bit0(&mut mem(), sy::DRAW_ICON, on);
    }
    /// `Ch_SetDrawIcon2(on)`.
    Ch_SetDrawIcon2 @ 0x0083_0c80 => extern "C" fn ch_set_draw_icon2(on: u32) {
        sy::set_bit0(&mut mem(), sy::DRAW_ICON2, on);
    }
    /// `Ch_SysSet7e4(v)`.
    Ch_SysSet7e4 @ 0x0083_0d00 => extern "C" fn ch_sys_set_7e4(v: u32) {
        sy::SYS_7E4.set(&mut mem(), v);
    }
    /// `Ch_SysSetSwitch7d8(on)`: returns the old value.
    Ch_SysSetSwitch7d8 @ 0x0083_17b0 => extern "C" fn ch_sys_set_switch_7d8(on: u32) -> i32 {
        sy::ch_sys_set_switch_7d8(&mut mem(), on)
    }
    /// `Ch_DrawGetShadeByLight()`.
    Ch_DrawGetShadeByLight @ 0x0083_17d0 => extern "C" fn ch_draw_get_shade_by_light() -> i32 {
        sy::get_i8(&mem(), p::SHADE_BY_LIGHT)
    }
    /// `Ch_SysSet6e0(v)`.
    Ch_SysSet6e0 @ 0x0083_20e0 => extern "C" fn ch_sys_set_6e0(v: u32) {
        sy::SYS_6E0.set(&mut mem(), v);
    }
    /// `Ch_FunctionHistoryClear()`.
    Ch_FunctionHistoryClear @ 0x0083_3760 => extern "C" fn ch_function_history_clear() {
        sy::HISTORY_9E8.set(&mut mem(), 0);
    }

    // ---- more accessors
    /// `Ch_WindGet(chr, &a, &b, &c)`.
    Ch_WindGet @ 0x0082_4c30 => extern "C" fn ch_wind_get(chr: u32, a: u32, b: u32, c: u32) {
        c::ch_wind_get(&mut mem(), chr, a, b, c);
    }
    /// `Ch_WindGetEnable(chr)`.
    Ch_WindGetEnable @ 0x0082_4c70 => extern "C" fn ch_wind_get_enable(chr: u32) -> u32 {
        c::ch_wind_get_enable(&mem(), chr)
    }
    /// `Chr_GetU16(p)`.
    Chr_GetU16 @ 0x0082_ba20 => extern "C" fn chr_get_u16(p: u32) -> u32 {
        c::chr_get_u16(&mem(), p)
    }
    /// `Ch_CalcHeight(chr)`.
    Ch_CalcHeight @ 0x0083_0b20 => extern "C" fn ch_calc_height(chr: u32) {
        c::ch_calc_height(&mut mem(), chr);
    }
    /// `Ch_Set7f8(chr, a, b)`.
    Ch_Set7f8 @ 0x0083_3940 => extern "C" fn ch_set_7f8(chr: u32, a: u32, b: u32) {
        c::ch_set_7f8(&mut mem(), chr, a, b);
    }

    // ---- every character
    /// `Ch_UnbindChildren(chr)`.
    Ch_UnbindChildren @ 0x0082_6150 => extern "C" fn ch_unbind_children(chr: u32) {
        h::ch_unbind_children(&mut mem(), chr);
    }
    /// `Ch_HandleFromId(id)`.
    Ch_HandleFromId @ 0x0082_61f0 => extern "C" fn ch_handle_from_id(id: u32) -> u32 {
        h::ch_handle_from_id(&mem(), id)
    }
    /// `Ch_CompareDrawOrder(&a, &b)`.
    Ch_CompareDrawOrder @ 0x0083_20f0 => extern "C" fn ch_compare_draw_order(a: u32, b: u32) -> u32 {
        h::ch_compare_draw_order(&mem(), a, b)
    }
    /// `Ch_AllClearFlags24()`.
    Ch_AllClearFlags24 @ 0x0083_2db0 => extern "C" fn ch_all_clear_flags24() {
        h::ch_all_clear_flags24(&mut mem());
    }

    // ---- elements and normals
    /// `Ch_SetCalcNorm2(holder)`.
    Ch_SetCalcNorm2 @ 0x0079_69c0 => extern "C" fn ch_set_calc_norm2_holder(p: u32) -> u32 {
        p::ch_set_calc_norm2_holder(&mut mem(), p)
    }
    /// `Ch_IsNotExceptionId(id)`.
    Ch_IsNotExceptionId @ 0x0083_1780 => extern "C" fn ch_is_not_exception_id(id: u32) -> u32 {
        p::ch_is_not_exception_id(&mem(), id)
    }
    /// `Ch_ElemInMotion(chr, e)`.
    Ch_ElemInMotion @ 0x0083_42c0 => extern "C" fn ch_elem_in_motion(chr: u32, e: u32) -> u32 {
        p::ch_elem_in_motion(&mem(), chr, e)
    }

    // ---- motion
    /// `Chr_RangeClass(a, b, x, y)`.
    Chr_RangeClass @ 0x0083_1330 => extern "C" fn chr_range_class(a: i32, b: i32, x: i32, y: i32) -> u32 {
        mo::range_class(a, b, x, y)
    }
    /// `Ch_MotionSetNull(chr)`.
    Ch_MotionSetNull @ 0x0083_7540 => extern "C" fn ch_motion_set_null(chr: u32) {
        mo::bridge::ch_motion_set_null(&mut mem(), chr);
    }
    /// `Ch_MotionSet737(chr, v)`.
    Ch_MotionSet737 @ 0x0083_7570 => extern "C" fn ch_motion_set_737(chr: u32, v: u32) -> u32 {
        mo::bridge::ch_motion_set_737(&mut mem(), chr, v)
    }
    /// `Ch_SeqLink()`.
    Ch_SeqLink @ 0x0083_8170 => extern "C" fn ch_seq_link() {
        mo::bridge::ch_seq_link(&mut mem());
    }
    /// `Ch_MotionLoopFrame(chr)`.
    Ch_MotionLoopFrame @ 0x0083_92d0 => extern "C" fn ch_motion_loop_frame(chr: u32) -> u32 {
        mo::bridge::ch_motion_loop_frame(&mut mem(), chr)
    }
    /// `Ch_MemRelinkBack(heap)`.
    Ch_MemRelinkBack @ 0x0083_c6a0 => extern "C" fn ch_mem_relink_back(heap: u32) {
        mb::ch_mem_relink_back(&mut mem(), heap);
    }

    // ---- matrices
    /// `Ch_GetRootScaledMatrix(out, chr)`.
    Ch_GetRootScaledMatrix @ 0x0082_6a80 => extern "C" fn ch_get_root_scaled_matrix(out: u32, chr: u32) {
        mx::ch_get_root_scaled_matrix(&mut mem(), out, chr);
    }
    /// `Ch_ElemInit(e)`.
    Ch_ElemInit @ 0x0082_8b60 => extern "C" fn ch_elem_init(e: u32) -> u32 {
        mx::ch_elem_init(&mut mem(), e)
    }
    /// `Ch_ModelGetBaseMatrix(chr, i, out)`.
    Ch_ModelGetBaseMatrix @ 0x0083_0500 => extern "C" fn ch_model_get_base_matrix(chr: u32, i: u32, out: u32) -> u32 {
        mx::ch_model_get_base_matrix(&mut mem(), chr, i, out)
    }
    /// `Ch_ModelGetMatrix(chr, i, out)`.
    Ch_ModelGetMatrix @ 0x0083_0550 => extern "C" fn ch_model_get_matrix(chr: u32, i: u32, out: u32) {
        mx::ch_model_get_matrix(&mut mem(), chr, i, out);
    }
    /// `Ch_GetRootGlobalMatrix(chr, out)`.
    Ch_GetRootGlobalMatrix @ 0x0083_3f80 => extern "C" fn ch_get_root_global_matrix(chr: u32, out: u32) -> u32 {
        mx::ch_get_root_global_matrix(&mut mem(), chr, out)
    }
    /// `Ch_GetRootLightMatrix(chr, out)`.
    Ch_GetRootLightMatrix @ 0x0083_3fc0 => extern "C" fn ch_get_root_light_matrix(chr: u32, out: u32) -> u32 {
        mx::ch_get_root_light_matrix(&mut mem(), chr, out)
    }

    // ---- texture animation
    /// `chr_texanim_init()`.
    chr_texanim_init @ 0x0077_c870 => extern "C" fn chr_texanim_init() {
        ta::chr_texanim_init(&mut mem());
    }
    /// `chr_texanim_memory_move(src, i)`.
    chr_texanim_memory_move @ 0x0077_c900 => extern "C" fn chr_texanim_memory_move(src: u32, i: u32) {
        ta::chr_texanim_memory_move(&mut mem(), src, i);
    }
    /// `chr_texanim_clut_swizzle(dst, src, which)`.
    chr_texanim_clut_swizzle @ 0x0077_ca10 => extern "C" fn chr_texanim_clut_swizzle(dst: u32, src: u32, which: u32) {
        ta::chr_texanim_clut_swizzle(&mut mem(), dst, src, which);
    }
    /// `chr_texanim_count(&n, &max)`.
    chr_texanim_count @ 0x0077_cdc0 => extern "C" fn chr_texanim_count(n: u32, max: u32) {
        ta::chr_texanim_count(&mut mem(), n, max);
    }
    /// `chr_texanim_memory_move_new(src, i)`.
    chr_texanim_memory_move_new @ 0x0077_f970 => extern "C" fn chr_texanim_memory_move_new(src: u32, i: u32) {
        ta::chr_texanim_memory_move_new(&mut mem(), src, i);
    }
    /// `chr_texanim_memory_move_old(src, i)`.
    chr_texanim_memory_move_old @ 0x0078_03a0 => extern "C" fn chr_texanim_memory_move_old(src: u32, i: u32) {
        ta::chr_texanim_memory_move_old(&mut mem(), src, i);
    }
    /// `Ch_TextureSetTex0(chr)`.
    Ch_TextureSetTex0 @ 0x0083_13f0 => extern "C" fn ch_texture_set_tex0(chr: u32) {
        ta::ch_texture_set_tex0(&mut mem(), chr);
    }
    /// `Ch_TextureSetClut(img, tex)`.
    Ch_TextureSetClut @ 0x0083_d1e0 => extern "C" fn ch_texture_set_clut(img: u32, tex: u32) {
        ta::ch_texture_set_clut(&mut mem(), img, tex);
    }

    // ---- round 4
    /// `Ch_OptposCalc(chr)`.
    Ch_OptposCalc @ 0x0083_50f0 => extern "C" fn ch_optpos_calc(chr: u32) {
        at::ch_optpos_calc(&mut mem(), chr);
    }
    /// `Ch_SetSequence(chr, id)`.
    Ch_SetSequence @ 0x0083_7ac0 => extern "C" fn ch_set_sequence(chr: u32, id: u32) -> u32 {
        mo::bridge::ch_set_sequence(&mut mem(), chr, id)
    }
    /// `Ch_UpdateClothMatrix(chr)`.
    Ch_UpdateClothMatrix @ 0x0083_14c0 => extern "C" fn ch_update_cloth_matrix(chr: u32) {
        cl::ch_update_cloth_matrix(&mut mem(), chr);
    }
    /// `Ch_EffectPacketStorePoly(a, b, c)`.
    Ch_EffectPacketStorePoly @ 0x0084_5820 => extern "C" fn ch_effect_packet_store_poly(a: u32, b: u32, c: u32) {
        sy::ch_effect_packet_store_poly(&mut mem(), a, b, c);
    }
    /// `Chr_FracStep(a, b, n)`.
    Chr_FracStep @ 0x0083_9f50 => extern "C" fn chr_frac_step(a: f32, b: f32, n: i32) -> f32 {
        mo::bridge::chr_frac_step(&mem(), a, b, n)
    }
    /// `Chr_AddVector3(dst, a, b)`.
    Chr_AddVector3 @ 0x0082_4ce0 => extern "C" fn chr_add_vector3(dst: u32, a: u32, b: u32) {
        c::chr_add_vector3(&mut mem(), dst, a, b);
    }
    /// `Ch_WindSetEnable(chr, on)`.
    Ch_WindSetEnable @ 0x0082_4cc0 => extern "C" fn ch_wind_set_enable(chr: u32, on: u32) {
        c::ch_wind_set_enable(&mut mem(), chr, on);
    }
    /// `Chr_CopyVector4(dst, src)`.
    Chr_CopyVector4 @ 0x0082_4d10 => extern "C" fn chr_copy_vector4(dst: u32, src: u32) {
        c::chr_copy_vector4(&mut mem(), dst, src);
    }
    /// `Ch_ElemGetBaseMatrix(chr, e, out)`.
    Ch_ElemGetBaseMatrix @ 0x0083_3b80 => extern "C" fn ch_elem_get_base_matrix(chr: u32, e: u32, out: u32) -> u32 {
        mx::ch_elem_get_base_matrix(&mut mem(), chr, e, out)
    }
    /// `Ch_ElemGetMatrix(chr, e, out)`.
    Ch_ElemGetMatrix @ 0x0083_3c30 => extern "C" fn ch_elem_get_matrix(chr: u32, e: u32, out: u32) {
        mx::ch_elem_get_matrix(&mut mem(), chr, e, out);
    }
    /// `Ch_ElemGetPos(chr, e, v, out)`.
    Ch_ElemGetPos @ 0x0083_3d10 => extern "C" fn ch_elem_get_pos(chr: u32, e: u32, v: u32, out: u32) {
        mx::ch_elem_get_pos(&mut mem(), chr, e, v, out);
    }
    /// `Chr_WrapAngleDiscard(a)`: wraps a copy and drops it.
    Chr_WrapAngleDiscard @ 0x0083_a010 => extern "C" fn chr_wrap_angle_discard(a: f32) {
        let _ = ffx_chara::chr::wrap_angle(a);
    }
    /// `Ch_TextureSetImage(clut, tex)`.
    Ch_TextureSetImage @ 0x0083_d220 => extern "C" fn ch_texture_set_image(clut: u32, tex: u32) {
        ta::ch_texture_set_image(&mut mem(), clut, tex);
    }
    /// `Ch_TextureSetRegs(img, tex)`.
    Ch_TextureSetRegs @ 0x0083_d250 => extern "C" fn ch_texture_set_regs(img: u32, tex: u32) {
        ta::ch_texture_set_regs(&mut mem(), img, tex);
    }
    /// `Ch_WindSet(chr, a, b, c)`.
    Ch_WindSet @ 0x0082_4c90 => extern "C" fn ch_wind_set(chr: u32, a: f32, b: f32, c: f32) {
        c::ch_wind_set(&mut mem(), chr, a, b, c);
    }
    /// `Chr_DotVector3(a, b, out)`.
    Chr_DotVector3 @ 0x0082_4d40 => extern "C" fn chr_dot_vector3(a: u32, b: u32, out: u32) {
        c::chr_dot_vector3(&mut mem(), a, b, out);
    }
    /// `Chr_ScaleVector3(dst, v, s)`.
    Chr_ScaleVector3 @ 0x0082_4d70 => extern "C" fn chr_scale_vector3(dst: u32, v: u32, s: f32) {
        c::chr_scale_vector3(&mut mem(), dst, v, s);
    }
    /// `Ch_OptposGetMatrix(chr, id, offset, out)`.
    Ch_OptposGetMatrix @ 0x0083_56b0 => extern "C" fn ch_optpos_get_matrix(chr: u32, id: u32, offset: u32, out: u32) {
        at::ch_optpos_get_matrix(&mut mem(), chr, id, offset, out);
    }
    /// `Ch_TextureReMapping(chr)`.
    Ch_TextureReMapping @ 0x0083_cdd0 => extern "C" fn ch_texture_re_mapping(chr: u32) {
        ta::ch_texture_re_mapping(&mut mem(), chr);
    }
    /// `Chr_SubVector3(dst, a, b)`.
    Chr_SubVector3 @ 0x0082_4da0 => extern "C" fn chr_sub_vector3(dst: u32, a: u32, b: u32) {
        c::chr_sub_vector3(&mut mem(), dst, a, b);
    }
    /// `Ch_MoveChrBlk(new, old)`.
    Ch_MoveChrBlk @ 0x0083_47a0 => extern "C" fn ch_move_chr_blk(new: u32, old: u32) {
        mb::ch_move_chr_blk(&mut mem(), new, old);
    }
}
