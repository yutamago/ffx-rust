//! `ffx-vfx`: particle and effect functions. All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::{et, fp, heap, op, opd, opus, ppp, prog, rand};

ported! {
    // ---- rand ----
    /// `op_random(x)`: a random value in `[-x, x)` from VU0's `R`.
    op_random @ 0x007e_6700 => extern "C" fn op_random(x: f32) -> f32 {
        rand::bridge::op_random(&mut mem(), x)
    }
    /// `op_random_p(x)`: a random value in `[0, x)`.
    op_random_p @ 0x007e_6780 => extern "C" fn op_random_p(x: f32) -> f32 {
        rand::bridge::op_random_p(&mut mem(), x)
    }

    // ---- ppp programs ----
    /// `pppKeDrctCon(obj, ctrl)`.
    pppKeDrctCon @ 0x0075_e470 => extern "C" fn ppp_ke_drct_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_drct_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeDrct(obj, key, ctrl)`.
    pppKeDrct @ 0x0075_e430 => extern "C" fn ppp_ke_drct(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_drct(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeMatSNCon(obj, ctrl)`.
    pppKeMatSNCon @ 0x0073_40f0 => extern "C" fn ppp_ke_mat_sn_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_mat_sn_con(&mut mem(), obj, ctrl);
    }
    /// `pppVtMimeCon(obj, ctrl)`.
    pppVtMimeCon @ 0x0072_ed70 => extern "C" fn ppp_vt_mime_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_vt_mime_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiLightEikyoCon(obj, ctrl)`.
    pppNeiLightEikyoCon @ 0x0075_d6c0 => extern "C" fn ppp_nei_light_eikyo_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_nei_light_eikyo_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThCon2(obj, ctrl)`.
    pppKeThCon2 @ 0x0073_64e0 => extern "C" fn ppp_ke_th_con2(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_th_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeThSftCon(obj, ctrl)`.
    pppKeThSftCon @ 0x0073_7130 => extern "C" fn ppp_ke_th_sft_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_th_sft_con(&mut mem(), obj, ctrl);
    }
    /// `pppColorCon(obj, ctrl)`.
    pppColorCon @ 0x0075_d240 => extern "C" fn ppp_color_con(obj: u32, ctrl: u32) {
        ppp::bridge::ppp_color_con(&mut mem(), obj, ctrl);
    }
    /// `pppColor(obj, key, ctrl)`.
    pppColor @ 0x0075_d1f0 => extern "C" fn ppp_color(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_color(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngle(obj, key, ctrl)`.
    pppAngle @ 0x0075_ce30 => extern "C" fn ppp_angle(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_angle(&mut mem(), obj, key, ctrl);
    }
    /// `pppNeiLightEikyo(obj, key, ctrl)`.
    pppNeiLightEikyo @ 0x0075_d5d0 => extern "C" fn ppp_nei_light_eikyo(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_nei_light_eikyo(&mut mem(), obj, key, ctrl);
    }
    /// `pppKeLnsLpSft(obj, key, ctrl)`.
    pppKeLnsLpSft @ 0x0075_9f30 => extern "C" fn ppp_ke_lns_lp_sft(obj: u32, key: u32, ctrl: u32) {
        ppp::bridge::ppp_ke_lns_lp_sft(&mut mem(), obj, key, ctrl);
    }
    /// `pppClearWait()`.
    pppClearWait @ 0x0076_b200 => extern "C" fn ppp_clear_wait() {
        ppp::bridge::ppp_clear_wait(&mut mem());
    }
    /// `pppGetRotMatrixX(out, angle)`.
    pppGetRotMatrixX @ 0x0072_c730 => extern "C" fn ppp_get_rot_matrix_x(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_x(&mut mem(), out, angle);
    }
    /// `pppGetRotMatrixY(out, angle)`.
    pppGetRotMatrixY @ 0x0072_c920 => extern "C" fn ppp_get_rot_matrix_y(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_y(&mut mem(), out, angle);
    }
    /// `pppGetRotMatrixZ(out, angle)`.
    pppGetRotMatrixZ @ 0x0072_cb10 => extern "C" fn ppp_get_rot_matrix_z(out: u32, angle: i32) {
        ppp::bridge::ppp_get_rot_matrix_z(&mut mem(), out, angle);
    }

    // ---- heaps ----
    /// `pppCreateHeap(heap, memory, size)`.
    pppCreateHeap @ 0x0072_c480 => extern "C" fn ppp_create_heap(h: u32, memory: u32, size: u32) {
        heap::bridge::ppp_create_heap(&mut mem(), h, memory, size);
    }
    /// `pppAlloc(heap, size)`.
    pppAlloc @ 0x0072_c3d0 => extern "C" fn ppp_alloc(h: u32, size: u32) -> u32 {
        heap::bridge::ppp_alloc(&mut mem(), h, size)
    }
    /// `pppFree(heap, data)`.
    pppFree @ 0x0072_c510 => extern "C" fn ppp_free(h: u32, data: u32) {
        heap::bridge::ppp_free(&mut mem(), h, data);
    }
    /// `pppFpGetHeapSize()`.
    pppFpGetHeapSize @ 0x0072_9290 => extern "C" fn ppp_fp_get_heap_size() -> u32 {
        heap::bridge::ppp_fp_get_heap_size(&mem())
    }

    // ---- field effects ----
    /// `Fp_Restart(id)`.
    Fp_Restart @ 0x0072_9700 => extern "C" fn fp_restart(id: u32) {
        fp::bridge::fp_restart(&mut mem(), id);
    }
    /// `Fp_RestartMode2(id)`.
    Fp_RestartMode2 @ 0x0072_9810 => extern "C" fn fp_restart_mode2(id: u32) {
        fp::bridge::fp_restart_mode2(&mut mem(), id);
    }
    /// `Fp_End(id)`.
    Fp_End @ 0x0072_9660 => extern "C" fn fp_end(id: u32) {
        fp::bridge::fp_end(&mut mem(), id);
    }
    /// `pppFpIDdispOff(id)`.
    pppFpIDdispOff @ 0x0072_9570 => extern "C" fn ppp_fp_id_disp_off(id: u32) {
        fp::bridge::ppp_fp_id_disp_off(&mut mem(), id);
    }
    /// `pppFpIDdispOn(id)`.
    pppFpIDdispOn @ 0x0072_95b0 => extern "C" fn ppp_fp_id_disp_on(id: u32) {
        fp::bridge::ppp_fp_id_disp_on(&mut mem(), id);
    }
    /// `pppFpGetMngPtr(id, &run, &disp)`.
    pppFpGetMngPtr @ 0x0072_92f0 => extern "C" fn ppp_fp_get_mng_ptr(id: u32, run: u32, disp: u32) {
        fp::bridge::ppp_fp_get_mng_ptr(&mut mem(), id, run, disp);
    }
    /// `pppFpResetStick(id)`.
    pppFpResetStick @ 0x0072_a120 => extern "C" fn ppp_fp_reset_stick(id: u32) {
        fp::bridge::ppp_fp_reset_stick(&mut mem(), id);
    }
    /// `pppFpSetStickMap(id, target)`.
    pppFpSetStickMap @ 0x0072_a590 => extern "C" fn ppp_fp_set_stick_map(id: u32, target: u32) {
        fp::bridge::ppp_fp_set_stick_map(&mut mem(), id, target);
    }
    /// `pppFpSetStickMapDObj(id, target)`.
    pppFpSetStickMapDObj @ 0x0072_a5e0 => extern "C" fn ppp_fp_set_stick_map_dobj(id: u32, target: u32) {
        fp::bridge::ppp_fp_set_stick_map_dobj(&mut mem(), id, target);
    }
    /// `pppFpStopStatus(stop)`.
    pppFpStopStatus @ 0x0072_a750 => extern "C" fn ppp_fp_stop_status(stop: u32) {
        fp::bridge::ppp_fp_stop_status(&mut mem(), stop);
    }
    /// `pppFpPointLightInit()`.
    pppFpPointLightInit @ 0x0072_9f70 => extern "C" fn ppp_fp_point_light_init() {
        fp::bridge::ppp_fp_point_light_init(&mut mem());
    }
    /// `pppFpSaveStat(out)`.
    pppFpSaveStat @ 0x0072_a180 => extern "C" fn ppp_fp_save_stat(out: u32) {
        fp::bridge::ppp_fp_save_stat(&mut mem(), out);
    }

    // ---- op layer ----
    /// `op_spr_alloc(n)`.
    op_spr_alloc @ 0x007e_6a00 => extern "C" fn op_spr_alloc(n: u32) {
        op::bridge::op_spr_alloc(&mut mem(), n);
    }
    /// `op_spr_free()`.
    op_spr_free @ 0x007e_6a60 => extern "C" fn op_spr_free() {
        op::bridge::op_spr_free(&mut mem());
    }
    /// `op_m_copy(dst, src)`.
    op_m_copy @ 0x007e_8010 => extern "C" fn op_m_copy(dst: u32, src: u32) {
        op::bridge::op_m_copy(&mut mem(), dst, src);
    }
    /// `op_unit_m_scale(m, s)`.
    op_unit_m_scale @ 0x007e_a700 => extern "C" fn op_unit_m_scale(m: u32, s: f32) {
        op::bridge::op_unit_m_scale(&mut mem(), m, s);
    }
    /// `op_func_ot_set(func, a, b, c, list)`.
    op_func_ot_set @ 0x007e_b500 => extern "C" fn op_func_ot_set(func: u32, a: u32, b: u32, c: u32, list: u32) -> i32 {
        op::bridge::op_func_ot_set(&mut mem(), func, a, b, c, list)
    }
    /// `op_func_ot_free(slot, list)`.
    op_func_ot_free @ 0x007e_b4a0 => extern "C" fn op_func_ot_free(slot: u32, list: u32) {
        op::bridge::op_func_ot_free(&mut mem(), slot, list);
    }
    /// `op_wot_triger(on)`.
    op_wot_triger @ 0x007e_c7d0 => extern "C" fn op_wot_triger(on: u32) {
        op::bridge::op_wot_triger(&mut mem(), on);
    }
    /// `op_pray_effect_check()`.
    op_pray_effect_check @ 0x007f_ce10 => extern "C" fn op_pray_effect_check() -> i32 {
        op::bridge::op_pray_effect_check(&mem())
    }
    /// `op_pad_set(pad, _, input)`.
    op_pad_set @ 0x007e_6610 => extern "C" fn op_pad_set(pad: u32, unused: u32, input: u32) {
        let _ = unused;
        op::bridge::op_pad_set(&mut mem(), pad, input);
    }
    /// `op_mdc_mem_transfer_cue_free(key)`.
    op_mdc_mem_transfer_cue_free @ 0x0080_3730 => extern "C" fn op_mdc_mem_transfer_cue_free(key: u32) {
        op::bridge::op_mdc_mem_transfer_cue_free(&mut mem(), key);
    }
    /// `op_mdc_mem_transfer_cue_free_all()`.
    op_mdc_mem_transfer_cue_free_all @ 0x0080_3760 => extern "C" fn op_mdc_mem_transfer_cue_free_all() {
        op::bridge::op_mdc_mem_transfer_cue_free_all(&mut mem());
    }
    /// `op_oef2_tex_clut_ovl_clear(mask)`.
    op_oef2_tex_clut_ovl_clear @ 0x007e_4e40 => extern "C" fn op_oef2_tex_clut_ovl_clear(mask: u32) {
        op::bridge::op_oef2_tex_clut_ovl_clear(&mut mem(), mask);
    }
    /// `op_oef2_tex_clut_cue_check(mask)`.
    op_oef2_tex_clut_cue_check @ 0x007e_46d0 => extern "C" fn op_oef2_tex_clut_cue_check(mask: u32) -> i32 {
        op::bridge::op_oef2_tex_clut_cue_check(&mem(), mask)
    }
    /// `op_oef2_clut_change(clut, v)`.
    op_oef2_clut_change @ 0x007e_45c0 => extern "C" fn op_oef2_clut_change(clut: u32, v: u32) -> u32 {
        op::bridge::op_oef2_clut_change(&mut mem(), clut, v)
    }
    /// `op_oef2_obj_a_get(base, part, n)`.
    op_oef2_obj_a_get @ 0x007e_3b40 => extern "C" fn op_oef2_obj_a_get(base: u32, part: u32, n: u32) -> u32 {
        op::bridge::op_oef2_obj_a_get(&mem(), base, part, n)
    }
    /// `op_oef2_stream_cue(cue, channel)`.
    op_oef2_stream_cue @ 0x0080_0390 => extern "C" fn op_oef2_stream_cue(cue: u32, channel: u32) -> i32 {
        op::bridge::op_oef2_stream_cue(&mut mem(), cue, channel)
    }

    // ---- et tables ----
    /// `op_get_chr_status_stone(h)`.
    op_get_chr_status_stone @ 0x007f_cc80 => extern "C" fn op_get_chr_status_stone(h: u32) -> u32 {
        et::bridge::op_get_chr_status_stone(&mem(), h)
    }
    /// `op_get_chrhd_by_chrno(no)`.
    op_get_chrhd_by_chrno @ 0x007f_cd10 => extern "C" fn op_get_chrhd_by_chrno(no: u32) -> u32 {
        et::bridge::op_get_chrhd_by_chrno(&mem(), no)
    }
    /// `op_get_chrhd_all_list(out)`.
    op_get_chrhd_all_list @ 0x007f_ccc0 => extern "C" fn op_get_chrhd_all_list(out: u32) -> u32 {
        et::bridge::op_get_chrhd_all_list(&mut mem(), out)
    }
    /// `op_et_battle_genko_counter_get()`.
    op_et_battle_genko_counter_get @ 0x007f_b240 => extern "C" fn op_et_battle_genko_counter_get() -> u32 {
        et::bridge::op_et_battle_genko_counter_get(&mem())
    }
    /// `op_et_bindeff_draw_on(h)`.
    op_et_bindeff_draw_on @ 0x007f_bab0 => extern "C" fn op_et_bindeff_draw_on(h: u32) -> u32 {
        et::bridge::op_et_bindeff_draw_on(&mut mem(), h)
    }
    /// `op_et_bindeff_draw_off(h)`.
    op_et_bindeff_draw_off @ 0x007f_ba40 => extern "C" fn op_et_bindeff_draw_off(h: u32) -> u32 {
        et::bridge::op_et_bindeff_draw_off(&mut mem(), h)
    }
    /// `op_et_bindeff2_draw_on(h)`.
    op_et_bindeff2_draw_on @ 0x007f_b5a0 => extern "C" fn op_et_bindeff2_draw_on(h: u32) -> u32 {
        et::bridge::op_et_bindeff2_draw_on(&mut mem(), h)
    }
    /// `op_et_bindeff2_walk_on(h)`.
    op_et_bindeff2_walk_on @ 0x007f_b830 => extern "C" fn op_et_bindeff2_walk_on(h: u32) {
        et::bridge::op_et_bindeff2_walk_on(&mut mem(), h);
    }
    /// `op_et_bindeff2_water_on(h)`.
    op_et_bindeff2_water_on @ 0x007f_b8e0 => extern "C" fn op_et_bindeff2_water_on(h: u32) {
        et::bridge::op_et_bindeff2_water_on(&mut mem(), h);
    }
    /// `op_et_bindeff2_breath_on(h)`.
    op_et_bindeff2_breath_on @ 0x007f_b4c0 => extern "C" fn op_et_bindeff2_breath_on(h: u32) {
        et::bridge::op_et_bindeff2_breath_on(&mut mem(), h);
    }
    /// `op_et_bindeff_off_signal(h)`.
    op_et_bindeff_off_signal @ 0x007f_bcd0 => extern "C" fn op_et_bindeff_off_signal(h: u32) {
        et::bridge::op_et_bindeff_off_signal(&mut mem(), h);
    }
    /// `op_et_eff_off_signal(id)`.
    op_et_eff_off_signal @ 0x007f_c450 => extern "C" fn op_et_eff_off_signal(id: u32) {
        et::bridge::op_et_eff_off_signal(&mut mem(), id);
    }
    /// `op_et_weapon_afterimage_fade(id)`.
    op_et_weapon_afterimage_fade @ 0x007f_ca40 => extern "C" fn op_et_weapon_afterimage_fade(id: u32) {
        et::bridge::op_et_weapon_afterimage_fade(&mut mem(), id);
    }

    // ---- opus ----
    /// `opus_get_weapon_index(obj)`.
    opus_get_weapon_index @ 0x0080_1000 => extern "C" fn opus_get_weapon_index(obj: u32) -> i32 {
        opus::bridge::opus_get_weapon_index(&mem(), obj)
    }
    /// `opus_get_monster_index(obj)`.
    opus_get_monster_index @ 0x0080_0e80 => extern "C" fn opus_get_monster_index(obj: u32) -> i32 {
        opus::bridge::opus_get_monster_index(&mem(), obj)
    }
    /// `opus_draw_off(obj, part)`.
    opus_draw_off @ 0x0080_0d10 => extern "C" fn opus_draw_off(obj: u32, part: u32) {
        opus::bridge::opus_draw_off(&mut mem(), obj, part);
    }
    /// `opus_get_osu_by_draw(obj, like, kind, any)`.
    opus_get_osu_by_draw @ 0x0080_0f60 => extern "C" fn opus_get_osu_by_draw(obj: u32, like: u32, kind: u32, any: u32) -> u32 {
        opus::bridge::opus_get_osu_by_draw(&mem(), obj, like, kind, any)
    }
    /// `opus_fullauto_no_get(obj)`.
    opus_fullauto_no_get @ 0x0080_0df0 => extern "C" fn opus_fullauto_no_get(obj: u32) {
        opus::bridge::opus_fullauto_no_get(&mut mem(), obj);
    }
    /// `opu_wait(obj, cmd)`.
    opu_wait @ 0x0080_c500 => extern "C" fn opu_wait(obj: u32, cmd: u32) -> u32 {
        opus::bridge::opu_wait(&mut mem(), obj, cmd)
    }
    /// `opu_et_no(obj, cmd)`.
    opu_et_no @ 0x0081_65d0 => extern "C" fn opu_et_no(obj: u32, cmd: u32) -> u32 {
        opus::bridge::opu_et_no(&mut mem(), obj, cmd)
    }
    /// `opp_anmm_fin_en_bone3x(_, _, cmd, part)`.
    opp_anmm_fin_en_bone3x @ 0x0081_0ed0 => extern "C" fn opp_anmm_fin_en_bone3x(a: u32, b: u32, cmd: u32, part: u32) -> u32 {
        let _ = (a, b);
        opus::bridge::opp_anmm_fin_en_bone3x(&mut mem(), cmd, part)
    }

    // ---- opd ----
    /// `opd_num_pal_default()`.
    opd_num_pal_default @ 0x00a5_de50 => extern "C" fn opd_num_pal_default() {
        opd::bridge::opd_num_pal_default(&mut mem());
    }
    /// `opd_num_pal_set(pal)`.
    opd_num_pal_set @ 0x00a5_de60 => extern "C" fn opd_num_pal_set(pal: u32) {
        opd::bridge::opd_num_pal_set(&mut mem(), pal);
    }
    /// `opd_num_display(mode)`.
    opd_num_display @ 0x00a5_dd90 => extern "C" fn opd_num_display(mode: u32) {
        opd::bridge::opd_num_display(&mut mem(), mode);
    }
    /// `opd_num_xy_set(x, y)`.
    opd_num_xy_set @ 0x00a5_e2b0 => extern "C" fn opd_num_xy_set(x: u32, y: u32) {
        opd::bridge::opd_num_xy_set(&mut mem(), x, y);
    }
    /// `opd_num_vdisp()`.
    opd_num_vdisp @ 0x00a5_dfb0 => extern "C" fn opd_num_vdisp() {
        opd::bridge::opd_num_vdisp(&mut mem());
    }
    /// `opd_heap_display_clear(n)`.
    opd_heap_display_clear @ 0x00a6_49c0 => extern "C" fn opd_heap_display_clear(n: u32) {
        opd::bridge::opd_heap_display_clear(&mut mem(), n);
    }
    /// `opd_heap_display_init()`.
    opd_heap_display_init @ 0x00a6_4a00 => extern "C" fn opd_heap_display_init() {
        opd::bridge::opd_heap_display_init(&mut mem());
    }
    /// `opd_heap_display_set(used, total, flags)`.
    opd_heap_display_set @ 0x00a6_4b20 => extern "C" fn opd_heap_display_set(used: u32, total: u32, flags: u32) {
        opd::bridge::opd_heap_display_set(&mut mem(), used, total, flags);
    }
    /// `opd_heap_display_set_par(heap, flags)`.
    opd_heap_display_set_par @ 0x00a6_4b60 => extern "C" fn opd_heap_display_set_par(h: u32, flags: u32) {
        opd::bridge::opd_heap_display_set_par(&mut mem(), h, flags);
    }

    // ---- op ordering-table wrappers ----
    /// `op_func_before_ot_set(func, a, b, c)`.
    op_func_before_ot_set @ 0x007e_b370 => extern "C" fn op_func_before_ot_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_before_ot_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_after_ot3d_set(func, a, b, c)`.
    op_func_after_ot3d_set @ 0x007e_b320 => extern "C" fn op_func_after_ot3d_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_after_ot3d_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_before_wot_set(func, a, b, c)`.
    op_func_before_wot_set @ 0x007e_b3c0 => extern "C" fn op_func_before_wot_set(func: u32, a: u32, b: u32, c: u32) -> i32 {
        op::bridge::op_func_before_wot_set(&mut mem(), func, a, b, c)
    }
    /// `op_func_before_ot_free(slot)`.
    op_func_before_ot_free @ 0x007e_b350 => extern "C" fn op_func_before_ot_free(slot: u32) {
        op::bridge::op_func_before_ot_free(&mut mem(), slot);
    }
    /// `op_func_after_ot3d_free(slot)`.
    op_func_after_ot3d_free @ 0x007e_b300 => extern "C" fn op_func_after_ot3d_free(slot: u32) {
        op::bridge::op_func_after_ot3d_free(&mut mem(), slot);
    }
    /// `op_func_before_wot_free(slot)`.
    op_func_before_wot_free @ 0x007e_b3a0 => extern "C" fn op_func_before_wot_free(slot: u32) {
        op::bridge::op_func_before_wot_free(&mut mem(), slot);
    }
    /// `op_heap_display_set_par(heap, flags)`.
    op_heap_display_set_par @ 0x007f_f2b0 => extern "C" fn op_heap_display_set_par(h: u32, flags: u32) {
        op::bridge::op_heap_display_set_par(&mut mem(), h, flags);
    }

    // ---- particle program table ----
    /// `pppDrawShapeCon(obj, ctrl)`.
    pppDrawShapeCon @ 0x0074_0eb0 => extern "C" fn ppp_draw_shape_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawShapeXCon(obj, ctrl)`.
    pppDrawShapeXCon @ 0x0074_1450 => extern "C" fn ppp_draw_shape_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiDrawShapePointLightCon(obj, ctrl)`.
    pppNeiDrawShapePointLightCon @ 0x0074_b9a0 => extern "C" fn ppp_nei_draw_shape_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_draw_shape_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApCon(obj, ctrl)`.
    pppVertexApCon @ 0x0075_7b30 => extern "C" fn ppp_vertex_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApDisPosCon(obj, ctrl)`.
    pppVertexApDisPosCon @ 0x0075_7e90 => extern "C" fn ppp_vertex_ap_dis_pos_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_dis_pos_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApLcCon(obj, ctrl)`.
    pppVertexApLcCon @ 0x0075_8250 => extern "C" fn ppp_vertex_ap_lc_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_lc_con(&mut mem(), obj, ctrl);
    }
    /// `pppVertexApAtCon(obj, ctrl)`.
    pppVertexApAtCon @ 0x0075_84b0 => extern "C" fn ppp_vertex_ap_at_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vertex_ap_at_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointApCon(obj, ctrl)`.
    pppPointApCon @ 0x0075_7470 => extern "C" fn ppp_point_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointRApCon(obj, ctrl)`.
    pppPointRApCon @ 0x0075_7750 => extern "C" fn ppp_point_r_ap_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_r_ap_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeHitBornCon(obj, ctrl)`.
    pppKeHitBornCon @ 0x0075_9990 => extern "C" fn ppp_ke_hit_born_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_hit_born_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThHitBornCon(obj, ctrl)`.
    pppKeThHitBornCon @ 0x0075_9f10 => extern "C" fn ppp_ke_th_hit_born_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_hit_born_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRndCon(obj, ctrl)`.
    pppKeBornRndCon @ 0x0075_8790 => extern "C" fn ppp_ke_born_rnd_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd2Con(obj, ctrl)`.
    pppKeBornRnd2Con @ 0x0075_8820 => extern "C" fn ppp_ke_born_rnd2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd3Con(obj, ctrl)`.
    pppKeBornRnd3Con @ 0x0075_89d0 => extern "C" fn ppp_ke_born_rnd3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd3_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd4Con(obj, ctrl)`.
    pppKeBornRnd4Con @ 0x0075_8dd0 => extern "C" fn ppp_ke_born_rnd4_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd4_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlRevCon(obj, ctrl)`.
    pppDrawMdlRevCon @ 0x0074_03f0 => extern "C" fn ppp_draw_mdl_rev_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_rev_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpSftCon(obj, ctrl)`.
    pppKeLnsLpSftCon @ 0x0075_9fb0 => extern "C" fn ppp_ke_lns_lp_sft_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_sft_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpCon2(obj, ctrl)`.
    pppKeLnsLpCon2 @ 0x0075_af60 => extern "C" fn ppp_ke_lns_lp_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeLnsLpTCon2(obj, ctrl)`.
    pppKeLnsLpTCon2 @ 0x0075_b110 => extern "C" fn ppp_ke_lns_lp_t_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_lns_lp_t_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd5Con(obj, ctrl)`.
    pppKeBornRnd5Con @ 0x0075_8f90 => extern "C" fn ppp_ke_born_rnd5_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd5_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornRnd6Con(obj, ctrl)`.
    pppKeBornRnd6Con @ 0x0075_9270 => extern "C" fn ppp_ke_born_rnd6_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_rnd6_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeHitChkPxBCon(obj, ctrl)`.
    pppKeHitChkPxBCon @ 0x0075_9bc0 => extern "C" fn ppp_ke_hit_chk_px_b_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_hit_chk_px_b_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailCon(obj, ctrl)`.
    pppKeShpTailCon @ 0x0074_c540 => extern "C" fn ppp_ke_shp_tail_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailXCon(obj, ctrl)`.
    pppKeShpTailXCon @ 0x0074_d540 => extern "C" fn ppp_ke_shp_tail_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2Con(obj, ctrl)`.
    pppKeShpTail2Con @ 0x0074_e540 => extern "C" fn ppp_ke_shp_tail2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTail2XCon(obj, ctrl)`.
    pppKeShpTail2XCon @ 0x0074_f580 => extern "C" fn ppp_ke_shp_tail2_x_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail2_x_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpTailLcCon(obj, ctrl)`.
    pppKeShpTailLcCon @ 0x0075_4fc0 => extern "C" fn ppp_ke_shp_tail_lc_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_tail_lc_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawShapeRevCon(obj, ctrl)`.
    pppDrawShapeRevCon @ 0x0074_1570 => extern "C" fn ppp_draw_shape_rev_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_shape_rev_con(&mut mem(), obj, ctrl);
    }
    /// `pppVtMimeCon2(obj, ctrl)`.
    pppVtMimeCon2 @ 0x0072_edb0 => extern "C" fn ppp_vt_mime_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_vt_mime_con2(&mut mem(), obj, ctrl);
    }
    /// `pppNeiPointLightCon(obj, ctrl)`.
    pppNeiPointLightCon @ 0x0075_d7c0 => extern "C" fn ppp_nei_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeTkFadeCon(obj, ctrl)`.
    pppKeTkFadeCon @ 0x0072_ee20 => extern "C" fn ppp_ke_tk_fade_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_tk_fade_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfdCon(obj, ctrl)`.
    pppKeMdlTfdCon @ 0x0074_97e0 => extern "C" fn ppp_ke_mdl_tfd_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfd2Con(obj, ctrl)`.
    pppKeMdlTfd2Con @ 0x0074_9b10 => extern "C" fn ppp_ke_mdl_tfd2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeMdlTfd3Con(obj, ctrl)`.
    pppKeMdlTfd3Con @ 0x0074_9e60 => extern "C" fn ppp_ke_mdl_tfd3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_mdl_tfd3_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveLoopCon(obj, ctrl)`.
    pppMoveLoopCon @ 0x0075_c150 => extern "C" fn ppp_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveLoopCon2(obj, ctrl)`.
    pppMoveLoopCon2 @ 0x0075_c180 => extern "C" fn ppp_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveLoopCon(obj, ctrl)`.
    pppSclMoveLoopCon @ 0x0075_c330 => extern "C" fn ppp_scl_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveLoopCon2(obj, ctrl)`.
    pppSclMoveLoopCon2 @ 0x0075_c360 => extern "C" fn ppp_scl_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveLoopCon(obj, ctrl)`.
    pppAngMoveLoopCon @ 0x0075_c210 => extern "C" fn ppp_ang_move_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveLoopCon2(obj, ctrl)`.
    pppAngMoveLoopCon2 @ 0x0075_c250 => extern "C" fn ppp_ang_move_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeOfsPtCon(obj, ctrl)`.
    pppKeOfsPtCon @ 0x0075_cdf0 => extern "C" fn ppp_ke_ofs_pt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_ofs_pt_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlSeaCon2(obj, ctrl)`.
    pppDrawMdlSeaCon2 @ 0x0073_ab40 => extern "C" fn ppp_draw_mdl_sea_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_sea_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlInfCon2(obj, ctrl)`.
    pppDrawMdlInfCon2 @ 0x0073_b800 => extern "C" fn ppp_draw_mdl_inf_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_inf_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlPSimCon2(obj, ctrl)`.
    pppDrawMdlPSimCon2 @ 0x0073_cfe0 => extern "C" fn ppp_draw_mdl_p_sim_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_p_sim_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopCon2(obj, ctrl)`.
    pppDrawMdlLoopCon2 @ 0x0073_e270 => extern "C" fn ppp_draw_mdl_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopZCon2(obj, ctrl)`.
    pppDrawMdlLoopZCon2 @ 0x0073_ea00 => extern "C" fn ppp_draw_mdl_loop_z_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_z_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopDisPosCon2(obj, ctrl)`.
    pppDrawMdlLoopDisPosCon2 @ 0x0073_f1f0 => extern "C" fn ppp_draw_mdl_loop_dis_pos_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraLoopCon2(obj, ctrl)`.
    pppDrawMdlCameraLoopCon2 @ 0x0073_fa10 => extern "C" fn ppp_draw_mdl_camera_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraCon2(obj, ctrl)`.
    pppDrawMdlCameraCon2 @ 0x0073_d8f0 => extern "C" fn ppp_draw_mdl_camera_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeShpDttCon(obj, ctrl)`.
    pppKeShpDttCon @ 0x0075_5010 => extern "C" fn ppp_ke_shp_dtt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_shp_dtt_con(&mut mem(), obj, ctrl);
    }
    /// `pppEiWindFunCon(obj, ctrl)`.
    pppEiWindFunCon @ 0x0075_d590 => extern "C" fn ppp_ei_wind_fun_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ei_wind_fun_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTsCon(obj, ctrl)`.
    pppDrawMdlTsCon @ 0x0073_84f0 => extern "C" fn ppp_draw_mdl_ts_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTs2Con(obj, ctrl)`.
    pppDrawMdlTs2Con @ 0x0073_9480 => extern "C" fn ppp_draw_mdl_ts2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts2_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlTs3Con(obj, ctrl)`.
    pppDrawMdlTs3Con @ 0x0073_aac0 => extern "C" fn ppp_draw_mdl_ts3_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_ts3_con(&mut mem(), obj, ctrl);
    }
    /// `pppNeiDrawMdlTsPointLightCon(obj, ctrl)`.
    pppNeiDrawMdlTsPointLightCon @ 0x0074_b500 => extern "C" fn ppp_nei_draw_mdl_ts_point_light_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_nei_draw_mdl_ts_point_light_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointCon(obj, ctrl)`.
    pppPointCon @ 0x0075_c4c0 => extern "C" fn ppp_point_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_con(&mut mem(), obj, ctrl);
    }
    /// `pppScaleCon(obj, ctrl)`.
    pppScaleCon @ 0x0075_d050 => extern "C" fn ppp_scale_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scale_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngleCon(obj, ctrl)`.
    pppAngleCon @ 0x0075_ce80 => extern "C" fn ppp_angle_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_con(&mut mem(), obj, ctrl);
    }
    /// `pppAcceleCon(obj, ctrl)`.
    pppAcceleCon @ 0x0075_b810 => extern "C" fn ppp_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclAcceleCon(obj, ctrl)`.
    pppSclAcceleCon @ 0x0075_b9d0 => extern "C" fn ppp_scl_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppMoveCon(obj, ctrl)`.
    pppMoveCon @ 0x0075_beb0 => extern "C" fn ppp_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppSclMoveCon(obj, ctrl)`.
    pppSclMoveCon @ 0x0075_c070 => extern "C" fn ppp_scl_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scl_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngAcceleCon(obj, ctrl)`.
    pppAngAcceleCon @ 0x0075_b8c0 => extern "C" fn ppp_ang_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngMoveCon(obj, ctrl)`.
    pppAngMoveCon @ 0x0075_bf60 => extern "C" fn ppp_ang_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeBornPtCmplCon(obj, ctrl)`.
    pppKeBornPtCmplCon @ 0x0075_9620 => extern "C" fn ppp_ke_born_pt_cmpl_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_born_pt_cmpl_con(&mut mem(), obj, ctrl);
    }
    /// `pppPointLoopCon(obj, ctrl)`.
    pppPointLoopCon @ 0x0075_c5a0 => extern "C" fn ppp_point_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppAngleLoopCon(obj, ctrl)`.
    pppAngleLoopCon @ 0x0075_cf40 => extern "C" fn ppp_angle_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppScaleLoopCon(obj, ctrl)`.
    pppScaleLoopCon @ 0x0075_d130 => extern "C" fn ppp_scale_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_scale_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraCon(obj, ctrl)`.
    pppDrawMdlCameraCon @ 0x0073_d8c0 => extern "C" fn ppp_draw_mdl_camera_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopCon(obj, ctrl)`.
    pppDrawMdlLoopCon @ 0x0073_e240 => extern "C" fn ppp_draw_mdl_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopZCon(obj, ctrl)`.
    pppDrawMdlLoopZCon @ 0x0073_e9d0 => extern "C" fn ppp_draw_mdl_loop_z_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_z_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlLoopDisPosCon(obj, ctrl)`.
    pppDrawMdlLoopDisPosCon @ 0x0073_f1c0 => extern "C" fn ppp_draw_mdl_loop_dis_pos_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_loop_dis_pos_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlCameraLoopCon(obj, ctrl)`.
    pppDrawMdlCameraLoopCon @ 0x0073_f9e0 => extern "C" fn ppp_draw_mdl_camera_loop_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_camera_loop_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawSpriteCon(obj, ctrl)`.
    pppDrawSpriteCon @ 0x0075_66a0 => extern "C" fn ppp_draw_sprite_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_sprite_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawFilterCon(obj, ctrl)`.
    pppDrawFilterCon @ 0x0075_7210 => extern "C" fn ppp_draw_filter_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_filter_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeGrvTgtCon(obj, ctrl)`.
    pppKeGrvTgtCon @ 0x0075_9800 => extern "C" fn ppp_ke_grv_tgt_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_grv_tgt_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThTp2Con(obj, ctrl)`.
    pppKeThTp2Con @ 0x0073_6e30 => extern "C" fn ppp_ke_th_tp2_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_tp2_con(&mut mem(), obj, ctrl);
    }
    /// `pppKeThTpCon(obj, ctrl)`.
    pppKeThTpCon @ 0x0073_6e60 => extern "C" fn ppp_ke_th_tp_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_ke_th_tp_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlPSimCon(obj, ctrl)`.
    pppDrawMdlPSimCon @ 0x0073_cfb0 => extern "C" fn ppp_draw_mdl_p_sim_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_p_sim_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawRainCon(obj, ctrl)`.
    pppDrawRainCon @ 0x0075_6240 => extern "C" fn ppp_draw_rain_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_rain_con(&mut mem(), obj, ctrl);
    }
    /// `pppDrawMdlSeaCon(obj, ctrl)`.
    pppDrawMdlSeaCon @ 0x0073_ab00 => extern "C" fn ppp_draw_mdl_sea_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_draw_mdl_sea_con(&mut mem(), obj, ctrl);
    }
    /// `pppColAcceleCon(obj, ctrl)`.
    pppColAcceleCon @ 0x0075_bac0 => extern "C" fn ppp_col_accele_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppColMoveCon(obj, ctrl)`.
    pppColMoveCon @ 0x0075_c410 => extern "C" fn ppp_col_move_con(obj: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele_con(&mut mem(), obj, ctrl);
    }
    /// `pppAccele(obj, key, ctrl)`.
    pppAccele @ 0x0075_b740 => extern "C" fn ppp_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclAccele(obj, key, ctrl)`.
    pppSclAccele @ 0x0075_b900 => extern "C" fn ppp_scl_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppMove(obj, key, ctrl)`.
    pppMove @ 0x0075_bde0 => extern "C" fn ppp_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclMove(obj, key, ctrl)`.
    pppSclMove @ 0x0075_bfa0 => extern "C" fn ppp_scl_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngAccele(obj, key, ctrl)`.
    pppAngAccele @ 0x0075_b850 => extern "C" fn ppp_ang_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngMove(obj, key, ctrl)`.
    pppAngMove @ 0x0075_bef0 => extern "C" fn ppp_ang_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppColAccele(obj, key, ctrl)`.
    pppColAccele @ 0x0075_ba40 => extern "C" fn ppp_col_accele(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppColMove(obj, key, ctrl)`.
    pppColMove @ 0x0075_c390 => extern "C" fn ppp_col_move(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_col_accele(&mut mem(), obj, key, ctrl);
    }
    /// `pppMoveLoop(obj, key, ctrl)`.
    pppMoveLoop @ 0x0075_c0b0 => extern "C" fn ppp_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppSclMoveLoop(obj, key, ctrl)`.
    pppSclMoveLoop @ 0x0075_c290 => extern "C" fn ppp_scl_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngMoveLoop(obj, key, ctrl)`.
    pppAngMoveLoop @ 0x0075_c1b0 => extern "C" fn ppp_ang_move_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ang_move_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppPoint(obj, key, ctrl)`.
    pppPoint @ 0x0075_c450 => extern "C" fn ppp_point(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point(&mut mem(), obj, key, ctrl);
    }
    /// `pppScale(obj, key, ctrl)`.
    pppScale @ 0x0075_cfe0 => extern "C" fn ppp_scale(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point(&mut mem(), obj, key, ctrl);
    }
    /// `pppPointLoop(obj, key, ctrl)`.
    pppPointLoop @ 0x0075_c500 => extern "C" fn ppp_point_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppScaleLoop(obj, key, ctrl)`.
    pppScaleLoop @ 0x0075_d090 => extern "C" fn ppp_scale_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppAngleLoop(obj, key, ctrl)`.
    pppAngleLoop @ 0x0075_cec0 => extern "C" fn ppp_angle_loop(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop(&mut mem(), obj, key, ctrl);
    }
    /// `pppPointLoopCon2(obj, ctrl)`.
    pppPointLoopCon2 @ 0x0075_c5d0 => extern "C" fn ppp_point_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppScaleLoopCon2(obj, ctrl)`.
    pppScaleLoopCon2 @ 0x0075_d160 => extern "C" fn ppp_scale_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_point_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppAngleLoopCon2(obj, ctrl)`.
    pppAngleLoopCon2 @ 0x0075_cf70 => extern "C" fn ppp_angle_loop_con2(obj: u32, ctrl: u32) {
        prog::bridge::ppp_angle_loop_con2(&mut mem(), obj, ctrl);
    }
    /// `pppKeGrvTgt(obj, key, ctrl)`.
    pppKeGrvTgt @ 0x0075_97b0 => extern "C" fn ppp_ke_grv_tgt(obj: u32, key: u32, ctrl: u32) {
        prog::bridge::ppp_ke_grv_tgt(&mut mem(), obj, key, ctrl);
    }
}
