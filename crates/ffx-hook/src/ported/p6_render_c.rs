//! M4 port round 1, list p6-render-c (`PhyreEngine` render, highest third
//! by address): the shims. Ports in `ffx_prender` (`descriptor`, ...).
//!
//! Class-descriptor callbacks (`ffx_prender::descriptor`): `thiscall(desc,
//! instance)`, `ret 4`, called through the descriptors' vtables by the
//! serializer and the loader (main thread and the async init job).

use super::{mem, ported};
use ffx_prender::descriptor;

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
}
