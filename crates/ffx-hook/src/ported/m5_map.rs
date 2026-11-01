//! `ffx_prender::m5m_*`: M5 stage 1, list `map` (the debug tweak panels,
//! the free camera's attach, scene and `std::string` helpers and the
//! field-map code the static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (`PhyreScene_bootInit`, `PhyreScene_render`,
//! the debug menu, the unwind funclets of the main thread's frames); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::l6_map::bridge as l6m;
use ffx_prender::l6_map2::bridge as l6m2;
use ffx_prender::l6_map3::bridge as l6m3;
use ffx_prender::l6_map4::bridge as l6m4;
use ffx_prender::l6_map5::bridge as l6m5;
use ffx_prender::l6_map6::bridge as l6m6;
use ffx_prender::l6_map7::bridge as l6m7;
use ffx_prender::m5m_a::{self as ma, bridge as a};
use ffx_prender::m5m_b::{self as mb, bridge as b};
use ffx_prender::m5m_c::{self as mc, bridge as c};
use ffx_prender::m5m_d::bridge as d;
use ffx_prender::m5m_e::bridge as e;
use ffx_prender::m5m_f::bridge as f;
use ffx_prender::m5m_g::bridge as g;
use ffx_prender::m5m_h::bridge as h;
use ffx_prender::m5m_i::bridge as gi;
use ffx_prender::m5m_j::bridge as gj;
use ffx_prender::m5m_k::bridge as gk;
use ffx_prender::m5m_l::bridge as gl;
use ffx_prender::m5m_m::bridge as gm;
use ffx_prender::m5m_n::bridge as gn;
use ffx_prender::m5m_o::bridge as go;
use ffx_prender::m5m_p::bridge as gp;
use ffx_prender::m5m_q::bridge as gq;
use ffx_prender::m5m_r::bridge as gr;
use ffx_prender::m5m_s::bridge as gs;
use ffx_prender::m5m_t::bridge as gt;
use ffx_prender::m5m_u::bridge as gu;
use ffx_prender::m5m_v::bridge as gv;

