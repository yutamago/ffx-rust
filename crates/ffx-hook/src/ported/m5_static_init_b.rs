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
}
