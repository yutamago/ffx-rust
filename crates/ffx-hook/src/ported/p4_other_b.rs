//! `ffx-pother-b` (M4 port list `p4-other-b`): `PhyreEngine` code outside
//! rendering.
//!
//! Threads: the class-descriptor callbacks run where descriptors build
//! default buffers and instances (start-up, the loader's threads); the
//! registrations at start-up; the destructors and constructors wherever
//! their objects live. No shim keeps state.

use super::{mem, ported};
use ffx_pother_b::{object, physics};

ported! {
    /// `constructInstance(desc, obj)` -> 1: `[+0] = 0, [+4] = Phyre_PBase_GetClassDescriptor(), [+8] = 0, [+0xc] = the same, [+0x10] = 0.0f when obj is non-null`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PPhysics_PPhysicsCallbackData__constructInstance @ 0x005f_83b0 => extern "thiscall" fn construct_p_physics_callback_data_5f83b0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::construct_callback_data(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `frees the owned storage [+4] of more than one element (header [+0]: count, bit 31 not owned), clears both`.
    Phyre_PClassDescriptorForType_Phyre_PSharray_Phyre_PPhysics_PPhysicsShape__destructInstance @ 0x005f_8600 => extern "thiscall" fn destruct_p_physics_shape_5f8600(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_shape_array(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsBox__destructInstance @ 0x005f_8640 => extern "thiscall" fn destruct_p_physics_box_5f8640(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsBoxBase__destructInstance @ 0x005f_8670 => extern "thiscall" fn destruct_p_physics_box_base_5f8670(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsBoxBullet__destructInstance @ 0x005f_86a0 => extern "thiscall" fn destruct_p_physics_box_bullet_5f86a0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `nothing to destroy (mov al, 1)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCallbackData__destructInstance @ 0x005f_86d0 => extern "thiscall" fn destruct_p_physics_callback_data_5f86d0(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCapsule__destructInstance @ 0x005f_86e0 => extern "thiscall" fn destruct_p_physics_capsule_5f86e0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCapsuleBase__destructInstance @ 0x005f_8710 => extern "thiscall" fn destruct_p_physics_capsule_base_5f8710(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCapsuleBullet__destructInstance @ 0x005f_8740 => extern "thiscall" fn destruct_p_physics_capsule_bullet_5f8740(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCharacterCamera__destructInstance @ 0x005f_8770 => extern "thiscall" fn destruct_p_physics_character_camera_5f8770(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `0x006016c0(ctrl +0xcc, obj) when set, PPhysicsCharacterControllerBullet_release 0x005f8e60, frees +0x100 unless its low bit is set`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCharacterController__destructInstance @ 0x005f_87a0 => extern "thiscall" fn destruct_p_physics_character_controller_5f87a0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_controller(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `frees the +0x100 buffer unless its low bit is set`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCharacterControllerBase__destructInstance @ 0x005f_8830 => extern "thiscall" fn destruct_p_physics_character_controller_base_5f8830(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_controller_base(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `PPhysicsCharacterControllerBullet_release 0x005f8e60, frees +0x100 unless its low bit is set`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCharacterControllerBullet__destructInstance @ 0x005f_8860 => extern "thiscall" fn destruct_p_physics_character_controller_bullet_5f8860(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_controller_bullet(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCylinder__destructInstance @ 0x005f_88d0 => extern "thiscall" fn destruct_p_physics_cylinder_5f88d0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCylinderBase__destructInstance @ 0x005f_8900 => extern "thiscall" fn destruct_p_physics_cylinder_base_5f8900(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsCylinderBullet__destructInstance @ 0x005f_8930 => extern "thiscall" fn destruct_p_physics_cylinder_bullet_5f8930(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `nothing to destroy (mov al, 1)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsInterfaceBase__destructInstance @ 0x005f_89b0 => extern "thiscall" fn destruct_p_physics_interface_base_5f89b0(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `destructInstance(desc, obj)` -> 1: `on p = obj - 1: frees owned non-null [p+0x20] (header [p+0x1c] bit 31 clear), clears both`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsInterfaceBullet__destructInstance @ 0x005f_89c0 => extern "thiscall" fn destruct_p_physics_interface_bullet_5f89c0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_interface_bullet(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `nothing to destroy (mov al, 1)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsMaterial__destructInstance @ 0x005f_8a10 => extern "thiscall" fn destruct_p_physics_material_5f8a10(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsMesh__destructInstance @ 0x005f_8a20 => extern "thiscall" fn destruct_p_physics_mesh_5f8a20(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsMeshBase__destructInstance @ 0x005f_8a50 => extern "thiscall" fn destruct_p_physics_mesh_base_5f8a50(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsMeshBullet__destructInstance @ 0x005f_8a80 => extern "thiscall" fn destruct_p_physics_mesh_bullet_5f8a80(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `thiscall 0x005e23f0(obj) (the type's destructor)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsModel__destructInstance @ 0x005f_8ab0 => extern "thiscall" fn destruct_p_physics_model_5f8ab0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_with(&mut mem(), obj, 0x005e_23f0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsPlane__destructInstance @ 0x005f_8ad0 => extern "thiscall" fn destruct_p_physics_plane_5f8ad0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsPlaneBase__destructInstance @ 0x005f_8b00 => extern "thiscall" fn destruct_p_physics_plane_base_5f8b00(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsPlaneBullet__destructInstance @ 0x005f_8b30 => extern "thiscall" fn destruct_p_physics_plane_bullet_5f8b30(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `thiscall 0x005e2570(obj) (the type's destructor)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsRigidBody__destructInstance @ 0x005f_8b60 => extern "thiscall" fn destruct_p_physics_rigid_body_5f8b60(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_with(&mut mem(), obj, 0x005e_2570)
    }
    /// `destructInstance(desc, obj)` -> 1: `thiscall 0x005e2660(obj) (the type's destructor)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsRigidBodyBase__destructInstance @ 0x005f_8b80 => extern "thiscall" fn destruct_p_physics_rigid_body_base_5f8b80(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_with(&mut mem(), obj, 0x005e_2660)
    }
    /// `destructInstance(desc, obj)` -> 1: `virtual dtors (flag 1) of +0xdc (slot 1) and +0xe0 (slot 0), clears +0xe0/+0xdc/+0xb0, then 0x005e2660`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsRigidBodyBullet__destructInstance @ 0x005f_8ba0 => extern "thiscall" fn destruct_p_physics_rigid_body_bullet_5f8ba0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_rigid_body_bullet(&mut mem(), obj)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsShape__destructInstance @ 0x005f_8c40 => extern "thiscall" fn destruct_p_physics_shape_5f8c40(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsShapeBullet__destructInstance @ 0x005f_8ca0 => extern "thiscall" fn destruct_p_physics_shape_bullet_5f8ca0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsSphere__destructInstance @ 0x005f_8cd0 => extern "thiscall" fn destruct_p_physics_sphere_5f8cd0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsSphereBase__destructInstance @ 0x005f_8d00 => extern "thiscall" fn destruct_p_physics_sphere_base_5f8d00(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsSphereBullet__destructInstance @ 0x005f_8d30 => extern "thiscall" fn destruct_p_physics_sphere_bullet_5f8d30(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsTaperedCapsule__destructInstance @ 0x005f_8d60 => extern "thiscall" fn destruct_p_physics_tapered_capsule_5f8d60(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[3](obj - 4, 0) (the type's virtual destructor, destroy only)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsTaperedCylinder__destructInstance @ 0x005f_8d90 => extern "thiscall" fn destruct_p_physics_tapered_cylinder_5f8d90(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 3)
    }
    /// `destructInstance(desc, obj)` -> 1: `thiscall 0x005e28d0(obj) (the type's destructor)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsWorld__destructInstance @ 0x005f_8dc0 => extern "thiscall" fn destruct_p_physics_world_5f8dc0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_with(&mut mem(), obj, 0x005e_28d0)
    }
    /// `destructInstance(desc, obj)` -> 1: `nothing to destroy (mov al, 1)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsWorldBase__destructInstance @ 0x005f_8de0 => extern "thiscall" fn destruct_p_physics_world_base_5f8de0(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `destructInstance(desc, obj)` -> 1: `thiscall 0x005e2950(obj) (the type's destructor)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PPhysicsWorldBullet__destructInstance @ 0x005f_8df0 => extern "thiscall" fn destruct_p_physics_world_bullet_5f8df0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_with(&mut mem(), obj, 0x005e_2950)
    }
    /// `destructInstance(desc, obj)` -> 1: `nothing to destroy (mov al, 1)`.
    Phyre_PClassDescriptorForType_Phyre_PPhysics_PRaycastResult__destructInstance @ 0x005f_8e10 => extern "thiscall" fn destruct_p_raycast_result_5f8e10(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `PPhysicsCharacterControllerBullet_release(obj)`: the world's
    /// unlink, the three virtual destructors, the fields cleared.
    PPhysicsCharacterControllerBullet_release @ 0x005f_8e60 => extern "thiscall" fn controller_bullet_release(obj: u32) {
        physics::bridge::controller_bullet_release(&mut mem(), obj);
    }
    /// `ClassDynamicMeshInstance(this, mesh)` -> `this` (`object::bridge::construct_mesh_instance`).
    ClassDynamicMeshInstance_ctor @ 0x0067_35f0 => extern "thiscall" fn ctor_6735f0(this: u32, mesh: u32) -> u32 {
        object::bridge::construct_mesh_instance(&mut mem(), this, mesh, 0x00b4_4128)
    }
    /// `DistortionGridDynamicMeshInstance(this, mesh)` -> `this` (`object::bridge::construct_mesh_instance`).
    DistortionGridDynamicMeshInstance_ctor @ 0x0067_4bf0 => extern "thiscall" fn ctor_674bf0(this: u32, mesh: u32) -> u32 {
        object::bridge::construct_mesh_instance(&mut mem(), this, mesh, 0x00b4_4460)
    }
    /// `RadialLineDynamicMeshInstance(this, mesh)` -> `this` (`object::bridge::construct_mesh_instance`).
    RadialLineDynamicMeshInstance_ctor @ 0x0067_5510 => extern "thiscall" fn ctor_675510(this: u32, mesh: u32) -> u32 {
        object::bridge::construct_mesh_instance(&mut mem(), this, mesh, 0x00b4_45d0)
    }
    /// `BrokenScreenPolygonDynamicMeshInstance(this, mesh)` -> `this` (`object::bridge::construct_mesh_instance`).
    BrokenScreenPolygonDynamicMeshInstance_ctor @ 0x0067_6b70 => extern "thiscall" fn ctor_676b70(this: u32, mesh: u32) -> u32 {
        object::bridge::construct_mesh_instance(&mut mem(), this, mesh, 0x00b4_4910)
    }
    /// `ShadowDynamicMeshInstance(this, mesh)` -> `this` (`object::bridge::construct_mesh_instance`).
    ShadowDynamicMeshInstance_ctor @ 0x006f_9490 => extern "thiscall" fn ctor_6f9490(this: u32, mesh: u32) -> u32 {
        object::bridge::construct_mesh_instance(&mut mem(), this, mesh, 0x00b4_c250)
    }
    /// `ClassDynamicMeshInstance`'s scalar deleting destructor `(this, flags)` -> `this`.
    ClassDynamicMeshInstance_scalar_deleting_dtor @ 0x0067_36e0 => extern "thiscall" fn dtor_6736e0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4128), Some(0x004f_a470))
    }
    /// `DistortionGridDynamicMeshInstance`'s scalar deleting destructor `(this, flags)` -> `this`.
    DistortionGridDynamicMeshInstance_scalar_deleting_dtor @ 0x0067_4ce0 => extern "thiscall" fn dtor_674ce0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4460), Some(0x004f_a470))
    }
    /// `RadialLineDynamicMeshInstance`'s scalar deleting destructor `(this, flags)` -> `this`.
    RadialLineDynamicMeshInstance_scalar_deleting_dtor @ 0x0067_5600 => extern "thiscall" fn dtor_675600(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_45d0), Some(0x004f_a470))
    }
    /// `BrokenScreenPolygonDynamicMeshInstance`'s scalar deleting destructor `(this, flags)` -> `this`.
    BrokenScreenPolygonDynamicMeshInstance_scalar_deleting_dtor @ 0x0067_6c60 => extern "thiscall" fn dtor_676c60(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4910), Some(0x004f_a470))
    }
    /// `DistortionGridDynamicMesh`'s scalar deleting destructor `(this, flags)` -> `this`.
    DistortionGridDynamicMesh_scalar_deleting_dtor @ 0x0067_40b0 => extern "thiscall" fn dtor_6740b0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4288), Some(0x0049_69f0))
    }
    /// `RadialLineDynamicMesh`'s scalar deleting destructor `(this, flags)` -> `this`.
    RadialLineDynamicMesh_scalar_deleting_dtor @ 0x0067_6030 => extern "thiscall" fn dtor_676030(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4740), Some(0x0049_69f0))
    }
    /// `BrokenScreenPolygonDynamicMesh`'s scalar deleting destructor `(this, flags)` -> `this`.
    BrokenScreenPolygonDynamicMesh_scalar_deleting_dtor @ 0x0067_7690 => extern "thiscall" fn dtor_677690(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4a90), Some(0x0049_69f0))
    }
    /// `ShadowDynamicMesh`'s scalar deleting destructor `(this, flags)` -> `this`.
    ShadowDynamicMesh_scalar_deleting_dtor @ 0x0067_8530 => extern "thiscall" fn dtor_678530(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_4c78), Some(0x0049_69f0))
    }
    /// `Phyre_ClothInstancingDynamicMesh`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_ClothInstancingDynamicMesh_scalar_deleting_dtor @ 0x006e_0db0 => extern "thiscall" fn dtor_6e0db0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_aa10), Some(0x0049_69f0))
    }
    /// `Phyre_DynamicMeshDefaultImplmenetation`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_DynamicMeshDefaultImplmenetation_scalar_deleting_dtor @ 0x006e_0e10 => extern "thiscall" fn dtor_6e0e10(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b4_a8f0), Some(0x0049_69f0))
    }
    /// `Phyre_ClothInstancingDynamicMeshInstance`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_ClothInstancingDynamicMeshInstance_scalar_deleting_dtor @ 0x006e_0de0 => extern "thiscall" fn dtor_6e0de0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, None, Some(0x006e_0ab0))
    }
    /// `Phyre_PInputs_PInputAction`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputAction_scalar_deleting_dtor @ 0x009e_3d80 => extern "thiscall" fn dtor_9e3d80(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, None, Some(0x009e_2330))
    }
    /// `Phyre_PInputs_PInputSourceJoypadAxis`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceJoypadAxis_scalar_deleting_dtor @ 0x009e_3e10 => extern "thiscall" fn dtor_9e3e10(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceJoypadButton`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceJoypadButton_scalar_deleting_dtor @ 0x009e_3e40 => extern "thiscall" fn dtor_9e3e40(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceKey`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceKey_scalar_deleting_dtor @ 0x009e_3e70 => extern "thiscall" fn dtor_9e3e70(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionAngularVelocityX_scalar_deleting_dtor @ 0x009e_3ea0 => extern "thiscall" fn dtor_9e3ea0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionAngularVelocityY_scalar_deleting_dtor @ 0x009e_3ed0 => extern "thiscall" fn dtor_9e3ed0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityZ`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionAngularVelocityZ_scalar_deleting_dtor @ 0x009e_3f00 => extern "thiscall" fn dtor_9e3f00(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationX_scalar_deleting_dtor @ 0x009e_3f30 => extern "thiscall" fn dtor_9e3f30(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationY_scalar_deleting_dtor @ 0x009e_3f60 => extern "thiscall" fn dtor_9e3f60(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationZ`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationZ_scalar_deleting_dtor @ 0x009e_3f90 => extern "thiscall" fn dtor_9e3f90(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatW`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionQuatW_scalar_deleting_dtor @ 0x009e_3fc0 => extern "thiscall" fn dtor_9e3fc0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionQuatX_scalar_deleting_dtor @ 0x009e_3ff0 => extern "thiscall" fn dtor_9e3ff0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionQuatY_scalar_deleting_dtor @ 0x009e_4020 => extern "thiscall" fn dtor_9e4020(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatZ`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMotionQuatZ_scalar_deleting_dtor @ 0x009e_4050 => extern "thiscall" fn dtor_9e4050(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMouseButton`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMouseButton_scalar_deleting_dtor @ 0x009e_4080 => extern "thiscall" fn dtor_9e4080(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMouseDeltaX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMouseDeltaX_scalar_deleting_dtor @ 0x009e_40b0 => extern "thiscall" fn dtor_9e40b0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceMouseDeltaY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceMouseDeltaY_scalar_deleting_dtor @ 0x009e_40e0 => extern "thiscall" fn dtor_9e40e0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchDragX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchDragX_scalar_deleting_dtor @ 0x009e_4110 => extern "thiscall" fn dtor_9e4110(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchDragY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchDragY_scalar_deleting_dtor @ 0x009e_4140 => extern "thiscall" fn dtor_9e4140(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchPinch`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchPinch_scalar_deleting_dtor @ 0x009e_4170 => extern "thiscall" fn dtor_9e4170(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchRotate`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchRotate_scalar_deleting_dtor @ 0x009e_41a0 => extern "thiscall" fn dtor_9e41a0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchTwoFingerDragX`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchTwoFingerDragX_scalar_deleting_dtor @ 0x009e_41d0 => extern "thiscall" fn dtor_9e41d0(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// `Phyre_PInputs_PInputSourceTouchTwoFingerDragY`'s scalar deleting destructor `(this, flags)` -> `this`.
    Phyre_PInputs_PInputSourceTouchTwoFingerDragY_scalar_deleting_dtor @ 0x009e_4200 => extern "thiscall" fn dtor_9e4200(this: u32, flags: u32) -> u32 {
        object::bridge::scalar_deleting_dtor(&mut mem(), this, flags, Some(0x00b7_0938), None)
    }
    /// Registers the class descriptor at `0x00cc_bb10` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbb10 @ 0x0067_3710 => extern "C" fn register_673710() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_bb10)
    }
    /// Registers the class descriptor at `0x00cc_bbe0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbbe0 @ 0x0067_40e0 => extern "C" fn register_6740e0() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_bbe0)
    }
    /// Registers the class descriptor at `0x00cc_bce0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbce0 @ 0x0067_4d10 => extern "C" fn register_674d10() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_bce0)
    }
    /// Registers the class descriptor at `0x00cc_bdb8` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbdb8 @ 0x0067_5630 => extern "C" fn register_675630() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_bdb8)
    }
    /// Registers the class descriptor at `0x00cc_be88` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbe88 @ 0x0067_6060 => extern "C" fn register_676060() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_be88)
    }
    /// Registers the class descriptor at `0x00cc_bf88` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccbf88 @ 0x0067_6c90 => extern "C" fn register_676c90() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_bf88)
    }
    /// Registers the class descriptor at `0x00cc_c058` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccc058 @ 0x0067_76c0 => extern "C" fn register_6776c0() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_c058)
    }
    /// Registers the class descriptor at `0x00cc_c158` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_00ccc158 @ 0x0067_8560 => extern "C" fn register_678560() -> u32 {
        object::bridge::register(&mut mem(), 0x00cc_c158)
    }
    /// Registers the class descriptor at `0x0194_14e8` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019414e8 @ 0x009e_4ba0 => extern "C" fn register_9e4ba0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_14e8)
    }
    /// Registers the class descriptor at `0x0194_1580` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941580 @ 0x009e_4bc0 => extern "C" fn register_9e4bc0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1580)
    }
    /// Registers the class descriptor at `0x0194_1618` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941618 @ 0x009e_4be0 => extern "C" fn register_9e4be0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1618)
    }
    /// Registers the class descriptor at `0x0194_16b0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019416b0 @ 0x009e_4c00 => extern "C" fn register_9e4c00() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_16b0)
    }
    /// Registers the class descriptor at `0x0194_1748` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941748 @ 0x009e_4c20 => extern "C" fn register_9e4c20() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1748)
    }
    /// Registers the class descriptor at `0x0194_17e0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019417e0 @ 0x009e_4c40 => extern "C" fn register_9e4c40() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_17e0)
    }
    /// Registers the class descriptor at `0x0194_1450` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941450 @ 0x009e_4c60 => extern "C" fn register_9e4c60() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1450)
    }
    /// Registers the class descriptor at `0x0194_1288` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941288 @ 0x009e_4c80 => extern "C" fn register_9e4c80() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1288)
    }
    /// Registers the class descriptor at `0x0194_1320` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941320 @ 0x009e_4ca0 => extern "C" fn register_9e4ca0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1320)
    }
    /// Registers the class descriptor at `0x0194_13b8` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019413b8 @ 0x009e_4cc0 => extern "C" fn register_9e4cc0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_13b8)
    }
    /// Registers the class descriptor at `0x0194_1028` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941028 @ 0x009e_4f40 => extern "C" fn register_9e4f40() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1028)
    }
    /// Registers the class descriptor at `0x0194_10c0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019410c0 @ 0x009e_4f60 => extern "C" fn register_9e4f60() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_10c0)
    }
    /// Registers the class descriptor at `0x0194_0f90` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01940f90 @ 0x009e_4f80 => extern "C" fn register_9e4f80() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_0f90)
    }
    /// Registers the class descriptor at `0x0194_0ef8` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01940ef8 @ 0x009e_4fa0 => extern "C" fn register_9e4fa0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_0ef8)
    }
    /// Registers the class descriptor at `0x0194_1158` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_01941158 @ 0x009e_4fc0 => extern "C" fn register_9e4fc0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_1158)
    }
    /// Registers the class descriptor at `0x0194_11f0` -> `addToNamespace`'s `eax`.
    Phyre_RegisterClass_019411f0 @ 0x009e_4fe0 => extern "C" fn register_9e4fe0() -> u32 {
        object::bridge::register(&mut mem(), 0x0194_11f0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_ClassDynamicMeshInstance__destructInstance @ 0x0067_3a70 => extern "thiscall" fn destruct_673a70(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_DistortionGridDynamicMesh__destructInstance @ 0x0067_4540 => extern "thiscall" fn destruct_674540(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_DistortionGridDynamicMeshInstance__destructInstance @ 0x0067_5070 => extern "thiscall" fn destruct_675070(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_RadialLineDynamicMeshInstance__destructInstance @ 0x0067_59d0 => extern "thiscall" fn destruct_6759d0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_RadialLineDynamicMesh__destructInstance @ 0x0067_6480 => extern "thiscall" fn destruct_676480(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_BrokenScreenPolygonDynamicMeshInstance__destructInstance @ 0x0067_7030 => extern "thiscall" fn destruct_677030(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `destructInstance(desc, obj)` -> 1: `vtable[0](obj - 4, 0)`.
    Phyre_PClassDescriptorForType_BrokenScreenPolygonDynamicMesh__destructInstance @ 0x0067_7ae0 => extern "thiscall" fn destruct_677ae0(_desc: u32, obj: u32) -> u8 {
        physics::bridge::destruct_virtual(&mut mem(), obj, 0)
    }
    /// `ClassDynamicMeshInstance` vtable slot 1 `(this, x)` -> 0x8.
    ClassDynamicMeshInstance_getKind_673ae0 @ 0x0067_3ae0 => extern "thiscall" fn konst_673ae0(_this: u32, _x: u32) -> u32 {
        0x8
    }
    /// `DistortionGridDynamicMeshInstance` vtable slot 1 `(this, x)` -> 0x18.
    DistortionGridDynamicMeshInstance_getKind_6750e0 @ 0x0067_50e0 => extern "thiscall" fn konst_6750e0(_this: u32, _x: u32) -> u32 {
        0x18
    }
}
