//! `ffx_glue_other::m5v_*`: M5 stage 1, list `vfx` (the VFX glue, effect
//! debug code and `GDraw` helpers the static call graph reaches and no
//! coverage run ran).
//!
//! Threads: the main thread (`graphicFinalize`, the frame's effect
//! scripts, the unwind funclets of the main thread's constructors); the
//! shims keep no state.

use super::{StackFrame, mem, ported};
use ffx_glue_other::m5v_a::bridge as a;
use ffx_glue_other::m5v_b::bridge as b;
use ffx_glue_other::m5v_c::bridge as c;
use ffx_glue_other::m5v_d::bridge as d;
use ffx_glue_other::m5v_e::bridge as e;
use ffx_glue_other::m5v_f::bridge as f;
use ffx_glue_other::m5v_g::{self as gm, bridge as g};
use ffx_glue_other::m5v_h::bridge as gh;
use ffx_glue_other::m5v_i::bridge as gi;
use ffx_glue_other::m5v_j::bridge as gj;

ported! {
    // ---- batch 1 (ffx_glue_other::m5v_a) ----
    /// `op_loop_end_ffx()`: nothing (its debug branch is a bare `ret`).
    op_loop_end_ffx @ 0x007f_f670 => extern "C" fn op_loop_end_ffx() {
        a::op_loop_end_ffx(&mut mem());
    }
    /// `GraphicUi_Destroy_0067b820()`.
    GraphicUi_Destroy_0067b820 @ 0x0067_b820 => extern "C" fn graphic_ui_destroy() {
        a::graphic_ui_destroy(&mut mem());
    }
    /// `GraphicUi_ReleaseSlotObjects_0067b850(ui)`.
    GraphicUi_ReleaseSlotObjects_0067b850 @ 0x0067_b850 => extern "thiscall" fn graphic_ui_release_slot_objects(this: u32) {
        a::graphic_ui_release_slot_objects(&mut mem(), this);
    }
    /// `ChrShaderRender_FreeBuffers_006a0070(r)`.
    ChrShaderRender_FreeBuffers_006a0070 @ 0x006a_0070 => extern "thiscall" fn chr_shader_free_buffers(this: u32) {
        a::chr_shader_free_buffers(&mut mem(), this);
    }
    /// `ChrShaderRender_Destroy_0069f540()`.
    ChrShaderRender_Destroy_0069f540 @ 0x0069_f540 => extern "C" fn chr_shader_destroy() {
        a::chr_shader_destroy(&mut mem());
    }
    /// `VFXRenderMap_Destroy_0069f580()`: the out iterator in the shim's
    /// frame (`ebp - 4`).
    VFXRenderMap_Destroy_0069f580 @ 0x0069_f580 => extern "C" fn vfx_render_map_destroy() {
        let mut f = StackFrame::<2>::new();
        a::vfx_render_map_destroy(&mut mem(), f.at() + 4);
    }
    /// `VFXRenderDataTable_dtor_0069e900(table)`: the out iterator in the
    /// shim's frame (`ebp - 0x10`).
    VFXRenderDataTable_dtor_0069e900 @ 0x0069_e900 => extern "thiscall" fn vfx_render_data_table_dtor(this: u32) {
        let mut f = StackFrame::<5>::new();
        a::vfx_render_data_table_dtor(&mut mem(), f.at() + 0x10, this);
    }
    /// `VFXRenderDataTable_Destroy_0069f5c0()`.
    VFXRenderDataTable_Destroy_0069f5c0 @ 0x0069_f5c0 => extern "C" fn vfx_render_data_table_destroy() {
        a::vfx_render_data_table_destroy(&mut mem());
    }
    /// `PTreeFreeList_clear_006c0520(t)` -> `Phyre_PFreeList_reset`'s `eax`.
    PTreeFreeList_clear_006c0520 @ 0x006c_0520 => extern "thiscall" fn tree_free_list_clear(this: u32) -> u32 {
        a::tree_free_list_clear(&mut mem(), this)
    }
    /// `VfxShaderManager_dtor_006bfaa0(m)`.
    VfxShaderManager_dtor_006bfaa0 @ 0x006b_faa0 => extern "thiscall" fn vfx_shader_manager_dtor(this: u32) {
        a::vfx_shader_manager_dtor(&mut mem(), this);
    }
    /// `VfxShaderManager_Destroy_006c14a0()`.
    VfxShaderManager_Destroy_006c14a0 @ 0x006c_14a0 => extern "C" fn vfx_shader_manager_destroy() {
        a::vfx_shader_manager_destroy(&mut mem());
    }
    /// `BrokenScreenEffect_Destroy_006d4db0()`.
    BrokenScreenEffect_Destroy_006d4db0 @ 0x006d_4db0 => extern "C" fn broken_screen_destroy() {
        a::broken_screen_destroy(&mut mem());
    }
    /// `graphicVFXAddCopyBufferEvent(a, b)`.
    graphicVFXAddCopyBufferEvent @ 0x0064_4be0 => extern "C" fn add_copy_buffer_event(x: u32, y: u32) {
        a::add_copy_buffer_event(&mut mem(), x, y, 0, 0);
    }
    /// `graphicVFXAddCopyBufferEventWithClearRGB(a, b)`.
    graphicVFXAddCopyBufferEventWithClearRGB @ 0x0064_4c40 => extern "C" fn add_copy_buffer_event_clear_rgb(x: u32, y: u32) {
        a::add_copy_buffer_event(&mut mem(), x, y, 1, 1);
    }
    /// `graphicVFXAddCopyBufferEventWithRGB(a, b)`.
    graphicVFXAddCopyBufferEventWithRGB @ 0x0064_4c70 => extern "C" fn add_copy_buffer_event_rgb(x: u32, y: u32) {
        a::add_copy_buffer_event(&mut mem(), x, y, 0, 1);
    }
    /// `graphicVFXAddDrawCharacterEvent(chr, flag)`: the event in the
    /// shim's frame (`ebp - 0x14`).
    graphicVFXAddDrawCharacterEvent @ 0x0064_4ca0 => extern "C" fn add_draw_character_event(chr: u32, flag: u32) {
        let mut f = StackFrame::<6>::new();
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        a::add_draw_event(&mut mem(), f.at() + 0x14, 5, chr, Some(flag as u8));
    }
    /// `graphicVFXAddDrawZPrePassEvent(chr)`: the event in the shim's
    /// frame (`ebp - 0x14`).
    graphicVFXAddDrawZPrePassEvent @ 0x0064_4cf0 => extern "C" fn add_draw_z_pre_pass_event(chr: u32) {
        let mut f = StackFrame::<6>::new();
        a::add_draw_event(&mut mem(), f.at() + 0x14, 6, chr, None);
    }
    /// `PArray_PushBack_006560c0(array; &value)` -> 0.
    PArray_PushBack_006560c0 @ 0x0065_60c0 => extern "thiscall" fn parray_push_back(this: u32, value: u32) -> u32 {
        a::parray_push_back(&mut mem(), this, value)
    }
    /// `VFXGeometry_CompareDepth(&a, &b)` -> -1 or 1 (writes its argument
    /// slots).
    VFXGeometry_CompareDepth @ 0x0065_9140 => extern "C" fn compare_depth(x: u32, y: u32; slots: u32) -> i32 {
        a::compare_depth(&mut mem(), slots, x, y)
    }
    /// `GraphicUi_AllocBuffers_0067b770(ui)` -> `memset`'s `eax`.
    GraphicUi_AllocBuffers_0067b770 @ 0x0067_b770 => extern "thiscall" fn graphic_ui_alloc_buffers(this: u32) -> u32 {
        a::graphic_ui_alloc_buffers(&mut mem(), this)
    }
    /// `GraphicUi_CreateInstance_0067b6c0()`.
    GraphicUi_CreateInstance_0067b6c0 @ 0x0067_b6c0 => extern "C" fn graphic_ui_create_instance() {
        a::graphic_ui_create_instance(&mut mem());
    }
    /// `MenuTextureManager_CreateInstance_0067b740()`.
    MenuTextureManager_CreateInstance_0067b740 @ 0x0067_b740 => extern "C" fn menu_texture_manager_create_instance() {
        a::menu_texture_manager_create_instance(&mut mem());
    }
    /// `PArray_release_0069e4a0(a)`.
    PArray_release_0069e4a0 @ 0x0069_e4a0 => extern "thiscall" fn parray_release(this: u32) {
        a::parray_release(&mut mem(), this, false);
    }
    /// `PArray_releaseNonNull_0069e4d0(a)`.
    PArray_releaseNonNull_0069e4d0 @ 0x0069_e4d0 => extern "thiscall" fn parray_release_non_null(this: u32) {
        a::parray_release(&mut mem(), this, true);
    }
    /// `PRedBlackTree_dtor_0069e500(t)`.
    PRedBlackTree_dtor_0069e500 @ 0x0069_e500 => extern "thiscall" fn tree_dtor(this: u32) {
        a::tree_dtor(&mut mem(), this);
    }
    /// `std_Map_dtor_0069e770(map)`: the out iterator in the shim's frame
    /// (`ebp - 4`).
    std_Map_dtor_0069e770 @ 0x0069_e770 => extern "thiscall" fn std_map_dtor(this: u32) {
        let mut f = StackFrame::<2>::new();
        a::std_map_dtor(&mut mem(), f.at() + 4, this);
    }
    /// `std_String_dtor_0069e7d0(s)`.
    std_String_dtor_0069e7d0 @ 0x0069_e7d0 => extern "thiscall" fn std_string_dtor(this: u32) {
        a::std_string_dtor(&mut mem(), this);
    }
    /// `ClassCharacter_dtor_0069e830(c)`.
    ClassCharacter_dtor_0069e830 @ 0x0069_e830 => extern "thiscall" fn class_character_dtor(this: u32) {
        a::class_character_dtor(&mut mem(), this);
    }
    /// `std_TreeIterator_dec_0069eca0(it)` -> `it`.
    std_TreeIterator_dec_0069eca0 @ 0x0069_eca0 => extern "thiscall" fn tree_iterator_dec_0069eca0(this: u32) -> u32 {
        a::tree_iterator_dec(&mut mem(), this)
    }
    /// `std_TreeIterator_dec_0069ed00(it)` -> `it`.
    std_TreeIterator_dec_0069ed00 @ 0x0069_ed00 => extern "thiscall" fn tree_iterator_dec_0069ed00(this: u32) -> u32 {
        a::tree_iterator_dec(&mut mem(), this)
    }
    /// `Key64_less_0069eda0(&a, &b)` -> `al` (the rest of `eax` as the
    /// original leaves it).
    Key64_less_0069eda0 @ 0x0069_eda0 => extern "stdcall" fn key64_less(x: u32, y: u32) -> u32 {
        a::key64_less(&mut mem(), x, y)
    }
    /// `ClassDynamicGeometry_vecDelDtor_0069edd0(g; flags)`.
    ClassDynamicGeometry_vecDelDtor_0069edd0 @ 0x0069_edd0 => extern "thiscall" fn dynamic_geometry_vec_del_dtor(this: u32, flags: u32) -> u32 {
        a::dynamic_geometry_vec_del_dtor(&mut mem(), this, flags)
    }
    /// `ClassCharacter_scalarDelDtor_0069ee90(c; flags)`.
    ClassCharacter_scalarDelDtor_0069ee90 @ 0x0069_ee90 => extern "thiscall" fn class_character_scalar_del_dtor(this: u32, flags: u32) -> u32 {
        a::class_character_scalar_del_dtor(&mut mem(), this, flags)
    }
    /// `ClassDynamicGeometry_scalarDelDtor_0069ef00(g; flags)`.
    ClassDynamicGeometry_scalarDelDtor_0069ef00 @ 0x0069_ef00 => extern "thiscall" fn dynamic_geometry_scalar_del_dtor_ef00(this: u32, flags: u32) -> u32 {
        a::dynamic_geometry_scalar_del_dtor(&mut mem(), this, flags)
    }
    /// `ClassDynamicGeometry_scalarDelDtor_0069efa0(g; flags)`.
    ClassDynamicGeometry_scalarDelDtor_0069efa0 @ 0x0069_efa0 => extern "thiscall" fn dynamic_geometry_scalar_del_dtor_efa0(this: u32, flags: u32) -> u32 {
        a::dynamic_geometry_scalar_del_dtor(&mut mem(), this, flags)
    }
    /// `ClassDynamicGeometry_scalarDelDtor_0069efd0(g; flags)`.
    ClassDynamicGeometry_scalarDelDtor_0069efd0 @ 0x0069_efd0 => extern "thiscall" fn dynamic_geometry_scalar_del_dtor_efd0(this: u32, flags: u32) -> u32 {
        a::dynamic_geometry_scalar_del_dtor(&mut mem(), this, flags)
    }
    // ---- batch 2 (ffx_glue_other::m5v_b) ----
    /// `ChrShaderRender_CreateInstance_0069f3a0()`.
    ChrShaderRender_CreateInstance_0069f3a0 @ 0x0069_f3a0 => extern "C" fn chr_shader_create_instance() {
        b::chr_shader_create_instance(&mut mem());
    }
    /// `ChrShaderRender_ShadowEnd_0069fdc0(r)`.
    ChrShaderRender_ShadowEnd_0069fdc0 @ 0x0069_fdc0 => extern "thiscall" fn chr_shadow_end(this: u32) {
        b::chr_shadow_end(&mut mem(), this);
    }
    /// `VFXDynamicGeometry_setPointLightParameters(g; &m0, &m1, f)`
    /// (writes its first argument slot).
    VFXDynamicGeometry_setPointLightParameters @ 0x006a_2fd0 => extern "thiscall" fn set_point_light_parameters(this: u32, m0: u32, m1: u32, f: u32; slots: u32) {
        b::set_point_light_parameters(&mut mem(), slots, this, m0, m1, f);
    }
    /// `VFXDynamicGeometry_setRGBAPower(g; &rgba)`.
    VFXDynamicGeometry_setRGBAPower @ 0x006a_30d0 => extern "thiscall" fn set_rgba_power(this: u32, rgba: u32) {
        b::set_rgba_power(&mut mem(), this, rgba);
    }
    /// `VFXDynamicGeometry_setUVOffset(g; &uv)`.
    VFXDynamicGeometry_setUVOffset @ 0x006a_3a00 => extern "thiscall" fn set_uv_offset(this: u32, uv: u32) {
        b::set_uv_offset(&mut mem(), this, uv);
    }
    /// `VFXDynamicGeometry_setYIQEnable(g; on)`: the YIQ values in the
    /// shim's frame (`ebp - 0x28`..`ebp - 8`).
    VFXDynamicGeometry_setYIQEnable @ 0x006a_3a20 => extern "thiscall" fn set_yiq_enable(this: u32, on: u32) {
        let mut f = StackFrame::<0xb>::new();
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        b::set_yiq_enable(&mut mem(), f.at() + 0x28, this, on as u8);
    }
    /// `VfxShaderNode_copy_006bf170(node; &src)` -> `node` (writes its
    /// argument slot).
    VfxShaderNode_copy_006bf170 @ 0x006b_f170 => extern "thiscall" fn shader_node_copy(this: u32, src: u32; slots: u32) -> u32 {
        b::shader_node_copy(&mut mem(), slots, this, src)
    }
    /// `PTreeIterator_begin_006bf2a0(it; tree)` -> `it`.
    PTreeIterator_begin_006bf2a0 @ 0x006b_f2a0 => extern "thiscall" fn tree_iterator_begin(this: u32, tree: u32) -> u32 {
        b::tree_iterator_begin(&mut mem(), this, tree)
    }
    /// `VfxShader_writeTextureParameter_006c14d0(buffer; param, value)`.
    VfxShader_writeTextureParameter_006c14d0 @ 0x006c_14d0 => extern "thiscall" fn write_texture_parameter(this: u32, param: u32, value: u32) -> u32 {
        b::write_texture_parameter(&mut mem(), this, param, value)
    }
    /// `PRedBlackTree_dtor_006bf590(t)`.
    PRedBlackTree_dtor_006bf590 @ 0x006b_f590 => extern "thiscall" fn tree_dtor_006bf590(this: u32) {
        b::tree_dtor(&mut mem(), this);
    }
    /// `PRedBlackTree_dtor_006bf5f0(t)`.
    PRedBlackTree_dtor_006bf5f0 @ 0x006b_f5f0 => extern "thiscall" fn tree_dtor_006bf5f0(this: u32) {
        b::tree_dtor(&mut mem(), this);
    }
    /// `PRedBlackTree_dtor_006bf650(t)`.
    PRedBlackTree_dtor_006bf650 @ 0x006b_f650 => extern "thiscall" fn tree_dtor_006bf650(this: u32) {
        b::tree_dtor(&mut mem(), this);
    }
    /// `PTreeFreeList_dtor_006bf6b0(t)`.
    PTreeFreeList_dtor_006bf6b0 @ 0x006b_f6b0 => extern "thiscall" fn tree_free_list_dtor_orphan(this: u32) {
        b::tree_free_list_dtor_orphan(&mut mem(), this);
    }
    /// `PTreeFreeList_clear_006c04e0(t)` -> `Phyre_PFreeList_reset`'s `eax`
    /// (a copy of `0x006c0520`).
    PTreeFreeList_clear_006c04e0 @ 0x006c_04e0 => extern "thiscall" fn tree_free_list_clear_006c04e0(this: u32) -> u32 {
        a::tree_free_list_clear(&mut mem(), this)
    }
    /// `PTreeFreeList_dtor_006bf730(t)`.
    PTreeFreeList_dtor_006bf730 @ 0x006b_f730 => extern "thiscall" fn tree_free_list_dtor_006bf730(this: u32) {
        b::tree_free_list_dtor(&mut mem(), this, false);
    }
    /// `PTreeFreeList_dtor_006bf7a0(t)`.
    PTreeFreeList_dtor_006bf7a0 @ 0x006b_f7a0 => extern "thiscall" fn tree_free_list_dtor_006bf7a0(this: u32) {
        b::tree_free_list_dtor(&mut mem(), this, true);
    }
    /// `VfxShader_setClusterTexture_006c1280(cluster, &args)`: the
    /// iterator and record in the shim's frame (`ebp - 0x30`).
    VfxShader_setClusterTexture_006c1280 @ 0x006c_1280 => extern "C" fn set_cluster_texture(cluster: u32, args: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0xc>::new();
        b::set_cluster_texture(&mut mem(), f.at() + 0x30, slots, cluster, args, false)
    }
    /// `VfxShader_setClusterCubeMap_006c1320(cluster, &args)` (writes its
    /// second argument slot).
    VfxShader_setClusterCubeMap_006c1320 @ 0x006c_1320 => extern "C" fn set_cluster_cube_map(cluster: u32, args: u32; slots: u32) -> u32 {
        let mut f = StackFrame::<0xc>::new();
        b::set_cluster_texture(&mut mem(), f.at() + 0x30, slots, cluster, args, true)
    }
    /// `BrokenScreenEffect_releaseObjects_006d3e00(e)`.
    BrokenScreenEffect_releaseObjects_006d3e00 @ 0x006d_3e00 => extern "thiscall" fn broken_screen_release_objects(this: u32) {
        b::broken_screen_release_objects(&mut mem(), this);
    }
    /// `BrokenScreen_vertexStride_006d3f50(e)` -> 0x18, 0x24 or 0.
    BrokenScreen_vertexStride_006d3f50 @ 0x006d_3f50 => extern "thiscall" fn vertex_stride(this: u32) -> u32 {
        b::vertex_stride(&mut mem(), this)
    }
    // ---- batch 3 (ffx_glue_other::m5v_c) ----
    /// `pppDrawFilterDes(id)` -> `graphicVFXDestroy`'s `eax`.
    pppDrawFilterDes @ 0x0075_7250 => extern "C" fn draw_filter_des(id: u32) -> u32 {
        c::draw_filter_des(&mut mem(), id)
    }
    /// `TkWaitSemaBGThread_00888880()` -> `WaitSema`'s `eax`.
    TkWaitSemaBGThread_00888880 @ 0x0088_8880 => extern "C" fn wait_sema_bg_thread() -> u32 {
        c::wait_sema_bg_thread(&mut mem())
    }
    /// `pppSendWait()`.
    pppSendWait @ 0x0076_b4a0 => extern "C" fn send_wait() {
        c::send_wait(&mut mem());
    }
    /// `InitVFXRenderDataRects(lo, hi, p, arg)`.
    InitVFXRenderDataRects @ 0x0071_1350 => extern "C" fn init_rects(lo: u32, hi: u32, p: u32, arg: u32) {
        c::init_rects(&mut mem(), lo, hi, p, arg);
    }
    /// `InitVFXRenderDataShp_OShapeVu1(lo, hi, a, b, c)`.
    InitVFXRenderDataShp_OShapeVu1 @ 0x0071_10d0 => extern "C" fn init_shp_vu1(lo: u32, hi: u32, x: u32, y: u32, z: u32) {
        c::init_shp_vu1(&mut mem(), lo, hi, x, y, z, true);
    }
    /// `InitVFXRenderData_KaGra001(lo, hi, a, b, c)`.
    InitVFXRenderData_KaGra001 @ 0x0071_1500 => extern "C" fn init_ka_gra001(lo: u32, hi: u32, x: u32, y: u32, z: u32) {
        c::init_shp_vu1(&mut mem(), lo, hi, x, y, z, false);
    }
    /// `VfxShaderManager_combineTextureNameInCurrentLevel(name, out)`: the
    /// folder buffer in the shim's frame (`ebp - 0x108`..`ebp - 4`).
    VfxShaderManager_combineTextureNameInCurrentLevel @ 0x006c_0560 => extern "stdcall" fn combine_texture_name(name: u32, out: u32) {
        let mut f = StackFrame::<0x43>::new();
        c::combine_texture_name(&mut mem(), f.at() + 0x108, name, out);
    }
    /// `VfxShaderTree_insert_006be9e0(tree; node)` -> 0 or the existing node.
    VfxShaderTree_insert_006be9e0 @ 0x006b_e9e0 => extern "thiscall" fn tree_insert(this: u32, node: u32) -> u32 {
        c::tree_insert(&mut mem(), this, node)
    }
    // ---- batch 4 (ffx_glue_other::m5v_d): the effect dump's operand printers ----
    /// `opd_print_u_unit_max_00a64c00(op)` -> `(i16)op[2]`.
    opd_print_u_unit_max_00a64c00 @ 0x00a6_4c00 => extern "C" fn print_u_unit_max(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_bf74, op)
    }
    /// `opd_print_u_et_no_00a64c20(op)` -> `(i16)op[2]`.
    opd_print_u_et_no_00a64c20 @ 0x00a6_4c20 => extern "C" fn print_u_et_no(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_bf88, op)
    }
    /// `opd_print_u_v_00a64c40(op)`: the line in the shim's frame.
    opd_print_u_v_00a64c40 @ 0x00a6_4c40 => extern "C" fn print_u_v(op: u32) {
        let mut f = StackFrame::<0x42>::new();
        d::print_flags(&mut mem(), f.at() + 0x104, 0x00b7_bfb4, op);
    }
    /// `opd_print_u_s_00a64ca0(op)`: the line in the shim's frame.
    opd_print_u_s_00a64ca0 @ 0x00a6_4ca0 => extern "C" fn print_u_s(op: u32) {
        let mut f = StackFrame::<0x42>::new();
        d::print_flags(&mut mem(), f.at() + 0x104, 0x00b7_bfc0, op);
    }
    /// `opd_print_u_sp_00a64d00(op)`: the line in the shim's frame.
    opd_print_u_sp_00a64d00 @ 0x00a6_4d00 => extern "C" fn print_u_sp(op: u32) {
        let mut f = StackFrame::<0x42>::new();
        d::print_flags(&mut mem(), f.at() + 0x104, 0x00b7_bfcc, op);
    }
    /// `opd_print_u_unit_00a64d60(op)` -> the unit number.
    opd_print_u_unit_00a64d60 @ 0x00a6_4d60 => extern "C" fn print_u_unit(op: u32) -> u32 {
        d::print_unit(&mut mem(), 0x00b7_bfd8, op)
    }
    /// `opd_print_u_task_00a64d90(op)` -> the target.
    opd_print_u_task_00a64d90 @ 0x00a6_4d90 => extern "C" fn print_u_task(op: u32) -> u32 {
        d::print_task(&mut mem(), 0x00b7_bfec, 0x00b7_c004, op)
    }
    /// `opd_print_u_task_off_00a64de0(op)` -> the task number.
    opd_print_u_task_off_00a64de0 @ 0x00a6_4de0 => extern "C" fn print_u_task_off(op: u32) -> u32 {
        d::print_task_off(&mut mem(), 0x00b7_c018, op)
    }
    /// `opd_print_e_anm_00a64e00(op)` -> `(i16)op[2]`.
    opd_print_e_anm_00a64e00 @ 0x00a6_4e00 => extern "C" fn print_e_anm(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_c02c, op)
    }
    /// `opd_print_e_flash_00a64e20(op)`.
    opd_print_e_flash_00a64e20 @ 0x00a6_4e20 => extern "C" fn print_e_flash(op: u32) {
        d::print_plain(&mut mem(), 0x00b7_c03c, op);
    }
    /// `opd_print_e_anmm_00a64e40(op)` -> `(i16)op[2]` or 0xf000.
    opd_print_e_anmm_00a64e40 @ 0x00a6_4e40 => extern "C" fn print_e_anmm(op: u32) -> u32 {
        d::print_anmm(&mut mem(), 0x00b7_c048, op)
    }
    /// `opd_print_u_texture_load_00a64e70(op)` -> `(i16)op[2]`.
    opd_print_u_texture_load_00a64e70 @ 0x00a6_4e70 => extern "C" fn print_u_texture_load(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_c058, op)
    }
    /// `opd_print_u_clut_load_00a64e90(op)` -> `(i16)op[2]`.
    opd_print_u_clut_load_00a64e90 @ 0x00a6_4e90 => extern "C" fn print_u_clut_load(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_c070, op)
    }
    /// `opd_print_u_tc_load_00a64eb0(op)` -> `(i16)op[2]`.
    opd_print_u_tc_load_00a64eb0 @ 0x00a6_4eb0 => extern "C" fn print_u_tc_load(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_c088, op)
    }
    /// `opd_print_u_world_00a64ed0(op)`.
    opd_print_u_world_00a64ed0 @ 0x00a6_4ed0 => extern "C" fn print_u_world(op: u32) {
        d::print_plain(&mut mem(), 0x00b7_c09c, op);
    }
    /// `opd_print_e_md_00a64ef0(op)` -> `(i16)op[2]`.
    opd_print_e_md_00a64ef0 @ 0x00a6_4ef0 => extern "C" fn print_e_md(op: u32) -> u32 {
        d::print_half(&mut mem(), 0x00b7_c0ac, op)
    }
    /// `opd_print_e_gline_00a64f10(op)` (its `eax` after a print is
    /// `(i16)op[4]`; the dispatcher drops it).
    opd_print_e_gline_00a64f10 @ 0x00a6_4f10 => extern "C" fn print_e_gline(op: u32) {
        let _ = d::print_gline(&mut mem(), 0x00b7_c0bc, op);
    }
    /// `opd_print_u_part_init_00a64f40(op)` -> `(i16)op[2]`.
    opd_print_u_part_init_00a64f40 @ 0x00a6_4f40 => extern "C" fn print_u_part_init(op: u32) -> u32 {
        d::print_part_init(&mut mem(), 0x00b7_c0d0, op)
    }
    /// `opd_print_e_weapon_shadow_00a64f60(op)` -> `op` (writes its
    /// argument slot).
    opd_print_e_weapon_shadow_00a64f60 @ 0x00a6_4f60 => extern "C" fn print_e_weapon_shadow(op: u32; slots: u32) -> u32 {
        d::print_weapon_shadow(&mut mem(), slots, 0x00b7_c0ec, op)
    }
    // ---- batch 5 (ffx_glue_other::m5v_e): code outside the dump ----
    /// `EtBindDefault_setBit12_007fb510()` -> 0x1000.
    EtBindDefault_setBit12_007fb510 @ 0x007f_b510 => extern "C" fn bind_set_bit12() -> u32 {
        e::bind_flags(&mut mem(), 0x1000, true)
    }
    /// `EtBindDefault_clearBit12_007fb520()` -> 0xefff.
    EtBindDefault_clearBit12_007fb520 @ 0x007f_b520 => extern "C" fn bind_clear_bit12() -> u32 {
        e::bind_flags(&mut mem(), 0xefff, false)
    }
    /// `EtBindDefault_setBit11_007fb680()` -> 0x800.
    EtBindDefault_setBit11_007fb680 @ 0x007f_b680 => extern "C" fn bind_set_bit11() -> u32 {
        e::bind_flags(&mut mem(), 0x0800, true)
    }
    /// `EtBindDefault_setBit13_007fb720()` -> 0x2000.
    EtBindDefault_setBit13_007fb720 @ 0x007f_b720 => extern "C" fn bind_set_bit13() -> u32 {
        e::bind_flags(&mut mem(), 0x2000, true)
    }
    /// `EtBindDefault_clearBit13_007fb730()` -> 0xdfff.
    EtBindDefault_clearBit13_007fb730 @ 0x007f_b730 => extern "C" fn bind_clear_bit13() -> u32 {
        e::bind_flags(&mut mem(), 0xdfff, false)
    }
    /// `op_debug_reset_008005f0()` -> 0.
    op_debug_reset_008005f0 @ 0x0080_05f0 => extern "C" fn op_debug_reset() -> u32 {
        e::op_debug_reset(&mut mem())
    }
    /// `jmp op_gs1_init` (`0x007eb760`): its `eax`.
    op_gs1_init_thunk_007eb570 @ 0x007e_b570 => extern "C" fn t_7eb570(work: u32, flags: u32) -> u32 {
        e::jmp_thunk(&mut mem(), const { ffx_glue_other::m5v_e::target(0x007e_b570) }, &[work, flags])
    }
    /// `jmp opus_change_chrhd_ffx_to_battle` (`0x007fe200`): its `eax`.
    opus_change_chrhd_ffx_to_battle_thunk_007fc290 @ 0x007f_c290 => extern "C" fn t_7fc290() -> u32 {
        e::jmp_thunk(&mut mem(), const { ffx_glue_other::m5v_e::target(0x007f_c290) }, &[])
    }
    /// `jmp op_oef2_tex0_get_007e5260` (`0x007e5260`): its `eax`.
    op_oef2_tex0_get_thunk_00808a90 @ 0x0080_8a90 => extern "C" fn t_808a90(oef: u32, no: u32, out: u32) -> u32 {
        e::jmp_thunk(&mut mem(), const { ffx_glue_other::m5v_e::target(0x0080_8a90) }, &[oef, no, out])
    }
    /// `jmp op_et_bindeff_draw_off` (`0x007fba40`): its `eax`.
    op_et_bindeff_draw_off_thunk @ 0x0092_d860 => extern "C" fn t_92d860(h: u32) -> u32 {
        e::jmp_thunk(&mut mem(), const { ffx_glue_other::m5v_e::target(0x0092_d860) }, &[h])
    }
    /// `jmp op_et_bindeff_draw_on` (`0x007fbab0`): its `eax`.
    op_et_bindeff_draw_on_thunk @ 0x0092_d870 => extern "C" fn t_92d870(h: u32) -> u32 {
        e::jmp_thunk(&mut mem(), const { ffx_glue_other::m5v_e::target(0x0092_d870) }, &[h])
    }
    // ---- round 5 batch 1 (ffx_glue_other::m5v_f) ----
    /// `pppFpFreeAll_007296a0()` (native 0x8008; its `eax` unread).
    pppFpFreeAll_007296a0 @ 0x0072_96a0 => extern "C" fn fp_free_all() {
        f::fp_free_all(&mut mem());
    }
    /// `pppFpResetSameIds_00729740()` (native 0x8007; its `eax` unread).
    pppFpResetSameIds_00729740 @ 0x0072_9740 => extern "C" fn fp_reset_same_ids() {
        f::fp_reset_same_ids(&mut mem());
    }
    /// `pppFpClearFrames_007297e0()` (native 0x805d; its `eax` unread).
    pppFpClearFrames_007297e0 @ 0x0072_97e0 => extern "C" fn fp_clear_frames() {
        f::fp_clear_frames(&mut mem());
    }
    /// `op_et_chr_field_mode_update_007fb690()` (native 0x0244).
    op_et_chr_field_mode_update_007fb690 @ 0x007f_b690 => extern "C" fn et_chr_field_mode_update() {
        f::et_chr_field_mode_update(&mut mem());
    }
    /// `opu_free_all_pobjects_00800970()` -> 0.
    opu_free_all_pobjects_00800970 @ 0x0080_0970 => extern "C" fn opu_free_all_pobjects() -> u32 {
        f::opu_free_all_pobjects(&mut mem())
    }
    /// `opu_run_battle_host_00800a30()` -> 0.
    opu_run_battle_host_00800a30 @ 0x0080_0a30 => extern "C" fn opu_run_battle_host() -> u32 {
        f::opu_run_battle_host(&mut mem())
    }
    /// `opus_map_back_color_restore_0080a4b0()`.
    opus_map_back_color_restore_0080a4b0 @ 0x0080_a4b0 => extern "C" fn back_color_restore() {
        f::back_color_restore(&mut mem());
    }
    /// `opus_map_back_color_set_0080a4d0(rgba)` -> `yiSetBackColor`'s
    /// `eax`: the four integers in this shim's frame.
    opus_map_back_color_set_0080a4d0 @ 0x0080_a4d0 => extern "C" fn back_color_set(rgba: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        f::back_color_set(&mut mem(), frame.at(), rgba)
    }
    /// `Pad_PortClose_00888990(port, slot)`.
    Pad_PortClose_00888990 @ 0x0088_8990 => extern "C" fn pad_port_close(port: u32, slot: u32) {
        f::pad_port_close(&mut mem(), port, slot);
    }
    /// `MsDbgFile_freeTree_007e1b70(node)`.
    MsDbgFile_freeTree_007e1b70 @ 0x007e_1b70 => extern "C" fn dbg_file_free_tree(node: u32) {
        f::dbg_file_free_tree(&mut mem(), node);
    }
    /// `MsDbgFile_hashNames_007e2d90(table)` -> the last name pointer.
    MsDbgFile_hashNames_007e2d90 @ 0x007e_2d90 => extern "C" fn dbg_file_hash_names(table: u32) -> u32 {
        f::dbg_file_hash_names(&mut mem(), table)
    }
    /// `op_file_read(rec, name)` -> `sceClose`'s `eax` or the negative
    /// descriptor.
    op_file_read @ 0x007e_2ed0 => extern "C" fn op_file_read(rec: u32, name: u32) -> u32 {
        f::op_file_read(&mut mem(), rec, name)
    }
    /// `op_debug_file_load_007e5c10(name, buf)` -> bytes read or 0.
    op_debug_file_load_007e5c10 @ 0x007e_5c10 => extern "C" fn debug_file_load(name: u32, buf: u32) -> u32 {
        f::debug_file_load(&mut mem(), name, buf)
    }
    /// `gdraw_SetTextureUniqueID_00a3abc0(handle, old, new)` -> the id after.
    gdraw_SetTextureUniqueID_00a3abc0 @ 0x00a3_abc0 => extern "stdcall" fn set_texture_unique_id(handle: u32, old: u32, new: u32) -> u32 {
        f::set_texture_unique_id(&mut mem(), handle, old, new)
    }
    /// `gdraw_Set3DTransform_00a3a9b0(m)` -> the `GDraw` state.
    gdraw_Set3DTransform_00a3a9b0 @ 0x00a3_a9b0 => extern "stdcall" fn set_3d_transform(m: u32) -> u32 {
        f::set_3d_transform(&mut mem(), m)
    }
    /// `gdraw_TryLock_00a3b010(handle, id, stats)` -> 1/0.
    gdraw_TryLock_00a3b010 @ 0x00a3_b010 => extern "stdcall" fn try_lock_b010(handle: u32, id: u32, _stats: u32) -> u32 {
        f::try_lock(&mut mem(), handle, id)
    }
    /// `gdraw_TryLock_00a3b050(handle, id, stats)` -> 1/0.
    gdraw_TryLock_00a3b050 @ 0x00a3_b050 => extern "stdcall" fn try_lock_b050(handle: u32, id: u32, _stats: u32) -> u32 {
        f::try_lock(&mut mem(), handle, id)
    }
    /// `gdraw_TryLock_00a3b0c0(handle, id, stats)` -> 1/0.
    gdraw_TryLock_00a3b0c0 @ 0x00a3_b0c0 => extern "stdcall" fn try_lock_b0c0(handle: u32, id: u32, _stats: u32) -> u32 {
        f::try_lock(&mut mem(), handle, id)
    }
    /// `gdraw_Unlock_00a3b100(handle, _, _)` -> `+0x30`.
    gdraw_Unlock_00a3b100 @ 0x00a3_b100 => extern "stdcall" fn unlock(handle: u32, _a: u32, _b: u32) -> u32 {
        f::unlock(&mut mem(), handle)
    }
    /// `gdraw_UpdateTextureRect_00a3b160(handle, _, x, y, pitch, w, h,
    /// data, _)` -> the method's `eax`: the box in this shim's frame.
    gdraw_UpdateTextureRect_00a3b160 @ 0x00a3_b160 => extern "stdcall" fn update_texture_rect(handle: u32, _a: u32, x: u32, y: u32, pitch: u32, w: u32, h: u32, data: u32, _b: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        f::update_texture_rect(&mut mem(), frame.at(), handle, x, y, pitch, w, h, data)
    }
    /// `gdraw_reportD3DError_00a3c170(hr, what, where)` -> the warning's
    /// `eax`.
    gdraw_reportD3DError_00a3c170 @ 0x00a3_c170 => extern "C" fn report_d3d_error(hr: u32, what: u32, where_: u32) -> u32 {
        f::report_d3d_error(&mut mem(), hr, what, where_)
    }
    /// `gdraw_FreeAll_00a380b0()`: the stats in this shim's frame.
    gdraw_FreeAll_00a380b0 @ 0x00a3_80b0 => extern "C" fn free_all() {
        let mut frame = StackFrame::<12>::new();
        f::free_all(&mut mem(), frame.at());
    }
    /// `gdraw_FreeRenderTarget_00a37ac0(t)` -> `FUN_00a3b480`'s `eax`: the
    /// stats in this shim's frame.
    gdraw_FreeRenderTarget_00a37ac0 @ 0x00a3_7ac0 => extern "stdcall" fn free_render_target(t: u32) -> u32 {
        let mut frame = StackFrame::<12>::new();
        f::free_render_target(&mut mem(), frame.at(), t)
    }
    // ---- round 5 batch 2 (ffx_glue_other::m5v_g) ----
    /// `ChrY0Shadow_createMesh(r)` -> the last virtual call's `eax`: the
    /// sampler record in this shim's frame.
    ChrY0Shadow_createMesh @ 0x0069_f9d0 => extern "thiscall" fn chr_y0_shadow_create_mesh(this: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        g::create_mesh(&mut mem(), frame.at(), this)
    }
    /// `VFXDynamicGeometry_setTkRuntimeClutParameters(g; c0, c1, f)`: the
    /// floats in this shim's frame; writes its argument slots.
    VFXDynamicGeometry_setTkRuntimeClutParameters @ 0x006a_37f0 => extern "thiscall" fn set_tk_runtime_clut(this: u32, c0: u32, c1: u32, f: u32; slots: u32) {
        let mut frame = StackFrame::<9>::new();
        g::set_tk_runtime_clut(&mut mem(), slots, frame.at(), this, c0, c1, f);
    }
    /// `ClassDynamicGeometry_resizeSegments_006a44d0(g; n, _, _)` -> 0 / 0xd.
    ClassDynamicGeometry_resizeSegments_006a44d0 @ 0x006a_44d0 => extern "thiscall" fn resize_segments(this: u32, n: u32, _b: u32, _c: u32) -> u32 {
        g::resize_segments(&mut mem(), this, n)
    }
    /// `VfxShaderManager_copyShaderMap_006bee80(map; src)` -> `map`.
    VfxShaderManager_copyShaderMap_006bee80 @ 0x006b_ee80 => extern "thiscall" fn copy_shader_map(this: u32, src: u32) -> u32 {
        g::copy_shader_map(&mut mem(), this, src)
    }
    /// `createVFXRenderDataBufferRects(lo, hi, n, flags)` -> 1: the
    /// original's 0x68b4-byte frame in this shim's.
    createVFXRenderDataBufferRects @ 0x0071_4c50 => extern "C" fn create_rects(lo: u32, hi: u32, n: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (gm::RECTS_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + gm::RECTS_FRAME;
        g::create_rects(&mut mem(), ebp, lo, hi, n, flags)
    }
    /// `createGuildMapRenderDataBuffer(lo, hi, model, info)` -> 1 / 0.
    createGuildMapRenderDataBuffer @ 0x0071_33e0 => extern "C" fn create_guild_map(lo: u32, hi: u32, model: u32, info: u32) -> u32 {
        let mut frame = StackFrame::<{ (gm::GUILD_MAP_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + gm::GUILD_MAP_FRAME;
        g::create_guild_map(&mut mem(), ebp, lo, hi, model, info)
    }
    /// `createVFXRenderDataBufferShp_OShapeVu1(lo, hi, shape, flags)` -> 1 / 0.
    createVFXRenderDataBufferShp_OShapeVu1 @ 0x0071_5810 => extern "C" fn create_shp_vu1(lo: u32, hi: u32, shape: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (gm::SHP_VU1_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + gm::SHP_VU1_FRAME;
        g::create_shp_vu1(&mut mem(), ebp, lo, hi, shape, flags)
    }
    /// `createU16MeshBreakRenderDataBuffer(lo, hi, model, chr, flags)` -> 1.
    createU16MeshBreakRenderDataBuffer @ 0x0080_2fb0 => extern "C" fn create_u16_mesh_break(lo: u32, hi: u32, model: u32, chr: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (gm::U16_BREAK_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + gm::U16_BREAK_FRAME;
        g::create_u16_mesh_break(&mut mem(), ebp, lo, hi, model, chr, flags)
    }
    /// `createGT3MeshBreakRenderDataBuffer(lo, hi, model, flags)` -> 1 / 0.
    createGT3MeshBreakRenderDataBuffer @ 0x00a7_3c20 => extern "C" fn create_gt3_mesh_break(lo: u32, hi: u32, model: u32, flags: u32) -> u32 {
        let mut frame = StackFrame::<{ (gm::GT3_BREAK_FRAME / 4) as usize }>::new();
        let ebp = frame.at() + gm::GT3_BREAK_FRAME;
        g::create_gt3_mesh_break(&mut mem(), ebp, lo, hi, model, flags)
    }
    // ---- round 5 batch 3 (ffx_glue_other::m5v_h) ----
    /// `gdraw_freeHandle_00a3b480(handle, stats)` -> the free-ring head.
    gdraw_freeHandle_00a3b480 @ 0x00a3_b480 => extern "C" fn gdraw_free_handle(handle: u32, stats: u32) -> u32 {
        gh::free_handle(&mut mem(), handle, stats)
    }
    /// `GDraw_pool_reclaim_00a3b500(pool, stats)` -> 1/0.
    GDraw_pool_reclaim_00a3b500 @ 0x00a3_b500 => extern "C" fn gdraw_pool_reclaim(pool: u32, stats: u32) -> u32 {
        gh::pool_reclaim(&mut mem(), pool, stats)
    }
    /// `gdraw_unbindTarget_00a3b590()` -> the target: the null buffer and
    /// stride in this shim's frame.
    gdraw_unbindTarget_00a3b590 @ 0x00a3_b590 => extern "C" fn gdraw_unbind_target() -> u32 {
        let mut frame = StackFrame::<2>::new();
        gh::unbind_target(&mut mem(), frame.at())
    }
    /// `gdraw_setViewportForTarget_00a3cd30()` -> the viewport's `eax`.
    gdraw_setViewportForTarget_00a3cd30 @ 0x00a3_cd30 => extern "C" fn gdraw_set_viewport_for_target() -> u32 {
        gh::set_viewport_for_target(&mut mem())
    }
    /// `gdraw_setTextureUnit_00a3ccb0(unit, texture, wrap, filter)` -> the
    /// last method's `eax`: a null view in its argument slot.
    gdraw_setTextureUnit_00a3ccb0 @ 0x00a3_ccb0 => extern "C" fn gdraw_set_texture_unit(unit: u32, texture: u32, wrap: u32, filter: u32; slots: u32) -> u32 {
        gh::set_texture_unit(&mut mem(), slots + 4, unit, texture, wrap, filter)
    }
    /// `gdraw_resetState_00a3c280()` -> the `GDraw` state.
    gdraw_resetState_00a3c280 @ 0x00a3_c280 => extern "C" fn gdraw_reset_state() -> u32 {
        gh::reset_state(&mut mem())
    }
    /// `gdraw_setProjection_00a3c450(x0, x1, y0, y1)` -> the `GDraw` state;
    /// writes its argument slots.
    gdraw_setProjection_00a3c450 @ 0x00a3_c450 => extern "C" fn gdraw_set_projection(x0: u32, x1: u32, y0: u32, y1: u32; slots: u32) -> u32 {
        gh::set_projection(&mut mem(), slots, x0, x1, y0, y1)
    }
    /// `gdraw_bindOutput_00a3c570(stats)` -> `RSSetState`'s `eax`: the
    /// target in this shim's frame.
    gdraw_bindOutput_00a3c570 @ 0x00a3_c570 => extern "C" fn gdraw_bind_output(stats: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        gh::bind_output(&mut mem(), frame.at(), stats)
    }
    /// `opu_v_u2(host, cmd)` (`ffx_vfx::opus`; parity in `vfx.rs`
    /// `r53e_opu_v_u2_matches_the_original`): nibbles other than 0 and
    /// 0x1000 read the never-written frame words `ebp - 8` / `- 0x10` as
    /// record pointers; the port uses 0 (brief 2026-10-24) and faults on
    /// the null record where the original reads stale stack, which the
    /// junk-read rule accepts (brief 2026-10-27).
    opu_v_u2 @ 0x0081_9c90 => extern "C" fn opu_v_u2(host: u32, cmd: u32) -> u32 {
        ffx_vfx::opus::bridge::opu_v_u2(&mut mem(), host, cmd, &mut |_, f, a| super::call_cdecl(f, a))
    }
    // ---- round 5 batch 4 (ffx_glue_other::m5v_i) ----
    /// `pppCrosshairPacket_00760ce0(x0, y0, x1, y1, w, h)` -> `sceDmaSend`'s
    /// `eax`.
    pppCrosshairPacket_00760ce0 @ 0x0076_0ce0 => extern "C" fn crosshair_packet(x0: u32, y0: u32, x1: u32, y1: u32, w: u32, h: u32) -> u32 {
        gi::crosshair_packet(&mut mem(), x0, y0, x1, y1, w, h)
    }
    /// `pppLoadClutBlocks_00762810(src, dbp)`: the load-image record in this
    /// shim's frame.
    pppLoadClutBlocks_00762810 @ 0x0076_2810 => extern "C" fn load_clut_blocks(src: u32, dbp: u32) -> u32 {
        let mut frame = StackFrame::<0x18>::new();
        gi::load_clut_blocks(&mut mem(), frame.at(), src, dbp)
    }
    /// `rpc_send_command_0076b030(rec, text)` -> 0.
    rpc_send_command_0076b030 @ 0x0076_b030 => extern "C" fn rpc_send_command(rec: u32, text: u32) -> u32 {
        gi::rpc_send_command(&mut mem(), rec, text)
    }
    /// `rpc_read(rec)` -> the receive buffer.
    rpc_read @ 0x0076_b4f0 => extern "C" fn rpc_read(rec: u32) -> u32 {
        gi::rpc_read(&mut mem(), rec)
    }
    // ---- round 5 batch 5 (ffx_glue_other::m5v_j) ----
    /// `opu_dbg(host, cmd)` -> the next command.
    opu_dbg @ 0x0081_5e80 => extern "C" fn opu_dbg(host: u32, cmd: u32) -> u32 {
        gj::opu_dbg(&mut mem(), host, cmd)
    }
    /// `op_et_bind_all_off_007fbc20()`.
    op_et_bind_all_off_007fbc20 @ 0x007f_bc20 => extern "C" fn bind_all_off() {
        gj::bind_all_off(&mut mem());
    }
    /// `op_et_bind_all_on_007fb940()`.
    op_et_bind_all_on_007fb940 @ 0x007f_b940 => extern "C" fn bind_all_on() {
        gj::bind_all_on(&mut mem());
    }
    /// `opd_num_end_tag_00a5f2d0(p, end)` -> 0x50.
    opd_num_end_tag_00a5f2d0 @ 0x00a5_f2d0 => extern "C" fn num_end_tag(p: u32, end: u32) -> u32 {
        gj::num_end_tag(&mut mem(), p, end)
    }
    /// `opd_num_tex_tag_00a5e7b0(p, tbw, pal)` -> 0x40.
    opd_num_tex_tag_00a5e7b0 @ 0x00a5_e7b0 => extern "C" fn num_tex_tag(p: u32, tbw: u32, pal: u32) -> u32 {
        gj::num_tex_tag(&mut mem(), p, tbw, pal)
    }
    /// `MsDbgFile_charClasses_007e12d0(table, s1, ..., s7)`.
    MsDbgFile_charClasses_007e12d0 @ 0x007e_12d0 => extern "C" fn char_classes(table: u32, s1: u32, s2: u32, s3: u32, s4: u32, s5: u32, s6: u32, s7: u32) {
        gj::char_classes(&mut mem(), table, [s1, s2, s3, s4, s5, s6, s7]);
    }
}
