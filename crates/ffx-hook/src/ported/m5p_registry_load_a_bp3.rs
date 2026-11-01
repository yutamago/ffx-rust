//! M5 stage 1, list `phyre-registry-load-a`, round big-phyre3
//! (`ffx_pload::m5p_registry_load_a_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_registry_load_a.rs` (`bp3_*`). The
//! frames are per call.

use ffx_pload::m5p_registry_load_a_bp3 as rl;

use super::{StackFrame, mem, ported};

ported! {
    /// `Phyre_PBinary_reloadClusterTextures(reader, info)`: the cluster
    /// iterator in a frame.
    Phyre_PBinary_reloadClusterTextures @ 0x0046_6da0 => extern "stdcall" fn rlbp3_00466da0(reader: u32, info: u32) -> u32 {
        let mut f = StackFrame::<{ rl::RELOAD_FRAME_WORDS as usize }>::new();
        let ebp = f.at() + 4 * rl::RELOAD_FRAME_WORDS;
        rl::reload::reload_cluster_textures(&mut mem(), ebp, reader, info)
    }
}
