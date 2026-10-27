//! `ffx_prender::m5r_*`: M5 stage 1, list `render` (the scene code the
//! static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (the frame's update and `graphicFinalize`),
//! and the loader's `asyncFFXInitialize` job for the string loads; the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_prender::m5r_a::bridge as a;
use ffx_prender::m5r_b::bridge as b;
use ffx_prender::m5r_c::bridge as c;

ported! {
    // ---- batch 1 (ffx_prender::m5r_a) ----
    /// `graphicSceneAnimate_00639300()`.
    graphicSceneAnimate_00639300 @ 0x0063_9300 => extern "C" fn scene_animate() -> u32 {
        a::scene_animate(&mut mem())
    }
    /// `PostProcessManager_resizeTarget_006980a0(this; w, h)` -> 0.
    PostProcessManager_resizeTarget_006980a0 @ 0x0069_80a0 => extern "thiscall" fn post_resize_target(this: u32, w: u32, h: u32) -> u32 {
        a::post_resize_target(&mut mem(), this, w, h)
    }
    /// `PhyreScene_resizePostTarget_0066c670(scene)`.
    PhyreScene_resizePostTarget_0066c670 @ 0x0066_c670 => extern "thiscall" fn scene_resize_post_target(this: u32) -> u32 {
        a::scene_resize_post_target(&mut mem(), this)
    }
    /// `graphicResizePostTarget_00642ce0()`.
    graphicResizePostTarget_00642ce0 @ 0x0064_2ce0 => extern "C" fn resize_post_target() -> u32 {
        a::resize_post_target(&mut mem())
    }
    /// `RenderDelegationHelper_remove_006ae240(this; delegation)`.
    RenderDelegationHelper_remove_006ae240 @ 0x006a_e240 => extern "thiscall" fn delegations_remove(this: u32, delegation: u32) -> u32 {
        a::delegations_remove(&mut mem(), this, delegation)
    }
    /// `RenderDelegationHelper_removeFlag1000_006ae530()`.
    RenderDelegationHelper_removeFlag1000_006ae530 @ 0x006a_e530 => extern "C" fn delegations_remove_flag_1000() -> u32 {
        a::delegations_remove_flag_1000(&mut mem())
    }
    /// `graphicGetFreeCameraType()`.
    graphicGetFreeCameraType @ 0x0064_11e0 => extern "C" fn free_camera_type() -> u32 {
        a::free_camera_type(&mut mem())
    }
    /// `Array_Reserve_00667e00(array; n)`, 4-byte elements.
    Array_Reserve_00667e00 @ 0x0066_7e00 => extern "thiscall" fn reserve_00667e00(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00667e80(array; n)`, 4-byte elements.
    Array_Reserve_00667e80 @ 0x0066_7e80 => extern "thiscall" fn reserve_00667e80(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00667f80(array; n)`, 4-byte elements.
    Array_Reserve_00667f80 @ 0x0066_7f80 => extern "thiscall" fn reserve_00667f80(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00668000(array; n)`, 4-byte elements.
    Array_Reserve_00668000 @ 0x0066_8000 => extern "thiscall" fn reserve_00668000(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00668080(array; n)`, 4-byte elements.
    Array_Reserve_00668080 @ 0x0066_8080 => extern "thiscall" fn reserve_00668080(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00668100(array; n)`, 4-byte elements.
    Array_Reserve_00668100 @ 0x0066_8100 => extern "thiscall" fn reserve_00668100(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00668200(array; n)`, 20-byte elements.
    Array_Reserve_00668200 @ 0x0066_8200 => extern "thiscall" fn reserve_00668200(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 20)
    }
    /// `Array_Reserve_00668280(array; n)`, 16-byte elements.
    Array_Reserve_00668280 @ 0x0066_8280 => extern "thiscall" fn reserve_00668280(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 16)
    }
    /// `PhyreScene_clearSpecialRenderList_00654b70(scene)` -> 0.
    PhyreScene_clearSpecialRenderList_00654b70 @ 0x0065_4b70 => extern "thiscall" fn clear_special_render_list(this: u32) -> u32 {
        a::clear_special_render_list(&mut mem(), this)
    }
    /// `graphicClearSpecialRenderList()`.
    graphicClearSpecialRenderList @ 0x0064_4dd0 => extern "C" fn graphic_clear_special_render_list() -> u32 {
        a::graphic_clear_special_render_list(&mut mem())
    }
    /// `AtlFindStringResourceInstance_0064f970(id, language)`.
    AtlFindStringResourceInstance_0064f970 @ 0x0064_f970 => extern "C" fn find_string_resource_instance(id: u32, language: u32) -> u32 {
        a::find_string_resource_instance(&mut mem(), id, language)
    }
    /// `ThrowStdException_0064fa50(hresult)`: throws (its argument slot is
    /// the exception object).
    ThrowStdException_0064fa50 @ 0x0064_fa50 => extern "C" fn throw(hresult: u32; slots: u32) -> u32 {
        a::throw(&mut mem(), slots, hresult)
    }
    /// `AtlCrtErrorCheck_0064f8d0(errno)`.
    AtlCrtErrorCheck_0064f8d0 @ 0x0064_f8d0 => extern "C" fn crt_error_check(errno: u32) -> u32 {
        a::crt_error_check(&mut mem(), errno)
    }
    /// `AtlThrowOutOfMemory_00654950()`.
    AtlThrowOutOfMemory_00654950 @ 0x0065_4950 => extern "C" fn throw_out_of_memory() -> u32 {
        a::throw_out_of_memory(&mut mem())
    }
    /// `CSimpleString_Fork_00651460(this; n)` -> `this`.
    CSimpleString_Fork_00651460 @ 0x0065_1460 => extern "thiscall" fn string_fork(this: u32, n: u32) -> u32 {
        a::string_fork(&mut mem(), this, n)
    }
    /// `CSimpleString_Reallocate_006528d0(this; n)` -> the new data.
    CSimpleString_Reallocate_006528d0 @ 0x0065_28d0 => extern "thiscall" fn string_reallocate(this: u32, n: u32) -> u32 {
        a::string_reallocate(&mut mem(), this, n)
    }
    /// `CSimpleString_PrepareWrite2_00652600(this; n)`.
    CSimpleString_PrepareWrite2_00652600 @ 0x0065_2600 => extern "thiscall" fn string_prepare_write2(this: u32, n: u32) -> u32 {
        a::string_prepare_write2(&mut mem(), this, n)
    }
    /// `CStringW_LoadString_00652310(this; instance, id)` -> 1 or 0.
    CStringW_LoadString_00652310 @ 0x0065_2310 => extern "thiscall" fn load_string(this: u32, instance: u32, id: u32) -> u32 {
        a::load_string(&mut mem(), this, instance, id)
    }
    /// `TextureAnimationManager_unregisterAll_006b1b60(this)`.
    TextureAnimationManager_unregisterAll_006b1b60 @ 0x006b_1b60 => extern "thiscall" fn texanim_unregister_all(this: u32) {
        a::texanim_unregister_all(&mut mem(), this);
    }
    /// `ClassLoadedCharacterTable_clear_006a11d0(this)`.
    ClassLoadedCharacterTable_clear_006a11d0 @ 0x006a_11d0 => extern "thiscall" fn loaded_characters_clear(this: u32) -> u32 {
        a::loaded_characters_clear(&mut mem(), this)
    }
    /// `ClassCharacter_clearInstanceList(this)`.
    ClassCharacter_clearInstanceList @ 0x006a_1200 => extern "thiscall" fn character_clear_instances(this: u32) -> u32 {
        a::character_clear_instances(&mut mem(), this)
    }
    /// `graphicCharacterDestroyAll_006378b0()`.
    graphicCharacterDestroyAll_006378b0 @ 0x0063_78b0 => extern "C" fn character_destroy_all() -> u32 {
        a::character_destroy_all(&mut mem())
    }
    /// `PostProcessManager_release_00698f10(this)`.
    PostProcessManager_release_00698f10 @ 0x0069_8f10 => extern "thiscall" fn post_release(this: u32) -> u32 {
        a::post_release(&mut mem(), this)
    }
    /// `TextUtilAsset_release_006f5c40(this)`.
    TextUtilAsset_release_006f5c40 @ 0x006f_5c40 => extern "thiscall" fn text_asset_release(this: u32) {
        a::text_asset_release(&mut mem(), this);
    }
    /// `PostProcessTweakUtil_terminate(this)`.
    PostProcessTweakUtil_terminate @ 0x0069_8f60 => extern "thiscall" fn tweak_terminate(this: u32) -> u32 {
        a::tweak_terminate(&mut mem(), this)
    }
    /// `MorphManager_clear_006acbd0(this)`.
    MorphManager_clear_006acbd0 @ 0x006a_cbd0 => extern "thiscall" fn morph_clear(this: u32) {
        a::morph_clear(&mut mem(), this);
    }
    /// `BootText_releaseBuffer_006ddfa0(this)`.
    BootText_releaseBuffer_006ddfa0 @ 0x006d_dfa0 => extern "thiscall" fn boot_text_release_buffer(this: u32) {
        a::boot_text_release_buffer(&mut mem(), this);
    }
    /// `TextUtilEntry_dtor_00692950(this)`.
    TextUtilEntry_dtor_00692950 @ 0x0069_2950 => extern "thiscall" fn text_entry_dtor(this: u32) -> u32 {
        a::text_entry_dtor(&mut mem(), this)
    }
    /// `BootText_terminate_006ddfd0(this)`.
    BootText_terminate_006ddfd0 @ 0x006d_dfd0 => extern "thiscall" fn boot_text_terminate(this: u32) {
        a::boot_text_terminate(&mut mem(), this);
    }
    /// `PhyreScene_releaseClusterfa70_006708e0(scene)` -> 0.
    PhyreScene_releaseClusterfa70_006708e0 @ 0x0067_08e0 => extern "thiscall" fn release_cluster_fa70(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa70)
    }
    /// `PhyreScene_releaseClusterfa74_00670920(scene)` -> 0.
    PhyreScene_releaseClusterfa74_00670920 @ 0x0067_0920 => extern "thiscall" fn release_cluster_fa74(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa74)
    }
    /// `PhyreScene_releaseClusterfa7c_00670ea0(scene)` -> 0.
    PhyreScene_releaseClusterfa7c_00670ea0 @ 0x0067_0ea0 => extern "thiscall" fn release_cluster_fa7c(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa7c)
    }
    /// `PhyreScene_releaseClusterfa80_006710a0(scene)` -> 0.
    PhyreScene_releaseClusterfa80_006710a0 @ 0x0067_10a0 => extern "thiscall" fn release_cluster_fa80(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa80)
    }
    /// `PhyreScene_releaseClusterfa84_006710e0(scene)` -> 0.
    PhyreScene_releaseClusterfa84_006710e0 @ 0x0067_10e0 => extern "thiscall" fn release_cluster_fa84(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa84)
    }
    /// `PhyreScene_releaseClusterfa88_00671140(scene)` -> 0.
    PhyreScene_releaseClusterfa88_00671140 @ 0x0067_1140 => extern "thiscall" fn release_cluster_fa88(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa88)
    }
    /// `PhyreScene_releaseClusterfa8c_00671180(scene)` -> 0.
    PhyreScene_releaseClusterfa8c_00671180 @ 0x0067_1180 => extern "thiscall" fn release_cluster_fa8c(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa8c)
    }
    /// `PhyreScene_releaseClusterfa78_006711c0(scene)` -> 0.
    PhyreScene_releaseClusterfa78_006711c0 @ 0x0067_11c0 => extern "thiscall" fn release_cluster_fa78(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa78)
    }
    /// `PhyreScene_releaseClusterfa90_00671200(scene)` -> 0.
    PhyreScene_releaseClusterfa90_00671200 @ 0x0067_1200 => extern "thiscall" fn release_cluster_fa90(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa90)
    }
    /// `PhyreScene_releaseClusterfa94_00671240(scene)` -> 0.
    PhyreScene_releaseClusterfa94_00671240 @ 0x0067_1240 => extern "thiscall" fn release_cluster_fa94(this: u32) -> u32 {
        a::scene_release_cluster(&mut mem(), this, 0xfa94)
    }
    // ---- batch 2 (ffx_prender::m5r_b) ----
    /// `graphicMenuUnloadExtraTexture()`.
    graphicMenuUnloadExtraTexture @ 0x0064_2380 => extern "C" fn menu_unload_extra_texture() -> u32 {
        b::menu_unload_extra_texture(&mut mem())
    }
    /// `MenuTextureTree_release_006a1230(this)` -> 0.
    MenuTextureTree_release_006a1230 @ 0x006a_1230 => extern "thiscall" fn menu_tree_release(this: u32) -> u32 {
        b::menu_tree_release(&mut mem(), this)
    }
    /// `PostProcessManager_dtor_00692820(this)`.
    PostProcessManager_dtor_00692820 @ 0x0069_2820 => extern "thiscall" fn post_manager_dtor(this: u32) -> u32 {
        b::post_manager_dtor(&mut mem(), this)
    }
    /// `PostProcessManager_destroyInstance_00692b90()`.
    PostProcessManager_destroyInstance_00692b90 @ 0x0069_2b90 => extern "C" fn post_manager_destroy() {
        b::post_manager_destroy(&mut mem());
    }
    /// `ClassDynamicGeometry_clear(this)`.
    ClassDynamicGeometry_clear @ 0x006a_1040 => extern "thiscall" fn dyn_geo_clear(this: u32) {
        b::dyn_geo_clear(&mut mem(), this);
    }
    /// `RenderDelegationHelper_clearDelegations_006adf70(this; word)` (the
    /// word is never read).
    RenderDelegationHelper_clearDelegations_006adf70 @ 0x006a_df70 => extern "thiscall" fn clear_delegations(this: u32, _word: u32) -> u32 {
        b::clear_delegations(&mut mem(), this)
    }
    /// `RenderDelegationHelper_dtor_006ad180(this)`.
    RenderDelegationHelper_dtor_006ad180 @ 0x006a_d180 => extern "thiscall" fn delegations_dtor(this: u32) {
        b::delegations_dtor(&mut mem(), this);
    }
    /// `TextureAnimationManager_destroyInstance_006b0120()`.
    TextureAnimationManager_destroyInstance_006b0120 @ 0x006b_0120 => extern "C" fn texanim_destroy() {
        b::texanim_destroy(&mut mem());
    }
    /// `VfxShaderManager_releaseClusters_006c1f20(this)`.
    VfxShaderManager_releaseClusters_006c1f20 @ 0x006c_1f20 => extern "thiscall" fn vfx_shader_release_clusters(this: u32) -> u32 {
        b::vfx_shader_release_clusters(&mut mem(), this)
    }
    /// `Distortion_releaseBuffers_006c2410(this)`.
    Distortion_releaseBuffers_006c2410 @ 0x006c_2410 => extern "thiscall" fn distortion_release(this: u32) {
        b::distortion_release(&mut mem(), this);
    }
    /// `Distortion_terminate(this)` -> 0.
    Distortion_terminate @ 0x006c_31c0 => extern "thiscall" fn distortion_terminate(this: u32) -> u32 {
        b::distortion_terminate(&mut mem(), this)
    }
    /// `Distortion_destroyInstance_006c2210()`.
    Distortion_destroyInstance_006c2210 @ 0x006c_2210 => extern "C" fn distortion_destroy() {
        b::distortion_destroy(&mut mem());
    }
    /// `RadialLine_destroyRadialLine(this)`.
    RadialLine_destroyRadialLine @ 0x006f_7b20 => extern "thiscall" fn radial_line_destroy(this: u32) {
        b::radial_line_destroy(&mut mem(), this);
    }
    /// `RadialBlur_release_006c3cd0(this)` -> 0.
    RadialBlur_release_006c3cd0 @ 0x006c_3cd0 => extern "thiscall" fn radial_blur_release(this: u32) -> u32 {
        b::radial_blur_release(&mut mem(), this)
    }
    /// `RadialBlur_destroyInstance_006c3550()`.
    RadialBlur_destroyInstance_006c3550 @ 0x006c_3550 => extern "C" fn radial_blur_destroy() {
        b::radial_blur_destroy(&mut mem());
    }
    /// `BlurEx_destroyInstance_006c4a30()`.
    BlurEx_destroyInstance_006c4a30 @ 0x006c_4a30 => extern "C" fn blur_ex_destroy() {
        b::blur_ex_destroy(&mut mem());
    }
    /// `PostEffect_destroyInstance_006c50b0()`.
    PostEffect_destroyInstance_006c50b0 @ 0x006c_50b0 => extern "C" fn post_effect_destroy() {
        b::post_effect_destroy(&mut mem());
    }
    /// `SpecularMapParamsManager_delete_006c57c0()`.
    SpecularMapParamsManager_delete_006c57c0 @ 0x006c_57c0 => extern "C" fn specular_map_params_delete() -> u32 {
        b::specular_map_params_delete(&mut mem())
    }
    /// `BrokenScreenEffect_release_006d5a00(this)` -> 0.
    BrokenScreenEffect_release_006d5a00 @ 0x006d_5a00 => extern "thiscall" fn broken_screen_release(this: u32) -> u32 {
        b::broken_screen_release(&mut mem(), this)
    }
    /// `DynGeoPool_dtor_006de420(this)`.
    DynGeoPool_dtor_006de420 @ 0x006d_e420 => extern "thiscall" fn dyn_geo_pool_dtor(this: u32) {
        b::dyn_geo_pool_dtor(&mut mem(), this);
    }
    /// `DynGeoMemManager_releasePool_006de860(this; i)`.
    DynGeoMemManager_releasePool_006de860 @ 0x006d_e860 => extern "thiscall" fn dyn_geo_release_pool(this: u32, i: u32) {
        b::dyn_geo_release_pool(&mut mem(), this, i);
    }
    /// `DynGeoMemManager_destroyInstance_006dec20()`.
    DynGeoMemManager_destroyInstance_006dec20 @ 0x006d_ec20 => extern "C" fn dyn_geo_destroy() {
        b::dyn_geo_destroy(&mut mem());
    }
    /// `graphicFinalize()` -> 0.
    graphicFinalize @ 0x0064_03e0 => extern "C" fn graphic_finalize() -> u32 {
        b::graphic_finalize(&mut mem())
    }
    /// `PhyreScene_exitRelease_0065b820(scene)` -> 0.
    PhyreScene_exitRelease_0065b820 @ 0x0065_b820 => extern "thiscall" fn scene_exit_release(this: u32) -> u32 {
        b::scene_exit_release(&mut mem(), this)
    }
    /// `graphicExitScene_006401c0()`.
    graphicExitScene_006401c0 @ 0x0064_01c0 => extern "C" fn graphic_exit_scene() -> u32 {
        b::graphic_exit_scene(&mut mem())
    }
    /// `PhyreScene_copyPostBuffer_006582a0(scene)`.
    PhyreScene_copyPostBuffer_006582a0 @ 0x0065_82a0 => extern "thiscall" fn scene_copy_post_buffer(this: u32) -> u32 {
        b::scene_copy_post_buffer(&mut mem(), this)
    }
    /// `graphicCopyPostBuffer_00639d10()`.
    graphicCopyPostBuffer_00639d10 @ 0x0063_9d10 => extern "C" fn copy_post_buffer() -> u32 {
        b::copy_post_buffer(&mut mem())
    }
    /// `graphicRenderCaptureAndIggy_00642be0()`.
    graphicRenderCaptureAndIggy_00642be0 @ 0x0064_2be0 => extern "C" fn render_capture_and_iggy() -> u32 {
        b::render_capture_and_iggy(&mut mem())
    }
    /// `PCluster_moveInstanceLists_004409e0(this; other)`.
    PCluster_moveInstanceLists_004409e0 @ 0x0044_09e0 => extern "thiscall" fn cluster_move_instance_lists(this: u32, other: u32) -> u32 {
        b::cluster_move_instance_lists(&mut mem(), this, other)
    }
    /// `PhyreCursor_seekValid_00441bf0(cursor)`.
    PhyreCursor_seekValid_00441bf0 @ 0x0044_1bf0 => extern "thiscall" fn cursor_seek_valid(this: u32) -> u32 {
        b::cursor_seek_valid(&mut mem(), this)
    }
    /// `StdString_tidy_006335a0(s)`.
    StdString_tidy_006335a0 @ 0x0063_35a0 => extern "stdcall" fn std_string_tidy(s: u32) {
        b::std_string_tidy(&mut mem(), s);
    }
    /// `LoadQueue_push_00633670(this; a, b, c, d, lane)`.
    LoadQueue_push_00633670 @ 0x0063_3670 => extern "thiscall" fn load_queue_push(this: u32, a: u32, bw: u32, c: u32, d: u32, lane: u32) {
        b::load_queue_push(&mut mem(), this, a, bw, c, d, lane);
    }
    /// `PArray_copyFrom_006337e0(this; src)` -> `this`.
    PArray_copyFrom_006337e0 @ 0x0063_37e0 => extern "thiscall" fn array_copy_from(this: u32, src: u32) -> u32 {
        b::array_copy_from(&mut mem(), this, src)
    }
    /// `PRedBlackTreePair_dtor_00633dc0(this)`.
    PRedBlackTreePair_dtor_00633dc0 @ 0x0063_3dc0 => extern "thiscall" fn tree_pair_dtor_00633dc0(this: u32) -> u32 {
        b::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_00633e20(this)`.
    PRedBlackTreePair_dtor_00633e20 @ 0x0063_3e20 => extern "thiscall" fn tree_pair_dtor_00633e20(this: u32) -> u32 {
        b::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_00633e80(this)`.
    PRedBlackTreePair_dtor_00633e80 @ 0x0063_3e80 => extern "thiscall" fn tree_pair_dtor_00633e80(this: u32) -> u32 {
        b::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreeFree_dtor_00633ef0(this)`.
    PRedBlackTreeFree_dtor_00633ef0 @ 0x0063_3ef0 => extern "thiscall" fn tree_free_dtor(this: u32) -> u32 {
        b::tree_free_dtor(&mut mem(), this)
    }
    /// `PhyreScene_ChangeToHSVPass(material, on)`.
    PhyreScene_ChangeToHSVPass @ 0x0064_faa0 => extern "C" fn change_to_hsv_pass(material: u32, on: u32) -> u32 {
        b::change_to_hsv_pass(&mut mem(), material, on)
    }
    /// `graphicEnableHSV(model, on)`: keeps the mesh index in its first
    /// argument slot.
    graphicEnableHSV @ 0x0064_0100 => extern "C" fn enable_hsv(model: u32, on: u32; slots: u32) {
        b::enable_hsv(&mut mem(), slots, model, on);
    }
    /// `graphicResetHSV_00637c10(chr)`.
    graphicResetHSV_00637c10 @ 0x0063_7c10 => extern "C" fn reset_hsv(chr: u32) {
        b::reset_hsv(&mut mem(), chr);
    }
    /// `LuaBind_pushObject_00441e70(L, p)` (stdcall).
    LuaBind_pushObject_00441e70 @ 0x0044_1e70 => extern "stdcall" fn lua_push_object(l: u32, p: u32) -> u32 {
        b::lua_push_object(&mut mem(), l, p)
    }
    // ---- batch 3 (ffx_prender::m5r_c) ----
    /// `getCurrentControlMode()` (`al`, zero-extended).
    getCurrentControlMode @ 0x0063_8e30 => extern "C" fn current_control_mode() -> u32 {
        c::current_control_mode(&mut mem())
    }
    /// `graphicGetSkipTexturePath_00638e90()`.
    graphicGetSkipTexturePath_00638e90 @ 0x0063_8e90 => extern "C" fn skip_texture_path() -> u32 {
        c::skip_texture_path(&mut mem())
    }
    /// `graphicActivateRadialBlur()`.
    graphicActivateRadialBlur @ 0x0063_9140 => extern "C" fn activate_radial_blur() -> u32 {
        c::activate_radial_blur(&mut mem())
    }
    /// `graphicAddScreenFadeTaskBeforeVFX(r, g, b, a, frames)`.
    graphicAddScreenFadeTaskBeforeVFX @ 0x0063_92c0 => extern "C" fn add_screen_fade_before_vfx(r: u32, g: u32, bl: u32, al: u32, frames: u32) -> u32 {
        c::add_screen_fade_before_vfx(&mut mem(), [r, g, bl, al], frames)
    }
    /// `PhyreScene_attachChrClusterTo3DWorld(scene)`.
    PhyreScene_attachChrClusterTo3DWorld @ 0x0065_78e0 => extern "thiscall" fn attach_chr_cluster(this: u32) {
        c::attach_chr_cluster(&mut mem(), this);
    }
    /// `PhyreScene_detachChrClusterFrom3DWorld_0065a550(scene)`.
    PhyreScene_detachChrClusterFrom3DWorld_0065a550 @ 0x0065_a550 => extern "thiscall" fn detach_chr_cluster(this: u32) {
        c::detach_chr_cluster(&mut mem(), this);
    }
    /// `graphicDetachChrClusterFrom3DWorld()`.
    graphicDetachChrClusterFrom3DWorld @ 0x0063_df50 => extern "C" fn graphic_detach_chr_cluster() -> u32 {
        c::graphic_detach_chr_cluster(&mut mem())
    }
    /// `graphicCharacterInstanceSetDrawSituationCallback(inst, cb)`.
    graphicCharacterInstanceSetDrawSituationCallback @ 0x0063_b7f0 => extern "C" fn set_draw_situation_callback(inst: u32, cb: u32) {
        c::set_draw_situation_callback(&mut mem(), inst, cb);
    }
    /// `graphicCharacterLightingSet4_0063c9e0(list, a, b, c, d)`.
    graphicCharacterLightingSet4_0063c9e0 @ 0x0063_c9e0 => extern "C" fn lighting_set_4(list: u32, f0: u32, f1: u32, f2: u32, f3: u32) {
        c::lighting_set_4(&mut mem(), list, [f0, f1, f2, f3]);
    }
    /// `graphicCharacterLightingSet8_0063ca50(list, 8 floats)`.
    graphicCharacterLightingSet8_0063ca50 @ 0x0063_ca50 => extern "C" fn lighting_set_8(list: u32, f0: u32, f1: u32, f2: u32, f3: u32, f4: u32, f5: u32, f6: u32, f7: u32) {
        c::lighting_set_8(&mut mem(), list, [f0, f1, f2, f3, f4, f5, f6, f7]);
    }
    /// `graphicCharacterReplaceSummonTexture(chr)`: the cluster name in the
    /// shim's frame (`ebp - 0x104`).
    graphicCharacterReplaceSummonTexture @ 0x0063_d400 => extern "C" fn replace_summon_texture(chr: u32) {
        let mut f = StackFrame::<0x41>::new();
        c::replace_summon_texture(&mut mem(), f.at() + 0x104, chr);
    }
    /// `PhyreScene_unloadPersistentVFXTexture(scene)`: the names in the
    /// shim's frame (`ebp - 0x204`).
    PhyreScene_unloadPersistentVFXTexture @ 0x0067_1280 => extern "thiscall" fn unload_persistent_vfx_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x81>::new();
        c::unload_persistent_vfx_texture(&mut mem(), f.at() + 0x204, this)
    }
    /// `MenuTextureManager_unloadDefaultTextures_006857c0()`: the name in
    /// the shim's frame (`ebp - 0x104`).
    MenuTextureManager_unloadDefaultTextures_006857c0 @ 0x0068_57c0 => extern "C" fn unload_default_menu_textures() -> u32 {
        let mut f = StackFrame::<0x41>::new();
        c::unload_default_menu_textures(&mut mem(), f.at() + 0x104)
    }
    /// `graphicSetHSVParam(model, h, s, v, a)`: the floats in the shim's
    /// frame (`ebp - 0x20`); keeps the mesh index in its first argument
    /// slot.
    graphicSetHSVParam @ 0x0064_3340 => extern "C" fn set_hsv_param(model: u32, f0: u32, f1: u32, f2: u32, f3: u32; slots: u32) {
        let mut f = StackFrame::<8>::new();
        c::set_hsv_param(&mut mem(), f.at() + 0x20, slots, model, [f0, f1, f2, f3]);
    }
    /// `graphicCharacterSetClipPlane(model, plane)`: keeps the mesh index
    /// in its first argument slot.
    graphicCharacterSetClipPlane @ 0x0063_d710 => extern "C" fn set_clip_plane(model: u32, plane: u32; slots: u32) {
        c::set_clip_plane(&mut mem(), slots, model, plane);
    }
    /// `graphicCharacterSetShaderType(chr, kind)` -> 0.
    graphicCharacterSetShaderType @ 0x0063_d810 => extern "C" fn set_shader_type(chr: u32, kind: u32) -> u32 {
        c::set_shader_type(&mut mem(), chr, kind)
    }
    /// `graphicCharacterShaderRestore()` -> 0.
    graphicCharacterShaderRestore @ 0x0063_d870 => extern "C" fn shader_restore() -> u32 {
        c::shader_restore(&mut mem())
    }
    /// `graphicCharacterShaderSetParameter(chr, name, value)` -> 0.
    graphicCharacterShaderSetParameter @ 0x0063_d880 => extern "C" fn shader_set_parameter(chr: u32, name: u32, value: u32) -> u32 {
        c::shader_set_parameter(&mut mem(), chr, name, value)
    }
    /// `graphicCharacterShaderSetParameterTexture(chr, name, texture)` -> 0.
    graphicCharacterShaderSetParameterTexture @ 0x0063_d8e0 => extern "C" fn shader_set_parameter_texture(chr: u32, name: u32, tex: u32) -> u32 {
        c::shader_set_parameter_texture(&mut mem(), chr, name, tex)
    }
    /// `graphicSetFreeCameraMode_00640e20(mode)`.
    graphicSetFreeCameraMode_00640e20 @ 0x0064_0e20 => extern "C" fn set_free_camera_mode(mode: u32) -> u32 {
        c::set_free_camera_mode(&mut mem(), mode)
    }
    /// `PhyreScene_getMonsterEtTextureName(scene; et, which)`.
    PhyreScene_getMonsterEtTextureName @ 0x0065_e200 => extern "thiscall" fn monster_et_texture_name(this: u32, et: u32, which: u32) -> u32 {
        c::monster_et_texture_name(&mut mem(), this, et, which)
    }
    /// `graphicGetMonsterEtTextureName(et, which)`.
    graphicGetMonsterEtTextureName @ 0x0064_1330 => extern "C" fn graphic_monster_et_texture_name(et: u32, which: u32) -> u32 {
        c::graphic_monster_et_texture_name(&mut mem(), et, which)
    }
    /// `graphicSegmentsAllReady_006522b0()`.
    graphicSegmentsAllReady_006522b0 @ 0x0065_22b0 => extern "C" fn segments_all_ready() -> u32 {
        c::segments_all_ready(&mut mem())
    }
    /// `PhyreScene_releaseCharacterClusters_00670960(scene)` -> 0: the
    /// cluster vector copy in the shim's frame (`ebp - 0x20`).
    PhyreScene_releaseCharacterClusters_00670960 @ 0x0067_0960 => extern "thiscall" fn release_character_clusters(this: u32) -> u32 {
        let mut f = StackFrame::<8>::new();
        c::release_character_clusters(&mut mem(), f.at() + 0x20, this)
    }
}
