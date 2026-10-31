//! M5 stage 1, list `phyre-scene`, round "design-frames"
//! (`ffx_prender::m5p_scene_df`): the varargs forwarder `0x0044e9e0`,
//! whose `va_list` points into its caller's argument words (the shim's
//! `slots`, docs/porting.md "Run-time frames and varargs"). The parity
//! test is in `crates/ffx-oracle/tests/m5_design_frames.rs`.
//!
//! Threads: the entity lookups' (main); the frame is per call.

use ffx_prender::m5p_scene_df::{self as sd, bridge as sb};

use super::{StackFrame, mem, ported, with_frame};

ported! {
    /// A member's value through its accessor `0x0044e9e0(member, object, ...)`.
    PEntity_memberValue_0044e9e0 @ 0x0044_e9e0 => extern "C" fn member_value(member: u32, object: u32; slots: u32) -> u32 {
        let mut list = StackFrame::<1>::new();
        let cell = list.at();
        // the variable words start after the two named ones
        let va = slots.wrapping_add(8);
        with_frame(sd::VALUE_BYTES, |value| sb::member_value(&mut mem(), member, object, va, cell, value))
    }
}
