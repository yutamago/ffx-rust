//! `ffx_prender::m5r_*`: M5 stage 1, list `render` (the scene code the
//! static call graph reaches and no coverage run ran).
//!
//! Threads: the main thread (the frame's update and `graphicFinalize`),
//! and the loader's `asyncFFXInitialize` job for the string loads; the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_graphic::host::Conv;
use ffx_prender::m5r_a::bridge as a;
use ffx_prender::m5r_b::bridge as b;
use ffx_prender::m5r_c::bridge as c;
use ffx_prender::m5r_d::bridge as d;
use ffx_prender::m5r_e::bridge as e;
use ffx_prender::m5r_f::bridge as f;
use ffx_prender::m5r_g::bridge as g;
use ffx_prender::m5r_h::bridge as h;
use ffx_prender::m5r_i::bridge as i;
use ffx_prender::m5r_j::bridge as j;
use ffx_prender::m5r_k::bridge as k;
use ffx_prender::m5r_l::{self as ml, bridge as l};
use ffx_prender::m5r_m::bridge as m;
use ffx_prender::m5r_n::bridge as n;
use ffx_prender::m5r_o::bridge as o;
use ffx_prender::m5r_p::bridge as p;

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
    // ---- batch 4 (ffx_prender::m5r_d) ----
    /// `graphicMenuSetTextureUnloadedInMaterial(x)`.
    graphicMenuSetTextureUnloadedInMaterial @ 0x0064_2360 => extern "C" fn menu_set_texture_unloaded(x: u32) -> u32 {
        d::menu_set_texture_unloaded(&mut mem(), x)
    }
    /// `graphicCharacterLoadingPush_00642980(x)` -> 0.
    graphicCharacterLoadingPush_00642980 @ 0x0064_2980 => extern "C" fn loading_push(x: u32) -> u32 {
        d::loading_push(&mut mem(), x)
    }
    /// `graphicRegisterDelegation_006429f0(a, b, c, d)`.
    graphicRegisterDelegation_006429f0 @ 0x0064_29f0 => extern "C" fn register_delegation(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        d::register_delegation(&mut mem(), [a0, a1, a2, a3])
    }
    /// `graphicRegisterResetHSV_00642a10(chr)`: its argument slot is the
    /// delegation's data.
    graphicRegisterResetHSV_00642a10 @ 0x0064_2a10 => extern "C" fn register_reset_hsv(_chr: u32; slots: u32) -> u32 {
        d::register_reset_hsv(&mut mem(), slots)
    }
    /// `graphicResetButtonIconScale()`.
    graphicResetButtonIconScale @ 0x0064_2c60 => extern "C" fn reset_button_icon_scale() -> u32 {
        d::reset_button_icon_scale(&mut mem())
    }
    /// `graphicResize_00642cd0()`.
    graphicResize_00642cd0 @ 0x0064_2cd0 => extern "C" fn resize() -> u32 {
        d::resize(&mut mem())
    }
    /// `graphicSetAkazaNoLightSpecularPassParam(a0..a8)`.
    graphicSetAkazaNoLightSpecularPassParam @ 0x0064_2e00 => extern "C" fn akaza_no_light_specular(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> u32 {
        d::akaza_no_light_specular(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8])
    }
    /// `graphicSetAkazaSpecularPassParam(a0..a11)`.
    graphicSetAkazaSpecularPassParam @ 0x0064_2e40 => extern "C" fn akaza_specular(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32, a10: u32, a11: u32) -> u32 {
        d::akaza_specular(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11])
    }
    /// `graphicSetNoLightSpecularPassParam(a0..a9)`.
    graphicSetNoLightSpecularPassParam @ 0x0064_3770 => extern "C" fn no_light_specular(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32, a9: u32) -> u32 {
        d::no_light_specular(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9])
    }
    /// `graphicSetSpecularMapParams(a0, a1, f2, f3, f4)`.
    graphicSetSpecularMapParams @ 0x0064_3980 => extern "C" fn specular_map_params(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32) -> u32 {
        d::specular_map_params(&mut mem(), [a0, a1, a2, a3, a4])
    }
    /// `graphicSetButtonIconScale(x, y)`.
    graphicSetButtonIconScale @ 0x0064_2ec0 => extern "C" fn set_button_icon_scale(x: u32, y: u32) -> u32 {
        d::set_button_icon_scale(&mut mem(), x, y)
    }
    /// `graphicSetShadowFloats_00642fb0(a, b)`.
    graphicSetShadowFloats_00642fb0 @ 0x0064_2fb0 => extern "C" fn set_shadow_floats(a0: u32, a1: u32) -> u32 {
        d::set_shadow_floats(&mut mem(), a0, a1)
    }
    /// `graphicSetFogColour(r, g, b)`.
    graphicSetFogColour @ 0x0064_3150 => extern "C" fn set_fog_colour(r: u32, g: u32, bl: u32) -> u32 {
        d::set_fog_colour(&mut mem(), [r, g, bl])
    }
    /// `graphicSetFogParam_00643180(x)`.
    graphicSetFogParam_00643180 @ 0x0064_3180 => extern "C" fn set_fog_param(x: u32) -> u32 {
        d::set_fog_float(&mut mem(), ffx_prender::m5r_d::FOG_SET_3760, x)
    }
    /// `graphicSetFogFar(x)`.
    graphicSetFogFar @ 0x0064_31a0 => extern "C" fn set_fog_far(x: u32) -> u32 {
        d::set_fog_float(&mut mem(), ffx_prender::m5r_d::FOG_SET_FAR, x)
    }
    /// `graphicSetFogNear(x)`.
    graphicSetFogNear @ 0x0064_31e0 => extern "C" fn set_fog_near(x: u32) -> u32 {
        d::set_fog_float(&mut mem(), ffx_prender::m5r_d::FOG_SET_NEAR, x)
    }
    /// `graphicSetAppFloatD8_006432c0(x)`.
    graphicSetAppFloatD8_006432c0 @ 0x0064_32c0 => extern "C" fn set_app_float_d8(x: u32) -> u32 {
        d::set_app_float(&mut mem(), 0xd8, x)
    }
    /// `graphicSetAppFloatD4_006432e0(x)`.
    graphicSetAppFloatD4_006432e0 @ 0x0064_32e0 => extern "C" fn set_app_float_d4(x: u32) -> u32 {
        d::set_app_float(&mut mem(), 0xd4, x)
    }
    /// `graphicSetAppFloatDC_00643300(x)`.
    graphicSetAppFloatDC_00643300 @ 0x0064_3300 => extern "C" fn set_app_float_dc(x: u32) -> u32 {
        d::set_app_float(&mut mem(), 0xdc, x)
    }
    /// `graphicSetAppFloatD0_00643320(x)`.
    graphicSetAppFloatD0_00643320 @ 0x0064_3320 => extern "C" fn set_app_float_d0(x: u32) -> u32 {
        d::set_app_float(&mut mem(), 0xd0, x)
    }
    /// `graphicSetSceneSix0_00643630(f0..f5)`.
    graphicSetSceneSix0_00643630 @ 0x0064_3630 => extern "C" fn scene_set_six_0(f0: u32, f1: u32, f2: u32, f3: u32, f4: u32, f5: u32) -> u32 {
        d::scene_set_six(&mut mem(), [f0, f1, f2, f3, f4, f5], 0)
    }
    /// `graphicSetSceneSix1_006436a0(f0..f5)`.
    graphicSetSceneSix1_006436a0 @ 0x0064_36a0 => extern "C" fn scene_set_six_1(f0: u32, f1: u32, f2: u32, f3: u32, f4: u32, f5: u32) -> u32 {
        d::scene_set_six(&mut mem(), [f0, f1, f2, f3, f4, f5], 1)
    }
    /// `graphicSetMeshVertexColorMorphValue(mesh, value)`.
    graphicSetMeshVertexColorMorphValue @ 0x0064_3700 => extern "C" fn set_vertex_color_morph(mesh: u32, value: u32) -> u32 {
        d::set_vertex_color_morph(&mut mem(), mesh, value)
    }
    /// `graphicSetPauseVideoAtEndFrame(on)`.
    graphicSetPauseVideoAtEndFrame @ 0x0064_37f0 => extern "C" fn set_pause_video_at_end(on: u32) -> u32 {
        d::set_pause_video_at_end(&mut mem(), on)
    }
    /// `graphicSetNumberText_006438c0(x)`.
    graphicSetNumberText_006438c0 @ 0x0064_38c0 => extern "C" fn set_number_text(x: u32) -> u32 {
        d::set_number_text(&mut mem(), x)
    }
    /// `graphicSceneMatrixTranspose_00643b80()`: the matrices in the shim's
    /// frame (`ebp - 0x80`).
    graphicSceneMatrixTranspose_00643b80 @ 0x0064_3b80 => extern "C" fn scene_matrix_transpose() -> u32 {
        let mut f = StackFrame::<0x20>::new();
        d::scene_matrix_transpose(&mut mem(), f.at() + 0x80)
    }
    // ---- batch 5 (ffx_prender::m5r_e, m5r_a's reserve, ffx_pload's pointer array resize) ----
    /// `graphicSetMeshesVisible_00643df0(on)`.
    graphicSetMeshesVisible_00643df0 @ 0x0064_3df0 => extern "C" fn set_meshes_visible(on: u32) -> u32 {
        e::set_meshes_visible(&mut mem(), on)
    }
    /// `PostProcessTweakUtil_setEnabled_006984f0(this; on)`.
    PostProcessTweakUtil_setEnabled_006984f0 @ 0x0069_84f0 => extern "thiscall" fn post_tweak_set_enabled(this: u32, on: u32) -> u32 {
        e::post_tweak_set_enabled(&mut mem(), this, on)
    }
    /// `DebugMenuManager_disableTweaks_006b49c0()`.
    DebugMenuManager_disableTweaks_006b49c0 @ 0x006b_49c0 => extern "C" fn disable_tweaks() -> u32 {
        e::disable_tweaks(&mut mem())
    }
    /// `PhyreScene_softResetScene(scene)`.
    PhyreScene_softResetScene @ 0x0066_fad0 => extern "thiscall" fn soft_reset(this: u32) -> u32 {
        e::soft_reset(&mut mem(), this)
    }
    /// `ShadowManager_resetIfSolution_006ce0c0(this; x)`.
    ShadowManager_resetIfSolution_006ce0c0 @ 0x006c_e0c0 => extern "thiscall" fn shadow_reset_if_solution(this: u32, x: u32) -> u32 {
        e::shadow_reset_if_solution(&mut mem(), this, x)
    }
    /// `graphicShadowReset_00643e80(x)` -> 1.
    graphicShadowReset_00643e80 @ 0x0064_3e80 => extern "C" fn shadow_reset(x: u32) -> u32 {
        e::shadow_reset(&mut mem(), x)
    }
    /// `graphicSetAnimatedMaterialsChar_006440e0(a0..a10)` (words 7 and 8
    /// are never read).
    graphicSetAnimatedMaterialsChar_006440e0 @ 0x0064_40e0 => extern "C" fn set_animated_materials_char(a0: u32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, _a7: u32, _a8: u32, a9: u32, a10: u32) -> u32 {
        e::set_animated_materials_char(&mut mem(), [a0, a1, a2, a3, a4, a5, a6, 0, 0, a9, a10])
    }
    /// `graphicTextureVideoPlay(x)`.
    graphicTextureVideoPlay @ 0x0064_4280 => extern "C" fn texture_video_play(x: u32) -> u32 {
        e::texture_video_play(&mut mem(), x)
    }
    /// `BlurEffect_updateRadialLine_006c3fd0(this; a, b, f, g)`.
    BlurEffect_updateRadialLine_006c3fd0 @ 0x006c_3fd0 => extern "thiscall" fn blur_update_radial_line(this: u32, a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        e::blur_update_radial_line(&mut mem(), this, [a0, a1, a2, a3])
    }
    /// `graphicUpdateRadialLineLogic(on, b, f, g)`.
    graphicUpdateRadialLineLogic @ 0x0064_49d0 => extern "C" fn update_radial_line_logic(a0: u32, a1: u32, a2: u32, a3: u32) -> u32 {
        e::update_radial_line_logic(&mut mem(), [a0, a1, a2, a3])
    }
    /// `graphicVideoGetElapsedFrame()`.
    graphicVideoGetElapsedFrame @ 0x0064_5d90 => extern "C" fn video_elapsed_frame() -> u32 {
        e::video_elapsed_frame(&mut mem())
    }
    /// `graphicSetWaveStrength_00645e40()`.
    graphicSetWaveStrength_00645e40 @ 0x0064_5e40 => extern "C" fn set_wave_strength() -> u32 {
        e::set_wave_strength(&mut mem())
    }
    /// `PhyreSceneCursor_ctorSeek2_0064cda0(this; cluster)` -> `this`.
    PhyreSceneCursor_ctorSeek2_0064cda0 @ 0x0064_cda0 => extern "thiscall" fn cursor_ctor_seek2(this: u32, cluster: u32) -> u32 {
        e::cursor_ctor(&mut mem(), this, cluster, ffx_prender::m5r_e::CURSOR_CLASS2, ffx_prender::m5r_e::CURSOR_SEEK2)
    }
    /// `PhyreSceneCursor_ctorSeek3_0064ce00(this; cluster)` -> `this`.
    PhyreSceneCursor_ctorSeek3_0064ce00 @ 0x0064_ce00 => extern "thiscall" fn cursor_ctor_seek3(this: u32, cluster: u32) -> u32 {
        e::cursor_ctor(&mut mem(), this, cluster, ffx_prender::m5r_e::CURSOR_CLASS3, ffx_prender::m5r_e::CURSOR_SEEK3)
    }
    /// `PhyreSceneCursor_ctorSeek4_0064ce60(this; cluster)` -> `this`.
    PhyreSceneCursor_ctorSeek4_0064ce60 @ 0x0064_ce60 => extern "thiscall" fn cursor_ctor_seek4(this: u32, cluster: u32) -> u32 {
        e::cursor_ctor(&mut mem(), this, cluster, ffx_prender::m5r_e::CURSOR_CLASS4, ffx_prender::m5r_e::CURSOR_SEEK4)
    }
    /// `WordArray_alloc_0064f7f0(n, src, count)`.
    WordArray_alloc_0064f7f0 @ 0x0064_f7f0 => extern "C" fn word_array_alloc_0064f7f0(n: u32, src: u32, count: u32) -> u32 {
        e::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `WordArray_alloc_0064f860(n, src, count)`.
    WordArray_alloc_0064f860 @ 0x0064_f860 => extern "C" fn word_array_alloc_0064f860(n: u32, src: u32, count: u32) -> u32 {
        e::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `AtlThrowLastError_0064fa70()`: throws.
    AtlThrowLastError_0064fa70 @ 0x0064_fa70 => extern "C" fn throw_last_error() -> u32 {
        e::throw_last_error(&mut mem())
    }
    /// `StdVector_allocate_00654d40(this; n)` (`al`).
    StdVector_allocate_00654d40 @ 0x0065_4d40 => extern "thiscall" fn vector_allocate_00654d40(this: u32, n: u32) -> u32 {
        e::vector_allocate(&mut mem(), this, n)
    }
    /// `StdVector_allocate_00654db0(this; n)` (`al`).
    StdVector_allocate_00654db0 @ 0x0065_4db0 => extern "thiscall" fn vector_allocate_00654db0(this: u32, n: u32) -> u32 {
        e::vector_allocate(&mut mem(), this, n)
    }
    /// `PhyreScene_restoreLoadingCluster_0065a590(scene)`.
    PhyreScene_restoreLoadingCluster_0065a590 @ 0x0065_a590 => extern "thiscall" fn restore_loading_cluster(this: u32) {
        e::restore_loading_cluster(&mut mem(), this);
    }
    /// `PhyreScene_loadWaterTexture_00665fd0(scene)`: the name in the
    /// shim's frame (`ebp - 0x104`).
    PhyreScene_loadWaterTexture_00665fd0 @ 0x0066_5fd0 => extern "thiscall" fn load_water_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        e::load_water_texture(&mut mem(), f.at() + 0x104, this)
    }
    /// `PhyreScene_onFontTextureLoaded(cluster)`: its argument slot is
    /// handed on.
    PhyreScene_onFontTextureLoaded @ 0x0066_66a0 => extern "C" fn on_font_texture_loaded(_cluster: u32; slots: u32) -> u32 {
        e::on_font_texture_loaded(&mut mem(), slots)
    }
    /// `Array_Reserve_00667f00(array; n)`, 4-byte elements.
    Array_Reserve_00667f00 @ 0x0066_7f00 => extern "thiscall" fn reserve_00667f00(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 4)
    }
    /// `Array_Reserve_00668180(array; n)`, 8-byte elements.
    Array_Reserve_00668180 @ 0x0066_8180 => extern "thiscall" fn reserve_00668180(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 8)
    }
    /// `Array_Reserve_00668300(array; n)`, 8-byte elements.
    Array_Reserve_00668300 @ 0x0066_8300 => extern "thiscall" fn reserve_00668300(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 8)
    }
    /// `Array_Reserve_00668380(array; n)`, 1-byte elements.
    Array_Reserve_00668380 @ 0x0066_8380 => extern "thiscall" fn reserve_00668380(this: u32, n: u32) -> u32 {
        a::reserve(&mut mem(), this, n, 1)
    }
    /// `PArray_ptr_resize_0066c240(this; n)`: the same code as
    /// `Phyre_PArray_ptr_resize_0047efc0`.
    PArray_ptr_resize_0066c240 @ 0x0066_c240 => extern "thiscall" fn pointer_array_resize_0066c240(this: u32, n: u32) -> u32 {
        ffx_pload::descriptor::bridge::pointer_array_resize(&mut mem(), this, n)
    }
    // ---- batch 6 (ffx_prender::m5r_f): the dynamic meshes' class descriptors and script bindings ----
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_Destruct_00673fc0`.
    PClassDescriptorForType_DistortionGridDynamicMesh_Destruct_00673fc0 @ 0x0067_3fc0 => extern "thiscall" fn destruct_00673fc0(this: u32) -> u32 {
        f::descriptor_destruct(&mut mem(), this, ffx_prender::m5r_f::VT_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674020`.
    PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674020 @ 0x0067_4020 => extern "thiscall" fn scalardeletingdtor_00674020(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674050`.
    PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674050 @ 0x0067_4050 => extern "thiscall" fn scalardeletingdtor_00674050(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674080`.
    PClassDescriptorForType_DistortionGridDynamicMesh_scalarDeletingDtor_00674080 @ 0x0067_4080 => extern "thiscall" fn scalardeletingdtor_00674080(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_virtualStore_00674650`.
    PClassDescriptorForType_DistortionGridDynamicMesh_virtualStore_00674650 @ 0x0067_4650 => extern "thiscall" fn virtualstore_00674650(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, out: u32) -> u32 {
        f::virtual_store(&mut mem(), this, [a0, a1, a2, a3], out)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_return0_00674680`.
    PClassDescriptorForType_DistortionGridDynamicMesh_return0_00674680 @ 0x0067_4680 => extern "stdcall" fn return0_00674680(_x: u32) -> u32 {
        0
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_returnMinus1_00674690`.
    PClassDescriptorForType_DistortionGridDynamicMesh_returnMinus1_00674690 @ 0x0067_4690 => extern "stdcall" fn returnminus1_00674690(_a: u32, _b: u32, _c: u32) -> u32 {
        0xffff_ffff
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_accessorStore_006746a0`.
    PClassDescriptorForType_DistortionGridDynamicMesh_accessorStore_006746a0 @ 0x0067_46a0 => extern "stdcall" fn accessorstore_006746a0(l: u32, out: u32) -> u32 {
        f::accessor_store(&mut mem(), l, out, ffx_prender::m5r_f::GET_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_notSupported_006746c0`.
    PClassDescriptorForType_DistortionGridDynamicMesh_notSupported_006746c0 @ 0x0067_46c0 => extern "stdcall" fn notsupported_006746c0(_a: u32, _b: u32) -> u32 {
        f::not_supported(&mut mem())
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_pushPointer_006748a0`.
    PClassDescriptorForType_DistortionGridDynamicMesh_pushPointer_006748a0 @ 0x0067_48a0 => extern "stdcall" fn pushpointer_006748a0(l: u32, pp: u32) -> u32 {
        f::push_pointer(&mut mem(), l, pp, ffx_prender::m5r_f::CLASS_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_pushObject_006748e0`.
    PClassDescriptorForType_DistortionGridDynamicMesh_pushObject_006748e0 @ 0x0067_48e0 => extern "stdcall" fn pushobject_006748e0(l: u32, p: u32) -> u32 {
        f::push_object(&mut mem(), l, p, ffx_prender::m5r_f::CLASS_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMesh_registerFlagged_00674910`.
    PClassDescriptorForType_DistortionGridDynamicMesh_registerFlagged_00674910 @ 0x0067_4910 => extern "stdcall" fn registerflagged_00674910(p: u32, flag: u32) -> u32 {
        f::register_flagged(&mut mem(), p, flag, ffx_prender::m5r_f::CLASS_DISTORTION_GRID)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_Destruct_00674c20`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_Destruct_00674c20 @ 0x0067_4c20 => extern "thiscall" fn destruct_00674c20(this: u32) -> u32 {
        f::descriptor_destruct(&mut mem(), this, ffx_prender::m5r_f::VT_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c50`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c50 @ 0x0067_4c50 => extern "thiscall" fn scalardeletingdtor_00674c50(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c80`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674c80 @ 0x0067_4c80 => extern "thiscall" fn scalardeletingdtor_00674c80(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674cb0`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_scalarDeletingDtor_00674cb0 @ 0x0067_4cb0 => extern "thiscall" fn scalardeletingdtor_00674cb0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_returnMinus1_006750b0`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_returnMinus1_006750b0 @ 0x0067_50b0 => extern "stdcall" fn returnminus1_006750b0(_a: u32, _b: u32, _c: u32) -> u32 {
        0xffff_ffff
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_accessorStore_006750c0`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_accessorStore_006750c0 @ 0x0067_50c0 => extern "stdcall" fn accessorstore_006750c0(l: u32, out: u32) -> u32 {
        f::accessor_store(&mut mem(), l, out, ffx_prender::m5r_f::GET_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_notSupported_006750f0`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_notSupported_006750f0 @ 0x0067_50f0 => extern "stdcall" fn notsupported_006750f0(_a: u32, _b: u32) -> u32 {
        f::not_supported(&mut mem())
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushPointer_00675190`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushPointer_00675190 @ 0x0067_5190 => extern "stdcall" fn pushpointer_00675190(l: u32, pp: u32) -> u32 {
        f::push_pointer(&mut mem(), l, pp, ffx_prender::m5r_f::CLASS_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushObject_006751d0`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_pushObject_006751d0 @ 0x0067_51d0 => extern "stdcall" fn pushobject_006751d0(l: u32, p: u32) -> u32 {
        f::push_object(&mut mem(), l, p, ffx_prender::m5r_f::CLASS_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_DistortionGridDynamicMeshInstance_registerFlagged_00675200`.
    PClassDescriptorForType_DistortionGridDynamicMeshInstance_registerFlagged_00675200 @ 0x0067_5200 => extern "stdcall" fn registerflagged_00675200(p: u32, flag: u32) -> u32 {
        f::register_flagged(&mut mem(), p, flag, ffx_prender::m5r_f::CLASS_DISTORTION_GRID_INSTANCE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMeshInstance_Destruct_00675540`.
    PClassDescriptorForType_RadialLineDynamicMeshInstance_Destruct_00675540 @ 0x0067_5540 => extern "thiscall" fn destruct_00675540(this: u32) -> u32 {
        f::descriptor_destruct(&mut mem(), this, ffx_prender::m5r_f::VT_RADIAL_LINE_INSTANCE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_00675570`.
    PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_00675570 @ 0x0067_5570 => extern "thiscall" fn scalardeletingdtor_00675570(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE_INSTANCE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755a0`.
    PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755a0 @ 0x0067_55a0 => extern "thiscall" fn scalardeletingdtor_006755a0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE_INSTANCE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755d0`.
    PClassDescriptorForType_RadialLineDynamicMeshInstance_scalarDeletingDtor_006755d0 @ 0x0067_55d0 => extern "thiscall" fn scalardeletingdtor_006755d0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE_INSTANCE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fa0`.
    PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fa0 @ 0x0067_5fa0 => extern "thiscall" fn scalardeletingdtor_00675fa0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fd0`.
    PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00675fd0 @ 0x0067_5fd0 => extern "thiscall" fn scalardeletingdtor_00675fd0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE)
    }
    /// `PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00676000`.
    PClassDescriptorForType_RadialLineDynamicMesh_scalarDeletingDtor_00676000 @ 0x0067_6000 => extern "thiscall" fn scalardeletingdtor_00676000(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_RADIAL_LINE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784a0`.
    PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784a0 @ 0x0067_84a0 => extern "thiscall" fn scalardeletingdtor_006784a0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784d0`.
    PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_006784d0 @ 0x0067_84d0 => extern "thiscall" fn scalardeletingdtor_006784d0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_00678500`.
    PClassDescriptorForType_ShadowDynamicMesh_scalarDeletingDtor_00678500 @ 0x0067_8500 => extern "thiscall" fn scalardeletingdtor_00678500(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_virtualStore_00678b10`.
    PClassDescriptorForType_ShadowDynamicMesh_virtualStore_00678b10 @ 0x0067_8b10 => extern "thiscall" fn virtualstore_00678b10(this: u32, a0: u32, a1: u32, a2: u32, a3: u32, out: u32) -> u32 {
        f::virtual_store(&mut mem(), this, [a0, a1, a2, a3], out)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_accessorStore_00678b60`.
    PClassDescriptorForType_ShadowDynamicMesh_accessorStore_00678b60 @ 0x0067_8b60 => extern "stdcall" fn accessorstore_00678b60(l: u32, out: u32) -> u32 {
        f::accessor_store(&mut mem(), l, out, ffx_prender::m5r_f::GET_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_notSupported_00678b80`.
    PClassDescriptorForType_ShadowDynamicMesh_notSupported_00678b80 @ 0x0067_8b80 => extern "stdcall" fn notsupported_00678b80(_a: u32, _b: u32) -> u32 {
        f::not_supported(&mut mem())
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_pushPointer_00678d70`.
    PClassDescriptorForType_ShadowDynamicMesh_pushPointer_00678d70 @ 0x0067_8d70 => extern "stdcall" fn pushpointer_00678d70(l: u32, pp: u32) -> u32 {
        f::push_pointer(&mut mem(), l, pp, ffx_prender::m5r_f::CLASS_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_pushObject_00678db0`.
    PClassDescriptorForType_ShadowDynamicMesh_pushObject_00678db0 @ 0x0067_8db0 => extern "stdcall" fn pushobject_00678db0(l: u32, p: u32) -> u32 {
        f::push_object(&mut mem(), l, p, ffx_prender::m5r_f::CLASS_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMesh_registerFlagged_00678de0`.
    PClassDescriptorForType_ShadowDynamicMesh_registerFlagged_00678de0 @ 0x0067_8de0 => extern "stdcall" fn registerflagged_00678de0(p: u32, flag: u32) -> u32 {
        f::register_flagged(&mut mem(), p, flag, ffx_prender::m5r_f::CLASS_SHADOW)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_Destruct_006f94c0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_Destruct_006f94c0 @ 0x006f_94c0 => extern "thiscall" fn destruct_006f94c0(this: u32) -> u32 {
        f::descriptor_destruct(&mut mem(), this, ffx_prender::m5r_f::VT_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f94f0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f94f0 @ 0x006f_94f0 => extern "thiscall" fn scalardeletingdtor_006f94f0(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9520`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9520 @ 0x006f_9520 => extern "thiscall" fn scalardeletingdtor_006f9520(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9550`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_scalarDeletingDtor_006f9550 @ 0x006f_9550 => extern "thiscall" fn scalardeletingdtor_006f9550(this: u32, flags: u32) -> u32 {
        f::descriptor_delete(&mut mem(), this, flags, ffx_prender::m5r_f::VT_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_accessorStore_006f99a0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_accessorStore_006f99a0 @ 0x006f_99a0 => extern "stdcall" fn accessorstore_006f99a0(l: u32, out: u32) -> u32 {
        f::accessor_store(&mut mem(), l, out, ffx_prender::m5r_f::GET_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_notSupported_006f99e0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_notSupported_006f99e0 @ 0x006f_99e0 => extern "stdcall" fn notsupported_006f99e0(_a: u32, _b: u32) -> u32 {
        f::not_supported(&mut mem())
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_pushPointer_006f9a80`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_pushPointer_006f9a80 @ 0x006f_9a80 => extern "stdcall" fn pushpointer_006f9a80(l: u32, pp: u32) -> u32 {
        f::push_pointer(&mut mem(), l, pp, ffx_prender::m5r_f::CLASS_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_pushObject_006f9ac0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_pushObject_006f9ac0 @ 0x006f_9ac0 => extern "stdcall" fn pushobject_006f9ac0(l: u32, p: u32) -> u32 {
        f::push_object(&mut mem(), l, p, ffx_prender::m5r_f::CLASS_SHADOW_INSTANCE)
    }
    /// `PClassDescriptorForType_ShadowDynamicMeshInstance_registerFlagged_006f9af0`.
    PClassDescriptorForType_ShadowDynamicMeshInstance_registerFlagged_006f9af0 @ 0x006f_9af0 => extern "stdcall" fn registerflagged_006f9af0(p: u32, flag: u32) -> u32 {
        f::register_flagged(&mut mem(), p, flag, ffx_prender::m5r_f::CLASS_SHADOW_INSTANCE)
    }
    // ---- batch 7 (ffx_prender::m5r_g) ----
    /// `PostProcessTweakUtil_scalarDeletingDtor_00692a60(this; flags)`.
    PostProcessTweakUtil_scalarDeletingDtor_00692a60 @ 0x0069_2a60 => extern "thiscall" fn post_tweak_delete(this: u32, flags: u32) -> u32 {
        g::post_tweak_delete(&mut mem(), this, flags)
    }
    /// `PhyreScene_AddVFXToUI2DList(scene; vfx)`.
    PhyreScene_AddVFXToUI2DList @ 0x006a_0d40 => extern "thiscall" fn add_vfx_to_ui2d(this: u32, vfx: u32) -> u32 {
        g::add_vfx_to_ui2d(&mut mem(), this, vfx)
    }
    /// `VfxRenderData_pushPair_006a0f30(this)`.
    VfxRenderData_pushPair_006a0f30 @ 0x006a_0f30 => extern "thiscall" fn push_pair_006a0f30(this: u32) -> u32 {
        g::push_pair(&mut mem(), this)
    }
    /// `VfxRenderData_pushPair_006a0f50(this)`.
    VfxRenderData_pushPair_006a0f50 @ 0x006a_0f50 => extern "thiscall" fn push_pair_006a0f50(this: u32) -> u32 {
        g::push_pair(&mut mem(), this)
    }
    /// `VfxRenderDataTable_clearTree_006a0fe0(table)`.
    VfxRenderDataTable_clearTree_006a0fe0 @ 0x006a_0fe0 => extern "thiscall" fn clear_tree(this: u32) -> u32 {
        g::clear_tree(&mut mem(), this)
    }
    /// `PArray_Reset_006ad130(this)`.
    PArray_Reset_006ad130 @ 0x006a_d130 => extern "thiscall" fn owned_reset_006ad130(this: u32) {
        g::owned_reset(&mut mem(), this, false);
    }
    /// `PArray_Reset_006af4e0(this)`.
    PArray_Reset_006af4e0 @ 0x006a_f4e0 => extern "thiscall" fn owned_reset_006af4e0(this: u32) {
        g::owned_reset(&mut mem(), this, false);
    }
    /// `PArray_ResetChecked_006af570(this)`.
    PArray_ResetChecked_006af570 @ 0x006a_f570 => extern "thiscall" fn owned_reset_006af570(this: u32) {
        g::owned_reset(&mut mem(), this, true);
    }
    /// `PArray_Reset_006c7e60(this)`.
    PArray_Reset_006c7e60 @ 0x006c_7e60 => extern "thiscall" fn owned_reset_006c7e60(this: u32) {
        g::owned_reset(&mut mem(), this, false);
    }
    /// `PArray_Reset_006de370(this)`.
    PArray_Reset_006de370 @ 0x006d_e370 => extern "thiscall" fn owned_reset_006de370(this: u32) {
        g::owned_reset(&mut mem(), this, false);
    }
    /// `TextureAnimationInfoChar_scalarDeletingDtor_006afe00(this; flags)`.
    TextureAnimationInfoChar_scalarDeletingDtor_006afe00 @ 0x006a_fe00 => extern "thiscall" fn texanim_info_char_delete(this: u32, flags: u32) -> u32 {
        g::delete_with(&mut mem(), this, flags, ffx_prender::m5r_g::TEXANIM_INFO_DTOR)
    }
    /// `TextureAnimationInfo_scalarDeletingDtor_006afe30(this; flags)`.
    TextureAnimationInfo_scalarDeletingDtor_006afe30 @ 0x006a_fe30 => extern "thiscall" fn texanim_info_delete(this: u32, flags: u32) -> u32 {
        g::delete_with(&mut mem(), this, flags, ffx_prender::m5r_g::TEXANIM_INFO2_DTOR)
    }
    /// `DebugMenuManager_scalarDeletingDtor_006b4920(this; flags)`.
    DebugMenuManager_scalarDeletingDtor_006b4920 @ 0x006b_4920 => extern "thiscall" fn debug_menu_delete(this: u32, flags: u32) -> u32 {
        g::debug_menu_delete(&mut mem(), this, flags)
    }
    /// `DebugMenuManager_CreateInstance()`.
    DebugMenuManager_CreateInstance @ 0x006b_4950 => extern "C" fn debug_menu_create() {
        g::debug_menu_create(&mut mem());
    }
    /// `SpecularMapParamsManager_renderSpecularPass(this; x)`.
    SpecularMapParamsManager_renderSpecularPass @ 0x006c_5b60 => extern "thiscall" fn render_specular_pass(this: u32, x: u32) -> u32 {
        g::render_specular_pass(&mut mem(), this, x)
    }
    /// `FloatLess_006c94e0(a, b)` (`al`).
    FloatLess_006c94e0 @ 0x006c_94e0 => extern "C" fn float_less(a: u32, b: u32) -> u8 {
        g::float_less(a, b)
    }
    /// `ShadowSolution_is1Or202b_006cc8e0(x)` (`al`, the rest `x`'s).
    ShadowSolution_is1Or202b_006cc8e0 @ 0x006c_c8e0 => extern "C" fn solution_is_1_or_202b(x: u32) -> u32 {
        g::solution_in(x, &[1, 0x202b], true)
    }
    /// `ShadowSolution_not2144_006cc900(x)`.
    ShadowSolution_not2144_006cc900 @ 0x006c_c900 => extern "C" fn solution_not_2144(x: u32) -> u32 {
        g::solution_in(x, &[0x2144, 0x206e], false)
    }
    /// `ShadowSolution_not201a_006cc920(x)`.
    ShadowSolution_not201a_006cc920 @ 0x006c_c920 => extern "C" fn solution_not_201a(x: u32) -> u32 {
        g::solution_in(x, &[0x201a, 0x2021, 0x215d, 0x5009], false)
    }
    /// `ShadowSolution_not20dd_006cc950(x)`.
    ShadowSolution_not20dd_006cc950 @ 0x006c_c950 => extern "C" fn solution_not_20dd(x: u32) -> u32 {
        g::solution_in(x, &[0x20dd, 0x215d, 0x215a], false)
    }
    /// `TextUtil_dumpEntry_006dd200(entry, file)`.
    TextUtil_dumpEntry_006dd200 @ 0x006d_d200 => extern "C" fn dump_entry(entry: u32, file: u32) -> u32 {
        g::dump_entry(&mut mem(), entry, file)
    }
    /// `TextUtil_claimSlot_006f5de0(slots; v)`.
    TextUtil_claimSlot_006f5de0 @ 0x006f_5de0 => extern "thiscall" fn claim_slot(this: u32, v: u32) -> u32 {
        g::claim_slot(&mut mem(), this, v)
    }
    /// `BlurEffect_setFloats4_006f8b10(this; src)`.
    BlurEffect_setFloats4_006f8b10 @ 0x006f_8b10 => extern "thiscall" fn set_floats4(this: u32, src: u32) -> u32 {
        g::copy_floats(&mut mem(), this, src, 0x38, 4)
    }
    /// `BlurEffect_setFloats2_006f8ba0(this; src)`.
    BlurEffect_setFloats2_006f8ba0 @ 0x006f_8ba0 => extern "thiscall" fn set_floats2(this: u32, src: u32) -> u32 {
        g::copy_floats(&mut mem(), this, src, 0x48, 2)
    }
    /// `ShadowDynamicMesh_Bind()`.
    ShadowDynamicMesh_Bind @ 0x0067_88c0 => extern "C" fn shadow_bind() -> u32 {
        g::bind_class(&mut mem(), ffx_prender::m5r_g::CLASS_SHADOW)
    }
    /// `ShadowDynamicMeshInstance_Bind()`.
    ShadowDynamicMeshInstance_Bind @ 0x006f_9870 => extern "C" fn shadow_instance_bind() -> u32 {
        g::bind_class(&mut mem(), ffx_prender::m5r_g::CLASS_SHADOW_INSTANCE)
    }
    /// `PhyreFIOS_UnloadPrx(module)`.
    PhyreFIOS_UnloadPrx @ 0x006f_9d20 => extern "C" fn unload_prx(module: u32) -> u32 {
        g::unload_prx(&mut mem(), module)
    }
    /// `syncStrPcSelect_0077e880(a, b, stream)` (`a` is never read).
    syncStrPcSelect_0077e880 @ 0x0077_e880 => extern "C" fn sync_str_select(_a: u32, b: u32, stream: u32) -> u32 {
        g::sync_str_select(&mut mem(), b, stream)
    }
    /// `GsPackTag_0077ead0(out, a, b, c)`.
    GsPackTag_0077ead0 @ 0x0077_ead0 => extern "C" fn gs_pack_tag(out: u32, a0: u32, a1: u32, a2: u32) -> u32 {
        g::gs_pack_tag(&mut mem(), out, a0, a1, a2)
    }
    // ---- batch 8 (ffx_prender::m5r_h, and twins of earlier ports) ----
    /// `RenderDelegationHelper_beforeScene(this)`: the flag-8 run
    /// (`p6_more::run_delegations`, the code of `runFlag200`).
    RenderDelegationHelper_beforeScene @ 0x006a_da20 => extern "thiscall" fn delegations_flag8(this: u32) -> u32 {
        ffx_prender::p6_more::bridge::run_delegations(&mut mem(), this, 8)
    }
    /// `RenderDelegationHelper_runFlag2_006adad0(this)`: the flag-2 run.
    RenderDelegationHelper_runFlag2_006adad0 @ 0x006a_dad0 => extern "thiscall" fn delegations_flag2(this: u32) -> u32 {
        ffx_prender::p6_more::bridge::run_delegations(&mut mem(), this, 2)
    }
    /// `PClassDescriptor_ShadowDynamicMeshInstance_destructInstance_006f9910(p)`:
    /// the code of `0x004469a0`.
    PClassDescriptor_ShadowDynamicMeshInstance_destructInstance_006f9910 @ 0x006f_9910 => extern "stdcall" fn shadow_instance_destruct(p: u32) -> u8 {
        ffx_pload::descriptor::bridge::destroy_virtual(&mut mem(), 0, 4, p)
    }
    /// `PClassDescriptorConcrete_ShadowDynamicMesh_constructInstance_006788e0(desc; obj)`.
    PClassDescriptorConcrete_ShadowDynamicMesh_constructInstance_006788e0 @ 0x0067_88e0 => extern "thiscall" fn shadow_construct(_desc: u32, obj: u32) -> u8 {
        ffx_prender::p6_descriptor::bridge::construct_with(&mut mem(), obj, 0x0049_68c0, Some(0x00b4_4c78), None)
    }
    /// `PClassDescriptorWithoutDefaultConstructor_ShadowDynamicMeshInstance_constructInstance_006f9890(desc; obj)`.
    PClassDescriptorWithoutDefaultConstructor_ShadowDynamicMeshInstance_constructInstance_006f9890 @ 0x006f_9890 => extern "thiscall" fn shadow_instance_construct(_desc: u32, obj: u32) -> u8 {
        ffx_pother_b::object::bridge::construct_mesh_instance_default(&mut mem(), obj, 0x00cc_c158, 0x00b4_c250)
    }
    /// `RadialLine_setParam6c_006f8b40(this; x, y, z, w)`: copies from its
    /// own argument words.
    RadialLine_setParam6c_006f8b40 @ 0x006f_8b40 => extern "thiscall" fn radial_line_set_param6c(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        ffx_glue_other::frames::post_set_param(&mut mem(), ffx_glue_other::frames::ParamSlot { guard: 0x6c, param: 0x6c, holder: 0x1c }, this, args)
    }
    /// `RadialLine_setParam70_006f8be0(this; x, y, z, w)`.
    RadialLine_setParam70_006f8be0 @ 0x006f_8be0 => extern "thiscall" fn radial_line_set_param70(this: u32, _x: u32, _y: u32, _z: u32, _w: u32; args: u32) -> u32 {
        ffx_glue_other::frames::post_set_param(&mut mem(), ffx_glue_other::frames::ParamSlot { guard: 0x74, param: 0x70, holder: 0x1c }, this, args)
    }
    /// `PArray_release20_006af510(this)`.
    PArray_release20_006af510 @ 0x006a_f510 => extern "thiscall" fn release_array20(this: u32) {
        h::release_array20(&mut mem(), this, Some(ffx_prender::m5r_h::ELEMENT20_DTOR));
    }
    /// `PArray_destroyObjects20_006af5d0(this)`.
    PArray_destroyObjects20_006af5d0 @ 0x006a_f5d0 => extern "thiscall" fn destroy_objects20(this: u32) {
        h::release_array20(&mut mem(), this, None);
    }
    /// `PhyreScene_moveLoadingCluster_00657920(scene)`.
    PhyreScene_moveLoadingCluster_00657920 @ 0x0065_7920 => extern "thiscall" fn move_loading_cluster(this: u32) {
        h::move_loading_cluster(&mut mem(), this);
    }
    /// `PhyreScene_PushMeshMatrix(scene; mesh, matrix)`: the record in the
    /// shim's frame (`ebp - 0x44`).
    PhyreScene_PushMeshMatrix @ 0x0065_6780 => extern "thiscall" fn push_mesh_matrix(this: u32, mesh: u32, matrix: u32) -> u32 {
        let mut f = StackFrame::<0x12>::new();
        h::push_mesh_matrix(&mut mem(), f.at() + 0x48, this, mesh, matrix)
    }
    /// `DynGeoMemManager_swapBuffer_006dece0(this)`.
    DynGeoMemManager_swapBuffer_006dece0 @ 0x006d_ece0 => extern "thiscall" fn swap_buffer(this: u32) -> u32 {
        h::swap_buffer(&mut mem(), this)
    }
    /// `PhyreScene_unloadWaterEffectTexture(scene)`: the name in the
    /// shim's frame.
    PhyreScene_unloadWaterEffectTexture @ 0x0067_1b90 => extern "thiscall" fn unload_water_texture(this: u32) -> u32 {
        let mut f = StackFrame::<0x41>::new();
        h::unload_water_texture(&mut mem(), f.at() + 0x104, this)
    }
    /// `PhyreScene_updateTextureVideoCallback(job)`.
    PhyreScene_updateTextureVideoCallback @ 0x0067_2070 => extern "C" fn update_texture_video_callback(job: u32) -> u32 {
        h::update_texture_video_callback(&mut mem(), job)
    }
    /// `PArray_resizeWords_0066c690(array; n)`.
    PArray_resizeWords_0066c690 @ 0x0066_c690 => extern "thiscall" fn resize_words_0066c690(this: u32, n: u32) -> u32 {
        h::resize_words(&mut mem(), this, n, 0x0064_f7f0)
    }
    /// `PArray_resizeWords_0066c700(array; n)`.
    PArray_resizeWords_0066c700 @ 0x0066_c700 => extern "thiscall" fn resize_words_0066c700(this: u32, n: u32) -> u32 {
        h::resize_words(&mut mem(), this, n, 0x0064_f860)
    }
    /// `PhyreScene_setInstanceListOwners_0066c770(scene)`.
    PhyreScene_setInstanceListOwners_0066c770 @ 0x0066_c770 => extern "thiscall" fn set_instance_list_owners(this: u32) {
        h::set_instance_list_owners(&mut mem(), this);
    }
    /// `FloatHeap_sortHeap_006c7890(first, last, pred)`: parks the back
    /// value in its second argument slot.
    FloatHeap_sortHeap_006c7890 @ 0x006c_7890 => extern "C" fn sort_heap(first: u32, last: u32, pred: u32; slots: u32) -> u32 {
        h::sort_heap(&mut mem(), slots + 4, first, last, pred)
    }
    /// `ClassCharacter_pushInstance_006a0d00(this; inst)`: hands on the
    /// address of its argument slot.
    ClassCharacter_pushInstance_006a0d00 @ 0x006a_0d00 => extern "thiscall" fn push_instance(this: u32, _inst: u32; slots: u32) -> u32 {
        h::push_instance(&mut mem(), this, slots)
    }
    /// `TaggedBuffer_free_006c7ec0(this)`.
    TaggedBuffer_free_006c7ec0 @ 0x006c_7ec0 => extern "thiscall" fn tagged_free(this: u32) {
        h::tagged_free(&mut mem(), this);
    }
    /// `ShadowDynamicMesh_copyOut_00678ba0(L, out)`.
    ShadowDynamicMesh_copyOut_00678ba0 @ 0x0067_8ba0 => extern "stdcall" fn shadow_copy_out(l: u32, out: u32) -> u32 {
        h::shadow_copy_out(&mut mem(), l, out)
    }
    /// `MagicDLLAsyncLoad(job)`.
    MagicDLLAsyncLoad @ 0x006f_9cc0 => extern "C" fn magic_dll_async_load(job: u32) -> u32 {
        h::magic_dll_async_load(&mut mem(), job)
    }
    /// `syncStrCheckPC_HDD(stream)`: the ioctl word in the shim's frame.
    syncStrCheckPC_HDD @ 0x0077_e8e0 => extern "C" fn sync_str_check_hdd(stream: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        h::sync_str_check_hdd(&mut mem(), f.at() + 4, stream)
    }
    /// `syncStrCheck_0077e8b0(stream)`.
    syncStrCheck_0077e8b0 @ 0x0077_e8b0 => extern "C" fn sync_str_check(stream: u32) -> u32 {
        h::sync_str_check(&mut mem(), stream)
    }
    /// `GsSetFrameRegs_0077e830(a, b, c)`.
    GsSetFrameRegs_0077e830 @ 0x0077_e830 => extern "C" fn gs_set_frame_regs(a0: u32, a1: u32, a2: u32) -> u32 {
        h::gs_set_frame_regs(&mut mem(), a0, a1, a2)
    }
    /// `Cache_Dump(tag)`: stores the tail message into its argument slot.
    Cache_Dump @ 0x0084_0510 => extern "C" fn cache_dump(tag: u32; slots: u32) -> u32 {
        h::cache_dump(&mut mem(), slots, tag)
    }
    // ---- batch 9 (ffx_prender::m5r_i, and twins of earlier ports) ----
    /// `PhyreScene_AddVFXToList3880(scene; v)`: the unique add on `+0x3878`.
    PhyreScene_AddVFXToList3880 @ 0x0065_65e0 => extern "thiscall" fn add_vfx_to_list3880(this: u32, v: u32) -> u32 {
        i::add_unique(&mut mem(), this, 0x3878, v)
    }
    /// `PhyreScene_AddVFXToDTTList(scene; vfx)`.
    PhyreScene_AddVFXToDTTList @ 0x0065_6990 => extern "thiscall" fn add_vfx_to_dtt(this: u32, vfx: u32) -> u32 {
        i::add_vfx_to_dtt(&mut mem(), this, vfx)
    }
    /// `TaggedTree_add_00693c50(this; &key, &value)`: `TaggedTree_add`'s
    /// code with the insert `0x00691e60`.
    TaggedTree_add_00693c50 @ 0x0069_3c50 => extern "thiscall" fn tagged_tree_add_00693c50(this: u32, key: u32, value: u32) -> u32 {
        ffx_glue_other::r3::tree_add(&mut mem(), ffx_prender::m5r_i::TREE_INSERT_691E60, this, key, value)
    }
    /// `StringTreeNode_ctor_0064cba0(this; name)`.
    StringTreeNode_ctor_0064cba0 @ 0x0064_cba0 => extern "thiscall" fn string_node_ctor(this: u32, name: u32) -> u32 {
        i::string_node_ctor(&mut mem(), this, name)
    }
    /// `StringPairTreeNode_ctor_0064cb00(this; key, value)`.
    StringPairTreeNode_ctor_0064cb00 @ 0x0064_cb00 => extern "thiscall" fn string_pair_node_ctor(this: u32, key: u32, value: u32) -> u32 {
        i::string_pair_node_ctor(&mut mem(), this, key, value)
    }
    /// `TextureAnimationInfo_dtor_006af760(this)`.
    TextureAnimationInfo_dtor_006af760 @ 0x006a_f760 => extern "thiscall" fn texanim_info_dtor(this: u32) {
        i::texanim_info_dtor(&mut mem(), this);
    }
    /// `SpecularMapParams_setTextures_006c5570(this; a, b)`: the texture
    /// record in the shim's frame (`ebp - 0x10`).
    SpecularMapParams_setTextures_006c5570 @ 0x006c_5570 => extern "thiscall" fn set_specular_textures(this: u32, a0: u32, a1: u32) {
        let mut f = StackFrame::<4>::new();
        i::set_specular_textures(&mut mem(), f.at() + 0x10, this, a0, a1);
    }
    /// `FloatSort_Median9_006c7560(first, mid, last, pred)`: stores the
    /// step into its first argument slot.
    FloatSort_Median9_006c7560 @ 0x006c_7560 => extern "C" fn median9(first: u32, mid: u32, last: u32, pred: u32; slots: u32) -> u32 {
        i::median9(&mut mem(), slots, first, mid, last, pred)
    }
    /// `DynGeoMemManager_createInstance_006deb90()`.
    DynGeoMemManager_createInstance_006deb90 @ 0x006d_eb90 => extern "C" fn dyn_geo_create() {
        i::dyn_geo_create(&mut mem());
    }
    /// `SamplerPair_dtor_00692680(this)`.
    SamplerPair_dtor_00692680 @ 0x0069_2680 => extern "thiscall" fn sampler_pair_dtor_00692680(this: u32) -> u32 {
        i::sampler_pair_dtor(&mut mem(), this)
    }
    /// `SamplerPair_dtor_006927a0(this)`.
    SamplerPair_dtor_006927a0 @ 0x0069_27a0 => extern "thiscall" fn sampler_pair_dtor_006927a0(this: u32) -> u32 {
        i::sampler_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreePair_dtor_006924e0(this)`: `0x00633dc0`'s code.
    PRedBlackTreePair_dtor_006924e0 @ 0x0069_24e0 => extern "thiscall" fn tree_pair_dtor_006924e0(this: u32) -> u32 {
        b::tree_pair_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreeFree_dtor_00692540(this)`: `0x00633ef0`'s code.
    PRedBlackTreeFree_dtor_00692540 @ 0x0069_2540 => extern "thiscall" fn tree_free_dtor_00692540(this: u32) -> u32 {
        b::tree_free_dtor(&mut mem(), this)
    }
    /// `PRedBlackTreeFree_dtor_006925c0(this)`: `0x00633ef0`'s code.
    PRedBlackTreeFree_dtor_006925c0 @ 0x0069_25c0 => extern "thiscall" fn tree_free_dtor_006925c0(this: u32) -> u32 {
        b::tree_free_dtor(&mut mem(), this)
    }
    // ---- batch 10 (ffx_prender::m5r_j) ----
    /// `RenderDelegationHelper_runFlag1000(this; value)` (`al`).
    RenderDelegationHelper_runFlag1000 @ 0x006a_e450 => extern "thiscall" fn run_flag1000(this: u32, value: u32) -> u32 {
        j::run_with_value(&mut mem(), this, 0x1000, value)
    }
    /// `RenderDelegationHelper_runFlag8000_006ae0c0(this; value)` (`al`).
    RenderDelegationHelper_runFlag8000_006ae0c0 @ 0x006a_e0c0 => extern "thiscall" fn run_flag8000(this: u32, value: u32) -> u32 {
        j::run_with_value(&mut mem(), this, 0x8000, value)
    }
    /// `PhyreScene_RenderVFXAfterPostProcess(scene)`.
    PhyreScene_RenderVFXAfterPostProcess @ 0x0065_38d0 => extern "thiscall" fn render_vfx_after_post(this: u32) -> u32 {
        j::render_vfx_after_post(&mut mem(), this)
    }
    // ---- batch 11 (ffx_prender::m5r_k) ----
    /// `PhyreFIOS_LoadsPrx(path, x, callback)` (`x` is never read).
    PhyreFIOS_LoadsPrx @ 0x006f_9bc0 => extern "C" fn loads_prx(path: u32, _x: u32, callback: u32) -> u32 {
        k::loads_prx(&mut mem(), path, callback)
    }
    /// `AutoTestManager_scalarDeletingDtor_006bcbd0(this; flags)`.
    AutoTestManager_scalarDeletingDtor_006bcbd0 @ 0x006b_cbd0 => extern "thiscall" fn auto_test_delete(this: u32, flags: u32) -> u32 {
        k::auto_test_delete(&mut mem(), this, flags)
    }
    /// `DebugTextPanel_ctor_006b53b0(this)`.
    DebugTextPanel_ctor_006b53b0 @ 0x006b_53b0 => extern "thiscall" fn debug_text_panel_ctor(this: u32) -> u32 {
        k::debug_text_panel_ctor(&mut mem(), this)
    }
    // ---- render-pcui batch 1 (ffx_prender::m5r_l) ----
    /// `graphicRemoveDelegations_0063dab0()`: `jmp RenderDelegationHelper_removeFlag1000_006ae530`.
    graphicRemoveDelegations_0063dab0 @ 0x0063_dab0 => extern "C" fn remove_delegations() -> u32 {
        a::remove_delegations(&mut mem())
    }
    /// `RadialLine_destroyRadialLine_thunk_006f7b10(this)`.
    RadialLine_destroyRadialLine_thunk_006f7b10 @ 0x006f_7b10 => extern "thiscall" fn radial_line_destroy_thunk(this: u32) {
        l::forward(&mut mem(), ml::RADIAL_LINE_DESTROY, Conv::Thiscall(this), &[]);
    }
    /// `MsGetEncountType_thunk_00638e20()`.
    MsGetEncountType_thunk_00638e20 @ 0x0063_8e20 => extern "C" fn ms_get_encount_type_thunk() -> u32 {
        l::forward(&mut mem(), ml::MS_GET_ENCOUNT_TYPE, Conv::Cdecl, &[])
    }
    /// `MsBattleCheck_thunk_00638e60()`.
    MsBattleCheck_thunk_00638e60 @ 0x0063_8e60 => extern "C" fn ms_battle_check_thunk() -> u32 {
        l::forward(&mut mem(), ml::MS_BATTLE_CHECK, Conv::Cdecl, &[])
    }
    /// `graphicAttachChrClusterTo3DWorld_00639350()`.
    graphicAttachChrClusterTo3DWorld_00639350 @ 0x0063_9350 => extern "C" fn attach_chr_cluster_thunk() {
        l::scene_forward(&mut mem(), ml::ATTACH_CHR_CLUSTER);
    }
    /// `graphicCharacterUnloadTask_thunk_0063d9a0(chr)`.
    graphicCharacterUnloadTask_thunk_0063d9a0 @ 0x0063_d9a0 => extern "C" fn character_unload_task_thunk(chr: u32) {
        l::forward(&mut mem(), ml::CHARACTER_UNLOAD_TASK, Conv::Cdecl, &[chr]);
    }
    /// `graphicGetVideoEndTime()`.
    graphicGetVideoEndTime @ 0x0064_1140 => extern "C" fn video_end_time() -> u32 {
        l::video_end_time(&mut mem())
    }
    /// `graphicGetAppFloat78_006411a0()` (`ST0`).
    graphicGetAppFloat78_006411a0 @ 0x0064_11a0 => extern "C" fn app_float_78() -> f32 {
        l::app_float(&mut mem(), 0x78)
    }
    /// `graphicGetAppFloatD8_00641270()` (`ST0`).
    graphicGetAppFloatD8_00641270 @ 0x0064_1270 => extern "C" fn app_float_d8() -> f32 {
        l::app_float(&mut mem(), 0xd8)
    }
    /// `graphicGetAppFloatD4_00641280()` (`ST0`).
    graphicGetAppFloatD4_00641280 @ 0x0064_1280 => extern "C" fn app_float_d4() -> f32 {
        l::app_float(&mut mem(), 0xd4)
    }
    /// `graphicGetAppFloatDC_00641290()` (`ST0`).
    graphicGetAppFloatDC_00641290 @ 0x0064_1290 => extern "C" fn app_float_dc() -> f32 {
        l::app_float(&mut mem(), 0xdc)
    }
    /// `graphicGetAppFloatD0_006412a0()` (`ST0`).
    graphicGetAppFloatD0_006412a0 @ 0x0064_12a0 => extern "C" fn app_float_d0() -> f32 {
        l::app_float(&mut mem(), 0xd0)
    }
    /// `graphicSegmentsAllReady_thunk_006420d0()`.
    graphicSegmentsAllReady_thunk_006420d0 @ 0x0064_20d0 => extern "C" fn segments_all_ready_thunk() -> u32 {
        l::forward(&mut mem(), ml::SEGMENTS_ALL_READY, Conv::Cdecl, &[])
    }
    /// `graphicVideoClearPlaying_00643e50()`.
    graphicVideoClearPlaying_00643e50 @ 0x0064_3e50 => extern "C" fn video_clear_playing() -> u32 {
        l::video_clear_playing(&mut mem())
    }
    /// `graphicUpdateMapYIQ_thunk_006449c0()`.
    graphicUpdateMapYIQ_thunk_006449c0 @ 0x0064_49c0 => extern "C" fn update_map_yiq_thunk() {
        l::scene_forward(&mut mem(), ml::SCENE_UPDATE_MAP_YIQ);
    }
    /// `graphicVideoState_00645dc0()`.
    graphicVideoState_00645dc0 @ 0x0064_5dc0 => extern "C" fn video_state() -> u32 {
        l::video_state(&mut mem())
    }
    /// `graphicisVideoAtEndFrame()`.
    graphicisVideoAtEndFrame @ 0x0064_5e50 => extern "C" fn video_at_end_frame() -> u32 {
        l::video_at_end_frame(&mut mem())
    }
    /// `graphicVideoIsPlaying_00660410()`.
    graphicVideoIsPlaying_00660410 @ 0x0066_0410 => extern "C" fn video_is_playing() -> u32 {
        l::video_is_playing(&mut mem())
    }
    /// `graphicPostProcessSetFlag_00699080()`.
    graphicPostProcessSetFlag_00699080 @ 0x0069_9080 => extern "C" fn post_set_flag() -> u32 {
        l::post_set_flag(&mut mem())
    }
    /// `FloatSwap_006c7c80(a, b)`.
    FloatSwap_006c7c80 @ 0x006c_7c80 => extern "C" fn float_swap(a: u32, b: u32; slots: u32) -> u32 {
        l::float_swap(&mut mem(), slots, a, b)
    }
    /// `BrokenScreenEffect_releaseObjects_thunk_006d3600(this)`.
    BrokenScreenEffect_releaseObjects_thunk_006d3600 @ 0x006d_3600 => extern "thiscall" fn broken_screen_release_thunk(this: u32) {
        l::forward(&mut mem(), ml::BROKEN_SCREEN_RELEASE, Conv::Thiscall(this), &[]);
    }
    // ---- render-pcui batch 2 (ffx_prender::m5r_m) ----
    /// `PhyreScene_resetMapObjects_0065f0e0(scene)` (cursor frame).
    PhyreScene_resetMapObjects_0065f0e0 @ 0x0065_f0e0 => extern "thiscall" fn map_objects_reset(this: u32) {
        let mut f = StackFrame::<9>::new();
        m::map_objects_reset(&mut mem(), f.at() + 0x24, this);
    }
    /// `SpecularMapParamsManager_AddSpecularEvent(x)` (stdcall; event frame).
    SpecularMapParamsManager_AddSpecularEvent @ 0x006c_56f0 => extern "stdcall" fn add_specular_event(x: u32) -> u32 {
        let mut f = StackFrame::<4>::new();
        m::add_specular_event(&mut mem(), f.at(), x)
    }
    /// `SpecularMapParamsManager_setAkazaNoLight_006c5b80(this; id, a, b, f0..f3, v, kind)`.
    SpecularMapParamsManager_setAkazaNoLight_006c5b80 @ 0x006c_5b80 => extern "thiscall" fn set_akaza_no_light(this: u32, id: u32, a: u32, b: u32, f0: u32, f1: u32, f2: u32, f3: u32, v: u32, kind: u32) {
        m::set_akaza_no_light(&mut mem(), this, id, a, b, [f0, f1, f2, f3], v, kind);
    }
    /// `SpecularMapParamsManager_setAkaza_006c5c90(this; id, a, b, f0..f3, v, last, lv, lc, event)`.
    SpecularMapParamsManager_setAkaza_006c5c90 @ 0x006c_5c90 => extern "thiscall" fn set_akaza(this: u32, id: u32, a: u32, b: u32, f0: u32, f1: u32, f2: u32, f3: u32, v: u32, last: u32, lv: u32, lc: u32, event: u32) {
        m::set_akaza(&mut mem(), this, id, a, b, [f0, f1, f2, f3], v, last, lv, lc, event);
    }
    /// `SpecularMapParamsManager_setNoLight_006c5e30(this; id, w0, f0, f1, w1, f2, w2, f3, f4, f5)`.
    SpecularMapParamsManager_setNoLight_006c5e30 @ 0x006c_5e30 => extern "thiscall" fn set_no_light(this: u32, id: u32, w0: u32, f0: u32, f1: u32, w1: u32, f2: u32, w2: u32, f3: u32, f4: u32, f5: u32) {
        m::set_no_light(&mut mem(), this, [id, w0, f0, f1, w1, f2, w2, f3, f4, f5]);
    }
    /// `PhyreScene_addCopyBufferEvent_006568a0(scene; b0, b1, x, h)`.
    PhyreScene_addCopyBufferEvent_006568a0 @ 0x0065_68a0 => extern "thiscall" fn add_copy_buffer_event(this: u32, b0: u32, b1: u32, x: u32, hw: u32) -> u32 {
        m::add_copy_buffer_event(&mut mem(), this, b0, b1, x, hw)
    }
    /// `PhyreScene_AddVFXToAfterPostList(scene; vfx)`.
    PhyreScene_AddVFXToAfterPostList @ 0x0065_67e0 => extern "thiscall" fn add_vfx_after_post(this: u32, vfx: u32) {
        m::add_vfx_after_post(&mut mem(), this, vfx);
    }
    /// `PArray44_pushInto_0065fbe0(array; block, item)`.
    PArray44_pushInto_0065fbe0 @ 0x0065_fbe0 => extern "thiscall" fn push_into_44(this: u32, block: u32, item: u32) -> u32 {
        m::push_into_44(&mut mem(), this, block, item)
    }
    /// `PArray4_pushInto_006add90(array; block, item)`.
    PArray4_pushInto_006add90 @ 0x006a_dd90 => extern "thiscall" fn push_into_4(this: u32, block: u32, item: u32) -> u32 {
        m::push_into_4(&mut mem(), this, block, item)
    }
    /// `ShadowMeshDescriptor_newInstance_00678940(this; cluster, existing)` (argument slots).
    ShadowMeshDescriptor_newInstance_00678940 @ 0x0067_8940 => extern "thiscall" fn shadow_instance(this: u32, cluster: u32, existing: u32; slots: u32) -> u32 {
        m::shadow_instance(&mut mem(), slots, this, cluster, existing)
    }
    /// `TreePair_clear_006df080(this)`.
    TreePair_clear_006df080 @ 0x006d_f080 => extern "thiscall" fn tree_clear_twice(this: u32) -> u32 {
        m::tree_clear_twice(&mut mem(), this)
    }
    /// `TextureAnimationManager_dtor_006af8a0(this)`.
    TextureAnimationManager_dtor_006af8a0 @ 0x006a_f8a0 => extern "thiscall" fn texanim_manager_dtor(this: u32) -> u32 {
        m::texanim_manager_dtor(&mut mem(), this)
    }
    /// `TexAnim_setTag_0077eb00(q, a, b, c, flag)`.
    TexAnim_setTag_0077eb00 @ 0x0077_eb00 => extern "C" fn set_tag(q: u32, a: u32, b: u32, c: u32, flag: u32) -> u32 {
        m::set_tag(&mut mem(), q, a, b, c, flag)
    }
    /// `FloatSort_introsortLoop_006c77a0(first, last, depth, less)` (argument slots; the pair frame).
    FloatSort_introsortLoop_006c77a0 @ 0x006c_77a0 => extern "C" fn introsort(first: u32, last: u32, depth: u32, less: u32; slots: u32) {
        let mut f = StackFrame::<2>::new();
        m::introsort(&mut mem(), slots, f.at(), first, last, depth, less);
    }
    // ---- round 5 batch 1 (ffx_prender::m5r_n) ----
    /// `PhyreScene_finalize_00659d20(scene)`.
    PhyreScene_finalize_00659d20 @ 0x0065_9d20 => extern "thiscall" fn scene_finalize(this: u32) {
        n::finalize(&mut mem(), this);
    }
    /// `graphicCharacterGetBounds_0063a410(chr, max, min)` (the 0xc4-byte frame).
    graphicCharacterGetBounds_0063a410 @ 0x0063_a410 => extern "C" fn chr_bounds(chr: u32, max: u32, min: u32) -> u32 {
        let mut f = StackFrame::<0x31>::new();
        n::chr_bounds(&mut mem(), f.at(), chr, max, min)
    }
    /// `graphicCharacterGetJointBounds_0063a610(chr, max, min, jmax, jmin)` (the corner).
    graphicCharacterGetJointBounds_0063a610 @ 0x0063_a610 => extern "C" fn chr_joint_bounds(chr: u32, max: u32, min: u32, jmax: u32, jmin: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        n::chr_joint_bounds(&mut mem(), f.at(), chr, max, min, jmax, jmin)
    }
    /// `graphicCharacterInstanceOutputAllVertexBuffers(inst)` (the mapped range).
    graphicCharacterInstanceOutputAllVertexBuffers @ 0x0063_b290 => extern "C" fn output_vertex_buffers(inst: u32) {
        let mut f = StackFrame::<2>::new();
        n::output_vertex_buffers(&mut mem(), f.at(), inst);
    }
    /// `graphicCharacterSetSummonTexture_0063c0c0(inst, name)` (the iterator and the sampler value).
    graphicCharacterSetSummonTexture_0063c0c0 @ 0x0063_c0c0 => extern "C" fn set_summon_texture(inst: u32, name: u32) {
        let mut f = StackFrame::<8>::new();
        n::set_summon_texture(&mut mem(), f.at(), inst, name);
    }
    /// `graphicCharacterClearSummonLayers_0063dad0(inst)`.
    graphicCharacterClearSummonLayers_0063dad0 @ 0x0063_dad0 => extern "C" fn clear_summon_layers(inst: u32) {
        n::clear_summon_layers(&mut mem(), inst);
    }
    /// `graphicCharacterRestoreSummonTexture()`.
    graphicCharacterRestoreSummonTexture @ 0x0063_d550 => extern "C" fn restore_summon_texture() -> u32 {
        n::restore_summon_texture(&mut mem())
    }
    /// `graphicDrawDream()` (the four UI elements).
    graphicDrawDream @ 0x0063_df70 => extern "C" fn draw_dream() {
        let mut f = StackFrame::<0x98>::new();
        n::draw_dream(&mut mem(), f.at());
    }
    /// `Cache_FreeClothVertexBuff_00640e70(slot)`.
    Cache_FreeClothVertexBuff_00640e70 @ 0x0064_0e70 => extern "C" fn free_cloth_buffers(slot: u32) -> u32 {
        n::free_cloth_buffers(&mut mem(), slot)
    }
    /// `graphicMagic0708ConvBallChr(list, center, range, rate)` (argument slots: `range` and `rate` are copied from them).
    graphicMagic0708ConvBallChr @ 0x0064_1e60 => extern "C" fn conv_ball_chr(list: u32, center: u32, range: u32, rate: u32; slots: u32) {
        let _ = (range, rate);
        n::conv_ball_chr(&mut mem(), slots, list, center);
    }
    /// `graphicMagic_DrawColorModel_KaMdldrw(inst, origin, modify, now, gradient)`.
    graphicMagic_DrawColorModel_KaMdldrw @ 0x0064_2110 => extern "C" fn color_model(inst: u32, origin: u32, modify: u32, now: u32, gradient: u32) {
        n::color_model(&mut mem(), inst, [origin, modify, now, gradient]);
    }
    /// `graphicSetLoadingScreen_00643500(on, mode)` (argument slots: the first holds the 0.0 handed on; the two parameters).
    graphicSetLoadingScreen_00643500 @ 0x0064_3500 => extern "C" fn loading_screen(on: u32, mode: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        n::loading_screen(&mut mem(), slots, f.at(), on, mode)
    }
    /// `PhyreScene_setColorScale_0066d3b0(scene; r, g, b, ...)` (`ret 0x1c`: four more words it never reads).
    PhyreScene_setColorScale_0066d3b0 @ 0x0066_d3b0 => extern "thiscall" fn set_color_scale(this: u32, r: u32, g: u32, b: u32, _w3: u32, _w4: u32, _w5: u32, _w6: u32) {
        n::set_color_scale(&mut mem(), this, [r, g, b]);
    }
    /// `SpecularMapParamsManager_setSpecularMaterial_006c6390(inst, name, uv0, uv1, vector1, float1, mat1, v2x, v2y, v2w)` (the 0x80-byte frame).
    SpecularMapParamsManager_setSpecularMaterial_006c6390 @ 0x006c_6390 => extern "stdcall" fn set_specular_material(inst: u32, name: u32, uv0: u32, uv1: u32, vector1: u32, float1: u32, mat1: u32, v2x: u32, v2y: u32, v2w: u32) {
        let mut f = StackFrame::<0x20>::new();
        o::set_specular_material(&mut mem(), f.at(), inst, name, [uv0, uv1], vector1, float1, mat1, [v2x, v2y, v2w]);
    }
    /// `SpecularMapParamsManager_setShadeSpecularMapParams(inst, name, u0, u1, shininess)` (the 0x7c-byte frame).
    SpecularMapParamsManager_setShadeSpecularMapParams @ 0x006c_6830 => extern "stdcall" fn set_shade_specular(inst: u32, name: u32, u0: u32, u1: u32, shininess: u32) {
        let mut f = StackFrame::<0x1f>::new();
        o::set_shade_specular(&mut mem(), f.at(), inst, name, [u0, u1], shininess);
    }
    /// `CharacterAnimationTweakUtil_updateAnimation(this)` (the name buffer).
    CharacterAnimationTweakUtil_updateAnimation @ 0x006b_c130 => extern "thiscall" fn update_animation(this: u32) {
        let mut f = StackFrame::<5>::new();
        o::update_animation(&mut mem(), f.at(), this);
    }
    /// `CharacterAnimationTweakUtil_updateText_006bc750(this)` (the line buffer).
    CharacterAnimationTweakUtil_updateText_006bc750 @ 0x006b_c750 => extern "thiscall" fn animation_text(this: u32) {
        let mut f = StackFrame::<13>::new();
        o::animation_text(&mut mem(), f.at(), this);
    }
    /// `CharacterTextureAnimationTweakUtil_updateText_006bc850(this)` (the line buffer).
    CharacterTextureAnimationTweakUtil_updateText_006bc850 @ 0x006b_c850 => extern "thiscall" fn texanim_text(this: u32) {
        let mut f = StackFrame::<13>::new();
        o::texanim_text(&mut mem(), f.at(), this);
    }
    /// `ObjectList_ctor_0064cec0(this; 2 unread words)` (the insert result and head locals).
    ObjectList_ctor_0064cec0 @ 0x0064_cec0 => extern "thiscall" fn object_list_ctor(this: u32, _w0: u32, _w1: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        o::object_list_ctor(&mut mem(), f.at(), this)
    }
    /// `CA2W_ctor_00651800(this; text, codepage)`.
    CA2W_ctor_00651800 @ 0x0065_1800 => extern "thiscall" fn ca2w(this: u32, text: u32, codepage: u32) {
        o::ca2w(&mut mem(), this, text, codepage);
    }
    /// `DistortionGridDynamicMesh_getValue_006746e0(object, out)`.
    DistortionGridDynamicMesh_getValue_006746e0 @ 0x0067_46e0 => extern "stdcall" fn grid_get_value(object: u32, out: u32) -> u32 {
        o::grid_get_value(&mut mem(), object, out)
    }
    /// `PhyreScene_setLayerGroupVisible_006589e0(scene; group)` -> `al`.
    PhyreScene_setLayerGroupVisible_006589e0 @ 0x0065_89e0 => extern "thiscall" fn set_layer_group(this: u32, group: u32) -> u8 {
        o::set_layer_group(&mut mem(), this, group)
    }
    /// `PhyreScene_linkMapObjects_0066f380(scene)` (cursor frame).
    PhyreScene_linkMapObjects_0066f380 @ 0x0066_f380 => extern "thiscall" fn map_objects_link(this: u32) {
        let mut f = StackFrame::<10>::new();
        o::map_objects_link(&mut mem(), f.at() + 0x28, this);
    }
    /// `PhyreScene_GetVFXSortMode(scene)`.
    PhyreScene_GetVFXSortMode @ 0x0067_21b0 => extern "thiscall" fn vfx_sort_mode(this: u32) -> u32 {
        o::vfx_sort_mode(&mut mem(), this)
    }
    // ---- round 5 batch 3 (ffx_prender::m5r_p) ----
    /// `RadialLine_update_006f8f00(this; hold, color, a, b)`.
    RadialLine_update_006f8f00 @ 0x006f_8f00 => extern "thiscall" fn radial_update(this: u32, hold: u32, color: u32, a: u32, b: u32) {
        p::radial_update(&mut mem(), this, hold as u8, color, a, b);
    }
}
