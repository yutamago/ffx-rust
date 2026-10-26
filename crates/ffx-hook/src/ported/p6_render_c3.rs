//! M4 port, list p6-render-c, continuation round 2 (batch 11 on): the
//! shims. Ports in `ffx_prender::p6_small`.

use super::{StackFrame, mem, ported};
use ffx_prender::p6_small as sm;

ported! {
    /// `renderMeshInstanceList`'s thunk (`this`; a, b, c) -> its `eax`.
    Phyre_PRendering_PRendererBase_renderMeshInstanceList_thunk @ 0x005b_a360 => extern "thiscall" fn t_005ba360(this: u32, a: u32, b: u32, c: u32) -> u32 {
        let (target, _) = const { sm::thunk(0x005b_a360) };
        sm::bridge::thunk(&mut mem(), target, this, &[a, b, c])
    }
    /// `PLegacyGlowGPUBase`'s thunk (`this`; a, b, c) -> its `eax`.
    Phyre_PPostProcessing_PLegacyGlowGPUBase_thunk_00a17980 @ 0x00a1_7980 => extern "thiscall" fn t_00a17980(this: u32, a: u32, b: u32, c: u32) -> u32 {
        let (target, _) = const { sm::thunk(0x00a1_7980) };
        sm::bridge::thunk(&mut mem(), target, this, &[a, b, c])
    }
    /// `PFXAABase::initializeCommonShaders`'s thunk (`this`; a, b) -> its `eax`.
    Phyre_PPostProcessing_PFXAA_initializeCommonShaders_thunk_00a1a810 @ 0x00a1_a810 => extern "thiscall" fn t_00a1a810(this: u32, a: u32, b: u32) -> u32 {
        let (target, _) = const { sm::thunk(0x00a1_a810) };
        sm::bridge::thunk(&mut mem(), target, this, &[a, b])
    }
    /// `PApplication::isKeyDown(this; key, i)` -> the input's float.
    Phyre_PFramework_PApplication_isKeyDown @ 0x0062_8460 => extern "thiscall" fn t_00628460(this: u32, key: u32, i: u32) -> f32 {
        sm::bridge::is_key_down(&mut mem(), this, key, i)
    }
    /// `PModifierNetworkDynamicMesh::sync(mesh)`.
    Phyre_PDynamicGeometry_PModifierNetworkDynamicMesh_sync @ 0x005a_fca0 => extern "stdcall" fn t_005afca0(mesh: u32) {
        sm::bridge::sync(&mut mem(), mesh);
    }
    /// The renderer's room check (`this`; bytes, commands) -> `al`.
    Phyre_PRendering_PRendererBase_hasRoom_005b6a70 @ 0x005b_6a70 => extern "thiscall" fn t_005b6a70(this: u32, bytes: u32, commands: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        let mut arg = StackFrame::<1>::new();
        sm::bridge::has_room(&mut mem(), this, frame.at(), arg.at(), bytes, commands)
    }
    /// A visit with a `PDeleteChildrenVisitor` (`this`; a) -> `al` 1.
    Phyre_PInternal_visitDeleteChildren_005c5e50 @ 0x005c_5e50 => extern "thiscall" fn t_005c5e50(this: u32, a: u32) -> u32 {
        let mut frame = StackFrame::<2>::new();
        sm::bridge::visit_delete(&mut mem(), this, frame.at(), a)
    }
    /// `GetType<PModifier>()` -> the descriptor.
    Phyre_PTypeDispenser_GetType_Phyre_PDynamicGeometry_PModifier @ 0x005c_7fe0 => extern "C" fn t_005c7fe0() -> u32 {
        sm::bridge::modifier_type(&mut mem())
    }
    /// The file's seek (`this`; distance, method) -> the new position.
    Phyre_PFile_seek_006080e0 @ 0x0060_80e0 => extern "thiscall" fn t_006080e0(this: u32, distance: u32, method: u32) -> u32 {
        sm::bridge::file_seek(&mut mem(), this, distance, method)
    }
    /// An owned array of 0x40-byte elements destroyed (`this`).
    Phyre_PRendering_PArray64_destroy_006141a0 @ 0x0061_41a0 => extern "thiscall" fn t_006141a0(this: u32) -> u32 {
        sm::bridge::array_destroy(&mut mem(), this)
    }
    /// The recorded releases run (`this`; context) -> the word `+4`.
    Phyre_PRendering_PRenderThread_flushReleases_00614800 @ 0x0061_4800 => extern "thiscall" fn t_00614800(this: u32, context: u32) -> u32 {
        sm::bridge::flush(&mut mem(), this, context)
    }
    /// `PInput::Update()` -> 0 or 9.
    Phyre_PFramework_PInput_Update @ 0x0062_5770 => extern "C" fn t_00625770() -> u32 {
        sm::bridge::input_update(&mut mem())
    }
    /// A new 3-word node (`ecx` = the word's address) -> the node.
    std_list_newNode_00625880 @ 0x0062_5880 => extern "thiscall" fn t_00625880(value: u32) -> u32 {
        sm::bridge::new_node(&mut mem(), value)
    }
    /// The list entry handed to `p` (p) -> `al`.
    Phyre_PFramework_lookupUse_00626c40 @ 0x0062_6c40 => extern "stdcall" fn t_00626c40(p: u32) -> u32 {
        sm::bridge::lookup_use(&mut mem(), p)
    }
    /// The application timer's frame step (`this`).
    Phyre_PFramework_PApplication_stepTimer_00628df0 @ 0x0062_8df0 => extern "thiscall" fn t_00628df0(this: u32) {
        sm::bridge::timer_step(&mut mem(), this);
    }
    /// `PCheckVisibleBoundsJob`'s creation -> the job.
    PCheckVisibleBoundsJob_create @ 0x009f_09e0 => extern "C" fn t_009f09e0() -> u32 {
        sm::bridge::job_create(&mut mem())
    }
    /// `startOccluderJobs(this; cursor)`.
    Phyre_PRendering_PVisibilityCheck_startOccluderJobs @ 0x009f_20f0 => extern "thiscall" fn t_009f20f0(this: u32, cursor: u32) -> u32 {
        sm::bridge::start_occluder_jobs(&mut mem(), this, cursor)
    }
    /// `startJobsForAllBounds(this; cursor)`.
    Phyre_PRendering_PVisibilityCheck_startJobsForAllBounds @ 0x009f_2250 => extern "thiscall" fn t_009f2250(this: u32, cursor: u32) -> u32 {
        sm::bridge::start_jobs_for_all_bounds(&mut mem(), this, cursor)
    }
    /// The occluder cursor's constructor (`this`; world) -> `this`.
    PWorldCursor_OccluderGeometryInstance_009f23f0 @ 0x009f_23f0 => extern "thiscall" fn t_009f23f0(this: u32, world: u32) -> u32 {
        sm::bridge::occluder_cursor(&mut mem(), this, world)
    }
    /// `~PVisibilityCheck(this)`.
    Phyre_PRendering_PVisibilityCheck_dtor @ 0x009f_26c0 => extern "thiscall" fn t_009f26c0(this: u32) -> u32 {
        sm::bridge::visibility_dtor(&mut mem(), this)
    }
    /// The visibility check's jobs released (`this`).
    Phyre_PRendering_PVisibilityCheck_releaseJobs_009f3590 @ 0x009f_3590 => extern "thiscall" fn t_009f3590(this: u32) -> u32 {
        sm::bridge::release_jobs(&mut mem(), this)
    }
    /// A deleting destructor (`this`; flags) -> `this`.
    Phyre_PPostProcessing_PDepthOfFieldD3D11_deletingDtor_00a027a0 @ 0x00a0_27a0 => extern "thiscall" fn t_00a027a0(this: u32, flags: u32) -> u32 {
        sm::bridge::deleting_dtor(&mut mem(), this, flags, const { sm::deleting(0x00a0_27a0) })
    }
    /// A deleting destructor (`this`; flags) -> `this`.
    Phyre_PPostProcessing_PDepthOfFieldD3D11_deletingDtor_00a02850 @ 0x00a0_2850 => extern "thiscall" fn t_00a02850(this: u32, flags: u32) -> u32 {
        sm::bridge::deleting_dtor(&mut mem(), this, flags, const { sm::deleting(0x00a0_2850) })
    }
    /// A deleting destructor (`this`; flags) -> `this`.
    Phyre_PPostProcessing_PMotionBlurD3D11_deletingDtor_00a02e90 @ 0x00a0_2e90 => extern "thiscall" fn t_00a02e90(this: u32, flags: u32) -> u32 {
        sm::bridge::deleting_dtor(&mut mem(), this, flags, const { sm::deleting(0x00a0_2e90) })
    }
    /// A deleting destructor (`this`; flags) -> `this`.
    Phyre_PPostProcessing_PMotionBlurD3D11_deletingDtor_00a02f40 @ 0x00a0_2f40 => extern "thiscall" fn t_00a02f40(this: u32, flags: u32) -> u32 {
        sm::bridge::deleting_dtor(&mut mem(), this, flags, const { sm::deleting(0x00a0_2f40) })
    }
    /// A render target resized (target, w, h, force) -> 0 or an error.
    Phyre_PPostProcessing_renderTargetResize_00a0e4b0 @ 0x00a0_e4b0 => extern "C" fn t_00a0e4b0(target: u32, w: u32, h: u32, force: u32) -> u32 {
        sm::bridge::target_resize(&mut mem(), target, w, h, force)
    }
    /// A render target's texture unbound (target).
    Phyre_PPostProcessing_renderTargetRelease_00a0e520 @ 0x00a0_e520 => extern "C" fn t_00a0e520(target: u32) -> u32 {
        sm::bridge::target_release(&mut mem(), target)
    }
    /// The depth of field's shaders (`this`; a, effect) -> 0 or an error.
    Phyre_PPostProcessing_PDepthOfField_initializeShaders_00a1a6c0 @ 0x00a1_a6c0 => extern "thiscall" fn t_00a1a6c0(this: u32, a: u32, effect: u32) -> u32 {
        sm::bridge::dof_shaders(&mut mem(), this, a, effect)
    }
    /// The bitmap text's colour (`this`; rgb) -> 0 or an error.
    Phyre_PText_PBitmapTextMaterial_setTextColor_00a2a650 @ 0x00a2_a650 => extern "thiscall" fn t_00a2a650(this: u32, rgb: u32) -> u32 {
        let mut frame = StackFrame::<4>::new();
        sm::bridge::text_color(&mut mem(), this, frame.at(), rgb)
    }
    /// The bitmap text set and its mesh rebuilt (`this`; a).
    Phyre_PText_PBitmapFontText_setText_00a2ad80 @ 0x00a2_ad80 => extern "thiscall" fn t_00a2ad80(this: u32, a: u32) -> u32 {
        sm::bridge::text_set(&mut mem(), this, a)
    }
    /// The Iggy start-up -> 0.
    Iggy_init_00a3e6d0 @ 0x00a3_e6d0 => extern "C" fn t_00a3e6d0() -> u32 {
        sm::bridge::iggy_init(&mut mem())
    }
}
