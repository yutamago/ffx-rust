//! `ffx-field`: data chunks, packets, geometry, scene objects, the guide
//! map, effect tasks, the field renderer's state, VU0 helpers, `Nkz`/`Sqs`.
//! Floats passed by value are taken as bits (they cross the x87 stack in
//! the original).

use super::{mem, ported};
use ffx_field::{geom, guide, model, nkz, packet, rcdt, task, vu0, yi};

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
}
