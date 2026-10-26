//! M4 port, list p6-render-c, continuation round 2 (batch 12): the
//! shims. Ports in `ffx_prender::p6_setup` (and one descriptor shape of
//! `ffx_prender::p6_descriptor`).

use super::{StackFrame, mem, ported};
use ffx_prender::{p6_descriptor as descriptor, p6_setup as su};

ported! {
    /// A texture array's slot marked (`this`; i, _) -> 0, 3 or 0x15.
    Phyre_PRendering_PTextureArray_markSlot_0059d320 @ 0x0059_d320 => extern "thiscall" fn u_0059d320(this: u32, i: u32, _unused: u32) -> u32 {
        su::bridge::mark_slot(&mut mem(), this, i)
    }
    /// `constructInstance(obj)` -> `al` 1.
    Phyre_PClassDescriptorConcrete_Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh__constructInstance @ 0x005a_da70 => extern "thiscall" fn u_005ada70(_desc: u32, obj: u32) -> u8 {
        descriptor::bridge::construct_with_zeroed(&mut mem(), obj, su::MESH_BASE_CTOR, Some(su::MESH_VTABLE), None, &su::MESH_ZEROS)
    }
    /// Two owned arrays freed (`this`).
    Phyre_PDynamicGeometry_freeArrays_005b0e40 @ 0x005b_0e40 => extern "thiscall" fn u_005b0e40(this: u32) -> u32 {
        su::bridge::free_arrays(&mut mem(), this)
    }
    /// `PRendererBase::PRendererBase(this; device)` -> `this`.
    Phyre_PRendering_PRendererBase_PRendererBase_005b4650 @ 0x005b_4650 => extern "thiscall" fn u_005b4650(this: u32, device: u32) -> u32 {
        su::bridge::renderer_base_ctor(&mut mem(), this, device)
    }
    /// `PRenderer::PRenderer(this; device)` -> `this`.
    Phyre_PRendering_PRenderer_PRenderer @ 0x005b_45b0 => extern "thiscall" fn u_005b45b0(this: u32, device: u32) -> u32 {
        su::bridge::renderer_ctor(&mut mem(), this, device)
    }
    /// The material parameter capture buffer (`this`; desc) -> the copy.
    Phyre_PRendering_PRendererBase_generateMaterialParameterCaptureBuffer @ 0x005b_7f30 => extern "thiscall" fn u_005b7f30(this: u32, desc: u32) -> u32 {
        su::bridge::material_capture(&mut mem(), this, desc)
    }
    /// The renderer's rings set up (`this`; data, commands, sizes, segments).
    Phyre_PRendering_PRendererBase_initRings_005b8e80 @ 0x005b_8e80 => extern "thiscall" fn u_005b8e80(this: u32, data: u32, commands: u32, data_size: u32, command_size: u32, segments: u32) -> u32 {
        su::bridge::init_rings(&mut mem(), this, data, commands, data_size, command_size, segments)
    }
    /// A node's three arrays visited (node, visitor).
    Phyre_PDynamicGeometry_visitArrays_005c5170 @ 0x005c_5170 => extern "C" fn u_005c5170(node: u32, visitor: u32) -> u32 {
        su::bridge::visit_arrays(&mut mem(), node, visitor)
    }
    /// `PRenderThread::initialize(this)` -> 0 or an error.
    Phyre_PRendering_PRenderThread_initialize @ 0x0061_5140 => extern "thiscall" fn u_00615140(this: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        su::bridge::thread_initialize(&mut mem(), this, frame.at())
    }
    /// The render state's viewport with the size assert (`this`; x, y, w, h).
    Phyre_PRendering_PRenderThread_setViewport_00615de0 @ 0x0061_5de0 => extern "thiscall" fn u_00615de0(this: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        su::bridge::thread_set_viewport(&mut mem(), this, x, y, w, h)
    }
    /// `PRenderThread::sync(this)` -> 0.
    Phyre_PRendering_PRenderThread_sync @ 0x0061_5ea0 => extern "thiscall" fn u_00615ea0(this: u32) -> u32 {
        let mut frame = StackFrame::<0x13>::new();
        su::bridge::thread_sync(&mut mem(), this, frame.at())
    }
    /// `PRendererBase::setViewport(this; x, y, w, h)` -> 0 or 0xd.
    Phyre_PRendering_PRendererBase_setViewport @ 0x005b_c060 => extern "thiscall" fn u_005bc060(this: u32, x: u32, y: u32, w: u32, h: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        su::bridge::renderer_set_viewport(&mut mem(), this, frame.at(), [x, y, w, h])
    }
    /// `PVisibilityCheck::buildOccluders(this)`.
    Phyre_PRendering_PVisibilityCheck_buildOccluders @ 0x009f_2fd0 => extern "thiscall" fn u_009f2fd0(this: u32) -> u32 {
        su::bridge::build_occluders(&mut mem(), this)
    }
    /// `PVisibilityCheck::waitAndCount(this)` -> the result count.
    Phyre_PRendering_PVisibilityCheck_waitAndCount @ 0x009f_30d0 => extern "thiscall" fn u_009f30d0(this: u32) -> u32 {
        su::bridge::wait_and_count(&mut mem(), this)
    }
    /// The world cursor's advance (`this`; n).
    PWorldCursor_advance_009f3dc0 @ 0x009f_3dc0 => extern "thiscall" fn u_009f3dc0(this: u32, n: u32) {
        su::bridge::advance(&mut mem(), this, n);
    }
    /// `PVisibilityCheck::startJobsForObjects(this; objects, n, stride)`.
    Phyre_PRendering_PVisibilityCheck_startJobsForObjects @ 0x009f_4000 => extern "thiscall" fn u_009f4000(this: u32, objects: u32, n: u32, stride: u32) {
        su::bridge::start_jobs_for_objects(&mut mem(), this, objects, n, stride);
    }
}
