//! M5 finish, WP5: originals the ports called in Unicorn
//! (`ffx_prender::wp5_hot`). The parity tests are
//! `crates/ffx-oracle/tests/wp5_hot.rs`.
//!
//! Threads: the render thread (`PRenderThread_processRenderCommands`'s job
//! lists). No shim keeps state.

use ffx_prender::wp5_hot as hot;

use super::{mem, ported};

ported! {
    /// `PRenderThread_JobList_ctor_00613f70`.
    PRenderThread_JobList_ctor_00613f70 @ 0x0061_3f70 => extern "thiscall" fn w_00613f70(this: u32) -> u32 {
        hot::job_list_ctor(&mut mem(), this)
    }
}
