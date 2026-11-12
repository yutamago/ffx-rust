//! M5 finish, WP5: originals the ports called in Unicorn
//! (`ffx_prender::wp5_hot`). The parity tests are
//! `crates/ffx-oracle/tests/wp5_hot.rs` and `wp5_cloth.rs`.
//!
//! Threads: the render thread (`PRenderThread_processRenderCommands`'s job
//! lists, `graphicCharacterInstanceSetClothVertexBuffers`' cloth batches).
//! No shim keeps state; the cloth batch's local is a `StackFrame`.

use ffx_prender::wp5_hot as hot;

use super::{StackFrame, mem, ported};

ported! {
    /// `PRenderThread_JobList_ctor_00613f70`.
    PRenderThread_JobList_ctor_00613f70 @ 0x0061_3f70 => extern "thiscall" fn w_00613f70(this: u32) -> u32 {
        hot::job_list_ctor(&mut mem(), this)
    }
    /// `batchSetClothVertexBuffers(_, &first, &last)` -> `last` (the
    /// stream map's two-word local in a frame).
    batchSetClothVertexBuffers @ 0x0063_65f0 => extern "C" fn w_006365f0(unused: u32, first: u32, last: u32) -> u32 {
        let _ = unused;
        let mut f = StackFrame::<2>::new();
        hot::cloth::batch_set_cloth_vertex_buffers(&mut mem(), first, last, f.at())
    }
    /// `VsyncHandler`: the vsync thread's callbacks (0, then 1 and 2 unless disabled) while on.
    VsyncHandler @ 0x0088_7d60 => extern "C" fn w_00887d60() -> u32 {
        hot::vsync_handler(&mut mem())
    }
}
