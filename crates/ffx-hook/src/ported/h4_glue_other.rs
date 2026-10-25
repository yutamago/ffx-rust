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

use ffx_glue_other::{effect as e, misc as m, sound as s};

use super::{mem, ported};

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
}
