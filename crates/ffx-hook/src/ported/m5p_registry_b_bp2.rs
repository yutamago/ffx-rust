//! M5 stage 1, list `phyre-registry-load-b`, round big-phyre2
//! (`ffx_pload::m5p_registry_b_bp2`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_registry_b.rs` (`bp2_*`). Culling code
//! (render side): the frame is per call.

use ffx_pload::m5p_registry_b_bp2::{self as rb, bridge as b};

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PVisibilityCheck_gatherOccluders_00a2f7d0(out, items, count, a)`:
    /// the matrices and the extent in a frame.
    Phyre_PRendering_PVisibilityCheck_gatherOccluders_00a2f7d0 @ 0x00a2_f7d0 => extern "C" fn rgbp2_00a2f7d0(out: u32, items: u32, count: u32, a: u32) {
        let mut f = StackFrame::<{ rb::GATHER_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rb::GATHER_FRAME_WORDS;
        b::gather_occluders(&mut mem(), ebp, out, items, count, a);
    }
}
