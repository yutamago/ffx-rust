//! M5 stage 1, list `scripting`, round big-phyre2
//! (`ffx_pother_a::m5p_scripting_bp2`): an occluder's silhouette planes,
//! with its locals in a `StackFrame` and its plane flags in a run-time
//! frame ([`super::with_frame`]). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scripting.rs` (`bp2_*`).
//!
//! Threads: the visibility check's (render side); the frames are per call.

use ffx_pother_a::m5p_scripting_bp2::{self as sb, bridge as b};

use super::{StackFrame, mem, ported, with_frame};

ported! {
    /// `POccluderGeometry_silhouettePlanes_00a312c0(item; owner, planes, block)`: the end of the planes written.
    POccluderGeometry_silhouettePlanes_00a312c0 @ 0x00a3_12c0 => extern "thiscall" fn sbp2_00a312c0(this: u32, owner: u32, planes: u32, block: u32) -> u32 {
        let mut f = StackFrame::<{ sb::SILHOUETTE_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * sb::SILHOUETTE_FRAME_WORDS;
        let bytes = b::silhouette_prepare(&mut mem(), ebp, this, owner);
        with_frame(bytes, |flags| b::silhouette_finish(&mut mem(), ebp, planes, block, flags))
    }
}
