//! M5 stage 1, list `phyre-core`, batch 4 (`ffx_pcore::m5p_core_d`):
//! `PWorldMatrix`'s look-at and copy, the Euler rotation matrix, the
//! 3-vector lerp, the character lighting's added-light parameters, the
//! shadow caster type's unlink, the tree's node count, byte-swaps, a
//! `PSharray` resize and the LOD group's switches.
//!
//! Threads: the lighting writes run on the render side of the frame
//! (main thread); the rest where objects are made and updated. No shim
//! keeps state; the locals callees see are per-call `StackFrame`s.

use super::{StackFrame, mem, ported};
use ffx_pcore::m5p_core_d::bridge as b;

ported! {
    /// `Phyre_PWorldMatrix_lookAt_00452520(this; &m)`; the 4x4 and the look-at's vectors in a per-call frame.
    Phyre_PWorldMatrix_lookAt_00452520 @ 0x0045_2520 => extern "thiscall" fn world_look_at(this: u32, m: u32) -> u32 {
        let mut frame = StackFrame::<0x1c>::new();
        b::world_look_at(&mut mem(), this, m, frame.at())
    }
    /// `Phyre_PWorldMatrix_copy3x4_00452740(this; &m)` -> `m`.
    Phyre_PWorldMatrix_copy3x4_00452740 @ 0x0045_2740 => extern "thiscall" fn copy_3x4(this: u32, m: u32) -> u32 {
        b::copy_3x4(&mut mem(), this, m)
    }
    /// `Phyre_PShadowCasterType_unlink_004cc050(type)` -> 0.
    Phyre_PShadowCasterType_unlink_004cc050 @ 0x004c_c050 => extern "C" fn shadow_type_unlink(ty: u32) -> u32 {
        b::shadow_type_unlink(&mut mem(), ty)
    }
    /// `Phyre_Vec3Lerp_006a9b40(out, t, &a, &b)` -> `out`.
    Phyre_Vec3Lerp_006a9b40 @ 0x006a_9b40 => extern "C" fn lerp3(out: u32, t: u32, a: u32, v: u32) -> u32 {
        b::lerp3(&mut mem(), out, t, a, v)
    }
    /// `PMatrix4_fromEuler_00574520(out, &angles)` -> `out`.
    PMatrix4_fromEuler_00574520 @ 0x0057_4520 => extern "C" fn euler_matrix(out: u32, angles: u32) -> u32 {
        b::euler_matrix(&mut mem(), out, angles)
    }
    /// `Phyre_PRedBlackTree_countNodes(tree)` -> the count.
    Phyre_PRedBlackTree_countNodes @ 0x0057_4860 => extern "thiscall" fn count_nodes(this: u32) -> u32 {
        b::count_nodes(&mut mem(), this)
    }
    /// `Phyre_PNamedMembers_reset_0055afc0(this)`.
    Phyre_PNamedMembers_reset_0055afc0 @ 0x0055_afc0 => extern "thiscall" fn reset_names(this: u32) -> u32 {
        b::reset_names(&mut mem(), this)
    }
    /// `Phyre_byteSwap4_00576c70(p)`.
    Phyre_byteSwap4_00576c70 @ 0x0057_6c70 => extern "C" fn swap4(p: u32) -> u32 {
        b::swap4(&mut mem(), p)
    }
    /// `Phyre_byteSwap3_00576d20(p)`.
    Phyre_byteSwap3_00576d20 @ 0x0057_6d20 => extern "C" fn swap3(p: u32) -> u32 {
        b::swap3(&mut mem(), p)
    }
    /// `Phyre_PSharray_resize_00599540(this; n)` -> 0 or 0xd.
    Phyre_PSharray_resize_00599540 @ 0x0059_9540 => extern "thiscall" fn sharray_resize(this: u32, n: u32) -> u32 {
        b::sharray_resize(&mut mem(), this, n)
    }
    /// `Phyre_PFloatPairs_setByKey_006ac0b0(this; key, f)` -> the count.
    Phyre_PFloatPairs_setByKey_006ac0b0 @ 0x006a_c0b0 => extern "thiscall" fn set_by_key(this: u32, key: u32, f: u32) -> u32 {
        b::set_by_key(&mut mem(), this, key, f)
    }
    /// `Phyre_PLODGroup_resetLevels_005a7990(group)` -> `group`.
    Phyre_PLODGroup_resetLevels_005a7990 @ 0x005a_7990 => extern "thiscall" fn lod_levels_reset(this: u32) -> u32 {
        b::lod_levels_reset(&mut mem(), this)
    }
    /// `Phyre_PLODLevel_setOwners_005a83a0(level; flag)`.
    Phyre_PLODLevel_setOwners_005a83a0 @ 0x005a_83a0 => extern "thiscall" fn lod_level_owners(this: u32, flag: u32) {
        b::lod_level_owners(&mut mem(), this, flag);
    }
    /// `Phyre_PLODGroup_setEnabled_005a7d10(group; flag)`.
    Phyre_PLODGroup_setEnabled_005a7d10 @ 0x005a_7d10 => extern "thiscall" fn lod_group_enable(this: u32, flag: u32) {
        b::lod_group_enable(&mut mem(), this, flag);
    }
    /// `Phyre_CharacterLighting_setAddLight_006a9bb0(cl; inst, r, g, b, intensity)`; the parameters in a per-call frame.
    Phyre_CharacterLighting_setAddLight_006a9bb0 @ 0x006a_9bb0 => extern "thiscall" fn add_light(this: u32, inst: u32, r: u32, g: u32, bl: u32, intensity: u32) {
        let mut frame = StackFrame::<9>::new();
        b::add_light(&mut mem(), this, inst, [r, g, bl], intensity, frame.at());
    }
    /// `Phyre_CharacterLighting_setAddLightMaterial_006a9cd0(cl; inst, c0..c3, s0..s2, intensity)`; the parameters in a per-call frame.
    Phyre_CharacterLighting_setAddLightMaterial_006a9cd0 @ 0x006a_9cd0 => extern "thiscall" fn add_light_material(this: u32, inst: u32, c0: u32, c1: u32, c2: u32, c3: u32, s0: u32, s1: u32, s2: u32, intensity: u32) {
        let mut frame = StackFrame::<13>::new();
        b::add_light_material(&mut mem(), this, inst, [c0, c1, c2, c3], [s0, s1, s2], intensity, frame.at());
    }
    /// `Phyre_PCamera_assign_004533b0(dst; &src)` -> `dst`.
    Phyre_PCamera_assign_004533b0 @ 0x0045_33b0 => extern "thiscall" fn camera_assign(this: u32, src: u32) -> u32 {
        b::camera_assign(&mut mem(), this, src)
    }
}
