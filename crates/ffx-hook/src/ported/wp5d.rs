//! M5 finish, WP5 part 3 (wp5d, `ffx_prender::wp5d`): the `FFX.exe`
//! frontier of the no-Unicorn boot. Parity tests:
//! `crates/ffx-oracle/tests/wp5d.rs`.
//!
//! Threads: any (the boot's main thread). No shim keeps state.

use ffx_prender::wp5d as d;

use super::{mem, ported};

ported! {
    /// `PPhysicsCharacterCamera_ctor_005dff30`: the camera's defaults; `this`.
    PPhysicsCharacterCamera_ctor_005dff30 @ 0x005d_ff30 => extern "thiscall" fn d_005dff30(this: u32) -> u32 {
        d::character_camera_ctor(&mut mem(), this)
    }
    /// `PPhysicsCharacterCamera_dtor_005e20c0`: the vtable back (`eax` the caller's).
    PPhysicsCharacterCamera_dtor_005e20c0 @ 0x005e_20c0 => extern "thiscall" fn d_005e20c0(this: u32) {
        d::character_camera_dtor(&mut mem(), this);
    }
    /// `DebugTweakUtil_CreateInstance_006b5cb0`: created once.
    DebugTweakUtil_CreateInstance_006b5cb0 @ 0x006b_5cb0 => extern "C" fn d_006b5cb0() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[0];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `DebugTextPanel_CreateInstance_006b5d30`: created once.
    DebugTextPanel_CreateInstance_006b5d30 @ 0x006b_5d30 => extern "C" fn d_006b5d30() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[1];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `DebugTextTweakUtil_CreateInstance_006b5db0`: created once.
    DebugTextTweakUtil_CreateInstance_006b5db0 @ 0x006b_5db0 => extern "C" fn d_006b5db0() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[2];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `TweakUtil_CreateInstance_006b5e30`: created once.
    TweakUtil_CreateInstance_006b5e30 @ 0x006b_5e30 => extern "C" fn d_006b5e30() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[3];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `TweakUtil_CreateInstance_006b5f30`: created once.
    TweakUtil_CreateInstance_006b5f30 @ 0x006b_5f30 => extern "C" fn d_006b5f30() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[4];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `TweakUtil_CreateInstance_006b5fc0`: created once.
    TweakUtil_CreateInstance_006b5fc0 @ 0x006b_5fc0 => extern "C" fn d_006b5fc0() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[5];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `TweakUtil_CreateInstance_006b6040`: created once.
    TweakUtil_CreateInstance_006b6040 @ 0x006b_6040 => extern "C" fn d_006b6040() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[6];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `ViewerManager_CreateInstance_006b60c0`: created once (`eax` when it exists: the instance).
    ViewerManager_CreateInstance_006b60c0 @ 0x006b_60c0 => extern "C" fn d_006b60c0() -> u32 {
        let (_, g, n, c) = d::CREATE_ONCE[7];
        d::create_once(&mut mem(), g, n, c)
    }
    /// `TweakUtil_ctor_006b5520`: a debug tool's constructor; `this`.
    TweakUtil_ctor_006b5520 @ 0x006b_5520 => extern "thiscall" fn d_006b5520(this: u32) -> u32 {
        d::tweak_ctor_6b5520(&mut mem(), this)
    }
    /// `TweakUtil_ctor_006b5670`: a debug tool's constructor; `this`.
    TweakUtil_ctor_006b5670 @ 0x006b_5670 => extern "thiscall" fn d_006b5670(this: u32) -> u32 {
        d::tweak_ctor_6b5670(&mut mem(), this)
    }
    /// `TweakUtil_ctor_006b5730`: a debug tool's constructor; `this`.
    TweakUtil_ctor_006b5730 @ 0x006b_5730 => extern "thiscall" fn d_006b5730(this: u32) -> u32 {
        d::tweak_ctor_6b5730(&mut mem(), this)
    }
    /// `TweakUtil_ctor_006b57d0`: a debug tool's constructor; `this`.
    TweakUtil_ctor_006b57d0 @ 0x006b_57d0 => extern "thiscall" fn d_006b57d0(this: u32) -> u32 {
        d::tweak_ctor_6b57d0(&mut mem(), this)
    }
    /// `ViewerManager_ctor_006b5870`: the viewer manager's constructor; `this`.
    ViewerManager_ctor_006b5870 @ 0x006b_5870 => extern "thiscall" fn d_006b5870(this: u32) -> u32 {
        d::viewer_manager_ctor(&mut mem(), this)
    }
}
