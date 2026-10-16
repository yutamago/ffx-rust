//! `ffx-graphic`: the remaster's render layer over `PhyreEngine`. Functions
//! whose original leaves a result in `eax` return it; those leaving a byte
//! in `al` over the caller's `eax` return `u8`.

use super::{mem, ported};
use ffx_bridge::Global;
use ffx_graphic::{cache, managers, scene, ui, vfx};

use managers::bridge as mg;
use scene::bridge as sc;

ported! {
    // ---- the effects' render data ----
    /// `ClassVFXRenderDataTable_MapFind(this, out, key_ptr)`.
    ClassVFXRenderDataTable_MapFind @ 0x006a_1f60 => extern "thiscall" fn vfx_table_map_find(this: u32, out: u32, key: u32) -> u32 {
        vfx::bridge::map_find_out(&mut mem(), this, out, key)
    }
    /// `ClassVFXRenderDataTable_GetDynamicGeometryByInstance(this, lo, hi)`.
    ClassVFXRenderDataTable_GetDynamicGeometryByInstance @ 0x0069_f620 => extern "thiscall" fn vfx_table_get_dynamic_geometry(this: u32, lo: u32, hi: u32) -> u32 {
        vfx::bridge::get_dynamic_geometry(&mut mem(), this, lo, hi)
    }
    /// `ClassVFXRenderDataTable_GetVFXDataByInstance(this, lo, hi)`.
    ClassVFXRenderDataTable_GetVFXDataByInstance @ 0x0069_f6e0 => extern "thiscall" fn vfx_table_get_vfx_data(this: u32, lo: u32, hi: u32) -> u32 {
        vfx::bridge::get_vfx_data(&mut mem(), this, lo, hi)
    }
    /// `graphicVFXGetRenderDataByInstance(lo, hi)`.
    graphicVFXGetRenderDataByInstance @ 0x0064_53d0 => extern "C" fn graphic_vfx_get_render_data_by_instance(lo: u32, hi: u32) -> u32 {
        vfx::bridge::get_render_data_by_instance(&mut mem(), lo, hi)
    }
    /// `graphicDynamicGeometryUpdateTexture(lo, hi)`.
    graphicDynamicGeometryUpdateTexture @ 0x0064_0060 => extern "C" fn graphic_dynamic_geometry_update_texture(lo: u32, hi: u32) -> u32 {
        vfx::bridge::dynamic_geometry_update_texture(&mut mem(), lo, hi)
    }
    /// `graphicVFXUpdate(lo, hi)`.
    graphicVFXUpdate @ 0x0064_58a0 => extern "C" fn graphic_vfx_update(lo: u32, hi: u32) {
        vfx::bridge::update(&mut mem(), lo, hi);
    }
    /// `graphicVFXUpdateWithType(lo, hi, flags)`.
    graphicVFXUpdateWithType @ 0x0064_5ba0 => extern "C" fn graphic_vfx_update_with_type(lo: u32, hi: u32, flags: u32) {
        vfx::bridge::update_with_type(&mut mem(), lo, hi, flags);
    }
    /// `graphicVFXPath3Update(lo, hi)`.
    graphicVFXPath3Update @ 0x0064_5600 => extern "C" fn graphic_vfx_path3_update(lo: u32, hi: u32) {
        vfx::bridge::path3_update(&mut mem(), lo, hi);
    }
    /// `graphicVFXIsTextureLoaded(a, name, c, d, e, f)`.
    graphicVFXIsTextureLoaded @ 0x0064_5400 => extern "C" fn graphic_vfx_is_texture_loaded(a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        vfx::bridge::is_texture_loaded(&mut mem(), [a, b, c, d, e, f])
    }
    /// `InitVFXRenderData(lo, hi, part, obj, e)`.
    InitVFXRenderData @ 0x0071_1180 => extern "C" fn init_vfx_render_data(lo: u32, hi: u32, part: u32, obj: u32, e: u32) {
        vfx::bridge::init_render_data(&mut mem(), [lo, hi, part, obj, e], false);
    }
    /// `InitVFXPath3RenderData(lo, hi, part, obj, e)`.
    InitVFXPath3RenderData @ 0x0071_0fd0 => extern "C" fn init_vfx_path3_render_data(lo: u32, hi: u32, part: u32, obj: u32, e: u32) {
        vfx::bridge::init_render_data(&mut mem(), [lo, hi, part, obj, e], true);
    }
    /// `InitVFXRenderDataShp(lo, hi, part, obj, shape, f)`.
    InitVFXRenderDataShp @ 0x0071_1460 => extern "C" fn init_vfx_render_data_shp(lo: u32, hi: u32, part: u32, obj: u32, shape: u32, f: u32) {
        vfx::bridge::init_render_data_shp(&mut mem(), [lo, hi, part, obj, shape, f], false);
    }
    /// `InitVFXPath3RenderDataShp(lo, hi, part, obj, shape, f)`.
    InitVFXPath3RenderDataShp @ 0x0071_1030 => extern "C" fn init_vfx_path3_render_data_shp(lo: u32, hi: u32, part: u32, obj: u32, shape: u32, f: u32) {
        vfx::bridge::init_render_data_shp(&mut mem(), [lo, hi, part, obj, shape, f], true);
    }

    /// `updateVFXRenderData(lo, hi, part, obj)`.
    updateVFXRenderData @ 0x0071_8610 => extern "C" fn update_vfx_render_data(lo: u32, hi: u32, part: u32, obj: u32) -> u32 {
        ffx_graphic::render_data::bridge::update(&mut mem(), lo, hi, part, obj)
    }
    /// `updateVFXRenderDataShp(lo, hi, part, obj, segment)`.
    updateVFXRenderDataShp @ 0x0071_b8a0 => extern "C" fn update_vfx_render_data_shp(lo: u32, hi: u32, part: u32, obj: u32, segment: u32) -> u32 {
        ffx_graphic::render_data::bridge::update_shp(&mut mem(), lo, hi, part, obj, segment)
    }
    /// `graphicVFX2DUpdate(lo, hi)`.
    graphicVFX2DUpdate @ 0x0064_4bc0 => extern "C" fn graphic_vfx_2d_update(lo: u32, hi: u32) {
        vfx::bridge::update_2d(&mut mem(), lo, hi);
    }
    /// `graphicVFXDttUpdate(lo, hi)`.
    graphicVFXDttUpdate @ 0x0064_5330 => extern "C" fn graphic_vfx_dtt_update(lo: u32, hi: u32) {
        vfx::bridge::update_dtt(&mut mem(), lo, hi);
    }
    /// `graphicVFXGetDynamicGeometryDataByInstance(lo, hi)`.
    graphicVFXGetDynamicGeometryDataByInstance @ 0x0064_5380 => extern "C" fn graphic_vfx_get_dynamic_geometry(lo: u32, hi: u32) -> u32 {
        vfx::bridge::get_dynamic_geometry_by_instance(&mut mem(), lo, hi)
    }
    /// `graphicVFXSetVisible(lo, hi, visible)`.
    graphicVFXSetVisible @ 0x0064_5710 => extern "C" fn graphic_vfx_set_visible(lo: u32, hi: u32, visible: u32) -> u32 {
        vfx::bridge::set_visible(&mut mem(), lo, hi, visible)
    }
    /// `graphicVFXEnableYIQ(lo, hi)`.
    graphicVFXEnableYIQ @ 0x0064_5350 => extern "C" fn graphic_vfx_enable_yiq(lo: u32, hi: u32) -> u32 {
        vfx::bridge::enable_yiq(&mut mem(), lo, hi)
    }
    /// `graphicVFXSetPointLightParameters(lo, hi, matrix, color, power)`.
    graphicVFXSetPointLightParameters @ 0x0064_56d0 => extern "C" fn graphic_vfx_set_point_light(lo: u32, hi: u32, a: u32, b: u32, f: f32) -> u32 {
        vfx::bridge::set_parameters(&mut mem(), vfx::bridge::SET_POINT_LIGHT, lo, hi, a, b, f)
    }
    /// `graphicVFXSetClutParameters(lo, hi, a, b, f)`.
    graphicVFXSetClutParameters @ 0x0064_5630 => extern "C" fn graphic_vfx_set_clut(lo: u32, hi: u32, a: u32, b: u32, f: f32) -> u32 {
        vfx::bridge::set_parameters(&mut mem(), vfx::bridge::SET_CLUT, lo, hi, a, b, f)
    }
    /// `graphicVFXSetZCorrectionValue(lo, hi, f)`.
    graphicVFXSetZCorrectionValue @ 0x0064_5750 => extern "C" fn graphic_vfx_set_z_correction(lo: u32, hi: u32, f: f32) -> u32 {
        vfx::bridge::set_z_correction(&mut mem(), lo, hi, f)
    }
    /// `graphicVFXSetMeshInstanceName(lo, hi, name)`.
    graphicVFXSetMeshInstanceName @ 0x0064_56a0 => extern "C" fn graphic_vfx_set_mesh_instance_name(lo: u32, hi: u32, name: u32) -> u32 {
        vfx::bridge::set_mesh_instance_name(&mut mem(), lo, hi, name)
    }
    /// `graphicVFXUpdateParameterOnly(g, z, uv, rgba)`.
    graphicVFXUpdateParameterOnly @ 0x0064_5a50 => extern "C" fn graphic_vfx_update_parameter_only(g: u32, z: f32, uv: u32, rgba: u32) {
        vfx::bridge::update_parameter_only(&mut mem(), g, z, uv, rgba);
    }
    /// `graphicVFXSyncPath()`.
    graphicVFXSyncPath @ 0x0064_5840 => extern "C" fn graphic_vfx_sync_path() {
        vfx::bridge::sync_path(&mut mem());
    }
    /// `graphicVFXDestroyAll()`.
    graphicVFXDestroyAll @ 0x0064_5310 => extern "C" fn graphic_vfx_destroy_all() -> u32 {
        vfx::bridge::table_call(&mut mem(), vfx::bridge::TABLE_CLEAR)
    }
    /// `graphicVFXDestroyAllExceptFieldAndEternal()`.
    graphicVFXDestroyAllExceptFieldAndEternal @ 0x0064_5320 => extern "C" fn graphic_vfx_destroy_all_except() -> u32 {
        vfx::bridge::destroy_all_except_field_and_eternal(&mut mem())
    }
    /// `graphicVFXRemoveDeletedPoolMeshInstances()`.
    graphicVFXRemoveDeletedPoolMeshInstances @ 0x0064_5620 => extern "C" fn graphic_vfx_remove_deleted() -> u32 {
        vfx::bridge::table_call(&mut mem(), vfx::bridge::TABLE_REMOVE_DELETED)
    }
    /// `graphicVFXUpdateDeletedPool()`.
    graphicVFXUpdateDeletedPool @ 0x0064_5a40 => extern "C" fn graphic_vfx_update_deleted_pool() -> u32 {
        vfx::bridge::table_call(&mut mem(), vfx::bridge::TABLE_UPDATE_DELETE_POOL)
    }
    /// `graphicVFXLoadPersistentTexture()`.
    graphicVFXLoadPersistentTexture @ 0x0064_54c0 => extern "C" fn graphic_vfx_load_persistent_texture() -> u32 {
        vfx::bridge::load_persistent_texture(&mut mem())
    }
    /// `graphicVFXLoadLevelData()`.
    graphicVFXLoadLevelData @ 0x0064_54a0 => extern "C" fn graphic_vfx_load_level_data() -> u32 {
        vfx::bridge::load_level_data(&mut mem())
    }
    /// `graphicVFXUnloadLevelData()`.
    graphicVFXUnloadLevelData @ 0x0064_5860 => extern "C" fn graphic_vfx_unload_level_data() -> u32 {
        vfx::bridge::unload_level_data(&mut mem())
    }

    // ---- PhyreScene ----
    /// `graphicLevelGetNameHash()`.
    graphicLevelGetNameHash @ 0x0064_1da0 => extern "C" fn graphic_level_get_name_hash() -> u32 {
        sc::level_get_name_hash(&mem())
    }
    /// `PhyreScene_getCurrentLevelNameHash(this)`.
    PhyreScene_getCurrentLevelNameHash @ 0x0065_de90 => extern "thiscall" fn scene_get_current_level_name_hash(this: u32) -> u32 {
        sc::current_level_name_hash(&mem(), this)
    }
    /// `PhyreScene_getPreviousLevelNameHash(this)`.
    PhyreScene_getPreviousLevelNameHash @ 0x0065_e310 => extern "thiscall" fn scene_get_previous_level_name_hash(this: u32) -> u32 {
        sc::previous_level_name_hash(&mem(), this)
    }
    /// `PhyreScene_setCurrentLevelNameHash(this, hash)`.
    PhyreScene_setCurrentLevelNameHash @ 0x0066_cf90 => extern "thiscall" fn scene_set_current_level_name_hash(this: u32, hash: u32) -> u32 {
        sc::set_current_level_name_hash(&mut mem(), this, hash)
    }
    /// `PhyreScene_getCurrentLevelName(this)`.
    PhyreScene_getCurrentLevelName @ 0x0065_de70 => extern "thiscall" fn scene_get_current_level_name(this: u32) -> u32 {
        sc::current_level_name(&mem(), this)
    }
    /// `PhyreScene_getCurrentMagicPath(this)`.
    PhyreScene_getCurrentMagicPath @ 0x0065_dea0 => extern "thiscall" fn scene_get_current_magic_path(this: u32) -> u32 {
        sc::current_magic_path(&mem(), this)
    }
    /// `PhyreScene_getToBeDeleteMagicPath(this)`.
    PhyreScene_getToBeDeleteMagicPath @ 0x0065_e5e0 => extern "thiscall" fn scene_get_to_be_delete_magic_path(this: u32) -> u32 {
        sc::to_be_delete_magic_path(&mem(), this)
    }
    /// `PhyreScene_GetSplitScreenStat(this)`.
    PhyreScene_GetSplitScreenStat @ 0x0065_17d0 => extern "thiscall" fn scene_get_split_screen_stat(this: u32) -> u32 {
        sc::field(&mem(), this, 0x10d8c)
    }
    /// `PhyreScene_SetSplitScreenStat(this, v)`.
    PhyreScene_SetSplitScreenStat @ 0x0065_4550 => extern "thiscall" fn scene_set_split_screen_stat(this: u32, v: u32) -> u32 {
        sc::set_if_changed(&mut mem(), this, 0x10d8c, v)
    }
    /// `PhyreScene_SetSplitScreenType(this, v)`.
    PhyreScene_SetSplitScreenType @ 0x0065_4570 => extern "thiscall" fn scene_set_split_screen_type(this: u32, v: u32) -> u32 {
        sc::set_if_changed(&mut mem(), this, 0x10d90, v)
    }
    /// `PhyreScene_getLinearSampler(this, which)`.
    PhyreScene_getLinearSampler @ 0x0065_e050 => extern "thiscall" fn scene_get_linear_sampler(this: u32, which: u32) -> u32 {
        sc::linear_sampler(this, which)
    }
    /// `PhyreScene_getVideoTexture(this)`.
    PhyreScene_getVideoTexture @ 0x0065_e640 => extern "thiscall" fn scene_get_video_texture(this: u32) -> u32 {
        sc::field(&mem(), this, 0xde0)
    }
    /// `PhyreScene_GetField0ddc(this)`.
    PhyreScene_GetField0ddc @ 0x0065_e630 => extern "thiscall" fn scene_get_field_ddc(this: u32) -> u32 {
        sc::field(&mem(), this, 0xddc)
    }
    /// `PhyreScene_GetField0dd8(this)`.
    PhyreScene_GetField0dd8 @ 0x0065_e650 => extern "thiscall" fn scene_get_field_dd8(this: u32) -> u32 {
        sc::field(&mem(), this, 0xdd8)
    }
    /// `PhyreScene_GetLivePointer00ac(this)`.
    PhyreScene_GetLivePointer00ac @ 0x0065_ded0 => extern "thiscall" fn scene_get_live_pointer_ac(this: u32) -> u32 {
        sc::live_pointer(&mem(), this, 0xac)
    }
    /// `PhyreScene_GetLivePointer1232c(this)`.
    PhyreScene_GetLivePointer1232c @ 0x0065_e2f0 => extern "thiscall" fn scene_get_live_pointer_1232c(this: u32) -> u32 {
        sc::live_pointer(&mem(), this, 0x1232c)
    }
    /// `PhyreScene_ForceSetShadowSolution(this, v)`.
    PhyreScene_ForceSetShadowSolution @ 0x0065_13f0 => extern "thiscall" fn scene_force_set_shadow_solution(this: u32, v: u32) -> u32 {
        sc::set_field(&mut mem(), this, 0xf9cc, v)
    }
    /// `PhyreScene_textureVideoPlay(this, play)`.
    PhyreScene_textureVideoPlay @ 0x0067_0750 => extern "thiscall" fn scene_texture_video_play(this: u32, play: u32) -> u8 {
        sc::set_byte_field(&mut mem(), this, 0x10c9, play)
    }
    /// `PhyreScene_isAnyBattleMap(this)`.
    PhyreScene_isAnyBattleMap @ 0x0066_0330 => extern "thiscall" fn scene_is_any_battle_map(this: u32) -> u8 {
        sc::scene_is_any_battle_map(&mem(), this)
    }
    /// `PhyreScene_isFading(this)`.
    PhyreScene_isFading @ 0x0065_23e0 => extern "thiscall" fn scene_is_fading(this: u32) -> u8 {
        sc::scene_is_fading(&mem(), this)
    }
    /// `PhyreScene_isDrawingNonUI2DScene(this)`.
    PhyreScene_isDrawingNonUI2DScene @ 0x0066_0350 => extern "thiscall" fn scene_is_drawing_non_ui_2d_scene(this: u32) -> u32 {
        sc::is_drawing_non_ui_2d_scene(&mem(), this)
    }
    /// `PhyreScene_is3DScene()`.
    PhyreScene_is3DScene @ 0x0066_01d0 => extern "C" fn scene_is_3d_scene() -> u32 {
        u32::from(sc::IS_3D.get(&mem()))
    }
    /// `PhyreScene_isOpeningScreenPlaying()`.
    PhyreScene_isOpeningScreenPlaying @ 0x0066_0400 => extern "C" fn scene_is_opening_screen_playing() -> u8 {
        sc::OPENING_PLAYING.get(&mem())
    }
    /// `PhyreScene_setMeshVisible(mesh, visible)`.
    PhyreScene_setMeshVisible @ 0x0066_d910 => extern "stdcall" fn scene_set_mesh_visible(mesh: u32, visible: u32) {
        sc::set_mesh_visible(&mut mem(), mesh, visible);
    }

    // ---- graphic* over the scene ----
    /// `graphicGetMiniMapType()`.
    graphicGetMiniMapType @ 0x0064_1320 => extern "C" fn graphic_get_mini_map_type() -> u32 {
        sc::scene_u32(&mem(), 0x74)
    }
    /// `graphicSetMiniMapType(v)`.
    graphicSetMiniMapType @ 0x0064_3720 => extern "C" fn graphic_set_mini_map_type(v: u32) -> u32 {
        sc::set_scene_u32(&mut mem(), 0x74, v)
    }
    /// `graphicIsInLucaTheater()`.
    graphicIsInLucaTheater @ 0x0064_1c10 => extern "C" fn graphic_is_in_luca_theater() -> u32 {
        sc::scene_byte(&mem(), 0x10d2e)
    }
    /// `graphicIsFieldMapLoading()`.
    graphicIsFieldMapLoading @ 0x0064_1bf0 => extern "C" fn graphic_is_field_map_loading() -> u32 {
        sc::scene_byte(&mem(), 0x1195)
    }
    /// `graphicIsBattleMap()`: the previous battle-map flag.
    graphicIsBattleMap @ 0x0064_1c20 => extern "C" fn graphic_is_battle_map() -> u32 {
        sc::scene_byte(&mem(), 0x1197)
    }
    /// `graphicGetIsBattleMap()`: the current battle-map flag.
    graphicGetIsBattleMap @ 0x0064_1bb0 => extern "C" fn graphic_get_is_battle_map() -> u32 {
        sc::scene_byte(&mem(), 0x1196)
    }
    /// `graphicNeedResetMapParam()`.
    graphicNeedResetMapParam @ 0x0064_2690 => extern "C" fn graphic_need_reset_map_param() -> u32 {
        sc::scene_byte(&mem(), 0x10d2d)
    }
    /// `graphicDoneResetMapParam()`.
    graphicDoneResetMapParam @ 0x0063_df60 => extern "C" fn graphic_done_reset_map_param() -> u32 {
        sc::set_scene_byte(&mut mem(), 0x10d2d, 0)
    }
    /// `graphicSetSceneFlagFb45()`.
    graphicSetSceneFlagFb45 @ 0x0064_2fa0 => extern "C" fn graphic_set_scene_flag_fb45() -> u32 {
        sc::set_scene_byte(&mut mem(), 0xfb45, 1)
    }
    /// `graphicIsMiniMapInBlitzBall()`.
    graphicIsMiniMapInBlitzBall @ 0x0064_1c70 => extern "C" fn graphic_is_mini_map_in_blitz_ball() -> u32 {
        sc::scene_byte(&mem(), 0)
    }
    /// `graphicGetMagic0146Key()`.
    graphicGetMagic0146Key @ 0x0064_1310 => extern "C" fn graphic_get_magic_0146_key() -> u32 {
        sc::scene_byte(&mem(), 3)
    }
    /// `graphicSetIsBattleMap(v)`.
    graphicSetIsBattleMap @ 0x0064_3490 => extern "C" fn graphic_set_is_battle_map(v: u32) -> u8 {
        sc::set_is_battle_map(&mut mem(), v)
    }
    /// `graphicIsAnyBattleMap()`.
    graphicIsAnyBattleMap @ 0x0064_1bd0 => extern "C" fn graphic_is_any_battle_map() -> u32 {
        sc::is_any_battle_map(&mem())
    }
    /// `graphicMinimapFogSetEnable(v)`.
    graphicMinimapFogSetEnable @ 0x0064_25d0 => extern "C" fn graphic_minimap_fog_set_enable(v: u32) -> u32 {
        sc::set_scene_flag(&mut mem(), 0x79, v)
    }
    /// `graphicMinimapSetEnable(v)`.
    graphicMinimapSetEnable @ 0x0064_25f0 => extern "C" fn graphic_minimap_set_enable(v: u32) -> u32 {
        sc::set_scene_flag(&mut mem(), 0x78, v)
    }
    /// `graphicSetMapAlphaFast(v)`.
    graphicSetMapAlphaFast @ 0x0064_3610 => extern "C" fn graphic_set_map_alpha_fast(v: u32) -> u32 {
        sc::set_scene_flag(&mut mem(), 0xd4, v)
    }
    /// `graphicScreenSetOffset(x, y)`.
    graphicScreenSetOffset @ 0x0064_2d60 => extern "C" fn graphic_screen_set_offset(x: f32, y: f32) -> u32 {
        sc::screen_set_offset(&mut mem(), x, y)
    }
    /// `graphicScreenGetDttOffset(&x, &y)`.
    graphicScreenGetDttOffset @ 0x0064_2cf0 => extern "C" fn graphic_screen_get_dtt_offset(x: u32, y: u32) -> u32 {
        sc::screen_get_dtt_offset(&mut mem(), x, y)
    }
    /// `graphicScreenSetDttOffset(x, y)`.
    graphicScreenSetDttOffset @ 0x0064_2d40 => extern "C" fn graphic_screen_set_dtt_offset(x: f32, y: f32) {
        sc::screen_set_dtt_offset(&mut mem(), x, y);
    }
    /// `graphicGetCut2DSceneBias()`.
    graphicGetCut2DSceneBias @ 0x0064_1130 => extern "C" fn graphic_get_cut_2d_scene_bias() -> f32 {
        sc::scene_float(&mem(), 0xf9bc)
    }
    /// `graphicCameraGetFOVy()`.
    graphicCameraGetFOVy @ 0x0063_9570 => extern "C" fn graphic_camera_get_fovy() -> f32 {
        sc::camera_float(&mem(), 0x154)
    }
    /// `getNearPlane()`.
    getNearPlane @ 0x0063_8e70 => extern "C" fn get_near_plane() -> f32 {
        sc::camera_float(&mem(), 0xb8)
    }
    /// `graphicCharacterInstanceSetDrawSituation(chr, v)`.
    graphicCharacterInstanceSetDrawSituation @ 0x0063_b7d0 => extern "C" fn graphic_character_instance_set_draw_situation(chr: u32, v: u32) {
        sc::character_set_draw_situation(&mut mem(), chr, v);
    }
    /// `graphicIsMagicFileAsyncLoadAvailable()`.
    graphicIsMagicFileAsyncLoadAvailable @ 0x0064_1c60 => extern "C" fn graphic_is_magic_file_async_load_available() -> u32 {
        sc::MAGIC_ASYNC_LOAD.get(&mem())
    }
    /// `graphicSetGameOverNotify(v)`.
    graphicSetGameOverNotify @ 0x0064_3260 => extern "C" fn graphic_set_game_over_notify(v: u32) -> u32 {
        sc::GAME_OVER_NOTIFY.set(&mut mem(), v);
        v
    }
    /// `graphicGetTextureVideoInfo()`.
    graphicGetTextureVideoInfo @ 0x0064_1400 => extern "C" fn graphic_get_texture_video_info() -> u16 {
        sc::TEXTURE_VIDEO_INFO.get(&mem())
    }
    /// `graphicIsCharacterLoadingOrInstanciating()`.
    graphicIsCharacterLoadingOrInstanciating @ 0x0064_1bc0 => extern "C" fn graphic_is_character_loading() -> u32 {
        sc::CHARACTER_LOADING.get(&mem()) & 0x7fff_ffff
    }
    /// `graphicGetNowUIType()`.
    graphicGetNowUIType @ 0x0064_1350 => extern "C" fn graphic_get_now_ui_type() -> u32 {
        let m = mem();
        sc::field(&m, ui::bridge::UI.get(&m), 0x2be0)
    }
    /// `graphicSetNowUIType(v)`.
    graphicSetNowUIType @ 0x0064_37c0 => extern "C" fn graphic_set_now_ui_type(v: u32) -> u32 {
        ui::bridge::set_now_ui_type(&mut mem(), v)
    }
    /// `cgraphicSetNowUIType(v)`.
    cgraphicSetNowUIType @ 0x0063_8d90 => extern "C" fn cgraphic_set_now_ui_type(v: u32) -> u32 {
        ui::bridge::set_now_ui_type(&mut mem(), v)
    }
    /// `graphicGetAbmap()`.
    graphicGetAbmap @ 0x0064_1060 => extern "C" fn graphic_get_abmap() -> u32 {
        ui::bridge::get_abmap(&mem())
    }

    // ---- UI ----
    /// `graphicUiRemapX2(x)`.
    graphicUiRemapX2 @ 0x0064_47e0 => extern "C" fn graphic_ui_remap_x2(x: f32) -> f32 {
        ui::remap_x2(x)
    }
    /// `graphicUiRemapY2(y)`.
    graphicUiRemapY2 @ 0x0064_4820 => extern "C" fn graphic_ui_remap_y2(y: f32) -> f32 {
        ui::remap_y2(y)
    }
    /// `graphicUiRemapX(x)`.
    graphicUiRemapX @ 0x0064_47c0 => extern "C" fn graphic_ui_remap_x(x: f32) -> f32 {
        ui::remap_x(x)
    }
    /// `graphicUiRemapY(y)`.
    graphicUiRemapY @ 0x0064_4800 => extern "C" fn graphic_ui_remap_y(y: f32) -> f32 {
        ui::remap_y(y)
    }
    /// `graphicGetScreenRatio()`.
    graphicGetScreenRatio @ 0x0064_13f0 => extern "C" fn graphic_get_screen_ratio() -> f32 {
        ui::SCREEN_RATIO
    }
    /// `graphicFontGetScreenWH(&w, &h)`.
    graphicFontGetScreenWH @ 0x0064_0db0 => extern "C" fn graphic_font_get_screen_wh(w: u32, h: u32) -> u32 {
        ui::bridge::font_get_screen_wh(&mut mem(), w, h)
    }
    /// `graphicFontGetEdgeRatio()`.
    graphicFontGetEdgeRatio @ 0x0064_0d70 => extern "C" fn graphic_font_get_edge_ratio() -> f32 {
        ui::bridge::font_get_edge_ratio(&mem())
    }

    // ---- managers ----
    /// `PostProcessManager_GetInstance()`.
    PostProcessManager_GetInstance @ 0x0069_2be0 => extern "C" fn post_process_manager_get_instance() -> u32 {
        instance(mg::POST_PROCESS)
    }
    /// `PostProcessTweakUtil_GetInstance()`.
    PostProcessTweakUtil_GetInstance @ 0x0069_2bf0 => extern "C" fn post_process_tweak_util_get_instance() -> u32 {
        instance(mg::POST_PROCESS_TWEAK)
    }
    /// `TextureAnimationManager_GetInstance()`.
    TextureAnimationManager_GetInstance @ 0x006b_02e0 => extern "C" fn texture_animation_manager_get_instance() -> u32 {
        instance(mg::TEXTURE_ANIMATION)
    }
    /// `ShadowManager_getInstance()`.
    ShadowManager_getInstance @ 0x006d_2390 => extern "C" fn shadow_manager_get_instance() -> u32 {
        instance(mg::SHADOW)
    }
    /// `ShadowManager_GetShadowSize()`.
    ShadowManager_GetShadowSize @ 0x006c_cd00 => extern "C" fn shadow_manager_get_shadow_size() -> u32 {
        instance(mg::SHADOW_SIZE)
    }
    /// `SpecularMapParamsManager_GetInstance()`.
    SpecularMapParamsManager_GetInstance @ 0x006c_57e0 => extern "C" fn specular_map_params_manager_get_instance() -> u32 {
        instance(mg::SPECULAR_MAP_PARAMS)
    }
    /// `DebugMenuManager_GetInstance()`.
    DebugMenuManager_GetInstance @ 0x006b_49b0 => extern "C" fn debug_menu_manager_get_instance() -> u32 {
        instance(mg::DEBUG_MENU)
    }
    /// `graphicCheckPauseGame()`.
    graphicCheckPauseGame @ 0x006b_cc90 => extern "C" fn graphic_check_pause_game() -> u32 {
        instance(mg::PAUSE_GAME)
    }
    /// `PostProcessTweakUtil_Destroy()`.
    PostProcessTweakUtil_Destroy @ 0x0069_2bc0 => extern "C" fn post_process_tweak_util_destroy() {
        mg::destroy_singleton(&mut mem(), mg::POST_PROCESS_TWEAK);
    }
    /// `DebugMenuManager_Destroy()`.
    DebugMenuManager_Destroy @ 0x006b_4990 => extern "C" fn debug_menu_manager_destroy() {
        mg::destroy_singleton(&mut mem(), mg::DEBUG_MENU);
    }
    /// `graphicPauseGameDestroy()`.
    graphicPauseGameDestroy @ 0x006b_cc70 => extern "C" fn graphic_pause_game_destroy() {
        mg::destroy_singleton(&mut mem(), mg::PAUSE_GAME);
    }
    /// `graphicDynGeoFillBufferIsValid()`.
    graphicDynGeoFillBufferIsValid @ 0x0063_fd70 => extern "C" fn graphic_dyn_geo_fill_buffer_is_valid() -> u32 {
        mg::dyn_geo_fill_buffer_is_valid(&mem())
    }
    /// `DynGeoMemManager_getCurrentBufferId(this)`.
    DynGeoMemManager_getCurrentBufferId @ 0x006d_e9f0 => extern "thiscall" fn dyn_geo_get_current_buffer_id(this: u32) -> u32 {
        mg::dyn_geo_current_buffer_id(&mem(), this)
    }
    /// `DynGeoMemManager_clearDynGeoMemory(this)`.
    DynGeoMemManager_clearDynGeoMemory @ 0x006d_e700 => extern "thiscall" fn dyn_geo_clear(this: u32) {
        mg::dyn_geo_clear(&mut mem(), this);
    }
    /// `DynGeoMemManager_selectBuffer(this, i)`.
    DynGeoMemManager_selectBuffer @ 0x006d_ec90 => extern "thiscall" fn dyn_geo_select(this: u32, i: u32) -> u32 {
        mg::dyn_geo_select(&mut mem(), this, i)
    }
    /// `TextureAnimationManager_getMaterialByLayNum(this, lay)`.
    TextureAnimationManager_getMaterialByLayNum @ 0x006b_2070 => extern "thiscall" fn texture_animation_material_by_lay_num(this: u32, lay: u32) -> u32 {
        mg::material_by_lay_num(&mem(), this, lay)
    }
    /// `TextureAnimation_NextFrame(this)`: quotient in `eax`, frame in `edx`.
    TextureAnimation_NextFrame @ 0x006b_22c0 => extern "thiscall" fn texture_animation_next_frame(this: u32) -> u64 {
        step_frame(this, 1)
    }
    /// `TextureAnimation_PrevFrame(this)`: quotient in `eax`, frame in `edx`.
    TextureAnimation_PrevFrame @ 0x006b_2380 => extern "thiscall" fn texture_animation_prev_frame(this: u32) -> u64 {
        step_frame(this, u32::MAX)
    }
    /// `TextureAnimationInfoChar_active(this)`.
    TextureAnimationInfoChar_active @ 0x006b_1c30 => extern "thiscall" fn texture_animation_info_char_active(this: u32) {
        mg::set_inactive(&mut mem(), this, false);
    }
    /// `TextureAnimationInfoChar_deactive(this)`.
    TextureAnimationInfoChar_deactive @ 0x006b_04a0 => extern "thiscall" fn texture_animation_info_char_deactive(this: u32) {
        mg::set_inactive(&mut mem(), this, true);
    }
    /// `TextureAnimation_GetFloatPair24(this, out)`.
    TextureAnimation_GetFloatPair24 @ 0x006b_1cc0 => extern "thiscall" fn texture_animation_get_pair_24(this: u32, out: u32) -> u32 {
        mg::get_float_pair(&mut mem(), this, 0x24, out)
    }
    /// `TextureAnimation_GetFloatPair2c(this, out)`.
    TextureAnimation_GetFloatPair2c @ 0x006b_1ce0 => extern "thiscall" fn texture_animation_get_pair_2c(this: u32, out: u32) -> u32 {
        mg::get_float_pair(&mut mem(), this, 0x2c, out)
    }
    /// `TextureAnimation_SetByte1c(this, v)`.
    TextureAnimation_SetByte1c @ 0x006b_3650 => extern "thiscall" fn texture_animation_set_byte_1c(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x1c, v)
    }
    /// `PostProcessManager_SetByte123b0(this, v)`.
    PostProcessManager_SetByte123b0 @ 0x0069_83f0 => extern "thiscall" fn post_process_set_byte_123b0(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x123b0, v)
    }
    /// `PostProcessManager_SetByte123b1(this, v)`.
    PostProcessManager_SetByte123b1 @ 0x0069_83e0 => extern "thiscall" fn post_process_set_byte_123b1(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x123b1, v)
    }
    /// `PostProcessManager_SetByte123b3(this, v)`.
    PostProcessManager_SetByte123b3 @ 0x0069_85b0 => extern "thiscall" fn post_process_set_byte_123b3(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x123b3, v)
    }
    /// `PostProcessManager_setCrossFadeCaptureEnable(this, v)`.
    PostProcessManager_setCrossFadeCaptureEnable @ 0x0069_8490 => extern "thiscall" fn post_process_set_cross_fade_capture_enable(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x123ba, v)
    }
    /// `PostProcessManager_SetByte123bb(this, v)`.
    PostProcessManager_SetByte123bb @ 0x0069_8480 => extern "thiscall" fn post_process_set_byte_123bb(this: u32, v: u32) -> u8 {
        mg::set_byte_field(&mut mem(), this, 0x123bb, v)
    }
    /// `PostProcessManager_setCrossFadeWeight(this, v)`.
    PostProcessManager_setCrossFadeWeight @ 0x0069_84b0 => extern "thiscall" fn post_process_set_cross_fade_weight(this: u32, v: u32) -> u32 {
        mg::set_field(&mut mem(), this, 0x123bc, v)
    }
    /// `PostProcessManager_setCrossFadeTime(this, v)`.
    PostProcessManager_setCrossFadeTime @ 0x0069_84a0 => extern "thiscall" fn post_process_set_cross_fade_time(this: u32, v: u32) -> u32 {
        mg::set_field(&mut mem(), this, 0x123c0, v)
    }
    /// `PostProcessManager_isEffectAllowed(id)`.
    PostProcessManager_isEffectAllowed @ 0x0069_8730 => extern "stdcall" fn post_process_is_effect_allowed(id: u32) -> u8 {
        mg::effect_allowed(id)
    }
    /// `Distortion_updateDistortionBlur(this, v)`.
    Distortion_updateDistortionBlur @ 0x006c_3230 => extern "thiscall" fn distortion_update_blur(this: u32, v: f32) -> u32 {
        mg::distortion_update_blur(&mut mem(), this, v)
    }
    /// `BlurEffect_activate(this)`.
    BlurEffect_activate @ 0x006c_35a0 => extern "thiscall" fn blur_effect_activate(this: u32) {
        mg::blur_activate(&mut mem(), this);
    }
    /// `BlurEffect_setTarget(this, v)`.
    BlurEffect_setTarget @ 0x006c_4a90 => extern "thiscall" fn blur_effect_set_target(this: u32, v: u32) -> u32 {
        mg::set_with_flag(&mut mem(), this, v)
    }
    /// `BlurEffect_SetField84(this, v)`.
    BlurEffect_SetField84 @ 0x006c_44d0 => extern "thiscall" fn blur_effect_set_field_84(this: u32, v: u32) -> u32 {
        mg::set_field(&mut mem(), this, 0x84, v)
    }
    /// `BlurEffect_SetFloats6c(this, a, b)`.
    BlurEffect_SetFloats6c @ 0x006c_4710 => extern "thiscall" fn blur_effect_set_floats_6c(this: u32, a: f32, b: f32) {
        let mut m = mem();
        mg::set_float(&mut m, this, 0x6c, a);
        mg::set_float(&mut m, this, 0x70, b);
    }
    /// `ShadowManager_SetFloats401c(this, a, b)`.
    ShadowManager_SetFloats401c @ 0x006c_ea80 => extern "thiscall" fn shadow_manager_set_floats_401c(this: u32, a: f32, b: f32) {
        let mut m = mem();
        mg::set_float(&mut m, this, 0x401c, a);
        mg::set_float(&mut m, this, 0x4018, b);
    }
    /// `ShadowManager_clearCascades(this)`.
    ShadowManager_clearCascades @ 0x006d_1780 => extern "thiscall" fn shadow_manager_clear_cascades(this: u32) -> u32 {
        mg::clear_words(&mut mem(), this)
    }
    /// `ShadowManager_getCascade(this, i)`.
    ShadowManager_getCascade @ 0x006d_22e0 => extern "thiscall" fn shadow_manager_get_cascade(this: u32, i: u32) -> u32 {
        mg::word_at(&mem(), this, 0x60, i)
    }
    /// `ShadowManager_getTextureSampler(this)`.
    ShadowManager_getTextureSampler @ 0x006d_2520 => extern "thiscall" fn shadow_manager_get_texture_sampler(this: u32) -> u32 {
        mg::embedded(this, 0x3fc4)
    }

    // ---- the model cache ----
    /// `Cache_CheckYet(kind, id)`.
    Cache_CheckYet @ 0x0083_ffe0 => extern "C" fn cache_check_yet(kind: i32, id: u32) -> u32 {
        cache::bridge::check_yet(&mem(), kind, id)
    }
    /// `Cache_Exist(kind, id)`.
    Cache_Exist @ 0x0084_06f0 => extern "C" fn cache_exist(kind: i32, id: u32) -> u32 {
        cache::bridge::exist(&mut mem(), kind, id)
    }
    /// `Cache_Exist3(kind, id)`.
    Cache_Exist3 @ 0x0084_0790 => extern "C" fn cache_exist3(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0xc)
    }
    /// `Cache_Exist_ClothMeshData(kind, id)`.
    Cache_Exist_ClothMeshData @ 0x0084_07d0 => extern "C" fn cache_exist_cloth_mesh_data(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0x18)
    }
    /// `Cache_Exist_ClothMeshMappingData(kind, id)`.
    Cache_Exist_ClothMeshMappingData @ 0x0084_0810 => extern "C" fn cache_exist_cloth_mesh_mapping_data(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0x1c)
    }
    /// `Cache_Exist_ClothVertexBuff(kind, id)`.
    Cache_Exist_ClothVertexBuff @ 0x0084_0850 => extern "C" fn cache_exist_cloth_vertex_buff(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0x20)
    }
    /// `Cache_Entry_ClothVertexBuff(kind, id, buffer)`.
    Cache_Entry_ClothVertexBuff @ 0x0084_05b0 => extern "C" fn cache_entry_cloth_vertex_buff(kind: i32, id: u32, buffer: u32) {
        cache::bridge::entry_cloth_vertex_buff(&mut mem(), kind, id, buffer);
    }
    /// `Cache_Entry_RenderData(kind, id, &a, &b, &c, &d)`.
    Cache_Entry_RenderData @ 0x0084_0680 => extern "C" fn cache_entry_render_data(kind: i32, id: u32, a: u32, b: u32, c: u32, d: u32) {
        cache::bridge::entry_render_data(&mut mem(), kind, id, [a, b, c, d]);
    }
    /// `Cache_SetLock(kind, id, lock)`.
    Cache_SetLock @ 0x0084_0ac0 => extern "C" fn cache_set_lock(kind: i32, id: u32, lock: u32) {
        cache::bridge::set_lock(&mut mem(), kind, id, lock);
    }
    /// `Cache_ToTop(kind, id)`.
    Cache_ToTop @ 0x0084_0cf0 => extern "C" fn cache_to_top(kind: i32, id: u32) -> u32 {
        cache::bridge::touch(&mut mem(), kind, id)
    }
    /// `Cache_Use(kind, id)`.
    Cache_Use @ 0x0084_0dd0 => extern "C" fn cache_use(kind: i32, id: u32) {
        cache::bridge::use_entry(&mut mem(), kind, id);
    }
    /// `Cache_Unuse(kind, id)`.
    Cache_Unuse @ 0x0084_0d70 => extern "C" fn cache_unuse(kind: i32, id: u32) {
        cache::bridge::unuse(&mut mem(), kind, id);
    }
    /// `Cache_Entry(kind, id, data)`.
    Cache_Entry @ 0x0084_0560 => extern "C" fn cache_entry(kind: u32, id: u32, data: u32) {
        cache::bridge::entry_add(&mut mem(), kind, id, data);
    }
    /// `Cache_Entry_NotYet(kind, id, yet)`.
    Cache_Entry_NotYet @ 0x0084_05f0 => extern "C" fn cache_entry_not_yet(kind: i32, id: u32, yet: u32) {
        cache::bridge::entry_not_yet(&mut mem(), kind, id, yet);
    }
}

/// A singleton's pointer.
fn instance(g: Global<u32>) -> u32 {
    mg::instance(&mem(), g)
}

/// A texture animation's frame step: the quotient in `eax`, the new frame
/// in `edx` (as the original's `div` leaves them).
fn step_frame(this: u32, step: u32) -> u64 {
    let mut m = mem();
    let q = mg::step_frame(&mut m, this, step);
    let frame: u32 = ffx_bridge::Mem::read(&m, this + 0x18);
    u64::from(frame) << 32 | u64::from(q)
}
