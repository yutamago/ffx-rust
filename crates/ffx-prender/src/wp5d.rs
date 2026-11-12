//! M5 finish, WP5 part 3 (wp5d, the `FFX.exe` frontier): originals the
//! ports still called in Unicorn in a headless boot (`entries.tsv` of
//! `ffx-rust --profile`) and the no-Unicorn boot's stops. Each follows
//! its original's machine code; `eax` is the original's.

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PPhysics::PPhysicsCharacterCamera`'s vtable.
pub const CHARACTER_CAMERA_VTABLE: u32 = 0x00b3_b0a8;

/// `PPhysicsCharacterCamera::PPhysicsCharacterCamera(this)`
/// (`0x005dff30`, thiscall; the camera member at `+0xbc` of
/// `Phyre_PGameplay_Component_ctor_005572e0`): the vtable, the defaults
/// (`f32`s from `.rdata`, stored as the bits the x87's `fst dword` gives
/// back: every one is exact), and `+0x1b0..+0x200` zeroed. Words `+0x3c`,
/// `+0x4c`, `+0x5c` and `+0x6c..+0x1b0` are not written. `this`.
pub fn character_camera_ctor(h: &mut impl Host, this: u32) -> u32 {
    let vt = h.rebase(CHARACTER_CAMERA_VTABLE);
    h.write(this, vt);
    // in the original's store order
    for (off, bits) in [
        (0x08u32, 0x3f00_0000u32), // 0.5
        (0x04, 0),
        (0x0c, 0x4080_0000), // 4
        (0x10, 0x4033_3333), // 2.8
        (0x14, 0x3dcc_cccd), // 0.1
        (0x18, 0x4000_0000), // 2
        (0x1c, 0x40c0_0000), // 6
        (0x20, 0x3f80_0000), // 1
        (0x24, 0),
        (0x28, 0x4040_0000), // 3
        (0x2c, 0x3f00_0000), // 0.5
        (0x30, 0),
        (0x34, 0x3e99_999a), // 0.3
        (0x38, 0),
        (0x40, 0),
        (0x44, 0),
        (0x48, 0),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x60, 0),
        (0x64, 0),
        (0x68, 0x3f80_0000), // 1
    ] {
        h.write(this + off, bits);
    }
    for off in (0x1b0..0x200).step_by(4) {
        h.write(this + off, 0u32);
    }
    this
}

/// `PPhysicsCharacterCamera::~PPhysicsCharacterCamera(this)`
/// (`0x005e20c0`, thiscall): the vtable back; `eax` is the caller's.
pub fn character_camera_dtor(h: &mut impl Host, this: u32) {
    let vt = h.rebase(CHARACTER_CAMERA_VTABLE);
    h.write(this, vt);
}

/// `operator new` (`0x00630310`, cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;

/// The debug tools' `CreateInstance` functions after
/// `CharacterSwitchTweakUtil_CreateInstance` (`0x006b5cb0` ..
/// `0x006b6040`, cdecl, one shape): when the global at `global` is 0,
/// `operator new(size)` and, when that is not NULL, the constructor
/// `ctor` (thiscall) on it; the global gets its result (or 0). The new
/// instance; when it already exists the original's `eax` is an address
/// in its own frame (`lea eax, [ebp-0xc]` of the SEH set-up), for which
/// this returns the instance (`graphicInitialize`, the only caller,
/// ignores it).
pub fn create_once(h: &mut impl Host, global: u32, size: u32, ctor: u32) -> u32 {
    let at = h.rebase(global);
    let g: u32 = h.read(at);
    if g != 0 {
        return g;
    }
    let n = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
    let v = if n == 0 {
        0
    } else {
        h.call_sym(ctor, Conv::Thiscall(n), &[])
    };
    h.write(at, v);
    v
}

/// The `CreateInstance` functions of [`create_once`]'s shape: address,
/// global, size, constructor.
pub const CREATE_ONCE: [(u32, u32, u32, u32); 8] = [
    (0x006b_5cb0, 0x00cc_cad0, 0x120, 0x006b_52a0),
    (0x006b_5d30, 0x00cc_cae0, 0x11c, 0x006b_53b0),
    (0x006b_5db0, 0x00cc_cad8, 0x1e4, 0x006b_5440),
    (0x006b_5e30, 0x00cc_cadc, 0xe0, 0x006b_5520),
    (0x006b_5f30, 0x00cc_cad4, 0xf0, 0x006b_5670),
    (0x006b_5fc0, 0x00cc_cae4, 0x108, 0x006b_5730),
    (0x006b_6040, 0x00cc_cac8, 0x214, 0x006b_57d0),
    // the frameless one (no SEH frame; `eax` when it exists: the caller's)
    (0x006b_60c0, 0x00cc_cac0, 0xb8, 0x006b_5870),
];

/// The debug text panel's text constructor (`0x006f5b50`, thiscall).
pub const TEXT_UTIL_CTOR: u32 = 0x006f_5b50;
/// The panels' text scale (`f32` 0.02, `.rdata`).
pub const TEXT_SCALE: u32 = 0x00b4_82d8;
/// `memset` (the CRT thunk, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `sprintf_s`'s import slot.
pub const SPRINTF_S_IAT: u32 = 0x00b0_c394;

/// An `f32` as `fld dword; fstp dword` copies it (a signalling NaN
/// quieted).
fn quiet(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// The text panel embedded in the debug tools (inlined in each
/// constructor): its flag byte at `flag` 0, the text at `flag + 4`
/// constructed, the scale at `scale`, the words at `zero_a`, `zero_b`
/// and the byte at `scale - 4` 0, the 1280 x 720 area, three floats 0
/// and the last 1.
fn panel(h: &mut impl Host, this: u32, flag: u32, scale: u32, zero_a: u32, zero_b: u32) {
    h.write(this + flag, 0u8);
    h.call_sym(TEXT_UTIL_CTOR, Conv::Thiscall(this + flag + 4), &[]);
    let s = quiet(h.read(h.rebase(TEXT_SCALE)));
    h.write(this + scale, s);
    h.write(this + zero_a, 0u32);
    h.write(this + zero_b, 0u32);
    h.write(this + scale - 4, 0u8);
    h.write(this + scale + 4, 0x500u32);
    h.write(this + scale + 8, 0x2d0u32);
    for k in 0..3 {
        h.write(this + scale + 0xc + 4 * k, 0u32);
    }
    h.write(this + scale + 0x18, 1.0f32.to_bits());
}

/// The debug tool constructed by `0x006b5e30` (`0x006b5520`, thiscall)
/// -> `this`: the panel (flag `+0x14`, scale `+0xc4`), the byte `+0` 1,
/// `+4..+0xc` 0, `+0x10` 1, `+0xa8..+0xb4` 0.
pub fn tweak_ctor_6b5520(h: &mut impl Host, this: u32) -> u32 {
    panel(h, this, 0x14, 0xc4, 0xb8, 0xbc);
    h.write(this, 1u8);
    for off in [4, 8, 0xc] {
        h.write(this + off, 0u32);
    }
    h.write(this + 0x10, 1u32);
    for off in [0xa8, 0xac, 0xb0, 0xb4] {
        h.write(this + off, 0u32);
    }
    this
}

/// The debug tool constructed by `0x006b5f30` (`0x006b5670`, thiscall)
/// -> `this`: the panel (flag `+0x1c`, scale `+0xd4`), the half-word
/// `+0x10` 0, `+0..+0xc`, `+0x18` and `+0xb0..+0xc4` 0.
pub fn tweak_ctor_6b5670(h: &mut impl Host, this: u32) -> u32 {
    panel(h, this, 0x1c, 0xd4, 0xc8, 0xcc);
    h.write(this + 0x10, 0u16);
    for off in [0, 4, 8, 0xc, 0x18] {
        h.write(this + off, 0u32);
    }
    for off in (0xb0..0xc8).step_by(4) {
        h.write(this + off, 0u32);
    }
    this
}

/// The debug tool constructed by `0x006b5fc0` (`0x006b5730`, thiscall)
/// -> `this`: the panel (flag `+0x10`, scale `+0xec`), `+0` and the
/// unaligned word `+6` 0, the bytes `+5` and `+0xa` 0, the 0x3c bytes at
/// `+0xa4` cleared (`memset`).
pub fn tweak_ctor_6b5730(h: &mut impl Host, this: u32) -> u32 {
    panel(h, this, 0x10, 0xec, 0xe0, 0xe4);
    h.write(this, 0u32);
    h.write(this + 6, 0u32);
    h.write(this + 5, 0u8);
    h.write(this + 0xa, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[this + 0xa4, 0, 0x3c]);
    this
}

/// The debug tool constructed by `0x006b6040` (`0x006b57d0`, thiscall)
/// -> `this`: the panel (flag `+0`, scale `+0x1f8`; its zero words
/// `+0x1e4`, `+0x1e8`), `+0x1ec` 0, `+0x1f0` 10, the 0xa8 bytes at
/// `+0x94` cleared (`memset`).
pub fn tweak_ctor_6b57d0(h: &mut impl Host, this: u32) -> u32 {
    panel(h, this, 0, 0x1f8, 0x1e4, 0x1e8);
    h.write(this + 0x1ec, 0u32);
    h.write(this + 0x1f0, 10u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x94, 0, 0xa8]);
    this
}

/// The vtable-like word `0x006b5870` stores at `+0`.
pub const VIEWER_WORD0: u32 = 0x00b4_86b8;
/// `"c001"`, `"grid00"` (`.rdata`).
pub const TEXT_C001: u32 = 0x00b4_86bc;
pub const TEXT_GRID00: u32 = 0x00b4_24c8;

/// The viewer manager's constructor (`0x006b5870`, thiscall; the object
/// `g_ViewerManager` `0x00cccac0` holds) -> `this`: `+0` the image word
/// `0x00b486b8`, an empty `std::string` at `+4` (capacity 15, size 0,
/// the first byte 0), `+0x44..+0x4c` 0, the byte `+0x50` 1, then
/// `sprintf_s(+0x1c, 20, "c001")` and `sprintf_s(+0x30, 20, "grid00")`.
pub fn viewer_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    let w = h.rebase(VIEWER_WORD0);
    h.write(this, w);
    h.write(this + 0x18, 0xfu32);
    h.write(this + 0x14, 0u32);
    h.write(this + 4, 0u8);
    for off in [0x44, 0x48, 0x4c] {
        h.write(this + off, 0u32);
    }
    h.write(this + 0x50, 1u8);
    let sprintf_s: u32 = h.read(h.rebase(SPRINTF_S_IAT));
    for (off, text) in [(0x1c, TEXT_C001), (0x30, TEXT_GRID00)] {
        let t = h.rebase(text);
        h.call(sprintf_s, Conv::Cdecl, &[this + off, 0x14, t]);
    }
    this
}

/// `KERNEL32` import slots: `CreateThread`, `SetThreadPriority`.
pub const CREATE_THREAD_IAT: u32 = 0x00b0_c104;
pub const SET_THREAD_PRIORITY_IAT: u32 = 0x00b0_c10c;

/// `Phyre::PThreadWin32`'s start (`0x004300b0`, thiscall `(this; desc)`,
/// `ret 4`) -> 0. `desc`: `+0` the routine, `+4` the stack size, `+8` the
/// parameter, `+0xc` the priority class, `+0x10` the name. The handle
/// of `CreateThread(0, [desc+4], [desc], [desc+8], 0, &id)` goes to
/// `[this]` (`id` the word at `frame`, game memory, zeroed first). With a
/// handle: priority class 0 is `SetThreadPriority(handle, 1)`, 2 is
/// `SetThreadPriority(handle, -1)`, others leave it. Left out: with a
/// thread id, the original names the thread for a debugger
/// (`RaiseException(0x406d1388, 0, 4, info)` with the name, `[desc+0x10]`
/// or `GetDefaultThreadName`'s, inside `__try`/`__except` that swallows
/// it): without a debugger attached nothing changes.
pub fn thread_start(h: &mut impl Host, this: u32, desc: u32, frame: u32) -> u32 {
    h.write(frame, 0u32);
    let routine: u32 = h.read(desc);
    let stack: u32 = h.read(desc + 4);
    let param: u32 = h.read(desc + 8);
    let f: u32 = h.read(h.rebase(CREATE_THREAD_IAT));
    let handle = h.call(f, Conv::Stdcall, &[0, stack, routine, param, 0, frame]);
    h.write(this, handle);
    if handle == 0 {
        return 0;
    }
    let priority = match h.read::<u32>(desc + 0xc) {
        0 => 1,
        2 => u32::MAX,
        _ => return 0,
    };
    let f: u32 = h.read(h.rebase(SET_THREAD_PRIORITY_IAT));
    let handle: u32 = h.read(this);
    h.call(f, Conv::Stdcall, &[handle, priority]);
    0
}

/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The memory report object's global (`Instance_006dc800_Get` reads it)
/// and the flag `0x006dc800` sets.
pub const MEMORY_REPORT: u32 = 0x00cd_ede8;
pub const MEMORY_REPORT_MADE: u32 = 0x00c3_1e98;
/// The panel's format, `"RAM=%s - VRAM=%s - HOST=%s - SRAM=%s -
/// FMOD=%s"` (47 bytes with the NUL, `.rdata`).
pub const MEMORY_FORMAT: u32 = 0x00b4_a390;

