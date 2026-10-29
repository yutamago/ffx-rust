//! M5 stage 1, list `scene`, batch 3 (`ffx_prender::m5p_scene_c`): the
//! utilities' registry, array resizes and element constructors, list
//! unlinks, the scene cursor and more small methods. Parity tests:
//! `crates/ffx-oracle/tests/m5p_scene.rs` (`m5p_scene_c_*`). The six
//! functions under five bytes have bridges and tests but no hook
//! (`symbols/m5-not-portable.tsv`).

use ffx_prender::m5p_scene_c as t;
use ffx_prender::m5p_scene_c::bridge as b;

use super::{mem, ported};

ported! {
    /// `Phyre_PScene_PUtilityScene_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PScene_PUtilityScene_scalar_deleting_dtor @ 0x0043_2b80 => extern "thiscall" fn sc_00432b80(this: u32, flags: u32) -> u32 {
        b::utility_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PAnimation_PUtilityAnimation_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PAnimation_PUtilityAnimation_scalar_deleting_dtor @ 0x0043_2ce0 => extern "thiscall" fn sc_00432ce0(this: u32, flags: u32) -> u32 {
        b::utility_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PGameplay_PUtilityGameplay_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PGameplay_PUtilityGameplay_scalar_deleting_dtor @ 0x0043_3460 => extern "thiscall" fn sc_00433460(this: u32, flags: u32) -> u32 {
        b::utility_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PGameplay_TypeList_unlink_00433550`: the node unlinked from the gameplay type list; 0.
    Phyre_PGameplay_TypeList_unlink_00433550 @ 0x0043_3550 => extern "C" fn sc_00433550(node: u32) -> u32 {
        b::list_unlink(&mut mem(), &t::GAMEPLAY_TYPES, node)
    }
    /// `PhyreSceneCursor_seek_00443930`: steps until the owner lists the node, then settles.
    PhyreSceneCursor_seek_00443930 @ 0x0044_3930 => extern "thiscall" fn sc_00443930(it: u32) -> u32 {
        b::cursor_seek(&mut mem(), it)
    }
    /// `PhyreSceneCursor_next_004439a0`: one step at least, then as the seek.
    PhyreSceneCursor_next_004439a0 @ 0x0044_39a0 => extern "thiscall" fn sc_004439a0(it: u32) -> u32 {
        b::cursor_next(&mut mem(), it)
    }
    /// `Phyre_PEntity_isNewWorldClass_0044ff60`: whether a world-derived class is new to the list (al).
    Phyre_PEntity_isNewWorldClass_0044ff60 @ 0x0044_ff60 => extern "thiscall" fn sc_0044ff60(this: u32, class: u32) -> u8 {
        b::world_class_new(&mem(), this, class)
    }
    /// `Phyre_PEntity_detachComponent_00450450`: the component unlinked from its owner; 5 for another owner, else 0.
    Phyre_PEntity_detachComponent_00450450 @ 0x0045_0450 => extern "thiscall" fn sc_00450450(this: u32, component: u32) -> u32 {
        b::component_detach(&mut mem(), this, component)
    }
    /// `Phyre_PEntity_setClass_00450500`: the class node moved in the class tree.
    Phyre_PEntity_setClass_00450500 @ 0x0045_0500 => extern "thiscall" fn sc_00450500(this: u32, class: u32) -> u32 {
        b::entity_set_class(&mut mem(), this, class)
    }
    /// `Phyre_PInstancesComponent_findByClass_004599c0`: the element of a class into `out`; `out`.
    Phyre_PInstancesComponent_findByClass_004599c0 @ 0x0045_99c0 => extern "thiscall" fn sc_004599c0(this: u32, out: u32, class: u32) -> u32 {
        b::instances_find(&mut mem(), this, out, class)
    }
    /// `Phyre_PRendering_PShadowCaster_CheckLightShadowCasterCompatibility`: whether the pair is compatible (al).
    Phyre_PRendering_PShadowCaster_CheckLightShadowCasterCompatibility @ 0x004c_ae20 => extern "C" fn sc_004cae20(light: u32, caster: u32) -> u8 {
        b::light_caster_compatible(&mem(), light, caster)
    }
    /// `Phyre_PRendering_PContextSwitchType_ctor_004d2e50`: the type built and listed when new; `this`.
    Phyre_PRendering_PContextSwitchType_ctor_004d2e50 @ 0x004d_2e50 => extern "thiscall" fn sc_004d2e50(this: u32, _unused: u32, name: u32) -> u32 {
        b::switch_type_ctor(&mut mem(), this, name)
    }
    /// `Phyre_PShaderPassInfo_zeroItems_004df240`: eight words of each 32-byte item zeroed.
    Phyre_PShaderPassInfo_zeroItems_004df240 @ 0x004d_f240 => extern "C" fn sc_004df240(items: u32, n: u32) {
        b::zero_items32(&mut mem(), items, n);
    }
    /// `Phyre_PArray_PSceneRenderPassPtr_resize_004e74c0`: resized through its reallocation; 0 or 0xd.
    Phyre_PArray_PSceneRenderPassPtr_resize_004e74c0 @ 0x004e_74c0 => extern "thiscall" fn sc_004e74c0(this: u32, n: u32) -> u32 {
        b::render_pass_resize(&mut mem(), this, n)
    }
    /// `Phyre_PSharray_copyConstruct_004f6db0`: a buffer of `n` with the source words; the buffer.
    Phyre_PSharray_copyConstruct_004f6db0 @ 0x004f_6db0 => extern "thiscall" fn sc_004f6db0(this: u32, n: u32, src: u32) -> u32 {
        b::sharray_copy(&mut mem(), this, n, src)
    }
    /// `Phyre_PSharray_copyFill_004f6e10`: a buffer of `n`, `m` source words and zeros; the buffer.
    Phyre_PSharray_copyFill_004f6e10 @ 0x004f_6e10 => extern "thiscall" fn sc_004f6e10(this: u32, n: u32, src: u32, m: u32) -> u32 {
        b::sharray_copy_fill(&mut mem(), this, n, src, m)
    }
    /// `Phyre_PArray_PConstantBuffer_clear_004fd160`: the constant buffers destroyed and freed, the header cleared.
    Phyre_PArray_PConstantBuffer_clear_004fd160 @ 0x004f_d160 => extern "thiscall" fn sc_004fd160(this: u32) -> u32 {
        b::constant_buffers_clear(&mut mem(), this)
    }
    /// `Phyre_PLOD_PUtilityLOD_scalar_deleting_dtor`: unregistered, deleted for `flags & 1`; `this`.
    Phyre_PLOD_PUtilityLOD_scalar_deleting_dtor @ 0x0050_41e0 => extern "thiscall" fn sc_005041e0(this: u32, flags: u32) -> u32 {
        b::utility_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PGameplay_LocatorShapeType_unlink_00545e80`: the node unlinked from the locator shape type list; 0.
    Phyre_PGameplay_LocatorShapeType_unlink_00545e80 @ 0x0054_5e80 => extern "C" fn sc_00545e80(node: u32) -> u32 {
        b::list_unlink(&mut mem(), &t::LOCATOR_SHAPES, node)
    }
    /// `Phyre_PSharray_copyFill_0054fa00`: a buffer of `n`, `m` source words and zeros; the buffer.
    Phyre_PSharray_copyFill_0054fa00 @ 0x0054_fa00 => extern "thiscall" fn sc_0054fa00(this: u32, n: u32, src: u32, m: u32) -> u32 {
        b::sharray_copy_fill(&mut mem(), this, n, src, m)
    }
    /// `Phyre_PGameplay_nameIs_00550430`: whether the held name equals `name` (al).
    Phyre_PGameplay_nameIs_00550430 @ 0x0055_0430 => extern "thiscall" fn sc_00550430(this: u32, name: u32) -> u8 {
        b::name_is(&mem(), this, name)
    }
    /// `Phyre_PGameplay_PTriggerReceivers_set_00550750`: the receivers buffer set; 0.
    Phyre_PGameplay_PTriggerReceivers_set_00550750 @ 0x0055_0750 => extern "thiscall" fn sc_00550750(this: u32, buffer: u32, n: u32) -> u32 {
        b::receivers_set(&mut mem(), this, buffer, n)
    }
    /// `Phyre_PGameplay_PTimerComponent_assign_00555e20`: the timer copied from a source; 1 for none, else 0.
    Phyre_PGameplay_PTimerComponent_assign_00555e20 @ 0x0055_5e20 => extern "thiscall" fn sc_00555e20(this: u32, src: u32) -> u32 {
        b::timer_assign(&mut mem(), this, src)
    }
    /// `Phyre_PSharray_copyConstruct_005a6aa0`: a buffer of `n` with the source words; the buffer.
    Phyre_PSharray_copyConstruct_005a6aa0 @ 0x005a_6aa0 => extern "thiscall" fn sc_005a6aa0(this: u32, n: u32, src: u32) -> u32 {
        b::sharray_copy(&mut mem(), this, n, src)
    }
    /// `Phyre_PSharray_copyFill_005a6ba0`: a buffer of `n`, `m` source words and zeros; the buffer.
    Phyre_PSharray_copyFill_005a6ba0 @ 0x005a_6ba0 => extern "thiscall" fn sc_005a6ba0(this: u32, n: u32, src: u32, m: u32) -> u32 {
        b::sharray_copy_fill(&mut mem(), this, n, src, m)
    }
    /// `PhyreSceneCursor_ctor_009f2470`: a cursor over the owner's node ring, sought; `this`.
    PhyreSceneCursor_ctor_009f2470 @ 0x009f_2470 => extern "thiscall" fn sc_009f2470(this: u32, owner: u32, mask: u32) -> u32 {
        b::cursor_ctor(&mut mem(), this, owner, mask)
    }
    /// `PhyreSceneCursor_seek_009f38c0`: steps until the owner lists the node, then settles (a copy of 0x00443930).
    PhyreSceneCursor_seek_009f38c0 @ 0x009f_38c0 => extern "thiscall" fn sc_009f38c0(it: u32) -> u32 {
        b::cursor_seek(&mut mem(), it)
    }
    /// `Phyre_PArray8_reset_00a30780`: a fresh buffer of `n` 8-byte elements; 0 or 0xd.
    Phyre_PArray8_reset_00a30780 @ 0x00a3_0780 => extern "thiscall" fn sc_00a30780(this: u32, n: u32) -> u32 {
        b::array8_reset(&mut mem(), this, n)
    }
    /// `Phyre_PArray_shrink_00a307f0`: the count cut to `n`, an owned emptied array freed.
    Phyre_PArray_shrink_00a307f0 @ 0x00a3_07f0 => extern "thiscall" fn sc_00a307f0(this: u32, n: u32) -> u32 {
        b::array_shrink(&mut mem(), this, n)
    }
}
