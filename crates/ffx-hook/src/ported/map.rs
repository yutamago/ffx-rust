//! `ffx-field`: data chunks, packets, geometry, scene objects, the guide
//! map, effect tasks, the field renderer's state, VU0 helpers, `Nkz`/`Sqs`.
//! Floats passed by value are taken as bits (they cross the x87 stack in
//! the original).

use super::{mem, ported};
use ffx_field::{geom, guide, ke, model, nkz, packet, rcdt, rcui, scene, task, vu0, yi};

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
}