/// The memory report panel's constructor (`0x006dc450`, thiscall; at
/// `+0x110` of the object `0x006dc800` makes) -> `this`: `+0x160`,
/// `+0x164` 0.0, `+0` 0, the half-word `+4` 0, the 1280 x 720 area at
/// `+0x158`, `+0x17c` 0, the 0x80 bytes at `+6` cleared, `+0x168..+0x178`
/// 0, the 0x50 bytes at `+0x86` cleared, and the format's 47 bytes
/// copied to `+0xd6` (through a frame copy: the same bytes).
pub fn memory_panel_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this + 0x160, 0u32);
    h.write(this + 0x164, 0u32);
    h.write(this, 0u32);
    h.write(this + 4, 0u16);
    h.write(this + 0x158, 0x500u32);
    h.write(this + 0x15c, 0x2d0u32);
    h.write(this + 0x17c, 0u32);
    h.call_sym(MEMSET, Conv::Cdecl, &[this + 6, 0, 0x80]);
    for off in (0x168..0x17c).step_by(4) {
        h.write(this + off, 0u32);
    }
    h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x86, 0, 0x50]);
    let mut text = [0u8; 47];
    h.read_bytes(h.rebase(MEMORY_FORMAT), &mut text);
    h.write_bytes(this + 0xd6, &text);
    this
}

