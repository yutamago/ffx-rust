//! `ffx-field`: data chunks, packets, geometry, scene objects, the guide
//! map, effect tasks, the field renderer's state, VU0 helpers, `Nkz`/`Sqs`.
//! Floats passed by value are taken as bits (they cross the x87 stack in
//! the original).

use super::{mem, ported};
use ffx_field::{
    abmap, camera, freecam, geom, guide, ke, model, nkz, packet, rcdt, rcui, scene, task, vu0, yi,
};

ported! {
    /// `rcdtChunkInit(chunk)`.
    rcdtChunkInit @ 0x0092_b840 => extern "C" fn rcdt_chunk_init(c: u32) {
        rcdt::bridge::rcdt_chunk_init(&mut mem(), c);
    }
    /// `rcdtChunkGetBegin(chunk)`.
    rcdtChunkGetBegin @ 0x0092_b810 => extern "C" fn rcdt_chunk_get_begin(c: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_get_begin(&mem(), c)
    }
    /// `rcdtChunkSetBin(chunk, data)`.
    rcdtChunkSetBin @ 0x0092_b940 => extern "C" fn rcdt_chunk_set_bin(c: u32, data: u32) {
        rcdt::bridge::rcdt_chunk_set_bin(&mut mem(), c, data);
    }
    /// `rcdtParseAdvance(parse, n)`.
    rcdtParseAdvance @ 0x0092_b960 => extern "C" fn rcdt_parse_advance(p: u32, n: u32) {
        rcdt::bridge::rcdt_parse_advance(&mut mem(), p, n);
    }
    /// `rcdtParseGetBegin(parse)`.
    rcdtParseGetBegin @ 0x0092_ba70 => extern "C" fn rcdt_parse_get_begin(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_begin(&mem(), p)
    }
    /// `rcdtParseSet(parse, chunk)`.
    rcdtParseSet @ 0x0092_c0a0 => extern "C" fn rcdt_parse_set(p: u32, c: u32) {
        rcdt::bridge::rcdt_parse_set(&mut mem(), p, c);
    }
    /// `rcdtParseSetPos(parse, offset)`.
    rcdtParseSetPos @ 0x0092_c0d0 => extern "C" fn rcdt_parse_set_pos(p: u32, offset: u32) {
        rcdt::bridge::rcdt_parse_set_pos(&mut mem(), p, offset);
    }
    /// `rcdtParseGetBin(parse, out, n)`.
    rcdtParseGetBin @ 0x0092_ba90 => extern "C" fn rcdt_parse_get_bin(p: u32, out: u32, n: u32) {
        rcdt::bridge::rcdt_parse_get_bin(&mut mem(), p, out, n);
    }
    /// `rcdtParseInit(parse)`.
    rcdtParseInit @ 0x0092_bf50 => extern "C" fn rcdt_parse_init(p: u32) {
        rcdt::bridge::rcdt_parse_init(&mut mem(), p);
    }
    /// `rcdtParseInitSeparators(parse)`.
    rcdtParseInitSeparators @ 0x0092_c000 => extern "C" fn rcdt_parse_init_separators(p: u32) {
        rcdt::bridge::rcdt_parse_init_separators(&mut mem(), p);
    }
    /// `rcPacketDataInit(packet)`.
    rcPacketDataInit @ 0x0092_7a20 => extern "C" fn rc_packet_data_init(p: u32) {
        packet::bridge::rc_packet_data_init(&mut mem(), p);
    }
    /// `rcPacketDataOpenAdr(packet, buffer)`.
    rcPacketDataOpenAdr @ 0x0092_7a70 => extern "C" fn rc_packet_data_open_adr(p: u32, buffer: u32) {
        packet::bridge::rc_packet_data_open_adr(&mut mem(), p, buffer);
    }
    /// `rcPacketDataCloseAdr(packet)`.
    rcPacketDataCloseAdr @ 0x0092_75b0 => extern "C" fn rc_packet_data_close_adr(p: u32) {
        packet::bridge::rc_packet_data_close_adr(&mut mem(), p);
    }
    /// `rcBBoxInit(box)`.
    rcBBoxInit @ 0x0092_14e0 => extern "C" fn rc_bbox_init(b: u32) {
        geom::bridge::rc_bbox_init(&mut mem(), b);
    }
    /// `rcBBoxGetCenter(box, out)`.
    rcBBoxGetCenter @ 0x0092_13d0 => extern "C" fn rc_bbox_get_center(b: u32, out: u32) {
        geom::bridge::rc_bbox_get_center(&mut mem(), b, out);
    }
    /// `rcBBoxGetShortConvertScale(box)`.
    rcBBoxGetShortConvertScale @ 0x0092_1410 => extern "C" fn rc_bbox_get_short_convert_scale(b: u32) -> f32 {
        geom::bridge::rc_bbox_get_short_convert_scale(&mem(), b)
    }
    /// `rcVec3fMul(out, a, b)`: the cross product.
    rcVec3fMul @ 0x0092_12c0 => extern "C" fn rc_vec3f_mul(out: u32, a: u32, b: u32) {
        geom::bridge::rc_vec3f_mul(&mut mem(), out, a, b);
    }
    /// `rcVec3fLength(v)`.
    rcVec3fLength @ 0x0092_1250 => extern "C" fn rc_vec3f_length(v: u32) -> f32 {
        geom::bridge::rc_vec3f_length(&mem(), v)
    }
    /// `rcVec3fNormalize(v)`.
    rcVec3fNormalize @ 0x0092_1360 => extern "C" fn rc_vec3f_normalize(v: u32) {
        geom::bridge::rc_vec3f_normalize(&mut mem(), v);
    }
    /// `rcMat4fSetTranslate(m, x, y, z)`.
    rcMat4fSetTranslate @ 0x0092_5850 => extern "C" fn rc_mat4f_set_translate(m: u32, x: u32, y: u32, z: u32) {
        geom::bridge::rc_mat4f_set_translate(&mut mem(), m, x, y, z);
    }
    /// `rcMat4fSetTranslateVec3f(m, v)`.
    rcMat4fSetTranslateVec3f @ 0x0092_5870 => extern "C" fn rc_mat4f_set_translate_vec3f(m: u32, v: u32) {
        geom::bridge::rc_mat4f_set_translate_vec3f(&mut mem(), m, v);
    }
    /// `rcbgModelCreateFromOMD(model, omd)`.
    rcbgModelCreateFromOMD @ 0x0092_7d70 => extern "C" fn rcbg_model_create_from_omd(m: u32, blob: u32) {
        model::bridge::rcbg_model_create_from_omd(&mut mem(), m, blob);
    }
    /// `rcbgModelGetPrimHeader(model, n)`.
    rcbgModelGetPrimHeader @ 0x0092_82c0 => extern "C" fn rcbg_model_get_prim_header(m: u32, n: i32) -> u32 {
        model::bridge::rcbg_model_get_prim_header(&mem(), m, n)
    }
    /// `rcbgModelGetPrim(model, n)`.
    rcbgModelGetPrim @ 0x0092_82a0 => extern "C" fn rcbg_model_get_prim(m: u32, n: i32) -> u32 {
        model::bridge::rcbg_model_get_prim(&mem(), m, n)
    }
    /// `rcbgModelSetAlpha(model, alpha)`.
    rcbgModelSetAlpha @ 0x0092_8460 => extern "C" fn rcbg_model_set_alpha(m: u32, alpha: u32) {
        model::bridge::rcbg_model_set_alpha(&mut mem(), m, alpha.to_le_bytes()[0]);
    }
    /// `rcbgLayerLoadCalcCenter(layer)`.
    rcbgLayerLoadCalcCenter @ 0x0092_b270 => extern "C" fn rcbg_layer_load_calc_center(l: u32) {
        model::bridge::rcbg_layer_load_calc_center(&mut mem(), l);
    }
    /// `rcbgTrxInit(trx)`.
    rcbgTrxInit @ 0x0092_d510 => extern "C" fn rcbg_trx_init(t: u32) {
        model::bridge::rcbg_trx_init(&mut mem(), t);
    }
    /// `rcbgTrxSetNo(trx, no)`.
    rcbgTrxSetNo @ 0x0092_d670 => extern "C" fn rcbg_trx_set_no(t: u32, no: u32) {
        model::bridge::rcbg_trx_set_no(&mut mem(), t, no);
    }
    /// `rcbgPalInit(pal)`.
    rcbgPalInit @ 0x0092_99b0 => extern "C" fn rcbg_pal_init(p: u32) {
        model::bridge::rcbg_pal_init(&mut mem(), p);
    }
    /// `rcbgPalSetNo(pal, no)`: the low 16 bits of `no`.
    rcbgPalSetNo @ 0x0092_9ab0 => extern "C" fn rcbg_pal_set_no(p: u32, no: u32) {
        let [lo, hi, _, _] = no.to_le_bytes();
        model::bridge::rcbg_pal_set_no(&mut mem(), p, u16::from_le_bytes([lo, hi]));
    }
    /// `rcbgAnimeInit(anime)`.
    rcbgAnimeInit @ 0x0093_08a0 => extern "C" fn rcbg_anime_init(a: u32) {
        model::bridge::rcbg_anime_init(&mut mem(), a);
    }
    /// `rcbgPSimSetInitCnt(psim, n)`.
    rcbgPSimSetInitCnt @ 0x0091_fef0 => extern "C" fn rcbg_psim_set_init_cnt(p: u32, n: u32) {
        model::bridge::rcbg_psim_set_init_cnt(&mut mem(), p, n);
    }
    /// `rcbgPSimClearOMD(psim)`.
    rcbgPSimClearOMD @ 0x0091_f9b0 => extern "C" fn rcbg_psim_clear_omd(p: u32) {
        model::bridge::rcbg_psim_clear_omd(&mut mem(), p);
    }
    /// `Guide_MeshObject(guide)`.
    Guide_MeshObject @ 0x0091_d460 => extern "C" fn guide_mesh_object(g: u32) -> u32 {
        guide::bridge::guide_mesh_object(&mem(), g)
    }
    /// `Guide_FadeStep(show)`.
    Guide_FadeStep @ 0x0091_d280 => extern "C" fn guide_fade_step(show: i32) {
        guide::bridge::guide_fade_step(&mut mem(), show);
    }
    /// `Guide_FadeOff()`.
    Guide_FadeOff @ 0x0091_d3a0 => extern "C" fn guide_fade_off() {
        guide::bridge::guide_fade_off(&mut mem());
    }
    /// `rcbgGuideSetDispProg(guide, on)`.
    rcbgGuideSetDispProg @ 0x0091_d560 => extern "C" fn rcbg_guide_set_disp_prog(g: u32, on: i32) {
        guide::bridge::rcbg_guide_set_disp_prog(&mut mem(), g, on);
    }
    /// `rcbgGuideBattleBegin(guide)`.
    rcbgGuideBattleBegin @ 0x0091_ccc0 => extern "C" fn rcbg_guide_battle_begin(g: u32) {
        guide::bridge::rcbg_guide_battle_begin(&mut mem(), g);
    }
    /// `rcbgGuideBattleEnd(guide)`.
    rcbgGuideBattleEnd @ 0x0091_cce0 => extern "C" fn rcbg_guide_battle_end(g: u32) {
        guide::bridge::rcbg_guide_battle_end(&mut mem(), g);
    }
    /// `Yn_BattleBegin()`.
    Yn_BattleBegin @ 0x0091_9a20 => extern "C" fn yn_battle_begin() {
        guide::bridge::yn_battle_begin(&mut mem());
    }
    /// `Yn_BattleEnd()`.
    Yn_BattleEnd @ 0x0091_9a40 => extern "C" fn yn_battle_end() {
        guide::bridge::yn_battle_end(&mut mem());
    }
    /// `Yn_GuideMapSetDispProg(on)`.
    Yn_GuideMapSetDispProg @ 0x0091_9d70 => extern "C" fn yn_guide_map_set_disp_prog(on: i32) {
        guide::bridge::yn_guide_map_set_disp_prog(&mut mem(), on);
    }
    /// `Yn_GuideMapGetMapNo()`.
    Yn_GuideMapGetMapNo @ 0x0091_9c70 => extern "C" fn yn_guide_map_get_map_no() -> i32 {
        guide::bridge::yn_guide_map_get_map_no(&mem())
    }
    /// `rcefTaskInit(task)`.
    rcefTaskInit @ 0x0092_ed10 => extern "C" fn rcef_task_init(t: u32) {
        task::bridge::rcef_task_init(&mut mem(), t);
    }
    /// `rcefTaskSetSeq(task, seq)`.
    rcefTaskSetSeq @ 0x0092_ee20 => extern "C" fn rcef_task_set_seq(t: u32, seq: u32) {
        task::bridge::rcef_task_set_seq(&mut mem(), t, seq);
    }
    /// `rcefTaskSetSeqCont(task, seq)`.
    rcefTaskSetSeqCont @ 0x0092_ee40 => extern "C" fn rcef_task_set_seq_cont(t: u32, seq: u32) {
        task::bridge::rcef_task_set_seq_cont(&mut mem(), t, seq);
    }
    /// `rcefTaskCallSeq(task, seq)`.
    rcefTaskCallSeq @ 0x0092_ecb0 => extern "C" fn rcef_task_call_seq(t: u32, seq: u32) {
        task::bridge::rcef_task_call_seq(&mut mem(), t, seq);
    }
    /// `rcefTaskRetSeqCont_Inner(task)`.
    rcefTaskRetSeqCont_Inner @ 0x0092_ede0 => extern "C" fn rcef_task_ret_seq_cont_inner(t: u32) {
        task::bridge::rcef_task_ret_seq_cont_inner(&mut mem(), t);
    }
    /// `rcefTaskRetSeqCont(task)`.
    rcefTaskRetSeqCont @ 0x0092_ee00 => extern "C" fn rcef_task_ret_seq_cont(t: u32) {
        task::bridge::rcef_task_ret_seq_cont(&mut mem(), t);
    }
    /// `rcefTaskWait(task, frames)`.
    rcefTaskWait @ 0x0092_ee60 => extern "C" fn rcef_task_wait(t: u32, frames: i32) {
        task::bridge::rcef_task_wait(&mut mem(), t, frames);
    }
    /// `rcefTaskWaitSeq(task)`.
    rcefTaskWaitSeq @ 0x0092_ee90 => extern "C" fn rcef_task_wait_seq(t: u32) {
        task::bridge::rcef_task_wait_seq(&mut mem(), t);
    }
    /// `rcefTaskCtrlInit(ctrl)`.
    rcefTaskCtrlInit @ 0x0093_0290 => extern "C" fn rcef_task_ctrl_init(c: u32) {
        task::bridge::rcef_task_ctrl_init(&mut mem(), c);
    }
    /// `rcefTaskCtrlIsEnd(ctrl)`.
    rcefTaskCtrlIsEnd @ 0x0093_02e0 => extern "C" fn rcef_task_ctrl_is_end(c: u32) -> u32 {
        u32::from(task::bridge::rcef_task_ctrl_is_end(&mem(), c))
    }
    /// `rcefTaskCtrlAdd(first, task)`.
    rcefTaskCtrlAdd @ 0x0093_0220 => extern "C" fn rcef_task_ctrl_add(first: u32, t: u32) {
        task::bridge::rcef_task_ctrl_add(&mut mem(), first, t);
    }
    /// `rcefObjIsParEnd(obj)`.
    rcefObjIsParEnd @ 0x0093_0010 => extern "C" fn rcef_obj_is_par_end(o: u32) -> u32 {
        u32::from(task::bridge::rcef_obj_is_par_end(&mem(), o))
    }
    /// `rcefObjIsAnimeEnd(obj)`.
    rcefObjIsAnimeEnd @ 0x0092_fff0 => extern "C" fn rcef_obj_is_anime_end(o: u32) -> u32 {
        u32::from(task::bridge::rcef_obj_is_anime_end(&mem(), o))
    }
    /// `rcefObjAttachChr(obj, chr, element)`.
    rcefObjAttachChr @ 0x0092_f970 => extern "C" fn rcef_obj_attach_chr(o: u32, chr: u32, element: u32) {
        task::bridge::rcef_obj_attach_chr(&mut mem(), o, chr, element);
    }
    /// `rcefObjClearMode(obj)`.
    rcefObjClearMode @ 0x0093_01e0 => extern "C" fn rcef_obj_clear_mode(o: u32) {
        task::bridge::rcef_obj_clear_mode(&mut mem(), o);
    }
    /// `rcefObjSetMode(obj, mode)`.
    rcefObjSetMode @ 0x0093_0200 => extern "C" fn rcef_obj_set_mode(o: u32, mode: u32) {
        task::bridge::rcef_obj_set_mode(&mut mem(), o, mode);
    }
    /// `rcefParInit(par)`.
    rcefParInit @ 0x0092_efd0 => extern "C" fn rcef_par_init(p: u32) {
        task::bridge::rcef_par_init(&mut mem(), p);
    }
    /// `rcefAnimeInit(anime)`.
    rcefAnimeInit @ 0x0092_f4c0 => extern "C" fn rcef_anime_init(a: u32) {
        task::bridge::rcef_anime_init(&mut mem(), a);
    }
    /// `rcefAnimeStart(anime, data, out0, out1)`.
    rcefAnimeStart @ 0x0092_f470 => extern "C" fn rcef_anime_start(a: u32, data: u32, out0: u32, out1: u32) {
        task::bridge::rcef_anime_start(&mut mem(), a, data, out0, out1);
    }
    /// `rcEffectTaskParWait(obj)`.
    rcEffectTaskParWait @ 0x0093_0490 => extern "C" fn rc_effect_task_par_wait(o: u32) {
        task::bridge::rc_effect_task_par_wait(&mut mem(), o);
    }
    /// `rcEffectTaskAnimeWait(obj)`.
    rcEffectTaskAnimeWait @ 0x0093_0350 => extern "C" fn rc_effect_task_anime_wait(o: u32) {
        task::bridge::rc_effect_task_anime_wait(&mut mem(), o);
    }
    /// `rcEffectTaskTaskWait(task)`.
    rcEffectTaskTaskWait @ 0x0093_0520 => extern "C" fn rc_effect_task_task_wait(t: u32) {
        task::bridge::rc_effect_task_task_wait(&mut mem(), t);
    }
    /// `rcEffectTaskFlagWait(task)`.
    rcEffectTaskFlagWait @ 0x0093_03c0 => extern "C" fn rc_effect_task_flag_wait(t: u32) {
        task::bridge::rc_effect_task_flag_wait(&mut mem(), t);
    }
    /// `yiGetRenderStatus()`.
    yiGetRenderStatus @ 0x0090_8f10 => extern "C" fn yi_get_render_status() -> u32 {
        yi::bridge::yi_get_render_status(&mem())
    }
    /// `yiRenderEnable(bits)`.
    yiRenderEnable @ 0x0090_9e20 => extern "C" fn yi_render_enable(bits: u32) {
        yi::bridge::yi_render_enable(&mut mem(), bits);
    }
    /// `yiRenderDisable(bits)`.
    yiRenderDisable @ 0x0090_9e10 => extern "C" fn yi_render_disable(bits: u32) {
        yi::bridge::yi_render_disable(&mut mem(), bits);
    }
    /// `yiGetDrawMask()`.
    yiGetDrawMask @ 0x0090_8960 => extern "C" fn yi_get_draw_mask() -> u32 {
        yi::bridge::yi_get_draw_mask(&mem())
    }
    /// `yiSetDrawMask(mask)`.
    yiSetDrawMask @ 0x0090_a2b0 => extern "C" fn yi_set_draw_mask(mask: u32) {
        yi::bridge::yi_set_draw_mask(&mut mem(), mask);
    }
    /// `yiGetVCount()`.
    yiGetVCount @ 0x0090_8f70 => extern "C" fn yi_get_vcount() -> u32 {
        yi::bridge::yi_get_vcount(&mem())
    }
    /// `yiGetFCount()`.
    yiGetFCount @ 0x0090_8970 => extern "C" fn yi_get_fcount() -> u32 {
        yi::bridge::yi_get_fcount(&mem())
    }
    /// `yiGetMapStatus()`.
    yiGetMapStatus @ 0x0090_8dd0 => extern "C" fn yi_get_map_status() -> u32 {
        yi::bridge::yi_get_map_status(&mem())
    }
    /// `yiGetMatrixDObj(index, out)`.
    yiGetMatrixDObj @ 0x0090_8df0 => extern "C" fn yi_get_matrix_dobj(index: u32, out: u32) -> i32 {
        yi::bridge::yi_get_matrix_dobj(&mut mem(), index, out)
    }
    /// `yiGetMatrixGroup(group, out)`.
    yiGetMatrixGroup @ 0x0090_8e30 => extern "C" fn yi_get_matrix_group(group: u32, out: u32) -> i32 {
        yi::bridge::yi_get_matrix_group(&mut mem(), group, out)
    }
    /// `yiSetScaleFactor(v)`.
    yiSetScaleFactor @ 0x0090_a950 => extern "C" fn yi_set_scale_factor(v: u32) {
        yi::bridge::yi_set_scale_factor(&mut mem(), v);
    }
    /// `yiRender_resettex(render)`.
    yiRender_resettex @ 0x0090_cea0 => extern "C" fn yi_render_resettex(r: u32) {
        yi::bridge::yi_render_resettex(&mut mem(), r);
    }
    /// `yiScreff_init(render)`.
    yiScreff_init @ 0x0091_3f30 => extern "C" fn yi_screff_init(r: u32) {
        yi::bridge::yi_screff_init(&mut mem(), r);
    }
    /// `KeVu0CopyVecXYZ(dst, src)`.
    KeVu0CopyVecXYZ @ 0x0075_ede0 => extern "C" fn ke_vu0_copy_vec_xyz(dst: u32, src: u32) {
        vu0::bridge::ke_vu0_copy_vec_xyz(&mut mem(), dst, src);
    }
    /// `KeVu0AddVecXYZ(dst, a, b)`.
    KeVu0AddVecXYZ @ 0x0075_eca0 => extern "C" fn ke_vu0_add_vec_xyz(dst: u32, a: u32, b: u32) {
        vu0::bridge::ke_vu0_add_vec_xyz(&mut mem(), dst, a, b);
    }
    /// `KeVu0ScaleVecXYZ(dst, src, s)`, `s` a vector by value.
    KeVu0ScaleVecXYZ @ 0x0075_fb50 => extern "C" fn ke_vu0_scale_vec_xyz(dst: u32, src: u32, s0: u32, s1: u32, s2: u32, s3: u32) {
        vu0::bridge::ke_vu0_scale_vec_xyz(&mut mem(), dst, src, [s0, s1, s2, s3]);
    }
    /// `KeVu0VecLen(v)`.
    KeVu0VecLen @ 0x0075_fda0 => extern "C" fn ke_vu0_vec_len(v: u32) -> f32 {
        vu0::bridge::ke_vu0_vec_len(&mem(), v)
    }
    /// `KeVu0VecLenXY(v)`.
    KeVu0VecLenXY @ 0x0075_fde0 => extern "C" fn ke_vu0_vec_len_xy(v: u32) -> f32 {
        vu0::bridge::ke_vu0_vec_len_xy(&mem(), v)
    }
    /// `KeVu0Sqrt(x)`.
    KeVu0Sqrt @ 0x0075_fbf0 => extern "C" fn ke_vu0_sqrt(x: u32) -> f32 {
        vu0::bridge::ke_vu0_sqrt(x)
    }
    /// `KeMdl_MkTkClut(a, b, c)`.
    KeMdl_MkTkClut @ 0x0076_7530 => extern "C" fn ke_mdl_mk_tk_clut(a: u32, b: u32, c: u32) {
        vu0::bridge::ke_mdl_mk_tk_clut(&mut mem(), a, b, c);
    }
    /// `NkzVersion()`.
    NkzVersion @ 0x00a5_cf00 => extern "C" fn nkz_version() -> u32 {
        nkz::bridge::nkz_version(&mem())
    }
    /// `NkzGetLuluLimitTime()`.
    NkzGetLuluLimitTime @ 0x00a5_cb10 => extern "C" fn nkz_get_lulu_limit_time() -> f32 {
        nkz::bridge::nkz_get_lulu_limit_time(&mem())
    }
    /// `NkzGetAuronLimitTime(level)`.
    NkzGetAuronLimitTime @ 0x00a5_ca70 => extern "C" fn nkz_get_auron_limit_time(level: i32) -> f32 {
        nkz::bridge::nkz_get_auron_limit_time(&mem(), level)
    }
    /// `NkzGetTidusLimitTime(level)`.
    NkzGetTidusLimitTime @ 0x00a5_cbd0 => extern "C" fn nkz_get_tidus_limit_time(level: i32) -> f32 {
        nkz::bridge::nkz_get_tidus_limit_time(&mem(), level)
    }
    /// `NkzGetAuronLimitButtonData(n)`.
    NkzGetAuronLimitButtonData @ 0x00a5_ca60 => extern "C" fn nkz_get_auron_limit_button_data(n: u32) -> u32 {
        nkz::bridge::nkz_get_auron_limit_button_data(&mem(), n)
    }
    /// `SqsReportIopSoundStatus(i)`.
    SqsReportIopSoundStatus @ 0x0081_e250 => extern "C" fn sqs_report_iop_sound_status(i: u32) -> u32 {
        nkz::bridge::sqs_report_iop_sound_status(&mem(), i)
    }
    /// `SqsIopSoundSystemStatus()`.
    SqsIopSoundSystemStatus @ 0x0081_d710 => extern "C" fn sqs_iop_sound_system_status() -> u32 {
        nkz::bridge::sqs_iop_sound_system_status(&mem())
    }
    /// `SqsIopSoundErrorStatus()`.
    SqsIopSoundErrorStatus @ 0x0081_d700 => extern "C" fn sqs_iop_sound_error_status() -> u32 {
        nkz::bridge::sqs_iop_sound_error_status(&mem())
    }
    /// `rcbgGuideSetPos(guide, x, y)`.
    rcbgGuideSetPos @ 0x0091_d900 => extern "C" fn rcbg_guide_set_pos(g: u32, x: u32, y: u32) {
        guide::bridge::rcbg_guide_set_pos(&mut mem(), g, x, y);
    }
    /// `rcbgGuideSetSize(guide, size)`.
    rcbgGuideSetSize @ 0x0091_d920 => extern "C" fn rcbg_guide_set_size(g: u32, size: u32) {
        guide::bridge::rcbg_guide_set_size(&mut mem(), g, size);
    }
    /// `rcbgGuideSetTarget(guide, target)`.
    rcbgGuideSetTarget @ 0x0091_d930 => extern "C" fn rcbg_guide_set_target(g: u32, target: u32) {
        guide::bridge::rcbg_guide_set_target(&mut mem(), g, target);
    }
    /// `rcbgGuideGetTarget()`.
    rcbgGuideGetTarget @ 0x0091_d480 => extern "C" fn rcbg_guide_get_target() -> u32 {
        guide::bridge::rcbg_guide_get_target(&mem())
    }
    /// `rcbgGuideSetFlags(guide, bits)`.
    rcbgGuideSetFlags @ 0x0091_d430 => extern "C" fn rcbg_guide_set_flags(g: u32, bits: u32) {
        guide::bridge::rcbg_guide_set_flags(&mut mem(), g, bits);
    }
    /// `rcbgGuideClearFlags(guide, bits)`.
    rcbgGuideClearFlags @ 0x0091_d420 => extern "C" fn rcbg_guide_clear_flags(g: u32, bits: u32) {
        guide::bridge::rcbg_guide_clear_flags(&mut mem(), g, bits);
    }
    /// `rcbgGuideSetFlag3(guide, on)`.
    rcbgGuideSetFlag3 @ 0x0091_d510 => extern "C" fn rcbg_guide_set_flag3(g: u32, on: i32) {
        guide::bridge::rcbg_guide_set_flag3(&mut mem(), g, on);
    }
    /// `rcbgGuideGetLimit(guide)`.
    rcbgGuideGetLimit @ 0x0091_d450 => extern "C" fn rcbg_guide_get_limit(g: u32) -> f32 {
        guide::bridge::rcbg_guide_get_limit(&mem(), g)
    }
    /// `rcbgGuideSetLimit(guide, limit)`.
    rcbgGuideSetLimit @ 0x0091_d580 => extern "C" fn rcbg_guide_set_limit(g: u32, limit: u32) {
        guide::bridge::rcbg_guide_set_limit(&mut mem(), g, limit);
    }
    /// `Guide_SetEnabled(on)`.
    Guide_SetEnabled @ 0x0091_f640 => extern "C" fn guide_set_enabled(on: u32) {
        guide::bridge::guide_set_enabled(&mut mem(), on);
    }
    /// `Yn_GuideMapSetMode(target)`.
    Yn_GuideMapSetMode @ 0x0091_9e20 => extern "C" fn yn_guide_map_set_mode(target: u32) {
        guide::bridge::yn_guide_map_set_mode(&mut mem(), target);
    }
    /// `Yn_GuideMapSetFlags(bits)`.
    Yn_GuideMapSetFlags @ 0x0091_9c30 => extern "C" fn yn_guide_map_set_flags(bits: u32) {
        guide::bridge::yn_guide_map_set_flags(&mut mem(), bits);
    }
    /// `Yn_GuideMapClearFlags(bits)`.
    Yn_GuideMapClearFlags @ 0x0091_9c00 => extern "C" fn yn_guide_map_clear_flags(bits: u32) {
        guide::bridge::yn_guide_map_clear_flags(&mut mem(), bits);
    }
    /// `Yn_GuideMapSetFlag3(on)`.
    Yn_GuideMapSetFlag3 @ 0x0091_9d40 => extern "C" fn yn_guide_map_set_flag3(on: i32) {
        guide::bridge::yn_guide_map_set_flag3(&mut mem(), on);
    }
    /// `Yn_GuideMapGetLimit()`.
    Yn_GuideMapGetLimit @ 0x0091_9ca0 => extern "C" fn yn_guide_map_get_limit() -> f32 {
        guide::bridge::yn_guide_map_get_limit(&mem())
    }
    /// `Yn_GuideMapSetLimit(limit)`.
    Yn_GuideMapSetLimit @ 0x0091_9df0 => extern "C" fn yn_guide_map_set_limit(limit: u32) {
        guide::bridge::yn_guide_map_set_limit(&mut mem(), limit);
    }
    /// `Yn_GuideMapSetPos(x, y)`.
    Yn_GuideMapSetPos @ 0x0091_9e50 => extern "C" fn yn_guide_map_set_pos(x: u32, y: u32) {
        guide::bridge::yn_guide_map_set_pos(&mut mem(), x, y);
    }
    /// `Yn_GuideMapSetSize(size)`.
    Yn_GuideMapSetSize @ 0x0091_9e80 => extern "C" fn yn_guide_map_set_size(size: u32) {
        guide::bridge::yn_guide_map_set_size(&mut mem(), size);
    }
    /// `Yn_MapPartCount()`.
    Yn_MapPartCount @ 0x0091_a3d0 => extern "C" fn yn_map_part_count() -> i32 {
        guide::bridge::yn_map_part_count(&mem())
    }
    /// `Yn_MapPartAnimeOn(part)`.
    Yn_MapPartAnimeOn @ 0x0091_9f90 => extern "C" fn yn_map_part_anime_on(part: i32) {
        guide::bridge::yn_map_part_anime_on(&mut mem(), part);
    }
    /// `Yn_MapPartAnimeOff(part)`.
    Yn_MapPartAnimeOff @ 0x0091_9f40 => extern "C" fn yn_map_part_anime_off(part: i32) {
        guide::bridge::yn_map_part_anime_off(&mut mem(), part);
    }
    /// `Yn_PSimGetCnt()`.
    Yn_PSimGetCnt @ 0x0091_9ef0 => extern "C" fn yn_psim_get_cnt() -> u32 {
        guide::bridge::yn_psim_get_cnt()
    }
    /// `Yn_SeaGetMatrix()`.
    Yn_SeaGetMatrix @ 0x0091_ae00 => extern "C" fn yn_sea_get_matrix() -> u32 {
        guide::bridge::yn_sea_get_matrix(&mem())
    }
    /// `rcbgSceneInit(scene)`.
    rcbgSceneInit @ 0x0092_09e0 => extern "C" fn rcbg_scene_init(s: u32) {
        scene::bridge::rcbg_scene_init(&mut mem(), s);
    }
    /// `rcbgSceneGetObject(scene, kind, n)`.
    rcbgSceneGetObject @ 0x0092_0950 => extern "C" fn rcbg_scene_get_object(s: u32, kind: u32, n: u32) -> u32 {
        scene::bridge::rcbg_scene_get_object(&mem(), s, kind, n)
    }
    /// `rcbgSceneCountObjects(scene, kind)`.
    rcbgSceneCountObjects @ 0x0092_09a0 => extern "C" fn rcbg_scene_count_objects(s: u32, kind: u32) -> u32 {
        scene::bridge::rcbg_scene_count_objects(&mem(), s, kind)
    }
    /// `rcbgChunkTag(i)`.
    rcbgChunkTag @ 0x0092_1b90 => extern "C" fn rcbg_chunk_tag(i: u32) -> u32 {
        scene::bridge::rcbg_chunk_tag(&mem(), i)
    }
    /// `rcbgLayerInit(layer)`.
    rcbgLayerInit @ 0x0092_ad20 => extern "C" fn rcbg_layer_init(l: u32) {
        scene::bridge::rcbg_layer_init(&mut mem(), l);
    }
    /// `rcdtParseSetMode(parse, mode)`.
    rcdtParseSetMode @ 0x0092_c0c0 => extern "C" fn rcdt_parse_set_mode(p: u32, mode: u32) {
        rcdt::bridge::rcdt_parse_set_mode(&mut mem(), p, mode);
    }
    /// `rcdtParseAddSeparators(parse, chars)`.
    rcdtParseAddSeparators @ 0x0092_c0f0 => extern "C" fn rcdt_parse_add_separators(p: u32, chars: u32) {
        rcdt::bridge::rcdt_parse_add_separators(&mut mem(), p, chars);
    }
    /// `rcdtChunkSet(chunk, data, size)`.
    rcdtChunkSet @ 0x0092_b920 => extern "C" fn rcdt_chunk_set(c: u32, data: u32, size: u32) {
        rcdt::bridge::rcdt_chunk_set(&mut mem(), c, data, size);
    }
    /// `rcdtChunkGetEnd(chunk)`.
    rcdtChunkGetEnd @ 0x0092_b820 => extern "C" fn rcdt_chunk_get_end(c: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_get_end(&mem(), c)
    }
    /// `rcdtChunkIsEmpty(chunk)`.
    rcdtChunkIsEmpty @ 0x0092_b860 => extern "C" fn rcdt_chunk_is_empty(c: u32) -> u32 {
        u32::from(rcdt::bridge::rcdt_chunk_is_empty(&mem(), c))
    }
    /// `rcdtTm2Init(tm2)`.
    rcdtTm2Init @ 0x0092_37e0 => extern "C" fn rcdt_tm2_init(t: u32) {
        rcdt::bridge::rcdt_tm2_init(&mut mem(), t);
    }
    /// `rcbgModelGetScale(model)`.
    rcbgModelGetScale @ 0x0092_8300 => extern "C" fn rcbg_model_get_scale(m: u32) -> f32 {
        model::bridge::rcbg_model_get_scale(&mem(), m)
    }
    /// `rcbgTrxGetCnt(trx)`.
    rcbgTrxGetCnt @ 0x0092_d4c0 => extern "C" fn rcbg_trx_get_cnt(t: u32) -> u32 {
        model::bridge::rcbg_trx_get_cnt(&mem(), t)
    }
    /// `rcbgTrxGetEntry(trx, n)`.
    rcbgTrxGetEntry @ 0x0092_d4d0 => extern "C" fn rcbg_trx_get_entry(t: u32, n: u32) -> u32 {
        model::bridge::rcbg_trx_get_entry(&mem(), t, n)
    }
    /// `rcbgTrxGetEntryData(trx, n)`.
    rcbgTrxGetEntryData @ 0x0092_d4f0 => extern "C" fn rcbg_trx_get_entry_data(t: u32, n: u32) -> u32 {
        model::bridge::rcbg_trx_get_entry_data(&mem(), t, n)
    }
    /// `rcbgPSimInit(psim)`.
    rcbgPSimInit @ 0x0091_fe00 => extern "C" fn rcbg_psim_init(p: u32) {
        model::bridge::rcbg_psim_init(&mut mem(), p);
    }
    /// `rcbgPSimGetOmdIn(psim)`.
    rcbgPSimGetOmdIn @ 0x0091_fdc0 => extern "C" fn rcbg_psim_get_omd_in(p: u32) -> u32 {
        model::bridge::rcbg_psim_get_omd_in(&mem(), p)
    }
    /// `rcbgFPm173End(fp)`.
    rcbgFPm173End @ 0x0092_2b50 => extern "C" fn rcbg_fpm173_end(f: u32) {
        model::bridge::rcbg_fpm173_end(&mut mem(), f);
    }
    /// `rcPad16(n)`.
    rcPad16 @ 0x0092_b2c0 => extern "C" fn rc_pad16(n: i32) -> i32 {
        model::pad16(n)
    }
    /// `rcLog2Ceil(n)`.
    rcLog2Ceil @ 0x0092_9910 => extern "C" fn rc_log2_ceil(n: i32) -> u32 {
        model::log2_ceil(n)
    }
    /// `rcefModelInit(model)`.
    rcefModelInit @ 0x0092_f750 => extern "C" fn rcef_model_init(m: u32) {
        task::bridge::rcef_model_init(&mut mem(), m);
    }
    /// `rcefIsBattle()`.
    rcefIsBattle @ 0x0092_e620 => extern "C" fn rcef_is_battle() -> u32 {
        u32::from(task::bridge::rcef_is_battle(&mem()))
    }
    /// `rcefDirToAngle(dir)`.
    rcefDirToAngle @ 0x0092_de30 => extern "C" fn rcef_dir_to_angle(dir: u32) -> f32 {
        task::bridge::rcef_dir_to_angle(&mem(), dir)
    }
    /// `rcVec3fDot(a, b)`.
    rcVec3fDot @ 0x0092_1220 => extern "C" fn rc_vec3f_dot(a: u32, b: u32) -> f32 {
        geom::bridge::rc_vec3f_dot(&mem(), a, b)
    }
    /// `rcBBoxSet(box, points, n)`.
    rcBBoxSet @ 0x0092_1530 => extern "C" fn rc_bbox_set(b: u32, points: u32, n: i32) {
        geom::bridge::rc_bbox_set(&mut mem(), b, points, n);
    }
    /// `rcCalcPersMatrix(dst, m)`.
    rcCalcPersMatrix @ 0x0092_0020 => extern "C" fn rc_calc_pers_matrix(dst: u32, m: u32) {
        geom::bridge::rc_calc_pers_matrix(&mut mem(), dst, m);
    }
    /// `rcbgUpdateMatrix()`.
    rcbgUpdateMatrix @ 0x0092_b160 => extern "C" fn rcbg_update_matrix() {
        geom::bridge::rcbg_update_matrix(&mut mem());
    }
    /// `yiSetRenderParam(v)`.
    yiSetRenderParam @ 0x0090_8580 => extern "C" fn yi_set_render_param(v: u32) {
        yi::bridge::yi_set_render_param(&mut mem(), v);
    }
    /// `yiCheckDObj(index)`.
    yiCheckDObj @ 0x0090_86b0 => extern "C" fn yi_check_dobj(index: u32) -> i32 {
        yi::bridge::yi_check_dobj(&mem(), index)
    }
    /// `yiSetColor3(r, g, b)`: the low bytes.
    yiSetColor3 @ 0x0090_a400 => extern "C" fn yi_set_color3(r: u32, g: u32, b: u32) -> u32 {
        let [r, g, b] = [r, g, b].map(|c| c.to_le_bytes()[0]);
        yi::bridge::yi_set_color3(&mut mem(), r, g, b)
    }
    /// `rcuiMouseMoveEnd()`.
    rcuiMouseMoveEnd @ 0x0093_c050 => extern "C" fn rcui_mouse_move_end() {
        rcui::bridge::rcui_mouse_move_end(&mut mem());
    }
    /// `rcuiMouseMoveReset()`: the same code.
    rcuiMouseMoveReset @ 0x0093_c110 => extern "C" fn rcui_mouse_move_reset() {
        rcui::bridge::rcui_mouse_move_end(&mut mem());
    }
    /// `rcuiMouseMoveGetID()`.
    rcuiMouseMoveGetID @ 0x0093_c100 => extern "C" fn rcui_mouse_move_get_id() -> u32 {
        rcui::bridge::rcui_mouse_move_get_id(&mem())
    }
    /// `rcuiMouseMoveIsID(id)`.
    rcuiMouseMoveIsID @ 0x0093_c120 => extern "C" fn rcui_mouse_move_is_id(id: u32) -> u32 {
        u32::from(rcui::bridge::rcui_mouse_move_is_id(&mem(), id))
    }
    /// `rcuiNumberIsOK(input)`.
    rcuiNumberIsOK @ 0x0093_c3d0 => extern "C" fn rcui_number_is_ok(input: u32) -> u32 {
        rcui::bridge::rcui_number_is_ok(&mut mem(), input)
    }
    /// `rcuiCopyVector(dst, src)`.
    rcuiCopyVector @ 0x0093_ce50 => extern "C" fn rcui_copy_vector(dst: u32, src: u32) {
        rcui::bridge::rcui_copy_vector(&mut mem(), dst, src);
    }
    /// `rcuiCopyMatrix(dst, src)`.
    rcuiCopyMatrix @ 0x0093_ce10 => extern "C" fn rcui_copy_matrix(dst: u32, src: u32) {
        rcui::bridge::rcui_copy_matrix(&mut mem(), dst, src);
    }
    /// `KeThResHd_Init(hd, count, size)`: the low bytes of both.
    KeThResHd_Init @ 0x0076_4bb0 => extern "C" fn ke_th_res_hd_init(hd: u32, count: u32, size: u32) {
        let [c, _, _, _] = count.to_le_bytes();
        let [s, _, _, _] = size.to_le_bytes();
        ke::bridge::ke_th_res_hd_init(&mut mem(), hd, c, s);
    }
    /// `KeThCp_Init(cp)`.
    KeThCp_Init @ 0x0076_4800 => extern "C" fn ke_th_cp_init(cp: u32) {
        ke::bridge::ke_th_cp_init(&mut mem(), cp);
    }
    /// `KeLnsFls_Init(obj)`.
    KeLnsFls_Init @ 0x0076_9690 => extern "C" fn ke_lns_fls_init(o: u32) {
        ke::bridge::ke_lns_init(&mut mem(), o);
    }
    /// `KeLnsCrn_Init(obj)`: the same code.
    KeLnsCrn_Init @ 0x0076_a170 => extern "C" fn ke_lns_crn_init(o: u32) {
        ke::bridge::ke_lns_init(&mut mem(), o);
    }
    /// `KeLnsClm_Init(obj)`.
    KeLnsClm_Init @ 0x0076_9ca0 => extern "C" fn ke_lns_clm_init(o: u32) {
        ke::bridge::ke_lns_clm_init(&mut mem(), o);
    }
    /// `KeLnsFlr_Init(obj, src)`.
    KeLnsFlr_Init @ 0x0076_a1d0 => extern "C" fn ke_lns_flr_init(o: u32, src: u32) {
        ke::bridge::ke_lns_flr_init(&mut mem(), o, src);
    }
    /// `rcdtChunkGetSize(chunk)`.
    rcdtChunkGetSize @ 0x0092_b830 => extern "C" fn rcdt_chunk_get_size(c: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_get_size(&mem(), c)
    }
    /// `rcdtParseGetChunk(parse)`.
    rcdtParseGetChunk @ 0x0092_bb20 => extern "C" fn rcdt_parse_get_chunk(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_chunk(&mem(), p)
    }
    /// `rcdtParseGetPos(parse)`.
    rcdtParseGetPos @ 0x0092_bda0 => extern "C" fn rcdt_parse_get_pos(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_pos(&mem(), p)
    }
    /// `rcdtParseGetMode(parse)`.
    rcdtParseGetMode @ 0x0092_bd10 => extern "C" fn rcdt_parse_get_mode(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_mode(&mem(), p)
    }
    /// `rcPacketDataSetAdr(packet, at)`.
    rcPacketDataSetAdr @ 0x0092_7a40 => extern "C" fn rc_packet_data_set_adr(p: u32, at: u32) {
        packet::bridge::rc_packet_data_set_adr(&mut mem(), p, at);
    }
    /// `rcPacketDataOpen(packet, buffer)`.
    rcPacketDataOpen @ 0x0092_7a50 => extern "C" fn rc_packet_data_open(p: u32, buffer: u32) {
        packet::bridge::rc_packet_data_open(&mut mem(), p, buffer);
    }
    /// `yiSetMatrixDObj(index, m)`.
    yiSetMatrixDObj @ 0x0090_a520 => extern "C" fn yi_set_matrix_dobj(index: u32, m: u32) -> i32 {
        yi::bridge::yi_set_matrix_dobj(&mut mem(), index, m)
    }
    /// `yiSetMatrixGroup(group, m)`.
    yiSetMatrixGroup @ 0x0090_a560 => extern "C" fn yi_set_matrix_group(group: u32, m: u32) -> i32 {
        yi::bridge::yi_set_matrix_group(&mut mem(), group, m)
    }
    /// `yiGetDObjGroup(index)`.
    yiGetDObjGroup @ 0x0090_8c50 => extern "C" fn yi_get_dobj_group(index: u32) -> u32 {
        yi::bridge::yi_get_dobj_group(&mem(), index)
    }
    /// `yiGetColorDObj(index, a, b)`.
    yiGetColorDObj @ 0x0090_8e90 => extern "C" fn yi_get_color_dobj(index: u32, a: u32, b: u32) -> i32 {
        yi::bridge::yi_get_color_dobj(&mut mem(), index, a, b)
    }
    /// `yiSetColorDObj(index, a, b)`.
    yiSetColorDObj @ 0x0090_a5c0 => extern "C" fn yi_set_color_dobj(index: u32, a: u32, b: u32) -> i32 {
        yi::bridge::yi_set_color_dobj(&mut mem(), index, a, b)
    }
    /// `yiGetViewAdjust(out)`.
    yiGetViewAdjust @ 0x0090_8f20 => extern "C" fn yi_get_view_adjust(out: u32) {
        yi::bridge::yi_get_view_adjust(&mut mem(), out);
    }
    /// `yiSetViewAdjust(m)`.
    yiSetViewAdjust @ 0x0090_a730 => extern "C" fn yi_set_view_adjust(m: u32) {
        yi::bridge::yi_set_view_adjust(&mut mem(), m);
    }
    /// `yiSetColorOffset(rgba)`.
    yiSetColorOffset @ 0x0090_9ff0 => extern "C" fn yi_set_color_offset(rgba: u32) {
        yi::bridge::yi_set_color_offset(&mut mem(), rgba);
    }
    /// `yiPkAddGsAD(q, reg, data)`.
    yiPkAddGsAD @ 0x0090_ae20 => extern "C" fn yi_pk_add_gs_ad(q: u32, reg: u32, data: u32) {
        yi::bridge::yi_pk_add_gs_ad(&mut mem(), q, reg, data);
    }
    /// `yiPkAddGsAD_0090bc40(q, reg, data)`: the same code.
    yiPkAddGsAD_0090bc40 @ 0x0090_bc40 => extern "C" fn yi_pk_add_gs_ad2(q: u32, reg: u32, data: u32) {
        yi::bridge::yi_pk_add_gs_ad(&mut mem(), q, reg, data);
    }
    /// `yiPkAddGsAD_00910040(q, reg, data)`: the same code.
    yiPkAddGsAD_00910040 @ 0x0091_0040 => extern "C" fn yi_pk_add_gs_ad3(q: u32, reg: u32, data: u32) {
        yi::bridge::yi_pk_add_gs_ad(&mut mem(), q, reg, data);
    }
    /// `yiPow2Ceil(n)`.
    yiPow2Ceil @ 0x0091_7460 => extern "C" fn yi_pow2_ceil(n: i32) -> i32 {
        yi::pow2_ceil(n)
    }
    /// `yiHighestBit(bits)`.
    yiHighestBit @ 0x0091_0580 => extern "C" fn yi_highest_bit(bits: u32) -> i32 {
        yi::highest_bit(bits)
    }
    /// `yiLog2Ceil16(bits)`.
    yiLog2Ceil16 @ 0x0091_7430 => extern "C" fn yi_log2_ceil16(bits: u32) -> i32 {
        yi::log2_ceil16(bits)
    }
    /// `yiIsPow2(bits)`.
    yiIsPow2 @ 0x0091_73e0 => extern "C" fn yi_is_pow2(bits: u32) -> u32 {
        u32::from(yi::is_pow2(bits))
    }
    /// `yiVramAlloc(bytes)`.
    yiVramAlloc @ 0x0091_8130 => extern "C" fn yi_vram_alloc(bytes: i32) -> u32 {
        yi::bridge::yi_vram_alloc(&mut mem(), bytes)
    }
    /// `yiVramSetLimit(pages)`.
    yiVramSetLimit @ 0x0091_9630 => extern "C" fn yi_vram_set_limit(pages: u32) -> u32 {
        yi::bridge::yi_vram_set_limit(&mut mem(), pages)
    }
    /// `rcbgGuideSetFlag8(guide, on)`.
    rcbgGuideSetFlag8 @ 0x0091_d530 => extern "C" fn rcbg_guide_set_flag8(g: u32, on: i32) {
        guide::bridge::rcbg_guide_set_flag8(&mut mem(), g, on);
    }
    /// `rcbgGuideGetScreen(guide, scale, offset)`.
    rcbgGuideGetScreen @ 0x0091_da30 => extern "C" fn rcbg_guide_get_screen(g: u32, scale: u32, offset: u32) {
        guide::bridge::rcbg_guide_get_screen(&mut mem(), g, scale, offset);
    }
    /// `Yn_SceneGetAC()`.
    Yn_SceneGetAC @ 0x0091_a6d0 => extern "C" fn yn_scene_get_ac() -> f32 {
        guide::bridge::yn_scene_get_ac(&mem())
    }
    /// `Yn_SceneGetB8()`.
    Yn_SceneGetB8 @ 0x0091_a700 => extern "C" fn yn_scene_get_b8() -> f32 {
        guide::bridge::yn_scene_get_b8(&mem())
    }
    /// `Yn_SceneGetBC()`.
    Yn_SceneGetBC @ 0x0091_a730 => extern "C" fn yn_scene_get_bc() -> f32 {
        guide::bridge::yn_scene_get_bc(&mem())
    }
    /// `Yn_SceneSetC0(a, b, c, d, e)`.
    Yn_SceneSetC0 @ 0x0091_ad80 => extern "C" fn yn_scene_set_c0(a: u32, b: u32, c: u32, d: u32, e: u32) {
        guide::bridge::yn_scene_set_c0(&mut mem(), [a, b, c, d, e]);
    }
    /// `Yn_SceneSetDC(a, b, c, d)`.
    Yn_SceneSetDC @ 0x0091_aa00 => extern "C" fn yn_scene_set_dc(a: u32, b: u32, c: u32, d: u32) {
        guide::bridge::yn_scene_set_dc(&mut mem(), a, b, c, d);
    }
    /// `Yn_SceneSet104(v)`.
    Yn_SceneSet104 @ 0x0091_a260 => extern "C" fn yn_scene_set_104(v: u32) {
        guide::bridge::yn_scene_set_104(&mut mem(), v);
    }
    /// `rcAngleDiff(a, b)`.
    rcAngleDiff @ 0x0092_e400 => extern "C" fn rc_angle_diff(a: u32, b: u32) -> f32 {
        task::bridge::rc_angle_diff(&mem(), a, b)
    }
    /// `rcefTargetIsDone(i)`.
    rcefTargetIsDone @ 0x0092_4ba0 => extern "C" fn rcef_target_is_done(i: u32) -> u32 {
        task::bridge::rcef_target_is_done(&mem(), i)
    }
    /// `rcefTargetClear(i)`.
    rcefTargetClear @ 0x0092_4bc0 => extern "C" fn rcef_target_clear(i: u32) {
        task::bridge::rcef_target_clear(&mut mem(), i);
    }
    /// `rcefTargetGetDist(i)`.
    rcefTargetGetDist @ 0x0092_4a00 => extern "C" fn rcef_target_get_dist(i: u32) -> f32 {
        task::bridge::rcef_target_get_dist(&mem(), i)
    }
    /// `rcefTargetGetAngle(i)`.
    rcefTargetGetAngle @ 0x0092_49e0 => extern "C" fn rcef_target_get_angle(i: u32) -> f32 {
        task::bridge::rcef_target_get_angle(&mem(), i)
    }
    /// `rcefTargetGetHeight(i)`.
    rcefTargetGetHeight @ 0x0092_49c0 => extern "C" fn rcef_target_get_height(i: u32) -> f32 {
        task::bridge::rcef_target_get_height(&mem(), i)
    }
    /// `rcefTargetSetDelay(frames)`: the low 16 bits.
    rcefTargetSetDelay @ 0x0092_4be0 => extern "C" fn rcef_target_set_delay(frames: u32) {
        let [lo, hi, _, _] = frames.to_le_bytes();
        task::bridge::rcef_target_set_delay(&mut mem(), u16::from_le_bytes([lo, hi]));
    }
    /// `rcefMainSetEnv(main)`.
    rcefMainSetEnv @ 0x0092_54f0 => extern "C" fn rcef_main_set_env(m: u32) {
        task::bridge::rcef_main_set_env(&mut mem(), m);
    }
    /// `rcefMainSetEnv_00925510(main)`: the same code.
    rcefMainSetEnv_00925510 @ 0x0092_5510 => extern "C" fn rcef_main_set_env2(m: u32) {
        task::bridge::rcef_main_set_env(&mut mem(), m);
    }
    /// `rcefObjLinkLast(obj, child)`.
    rcefObjLinkLast @ 0x0092_f920 => extern "C" fn rcef_obj_link_last(o: u32, child: u32) {
        task::bridge::rcef_obj_link_last(&mut mem(), o, child);
    }
    /// `rcbgAnimeGetFrame(anime, n)`.
    rcbgAnimeGetFrame @ 0x0093_0860 => extern "C" fn rcbg_anime_get_frame(a: u32, n: i32) -> u32 {
        model::bridge::rcbg_anime_get_frame(&mem(), a, n)
    }
    /// `rcbgAnimeGetCurrent(anime)`.
    rcbgAnimeGetCurrent @ 0x0093_0880 => extern "C" fn rcbg_anime_get_current(a: u32) -> u32 {
        model::bridge::rcbg_anime_get_current(&mem(), a)
    }
    /// `rcVec3fApplyMatrix(out, v, m)`.
    rcVec3fApplyMatrix @ 0x0092_1300 => extern "C" fn rc_vec3f_apply_matrix(out: u32, v: u32, m: u32) {
        geom::bridge::rc_vec3f_apply_matrix(&mut mem(), out, v, m);
    }
    /// `rcVec3fLerp(out, a, b, t)`.
    rcVec3fLerp @ 0x0092_1280 => extern "C" fn rc_vec3f_lerp(out: u32, a: u32, b: u32, t: u32) {
        geom::bridge::rc_vec3f_lerp(&mut mem(), out, a, b, t);
    }
    /// `yiLerpVec4(t, a, b, out)`.
    yiLerpVec4 @ 0x0091_48d0 => extern "C" fn yi_lerp_vec4(t: u32, a: u32, b: u32, out: u32) {
        geom::bridge::yi_lerp_vec4(&mut mem(), t, a, b, out);
    }
    /// `yiLerpVec2(t, a, b, out)`.
    yiLerpVec2 @ 0x0091_48a0 => extern "C" fn yi_lerp_vec2(t: u32, a: u32, b: u32, out: u32) {
        geom::bridge::yi_lerp_vec2(&mut mem(), t, a, b, out);
    }
    /// `rcuiValueGet(p, kind)`.
    rcuiValueGet @ 0x0093_c210 => extern "C" fn rcui_value_get(p: u32, kind: u32) -> u32 {
        rcui::bridge::rcui_value_get(&mem(), p, kind)
    }
    /// `rcuiValueSet(p, kind, v)`.
    rcuiValueSet @ 0x0093_c270 => extern "C" fn rcui_value_set(p: u32, kind: u32, v: u32) {
        rcui::bridge::rcui_value_set(&mut mem(), p, kind, v);
    }
    /// `rcuiTake18(item)`.
    rcuiTake18 @ 0x0093_c800 => extern "C" fn rcui_take18(item: u32) -> u32 {
        rcui::bridge::rcui_take(&mut mem(), item, 0x18)
    }
    /// `rcuiTake1C(item)`.
    rcuiTake1C @ 0x0093_c820 => extern "C" fn rcui_take1c(item: u32) -> u32 {
        rcui::bridge::rcui_take(&mut mem(), item, 0x1c)
    }
    /// `rcuiTake14(item)`.
    rcuiTake14 @ 0x0093_c840 => extern "C" fn rcui_take14(item: u32) -> u32 {
        rcui::bridge::rcui_take(&mut mem(), item, 0x14)
    }
    /// `rcuiItemInit(item)`.
    rcuiItemInit @ 0x0093_c2c0 => extern "C" fn rcui_item_init(item: u32) {
        rcui::bridge::rcui_item_init(&mut mem(), item);
    }
    /// `rcuiAddVector(out, a, b)`.
    rcuiAddVector @ 0x0093_cbc0 => extern "C" fn rcui_add_vector(out: u32, a: u32, b: u32) {
        rcui::bridge::rcui_add_vector(&mut mem(), out, a, b);
    }
    /// `rcuiMulVector(out, a, b)`.
    rcuiMulVector @ 0x0093_d290 => extern "C" fn rcui_mul_vector(out: u32, a: u32, b: u32) {
        rcui::bridge::rcui_mul_vector(&mut mem(), out, a, b);
    }
    /// `rcuiItoF(out, ints)`.
    rcuiItoF @ 0x0093_d030 => extern "C" fn rcui_itof(out: u32, ints: u32) {
        rcui::bridge::rcui_itof(&mut mem(), out, ints);
    }
    /// `rcuiDot(a, b)`.
    rcuiDot @ 0x0093_d060 => extern "C" fn rcui_dot(a: u32, b: u32) -> f32 {
        rcui::bridge::rcui_dot(&mem(), a, b)
    }
    /// `rcuiVu0CopyVector(dst, src)`.
    rcuiVu0CopyVector @ 0x0093_82e0 => extern "C" fn rcui_vu0_copy_vector(dst: u32, src: u32) {
        rcui::bridge::rcui_vu0_copy_vector(&mut mem(), dst, src);
    }
    /// `yiRender_addRef(render, addr, qwc)`: the low 16 bits of `qwc`.
    yiRender_addRef @ 0x0090_c1f0 => extern "C" fn yi_render_add_ref(r: u32, addr: u32, qwc: u32) {
        let [lo, hi, _, _] = qwc.to_le_bytes();
        yi::bridge::yi_render_add_ref(&mut mem(), r, addr, u16::from_le_bytes([lo, hi]));
    }
    /// `yiRender_addRefData(render, block)`.
    yiRender_addRefData @ 0x0090_c230 => extern "C" fn yi_render_add_ref_data(r: u32, block: u32) {
        yi::bridge::yi_render_add_ref_data(&mut mem(), r, block);
    }
    /// `yiPkUnpackMatrix(m, out, addr)`.
    yiPkUnpackMatrix @ 0x0090_cd10 => extern "C" fn yi_pk_unpack_matrix(m: u32, out: u32, addr: u32) {
        yi::bridge::yi_pk_unpack_matrix(&mut mem(), m, out, addr);
    }
    /// `yiPkUnpackMatrixDma(m, out, addr)`.
    yiPkUnpackMatrixDma @ 0x0090_ccb0 => extern "C" fn yi_pk_unpack_matrix_dma(m: u32, out: u32, addr: u32) {
        yi::bridge::yi_pk_unpack_matrix_dma(&mut mem(), m, out, addr);
    }
    // ---- round 4 ----
    /// `yiGetColorScale(out)`.
    yiGetColorScale @ 0x0090_88d0 => extern "C" fn yi_get_color_scale(out: u32) -> u32 {
        yi::bridge::yi_get_color_scale(&mut mem(), out)
    }
    /// `yiSetColorScale(v)`.
    yiSetColorScale @ 0x0090_a270 => extern "C" fn yi_set_color_scale(v: u32) -> u32 {
        yi::bridge::yi_set_color_scale(&mut mem(), v)
    }
    /// `yiSetColorShift(out)`.
    yiSetColorShift @ 0x0090_8c30 => extern "C" fn yi_set_color_shift(out: u32) -> u32 {
        yi::bridge::yi_set_color_shift(&mut mem(), out)
    }
    /// `yiSetBattleCenter(v)`.
    yiSetBattleCenter @ 0x0090_a0e0 => extern "C" fn yi_set_battle_center(v: u32) -> u32 {
        yi::bridge::yi_set_battle_center(&mut mem(), v)
    }
    /// `yiGetBattleCenter(out)`.
    yiGetBattleCenter @ 0x0090_87f0 => extern "C" fn yi_get_battle_center(out: u32) -> u32 {
        yi::bridge::yi_get_battle_center(&mut mem(), out)
    }
    /// `yiPushRenderState()`.
    yiPushRenderState @ 0x0090_9c60 => extern "C" fn yi_push_render_state() -> u32 {
        yi::bridge::yi_push_render_state(&mut mem())
    }
    /// `yiPopRenderState()`.
    yiPopRenderState @ 0x0090_9b90 => extern "C" fn yi_pop_render_state() -> u32 {
        yi::bridge::yi_pop_render_state(&mut mem())
    }
    /// `yiGetBackColor(out)`.
    yiGetBackColor @ 0x0090_86d0 => extern "C" fn yi_get_back_color(out: u32) -> u32 {
        yi::bridge::yi_get_back_color(&mut mem(), out)
    }
    /// `yiImage_init(image)`.
    yiImage_init @ 0x0091_88c0 => extern "C" fn yi_image_init(im: u32) -> u32 {
        yi::bridge::yi_image_init(&mut mem(), im)
    }
    /// `yiTexSwap_init(render)`.
    yiTexSwap_init @ 0x0091_52d0 => extern "C" fn yi_tex_swap_init(r: u32) -> u32 {
        yi::bridge::yi_tex_swap_init(&mut mem(), r)
    }
    /// `yiTexSwapSetFrame(index, frame)`.
    yiTexSwapSetFrame @ 0x0090_9e30 => extern "C" fn yi_tex_swap_set_frame(index: u32, frame: u32) -> u32 {
        yi::bridge::yi_tex_swap_set_frame(&mut mem(), index, frame)
    }
    /// `yiAnimAttachDefault_009143c0(render, id, a, b)`.
    yiAnimAttachDefault_009143c0 @ 0x0091_43c0 => extern "C" fn yi_anim_attach_default(r: u32, id: u32, a: u32, b: u32) -> u32 {
        yi::bridge::yi_anim_attach_default(&mut mem(), r, id, a, b)
    }
    /// `yiKeyFrame_attachChr(render, id, keyframe, arg)`.
    yiKeyFrame_attachChr @ 0x0091_4630 => extern "C" fn yi_key_frame_attach_chr(r: u32, id: u32, kf: u32, arg: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the original stores the low half
        yi::bridge::yi_key_frame_attach_chr(&mut mem(), r, id, kf, arg as u16)
    }
    /// `yiSetFieldVec4i(a, b, c, d)`.
    yiSetFieldVec4i @ 0x0090_a1d0 => extern "C" fn yi_set_field_vec4i(a: i32, b: i32, c: i32, d: i32) {
        yi::bridge::yi_set_field_vec4i(&mut mem(), [a, b, c, d]);
    }
    /// `yiSetFieldVec4(a, b, c, d)`.
    yiSetFieldVec4 @ 0x0090_a200 => extern "C" fn yi_set_field_vec4(a: u32, b: u32, c: u32, d: u32) {
        yi::bridge::yi_set_field_vec4(&mut mem(), [a, b, c, d]);
    }
    /// `yiSetFieldVec6(a, b, c, d, e, f)`.
    yiSetFieldVec6 @ 0x0090_a230 => extern "C" fn yi_set_field_vec6(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) {
        yi::bridge::yi_set_field_vec6(&mut mem(), [a, b, c, d, e, f]);
    }
    /// `yiSetFieldPair(on, a, b)`.
    yiSetFieldPair @ 0x0090_a430 => extern "C" fn yi_set_field_pair(on: u32, a: u32, b: u32) -> u32 {
        yi::bridge::yi_set_field_pair(&mut mem(), on, a, b)
    }
    /// `yiSetFieldVec3Int(a, b, c, n)`.
    yiSetFieldVec3Int @ 0x0090_a470 => extern "C" fn yi_set_field_vec3_int(a: u32, b: u32, c: u32, n: u32) -> u32 {
        yi::bridge::yi_set_field_vec3_int(&mut mem(), a, b, c, n)
    }
    /// `yiMapGetSection(map, index)`.
    yiMapGetSection @ 0x0090_7f00 => extern "C" fn yi_map_get_section(map: u32, index: u32) -> u32 {
        yi::bridge::yi_map_get_section(&mem(), map, index)
    }
    /// `yiVramAllocPsm(bytes, psm)`.
    yiVramAllocPsm @ 0x0091_8090 => extern "C" fn yi_vram_alloc_psm(bytes: i32, psm: u32) -> u32 {
        yi::bridge::yi_vram_alloc_psm(&mut mem(), bytes, psm)
    }
    /// `rcMat4fSetScale(m, x, y, z)`.
    rcMat4fSetScale @ 0x0092_57f0 => extern "C" fn rc_mat4f_set_scale(m: u32, x: u32, y: u32, z: u32) -> u32 {
        geom::bridge::rc_mat4f_set_scale(&mut mem(), m, x, y, z)
    }
    /// `rcbgSetClipMatrix(w, h, d, near, far)`.
    rcbgSetClipMatrix @ 0x0092_b0d0 => extern "C" fn rcbg_set_clip_matrix(w: u32, h: u32, d: u32, near: u32, far: u32) -> u32 {
        let f = f32::from_bits;
        geom::bridge::rcbg_set_clip_matrix(&mut mem(), f(w), f(h), f(d), f(near), f(far))
    }
    /// `rcbgSetWorldMatrix(m)`.
    rcbgSetWorldMatrix @ 0x0092_b140 => extern "C" fn rcbg_set_world_matrix(m: u32) -> u32 {
        geom::bridge::rcbg_set_world_matrix(&mut mem(), m)
    }
    /// `Yn_SeaResetMatrix()`.
    Yn_SeaResetMatrix @ 0x0091_ae30 => extern "C" fn yn_sea_reset_matrix() -> u32 {
        guide::bridge::yn_sea_reset_matrix(&mut mem())
    }
    /// `Yn_SeaSetMatrix(m)`.
    Yn_SeaSetMatrix @ 0x0091_ae60 => extern "C" fn yn_sea_set_matrix(m: u32) -> u32 {
        guide::bridge::yn_sea_set_matrix(&mut mem(), m)
    }
    /// `Yn_PreBgLayerDispGet(part)`.
    Yn_PreBgLayerDispGet @ 0x0091_a770 => extern "C" fn yn_pre_bg_layer_disp_get(part: i32) -> u32 {
        guide::bridge::yn_pre_bg_layer_disp_get(&mem(), part)
    }
    /// `Guide_RevealFog(guide, x, z)`.
    Guide_RevealFog @ 0x0091_f420 => extern "C" fn guide_reveal_fog(g: u32, x: u32, z: u32) -> u32 {
        guide::bridge::guide_reveal_fog(&mut mem(), g, f32::from_bits(x), f32::from_bits(z))
    }
    /// `KeLnsLp_Init(obj)`.
    KeLnsLp_Init @ 0x0076_a670 => extern "C" fn ke_lns_lp_init(o: u32) -> u32 {
        ke::bridge::ke_lns_lp_init(&mut mem(), o)
    }
    /// `KeLnsCrn_SetShp(obj, shp)`.
    KeLnsCrn_SetShp @ 0x0076_a1a0 => extern "C" fn ke_lns_crn_set_shp(o: u32, shp: u32) -> u32 {
        ke::bridge::ke_lns_crn_set_shp(&mut mem(), o, shp)
    }
    /// `KeLnsShp_Draw(obj)`.
    KeLnsShp_Draw @ 0x0076_ab90 => extern "C" fn ke_lns_shp_draw(o: u32) -> u32 {
        ke::bridge::ke_lns_shp_draw(&mut mem(), o)
    }
    /// `KeLnsShp_Update(obj)`.
    KeLnsShp_Update @ 0x0076_abe0 => extern "C" fn ke_lns_shp_update(o: u32) -> u32 {
        ke::bridge::ke_lns_shp_update(&mut mem(), o)
    }
    /// `KeLnsClm_SetShp(obj, shp, seed)`.
    KeLnsClm_SetShp @ 0x0076_9cd0 => extern "C" fn ke_lns_clm_set_shp(o: u32, shp: u32, seed: u32) {
        ke::bridge::ke_lns_clm_set_shp(&mut mem(), o, shp, seed);
    }
    /// `KeDttMdl_UseSingleInstance()`.
    KeDttMdl_UseSingleInstance @ 0x0072_e6d0 => extern "C" fn ke_dtt_mdl_use_single_instance() -> u32 {
        ke::bridge::ke_dtt_mdl_use_single_instance(&mem())
    }
    /// `AbmapAllLinksLimitFlag(mask)`.
    AbmapAllLinksLimitFlag @ 0x00a4_73d0 => extern "C" fn abmap_all_links_limit_flag(mask: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        abmap::bridge::abmap_all_links_limit_flag(&mut mem(), mask as u8)
    }
    /// `abmap_lost_panel(bit, node)`.
    abmap_lost_panel @ 0x00a4_58c0 => extern "C" fn abmap_lost_panel(bit: u32, node: u32) -> u32 {
        abmap::bridge::abmap_lost_panel(&mut mem(), bit, node)
    }
    /// `AbmapGetNodePosRadius(node, out)`.
    AbmapGetNodePosRadius @ 0x00a5_b4c0 => extern "C" fn abmap_get_node_pos_radius(node: u32, out: u32) -> f32 {
        #[allow(clippy::cast_possible_truncation)] // the low half
        abmap::bridge::abmap_get_node_pos_radius(&mut mem(), node as u16, out)
    }
    /// `rcbgPSimGetPos(psim, which, out)`.
    rcbgPSimGetPos @ 0x0091_fdd0 => extern "C" fn rcbg_psim_get_pos(p: u32, which: u32, out: u32) -> u32 {
        model::bridge::rcbg_psim_get_pos(&mut mem(), p, which, out)
    }
    /// `rcbgSceneSetPal(scene)`.
    rcbgSceneSetPal @ 0x0092_10a0 => extern "C" fn rcbg_scene_set_pal(s: u32) {
        model::bridge::rcbg_scene_set_pal(&mut mem(), s);
    }
    /// `rcbgChkId(id)`.
    rcbgChkId @ 0x0092_1940 => extern "C" fn rcbg_chk_id(id: u32) -> i32 {
        model::bridge::rcbg_chk_id(&mem(), id)
    }
    /// `rcbgFPm097Init(part)`.
    rcbgFPm097Init @ 0x0092_2560 => extern "C" fn rcbg_fpm097_init(p: u32) -> u32 {
        model::bridge::rcbg_fpm097_init(&mut mem(), p)
    }
    /// `rcbgFPm173Init(part)`.
    rcbgFPm173Init @ 0x0092_2b70 => extern "C" fn rcbg_fpm173_init(p: u32) -> u32 {
        model::bridge::rcbg_fpm173_init(&mut mem(), p)
    }
    /// `rcbgFPm565Init(part)`.
    rcbgFPm565Init @ 0x0092_2f50 => extern "C" fn rcbg_fpm565_init(p: u32) -> u32 {
        model::bridge::rcbg_fpm565_init(&mut mem(), p)
    }
    /// `rcfxDampInit(damp)`.
    rcfxDampInit @ 0x0092_93d0 => extern "C" fn rcfx_damp_init(p: u32) -> u32 {
        model::bridge::rcfx_damp_init(&mut mem(), p)
    }
    /// `rcbgFpGetWork(id, part, index)`.
    rcbgFpGetWork @ 0x0092_3100 => extern "C" fn rcbg_fp_get_work(id: u32, part: u32, index: u32) -> u32 {
        model::bridge::rcbg_fp_get_work(&mut mem(), id, part, index, fp_run)
    }
    /// `rcbgSceneDrawLayer(layer)`.
    rcbgSceneDrawLayer @ 0x0092_0830 => extern "C" fn rcbg_scene_draw_layer(layer: u32) -> u32 {
        model::bridge::rcbg_scene_draw_layer(&mut mem(), layer, |m, f, l| {
            ffx_vfx::op::bridge::op_func_after_ot3d_set(m, f, l, 0, 1).cast_unsigned()
        })
    }
    /// `rcbgSceneWaveDCStop(scene)`.
    rcbgSceneWaveDCStop @ 0x0092_0650 => extern "C" fn rcbg_scene_wave_dc_stop(s: u32) {
        model::bridge::rcbg_scene_wave_dc_stop(&mut mem(), s);
    }
    /// `rcbgSceneWaveC0Stop(scene)`.
    rcbgSceneWaveC0Stop @ 0x0092_1200 => extern "C" fn rcbg_scene_wave_c0_stop(s: u32) {
        model::bridge::rcbg_scene_wave_c0_stop(&mut mem(), s);
    }
    /// `rcfxParticleClearForces(p)`.
    rcfxParticleClearForces @ 0x0092_89b0 => extern "C" fn rcfx_particle_clear_forces(p: u32) {
        model::bridge::rcfx_particle_clear_forces(&mut mem(), p);
    }
    /// `rcdtTm2GetPsm(picture)`.
    rcdtTm2GetPsm @ 0x0092_3280 => extern "C" fn rcdt_tm2_get_psm(pic: u32) -> u32 {
        model::bridge::rcdt_tm2_get_psm(&mem(), pic)
    }
    /// `rcdtTm2ClutAlphaToPs2(clut)`.
    rcdtTm2ClutAlphaToPs2 @ 0x0092_3240 => extern "C" fn rcdt_tm2_clut_alpha_to_ps2(clut: u32) {
        model::bridge::rcdt_tm2_clut_alpha_to_ps2(&mut mem(), clut);
    }
    /// `rcdtRsd2Init(rsd)`.
    rcdtRsd2Init @ 0x0092_4390 => extern "C" fn rcdt_rsd2_init(p: u32) {
        model::bridge::rcdt_rsd2_init(&mut mem(), p);
    }
    /// `rcbgTrxSetRegs(t, a, b, c, d, e, f, g)`, the `i16`s widened.
    rcbgTrxSetRegs @ 0x0092_d360 => extern "C" fn rcbg_trx_set_regs(t: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low halves
        let h = |v: u32| (v as u16).cast_signed();
        model::bridge::rcbg_trx_set_regs(&mut mem(), t, h(a), h(b), h(c), h(d), h(e), h(f), h(g))
    }
    /// `rcbgClipCodeAnd(out, a, b, c, d)`.
    rcbgClipCodeAnd @ 0x0092_aed0 => extern "C" fn rcbg_clip_code_and(out: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        model::bridge::rcbg_clip_code_and(&mut mem(), out, a, b, c, d)
    }
    /// `rcefTaskProc(task)`.
    rcefTaskProc @ 0x0092_ed90 => extern "C" fn rcef_task_proc(t: u32) -> u32 {
        task::bridge::rcef_task_proc(&mut mem(), t, |_, f, t| call_ptr1(f, t))
    }
    /// `rcefTaskDestruct(task)`.
    rcefTaskDestruct @ 0x0092_ec90 => extern "C" fn rcef_task_destruct(t: u32) -> u32 {
        task::bridge::rcef_task_destruct(&mut mem(), t, |_, f, t| call_ptr1(f, t))
    }
    /// `rcEffectTaskPrayWait(task)`.
    rcEffectTaskPrayWait @ 0x0093_04b0 => extern "C" fn rc_effect_task_pray_wait(t: u32) {
        task::bridge::rc_effect_task_pray_wait(&mut mem(), t, |m, _| ffx_vfx::op::bridge::op_pray_effect_check(m));
    }
    /// `rcefObjInit(obj)`.
    rcefObjInit @ 0x0092_ff20 => extern "C" fn rcef_obj_init(o: u32) -> u32 {
        task::bridge::rcef_obj_init(&mut mem(), o)
    }
    /// `rcefParCreate(par, data, index, a, b, c)`.
    rcefParCreate @ 0x0092_eee0 => extern "C" fn rcef_par_create(p: u32, data: u32, index: u32, a: u32, b: u32, c: u32) -> u32 {
        task::bridge::rcef_par_create(&mut mem(), p, data, index, a, b, c)
    }
    /// `MapPart::InitLangOverrides()` (`this` in ECX).
    MapPart_InitLangOverrides @ 0x0067_c620 => extern "fastcall" fn map_part_init_lang_overrides(this: u32) -> u32 {
        scene::bridge::map_part_init_lang_overrides(&mut mem(), this)
    }
    /// `MapPart::LangOverride(lang, map)`.
    MapPart_LangOverride @ 0x0067_fb30 => extern "thiscall" fn map_part_lang_override(this: u32, lang: u32, map: u32) -> u32 {
        scene::bridge::map_part_lang_override(&mut mem(), this, lang, map, |_| {
            // Phyre::PhyrePrintf(2, "\n Warning: uncorrect LanguageType \n")
            // SAFETY: a variadic cdecl function of this build and its string.
            unsafe {
                let f: unsafe extern "C" fn(u32, u32) -> u32 =
                    std::mem::transmute(crate::rebase(0x0043_52f0));
                #[allow(clippy::cast_possible_truncation)] // a 32-bit process
                f(2, crate::rebase(0x00b4_5b7c) as u32);
            }
        })
    }
    /// `TextUtilList::TextUtilList()`.
    TextUtilList_TextUtilList @ 0x0090_e6d0 => extern "C" fn text_util_list_text_util_list(this: u32) -> u32 {
        rcui::bridge::text_util_list_text_util_list(&mut mem(), this)
    }
    /// `rcMat4fSetScaleVec3f(m, v)`.
    rcMat4fSetScaleVec3f @ 0x0092_5820 => extern "C" fn rc_mat4f_set_scale_vec3f(m: u32, v: u32) -> u32 {
        geom::bridge::rc_mat4f_set_scale_vec3f(&mut mem(), m, v)
    }
    /// `rcbgModelSetScale(model, s)`.
    rcbgModelSetScale @ 0x0092_8540 => extern "C" fn rcbg_model_set_scale(model: u32, s: u32) -> u32 {
        geom::bridge::rcbg_model_set_scale(&mut mem(), model, f32::from_bits(s))
    }
    /// `yiAttachChrDObj(id, a, b)`.
    yiAttachChrDObj @ 0x0090_8350 => extern "C" fn yi_attach_chr_dobj(id: u32, a: u32, b: u32) -> u32 {
        yi::bridge::yi_attach_chr_dobj(&mut mem(), id, a, b)
    }
    /// `yiCallFieldParticle()`.
    yiCallFieldParticle @ 0x0090_83e0 => extern "C" fn yi_call_field_particle() {
        yi::bridge::yi_call_field_particle(&mut mem(), |_| {
            super::call0(0x0072_9ab0); // pppFpLoop
        });
    }
    /// `rcfxParticleResetForce(p)`.
    rcfxParticleResetForce @ 0x0092_8970 => extern "C" fn rcfx_particle_reset_force(p: u32) {
        model::bridge::rcfx_particle_reset_force(&mut mem(), p, |_, f, a, b| {
            call_ptr2(f, a, b);
        });
    }
    /// `rcbgSceneDraw(scene)`.
    rcbgSceneDraw @ 0x0092_0740 => extern "C" fn rcbg_scene_draw(s: u32) -> u32 {
        model::bridge::rcbg_scene_draw(
            &mut mem(),
            s,
            |_, f, o| {
                call_ptr1(f, o);
            },
            |m, f, l| ffx_vfx::op::bridge::op_func_after_ot3d_set(m, f, l, 0, 1).cast_unsigned(),
        )
    }
    /// `Yn_PSimGetHandle(n)`.
    Yn_PSimGetHandle @ 0x0091_99f0 => extern "C" fn yn_psim_get_handle(n: u32) -> u32 {
        model::bridge::yn_psim_get_handle(&mut mem(), n, fp_run)
    }
    /// `rcefMainSetPar(main, obj, data_index, index)`.
    rcefMainSetPar @ 0x0092_56d0 => extern "C" fn rcef_main_set_par(main: u32, o: u32, data_index: u32, index: u32) -> u32 {
        task::bridge::rcef_main_set_par(&mut mem(), main, o, data_index, index)
    }
    /// `Yn_PSimGetHandle_00919f00(n)`: a jump to `Yn_PSimGetHandle`.
    Yn_PSimGetHandle_00919f00 @ 0x0091_9f00 => extern "C" fn yn_psim_get_handle_jump(n: u32) -> u32 {
        model::bridge::yn_psim_get_handle(&mut mem(), n, fp_run)
    }
    /// `rcbgPSimSetReady(psim)`.
    rcbgPSimSetReady @ 0x0091_ffb0 => extern "C" fn rcbg_psim_set_ready(p: u32) {
        model::bridge::rcbg_psim_set_ready(&mut mem(), p, |_, f, a, b| {
            call_ptr2(f, a, b);
        });
    }
    /// `Yn_MapPartAnimeGetCount(part)`.
    Yn_MapPartAnimeGetCount @ 0x0091_a2a0 => extern "C" fn yn_map_part_anime_get_count(part: i32) -> u32 {
        guide::bridge::yn_map_part_anime_get_count(&mem(), part)
    }
    /// `Yn_MapPartGetAlpha(part)`.
    Yn_MapPartGetAlpha @ 0x0091_a2e0 => extern "C" fn yn_map_part_get_alpha(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_alpha(&mem(), part)
    }
    /// `Yn_MapPartGetPosX(part)`.
    Yn_MapPartGetPosX @ 0x0091_a330 => extern "C" fn yn_map_part_get_pos_x(part: i32) -> f32 {
        guide::bridge::yn_map_part_get_pos_x(&mem(), part)
    }
    /// `Yn_MapPartGetPosY(part)`.
    Yn_MapPartGetPosY @ 0x0091_a380 => extern "C" fn yn_map_part_get_pos_y(part: i32) -> f32 {
        guide::bridge::yn_map_part_get_pos_y(&mem(), part)
    }
    /// `Yn_MapPartGetHeight(part)`.
    Yn_MapPartGetHeight @ 0x0091_a3f0 => extern "C" fn yn_map_part_get_height(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_height(&mem(), part)
    }
    /// `Yn_MapPartGet0C(part)`.
    Yn_MapPartGet0C @ 0x0091_a440 => extern "C" fn yn_map_part_get_0c(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_0c(&mem(), part)
    }
    /// `Yn_MapPartGet10(part)`.
    Yn_MapPartGet10 @ 0x0091_a490 => extern "C" fn yn_map_part_get_10(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_10(&mem(), part)
    }
    /// `Yn_MapPartGetScaleY(part)`.
    Yn_MapPartGetScaleY @ 0x0091_a4e0 => extern "C" fn yn_map_part_get_scale_y(part: i32) -> f32 {
        guide::bridge::yn_map_part_get_scale_y(&mem(), part)
    }
    /// `Yn_MapPartGetScaleX(part)`.
    Yn_MapPartGetScaleX @ 0x0091_a530 => extern "C" fn yn_map_part_get_scale_x(part: i32) -> f32 {
        guide::bridge::yn_map_part_get_scale_x(&mem(), part)
    }
    /// `Yn_MapPartGet24(part)`.
    Yn_MapPartGet24 @ 0x0091_a580 => extern "C" fn yn_map_part_get_24(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_24(&mem(), part)
    }
    /// `Yn_MapPartGet28(part)`.
    Yn_MapPartGet28 @ 0x0091_a5d0 => extern "C" fn yn_map_part_get_28(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_28(&mem(), part)
    }
    /// `Yn_MapPartGetWidth(part)`.
    Yn_MapPartGetWidth @ 0x0091_a620 => extern "C" fn yn_map_part_get_width(part: i32) -> u32 {
        guide::bridge::yn_map_part_get_width(&mem(), part)
    }
    /// `Yn_MapPartGet14(part)`.
    Yn_MapPartGet14 @ 0x0091_a670 => extern "C" fn yn_map_part_get_14(part: i32) -> f32 {
        guide::bridge::yn_map_part_get_14(&mem(), part)
    }
    /// `Yn_MapPartAnimeSetMode(part, bits)`.
    Yn_MapPartAnimeSetMode @ 0x0091_9fe0 => extern "C" fn yn_map_part_anime_set_mode(part: i32, bits: u32) {
        guide::bridge::yn_map_part_anime_set_mode(&mut mem(), part, bits);
    }
    /// `Yn_MapPartAnimeFlag0Off(part)`.
    Yn_MapPartAnimeFlag0Off @ 0x0091_a030 => extern "C" fn yn_map_part_anime_flag0_off(part: i32) {
        guide::bridge::yn_map_part_anime_flag0_off(&mut mem(), part);
    }
    /// `Yn_MapPartAnimeFlag0On(part)`.
    Yn_MapPartAnimeFlag0On @ 0x0091_a080 => extern "C" fn yn_map_part_anime_flag0_on(part: i32) {
        guide::bridge::yn_map_part_anime_flag0_on(&mut mem(), part);
    }
    /// `Yn_MapPartMoveTo(part, x, y, frames)`.
    Yn_MapPartMoveTo @ 0x0091_a0f0 => extern "C" fn yn_map_part_move_to(part: i32, x: u32, y: u32, frames: u32) {
        guide::bridge::yn_map_part_move_to(&mut mem(), part, x, y, frames);
    }
    /// `Yn_MapPartScaleTo(part, sx, sy, frames)`.
    Yn_MapPartScaleTo @ 0x0091_a150 => extern "C" fn yn_map_part_scale_to(part: i32, sx: u32, sy: u32, frames: u32) {
        guide::bridge::yn_map_part_scale_to(&mut mem(), part, sx, sy, frames);
    }
    /// `Yn_MapPartAnimeSetLastFrame(part)`.
    Yn_MapPartAnimeSetLastFrame @ 0x0091_a970 => extern "C" fn yn_map_part_anime_set_last_frame(part: i32) {
        guide::bridge::yn_map_part_anime_set_last_frame(&mut mem(), part);
    }
    /// `Yn_MapPartAnimeSetRate(part, rate)`.
    Yn_MapPartAnimeSetRate @ 0x0091_a9c0 => extern "C" fn yn_map_part_anime_set_rate(part: i32, rate: u32) {
        guide::bridge::yn_map_part_anime_set_rate(&mut mem(), part, rate);
    }
    /// `Yn_MapPartSetPos(part, x, y)`.
    Yn_MapPartSetPos @ 0x0091_aae0 => extern "C" fn yn_map_part_set_pos(part: i32, x: u32, y: u32) {
        guide::bridge::yn_map_part_set_pos(&mut mem(), part, x, y);
    }
    /// `Yn_MapPartSet24(part, a, b)`.
    Yn_MapPartSet24 @ 0x0091_ac40 => extern "C" fn yn_map_part_set_24(part: i32, a: u32, b: u32) {
        guide::bridge::yn_map_part_set_24(&mut mem(), part, a, b);
    }
    /// `Yn_MapPartSetSize(part, w, h)`.
    Yn_MapPartSetSize @ 0x0091_ac80 => extern "C" fn yn_map_part_set_size(part: i32, w: u32, h: u32) {
        guide::bridge::yn_map_part_set_size(&mut mem(), part, w, h);
    }
    /// `Yn_MapPartSet14(part, v)`.
    Yn_MapPartSet14 @ 0x0091_acc0 => extern "C" fn yn_map_part_set_14(part: i32, v: u32) {
        guide::bridge::yn_map_part_set_14(&mut mem(), part, v);
    }
    /// `Yn_MapPartAnimeSetFrame(part, frame)`.
    Yn_MapPartAnimeSetFrame @ 0x0091_a930 => extern "C" fn yn_map_part_anime_set_frame(part: i32, frame: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low half
        guide::bridge::yn_map_part_anime_set_frame(&mut mem(), part, frame as u16);
    }
    /// `Yn_SeaGetWork()`.
    Yn_SeaGetWork @ 0x0091_9a10 => extern "C" fn yn_sea_get_work() -> u32 {
        model::bridge::yn_sea_get_work(&mut mem(), fp_run)
    }
    /// `AbmapInitHoming(node, t)`.
    AbmapInitHoming @ 0x00a4_8d70 => extern "C" fn abmap_init_homing(node: u32, t: u32) -> u32 {
        abmap::bridge::abmap_init_homing(&mut mem(), node, t)
    }
    /// `AbmapInitMovingToTarget(target, node)`.
    AbmapInitMovingToTarget @ 0x00a4_8c80 => extern "C" fn abmap_init_moving_to_target(target: u32, node: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low half
        abmap::bridge::abmap_init_moving_to_target(&mut mem(), target, node as u16)
    }
    /// `AbmapCalcMoveCost(node, cost)`.
    AbmapCalcMoveCost @ 0x00a4_7560 => extern "C" fn abmap_calc_move_cost(node: u32, cost: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low half
        abmap::bridge::abmap_calc_move_cost(&mut mem(), node as u16, cost)
    }
    /// `AbmapCtrlTilt()`.
    AbmapCtrlTilt @ 0x00a4_8f50 => extern "C" fn abmap_ctrl_tilt() -> u32 {
        abmap::bridge::abmap_ctrl_tilt(&mut mem())
    }
    /// `AbmapState_HomeOnNode()`.
    AbmapState_HomeOnNode @ 0x00a5_9e80 => extern "C" fn abmap_state_home_on_node() -> u32 {
        abmap::bridge::abmap_state_home_on_node(&mut mem())
    }
    /// `LoadSaveAbmap()`.
    LoadSaveAbmap @ 0x00a4_9590 => extern "C" fn load_save_abmap() -> u32 {
        abmap::bridge::load_save_abmap(&mut mem())
    }
    /// `abmap_test_panel(bit, node)`.
    abmap_test_panel @ 0x00a4_57d0 => extern "C" fn abmap_test_panel(bit: u32, node: u32) -> u32 {
        abmap::bridge::abmap_test_panel(&mem(), bit, node)
    }
    /// `abmap_other_has_panel(chr, node)`.
    abmap_other_has_panel @ 0x00a4_5800 => extern "C" fn abmap_other_has_panel(chr: u32, node: u32) -> u32 {
        abmap::bridge::abmap_other_has_panel(&mem(), chr, node)
    }
    /// `abmap_chr_node(chr)`.
    abmap_chr_node @ 0x00a4_5870 => extern "C" fn abmap_chr_node(chr: u32) -> u32 {
        abmap::bridge::abmap_chr_node(&mem(), chr)
    }
    /// `rcbgPSimSetPos(psim, which, v)`.
    rcbgPSimSetPos @ 0x0091_ff10 => extern "C" fn rcbg_psim_set_pos(p: u32, which: u32, v: u32) {
        model::bridge::rcbg_psim_set_pos(&mut mem(), p, which, v, |_, f, a, b| {
            call_ptr2(f, a, b);
        });
    }
    /// `KeThBP_Quit(bp)`.
    KeThBP_Quit @ 0x0076_36c0 => extern "C" fn ke_th_bp_quit(bp: u32) {
        ke::bridge::ke_th_bp_quit(&mut mem(), bp);
    }
    /// `SqsBattleStreamingDataSend(a, b)`.
    SqsBattleStreamingDataSend @ 0x0081_cef0 => extern "C" fn sqs_battle_streaming_data_send(a: u32, b: u32) {
        nkz::bridge::sqs_battle_streaming_data_send(&mut mem(), a, b, |_| {
            // fprintf(stderr, "FMOD SFX ERROR! ...") through the CRT's imports.
            // SAFETY: the import slots hold MSVCR110's __iob_func and fprintf.
            unsafe {
                let iob: unsafe extern "C" fn() -> usize =
                    std::mem::transmute(*(crate::rebase(0x00b0_c350) as *const usize));
                let fprintf: unsafe extern "C" fn(usize, usize) -> i32 =
                    std::mem::transmute(*(crate::rebase(0x00b0_c38c) as *const usize));
                fprintf(iob() + 0x40, crate::rebase(0x00b5_8f40));
            }
        });
    }
    /// `yiClutAlloc_makepacket(packet, cluts, n)`.
    yiClutAlloc_makepacket @ 0x0091_7050 => extern "C" fn yi_clut_alloc_makepacket(pk: u32, cluts: u32, n: u32) -> u32 {
        yi::bridge::yi_clut_alloc_makepacket(&mut mem(), pk, cluts, n)
    }
    /// `yiRender_setupparamGS(render)`.
    yiRender_setupparamGS @ 0x0090_d260 => extern "C" fn yi_render_setupparam_gs(r: u32) {
        yi::bridge::yi_render_setupparam_gs(&mut mem(), r);
    }
    /// `KeVu0RndI()`.
    KeVu0RndI @ 0x0075_f920 => extern "C" fn ke_vu0_rnd_i() -> u32 {
        vu0::bridge::ke_vu0_rnd_i(&mut mem(), |_, seed| crt_srand(seed))
    }
    /// `NkzSetLuluLimitTbl(group)`.
    NkzSetLuluLimitTbl @ 0x00a5_cc70 => extern "C" fn nkz_set_lulu_limit_tbl(group: u32) {
        nkz::bridge::nkz_set_lulu_limit_tbl(&mut mem(), group);
    }
    /// `rcbgPalGetEntryData(pal, n)`.
    rcbgPalGetEntryData @ 0x0092_9990 => extern "C" fn rcbg_pal_get_entry_data(pal: u32, n: u32) -> u32 {
        model::bridge::rcbg_pal_get_entry_data(&mem(), pal, n)
    }
    /// `rcbgPalGetLastEntryData(pal)`.
    rcbgPalGetLastEntryData @ 0x0092_9930 => extern "C" fn rcbg_pal_get_last_entry_data(pal: u32) -> u32 {
        model::bridge::rcbg_pal_get_last_entry_data(&mem(), pal)
    }
    /// `rcbgLayerGetTexData(layer)`.
    rcbgLayerGetTexData @ 0x0092_acf0 => extern "C" fn rcbg_layer_get_tex_data(layer: u32) -> u32 {
        model::bridge::rcbg_layer_get_tex_data(&mem(), layer)
    }
    /// `rcbgTexGetLog2Size(w, h, lw, lh, tbw)`.
    rcbgTexGetLog2Size @ 0x0092_0050 => extern "C" fn rcbg_tex_get_log2_size(w: i32, h: i32, lw: u32, lh: u32, tbw: u32) {
        model::bridge::rcbg_tex_get_log2_size(&mut mem(), w, h, lw, lh, tbw);
    }
    /// `Yn_SceneGetObject(kind, n)`.
    Yn_SceneGetObject @ 0x0092_1be0 => extern "C" fn yn_scene_get_object(kind: u32, n: u32) -> u32 {
        model::bridge::yn_scene_get_object(&mem(), kind, n)
    }
    /// `Yn_SceneCountObjects(kind)`.
    Yn_SceneCountObjects @ 0x0092_1c20 => extern "C" fn yn_scene_count_objects(kind: u32) -> u32 {
        model::bridge::yn_scene_count_objects(&mem(), kind)
    }
    /// `Yn_BattleCaptureNum()`.
    Yn_BattleCaptureNum @ 0x0092_1910 => extern "C" fn yn_battle_capture_num() {
        model::bridge::yn_battle_capture_num(&mut mem());
    }
    /// `rcbgFpFadeInit(fade, id, step, frames)`.
    rcbgFpFadeInit @ 0x0092_2930 => extern "C" fn rcbg_fp_fade_init(fade: u32, id: u32, step: u32, frames: u32) {
        model::bridge::rcbg_fp_fade_init(&mut mem(), fade, id, step, frames, |m, id| {
            ffx_vfx::fp::bridge::ppp_fp_id_disp_off(m, id);
        });
    }
    /// `rcbgChrFindById(id)`.
    rcbgChrFindById @ 0x0092_e1e0 => extern "C" fn rcbg_chr_find_by_id(id: u32) -> u32 {
        model::bridge::rcbg_chr_find_by_id(&mut mem(), id)
    }
    /// `rcbgChrHandleIndex(handle)`.
    rcbgChrHandleIndex @ 0x0092_4f30 => extern "C" fn rcbg_chr_handle_index(handle: u32) -> u32 {
        model::bridge::rcbg_chr_handle_index(&mem(), handle)
    }
    /// `rcdtChunkHandleGetEnd(&chunk)`.
    rcdtChunkHandleGetEnd @ 0x0092_bb90 => extern "C" fn rcdt_chunk_handle_get_end(h: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_handle_get_end(&mem(), h)
    }
    /// `rcMat4fSetScale001(m)`.
    rcMat4fSetScale001 @ 0x0092_de50 => extern "C" fn rc_mat4f_set_scale_001(m: u32) -> u32 {
        geom::bridge::rc_mat4f_set_scale_001(&mut mem(), m)
    }
    /// `rcMat4fSetScale0001(m)`.
    rcMat4fSetScale0001 @ 0x0092_de80 => extern "C" fn rc_mat4f_set_scale_0001(m: u32) -> u32 {
        geom::bridge::rc_mat4f_set_scale_0001(&mut mem(), m)
    }
    /// `rcuiCollectChrHandles()`.
    rcuiCollectChrHandles @ 0x0093_2570 => extern "C" fn rcui_collect_chr_handles() {
        rcui::bridge::rcui_collect_chr_handles(&mut mem());
    }
    /// `KePmcomAlloc(bytes)`.
    KePmcomAlloc @ 0x0076_2f50 => extern "C" fn ke_pmcom_alloc(bytes: u32) -> u32 {
        ke::bridge::ke_pmcom_alloc(&mut mem(), bytes)
    }
    /// `KePmcomQueuePush(kind, p)`.
    KePmcomQueuePush @ 0x0076_2fa0 => extern "C" fn ke_pmcom_queue_push(kind: u32, p: u32) {
        ke::bridge::ke_pmcom_queue_push(&mut mem(), kind, p);
    }
    /// `KeLnsDefault_Init(obj)`.
    KeLnsDefault_Init @ 0x0076_a640 => extern "C" fn ke_lns_default_init(o: u32) -> u32 {
        ke::bridge::ke_lns_default_init(&mut mem(), o)
    }
    /// `KeVu0MulX(dst, src, s)`.
    KeVu0MulX @ 0x0076_76c0 => extern "C" fn ke_vu0_mul_x(dst: u32, src: u32, s: u32) {
        ke::bridge::ke_vu0_mul_x(&mut mem(), dst, src, f32::from_bits(s));
    }
    /// `KeVu0SubX(dst, src, s)`.
    KeVu0SubX @ 0x0076_7720 => extern "C" fn ke_vu0_sub_x(dst: u32, src: u32, s: u32) {
        ke::bridge::ke_vu0_sub_x(&mut mem(), dst, src, f32::from_bits(s));
    }
    /// `NkzGetMonsterListNum()`.
    NkzGetMonsterListNum @ 0x00a5_cb20 => extern "C" fn nkz_get_monster_list_num() -> u32 {
        nkz::bridge::nkz_get_monster_list_num()
    }
    // ---- round 6 ----
    /// `rcbgTrxLoad(trx, parse)`.
    rcbgTrxLoad @ 0x0092_b710 => extern "C" fn rcbg_trx_load(t: u32, p: u32) {
        model::bridge::rcbg_trx_load(&mut mem(), t, p);
    }
    /// `Yn_SeaSetAng(v)`.
    Yn_SeaSetAng @ 0x0091_ae40 => extern "C" fn yn_sea_set_ang(v: u32) -> u32 {
        model::bridge::yn_sea_set_ang(&mut mem(), v, fp_run)
    }
    /// `Yn_SeaSetPos(v)`.
    Yn_SeaSetPos @ 0x0091_ae80 => extern "C" fn yn_sea_set_pos(v: u32) -> u32 {
        model::bridge::yn_sea_set_pos(&mut mem(), v, fp_run)
    }
    /// `Yn_SeaSetUvSpd(u, v)`.
    Yn_SeaSetUvSpd @ 0x0091_aea0 => extern "C" fn yn_sea_set_uv_spd(u: u32, v: u32) {
        model::bridge::yn_sea_set_uv_spd(&mut mem(), u, v, fp_run);
    }
    /// `Yn_PSimSetPos(psim, which, v)`.
    Yn_PSimSetPos @ 0x0091_9f10 => extern "C" fn yn_psim_set_pos(p: u32, which: i32, v: u32) {
        model::bridge::yn_psim_set_pos(&mut mem(), p, which, v, |_, f, a, b| {
            call_ptr2(f, a, b);
        });
    }
    /// `SqsShoutStreamingPlayState()`.
    SqsShoutStreamingPlayState @ 0x0081_edf0 => extern "C" fn sqs_shout_streaming_play_state() -> u32 {
        nkz::bridge::sqs_shout_streaming_play_state(&mut mem())
    }
    /// `SqsSifDmaStart(_, src, dst, size)`.
    SqsSifDmaStart @ 0x0081_eeb0 => extern "C" fn sqs_sif_dma_start(_a: u32, src: u32, dst: u32, size: u32) -> u32 {
        nkz::bridge::sqs_sif_dma_start(&mut mem(), src, dst, size)
    }
    /// `SqsShoutStreamingDataSend(src, size)`.
    SqsShoutStreamingDataSend @ 0x0081_ed40 => extern "C" fn sqs_shout_streaming_data_send(src: u32, size: i32) {
        nkz::bridge::sqs_shout_streaming_data_send(&mut mem(), src, size);
    }
    /// `AbmapTransitionToHoming(node, t)`.
    AbmapTransitionToHoming @ 0x00a4_8d20 => extern "C" fn abmap_transition_to_homing(node: u32, t: u32) {
        abmap::bridge::abmap_transition_to_homing(&mut mem(), node, t);
    }
    /// `yiArcPrintSize()`.
    yiArcPrintSize @ 0x0090_e6f0 => extern "C" fn yi_arc_print_size() -> u32 {
        yi::bridge::yi_arc_print_size()
    }
    /// `KeBornRnd2(obj, arg, born, &index, shape, random, n)`.
    KeBornRnd2 @ 0x0076_9230 => extern "C" fn ke_born_rnd2(o: u32, arg: u32, born: u32, index: u32, shape: u32, random: u32, n: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low parts
        ke::bridge::ke_born_rnd2(
            &mut mem(),
            o,
            arg,
            born,
            index,
            (shape as u16).cast_signed(),
            random as u8,
            n as u8,
            |m| vu0::bridge::ke_vu0_rnd_i(m, |_, seed| crt_srand(seed)),
            |_, f, a, b, at| {
                let mut more = 0u32;
                #[allow(clippy::cast_possible_truncation)] // a 32-bit process
                let p = (&raw mut more).expose_provenance() as u32;
                // SAFETY: the born callback is a cdecl function of (obj, arg, vertex, &more).
                unsafe {
                    let f: unsafe extern "C" fn(u32, u32, u32, u32) -> u32 = std::mem::transmute(f as usize);
                    f(a, b, at, p);
                }
                more
            },
        )
    }
    /// `KeDttSpr_MkGifPk_Pre(spr)`.
    KeDttSpr_MkGifPk_Pre @ 0x0076_8d30 => extern "C" fn ke_dtt_spr_mk_gif_pk_pre(o: u32) {
        ke::bridge::ke_dtt_spr_mk_gif_pk_pre(
            &mut mem(),
            o,
            |m| ffx_vfx::op::bridge::op_spr_alloc(m, 0x90),
            ffx_vfx::op::bridge::op_spr_free,
        );
    }
    /// `rcdtParseIsEnd(parse)`.
    rcdtParseIsEnd @ 0x0092_bfb0 => extern "C" fn rcdt_parse_is_end(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_is_end(&mem(), p)
    }
    /// `rcdtParseSkipSeparators(parse)`.
    rcdtParseSkipSeparators @ 0x0092_bd60 => extern "C" fn rcdt_parse_skip_separators(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_skip_separators(&mem(), p)
    }
    /// `rcdtParseSkipToken(parse)`.
    rcdtParseSkipToken @ 0x0092_bd20 => extern "C" fn rcdt_parse_skip_token(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_skip_token(&mem(), p)
    }
    /// `rcdtParseFindChar(parse, c)`.
    rcdtParseFindChar @ 0x0092_bae0 => extern "C" fn rcdt_parse_find_char(p: u32, c: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        rcdt::bridge::rcdt_parse_find_char(&mem(), p, (c as u8).cast_signed())
    }
    /// `rcuiVecToInt(out, v)`.
    rcuiVecToInt @ 0x0093_cff0 => extern "C" fn rcui_vec_to_int(out: u32, v: u32) -> u32 {
        rcui::bridge::rcui_vec_to_int(&mut mem(), out, v)
    }
    /// `rcuiApplyMatrix(out, m, v)`.
    rcuiApplyMatrix @ 0x0093_cbf0 => extern "C" fn rcui_apply_matrix(out: u32, m: u32, v: u32) {
        rcui::bridge::rcui_apply_matrix(&mut mem(), out, m, v);
    }
    /// `rcuiMulMatrix(out, b, a)`.
    rcuiMulMatrix @ 0x0093_d1b0 => extern "C" fn rcui_mul_matrix(out: u32, b: u32, a: u32) {
        rcui::bridge::rcui_mul_matrix(&mut mem(), out, b, a);
    }
    /// `rcuiNormalize(out, v)`.
    rcuiNormalize @ 0x0093_d3d0 => extern "C" fn rcui_normalize(out: u32, v: u32) {
        rcui::bridge::rcui_normalize(&mut mem(), out, v);
    }
    /// `rcuiMouseMoveBegin(id, arg)`.
    rcuiMouseMoveBegin @ 0x0093_c150 => extern "C" fn rcui_mouse_move_begin(id: u32, arg: u32) {
        use ffx_menu::gu::bridge as gu;
        rcui::bridge::rcui_mouse_move_begin(
            &mut mem(),
            id,
            arg,
            gu::gu_mouse_set_capture,
            |m, _| gu::MOUSE_X.get(m).cast_unsigned(),
            |m, _| gu::MOUSE_Y.get(m).cast_unsigned(),
        );
    }
    /// `Yn_SeaGetAngPtr()`.
    Yn_SeaGetAngPtr @ 0x0091_adf0 => extern "C" fn yn_sea_get_ang_ptr() -> u32 {
        model::bridge::yn_sea_get_ang_ptr(&mut mem(), fp_run)
    }
    /// `Yn_SeaGetPosPtr()`.
    Yn_SeaGetPosPtr @ 0x0091_ae10 => extern "C" fn yn_sea_get_pos_ptr() -> u32 {
        model::bridge::yn_sea_get_pos_ptr(&mut mem(), fp_run)
    }
    /// `rcbgPalCopyToLast(pal)`.
    rcbgPalCopyToLast @ 0x0092_99d0 => extern "C" fn rcbg_pal_copy_to_last(pal: u32) {
        model::bridge::rcbg_pal_copy_to_last(&mut mem(), pal);
    }
    /// `rcdtTm2CopyRect(tm2, dst, x, y, bytes, rows)`.
    rcdtTm2CopyRect @ 0x0092_3ae0 => extern "C" fn rcdt_tm2_copy_rect(tm2: u32, dst: u32, x: u32, y: u32, bytes: u32, rows: i32) {
        model::bridge::rcdt_tm2_copy_rect(&mut mem(), tm2, dst, x, y, bytes, rows);
    }
    /// `rcVec4fToFixed(out, v)`.
    rcVec4fToFixed @ 0x0092_5950 => extern "C" fn rc_vec4f_to_fixed(out: u32, v: u32) -> u32 {
        model::bridge::rc_vec4f_to_fixed(&mut mem(), out, v)
    }
    /// `Yn_DepthToScreen(d)`.
    Yn_DepthToScreen @ 0x0092_01a0 => extern "C" fn yn_depth_to_screen(d: u32) -> u32 {
        guide::bridge::yn_depth_to_screen(&mem(), f32::from_bits(d))
    }
    /// `yiSetViewMatrix(m)`.
    yiSetViewMatrix @ 0x0090_a980 => extern "C" fn yi_set_view_matrix(m: u32) {
        yi::bridge::yi_set_view_matrix(&mut mem(), m);
    }
    /// `AbmapMakeNodeMatrix(out, pos, scale)`.
    AbmapMakeNodeMatrix @ 0x00a5_ad30 => extern "C" fn abmap_make_node_matrix(out: u32, pos: u32, scale: u32) -> u32 {
        abmap::bridge::abmap_make_node_matrix(&mut mem(), out, pos, scale)
    }
    /// `AbmapMakeLinkMatrix(out, a, b, scale)`.
    AbmapMakeLinkMatrix @ 0x00a5_a360 => extern "C" fn abmap_make_link_matrix(out: u32, a: u32, b: u32, scale: u32) -> u32 {
        abmap::bridge::abmap_make_link_matrix(&mut mem(), out, a, b, scale)
    }
    /// `AbmapForEachNode(f, arg)`.
    AbmapForEachNode @ 0x00a5_9710 => extern "C" fn abmap_for_each_node(f: u32, arg: u32) -> u32 {
        abmap::bridge::abmap_for_each_node(&mut mem(), f, arg, |_, f, n, a| call_ptr2(f, n, a))
    }
    /// `AbmapForNodeAndLinked(node, f, arg)`.
    AbmapForNodeAndLinked @ 0x00a5_9760 => extern "C" fn abmap_for_node_and_linked(node: u32, f: u32, arg: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        abmap::bridge::abmap_for_node_and_linked(&mut mem(), node as u16, f, arg, |_, f, n, a| call_ptr2(f, n, a))
    }
    /// `AbmapResetHelp()`.
    AbmapResetHelp @ 0x00a5_b930 => extern "C" fn abmap_reset_help() -> u32 {
        abmap::bridge::abmap_reset_help(&mut mem())
    }
    /// `AbmapPulseCursor()`.
    AbmapPulseCursor @ 0x00a4_7440 => extern "C" fn abmap_pulse_cursor() -> u32 {
        abmap::bridge::abmap_pulse_cursor(&mut mem())
    }
    /// `AbmapSetChrMarker(chr)`.
    AbmapSetChrMarker @ 0x00a5_a990 => extern "C" fn abmap_set_chr_marker(chr: u32) -> u32 {
        abmap::bridge::abmap_set_chr_marker(&mut mem(), chr)
    }
    /// `AbmapQuarterOrOne_a5cee0(_, n)`.
    AbmapQuarterOrOne_a5cee0 @ 0x00a5_cee0 => extern "C" fn abmap_quarter_or_one_a(_a: u32, n: u32) -> u32 {
        abmap::bridge::abmap_quarter_or_one(n.cast_signed())
    }
    /// `AbmapQuarterOrOne_a5cf10(_, n)`.
    AbmapQuarterOrOne_a5cf10 @ 0x00a5_cf10 => extern "C" fn abmap_quarter_or_one_b(_a: u32, n: u32) -> u32 {
        abmap::bridge::abmap_quarter_or_one(n.cast_signed())
    }
    /// `AbmapVu0LoadMatrix1(m)`.
    AbmapVu0LoadMatrix1 @ 0x00a5_be30 => extern "C" fn abmap_vu0_load_matrix1(m: u32) -> u32 {
        abmap::bridge::abmap_vu0_load_matrix1(&mut mem(), m)
    }
    /// `AbmapVu0LoadMatrix2(m)`.
    AbmapVu0LoadMatrix2 @ 0x00a5_bec0 => extern "C" fn abmap_vu0_load_matrix2(m: u32) -> u32 {
        abmap::bridge::abmap_vu0_load_matrix2(&mut mem(), m)
    }
    /// `yiPolyBufClear(buf)`.
    yiPolyBufClear @ 0x0091_1970 => extern "C" fn yi_poly_buf_clear(buf: u32) -> u32 {
        yi::bridge::yi_poly_buf_clear(&mut mem(), buf)
    }
    /// `yiTriColorsFromGif(out, gif, first)`.
    yiTriColorsFromGif @ 0x0091_0340 => extern "C" fn yi_tri_colors_from_gif(out: u32, gif: u32, first: u32) -> u32 {
        yi::bridge::yi_tri_colors_from_gif(&mut mem(), out, gif, first)
    }
    /// `yiTriColorsUvFromGif(out, gif, first, uv)`.
    yiTriColorsUvFromGif @ 0x0091_03e0 => extern "C" fn yi_tri_colors_uv_from_gif(out: u32, gif: u32, first: u32, uv: u32) -> u32 {
        yi::bridge::yi_tri_colors_uv_from_gif(&mut mem(), out, gif, first, uv)
    }
    /// `yiLoadMatrixChunk(scene, chunk)`.
    yiLoadMatrixChunk @ 0x0090_ed10 => extern "C" fn yi_load_matrix_chunk(scene: u32, chunk: u32) -> u32 {
        yi::bridge::yi_load_matrix_chunk(&mut mem(), scene, chunk);
        0
    }
    /// `yiRender_setupFogParam(scene)`.
    yiRender_setupFogParam @ 0x0090_d1b0 => extern "C" fn yi_render_setup_fog_param(scene: u32) {
        yi::bridge::yi_render_setup_fog_param(&mut mem(), scene);
    }
    /// `yiObjSave(record, obj)`.
    yiObjSave @ 0x0090_ee60 => extern "C" fn yi_obj_save(rec: u32, obj: u32) -> u32 {
        yi::bridge::yi_obj_save(&mut mem(), rec, obj)
    }
    /// `yiObjLoad(obj, record)`.
    yiObjLoad @ 0x0090_edb0 => extern "C" fn yi_obj_load(obj: u32, rec: u32) -> u32 {
        yi::bridge::yi_obj_load(&mut mem(), obj, rec)
    }
    /// `yiWindowSetHandlers(window)`.
    yiWindowSetHandlers @ 0x0090_7d80 => extern "C" fn yi_window_set_handlers(w: u32) -> u32 {
        yi::bridge::yi_window_set_handlers(&mut mem(), w)
    }
    /// `yiCameraLoad(camera, record)`.
    yiCameraLoad @ 0x0090_ef10 => extern "C" fn yi_camera_load(cam: u32, rec: u32) -> u32 {
        yi::bridge::yi_camera_load(&mut mem(), cam, rec)
    }
    /// `yiFogLoad(scene, record)`.
    yiFogLoad @ 0x0090_f090 => extern "C" fn yi_fog_load(scene: u32, rec: u32) -> u32 {
        yi::bridge::yi_fog_load(&mut mem(), scene, rec)
    }
    /// `Yn_NearestOnLine2D(out, a, b, p, clamp)`.
    Yn_NearestOnLine2D @ 0x0091_cc00 => extern "C" fn yn_nearest_on_line_2d(out: u32, a: u32, b: u32, p: u32, clamp: u32) -> u32 {
        guide::bridge::yn_nearest_on_line_2d(&mut mem(), out, a, b, p, clamp)
    }
    /// `rcMeshPastIndices(obj)`.
    rcMeshPastIndices @ 0x0092_8270 => extern "C" fn rc_mesh_past_indices(obj: u32) -> u32 {
        model::bridge::rc_mesh_past_indices(&mem(), obj)
    }
    /// `rcbgIdentity(x)`.
    rcbgIdentity @ 0x0092_02a0 => extern "C" fn rcbg_identity(x: u32) -> u32 {
        model::bridge::rcbg_identity(x)
    }
    /// `rcfxGravityInit(fx)`.
    rcfxGravityInit @ 0x0092_9570 => extern "C" fn rcfx_gravity_init(fx: u32) -> u32 {
        model::bridge::rcfx_gravity_init(&mut mem(), fx)
    }
    /// `rcfxParticleInit(fx)`.
    rcfxParticleInit @ 0x0092_88e0 => extern "C" fn rcfx_particle_init(fx: u32) -> u32 {
        model::bridge::rcfx_particle_init(&mut mem(), fx)
    }
    /// `rcfxNailInit(fx)`.
    rcfxNailInit @ 0x0092_9100 => extern "C" fn rcfx_nail_init(fx: u32) -> u32 {
        model::bridge::rcfx_nail_init(&mut mem(), fx)
    }
    /// `rcfxWindInit(fx)`.
    rcfxWindInit @ 0x0092_9850 => extern "C" fn rcfx_wind_init(fx: u32) -> u32 {
        model::bridge::rcfx_wind_init(&mut mem(), fx)
    }
    /// `rcefAnimeStep(anim)`.
    rcefAnimeStep @ 0x0092_f400 => extern "C" fn rcef_anime_step(anim: u32) -> u32 {
        model::bridge::rcef_anime_step(&mut mem(), anim)
    }
    /// `rcefSetupDraw(draw, tex, a, b)`.
    rcefSetupDraw @ 0x0092_f6b0 => extern "C" fn rcef_setup_draw(draw: u32, tex: u32, a: u32, b: u32) -> u32 {
        model::bridge::rcef_setup_draw(&mut mem(), draw, tex, a, b)
    }
    /// `rcuiCrossProduct(out, a, b)`.
    rcuiCrossProduct @ 0x0093_d440 => extern "C" fn rcui_cross_product(out: u32, a: u32, b: u32) -> u32 {
        rcui::bridge::rcui_cross_product(&mut mem(), out, a, b)
    }
    /// `rcuiClearSelect()`.
    rcuiClearSelect @ 0x0093_4af0 => extern "C" fn rcui_clear_select() -> u32 {
        rcui::bridge::rcui_clear_select(&mut mem())
    }
    /// `rcuiSetSelect()`.
    rcuiSetSelect @ 0x0093_4b50 => extern "C" fn rcui_set_select() -> u32 {
        rcui::bridge::rcui_set_select(&mut mem())
    }
    /// `KeAddScaledColor(out, base, step, k)`.
    KeAddScaledColor @ 0x0076_4c40 => extern "C" fn ke_add_scaled_color(out: u32, base: u32, step: u32, k: u32) {
        ke::bridge::ke_add_scaled_color(&mut mem(), out, base, step, k);
    }
    /// `FreeCamera::FreeCamera()`.
    FreeCamera_FreeCamera @ 0x0067_fe70 => extern "thiscall" fn free_camera_ctor(this: u32) -> u32 {
        freecam::bridge::free_camera_ctor(&mut mem(), this, |_, f, a, tc| phyre_call(f, a, tc))
    }
    /// `FreeCamera::~FreeCamera()`.
    FreeCamera_dtor @ 0x0067_ff10 => extern "thiscall" fn free_camera_dtor(this: u32) {
        freecam::bridge::free_camera_dtor(&mut mem(), this, |_, f, a, tc| phyre_call(f, a, tc));
    }
    /// `FreeCamera`'s scalar deleting destructor.
    FreeCamera_scalarDeletingDtor @ 0x0067_ffb0 => extern "thiscall" fn free_camera_delete(this: u32, flags: u32) -> u32 {
        freecam::bridge::free_camera_delete(&mut mem(), this, flags, |_, f, a, tc| phyre_call(f, a, tc))
    }
    /// `FreeCamera_destroyInstance()`.
    FreeCamera_destroyInstance @ 0x0068_0000 => extern "C" fn free_camera_destroy_instance() {
        freecam::bridge::free_camera_destroy_instance(&mut mem(), |_, f, a, tc| phyre_call(f, a, tc));
    }
    /// `rcdtChunkFini(c)`.
    rcdtChunkFini @ 0x0092_b7f0 => extern "C" fn rcdt_chunk_fini(c: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_fini(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtChunkDelete(c)`.
    rcdtChunkDelete @ 0x0092_b7d0 => extern "C" fn rcdt_chunk_delete(c: u32) -> u32 {
        rcdt::bridge::rcdt_chunk_delete(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelFini(model)`.
    rcbgModelFini @ 0x0092_8240 => extern "C" fn rcbg_model_fini(model: u32) -> u32 {
        model::bridge::rcbg_model_fini(&mut mem(), model, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelDelete(model)`.
    rcbgModelDelete @ 0x0092_8000 => extern "C" fn rcbg_model_delete(model: u32) -> u32 {
        model::bridge::rcbg_model_delete(&mut mem(), model, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgTrxFini(trx)`.
    rcbgTrxFini @ 0x0092_d490 => extern "C" fn rcbg_trx_fini(trx: u32)  {
        model::bridge::rcbg_trx_fini(&mut mem(), trx, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxDelete(trx)`.
    rcbgTrxDelete @ 0x0092_d450 => extern "C" fn rcbg_trx_delete(trx: u32) -> u32 {
        model::bridge::rcbg_trx_delete(&mut mem(), trx, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgLayerAnimeFini(a)`.
    rcbgLayerAnimeFini @ 0x0093_0830 => extern "C" fn rcbg_layer_anime_fini(a: u32) -> u32 {
        model::bridge::rcbg_layer_anime_fini(&mut mem(), a, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgLayerFini(layer)`.
    rcbgLayerFini @ 0x0092_ac90 => extern "C" fn rcbg_layer_fini(layer: u32) -> u32 {
        scene::bridge::rcbg_layer_fini(&mut mem(), layer, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgSceneFini(s)`.
    rcbgSceneFini @ 0x0092_0870 => extern "C" fn rcbg_scene_fini(s: u32) -> u32 {
        scene::bridge::rcbg_scene_fini(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgGuideFini(g)`.
    rcbgGuideFini @ 0x0091_d3d0 => extern "C" fn rcbg_guide_fini(g: u32) -> u32 {
        guide::bridge::rcbg_guide_fini(&mut mem(), g, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgGuideDelete(g)`.
    rcbgGuideDelete @ 0x0091_cef0 => extern "C" fn rcbg_guide_delete(g: u32) -> u32 {
        guide::bridge::rcbg_guide_delete(&mut mem(), g, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgPreFini()`.
    rcbgPreFini @ 0x0092_16b0 => extern "C" fn rcbg_pre_fini() -> u32 {
        guide::bridge::rcbg_pre_fini(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFini()`.
    rcbgFini @ 0x0092_1aa0 => extern "C" fn rcbg_fini() -> u32 {
        guide::bridge::rcbg_fini(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `Yn_MapSystemFini()`.
    Yn_MapSystemFini @ 0x0091_9a60 => extern "C" fn yn_map_system_fini() -> u32 {
        guide::bridge::yn_map_system_fini(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowGrowInit(window)`.
    yiWindowGrowInit @ 0x0090_7cc0 => extern "C" fn shim_yiwindowgrowinit(w: u32) -> u32 {
        yi::bridge::yi_window_grow_init(&mut mem(), w)
    }
    /// `yiWindowGrowOpen(window)`.
    yiWindowGrowOpen @ 0x0090_7cf0 => extern "C" fn shim_yiwindowgrowopen(w: u32) -> u32 {
        yi::bridge::yi_window_grow_open(&mut mem(), w)
    }
    /// `yiWindowGrowClose(window)`.
    yiWindowGrowClose @ 0x0090_6fc0 => extern "C" fn shim_yiwindowgrowclose(w: u32) -> u32 {
        yi::bridge::yi_window_grow_close(&mut mem(), w)
    }
    /// `yiWindowFinishAll(window)`.
    yiWindowFinishAll @ 0x0090_7040 => extern "C" fn shim_yiwindowfinishall(w: u32) -> u32 {
        yi::bridge::yi_window_finish_all(&mut mem(), w)
    }
    /// `yiWindowFinishAll_00907bf0(window)`.
    yiWindowFinishAll_00907bf0 @ 0x0090_7bf0 => extern "C" fn shim_yiwindowfinishall_00907bf0(w: u32) -> u32 {
        yi::bridge::yi_window_finish_all(&mut mem(), w)
    }
    /// `yiWindowPopInit(window)`.
    yiWindowPopInit @ 0x0090_7160 => extern "C" fn shim_yiwindowpopinit(w: u32) -> u32 {
        yi::bridge::yi_window_pop_init(&mut mem(), w)
    }
    /// `yiWindowPopOpen(window)`.
    yiWindowPopOpen @ 0x0090_7190 => extern "C" fn shim_yiwindowpopopen(w: u32) -> u32 {
        yi::bridge::yi_window_pop_open(&mut mem(), w)
    }
    /// `yiWindowPopClose(window)`.
    yiWindowPopClose @ 0x0090_71c0 => extern "C" fn shim_yiwindowpopclose(w: u32) -> u32 {
        yi::bridge::yi_window_pop_close(&mut mem(), w)
    }
    /// `yiWindowFinish_009071f0(window)`.
    yiWindowFinish_009071f0 @ 0x0090_71f0 => extern "C" fn shim_yiwindowfinish_009071f0(w: u32) -> u32 {
        yi::bridge::yi_window_finish(&mut mem(), w)
    }
    /// `yiWindowSlideDownInit(window)`.
    yiWindowSlideDownInit @ 0x0090_7450 => extern "C" fn shim_yiwindowslidedowninit(w: u32) -> u32 {
        yi::bridge::yi_window_slide_down_init(&mut mem(), w)
    }
    /// `yiWindowSlideDownOpen(window)`.
    yiWindowSlideDownOpen @ 0x0090_72e0 => extern "C" fn shim_yiwindowslidedownopen(w: u32) -> u32 {
        yi::bridge::yi_window_slide_down_open(&mut mem(), w)
    }
    /// `yiWindowSlideDownClose(window)`.
    yiWindowSlideDownClose @ 0x0090_7330 => extern "C" fn shim_yiwindowslidedownclose(w: u32) -> u32 {
        yi::bridge::yi_window_slide_down_close(&mut mem(), w)
    }
    /// `yiWindowFinish_00907370(window)`.
    yiWindowFinish_00907370 @ 0x0090_7370 => extern "C" fn shim_yiwindowfinish_00907370(w: u32) -> u32 {
        yi::bridge::yi_window_finish(&mut mem(), w)
    }
    /// `yiWindowSlideUpInitHere(window)`.
    yiWindowSlideUpInitHere @ 0x0090_74b0 => extern "C" fn shim_yiwindowslideupinithere(w: u32) -> u32 {
        yi::bridge::yi_window_slide_up_init_here(&mut mem(), w)
    }
    /// `yiWindowSlideUpInit(window)`.
    yiWindowSlideUpInit @ 0x0090_7690 => extern "C" fn shim_yiwindowslideupinit(w: u32) -> u32 {
        yi::bridge::yi_window_slide_up_init(&mut mem(), w)
    }
    /// `yiWindowSlideUpOpen(window)`.
    yiWindowSlideUpOpen @ 0x0090_7520 => extern "C" fn shim_yiwindowslideupopen(w: u32) -> u32 {
        yi::bridge::yi_window_slide_up_open(&mut mem(), w)
    }
    /// `yiWindowSlideUpClose(window)`.
    yiWindowSlideUpClose @ 0x0090_7570 => extern "C" fn shim_yiwindowslideupclose(w: u32) -> u32 {
        yi::bridge::yi_window_slide_up_close(&mut mem(), w)
    }
    /// `yiWindowFinish_009075b0(window)`.
    yiWindowFinish_009075b0 @ 0x0090_75b0 => extern "C" fn shim_yiwindowfinish_009075b0(w: u32) -> u32 {
        yi::bridge::yi_window_finish(&mut mem(), w)
    }
    /// `yiWindowSlideRightInitHere(window)`.
    yiWindowSlideRightInitHere @ 0x0090_76f0 => extern "C" fn shim_yiwindowsliderightinithere(w: u32) -> u32 {
        yi::bridge::yi_window_slide_right_init_here(&mut mem(), w)
    }
    /// `yiWindowSlideRightInit(window)`.
    yiWindowSlideRightInit @ 0x0090_78c0 => extern "C" fn shim_yiwindowsliderightinit(w: u32) -> u32 {
        yi::bridge::yi_window_slide_right_init(&mut mem(), w)
    }
    /// `yiWindowSlideRightOpen(window)`.
    yiWindowSlideRightOpen @ 0x0090_7750 => extern "C" fn shim_yiwindowsliderightopen(w: u32) -> u32 {
        yi::bridge::yi_window_slide_right_open(&mut mem(), w)
    }
    /// `yiWindowSlideRightClose(window)`.
    yiWindowSlideRightClose @ 0x0090_77a0 => extern "C" fn shim_yiwindowsliderightclose(w: u32) -> u32 {
        yi::bridge::yi_window_slide_right_close(&mut mem(), w)
    }
    /// `yiWindowFinish_009077e0(window)`.
    yiWindowFinish_009077e0 @ 0x0090_77e0 => extern "C" fn shim_yiwindowfinish_009077e0(w: u32) -> u32 {
        yi::bridge::yi_window_finish(&mut mem(), w)
    }
    /// `yiWindowSlideLeftInitHere(window)`.
    yiWindowSlideLeftInitHere @ 0x0090_7920 => extern "C" fn shim_yiwindowslideleftinithere(w: u32) -> u32 {
        yi::bridge::yi_window_slide_left_init_here(&mut mem(), w)
    }
    /// `yiWindowSlideLeftInit(window)`.
    yiWindowSlideLeftInit @ 0x0090_7b00 => extern "C" fn shim_yiwindowslideleftinit(w: u32) -> u32 {
        yi::bridge::yi_window_slide_left_init(&mut mem(), w)
    }
    /// `yiWindowSlideLeftOpen(window)`.
    yiWindowSlideLeftOpen @ 0x0090_7990 => extern "C" fn shim_yiwindowslideleftopen(w: u32) -> u32 {
        yi::bridge::yi_window_slide_left_open(&mut mem(), w)
    }
    /// `yiWindowSlideLeftClose(window)`.
    yiWindowSlideLeftClose @ 0x0090_79e0 => extern "C" fn shim_yiwindowslideleftclose(w: u32) -> u32 {
        yi::bridge::yi_window_slide_left_close(&mut mem(), w)
    }
    /// `yiWindowFinish_00907a20(window)`.
    yiWindowFinish_00907a20 @ 0x0090_7a20 => extern "C" fn shim_yiwindowfinish_00907a20(w: u32) -> u32 {
        yi::bridge::yi_window_finish(&mut mem(), w)
    }
    /// `yiWindowBarInit(window)`.
    yiWindowBarInit @ 0x0090_7b60 => extern "C" fn shim_yiwindowbarinit(w: u32) -> u32 {
        yi::bridge::yi_window_bar_init(&mut mem(), w)
    }
    /// `yiWindowBarOpen(window)`.
    yiWindowBarOpen @ 0x0090_7ba0 => extern "C" fn shim_yiwindowbaropen(w: u32) -> u32 {
        yi::bridge::yi_window_bar_open(&mut mem(), w)
    }
    /// `yiWindowBarClose(window)`.
    yiWindowBarClose @ 0x0090_7bd0 => extern "C" fn shim_yiwindowbarclose(w: u32) -> u32 {
        yi::bridge::yi_window_bar_close(&mut mem(), w)
    }
    /// `rcCopyQword_0091cb40(dst, src)`.
    rcCopyQword_0091cb40 @ 0x0091_cb40 => extern "C" fn rc_copy_qword_0091cb40(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_0091cbd0(dst, src)`.
    rcCopyQword_0091cbd0 @ 0x0091_cbd0 => extern "C" fn rc_copy_qword_0091cbd0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_0091f710(dst, src)`.
    rcCopyQword_0091f710 @ 0x0091_f710 => extern "C" fn rc_copy_qword_0091f710(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_00922510(dst, src)`.
    rcCopyQword_00922510 @ 0x0092_2510 => extern "C" fn rc_copy_qword_00922510(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_00924830(dst, src)`.
    rcCopyQword_00924830 @ 0x0092_4830 => extern "C" fn rc_copy_qword_00924830(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_00924f90(dst, src)`.
    rcCopyQword_00924f90 @ 0x0092_4f90 => extern "C" fn rc_copy_qword_00924f90(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_009250a0(dst, src)`.
    rcCopyQword_009250a0 @ 0x0092_50a0 => extern "C" fn rc_copy_qword_009250a0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_009273b0(dst, src)`.
    rcCopyQword_009273b0 @ 0x0092_73b0 => extern "C" fn rc_copy_qword_009273b0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_00927a90(dst, src)`.
    rcCopyQword_00927a90 @ 0x0092_7a90 => extern "C" fn rc_copy_qword_00927a90(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_00928eb0(dst, src)`.
    rcCopyQword_00928eb0 @ 0x0092_8eb0 => extern "C" fn rc_copy_qword_00928eb0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_009296a0(dst, src)`.
    rcCopyQword_009296a0 @ 0x0092_96a0 => extern "C" fn rc_copy_qword_009296a0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_0092d6f0(dst, src)`.
    rcCopyQword_0092d6f0 @ 0x0092_d6f0 => extern "C" fn rc_copy_qword_0092d6f0(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcCopyQword_0092f850(dst, src)`.
    rcCopyQword_0092f850 @ 0x0092_f850 => extern "C" fn rc_copy_qword_0092f850(dst: u32, src: u32) -> u32 {
        vu0::bridge::rc_copy_qword(&mut mem(), dst, src)
    }
    /// `rcdtParseNextToken(parse)`.
    rcdtParseNextToken @ 0x0092_ba30 => extern "C" fn rcdt_parse_next_token(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_next_token(&mut mem(), p)
    }
    /// `rcdtParseTokenLen(parse)`.
    rcdtParseTokenLen @ 0x0092_bf30 => extern "C" fn rcdt_parse_token_len(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_token_len(&mem(), p)
    }
    /// `rcdtParseTokenLenText(parse)`.
    rcdtParseTokenLenText @ 0x0092_be10 => extern "C" fn rcdt_parse_token_len_text(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_token_len_text(&mem(), p)
    }
    /// `rcdtParseSeekChar(parse, c)`.
    rcdtParseSeekChar @ 0x0092_b990 => extern "C" fn rcdt_parse_seek_char(p: u32, c: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte, as movsx reads it
        rcdt::bridge::rcdt_parse_seek_char(&mut mem(), p, (c as u8).cast_signed())
    }
    /// `rcbgEmbeddedTrxGetCnt(obj)`.
    rcbgEmbeddedTrxGetCnt @ 0x0092_2140 => extern "C" fn rcbg_embedded_trx_get_cnt(obj: u32) -> u32 {
        model::bridge::rcbg_embedded_trx_get_cnt(&mem(), obj)
    }
    /// `Yn_EmbeddedTrxGetCnt(obj)`.
    Yn_EmbeddedTrxGetCnt @ 0x0091_9bc0 => extern "C" fn yn_embedded_trx_get_cnt(obj: u32) -> u32 {
        model::bridge::rcbg_embedded_trx_get_cnt(&mem(), obj)
    }
    /// `rcefSetStream(stream)`.
    rcefSetStream @ 0x0092_5710 => extern "C" fn rcef_set_stream(s: u32) -> u32 {
        model::bridge::rcef_set_stream(&mut mem(), s)
    }
    /// `rcbgRunRecordHook(obj)`.
    rcbgRunRecordHook @ 0x0091_fd60 => extern "C" fn rcbg_run_record_hook(obj: u32) -> u32 {
        model::bridge::rcbg_run_record_hook(&mut mem(), obj, |_, f, r| call_ptr1(f, r))
    }
    /// `SqsCmdSetC2(ctx, cmd)`.
    SqsCmdSetC2 @ 0x0081_c140 => extern "C" fn sqs_cmd_set_c2(ctx: u32, cmd: u32) -> u32 {
        nkz::bridge::sqs_cmd_set_c2(&mut mem(), ctx, cmd)
    }
    /// `yiObjInit(obj)`.
    yiObjInit @ 0x0090_ff40 => extern "C" fn yi_obj_init(obj: u32) {
        yi::bridge::yi_obj_init(&mut mem(), obj);
    }
    /// `yiLoadVramImage(scene, entry, data)`.
    yiLoadVramImage @ 0x0090_e5d0 => extern "C" fn yi_load_vram_image(scene: u32, entry: u32, data: u32) -> u32 {
        yi::bridge::yi_load_vram_image(&mut mem(), scene, entry, data, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiMapFileRead(scene, file)`.
    yiMapFileRead @ 0x0090_f900 => extern "C" fn yi_map_file_read(scene: u32, file: u32) -> u32 {
        let mut image = Image([0; 0x60]);
        let at = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_map_file_read(&mut mem(), scene, file, at, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawPlate(window)`.
    yiWindowDrawPlate @ 0x0090_7390 => extern "C" fn shim_yiwindowdrawplate(w: u32) -> u32 {
        yi::bridge::yi_window_draw_plate(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawPlate_009075d0(window)`.
    yiWindowDrawPlate_009075d0 @ 0x0090_75d0 => extern "C" fn shim_yiwindowdrawplate_009075d0(w: u32) -> u32 {
        yi::bridge::yi_window_draw_plate(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawPlate_00907800(window)`.
    yiWindowDrawPlate_00907800 @ 0x0090_7800 => extern "C" fn shim_yiwindowdrawplate_00907800(w: u32) -> u32 {
        yi::bridge::yi_window_draw_plate(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawPlate_00907a40(window)`.
    yiWindowDrawPlate_00907a40 @ 0x0090_7a40 => extern "C" fn shim_yiwindowdrawplate_00907a40(w: u32) -> u32 {
        yi::bridge::yi_window_draw_plate(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawMenu(window)`.
    yiWindowDrawMenu @ 0x0090_7060 => extern "C" fn shim_yiwindowdrawmenu(w: u32) -> u32 {
        yi::bridge::yi_window_draw_menu(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawPop(window)`.
    yiWindowDrawPop @ 0x0090_7210 => extern "C" fn shim_yiwindowdrawpop(w: u32) -> u32 {
        yi::bridge::yi_window_draw_pop(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiWindowDrawBar(w)`.
    yiWindowDrawBar @ 0x0090_7c10 => extern "C" fn shim_yi_window_draw_bar(w: u32) -> u32 {
        yi::bridge::yi_window_draw_bar(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcAlloc(size: u32, _file: u32, _line: u32)`.
    rcAlloc @ 0x0092_58a0 => extern "C" fn shim_rc_alloc(size: u32, _file: u32, _line: u32) -> u32 {
        model::bridge::rc_alloc(&mut mem(), &mut |_, f, a| call_cdecl(f, a), size)
    }
    /// `rcCalloc(n: u32, size: u32, _file: u32, _line: u32)`.
    rcCalloc @ 0x0092_58b0 => extern "C" fn shim_rc_calloc(n: u32, size: u32, _file: u32, _line: u32) -> u32 {
        model::bridge::rc_calloc(&mut mem(), &mut |_, f, a| call_cdecl(f, a), n, size)
    }
    /// `rcRealloc(p: u32, size: u32, _file: u32, _line: u32)`.
    rcRealloc @ 0x0092_58f0 => extern "C" fn shim_rc_realloc(p: u32, size: u32, _file: u32, _line: u32) -> u32 {
        model::bridge::rc_realloc(&mut mem(), &mut |_, f, a| call_cdecl(f, a), p, size)
    }
    /// `yiMalloc(size: u32)`.
    yiMalloc @ 0x0090_bbd0 => extern "C" fn shim_yi_malloc(size: u32) -> u32 {
        model::bridge::yi_malloc(&mut mem(), &mut |_, f, a| call_cdecl(f, a), size)
    }
    /// `yiMalloc64(size: u32)`.
    yiMalloc64 @ 0x0090_bbe0 => extern "C" fn shim_yi_malloc64(size: u32) -> u32 {
        model::bridge::yi_malloc64(&mut mem(), &mut |_, f, a| call_cdecl(f, a), size)
    }
    /// `rcbgTrxNew()`.
    rcbgTrxNew @ 0x0092_d580 => extern "C" fn shim_rcbg_trx_new() -> u32 {
        model::bridge::rcbg_trx_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgTrxTexInit(t: u32)`.
    rcbgTrxTexInit @ 0x0092_2160 => extern "C" fn shim_rcbg_trx_tex_init(t: u32)  {
        model::bridge::rcbg_trx_tex_init(&mut mem(), t);
    }
    /// `rcbgTrxTexNew()`.
    rcbgTrxTexNew @ 0x0092_21b0 => extern "C" fn shim_rcbg_trx_tex_new() -> u32 {
        model::bridge::rcbg_trx_tex_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFpTrxTexNew()`.
    rcbgFpTrxTexNew @ 0x0092_20c0 => extern "C" fn shim_rcbg_fp_trx_tex_new() -> u32 {
        model::bridge::rcbg_fp_trx_tex_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelInit(m: u32)`.
    rcbgModelInit @ 0x0092_8320 => extern "C" fn shim_rcbg_model_init(m: u32)  {
        model::bridge::rcbg_model_init(&mut mem(), m);
    }
    /// `rcbgModelNew()`.
    rcbgModelNew @ 0x0092_83b0 => extern "C" fn shim_rcbg_model_new() -> u32 {
        model::bridge::rcbg_model_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelLoad(m: u32, p: u32)`.
    rcbgModelLoad @ 0x0092_b2f0 => extern "C" fn shim_rcbg_model_load(m: u32, p: u32)  {
        model::bridge::rcbg_model_load(&mut mem(), m, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxTexLoad(t: u32, p: u32)`.
    rcbgTrxTexLoad @ 0x0092_b780 => extern "C" fn shim_rcbg_trx_tex_load(t: u32, p: u32)  {
        model::bridge::rcbg_trx_tex_load(&mut mem(), t, p);
    }
    /// `rcbgAnimeSetDataCnt(a: u32, n: i32)`.
    rcbgAnimeSetDataCnt @ 0x0093_08d0 => extern "C" fn shim_rcbg_anime_set_data_cnt(a: u32, n: i32)  {
        model::bridge::rcbg_anime_set_data_cnt(&mut mem(), a, n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxTexFini(t: u32)`.
    rcbgTrxTexFini @ 0x0092_2120 => extern "C" fn shim_rcbg_trx_tex_fini(t: u32)  {
        model::bridge::rcbg_trx_tex_fini(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxTexDelete(t: u32)`.
    rcbgTrxTexDelete @ 0x0092_20e0 => extern "C" fn shim_rcbg_trx_tex_delete(t: u32) -> u32 {
        model::bridge::rcbg_trx_tex_delete(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgTrxTexDraw(t: u32)`.
    rcbgTrxTexDraw @ 0x0092_2100 => extern "C" fn shim_rcbg_trx_tex_draw(t: u32)  {
        scene::bridge::rcbg_trx_tex_draw(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneAddObject(s: u32, obj: u32)`.
    rcbgSceneAddObject @ 0x0092_0540 => extern "C" fn shim_rcbg_scene_add_object(s: u32, obj: u32)  {
        scene::bridge::rcbg_scene_add_object(&mut mem(), s, obj, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneModelLoad(s: u32, p: u32)`.
    rcbgSceneModelLoad @ 0x0092_b6b0 => extern "C" fn shim_rcbg_scene_model_load(s: u32, p: u32)  {
        scene::bridge::rcbg_scene_model_load(&mut mem(), s, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneTrxLoad(s: u32, p: u32)`.
    rcbgSceneTrxLoad @ 0x0092_b6e0 => extern "C" fn shim_rcbg_scene_trx_load(s: u32, p: u32)  {
        scene::bridge::rcbg_scene_trx_load(&mut mem(), s, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgLayerLoad(l: u32, p: u32)`.
    rcbgLayerLoad @ 0x0092_b1a0 => extern "C" fn shim_rcbg_layer_load(l: u32, p: u32)  {
        scene::bridge::rcbg_layer_load(&mut mem(), l, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxDraw_0092d470(t: u32)`.
    rcbgTrxDraw_0092d470 @ 0x0092_d470 => extern "C" fn shim_rcbg_trx_draw_pending(t: u32)  {
        scene::bridge::rcbg_trx_draw_pending(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneChkSortLayer(s: u32)`.
    rcbgSceneChkSortLayer @ 0x0092_b460 => extern "C" fn shim_rcbg_scene_chk_sort_layer(s: u32)  {
        scene::bridge::rcbg_scene_chk_sort_layer(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneChkLayers(s: u32)`.
    rcbgSceneChkLayers @ 0x0092_b400 => extern "C" fn shim_rcbg_scene_chk_layers(s: u32)  {
        scene::bridge::rcbg_scene_chk_layers(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneLoad(s: u32, p: u32)`.
    rcbgSceneLoad @ 0x0092_b5e0 => extern "C" fn shim_rcbg_scene_load(s: u32, p: u32)  {
        scene::bridge::rcbg_scene_load(&mut mem(), s, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreLoad(p: u32)`.
    rcbgPreLoad @ 0x0092_b390 => extern "C" fn shim_rcbg_pre_load(p: u32)  {
        guide::bridge::rcbg_pre_load(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreInit()`.
    rcbgPreInit @ 0x0092_1770 => extern "C" fn shim_rcbg_pre_init()  {
        guide::bridge::rcbg_pre_init(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreDummyInit()`.
    rcbgPreDummyInit @ 0x0092_1640 => extern "C" fn shim_rcbg_pre_dummy_init()  {
        guide::bridge::rcbg_pre_dummy_init(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreAddObject(obj: u32)`.
    rcbgPreAddObject @ 0x0092_18c0 => extern "C" fn shim_rcbg_pre_add_object(obj: u32)  {
        guide::bridge::rcbg_pre_add_object(&mut mem(), obj, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgGuideInit(g: u32)`.
    rcbgGuideInit @ 0x0091_d490 => extern "C" fn shim_rcbg_guide_init(g: u32)  {
        guide::bridge::rcbg_guide_init(&mut mem(), g);
    }
    /// `rcbgGuideNew()`.
    rcbgGuideNew @ 0x0091_d4e0 => extern "C" fn shim_rcbg_guide_new() -> u32 {
        guide::bridge::rcbg_guide_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgInit()`.
    rcbgInit @ 0x0092_1ce0 => extern "C" fn shim_rcbg_init()  {
        guide::bridge::rcbg_init(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgGuideSetModel(g: u32, model: u32)`.
    rcbgGuideSetModel @ 0x0091_d590 => extern "C" fn shim_rcbg_guide_set_model(g: u32, model: u32) -> u32 {
        guide::bridge::rcbg_guide_set_model(&mut mem(), g, model, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgPreSetSceneNo(n: i32)`.
    rcbgPreSetSceneNo @ 0x0092_17e0 => extern "C" fn shim_rcbg_pre_set_scene_no(n: i32)  {
        guide::bridge::rcbg_pre_set_scene_no(&mut mem(), n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiArc_readmem(scene: u32, file: u32)`.
    yiArc_readmem @ 0x0090_f8e0 => extern "C" fn shim_yi_arc_readmem(scene: u32, file: u32) -> u32 {
        let mut image = Image([0; 0x60]);
        let at = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_arc_readmem(&mut mem(), scene, file, at, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiAnimInfo_init(info: u32)`.
    yiAnimInfo_init @ 0x0091_4310 => extern "C" fn shim_yi_anim_info_init(info: u32)  {
        yi::bridge::yi_anim_info_init(&mut mem(), info, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiAnimDetach(scene: u32, obj: u32, kind: u32)`.
    yiAnimDetach @ 0x0091_4920 => extern "C" fn shim_yi_anim_detach(scene: u32, obj: u32, kind: u32) -> u32 {
        yi::bridge::yi_anim_detach(&mut mem(), scene, obj, kind, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiKeyFrame_attachDObj(scene: u32, obj: u32, anim: u32, arg: u32)`.
    yiKeyFrame_attachDObj @ 0x0091_44a0 => extern "C" fn shim_yi_key_frame_attach_dobj(scene: u32, obj: u32, anim: u32, arg: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the original stores the low half
        yi::bridge::yi_key_frame_attach_dobj(&mut mem(), scene, obj, anim, arg as u16, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiAnimAttachDefault(scene: u32)`.
    yiAnimAttachDefault @ 0x0091_4250 => extern "C" fn shim_yi_anim_attach_all(scene: u32)  {
        yi::bridge::yi_anim_attach_all(&mut mem(), scene, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiRender_resetobj(r: u32)`.
    yiRender_resetobj @ 0x0090_cd90 => extern "C" fn shim_yi_render_resetobj(r: u32)  {
        yi::bridge::yi_render_resetobj(&mut mem(), r, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiResetObj()`.
    yiResetObj @ 0x0090_9ea0 => extern "C" fn shim_yi_reset_obj()  {
        yi::bridge::yi_reset_obj(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgLoadBin(data)`.
    rcbgLoadBin @ 0x0092_1d60 => extern "C" fn shim_rcbg_load_bin(data: u32) {
        let mut work = Work([0; 0xa4]);
        let at = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        guide::bridge::rcbg_load_bin(&mut mem(), data, at, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `Yn_PreBgSetData(data)`.
    Yn_PreBgSetData @ 0x0091_aa60 => extern "C" fn shim_yn_pre_bg_set_data(data: u32) {
        let mut work = Work([0; 0xa4]);
        let at = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        guide::bridge::yn_pre_bg_set_data(&mut mem(), data, at, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiObjReadFinish(read)`.
    yiObjReadFinish @ 0x0090_a9e0 => extern "C" fn shim_yi_obj_read_finish(read: u32) {
        let mut image = Image([0; 0x60]);
        let at = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_obj_read_finish(&mut mem(), read, at, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxParticleNew()`.
    rcfxParticleNew @ 0x0092_8950 => extern "C" fn shim_rcfx_particle_new() -> u32 {
        model::bridge::rcfx_particle_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxNailNew()`.
    rcfxNailNew @ 0x0092_9170 => extern "C" fn shim_rcfx_nail_new() -> u32 {
        model::bridge::rcfx_nail_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxGravityNew()`.
    rcfxGravityNew @ 0x0092_95c0 => extern "C" fn shim_rcfx_gravity_new() -> u32 {
        model::bridge::rcfx_gravity_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxWindNew()`.
    rcfxWindNew @ 0x0092_98c0 => extern "C" fn shim_rcfx_wind_new() -> u32 {
        model::bridge::rcfx_wind_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxDampNew()`.
    rcfxDampNew @ 0x0092_9420 => extern "C" fn shim_rcfx_damp_new() -> u32 {
        model::bridge::rcfx_damp_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgPSimNew()`.
    rcbgPSimNew @ 0x0091_fe40 => extern "C" fn shim_rcbg_psim_new() -> u32 {
        model::bridge::rcbg_psim_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFPm097New()`.
    rcbgFPm097New @ 0x0092_25a0 => extern "C" fn shim_rcbg_fpm097_new() -> u32 {
        model::bridge::rcbg_fpm097_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFPm173New()`.
    rcbgFPm173New @ 0x0092_2bb0 => extern "C" fn shim_rcbg_fpm173_new() -> u32 {
        model::bridge::rcbg_fpm173_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFPm565New()`.
    rcbgFPm565New @ 0x0092_2f80 => extern "C" fn shim_rcbg_fpm565_new() -> u32 {
        model::bridge::rcbg_fpm565_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtRsd2New()`.
    rcdtRsd2New @ 0x0092_4750 => extern "C" fn shim_rcdt_rsd2_new() -> u32 {
        model::bridge::rcdt_rsd2_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtChunkNew()`.
    rcdtChunkNew @ 0x0092_b900 => extern "C" fn shim_rcdt_chunk_new() -> u32 {
        model::bridge::rcdt_chunk_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgGuideSetWalkFog(guide)`.
    rcbgGuideSetWalkFog @ 0x0091_ce50 => extern "C" fn shim_rcbg_guide_set_walk_fog(g: u32) {
        guide::bridge::rcbg_guide_set_walk_fog(&mut mem(), g, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgGuideSetModelSub(guide)`.
    rcbgGuideSetModelSub @ 0x0091_d880 => extern "C" fn shim_rcbg_guide_set_model_sub(g: u32) {
        guide::bridge::rcbg_guide_set_model_sub(&mut mem(), g, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiObjReadCallback(read)`.
    yiObjReadCallback @ 0x0090_97c0 => extern "C" fn shim_yi_obj_read_callback(read: u32) {
        let mut work = Work([0; 0xa4]);
        let mut image = Image([0; 0x60]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let i = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_obj_read_callback(&mut mem(), read, w, i, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgScaleVector(dst, v, s)`.
    rcbgScaleVector @ 0x0092_00a0 => extern "C" fn shim_rcbg_scale_vector(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgModelScaleVector(dst, v, s)`.
    rcbgModelScaleVector @ 0x0092_7b00 => extern "C" fn shim_rcbg_model_scale_vector(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_0091cae0(dst, v, s)`.
    rcbgScaleVector_0091cae0 @ 0x0091_cae0 => extern "C" fn shim_rcbg_scale_vector_0091cae0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_0091f6b0(dst, v, s)`.
    rcbgScaleVector_0091f6b0 @ 0x0091_f6b0 => extern "C" fn shim_rcbg_scale_vector_0091f6b0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_009224b0(dst, v, s)`.
    rcbgScaleVector_009224b0 @ 0x0092_24b0 => extern "C" fn shim_rcbg_scale_vector_009224b0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_009247d0(dst, v, s)`.
    rcbgScaleVector_009247d0 @ 0x0092_47d0 => extern "C" fn shim_rcbg_scale_vector_009247d0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_00928600(dst, v, s)`.
    rcbgScaleVector_00928600 @ 0x0092_8600 => extern "C" fn shim_rcbg_scale_vector_00928600(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_00928e50(dst, v, s)`.
    rcbgScaleVector_00928e50 @ 0x0092_8e50 => extern "C" fn shim_rcbg_scale_vector_00928e50(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_009292a0(dst, v, s)`.
    rcbgScaleVector_009292a0 @ 0x0092_92a0 => extern "C" fn shim_rcbg_scale_vector_009292a0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_009294a0(dst, v, s)`.
    rcbgScaleVector_009294a0 @ 0x0092_94a0 => extern "C" fn shim_rcbg_scale_vector_009294a0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_00929640(dst, v, s)`.
    rcbgScaleVector_00929640 @ 0x0092_9640 => extern "C" fn shim_rcbg_scale_vector_00929640(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_0092d690(dst, v, s)`.
    rcbgScaleVector_0092d690 @ 0x0092_d690 => extern "C" fn shim_rcbg_scale_vector_0092d690(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_0092d780(dst, v, s)`.
    rcbgScaleVector_0092d780 @ 0x0092_d780 => extern "C" fn shim_rcbg_scale_vector_0092d780(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector_0092f7f0(dst, v, s)`.
    rcbgScaleVector_0092f7f0 @ 0x0092_f7f0 => extern "C" fn shim_rcbg_scale_vector_0092f7f0(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector4_0091cb70(dst, v, s)`.
    rcbgScaleVector4_0091cb70 @ 0x0091_cb70 => extern "C" fn shim_rcbg_scale_vector4_0091cb70(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector4(&mut mem(), dst, v, s)
    }
    /// `rcbgScaleVector4_00925040(dst, v, s)`.
    rcbgScaleVector4_00925040 @ 0x0092_5040 => extern "C" fn shim_rcbg_scale_vector4_00925040(dst: u32, v: u32, s: u32) -> u32 {
        vu0::bridge::rcbg_scale_vector4(&mut mem(), dst, v, s)
    }
    /// `rcVectorToFixed_009273e0(dst, v)`.
    rcVectorToFixed_009273e0 @ 0x0092_73e0 => extern "C" fn shim_rc_vector_to_fixed_009273e0(dst: u32, v: u32) -> u32 {
        vu0::bridge::rc_vector_to_fixed(&mut mem(), dst, v)
    }
    /// `rcVectorToFixed_00927ac0(dst, v)`.
    rcVectorToFixed_00927ac0 @ 0x0092_7ac0 => extern "C" fn shim_rc_vector_to_fixed_00927ac0(dst: u32, v: u32) -> u32 {
        vu0::bridge::rc_vector_to_fixed(&mut mem(), dst, v)
    }
    /// `rcbgModelVectorToInt(dst, v)`.
    rcbgModelVectorToInt @ 0x0092_7b60 => extern "C" fn shim_rcbg_model_vector_to_int(dst: u32, v: u32) -> u32 {
        vu0::bridge::rcbg_model_vector_to_int(&mut mem(), dst, v)
    }
    /// `rcbgSetProjectionMatrixSub(out, fov, dx, dy, unused, distance)`.
    rcbgSetProjectionMatrixSub @ 0x0092_02e0 => extern "C" fn shim_rcbg_set_projection_matrix_sub(out: u32, fov: u32, dx: u32, dy: u32, unused: u32, distance: u32) -> f32 {
        let _ = unused;
        camera::bridge::rcbg_set_projection_matrix_sub(
            &mut mem(),
            out,
            fov,
            dx,
            dy,
            distance,
            &mut |_, f, a| call_cdecl(f, a),
            &mut |_, f, a| call_float(f, a),
        )
    }
    /// `rcbgSetViewMatrix(out, eye, at, up, roll)`.
    rcbgSetViewMatrix @ 0x0092_04b0 => extern "C" fn shim_rcbg_set_view_matrix(out: u32, eye: u32, at: u32, up: u32, roll: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        camera::bridge::rcbg_set_view_matrix(&mut mem(), out, eye, at, up, roll, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgSceneSetMatrix(view)`.
    rcbgSceneSetMatrix @ 0x0092_0c00 => extern "C" fn shim_rcbg_scene_set_matrix(v: u32) {
        let mut work = CameraWork([0; camera::WORK as usize]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        camera::bridge::rcbg_scene_set_matrix(
            &mut mem(),
            v,
            w,
            &mut |_, f, a| call_cdecl(f, a),
            &mut |_, f, a| call_float(f, a),
        );
    }
    /// `rcbgModelCreate(model, desc)`.
    rcbgModelCreate @ 0x0092_7be0 => extern "C" fn shim_rcbg_model_create(m: u32, desc: u32) -> u32 {
        model::bridge::rcbg_model_create(&mut mem(), m, desc, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelCreateOnePrimH(model, type, count, verts, normals)`.
    rcbgModelCreateOnePrimH @ 0x0092_7fa0 => extern "C" fn shim_rcbg_model_create_one_prim_h(m: u32, kind: u32, count: u32, verts: u32, normals: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_model_create_one_prim_h(&mut mem(), m, kind, count, verts, normals, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgModelFToS0(out, points, n, scale)`.
    rcbgModelFToS0 @ 0x0092_81d0 => extern "C" fn shim_rcbg_model_f_to_s0(out: u32, points: u32, n: i32, scale: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_model_f_to_s0(&mut mem(), out, points, n, scale, w)
    }
    /// `rcbgGuideSetModelFromIdMap(guide)`.
    rcbgGuideSetModelFromIdMap @ 0x0091_d5e0 => extern "C" fn shim_rcbg_guide_set_model_from_id_map(g: u32) {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        guide::bridge::rcbg_guide_set_model_from_id_map(&mut mem(), g, w, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiSwizzleColumns(src, dst)`.
    yiSwizzleColumns @ 0x0090_e000 => extern "C" fn shim_yi_swizzle_columns(src: u32, dst: u32) -> u32 {
        yi::bridge::yi_swizzle_columns(&mut mem(), src, dst)
    }
    /// `yiSwizzlePage(w, h, src, dst)`.
    yiSwizzlePage @ 0x0090_e2e0 => extern "C" fn shim_yi_swizzle_page(w: i32, h: i32, src: u32, dst: u32) {
        let mut work = Swizzle([0; 0x200]);
        let at = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_swizzle_page(&mut mem(), w, h, src, dst, at, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiSwizzleImage(w, h, src, dst)`.
    yiSwizzleImage @ 0x0090_e0b0 => extern "C" fn shim_yi_swizzle_image(w: i32, h: i32, src: u32, dst: u32) -> u32 {
        let mut work = Swizzle([0; 0x200]);
        let at = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_swizzle_image(&mut mem(), w, h, src, dst, at, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiMapReadMaster(map, file, cd_map, cd_file, alarm)`.
    yiMapReadMaster @ 0x0090_93b0 => extern "C" fn shim_yi_map_read_master(map: u32, file: u32, cd_map: u32, cd_file: u32, alarm: u32) -> u32 {
        let mut image = Image([0; 0x60]);
        let im = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        let mut work = Work([0; 0xa4]);
        let wk = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let mut local = Scratch([0; 0x20]);
        let lc = u32::try_from((&raw mut local).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_map_read_master(&mut mem(), map, file, cd_map, cd_file, alarm, im, wk, lc, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiMapSetup(map)`.
    yiMapSetup @ 0x0090_9680 => extern "C" fn shim_yi_map_read_host(map: u32) -> u32 {
        let mut work = Host([0; 0x1000]);
        let wk = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let mut image = Image([0; 0x60]);
        let im = u32::try_from((&raw mut image).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_map_read_host(&mut mem(), map, wk, im, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiAttachAnimDObj(obj, anim, arg)`.
    yiAttachAnimDObj @ 0x0090_8330 => extern "C" fn shim_yi_attach_anim_dobj(obj: u32, anim: u32, arg: u32) -> u32 {
        yi::bridge::yi_attach_anim_dobj(&mut mem(), obj, anim, arg, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiMallocQuiet(size)`.
    yiMallocQuiet @ 0x0090_bc10 => extern "C" fn shim_yi_malloc_quiet(size: u32) -> u32 {
        yi::bridge::yi_malloc_quiet(&mut mem(), size, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiRender_resetall(r)`.
    yiRender_resetall @ 0x0090_cd50 => extern "C" fn shim_yi_render_resetall(r: u32) {
        yi::bridge::yi_render_resetall(&mut mem(), r, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiRender_initpacket(r)`.
    yiRender_initpacket @ 0x0090_cf00 => extern "C" fn shim_yi_render_initpacket(r: u32) -> u32 {
        yi::bridge::yi_render_initpacket(&mut mem(), r)
    }
    /// `yiRender_initfog(r)`.
    yiRender_initfog @ 0x0091_3f50 => extern "C" fn shim_yi_render_initfog(r: u32) {
        yi::bridge::yi_render_initfog(&mut mem(), r);
    }
    /// `yiRotMatrixSeq(m, angles, order)`.
    yiRotMatrixSeq @ 0x0091_51c0 => extern "C" fn shim_yi_rot_matrix_seq(m: u32, angles: u32, order: u32) -> u32 {
        yi::bridge::yi_rot_matrix_seq(&mut mem(), m, angles, order, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `Yn_GuideMapSetMapNo(n)`.
    Yn_GuideMapSetMapNo @ 0x0091_9da0 => extern "C" fn shim_yn_guide_map_set_map_no(n: i32) -> u32 {
        guide::bridge::yn_guide_map_set_map_no(&mut mem(), n, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `Yn_PreBgChangeStage(n)`.
    Yn_PreBgChangeStage @ 0x0091_a0d0 => extern "C" fn shim_yn_pre_bg_change_stage(n: i32) {
        guide::bridge::yn_pre_bg_change_stage(&mut mem(), n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreAddFp(map)`.
    rcbgPreAddFp @ 0x0092_1a50 => extern "C" fn shim_rcbg_pre_add_fp(map: i32) {
        guide::bridge::rcbg_pre_add_fp(&mut mem(), map, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreGetMapNo()`.
    rcbgPreGetMapNo @ 0x0092_1ba0 => extern "C" fn shim_rcbg_pre_get_map_no() -> u32 {
        guide::bridge::rcbg_pre_get_map_no(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `Yn_SceneFindObject(kind, n)`.
    Yn_SceneFindObject @ 0x0092_1c60 => extern "C" fn shim_yn_scene_find_object(kind: u32, n: u32) -> u32 {
        guide::bridge::yn_scene_find_object(&mem(), kind, n)
    }
    /// `rcbgSceneDelObject(scene, obj)`.
    rcbgSceneDelObject @ 0x0092_0670 => extern "C" fn shim_rcbg_scene_del_object(s: u32, obj: u32) {
        scene::bridge::rcbg_scene_del_object(&mut mem(), s, obj, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxTexSetAddr(tex, image, clut)`.
    rcbgTrxTexSetAddr @ 0x0092_2210 => extern "C" fn shim_rcbg_trx_tex_set_addr(t: u32, image: u32, clut: u32) {
        model::bridge::rcbg_trx_tex_set_addr(&mut mem(), t, image, clut);
    }
    /// `rcdtListsFini(l)`.
    rcdtListsFini @ 0x0092_42f0 => extern "C" fn shim_rcdt_lists_fini(l: u32) {
        model::bridge::rcdt_lists_fini(&mut mem(), l, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxParticleAddForce(ps, force)`.
    rcfxParticleAddForce @ 0x0092_8660 => extern "C" fn shim_rcfx_particle_add_force(ps: u32, force: u32) {
        model::bridge::rcfx_particle_add_force(&mut mem(), ps, force, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxParticleFiniForces(ps)`.
    rcfxParticleFiniForces @ 0x0092_8800 => extern "C" fn shim_rcfx_particle_fini_forces(ps: u32) {
        model::bridge::rcfx_particle_fini_forces(&mut mem(), ps, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxParticleSetCnt(ps, n)`.
    rcfxParticleSetCnt @ 0x0092_89f0 => extern "C" fn shim_rcfx_particle_set_cnt(ps: u32, n: u32) {
        model::bridge::rcfx_particle_set_cnt(&mut mem(), ps, n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxSpringFini(s)`.
    rcfxSpringFini @ 0x0092_8c80 => extern "C" fn shim_rcfx_spring_fini(s: u32) {
        model::bridge::rcfx_spring_fini(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxSpringInit(s)`.
    rcfxSpringInit @ 0x0092_8ce0 => extern "C" fn shim_rcfx_spring_init(s: u32) {
        model::bridge::rcfx_spring_init(&mut mem(), s);
    }
    /// `rcfxSpringSetCnt(s, n)`.
    rcfxSpringSetCnt @ 0x0092_8d80 => extern "C" fn shim_rcfx_spring_set_cnt(s: u32, n: u32) {
        model::bridge::rcfx_spring_set_cnt(&mut mem(), s, n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxNailFini(nail)`.
    rcfxNailFini @ 0x0092_9090 => extern "C" fn shim_rcfx_nail_fini(nail: u32) {
        model::bridge::rcfx_nail_fini(&mut mem(), nail, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxNailSetCnt(nail, n)`.
    rcfxNailSetCnt @ 0x0092_9230 => extern "C" fn shim_rcfx_nail_set_cnt(nail: u32, n: u32) {
        model::bridge::rcfx_nail_set_cnt(&mut mem(), nail, n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxWindSet(wind, dir, pos, power)`.
    rcfxWindSet @ 0x0092_98e0 => extern "C" fn shim_rcfx_wind_set(wind: u32, dir: u32, pos: u32, power: u32) {
        model::bridge::rcfx_wind_set(&mut mem(), wind, dir, pos, power);
    }
    /// `rcbgPalRotate(pal)`.
    rcbgPalRotate @ 0x0092_9a10 => extern "C" fn shim_rcbg_pal_rotate(pal: u32) {
        model::bridge::rcbg_pal_rotate(&mut mem(), pal);
    }
    /// `rcbgPalShift(pal)`.
    rcbgPalShift @ 0x0092_9ac0 => extern "C" fn shim_rcbg_pal_shift(pal: u32) {
        model::bridge::rcbg_pal_shift(&mut mem(), pal);
    }
    /// `rcbgPalWhite(pal)`.
    rcbgPalWhite @ 0x0092_9ba0 => extern "C" fn shim_rcbg_pal_white(pal: u32) {
        model::bridge::rcbg_pal_white(&mut mem(), pal);
    }
    /// `rcbgTrxAllocEntries(t, format, w, h, n)`.
    rcbgTrxAllocEntries @ 0x0092_d3e0 => extern "C" fn shim_rcbg_trx_alloc_entries(t: u32, format: u32, w: u32, h: u32, n: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low halves
        let h16 = |v: u32| (v as u16).cast_signed();
        model::bridge::rcbg_trx_alloc_entries(&mut mem(), t, h16(format), h16(w), h16(h), n, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgTrxPacketMake(t, out)`.
    rcbgTrxPacketMake @ 0x0092_d5a0 => extern "C" fn shim_rcbg_trx_make_packet(t: u32, out: u32) -> u32 {
        model::bridge::rcbg_trx_make_packet(&mut mem(), t, out)
    }
    /// `rcdtTm2Fini(tm2)`.
    rcdtTm2Fini @ 0x0092_3470 => extern "C" fn shim_rcdt_tm2_fini(t: u32) {
        rcdt::bridge::rcdt_tm2_fini(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcdtTm2New()`.
    rcdtTm2New @ 0x0092_3b40 => extern "C" fn shim_rcdt_tm2_new() -> u32 {
        rcdt::bridge::rcdt_tm2_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtParseNew()`.
    rcdtParseNew @ 0x0092_bfd0 => extern "C" fn shim_rcdt_parse_new() -> u32 {
        rcdt::bridge::rcdt_parse_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtParseSkipLine(parse)`.
    rcdtParseSkipLine @ 0x0092_b970 => extern "C" fn shim_rcdt_parse_skip_line(p: u32) {
        rcdt::bridge::rcdt_parse_skip_line(&mut mem(), p);
    }
    /// `rcdtParseGetInt(parse)`.
    rcdtParseGetInt @ 0x0092_bc50 => extern "C" fn shim_rcdt_parse_get_int(p: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_int(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefTaskKill(t)`.
    rcefTaskKill @ 0x0092_ed50 => extern "C" fn shim_rcef_task_kill(t: u32) -> u32 {
        task::bridge::rcef_task_kill(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefChrInit(c)`.
    rcefChrInit @ 0x0093_0770 => extern "C" fn shim_rcef_chr_init(c: u32) -> u32 {
        task::bridge::rcef_chr_init(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefWorkSwap(w)`.
    rcefWorkSwap @ 0x0092_5530 => extern "C" fn shim_rcef_work_swap(w: u32) {
        let mut tmp = Scratch([0; 0x20]);
        let t = u32::try_from((&raw mut tmp).expose_provenance()).unwrap_or(0);
        task::bridge::rcef_work_swap(&mut mem(), w, t);
    }
    /// `rcefWorkSet(w, a, b, v)`.
    rcefWorkSet @ 0x0092_5630 => extern "C" fn shim_rcef_work_set(w: u32, a: u32, b: u32, v: u32) {
        task::bridge::rcef_work_set(&mut mem(), w, a, b, v);
    }
    /// `Yn_FpTrxTexGet(n)`.
    Yn_FpTrxTexGet @ 0x0091_9ba0 => extern "C" fn shim_yn_fp_trx_tex_get(n: u32) -> u32 {
        guide::bridge::yn_fp_trx_tex_get(&mem(), n)
    }
    /// `Yn_FpTrxTexSetAddr(tex, image, clut)`.
    Yn_FpTrxTexSetAddr @ 0x0091_9be0 => extern "C" fn shim_yn_fp_trx_tex_set_addr(t: u32, image: u32, clut: u32) {
        guide::bridge::yn_fp_trx_tex_set_addr(&mut mem(), t, image, clut);
    }
    /// `rcdtParseAdvanceLine(parse)`.
    rcdtParseAdvanceLine @ 0x0092_b9b0 => extern "C" fn shim_rcdt_parse_advance_line(p: u32) {
        rcdt::bridge::rcdt_parse_advance_line(&mut mem(), p);
    }
    /// `rcdtParseGetKeyword(parse, table)`.
    rcdtParseGetKeyword @ 0x0092_be40 => extern "C" fn shim_rcdt_parse_get_keyword(p: u32, table: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_keyword(&mut mem(), p, table, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtParseGetFloat(parse)`.
    rcdtParseGetFloat @ 0x0092_bbb0 => extern "C" fn shim_rcdt_parse_get_float(p: u32) -> f32 {
        rcdt::bridge::rcdt_parse_get_float(&mut mem(), p, &mut |_, f, a| call_float(f, a))
    }
    /// `rcdtParseGetText(parse, out)`.
    rcdtParseGetText @ 0x0092_bdd0 => extern "C" fn shim_rcdt_parse_get_text(p: u32, out: u32) -> u32 {
        rcdt::bridge::rcdt_parse_get_text(&mut mem(), p, out, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcdtTm2Delete(tm2)`.
    rcdtTm2Delete @ 0x0092_3450 => extern "C" fn shim_rcdt_tm2_delete(t: u32) -> u32 {
        rcdt::bridge::rcdt_tm2_delete(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefVecToField(v)`.
    rcefVecToField @ 0x0092_d7e0 => extern "C" fn shim_rcef_vec_to_field(v: u32) {
        vu0::bridge::rcef_vec_to_field(&mut mem(), v);
    }
    /// `rcbgScaleMatrix100(m)`.
    rcbgScaleMatrix100 @ 0x0091_ffd0 => extern "C" fn shim_rcbg_scale_matrix100(m: u32) {
        vu0::bridge::rcbg_scale_rows(&mut mem(), m, vu0::SCALE_100);
    }
    /// `rcefScaleMatrix100(m)`.
    rcefScaleMatrix100 @ 0x0092_e630 => extern "C" fn shim_rcef_scale_matrix100(m: u32) {
        vu0::bridge::rcbg_scale_rows(&mut mem(), m, vu0::SCALE_100);
    }
    /// `rcbgFPmGetElemMatrix(out, chr, elem)`.
    rcbgFPmGetElemMatrix @ 0x0092_2350 => extern "C" fn shim_rcbg_fpm_get_elem_matrix(out: u32, chr: u32, elem: u32) {
        model::bridge::rcbg_fpm_get_elem_matrix(&mut mem(), out, chr, elem, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgFPmGetMatrix(fp, out)`.
    rcbgFPmGetMatrix @ 0x0092_2290 => extern "C" fn shim_rcbg_fpm_get_matrix(fp: u32, out: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_fpm_get_matrix(&mut mem(), fp, out, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFPmRestart(id, chr, elem)`.
    rcbgFPmRestart @ 0x0092_2300 => extern "C" fn shim_rcbg_fpm_restart(id: u32, chr: u32, elem: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_fpm_restart(&mut mem(), id, chr, elem, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiScaleVector(dst, v, s, a, b, c)`.
    rcuiScaleVector @ 0x0093_0f30 => extern "C" fn shim_rcui_scale_vector(dst: u32, v: u32, s: u32, a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rcui_scale_vector(&mut mem(), dst, v, s, [a, b, c])
    }
    /// `rcefGetVf00(dst)`.
    rcefGetVf00 @ 0x0092_5020 => extern "C" fn shim_rcef_get_vf00(dst: u32) -> u32 {
        vu0::bridge::rcef_get_vf00(&mut mem(), dst)
    }
    /// `yiFieldMapRead(map, alarm)`.
    yiFieldMapRead @ 0x0090_85d0 => extern "C" fn shim_yi_field_map_read(map: u32, alarm: u32) {
        yi::bridge::yi_field_map_read(&mut mem(), map, alarm, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiBattleMapRead(map, alarm)`.
    yiBattleMapRead @ 0x0090_8370 => extern "C" fn shim_yi_battle_map_read(map: u32, alarm: u32) {
        yi::bridge::yi_battle_map_read(&mut mem(), map, alarm, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiGetMapName(out)`.
    yiGetMapName @ 0x0090_7e70 => extern "C" fn shim_yi_get_map_name(out: u32) -> u32 {
        yi::bridge::yi_get_map_name(&mut mem(), out, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiMakeBasePath()`.
    rcuiMakeBasePath @ 0x0093_7af0 => extern "C" fn shim_rcui_make_base_path() -> u32 {
        rcui::bridge::rcui_make_base_path(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiCloseWindow(&w)`.
    rcuiCloseWindow @ 0x0093_7f50 => extern "C" fn shim_rcui_close_window(at: u32) {
        rcui::bridge::rcui_close_window(&mut mem(), at, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcuiReset()`.
    rcuiReset @ 0x0093_6570 => extern "C" fn shim_rcui_reset() {
        rcui::bridge::rcui_reset(&mut mem());
    }
    /// `rcbgTrxSetRegsAlloc(t, a, b, c, d, e, f, g, n)`.
    rcbgTrxSetRegsAlloc @ 0x0092_d320 => extern "C" fn shim_rcbg_trx_set_regs_alloc(t: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, n: u32) {
        model::bridge::rcbg_trx_set_regs_alloc(&mut mem(), t, [a, b, c, d, e, f, g], n, &mut |_, fun, args| call_cdecl(fun, args));
    }
    /// `rcdtListsDelete(l)`.
    rcdtListsDelete @ 0x0092_42d0 => extern "C" fn shim_rcdt_lists_delete(l: u32) -> u32 {
        model::bridge::rcdt_lists_delete(&mut mem(), l, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxSpringNew()`.
    rcfxSpringNew @ 0x0092_8d50 => extern "C" fn shim_rcfx_spring_new() -> u32 {
        model::bridge::rcfx_spring_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcfxParticleFini(ps)`.
    rcfxParticleFini @ 0x0092_8870 => extern "C" fn shim_rcfx_particle_fini(ps: u32) {
        model::bridge::rcfx_particle_fini(&mut mem(), ps, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPSimAddDamp(psim, k)`.
    rcbgPSimAddDamp @ 0x0091_f7a0 => extern "C" fn shim_rcbg_psim_add_damp(psim: u32, k: u32) {
        model::bridge::rcbg_psim_add_damp(&mut mem(), psim, k, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPSimAddWind(psim, dir, pos, power)`.
    rcbgPSimAddWind @ 0x0091_f980 => extern "C" fn shim_rcbg_psim_add_wind(psim: u32, dir: u32, pos: u32, power: u32) {
        model::bridge::rcbg_psim_add_wind(&mut mem(), psim, dir, pos, power, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPSimAddGravity(psim, g)`.
    rcbgPSimAddGravity @ 0x0091_f7c0 => extern "C" fn shim_rcbg_psim_add_gravity(psim: u32, g: u32) {
        model::bridge::rcbg_psim_add_gravity(&mut mem(), psim, g, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPalProc(pal)`.
    rcbgPalProc @ 0x0092_9b60 => extern "C" fn shim_rcbg_pal_proc(pal: u32) {
        model::bridge::rcbg_pal_proc(&mut mem(), pal);
    }
    /// `rcbgModelSetTexReg(page)`.
    rcbgModelSetTexReg @ 0x0092_84d0 => extern "C" fn shim_rcbg_model_set_tex_reg(page: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_model_set_tex_reg(&mut mem(), page, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefRunPart(env, part, arg)`.
    rcefRunPart @ 0x0092_eec0 => extern "C" fn shim_rcef_run_part(env: u32, part: u32, arg: u32) -> u32 {
        task::bridge::rcef_run_part(&mut mem(), env, part, arg, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefObjNew()`.
    rcefObjNew @ 0x0093_0030 => extern "C" fn shim_rcef_obj_new() -> u32 {
        task::bridge::rcef_obj_new(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefTaskCtrlFini(ctrl)`.
    rcefTaskCtrlFini @ 0x0093_0260 => extern "C" fn shim_rcef_task_ctrl_fini(ctrl: u32) {
        task::bridge::rcef_task_ctrl_fini(&mut mem(), ctrl, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcefTaskCtrlProc(first)`.
    rcefTaskCtrlProc @ 0x0093_0300 => extern "C" fn shim_rcef_task_ctrl_proc(first: u32) {
        task::bridge::rcef_task_ctrl_proc(&mut mem(), first, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcefChrFini(c)`.
    rcefChrFini @ 0x0093_0710 => extern "C" fn shim_rcef_chr_fini(c: u32) {
        task::bridge::rcef_chr_fini(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `Yn_PreBgSetScrBank(roll)`.
    Yn_PreBgSetScrBank @ 0x0091_ad10 => extern "C" fn shim_yn_pre_bg_set_scr_bank(roll: u32) {
        guide::bridge::yn_pre_bg_set_scr_bank(&mut mem(), roll, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `Yn_PreBgSetScrOffset(x, y)`.
    Yn_PreBgSetScrOffset @ 0x0091_ad40 => extern "C" fn shim_yn_pre_bg_set_scr_offset(x: u32, y: u32) {
        guide::bridge::yn_pre_bg_set_scr_offset(&mut mem(), x, y, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgPreAddFpObjects()`.
    rcbgPreAddFpObjects @ 0x0092_2010 => extern "C" fn shim_rcbg_pre_add_fp_objects() {
        guide::bridge::rcbg_pre_add_fp_objects(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneDelObjects(scene, kind)`.
    rcbgSceneDelObjects @ 0x0092_06f0 => extern "C" fn shim_rcbg_scene_del_objects(s: u32, kind: u32) {
        scene::bridge::rcbg_scene_del_objects(&mut mem(), s, kind, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcefAngleWrap(a)`.
    rcefAngleWrap @ 0x0092_d810 => extern "C" fn shim_rcef_angle_wrap(a: u32) -> f32 {
        vu0::bridge::rcef_angle_wrap(&mut mem(), a, &mut |_, f, x| call_float(f, x))
    }
    /// `rcefVecToAngle(v)`.
    rcefVecToAngle @ 0x0092_ec20 => extern "C" fn shim_rcef_vec_to_angle(v: u32) -> f32 {
        vu0::bridge::rcef_vec_to_angle(&mut mem(), v, &mut |_, f, x| call_float(f, x))
    }
    /// `rcefAngleToDir(out, a)`.
    rcefAngleToDir @ 0x0092_ded0 => extern "C" fn shim_rcef_angle_to_dir(out: u32, a: u32) {
        vu0::bridge::rcef_angle_to_dir(&mut mem(), out, a, &mut |_, f, x| call_float(f, x));
    }
    /// `rcuiScaleVector_00937610(dst, v, s, a, b, c)`.
    rcuiScaleVector_00937610 @ 0x0093_7610 => extern "C" fn shim_rcui_scale_vector_00937610(dst: u32, v: u32, s: u32, a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rcui_scale_vector(&mut mem(), dst, v, s, [a, b, c])
    }
    /// `rcefMainCreateObj(main)`.
    rcefMainCreateObj @ 0x0092_5140 => extern "C" fn shim_rcef_main_create_obj(m: u32) -> u32 {
        task::bridge::rcef_main_create_obj(&mut mem(), m, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcefMainFini(main)`.
    rcefMainFini @ 0x0092_51c0 => extern "C" fn shim_rcef_main_fini(m: u32) {
        task::bridge::rcef_main_fini(&mut mem(), m, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcefWorkSetFromVf00(w, b, v)`.
    rcefWorkSetFromVf00 @ 0x0092_55d0 => extern "C" fn shim_rcef_work_set_from_vf00(w: u32, b: u32, v: u32) {
        task::bridge::rcef_work_set_from_vf00(&mut mem(), w, b, v);
    }
    /// `rcbgPalAlloc(pal, entries)`.
    rcbgPalAlloc @ 0x0092_9950 => extern "C" fn shim_rcbg_pal_alloc(pal: u32, entries: u32) {
        model::bridge::rcbg_pal_alloc(&mut mem(), pal, entries, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcfxParticleDelete(ps)`.
    rcfxParticleDelete @ 0x0092_8850 => extern "C" fn shim_rcfx_particle_delete(ps: u32) -> u32 {
        model::bridge::rcfx_particle_delete(&mut mem(), ps, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgFPm173ProcSphere(fp)`.
    rcbgFPm173ProcSphere @ 0x0092_2eb0 => extern "C" fn shim_rcbg_fpm173_proc_sphere(fp: u32) {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_fpm173_proc_sphere(&mut mem(), fp, w, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgFPmGetPartData(id, n, part, k)`.
    rcbgFPmGetPartData @ 0x0092_3190 => extern "C" fn shim_rcbg_fpm_get_part_data(id: u32, n: i32, part: u32, k: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_fpm_get_part_data(&mut mem(), id, n, part, k, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgPalLoadConfig(pal, parse)`.
    rcbgPalLoadConfig @ 0x0093_adf0 => extern "C" fn shim_rcbg_pal_load_config(pal: u32, p: u32) {
        model::bridge::rcbg_pal_load_config(&mut mem(), pal, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcbgSceneLoadTextSubConfigKeys(obj, parse)`.
    rcbgSceneLoadTextSubConfigKeys @ 0x0093_aee0 => extern "C" fn shim_rcbg_scene_load_text_sub_keys(o: u32, p: u32) {
        model::bridge::rcbg_scene_load_text_sub_keys(&mut mem(), o, p, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `yiSetBattleView(view)`.
    yiSetBattleView @ 0x0090_a140 => extern "C" fn shim_yi_set_battle_view(view: u32) {
        let mut work = Matrix([0; 0x40]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_set_battle_view(&mut mem(), view, w, &mut |_, f, a| call_cdecl(f, a), &mut |_, f, a| call_float(f, a));
    }
    /// `yiSetViewMatrices(m, view)`.
    yiSetViewMatrices @ 0x0090_a640 => extern "C" fn shim_yi_set_view_matrices(m: u32, view: u32) {
        let mut work = Matrix([0; 0x40]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_set_view_matrices(&mut mem(), m, view, w, &mut |_, f, a| call_cdecl(f, a), &mut |_, f, a| call_float(f, a));
    }
    /// `yiSetLightMatrices()`.
    yiSetLightMatrices @ 0x0090_8d00 => extern "C" fn shim_yi_set_light_matrices() -> u32 {
        let mut work = Lights([0; 0x70]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_set_light_matrices(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiImageVramAlloc(img)`.
    yiImageVramAlloc @ 0x0091_7d80 => extern "C" fn shim_yi_image_vram_alloc(img: u32) -> u32 {
        yi::bridge::yi_image_vram_alloc(&mut mem(), img)
    }
    /// `rcuiMotionStep(d)`.
    rcuiMotionStep @ 0x0093_5690 => extern "C" fn shim_rcui_motion_step(d: i32) {
        rcui::bridge::rcui_motion_step(&mut mem(), d);
    }
    /// `rcuiSetNumber(n)`.
    rcuiSetNumber @ 0x0093_7f70 => extern "C" fn shim_rcui_set_number(n: i32) {
        rcui::bridge::rcui_set_number(&mut mem(), n);
    }
    /// `rcuiGuideSaveSize(g)`.
    rcuiGuideSaveSize @ 0x0093_96a0 => extern "C" fn shim_rcui_guide_save_size(g: u32) -> u32 {
        rcui::bridge::rcui_guide_save_size(&mem(), g)
    }
    /// `rcbgDelObjects(kind)`.
    rcbgDelObjects @ 0x0092_19e0 => extern "C" fn shim_rcbg_del_objects(kind: u32) {
        guide::bridge::rcbg_del_objects(&mut mem(), kind, &mut |_, f, a| call_cdecl(f, a));
    }
    // ---- round 12 ----
    /// `KeVecAddRandom(dst, range, scaled)`.
    KeVecAddRandom @ 0x0076_6f10 => extern "C" fn shim_ke_vec_add_random(dst: u32, range: u32, scaled: u32) {
        ke::bridge::ke_vec_add_random(&mut mem(), dst, range, scaled, &mut |_, f, a| call_float(f, a));
    }
    /// `KeVecScaleRandom(dst, v, pair)`.
    KeVecScaleRandom @ 0x0076_6f70 => extern "C" fn shim_ke_vec_scale_random(dst: u32, v: u32, pair: u32) -> u32 {
        ke::bridge::ke_vec_scale_random(&mut mem(), dst, v, pair, &mut |_, f, a| call_float(f, a))
    }
    /// `pppFreeAllEnvObjects()`.
    pppFreeAllEnvObjects @ 0x0076_05d0 => extern "C" fn shim_ppp_free_all_env_objects() {
        ke::bridge::ppp_free_all_env_objects(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `pppInitPmcomEnv()`.
    pppInitPmcomEnv @ 0x0076_0610 => extern "C" fn shim_ppp_init_pmcom_env() -> u32 {
        ke::bridge::ppp_init_pmcom_env(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `KeResampleRgbaToRgb(src, sw, sh, dst, dw, dh)`.
    KeResampleRgbaToRgb @ 0x0076_0640 => extern "C" fn shim_ke_resample_rgba_to_rgb(src: u32, sw: i32, sh: i32, dst: u32, dw: i32, dh: i32) -> u32 {
        ke::bridge::ke_resample_rgba_to_rgb(&mut mem(), src, sw, sh, dst, dw, dh)
    }
    /// `KeThBP_Init(args)`.
    KeThBP_Init @ 0x0076_3030 => extern "C" fn shim_ke_th_bp_init(args: u32) -> u32 {
        ke::bridge::ke_th_bp_init(&mut mem(), args)
    }
    /// `KeVu0ScaleVecXY(dst, src, s)`.
    KeVu0ScaleVecXY @ 0x0075_fab0 => extern "C" fn shim_ke_vu0_scale_vec_xy(dst: u32, src: u32, s: u32) -> u32 {
        vu0::bridge::ke_vu0_scale_vec_xy(&mut mem(), dst, src, s)
    }
    /// `KeVu0SubVecXYZ(dst, a, b)`.
    KeVu0SubVecXYZ @ 0x0075_fcd0 => extern "C" fn shim_ke_vu0_sub_vec_xyz(dst: u32, a: u32, b: u32) -> u32 {
        vu0::bridge::ke_vu0_sub_vec_xyz(&mut mem(), dst, a, b)
    }
    /// `AbmapResetCursorCamera()`.
    AbmapResetCursorCamera @ 0x00a5_b030 => extern "C" fn shim_abmap_reset_cursor_camera() -> u32 {
        abmap::bridge::abmap_reset_cursor_camera(&mut mem())
    }
    /// `AbmapStoreSave()`.
    AbmapStoreSave @ 0x00a5_bb70 => extern "C" fn shim_abmap_store_save() -> u32 {
        abmap::bridge::abmap_store_save(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapSetupPlyTag(tag)`.
    AbmapSetupPlyTag @ 0x00a4_7c60 => extern "C" fn shim_abmap_setup_ply_tag(tag: u32) -> u32 {
        abmap::bridge::abmap_setup_ply_tag(&mut mem(), tag)
    }
    /// `AbmapScaledLevelValue(level, i, j)`.
    AbmapScaledLevelValue @ 0x00a5_cbe0 => extern "C" fn shim_abmap_scaled_level_value(level: i32, i: u32, j: u32) -> u32 {
        abmap::bridge::abmap_scaled_level_value(&mem(), level, i, j)
    }
    /// `rcuiChrMotionSetup()`.
    rcuiChrMotionSetup @ 0x0093_2470 => extern "C" fn shim_rcui_chr_motion_setup() -> u32 {
        rcui::bridge::rcui_chr_motion_setup(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgSceneLoadTextCameraCalcRef(cam)`.
    rcbgSceneLoadTextCameraCalcRef @ 0x0093_b110 => extern "C" fn shim_rcbg_scene_load_text_camera_calc_ref(cam: u32) -> u32 {
        let mut work = Matrix([0; 0x40]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        scene::bridge::rcbg_scene_load_text_camera_calc_ref(&mut mem(), cam, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiImage_dumpall(rman)`.
    yiImage_dumpall @ 0x0091_8680 => extern "C" fn shim_yi_image_dumpall(rman: u32) -> u32 {
        yi::bridge::yi_image_dumpall(rman)
    }
    /// `yiRender_addDirectBlock(render, src, n)`.
    yiRender_addDirectBlock @ 0x0090_c120 => extern "C" fn shim_yi_render_add_direct_block(r: u32, src: u32, n: u32) -> u32 {
        yi::bridge::yi_render_add_direct_block(&mut mem(), r, src, n)
    }
    /// `yiTriContainsXZ(tri, p)`.
    yiTriContainsXZ @ 0x0091_0270 => extern "C" fn shim_yi_tri_contains_xz(tri: u32, p: u32) -> u32 {
        yi::bridge::yi_tri_contains_xz(&mem(), tri, p)
    }
    /// `yiBoundsMinMax(obj, min, max)`.
    yiBoundsMinMax @ 0x0091_30f0 => extern "C" fn shim_yi_bounds_min_max(obj: u32, min: u32, max: u32) -> u32 {
        yi::bridge::yi_bounds_min_max(&mut mem(), obj, min, max)
    }
    // ---- round 13 ----
    /// `rcbgPSimFini(p)`.
    rcbgPSimFini @ 0x0091_fd80 => extern "C" fn shim_rcbg_psim_fini(p: u32) -> u32 {
        model::bridge::rcbg_psim_fini(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgPSimDelete(p)`.
    rcbgPSimDelete @ 0x0091_fd40 => extern "C" fn shim_rcbg_psim_delete(p: u32) -> u32 {
        model::bridge::rcbg_psim_delete(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiLoadSetTitle()`.
    rcuiLoadSetTitle @ 0x0093_4fb0 => extern "C" fn shim_rcui_load_set_title() -> u32 {
        let mut buf = Matrix([0; 0x40]);
        let b = u32::try_from((&raw mut buf).expose_provenance()).unwrap_or(0);
        rcui::bridge::rcui_load_set_title(&mut mem(), b, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgSceneWaveDCProc(scene)`.
    rcbgSceneWaveDCProc @ 0x0092_0580 => extern "C" fn shim_rcbg_scene_wave_dc_proc(s: u32) {
        scene::bridge::rcbg_scene_wave_dc_proc(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcefMainKickPar(main, a, b, chr, bone, p0, p1, p2, flags, force)`.
    rcefMainKickPar @ 0x0092_53a0 => extern "C" fn shim_rcef_main_kick_par(m: u32, a: u32, b: u32, chr: u32, bone: u32, p0: u32, p1: u32, p2: u32, flags: u32, force: u32) -> u32 {
        task::bridge::rcef_main_kick_par(&mut mem(), [m, a, b, chr, bone, p0, p1, p2, flags, force], &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiKeyFrameWrapAngles(v)`.
    yiKeyFrameWrapAngles @ 0x0091_5120 => extern "C" fn shim_yi_key_frame_wrap_angles(v: u32) {
        yi::bridge::yi_key_frame_wrap_angles(&mut mem(), v);
    }
    /// `yiImageTakeLastPage(img, page)`.
    yiImageTakeLastPage @ 0x0091_7e70 => extern "C" fn shim_yi_image_take_last_page(img: u32, page: u32) -> u32 {
        yi::bridge::yi_image_take_last_page(&mut mem(), img, page)
    }
    /// `yiImageRectFree(page, list, img)`.
    yiImageRectFree @ 0x0091_7490 => extern "C" fn shim_yi_image_rect_free(page: u32, list: u32, img: u32) -> u32 {
        yi::bridge::yi_image_rect_free(&mem(), page, list, img)
    }
    /// `yiVramSetup(pages, mode)`.
    yiVramSetup @ 0x0091_7f20 => extern "C" fn shim_yi_vram_setup(pages: u32, mode: u32) -> u32 {
        yi::bridge::yi_vram_setup(&mut mem(), pages, mode)
    }
    // ---- round 14 ----
    /// `rcefGetChrDir(chr)`.
    rcefGetChrDir @ 0x0092_dfd0 => extern "C" fn shim_rcef_get_chr_dir(chr: u32) -> f32 {
        let mut out = Scratch([0; 0x20]);
        let o = u32::try_from((&raw mut out).expose_provenance()).unwrap_or(0);
        vu0::bridge::rcef_get_chr_dir(&mut mem(), chr, o, &mut |_, f, a| call_cdecl(f, a), &mut |_, f, a| call_float(f, a))
    }
    /// `rcuiCollectChrHandlesByGroup()`.
    rcuiCollectChrHandlesByGroup @ 0x0093_23b0 => extern "C" fn shim_rcui_collect_chr_handles_by_group() {
        rcui::bridge::rcui_collect_chr_handles_by_group(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcuiVu0AddVector(dst, a, b)`.
    rcuiVu0AddVector @ 0x0093_7540 => extern "C" fn shim_rcui_vu0_add_vector(dst: u32, a: u32, b: u32) {
        rcui::bridge::rcui_vu0_add_vector(&mut mem(), dst, a, b);
    }
    /// `rcbgLayerAnimStep(a)`.
    rcbgLayerAnimStep @ 0x0093_0920 => extern "C" fn shim_rcbg_layer_anim_step(a: u32) -> u32 {
        scene::bridge::rcbg_layer_anim_step(&mut mem(), a)
    }
    /// `yiImageRman_serch(rman, page, img)`.
    yiImageRman_serch @ 0x0091_7b70 => extern "C" fn shim_yi_image_rman_serch(_rman: u32, page: u32, img: u32) -> u32 {
        yi::bridge::yi_image_rman_serch(&mut mem(), page, img)
    }
    /// `yiKeyFrameAdvance(r, a, now)`.
    yiKeyFrameAdvance @ 0x0091_4180 => extern "C" fn shim_yi_key_frame_advance(r: u32, a: u32, now: u32) -> u32 {
        yi::bridge::yi_key_frame_advance(&mut mem(), r, a, now, &mut |_, f, a| call_cdecl(f, a))
    }
    // ---- round 15 ----
    /// `yiHermiteVec2(t, p0, p1, p2, p3, out)`.
    yiHermiteVec2 @ 0x0091_4700 => extern "C" fn shim_yi_hermite_vec2(t: u32, p0: u32, p1: u32, p2: u32, p3: u32, out: u32) {
        yi::bridge::yi_hermite_vec2(&mut mem(), t, [p0, p1, p2, p3], out);
    }
    /// `yiHermiteVec4(t, p0, p1, p2, p3, out)`.
    yiHermiteVec4 @ 0x0091_47b0 => extern "C" fn shim_yi_hermite_vec4(t: u32, p0: u32, p1: u32, p2: u32, p3: u32, out: u32) {
        yi::bridge::yi_hermite_vec4(&mut mem(), t, [p0, p1, p2, p3], out);
    }
    /// `yiKeyFrameAdvanceLinked(r, track, now)`.
    yiKeyFrameAdvanceLinked @ 0x0091_4070 => extern "C" fn shim_yi_key_frame_advance_linked(r: u32, a: u32, now: u32) -> u32 {
        yi::bridge::yi_key_frame_advance_linked(&mut mem(), r, a, now, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiRectTriOverlapXZ(tri, m, a, b)`.
    yiRectTriOverlapXZ @ 0x0091_0070 => extern "C" fn shim_yi_rect_tri_overlap_xz(tri: u32, m: u32, a: u32, b: u32) -> u32 {
        let mut work = Work([0; 0xa4]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_rect_tri_overlap_xz(&mut mem(), tri, m, [a, b], w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `KeLnsArnd_Draw(e, flags)`.
    KeLnsArnd_Draw @ 0x0076_9340 => extern "C" fn shim_ke_lns_arnd_draw(e: u32, flags: u32) {
        let mut work = Work([0; 0xa4]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        ke::bridge::ke_lns_arnd_draw(&mut mem(), e, flags as u16, w, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `KeLnsCrn_Draw(e, flags)`.
    KeLnsCrn_Draw @ 0x0076_9d70 => extern "C" fn shim_ke_lns_crn_draw(e: u32, flags: u32) {
        let mut work = Work([0; 0xa4]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        ke::bridge::ke_lns_crn_draw(&mut mem(), e, flags as u16, w, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `KeLnsFls_Draw(e, flags)`.
    KeLnsFls_Draw @ 0x0076_a1f0 => extern "C" fn shim_ke_lns_fls_draw(e: u32, flags: u32) {
        let mut work = Work([0; 0xa4]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        #[allow(clippy::cast_possible_truncation)] // a u16 argument
        ke::bridge::ke_lns_fls_draw(&mut mem(), e, flags as u16, w, &mut |_, f, a| call_cdecl(f, a));
    }
    // ---- round 16 ----
    /// `KeMdl_Draw(e, rec, col, p4, p5, p6, mode, zmat, p9, p10)`.
    KeMdl_Draw @ 0x0076_6ff0 => extern "C" fn shim_ke_mdl_draw(e: u32, rec: u32, col: u32, p4: u32, p5: u32, p6: u32, mode: u32, zmat: u32, p9: u32, p10: u32) {
        let mut work = MdlWork([0; 0xe0]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        ke::bridge::ke_mdl_draw(&mut mem(), [e, rec, col, p4, p5, p6, mode, zmat, p9, p10], w, &mut |_, f, a| call_cdecl(f, a));
    }
    /// `rcuiVu0SubVector(dst, a, b)`.
    rcuiVu0SubVector @ 0x0093_76b0 => extern "C" fn shim_rcui_vu0_sub_vector(dst: u32, a: u32, b: u32) {
        rcui::bridge::rcui_vu0_sub_vector(&mut mem(), dst, a, b);
    }
    /// `rcuiClampVector(dst, v, lo, hi)`.
    rcuiClampVector @ 0x0093_cd20 => extern "C" fn shim_rcui_clamp_vector(dst: u32, v: u32, lo: u32, hi: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        rcui::bridge::rcui_clamp_vector(&mut mem(), dst, v, (lo, hi), w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiInvertRigidMatrix(dst, m)`.
    rcuiInvertRigidMatrix @ 0x0093_d090 => extern "C" fn shim_rcui_invert_rigid_matrix(dst: u32, m: u32) -> u32 {
        let mut work = Matrix([0; 0x40]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        rcui::bridge::rcui_invert_rigid_matrix(&mut mem(), dst, m, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcuiProjectToInt(out, m, v, raw)`.
    rcuiProjectToInt @ 0x0093_d710 => extern "C" fn shim_rcui_project_to_int(out: u32, m: u32, v: u32, raw: u32) -> f32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        rcui::bridge::rcui_project_to_int(&mut mem(), out, (m, v), raw, w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `rcbgAnimeLoadConfig(anime, parse)`.
    rcbgAnimeLoadConfig @ 0x0093_a650 => extern "C" fn shim_rcbg_anime_load_config(anime: u32, p: u32) -> u32 {
        model::bridge::rcbg_anime_load_config(&mut mem(), anime, p, &mut |_, f, a| call_cdecl(f, a))
    }
    // ---- round 17 ----
    /// `FFXSamp0TransMatrix(dst, m, t)`.
    FFXSamp0TransMatrix @ 0x0093_d830 => extern "C" fn shim_ffx_samp0_trans_matrix(dst: u32, m: u32, t: u32) -> u32 {
        rcui::bridge::ffx_samp0_trans_matrix(&mut mem(), dst, m, t)
    }
    /// `rcuiScaleVec4(dst, v, s)`.
    rcuiScaleVec4 @ 0x0093_d7d0 => extern "C" fn shim_rcui_scale_vec4(dst: u32, v: u32, s: u32) -> u32 {
        rcui::bridge::rcui_scale_vec4(&mut mem(), dst, v, s)
    }
    /// `abmap_snd_chg_panel0()`.
    abmap_snd_chg_panel0 @ 0x00a4_5910 => extern "C" fn shim_abmap_snd_chg_panel0() -> u32 {
        abmap::bridge::abmap_snd_chg_panel(&mut mem(), 0, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `abmap_snd_chg_panel1()`.
    abmap_snd_chg_panel1 @ 0x00a4_5920 => extern "C" fn shim_abmap_snd_chg_panel1() -> u32 {
        abmap::bridge::abmap_snd_chg_panel(&mut mem(), 1, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `yiSegIntersect2(a, b, c, d, out)`.
    yiSegIntersect2 @ 0x0091_cd00 => extern "C" fn shim_yi_seg_intersect2(a: u32, b: u32, c: u32, d: u32, out: u32) -> u32 {
        yi::bridge::yi_seg_intersect2(&mut mem(), [a, b, c, d], out)
    }
    // ---- round 18 ----
    /// `NkzTidusLimitTableA(i)`.
    NkzTidusLimitTableA @ 0x00a5_cb30 => extern "C" fn shim_nkz_tidus_limit_table_a(i: u32) -> u32 {
        nkz::bridge::nkz_tidus_limit_table(0, i)
    }
    /// `NkzTidusLimitTableB(i)`.
    NkzTidusLimitTableB @ 0x00a5_cb80 => extern "C" fn shim_nkz_tidus_limit_table_b(i: u32) -> u32 {
        nkz::bridge::nkz_tidus_limit_table(1, i)
    }
    /// `AbmapSetKindSizes(src)`.
    AbmapSetKindSizes @ 0x00a5_adf0 => extern "C" fn shim_abmap_set_kind_sizes(src: u32) -> u32 {
        abmap::bridge::abmap_set_kind_sizes(&mut mem(), src, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapMarkLockedEntries(pad)`.
    AbmapMarkLockedEntries @ 0x00a5_a2e0 => extern "C" fn shim_abmap_mark_locked_entries(pad: u32) -> u32 {
        abmap::bridge::abmap_mark_locked_entries(&mut mem(), pad, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapLinkCells(link)`.
    AbmapLinkCells @ 0x00a5_a760 => extern "C" fn shim_abmap_link_cells(link: u32) -> u32 {
        abmap::bridge::abmap_link_cells(&mut mem(), link)
    }
    /// `AbmapCollectNodeLinks()`.
    AbmapCollectNodeLinks @ 0x00a5_b140 => extern "C" fn shim_abmap_collect_node_links() -> u32 {
        abmap::bridge::abmap_collect_node_links(&mut mem())
    }
    /// `KeThCp_AddBp(cp, pool, a, b)`.
    KeThCp_AddBp @ 0x0076_44d0 => extern "C" fn shim_ke_th_cp_add_bp(cp: u32, pool: u32, a: u32, b: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        ke::bridge::ke_th_cp_add_bp(&mut mem(), cp, pool, (a, b), w)
    }
    // ---- round 19 ----
    /// `AbmapItemUsableAt(chr, node, item)`.
    AbmapItemUsableAt @ 0x00a4_9310 => extern "C" fn shim_abmap_item_usable_at(chr: u32, node: u32, item: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_item_usable_at(&mut mem(), [chr, node, item], w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapItemApplyAt(chr, node, item, mark)`.
    AbmapItemApplyAt @ 0x00a5_b400 => extern "C" fn shim_abmap_item_apply_at(chr: u32, node: u32, item: u32, mark: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_item_apply_at(&mut mem(), [chr, node, item, mark], w, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapItemNodeEffect(node, ctx)`.
    AbmapItemNodeEffect @ 0x00a4_9440 => extern "C" fn shim_abmap_item_node_effect(node: u32, ctx: u32) -> u32 {
        let mut tmp = Scratch([0; 0x20]);
        let t = u32::try_from((&raw mut tmp).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_item_node_effect(&mut mem(), node, ctx, t, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapItemApplyNode(node, ctx)`.
    AbmapItemApplyNode @ 0x00a5_b490 => extern "C" fn shim_abmap_item_apply_node(node: u32, ctx: u32) -> u32 {
        let mut tmp = Scratch([0; 0x20]);
        let t = u32::try_from((&raw mut tmp).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_item_apply_node(&mut mem(), node, ctx, t, &mut |_, f, a| call_cdecl(f, a))
    }
    /// `AbmapSetupKindShapes()`.
    AbmapSetupKindShapes @ 0x00a5_b230 => extern "C" fn shim_abmap_setup_kind_shapes() -> u32 {
        let mut tmp = Scratch([0; 0x20]);
        let t = u32::try_from((&raw mut tmp).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_setup_kind_shapes(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiGetFogParam(near, far, limit)`.
    yiGetFogParam @ 0x0090_8bb0 => extern "C" fn shim_yi_get_fog_param(near: u32, far: u32, limit: u32) {
        yi::bridge::yi_get_fog_param(&mut mem(), [near, far, limit], &mut |_, f, a| call_float(f, a));
    }

    /// `AbmapPadStartSlideOut(pad)`.
    AbmapPadStartSlideOut @ 0x00a4_8c20 => extern "C" fn shim_abmap_pad_start_slide_out(pad: u32) {
        abmap::bridge::abmap_pad_start_slide_out(&mut mem(), pad);
    }

    /// `AbmapPadStartSlideIn(pad)`.
    AbmapPadStartSlideIn @ 0x00a4_8f20 => extern "C" fn shim_abmap_pad_start_slide_in(pad: u32) {
        abmap::bridge::abmap_pad_start_slide_in(&mut mem(), pad);
    }

    /// `AbmapPadSlideOutStep(p)`.
    AbmapPadSlideOutStep @ 0x00a5_9800 => extern "C" fn shim_abmap_pad_slide_out_step(p: u32) {
        abmap::bridge::abmap_pad_slide_out_step(&mut mem(), p);
    }

    /// `AbmapPadSlideInStep(p)`.
    AbmapPadSlideInStep @ 0x00a5_a270 => extern "C" fn shim_abmap_pad_slide_in_step(p: u32) {
        abmap::bridge::abmap_pad_slide_in_step(&mut mem(), p);
    }

    /// `AbmapRectTweenIn(r)`.
    AbmapRectTweenIn @ 0x00a5_a080 => extern "C" fn shim_abmap_rect_tween_in(r: u32) {
        abmap::bridge::abmap_rect_tween_in(&mut mem(), r);
    }

    /// `AbmapRectTweenOut(r)`.
    AbmapRectTweenOut @ 0x00a5_a170 => extern "C" fn shim_abmap_rect_tween_out(r: u32) {
        abmap::bridge::abmap_rect_tween_out(&mut mem(), r);
    }

    /// `AbmapMaskNodeFlags(mask)`.
    AbmapMaskNodeFlags @ 0x00a4_7400 => extern "C" fn shim_abmap_mask_node_flags(mask: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        abmap::bridge::abmap_mask_node_flags(&mut mem(), mask as u8);
    }

    /// `AbmapMarkEmptyMenus()`.
    AbmapMarkEmptyMenus @ 0x00a5_aca0 => extern "C" fn shim_abmap_mark_empty_menus() -> u32 {
        abmap::bridge::abmap_mark_empty_menus(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiGetFogColor(out)`.
    yiGetFogColor @ 0x0090_8980 => extern "C" fn shim_yi_get_fog_color(out: u32) -> u32 {
        let mut tmp = Scratch([0; 0x20]);
        let t = u32::try_from((&raw mut tmp).expose_provenance()).unwrap_or(0);
        yi::bridge::yi_get_fog_color(&mut mem(), out, t, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefPostRequest(a, f, b, c, d)`.
    rcefPostRequest @ 0x0092_eba0 => extern "C" fn shim_rcef_post_request(a: u32, f: u32, b: u32, c: u32, d: u32) -> u32 {
        task::bridge::rcef_post_request(&mut mem(), [a, f, b, c, d])
    }

    /// `rcEffectTaskRequestWait(task)`.
    rcEffectTaskRequestWait @ 0x0093_04e0 => extern "C" fn shim_rc_effect_task_request_wait(t: u32) {
        task::bridge::rc_effect_task_request_wait(&mut mem(), t);
    }

    /// `rcefAnimeFini(id)`.
    rcefAnimeFini @ 0x0092_f4a0 => extern "C" fn shim_rcef_anime_fini(id: u32) -> u32 {
        task::bridge::rcef_anime_fini(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefModelFini(id)`.
    rcefModelFini @ 0x0092_f710 => extern "C" fn shim_rcef_model_fini(id: u32) -> u32 {
        task::bridge::rcef_model_fini(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiAnimDetachAll(scene, obj)`.
    yiAnimDetachAll @ 0x0091_49a0 => extern "C" fn shim_yi_anim_detach_all(scene: u32, obj: u32) -> u32 {
        yi::bridge::yi_anim_detach_all(&mut mem(), scene, obj, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Yn_DrawScene()`.
    Yn_DrawScene @ 0x0091_a240 => extern "C" fn shim_yn_draw_scene() {
        model::bridge::yn_draw_scene(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }

    /// `AbmapItemEffectAtNode(chr, node, item)`.
    AbmapItemEffectAtNode @ 0x00a4_93b0 => extern "C" fn shim_abmap_item_effect_at_node(chr: u32, node: u32, item: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_item_effect_at_node(&mut mem(), [chr, node, item], w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `AbmapEnterCommandMenu()`.
    AbmapEnterCommandMenu @ 0x00a5_9950 => extern "C" fn shim_abmap_enter_command_menu() -> u32 {
        abmap::bridge::abmap_enter_command_menu(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `AbmapEnterItemMenu()`.
    AbmapEnterItemMenu @ 0x00a5_98a0 => extern "C" fn shim_abmap_enter_item_menu() -> u32 {
        abmap::bridge::abmap_enter_item_menu(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiDisposeAnimDObj(obj, kind)`.
    yiDisposeAnimDObj @ 0x0090_84b0 => extern "C" fn shim_yi_dispose_anim_dobj(obj: u32, kind: u32) -> u32 {
        yi::bridge::yi_dispose_anim_dobj(&mut mem(), obj, kind, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefObjFini(obj)`.
    rcefObjFini @ 0x0092_fee0 => extern "C" fn shim_rcef_obj_fini(o: u32) -> u32 {
        task::bridge::rcef_obj_fini(&mut mem(), o, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `AbmapFillCommandMenu(pad, kind)`.
    AbmapFillCommandMenu @ 0x00a4_59e0 => extern "C" fn shim_abmap_fill_command_menu(pad: u32, kind: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_fill_command_menu(&mut mem(), pad, kind, w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Yn_GuideEdgeFade(p, a, b, rect, out)`.
    Yn_GuideEdgeFade @ 0x0091_f530 => extern "C" fn shim_yn_guide_edge_fade(p: u32, a: u32, b: u32, rect: u32, out: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        guide::bridge::yn_guide_edge_fade(&mut mem(), [p, a, b, rect, out], w)
    }

    /// `rcbgSceneSetChadist(scene)`.
    rcbgSceneSetChadist @ 0x0092_0790 => extern "C" fn shim_rcbg_scene_set_chadist(scene: u32) -> u32 {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::rcbg_scene_set_chadist(&mut mem(), scene, w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcEffectTaskSoundCueWait(task)`.
    rcEffectTaskSoundCueWait @ 0x0093_0500 => extern "C" fn shim_rc_effect_task_sound_cue_wait(t: u32) {
        task::bridge::rc_effect_task_sound_cue_wait(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `rcEffectTaskMotionWait(task)`.
    rcEffectTaskMotionWait @ 0x0093_0430 => extern "C" fn shim_rc_effect_task_motion_wait(t: u32) {
        task::bridge::rc_effect_task_motion_wait(&mut mem(), t, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `rcefSetChrDirTarget(chr, a, b)`.
    rcefSetChrDirTarget @ 0x0092_e750 => extern "C" fn shim_rcef_set_chr_dir_target(chr: u32, a: u32, b: u32) -> u32 {
        task::bridge::rcef_set_chr_dir_target(&mut mem(), [chr, a, b], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefSetChrDirTarget2(chr, a, b, c)`.
    rcefSetChrDirTarget2 @ 0x0092_e960 => extern "C" fn shim_rcef_set_chr_dir_target2(chr: u32, a: u32, b: u32, c: u32) -> u32 {
        task::bridge::rcef_set_chr_dir_target2(&mut mem(), [chr, a, b, c], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefWeaponSetHide(chr, show)`.
    rcefWeaponSetHide @ 0x0092_eb70 => extern "C" fn shim_rcef_weapon_set_hide(chr: u32, show: u32) -> u32 {
        task::bridge::rcef_weapon_set_hide(&mut mem(), chr, show, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefPlaySE(id)`.
    rcefPlaySE @ 0x0092_e680 => extern "C" fn shim_rcef_play_se(id: u32) -> u32 {
        task::bridge::rcef_play_se(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefMainCreateTask(kind, seq)`.
    rcefMainCreateTask @ 0x0092_5170 => extern "C" fn shim_rcef_main_create_task(kind: u32, seq: u32) -> u32 {
        task::bridge::rcef_main_create_task(&mut mem(), kind, seq, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefChrDelete(obj)`.
    rcefChrDelete @ 0x0093_0600 => extern "C" fn shim_rcef_chr_delete(o: u32) -> u32 {
        task::bridge::rcef_chr_delete(&mut mem(), o, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Yn_SetChadist()`.
    Yn_SetChadist @ 0x0091_a250 => extern "C" fn shim_yn_set_chadist() {
        model::bridge::yn_set_chadist(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }

    /// `Yn_PreBgLayerDispOff(n)`.
    Yn_PreBgLayerDispOff @ 0x0091_a7c0 => extern "C" fn shim_yn_pre_bg_layer_disp_off(n: u32) -> u32 {
        model::bridge::yn_pre_bg_layer_disp_off(&mut mem(), n, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Yn_PreBgLayerDispOn(n)`.
    Yn_PreBgLayerDispOn @ 0x0091_a830 => extern "C" fn shim_yn_pre_bg_layer_disp_on(n: u32) -> u32 {
        model::bridge::yn_pre_bg_layer_disp_on(&mut mem(), n, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcfxSpringDelete(spring)`.
    rcfxSpringDelete @ 0x0092_8c60 => extern "C" fn shim_rcfx_spring_delete(p: u32) -> u32 {
        model::bridge::rcfx_spring_delete(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcfxNailDelete(nail)`.
    rcfxNailDelete @ 0x0092_9070 => extern "C" fn shim_rcfx_nail_delete(p: u32) -> u32 {
        model::bridge::rcfx_nail_delete(&mut mem(), p, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Yn_FPm173End()`.
    Yn_FPm173End @ 0x0091_9ab0 => extern "C" fn shim_yn_fpm173_end() -> u32 {
        model::bridge::yn_fpm173_end(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcbgTrxPairSetNo(p, n)`.
    rcbgTrxPairSetNo @ 0x0092_21e0 => extern "C" fn shim_rcbg_trx_pair_set_no(p: u32, n: u32) -> u32 {
        model::bridge::rcbg_trx_pair_set_no(&mut mem(), p, n, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainStr(pc, v)`.
    NkzAeonGainStr @ 0x00a5_c660 => extern "C" fn shim_nkz_aeon_gain_str(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainDef(pc, v)`.
    NkzAeonGainDef @ 0x00a5_c740 => extern "C" fn shim_nkz_aeon_gain_def(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainMag(pc, v)`.
    NkzAeonGainMag @ 0x00a5_c910 => extern "C" fn shim_nkz_aeon_gain_mag(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainMDef(pc, v)`.
    NkzAeonGainMDef @ 0x00a5_c980 => extern "C" fn shim_nkz_aeon_gain_mdef(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainAgi(pc, v)`.
    NkzAeonGainAgi @ 0x00a5_c9f0 => extern "C" fn shim_nkz_aeon_gain_agi(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainLuck(pc, v)`.
    NkzAeonGainLuck @ 0x00a5_c860 => extern "C" fn shim_nkz_aeon_gain_luck(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainEva(pc, v)`.
    NkzAeonGainEva @ 0x00a5_c6d0 => extern "C" fn shim_nkz_aeon_gain_eva(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainAcc(pc, v)`.
    NkzAeonGainAcc @ 0x00a5_c7f0 => extern "C" fn shim_nkz_aeon_gain_acc(pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_half(&mut mem(), pc, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `NkzAeonGainHp(pc, v)`.
    NkzAeonGainHp @ 0x00a5_c7b0 => extern "C" fn shim_nkz_aeon_gain_hp(_pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_hp(&mem(), v)
    }

    /// `NkzAeonGainMp(pc, v)`.
    NkzAeonGainMp @ 0x00a5_c8d0 => extern "C" fn shim_nkz_aeon_gain_mp(_pc: u32, v: i32) -> u32 {
        nkz::bridge::nkz_aeon_gain_mp(&mem(), v)
    }

    /// `rcefSetVibrate(id, arg)`.
    rcefSetVibrate @ 0x0092_ec60 => extern "C" fn shim_rcef_set_vibrate(id: u32, arg: u32) -> u32 {
        task::bridge::rcef_set_vibrate(&mut mem(), id, arg, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefStreamCue(id, arg)`.
    rcefStreamCue @ 0x0092_ebf0 => extern "C" fn shim_rcef_stream_cue(id: u32, arg: u32) -> u32 {
        task::bridge::rcef_stream_cue(&mut mem(), id, arg, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefPlayStream(id)`.
    rcefPlayStream @ 0x0092_e6b0 => extern "C" fn shim_rcef_play_stream(id: u32) -> u32 {
        task::bridge::rcef_play_stream(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefObjAnimeStart(env, obj, id)`.
    rcefObjAnimeStart @ 0x0092_5590 => extern "C" fn shim_rcef_obj_anime_start(env: u32, o: u32, id: u32) -> u32 {
        task::bridge::rcef_obj_anime_start(&mut mem(), [env, o, id], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefObjSetupDraw(env, obj, id)`.
    rcefObjSetupDraw @ 0x0092_5690 => extern "C" fn shim_rcef_obj_setup_draw(env: u32, o: u32, id: u32) -> u32 {
        task::bridge::rcef_obj_setup_draw(&mut mem(), [env, o, id], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcuiScaleVector_copy_00756600(dst, v, s, a, b, c)`.
    rcuiScaleVector_copy_00756600 @ 0x0075_6600 => extern "C" fn shim_rcui_scale_vector_copy(dst: u32, v: u32, s: u32, a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rcui_scale_vector(&mut mem(), dst, v, s, [a, b, c])
    }

    /// `rcVec4fToFixed_copy_007e6cc0(out, v)`.
    rcVec4fToFixed_copy_007e6cc0 @ 0x007e_6cc0 => extern "C" fn shim_rc_vec4f_to_fixed_copy(out: u32, v: u32) -> u32 {
        model::bridge::rc_vec4f_to_fixed(&mut mem(), out, v)
    }

    /// `yiSetMIMeParamDObj(obj, p)`.
    yiSetMIMeParamDObj @ 0x0090_a4d0 => extern "C" fn shim_yi_set_mime_param_dobj(obj: u32, p: u32) -> u32 {
        yi::bridge::yi_set_mime_param_dobj(&mut mem(), obj, p, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiGetColorOffsetInt(out)`.
    yiGetColorOffsetInt @ 0x0090_8660 => extern "C" fn shim_yi_get_color_offset_int(out: u32) -> u32 {
        yi::bridge::yi_get_color_offset_int(&mut mem(), out)
    }

    /// `rcefLerp(a, b, t, mode)` (a float in `ST0`).
    rcefLerp @ 0x0092_e580 => extern "C" fn shim_rcef_lerp(a: u32, b: u32, t: u32, mode: u32) -> f32 {
        task::bridge::rcef_lerp(&mem(), [a, b, t, mode])
    }

    /// `rcefColorSet(r, g, b, a, v)`.
    rcefColorSet @ 0x0092_e9c0 => extern "C" fn shim_rcef_color_set(r: u32, g: u32, b: u32, a: u32, v: u32) {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        task::bridge::rcef_color_set(&mut mem(), [r, g, b, a, v], w);
    }

    /// `rcefColorFade(r0, g0, b0, a0, r1, g1, b1, a1, v)`.
    rcefColorFade @ 0x0092_ea40 => extern "C" fn shim_rcef_color_fade(r0: u32, g0: u32, b0: u32, a0: u32, r1: u32, g1: u32, b1: u32, a1: u32, v: u32) {
        let mut work = Scratch([0; 0x20]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        task::bridge::rcef_color_fade(&mut mem(), [r0, g0, b0, a0, r1, g1, b1, a1, v], w);
    }

    /// `rcefScreenShake(e)`.
    rcefScreenShake @ 0x0092_4df0 => extern "C" fn shim_rcef_screen_shake(e: u32) {
        task::bridge::rcef_screen_shake(&mut mem(), e, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `rcbgSceneWaveC0Update(scene)`.
    rcbgSceneWaveC0Update @ 0x0092_10e0 => extern "C" fn shim_rcbg_scene_wave_c0_update(s: u32) {
        model::bridge::rcbg_scene_wave_c0_update(&mut mem(), s, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `AbmapStartPart(part, n, x, y, z, a, b, c, d, e, f)`.
    AbmapStartPart @ 0x00a5_bad0 => extern "C" fn shim_abmap_start_part(part: u32, n: u32, x: u32, y: u32, z: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        abmap::bridge::abmap_start_part(&mut mem(), [part, n, x, y, z, a, b, c, d, e, f], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `loadPhyreFieldMap()`.
    loadPhyreFieldMap @ 0x0090_8000 => extern "C" fn shim_load_phyre_field_map() {
        yi::bridge::load_phyre_field_map(&mut mem(), &mut |_, f, a| call_cdecl(f, a));
    }

    /// `AbmapStartWarp(chr, target)`.
    AbmapStartWarp @ 0x00a4_8a80 => extern "C" fn shim_abmap_start_warp(chr: u32, target: u32) -> u32 {
        abmap::bridge::abmap_start_warp(&mut mem(), chr, target, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcAngle2D(x0, y0, x1, y1)` (a float in `ST0`).
    rcAngle2D @ 0x007e_6d80 => extern "C" fn shim_rc_angle_2d(x0: u32, y0: u32, x1: u32, y1: u32) -> f32 {
        geom::bridge::rc_angle_2d(&mut mem(), [x0, y0, x1, y1], |_, f, a| call_float(f, a))
    }

    /// `abmap_set_chr_pos(chr, target)`.
    abmap_set_chr_pos @ 0x00a4_58f0 => extern "C" fn shim_abmap_set_chr_pos(chr: u32, target: u32) -> u32 {
        abmap::bridge::abmap_set_chr_pos(&mut mem(), chr, target, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcefBlurUpdate(e)`.
    rcefBlurUpdate @ 0x0092_4bf0 => extern "C" fn shim_rcef_blur_update(e: u32) {
        task::bridge::rcef_blur_update(&mut mem(), e, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `rcfxNailUpdate(fx, ctx)`.
    rcfxNailUpdate @ 0x0092_9190 => extern "C" fn shim_rcfx_nail_update(fx: u32, ctx: u32) -> u32 {
        model::bridge::rcfx_nail_update(&mut mem(), fx, ctx)
    }

    /// `AbmapCheckAchievements()`.
    AbmapCheckAchievements @ 0x00a4_7210 => extern "C" fn shim_abmap_check_achievements() {
        #[repr(C, align(16))]
        struct Locals([u8; 0x490]);
        let mut work = Locals([0; 0x490]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        abmap::bridge::abmap_check_achievements(&mut mem(), w, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `AbmapSpecialEffect(kind, node, row, chr)`.
    AbmapSpecialEffect @ 0x00a5_d120 => extern "C" fn shim_abmap_special_effect(kind: u32, node: u32, row: u32, chr: u32) -> u32 {
        abmap::bridge::abmap_special_effect(&mut mem(), [kind, node, row, chr], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `AbmapCalcBounds()`.
    AbmapCalcBounds @ 0x00a5_ae40 => extern "C" fn shim_abmap_calc_bounds() -> u32 {
        abmap::bridge::abmap_calc_bounds(&mut mem())
    }

    /// `Yn_SeaFpFade(fade)`.
    Yn_SeaFpFade @ 0x0092_2970 => extern "C" fn shim_yn_sea_fp_fade(fade: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0x50]);
        let mut work = Locals([0; 0x50]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        model::bridge::yn_sea_fp_fade(&mut mem(), fade, w, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `KeVu0ItrsBallLine(q)`.
    KeVu0ItrsBallLine @ 0x0075_eed0 => extern "C" fn shim_ke_vu0_itrs_ball_line(q: u32) -> u32 {
        ke::bridge::ke_vu0_itrs_ball_line(&mut mem(), q)
    }

    /// `KeBornPtCmlp(pc, pd, prog, out)`.
    KeBornPtCmlp @ 0x0075_9400 => extern "C" fn shim_ke_born_pt_cmlp(pc: u32, pd: u32, prog: u32, out: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0x30]);
        let mut work = Locals([0; 0x30]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        ke::bridge::ke_born_pt_cmlp(&mut mem(), [pc, pd, prog, out], w, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `KeLnsLp_Update(lp)`.
    KeLnsLp_Update @ 0x0076_a690 => extern "C" fn shim_ke_lns_lp_update(lp: u32) {
        #[repr(C, align(16))]
        struct Locals([u8; 0xc0]);
        let mut work = Locals([0; 0xc0]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        ke::bridge::ke_lns_lp_update(&mut mem(), lp, w, &mut |_, f, a| call_cdecl(f, a), |_, f, a| call_float(f, a));
    }

    /// `yiSetBackColor(c)`.
    yiSetBackColor @ 0x0090_a040 => extern "C" fn shim_yi_set_back_color(c: u32) {
        yi::bridge::yi_set_back_color(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `yiSetFogColor(c)`.
    yiSetFogColor @ 0x0090_a2c0 => extern "C" fn shim_yi_set_fog_color(c: u32) {
        yi::bridge::yi_set_fog_color(&mut mem(), c, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `yiSetFogParam(near, far, limit)`.
    yiSetFogParam @ 0x0090_a350 => extern "C" fn shim_yi_set_fog_param(near: u32, far: u32, limit: u32) {
        yi::bridge::yi_set_fog_param(&mut mem(), [near, far, limit], &mut |_, f, a| call_cdecl(f, a));
    }

    /// `yiDisableHamonEff()`.
    yiDisableHamonEff @ 0x0090_8450 => extern "C" fn shim_yi_disable_hamon_eff() -> u32 {
        yi::bridge::yi_disable_hamon_eff(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    // ---- field round 33: the Sqs IOP calls and SE wrappers ----

    /// `Sqs_CallRpc(cmd)`.
    Sqs_CallRpc @ 0x0081_cbd0 => extern "C" fn shim_sqs_call_rpc(cmd: u32) -> u32 {
        nkz::bridge::sqs_call_rpc(&mut mem(), cmd, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Sqs_CallRpc2(cmd, _, end, arg)`.
    Sqs_CallRpc2 @ 0x0081_cc20 => extern "C" fn shim_sqs_call_rpc2(cmd: u32, _b: u32, end: u32, arg: u32) -> u32 {
        nkz::bridge::sqs_call_rpc2(&mut mem(), cmd, end, arg, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsIopCommandSend()`.
    SqsIopCommandSend @ 0x0081_d5f0 => extern "C" fn shim_sqs_iop_command_send() -> u32 {
        nkz::bridge::sqs_iop_command_send(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSifDmaState(_, wait)`.
    SqsSifDmaState @ 0x0081_eef0 => extern "C" fn shim_sqs_sif_dma_state(_a: u32, wait: u32) -> u32 {
        nkz::bridge::sqs_sif_dma_state(&mut mem(), wait, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSePlay(_, id, pan, volume)`.
    SqsSePlay @ 0x0081_e4e0 => extern "C" fn shim_sqs_se_play(_k: u32, id: u32, pan: u32, volume: u32) -> u32 {
        nkz::bridge::sqs_se_play(&mut mem(), nkz::I_SFX_START, [id, pan, volume], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeForcePlay(_, id, pan, volume)`.
    SqsSeForcePlay @ 0x0081_e410 => extern "C" fn shim_sqs_se_force_play(_k: u32, id: u32, pan: u32, volume: u32) -> u32 {
        nkz::bridge::sqs_se_play(&mut mem(), nkz::I_SFX_FORCE_START, [id, pan, volume], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeStop(_, id)`.
    SqsSeStop @ 0x0081_e7c0 => extern "C" fn shim_sqs_se_stop(_k: u32, id: u32) -> u32 {
        nkz::bridge::sqs_se_stop(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSePanDirect(_, id, pan)`.
    SqsSePanDirect @ 0x0081_e460 => extern "C" fn shim_sqs_se_pan_direct(_k: u32, id: u32, pan: u32) -> u32 {
        nkz::bridge::sqs_se_direct(&mut mem(), nkz::I_SFX_SET_PAN, id, pan, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeVolumeDirect(_, id, volume)`.
    SqsSeVolumeDirect @ 0x0081_e820 => extern "C" fn shim_sqs_se_volume_direct(_k: u32, id: u32, v: u32) -> u32 {
        nkz::bridge::sqs_se_direct(&mut mem(), nkz::I_SFX_SET_VOLUME, id, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSePanTranslate(_, id, frames, pan)`.
    SqsSePanTranslate @ 0x0081_e480 => extern "C" fn shim_sqs_se_pan_translate(_k: u32, id: u32, frames: u32, pan: u32) -> u32 {
        nkz::bridge::sqs_se_translate(&mut mem(), nkz::I_SFX_SET_PAN_TRANSLATE, [id, frames, pan], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeVolumeTranslate(_, id, frames, volume)`.
    SqsSeVolumeTranslate @ 0x0081_e860 => extern "C" fn shim_sqs_se_volume_translate(_k: u32, id: u32, frames: u32, v: u32) -> u32 {
        nkz::bridge::sqs_se_translate(&mut mem(), nkz::I_SFX_SET_VOLUME_TRANSLATE, [id, frames, v], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeVolumeFadeOut(_, id, frames)`.
    SqsSeVolumeFadeOut @ 0x0081_e840 => extern "C" fn shim_sqs_se_volume_fade_out(_k: u32, id: u32, frames: u32) -> u32 {
        nkz::bridge::sqs_se_volume_fade_out(&mut mem(), id, frames, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeBlockPlaying(id)`.
    SqsSeBlockPlaying @ 0x0081_e3e0 => extern "C" fn shim_sqs_se_block_playing(id: u32) -> u32 {
        nkz::bridge::sqs_se_playing(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeSeparatePlaying(id)`.
    SqsSeSeparatePlaying @ 0x0081_e7a0 => extern "C" fn shim_sqs_se_separate_playing(id: u32) -> u32 {
        nkz::bridge::sqs_se_playing(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeSeparateClear(id)`.
    SqsSeSeparateClear @ 0x0081_e5d0 => extern "C" fn shim_sqs_se_separate_clear(id: u32) -> u32 {
        nkz::bridge::sqs_se_separate_clear(&mut mem(), id, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeSeparateMaskClear(m0..m7)`.
    SqsSeSeparateMaskClear @ 0x0081_e750 => extern "C" fn shim_sqs_se_separate_mask_clear(m0: u32, m1: u32, m2: u32, m3: u32, m4: u32, m5: u32, m6: u32, m7: u32) -> u32 {
        nkz::bridge::sqs_se_separate_mask_clear(&mut mem(), [m0, m1, m2, m3, m4, m5, m6, m7], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsBattleWaveDataSend(wave)`.
    SqsBattleWaveDataSend @ 0x0081_cf90 => extern "C" fn shim_sqs_battle_wave_data_send(wave: u32) -> u32 {
        nkz::bridge::sqs_battle_wave_data_send(&mut mem(), wave, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsBattleStreamingStart(_, a, b)`.
    SqsBattleStreamingStart @ 0x0081_cf40 => extern "C" fn shim_sqs_battle_streaming_start(_k: u32, a: u32, b: u32) -> u32 {
        nkz::bridge::sqs_battle_streaming_start(&mut mem(), a, b, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsCmdSetC4C6(ctx, cmd)`.
    SqsCmdSetC4C6 @ 0x0081_c260 => extern "C" fn shim_sqs_cmd_set_c4_c6(ctx: u32, cmd: u32) -> u32 {
        nkz::bridge::sqs_cmd_set_c4_c6(&mut mem(), ctx, cmd)
    }

    /// `Sqs_DecodeAdpcm(src, dst)`.
    Sqs_DecodeAdpcm @ 0x0081_cc70 => extern "C" fn shim_sqs_decode_adpcm(src: u32, dst: u32) -> u32 {
        nkz::bridge::sqs_decode_adpcm(&mut mem(), src, dst)
    }

    /// `yiPolyBufUnpack(buf, src, code)`.
    yiPolyBufUnpack @ 0x0091_1620 => extern "C" fn shim_yi_poly_buf_unpack(buf: u32, src: u32, code: u32) -> u32 {
        yi::bridge::yi_poly_buf_unpack(&mut mem(), buf, src, code)
    }

    /// `rcPrimLine(x0, y0, x1, y1, colour, ui)`: the vertices in a local
    /// buffer, as the original's.
    rcPrimLine @ 0x0092_6c00 => extern "C" fn shim_rc_prim_line(x0: u32, y0: u32, x1: u32, y1: u32, colour: u32, ui: u32) -> u32 {
        #[repr(C, align(16))]
        struct Verts([u32; 8]);
        let mut v = Verts([0; 8]);
        let w = u32::try_from((&raw mut v).expose_provenance()).unwrap_or(0);
        rcui::bridge::rc_prim_line(&mut mem(), [x0, y0, x1, y1, colour, ui], w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcPrimCircleLine(x, y, r, colour, ui)`: the 65 vertices in a local
    /// buffer, as the original's.
    rcPrimCircleLine @ 0x0092_5ea0 => extern "C" fn shim_rc_prim_circle_line(x: u32, y: u32, r: u32, colour: u32, ui: u32) -> u32 {
        #[repr(C, align(16))]
        struct Verts([u32; 65 * 4]);
        let mut v = Verts([0; 65 * 4]);
        let w = u32::try_from((&raw mut v).expose_provenance()).unwrap_or(0);
        rcui::bridge::rc_prim_circle_line(&mut mem(), [x, y, r, colour, ui], w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcPrimTri(x, y, r, angle, colour, ui)`: the vertices in a local
    /// buffer, as the original's.
    rcPrimTri @ 0x0092_70c0 => extern "C" fn shim_rc_prim_tri(x: u32, y: u32, r: u32, angle: u32, colour: u32, ui: u32) -> u32 {
        #[repr(C, align(16))]
        struct Verts([u32; 12]);
        let mut v = Verts([0; 12]);
        let w = u32::try_from((&raw mut v).expose_provenance()).unwrap_or(0);
        rcui::bridge::rc_prim_tri(&mut mem(), [x, y, r, angle, colour, ui], w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `InitVFXKeThRenderData(a, b, c)`: the arrays and names in a local
    /// buffer, as the original's.
    InitVFXKeThRenderData @ 0x0076_58b0 => extern "C" fn shim_init_vfx_ke_th_render_data(a: u32, b: u32, c: u32) -> u32 {
        #[repr(C, align(16))]
        struct Work([u8; ke::bridge::KE_TH_WORK as usize]);
        let mut v = Work([0; ke::bridge::KE_TH_WORK as usize]);
        let w = u32::try_from((&raw mut v).expose_provenance()).unwrap_or(0);
        ke::bridge::init_vfx_ke_th_render_data(&mut mem(), [a, b, c], w, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `pppDrawCrosshair()`.
    pppDrawCrosshair @ 0x0076_06f0 => extern "C" fn shim_ppp_draw_crosshair() -> u32 {
        ke::bridge::ppp_draw_crosshair(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcbgFPm173ProcKonjyo(obj)`.
    rcbgFPm173ProcKonjyo @ 0x0092_2e60 => extern "C" fn shim_rcbg_fpm173_proc_konjyo(obj: u32) -> u32 {
        model::bridge::rcbg_fpm173_proc_konjyo(&mut mem(), obj, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcbgFPm173ProcJect(obj)`.
    rcbgFPm173ProcJect @ 0x0092_2c50 => extern "C" fn shim_rcbg_fpm173_proc_ject(obj: u32) -> u32 {
        model::bridge::rcbg_fpm173_proc_ject(&mut mem(), obj, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `Sg_IsSceneLoading()`.
    Sg_IsSceneLoading @ 0x0081_fcd0 => extern "C" fn shim_sg_is_scene_loading() -> u32 {
        yi::bridge::sg_is_scene_loading(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `rcVu0IntToFloat(a, b)`.
    rcVu0IntToFloat @ 0x0092_7ba0 => extern "C" fn shim_rc_vu0_int_to_float(a: u32, b: u32) -> u32 {
        vu0::bridge::rc_vu0_int_to_float(&mut mem(), a, b, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsOefSetPacketAddrRange(oef, lo, hi, v)`: void, its caller ignores
    /// `eax`.
    SqsOefSetPacketAddrRange @ 0x0081_ca20 => extern "C" fn shim_sqs_oef_set_packet_addr_range(oef: u32, lo: u32, hi: u32, v: u32) {
        nkz::bridge::sqs_oef_set_packet_addr_range(&mut mem(), [oef, lo, hi, v], &mut |_, f, a| call_cdecl(f, a));
    }

    /// `AbmapDigitRight(a, x, y, d)`.
    AbmapDigitRight @ 0x00a5_9630 => extern "C" fn shim_abmap_digit_right(a: u32, x: u32, y: u32, d: u32) -> u32 {
        abmap::bridge::abmap_digit_right(&mut mem(), [a, x, y, d], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `AbmapHelpLineDraw(a, b, c)`.
    AbmapHelpLineDraw @ 0x00a5_94c0 => extern "C" fn shim_abmap_help_line_draw(a: u32, b: u32, c: u32) -> u32 {
        abmap::bridge::abmap_help_line_draw(&mut mem(), [a, b, c], &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SndStandbyShoutStream(data)`.
    SndStandbyShoutStream @ 0x0081_e3b0 => extern "C" fn shim_snd_standby_shout_stream(data: u32) -> u32 {
        nkz::bridge::snd_standby_shout_stream(&mut mem(), data, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSeSeparateDataSend(data, size)`.
    SqsSeSeparateDataSend @ 0x0081_e600 => extern "C" fn shim_sqs_se_separate_data_send(data: u32, size: u32) -> u32 {
        nkz::bridge::sqs_se_separate_data_send(&mut mem(), data, size, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSetCycleMode(a)`.
    SqsSetCycleMode @ 0x0081_e900 => extern "C" fn shim_sqs_set_cycle_mode(a: u32) -> u32 {
        nkz::bridge::sqs_cmd_arg(&mut mem(), 4, a, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSetReverbPlaySwitch(a)`.
    SqsSetReverbPlaySwitch @ 0x0081_ec20 => extern "C" fn shim_sqs_set_reverb_play_switch(a: u32) -> u32 {
        nkz::bridge::sqs_cmd_arg(&mut mem(), 0xc, a, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSetStraightStreamAreaSize(a)`.
    SqsSetStraightStreamAreaSize @ 0x0081_ef50 => extern "C" fn shim_sqs_set_straight_stream_area_size(a: u32) -> u32 {
        nkz::bridge::sqs_cmd_arg(&mut mem(), 5, a, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsAllVoiceTerminate()`.
    SqsAllVoiceTerminate @ 0x0081_ce70 => extern "C" fn shim_sqs_all_voice_terminate() -> u32 {
        nkz::bridge::sqs_all_voice_terminate(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsMusicDataRead(a)`.
    SqsMusicDataRead @ 0x0081_d820 => extern "C" fn shim_sqs_music_data_read(a: u32) -> u32 {
        nkz::bridge::sqs_music_data_read(&mut mem(), a, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSetPlayMode(mode)`.
    SqsSetPlayMode @ 0x0081_ea00 => extern "C" fn shim_sqs_set_play_mode(mode: u32) -> u32 {
        nkz::bridge::sqs_set_play_mode(&mut mem(), mode, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsShoutStreamingStart(a)`.
    SqsShoutStreamingStart @ 0x0081_ee20 => extern "C" fn shim_sqs_shout_streaming_start(a: u32) -> u32 {
        nkz::bridge::sqs_shout_streaming_start(&mut mem(), a, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsStraightStreamingStart()`.
    SqsStraightStreamingStart @ 0x0081_f6c0 => extern "C" fn shim_sqs_straight_streaming_start() -> u32 {
        nkz::bridge::sqs_straight_streaming_start(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsStraightStreamingStop()`.
    SqsStraightStreamingStop @ 0x0081_f770 => extern "C" fn shim_sqs_straight_streaming_stop() -> u32 {
        nkz::bridge::sqs_straight_streaming_stop(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsStraightStreamingVolumeDirect(v)`.
    SqsStraightStreamingVolumeDirect @ 0x0081_f800 => extern "C" fn shim_sqs_straight_streaming_volume_direct(v: u32) -> u32 {
        nkz::bridge::sqs_straight_streaming_volume_direct(&mut mem(), v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsStraightStreamingVolumeTranslate(frames, v)`.
    SqsStraightStreamingVolumeTranslate @ 0x0081_f880 => extern "C" fn shim_sqs_straight_streaming_volume_translate(frames: u32, v: u32) -> u32 {
        nkz::bridge::sqs_straight_streaming_volume_translate(&mut mem(), frames, v, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsInitWaveInfo(src, _, a)`: void, the thunk keeps the caller's
    /// `eax` as the original's early return does (no caller reads it).
    SqsInitWaveInfo @ 0x0081_d3f0 => extern "C" fn shim_sqs_init_wave_info(src: u32, _b: u32, a: u32) {
        nkz::bridge::sqs_init_wave_info(&mut mem(), src, a, &mut |_, f, a| call_cdecl(f, a));
    }

    /// `rcVu0Add(a, b, c)`.
    rcVu0Add @ 0x0091_f650 => extern "C" fn shim_rc_vu0_add(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Add_00924770(a, b, c)`.
    rcVu0Add_00924770 @ 0x0092_4770 => extern "C" fn shim_rc_vu0_add_00924770(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Sub_00924860(a, b, c)`.
    rcVu0Sub_00924860 @ 0x0092_4860 => extern "C" fn shim_rc_vu0_sub_00924860(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Sub3, a, b, c)
    }

    /// `rcVu0Add4(a, b, c)`.
    rcVu0Add4 @ 0x0092_4fc0 => extern "C" fn shim_rc_vu0_add4(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add4, a, b, c)
    }

    /// `rcVu0Sub4(a, b, c)`.
    rcVu0Sub4 @ 0x0092_50d0 => extern "C" fn shim_rc_vu0_sub4(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Sub4, a, b, c)
    }

    /// `rcVu0Add_009285a0(a, b, c)`.
    rcVu0Add_009285a0 @ 0x0092_85a0 => extern "C" fn shim_rc_vu0_add_009285a0(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Add_00928df0(a, b, c)`.
    rcVu0Add_00928df0 @ 0x0092_8df0 => extern "C" fn shim_rc_vu0_add_00928df0(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Sub_00928ee0(a, b, c)`.
    rcVu0Sub_00928ee0 @ 0x0092_8ee0 => extern "C" fn shim_rc_vu0_sub_00928ee0(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Sub3, a, b, c)
    }

    /// `rcVu0Sub_00929300(a, b, c)`.
    rcVu0Sub_00929300 @ 0x0092_9300 => extern "C" fn shim_rc_vu0_sub_00929300(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Sub3, a, b, c)
    }

    /// `rcVu0Add_00929440(a, b, c)`.
    rcVu0Add_00929440 @ 0x0092_9440 => extern "C" fn shim_rc_vu0_add_00929440(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Add_009295e0(a, b, c)`.
    rcVu0Add_009295e0 @ 0x0092_95e0 => extern "C" fn shim_rc_vu0_add_009295e0(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Add_0092f790(a, b, c)`.
    rcVu0Add_0092f790 @ 0x0092_f790 => extern "C" fn shim_rc_vu0_add_0092f790(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add3, a, b, c)
    }

    /// `rcVu0Add4_0092f880(a, b, c)`.
    rcVu0Add4_0092f880 @ 0x0092_f880 => extern "C" fn shim_rc_vu0_add4_0092f880(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_op(&mut mem(), vu0::bridge::Vu0Op::Add4, a, b, c)
    }

    /// `rcVu0Sub(a, b, c)`.
    rcVu0Sub @ 0x0091_f740 => extern "C" fn shim_rc_vu0_sub(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_sub(&mut mem(), a, b, c)
    }

    /// `rcVu0Sub_0092d720(a, b, c)`: the same code.
    rcVu0Sub_0092d720 @ 0x0092_d720 => extern "C" fn shim_rc_vu0_sub_0092d720(a: u32, b: u32, c: u32) -> u32 {
        vu0::bridge::rc_vu0_sub(&mut mem(), a, b, c)
    }

    /// `yiSetIDPage(page)`.
    yiSetIDPage @ 0x0090_a4a0 => extern "C" fn shim_yi_set_id_page(page: u32) -> u32 {
        yi::bridge::yi_set_id_page(&mut mem(), page, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiSetPreBgPage(page)`.
    yiSetPreBgPage @ 0x0090_a6f0 => extern "C" fn shim_yi_set_pre_bg_page(page: u32) -> u32 {
        yi::bridge::yi_set_pre_bg_page(&mut mem(), page, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiReset()`.
    yiReset @ 0x0090_9e60 => extern "C" fn shim_yi_reset() -> u32 {
        yi::bridge::yi_reset(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsReportStandby()`.
    SqsReportStandby @ 0x0081_e260 => extern "C" fn shim_sqs_report_standby() -> u32 {
        nkz::bridge::sqs_report_standby(&mut mem(), &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsReportStandbyStatus(flush)`.
    SqsReportStandbyStatus @ 0x0081_e2d0 => extern "C" fn shim_sqs_report_standby_status(flush: u32) -> u32 {
        nkz::bridge::sqs_report_standby_status(&mut mem(), flush, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsDataSendingCompleted(wait)`.
    SqsDataSendingCompleted @ 0x0081_cfd0 => extern "C" fn shim_sqs_data_sending_completed(wait: u32) -> u32 {
        nkz::bridge::sqs_data_sending_completed(&mut mem(), wait, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsInitMusicInfoHdd(path)`.
    SqsInitMusicInfoHdd @ 0x0081_d3c0 => extern "C" fn shim_sqs_init_music_info_hdd(path: u32) -> u32 {
        nkz::bridge::sqs_init_info_hdd(&mut mem(), path, 7, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsInitWaveInfoHdd(path)`.
    SqsInitWaveInfoHdd @ 0x0081_d470 => extern "C" fn shim_sqs_init_wave_info_hdd(path: u32) -> u32 {
        nkz::bridge::sqs_init_info_hdd(&mut mem(), path, 6, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `SqsSetProjectBase(path)`.
    SqsSetProjectBase @ 0x0081_eab0 => extern "C" fn shim_sqs_set_project_base(path: u32) -> u32 {
        nkz::bridge::sqs_set_project_base(&mut mem(), path, &mut |_, f, a| call_cdecl(f, a))
    }

    /// `yiPolyBufExecVif(buf, codes, words)`.
    yiPolyBufExecVif @ 0x0091_13a0 => extern "C" fn shim_yi_poly_buf_exec_vif(buf: u32, codes: u32, words: u32) -> u32 {
        yi::bridge::yi_poly_buf_exec_vif(&mut mem(), buf, codes, words)
    }

    /// `Sg_MemSetFree(block)`.
    Sg_MemSetFree @ 0x0083_bdb0 => extern "C" fn shim_sg_mem_set_free(block: u32) -> u32 {
        yi::bridge::sg_mem_set_free(&mut mem(), block)
    }
}

/// The original's 16-byte aligned stack buffer for a load-image packet
/// (`yiMapFileRead`).
#[repr(C, align(16))]
struct Image([u8; 0x60]);

/// The original's stack chunk and parser (`rcbgLoadBin`).
#[repr(C, align(16))]
struct Work([u8; 0xa4]);

/// `KeMdl_Draw`'s render description and view matrix.
#[repr(C, align(16))]
struct MdlWork([u8; 0xe0]);

/// A small 16-byte aligned stack buffer of the original (a vector pair, a
/// model description, a CD read's result).
#[repr(C, align(16))]
struct Scratch([u8; 0x20]);

/// A 16-byte aligned stack matrix of the original.
#[repr(C, align(16))]
struct Matrix([u8; 0x40]);

/// `yiSetLightMatrices`'s seven stack vectors.
#[repr(C, align(16))]
struct Lights([u8; 0x70]);

/// `rcbgSceneSetMatrix`'s matrices and scaled vectors.
#[repr(C, align(16))]
struct CameraWork([u8; camera::WORK as usize]);

/// `yiSwizzlePage`'s two block buffers.
#[repr(C, align(16))]
struct Swizzle([u8; 0x200]);

/// `yiMapSetup`'s four path strings.
#[repr(C, align(16))]
struct Host([u8; 0x1000]);

/// Calls an original routine that returns in `ST0`: an `_CI*` intrinsic
/// (its argument in `ST0`) or a cdecl function of two words; the result at
/// the x87's precision.
fn call_float(f: u32, args: camera::FArgs<'_>) -> f64 {
    match args {
        camera::FArgs::St0(x) => st0_call(f, x),
        camera::FArgs::St01(a, b) => st01_call(f, a, b),
        // SAFETY: the callers pass cdecl functions of two words returning
        // a float (`Ch_SetClipZ`); the declared `f32` return pops ST0.
        // SAFETY: a cdecl function of one word returning a double
        // (`atof`); the declared `f64` return pops ST0.
        // SAFETY: a cdecl function without arguments returning a float
        // (`MsCameraGetScrDpt`); the declared `f64` return pops ST0.
        camera::FArgs::Stack(&[]) => unsafe {
            std::mem::transmute::<usize, unsafe extern "C" fn() -> f64>(f as usize)()
        },
        camera::FArgs::Stack(&[a]) => unsafe {
            std::mem::transmute::<usize, unsafe extern "C" fn(u32) -> f64>(f as usize)(a)
        },
        camera::FArgs::Stack(&[a, b]) => unsafe {
            f64::from(std::mem::transmute::<
                usize,
                unsafe extern "C" fn(u32, u32) -> f32,
            >(f as usize)(a, b))
        },
        camera::FArgs::Stack(a) => {
            crate::log::log(&format!("map: no float call of {} words", a.len()));
            f64::NAN
        }
    }
}

/// An `_CI*` intrinsic through the game's import thunk: `x` in `ST0`
/// (`fld qword`), the result read back from `ST0` (`fstp qword`).
fn st0_call(f: u32, x: f64) -> f64 {
    // SAFETY: the thunk jumps to the CRT's intrinsic, which takes its
    // argument in ST0 and returns in ST0 (eax, ecx, edx, the x87 and SSE
    // registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let mut out = 0f64;
        std::arch::asm!(
            "fld qword ptr [{x}]",
            "call {f}",
            "fstp qword ptr [edi]",
            x = in(reg) &raw const x,
            f = in(reg) f,
            // edi: the callee keeps it (eax, ecx and edx it may not)
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        // the CRT's intrinsics only run in the 32-bit game
        let _ = (f, x);
        unreachable!("the CRT intrinsics: this runs in the 32-bit game")
    }
}

/// An `_CI*` intrinsic of two operands (`_CIfmod`) through the game's
/// import thunk: `a` in `ST1`, `b` in `ST0` (`fld qword` each), the result
/// read back from `ST0`.
fn st01_call(f: u32, a: f64, b: f64) -> f64 {
    // SAFETY: the thunk jumps to the CRT's intrinsic, which takes its
    // operands in ST1 and ST0 and leaves the result alone in ST0 (eax,
    // ecx, edx, the x87 and SSE registers clobbered).
    #[cfg(target_arch = "x86")]
    unsafe {
        let mut out = 0f64;
        std::arch::asm!(
            "fld qword ptr [{a}]",
            "fld qword ptr [{b}]",
            "call {f}",
            "fstp qword ptr [edi]",
            a = in(reg) &raw const a,
            b = in(reg) &raw const b,
            f = in(reg) f,
            // edi: the callee keeps it (eax, ecx and edx it may not)
            in("edi") &raw mut out,
            clobber_abi("C"),
        );
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        // the CRT's intrinsics only run in the 32-bit game
        let _ = (f, a, b);
        unreachable!("the CRT intrinsics: this runs in the 32-bit game")
    }
}

/// Calls an original cdecl function (absolute address) with its word
/// arguments, returning `eax`.
fn call_cdecl(f: u32, a: &[u32]) -> u32 {
    super::call_cdecl(f, a)
}

/// Calls a Phyre/CRT routine for the free camera: `arg` as `this`
/// (thiscall) or the one cdecl argument.
fn phyre_call(f: u32, arg: u32, thiscall: bool) {
    if thiscall {
        // SAFETY: the free camera's constructor/destructor slots are
        // thiscall functions of `this` alone.
        unsafe {
            let f: unsafe extern "thiscall" fn(u32) -> u32 = std::mem::transmute(f as usize);
            f(arg);
        }
    } else {
        call_ptr1(f, arg);
    }
}

/// Calls a game function pointer (an absolute address, cdecl) with one
/// argument: the sequence, after- and destructor functions of `rcef` tasks.
fn call_ptr1(f: u32, arg: u32) -> u32 {
    // SAFETY: the task's function slots hold cdecl functions taking it.
    unsafe {
        let f: unsafe extern "C" fn(u32) -> u32 = std::mem::transmute(f as usize);
        f(arg)
    }
}

/// Calls a force's reset function (an absolute address, cdecl) with the
/// force and its particle system (`rcfxParticleResetForce`).
fn call_ptr2(f: u32, a: u32, b: u32) -> u32 {
    // SAFETY: the force's reset slot and the sphere grid visitors hold
    // cdecl functions of two words.
    unsafe {
        let f: unsafe extern "C" fn(u32, u32) -> u32 = std::mem::transmute(f as usize);
        f(a, b)
    }
}

/// The VFX run record of an FP id (`pppFpGetMngPtr(id, &run, NULL)`).
fn fp_run(m: &mut super::RawMem, id: u32) -> u32 {
    let mut run = 0u32;
    #[allow(clippy::cast_possible_truncation)] // a 32-bit process
    let at = (&raw mut run).expose_provenance() as u32;
    ffx_vfx::fp::bridge::ppp_fp_get_mng_ptr(m, id, at, 0);
    run
}

/// MSVCR110's `srand`, through the game's import slot (`0x00b0c484`).
fn crt_srand(seed: u32) {
    // SAFETY: the import slot holds MSVCR110's cdecl srand(seed).
    unsafe {
        let f: unsafe extern "C" fn(u32) =
            std::mem::transmute(*(crate::rebase(0x00b0_c484) as *const usize));
        f(seed);
    }
}
