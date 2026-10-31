//! M5 stage 1, the leftovers of list `phyre-framework-input` (round left2)
//! (`ffx_pother_b::m5p_framework_input_left2`). The parity tests are in
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`left2_*`).
//!
//! Threads: the screen-mode switch runs on the window's (main) thread, the
//! exception filter on whichever thread crashed; the frames the originals
//! hand to Win32 are per-call [`StackFrame`]s.

use ffx_pother_b::m5p_framework_input_left2::{self as lf, bridge as lb};

use super::{StackFrame, mem, ported};

ported! {
    /// `0x0062a0d0(out, v)`: `v` normalised.
    Phyre_Vector2_normalize_0062a0d0 @ 0x0062_a0d0 => extern "C" fn normalize2(out: u32, v: u32) -> u32 {
        lb::normalize2(&mut mem(), out, v)
    }
    /// `CrashDump_UnhandledExceptionFilter(pointers)`.
    CrashDump_UnhandledExceptionFilter @ 0x0062_68e0 => extern "stdcall" fn crash_filter(pointers: u32) -> u32 {
        let mut f = StackFrame::<3>::new();
        lb::crash_filter(&mut mem(), pointers, f.at())
    }
    /// `PWindowWin32Base::applyScreenMode()`.
    Phyre_PFramework_PWindowWin32Base_applyScreenMode @ 0x0062_bf00 => extern "C" fn apply_screen_mode() {
        let mut f = StackFrame::<{ lf::SCREEN_FRAME_WORDS as usize }>::new();
        lb::apply_screen_mode(&mut mem(), f.at());
    }
    /// The jump to `applyScreenMode` at `0x0062bef0`.
    Phyre_PFramework_PWindowWin32Base_applyScreenMode_jump_0062bef0 @ 0x0062_bef0 => extern "C" fn apply_screen_mode_jump() {
        let mut f = StackFrame::<{ lf::SCREEN_FRAME_WORDS as usize }>::new();
        lb::apply_screen_mode(&mut mem(), f.at());
    }
    /// `PStream::readAll(array, terminate)`.
    Phyre_PSerialization_PStream_readAll_0057dd60 @ 0x0057_dd60 => extern "thiscall" fn read_all(stream: u32, array: u32, terminate: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::read_all(&mut mem(), stream, array, terminate, f.at())
    }
    /// The one-value file read `(path)`.
    Phyre_PFramework_readValueFile_006285b0 @ 0x0062_85b0 => extern "C" fn read_value_file(path: u32) -> u32 {
        let mut f = StackFrame::<{ lf::VALUE_FRAME_WORDS as usize }>::new();
        lb::read_value_file(&mut mem(), path, f.at())
    }
    /// `PApplicationScriptProxy::buttonPress(app, name)`.
    PApplicationScriptProxy_buttonPress @ 0x0062_2d40 => extern "C" fn button_press(app: u32, name: u32) -> u32 {
        lb::button_press(&mut mem(), app, name)
    }
    /// The script's pad button set `(app, name, value)`.
    Phyre_PFramework_PApplicationScriptProxy_buttonSet_0062c730 @ 0x0062_c730 => extern "C" fn button_set(app: u32, name: u32, value: u32) -> u32 {
        lb::button_set(&mut mem(), app, name, value)
    }
    /// The input map's release on exit.
    Phyre_PFramework_PInputMapper_release_0062d170 @ 0x0062_d170 => extern "thiscall" fn release_input_map(this: u32) {
        lb::release_input_map(&mut mem(), this);
    }
    /// `PInputDeviceTouch::update()`.
    Phyre_PFramework_PInputDeviceTouch_update @ 0x0062_de50 => extern "thiscall" fn touch_update(this: u32) -> u32 {
        lb::touch_update(&mut mem(), this)
    }
    /// `PMeshParticleSystemBase`'s constructor.
    Phyre_PPostProcessing_PMeshParticleSystemBase_ctor_009fc9e0 @ 0x009f_c9e0 => extern "thiscall" fn particle_system_ctor(this: u32) -> u32 {
        lb::particle_system_ctor(&mut mem(), this)
    }
    /// `PInputDevicePadDirectInput::update()`.
    PInputDevicePadDirectInput_update @ 0x0062_d9d0 => extern "thiscall" fn dinput_pad_update(this: u32) {
        let mut f = StackFrame::<{ lf::DIJOYSTATE2_WORDS as usize }>::new();
        lb::dinput_pad_update(&mut mem(), this, f.at());
    }
    /// `PInputDevicePadXInput::update()`.
    PadXInput_Update @ 0x0062_dba0 => extern "thiscall" fn xinput_pad_update(this: u32) {
        let mut f = StackFrame::<{ lf::XINPUT_FRAME_WORDS as usize }>::new();
        lb::xinput_pad_update(&mut mem(), this, f.at());
    }
    /// `PMotionBlurBase::initializeCommonShaders(cluster, effect)` (the
    /// generic two-sampler port in `ffx_prender::m5p_render_b_left2`).
    Phyre_PPostProcessing_PMotionBlurBase_initializeCommonShaders @ 0x00a1_8c80 => extern "thiscall" fn init_motion_blur_shaders(this: u32, cluster: u32, effect: u32) -> u32 {
        use ffx_prender::m5p_render_b_left2 as rf;
        let mut f = StackFrame::<{ rf::SAMPLER_FRAME_WORDS as usize }>::new();
        rf::bridge::init_sampler_shaders(&mut mem(), &const { rf::sampler_shaders_of(0x00a1_8c80) }, this, cluster, effect, f.at())
    }
    /// A gameplay path's point `(out, index)`.
    Phyre_PGameplay_PPath_getPoint_0054b080 @ 0x0054_b080 => extern "thiscall" fn path_point(this: u32, out: u32, index: u32) -> u32 {
        lb::path_point(&mut mem(), this, out, index)
    }
    /// A gameplay path's rotation `(out, t)`.
    Phyre_PGameplay_PPath_getRotation_0054b550 @ 0x0054_b550 => extern "thiscall" fn path_rotation(this: u32, out: u32, t: u32) -> u32 {
        let mut f = StackFrame::<{ lf::PATH_ROTATION_WORDS as usize }>::new();
        lb::path_rotation(&mut mem(), this, out, t, f.at())
    }
    /// A gameplay path's transform `(out, t)`.
    Phyre_PGameplay_PPath_getTransform_0054b690 @ 0x0054_b690 => extern "thiscall" fn path_transform(this: u32, out: u32, t: u32) -> u32 {
        let mut f = StackFrame::<{ lf::PATH_TRANSFORM_WORDS as usize }>::new();
        lb::path_transform(&mut mem(), this, out, t, f.at())
    }
    /// A gameplay path's frame `(t, out)`.
    Phyre_PGameplay_PPath_getFrame_0054b320 @ 0x0054_b320 => extern "thiscall" fn path_frame(this: u32, t: u32, out: u32) -> u32 {
        let mut f = StackFrame::<{ lf::PATH_FRAME_WORDS as usize }>::new();
        lb::path_frame(&mut mem(), this, t, out, f.at())
    }
    /// The application map's `erase(&out, where)`: the iterator advanced
    /// in a frame word (the original advances its own argument slot,
    /// which its callers never read: `ret 8`).
    Phyre_PFramework_StdMap_erase_006271d0 @ 0x0062_71d0 => extern "thiscall" fn map_erase(this: u32, out: u32, at: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        let slot = f.at();
        let mut m = mem();
        ffx_bridge::Mem::write(&mut m, slot, at);
        lb::map_erase(&mut m, this, out, slot)
    }
    /// `PApplication::logKeyboardInput()`.
    Phyre_PFramework_PApplication_logKeyboardInput @ 0x0062_9e40 => extern "thiscall" fn log_keyboard(this: u32) -> u32 {
        lb::log_keyboard(&mut mem(), this)
    }
    /// `PApplication::onExit()`; the render scope lock in a frame word.
    Phyre_PFramework_PApplication_onExit @ 0x0062_a130 => extern "thiscall" fn on_exit(this: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::on_exit(&mut mem(), this, f.at())
    }
    /// `PApplication::parseCommandLine(argc, argv, config)`; the
    /// resolution lookup's default in a frame word.
    Phyre_PFramework_PApplication_parseCommandLine @ 0x0062_adc0 => extern "thiscall" fn parse_command_line(this: u32, argc: u32, argv: u32, config: u32) -> u32 {
        let mut f = StackFrame::<1>::new();
        lb::parse_command_line(&mut mem(), this, argc, argv, config, f.at())
    }
}
