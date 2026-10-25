//! M4 port round 1, list h1-render-a: `PhyreEngine` render code
//! (`ffx_prender`), lowest third by address.
//!
//! Threads: the class-descriptor callbacks run where objects are made,
//! loaded and freed (the main thread and the loader); `Bind` at start-up.
//! No shim keeps state.

use super::{mem, ported};
use ffx_prender::{basic, class_slots as descriptor, h1_bind, registry, texture};

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
        let off = const { descriptor::callee_of(descriptor::TAGGED_FREE, 0x0045_dd10) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p, off))
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
        let off = const { descriptor::callee_of(descriptor::TAGGED_FREE, 0x004a_2f60) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p, off))
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
        let off = const { descriptor::callee_of(descriptor::TAGGED_FREE, 0x004b_0750) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p, off))
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
    /// `constructInstance(descriptor, p)`: an empty world in place.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PWorld__constructInstance @ 0x0044_3400 => extern "thiscall" fn h1_00443400(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_list_head(&mut mem(), p))
    }
    /// `destroyInstance(descriptor, p)`: frees the untagged block in `p`.
    Phyre_PClassDescriptorForType_Phyre_PCamera__destroyInstance @ 0x0045_4980 => extern "thiscall" fn h1_00454980(_descriptor: u32, p: u32) -> u8 {
        let off = const { descriptor::callee_of(descriptor::TAGGED_FREE, 0x0045_4980) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p, off))
    }
    /// `constructInstance(descriptor, p)`: the class's own function on `p`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PAssetReference__constructInstance @ 0x0045_dcd0 => extern "thiscall" fn h1_0045dcd0(_descriptor: u32, p: u32) -> u32 {
        descriptor::bridge::forward(&mut mem(), const { descriptor::callee_of(descriptor::FORWARD, 0x0045_dcd0) }, p)
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: the class's own function on `p`.
    Phyre_PClassDescriptorAbstract_Phyre_PAssetReference__fixObjectsAfterLoading @ 0x0045_dd70 => extern "thiscall" fn h1_0045dd70(_descriptor: u32, p: u32) -> u32 {
        descriptor::bridge::forward(&mut mem(), const { descriptor::callee_of(descriptor::FORWARD, 0x0045_dd70) }, p)
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PVertexStream__constructInstance @ 0x0048_0a00 => extern "thiscall" fn h1_00480a00(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_0a00) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PIndexDataBlockBase__constructInstance @ 0x0048_5cd0 => extern "thiscall" fn h1_00485cd0(_descriptor: u32, p: u32) -> u8 {
        let spans = const { descriptor::spans_of(0x0048_5cd0) };
        u8::from(descriptor::bridge::construct_spans(&mut mem(), p, spans))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PSkeletonJointBounds__constructInstance @ 0x0049_3da0 => extern "thiscall" fn h1_00493da0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0049_3da0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PConstantBufferBase__constructInstance @ 0x004a_0530 => extern "thiscall" fn h1_004a0530(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_0530) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer__constructInstance @ 0x004a_2ea0 => extern "thiscall" fn h1_004a2ea0(_descriptor: u32, p: u32) -> u8 {
        let spans = const { descriptor::spans_of(0x004a_2ea0) };
        u8::from(descriptor::bridge::construct_spans(&mut mem(), p, spans))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferByteAddressBuffer__constructInstance @ 0x004a_ffc0 => extern "thiscall" fn h1_004affc0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_ffc0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferDataBlock__constructInstance @ 0x004b_0000 => extern "thiscall" fn h1_004b0000(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0000) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferIndexDataBlock__constructInstance @ 0x004b_0030 => extern "thiscall" fn h1_004b0030(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0030) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferRWByteAddressBuffer__constructInstance @ 0x004b_0060 => extern "thiscall" fn h1_004b0060(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0060) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferRWStructuredBuffer__constructInstance @ 0x004b_00a0 => extern "thiscall" fn h1_004b00a0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_00a0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferRWTexture2D__constructInstance @ 0x004b_00e0 => extern "thiscall" fn h1_004b00e0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_00e0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferRWTexture3D__constructInstance @ 0x004b_0110 => extern "thiscall" fn h1_004b0110(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0110) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferSampler__constructInstance @ 0x004b_0140 => extern "thiscall" fn h1_004b0140(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0140) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferStructuredBuffer__constructInstance @ 0x004b_0180 => extern "thiscall" fn h1_004b0180(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0180) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferTexture2D__constructInstance @ 0x004b_01c0 => extern "thiscall" fn h1_004b01c0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_01c0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferTexture3D__constructInstance @ 0x004b_01f0 => extern "thiscall" fn h1_004b01f0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_01f0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureBufferTextureCubeMap__constructInstance @ 0x004b_0220 => extern "thiscall" fn h1_004b0220(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0220) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterCaptureConstantBuffer__constructInstance @ 0x004b_0250 => extern "thiscall" fn h1_004b0250(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0250) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderStreamDefinition__constructInstance @ 0x004b_02c0 => extern "thiscall" fn h1_004b02c0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_02c0) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderParameterCaptureBufferTextureBase__constructInstance @ 0x004b_0440 => extern "thiscall" fn h1_004b0440(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_0440) }))
    }
    /// `constructInstance(descriptor, p)`: the default fields in place.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderPassParameterLocationTypesBase__constructInstance @ 0x004b_9770 => extern "thiscall" fn h1_004b9770(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_9770) }))
    }
    /// `destroyInstance(descriptor, p)`: the texture base's destructor, inlined.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PTextureCommonBase__destroyInstance @ 0x004c_26e0 => extern "thiscall" fn h1_004c26e0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::destroy_texture(&mut mem(), p))
    }
    /// `fixObjectsAfterLoading(descriptor, p)`: the class's own function on `p`.
    Phyre_PClassDescriptorAbstract_Phyre_PRendering_PParameterBuffer__fixObjectsAfterLoading @ 0x004d_7230 => extern "thiscall" fn h1_004d7230(_descriptor: u32, p: u32) -> u32 {
        descriptor::bridge::forward(&mut mem(), const { descriptor::callee_of(descriptor::FORWARD, 0x004d_7230) }, p)
    }
    /// `destroyInstance(descriptor, p)`: frees the untagged block in `p`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PMaterial__destroyInstance @ 0x004d_86c0 => extern "thiscall" fn h1_004d86c0(_descriptor: u32, p: u32) -> u8 {
        let off = const { descriptor::callee_of(descriptor::TAGGED_FREE, 0x004d_86c0) };
        u8::from(descriptor::bridge::destroy_tagged(&mut mem(), p, off))
    }
    /// `0x0042fcc0(this)` -> `this`.
    PhyreCtor_zeroWord_0042fcc0 @ 0x0042_fcc0 => extern "thiscall" fn h1_0042fcc0(this: u32) -> u32 {
        basic::bridge::clear_word(&mut mem(), this)
    }
    /// `PScene::PUtilityScene::initialize()` -> 0.
    Phyre_PScene_PUtilityScene_initialize @ 0x0043_2be0 => extern "C" fn h1_00432be0() -> u32 {
        basic::bridge::scene_initialize(&mut mem())
    }
    /// `PSprite::PUtilitySprite::initialize()` -> 0.
    Phyre_PSprite_PUtilitySprite_initialize @ 0x0043_4be0 => extern "C" fn h1_00434be0() -> u32 {
        basic::bridge::sprite_initialize(&mut mem())
    }
    /// `PWorld::PWorld(this)` -> `this`.
    Phyre_PWorld_PWorld @ 0x0044_29e0 => extern "thiscall" fn h1_004429e0(this: u32) -> u32 {
        basic::bridge::world_ctor(&mut mem(), this)
    }
    /// `PWorld_classEntry(world, descriptor)` -> the entry or 0.
    PWorld_classEntry_004437e0 @ 0x0044_37e0 => extern "thiscall" fn h1_004437e0(world: u32, descriptor: u32) -> u32 {
        basic::bridge::class_entry(&mem(), world, descriptor)
    }
    /// `0x00464710()` -> the renderer id `"11XD"`.
    PRenderInterfaceD3D11_platformId_00464710 @ 0x0046_4710 => extern "C" fn h1_00464710() -> u32 {
        basic::RENDERER_ID
    }
    /// `0x0047fac0(this, a, b, c, d, _)`: `this`'s virtual slot 1.
    PIChildClassVisitor_forward_0047fac0 @ 0x0047_fac0 => extern "thiscall" fn h1_0047fac0(this: u32, a: u32, b: u32, c: u32, d: u32, _e: u32) -> u32 {
        basic::bridge::forward_visit(&mut mem(), this, [a, b, c, d])
    }
    /// `0x0047fbe0(this, a, b)` -> `this`.
    PhyreStaticEntry_ctor_0047fbe0 @ 0x0047_fbe0 => extern "thiscall" fn h1_0047fbe0(this: u32, a: u32, b: u32) -> u32 {
        basic::bridge::ctor_47fbe0(&mut mem(), this, a, b as u8)
    }
    /// `0x00482d70(data, count)`: default 12-byte elements.
    PhyreInitElements12_00482d70 @ 0x0048_2d70 => extern "C" fn h1_00482d70(data: u32, count: u32) {
        basic::bridge::init_elements(&mut mem(), data, count);
    }
    /// `0x004835d0(this, a, b, c)` -> the sub-constructor's `eax`.
    PhyreCtor_withSub8_004835d0 @ 0x0048_35d0 => extern "thiscall" fn h1_004835d0(this: u32, a: u32, b: u32, c: u32) -> u32 {
        basic::bridge::ctor_4835d0(&mut mem(), this, a, b, c)
    }
    /// `PDynamicMesh::PDynamicMesh(this)` -> `this`.
    Phyre_PGeometry_PDynamicMesh_PDynamicMesh @ 0x0049_68c0 => extern "thiscall" fn h1_004968c0(this: u32) -> u32 {
        basic::bridge::dynamic_mesh_ctor(&mut mem(), this)
    }
    /// `0x0049f990(this)` -> `this`.
    PhyreCtor_zero9_0049f990 @ 0x0049_f990 => extern "thiscall" fn h1_0049f990(this: u32) -> u32 {
        basic::bridge::clear_9(&mut mem(), this)
    }
    /// `0x004a1660(this)` -> `this`.
    PhyreCtor_flags34460000_004a1660 @ 0x004a_1660 => extern "thiscall" fn h1_004a1660(this: u32) -> u32 {
        basic::bridge::ctor_4a1660(&mut mem(), this)
    }
    /// `0x004b5050(this)` -> `this`.
    PhyreCtor_zero24_004b5050 @ 0x004b_5050 => extern "thiscall" fn h1_004b5050(this: u32) -> u32 {
        basic::bridge::clear_24(&mut mem(), this)
    }
    /// `0x004bc850(data, count)`: destroys the passes, frees the block.
    PArray_releaseShaderPasses_004bc850 @ 0x004b_c850 => extern "C" fn h1_004bc850(data: u32, count: u32) {
        basic::bridge::release_shader_passes(&mut mem(), data, count);
    }
    /// `0x004bd770(this, key)` -> the entry or 0.
    PhyreFindEntry16_004bd770 @ 0x004b_d770 => extern "thiscall" fn h1_004bd770(this: u32, key: u32) -> u32 {
        basic::bridge::find_entry(&mem(), this, key)
    }
    /// `0x004be860(this)` -> `this`.
    PSamplerStateD3D11_new_004be860 @ 0x004b_e860 => extern "thiscall" fn h1_004be860(this: u32) -> u32 {
        basic::bridge::sampler_d3d11_new(&mut mem(), this)
    }
    /// `PSamplerStateBase::PSamplerStateBase(this)` -> `this`.
    PSamplerStateBase_ctor @ 0x004b_e870 => extern "thiscall" fn h1_004be870(this: u32) -> u32 {
        texture::bridge::sampler_ctor(&mut mem(), this)
    }
    /// `PSamplerStateBase::setMinFilter(this, v)` -> 0 or 0x16.
    Phyre_PRendering_PSamplerStateBase_setMinFilter @ 0x004c_07f0 => extern "thiscall" fn h1_004c07f0(this: u32, v: u32) -> u32 {
        texture::bridge::sampler_set(&mut mem(), this, texture::SET_MIN_FILTER, v as u8)
    }
    /// `PSamplerStateBase::setWrapR(this, v)` -> 0 or 0x16.
    Phyre_PRendering_PSamplerStateBase_setWrapR @ 0x004c_0840 => extern "thiscall" fn h1_004c0840(this: u32, v: u32) -> u32 {
        texture::bridge::sampler_set(&mut mem(), this, texture::SET_WRAP_R, v as u8)
    }
    /// `PSamplerStateBase::setWrapS(this, v)` -> 0 or 0x16.
    Phyre_PRendering_PSamplerStateBase_setWrapS @ 0x004c_0870 => extern "thiscall" fn h1_004c0870(this: u32, v: u32) -> u32 {
        texture::bridge::sampler_set(&mut mem(), this, texture::SET_WRAP_S, v as u8)
    }
    /// `PSamplerStateBase::setWrapT(this, v)` -> 0 or 0x16.
    Phyre_PRendering_PSamplerStateBase_setWrapT @ 0x004c_08a0 => extern "thiscall" fn h1_004c08a0(this: u32, v: u32) -> u32 {
        texture::bridge::sampler_set(&mut mem(), this, texture::SET_WRAP_T, v as u8)
    }
    /// `PTextureCommonBase::PTextureCommonBase(this)` -> `this`.
    Phyre_PRendering_PTextureCommonBase_PTextureCommonBase @ 0x004c_1d70 => extern "thiscall" fn h1_004c1d70(this: u32) -> u32 {
        texture::bridge::common_ctor(&mut mem(), this)
    }
    /// `PTextureCommonBase`'s destructor (`this`).
    Phyre_PRendering_PTextureCommonBase_dtor_004c1df0 @ 0x004c_1df0 => extern "thiscall" fn h1_004c1df0(this: u32) {
        texture::bridge::common_dtor(&mut mem(), this);
    }
    /// `GetMipCountForTextureSize(w, h, d)`.
    Phyre_PRendering_PTextureCommonBase_GetMipCountForTextureSize @ 0x004c_2430 => extern "C" fn h1_004c2430(w: u32, h: u32, d: u32) -> u32 {
        texture::mip_count(w, h, d)
    }
    /// `0x004c28f0(this, data)` -> 0 or 5.
    Phyre_PRendering_PTextureCommonBase_setData_004c28f0 @ 0x004c_28f0 => extern "thiscall" fn h1_004c28f0(this: u32, data: u32) -> u32 {
        texture::bridge::set_data(&mut mem(), this, data)
    }
    /// `PTexture2DBase::PTexture2DBase(this)` -> `this`.
    Phyre_PRendering_PTexture2DBase_PTexture2DBase @ 0x004c_2f90 => extern "thiscall" fn h1_004c2f90(this: u32) -> u32 {
        texture::bridge::ctor_2d(&mut mem(), this)
    }
    /// A derived texture's destructor: `PTextureCommonBase`'s (`jmp`).
    PTextureCommonBase_dtorThunk_004c3070 @ 0x004c_3070 => extern "thiscall" fn h1_004c3070(this: u32) {
        texture::bridge::common_dtor(&mut mem(), this);
    }
    /// `PTexture2DBase::updateMaxLevel(this)` -> the max level.
    Phyre_PRendering_PTexture2DBase_updateMaxLevel @ 0x004c_4250 => extern "thiscall" fn h1_004c4250(this: u32) -> u32 {
        texture::bridge::update_max_level(&mut mem(), this)
    }
    /// `PTexture3DBase::PTexture3DBase(this)` -> `this`.
    Phyre_PRendering_PTexture3DBase_PTexture3DBase @ 0x004c_47a0 => extern "thiscall" fn h1_004c47a0(this: u32) -> u32 {
        texture::bridge::ctor_3d(&mut mem(), this)
    }
    /// A derived texture's destructor: `PTextureCommonBase`'s (`jmp`).
    PTextureCommonBase_dtorThunk_004c4890 @ 0x004c_4890 => extern "thiscall" fn h1_004c4890(this: u32) {
        texture::bridge::common_dtor(&mut mem(), this);
    }
    /// `PTextureCubeMapBase::PTextureCubeMapBase(this)` -> `this`.
    Phyre_PRendering_PTextureCubeMapBase_PTextureCubeMapBase @ 0x004c_6070 => extern "thiscall" fn h1_004c6070(this: u32) -> u32 {
        texture::bridge::ctor_cube(&mut mem(), this)
    }
    /// A derived texture's destructor: `PTextureCommonBase`'s (`jmp`).
    PTextureCommonBase_dtorThunk_004c6150 @ 0x004c_6150 => extern "thiscall" fn h1_004c6150(this: u32) {
        texture::bridge::common_dtor(&mut mem(), this);
    }
    /// The texture data object's destructor: its release (`jmp`), whose
    /// `eax` it returns.
    PTextureData_releaseThunk_004c79f0 @ 0x004c_79f0 => extern "thiscall" fn h1_004c79f0(this: u32) -> u32 {
        texture::bridge::release_data(&mut mem(), this)
    }
    /// `0x004d3700(a, b)` -> 1 when `a`'s second word is below `b`'s.
    PhyreLess_bySecondWord_004d3700 @ 0x004d_3700 => extern "C" fn h1_004d3700(a: u32, b: u32) -> u32 {
        basic::bridge::less_by_second(&mem(), a, b)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PTypedObject_4__PClassDescriptorAbstract @ 0x0045_77e0 => extern "thiscall" fn h1_004577e0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0045_77e0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PVertexStream_4__PClassDescriptorAbstract @ 0x0048_1470 => extern "thiscall" fn h1_00481470(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_1470) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2__PClassDescriptorAbstract @ 0x0048_6c20 => extern "thiscall" fn h1_00486c20(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_6c20) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PSharray_Phyre_PRendering_PMaterial__PClassDescriptorAbstract @ 0x0048_a310 => extern "thiscall" fn h1_0048a310(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_a310) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_int_4__PClassDescriptorAbstract @ 0x0048_e7e0 => extern "thiscall" fn h1_0048e7e0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_e7e0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PMatrix4_4__PClassDescriptorAbstract @ 0x0048_e880 => extern "thiscall" fn h1_0048e880(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_e880) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PMeshSegment_4__PClassDescriptorAbstract @ 0x0048_e920 => extern "thiscall" fn h1_0048e920(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_e920) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PSkeletonJointBounds_4__PClassDescriptorAbstract @ 0x0048_e9c0 => extern "thiscall" fn h1_0048e9c0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_e9c0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PString_4__PClassDescriptorAbstract @ 0x0048_ea60 => extern "thiscall" fn h1_0048ea60(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0048_ea60) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_4__PClassDescriptorAbstract @ 0x0049_6610 => extern "thiscall" fn h1_00496610(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0049_6610) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_unsigned_char_4__PClassDescriptorAbstract @ 0x0049_9b00 => extern "thiscall" fn h1_00499b00(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x0049_9b00) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// `PTypeDispenser::GetType<T>()` -> the type object.
    Phyre_PTypeDispenser_GetType_0049d050 @ 0x0049_d050 => extern "C" fn h1_0049d050() -> u32 {
        let k = const { registry::singleton_of(0x0049_d050) };
        registry::bridge::type_singleton(&mut mem(), k)
    }
    /// `PTypeDispenser::GetType<T>()` -> the type object.
    Phyre_PTypeDispenser_GetType_0049d100 @ 0x0049_d100 => extern "C" fn h1_0049d100() -> u32 {
        let k = const { registry::singleton_of(0x0049_d100) };
        registry::bridge::type_singleton(&mut mem(), k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderParameterDefinition_4__PClassDescriptorAbstract @ 0x004a_6730 => extern "thiscall" fn h1_004a6730(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004a_6730) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderStreamDefinition_4__PClassDescriptorAbstract @ 0x004a_67d0 => extern "thiscall" fn h1_004a67d0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004a_67d0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004a9ec0 @ 0x004a_9ec0 => extern "C" fn h1_004a9ec0() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_9ec0) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004a9f60 @ 0x004a_9f60 => extern "C" fn h1_004a9f60() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_9f60) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa210 @ 0x004a_a210 => extern "C" fn h1_004aa210() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a210) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa2b0 @ 0x004a_a2b0 => extern "C" fn h1_004aa2b0() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a2b0) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa350 @ 0x004a_a350 => extern "C" fn h1_004aa350() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a350) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa4f0 @ 0x004a_a4f0 => extern "C" fn h1_004aa4f0() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a4f0) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa590 @ 0x004a_a590 => extern "C" fn h1_004aa590() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a590) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa760 @ 0x004a_a760 => extern "C" fn h1_004aa760() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a760) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// `T::Bind()` with a member descriptor -> `addToNamespace`'s `eax`.
    PClassBind_withMember_004aa800 @ 0x004a_a800 => extern "C" fn h1_004aa800() -> u32 {
        let (d, k) = const { registry::bind_member_of(0x004a_a800) };
        registry::bridge::bind_member(&mut mem(), d, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocation_2__PClassDescriptorAbstract @ 0x004b_4790 => extern "thiscall" fn h1_004b4790(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004b_4790) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationType_2__PClassDescriptorAbstract @ 0x004b_4830 => extern "thiscall" fn h1_004b4830(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004b_4830) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_4__PClassDescriptorAbstract @ 0x004b_48d0 => extern "thiscall" fn h1_004b48d0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004b_48d0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShaderPass_4__PClassDescriptorAbstract @ 0x004b_b9f0 => extern "thiscall" fn h1_004bb9f0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004b_b9f0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// `PTypeDispenser::GetType<T>()` -> the type object.
    Phyre_PTypeDispenser_GetType_Phyre_PRendering_PTextureFormatBase @ 0x004c_0b10 => extern "C" fn h1_004c0b10() -> u32 {
        let k = const { registry::singleton_of(0x004c_0b10) };
        registry::bridge::type_singleton(&mut mem(), k)
    }
    /// `PTypeDispenser::GetType<T>()` -> the type object.
    Phyre_PTypeDispenser_GetType_004c8a70 @ 0x004c_8a70 => extern "C" fn h1_004c8a70() -> u32 {
        let k = const { registry::singleton_of(0x004c_8a70) };
        registry::bridge::type_singleton(&mut mem(), k)
    }
    /// `PTypeDispenser::GetType<T>()` -> the type object.
    Phyre_PTypeDispenser_GetType_004c8ba0 @ 0x004c_8ba0 => extern "C" fn h1_004c8ba0() -> u32 {
        let k = const { registry::singleton_of(0x004c_8ba0) };
        registry::bridge::type_singleton(&mut mem(), k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4__PClassDescriptorAbstract @ 0x004c_9020 => extern "thiscall" fn h1_004c9020(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004c_9020) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// The descriptor's constructor (`this`) -> `this`.
    Phyre_PClassDescriptorAbstract_Phyre_PSharray_unsigned_int__PClassDescriptorAbstract @ 0x004c_f1d0 => extern "thiscall" fn h1_004cf1d0(this: u32) -> u32 {
        let k = const { registry::five_of(registry::DESCRIPTOR_CTOR, 0x004c_f1d0) };
        registry::bridge::descriptor_ctor(&mut mem(), this, k)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PGeometry_PVertexStream_4 @ 0x0048_2170 => extern "C" fn h1_00482170(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0048_2170) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PGeometry_PMeshSegment_4 @ 0x0049_0d70 => extern "C" fn h1_00490d70(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_0d70) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PGeometry_PSkeletonJointBounds_4 @ 0x0049_0f80 => extern "C" fn h1_00490f80(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_0f80) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PString_4 @ 0x0049_1190 => extern "C" fn h1_00491190(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_1190) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_4 @ 0x0049_7260 => extern "C" fn h1_00497260(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_7260) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderParameterDefinition_4 @ 0x004a_b0b0 => extern "C" fn h1_004ab0b0(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004a_b0b0) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderStreamDefinition_4 @ 0x004a_b2c0 => extern "C" fn h1_004ab2c0(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004a_b2c0) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_4 @ 0x004b_6d30 => extern "C" fn h1_004b6d30(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004b_6d30) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderPass_4 @ 0x004b_c640 => extern "C" fn h1_004bc640(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004b_c640) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShadowCaster_PShadowSplit_4 @ 0x004c_ae60 => extern "C" fn h1_004cae60(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004c_ae60) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockBase__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PDataBlockBase__constructInstance @ 0x0048_3170 => extern "thiscall" fn h1_00483170(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x0048_3170) }))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShader__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShader__constructInstance @ 0x004b_d5a0 => extern "thiscall" fn h1_004bd5a0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004b_d5a0) }))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCommonBase__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCommonBase__constructInstance @ 0x004c_26a0 => extern "thiscall" fn h1_004c26a0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_texture(&mut mem(), p))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSamplerStateBase__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PSamplerStateBase__constructInstance @ 0x004c_0190 => extern "thiscall" fn h1_004c0190(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_sampler(&mut mem(), p))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PCameraOrthographic__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PCameraOrthographic__constructInstance @ 0x0045_5b30 => extern "thiscall" fn h1_00455b30(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_camera_ortho(&mut mem(), descriptor::CAMERA_ORTHO, p))
    }
    /// `Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderSource__constructInstance` (thiscall).
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PRendering_PShaderSource__constructInstance @ 0x004b_0470 => extern "thiscall" fn h1_004b0470(_descriptor: u32, p: u32) -> u32 {
        descriptor::bridge::construct_default_fields(&mut mem(), descriptor::SHADER_SOURCE_DEFAULT, p)
    }
    /// `Phyre_PArray_setToExternalBuffer_00495440` (thiscall).
    Phyre_PArray_setToExternalBuffer_00495440 @ 0x0049_5440 => extern "thiscall" fn h1_00495440(array: u32, buffer: u32, count: u32) -> u32 {
        let release = const { descriptor::release_of(descriptor::SET_EXTERNAL, 0x0049_5440) };
        descriptor::bridge::set_external(&mut mem(), array, buffer, count, release)
    }
    /// `PWorld_initClassEntries_00443150` (C).
    PWorld_initClassEntries_00443150 @ 0x0044_3150 => extern "C" fn h1_00443150(data: u32, count: u32) {
        basic::bridge::init_list_heads(&mut mem(), data, count);
    }
    /// `PhyreInitElements16_00497db0` (C).
    PhyreInitElements16_00497db0 @ 0x0049_7db0 => extern "C" fn h1_00497db0(data: u32, count: u32) {
        basic::bridge::zero_elements16(&mut mem(), data, count);
    }
    /// `PhyreCopyElements16_0049f450` (C).
    PhyreCopyElements16_0049f450 @ 0x0049_f450 => extern "C" fn h1_0049f450(dst: u32, src: u32, count: u32) {
        basic::bridge::copy_elements16(&mut mem(), dst, src, count);
    }
    /// `PhyreCopyElements12_00480bb0` (C).
    PhyreCopyElements12_00480bb0 @ 0x0048_0bb0 => extern "C" fn h1_00480bb0(dst: u32, src: u32, count: u32) {
        basic::bridge::copy_elements10(&mut mem(), dst, src, count);
    }
    /// `PArray_releaseTaggedWords_00434360` (C).
    PArray_releaseTaggedWords_00434360 @ 0x0043_4360 => extern "C" fn h1_00434360(data: u32, count: u32) {
        basic::bridge::release_tagged_words(&mut mem(), data, count);
    }
    /// `PType_create_0047fdf0` (C).
    PType_create_0047fdf0 @ 0x0047_fdf0 => extern "C" fn h1_0047fdf0(out: u32, name: u32) -> u32 {
        basic::bridge::create_result(&mut mem(), 0x0047_fca0, out, name)
    }
    /// `PType_create_0049e900` (C).
    PType_create_0049e900 @ 0x0049_e900 => extern "C" fn h1_0049e900(out: u32, name: u32) -> u32 {
        basic::bridge::create_result(&mut mem(), 0x0049_e2c0, out, name)
    }
    /// `PType_create_004c13f0` (C).
    PType_create_004c13f0 @ 0x004c_13f0 => extern "C" fn h1_004c13f0(out: u32, name: u32) -> u32 {
        basic::bridge::create_result(&mut mem(), 0x004c_0f10, out, name)
    }
    /// `PType_register_0047fdb0` (C).
    PType_register_0047fdb0 @ 0x0047_fdb0 => extern "C" fn h1_0047fdb0(obj: u32) -> u32 {
        basic::bridge::register_named(&mut mem(), obj, 0x0047_fca0, 0x00c9_85b0, None)
    }
    /// `PType_register_004c1000` (C).
    PType_register_004c1000 @ 0x004c_1000 => extern "C" fn h1_004c1000(obj: u32) -> u32 {
        basic::bridge::register_named(&mut mem(), obj, 0x004c_0f10, 0x00ca_2f04, Some(0x00ca_2f08))
    }
    /// `Phyre_PCameraOrthographic_deletingDtor_00455560` (thiscall).
    Phyre_PCameraOrthographic_deletingDtor_00455560 @ 0x0045_5560 => extern "thiscall" fn h1_00455560(this: u32, flags: u32) -> u32 {
        basic::bridge::camera_delete(&mut mem(), this, flags, 0x00b1_0f5c)
    }
    /// `Phyre_PCameraPerspective_deletingDtor_00456500` (thiscall).
    Phyre_PCameraPerspective_deletingDtor_00456500 @ 0x0045_6500 => extern "thiscall" fn h1_00456500(this: u32, flags: u32) -> u32 {
        basic::bridge::camera_delete(&mut mem(), this, flags, 0x00b1_0f5c)
    }
    /// `PhyreCtor_eventOwner_004315e0` (thiscall).
    PhyreCtor_eventOwner_004315e0 @ 0x0043_15e0 => extern "thiscall" fn h1_004315e0(this: u32) -> u32 {
        basic::bridge::event_owner_ctor(&mut mem(), this)
    }
    /// `PhyreCtor_zero32_00463d10` (thiscall).
    PhyreCtor_zero32_00463d10 @ 0x0046_3d10 => extern "thiscall" fn h1_00463d10(this: u32) -> u32 {
        basic::bridge::clear_32(&mut mem(), this)
    }
    /// `PType_find_0047fca0` (C).
    PType_find_0047fca0 @ 0x0047_fca0 => extern "C" fn h1_0047fca0(name: u32) -> u32 {
        basic::bridge::find_named(&mem(), 0x00c9_85b0, 0, 4, name)
    }
    /// `PType_find_004c0f10` (C).
    PType_find_004c0f10 @ 0x004c_0f10 => extern "C" fn h1_004c0f10(name: u32) -> u32 {
        basic::bridge::find_named(&mem(), 0x00ca_2f04, 0, 4, name)
    }
    /// `PType_find_0049e2c0` (C).
    PType_find_0049e2c0 @ 0x0049_e2c0 => extern "C" fn h1_0049e2c0(name: u32) -> u32 {
        basic::bridge::find_named(&mem(), 0x00c9_aef8, 4, 8, name)
    }
    /// `PType_find_004cb090` (C).
    PType_find_004cb090 @ 0x004c_b090 => extern "C" fn h1_004cb090(name: u32) -> u32 {
        basic::bridge::find_named(&mem(), 0x00ca_43a8, 4, 8, name)
    }
    /// `PType_find_004d3780` (C).
    PType_find_004d3780 @ 0x004d_3780 => extern "C" fn h1_004d3780(name: u32) -> u32 {
        basic::bridge::find_named(&mem(), 0x00ca_4e18, 4, 8, name)
    }
    /// `PhyreFindByName_004d37e0` (C).
    PhyreFindByName_004d37e0 @ 0x004d_37e0 => extern "C" fn h1_004d37e0(name: u32) -> u32 {
        basic::bridge::find_named_by(&mut mem(), 0x00ca_4e18, 0x00b0_c398, name)
    }
    /// `PSharray_element20_004348c0` (thiscall).
    PSharray_element20_004348c0 @ 0x0043_48c0 => extern "thiscall" fn h1_004348c0(this: u32, i: u32) -> u32 {
        basic::bridge::sharray_element(&mem(), this, i)
    }
    /// `PSharray_element20_00434910` (thiscall).
    PSharray_element20_00434910 @ 0x0043_4910 => extern "thiscall" fn h1_00434910(this: u32, i: u32) -> u32 {
        basic::bridge::sharray_element(&mem(), this, i)
    }
    /// `PTextureData_ctor_004c78a0` (thiscall).
    PTextureData_ctor_004c78a0 @ 0x004c_78a0 => extern "thiscall" fn h1_004c78a0(this: u32, texture: u32) -> u32 {
        texture::bridge::data_ctor(&mut mem(), this, texture)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMeshSegmentBase__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PGeometry_PMeshSegmentBase__constructInstance @ 0x0048_8f40 => extern "thiscall" fn h1_00488f40(_descriptor: u32, p: u32) -> u8 {
        let spans = const { descriptor::spans_of(0x0048_8f40) };
        u8::from(descriptor::bridge::construct_spans(&mut mem(), p, spans))
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterDefinition__constructInstance` (thiscall).
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PShaderParameterDefinition__constructInstance @ 0x004a_2ec0 => extern "thiscall" fn h1_004a2ec0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_init(&mut mem(), p, const { descriptor::init_of(0x004a_2ec0) }))
    }
    /// `PhyreCopyElements20_00442290` (C).
    PhyreCopyElements20_00442290 @ 0x0044_2290 => extern "C" fn h1_00442290(dst: u32, src: u32, count: u32) {
        basic::bridge::copy_elements20(&mut mem(), dst, src, count);
    }
    /// `PSharray_element20_00434870` (thiscall).
    PSharray_element20_00434870 @ 0x0043_4870 => extern "thiscall" fn h1_00434870(this: u32, i: u32) -> u32 {
        basic::bridge::sharray_element_at(&mem(), this + 0x18, i)
    }
    /// `Phyre_PGeometry_PDynamicMesh_getDataBlockCount` (thiscall).
    Phyre_PGeometry_PDynamicMesh_getDataBlockCount @ 0x0049_8770 => extern "thiscall" fn h1_00498770(mesh: u32, kind: u32) -> u32 {
        basic::bridge::data_block_count(&mem(), mesh, kind)
    }
    /// `Phyre_PCamera_ConvertProjectionMatrixDepthRangeToZeroOne` (C).
    Phyre_PCamera_ConvertProjectionMatrixDepthRangeToZeroOne @ 0x0045_4260 => extern "C" fn h1_00454260(camera: u32, near: u32, far: u32) {
        basic::bridge::depth_range_zero_one(&mut mem(), camera, near, far);
    }
    /// `Phyre_PRendering_PMaterial_PMaterial` (thiscall).
    Phyre_PRendering_PMaterial_PMaterial @ 0x004d_7b10 => extern "thiscall" fn h1_004d7b10(this: u32, a: u32, b: u32) -> u32 {
        basic::bridge::material_ctor(&mut mem(), this, a, b)
    }
    /// `PSharray_setWord_0048c540` (thiscall).
    PSharray_setWord_0048c540 @ 0x0048_c540 => extern "thiscall" fn h1_0048c540(array: u32, i: u32, v: u32) -> u32 {
        basic::bridge::sharray_set(&mut mem(), array, i, v)
    }
    /// `PClassDataMember_applyDefaults_0048c6e0` (thiscall).
    PClassDataMember_applyDefaults_0048c6e0 @ 0x0048_c6e0 => extern "thiscall" fn h1_0048c6e0(this: u32, buffer: u32, flag: u32) -> u32 {
        basic::bridge::apply_defaults(&mut mem(), this, buffer, flag)
    }
    /// `PTextureIterator_ctor_00463bf0` (thiscall).
    PTextureIterator_ctor_00463bf0 @ 0x0046_3bf0 => extern "thiscall" fn h1_00463bf0(this: u32, list: u32) -> u32 {
        basic::bridge::iterator_ctor(&mut mem(), this, list)
    }
    /// `Phyre_PSprite_PUtilitySprite_PUtilitySprite` (thiscall).
    Phyre_PSprite_PUtilitySprite_PUtilitySprite @ 0x0043_3870 => extern "thiscall" fn h1_00433870(this: u32) -> u32 {
        basic::bridge::utility_sprite_ctor(&mut mem(), this)
    }
    /// `PDataBlock_setToExternalBuffer_00483810` (thiscall).
    PDataBlock_setToExternalBuffer_00483810 @ 0x0048_3810 => extern "thiscall" fn h1_00483810(this: u32, buffer: u32, count: u32) -> u32 {
        basic::bridge::set_external_at8(&mut mem(), this, buffer, count)
    }
    /// `Phyre_PWorld_releaseEntries_00442a80` (thiscall).
    Phyre_PWorld_releaseEntries_00442a80 @ 0x0044_2a80 => extern "thiscall" fn h1_00442a80(this: u32) {
        basic::bridge::world_release(&mut mem(), this);
    }
    /// `constructInstance(descriptor, p)` of `PTexture2DBase`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture2DBase__constructInstance @ 0x004c_3b50 => extern "thiscall" fn h1_004c3b50(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_texture_2d(&mut mem(), p))
    }
    /// `constructInstance(descriptor, p)` of `PTextureCubeMapBase`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTextureCubeMapBase__constructInstance @ 0x004c_6be0 => extern "thiscall" fn h1_004c6be0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_texture_cube(&mut mem(), p))
    }
    /// `constructInstance(descriptor, p)` of `PLight`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PLight__constructInstance @ 0x0049_ebc0 => extern "thiscall" fn h1_0049ebc0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_light(&mut mem(), descriptor::LIGHT, p))
    }
    /// `constructInstance(descriptor, p)` of `PAssetReferenceImport`.
    Phyre_PClassDescriptorConcrete_Phyre_PAssetReferenceImport__constructInstance @ 0x0045_dc70 => extern "thiscall" fn h1_0045dc70(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_asset_import(&mut mem(), p))
    }
    /// `PConstantBuffer`'s destructor (`this`) -> the last call's `eax`.
    Phyre_PRendering_PConstantBuffer_dtor_PConstantBuffer @ 0x0049_fa10 => extern "thiscall" fn h1_0049fa10(this: u32) -> u32 {
        let calls = const { descriptor::three_of(descriptor::DESTROY_CHECKED, 0x004a_0560) };
        basic::bridge::dtor_checked(&mut mem(), calls, this)
    }
    /// `0x00433900(this)`: releases an array of 0x40-byte elements.
    PArray_release40_00433900 @ 0x0043_3900 => extern "thiscall" fn h1_00433900(this: u32) {
        basic::bridge::release_array40(&mut mem(), this);
    }
    /// `0x004d57d0(this, parameter, src)` -> 0, 1 or 3.
    PParameterBuffer_writeParameter_004d57d0 @ 0x004d_57d0 => extern "thiscall" fn h1_004d57d0(this: u32, parameter: u32, src: u32) -> u32 {
        basic::bridge::write_parameter(&mut mem(), this, parameter, src)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    PClassBind_withTypedMember_0049fe10 @ 0x0049_fe10 => extern "C" fn h1_0049fe10() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x0049_fe10) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    Phyre_PRendering_PShaderPassStateBase_Bind @ 0x004b_6870 => extern "C" fn h1_004b6870() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x004b_6870) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    PClassBind_withTypedMember_004a1b40 @ 0x004a_1b40 => extern "C" fn h1_004a1b40() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x004a_1b40) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    PClassBind_withTypedMember_004d65b0 @ 0x004d_65b0 => extern "C" fn h1_004d65b0() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x004d_65b0) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    PClassBind_withTypedMember_004a1be0 @ 0x004a_1be0 => extern "C" fn h1_004a1be0() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x004a_1be0) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `T::Bind()` with a typed member -> `addToNamespace`'s `eax`.
    PClassBind_withTypedMember_004a1c80 @ 0x004a_1c80 => extern "C" fn h1_004a1c80() -> u32 {
        let (d, k) = const { registry::bind_typed_of(0x004a_1c80) };
        registry::bridge::bind_member_typed(&mut mem(), d, k)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PGeometry_PSkinBoneRemap_2 @ 0x0048_7d80 => extern "C" fn h1_00487d80(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0048_7d80) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocation_2 @ 0x004b_6910 => extern "C" fn h1_004b6910(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004b_6910) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PRendering_PShaderParameterCaptureBufferLocationType_2 @ 0x004b_6b20 => extern "C" fn h1_004b6b20(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x004b_6b20) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `T::Bind()` with two members -> `addToNamespace`'s `eax`.
    PClassBind_withTwoMembers_004a9de0 @ 0x004a_9de0 => extern "C" fn h1_004a9de0() -> u32 {
        registry::bridge::bind_two_members(&mut mem(), const { registry::bind_two_of(0x004a_9de0) })
    }
    /// `T::Bind()` with two members -> `addToNamespace`'s `eax`.
    PClassBind_withTwoMembers_004aa130 @ 0x004a_a130 => extern "C" fn h1_004aa130() -> u32 {
        registry::bridge::bind_two_members(&mut mem(), const { registry::bind_two_of(0x004a_a130) })
    }
    /// `T::Bind()` with two members -> `addToNamespace`'s `eax`.
    PClassBind_withTwoMembers_004aa410 @ 0x004a_a410 => extern "C" fn h1_004aa410() -> u32 {
        registry::bridge::bind_two_members(&mut mem(), const { registry::bind_two_of(0x004a_a410) })
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_int_4 @ 0x0049_0950 => extern "C" fn h1_00490950(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_0950) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `GetType<T>(name)` of an array type -> its descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PArray_Phyre_PMatrix4_4 @ 0x0049_0b60 => extern "C" fn h1_00490b60(name: u32) -> u32 {
        let k = const { registry::array_type_of(0x0049_0b60) };
        registry::bridge::array_type(&mut mem(), &k, name)
    }
    /// `PWorld::addCluster(world, cluster)`.
    Phyre_PWorld_addCluster @ 0x0044_3250 => extern "thiscall" fn h1_00443250(world: u32, cluster: u32) {
        basic::bridge::add_cluster(&mut mem(), world, cluster);
    }
    /// `PWorld::addInstanceList(world, list)`.
    Phyre_PWorld_addInstanceList @ 0x0044_3320 => extern "thiscall" fn h1_00443320(world: u32, list: u32) {
        basic::bridge::add_instance_list(&mut mem(), world, list);
    }
    /// `PWorld::removeCluster(world, cluster)`.
    Phyre_PWorld_removeCluster @ 0x0044_3b70 => extern "thiscall" fn h1_00443b70(world: u32, cluster: u32) {
        basic::bridge::remove_cluster(&mut mem(), world, cluster);
    }
    /// `PWorld::removeInstanceList(world, list)` -> 0.
    Phyre_PWorld_removeInstanceList @ 0x0044_3c20 => extern "thiscall" fn h1_00443c20(world: u32, list: u32) -> u32 {
        basic::bridge::remove_instance_list(&mut mem(), world, list)
    }
    /// `0x004bbe40(this)`: releases an array of `PShaderPass`es.
    PArray_releaseShaderPasses_004bbe40 @ 0x004b_be40 => extern "thiscall" fn h1_004bbe40(this: u32) {
        basic::bridge::release_shader_pass_array(&mut mem(), this);
    }
    /// `0x00433700(this)` -> `this`.
    PhyreCtor_zero56_00433700 @ 0x0043_3700 => extern "thiscall" fn h1_00433700(this: u32) -> u32 {
        basic::bridge::clear_56(&mut mem(), this)
    }
    /// `PRenderTargetBase::getViewportSize(this, width, height)` -> 0 or 0x15.
    Phyre_PRendering_PRenderTargetBase_getViewportSize @ 0x004c_82b0 => extern "thiscall" fn h1_004c82b0(this: u32, width: u32, height: u32) -> u32 {
        basic::bridge::viewport_size(&mut mem(), this, width, height)
    }
    /// `0x00498500(ctx, _, segment, blocks, indirect, extra)` -> the draw's `eax`.
    PMeshDraw_drawSegment_00498500 @ 0x0049_8500 => extern "stdcall" fn h1_00498500(ctx: u32, _unused: u32, segment: u32, blocks: u32, indirect: u32, extra: u32) -> u32 {
        basic::bridge::draw_segment(&mut mem(), ctx, segment, blocks, indirect, extra)
    }
    /// `constructInstance(descriptor, p)` of `PTexture3DBase`.
    Phyre_PClassDescriptorConcrete_Phyre_PRendering_PTexture3DBase__constructInstance @ 0x004c_53b0 => extern "thiscall" fn h1_004c53b0(_descriptor: u32, p: u32) -> u8 {
        u8::from(descriptor::bridge::construct_texture_3d(&mut mem(), p))
    }
    /// `0x004d7560(this, a, b)` -> the stored function's `eax`.
    PClassDescriptorHeader_callStored_004d7560 @ 0x004d_7560 => extern "thiscall" fn h1_004d7560(this: u32, a: u32, b: u32) -> u32 {
        basic::bridge::call_stored(&mut mem(), this, a, b)
    }
    /// `0x004649d0()` -> the renderer context.
    PRendererContext_instance_004649d0 @ 0x0046_49d0 => extern "C" fn h1_004649d0() -> u32 {
        basic::bridge::renderer_context(&mut mem())
    }
    // round 2: the Bind functions with several members (h1_bind)
    /// `Phyre_PRendering_PTextureCubeMapBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PTextureCubeMapBase_Bind @ 0x004c_6500 => extern "C" fn h1_004c6500() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_6500) })
    }
    /// `Phyre_PCameraPerspective_Bind()`: registers its class descriptor and members.
    Phyre_PCameraPerspective_Bind @ 0x0045_6540 => extern "C" fn h1_00456540() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_6540) })
    }
    /// `Phyre_PRendering_PShadowCaster_PShadowSplit_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShadowCaster_PShadowSplit_Bind @ 0x004c_ad80 => extern "C" fn h1_004cad80() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_ad80) })
    }
    /// `Phyre_PRendering_PMaterialSwitch_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PMaterialSwitch_Bind @ 0x004d_80a0 => extern "C" fn h1_004d80a0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004d_80a0) })
    }
    /// `Phyre_PGeometry_PSkinBoneRemap_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PSkinBoneRemap_Bind @ 0x0048_7ca0 => extern "C" fn h1_00487ca0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_7ca0) })
    }
    /// `Phyre_PRendering_PTexture2DBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PTexture2DBase_Bind @ 0x004c_3400 => extern "C" fn h1_004c3400() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_3400) })
    }
    /// `Phyre_PRendering_PRenderTargetBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PRenderTargetBase_Bind @ 0x004c_7b70 => extern "C" fn h1_004c7b70() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_7b70) })
    }
    /// `Phyre_PWorld_Bind()`: registers its class descriptor and members.
    Phyre_PWorld_Bind @ 0x0044_2e30 => extern "C" fn h1_00442e30() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0044_2e30) })
    }
    /// `Phyre_PRendering_PShaderPassParameterLocationTypesBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderPassParameterLocationTypesBase_Bind @ 0x004b_6680 => extern "C" fn h1_004b6680() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_6680) })
    }
    /// `Phyre_PRendering_PShaderPassParameterLocationTypes_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderPassParameterLocationTypes_Bind @ 0x004b_6580 => extern "C" fn h1_004b6580() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_6580) })
    }
    /// `Phyre_PRendering_PShaderPassParameterLocationTypesConstantBuffer_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderPassParameterLocationTypesConstantBuffer_Bind @ 0x004b_6770 => extern "C" fn h1_004b6770() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_6770) })
    }
    /// `Phyre_PRendering_PNodeContext_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PNodeContext_Bind @ 0x004c_f7c0 => extern "C" fn h1_004cf7c0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_f7c0) })
    }
    /// `Phyre_PRendering_PShaderParameterCaptureBufferRWByteAddressBuffer_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderParameterCaptureBufferRWByteAddressBuffer_Bind @ 0x004a_a000 => extern "C" fn h1_004aa000() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_a000) })
    }
    /// `Phyre_PRendering_PShaderParameterCaptureBufferTextureBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderParameterCaptureBufferTextureBase_Bind @ 0x004a_a630 => extern "C" fn h1_004aa630() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_a630) })
    }
    /// `Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderParameterCaptureBufferLocationTypeConstantBuffer_Bind @ 0x004a_1d20 => extern "C" fn h1_004a1d20() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_1d20) })
    }
    /// `Phyre_PRendering_PTexture3DBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PTexture3DBase_Bind @ 0x004c_4c40 => extern "C" fn h1_004c4c40() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_4c40) })
    }
    /// `Phyre_PRendering_PParameterBuffer_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PParameterBuffer_Bind @ 0x004d_6470 => extern "C" fn h1_004d6470() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004d_6470) })
    }
    /// `Phyre_PCameraOrthographic_Bind()`: registers its class descriptor and members.
    Phyre_PCameraOrthographic_Bind @ 0x0045_55e0 => extern "C" fn h1_004555e0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_55e0) })
    }
    /// `Phyre_PGeometry_PVertexStream_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PVertexStream_Bind @ 0x0048_0510 => extern "C" fn h1_00480510() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_0510) })
    }
    /// `Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PDynamicMesh_PDynamicSegmentDesc_Bind @ 0x0049_70e0 => extern "C" fn h1_004970e0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_70e0) })
    }
    /// `Phyre_PGeometry_PSkeletonJointBounds_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PSkeletonJointBounds_Bind @ 0x0049_07c0 => extern "C" fn h1_004907c0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_07c0) })
    }
    /// `Phyre_PRendering_PShaderProgramParamsAndStreams_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderProgramParamsAndStreams_Bind @ 0x004a_a8c0 => extern "C" fn h1_004aa8c0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_a8c0) })
    }
    /// `Phyre_PGeometry_PIndexDataBlockBase_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PIndexDataBlockBase_Bind @ 0x0048_5470 => extern "C" fn h1_00485470() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_5470) })
    }
    /// `Phyre_PRendering_PShaderSource_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderSource_Bind @ 0x004a_aa70 => extern "C" fn h1_004aaa70() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_aa70) })
    }
    /// `Phyre_PGeometry_PDataBlockBase_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PDataBlockBase_Bind @ 0x0048_1f80 => extern "C" fn h1_00481f80() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_1f80) })
    }
    /// `Phyre_PRendering_PShadowCaster_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShadowCaster_Bind @ 0x004c_aba0 => extern "C" fn h1_004caba0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_aba0) })
    }
    /// `Phyre_PCameraProjection_Bind()`: registers its class descriptor and members.
    Phyre_PCameraProjection_Bind @ 0x0045_4070 => extern "C" fn h1_00454070() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_4070) })
    }
    /// `Phyre_PGeometry_PDynamicMesh_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PDynamicMesh_Bind @ 0x0049_6ee0 => extern "C" fn h1_00496ee0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_6ee0) })
    }
    /// `Phyre_PRendering_PMaterial_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PMaterial_Bind @ 0x004d_7e90 => extern "C" fn h1_004d7e90() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004d_7e90) })
    }
    /// `Phyre_PRendering_PShaderParameterDefinition_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderParameterDefinition_Bind @ 0x004a_1e50 => extern "C" fn h1_004a1e50() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_1e50) })
    }
    /// `Phyre_PRendering_PShaderStreamDefinition_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderStreamDefinition_Bind @ 0x004a_ac50 => extern "C" fn h1_004aac50() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004a_ac50) })
    }
    /// `Phyre_PRendering_PShaderPassBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShaderPassBase_Bind @ 0x004b_6350 => extern "C" fn h1_004b6350() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_6350) })
    }
    /// `Phyre_PGeometry_PDynamicDataBlock_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PDynamicDataBlock_Bind @ 0x0048_4300 => extern "C" fn h1_00484300() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_4300) })
    }
    /// `Phyre_PRendering_PTextureCommonBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PTextureCommonBase_Bind @ 0x004c_1fb0 => extern "C" fn h1_004c1fb0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004c_1fb0) })
    }
    /// `Phyre_PGeometry_PMaterialSet_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PMaterialSet_Bind @ 0x0048_aa00 => extern "C" fn h1_0048aa00() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_aa00) })
    }
    /// `Phyre_PRendering_PShader_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PShader_Bind @ 0x004b_c360 => extern "C" fn h1_004bc360() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_c360) })
    }
    /// `Phyre_PGeometry_PShape_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PShape_Bind @ 0x0049_a180 => extern "C" fn h1_0049a180() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_a180) })
    }
    /// `Phyre_PSerialization_PBinary_PInternal_PInstanceListHeader_Bind()`: registers its class descriptor and members.
    Phyre_PSerialization_PBinary_PInternal_PInstanceListHeader_Bind @ 0x0046_a340 => extern "C" fn h1_0046a340() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0046_a340) })
    }
    /// `Phyre_PGeometry_PMeshSegmentBase_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PMeshSegmentBase_Bind @ 0x0048_7990 => extern "C" fn h1_00487990() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0048_7990) })
    }
    /// `Phyre_PWorldMatrix_Bind()`: registers its class descriptor and members.
    Phyre_PWorldMatrix_Bind @ 0x0045_14a0 => extern "C" fn h1_004514a0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_14a0) })
    }
    /// `Phyre_PInstancesComponent_Bind()`: registers its class descriptor and members.
    Phyre_PInstancesComponent_Bind @ 0x0045_80c0 => extern "C" fn h1_004580c0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_80c0) })
    }
    /// `Phyre_PGeometry_PMesh_Bind()`: registers its class descriptor and members.
    Phyre_PGeometry_PMesh_Bind @ 0x0049_0250 => extern "C" fn h1_00490250() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_0250) })
    }
    /// `Phyre_PCamera_Bind()`: registers its class descriptor and members.
    Phyre_PCamera_Bind @ 0x0045_37d0 => extern "C" fn h1_004537d0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0045_37d0) })
    }
    /// `Phyre_PRendering_PLight_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PLight_Bind @ 0x0049_d8c0 => extern "C" fn h1_0049d8c0() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x0049_d8c0) })
    }
    /// `Phyre_PRendering_PSamplerStateBase_Bind()`: registers its class descriptor and members.
    Phyre_PRendering_PSamplerStateBase_Bind @ 0x004b_ed60 => extern "C" fn h1_004bed60() -> u32 {
        h1_bind::bridge::bind(&mut mem(), const { h1_bind::bind_of(0x004b_ed60) })
    }
}
