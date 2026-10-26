//! M4 coverage 2's new functions (list `c2-new`): the shadow cascades'
//! caster rectangles (`ffx_prender::c2_shadow`), FXAA, the SMAA parameter
//! store and the D3D11 resolve's element word (`ffx_prender::c2_post`),
//! SMAA (`ffx_prender::c2_smaa`),
//! and the PC menu's `ExitAudioMenu` callback
//! (`ffx_glue_other::c2_iggy`).
//!
//! Threads: the main thread (the render pass and the menu); the shims
//! keep no state, frames are per call.

use super::{StackFrame, mem, ported};
use ffx_glue_other::c2_iggy::bridge as gi;
use ffx_prender::c2_post::bridge as gp;
use ffx_prender::c2_shadow::bridge as gs;
use ffx_prender::c2_smaa::bridge as gm;

ported! {
    // ---- the shadow cascades (settings-shadow) ----
    /// `FUN_006c7190(dst, src, n)`.
    ShadowRects_copyRecords_006c7190 @ 0x006c_7190 => extern "C" fn copy_records(dst: u32, src: u32, n: u32) {
        gs::copy_records(&mut mem(), dst, src, n);
    }
    /// `FUN_006c7cc0(this; m3x4, mat, a, b)`: frame `ebp - 0x80`.
    ShadowCascadeView_init_006c7cc0 @ 0x006c_7cc0 => extern "thiscall" fn view_record_init(this: u32, m: u32, mat: u32, a: u32, b: u32) -> u32 {
        let mut f = StackFrame::<0x20>::new();
        gs::view_record_init(&mut mem(), f.at() + 0x80, this, m, mat, f32::from_bits(a), f32::from_bits(b))
    }
    /// `FUN_006c88a0(this; i, scale_x, scale_y)`: frame `ebp - 0x14c`.
    ShadowManager_buildCasterRects_006c88a0 @ 0x006c_88a0 => extern "thiscall" fn build_rects(this: u32, i: u32, sx: u32, sy: u32) {
        let mut f = StackFrame::<0x54>::new();
        gs::build_rects(&mut mem(), f.at() + 0x150, this, i, sx, sy);
    }
    /// `FUN_006c8f50(this; view (16 words), flag, ratio, scale_x,
    /// scale_y)` -> `al`: frame `ebp - 0x17c`; the bridge reads the
    /// arguments in place (the view matrix by address).
    ShadowManager_casterCrossesEdge_006c8f50 @ 0x006c_8f50 => extern "thiscall" fn caster_crosses_edge(
        this: u32, m0: u32, m1: u32, m2: u32, m3: u32, m4: u32, m5: u32, m6: u32, m7: u32,
        m8: u32, m9: u32, m10: u32, m11: u32, m12: u32, m13: u32, m14: u32, m15: u32,
        flag: u32, ratio: u32, sx: u32, sy: u32; slots: u32
    ) -> u8 {
        let _ = (m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15);
        let _ = (flag, ratio, sx, sy);
        let mut f = StackFrame::<0x60>::new();
        gs::caster_crosses_edge(&mut mem(), f.at() + 0x180, this, slots)
    }
    /// `FUN_006c9780(this)` (`ecx`).
    ShadowManager_syncCascadeMatrices_006c9780 @ 0x006c_9780 => extern "thiscall" fn sync_cascades(this: u32) {
        gs::sync_cascades(&mut mem(), this);
    }
    /// `FUN_006d12f0(this; i, meshes, extras, scale, unused, depth)`.
    ShadowManager_setupGradMeshesA_006d12f0 @ 0x006d_12f0 => extern "thiscall" fn grad_meshes_a(this: u32, i: u32, meshes: u32, extras: u32, scale: u32, unused: u32, depth: u32) {
        gs::grad_meshes_a(&mut mem(), this, i, meshes, extras, scale, unused, depth);
    }
    /// `FUN_006d1530(this; i, meshes, extras, scale, unused, depth)`.
    ShadowManager_setupGradMeshesB_006d1530 @ 0x006d_1530 => extern "thiscall" fn grad_meshes_b(this: u32, i: u32, meshes: u32, extras: u32, scale: u32, unused: u32, depth: u32) {
        gs::grad_meshes_b(&mut mem(), this, i, meshes, extras, scale, unused, depth);
    }
    /// `FUN_006d17d0(this; &record)`.
    PArray16_pushBack_006d17d0 @ 0x006d_17d0 => extern "thiscall" fn push_record(this: u32, record: u32) -> u32 {
        gs::push_record(&mut mem(), this, record)
    }
    /// `FUN_006d1800(this; i)`.
    ShadowManager_alignCasterRects_006d1800 @ 0x006d_1800 => extern "thiscall" fn align_rects(this: u32, i: u32) {
        gs::align_rects(&mut mem(), this, i);
    }
    /// `FUN_006d2760(this; storage, &record)`.
    PArray16_appendInto_006d2760 @ 0x006d_2760 => extern "thiscall" fn append_record(this: u32, storage: u32, record: u32) -> u32 {
        gs::append_record(&mut mem(), this, storage, record)
    }
    /// `FUN_006d2b50(a0, a1, a2, a3, b0, b1, b2, b3, &o0, &o1, &o2, &o3)`.
    Rect_unionIfOverlap_006d2b50 @ 0x006d_2b50 => extern "stdcall" fn rect_union(
        a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32,
        o0: u32, o1: u32, o2: u32, o3: u32
    ) -> u32 {
        gs::rect_union(&mut mem(), [a0, a1, a2, a3], [b0, b1, b2, b3], [o0, o1, o2, o3])
    }

    // ---- post anti-aliasing (settings-fxaa, settings-smaa) ----
    /// `PostProcessManager_renderFXAA(this; renderer, target)`.
    PostProcessManager_renderFXAA @ 0x0069_5740 => extern "thiscall" fn render_fxaa(this: u32, renderer: u32, target: u32) -> u32 {
        gp::render_fxaa(&mut mem(), this, renderer, target)
    }
    /// `PostProcessManager_renderSMAA(this; renderer)`: frame `ebp -
    /// 0x104`.
    PostProcessManager_renderSMAA @ 0x0069_6af0 => extern "thiscall" fn render_smaa(this: u32, renderer: u32) -> u32 {
        let mut f = StackFrame::<0x42>::new();
        gm::render_smaa(&mut mem(), f.at() + 0x108, this, renderer)
    }
    /// `FUN_00698900(buffer; desc, src)`.
    ShaderParamBuffer_setValueChecked_00698900 @ 0x0069_8900 => extern "thiscall" fn set_value_checked(buffer: u32, desc: u32, src: u32) -> u32 {
        gp::set_value_checked(&mut mem(), buffer, desc, src)
    }
    /// `FUN_00a20350(this; renderer, target, source, luma)`.
    PostFXAA_render_00a20350 @ 0x00a2_0350 => extern "thiscall" fn fxaa_forward(this: u32, renderer: u32, target: u32, source: u32, luma: u32) -> u32 {
        gp::fxaa_forward(&mut mem(), this, renderer, target, source, luma)
    }
    /// `FUN_00a20360(this; renderer, target, source, luma)`: frame `ebp -
    /// 0x60`.
    PostFXAA_renderPass_00a20360 @ 0x00a2_0360 => extern "thiscall" fn fxaa_pass(this: u32, renderer: u32, target: u32, source: u32, luma: u32) -> u32 {
        let mut f = StackFrame::<0x18>::new();
        gp::fxaa_pass(&mut mem(), f.at() + 0x60, this, renderer, target, source, luma)
    }

    // ---- the D3D11 resolve (settings-msaa) ----
    /// `FUN_00593e80(this; i)`.
    Phyre_PRendering_ElementField0_00593e80 @ 0x0059_3e80 => extern "thiscall" fn element_word(this: u32, i: u32) -> u32 {
        gp::element_word(&mut mem(), this, i)
    }

    // ---- the PC menu (iggy-menus) ----
    /// `FFEscMenu_onExitAudioMenu(handlers, args)`.
    FFEscMenu_onExitAudioMenu @ 0x006f_f5f0 => extern "C" fn on_exit_audio_menu(handlers: u32, args: u32) -> u32 {
        gi::on_exit_audio_menu(&mut mem(), handlers, args)
    }
}
