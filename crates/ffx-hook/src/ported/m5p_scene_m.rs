//! M5 stage 1, list `scene`, batch 13 (`ffx_prender::m5p_scene_m`): the
//! shader LOD context switch's define string. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_m_*`).

use ffx_prender::m5p_scene_m::bridge as b;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PRendering_PContextSwitchShaderLODLevel_getStringsFromPackedState`: the next packed level as a `-D<name>=<level>` define; 0 or 0xd.
    Phyre_PRendering_PContextSwitchShaderLODLevel_getStringsFromPackedState @ 0x004d_4ba0 => extern "thiscall" fn sm_004d4ba0(this: u32, cursor: u32, _unused: u32, strings: u32) -> u32 {
        let mut frame = StackFrame::<3>::new();
        b::lod_level_strings(&mut mem(), this, cursor, strings, frame.at())
    }
}
