//! M5 stage 1, list `phyre-scripting`, round big-phyre3
//! (`ffx_pother_a::m5p_scripting_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scripting.rs` (`bp3_*`).

use ffx_pother_a::m5p_scripting_bp3 as sc;

use super::{mem, ported};

ported! {
    /// `Phyre_PScripting_PClassCallableMethodScript_PCallerScript_vcall(this; object, ret, holder, va)`:
    /// `pushBase` gets the object's own argument slot, so the shim takes
    /// `slots`.
    Phyre_PScripting_PClassCallableMethodScript_PCallerScript_vcall @ 0x0061_a2a0 => extern "thiscall" fn scbp3_0061a2a0(this: u32, _object: u32, ret: u32, holder: u32, va: u32; slots: u32) -> u32 {
        sc::vcall::vcall(&mut mem(), slots, this, ret, holder, va)
    }
}
