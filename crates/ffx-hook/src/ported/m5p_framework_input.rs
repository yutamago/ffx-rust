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
use ffx_pother_b::m5p_framework_input_app::bridge as ab;
use ffx_pother_b::m5p_framework_input_devices::{self as dv, bridge as db};
use ffx_pother_b::m5p_framework_input_life::bridge as lb;
use ffx_pother_b::m5p_framework_input_misc::bridge as mb;
use ffx_pother_b::m5p_framework_input_script::bridge as sb6;
use ffx_pother_b::m5p_framework_input_small::{self as sm, bridge as smb};
use ffx_pother_b::m5p_framework_input_sources::{self as so, bridge as sb};
use ffx_pother_b::m5p_framework_input_streams::bridge as tb;

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
    /// `Phyre_PSerialization_PStreamReaderFile_deletingDtor_00576820`: its vtable, both destructors, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PSerialization_PStreamReaderFile_deletingDtor_00576820 @ 0x0057_6820 => extern "thiscall" fn dd_00576820(this: u32, flags: u32) -> u32 {
        tb::reader_file_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PSerialization_PStreamReaderFile_open_00576990`: opens `path`, keeps its name and size; 1 when the open returned 0.
    Phyre_PSerialization_PStreamReaderFile_open_00576990 @ 0x0057_6990 => extern "thiscall" fn fi_00576990(this: u32, path: u32) -> u32 {
        tb::reader_open(&mut mem(), this, path)
    }
    /// `Phyre_PSerialization_PStreamReaderFile_reset_00576a10`: seeks back to the start when open; `al` the answer.
    Phyre_PSerialization_PStreamReaderFile_reset_00576a10 @ 0x0057_6a10 => extern "thiscall" fn fi_00576a10(this: u32) -> u32 {
        tb::reader_reset(&mut mem(), this)
    }
    /// `Phyre_PSerialization_PStreamFileWin32_seek_00608120`: a relative seek through the archive entry or `SetFilePointer`; `offset` or 0.
    Phyre_PSerialization_PStreamFileWin32_seek_00608120 @ 0x0060_8120 => extern "thiscall" fn fi_00608120(this: u32, offset: u32) -> u32 {
        tb::file_seek(&mut mem(), this, offset)
    }
    /// `Phyre_PSerialization_PStreamFileWin32_write_00608160`: `WriteFile` on a loose file (1 when it returns 1); `al` = 0 for an archive entry.
    Phyre_PSerialization_PStreamFileWin32_write_00608160 @ 0x0060_8160 => extern "thiscall" fn fi_00608160(this: u32, data: u32, size: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let written = frame.at();
        tb::file_write(&mut mem(), this, data, size, written).unwrap_or(0)
    }
    /// `Phyre_PSerialization_PStream_deletingDtor_0060f100`: its vtable, the name freed unless borrowed, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PSerialization_PStream_deletingDtor_0060f100 @ 0x0060_f100 => extern "thiscall" fn dd_0060f100(this: u32, flags: u32) -> u32 {
        tb::stream_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PInputs_PInputSourceJoypadAxis_ctor_009e1420`: the source's fields, `+0x18` from the map; `this`.
    Phyre_PInputs_PInputSourceJoypadAxis_ctor_009e1420 @ 0x009e_1420 => extern "thiscall" fn ct_009e1420(this: u32, map: u32, id: u32, flag: u32, scale: u32) -> u32 {
        sb::joypad_axis_ctor(&mut mem(), this, [map, id, flag, scale])
    }
    /// `Phyre_PInputs_PInputSourceMouseButton_ctor_009e1910`: the source's fields, then the mouse-device rule; `this`.
    Phyre_PInputs_PInputSourceMouseButton_ctor_009e1910 @ 0x009e_1910 => extern "thiscall" fn ct_009e1910(this: u32, map: u32, id: u32, flag: u32, scale: u32) -> u32 {
        sb::mouse_button_ctor(&mut mem(), this, [map, id, flag, scale])
    }
    /// `Phyre_PInputs_PInputSourceJoypadAxis_clone`: a copy through the class's constructor, or 0.
    Phyre_PInputs_PInputSourceJoypadAxis_clone @ 0x009e_b7a0 => extern "thiscall" fn cl_009eb7a0(this: u32) -> u32 {
        sb::ctor_clone(&mut mem(), const { so::ctor_of(0x009e_b7a0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceJoypadButton_clone`: a copy with both keys and its semantic check, or 0.
    Phyre_PInputs_PInputSourceJoypadButton_clone @ 0x009e_b820 => extern "thiscall" fn cl_009eb820(this: u32) -> u32 {
        sb::pair_clone(&mut mem(), const { so::clone_of(so::PAIR_CLONES, 0x009e_b820) }, this)
    }
    /// `Phyre_PInputs_PInputSourceKey_clone`: a copy with both keys and its semantic check, or 0.
    Phyre_PInputs_PInputSourceKey_clone @ 0x009e_b8d0 => extern "thiscall" fn cl_009eb8d0(this: u32) -> u32 {
        sb::pair_clone(&mut mem(), const { so::clone_of(so::PAIR_CLONES, 0x009e_b8d0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityX_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionAngularVelocityX_clone @ 0x009e_b980 => extern "thiscall" fn cl_009eb980(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_b980) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityY_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionAngularVelocityY_clone @ 0x009e_b9d0 => extern "thiscall" fn cl_009eb9d0(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_b9d0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionAngularVelocityZ_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionAngularVelocityZ_clone @ 0x009e_ba20 => extern "thiscall" fn cl_009eba20(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_ba20) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationX_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationX_clone @ 0x009e_ba70 => extern "thiscall" fn cl_009eba70(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_ba70) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationY_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationY_clone @ 0x009e_bac0 => extern "thiscall" fn cl_009ebac0(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bac0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionLinearAccelerationZ_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionLinearAccelerationZ_clone @ 0x009e_bb10 => extern "thiscall" fn cl_009ebb10(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bb10) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatW_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionQuatW_clone @ 0x009e_bb60 => extern "thiscall" fn cl_009ebb60(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bb60) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatX_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionQuatX_clone @ 0x009e_bbb0 => extern "thiscall" fn cl_009ebbb0(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bbb0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatY_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionQuatY_clone @ 0x009e_bc00 => extern "thiscall" fn cl_009ebc00(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bc00) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMotionQuatZ_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceMotionQuatZ_clone @ 0x009e_bc50 => extern "thiscall" fn cl_009ebc50(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bc50) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMouseButton_clone`: a copy through the class's constructor, or 0.
    Phyre_PInputs_PInputSourceMouseButton_clone @ 0x009e_bca0 => extern "thiscall" fn cl_009ebca0(this: u32) -> u32 {
        sb::ctor_clone(&mut mem(), const { so::ctor_of(0x009e_bca0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMouseDeltaX_clone`: a copy with both keys and its semantic check, or 0.
    Phyre_PInputs_PInputSourceMouseDeltaX_clone @ 0x009e_bd20 => extern "thiscall" fn cl_009ebd20(this: u32) -> u32 {
        sb::pair_clone(&mut mem(), const { so::clone_of(so::PAIR_CLONES, 0x009e_bd20) }, this)
    }
    /// `Phyre_PInputs_PInputSourceMouseDeltaY_clone`: a copy with both keys and its semantic check, or 0.
    Phyre_PInputs_PInputSourceMouseDeltaY_clone @ 0x009e_bdd0 => extern "thiscall" fn cl_009ebdd0(this: u32) -> u32 {
        sb::pair_clone(&mut mem(), const { so::clone_of(so::PAIR_CLONES, 0x009e_bdd0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchDragX_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchDragX_clone @ 0x009e_be80 => extern "thiscall" fn cl_009ebe80(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_be80) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchDragY_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchDragY_clone @ 0x009e_bed0 => extern "thiscall" fn cl_009ebed0(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bed0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchPinch_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchPinch_clone @ 0x009e_bf20 => extern "thiscall" fn cl_009ebf20(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bf20) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchRotate_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchRotate_clone @ 0x009e_bf70 => extern "thiscall" fn cl_009ebf70(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bf70) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchTwoFingerDragX_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchTwoFingerDragX_clone @ 0x009e_bfd0 => extern "thiscall" fn cl_009ebfd0(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_bfd0) }, this)
    }
    /// `Phyre_PInputs_PInputSourceTouchTwoFingerDragY_clone`: a copy of the source (`PhyreMalloc(0x18)`), or 0.
    Phyre_PInputs_PInputSourceTouchTwoFingerDragY_clone @ 0x009e_c020 => extern "thiscall" fn cl_009ec020(this: u32) -> u32 {
        sb::plain_clone(&mut mem(), const { so::clone_of(so::PLAIN_CLONES, 0x009e_c020) }, this)
    }
    /// `Phyre_PArray_setStorage_009ef5b0`: `PArray::setStorage(data, count)` (a copy of `Phyre_PArray_setStorage_004f02e0`).
    Phyre_PArray_setStorage_009ef5b0 @ 0x009e_f5b0 => extern "thiscall" fn ss_009ef5b0(this: u32, data: u32, count: u32) -> u32 {
        ffx_prender::array::bridge::set_storage(&mut mem(), this, data, count, None)
    }
    /// `Phyre_PArray_setStorage_009ef600`: `PArray::setStorage(data, count)` (a copy of `Phyre_PArray_setStorage_004f02e0`).
    Phyre_PArray_setStorage_009ef600 @ 0x009e_f600 => extern "thiscall" fn ss_009ef600(this: u32, data: u32, count: u32) -> u32 {
        ffx_prender::array::bridge::set_storage(&mut mem(), this, data, count, None)
    }
    /// `Phyre_PInputs_PInputSourceMouseButton_refresh_009f0430`: a non-mouse device falls back on the default; `+0x18` = its kind, returned.
    Phyre_PInputs_PInputSourceMouseButton_refresh_009f0430 @ 0x009f_0430 => extern "thiscall" fn fi_009f0430(this: u32) -> u32 {
        sb::mouse_button_refresh(&mut mem(), this)
    }
    /// `Phyre_PSerialization_PStreamWriter_write_009f06a0`: the stream's write; 0 and the position advanced, or 9.
    Phyre_PSerialization_PStreamWriter_write_009f06a0 @ 0x009f_06a0 => extern "thiscall" fn fi_009f06a0(this: u32, data: u32, n: u32) -> u32 {
        tb::writer_write(&mut mem(), this, data, n)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_ctor_009f06d0`: the file, the reader base, its vtable; `this`.
    Phyre_PSerialization_PStreamWriterFile_ctor_009f06d0 @ 0x009f_06d0 => extern "thiscall" fn fi_009f06d0(this: u32, path: u32) -> u32 {
        tb::writer_file_ctor(&mut mem(), this, path)
    }
    /// `Phyre_PSerialization_PStreamWriterFile_dtor_009f0740`: its vtable, the writer's and the file's destructors; the writer's `eax`.
    Phyre_PSerialization_PStreamWriterFile_dtor_009f0740 @ 0x009f_0740 => extern "thiscall" fn fi_009f0740(this: u32) -> u32 {
        tb::writer_file_dtor(&mut mem(), this)
    }
    /// `Phyre_PInputs_PUtilityInputs_deletingDtor_009f4590`: its vtable, unlinked from the registry, `operator delete` when `flags & 1`; `this`.
    Phyre_PInputs_PUtilityInputs_deletingDtor_009f4590 @ 0x009f_4590 => extern "thiscall" fn dd_009f4590(this: u32, flags: u32) -> u32 {
        tb::registry_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputMap_4__swapEndiannessStatic1`: builds the static descriptor on first use, then `swapEndiannessOfObject(p, 1)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputMap_4__swapEndiannessStatic1 @ 0x00a3_2c60 => extern "C" fn sl_00a32c60(p: u32) -> u32 {
        ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy(&mut mem(), &const { so::lazy_of(0x00a3_2c60) }, p)
    }
    /// `Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputMap_4__swapEndiannessStatic0`: builds the static descriptor on first use, then `swapEndiannessOfObject(p, 0)`; its `eax`.
    Phyre_PClassDescriptorConcrete_Phyre_PArray_Phyre_PInputs_PInputMap_4__swapEndiannessStatic0 @ 0x00a3_2d10 => extern "C" fn sl_00a32d10(p: u32) -> u32 {
        ffx_pload::m5p_tmpl_fortype::bridge::swap_lazy(&mut mem(), &const { so::lazy_of(0x00a3_2d10) }, p)
    }
    /// `Phyre_PhyreExit`: when the engine is up: the registry entries' `+8`, the timer, the pool, the namespace, the lock; the flag cleared; the lock release's `eax` (an `atexit` entry too, whose dispatcher drops it; 0 when the engine was down, where the original leaves the caller's).
    Phyre_PhyreExit @ 0x0043_0d60 => extern "C" fn ap_00430d60() -> u32 {
        ab::phyre_exit(&mut mem()).unwrap_or(0)
    }
    /// `Phyre_PFramework_PInputDevicePad_dtor_00620ff0`: its vtable, the pad slots destroyed, `PInputDevice`'s vtable.
    Phyre_PFramework_PInputDevicePad_dtor_00620ff0 @ 0x0062_0ff0 => extern "thiscall" fn ap_00620ff0(this: u32) -> u32 {
        ab::pad_dtor(&mut mem(), this)
    }
    /// `Phyre_PFramework_PInputDeviceTouch_dtor_00621130`: its vtable, the critical section released, `PInputDevice`'s vtable.
    Phyre_PFramework_PInputDeviceTouch_dtor_00621130 @ 0x0062_1130 => extern "thiscall" fn ap_00621130(this: u32) -> u32 {
        ab::touch_dtor(&mut mem(), this)
    }
    /// `Phyre_StdMapIterator_increment_006214b0`: `++it`; the iterator.
    Phyre_StdMapIterator_increment_006214b0 @ 0x0062_14b0 => extern "thiscall" fn ap_006214b0(it: u32) -> u32 {
        ab::tree_next(&mut mem(), it)
    }
    /// `Phyre_StdMapIterator_decrement_00621580`: `--it`; the iterator.
    Phyre_StdMapIterator_decrement_00621580 @ 0x0062_1580 => extern "thiscall" fn ap_00621580(it: u32) -> u32 {
        ab::tree_prev(&mut mem(), it)
    }
    /// `Phyre_PFramework_PApplication_getInputPair_00623ba0`: `out` = (application + 4 or 0, the word at `0x00c30fa4`); `out`.
    Phyre_PFramework_PApplication_getInputPair_00623ba0 @ 0x0062_3ba0 => extern "C" fn ap_00623ba0(out: u32) -> u32 {
        ab::app_pair(&mut mem(), out)
    }
    /// `PWindowWin32_AttachToHost`: `SetParent`, then `SendMessageA` of the registered `PHYREMSG_SENDHWND`; nothing without a host.
    PWindowWin32_AttachToHost @ 0x0062_3d50 => extern "C" fn ap_00623d50(host: u32, hwnd: u32) {
        ab::attach_to_host(&mut mem(), host, hwnd);
    }
    /// `PWindowWin32_MouseHookProc`: `WM_NCLBUTTONUP` posted for a left release during a non-client drag; `CallNextHookEx`'s `eax`.
    PWindowWin32_MouseHookProc @ 0x0062_50e0 => extern "stdcall" fn ap_006250e0(code: u32, wparam: u32, lparam: u32) -> u32 {
        ab::mouse_hook(&mut mem(), code, wparam, lparam)
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_setFrameTime_00625720`: the frame time from `text` to `+0x3c`; at or below 0 the clock is rebased; 0.
    Phyre_PFramework_PApplicationScriptProxy_setFrameTime_00625720 @ 0x0062_5720 => extern "C" fn ap_00625720(app: u32, text: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        ab::set_frame_time(&mut mem(), app, text, local)
    }
    /// `PInputDeviceKeyboard_bind`: 5 below id 0x1c, 0x12 when bound; else the mapping reads the key byte and joins the list; 0.
    PInputDeviceKeyboard_bind @ 0x0062_5ca0 => extern "thiscall" fn ap_00625ca0(this: u32, mapping: u32, id: u32) -> u32 {
        ab::keyboard_bind(&mut mem(), this, mapping, id)
    }
    /// `Phyre_PFramework_PApplication_takeKey_00626a00`: takes (reads and clears) a pressed key; `al` the answer.
    Phyre_PFramework_PApplication_takeKey_00626a00 @ 0x0062_6a00 => extern "thiscall" fn ap_00626a00(this: u32, key: u32, device: u32) -> u32 {
        ab::take_key(&mut mem(), this, key, device)
    }
    /// `Phyre_StdMap_clear_00626b90`: erases every node; the head back to empty; the head.
    Phyre_StdMap_clear_00626b90 @ 0x0062_6b90 => extern "thiscall" fn ap_00626b90(this: u32) -> u32 {
        ab::tree_clear(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_findSpace_00627810`: the first blank before the end, or 0.
    Phyre_PFramework_PApplicationScriptBuffer_findSpace_00627810 @ 0x0062_7810 => extern "thiscall" fn ap_00627810(this: u32) -> u32 {
        ab::find_space(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplication_getAxisValue_006282c0`: the mapped axis value of `id`, or 0, in `ST0`.
    Phyre_PFramework_PApplication_getAxisValue_006282c0 @ 0x0062_82c0 => extern "thiscall" fn ap_006282c0(this: u32, id: u32) -> f32 {
        ab::axis_value(&mut mem(), this, id)
    }
    /// `Phyre_PFramework_PApplicationViewport_render`: the renderer's `setViewport` (y flipped against the application's height), or 0x15.
    Phyre_PFramework_PApplicationViewport_render @ 0x0062_be00 => extern "thiscall" fn ap_0062be00(this: u32) -> u32 {
        ab::viewport_render(&mut mem(), this)
    }
    /// `PApplication_setMousePosition_0062bea0`: the input's mouse position, deltas 0.
    PApplication_setMousePosition_0062bea0 @ 0x0062_bea0 => extern "thiscall" fn ap_0062bea0(this: u32, x: u32, y: u32) {
        ab::set_mouse_position(&mut mem(), this, x, y);
    }
    /// `Phyre_PFramework_PApplication_scriptedFrame`: 9 while paused; the first frame's virtual `+0x24`; else the script's next command, its `eax`.
    Phyre_PFramework_PApplication_scriptedFrame @ 0x0062_c6f0 => extern "thiscall" fn ap_0062c6f0(this: u32) -> u32 {
        ab::scripted_frame(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_skipBlank_0062cf90`: skips blanks and `#` lines; the next command, or 0 at the end.
    Phyre_PFramework_PApplicationScriptBuffer_skipBlank_0062cf90 @ 0x0062_cf90 => extern "thiscall" fn ap_0062cf90(this: u32) -> u32 {
        ab::skip_blank(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_initialize_00a1b7e0`: the common shaders for the target, or `+0xc` = `+8` and 0.
    Phyre_PPostProcessing_PMotionBlur_initialize_00a1b7e0 @ 0x00a1_b7e0 => extern "thiscall" fn ap_00a1b7e0(this: u32) -> u32 {
        ab::motion_blur_init(&mut mem(), this)
    }
    /// `Phyre_PArray_releaseE4_00620990`: each element's virtual destructor, the buffer freed; both words zero.
    Phyre_PArray_releaseE4_00620990 @ 0x0062_0990 => extern "thiscall" fn ap_00620990(this: u32) -> u32 {
        db::release_e4(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScript_dtor_00620d00`: its vtable, the text freed, fields cleared, the buffer's destructor; its `eax`.
    Phyre_PFramework_PApplicationScript_dtor_00620d00 @ 0x0062_0d00 => extern "thiscall" fn ap_00620d00(this: u32) -> u32 {
        db::script_dtor(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_dtor_00620d80`: its vtable, the buffer released; the release's `eax`.
    Phyre_PFramework_PApplicationScriptBuffer_dtor_00620d80 @ 0x0062_0d80 => extern "thiscall" fn ap_00620d80(this: u32) -> u32 {
        db::script_buffer_dtor(&mut mem(), this)
    }
    /// `Phyre_PArray_releasePair_006211b0`: the element array at `+8`, then the array at `this`.
    Phyre_PArray_releasePair_006211b0 @ 0x0062_11b0 => extern "thiscall" fn ap_006211b0(this: u32) -> u32 {
        db::release_pair(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplication_releaseArrays_00621230`: the arrays at `+0xa4` and `+0x9c` released.
    Phyre_PFramework_PApplication_releaseArrays_00621230 @ 0x0062_1230 => extern "thiscall" fn ap_00621230(this: u32) -> u32 {
        db::release_a4_9c(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScript_deletingDtor_00621a30`: the script's teardown, `operator delete` when `flags & 1`; `this`.
    Phyre_PFramework_PApplicationScript_deletingDtor_00621a30 @ 0x0062_1a30 => extern "thiscall" fn dd_00621a30(this: u32, flags: u32) -> u32 {
        db::script_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_deletingDtor_00621b00`: the script's teardown, `operator delete` when `flags & 1`; `this`.
    Phyre_PFramework_PApplicationScriptProxy_deletingDtor_00621b00 @ 0x0062_1b00 => extern "thiscall" fn dd_00621b00(this: u32, flags: u32) -> u32 {
        db::script_delete(&mut mem(), this, flags)
    }
    /// `Phyre_PFramework_PInputDeviceKeyboard_deletingDtor_00621c00`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDeviceKeyboard_deletingDtor_00621c00 @ 0x0062_1c00 => extern "thiscall" fn dd_00621c00(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1c00) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDeviceMouse_deletingDtor_00621cb0`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDeviceMouse_deletingDtor_00621cb0 @ 0x0062_1cb0 => extern "thiscall" fn dd_00621cb0(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1cb0) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDevicePad_deletingDtor_00621d50`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDevicePad_deletingDtor_00621d50 @ 0x0062_1d50 => extern "thiscall" fn dd_00621d50(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1d50) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDevicePadDirectInput_deletingDtor_00621dd0`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDevicePadDirectInput_deletingDtor_00621dd0 @ 0x0062_1dd0 => extern "thiscall" fn dd_00621dd0(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1dd0) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDevicePadXInput_deletingDtor_00621e70`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDevicePadXInput_deletingDtor_00621e70 @ 0x0062_1e70 => extern "thiscall" fn dd_00621e70(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1e70) }, this, flags)
    }
    /// `Phyre_PFramework_PInputDeviceTouch_deletingDtor_00621ef0`: its teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`; `this`.
    Phyre_PFramework_PInputDeviceTouch_deletingDtor_00621ef0 @ 0x0062_1ef0 => extern "thiscall" fn dd_00621ef0(this: u32, flags: u32) -> u32 {
        db::device_delete(&mut mem(), const { dv::device_of(0x0062_1ef0) }, this, flags)
    }
    /// `PApplication_NotifySuspend_00627850`: virtual slot 0 of every notifier.
    PApplication_NotifySuspend_00627850 @ 0x0062_7850 => extern "C" fn ap_00627850() {
        db::notify_all(&mut mem(), 0);
    }
    /// `PApplication_NotifyResume_006278b0`: virtual slot 1 of every notifier.
    PApplication_NotifyResume_006278b0 @ 0x0062_78b0 => extern "C" fn ap_006278b0() {
        db::notify_all(&mut mem(), 1);
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_nextToken_00628e40`: the next `delim`-separated token, NUL-terminated, or 0.
    Phyre_PFramework_PApplicationScriptProxy_nextToken_00628e40 @ 0x0062_8e40 => extern "C" fn ap_00628e40(p: u32, end: u32, delim: u8) -> u32 {
        db::next_token(&mut mem(), p, end, delim)
    }
    /// `Phyre_PArray_resize90_0062c1f0`: a new 0x90-byte-record buffer for `n`; 0, or 0xd when the allocation fails.
    Phyre_PArray_resize90_0062c1f0 @ 0x0062_c1f0 => extern "thiscall" fn ap_0062c1f0(this: u32, n: u32) -> u32 {
        db::resize_90(&mut mem(), this, n)
    }
    /// `Phyre_PFramework_PApplication_drainCommands_0062d100`: the lock released, the queue drained, the fields cleared; 0.
    Phyre_PFramework_PApplication_drainCommands_0062d100 @ 0x0062_d100 => extern "thiscall" fn ap_0062d100(this: u32) -> u32 {
        db::drain_commands(&mut mem(), this)
    }
    /// `Phyre_PFramework_PApplicationScript_ctor_0061fac0`: the buffer over a copy of `text`, the owner, its vtable; `this`.
    Phyre_PFramework_PApplicationScript_ctor_0061fac0 @ 0x0061_fac0 => extern "thiscall" fn ms_0061fac0(this: u32, owner: u32, text: u32) -> u32 {
        mb::script_ctor(&mut mem(), this, owner, text)
    }
    /// `Phyre_StdMap_eraseRange_00627130`: the range erased (all at once for `begin..end`); `*out` the iterator after it; `out`.
    Phyre_StdMap_eraseRange_00627130 @ 0x0062_7130 => extern "thiscall" fn ms_00627130(this: u32, out: u32, first: u32, last: u32) -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let local = frame.at();
        mb::erase_range(&mut mem(), this, out, first, last, local)
    }
    /// `PInput_GetPadIndex`: the device's index among the pads (type 2), or -1.
    PInput_GetPadIndex @ 0x0062_8170 => extern "thiscall" fn ms_00628170(this: u32) -> u32 {
        mb::pad_index(&mut mem(), this)
    }
    /// `Phyre_PFramework_PInputMapping_getStrongestValue_00629620`: the strongest source value of `mapping`, or 0, in `ST0`.
    Phyre_PFramework_PInputMapping_getStrongestValue_00629620 @ 0x0062_9620 => extern "thiscall" fn ms_00629620(this: u32, mapping: u32) -> f32 {
        mb::strongest_value(&mut mem(), this, mapping)
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_addCommand_0062bab0`: a `(handler, name)` command record appended, the table grown when full.
    Phyre_PFramework_PApplicationScriptProxy_addCommand_0062bab0 @ 0x0062_bab0 => extern "thiscall" fn ms_0062bab0(this: u32, handler: u32, name: u32) {
        mb::add_command(&mut mem(), this, handler, name);
    }
    /// `Phyre_PFramework_PApplication_syncRenderInterface_0062d3d0`: the render interface synced under the scope lock (built on first use); 0.
    Phyre_PFramework_PApplication_syncRenderInterface_0062d3d0 @ 0x0062_d3d0 => extern "C" fn ms_0062d3d0() -> u32 {
        let mut frame = super::StackFrame::<1>::new();
        let lock = frame.at();
        mb::render_sync(&mut mem(), lock)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_apply_00a21c40`: the blur's set-up and apply; its `eax`, 5 or 0x15.
    Phyre_PPostProcessing_PMotionBlur_apply_00a21c40 @ 0x00a2_1c40 => extern "thiscall" fn ms_00a21c40(this: u32, a: u32, b: u32, target: u32, d: u32, _e: u32) -> u32 {
        mb::motion_blur_apply(&mut mem(), this, [a, b, target, d])
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_PApplicationScriptBuffer`: the script file read into the buffer through a stack `PStreamReaderFile`; `this`.
    Phyre_PFramework_PApplicationScriptBuffer_PApplicationScriptBuffer @ 0x0061_fc20 => extern "thiscall" fn sc_0061fc20(this: u32, path: u32) -> u32 {
        let mut frame = super::StackFrame::<8>::new();
        let reader = frame.at();
        sb6::buffer_ctor(&mut mem(), this, path, reader)
    }
    /// `PInputDevicePad_bind`: a button or axis id bound to the mapping; 0, 5 or 0x12.
    PInputDevicePad_bind @ 0x0062_5dd0 => extern "thiscall" fn sc_00625dd0(this: u32, mapping: u32, id: u32) -> u32 {
        sb6::pad_bind(&mut mem(), this, mapping, id)
    }
    /// `Phyre_PFramework_PApplicationScript_executeNextCommand`: a pending repeat, the repeat count, a known command's handler, or 0x11.
    Phyre_PFramework_PApplicationScript_executeNextCommand @ 0x0062_7480 => extern "thiscall" fn sc_00627480(this: u32) -> u32 {
        let mut frame = super::StackFrame::<2>::new();
        let at = frame.at();
        sb6::execute_next(&mut mem(), this, at)
    }
    /// `Phyre_PFramework_PApplicationScriptBuffer_findNextCommand`: the next `name(args)`; `al` 1 when well formed.
    Phyre_PFramework_PApplicationScriptBuffer_findNextCommand @ 0x0062_76e0 => extern "thiscall" fn sc_006276e0(this: u32, name: u32, args: u32) -> u32 {
        sb6::find_next_command(&mut mem(), this, name, args)
    }
    /// `Phyre_PFramework_PApplicationScriptProxy_registerScriptCommands`: the sixteen commands added in order.
    Phyre_PFramework_PApplicationScriptProxy_registerScriptCommands @ 0x0062_bb60 => extern "thiscall" fn sc_0062bb60(this: u32) {
        sb6::register_commands(&mut mem(), this);
    }
    /// `PInputDevicePad_ctor`: the device lists, the pad's slots and state; `this`.
    PInputDevicePad_ctor @ 0x0062_01a0 => extern "thiscall" fn lf_006201a0(this: u32, device: u32) -> u32 {
        lb::pad_ctor(&mut mem(), this, device)
    }
    /// `Phyre_PFramework_PApplication_dtor_PApplication`: the application's names, world, viewports, renderer surfaces and arrays released; the last release's `eax`.
    Phyre_PFramework_PApplication_dtor_PApplication @ 0x0062_0bc0 => extern "thiscall" fn lf_00620bc0(this: u32) -> u32 {
        lb::application_dtor(&mut mem(), this)
    }
    /// `Phyre_PPostProcessing_PMotionBlur_checkTargets_00a235f0`: both render targets sized to `(w, h)`; 0, 1, or a failing step's `eax`.
    Phyre_PPostProcessing_PMotionBlur_checkTargets_00a235f0 @ 0x00a2_35f0 => extern "thiscall" fn lf_00a235f0(this: u32, w: u32, h: u32) -> u32 {
        lb::check_targets(&mut mem(), this, w, h)
    }
}
