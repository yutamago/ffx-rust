//! M5 stage 1, list `static-init-b`: the compiler-generated constructors
//! of globals and the destructors they register (run before `WinMain`).
//! Each shim calls its row of `ffx_pload::m5_static_init_b` (the family
//! tables), so one shim per address; the parity tests are
//! `crates/ffx-oracle/tests/m5_static_init_b.rs`.

use ffx_pload::m5_static_init_b as si;

use super::{mem, ported};

ported! {
    /// `StaticInit_PPhysicsCallbackData_00aecc50`: class descriptor `PPhysicsCallbackData` (`cd_desc_set8`): constructor, buffers, set8; atexit.
    StaticInit_PPhysicsCallbackData_00aecc50 @ 0x00ae_cc50 => extern "C" fn s_00aecc50() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[0])
    }
    /// `StaticInit_PPhysicsCapsule_00aecd10`: class descriptor `PPhysicsCapsule` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCapsule_00aecd10 @ 0x00ae_cd10 => extern "C" fn s_00aecd10() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[1])
    }
    /// `StaticInit_PPhysicsCapsuleBase_00aecdd0`: class descriptor `PPhysicsCapsuleBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCapsuleBase_00aecdd0 @ 0x00ae_cdd0 => extern "C" fn s_00aecdd0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[2])
    }
    /// `StaticInit_PPhysicsCapsuleBullet_00aece90`: class descriptor `PPhysicsCapsuleBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCapsuleBullet_00aece90 @ 0x00ae_ce90 => extern "C" fn s_00aece90() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[3])
    }
    /// `StaticInit_CtorVt_00aecf50`: no-argument constructor `0x005d_d540` on a global, flags &= ~2, its vtable, atexit.
    StaticInit_CtorVt_00aecf50 @ 0x00ae_cf50 => extern "C" fn s_00aecf50() -> u32 {
        si::ctor0_flags_vt(&mut mem(), &si::CTOR0_FLAGS_VT[0])
    }
    /// `StaticInit_PPhysicsCharacterController_00aecf80`: class descriptor `PPhysicsCharacterController` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCharacterController_00aecf80 @ 0x00ae_cf80 => extern "C" fn s_00aecf80() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[4])
    }
    /// `StaticInit_CtorVt_00aed040`: no-argument constructor `0x005d_d680` on a global, flags &= ~2, its vtable, atexit.
    StaticInit_CtorVt_00aed040 @ 0x00ae_d040 => extern "C" fn s_00aed040() -> u32 {
        si::ctor0_flags_vt(&mut mem(), &si::CTOR0_FLAGS_VT[1])
    }
    /// `StaticInit_PPhysicsCharacterControllerBullet_00aed070`: class descriptor `PPhysicsCharacterControllerBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCharacterControllerBullet_00aed070 @ 0x00ae_d070 => extern "C" fn s_00aed070() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[5])
    }
    /// `StaticInit_PPhysicsCylinder_00aed130`: class descriptor `PPhysicsCylinder` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCylinder_00aed130 @ 0x00ae_d130 => extern "C" fn s_00aed130() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[6])
    }
    /// `StaticInit_PPhysicsCylinderBase_00aed1f0`: class descriptor `PPhysicsCylinderBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCylinderBase_00aed1f0 @ 0x00ae_d1f0 => extern "C" fn s_00aed1f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[7])
    }
    /// `StaticInit_PPhysicsCylinderBullet_00aed2b0`: class descriptor `PPhysicsCylinderBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsCylinderBullet_00aed2b0 @ 0x00ae_d2b0 => extern "C" fn s_00aed2b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[8])
    }
    /// `StaticInit_PPhysicsInterface_00aed370`: class descriptor `PPhysicsInterface` (`cd_const_keep`): constructor, buffers, keep; atexit.
    StaticInit_PPhysicsInterface_00aed370 @ 0x00ae_d370 => extern "C" fn s_00aed370() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[9])
    }
    /// `StaticInit_PPhysicsInterfaceBase_00aed420`: class descriptor `PPhysicsInterfaceBase` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsInterfaceBase_00aed420 @ 0x00ae_d420 => extern "C" fn s_00aed420() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[10])
    }
    /// `StaticInit_PPhysicsInterfaceBullet_00aed4e0`: class descriptor `PPhysicsInterfaceBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsInterfaceBullet_00aed4e0 @ 0x00ae_d4e0 => extern "C" fn s_00aed4e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[11])
    }
    /// `StaticInit_PPhysicsMaterial_00aed5a0`: class descriptor `PPhysicsMaterial` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsMaterial_00aed5a0 @ 0x00ae_d5a0 => extern "C" fn s_00aed5a0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[12])
    }
    /// `StaticInit_PPhysicsMesh_00aed660`: class descriptor `PPhysicsMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsMesh_00aed660 @ 0x00ae_d660 => extern "C" fn s_00aed660() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[13])
    }
    /// `StaticInit_PPhysicsMeshBase_00aed720`: class descriptor `PPhysicsMeshBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsMeshBase_00aed720 @ 0x00ae_d720 => extern "C" fn s_00aed720() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[14])
    }
    /// `StaticInit_PPhysicsMeshBullet_00aed7e0`: class descriptor `PPhysicsMeshBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsMeshBullet_00aed7e0 @ 0x00ae_d7e0 => extern "C" fn s_00aed7e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[15])
    }
    /// `StaticInit_PPhysicsModel_00aed8a0`: class descriptor `PPhysicsModel` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsModel_00aed8a0 @ 0x00ae_d8a0 => extern "C" fn s_00aed8a0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[16])
    }
    /// `StaticInit_PPhysicsPlane_00aed960`: class descriptor `PPhysicsPlane` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsPlane_00aed960 @ 0x00ae_d960 => extern "C" fn s_00aed960() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[17])
    }
    /// `StaticInit_PPhysicsPlaneBase_00aeda20`: class descriptor `PPhysicsPlaneBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsPlaneBase_00aeda20 @ 0x00ae_da20 => extern "C" fn s_00aeda20() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[18])
    }
    /// `StaticInit_PPhysicsPlaneBullet_00aedae0`: class descriptor `PPhysicsPlaneBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsPlaneBullet_00aedae0 @ 0x00ae_dae0 => extern "C" fn s_00aedae0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[19])
    }
    /// `StaticInit_PPhysicsRigidBody_00aedba0`: class descriptor `PPhysicsRigidBody` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsRigidBody_00aedba0 @ 0x00ae_dba0 => extern "C" fn s_00aedba0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[20])
    }
    /// `StaticInit_CtorVt_00aedc60`: no-argument constructor `0x005d_e070` on a global, flags &= ~2, its vtable, atexit.
    StaticInit_CtorVt_00aedc60 @ 0x00ae_dc60 => extern "C" fn s_00aedc60() -> u32 {
        si::ctor0_flags_vt(&mut mem(), &si::CTOR0_FLAGS_VT[2])
    }
    /// `StaticInit_PPhysicsRigidBodyBullet_00aedc90`: class descriptor `PPhysicsRigidBodyBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsRigidBodyBullet_00aedc90 @ 0x00ae_dc90 => extern "C" fn s_00aedc90() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[21])
    }
    /// `StaticInit_PPhysicsShape_00aedd50`: class descriptor `PPhysicsShape` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsShape_00aedd50 @ 0x00ae_dd50 => extern "C" fn s_00aedd50() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[22])
    }
    /// `StaticInit_PPhysicsShapeBase_00aede10`: class descriptor `PPhysicsShapeBase` (`cd_desc_keep`): constructor, buffers, keep; atexit.
    StaticInit_PPhysicsShapeBase_00aede10 @ 0x00ae_de10 => extern "C" fn s_00aede10() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[23])
    }
    /// `StaticInit_PPhysicsShapeBullet_00aedec0`: class descriptor `PPhysicsShapeBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsShapeBullet_00aedec0 @ 0x00ae_dec0 => extern "C" fn s_00aedec0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[24])
    }
    /// `StaticInit_PPhysicsSphere_00aedfa0`: class descriptor `PPhysicsSphere` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsSphere_00aedfa0 @ 0x00ae_dfa0 => extern "C" fn s_00aedfa0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[25])
    }
    /// `StaticInit_PPhysicsSphereBase_00aee060`: class descriptor `PPhysicsSphereBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsSphereBase_00aee060 @ 0x00ae_e060 => extern "C" fn s_00aee060() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[26])
    }
    /// `StaticInit_PPhysicsSphereBullet_00aee120`: class descriptor `PPhysicsSphereBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsSphereBullet_00aee120 @ 0x00ae_e120 => extern "C" fn s_00aee120() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[27])
    }
    /// `StaticInit_PPhysicsTaperedCapsule_00aee1e0`: class descriptor `PPhysicsTaperedCapsule` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsTaperedCapsule_00aee1e0 @ 0x00ae_e1e0 => extern "C" fn s_00aee1e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[28])
    }
    /// `StaticInit_PPhysicsTaperedCylinder_00aee2a0`: class descriptor `PPhysicsTaperedCylinder` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsTaperedCylinder_00aee2a0 @ 0x00ae_e2a0 => extern "C" fn s_00aee2a0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[29])
    }
    /// `StaticInit_PPhysicsWorld_00aee360`: class descriptor `PPhysicsWorld` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsWorld_00aee360 @ 0x00ae_e360 => extern "C" fn s_00aee360() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[30])
    }
    /// `StaticInit_PPhysicsWorldBase_00aee420`: class descriptor `PPhysicsWorldBase` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsWorldBase_00aee420 @ 0x00ae_e420 => extern "C" fn s_00aee420() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[31])
    }
    /// `StaticInit_PPhysicsWorldBullet_00aee4e0`: class descriptor `PPhysicsWorldBullet` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPhysicsWorldBullet_00aee4e0 @ 0x00ae_e4e0 => extern "C" fn s_00aee4e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[32])
    }
    /// `StaticInit_PRaycastResult_00aee5a0`: class descriptor `PRaycastResult` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PRaycastResult_00aee5a0 @ 0x00ae_e5a0 => extern "C" fn s_00aee5a0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[33])
    }
    /// `StaticInit_ChainAppend_00aee660`: appends the object at `0x00c2_a0f4` to the chain at `0xc901a8` (before the first block at least `0xff` full); atexit.
    StaticInit_ChainAppend_00aee660 @ 0x00ae_e660 => extern "C" fn s_00aee660() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[0])
    }
    /// `StaticInit_Atexit_00aee6b0`: only registers its destructor with atexit.
    StaticInit_Atexit_00aee6b0 @ 0x00ae_e6b0 => extern "C" fn s_00aee6b0() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[0])
    }
    /// `StaticInit_PAsyncProcessHeader_00aee6c0`: class descriptor `PAsyncProcessHeader` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PAsyncProcessHeader_00aee6c0 @ 0x00ae_e6c0 => extern "C" fn s_00aee6c0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[34])
    }
    /// `StaticInit_PClassCallableMethodScript_00aee780`: class descriptor `PClassCallableMethodScript` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_PClassCallableMethodScript_00aee780 @ 0x00ae_e780 => extern "C" fn s_00aee780() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[35])
    }
    /// `StaticInit_CtorStore_00aee840`: constructor `0x0062_5850` on a global, its result stored in the global; atexit.
    StaticInit_CtorStore_00aee840 @ 0x00ae_e840 => extern "C" fn s_00aee840() -> u32 {
        si::ctor_store(&mut mem(), &si::CTOR_STORES[0])
    }
    /// `StaticInit_Ctor_00aee860`: no-argument constructor `0x0061_f1d0` on a global, then atexit.
    StaticInit_Ctor_00aee860 @ 0x00ae_e860 => extern "C" fn s_00aee860() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[0])
    }
    /// `StaticInit_PApplicationViewport_00aee880`: class descriptor `PApplicationViewport` (`cd_desc_set8`): constructor, buffers, set8; atexit.
    StaticInit_PApplicationViewport_00aee880 @ 0x00ae_e880 => extern "C" fn s_00aee880() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[36])
    }
    /// `StaticInit_CtorVt_00aee940`: no-argument constructor `0x0061_f310` on a global, flags &= ~2, its vtable, atexit.
    StaticInit_CtorVt_00aee940 @ 0x00ae_e940 => extern "C" fn s_00aee940() -> u32 {
        si::ctor0_flags_vt(&mut mem(), &si::CTOR0_FLAGS_VT[3])
    }
    /// `StaticInit_Atexit_00aee970`: only registers its destructor with atexit.
    StaticInit_Atexit_00aee970 @ 0x00ae_e970 => extern "C" fn s_00aee970() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[1])
    }
    /// `StaticInit_FreeList_PMapPair_00aee9d0`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00aee9d0 @ 0x00ae_e9d0 => extern "C" fn s_00aee9d0() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[0])
    }
    /// `StaticInit_Ctor_00aeea80`: no-argument constructor `0x0063_0f40` on a global, then atexit.
    StaticInit_Ctor_00aeea80 @ 0x00ae_ea80 => extern "C" fn s_00aeea80() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[1])
    }
    /// `StaticInit_MallocCopy_00aeeaf0`: `PhyreMalloc`(`0x11`, 2) copied from the template at `0x00b0_da48`, stored in `0x00cc_b6f0`; atexit.
    StaticInit_MallocCopy_00aeeaf0 @ 0x00ae_eaf0 => extern "C" fn s_00aeeaf0() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[0])
    }
    /// `StaticInit_MallocCopy_00aeeb40`: `PhyreMalloc`(`0x10`, 2) copied from the template at `0x00b0_da38`, stored in `0x00cc_b6ec`; atexit.
    StaticInit_MallocCopy_00aeeb40 @ 0x00ae_eb40 => extern "C" fn s_00aeeb40() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[1])
    }
    /// `StaticInit_Atexit_00aeeb90`: only registers its destructor with atexit.
    StaticInit_Atexit_00aeeb90 @ 0x00ae_eb90 => extern "C" fn s_00aeeb90() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[2])
    }
    /// `StaticInit_CtorArg_00aeeba0`: constructor `0x0063_8240` on a global with one argument (`0x32`); atexit.
    StaticInit_CtorArg_00aeeba0 @ 0x00ae_eba0 => extern "C" fn s_00aeeba0() -> u32 {
        si::ctor_arg(&mut mem(), &si::CTOR_ARGS[0])
    }
    /// `StaticInit_ImportStore_00aeebc0`: stdcall import through the slot `0x00b0_c198`, its result stored in `0x00cc_b6d0`; atexit.
    StaticInit_ImportStore_00aeebc0 @ 0x00ae_ebc0 => extern "C" fn s_00aeebc0() -> u32 {
        si::import_store(&mut mem(), &si::IMPORT_STORES[0])
    }
    /// `StaticInit_CutBuffer_00aeec30`: render pass type `CutBuffer` built on its global (name, flag 0); atexit.
    StaticInit_CutBuffer_00aeec30 @ 0x00ae_ec30 => extern "C" fn s_00aeec30() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[0])
    }
    /// `StaticInit_MallocCopy_00aeec50`: `PhyreMalloc`(`0x11`, 2) copied from the template at `0x00b0_da48`, stored in `0x00cc_b9e4`; atexit.
    StaticInit_MallocCopy_00aeec50 @ 0x00ae_ec50 => extern "C" fn s_00aeec50() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[2])
    }
    /// `StaticInit_MallocCopy_00aeeca0`: `PhyreMalloc`(`0x10`, 2) copied from the template at `0x00b0_da38`, stored in `0x00cc_b9e0`; atexit.
    StaticInit_MallocCopy_00aeeca0 @ 0x00ae_eca0 => extern "C" fn s_00aeeca0() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[3])
    }
    /// `StaticInit_ScissorPass_00aeecf0`: render pass type `ScissorPass` built on its global (name, flag 0); atexit.
    StaticInit_ScissorPass_00aeecf0 @ 0x00ae_ecf0 => extern "C" fn s_00aeecf0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[1])
    }
    /// `StaticInit_ClassDynamicMesh_00aeed60`: class descriptor `ClassDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_ClassDynamicMesh_00aeed60 @ 0x00ae_ed60 => extern "C" fn s_00aeed60() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[37])
    }
    /// `StaticInit_ClassDynamicMeshInstance_00aeee70`: class descriptor `ClassDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_ClassDynamicMeshInstance_00aeee70 @ 0x00ae_ee70 => extern "C" fn s_00aeee70() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[38])
    }
    /// `StaticInit_DistortionGridDynamicMesh_00aeefa0`: class descriptor `DistortionGridDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_DistortionGridDynamicMesh_00aeefa0 @ 0x00ae_efa0 => extern "C" fn s_00aeefa0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[39])
    }
    /// `StaticInit_DistortionGridDynamicMeshInstance_00aef0b0`: class descriptor `DistortionGridDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_DistortionGridDynamicMeshInstance_00aef0b0 @ 0x00ae_f0b0 => extern "C" fn s_00aef0b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[40])
    }
    /// `StaticInit_RadialLineDynamicMeshInstance_00aef1c0`: class descriptor `RadialLineDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_RadialLineDynamicMeshInstance_00aef1c0 @ 0x00ae_f1c0 => extern "C" fn s_00aef1c0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[41])
    }
    /// `StaticInit_RadialLineDynamicMesh_00aef2d0`: class descriptor `RadialLineDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_RadialLineDynamicMesh_00aef2d0 @ 0x00ae_f2d0 => extern "C" fn s_00aef2d0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[42])
    }
    /// `StaticInit_BrokenScreenPolygonDynamicMeshInstance_00aef3e0`: class descriptor `BrokenScreenPolygonDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_BrokenScreenPolygonDynamicMeshInstance_00aef3e0 @ 0x00ae_f3e0 => extern "C" fn s_00aef3e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[43])
    }
    /// `StaticInit_BrokenScreenPolygonDynamicMesh_00aef4f0`: class descriptor `BrokenScreenPolygonDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_BrokenScreenPolygonDynamicMesh_00aef4f0 @ 0x00ae_f4f0 => extern "C" fn s_00aef4f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[44])
    }
    /// `StaticInit_ShadowDynamicMesh_00aef600`: class descriptor `ShadowDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_ShadowDynamicMesh_00aef600 @ 0x00ae_f600 => extern "C" fn s_00aef600() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[45])
    }
    /// `StaticInit_Copy_00aef760`: copies the word at `0x00c3_3478` to `0x00cc_c7c8` and returns it.
    StaticInit_Copy_00aef760 @ 0x00ae_f760 => extern "C" fn s_00aef760() -> u32 {
        si::copy_word(&mut mem(), &si::COPY_WORDS[0])
    }
    /// `StaticInit_MallocCopy_00aef7c0`: `PhyreMalloc`(`0x11`, 2) copied from the template at `0x00b0_da48`, stored in `0x00cc_c82c`; atexit.
    StaticInit_MallocCopy_00aef7c0 @ 0x00ae_f7c0 => extern "C" fn s_00aef7c0() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[4])
    }
    /// `StaticInit_MallocCopy_00aef810`: `PhyreMalloc`(`0x10`, 2) copied from the template at `0x00b0_da38`, stored in `0x00cc_c828`; atexit.
    StaticInit_MallocCopy_00aef810 @ 0x00ae_f810 => extern "C" fn s_00aef810() -> u32 {
        si::malloc_copy(&mut mem(), &si::MALLOC_COPIES[5])
    }
    /// `StaticInit_CtorStore_00aef900`: constructor `0x0068_6bb0` on a global, its result stored in the global; atexit.
    StaticInit_CtorStore_00aef900 @ 0x00ae_f900 => extern "C" fn s_00aef900() -> u32 {
        si::ctor_store(&mut mem(), &si::CTOR_STORES[1])
    }
    /// `StaticInit_BlendingWeightCalculation_00aefab0`: render pass type `BlendingWeightCalculation` built on its global (name, flag 0); atexit.
    StaticInit_BlendingWeightCalculation_00aefab0 @ 0x00ae_fab0 => extern "C" fn s_00aefab0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[2])
    }
    /// `StaticInit_BrightBlend_00aefad0`: render pass type `BrightBlend` built on its global (name, flag 0); atexit.
    StaticInit_BrightBlend_00aefad0 @ 0x00ae_fad0 => extern "C" fn s_00aefad0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[3])
    }
    /// `StaticInit_ColorCorrection_00aefaf0`: render pass type `ColorCorrection` built on its global (name, flag 0); atexit.
    StaticInit_ColorCorrection_00aefaf0 @ 0x00ae_faf0 => extern "C" fn s_00aefaf0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[4])
    }
    /// `StaticInit_ColorEdgeDetection_00aefb10`: render pass type `ColorEdgeDetection` built on its global (name, flag 0); atexit.
    StaticInit_ColorEdgeDetection_00aefb10 @ 0x00ae_fb10 => extern "C" fn s_00aefb10() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[5])
    }
    /// `StaticInit_CopyBufferWithAdditive_00aefb30`: render pass type `CopyBufferWithAdditive` built on its global (name, flag 0); atexit.
    StaticInit_CopyBufferWithAdditive_00aefb30 @ 0x00ae_fb30 => extern "C" fn s_00aefb30() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[6])
    }
    /// `StaticInit_CopyBufferWithAlphaBlend_00aefb50`: render pass type `CopyBufferWithAlphaBlend` built on its global (name, flag 0); atexit.
    StaticInit_CopyBufferWithAlphaBlend_00aefb50 @ 0x00ae_fb50 => extern "C" fn s_00aefb50() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[7])
    }
    /// `StaticInit_CopyBuffer_00aefb70`: render pass type `CopyBuffer` built on its global (name, flag 0); atexit.
    StaticInit_CopyBuffer_00aefb70 @ 0x00ae_fb70 => extern "C" fn s_00aefb70() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[8])
    }
    /// `StaticInit_CopyDepthBuffer_00aefb90`: render pass type `CopyDepthBuffer` built on its global (name, flag 0); atexit.
    StaticInit_CopyDepthBuffer_00aefb90 @ 0x00ae_fb90 => extern "C" fn s_00aefb90() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[9])
    }
    /// `StaticInit_CopyRGBBufferWithAdditive_00aefbb0`: render pass type `CopyRGBBufferWithAdditive` built on its global (name, flag 0); atexit.
    StaticInit_CopyRGBBufferWithAdditive_00aefbb0 @ 0x00ae_fbb0 => extern "C" fn s_00aefbb0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[10])
    }
    /// `StaticInit_CopyRGBBufferWithAlphaBlend_00aefbd0`: render pass type `CopyRGBBufferWithAlphaBlend` built on its global (name, flag 0); atexit.
    StaticInit_CopyRGBBufferWithAlphaBlend_00aefbd0 @ 0x00ae_fbd0 => extern "C" fn s_00aefbd0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[11])
    }
    /// `StaticInit_GenerateGlowMap_00aefbf0`: render pass type `GenerateGlowMap` built on its global (name, flag 0); atexit.
    StaticInit_GenerateGlowMap_00aefbf0 @ 0x00ae_fbf0 => extern "C" fn s_00aefbf0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[12])
    }
    /// `StaticInit_HDAO_Blur_USM_00aefc10`: render pass type `HDAO_Blur_USM` built on its global (name, flag 0); atexit.
    StaticInit_HDAO_Blur_USM_00aefc10 @ 0x00ae_fc10 => extern "C" fn s_00aefc10() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[13])
    }
    /// `StaticInit_HDAO_Blur_00aefc30`: render pass type `HDAO_Blur` built on its global (name, flag 0); atexit.
    StaticInit_HDAO_Blur_00aefc30 @ 0x00ae_fc30 => extern "C" fn s_00aefc30() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[14])
    }
    /// `StaticInit_HDAO_BufferGen_00aefc50`: render pass type `HDAO_BufferGen` built on its global (name, flag 0); atexit.
    StaticInit_HDAO_BufferGen_00aefc50 @ 0x00ae_fc50 => extern "C" fn s_00aefc50() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[15])
    }
    /// `StaticInit_HDAO_BufferRender_00aefc70`: render pass type `HDAO_BufferRender` built on its global (name, flag 0); atexit.
    StaticInit_HDAO_BufferRender_00aefc70 @ 0x00ae_fc70 => extern "C" fn s_00aefc70() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[16])
    }
    /// `StaticInit_HDAO_Combine_00aefc90`: render pass type `HDAO_Combine` built on its global (name, flag 0); atexit.
    StaticInit_HDAO_Combine_00aefc90 @ 0x00ae_fc90 => extern "C" fn s_00aefc90() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[17])
    }
    /// `StaticInit_LumaEdgeDetection_00aefcb0`: render pass type `LumaEdgeDetection` built on its global (name, flag 0); atexit.
    StaticInit_LumaEdgeDetection_00aefcb0 @ 0x00ae_fcb0 => extern "C" fn s_00aefcb0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[18])
    }
    /// `StaticInit_NeighborhoodBlending_00aefcd0`: render pass type `NeighborhoodBlending` built on its global (name, flag 0); atexit.
    StaticInit_NeighborhoodBlending_00aefcd0 @ 0x00ae_fcd0 => extern "C" fn s_00aefcd0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[19])
    }
    /// `StaticInit_ResolveDepthBuffer2_00aefcf0`: render pass type `ResolveDepthBuffer2` built on its global (name, flag 0); atexit.
    StaticInit_ResolveDepthBuffer2_00aefcf0 @ 0x00ae_fcf0 => extern "C" fn s_00aefcf0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[20])
    }
    /// `StaticInit_ResolveDepthBuffer4_00aefd10`: render pass type `ResolveDepthBuffer4` built on its global (name, flag 0); atexit.
    StaticInit_ResolveDepthBuffer4_00aefd10 @ 0x00ae_fd10 => extern "C" fn s_00aefd10() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[21])
    }
    /// `StaticInit_ResolveDepthBuffer8_00aefd30`: render pass type `ResolveDepthBuffer8` built on its global (name, flag 0); atexit.
    StaticInit_ResolveDepthBuffer8_00aefd30 @ 0x00ae_fd30 => extern "C" fn s_00aefd30() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[22])
    }
    /// `StaticInit_ScreenFadeAdditive_00aefd50`: render pass type `ScreenFadeAdditive` built on its global (name, flag 0); atexit.
    StaticInit_ScreenFadeAdditive_00aefd50 @ 0x00ae_fd50 => extern "C" fn s_00aefd50() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[23])
    }
    /// `StaticInit_ScreenFadeSubtract_00aefd70`: render pass type `ScreenFadeSubtract` built on its global (name, flag 0); atexit.
    StaticInit_ScreenFadeSubtract_00aefd70 @ 0x00ae_fd70 => extern "C" fn s_00aefd70() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[24])
    }
    /// `StaticInit_ScreenFade_00aefd90`: render pass type `ScreenFade` built on its global (name, flag 0); atexit.
    StaticInit_ScreenFade_00aefd90 @ 0x00ae_fd90 => extern "C" fn s_00aefd90() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[25])
    }
    /// `StaticInit_USM_Apply_00aefdb0`: render pass type `USM_Apply` built on its global (name, flag 0); atexit.
    StaticInit_USM_Apply_00aefdb0 @ 0x00ae_fdb0 => extern "C" fn s_00aefdb0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[26])
    }
    /// `StaticInit_USM_Blur_00aefdd0`: render pass type `USM_Blur` built on its global (name, flag 0); atexit.
    StaticInit_USM_Blur_00aefdd0 @ 0x00ae_fdd0 => extern "C" fn s_00aefdd0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[27])
    }
    /// `StaticInit_ImportStore_00aefe90`: stdcall import through the slot `0x00b0_c198`, its result stored in `0x00cc_ca78`; atexit.
    StaticInit_ImportStore_00aefe90 @ 0x00ae_fe90 => extern "C" fn s_00aefe90() -> u32 {
        si::import_store(&mut mem(), &si::IMPORT_STORES[1])
    }
    /// `StaticInit_FreeList_PMapPair_00aefeb0`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00aefeb0 @ 0x00ae_feb0 => extern "C" fn s_00aefeb0() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[1])
    }
    /// `StaticInit_ImportStore_00aeff10`: stdcall import through the slot `0x00b0_c198`, its result stored in `0x00cc_caa0`; atexit.
    StaticInit_ImportStore_00aeff10 @ 0x00ae_ff10 => extern "C" fn s_00aeff10() -> u32 {
        si::import_store(&mut mem(), &si::IMPORT_STORES[2])
    }
    /// `StaticInit_DistortionCopyBuffer_00af0110`: render pass type `DistortionCopyBuffer` built on its global (name, flag 0); atexit.
    StaticInit_DistortionCopyBuffer_00af0110 @ 0x00af_0110 => extern "C" fn s_00af0110() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[28])
    }
    /// `StaticInit_ApplyBufferWithColor_00af0130`: render pass type `ApplyBufferWithColor` built on its global (name, flag 0); atexit.
    StaticInit_ApplyBufferWithColor_00af0130 @ 0x00af_0130 => extern "C" fn s_00af0130() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[29])
    }
    /// `StaticInit_ApplyBuffer_00af0150`: render pass type `ApplyBuffer` built on its global (name, flag 0); atexit.
    StaticInit_ApplyBuffer_00af0150 @ 0x00af_0150 => extern "C" fn s_00af0150() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[30])
    }
    /// `StaticInit_ApplyScratch_00af0170`: render pass type `ApplyScratch` built on its global (name, flag 0); atexit.
    StaticInit_ApplyScratch_00af0170 @ 0x00af_0170 => extern "C" fn s_00af0170() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[31])
    }
    /// `StaticInit_CopyBufferExWithColor_00af0190`: render pass type `CopyBufferExWithColor` built on its global (name, flag 0); atexit.
    StaticInit_CopyBufferExWithColor_00af0190 @ 0x00af_0190 => extern "C" fn s_00af0190() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[32])
    }
    /// `StaticInit_CopyBufferEx_00af01b0`: render pass type `CopyBufferEx` built on its global (name, flag 0); atexit.
    StaticInit_CopyBufferEx_00af01b0 @ 0x00af_01b0 => extern "C" fn s_00af01b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[33])
    }
    /// `StaticInit_BlendFrame_00af0220`: render pass type `BlendFrame` built on its global (name, flag 0); atexit.
    StaticInit_BlendFrame_00af0220 @ 0x00af_0220 => extern "C" fn s_00af0220() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[34])
    }
    /// `StaticInit_ShadowGaussianBlurX_FromDepth_00af02e0`: render pass type `ShadowGaussianBlurX_FromDepth` built on its global (name, flag 0); atexit.
    StaticInit_ShadowGaussianBlurX_FromDepth_00af02e0 @ 0x00af_02e0 => extern "C" fn s_00af02e0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[35])
    }
    /// `StaticInit_ShadowGaussianBlurX_00af0300`: render pass type `ShadowGaussianBlurX` built on its global (name, flag 0); atexit.
    StaticInit_ShadowGaussianBlurX_00af0300 @ 0x00af_0300 => extern "C" fn s_00af0300() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[36])
    }
    /// `StaticInit_ShadowGaussianBlurY_Adv_00af0320`: render pass type `ShadowGaussianBlurY_Adv` built on its global (name, flag 0); atexit.
    StaticInit_ShadowGaussianBlurY_Adv_00af0320 @ 0x00af_0320 => extern "C" fn s_00af0320() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[37])
    }
    /// `StaticInit_ShadowGaussianBlurY_FromDepth_00af0340`: render pass type `ShadowGaussianBlurY_FromDepth` built on its global (name, flag 0); atexit.
    StaticInit_ShadowGaussianBlurY_FromDepth_00af0340 @ 0x00af_0340 => extern "C" fn s_00af0340() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[38])
    }
    /// `StaticInit_ShadowGaussianBlurY_00af0360`: render pass type `ShadowGaussianBlurY` built on its global (name, flag 0); atexit.
    StaticInit_ShadowGaussianBlurY_00af0360 @ 0x00af_0360 => extern "C" fn s_00af0360() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[39])
    }
    /// `StaticInit_Float_00af0380`: float global `0x00cd_ec2c` = 1 - (a - b) / (a - c) of the constants at `0x00c3_43f0`, `0x00c3_43f4`, `0x00c3_43ec`, rounded to float.
    StaticInit_Float_00af0380 @ 0x00af_0380 => extern "C" fn s_00af0380() {
        si::float_row(&mut mem(), &si::FLOATS[0]);
    }
    /// `StaticInit_Float_00af03b0`: float global `0x00cd_ec34` = (a - b) / (c - b) of the constants at `0x00c3_43f0`, `0x00c3_43f4`, `0x00c3_43f8`, rounded to float.
    StaticInit_Float_00af03b0 @ 0x00af_03b0 => extern "C" fn s_00af03b0() {
        si::float_row(&mut mem(), &si::FLOATS[1]);
    }
    /// `StaticInit_AdditiveColor_00af0420`: render pass type `AdditiveColor` built on its global (name, flag 0); atexit.
    StaticInit_AdditiveColor_00af0420 @ 0x00af_0420 => extern "C" fn s_00af0420() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[40])
    }
    /// `StaticInit_ApplyColor_00af0440`: render pass type `ApplyColor` built on its global (name, flag 0); atexit.
    StaticInit_ApplyColor_00af0440 @ 0x00af_0440 => extern "C" fn s_00af0440() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[41])
    }
    /// `StaticInit_ApplyTextureWithColor_00af0460`: render pass type `ApplyTextureWithColor` built on its global (name, flag 0); atexit.
    StaticInit_ApplyTextureWithColor_00af0460 @ 0x00af_0460 => extern "C" fn s_00af0460() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[42])
    }
    /// `StaticInit_ApplyTexture_00af0480`: render pass type `ApplyTexture` built on its global (name, flag 0); atexit.
    StaticInit_ApplyTexture_00af0480 @ 0x00af_0480 => extern "C" fn s_00af0480() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[43])
    }
    /// `StaticInit_CopyColor_00af04a0`: render pass type `CopyColor` built on its global (name, flag 0); atexit.
    StaticInit_CopyColor_00af04a0 @ 0x00af_04a0 => extern "C" fn s_00af04a0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[44])
    }
    /// `StaticInit_CopyTextureWithAdditiveColor_00af04c0`: render pass type `CopyTextureWithAdditiveColor` built on its global (name, flag 0); atexit.
    StaticInit_CopyTextureWithAdditiveColor_00af04c0 @ 0x00af_04c0 => extern "C" fn s_00af04c0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[45])
    }
    /// `StaticInit_CopyTextureWithColor_00af04e0`: render pass type `CopyTextureWithColor` built on its global (name, flag 0); atexit.
    StaticInit_CopyTextureWithColor_00af04e0 @ 0x00af_04e0 => extern "C" fn s_00af04e0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[46])
    }
    /// `StaticInit_CopyTexture_00af0500`: render pass type `CopyTexture` built on its global (name, flag 0); atexit.
    StaticInit_CopyTexture_00af0500 @ 0x00af_0500 => extern "C" fn s_00af0500() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[47])
    }
    /// `StaticInit_PaintColor_00af0520`: render pass type `PaintColor` built on its global (name, flag 0); atexit.
    StaticInit_PaintColor_00af0520 @ 0x00af_0520 => extern "C" fn s_00af0520() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[48])
    }
    /// `StaticInit_VideoScreen_00af0590`: render pass type `VideoScreen` built on its global (name, flag 0); atexit.
    StaticInit_VideoScreen_00af0590 @ 0x00af_0590 => extern "C" fn s_00af0590() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[49])
    }
    /// `StaticInit_Sentinel_00af06a0`: list object `0x00cd_ee20` constructed, then its sentinel `0x00cd_ee68` links to itself; atexit.
    StaticInit_Sentinel_00af06a0 @ 0x00af_06a0 => extern "C" fn s_00af06a0() -> u32 {
        si::sentinel(&mut mem(), &si::SENTINELS[0])
    }
    /// `StaticInit_FreeList_PMapPair_00af06d0`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00af06d0 @ 0x00af_06d0 => extern "C" fn s_00af06d0() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[2])
    }
    /// `StaticInit_FreeList_PMapPair_00af0730`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00af0730 @ 0x00af_0730 => extern "C" fn s_00af0730() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[3])
    }
    /// `StaticInit_ClothInstancingDynamicMesh_00af0790`: class descriptor `ClothInstancingDynamicMesh` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_ClothInstancingDynamicMesh_00af0790 @ 0x00af_0790 => extern "C" fn s_00af0790() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[46])
    }
    /// `StaticInit_ClothInstancingDynamicMeshInstance_00af0850`: class descriptor `ClothInstancingDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_ClothInstancingDynamicMeshInstance_00af0850 @ 0x00af_0850 => extern "C" fn s_00af0850() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[47])
    }
    /// `StaticInit_DynamicMeshDefaultImplmenetation_00af0910`: class descriptor `DynamicMeshDefaultImplmenetation` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_DynamicMeshDefaultImplmenetation_00af0910 @ 0x00af_0910 => extern "C" fn s_00af0910() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[48])
    }
    /// `StaticInit_Atexit_00af09d0`: only registers its destructor with atexit.
    StaticInit_Atexit_00af09d0 @ 0x00af_09d0 => extern "C" fn s_00af09d0() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[3])
    }
    /// `StaticInit_Sentinel_00af0b20`: list object `0x00ce_6d70` constructed, then its sentinel `0x00ce_6db8` links to itself; atexit.
    StaticInit_Sentinel_00af0b20 @ 0x00af_0b20 => extern "C" fn s_00af0b20() -> u32 {
        si::sentinel(&mut mem(), &si::SENTINELS[1])
    }
    /// `StaticInit_FreeList_PMapPair_00af0b50`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00af0b50 @ 0x00af_0b50 => extern "C" fn s_00af0b50() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[4])
    }
    /// `StaticInit_FreeList_PMapPair_00af0bb0`: free list of `PMapPair` built on a global (SEH frame); atexit.
    StaticInit_FreeList_PMapPair_00af0bb0 @ 0x00af_0bb0 => extern "C" fn s_00af0bb0() -> u32 {
        si::free_list(&mut mem(), &si::FREE_LISTS[5])
    }
    /// `StaticInit_ExtraPass_00af0cb0`: render pass type `ExtraPass` built on its global (name, flag 0); atexit.
    StaticInit_ExtraPass_00af0cb0 @ 0x00af_0cb0 => extern "C" fn s_00af0cb0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[50])
    }
    /// `StaticInit_CtorStore_00af0d70`: constructor `0x0065_4ea0` on a global, its result stored in the global; atexit.
    StaticInit_CtorStore_00af0d70 @ 0x00af_0d70 => extern "C" fn s_00af0d70() -> u32 {
        si::ctor_store(&mut mem(), &si::CTOR_STORES[2])
    }
    /// `StaticInit_Ctor_00af0e10`: no-argument constructor `0x0058_3b20` on a global, then atexit.
    StaticInit_Ctor_00af0e10 @ 0x00af_0e10 => extern "C" fn s_00af0e10() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[2])
    }
    /// `StaticInit_Ctor_00af0e30`: no-argument constructor `0x0058_3b20` on a global, then atexit.
    StaticInit_Ctor_00af0e30 @ 0x00af_0e30 => extern "C" fn s_00af0e30() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[3])
    }
    /// `StaticInit_ShadowDynamicMeshInstance_00af0f40`: class descriptor `ShadowDynamicMeshInstance` (`cd_const_set8`): constructor, buffers, set8; atexit.
    StaticInit_ShadowDynamicMeshInstance_00af0f40 @ 0x00af_0f40 => extern "C" fn s_00af0f40() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[49])
    }
    /// `StaticInit_CtorStore_00af1230`: constructor `0x0080_6b20` on a global, its result stored in the global; atexit.
    StaticInit_CtorStore_00af1230 @ 0x00af_1230 => extern "C" fn s_00af1230() -> u32 {
        si::ctor_store(&mut mem(), &si::CTOR_STORES[3])
    }
    /// `StaticInit_CtorStore_00af1250`: constructor `0x0080_6af0` on a global, its result stored in the global; atexit.
    StaticInit_CtorStore_00af1250 @ 0x00af_1250 => extern "C" fn s_00af1250() -> u32 {
        si::ctor_store(&mut mem(), &si::CTOR_STORES[4])
    }
    /// `StaticInit_ImportStore_00af1270`: stdcall import through the slot `0x00b0_c11c`, its result stored in `0x012f_410c`; atexit.
    StaticInit_ImportStore_00af1270 @ 0x00af_1270 => extern "C" fn s_00af1270() -> u32 {
        si::import_store(&mut mem(), &si::IMPORT_STORES[3])
    }
    /// `StaticInit_CondTail_00af1290`: jumps to `0x0094_28b0` when the word at `0x0193_cd2c` is zero, else returns.
    StaticInit_CondTail_00af1290 @ 0x00af_1290 => extern "C" fn s_00af1290() -> u32 {
        si::cond_tail(&mut mem(), &si::COND_TAILS[0])
    }
    /// `StaticInit_Atexit_00af12a0`: only registers its destructor with atexit.
    StaticInit_Atexit_00af12a0 @ 0x00af_12a0 => extern "C" fn s_00af12a0() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[4])
    }
    /// `StaticInit_Atexit_00af12ac`: only registers its destructor with atexit.
    StaticInit_Atexit_00af12ac @ 0x00af_12ac => extern "C" fn s_00af12ac() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[5])
    }
    /// `StaticInit_ChainAppend_00af12c0`: appends the object at `0x00c6_13f0` to the chain at `0xc901a8` (before the first block at least `0xff` full); atexit.
    StaticInit_ChainAppend_00af12c0 @ 0x00af_12c0 => extern "C" fn s_00af12c0() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[1])
    }
    /// `StaticInit_Ctor_00af1308`: no-argument constructor `0x009d_9c20` on a global, then atexit.
    StaticInit_Ctor_00af1308 @ 0x00af_1308 => extern "C" fn s_00af1308() -> u32 {
        si::ctor0_at_exit(&mut mem(), &si::CTOR0_AT_EXIT[4])
    }
    /// `StaticInit_ImportObject_00af131e`: object `0x0193_fcfc` from the import at `0x00b0_c060` (vtable, handle, false); atexit.
    StaticInit_ImportObject_00af131e @ 0x00af_131e => extern "C" fn s_00af131e() -> u32 {
        si::import_object(&mut mem(), &si::IMPORT_OBJECTS[0])
    }
    /// `StaticInit_Atexit_00af1346`: only registers its destructor with atexit.
    StaticInit_Atexit_00af1346 @ 0x00af_1346 => extern "C" fn s_00af1346() -> u32 {
        si::at_exit_only(&mut mem(), &si::AT_EXIT_ONLY[6])
    }
    /// `StaticInit_PInputAction_00af1360`: class descriptor `PInputAction` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputAction_00af1360 @ 0x00af_1360 => extern "C" fn s_00af1360() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[50])
    }
    /// `StaticInit_PInputMap_00af1420`: class descriptor `PInputMap` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputMap_00af1420 @ 0x00af_1420 => extern "C" fn s_00af1420() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[51])
    }
    /// `StaticInit_JoypadAxis_00af2670`: type-enum value `JoypadAxis`: when 0x009f48f0 finds no such name, the node 0x00c68240 goes on the head of its list 0x0194359c (old head to 0x00c68248); atexit(0x00b08380).
    StaticInit_JoypadAxis_00af2670 @ 0x00af_2670 => extern "C" fn s_00af2670() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[0])
    }
    /// `StaticInit_JoypadButton_00af26b0`: type-enum value `JoypadButton`: when 0x009f48f0 finds no such name, the node 0x00c68234 goes on the head of its list 0x0194359c (old head to 0x00c6823c); atexit(0x00b083a0).
    StaticInit_JoypadButton_00af26b0 @ 0x00af_26b0 => extern "C" fn s_00af26b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[1])
    }
    /// `StaticInit_Key_00af26f0`: type-enum value `Key`: when 0x009f48f0 finds no such name, the node 0x00c68228 goes on the head of its list 0x0194359c (old head to 0x00c68230); atexit(0x00b083c0).
    StaticInit_Key_00af26f0 @ 0x00af_26f0 => extern "C" fn s_00af26f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[2])
    }
    /// `StaticInit_MotionAngularVelocityX_00af2730`: type-enum value `MotionAngularVelocityX`: when 0x009f48f0 finds no such name, the node 0x00c682e8 goes on the head of its list 0x0194359c (old head to 0x00c682f0); atexit(0x00b083e0).
    StaticInit_MotionAngularVelocityX_00af2730 @ 0x00af_2730 => extern "C" fn s_00af2730() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[3])
    }
    /// `StaticInit_MotionAngularVelocityY_00af2770`: type-enum value `MotionAngularVelocityY`: when 0x009f48f0 finds no such name, the node 0x00c682f4 goes on the head of its list 0x0194359c (old head to 0x00c682fc); atexit(0x00b08400).
    StaticInit_MotionAngularVelocityY_00af2770 @ 0x00af_2770 => extern "C" fn s_00af2770() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[4])
    }
    /// `StaticInit_MotionAngularVelocityZ_00af27b0`: type-enum value `MotionAngularVelocityZ`: when 0x009f48f0 finds no such name, the node 0x00c68300 goes on the head of its list 0x0194359c (old head to 0x00c68308); atexit(0x00b08420).
    StaticInit_MotionAngularVelocityZ_00af27b0 @ 0x00af_27b0 => extern "C" fn s_00af27b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[5])
    }
    /// `StaticInit_MotionLinearAccelerationX_00af27f0`: type-enum value `MotionLinearAccelerationX`: when 0x009f48f0 finds no such name, the node 0x00c6830c goes on the head of its list 0x0194359c (old head to 0x00c68314); atexit(0x00b08440).
    StaticInit_MotionLinearAccelerationX_00af27f0 @ 0x00af_27f0 => extern "C" fn s_00af27f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[6])
    }
    /// `StaticInit_MotionLinearAccelerationY_00af2830`: type-enum value `MotionLinearAccelerationY`: when 0x009f48f0 finds no such name, the node 0x00c68318 goes on the head of its list 0x0194359c (old head to 0x00c68320); atexit(0x00b08460).
    StaticInit_MotionLinearAccelerationY_00af2830 @ 0x00af_2830 => extern "C" fn s_00af2830() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[7])
    }
    /// `StaticInit_MotionLinearAccelerationZ_00af2870`: type-enum value `MotionLinearAccelerationZ`: when 0x009f48f0 finds no such name, the node 0x00c68324 goes on the head of its list 0x0194359c (old head to 0x00c6832c); atexit(0x00b08480).
    StaticInit_MotionLinearAccelerationZ_00af2870 @ 0x00af_2870 => extern "C" fn s_00af2870() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[8])
    }
    /// `StaticInit_MotionQuatW_00af28b0`: type-enum value `MotionQuatW`: when 0x009f48f0 finds no such name, the node 0x00c682dc goes on the head of its list 0x0194359c (old head to 0x00c682e4); atexit(0x00b084a0).
    StaticInit_MotionQuatW_00af28b0 @ 0x00af_28b0 => extern "C" fn s_00af28b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[9])
    }
    /// `StaticInit_MotionQuatX_00af28f0`: type-enum value `MotionQuatX`: when 0x009f48f0 finds no such name, the node 0x00c682b8 goes on the head of its list 0x0194359c (old head to 0x00c682c0); atexit(0x00b084c0).
    StaticInit_MotionQuatX_00af28f0 @ 0x00af_28f0 => extern "C" fn s_00af28f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[10])
    }
    /// `StaticInit_MotionQuatY_00af2930`: type-enum value `MotionQuatY`: when 0x009f48f0 finds no such name, the node 0x00c682c4 goes on the head of its list 0x0194359c (old head to 0x00c682cc); atexit(0x00b084e0).
    StaticInit_MotionQuatY_00af2930 @ 0x00af_2930 => extern "C" fn s_00af2930() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[11])
    }
    /// `StaticInit_MotionQuatZ_00af2970`: type-enum value `MotionQuatZ`: when 0x009f48f0 finds no such name, the node 0x00c682d0 goes on the head of its list 0x0194359c (old head to 0x00c682d8); atexit(0x00b08500).
    StaticInit_MotionQuatZ_00af2970 @ 0x00af_2970 => extern "C" fn s_00af2970() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[12])
    }
    /// `StaticInit_MouseButton_00af29b0`: type-enum value `MouseButton`: when 0x009f48f0 finds no such name, the node 0x00c6824c goes on the head of its list 0x0194359c (old head to 0x00c68254); atexit(0x00b08520).
    StaticInit_MouseButton_00af29b0 @ 0x00af_29b0 => extern "C" fn s_00af29b0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[13])
    }
    /// `StaticInit_MouseDeltaX_00af29f0`: type-enum value `MouseDeltaX`: when 0x009f48f0 finds no such name, the node 0x00c68258 goes on the head of its list 0x0194359c (old head to 0x00c68260); atexit(0x00b08540).
    StaticInit_MouseDeltaX_00af29f0 @ 0x00af_29f0 => extern "C" fn s_00af29f0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[14])
    }
    /// `StaticInit_MouseDeltaY_00af2a30`: type-enum value `MouseDeltaY`: when 0x009f48f0 finds no such name, the node 0x00c68264 goes on the head of its list 0x0194359c (old head to 0x00c6826c); atexit(0x00b08560).
    StaticInit_MouseDeltaY_00af2a30 @ 0x00af_2a30 => extern "C" fn s_00af2a30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[15])
    }
    /// `StaticInit_TouchDragX_00af2a70`: type-enum value `TouchDragX`: when 0x009f48f0 finds no such name, the node 0x00c68270 goes on the head of its list 0x0194359c (old head to 0x00c68278); atexit(0x00b08580).
    StaticInit_TouchDragX_00af2a70 @ 0x00af_2a70 => extern "C" fn s_00af2a70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[16])
    }
    /// `StaticInit_TouchDragY_00af2ab0`: type-enum value `TouchDragY`: when 0x009f48f0 finds no such name, the node 0x00c6827c goes on the head of its list 0x0194359c (old head to 0x00c68284); atexit(0x00b085a0).
    StaticInit_TouchDragY_00af2ab0 @ 0x00af_2ab0 => extern "C" fn s_00af2ab0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[17])
    }
    /// `StaticInit_TouchPinch_00af2af0`: type-enum value `TouchPinch`: when 0x009f48f0 finds no such name, the node 0x00c682a0 goes on the head of its list 0x0194359c (old head to 0x00c682a8); atexit(0x00b085c0).
    StaticInit_TouchPinch_00af2af0 @ 0x00af_2af0 => extern "C" fn s_00af2af0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[18])
    }
    /// `StaticInit_TouchRotate_00af2b30`: type-enum value `TouchRotate`: when 0x009f48f0 finds no such name, the node 0x00c682ac goes on the head of its list 0x0194359c (old head to 0x00c682b4); atexit(0x00b085e0).
    StaticInit_TouchRotate_00af2b30 @ 0x00af_2b30 => extern "C" fn s_00af2b30() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[19])
    }
    /// `StaticInit_TouchTwoFingerDragX_00af2b70`: type-enum value `TouchTwoFingerDragX`: when 0x009f48f0 finds no such name, the node 0x00c68288 goes on the head of its list 0x0194359c (old head to 0x00c68290); atexit(0x00b08600).
    StaticInit_TouchTwoFingerDragX_00af2b70 @ 0x00af_2b70 => extern "C" fn s_00af2b70() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[20])
    }
    /// `StaticInit_TouchTwoFingerDragY_00af2bb0`: type-enum value `TouchTwoFingerDragY`: when 0x009f48f0 finds no such name, the node 0x00c68294 goes on the head of its list 0x0194359c (old head to 0x00c6829c); atexit(0x00b08620).
    StaticInit_TouchTwoFingerDragY_00af2bb0 @ 0x00af_2bb0 => extern "C" fn s_00af2bb0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[21])
    }
    /// `StaticInit_Gyro_00af4c50`: type-enum value `Gyro`: when 0x00a2dea0 finds no such name, the node 0x00c6b6b4 goes on the head of its list 0x01a85318 (old head to 0x00c6b6bc); atexit(0x00b09220).
    StaticInit_Gyro_00af4c50 @ 0x00af_4c50 => extern "C" fn s_00af4c50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[22])
    }
    /// `StaticInit_XAxis_0_00af4c90`: type-enum value `XAxis_0`: when 0x00a2dea0 finds no such name, the node 0x00c6b744 goes on the head of its list 0x01a85318 (old head to 0x00c6b74c); atexit(0x00b09240).
    StaticInit_XAxis_0_00af4c90 @ 0x00af_4c90 => extern "C" fn s_00af4c90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[23])
    }
    /// `StaticInit_XAxis_1_00af4cd0`: type-enum value `XAxis_1`: when 0x00a2dea0 finds no such name, the node 0x00c6b714 goes on the head of its list 0x01a85318 (old head to 0x00c6b71c); atexit(0x00b09260).
    StaticInit_XAxis_1_00af4cd0 @ 0x00af_4cd0 => extern "C" fn s_00af4cd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[24])
    }
    /// `StaticInit_XAxis_2_00af4d10`: type-enum value `XAxis_2`: when 0x00a2dea0 finds no such name, the node 0x00c6b6e4 goes on the head of its list 0x01a85318 (old head to 0x00c6b6ec); atexit(0x00b09280).
    StaticInit_XAxis_2_00af4d10 @ 0x00af_4d10 => extern "C" fn s_00af4d10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[25])
    }
    /// `StaticInit_YAxis_0_00af4d50`: type-enum value `YAxis_0`: when 0x00a2dea0 finds no such name, the node 0x00c6b734 goes on the head of its list 0x01a85318 (old head to 0x00c6b73c); atexit(0x00b092a0).
    StaticInit_YAxis_0_00af4d50 @ 0x00af_4d50 => extern "C" fn s_00af4d50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[26])
    }
    /// `StaticInit_YAxis_1_00af4d90`: type-enum value `YAxis_1`: when 0x00a2dea0 finds no such name, the node 0x00c6b704 goes on the head of its list 0x01a85318 (old head to 0x00c6b70c); atexit(0x00b092c0).
    StaticInit_YAxis_1_00af4d90 @ 0x00af_4d90 => extern "C" fn s_00af4d90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[27])
    }
    /// `StaticInit_YAxis_2_00af4dd0`: type-enum value `YAxis_2`: when 0x00a2dea0 finds no such name, the node 0x00c6b6d4 goes on the head of its list 0x01a85318 (old head to 0x00c6b6dc); atexit(0x00b092e0).
    StaticInit_YAxis_2_00af4dd0 @ 0x00af_4dd0 => extern "C" fn s_00af4dd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[28])
    }
    /// `StaticInit_ZAxis_0_00af4e10`: type-enum value `ZAxis_0`: when 0x00a2dea0 finds no such name, the node 0x00c6b724 goes on the head of its list 0x01a85318 (old head to 0x00c6b72c); atexit(0x00b09300).
    StaticInit_ZAxis_0_00af4e10 @ 0x00af_4e10 => extern "C" fn s_00af4e10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[29])
    }
    /// `StaticInit_ZAxis_1_00af4e50`: type-enum value `ZAxis_1`: when 0x00a2dea0 finds no such name, the node 0x00c6b6f4 goes on the head of its list 0x01a85318 (old head to 0x00c6b6fc); atexit(0x00b09320).
    StaticInit_ZAxis_1_00af4e50 @ 0x00af_4e50 => extern "C" fn s_00af4e50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[30])
    }
    /// `StaticInit_ZAxis_2_00af4e90`: type-enum value `ZAxis_2`: when 0x00a2dea0 finds no such name, the node 0x00c6b6c4 goes on the head of its list 0x01a85318 (old head to 0x00c6b6cc); atexit(0x00b09340).
    StaticInit_ZAxis_2_00af4e90 @ 0x00af_4e90 => extern "C" fn s_00af4e90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[31])
    }
    /// `StaticInit_Button_0_00af4ed0`: type-enum value `Button_0`: when 0x00a2e310 finds no such name, the node 0x00c6bb34 goes on the head of its list 0x01a85354 (old head to 0x00c6bb3c); atexit(0x00b09370).
    StaticInit_Button_0_00af4ed0 @ 0x00af_4ed0 => extern "C" fn s_00af4ed0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[32])
    }
    /// `StaticInit_Button_10_00af4f10`: type-enum value `Button_10`: when 0x00a2e310 finds no such name, the node 0x00c6b964 goes on the head of its list 0x01a85354 (old head to 0x00c6b96c); atexit(0x00b09390).
    StaticInit_Button_10_00af4f10 @ 0x00af_4f10 => extern "C" fn s_00af4f10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[33])
    }
    /// `StaticInit_Button_11_00af4f50`: type-enum value `Button_11`: when 0x00a2e310 finds no such name, the node 0x00c6b934 goes on the head of its list 0x01a85354 (old head to 0x00c6b93c); atexit(0x00b093b0).
    StaticInit_Button_11_00af4f50 @ 0x00af_4f50 => extern "C" fn s_00af4f50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[34])
    }
    /// `StaticInit_Button_12_00af4f90`: type-enum value `Button_12`: when 0x00a2e310 finds no such name, the node 0x00c6b904 goes on the head of its list 0x01a85354 (old head to 0x00c6b90c); atexit(0x00b093d0).
    StaticInit_Button_12_00af4f90 @ 0x00af_4f90 => extern "C" fn s_00af4f90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[35])
    }
    /// `StaticInit_Button_13_00af4fd0`: type-enum value `Button_13`: when 0x00a2e310 finds no such name, the node 0x00c6b8e4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8ec); atexit(0x00b093f0).
    StaticInit_Button_13_00af4fd0 @ 0x00af_4fd0 => extern "C" fn s_00af4fd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[36])
    }
    /// `StaticInit_Button_14_00af5010`: type-enum value `Button_14`: when 0x00a2e310 finds no such name, the node 0x00c6b8c4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8cc); atexit(0x00b09410).
    StaticInit_Button_14_00af5010 @ 0x00af_5010 => extern "C" fn s_00af5010() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[37])
    }
    /// `StaticInit_Button_15_00af5050`: type-enum value `Button_15`: when 0x00a2e310 finds no such name, the node 0x00c6b8a4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8ac); atexit(0x00b09430).
    StaticInit_Button_15_00af5050 @ 0x00af_5050 => extern "C" fn s_00af5050() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[38])
    }
    /// `StaticInit_Button_1_00af5090`: type-enum value `Button_1`: when 0x00a2e310 finds no such name, the node 0x00c6bb04 goes on the head of its list 0x01a85354 (old head to 0x00c6bb0c); atexit(0x00b09450).
    StaticInit_Button_1_00af5090 @ 0x00af_5090 => extern "C" fn s_00af5090() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[39])
    }
    /// `StaticInit_Button_2_00af50d0`: type-enum value `Button_2`: when 0x00a2e310 finds no such name, the node 0x00c6bad4 goes on the head of its list 0x01a85354 (old head to 0x00c6badc); atexit(0x00b09470).
    StaticInit_Button_2_00af50d0 @ 0x00af_50d0 => extern "C" fn s_00af50d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[40])
    }
    /// `StaticInit_Button_3_00af5110`: type-enum value `Button_3`: when 0x00a2e310 finds no such name, the node 0x00c6baa4 goes on the head of its list 0x01a85354 (old head to 0x00c6baac); atexit(0x00b09490).
    StaticInit_Button_3_00af5110 @ 0x00af_5110 => extern "C" fn s_00af5110() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[41])
    }
    /// `StaticInit_Button_4_00af5150`: type-enum value `Button_4`: when 0x00a2e310 finds no such name, the node 0x00c6ba74 goes on the head of its list 0x01a85354 (old head to 0x00c6ba7c); atexit(0x00b094b0).
    StaticInit_Button_4_00af5150 @ 0x00af_5150 => extern "C" fn s_00af5150() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[42])
    }
    /// `StaticInit_Button_5_00af5190`: type-enum value `Button_5`: when 0x00a2e310 finds no such name, the node 0x00c6ba44 goes on the head of its list 0x01a85354 (old head to 0x00c6ba4c); atexit(0x00b094d0).
    StaticInit_Button_5_00af5190 @ 0x00af_5190 => extern "C" fn s_00af5190() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[43])
    }
    /// `StaticInit_Button_6_00af51d0`: type-enum value `Button_6`: when 0x00a2e310 finds no such name, the node 0x00c6ba14 goes on the head of its list 0x01a85354 (old head to 0x00c6ba1c); atexit(0x00b094f0).
    StaticInit_Button_6_00af51d0 @ 0x00af_51d0 => extern "C" fn s_00af51d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[44])
    }
    /// `StaticInit_Button_7_00af5210`: type-enum value `Button_7`: when 0x00a2e310 finds no such name, the node 0x00c6b9e4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9ec); atexit(0x00b09510).
    StaticInit_Button_7_00af5210 @ 0x00af_5210 => extern "C" fn s_00af5210() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[45])
    }
    /// `StaticInit_Button_8_00af5250`: type-enum value `Button_8`: when 0x00a2e310 finds no such name, the node 0x00c6b9b4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9bc); atexit(0x00b09530).
    StaticInit_Button_8_00af5250 @ 0x00af_5250 => extern "C" fn s_00af5250() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[46])
    }
    /// `StaticInit_Button_9_00af5290`: type-enum value `Button_9`: when 0x00a2e310 finds no such name, the node 0x00c6b984 goes on the head of its list 0x01a85354 (old head to 0x00c6b98c); atexit(0x00b09550).
    StaticInit_Button_9_00af5290 @ 0x00af_5290 => extern "C" fn s_00af5290() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[47])
    }
    /// `StaticInit_Button_A_00af52d0`: type-enum value `Button_A`: when 0x00a2e310 finds no such name, the node 0x00c6bae4 goes on the head of its list 0x01a85354 (old head to 0x00c6baec); atexit(0x00b09570).
    StaticInit_Button_A_00af52d0 @ 0x00af_52d0 => extern "C" fn s_00af52d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[48])
    }
    /// `StaticInit_Button_B_00af5310`: type-enum value `Button_B`: when 0x00a2e310 finds no such name, the node 0x00c6bab4 goes on the head of its list 0x01a85354 (old head to 0x00c6babc); atexit(0x00b09590).
    StaticInit_Button_B_00af5310 @ 0x00af_5310 => extern "C" fn s_00af5310() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[49])
    }
    /// `StaticInit_Button_Back_00af5350`: type-enum value `Button_Back`: when 0x00a2e310 finds no such name, the node 0x00c6b994 goes on the head of its list 0x01a85354 (old head to 0x00c6b99c); atexit(0x00b095b0).
    StaticInit_Button_Back_00af5350 @ 0x00af_5350 => extern "C" fn s_00af5350() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[50])
    }
    /// `StaticInit_Button_Circle_00af5390`: type-enum value `Button_Circle`: when 0x00a2e310 finds no such name, the node 0x00c6bac4 goes on the head of its list 0x01a85354 (old head to 0x00c6bacc); atexit(0x00b095d0).
    StaticInit_Button_Circle_00af5390 @ 0x00af_5390 => extern "C" fn s_00af5390() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[51])
    }
    /// `StaticInit_Button_Cross_00af53d0`: type-enum value `Button_Cross`: when 0x00a2e310 finds no such name, the node 0x00c6baf4 goes on the head of its list 0x01a85354 (old head to 0x00c6bafc); atexit(0x00b095f0).
    StaticInit_Button_Cross_00af53d0 @ 0x00af_53d0 => extern "C" fn s_00af53d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[52])
    }
    /// `StaticInit_Button_Down_00af5410`: type-enum value `Button_Down`: when 0x00a2e310 finds no such name, the node 0x00c6b8b4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8bc); atexit(0x00b09610).
    StaticInit_Button_Down_00af5410 @ 0x00af_5410 => extern "C" fn s_00af5410() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[53])
    }
    /// `StaticInit_Button_L1_00af5450`: type-enum value `Button_L1`: when 0x00a2e310 finds no such name, the node 0x00c6ba64 goes on the head of its list 0x01a85354 (old head to 0x00c6ba6c); atexit(0x00b09630).
    StaticInit_Button_L1_00af5450 @ 0x00af_5450 => extern "C" fn s_00af5450() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[54])
    }
    /// `StaticInit_Button_L2_00af5490`: type-enum value `Button_L2`: when 0x00a2e310 finds no such name, the node 0x00c6ba04 goes on the head of its list 0x01a85354 (old head to 0x00c6ba0c); atexit(0x00b09650).
    StaticInit_Button_L2_00af5490 @ 0x00af_5490 => extern "C" fn s_00af5490() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[55])
    }
    /// `StaticInit_Button_L3_00af54d0`: type-enum value `Button_L3`: when 0x00a2e310 finds no such name, the node 0x00c6b954 goes on the head of its list 0x01a85354 (old head to 0x00c6b95c); atexit(0x00b09670).
    StaticInit_Button_L3_00af54d0 @ 0x00af_54d0 => extern "C" fn s_00af54d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[56])
    }
    /// `StaticInit_Button_LB_00af5510`: type-enum value `Button_LB`: when 0x00a2e310 finds no such name, the node 0x00c6ba54 goes on the head of its list 0x01a85354 (old head to 0x00c6ba5c); atexit(0x00b09690).
    StaticInit_Button_LB_00af5510 @ 0x00af_5510 => extern "C" fn s_00af5510() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[57])
    }
    /// `StaticInit_Button_LS_00af5550`: type-enum value `Button_LS`: when 0x00a2e310 finds no such name, the node 0x00c6b944 goes on the head of its list 0x01a85354 (old head to 0x00c6b94c); atexit(0x00b096b0).
    StaticInit_Button_LS_00af5550 @ 0x00af_5550 => extern "C" fn s_00af5550() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[58])
    }
    /// `StaticInit_Button_LT_00af5590`: type-enum value `Button_LT`: when 0x00a2e310 finds no such name, the node 0x00c6b9f4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9fc); atexit(0x00b096d0).
    StaticInit_Button_LT_00af5590 @ 0x00af_5590 => extern "C" fn s_00af5590() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[59])
    }
    /// `StaticInit_Button_Left_00af55d0`: type-enum value `Button_Left`: when 0x00a2e310 finds no such name, the node 0x00c6b894 goes on the head of its list 0x01a85354 (old head to 0x00c6b89c); atexit(0x00b096f0).
    StaticInit_Button_Left_00af55d0 @ 0x00af_55d0 => extern "C" fn s_00af55d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[60])
    }
    /// `StaticInit_Button_R1_00af5610`: type-enum value `Button_R1`: when 0x00a2e310 finds no such name, the node 0x00c6ba34 goes on the head of its list 0x01a85354 (old head to 0x00c6ba3c); atexit(0x00b09710).
    StaticInit_Button_R1_00af5610 @ 0x00af_5610 => extern "C" fn s_00af5610() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[61])
    }
    /// `StaticInit_Button_R2_00af5650`: type-enum value `Button_R2`: when 0x00a2e310 finds no such name, the node 0x00c6b9d4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9dc); atexit(0x00b09730).
    StaticInit_Button_R2_00af5650 @ 0x00af_5650 => extern "C" fn s_00af5650() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[62])
    }
    /// `StaticInit_Button_R3_00af5690`: type-enum value `Button_R3`: when 0x00a2e310 finds no such name, the node 0x00c6b924 goes on the head of its list 0x01a85354 (old head to 0x00c6b92c); atexit(0x00b09750).
    StaticInit_Button_R3_00af5690 @ 0x00af_5690 => extern "C" fn s_00af5690() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[63])
    }
    /// `StaticInit_Button_RB_00af56d0`: type-enum value `Button_RB`: when 0x00a2e310 finds no such name, the node 0x00c6ba24 goes on the head of its list 0x01a85354 (old head to 0x00c6ba2c); atexit(0x00b09770).
    StaticInit_Button_RB_00af56d0 @ 0x00af_56d0 => extern "C" fn s_00af56d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[64])
    }
    /// `StaticInit_Button_RS_00af5710`: type-enum value `Button_RS`: when 0x00a2e310 finds no such name, the node 0x00c6b914 goes on the head of its list 0x01a85354 (old head to 0x00c6b91c); atexit(0x00b09790).
    StaticInit_Button_RS_00af5710 @ 0x00af_5710 => extern "C" fn s_00af5710() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[65])
    }
    /// `StaticInit_Button_RT_00af5750`: type-enum value `Button_RT`: when 0x00a2e310 finds no such name, the node 0x00c6b9c4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9cc); atexit(0x00b097b0).
    StaticInit_Button_RT_00af5750 @ 0x00af_5750 => extern "C" fn s_00af5750() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[66])
    }
    /// `StaticInit_Button_Right_00af5790`: type-enum value `Button_Right`: when 0x00a2e310 finds no such name, the node 0x00c6b8d4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8dc); atexit(0x00b097d0).
    StaticInit_Button_Right_00af5790 @ 0x00af_5790 => extern "C" fn s_00af5790() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[67])
    }
    /// `StaticInit_Button_Select_00af57d0`: type-enum value `Button_Select`: when 0x00a2e310 finds no such name, the node 0x00c6b9a4 goes on the head of its list 0x01a85354 (old head to 0x00c6b9ac); atexit(0x00b097f0).
    StaticInit_Button_Select_00af57d0 @ 0x00af_57d0 => extern "C" fn s_00af57d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[68])
    }
    /// `StaticInit_Button_Square_00af5810`: type-enum value `Button_Square`: when 0x00a2e310 finds no such name, the node 0x00c6bb24 goes on the head of its list 0x01a85354 (old head to 0x00c6bb2c); atexit(0x00b09810).
    StaticInit_Button_Square_00af5810 @ 0x00af_5810 => extern "C" fn s_00af5810() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[69])
    }
    /// `StaticInit_Button_Start_00af5850`: type-enum value `Button_Start`: when 0x00a2e310 finds no such name, the node 0x00c6b974 goes on the head of its list 0x01a85354 (old head to 0x00c6b97c); atexit(0x00b09830).
    StaticInit_Button_Start_00af5850 @ 0x00af_5850 => extern "C" fn s_00af5850() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[70])
    }
    /// `StaticInit_Button_Triangle_00af5890`: type-enum value `Button_Triangle`: when 0x00a2e310 finds no such name, the node 0x00c6ba94 goes on the head of its list 0x01a85354 (old head to 0x00c6ba9c); atexit(0x00b09850).
    StaticInit_Button_Triangle_00af5890 @ 0x00af_5890 => extern "C" fn s_00af5890() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[71])
    }
    /// `StaticInit_Button_Up_00af58d0`: type-enum value `Button_Up`: when 0x00a2e310 finds no such name, the node 0x00c6b8f4 goes on the head of its list 0x01a85354 (old head to 0x00c6b8fc); atexit(0x00b09870).
    StaticInit_Button_Up_00af58d0 @ 0x00af_58d0 => extern "C" fn s_00af58d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[72])
    }
    /// `StaticInit_Button_X_00af5910`: type-enum value `Button_X`: when 0x00a2e310 finds no such name, the node 0x00c6bb14 goes on the head of its list 0x01a85354 (old head to 0x00c6bb1c); atexit(0x00b09890).
    StaticInit_Button_X_00af5910 @ 0x00af_5910 => extern "C" fn s_00af5910() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[73])
    }
    /// `StaticInit_Button_Y_00af5950`: type-enum value `Button_Y`: when 0x00a2e310 finds no such name, the node 0x00c6ba84 goes on the head of its list 0x01a85354 (old head to 0x00c6ba8c); atexit(0x00b098b0).
    StaticInit_Button_Y_00af5950 @ 0x00af_5950 => extern "C" fn s_00af5950() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[74])
    }
    /// `StaticInit_Empty_00af5990`: type-enum value `Empty`: when 0x00a2e310 finds no such name, the node 0x00c6b884 goes on the head of its list 0x01a85354 (old head to 0x00c6b88c); atexit(0x00b098d0).
    StaticInit_Empty_00af5990 @ 0x00af_5990 => extern "C" fn s_00af5990() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[75])
    }
    /// `StaticInit_Empty_00af59d0`: type-enum value `Empty`: when 0x00a2e780 finds no such name, the node 0x00c6bc94 goes on the head of its list 0x01a85390 (old head to 0x00c6bc9c); atexit(0x00b09900).
    StaticInit_Empty_00af59d0 @ 0x00af_59d0 => extern "C" fn s_00af59d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[76])
    }
    /// `StaticInit_Key_0_00af5a10`: type-enum value `Key_0`: when 0x00a2e780 finds no such name, the node 0x00c6c064 goes on the head of its list 0x01a85390 (old head to 0x00c6c06c); atexit(0x00b09920).
    StaticInit_Key_0_00af5a10 @ 0x00af_5a10 => extern "C" fn s_00af5a10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[77])
    }
    /// `StaticInit_Key_1_00af5a50`: type-enum value `Key_1`: when 0x00a2e780 finds no such name, the node 0x00c6c0f4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0fc); atexit(0x00b09940).
    StaticInit_Key_1_00af5a50 @ 0x00af_5a50 => extern "C" fn s_00af5a50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[78])
    }
    /// `StaticInit_Key_2_00af5a90`: type-enum value `Key_2`: when 0x00a2e780 finds no such name, the node 0x00c6c0e4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0ec); atexit(0x00b09960).
    StaticInit_Key_2_00af5a90 @ 0x00af_5a90 => extern "C" fn s_00af5a90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[79])
    }
    /// `StaticInit_Key_3_00af5ad0`: type-enum value `Key_3`: when 0x00a2e780 finds no such name, the node 0x00c6c0d4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0dc); atexit(0x00b09980).
    StaticInit_Key_3_00af5ad0 @ 0x00af_5ad0 => extern "C" fn s_00af5ad0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[80])
    }
    /// `StaticInit_Key_4_00af5b10`: type-enum value `Key_4`: when 0x00a2e780 finds no such name, the node 0x00c6c0c4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0cc); atexit(0x00b099a0).
    StaticInit_Key_4_00af5b10 @ 0x00af_5b10 => extern "C" fn s_00af5b10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[81])
    }
    /// `StaticInit_Key_5_00af5b50`: type-enum value `Key_5`: when 0x00a2e780 finds no such name, the node 0x00c6c0b4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0bc); atexit(0x00b099c0).
    StaticInit_Key_5_00af5b50 @ 0x00af_5b50 => extern "C" fn s_00af5b50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[82])
    }
    /// `StaticInit_Key_6_00af5b90`: type-enum value `Key_6`: when 0x00a2e780 finds no such name, the node 0x00c6c0a4 goes on the head of its list 0x01a85390 (old head to 0x00c6c0ac); atexit(0x00b099e0).
    StaticInit_Key_6_00af5b90 @ 0x00af_5b90 => extern "C" fn s_00af5b90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[83])
    }
    /// `StaticInit_Key_7_00af5bd0`: type-enum value `Key_7`: when 0x00a2e780 finds no such name, the node 0x00c6c094 goes on the head of its list 0x01a85390 (old head to 0x00c6c09c); atexit(0x00b09a00).
    StaticInit_Key_7_00af5bd0 @ 0x00af_5bd0 => extern "C" fn s_00af5bd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[84])
    }
    /// `StaticInit_Key_8_00af5c10`: type-enum value `Key_8`: when 0x00a2e780 finds no such name, the node 0x00c6c084 goes on the head of its list 0x01a85390 (old head to 0x00c6c08c); atexit(0x00b09a20).
    StaticInit_Key_8_00af5c10 @ 0x00af_5c10 => extern "C" fn s_00af5c10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[85])
    }
    /// `StaticInit_Key_9_00af5c50`: type-enum value `Key_9`: when 0x00a2e780 finds no such name, the node 0x00c6c074 goes on the head of its list 0x01a85390 (old head to 0x00c6c07c); atexit(0x00b09a40).
    StaticInit_Key_9_00af5c50 @ 0x00af_5c50 => extern "C" fn s_00af5c50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[86])
    }
    /// `StaticInit_Key_A_00af5c90`: type-enum value `Key_A`: when 0x00a2e780 finds no such name, the node 0x00c6c294 goes on the head of its list 0x01a85390 (old head to 0x00c6c29c); atexit(0x00b09a60).
    StaticInit_Key_A_00af5c90 @ 0x00af_5c90 => extern "C" fn s_00af5c90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[87])
    }
    /// `StaticInit_Key_AccentGrave_00af5cd0`: type-enum value `Key_AccentGrave`: when 0x00a2e780 finds no such name, the node 0x00c6bca4 goes on the head of its list 0x01a85390 (old head to 0x00c6bcac); atexit(0x00b09a80).
    StaticInit_Key_AccentGrave_00af5cd0 @ 0x00af_5cd0 => extern "C" fn s_00af5cd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[88])
    }
    /// `StaticInit_Key_Apostrophe_00af5d10`: type-enum value `Key_Apostrophe`: when 0x00a2e780 finds no such name, the node 0x00c6bee4 goes on the head of its list 0x01a85390 (old head to 0x00c6beec); atexit(0x00b09aa0).
    StaticInit_Key_Apostrophe_00af5d10 @ 0x00af_5d10 => extern "C" fn s_00af5d10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[89])
    }
    /// `StaticInit_Key_B_00af5d50`: type-enum value `Key_B`: when 0x00a2e780 finds no such name, the node 0x00c6c284 goes on the head of its list 0x01a85390 (old head to 0x00c6c28c); atexit(0x00b09ac0).
    StaticInit_Key_B_00af5d50 @ 0x00af_5d50 => extern "C" fn s_00af5d50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[90])
    }
    /// `StaticInit_Key_Backslash_00af5d90`: type-enum value `Key_Backslash`: when 0x00a2e780 finds no such name, the node 0x00c6bf04 goes on the head of its list 0x01a85390 (old head to 0x00c6bf0c); atexit(0x00b09ae0).
    StaticInit_Key_Backslash_00af5d90 @ 0x00af_5d90 => extern "C" fn s_00af5d90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[91])
    }
    /// `StaticInit_Key_Backspace_00af5dd0`: type-enum value `Key_Backspace`: when 0x00a2e780 finds no such name, the node 0x00c6bf74 goes on the head of its list 0x01a85390 (old head to 0x00c6bf7c); atexit(0x00b09b00).
    StaticInit_Key_Backspace_00af5dd0 @ 0x00af_5dd0 => extern "C" fn s_00af5dd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[92])
    }
    /// `StaticInit_Key_C_00af5e10`: type-enum value `Key_C`: when 0x00a2e780 finds no such name, the node 0x00c6c274 goes on the head of its list 0x01a85390 (old head to 0x00c6c27c); atexit(0x00b09b20).
    StaticInit_Key_C_00af5e10 @ 0x00af_5e10 => extern "C" fn s_00af5e10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[93])
    }
    /// `StaticInit_Key_Comma_00af5e50`: type-enum value `Key_Comma`: when 0x00a2e780 finds no such name, the node 0x00c6bed4 goes on the head of its list 0x01a85390 (old head to 0x00c6bedc); atexit(0x00b09b40).
    StaticInit_Key_Comma_00af5e50 @ 0x00af_5e50 => extern "C" fn s_00af5e50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[94])
    }
    /// `StaticInit_Key_D_00af5e90`: type-enum value `Key_D`: when 0x00a2e780 finds no such name, the node 0x00c6c264 goes on the head of its list 0x01a85390 (old head to 0x00c6c26c); atexit(0x00b09b60).
    StaticInit_Key_D_00af5e90 @ 0x00af_5e90 => extern "C" fn s_00af5e90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[95])
    }
    /// `StaticInit_Key_Delete_00af5ed0`: type-enum value `Key_Delete`: when 0x00a2e780 finds no such name, the node 0x00c6be14 goes on the head of its list 0x01a85390 (old head to 0x00c6be1c); atexit(0x00b09b80).
    StaticInit_Key_Delete_00af5ed0 @ 0x00af_5ed0 => extern "C" fn s_00af5ed0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[96])
    }
    /// `StaticInit_Key_Down_00af5f10`: type-enum value `Key_Down`: when 0x00a2e780 finds no such name, the node 0x00c6be74 goes on the head of its list 0x01a85390 (old head to 0x00c6be7c); atexit(0x00b09ba0).
    StaticInit_Key_Down_00af5f10 @ 0x00af_5f10 => extern "C" fn s_00af5f10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[97])
    }
    /// `StaticInit_Key_E_00af5f50`: type-enum value `Key_E`: when 0x00a2e780 finds no such name, the node 0x00c6c254 goes on the head of its list 0x01a85390 (old head to 0x00c6c25c); atexit(0x00b09bc0).
    StaticInit_Key_E_00af5f50 @ 0x00af_5f50 => extern "C" fn s_00af5f50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[98])
    }
    /// `StaticInit_Key_End_00af5f90`: type-enum value `Key_End`: when 0x00a2e780 finds no such name, the node 0x00c6be54 goes on the head of its list 0x01a85390 (old head to 0x00c6be5c); atexit(0x00b09be0).
    StaticInit_Key_End_00af5f90 @ 0x00af_5f90 => extern "C" fn s_00af5f90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[99])
    }
    /// `StaticInit_Key_Enter_00af5fd0`: type-enum value `Key_Enter`: when 0x00a2e780 finds no such name, the node 0x00c6bf84 goes on the head of its list 0x01a85390 (old head to 0x00c6bf8c); atexit(0x00b09c00).
    StaticInit_Key_Enter_00af5fd0 @ 0x00af_5fd0 => extern "C" fn s_00af5fd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[100])
    }
    /// `StaticInit_Key_Equals_00af6010`: type-enum value `Key_Equals`: when 0x00a2e780 finds no such name, the node 0x00c6bf34 goes on the head of its list 0x01a85390 (old head to 0x00c6bf3c); atexit(0x00b09c20).
    StaticInit_Key_Equals_00af6010 @ 0x00af_6010 => extern "C" fn s_00af6010() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[101])
    }
    /// `StaticInit_Key_Escape_00af6050`: type-enum value `Key_Escape`: when 0x00a2e780 finds no such name, the node 0x00c6c054 goes on the head of its list 0x01a85390 (old head to 0x00c6c05c); atexit(0x00b09c40).
    StaticInit_Key_Escape_00af6050 @ 0x00af_6050 => extern "C" fn s_00af6050() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[102])
    }
    /// `StaticInit_Key_F10_00af6090`: type-enum value `Key_F10`: when 0x00a2e780 finds no such name, the node 0x00c6bfb4 goes on the head of its list 0x01a85390 (old head to 0x00c6bfbc); atexit(0x00b09c60).
    StaticInit_Key_F10_00af6090 @ 0x00af_6090 => extern "C" fn s_00af6090() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[103])
    }
    /// `StaticInit_Key_F11_00af60d0`: type-enum value `Key_F11`: when 0x00a2e780 finds no such name, the node 0x00c6bfa4 goes on the head of its list 0x01a85390 (old head to 0x00c6bfac); atexit(0x00b09c80).
    StaticInit_Key_F11_00af60d0 @ 0x00af_60d0 => extern "C" fn s_00af60d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[104])
    }
    /// `StaticInit_Key_F12_00af6110`: type-enum value `Key_F12`: when 0x00a2e780 finds no such name, the node 0x00c6bf94 goes on the head of its list 0x01a85390 (old head to 0x00c6bf9c); atexit(0x00b09ca0).
    StaticInit_Key_F12_00af6110 @ 0x00af_6110 => extern "C" fn s_00af6110() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[105])
    }
    /// `StaticInit_Key_F1_00af6150`: type-enum value `Key_F1`: when 0x00a2e780 finds no such name, the node 0x00c6c044 goes on the head of its list 0x01a85390 (old head to 0x00c6c04c); atexit(0x00b09cc0).
    StaticInit_Key_F1_00af6150 @ 0x00af_6150 => extern "C" fn s_00af6150() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[106])
    }
    /// `StaticInit_Key_F2_00af6190`: type-enum value `Key_F2`: when 0x00a2e780 finds no such name, the node 0x00c6c034 goes on the head of its list 0x01a85390 (old head to 0x00c6c03c); atexit(0x00b09ce0).
    StaticInit_Key_F2_00af6190 @ 0x00af_6190 => extern "C" fn s_00af6190() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[107])
    }
    /// `StaticInit_Key_F3_00af61d0`: type-enum value `Key_F3`: when 0x00a2e780 finds no such name, the node 0x00c6c024 goes on the head of its list 0x01a85390 (old head to 0x00c6c02c); atexit(0x00b09d00).
    StaticInit_Key_F3_00af61d0 @ 0x00af_61d0 => extern "C" fn s_00af61d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[108])
    }
    /// `StaticInit_Key_F4_00af6210`: type-enum value `Key_F4`: when 0x00a2e780 finds no such name, the node 0x00c6c014 goes on the head of its list 0x01a85390 (old head to 0x00c6c01c); atexit(0x00b09d20).
    StaticInit_Key_F4_00af6210 @ 0x00af_6210 => extern "C" fn s_00af6210() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[109])
    }
    /// `StaticInit_Key_F5_00af6250`: type-enum value `Key_F5`: when 0x00a2e780 finds no such name, the node 0x00c6c004 goes on the head of its list 0x01a85390 (old head to 0x00c6c00c); atexit(0x00b09d40).
    StaticInit_Key_F5_00af6250 @ 0x00af_6250 => extern "C" fn s_00af6250() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[110])
    }
    /// `StaticInit_Key_F6_00af6290`: type-enum value `Key_F6`: when 0x00a2e780 finds no such name, the node 0x00c6bff4 goes on the head of its list 0x01a85390 (old head to 0x00c6bffc); atexit(0x00b09d60).
    StaticInit_Key_F6_00af6290 @ 0x00af_6290 => extern "C" fn s_00af6290() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[111])
    }
    /// `StaticInit_Key_F7_00af62d0`: type-enum value `Key_F7`: when 0x00a2e780 finds no such name, the node 0x00c6bfe4 goes on the head of its list 0x01a85390 (old head to 0x00c6bfec); atexit(0x00b09d80).
    StaticInit_Key_F7_00af62d0 @ 0x00af_62d0 => extern "C" fn s_00af62d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[112])
    }
    /// `StaticInit_Key_F8_00af6310`: type-enum value `Key_F8`: when 0x00a2e780 finds no such name, the node 0x00c6bfd4 goes on the head of its list 0x01a85390 (old head to 0x00c6bfdc); atexit(0x00b09da0).
    StaticInit_Key_F8_00af6310 @ 0x00af_6310 => extern "C" fn s_00af6310() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[113])
    }
    /// `StaticInit_Key_F9_00af6350`: type-enum value `Key_F9`: when 0x00a2e780 finds no such name, the node 0x00c6bfc4 goes on the head of its list 0x01a85390 (old head to 0x00c6bfcc); atexit(0x00b09dc0).
    StaticInit_Key_F9_00af6350 @ 0x00af_6350 => extern "C" fn s_00af6350() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[114])
    }
    /// `StaticInit_Key_F_00af6390`: type-enum value `Key_F`: when 0x00a2e780 finds no such name, the node 0x00c6c244 goes on the head of its list 0x01a85390 (old head to 0x00c6c24c); atexit(0x00b09de0).
    StaticInit_Key_F_00af6390 @ 0x00af_6390 => extern "C" fn s_00af6390() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[115])
    }
    /// `StaticInit_Key_G_00af63d0`: type-enum value `Key_G`: when 0x00a2e780 finds no such name, the node 0x00c6c234 goes on the head of its list 0x01a85390 (old head to 0x00c6c23c); atexit(0x00b09e00).
    StaticInit_Key_G_00af63d0 @ 0x00af_63d0 => extern "C" fn s_00af63d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[116])
    }
    /// `StaticInit_Key_H_00af6410`: type-enum value `Key_H`: when 0x00a2e780 finds no such name, the node 0x00c6c224 goes on the head of its list 0x01a85390 (old head to 0x00c6c22c); atexit(0x00b09e20).
    StaticInit_Key_H_00af6410 @ 0x00af_6410 => extern "C" fn s_00af6410() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[117])
    }
    /// `StaticInit_Key_Home_00af6450`: type-enum value `Key_Home`: when 0x00a2e780 finds no such name, the node 0x00c6be64 goes on the head of its list 0x01a85390 (old head to 0x00c6be6c); atexit(0x00b09e40).
    StaticInit_Key_Home_00af6450 @ 0x00af_6450 => extern "C" fn s_00af6450() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[118])
    }
    /// `StaticInit_Key_I_00af6490`: type-enum value `Key_I`: when 0x00a2e780 finds no such name, the node 0x00c6c214 goes on the head of its list 0x01a85390 (old head to 0x00c6c21c); atexit(0x00b09e60).
    StaticInit_Key_I_00af6490 @ 0x00af_6490 => extern "C" fn s_00af6490() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[119])
    }
    /// `StaticInit_Key_Insert_00af64d0`: type-enum value `Key_Insert`: when 0x00a2e780 finds no such name, the node 0x00c6be24 goes on the head of its list 0x01a85390 (old head to 0x00c6be2c); atexit(0x00b09e80).
    StaticInit_Key_Insert_00af64d0 @ 0x00af_64d0 => extern "C" fn s_00af64d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[120])
    }
    /// `StaticInit_Key_J_00af6510`: type-enum value `Key_J`: when 0x00a2e780 finds no such name, the node 0x00c6c204 goes on the head of its list 0x01a85390 (old head to 0x00c6c20c); atexit(0x00b09ea0).
    StaticInit_Key_J_00af6510 @ 0x00af_6510 => extern "C" fn s_00af6510() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[121])
    }
    /// `StaticInit_Key_K_00af6550`: type-enum value `Key_K`: when 0x00a2e780 finds no such name, the node 0x00c6c1f4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1fc); atexit(0x00b09ec0).
    StaticInit_Key_K_00af6550 @ 0x00af_6550 => extern "C" fn s_00af6550() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[122])
    }
    /// `StaticInit_Key_L_00af6590`: type-enum value `Key_L`: when 0x00a2e780 finds no such name, the node 0x00c6c1e4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1ec); atexit(0x00b09ee0).
    StaticInit_Key_L_00af6590 @ 0x00af_6590 => extern "C" fn s_00af6590() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[123])
    }
    /// `StaticInit_Key_LeftBracket_00af65d0`: type-enum value `Key_LeftBracket`: when 0x00a2e780 finds no such name, the node 0x00c6bf24 goes on the head of its list 0x01a85390 (old head to 0x00c6bf2c); atexit(0x00b09f00).
    StaticInit_Key_LeftBracket_00af65d0 @ 0x00af_65d0 => extern "C" fn s_00af65d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[124])
    }
    /// `StaticInit_Key_Left_Alt_00af6610`: type-enum value `Key_Left_Alt`: when 0x00a2e780 finds no such name, the node 0x00c6bce4 goes on the head of its list 0x01a85390 (old head to 0x00c6bcec); atexit(0x00b09f20).
    StaticInit_Key_Left_Alt_00af6610 @ 0x00af_6610 => extern "C" fn s_00af6610() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[125])
    }
    /// `StaticInit_Key_Left_Control_00af6650`: type-enum value `Key_Left_Control`: when 0x00a2e780 finds no such name, the node 0x00c6bd04 goes on the head of its list 0x01a85390 (old head to 0x00c6bd0c); atexit(0x00b09f40).
    StaticInit_Key_Left_Control_00af6650 @ 0x00af_6650 => extern "C" fn s_00af6650() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[126])
    }
    /// `StaticInit_Key_Left_Shift_00af6690`: type-enum value `Key_Left_Shift`: when 0x00a2e780 finds no such name, the node 0x00c6bcf4 goes on the head of its list 0x01a85390 (old head to 0x00c6bcfc); atexit(0x00b09f60).
    StaticInit_Key_Left_Shift_00af6690 @ 0x00af_6690 => extern "C" fn s_00af6690() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[127])
    }
    /// `StaticInit_Key_Left_00af66d0`: type-enum value `Key_Left`: when 0x00a2e780 finds no such name, the node 0x00c6bea4 goes on the head of its list 0x01a85390 (old head to 0x00c6beac); atexit(0x00b09f80).
    StaticInit_Key_Left_00af66d0 @ 0x00af_66d0 => extern "C" fn s_00af66d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[128])
    }
    /// `StaticInit_Key_M_00af6710`: type-enum value `Key_M`: when 0x00a2e780 finds no such name, the node 0x00c6c1d4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1dc); atexit(0x00b09fa0).
    StaticInit_Key_M_00af6710 @ 0x00af_6710 => extern "C" fn s_00af6710() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[129])
    }
    /// `StaticInit_Key_Minus_00af6750`: type-enum value `Key_Minus`: when 0x00a2e780 finds no such name, the node 0x00c6bf44 goes on the head of its list 0x01a85390 (old head to 0x00c6bf4c); atexit(0x00b09fc0).
    StaticInit_Key_Minus_00af6750 @ 0x00af_6750 => extern "C" fn s_00af6750() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[130])
    }
    /// `StaticInit_Key_N_00af6790`: type-enum value `Key_N`: when 0x00a2e780 finds no such name, the node 0x00c6c1c4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1cc); atexit(0x00b09fe0).
    StaticInit_Key_N_00af6790 @ 0x00af_6790 => extern "C" fn s_00af6790() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[131])
    }
    /// `StaticInit_Key_Numpad_0_00af67d0`: type-enum value `Key_Numpad_0`: when 0x00a2e780 finds no such name, the node 0x00c6bd74 goes on the head of its list 0x01a85390 (old head to 0x00c6bd7c); atexit(0x00b0a000).
    StaticInit_Key_Numpad_0_00af67d0 @ 0x00af_67d0 => extern "C" fn s_00af67d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[132])
    }
    /// `StaticInit_Key_Numpad_1_00af6810`: type-enum value `Key_Numpad_1`: when 0x00a2e780 finds no such name, the node 0x00c6be04 goes on the head of its list 0x01a85390 (old head to 0x00c6be0c); atexit(0x00b0a020).
    StaticInit_Key_Numpad_1_00af6810 @ 0x00af_6810 => extern "C" fn s_00af6810() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[133])
    }
    /// `StaticInit_Key_Numpad_2_00af6850`: type-enum value `Key_Numpad_2`: when 0x00a2e780 finds no such name, the node 0x00c6bdf4 goes on the head of its list 0x01a85390 (old head to 0x00c6bdfc); atexit(0x00b0a040).
    StaticInit_Key_Numpad_2_00af6850 @ 0x00af_6850 => extern "C" fn s_00af6850() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[134])
    }
    /// `StaticInit_Key_Numpad_3_00af6890`: type-enum value `Key_Numpad_3`: when 0x00a2e780 finds no such name, the node 0x00c6bde4 goes on the head of its list 0x01a85390 (old head to 0x00c6bdec); atexit(0x00b0a060).
    StaticInit_Key_Numpad_3_00af6890 @ 0x00af_6890 => extern "C" fn s_00af6890() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[135])
    }
    /// `StaticInit_Key_Numpad_4_00af68d0`: type-enum value `Key_Numpad_4`: when 0x00a2e780 finds no such name, the node 0x00c6bdd4 goes on the head of its list 0x01a85390 (old head to 0x00c6bddc); atexit(0x00b0a080).
    StaticInit_Key_Numpad_4_00af68d0 @ 0x00af_68d0 => extern "C" fn s_00af68d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[136])
    }
    /// `StaticInit_Key_Numpad_5_00af6910`: type-enum value `Key_Numpad_5`: when 0x00a2e780 finds no such name, the node 0x00c6bdc4 goes on the head of its list 0x01a85390 (old head to 0x00c6bdcc); atexit(0x00b0a0a0).
    StaticInit_Key_Numpad_5_00af6910 @ 0x00af_6910 => extern "C" fn s_00af6910() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[137])
    }
    /// `StaticInit_Key_Numpad_6_00af6950`: type-enum value `Key_Numpad_6`: when 0x00a2e780 finds no such name, the node 0x00c6bdb4 goes on the head of its list 0x01a85390 (old head to 0x00c6bdbc); atexit(0x00b0a0c0).
    StaticInit_Key_Numpad_6_00af6950 @ 0x00af_6950 => extern "C" fn s_00af6950() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[138])
    }
    /// `StaticInit_Key_Numpad_7_00af6990`: type-enum value `Key_Numpad_7`: when 0x00a2e780 finds no such name, the node 0x00c6bda4 goes on the head of its list 0x01a85390 (old head to 0x00c6bdac); atexit(0x00b0a0e0).
    StaticInit_Key_Numpad_7_00af6990 @ 0x00af_6990 => extern "C" fn s_00af6990() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[139])
    }
    /// `StaticInit_Key_Numpad_8_00af69d0`: type-enum value `Key_Numpad_8`: when 0x00a2e780 finds no such name, the node 0x00c6bd94 goes on the head of its list 0x01a85390 (old head to 0x00c6bd9c); atexit(0x00b0a100).
    StaticInit_Key_Numpad_8_00af69d0 @ 0x00af_69d0 => extern "C" fn s_00af69d0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[140])
    }
    /// `StaticInit_Key_Numpad_9_00af6a10`: type-enum value `Key_Numpad_9`: when 0x00a2e780 finds no such name, the node 0x00c6bd84 goes on the head of its list 0x01a85390 (old head to 0x00c6bd8c); atexit(0x00b0a120).
    StaticInit_Key_Numpad_9_00af6a10 @ 0x00af_6a10 => extern "C" fn s_00af6a10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[141])
    }
    /// `StaticInit_Key_Numpad_Asterisk_00af6a50`: type-enum value `Key_Numpad_Asterisk`: when 0x00a2e780 finds no such name, the node 0x00c6bd54 goes on the head of its list 0x01a85390 (old head to 0x00c6bd5c); atexit(0x00b0a140).
    StaticInit_Key_Numpad_Asterisk_00af6a50 @ 0x00af_6a50 => extern "C" fn s_00af6a50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[142])
    }
    /// `StaticInit_Key_Numpad_Enter_00af6a90`: type-enum value `Key_Numpad_Enter`: when 0x00a2e780 finds no such name, the node 0x00c6bd24 goes on the head of its list 0x01a85390 (old head to 0x00c6bd2c); atexit(0x00b0a160).
    StaticInit_Key_Numpad_Enter_00af6a90 @ 0x00af_6a90 => extern "C" fn s_00af6a90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[143])
    }
    /// `StaticInit_Key_Numpad_Minus_00af6ad0`: type-enum value `Key_Numpad_Minus`: when 0x00a2e780 finds no such name, the node 0x00c6bd44 goes on the head of its list 0x01a85390 (old head to 0x00c6bd4c); atexit(0x00b0a180).
    StaticInit_Key_Numpad_Minus_00af6ad0 @ 0x00af_6ad0 => extern "C" fn s_00af6ad0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[144])
    }
    /// `StaticInit_Key_Numpad_Period_00af6b10`: type-enum value `Key_Numpad_Period`: when 0x00a2e780 finds no such name, the node 0x00c6bd14 goes on the head of its list 0x01a85390 (old head to 0x00c6bd1c); atexit(0x00b0a1a0).
    StaticInit_Key_Numpad_Period_00af6b10 @ 0x00af_6b10 => extern "C" fn s_00af6b10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[145])
    }
    /// `StaticInit_Key_Numpad_Plus_00af6b50`: type-enum value `Key_Numpad_Plus`: when 0x00a2e780 finds no such name, the node 0x00c6bd34 goes on the head of its list 0x01a85390 (old head to 0x00c6bd3c); atexit(0x00b0a1c0).
    StaticInit_Key_Numpad_Plus_00af6b50 @ 0x00af_6b50 => extern "C" fn s_00af6b50() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[146])
    }
    /// `StaticInit_Key_Numpad_Slash_00af6b90`: type-enum value `Key_Numpad_Slash`: when 0x00a2e780 finds no such name, the node 0x00c6bd64 goes on the head of its list 0x01a85390 (old head to 0x00c6bd6c); atexit(0x00b0a1e0).
    StaticInit_Key_Numpad_Slash_00af6b90 @ 0x00af_6b90 => extern "C" fn s_00af6b90() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[147])
    }
    /// `StaticInit_Key_O_00af6bd0`: type-enum value `Key_O`: when 0x00a2e780 finds no such name, the node 0x00c6c1b4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1bc); atexit(0x00b0a200).
    StaticInit_Key_O_00af6bd0 @ 0x00af_6bd0 => extern "C" fn s_00af6bd0() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[148])
    }
    /// `StaticInit_Key_P_00af6c10`: type-enum value `Key_P`: when 0x00a2e780 finds no such name, the node 0x00c6c1a4 goes on the head of its list 0x01a85390 (old head to 0x00c6c1ac); atexit(0x00b0a220).
    StaticInit_Key_P_00af6c10 @ 0x00af_6c10 => extern "C" fn s_00af6c10() -> u32 {
        si::type_enum(&mut mem(), &si::TYPE_ENUMS[149])
    }
    /// `StaticInit_PInputSource_00af14e0`: class descriptor `PInputSource` (`cd_desc_keep`): constructor, buffers, keep; atexit.
    StaticInit_PInputSource_00af14e0 @ 0x00af_14e0 => extern "C" fn s_00af14e0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[52])
    }
    /// `StaticInit_PInputSourceJoypadAxis_00af1590`: class descriptor `PInputSourceJoypadAxis` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceJoypadAxis_00af1590 @ 0x00af_1590 => extern "C" fn s_00af1590() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[53])
    }
    /// `StaticInit_PInputSourceJoypadButton_00af1650`: class descriptor `PInputSourceJoypadButton` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceJoypadButton_00af1650 @ 0x00af_1650 => extern "C" fn s_00af1650() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[54])
    }
    /// `StaticInit_PInputSourceKey_00af1710`: class descriptor `PInputSourceKey` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceKey_00af1710 @ 0x00af_1710 => extern "C" fn s_00af1710() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[55])
    }
    /// `StaticInit_PInputSourceMotionAngularVelocityX_00af17d0`: class descriptor `PInputSourceMotionAngularVelocityX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionAngularVelocityX_00af17d0 @ 0x00af_17d0 => extern "C" fn s_00af17d0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[56])
    }
    /// `StaticInit_PInputSourceMotionAngularVelocityY_00af1890`: class descriptor `PInputSourceMotionAngularVelocityY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionAngularVelocityY_00af1890 @ 0x00af_1890 => extern "C" fn s_00af1890() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[57])
    }
    /// `StaticInit_PInputSourceMotionAngularVelocityZ_00af1950`: class descriptor `PInputSourceMotionAngularVelocityZ` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionAngularVelocityZ_00af1950 @ 0x00af_1950 => extern "C" fn s_00af1950() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[58])
    }
    /// `StaticInit_PInputSourceMotionLinearAccelerationX_00af1a10`: class descriptor `PInputSourceMotionLinearAccelerationX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionLinearAccelerationX_00af1a10 @ 0x00af_1a10 => extern "C" fn s_00af1a10() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[59])
    }
    /// `StaticInit_PInputSourceMotionLinearAccelerationY_00af1ad0`: class descriptor `PInputSourceMotionLinearAccelerationY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionLinearAccelerationY_00af1ad0 @ 0x00af_1ad0 => extern "C" fn s_00af1ad0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[60])
    }
    /// `StaticInit_PInputSourceMotionLinearAccelerationZ_00af1b90`: class descriptor `PInputSourceMotionLinearAccelerationZ` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionLinearAccelerationZ_00af1b90 @ 0x00af_1b90 => extern "C" fn s_00af1b90() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[61])
    }
    /// `StaticInit_PInputSourceMotionQuatW_00af1c50`: class descriptor `PInputSourceMotionQuatW` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionQuatW_00af1c50 @ 0x00af_1c50 => extern "C" fn s_00af1c50() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[62])
    }
    /// `StaticInit_PInputSourceMotionQuatX_00af1d10`: class descriptor `PInputSourceMotionQuatX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionQuatX_00af1d10 @ 0x00af_1d10 => extern "C" fn s_00af1d10() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[63])
    }
    /// `StaticInit_PInputSourceMotionQuatY_00af1dd0`: class descriptor `PInputSourceMotionQuatY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionQuatY_00af1dd0 @ 0x00af_1dd0 => extern "C" fn s_00af1dd0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[64])
    }
    /// `StaticInit_PInputSourceMotionQuatZ_00af1e90`: class descriptor `PInputSourceMotionQuatZ` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMotionQuatZ_00af1e90 @ 0x00af_1e90 => extern "C" fn s_00af1e90() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[65])
    }
    /// `StaticInit_PInputSourceMouseButton_00af1f50`: class descriptor `PInputSourceMouseButton` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMouseButton_00af1f50 @ 0x00af_1f50 => extern "C" fn s_00af1f50() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[66])
    }
    /// `StaticInit_PInputSourceMouseDeltaX_00af2010`: class descriptor `PInputSourceMouseDeltaX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMouseDeltaX_00af2010 @ 0x00af_2010 => extern "C" fn s_00af2010() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[67])
    }
    /// `StaticInit_PInputSourceMouseDeltaY_00af20d0`: class descriptor `PInputSourceMouseDeltaY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceMouseDeltaY_00af20d0 @ 0x00af_20d0 => extern "C" fn s_00af20d0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[68])
    }
    /// `StaticInit_PInputSourceTouchDragX_00af2190`: class descriptor `PInputSourceTouchDragX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchDragX_00af2190 @ 0x00af_2190 => extern "C" fn s_00af2190() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[69])
    }
    /// `StaticInit_PInputSourceTouchDragY_00af2250`: class descriptor `PInputSourceTouchDragY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchDragY_00af2250 @ 0x00af_2250 => extern "C" fn s_00af2250() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[70])
    }
    /// `StaticInit_PInputSourceTouchPinch_00af2310`: class descriptor `PInputSourceTouchPinch` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchPinch_00af2310 @ 0x00af_2310 => extern "C" fn s_00af2310() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[71])
    }
    /// `StaticInit_PInputSourceTouchRotate_00af23d0`: class descriptor `PInputSourceTouchRotate` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchRotate_00af23d0 @ 0x00af_23d0 => extern "C" fn s_00af23d0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[72])
    }
    /// `StaticInit_PInputSourceTouchTwoFingerDragX_00af2490`: class descriptor `PInputSourceTouchTwoFingerDragX` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchTwoFingerDragX_00af2490 @ 0x00af_2490 => extern "C" fn s_00af2490() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[73])
    }
    /// `StaticInit_PInputSourceTouchTwoFingerDragY_00af2550`: class descriptor `PInputSourceTouchTwoFingerDragY` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PInputSourceTouchTwoFingerDragY_00af2550 @ 0x00af_2550 => extern "C" fn s_00af2550() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[74])
    }
    /// `StaticInit_TailThiscall_00af2610`: ecx = `0x0194_2998`, then a tail jump to 0x009f2510 (thiscall, no stack words), whose eax is returned.
    StaticInit_TailThiscall_00af2610 @ 0x00af_2610 => extern "C" fn s_00af2610() -> u32 {
        si::tail_thiscall(&mut mem(), &si::TAIL_THISCALLS[0])
    }
    /// `StaticInit_ChainAppend_00af2620`: appends the object at `0x00c6_81ec` (vtable 0x00b73d8c) to the chain at 0xc901a8 (before the first block at least 0xff full); atexit(0x00b08330).
    StaticInit_ChainAppend_00af2620 @ 0x00af_2620 => extern "C" fn s_00af2620() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[2])
    }
    /// `StaticInit_ChainAppend_00af2bf0`: appends the object at `0x00c6_845c` (vtable 0x00b73f58) to the chain at 0xc901a8 (before the first block at least 0xff full); atexit(0x00b08650).
    StaticInit_ChainAppend_00af2bf0 @ 0x00af_2bf0 => extern "C" fn s_00af2bf0() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[3])
    }
    /// `StaticInit_Sentinel_00af2c40`: list object `0x0194_3da0` constructed, then its sentinel `0x0194_3de8` links to itself; atexit(0x00b086a0).
    StaticInit_Sentinel_00af2c40 @ 0x00af_2c40 => extern "C" fn s_00af2c40() -> u32 {
        si::sentinel(&mut mem(), &si::SENTINELS[2])
    }
    /// `StaticInit_PDeferredLighting_00af2c70`: class descriptor `PDeferredLighting` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDeferredLighting_00af2c70 @ 0x00af_2c70 => extern "C" fn s_00af2c70() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[75])
    }
    /// `StaticInit_PDeferredLightingBase_00af2d30`: class descriptor `PDeferredLightingBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDeferredLightingBase_00af2d30 @ 0x00af_2d30 => extern "C" fn s_00af2d30() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[76])
    }
    /// `StaticInit_PDeferredLightingD3D11_00af2df0`: class descriptor `PDeferredLightingD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDeferredLightingD3D11_00af2df0 @ 0x00af_2df0 => extern "C" fn s_00af2df0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[77])
    }
    /// `StaticInit_PDepthOfField_00af2eb0`: class descriptor `PDepthOfField` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDepthOfField_00af2eb0 @ 0x00af_2eb0 => extern "C" fn s_00af2eb0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[78])
    }
    /// `StaticInit_PDepthOfFieldBase_00af2f70`: class descriptor `PDepthOfFieldBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDepthOfFieldBase_00af2f70 @ 0x00af_2f70 => extern "C" fn s_00af2f70() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[79])
    }
    /// `StaticInit_PDepthOfFieldD3D11_00af3030`: class descriptor `PDepthOfFieldD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PDepthOfFieldD3D11_00af3030 @ 0x00af_3030 => extern "C" fn s_00af3030() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[80])
    }
    /// `StaticInit_PFXAA_00af30f0`: class descriptor `PFXAA` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PFXAA_00af30f0 @ 0x00af_30f0 => extern "C" fn s_00af30f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[81])
    }
    /// `StaticInit_PFXAABase_00af31b0`: class descriptor `PFXAABase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PFXAABase_00af31b0 @ 0x00af_31b0 => extern "C" fn s_00af31b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[82])
    }
    /// `StaticInit_PFXAAD3D11_00af3270`: class descriptor `PFXAAD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PFXAAD3D11_00af3270 @ 0x00af_3270 => extern "C" fn s_00af3270() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[83])
    }
    /// `StaticInit_PGlow_00af3330`: class descriptor `PGlow` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PGlow_00af3330 @ 0x00af_3330 => extern "C" fn s_00af3330() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[84])
    }
    /// `StaticInit_PGlowBase_00af33f0`: class descriptor `PGlowBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PGlowBase_00af33f0 @ 0x00af_33f0 => extern "C" fn s_00af33f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[85])
    }
    /// `StaticInit_PGlowD3D11_00af34b0`: class descriptor `PGlowD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PGlowD3D11_00af34b0 @ 0x00af_34b0 => extern "C" fn s_00af34b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[86])
    }
    /// `StaticInit_PGlowGPUBase_00af3570`: class descriptor `PGlowGPUBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PGlowGPUBase_00af3570 @ 0x00af_3570 => extern "C" fn s_00af3570() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[87])
    }
    /// `StaticInit_PLegacyGlow_00af3630`: class descriptor `PLegacyGlow` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PLegacyGlow_00af3630 @ 0x00af_3630 => extern "C" fn s_00af3630() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[88])
    }
    /// `StaticInit_PLegacyGlowBase_00af36f0`: class descriptor `PLegacyGlowBase` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PLegacyGlowBase_00af36f0 @ 0x00af_36f0 => extern "C" fn s_00af36f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[89])
    }
    /// `StaticInit_PLegacyGlowD3D11_00af37b0`: class descriptor `PLegacyGlowD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PLegacyGlowD3D11_00af37b0 @ 0x00af_37b0 => extern "C" fn s_00af37b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[90])
    }
    /// `StaticInit_PLegacyGlowGPUBase_00af3870`: class descriptor `PLegacyGlowGPUBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PLegacyGlowGPUBase_00af3870 @ 0x00af_3870 => extern "C" fn s_00af3870() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[91])
    }
    /// `StaticInit_PMLAA_00af3930`: class descriptor `PMLAA` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMLAA_00af3930 @ 0x00af_3930 => extern "C" fn s_00af3930() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[92])
    }
    /// `StaticInit_PMLAABase_00af39f0`: class descriptor `PMLAABase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMLAABase_00af39f0 @ 0x00af_39f0 => extern "C" fn s_00af39f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[93])
    }
    /// `StaticInit_PMLAAD3D11_00af3ab0`: class descriptor `PMLAAD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMLAAD3D11_00af3ab0 @ 0x00af_3ab0 => extern "C" fn s_00af3ab0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[94])
    }
    /// `StaticInit_PMeshParticleSystemBase_00af3b70`: class descriptor `PMeshParticleSystemBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMeshParticleSystemBase_00af3b70 @ 0x00af_3b70 => extern "C" fn s_00af3b70() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[95])
    }
    /// `StaticInit_PMotionBlur_00af3c30`: class descriptor `PMotionBlur` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMotionBlur_00af3c30 @ 0x00af_3c30 => extern "C" fn s_00af3c30() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[96])
    }
    /// `StaticInit_PMotionBlurBase_00af3cf0`: class descriptor `PMotionBlurBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMotionBlurBase_00af3cf0 @ 0x00af_3cf0 => extern "C" fn s_00af3cf0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[97])
    }
    /// `StaticInit_PMotionBlurD3D11_00af3db0`: class descriptor `PMotionBlurD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PMotionBlurD3D11_00af3db0 @ 0x00af_3db0 => extern "C" fn s_00af3db0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[98])
    }
    /// `StaticInit_PPostEffectBase_00af3e70`: class descriptor `PPostEffectBase` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPostEffectBase_00af3e70 @ 0x00af_3e70 => extern "C" fn s_00af3e70() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[99])
    }
    /// `StaticInit_PPostEffectManager_00af3f30`: class descriptor `PPostEffectManager` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PPostEffectManager_00af3f30 @ 0x00af_3f30 => extern "C" fn s_00af3f30() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[100])
    }
    /// `StaticInit_PScreenSpaceAmbientOcclusion_00af3ff0`: class descriptor `PScreenSpaceAmbientOcclusion` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceAmbientOcclusion_00af3ff0 @ 0x00af_3ff0 => extern "C" fn s_00af3ff0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[101])
    }
    /// `StaticInit_PScreenSpaceAmbientOcclusionBase_00af40b0`: class descriptor `PScreenSpaceAmbientOcclusionBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceAmbientOcclusionBase_00af40b0 @ 0x00af_40b0 => extern "C" fn s_00af40b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[102])
    }
    /// `StaticInit_PScreenSpaceAmbientOcclusionD3D11_00af4170`: class descriptor `PScreenSpaceAmbientOcclusionD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceAmbientOcclusionD3D11_00af4170 @ 0x00af_4170 => extern "C" fn s_00af4170() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[103])
    }
    /// `StaticInit_PScreenSpaceReflection_00af4230`: class descriptor `PScreenSpaceReflection` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceReflection_00af4230 @ 0x00af_4230 => extern "C" fn s_00af4230() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[104])
    }
    /// `StaticInit_PScreenSpaceReflectionBase_00af42f0`: class descriptor `PScreenSpaceReflectionBase` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceReflectionBase_00af42f0 @ 0x00af_42f0 => extern "C" fn s_00af42f0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[105])
    }
    /// `StaticInit_PScreenSpaceReflectionD3D11_00af43b0`: class descriptor `PScreenSpaceReflectionD3D11` (`cd_const_clear`): constructor, buffers, clear; atexit.
    StaticInit_PScreenSpaceReflectionD3D11_00af43b0 @ 0x00af_43b0 => extern "C" fn s_00af43b0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[106])
    }
    /// `StaticInit_BetweenBlurTiledPass_00af4470`: render pass type `BetweenBlurTiledPass` built on its global (name, flag 0); atexit.
    StaticInit_BetweenBlurTiledPass_00af4470 @ 0x00af_4470 => extern "C" fn s_00af4470() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[51])
    }
    /// `StaticInit_BlockerSearch_00af4490`: render pass type `BlockerSearch` built on its global (name, flag 0); atexit.
    StaticInit_BlockerSearch_00af4490 @ 0x00af_4490 => extern "C" fn s_00af4490() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[52])
    }
    /// `StaticInit_CalculateFocusDistance_00af44b0`: render pass type `CalculateFocusDistance` built on its global (name, flag 0); atexit.
    StaticInit_CalculateFocusDistance_00af44b0 @ 0x00af_44b0 => extern "C" fn s_00af44b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[53])
    }
    /// `StaticInit_CompositeScreenSpaceReflection_00af44d0`: render pass type `CompositeScreenSpaceReflection` built on its global (name, flag 0); atexit.
    StaticInit_CompositeScreenSpaceReflection_00af44d0 @ 0x00af_44d0 => extern "C" fn s_00af44d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[54])
    }
    /// `StaticInit_CompositeToScreenTiled_00af44f0`: render pass type `CompositeToScreenTiled` built on its global (name, flag 0); atexit.
    StaticInit_CompositeToScreenTiled_00af44f0 @ 0x00af_44f0 => extern "C" fn s_00af44f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[55])
    }
    /// `StaticInit_CompositeToScreen_00af4510`: render pass type `CompositeToScreen` built on its global (name, flag 0); atexit.
    StaticInit_CompositeToScreen_00af4510 @ 0x00af_4510 => extern "C" fn s_00af4510() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[56])
    }
    /// `StaticInit_ConvertShadowToExponential_00af4530`: render pass type `ConvertShadowToExponential` built on its global (name, flag 0); atexit.
    StaticInit_ConvertShadowToExponential_00af4530 @ 0x00af_4530 => extern "C" fn s_00af4530() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[57])
    }
    /// `StaticInit_CopyCompositedOutputToScreen_00af4550`: render pass type `CopyCompositedOutputToScreen` built on its global (name, flag 0); atexit.
    StaticInit_CopyCompositedOutputToScreen_00af4550 @ 0x00af_4550 => extern "C" fn s_00af4550() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[58])
    }
    /// `StaticInit_CopyGaussianBlurToScreen_00af4570`: render pass type `CopyGaussianBlurToScreen` built on its global (name, flag 0); atexit.
    StaticInit_CopyGaussianBlurToScreen_00af4570 @ 0x00af_4570 => extern "C" fn s_00af4570() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[59])
    }
    /// `StaticInit_CopyNoMLAA_00af4590`: render pass type `CopyNoMLAA` built on its global (name, flag 0); atexit.
    StaticInit_CopyNoMLAA_00af4590 @ 0x00af_4590 => extern "C" fn s_00af4590() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[60])
    }
    /// `StaticInit_DownsampleColorBuffer_00af45b0`: render pass type `DownsampleColorBuffer` built on its global (name, flag 0); atexit.
    StaticInit_DownsampleColorBuffer_00af45b0 @ 0x00af_45b0 => extern "C" fn s_00af45b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[61])
    }
    /// `StaticInit_DownsampleScreenSpaceReflectionDepth_00af45d0`: render pass type `DownsampleScreenSpaceReflectionDepth` built on its global (name, flag 0); atexit.
    StaticInit_DownsampleScreenSpaceReflectionDepth_00af45d0 @ 0x00af_45d0 => extern "C" fn s_00af45d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[62])
    }
    /// `StaticInit_FullBlurTiledPass_00af45f0`: render pass type `FullBlurTiledPass` built on its global (name, flag 0); atexit.
    StaticInit_FullBlurTiledPass_00af45f0 @ 0x00af_45f0 => extern "C" fn s_00af45f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[63])
    }
    /// `StaticInit_FullDofGPU_00af4610`: render pass type `FullDofGPU` built on its global (name, flag 0); atexit.
    StaticInit_FullDofGPU_00af4610 @ 0x00af_4610 => extern "C" fn s_00af4610() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[64])
    }
    /// `StaticInit_GaussianBlur_00af4630`: render pass type `GaussianBlur` built on its global (name, flag 0); atexit.
    StaticInit_GaussianBlur_00af4630 @ 0x00af_4630 => extern "C" fn s_00af4630() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[65])
    }
    /// `StaticInit_GenerateGlowBuffer_00af4650`: render pass type `GenerateGlowBuffer` built on its global (name, flag 0); atexit.
    StaticInit_GenerateGlowBuffer_00af4650 @ 0x00af_4650 => extern "C" fn s_00af4650() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[66])
    }
    /// `StaticInit_GenerateLightScatteringInstantLights_00af4670`: render pass type `GenerateLightScatteringInstantLights` built on its global (name, flag 0); atexit.
    StaticInit_GenerateLightScatteringInstantLights_00af4670 @ 0x00af_4670 => extern "C" fn s_00af4670() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[67])
    }
    /// `StaticInit_GenerateLightScattering_00af4690`: render pass type `GenerateLightScattering` built on its global (name, flag 0); atexit.
    StaticInit_GenerateLightScattering_00af4690 @ 0x00af_4690 => extern "C" fn s_00af4690() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[68])
    }
    /// `StaticInit_GenerateLightingInstantLightsTiled_00af46b0`: render pass type `GenerateLightingInstantLightsTiled` built on its global (name, flag 0); atexit.
    StaticInit_GenerateLightingInstantLightsTiled_00af46b0 @ 0x00af_46b0 => extern "C" fn s_00af46b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[69])
    }
    /// `StaticInit_GenerateLightingTiled_00af46d0`: render pass type `GenerateLightingTiled` built on its global (name, flag 0); atexit.
    StaticInit_GenerateLightingTiled_00af46d0 @ 0x00af_46d0 => extern "C" fn s_00af46d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[70])
    }
    /// `StaticInit_GenerateVelocityBuffer_00af46f0`: render pass type `GenerateVelocityBuffer` built on its global (name, flag 0); atexit.
    StaticInit_GenerateVelocityBuffer_00af46f0 @ 0x00af_46f0 => extern "C" fn s_00af46f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[71])
    }
    /// `StaticInit_NoBlurTiledPass_00af4710`: render pass type `NoBlurTiledPass` built on its global (name, flag 0); atexit.
    StaticInit_NoBlurTiledPass_00af4710 @ 0x00af_4710 => extern "C" fn s_00af4710() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[72])
    }
    /// `StaticInit_RenderCameraMotionBlur_00af4730`: render pass type `RenderCameraMotionBlur` built on its global (name, flag 0); atexit.
    StaticInit_RenderCameraMotionBlur_00af4730 @ 0x00af_4730 => extern "C" fn s_00af4730() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[73])
    }
    /// `StaticInit_RenderDirectionalLight_00af4750`: render pass type `RenderDirectionalLight` built on its global (name, flag 0); atexit.
    StaticInit_RenderDirectionalLight_00af4750 @ 0x00af_4750 => extern "C" fn s_00af4750() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[74])
    }
    /// `StaticInit_RenderEdgeDetect_00af4770`: render pass type `RenderEdgeDetect` built on its global (name, flag 0); atexit.
    StaticInit_RenderEdgeDetect_00af4770 @ 0x00af_4770 => extern "C" fn s_00af4770() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[75])
    }
    /// `StaticInit_RenderEdgeLength0_00af4790`: render pass type `RenderEdgeLength0` built on its global (name, flag 0); atexit.
    StaticInit_RenderEdgeLength0_00af4790 @ 0x00af_4790 => extern "C" fn s_00af4790() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[76])
    }
    /// `StaticInit_RenderEdgeLength_00af47b0`: render pass type `RenderEdgeLength` built on its global (name, flag 0); atexit.
    StaticInit_RenderEdgeLength_00af47b0 @ 0x00af_47b0 => extern "C" fn s_00af47b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[77])
    }
    /// `StaticInit_RenderFXAA_00af47d0`: render pass type `RenderFXAA` built on its global (name, flag 0); atexit.
    StaticInit_RenderFXAA_00af47d0 @ 0x00af_47d0 => extern "C" fn s_00af47d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[78])
    }
    /// `StaticInit_RenderGaussianBlurX_00af47f0`: render pass type `RenderGaussianBlurX` built on its global (name, flag 0); atexit.
    StaticInit_RenderGaussianBlurX_00af47f0 @ 0x00af_47f0 => extern "C" fn s_00af47f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[79])
    }
    /// `StaticInit_RenderGaussianBlurYAdditive_00af4810`: render pass type `RenderGaussianBlurYAdditive` built on its global (name, flag 0); atexit.
    StaticInit_RenderGaussianBlurYAdditive_00af4810 @ 0x00af_4810 => extern "C" fn s_00af4810() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[80])
    }
    /// `StaticInit_RenderGaussianBlurYCompositeCombine_00af4830`: render pass type `RenderGaussianBlurYCompositeCombine` built on its global (name, flag 0); atexit.
    StaticInit_RenderGaussianBlurYCompositeCombine_00af4830 @ 0x00af_4830 => extern "C" fn s_00af4830() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[81])
    }
    /// `StaticInit_RenderGaussianBlurYComposite_00af4850`: render pass type `RenderGaussianBlurYComposite` built on its global (name, flag 0); atexit.
    StaticInit_RenderGaussianBlurYComposite_00af4850 @ 0x00af_4850 => extern "C" fn s_00af4850() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[82])
    }
    /// `StaticInit_RenderGaussianBlurY_00af4870`: render pass type `RenderGaussianBlurY` built on its global (name, flag 0); atexit.
    StaticInit_RenderGaussianBlurY_00af4870 @ 0x00af_4870 => extern "C" fn s_00af4870() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[83])
    }
    /// `StaticInit_RenderHorizVertMLAA_00af4890`: render pass type `RenderHorizVertMLAA` built on its global (name, flag 0); atexit.
    StaticInit_RenderHorizVertMLAA_00af4890 @ 0x00af_4890 => extern "C" fn s_00af4890() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[84])
    }
    /// `StaticInit_RenderMLAA_00af48b0`: render pass type `RenderMLAA` built on its global (name, flag 0); atexit.
    StaticInit_RenderMLAA_00af48b0 @ 0x00af_48b0 => extern "C" fn s_00af48b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[85])
    }
    /// `StaticInit_RenderMotionBlur_00af48d0`: render pass type `RenderMotionBlur` built on its global (name, flag 0); atexit.
    StaticInit_RenderMotionBlur_00af48d0 @ 0x00af_48d0 => extern "C" fn s_00af48d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[86])
    }
    /// `StaticInit_RenderNoMLAA_00af48f0`: render pass type `RenderNoMLAA` built on its global (name, flag 0); atexit.
    StaticInit_RenderNoMLAA_00af48f0 @ 0x00af_48f0 => extern "C" fn s_00af48f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[87])
    }
    /// `StaticInit_RenderPointLight_00af4910`: render pass type `RenderPointLight` built on its global (name, flag 0); atexit.
    StaticInit_RenderPointLight_00af4910 @ 0x00af_4910 => extern "C" fn s_00af4910() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[88])
    }
    /// `StaticInit_RenderRGBL_00af4930`: render pass type `RenderRGBL` built on its global (name, flag 0); atexit.
    StaticInit_RenderRGBL_00af4930 @ 0x00af_4930 => extern "C" fn s_00af4930() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[89])
    }
    /// `StaticInit_RenderScreenSpaceReflection_00af4950`: render pass type `RenderScreenSpaceReflection` built on its global (name, flag 0); atexit.
    StaticInit_RenderScreenSpaceReflection_00af4950 @ 0x00af_4950 => extern "C" fn s_00af4950() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[90])
    }
    /// `StaticInit_RenderSpotLight_00af4970`: render pass type `RenderSpotLight` built on its global (name, flag 0); atexit.
    StaticInit_RenderSpotLight_00af4970 @ 0x00af_4970 => extern "C" fn s_00af4970() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[91])
    }
    /// `StaticInit_RenderVolumetricLight_00af4990`: render pass type `RenderVolumetricLight` built on its global (name, flag 0); atexit.
    StaticInit_RenderVolumetricLight_00af4990 @ 0x00af_4990 => extern "C" fn s_00af4990() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[92])
    }
    /// `StaticInit_ScreenSpaceReflectionPrePass_00af49b0`: render pass type `ScreenSpaceReflectionPrePass` built on its global (name, flag 0); atexit.
    StaticInit_ScreenSpaceReflectionPrePass_00af49b0 @ 0x00af_49b0 => extern "C" fn s_00af49b0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[93])
    }
    /// `StaticInit_ShadowDirectionalLight_00af49d0`: render pass type `ShadowDirectionalLight` built on its global (name, flag 0); atexit.
    StaticInit_ShadowDirectionalLight_00af49d0 @ 0x00af_49d0 => extern "C" fn s_00af49d0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[94])
    }
    /// `StaticInit_ShadowPointLight_00af49f0`: render pass type `ShadowPointLight` built on its global (name, flag 0); atexit.
    StaticInit_ShadowPointLight_00af49f0 @ 0x00af_49f0 => extern "C" fn s_00af49f0() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[95])
    }
    /// `StaticInit_ShadowSpotLight_00af4a10`: render pass type `ShadowSpotLight` built on its global (name, flag 0); atexit.
    StaticInit_ShadowSpotLight_00af4a10 @ 0x00af_4a10 => extern "C" fn s_00af4a10() -> u32 {
        si::named_type(&mut mem(), &si::NAMED_TYPES[96])
    }
    /// `StaticInit_ChainAppend_00af4a30`: appends the object at `0x00c6_8494` (vtable 0x00b73f70) to the chain at 0xc901a8 (before the first block at least 0xff full); atexit(0x00b09010).
    StaticInit_ChainAppend_00af4a30 @ 0x00af_4a30 => extern "C" fn s_00af4a30() -> u32 {
        si::list_append(&mut mem(), &si::LIST_APPENDS[4])
    }
    /// `StaticInit_PBitmapFont_00af4ad0`: class descriptor `PBitmapFont` (`cd_desc_set8`): constructor, buffers, set8; atexit.
    StaticInit_PBitmapFont_00af4ad0 @ 0x00af_4ad0 => extern "C" fn s_00af4ad0() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[107])
    }
    /// `StaticInit_PBitmapFontCharInfo_00af4b90`: class descriptor `PBitmapFontCharInfo` (`cd_desc_clear`): constructor, buffers, clear; atexit.
    StaticInit_PBitmapFontCharInfo_00af4b90 @ 0x00af_4b90 => extern "C" fn s_00af4b90() -> u32 {
        si::class_decl(&mut mem(), &si::CLASS_DECLS[108])
    }
}
