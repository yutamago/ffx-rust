//! M5 stage 1, list `vfx` (round 7, `left7`): `code_007e3f90`, a bare
//! `ret` (1 byte, int3 padding after it) under five bytes, so
//! [`host_only!`](super::host_only) gives the 32-bit game no hook and
//! `ffx-host` a table entry (docs/porting.md, "Functions under five
//! bytes"; crates/ffx-host/tests/m5_vfx_stubs.rs runs it from the table).

use super::host_only;

host_only! {
    /// `code_007e3f90(x)` (cdecl, called from `0x0079e789` after
    /// `MsEtEffectReadInit` with one word it never reads): nothing; the
    /// caller's `eax`, `ecx` and `edx` stay.
    code_007e3f90 @ 0x007e_3f90 => extern "C" fn nothing_007e3f90() {
        ffx_pcore::m5p_stubs::bridge::nothing();
    }
}
