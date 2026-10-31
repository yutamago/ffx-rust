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
}
