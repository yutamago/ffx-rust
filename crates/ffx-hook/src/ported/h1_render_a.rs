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
    /// `destroyInstance(descriptor, p)`: the object's virtual destructor.
    Phyre_PClassDescriptorForType_Phyre_PCameraOrthographic__destroyInstance @ 0x0045_5b70 => extern "thiscall" fn h1_00455b70(_descriptor: u32, p: u32) -> u8 {
        const { descriptor::listed(descriptor::VIRTUAL_DELETE, 0x0045_5b70) };
        u8::from(descriptor::bridge::destroy_virtual(&mut mem(), p))
    }
    /// `destroyInstance(descriptor, p)`: the object's virtual destructor.
    Phyre_PClassDescriptorForType_Phyre_PCameraPerspective__destroyInstance @ 0x0045_6950 => extern "thiscall" fn h1_00456950(_descriptor: u32, p: u32) -> u8 {
        const { descriptor::listed(descriptor::VIRTUAL_DELETE, 0x0045_6950) };
        u8::from(descriptor::bridge::destroy_virtual(&mut mem(), p))
    }
    /// `destroyInstance(descriptor, p)`: frees the untagged block at +4.
    Phyre_PClassDescriptorForType_Phyre_PAssetReferenceImport__destroyInstance @ 0x0045_dd10 => extern "thiscall" fn h1_0045dd10(_descriptor: u32, p: u32) -> u8 {
        const { descriptor::listed(descriptor::TAGGED_FREE, 0x0045_dd10) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlock__constructInstance @ 0x0048_3120 => extern "thiscall" fn h1_00483120(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0048_3120) }, p))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PVertexStream_4__destroyInstance @ 0x0048_31b0 => extern "thiscall" fn h1_004831b0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_31b0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlock__destroyInstance @ 0x0048_31f0 => extern "thiscall" fn h1_004831f0(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x0048_31f0) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PDataBlockBase__destroyInstance @ 0x0048_3260 => extern "thiscall" fn h1_00483260(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_3260) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004839d0 @ 0x0048_39d0 => extern "thiscall" fn h1_004839d0(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0048_39d0) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PVertexStream_4__setToExternalBuffer @ 0x0048_3a20 => extern "thiscall" fn h1_00483a20(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0048_3a20) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDynamicDataBlock__constructInstance @ 0x0048_4830 => extern "thiscall" fn h1_00484830(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_4830) }))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlock__constructInstance @ 0x0048_5c80 => extern "thiscall" fn h1_00485c80(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0048_5c80) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PIndexDataBlock__destroyInstance @ 0x0048_5d10 => extern "thiscall" fn h1_00485d10(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x0048_5d10) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMeshSegment__constructInstance @ 0x0048_8ef0 => extern "thiscall" fn h1_00488ef0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x0048_8ef0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__destroyInstance @ 0x0048_8fb0 => extern "thiscall" fn h1_00488fb0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_8fb0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMeshSegmentBase__destroyInstance @ 0x0048_9080 => extern "thiscall" fn h1_00489080(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_9080) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_00489880 @ 0x0048_9880 => extern "thiscall" fn h1_00489880(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0048_9880) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__setToExternalBuffer @ 0x0048_98d0 => extern "thiscall" fn h1_004898d0(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0048_98d0) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PRendering_PMaterial__destroyInstance @ 0x0048_be50 => extern "thiscall" fn h1_0048be50(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_be50) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PGeometry_PMaterialSet__destroyInstance @ 0x0048_be90 => extern "thiscall" fn h1_0048be90(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0048_be90) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `getArrayStart(member, object)`: the member's getter, or the data word at the member's offset.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMaterial__getArrayStart @ 0x0048_c060 => extern "thiscall" fn h1_0048c060(member: u32, object: u32) -> u32 {
        descriptor::bridge::member_start(&mut mem(), member, object)
    }
    /// `getCount(member, object)`: the member's getter, or 0.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMaterial__getCount @ 0x0048_c140 => extern "thiscall" fn h1_0048c140(member: u32, object: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_COUNT, 0x0048_c140) };
        descriptor::bridge::member_count(&mut mem(), member, object)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_Phyre_PRendering_PMaterial__setToExternalBuffer @ 0x0048_c730 => extern "thiscall" fn h1_0048c730(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0048_c730) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_int_4__destroyInstance @ 0x0049_3de0 => extern "thiscall" fn h1_00493de0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_3de0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PMatrix4_4__destroyInstance @ 0x0049_3e20 => extern "thiscall" fn h1_00493e20(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_3e20) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PMeshSegment_4__destroyInstance @ 0x0049_3e60 => extern "thiscall" fn h1_00493e60(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_3e60) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PSkeletonJointBounds_4__destroyInstance @ 0x0049_3ea0 => extern "thiscall" fn h1_00493ea0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_3ea0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PString_4__destroyInstance @ 0x0049_3ee0 => extern "thiscall" fn h1_00493ee0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_3ee0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_00495490 @ 0x0049_5490 => extern "thiscall" fn h1_00495490(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_5490) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004954e0 @ 0x0049_54e0 => extern "thiscall" fn h1_004954e0(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_54e0) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_00495530 @ 0x0049_5530 => extern "thiscall" fn h1_00495530(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_5530) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_00495580 @ 0x0049_5580 => extern "thiscall" fn h1_00495580(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_5580) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_int_4__setToExternalBuffer @ 0x0049_55d0 => extern "thiscall" fn h1_004955d0(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_55d0) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PMatrix4_4__setToExternalBuffer @ 0x0049_5600 => extern "thiscall" fn h1_00495600(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_5600) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PMeshSegment_4__setToExternalBuffer @ 0x0049_5630 => extern "thiscall" fn h1_00495630(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_5630) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PSkeletonJointBounds_4__setToExternalBuffer @ 0x0049_5660 => extern "thiscall" fn h1_00495660(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_5660) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PString_4__setToExternalBuffer @ 0x0049_5690 => extern "thiscall" fn h1_00495690(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_5690) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc__constructInstance @ 0x0049_8160 => extern "thiscall" fn h1_00498160(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_8160) }))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_4__destroyInstance @ 0x0049_8480 => extern "thiscall" fn h1_00498480(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x0049_8480) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_00499220 @ 0x0049_9220 => extern "thiscall" fn h1_00499220(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_9220) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_4__setToExternalBuffer @ 0x0049_9270 => extern "thiscall" fn h1_00499270(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x0049_9270) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBuffer__constructInstance @ 0x004a_04e0 => extern "thiscall" fn h1_004a04e0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004a_04e0) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PConstantBuffer__destroyInstance @ 0x004a_0560 => extern "thiscall" fn h1_004a0560(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004a_0560) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `destroyInstance(descriptor, p)`: frees the untagged block at +4.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderParameterDefinition__destroyInstance @ 0x004a_2f60 => extern "thiscall" fn h1_004a2f60(_descriptor: u32, p: u32) -> u8 {
        const { descriptor::listed(descriptor::TAGGED_FREE, 0x004a_2f60) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p))
    }
    /// `(data, count)`: frees each element's untagged block, then `data`.
    PArray_releaseTaggedElements_004ab4d0 @ 0x004a_b4d0 => extern "C" fn h1_004ab4d0(data: u32, count: u32) {
        const { descriptor::listed(descriptor::RELEASE_TAGGED, 0x004a_b4d0) };
        descriptor::bridge::release_tagged(&mut mem(), data, count);
    }
    /// `(data, count)`: frees each element's untagged block, then `data`.
    PArray_releaseTaggedElements_004ab510 @ 0x004a_b510 => extern "C" fn h1_004ab510(data: u32, count: u32) {
        const { descriptor::listed(descriptor::RELEASE_TAGGED, 0x004a_b510) };
        descriptor::bridge::release_tagged(&mut mem(), data, count);
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderProgramParamsAndStreams__constructInstance @ 0x004b_0290 => extern "thiscall" fn h1_004b0290(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0290) }))
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderComputeProgram__constructInstance @ 0x004b_02f0 => extern "thiscall" fn h1_004b02f0(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004b_02f0) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderFragmentProgram__constructInstance @ 0x004b_0360 => extern "thiscall" fn h1_004b0360(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004b_0360) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderGeometryProgram__constructInstance @ 0x004b_03d0 => extern "thiscall" fn h1_004b03d0(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004b_03d0) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderVertexProgram__constructInstance @ 0x004b_04c0 => extern "thiscall" fn h1_004b04c0(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004b_04c0) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderParameterDefinition_4__destroyInstance @ 0x004b_0530 => extern "thiscall" fn h1_004b0530(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_0530) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderStreamDefinition_4__destroyInstance @ 0x004b_0570 => extern "thiscall" fn h1_004b0570(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_0570) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: frees the untagged block at +4.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderStreamDefinition__destroyInstance @ 0x004b_0750 => extern "thiscall" fn h1_004b0750(_descriptor: u32, p: u32) -> u8 {
        const { descriptor::listed(descriptor::TAGGED_FREE, 0x004b_0750) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004b2660 @ 0x004b_2660 => extern "thiscall" fn h1_004b2660(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x004b_2660) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004b26b0 @ 0x004b_26b0 => extern "thiscall" fn h1_004b26b0(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x004b_26b0) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderParameterDefinition_4__setToExternalBuffer @ 0x004b_2700 => extern "thiscall" fn h1_004b2700(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x004b_2700) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderStreamDefinition_4__setToExternalBuffer @ 0x004b_2730 => extern "thiscall" fn h1_004b2730(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x004b_2730) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPass__constructInstance @ 0x004b_9690 => extern "thiscall" fn h1_004b9690(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004b_9690) }, p))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassBase__constructInstance @ 0x004b_96e0 => extern "thiscall" fn h1_004b96e0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_96e0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassParameterLocationTypes__constructInstance @ 0x004b_9720 => extern "thiscall" fn h1_004b9720(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9720) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassParameterLocationTypesConstantBuffer__constructInstance @ 0x004b_97b0 => extern "thiscall" fn h1_004b97b0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_97b0) }))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocation_2__destroyInstance @ 0x004b_9820 => extern "thiscall" fn h1_004b9820(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_9820) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationType_2__destroyInstance @ 0x004b_9860 => extern "thiscall" fn h1_004b9860(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_9860) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_4__destroyInstance @ 0x004b_98a0 => extern "thiscall" fn h1_004b98a0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_98a0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassBase__destroyInstance @ 0x004b_9900 => extern "thiscall" fn h1_004b9900(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_9900) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassParameterLocationTypes__destroyInstance @ 0x004b_9940 => extern "thiscall" fn h1_004b9940(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_9940) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassParameterLocationTypesConstantBuffer__destroyInstance @ 0x004b_9990 => extern "thiscall" fn h1_004b9990(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004b_9990) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004baa70 @ 0x004b_aa70 => extern "thiscall" fn h1_004baa70(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x004b_aa70) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004bab10 @ 0x004b_ab10 => extern "thiscall" fn h1_004bab10(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x004b_ab10) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocation_2__setToExternalBuffer @ 0x004b_ab60 => extern "thiscall" fn h1_004bab60(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x004b_ab60) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_4__setToExternalBuffer @ 0x004b_abc0 => extern "thiscall" fn h1_004babc0(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x004b_abc0) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `PArray::setToExternalBuffer(array, buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004bdde0 @ 0x004b_dde0 => extern "thiscall" fn h1_004bdde0(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x004b_dde0) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `setToExternalBuffer(member, object, buffer, count)`: the member's setter, or 0x15.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PArray_Phyre_PRendering_PShaderPass_4__setToExternalBuffer @ 0x004b_de30 => extern "thiscall" fn h1_004bde30(member: u32, object: u32, buffer: u32, count: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_SET_EXTERNAL, 0x004b_de30) };
        descriptor::bridge::member_set_external(&mut mem(), member, object, buffer, count)
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSamplerState__constructInstance @ 0x004c_0140 => extern "thiscall" fn h1_004c0140(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004c_0140) }, p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture2D__constructInstance @ 0x004c_3b00 => extern "thiscall" fn h1_004c3b00(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004c_3b00) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture2D__destroyInstance @ 0x004c_3bb0 => extern "thiscall" fn h1_004c3bb0(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004c_3bb0) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture3D__constructInstance @ 0x004c_5360 => extern "thiscall" fn h1_004c5360(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004c_5360) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTexture3D__destroyInstance @ 0x004c_5420 => extern "thiscall" fn h1_004c5420(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004c_5420) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `constructInstance(descriptor, p)`: the constructor in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCubeMap__constructInstance @ 0x004c_6b90 => extern "thiscall" fn h1_004c6b90(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct(&mut mem(), const { descriptor::callee_of(descriptor::CONSTRUCT, 0x004c_6b90) }, p))
    }
    /// `destroyInstance(descriptor, p)`: the release when owned, then the destructor.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTextureCubeMap__destroyInstance @ 0x004c_6c40 => extern "thiscall" fn h1_004c6c40(_descriptor: u32, p: u32) -> u8 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004c_6c40) };
        u8::from(descriptor::bridge::destroy_checked(&mut mem(), calls, p))
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PRenderTarget__constructInstance @ 0x004c_8130 => extern "thiscall" fn h1_004c8130(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004c_8130) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PSharray_unsigned_int__destroyInstance @ 0x004d_0590 => extern "thiscall" fn h1_004d0590(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004d_0590) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `destroyInstance(descriptor, p)`: releases the array's block, empties it.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PNodeContext__destroyInstance @ 0x004d_05e0 => extern "thiscall" fn h1_004d05e0(_descriptor: u32, p: u32) -> u8 {
        let (off, release) = const { descriptor::array_of(0x004d_05e0) };
        u8::from(descriptor::bridge::destroy_array(&mut mem(), p, off, release))
    }
    /// `getCount(member, object)`: the member's getter, or 0.
    Phyre_PClassDataMemberArrayDynamic_Phyre_PSharray_unsigned_int__getCount @ 0x004d_0780 => extern "thiscall" fn h1_004d0780(member: u32, object: u32) -> u32 {
        const { descriptor::listed(descriptor::MEMBER_COUNT, 0x004d_0780) };
        descriptor::bridge::member_count(&mut mem(), member, object)
    }
    /// `constructInstance(descriptor, p)`: copies the descriptor's default object.
    Phyre_PClassDescriptorHeaderWithoutDefaultConstructor_Phyre_PRendering_PParameterBuffer__constructInstance @ 0x004d_70d0 => extern "thiscall" fn h1_004d70d0(_descriptor: u32, p: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::CONSTRUCT_DEFAULT, 0x004d_70d0) };
        descriptor::bridge::construct_default(&mut mem(), calls, p)
    }
}
