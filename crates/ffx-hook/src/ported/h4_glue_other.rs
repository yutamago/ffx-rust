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
    copies as c, effect as e, frames as fr, menu as mn, misc as m, movie as mv, sound as s,
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
}
