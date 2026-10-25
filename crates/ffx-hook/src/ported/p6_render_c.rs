//! M4 port round 1, list p6-render-c (`PhyreEngine` render, highest third
//! by address): the shims. Ports in `ffx_prender` (`descriptor`, ...).
//!
//! Class-descriptor callbacks (`ffx_prender::descriptor`): `thiscall(desc,
//! instance)`, `ret 4`, called through the descriptors' vtables by the
//! serializer and the loader (main thread and the async init job).

use super::{mem, ported};
use ffx_prender::{descriptor, misc};

ported! {
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_unsigned_char_const_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_unsigned_char_const_4__constructInstance @ 0x005a_da10 => extern "thiscall" fn d_005ada10(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__constructInstance @ 0x005a_da30 => extern "thiscall" fn d_005ada30(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__constructInstance @ 0x005a_da50 => extern "thiscall" fn d_005ada50(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_const_4__destroyInstance` (`array_free`).
    Phyre_PClassDescriptorForType_Phyre_PArray_unsigned_char_const_4__destroyInstance @ 0x005a_dff0 => extern "thiscall" fn d_005adff0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__destroyInstance` (`array_free_nn`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__destroyInstance @ 0x005a_e030 => extern "thiscall" fn d_005ae030(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__destroyInstance` (`array_free_nn`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__destroyInstance @ 0x005a_e070 => extern "thiscall" fn d_005ae070(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh__destroyInstance @ 0x005a_e0b0 => extern "thiscall" fn d_005ae0b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__destroyInstance @ 0x005a_e0e0 => extern "thiscall" fn d_005ae0e0(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh__fixObjectsAfterLoading @ 0x005a_e2d0 => extern "thiscall" fn d_005ae2d0(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__fixObjectsAfterLoading @ 0x005a_e2e0 => extern "thiscall" fn d_005ae2e0(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PBase_const_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PBase_const_4__constructInstance @ 0x005b_2990 => extern "thiscall" fn d_005b2990(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput__constructInstance` (`true1`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput__constructInstance @ 0x005b_2a20 => extern "thiscall" fn d_005b2a20(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PBase_const_4__destroyInstance` (`array_free`).
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PBase_const_4__destroyInstance @ 0x005b_2a30 => extern "thiscall" fn d_005b2a30(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstance__destroyInstance` (`destroy_with`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstance__destroyInstance @ 0x005b_2a70 => extern "thiscall" fn d_005b2a70(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_with(&mut mem(), 0x005b_0e40, obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput__destroyInstance @ 0x005b_2a90 => extern "thiscall" fn d_005b2a90(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInstancePacketInput__destroyInstance @ 0x005b_2aa0 => extern "thiscall" fn d_005b2aa0(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkInstance__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkInstance__fixObjectsAfterLoading @ 0x005b_2ac0 => extern "thiscall" fn d_005b2ac0(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__constructInstance @ 0x005c_5af0 => extern "thiscall" fn d_005c5af0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__constructInstance @ 0x005c_5b10 => extern "thiscall" fn d_005c5b10(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__constructInstance @ 0x005c_5b30 => extern "thiscall" fn d_005c5b30(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer__constructInstance` (`true1`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer__constructInstance @ 0x005c_5bf0 => extern "thiscall" fn d_005c5bf0(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance__constructInstance` (`true1`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance__constructInstance @ 0x005c_5c50 => extern "thiscall" fn d_005c5c50(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PRenderStream__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PRenderStream__constructInstance @ 0x005c_5c60 => extern "thiscall" fn d_005c5c60(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorHeader_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__constructInstance` (`true1`).
    Phyre_PClassDescriptorHeader_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__constructInstance @ 0x005c_5ca0 => extern "thiscall" fn d_005c5ca0(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__destroyInstance` (`array_free_with`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__destroyInstance @ 0x005c_5cc0 => extern "thiscall" fn d_005c5cc0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free_with(&mut mem(), 0x005c_1ef0, arr)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__destroyInstance` (`array_free_nn`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__destroyInstance @ 0x005c_5d00 => extern "thiscall" fn d_005c5d00(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__destroyInstance` (`array_free_nn`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__destroyInstance @ 0x005c_5d40 => extern "thiscall" fn d_005c5d40(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, true)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierAndInputs__destroyInstance` (`array_free_nn_c`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierAndInputs__destroyInstance @ 0x005c_5d80 => extern "thiscall" fn d_005c5d80(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), obj, 0xc, true)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_84` (`destroy_with`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_84 @ 0x005c_5dc0 => extern "thiscall" fn d_005c5dc0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_with(&mut mem(), 0x005b_f7d0, obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkBuffer__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkBuffer__destroyInstance @ 0x005c_5de0 => extern "thiscall" fn d_005c5de0(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_Buffer__destroyInstance @ 0x005c_5e00 => extern "thiscall" fn d_005c5e00(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierCode__destroyInstance @ 0x005c_5e10 => extern "thiscall" fn d_005c5e10(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket_ModifierInstance__destroyInstance @ 0x005c_5e20 => extern "thiscall" fn d_005c5e20(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStream__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStream__destroyInstance @ 0x005c_5e30 => extern "thiscall" fn d_005c5e30(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStreamInput__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PDynamicGeometry_PRenderStreamInput__destroyInstance @ 0x005c_5e40 => extern "thiscall" fn d_005c5e40(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierAndInputs__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierAndInputs__fixObjectsAfterLoading @ 0x005c_5ee0 => extern "thiscall" fn d_005c5ee0(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_d8` (`zero1`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_d8 @ 0x005c_5ef0 => extern "thiscall" fn d_005c5ef0(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkBuffer__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkBuffer__fixObjectsAfterLoading @ 0x005c_5f00 => extern "thiscall" fn d_005c5f00(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__fixObjectsAfterLoading @ 0x005c_5f10 => extern "thiscall" fn d_005c5f10(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PRenderStream__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PRenderStream__fixObjectsAfterLoading @ 0x005c_5f50 => extern "thiscall" fn d_005c5f50(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PRenderStreamInput__fixObjectsAfterLoading` (`zero1`).
    Phyre_PClassDescriptorAbstract_Phyre_PDynamicGeometry_PRenderStreamInput__fixObjectsAfterLoading @ 0x005c_5f60 => extern "thiscall" fn d_005c5f60(_desc: u32, _obj: u32) -> u32 {
        0
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PFramework_PApplicationViewport__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PFramework_PApplicationViewport__destroyInstance @ 0x0062_7060 => extern "thiscall" fn d_00627060(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PFramework_PInputMapper__destroyInstance` (`true1`).
    Phyre_PClassDescriptorForType_Phyre_PFramework_PInputMapper__destroyInstance @ 0x0062_7090 => extern "thiscall" fn d_00627090(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `Phyre_PClassDescriptorForType_ClassDynamicMesh__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_ClassDynamicMesh__destroyInstance @ 0x0067_2b50 => extern "thiscall" fn d_00672b50(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputAction_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputAction_4__constructInstance @ 0x009e_afd0 => extern "thiscall" fn d_009eafd0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputSource_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputSource_4__constructInstance @ 0x009e_aff0 => extern "thiscall" fn d_009eaff0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputMap__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputMap__constructInstance @ 0x009e_b040 => extern "thiscall" fn d_009eb040(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputAction_4__destroyInstance` (`array_free`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputAction_4__destroyInstance @ 0x009e_c070 => extern "thiscall" fn d_009ec070(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputSource_4__destroyInstance` (`array_free`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputSource_4__destroyInstance @ 0x009e_c0b0 => extern "thiscall" fn d_009ec0b0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputAction__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputAction__destroyInstance @ 0x009e_c0f0 => extern "thiscall" fn d_009ec0f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputMap__destroyInstance` (`destroy_with`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputMap__destroyInstance @ 0x009e_c120 => extern "thiscall" fn d_009ec120(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_with(&mut mem(), 0x009e_2420, obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadAxis__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadAxis__destroyInstance @ 0x009e_c170 => extern "thiscall" fn d_009ec170(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadButton__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceJoypadButton__destroyInstance @ 0x009e_c1a0 => extern "thiscall" fn d_009ec1a0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceKey__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceKey__destroyInstance @ 0x009e_c1d0 => extern "thiscall" fn d_009ec1d0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityX__destroyInstance @ 0x009e_c200 => extern "thiscall" fn d_009ec200(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityY__destroyInstance @ 0x009e_c230 => extern "thiscall" fn d_009ec230(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityZ__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionAngularVelocityZ__destroyInstance @ 0x009e_c260 => extern "thiscall" fn d_009ec260(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationX__destroyInstance @ 0x009e_c290 => extern "thiscall" fn d_009ec290(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationY__destroyInstance @ 0x009e_c2c0 => extern "thiscall" fn d_009ec2c0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationZ__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionLinearAccelerationZ__destroyInstance @ 0x009e_c2f0 => extern "thiscall" fn d_009ec2f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatW__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatW__destroyInstance @ 0x009e_c320 => extern "thiscall" fn d_009ec320(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatX__destroyInstance @ 0x009e_c350 => extern "thiscall" fn d_009ec350(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatY__destroyInstance @ 0x009e_c380 => extern "thiscall" fn d_009ec380(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatZ__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMotionQuatZ__destroyInstance @ 0x009e_c3b0 => extern "thiscall" fn d_009ec3b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseButton__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseButton__destroyInstance @ 0x009e_c3e0 => extern "thiscall" fn d_009ec3e0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaX__destroyInstance @ 0x009e_c410 => extern "thiscall" fn d_009ec410(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceMouseDeltaY__destroyInstance @ 0x009e_c440 => extern "thiscall" fn d_009ec440(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragX__destroyInstance @ 0x009e_c470 => extern "thiscall" fn d_009ec470(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchDragY__destroyInstance @ 0x009e_c4a0 => extern "thiscall" fn d_009ec4a0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchPinch__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchPinch__destroyInstance @ 0x009e_c4d0 => extern "thiscall" fn d_009ec4d0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchRotate__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchRotate__destroyInstance @ 0x009e_c500 => extern "thiscall" fn d_009ec500(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragX__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragX__destroyInstance @ 0x009e_c530 => extern "thiscall" fn d_009ec530(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragY__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PInputs_PInputSourceTouchTwoFingerDragY__destroyInstance @ 0x009e_c560 => extern "thiscall" fn d_009ec560(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__constructInstance` (`array_init`).
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__constructInstance @ 0x00a0_f020 => extern "thiscall" fn d_00a0f020(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_init(&mut mem(), arr)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__destroyInstance` (`array_free`).
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PPostProcessing_PPostEffectBase_4__destroyInstance @ 0x00a1_06e0 => extern "thiscall" fn d_00a106e0(_desc: u32, arr: u32) -> u32 {
        descriptor::bridge::array_free(&mut mem(), arr, 0, false)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLighting__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLighting__destroyInstance @ 0x00a1_0720 => extern "thiscall" fn d_00a10720(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingBase__destroyInstance @ 0x00a1_0750 => extern "thiscall" fn d_00a10750(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDeferredLightingD3D11__destroyInstance @ 0x00a1_0780 => extern "thiscall" fn d_00a10780(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfField__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfField__destroyInstance @ 0x00a1_07b0 => extern "thiscall" fn d_00a107b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldBase__destroyInstance @ 0x00a1_07e0 => extern "thiscall" fn d_00a107e0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PDepthOfFieldD3D11__destroyInstance @ 0x00a1_0810 => extern "thiscall" fn d_00a10810(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAA__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAA__destroyInstance @ 0x00a1_0840 => extern "thiscall" fn d_00a10840(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAABase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAABase__destroyInstance @ 0x00a1_0870 => extern "thiscall" fn d_00a10870(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAAD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PFXAAD3D11__destroyInstance @ 0x00a1_08a0 => extern "thiscall" fn d_00a108a0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlow__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlow__destroyInstance @ 0x00a1_08d0 => extern "thiscall" fn d_00a108d0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowBase__destroyInstance @ 0x00a1_0900 => extern "thiscall" fn d_00a10900(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowD3D11__destroyInstance @ 0x00a1_0930 => extern "thiscall" fn d_00a10930(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowGPUBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PGlowGPUBase__destroyInstance @ 0x00a1_0960 => extern "thiscall" fn d_00a10960(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAA__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAA__destroyInstance @ 0x00a1_0b10 => extern "thiscall" fn d_00a10b10(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAABase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAABase__destroyInstance @ 0x00a1_0b40 => extern "thiscall" fn d_00a10b40(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAAD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMLAAD3D11__destroyInstance @ 0x00a1_0b70 => extern "thiscall" fn d_00a10b70(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlur__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlur__destroyInstance @ 0x00a1_0bd0 => extern "thiscall" fn d_00a10bd0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurBase__destroyInstance @ 0x00a1_0c00 => extern "thiscall" fn d_00a10c00(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PMotionBlurD3D11__destroyInstance @ 0x00a1_0c30 => extern "thiscall" fn d_00a10c30(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectBase__destroyInstance @ 0x00a1_0c60 => extern "thiscall" fn d_00a10c60(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectManager__destroyInstance` (`destroy_with`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PPostEffectManager__destroyInstance @ 0x00a1_0c90 => extern "thiscall" fn d_00a10c90(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_with(&mut mem(), 0x009f_e790, obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion__destroyInstance @ 0x00a1_0cb0 => extern "thiscall" fn d_00a10cb0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase__destroyInstance @ 0x00a1_0ce0 => extern "thiscall" fn d_00a10ce0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11__destroyInstance @ 0x00a1_0d10 => extern "thiscall" fn d_00a10d10(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflection__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflection__destroyInstance @ 0x00a1_0d40 => extern "thiscall" fn d_00a10d40(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionBase__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionBase__destroyInstance @ 0x00a1_0d70 => extern "thiscall" fn d_00a10d70(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11__destroyInstance` (`vdestroy`).
    Phyre_PClassDescriptorForType_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11__destroyInstance @ 0x00a1_0da0 => extern "thiscall" fn d_00a10da0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::destroy_virtual(&mut mem(), obj)
    }
    /// `Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_deleting_dtor` (`dtor_free`).
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_deleting_dtor @ 0x005a_ade0 => extern "thiscall" fn d_005aade0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x005a_a4b0))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__constructInstance` (`zero4`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment__constructInstance @ 0x005a_db00 => extern "thiscall" fn d_005adb00(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_zeroed(&mut mem(), obj, 4)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_char_const_4__vfunc_18` (`member_get`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_char_const_4__vfunc_18 @ 0x005a_e3a0 => extern "thiscall" fn d_005ae3a0(member: u32, obj: u32) -> u32 {
        descriptor::bridge::member_get(&mut mem(), member, obj)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_char_const_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_unsigned_char_const_4__vfunc_20 @ 0x005a_f9c0 => extern "thiscall" fn d_005af9c0(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkDynamicMeshSegment_4__vfunc_20 @ 0x005a_f9f0 => extern "thiscall" fn d_005af9f0(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStream_4__vfunc_20 @ 0x005a_fa20 => extern "thiscall" fn d_005afa20(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstance__constructInstance` (`zero5`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstance__constructInstance @ 0x005b_29b0 => extern "thiscall" fn d_005b29b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_zeroed(&mut mem(), obj, 5)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput__constructInstance` (`minus2zero`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkInstanceInput__constructInstance @ 0x005b_29f0 => extern "thiscall" fn d_005b29f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_unset_pair_zeroed(&mut mem(), obj)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PBase_const_4__vfunc_18` (`member_get`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PBase_const_4__vfunc_18 @ 0x005b_2b40 => extern "thiscall" fn d_005b2b40(member: u32, obj: u32) -> u32 {
        descriptor::bridge::member_get(&mut mem(), member, obj)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PBase_const_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PBase_const_4__vfunc_20 @ 0x005b_3410 => extern "thiscall" fn d_005b3410(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__constructInstance` (`zero5`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__constructInstance @ 0x005c_5b50 => extern "thiscall" fn d_005c5b50(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_zeroed(&mut mem(), obj, 5)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_110` (`ctor`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierAndInputs__vfunc_110 @ 0x005c_5b90 => extern "thiscall" fn d_005c5b90(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x005b_f340, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkBuffer__constructInstance` (`zero5`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkBuffer__constructInstance @ 0x005c_5bb0 => extern "thiscall" fn d_005c5bb0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_zeroed(&mut mem(), obj, 5)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PRenderStreamInput__constructInstance` (`minus2`).
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PRenderStreamInput__constructInstance @ 0x005c_5c80 => extern "thiscall" fn d_005c5c80(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_unset_pair(&mut mem(), obj)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierAndInputs_4__vfunc_20 @ 0x005c_7ab0 => extern "thiscall" fn d_005c7ab0(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PModifierNetworkBuffer_4__vfunc_20 @ 0x005c_7ae0 => extern "thiscall" fn d_005c7ae0(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__vfunc_20` (`member_call`).
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PDynamicGeometry_PRenderStreamInput_4__vfunc_20 @ 0x005c_7b10 => extern "thiscall" fn d_005c7b10(member: u32, obj: u32, a: u32, b: u32) -> u32 {
        descriptor::bridge::member_call(&mut mem(), member, obj, a, b)
    }
    /// `Phyre_PFramework_PApplicationViewport_deleting_dtor` (`vt_free`).
    Phyre_PFramework_PApplicationViewport_deleting_dtor @ 0x0062_1ba0 => extern "thiscall" fn d_00621ba0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b3_ff50), None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PFramework_PInputMapper__constructInstance` (`ctor`).
    Phyre_PClassDescriptorConcrete_Phyre_PFramework_PInputMapper__constructInstance @ 0x0062_6c20 => extern "thiscall" fn d_00626c20(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x0062_0590, None, None)
    }
    /// `ClassDynamicMesh_deleting_dtor` (`vt_dtor_free`).
    ClassDynamicMesh_deleting_dtor @ 0x0067_2760 => extern "thiscall" fn d_00672760(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b4_3ed8), Some(0x0049_69f0))
    }
    /// `Phyre_PClassDescriptorConcrete_ClassDynamicMesh__constructInstance` (`ctor_seh_vt`).
    Phyre_PClassDescriptorConcrete_ClassDynamicMesh__constructInstance @ 0x0067_2ad0 => extern "thiscall" fn d_00672ad0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x0049_68c0, Some(0x00b4_3ed8), None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputAction__constructInstance` (`vt_zero3`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputAction__constructInstance @ 0x009e_b010 => extern "thiscall" fn d_009eb010(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_vtable_zeroed(&mut mem(), obj, 0x00b7_2080, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityX__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityX__constructInstance @ 0x009e_b1f0 => extern "thiscall" fn d_009eb1f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_14e8, 0x00c6_82e8, 0x00b7_1a38])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityY__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityY__constructInstance @ 0x009e_b230 => extern "thiscall" fn d_009eb230(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1580, 0x00c6_82f4, 0x00b7_1b40])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityZ__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionAngularVelocityZ__constructInstance @ 0x009e_b270 => extern "thiscall" fn d_009eb270(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1618, 0x00c6_8300, 0x00b7_1c48])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationX__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationX__constructInstance @ 0x009e_b2b0 => extern "thiscall" fn d_009eb2b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_16b0, 0x00c6_830c, 0x00b7_1d50])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationY__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationY__constructInstance @ 0x009e_b2f0 => extern "thiscall" fn d_009eb2f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1748, 0x00c6_8318, 0x00b7_1e60])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationZ__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionLinearAccelerationZ__constructInstance @ 0x009e_b330 => extern "thiscall" fn d_009eb330(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_17e0, 0x00c6_8324, 0x00b7_1f70])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatW__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatW__constructInstance @ 0x009e_b370 => extern "thiscall" fn d_009eb370(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1450, 0x00c6_82dc, 0x00b7_1938])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatX__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatX__constructInstance @ 0x009e_b3b0 => extern "thiscall" fn d_009eb3b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1288, 0x00c6_82b8, 0x00b7_1638])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatY__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatY__constructInstance @ 0x009e_b3f0 => extern "thiscall" fn d_009eb3f0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1320, 0x00c6_82c4, 0x00b7_1738])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatZ__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceMotionQuatZ__constructInstance @ 0x009e_b430 => extern "thiscall" fn d_009eb430(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_13b8, 0x00c6_82d0, 0x00b7_1838])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchDragX__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchDragX__constructInstance @ 0x009e_b620 => extern "thiscall" fn d_009eb620(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1028, 0x00c6_8270, 0x00b7_1438])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchDragY__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchDragY__constructInstance @ 0x009e_b660 => extern "thiscall" fn d_009eb660(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_10c0, 0x00c6_827c, 0x00b7_1538])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchPinch__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchPinch__constructInstance @ 0x009e_b6a0 => extern "thiscall" fn d_009eb6a0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_0f90, 0x00c6_82a0, 0x00b7_1128])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchRotate__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchRotate__constructInstance @ 0x009e_b6e0 => extern "thiscall" fn d_009eb6e0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_0ef8, 0x00c6_82ac, 0x00b7_1028])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchTwoFingerDragX__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchTwoFingerDragX__constructInstance @ 0x009e_b720 => extern "thiscall" fn d_009eb720(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_1158, 0x00c6_8288, 0x00b7_1228])
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchTwoFingerDragY__constructInstance` (`input52`).
    Phyre_PClassDescriptorConcrete_Phyre_PInputs_PInputSourceTouchTwoFingerDragY__constructInstance @ 0x009e_b760 => extern "thiscall" fn d_009eb760(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_input(&mut mem(), obj, [0x0194_11f0, 0x00c6_8294, 0x00b7_1330])
    }
    /// `Phyre_PPostProcessing_PDeferredLighting_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PDeferredLighting_deleting_dtor @ 0x00a0_26a0 => extern "thiscall" fn d_00a026a0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_dc00))
    }
    /// `Phyre_PPostProcessing_PDeferredLightingD3D11_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PDeferredLightingD3D11_deleting_dtor @ 0x00a0_2770 => extern "thiscall" fn d_00a02770(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_dc00))
    }
    /// `Phyre_PPostProcessing_PDepthOfFieldBase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PDepthOfFieldBase_deleting_dtor @ 0x00a0_2820 => extern "thiscall" fn d_00a02820(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_dd30))
    }
    /// `Phyre_PPostProcessing_PFXAA_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PFXAA_deleting_dtor @ 0x00a0_29f0 => extern "thiscall" fn d_00a029f0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_df60))
    }
    /// `Phyre_PPostProcessing_PFXAABase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PFXAABase_deleting_dtor @ 0x00a0_2a20 => extern "thiscall" fn d_00a02a20(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_df60))
    }
    /// `Phyre_PPostProcessing_PFXAAD3D11_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PFXAAD3D11_deleting_dtor @ 0x00a0_2a50 => extern "thiscall" fn d_00a02a50(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_df60))
    }
    /// `Phyre_PPostProcessing_PGlow_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PGlow_deleting_dtor @ 0x00a0_2a80 => extern "thiscall" fn d_00a02a80(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e0d0))
    }
    /// `Phyre_PPostProcessing_PGlowBase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PGlowBase_deleting_dtor @ 0x00a0_2ab0 => extern "thiscall" fn d_00a02ab0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e020))
    }
    /// `Phyre_PPostProcessing_PGlowD3D11_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PGlowD3D11_deleting_dtor @ 0x00a0_2ae0 => extern "thiscall" fn d_00a02ae0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e0d0))
    }
    /// `Phyre_PPostProcessing_PGlowGPUBase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PGlowGPUBase_deleting_dtor @ 0x00a0_2b10 => extern "thiscall" fn d_00a02b10(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e0d0))
    }
    /// `Phyre_PPostProcessing_PMLAA_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PMLAA_deleting_dtor @ 0x00a0_2d00 => extern "thiscall" fn d_00a02d00(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e2a0))
    }
    /// `Phyre_PPostProcessing_PMLAABase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PMLAABase_deleting_dtor @ 0x00a0_2d30 => extern "thiscall" fn d_00a02d30(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e2a0))
    }
    /// `Phyre_PPostProcessing_PMLAAD3D11_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PMLAAD3D11_deleting_dtor @ 0x00a0_2d60 => extern "thiscall" fn d_00a02d60(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e2a0))
    }
    /// `Phyre_PPostProcessing_PMotionBlurBase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PMotionBlurBase_deleting_dtor @ 0x00a0_2f10 => extern "thiscall" fn d_00a02f10(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e640))
    }
    /// `Phyre_PPostProcessing_PPostEffectBase_deleting_dtor` (`vt_free`).
    Phyre_PPostProcessing_PPostEffectBase_deleting_dtor @ 0x00a0_2fc0 => extern "thiscall" fn d_00a02fc0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b7_4058), None)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion_deleting_dtor` (`vt_free`).
    Phyre_PPostProcessing_PScreenSpaceAmbientOcclusion_deleting_dtor @ 0x00a0_3020 => extern "thiscall" fn d_00a03020(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b7_4058), None)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase_deleting_dtor` (`vt_free`).
    Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionBase_deleting_dtor @ 0x00a0_3050 => extern "thiscall" fn d_00a03050(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b7_4058), None)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11_deleting_dtor` (`vt_free`).
    Phyre_PPostProcessing_PScreenSpaceAmbientOcclusionD3D11_deleting_dtor @ 0x00a0_3080 => extern "thiscall" fn d_00a03080(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, Some(0x00b7_4058), None)
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflection_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PScreenSpaceReflection_deleting_dtor @ 0x00a0_30b0 => extern "thiscall" fn d_00a030b0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e890))
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflectionBase_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PScreenSpaceReflectionBase_deleting_dtor @ 0x00a0_30e0 => extern "thiscall" fn d_00a030e0(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e890))
    }
    /// `Phyre_PPostProcessing_PScreenSpaceReflectionD3D11_deleting_dtor` (`dtor_free`).
    Phyre_PPostProcessing_PScreenSpaceReflectionD3D11_deleting_dtor @ 0x00a0_3110 => extern "thiscall" fn d_00a03110(this: u32, flags: u32) -> u32 {
        descriptor::bridge::deleting_dtor(&mut mem(), this, flags, None, Some(0x009f_e890))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLighting__constructInstance` (`ctor_seh_vt2`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLighting__constructInstance @ 0x00a0_f040 => extern "thiscall" fn d_00a0f040(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_bb30, Some(0x00b7_4380), Some(0x0194_3718))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLightingBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLightingBase__constructInstance @ 0x00a0_f0a0 => extern "thiscall" fn d_00a0f0a0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_b830, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLightingD3D11__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDeferredLightingD3D11__constructInstance @ 0x00a0_f0f0 => extern "thiscall" fn d_00a0f0f0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_bb30, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDepthOfFieldBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PDepthOfFieldBase__constructInstance @ 0x00a0_f1d0 => extern "thiscall" fn d_00a0f1d0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_bd90, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAA__constructInstance` (`ctor_seh_vt2`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAA__constructInstance @ 0x00a0_f2a0 => extern "thiscall" fn d_00a0f2a0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c160, Some(0x00b7_5c88), Some(0x0194_45a8))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAABase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAABase__constructInstance @ 0x00a0_f300 => extern "thiscall" fn d_00a0f300(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c160, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAAD3D11__constructInstance` (`ctor_seh_vt`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PFXAAD3D11__constructInstance @ 0x00a0_f350 => extern "thiscall" fn d_00a0f350(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c160, Some(0x00b7_5b88), None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlow__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlow__constructInstance @ 0x00a0_f3b0 => extern "thiscall" fn d_00a0f3b0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c2b0, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowBase__constructInstance @ 0x00a0_f400 => extern "thiscall" fn d_00a0f400(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c310, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowD3D11__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowD3D11__constructInstance @ 0x00a0_f450 => extern "thiscall" fn d_00a0f450(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c480, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowGPUBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PGlowGPUBase__constructInstance @ 0x00a0_f4a0 => extern "thiscall" fn d_00a0f4a0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c4e0, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAA__constructInstance` (`ctor_seh_vt2`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAA__constructInstance @ 0x00a0_f6f0 => extern "thiscall" fn d_00a0f6f0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c7e0, Some(0x00b7_50b8), Some(0x0194_3e88))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAABase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAABase__constructInstance @ 0x00a0_f750 => extern "thiscall" fn d_00a0f750(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c7e0, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAAD3D11__constructInstance` (`ctor_seh_vt`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMLAAD3D11__constructInstance @ 0x00a0_f7a0 => extern "thiscall" fn d_00a0f7a0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_c7e0, Some(0x00b7_4fb8), None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMotionBlurBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PMotionBlurBase__constructInstance @ 0x00a0_f8c0 => extern "thiscall" fn d_00a0f8c0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_cbb0, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PPostEffectBase__constructInstance` (`vt_zero3_w`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PPostEffectBase__constructInstance @ 0x00a0_f970 => extern "thiscall" fn d_00a0f970(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_vtable_zeroed(&mut mem(), obj, 0x00b7_4058, Some(0x100))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PPostEffectManager__constructInstance` (`zero4`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PPostEffectManager__constructInstance @ 0x00a0_f9b0 => extern "thiscall" fn d_00a0f9b0(_desc: u32, obj: u32) -> u32 {
        descriptor::bridge::construct_zeroed(&mut mem(), obj, 4)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflection__constructInstance` (`ctor_seh_vt2`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflection__constructInstance @ 0x00a0_fb00 => extern "thiscall" fn d_00a0fb00(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_d080, Some(0x00b7_5fa8), Some(0x0194_4770))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflectionBase__constructInstance` (`ctor_seh`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflectionBase__constructInstance @ 0x00a0_fb60 => extern "thiscall" fn d_00a0fb60(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_d080, None, None)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11__constructInstance` (`ctor_seh_vt`).
    Phyre_PClassDescriptorConcrete_Phyre_PPostProcessing_PScreenSpaceReflectionD3D11__constructInstance @ 0x00a0_fbb0 => extern "thiscall" fn d_00a0fbb0(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with(&mut mem(), obj, 0x009f_d080, Some(0x00b7_5e98), None)
    }
    /// `Phyre_PRendering_PRenderInterfaceNVN_unbindFromCurrentThread`.
    Phyre_PRendering_PRenderInterfaceNVN_unbindFromCurrentThread @ 0x0059_d2e0 => extern "C" fn p_0059d2e0() -> u32 {
        0x17
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineCpu_initialize_0060ed70`.
    Phyre_PProfiling_PProcessorProfileTimelineCpu_initialize_0060ed70 @ 0x0060_ed70 => extern "stdcall" fn p_0060ed70(_a: u32, _b: u32) -> u32 {
        0
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineCollection_reset`.
    Phyre_PProfiling_PProcessorProfileTimelineCollection_reset @ 0x0060_ee00 => extern "stdcall" fn p_0060ee00(_a: u32, _b: u32) -> u32 {
        0
    }
    /// `Phyre_PFramework_PApplication_GetApplication`.
    Phyre_PFramework_PApplication_GetApplication @ 0x0062_3b90 => extern "C" fn p_00623b90() -> u32 {
        misc::bridge::get_application(&mut mem())
    }
    /// `Phyre_PArray_PBase_const_getAt_005b2c70`.
    Phyre_PArray_PBase_const_getAt_005b2c70 @ 0x005b_2c70 => extern "thiscall" fn p_005b2c70(array: u32, i: u32) -> u32 {
        misc::bridge::array_at(&mut mem(), array, i)
    }
    /// `Phyre_PRendering_PRenderSurface_PRenderSurface`.
    Phyre_PRendering_PRenderSurface_PRenderSurface @ 0x005b_35c0 => extern "thiscall" fn p_005b35c0(surface: u32, a: u32, b: u32) -> u32 {
        misc::bridge::render_surface_ctor(&mut mem(), surface, a, b)
    }
    /// `PZeroQuad_init_005b3620`.
    PZeroQuad_init_005b3620 @ 0x005b_3620 => extern "thiscall" fn p_005b3620(obj: u32) -> u32 {
        misc::bridge::zero4(&mut mem(), obj)
    }
    /// `Phyre_PRendering_setRange_005b3660`.
    Phyre_PRendering_setRange_005b3660 @ 0x005b_3660 => extern "thiscall" fn p_005b3660(obj: u32, a: u32, b: u32, c: u32) -> u32 {
        misc::bridge::set_three(&mut mem(), obj, a, b, c)
    }
    /// `Phyre_PRendering_PRendererBase_renderMeshInstanceForPass_005b9a70`.
    Phyre_PRendering_PRendererBase_renderMeshInstanceForPass_005b9a70 @ 0x005b_9a70 => extern "thiscall" fn p_005b9a70(renderer: u32, instance: u32) -> u32 {
        misc::bridge::render_instance(&mut mem(), renderer, instance)
    }
    /// `Phyre_PTypeDispenser_lookupInto_005c86e0`.
    Phyre_PTypeDispenser_lookupInto_005c86e0 @ 0x005c_86e0 => extern "C" fn p_005c86e0(out: u32, x: u32) -> u32 {
        misc::bridge::lookup_into(&mut mem(), 0x005c_84f0, out, x)
    }
    /// `Phyre_PDynamicGeometry_tableCall1_005aead0`.
    Phyre_PDynamicGeometry_tableCall1_005aead0 @ 0x005a_ead0 => extern "thiscall" fn p_005aead0(obj: u32, i: u32, x: u32) -> u32 {
        misc::bridge::table_call(&mut mem(), 0x005c_6680, obj, i, &[x])
    }
    /// `Phyre_PDynamicGeometry_tableCall2_005aece0`.
    Phyre_PDynamicGeometry_tableCall2_005aece0 @ 0x005a_ece0 => extern "thiscall" fn p_005aece0(obj: u32, i: u32, x: u32, y: u32) -> u32 {
        misc::bridge::table_call(&mut mem(), 0x005c_6c30, obj, i, &[x, y])
    }
    /// `Phyre_PTextureFormat_isOneOf_005a04d0`.
    Phyre_PTextureFormat_isOneOf_005a04d0 @ 0x005a_04d0 => extern "C" fn p_005a04d0(format: u32) -> u32 {
        misc::bridge::format_in_set(format)
    }
    /// `Phyre_PRendering_PRenderer_syncRender`.
    Phyre_PRendering_PRenderer_syncRender @ 0x005b_c3f0 => extern "C" fn p_005bc3f0() -> u32 {
        misc::bridge::sync_render(&mut mem())
    }
    /// `Phyre_PRendering_PRenderRingBufferSegmented_allocate`.
    Phyre_PRendering_PRenderRingBufferSegmented_allocate @ 0x0061_3e10 => extern "thiscall" fn p_00613e10(ring: u32, size: u32) -> u32 {
        misc::bridge::segmented_allocate(&mut mem(), ring, size)
    }
    /// `Phyre_PRendering_PRenderRingBuffer_init_00613e80`.
    Phyre_PRendering_PRenderRingBuffer_init_00613e80 @ 0x0061_3e80 => extern "thiscall" fn p_00613e80(ring: u32, base: u32, size: u32, extra: u32) -> u32 {
        misc::bridge::ring_init(&mut mem(), ring, base, size, extra)
    }
    /// `Phyre_PRendering_PRenderRingBuffer_stepForAllocation`.
    Phyre_PRendering_PRenderRingBuffer_stepForAllocation @ 0x0061_3f00 => extern "thiscall" fn p_00613f00(ring: u32, cursor: u32, size: u32, out: u32) -> u32 {
        misc::bridge::step_for_allocation(&mut mem(), ring, cursor, size, out)
    }
    /// `Phyre_PRendering_PKick_PKick`.
    Phyre_PRendering_PKick_PKick @ 0x0061_3fc0 => extern "thiscall" fn p_00613fc0(kick: u32) -> u32 {
        misc::bridge::kick_ctor(&mut mem(), kick)
    }
    /// `Phyre_PRendering_PRenderThread_SignalSynchronizationEventFlagRenderCallback`.
    Phyre_PRendering_PRenderThread_SignalSynchronizationEventFlagRenderCallback @ 0x0061_4700 => extern "C" fn p_00614700(cmd: u32) -> u32 {
        misc::bridge::signal_sync_flag(&mut mem(), cmd)
    }
    /// `Phyre_PRendering_PRenderThread_StaticThreadEntry`.
    Phyre_PRendering_PRenderThread_StaticThreadEntry @ 0x0061_4710 => extern "C" fn p_00614710(thread: u32) -> u32 {
        misc::bridge::static_thread_entry(&mut mem(), thread)
    }
    /// `Phyre_PRendering_PRenderThread_getFlipSlot`.
    Phyre_PRendering_PRenderThread_getFlipSlot @ 0x0061_4c00 => extern "thiscall" fn p_00614c00(thread: u32) -> u32 {
        misc::bridge::get_flip_slot(&mut mem(), thread)
    }
    /// `Phyre_PRendering_PooledHandle_release_0061c890`.
    Phyre_PRendering_PooledHandle_release_0061c890 @ 0x0061_c890 => extern "thiscall" fn p_0061c890(handle: u32) {
        misc::bridge::release_handle(&mut mem(), 0x004b_a540, handle);
    }
    /// `Phyre_PFramework_PWindowWin32Base_SetFlipVSyncInterval`.
    Phyre_PFramework_PWindowWin32Base_SetFlipVSyncInterval @ 0x0062_5090 => extern "C" fn p_00625090(n: u32) -> u32 {
        misc::bridge::set_flip_interval(&mut mem(), n)
    }
    /// `std_Tree_buyHeadNode_00625850`.
    std_Tree_buyHeadNode_00625850 @ 0x0062_5850 => extern "C" fn p_00625850() -> u32 {
        misc::bridge::tree_head(&mut mem())
    }
    /// `Phyre_PFramework_PApplication_initRange_00629840`.
    Phyre_PFramework_PApplication_initRange_00629840 @ 0x0062_9840 => extern "thiscall" fn p_00629840(lock: u32, base: u32, size: u32) -> u32 {
        misc::bridge::range_init(&mut mem(), lock, base, size)
    }
    /// `Phyre_PFramework_PApplication_prePhyreInit`.
    Phyre_PFramework_PApplication_prePhyreInit @ 0x0062_ac70 => extern "C" fn p_0062ac70() -> u32 {
        misc::bridge::pre_phyre_init(&mut mem())
    }
    /// `Phyre_PFramework_PApplication_setWindowIconID`.
    Phyre_PFramework_PApplication_setWindowIconID @ 0x0062_cf50 => extern "thiscall" fn p_0062cf50(app: u32, id: u32) -> u32 {
        misc::bridge::set_window_icon_id(&mut mem(), app, id)
    }
    /// `ClassDynamicMesh_ClassDynamicMesh_00672630`.
    ClassDynamicMesh_ClassDynamicMesh_00672630 @ 0x0067_2630 => extern "thiscall" fn p_00672630(obj: u32) -> u32 {
        misc::bridge::derived_ctor(&mut mem(), obj, 0x0049_68c0, 0x00b4_3ed8, None)
    }
    /// `ClassDynamicMesh_registerType_00672790`.
    ClassDynamicMesh_registerType_00672790 @ 0x0067_2790 => extern "C" fn p_00672790() -> u32 {
        misc::bridge::register_dynamic_mesh(&mut mem())
    }
    /// `PWorldCursor_runLength_009f3810`.
    PWorldCursor_runLength_009f3810 @ 0x009f_3810 => extern "thiscall" fn p_009f3810(cursor: u32) -> u32 {
        misc::bridge::cursor_run_length(&mut mem(), cursor)
    }
    /// `Phyre_PPostProcessing_PFXAA_PFXAA_009fc140`.
    Phyre_PPostProcessing_PFXAA_PFXAA_009fc140 @ 0x009f_c140 => extern "thiscall" fn p_009fc140(obj: u32) -> u32 {
        misc::bridge::derived_ctor(&mut mem(), obj, 0x009f_c160, 0x00b7_5c88, Some(0x0194_45a8))
    }
    /// `Phyre_PPostProcessing_PMLAA_PMLAA_009fc7c0`.
    Phyre_PPostProcessing_PMLAA_PMLAA_009fc7c0 @ 0x009f_c7c0 => extern "thiscall" fn p_009fc7c0(obj: u32) -> u32 {
        misc::bridge::derived_ctor(&mut mem(), obj, 0x009f_c7e0, 0x00b7_50b8, Some(0x0194_3e88))
    }
    /// `Phyre_PIggy_PUtilityIggy_allocCallback_00a3e5a0`.
    Phyre_PIggy_PUtilityIggy_allocCallback_00a3e5a0 @ 0x00a3_e5a0 => extern "stdcall" fn p_00a3e5a0(_user: u32, size: u32, _c: u32) -> u32 {
        misc::bridge::iggy_alloc(&mut mem(), size)
    }
    /// `Phyre_PIggy_PUtilityIggy_freeCallback_00a3e5c0`.
    Phyre_PIggy_PUtilityIggy_freeCallback_00a3e5c0 @ 0x00a3_e5c0 => extern "stdcall" fn p_00a3e5c0(_user: u32, ptr: u32) -> u32 {
        misc::bridge::iggy_free(&mut mem(), ptr)
    }
    /// `Phyre_PClassDescriptorHeader_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__vfunc_3c`.
    Phyre_PClassDescriptorHeader_Phyre_PDynamicGeometry_PModifierNetworkInfoPacket__vfunc_3c @ 0x005c_7f50 => extern "thiscall" fn p_005c7f50(header: u32, a: u32, b: u32) -> u32 {
        misc::bridge::header_call(&mut mem(), header, a, b)
    }
    /// `Phyre_PText_PUtilityText_initialize`.
    Phyre_PText_PUtilityText_initialize @ 0x00a3_ea40 => extern "C" fn p_00a3ea40() -> u32 {
        misc::bridge::text_initialize(&mut mem())
    }
}
