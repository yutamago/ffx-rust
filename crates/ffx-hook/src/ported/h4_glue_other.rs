//! `ffx-glue-other`: the glue outside rendering (M4 port round 1, list
//! `h4-glue-other`): FMOD entry points and callbacks, the post-effect and
//! movie singletons, menu, Iggy and text helpers.
//!
//! Threads: the `i*` sound calls run on the game thread and the loader
//! threads; `FmodMemory_free` on the FMOD thread; the shader-cluster
//! callbacks on the async loader. None keeps state between calls (no
//! frames, no scratch).
//!
//! Floats are passed as their bits (`u32`): the originals only copy them
//! through `fld`/`fstp`, which the bridges model.

use ffx_glue_other::{
    copies as c, effect as e, frames as fr, h4_c1 as c1, h4_c2 as c2, h4_c3 as c3, h4_c4 as c4,
    h4_d1 as d1, h4_d2 as d2, h4_d3 as d3, h4_d4 as d4, h4_d5 as d5, h4_e1 as e1, h4_e2 as e2,
    h4_e3 as e3, h4_e4 as e4, h4_e5 as e5, h4_f1 as f1, h4_f2 as f2, h4_f3 as f3, h4_f4 as f4,
    h4_f5 as f5, h4_f6 as f6, h4_f7 as f7, h4_g1 as g1, h4_g2 as g2, h4_h1 as h1, h4_h2 as h2,
    h4_h3 as h3, h4_h4 as h4, h4_h5 as h5, h4_h6 as h6, h4_h7 as h7, h4_h8 as h8, h4_i1 as i1,
    h4_i2 as i2, h4_i3 as i3, h4_i4 as i4, h4_i5 as i5, h4_i6 as i6, h4_i7 as i7, h4_i8 as i8,
    h4_j1 as j1, h4_j2 as j2, h4_j3 as j3, h4_j4 as j4, h4_j5 as j5, h4_j6 as j6, h4_k1 as k1,
    h4_k2 as k2, h4_k3 as k3, h4_k4 as k4, h4_l1 as l1, h4_l2 as l2, h4_l3 as l3, h4_l4 as l4,
    h4_l5 as l5, h4_m1 as m1, h4_m2 as m2, h4_m3 as m3, h4_m4 as m4, h4_m5 as m5, h4_m6 as m6,
    h4_m7 as m7, h4_m8 as m8, h4_m9 as m9, menu as mn, misc as m, movie as mv, r3, sound as s,
};

use super::{StackFrame, mem, ported};

