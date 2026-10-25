//! `ffx-pother-a`: `PhyreEngine`'s other code the game reaches (M4 port
//! round 1, list `p3-other-a`): class registration, small constructors,
//! the class descriptors' create/destroy callbacks.
//!
//! Threads: class registration runs at start-up on the main thread; the
//! create/destroy callbacks run wherever objects are made (the loader
//! included). No shim keeps state: every frame is the caller's or
//! original code's.

use super::{mem, ported};
use ffx_pother_a::{class::bridge as class, object::bridge as object, physics::bridge as physics};

ported! {
    /// `Phyre_PBase_GetClassDescriptor()` -> `PBase`'s descriptor.
    Phyre_PBase_GetClassDescriptor @ 0x0043_5740 => extern "C" fn pbase_descriptor() -> u32 {
        object::pbase_descriptor(&mem())
    }
    /// `Phyre_PBase_Bind()`.
    Phyre_PBase_Bind @ 0x0043_5610 => extern "C" fn pbase_bind() -> u32 {
        class::pbase_bind(&mut mem())
    }
    /// `0x004359d0(class, name)` -> the member or 0.
    Phyre_PClassDescriptor_findMember_004359d0 @ 0x0043_59d0 => extern "thiscall" fn find_member(this: u32, name: u32) -> u32 {
        class::find_member(&mut mem(), this, name)
    }
    /// `0x00436350(name)` -> the entry or 0.
    Phyre_FindNamed_00436350 @ 0x0043_6350 => extern "C" fn find_named(name: u32) -> u32 {
        class::find_named(&mut mem(), name)
    }
    /// `0x0044cdf0(v)`: stores the global `0x00c0b95c`.
    Phyre_StoreGlobal_0044cdf0 @ 0x0044_cdf0 => extern "C" fn store_c0b95c(v: u32) -> u32 {
        object::set_global(&mut mem(), 0x00c0_b95c, v)
    }
    /// `0x0044ce00(v)`: stores the global `0x00c0b958`.
    Phyre_StoreGlobal_0044ce00 @ 0x0044_ce00 => extern "C" fn store_c0b958(v: u32) -> u32 {
        object::set_global(&mut mem(), 0x00c0_b958, v)
    }
    /// `0x0044d6a0(ref, descriptor)` -> `ref`.
    Phyre_PObjectRef_init_0044d6a0 @ 0x0044_d6a0 => extern "thiscall" fn ref_init(this: u32, d: u32) -> u32 {
        object::ref_init(&mut mem(), this, d)
    }
    /// `0x0044d730(ref)`.
    Phyre_PObjectRef_release_0044d730 @ 0x0044_d730 => extern "thiscall" fn ref_release(this: u32) -> u32 {
        object::ref_release(&mut mem(), this)
    }
    /// `0x0044dff0(p)` (create callback).
    Phyre_PBaseRef_create_0044dff0 @ 0x0044_dff0 => extern "stdcall" fn create_pbase_ref(p: u32) -> u8 {
        object::create_pbase_ref(&mut mem(), p)
    }
    /// `0x0044e050(p)` (destroy callback).
    Phyre_PObjectRef_destroyUnlink_0044e050 @ 0x0044_e050 => extern "stdcall" fn destroy_unlink(p: u32) -> u8 {
        object::destroy_unlink(&mut mem(), p)
    }
    /// `0x0044ede0(node)`.
    Phyre_PNode_release_0044ede0 @ 0x0044_ede0 => extern "thiscall" fn node_release(this: u32) -> u32 {
        object::node_release(&mut mem(), this)
    }
    /// `0x0044ffe0(p)` (create callback).
    Phyre_PEntity_createRefTree_0044ffe0 @ 0x0044_ffe0 => extern "stdcall" fn create_ref_tree(p: u32) -> u32 {
        object::create_ref_tree(&mut mem(), p)
    }
    /// `0x00450020(p)` (destroy callback).
    Phyre_PNode_destroy_00450020 @ 0x0045_0020 => extern "stdcall" fn destroy_node(p: u32) -> u8 {
        object::destroy_node(&mut mem(), p)
    }
    /// `0x0045b050(p)` (create callback).
    Phyre_PNameComponent_create_0045b050 @ 0x0045_b050 => extern "stdcall" fn create_listed_ref(p: u32) -> u8 {
        object::create_listed_ref(&mut mem(), p)
    }
    /// `0x0045b0d0(p)` (destroy callback).
    Phyre_PNameComponent_destroy_0045b0d0 @ 0x0045_b0d0 => extern "stdcall" fn destroy_listed_ref(p: u32) -> u8 {
        object::destroy_listed_ref(&mut mem(), p)
    }
    /// `0x00464400()` -> 0.
    Phyre_PShaderPass_InitAll_00464400 @ 0x0046_4400 => extern "C" fn shader_pass_init() -> u32 {
        class::shader_pass_init(&mut mem())
    }
    /// `0x004677d0(pair)` -> `pair`.
    Phyre_PPair_init_004677d0 @ 0x0046_77d0 => extern "thiscall" fn pair_init(this: u32) -> u32 {
        object::pair_init(&mut mem(), this)
    }
    /// `0x0047d2f0(table, key)` -> the entry or 0.
    Phyre_PClassView_findByKey_0047d2f0 @ 0x0047_d2f0 => extern "thiscall" fn find_by_key(this: u32, key: u32) -> u32 {
        class::find_by_key(&mut mem(), this, key)
    }
    /// `0x0047e4e0(view, offset)` -> the entry or 0.
    Phyre_PClassView_findByOffset_0047e4e0 @ 0x0047_e4e0 => extern "thiscall" fn find_by_offset(this: u32, offset: u32) -> u32 {
        class::find_by_offset(&mut mem(), this, offset)
    }
    /// `0x00503ad0(p)` (create callback).
    Phyre_CreatePair_00503ad0 @ 0x0050_3ad0 => extern "stdcall" fn create_pair_503ad0(p: u32) -> u32 {
        object::create_pair(&mut mem(), p)
    }
    /// `0x00503af0(p)` (destroy callback).
    Phyre_DestroyWhenIdle_00503af0 @ 0x0050_3af0 => extern "stdcall" fn destroy_when_idle(p: u32) -> u8 {
        object::destroy_when_idle(&mut mem(), p)
    }
    /// `0x00503b60(p)` -> 1.
    Phyre_CallbackTrue_00503b60 @ 0x0050_3b60 => extern "stdcall" fn callback_true_503b60(_p: u32) -> u8 {
        1
    }
    /// `Phyre_PLOD_PUtilityLOD_initialize()` -> 0.
    Phyre_PLOD_PUtilityLOD_initialize @ 0x0050_4260 => extern "C" fn lod_initialize() -> u32 {
        class::lod_initialize(&mut mem())
    }
    /// `0x00545840(name)` -> the entry or 0.
    Phyre_FindNamed_00545840 @ 0x0054_5840 => extern "C" fn find_named2(name: u32) -> u32 {
        class::find_named2(&mut mem(), name)
    }
    /// `0x00546040(p)` (create callback).
    Phyre_PLocatorRef_create_00546040 @ 0x0054_6040 => extern "stdcall" fn create_locator_ref(p: u32) -> u32 {
        object::create_locator_ref(&mut mem(), p)
    }
    /// `0x00546070(p)` (destroy callback).
    Phyre_DestroyOwnedPtr_00546070 @ 0x0054_6070 => extern "stdcall" fn destroy_owned_ptr(p: u32) -> u8 {
        object::destroy_owned_ptr(&mut mem(), p)
    }
    /// `0x00547d20(this, flags)` (scalar deleting destructor) -> `this`.
    Phyre_ScalarDelete_00547d20 @ 0x0054_7d20 => extern "thiscall" fn scalar_delete_547d20(this: u32, flags: u32) -> u32 {
        object::scalar_delete(&mut mem(), 0x0054_7580, this, flags)
    }
    /// `0x00549d50(p)` (create callback).
    Phyre_CreatePair_00549d50 @ 0x0054_9d50 => extern "stdcall" fn create_pair_549d50(p: u32) -> u32 {
        object::create_pair(&mut mem(), p)
    }
    /// `0x00549d70(p)` (destroy callback).
    Phyre_DestroyCall_00549d70 @ 0x0054_9d70 => extern "stdcall" fn destroy_call_549d70(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x0054_72c0, p)
    }
    /// `0x00549dc0(p)` (destroy callback).
    Phyre_DestroyArray_00549dc0 @ 0x0054_9dc0 => extern "stdcall" fn destroy_array_549dc0(p: u32) -> u8 {
        object::destroy_array(&mut mem(), p)
    }
    /// `0x00549e00(p)` (destroy callback).
    Phyre_DestroyVirtual_00549e00 @ 0x0054_9e00 => extern "stdcall" fn destroy_virtual_549e00(p: u32) -> u8 {
        object::destroy_virtual(&mut mem(), p)
    }
    /// `0x0054fda0(p)` (create callback).
    Phyre_CreatePair_0054fda0 @ 0x0054_fda0 => extern "stdcall" fn create_pair_54fda0(p: u32) -> u32 {
        object::create_pair(&mut mem(), p)
    }
    /// `0x0054fdc0(p)` (destroy callback).
    Phyre_DestroyCall_0054fdc0 @ 0x0054_fdc0 => extern "stdcall" fn destroy_call_54fdc0(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x0054_e0f0, p)
    }
    /// `0x0054fdf0(p)` (destroy callback).
    Phyre_DestroyArray_0054fdf0 @ 0x0054_fdf0 => extern "stdcall" fn destroy_array_54fdf0(p: u32) -> u8 {
        object::destroy_array_gt1(&mut mem(), p, 0)
    }
    /// `0x0054fe30(p)` (destroy callback).
    Phyre_DestroyArray_0054fe30 @ 0x0054_fe30 => extern "stdcall" fn destroy_array_54fe30(p: u32) -> u8 {
        object::destroy_array_gt1(&mut mem(), p, 4)
    }
    /// `0x00550ed0(out)` -> `al` whether the trigger callback types exist.
    Phyre_PGameplay_PTriggerCallbackTypes_00550ed0 @ 0x0055_0ed0 => extern "C" fn callback_types(out: u32) -> u32 {
        class::callback_types(&mut mem(), out)
    }
    /// `0x005511d0(out)`: `0x00550ed0(out)`'s `eax`.
    Phyre_PGameplay_PTriggerCallbackTypes_005511d0 @ 0x0055_11d0 => extern "stdcall" fn callback_types_thunk(out: u32) -> u32 {
        class::callback_types(&mut mem(), out)
    }
    /// `0x005511f0(p)` -> 1.
    Phyre_CallbackTrue_005511f0 @ 0x0055_11f0 => extern "stdcall" fn callback_true_5511f0(_p: u32) -> u8 {
        1
    }
    /// `0x00552850(p)` (destroy callback).
    Phyre_DestroyRef_00552850 @ 0x0055_2850 => extern "stdcall" fn destroy_ref(p: u32) -> u8 {
        object::destroy_ref(&mut mem(), p)
    }
    /// `PScriptableComponent(this, descriptor)` -> `this`.
    Phyre_PGameplay_PScriptableComponent_PScriptableComponent @ 0x0055_2c00 => extern "thiscall" fn scriptable_init(this: u32, d: u32) -> u32 {
        object::scriptable_init(&mut mem(), this, d)
    }
    /// `0x00553d20(this, flags)` (scalar deleting destructor) -> `this`.
    Phyre_ScalarDelete_00553d20 @ 0x0055_3d20 => extern "thiscall" fn scalar_delete_553d20(this: u32, flags: u32) -> u32 {
        object::scalar_delete(&mut mem(), 0x0055_3a70, this, flags)
    }
    /// `0x00554690(p)` (destroy callback).
    Phyre_DestroyVirtual_00554690 @ 0x0055_4690 => extern "stdcall" fn destroy_virtual_554690(p: u32) -> u8 {
        object::destroy_virtual(&mut mem(), p)
    }
    /// `0x00555c30(p)` (destroy callback).
    Phyre_DestroyVirtual_00555c30 @ 0x0055_5c30 => extern "stdcall" fn destroy_virtual_555c30(p: u32) -> u8 {
        object::destroy_virtual(&mut mem(), p)
    }
    /// `0x00556980(p)` (destroy callback).
    Phyre_DestroyRefC_00556980 @ 0x0055_6980 => extern "stdcall" fn destroy_ref_c(p: u32) -> u8 {
        object::destroy_ref_c(&mut mem(), p)
    }
    /// `0x00570670(descriptor)` -> `descriptor`.
    Phyre_PClassDescriptorBase_PClassDescriptorBase_00570670 @ 0x0057_0670 => extern "thiscall" fn base_descriptor_init(this: u32) -> u32 {
        class::base_descriptor_init(&mut mem(), this)
    }
    /// `0x00570830(this, a)` -> `this`.
    Phyre_Quad_init_00570830 @ 0x0057_0830 => extern "thiscall" fn quad_init(this: u32, a: u32) -> u32 {
        object::quad_init(&mut mem(), this, a)
    }
    /// `PEnumDescriptor::setTokens(this, tokens, count)` -> `count`.
    Phyre_PEnumDescriptor_setTokens @ 0x0057_0a60 => extern "thiscall" fn enum_set_tokens(this: u32, tokens: u32, count: u32) -> u32 {
        object::enum_set_tokens(&mut mem(), this, tokens, count)
    }
    /// `PRedBlackTree::orphanAllContents(tree)`.
    Phyre_PRedBlackTree_orphanAllContents @ 0x0057_4970 => extern "thiscall" fn tree_orphan_all(this: u32) {
        object::tree_orphan_all(&mut mem(), this);
    }
    /// `PIClassDataMemberDynamicType(this, member)` -> `this`.
    Phyre_PIClassDataMemberDynamicType_PIClassDataMemberDynamicType @ 0x0057_5440 => extern "thiscall" fn dynamic_type_init(this: u32, member: u32) -> u32 {
        object::dynamic_type_init(&mut mem(), this, member)
    }
    /// `PAnnotation(this, a, b, c)` -> `this`.
    Phyre_PAnnotation_PAnnotation @ 0x0057_5840 => extern "thiscall" fn annotation_init(this: u32, a: u32, b: u32, c: u32) -> u32 {
        object::annotation_init(&mut mem(), this, a, b, c)
    }
    /// `0x00575870(this, a)` -> `this`.
    Phyre_Pair_set_00575870 @ 0x0057_5870 => extern "thiscall" fn pair_set(this: u32, a: u32) -> u32 {
        object::pair_set(&mut mem(), this, a)
    }
    /// `PAnnotatable::addAnnotation(list, annotation)`.
    Phyre_PAnnotatable_addAnnotation @ 0x0057_5c10 => extern "thiscall" fn add_annotation(this: u32, a: u32) -> u32 {
        object::add_annotation(&mut mem(), this, a)
    }
    /// `0x00587220()`: registers `0x00cb3de0`.
    Phyre_RegisterClass_00587220 @ 0x0058_7220 => extern "C" fn register_587220() -> u32 {
        class::register(&mut mem(), 0x00cb_3de0)
    }
    /// `0x00587240()`: registers `0x00cb3d48`.
    Phyre_RegisterClass_00587240 @ 0x0058_7240 => extern "C" fn register_587240() -> u32 {
        class::register(&mut mem(), 0x00cb_3d48)
    }
    /// `0x00587260()`: registers `0x00cb3b68`.
    Phyre_RegisterClass_00587260 @ 0x0058_7260 => extern "C" fn register_587260() -> u32 {
        class::register(&mut mem(), 0x00cb_3b68)
    }
    /// `0x00587280()`: registers `0x00cb3c00`.
    Phyre_RegisterClass_00587280 @ 0x0058_7280 => extern "C" fn register_587280() -> u32 {
        class::register(&mut mem(), 0x00cb_3c00)
    }
    /// `0x005872a0()`: registers `0x00cb3908`.
    Phyre_RegisterClass_005872a0 @ 0x0058_72a0 => extern "C" fn register_5872a0() -> u32 {
        class::register(&mut mem(), 0x00cb_3908)
    }
    /// `0x005872c0()`: registers `0x00cb37d8`.
    Phyre_RegisterClass_005872c0 @ 0x0058_72c0 => extern "C" fn register_5872c0() -> u32 {
        class::register(&mut mem(), 0x00cb_37d8)
    }
    /// `0x005872e0()`: registers `0x00cb3870`.
    Phyre_RegisterClass_005872e0 @ 0x0058_72e0 => extern "C" fn register_5872e0() -> u32 {
        class::register(&mut mem(), 0x00cb_3870)
    }
    /// `0x00587cf0()`: registers `0x00cb3cb0`.
    Phyre_RegisterClass_00587cf0 @ 0x0058_7cf0 => extern "C" fn register_587cf0() -> u32 {
        class::register(&mut mem(), 0x00cb_3cb0)
    }
    /// `0x00587d10()`: registers `0x00cb39a0`.
    Phyre_RegisterClass_00587d10 @ 0x0058_7d10 => extern "C" fn register_587d10() -> u32 {
        class::register(&mut mem(), 0x00cb_39a0)
    }
    /// `0x00587d30()`: registers `0x00cb3a38`.
    Phyre_RegisterClass_00587d30 @ 0x0058_7d30 => extern "C" fn register_587d30() -> u32 {
        class::register(&mut mem(), 0x00cb_3a38)
    }
    /// `0x00587d50()`: registers `0x00cb3ad0`.
    Phyre_RegisterClass_00587d50 @ 0x0058_7d50 => extern "C" fn register_587d50() -> u32 {
        class::register(&mut mem(), 0x00cb_3ad0)
    }
    /// `0x005f7a10(p)`: `PPhysicsBox`'s create callback.
    Phyre_PPhysics_PPhysicsBox_create_005f7a10 @ 0x005f_7a10 => extern "stdcall" fn create_5f7a10(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::BOX) as u8
    }
    /// `0x005f7a50(p)`: `PPhysicsBoxBase`'s create callback.
    Phyre_PPhysics_PPhysicsBoxBase_create_005f7a50 @ 0x005f_7a50 => extern "stdcall" fn create_5f7a50(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::BOX_BASE) as u8
    }
    /// `0x005f7a90(p)`: `PPhysicsBoxBullet`'s create callback.
    Phyre_PPhysics_PPhysicsBoxBullet_create_005f7a90 @ 0x005f_7a90 => extern "stdcall" fn create_5f7a90(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::BOX_BULLET) as u8
    }
    /// `0x005f7ad0(p)`: `PPhysicsCapsule`'s create callback.
    Phyre_PPhysics_PPhysicsCapsule_create_005f7ad0 @ 0x005f_7ad0 => extern "stdcall" fn create_5f7ad0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CAPSULE) as u8
    }
    /// `0x005f7b20(p)`: `PPhysicsCapsuleBase`'s create callback.
    Phyre_PPhysics_PPhysicsCapsuleBase_create_005f7b20 @ 0x005f_7b20 => extern "stdcall" fn create_5f7b20(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CAPSULE_BASE) as u8
    }
    /// `0x005f7b70(p)`: `PPhysicsCapsuleBullet`'s create callback.
    Phyre_PPhysics_PPhysicsCapsuleBullet_create_005f7b70 @ 0x005f_7b70 => extern "stdcall" fn create_5f7b70(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CAPSULE_BULLET) as u8
    }
    /// `0x005f7cd0(p)`: `PPhysicsCylinder`'s create callback.
    Phyre_PPhysics_PPhysicsCylinder_create_005f7cd0 @ 0x005f_7cd0 => extern "stdcall" fn create_5f7cd0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CYLINDER) as u8
    }
    /// `0x005f7d20(p)`: `PPhysicsCylinderBase`'s create callback.
    Phyre_PPhysics_PPhysicsCylinderBase_create_005f7d20 @ 0x005f_7d20 => extern "stdcall" fn create_5f7d20(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CYLINDER_BASE) as u8
    }
    /// `0x005f7d60(p)`: `PPhysicsCylinderBullet`'s create callback.
    Phyre_PPhysics_PPhysicsCylinderBullet_create_005f7d60 @ 0x005f_7d60 => extern "stdcall" fn create_5f7d60(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::CYLINDER_BULLET) as u8
    }
    /// `0x005f7e50(p)`: `PPhysicsMesh`'s create callback.
    Phyre_PPhysics_PPhysicsMesh_create_005f7e50 @ 0x005f_7e50 => extern "stdcall" fn create_5f7e50(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::MESH) as u8
    }
    /// `0x005f7ea0(p)`: `PPhysicsMeshBase`'s create callback.
    Phyre_PPhysics_PPhysicsMeshBase_create_005f7ea0 @ 0x005f_7ea0 => extern "stdcall" fn create_5f7ea0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::MESH_BASE) as u8
    }
    /// `0x005f7ee0(p)`: `PPhysicsMeshBullet`'s create callback.
    Phyre_PPhysics_PPhysicsMeshBullet_create_005f7ee0 @ 0x005f_7ee0 => extern "stdcall" fn create_5f7ee0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::MESH_BULLET) as u8
    }
    /// `0x005f7f60(p)`: `PPhysicsPlane`'s create callback.
    Phyre_PPhysics_PPhysicsPlane_create_005f7f60 @ 0x005f_7f60 => extern "stdcall" fn create_5f7f60(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::PLANE) as u8
    }
    /// `0x005f7fb0(p)`: `PPhysicsPlaneBase`'s create callback.
    Phyre_PPhysics_PPhysicsPlaneBase_create_005f7fb0 @ 0x005f_7fb0 => extern "stdcall" fn create_5f7fb0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::PLANE_BASE) as u8
    }
    /// `0x005f7ff0(p)`: `PPhysicsPlaneBullet`'s create callback.
    Phyre_PPhysics_PPhysicsPlaneBullet_create_005f7ff0 @ 0x005f_7ff0 => extern "stdcall" fn create_5f7ff0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::PLANE_BULLET) as u8
    }
    /// `0x005f8170(p)`: `PPhysicsShape`'s create callback.
    Phyre_PPhysics_PPhysicsShape_create_005f8170 @ 0x005f_8170 => extern "stdcall" fn create_5f8170(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::SHAPE) as u8
    }
    /// `0x005f81a0(p)`: `PPhysicsShapeBullet`'s create callback.
    Phyre_PPhysics_PPhysicsShapeBullet_create_005f81a0 @ 0x005f_81a0 => extern "stdcall" fn create_5f81a0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::SHAPE_BULLET) as u8
    }
    /// `0x005f81d0(p)`: `PPhysicsSphere`'s create callback.
    Phyre_PPhysics_PPhysicsSphere_create_005f81d0 @ 0x005f_81d0 => extern "stdcall" fn create_5f81d0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::SPHERE) as u8
    }
    /// `0x005f8210(p)`: `PPhysicsSphereBase`'s create callback.
    Phyre_PPhysics_PPhysicsSphereBase_create_005f8210 @ 0x005f_8210 => extern "stdcall" fn create_5f8210(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::SPHERE_BASE) as u8
    }
    /// `0x005f8250(p)`: `PPhysicsSphereBullet`'s create callback.
    Phyre_PPhysics_PPhysicsSphereBullet_create_005f8250 @ 0x005f_8250 => extern "stdcall" fn create_5f8250(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::SPHERE_BULLET) as u8
    }
    /// `0x005f8290(p)`: `PPhysicsTaperedCapsule`'s create callback.
    Phyre_PPhysics_PPhysicsTaperedCapsule_create_005f8290 @ 0x005f_8290 => extern "stdcall" fn create_5f8290(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::TAPERED_CAPSULE) as u8
    }
    /// `0x005f82e0(p)`: `PPhysicsTaperedCylinder`'s create callback.
    Phyre_PPhysics_PPhysicsTaperedCylinder_create_005f82e0 @ 0x005f_82e0 => extern "stdcall" fn create_5f82e0(p: u32) -> u8 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::TAPERED_CYLINDER) as u8
    }
    /// `0x005f79f0(p)` (create callback).
    Phyre_PPhysics_CreatePair_005f79f0 @ 0x005f_79f0 => extern "stdcall" fn create_5f79f0(p: u32) -> u32 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::PAIR)
    }
    /// `0x005f7db0(p)` (create callback).
    Phyre_PPhysics_CreateHeader_005f7db0 @ 0x005f_7db0 => extern "stdcall" fn create_5f7db0(p: u32) -> u32 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::BYTE_HEADER)
    }
    /// `0x005f7e30(p)` (create callback).
    Phyre_PPhysics_CreateVec3Zero_005f7e30 @ 0x005f_7e30 => extern "stdcall" fn create_5f7e30(p: u32) -> u32 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::VEC3_ZERO)
    }
    /// `0x005f7f30(p)` (create callback).
    Phyre_PPhysics_CreateTriple_005f7f30 @ 0x005f_7f30 => extern "stdcall" fn create_5f7f30(p: u32) -> u32 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::TRIPLE)
    }
    /// `0x005f8390(p)` (create callback).
    Phyre_PPhysics_CreateTailPair_005f8390 @ 0x005f_8390 => extern "stdcall" fn create_5f8390(p: u32) -> u32 {
        physics::create(&mut mem(), p, &ffx_pother_a::physics::TAIL_PAIR)
    }
    /// `0x005f7bc0(p)` (destroy callback): `0x005dff30(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f7bc0 @ 0x005f_7bc0 => extern "stdcall" fn destroy_5f7bc0(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005d_ff30, p)
    }
    /// `0x005f7be0(p)` (destroy callback): `0x005e01c0(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f7be0 @ 0x005f_7be0 => extern "stdcall" fn destroy_5f7be0(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_01c0, p)
    }
    /// `0x005f7c30(p)` (destroy callback): `0x005e0050(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f7c30 @ 0x005f_7c30 => extern "stdcall" fn destroy_5f7c30(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0050, p)
    }
    /// `0x005f7c80(p)` (destroy callback): `0x005e01c0(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f7c80 @ 0x005f_7c80 => extern "stdcall" fn destroy_5f7c80(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_01c0, p)
    }
    /// `0x005f7de0(p)` (destroy callback): `0x005e0330(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f7de0 @ 0x005f_7de0 => extern "stdcall" fn destroy_5f7de0(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0330, p)
    }
    /// `0x005f80b0(p)` (destroy callback): `0x005e0590(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f80b0 @ 0x005f_80b0 => extern "stdcall" fn destroy_5f80b0(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0590, p)
    }
    /// `0x005f8330(p)` (destroy callback): `0x005e0970(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f8330 @ 0x005f_8330 => extern "stdcall" fn destroy_5f8330(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0970, p)
    }
    /// `0x005f8350(p)` (destroy callback): `0x005e0900(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f8350 @ 0x005f_8350 => extern "stdcall" fn destroy_5f8350(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0900, p)
    }
    /// `0x005f8370(p)` (destroy callback): `0x005e0970(p)` unless null.
    Phyre_PPhysics_DestroyCall_005f8370 @ 0x005f_8370 => extern "stdcall" fn destroy_5f8370(p: u32) -> u8 {
        object::destroy_call(&mut mem(), 0x005e_0970, p)
    }
    /// `0x005f8040(p)` (destroy callback).
    Phyre_PPhysics_DestroyBody_005f8040 @ 0x005f_8040 => extern "stdcall" fn destroy_5f8040(p: u32) -> u8 {
        physics::destroy_body(&mut mem(), p)
    }
    /// `0x005f8100(p)` (destroy callback).
    Phyre_PPhysics_DestroyBody_005f8100 @ 0x005f_8100 => extern "stdcall" fn destroy_5f8100(p: u32) -> u8 {
        physics::destroy_body(&mut mem(), p)
    }
}
