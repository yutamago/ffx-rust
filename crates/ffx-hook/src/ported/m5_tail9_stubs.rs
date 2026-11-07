//! M5 stage 1, round 9 (`tail9`): `YiWindowHandler_idle_00907e50`, an
//! `xor eax, eax; ret` (3 bytes, int3 padding after it) under five bytes,
//! so [`host_only!`](super::host_only) gives the 32-bit game no hook and
//! `ffx-host` a table entry (docs/porting.md, "Functions under five
//! bytes"; crates/ffx-host/tests/m5_system_a.rs runs it from the table).

use super::host_only;

host_only! {
    /// `YiWindowHandler_idle_00907e50(window)` (cdecl, the idle handler
    /// `yiWindowFinishAll` and `yiWindowFinish_009071f0` store at window
    /// `+0x74`; it reads no argument): 0.
    YiWindowHandler_idle_00907e50 @ 0x0090_7e50 => extern "C" fn zero_00907e50() -> u32 {
        ffx_pcore::m5p_stubs::bridge::zero()
    }
}
