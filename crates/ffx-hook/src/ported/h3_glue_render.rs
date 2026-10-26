//! `ffx_prender::glue`: the game's render-side glue (M4 port round 1,
//! list `h3-glue-render`). One-line forwarders return their callee's `eax`
//! (the original leaves it to its callers).
//!
//! Threads: the main thread, and the loader's `asyncFFXInitialize` job;
//! the shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::glue::bridge as g;
use ffx_prender::glue2::bridge as g2;
use ffx_prender::glue3::bridge as g3;
use ffx_prender::glue4::bridge as g4;
use ffx_prender::glue5::bridge as g5;
use ffx_prender::glue6::bridge as g6;
use ffx_prender::glue7::bridge as g7;
use ffx_prender::glue8::bridge as g8;
use ffx_prender::glue9::bridge as g9;
use ffx_prender::glue10::bridge as g10;
use ffx_prender::glue11::bridge as g11;
use ffx_prender::h3_glue12::bridge as g12;
use ffx_prender::h3_glue13::bridge as g13;
use ffx_prender::h3_glue14::bridge as g14;
use ffx_prender::h3_glue15::bridge as g15;
use ffx_prender::h3_glue16::bridge as g16;
use ffx_prender::h3_glue17::bridge as g17;
use ffx_prender::h3_glue18::bridge as g18;
use ffx_prender::h3_glue19::bridge as g19;
use ffx_prender::h3_glue20::bridge as g20;

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

    // ---- batch 3 (ffx_prender::glue3) ----
    /// `graphicSetMaterialColourScale2(4 floats)` (bits).
    graphicSetMaterialColourScale2 @ 0x0064_3670 => extern "C" fn set_material_colour_scale2(a: u32, b: u32, c: u32, d: u32) {
        g3::set_material_colour_scale2(&mut mem(), [a, b, c, d]);
    }
    /// `graphicBlurEffectSetPositiveParams_00643830(4 floats)` (bits).
    graphicBlurEffectSetPositiveParams_00643830 @ 0x0064_3830 => extern "C" fn blur_set_positive_params(a: u32, b: u32, c: u32, d: u32) -> u32 {
        g3::blur_set_positive_params(&mut mem(), [a, b, c, d])
    }
    /// `graphicBlurEffectSetFloats74_00643860(4 floats)` (bits).
    graphicBlurEffectSetFloats74_00643860 @ 0x0064_3860 => extern "C" fn blur_set_floats74(a: u32, b: u32, c: u32, d: u32) -> u32 {
        g3::blur_set_floats74(&mut mem(), [a, b, c, d])
    }
    /// `graphicAddScreenFadeTask(4 floats, e)`.
    graphicAddScreenFadeTask @ 0x0063_9240 => extern "C" fn add_screen_fade_task(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        g3::add_screen_fade_task(&mut mem(), [a, b, c, d], e, 0x1001)
    }
    /// `graphicAddScreenFadeTaskAfterRender(4 floats, e)`.
    graphicAddScreenFadeTaskAfterRender @ 0x0063_9280 => extern "C" fn add_screen_fade_task_after_render(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        g3::add_screen_fade_task(&mut mem(), [a, b, c, d], e, 0x801)
    }
    /// `PhyreScene_asyncDoFieldMapLoadTask(task)`.
    PhyreScene_asyncDoFieldMapLoadTask @ 0x0065_7820 => extern "C" fn async_field_map_load_task(task: u32) -> u32 {
        g3::async_field_map_load_task(&mut mem(), task)
    }
    /// `SetFields12400_00693be0(this, a, b, c, d)` (`d` unused).
    SetFields12400_00693be0 @ 0x0069_3be0 => extern "thiscall" fn set_fields_12400(this: u32, a: u32, b: u32, c: u32, _d: u32) -> u32 {
        g3::set_fields_12400(&mut mem(), this, a, b, c)
    }
    /// `ClusterManager_newEntry_00699300(this, key)`.
    ClusterManager_newEntry_00699300 @ 0x0069_9300 => extern "thiscall" fn cluster_new_entry(this: u32, key: u32) -> u32 {
        g3::cluster_new_entry(&mut mem(), this, key)
    }
    /// `PhyreScene_isMotionBlurOn_006601e0(this)`: `al`.
    PhyreScene_isMotionBlurOn_006601e0 @ 0x0066_01e0 => extern "thiscall" fn scene_is_motion_blur_on(this: u32) -> u8 {
        g3::scene_is_motion_blur_on(&mut mem(), this)
    }
    /// `PhyreScene_check1195_006662f0(this)`: `al`.
    PhyreScene_check1195_006662f0 @ 0x0066_62f0 => extern "thiscall" fn scene_check_1195(this: u32) -> u8 {
        g3::scene_check_1195(&mut mem(), this)
    }
    /// `PhyreScene_isGlowAfterVFX(this)`: `al`.
    PhyreScene_isGlowAfterVFX @ 0x0065_efa0 => extern "thiscall" fn scene_is_glow_after_vfx(this: u32) -> u8 {
        g3::scene_is_glow_after_vfx(&mut mem(), this)
    }
    /// `graphicCreatePauseGame_006bcc40()`.
    graphicCreatePauseGame_006bcc40 @ 0x006b_cc40 => extern "C" fn create_pause_game() {
        g3::create_pause_game(&mut mem());
    }
    /// `std_Tree_BuyHeadNode_00654ed0()`.
    std_Tree_BuyHeadNode_00654ed0 @ 0x0065_4ed0 => extern "C" fn tree_buy_head_node() -> u32 {
        g3::tree_buy_head_node(&mut mem())
    }
    /// `PhyreScene_resizeTarget_00698060(this, a, b)`.
    PhyreScene_resizeTarget_00698060 @ 0x0069_8060 => extern "thiscall" fn scene_resize_target(this: u32, a: u32, b: u32) -> u32 {
        g3::scene_resize_target(&mut mem(), this, a, b)
    }
    /// `graphicCharacterRegistHelper_00637880(out, obj, x, callback)`.
    graphicCharacterRegistHelper_00637880 @ 0x0063_7880 => extern "C" fn character_regist_helper(out: u32, obj: u32, x: u32, callback: u32) -> u32 {
        g3::character_regist_helper(&mut mem(), out, obj, x, callback)
    }

    // ---- round 2, batch 4 (ffx_prender::glue4) ----
    /// `graphicResetYIQ()`.
    graphicResetYIQ @ 0x0064_2cc0 => extern "C" fn graphic_reset_yiq() -> u32 {
        g4::graphic_reset_yiq(&mut mem())
    }
    /// `PhyreScene_resetYIQ(this)`.
    PhyreScene_resetYIQ @ 0x0066_bf40 => extern "thiscall" fn scene_reset_yiq(this: u32) -> u32 {
        g4::scene_reset_yiq(&mut mem(), this)
    }
    /// `graphicCharacterInstanceUpdateSkeletonounds(list)`.
    graphicCharacterInstanceUpdateSkeletonounds @ 0x0063_c500 => extern "C" fn update_skeleton_bounds(list: u32) {
        g4::update_skeleton_bounds(&mut mem(), list);
    }
    /// `graphicSetAbmap(5 floats)`.
    graphicSetAbmap @ 0x0064_2da0 => extern "C" fn set_abmap(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {
        g4::set_abmap(&mut mem(), [a, b, c, d, e])
    }
    /// `graphicAbmapUpdate()`.
    graphicAbmapUpdate @ 0x0063_90f0 => extern "C" fn abmap_update() {
        g4::abmap_update(&mut mem());
    }
    /// `PostProcessManager_onShaderBinLoaded_00694c20(data, task)`.
    PostProcessManager_onShaderBinLoaded_00694c20 @ 0x0069_4c20 => extern "C" fn post_on_shader_bin_loaded(data: u32, task: u32) -> u32 {
        g4::post_on_shader_bin_loaded(&mut mem(), data, task)
    }
    /// `graphicBrokenScreenUpdatePolygonData(a, b, 9 words)`.
    graphicBrokenScreenUpdatePolygonData @ 0x0063_9500 => extern "C" fn broken_screen_update_polygon_data(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: u32, i: u32, j: u32, k: u32) -> u32 {
        g4::broken_screen_update_polygon_data(&mut mem(), &[a, b, c, d, e, f, g, h, i, j, k])
    }
    /// `ClassVFXRenderDataTable_RemoveDeletedPoolMeshInstances(this)`.
    ClassVFXRenderDataTable_RemoveDeletedPoolMeshInstances @ 0x0069_fc70 => extern "thiscall" fn vfx_remove_deleted_pool_mesh_instances(this: u32) -> u32 {
        g4::vfx_remove_deleted_pool_mesh_instances(&mut mem(), this)
    }
    /// `PhyreScene_isMeshVisible_006603c0(this, entry)`.
    PhyreScene_isMeshVisible_006603c0 @ 0x0066_03c0 => extern "thiscall" fn scene_is_mesh_visible(this: u32, entry: u32) -> u32 {
        g4::scene_is_mesh_visible(&mut mem(), this, entry)
    }
    /// `graphicUseOptimizedVFXShaderByVFXInstance(lo, hi)`.
    graphicUseOptimizedVFXShaderByVFXInstance @ 0x0064_4b10 => extern "C" fn use_optimized_vfx_shader_by_instance(lo: u32, hi: u32) -> u32 {
        g4::use_optimized_vfx_shader_by_instance(&mut mem(), lo, hi)
    }
    /// `PhyreScene_moveClusterTo7c_00658f00(this)`.
    PhyreScene_moveClusterTo7c_00658f00 @ 0x0065_8f00 => extern "thiscall" fn scene_move_cluster(this: u32) {
        g4::scene_move_cluster(&mut mem(), this);
    }
    /// `PhyreScene_clearList_00658fc0(this)`.
    PhyreScene_clearList_00658fc0 @ 0x0065_8fc0 => extern "thiscall" fn scene_clear_list_658fc0(this: u32) -> u32 {
        g4::scene_clear_list(&mut mem(), this)
    }
    /// `graphicVFXisAtelShpInstanced(lo, hi)`.
    graphicVFXisAtelShpInstanced @ 0x0064_5d40 => extern "C" fn vfx_is_atel_shp_instanced(lo: u32, hi: u32) -> u32 {
        g4::vfx_is_atel_shp_instanced(&mut mem(), lo, hi)
    }
    /// `PhyreScene_isSpecialMagic_0065f1f0(this)` (`ecx` passed on).
    PhyreScene_isSpecialMagic_0065f1f0 @ 0x0065_f1f0 => extern "thiscall" fn scene_is_special_magic(this: u32) -> u32 {
        g4::scene_is_special_magic(&mut mem(), this)
    }
    /// `PhyreScene_UseVFXOptimizedTrans(material, on)`.
    PhyreScene_UseVFXOptimizedTrans @ 0x0065_4b20 => extern "C" fn use_vfx_optimized_trans(material: u32, on: u32) -> u32 {
        g4::use_vfx_optimized_trans(&mut mem(), material, on)
    }
    /// `PhyreScene_ForceTransparent(material, on)`.
    PhyreScene_ForceTransparent @ 0x0065_1400 => extern "C" fn force_transparent(material: u32, on: u32) -> u32 {
        g4::force_transparent(&mut mem(), material, on)
    }
    /// `graphicUpdateDistortionVertex(a, b, 6 floats)`.
    graphicUpdateDistortionVertex @ 0x0064_4940 => extern "C" fn update_distortion_vertex(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: u32) -> u32 {
        g4::update_distortion_vertex(&mut mem(), a, b, [c, d, e, f, g, h])
    }
    /// `PhyreScene_renderPostAA(this, mode)`.
    PhyreScene_renderPostAA @ 0x0065_35b0 => extern "thiscall" fn scene_render_post_aa(this: u32, mode: u32) -> u32 {
        g4::scene_render_post_aa(&mut mem(), this, mode)
    }
    /// `graphicUseOptimizedVFXShader(geometry, on)`.
    graphicUseOptimizedVFXShader @ 0x0064_4ac0 => extern "C" fn use_optimized_vfx_shader(geometry: u32, on: u32) -> u32 {
        g4::use_optimized_vfx_shader(&mut mem(), geometry, on)
    }
    /// `PArray_assign_0064ead0(this, other)`.
    PArray_assign_0064ead0 @ 0x0064_ead0 => extern "thiscall" fn parray_assign_0064ead0(this: u32, other: u32) -> u32 {
        g4::parray_assign(&mut mem(), ffx_prender::glue4::ARRAY_RESERVE_667F00, this, other)
    }
    /// `PArray_assign_0064eb20(this, other)`.
    PArray_assign_0064eb20 @ 0x0064_eb20 => extern "thiscall" fn parray_assign_0064eb20(this: u32, other: u32) -> u32 {
        g4::parray_assign(&mut mem(), ffx_prender::glue4::ARRAY_RESERVE_668080, this, other)
    }
    /// `PhyreScene_showAutoSavingPrompt(this)`.
    PhyreScene_showAutoSavingPrompt @ 0x0066_f480 => extern "thiscall" fn scene_show_auto_saving_prompt(this: u32) {
        g4::scene_show_auto_saving_prompt(&mut mem(), this);
    }
    /// `graphicVFXIsTextureWithBpLoaded(7 words)`.
    graphicVFXIsTextureWithBpLoaded @ 0x0064_5450 => extern "C" fn vfx_is_texture_with_bp_loaded(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32) -> u32 {
        g4::vfx_is_texture_with_bp_loaded(&mut mem(), &[a, b, c, d, e, f, g])
    }
    /// `graphicCharacterInstanceIsVisible(list)`.
    graphicCharacterInstanceIsVisible @ 0x0063_b230 => extern "C" fn character_instance_is_visible(list: u32) -> u32 {
        g4::character_instance_is_visible(&mut mem(), list)
    }
    /// `PhyreScene_addToPipeline_00656590(this, entry)`.
    PhyreScene_addToPipeline_00656590 @ 0x0065_6590 => extern "thiscall" fn scene_add_to_pipeline(this: u32, entry: u32) {
        g4::scene_add_to_pipeline(&mut mem(), this, entry);
    }
    /// `VFXDrawEventArray_Push(this, event)`.
    VFXDrawEventArray_Push @ 0x0065_6350 => extern "thiscall" fn vfx_draw_event_push(this: u32, event: u32) -> u32 {
        g4::vfx_draw_event_push(&mut mem(), this, event)
    }
    /// `PhyreScene_addVFXDrawEvent(this, event)`.
    PhyreScene_addVFXDrawEvent @ 0x0065_6940 => extern "thiscall" fn scene_add_vfx_draw_event(this: u32, event: u32) -> u32 {
        g4::scene_add_vfx_draw_event(&mut mem(), this, event)
    }
    /// `PRedBlackTree_iteratorBegin_0064dae0(this, tree)`.
    PRedBlackTree_iteratorBegin_0064dae0 @ 0x0064_dae0 => extern "thiscall" fn tree_iterator_begin(this: u32, tree: u32) -> u32 {
        g4::tree_iterator_begin(&mut mem(), this, tree)
    }
    /// `BrokenScreenEffect_fillFloats_006d4150(this)`.
    BrokenScreenEffect_fillFloats_006d4150 @ 0x006d_4150 => extern "thiscall" fn broken_fill_floats(this: u32) -> u32 {
        g4::broken_fill_floats(&mut mem(), this)
    }
    /// `ShadowDynamicMesh_descriptorDestroy_006789c0(p)`.
    ShadowDynamicMesh_descriptorDestroy_006789c0 @ 0x0067_89c0 => extern "stdcall" fn shadow_dynamic_mesh_destroy(p: u32) -> u32 {
        g4::shadow_dynamic_mesh_destroy(&mut mem(), p)
    }
    /// `RenderDelegationHelper_setDataToFreeList(this, data)`.
    RenderDelegationHelper_setDataToFreeList @ 0x006a_e1a0 => extern "thiscall" fn set_data_to_free_list(this: u32, data: u32) -> u32 {
        g4::set_data_to_free_list(&mut mem(), this, data)
    }

    // ---- round 2, batch 5 (ffx_prender::glue5) ----
    /// `PhyreSceneCursor_Next_006377a0(this)`.
    PhyreSceneCursor_Next_006377a0 @ 0x0063_77a0 => extern "thiscall" fn cursor_next_6377a0(this: u32) -> u32 {
        g5::cursor_next(&mut mem(), this, ffx_prender::glue5::CURSOR_IS_VALID_637F80)
    }
    /// `PhyreSceneCursor_Next_006b22e0(this)`.
    PhyreSceneCursor_Next_006b22e0 @ 0x006b_22e0 => extern "thiscall" fn cursor_next_6b22e0(this: u32) -> u32 {
        g5::cursor_next(&mut mem(), this, ffx_prender::glue5::CURSOR_IS_VALID_6B28F0)
    }
    /// `PhyreSceneCursor_Begin_00637730(this)`.
    PhyreSceneCursor_Begin_00637730 @ 0x0063_7730 => extern "thiscall" fn cursor_begin_637730(this: u32) -> u32 {
        g5::cursor_begin(&mut mem(), this, ffx_prender::glue5::CURSOR_IS_VALID_637F80)
    }
    /// `PhyreSceneCursor_Begin_0065eb60(this)`.
    PhyreSceneCursor_Begin_0065eb60 @ 0x0065_eb60 => extern "thiscall" fn cursor_begin_65eb60(this: u32) -> u32 {
        g5::cursor_begin(&mut mem(), this, ffx_prender::glue5::CURSOR_IS_VALID_660270)
    }
    /// `PhyreSceneCursor_Begin_006b2230(this)`.
    PhyreSceneCursor_Begin_006b2230 @ 0x006b_2230 => extern "thiscall" fn cursor_begin_6b2230(this: u32) -> u32 {
        g5::cursor_begin(&mut mem(), this, ffx_prender::glue5::CURSOR_IS_VALID_6B28F0)
    }
    /// `PArray_pushBack20_006562f0(this, entry)`.
    PArray_pushBack20_006562f0 @ 0x0065_62f0 => extern "thiscall" fn parray_push_back20(this: u32, entry: u32) -> u32 {
        g5::parray_push_back20(&mut mem(), this, entry)
    }
    /// `PhyreScene_deleteObjectInCluster_0065a250(this, object, cluster)`.
    PhyreScene_deleteObjectInCluster_0065a250 @ 0x0065_a250 => extern "thiscall" fn scene_delete_object_in_cluster(this: u32, object: u32, cluster: u32) -> u32 {
        g5::scene_delete_object_in_cluster(&mut mem(), this, object, cluster)
    }
    /// `std_vector_push_back_00667620(this, value)`.
    std_vector_push_back_00667620 @ 0x0066_7620 => extern "thiscall" fn vector_push_back(this: u32, value: u32) {
        g5::vector_push_back(&mut mem(), this, value);
    }
    /// `graphicEnableChrClusterDelayRelease()`.
    graphicEnableChrClusterDelayRelease @ 0x0064_0090 => extern "C" fn enable_chr_cluster_delay_release() -> u32 {
        g5::enable_chr_cluster_delay_release(&mut mem())
    }
    /// `graphicVFX2DSetup(lo, hi)`.
    graphicVFX2DSetup @ 0x0064_4b50 => extern "C" fn vfx_2d_setup(lo: u32, hi: u32) -> u32 {
        g5::vfx_2d_setup(&mut mem(), lo, hi)
    }
    /// `PhyreScene_onMenuTextureLoaded(cluster)`: the argument's slot in a
    /// frame of its own.
    PhyreScene_onMenuTextureLoaded @ 0x0066_6740 => extern "C" fn scene_on_menu_texture_loaded(cluster: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g5::scene_on_menu_texture_loaded(&mut mem(), f.at(), cluster)
    }
    /// `VfxShaderManager_onShaderLoaded_006c13e0(cluster, task)`.
    VfxShaderManager_onShaderLoaded_006c13e0 @ 0x006c_13e0 => extern "C" fn vfx_shader_on_loaded(cluster: u32, task: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g5::vfx_shader_on_loaded(&mut mem(), f.at(), cluster, task)
    }
    /// `PhyreScene_setMaterialVisibilityBySetAlpha(material, visible)`.
    PhyreScene_setMaterialVisibilityBySetAlpha @ 0x0066_d5c0 => extern "stdcall" fn scene_set_material_visibility(material: u32, visible: u32) {
        let mut f = StackFrame::<4>::new();
        g5::scene_set_material_visibility(&mut mem(), f.at() + 0x10, material, visible);
    }
    /// `graphicProcessWindowsMessage()`.
    graphicProcessWindowsMessage @ 0x0064_2860 => extern "C" fn process_windows_message() -> u32 {
        let mut f = StackFrame::<8>::new();
        g5::process_windows_message(&mut mem(), f.at() + 0x1c)
    }
    /// `BrokenScreenEffect_setConstant_006d46a0(this, 4 words)`.
    BrokenScreenEffect_setConstant_006d46a0 @ 0x006d_46a0 => extern "thiscall" fn broken_set_constant_a0(this: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        g5::broken_set_constant(&mut mem(), f.at(), this, [a, b, c, d], 0xa0)
    }
    /// `BrokenScreenEffect_setConstant_006d4710(this, 4 words)`.
    BrokenScreenEffect_setConstant_006d4710 @ 0x006d_4710 => extern "thiscall" fn broken_set_constant_a4(this: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        let mut f = StackFrame::<6>::new();
        g5::broken_set_constant(&mut mem(), f.at(), this, [a, b, c, d], 0xa4)
    }
    /// `ClassVFXRenderDataTable_Remove_0069fc10(this, lo, hi)`.
    ClassVFXRenderDataTable_Remove_0069fc10 @ 0x0069_fc10 => extern "thiscall" fn vfx_table_remove(this: u32, lo: u32, hi: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        g5::vfx_table_remove(&mut mem(), f.at() + 4, this, lo, hi)
    }
    /// `ClassVFXRenderDataTable_Insert(this, lo, hi, geometry)`.
    ClassVFXRenderDataTable_Insert @ 0x0069_f890 => extern "thiscall" fn vfx_table_insert(this: u32, lo: u32, hi: u32, geometry: u32) -> u32 {
        let mut f = StackFrame::<9>::new();
        g5::vfx_table_insert(&mut mem(), f.at() + 0x10, this, lo, hi, geometry)
    }
    /// `MeshInstance_isVisible_00663cc0(this, matrix)`.
    MeshInstance_isVisible_00663cc0 @ 0x0066_3cc0 => extern "thiscall" fn mesh_instance_is_visible(this: u32, matrix: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        g5::mesh_instance_is_visible(&mut mem(), f.at() + 0x80, this, matrix)
    }

    // ---- round 2, batch 6 (ffx_prender::glue6) ----
    /// `ClassVFXRenderDataTable_CreateInstance()`.
    ClassVFXRenderDataTable_CreateInstance @ 0x0069_f4c0 => extern "C" fn create_vfx_render_data_table() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::VFX_RENDER_DATA_TABLE_SINGLETON);
    }
    /// `BrokenScreenEffect_CreateInstance_006d4d30()`.
    BrokenScreenEffect_CreateInstance_006d4d30 @ 0x006d_4d30 => extern "C" fn create_broken_screen() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::BROKEN_SCREEN_SINGLETON);
    }
    /// `FreeCamera_CreateInstance_0042f260()`.
    FreeCamera_CreateInstance_0042f260 @ 0x0042_f260 => extern "C" fn create_free_camera() -> u32 {
        g6::free_camera_instance(&mut mem())
    }
    /// `ClusterManager_CreateInstance_0069a060()`.
    ClusterManager_CreateInstance_0069a060 @ 0x0069_a060 => extern "C" fn create_cluster_manager() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::CLUSTER_MANAGER_SINGLETON);
    }
    /// `VfxShaderManager_CreateInstance_006c1420()`.
    VfxShaderManager_CreateInstance_006c1420 @ 0x006c_1420 => extern "C" fn create_vfx_shader_manager() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::VFX_SHADER_MANAGER_SINGLETON);
    }
    /// `Distortion_CreateInstance_006c2190()`.
    Distortion_CreateInstance_006c2190 @ 0x006c_2190 => extern "C" fn create_distortion() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::DISTORTION_SINGLETON);
    }
    /// `ShadowManager_CreateInstance_006c9500()`.
    ShadowManager_CreateInstance_006c9500 @ 0x006c_9500 => extern "C" fn create_shadow_manager() {
        g6::create_singleton(&mut mem(), ffx_prender::glue6::SHADOW_MANAGER_SINGLETON);
    }
    /// `VFXRenderMap_CreateInstance_0069f440()`.
    VFXRenderMap_CreateInstance_0069f440 @ 0x0069_f440 => extern "C" fn create_vfx_render_map() {
        g6::create_vfx_render_map(&mut mem());
    }
    /// `PhyreScene_onFieldMapAssetLoaded(asset, task)`.
    PhyreScene_onFieldMapAssetLoaded @ 0x0066_6630 => extern "C" fn scene_on_field_map_asset_loaded(asset: u32, task: u32) -> u32 {
        g6::scene_on_field_map_asset_loaded(&mut mem(), asset, task)
    }
    /// `PhyreScene_onVfxTextureLoaded_00666880(cluster, task)`.
    PhyreScene_onVfxTextureLoaded_00666880 @ 0x0066_6880 => extern "C" fn scene_on_vfx_texture_loaded(cluster: u32, task: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g6::scene_on_vfx_texture_loaded(&mut mem(), f.at(), cluster, task)
    }
    /// `graphicProcessPs2DataLoadingCallback()`.
    graphicProcessPs2DataLoadingCallback @ 0x0064_27f0 => extern "C" fn process_ps2_data_loading_callback() -> u32 {
        g6::process_ps2_data_loading_callback(&mut mem())
    }
    /// `PhyreScene_RenderDTTVFX(this)`.
    PhyreScene_RenderDTTVFX @ 0x0065_31b0 => extern "thiscall" fn scene_render_dtt_vfx(this: u32) -> u32 {
        g6::scene_render_dtt_vfx(&mut mem(), this)
    }
    /// `SamplerHolder_ctor_006f5ae0(this, owner)`.
    SamplerHolder_ctor_006f5ae0 @ 0x006f_5ae0 => extern "thiscall" fn sampler_holder_ctor(this: u32, owner: u32) -> u32 {
        g6::sampler_holder_ctor(&mut mem(), this, owner)
    }
    /// `PhyreScene_deletePoolObject56_0065a430(this, object, cluster)`.
    PhyreScene_deletePoolObject56_0065a430 @ 0x0065_a430 => extern "thiscall" fn scene_delete_pool_object56(this: u32, object: u32, cluster: u32) -> u32 {
        g6::scene_delete_pool_object56(&mut mem(), this, object, cluster)
    }
    /// `PhyreScene_deletePoolObject28_0065a1d0(this, object, cluster)`.
    PhyreScene_deletePoolObject28_0065a1d0 @ 0x0065_a1d0 => extern "thiscall" fn scene_delete_pool_object28(this: u32, object: u32, cluster: u32) -> u32 {
        g6::scene_delete_pool_object28(&mut mem(), this, object, cluster)
    }
    /// `graphicLevelSetName(name)`.
    graphicLevelSetName @ 0x0064_1db0 => extern "C" fn level_set_name(name: u32) -> u32 {
        g6::level_set_name(&mut mem(), name)
    }
    /// `PostProcessManager_lookup_006935f0(this, i)`.
    PostProcessManager_lookup_006935f0 @ 0x0069_35f0 => extern "thiscall" fn post_lookup(this: u32, i: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        g6::post_lookup(&mut mem(), f.at() + 0x20, this, i)
    }
    /// `PhyreScene_registClustersToAddToWorld(this, a, b)`.
    PhyreScene_registClustersToAddToWorld @ 0x0066_8550 => extern "thiscall" fn scene_regist_clusters_to_add(this: u32, a: u32, b: u32) -> u32 {
        g6::scene_regist_clusters_to_add(&mut mem(), this, a, b)
    }
    /// `PhyreScene_beginRenderOpeningScreen(this)`.
    PhyreScene_beginRenderOpeningScreen @ 0x0065_79d0 => extern "thiscall" fn scene_begin_render_opening_screen(this: u32) -> u32 {
        g6::scene_begin_render_opening_screen(&mut mem(), this)
    }
    /// `BrokenScreenEffect_setColour_006d4620(this, colour)`.
    BrokenScreenEffect_setColour_006d4620 @ 0x006d_4620 => extern "thiscall" fn broken_set_colour(this: u32, colour: u32) {
        g6::broken_set_colour(&mut mem(), this, colour);
    }
    /// `PRedBlackTree_insertNew_0065fd50(this, &key, &value)`.
    PRedBlackTree_insertNew_0065fd50 @ 0x0065_fd50 => extern "thiscall" fn tree_insert_new_0065fd50(this: u32, key: u32, value: u32) -> u32 {
        g6::tree_insert_new(&mut mem(), ffx_prender::glue6::TREE_INSERT_64C020, this, key, value)
    }
    /// `PRedBlackTree_insertNew_006c0810(this, &key, &value)`.
    PRedBlackTree_insertNew_006c0810 @ 0x006c_0810 => extern "thiscall" fn tree_insert_new_006c0810(this: u32, key: u32, value: u32) -> u32 {
        g6::tree_insert_new(&mut mem(), ffx_prender::glue6::TREE_INSERT_6BE960, this, key, value)
    }
    /// `PhyreScene_renderOpeningScreen(this)`.
    PhyreScene_renderOpeningScreen @ 0x0066_b2e0 => extern "thiscall" fn scene_render_opening_screen(this: u32) -> u32 {
        // the harness's fast mode moves the clock instead of sleeping
        // (the original's patched call at 0x0066b345)
        match crate::harness::boot_sleep() {
            Some(sleep) => g6::scene_render_opening_screen_with(&mut mem(), this, sleep),
            None => g6::scene_render_opening_screen(&mut mem(), this),
        }
    }
    /// `PSamplerStates_init_006f5470()`.
    PSamplerStates_init_006f5470 @ 0x006f_5470 => extern "C" fn sampler_states_init() -> u32 {
        g6::sampler_states_init(&mut mem())
    }

    // ---- round 3 batch 7 (ffx_prender::glue7) ----
    /// `graphicGetTime()`: a `float` in `ST0`.
    graphicGetTime @ 0x0064_1410 => extern "C" fn get_time() -> f64 {
        g7::get_time(&mut mem())
    }
    /// `graphicGetTimeInMicroseconds()`: 64 bits in `edx:eax`.
    graphicGetTimeInMicroseconds @ 0x0064_1420 => extern "C" fn get_time_in_microseconds() -> u64 {
        g7::get_time_in_microseconds(&mut mem())
    }
    /// `graphicPS2LightingSetupEffect(effect, a1, .., a10)`.
    graphicPS2LightingSetupEffect @ 0x0064_2750 => extern "C" fn ps2_lighting_setup_effect(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32) -> u32 {
        g7::ps2_lighting_setup_effect(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10])
    }
    /// `PostProcessManager_loadShaderBinFiles_00693d20(this)`.
    PostProcessManager_loadShaderBinFiles_00693d20 @ 0x0069_3d20 => extern "thiscall" fn post_load_shader_bin_files(this: u32) -> u32 {
        g7::post_load_shader_bin_files(&mut mem(), this)
    }
    /// `PhyreScene_loadCharacterAsync(this, name, id)`.
    PhyreScene_loadCharacterAsync @ 0x0066_42f0 => extern "thiscall" fn scene_load_character_async(this: u32, name: u32, id: u32) -> u32 {
        g7::scene_load_character_async(&mut mem(), this, name, id)
    }
    /// `PhyreScene_loadMagicFileVFXTextureAsync(this, id, name)`.
    PhyreScene_loadMagicFileVFXTextureAsync @ 0x0066_4d10 => extern "thiscall" fn scene_load_magic_file_vfx_texture_async(this: u32, id: u32, name: u32) -> u32 {
        g7::scene_load_magic_file_vfx_texture_async(&mut mem(), this, id, name)
    }
    /// `PhyreScene_loadVFXTextureAsync(this, id, name)`.
    PhyreScene_loadVFXTextureAsync @ 0x0066_5f60 => extern "thiscall" fn scene_load_vfx_texture_async(this: u32, id: u32, name: u32) -> u32 {
        g7::scene_load_vfx_texture_async(&mut mem(), this, id, name)
    }

    // ---- round 3 batch 8 (ffx_prender::glue8, and PArray_PushInto copies) ----
    /// `PArray_PushInto_00637e30(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_00637e30 @ 0x0063_7e30 => extern "thiscall" fn parray_push_into_00637e30(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_00637ec0(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_00637ec0 @ 0x0063_7ec0 => extern "thiscall" fn parray_push_into_00637ec0(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_0065fac0(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_0065fac0 @ 0x0065_fac0 => extern "thiscall" fn parray_push_into_0065fac0(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_0065fb50(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_0065fb50 @ 0x0065_fb50 => extern "thiscall" fn parray_push_into_0065fb50(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_006b24e0(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_006b24e0 @ 0x006b_24e0 => extern "thiscall" fn parray_push_into_006b24e0(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_006b2570(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_006b2570 @ 0x006b_2570 => extern "thiscall" fn parray_push_into_006b2570(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_006d26d0(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_006d26d0 @ 0x006d_26d0 => extern "thiscall" fn parray_push_into_006d26d0(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PArray_PushInto_006dea60(this; dst, value)`: the same code as
    /// `PArray_PushInto_00629b90`.
    PArray_PushInto_006dea60 @ 0x006d_ea60 => extern "thiscall" fn parray_push_into_006dea60(this: u32, dst: u32, value: u32) -> u32 {
        ffx_system::container::bridge::parray_push_into(&mut mem(), this, dst, value, ffx_glue_other::copies::phyre_free)
    }
    /// `PhyreScene_releaseClusterObjects_00658f40(this)`.
    PhyreScene_releaseClusterObjects_00658f40 @ 0x0065_8f40 => extern "thiscall" fn scene_release_cluster_objects(this: u32) {
        g8::scene_release_cluster_objects(&mut mem(), this);
    }
    /// `PhyreScene_isSpecialMagicId_0065efd0(id)`.
    PhyreScene_isSpecialMagicId_0065efd0 @ 0x0065_efd0 => extern "stdcall" fn is_special_magic_id(id: u32) -> u32 {
        g8::is_listed_65efd0(id)
    }
    /// `PhyreScene_releasePoolArray_0065a3a0(this, array, cluster)`.
    PhyreScene_releasePoolArray_0065a3a0 @ 0x0065_a3a0 => extern "thiscall" fn scene_release_pool_array(this: u32, array: u32, cluster: u32) {
        g8::scene_release_pool_array(&mut mem(), this, array, cluster);
    }
    /// `ClassVFXRenderDataTable_GetVFXDynamicGeometrySegmentFromPool(this, &segment, &data)`.
    ClassVFXRenderDataTable_GetVFXDynamicGeometrySegmentFromPool @ 0x0069_f7f0 => extern "thiscall" fn vfx_segment_from_pool(this: u32, out: u32, data: u32) -> u32 {
        g8::vfx_segment_from_pool(&mut mem(), this, out, data)
    }
    /// `PhyreScene_clearArrays_00659000(this)`.
    PhyreScene_clearArrays_00659000 @ 0x0065_9000 => extern "thiscall" fn scene_clear_arrays(this: u32) -> u32 {
        g8::scene_clear_arrays(&mut mem(), this)
    }
    /// `PhyreScene_updateCameras_00671be0(this)`.
    PhyreScene_updateCameras_00671be0 @ 0x0067_1be0 => extern "thiscall" fn scene_update_cameras(this: u32) {
        g8::scene_update_cameras(&mut mem(), this);
    }
    /// `PRedBlackTree_insert_006be960(this, node)`.
    PRedBlackTree_insert_006be960 @ 0x006b_e960 => extern "thiscall" fn tree_insert_6be960(this: u32, node: u32) -> u32 {
        g8::tree_find_or_link(&mut mem(), this, node)
    }
    /// `BrokenScreenEffect_render_006d44c0(this)`.
    BrokenScreenEffect_render_006d44c0 @ 0x006d_44c0 => extern "thiscall" fn broken_render(this: u32) -> u32 {
        g8::broken_render(&mut mem(), this)
    }
    /// `SamplerPair_init_006b3f40(this)`.
    SamplerPair_init_006b3f40 @ 0x006b_3f40 => extern "thiscall" fn sampler_pair_init(this: u32) -> u32 {
        g8::sampler_pair_init(&mut mem(), this)
    }
    /// `graphicCharacterRegistToInstantiate(a, chr, c, d)`: the request
    /// goes into the caller's `chr` slot; `eax` 0 for a null `chr` (its two
    /// callers in `Ch_Allocate` don't read it).
    graphicCharacterRegistToInstantiate @ 0x0063_d370 => extern "C" fn character_regist_to_instantiate(a: u32, chr: u32, c: u32, d: u32; slots: u32) -> u32 {
        g8::character_regist_to_instantiate(&mut mem(), slots, a, chr, c, d).unwrap_or(0)
    }
    /// `graphicLevelAnalysisName(path)`.
    graphicLevelAnalysisName @ 0x0064_1d00 => extern "C" fn level_analysis_name(path: u32) {
        let mut f = StackFrame::<9>::new();
        g8::level_analysis_name(&mut mem(), f.at() + 0x24, path);
    }
    /// `PhyreScene_initPostProcess_006656e0(this)`.
    PhyreScene_initPostProcess_006656e0 @ 0x0066_56e0 => extern "thiscall" fn scene_init_post_process(this: u32) -> u32 {
        g8::scene_init_post_process(&mut mem(), this)
    }
    /// `graphicVFXDestroy(lo, hi)`.
    graphicVFXDestroy @ 0x0064_5280 => extern "C" fn vfx_destroy(lo: u32, hi: u32) {
        g8::vfx_destroy(&mut mem(), lo, hi);
    }
    /// `PostProcessManager_checkShaderBin_00693540(this, i)`.
    PostProcessManager_checkShaderBin_00693540 @ 0x0069_3540 => extern "thiscall" fn post_check_shader_bin(this: u32, i: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        g8::post_check_shader_bin(&mut mem(), f.at() + 0x20, this, i)
    }
    /// `PhyreScene_loadPersistentVFXTexture(this)`.
    PhyreScene_loadPersistentVFXTexture @ 0x0066_5660 => extern "thiscall" fn scene_load_persistent_vfx_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        g8::scene_load_persistent_vfx_texture(&mut mem(), f.at() + 0x204, this)
    }
    /// `graphicCharacterInstanceSetClothVertexBuffers()`.
    graphicCharacterInstanceSetClothVertexBuffers @ 0x0063_b740 => extern "C" fn character_set_cloth_vertex_buffers() {
        let mut f = StackFrame::<2>::new();
        g8::character_set_cloth_vertex_buffers(&mut mem(), f.at() + 8);
    }

    // ---- round 3 batch 9 (ffx_prender::glue9) ----
    /// `PhyreScene_deletePMeshInstanceInCluster(this, mesh, cluster)`.
    PhyreScene_deletePMeshInstanceInCluster @ 0x0065_a4c0 => extern "thiscall" fn scene_delete_pmesh_instance(this: u32, mesh: u32, cluster: u32) {
        g9::scene_delete_pmesh_instance(&mut mem(), this, mesh, cluster);
    }
    /// `PhyreScene_doDeleteMeshInstances(this)`.
    PhyreScene_doDeleteMeshInstances @ 0x0065_a620 => extern "thiscall" fn scene_do_delete_mesh_instances(this: u32) {
        g9::scene_do_delete_mesh_instances(&mut mem(), this);
    }
    /// `PhyreScene_doOnAllFieldMapAssetsLoaded(this, a, b, list, reader)`.
    PhyreScene_doOnAllFieldMapAssetsLoaded @ 0x0065_a6c0 => extern "thiscall" fn scene_do_on_all_field_map_assets_loaded(this: u32, a: u32, _b: u32, list: u32, reader: u32) -> u32 {
        g9::scene_do_on_all_field_map_assets_loaded(&mut mem(), this, a, list, reader)
    }
    /// `ClassVFXRenderDataTable_GetVFXDynamicGeometryFromPool(this, &segment, &geometry)`.
    ClassVFXRenderDataTable_GetVFXDynamicGeometryFromPool @ 0x0069_f750 => extern "thiscall" fn vfx_geometry_from_pool(this: u32, out: u32, geometry: u32) -> u32 {
        g9::vfx_geometry_from_pool(&mut mem(), this, out, geometry)
    }
    /// `ParallelArray_erase_006b2ca0(this, value)`.
    ParallelArray_erase_006b2ca0 @ 0x006b_2ca0 => extern "thiscall" fn parallel_erase(this: u32, value: u32) -> u32 {
        g9::parallel_erase(&mut mem(), this, value)
    }
    /// `BrokenScreenEffect_setVertex_006d49f0(this, i, p0, .., p7)`.
    BrokenScreenEffect_setVertex_006d49f0 @ 0x006d_49f0 => extern "thiscall" fn broken_set_vertex(this: u32, i: u32, p0: u32, p1: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: u32) {
        g9::broken_set_vertex(&mut mem(), this, i, [p0, p1, p2, p3, p4, p5, p6, p7]);
    }
    /// `PRedBlackTree_insertArray_0065fca0(this, &key, &array)`.
    PRedBlackTree_insertArray_0065fca0 @ 0x0065_fca0 => extern "thiscall" fn tree_insert_array(this: u32, key: u32, array: u32) -> u32 {
        g9::tree_insert_array(&mut mem(), this, key, array)
    }
    /// `PhyreScene_newPoolArray_00659860(this, cluster)`.
    PhyreScene_newPoolArray_00659860 @ 0x0065_9860 => extern "thiscall" fn scene_new_pool_array(this: u32, cluster: u32) -> u32 {
        g9::scene_new_pool_array(&mut mem(), this, cluster)
    }
    /// `PhyreScene_newPoolObject56_00659910(this, cluster)`.
    PhyreScene_newPoolObject56_00659910 @ 0x0065_9910 => extern "thiscall" fn scene_new_pool_object56(this: u32, cluster: u32) -> u32 {
        g9::scene_new_pool_object56(&mut mem(), this, cluster)
    }
    /// `Distortion_ctor_006c2010(this)`.
    Distortion_ctor_006c2010 @ 0x006c_2010 => extern "thiscall" fn distortion_ctor(this: u32) -> u32 {
        g9::distortion_ctor(&mut mem(), this)
    }
    /// `PhyreScene_renderCaptureTexture(this)`.
    PhyreScene_renderCaptureTexture @ 0x0066_b130 => extern "thiscall" fn scene_render_capture_texture(this: u32) -> u32 {
        g9::scene_render_capture_texture(&mut mem(), this)
    }
    /// `graphicCheckChrClusterRelationship()`.
    graphicCheckChrClusterRelationship @ 0x0063_d9c0 => extern "C" fn check_chr_cluster_relationship() -> u32 {
        g9::check_chr_cluster_relationship(&mut mem())
    }
    /// `PhyreScene_loadMapVFXTexture(this)`.
    PhyreScene_loadMapVFXTexture @ 0x0066_4f60 => extern "thiscall" fn scene_load_map_vfx_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        g9::scene_load_map_vfx_texture(&mut mem(), f.at() + 0x204, this)
    }
    /// `PhyreScene_setVFXTextureLayout_00656ae0(id, kind, name)`: `sscanf`
    /// writes the caller's argument slots, as in the original.
    PhyreScene_setVFXTextureLayout_00656ae0 @ 0x0065_6ae0 => extern "stdcall" fn scene_set_vfx_texture_layout(id: u32, kind: u32, name: u32; slots: u32) {
        let mut f = StackFrame::<4>::new();
        g9::scene_set_vfx_texture_layout(&mut mem(), slots, f.at(), id, kind, name);
    }
    /// `PhyreScene_findNamedTexture_00660420(this, name)`.
    PhyreScene_findNamedTexture_00660420 @ 0x0066_0420 => extern "thiscall" fn scene_find_named(this: u32, name: u32) -> u32 {
        g9::scene_find_named(&mut mem(), this, name)
    }
    /// `PhyreScene_setResolutionScale_00652910(this)`.
    PhyreScene_setResolutionScale_00652910 @ 0x0065_2910 => extern "thiscall" fn scene_set_resolution_scale(this: u32) -> u32 {
        g9::scene_set_resolution_scale(&mut mem(), this)
    }
    /// `BrokenScreenEffect_renderPass_006d55b0(this, i, a, b)`.
    BrokenScreenEffect_renderPass_006d55b0 @ 0x006d_55b0 => extern "thiscall" fn broken_render_pass(this: u32, i: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g9::broken_render_pass(&mut mem(), f.at() + 0x10, this, i, a, b)
    }
    /// `graphicUpdateShadowMap(out, a1, .., a11)`: 16 bytes through `out`.
    graphicUpdateShadowMap @ 0x0064_4a00 => extern "C" fn update_shadow_map(out: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) -> u32 {
        let mut f = StackFrame::<5>::new();
        g9::update_shadow_map(&mut mem(), f.at() + 0x14, out, [a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11])
    }

    // ---- round 3 batch 10 (ffx_prender::glue10) ----
    /// `PhyreScene_loadAbConeShader(this)`.
    PhyreScene_loadAbConeShader @ 0x0066_3d40 => extern "thiscall" fn scene_load_ab_cone_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::AB_CONE)
    }
    /// `PhyreScene_loadAbWaveShader(this)`.
    PhyreScene_loadAbWaveShader @ 0x0066_3e20 => extern "thiscall" fn scene_load_ab_wave_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::AB_WAVE)
    }
    /// `PhyreScene_loadChrShaShader(this)`.
    PhyreScene_loadChrShaShader @ 0x0066_4360 => extern "thiscall" fn scene_load_chr_sha_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::CHR_SHA)
    }
    /// `PhyreScene_loadMenuNoTextureShader(this)`.
    PhyreScene_loadMenuNoTextureShader @ 0x0066_5010 => extern "thiscall" fn scene_load_menu_no_texture_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::MENU_NO_TEXTURE)
    }
    /// `PhyreScene_loadMenuShader(this)`.
    PhyreScene_loadMenuShader @ 0x0066_50f0 => extern "thiscall" fn scene_load_menu_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::MENU)
    }
    /// `PhyreScene_loadNAMenuShader(this)`.
    PhyreScene_loadNAMenuShader @ 0x0066_5200 => extern "thiscall" fn scene_load_na_menu_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::NA_MENU)
    }
    /// `PhyreScene_loadNBMenuShader(this)`.
    PhyreScene_loadNBMenuShader @ 0x0066_52e0 => extern "thiscall" fn scene_load_nb_menu_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::NB_MENU)
    }
    /// `PhyreScene_loadNTMenuZTestShader(this)`.
    PhyreScene_loadNTMenuZTestShader @ 0x0066_53c0 => extern "thiscall" fn scene_load_nt_menu_ztest_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::NT_MENU_ZTEST)
    }
    /// `PhyreScene_loadNTNBMenuShader(this)`.
    PhyreScene_loadNTNBMenuShader @ 0x0066_54a0 => extern "thiscall" fn scene_load_ntnb_menu_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::NTNB_MENU)
    }
    /// `PhyreScene_loadOptimizedFontShader(this)`.
    PhyreScene_loadOptimizedFontShader @ 0x0066_5580 => extern "thiscall" fn scene_load_optimized_font_shader(this: u32) -> u32 {
        load_shader(this, ffx_prender::glue10::OPTIMIZED_FONT)
    }
    /// `graphicCharacterUnregistLoading_00637b30(chr)`.
    graphicCharacterUnregistLoading_00637b30 @ 0x0063_7b30 => extern "C" fn character_unregist_loading(chr: u32) {
        g10::character_unregist_loading(&mut mem(), chr);
    }
    /// `graphicVFXSetup(lo, hi)`.
    graphicVFXSetup @ 0x0064_5780 => extern "C" fn vfx_setup(lo: u32, hi: u32) -> u32 {
        g10::vfx_setup(&mut mem(), lo, hi)
    }
    /// `PhyreScene_detachChrClusterFrom3DWorld(this)`.
    PhyreScene_detachChrClusterFrom3DWorld @ 0x0065_66c0 => extern "thiscall" fn scene_detach_chr_cluster(this: u32) {
        g10::scene_detach_chr_cluster(&mut mem(), this);
    }
    /// `RenderDelegationHelper_beforeVFX(this)`.
    RenderDelegationHelper_beforeVFX @ 0x006a_db80 => extern "thiscall" fn delegation_before_vfx(this: u32) {
        g10::delegation_before_vfx(&mut mem(), this);
    }
    /// `ShadowManager_resizeShadowTextures_006d2df0(this)`.
    ShadowManager_resizeShadowTextures_006d2df0 @ 0x006d_2df0 => extern "thiscall" fn shadow_resize_textures(this: u32) -> u32 {
        g10::shadow_resize_textures(&mut mem(), this)
    }
    /// `VFXDynamicGeometry_VFXDynamicGeometry(this)`.
    VFXDynamicGeometry_VFXDynamicGeometry @ 0x0069_e3e0 => extern "thiscall" fn vfx_dyn_geo_ctor(this: u32) -> u32 {
        g10::vfx_dyn_geo_ctor(&mut mem(), this)
    }
    /// `PhyreScene_unloadMapVFXTexture(this)`.
    PhyreScene_unloadMapVFXTexture @ 0x0067_0fe0 => extern "thiscall" fn scene_unload_map_vfx_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        g10::scene_unload_map_vfx_texture(&mut mem(), f.at() + 0x204, this)
    }
    /// `ClassVFXRenderDataTable_InsertToDeletePool(this, lo, hi, a, b)`:
    /// the appends read the arguments by address (a copy here).
    ClassVFXRenderDataTable_InsertToDeletePool @ 0x0069_f900 => extern "thiscall" fn vfx_insert_to_delete_pool(this: u32, lo: u32, hi: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        let args = f.at();
        let mut m = mem();
        for (k, w) in [lo, hi, a, b].into_iter().enumerate() {
            ffx_bridge::Mem::write(&mut m, args + 4 * k as u32, w);
        }
        g10::vfx_insert_to_delete_pool(&mut m, args, this, lo, hi, a, b)
    }
    /// `PArray_pushInto32_006b2730(this, dst, &value)`.
    PArray_pushInto32_006b2730 @ 0x006b_2730 => extern "thiscall" fn parray_push_into32(this: u32, dst: u32, value: u32) -> u32 {
        g10::parray_push_into_elements(&mut mem(), this, dst, value, ffx_prender::glue10::ELEMENTS_32)
    }
    /// `PArray_pushInto64_006b2800(this, dst, &value)`.
    PArray_pushInto64_006b2800 @ 0x006b_2800 => extern "thiscall" fn parray_push_into64(this: u32, dst: u32, value: u32) -> u32 {
        g10::parray_push_into_elements(&mut mem(), this, dst, value, ffx_prender::glue10::ELEMENTS_64)
    }
    /// `ClassVFXRenderDataTable_ClearVFXDrawData(this)`.
    ClassVFXRenderDataTable_ClearVFXDrawData @ 0x0069_f2b0 => extern "thiscall" fn vfx_clear_draw_data(this: u32) {
        g10::vfx_clear_draw_data(&mut mem(), this);
    }

    // ---- round 3 batch 11 (ffx_prender::glue11) ----
    /// `PRedBlackTree_insertPair_0065fdd0(this, a, b)`.
    PRedBlackTree_insertPair_0065fdd0 @ 0x0065_fdd0 => extern "thiscall" fn tree_insert_pair(this: u32, a: u32, b: u32) -> u32 {
        g11::tree_insert_pair(&mut mem(), this, a, b)
    }
    /// `RenderTextureInfo_set_006b2400(this, &array, name, path, f0, .., f4)`.
    RenderTextureInfo_set_006b2400 @ 0x006b_2400 => extern "thiscall" fn render_texture_info_set(this: u32, array: u32, name: u32, path: u32, f0: u32, f1: u32, f2: u32, f3: u32, f4: u32) -> u32 {
        g11::render_texture_info_set(&mut mem(), this, array, name, path, [f0, f1, f2, f3, f4])
    }
    /// `PhyreScene_loadCutbufferShader(this)`.
    PhyreScene_loadCutbufferShader @ 0x0066_4440 => extern "thiscall" fn scene_load_cutbuffer_shader(this: u32) -> u32 {
        let mut f = StackFrame::<0x4b>::new();
        g11::scene_load_cutbuffer_shader(&mut mem(), f.at() + 0x12c, this)
    }
    /// `PhyreScene_setMeshInstanceGroupIdVisible(this, mask)`.
    PhyreScene_setMeshInstanceGroupIdVisible @ 0x0066_d610 => extern "thiscall" fn scene_set_group_visible(this: u32, mask: u32) {
        let mut f = StackFrame::<4>::new();
        g11::scene_set_group_visible(&mut mem(), f.at() + 0x10, this, mask);
    }
    /// `graphicDynGeoMemManagerSwapBuffer(keep)`.
    graphicDynGeoMemManagerSwapBuffer @ 0x0063_ff70 => extern "C" fn dyn_geo_swap_buffer(keep: u32) -> u32 {
        g11::dyn_geo_swap_buffer(&mut mem(), keep)
    }
    /// `PhyreScene_createRWTTTarget(this)`.
    PhyreScene_createRWTTTarget @ 0x0065_9c30 => extern "thiscall" fn scene_create_rwtt_target(this: u32) -> u32 {
        g11::scene_create_rwtt_target(&mut mem(), this)
    }
    /// `PhyreScene_deleteMaterialInCluster(this, material, cluster, a, b)`.
    PhyreScene_deleteMaterialInCluster @ 0x0065_a2c0 => extern "thiscall" fn scene_delete_material(this: u32, material: u32, cluster: u32, a: u32, b: u32) {
        g11::scene_delete_material(&mut mem(), this, material, cluster, a, b);
    }
    /// `CopyNamedPairs_006991f0(dst, src, n)`: it counts in its argument
    /// slots, which callers may read.
    CopyNamedPairs_006991f0 @ 0x0069_91f0 => extern "C" fn copy_named_pairs(dst: u32, src: u32, n: u32; slots: u32) {
        g11::copy_named_pairs(&mut mem(), slots, dst, src, n);
    }
    // ---- round 4 batch 12 (ffx_prender::h3_glue12) ----
    /// `FogEffectManager_updateChrFogFade_006e3f40(this, dt)`.
    FogEffectManager_updateChrFogFade_006e3f40 @ 0x006e_3f40 => extern "thiscall" fn fog_update_chr_fade(this: u32, dt: u32) {
        g12::fog_update_chr_fade(&mut mem(), this, dt);
    }
    /// `PhyreScene_getMinimapViewportTop_0065e4a0(scene)`.
    PhyreScene_getMinimapViewportTop_0065e4a0 @ 0x0065_e4a0 => extern "thiscall" fn scene_minimap_viewport_top(this: u32) -> u32 {
        g12::scene_minimap_viewport_top(&mut mem(), this)
    }
    /// `PhyreScene_restoreDefaultRenderTargets(scene)`.
    PhyreScene_restoreDefaultRenderTargets @ 0x0066_c7f0 => extern "thiscall" fn scene_restore_default_targets(this: u32) -> u32 {
        let mut f = StackFrame::<0xb>::new();
        g12::scene_restore_default_targets(&mut mem(), f.at() + 0x2c, this)
    }
    /// `AutoTestManager_ctor_006bca90(this)`.
    AutoTestManager_ctor_006bca90 @ 0x006b_ca90 => extern "thiscall" fn auto_test_manager_ctor(this: u32) -> u32 {
        let mut f = StackFrame::<0xc>::new();
        g12::auto_test_manager_ctor(&mut mem(), f.at() + 0x30, this)
    }
    /// `PhyreScene_DrawUI2DVFX(scene)`.
    PhyreScene_DrawUI2DVFX @ 0x0065_0ea0 => extern "thiscall" fn scene_render_fade_meshes(this: u32) -> u32 {
        g12::scene_render_fade_meshes(&mut mem(), this)
    }
    /// `PhyreScene_addUniqueToArray386c_00656a00(scene, v)`.
    PhyreScene_addUniqueToArray386c_00656a00 @ 0x0065_6a00 => extern "thiscall" fn scene_add_unique(this: u32, v: u32) -> u32 {
        g12::scene_add_unique(&mut mem(), this, v)
    }
    /// `PhyreScene_unloadMagicVFXTexture(scene, id)`.
    PhyreScene_unloadMagicVFXTexture @ 0x0067_0f00 => extern "thiscall" fn scene_unload_magic_vfx_texture(this: u32, id: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        g12::scene_unload_magic_vfx_texture(&mut mem(), f.at() + 0x204, this, id)
    }
    /// `ShadowManager_InsertIntoShadowMap(this, a, b, &col, &row, name, word)`.
    ShadowManager_InsertIntoShadowMap @ 0x006c_e4f0 => extern "thiscall" fn shadow_insert_block(this: u32, a: u32, b: u32, col: u32, row: u32, name: u32, word: u32) -> u32 {
        g12::shadow_insert_block(&mut mem(), this, a, b, col, row, name, word)
    }
    /// `ShadowManager_InsertIntoShadowMap_Double(this, a, b, &col, &row, name, word)`.
    ShadowManager_InsertIntoShadowMap_Double @ 0x006c_e3e0 => extern "thiscall" fn shadow_insert_cell(this: u32, a: u32, b: u32, col: u32, row: u32, name: u32, word: u32) -> u32 {
        g12::shadow_insert_cell(&mut mem(), this, a, b, col, row, name, word)
    }
    /// `PhyreScene_resize(scene)`.
    PhyreScene_resize @ 0x0066_c580 => extern "thiscall" fn scene_resize(this: u32) -> u32 {
        g12::scene_resize(&mut mem(), this)
    }
    /// `PhyreScene_DrawUIPrerenderScene(scene)`.
    PhyreScene_DrawUIPrerenderScene @ 0x0065_0f80 => extern "thiscall" fn scene_render_fade_world(this: u32) -> u32 {
        g12::scene_render_fade_world(&mut mem(), this)
    }
    /// `Render_SurfaceFormat_0066e4f0(buffer, parameter, &record)`.
    Render_SurfaceFormat_0066e4f0 @ 0x0066_e4f0 => extern "thiscall" fn write_vector_parameter(this: u32, parameter: u32, record: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g12::write_vector_parameter(&mut mem(), f.at() + 0x10, this, parameter, record)
    }
    /// `PhyreScene_copyPreviousFrameBufferForDtt(scene)` (`al`).
    PhyreScene_copyPreviousFrameBufferForDtt @ 0x0065_93d0 => extern "thiscall" fn scene_copy_previous_frame_for_dtt(this: u32) -> u32 {
        u32::from(g12::scene_copy_previous_frame_for_dtt(&mut mem(), this))
    }
    /// `BrokenScreenEffect_setMatrix_006d4a90(this, &m)`.
    BrokenScreenEffect_setMatrix_006d4a90 @ 0x006d_4a90 => extern "thiscall" fn broken_screen_set_matrix(this: u32, m: u32) -> u32 {
        g12::broken_screen_set_matrix(&mut mem(), this, m)
    }
    /// `VfxShaderManager_ctor_006bf4a0(this)`.
    VfxShaderManager_ctor_006bf4a0 @ 0x006b_f4a0 => extern "thiscall" fn vfx_shader_manager_ctor(this: u32) -> u32 {
        g12::vfx_shader_manager_ctor(&mut mem(), this)
    }
    /// `graphicSgSetSpecular(model, index, &colour, &light, &camera, &param)`.
    graphicSgSetSpecular @ 0x0064_3cc0 => extern "C" fn sg_set_specular(model: u32, index: u32, a: u32, b: u32, c: u32, d: u32) {
        g12::sg_set_specular(&mut mem(), model, index, [a, b, c, d]);
    }
    /// `graphicCharacterInstantiateLoaded_00639db0()`.
    graphicCharacterInstantiateLoaded_00639db0 @ 0x0063_9db0 => extern "C" fn character_instantiate_loaded() {
        g12::character_instantiate_loaded(&mut mem());
    }
    /// `PostProcessManager_setMSAA(pp, samples)`.
    PostProcessManager_setMSAA @ 0x0069_7e10 => extern "thiscall" fn post_process_set_msaa(this: u32, samples: u32) -> u32 {
        g12::post_process_set_msaa(&mut mem(), this, samples)
    }
    /// `TextureAnimationManager_unregisterByName_006b1a70(this, name)`.
    TextureAnimationManager_unregisterByName_006b1a70 @ 0x006b_1a70 => extern "thiscall" fn texture_animation_unregister_by_name(this: u32, name: u32) {
        g12::texture_animation_unregister_by_name(&mut mem(), this, name);
    }
    /// `PhyreScene_addChrDrawSituation(scene, on)`.
    PhyreScene_addChrDrawSituation @ 0x0065_64a0 => extern "thiscall" fn scene_add_chr_draw_situation(this: u32, on: u32) -> u32 {
        g12::scene_add_chr_draw_situation(&mut mem(), this, on)
    }
    /// `PhyreScene_renderLoadingScreen(scene)`.
    PhyreScene_renderLoadingScreen @ 0x0066_b1e0 => extern "thiscall" fn scene_render_loading_screen(this: u32) -> u32 {
        g12::scene_render_loading_screen(&mut mem(), this)
    }
    /// `RenderDelegationHelper_init_006adc90(this)`.
    RenderDelegationHelper_init_006adc90 @ 0x006a_dc90 => extern "thiscall" fn delegation_helper_init(this: u32) -> u32 {
        g12::delegation_helper_init(&mut mem(), this)
    }
    /// `ModelBuffer_ReleaseBufferByIndex(index)`.
    ModelBuffer_ReleaseBufferByIndex @ 0x0072_c1b0 => extern "C" fn model_buffer_release(index: u32) {
        g12::model_buffer_release(&mut mem(), index);
    }

    // ---- round 4 batch 13 (ffx_prender::h3_glue13) ----
    /// `BrokenScreenEffect_ctor_006d4ba0(this)`.
    BrokenScreenEffect_ctor_006d4ba0 @ 0x006d_4ba0 => extern "thiscall" fn broken_screen_ctor(this: u32) -> u32 {
        g13::broken_screen_ctor(&mut mem(), this)
    }
    /// `graphicVFXUpdateTexture(a, b, name)`.
    graphicVFXUpdateTexture @ 0x0064_5aa0 => extern "C" fn vfx_update_texture(a: u32, b: u32, name: u32) {
        g13::vfx_update_texture(&mut mem(), a, b, name);
    }
    /// `PhyreScene_setupMapFog_0066ee70(scene)`.
    PhyreScene_setupMapFog_0066ee70 @ 0x0066_ee70 => extern "thiscall" fn scene_setup_map_fog(this: u32) -> u32 {
        g13::scene_setup_map_fog(&mut mem(), this)
    }
    /// `ClassVFXRenderDataTable_Clear(this)`.
    ClassVFXRenderDataTable_Clear @ 0x0069_f000 => extern "thiscall" fn vfx_table_clear(this: u32) {
        let mut f = StackFrame::<5>::new();
        g13::vfx_table_clear(&mut mem(), f.at() + 0x14, this);
    }
    /// `RenderDelegationHelper_unregisterDelegationOfGivenTaskAndData(this, task, data, size)`.
    RenderDelegationHelper_unregisterDelegationOfGivenTaskAndData @ 0x006a_e340 => extern "thiscall" fn delegation_unregister(this: u32, task: u32, data: u32, size: u32) -> u32 {
        g13::delegation_unregister(&mut mem(), this, task, data, size)
    }

    // ---- round 5 batch 14 (ffx_prender::h3_glue14) ----
    /// `PhyreScene_findGlobalMapObject_006518c0(scene)`.
    PhyreScene_findGlobalMapObject_006518c0 @ 0x0065_18c0 => extern "thiscall" fn scene_find_global_object(this: u32) -> u32 {
        let mut f = StackFrame::<10>::new();
        g14::scene_find_global_object(&mut mem(), f.at() + 0x28, this)
    }
    /// `PostProcessManager_resizeTargets_00697f20(this, w, h)`.
    PostProcessManager_resizeTargets_00697f20 @ 0x0069_7f20 => extern "thiscall" fn post_resize_targets(this: u32, w: u32, h: u32) -> u32 {
        g14::post_resize_targets(&mut mem(), this, w, h)
    }
    /// `PhyreScene_newPoolDynamicMesh_006594c0(this, cluster)`.
    PhyreScene_newPoolDynamicMesh_006594c0 @ 0x0065_94c0 => extern "thiscall" fn scene_new_dynamic_mesh(this: u32, cluster: u32) -> u32 {
        g14::scene_new_dynamic_mesh(&mut mem(), this, cluster)
    }
    /// `PhyreScene_newPoolMeshInstance_006595d0(this, mesh, cluster)`.
    PhyreScene_newPoolMeshInstance_006595d0 @ 0x0065_95d0 => extern "thiscall" fn scene_new_mesh_instance(this: u32, mesh: u32, cluster: u32) -> u32 {
        g14::scene_new_mesh_instance(&mut mem(), this, mesh, cluster)
    }
    /// `TextureAnimationManager_sceenTextureAnimationClear(this)`.
    TextureAnimationManager_sceenTextureAnimationClear @ 0x006b_2f50 => extern "thiscall" fn texture_animation_clear(this: u32) {
        g14::texture_animation_clear(&mut mem(), this);
    }
    /// `PhyreScene_removeMeshInstance(this, id)`: the tree search reads `id`
    /// through its argument slot.
    PhyreScene_removeMeshInstance @ 0x0066_8680 => extern "thiscall" fn scene_remove_mesh_instance(this: u32, id: u32; slots: u32) -> u32 {
        let _ = id;
        g14::scene_remove_mesh_instance(&mut mem(), this, slots)
    }
    /// `TextureAnimationManager_unregisterChararcterInstance(this, chr)`.
    TextureAnimationManager_unregisterChararcterInstance @ 0x006b_4330 => extern "thiscall" fn texture_animation_unregister(this: u32, chr: u32) -> u32 {
        g14::texture_animation_unregister(&mut mem(), this, chr)
    }
    /// `graphicCharacterInstanceDestroy(chr, id)`: the delegation search
    /// reads `chr` through its argument slot.
    graphicCharacterInstanceDestroy @ 0x0063_a030 => extern "C" fn character_instance_destroy(chr: u32, id: u32; slots: u32) -> u32 {
        g14::character_instance_destroy(&mut mem(), slots, chr, id)
    }
    /// `PhyreScene_onCharacterAssetLoadedItem_0065ae40(out, reader, array, asset, id)`.
    PhyreScene_onCharacterAssetLoadedItem_0065ae40 @ 0x0065_ae40 => extern "stdcall" fn scene_on_character_asset(out: u32, reader: u32, array: u32, asset: u32, id: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        g14::scene_on_character_asset(&mut mem(), f.at() + 0x20, slots, out, reader, array, asset, id)
    }
    /// `ShadowManager_fitShadowSize_006d23d0(w, h, out_w, out_h, n)`: it
    /// reuses its first two argument slots.
    ShadowManager_fitShadowSize_006d23d0 @ 0x006d_23d0 => extern "stdcall" fn shadow_fit_size(w: u32, h: u32, out_w: u32, out_h: u32, n: u32; slots: u32) {
        g14::shadow_fit_size(&mut mem(), slots, w, h, out_w, out_h, n);
    }
    /// `graphicCharacterLoad(name, id)`.
    graphicCharacterLoad @ 0x0063_d140 => extern "C" fn character_load(name: u32, id: u32) -> u32 {
        let mut f = StackFrame::<0x42>::new();
        g14::character_load(&mut mem(), f.at() + 0x108, name, id)
    }
    /// `VFXDynamicGeometry_SetupWithShader(this, shader, cluster)`.
    VFXDynamicGeometry_SetupWithShader @ 0x006a_46e0 => extern "thiscall" fn geometry_set_shader(this: u32, shader: u32, cluster: u32) -> u32 {
        let mut f = StackFrame::<0x79>::new();
        g14::geometry_set_shader(&mut mem(), f.at() + 0x1e4, this, shader, cluster)
    }

    // ---- round 5 batch 15 (ffx_prender::h3_glue15) ----
    /// `RenderDelegationHelper_registerDelegation(this, data, size, task, arg)`.
    RenderDelegationHelper_registerDelegation @ 0x006a_de20 => extern "thiscall" fn delegation_register(this: u32, data: u32, size: u32, task: u32, arg: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        g15::delegation_register(&mut mem(), f.at() + 4, this, data, size, task, arg)
    }
    /// `graphicGuildmapCreate(a, b, arg)`.
    graphicGuildmapCreate @ 0x0064_1510 => extern "C" fn guildmap_create(a: u32, b: u32, arg: u32) -> u32 {
        g15::guildmap_create(&mut mem(), a, b, arg)
    }
    /// `graphicCharacterInstanceDestroyClothVertexBuffers(cloth)`: its loop
    /// counter lives in the argument's slot.
    graphicCharacterInstanceDestroyClothVertexBuffers @ 0x0063_a160 => extern "C" fn cloth_vertex_buffers_destroy(cloth: u32; slots: u32) -> u32 {
        g15::cloth_vertex_buffers_destroy(&mut mem(), slots, cloth)
    }
    /// `PhyreScene_createMaterial_00659710(this, variant, cluster, a, b)`.
    PhyreScene_createMaterial_00659710 @ 0x0065_9710 => extern "thiscall" fn scene_create_material(this: u32, variant: u32, cluster: u32, a: u32, b: u32) -> u32 {
        g15::scene_create_material(&mut mem(), this, variant, cluster, a, b)
    }
    /// `PhyreScene_setEnableRenderWorldToTexture(this, on, m, f0..f7, n)`.
    PhyreScene_setEnableRenderWorldToTexture @ 0x0066_d000 => extern "thiscall" fn scene_set_render_world_to_texture(this: u32, on: u32, m: u32, f0: u32, f1: u32, f2: u32, f3: u32, f4: u32, f5: u32, f6: u32, f7: u32, n: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        let floats = [f0, f1, f2, f3, f4, f5, f6, f7];
        g15::scene_set_render_world_to_texture(&mut mem(), f.at() + 0x40, this, on, m, floats, n)
    }
    /// `graphicFieldMapLoad(path, flag)`.
    graphicFieldMapLoad @ 0x0064_0210 => extern "C" fn field_map_load(path: u32, flag: u32) -> u32 {
        g15::field_map_load(&mut mem(), path, flag)
    }
    /// `PhyreScene_drawMainMenuWaveEffect()`.
    PhyreScene_drawMainMenuWaveEffect @ 0x0065_b5b0 => extern "C" fn scene_draw_main_menu_wave() -> u32 {
        let mut f = StackFrame::<0xa7>::new();
        g15::scene_draw_main_menu_wave(&mut mem(), f.at() + 0x29c)
    }

    // ---- round 5 batch 16 (ffx_prender::h3_glue16) ----
    /// `PhyreScene_removeVFXFromRenderArray(this, mesh)`.
    PhyreScene_removeVFXFromRenderArray @ 0x0066_87a0 => extern "thiscall" fn scene_remove_vfx_from_render_array(this: u32, mesh: u32) -> u32 {
        g16::scene_remove_vfx_from_render_array(&mut mem(), this, mesh)
    }
    /// `PhyreScene_fieldMapShutdown(this)`.
    PhyreScene_fieldMapShutdown @ 0x0065_d0c0 => extern "thiscall" fn scene_field_map_shutdown(this: u32) -> u32 {
        g16::scene_field_map_shutdown(&mut mem(), this)
    }
    /// `PhyreScene_textureVideoUnload(this)`.
    PhyreScene_textureVideoUnload @ 0x0067_0760 => extern "thiscall" fn scene_texture_video_unload(this: u32) -> u32 {
        g16::scene_texture_video_unload(&mut mem(), this)
    }
    /// `PhyreScene_setupSamplers_0065f290(this)`. Hooked again (r6): the
    /// few UI pixels r5 saw came from pre-lockstep state, which the
    /// original shows too (pixel-regress, 2026-10-26).
    PhyreScene_setupSamplers_0065f290 @ 0x0065_f290 => extern "thiscall" fn scene_setup_samplers(this: u32) -> u32 {
        g16::scene_setup_samplers(&mut mem(), this)
    }
    /// `PhyreScene_setupPostProcessParams(this)`.
    PhyreScene_setupPostProcessParams @ 0x0066_f140 => extern "thiscall" fn scene_setup_post_process_params(this: u32) -> u32 {
        g16::scene_setup_post_process_params(&mut mem(), this)
    }
    /// `LayoutDesc_setup_00678bd0(this, a, b)`.
    LayoutDesc_setup_00678bd0 @ 0x0067_8bd0 => extern "thiscall" fn layout_setup(this: u32, a: u32, b: u32) -> u32 {
        g16::layout_setup(&mut mem(), this, a, b)
    }

    // ---- round 5 batch 17 (ffx_prender::h3_glue17) ----
    /// `PhyreScene_UpdateTextureVideo(this)`.
    PhyreScene_UpdateTextureVideo @ 0x0065_4960 => extern "thiscall" fn scene_update_texture_video(this: u32) {
        g17::scene_update_texture_video(&mut mem(), this);
    }
    /// `graphicInitialize()`.
    graphicInitialize @ 0x0064_19d0 => extern "C" fn graphic_initialize() -> u32 {
        g17::graphic_initialize(&mut mem())
    }
    /// `PostProcessManager_setNon16By9Viewport(renderer, keep)`: it reuses
    /// `keep`'s argument slot.
    PostProcessManager_setNon16By9Viewport @ 0x0069_8750 => extern "stdcall" fn post_set_non_16_by_9_viewport(renderer: u32, keep: u32; slots: u32) -> u32 {
        g17::post_set_non_16_by_9_viewport(&mut mem(), slots, renderer, keep)
    }
    /// `PhyreScene_setupCharacterLighting_0066eb20(this, flag)`.
    PhyreScene_setupCharacterLighting_0066eb20 @ 0x0066_eb20 => extern "thiscall" fn scene_setup_character_lighting(this: u32, flag: u32) {
        g17::scene_setup_character_lighting(&mut mem(), this, flag);
    }
    /// `PhyreScene_loadMagicVFXTexture(this, id)`.
    PhyreScene_loadMagicVFXTexture @ 0x0066_4d80 => extern "thiscall" fn scene_load_magic_vfx_texture(this: u32, id: u32) -> u32 {
        let mut f = StackFrame::<0x82>::new();
        g17::scene_load_magic_vfx_texture(&mut mem(), f.at() + 0x208, this, id)
    }

    // ---- round 5 batch 18 (ffx_prender::h3_glue18) ----
    /// `PhyreScene_preRender(this)`.
    PhyreScene_preRender @ 0x0065_1be0 => extern "thiscall" fn scene_pre_render(this: u32) -> u32 {
        let mut f = StackFrame::<0x13>::new();
        g18::scene_pre_render(&mut mem(), f.at() + 0x4c, this)
    }
    /// `ShadowManager_createshadowbuffer(this, a, b)`: the two words are
    /// unused.
    ShadowManager_createshadowbuffer @ 0x006d_18b0 => extern "thiscall" fn shadow_create_buffers(this: u32, a: u32, b: u32) -> u32 {
        let _ = (a, b);
        g18::shadow_create_buffers(&mut mem(), this)
    }

    // ---- round 6 batch 19 (ffx_prender::h3_glue19) ----
    /// `BrokenScreenEffect_fillIndices_006d41b0(this)` (`ecx` only).
    BrokenScreenEffect_fillIndices_006d41b0 @ 0x006d_41b0 => extern "thiscall" fn broken_fill_indices(this: u32) -> u32 {
        g19::broken_fill_indices(&mut mem(), this)
    }
    /// `BrokenScreenEffect_setColorBuffer_006d4540(this, texture, sampler)`.
    BrokenScreenEffect_setColorBuffer_006d4540 @ 0x006d_4540 => extern "thiscall" fn broken_set_color_buffer(this: u32, texture: u32, sampler: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        g19::broken_set_color_buffer(&mut mem(), f.at() + 0x10, this, texture, sampler)
    }
    /// `ShadowManager_updateCascades_006ce860(this, flag, a, b, map)`: the
    /// battle byte goes over `map`'s slot.
    ShadowManager_updateCascades_006ce860 @ 0x006c_e860 => extern "thiscall" fn shadow_update_cascades(this: u32, flag: u32, a: u32, b: u32, map: u32; slots: u32) -> u32 {
        g19::shadow_update_cascades(&mut mem(), slots, this, flag, a, b, map)
    }
    /// `ClassVFXRenderDataTable_ClassVFXRenderDataTable(this)`.
    ClassVFXRenderDataTable_ClassVFXRenderDataTable @ 0x0069_e140 => extern "thiscall" fn vfx_table_ctor(this: u32) -> u32 {
        g19::vfx_table_ctor(&mut mem(), this)
    }
    /// `PhyreScene_DrawMostVFX(this)` (`ecx` only).
    PhyreScene_DrawMostVFX @ 0x0065_0670 => extern "thiscall" fn scene_draw_most_vfx(this: u32) -> u32 {
        g19::scene_draw_most_vfx(&mut mem(), this)
    }
    /// `PhyreScene_findGlobalCamera(this)` (`ecx` only).
    PhyreScene_findGlobalCamera @ 0x0065_19f0 => extern "thiscall" fn scene_find_global_camera(this: u32) -> u32 {
        let mut f = StackFrame::<0xa>::new();
        g19::scene_find_global_camera(&mut mem(), f.at() + 0x28, this)
    }
    /// `graphicCharacterUnloadTask(chr)`: without a character the original
    /// leaves the caller's `eax`, which the thunk keeps.
    graphicCharacterUnloadTask @ 0x0063_7980 => extern "C" fn character_unload_task(chr: u32) {
        let mut f = StackFrame::<3>::new();
        let _ = g19::character_unload_task(&mut mem(), f.at() + 0xc, chr);
    }

    // ---- round 6 batch 20 (ffx_prender::h3_glue20) ----
    /// `DrawCrossMenuScrollParts(x, y, w, h, first, shown, total)`: it
    /// stores floats over the `y` and `total` slots.
    DrawCrossMenuScrollParts @ 0x008e_6cc0 => extern "C" fn draw_cross_menu_scroll_parts(x: u32, y: u32, w: u32, ht: u32, first: u32, shown: u32, total: u32; slots: u32) {
        g20::draw_cross_menu_scroll_parts(&mut mem(), slots, x, y, w, ht, first.cast_signed(), shown.cast_signed(), total.cast_signed());
    }
    /// `graphicCharacterInstanceSetMeshAlpha(chr, _, alpha)`.
    graphicCharacterInstanceSetMeshAlpha @ 0x0063_b810 => extern "C" fn character_set_mesh_alpha(chr: u32, unused: u32, alpha: u32) {
        let _ = unused;
        let mut f = StackFrame::<9>::new();
        g20::character_set_mesh_alpha(&mut mem(), f.at() + 0x24, chr, alpha);
    }
    /// `PhyreScene_updateMapYIQ(this)` (`ecx` only).
    PhyreScene_updateMapYIQ @ 0x0067_1da0 => extern "thiscall" fn scene_update_map_yiq(this: u32) {
        let mut f = StackFrame::<0xf>::new();
        g20::scene_update_map_yiq(&mut mem(), f.at() + 0x3c, this);
    }
}

/// The scene's shader-bin loaders, with their frame (0x12c bytes below
/// `ebp`).
fn load_shader(this: u32, bin: ffx_prender::glue10::ShaderBin) -> u32 {
    let mut f = StackFrame::<0x4b>::new();
    g10::scene_load_shader(&mut mem(), f.at() + 0x12c, this, bin)
}
