//! M5 stage 1, list `phyre-scene`, round big-phyre3
//! (`ffx_prender::m5p_scene_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_bp3_*`). The
//! frames are per call.

use ffx_prender::m5p_scene_bp3 as sc;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PContextSwitchLights_getStringsFromPackedState(this; a, b, out)`:
    /// the decoded types and the three string builders in a frame.
    Phyre_PRendering_PContextSwitchLights_getStringsFromPackedState @ 0x004d_4810 => extern "thiscall" fn sbp3_004d4810(this: u32, a: u32, b: u32, out: u32) -> u32 {
        let mut f = StackFrame::<{ sc::LIGHT_STRINGS_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * sc::LIGHT_STRINGS_FRAME_WORDS;
        sc::light_strings::get_strings(&mut mem(), ebp, this, a, b, out)
    }
}
