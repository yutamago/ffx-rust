//! M5 stage 1, list `scene`, batch 10 (`ffx_prender::m5p_scene_j`): the
//! light sort's heap sort step and the occluder job set-up. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_j_*`).

use ffx_prender::m5p_scene_j::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PContextSwitchLights_sortHeap_004d2360`: the heap of 8-byte records sorted.
    Phyre_PRendering_PContextSwitchLights_sortHeap_004d2360 @ 0x004d_2360 => extern "C" fn sj_004d2360(first: u32, last: u32, pred: u32) {
        let mut frame = StackFrame::<2>::new();
        b::sort_heap(&mut mem(), first, last, pred, frame.at());
    }
    /// `Phyre_PRendering_PVisibilityCheck_addOccluderJob_009f2f20`: an occluder job allocated, filled and appended; the old tail.
    Phyre_PRendering_PVisibilityCheck_addOccluderJob_009f2f20 @ 0x009f_2f20 => extern "thiscall" fn sj_009f2f20(this: u32, world: u32, n: u32, context: u32) -> u32 {
        b::occluder_job(&mut mem(), this, world, n, context)
    }
}
