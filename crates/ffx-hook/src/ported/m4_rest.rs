//! M4 leftovers (`m4-rest`): the functions eight earlier lists left
//! behind. The rest of those lists that can't be hooked is in
//! `~/ffx-work/m4/port/not-portable.tsv` with the reasons.
//!
//! Threads: the dynamic meshes' draw tables are sized where the meshes
//! update (the main thread); the radial lines are set up by
//! `RadialLine_initialize` on the main thread; `fillDynamicBlocks` runs
//! where the renderer binds a dynamic mesh's streams (the render
//! thread's mesh binding); `renderMeshInstanceList` where a world is
//! rendered (`graphicRender`, the main thread; 9 KB frame), and
//! `generateNodeParameterCaptureBuffer` for each node it draws; the
//! modifier network's run where `PModifierNetworkDynamicMesh::fillPacket`
//! fills a packet (its `_alloca` arrays on the heap: no large frames on
//! a worker's stack). The shims keep no state; frames are per call.

use super::{StackFrame, mem, ported};
use ffx_pother_b::m4r_dynblocks::bridge as blocks;
use ffx_pother_b::m4r_mesh::bridge as mesh;
use ffx_prender::m4r_meshlist as meshlist;
use ffx_prender::m4r_modnet as modnet;
use ffx_prender::m4r_nodecapture as nodecapture;

ported! {
    // ---- h1-render-a ----
    /// `PArray16_resize_00498f30(array, count)` -> 0, or 0xd when the
    /// allocation fails.
    PArray16_resize_00498f30 @ 0x0049_8f30 => extern "thiscall" fn resize16(array: u32, count: u32) -> u32 {
        mesh::resize16(&mut mem(), array, count)
    }
    // ---- p4-other-b ----
    /// `RadialLine_createMeshData_006f8c50(lines)` -> the last callee's
    /// `eax`.
    RadialLine_createMeshData_006f8c50 @ 0x006f_8c50 => extern "thiscall" fn radial_create_mesh(lines: u32) -> u32 {
        mesh::create_mesh_data(&mut mem(), lines)
    }
    // ---- p6-render-c ----
    /// `ClassDynamicMesh_fillDynamicBlocks_00672b80(index, inst, streams,
    /// index_block)` -> 0 (vtable slot 5; `ecx` unused).
    ClassDynamicMesh_fillDynamicBlocks_00672b80 @ 0x0067_2b80 => extern "thiscall" fn fill_dynamic_blocks(_mesh: u32, index: u32, inst: u32, streams: u32, index_block: u32) -> u32 {
        blocks::fill_dynamic_blocks(&mut mem(), index, inst, streams, index_block)
    }
    /// `PRendererBase::renderMeshInstanceList(renderer; begin, end,
    /// contexts)` -> 0, or the single-thread path's last error.
    Phyre_PRendering_PRendererBase_renderMeshInstanceList @ 0x005b_3df0 => extern "thiscall" fn render_mesh_instance_list(renderer: u32, begin: u32, end: u32, contexts: u32) -> u32 {
        // ebp - 0x23e0 .. ebp: the job builder, the cursor, the builder's
        // parameters and counters (the callees get pointers into it)
        let mut frame = StackFrame::<0x8f8>::new();
        let ebp = frame.at() + meshlist::FRAME;
        meshlist::bridge::render_list(&mut mem(), renderer, begin, end, contexts, ebp)
    }
    /// `PRendererBase::generateNodeParameterCaptureBuffer(renderer;
    /// node)` -> the capture buffer, or 0.
    Phyre_PRendering_PRendererBase_generateNodeParameterCaptureBuffer @ 0x005b_7fe0 => extern "thiscall" fn generate_node_parameters(renderer: u32, node: u32) -> u32 {
        // ebp - 0x60c .. ebp: the matrices the helpers fill, the values
        // the buffer copies (zeroed: words a value's size reaches past
        // what this call wrote read 0, brief 2026-10-24)
        let mut frame = StackFrame::<0x183>::new();
        let ebp = frame.at() + nodecapture::FRAME;
        nodecapture::bridge::generate(&mut mem(), renderer, node, ebp)
    }
    /// `PModifierNetwork_runPacket_005c1f90(packet)` -> the packet.
    PModifierNetwork_runPacket_005c1f90 @ 0x005c_1f90 => extern "C" fn modifier_run(packet: u32) -> u32 {
        // the modifiers' context (the original's ebp - 0x58, two words)
        let mut frame = StackFrame::<2>::new();
        let ctx = frame.at();
        let mut heap: Vec<Vec<u32>> = Vec::new();
        let r = modnet::bridge::run(&mut mem(), packet, ctx, &mut |bytes| {
            let v = vec![0_u32; (bytes as usize).div_ceil(4) + 4];
            let at = u32::try_from(v.as_ptr().expose_provenance()).unwrap_or(0);
            heap.push(v);
            at
        });
        std::hint::black_box(&mut heap);
        r
    }
}
