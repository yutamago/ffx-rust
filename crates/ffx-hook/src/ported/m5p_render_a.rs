//! `ffx_prender::m5p_render_a*`: M5 stage 1, `PhyreEngine` list
//! `render-a` (render resources, shaders, buffers, sprites and geometry
//! utilities the class-gated model reaches; none ran in a coverage run).
//!
//! Threads: the main thread (atexit teardown, the loader's fix-ups) and
//! the loader thread; the shims keep no state.

use super::{mem, ported};
use ffx_prender::m5p_render_a::{self as ma, bridge as a};

ported! {
    // ---- batch 1 ----
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PVertexStream__dtor @ 0x0048_0430 => extern "thiscall" fn fortype_dtor_00480430(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_0430) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PVertexStream__dtor @ 0x0048_1880 => extern "thiscall" fn fortype_dtor_00481880(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_1880) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDataBlock__dtor @ 0x0048_1890 => extern "thiscall" fn fortype_dtor_00481890(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_1890) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDataBlockBase__dtor @ 0x0048_18a0 => extern "thiscall" fn fortype_dtor_004818a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_18a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDynamicDataBlock__dtor @ 0x0048_4150 => extern "thiscall" fn fortype_dtor_00484150(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_4150) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PIndexDataBlock__dtor @ 0x0048_5030 => extern "thiscall" fn fortype_dtor_00485030(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_5030) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PIndexDataBlockBase__dtor @ 0x0048_5040 => extern "thiscall" fn fortype_dtor_00485040(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_5040) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PSkinBoneRemap__dtor @ 0x0048_7270 => extern "thiscall" fn fortype_dtor_00487270(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_7270) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSharray_PMaterial__dtor @ 0x0048_a5d0 => extern "thiscall" fn fortype_dtor_0048a5d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_a5d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PMaterialSet__dtor @ 0x0048_a5e0 => extern "thiscall" fn fortype_dtor_0048a5e0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_a5e0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PInt32__dtor @ 0x0048_f380 => extern "thiscall" fn fortype_dtor_0048f380(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_f380) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PMatrix4__dtor @ 0x0048_f390 => extern "thiscall" fn fortype_dtor_0048f390(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_f390) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PMeshSegment__dtor @ 0x0048_f3a0 => extern "thiscall" fn fortype_dtor_0048f3a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_f3a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PSkeletonJointBounds__dtor @ 0x0048_f3b0 => extern "thiscall" fn fortype_dtor_0048f3b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_f3b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PString__dtor @ 0x0048_f3c0 => extern "thiscall" fn fortype_dtor_0048f3c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0048_f3c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PDynamicSegmentDesc__dtor @ 0x0049_69c0 => extern "thiscall" fn fortype_dtor_004969c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0049_69c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDynamicMesh__dtor @ 0x0049_69d0 => extern "thiscall" fn fortype_dtor_004969d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0049_69d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDynamicSegmentDesc__dtor @ 0x0049_69e0 => extern "thiscall" fn fortype_dtor_004969e0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0049_69e0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PConstantBuffer__dtor @ 0x0049_f9f0 => extern "thiscall" fn fortype_dtor_0049f9f0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0049_f9f0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PConstantBufferBase__dtor @ 0x0049_fa00 => extern "thiscall" fn fortype_dtor_0049fa00(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0049_fa00) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderParameterDefinition__dtor @ 0x004a_8640 => extern "thiscall" fn fortype_dtor_004a8640(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8640) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderStreamDefinition__dtor @ 0x004a_8650 => extern "thiscall" fn fortype_dtor_004a8650(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8650) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderComputeProgram__dtor @ 0x004a_8660 => extern "thiscall" fn fortype_dtor_004a8660(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8660) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderFragmentProgram__dtor @ 0x004a_8670 => extern "thiscall" fn fortype_dtor_004a8670(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8670) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderGeometryProgram__dtor @ 0x004a_8680 => extern "thiscall" fn fortype_dtor_004a8680(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8680) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferByteAddressBuffer__dtor @ 0x004a_8690 => extern "thiscall" fn fortype_dtor_004a8690(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8690) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferDataBlock__dtor @ 0x004a_86a0 => extern "thiscall" fn fortype_dtor_004a86a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferIndexDataBlock__dtor @ 0x004a_86b0 => extern "thiscall" fn fortype_dtor_004a86b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferRWByteAddressBuffer__dtor @ 0x004a_86c0 => extern "thiscall" fn fortype_dtor_004a86c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferRWStructuredBuffer__dtor @ 0x004a_86d0 => extern "thiscall" fn fortype_dtor_004a86d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferRWTexture2D__dtor @ 0x004a_86e0 => extern "thiscall" fn fortype_dtor_004a86e0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86e0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferRWTexture3D__dtor @ 0x004a_86f0 => extern "thiscall" fn fortype_dtor_004a86f0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_86f0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferSampler__dtor @ 0x004a_8700 => extern "thiscall" fn fortype_dtor_004a8700(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8700) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferStream__dtor @ 0x004a_8710 => extern "thiscall" fn fortype_dtor_004a8710(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8710) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferStructuredBuffer__dtor @ 0x004a_8720 => extern "thiscall" fn fortype_dtor_004a8720(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8720) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferTexture2D__dtor @ 0x004a_8730 => extern "thiscall" fn fortype_dtor_004a8730(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8730) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferTexture3D__dtor @ 0x004a_8740 => extern "thiscall" fn fortype_dtor_004a8740(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8740) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferTextureBase__dtor @ 0x004a_8750 => extern "thiscall" fn fortype_dtor_004a8750(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8750) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureBufferTextureCubeMap__dtor @ 0x004a_8760 => extern "thiscall" fn fortype_dtor_004a8760(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8760) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderParameterCaptureConstantBuffer__dtor @ 0x004a_8770 => extern "thiscall" fn fortype_dtor_004a8770(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8770) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderProgramBase__dtor @ 0x004a_8780 => extern "thiscall" fn fortype_dtor_004a8780(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8780) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderProgramParamsAndStreams__dtor @ 0x004a_8790 => extern "thiscall" fn fortype_dtor_004a8790(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_8790) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderSource__dtor @ 0x004a_87a0 => extern "thiscall" fn fortype_dtor_004a87a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_87a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderStreamDefinition__dtor @ 0x004a_87b0 => extern "thiscall" fn fortype_dtor_004a87b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_87b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderVertexProgram__dtor @ 0x004a_87c0 => extern "thiscall" fn fortype_dtor_004a87c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004a_87c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderParameterCaptureBufferLocation__dtor @ 0x004b_5350 => extern "thiscall" fn fortype_dtor_004b5350(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_5350) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderParameterCaptureBufferLocationType__dtor @ 0x004b_5360 => extern "thiscall" fn fortype_dtor_004b5360(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_5360) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderParameterCaptureBufferLocationTypeConstantBuffer__dtor @ 0x004b_5370 => extern "thiscall" fn fortype_dtor_004b5370(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_5370) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPass__dtor @ 0x004b_5380 => extern "thiscall" fn fortype_dtor_004b5380(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_5380) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassBase__dtor @ 0x004b_5390 => extern "thiscall" fn fortype_dtor_004b5390(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_5390) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassParameterLocationTypes__dtor @ 0x004b_53a0 => extern "thiscall" fn fortype_dtor_004b53a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_53a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassParameterLocationTypesBase__dtor @ 0x004b_53b0 => extern "thiscall" fn fortype_dtor_004b53b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_53b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassParameterLocationTypesConstantBuffer__dtor @ 0x004b_53c0 => extern "thiscall" fn fortype_dtor_004b53c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_53c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassStateBase__dtor @ 0x004b_53d0 => extern "thiscall" fn fortype_dtor_004b53d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_53d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShaderPass__dtor @ 0x004b_bf00 => extern "thiscall" fn fortype_dtor_004bbf00(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_bf00) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShader__dtor @ 0x004b_bf10 => extern "thiscall" fn fortype_dtor_004bbf10(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_bf10) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSamplerState__dtor @ 0x004b_e8f0 => extern "thiscall" fn fortype_dtor_004be8f0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004b_e8f0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTextureCommonBase__dtor @ 0x004c_1dd0 => extern "thiscall" fn fortype_dtor_004c1dd0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_1dd0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture2D__dtor @ 0x004c_2ff0 => extern "thiscall" fn fortype_dtor_004c2ff0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_2ff0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture2DBase__dtor @ 0x004c_3000 => extern "thiscall" fn fortype_dtor_004c3000(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_3000) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture3D__dtor @ 0x004c_4810 => extern "thiscall" fn fortype_dtor_004c4810(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_4810) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture3DBase__dtor @ 0x004c_4820 => extern "thiscall" fn fortype_dtor_004c4820(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_4820) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTextureCubeMap__dtor @ 0x004c_60d0 => extern "thiscall" fn fortype_dtor_004c60d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_60d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTextureCubeMapBase__dtor @ 0x004c_60e0 => extern "thiscall" fn fortype_dtor_004c60e0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_60e0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PRenderTarget__dtor @ 0x004c_79b0 => extern "thiscall" fn fortype_dtor_004c79b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_79b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PRenderTargetBase__dtor @ 0x004c_79c0 => extern "thiscall" fn fortype_dtor_004c79c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004c_79c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PParameterBuffer__dtor @ 0x004d_6260 => extern "thiscall" fn fortype_dtor_004d6260(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004d_6260) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PParameterBufferBase__dtor @ 0x004d_6270 => extern "thiscall" fn fortype_dtor_004d6270(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004d_6270) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PMaterial__dtor @ 0x004d_7ba0 => extern "thiscall" fn fortype_dtor_004d7ba0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004d_7ba0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PMaterialSwitch__dtor @ 0x004d_7bb0 => extern "thiscall" fn fortype_dtor_004d7bb0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004d_7bb0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PEffectVariant__dtor @ 0x004e_a270 => extern "thiscall" fn fortype_dtor_004ea270(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a270) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PContextSwitch__dtor @ 0x004e_a280 => extern "thiscall" fn fortype_dtor_004ea280(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a280) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PLightType__dtor @ 0x004e_a290 => extern "thiscall" fn fortype_dtor_004ea290(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a290) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PShadowCasterType__dtor @ 0x004e_a2a0 => extern "thiscall" fn fortype_dtor_004ea2a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a2a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PNodeContext__dtor @ 0x004e_a2b0 => extern "thiscall" fn fortype_dtor_004ea2b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a2b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PEffect__dtor @ 0x004e_a2c0 => extern "thiscall" fn fortype_dtor_004ea2c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004e_a2c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PDynamicMeshInstance__dtor @ 0x004f_a450 => extern "thiscall" fn fortype_dtor_004fa450(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004f_a450) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PModifierNetworkInstance__dtor @ 0x004f_b4a0 => extern "thiscall" fn fortype_dtor_004fb4a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004f_b4a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PModifierNetworkDynamicMeshInstance__dtor @ 0x004f_b4b0 => extern "thiscall" fn fortype_dtor_004fb4b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x004f_b4b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PStructuredBuffer__dtor @ 0x0050_1cb0 => extern "thiscall" fn fortype_dtor_00501cb0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0050_1cb0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PStructuredBufferBase__dtor @ 0x0050_1cc0 => extern "thiscall" fn fortype_dtor_00501cc0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0050_1cc0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PIndirectArgsBuffer__dtor @ 0x0050_3230 => extern "thiscall" fn fortype_dtor_00503230(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0050_3230) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PIndirectArgsBufferBase__dtor @ 0x0050_3240 => extern "thiscall" fn fortype_dtor_00503240(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0050_3240) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PNode__dtor @ 0x0050_4f70 => extern "thiscall" fn fortype_dtor_00504f70(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0050_4f70) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PSubTextureInfo__dtor @ 0x0056_7440 => extern "thiscall" fn fortype_dtor_00567440(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7440) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSpriteAnimationInfo__dtor @ 0x0056_7450 => extern "thiscall" fn fortype_dtor_00567450(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7450) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSpriteAnimationInfoChar__dtor @ 0x0056_7460 => extern "thiscall" fn fortype_dtor_00567460(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7460) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSpriteAttributes__dtor @ 0x0056_7470 => extern "thiscall" fn fortype_dtor_00567470(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7470) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSpriteCollection__dtor @ 0x0056_7480 => extern "thiscall" fn fortype_dtor_00567480(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7480) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSubTextureInfo__dtor @ 0x0056_7490 => extern "thiscall" fn fortype_dtor_00567490(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_7490) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTextureAtlasInfo__dtor @ 0x0056_74a0 => extern "thiscall" fn fortype_dtor_005674a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_74a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSpriteAnimationInfoInstance__dtor @ 0x0056_f8d0 => extern "thiscall" fn fortype_dtor_0056f8d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0056_f8d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PArray_PStreamInputDescD3D11__dtor @ 0x0058_4800 => extern "thiscall" fn fortype_dtor_00584800(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4800) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PConstantBufferD3D11__dtor @ 0x0058_4810 => extern "thiscall" fn fortype_dtor_00584810(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4810) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PIndirectArgsBufferD3D11__dtor @ 0x0058_4820 => extern "thiscall" fn fortype_dtor_00584820(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4820) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PRenderTargetD3D11__dtor @ 0x0058_4830 => extern "thiscall" fn fortype_dtor_00584830(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4830) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PSamplerStateD3D11__dtor @ 0x0058_4840 => extern "thiscall" fn fortype_dtor_00584840(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4840) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderComputeProgramD3D11__dtor @ 0x0058_4850 => extern "thiscall" fn fortype_dtor_00584850(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4850) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderFragmentProgramD3D11__dtor @ 0x0058_4860 => extern "thiscall" fn fortype_dtor_00584860(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4860) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderGeometryProgramD3D11__dtor @ 0x0058_4870 => extern "thiscall" fn fortype_dtor_00584870(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4870) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassD3D11__dtor @ 0x0058_4880 => extern "thiscall" fn fortype_dtor_00584880(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4880) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderPassStateD3D11__dtor @ 0x0058_4890 => extern "thiscall" fn fortype_dtor_00584890(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4890) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderProgramD3D11__dtor @ 0x0058_48a0 => extern "thiscall" fn fortype_dtor_005848a0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48a0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PShaderVertexProgramD3D11__dtor @ 0x0058_48b0 => extern "thiscall" fn fortype_dtor_005848b0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48b0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PStreamInputDescD3D11__dtor @ 0x0058_48c0 => extern "thiscall" fn fortype_dtor_005848c0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48c0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PStreamInputLayoutD3D11__dtor @ 0x0058_48d0 => extern "thiscall" fn fortype_dtor_005848d0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48d0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PStructuredBufferD3D11__dtor @ 0x0058_48e0 => extern "thiscall" fn fortype_dtor_005848e0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48e0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture2DD3D11__dtor @ 0x0058_48f0 => extern "thiscall" fn fortype_dtor_005848f0(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_48f0) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTexture3DD3D11__dtor @ 0x0058_4900 => extern "thiscall" fn fortype_dtor_00584900(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4900) })
    }
    /// `PClassDescriptorForType<T>` destructor body (`m5p_render_a::FORTYPE_DTORS`).
    Phyre_PClassDescriptorForType_PTextureCubeMapD3D11__dtor @ 0x0058_4910 => extern "thiscall" fn fortype_dtor_00584910(this: u32) -> u32 {
        a::fortype_dtor(&mut mem(), this, const { ma::fortype_vtable(0x0058_4910) })
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_004ea010 @ 0x004e_a010 => extern "thiscall" fn reset_004ea010(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_004ea040 @ 0x004e_a040 => extern "thiscall" fn reset_004ea040(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_004ea070 @ 0x004e_a070 => extern "thiscall" fn reset_004ea070(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_004ea0a0 @ 0x004e_a0a0 => extern "thiscall" fn reset_004ea0a0(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_005844f0 @ 0x0058_44f0 => extern "thiscall" fn reset_005844f0(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset (`RESET_OWNED`).
    PArray_Reset_00584520 @ 0x0058_4520 => extern "thiscall" fn reset_00584520(this: u32) -> u32 {
        a::reset_owned(&mut mem(), this, 0x0)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_004817d0 @ 0x0048_17d0 => extern "thiscall" fn reset_004817d0(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x0)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_004db6e0 @ 0x004d_b6e0 => extern "thiscall" fn reset_004db6e0(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x0)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_005672d0 @ 0x0056_72d0 => extern "thiscall" fn reset_005672d0(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x0)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_00584590 @ 0x0058_4590 => extern "thiscall" fn reset_00584590(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x0)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_00433d10 @ 0x0043_3d10 => extern "thiscall" fn reset_00433d10(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x8)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_00481910 @ 0x0048_1910 => extern "thiscall" fn reset_00481910(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x8)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_004b53f0 @ 0x004b_53f0 => extern "thiscall" fn reset_004b53f0(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x10)
    }
    /// Owned-array reset, null block skipped (`RESET_NONNULL`).
    PArray_Reset_004b5450 @ 0x004b_5450 => extern "thiscall" fn reset_004b5450(this: u32) -> u32 {
        a::reset_owned_nonnull(&mut mem(), this, 0x10)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00433ab0 @ 0x0043_3ab0 => extern "thiscall" fn reset_00433ab0(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, false)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_004872b0 @ 0x0048_72b0 => extern "thiscall" fn reset_004872b0(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_0048f400 @ 0x0048_f400 => extern "thiscall" fn reset_0048f400(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00584c70 @ 0x0058_4c70 => extern "thiscall" fn reset_00584c70(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00584cb0 @ 0x0058_4cb0 => extern "thiscall" fn reset_00584cb0(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00584cf0 @ 0x0058_4cf0 => extern "thiscall" fn reset_00584cf0(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00584d30 @ 0x0058_4d30 => extern "thiscall" fn reset_00584d30(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// `PSharray` reset (`RESET_SHARED`).
    PSharray_Reset_00584d70 @ 0x0058_4d70 => extern "thiscall" fn reset_00584d70(this: u32) -> u32 {
        a::reset_shared(&mut mem(), this, true)
    }
    /// Owned-array reset through the element release (`RESET_RELEASE`).
    PArray_ResetRelease_00433a30 @ 0x0043_3a30 => extern "thiscall" fn reset_release_00433a30(this: u32) -> u32 {
        a::reset_release(&mut mem(), this, 0x0043_4360)
    }
    /// Owned-array reset through the element release (`RESET_RELEASE`).
    PArray_ResetRelease_004a82c0 @ 0x004a_82c0 => extern "thiscall" fn reset_release_004a82c0(this: u32) -> u32 {
        a::reset_release(&mut mem(), this, 0x004a_b4d0)
    }
    /// Owned-array reset through the element release (`RESET_RELEASE`).
    PArray_ResetRelease_004db770 @ 0x004d_b770 => extern "thiscall" fn reset_release_004db770(this: u32) -> u32 {
        a::reset_release(&mut mem(), this, 0x004d_d770)
    }
    /// Owned-array reset through the element release (`RESET_RELEASE`).
    PArray_ResetRelease_00585770 @ 0x0058_5770 => extern "thiscall" fn reset_release_00585770(this: u32) -> u32 {
        a::reset_release(&mut mem(), this, 0x0058_8190)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004e4160 @ 0x004e_4160 => extern "C" fn word_array_alloc_004e4160(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004eb0a0 @ 0x004e_b0a0 => extern "C" fn word_array_alloc_004eb0a0(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004eb110 @ 0x004e_b110 => extern "C" fn word_array_alloc_004eb110(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004eb180 @ 0x004e_b180 => extern "C" fn word_array_alloc_004eb180(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004eb1f0 @ 0x004e_b1f0 => extern "C" fn word_array_alloc_004eb1f0(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(n, src, count)` -> the block (`WORD_ARRAY_ALLOCS`).
    WordArray_alloc_004fb850 @ 0x004f_b850 => extern "C" fn word_array_alloc_004fb850(n: u32, src: u32, count: u32) -> u32 {
        a::word_array_alloc(&mut mem(), n, src, count)
    }
    /// `(dst, src, count)` (`COPY_ELEMENTS`).
    PhyreCopyElements20_00480b60 @ 0x0048_0b60 => extern "C" fn copy_elements_00480b60(dst: u32, src: u32, count: u32) {
        a::copy_elements(&mut mem(), dst, src, count, 5);
    }
    /// `(dst, src, count)` (`COPY_ELEMENTS`).
    PhyreCopyElements20_00484a30 @ 0x0048_4a30 => extern "C" fn copy_elements_00484a30(dst: u32, src: u32, count: u32) {
        a::copy_elements(&mut mem(), dst, src, count, 5);
    }
    /// `(dst, src, count)` (`COPY_ELEMENTS`).
    PhyreCopyElements20_004d89e0 @ 0x004d_89e0 => extern "C" fn copy_elements_004d89e0(dst: u32, src: u32, count: u32) {
        a::copy_elements(&mut mem(), dst, src, count, 5);
    }
    /// `(dst, src, count)` (`COPY_ELEMENTS`).
    PhyreCopyElements16_00495bc0 @ 0x0049_5bc0 => extern "C" fn copy_elements_00495bc0(dst: u32, src: u32, count: u32) {
        a::copy_elements(&mut mem(), dst, src, count, 4);
    }
    /// `(dst, src, count)` (`COPY_ELEMENTS`).
    PhyreCopyElements16_005465f0 @ 0x0054_65f0 => extern "C" fn copy_elements_005465f0(dst: u32, src: u32, count: u32) {
        a::copy_elements(&mut mem(), dst, src, count, 4);
    }
    /// `(this; flags)` -> this (`UTILITY_DELETES`).
    PUtility_scalarDeletingDtor_00432250 @ 0x0043_2250 => extern "thiscall" fn utility_delete_00432250(this: u32, flags: u32) -> u32 {
        a::utility_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`UTILITY_DELETES`).
    PUtility_scalarDeletingDtor_004325d0 @ 0x0043_25d0 => extern "thiscall" fn utility_delete_004325d0(this: u32, flags: u32) -> u32 {
        a::utility_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`UTILITY_DELETES`).
    PUtility_scalarDeletingDtor_00503e70 @ 0x0050_3e70 => extern "thiscall" fn utility_delete_00503e70(this: u32, flags: u32) -> u32 {
        a::utility_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d34d0 @ 0x004d_34d0 => extern "thiscall" fn context_switch_delete_004d34d0(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d3510 @ 0x004d_3510 => extern "thiscall" fn context_switch_delete_004d3510(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d3550 @ 0x004d_3550 => extern "thiscall" fn context_switch_delete_004d3550(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d3590 @ 0x004d_3590 => extern "thiscall" fn context_switch_delete_004d3590(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d35d0 @ 0x004d_35d0 => extern "thiscall" fn context_switch_delete_004d35d0(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d3610 @ 0x004d_3610 => extern "thiscall" fn context_switch_delete_004d3610(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this (`CONTEXT_SWITCH_DELETES`).
    PContextSwitch_scalarDeletingDtor_004d3650 @ 0x004d_3650 => extern "thiscall" fn context_switch_delete_004d3650(this: u32, flags: u32) -> u32 {
        a::context_switch_delete(&mut mem(), this, flags)
    }
    /// `(item)` -> 0 (`LIST_UNLINKS`).
    PContextSwitch_unlink_00432760 @ 0x0043_2760 => extern "C" fn list_unlink_00432760(item: u32) -> u32 {
        a::list_unlink(&mut mem(), item, 0x00ca_4e18, 8)
    }
    /// `(item)` -> 0 (`LIST_UNLINKS`).
    List_unlink_004327b0 @ 0x0043_27b0 => extern "C" fn list_unlink_004327b0(item: u32) -> u32 {
        a::list_unlink(&mut mem(), item, 0x00ca_54f8, 4)
    }
    /// `(item)` -> 0 (`LIST_UNLINKS`).
    List_unlink_005c9930 @ 0x005c_9930 => extern "C" fn list_unlink_005c9930(item: u32) -> u32 {
        a::list_unlink(&mut mem(), item, 0x00cb_d348, 4)
    }
    /// `(data, count)`.
    PhyreZeroElements20_00560df0 @ 0x0056_0df0 => extern "C" fn zero_elements20_00560df0(data: u32, count: u32) {
        a::zero_elements20(&mut mem(), data, count);
    }
    /// `(data, count)`.
    PhyreZeroElements20_00560e40 @ 0x0056_0e40 => extern "C" fn zero_elements20_00560e40(data: u32, count: u32) {
        a::zero_elements20(&mut mem(), data, count);
    }
    /// `(this)`: two half destructors.
    PRedBlackTreePair_dtor_005849d0 @ 0x0058_49d0 => extern "thiscall" fn tree_pair_dtor_005849d0(this: u32) -> u32 {
        a::tree_pair_dtor(&mut mem(), this)
    }
    /// `(this)`: two half destructors.
    PRedBlackTreePair_dtor_00584a30 @ 0x0058_4a30 => extern "thiscall" fn tree_pair_dtor_00584a30(this: u32) -> u32 {
        a::tree_pair_dtor(&mut mem(), this)
    }
    /// `(this; buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004e7900 @ 0x004e_7900 => extern "thiscall" fn set_external_004e7900(this: u32, buffer: u32, count: u32) -> u32 {
        a::set_external(&mut mem(), this, buffer, count, false)
    }
    /// `(this; buffer, count)` -> 0.
    Phyre_PArray_setToExternalBuffer_004e15a0 @ 0x004e_15a0 => extern "thiscall" fn set_external_004e15a0(this: u32, buffer: u32, count: u32) -> u32 {
        a::set_external(&mut mem(), this, buffer, count, true)
    }
    /// `(this; &key)` -> node or 0.
    Phyre_PRedBlackTree_find_004da4e0 @ 0x004d_a4e0 => extern "thiscall" fn tree_find_004da4e0(this: u32, key: u32) -> u32 {
        a::tree_find(&mut mem(), this, key)
    }
    /// `(this)` -> this.
    Phyre_PPair_init_004d7b60 @ 0x004d_7b60 => extern "thiscall" fn pair_init_004d7b60(this: u32) -> u32 {
        a::pair_init(&mut mem(), this)
    }
    /// `(sp, out, len)` -> 0.
    StringWithLength_004dec30 @ 0x004d_ec30 => extern "C" fn string_with_length_004dec30(sp: u32, out: u32, len: u32) -> u32 {
        a::string_with_length(&mut mem(), sp, out, len)
    }
    /// `(this)`: the vtable.
    Phyre_PIChildClassVisitor_dtor_004d6290 @ 0x004d_6290 => extern "thiscall" fn visitor_dtor_004d6290(this: u32) {
        a::store_vtable(&mut mem(), this, ma::CHILD_VISITOR_VTABLE);
    }
    /// `(this)`: the vtable.
    Phyre_PAnimation_PBufferAcceptance_dtor_00616140 @ 0x0061_6140 => extern "thiscall" fn buffer_acceptance_dtor_00616140(this: u32) {
        a::store_vtable(&mut mem(), this, ma::BUFFER_ACCEPTANCE_VTABLE);
    }
    /// `(this; flags)` -> this.
    Phyre_PIChildClassVisitor_scalarDeletingDtor_004d63f0 @ 0x004d_63f0 => extern "thiscall" fn visitor_delete_004d63f0(this: u32, flags: u32) -> u32 {
        a::visitor_delete(&mut mem(), this, flags)
    }
    /// `(this; flags)` -> this.
    Phyre_PSprite_PUtilitySprite_scalarDeletingDtor @ 0x0043_40a0 => extern "thiscall" fn utility_sprite_delete(this: u32, flags: u32) -> u32 {
        a::delete_with(&mut mem(), this, flags, 0x0043_3dd0, ma::OPERATOR_DELETE)
    }
    /// `(this; flags)` -> this.
    Phyre_PGeometry_PDynamicMesh_scalarDeletingDtor @ 0x0049_6d90 => extern "thiscall" fn dynamic_mesh_delete(this: u32, flags: u32) -> u32 {
        a::delete_with(&mut mem(), this, flags, 0x0049_69f0, ma::PHYRE_FREE)
    }
    /// `(this)` -> 0.
    Phyre_PSprite_PUtilitySprite_terminate @ 0x0043_5090 => extern "thiscall" fn utility_sprite_terminate(this: u32) -> u32 {
        a::sprite_terminate(&mut mem(), this)
    }
    /// `(this)`: `jmp Phyre_PClusterBase_clear`.
    RenderTarget_release_00440ea0 @ 0x0044_0ea0 => extern "thiscall" fn cluster_clear_forward(this: u32) -> u32 {
        a::forward0(&mut mem(), this, ma::CLUSTER_CLEAR)
    }
    /// `(this)`: `PRecursiveLock_release(*this)`.
    PRecursiveLock_releaseHeld_00463e60 @ 0x0046_3e60 => extern "thiscall" fn release_held_lock(this: u32) -> u32 {
        a::release_held_lock(&mut mem(), this)
    }
    /// `(item)`: `jmp List_unlink_005c9930`.
    List_unlinkForward_005c9920 @ 0x005c_9920 => extern "C" fn list_unlink_forward(item: u32) -> u32 {
        a::forward_cdecl1(&mut mem(), 0x005c_9930, item)
    }
    /// `(this; arg)`: tail jump.
    PString_AssignForward_0056e570 @ 0x0056_e570 => extern "thiscall" fn forward_0056e570(this: u32, arg: u32) -> u32 {
        a::forward1(&mut mem(), this, 0x0, ma::PSTRING_ASSIGN, arg)
    }
    /// `(this; arg)`: tail jump.
    PString_AssignAt14_0056e510 @ 0x0056_e510 => extern "thiscall" fn forward_0056e510(this: u32, arg: u32) -> u32 {
        a::forward1(&mut mem(), this, 0x14, ma::PSTRING_ASSIGN, arg)
    }
    /// `(this; arg)`: tail jump.
    PString_AssignAt4_00546450 @ 0x0054_6450 => extern "thiscall" fn forward_00546450(this: u32, arg: u32) -> u32 {
        a::forward1(&mut mem(), this, 0x4, ma::PSTRING_ASSIGN, arg)
    }
    /// `(this; arg)`: tail jump.
    PMatrix4_CopyAtC_0055b630 @ 0x0055_b630 => extern "thiscall" fn forward_0055b630(this: u32, arg: u32) -> u32 {
        a::forward1(&mut mem(), this, 0xc, ma::PMATRIX4_COPY, arg)
    }
    /// Float setter.
    SetFloats0_0049f170 @ 0x0049_f170 => extern "thiscall" fn set_floats_0049f170(this: u32, v0: u32, v1: u32, v2: u32, v3: u32) {
        a::set_floats(&mut mem(), this, 0x0, &[v0, v1, v2, v3]);
    }
    /// Float setter.
    SetFloat1C_0049f240 @ 0x0049_f240 => extern "thiscall" fn set_floats_0049f240(this: u32, v0: u32) {
        a::set_floats(&mut mem(), this, 0x1c, &[v0]);
    }
    /// Float setter.
    SetFloat24_0049f250 @ 0x0049_f250 => extern "thiscall" fn set_floats_0049f250(this: u32, v0: u32) {
        a::set_floats(&mut mem(), this, 0x24, &[v0]);
    }
    /// Float setter.
    SetFloat20_0049f270 @ 0x0049_f270 => extern "thiscall" fn set_floats_0049f270(this: u32, v0: u32) {
        a::set_floats(&mut mem(), this, 0x20, &[v0]);
    }
    /// Float setter.
    SetFloats28_0049f280 @ 0x0049_f280 => extern "thiscall" fn set_floats_0049f280(this: u32, v0: u32, v1: u32) {
        a::set_floats(&mut mem(), this, 0x28, &[v0, v1]);
    }
    /// Float setter.
    SetFloat30_0049f2a0 @ 0x0049_f2a0 => extern "thiscall" fn set_floats_0049f2a0(this: u32, v0: u32) {
        a::set_floats(&mut mem(), this, 0x30, &[v0]);
    }
    /// Float setter.
    SetFloats18_0056ece0 @ 0x0056_ece0 => extern "thiscall" fn set_floats_0056ece0(this: u32, v0: u32, v1: u32, v2: u32, v3: u32) {
        a::set_floats(&mut mem(), this, 0x18, &[v0, v1, v2, v3]);
    }
    /// `(this; a, b)` (float bits).
    SetAngleRange_0049f190 @ 0x0049_f190 => extern "thiscall" fn set_angle_range(this: u32, a: u32, b: u32) {
        a::set_angle_range(&mut mem(), this, a, b);
    }
    /// `(this; p)`.
    SetAngleRangeFrom_0049f1e0 @ 0x0049_f1e0 => extern "thiscall" fn set_angle_range_from(this: u32, p: u32) {
        a::set_angle_range_from(&mut mem(), this, p);
    }
    /// `(this; v)` -> v.
    SetWord10_0049f260 @ 0x0049_f260 => extern "thiscall" fn set_word_0049f260(this: u32, v: u32) -> u32 {
        a::set_word(&mut mem(), this, 0x10, v)
    }
    /// `(this; v)` -> v.
    SetWord0_00546440 @ 0x0054_6440 => extern "thiscall" fn set_word_00546440(this: u32, v: u32) -> u32 {
        a::set_word(&mut mem(), this, 0, v)
    }
    /// `(this; b)` -> al.
    SetByte68_0055b620 @ 0x0055_b620 => extern "thiscall" fn set_byte68(this: u32, b: u32) -> u8 {
        a::set_byte68(&mut mem(), this, b)
    }
    /// `(this; out)` -> out.
    GetPair4C_0055b560 @ 0x0055_b560 => extern "thiscall" fn get_pair4c(this: u32, out: u32) -> u32 {
        a::get_pair4c(&mut mem(), this, out)
    }
    /// `(this)` -> this.
    Phyre_PAnimation_PNetworkOptimizeBufferAllocator_ctor_00616120 @ 0x0061_6120 => extern "thiscall" fn buffer_allocator_ctor(this: u32) -> u32 {
        a::buffer_allocator_ctor(&mut mem(), this)
    }
    /// `(this)`.
    Phyre_PAnimation_PNetworkOptimizeBufferAllocator_dtor_00616150 @ 0x0061_6150 => extern "thiscall" fn buffer_allocator_dtor(this: u32) -> u32 {
        a::buffer_allocator_dtor(&mut mem(), this)
    }
    /// `(this; i)` -> i.
    Phyre_PAnimation_PNetworkOptimizeBufferAllocator_clearSlot_00616310 @ 0x0061_6310 => extern "thiscall" fn clear_slot(this: u32, i: u32) -> u32 {
        a::clear_slot_flag(&mut mem(), this, i)
    }
    /// `(this; i)` -> setb al.
    PSprite_frameBelowLimit_0056dc70 @ 0x0056_dc70 => extern "thiscall" fn frame_below(this: u32, i: u32) -> u32 {
        a::frame_below(&mut mem(), this, i)
    }
    /// `(this)` -> 0.
    PSprite_defaultB8_0056dca0 @ 0x0056_dca0 => extern "thiscall" fn default_b8(this: u32) -> u32 {
        a::default_b8(&mut mem(), this)
    }
    /// `(this; i)` -> word.
    PSharray_word_0055b380 @ 0x0055_b380 => extern "thiscall" fn sharray_word(this: u32, i: u32) -> u32 {
        a::sharray_word(&mut mem(), this, i)
    }
    /// `(this; arg)`.
    PGameplay_forward58_0055b780 @ 0x0055_b780 => extern "thiscall" fn forward_58(this: u32, arg: u32) -> u32 {
        a::forward_58(&mut mem(), this, arg)
    }
}
