//! M5 stage 1, the leftovers of list `phyre-scripting` (round left2)
//! (`ffx_pother_a::m5p_scripting_left2`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_scripting.rs` (`left2_*`).
//!
//! Threads: the script layer runs on the main thread; the `PString`
//! words the originals keep in their frames are per-call [`StackFrame`]s.

use ffx_pother_a::m5p_scripting_left2::bridge as lb;

use super::{StackFrame, mem, ported};

ported! {
    /// The cloth mesh's `createInstance(cluster, existing)`.
    ClothInstancingDynamicMesh_createInstance_006e1ec0 @ 0x006e_1ec0 => extern "thiscall" fn cloth_create_instance(mesh: u32, cluster: u32, existing: u32) -> u32 {
        lb::cloth_create_instance(&mut mem(), mesh, cluster, existing)
    }
    /// The script `require` handler `(L)`.
    Phyre_PScripting_requireHandler_005d67b0 @ 0x005d_67b0 => extern "C" fn require(l: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::require(&mut mem(), l, f.at())
    }
    /// `PScriptableComponent::reloadScript(component)`.
    Phyre_PGameplay_PScriptableComponent_reloadScript @ 0x0055_32a0 => extern "thiscall" fn reload_script(this: u32, component: u32) {
        let mut f = StackFrame::<1>::new();
        lb::reload_script(&mut mem(), this, component, f.at());
    }
    /// The dotted member-path check `(class, path)`; `al`.
    Phyre_PScripting_hasMemberPath_0050be90 @ 0x0050_be90 => extern "C" fn has_member_path(class: u32, path: u32) -> u8 {
        lb::has_member_path(&mut mem(), class, path)
    }
}
