//! `ffx-graphic`: the remaster's render layer over `PhyreEngine`. Functions
//! whose original leaves a result in `eax` return it; those leaving a byte
//! in `al` over the caller's `eax` return `u8`.

use super::{mem, ported};
use ffx_bridge::Global;
use ffx_graphic::{cache, managers, scene, ui, vfx};

use ffx_graphic::state::bridge as st;
use ffx_graphic::texanim::bridge as ta;
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
    /// `DynGeoMemManager_getBufferFromDynGeoMemory(this, size)`.
    DynGeoMemManager_getBufferFromDynGeoMemory @ 0x006d_e940 => extern "thiscall" fn dyn_geo_get_buffer(this: u32, size: u32) -> u32 {
        mg::dyn_geo_get_buffer(&mut mem(), this, size)
    }
    /// `graphicDynGeoMemManagerGetBufferForSegment(data, segment, vertices, indices)`.
    graphicDynGeoMemManagerGetBufferForSegment @ 0x0063_fdb0 => extern "C" fn graphic_dyn_geo_get_buffer_for_segment(data: u32, segment: u32, vertices: u32, indices: u32) -> u32 {
        mg::get_buffer_for_segment(&mut mem(), data, segment, vertices, indices)
    }
    /// `PhyreScene_getYIQParam(this, &y0, &y1, &y2, &o0, &o1, &o2)`.
    PhyreScene_getYIQParam @ 0x0065_e9e0 => extern "thiscall" fn scene_get_yiq_param(this: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        sc::yiq_param(&mut mem(), this, [a, b, c, d, e, f])
    }
    /// `PhyreScene_removeChrFromPipeline(this, chr)`.
    PhyreScene_removeChrFromPipeline @ 0x0066_8640 => extern "thiscall" fn scene_remove_chr_from_pipeline(this: u32, chr: u32) -> u32 {
        sc::remove_chr_from_pipeline(&mut mem(), this, chr)
    }
    /// `graphicCharacterInstanceSetMeshSegmentHide(inst, id, hide)`.
    graphicCharacterInstanceSetMeshSegmentHide @ 0x0063_bdf0 => extern "C" fn graphic_character_set_mesh_segment_hide(inst: u32, id: u32, hide: u32) -> u32 {
        sc::set_mesh_segment_hide(&mut mem(), inst, id, hide)
    }
    /// `graphicIsEscMenuActive()`.
    graphicIsEscMenuActive @ 0x0063_da80 => extern "C" fn graphic_is_esc_menu_active() -> u32 {
        sc::is_esc_menu_active(&mem())
    }
    /// `ShadowManager_RemoveFromShadowMap(this, obj)`.
    ShadowManager_RemoveFromShadowMap @ 0x006c_e9d0 => extern "thiscall" fn shadow_remove_from_shadow_map(this: u32, obj: u32) -> u8 {
        mg::remove_from_shadow_map(&mut mem(), this, obj)
    }
    /// `DynGeoTree_Find(this, &key)`.
    DynGeoTree_Find @ 0x006d_edd0 => extern "thiscall" fn dyn_geo_tree_find(this: u32, key: u32) -> u32 {
        mg::tree_find(&mem(), this, key)
    }
    // ---- texture animation ----
    /// `loadImage(addr, bytes)`.
    loadImage @ 0x0077_e730 => extern "C" fn load_image(addr: u32, bytes: u32) -> u32 {
        ta::load_image(&mut mem(), addr, bytes, 0x100)
    }
    /// `TexAnim_LoadImage101(addr, bytes)`.
    TexAnim_LoadImage101 @ 0x0077_e7c0 => extern "C" fn tex_anim_load_image_101(addr: u32, bytes: u32) -> u32 {
        ta::load_image(&mut mem(), addr, bytes, 0x101)
    }
    /// `TexAnim_LoadImageRegs(a, b, bytes)`.
    TexAnim_LoadImageRegs @ 0x0077_e770 => extern "C" fn tex_anim_load_image_regs(a: u32, b: u32, bytes: u32) -> u32 {
        ta::load_image_regs(&mut mem(), a, b, bytes)
    }
    /// `TexAnim_SyncPath(a)`.
    TexAnim_SyncPath @ 0x0077_e800 => extern "C" fn tex_anim_sync_path(a: u32) -> u32 {
        ta::sync_path(&mut mem(), a)
    }
    /// `TexAnim_SyncPath2(a)`.
    TexAnim_SyncPath2 @ 0x0077_eaa0 => extern "C" fn tex_anim_sync_path2(a: u32) -> u32 {
        ta::sync_path(&mut mem(), a)
    }
    /// `TexAnim_SetQword(q, a, b, c)`.
    TexAnim_SetQword @ 0x0077_ebc0 => extern "C" fn tex_anim_set_qword(q: u32, a: u32, b: u32, c: u32) -> u32 {
        ta::qword(&mut mem(), q, a, b, c)
    }
    /// `TexAnim_SetDmaTag(q, a, b, c, d, e, f)`.
    TexAnim_SetDmaTag @ 0x0077_eb70 => extern "C" fn tex_anim_set_dma_tag(q: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 {
        ta::dma_tag(&mut mem(), q, [a, b, c, d, e, f])
    }
    /// `TexAnim_SetGifTag(q, p2..p9)`.
    TexAnim_SetGifTag @ 0x0077_ebe0 => extern "C" fn tex_anim_set_gif_tag(q: u32, p2: u32, p3: u32, p4: u32, p5: u32, p6: u32, p7: u32, p8: u32, p9: u32) -> u32 {
        ta::gif_tag(&mut mem(), q, [p2, p3, p4, p5, p6, p7, p8, p9])
    }
    /// `TexAnim_GetWork(i)`.
    TexAnim_GetWork @ 0x0077_e9f0 => extern "C" fn tex_anim_get_work(i: u32) -> u32 {
        ta::work(&mem(), i)
    }
    /// `tex_anim_no_set(i, a, b, c, d)`.
    tex_anim_no_set @ 0x0077_e940 => extern "C" fn tex_anim_no_set(i: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        ta::no_set(&mut mem(), i, a, b, c, d)
    }
    /// `tex_anim_dlg(arg)`.
    tex_anim_dlg @ 0x0077_e920 => extern "C" fn tex_anim_dlg(arg: u32) -> u32 {
        ta::dlg(&mut mem(), arg)
    }
    /// `TexAnim_ResetPos(i, which, v)`.
    TexAnim_ResetPos @ 0x0077_fda0 => extern "C" fn tex_anim_reset_pos(i: u32, which: u32, v: u32) {
        ta::reset_pos(&mut mem(), i, which, v);
    }
    /// `DynGeoTree_Insert(this, node)`.
    DynGeoTree_Insert @ 0x006d_ee50 => extern "thiscall" fn dyn_geo_tree_insert(this: u32, node: u32) -> u32 {
        mg::tree_insert(&mut mem(), this, node)
    }
    /// `PhyreScene_getMinimapViewportSize(this)`.
    PhyreScene_getMinimapViewportSize @ 0x0065_e320 => extern "thiscall" fn scene_minimap_viewport_size(this: u32) -> u32 {
        sc::minimap_viewport_size(&mem(), this)
    }
    /// `PhyreScene_getMinimapViewportX(this)`.
    PhyreScene_getMinimapViewportX @ 0x0065_e3f0 => extern "thiscall" fn scene_minimap_viewport_x(this: u32) -> u32 {
        sc::minimap_viewport_x(&mem(), this)
    }
    /// `PhyreScene_getMinimapViewportY(this)`.
    PhyreScene_getMinimapViewportY @ 0x0065_e560 => extern "thiscall" fn scene_minimap_viewport_y(this: u32) -> u32 {
        sc::minimap_viewport_y(&mem(), this)
    }
    /// `graphicUiElementRemap(element, kind)`.
    graphicUiElementRemap @ 0x0064_4320 => extern "C" fn graphic_ui_element_remap(element: u32, kind: u32) {
        ui::bridge::element_remap(&mut mem(), element, kind);
    }
    /// `tex_anim_pos_set_old(i, which, v)`.
    tex_anim_pos_set_old @ 0x0078_08f0 => extern "C" fn tex_anim_pos_set_old(i: u32, which: u32, v: u32) {
        ta::pos_set_old(&mut mem(), i, which, v);
    }
    /// `TextureAnimation_FindByName(this, name)`.
    TextureAnimation_FindByName @ 0x006b_1d10 => extern "thiscall" fn texture_animation_find_by_name(this: u32, name: u32) -> u32 {
        mg::find_by_name(&mem(), this, name)
    }
    /// `TextureAnimation_FindMaterial(this, list, index, name)`.
    TextureAnimation_FindMaterial @ 0x006b_1e40 => extern "thiscall" fn texture_animation_find_material(this: u32, list: u32, index: u32, name: u32) -> u32 {
        mg::find_material(&mem(), this, list, index, name)
    }
    // ---- round 4 ----
    /// `Cache_Init()`.
    Cache_Init @ 0x0084_0910 => extern "C" fn cache_init() {
        cache::bridge::init(&mut mem());
    }
    /// `Cache_ClearCheck()`.
    Cache_ClearCheck @ 0x0084_0120 => extern "C" fn cache_clear_check() {
        cache::bridge::clear_check(&mut mem());
    }
    /// `Cache_Lock(kind, id)`.
    Cache_Lock @ 0x0084_0960 => extern "C" fn cache_lock(kind: i32, id: u32) {
        cache::bridge::lock(&mut mem(), kind, id);
    }
    /// `Cache_Exist_RenderData(kind, id)`.
    Cache_Exist_RenderData @ 0x0084_0890 => extern "C" fn cache_exist_render_data(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0x10)
    }
    /// `Cache_Exist_ShadowMeshData(kind, id)`.
    Cache_Exist_ShadowMeshData @ 0x0084_08d0 => extern "C" fn cache_exist_shadow_mesh_data(kind: i32, id: u32) -> u32 {
        cache::bridge::exist_field(&mem(), kind, id, 0x14)
    }
    /// `graphicScreenGetOffset(&x, &y)`.
    graphicScreenGetOffset @ 0x0064_2d10 => extern "C" fn graphic_screen_get_offset(x: u32, y: u32) -> u32 {
        sc::screen_get_offset(&mut mem(), x, y)
    }
    /// `PhyreScene_setTextureVideoInfo(this, info, playing)`.
    PhyreScene_setTextureVideoInfo @ 0x0066_e800 => extern "thiscall" fn scene_set_texture_video_info(this: u32, info: u32, playing: u32) -> u32 {
        sc::set_texture_video_info(&mut mem(), this, info, playing)
    }
    /// `PostProcessManager_setFocusDepth(this, which, depth)`.
    PostProcessManager_setFocusDepth @ 0x0069_8550 => extern "thiscall" fn post_process_set_focus_depth(this: u32, which: u32, depth: f32) -> u32 {
        mg::set_focus_depth(&mut mem(), this, which, depth)
    }
    /// `SpecularMapParamsManager_needDrawSpecularPass(this)`.
    SpecularMapParamsManager_needDrawSpecularPass @ 0x006c_59a0 => extern "thiscall" fn specular_need_draw_pass(this: u32) -> u32 {
        mg::need_draw_specular_pass(&mem(), this)
    }
    /// `ShadowManager_AlreadyinShadowMap(this, obj, &col, &row)`.
    ShadowManager_AlreadyinShadowMap @ 0x006c_8040 => extern "thiscall" fn shadow_already_in_map(this: u32, obj: u32, col: u32, row: u32) -> u8 {
        mg::already_in_shadow_map(&mut mem(), this, obj, col, row)
    }
    /// `PhyreCursor_IsValid_00660240(this)`.
    PhyreCursor_IsValid_00660240 @ 0x0066_0240 => extern "thiscall" fn cursor_valid_660240(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_00660270(this)`.
    PhyreCursor_IsValid_00660270 @ 0x0066_0270 => extern "thiscall" fn cursor_valid_660270(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_006602a0(this)`.
    PhyreCursor_IsValid_006602a0 @ 0x0066_02a0 => extern "thiscall" fn cursor_valid_6602a0(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_006602d0(this)`.
    PhyreCursor_IsValid_006602d0 @ 0x0066_02d0 => extern "thiscall" fn cursor_valid_6602d0(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_00660300(this)`.
    PhyreCursor_IsValid_00660300 @ 0x0066_0300 => extern "thiscall" fn cursor_valid_660300(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreCursor_IsValid_006b28f0(this)`.
    PhyreCursor_IsValid_006b28f0 @ 0x006b_28f0 => extern "thiscall" fn cursor_valid_6b28f0(this: u32) -> u8 {
        mg::cursor_valid(&mem(), this)
    }
    /// `PhyreScene_SetByte01(this, v)`.
    PhyreScene_SetByte01 @ 0x0066_cfe0 => extern "thiscall" fn scene_set_byte_01(this: u32, v: u32) -> u8 {
        sc::set_byte_field(&mut mem(), this, 1, v)
    }
    /// `PhyreScene_SetByte03(this, v)`.
    PhyreScene_SetByte03 @ 0x0066_d1b0 => extern "thiscall" fn scene_set_byte_03(this: u32, v: u32) -> u8 {
        sc::set_byte_field(&mut mem(), this, 3, v)
    }
    /// `PostProcessManager_setAllSwitches(this, v)`.
    PostProcessManager_setAllSwitches @ 0x0069_89c0 => extern "thiscall" fn post_process_set_all_switches(this: u32, v: u32) -> u8 {
        mg::set_all_switches(&mut mem(), this, v)
    }
    /// `PostProcessManager_resetCrossFade(this)`.
    PostProcessManager_resetCrossFade @ 0x0069_8f70 => extern "thiscall" fn post_process_reset_cross_fade(this: u32) {
        mg::reset_cross_fade(&mut mem(), this);
    }
    /// `BlurEffect_SetFloats74(this, a, b, c, d)`.
    BlurEffect_SetFloats74 @ 0x006c_46e0 => extern "thiscall" fn blur_effect_set_floats_74(this: u32, a: f32, b: f32, c: f32, d: f32) {
        mg::set_floats4(&mut mem(), this, [a, b, c, d]);
    }
    /// `Distortion_updateDistortionVertex(this, i, j, x, y, z, w, u, v)`.
    Distortion_updateDistortionVertex @ 0x006c_3240 => extern "thiscall" fn distortion_update_vertex(this: u32, i: u32, j: u32, x: f32, y: f32, z: f32, w: f32, u: f32, v: f32) -> u32 {
        mg::distortion_update_vertex(&mut mem(), this, i, j, [x, y, z, w, u, v])
    }
    /// `graphicCharacterInstanceGetMaxiumBoundingBox(inst, &x, &z, &y)`.
    graphicCharacterInstanceGetMaxiumBoundingBox @ 0x0063_b1b0 => extern "C" fn graphic_character_max_bounding_box(inst: u32, x: u32, z: u32, y: u32) {
        sc::character_max_bounding_box(&mut mem(), inst, x, z, y);
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
    /// `PhyreScene_isOpeningScreenPlaying()`.
    PhyreScene_isOpeningScreenPlaying @ 0x0066_0400 => extern "C" fn scene_is_opening_screen_playing() -> u8 {
        sc::OPENING_PLAYING.get(&mem())
    }
    /// `PhyreScene_setMeshVisible(mesh, visible)`.
    PhyreScene_setMeshVisible @ 0x0066_d910 => extern "stdcall" fn scene_set_mesh_visible(mesh: u32, visible: u32) {
        sc::set_mesh_visible(&mut mem(), mesh, visible);
    }

    // ---- graphic* over the scene ----
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
    /// `graphicGetTextureVideoInfo()`.
    graphicGetTextureVideoInfo @ 0x0064_1400 => extern "C" fn graphic_get_texture_video_info() -> u16 {
        sc::TEXTURE_VIDEO_INFO.get(&mem())
    }
    /// `graphicIsCharacterLoadingOrInstanciating()`.
    graphicIsCharacterLoadingOrInstanciating @ 0x0064_1bc0 => extern "C" fn graphic_is_character_loading() -> u32 {
        sc::CHARACTER_LOADING.get(&mem()) & 0x7fff_ffff
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
    /// `graphicFontGetEdgeRatio()`.
    graphicFontGetEdgeRatio @ 0x0064_0d70 => extern "C" fn graphic_font_get_edge_ratio() -> f32 {
        ui::bridge::font_get_edge_ratio(&mem())
    }

    // ---- managers ----
    /// `PostProcessTweakUtil_GetInstance()`.
    PostProcessTweakUtil_GetInstance @ 0x0069_2bf0 => extern "C" fn post_process_tweak_util_get_instance() -> u32 {
        instance(mg::POST_PROCESS_TWEAK)
    }
    /// `ShadowManager_GetShadowSize()`.
    ShadowManager_GetShadowSize @ 0x006c_cd00 => extern "C" fn shadow_manager_get_shadow_size() -> u32 {
        instance(mg::SHADOW_SIZE)
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

    // ---- scene state (round 5): fog, ambient, background, screen fade ----
    /// `graphicGetFogColor(r*, g*, b*)`.
    graphicGetFogColor @ 0x0064_1160 => extern "C" fn graphic_get_fog_color(r: u32, g: u32, b: u32) {
        st::get_app_color(&mut mem(), st::FOG_COLOR, [r, g, b]);
    }
    /// `graphicGetGlobalAmbientColor(r*, g*, b*)`.
    graphicGetGlobalAmbientColor @ 0x0064_1230 => extern "C" fn graphic_get_global_ambient_color(r: u32, g: u32, b: u32) {
        st::get_app_color(&mut mem(), st::AMBIENT, [r, g, b]);
    }
    /// `graphicGetFogNear()`.
    graphicGetFogNear @ 0x0064_11d0 => extern "C" fn graphic_get_fog_near() -> f32 {
        st::app_float(&mem(), st::FOG_NEAR)
    }
    /// `graphicGetFogFar()`.
    graphicGetFogFar @ 0x0064_11b0 => extern "C" fn graphic_get_fog_far() -> f32 {
        st::app_float(&mem(), st::FOG_FAR)
    }
    /// `graphicGetFogLimit()`.
    graphicGetFogLimit @ 0x0064_11c0 => extern "C" fn graphic_get_fog_limit() -> f32 {
        st::app_float(&mem(), st::FOG_LIMIT)
    }
    /// `graphicSetGlobalAmbientColor(r, g, b)`.
    graphicSetGlobalAmbientColor @ 0x0064_3270 => extern "C" fn graphic_set_global_ambient_color(r: f32, g: f32, b: f32) -> u32 {
        st::set_app_color(&mut mem(), st::AMBIENT, [r, g, b])
    }
    /// `graphicSetBackgroundColour(r, g, b)`.
    graphicSetBackgroundColour @ 0x0064_2e90 => extern "C" fn graphic_set_background_colour(r: f32, g: f32, b: f32) -> u32 {
        st::set_app_color(&mut mem(), st::BACKGROUND, [r, g, b])
    }
    /// `graphicFFXApplicationEnableKeyframeAnim(on)`.
    graphicFFXApplicationEnableKeyframeAnim @ 0x0064_01e0 => extern "C" fn graphic_enable_keyframe_anim(on: u32) -> u32 {
        st::enable_keyframe_anim(&mut mem(), on)
    }
    /// `graphicGetScreenFadeColor(r*, g*, b*, a*)`.
    graphicGetScreenFadeColor @ 0x0064_1390 => extern "C" fn graphic_get_screen_fade_color(r: u32, g: u32, b: u32, a: u32) -> u32 {
        st::get_screen_fade_color(&mut mem(), [r, g, b, a])
    }
    /// `graphicGetScreenFadeLastReason()`.
    graphicGetScreenFadeLastReason @ 0x0064_13e0 => extern "C" fn graphic_get_screen_fade_last_reason() -> u32 {
        st::screen_fade_last_reason(&mem())
    }
    /// `graphicIsScreenFadeColorLocked()`.
    graphicIsScreenFadeColorLocked @ 0x0064_1cb0 => extern "C" fn graphic_is_screen_fade_color_locked() -> u32 {
        st::is_screen_fade_color_locked(&mem())
    }
    /// `graphicLockScreenFadeColor()`.
    graphicLockScreenFadeColor @ 0x0064_1e20 => extern "C" fn graphic_lock_screen_fade_color() -> u32 {
        st::set_post_process_byte(&mut mem(), st::FADE_LOCKED, 1)
    }
    /// `graphicUnlockScreenFadeColor()`.
    graphicUnlockScreenFadeColor @ 0x0064_48d0 => extern "C" fn graphic_unlock_screen_fade_color() -> u32 {
        st::unlock_screen_fade_color(&mut mem())
    }
    /// `graphicSetScreenFadeAlphaMode(mode)`.
    graphicSetScreenFadeAlphaMode @ 0x0064_38e0 => extern "C" fn graphic_set_screen_fade_alpha_mode(mode: u32) -> u32 {
        st::set_screen_fade_alpha_mode(&mut mem(), mode)
    }
    /// `graphicSetScreenFadeColor(r, g, b, a)`.
    graphicSetScreenFadeColor @ 0x0064_3900 => extern "C" fn graphic_set_screen_fade_color(r: f32, g: f32, b: f32, a: f32) -> u32 {
        st::set_screen_fade_color(&mut mem(), [r, g, b, a])
    }
    /// `graphicSetScreenFadeLastReason(reason)`.
    graphicSetScreenFadeLastReason @ 0x0064_3940 => extern "C" fn graphic_set_screen_fade_last_reason(reason: u32) -> u32 {
        st::set_screen_fade_last_reason(&mut mem(), reason)
    }
    /// `graphicHasRunDrawFilterInThisFrame()`.
    graphicHasRunDrawFilterInThisFrame @ 0x0064_1660 => extern "C" fn graphic_has_run_draw_filter() -> u32 {
        st::set_post_process_byte(&mut mem(), st::RAN_DRAW_FILTER, 1)
    }
    /// `graphicHasRunDrawFocusInThisFrame()`.
    graphicHasRunDrawFocusInThisFrame @ 0x0064_1670 => extern "C" fn graphic_has_run_draw_focus() -> u32 {
        st::set_post_process_byte(&mut mem(), st::RAN_DRAW_FOCUS, 1)
    }
    /// `graphicResetHasRunDrawFocusInThisFrame()`.
    graphicResetHasRunDrawFocusInThisFrame @ 0x0064_2c70 => extern "C" fn graphic_reset_has_run_draw_focus() -> u32 {
        st::set_post_process_byte(&mut mem(), st::RAN_DRAW_FOCUS, 0)
    }
    /// `graphicSetFocusZ(which, z)`.
    graphicSetFocusZ @ 0x0064_30f0 => extern "C" fn graphic_set_focus_z(which: u32, z: u32) -> u32 {
        st::set_focus_z(&mut mem(), which, z)
    }

    // ---- scene state: wrappers ----
    /// `graphicPreviousLevelGetNameHash()`.
    graphicPreviousLevelGetNameHash @ 0x0064_27e0 => extern "C" fn graphic_previous_level_get_name_hash() -> u32 {
        st::scene_call(&mut mem(), st::SCENE_PREVIOUS_LEVEL_NAME_HASH, &[])
    }
    /// `graphicIs3DScene()`.
    graphicIs3DScene @ 0x0064_1b90 => extern "C" fn graphic_is_3d_scene() -> u32 {
        st::scene_call(&mut mem(), st::SCENE_IS_3D, &[])
    }
    /// `graphicIsInCut2DScene()`.
    graphicIsInCut2DScene @ 0x0064_1c00 => extern "C" fn graphic_is_in_cut_2d_scene() -> u32 {
        st::scene_call(&mut mem(), st::SCENE_IS_DRAWING_NON_UI_2D, &[])
    }
    /// `graphicMagicGetPath()`.
    graphicMagicGetPath @ 0x0064_2030 => extern "C" fn graphic_magic_get_path() -> u32 {
        st::scene_call(&mut mem(), st::SCENE_CURRENT_MAGIC_PATH, &[])
    }
    /// `graphicMagicGetToBeDeletePath()`.
    graphicMagicGetToBeDeletePath @ 0x0064_2040 => extern "C" fn graphic_magic_get_to_be_delete_path() -> u32 {
        st::scene_call(&mut mem(), st::SCENE_TO_BE_DELETE_MAGIC_PATH, &[])
    }
    /// `graphicDttPageFlag(v)`.
    graphicDttPageFlag @ 0x0063_fc10 => extern "C" fn graphic_dtt_page_flag(v: u32) -> u32 {
        st::scene_call(&mut mem(), st::SCENE_SET_BYTE_01, &[v])
    }
    /// `graphicCharacterInstanceRemoveFromPipeline(chr)`.
    graphicCharacterInstanceRemoveFromPipeline @ 0x0063_b5b0 => extern "C" fn graphic_character_remove_from_pipeline(chr: u32) -> u32 {
        st::scene_call(&mut mem(), st::SCENE_REMOVE_CHR_FROM_PIPELINE, &[chr])
    }
    /// `graphicIsAllSceneAssetsLoaded()`.
    graphicIsAllSceneAssetsLoaded @ 0x0064_1ba0 => extern "C" fn graphic_is_all_scene_assets_loaded() -> u32 {
        st::is_all_scene_assets_loaded(&mut mem())
    }
    /// `graphicSetChrFogFade(on)`.
    graphicSetChrFogFade @ 0x0064_2f10 => extern "C" fn graphic_set_chr_fog_fade(on: u32) -> u32 {
        st::set_chr_fog_fade(&mut mem(), on)
    }
    /// `graphicSet2DStartLayer(layer)`.
    graphicSet2DStartLayer @ 0x0064_2d90 => extern "C" fn graphic_set_2d_start_layer(layer: u32) -> u32 {
        st::set_2d_start_layer(&mut mem(), layer)
    }
    /// `graphicFontSetUiShadowType(kind)`.
    graphicFontSetUiShadowType @ 0x0064_0dd0 => extern "C" fn graphic_font_set_ui_shadow_type(kind: u32) {
        st::font_set_ui_shadow_type(&mut mem(), kind);
    }
    /// `graphicPushTip(id)`.
    graphicPushTip @ 0x0064_2910 => extern "C" fn graphic_push_tip(id: u32) -> u32 {
        st::push_tip(&mut mem(), st::TIP_PUSH, id)
    }
    /// `graphicPushTipByLocId(id)`.
    graphicPushTipByLocId @ 0x0064_2930 => extern "C" fn graphic_push_tip_by_loc_id(id: u32) -> u32 {
        st::push_tip(&mut mem(), st::TIP_PUSH_BY_LOC_ID, id)
    }
    /// `graphicConvertScreenCoords(x, y, *fx, *fy)`.
    graphicConvertScreenCoords @ 0x0063_de10 => extern "C" fn graphic_convert_screen_coords(x: u32, y: u32, fx: u32, fy: u32) -> u32 {
        st::convert_screen_coords(&mut mem(), x, y, fx, fy)
    }
    /// `graphicGuidemapSetup(lo, hi)`.
    graphicGuidemapSetup @ 0x0064_1490 => extern "C" fn graphic_guidemap_setup(lo: u32, hi: u32) -> u32 {
        st::guidemap_setup(&mut mem(), lo, hi)
    }
    /// `graphicGuidemapUpdate(lo, hi)`: `graphicVFXUpdateWithType(lo, hi, 3)`.
    graphicGuidemapUpdate @ 0x0064_14f0 => extern "C" fn graphic_guidemap_update(lo: u32, hi: u32) {
        vfx::bridge::update_with_type(&mut mem(), lo, hi, 3);
    }

    // ---- scene state: character lighting and instances ----
    /// `graphicCharacterLightingSetLightDir(light, x, y, z)`.
    graphicCharacterLightingSetLightDir @ 0x0063_cb50 => extern "C" fn graphic_lighting_set_light_dir(light: u32, x: f32, y: f32, z: f32) -> u32 {
        st::lighting_set_light_dir(&mut mem(), light, [x, y, z])
    }
    /// `graphicCharacterLightingSetMapIdFlagChanged(a, b)`.
    graphicCharacterLightingSetMapIdFlagChanged @ 0x0063_cd20 => extern "C" fn graphic_lighting_set_map_id_flag_changed(a: u32, b: u32) -> u32 {
        st::lighting_set_map_id_flag_changed(&mut mem(), a, b)
    }
    /// `graphicCharacterLightingSetPointLightColor(light, r, g, b)`.
    graphicCharacterLightingSetPointLightColor @ 0x0063_cd40 => extern "C" fn graphic_lighting_set_point_light_color(light: u32, r: f32, g: f32, b: f32) -> u32 {
        st::lighting_set_point_light3(&mut mem(), st::LIGHTING_SET_POINT_LIGHT_COLOR, light, [r, g, b])
    }
    /// `graphicCharacterLightingSetPointLightPosition(light, x, y, z)`.
    graphicCharacterLightingSetPointLightPosition @ 0x0063_cd70 => extern "C" fn graphic_lighting_set_point_light_position(light: u32, x: f32, y: f32, z: f32) -> u32 {
        st::lighting_set_point_light3(&mut mem(), st::LIGHTING_SET_POINT_LIGHT_POSITION, light, [x, y, z])
    }
    /// `graphicCharacterLightingSetPointLightRange(light, near, far)`.
    graphicCharacterLightingSetPointLightRange @ 0x0063_cda0 => extern "C" fn graphic_lighting_set_point_light_range(light: u32, near: f32, far: f32) -> u32 {
        st::lighting_set_point_light_range(&mut mem(), light, [near, far])
    }
    /// `graphicCharacterLightingSetShadeColor(list, r, g, b)`.
    graphicCharacterLightingSetShadeColor @ 0x0063_d050 => extern "C" fn graphic_lighting_set_shade_color(list: u32, r: f32, g: f32, b: f32) {
        st::lighting_set_shade_color(&mut mem(), list, [r, g, b]);
    }
    /// `graphicCharacterInstanceSetVisible(list, visible)`.
    graphicCharacterInstanceSetVisible @ 0x0063_c390 => extern "C" fn graphic_character_set_visible(list: u32, visible: u32) {
        st::character_set_visible(&mut mem(), list, visible);
    }
    /// `graphicCharacterInstanceSetSkeletonMatrices(chr, list, bones)`.
    graphicCharacterInstanceSetSkeletonMatrices @ 0x0063_be60 => extern "C" fn graphic_character_set_skeleton_matrices(chr: u32, list: u32, bones: u32) {
        st::character_set_skeleton_matrices(&mut mem(), chr, list, bones);
    }
    /// `graphicCharacterInstanceSetSkeletonMatricesRestoreDefault(list, bone)`.
    graphicCharacterInstanceSetSkeletonMatricesRestoreDefault @ 0x0063_bfc0 => extern "C" fn graphic_character_restore_skeleton_matrix(list: u32, bone: u32) {
        st::character_restore_skeleton_matrix(&mut mem(), list, bone);
    }

    // ---- scene state: cameras and 2D layers ----
    /// `graphicCameraSetWorldMatrix(view)`.
    graphicCameraSetWorldMatrix @ 0x0063_9860 => extern "C" fn graphic_camera_set_world_matrix(view: u32) {
        st::camera_set_world_matrix(&mut mem(), view);
    }
    /// `PhyreScene_setCameraWorldMatrix(this, m, alt)`.
    PhyreScene_setCameraWorldMatrix @ 0x0066_ce70 => extern "thiscall" fn scene_set_camera_world_matrix(this: u32, m: u32, alt: u32) -> u32 {
        st::scene_set_camera_world_matrix(&mut mem(), this, m, alt)
    }
    /// `PhyreScene_setMinimapCameraWorldMatrix(this, m)`.
    PhyreScene_setMinimapCameraWorldMatrix @ 0x0066_d940 => extern "thiscall" fn scene_set_minimap_camera_world_matrix(this: u32, m: u32) {
        st::scene_set_minimap_camera_world_matrix(&mut mem(), this, m);
    }
    /// `PhyreScene_setCameraFOVy(this, fovy)`.
    PhyreScene_setCameraFOVy @ 0x0066_cda0 => extern "thiscall" fn scene_set_camera_fovy(this: u32, fovy: u32) -> u32 {
        st::scene_set_camera_fovy(&mut mem(), this, fovy)
    }
    /// `PhyreScene_setCameraAspectRatio(this, aspect)`.
    PhyreScene_setCameraAspectRatio @ 0x0066_cbe0 => extern "thiscall" fn scene_set_camera_aspect_ratio(this: u32, aspect: u32) -> u32 {
        st::scene_set_camera_aspect_ratio(&mut mem(), this, aspect)
    }
    /// `PhyreScene_setAbmapCamera(this, x, y, z, height, scale)`.
    PhyreScene_setAbmapCamera @ 0x0066_ca60 => extern "thiscall" fn scene_set_abmap_camera(this: u32, x: u32, y: u32, z: u32, height: u32, scale: u32) -> u32 {
        st::scene_set_abmap_camera(&mut mem(), this, [x, y, z, height, scale])
    }
    /// `PhyreScene_show2DLayer(this, id)`.
    PhyreScene_show2DLayer @ 0x0066_f2f0 => extern "thiscall" fn scene_show_2d_layer(this: u32, id: u32) {
        st::scene_set_2d_layer_shown(&mut mem(), this, id, true);
    }
    /// `PhyreScene_hide2DLayer(this, id)`.
    PhyreScene_hide2DLayer @ 0x0065_f050 => extern "thiscall" fn scene_hide_2d_layer(this: u32, id: u32) {
        st::scene_set_2d_layer_shown(&mut mem(), this, id, false);
    }
    /// `PhyreScene_setCurrentLevelName(this, name)`.
    PhyreScene_setCurrentLevelName @ 0x0066_cf80 => extern "thiscall" fn scene_set_current_level_name(this: u32, name: u32) -> u32 {
        st::scene_set_current_level_name(&mut mem(), this, name)
    }
    /// `setFarPlane(far)`.
    setFarPlane @ 0x0064_5e90 => extern "C" fn set_far_plane(far: u32) -> u32 {
        st::set_far_plane(&mut mem(), far)
    }
    /// `PhyreScene_setMeshInstanceWorldMatrix(mesh, m)`.
    PhyreScene_setMeshInstanceWorldMatrix @ 0x0066_d810 => extern "stdcall" fn scene_set_mesh_instance_world_matrix(mesh: u32, m: u32) -> u32 {
        st::scene_set_mesh_instance_world_matrix(&mut mem(), mesh, m)
    }
    /// `graphicSetTextureVideoInfo(a, b)`.
    graphicSetTextureVideoInfo @ 0x0064_3aa0 => extern "C" fn graphic_set_texture_video_info(a: u32, b: u32) -> u32 {
        st::set_texture_video_info(&mut mem(), a, b)
    }
    /// `graphicUpdateCrossFade(weight, time)`.
    graphicUpdateCrossFade @ 0x0064_4910 => extern "C" fn graphic_update_cross_fade(weight: u32, time: u32) -> u32 {
        st::update_cross_fade(&mut mem(), weight, time)
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
