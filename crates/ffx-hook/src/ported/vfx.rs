//! `ffx-vfx`: particle and effect functions. All cdecl; functions whose
//! original leaves a result in `eax` return the full `u32`.

use super::{mem, ported};
use ffx_vfx::{et, fp, heap, op, opd, opus, ppp, rand};

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
    op_spr_alloc @ 0x007e_6a00 => extern "C" fn op_spr_alloc(n: u32) -> u32 {
        op::bridge::op_spr_alloc(&mut mem(), n)
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
    opus_fullauto_no_get @ 0x0080_0df0 => extern "C" fn opus_fullauto_no_get(obj: u32) -> u32 {
        opus::bridge::opus_fullauto_no_get(&mut mem(), obj)
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
}
