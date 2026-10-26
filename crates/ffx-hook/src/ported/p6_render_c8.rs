//! M4 port, list p6-render-c, continuation round 4: the shims. Ports in
//! `ffx_prender::p6_quads` and `p6_tail`.

use super::{StackFrame, mem, ported};
use ffx_prender::{p6_quads as qd, p6_tail as tl};

ported! {
    /// The full-screen triangle (position, texture coordinates) in
    /// `cluster` -> its mesh instance's constructor's `eax`, or 0.
    FullscreenMesh_createTriangle_00a05e70 @ 0x00a0_5e70 => extern "C" fn x_00a05e70(cluster: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        qd::bridge::build(&mut mem(), cluster, frame.at(), &qd::FULLSCREEN_MESHES[0])
    }
    /// The full-screen triangle with a white colour stream.
    FullscreenMesh_createColoredTriangle_00a06120 @ 0x00a0_6120 => extern "C" fn x_00a06120(cluster: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        qd::bridge::build(&mut mem(), cluster, frame.at(), &qd::FULLSCREEN_MESHES[1])
    }
    /// The full-screen quad (six vertices, texture coordinates 0..1).
    FullscreenMesh_createQuad_00a06470 @ 0x00a0_6470 => extern "C" fn x_00a06470(cluster: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        qd::bridge::build(&mut mem(), cluster, frame.at(), &qd::FULLSCREEN_MESHES[2])
    }
    /// The full-screen quad with v mirrored.
    FullscreenMesh_createQuadFlippedV_00a06750 @ 0x00a0_6750 => extern "C" fn x_00a06750(cluster: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        qd::bridge::build(&mut mem(), cluster, frame.at(), &qd::FULLSCREEN_MESHES[3])
    }
    /// Iggy's warning callback `(_, _, code, text)` -> the TTY dispatch's
    /// `eax`: the text in a 16 KB buffer, as the original's frame.
    Iggy_warningCallback_00a3e600 @ 0x00a3_e600 => extern "stdcall" fn x_00a3e600(a: u32, b: u32, code: i32, text: u32) -> u32 {
        let _ = (a, b);
        let mut frame = StackFrame::<0x1000>::new();
        tl::bridge::iggy_warning(&mut mem(), frame.at(), code, text)
    }
    /// `PBitmapTextMaterial::initialize(this; cluster, technique)` -> 0 or
    /// an error.
    Phyre_PText_PBitmapTextMaterial_initialize @ 0x00a2_9ee0 => extern "thiscall" fn x_00a29ee0(this: u32, cluster: u32, technique: u32) -> u32 {
        let mut frame = StackFrame::<8>::new();
        tl::material::initialize(&mut mem(), this, frame.at(), cluster, technique).0
    }
    /// The parameter layout `(base, params, n, &end, sizer)` -> 0 or an
    /// error.
    ParameterLayout_build_005a0600 @ 0x005a_0600 => extern "C" fn x_005a0600(base: u32, params: u32, n: u32, end: u32, sizer: u32) -> u32 {
        let mut frame = StackFrame::<3>::new();
        tl::layout::layout(&mut mem(), frame.at(), [base, params, n, end, sizer])
    }
    /// The layout records' build `(records; params, n, sizer)`.
    ParameterLayout_buildRecords_005a08b0 @ 0x005a_08b0 => extern "thiscall" fn x_005a08b0(records: u32, params: u32, n: u32, sizer: u32) -> u32 {
        tl::layout::build_records(&mut mem(), records, params, n, sizer)
    }
    /// The layout records' packing `(records; start)` -> 0: the sorted
    /// pointer array (the original's `_alloca`) in this frame up to 64
    /// records, else on the heap.
    ParameterLayout_pack_005a0cf0 @ 0x005a_0cf0 => extern "thiscall" fn x_005a0cf0(records: u32, start: u32) -> u32 {
        let mut frame = StackFrame::<64>::new();
        let mut heap: Vec<u32> = Vec::new();
        let small = frame.at();
        let r = tl::layout::pack(&mut mem(), records, start, |n| {
            if n <= 64 {
                small
            } else {
                heap = vec![0u32; n as usize];
                u32::try_from(heap.as_mut_ptr().expose_provenance()).unwrap_or(0)
            }
        });
        std::hint::black_box(&mut heap);
        r
    }
    /// `PInputDeviceMouse::update(mouse)` (vtable slot 5) -> `eax`.
    Phyre_PFramework_PInputDeviceMouse_update @ 0x0062_d6f0 => extern "thiscall" fn x_0062d6f0(mouse: u32) -> u32 {
        let mut frame = StackFrame::<33>::new();
        tl::mouse::update(&mut mem(), mouse, frame.at())
    }
}