/// `user_malloc` (`0x00687020`, cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `StackWalker::StackWalker` (`0x006870b0`, thiscall, 4 stack words).
pub const STACK_WALKER_CTOR: u32 = 0x0068_70b0;
/// `BacktraceOutputWin32`'s vtable and the global its instance goes to.
pub const BACKTRACE_VTABLE: u32 = 0x00b4_1358;
pub const BACKTRACE: u32 = 0x00cc_b15c;
/// `KERNEL32` import slots: `GetCurrentProcess`, `GetCurrentProcessId`,
/// `CreateMutexW`.
pub const GET_CURRENT_PROCESS_IAT: u32 = 0x00b0_c19c;
pub const GET_CURRENT_PROCESS_ID_IAT: u32 = 0x00b0_c1c4;
pub const CREATE_MUTEX_W_IAT: u32 = 0x00b0_c198;

fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(slot));
    h.call(f, Conv::Stdcall, args)
}

/// The crash reporter's `BacktraceOutputWin32` made (`0x00630870`,
/// cdecl; called by `0x006dc800`): `user_malloc(0xac)`; NULL: the global
/// `0x00ccb15c` 0, `eax` 0. Else the `StackWalker` constructed on it
/// (options 0x3f, no symbol path, `GetCurrentProcessId()`,
/// `GetCurrentProcess()`, the process asked first), its vtable
/// `BacktraceOutputWin32`'s, `+0xa8` a new mutex (`CreateMutexW(0, 0,
/// 0)`), the global it; `eax` the mutex handle.
pub fn backtrace_create(h: &mut impl Host) -> u32 {
    let n = h.call_sym(USER_MALLOC, Conv::Cdecl, &[0xac]);
    let g = h.rebase(BACKTRACE);
    if n == 0 {
        h.write(g, 0u32);
        return 0;
    }
    let process = import(h, GET_CURRENT_PROCESS_IAT, &[]);
    let id = import(h, GET_CURRENT_PROCESS_ID_IAT, &[]);
    h.call_sym(
        STACK_WALKER_CTOR,
        Conv::Thiscall(n),
        &[0x3f, 0, id, process],
    );
    let vt = h.rebase(BACKTRACE_VTABLE);
    h.write(n, vt);
    let m = import(h, CREATE_MUTEX_W_IAT, &[0, 0, 0]);
    h.write(n + 0xa8, m);
    h.write(g, n);
    m
}