ported! {
    // ---- batch 1 (ffx_prender::m5m_a) ----
    /// `SectionVector_pushBack_006ba110(v; &item)`.
    SectionVector_pushBack_006ba110 @ 0x006b_a110 => extern "thiscall" fn section_push_back(this: u32, item: u32) {
        a::section_push_back(&mut mem(), this, item);
    }
    /// `BasicDebugTweakUtil_setEnabled(t; on)`: the strings, the line and
    /// the three numbers in the shim's frame (`ebp - 0x198..`).
    BasicDebugTweakUtil_setEnabled @ 0x006b_af60 => extern "thiscall" fn basic_set_enabled(this: u32, on: u32) -> u32 {
        let mut f = StackFrame::<0x68>::new();
        a::basic_set_enabled(&mut mem(), f.at() + 0x1a0, this, on)
    }
    /// `CharacterAnimationTweakUtil_setEnabled(t; on)`.
    CharacterAnimationTweakUtil_setEnabled @ 0x006b_b5f0 => extern "thiscall" fn char_anim_set_enabled(this: u32, on: u32) -> u32 {
        a::char_anim_set_enabled(&mut mem(), this, on)
    }
    /// `CharacterSwitchTweakUtil_setEnabled_006bb670(t; on)`.
    CharacterSwitchTweakUtil_setEnabled_006bb670 @ 0x006b_b670 => extern "thiscall" fn char_switch_set_enabled(this: u32, on: u32) -> u32 {
        a::set_enabled(&mut mem(), ma::CHAR_SWITCH_ENABLE, this, on)
    }
    /// `CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0(t; on)`.
    CharacterTextureAnimationTweakUtil_setEnabled_006bb6d0 @ 0x006b_b6d0 => extern "thiscall" fn char_tex_set_enabled(this: u32, on: u32) -> u32 {
        a::char_tex_set_enabled(&mut mem(), this, on)
    }
    /// `MapSwitchTweakUtil_setEnabled_006bb880(t; on)`.
    MapSwitchTweakUtil_setEnabled_006bb880 @ 0x006b_b880 => extern "thiscall" fn map_switch_set_enabled(this: u32, on: u32) -> u32 {
        a::set_enabled(&mut mem(), ma::MAP_SWITCH_ENABLE, this, on)
    }
    /// `InlineBuffer_free_0064e9a0(b)`.
    InlineBuffer_free_0064e9a0 @ 0x0064_e9a0 => extern "thiscall" fn inline_buffer_free(this: u32) -> u32 {
        a::inline_buffer_free(&mut mem(), this)
    }
    /// `AtlCString_release_0064e9e0(s)`.
    AtlCString_release_0064e9e0 @ 0x0064_e9e0 => extern "thiscall" fn cstring_release(this: u32) -> u32 {
        a::cstring_release(&mut mem(), this)
    }
    /// `std_string_tidy_0064ea00(s)`.
    std_string_tidy_0064ea00 @ 0x0064_ea00 => extern "thiscall" fn string_tidy(this: u32) {
        a::string_tidy(&mut mem(), this);
    }
    /// `std_vector_tidy_0064ea30(v)`.
    std_vector_tidy_0064ea30 @ 0x0064_ea30 => extern "thiscall" fn vector_tidy(this: u32) -> u32 {
        a::vector_tidy(&mut mem(), this)
    }
    /// `PhyreSceneCursor_postNext4_0064f1a0(c; out, _)` -> out.
    PhyreSceneCursor_postNext4_0064f1a0 @ 0x0064_f1a0 => extern "thiscall" fn cursor_post_next4(this: u32, out: u32, _unused: u32) -> u32 {
        a::cursor_post_next4(&mut mem(), this, out)
    }
    /// `std_string_less_0064f550(a, b)`.
    std_string_less_0064f550 @ 0x0064_f550 => extern "stdcall" fn string_less(x: u32, y: u32) -> u32 {
        a::string_less(&mut mem(), x, y)
    }
    /// `std_string_append_cstr_0064f5e0(s; p)`.
    std_string_append_cstr_0064f5e0 @ 0x0064_f5e0 => extern "thiscall" fn string_append_cstr(this: u32, p: u32) -> u32 {
        a::string_append_cstr(&mut mem(), this, p)
    }
    /// `FreeCamera_attach_0067fda0(cam; target)`: the matrices in the
    /// shim's frame (`ebp - 0x80..`).
    FreeCamera_attach_0067fda0 @ 0x0067_fda0 => extern "thiscall" fn free_camera_attach(this: u32, target: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        a::free_camera_attach(&mut mem(), f.at() + 0x80, this, target)
    }
    /// `BasicDebugTweakUtil_initialize(t)`: the line in the shim's frame.
    BasicDebugTweakUtil_initialize @ 0x006b_9360 => extern "thiscall" fn basic_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::BASIC_INIT, f.at() + 0x104, this)
    }
    /// `BtlChrParamConfigTweakUitl_initialize(t)`.
    BtlChrParamConfigTweakUitl_initialize @ 0x006b_94a0 => extern "thiscall" fn btl_chr_param_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::BTL_CHR_PARAM_INIT, f.at() + 0x104, this)
    }
    /// `BtlDebugTweakUtil_initialize(t)`.
    BtlDebugTweakUtil_initialize @ 0x006b_9600 => extern "thiscall" fn btl_debug_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::btl_debug_init(&mut mem(), f.at() + 0x104, this)
    }
    /// `CharacterAnimationTweakUtil_initialize(t)`.
    CharacterAnimationTweakUtil_initialize @ 0x006b_97f0 => extern "thiscall" fn char_anim_init(this: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        a::char_anim_init(&mut mem(), f.at() + 0x2c, this)
    }
    /// `CharacterSwitchTweakUtil_initialize(t)`.
    CharacterSwitchTweakUtil_initialize @ 0x006b_9970 => extern "thiscall" fn char_switch_init(this: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        a::char_switch_init(&mut mem(), f.at() + 0x18, this)
    }
    /// `CharacterTextureAnimationTweakUtil_initialize(t)`.
    CharacterTextureAnimationTweakUtil_initialize @ 0x006b_9aa0 => extern "thiscall" fn char_tex_init(this: u32) -> u32 {
        let mut f = StackFrame::<0xe>::new();
        a::char_tex_init(&mut mem(), f.at() + 0x38, this)
    }
    /// `GameParamConfigTweakUitl_initialize(t)`.
    GameParamConfigTweakUitl_initialize @ 0x006b_9cd0 => extern "thiscall" fn game_param_init(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::init_lines(&mut mem(), ma::GAME_PARAM_INIT, f.at() + 0x104, this)
    }
    /// `BasicDebugTweakUtil_render(t; ctx, fps)`.
    BasicDebugTweakUtil_render @ 0x006b_a1a0 => extern "thiscall" fn basic_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BASIC_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `BtlChrParamConfigTweakUitl_render(t; ctx, fps)`.
    BtlChrParamConfigTweakUitl_render @ 0x006b_a240 => extern "thiscall" fn btl_chr_param_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BTL_CHR_PARAM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `BtlDebugTweakUtil_render(t; ctx, fps)`.
    BtlDebugTweakUtil_render @ 0x006b_a2e0 => extern "thiscall" fn btl_debug_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::BTL_DEBUG_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `CharacterAnimationTweakUtil_render(t; ctx, fps)`.
    CharacterAnimationTweakUtil_render @ 0x006b_a390 => extern "thiscall" fn char_anim_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::CHAR_ANIM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `CharacterSwitchTweakUtil_render(t; ctx, fps)`.
    CharacterSwitchTweakUtil_render @ 0x006b_a430 => extern "thiscall" fn char_switch_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        a::render(&mut mem(), ma::CHAR_SWITCH_RENDER, f.at() + 0x24, this, ctx, fps)
    }
    /// `CharacterTextureAnimationTweakUtil_render(t; ctx, fps)`.
    CharacterTextureAnimationTweakUtil_render @ 0x006b_a4d0 => extern "thiscall" fn char_tex_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::CHAR_TEX_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `GameParamConfigTweakUitl_render(t; ctx, fps)`.
    GameParamConfigTweakUitl_render @ 0x006b_a570 => extern "thiscall" fn game_param_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::GAME_PARAM_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `MapSwitchTweakUtil_render(t; ctx, fps)`.
    MapSwitchTweakUtil_render @ 0x006b_a610 => extern "thiscall" fn map_switch_render(this: u32, ctx: u32, fps: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        a::render(&mut mem(), ma::MAP_SWITCH_RENDER, f.at() + 0x104, this, ctx, fps)
    }
    /// `rcbgGuideMapSave(a, loc)`: writes its second argument slot.
    rcbgGuideMapSave @ 0x006e_9570 => extern "C" fn guide_map_save(x: u32, loc: u32; slots: u32) -> u32 {
        a::guide_map_save(&mut mem(), slots, x, loc)
    }

    // ---- batch 2 (ffx_prender::m5m_b) ----
    /// `SqsStraightStreamingNextHalf_0081f160()`.
    SqsStraightStreamingNextHalf_0081f160 @ 0x0081_f160 => extern "C" fn stream_next_half() -> u32 {
        b::stream_next_half(&mut mem())
    }
    /// `SqsStraightStreamingRecord_0081f760()`.
    SqsStraightStreamingRecord_0081f760 @ 0x0081_f760 => extern "C" fn stream_record() -> u32 {
        b::stream_record(&mut mem())
    }
    /// `SqsStraightStreamingDataSend(data, start, adpcm, arg)`: reuses its
    /// first two argument slots.
    SqsStraightStreamingDataSend @ 0x0081_f210 => extern "C" fn straight_data_send(data: u32, start: u32, adpcm: u32, arg: u32; slots: u32) -> u32 {
        b::straight_data_send(&mut mem(), slots, data, start, adpcm, arg)
    }
    /// `SqsStraightStreamingDigitalModeSet(x)`.
    SqsStraightStreamingDigitalModeSet @ 0x0081_f550 => extern "C" fn digital_mode_set(x: u32) -> u32 {
        b::digital_mode_set(&mut mem(), x)
    }
    /// `YiRenderStatus_clearBit8_00908440()`.
    YiRenderStatus_clearBit8_00908440 @ 0x0090_8440 => extern "C" fn yi_status_clear_bit8() {
        b::yi_status_clear_bit8(&mut mem());
    }
    /// `yiGui_setfogparam()`.
    yiGui_setfogparam @ 0x0090_8f90 => extern "C" fn set_fog_param() -> u32 {
        b::set_fog_param(&mut mem())
    }
    /// `TkVibrateCurve_get_009090b0()`.
    TkVibrateCurve_get_009090b0 @ 0x0090_90b0 => extern "C" fn vibrate_curve() -> u32 {
        b::vibrate_curve(&mut mem())
    }
    /// `YiRender_initFogDefault_0090a420()`.
    YiRender_initFogDefault_0090a420 @ 0x0090_a420 => extern "C" fn yi_init_fog() -> u32 {
        b::yi_init_fog(&mut mem())
    }
    /// `yiSendGsPacket(packet, qwc)`: clears its argument slots.
    yiSendGsPacket @ 0x0091_2690 => extern "C" fn send_gs_packet(packet: u32, qwc: u32; slots: u32) -> u32 {
        b::send_gs_packet(&mut mem(), slots, packet, qwc)
    }
    /// `YnMapChain_clearBit0_0091a200()`.
    YnMapChain_clearBit0_0091a200 @ 0x0091_a200 => extern "C" fn chain_clear_bit0() -> u32 {
        b::chain_bit0(&mut mem(), false)
    }
    /// `YnMapChain_setBit0_0091a220()`.
    YnMapChain_setBit0_0091a220 @ 0x0091_a220 => extern "C" fn chain_set_bit0() -> u32 {
        b::chain_bit0(&mut mem(), true)
    }
    /// `rcdtChunkLoadFromFile(chunk, path)`.
    rcdtChunkLoadFromFile @ 0x0092_b880 => extern "C" fn chunk_load_from_file(chunk: u32, path: u32) -> u32 {
        b::chunk_load_from_file(&mut mem(), chunk, path)
    }
    /// `Yn_FpMemWinOpen()`.
    Yn_FpMemWinOpen @ 0x0093_3c80 => extern "C" fn fp_mem_win_open() -> u32 {
        b::fp_mem_win_open(&mut mem())
    }
    /// `rcuiInit()`.
    rcuiInit @ 0x0093_4390 => extern "C" fn rcui_init() -> u32 {
        b::rcui_init(&mut mem())
    }
    /// `rcuiWindowCreate(holder, x, y, w, h, flags, proc, title, parent)`.
    rcuiWindowCreate @ 0x0093_7ef0 => extern "C" fn window_create(holder: u32, x: u32, y: u32, w: u32, h: u32, flags: u32, proc: u32, title: u32, parent: u32) -> u32 {
        b::window_create(&mut mem(), holder, [x, y, w, h, flags, proc], title, parent)
    }
    /// `SqsSeSeparateMaskClear_thunk_00886dc0(m0..m7)`.
    SqsSeSeparateMaskClear_thunk_00886dc0 @ 0x0088_6dc0 => extern "C" fn se_separate_mask_clear_thunk(m0: u32, m1: u32, m2: u32, m3: u32, m4: u32, m5: u32, m6: u32, m7: u32) -> u32 {
        b::thunk(&mut mem(), mb::SE_SEPARATE_MASK_CLEAR_THUNK, &[m0, m1, m2, m3, m4, m5, m6, m7])
    }
    /// `SqsSetCycleMode_thunk(a)`.
    SqsSetCycleMode_thunk @ 0x0088_7070 => extern "C" fn set_cycle_mode_thunk(x: u32) -> u32 {
        b::thunk(&mut mem(), mb::SET_CYCLE_MODE_THUNK, &[x])
    }
    /// `SqsSetReverbPlaySwitch_thunk(a)`.
    SqsSetReverbPlaySwitch_thunk @ 0x0088_7170 => extern "C" fn set_reverb_play_switch_thunk(x: u32) -> u32 {
        b::thunk(&mut mem(), mb::SET_REVERB_PLAY_SWITCH_THUNK, &[x])
    }
    /// `SqsAllVoiceTerminate_thunk_008872e0()`.
    SqsAllVoiceTerminate_thunk_008872e0 @ 0x0088_72e0 => extern "C" fn all_voice_terminate_thunk() -> u32 {
        b::thunk(&mut mem(), mb::ALL_VOICE_TERMINATE_THUNK, &[])
    }
    /// `SqsShoutStreamingDataSend_thunk(src, size)`.
    SqsShoutStreamingDataSend_thunk @ 0x0088_7560 => extern "C" fn shout_streaming_data_send_thunk(src: u32, size: u32) -> u32 {
        b::thunk(&mut mem(), mb::SHOUT_STREAMING_DATA_SEND_THUNK, &[src, size])
    }
    /// `yiFree(p)`.
    yiFree @ 0x0090_bbc0 => extern "C" fn yi_free(p: u32) -> u32 {
        b::thunk(&mut mem(), mb::YI_FREE, &[p])
    }
    /// `rcbgInit_thunk_00919a70()`.
    rcbgInit_thunk_00919a70 @ 0x0091_9a70 => extern "C" fn rcbg_init_thunk() -> u32 {
        b::thunk(&mut mem(), mb::RCBG_INIT_THUNK, &[])
    }
    /// `pppFpDispDebug_thunk_00919b70()`.
    pppFpDispDebug_thunk_00919b70 @ 0x0091_9b70 => extern "C" fn fp_disp_debug_thunk() -> u32 {
        b::thunk(&mut mem(), mb::FP_DISP_DEBUG_THUNK, &[])
    }
    /// `Yn_FpMemWinOpen_thunk_00919b80()`.
    Yn_FpMemWinOpen_thunk_00919b80 @ 0x0091_9b80 => extern "C" fn fp_mem_win_open_thunk() -> u32 {
        b::thunk(&mut mem(), mb::FP_MEM_WIN_OPEN_THUNK, &[])
    }
    /// `Yn_PreBgSetData_thunk_00919b90(data)`.
    Yn_PreBgSetData_thunk_00919b90 @ 0x0091_9b90 => extern "C" fn pre_bg_set_data_thunk_a(data: u32) -> u32 {
        b::thunk(&mut mem(), mb::PRE_BG_SET_DATA_THUNK_A, &[data])
    }
    /// `rcbgTrxPairSetNo_thunk_00919bd0(p, n)`.
    rcbgTrxPairSetNo_thunk_00919bd0 @ 0x0091_9bd0 => extern "C" fn trx_pair_set_no_thunk(p: u32, n: u32) -> u32 {
        b::thunk(&mut mem(), mb::TRX_PAIR_SET_NO_THUNK, &[p, n])
    }
    /// `Yn_PreBgSetData_thunk_00919d30(data)`.
    Yn_PreBgSetData_thunk_00919d30 @ 0x0091_9d30 => extern "C" fn pre_bg_set_data_thunk_b(data: u32) -> u32 {
        b::thunk(&mut mem(), mb::PRE_BG_SET_DATA_THUNK_B, &[data])
    }
    /// `rcbgSceneSetChadist_thunk(scene)`.
    rcbgSceneSetChadist_thunk @ 0x0092_0780 => extern "C" fn scene_set_chadist_thunk(scene: u32) -> u32 {
        b::thunk(&mut mem(), mb::SCENE_SET_CHADIST_THUNK, &[scene])
    }
    /// `rcFree(p)`.
    rcFree @ 0x0092_58e0 => extern "C" fn rc_free(p: u32) -> u32 {
        b::thunk(&mut mem(), mb::RC_FREE, &[p])
    }
    /// `rcFree_thunk_0092ba50(p)`.
    rcFree_thunk_0092ba50 @ 0x0092_ba50 => extern "C" fn rc_free_thunk(p: u32) -> u32 {
        b::thunk(&mut mem(), mb::RC_FREE_THUNK, &[p])
    }
    /// `rcefTaskRetSeqCont_thunk(t)`.
    rcefTaskRetSeqCont_thunk @ 0x0093_04d0 => extern "C" fn task_ret_seq_cont_thunk(t: u32) -> u32 {
        b::thunk(&mut mem(), mb::TASK_RET_SEQ_CONT_THUNK, &[t])
    }

    // ---- batch 3 (ffx_prender::m5m_c) ----
    /// `rcdtTm2LoadEx_009232a0(pic, path, ps2_alpha)`.
    rcdtTm2LoadEx_009232a0 @ 0x0092_32a0 => extern "C" fn tm2_load(pic: u32, path: u32, ps2_alpha: u32) -> u32 {
        c::tm2_load(&mut mem(), pic, path, ps2_alpha)
    }
    /// `rcdtTm2OpaqueBounds_009234d0(pic, &x, &y, &w, &h)`.
    rcdtTm2OpaqueBounds_009234d0 @ 0x0092_34d0 => extern "C" fn tm2_opaque_bounds(pic: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        c::tm2_bounds(&mut mem(), mc::Opaque::Alpha, pic, [x, y, w, h])
    }
    /// `rcdtTm2KeyBounds_00923660(pic, &x, &y, &w, &h)`.
    rcdtTm2KeyBounds_00923660 @ 0x0092_3660 => extern "C" fn tm2_key_bounds(pic: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        c::tm2_bounds(&mut mem(), mc::Opaque::Black, pic, [x, y, w, h])
    }
    /// `rcdtTm2Load(pic, path)`.
    rcdtTm2Load @ 0x0092_3800 => extern "C" fn tm2_load_ps2(pic: u32, path: u32) -> u32 {
        c::tm2_load_ps2(&mut mem(), pic, path)
    }
    /// `rcdtRsdLoad_LoadMat_00923b60(model, path)`.
    rcdtRsdLoad_LoadMat_00923b60 @ 0x0092_3b60 => extern "C" fn load_mat(model: u32, path: u32) -> u32 {
        c::load_mat(&mut mem(), model, path)
    }
    /// `rcbgIDLoad()`: the path in the shim's frame.
    rcbgIDLoad @ 0x0092_5730 => extern "C" fn id_load() -> u32 {
        let mut f = StackFrame::<0x42>::new();
        c::id_load(&mut mem(), f.at() + 0x104)
    }
    /// `rcPacketRect_00927630(pkt, x, y, w, h, rgba)`.
    rcPacketRect_00927630 @ 0x0092_7630 => extern "C" fn packet_rect(pkt: u32, x: u32, y: u32, w: u32, h: u32, rgba: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        c::packet_rect(&mut mem(), f.at() + 0x20, pkt, [x, y, w, h, rgba])
    }
    /// `rcPacketCircle_00927720(pkt, x, y, r, rgba)`.
    rcPacketCircle_00927720 @ 0x0092_7720 => extern "C" fn packet_circle(pkt: u32, x: u32, y: u32, r: u32, rgba: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        c::packet_circle(&mut mem(), f.at() + 0x20, pkt, [x, y, r, rgba])
    }
    /// `rcPacketLine_00927810(pkt, x0, y0, x1, y1, rgba)`.
    rcPacketLine_00927810 @ 0x0092_7810 => extern "C" fn packet_line(pkt: u32, x0: u32, y0: u32, x1: u32, y1: u32, rgba: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        c::packet_line(&mut mem(), f.at() + 0x20, pkt, [x0, y0, x1, y1, rgba])
    }
    /// `rcdtRsdLoad_LoadMatRsd(model, path)`: the type byte in the shim's
    /// frame.
    rcdtRsdLoad_LoadMatRsd @ 0x0092_3bc0 => extern "C" fn load_mat_rsd(model: u32, path: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        c::load_mat_rsd(&mut mem(), f.at() + 0x14, model, path)
    }
    /// `rcdtRsdLoad_LoadMatRsd2(model, path)`: the type byte in the
    /// shim's frame.
    rcdtRsdLoad_LoadMatRsd2 @ 0x0092_3ed0 => extern "C" fn load_mat_rsd2(model: u32, path: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        c::load_mat_rsd2(&mut mem(), f.at() + 0x14, model, path)
    }
    /// `rcdtRsdLoad_LoadPly(model, path)`.
    rcdtRsdLoad_LoadPly @ 0x0092_4090 => extern "C" fn load_ply(model: u32, path: u32) -> u32 {
        c::load_ply(&mut mem(), model, path)
    }
    /// `rcdtRsdLoad_009243c0(model, path)`: the keyword table and the
    /// path buffers in the shim's frame (`ebp - 0x328..`).
    rcdtRsdLoad_009243c0 @ 0x0092_43c0 => extern "C" fn rsd_load(model: u32, path: u32) -> u32 {
        let mut f = StackFrame::<0xcc>::new();
        c::rsd_load(&mut mem(), f.at() + 0x330, model, path)
    }
    /// `rcbgDrawGuidePackets_0092c130()`: the register block and the
    /// packet in the shim's frame.
    rcbgDrawGuidePackets_0092c130 @ 0x0092_c130 => extern "C" fn draw_guide_packets() -> u32 {
        let mut f = StackFrame::<0x12>::new();
        d::draw_guide_packets(&mut mem(), f.at() + 0x44)
    }
    /// `rcbgDrawGuidePoints_0092c4c0(pkt, obj)`: the point record, matrix
    /// and vector in the shim's frame.
    rcbgDrawGuidePoints_0092c4c0 @ 0x0092_c4c0 => extern "C" fn draw_guide_points(pkt: u32, obj: u32) -> u32 {
        let mut f = StackFrame::<0x27>::new();
        d::draw_guide_points(&mut mem(), f.at() + 0x98, pkt, obj)
    }
    /// `rcuiGetStageName2(no, out, battle)`: the chunk and parse in the
    /// shim's frame.
    rcuiGetStageName2 @ 0x0093_7960 => extern "C" fn stage_name2(no: u32, out: u32, battle: u32) -> u32 {
        let mut f = StackFrame::<0x28>::new();
        d::stage_name2(&mut mem(), f.at() + 0xa0, no, out, battle)
    }
    /// `rcuiMouseMoveGet(&dx, &dy)`.
    rcuiMouseMoveGet @ 0x0093_c060 => extern "C" fn mouse_move_get(dx: u32, dy: u32) -> u32 {
        d::mouse_move_get(&mut mem(), dx, dy)
    }
    /// `rcuiAddItems(window, items)`.
    rcuiAddItems @ 0x0093_7780 => extern "C" fn add_items(window: u32, items: u32) -> u32 {
        d::add_items(&mut mem(), window, items)
    }
    /// `rcuiPercentPrint_00930ba0(x)` (`x` an f32's bits).
    rcuiPercentPrint_00930ba0 @ 0x0093_0ba0 => extern "C" fn percent_print(x: u32) -> u32 {
        d::percent_print(&mut mem(), x)
    }
    /// `rcuiNumberCreate(num, value, kind, title, parent)`.
    rcuiNumberCreate @ 0x0093_c2e0 => extern "C" fn number_create(num: u32, value: u32, kind: u32, title: u32, parent: u32) {
        d::number_create(&mut mem(), num, value, kind, title, parent);
    }
    /// `Wrap_0092c600_0092c5e0(a, b)`.
    Wrap_0092c600_0092c5e0 @ 0x0092_c5e0 => extern "C" fn wrap_0092c600(x: u32, y: u32) -> u32 {
        c::wrap_0092c600(&mut mem(), x, y)
    }
    /// `rcuiGetStageName(a, b)`.
    rcuiGetStageName @ 0x0093_7940 => extern "C" fn stage_name(x: u32, y: u32) -> u32 {
        c::stage_name(&mut mem(), x, y)
    }
    /// `rcbgFpSaveCurrent_00938340()`.
    rcbgFpSaveCurrent_00938340 @ 0x0093_8340 => extern "C" fn fp_save_current() -> u32 {
        c::fp_save_current(&mut mem())
    }
    /// `rcbgGuideMapConvCurrent_00938570()`.
    rcbgGuideMapConvCurrent_00938570 @ 0x0093_8570 => extern "C" fn guide_map_conv_current() -> u32 {
        c::guide_map_conv_current(&mut mem())
    }
    /// `rcbgFpLoadTextCurrent_0093a7e0()`.
    rcbgFpLoadTextCurrent_0093a7e0 @ 0x0093_a7e0 => extern "C" fn fp_load_text_current() -> u32 {
        c::fp_load_text_current(&mut mem())
    }
    // ---- round 5 batch 1 (ffx_prender::m5m_e) ----
    /// `rcuiPreMemWinOpen()` -> `rcuiWindowCreate`'s `eax`.
    rcuiPreMemWinOpen @ 0x0093_64a0 => extern "C" fn pre_mem_win_open() -> u32 {
        e::pre_mem_win_open(&mut mem())
    }
    /// `rcuiPreMemProc(window, msg, param)` -> `eax` as the original
    /// leaves it.
    rcuiPreMemProc @ 0x0093_63f0 => extern "C" fn pre_mem_proc(_window: u32, msg: u32, _param: u32) -> u32 {
        e::pre_mem_proc(&mut mem(), msg)
    }
    /// `rcuiParProcKick(force)`.
    rcuiParProcKick @ 0x0093_6340 => extern "C" fn par_proc_kick(force: u32) {
        e::par_proc_kick(&mut mem(), force);
    }
    /// `rcuiMakeFullPath()` -> the map number.
    rcuiMakeFullPath @ 0x0093_7b20 => extern "C" fn make_full_path() -> u32 {
        e::make_full_path(&mut mem())
    }
    /// `rcuiTreeWindowOpen_0093c1d0(x, y)`.
    rcuiTreeWindowOpen_0093c1d0 @ 0x0093_c1d0 => extern "C" fn tree_window_open(x: u32, y: u32) -> u32 {
        e::tree_window_open(&mut mem(), x, y)
    }
    /// `jmp abmap_cheng_panel` (`0x00a48740`): its `eax`.
    abmap_cheng_panel_thunk_00a45860 @ 0x00a4_5860 => extern "C" fn abmap_cheng_panel_thunk(kind: u32, node: u32) -> u32 {
        e::abmap_cheng_panel_thunk(&mut mem(), kind, node)
    }
    /// `rcbgSceneLoadAll_0093aec0(scene)`.
    rcbgSceneLoadAll_0093aec0 @ 0x0093_aec0 => extern "C" fn scene_load_all(scene: u32) -> u32 {
        e::scene_load_all(&mut mem(), scene)
    }
    /// `rcbgPreLoadText()`.
    rcbgPreLoadText @ 0x0093_ae60 => extern "C" fn pre_load_text() -> u32 {
        e::pre_load_text(&mut mem())
    }
    /// `rcbgFpTrxTexLoadAll_0093a890(path)`: the number buffer in this
    /// shim's frame.
    rcbgFpTrxTexLoadAll_0093a890 @ 0x0093_a890 => extern "C" fn fp_trx_load_all(path: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        e::fp_trx_load_all(&mut mem(), frame.at(), path)
    }
    /// `rcbgFpLoadTextSub(id, flag)`: the stage name and path in this
    /// shim's frame.
    rcbgFpLoadTextSub @ 0x0093_a800 => extern "C" fn fp_load_text_sub(id: u32, flag: u32) -> u32 {
        let mut frame = StackFrame::<0x42>::new();
        let at = frame.at();
        e::fp_load_text_sub(&mut mem(), at + 0x100, at, id, flag)
    }
    /// `rcbgSaveBin_00938fd0()`: the pack list in this shim's frame.
    rcbgSaveBin_00938fd0 @ 0x0093_8fd0 => extern "C" fn save_bin() -> u32 {
        let mut frame = StackFrame::<0x42>::new();
        e::save_bin(&mut mem(), frame.at())
    }
    /// `rcbgTrxSave(t, fd)`.
    rcbgTrxSave @ 0x0093_9950 => extern "C" fn trx_save(t: u32, fd: u32) -> u32 {
        e::trx_save(&mut mem(), t, fd)
    }
    /// `rcbgSaveChunkHeader_00938f40(size, fd)`: the header in this shim's
    /// frame; writes from its argument slot.
    rcbgSaveChunkHeader_00938f40 @ 0x0093_8f40 => extern "C" fn save_chunk_header(_size: u32, fd: u32; slots: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        e::save_chunk_header(&mut mem(), slots, frame.at(), fd)
    }
    /// `rcbgSaveMesh_00938e70(obj, fd)`: writes its first argument slot.
    rcbgSaveMesh_00938e70 @ 0x0093_8e70 => extern "C" fn save_mesh(obj: u32, fd: u32; slots: u32) -> u32 {
        e::save_mesh(&mut mem(), slots, obj, fd)
    }
    /// `rcbgModelLoadByExt_0093ad60(a, name)` -> the model or 0.
    rcbgModelLoadByExt_0093ad60 @ 0x0093_ad60 => extern "C" fn model_load_by_ext(a: u32, name: u32) -> u32 {
        e::model_load_by_ext(&mut mem(), a, name)
    }
    /// `rcbgCountNumberedFiles_0093acb0(pattern, prefix, digits)` -> the
    /// count: the file list in this shim's frame.
    rcbgCountNumberedFiles_0093acb0 @ 0x0093_acb0 => extern "C" fn count_numbered_files(pattern: u32, prefix: u32, digits: u32) -> u32 {
        let mut frame = StackFrame::<3>::new();
        e::count_numbered_files(&mut mem(), frame.at(), pattern, prefix, digits)
    }
    /// `rcuiGaugeDraw_0093c730(window, x, y, value)`.
    rcuiGaugeDraw_0093c730 @ 0x0093_c730 => extern "C" fn gauge_draw(window: u32, x: u32, y: u32, value: u32) -> u32 {
        e::gauge_draw(&mut mem(), window, x, y, value)
    }
    // ---- round 5 batch 2 (ffx_prender::m5m_f) ----
    /// `rcbgFpTrxTexLoadTextGetTrxCnt(path, base)` -> the count: the list,
    /// command and name in this shim's frame.
    rcbgFpTrxTexLoadTextGetTrxCnt @ 0x0093_a910 => extern "C" fn trx_count(path: u32, base: u32) -> u32 {
        let mut frame = StackFrame::<0x4a>::new();
        f::trx_count(&mut mem(), frame.at(), path, base)
    }
    /// `rcbgGuideMapLoadTextSub(id)` -> the load's result: the stage name
    /// and path in this shim's frame.
    rcbgGuideMapLoadTextSub @ 0x0093_aa60 => extern "C" fn guide_map_load_text_sub(id: u32) -> u32 {
        let mut frame = StackFrame::<0x42>::new();
        let at = frame.at();
        f::guide_map_load_text_sub(&mut mem(), at + 0x100, at, id)
    }
    /// `rcbgTrxSave2_009399f0(t, fd)`: the pad in this shim's frame.
    rcbgTrxSave2_009399f0 @ 0x0093_99f0 => extern "C" fn trx_save2(t: u32, fd: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        f::trx_save2(&mut mem(), frame.at(), t, fd)
    }
    // ---- round 5 batch 3 (ffx_prender::m5m_g) ----
    /// `rcbgSceneLoadTextObjConfig_0093ab80(obj, p)` -> `rcdtParseIsEnd`'s
    /// `eax` or 9.
    rcbgSceneLoadTextObjConfig_0093ab80 @ 0x0093_ab80 => extern "C" fn obj_config(obj: u32, p: u32) -> u32 {
        g::obj_config(&mut mem(), obj, p)
    }
    /// `rcbgSceneLoadTextSubConfig(scene, name)` -> 1/0: the chunk and
    /// parser in this shim's frame.
    rcbgSceneLoadTextSubConfig @ 0x0093_b590 => extern "C" fn load_text_sub_config(scene: u32, name: u32) -> u32 {
        let mut frame = StackFrame::<0x26>::new();
        g::load_text_sub_config(&mut mem(), frame.at(), scene, name)
    }
    // ---- round 5 batch 4 (ffx_prender::m5m_h) ----
    /// `rcuiVramWrite()` -> `sceDmaSync`'s `eax`: the PSM table in this
    /// shim's frame.
    rcuiVramWrite @ 0x0093_7190 => extern "C" fn vram_write() -> u32 {
        let mut frame = StackFrame::<14>::new();
        h::vram_write(&mut mem(), frame.at())
    }
    // ---- round 5 batch 5 (ffx_prender::m5m_i) ----
    /// `rcbgGuideMapConvSub(id, sub)` -> the model or 0: the stage name and
    /// path in this shim's frame.
    rcbgGuideMapConvSub @ 0x0093_8080 => extern "C" fn guide_map_conv_sub(id: u32, sub: u32) -> u32 {
        let mut frame = StackFrame::<0x42>::new();
        let at = frame.at();
        gi::guide_map_conv_sub(&mut mem(), at + 0x100, at, id, sub)
    }
    // ---- round 5 batch 6 (ffx_prender::m5m_j) ----
    /// `rcuiNumWindowCreate_0093c670(win, value, title, parent)` -> the
    /// whole part: the original's 8-byte frame (`modf`'s whole part) in
    /// this shim's.
    rcuiNumWindowCreate_0093c670 @ 0x0093_c670 => extern "C" fn rcui_num_window_create(win: u32, value: u32, title: u32, parent: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_j::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_j::FRAME;
        gj::num_window_create(&mut mem(), ebp, win, value, title, parent)
    }
    // ---- round 5 batch 7 (ffx_prender::m5m_k) ----
    /// `rcuiGuideSaveMap_00938d20(m, fd)` -> the last `sceWrite`'s `eax`
    /// (the original's 0x14-byte frame in this shim's).
    rcuiGuideSaveMap_00938d20 @ 0x0093_8d20 => extern "C" fn rcui_guide_save_map(m: u32, fd: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_k::MAP_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_k::MAP_FRAME;
        gk::save_map(&mut mem(), ebp, m, fd)
    }
    /// `rcuiGuideSave_009397f0(g, fd)` -> the last `sceWrite`'s `eax` (the
    /// original's 0x30-byte frame in this shim's).
    rcuiGuideSave_009397f0 @ 0x0093_97f0 => extern "C" fn rcui_guide_save(g: u32, fd: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_k::SAVE_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_k::SAVE_FRAME;
        gk::save(&mut mem(), ebp, g, fd)
    }
    // ---- round 5 batch 8 (ffx_prender::m5m_l) ----
    /// `rcuiMapIdFromCsv_009377e0(name)` -> the id or -1 (the original's
    /// 0x98-byte frame, its chunk and parser, in this shim's).
    rcuiMapIdFromCsv_009377e0 @ 0x0093_77e0 => extern "C" fn rcui_map_id_from_csv(name: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_l::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_l::FRAME;
        gl::map_id_from_csv(&mut mem(), ebp, name)
    }
    // ---- round 5 batch 9 (ffx_prender::m5m_m) ----
    /// `rcbgSaveBinNormal()` -> the last `rcPrint`'s `eax` (the original's
    /// 0x318-byte frame, its path, command, reply and header, in this
    /// shim's).
    rcbgSaveBinNormal @ 0x0093_9040 => extern "C" fn rcbg_save_bin_normal() -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_m::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_m::FRAME;
        gm::save_bin_normal(&mut mem(), ebp)
    }
    // ---- round 5 batch 10 (ffx_prender::m5m_n) ----
    /// `rcbgSceneLoadTextureNormal(scene, i)` -> `rcdtTm2Delete`'s `eax`
    /// (the original's 0x118-byte frame, its path and outputs, in this
    /// shim's).
    rcbgSceneLoadTextureNormal @ 0x0093_bd10 => extern "C" fn rcbg_scene_load_texture_normal(scene: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_n::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_n::FRAME;
        gn::load_texture_normal(&mut mem(), ebp, scene, i)
    }
    // ---- round 5 batch 11 (ffx_prender::m5m_o) ----
    /// `rcbgSceneLoadTextSub(scene)` -> `rcdtParseFini`'s `eax` (the
    /// original's 0x198-byte frame, its chunk, parser and path, in this
    /// shim's).
    rcbgSceneLoadTextSub @ 0x0093_b3e0 => extern "C" fn rcbg_scene_load_text_sub(scene: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_o::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_o::FRAME;
        go::load_text_sub(&mut mem(), ebp, scene)
    }
    // ---- round 5 batch 12 (ffx_prender::m5m_p) ----
    /// `rcbgTrxTexLoadFromFile(t, dir, name, digits)` -> `rcdtTm2Fini`'s
    /// `eax` (the original's 0x25c-byte frame, its TIM2 record, format and
    /// path, in this shim's).
    rcbgTrxTexLoadFromFile @ 0x0093_beb0 => extern "C" fn rcbg_trx_tex_load_from_file(t: u32, dir: u32, name: u32, digits: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_p::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_p::FRAME;
        gp::tex_load_from_file(&mut mem(), ebp, t, dir, name, digits)
    }
    // ---- round 5 batch 13 (ffx_prender::m5m_q) ----
    /// `rcbgSceneLoadTextureAnimeRect(i, x, y, w, h)` -> the last callee's
    /// `eax` (the original's 0x11c-byte frame, its name and path, in this
    /// shim's).
    rcbgSceneLoadTextureAnimeRect @ 0x0093_bbc0 => extern "C" fn rcbg_scene_load_texture_anime_rect(i: u32, x: u32, y: u32, w: u32, hh: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_q::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_q::FRAME;
        gq::load_anime_rect(&mut mem(), ebp, i, x, y, w, hh)
    }
    // ---- round 5 batch 14 (ffx_prender::m5m_r) ----
    /// `rcbgSceneLoadTextureAnime(scene, i)` -> the last callee's `eax` (the
    /// original's 0x138-byte frame, its name, path and spills, in this
    /// shim's).
    rcbgSceneLoadTextureAnime @ 0x0093_b930 => extern "C" fn rcbg_scene_load_texture_anime(scene: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_r::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_r::FRAME;
        gr::load_texture_anime(&mut mem(), ebp, scene, i)
    }
    // ---- round 5 batch 15 (ffx_prender::m5m_s) ----
    /// `rcbgSaveChkPackList(maps, pack)` -> the map count or 0 (the
    /// original's 0x1a4-byte frame, its chunk, parser and path, in this
    /// shim's).
    rcbgSaveChkPackList @ 0x0093_9480 => extern "C" fn rcbg_save_chk_pack_list(maps: u32, pack: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_s::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_s::FRAME;
        gs::chk_pack_list(&mut mem(), ebp, maps, pack)
    }
    // ---- round 5 batch 16 (ffx_prender::m5m_t) ----
    /// `rcbgSceneLoadTexture(scene)` -> the last callee's `eax` (the
    /// original's 0x118-byte frame, its file list and path, in this
    /// shim's).
    rcbgSceneLoadTexture @ 0x0093_b6b0 => extern "C" fn rcbg_scene_load_texture(scene: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_t::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_t::FRAME;
        gt::load_texture(&mut mem(), ebp, scene)
    }
    // ---- round 5 batch 17 (ffx_prender::m5m_u) ----
    /// `rcbgGuideMapSaveSub(n, maps, number)` -> `rcbgSceneLoadAll`'s `eax`
    /// (the original's 0x534-byte frame, its scene, names, path and header,
    /// in this shim's).
    rcbgGuideMapSaveSub @ 0x0093_91d0 => extern "C" fn rcbg_guide_map_save_sub(n: u32, maps: u32, number: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_u::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_u::FRAME;
        gu::guide_map_save_sub(&mut mem(), ebp, n, maps, number)
    }
    // ---- round 5 batch 18 (ffx_prender::m5m_v) ----
    /// `rcuiGuideProcConvertAll()` -> `rcdtChunkFini`'s `eax` (the
    /// original's 0x98-byte frame, its chunk and parser, in this shim's).
    rcuiGuideProcConvertAll @ 0x0093_4250 => extern "C" fn rcui_guide_proc_convert_all() -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::m5m_v::FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::m5m_v::FRAME;
        gv::convert_all(&mut mem(), ebp)
    }
    // ---- round 6 (left6) batch 1 (ffx_prender::l6_map) ----
    // The window procedures' `None` (the caller's or the cookie's `eax`)
    // becomes 0: Gu's window dispatch drops `eax` (ffx_prender::l6_map).
    /// `FUN_00932320()` -> `rcuiChrMotionSetup`'s `eax`.
    rcuiChrSelectReset_00932320 @ 0x0093_2320 => extern "C" fn rcui_chr_select_reset() -> u32 {
        l6m::chr_select_reset(&mut mem())
    }
    /// `(window, msg, param)`: the save window's procedure.
    rcuiSaveProc_009364d0 @ 0x0093_64d0 => extern "C" fn rcui_save_proc(window: u32, msg: u32, param: u32) -> u32 {
        l6m::save_proc(&mut mem(), window, msg, param).unwrap_or(0)
    }
    /// `(window, msg)`: the point-simulation window's procedure (the
    /// position vector in a frame).
    rcuiPSimProc_00935710 @ 0x0093_5710 => extern "C" fn rcui_psim_proc(window: u32, msg: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map::PSIM_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map::PSIM_FRAME;
        l6m::psim_proc(&mut mem(), ebp, window, msg).unwrap_or(0)
    }
    /// `(window, msg, param)`: the guide-target window's procedure (the
    /// text in a frame).
    rcuiGuideTargetProc_00934110 @ 0x0093_4110 => extern "C" fn rcui_guide_target_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map::GUIDE_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map::GUIDE_FRAME;
        l6m::guide_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `(window, msg, param)`: the palette window's procedure.
    rcuiPaletteProc @ 0x0093_5820 => extern "C" fn rcui_palette_proc(window: u32, msg: u32, param: u32) -> u32 {
        l6m::palette_proc(&mut mem(), window, msg, param).unwrap_or(0)
    }
    /// `(pkt, points, n, colour, matrix)` -> `pkt` (the matrix, point and
    /// fixed-point pair in a frame).
    rcbgGuidePackPoints_009278d0 @ 0x0092_78d0 => extern "C" fn rcbg_guide_pack_points(pkt: u32, points: u32, n: u32, colour: u32, matrix: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map::PACK_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map::PACK_FRAME;
        l6m::pack_points(&mut mem(), ebp, pkt, points, n, colour, matrix)
    }
    /// `(pkt, matrix, size)` -> `rcPacketCircle`'s `eax` (the colour and
    /// origin in a frame).
    rcbgGuideAxisMarker_0092c600 @ 0x0092_c600 => extern "C" fn rcbg_guide_axis_marker(pkt: u32, matrix: u32, size: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map::AXIS_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map::AXIS_FRAME;
        // junk read: the scale vector's y, z, w (the caller's saved edi, esi,
        // ebx in the original); 0 (brief 2026-10-24)
        l6m::axis_marker(&mut mem(), ebp, pkt, matrix, size, [0; 3])
    }
    // ---- round 6 (left6) batch 1, part 2 (ffx_prender::l6_map2) ----
    /// `(window, msg, param)`: the layer selector's procedure (the number
    /// text in a frame).
    rcuiLayerSelectProc_00936840 @ 0x0093_6840 => extern "C" fn rcui_layer_select_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map2::LAYER_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map2::LAYER_FRAME;
        l6m2::layer_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `(window, msg; slots)`: the FP memory window's procedure (its counts
    /// in a frame; it keeps floats in its `msg` slot).
    rcuiFpMemProc_00933aa0 @ 0x0093_3aa0 => extern "C" fn rcui_fp_mem_proc(_window: u32, msg: u32; slots: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map2::FP_MEM_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map2::FP_MEM_FRAME;
        l6m2::fp_mem_proc(&mut mem(), ebp, slots, msg)
    }
    /// `(window, msg, param; slots)`: the number entry's procedure (the
    /// float it enters goes through its `msg` slot).
    rcuiNumEntryProc_0093c3f0 @ 0x0093_c3f0 => extern "C" fn rcui_num_entry_proc(window: u32, msg: u32, param: u32; slots: u32) -> u32 {
        l6m2::entry_proc(&mut mem(), slots, window, msg, param)
    }
    /// `(window, msg, param)`: the map scroll window's procedure (its
    /// limits and the mouse deltas in a frame).
    rcuiScrollProc_009365c0 @ 0x0093_65c0 => extern "C" fn rcui_scroll_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map2::SCROLL_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map2::SCROLL_FRAME;
        l6m2::scroll_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `rcbgFpSaveSub(name)` -> the last `rcPrint`'s `eax` (the original's
    /// 0x340-byte frame, its path, command, headers and padding, in this
    /// shim's).
    rcbgFpSaveSub @ 0x0093_8350 => extern "C" fn rcbg_fp_save_sub(name: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map2::FP_SAVE_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map2::FP_SAVE_FRAME;
        l6m2::fp_save_sub(&mut mem(), ebp, name)
    }
    // ---- round 6 (left6) batch 2 (ffx_prender::l6_map3) ----
    /// `(pkt, chr)` -> the last callee's `eax` (the original's 0x94-byte
    /// frame, its range, cursors and matrices, in this shim's).
    rcbgGuideSkeleton_0092c250 @ 0x0092_c250 => extern "C" fn rcbg_guide_skeleton(pkt: u32, chr: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map3::SKELETON_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map3::SKELETON_FRAME;
        l6m3::skeleton(&mut mem(), ebp, pkt, chr)
    }
    // ---- round 6 (left6) batch 3 (ffx_prender::l6_map4) ----
    /// The scroll model's per-frame entry (`g_MagicImports + 0xd8c`); the
    /// creation gets a frame address it never reads.
    updateVFXRenderData_ScrollModelEntry_00a5bf50 @ 0x00a5_bf50 => extern "C" fn scroll_model_entry(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32) -> u32 {
        let mut frame = StackFrame::<0x1e>::new();
        let buf = frame.at();
        l6m4::scroll_entry(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16], buf)
    }
    /// `(lo, hi, model, _, d1 lo, d1 hi, d2 lo, d2 hi, mode)` -> 1 or 0
    /// (the original's 0x1f78-byte frame, its counts and texture slots,
    /// in this shim's).
    createVFXRenderDataBuffer_DrawScrollModel @ 0x00a5_c130 => extern "C" fn create_scroll_model(lo: u32, hi: u32, model: u32, _unused: u32, d1lo: u32, d1hi: u32, d2lo: u32, d2hi: u32, mode: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map4::CREATE_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map4::CREATE_FRAME;
        l6m4::create_buffer(&mut mem(), ebp, [lo, hi, model, 0, d1lo, d1hi, d2lo, d2hi, mode])
    }
    /// 17 words -> 1 or 0 (keeps four values in its argument slots).
    updateVFXRenderData_DrawModelScroll @ 0x00a5_c370 => extern "C" fn update_scroll_model(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32, a12: u32, a13: u32, a14: u32, a15: u32, a16: u32; slots: u32) -> u32 {
        l6m4::update(&mut mem(), slots, [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16])
    }
    /// `(scene, parse)`: the scene text's camera line.
    rcbgSceneLoadTextCamera_0093af60 @ 0x0093_af60 => extern "C" fn rcbg_scene_load_text_camera(scene: u32, parse: u32) -> u32 {
        l6m5::camera_line(&mut mem(), scene, parse)
    }
    /// `(window, msg, param; slots)`: the slider window's procedure (the
    /// mouse deltas in a frame; floats in its `msg` slot).
    rcuiSliderProc_0093c860 @ 0x0093_c860 => extern "C" fn rcui_slider_proc(window: u32, msg: u32, param: u32; slots: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map5::bridge::SLIDER_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map5::bridge::SLIDER_FRAME;
        l6m5::slider_proc(&mut mem(), ebp, slots, window, msg, param)
    }
    /// `(window, msg, param)`: the motion test window's procedure (the
    /// speed text in a frame).
    rcuiMotionProc @ 0x0093_5340 => extern "C" fn rcui_motion_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map5::bridge::MOTION_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map5::bridge::MOTION_FRAME;
        l6m5::motion_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `(window, msg, param)`: the layer alpha window's procedure (the
    /// FIX text in a frame).
    rcuiAlphaProc @ 0x0093_1050 => extern "C" fn rcui_alpha_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map5::bridge::ALPHA_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map5::bridge::ALPHA_FRAME;
        l6m5::alpha_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    // ---- round 6 (left6) batch 4 (ffx_prender::l6_map6) ----
    /// `(window, msg, param)`: the map tool's main menu window.
    rcuiMenuProc_00935000 @ 0x0093_5000 => extern "C" fn rcui_menu_proc(window: u32, msg: u32, param: u32) -> u32 {
        l6m6::menu_proc(&mut mem(), window, msg, param).unwrap_or(0)
    }
    /// `rcuiResetChrPos()` -> `Ch_SetPos3`'s `eax` (the original's
    /// 0x5c-byte frame, its vectors and scratch, in this shim's).
    rcuiResetChrPos @ 0x0093_7b70 => extern "C" fn rcui_reset_chr_pos() -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map6::bridge::RESET_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map6::bridge::RESET_FRAME;
        // junk read: the scale vectors' saved esi/ebx/edi words; 0 (brief
        // 2026-10-24)
        l6m6::reset_chr_pos(&mut mem(), ebp, [0; 3])
    }
    /// `(window, msg, param)`: the field effect test window's procedure
    /// (its texts in a frame).
    rcuiFpProc_00933cc0 @ 0x0093_3cc0 => extern "C" fn rcui_fp_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map6::bridge::FP_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map6::bridge::FP_FRAME;
        l6m6::fp_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    // ---- round 6 (left6) batch 6 (ffx_prender::l6_map7) ----
    /// `(window, msg, param)`: the load window's procedure (the original's
    /// 0x11c-byte frame, its path and list layout, in this shim's); the
    /// third word is not read.
    rcuiLoadProc @ 0x0093_4bc0 => extern "C" fn rcui_load_proc(window: u32, msg: u32, _param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map7::LOAD_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map7::LOAD_FRAME;
        l6m7::load_proc(&mut mem(), ebp, window, msg)
    }
    /// `(window, msg, param)`: the layer rectangle window's procedure
    /// (its text and a copied preset in a frame).
    rcuiLayerRectProc_00932a60 @ 0x0093_2a60 => extern "C" fn rcui_layer_rect_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map7::bridge::RECT_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map7::bridge::RECT_FRAME;
        l6m7::rect_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `(window, msg, param)`: the map tool's debug window procedure (its
    /// texts in a frame).
    rcuiDebugProc_009325c0 @ 0x0093_25c0 => extern "C" fn rcui_debug_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map7::bridge::DEBUG_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map7::bridge::DEBUG_FRAME;
        l6m7::debug_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
    /// `(window, msg, param)`: the character window's procedure (its text
    /// in a frame).
    rcuiChrProc_00931e90 @ 0x0093_1e90 => extern "C" fn rcui_chr_proc(window: u32, msg: u32, param: u32) -> u32 {
        let mut frame = StackFrame::<{ (ffx_prender::l6_map7::bridge::CHR_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + ffx_prender::l6_map7::bridge::CHR_FRAME;
        l6m7::chr_proc(&mut mem(), ebp, window, msg, param).unwrap_or(0)
    }
}
