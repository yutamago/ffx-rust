//! M5 stage 1, list `scene`, batch 9 (`ffx_prender::m5p_scene_i`): the
//! light sort's partition step in its three instantiations. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_i_*`).

use ffx_prender::m5p_scene_i::Order;
use ffx_prender::m5p_scene_i::bridge as b;

use super::{mem, ported};

ported! {
    /// `Phyre_PRendering_PContextSwitchLights_partitionKeys_004d2400`: 4-byte keys partitioned around their median; `out`.
    Phyre_PRendering_PContextSwitchLights_partitionKeys_004d2400 @ 0x004d_2400 => extern "C" fn si_004d2400(out: u32, first: u32, last: u32) -> u32 {
        b::partition(&mut mem(), Order::Below, out, first, last)
    }
    /// `Phyre_PRendering_PContextSwitchLights_partitionValues_004d2560`: 4-byte values partitioned by a predicate; `out`.
    Phyre_PRendering_PContextSwitchLights_partitionValues_004d2560 @ 0x004d_2560 => extern "C" fn si_004d2560(out: u32, first: u32, last: u32, pred: u32) -> u32 {
        b::partition(&mut mem(), Order::ByValue(pred), out, first, last)
    }
    /// `Phyre_PRendering_PContextSwitchLights_partitionRecords_004d2740`: 8-byte records partitioned by a predicate; `out`.
    Phyre_PRendering_PContextSwitchLights_partitionRecords_004d2740 @ 0x004d_2740 => extern "C" fn si_004d2740(out: u32, first: u32, last: u32, pred: u32) -> u32 {
        b::partition(&mut mem(), Order::ByPointer(pred), out, first, last)
    }
}
