//! M5 finish, WP5 part 2 (`ffx_prender::wp5_small`): small originals
//! after ported symbols. Parity tests:
//! `crates/ffx-oracle/tests/wp5_string.rs` (`wp5b_small_*`).
//!
//! Threads: any. No shim keeps state.

use ffx_prender::wp5_small as s;

use super::{StackFrame, mem, ported};

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
    /// `NamedPair_copy_00699b70(&src)`: the word and a copy of the name; `this`.
    NamedPair_copy_00699b70 @ 0x0069_9b70 => extern "thiscall" fn sm_00699b70(this: u32, src: u32) -> u32 {
        s::named_pair_copy(&mut mem(), this, src)
    }
    /// `Instance_006dc800_reportMapLoad_006dcd50(name)`: the `"MapLoad"` report when on.
    Instance_006dc800_reportMapLoad_006dcd50 @ 0x006d_cd50 => extern "thiscall" fn sm_006dcd50(this: u32, name: u32) {
        let mut frame = super::StackFrame::<0x40>::new();
        s::report_map_load(&mut mem(), this, name, frame.at());
    }
    /// `CharacterSwitchTweakUtil_CharacterSwitchTweakUtil`: the tweak tool's constructor; `this`.
    CharacterSwitchTweakUtil_CharacterSwitchTweakUtil @ 0x006b_55d0 => extern "thiscall" fn sm_006b55d0(this: u32) -> u32 {
        s::char_switch_ctor(&mut mem(), this)
    }
    /// `CharacterSwitchTweakUtil_CreateInstance`: the tweak tool created once.
    CharacterSwitchTweakUtil_CreateInstance @ 0x006b_5eb0 => extern "C" fn sm_006b5eb0() -> u32 {
        s::char_switch_create(&mut mem())
    }
    /// `AtlBaseModule_ctor_009d9c20`: ATL's base module constructed; `this`.
    AtlBaseModule_ctor_009d9c20 @ 0x009d_9c20 => extern "thiscall" fn sm_009d9c20(this: u32) -> u32 {
        s::atl_base_module_ctor(&mut mem(), this)
    }
    /// `ComCriticalSection_Init_009d9dfe` (`CComCriticalSection::Init`): the lock initialised; an `HRESULT`.
    ComCriticalSection_Init_009d9dfe @ 0x009d_9dfe => extern "thiscall" fn sm_009d9dfe(this: u32) -> u32 {
        s::atl_cs_init(&mut mem(), this)
    }
    /// `PtrArray_reserve_00638240(n)`: room for `n` words.
    PtrArray_reserve_00638240 @ 0x0063_8240 => extern "thiscall" fn sm_00638240(this: u32, n: u32) -> u32 {
        s::ptr_array_reserve(&mut mem(), this, n)
    }
    /// `Table256_ctor_009f2510`: 256 (0, 1.0, 0.0) entries and a zero count; `this`.
    Table256_ctor_009f2510 @ 0x009f_2510 => extern "thiscall" fn sm_009f2510(this: u32) -> u32 {
        s::table256_ctor(&mut mem(), this)
    }
    /// `PSceneRenderPassType_assignIndices_004e62c0()`: the pass types indexed, the effect variants' over-allocation; 0x12 once the slots were assigned, else 0.
    PSceneRenderPassType_assignIndices_004e62c0 @ 0x004e_62c0 => extern "C" fn sm_004e62c0() -> u32 {
        s::pass_assign_indices(&mut mem())
    }
    /// `Phyre_PArray_PAnimationConstantChannel_dtor_00513240`: owned elements destroyed and freed, the array cleared; its `eax`.
    Phyre_PArray_PAnimationConstantChannel_dtor_00513240 @ 0x0051_3240 => extern "thiscall" fn sm_00513240(this: u32) -> u32 {
        s::constant_channel_array_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationClip_dtor_005133f0`: the clip's name and arrays released; its `eax`.
    Phyre_PAnimation_PAnimationClip_dtor_005133f0 @ 0x0051_33f0 => extern "thiscall" fn sm_005133f0(this: u32) -> u32 {
        s::animation_clip_dtor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationChannel_ctor_0050d150`: the base descriptor and eight zero words; `this`.
    Phyre_PAnimation_PAnimationChannel_ctor_0050d150 @ 0x0050_d150 => extern "thiscall" fn sm_0050d150(this: u32) -> u32 {
        s::channel_ctor(&mut mem(), this)
    }
    /// `Phyre_PAnimation_PAnimationChannel_dtor_0050d1d0`: `+0x1c` cleared, the owned `+0x14` freed; `al` as the original's.
    Phyre_PAnimation_PAnimationChannel_dtor_0050d1d0 @ 0x0050_d1d0 => extern "thiscall" fn sm_0050d1d0(this: u32) -> u32 {
        s::channel_dtor(&mut mem(), this)
    }
    /// `PPhysicsDefaults_ctor_005e0050`: the physics defaults and two identity matrices (frame `ebp - 0x40`); `this`.
    PPhysicsDefaults_ctor_005e0050 @ 0x005e_0050 => extern "thiscall" fn sm_005e0050(this: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        s::physics_defaults_ctor(&mut mem(), this, f.at())
    }
    /// `PPhysicsDefaults_ctor_005e01c0`: 0x005e0050's defaults, four zero words and an identity matrix at `+0x11c` (frame `ebp - 0x40`); `this`.
    PPhysicsDefaults_ctor_005e01c0 @ 0x005e_01c0 => extern "thiscall" fn sm_005e01c0(this: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        s::physics_defaults_ctor_b(&mut mem(), this, f.at())
    }
    /// `PPhysicsHeader_ctor_005e0330`: the header words, a one-word member and two semaphores constructed; `this`.
    PPhysicsHeader_ctor_005e0330 @ 0x005e_0330 => extern "thiscall" fn sm_005e0330(this: u32) -> u32 {
        s::physics_header_ctor(&mut mem(), this)
    }
    /// `PPhysicsBody_ctor_005e0590`: an identity 3x4 at `+4` and the body defaults (frame `ebp - 0x50`); `this`.
    PPhysicsBody_ctor_005e0590 @ 0x005e_0590 => extern "thiscall" fn sm_005e0590(this: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        s::physics_body_ctor(&mut mem(), this, f.at())
    }
    /// `Phyre_PPhysics_PPhysicsRigidBodyBase_dtor_005e2660`: the owned name and shared array freed, the array cleared; its `eax`.
    Phyre_PPhysics_PPhysicsRigidBodyBase_dtor_005e2660 @ 0x005e_2660 => extern "thiscall" fn sm_005e2660(this: u32) -> u32 {
        s::rigid_body_base_dtor(&mut mem(), this)
    }
    /// `Phyre_PPhysics_PPhysicsRigidBody_dtor_005e2570`: the body taken out of its model and world, its shape and `+0xe0` deleted, then the base destructor; its `eax`.
    Phyre_PPhysics_PPhysicsRigidBody_dtor_005e2570 @ 0x005e_2570 => extern "thiscall" fn sm_005e2570(this: u32) -> u32 {
        s::rigid_body_dtor(&mut mem(), this)
    }
    /// `PPhysicsWorldSettings_ctor_005e0900`: gravity -9.8, bounds -5000..5000 and zeros; `this`.
    PPhysicsWorldSettings_ctor_005e0900 @ 0x005e_0900 => extern "thiscall" fn sm_005e0900(this: u32) -> u32 {
        s::world_settings_ctor(&mut mem(), this)
    }
    /// `PPhysicsWorldSettings_ctor_005e0970`: 0x005e0900's settings, zeros, the all-ones block and its two words; `this`.
    PPhysicsWorldSettings_ctor_005e0970 @ 0x005e_0970 => extern "thiscall" fn sm_005e0970(this: u32) -> u32 {
        s::world_settings_ctor_b(&mut mem(), this)
    }
    /// `Phyre_PPhysics_PPhysicsWorldBullet_dtor_005e2950`: the parts released, four arrays freed and cleared; its `eax`.
    Phyre_PPhysics_PPhysicsWorldBullet_dtor_005e2950 @ 0x005e_2950 => extern "thiscall" fn sm_005e2950(this: u32) -> u32 {
        s::world_bullet_dtor(&mut mem(), this)
    }
    /// `Phyre_PPhysics_PPhysicsWorld_dtor_005e28d0`: group resources released, controllers removed, then the Bullet part's destructor; its `eax`.
    Phyre_PPhysics_PPhysicsWorld_dtor_005e28d0 @ 0x005e_28d0 => extern "thiscall" fn sm_005e28d0(this: u32) -> u32 {
        s::world_dtor(&mut mem(), this)
    }
    /// `Phyre_PPhysics_PPhysicsShapeBase_ctor_005e0740`: the vtable, an identity 3x4 at `+0x14` (frame `ebp - 0x40`), the scale 1 and `+0x54` 8; `this`.
    Phyre_PPhysics_PPhysicsShapeBase_ctor_005e0740 @ 0x005e_0740 => extern "thiscall" fn sm_005e0740(this: u32) -> u32 {
        let mut f = StackFrame::<16>::new();
        s::shape_base_ctor(&mut mem(), this, f.at())
    }
    /// `Phyre_PPhysics_PPhysicsMeshBullet_dtor_005e2360`: three owned objects deleted, the base vtable restored; the last deletion's `eax` (0 for none; see the bridge).
    Phyre_PPhysics_PPhysicsMeshBullet_dtor_005e2360 @ 0x005e_2360 => extern "thiscall" fn sm_005e2360(this: u32) -> u32 {
        s::mesh_bullet_dtor(&mut mem(), this)
    }
}
