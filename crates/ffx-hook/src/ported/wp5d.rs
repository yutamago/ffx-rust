//! M5 finish, WP5 part 3 (wp5d, `ffx_prender::wp5d`): the `FFX.exe`
//! frontier of the no-Unicorn boot. Parity tests:
//! `crates/ffx-oracle/tests/wp5d.rs`.
//!
//! Threads: any (the boot's main thread). No shim keeps state.

use ffx_prender::wp5d as d;

use super::{StackFrame, mem, ported};

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
    /// `Phyre_PThreadWin32_start_004300b0`: `CreateThread` and the priority; 0 (the debugger's thread name left out).
    Phyre_PThreadWin32_start_004300b0 @ 0x0043_00b0 => extern "thiscall" fn d_004300b0(this: u32, desc: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        d::thread_start(&mut mem(), this, desc, frame.at())
    }
    /// `MemoryPanel_ctor_006dc450`: the memory report's panel; `this`.
    MemoryPanel_ctor_006dc450 @ 0x006d_c450 => extern "thiscall" fn d_006dc450(this: u32) -> u32 {
        d::memory_panel_ctor(&mut mem(), this)
    }
    /// `BacktraceOutput_CreateInstance_00630870`: the crash reporter's output; the mutex.
    BacktraceOutput_CreateInstance_00630870 @ 0x0063_0870 => extern "C" fn d_00630870() -> u32 {
        d::backtrace_create(&mut mem())
    }
    /// `MemoryReport_CreateInstance_006dc800`: the memory report made once.
    MemoryReport_CreateInstance_006dc800 @ 0x006d_c800 => extern "C" fn d_006dc800() -> u32 {
        d::memory_report_create(&mut mem())
    }
    /// `SysThread_create_00679cb0`: `CreateThread` (suspended unless `run` is 1); 0 (the debugger's thread name left out).
    SysThread_create_00679cb0 @ 0x0067_9cb0 => extern "thiscall" fn d_00679cb0(this: u32, stack: u32, run: u32, unused: u32, name: u32) -> u32 {
        let _ = (unused, name);
        d::sys_thread_create(&mut mem(), this, stack, run)
    }
    /// `ClusterManager_ctor_00699ab0`: the cluster manager's constructor; `this`.
    ClusterManager_ctor_00699ab0 @ 0x0069_9ab0 => extern "thiscall" fn d_00699ab0(this: u32) -> u32 {
        d::cluster_manager_ctor(&mut mem(), this)
    }
    /// `ZeroPair_ctor_006c7ca0`: two words 0; `this`.
    ZeroPair_ctor_006c7ca0 @ 0x006c_7ca0 => extern "thiscall" fn d_006c7ca0(this: u32) -> u32 {
        d::zero_pair(&mut mem(), this)
    }
    /// `ZeroPair_ctor_006c7cb0`: the same code; `this`.
    ZeroPair_ctor_006c7cb0 @ 0x006c_7cb0 => extern "thiscall" fn d_006c7cb0(this: u32) -> u32 {
        d::zero_pair(&mut mem(), this)
    }
    /// `StdVector_insertN_00655460`: `count` copies of a word inserted; `out`.
    StdVector_insertN_00655460 @ 0x0065_5460 => extern "thiscall" fn d_00655460(this: u32, out: u32, at: u32, count: u32, value: u32) -> u32 {
        d::vector_insert_n(&mut mem(), this, out, at, count, value)
    }
    /// `StdString_destroyRange_00635c40`: the strings of `[first, last)` emptied.
    StdString_destroyRange_00635c40 @ 0x0063_5c40 => extern "stdcall" fn d_00635c40(first: u32, last: u32) {
        d::destroy_strings(&mut mem(), first, last);
    }
    /// `StdTree_eraseStrings_00635c80`: a string map's subtree freed.
    StdTree_eraseStrings_00635c80 @ 0x0063_5c80 => extern "thiscall" fn d_00635c80(this: u32, node: u32) {
        d::tree_erase_strings(&mut mem(), this, node);
    }
}
