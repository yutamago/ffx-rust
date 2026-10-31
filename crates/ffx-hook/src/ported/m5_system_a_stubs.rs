//! M5 stage 1, list `system-a` (round 3): three `xor eax, eax; ret`
//! functions (3 bytes, int3 padding after them) under five bytes, so
//! [`host_only!`](super::host_only) gives the 32-bit game no hook and
//! `ffx-host` a table entry each (docs/porting.md, "Functions under five
//! bytes"; crates/ffx-host/tests/m5_system_a.rs runs them from the table).

use super::host_only;

host_only! {
    /// `code_0062f640()` (cdecl, from `0x00885560`, the HDD-mode refresh): 0.
    code_0062f640 @ 0x0062_f640 => extern "C" fn zero_0062f640() -> u32 {
        ffx_pcore::m5p_stubs::bridge::zero()
    }
    /// `code_0062f980()` (cdecl, from `TkSyncVEI`): 0.
    code_0062f980 @ 0x0062_f980 => extern "C" fn zero_0062f980() -> u32 {
        ffx_pcore::m5p_stubs::bridge::zero()
    }
    /// `code_0062fb00()` (cdecl, from `TkRefreshHDDMode`): 0.
    code_0062fb00 @ 0x0062_fb00 => extern "C" fn zero_0062fb00() -> u32 {
        ffx_pcore::m5p_stubs::bridge::zero()
    }
}