/// The memory report object made once (`0x006dc800`, cdecl; from
/// `graphicInitialize`): when the global `0x00cdede8` is 0,
/// `Phyre_PhyreMalloc(0x290, 4)` goes there; when not NULL its half-word
/// `+0`, `+8` and `+0x10c` are 0 and the panel at `+0x110` is
/// constructed (`0x006dc450`); the global is written again (0 when the
/// allocation failed), the backtrace output made (`0x00630870`) and
/// `0x00c31e98` set to 1. `eax` the last call's; when the object exists
/// the original leaves the caller's (`cmp`, `jne` to `ret`), for which
/// this returns 0 (`graphicInitialize` ignores it).
pub fn memory_report_create(h: &mut impl Host) -> u32 {
    let g = h.rebase(MEMORY_REPORT);
    if h.read::<u32>(g) != 0 {
        return 0;
    }
    let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x290, 4]);
    h.write(g, p);
    if p != 0 {
        h.write(p, 0u16);
        h.write(p + 8, 0u32);
        h.write(p + 0x10c, 0u32);
        h.call_sym(0x006d_c450, Conv::Thiscall(p + 0x110), &[]);
    }
    h.write(g, p);
    let r = h.call_sym(0x0063_0870, Conv::Cdecl, &[]);
    h.write(h.rebase(MEMORY_REPORT_MADE), 1u32);
    r
}

