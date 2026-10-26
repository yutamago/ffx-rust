//! M4 leftovers (`m4-rest`): the functions eight earlier lists left
//! behind. The rest of those lists that can't be hooked is in
//! `~/ffx-work/m4/port/not-portable.tsv` with the reasons.
//!
//! Threads: the dynamic meshes' draw tables are sized where the meshes
//! update (the main thread); the radial lines are set up by
//! `RadialLine_initialize` on the main thread. The shims keep no state.

use super::{mem, ported};
use ffx_pother_b::m4r_mesh::bridge as mesh;

ported! {
    // ---- h1-render-a ----
    /// `PArray16_resize_00498f30(array, count)` -> 0, or 0xd when the
    /// allocation fails.
    PArray16_resize_00498f30 @ 0x0049_8f30 => extern "thiscall" fn resize16(array: u32, count: u32) -> u32 {
        mesh::resize16(&mut mem(), array, count)
    }
    // ---- p4-other-b ----
    /// `RadialLine_createMeshData_006f8c50(lines)` -> the last callee's
    /// `eax`.
    RadialLine_createMeshData_006f8c50 @ 0x006f_8c50 => extern "thiscall" fn radial_create_mesh(lines: u32) -> u32 {
        mesh::create_mesh_data(&mut mem(), lines)
    }
}
