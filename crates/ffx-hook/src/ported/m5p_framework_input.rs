//! M5 stage 1, list `phyre-framework-input` (`ffx_pother_b::m5p_framework_input*`):
//! `PhyreEngine`'s application framework (`PApplication`, its script proxy
//! and input log), timers, `PhyreOS`, file streams and the input devices,
//! outside M4's coverage. Template copies take their constants from the
//! crate's tables, so each copy runs its own original in the tests
//! (`crates/ffx-oracle/tests/m5p_framework_input.rs`).
//!
//! Threads: the input devices update on the main thread and the async
//! controller thread; the descriptors' destructors run at exit or in
//! unwind funclets. No shim keeps state; each works on the game's memory
//! through [`mem`].

use super::{mem, ported};
use ffx_pother_b::m5p_framework_input_small::{self as sm, bridge as smb};

ported! {
    /// `Phyre_PhyreOS_Sleep(ms)`: `Sleep(ms)`; its `eax`.
    Phyre_PhyreOS_Sleep @ 0x0043_0030 => extern "C" fn fi_00430030(ms: u32) -> u32 {
        smb::sleep(&mut mem(), ms)
    }
    /// `Phyre_PTimerWin32_Terminate`: `timeEndPeriod(5)`; its `eax`.
    Phyre_PTimerWin32_Terminate @ 0x0043_0040 => extern "C" fn fi_00430040() -> u32 {
        smb::timer_terminate(&mut mem())
    }
    /// `Phyre_PMatrix4x3_setVec20_004632f0`: `src[0..3]` to `this + 0x20..0x2c`; `this`.
    Phyre_PMatrix4x3_setVec20_004632f0 @ 0x0046_32f0 => extern "thiscall" fn fi_004632f0(this: u32, src: u32) -> u32 {
        smb::set_vec3_20(&mut mem(), this, src)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PContextVariantFoldingTable_4__dtor @ 0x004d_b900 => extern "thiscall" fn td_004db900(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b900) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShader_4__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShader_4__dtor @ 0x004d_b910 => extern "thiscall" fn td_004db910(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b910) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PRendering_PShaderPassInfo_4__dtor @ 0x004d_b920 => extern "thiscall" fn td_004db920(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b920) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PContextVariantFoldingTable__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PContextVariantFoldingTable__dtor @ 0x004d_b930 => extern "thiscall" fn td_004db930(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b930) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneRenderPass__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PSceneRenderPass__dtor @ 0x004d_b940 => extern "thiscall" fn td_004db940(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b940) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassInfo__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PRendering_PShaderPassInfo__dtor @ 0x004d_b950 => extern "thiscall" fn td_004db950(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x004d_b950) }, this)
    }
    /// `Phyre_PSerialization_PStreamWriter_dtor_PStreamWriter`: the static-object unlink of `this`; its `eax`.
    Phyre_PSerialization_PStreamWriter_dtor_PStreamWriter @ 0x004d_bab0 => extern "thiscall" fn fi_004dbab0(this: u32) -> u32 {
        smb::static_unlink(&mut mem(), this)
    }
    /// `Phyre_PGameplay_PSpline_getSegmentCount_0054b300`: 0 below two points, else `n - 1` open or `n` closed.
    Phyre_PGameplay_PSpline_getSegmentCount_0054b300 @ 0x0054_b300 => extern "thiscall" fn fi_0054b300(this: u32) -> u32 {
        smb::spline_segments(&mem(), this)
    }
    /// `Phyre_PGameplay_PSpline_evaluateToTranslation_0054b520`: the point at `t` to `out + 0x30..0x3c`; the evaluation's `eax`.
    Phyre_PGameplay_PSpline_evaluateToTranslation_0054b520 @ 0x0054_b520 => extern "thiscall" fn fi_0054b520(this: u32, t: u32, out: u32) -> u32 {
        let mut frame = super::StackFrame::<4>::new();
        smb::spline_point_to_30(&mut mem(), this, t, out, frame.at())
    }
    /// `Phyre_PSerialization_PStreamReaderFile_seek_00576a80`: the file's seek on `this + 0x14` (tail jump); its `eax`.
    Phyre_PSerialization_PStreamReaderFile_seek_00576a80 @ 0x0057_6a80 => extern "thiscall" fn fi_00576a80(this: u32, offset: u32) -> u32 {
        smb::reader_file_seek(&mut mem(), this, offset)
    }
    /// `Phyre_PSerialization_PStreamReader_deletingDtor_0057dc80`: its vtable, `PStream::~PStream`, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PSerialization_PStreamReader_deletingDtor_0057dc80 @ 0x0057_dc80 => extern "thiscall" fn dd_0057dc80(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x0057_dc80) }, this, flags)
    }
    /// `Phyre_PSerialization_freeBlockChain_0057dcb0`: `PhyreFree` of each block of `*head`, `*head` = 0.
    Phyre_PSerialization_freeBlockChain_0057dcb0 @ 0x0057_dcb0 => extern "C" fn fi_0057dcb0(head: u32) {
        smb::free_chain(&mut mem(), head);
    }
    /// `Phyre_PSerialization_PStreamReader_read_0057e160`: 0 for `n == 0`, else vtable `+0x18` (`n`) added to `+0xc`; its `eax`.
    Phyre_PSerialization_PStreamReader_read_0057e160 @ 0x0057_e160 => extern "thiscall" fn fi_0057e160(this: u32, n: u32) -> u32 {
        smb::read_counted(&mut mem(), this, n)
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineBase_dtor_0060ec70`: `*this` = the base vtable.
    Phyre_PProfiling_PProcessorProfileTimelineBase_dtor_0060ec70 @ 0x0060_ec70 => extern "thiscall" fn fi_0060ec70(this: u32) {
        smb::set_vtable(&mut mem(), this, sm::TIMELINE_BASE_VTABLE);
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineCollection_removeTimeline`: 0.
    Phyre_PProfiling_PProcessorProfileTimelineCollection_removeTimeline @ 0x0060_ede0 => extern "thiscall" fn kc_0060ede0(_this: u32, _a: u32) -> u32 {
        const { sm::constant_of(0x0060_ede0) }
    }
    /// `Phyre_PProfiling_PProcessorProfileTimelineCpu_slot1_0060ee10`: 0.
    Phyre_PProfiling_PProcessorProfileTimelineCpu_slot1_0060ee10 @ 0x0060_ee10 => extern "thiscall" fn kc_0060ee10(_this: u32, _a: u32, _b: u32, _c: u32, _d: u32) -> u32 {
        const { sm::constant_of(0x0060_ee10) }
    }
    /// `Phyre_PArray_release_00620900`: `PhyreFree` of owned elements, both words zero.
    Phyre_PArray_release_00620900 @ 0x0062_0900 => extern "thiscall" fn pr_00620900(this: u32) -> u32 {
        smb::release(&mut mem(), this)
    }
    /// `Phyre_PArray_release_00620930`: as `Phyre_PArray_release_00620900`.
    Phyre_PArray_release_00620930 @ 0x0062_0930 => extern "thiscall" fn pr_00620930(this: u32) -> u32 {
        smb::release(&mut mem(), this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PFramework_PApplication__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PFramework_PApplication__dtor @ 0x0062_0a40 => extern "thiscall" fn td_00620a40(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x0062_0a40) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PFramework_PApplicationViewport__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PFramework_PApplicationViewport__dtor @ 0x0062_0a50 => extern "thiscall" fn td_00620a50(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x0062_0a50) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PFramework_PInputMapper__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PFramework_PInputMapper__dtor @ 0x0062_0a60 => extern "thiscall" fn td_00620a60(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x0062_0a60) }, this)
    }
    /// `Phyre_PFramework_PInputDevice_dtor_00620ed0`: `*this` = `PInputDevice`'s vtable.
    Phyre_PFramework_PInputDevice_dtor_00620ed0 @ 0x0062_0ed0 => extern "thiscall" fn fi_00620ed0(this: u32) {
        smb::set_vtable(&mut mem(), this, sm::INPUT_DEVICE_VTABLE);
    }
    /// `Phyre_PFramework_PApplication_deletingDtor_00621a00`: `~PApplication`, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PApplication_deletingDtor_00621a00 @ 0x0062_1a00 => extern "thiscall" fn dd_00621a00(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x0062_1a00) }, this, flags)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_deletingDtor_00621ad0`: its destructor, `operator delete` when `flags & 1`; `this`.
    Phyre_PFramework_PApplicationScriptBuffer_deletingDtor_00621ad0 @ 0x0062_1ad0 => extern "thiscall" fn dd_00621ad0(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x0062_1ad0) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDevice_deletingDtor_00621bd0`: its vtable, `operator delete` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDevice_deletingDtor_00621bd0 @ 0x0062_1bd0 => extern "thiscall" fn dd_00621bd0(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x0062_1bd0) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDeviceMotion_deletingDtor_00621c80`: `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDeviceMotion_deletingDtor_00621c80 @ 0x0062_1c80 => extern "thiscall" fn dd_00621c80(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x0062_1c80) }, this, flags)
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_Echo(state, text)`: prints `text` at level 2; 0.
    Phyre_PFramework_PApplicationScriptProxy_Echo @ 0x0062_3310 => extern "C" fn fi_00623310(_state: u32, text: u32) -> u32 {
        smb::echo(&mut mem(), text)
    }
    /// `Phyre_PFramework_PInput_setPollFlag_006250d0(flag)`: the byte to `0x00cc9cf4`.
    Phyre_PFramework_PInput_setPollFlag_006250d0 @ 0x0062_50d0 => extern "C" fn fi_006250d0(flag: u8) {
        smb::set_poll_flag(&mut mem(), flag);
    }
    /// `Phyre_PFramework_PInputDeviceMotion_slot2_00625d00`: `0x17`.
    Phyre_PFramework_PInputDeviceMotion_slot2_00625d00 @ 0x0062_5d00 => extern "thiscall" fn kc_00625d00(_this: u32, _a: u32, _b: u32) -> u32 {
        const { sm::constant_of(0x0062_5d00) }
    }
    /// `Phyre_PFramework_PInputDeviceTouch_slot2_00625ed0`: `0x17`.
    Phyre_PFramework_PInputDeviceTouch_slot2_00625ed0 @ 0x0062_5ed0 => extern "thiscall" fn kc_00625ed0(_this: u32, _a: u32, _b: u32) -> u32 {
        const { sm::constant_of(0x0062_5ed0) }
    }
    /// `Phyre_PFramework_PWindowWin32Base_restoreDisplay`: the desktop mode back when full screen, `ReleaseDC`; its `eax`.
    Phyre_PFramework_PWindowWin32Base_restoreDisplay @ 0x0062_70a0 => extern "thiscall" fn fi_006270a0(this: u32) -> u32 {
        smb::restore_display(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_findNewline_006277b0`: the next `'\n'` before the end, or 0.
    Phyre_PFramework_PApplicationScriptBuffer_findNewline_006277b0 @ 0x0062_77b0 => extern "thiscall" fn fi_006277b0(this: u32) -> u32 {
        smb::find_newline(&mem(), this)
    }
    /// `Phyre_PFramework_PInputDeviceMotion_getType_00628200`: 4.
    Phyre_PFramework_PInputDeviceMotion_getType_00628200 @ 0x0062_8200 => extern "thiscall" fn kc_00628200(_this: u32) -> u32 {
        const { sm::constant_of(0x0062_8200) }
    }
    /// `Phyre_PFramework_PInputDevicePad_getType_00628220`: 2.
    Phyre_PFramework_PInputDevicePad_getType_00628220 @ 0x0062_8220 => extern "thiscall" fn kc_00628220(_this: u32) -> u32 {
        const { sm::constant_of(0x0062_8220) }
    }
    /// `Phyre_PFramework_PInputDeviceTouch_getType_00628230`: 3.
    Phyre_PFramework_PInputDeviceTouch_getType_00628230 @ 0x0062_8230 => extern "thiscall" fn kc_00628230(_this: u32) -> u32 {
        const { sm::constant_of(0x0062_8230) }
    }
    /// `Phyre_PFramework_PInputDeviceMotion_getButton_006289c0`: `al` = 0.
    Phyre_PFramework_PInputDeviceMotion_getButton_006289c0 @ 0x0062_89c0 => extern "thiscall" fn fi_006289c0(_this: u32, _id: u32) -> u8 {
        0
    }
    /// `PInputDevicePad_getButton(id)`: the button byte in `al` (ids `0xb..=0x1a`), the id's upper bytes above it.
    PInputDevicePad_getButton @ 0x0062_89f0 => extern "thiscall" fn fi_006289f0(this: u32, id: u32) -> u32 {
        smb::pad_button(&mem(), this, id)
    }
    /// `Phyre_PFramework_PInputDeviceTouch_getButton_00628a10`: `al` = 0.
    Phyre_PFramework_PInputDeviceTouch_getButton_00628a10 @ 0x0062_8a10 => extern "thiscall" fn fi_00628a10(_this: u32, _id: u32) -> u8 {
        0
    }
    /// `Phyre_PFramework_PInputDeviceKeyboard_getAxis_00628a20`: 0 in `ST0`.
    Phyre_PFramework_PInputDeviceKeyboard_getAxis_00628a20 @ 0x0062_8a20 => extern "thiscall" fn fi_00628a20(_this: u32, _id: u32) -> f32 {
        0.0
    }
    /// `Phyre_PFramework_PInputDeviceMotion_getAxis_00628a30`: 0 in `ST0`.
    Phyre_PFramework_PInputDeviceMotion_getAxis_00628a30 @ 0x0062_8a30 => extern "thiscall" fn fi_00628a30(_this: u32, _id: u32) -> f32 {
        0.0
    }
    /// `Phyre_PFramework_PInputDeviceTouch_getAxis_00628cb0`: 0 in `ST0`.
    Phyre_PFramework_PInputDeviceTouch_getAxis_00628cb0 @ 0x0062_8cb0 => extern "thiscall" fn fi_00628cb0(_this: u32, _id: u32) -> f32 {
        0.0
    }
    /// `Phyre_PFramework_PApplication_getViewport_00629750(i)`: word `i` of the table at `+0x2c0`.
    Phyre_PFramework_PApplication_getViewport_00629750 @ 0x0062_9750 => extern "thiscall" fn fi_00629750(this: u32, i: u32) -> u32 {
        smb::viewport_at(&mem(), this, i)
    }
    /// `PApplication_getWindowDisplayInfo`: `+0x264`.
    PApplication_getWindowDisplayInfo @ 0x0062_97a0 => extern "thiscall" fn fi_006297a0(this: u32) -> u32 {
        smb::window_display_info(&mem(), this)
    }
    /// `Phyre_PFramework_PApplication_slot2_00629810`: 0.
    Phyre_PFramework_PApplication_slot2_00629810 @ 0x0062_9810 => extern "thiscall" fn kc_00629810(_this: u32, _a: u32, _b: u32) -> u32 {
        const { sm::constant_of(0x0062_9810) }
    }
    /// `Phyre_PFramework_PInputDevicePad_setVibration(low, high)`: the two bytes, then the changed flag.
    Phyre_PFramework_PInputDevicePad_setVibration @ 0x0062_cb70 => extern "thiscall" fn fi_0062cb70(this: u32, low: u8, high: u8) {
        smb::set_vibration(&mut mem(), this, low, high);
    }
    /// `Phyre_PInputs_PInputSource_deletingDtor_009e3de0`: its vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PInputs_PInputSource_deletingDtor_009e3de0 @ 0x009e_3de0 => extern "thiscall" fn dd_009e3de0(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x009e_3de0) }, this, flags)
    }
    /// `Phyre_PInputs_PInputSourceJoypadAxis_update_009f0330`: `+0x18` = the map's value (0 without a map).
    Phyre_PInputs_PInputSourceJoypadAxis_update_009f0330 @ 0x009f_0330 => extern "thiscall" fn fi_009f0330(this: u32) {
        smb::axis_refresh(&mut mem(), this);
    }
    /// `Phyre_PSerialization_PStreamReader_PStreamReader(name)`: the stream constructor, its vtable, `+0xc` = 0; `this`.
    Phyre_PSerialization_PStreamReader_PStreamReader @ 0x009f_0630 => extern "thiscall" fn fi_009f0630(this: u32, name: u32) -> u32 {
        smb::stream_reader_ctor(&mut mem(), this, name)
    }
    /// `Phyre_PSerialization_PStreamWriter_dtor_009f0660`: its vtable, then `PStream::~PStream` (tail jump); its `eax`.
    Phyre_PSerialization_PStreamWriter_dtor_009f0660 @ 0x009f_0660 => extern "thiscall" fn td_009f0660(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x009f_0660) }, this)
    }
    /// `Phyre_PSerialization_PStreamWriter_deletingDtor_009f0670`: its vtable, `PStream::~PStream`, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PSerialization_PStreamWriter_deletingDtor_009f0670 @ 0x009f_0670 => extern "thiscall" fn dd_009f0670(this: u32, flags: u32) -> u32 {
        smb::deleting_dtor(&mut mem(), const { sm::delete_of(0x009f_0670) }, this, flags)
    }
    /// `Phyre_PPostProcessing_PPostEffectManager_getEntryPlus4_00a16500(i)`: element `i` plus 4, or 0.
    Phyre_PPostProcessing_PPostEffectManager_getEntryPlus4_00a16500 @ 0x00a1_6500 => extern "thiscall" fn fi_00a16500(this: u32, i: u32) -> u32 {
        smb::element_plus4(&mem(), this, i)
    }
    /// `Phyre_PPostProcessing_PPostEffectManager_getEntryWord4_00a16530(i)`: the word at `+4` of element `i`, or 0.
    Phyre_PPostProcessing_PPostEffectManager_getEntryWord4_00a16530 @ 0x00a1_6530 => extern "thiscall" fn fi_00a16530(this: u32, i: u32) -> u32 {
        smb::element_word4(&mem(), this, i)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_slot3_00a1b980(a, b)`: 0 unless enabled, else the check's `eax`.
    Phyre_PPostProcessing_PMotionBlur_slot3_00a1b980 @ 0x00a1_b980 => extern "thiscall" fn fi_00a1b980(this: u32, a: u32, b: u32) -> u32 {
        smb::motion_blur_check(&mut mem(), this, a, b)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputMap_4__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PArray_Phyre_PInputs_PInputMap_4__dtor @ 0x00a3_3180 => extern "thiscall" fn td_00a33180(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x00a3_3180) }, this)
    }
    /// `Phyre_PClassDescriptorForType_Phyre_PGame_PGameSettings__dtor`: its vtable, then `PClassDescriptor::Destruct` (tail jump); its `eax`.
    Phyre_PClassDescriptorForType_Phyre_PGame_PGameSettings__dtor @ 0x00a3_3190 => extern "thiscall" fn td_00a33190(this: u32) -> u32 {
        smb::tail_dtor(&mut mem(), const { sm::tail_of(0x00a3_3190) }, this)
    }
}
