//! M5 finish, WP5 part 2 (`ffx_prender::wp5_small`): small originals
//! after ported symbols. Parity tests:
//! `crates/ffx-oracle/tests/wp5_string.rs` (`wp5b_small_*`).
//!
//! Threads: any. No shim keeps state.

use ffx_prender::wp5_small as s;

use super::{mem, ported};

ported! {
    /// `Phyre_PArray_count_004320c0`: the count without bit 31.
    Phyre_PArray_count_004320c0 @ 0x0043_20c0 => extern "thiscall" fn sm_004320c0(this: u32) -> u32 {
        s::parray_count(&mem(), this)
    }
    /// `std_List_buyNode_00654f00(next, prev)`: a 12-byte list node.
    std_List_buyNode_00654f00 @ 0x0065_4f00 => extern "stdcall" fn sm_00654f00(next: u32, prev: u32) -> u32 {
        s::list_buy_node(&mut mem(), next, prev)
    }
    /// `graphicAbmapFree_00684ae0`: `g_GraphicAbmap` freed and cleared.
    graphicAbmapFree_00684ae0 @ 0x0068_4ae0 => extern "C" fn sm_00684ae0() -> u32 {
        s::abmap_free(&mut mem())
    }
    /// `ZeroThreeWords_006906b0`: three words zeroed; `this`.
    ZeroThreeWords_006906b0 @ 0x0069_06b0 => extern "thiscall" fn sm_006906b0(this: u32) -> u32 {
        s::zero_three_words(&mut mem(), this)
    }
    /// `ClassCharacterInstance_deleteObject_006a1550`: the cluster object deleted.
    ClassCharacterInstance_deleteObject_006a1550 @ 0x006a_1550 => extern "thiscall" fn sm_006a1550(this: u32) -> u32 {
        s::character_delete_object(&mut mem(), this)
    }
    /// `ClassCharacterInstance_deleteMeshInstance_006a1570`: the mesh instance deleted.
    ClassCharacterInstance_deleteMeshInstance_006a1570 @ 0x006a_1570 => extern "thiscall" fn sm_006a1570(this: u32) -> u32 {
        s::character_delete_mesh_instance(&mut mem(), this)
    }
    /// `ClearWord10_007038f0`: `+0x10` zeroed; `this`.
    ClearWord10_007038f0 @ 0x0070_38f0 => extern "thiscall" fn sm_007038f0(this: u32) -> u32 {
        s::clear_word10(&mut mem(), this)
    }
    /// `JobWorker_ctor_00703960`: a job worker's fields and vtable; `this`.
    JobWorker_ctor_00703960 @ 0x0070_3960 => extern "thiscall" fn sm_00703960(this: u32) -> u32 {
        s::job_worker_ctor(&mut mem(), this)
    }
    /// `cd_resetChannels_0076dcb0`: the CD channels and reader state cleared.
    cd_resetChannels_0076dcb0 @ 0x0076_dcb0 => extern "C" fn sm_0076dcb0() -> u32 {
        s::cd_reset_channels(&mut mem())
    }
    /// `std_Tree_BuyHeadNode_00654ea0`: the same code as `std_Tree_BuyHeadNode_00654ed0`.
    std_Tree_BuyHeadNode_00654ea0 @ 0x0065_4ea0 => extern "C" fn sm_00654ea0() -> u32 {
        ffx_prender::glue3::bridge::tree_buy_head_node(&mut mem())
    }
    /// `FreeCamera_handleInput(dt, x)`: the free camera's toggles, mouse capture and step.
    FreeCamera_handleInput @ 0x0068_0030 => extern "thiscall" fn sm_00680030(this: u32, dt: u32, x: u32) {
        let mut frame = super::StackFrame::<{ ffx_prender::wp5_camera::FRAME_WORDS }>::new();
        ffx_prender::wp5_camera::handle_input(&mut mem(), this, dt, x, frame.at());
    }
    /// `FreeCamera_setType(t)`: the capture counter set; 0 frees the cursor.
    FreeCamera_setType @ 0x0068_0410 => extern "thiscall" fn sm_00680410(this: u32, t: u32) -> u32 {
        ffx_prender::wp5_camera::set_type(&mut mem(), this, t)
    }
    /// `asyncFFXInitialize(out, &a, &b)`: `*out = 0x00822490(*a, *b)`.
    asyncFFXInitialize @ 0x0063_6410 => extern "C" fn sm_00636410(out: u32, a: u32, b: u32) -> u32 {
        s::async_ffx_initialize(&mut mem(), out, a, b)
    }
    /// `ClassCharacterInstance_destroyMeshes`: the mesh instances deleted from the scene.
    ClassCharacterInstance_destroyMeshes @ 0x006a_15e0 => extern "thiscall" fn sm_006a15e0(this: u32) {
        s::character_destroy_meshes(&mut mem(), this);
    }
    /// `ClassCharacterInstance_destroyClothInstances`: the cloths released and freed.
    ClassCharacterInstance_destroyClothInstances @ 0x006a_1590 => extern "thiscall" fn sm_006a1590(this: u32) {
        s::character_destroy_cloths(&mut mem(), this);
    }
    /// `spendSlv(who, n)`: -1, or 0 with the level spent into the gauge.
    spendSlv @ 0x0078_6ef0 => extern "C" fn sm_00786ef0(who: u32, n: u32) -> u32 {
        s::spend_slv(&mut mem(), who, n)
    }
}