/// `Thread_Start_00679db0`: the routine `SysThread_create` starts.
pub const SYS_THREAD_START: u32 = 0x0067_9db0;

/// `Sys::Thread`'s creation (`0x00679cb0`, thiscall `(this; stack,
/// run, unused, name)`, `ret 0x10`) -> 0. With a thread already there
/// (the byte `+0xc` 0 and a handle at `+4`) nothing. Else `+0xc` is 0
/// and `[this+4] = CreateThread(0, stack, Thread_Start_00679db0, this,
/// flags, this + 8)`, `flags` 0 (run at once) when the byte `run` is 1,
/// else `CREATE_SUSPENDED` (4). Left out as in
/// `Phyre_PThreadWin32_start_004300b0`: the debugger's thread name
/// (`RaiseException(0x406d1388, ...)` with `name` and the id at `+8`,
/// swallowed by its `__except`). `eax`: the original's is a frame
/// address on the first path and whatever the exception's dispatch
/// left on the other; its callers (`JobWorker_start_00703a50` and two
/// more) don't read it.
pub fn sys_thread_create(h: &mut impl Host, this: u32, stack: u32, run: u32) -> u32 {
    if h.read::<u8>(this + 0xc) == 0 && h.read::<u32>(this + 4) != 0 {
        return 0;
    }
    h.write(this + 0xc, 0u8);
    let flags = if run & 0xff == 1 { 0 } else { 4 };
    let start = h.rebase(SYS_THREAD_START);
    let f: u32 = h.read(h.rebase(CREATE_THREAD_IAT));
    let handle = h.call(f, Conv::Stdcall, &[0, stack, start, this, flags, this + 8]);
    h.write(this + 4, handle);
    0
}

/// `StdMap_NewHead_0069a1b0` (thiscall on the map, no stack words): a
/// new head node.
pub const MAP_NEW_HEAD: u32 = 0x0069_a1b0;

