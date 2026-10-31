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
    /// `Phyre::PScripting::RegisterTypes(L, ns, name)`.
    Phyre_PScripting_RegisterTypes @ 0x0043_6f90 => extern "C" fn register_types(l: u32, ns: u32, name: u32) -> u32 {
        lb::register_types(&mut mem(), l, ns, name)
    }
    /// `Phyre::PScripting::RegisterVectorMatrixTypes(L)`.
    Phyre_PScripting_RegisterVectorMatrixTypes @ 0x0057_21b0 => extern "C" fn register_vector_matrix(l: u32) -> u32 {
        lb::register_vector_matrix(&mut mem(), l)
    }
    /// `Phyre::PScripting::Require(L)`.
    Phyre_PScripting_Require @ 0x005d_63a0 => extern "C" fn require_module(l: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::require_module(&mut mem(), l, f.at())
    }
    /// `Phyre::PScripting::IndexHandler(L)`.
    Phyre_PScripting_IndexHandler @ 0x0043_6430 => extern "C" fn index_handler(l: u32) -> u32 {
        lb::index_handler(&mut mem(), l)
    }
    /// `Phyre::PScripting::NewIndexHandler(L)`.
    Phyre_PScripting_NewIndexHandler @ 0x0043_6860 => extern "C" fn new_index_handler(l: u32) -> u32 {
        lb::new_index_handler(&mut mem(), l)
    }
    /// `Phyre::PScripting::GetScriptFunctionNames(script, names)`.
    Phyre_PScripting_GetScriptFunctionNames @ 0x005d_5860 => extern "C" fn script_function_names(script: u32, names: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::script_function_names(&mut mem(), script, names, f.at())
    }
    /// `PClassCallableMethodScript::CopyValuesFromContext(from, to, first, n)`.
    Phyre_PScripting_PClassCallableMethodScript_CopyValuesFromContext @ 0x0061_96e0 => extern "C" fn copy_values(from: u32, to: u32, first: u32, n: u32) -> u32 {
        lb::copy_values(&mut mem(), from, to, first, n)
    }
    /// `PCallerScript`'s script function call `(L)`.
    Phyre_PScripting_PCallerScript_callFunction_00619e90 @ 0x0061_9e90 => extern "thiscall" fn caller_call(this: u32, l: u32) -> u32 {
        lb::caller_call(&mut mem(), this, l)
    }
    /// `PRingBuffer<PAsyncProcessBufferBlock>::allocateBlock(size)`.
    Phyre_PRingBuffer_Phyre_PScripting_PAsyncProcessBufferBlock__allocateBlock @ 0x005c_e6e0 => extern "thiscall" fn ring_allocate(ring: u32, size: u32) -> u32 {
        lb::ring_allocate(&mut mem(), ring, size)
    }
    /// The animation utility's clip switch `(clip)`.
    Phyre_PAnimation_PUtilityAnimation_setClip_00543150 @ 0x0054_3150 => extern "thiscall" fn set_clip(this: u32, clip: u32) -> u32 {
        let mut f = StackFrame::<2>::new();
        lb::set_clip(&mut mem(), this, clip, f.at())
    }
    /// The script's `Matrix(...)` constructor `(L)`.
    Phyre_PScripting_Matrix_new_00570e30 @ 0x0057_0e30 => extern "C" fn matrix_new(l: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        lb::matrix_new(&mut mem(), l, f.at())
    }
    /// The script's `Matrix.LookAt(eye, target, up)` `(L)`.
    Phyre_PScripting_Matrix_lookAt_00571200 @ 0x0057_1200 => extern "C" fn matrix_look_at(l: u32) -> u32 {
        let mut f = StackFrame::<30>::new();
        lb::matrix_look_at(&mut mem(), l, f.at())
    }
    /// The script's matrix `__index` `(L)`.
    Phyre_PScripting_MatrixIndexHandler_00571770 @ 0x0057_1770 => extern "C" fn matrix_index(l: u32) -> u32 {
        lb::matrix_index(&mut mem(), l)
    }
    /// The script's matrix `__mul` `(L)`.
    Phyre_PScripting_Matrix_mul_00571a60 @ 0x0057_1a60 => extern "C" fn matrix_mul(l: u32) -> u32 {
        let mut f = StackFrame::<32>::new();
        lb::matrix_mul(&mut mem(), l, f.at())
    }
    /// `Phyre::PScripting::VectorIndexHandler(L)`.
    Phyre_PScripting_VectorIndexHandler @ 0x0057_2e70 => extern "C" fn vector_index(l: u32) -> u32 {
        lb::vector_index(&mut mem(), l)
    }
    /// The occluder's plane through a point `(out, a, b, p)`.
    POccluderGeometry_plane_00a30eb0 @ 0x00a3_0eb0 => extern "C" fn occluder_plane(out: u32, a: u32, b: u32, p: u32) -> u32 {
        lb::occluder_plane(&mut mem(), out, a, b, p)
    }
    /// The occluder's edge quad `(a, b, d)` appended to a job's buffer.
    POccluderGeometry_pushEdgeQuad_00a30600 @ 0x00a3_0600 => extern "thiscall" fn occluder_quad(list: u32, a: u32, b: u32, d: u32) {
        lb::occluder_quad(&mut mem(), list, a, b, d);
    }
}
