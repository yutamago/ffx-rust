//! M5 finish, WP5 part 3 (wp5d, `ffx_prender::wp5d`): the `FFX.exe`
//! frontier of the no-Unicorn boot. Parity tests:
//! `crates/ffx-oracle/tests/wp5d.rs`.
//!
//! Threads: any (the boot's main thread). No shim keeps state.

use ffx_prender::wp5d as d;

use super::{StackFrame, mem, ported};

/// A string map's `_Insert_at`: the overflow path, else
/// `StdTree_insertAt_006993d0`'s port (whose own limit is above this one).
fn string_insert_at(this: u32, out: u32, add_left: u32, at: u32, node: u32, key: u32) -> u32 {
    let mut m = mem();
    let size: u32 = ffx_bridge::Mem::read(&m, this.wrapping_add(4));
    if size >= d::STRING_MAP_MAX {
        d::string_node_overflow(&mut m, node, key);
        return out;
    }
    ffx_system::m5b_stl::bridge::tree_insert_at(&mut m, this, out, add_left, at, node)
}

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
    /// `PCluster_addRenderTarget_0064adf0`: a render target made in the cluster; it or 0.
    PCluster_addRenderTarget_0064adf0 @ 0x0064_adf0 => extern "thiscall" fn d_0064adf0(cluster: u32, a: u32) -> u32 {
        d::cluster_add_render_target(&mut mem(), cluster, a)
    }
    /// `PArray_u8_resize_00444e90`: the string builder's buffer resized; 0 or 0xd.
    PArray_u8_resize_00444e90 @ 0x0044_4e90 => extern "thiscall" fn d_00444e90(this: u32, n: u32) -> u32 {
        d::byte_array_resize(&mut mem(), this, n)
    }
    /// `PArray_u8_resize_thunk_00444e80`: a frame and a jump to `0x00444e90`.
    PArray_u8_resize_thunk_00444e80 @ 0x0044_4e80 => extern "thiscall" fn d_00444e80(this: u32, n: u32) -> u32 {
        d::byte_array_resize(&mut mem(), this, n)
    }
    /// `dlmalloc_init_mparams_006ea050`: `mparams` set once under the global lock; 1.
    dlmalloc_init_mparams_006ea050 @ 0x006e_a050 => extern "C" fn d_006ea050() -> u32 {
        let mut frame = StackFrame::<9>::new();
        d::init_mparams(&mut mem(), frame.at())
    }
    /// `dlmalloc_init_user_mstate_006ea180`: an mspace's state in its base; the state.
    dlmalloc_init_user_mstate_006ea180 @ 0x006e_a180 => extern "C" fn d_006ea180(base: u32, size: u32) -> u32 {
        d::init_user_mstate(&mut mem(), base, size)
    }
    /// `std_Tree_buyNode2c_00654f40`: a 0x2c-byte node hung on the head (`std_Tree_buyNode2c_006a03c0`'s code).
    std_Tree_buyNode2c_00654f40 @ 0x0065_4f40 => extern "thiscall" fn d_00654f40(this: u32) -> u32 {
        ffx_system::wp5_tree::bridge::buy_node(&mut mem(), this, 0x2c)
    }
    /// `std_Tree_buyNode2c_00654f80`: the same code.
    std_Tree_buyNode2c_00654f80 @ 0x0065_4f80 => extern "thiscall" fn d_00654f80(this: u32) -> u32 {
        ffx_system::wp5_tree::bridge::buy_node(&mut mem(), this, 0x2c)
    }
    /// `StdMapStringU32_buyNode_006e8970`: a node holding a string key from a C string and a word.
    StdMapStringU32_buyNode_006e8970 @ 0x006e_8970 => extern "thiscall" fn d_006e8970(this: u32, pair: u32) -> u32 {
        d::string_map_buy_node(&mut mem(), this, pair)
    }
    /// `AsyncLoad_startupTask_00636430`: the loader's start-up loads; `g_PhyreScene`.
    AsyncLoad_startupTask_00636430 @ 0x0063_6430 => extern "C" fn d_00636430() -> u32 {
        d::loader_startup_task(&mut mem())
    }
    /// `StdMapString_insertAt_006e8a50`: `StdTree_insertAt_006993d0`'s code but the overflow (the key at `+0x10`).
    StdMapString_insertAt_006e8a50 @ 0x006e_8a50 => extern "thiscall" fn d_006e8a50(this: u32, out: u32, add_left: u32, at: u32, value: u32, node: u32) -> u32 {
        let _ = value;
        string_insert_at(this, out, add_left, at, node, 0x10)
    }
    /// `VideoIndexMap_insertAt_006d5dc0`: the same (the key at `+0x14`).
    VideoIndexMap_insertAt_006d5dc0 @ 0x006d_5dc0 => extern "thiscall" fn d_006d5dc0(this: u32, out: u32, add_left: u32, at: u32, value: u32, node: u32) -> u32 {
        let _ = value;
        string_insert_at(this, out, add_left, at, node, 0x14)
    }
    /// `StdMapString_insertAt_0069c6b0`: the same (the key at `+0x10`).
    StdMapString_insertAt_0069c6b0 @ 0x0069_c6b0 => extern "thiscall" fn d_0069c6b0(this: u32, out: u32, add_left: u32, at: u32, value: u32, node: u32) -> u32 {
        let _ = value;
        string_insert_at(this, out, add_left, at, node, 0x10)
    }
    /// `StdMapString_insertAt_0064a130`: the same (the key at `+0x10`).
    StdMapString_insertAt_0064a130 @ 0x0064_a130 => extern "thiscall" fn d_0064a130(this: u32, out: u32, add_left: u32, at: u32, value: u32, node: u32) -> u32 {
        let _ = value;
        string_insert_at(this, out, add_left, at, node, 0x10)
    }
    /// `StdMapString_insertNohint_006e8c80`: a string-keyed insert without a hint; `out`.
    StdMapString_insertNohint_006e8c80 @ 0x006e_8c80 => extern "thiscall" fn d_006e8c80(this: u32, out: u32, leftish: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        let (_, ins, dec) = d::STRING_INSERT_NOHINT[0];
        d::string_insert_nohint(&mut mem(), this, out, leftish, value, node, frame.at(), ins, dec)
    }
    /// `StdMapString_insertNohint_0064a360`: the same with its own `_Insert_at`/`--`.
    StdMapString_insertNohint_0064a360 @ 0x0064_a360 => extern "thiscall" fn d_0064a360(this: u32, out: u32, leftish: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        let (_, ins, dec) = d::STRING_INSERT_NOHINT[1];
        d::string_insert_nohint(&mut mem(), this, out, leftish, value, node, frame.at(), ins, dec)
    }
    /// `StdMapString_insertNohint_0069cd40`: the same with its own `_Insert_at`/`--`.
    StdMapString_insertNohint_0069cd40 @ 0x0069_cd40 => extern "thiscall" fn d_0069cd40(this: u32, out: u32, leftish: u32, value: u32, node: u32) -> u32 {
        let mut frame = StackFrame::<1>::new();
        let (_, ins, dec) = d::STRING_INSERT_NOHINT[2];
        d::string_insert_nohint(&mut mem(), this, out, leftish, value, node, frame.at(), ins, dec)
    }
    /// `IggyRenderCommand_trampoline_00a34b00`: a jump to `[p + 4]` with `p`; its `eax`.
    IggyRenderCommand_trampoline_00a34b00 @ 0x00a3_4b00 => extern "C" fn d_00a34b00(p: u32) -> u32 {
        d::render_command_trampoline(&mut mem(), p)
    }
    /// `BasicDebugTweakUtil_getLineText`: a tool line into `buf`; `sprintf_s`'s `eax`.
    BasicDebugTweakUtil_getLineText @ 0x006b_69a0 => extern "thiscall" fn d_006b69a0(this: u32, n: u32, buf: u32) -> u32 {
        d::basic_debug_line(&mut mem(), this, n, buf)
    }
    /// `BtlChrParamConfigTweakUitl_getLineText_006b6bb0`: a tool line into `buf`.
    BtlChrParamConfigTweakUitl_getLineText_006b6bb0 @ 0x006b_6bb0 => extern "thiscall" fn d_006b6bb0(this: u32, n: u32, buf: u32) -> u32 {
        d::battle_param_line(&mut mem(), this, n, buf)
    }
    /// `GameParamConfigTweakUitl_getLineText`: a tool line into `buf`.
    GameParamConfigTweakUitl_getLineText @ 0x006b_72e0 => extern "thiscall" fn d_006b72e0(this: u32, n: u32, buf: u32) -> u32 {
        d::game_param_line(&mut mem(), this, n, buf)
    }
    /// `SaveData_string_007870d0`: the save data's string `i` (clamped 0..7).
    SaveData_string_007870d0 @ 0x0078_70d0 => extern "C" fn d_007870d0(i: u32) -> u32 {
        d::save_string(&mut mem(), i)
    }
    /// `gdraw_resetFrame_00a37fe0`: the device state cleared, the frame counters and draw lists reset.
    gdraw_resetFrame_00a37fe0 @ 0x00a3_7fe0 => extern "C" fn d_00a37fe0() -> u32 {
        d::gdraw_reset_frame(&mut mem())
    }
    /// `NameList_buyNode_00680490`: a list node holding a copy of `{string, word}`.
    NameList_buyNode_00680490 @ 0x0068_0490 => extern "thiscall" fn d_00680490(this: u32, next: u32, prev: u32, value: u32) -> u32 {
        let _ = this;
        d::name_list_buy_node(&mut mem(), next, prev, value)
    }
    /// `RemapButtonIcon_buttonMaterial_006e8390`: a button's cluster, texture and material set up.
    RemapButtonIcon_buttonMaterial_006e8390 @ 0x006e_8390 => extern "thiscall" fn d_006e8390(this: u32, name: u32, cluster: u32, texture: u32, material: u32, instance: u32) -> u32 {
        let _ = this;
        let mut frame = StackFrame::<12>::new();
        d::button_material(&mut mem(), frame.at(), name, cluster, texture, material, instance)
    }
}
