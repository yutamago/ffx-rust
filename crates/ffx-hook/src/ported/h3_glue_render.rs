//! `ffx_prender::glue`: the game's render-side glue (M4 port round 1,
//! list `h3-glue-render`). One-line forwarders return their callee's `eax`
//! (the original leaves it to its callers).
//!
//! Threads: the main thread, and the loader's `asyncFFXInitialize` job;
//! the shims keep no state.

use super::{mem, ported};
use ffx_prender::glue::bridge as g;
use ffx_prender::glue2::bridge as g2;

ported! {
    // ---- g_PhyreScene one-liners ----
    /// `graphicActivateLoadingScreen()`.
    graphicActivateLoadingScreen @ 0x0063_9130 => extern "C" fn activate_loading_screen() -> u32 {
        g::activate_loading_screen(&mut mem())
    }
    /// `graphicDeActivateLoadingScreen()`.
    graphicDeActivateLoadingScreen @ 0x0063_de40 => extern "C" fn deactivate_loading_screen() -> u32 {
        g::deactivate_loading_screen(&mut mem())
    }
    /// `graphicSceneClearList_0063dac0()`.
    graphicSceneClearList_0063dac0 @ 0x0063_dac0 => extern "C" fn scene_clear_list() -> u32 {
        g::scene_clear_list(&mut mem())
    }
    /// `graphicDrawMainMenuWaterEffect()`.
    graphicDrawMainMenuWaterEffect @ 0x0063_e920 => extern "C" fn draw_main_menu_water_effect() -> u32 {
        g::draw_main_menu_water_effect(&mut mem())
    }
    /// `graphicFieldMapShutdown()`.
    graphicFieldMapShutdown @ 0x0064_03b0 => extern "C" fn field_map_shutdown() -> u32 {
        g::field_map_shutdown(&mut mem())
    }
    /// `graphicFrameStart_00640df0()`.
    graphicFrameStart_00640df0 @ 0x0064_0df0 => extern "C" fn frame_start() {
        g::frame_start(&mut mem());
    }
    /// `graphicLevelGetName()`.
    graphicLevelGetName @ 0x0064_1d90 => extern "C" fn level_get_name() -> u32 {
        g::level_get_name(&mut mem())
    }
    /// `graphicRenderLoadingScreen_00642c00()`.
    graphicRenderLoadingScreen_00642c00 @ 0x0064_2c00 => extern "C" fn render_loading_screen() -> u32 {
        g::render_loading_screen(&mut mem())
    }
    /// `graphicShowAutoSavePrompt()`.
    graphicShowAutoSavePrompt @ 0x0064_3de0 => extern "C" fn show_auto_save_prompt() {
        g::show_auto_save_prompt(&mut mem());
    }
    /// `graphicTextureVideoUpdate()`.
    graphicTextureVideoUpdate @ 0x0064_42c0 => extern "C" fn texture_video_update() -> u32 {
        g::texture_video_update(&mut mem())
    }
    /// `graphicFieldMapInitialize()`.
    graphicFieldMapInitialize @ 0x0064_0200 => extern "C" fn field_map_initialize() -> u32 {
        g::field_map_initialize(&mut mem())
    }
    /// `graphicIsSplitScreenEnabled()`.
    graphicIsSplitScreenEnabled @ 0x0064_1cc0 => extern "C" fn is_split_screen_enabled() -> u32 {
        g::is_split_screen_enabled(&mut mem())
    }

    // ---- g_PhyreScene with one argument ----
    /// `graphicAtelLoadTexture_00639310(id)`.
    graphicAtelLoadTexture_00639310 @ 0x0063_9310 => extern "C" fn atel_load_texture(id: u32) -> u32 {
        g::atel_load_texture(&mut mem(), id)
    }
    /// `graphicAtelUnloadTexture(id)`.
    graphicAtelUnloadTexture @ 0x0063_9330 => extern "C" fn atel_unload_texture(id: u32) -> u32 {
        g::atel_unload_texture(&mut mem(), id)
    }
    /// `graphicCharacterInstanceAddToPipeline(chr)`.
    graphicCharacterInstanceAddToPipeline @ 0x0063_a010 => extern "C" fn character_instance_add_to_pipeline(chr: u32) {
        g::character_instance_add_to_pipeline(&mut mem(), chr);
    }
    /// `graphicSetTidusPos_00641080(out)`.
    graphicSetTidusPos_00641080 @ 0x0064_1080 => extern "C" fn get_cha_pos(out: u32) -> u32 {
        g::get_cha_pos(&mut mem(), out)
    }
    /// `graphicMagicSetPath(path)`.
    graphicMagicSetPath @ 0x0064_2050 => extern "C" fn magic_set_path(path: u32) -> u32 {
        g::magic_set_path(&mut mem(), path)
    }
    /// `graphicMagicSetToBeDeletePath(path)`.
    graphicMagicSetToBeDeletePath @ 0x0064_2070 => extern "C" fn magic_set_to_be_delete_path(path: u32) -> u32 {
        g::magic_set_to_be_delete_path(&mut mem(), path)
    }
    /// `graphicMagicVFXLoadTexture(id)`.
    graphicMagicVFXLoadTexture @ 0x0064_20b0 => extern "C" fn magic_vfx_load_texture(id: u32) -> u32 {
        g::magic_vfx_load_texture(&mut mem(), id)
    }
    /// `graphicMagicVFXUnloadTexture(id)`.
    graphicMagicVFXUnloadTexture @ 0x0064_20f0 => extern "C" fn magic_vfx_unload_texture(id: u32) -> u32 {
        g::magic_vfx_unload_texture(&mut mem(), id)
    }
    /// `graphicMeshInstanceGroupIdSetVisible(id)`.
    graphicMeshInstanceGroupIdSetVisible @ 0x0064_23d0 => extern "C" fn mesh_instance_group_id_set_visible(id: u32) -> u32 {
        g::mesh_instance_group_id_set_visible(&mut mem(), id)
    }
    /// `graphicMenuUnloadSpecialTexture(id)`.
    graphicMenuUnloadSpecialTexture @ 0x0064_2390 => extern "C" fn menu_unload_special_texture(id: u32) -> u32 {
        g::menu_unload_special_texture(&mut mem(), id)
    }

    // ---- dynamic geometry, the character shader renderer ----
    /// `graphicDynGeoMemManagerClearBuffer()`.
    graphicDynGeoMemManagerClearBuffer @ 0x0063_fd80 => extern "C" fn dyn_geo_clear_buffer() {
        g::dyn_geo_clear_buffer(&mut mem());
    }
    /// `graphicDynGeoMemManagerGetCurrentBufferId()`.
    graphicDynGeoMemManagerGetCurrentBufferId @ 0x0063_ff50 => extern "C" fn dyn_geo_current_buffer_id() -> u32 {
        g::dyn_geo_current_buffer_id(&mut mem())
    }
    /// `graphicDynGeoMemManagerLockForRecordNumInKeLns()`.
    graphicDynGeoMemManagerLockForRecordNumInKeLns @ 0x0063_ff60 => extern "C" fn dyn_geo_lock() -> u32 {
        g::dyn_geo_lock(&mut mem())
    }
    /// `graphicDynGeoMemManagerUnlockForRecordNumInKeLns()`.
    graphicDynGeoMemManagerUnlockForRecordNumInKeLns @ 0x0064_0050 => extern "C" fn dyn_geo_unlock() -> u32 {
        g::dyn_geo_unlock(&mut mem())
    }
    /// `DynGeoMemManager_lockForRecordNumInKeLns(this)`.
    DynGeoMemManager_lockForRecordNumInKeLns @ 0x006d_eaf0 => extern "thiscall" fn lock_for_record_num(this: u32) -> u32 {
        g::lock_for_record_num(&mut mem(), this)
    }
    /// `DynGeoMemManager_unlockForRecordNumInKeLns(this)`.
    DynGeoMemManager_unlockForRecordNumInKeLns @ 0x006d_ed50 => extern "thiscall" fn unlock_for_record_num(this: u32) -> u32 {
        g::unlock_for_record_num(&mut mem(), this)
    }
    /// `DynGeoMemManager_swapDynGeoMemory(this)`.
    DynGeoMemManager_swapDynGeoMemory @ 0x006d_ed40 => extern "thiscall" fn swap_dyn_geo_memory(this: u32) {
        g::swap_dyn_geo_memory(&mut mem(), this);
    }
    /// `graphicShaderGetVertexInfo()`.
    graphicShaderGetVertexInfo @ 0x0064_3db0 => extern "C" fn shader_get_vertex_info() -> u32 {
        g::shader_get_vertex_info(&mut mem())
    }
    /// `graphicShadowStart_00643dc0()`.
    graphicShadowStart_00643dc0 @ 0x0064_3dc0 => extern "C" fn shadow_start() -> u32 {
        g::shadow_start(&mut mem())
    }
    /// `graphicShadowEnd_00643dd0()`.
    graphicShadowEnd_00643dd0 @ 0x0064_3dd0 => extern "C" fn shadow_end() {
        g::shadow_end(&mut mem());
    }

    // ---- manager singletons ----
    /// `graphicBrokenScreenSetFlag5_00639360()`.
    graphicBrokenScreenSetFlag5_00639360 @ 0x0063_9360 => extern "C" fn broken_screen_set_flag5() -> u32 {
        g::broken_screen_set_flag5(&mut mem())
    }
    /// `graphicBrokenScreenSetFlag4_00639370()`.
    graphicBrokenScreenSetFlag4_00639370 @ 0x0063_9370 => extern "C" fn broken_screen_set_flag4() -> u32 {
        g::broken_screen_set_flag4(&mut mem())
    }
    /// `graphicBrokenScreenSetFlag6_00639390()`.
    graphicBrokenScreenSetFlag6_00639390 @ 0x0063_9390 => extern "C" fn broken_screen_set_flag6() -> u32 {
        g::broken_screen_set_flag6(&mut mem())
    }
    /// `graphicBrokenScreenReset()`.
    graphicBrokenScreenReset @ 0x0063_9490 => extern "C" fn broken_screen_reset() {
        g::broken_screen_reset(&mut mem());
    }
    /// `graphicBrokenScreenStartKeepFrame()`.
    graphicBrokenScreenStartKeepFrame @ 0x0063_94a0 => extern "C" fn broken_screen_start_keep_frame() -> u32 {
        g::broken_screen_start_keep_frame(&mut mem())
    }
    /// `graphicGetCurrentFrame()`.
    graphicGetCurrentFrame @ 0x0064_10d0 => extern "C" fn get_current_frame() -> u32 {
        g::get_current_frame(&mut mem())
    }
    /// `graphicVideoPause()`.
    graphicVideoPause @ 0x0064_5de0 => extern "C" fn video_pause() {
        g::video_pause(&mut mem());
    }
    /// `graphicDestroyFmv()`.
    graphicDestroyFmv @ 0x0063_df30 => extern "C" fn destroy_fmv() -> u32 {
        g::destroy_fmv(&mut mem())
    }
    /// `graphicIsVideoBusy()`.
    graphicIsVideoBusy @ 0x0064_1ce0 => extern "C" fn is_video_busy() -> u32 {
        g::is_video_busy(&mut mem())
    }
    /// `graphicIsVideoPlaying()`.
    graphicIsVideoPlaying @ 0x0064_1cf0 => extern "C" fn is_video_playing() -> u32 {
        g::is_video_playing(&mut mem())
    }
    /// `PhyreScene_isPlayingVideo()`.
    PhyreScene_isPlayingVideo @ 0x0064_5db0 => extern "C" fn scene_is_playing_video() -> u32 {
        g::scene_is_playing_video(&mut mem())
    }
    /// `graphicMagicVFXUnloadLoadSpecialShaderTable()`.
    graphicMagicVFXUnloadLoadSpecialShaderTable @ 0x0064_20e0 => extern "C" fn magic_vfx_unload_special_shader_table() -> u32 {
        g::magic_vfx_unload_special_shader_table(&mut mem())
    }
    /// `graphicUpdateShadowVisibility()`.
    graphicUpdateShadowVisibility @ 0x0064_4ab0 => extern "C" fn update_shadow_visibility() -> u32 {
        g::update_shadow_visibility(&mut mem())
    }
    /// `graphicGetFileReadSemaId()`.
    graphicGetFileReadSemaId @ 0x0064_1150 => extern "C" fn get_file_read_sema_id() -> u32 {
        g::get_file_read_sema_id(&mut mem())
    }
    /// `graphicGetOldFileReadCompleteSemaId()`.
    graphicGetOldFileReadCompleteSemaId @ 0x0064_1360 => extern "C" fn get_old_file_read_complete_sema_id() -> u32 {
        g::get_old_file_read_complete_sema_id(&mut mem())
    }

    // ---- the application ----
    /// `PhyreScene_startLoadingScreen()` (reads no `this`).
    PhyreScene_startLoadingScreen @ 0x0066_fb30 => extern "C" fn start_loading_screen() -> u32 {
        g::start_loading_screen(&mut mem())
    }
    /// `PhyreScene_stopLoadingScreen()` (reads no `this`).
    PhyreScene_stopLoadingScreen @ 0x0066_fb50 => extern "C" fn stop_loading_screen() -> u32 {
        g::stop_loading_screen(&mut mem())
    }
    /// `graphicIsLoadingScreenActive_00641c40()`.
    graphicIsLoadingScreenActive_00641c40 @ 0x0064_1c40 => extern "C" fn is_loading_screen_active() -> u32 {
        g::is_loading_screen_active(&mut mem())
    }
    /// `graphicSetFlipVsnc(interval)`.
    graphicSetFlipVsnc @ 0x0064_30e0 => extern "C" fn set_flip_vsync(interval: u32) -> u32 {
        g::set_flip_vsync(&mut mem(), interval)
    }

    // ---- scene and geometry methods ----
    /// `PhyreScene_setMagicPath_0066cfb0(this, path)`.
    PhyreScene_setMagicPath_0066cfb0 @ 0x0066_cfb0 => extern "thiscall" fn scene_set_magic_path(this: u32, path: u32) -> u32 {
        g::scene_set_magic_path(&mut mem(), this, path)
    }
    /// `PhyreScene_setToBeDeletePath_0066e830(this, path)`.
    PhyreScene_setToBeDeletePath_0066e830 @ 0x0066_e830 => extern "thiscall" fn scene_set_to_be_delete_path(this: u32, path: u32) -> u32 {
        g::scene_set_to_be_delete_path(&mut mem(), this, path)
    }
    /// `PhyreScene_loadAtelVFXTexture_00663f00(this, id)`.
    PhyreScene_loadAtelVFXTexture_00663f00 @ 0x0066_3f00 => extern "thiscall" fn scene_load_atel_vfx_texture(this: u32, id: u32) -> u32 {
        g::scene_load_atel_vfx_texture(&mut mem(), this, id)
    }
    /// `PhyreScene_unloadAtelVFXTexture_00670b00(this, id)`.
    PhyreScene_unloadAtelVFXTexture_00670b00 @ 0x0067_0b00 => extern "thiscall" fn scene_unload_atel_vfx_texture(this: u32, id: u32) -> u32 {
        g::scene_unload_atel_vfx_texture(&mut mem(), this, id)
    }
    /// `PhyreScene_releaseVFXMeshes(this)`.
    PhyreScene_releaseVFXMeshes @ 0x0065_9090 => extern "thiscall" fn scene_release_vfx_meshes(this: u32) -> u32 {
        g::scene_release_vfx_meshes(&mut mem(), this)
    }
    /// `VFXDynamicGeometry_setZCorrectionValue(this, value)`.
    VFXDynamicGeometry_setZCorrectionValue @ 0x006a_3b20 => extern "thiscall" fn set_z_correction(this: u32, value: f32) {
        g::set_z_correction(&mut mem(), this, value);
    }
    /// `ClassDynamicGeometry_getReadyA8_006a2010(this)`.
    ClassDynamicGeometry_getReadyA8_006a2010 @ 0x006a_2010 => extern "thiscall" fn get_ready_a8(this: u32) -> u32 {
        g::get_ready_a8(&mem(), this)
    }

    // ---- batch 2 (ffx_prender::glue2) ----
    /// `graphicActivateBlurEffectEx(target)`.
    graphicActivateBlurEffectEx @ 0x0063_9110 => extern "C" fn activate_blur_effect_ex(target: u32) -> u32 {
        g2::activate_blur_effect_ex(&mut mem(), target)
    }
    /// `graphicSetFlipVsnc_00643810(v)`.
    graphicSetFlipVsnc_00643810 @ 0x0064_3810 => extern "C" fn blur_set_field84(v: u32) -> u32 {
        g2::blur_set_field84(&mut mem(), v)
    }
    /// `graphicBlurEffectSetFloats6c_00643890(a, b)` (float bits).
    graphicBlurEffectSetFloats6c_00643890 @ 0x0064_3890 => extern "C" fn blur_set_floats(a: u32, b: u32) -> u32 {
        g2::blur_set_floats(&mut mem(), a, b)
    }
    /// `graphicMagicVFXLoadSpecialShaderTable(v)`.
    graphicMagicVFXLoadSpecialShaderTable @ 0x0064_2090 => extern "C" fn magic_vfx_load_special_shader_table(v: u32) -> u32 {
        g2::magic_vfx_load_special_shader_table(&mut mem(), v)
    }
    /// `graphicSetDistortionSkipped(on)`.
    graphicSetDistortionSkipped @ 0x0064_2f80 => extern "C" fn set_distortion_skipped(on: u32) -> u32 {
        g2::set_distortion_skipped(&mut mem(), on)
    }
    /// `graphicSetDistortionEnable(on)`.
    graphicSetDistortionEnable @ 0x0064_2f50 => extern "C" fn set_distortion_enable(on: u32) -> u32 {
        g2::set_distortion_enable(&mut mem(), on)
    }
    /// `graphicSetDistortionBlur(f)` (float bits).
    graphicSetDistortionBlur @ 0x0064_2f30 => extern "C" fn set_distortion_blur(f: u32) -> u32 {
        g2::set_distortion_blur(&mut mem(), f)
    }
    /// `graphicSetFogLimit(f)` (float bits).
    graphicSetFogLimit @ 0x0064_31c0 => extern "C" fn set_fog_limit(f: u32) -> u32 {
        g2::set_fog_limit(&mut mem(), f)
    }
    /// `graphicSetMotionBlurWeight(f)` (float bits).
    graphicSetMotionBlurWeight @ 0x0064_3750 => extern "C" fn set_motion_blur_weight(f: u32) {
        g2::set_motion_blur_weight(&mut mem(), f);
    }
    /// `graphicSetMotionBlurEnable(on)`.
    graphicSetMotionBlurEnable @ 0x0064_3730 => extern "C" fn set_motion_blur_enable(on: u32) -> u32 {
        g2::set_motion_blur_enable(&mut mem(), on)
    }
    /// `graphicCrossFadeCaptureStart_00639150()`.
    graphicCrossFadeCaptureStart_00639150 @ 0x0063_9150 => extern "C" fn cross_fade_capture_start() -> u32 {
        g2::cross_fade_capture_start(&mut mem())
    }
    /// `WaterDistortionEffectDelegations_captureFrame(p)`.
    WaterDistortionEffectDelegations_captureFrame @ 0x006f_5420 => extern "C" fn water_distortion_capture_frame(p: u32) -> u32 {
        g2::water_distortion_capture_frame(&mut mem(), p)
    }
    /// `graphicBrokenScreenUpdateOpacity(f)` (float bits).
    graphicBrokenScreenUpdateOpacity @ 0x0063_94e0 => extern "C" fn broken_screen_update_opacity(f: u32) -> u32 {
        g2::broken_screen_update_opacity(&mut mem(), f)
    }
    /// `graphicBrokenScreenInitializeBlockDataWithGivenBuffer(a, b, c)`.
    graphicBrokenScreenInitializeBlockDataWithGivenBuffer @ 0x0063_9470 => extern "C" fn broken_screen_init_block_data(a: u32, b: u32, c: u32) -> u32 {
        g2::broken_screen_init_block_data(&mut mem(), a, b, c)
    }
    /// `graphicBrokenScreenUpdateLightParameters(7 words)`.
    graphicBrokenScreenUpdateLightParameters @ 0x0063_94b0 => extern "C" fn broken_screen_update_light_parameters(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, k: u32) -> u32 {
        g2::broken_screen_update_light_parameters(&mut mem(), [a, b, c, d, e, f, k])
    }
    /// `graphicBrokenScreenUpdatePolygonWMatrix(a, b, m)`.
    graphicBrokenScreenUpdatePolygonWMatrix @ 0x0063_9540 => extern "C" fn broken_screen_update_polygon_matrix(a: u32, b: u32, m: u32) -> u32 {
        g2::broken_screen_update_polygon_matrix(&mut mem(), a, b, m)
    }
    /// `graphicBrokenScreenBlockBytes_006393a0(n)`.
    graphicBrokenScreenBlockBytes_006393a0 @ 0x0063_93a0 => extern "C" fn broken_screen_block_bytes(n: u32) -> u32 {
        g2::broken_screen_block_bytes(&mut mem(), n)
    }
    /// `graphicSetChadist(f)` (float bits).
    graphicSetChadist @ 0x0064_2ef0 => extern "C" fn set_chadist(f: u32) {
        g2::set_chadist(&mut mem(), f);
    }
    /// `graphicCharacterInsertDrawSituation(chr)`.
    graphicCharacterInsertDrawSituation @ 0x0063_9ff0 => extern "C" fn character_insert_draw_situation(chr: u32) {
        g2::character_insert_draw_situation(&mut mem(), chr);
    }
    /// `graphicFieldMapUnload()`.
    graphicFieldMapUnload @ 0x0064_03c0 => extern "C" fn field_map_unload() -> u32 {
        g2::field_map_unload(&mut mem())
    }
    /// `graphicRender()`.
    graphicRender @ 0x0064_2bb0 => extern "C" fn render() -> u32 {
        g2::render(&mut mem())
    }
    /// `graphicTextureAnimationSetMaterialVisiability(layer, alpha)`.
    graphicTextureAnimationSetMaterialVisiability @ 0x0064_4120 => extern "C" fn texture_animation_set_material_visibility(layer: u32, alpha: u32) -> u32 {
        g2::texture_animation_set_material_visibility(&mut mem(), layer, alpha)
    }
    /// `graphicSetMagicFileAsyncLoadEnable(v)`.
    graphicSetMagicFileAsyncLoadEnable @ 0x0064_35e0 => extern "C" fn set_magic_file_async_load_enable(v: u32) -> u32 {
        g2::set_magic_file_async_load_enable(&mut mem(), v)
    }
    /// `PhyreScene_forceUnloadMenuTexture(this, id)` (`this` unused).
    PhyreScene_forceUnloadMenuTexture @ 0x0065_d780 => extern "thiscall" fn scene_force_unload_menu_texture(_this: u32, id: u32) -> u32 {
        g2::scene_force_unload_menu_texture(&mut mem(), id)
    }
    /// `PhyreScene_unloadMenuTexture(this, id)` (`this` unused).
    PhyreScene_unloadMenuTexture @ 0x0067_1120 => extern "thiscall" fn scene_unload_menu_texture(_this: u32, id: u32) -> u32 {
        g2::scene_unload_menu_texture(&mut mem(), id)
    }
    /// `PhyreScene_loadMenuTexture(this, id)` (`this` unused).
    PhyreScene_loadMenuTexture @ 0x0066_51d0 => extern "thiscall" fn scene_load_menu_texture(_this: u32, id: u32) -> u32 {
        g2::scene_load_menu_texture(&mut mem(), id)
    }
    /// `PhyreScene_onCutbufferShaderLoaded(shader, task)`.
    PhyreScene_onCutbufferShaderLoaded @ 0x0066_6610 => extern "C" fn scene_on_cutbuffer_shader_loaded(shader: u32, task: u32) -> u32 {
        g2::scene_on_cutbuffer_shader_loaded(&mut mem(), shader, task)
    }
    /// `PhyreScene_deleteIggyMenu_0065b750(this)`.
    PhyreScene_deleteIggyMenu_0065b750 @ 0x0065_b750 => extern "thiscall" fn scene_delete_iggy_menu(this: u32) {
        g2::scene_delete_iggy_menu(&mut mem(), this);
    }
    /// `graphicAsyncTaskCallback(task)`.
    graphicAsyncTaskCallback @ 0x0063_7860 => extern "C" fn async_task_callback(task: u32) -> u32 {
        g2::async_task_callback(&mut mem(), task)
    }
    /// `graphicRegistAsyncTask(arg, b)`.
    graphicRegistAsyncTask @ 0x0064_2950 => extern "C" fn regist_async_task(arg: u32, b: u32) -> u32 {
        g2::regist_async_task(&mut mem(), arg, b)
    }
    /// `graphicTryAsyncLoadIfAvailable(arg, b)` (`graphicRegistAsyncTask`'s code).
    graphicTryAsyncLoadIfAvailable @ 0x0064_42f0 => extern "C" fn try_async_load_if_available(arg: u32, b: u32) -> u32 {
        g2::regist_async_task(&mut mem(), arg, b)
    }
    /// `PhyreScene_asyncDoLoadCharacterTask(task)`.
    PhyreScene_asyncDoLoadCharacterTask @ 0x0065_7850 => extern "C" fn async_load_character_task(task: u32) -> u32 {
        g2::async_load_character_task(&mut mem(), task)
    }
    /// `PhyreScene_asyncDoloadMagicFileVFXTextureTask(task)`.
    PhyreScene_asyncDoloadMagicFileVFXTextureTask @ 0x0065_7880 => extern "C" fn async_load_magic_file_vfx_texture_task(task: u32) -> u32 {
        g2::async_load_magic_file_vfx_texture_task(&mut mem(), task)
    }
    /// `PhyreScene_asyncDoloadVFXTextureTask(task)`.
    PhyreScene_asyncDoloadVFXTextureTask @ 0x0065_78b0 => extern "C" fn async_load_vfx_texture_task(task: u32) -> u32 {
        g2::async_load_vfx_texture_task(&mut mem(), task)
    }
    /// `graphicMenuLoadSpecialTexture(a, b, c)`.
    graphicMenuLoadSpecialTexture @ 0x0064_2300 => extern "C" fn menu_load_special_texture(a: u32, b: u32, c: u32) -> u32 {
        g2::menu_load_special_texture(&mut mem(), a, b, c)
    }
    /// `graphicAbmapGetVertexInfo(a, b)`.
    graphicAbmapGetVertexInfo @ 0x0063_8fd0 => extern "C" fn abmap_get_vertex_info(a: u32, b: u32) -> u32 {
        g2::abmap_get_vertex_info(&mut mem(), a, b)
    }
    /// `graphicAbmapResetLayers_006390d0(v)`.
    graphicAbmapResetLayers_006390d0 @ 0x0063_90d0 => extern "C" fn abmap_reset_layers(v: u32) {
        g2::abmap_reset_layers(&mut mem(), v);
    }
    /// `graphicAbmapCreate(a)`.
    graphicAbmapCreate @ 0x0063_8f90 => extern "C" fn abmap_create(a: u32) -> u32 {
        g2::abmap_create(&mut mem(), a)
    }
    /// `graphicAbmapDestory()`.
    graphicAbmapDestory @ 0x0063_8fb0 => extern "C" fn abmap_destroy() {
        g2::abmap_destroy(&mut mem());
    }
    /// `graphicMapPartLangOverride_006412b0(a, b)`.
    graphicMapPartLangOverride_006412b0 @ 0x0064_12b0 => extern "C" fn map_part_lang_override(a: u32, b: u32) -> u32 {
        g2::map_part_lang_override(&mut mem(), a, b)
    }
    /// `graphicSetUiDrawType(v)`.
    graphicSetUiDrawType @ 0x0064_3af0 => extern "C" fn set_ui_draw_type(v: u32) {
        g2::set_ui_draw_type(&mut mem(), v);
    }
    /// `graphicMenuReset()`.
    graphicMenuReset @ 0x0064_2320 => extern "C" fn menu_reset() -> u32 {
        g2::menu_reset(&mut mem())
    }
    /// `graphicMenuUpdate()`.
    graphicMenuUpdate @ 0x0064_23b0 => extern "C" fn menu_update() -> u32 {
        g2::menu_update(&mut mem())
    }
    /// `graphicUpdateMapYIQ()`.
    graphicUpdateMapYIQ @ 0x0064_4990 => extern "C" fn update_map_yiq() -> u32 {
        g2::update_map_yiq(&mut mem())
    }
    /// `graphicGetLocTextWithLength(a, b, c)`.
    graphicGetLocTextWithLength @ 0x0064_12f0 => extern "C" fn get_loc_text_with_length(a: u32, b: u32, c: u32) -> u32 {
        g2::get_loc_text_with_length(&mut mem(), a, b, c)
    }
    /// `graphicSetMaterialUV_006436e0(a, b, c)`.
    graphicSetMaterialUV_006436e0 @ 0x0064_36e0 => extern "C" fn set_material_uv(a: u32, b: u32, c: u32) -> u32 {
        g2::set_material_uv(&mut mem(), a, b, c)
    }
    /// `graphicTextureAnimationSetPaused_00643a00(a, paused)`.
    graphicTextureAnimationSetPaused_00643a00 @ 0x0064_3a00 => extern "C" fn texture_animation_set_paused(a: u32, paused: u32) -> u32 {
        g2::texture_animation_set_paused(&mut mem(), a, paused)
    }
    /// `graphicPlayVideo(on)`.
    graphicPlayVideo @ 0x0064_27c0 => extern "C" fn play_video(on: u32) -> u32 {
        g2::play_video(&mut mem(), on)
    }
    /// `graphicInitFMVPlayer(a, b)`.
    graphicInitFMVPlayer @ 0x0064_1690 => extern "C" fn init_fmv_player(a: u32, b: u32) -> u32 {
        g2::init_fmv_player(&mut mem(), a, b)
    }
    /// `graphicVideoUpdate()`.
    graphicVideoUpdate @ 0x0064_5df0 => extern "C" fn video_update() -> u32 {
        g2::video_update(&mut mem())
    }
    /// `graphicsRegisterResetScreenFadeDelegation(v)`.
    graphicsRegisterResetScreenFadeDelegation @ 0x0064_5e60 => extern "C" fn register_reset_screen_fade_delegation(v: u32) -> u32 {
        g2::register_reset_screen_fade_delegation(&mut mem(), v)
    }
    /// `ShadowManager_Initialize(this)`.
    ShadowManager_Initialize @ 0x006c_e100 => extern "thiscall" fn shadow_manager_initialize(this: u32) -> u32 {
        g2::shadow_manager_initialize(&mut mem(), this)
    }
    /// `ModelBuffer_ClearAll()`.
    ModelBuffer_ClearAll @ 0x0072_ac30 => extern "C" fn model_buffer_clear_all() -> u32 {
        g2::model_buffer_clear_all(&mut mem())
    }
    /// `graphicSetForceNoVsync(on)`.
    graphicSetForceNoVsync @ 0x0064_3200 => extern "C" fn set_force_no_vsync(on: u32) -> u32 {
        g2::set_force_no_vsync(&mut mem(), on)
    }
    /// `FFXApplication_getModeOr2_0042f750(this)`.
    FFXApplication_getModeOr2_0042f750 @ 0x0042_f750 => extern "thiscall" fn application_mode_or_2(this: u32) -> u32 {
        g2::application_mode_or_2(&mem(), this)
    }
}