/// `ClusterManager`'s constructor (`0x00699ab0`, thiscall; from
/// `ClusterManager_CreateInstance_0069a060`) -> `this`: `+0` 2, `+4`
/// 0.0, `+8..+0x14` 0, the map at `+0x10` gets its head node
/// (`StdMap_NewHead`), `+0x641c` 0; a 4-byte block (`operator new`)
/// holding `CreateMutexW(0, 0, 0)` goes to `+0x18` (0 when the
/// allocation fails); the 0x400 bytes at `+0x601c` cleared.
pub fn cluster_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 2u32);
    h.write(this + 4, 0u32);
    for off in [8, 0xc, 0x10, 0x14] {
        h.write(this + off, 0u32);
    }
    let head = h.call_sym(MAP_NEW_HEAD, Conv::Thiscall(this + 0x10), &[]);
    h.write(this + 0x10, head);
    h.write(this + 0x641c, 0u32);
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[4]);
    if p != 0 {
        let m = import(h, CREATE_MUTEX_W_IAT, &[0, 0, 0]);
        h.write(p, m);
    }
    h.write(this + 0x18, p);
    h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x601c, 0, 0x400]);
    this
}

/// A pair's constructor (`0x006c7ca0`, and its copy `0x006c7cb0`;
/// thiscall, the element constructor of `ShadowManager_ctor_006c7d80`'s
/// arrays): `+0`, `+4` 0. `this`.
pub fn zero_pair(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u32);
    h.write(this + 4, 0u32);
    this
}

/// `operator delete` (`0x00630390`, cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

/// The fill helper `0x0064a970` (cdecl `(dst, n, &value, ..)`): `n`
/// words from `dst` get `*value` (read again for each word); a NULL
/// `dst` writes nothing but still counts.
fn fill_words(h: &mut impl Host, dst: u32, n: u32, value: u32) {
    let mut at = dst;
    for _ in 0..n {
        if at != 0 {
            let v: u32 = h.read(value);
            h.write(at, v);
        }
        at = at.wrapping_add(4);
    }
}

/// The copy helper `0x0064a9e0` (cdecl `(first, last, dst, ..)`): the
/// words of `[first, last)` to `dst` in order (none written to a NULL
/// `dst`); the end of the destination.
fn copy_words(h: &mut impl Host, first: u32, last: u32, dst: u32) -> u32 {
    let (mut p, mut d) = (first, dst);
    while p != last {
        if d != 0 {
            let v: u32 = h.read(p);
            h.write(d, v);
        }
        p = p.wrapping_add(4);
        d = d.wrapping_add(4);
    }
    d
}

/// Signed word distance `(b - a) / 4` as the original's `sub; sar 2`.
fn words(a: u32, b: u32) -> u32 {
    (b.wrapping_sub(a).cast_signed() >> 2).cast_unsigned()
}

