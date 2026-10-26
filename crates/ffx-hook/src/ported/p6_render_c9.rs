//! M4 port, list p6-render-c, continuation round 5: the shims. Ports in
//! `ffx_prender::p6_cmd`, `p6_appframe`, `p6_appinit` and `p6_quads`.

use super::{StackFrame, mem, ported};
use ffx_prender::{p6_appframe as af, p6_appinit as ai, p6_cmd as cm, p6_quads as qd};

ported! {
    /// `PRenderThread::handleNonRenderInstanceCommand(this; renderer, cmd,
    /// dirty)` -> the command's callee's `eax` (dropped by its caller).
    Phyre_PRendering_PRenderThread_handleNonRenderInstanceCommand @ 0x0061_4c60 => extern "thiscall" fn c_00614c60(this: u32, renderer: u32, cmd: u32, dirty: u32) -> u32 {
        cm::bridge::handle(&mut mem(), this, renderer, cmd, dirty)
    }
    /// `PApplication::frame(this)` -> 0, 9 (quit) or a step's error.
    Phyre_PFramework_PApplication_frame @ 0x0062_7940 => extern "thiscall" fn c_00627940(this: u32) -> u32 {
        // the four render scope locks at ebp - 0x10 .. ebp - 0x1c
        let mut frame = StackFrame::<8>::new();
        let ebp = frame.at() + 0x20;
        af::bridge::frame(&mut mem(), this, ebp)
    }
    /// `PApplication::onInit(this; a, b)` -> 0, 0xd or a step's error.
    Phyre_PFramework_PApplication_onInit @ 0x0062_a600 => extern "thiscall" fn c_0062a600(this: u32, a: u32, b: u32) -> u32 {
        // ebp - 0x130 .. ebp: the text buffer, the lock, the string and
        // the allocation the original keeps there
        let mut frame = StackFrame::<0x4c>::new();
        let ebp = frame.at() + 0x130;
        ai::bridge::on_init(&mut mem(), this, a, b, ebp)
    }
    /// The split full-screen mesh in `cluster` at the texture
    /// coordinates `u`, `v` (floats) -> its mesh instance's constructor's
    /// `eax`, or 0.
    FullscreenMesh_createSplit_00a06e60 @ 0x00a0_6e60 => extern "C" fn c_00a06e60(cluster: u32, u: u32, v: u32) -> u32 {
        let mut frame = StackFrame::<6>::new();
        qd::bridge::split(&mut mem(), cluster, frame.at(), u, v)
    }
}
