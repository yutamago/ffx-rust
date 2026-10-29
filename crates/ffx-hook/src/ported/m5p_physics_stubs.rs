//! M5 stage 1, list `phyre-physics`: `PUtilityPhysics`'s slot 2, which
//! `Phyre_PhyreExit` calls on every utility; `xor eax, eax; ret` is under
//! five bytes, so [`host_only!`](super::host_only) gives the 32-bit game no
//! hook and `ffx-host` a table entry (`docs/porting.md`, "Functions under
//! five bytes").

use ffx_pcore::m5p_stubs as t;

use super::host_only;

host_only! {
    /// `xor eax, eax; ret` (3 bytes): `eax = 0`; `thiscall(this)`.
    Phyre_PPhysics_PUtilityPhysics_slot2 @ 0x0060_5720 => extern "thiscall" fn ph_00605720(_this: u32) -> u32 {
        t::bridge::zero()
    }
}
