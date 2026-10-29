//! M5 stage 1, list `phyre-core` (`ffx_pcore::m5p_core`): `PhyreEngine`'s
//! small core members, the `ForType<T>` descriptor destructors and the
//! owned-array resets of this list (one generic port each, a table of
//! per-copy constants), and byte copies of members ported elsewhere
//! (their shims call those ports).
//!
//! Threads: the pools' members run on the pool threads they start; the
//! rest where objects are made, loaded and freed. No shim keeps state;
//! the one local a callee sees is a per-call `StackFrame`.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core::{self as c, bridge as b};
use ffx_prender::{class_slots as slots, m5r_f};

ported! {
    /// `PClassDescriptorForType_Phyre_PString_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PString__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PString_Destruct @ 0x0044_5360 => extern "thiscall" fn destruct_00445360(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0044_5360) })
    }
    /// `PClassDescriptorForType_Phyre_PSharray_Phyre_PString_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PString__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PSharray_Phyre_PString_Destruct @ 0x0044_aee0 => extern "thiscall" fn destruct_0044aee0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0044_aee0) })
    }
    /// `PClassDescriptorForType_Phyre_PClusterDependencyList_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PClusterDependencyList__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PClusterDependencyList_Destruct @ 0x0044_aef0 => extern "thiscall" fn destruct_0044aef0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0044_aef0) })
    }
    /// `PClassDescriptorForType_Phyre_PWorldMatrix_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PWorldMatrix__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PWorldMatrix_Destruct @ 0x0045_1030 => extern "thiscall" fn destruct_00451030(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0045_1030) })
    }
    /// `PClassDescriptorForType_Phyre_PCamera_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PCamera__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PCamera_Destruct @ 0x0045_32e0 => extern "thiscall" fn destruct_004532e0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0045_32e0) })
    }
    /// `PClassDescriptorForType_Phyre_PCameraProjection_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PCameraProjection__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PCameraProjection_Destruct @ 0x0045_32f0 => extern "thiscall" fn destruct_004532f0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0045_32f0) })
    }
    /// `PClassDescriptorForType_Phyre_PRandom_PRandomGenerator_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PRandom_PRandomGenerator__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PRandom_PRandomGenerator_Destruct @ 0x0045_e860 => extern "thiscall" fn destruct_0045e860(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0045_e860) })
    }
    /// `PClassDescriptorForType_Phyre_PArray_unsigned_char_4_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_4__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PArray_unsigned_char_4_Destruct @ 0x0049_9da0 => extern "thiscall" fn destruct_00499da0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0049_9da0) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PShape_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PShape__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PShape_Destruct @ 0x0049_9db0 => extern "thiscall" fn destruct_00499db0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0049_9db0) })
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4_Destruct @ 0x004c_9890 => extern "thiscall" fn destruct_004c9890(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x004c_9890) })
    }
    /// `PClassDescriptorForType_Phyre_PRendering_PShadowCaster_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PRendering_PShadowCaster__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PRendering_PShadowCaster_Destruct @ 0x004c_98a0 => extern "thiscall" fn destruct_004c98a0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x004c_98a0) })
    }
    /// `PClassDescriptorForType_Phyre_PRendering_PShadowCaster_PShadowSplit_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PRendering_PShadowCaster_PShadowSplit__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PRendering_PShadowCaster_PShadowSplit_Destruct @ 0x004c_98b0 => extern "thiscall" fn destruct_004c98b0(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x004c_98b0) })
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDataBlockD3D11_4_Destruct @ 0x0055_df20 => extern "thiscall" fn destruct_0055df20(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df20) })
    }
    /// `PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PDataBlockBufferD3D11_Destruct @ 0x0055_df30 => extern "thiscall" fn destruct_0055df30(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df30) })
    }
    /// `PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PSharray_Phyre_PGeometry_PIndexDataBlockBufferD3D11_Destruct @ 0x0055_df40 => extern "thiscall" fn destruct_0055df40(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df40) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PDataBlockBufferD3D11_Destruct @ 0x0055_df50 => extern "thiscall" fn destruct_0055df50(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df50) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PDataBlockD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PDataBlockD3D11_Destruct @ 0x0055_df60 => extern "thiscall" fn destruct_0055df60(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df60) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBufferD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBufferD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBufferD3D11_Destruct @ 0x0055_df70 => extern "thiscall" fn destruct_0055df70(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df70) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockD3D11_Destruct @ 0x0055_df80 => extern "thiscall" fn destruct_0055df80(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df80) })
    }
    /// `PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentD3D11_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentD3D11__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentD3D11_Destruct @ 0x0055_df90 => extern "thiscall" fn destruct_0055df90(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x0055_df90) })
    }
    /// `PClassDescriptorForType_Phyre_PArray_Phyre_PLOD_PLODLevel_4_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PLOD_PLODLevel_4__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PArray_Phyre_PLOD_PLODLevel_4_Destruct @ 0x005a_2820 => extern "thiscall" fn destruct_005a2820(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x005a_2820) })
    }
    /// `PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstance_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstance__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMeshInstance_Destruct @ 0x005a_2830 => extern "thiscall" fn destruct_005a2830(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x005a_2830) })
    }
    /// `PClassDescriptorForType_Phyre_PLOD_PLODGroup_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PLOD_PLODGroup__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PLOD_PLODGroup_Destruct @ 0x005a_2840 => extern "thiscall" fn destruct_005a2840(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x005a_2840) })
    }
    /// `PClassDescriptorForType_Phyre_PLOD_PLODLevel_Destruct(this)`: `[this] = Phyre_PClassDescriptorForType_Phyre_PLOD_PLODLevel__vftable`, then `Phyre_PClassDescriptor_Destruct`'s `eax`.
    PClassDescriptorForType_Phyre_PLOD_PLODLevel_Destruct @ 0x005a_2850 => extern "thiscall" fn destruct_005a2850(this: u32) -> u32 {
        m5r_f::bridge::descriptor_destruct(&mut mem(), this, const { c::vtable_of(0x005a_2850) })
    }
    /// `PArray_Reset_00431b60(this)`: the owned-array reset.
    PArray_Reset_00431b60 @ 0x0043_1b60 => extern "thiscall" fn reset_00431b60(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_00431d20(this)`: the owned-array reset.
    PArray_Reset_00431d20 @ 0x0043_1d20 => extern "thiscall" fn reset_00431d20(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_004338d0(this)`: the owned-array reset.
    PArray_Reset_004338d0 @ 0x0043_38d0 => extern "thiscall" fn reset_004338d0(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_00433960(this)`: the owned-array reset.
    PArray_Reset_00433960 @ 0x0043_3960 => extern "thiscall" fn reset_00433960(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_004339d0(this)`: the owned-array reset.
    PArray_Reset_004339d0 @ 0x0043_39d0 => extern "thiscall" fn reset_004339d0(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_00444a80(this)`: the owned-array reset.
    PArray_Reset_00444a80 @ 0x0044_4a80 => extern "thiscall" fn reset_00444a80(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `Phyre_PStringBuilder_dtor_PStringBuilder(this)`: the owned-array reset.
    Phyre_PStringBuilder_dtor_PStringBuilder @ 0x0044_4ab0 => extern "thiscall" fn reset_00444ab0(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_00499d20(this)`: the owned-array reset.
    PArray_Reset_00499d20 @ 0x0049_9d20 => extern "thiscall" fn reset_00499d20(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_005cd430(this)`: the owned-array reset.
    PArray_Reset_005cd430 @ 0x005c_d430 => extern "thiscall" fn reset_005cd430(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    /// `PArray_Reset_005cd460(this)`: the owned-array reset.
    PArray_Reset_005cd460 @ 0x005c_d460 => extern "thiscall" fn reset_005cd460(this: u32) -> u32 {
        b::owned_reset(&mut mem(), this)
    }
    // ---- byte copies of members ported elsewhere ----
    /// `Phyre_PStringBuilder_reserve_00444e10(this; n)`: the code of `Phyre_PArray_u8_resize_0044a010`.
    Phyre_PStringBuilder_reserve_00444e10 @ 0x0044_4e10 => extern "thiscall" fn reserve_00444e10(this: u32, n: u32) -> u32 {
        ffx_pload::descriptor::bridge::array_resize(&mut mem(), this, n)
    }
    /// `PMatrix4_identity_00454cf0(out)` -> `out`: the code of `PMatrix4_Identity_00451bb0`.
    PMatrix4_identity_00454cf0 @ 0x0045_4cf0 => extern "C" fn identity_00454cf0(out: u32) -> u32 {
        ffx_pcore::matrix::bridge::identity(&mut mem(), out)
    }
    /// `Phyre_PRedBlackTree_successor_005751d0(tree; node)`: the code of `Phyre_PRedBlackTree_successor`.
    Phyre_PRedBlackTree_successor_005751d0 @ 0x0057_51d0 => extern "thiscall" fn successor_005751d0(this: u32, node: u32) -> u32 {
        ffx_pother_a::p3_tree::bridge::successor(&mut mem(), this, node)
    }
    /// `Phyre_PArray_setData_0049c100(this; data, count)`: the code of `Phyre_PArray_u8_setData_0044a2d0`.
    Phyre_PArray_setData_0049c100 @ 0x0049_c100 => extern "thiscall" fn set_data_0049c100(this: u32, data: u32, count: i32) -> u32 {
        ffx_pload::cluster::bridge::array_set_data(&mut mem(), this, data, count)
    }
    /// `Phyre_PArray_setToExternalBuffer_004baac0(array; buffer, count)` -> 0: the code of `0x004839d0`.
    Phyre_PArray_setToExternalBuffer_004baac0 @ 0x004b_aac0 => extern "thiscall" fn set_external_004baac0(array: u32, buffer: u32, count: u32) -> u32 {
        slots::bridge::set_external(&mut mem(), array, buffer, count, slots::Release::Free)
    }
    /// `Phyre_PArray_setToExternalBuffer_004cd600(array; buffer, count)` -> 0: the code of `0x004839d0`.
    Phyre_PArray_setToExternalBuffer_004cd600 @ 0x004c_d600 => extern "thiscall" fn set_external_004cd600(array: u32, buffer: u32, count: u32) -> u32 {
        slots::bridge::set_external(&mut mem(), array, buffer, count, slots::Release::Free)
    }
    /// `PSharray_setToExternalBuffer_004f8f90(this; data, count)` -> 0 or 5: the code of `0x0048c650`.
    PSharray_setToExternalBuffer_004f8f90 @ 0x004f_8f90 => extern "thiscall" fn sharray_set_004f8f90(this: u32, data: u32, count: u32) -> u32 {
        ffx_prender::h1_small2::bridge::sharray_set(&mut mem(), this, data, count)
    }
    /// `Phyre_PPair_init_005cd3f0(pair)` -> `pair`: the code of `Phyre_PPair_init_004677d0`.
    Phyre_PPair_init_005cd3f0 @ 0x005c_d3f0 => extern "thiscall" fn pair_init_005cd3f0(this: u32) -> u32 {
        ffx_pother_a::object::bridge::pair_init(&mut mem(), this)
    }
    /// `Phyre_PThreadPoolImmediate_workerThread(arg)` -> 0 (vtable slot 0; `xor eax, eax; ret 4`).
    Phyre_PThreadPoolImmediate_workerThread @ 0x0057_d0a0 => extern "stdcall" fn worker_0057d0a0(_arg: u32) -> u32 { 0 }
    /// `Phyre_PSharray_element28_00585c30(array; i)`: the code of `Phyre_PSharray_element28_00585c90`.
    Phyre_PSharray_element28_00585c30 @ 0x0058_5c30 => extern "thiscall" fn element_00585c30(this: u32, i: u32) -> u32 {
        ffx_prender::array::bridge::element(&mut mem(), this, i, 0x1c)
    }
    /// `Phyre_PArray_setStorage_00a347b0(array; data, count)`: the code of `Phyre_PArray_setStorage_004f02e0`.
    Phyre_PArray_setStorage_00a347b0 @ 0x00a3_47b0 => extern "thiscall" fn set_storage_00a347b0(a0: u32, a1: u32, a2: u32) -> u32 {
        ffx_prender::array::bridge::set_storage(&mut mem(), a0, a1, a2, None)
    }
    // ---- the list's own members ----
    /// `Phyre_PString_dtor_PString(this)`.
    Phyre_PString_dtor_PString @ 0x0042_f120 => extern "thiscall" fn string_dtor(this: u32) {
        b::string_dtor(&mut mem(), this);
    }
    /// `Phyre_PThreadInitializationBase_GetDefaultThreadName()` -> the text.
    Phyre_PThreadInitializationBase_GetDefaultThreadName @ 0x0043_17a0 => extern "C" fn default_thread_name() -> u32 {
        b::default_thread_name(&mem())
    }
    /// `Phyre_PThreadWin32_join_004301b0(thread; &code)` -> 0; the exit code's local in a per-call frame.
    Phyre_PThreadWin32_join_004301b0 @ 0x0043_01b0 => extern "thiscall" fn join_004301b0(this: u32, code: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        b::thread_join(&mut mem(), this, code, frame.at())
    }
    /// `Phyre_CloseHandle_00430240(this)` -> 0.
    Phyre_CloseHandle_00430240 @ 0x0043_0240 => extern "thiscall" fn close_00430240(this: u32) -> u32 {
        b::close_handle(&mut mem(), this)
    }
    /// `Phyre_CloseHandle_00430250(this)` -> 0.
    Phyre_CloseHandle_00430250 @ 0x0043_0250 => extern "thiscall" fn close_00430250(this: u32) -> u32 {
        b::close_handle(&mut mem(), this)
    }
    /// `Phyre_PUtility_dtor_PUtility(this)`.
    Phyre_PUtility_dtor_PUtility @ 0x0043_1d50 => extern "thiscall" fn utility_dtor(this: u32) {
        b::utility_dtor(&mut mem(), this);
    }
    /// `PMeshSegmentArray_reset_00433990(this)`.
    PMeshSegmentArray_reset_00433990 @ 0x0043_3990 => extern "thiscall" fn segments_reset(this: u32) -> u32 {
        b::segments_reset(&mut mem(), this)
    }
    /// `Phyre_PAnnotation_dtor_PAnnotation(this)` -> the previous node.
    Phyre_PAnnotation_dtor_PAnnotation @ 0x0057_5920 => extern "thiscall" fn annotation_dtor(this: u32) -> u32 {
        b::annotation_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnnotation_scalarDeletingDtor_005759f0(this; flags)` -> `this`.
    Phyre_PAnnotation_scalarDeletingDtor_005759f0 @ 0x0057_59f0 => extern "thiscall" fn annotation_delete(this: u32, flags: u32) -> u32 {
        b::annotation_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PClassDescriptorDynamic_registerMember_0044a450(descriptor; offset, flag)` -> `al` 1.
    Phyre_PClassDescriptorDynamic_registerMember_0044a450 @ 0x0044_a450 => extern "thiscall" fn register_0044a450(this: u32, offset: u32, flag: u32) -> u32 {
        b::register_member(&mut mem(), this, offset, flag)
    }
    /// `Phyre_PThreadPool_threadState_init_0044cc20(this)` -> `this`.
    Phyre_PThreadPool_threadState_init_0044cc20 @ 0x0044_cc20 => extern "thiscall" fn pool_state_init(this: u32) -> u32 {
        b::pool_state_init(&mut mem(), this)
    }
    /// `Phyre_PThreadPool_dtor_PThreadPool(this)`.
    Phyre_PThreadPool_dtor_PThreadPool @ 0x0044_cc60 => extern "thiscall" fn pool_dtor(this: u32) -> u32 {
        b::pool_dtor(&mut mem(), this)
    }
    /// `Phyre_PThreadPool_deleting_dtor_0044ccd0(this; flags)` -> `this`.
    Phyre_PThreadPool_deleting_dtor_0044ccd0 @ 0x0044_ccd0 => extern "thiscall" fn pool_delete(this: u32, flags: u32) -> u32 {
        b::pool_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PThreadPool_threadEntry_0044ce30(arg)` -> the worker's `eax`.
    Phyre_PThreadPool_threadEntry_0044ce30 @ 0x0044_ce30 => extern "C" fn thread_entry(arg: u32) -> u32 {
        b::thread_entry(&mut mem(), arg)
    }
    /// `PMatrix4_toWorldMatrix3x4_jump_00452730(dst; m)`.
    PMatrix4_toWorldMatrix3x4_jump_00452730 @ 0x0045_2730 => extern "thiscall" fn to_world_00452730(this: u32, m: u32) -> u32 {
        b::to_world_matrix(&mut mem(), this, m)
    }
    /// `Phyre_Vector4_setW_00452790(this; w)` -> `this`.
    Phyre_Vector4_setW_00452790 @ 0x0045_2790 => extern "thiscall" fn set_w(this: u32, v: u32) -> u32 {
        b::set_float(&mut mem(), this, 0xc, v)
    }
    /// `Phyre_Vector4_setX_00454e50(this; x)` -> `this`.
    Phyre_Vector4_setX_00454e50 @ 0x0045_4e50 => extern "thiscall" fn set_x(this: u32, v: u32) -> u32 {
        b::set_float(&mut mem(), this, 0, v)
    }
    /// `Phyre_Vector4_setY_00454e60(this; y)` -> `this`.
    Phyre_Vector4_setY_00454e60 @ 0x0045_4e60 => extern "thiscall" fn set_y(this: u32, v: u32) -> u32 {
        b::set_float(&mut mem(), this, 4, v)
    }
    /// `Phyre_Vector4_setZ_00454e70(this; z)` -> `this`.
    Phyre_Vector4_setZ_00454e70 @ 0x0045_4e70 => extern "thiscall" fn set_z(this: u32, v: u32) -> u32 {
        b::set_float(&mut mem(), this, 8, v)
    }
    /// `Phyre_PCameraProjection_setBC_00454e10(this; v)` -> the update's `eax`.
    Phyre_PCameraProjection_setBC_00454e10 @ 0x0045_4e10 => extern "thiscall" fn projection_bc(this: u32, v: u32) -> u32 {
        b::set_float_update(&mut mem(), this, 0xbc, v)
    }
    /// `Phyre_PCameraProjection_setB8_00454e30(this; v)` -> the update's `eax`.
    Phyre_PCameraProjection_setB8_00454e30 @ 0x0045_4e30 => extern "thiscall" fn projection_b8(this: u32, v: u32) -> u32 {
        b::set_float_update(&mut mem(), this, 0xb8, v)
    }
    /// `Phyre_PContextSwitchShader16BitIndices_evaluate_004d43f0(a0, a1, a2, cursor, a4, shader)` -> 0.
    Phyre_PContextSwitchShader16BitIndices_evaluate_004d43f0 @ 0x004d_43f0 => extern "stdcall" fn index16_flag(_a0: u32, _a1: u32, _a2: u32, cursor: u32, _a4: u32, shader: u32) -> u32 {
        b::index16_flag(&mut mem(), cursor, shader)
    }
    /// `Phyre_PMatrixArray_set_004f8e80(array; i, m)`.
    Phyre_PMatrixArray_set_004f8e80 @ 0x004f_8e80 => extern "thiscall" fn matrix_set(this: u32, i: u32, m: u32) -> u32 {
        b::matrix_set(&mut mem(), this, i, m)
    }
    /// `Phyre_PMeshInstance_setOwnerIfPositive_004f9260(this; &v)`.
    Phyre_PMeshInstance_setOwnerIfPositive_004f9260 @ 0x004f_9260 => extern "thiscall" fn owner_if_positive(this: u32, v: u32) {
        b::set_owner_if_positive(&mut mem(), this, v);
    }
    /// `Phyre_maxFloat_005126e0(a, b)` -> `ST0`.
    Phyre_maxFloat_005126e0 @ 0x0051_26e0 => extern "C" fn max_float(a: u32, b: u32) -> f32 {
        b::max_float(a, b)
    }
    /// `Phyre_PThreadPoolJob_setVftable_005cd3c0(this)`.
    Phyre_PThreadPoolJob_setVftable_005cd3c0 @ 0x005c_d3c0 => extern "thiscall" fn job_vtable_005cd3c0(this: u32) {
        b::job_vtable(&mut mem(), this);
    }
    /// `Phyre_PThreadPoolJob_setVftable_005cd3b0(this)`.
    Phyre_PThreadPoolJob_setVftable_005cd3b0 @ 0x005c_d3b0 => extern "thiscall" fn job_vtable_005cd3b0(this: u32) {
        b::job_vtable(&mut mem(), this);
    }
    /// `Phyre_PAnimatableComponent_deleteElements_005cd490(this)`.
    Phyre_PAnimatableComponent_deleteElements_005cd490 @ 0x005c_d490 => extern "thiscall" fn delete_elements(this: u32) -> u32 {
        b::element_array_delete(&mut mem(), this)
    }
    /// `PMatrix4_setRow_00574680(this; i, &v)` -> `this`.
    PMatrix4_setRow_00574680 @ 0x0057_4680 => extern "thiscall" fn set_row(this: u32, i: u32, v: u32) -> u32 {
        b::set_row(&mut mem(), this, i, v)
    }
    /// `Phyre_PRedBlackTree_erase_0057cb70(tree; &key)`.
    Phyre_PRedBlackTree_erase_0057cb70 @ 0x0057_cb70 => extern "thiscall" fn tree_erase(this: u32, key: u32) -> u32 {
        b::tree_erase(&mut mem(), this, key)
    }
    /// `Phyre_PThreadPoolImmediate_scalarDeletingDtor(this; flags)` -> `this`.
    Phyre_PThreadPoolImmediate_scalarDeletingDtor @ 0x0057_cf20 => extern "thiscall" fn immediate_delete(this: u32, flags: u32) -> u32 {
        b::immediate_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PThreadPoolImmediate_shutdown_0057d050(this)`.
    Phyre_PThreadPoolImmediate_shutdown_0057d050 @ 0x0057_d050 => extern "thiscall" fn immediate_shutdown(this: u32) -> u32 {
        b::free_buffer(&mut mem(), this)
    }
    /// `Phyre_PThreadPoolImmediate_runJob_0057d070(this; job)` -> 0.
    Phyre_PThreadPoolImmediate_runJob_0057d070 @ 0x0057_d070 => extern "thiscall" fn immediate_run(this: u32, job: u32) -> u32 {
        b::run_job(&mut mem(), this, job)
    }
    /// `Phyre_PSemaphoreFlag_init_0057d0f0(this)` -> `this`.
    Phyre_PSemaphoreFlag_init_0057d0f0 @ 0x0057_d0f0 => extern "thiscall" fn semaphore_flag_init(this: u32) -> u32 {
        b::semaphore_flag_init(&mut mem(), this)
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineBase_scalarDeletingDtor(this; flags)` -> `this`.
    Phyre_PProfiling_PProcessorProfileTimelineBase_scalarDeletingDtor @ 0x0057_d2e0 => extern "thiscall" fn timeline_delete(this: u32, flags: u32) -> u32 {
        b::timeline_delete(&mut mem(), this, flags)
    }
    /// `Phyre_uninitializedCopy12_005a1030(dst, src, n)`.
    Phyre_uninitializedCopy12_005a1030 @ 0x005a_1030 => extern "C" fn copy12(dst: u32, src: u32, n: u32) {
        b::copy12(&mut mem(), dst, src, n);
    }
    /// `Phyre_PLockedList_init_005ccfb0(this)` -> `this`.
    Phyre_PLockedList_init_005ccfb0 @ 0x005c_cfb0 => extern "thiscall" fn locked_list_init(this: u32) -> u32 {
        b::locked_list_init(&mut mem(), this)
    }
    /// `Phyre_skipEmptyNodes_005cd0c0(end, &cursor)` -> 0.
    Phyre_skipEmptyNodes_005cd0c0 @ 0x005c_d0c0 => extern "C" fn skip_empty(end: u32, cursor: u32) -> u32 {
        b::skip_empty(&mut mem(), end, cursor)
    }
    /// `Phyre_PPhysicsRigidBody_forwardSlot20_005d3e90(this)`.
    Phyre_PPhysicsRigidBody_forwardSlot20_005d3e90 @ 0x005d_3e90 => extern "thiscall" fn rigid_body_slot(this: u32) -> u32 {
        b::rigid_body_slot8(&mut mem(), this)
    }
    /// `MorphManager_destroyInstance_006abeb0()`.
    MorphManager_destroyInstance_006abeb0 @ 0x006a_beb0 => extern "C" fn morph_manager_destroy() {
        b::morph_manager_destroy(&mut mem());
    }
}
