//! `ffx_prender::glue`: the game's render-side glue (M4 port round 1,
//! list `h3-glue-render`). One-line forwarders return their callee's `eax`
//! (the original leaves it to its callers).
//!
//! Threads: the main thread, and the loader's `asyncFFXInitialize` job;
//! the shims keep no state.

use super::{mem, ported};
use ffx_prender::glue::bridge as g;

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
}
