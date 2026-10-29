//! M5 stage 1, list `phyre-core`, batch 3 (`ffx_pcore::m5p_core_c`):
//! lookups, `PShaderPass` copies, the shader context switches'
//! evaluators, the lazily built array descriptors' registers (one
//! generic port, a table of each copy's static), `PAnimatableComponent`'s
//! elements, the animation key's hash and lookup, the rigid body's
//! forwards.
//!
//! Threads: the evaluators run where shaders are set up (main thread,
//! loader); the rest where objects are made and animated. No shim keeps
//! state; the locals callees see are per-call `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core_c::{self as c, bridge as b};

ported! {
    /// `Phyre_PNamedTree_findByName_0047d330(tree; name)` -> the object or 0.
    Phyre_PNamedTree_findByName_0047d330 @ 0x0047_d330 => extern "thiscall" fn find_by_name(this: u32, name: u32) -> u32 {
        b::find_by_name(&mut mem(), this, name)
    }
    /// `Phyre_PIndexBuffer_indices32_0049bd10(buffer; &out)` -> the 32-bit indices or 0.
    Phyre_PIndexBuffer_indices32_0049bd10 @ 0x0049_bd10 => extern "thiscall" fn indices32(this: u32, out: u32) -> u32 {
        b::indices32(&mut mem(), this, out)
    }
    /// `Phyre_PFreeList_merge_0049c730(list; &other)` -> 0 or 5.
    Phyre_PFreeList_merge_0049c730 @ 0x0049_c730 => extern "thiscall" fn free_list_merge(this: u32, other: u32) -> u32 {
        b::free_list_merge(&mut mem(), this, other)
    }
    /// `Phyre_PShaderPass_copyArray2_004bbc30(this; &src)` -> `this`.
    Phyre_PShaderPass_copyArray2_004bbc30 @ 0x004b_bc30 => extern "thiscall" fn pass_array2_copy(this: u32, src: u32) -> u32 {
        b::pass_array_copy(&mut mem(), this, src, 2, 2, c::COPY2)
    }
    /// `Phyre_PShaderPass_copyArray16_004bbdc0(this; &src)` -> `this`.
    Phyre_PShaderPass_copyArray16_004bbdc0 @ 0x004b_bdc0 => extern "thiscall" fn pass_array16_copy(this: u32, src: u32) -> u32 {
        b::pass_array_copy(&mut mem(), this, src, 16, 4, c::COPY16)
    }
    /// `Phyre_PShaderPass_copy_004bbcb0(this; &src)` -> `this`.
    Phyre_PShaderPass_copy_004bbcb0 @ 0x004b_bcb0 => extern "thiscall" fn shader_pass_copy(this: u32, src: u32) -> u32 {
        b::shader_pass_copy(&mut mem(), this, src)
    }
    /// `Phyre_PContextSwitchShaderLODLevel_evaluate_004d4430(shader, segment, a2, cursor, a4, a5, a6)` -> 0.
    Phyre_PContextSwitchShaderLODLevel_evaluate_004d4430 @ 0x004d_4430 => extern "stdcall" fn lod_level(shader: u32, segment: u32, _a2: u32, cursor: u32, _a4: u32, _a5: u32, _a6: u32) -> u32 {
        b::lod_level(&mut mem(), shader, segment, cursor)
    }
    /// `Phyre_PContextSwitchShaderOrthoCamera_evaluate_004d4480(a0, a1, a2, cursor, a4, &camera)` -> 0.
    Phyre_PContextSwitchShaderOrthoCamera_evaluate_004d4480 @ 0x004d_4480 => extern "stdcall" fn ortho_camera(_a0: u32, _a1: u32, _a2: u32, cursor: u32, _a4: u32, camera: u32) -> u32 {
        b::ortho_camera(&mut mem(), cursor, camera)
    }
    /// `Phyre_PContextSwitch_lightTypeDefine_004d46a0(switch; &cursor, a1, &defines)` -> 0 or 0xd; the builder in a per-call frame.
    Phyre_PContextSwitch_lightTypeDefine_004d46a0 @ 0x004d_46a0 => extern "thiscall" fn light_define(this: u32, cursor: u32, _a1: u32, defines: u32) -> u32 {
        let mut frame = StackFrame::<3>::new();
        b::light_define(&mut mem(), this, cursor, defines, frame.at())
    }
    /// `Phyre_PContextSwitchShaderLODLevel_segmentArrays_004d5400(shader, &arrays)` -> 0 or 0xd.
    Phyre_PContextSwitchShaderLODLevel_segmentArrays_004d5400 @ 0x004d_5400 => extern "stdcall" fn lod_segment_arrays(shader: u32, arrays: u32) -> u32 {
        b::lod_segment_arrays(&mut mem(), shader, arrays)
    }
    /// `PClassDescriptorConcrete_PArray_PAnimationChannel_register_00512740(p)`.
    PClassDescriptorConcrete_PArray_PAnimationChannel_register_00512740 @ 0x0051_2740 => extern "C" fn lazy_00512740(p: u32) -> u32 {
        b::lazy_register(&mut mem(), p, const { c::lazy_of(0x0051_2740) })
    }
    /// `PClassDescriptorConcrete_PArray_PAnimationConstantChannel_register_005127d0(p)`.
    PClassDescriptorConcrete_PArray_PAnimationConstantChannel_register_005127d0 @ 0x0051_27d0 => extern "C" fn lazy_005127d0(p: u32) -> u32 {
        b::lazy_register(&mut mem(), p, const { c::lazy_of(0x0051_27d0) })
    }
    /// `PClassDescriptorConcrete_PArray_PAnimationChannel_register_00512880(p)` (flag 0).
    PClassDescriptorConcrete_PArray_PAnimationChannel_register_00512880 @ 0x0051_2880 => extern "C" fn lazy_00512880(p: u32) -> u32 {
        b::lazy_register(&mut mem(), p, const { c::lazy_of(0x0051_2880) })
    }
    /// `PClassDescriptorConcrete_PArray_PAnimationConstantChannel_register_00512910(p)` (flag 0).
    PClassDescriptorConcrete_PArray_PAnimationConstantChannel_register_00512910 @ 0x0051_2910 => extern "C" fn lazy_00512910(p: u32) -> u32 {
        b::lazy_register(&mut mem(), p, const { c::lazy_of(0x0051_2910) })
    }
    /// `Phyre_PArray_setToExternalBuffer_0052ab30(array; data, n)` -> 0.
    Phyre_PArray_setToExternalBuffer_0052ab30 @ 0x0052_ab30 => extern "thiscall" fn set_external_0052ab30(this: u32, data: u32, n: u32) -> u32 {
        b::slot_filters_set_external(&mut mem(), this, data, n)
    }
    /// `Phyre_PAnimationTargetBlenderController_validate_00531b40(this; a, b)` -> `al`.
    Phyre_PAnimationTargetBlenderController_validate_00531b40 @ 0x0053_1b40 => extern "thiscall" fn validate_pair(this: u32, a: u32, v: u32) -> u32 {
        b::validate_pair(&mut mem(), this, a, v)
    }
    /// `Phyre_PAnimatableComponent_setByKey_005cd660(this; key, v)`.
    Phyre_PAnimatableComponent_setByKey_005cd660 @ 0x005c_d660 => extern "thiscall" fn element_set(this: u32, key: u32, v: u32) -> u32 {
        b::element_set(&mut mem(), this, key, v)
    }
    /// `Phyre_PAnimatableComponent_getByKey_005cd6b0(this; key)`.
    Phyre_PAnimatableComponent_getByKey_005cd6b0 @ 0x005c_d6b0 => extern "thiscall" fn element_get(this: u32, key: u32) -> u32 {
        b::element_get(&mut mem(), this, key)
    }
    /// `Phyre_PAnimatableComponent_setByKeyB_005cd700(this; key, v)`.
    Phyre_PAnimatableComponent_setByKeyB_005cd700 @ 0x005c_d700 => extern "thiscall" fn element_set_b(this: u32, key: u32, v: u32) -> u32 {
        b::element_set_b(&mut mem(), this, key, v)
    }
    /// `Phyre_PAnimatableComponent_elementDtor_005cd4c0(e)`.
    Phyre_PAnimatableComponent_elementDtor_005cd4c0 @ 0x005c_d4c0 => extern "thiscall" fn element_dtor(this: u32) -> u32 {
        b::element_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimationKey_hash_005ccd70(&key)`.
    Phyre_PAnimationKey_hash_005ccd70 @ 0x005c_cd70 => extern "C" fn key_hash(key: u32) -> u32 {
        b::key_hash(&mut mem(), key)
    }
    /// `Phyre_PAnimationKeyMap_indexOf_005cce50(this; &key)` -> the index or -1; the key's copy in a per-call frame.
    Phyre_PAnimationKeyMap_indexOf_005cce50 @ 0x005c_ce50 => extern "thiscall" fn key_index(this: u32, key: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        b::key_index(&mut mem(), this, key, frame.at())
    }
    /// `Phyre_PPhysicsRigidBody_forwardSlot18_005d3e30(this)`.
    Phyre_PPhysicsRigidBody_forwardSlot18_005d3e30 @ 0x005d_3e30 => extern "thiscall" fn rigid_body_slot6(this: u32) -> u32 {
        b::rigid_body_slot6(&mut mem(), this)
    }
}