ported! {
    /// `iSoundInit_0067a500()`.
    iSoundInit_0067a500 @ 0x0067_a500 => extern "C" fn i_sound_init() -> u32 {
        s::i_sound_init(&mut mem())
    }
    /// `iSoundStageFlush(stage)`.
    iSoundStageFlush @ 0x0067_a5d0 => extern "C" fn i_sound_stage_flush(stage: u32) -> u32 {
        s::i_sound_stage_flush(&mut mem(), stage)
    }
    /// `iSfxUnloadAllData()`.
    iSfxUnloadAllData @ 0x0067_a4d0 => extern "C" fn i_sfx_unload_all_data() -> u32 {
        s::i_sfx_unload_all_data(&mut mem())
    }
    /// `iSfxStop(id)`.
    iSfxStop @ 0x0067_a460 => extern "C" fn i_sfx_stop(id: u32) -> u32 {
        s::i_sfx_stop(&mut mem(), id)
    }
    /// `iSfxStopNonBlockingSound(id)`.
    iSfxStopNonBlockingSound @ 0x0067_a4a0 => extern "C" fn i_sfx_stop_non_blocking_sound(id: u32) -> u32 {
        s::i_sfx_stop_non_blocking_sound(&mut mem(), id)
    }
    /// `iSfxStopAllNonBlockingSound(id)`.
    iSfxStopAllNonBlockingSound @ 0x0067_a480 => extern "C" fn i_sfx_stop_all_non_blocking_sound(id: u32) -> u32 {
        s::i_sfx_stop_all_non_blocking_sound(&mut mem(), id)
    }
    /// `iSfxSetVolume(id, volume)`.
    iSfxSetVolume @ 0x0067_a400 => extern "C" fn i_sfx_set_volume(id: u32, volume: u32) -> u32 {
        s::i_sfx_set_volume(&mut mem(), id, volume)
    }
    /// `iSfxSetVolumeTranslate(id, volume, frames)`.
    iSfxSetVolumeTranslate @ 0x0067_a420 => extern "C" fn i_sfx_set_volume_translate(id: u32, volume: u32, frames: u32) -> u32 {
        s::i_sfx_set_volume_translate(&mut mem(), id, volume, frames)
    }
    /// `iSfxStart(id, a, b)`.
    iSfxStart @ 0x0067_a440 => extern "C" fn i_sfx_start(id: u32, a: u32, b: u32) -> u32 {
        s::i_sfx_start(&mut mem(), id, a, b)
    }
    /// `iVoiceStop()`.
    iVoiceStop @ 0x0067_a7a0 => extern "C" fn i_voice_stop() -> u32 {
        s::i_voice_stop(&mut mem())
    }
    /// `iVoiceRead(id)`.
    iVoiceRead @ 0x0067_a720 => extern "C" fn i_voice_read(id: u32) -> u32 {
        s::i_voice_read(&mut mem(), id)
    }
    /// `iVoiceSetPaused(flag)`.
    iVoiceSetPaused @ 0x0067_a750 => extern "C" fn i_voice_set_paused(flag: u32) -> u32 {
        s::i_voice_set_paused(&mut mem(), flag)
    }
    /// `iVoiceDataChange(id)` -> 0.
    iVoiceDataChange @ 0x0067_a6a0 => extern "C" fn i_voice_data_change(id: u32) -> u32 {
        s::i_voice_data_change(&mut mem(), id)
    }
    /// `iVoiceStart(pan, volume)` -> 0.
    iVoiceStart @ 0x0067_a770 => extern "C" fn i_voice_start(pan: u32, volume: u32) -> u32 {
        s::i_voice_start(&mut mem(), pan, volume)
    }
    /// `iVoicePlayStatus()`.
    iVoicePlayStatus @ 0x0067_a710 => extern "C" fn i_voice_play_status() -> u32 {
        s::i_voice_play_status(&mut mem())
    }
    /// `iVoiceIsPaused()`.
    iVoiceIsPaused @ 0x0067_a6c0 => extern "C" fn i_voice_is_paused() -> u32 {
        s::i_voice_is_paused(&mut mem())
    }
    /// `FmodManager_signalVoiceStart(mgr)`.
    FmodManager_signalVoiceStart @ 0x006f_a6c0 => extern "thiscall" fn fmod_manager_signal_voice_start(mgr: u32) -> u32 {
        s::fmod_manager_signal_voice_start(&mut mem(), mgr)
    }
    /// `iSyncInit()`.
    iSyncInit @ 0x0067_a950 => extern "C" fn i_sync_init() -> u32 {
        s::i_sync_init(&mut mem())
    }
    /// `iSyncDataChange(id)`.
    iSyncDataChange @ 0x0067_a8e0 => extern "C" fn i_sync_data_change(id: u32) -> u32 {
        s::i_sync_data_change(&mut mem(), id)
    }
    /// `iSyncGetData(a, b, c)`.
    iSyncGetData @ 0x0067_a930 => extern "C" fn i_sync_get_data(a: u32, b: u32, c: u32) -> u32 {
        s::i_sync_get_data(&mut mem(), a, b, c)
    }
    /// `SyncManager_SyncManager(this)` -> `this`.
    SyncManager_SyncManager @ 0x006f_a8b0 => extern "thiscall" fn sync_manager_ctor(this: u32) -> u32 {
        s::sync_manager_ctor(&mut mem(), this)
    }
    /// `FmodMusic_getState(id)`.
    FmodMusic_getState @ 0x0070_8be0 => extern "C" fn fmod_music_get_state(id: u32) -> u32 {
        s::fmod_music_get_state(&mem(), id)
    }
    /// `FmodMemory_free(p, type, source)` (FMOD's free callback).
    FmodMemory_free @ 0x0070_6710 => extern "stdcall" fn fmod_memory_free(p: u32, _kind: u32, _source: u32) {
        s::fmod_memory_free(&mut mem(), p);
    }
    /// `PVideoPlaybackWin32_setVolume(this; volume)`.
    PVideoPlaybackWin32_setVolume @ 0x00a2_8330 => extern "thiscall" fn video_set_volume(this: u32, volume: u32) -> u32 {
        s::video_set_volume(&mut mem(), this, volume)
    }
    /// `iMusicRefreshBgmType()`.
    iMusicRefreshBgmType @ 0x008c_c170 => extern "C" fn i_music_refresh_bgm_type() -> u32 {
        s::i_music_refresh_bgm_type(&mut mem())
    }

    /// `Distortion_GetInstance()`.
    Distortion_GetInstance @ 0x006c_2290 => extern "C" fn distortion_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::DISTORTION)
    }
    /// `BlurEffect_GetInstance()`.
    BlurEffect_GetInstance @ 0x006c_3580 => extern "C" fn blur_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::BLUR_EFFECT)
    }
    /// `BlurEffectEx_GetInstance()`.
    BlurEffectEx_GetInstance @ 0x006c_4a70 => extern "C" fn blur_ex_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::BLUR_EFFECT_EX)
    }
    /// `MotionBlur_GetInstance()`.
    MotionBlur_GetInstance @ 0x006c_5130 => extern "C" fn motion_blur_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::MOTION_BLUR)
    }
    /// `BrokenScreenEffect_GetInstance()`.
    BrokenScreenEffect_GetInstance @ 0x006d_4de0 => extern "C" fn broken_screen_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::BROKEN_SCREEN)
    }
    /// `PhyFMVPlayerManager_GetInstance()`.
    PhyFMVPlayerManager_GetInstance @ 0x006d_73a0 => extern "C" fn fmv_get_instance() -> u32 {
        e::get_instance(&mut mem(), e::FMV_PLAYER_MANAGER)
    }
    /// `BrokenScreenEffect_resize(this; w, h)` -> 0.
    BrokenScreenEffect_resize @ 0x006d_59c0 => extern "thiscall" fn broken_screen_resize(this: u32, w: u32, h: u32) -> u32 {
        e::resize(&mut mem(), this, 0, w, h)
    }
    /// `Distortion_resize(this; w, h)` -> 0.
    Distortion_resize @ 0x006c_2fd0 => extern "thiscall" fn distortion_resize(this: u32, w: u32, h: u32) -> u32 {
        e::resize(&mut mem(), this, 0x28, w, h)
    }
    /// `RadialBlur_resize_007035f0(this; w, h)` -> 0.
    RadialBlur_resize_007035f0 @ 0x0070_35f0 => extern "thiscall" fn radial_blur_resize(this: u32, w: u32, h: u32) -> u32 {
        e::resize(&mut mem(), this, 0x68, w, h)
    }
    /// `BrokenScreenEffect_setOpacity(this; opacity)`.
    BrokenScreenEffect_setOpacity @ 0x006d_59e0 => extern "thiscall" fn broken_screen_set_opacity(this: u32, opacity: u32) {
        e::broken_screen_set_opacity(&mut mem(), this, opacity);
    }
    /// `graphicBrokenScreenInit_00639450(n, buf)`.
    graphicBrokenScreenInit_00639450 @ 0x0063_9450 => extern "C" fn graphic_broken_screen_init(n: u32, buf: u32) -> u32 {
        e::graphic_broken_screen_init(&mut mem(), n, buf)
    }
    /// `graphicChrDelayReleaseStep_006442d0(dt)`.
    graphicChrDelayReleaseStep_006442d0 @ 0x0064_42d0 => extern "C" fn chr_delay_release_step(dt: u32) -> u32 {
        e::chr_delay_release_step(&mut mem(), dt)
    }
    /// `BlurEffect_onShaderClusterLoaded_006c3950(cluster, &blur)`.
    BlurEffect_onShaderClusterLoaded_006c3950 @ 0x006c_3950 => extern "C" fn blur_on_shader_cluster_loaded(cluster: u32, object: u32) -> u32 {
        e::on_shader_cluster_loaded(&mut mem(), cluster, object, e::BLUR_CLUSTER, e::BLUR_ON_LOADED)
    }
    /// `PhyFMVPlayerManager_onShaderClusterLoaded(cluster, &manager)`.
    PhyFMVPlayerManager_onShaderClusterLoaded @ 0x006d_a460 => extern "C" fn fmv_on_shader_cluster_loaded(cluster: u32, object: u32) -> u32 {
        e::on_shader_cluster_loaded(&mut mem(), cluster, object, e::FMV_CLUSTER, e::FMV_ON_LOADED)
    }

    /// `PhyreFIOS_UnloadMagicfilePrx(handle)`.
    PhyreFIOS_UnloadMagicfilePrx @ 0x0067_94f0 => extern "C" fn unload_magicfile_prx(handle: u32) -> u32 {
        m::unload_magicfile_prx(&mut mem(), handle)
    }
    /// `ClusterManager_loadPClusterJump_0069bb00(this; path)`.
    ClusterManager_loadPClusterJump_0069bb00 @ 0x0069_bb00 => extern "thiscall" fn load_pcluster_jump(this: u32, path: u32) -> u32 {
        m::load_pcluster_jump(&mut mem(), this, path)
    }
    /// `MsSaveAddAPJump_008ba390(chr, n)`.
    MsSaveAddAPJump_008ba390 @ 0x008b_a390 => extern "C" fn save_add_ap_jump(chr: u32, n: u32) -> u32 {
        m::save_add_ap_jump(&mut mem(), chr, n)
    }
    /// `IggyMenuManager_SetViewportJump_00a26960(this; w, h)`.
    IggyMenuManager_SetViewportJump_00a26960 @ 0x00a2_6960 => extern "thiscall" fn set_viewport_jump(this: u32, w: u32, h: u32) -> u32 {
        m::set_viewport_jump(&mut mem(), this, w, h)
    }
    /// `AsyncLoadManager_RegistTaskRequest_run_00691350(this)`.
    AsyncLoadManager_RegistTaskRequest_run_00691350 @ 0x0069_1350 => extern "thiscall" fn regist_task_request_run(this: u32) -> u32 {
        m::regist_task_request_run(&mut mem(), this)
    }
    /// `TOMenuDrawKickTmp()`.
    TOMenuDrawKickTmp @ 0x008b_ea20 => extern "C" fn menu_draw_kick_tmp() -> u32 {
        m::menu_draw_kick_tmp(&mut mem())
    }
    /// `movie_err()`.
    movie_err @ 0x0076_efa0 => extern "C" fn movie_err() {
        m::movie_message(&mut mem(), m::MOVIE_ERR);
    }
    /// `movie_init()`.
    movie_init @ 0x0076_eff0 => extern "C" fn movie_init() {
        m::movie_message(&mut mem(), m::MOVIE_INIT);
    }
    /// `movie_start_prog()`.
    movie_start_prog @ 0x0076_f080 => extern "C" fn movie_start_prog() {
        m::movie_message(&mut mem(), m::MOVIE_START_PROG);
    }
    /// `movie_stop_prog()`.
    movie_stop_prog @ 0x0076_f090 => extern "C" fn movie_stop_prog() {
        m::movie_message(&mut mem(), m::MOVIE_STOP_PROG);
    }
    /// `Sg_MFIFOSpadSync()`.
    Sg_MFIFOSpadSync @ 0x0084_32d0 => extern "C" fn mfifo_spad_sync() -> u32 {
        m::mfifo_spad_sync(&mem())
    }
    /// `Sg_GsSync()`.
    Sg_GsSync @ 0x0084_1d60 => extern "C" fn gs_sync() -> u32 {
        m::gs_sync(&mut mem())
    }
    /// `SaveDataMachine_disableMenu_00702a70()`.
    SaveDataMachine_disableMenu_00702a70 @ 0x0070_2a70 => extern "C" fn save_menu_disable() -> u32 {
        m::save_menu_disable(&mut mem())
    }
    /// `IggyMenuHandlers_callCdecl_00a28c00(ctx, f, arg)`.
    IggyMenuHandlers_callCdecl_00a28c00 @ 0x00a2_8c00 => extern "C" fn iggy_call_cdecl(_ctx: u32, f: u32, arg: u32) -> u32 {
        m::iggy_call_cdecl(&mut mem(), f, arg)
    }
    /// `FFEscMenu_callMember_0068dfa0(this, f, arg)`.
    FFEscMenu_callMember_0068dfa0 @ 0x0068_dfa0 => extern "C" fn esc_menu_call_member(this: u32, f: u32, arg: u32) -> u32 {
        m::esc_menu_call_member(&mut mem(), this, f, arg)
    }
    /// `RemapButtonIcon_getCurrentControlMachine()`.
    RemapButtonIcon_getCurrentControlMachine @ 0x006e_6450 => extern "C" fn current_control_machine() -> u32 {
        m::current_control_machine(&mut mem())
    }
    /// `TextUtil_setEnabled(this; on)`.
    TextUtil_setEnabled @ 0x006f_68f0 => extern "thiscall" fn text_set_enabled(this: u32, on: u32) {
        m::text_set_enabled(&mut mem(), this, on);
    }
    /// `FFEscMenu_rebuildVideoSettingPage_006ff020(this)`.
    FFEscMenu_rebuildVideoSettingPage_006ff020 @ 0x006f_f020 => extern "thiscall" fn rebuild_video_setting_page(this: u32) -> u32 {
        m::rebuild_video_setting_page(&mut mem(), this)
    }
    /// `crossThreadImpl_run_006f29f0(this)` -> 0.
    crossThreadImpl_run_006f29f0 @ 0x006f_29f0 => extern "thiscall" fn cross_thread_run(this: u32) -> u32 {
        m::cross_thread_run(&mut mem(), this)
    }
    /// `IggyValueRef_free_00a28a20(this)`.
    IggyValueRef_free_00a28a20 @ 0x00a2_8a20 => extern "thiscall" fn iggy_value_ref_free(this: u32) {
        m::iggy_value_ref_free(&mut mem(), this);
    }
    /// `ClassDynamicGeometry_hide(this)`.
    ClassDynamicGeometry_hide @ 0x006a_2370 => extern "thiscall" fn geometry_hide(this: u32) -> u32 {
        m::geometry_set_visible(&mut mem(), this, 0)
    }
    /// `ClassDynamicGeometry_show(this)`.
    ClassDynamicGeometry_show @ 0x006a_49e0 => extern "thiscall" fn geometry_show(this: u32) -> u32 {
        m::geometry_set_visible(&mut mem(), this, 1)
    }
    /// `MenuManager_GetMenuCount(this)`.
    MenuManager_GetMenuCount @ 0x0067_b970 => extern "thiscall" fn menu_count(this: u32) -> u32 {
        m::menu_count(&mem(), this)
    }
    /// `IggyValue_setType9_00a28dd0(value, v)`.
    IggyValue_setType9_00a28dd0 @ 0x00a2_8dd0 => extern "C" fn iggy_value_set_type9(value: u32, v: u32) {
        m::iggy_value_set_type9(&mut mem(), value, v);
    }
    /// `graphicInitialize_allocOnce_006e5490()`.
    graphicInitialize_allocOnce_006e5490 @ 0x006e_5490 => extern "C" fn alloc_once() {
        m::alloc_once(&mut mem());
    }
    /// `TextUtil_setPosition_006f6910(this; x, y)`.
    TextUtil_setPosition_006f6910 @ 0x006f_6910 => extern "thiscall" fn text_set_position(this: u32, x: u32, y: u32) {
        m::text_set_position(&mut mem(), this, x, y);
    }
    /// `IggyMenu_addCallback_00a28e90(this; a, b)`.
    IggyMenu_addCallback_00a28e90 @ 0x00a2_8e90 => extern "thiscall" fn iggy_menu_add_callback(this: u32, a: u32, b: u32) -> u32 {
        m::iggy_menu_add_callback(&mut mem(), this, a, b)
    }
    /// `TextUtil_setTitleText(this; s)`.
    TextUtil_setTitleText @ 0x006f_6930 => extern "thiscall" fn text_set_title(this: u32, s: u32) -> u32 {
        m::text_set_title(&mut mem(), this, s)
    }
    /// `VideoText_render_006ddeb0(this; a)`.
    VideoText_render_006ddeb0 @ 0x006d_deb0 => extern "thiscall" fn video_text_render(this: u32, a: u32) {
        m::video_text_render(&mut mem(), this, a);
    }
    /// `TOGetFontWidthInterMode(a, b, c)`.
    TOGetFontWidthInterMode @ 0x008b_7be0 => extern "C" fn font_width_inter_mode(a: u32, b: u32, c: u32) -> u32 {
        m::font_width_inter_mode(&mut mem(), a, b, c)
    }
    /// `graphicResetSplitScreen_00638d20()`.
    graphicResetSplitScreen_00638d20 @ 0x0063_8d20 => extern "C" fn reset_split_screen() -> u32 {
        m::reset_split_screen(&mut mem())
    }
    /// `AsyncLoadManager_waitTasks_00691d40(this)`.
    AsyncLoadManager_waitTasks_00691d40 @ 0x0069_1d40 => extern "thiscall" fn wait_tasks(this: u32) {
        m::wait_tasks(&mut mem(), this);
    }
    /// `TOMkpSetEnvSubFont()`.
    TOMkpSetEnvSubFont @ 0x0090_3300 => extern "C" fn set_env_sub_font() -> u32 {
        m::set_env_sub_font(&mut mem())
    }

    // Batch 2: FMOD file callbacks, the movie manager, menus and text, PS2
    // packet helpers, and exact copies of other modules' ports.
    /// `eventcallback(event, type, a, b, out)` (FMOD's event callback).
    eventcallback @ 0x0070_7a50 => extern "stdcall" fn event_callback(_event: u32, kind: u32, _a: u32, _b: u32, out: u32) -> u32 {
        s::event_callback(&mut mem(), kind, out)
    }
    /// `PhyreFIOSStreamFile_seek(this; pos, whence)`.
    PhyreFIOSStreamFile_seek @ 0x006f_9d80 => extern "thiscall" fn fios_seek(this: u32, pos: u32, whence: u32) -> u32 {
        s::fios_seek(&mut mem(), this, pos, whence)
    }
    /// `FmodFile_seek(handle, pos, user)`.
    FmodFile_seek @ 0x0070_85e0 => extern "stdcall" fn fmod_file_seek(handle: u32, pos: u32, _user: u32) -> u32 {
        s::fmod_file_seek(&mut mem(), handle, pos)
    }
    /// `FmodFile_close(handle, user)`.
    FmodFile_close @ 0x0070_84d0 => extern "stdcall" fn fmod_file_close(handle: u32, _user: u32) -> u32 {
        s::fmod_file_close(&mut mem(), handle)
    }
    /// `FmodSfx_anyEventFlag_0070ed30(list)` (`al`).
    FmodSfx_anyEventFlag_0070ed30 @ 0x0070_ed30 => extern "thiscall" fn sfx_any_event_flag(list: u32) -> u8 {
        s::sfx_any_event_flag(&mem(), list)
    }

    /// `movie_with3DScene()`.
    movie_with3DScene @ 0x0076_f0c0 => extern "C" fn movie_with_3d_scene() -> u32 {
        mv::with_3d_scene(&mut mem())
    }
    /// `movie_have_camera()`.
    movie_have_camera @ 0x0076_efc0 => extern "C" fn movie_have_camera() -> u32 {
        mv::have_camera(&mut mem())
    }
    /// `PhyFMVPlayerManager_fmvPlayer_terminate(this)`.
    PhyFMVPlayerManager_fmvPlayer_terminate @ 0x006d_9180 => extern "thiscall" fn fmv_terminate(this: u32) -> u32 {
        mv::fmv_terminate(&mut mem(), this)
    }
    /// `PhyFMVPlayerManager_pauseMovie(this)`.
    PhyFMVPlayerManager_pauseMovie @ 0x006d_a4b0 => extern "thiscall" fn fmv_pause(this: u32) {
        mv::fmv_pause(&mut mem(), this);
    }
    /// `PhyFMVPlayerManager_resumeMovie()`.
    PhyFMVPlayerManager_resumeMovie @ 0x006d_8f80 => extern "C" fn fmv_resume() -> u32 {
        mv::fmv_resume(&mut mem())
    }
    /// `TreeHead_new_006d78c0()`.
    TreeHead_new_006d78c0 @ 0x006d_78c0 => extern "C" fn tree_head_new_6d78c0() -> u32 {
        mv::tree_head_new(&mut mem())
    }
    /// `TreeHead_new_006a0360()`.
    TreeHead_new_006a0360 @ 0x006a_0360 => extern "C" fn tree_head_new_6a0360() -> u32 {
        mv::tree_head_new(&mut mem())
    }

    /// `FFEscMenu_addHandler_00702570(this; i)`.
    FFEscMenu_addHandler_00702570 @ 0x0070_2570 => extern "thiscall" fn esc_add_handler(this: u32, i: u32) -> u32 {
        mn::esc_add_handler(&mut mem(), this, i)
    }
    /// `ClassDynamicGeometry_applyBuffer_006a46c0(this)` -> 0.
    ClassDynamicGeometry_applyBuffer_006a46c0 @ 0x006a_46c0 => extern "thiscall" fn geometry_apply_buffer(this: u32) -> u32 {
        mn::geometry_apply_buffer(&mut mem(), this)
    }
    /// `IggyMenu_Draw_00a28eb0(this)`.
    IggyMenu_Draw_00a28eb0 @ 0x00a2_8eb0 => extern "thiscall" fn iggy_menu_draw(this: u32) -> u32 {
        mn::iggy_menu_draw(&mut mem(), this)
    }
    /// `PCluster_createInRoot_006f7a00(name)` (`ecx` passed on to
    /// `Phyre_PCluster_create`).
    PCluster_createInRoot_006f7a00 @ 0x006f_7a00 => extern "thiscall" fn cluster_create_in_root(ecx: u32, name: u32) -> u32 {
        mn::cluster_create_in_root(&mut mem(), ecx, name)
    }
    /// `AsyncLoadManager_LoadClusterRequest_run_00691320(this)`.
    AsyncLoadManager_LoadClusterRequest_run_00691320 @ 0x0069_1320 => extern "thiscall" fn load_cluster_request_run(this: u32) -> u32 {
        mn::load_cluster_request_run(&mut mem(), this)
    }
    /// `MotionBlur_onShaderClusterLoaded_006c53e0(cluster, &args)`.
    MotionBlur_onShaderClusterLoaded_006c53e0 @ 0x006c_53e0 => extern "C" fn motion_blur_on_loaded(cluster: u32, args: u32) -> u32 {
        mn::motion_blur_on_loaded(&mut mem(), cluster, args)
    }
    /// `graphicVideoPlayIfReady_007185e0()`.
    graphicVideoPlayIfReady_007185e0 @ 0x0071_85e0 => extern "C" fn video_play_if_ready() -> u32 {
        mn::video_play_if_ready(&mut mem())
    }
    /// `TextUtil_onTextUtilAssetLoaded(asset, &slot)`.
    TextUtil_onTextUtilAssetLoaded @ 0x006f_63f0 => extern "C" fn text_on_asset_loaded(asset: u32, slot: u32) -> u32 {
        mn::text_on_asset_loaded(&mut mem(), asset, slot)
    }
    /// `IconPanel_create_006f77f0()`.
    IconPanel_create_006f77f0 @ 0x006f_77f0 => extern "C" fn icon_panel_create() -> u32 {
        mn::icon_panel_create(&mut mem())
    }
    /// `PhyreScene_chrLoadCountdown_00782fa0()`.
    PhyreScene_chrLoadCountdown_00782fa0 @ 0x0078_2fa0 => extern "C" fn chr_load_countdown() {
        mn::chr_load_countdown(&mut mem());
    }
    /// `TextUtil_changeLine(this; i, s)`.
    TextUtil_changeLine @ 0x006f_6070 => extern "thiscall" fn text_change_line(this: u32, i: u32, s: u32) -> u32 {
        mn::text_change_line(&mut mem(), this, i, s)
    }
    /// `IggyMenuManager_createInstance()`.
    IggyMenuManager_createInstance @ 0x00a2_6240 => extern "C" fn iggy_menu_manager_create() {
        mn::create_instance(&mut mem(), mn::IGGY_MENU_MANAGER_NEW);
    }
    /// `RemapButtonIcon_sCreatInstance()`.
    RemapButtonIcon_sCreatInstance @ 0x006e_76c0 => extern "C" fn remap_button_icon_create() {
        mn::create_instance(&mut mem(), mn::REMAP_BUTTON_ICON);
    }
    /// `PostEffect_createInstance_006c5790()`.
    PostEffect_createInstance_006c5790 @ 0x006c_5790 => extern "C" fn post_effect_create() {
        mn::create_instance(&mut mem(), mn::POST_EFFECT);
    }
    /// `IconPanel_setVisible(this; on)`.
    IconPanel_setVisible @ 0x006f_7940 => extern "thiscall" fn icon_panel_set_visible(this: u32, on: u32) {
        mn::icon_panel_set_visible(&mut mem(), this, on);
    }
    /// `IconPanel_hideAllIcons(this)`.
    IconPanel_hideAllIcons @ 0x006f_78d0 => extern "thiscall" fn icon_panel_hide_all(this: u32) {
        mn::icon_panel_hide_all(&mut mem(), this);
    }
    /// `MenuDynamicGeometry_resetMaterials(this)`.
    MenuDynamicGeometry_resetMaterials @ 0x006a_2c60 => extern "thiscall" fn reset_materials(this: u32) -> u32 {
        mn::reset_materials(&mut mem(), this)
    }
    /// `BrokenScreenEffect_getEntryWord_006d5190(this; i, k)`.
    BrokenScreenEffect_getEntryWord_006d5190 @ 0x006d_5190 => extern "thiscall" fn broken_screen_entry_word(this: u32, i: u32, k: u32) -> u32 {
        mn::broken_screen_entry_word(&mem(), this, i, k)
    }
    /// `CheckEncounterKey()`.
    CheckEncounterKey @ 0x006f_75c0 => extern "C" fn check_encounter_key() -> u32 {
        mn::check_encounter_key(&mut mem())
    }
    /// `LocString_ToValue_0068e270(this; out)` -> `out`.
    LocString_ToValue_0068e270 @ 0x0068_e270 => extern "thiscall" fn loc_string_to_value(this: u32, out: u32) -> u32 {
        mn::loc_string_to_value(&mut mem(), this, out)
    }
    /// `PArray_destroy_006b01f0(array, n)`.
    PArray_destroy_006b01f0 @ 0x006b_01f0 => extern "C" fn parray_destroy_20(array: u32, n: u32) {
        mn::parray_destroy(&mut mem(), array, n, 0x20);
    }
    /// `PArray_destroy_006b0230(array, n)`.
    PArray_destroy_006b0230 @ 0x006b_0230 => extern "C" fn parray_destroy_40(array: u32, n: u32) {
        mn::parray_destroy(&mut mem(), array, n, 0x40);
    }
    /// `graphicInitialize_allocOnce2_006e56a0()`.
    graphicInitialize_allocOnce2_006e56a0 @ 0x006e_56a0 => extern "C" fn alloc_once_2() {
        mn::alloc_once_2(&mut mem());
    }

    /// `TOInitScreen_008fae20()`.
    TOInitScreen_008fae20 @ 0x008f_ae20 => extern "C" fn to_init_screen() -> u32 {
        m::to_init_screen(&mut mem())
    }
    /// `TOAddPktMenuIcon(a, b, c, d)`.
    TOAddPktMenuIcon @ 0x008e_b240 => extern "C" fn add_pkt_menu_icon(a: u32, b: u32, c: u32, d: u32) -> u32 {
        m::add_pkt_menu_icon(&mut mem(), a, b, c, d)
    }
    /// `TOMkpShapeXYC2(a, b, c, d, e)`.
    TOMkpShapeXYC2 @ 0x0090_3a50 => extern "C" fn mkp_shape_xyc2(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        m::mkp_shape_xyc2(&mut mem(), a, b, c, d, e)
    }
    /// `rcPacketDataDraw(packet)`.
    rcPacketDataDraw @ 0x0092_7600 => extern "C" fn rc_packet_data_draw(packet: u32) -> u32 {
        m::rc_packet_data_draw(&mut mem(), packet)
    }
    /// `Sg_MFIFOEnd()`.
    Sg_MFIFOEnd @ 0x0084_3260 => extern "C" fn mfifo_end() -> u32 {
        m::mfifo_end(&mut mem())
    }

    /// `PhyreCursor_IsValid_copy_00637f80(this)`: the same code as
    /// `PhyreCursor_IsValid_00441e10` (tools/dupes.py).
    PhyreCursor_IsValid_copy_00637f80 @ 0x0063_7f80 => extern "thiscall" fn cursor_valid_637f80(this: u32) -> u8 {
        ffx_graphic::managers::bridge::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_006f63b0(this)`: the same code.
    PhyreCursor_IsValid_006f63b0 @ 0x006f_63b0 => extern "thiscall" fn cursor_valid_6f63b0(this: u32) -> u8 {
        ffx_graphic::managers::bridge::cursor_valid(&mem(), this)
    }
    /// `TaggedTreeInsert_0069dae0(this; node)`: the same code as
    /// `TaggedTreeInsert_0060c990`.
    TaggedTreeInsert_0069dae0 @ 0x0069_dae0 => extern "thiscall" fn tagged_tree_insert_69dae0(this: u32, node: u32) -> u32 {
        ffx_system::stl::bridge::tagged_tree_insert(&mut mem(), this, node)
    }
    /// `PArray_PushInto_006a2670(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_006a2670 @ 0x006a_2670 => extern "thiscall" fn parray_push_into_6a2670(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, c::phyre_free)
    }
    /// `PArray_PushInto_006a2700(this; dst, value)`: the same code.
    PArray_PushInto_006a2700 @ 0x006a_2700 => extern "thiscall" fn parray_push_into_6a2700(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, c::phyre_free)
    }
    /// `vfxIntTree_Insert_0070bb20(this; node)`: the same code as
    /// `vfxIntTree_Insert`.
    vfxIntTree_Insert_0070bb20 @ 0x0070_bb20 => extern "thiscall" fn vfx_int_tree_insert_70bb20(this: u32, node: u32) -> u32 {
        ffx_vfx::tree::bridge::vfx_int_tree_insert(&mut mem(), this, node)
    }
    /// `vfxIntTree_Insert_0070bba0(this; node)`: the same code.
    vfxIntTree_Insert_0070bba0 @ 0x0070_bba0 => extern "thiscall" fn vfx_int_tree_insert_70bba0(this: u32, node: u32) -> u32 {
        ffx_vfx::tree::bridge::vfx_int_tree_insert(&mut mem(), this, node)
    }
    /// `vfxIntTree_Insert_00710540(this; node)`: the same code.
    vfxIntTree_Insert_00710540 @ 0x0071_0540 => extern "thiscall" fn vfx_int_tree_insert_710540(this: u32, node: u32) -> u32 {
        ffx_vfx::tree::bridge::vfx_int_tree_insert(&mut mem(), this, node)
    }
    /// `SfxEventTree_Find_0070ba20(tree; &key)`: the same code as
    /// `SfxEventTree_Find`.
    SfxEventTree_Find_0070ba20 @ 0x0070_ba20 => extern "thiscall" fn sfx_event_tree_find_70ba20(tree: u32, key: u32) -> u32 {
        let m = mem();
        ffx_sound::ffxfmod::bridge::sfx_find(&m, tree, ffx_bridge::Mem::read(&m, key))
    }
    /// `SfxEventTree_Find_007104c0(tree; &key)`: the same code.
    SfxEventTree_Find_007104c0 @ 0x0071_04c0 => extern "thiscall" fn sfx_event_tree_find_7104c0(tree: u32, key: u32) -> u32 {
        let m = mem();
        ffx_sound::ffxfmod::bridge::sfx_find(&m, tree, ffx_bridge::Mem::read(&m, key))
    }
    // Batch 3: FMOD memory/read callbacks and event pauses, the FMV
    // manager's state and volume, menu records and updates, Iggy value
    // references and handler records, map nodes, post-effect setters.
    /// `FmodMemory_alloc(size, type, source)` (FMOD's alloc callback).
    FmodMemory_alloc @ 0x0070_66d0 => extern "stdcall" fn fmod_memory_alloc(size: u32, _kind: u32, _source: u32) -> u32 {
        s::fmod_memory_alloc(&mut mem(), size)
    }
    /// `FmodFile_read(handle, buf, size, read, user)`.
    FmodFile_read @ 0x0070_85a0 => extern "stdcall" fn fmod_file_read(handle: u32, buf: u32, size: u32, read: u32, _user: u32) -> u32 {
        s::fmod_file_read(&mut mem(), handle, buf, size, read)
    }
    /// `SfxEvent_setPaused(list; paused)`.
    SfxEvent_setPaused @ 0x0070_f100 => extern "thiscall" fn sfx_event_set_paused(list: u32, paused: u32) {
        s::sfx_event_set_paused(&mut mem(), list, paused);
    }
    /// `PhyFMVPlayerManager_hasState_006da1f0(this; v)`.
    PhyFMVPlayerManager_hasState_006da1f0 @ 0x006d_a1f0 => extern "thiscall" fn fmv_has_state(this: u32, v: u32) -> u32 {
        mv::fmv_has_state(&mem(), this, v)
    }
    /// `PhyFMVPlayerManager_StateCheck_006da2a0(this)` (`al`).
    PhyFMVPlayerManager_StateCheck_006da2a0 @ 0x006d_a2a0 => extern "thiscall" fn fmv_state_check(this: u32) -> u8 {
        mv::fmv_state_check(&mem(), this)
    }
    /// `PhyFMVPlayerManager_SetVolume_006dabf0(volume)`.
    PhyFMVPlayerManager_SetVolume_006dabf0 @ 0x006d_abf0 => extern "C" fn fmv_set_volume(volume: u32) {
        mv::fmv_set_volume(&mut mem(), volume);
    }
    /// `TreeNode_new_006d78f0(left, right)`.
    TreeNode_new_006d78f0 @ 0x006d_78f0 => extern "stdcall" fn tree_node_new(left: u32, right: u32) -> u32 {
        mv::tree_node_new(&mut mem(), left, right)
    }
    /// `IconPanel_hideIcon(this; i)`.
    IconPanel_hideIcon @ 0x006f_7900 => extern "thiscall" fn icon_panel_hide_icon(this: u32, i: u32) {
        mn::icon_panel_hide_icon(&mut mem(), this, i);
    }
    /// `IggyValueRef_create_00a288b0(this; player, temp)` -> `this`.
    IggyValueRef_create_00a288b0 @ 0x00a2_88b0 => extern "thiscall" fn iggy_value_ref_create(this: u32, player: u32, temp: u32) -> u32 {
        mn::iggy_value_ref_create(&mut mem(), this, player, temp)
    }
    /// `IggyMenuHandlers_flush_00a28ff0(this)`.
    IggyMenuHandlers_flush_00a28ff0 @ 0x00a2_8ff0 => extern "thiscall" fn iggy_handlers_flush(this: u32) {
        mn::iggy_handlers_flush(&mut mem(), this);
    }
    /// `RemapButtonIcon_remapButtonIcon(this; a, b, c, d, e)`.
    RemapButtonIcon_remapButtonIcon @ 0x006e_69e0 => extern "thiscall" fn remap_button_icon(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        mn::remap_button_icon(&mut mem(), this, [a, b, c, d, e])
    }
    /// `MenuManager_UpdateMenu(this)`.
    MenuManager_UpdateMenu @ 0x0067_faa0 => extern "thiscall" fn menu_update(this: u32) -> u32 {
        mn::menu_update(&mut mem(), this)
    }
    /// `MenuManager_GetMenuRecord_0067b990(this; i)`.
    MenuManager_GetMenuRecord_0067b990 @ 0x0067_b990 => extern "thiscall" fn menu_record(this: u32, i: u32) -> u32 {
        mn::menu_record(&mem(), this, i)
    }
    /// `IggyMenuManager_removeMenu_00a26c50(this; menu)`.
    IggyMenuManager_removeMenu_00a26c50 @ 0x00a2_6c50 => extern "thiscall" fn iggy_manager_remove_menu(this: u32, menu: u32) {
        mn::iggy_manager_remove_menu(&mut mem(), this, menu);
    }
    /// `TreeHead_copy_0069a1e0(this)`.
    TreeHead_copy_0069a1e0 @ 0x0069_a1e0 => extern "thiscall" fn tree_head_copy(this: u32) -> u32 {
        mn::tree_head_copy(&mut mem(), this)
    }
    /// `TreeNode_newWithKey_0069c580(this; key)`.
    TreeNode_newWithKey_0069c580 @ 0x0069_c580 => extern "thiscall" fn tree_node_new_with_key(this: u32, key: u32) -> u32 {
        mn::tree_node_new_with_key(&mut mem(), this, key)
    }
    /// `PhyreScene_setColor4_00638840(this; v)`.
    PhyreScene_setColor4_00638840 @ 0x0063_8840 => extern "thiscall" fn scene_set_color4(this: u32, v: u32) {
        e::scene_set_color4(&mut mem(), this, v);
    }
    /// `MotionBlur_setMotionBlurWeight(this; w)`.
    MotionBlur_setMotionBlurWeight @ 0x006c_5600 => extern "thiscall" fn motion_blur_set_weight(this: u32, w: u32) {
        e::motion_blur_set_weight(&mut mem(), this, w);
    }
    /// `PostEffect_PostEffect_006c5670(this)` -> `this`.
    PostEffect_PostEffect_006c5670 @ 0x006c_5670 => extern "thiscall" fn post_effect_ctor(this: u32) -> u32 {
        e::post_effect_ctor(&mut mem(), this)
    }
    /// `TOMkpShapeXY(a, b, c)`.
    TOMkpShapeXY @ 0x0090_3a10 => extern "C" fn mkp_shape_xy(a: u32, b: u32, c: u32) -> u32 {
        m::mkp_shape_xy(&mut mem(), a, b, c)
    }
    /// `ThreadCall_run_00703910(this)`.
    ThreadCall_run_00703910 @ 0x0070_3910 => extern "thiscall" fn thread_call_run(this: u32) -> u32 {
        m::thread_call_run(&mut mem(), this)
    }
    /// `TextUtil_animate_006f69f0(this; dt)`.
    TextUtil_animate_006f69f0 @ 0x006f_69f0 => extern "thiscall" fn text_animate(this: u32, dt: u32) {
        m::text_animate(&mut mem(), this, dt);
    }
    // Batch 4: FMOD music data and word arrays, the volume scale, the FMV
    // initialize, IggyMenu's destructor, the escape menu's resume and
    // dialog text, the save pop-up, post-effect updates and resizes, the
    // scene cursor.
    /// `SfxEvent_volumeFrom127_0070ea20(v)` (`ST0`).
    SfxEvent_volumeFrom127_0070ea20 @ 0x0070_ea20 => extern "stdcall" fn volume_from_127(v: u32) -> f32 {
        s::volume_from_127(&mem(), v)
    }
    /// `FmodMusic_initData(this; system)`.
    FmodMusic_initData @ 0x0070_8c00 => extern "thiscall" fn fmod_music_init_data(this: u32, system: u32) -> u32 {
        s::fmod_music_init_data(&mut mem(), this, system)
    }
    /// `SfxIdArray_assign_0070c5c0(this; src)` -> `this`.
    SfxIdArray_assign_0070c5c0 @ 0x0070_c5c0 => extern "thiscall" fn sfx_id_array_assign(this: u32, src: u32) -> u32 {
        s::word_array_assign(&mut mem(), this, src, 0x0070_d3a0)
    }
    /// `IdArray_assign_007109e0(this; src)` -> `this`.
    IdArray_assign_007109e0 @ 0x0071_09e0 => extern "thiscall" fn id_array_assign(this: u32, src: u32) -> u32 {
        s::word_array_assign(&mut mem(), this, src, 0x0066_7e00)
    }
    /// `PhyFMVPlayerManager_fmvPlayer_initialize(name)`.
    PhyFMVPlayerManager_fmvPlayer_initialize @ 0x006d_8f30 => extern "stdcall" fn fmv_initialize(name: u32) -> u32 {
        mv::fmv_initialize(&mut mem(), name)
    }
    /// `IggyMenu_dtor_IggyMenu(this)`.
    IggyMenu_dtor_IggyMenu @ 0x00a2_8a40 => extern "thiscall" fn iggy_menu_dtor(this: u32) {
        mn::iggy_menu_dtor(&mut mem(), this);
    }
    /// `FFEscMenu_unpauseAudioVideo(this)`.
    FFEscMenu_unpauseAudioVideo @ 0x0068_ff60 => extern "thiscall" fn esc_unpause(this: u32) {
        mn::esc_unpause(&mut mem(), this);
    }
    /// `PArray_copyNew_006affc0(n, src)`.
    PArray_copyNew_006affc0 @ 0x006a_ffc0 => extern "C" fn parray_copy_new_6affc0(n: u32, src: u32) -> u32 {
        mn::parray_copy_new(&mut mem(), n, src)
    }
    /// `PArray_copyNew_006b0010(n, src)`.
    PArray_copyNew_006b0010 @ 0x006b_0010 => extern "C" fn parray_copy_new_6b0010(n: u32, src: u32) -> u32 {
        mn::parray_copy_new(&mut mem(), n, src)
    }
    /// `VideoText_setSize_006dded0(this; w, h)`.
    VideoText_setSize_006dded0 @ 0x006d_ded0 => extern "thiscall" fn video_text_set_size(this: u32, w: u32, h: u32) -> u32 {
        mn::video_text_set_size(&mut mem(), this, w, h)
    }
    /// `FFEscMenu_dialogText_0068e560(id)`.
    FFEscMenu_dialogText_0068e560 @ 0x0068_e560 => extern "C" fn esc_dialog_text(id: u32) -> u32 {
        mn::esc_dialog_text(&mut mem(), id)
    }
    /// `SaveDataMachine_showPopUp_00702e90(id)`.
    SaveDataMachine_showPopUp_00702e90 @ 0x0070_2e90 => extern "C" fn save_show_popup(id: u32) -> u32 {
        mn::save_show_popup(&mut mem(), id)
    }
    /// `BrokenScreenEffect_reset(this)`.
    BrokenScreenEffect_reset @ 0x006d_5970 => extern "thiscall" fn broken_screen_reset(this: u32) {
        e::broken_screen_reset(&mut mem(), this);
    }
    /// `ShaderParam_Write_006426e0(params, a, b)` -> 0.
    ShaderParam_Write_006426e0 @ 0x0064_26e0 => extern "C" fn shader_param_write(params: u32, a: u32, b: u32) -> u32 {
        e::shader_param_write(&mut mem(), params, a, b)
    }
    /// `BlurEffectEx_initialize(this; a, b, c, d, e)` -> 0.
    BlurEffectEx_initialize @ 0x006c_4b00 => extern "thiscall" fn blur_ex_initialize(this: u32, a: u32, _b: u32, _c: u32, d: u32, x: u32) -> u32 {
        e::blur_ex_initialize(&mut mem(), this, a, d, x)
    }
    /// `RadialBlur_resizeAll_006c3c40(this; w, h)` -> 0.
    RadialBlur_resizeAll_006c3c40 @ 0x006c_3c40 => extern "thiscall" fn radial_blur_resize_all(this: u32, w: u32, h: u32) -> u32 {
        e::radial_blur_resize_all(&mut mem(), this, w, h)
    }
    /// `BlurEffectEx_update(this; dt)` -> 0.
    BlurEffectEx_update @ 0x006c_4e00 => extern "thiscall" fn blur_ex_update(this: u32, _dt: u32) -> u32 {
        e::blur_ex_update(&mut mem(), this)
    }
    /// `PhyreSceneCursor_ctor_00633880(this; world)` -> `this`.
    PhyreSceneCursor_ctor_00633880 @ 0x0063_3880 => extern "thiscall" fn scene_cursor_ctor(this: u32, world: u32) -> u32 {
        m::scene_cursor_ctor(&mut mem(), this, world)
    }
    // Batch 5: SfxEvent volumes, FMOD event info, the shader-parameter
    // copies, the broken screen's light, Iggy calls, the viewport rect,
    // the language suffix callback, the audio preview callbacks, the Iggy
    // heap, the PS2 accumulation frames.
    /// `SfxEvent_applyVolume_0070f140(list; volume)`.
    SfxEvent_applyVolume_0070f140 @ 0x0070_f140 => extern "thiscall" fn sfx_event_set_volume(list: u32, volume: u32) {
        s::sfx_event_set_volume(&mut mem(), list, volume);
    }
    /// `FmodEventInfo_Init(this)` -> `this`.
    FmodEventInfo_Init @ 0x0070_a140 => extern "thiscall" fn fmod_event_info_init(this: u32) -> u32 {
        s::fmod_event_info_init(&mut mem(), this)
    }
    /// `ShaderParamBuffer_SetValue2(buffer; param, src)`.
    ShaderParamBuffer_SetValue2 @ 0x0063_86c0 => extern "thiscall" fn shader_param_set_value2(buffer: u32, param: u32, src: u32) -> u32 {
        e::shader_param_copy(&mut mem(), buffer, param, src)
    }
    /// `ShaderParam_CopyToBuffer_00638720(buffer; param, src)`.
    ShaderParam_CopyToBuffer_00638720 @ 0x0063_8720 => extern "thiscall" fn shader_param_copy_638720(buffer: u32, param: u32, src: u32) -> u32 {
        e::shader_param_copy(&mut mem(), buffer, param, src)
    }
    /// `ShaderParamBuffer_SetValue(buffer; param, src)`.
    ShaderParamBuffer_SetValue @ 0x0063_8780 => extern "thiscall" fn shader_param_set_value(buffer: u32, param: u32, src: u32) -> u32 {
        e::shader_param_copy(&mut mem(), buffer, param, src)
    }
    /// `ShaderParam_CopyToBuffer_00698960(buffer; param, src)`.
    ShaderParam_CopyToBuffer_00698960 @ 0x0069_8960 => extern "thiscall" fn shader_param_copy_698960(buffer: u32, param: u32, src: u32) -> u32 {
        e::shader_param_copy(&mut mem(), buffer, param, src)
    }
    /// `BrokenScreenEffect_updateLightParameters(this; p0..p6)` -> `p6`.
    BrokenScreenEffect_updateLightParameters @ 0x006d_5a70 => extern "thiscall" fn broken_screen_update_light(this: u32, p0: u32, p1: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32) -> u32 {
        e::broken_screen_update_light(&mut mem(), this, [p0, p1, p2, p3, p4, p5, p6])
    }
    /// `IggyMenu_callIggyAS3External(this; target, name, n, values, unused)`.
    IggyMenu_callIggyAS3External @ 0x00a2_8ba0 => extern "thiscall" fn iggy_call_as3(this: u32, target: u32, name: u32, n: u32, values: u32, _unused: u32) -> u32 {
        mn::iggy_call_as3(&mut mem(), this, [target, name, n, values])
    }
    /// `IggyMenuManager_setViewportRect_00a26bd0(this; r0..r5)`.
    IggyMenuManager_setViewportRect_00a26bd0 @ 0x00a2_6bd0 => extern "thiscall" fn iggy_set_viewport_rect(this: u32, r0: u32, r1: u32, r2: u32, r3: u32, r4: u32, r5: u32) -> u32 {
        mn::iggy_set_viewport_rect(&mut mem(), this, [r0, r1, r2, r3, r4, r5])
    }
    /// `IggyPlayer_resultLanguageSuffix_006e9510(callback)`.
    IggyPlayer_resultLanguageSuffix_006e9510 @ 0x006e_9510 => extern "C" fn iggy_language_suffix(callback: u32) -> u32 {
        mn::iggy_language_suffix(&mut mem(), callback)
    }
    /// `FFEscMenu_registerAudioPreviewCallbacks(this)`.
    FFEscMenu_registerAudioPreviewCallbacks @ 0x006f_fa20 => extern "thiscall" fn esc_register_audio_preview(this: u32) -> u32 {
        mn::esc_register_audio_preview(&mut mem(), this)
    }
    /// `IggyHeap_takeFree_00a39a30(heap)`.
    IggyHeap_takeFree_00a39a30 @ 0x00a3_9a30 => extern "C" fn iggy_heap_take_free(heap: u32) -> u32 {
        mn::iggy_heap_take_free(&mut mem(), heap)
    }
    /// `Sg_AccSetFrameMap()`.
    Sg_AccSetFrameMap @ 0x0082_bc50 => extern "C" fn acc_set_frame_map() -> u32 {
        m::acc_set_frame_map(&mut mem())
    }
    /// `Sg_AccSetFrameChr()`.
    Sg_AccSetFrameChr @ 0x0082_be20 => extern "C" fn acc_set_frame_chr() -> u32 {
        m::acc_set_frame_chr(&mut mem())
    }
    // Batch 6: SfxEvent's 0..127 volume and pan, the FMV manager's
    // creation, the escape menu's pause, array and character list
    // clean-ups, the scene cursors' walks, the second save pop-up, the
    // prototype lookup, a string move, PS2 frame and store helpers, the
    // particle fill buffers.
    /// `SfxEvent_setVolume(event; v)`.
    SfxEvent_setVolume @ 0x0070_f1a0 => extern "thiscall" fn sfx_event_set_volume_127(event: u32, v: u32) -> u32 {
        s::sfx_event_set_volume_127(&mut mem(), event, v)
    }
    /// `SfxEvent_setPan(event; v)`.
    SfxEvent_setPan @ 0x0070_ef80 => extern "thiscall" fn sfx_event_set_pan(event: u32, v: u32) -> u32 {
        s::sfx_event_set_pan(&mut mem(), event, v)
    }
    /// `PhyFMVPlayerManager_CreateInstance()`.
    PhyFMVPlayerManager_CreateInstance @ 0x006d_7130 => extern "C" fn fmv_create_instance() {
        mv::fmv_create_instance(&mut mem());
    }
    /// `FFEscMenu_pauseAudioVideo(this)`.
    FFEscMenu_pauseAudioVideo @ 0x0068_f940 => extern "thiscall" fn esc_pause(this: u32) -> u32 {
        mn::esc_pause(&mut mem(), this)
    }
    /// `PArray_destroyObjects_006af630(this)`.
    PArray_destroyObjects_006af630 @ 0x006a_f630 => extern "thiscall" fn parray_destroy_objects(this: u32) {
        mn::parray_destroy_objects(&mut mem(), this);
    }
    /// `PhyreSceneCursor_Begin_006a2260(this)`.
    PhyreSceneCursor_Begin_006a2260 @ 0x006a_2260 => extern "thiscall" fn cursor_begin_6a2260(this: u32) -> u32 {
        mn::cursor_begin(&mut mem(), this, 0x006a_29c0)
    }
    /// `PhyreSceneCursor_Begin_006f6300(this)`.
    PhyreSceneCursor_Begin_006f6300 @ 0x006f_6300 => extern "thiscall" fn cursor_begin_6f6300(this: u32) -> u32 {
        mn::cursor_begin(&mut mem(), this, 0x006f_63b0)
    }
    /// `PhyreSceneCursor_Next_006a22d0(this)`.
    PhyreSceneCursor_Next_006a22d0 @ 0x006a_22d0 => extern "thiscall" fn cursor_next_6a22d0(this: u32) -> u32 {
        mn::cursor_next(&mut mem(), this, 0x006a_29c0)
    }
    /// `SaveDataMachine_showPopUp2_00702d20(n, id)`.
    SaveDataMachine_showPopUp2_00702d20 @ 0x0070_2d20 => extern "C" fn save_show_popup2(n: u32, id: u32) -> u32 {
        mn::save_show_popup2(&mut mem(), n, id)
    }
    /// `ClassLoadedCharacterTable_existPrototype(this; id)` (`al`).
    ClassLoadedCharacterTable_existPrototype @ 0x006a_1e80 => extern "thiscall" fn exist_prototype(this: u32, id: u32) -> u8 {
        mn::exist_prototype(&mem(), this, id)
    }
    /// `ClassCharacter_removeInstance(this; chr)`.
    ClassCharacter_removeInstance @ 0x006a_2b90 => extern "thiscall" fn character_remove_instance(this: u32, chr: u32) -> u32 {
        mn::character_remove_instance(&mut mem(), this, chr)
    }
    /// `StdString_moveCtor_0069c270(this; src, unused)` -> `this`.
    StdString_moveCtor_0069c270 @ 0x0069_c270 => extern "thiscall" fn string_move_ctor(this: u32, src: u32, _unused: u32) -> u32 {
        mn::string_move_ctor(&mut mem(), this, src)
    }
    /// `Sg_AccDrawBG()`.
    Sg_AccDrawBG @ 0x0082_be80 => extern "C" fn acc_draw_bg() -> u32 {
        m::acc_draw_bg(&mut mem())
    }
    /// `JobSchedule_pppPart_initFillBufferRequest()` -> 0.
    JobSchedule_pppPart_initFillBufferRequest @ 0x0072_8750 => extern "C" fn init_fill_buffer_request() -> u32 {
        m::init_fill_buffer_request(&mut mem())
    }
    /// `op_gs_store(rect, dst)`.
    op_gs_store @ 0x007e_4540 => extern "C" fn gs_store(rect: u32, dst: u32) -> u32 {
        m::gs_store(&mut mem(), rect, dst)
    }
    // Batch 7: the sfx id array's reserve, FmodMusic's constructor, the
    // movie names, the button remap's constructor, the Iggy heap's block
    // move, the menu's VRAM block transfers.
    /// `SfxIdArray_reserve_0070d3a0(this; n)`.
    SfxIdArray_reserve_0070d3a0 @ 0x0070_d3a0 => extern "thiscall" fn sfx_id_array_reserve(this: u32, n: u32) -> u32 {
        s::sfx_id_array_reserve(&mut mem(), this, n)
    }
    /// `FmodMusic_FmodMusic(this)` -> `this`.
    FmodMusic_FmodMusic @ 0x0070_8630 => extern "thiscall" fn fmod_music_ctor(this: u32) -> u32 {
        s::fmod_music_ctor(&mut mem(), this)
    }
    /// `PhyFMVPlayerManager_getVideoName(this; id, out)` (`al`).
    PhyFMVPlayerManager_getVideoName @ 0x006d_93e0 => extern "thiscall" fn fmv_video_name(this: u32, id: u32, out: u32) -> u8 {
        mv::fmv_video_name(&mut mem(), this, id, out)
    }
    /// `RemapButtonIcon_RemapButtonIcon(this)` -> `this`.
    RemapButtonIcon_RemapButtonIcon @ 0x006e_6330 => extern "thiscall" fn remap_button_icon_ctor(this: u32) -> u32 {
        mn::remap_button_icon_ctor(&mut mem(), this)
    }
    /// `IggyHeap_moveBlock_00a39a90(block, size, owner, kind)`.
    IggyHeap_moveBlock_00a39a90 @ 0x00a3_9a90 => extern "C" fn iggy_heap_move_block(block: u32, size: u32, owner: u32, kind: u32) -> u32 {
        mn::iggy_heap_move_block(&mut mem(), block, size, owner, kind)
    }
    /// `ClusterSlots_take_0069aba0(this)`.
    ClusterSlots_take_0069aba0 @ 0x0069_aba0 => extern "thiscall" fn cluster_slots_take(this: u32) -> u32 {
        mn::cluster_slots_take(&mut mem(), this)
    }
    /// `BlurEffectEx_CreateInstance()`.
    BlurEffectEx_CreateInstance @ 0x006c_49a0 => extern "C" fn blur_ex_create_instance() {
        e::blur_ex_create_instance(&mut mem());
    }
    /// `ChrResourceCache_DelayReleaseStep_0069c010(timer; dt)`.
    ChrResourceCache_DelayReleaseStep_0069c010 @ 0x0069_c010 => extern "thiscall" fn delay_release_step(timer: u32, dt: u32) {
        e::delay_release_step(&mut mem(), timer, dt);
    }
    /// `MenuMain_LoadVramBlock(buffer, address, size)`.
    MenuMain_LoadVramBlock @ 0x008a_a710 => extern "C" fn menu_load_vram_block(buffer: u32, address: u32, size: u32) -> u32 {
        m::menu_vram_block(&mut mem(), true, false, buffer, address, size)
    }
    /// `MenuMain_StoreVramBlock(buffer, address, size)`.
    MenuMain_StoreVramBlock @ 0x008a_ac10 => extern "C" fn menu_store_vram_block(buffer: u32, address: u32, size: u32) -> u32 {
        m::menu_vram_block(&mut mem(), false, false, buffer, address, size)
    }
    /// `MenuMain_StoreFrameBlock(buffer, address, size)`.
    MenuMain_StoreFrameBlock @ 0x008a_acf0 => extern "C" fn menu_store_frame_block(buffer: u32, address: u32, size: u32) -> u32 {
        m::menu_vram_block(&mut mem(), false, true, buffer, address, size)
    }
    // Batch 8: FMOD voice clear, a post-effect parameter block, the third
    // save pop-up, the escape menu's resume, text scrolling, the MFIFO
    // reset.
    /// `FmodVoice_clear(voice)` -> 0.
    FmodVoice_clear @ 0x0070_a590 => extern "thiscall" fn fmod_voice_clear(voice: u32) -> u32 {
        s::fmod_voice_clear(&mut mem(), voice)
    }
    /// `PostEffectParams_ctor_00702ff0(this)` -> `this`.
    PostEffectParams_ctor_00702ff0 @ 0x0070_2ff0 => extern "thiscall" fn post_effect_params_ctor(this: u32) -> u32 {
        e::post_effect_params_ctor(&mut mem(), this)
    }
    /// `SaveDataMachine_showPopUp4_00702ef0(id)`.
    SaveDataMachine_showPopUp4_00702ef0 @ 0x0070_2ef0 => extern "C" fn save_show_popup4(id: u32) -> u32 {
        mn::save_show_popup4(&mut mem(), id)
    }
    /// `FFEscMenu_resume(menu)`.
    FFEscMenu_resume @ 0x0068_fe60 => extern "C" fn esc_resume(menu: u32) {
        mn::esc_resume(&mut mem(), menu);
    }
    /// `TextUtil_scrollToVisible(this; n)`.
    TextUtil_scrollToVisible @ 0x006f_6860 => extern "thiscall" fn text_scroll(this: u32, n: u32) {
        mn::text_scroll(&mut mem(), this, n);
    }
    /// `Sg_MFIFOReset_008432e0()`.
    Sg_MFIFOReset_008432e0 @ 0x0084_32e0 => extern "C" fn mfifo_reset() -> u32 {
        m::mfifo_reset(&mut mem())
    }
    /// `FFEscMenu_playMenuSound(this; id)`: void, the pointer callers drop
    /// `eax` (check-clobber `VOID_OK`, dispatcher 0x00a28ff0).
    FFEscMenu_playMenuSound @ 0x0068_f9b0 => extern "thiscall" fn esc_play_menu_sound(this: u32, id: u32) {
        mn::esc_play_menu_sound(&mut mem(), this, id);
    }
    /// `IggyMenu_callFlashFunction_1(menu, name, 1 values by value)`: the values are
    /// read from the caller's arguments.
    IggyMenu_callFlashFunction_1 @ 0x0068_dd40 => extern "C" fn iggy_call_flash_1(_menu: u32, _name: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        fr::iggy_call_flash(&mut mem(), frame.at(), 1, args)
    }
    /// `IggyMenu_callFlashFunction_2_0068dd80(menu, name, 2 values by value)`: the values are
    /// read from the caller's arguments.
    IggyMenu_callFlashFunction_2_0068dd80 @ 0x0068_dd80 => extern "C" fn iggy_call_flash_2(_menu: u32, _name: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        fr::iggy_call_flash(&mut mem(), frame.at(), 2, args)
    }
    /// `IggyMenu_callFlashFunction_3_0068ddd0(menu, name, 3 values by value)`: the values are
    /// read from the caller's arguments.
    IggyMenu_callFlashFunction_3_0068ddd0 @ 0x0068_ddd0 => extern "C" fn iggy_call_flash_3(_menu: u32, _name: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<12>::new();
        fr::iggy_call_flash(&mut mem(), frame.at(), 3, args)
    }
    /// `IggyMenu_callFlashFunction_4_0068de50(menu, name, 4 values by value)`: the values are
    /// read from the caller's arguments.
    IggyMenu_callFlashFunction_4_0068de50 @ 0x0068_de50 => extern "C" fn iggy_call_flash_4(_menu: u32, _name: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<16>::new();
        fr::iggy_call_flash(&mut mem(), frame.at(), 4, args)
    }
    /// `IggyMenu_callFlashFunction_5_0068def0(menu, name, 5 values by value)`: the values are
    /// read from the caller's arguments.
    IggyMenu_callFlashFunction_5_0068def0 @ 0x0068_def0 => extern "C" fn iggy_call_flash_5(_menu: u32, _name: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<20>::new();
        fr::iggy_call_flash(&mut mem(), frame.at(), 5, args)
    }
    /// `IggyMenu_getArrayS32_00a28b20(this; i)`.
    IggyMenu_getArrayS32_00a28b20 @ 0x00a2_8b20 => extern "thiscall" fn iggy_array_s32(this: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::ARRAY_S32_FRAME / 4) as usize }>::new();
        fr::iggy_array_s32(&mut mem(), frame.at(), this, i)
    }
    /// `FmodEventTree_getEvent_00710290(this; key)`.
    FmodEventTree_getEvent_00710290 @ 0x0071_0290 => extern "thiscall" fn fmod_tree_get_event(this: u32, key: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::TREE_EVENT_FRAME / 4) as usize }>::new();
        fr::fmod_tree_get_event(&mut mem(), frame.at(), this, key)
    }
    /// `Distortion_resetRenderTarget_006c24b0(this)`.
    Distortion_resetRenderTarget_006c24b0 @ 0x006c_24b0 => extern "thiscall" fn distortion_reset_target(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::DISTORTION_FRAME / 4) as usize }>::new();
        fr::distortion_reset_target(&mut mem(), frame.at(), this)
    }
    /// `PostProcessManager_findClusterData_00698a30(this; i)`.
    PostProcessManager_findClusterData_00698a30 @ 0x0069_8a30 => extern "thiscall" fn post_find_cluster_data(this: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::CLUSTER_CURSOR_FRAME / 4) as usize }>::new();
        fr::post_find_cluster_data(&mut mem(), frame.at(), this, i)
    }
    /// `ScreenFadeDelegations_registerDelegation(r, g, b, a, x, y)`.
    ScreenFadeDelegations_registerDelegation @ 0x006a_e650 => extern "C" fn screen_fade_register(r: u32, g: u32, b: u32, a: u32, x: u32, y: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::FADE_RECORD_FRAME / 4) as usize }>::new();
        fr::screen_fade_register(&mut mem(), frame.at(), [r, g, b, a], x, y)
    }
    /// `gdraw_MapBuffer_00a3cec0(resource)`.
    gdraw_MapBuffer_00a3cec0 @ 0x00a3_cec0 => extern "C" fn gdraw_map_buffer(resource: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::MAPPED_FRAME / 4) as usize }>::new();
        fr::gdraw_map_buffer(&mut mem(), frame.at(), resource)
    }
    /// `gdraw_clearBindings_00a3d280()`.
    gdraw_clearBindings_00a3d280 @ 0x00a3_d280 => extern "C" fn gdraw_clear_bindings() -> u32 {
        let mut frame = StackFrame::<{ (fr::CLEAR_BINDINGS_FRAME / 4) as usize }>::new();
        fr::gdraw_clear_bindings(&mut mem(), frame.at())
    }
    /// `LocString_Load_0068e180(this; id)`.
    LocString_Load_0068e180 @ 0x0068_e180 => extern "thiscall" fn loc_string_load(this: u32, id: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::LOC_STRING_FRAME / 4) as usize }>::new();
        fr::loc_string_load(&mut mem(), frame.at(), this, id)
    }
    /// `getFullFilename(out, name)`.
    getFullFilename @ 0x0070_7aa0 => extern "C" fn full_filename(out: u32, name: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::FULL_FILENAME_FRAME / 4) as usize }>::new();
        fr::full_filename(&mut mem(), frame.at(), out, name)
    }
    /// `ClassLoadedCharacterTable_makeEntry_0069da20(out, name, value)`.
    ClassLoadedCharacterTable_makeEntry_0069da20 @ 0x0069_da20 => extern "C" fn prototype_entry(out: u32, name: u32, value: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::PROTOTYPE_ENTRY_FRAME / 4) as usize }>::new();
        fr::prototype_entry(&mut mem(), frame.at(), out, name, value)
    }
    /// `ERRCHECK(result, file, line)`: writes its own `result` slot
    /// (`args`), as the original does.
    ERRCHECK @ 0x0070_79d0 => extern "C" fn errcheck(_result: u32, _file: u32, _line: u32; args: u32) -> u32 {
        let mut local = StackFrame::<1>::new();
        fr::errcheck(&mut mem(), local.at(), args)
    }
    /// `printToOutput(prefix, fmt, ...)`: the varargs are the caller's.
    printToOutput @ 0x0070_7b20 => extern "C" fn print_to_output(_prefix: u32, _fmt: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::PRINT_FRAME / 4) as usize }>::new();
        fr::print_to_output(&mut mem(), frame.at(), args)
    }
    /// `SfxBank_getNumPlayingEvents(this)`.
    SfxBank_getNumPlayingEvents @ 0x0070_fb20 => extern "thiscall" fn sfx_bank_playing_events(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::BANK_INFO_FRAME / 4) as usize }>::new();
        fr::sfx_bank_playing_events(&mut mem(), frame.at(), this)
    }
    /// `FmodVoice_volumeDirect(this; volume, keep)`.
    FmodVoice_volumeDirect @ 0x0070_b7f0 => extern "thiscall" fn voice_volume_direct(this: u32, volume: u32, keep: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        fr::voice_volume_direct(&mut mem(), frame.at(), this, volume, keep)
    }
    /// `FmodSfx_isPlaying(this; id)`.
    FmodSfx_isPlaying @ 0x0070_ce90 => extern "thiscall" fn sfx_is_playing(this: u32, id: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        fr::sfx_is_playing(&mut mem(), frame.at(), this, id)
    }
    /// `gdraw_setRenderTargetView_00a38300(view, a, b, c, d)`.
    gdraw_setRenderTargetView_00a38300 @ 0x00a3_8300 => extern "C" fn gdraw_set_target(view: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::VIEW_DESC_FRAME / 4) as usize }>::new();
        fr::gdraw_set_target(&mut mem(), frame.at(), view, [a, b, c, d])
    }
    /// `gdraw_mapResource_00a3c070(ctx, resource, discard)`.
    gdraw_mapResource_00a3c070 @ 0x00a3_c070 => extern "C" fn gdraw_map_resource(ctx: u32, resource: u32, discard: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::MAPPED_FRAME / 4) as usize }>::new();
        fr::gdraw_map_resource(&mut mem(), frame.at(), ctx, resource, discard)
    }
    /// `BlurEffect_initialize_006c38b0(this; a, b, c, d, e)`.
    BlurEffect_initialize_006c38b0 @ 0x006c_38b0 => extern "thiscall" fn blur_initialize(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        let mut frame = StackFrame::<{ (fr::BLUR_PATH_FRAME / 4) as usize }>::new();
        fr::blur_initialize(&mut mem(), frame.at(), this, [a, b, c, d, e])
    }
    /// `FmodShout_setVolume(this; volume)`: void, both callers drop `eax`
    /// (0x0070b308 calls on, 0x0070b46e zeroes it).
    FmodShout_setVolume @ 0x0070_e400 => extern "thiscall" fn shout_set_volume(this: u32, volume: u32) {
        let mut frame = StackFrame::<1>::new();
        fr::shout_set_volume(&mut mem(), frame.at(), this, volume);
    }
    /// `gdraw_newPool_00a3bcd0(kind)`: overwrites its own argument, as the
    /// original does.
    gdraw_newPool_00a3bcd0 @ 0x00a3_bcd0 => extern "C" fn gdraw_new_pool(_kind: u32; args: u32) -> u32 {
        fr::gdraw_new_pool(&mut mem(), args)
    }
    /// `Distortion_onShaderClusterLoaded_006c28b0(cluster, &distortion)`:
    /// hands on the address of its own `cluster` argument.
    Distortion_onShaderClusterLoaded_006c28b0 @ 0x006c_28b0 => extern "C" fn distortion_on_cluster_loaded(_cluster: u32, _holder: u32; args: u32) -> u32 {
        fr::distortion_on_cluster_loaded(&mut mem(), args)
    }
    /// `LoadingScreen_initialize(this)`.
    LoadingScreen_initialize @ 0x006e_4720 => extern "thiscall" fn loading_screen_initialize(this: u32) -> u32 {
        m::loading_screen_initialize(&mut mem(), this)
    }
    /// `SaveDataMachine_setSaveLoadFocus_00702cd0(i)`.
    SaveDataMachine_setSaveLoadFocus_00702cd0 @ 0x0070_2cd0 => extern "C" fn save_set_focus(i: u32) -> u32 {
        m::save_set_focus(&mut mem(), i)
    }
    /// `ClusterSlots_give_0069ae20(this; p)`.
    ClusterSlots_give_0069ae20 @ 0x0069_ae20 => extern "thiscall" fn cluster_slots_give(this: u32, p: u32) -> u32 {
        m::cluster_slots_give(&mut mem(), this, p)
    }
    /// `IggyMenuManager_callback_00a260f0(a, b, c)`.
    IggyMenuManager_callback_00a260f0 @ 0x00a2_60f0 => extern "stdcall" fn iggy_manager_callback(a: u32, b: u32, c: u32) -> u32 {
        m::iggy_manager_handle_callback(&mut mem(), a, b, c)
    }
    /// `BlurEffectEx_setParam48_006c45c0(this; x, y, z, w)`: copies from
    /// its own argument words (the caller's, through `args`).
    BlurEffectEx_setParam48_006c45c0 @ 0x006c_45c0 => extern "thiscall" fn blur_set_param48(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        fr::post_set_param(&mut mem(), fr::PARAM_48, this, args)
    }
    /// `BlurEffectEx_setParam4c_006c4680(this; x, y, z, w)`.
    BlurEffectEx_setParam4c_006c4680 @ 0x006c_4680 => extern "thiscall" fn blur_set_param4c(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        fr::post_set_param(&mut mem(), fr::PARAM_4C, this, args)
    }
    /// `BlurEffectEx_setParam50_006c4730(this; x, y, z, w)`.
    BlurEffectEx_setParam50_006c4730 @ 0x006c_4730 => extern "thiscall" fn blur_set_param50(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        fr::post_set_param(&mut mem(), fr::PARAM_50, this, args)
    }
    /// `RadialBlur_setParam54_007036f0(this; x, y, z, w)`.
    RadialBlur_setParam54_007036f0 @ 0x0070_36f0 => extern "thiscall" fn radial_set_param54(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        fr::post_set_param(&mut mem(), fr::PARAM_54, this, args)
    }
    /// `TextUtil_init(n)` (`n` in `ecx`): the nine-word cluster loads go
    /// through `call_cdecl_words`.
    TextUtil_init @ 0x006f_6370 => extern "thiscall" fn text_util_init(n: u32) -> u32 {
        r3::text_util_init(&mut mem(), n, &mut |_, f, a| super::call_cdecl_words(f, a))
    }
    /// `TOMkpCrossEasyStrFontSClut(s, x, y, w, sx, sy)`: the ten-word call
    /// through `call_cdecl_words`.
    TOMkpCrossEasyStrFontSClut @ 0x0090_1660 => extern "C" fn mkp_cross_easy(s: u32, x: u32, y: u32, w: u32, sx: u32, sy: u32) -> u32 {
        r3::mkp_cross_easy(&mut mem(), [s, x, y, w, sx, sy], &mut |_, f, a| super::call_cdecl_words(f, a))
    }
    /// `AsyncLoadManager_createInstance_00691a80()`.
    AsyncLoadManager_createInstance_00691a80 @ 0x0069_1a80 => extern "C" fn async_load_manager_create() {
        mn::create_instance(&mut mem(), r3::ASYNC_LOAD_MANAGER);
    }
    /// `PostProcessManager_createInstance_00692a90()`.
    PostProcessManager_createInstance_00692a90 @ 0x0069_2a90 => extern "C" fn post_process_manager_create() {
        mn::create_instance(&mut mem(), r3::POST_PROCESS_MANAGER);
    }
    /// `PostProcessTweakUtil_createInstance_00692b10()`.
    PostProcessTweakUtil_createInstance_00692b10 @ 0x0069_2b10 => extern "C" fn post_process_tweak_create() {
        mn::create_instance(&mut mem(), r3::POST_PROCESS_TWEAK);
    }
    /// `TextureAnimationManager_createInstance_006b00a0()`.
    TextureAnimationManager_createInstance_006b00a0 @ 0x006b_00a0 => extern "C" fn texture_animation_create() {
        r3::texture_animation_create(&mut mem());
    }
    /// `Array8_reserve_0070edb0(this; n)`.
    Array8_reserve_0070edb0 @ 0x0070_edb0 => extern "thiscall" fn array8_reserve(this: u32, n: u32) -> u32 {
        r3::array8_reserve(&mut mem(), this, n)
    }
    /// `WordArray_reserve_0070fbf0(this; n)`: the body of
    /// `SfxIdArray_reserve_0070d3a0`.
    WordArray_reserve_0070fbf0 @ 0x0070_fbf0 => extern "thiscall" fn word_array_reserve(this: u32, n: u32) -> u32 {
        s::sfx_id_array_reserve(&mut mem(), this, n)
    }
    /// `PArray40_assign_006afa70(this; src)`: keeps `src`'s storage in its
    /// argument slot, as the original does.
    PArray40_assign_006afa70 @ 0x006a_fa70 => extern "thiscall" fn parray40_assign(this: u32, _src: u32; args: u32) -> u32 {
        r3::parray40_assign(&mut mem(), this, args)
    }
    /// `Tree_eraseValue_006a1d10(this; value)`: the erase writes its
    /// iterator into the argument slot.
    Tree_eraseValue_006a1d10 @ 0x006a_1d10 => extern "thiscall" fn tree_erase_value(this: u32, _value: u32; args: u32) -> u32 {
        r3::tree_erase_value(&mut mem(), this, args)
    }
    /// `create_mspace_with_base(base, capacity, locked)`.
    create_mspace_with_base @ 0x006e_bd30 => extern "C" fn create_mspace_with_base(base: u32, capacity: u32, locked: u32) -> u32 {
        r3::create_mspace_with_base(&mut mem(), base, capacity, locked)
    }
    /// `FFEscMenu_onCancelInput_007018a0(this; input, b)`.
    FFEscMenu_onCancelInput_007018a0 @ 0x0070_18a0 => extern "thiscall" fn esc_on_cancel_input(this: u32, input: u32, _b: u32) -> u8 {
        let mut frame = StackFrame::<3>::new();
        r3::esc_on_cancel_input(&mut mem(), frame.at(), this, input)
    }
    /// `MenuTextureManager_LoadMainMenuWaveTexture()`.
    MenuTextureManager_LoadMainMenuWaveTexture @ 0x0067_cba0 => extern "C" fn load_main_menu_wave_texture() -> u32 {
        r3::load_main_menu_wave_texture(&mut mem())
    }
    /// `opus_0706_acc_draw(rec, base, out)`.
    opus_0706_acc_draw @ 0x007f_7360 => extern "C" fn opus_0706_acc_draw(rec: u32, base: u32, out: u32) -> u32 {
        r3::opus_0706_acc_draw(&mut mem(), rec, base, out)
    }
    /// `rcbgRestoreReg()`.
    rcbgRestoreReg @ 0x0092_0250 => extern "C" fn rcbg_restore_reg() -> u32 {
        let mut frame = StackFrame::<2>::new();
        r3::rcbg_restore_reg(&mut mem(), frame.at())
    }
    /// `MenuTextureManager_MenuUnloadTexture(id)`.
    MenuTextureManager_MenuUnloadTexture @ 0x0067_dbf0 => extern "stdcall" fn menu_unload_texture(id: u32) -> u32 {
        let mut frame = StackFrame::<{ (r3::UNLOAD_TEXTURE_FRAME / 4) as usize }>::new();
        r3::menu_unload_texture(&mut mem(), frame.at(), id)
    }
    /// `BlurEffect_createInstance_006c34d0()`.
    BlurEffect_createInstance_006c34d0 @ 0x006c_34d0 => extern "C" fn blur_effect_create() {
        mn::create_instance(&mut mem(), r3::BLUR_EFFECT_NEW);
    }
    /// `MotionBlur_createInstance_006c5030()`.
    MotionBlur_createInstance_006c5030 @ 0x006c_5030 => extern "C" fn motion_blur_create() {
        mn::create_instance(&mut mem(), r3::MOTION_BLUR_NEW);
    }
    /// `GraphicAbmap_createInstance_00684a60(arg)`.
    GraphicAbmap_createInstance_00684a60 @ 0x0068_4a60 => extern "C" fn abmap_create(arg: u32) {
        r3::abmap_create(&mut mem(), arg);
    }
    /// `MotionBlur_ctor_006c4ec0(this)`.
    MotionBlur_ctor_006c4ec0 @ 0x006c_4ec0 => extern "thiscall" fn motion_blur_ctor(this: u32) -> u32 {
        r3::motion_blur_ctor(&mut mem(), this)
    }
    /// `InitVFXRenderDataLine(a, b, c, d)`.
    InitVFXRenderDataLine @ 0x0071_12d0 => extern "C" fn init_vfx_render_data_line(a: u32, b: u32, c: u32, d: u32) -> u32 {
        r3::init_vfx_render_data_line(&mut mem(), a, b, c, d)
    }
    /// `TaggedTree_add_006a2830(this; &key, &value)`.
    TaggedTree_add_006a2830 @ 0x006a_2830 => extern "thiscall" fn tagged_tree_add(this: u32, key: u32, value: u32) -> u32 {
        r3::tree_add(&mut mem(), r3::TAGGED_TREE_INSERT, this, key, value)
    }
    /// `vfxIntTree_add_0070cd60(this; &key, &value)`.
    vfxIntTree_add_0070cd60 @ 0x0070_cd60 => extern "thiscall" fn vfx_int_tree_add(this: u32, key: u32, value: u32) -> u32 {
        r3::tree_add(&mut mem(), r3::VFX_INT_TREE_INSERT, this, key, value)
    }
    /// `MotionBlur_initialize(this; a, b, c)`: the eleven-word cluster
    /// load through `call_cdecl_words`.
    MotionBlur_initialize @ 0x006c_5360 => extern "thiscall" fn motion_blur_initialize(this: u32, _a: u32, b: u32, c: u32) -> u32 {
        let mut frame = StackFrame::<{ (r3::MOTION_BLUR_FRAME / 4) as usize }>::new();
        r3::motion_blur_initialize(&mut mem(), frame.at(), this, [b, c], &mut |_, f, a| super::call_cdecl_words(f, a))
    }
    /// `FFEscMenu_setEnableParaAndSpecial_006fca50(this; on)`.
    FFEscMenu_setEnableParaAndSpecial_006fca50 @ 0x006f_ca50 => extern "thiscall" fn esc_set_enable_para_and_special(this: u32, on: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        r3::esc_set_enable_para_and_special(&mut mem(), frame.at(), this, on)
    }
    /// `gdraw_lockDynamicBuffer_00a3cf40(buffer, n)`.
    gdraw_lockDynamicBuffer_00a3cf40 @ 0x00a3_cf40 => extern "C" fn gdraw_lock_dynamic(buffer: u32, n: u32) -> u32 {
        r3::gdraw_lock_dynamic(&mut mem(), buffer, n)
    }
    /// `ButtonIcon_setUvs_006e8020(this; id)`.
    ButtonIcon_setUvs_006e8020 @ 0x006e_8020 => extern "thiscall" fn button_icon_set_uvs(this: u32, id: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        r3::button_icon_set_uvs(&mut mem(), frame.at(), this, id)
    }
    /// `ClusterManager_getPClusterByName(this; name)`: leaves the map
    /// iterator in its argument slot, as the original does.
    ClusterManager_getPClusterByName @ 0x0069_b4b0 => extern "thiscall" fn get_pcluster_by_name(this: u32, _name: u32; args: u32) -> u32 {
        r3::get_pcluster_by_name(&mut mem(), this, args)
    }
    /// `FmodFile_open(name, unicode, filesize, handle, userdata)`.
    FmodFile_open @ 0x0070_8510 => extern "stdcall" fn fmod_file_open(name: u32, _unicode: u32, filesize: u32, handle: u32, _userdata: u32) -> u32 {
        r3::fmod_file_open(&mut mem(), name, filesize, handle)
    }
    /// `SfxBank_destroyEvents_00710960(this)`.
    SfxBank_destroyEvents_00710960 @ 0x0071_0960 => extern "thiscall" fn sfx_bank_destroy_events(this: u32) -> u32 {
        r3::sfx_bank_destroy_events(&mut mem(), this)
    }
    /// `VfxRenderDataMap_eraseRange_0069d990(this; first, last)`: steps
    /// the iterator in its argument slot.
    VfxRenderDataMap_eraseRange_0069d990 @ 0x0069_d990 => extern "thiscall" fn vfx_map_erase_range(this: u32, _first: u32, _last: u32; args: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        r3::vfx_map_erase_range(&mut mem(), frame.at(), this, args)
    }
    /// `PArray_appendInto_006a25e0(this; dst, &value)`.
    PArray_appendInto_006a25e0 @ 0x006a_25e0 => extern "thiscall" fn parray_append_into(this: u32, dst: u32, value: u32) -> u32 {
        r3::parray_append_into(&mut mem(), this, dst, value)
    }
    /// `MenuManager_TakeMultiRecord_0067c170(this; chain)`.
    MenuManager_TakeMultiRecord_0067c170 @ 0x0067_c170 => extern "thiscall" fn take_multi_record(this: u32, chain: u32) -> u32 {
        r3::take_multi_record(&mut mem(), this, chain)
    }
    /// `MenuDynamicGeometry_setMeshWorldMatrix(this)`.
    MenuDynamicGeometry_setMeshWorldMatrix @ 0x006a_2e60 => extern "thiscall" fn set_mesh_world_matrix(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (r3::MESH_WORLD_FRAME / 4) as usize }>::new();
        r3::set_mesh_world_matrix(&mut mem(), frame.at(), this)
    }
    // ---- continuation round, batch 1 (ffx_glue_other::h4_c1)
    /// `Sg_AccDrawAlphaQuad_0082baf0(fix)`.
    Sg_AccDrawAlphaQuad_0082baf0 @ 0x0082_baf0 => extern "C" fn acc_draw_alpha_quad(fix: u32) -> u32 {
        c1::acc_draw_alpha_quad(&mut mem(), fix)
    }
    /// `Sg_DrawFilterQuad_0082bb90(z)`.
    Sg_DrawFilterQuad_0082bb90 @ 0x0082_bb90 => extern "C" fn draw_filter_quad(z: u32) -> u32 {
        c1::draw_filter_quad(&mut mem(), z)
    }
    /// `sg_AccSetCurrent()`.
    sg_AccSetCurrent @ 0x0082_ce90 => extern "C" fn acc_set_current() {
        c1::acc_set_current(&mut mem());
    }
    /// `MenuManager_TakeSingleRecord_0067c210(this; chain)`.
    MenuManager_TakeSingleRecord_0067c210 @ 0x0067_c210 => extern "thiscall" fn take_single_record(this: u32, chain: u32) -> u32 {
        c1::take_single_record(&mut mem(), this, chain)
    }
    /// `MenuManager_findNameIndex_0067e310(this; name)`.
    MenuManager_findNameIndex_0067e310 @ 0x0067_e310 => extern "thiscall" fn find_name_index(this: u32, name: u32) -> u32 {
        c1::find_name_index(&mut mem(), this, name)
    }
    /// `PArray_reset16_0068bc60(this)`.
    PArray_reset16_0068bc60 @ 0x0068_bc60 => extern "thiscall" fn parray_reset16(this: u32) {
        c1::parray_reset16(&mut mem(), this);
    }
    /// `PArray8_appendInto_006a2790(this; dst, &value)`.
    PArray8_appendInto_006a2790 @ 0x006a_2790 => extern "thiscall" fn parray8_append_into(this: u32, dst: u32, value: u32) -> u32 {
        c1::parray8_append_into(&mut mem(), this, dst, value)
    }
    /// `op_texels_mip_00a384d0(dst, dst_step, w, h, src, src_step, size)`.
    op_texels_mip_00a384d0 @ 0x00a3_84d0 => extern "C" fn texels_mip(dst: u32, dst_step: u32, w: u32, rows: u32, src: u32, src_step: u32, size: u32) {
        c1::texels_mip(&mut mem(), [dst, dst_step, w, rows, src, src_step, size]);
    }
    /// `SfxEventGroup_appendEvents_00710ae0(this; key, out)`: hands its
    /// callees its own argument slots, as the original does.
    SfxEventGroup_appendEvents_00710ae0 @ 0x0071_0ae0 => extern "thiscall" fn append_group_events(this: u32, _key: u32, _out: u32; args: u32) -> u8 {
        c1::append_group_events(&mut mem(), this, args)
    }
    /// `SfxBank_appendEventById_0070fa80(this; id, out)`.
    SfxBank_appendEventById_0070fa80 @ 0x0070_fa80 => extern "thiscall" fn append_event_by_id(this: u32, id: u32, out: u32) -> u8 {
        c1::append_event_by_id(&mut mem(), this, id, out)
    }
    /// `ChrResourceCacheManager_enableChrDelayRelease(this)`.
    ChrResourceCacheManager_enableChrDelayRelease @ 0x0069_ae80 => extern "thiscall" fn enable_chr_delay_release(this: u32) -> u32 {
        c1::enable_chr_delay_release(&mut mem(), this)
    }
    /// `FFEscMenu_refreshParameterAndSpecialPageEnableState(this)`.
    FFEscMenu_refreshParameterAndSpecialPageEnableState @ 0x006f_c9b0 => extern "thiscall" fn esc_refresh_page_state(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (c1::REFRESH_FRAME / 4) as usize }>::new();
        c1::esc_refresh_page_state(&mut mem(), frame.at(), this)
    }
    /// `IggyStringArray_ctor_006fcbc0(this; root, names, n)`.
    IggyStringArray_ctor_006fcbc0 @ 0x006f_cbc0 => extern "thiscall" fn iggy_string_array_ctor(this: u32, root: u32, names: u32, n: u32) -> u32 {
        let mut frame = StackFrame::<{ (c1::STRING_ARRAY_FRAME / 4) as usize }>::new();
        c1::iggy_string_array_ctor(&mut mem(), frame.at(), this, [root, names, n])
    }
    /// `TextureAnimationInfo_dtor_006afd60(this; flags)`.
    TextureAnimationInfo_dtor_006afd60 @ 0x006a_fd60 => extern "thiscall" fn texture_animation_info_dtor(this: u32, flags: u32) -> u32 {
        c1::texture_animation_info_dtor(&mut mem(), this, flags)
    }
    /// `TextureAnimationInfoChar_dtor_006af7f0(this)`.
    TextureAnimationInfoChar_dtor_006af7f0 @ 0x006a_f7f0 => extern "thiscall" fn texture_animation_info_char_dtor(this: u32) {
        c1::texture_animation_info_char_dtor(&mut mem(), this);
    }
    /// `TextureAnimationManager_ctor_006af370(this)`.
    TextureAnimationManager_ctor_006af370 @ 0x006a_f370 => extern "thiscall" fn texture_animation_manager_ctor(this: u32) -> u32 {
        c1::texture_animation_manager_ctor(&mut mem(), this)
    }
    /// `TextureAnimationMaterial_dtor_006af690(this)`.
    TextureAnimationMaterial_dtor_006af690 @ 0x006a_f690 => extern "thiscall" fn texture_animation_material_dtor(this: u32) {
        c1::texture_animation_material_dtor(&mut mem(), this);
    }
    /// `TextureAnimationMaterial_assign_006afaf0(this; src)`.
    TextureAnimationMaterial_assign_006afaf0 @ 0x006a_faf0 => extern "thiscall" fn texture_animation_material_assign(this: u32, src: u32) -> u32 {
        c1::texture_animation_material_assign(&mut mem(), this, src)
    }
    /// `TOAdpMenuIconItemAbmap(pkt, item, x, y, colour)`: the twelve-word
    /// text call through `call_cdecl_words`.
    TOAdpMenuIconItemAbmap @ 0x008f_0700 => extern "C" fn adp_menu_icon_item(pkt: u32, item: u32, x: u32, y: u32, colour: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        c1::adp_menu_icon_item(&mut mem(), frame.at(), [pkt, item, x, y, colour], &mut |_, f, a| super::call_cdecl_words(f, a))
    }
    // ---- continuation round, batch 2 (ffx_glue_other::h4_c2)
    /// `ClusterManager_forceReleasePCluster(this; name)`: leaves the map
    /// iterator in its argument slot, as the original does.
    ClusterManager_forceReleasePCluster @ 0x0069_b310 => extern "thiscall" fn force_release_pcluster(this: u32, _name: u32; args: u32) -> u32 {
        c2::force_release_pcluster(&mut mem(), this, args)
    }
    /// `ClusterManager_releasePCluster_0069be90(this; name)`.
    ClusterManager_releasePCluster_0069be90 @ 0x0069_be90 => extern "thiscall" fn release_pcluster_by_name(this: u32, _name: u32; args: u32) -> u32 {
        c2::release_pcluster_by_name(&mut mem(), this, args)
    }
    /// `ClusterManager_releasePCluster(this; cluster)`.
    ClusterManager_releasePCluster @ 0x0069_bdb0 => extern "thiscall" fn release_pcluster(this: u32, _cluster: u32; args: u32) -> u32 {
        c2::release_pcluster(&mut mem(), this, args)
    }
    /// `ClusterMap_eraseRange_0069af70(this; out, first, last)`.
    ClusterMap_eraseRange_0069af70 @ 0x0069_af70 => extern "thiscall" fn cluster_map_erase_range(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        c2::cluster_map_erase_range(&mut mem(), frame.at(), this, [out, first, last])
    }
    /// `PArrayNamedPair_appendInto_0069b6e0(this; dst, &value)`.
    PArrayNamedPair_appendInto_0069b6e0 @ 0x0069_b6e0 => extern "thiscall" fn named_pair_append_into(this: u32, dst: u32, value: u32) -> u32 {
        c2::named_pair_append_into(&mut mem(), this, dst, value)
    }
    /// `SfxBank_dtor_SfxBank(this)`.
    SfxBank_dtor_SfxBank @ 0x0070_f930 => extern "thiscall" fn sfx_bank_dtor(this: u32) {
        c2::sfx_bank_dtor(&mut mem(), this);
    }
    /// `SfxEventMap_dtor_00710180(this)`.
    SfxEventMap_dtor_00710180 @ 0x0071_0180 => extern "thiscall" fn sfx_event_map_dtor(this: u32) {
        c2::sfx_event_map_dtor(&mut mem(), this);
    }
    /// `SfxIdTree_insertArray_0070cde0(this; &key, &array)`.
    SfxIdTree_insertArray_0070cde0 @ 0x0070_cde0 => extern "thiscall" fn sfx_id_tree_insert_array(this: u32, key: u32, array: u32) -> u32 {
        c2::id_tree_insert_array(&mut mem(), c2::SFX_ID_TREE, this, key, array)
    }
    /// `IdTree_insertArray_00710ba0(this; &key, &array)`.
    IdTree_insertArray_00710ba0 @ 0x0071_0ba0 => extern "thiscall" fn id_tree_insert_array(this: u32, key: u32, array: u32) -> u32 {
        c2::id_tree_insert_array(&mut mem(), c2::ID_TREE, this, key, array)
    }
    /// `FmodSfx_stopAllNonBlockingSound(this; keep, immediate)`.
    FmodSfx_stopAllNonBlockingSound @ 0x0070_d930 => extern "thiscall" fn stop_all_non_blocking(this: u32, keep: u32, immediate: u32) {
        c2::stop_all_non_blocking(&mut mem(), this, keep, immediate);
    }
    /// `FmodSfx_pause(this; paused)`.
    FmodSfx_pause @ 0x0070_d0c0 => extern "thiscall" fn sfx_pause(this: u32, paused: u32) {
        c2::sfx_pause(&mut mem(), this, paused);
    }
    /// The `PArray<PInputs::PInputAction>` class descriptor's constructor
    /// (`this`) -> `this`: `ffx_prender`'s descriptor bridge with its
    /// constants.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PInputs_PInputAction_4__PClassDescriptorAbstract @ 0x009d_fb30 => extern "thiscall" fn input_action_array_descriptor(this: u32) -> u32 {
        ffx_prender::registry::bridge::descriptor_ctor(&mut mem(), this, c2::INPUT_ACTION_ARRAY_DESCRIPTOR)
    }
    // ---- continuation round, batch 3 (ffx_glue_other::h4_c3)
    /// `FmodVoice_stop(this)`.
    FmodVoice_stop @ 0x0070_b6b0 => extern "thiscall" fn voice_stop(this: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        c3::voice_stop(&mut mem(), frame.at(), this)
    }
    /// `FmodVoice_releaseData(this)`.
    FmodVoice_releaseData @ 0x0070_b0b0 => extern "thiscall" fn voice_release_data(this: u32) -> u32 {
        c3::voice_release_data(&mut mem(), this)
    }
    /// `FmodVoice_updateState(this; arg)`: the microsecond clock's
    /// `edx:eax` through `call_cdecl_u64`.
    FmodVoice_updateState @ 0x0070_b750 => extern "thiscall" fn voice_update_state(this: u32, arg: u32) -> u32 {
        c3::voice_update_state(&mut mem(), this, arg, &mut |_, f| super::call_cdecl_u64(f, &[]))
    }
    /// `SyncManager_syncDataChangeAsync(this; id)`.
    SyncManager_syncDataChangeAsync @ 0x006f_b080 => extern "thiscall" fn sync_data_change_async(this: u32, id: u32) -> u32 {
        c3::sync_data_change_async(&mut mem(), this, id)
    }
    /// `InitVFXRenderDataShapeField(a, b, c, d, e)`.
    InitVFXRenderDataShapeField @ 0x0071_13d0 => extern "C" fn init_vfx_render_data_shape_field(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        c3::init_vfx_render_data_shape_field(&mut mem(), [a, b, c, d, e])
    }
    /// `MenuDynamicGeometry_setTextureInMaterialUnloaded(name)`.
    MenuDynamicGeometry_setTextureInMaterialUnloaded @ 0x006a_3740 => extern "stdcall" fn set_texture_in_material_unloaded(name: u32) -> u32 {
        c3::set_texture_in_material_unloaded(&mut mem(), name)
    }
    /// `SfxEventGroup_ctor_00710780(this; project, arg)`.
    SfxEventGroup_ctor_00710780 @ 0x0071_0780 => extern "thiscall" fn sfx_event_group_ctor(this: u32, project: u32, arg: u32) -> u32 {
        c3::sfx_event_group_ctor(&mut mem(), this, project, arg)
    }
    /// `FmodEventTree_ctor_0070ff50(this; project, arg)`.
    FmodEventTree_ctor_0070ff50 @ 0x0070_ff50 => extern "thiscall" fn fmod_event_tree_ctor(this: u32, project: u32, arg: u32) -> u32 {
        c3::fmod_event_tree_ctor(&mut mem(), this, project, arg)
    }
    /// `IggyMenuManager_dispatchCallback_00a26100(this; a, player, call)`.
    IggyMenuManager_dispatchCallback_00a26100 @ 0x00a2_6100 => extern "thiscall" fn dispatch_callback(this: u32, _a: u32, player: u32, call: u32) -> u32 {
        c3::dispatch_callback(&mut mem(), this, player, call)
    }
    /// `std_string_rfind_00638340(this; s, pos, n)`.
    std_string_rfind_00638340 @ 0x0063_8340 => extern "thiscall" fn string_rfind(this: u32, s: u32, pos: u32, n: u32) -> u32 {
        c3::string_rfind(&mut mem(), this, [s, pos, n])
    }
    /// `FmodMusic_clear(this; i)`.
    FmodMusic_clear @ 0x0070_8720 => extern "thiscall" fn music_clear(this: u32, i: u32) -> u32 {
        c3::music_clear(&mut mem(), this, i)
    }
    /// `FmodMusic_clearAll(this)`.
    FmodMusic_clearAll @ 0x0070_8810 => extern "thiscall" fn music_clear_all(this: u32) -> u32 {
        c3::music_clear_all(&mut mem(), this)
    }
    /// `FmodMusic_forceClearAll(this)`.
    FmodMusic_forceClearAll @ 0x0070_8af0 => extern "thiscall" fn music_force_clear_all(this: u32) {
        c3::music_force_clear_all(&mut mem(), this);
    }
    // ---- continuation round, batch 4 (ffx_glue_other::h4_c4)
    /// `BlurEffectEx_render(this)`.
    BlurEffectEx_render @ 0x006c_4cf0 => extern "thiscall" fn blur_ex_render(this: u32) -> u32 {
        c4::blur_ex_render(&mut mem(), this)
    }
    /// `FmodVoice_checkEvent_0070a4a0(this; system)`.
    FmodVoice_checkEvent_0070a4a0 @ 0x0070_a4a0 => extern "thiscall" fn voice_check_event(this: u32, system: u32) {
        let mut frame = StackFrame::<1>::new();
        c4::voice_check_event(&mut mem(), frame.at(), this, system);
    }
    /// `IggyMenu_callFlashFunction_6_006fcaf0(menu, name, six 16-byte
    /// values)`: 26 stack words.
    IggyMenu_callFlashFunction_6_006fcaf0 @ 0x006f_caf0 => extern "C" fn call_flash_6(menu: u32, name: u32, v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32, v6: u32, v7: u32, v8: u32, v9: u32, v10: u32, v11: u32, v12: u32, v13: u32, v14: u32, v15: u32, v16: u32, v17: u32, v18: u32, v19: u32, v20: u32, v21: u32, v22: u32, v23: u32) -> u32 {
        let mut frame = StackFrame::<{ (c4::FLASH_6_FRAME / 4) as usize }>::new();
        c4::call_flash_6(&mut mem(), frame.at(), menu, name, &[v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15, v16, v17, v18, v19, v20, v21, v22, v23])
    }
    /// `String_replaceFirst_00702590(text, from, to)`.
    String_replaceFirst_00702590 @ 0x0070_2590 => extern "C" fn string_replace_first(text: u32, from: u32, to: u32) -> u8 {
        c4::string_replace_first(&mut mem(), text, from, to)
    }
    /// `MenuTextureManager_UnloadMainMenuWaveTexture()`.
    MenuTextureManager_UnloadMainMenuWaveTexture @ 0x0067_efa0 => extern "C" fn unload_main_menu_wave_texture() {
        let mut frame = StackFrame::<{ (c4::WAVE_FRAME / 4) as usize }>::new();
        c4::unload_main_menu_wave_texture(&mut mem(), frame.at());
    }
    /// `MenuVertexInfo_move_0067dc70(from, to)`.
    MenuVertexInfo_move_0067dc70 @ 0x0067_dc70 => extern "stdcall" fn vertex_info_move(from: u32, to: u32) -> u32 {
        c4::vertex_info_move(&mut mem(), from, to)
    }
    // ---- continuation round 2, batch 1 (ffx_glue_other::h4_d1)
    /// `JobSchedule_pppPart_fillBuffer(parts, &start, &end)`: job-worker
    /// threads too.
    JobSchedule_pppPart_fillBuffer @ 0x0071_ed10 => extern "C" fn fill_buffer(parts: u32, start: u32, end: u32) -> u32 {
        d1::fill_buffer(&mut mem(), parts, start, end)
    }
    /// `JobSchedule_pppPart_fillBuffer_Kick(inline)`.
    JobSchedule_pppPart_fillBuffer_Kick @ 0x0071_edb0 => extern "C" fn fill_buffer_kick(inline: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::KICK_FRAME / 4) as usize }>::new();
        d1::fill_buffer_kick(&mut mem(), frame.at(), inline)
    }
    /// `IggyMenuManager_Render_00a267f0(this)`.
    IggyMenuManager_Render_00a267f0 @ 0x00a2_67f0 => extern "thiscall" fn iggy_manager_render(this: u32) -> u32 {
        d1::iggy_manager_render(&mut mem(), this)
    }
    /// `IggyMenuManager_addMenu_00a265f0(this; menu)`: its argument slot
    /// is a `StackFrame` word.
    IggyMenuManager_addMenu_00a265f0 @ 0x00a2_65f0 => extern "thiscall" fn iggy_manager_add_menu(this: u32, menu: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        d1::iggy_manager_add_menu(&mut mem(), frame.at(), this, menu)
    }
    /// `IggyGDraw_renderSetup_00a3ba50(this; renderer, &audio, arg)`.
    IggyGDraw_renderSetup_00a3ba50 @ 0x00a3_ba50 => extern "thiscall" fn gdraw_render_setup(this: u32, renderer: u32, audio: u32, arg: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::GDRAW_SETUP_FRAME / 4) as usize }>::new();
        d1::gdraw_render_setup(&mut mem(), frame.at(), this, renderer, audio, arg)
    }
    /// `SyncManager_readSyncFile(path)`.
    SyncManager_readSyncFile @ 0x006f_acf0 => extern "stdcall" fn read_sync_file(path: u32) -> u32 {
        d1::read_sync_file(&mut mem(), path)
    }
    /// `FmodFile_getDataDirectory_00679a10(name, out, size)`.
    FmodFile_getDataDirectory_00679a10 @ 0x0067_9a10 => extern "C" fn data_directory(name: u32, out: u32, size: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::DATA_DIRECTORY_FRAME / 4) as usize }>::new();
        d1::data_directory(&mut mem(), frame.at(), name, out, size)
    }
    /// `MenuTextureManager_UnloadSpecialMenuTexture(this; kind)`.
    MenuTextureManager_UnloadSpecialMenuTexture @ 0x0067_f830 => extern "thiscall" fn unload_special_menu_texture(this: u32, kind: u32) -> u32 {
        d1::unload_special_menu_texture(&mut mem(), this, kind)
    }
    /// `MenuTextureManager_LoadSpecialMenuTexture(this; kind, a, b)`.
    MenuTextureManager_LoadSpecialMenuTexture @ 0x0067_d550 => extern "thiscall" fn load_special_menu_texture(this: u32, kind: u32, a: u32, b: u32) -> u32 {
        d1::load_special_menu_texture(&mut mem(), this, kind, a, b)
    }
    /// `MenuTextureManager_PreLoadSaSubMenu(unused)`.
    MenuTextureManager_PreLoadSaSubMenu @ 0x0067_dd70 => extern "stdcall" fn preload_sa_sub_menu(_unused: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::PRELOAD_FRAME / 4) as usize }>::new();
        d1::preload_sa_sub_menu(&mut mem(), frame.at())
    }
    /// `Distortion_createRenderTarget(this)`.
    Distortion_createRenderTarget @ 0x006c_2350 => extern "thiscall" fn distortion_create_target(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::DISTORTION_CREATE_FRAME / 4) as usize }>::new();
        d1::distortion_create_target(&mut mem(), frame.at(), this)
    }
    /// `BlurEffectEx_setTextureParam_006c44e0(this; sampler, texture)`.
    BlurEffectEx_setTextureParam_006c44e0 @ 0x006c_44e0 => extern "thiscall" fn blur_set_texture_param(this: u32, sampler: u32, texture: u32) -> u32 {
        let mut frame = StackFrame::<{ (d1::BLUR_PARAM_FRAME / 4) as usize }>::new();
        d1::blur_set_texture_param(&mut mem(), frame.at(), this, sampler, texture)
    }
    /// `SfxEvent_isPlaying(this)` -> `al`.
    SfxEvent_isPlaying @ 0x0070_ec40 => extern "thiscall" fn sfx_event_is_playing(this: u32) -> u8 {
        let mut frame = StackFrame::<{ (d1::IS_PLAYING_FRAME / 4) as usize }>::new();
        d1::sfx_event_is_playing(&mut mem(), frame.at(), this)
    }
    /// `InitVFXRenderDataAtelShp(lo, hi, c, d, e)`.
    InitVFXRenderDataAtelShp @ 0x0071_11e0 => extern "C" fn init_vfx_render_data_atel_shp(lo: u32, hi: u32, c: u32, d: u32, e: u32) -> u32 {
        d1::init_vfx_render_data_atel_shp(&mut mem(), [lo, hi, c, d, e])
    }
    // ---- continuation round 2, batch 2 (ffx_glue_other::h4_d2)
    /// `FFEscMenu_playSound(menu, value)`: an Iggy callback.
    FFEscMenu_playSound @ 0x0068_f9e0 => extern "C" fn esc_play_sound(menu: u32, value: u32) -> u32 {
        d2::esc_play_sound(&mut mem(), menu, value)
    }
    /// `FFEscMenu_registerCallbacks(menu)`.
    FFEscMenu_registerCallbacks @ 0x0068_fd40 => extern "thiscall" fn esc_register_callbacks(menu: u32) -> u32 {
        d2::esc_register_callbacks(&mut mem(), menu)
    }
    /// `FFEscMenu_setLanguage_0068f400(menu)`.
    FFEscMenu_setLanguage_0068f400 @ 0x0068_f400 => extern "thiscall" fn esc_set_language(menu: u32) -> u32 {
        let mut frame = StackFrame::<{ (d2::SET_LANGUAGE_FRAME / 4) as usize }>::new();
        d2::esc_set_language(&mut mem(), frame.at(), menu)
    }
    /// `IggyCallbackTable_add_00a266b0(table; menu, name, fn)`.
    IggyCallbackTable_add_00a266b0 @ 0x00a2_66b0 => extern "thiscall" fn callback_table_add(this: u32, menu: u32, name: u32, f: u32) -> u32 {
        d2::callback_table_add(&mut mem(), this, menu, name, f)
    }
    /// `RemapButtonIcon_initPs2MessageInputPcMap(this)`.
    RemapButtonIcon_initPs2MessageInputPcMap @ 0x006e_6730 => extern "thiscall" fn init_message_input_map(this: u32) {
        d2::init_message_input_map(&mut mem(), this);
    }
    /// `RemapButtonIcon_initPs2UIButtonPcMap(this)`.
    RemapButtonIcon_initPs2UIButtonPcMap @ 0x006e_6890 => extern "thiscall" fn init_ui_button_map(this: u32) {
        d2::init_ui_button_map(&mut mem(), this);
    }
    /// `MenuManager_gatherMenuBuffers_0067b530(this; which)`.
    MenuManager_gatherMenuBuffers_0067b530 @ 0x0067_b530 => extern "thiscall" fn gather_menu_buffers(this: u32, which: u32) -> u32 {
        d2::gather_menu_buffers(&mut mem(), this, which)
    }
    /// `MenuManager_SetMenuUiInstanceUnloaded(this; name)`.
    MenuManager_SetMenuUiInstanceUnloaded @ 0x0067_e4f0 => extern "thiscall" fn set_menu_ui_instance_unloaded(this: u32, name: u32) -> u32 {
        d2::set_menu_ui_instance_unloaded(&mut mem(), this, name)
    }
    /// `GuiTouch_reset_0068a660(this)`.
    GuiTouch_reset_0068a660 @ 0x0068_a660 => extern "thiscall" fn touch_reset(this: u32) -> u32 {
        d2::touch_reset(&mut mem(), this)
    }
    /// `ShaderTexture_rebind_006f6c00(this; owner, force)`.
    ShaderTexture_rebind_006f6c00 @ 0x006f_6c00 => extern "thiscall" fn rebind_textures(this: u32, owner: u32, force: u32; slots: u32) {
        let mut frame = StackFrame::<{ (d2::REBIND_FRAME / 4) as usize }>::new();
        d2::rebind_textures(&mut mem(), frame.at(), slots, this, owner, force);
    }
    /// `ShaderTexture_rebindAlt_006f6a90(this; owner, alt, force)`.
    ShaderTexture_rebindAlt_006f6a90 @ 0x006f_6a90 => extern "thiscall" fn rebind_textures_alt(this: u32, owner: u32, alt: u32, force: u32; slots: u32) {
        let mut frame = StackFrame::<{ (d2::REBIND_ALT_FRAME / 4) as usize }>::new();
        d2::rebind_textures_alt(&mut mem(), frame.at(), slots, this, owner, alt, force);
    }
    // ---- continuation round 2, batch 3 (ffx_glue_other::h4_d3)
    /// `MenuTextureManager_UnloadSceneBattleFontTexture(this)`.
    MenuTextureManager_UnloadSceneBattleFontTexture @ 0x0067_f3e0 => extern "thiscall" fn unload_battle_font(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d3::UNLOAD_FONT_FRAME / 4) as usize }>::new();
        d3::unload_font_texture(&mut mem(), frame.at(), this, 5, 0x106)
    }
    /// `MenuTextureManager_UnloadSceneEventFontTexture(this)`.
    MenuTextureManager_UnloadSceneEventFontTexture @ 0x0067_f610 => extern "thiscall" fn unload_event_font(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d3::UNLOAD_FONT_FRAME / 4) as usize }>::new();
        d3::unload_font_texture(&mut mem(), frame.at(), this, 4, 6)
    }
    /// `MenuTextureManager_LoadSceneBattleFontTexture(this; prefix)`.
    MenuTextureManager_LoadSceneBattleFontTexture @ 0x0067_d180 => extern "thiscall" fn load_battle_font(this: u32, prefix: u32) -> u32 {
        let mut frame = StackFrame::<{ (d3::LOAD_FONT_FRAME / 4) as usize }>::new();
        d3::load_battle_font_texture(&mut mem(), frame.at(), this, prefix)
    }
    /// `MenuTextureManager_LoadSaMenuTexture(this; arg)`.
    MenuTextureManager_LoadSaMenuTexture @ 0x0067_ce50 => extern "thiscall" fn load_sa_menu_texture(this: u32, arg: u32) -> u32 {
        d3::load_sa_menu_texture(&mut mem(), this, arg)
    }
    /// `MenuTextureManager_LoadMon_BokuTextures()`: reads no `ecx`.
    MenuTextureManager_LoadMon_BokuTextures @ 0x0067_cbd0 => extern "C" fn load_mon_boku_textures() -> u32 {
        d3::load_mon_boku_textures(&mut mem())
    }
    /// `MenuTextureManager_LoadLanguageTextures_0067c920(this; lang)`.
    MenuTextureManager_LoadLanguageTextures_0067c920 @ 0x0067_c920 => extern "thiscall" fn load_language_textures(this: u32, lang: u32) -> u32 {
        d3::load_language_textures(&mut mem(), this, lang)
    }
    /// `Distortion_buildGridIndices_006c2990(this)`.
    Distortion_buildGridIndices_006c2990 @ 0x006c_2990 => extern "thiscall" fn distortion_grid_indices(this: u32) -> u32 {
        d3::distortion_grid_indices(&mut mem(), this)
    }
    /// `LoadingScreen_loadTextures(this)`.
    LoadingScreen_loadTextures @ 0x006e_4770 => extern "thiscall" fn loading_screen_load_textures(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d3::LOADING_FRAME / 4) as usize }>::new();
        d3::loading_screen_load_textures(&mut mem(), frame.at(), this)
    }
    /// `Fmod_PushCommand(manager; cmd, a, b, c, d)`: game and loader
    /// threads; the new write index also goes into `a`'s slot.
    Fmod_PushCommand @ 0x006f_a300 => extern "thiscall" fn push_command(this: u32, cmd: u32, a: u32, b: u32, c: u32, d: u32; slots: u32) -> u32 {
        d3::push_command(&mut mem(), slots, this, [cmd, a, b, c, d])
    }
    /// `FmodManager_registCommandSync(manager; cmd, a, b, c, d)`.
    FmodManager_registCommandSync @ 0x006f_a500 => extern "thiscall" fn regist_command_sync(this: u32, cmd: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        d3::regist_command_sync(&mut mem(), this, [cmd, a, b, c, d])
    }
    /// `TextureAnimationSlots_ctor_006af420(this; n)`.
    TextureAnimationSlots_ctor_006af420 @ 0x006a_f420 => extern "thiscall" fn slot_table_ctor(this: u32, n: u32) -> u32 {
        d3::slot_table_ctor(&mut mem(), this, n)
    }
    /// `RadialLine_ctor_006f7a30(this)`.
    RadialLine_ctor_006f7a30 @ 0x006f_7a30 => extern "thiscall" fn radial_line_ctor(this: u32) -> u32 {
        d3::radial_line_ctor(&mut mem(), this)
    }
    /// `RemapButtonIcon_getPcButtonUv(key, &u, &v)`.
    RemapButtonIcon_getPcButtonUv @ 0x006e_6670 => extern "stdcall" fn pc_button_uv(key: u32, u: u32, v: u32) -> u32 {
        d3::pc_button_uv(&mut mem(), key, u, v)
    }
    /// `RemapButtonIcon_getJoypadUV(this; key, &u, &v)`.
    RemapButtonIcon_getJoypadUV @ 0x006e_6460 => extern "thiscall" fn joypad_uv(this: u32, key: u32, u: u32, v: u32) -> u32 {
        d3::joypad_uv(&mut mem(), this, key, u, v)
    }
    // ---- continuation round 2, batch 4 (ffx_glue_other::h4_d4)
    /// `IggyMenu_callFlashFunction_8_006ffaa0(menu, name, eight 16-byte
    /// values)`: 34 stack words.
    IggyMenu_callFlashFunction_8_006ffaa0 @ 0x006f_faa0 => extern "C" fn call_flash_8(menu: u32, name: u32, v0: u32, v1: u32, v2: u32, v3: u32, v4: u32, v5: u32, v6: u32, v7: u32, v8: u32, v9: u32, v10: u32, v11: u32, v12: u32, v13: u32, v14: u32, v15: u32, v16: u32, v17: u32, v18: u32, v19: u32, v20: u32, v21: u32, v22: u32, v23: u32, v24: u32, v25: u32, v26: u32, v27: u32, v28: u32, v29: u32, v30: u32, v31: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::FLASH_8_FRAME / 4) as usize }>::new();
        d4::call_flash_8(&mut mem(), frame.at(), menu, name, &[v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15, v16, v17, v18, v19, v20, v21, v22, v23, v24, v25, v26, v27, v28, v29, v30, v31])
    }
    /// `yiRender_setTextureAnimFrame_0090bdb0(base, a, b)`.
    yiRender_setTextureAnimFrame_0090bdb0 @ 0x0090_bdb0 => extern "C" fn set_texture_anim_frame(base: u32, a: u32, b: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::ANIM_FRAME_FRAME / 4) as usize }>::new();
        d4::set_texture_anim_frame(&mut mem(), frame.at(), base, a, b)
    }
    /// `SoftParticle_resetSpecularMaps_006c5850(this)`.
    SoftParticle_resetSpecularMaps_006c5850 @ 0x006c_5850 => extern "thiscall" fn reset_specular_maps(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::SPECULAR_FRAME / 4) as usize }>::new();
        d4::reset_specular_maps(&mut mem(), frame.at(), this)
    }
    /// `SyncManager_initData(this)`.
    SyncManager_initData @ 0x006f_a8d0 => extern "thiscall" fn sync_init_data(this: u32) -> u32 {
        d4::sync_init_data(&mut mem(), this)
    }
    /// `FmodVoice_initList(this)`.
    FmodVoice_initList @ 0x0070_ab90 => extern "thiscall" fn voice_init_list(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::VOICE_LIST_FRAME / 4) as usize }>::new();
        d4::voice_init_list(&mut mem(), frame.at(), this)
    }
    /// `FmodMusic_loadEventProject(system, name, &project, &group)`.
    FmodMusic_loadEventProject @ 0x0070_8d40 => extern "stdcall" fn load_event_project(system: u32, name: u32, project: u32, group: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::PROJECT_FRAME / 4) as usize }>::new();
        d4::load_event_project(&mut mem(), frame.at(), system, name, project, group)
    }
    /// `FmodShout_initData(this; system, flag)`.
    FmodShout_initData @ 0x0070_e2d0 => extern "thiscall" fn shout_init_data(this: u32, system: u32, flag: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::SHOUT_FRAME / 4) as usize }>::new();
        d4::shout_init_data(&mut mem(), frame.at(), this, system, flag)
    }
    /// `FmodVoice_initData(this; event_system, system)`.
    FmodVoice_initData @ 0x0070_aa30 => extern "thiscall" fn voice_init_data(this: u32, event_system: u32, system: u32) -> u32 {
        let mut frame = StackFrame::<{ (d4::PROJECT_FRAME / 4) as usize }>::new();
        d4::voice_init_data(&mut mem(), frame.at(), this, event_system, system)
    }
    // ---- continuation round 2, batch 5 (ffx_glue_other::h4_d5)
    /// `TODrawMessage(i)`.
    TODrawMessage @ 0x008b_5dc0 => extern "C" fn draw_message(i: u32) -> u32 {
        d5::draw_message(&mut mem(), i)
    }
    /// `TODrawMessageWindow()`.
    TODrawMessageWindow @ 0x008a_bd30 => extern "C" fn draw_message_window() -> u32 {
        let mut frame = StackFrame::<{ (d5::PASS_FRAME / 4) as usize }>::new();
        d5::draw_message_window(&mut mem(), frame.at())
    }
    /// `TODrawMessageWindowForMenu()`.
    TODrawMessageWindowForMenu @ 0x008a_be40 => extern "C" fn draw_message_window_for_menu() -> u32 {
        let mut frame = StackFrame::<{ (d5::PASS_FRAME / 4) as usize }>::new();
        d5::draw_message_window_for_menu(&mut mem(), frame.at())
    }
    /// `IggyMenu_handleMouseEvent_00a28c10(menu; event, result)`.
    IggyMenu_handleMouseEvent_00a28c10 @ 0x00a2_8c10 => extern "thiscall" fn menu_mouse_event(this: u32, event: u32, result: u32) -> u32 {
        let mut frame = StackFrame::<{ (d5::MOUSE_FRAME / 4) as usize }>::new();
        d5::menu_mouse_event(&mut mem(), frame.at(), this, event, result)
    }
    // ---- continuation round 3, batch 1 (ffx_glue_other::h4_e1)
    /// `TkMenuEquip_DrawCursor_008cf990()`.
    TkMenuEquip_DrawCursor_008cf990 @ 0x008c_f990 => extern "C" fn equip_cursor() {
        e1::equip_cursor(&mut mem());
    }
    /// `MenuWeapon_DrawListCursor_008d0100(list, x, y)`.
    MenuWeapon_DrawListCursor_008d0100 @ 0x008d_0100 => extern "C" fn weapon_list_cursor(list: u32, x: u32, y: u32) {
        e1::weapon_list_cursor(&mut mem(), list, x, y);
    }
    /// `MenuWShop_DrawBuyListCursor_008d8bb0(list, x, y)`.
    MenuWShop_DrawBuyListCursor_008d8bb0 @ 0x008d_8bb0 => extern "C" fn shop_buy_list_cursor(list: u32, x: u32, y: u32) -> u32 {
        e1::shop_list_cursor(&mut mem(), list, x, y)
    }
    /// `MenuWShop_DrawSellListCursor_008d8e60(list, x, y)`: a copy of the
    /// buy list's.
    MenuWShop_DrawSellListCursor_008d8e60 @ 0x008d_8e60 => extern "C" fn shop_sell_list_cursor(list: u32, x: u32, y: u32) -> u32 {
        e1::shop_list_cursor(&mut mem(), list, x, y)
    }
    /// `MenuWShop_BuyListCursor_008dab70(list, x, y)`.
    MenuWShop_BuyListCursor_008dab70 @ 0x008d_ab70 => extern "C" fn shop_buy_cursor(list: u32, x: u32, y: u32) -> u32 {
        e1::shop_buy_cursor(&mut mem(), list, x, y)
    }
    /// `MenuWShop_SellWindowCursor_008dae00(list, x, y)`.
    MenuWShop_SellWindowCursor_008dae00 @ 0x008d_ae00 => extern "C" fn shop_sell_cursor(list: u32, x: u32, y: u32) -> u32 {
        e1::shop_sell_cursor(&mut mem(), list, x, y)
    }
    /// `MenuItem_DrawListCursor_008c7af0(list, x, y)`.
    MenuItem_DrawListCursor_008c7af0 @ 0x008c_7af0 => extern "C" fn item_list_cursor(list: u32, x: u32, y: u32) -> u32 {
        e1::item_list_cursor(&mut mem(), list, x, y).unwrap_or(0)
    }
    /// `TkMn2DrawCrossCmdCursor(x, y, blink)`.
    TkMn2DrawCrossCmdCursor @ 0x008c_0580 => extern "C" fn cross_cmd_cursor(x: u32, y: u32, blink: u32) -> u32 {
        e1::cross_cmd_cursor(&mut mem(), x, y, blink).unwrap_or(0)
    }
    /// `TkMn2DrawStatiCursor(x, y, hidden)`.
    TkMn2DrawStatiCursor @ 0x008c_13d0 => extern "C" fn stati_cursor(x: u32, y: u32, hidden: u32) {
        e1::stati_cursor(&mut mem(), x, y, hidden);
    }
    /// `TkMn2DrawBrush(a, x, y)`.
    TkMn2DrawBrush @ 0x008b_f830 => extern "C" fn draw_brush(_a: u32, x: u32, y: u32) {
        e1::draw_brush(&mut mem(), x, y);
    }
    /// `TOMenuDrawBrush2(i)`.
    TOMenuDrawBrush2 @ 0x008f_f260 => extern "C" fn draw_brush2(i: u32) {
        e1::draw_brush2(&mut mem(), i);
    }
    /// `TODrawMenuBGC2(a, b)`.
    TODrawMenuBGC2 @ 0x008f_5c60 => extern "C" fn draw_menu_bg_c2(a: u32, b: u32) {
        e1::draw_menu_bg_c2(&mut mem(), a, b);
    }
    /// `TkMn2DrawStatusBG(colour, kind)`.
    TkMn2DrawStatusBG @ 0x008d_4170 => extern "C" fn draw_status_bg(colour: u32, kind: u32) {
        e1::draw_status_bg(&mut mem(), colour, kind);
    }
    /// `Menu_DrawWeaponAbilities_008d8aa0(x, y, weapon)`.
    Menu_DrawWeaponAbilities_008d8aa0 @ 0x008d_8aa0 => extern "C" fn weapon_abilities(x: u32, y: u32, weapon: u32) {
        e1::weapon_abilities(&mut mem(), x, y, weapon);
    }
    /// `MenuEquip_DrawWeaponAbilities_008d02e0()`.
    MenuEquip_DrawWeaponAbilities_008d02e0 @ 0x008d_02e0 => extern "C" fn equip_weapon_abilities() -> u32 {
        let mut frame = StackFrame::<{ (e1::EQUIP_ABILITIES_FRAME / 4) as usize }>::new();
        e1::equip_weapon_abilities(&mut mem(), frame.at()).unwrap_or(0)
    }
    /// `TOBtlDrawTargetCursor()`.
    TOBtlDrawTargetCursor @ 0x0089_c870 => extern "C" fn target_cursor() {
        let mut frame = StackFrame::<{ (e1::TARGET_FRAME / 4) as usize }>::new();
        e1::target_cursor(&mut mem(), frame.at());
    }
    /// `TOBtlDrawEnemyLimit()`.
    TOBtlDrawEnemyLimit @ 0x0089_33c0 => extern "C" fn enemy_limit() {
        let mut frame = StackFrame::<{ (e1::ENEMY_LIMIT_FRAME / 4) as usize }>::new();
        e1::enemy_limit(&mut mem(), frame.at());
    }
    // ---- continuation round 3, batch 2 (ffx_glue_other::h4_e2)
    /// `FmodMusic_volumeDirect(this; i, volume, keep)`.
    FmodMusic_volumeDirect @ 0x0070_9dd0 => extern "thiscall" fn music_volume_direct(this: u32, i: u32, volume: u32, keep: u32) -> u32 {
        e2::music_volume_direct(&mut mem(), this, i, volume, keep)
    }
    /// `FmodMusic_setEntireVolume(this; volume)`.
    FmodMusic_setEntireVolume @ 0x0070_93e0 => extern "thiscall" fn music_set_entire_volume(this: u32, volume: u32) {
        e2::music_set_entire_volume(&mut mem(), this, volume);
    }
    /// `FmodMusic_volumeTranslate(this; i, frames, target)`.
    FmodMusic_volumeTranslate @ 0x0070_9ff0 => extern "thiscall" fn music_volume_translate(this: u32, i: u32, frames: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<{ (e2::TRANSLATE_FRAME / 4) as usize }>::new();
        e2::music_volume_translate(&mut mem(), frame.at(), this, i, frames, target)
    }
    /// `FmodMusic_releaseOldMusic(this; keep)`.
    FmodMusic_releaseOldMusic @ 0x0070_91b0 => extern "thiscall" fn release_old_music(this: u32, keep: u32) {
        let mut frame = StackFrame::<{ (e2::RELEASE_OLD_FRAME / 4) as usize }>::new();
        e2::release_old_music(&mut mem(), frame.at(), this, keep);
    }
    /// `FmodVoice_loopVolumeTranslate(this)`.
    FmodVoice_loopVolumeTranslate @ 0x0070_acf0 => extern "thiscall" fn voice_loop_volume(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (e2::VOICE_LOOP_FRAME / 4) as usize }>::new();
        e2::voice_loop_volume(&mut mem(), frame.at(), this)
    }
    /// `SfxEvent_applyPan(this; pan)`.
    SfxEvent_applyPan @ 0x0070_eeb0 => extern "thiscall" fn sfx_event_apply_pan(this: u32, pan: u32) {
        let mut frame = StackFrame::<{ (e2::APPLY_PAN_FRAME / 4) as usize }>::new();
        e2::sfx_event_apply_pan(&mut mem(), frame.at(), this, pan);
    }
    /// `SfxEvent_stop(this; immediate)`.
    SfxEvent_stop @ 0x0070_f490 => extern "thiscall" fn sfx_event_stop(this: u32, immediate: u32) {
        let mut frame = StackFrame::<{ (e2::STOP_FRAME / 4) as usize }>::new();
        e2::sfx_event_stop(&mut mem(), frame.at(), this, immediate);
    }
    /// `SyncManager_syncGetData(this; &x, &y, &id)`.
    SyncManager_syncGetData @ 0x006f_b160 => extern "thiscall" fn sync_get_data(this: u32, x: u32, y: u32, id: u32) -> u32 {
        e2::sync_get_data(&mut mem(), this, x, y, id)
    }
    // ---- continuation round 3, batch 3 (ffx_glue_other::h4_e3)
    /// `FmodMusic_stop(this; i)`.
    FmodMusic_stop @ 0x0070_99a0 => extern "thiscall" fn music_stop(this: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (e3::STOP_FRAME / 4) as usize }>::new();
        e3::music_stop(&mut mem(), frame.at(), this, i)
    }
    /// `FmodMusic_stopAll(this)`.
    FmodMusic_stopAll @ 0x0070_9b10 => extern "thiscall" fn music_stop_all(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (e3::STOP_ALL_FRAME / 4) as usize }>::new();
        e3::music_stop_all(&mut mem(), frame.at(), this)
    }
    /// `FmodMusic_restart(this; i)`.
    FmodMusic_restart @ 0x0070_9270 => extern "thiscall" fn music_restart(this: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (e3::RESTART_FRAME / 4) as usize }>::new();
        e3::music_restart(&mut mem(), frame.at(), this, i)
    }
    /// `FmodMusic_loopVolumeTranslate(this)`.
    FmodMusic_loopVolumeTranslate @ 0x0070_8e50 => extern "thiscall" fn music_loop_volume(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (e3::LOOP_FRAME / 4) as usize }>::new();
        e3::music_loop_volume(&mut mem(), frame.at(), this)
    }
    /// `SfxEvent_start(this; volume, pan)`.
    SfxEvent_start @ 0x0070_f310 => extern "thiscall" fn sfx_event_start(this: u32, volume: u32, pan: u32) {
        e3::sfx_event_start(&mut mem(), this, volume, pan);
    }
    /// `SfxEvent_updateFades_0070f5c0(this; dt)`.
    SfxEvent_updateFades_0070f5c0 @ 0x0070_f5c0 => extern "thiscall" fn sfx_event_update_fades(this: u32, dt: u32) {
        e3::sfx_event_update_fades(&mut mem(), this, dt);
    }
    // ---- continuation round 3, batch 4 (ffx_glue_other::h4_e4)
    /// `TOMkpActiveChrCursor(x0, y0, x1, y1, x2, y2, c1, c0)`.
    TOMkpActiveChrCursor @ 0x0090_1190 => extern "C" fn active_chr_cursor(x0: u32, y0: u32, x1: u32, y1: u32, x2: u32, y2: u32, c1: u32, c0: u32) -> u32 {
        let mut rec = StackFrame::<{ (e4::RECORD / 4) as usize }>::new();
        e4::active_chr_cursor(&mut mem(), rec.at(), [x0, y0, x1, y1, x2, y2], c1, c0)
    }
    /// `TOSeqDrawRect_008fdf20(p)`.
    TOSeqDrawRect_008fdf20 @ 0x008f_df20 => extern "C" fn seq_draw_rect(p: u32) -> u32 {
        let mut rec = StackFrame::<{ (e4::RECORD / 4) as usize }>::new();
        e4::seq_draw_rect(&mut mem(), rec.at(), p)
    }
    /// `TODrawCrossBoxXYWHC2Mode2_008f4df0(x, y, w, h, c0, c1)`.
    TODrawCrossBoxXYWHC2Mode2_008f4df0 @ 0x008f_4df0 => extern "C" fn cross_box_mode2(x: u32, y: u32, w: u32, h: u32, c0: u32, c1: u32) -> u32 {
        let mut frame = StackFrame::<{ (e4::BOX_FRAME / 4) as usize }>::new();
        e4::cross_box_mode2(&mut mem(), frame.at(), [x, y, w, h, c0, c1])
    }
    /// `Distortion_fillGridOnes_006c28e0(this)`.
    Distortion_fillGridOnes_006c28e0 @ 0x006c_28e0 => extern "thiscall" fn distortion_fill_ones(this: u32) -> u32 {
        e4::distortion_fill_ones(&mut mem(), this)
    }
    /// `RemapButtonIcon_remapIconUV(kind, rec, u, v)`.
    RemapButtonIcon_remapIconUV @ 0x006e_6f20 => extern "stdcall" fn remap_icon_uv(kind: u32, rec: u32, u: u32, v: u32) {
        e4::remap_icon_uv(&mut mem(), kind, rec, u, v);
    }
    /// `RemapButtonIcon_remapIconSize(this; kind, rec)`.
    RemapButtonIcon_remapIconSize @ 0x006e_6db0 => extern "thiscall" fn remap_icon_size(this: u32, kind: u32, rec: u32) {
        e4::remap_icon_size(&mut mem(), this, kind, rec);
    }
    /// `RemapButtonIcon_remapUIButtonPosition(a, rec, b, kind)`.
    RemapButtonIcon_remapUIButtonPosition @ 0x006e_7470 => extern "stdcall" fn remap_ui_button_position(_a: u32, rec: u32, _b: u32, kind: u32) -> u32 {
        e4::remap_ui_button_position(&mut mem(), rec, kind)
    }
    // ---- continuation round 3, batch 5 (ffx_glue_other::h4_e5)
    /// `FmodMusic_start(this; i)`.
    FmodMusic_start @ 0x0070_96f0 => extern "thiscall" fn music_start(this: u32, i: u32) -> u32 {
        let mut frame = StackFrame::<{ (e5::START_FRAME / 4) as usize }>::new();
        e5::music_start(&mut mem(), frame.at(), this, i)
    }
    /// `FmodMusic_read(this; i, flag)`.
    FmodMusic_read @ 0x0070_9080 => extern "thiscall" fn music_read(this: u32, i: u32, flag: u32) -> u32 {
        e5::music_read(&mut mem(), this, i, flag)
    }
    /// `FmodMusic_dataSend(this; a, flag)`.
    FmodMusic_dataSend @ 0x0070_8a00 => extern "thiscall" fn music_data_send(this: u32, _a: u32, flag: u32) -> u32 {
        e5::music_data_send(&mut mem(), this, flag)
    }
    /// `FmodMusic_crossPlay(this; i, frames, flag)`.
    FmodMusic_crossPlay @ 0x0070_8900 => extern "thiscall" fn music_cross_play(this: u32, i: u32, frames: u32, flag: u32) -> u32 {
        e5::music_cross_play(&mut mem(), this, i, frames, flag)
    }
    /// `FmodMusic_switchBGM(this; bgm, flag)`.
    FmodMusic_switchBGM @ 0x0070_9c80 => extern "thiscall" fn music_switch_bgm(this: u32, bgm: u32, flag: u32) {
        e5::music_switch_bgm(&mut mem(), this, bgm, flag);
    }
    // ---- continuation round 4, batch 1 (ffx_glue_other::h4_f1)
    /// `FmodVoice_start(this; pan, volume)`.
    FmodVoice_start @ 0x0070_b5c0 => extern "thiscall" fn voice_start(this: u32, _pan: u32, _volume: u32) -> u32 {
        let mut frame = StackFrame::<{ (f1::VOICE_FRAME / 4) as usize }>::new();
        f1::voice_start(&mut mem(), frame.at(), this)
    }
    /// `FmodVoice_restart(this)`.
    FmodVoice_restart @ 0x0070_b160 => extern "thiscall" fn voice_restart(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (f1::VOICE_FRAME / 4) as usize }>::new();
        f1::voice_restart(&mut mem(), frame.at(), this)
    }
    /// `FfxFmod_soundLoop(this)`.
    FfxFmod_soundLoop @ 0x0070_70d0 => extern "thiscall" fn sound_loop(this: u32) -> u32 {
        f1::sound_loop(&mut mem(), this)
    }
    /// `FmodSfx_setEntireVolume(this; volume)`.
    FmodSfx_setEntireVolume @ 0x0070_d500 => extern "thiscall" fn sfx_set_entire_volume(this: u32, volume: u32) {
        f1::sfx_set_entire_volume(&mut mem(), this, volume);
    }
    /// `FmodSfx_unloadProjects(this; all, _)`.
    FmodSfx_unloadProjects @ 0x0070_dd10 => extern "thiscall" fn sfx_unload_projects(this: u32, all: u32, _b: u32) -> u32 {
        f1::sfx_unload_projects(&mut mem(), this, all)
    }
    /// `SfxEvent_setVolume2(this; volume, frames)`.
    SfxEvent_setVolume2 @ 0x0070_f210 => extern "thiscall" fn sfx_event_set_volume2(this: u32, volume: u32, frames: u32) -> u32 {
        let mut frame = StackFrame::<{ (f1::SET_VOLUME2_FRAME / 4) as usize }>::new();
        f1::sfx_event_set_volume2(&mut mem(), frame.at(), this, volume, frames).unwrap_or(0)
    }
    /// `MenuManager_MenuInstanceSetup(this; rec, name, big)`: 64 bits in `edx:eax`.
    MenuManager_MenuInstanceSetup @ 0x0067_d760 => extern "thiscall" fn menu_instance_setup(this: u32, rec: u32, name: u32, big: u32) -> u64 {
        f1::menu_instance_setup(&mut mem(), this, rec, name, big)
    }
    /// `MenuManager_SetUpAllMenuInstance(this)`: the last setup's `edx:eax`.
    MenuManager_SetUpAllMenuInstance @ 0x0067_e6c0 => extern "thiscall" fn menu_set_up_all(this: u32) -> u64 {
        f1::menu_set_up_all(&mut mem(), this)
    }
    /// `MenuManager_SwapMenuBufferChain(this)`.
    MenuManager_SwapMenuBufferChain @ 0x0067_e7f0 => extern "thiscall" fn menu_swap_buffer_chain(this: u32) {
        f1::menu_swap_buffer_chain(&mut mem(), this);
    }
    // ---- continuation round 4, batch 2 (ffx_glue_other::h4_f2)
    /// `PRenderThread_setShaderPass_0061c400(this; pass)`.
    PRenderThread_setShaderPass_0061c400 @ 0x0061_c400 => extern "thiscall" fn set_shader_pass(this: u32, pass: u32) -> u32 {
        f2::set_shader_pass(&mut mem(), this, pass)
    }
    /// `MenuManager_MenuInstanceUpdateMaterialShader(this; slot, rec)`.
    MenuManager_MenuInstanceUpdateMaterialShader @ 0x0067_d920 => extern "thiscall" fn menu_update_material(_this: u32, slot: u32, rec: u32) -> u32 {
        let mut frame = StackFrame::<{ (f2::UPDATE_MATERIAL_FRAME / 4) as usize }>::new();
        f2::menu_update_material(&mut mem(), frame.at(), slot, rec)
    }
    /// `MenuManager_resetMenuBufferChain_0067dfc0(this)`.
    MenuManager_resetMenuBufferChain_0067dfc0 @ 0x0067_dfc0 => extern "thiscall" fn menu_reset_buffer_chain(this: u32) {
        f2::menu_reset_buffer_chain(&mut mem(), this);
    }
    /// `AbmapManager_GetVertexInfo(this; name, kind)`.
    AbmapManager_GetVertexInfo @ 0x0068_4cd0 => extern "thiscall" fn abmap_vertex_info(this: u32, name: u32, kind: u32) -> u32 {
        f2::abmap_vertex_info(&mut mem(), this, name, kind)
    }
    /// `SfxEvent_ctor_0070e710(this; src, volume)`.
    SfxEvent_ctor_0070e710 @ 0x0070_e710 => extern "thiscall" fn sfx_event_ctor(this: u32, src: u32, volume: u32) -> u32 {
        f2::sfx_event_ctor(&mut mem(), this, src, volume)
    }
    /// `SfxEvent_dtor_0070e850(this)`.
    SfxEvent_dtor_0070e850 @ 0x0070_e850 => extern "thiscall" fn sfx_event_dtor(this: u32) {
        f2::sfx_event_dtor(&mut mem(), this);
    }
    // ---- continuation round 4, batch 3 (ffx_glue_other::h4_f3)
    /// `FmodSfx_update(this; dt)`.
    FmodSfx_update @ 0x0070_dec0 => extern "thiscall" fn sfx_update(this: u32, dt: u32) {
        f3::sfx_update(&mut mem(), this, dt);
    }
    /// `FmodSfx_createSfxEvent(this; id)`.
    FmodSfx_createSfxEvent @ 0x0070_c8f0 => extern "thiscall" fn sfx_create_event(this: u32, id: u32) -> u32 {
        let mut frame = StackFrame::<{ (f3::CREATE_FRAME / 4) as usize }>::new();
        f3::sfx_create_event(&mut mem(), frame.at(), this, id)
    }
    /// `FmodSfx_playBattleStreaming(this; a, id, volume, pan)`.
    FmodSfx_playBattleStreaming @ 0x0070_d190 => extern "thiscall" fn sfx_play_battle_streaming(this: u32, a: u32, id: u32, volume: u32, pan: u32) {
        let mut frame = StackFrame::<{ (f3::STREAM_FRAME / 4) as usize }>::new();
        f3::sfx_play_battle_streaming(&mut mem(), frame.at(), this, [a, id, volume, pan]);
    }
    /// `FmodSfx_loadProject(this; a, id, resident)`.
    FmodSfx_loadProject @ 0x0070_cf10 => extern "thiscall" fn sfx_load_project(this: u32, a: u32, id: u32, resident: u32) {
        let mut frame = StackFrame::<{ (f3::LOAD_FRAME / 4) as usize }>::new();
        f3::sfx_load_project(&mut mem(), frame.at(), this, [a, id, resident]);
    }
    /// `FmodSfx_ctor_0070bfc0(this)`.
    FmodSfx_ctor_0070bfc0 @ 0x0070_bfc0 => extern "thiscall" fn sfx_ctor(this: u32) -> u32 {
        f3::sfx_ctor(&mut mem(), this)
    }
    // ---- continuation round 4, batch 4 (ffx_glue_other::h4_f4)
    /// `FfxFmod_initEventSystem(this)`.
    FfxFmod_initEventSystem @ 0x0070_6820 => extern "thiscall" fn init_event_system(this: u32) -> u32 {
        f4::init_event_system(&mut mem(), this)
    }
    /// `SfxBank_ctor_0070f760(this; system, path)`.
    SfxBank_ctor_0070f760 @ 0x0070_f760 => extern "thiscall" fn sfx_bank_ctor(this: u32, system: u32, path: u32) -> u32 {
        let mut frame = StackFrame::<{ (f4::BANK_FRAME / 4) as usize }>::new();
        f4::sfx_bank_ctor(&mut mem(), frame.at(), this, system, path)
    }
    /// `FmodEventTree_load_00710370(this; path)`.
    FmodEventTree_load_00710370 @ 0x0071_0370 => extern "thiscall" fn event_tree_load(this: u32, path: u32) -> u32 {
        f4::event_tree_load(&mut mem(), this, path)
    }
    // ---- continuation round 4, batch 5 (ffx_glue_other::h4_f5)
    /// `FmodVoice_read(this; id, flag)`.
    FmodVoice_read @ 0x0070_add0 => extern "thiscall" fn voice_read(this: u32, id: u32, flag: u32) -> u32 {
        f5::voice_read(&mut mem(), this, id, flag)
    }
    /// `SyncManager_syncInit(this)`.
    SyncManager_syncInit @ 0x006f_b210 => extern "thiscall" fn sync_init(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (f5::SYNC_INIT_FRAME / 4) as usize }>::new();
        f5::sync_init(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 4, batch 6 (ffx_glue_other::h4_f6)
    /// `TKMallocAndReadMenuMessage(name, out)`.
    TKMallocAndReadMenuMessage @ 0x0088_ccf0 => extern "C" fn read_menu_message(name: u32, out: u32) -> u32 {
        let mut frame = StackFrame::<{ (f6::MESSAGE_FRAME / 4) as usize }>::new();
        f6::read_menu_message(&mut mem(), frame.at(), name, out)
    }
    // ---- continuation round 4, batch 7 (ffx_glue_other::h4_f7)
    /// `SfxEventGroup_load_00710c50(this; path)`.
    SfxEventGroup_load_00710c50 @ 0x0071_0c50 => extern "thiscall" fn event_group_load(this: u32, path: u32) -> u32 {
        let mut frame = StackFrame::<{ (f7::GROUP_LOAD_FRAME / 4) as usize }>::new();
        f7::event_group_load(&mut mem(), frame.at(), this, path)
    }
    // ---- continuation round 5, batch 1 (ffx_glue_other::h4_g1)
    /// `TOMakePktBtlDamageNumRGBA(n, x, y, r, g, b, a)`.
    TOMakePktBtlDamageNumRGBA @ 0x008f_b840 => extern "C" fn damage_num(n: u32, x: u32, y: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        g1::damage_num(&mut mem(), n, x, y, [r, g, b, a])
    }
    /// `TOMakePktBtlIconA(kind, x, y, alpha)`.
    TOMakePktBtlIconA @ 0x008f_bcc0 => extern "C" fn btl_icon(kind: u32, x: u32, y: u32, alpha: u32) -> u32 {
        g1::btl_icon(&mut mem(), kind, x, y, alpha)
    }
    /// `TODrawLimitBG()`.
    TODrawLimitBG @ 0x008a_1ce0 => extern "C" fn limit_bg() -> u32 {
        g1::limit_bg(&mut mem())
    }
    /// `TOMkpBtlLimitSpecialGauge(x, y, w)`.
    TOMkpBtlLimitSpecialGauge @ 0x0090_1300 => extern "C" fn limit_special_gauge(x: u32, y: u32, w: u32) -> u32 {
        g1::limit_special_gauge(&mut mem(), x, y, w)
    }
    /// `TOMkpBtlRatioNumber_00901490(ratio, x, y, c)`.
    TOMkpBtlRatioNumber_00901490 @ 0x0090_1490 => extern "C" fn ratio_number(ratio: u32, x: u32, y: u32, c: u32) -> u32 {
        g1::ratio_number(&mut mem(), ratio, x, y, c)
    }
    /// `TOBtlDrawSimpleHelpMes(i)`: the width word the original keeps in
    /// its argument slot is a `StackFrame` word.
    TOBtlDrawSimpleHelpMes @ 0x0089_d9b0 => extern "C" fn simple_help(i: u32) -> u32 {
        let mut width = StackFrame::<1>::new();
        g1::simple_help(&mut mem(), width.at(), i).unwrap_or(0)
    }
    /// `TODrawWaitBtlWinSub(w)`.
    TODrawWaitBtlWinSub @ 0x008a_7520 => extern "C" fn wait_win_sub(w: u32) -> u32 {
        g1::wait_win_sub(&mut mem(), w).unwrap_or(0)
    }
    /// `TOMkpShape2ECCCell_008f8d50(n, x, y, w, h, alpha)`.
    TOMkpShape2ECCCell_008f8d50 @ 0x008f_8d50 => extern "C" fn shape_cell(n: u32, x: u32, y: u32, w: u32, h: u32, alpha: u32) -> u32 {
        let mut frame = StackFrame::<{ (g1::CELL_FRAME / 4) as usize }>::new();
        g1::shape_cell(&mut mem(), frame.at(), [n, x, y, w, h, alpha])
    }
    /// `TOSeqPrimLayoutImage_008f59c0(x, y, w, h, u, v, c0, c1)`.
    TOSeqPrimLayoutImage_008f59c0 @ 0x008f_59c0 => extern "C" fn seq_layout_image(x: u32, y: u32, w: u32, h: u32, u: u32, v: u32, c0: u32, c1: u32) -> u32 {
        let mut frame = StackFrame::<{ (g1::CELL_FRAME / 4) as usize }>::new();
        g1::seq_layout_image(&mut mem(), frame.at(), [x, y, w, h, u, v, c0, c1])
    }
    /// `TOAddPktMenuIconRGBA(ret, x, y, icon, r, g, b, a)`.
    TOAddPktMenuIconRGBA @ 0x008e_b270 => extern "C" fn menu_icon(ret: u32, x: u32, y: u32, icon: u32, r: u32, g: u32, b: u32, a: u32) -> u32 {
        let mut frame = StackFrame::<{ (g1::ICON_FRAME / 4) as usize }>::new();
        g1::menu_icon(&mut mem(), frame.at(), [ret, x, y, icon, r, g, b, a])
    }
    /// `movie_disp2(on, m, v0, v1, e)`.
    movie_disp2 @ 0x0064_2fe0 => extern "C" fn movie_disp2(on: u32, m: u32, v0: u32, v1: u32, e: u32) -> u32 {
        let mut frame = StackFrame::<{ (g1::DISP2_FRAME / 4) as usize }>::new();
        g1::movie_disp2(&mut mem(), frame.at(), [on, m, v0, v1, e])
    }
    // ---- continuation round 5, batch 2 (ffx_glue_other::h4_g2)
    /// `Distortion_fillGridUV_006c2bb0(this)`.
    Distortion_fillGridUV_006c2bb0 @ 0x006c_2bb0 => extern "thiscall" fn grid_uv(this: u32) -> u32 {
        g2::grid_uv(&mut mem(), this)
    }
    /// `Distortion_fillGridPositions_006c2ca0(this)`.
    Distortion_fillGridPositions_006c2ca0 @ 0x006c_2ca0 => extern "thiscall" fn grid_positions(this: u32) -> u32 {
        g2::grid_positions(&mut mem(), this)
    }
    /// `BlurEffectExParams_ctor_006c4020(this)`.
    BlurEffectExParams_ctor_006c4020 @ 0x006c_4020 => extern "thiscall" fn blur_params_ctor(this: u32) -> u32 {
        g2::blur_params_ctor(&mut mem(), this)
    }
    /// `BlurEffectEx_ctor_006c4820(this)`.
    BlurEffectEx_ctor_006c4820 @ 0x006c_4820 => extern "thiscall" fn blur_ex_ctor(this: u32) -> u32 {
        g2::blur_ex_ctor(&mut mem(), this)
    }
    /// `TextUtil_TextUtil(this)`.
    TextUtil_TextUtil @ 0x006f_5b50 => extern "thiscall" fn text_util_ctor(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (g2::IDENTITY_FRAME / 4) as usize }>::new();
        g2::text_util_ctor(&mut mem(), frame.at(), this)
    }
    /// `PostProcessTweakUtil_ctor_00692410(this)`.
    PostProcessTweakUtil_ctor_00692410 @ 0x0069_2410 => extern "thiscall" fn tweak_util_ctor(this: u32) -> u32 {
        g2::tweak_util_ctor(&mut mem(), this)
    }
    /// `IggyMenu_setViewport_00a28f40(this; x, y, w, h)`.
    IggyMenu_setViewport_00a28f40 @ 0x00a2_8f40 => extern "thiscall" fn iggy_set_viewport(this: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        g2::iggy_set_viewport(&mut mem(), this, [x, y, w, h])
    }
    /// `GDraw_SubRectUV_00a3c1b0(out, slot)`.
    GDraw_SubRectUV_00a3c1b0 @ 0x00a3_c1b0 => extern "C" fn gdraw_sub_rect_uv(out: u32, slot: u32) -> u32 {
        g2::gdraw_sub_rect_uv(&mut mem(), out, slot)
    }
    // ---- continuation round 6, batch 1 (ffx_glue_other::h4_h1)
    /// `TextureAnimationInfo_copy_006af0d0(this; src)`.
    TextureAnimationInfo_copy_006af0d0 @ 0x006a_f0d0 => extern "thiscall" fn tex_anim_info_copy(this: u32, src: u32) -> u32 {
        h1::info_copy(&mut mem(), this, src)
    }
    /// `TextureAnimationInfoChar_copy_006af270(this; src)`.
    TextureAnimationInfoChar_copy_006af270 @ 0x006a_f270 => extern "thiscall" fn tex_anim_info_char_copy(this: u32, src: u32) -> u32 {
        h1::info_char_copy(&mut mem(), this, src)
    }
    /// `TextureAnimationInfo_copyArray_006ae9c0(dst, src, n)`.
    TextureAnimationInfo_copyArray_006ae9c0 @ 0x006a_e9c0 => extern "C" fn tex_anim_info_copy_array(dst: u32, src: u32, n: u32) -> u32 {
        h1::info_copy_array(&mut mem(), dst, src, n)
    }
    /// `TextureAnimationInfoChar_copyArray_006aeb20(dst, src, n)`.
    TextureAnimationInfoChar_copyArray_006aeb20 @ 0x006a_eb20 => extern "C" fn tex_anim_info_char_copy_array(dst: u32, src: u32, n: u32) -> u32 {
        h1::info_char_copy_array(&mut mem(), dst, src, n)
    }
    /// `TextureAnimationMaterial_copy_006aeee0(this; src)`.
    TextureAnimationMaterial_copy_006aeee0 @ 0x006a_eee0 => extern "thiscall" fn tex_anim_material_copy(this: u32, src: u32) -> u32 {
        h1::material_copy(&mut mem(), this, src)
    }
    /// `TextureAnimationMaterial_copyArray_006ae730(dst, src, n)`.
    TextureAnimationMaterial_copyArray_006ae730 @ 0x006a_e730 => extern "C" fn tex_anim_material_copy_array(dst: u32, src: u32, n: u32) -> u32 {
        h1::material_copy_array(&mut mem(), dst, src, n)
    }
    /// `SemaphoreQueue_ctor_006db560(this)`.
    SemaphoreQueue_ctor_006db560 @ 0x006d_b560 => extern "thiscall" fn semaphore_queue_ctor(this: u32) -> u32 {
        h1::semaphore_queue_ctor(&mut mem(), this)
    }
    /// `SemaphoreQueues_ctor_006db670(this)`.
    SemaphoreQueues_ctor_006db670 @ 0x006d_b670 => extern "thiscall" fn semaphore_queues_ctor(this: u32) -> u32 {
        h1::semaphore_queues_ctor(&mut mem(), this)
    }
    /// `PostProcessState_init_006ddbf0(this)`.
    PostProcessState_init_006ddbf0 @ 0x006d_dbf0 => extern "thiscall" fn post_process_state_init(this: u32) -> u32 {
        h1::post_process_state_init(&mut mem(), this)
    }
    /// `DynGeoMemPool_ctor_006de2c0(this)`.
    DynGeoMemPool_ctor_006de2c0 @ 0x006d_e2c0 => extern "thiscall" fn dyn_geo_pool_ctor(this: u32) -> u32 {
        h1::dyn_geo_pool_ctor(&mut mem(), this)
    }
    /// `ShadowManager_ctor_006c7d80(this)`.
    ShadowManager_ctor_006c7d80 @ 0x006c_7d80 => extern "thiscall" fn shadow_manager_ctor(this: u32) -> u32 {
        h1::shadow_manager_ctor(&mut mem(), this)
    }
    /// `ShaderSourceReader_ctor_006904c0(this)`.
    ShaderSourceReader_ctor_006904c0 @ 0x0069_04c0 => extern "thiscall" fn shader_source_reader_ctor(this: u32) -> u32 {
        h1::shader_source_reader_ctor(&mut mem(), this)
    }
    /// `FFEscMenu_create_0068e400(a, f0, f1, f2, f3)`.
    FFEscMenu_create_0068e400 @ 0x0068_e400 => extern "C" fn esc_menu_create(a: u32, f0: u32, f1: u32, f2: u32, f3: u32) -> u32 {
        h1::esc_menu_create(&mut mem(), a, [f0, f1, f2, f3]).unwrap_or(0)
    }
    // ---- continuation round 6, batch 2 (ffx_glue_other::h4_h2)
    /// `LoadingScreen_LoadingScreen(this)`.
    LoadingScreen_LoadingScreen @ 0x006e_45e0 => extern "thiscall" fn loading_screen_ctor(this: u32) -> u32 {
        h2::loading_screen_ctor(&mut mem(), this)
    }
    /// `BrokenScreenEffect_createRenderTarget(this)`.
    BrokenScreenEffect_createRenderTarget @ 0x006d_5050 => extern "thiscall" fn broken_screen_create_render_target(this: u32) -> u32 {
        h2::broken_screen_create_render_target(&mut mem(), this)
    }
    /// `BootText_create_006ddc80(this)`.
    BootText_create_006ddc80 @ 0x006d_dc80 => extern "thiscall" fn boot_text_create(this: u32) -> u32 {
        h2::boot_text_create(&mut mem(), this)
    }
    /// `Distortion_initialize_006c2710(this; a, b, c, d, e)`.
    Distortion_initialize_006c2710 @ 0x006c_2710 => extern "thiscall" fn distortion_initialize(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        let mut frame = StackFrame::<{ (h2::DISTORTION_FRAME / 4) as usize }>::new();
        h2::distortion_initialize(&mut mem(), frame.at(), this, [a, b, c, d, e])
    }
    /// `Distortion_onShadersLoaded_006c2520(this)`.
    Distortion_onShadersLoaded_006c2520 @ 0x006c_2520 => extern "thiscall" fn distortion_on_shaders(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (h2::CURSOR_FRAME / 4) as usize }>::new();
        h2::distortion_on_shaders(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 6, batch 3 (ffx_glue_other::h4_h3)
    /// `ClassLoadedCharacterTable_getPrototypeByFilename(this; name)`.
    ClassLoadedCharacterTable_getPrototypeByFilename @ 0x006a_2110 => extern "thiscall" fn get_prototype(this: u32, name: u32) -> u32 {
        let mut frame = StackFrame::<{ (h3::GET_FRAME / 4) as usize }>::new();
        h3::get_prototype(&mut mem(), frame.at(), this, name)
    }
    /// `ClassLoadedCharacterTable_insertPrototype(this; name, prototype)`.
    ClassLoadedCharacterTable_insertPrototype @ 0x006a_28b0 => extern "thiscall" fn insert_prototype(this: u32, name: u32, prototype: u32) -> u32 {
        let mut frame = StackFrame::<{ (h3::INSERT_FRAME / 4) as usize }>::new();
        h3::insert_prototype(&mut mem(), frame.at(), this, name, prototype)
    }
    /// `IggyCallback_lookupName_006e9590(a, args)`.
    IggyCallback_lookupName_006e9590 @ 0x006e_9590 => extern "C" fn iggy_lookup(a: u32, args: u32) -> u32 {
        let mut frame = StackFrame::<{ (h3::LOOKUP_FRAME / 4) as usize }>::new();
        h3::iggy_lookup(&mut mem(), frame.at(), a, args)
    }
    /// `ChrResourceCacheManager_onLoadPCluster(this; path, hash, record)`.
    ChrResourceCacheManager_onLoadPCluster @ 0x0069_bb80 => extern "thiscall" fn on_load_pcluster(this: u32, path: u32, hash: u32, record: u32) -> u32 {
        let mut frame = StackFrame::<{ (h3::ON_LOAD_FRAME / 4) as usize }>::new();
        h3::on_load_pcluster(&mut mem(), frame.at(), this, path, hash, record)
    }
    /// `ClusterManager_loadPCluster(this; name)`.
    ClusterManager_loadPCluster @ 0x0069_b940 => extern "thiscall" fn load_pcluster(this: u32, name: u32) -> u32 {
        let mut frame = StackFrame::<{ (h3::LOAD_FRAME / 4) as usize }>::new();
        h3::load_pcluster(&mut mem(), frame.at(), this, name)
    }
    // ---- continuation round 6, batch 4 (ffx_glue_other::h4_h4)
    /// `BlurPass_initialize_006c40e0(this; a, cluster, scene, w, h)`.
    BlurPass_initialize_006c40e0 @ 0x006c_40e0 => extern "thiscall" fn blur_pass_init(this: u32, a: u32, cluster: u32, scene: u32, w: u32, h: u32) -> u32 {
        let mut frame = StackFrame::<{ (h4::PASS_FRAME / 4) as usize }>::new();
        h4::blur_pass_init(&mut mem(), frame.at(), this, [a, cluster, scene, w, h])
    }
    /// `BlurEffectEx_postInitialize(this)`.
    BlurEffectEx_postInitialize @ 0x006c_4b60 => extern "thiscall" fn blur_ex_post_initialize(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (h4::EFFECT_FRAME / 4) as usize }>::new();
        h4::blur_ex_post_initialize(&mut mem(), frame.at(), this)
    }
    /// `BlurEffect_doOnShaderClusterLoaded(this)`.
    BlurEffect_doOnShaderClusterLoaded @ 0x006c_35d0 => extern "thiscall" fn blur_effect_on_shaders(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (h4::EFFECT_FRAME / 4) as usize }>::new();
        h4::blur_effect_on_shaders(&mut mem(), frame.at(), this)
    }
    /// `RadialBlur_initialize(this; cluster, a, scene, w, h)`.
    RadialBlur_initialize @ 0x0070_3230 => extern "thiscall" fn radial_blur_init(this: u32, cluster: u32, a: u32, scene: u32, w: u32, h: u32) -> u32 {
        let mut frame = StackFrame::<{ (h4::PASS_FRAME / 4) as usize }>::new();
        h4::radial_blur_init(&mut mem(), frame.at(), this, [cluster, a, scene, w, h])
    }
    /// `RadialLine_initialize(this; cluster, a, scene, w, h, target)`.
    RadialLine_initialize @ 0x006f_7c10 => extern "thiscall" fn radial_line_init(this: u32, cluster: u32, a: u32, scene: u32, w: u32, h: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<{ (h4::PASS_FRAME / 4) as usize }>::new();
        h4::radial_line_init(&mut mem(), frame.at(), this, [cluster, a, scene, w, h, target])
    }
    // ---- continuation round 6, batch 5 (ffx_glue_other::h4_h5)
    /// `MotionBlur_setupShaders_006c5150(this; base, target)`.
    MotionBlur_setupShaders_006c5150 @ 0x006c_5150 => extern "thiscall" fn motion_blur_setup(this: u32, base: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<{ (h5::MOTION_FRAME / 4) as usize }>::new();
        h5::motion_blur_setup(&mut mem(), frame.at(), this, base, target)
    }
    /// `PhyFMVPlayerManager_doOnShaderBinFilesLoaded(this)`.
    PhyFMVPlayerManager_doOnShaderBinFilesLoaded @ 0x006d_86e0 => extern "thiscall" fn fmv_on_shaders(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (h5::FMV_FRAME / 4) as usize }>::new();
        h5::fmv_on_shaders(&mut mem(), frame.at(), this)
    }
    /// `BrokenScreenEffect_initialize(this; a, b, c, d, e, w, h)`.
    BrokenScreenEffect_initialize @ 0x006d_51d0 => extern "thiscall" fn broken_screen_initialize(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32, w: u32, h: u32) -> u32 {
        let mut frame = StackFrame::<{ (h5::BROKEN_FRAME / 4) as usize }>::new();
        h5::broken_screen_initialize(&mut mem(), frame.at(), this, [a, b, c, d, e, w, h])
    }
    // ---- continuation round 6, batch 6 (ffx_glue_other::h4_h6)
    /// `PhyFMVPlayerManager_PlayVideo(this; flag)`.
    PhyFMVPlayerManager_PlayVideo @ 0x006d_7430 => extern "thiscall" fn play_video(this: u32, flag: u32) -> u32 {
        let mut frame = StackFrame::<{ (h6::PLAY_FRAME / 4) as usize }>::new();
        h6::play_video(&mut mem(), frame.at(), this, flag)
    }
    /// `PhyFMVPlayerManager_DestroyFMVPlayer(this)`.
    PhyFMVPlayerManager_DestroyFMVPlayer @ 0x006d_71b0 => extern "thiscall" fn destroy_fmv_player(this: u32) -> u32 {
        h6::destroy_fmv_player(&mut mem(), this)
    }
    /// `PhyFMVPlayerManager_handleInput(this; unused)`.
    PhyFMVPlayerManager_handleInput @ 0x006d_9460 => extern "thiscall" fn handle_input(this: u32, _unused: u32) {
        h6::handle_input(&mut mem(), this);
    }
    // ---- continuation round 6, batch 7 (ffx_glue_other::h4_h7)
    /// `PVideoPlaybackWin32_ctor(this)`.
    PVideoPlaybackWin32_ctor @ 0x00a2_6e70 => extern "thiscall" fn video_playback_ctor(this: u32) -> u32 {
        h7::video_playback_ctor(&mut mem(), this)
    }
    /// `PhyFMVPlayerManager_PhyFMVPlayerManager(this)`.
    PhyFMVPlayerManager_PhyFMVPlayerManager @ 0x006d_6970 => extern "thiscall" fn fmv_manager_ctor(this: u32) -> u32 {
        h7::fmv_manager_ctor(&mut mem(), this)
    }
    /// `gdraw_allocBuffers_00a3a540(a, unused, n0, n1, out, c)`.
    gdraw_allocBuffers_00a3a540 @ 0x00a3_a540 => extern "stdcall" fn gdraw_alloc_buffers(a: u32, _unused: u32, n0: u32, n1: u32, out: u32, c: u32) -> u32 {
        h7::gdraw_alloc_buffers(&mut mem(), a, n0, n1, out, c)
    }
    // ---- continuation round 6, batch 8 (ffx_glue_other::h4_h8)
    /// `SkipPrompt_render_006e7f00(this; renderer, mode)`.
    SkipPrompt_render_006e7f00 @ 0x006e_7f00 => extern "thiscall" fn skip_prompt_render(this: u32, renderer: u32, mode: u32) {
        let mut frame = StackFrame::<{ (h8::PROMPT_FRAME / 4) as usize }>::new();
        h8::skip_prompt_render(&mut mem(), frame.at(), this, renderer, mode);
    }
    /// `LoadingScreen_render(this; renderer)`.
    LoadingScreen_render @ 0x006e_4ea0 => extern "thiscall" fn loading_screen_render(this: u32, renderer: u32) {
        let mut frame = StackFrame::<{ (h8::LOADING_FRAME / 4) as usize }>::new();
        h8::loading_screen_render(&mut mem(), frame.at(), this, renderer);
    }
    // ---- continuation round 7, batch 1 (ffx_glue_other::h4_i1)
    /// `PVideoPlaybackWin32_close(this)`: `al = 1` over the last callee's
    /// `eax`, as the original leaves it.
    PVideoPlaybackWin32_close @ 0x00a2_8350 => extern "thiscall" fn video_close(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (i1::CLOSE_FRAME / 4) as usize }>::new();
        i1::video_close(&mut mem(), frame.at(), this)
    }
    /// `Sg_SaveVramResource()`.
    Sg_SaveVramResource @ 0x0083_fd60 => extern "C" fn save_vram_resource() -> u32 {
        // laid out and aligned as the original's frame (`ebp - 0x60` on 16)
        #[repr(C, align(16))]
        struct Work([u8; i1::SAVE_VRAM_FRAME as usize]);
        let mut work = Work([0; i1::SAVE_VRAM_FRAME as usize]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let r = i1::save_vram_resource(&mut mem(), w);
        std::hint::black_box(&mut work);
        r
    }
    /// `ReadDebug_loadFile_00a42ab0(path, size_out)`.
    ReadDebug_loadFile_00a42ab0 @ 0x00a4_2ab0 => extern "C" fn read_debug_file(path: u32, size_out: u32) -> u32 {
        i1::read_debug_file(&mut mem(), path, size_out)
    }
    /// `PApplication_InitWindowAndApp_00629ab0(this; a, b)`.
    PApplication_InitWindowAndApp_00629ab0 @ 0x0062_9ab0 => extern "thiscall" fn init_window_and_app(this: u32, a: u32, b: u32) -> u32 {
        i1::init_window_and_app(&mut mem(), this, a, b)
    }
    /// `createVFXRenderDataBufferLine(a, b, n, arg)`.
    createVFXRenderDataBufferLine @ 0x0071_4b60 => extern "C" fn create_line_buffer(a: u32, b: u32, n: u32, arg: u32) -> u32 {
        let mut frame = StackFrame::<{ (i1::LINE_FRAME / 4) as usize }>::new();
        i1::create_line_buffer(&mut mem(), frame.at(), a, b, n, arg)
    }
    /// `pppMdl_copyPositions_007163e0(part, job)`.
    pppMdl_copyPositions_007163e0 @ 0x0071_63e0 => extern "C" fn copy_positions(part: u32, job: u32) -> u32 {
        let mut matrix = StackFrame::<16>::new();
        i1::copy_positions(&mut mem(), matrix.at(), part, job)
    }
    /// `opd_config_memory(pool, size, arg)`.
    opd_config_memory @ 0x00a3_b210 => extern "C" fn config_memory(pool: u32, size: u32, arg: u32) -> u32 {
        i1::config_memory(&mut mem(), pool, size, arg)
    }
    /// `GDraw_pool_freeAll_00a3b380(pool, stats)`.
    GDraw_pool_freeAll_00a3b380 @ 0x00a3_b380 => extern "C" fn free_all(pool: u32, stats: u32) {
        i1::free_all(&mut mem(), pool, stats);
    }
    // ---- continuation round 7, batch 2 (ffx_glue_other::h4_i2)
    /// `PhyFMVPlayerManager_fmvPlayer_play(path, b, wait)`.
    PhyFMVPlayerManager_fmvPlayer_play @ 0x006d_8fc0 => extern "stdcall" fn fmv_play(path: u32, b: u32, wait: u32) {
        let mut frame = StackFrame::<{ (i2::PLAY_FRAME / 4) as usize }>::new();
        i2::fmv_play(&mut mem(), frame.at(), path, b, wait);
    }
    /// `PhyFMVPlayerManager_Render(this; renderer)`.
    PhyFMVPlayerManager_Render @ 0x006d_7570 => extern "thiscall" fn fmv_render(this: u32, renderer: u32) -> u32 {
        i2::fmv_render(&mut mem(), this, renderer)
    }
    /// `Sg_DrawSetEnvironment()`.
    Sg_DrawSetEnvironment @ 0x0082_c7b0 => extern "C" fn draw_set_environment() -> u32 {
        let mut frame = StackFrame::<{ (i2::ENV_FRAME / 4) as usize }>::new();
        i2::draw_set_environment(&mut mem(), frame.at())
    }
    /// `op_gs_loadImage_007e4420(image, data)`.
    op_gs_loadImage_007e4420 @ 0x007e_4420 => extern "C" fn load_image_split(image: u32, data: u32) {
        i2::load_image_split(&mut mem(), image, data);
    }
    /// `IggyMenu_ctor_00a288f0(this; path, x, y, w, h, flags)`.
    IggyMenu_ctor_00a288f0 @ 0x00a2_88f0 => extern "thiscall" fn iggy_menu_ctor(this: u32, path: u32, x: u32, y: u32, w: u32, h: u32, flags: u32) -> u32 {
        i2::iggy_menu_ctor(&mut mem(), this, path, [x, y, w, h], flags)
    }
    /// `SemaphoreQueue_start_006dba20(this)`.
    SemaphoreQueue_start_006dba20 @ 0x006d_ba20 => extern "thiscall" fn semaphore_queue_start(this: u32) {
        i2::semaphore_queue_start(&mut mem(), this);
    }
    /// `FFEscMenu_onDialog_0068e5b0(menu, args)`.
    FFEscMenu_onDialog_0068e5b0 @ 0x0068_e5b0 => extern "C" fn esc_dialog(menu: u32, args: u32) -> u32 {
        let mut frame = StackFrame::<{ (i2::DIALOG_FRAME / 4) as usize }>::new();
        i2::esc_dialog(&mut mem(), frame.at(), menu, args)
    }
    // ---- continuation round 7, batch 3 (ffx_glue_other::h4_i3)
    /// `BlurEffect_ctor_006c32d0(this)`.
    BlurEffect_ctor_006c32d0 @ 0x006c_32d0 => extern "thiscall" fn blur_effect_ctor(this: u32) -> u32 {
        i3::blur_effect_ctor(&mut mem(), this)
    }
    /// `MotionBlur_renderMotionBlur(this; renderer, target)`.
    MotionBlur_renderMotionBlur @ 0x006c_5410 => extern "thiscall" fn motion_blur_render(this: u32, renderer: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<{ (i3::MOTION_FRAME / 4) as usize }>::new();
        i3::motion_blur_render(&mut mem(), frame.at(), this, renderer, target)
    }
    /// `SaveLoadMenu_open_00702a80()`.
    SaveLoadMenu_open_00702a80 @ 0x0070_2a80 => extern "C" fn save_load_open() {
        let mut frame = StackFrame::<{ (i3::SAVE_LOAD_FRAME / 4) as usize }>::new();
        i3::save_load_open(&mut mem(), frame.at());
    }
    /// `BootText_update_006de010(this; dt)`.
    BootText_update_006de010 @ 0x006d_e010 => extern "thiscall" fn boot_text_update(this: u32, dt: u32) {
        i3::boot_text_update(&mut mem(), this, dt);
    }
    /// `gdraw_D3D11_SetResourceLimits_00a38180(kind, handles, bytes)`.
    gdraw_D3D11_SetResourceLimits_00a38180 @ 0x00a3_8180 => extern "C" fn set_resource_limits(kind: u32, handles: u32, bytes: u32) -> u32 {
        let mut stats = StackFrame::<{ (i3::STATS_FRAME / 4) as usize }>::new();
        i3::set_resource_limits(&mut mem(), stats.at(), kind, handles, bytes)
    }
    /// `IggyGDraw_setup_00a3baf0(this; device, limits)`.
    IggyGDraw_setup_00a3baf0 @ 0x00a3_baf0 => extern "thiscall" fn gdraw_setup(this: u32, device: u32, limits: u32) -> u32 {
        let mut stats = StackFrame::<{ (i3::STATS_FRAME / 4) as usize }>::new();
        i3::gdraw_setup(&mut mem(), stats.at(), this, device, limits)
    }
    // ---- continuation round 7, batch 4 (ffx_glue_other::h4_i4)
    /// `Sg_DrawSetEnvironmentInit()`.
    Sg_DrawSetEnvironmentInit @ 0x0082_c8d0 => extern "C" fn draw_set_environment_init() -> u32 {
        let mut frame = StackFrame::<{ (i4::ENV_INIT_FRAME / 4) as usize }>::new();
        i4::draw_set_environment_init(&mut mem(), frame.at())
    }
    /// `op_gs1_transfer()`.
    op_gs1_transfer @ 0x007e_b580 => extern "C" fn gs1_transfer() -> u32 {
        // laid out and aligned as the original's frame (`ebp - 0xc0` on 16)
        #[repr(C, align(16))]
        struct Work([u8; i4::GS1_FRAME as usize]);
        let mut work = Work([0; i4::GS1_FRAME as usize]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let r = i4::gs1_transfer(&mut mem(), w);
        std::hint::black_box(&mut work);
        r
    }
    /// `op_oef2_texture_load(texture, flush)`.
    op_oef2_texture_load @ 0x007e_5030 => extern "C" fn oef2_texture_load(texture: u32, flush: u32) {
        // laid out and aligned as the original's frame (`ebp - 0x60` on 16)
        #[repr(C, align(16))]
        struct Work([u8; i4::LOAD_FRAME as usize]);
        let mut work = Work([0; i4::LOAD_FRAME as usize]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        i4::oef2_texture_load(&mut mem(), w, texture, flush);
        std::hint::black_box(&mut work);
    }
    /// `SaMenu_loadSbinTextures_00775ed0(sbin)`.
    SaMenu_loadSbinTextures_00775ed0 @ 0x0077_5ed0 => extern "C" fn sa_load_sbin_textures(sbin: u32) -> u32 {
        // laid out and aligned as the original's frame (`ebp - 0x60` on 16)
        #[repr(C, align(16))]
        struct Work([u8; i4::LOAD_FRAME as usize]);
        let mut work = Work([0; i4::LOAD_FRAME as usize]);
        let w = u32::try_from((&raw mut work).expose_provenance()).unwrap_or(0);
        let r = i4::sa_load_sbin_textures(&mut mem(), w, sbin);
        std::hint::black_box(&mut work);
        r
    }
    // ---- continuation round 7, batch 5 (ffx_glue_other::h4_i5)
    /// `pppMdl_projectPositions_007161f0(part, job)`.
    pppMdl_projectPositions_007161f0 @ 0x0071_61f0 => extern "C" fn project_positions(part: u32, job: u32) -> u32 {
        let mut frame = StackFrame::<{ (i5::PROJECT_FRAME / 4) as usize }>::new();
        i5::project_positions(&mut mem(), frame.at(), part, job)
    }
    /// `gdraw_getDepthStencilState_00a3d020(depth, write, read_mask, write_mask)`.
    gdraw_getDepthStencilState_00a3d020 @ 0x00a3_d020 => extern "C" fn depth_stencil_state(depth: u32, write: u32, read_mask: u32, write_mask: u32) -> u32 {
        let mut desc = StackFrame::<{ (i5::DEPTH_FRAME / 4) as usize }>::new();
        i5::depth_stencil_state(&mut mem(), desc.at(), depth, write, read_mask, write_mask)
    }
    /// `Distortion_setupDistortionGrid(this)`.
    Distortion_setupDistortionGrid @ 0x006c_2ff0 => extern "thiscall" fn setup_distortion_grid(this: u32) {
        i5::setup_distortion_grid(&mut mem(), this);
    }
    /// `gdraw_createDepthBuffer_00a3b840(stats)`.
    gdraw_createDepthBuffer_00a3b840 @ 0x00a3_b840 => extern "C" fn depth_buffer(stats: u32) -> u32 {
        let mut desc = StackFrame::<{ (i5::TEXTURE_FRAME / 4) as usize }>::new();
        i5::depth_buffer(&mut mem(), desc.at(), stats)
    }
    // ---- continuation round 7, batch 6 (ffx_glue_other::h4_i6)
    /// `BrokenScreenEffect_captureFrame(this)`.
    BrokenScreenEffect_captureFrame @ 0x006d_4e20 => extern "thiscall" fn capture_frame(this: u32) {
        let mut frame = StackFrame::<{ (i6::CAPTURE_FRAME / 4) as usize }>::new();
        i6::capture_frame(&mut mem(), frame.at(), this);
    }
    /// `PhyFMVPlayerManager_UpdateTexture(this)`.
    PhyFMVPlayerManager_UpdateTexture @ 0x006d_7680 => extern "thiscall" fn fmv_update_texture(this: u32) -> u32 {
        i6::fmv_update_texture(&mut mem(), this)
    }
    // ---- continuation round 7, batch 7 (ffx_glue_other::h4_i7)
    /// `ClusterManager_createPreallocateDataList(name)`.
    ClusterManager_createPreallocateDataList @ 0x0063_d260 => extern "C" fn create_preallocate_data_list(name: u32) -> u32 {
        let mut frame = StackFrame::<{ (i7::PREALLOCATE_FRAME / 4) as usize }>::new();
        i7::create_preallocate_data_list(&mut mem(), frame.at(), name)
    }
    // ---- continuation round 7, batch 8 (ffx_glue_other::h4_i8)
    /// `Distortion_renderDistortion(this)`.
    Distortion_renderDistortion @ 0x006c_2dc0 => extern "thiscall" fn render_distortion(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (i8::DISTORTION_FRAME / 4) as usize }>::new();
        i8::render_distortion(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 8, batch 1 (ffx_glue_other::h4_j1)
    /// `op_gs1_init(work, flags)`.
    op_gs1_init @ 0x007e_b760 => extern "C" fn op_gs1_init(work: u32, flags: u32) -> u32 {
        j1::op_gs1_init(&mut mem(), work, flags)
    }
    /// `FFEscMenu_initControllerKeyNames_00702250()`.
    FFEscMenu_initControllerKeyNames_00702250 @ 0x0070_2250 => extern "C" fn init_controller_key_names() -> u32 {
        j1::fill_controller_names(&mut mem())
    }
    /// `FFEscMenu_initKeyboardKeyNames_006ffc80()`.
    FFEscMenu_initKeyboardKeyNames_006ffc80 @ 0x006f_fc80 => extern "C" fn init_keyboard_key_names() -> u32 {
        j1::fill_keyboard_names(&mut mem())
    }
    /// `RemapButtonIcon_remapUIButtonData(this; key, rec, path, kind)`.
    RemapButtonIcon_remapUIButtonData @ 0x006e_7300 => extern "thiscall" fn remap_ui_button_data(this: u32, key: u32, rec: u32, path: u32, kind: u32) -> u32 {
        let mut frame = StackFrame::<{ (j1::REMAP_FRAME / 4) as usize }>::new();
        j1::remap_ui_button_data(&mut mem(), frame.at(), this, key, rec, path, kind)
    }
    /// `ChrMeshInstances_buildSuffixes_006a2390(this)`.
    ChrMeshInstances_buildSuffixes_006a2390 @ 0x006a_2390 => extern "thiscall" fn build_instance_suffixes(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j1::SUFFIX_FRAME / 4) as usize }>::new();
        j1::build_instance_suffixes(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 8, batch 2 (ffx_glue_other::h4_j2)
    /// `FFEscMenu_initializeControllerSettingPage(this)`.
    FFEscMenu_initializeControllerSettingPage @ 0x0070_1fa0 => extern "thiscall" fn controller_setting_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j2::CONTROLLER_FRAME / 4) as usize }>::new();
        j2::controller_page(&mut mem(), frame.at(), this)
    }
    /// `FFEscMenu_initializeKeyboardSettingPage(this)`.
    FFEscMenu_initializeKeyboardSettingPage @ 0x0070_0da0 => extern "thiscall" fn keyboard_setting_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j2::KEYBOARD_FRAME / 4) as usize }>::new();
        j2::keyboard_page(&mut mem(), frame.at(), this)
    }
    /// `FFEscMenu_initializeAudioSettingPage(this)`.
    FFEscMenu_initializeAudioSettingPage @ 0x006f_f770 => extern "thiscall" fn audio_setting_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j2::AUDIO_FRAME / 4) as usize }>::new();
        j2::audio_page(&mut mem(), frame.at(), this)
    }
    /// `FFEscMenu_initializeParameterChangePage(this)`.
    FFEscMenu_initializeParameterChangePage @ 0x006f_c3f0 => extern "thiscall" fn parameter_change_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j2::PARAMETER_FRAME / 4) as usize }>::new();
        j2::parameter_page(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 8, batch 3 (ffx_glue_other::h4_j3)
    /// `GraphicAbmap_loadTextureClusters_00684ef0()`.
    GraphicAbmap_loadTextureClusters_00684ef0 @ 0x0068_4ef0 => extern "C" fn load_texture_clusters() -> u32 {
        let mut frame = StackFrame::<{ (j3::LOAD_FRAME / 4) as usize }>::new();
        j3::load_texture_clusters(&mut mem(), frame.at())
    }
    /// `RemapButtonIcon_remapMessgeInputData(this; key, rec, path, kind)`.
    RemapButtonIcon_remapMessgeInputData @ 0x006e_7050 => extern "thiscall" fn remap_message_input_data(this: u32, key: u32, rec: u32, path: u32, kind: u32) -> u32 {
        let mut frame = StackFrame::<{ (j3::MESSAGE_FRAME / 4) as usize }>::new();
        j3::remap_message_input_data(&mut mem(), frame.at(), this, key, rec, path, kind)
    }
    /// `FFEscMenu_updateSpecialFeaturePage(this)`.
    FFEscMenu_updateSpecialFeaturePage @ 0x0070_2660 => extern "thiscall" fn special_feature_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j3::FEATURE_FRAME / 4) as usize }>::new();
        j3::special_feature_page(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 8, batch 4 (ffx_glue_other::h4_j4)
    /// `FFEscMenu_initializeMainSettingPage(this)`.
    FFEscMenu_initializeMainSettingPage @ 0x0068_e6f0 => extern "thiscall" fn main_setting_page(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j4::MAIN_FRAME / 4) as usize }>::new();
        j4::main_setting_page(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 8, batch 5 (ffx_glue_other::h4_j5)
    /// `TextUtil_doOnAllTextUtilAssetsLoaded(this)`.
    TextUtil_doOnAllTextUtilAssetsLoaded @ 0x006f_60a0 => extern "thiscall" fn text_util_assets_loaded(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (j5::ASSETS_FRAME / 4) as usize }>::new();
        j5::on_all_assets_loaded(&mut mem(), frame.at(), this)
    }
    /// `ClassChrShaderRender_VertexInstanceSetup(this)`.
    ClassChrShaderRender_VertexInstanceSetup @ 0x006a_0100 => extern "thiscall" fn vertex_instance_setup(this: u32) -> u32 {
        j5::vertex_instance_setup(&mut mem(), this)
    }
    // ---- continuation round 8, batch 6 (ffx_glue_other::h4_j6)
    /// `GuildMapDynamicGeometry_updateRenderData(this; unused)` (a vtable
    /// entry; the word is popped, never read).
    GuildMapDynamicGeometry_updateRenderData @ 0x006a_5070 => extern "thiscall" fn guild_map_update_render_data(this: u32, _unused: u32) -> u32 {
        let mut frame = StackFrame::<{ (j6::GUILD_FRAME / 4) as usize }>::new();
        j6::update_render_data(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 9, batch 1 (ffx_glue_other::h4_k1)
    /// `updateVFXRenderDataLine(a, b, lines)`.
    updateVFXRenderDataLine @ 0x0071_a0c0 => extern "C" fn update_vfx_render_data_line(a: u32, b: u32, lines: u32) -> u32 {
        k1::update_line(&mut mem(), a, b, lines)
    }
    /// `BlurEffectEx_blur_006c4290(this; texture, target)`.
    BlurEffectEx_blur_006c4290 @ 0x006c_4290 => extern "thiscall" fn blur_effect_ex_blur(this: u32, texture: u32, target: u32) -> u32 {
        let mut frame = StackFrame::<{ (k1::BLUR_FRAME / 4) as usize }>::new();
        k1::blur(&mut mem(), frame.at(), this, texture, target)
    }
    /// `BlurEffect_render_006c3970(this)`.
    BlurEffect_render_006c3970 @ 0x006c_3970 => extern "thiscall" fn blur_effect_render(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (k1::RENDER_FRAME / 4) as usize }>::new();
        k1::render(&mut mem(), frame.at(), this)
    }
    /// `BlurEffect_update_006c3d40(this; dt)` (`dt` is never read).
    BlurEffect_update_006c3d40 @ 0x006c_3d40 => extern "thiscall" fn blur_effect_update(this: u32, _dt: u32) -> u32 {
        k1::update(&mut mem(), this, &mut |_, a, b| ci_fmod(a, b))
    }
    // ---- continuation round 9, batch 2 (ffx_glue_other::h4_k2)
    /// `TextUtil_appendLine(this; text)`.
    TextUtil_appendLine @ 0x006f_5e30 => extern "thiscall" fn text_util_append_line(this: u32, text: u32) -> u32 {
        let mut frame = StackFrame::<{ (k2::APPEND_FRAME / 4) as usize }>::new();
        k2::append_line(&mut mem(), frame.at(), this, text)
    }
    /// `RemapButtonIcon_remapIconPosition(this; kind, quad, button)`.
    RemapButtonIcon_remapIconPosition @ 0x006e_6a50 => extern "thiscall" fn remap_icon_position(this: u32, kind: u32, quad: u32, button: u32) -> u32 {
        k2::remap_icon_position(&mut mem(), this, kind, quad, button)
    }
    /// `LoadingScreen_update_006e50a0(this; dt)` (callers read no result).
    LoadingScreen_update_006e50a0 @ 0x006e_50a0 => extern "thiscall" fn loading_screen_update(this: u32, dt: u32) {
        let mut frame = StackFrame::<{ (k2::LOADING_FRAME / 4) as usize }>::new();
        k2::loading_update(&mut mem(), frame.at(), this, dt);
    }
    // ---- continuation round 9, batch 3 (ffx_glue_other::h4_k3)
    /// `DepthBiasHooks_onCharacterInstantiated(name, count, items)` (the
    /// array by value; the caller reads no result).
    DepthBiasHooks_onCharacterInstantiated @ 0x006e_4100 => extern "C" fn depth_bias_on_character_instantiated(name: u32, count: u32, items: u32) {
        k3::on_character_instantiated(&mut mem(), name, count, items);
    }
    /// `PostProcessManager_ctor_00692020(this)` -> `this`.
    PostProcessManager_ctor_00692020 @ 0x0069_2020 => extern "thiscall" fn post_process_manager_ctor(this: u32) -> u32 {
        k3::post_process_manager_ctor(&mut mem(), this)
    }
    /// `BrokenScreenEffect_render(this)` -> 0.
    BrokenScreenEffect_render @ 0x006d_5680 => extern "thiscall" fn broken_screen_effect_render(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (k3::BROKEN_FRAME / 4) as usize }>::new();
        k3::broken_render(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 9, batch 4 (ffx_glue_other::h4_k4)
    /// `createVFXRenderDataBufferAtelShp(lo, hi, shapes, index)` (`index`
    /// a u16 in its word).
    createVFXRenderDataBufferAtelShp @ 0x0071_47b0 => extern "C" fn create_vfx_render_data_buffer_atel_shp(lo: u32, hi: u32, shapes: u32, index: u32) -> u32 {
        let mut frame = StackFrame::<{ (k4::ATEL_FRAME / 4) as usize }>::new();
        k4::create_atel_shp(&mut mem(), frame.at(), lo, hi, shapes, index)
    }
    // ---- continuation round 9, batch 5 (ffx_glue_other::h4_k4)
    /// `createVFXRenderDataBufferShapeField(lo, hi, shapes, scale, flags)`
    /// (a 0x6f1c-byte frame, as the original's).
    createVFXRenderDataBufferShapeField @ 0x0071_4d40 => extern "C" fn create_vfx_render_data_buffer_shape_field(lo: u32, hi: u32, shapes: u32, scale: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (k4::SHAPE_FRAME / 4) as usize }>::new();
        k4::create_shape_field(&mut mem(), frame.at(), lo, hi, shapes, scale, flags)
    }
    // ---- continuation round 10, batch 1 (ffx_glue_other::h4_l1)
    /// `createVFXRenderDataBufferShp(lo, hi, shapes, unused, flags)` (a
    /// 0x68dc-byte frame, as the original's; the fourth word is never
    /// read).
    createVFXRenderDataBufferShp @ 0x0071_5170 => extern "C" fn create_vfx_render_data_buffer_shp(lo: u32, hi: u32, shapes: u32, _unused: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (l1::SHP_FRAME / 4) as usize }>::new();
        l1::create_shp(&mut mem(), frame.at(), lo, hi, shapes, flags)
    }
    /// `MenuManager_GetMenuVertexInfo(this; name, kind)` -> the record.
    MenuManager_GetMenuVertexInfo @ 0x0067_ba50 => extern "thiscall" fn menu_manager_get_menu_vertex_info(this: u32, name: u32, kind: u32) -> u32 {
        l1::get_menu_vertex_info(&mut mem(), this, name, kind)
    }
    /// `ChrResourceCacheManager_applyDelayedRelease(this)` (`ecx`, plain
    /// ret) -> `this`.
    ChrResourceCacheManager_applyDelayedRelease @ 0x0069_a6b0 => extern "thiscall" fn chr_resource_cache_manager_apply_delayed_release(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (l1::RELEASE_FRAME / 4) as usize }>::new();
        l1::apply_delayed_release(&mut mem(), frame.at(), this)
    }
    /// `FmodVoice_dataChange(this; id, system)` -> 1, 0 (nothing to do) or
    /// -1 (a project missing).
    FmodVoice_dataChange @ 0x0070_a630 => extern "thiscall" fn fmod_voice_data_change(this: u32, id: u32, system: u32) -> u32 {
        let mut frame = StackFrame::<{ (l1::VOICE_FRAME / 4) as usize }>::new();
        l1::data_change(&mut mem(), frame.at(), this, id, system)
    }
    // ---- continuation round 10, batch 2 (ffx_glue_other::h4_l2)
    /// `updateGuildMapRenderData(lo, hi, map, tint)` -> 1 or 0.
    updateGuildMapRenderData @ 0x0071_7620 => extern "C" fn update_guild_map_render_data(lo: u32, hi: u32, map: u32, tint: u32) -> u32 {
        l2::update_guild_map(&mut mem(), lo, hi, map, tint)
    }
    // ---- continuation round 10, batch 3 (ffx_glue_other::h4_l3, h4_l4, h4_l5)
    /// `PhyFMVPlayerManager_createVideoIndexMap(this; english)` (a
    /// 0x3360-byte frame, as the original's; the main thread at boot).
    PhyFMVPlayerManager_createVideoIndexMap @ 0x006d_82f0 => extern "thiscall" fn phy_fmv_player_manager_create_video_index_map(this: u32, english: u32) {
        let mut frame = StackFrame::<{ (l3::VIDEO_FRAME / 4) as usize }>::new();
        l3::create_video_index_map(&mut mem(), frame.at(), this, english);
    }
    /// `menu_list_drawShape_00774cb0(unused, item, x, y)` -> the draw's
    /// result.
    menu_list_drawShape_00774cb0 @ 0x0077_4cb0 => extern "C" fn menu_list_draw_shape(_unused: u32, item: u32, x: u32, y: u32) -> u32 {
        let mut frame = StackFrame::<{ (l4::SHAPE_ICON_FRAME / 4) as usize }>::new();
        l4::draw_shape(&mut mem(), frame.at(), item, x, y)
    }
    /// `FFEscMenu_update(this)` (`ecx`, plain ret) -> the handlers'
    /// flush result (0 where the original returns its caller's `eax`).
    FFEscMenu_update @ 0x0068_ffb0 => extern "thiscall" fn ff_esc_menu_update(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (l5::ESC_FRAME / 4) as usize }>::new();
        l5::esc_update(&mut mem(), frame.at(), this).unwrap_or(0)
    }
    // ---- continuation round 11, batch 1 (ffx_glue_other::h4_m1)
    /// `MenuDynamicGeometry_setShaderClusterAndMaterial(this; cluster,
    /// name, load)` -> 0, 1, 3, 0x13 or the texture binding's error (a
    /// 0x60-byte frame: the original's locals and argument words).
    MenuDynamicGeometry_setShaderClusterAndMaterial @ 0x006a_3100 => extern "thiscall" fn menu_dynamic_geometry_set_shader_cluster_and_material(this: u32, cluster: u32, name: u32, load: u32) -> u32 {
        let mut frame = StackFrame::<{ (m1::MATERIAL_FRAME / 4) as usize }>::new();
        m1::set_shader_cluster_and_material(&mut mem(), frame.at(), this, cluster, name, load)
    }
    /// `SkipPrompt_buildQuad_006e7940(x, y, w, h, &mesh, &instance,
    /// &materials, material)` (the floats as bits) -> the second unmap's
    /// result (a 0x10-byte frame: the two mapped data pointers).
    SkipPrompt_buildQuad_006e7940 @ 0x006e_7940 => extern "stdcall" fn skip_prompt_build_quad(x: u32, y: u32, w: u32, h: u32, mesh: u32, instance: u32, materials: u32, material: u32) -> u32 {
        let mut frame = StackFrame::<{ (m2::QUAD_FRAME / 4) as usize }>::new();
        m2::build_quad(&mut mem(), frame.at(), [x, y, w, h], mesh, instance, materials, material)
    }
    /// `PhyFMVPlayerManager_initialize(this; id, wide, flag)` -> 0 or the
    /// texture register's error (a 0x100-byte frame: the shader cluster's
    /// path; the main thread).
    PhyFMVPlayerManager_initialize @ 0x006d_9c80 => extern "thiscall" fn phy_fmv_player_manager_initialize(this: u32, id: u32, wide: u32, flag: u32) -> u32 {
        let mut frame = StackFrame::<{ (m3::INIT_FRAME / 4) as usize }>::new();
        m3::initialize(&mut mem(), frame.at(), this, id, wide, flag)
    }
    /// `LoadingScreen_prepareMesh(this)` (`ecx`, plain ret) -> the last
    /// `SetValue`'s result (0 where the original leaves its frame's
    /// address; a 0x40-byte frame: the texture records, the colour and the
    /// two mapped data pointers).
    LoadingScreen_prepareMesh @ 0x006e_4920 => extern "thiscall" fn loading_screen_prepare_mesh(this: u32) -> u32 {
        let mut frame = StackFrame::<{ (m4::MESH_FRAME / 4) as usize }>::new();
        m4::prepare_mesh(&mut mem(), frame.at(), this).unwrap_or(0)
    }
    // ---- continuation round 11, batch 2 (ffx_glue_other::h4_m5, h4_m6, h4_m7)
    /// `GraphicAbmap_update_00685840(this)` (`ecx`, plain ret;
    /// `graphicAbmapUpdate` tail-jumps to it) -> the original's `eax` (the
    /// state, its negation, or the last geometry destructor's result).
    GraphicAbmap_update_00685840 @ 0x0068_5840 => extern "thiscall" fn graphic_abmap_update(this: u32) -> u32 {
        m5::abmap_update(&mut mem(), this)
    }
    /// `ClassDynamicGeometry_setupMaterial(this; n, names, clusters)` -> 0,
    /// 0xa, 3, 0x13 or a callee's error (a 0xf0-byte frame: the texture
    /// records, the cursor, the material handles and fade values).
    ClassDynamicGeometry_setupMaterial @ 0x006a_3b50 => extern "thiscall" fn class_dynamic_geometry_setup_material(this: u32, n: u32, names: u32, clusters: u32) -> u32 {
        let mut frame = StackFrame::<{ (m6::MATERIAL_FRAME / 4) as usize }>::new();
        m6::setup_material(&mut mem(), frame.at(), this, n, names, clusters)
    }
    /// `ClassDynamicGeometry_updateRenderData(this; unused)` -> 0, 1 or a
    /// binding's error (the word is never read; a 0x100-byte frame: the
    /// segment record's copy, the z matrix, the fade values, the bounds).
    ClassDynamicGeometry_updateRenderData @ 0x006a_4a50 => extern "thiscall" fn class_dynamic_geometry_update_render_data(this: u32, _unused: u32) -> u32 {
        let mut frame = StackFrame::<{ (m7::RENDER_FRAME / 4) as usize }>::new();
        m7::update_render_data(&mut mem(), frame.at(), this)
    }
    // ---- continuation round 11, batch 3 (ffx_glue_other::h4_m8)
    /// `createVFXRenderDataBuffer(lo, hi, effect, data, flags)` -> 1 or 0
    /// (a 0x23d8-byte frame, as the original's: the counts, names and
    /// flags it hands the data buffers; zeroed, so the kinds of segments
    /// that never set theirs read 0).
    createVFXRenderDataBuffer @ 0x0071_3790 => extern "C" fn create_vfx_render_data_buffer(lo: u32, hi: u32, effect: u32, data: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (m8::BUFFER_FRAME / 4) as usize }>::new();
        m8::create_render_data_buffer(&mut mem(), frame.at(), lo, hi, effect, data, flags)
    }
    // ---- continuation round 11, batch 4 (ffx_glue_other::h4_m9)
    /// `updateVFXRenderDataAtelShp(lo, hi, shapes, obj, index)` -> 1 or 0
    /// (the index is a u16; a 0x90-byte frame: the screen size the font
    /// call fills, the bounds vectors the matrix transforms).
    updateVFXRenderDataAtelShp @ 0x0071_96f0 => extern "C" fn update_vfx_render_data_atel_shp(lo: u32, hi: u32, shapes: u32, obj: u32, index: u32) -> u32 {
        let mut frame = StackFrame::<{ (m9::ATEL_FRAME / 4) as usize }>::new();
        m9::update_atel_shp(&mut mem(), frame.at(), lo, hi, shapes, obj, index)
    }
    // ---- round h4x, batch 1 (ffx_glue_other::h4x_video)
    /// `FFEscMenu_initializeVideoSettingPage(this)` (a 0xac8-byte frame:
    /// the Iggy values, the choices' `LocString`s and lists).
    FFEscMenu_initializeVideoSettingPage @ 0x006f_cca0 => extern "thiscall" fn video_setting_page(this: u32) -> u32 {
        use ffx_glue_other::h4x_video as xv;
        let mut frame = StackFrame::<{ (xv::VIDEO_FRAME / 4) as usize }>::new();
        xv::video_setting_page(&mut mem(), frame.at(), this)
    }
}

/// `_CIfmod(a, b)` through the game's import thunk (`0x009498e0`): `a` in
/// `ST1`, `b` in `ST0` (`fld qword` each), the result read back from
/// `ST0` as `fstp qword` stores it (exact: the remainder of two `f64`s is
/// one).
fn ci_fmod(a: f64, b: f64) -> f64 {
    let f = crate::rebase(k1::CI_FMOD);
    #[cfg(target_arch = "x86")]
    {
        let pair = [a, b];
        let mut out = 0f64;
        // SAFETY: the thunk jumps to the CRT's `_CIfmod`, which takes its
        // operands in ST1 and ST0 and leaves the result alone in ST0 (eax,
        // ecx, edx, the x87 and SSE registers clobbered); the asm loads
        // them, calls it and stores the result through edi (which the
        // callee keeps), leaving the x87 stack empty.
        unsafe {
            std::arch::asm!(
                "fld qword ptr [{p}]",
                "fld qword ptr [{p} + 8]",
                "call {f}",
                "fstp qword ptr [edi]",
                p = in(reg) &raw const pair,
                in("edi") &raw mut out,
                f = in(reg) f,
                clobber_abi("C"),
            );
        }
        out
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = (f, a, b);
        0.0
    }
}
