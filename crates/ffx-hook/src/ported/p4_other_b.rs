//! `ffx-pother-b` (M4 port list `p4-other-b`): `PhyreEngine` code outside
//! rendering.
//!
//! Threads: the class-descriptor callbacks run where descriptors build
//! default buffers and instances (start-up, the loader's threads); the
//! registrations at start-up; the destructors and constructors wherever
//! their objects live. No shim keeps state.

use super::{mem, ported};
use ffx_pother_b::{
    methods, object, p4_bind, p4_cloth, p4_fog, p4_frames, p4_input, p4_light, p4_mesh, p4_morph,
    physics, small,
};

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
    /// `PProcessorProfileTimelineCpu::initialize(this, x)` -> 0.
    Phyre_PProfiling_PProcessorProfileTimelineCpu_initialize @ 0x0060_ed40 => extern "thiscall" fn profile_timeline_initialize(_this: u32, _x: u32) -> u32 {
        0
    }
    /// `constructInstance(desc, obj)` -> 1 (nothing to build).
    Phyre_PClassDescriptorConcrete_Phyre_PScripting_PAsyncProcessHeader__constructInstance @ 0x0061_7de0 => extern "thiscall" fn construct_async_process_header(_desc: u32, _obj: u32) -> u8 {
        1
    }
    /// `destructInstance(desc, obj)` -> 1 (nothing to destroy).
    Phyre_PClassDescriptorForType_Phyre_PScripting_PAsyncProcessHeader__destructInstance @ 0x0061_7df0 => extern "thiscall" fn destruct_async_process_header(_desc: u32, _obj: u32) -> u8 {
        physics::bridge::destruct_nothing()
    }
    /// `constructInstance(desc, obj)` -> 0 (no default constructor).
    Phyre_PClassDescriptorWithoutDefaultConstructor_Phyre_PScripting_PClassCallableMethodScript__constructInstance @ 0x0061_a0a0 => extern "thiscall" fn construct_callable_method_script(_desc: u32, _obj: u32) -> u8 {
        0
    }
    /// `PInputDeviceKeyboard` vtable slot 1 `(kb)` -> 1.
    PInputDeviceKeyboard_vslot1_006281f0 @ 0x0062_81f0 => extern "thiscall" fn keyboard_slot1(_kb: u32) -> u32 {
        1
    }
    /// `PInputDeviceKeyboard_getButton(kb, i)`.
    PInputDeviceKeyboard_getButton @ 0x0062_89a0 => extern "thiscall" fn keyboard_get_button(kb: u32, i: u32) -> u32 {
        small::bridge::keyboard_button(&mut mem(), kb, i)
    }
    /// `PInputDeviceMouse_getButton(mouse, i)`.
    PInputDeviceMouse_getButton @ 0x0062_89d0 => extern "thiscall" fn mouse_get_button(mouse: u32, i: u32) -> u32 {
        small::bridge::mouse_button(&mut mem(), mouse, i)
    }
    /// `CharacterLighting::GetInstance()`.
    Phyre_CharacterLighting_GetInstance @ 0x006a_6e30 => extern "C" fn character_lighting_instance() -> u32 {
        small::bridge::singleton(&mut mem(), small::G_CHARACTER_LIGHTING)
    }
    /// `CharacterLightingTweakUtil::GetInstance()`.
    Phyre_CharacterLightingTweakUtil_GetInstance @ 0x006a_6e40 => extern "C" fn character_lighting_tweak_instance() -> u32 {
        small::bridge::singleton(&mut mem(), small::G_CHARACTER_LIGHTING_TWEAK)
    }
    /// `MorphManager::GetInstance()`.
    MorphManager_GetInstance @ 0x006a_bf40 => extern "C" fn morph_manager_instance() -> u32 {
        small::bridge::singleton(&mut mem(), small::G_MORPH_MANAGER)
    }
    /// `FogEffectManager::GetInstance()`.
    Phyre_FogEffectManager_GetInstance @ 0x006e_32c0 => extern "C" fn fog_effect_manager_instance() -> u32 {
        small::bridge::singleton(&mut mem(), small::G_FOG_EFFECT_MANAGER)
    }
    /// `CharacterLighting::calcShadeCoeffWithConfidence(cl, a, coeff, c)`
    /// -> `coeff` in `ST0`.
    Phyre_CharacterLighting_calcShadeCoeffWithConfidence @ 0x006a_77a0 => extern "thiscall" fn calc_shade_coeff_with_confidence(_cl: u32, _a: u32, coeff: f32, _c: u32) -> f32 {
        coeff
    }
    /// `CharacterLighting_isInitSuccessful(cl)` -> `al`.
    Phyre_CharacterLighting_isInitSuccessful @ 0x006a_9ad0 => extern "thiscall" fn is_init_successful(cl: u32) -> u8 {
        small::bridge::is_init_successful(&mut mem(), cl)
    }
    /// `CharacterLighting_iteratorValid(it)` -> `al`.
    CharacterLighting_iteratorValid_006a9a80 @ 0x006a_9a80 => extern "thiscall" fn iterator_valid(it: u32) -> u8 {
        small::bridge::iterator_valid(&mut mem(), it)
    }
    /// `CharacterLighting_setCoefficient(cl, i, value)`.
    CharacterLighting_setCoefficient_006aa920 @ 0x006a_a920 => extern "thiscall" fn set_coefficient(cl: u32, i: u32, value: u32) {
        small::bridge::set_coefficient(&mut mem(), cl, i, value);
    }
    /// `CharacterLighting::setMapIdFlagChanged(cl, a, b)` -> `b`.
    Phyre_CharacterLighting_setMapIdFlagChanged @ 0x006a_ad00 => extern "thiscall" fn set_map_id_flag_changed(cl: u32, a: u32, b: u32) -> u32 {
        small::bridge::set_map_id_flag_changed(&mut mem(), cl, a, b)
    }
    /// `ClassDynamicMeshInstance::populatePreprocessData(inst, out, x)` -> 0.
    ClassDynamicMeshInstance_populatePreprocessData @ 0x0067_3b20 => extern "thiscall" fn populate_preprocess_data(inst: u32, out: u32, _x: u32) -> u32 {
        small::bridge::populate_preprocess_data(&mut mem(), inst, out)
    }
    /// `CharacterShaderManager::loadShader()`.
    Phyre_CharacterShaderManager_loadShader @ 0x006d_f3c0 => extern "C" fn load_shader() -> u32 {
        small::bridge::load_shaders(&mut mem())
    }
    /// `Clear4(this)` -> `this`.
    Clear4_006e0800 @ 0x006e_0800 => extern "thiscall" fn clear4(this: u32) -> u32 {
        small::bridge::clear(&mut mem(), this)
    }
    /// `DynamicMeshDefaultImplementation` vtable slot 5 `(this, a, b, c, d)`
    /// -> `b`.
    DynamicMeshDefaultImplementation_vslot5_006e21b0 @ 0x006e_21b0 => extern "thiscall" fn default_mesh_slot5(_this: u32, _a: u32, b: u32, _c: u32, _d: u32) -> u32 {
        b
    }
    /// `ClothInstancingDynamicMeshInstance` vtable slot 1 `(inst, x)`.
    ClothInstancingDynamicMeshInstance_vslot1_006e23d0 @ 0x006e_23d0 => extern "thiscall" fn cloth_instance_slot1(inst: u32, x: u32) -> u32 {
        small::bridge::mesh_slot1_plus8(&mut mem(), inst, x)
    }
    /// `ClothInstancingDynamicMesh_setVertexBuffer(mesh, value, i)` -> `value`.
    ClothInstancingDynamicMesh_setVertexBuffer_006e2b10 @ 0x006e_2b10 => extern "thiscall" fn cloth_set_vertex_buffer(mesh: u32, value: u32, i: u32) -> u32 {
        small::bridge::set_slot(&mut mem(), mesh, value, i)
    }
    /// `DynamicBuffer_upload(this, data, size)` -> 0.
    DynamicBuffer_upload_006f89b0 @ 0x006f_89b0 => extern "thiscall" fn dynamic_buffer_upload(this: u32, data: u32, size: u32) -> u32 {
        small::bridge::upload(&mut mem(), this, data, size)
    }
    /// `ClothInstancingDynamicMesh_waitIdle(mesh)` (vtable `+0x1c`, `ret 4`).
    ClothInstancingDynamicMesh_waitIdle_006e2df0 @ 0x006e_2df0 => extern "stdcall" fn cloth_wait_idle(mesh: u32) {
        methods::bridge::wait_idle(&mut mem(), mesh);
    }
    /// `CharacterLightingTweakUtil::resize(tw, w, h)`.
    Phyre_CharacterLightingTweakUtil_resize @ 0x006a_a3d0 => extern "thiscall" fn tweak_resize(tw: u32, w: u32, h: u32) -> u32 {
        methods::bridge::tweak_resize(&mut mem(), tw, w, h)
    }
    /// `ClothInstancingDynamicMesh` vtable slot 5 `(mesh, a, b, c, d)`.
    ClothInstancingDynamicMesh_vslot5_006e2150 @ 0x006e_2150 => extern "thiscall" fn cloth_slot5(mesh: u32, a: u32, b: u32, c: u32, d: u32) -> u32 {
        methods::bridge::cloth_slot5(&mut mem(), mesh, a, b, c, d)
    }
    /// `CharacterLighting::setRotation(cl, i, x, y)`.
    Phyre_CharacterLighting_setRotation @ 0x006a_aea0 => extern "thiscall" fn set_rotation(cl: u32, i: u32, x: u32, y: u32) {
        methods::bridge::set_rotation(&mut mem(), cl, i, x, y);
    }
    /// `FogEffectManager_copyParameters(fog)` -> the source.
    FogEffectManager_copyParameters_006e3b80 @ 0x006e_3b80 => extern "thiscall" fn fog_copy(fog: u32) -> u32 {
        methods::bridge::fog_copy(&mut mem(), fog)
    }
    /// `SearchTree_findSigned(tree, &key)`.
    SearchTree_findSigned_0064b9a0 @ 0x0064_b9a0 => extern "thiscall" fn tree_find_signed(tree: u32, key: u32) -> u32 {
        methods::bridge::tree_find(&mut mem(), tree, key, true)
    }
    /// `SearchTree_findUnsigned(tree, &key)`.
    SearchTree_findUnsigned_006a5ec0 @ 0x006a_5ec0 => extern "thiscall" fn tree_find_unsigned(tree: u32, key: u32) -> u32 {
        methods::bridge::tree_find(&mut mem(), tree, key, false)
    }
    /// `ClothInstancingDynamicMesh_setSlotBuffer(mesh, a, b, i)`.
    ClothInstancingDynamicMesh_setSlotBuffer_006e2ac0 @ 0x006e_2ac0 => extern "thiscall" fn cloth_slot_buffer(mesh: u32, a: u32, b: u32, i: u32) -> u32 {
        methods::bridge::cloth_slot_buffer(&mut mem(), mesh, a, b, i)
    }
    /// `constructInstance(desc, obj)` of `ClassDynamicMeshInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_ClassDynamicMeshInstance__constructInstance @ 0x0067_39f0 => extern "thiscall" fn construct_class_mesh_instance(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh_instance_default(&mut mem(), obj, 0x00cc_ba18, 0x00b4_4128)
    }
    /// `constructInstance(desc, obj)` of `DistortionGridDynamicMeshInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_DistortionGridDynamicMeshInstance__constructInstance @ 0x0067_4ff0 => extern "thiscall" fn construct_distortion_instance(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh_instance_default(&mut mem(), obj, 0x00cc_bbe0, 0x00b4_4460)
    }
    /// `constructInstance(desc, obj)` of `RadialLineDynamicMeshInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_RadialLineDynamicMeshInstance__constructInstance @ 0x0067_5950 => extern "thiscall" fn construct_radial_instance(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh_instance_default(&mut mem(), obj, 0x00cc_be88, 0x00b4_45d0)
    }
    /// `constructInstance(desc, obj)` of `BrokenScreenPolygonDynamicMeshInstance`.
    Phyre_PClassDescriptorWithoutDefaultConstructor_BrokenScreenPolygonDynamicMeshInstance__constructInstance @ 0x0067_6fb0 => extern "thiscall" fn construct_broken_instance(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh_instance_default(&mut mem(), obj, 0x00cc_c058, 0x00b4_4910)
    }
    /// `constructInstance(desc, obj)` of `DistortionGridDynamicMesh`.
    Phyre_PClassDescriptorConcrete_DistortionGridDynamicMesh__constructInstance @ 0x0067_4460 => extern "thiscall" fn construct_distortion_mesh(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh(&mut mem(), obj, 0x00b4_4288)
    }
    /// `constructInstance(desc, obj)` of `RadialLineDynamicMesh`.
    Phyre_PClassDescriptorConcrete_RadialLineDynamicMesh__constructInstance @ 0x0067_63a0 => extern "thiscall" fn construct_radial_mesh(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh(&mut mem(), obj, 0x00b4_4740)
    }
    /// `constructInstance(desc, obj)` of `BrokenScreenPolygonDynamicMesh`.
    Phyre_PClassDescriptorConcrete_BrokenScreenPolygonDynamicMesh__constructInstance @ 0x0067_7a00 => extern "thiscall" fn construct_broken_mesh(_desc: u32, obj: u32) -> u8 {
        object::bridge::construct_mesh(&mut mem(), obj, 0x00b4_4a90)
    }
    /// `BrokenScreenPolygonDynamicMeshInstance_populatePreprocessData(inst,
    /// out, x)` -> 0.
    BrokenScreenPolygonDynamicMeshInstance_populatePreprocessData_006770f0 @ 0x0067_70f0 => extern "thiscall" fn broken_screen_preprocess(inst: u32, out: u32, _x: u32) -> u32 {
        methods::bridge::broken_screen_preprocess(&mut mem(), inst, out)
    }
    /// `PInput_GetDevice(kind, index)`.
    PInput_GetDevice @ 0x0062_3c90 => extern "C" fn get_device(kind: u32, index: u32) -> u32 {
        methods::bridge::get_device(&mut mem(), kind, index)
    }
    /// `CharacterLightingTweakUtil::setShadeCoeff_dbgHook(tw, id, coeff, mode)`.
    Phyre_CharacterLightingTweakUtil_setShadeCoeff_dbgHook @ 0x006a_afc0 => extern "thiscall" fn tweak_set_shade_coeff(tw: u32, id: u32, coeff: u32, mode: u32) {
        methods::bridge::tweak_set_shade_coeff(&mut mem(), tw, id, coeff, mode);
    }
    /// `CharacterLighting_setIntensity(cl, i, value)`.
    CharacterLighting_setIntensity_006aa8c0 @ 0x006a_a8c0 => extern "thiscall" fn set_intensity(cl: u32, i: u32, value: u32) {
        methods::bridge::set_intensity(&mut mem(), cl, i, value);
    }
    /// `FogEffectManager_setLimit(fog, limit)`.
    FogEffectManager_setLimit_006e3aa0 @ 0x006e_3aa0 => extern "thiscall" fn fog_set_limit(fog: u32, limit: u32) {
        methods::bridge::fog_set_limit(&mut mem(), fog, limit);
    }
    /// `FogEffectManager_create()`.
    FogEffectManager_create_006e3240 @ 0x006e_3240 => extern "C" fn fog_create() {
        methods::bridge::fog_create(&mut mem());
    }
    // ---- continuation round (p4-other-b c1) ----
    /// `PApplicationViewport::PApplicationViewport(vp, app)` -> `vp`.
    PApplicationViewport_ctor_0061fdd0 @ 0x0061_fdd0 => extern "thiscall" fn viewport_ctor(vp: u32, app: u32) -> u32 {
        p4_input::bridge::viewport_ctor(&mut mem(), vp, app)
    }
    /// `PInputDeviceKeyboard::PInputDeviceKeyboard(dev, state)` -> `dev`.
    PInputDeviceKeyboard_ctor @ 0x0061_fff0 => extern "thiscall" fn keyboard_ctor(dev: u32, state: u32) -> u32 {
        p4_input::bridge::keyboard_ctor(&mut mem(), dev, state)
    }
    /// `PInputDeviceMouse::getAxis(dev, id)` -> `ST0`.
    PInputDeviceMouse_getAxis @ 0x0062_8a40 => extern "thiscall" fn mouse_get_axis(dev: u32, id: u32) -> f32 {
        p4_input::bridge::mouse_get_axis(&mut mem(), dev, id)
    }
    /// `PInputDeviceMouse::bind(dev, mapping, id)` -> 0, 0x12 or 5.
    PInputDeviceMouse_bind @ 0x0062_5d10 => extern "thiscall" fn mouse_bind(dev: u32, mapping: u32, id: u32) -> u32 {
        p4_input::bridge::mouse_bind(&mut mem(), dev, mapping, id)
    }
    /// `PInputMapping_update(mapping, raw)`.
    PInputMapping_update_006263e0 @ 0x0062_63e0 => extern "thiscall" fn mapping_update(mapping: u32, raw: u32) {
        p4_input::bridge::mapping_update(&mut mem(), mapping, raw);
    }
    /// `PInputDevice::updateMappings(dev)`.
    PInputDevice_updateMappings @ 0x0062_d550 => extern "thiscall" fn update_mappings(dev: u32) {
        p4_input::bridge::update_mappings(&mut mem(), dev);
    }
    /// `App_Frame()` -> 0 or 9.
    App_Frame @ 0x0062_e340 => extern "C" fn app_frame() -> u32 {
        p4_input::bridge::app_frame(&mut mem())
    }
    /// `SearchTree_insertSigned(tree, node)` -> the equal node or 0.
    SearchTree_insertSigned_0064bfa0 @ 0x0064_bfa0 => extern "thiscall" fn tree_insert_signed(tree: u32, node: u32) -> u32 {
        p4_input::bridge::tree_insert(&mut mem(), tree, node, true)
    }
    /// `SearchTree_insertUnsigned(tree, node)` -> the equal node or 0.
    SearchTree_insertUnsigned_0064c020 @ 0x0064_c020 => extern "thiscall" fn tree_insert_unsigned(tree: u32, node: u32) -> u32 {
        p4_input::bridge::tree_insert(&mut mem(), tree, node, false)
    }
    /// `SearchTree_insertUnsigned(tree, node)` (the lighting's copy).
    SearchTree_insertUnsigned_006a5fc0 @ 0x006a_5fc0 => extern "thiscall" fn tree_insert_unsigned_2(tree: u32, node: u32) -> u32 {
        p4_input::bridge::tree_insert(&mut mem(), tree, node, false)
    }
    /// `DistortionGridDynamicMesh_getParams(params, x)` -> 0.
    DistortionGridDynamicMesh_getParams_00675120 @ 0x0067_5120 => extern "stdcall" fn distortion_params(params: u32, _x: u32) -> u32 {
        p4_input::bridge::distortion_params(&mut mem(), params)
    }
    /// `CharacterLighting_create()`.
    CharacterLighting_create_006a6cd0 @ 0x006a_6cd0 => extern "C" fn lighting_create() {
        p4_light::bridge::lighting_create(&mut mem());
    }
    /// `CharacterLightingTweakUtil_create()`.
    CharacterLightingTweakUtil_create_006a6d50 @ 0x006a_6d50 => extern "C" fn tweak_create() {
        p4_light::bridge::tweak_create(&mut mem());
    }
    /// `MorphManager_create()`.
    MorphManager_create_006abe40 @ 0x006a_be40 => extern "C" fn morph_create() {
        p4_light::bridge::morph_create(&mut mem());
    }
    /// `CharacterLightingTweakUtil::CharacterLightingTweakUtil(tw)` -> `tw`.
    CharacterLightingTweakUtil_ctor_006a6610 @ 0x006a_6610 => extern "thiscall" fn tweak_ctor(tw: u32) -> u32 {
        p4_light::bridge::tweak_ctor(&mut mem(), tw)
    }
    /// `CharacterLighting::hasIntensity(cl, i)` -> 0 or 1.
    CharacterLighting_hasIntensity_006a9ae0 @ 0x006a_9ae0 => extern "thiscall" fn has_intensity(cl: u32, i: u32) -> u32 {
        p4_light::bridge::has_intensity(&mut mem(), cl, i)
    }
    /// `CharacterLighting::setPointLight(cl, r, g, b, i)`.
    CharacterLighting_setPointLight_006aad20 @ 0x006a_ad20 => extern "thiscall" fn set_point_light(cl: u32, r: u32, g: u32, b: u32, i: u32) {
        p4_light::bridge::set_point_light(&mut mem(), cl, r, g, b, i);
    }
    /// `CharacterLighting::setLightColor(cl, kind, r, g, b)`.
    Phyre_CharacterLighting_setLightColor_006aa940 @ 0x006a_a940 => extern "thiscall" fn set_light_color(cl: u32, kind: u32, r: u32, g: u32, b: u32) {
        p4_light::bridge::set_light_color(&mut mem(), cl, kind, r, g, b);
    }
    /// `CharacterLighting::defaultShadeCoeff(cl)` -> `ST0`.
    CharacterLighting_defaultShadeCoeff_006a8c40 @ 0x006a_8c40 => extern "thiscall" fn shade_coeff_default(cl: u32) -> f32 {
        p4_light::bridge::shade_coeff_default(&mut mem(), cl)
    }
    /// `CharacterLighting::resetShadeCoeff(cl, inst)`.
    Phyre_CharacterLighting_resetShadeCoeff @ 0x006a_afa0 => extern "thiscall" fn reset_shade_coeff(cl: u32, inst: u32) -> u32 {
        p4_light::bridge::reset_shade_coeff(&mut mem(), cl, inst)
    }
    /// `CharacterLightingTweakUtil::applyShadeCoeffs(tw)`.
    CharacterLightingTweakUtil_applyShadeCoeffs_006ab720 @ 0x006a_b720 => extern "thiscall" fn tweak_apply_shade_coeffs(tw: u32) {
        p4_light::bridge::tweak_apply_shade_coeffs(&mut mem(), tw);
    }
    /// `PArray<u32>::push(arr, buffer, &item)` -> 0 or 0xd.
    PArray_pushWord_006a94b0 @ 0x006a_94b0 => extern "thiscall" fn array_push_word(arr: u32, buffer: u32, item: u32) -> u32 {
        p4_light::bridge::array_push(&mut mem(), arr, buffer, item, 1, false)
    }
    /// `PArray<pair>::push(arr, buffer, &item)` -> 0 or 0xd.
    PArray_pushPair_006ac380 @ 0x006a_c380 => extern "thiscall" fn array_push_pair(arr: u32, buffer: u32, item: u32) -> u32 {
        p4_light::bridge::array_push(&mut mem(), arr, buffer, item, 2, true)
    }
    /// `FogEffectManager::reset(fog)`.
    Phyre_FogEffectManager_reset @ 0x006e_34f0 => extern "thiscall" fn fog_reset(fog: u32) {
        p4_fog::bridge::reset(&mut mem(), fog);
    }
    /// `FogEffectManager::initialize(fog, src, a, b)`.
    FogEffectManager_initialize_006e3400 @ 0x006e_3400 => extern "thiscall" fn fog_initialize(fog: u32, src: u32, a: u32, b: u32) {
        p4_fog::bridge::initialize(&mut mem(), fog, src, a, b);
    }
    /// `FogEffectManager::distance(fog, other)` -> `ST0`.
    FogEffectManager_distance_006e3320 @ 0x006e_3320 => extern "thiscall" fn fog_distance(fog: u32, other: u32) -> f32 {
        p4_fog::bridge::distance_to(&mut mem(), fog, other)
    }
    /// `FogEffectManager::isMoving(fog)` -> `al`.
    FogEffectManager_isMoving_006e3480 @ 0x006e_3480 => extern "thiscall" fn fog_is_moving(fog: u32) -> u8 {
        p4_fog::bridge::is_moving(&mut mem(), fog)
    }
    // ---- p4-other-b c1 batch 2 ----
    /// `CharacterLighting_iteratorSkipInvalid(it)` -> `SetNode`'s `eax` or 0.
    CharacterLighting_iteratorSkipInvalid_006a8d30 @ 0x006a_8d30 => extern "thiscall" fn iterator_skip_invalid(it: u32) -> u32 {
        p4_light::bridge::iterator_advance(&mut mem(), it, true)
    }
    /// `CharacterLighting_iteratorNext(it)` -> `SetNode`'s `eax` or 0.
    CharacterLighting_iteratorNext_006a8da0 @ 0x006a_8da0 => extern "thiscall" fn iterator_next(it: u32) -> u32 {
        p4_light::bridge::iterator_advance(&mut mem(), it, false)
    }
    /// `PInputMapper::latchButtons(mapper)`.
    PInputMapper_latchButtons @ 0x0062_8320 => extern "thiscall" fn latch_buttons(mapper: u32) {
        p4_input::bridge::latch_buttons(&mut mem(), mapper);
    }
    /// `Material_setParameter(inst, name, src)`.
    Material_setParameter_006e60d0 @ 0x006e_60d0 => extern "C" fn set_parameter(inst: u32, name: u32, src: u32) {
        p4_morph::bridge::set_parameter(&mut mem(), inst, name, src);
    }
    /// `CharacterLighting::setShadeCoeff(cl, inst, coeff)` (`coeff` unused).
    Phyre_CharacterLighting_setShadeCoeff @ 0x006a_aef0 => extern "thiscall" fn set_shade_coeff(cl: u32, inst: u32, _coeff: u32) {
        let mut frame = super::StackFrame::<64>::new();
        p4_morph::bridge::set_shade_coeff(&mut mem(), cl, inst, frame.at());
    }
    /// `CharacterLighting::setShadeColor(cl, inst, r, g, b)`.
    Phyre_CharacterLighting_setShadeColor @ 0x006a_b010 => extern "thiscall" fn set_shade_color(cl: u32, inst: u32, r: u32, g: u32, b: u32) {
        let mut frame = super::StackFrame::<64>::new();
        p4_morph::bridge::set_shade_color(&mut mem(), cl, inst, [r, g, b], frame.at());
    }
    /// `Material_hasParameter(inst, name)` -> `al`.
    Material_hasParameter_006ac4a0 @ 0x006a_c4a0 => extern "stdcall" fn has_parameter(inst: u32, name: u32) -> u8 {
        p4_morph::bridge::has_parameter(&mut mem(), inst, name)
    }
    /// `MorphManager_setMorphWeight(inst, w)`.
    MorphManager_setMorphWeight_006ac900 @ 0x006a_c900 => extern "stdcall" fn set_morph_weight(inst: u32, w: u32) {
        p4_morph::bridge::set_morph_weight(&mut mem(), inst, w);
    }
    /// `MorphManager_addPair(arr, key, value)` -> 0 or the push's result.
    MorphManager_addPair_006ab8c0 @ 0x006a_b8c0 => extern "stdcall" fn add_pair(arr: u32, key: u32, value: u32) -> u32 {
        let mut item = super::StackFrame::<2>::new();
        p4_morph::bridge::add_pair(&mut mem(), arr, key, value, item.at())
    }
    /// `MorphManager_setWeights(mm, id, &a, &b)`.
    MorphManager_setWeights_006abfc0 @ 0x006a_bfc0 => extern "thiscall" fn set_weights(mm: u32, id: u32, a: u32, b: u32) {
        p4_morph::bridge::set_weights(&mut mem(), mm, id, a, b);
    }
    /// `ClothInstancingDynamicMesh_submit(mesh, inst, job)` -> the inner mesh's `eax`.
    ClothInstancingDynamicMesh_submit_006e28f0 @ 0x006e_28f0 => extern "thiscall" fn cloth_submit(mesh: u32, inst: u32, job: u32) -> u32 {
        p4_cloth::bridge::submit(&mut mem(), mesh, inst, job)
    }
    /// `ClothInstancing_releaseExpired()`.
    ClothInstancing_releaseExpired_006e2f70 @ 0x006e_2f70 => extern "C" fn cloth_release_expired() {
        p4_cloth::bridge::release_expired(&mut mem());
    }
    /// `ClothInstancingDynamicMeshInstance::~ClothInstancingDynamicMeshInstance(inst)`.
    ClothInstancingDynamicMeshInstance_dtor_006e0ab0 @ 0x006e_0ab0 => extern "thiscall" fn cloth_instance_dtor(inst: u32) {
        p4_cloth::bridge::instance_dtor(&mut mem(), inst);
    }
    /// `DistortionGridDynamicMesh_copyBuffers(x, src, outs, mesh)` -> 0.
    DistortionGridDynamicMesh_copyBuffers_00674570 @ 0x0067_4570 => extern "stdcall" fn distortion_copy_buffers(_x: u32, src: u32, outs: u32, mesh: u32) -> u32 {
        p4_cloth::bridge::copy_buffers(&mut mem(), src, outs, mesh)
    }
    /// `BrokenScreenPolygonDynamicMesh_copyBuffers(x, src, outs, mesh)` -> 0.
    BrokenScreenPolygonDynamicMesh_copyBuffers_00677b10 @ 0x0067_7b10 => extern "stdcall" fn broken_copy_buffers(_x: u32, src: u32, outs: u32, mesh: u32) -> u32 {
        p4_cloth::bridge::copy_buffers(&mut mem(), src, outs, mesh)
    }
    // ---- p4-other-b c1 batch 3 ----
    /// `Phyre_PInputs_PInputAction_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputAction_Bind @ 0x009e_4430 => extern "C" fn input_action_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::INPUT_ACTION)
    }
    /// `Phyre_PInputs_PInputMap_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputMap_Bind @ 0x009e_46d0 => extern "C" fn input_map_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::INPUT_MAP)
    }
    /// `Phyre_PInputs_PInputSource_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSource_Bind @ 0x009e_47c0 => extern "C" fn input_source_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::INPUT_SOURCE)
    }
    /// `Phyre_PInputs_PInputSourceJoypadAxis_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceJoypadAxis_Bind @ 0x009e_4940 => extern "C" fn joypad_axis_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::JOYPAD_AXIS)
    }
    /// `Phyre_PInputs_PInputSourceJoypadButton_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceJoypadButton_Bind @ 0x009e_49e0 => extern "C" fn joypad_button_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::JOYPAD_BUTTON)
    }
    /// `Phyre_PInputs_PInputSourceKey_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceKey_Bind @ 0x009e_4ac0 => extern "C" fn source_key_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::SOURCE_KEY)
    }
    /// `Phyre_PInputs_PInputSourceMouseButton_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceMouseButton_Bind @ 0x009e_4ce0 => extern "C" fn mouse_button_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::MOUSE_BUTTON)
    }
    /// `Phyre_PInputs_PInputSourceWithModifiers_Bind_009e4d80()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceWithModifiers_Bind_009e4d80 @ 0x009e_4d80 => extern "C" fn modifiers_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::MOUSE_BUTTON_KEY)
    }
    /// `Phyre_PInputs_PInputSourceWithModifiers_Bind_009e4e60()` -> `addToNamespace`'s `eax`.
    Phyre_PInputs_PInputSourceWithModifiers_Bind_009e4e60 @ 0x009e_4e60 => extern "C" fn modifiers_bind_2() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::MOUSE_BUTTON_KEY_2)
    }
    /// `Phyre_PGame_PGameSettings_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PGame_PGameSettings_Bind @ 0x00a3_3540 => extern "C" fn game_settings_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::GAME_SETTINGS)
    }
    /// `Phyre_PScripting_PAsyncProcessHeader_Bind()` -> `addToNamespace`'s `eax`.
    Phyre_PScripting_PAsyncProcessHeader_Bind @ 0x0061_7960 => extern "C" fn async_process_header_bind() -> u32 {
        p4_bind::bridge::bind(&mut mem(), &p4_bind::ASYNC_PROCESS_HEADER)
    }
    /// `PArray<PInputAction*>`'s declaration `(name)` -> the descriptor.
    PArray_PInputAction_DeclareClassDescriptor_009e5010 @ 0x009e_5010 => extern "C" fn array_input_action_decl(name: u32) -> u32 {
        p4_bind::bridge::array_decl(&mut mem(), &p4_bind::ARRAY_INPUT_ACTION, name)
    }
    /// `PArray<PInputSource*>`'s declaration `(name)` -> the descriptor.
    PArray_PInputSource_DeclareClassDescriptor_009e5220 @ 0x009e_5220 => extern "C" fn array_input_source_decl(name: u32) -> u32 {
        p4_bind::bridge::array_decl(&mut mem(), &p4_bind::ARRAY_INPUT_SOURCE, name)
    }
    /// `PClassDescriptorAbstract<PArray<PInputSource*>>(desc)` -> `desc`.
    PClassDescriptorAbstract_PArray_PInputSource_ctor_009dfbd0 @ 0x009d_fbd0 => extern "thiscall" fn input_source_array_desc_ctor(desc: u32) -> u32 {
        p4_bind::bridge::descriptor_ctor(&mut mem(), &p4_bind::CTOR_INPUT_SOURCE_ARRAY, desc)
    }
    /// `PClassDescriptorAbstract<PArray<PInputMap*>>(desc)` -> `desc`.
    PClassDescriptorAbstract_PArray_PInputMap_ctor_00a32ef0 @ 0x00a3_2ef0 => extern "thiscall" fn input_map_array_desc_ctor(desc: u32) -> u32 {
        p4_bind::bridge::descriptor_ctor(&mut mem(), &p4_bind::CTOR_INPUT_MAP_ARRAY, desc)
    }
    /// `PUtilityInputs::initialize()` -> 0.
    Phyre_PInputs_PUtilityInputs_initialize @ 0x009f_4650 => extern "C" fn utility_inputs_initialize() -> u32 {
        p4_bind::startup::utility_inputs_initialize(&mut mem())
    }
    /// `PApplication_Startup(app, argv, argc)` -> 0 or 9.
    PApplication_Startup_00629a40 @ 0x0062_9a40 => extern "thiscall" fn startup(app: u32, argv: u32, argc: u32) -> u32 {
        p4_bind::startup::startup(&mut mem(), app, argv, argc)
    }
    // ---- p4-other-b c1 batch 4 ----
    /// `PInputMapper::getStickAxis(mapper, axis, pad)` -> `ST0`.
    PInputMapper_getStickAxis @ 0x0062_8470 => extern "thiscall" fn get_stick_axis(mapper: u32, axis: u32, pad: u32) -> f32 {
        p4_input::bridge4::get_stick_axis(&mut mem(), mapper, axis, pad)
    }
    /// `PInputMapper::attachPads(mapper)`.
    PInputMapper_attachPads @ 0x0062_6aa0 => extern "thiscall" fn attach_pads(mapper: u32) {
        p4_input::bridge4::attach_pads(&mut mem(), mapper);
    }
    /// `CharacterLighting::setMapName(cl, name)`.
    CharacterLighting_setMapName_006ab0d0 @ 0x006a_b0d0 => extern "thiscall" fn set_map_name(cl: u32, name: u32) {
        p4_light::bridge4::set_map_name(&mut mem(), cl, name);
    }
    /// `CharacterLighting::updateFlash(cl, dt)`.
    CharacterLighting_updateFlash_006ab630 @ 0x006a_b630 => extern "thiscall" fn update_flash(cl: u32, dt: u32) {
        p4_light::bridge4::update_flash(&mut mem(), cl, dt);
    }
    // ---- p4-other-b c2 batch 1 ----
    /// `DynamicMesh_setStreams(mesh, a, b, c)` -> the stream table.
    DynamicMesh_setStreams_00676680 @ 0x0067_6680 => extern "thiscall" fn set_streams_676680(mesh: u32, a: u32, b: u32, c: u32) -> u32 {
        p4_mesh::bridge::set_streams(&mut mem(), mesh, a, b, c)
    }
    /// `DynamicMesh_setStreams(mesh, a, b, c)` -> the stream table.
    DynamicMesh_setStreams_00677ce0 @ 0x0067_7ce0 => extern "thiscall" fn set_streams_677ce0(mesh: u32, a: u32, b: u32, c: u32) -> u32 {
        p4_mesh::bridge::set_streams(&mut mem(), mesh, a, b, c)
    }
    /// `DistortionGridDynamicMeshInstance_create(cluster, &mesh)` -> the instance or 0.
    DistortionGridDynamicMeshInstance_create_00673e20 @ 0x0067_3e20 => extern "thiscall" fn distortion_create(cluster: u32, p: u32) -> u32 {
        p4_mesh::bridge::create_instance(&mut mem(), cluster, p4_mesh::DISTORTION_DESC, p4_mesh::DISTORTION_CTOR, p)
    }
    /// `RadialLineDynamicMeshInstance_create(cluster, &mesh)` -> the instance or 0.
    RadialLineDynamicMeshInstance_create_00675d90 @ 0x0067_5d90 => extern "thiscall" fn radial_create(cluster: u32, p: u32) -> u32 {
        p4_mesh::bridge::create_instance(&mut mem(), cluster, p4_mesh::RADIAL_DESC, p4_mesh::RADIAL_CTOR, p)
    }
    /// `ClassDynamicMesh_fillDrawTable(mesh, src)`; leaves the records'
    /// running offset in its argument's slot, as the original does.
    ClassDynamicMesh_fillDrawTable_006732a0 @ 0x0067_32a0 => extern "thiscall" fn fill_draw_table(mesh: u32, src: u32; slots: u32) {
        let mut m = mem();
        let word = p4_mesh::bridge::fill_draw_table(&mut m, mesh, src);
        ffx_bridge::Mem::write(&mut m, slots, word);
    }
    /// `ClothInstancingDynamicMeshInstance(inst, mesh)` -> `inst`.
    ClothInstancingDynamicMeshInstance_ctor_006e0860 @ 0x006e_0860 => extern "thiscall" fn cloth_instance_ctor(inst: u32, mesh: u32) -> u32 {
        p4_mesh::bridge::cloth_instance_ctor(&mut mem(), inst, mesh)
    }
    /// `CharacterShaderManager::loadShaderLower()` -> the found object, the seek's `eax` or 0.
    Phyre_CharacterShaderManager_loadShaderLower @ 0x006d_f3d0 => extern "C" fn load_shader_lower() -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        let ebp = frame.at() + 0x20;
        p4_frames::bridge::load_shader_cluster(&mut mem(), ebp, &p4_frames::BTM_TO_TOP)
    }
    /// `CharacterShaderManager::loadShaderMagic249()` -> the found object, the seek's `eax` or 0.
    Phyre_CharacterShaderManager_loadShaderMagic249 @ 0x006d_f470 => extern "C" fn load_shader_magic249() -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        let ebp = frame.at() + 0x20;
        p4_frames::bridge::load_shader_cluster(&mut mem(), ebp, &p4_frames::MAGIC_249)
    }
    /// `CharacterShaderManager::loadShaderMagic297()` -> the found object, the seek's `eax` or 0.
    Phyre_CharacterShaderManager_loadShaderMagic297 @ 0x006d_f510 => extern "C" fn load_shader_magic297() -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        let ebp = frame.at() + 0x20;
        p4_frames::bridge::load_shader_cluster(&mut mem(), ebp, &p4_frames::MAGIC_297)
    }
    /// `WindowEventHandler_insert(key)` -> the tree insert's `eax`.
    WindowEventHandler_insert_00625c70 @ 0x0062_5c70 => extern "C" fn window_handler_insert(key: u32) -> u32 {
        // ebp - 8 .. ebp + 0xc: the out pair, two unused words, the key
        let mut frame = super::StackFrame::<5>::new();
        let ebp = frame.at() + 8;
        p4_frames::bridge::insert_handler(&mut mem(), ebp, key)
    }
    /// `NamedValueMap_insert(map, name, &value)` -> `map`.
    NamedValueMap_insert_0064bde0 @ 0x0064_bde0 => extern "C" fn named_value_insert(map: u32, name: u32, value: u32) -> u32 {
        let mut frame = super::StackFrame::<9>::new();
        let ebp = frame.at() + 0x24;
        p4_frames::bridge::insert_named(&mut mem(), ebp, map, name, value)
    }
    /// `CharacterLighting::setPointLightPosition(cl, x, y, z, i)` -> the matrix copy's `eax`.
    Phyre_CharacterLighting_setPointLightPosition @ 0x006a_ad90 => extern "thiscall" fn set_point_light_position(cl: u32, x: u32, y: u32, z: u32, i: u32) -> u32 {
        let mut m = super::StackFrame::<16>::new();
        p4_light::bridge5::set_light_pos(&mut mem(), cl, [x, y, z], i, m.at())
    }
    /// `PInputDeviceMouse::PInputDeviceMouse(dev, state)` -> `dev`.
    PInputDeviceMouse_ctor @ 0x0062_00d0 => extern "thiscall" fn mouse_ctor(dev: u32, state: u32) -> u32 {
        p4_input::bridge5::mouse_ctor(&mut mem(), dev, state)
    }
}
