//! M5 stage 1, list `scene`, batch 11 (`ffx_prender::m5p_scene_k`): the
//! world's instance count of a class. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_k_*`).

use ffx_prender::m5p_scene_k::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PWorld_instanceCount_00443550`: the instances of a class (its own list, or its derived classes' lists).
    Phyre_PWorld_instanceCount_00443550 @ 0x0044_3550 => extern "thiscall" fn sk_00443550(this: u32, class: u32) -> u32 {
        let mut frame = StackFrame::<3>::new();
        b::world_instance_count(&mut mem(), this, class, frame.at())
    }
}