/// `std::vector<T *>::_Insert_n(this; &out, where, count, &value)`
/// (`0x00655460`, thiscall, `ret 0x10`; from `ObjectList_ctor_0064cec0`
/// and five more) -> `out`: `count` copies of `*value` inserted before
/// `where`, `[out]` the iterator at the insertion point (the new
/// buffer's when it grew). Without room: a new buffer of `max(size +
/// count, capacity * 1.5)` words (the growth 0 when `capacity / 2`
/// overflows `0x3fffffff`), the copies, the words before and after
/// moved in, the old buffer deleted. With room: the words after `where`
/// moved back by `count` (forward through the free space past the end,
/// backward over the overlap), and the gap filled with the value read
/// once. Not ported: the `_Xlength_error`/`_Xbad_alloc` throws (a size
/// past `0x3fffffff` words, `operator new` returning NULL); the port
/// leaves the vector as it is then and still writes `[out]`.
pub fn vector_insert_n(
    h: &mut impl Host,
    this: u32,
    out: u32,
    at: u32,
    count: u32,
    value: u32,
) -> u32 {
    let first: u32 = h.read(this);
    let off = words(first, at);
    if count != 0 {
        let last: u32 = h.read(this + 4);
        let end: u32 = h.read(this + 8);
        if words(last, end) < count {
            let size = words(first, last);
            if 0x3fff_ffffu32.wrapping_sub(size) < count {
                return out;
            }
            let need = size.wrapping_add(count);
            let cap = words(first, end);
            let mut grown = if 0x3fff_ffff - (cap >> 1) < cap {
                0
            } else {
                cap + (cap >> 1)
            };
            if grown < need {
                grown = need;
            }
            let mut buf = 0;
            if grown != 0 {
                if grown > 0x3fff_ffff {
                    return out;
                }
                buf = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[grown * 4]);
                if buf == 0 {
                    return out;
                }
            }
            let off = words(first, at);
            fill_words(h, buf.wrapping_add(off.wrapping_mul(4)), count, value);
            copy_words(h, first, at, buf);
            copy_words(
                h,
                at,
                last,
                buf.wrapping_add(off.wrapping_add(count).wrapping_mul(4)),
            );
            let first: u32 = h.read(this);
            let last: u32 = h.read(this + 4);
            let total = count.wrapping_add(words(first, last));
            if first != 0 {
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[first]);
            }
            h.write(this, buf);
            h.write(this + 8, buf.wrapping_add(grown * 4));
            h.write(this + 4, buf.wrapping_add(total.wrapping_mul(4)));
        } else {
            let v: u32 = h.read(value);
            let bytes = count.wrapping_mul(4);
            if words(at, last) < count {
                copy_words(h, at, last, at.wrapping_add(bytes));
                let mut p = last;
                for _ in 0..count - words(at, last) {
                    h.write(p, v);
                    p = p.wrapping_add(4);
                }
                let new_last = h.read::<u32>(this + 4).wrapping_add(bytes);
                h.write(this + 4, new_last);
                let mut p = at;
                while p != new_last.wrapping_sub(bytes) {
                    h.write(p, v);
                    p = p.wrapping_add(4);
                }
            } else {
                let tail = last.wrapping_sub(bytes);
                let e = copy_words(h, tail, last, last);
                h.write(this + 4, e);
                let mut src = tail;
                while src != at {
                    src = src.wrapping_sub(4);
                    let w: u32 = h.read(src);
                    h.write(src.wrapping_add(bytes), w);
                }
                let mut p = at;
                while p != at.wrapping_add(bytes) {
                    h.write(p, v);
                    p = p.wrapping_add(4);
                }
            }
        }
    }
    let first: u32 = h.read(this);
    h.write(out, first.wrapping_add(off.wrapping_mul(4)));
    out
}

/// A `std::string` (MSVC 2012: 16 inline bytes or a heap pointer at
/// `+0`, size `+0x10`, capacity `+0x14`) emptied as its destructor
/// leaves it: the heap buffer deleted (capacity 16 or more), capacity
/// 15, size 0, the first byte 0.
fn string_tidy(h: &mut impl Host, s: u32) {
    if h.read::<u32>(s + 0x14) >= 0x10 {
        let p: u32 = h.read(s);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }
    h.write(s + 0x14, 0xfu32);
    h.write(s + 0x10, 0u32);
    h.write(s, 0u8);
}

/// `std::_Destroy_range` for `std::string` (`0x00635c40`, `(first,
/// last)`, callee pops 8, `ecx` unused; `AssetListReader_clear` and nine
/// more): each string of `[first, last)` (0x18 bytes apart) emptied.
/// `eax` is junk (the last delete's, or the caller's); no caller reads
/// it.
pub fn destroy_strings(h: &mut impl Host, first: u32, last: u32) {
    let mut s = first;
    while s != last {
        string_tidy(h, s);
        s = s.wrapping_add(0x18);
    }
}

/// `std::_Tree<.., std::string>::_Erase(this; node)` (`0x00635c80`,
/// thiscall, `ret 4`): the subtree under `node` freed (until a nil
/// node, byte `+0xd`): the right subtree first (recursively; the
/// original calls itself), then the node's string (`+0x10`) emptied and the node
/// deleted, then on to its left child (`+0`, read before). `eax` junk.
pub fn tree_erase_strings(h: &mut impl Host, this: u32, node: u32) {
    let mut n = node;
    while h.read::<u8>(n + 0xd) == 0 {
        let right: u32 = h.read(n + 8);
        tree_erase_strings(h, this, right);
        let left: u32 = h.read(n);
        string_tidy(h, n + 0x10);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[n]);
        n = left;
    }
}
