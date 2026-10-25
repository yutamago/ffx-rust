//! M4 port round 1, list h1-render-a: `PhyreEngine` render code
//! (`ffx_prender`), lowest third by address.
//!
//! Threads: the class-descriptor callbacks run where objects are made,
//! loaded and freed (the main thread and the loader); `Bind` at start-up.
//! No shim keeps state.

use super::{mem, ported};
use ffx_prender::descriptor;

ported! {
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PWorld__destroyInstance @ 0x0044_3430 => extern "thiscall" fn h1_00443430(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x0044_3430) }, p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PCamera__constructInstance @ 0x0045_4960 => extern "thiscall" fn h1_00454960(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0045_4960) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PCameraOrthographic__fixObjectsAfterLoading @ 0x0045_5ba0 => extern "thiscall" fn h1_00455ba0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0045_5ba0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PCameraPerspective__constructInstance @ 0x0045_6930 => extern "thiscall" fn h1_00456930(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0045_6930) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PCameraPerspective__fixObjectsAfterLoading @ 0x0045_6980 => extern "thiscall" fn h1_00456980(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0045_6980) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PAssetReference__destroyInstance @ 0x0045_dcf0 => extern "thiscall" fn h1_0045dcf0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x0045_dcf0) }, p))
    }
    /// `constructInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorConcrete_Phyre_PSerialization_PBinary_PInternal_PInstanceListHeader__constructInstance @ 0x0046_a920 => extern "thiscall" fn h1_0046a920(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0046_a920) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PSerialization_PBinary_PInternal_PInstanceListHeader__destroyInstance @ 0x0046_a930 => extern "thiscall" fn h1_0046a930(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0046_a930) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PVertexStream__destroyInstance @ 0x0048_0a30 => extern "thiscall" fn h1_00480a30(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0048_0a30) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PVertexStream__fixObjectsAfterLoading @ 0x0048_0a40 => extern "thiscall" fn h1_00480a40(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0048_0a40) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PGeometry_PDataBlock_Bind()`: registers its class descriptor.
    Phyre_PGeometry_PDataBlock_Bind @ 0x0048_1f60 => extern "C" fn h1_00481f60() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x0048_1f60) })
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PVertexStream_4__constructInstance @ 0x0048_3100 => extern "thiscall" fn h1_00483100(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_3100) }))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PDataBlock__fixObjectsAfterLoading @ 0x0048_32b0 => extern "thiscall" fn h1_004832b0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0048_32b0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDynamicDataBlock__destroyInstance @ 0x0048_4870 => extern "thiscall" fn h1_00484870(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x0048_4870) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PDynamicDataBlock__fixObjectsAfterLoading @ 0x0048_4890 => extern "thiscall" fn h1_00484890(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0048_4890) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PGeometry_PIndexDataBlock_Bind()`: registers its class descriptor.
    Phyre_PGeometry_PIndexDataBlock_Bind @ 0x0048_5450 => extern "C" fn h1_00485450() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x0048_5450) })
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlockBase__destroyInstance @ 0x0048_5d80 => extern "thiscall" fn h1_00485d80(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0048_5d80) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `Phyre_PGeometry_PMeshSegment_Bind()`: registers its class descriptor.
    Phyre_PGeometry_PMeshSegment_Bind @ 0x0048_7970 => extern "C" fn h1_00487970() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x0048_7970) })
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__constructInstance @ 0x0048_8ed0 => extern "thiscall" fn h1_00488ed0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_8ed0) }))
    }
    /// `constructInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PSkinBoneRemap__constructInstance @ 0x0048_8fa0 => extern "thiscall" fn h1_00488fa0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0048_8fa0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkinBoneRemap__destroyInstance @ 0x0048_90c0 => extern "thiscall" fn h1_004890c0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0048_90c0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PMeshSegment__fixObjectsAfterLoading @ 0x0048_91e0 => extern "thiscall" fn h1_004891e0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0048_91e0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PSkinBoneRemap__fixObjectsAfterLoading @ 0x0048_9200 => extern "thiscall" fn h1_00489200(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0048_9200) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_Phyre_PRendering_PMaterial__constructInstance @ 0x0048_bd90 => extern "thiscall" fn h1_0048bd90(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_bd90) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMaterialSet__constructInstance @ 0x0048_bdb0 => extern "thiscall" fn h1_0048bdb0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_bdb0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_int_4__constructInstance @ 0x0049_3ce0 => extern "thiscall" fn h1_00493ce0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3ce0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PMatrix4_4__constructInstance @ 0x0049_3d00 => extern "thiscall" fn h1_00493d00(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3d00) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PMeshSegment_4__constructInstance @ 0x0049_3d20 => extern "thiscall" fn h1_00493d20(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3d20) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PSkeletonJointBounds_4__constructInstance @ 0x0049_3d40 => extern "thiscall" fn h1_00493d40(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3d40) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PString_4__constructInstance @ 0x0049_3d60 => extern "thiscall" fn h1_00493d60(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3d60) }))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMesh__constructInstance @ 0x0049_3d80 => extern "thiscall" fn h1_00493d80(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0049_3d80) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMesh__destroyInstance @ 0x0049_3f20 => extern "thiscall" fn h1_00493f20(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x0049_3f20) }, p))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PSkeletonJointBounds__destroyInstance @ 0x0049_3f40 => extern "thiscall" fn h1_00493f40(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0049_3f40) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PMesh__fixObjectsAfterLoading @ 0x0049_4020 => extern "thiscall" fn h1_00494020(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0049_4020) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PSkeletonJointBounds__fixObjectsAfterLoading @ 0x0049_4030 => extern "thiscall" fn h1_00494030(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0049_4030) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_4__constructInstance @ 0x0049_8140 => extern "thiscall" fn h1_00498140(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_8140) }))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc__destroyInstance @ 0x0049_84f0 => extern "thiscall" fn h1_004984f0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0049_84f0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc__fixObjectsAfterLoading @ 0x0049_8640 => extern "thiscall" fn h1_00498640(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x0049_8640) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PLight__destroyInstance @ 0x0049_ec20 => extern "thiscall" fn h1_0049ec20(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x0049_ec20) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `Phyre_PRendering_PConstantBuffer_Bind()`: registers its class descriptor.
    Phyre_PRendering_PConstantBuffer_Bind @ 0x0049_fdf0 => extern "C" fn h1_0049fdf0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x0049_fdf0) })
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PConstantBufferBase__destroyInstance @ 0x004a_05d0 => extern "thiscall" fn h1_004a05d0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004a_05d0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferLocation__constructInstance @ 0x004a_2e40 => extern "thiscall" fn h1_004a2e40(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_2e40) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferLocationSize__constructInstance @ 0x004a_2e60 => extern "thiscall" fn h1_004a2e60(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_2e60) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferLocationType__constructInstance @ 0x004a_2e80 => extern "thiscall" fn h1_004a2e80(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_2e80) }))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocation__destroyInstance @ 0x004a_2f20 => extern "thiscall" fn h1_004a2f20(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004a_2f20) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationSize__destroyInstance @ 0x004a_2f30 => extern "thiscall" fn h1_004a2f30(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004a_2f30) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationType__destroyInstance @ 0x004a_2f40 => extern "thiscall" fn h1_004a2f40(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004a_2f40) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer__destroyInstance @ 0x004a_2f50 => extern "thiscall" fn h1_004a2f50(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004a_2f50) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderParameterCaptureBufferLocation__fixObjectsAfterLoading @ 0x004a_2f80 => extern "thiscall" fn h1_004a2f80(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004a_2f80) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer__fixObjectsAfterLoading @ 0x004a_2fb0 => extern "thiscall" fn h1_004a2fb0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004a_2fb0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderParameterDefinition__fixObjectsAfterLoading @ 0x004a_2fc0 => extern "thiscall" fn h1_004a2fc0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004a_2fc0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PShaderComputeProgram_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderComputeProgram_Bind @ 0x004a_9d80 => extern "C" fn h1_004a9d80() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_9d80) })
    }
    /// `Phyre_PRendering_PShaderFragmentProgram_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderFragmentProgram_Bind @ 0x004a_9da0 => extern "C" fn h1_004a9da0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_9da0) })
    }
    /// `Phyre_PRendering_PShaderGeometryProgram_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderGeometryProgram_Bind @ 0x004a_9dc0 => extern "C" fn h1_004a9dc0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_9dc0) })
    }
    /// `Phyre_PRendering_PShaderParameterCaptureBufferStream_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderParameterCaptureBufferStream_Bind @ 0x004a_a3f0 => extern "C" fn h1_004aa3f0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_a3f0) })
    }
    /// `Phyre_PRendering_PShaderProgramBase_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderProgramBase_Bind @ 0x004a_a8a0 => extern "C" fn h1_004aa8a0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_a8a0) })
    }
    /// `Phyre_PRendering_PShaderVertexProgram_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderVertexProgram_Bind @ 0x004a_ae60 => extern "C" fn h1_004aae60() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004a_ae60) })
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderParameterDefinition_4__constructInstance @ 0x004a_ff80 => extern "thiscall" fn h1_004aff80(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_ff80) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderStreamDefinition_4__constructInstance @ 0x004a_ffa0 => extern "thiscall" fn h1_004affa0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_ffa0) }))
    }
    /// `constructInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferStream__constructInstance @ 0x004b_0170 => extern "thiscall" fn h1_004b0170(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0170) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `constructInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderProgramBase__constructInstance @ 0x004b_0280 => extern "thiscall" fn h1_004b0280(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0280) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderComputeProgram__destroyInstance @ 0x004b_05b0 => extern "thiscall" fn h1_004b05b0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_05b0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderFragmentProgram__destroyInstance @ 0x004b_05d0 => extern "thiscall" fn h1_004b05d0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_05d0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderGeometryProgram__destroyInstance @ 0x004b_05f0 => extern "thiscall" fn h1_004b05f0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_05f0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferByteAddressBuffer__destroyInstance @ 0x004b_0610 => extern "thiscall" fn h1_004b0610(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0610) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferDataBlock__destroyInstance @ 0x004b_0620 => extern "thiscall" fn h1_004b0620(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0620) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferIndexDataBlock__destroyInstance @ 0x004b_0630 => extern "thiscall" fn h1_004b0630(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0630) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferRWByteAddressBuffer__destroyInstance @ 0x004b_0640 => extern "thiscall" fn h1_004b0640(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0640) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferRWStructuredBuffer__destroyInstance @ 0x004b_0650 => extern "thiscall" fn h1_004b0650(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0650) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferRWTexture2D__destroyInstance @ 0x004b_0660 => extern "thiscall" fn h1_004b0660(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0660) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferRWTexture3D__destroyInstance @ 0x004b_0670 => extern "thiscall" fn h1_004b0670(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0670) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferSampler__destroyInstance @ 0x004b_0680 => extern "thiscall" fn h1_004b0680(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0680) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferStream__destroyInstance @ 0x004b_0690 => extern "thiscall" fn h1_004b0690(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0690) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferStructuredBuffer__destroyInstance @ 0x004b_06a0 => extern "thiscall" fn h1_004b06a0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06a0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferTexture2D__destroyInstance @ 0x004b_06b0 => extern "thiscall" fn h1_004b06b0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06b0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferTexture3D__destroyInstance @ 0x004b_06c0 => extern "thiscall" fn h1_004b06c0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06c0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferTextureBase__destroyInstance @ 0x004b_06d0 => extern "thiscall" fn h1_004b06d0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06d0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureBufferTextureCubeMap__destroyInstance @ 0x004b_06e0 => extern "thiscall" fn h1_004b06e0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06e0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterCaptureConstantBuffer__destroyInstance @ 0x004b_06f0 => extern "thiscall" fn h1_004b06f0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_06f0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderProgramBase__destroyInstance @ 0x004b_0700 => extern "thiscall" fn h1_004b0700(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_0700) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderProgramParamsAndStreams__destroyInstance @ 0x004b_0710 => extern "thiscall" fn h1_004b0710(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_0710) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderSource__destroyInstance @ 0x004b_0730 => extern "thiscall" fn h1_004b0730(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_0730) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderVertexProgram__destroyInstance @ 0x004b_0770 => extern "thiscall" fn h1_004b0770(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_0770) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderFragmentProgram__fixObjectsAfterLoading @ 0x004b_07c0 => extern "thiscall" fn h1_004b07c0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004b_07c0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderStreamDefinition__fixObjectsAfterLoading @ 0x004b_0900 => extern "thiscall" fn h1_004b0900(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004b_0900) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderVertexProgram__fixObjectsAfterLoading @ 0x004b_0910 => extern "thiscall" fn h1_004b0910(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004b_0910) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PShaderPass_Bind()`: registers its class descriptor.
    Phyre_PRendering_PShaderPass_Bind @ 0x004b_6330 => extern "C" fn h1_004b6330() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004b_6330) })
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocation_2__constructInstance @ 0x004b_9630 => extern "thiscall" fn h1_004b9630(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9630) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationType_2__constructInstance @ 0x004b_9650 => extern "thiscall" fn h1_004b9650(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9650) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_4__constructInstance @ 0x004b_9670 => extern "thiscall" fn h1_004b9670(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9670) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassStateBase__constructInstance @ 0x004b_9800 => extern "thiscall" fn h1_004b9800(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9800) }))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPass__destroyInstance @ 0x004b_98e0 => extern "thiscall" fn h1_004b98e0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_98e0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassParameterLocationTypesBase__destroyInstance @ 0x004b_9980 => extern "thiscall" fn h1_004b9980(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_9980) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassStateBase__destroyInstance @ 0x004b_99d0 => extern "thiscall" fn h1_004b99d0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004b_99d0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShaderPass__fixObjectsAfterLoading @ 0x004b_9a10 => extern "thiscall" fn h1_004b9a10(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004b_9a10) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PRendering_PShaderPass_4__constructInstance @ 0x004b_d580 => extern "thiscall" fn h1_004bd580(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_d580) }))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderPass_4__destroyInstance @ 0x004b_d5f0 => extern "thiscall" fn h1_004bd5f0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_d5f0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShader__destroyInstance @ 0x004b_d610 => extern "thiscall" fn h1_004bd610(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004b_d610) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PShader__fixObjectsAfterLoading @ 0x004b_d7c0 => extern "thiscall" fn h1_004bd7c0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004b_d7c0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PSamplerState_Bind()`: registers its class descriptor.
    Phyre_PRendering_PSamplerState_Bind @ 0x004b_ed40 => extern "C" fn h1_004bed40() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004b_ed40) })
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerState__destroyInstance @ 0x004c_01d0 => extern "thiscall" fn h1_004c01d0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004c_01d0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: nothing to do: true.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSamplerStateBase__destroyInstance @ 0x004c_01f0 => extern "thiscall" fn h1_004c01f0(_descriptor: u32, _p: u32) -> u8 {
        const { descriptor::listed(descriptor::NOTHING, 0x004c_01f0) };
        u8::from(descriptor::bridge::nothing_to_do())
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PSamplerState__fixObjectsAfterLoading @ 0x004c_0200 => extern "thiscall" fn h1_004c0200(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004c_0200) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PTexture2D_Bind()`: registers its class descriptor.
    Phyre_PRendering_PTexture2D_Bind @ 0x004c_33e0 => extern "C" fn h1_004c33e0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004c_33e0) })
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture2DBase__destroyInstance @ 0x004c_3c20 => extern "thiscall" fn h1_004c3c20(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004c_3c20) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PTexture2D__fixObjectsAfterLoading @ 0x004c_3c40 => extern "thiscall" fn h1_004c3c40(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004c_3c40) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PTexture3D_Bind()`: registers its class descriptor.
    Phyre_PRendering_PTexture3D_Bind @ 0x004c_4c20 => extern "C" fn h1_004c4c20() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004c_4c20) })
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture3DBase__destroyInstance @ 0x004c_5490 => extern "thiscall" fn h1_004c5490(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004c_5490) }, p))
    }
    /// `Phyre_PRendering_PTextureCubeMap_Bind()`: registers its class descriptor.
    Phyre_PRendering_PTextureCubeMap_Bind @ 0x004c_64e0 => extern "C" fn h1_004c64e0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004c_64e0) })
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTextureCubeMapBase__destroyInstance @ 0x004c_6cb0 => extern "thiscall" fn h1_004c6cb0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004c_6cb0) }, p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PTextureCubeMap__fixObjectsAfterLoading @ 0x004c_6cd0 => extern "thiscall" fn h1_004c6cd0(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004c_6cd0) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PRenderTarget_Bind()`: registers its class descriptor.
    Phyre_PRendering_PRenderTarget_Bind @ 0x004c_7b50 => extern "C" fn h1_004c7b50() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004c_7b50) })
    }
    /// `destroyInstance(descriptor, p)`: the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PRenderTarget__destroyInstance @ 0x004c_81a0 => extern "thiscall" fn h1_004c81a0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy(&mut mem(), const { descriptor::callee_of(descriptor::DESTROY, 0x004c_81a0) }, p))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PSharray_unsigned_int__constructInstance @ 0x004d_0550 => extern "thiscall" fn h1_004d0550(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004d_0550) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PNodeContext__constructInstance @ 0x004d_0570 => extern "thiscall" fn h1_004d0570(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004d_0570) }))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: nothing to fix: 0.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PNodeContext__fixObjectsAfterLoading @ 0x004d_0640 => extern "thiscall" fn h1_004d0640(_descriptor: u32, _p: u32) -> u32 {
        const { descriptor::listed(descriptor::FIX_NONE, 0x004d_0640) };
        descriptor::bridge::no_objects_to_fix()
    }
    /// `Phyre_PRendering_PMeshSegmentContext_Bind()`: registers its class descriptor.
    Phyre_PRendering_PMeshSegmentContext_Bind @ 0x004d_36e0 => extern "C" fn h1_004d36e0() -> u32 {
        descriptor::bridge::bind(&mut mem(), const { descriptor::callee_of(descriptor::BIND, 0x004d_36e0) })
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PMaterialSwitch__constructInstance @ 0x004d_8610 => extern "thiscall" fn h1_004d8610(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004d_8610) }))
    }
}
