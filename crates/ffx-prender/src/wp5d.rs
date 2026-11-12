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

/// `Phyre_PCluster_allocateInstanceList` (thiscall `(cluster; n,
/// descriptor, c)`), `PRenderTarget`'s class descriptor and the render
/// target's constructor `Phyre_PRendering_Ctor_00583a60` (thiscall
/// `(this; a)`).
pub const ALLOCATE_INSTANCE_LIST: u32 = 0x0044_0b70;
pub const RENDER_TARGET_DESCRIPTOR: u32 = 0x00ca_4200;
pub const RENDER_TARGET_CTOR: u32 = 0x0058_3a60;

/// A render target added to a cluster (`0x0064adf0`, thiscall
/// `(cluster; a)`, `ret 4`; `PostProcessManager_onAllShaderBinFilesLoaded`
/// and ten more): one `PRenderTarget` instance list allocated in
/// `cluster`; with one and its first instance (`+0x34`), that instance
/// constructed with `a`. The instance, or 0.
pub fn cluster_add_render_target(h: &mut impl Host, cluster: u32, a: u32) -> u32 {
    let desc = h.rebase(RENDER_TARGET_DESCRIPTOR);
    let list = h.call_sym(
        ALLOCATE_INSTANCE_LIST,
        Conv::Thiscall(cluster),
        &[1, desc, 0],
    );
    if list == 0 {
        return 0;
    }
    let inst: u32 = h.read(list + 0x34);
    if inst == 0 {
        return 0;
    }
    h.call_sym(RENDER_TARGET_CTOR, Conv::Thiscall(inst), &[a]);
    inst
}

/// `Phyre_AllocCopy_00444c50` (cdecl `(n, src, count)`) and
/// `Phyre_PhyreFree` (cdecl).
pub const ALLOC_COPY: u32 = 0x0044_4c50;
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// `PArray<u8>::resize(this; n)` as `PStringBuilder` uses it
/// (`0x00444e90`, thiscall, `ret 4`; `Phyre_PStringBuilder_operator`
/// and the thunk `0x00444e80`) -> 0, or 0xd when the new buffer can't
/// be had. The count word `+0` (bit 31: the buffer at `+4` is not
/// ours) equal to `n` as a whole: nothing. Else the new buffer is
/// `Phyre_AllocCopy(n, [+4], min(n, count))`; NULL for a non-zero `n`:
/// 0xd, nothing changed. The old buffer is freed when it differs from
/// the new one, is ours and is not NULL; `+4` the new one, `+0` `n`.
pub fn byte_array_resize(h: &mut impl Host, this: u32, n: u32) -> u32 {
    let word: u32 = h.read(this);
    let count = word & 0x7fff_ffff;
    if n == word {
        return 0;
    }
    let keep = n.min(count);
    let old: u32 = h.read(this + 4);
    let buf = h.call_sym(ALLOC_COPY, Conv::Cdecl, &[n, old, keep]);
    if buf == 0 && n != 0 {
        return 0xd;
    }
    let old: u32 = h.read(this + 4);
    if old != buf && h.read::<u32>(this) >> 31 == 0 && old != 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + 4, buf);
    h.write(this, n);
    0
}

/// dlmalloc's globals: the global lock (`0x00ce6eb8`: the word, its
/// recursion count, its owner thread) and `mparams` (magic, page size,
/// granularity, mmap threshold, trim threshold, default flags).
pub const MALLOC_LOCK: u32 = 0x00ce_6eb8;
pub const MALLOC_LOCK_COUNT: u32 = 0x00ce_6ebc;
pub const MALLOC_LOCK_OWNER: u32 = 0x00ce_6ec0;
pub const MPARAMS_MAGIC: u32 = 0x00ce_6ec4;
pub const MPARAMS_PAGE_SIZE: u32 = 0x00ce_6ec8;
pub const MPARAMS_GRANULARITY: u32 = 0x00ce_6ecc;
pub const MPARAMS_MMAP_THRESHOLD: u32 = 0x00ce_6ed0;
pub const MPARAMS_TRIM_THRESHOLD: u32 = 0x00ce_6ed4;
pub const MPARAMS_DEFAULT_MFLAGS: u32 = 0x00ce_6ed8;
/// `KERNEL32` import slots: `GetCurrentThreadId`, `SleepEx`,
/// `GetSystemInfo`, `GetTickCount`.
pub const GET_CURRENT_THREAD_ID_IAT: u32 = 0x00b0_c108;
pub const SLEEP_EX_IAT: u32 = 0x00b0_c0a8;
pub const GET_SYSTEM_INFO_IAT: u32 = 0x00b0_c0a4;
pub const GET_TICK_COUNT_IAT: u32 = 0x00b0_c0a0;
/// `InterlockedCompareExchange`'s slot: the lock's `xchg [lock], 1` is
/// `InterlockedCompareExchange(lock, 1, 0)` here (the word is only ever
/// 0 or 1, so both leave 1 and tell whether it was free; atomic in the
/// hooked game too).
pub const INTERLOCKED_CMPXCHG_IAT: u32 = 0x00b0_c12c;

/// dlmalloc's `init_mparams` (`0x006ea050`, cdecl; `create_mspace_with_base`
/// when the magic is 0) -> 1. The global lock taken (recursively by its
/// owner; else spun on (see [`INTERLOCKED_CMPXCHG_IAT`]), `SleepEx(0, 0)` every 64th turn);
/// with no magic yet: `GetSystemInfo` into `frame` (0x24 bytes of game
/// memory, the original's `ebp - 0x24`) gives the page size and the
/// allocation granularity, the thresholds 0x40000 and 0x200000, the
/// flags 7, and the magic `(GetTickCount() ^ 0x55555550) & ~7 | 8`; the
/// lock released.
pub fn init_mparams(h: &mut impl Host, frame: u32) -> u32 {
    let lock = h.rebase(MALLOC_LOCK);
    let count = h.rebase(MALLOC_LOCK_COUNT);
    let owner = h.rebase(MALLOC_LOCK_OWNER);
    let mut spins = 0u32;
    loop {
        if h.read::<u32>(lock) != 0 {
            let me = import(h, GET_CURRENT_THREAD_ID_IAT, &[]);
            if h.read::<u32>(owner) == me {
                let c: u32 = h.read(count);
                h.write(count, c.wrapping_add(1));
                break;
            }
        } else if import(h, INTERLOCKED_CMPXCHG_IAT, &[lock, 1, 0]) == 0 {
            let me = import(h, GET_CURRENT_THREAD_ID_IAT, &[]);
            h.write(owner, me);
            h.write(count, 1u32);
            break;
        }
        spins = spins.wrapping_add(1);
        if spins & 0x3f == 0 {
            import(h, SLEEP_EX_IAT, &[0, 0]);
        }
    }
    if h.read::<u32>(h.rebase(MPARAMS_MAGIC)) == 0 {
        import(h, GET_SYSTEM_INFO_IAT, &[frame]);
        let granularity: u32 = h.read(frame + 0x1c);
        h.write(h.rebase(MPARAMS_GRANULARITY), granularity);
        let page: u32 = h.read(frame + 4);
        h.write(h.rebase(MPARAMS_PAGE_SIZE), page);
        h.write(h.rebase(MPARAMS_MMAP_THRESHOLD), 0x4_0000u32);
        h.write(h.rebase(MPARAMS_TRIM_THRESHOLD), 0x20_0000u32);
        h.write(h.rebase(MPARAMS_DEFAULT_MFLAGS), 7u32);
        let t = import(h, GET_TICK_COUNT_IAT, &[]);
        h.write(h.rebase(MPARAMS_MAGIC), ((t ^ 0x5555_5550) & !7) | 8);
    }
    let c = h.read::<u32>(count).wrapping_sub(1);
    h.write(count, c);
    if c == 0 {
        h.write(owner, 0u32);
        // the original's `xchg` (a release by the owner: a store)
        h.write(lock, 0u32);
    }
    1
}

/// dlmalloc's `align_offset` of an address: the bytes to the next
/// multiple of 8.
fn align8(a: u32) -> u32 {
    if a & 7 == 0 { 0 } else { a.wrapping_neg() & 7 }
}

/// dlmalloc's `init_user_mstate(base, size)` (`0x006ea180`, cdecl;
/// `create_mspace_with_base`) -> the state `m`: placed in the first
/// chunk at `base` (aligned), cleared (0x1e8 bytes; that chunk's head
/// 0x1e8 | 3), the magic, `least_addr` and the segment's base `base`,
/// footprint, max footprint and the segment's size `size`, release
/// checks 0xfff, the flags `default | 4` (no system allocation); the 32
/// small bins point at themselves; the top chunk is the rest of the
/// space but its 0x28-byte foot (`init_top`, inlined), the trim
/// threshold copied.
pub fn init_user_mstate(h: &mut impl Host, base: u32, size: u32) -> u32 {
    let off = align8(base);
    let m = base.wrapping_add(8).wrapping_add(off);
    h.call_sym(MEMSET, Conv::Cdecl, &[m, 0, 0x1e8]);
    for o in [0x1c4, 0x1c0, 0x1bc] {
        h.write(m + o, 0u32);
    }
    h.write(base.wrapping_add(off).wrapping_add(4), 0x1ebu32);
    let magic: u32 = h.read(h.rebase(MPARAMS_MAGIC));
    h.write(m + 0x24, magic);
    let flags: u32 = h.read(h.rebase(MPARAMS_DEFAULT_MFLAGS));
    h.write(m + 0x10, base);
    h.write(m + 0x1c8, base);
    h.write(m + 0x1b4, size);
    h.write(m + 0x1b0, size);
    h.write(m + 0x1cc, size);
    h.write(m + 0x20, 0xfffu32);
    h.write(m + 0x1d8, 0u32);
    h.write(m + 0x1dc, 0u32);
    h.write(m + 0x1b8, flags | 4);
    for i in 0..32u32 {
        let bin = m + 0x28 + 8 * i;
        h.write(bin + 0xc, bin);
        h.write(bin + 8, bin);
    }
    // the chunk after m's: init_top(m, next, base + size - next - 0x28)
    let head: u32 = h.read(m.wrapping_sub(4));
    let next = (head & !7).wrapping_sub(8).wrapping_add(m);
    let rest = size
        .wrapping_sub(0x28)
        .wrapping_add(base.wrapping_sub(next));
    let off = align8(next);
    let top = next.wrapping_add(off);
    let tsize = rest.wrapping_sub(off);
    h.write(m + 0xc, tsize);
    h.write(m + 0x18, top);
    h.write(top.wrapping_add(4), tsize | 1);
    let trim: u32 = h.read(h.rebase(MPARAMS_TRIM_THRESHOLD));
    h.write(tsize.wrapping_add(top).wrapping_add(4), 0x28u32);
    h.write(m + 0x1c, trim);
    m
}

/// `std::_Tree::_Buynode0` copy for the string maps (`0x00654f40`,
/// thiscall on the tree; the same code as `std_Tree_buyNode2c_006a03c0`).
pub const BUY_NODE_2C: u32 = 0x0065_4f40;
/// `std::string::assign(p, n)` (`0x00404270`, thiscall, `ret 8`).
pub const STRING_ASSIGN: u32 = 0x0040_4270;

/// `std::map<std::string, u32>`'s `_Buynode(&pair)` (`0x006e8970`,
/// thiscall on the tree, `ret 4`; `InitIggyHelper` and `0x006e9140`):
/// a 0x2c-byte node (`0x00654f40`), the word `+0xc` (colour, nil) 0,
/// the key at `+0x10` an empty string assigned the C string `[pair]`
/// (`strlen` of it), the value `+0x28` `[pair + 4]`. The node.
pub fn string_map_buy_node(h: &mut impl Host, this: u32, pair: u32) -> u32 {
    let node = h.call_sym(BUY_NODE_2C, Conv::Thiscall(this), &[]);
    h.write(node.wrapping_add(0xc), 0u16);
    let s = node.wrapping_add(0x10);
    if s == 0 {
        return node;
    }
    let p: u32 = h.read(pair);
    h.write(s + 0x14, 0xfu32);
    h.write(s + 0x10, 0u32);
    h.write(s, 0u8);
    let mut len = 0u32;
    while h.read::<u8>(p.wrapping_add(len)) != 0 {
        len += 1;
    }
    h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[p, len]);
    let v: u32 = h.read(pair + 4);
    h.write(s + 0x18, v);
    node
}

/// `g_PhyreScene` (the scene the loads run on).
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// The menu textures' manager global (`MenuTextures_ResetBuffers`'s
/// `this`).
pub const MENU_TEXTURES: u32 = 0x00cc_c81c;
/// `RemapButtonIcon`'s strings for `setButtons` and its layout
/// constants (`f32`, `.rdata`): `0x00b41d80..0x00b41d94`, and the scale
/// `0x00b3eb98` stored at `+0`.
pub const REMAP_TEXT_A: u32 = 0x00b4_1548;
pub const REMAP_TEXT_B: u32 = 0x00b4_1588;
pub const REMAP_LAYOUT: u32 = 0x00b4_1d80;
pub const REMAP_SCALE: u32 = 0x00b3_eb98;
/// `RemapButtonIcon_setLayout_006e8140` (thiscall, eight `f32` words).
pub const REMAP_SET_LAYOUT: u32 = 0x006e_8140;

/// The asset loader's start-up task (`0x00636430`, cdecl; run by
/// `AsyncLoadManager_RegistTaskRequest_run_00691350`, registered by
/// `asyncFFXInitialize`) -> `g_PhyreScene`: the scene's shaders loaded
/// in order (wave, cone, the menu shaders, the character shadow, the
/// optimised font) and its settings applied; the shadow manager
/// initialised, `0x006e55e0` (an empty function) on the
/// `0x00ce5194` object, the character shader renderer made, the effect
/// shaders loaded, the scene's boot set-up, the menu textures and the
/// character shaders, the video index map (Japanese: `english` 0), the UI and the menu
/// textures' buffers reset. Without a remap button icon in the scene
/// (`+0x10de0`): one made (`operator new(0x6c)`, its constructor; 0 when
/// that fails), its buttons set (`setButtons(text a, text b, the skip
/// texture path)`), its layout (eight `f32`s from `.rdata`), `+0` the
/// scale, and it goes to the scene.
pub fn loader_startup_task(h: &mut impl Host) -> u32 {
    let scene_at = h.rebase(PHYRE_SCENE);
    for f in [
        0x0066_3e20u32,
        0x0066_3d40,
        0x0066_53c0,
        0x0066_50f0,
        0x0066_5200,
        0x0066_52e0,
        0x0066_4360,
        0x0066_5010,
        0x0066_54a0,
        0x0066_5580,
        0x0066_f010,
    ] {
        let scene: u32 = h.read(scene_at);
        h.call_sym(f, Conv::Thiscall(scene), &[]);
    }
    let shadow = h.call_sym(0x006d_2390, Conv::Cdecl, &[]);
    h.call_sym(0x006c_e100, Conv::Thiscall(shadow), &[]);
    let g = h.call_sym(0x006e_55d0, Conv::Cdecl, &[]);
    h.call_sym(0x006e_55e0, Conv::Thiscall(g), &[]);
    h.call_sym(0x0069_f3a0, Conv::Cdecl, &[]);
    let vfx = h.call_sym(0x006c_14c0, Conv::Cdecl, &[]);
    h.call_sym(0x006c_0fc0, Conv::Thiscall(vfx), &[]);
    let scene: u32 = h.read(scene_at);
    h.call_sym(0x0065_7a50, Conv::Thiscall(scene), &[]);
    h.call_sym(0x0067_b740, Conv::Cdecl, &[]);
    h.call_sym(0x006d_f3c0, Conv::Cdecl, &[]);
    // createVideoIndexMap(0) (`english` pushed before the getter runs)
    let fmv = h.call_sym(0x006d_73a0, Conv::Cdecl, &[]);
    h.call_sym(0x006d_82f0, Conv::Thiscall(fmv), &[0]);
    h.call_sym(0x0067_b6c0, Conv::Cdecl, &[]);
    let menu: u32 = h.read(h.rebase(MENU_TEXTURES));
    h.call_sym(0x0067_c440, Conv::Thiscall(menu), &[]);
    let scene: u32 = h.read(scene_at);
    if h.read::<u32>(scene + 0x10de0) != 0 {
        return scene;
    }
    let n = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x6c]);
    let icon = if n == 0 {
        0
    } else {
        h.call_sym(0x006e_77d0, Conv::Thiscall(n), &[])
    };
    let path = h.call_sym(0x0063_8e90, Conv::Cdecl, &[]);
    let (a, b) = (h.rebase(REMAP_TEXT_A), h.rebase(REMAP_TEXT_B));
    h.call_sym(0x006e_78e0, Conv::Thiscall(icon), &[a, b, path]);
    let l = h.rebase(REMAP_LAYOUT);
    let mut f = [0u32; 6];
    for (k, w) in f.iter_mut().enumerate() {
        *w = quiet(h.read(l + 4 * k as u32));
    }
    // the words as the original stores them: [esp] .. [esp+0x1c]
    let args = [f[5], f[2], f[1], f[1], f[3], f[2], f[4], f[0]];
    h.call_sym(REMAP_SET_LAYOUT, Conv::Thiscall(icon), &args);
    let s = quiet(h.read(h.rebase(REMAP_SCALE)));
    h.write(icon, s);
    let scene: u32 = h.read(scene_at);
    h.write(scene + 0x10de0, icon);
    scene
}

/// The `_Insert_at` copies of the string-keyed maps: the node count past
/// which they throw (`0x5d1745c`, 0x2c-byte nodes), `_Xlength_error`'s
/// slot and its message (`"map/set<T> too long"`).
pub const STRING_MAP_MAX: u32 = 0x05d1_745c;
pub const XLENGTH_ERROR_IAT: u32 = 0x00b0_c29c;
pub const MAP_TOO_LONG: u32 = 0x00b4_12ac;

/// The overflow path of a string map's `_Insert_at` (`0x006e8a50`,
/// `0x006d5dc0`, `0x0069c6b0`, `0x0064a130`; the count `[this+4]` at
/// [`STRING_MAP_MAX`] or more): the node's key string (at `node +
/// key`) emptied, the node deleted, `_Xlength_error` (it throws). The
/// rest of these copies is `StdTree_insertAt_006993d0`'s code.
pub fn string_node_overflow(h: &mut impl Host, node: u32, key: u32) {
    string_tidy(h, node.wrapping_add(key));
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
    let f: u32 = h.read(h.rebase(XLENGTH_ERROR_IAT));
    let msg = h.rebase(MAP_TOO_LONG);
    h.call(f, Conv::Cdecl, &[msg]);
}

/// The sign of `memcmp(a, b, n)` (unsigned bytes).
fn memcmp_sign(h: &impl Host, a: u32, b: u32, n: u32) -> i32 {
    for i in 0..n {
        let x: u8 = h.read(a.wrapping_add(i));
        let y: u8 = h.read(b.wrapping_add(i));
        if x != y {
            return if x < y { -1 } else { 1 };
        }
    }
    0
}

/// The characters and size of the `std::string` at `s`.
fn string_parts(h: &impl Host, s: u32) -> (u32, u32) {
    let cap: u32 = h.read(s.wrapping_add(0x14));
    let at = if cap >= 16 { h.read(s) } else { s };
    (at, h.read(s.wrapping_add(0x10)))
}

/// `std::string` comparison as the inlined code does it: `memcmp` of
/// the shorter size, then the sizes.
fn string_order(h: &impl Host, a: u32, b: u32) -> i32 {
    let (pa, na) = string_parts(h, a);
    let (pb, nb) = string_parts(h, b);
    match memcmp_sign(h, pa, pb, na.min(nb)) {
        0 if na < nb => -1,
        0 => i32::from(na != nb),
        r => r,
    }
}

/// `std::string::compare(pos, n, p, len)` (`0x006590d0`, thiscall,
/// `ret 0x10`).
pub const STRING_COMPARE: u32 = 0x0065_90d0;

/// The string maps' `_Insert_nohint` copies: address, their
/// `_Insert_at` and their iterator `--`.
pub const STRING_INSERT_NOHINT: [(u32, u32, u32); 3] = [
    (0x006e_8c80, 0x006e_8a50, 0x006e_91f0),
    (0x0064_a360, 0x0064_a130, 0x0064_f360),
    (0x0069_cd40, 0x0069_c6b0, 0x0069_eca0),
];

/// `std::map<std::string, ..>::_Insert_nohint(this; &out, leftish,
/// &value, node)` (`0x006e8c80`, `0x0064a360`, `0x0069cd40`; thiscall,
/// `ret 0x10`): down from the root, left while the value's key (the
/// string at `value`) orders before the node's (`+0x10`; at most it with
/// `leftish`); the predecessor of the place (the iterator `dec` one
/// back, unless the walk ended left of the leftmost) compared with the
/// value (`std::string::compare`); when it is smaller (or the place is
/// the leftmost), `node` is hung there (`insert_at(&it, left, place,
/// value, node)`) and `out` = (it, 1); else `node`'s key emptied, the
/// node deleted, `out` = (the predecessor, 0). `frame`: one word of game
/// memory (the original's argument slot `ebp + 0xc`). Returns `out`.
#[allow(clippy::too_many_arguments)]
pub fn string_insert_nohint<H: Host>(
    h: &mut H,
    this: u32,
    out: u32,
    leftish: u32,
    value: u32,
    node: u32,
    frame: u32,
    insert_at: u32,
    iter_dec: u32,
) -> u32 {
    let head: u32 = h.read(this);
    let mut add_left = true;
    let mut place = head;
    let mut at: u32 = h.read(head.wrapping_add(4));
    while h.read::<u8>(at.wrapping_add(0xd)) == 0 {
        place = at;
        let k = at.wrapping_add(0x10);
        add_left = if leftish & 0xff != 0 {
            string_order(h, k, value) >= 0
        } else {
            string_order(h, value, k) < 0
        };
        at = h.read(at.wrapping_add(if add_left { 0 } else { 8 }));
    }
    h.write(frame, place);
    let hang = |h: &mut H, left: bool| {
        let it = h.call_sym(
            insert_at,
            Conv::Thiscall(this),
            &[frame, u32::from(left), place, value, node],
        );
        let it: u32 = h.read(it);
        h.write(out, it);
        h.write(out.wrapping_add(4), 1u8);
        out
    };
    if add_left {
        if place == h.read::<u32>(head) {
            return hang(h, true);
        }
        h.call_sym(iter_dec, Conv::Thiscall(frame), &[]);
    }
    let before: u32 = h.read(frame);
    let k = before.wrapping_add(0x10);
    let (p, n) = string_parts(h, value);
    let klen: u32 = h.read(k.wrapping_add(0x10));
    let r = h.call_sym(STRING_COMPARE, Conv::Thiscall(k), &[0, klen, p, n]);
    if r.cast_signed() < 0 {
        return hang(h, add_left);
    }
    string_tidy(h, node.wrapping_add(0x10));
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[node]);
    h.write(out, before);
    h.write(out.wrapping_add(4), 0u8);
    out
}

/// Iggy's render-thread command trampoline (`0x00a34b00`, cdecl `(p)`;
/// the callback of `PRenderThread::processRenderCommands`' command 0x2c,
/// `p` the frame pair (context, routine)): a tail jump to the routine
/// `[p + 4]` with the caller's stack, so the routine gets `p` as its one
/// word (its caller pushed only it). The routine's `eax`.
pub fn render_command_trampoline(h: &mut impl Host, p: u32) -> u32 {
    let routine: u32 = h.read(p.wrapping_add(4));
    h.call(routine, Conv::Cdecl, &[p])
}

/// The tweak tools' line texts (`.rdata`): `"init failed"`, the cursor
/// marks `">"`/`" "`, the formats and the words they print.
pub const TXT_INIT_FAILED: u32 = 0x00b4_8244;
pub const TXT_CURSOR: u32 = 0x00b4_8250;
pub const TXT_NO_CURSOR: u32 = 0x00b2_0b30;
pub const FMT_SSS: u32 = 0x00b4_2a3c; // "%s%s%s"
pub const FMT_SSD: u32 = 0x00b4_88e0; // "%s%s%d"
pub const FMT_SS_D: u32 = 0x00b4_8ad0; // "%s%s %d"
pub const FMT_SS_S: u32 = 0x00b4_8de4; // "%s%s %s"
pub const FMT_SS: u32 = 0x00b4_0eb0; // "%s%s"
pub const FMT_S: u32 = 0x00b0_e160; // "%s"
pub const FMT_SECTION: u32 = 0x00b4_8a98; // "%s%s GameSection: %d, EventID: %d, ScenarioFlag: %d, "
pub const FMT_TIME: u32 = 0x00b4_8dec; // "%s%s%03d:%02d:%02d"
pub const FMT_RATE: u32 = 0x00b4_8e00; // "%s%s%d B/s"
pub const TXT_TRUE: u32 = 0x00b3_9230;
pub const TXT_FALSE: u32 = 0x00b3_9238;
pub const TXT_TRUE_CAP: u32 = 0x00b4_8260;
pub const TXT_FALSE_CAP: u32 = 0x00b4_8268;
pub const TXT_KEEP: u32 = 0x00b4_8ddc;
pub const TXT_OFF: u32 = 0x00b4_88dc;
pub const TXT_ON: u32 = 0x00b4_88d8;
pub const TXT_ZOOM: u32 = 0x00b4_8a8c;
pub const TXT_DONT_CHECK: u32 = 0x00b4_8ad8;
pub const TXT_NEED_CHECK: u32 = 0x00b4_8ae4;
pub const TXT_DASHES: u32 = 0x00b4_8c7c;
/// Globals the lines show: `g_ViewerManager`, the battle camera mode,
/// `g_SaveData_CrcCheckOff`, two `GameParamConfig` flags, the string
/// table the first battle line indexes.
pub const VIEWER_MANAGER: u32 = 0x00cc_cac0;
pub const CAMERA_MODE: u32 = 0x0113_6fc0;
pub const CRC_CHECK_OFF: u32 = 0x00cc_b9a4;
pub const PARAM_FLAG_CC: u32 = 0x00cc_cacc;
pub const PARAM_FLAG_B8: u32 = 0x00cc_cab8;
pub const BATTLE_NAMES: u32 = 0x00c3_432c;
/// `GetU32_018763e0`, `GetFullArenaLocalization` (cdecl getters) and
/// the save data's address getter `MsGetSaveEventAddress`.
pub const GET_U32_018763E0: u32 = 0x0090_6730;
pub const GET_FULL_ARENA_LOCALIZATION: u32 = 0x00a4_2fa0;
pub const SAVE_EVENT_ADDRESS: u32 = 0x0078_5240;

/// `sprintf_s(buf, 0x100, fmt, rest..)` through its import; its `eax`.
fn line(h: &mut impl Host, buf: u32, fmt: u32, rest: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(SPRINTF_S_IAT));
    let mut words = vec![buf, 0x100, h.rebase(fmt)];
    words.extend_from_slice(rest);
    h.call(f, Conv::Cdecl, &words)
}

/// `"true"`/`"false"` (or the capitalised pair) by `on`.
fn yes_no(h: &impl Host, on: bool, caps: bool) -> u32 {
    let t = match (on, caps) {
        (true, false) => TXT_TRUE,
        (false, false) => TXT_FALSE,
        (true, true) => TXT_TRUE_CAP,
        (false, true) => TXT_FALSE_CAP,
    };
    h.rebase(t)
}

/// The cursor mark of line `n`: `">"` when the tool's cursor (`+cursor`)
/// is on it, else `" "`.
fn mark(h: &impl Host, this: u32, cursor: u32, n: u32) -> u32 {
    let on = h.read::<u32>(this + cursor) == n;
    h.rebase(if on { TXT_CURSOR } else { TXT_NO_CURSOR })
}

/// `BasicDebugTweakUtil::getLineText(this; n, buf)` (`0x006b69a0`,
/// thiscall, `ret 8`): the tool's line `n` into `buf` (`sprintf_s`, 256
/// bytes): `"init failed"` before its initialisation (the byte `+0x28`
/// 0); else the cursor mark, the line's label (`+0xc4 + 4 n`) and its
/// value: the viewer manager's map names (lines 0 and 2), 0 (line 1),
/// the flags `+0`, `+8`, the viewer's byte `+0x50`, `+4`, the camera mode
/// (off/on/zoom), `+0xc`, the game section (`+0x10`, two 0s), `+0x18`,
/// the CRC check, `+0x14`; lines past 12 write nothing. `eax`:
/// `sprintf_s`'s, `n` for those.
pub fn basic_debug_line<H: Host>(h: &mut H, this: u32, n: u32, buf: u32) -> u32 {
    if h.read::<u8>(this + 0x28) == 0 {
        let t = h.rebase(TXT_INIT_FAILED);
        let f: u32 = h.read(h.rebase(SPRINTF_S_IAT));
        return h.call(f, Conv::Cdecl, &[buf, 0x100, t]);
    }
    let m = mark(h, this, 0xfc, n);
    let label = |h: &H, off: u32| -> u32 { h.read(this + off) };
    let flag = |h: &H, off: u32| -> u32 { yes_no(h, h.read::<u32>(this + off) != 0, false) };
    let viewer: u32 = h.read(h.rebase(VIEWER_MANAGER));
    let (fmt, rest): (u32, Vec<u32>) = match n {
        0 => (FMT_SSS, vec![label(h, 0xc4), viewer.wrapping_add(0x1c)]),
        1 => (FMT_SSD, vec![label(h, 0xc8), 0]),
        2 => (FMT_SSS, vec![label(h, 0xcc), viewer.wrapping_add(0x30)]),
        3 => (FMT_SSS, vec![label(h, 0xd0), flag(h, 0)]),
        4 => (FMT_SSS, vec![label(h, 0xd4), flag(h, 8)]),
        5 => {
            let on = h.read::<u8>(viewer.wrapping_add(0x50)) != 0;
            (FMT_SSS, vec![label(h, 0xd8), yes_no(h, on, false)])
        }
        6 => (FMT_SSS, vec![label(h, 0xdc), flag(h, 4)]),
        7 => {
            let t = match h.read::<u32>(h.rebase(CAMERA_MODE)) {
                0 => TXT_OFF,
                1 => TXT_ON,
                _ => TXT_ZOOM,
            };
            (FMT_SSS, vec![label(h, 0xe0), h.rebase(t)])
        }
        8 => (FMT_SSS, vec![label(h, 0xe4), flag(h, 0xc)]),
        9 => {
            let section: u32 = h.read(this + 0x10);
            (FMT_SECTION, vec![label(h, 0xe8), section, 0, 0])
        }
        10 => {
            let v: u32 = h.read(this + 0x18);
            (FMT_SS_D, vec![label(h, 0xec), v])
        }
        11 => {
            let off = h.read::<u32>(h.rebase(CRC_CHECK_OFF)) != 0;
            let t = h.rebase(if off { TXT_DONT_CHECK } else { TXT_NEED_CHECK });
            (FMT_SSS, vec![label(h, 0xf0), t])
        }
        12 => (FMT_SSS, vec![label(h, 0xf4), flag(h, 0x14)]),
        _ => return n,
    };
    let mut words = vec![m];
    words.extend(rest);
    line(h, buf, fmt, &words)
}

/// The save data's string for `i` (`0x007870d0`, cdecl): `[save + 0x2c
/// + 4 i]`, `i` clamped to 0..7 (signed), `save` from
/// `MsGetSaveEventAddress`.
pub fn save_string(h: &mut impl Host, i: u32) -> u32 {
    let save = h.call_sym(SAVE_EVENT_ADDRESS, Conv::Cdecl, &[]);
    let k = i.cast_signed().clamp(0, 7).cast_unsigned();
    h.read(save.wrapping_add(0x2c + 4 * k))
}

/// `BtlChrParamConfigTweakUitl::getLineText(this; n, buf)` (`0x006b6bb0`,
/// thiscall, `ret 8`): `"init failed"` before the initialisation (the
/// byte `+0x1c` 0); else the cursor mark (`+0xf8`), the label (`+0xb0 +
/// 4 n`) and: the battle string `BATTLE_NAMES[[+0]]` (line 0), the
/// bytes `+4..+7` as true/false (1..4), the words `+0xc`, `+0x10` and the
/// bytes `+8`, `+0x14..+0x1b` as numbers (5..15), the save data's string
/// for `[+0]` printed with `%d` (16; its pointer, as the original);
/// other lines `"------------"`. `sprintf_s`'s `eax`.
pub fn battle_param_line(h: &mut impl Host, this: u32, n: u32, buf: u32) -> u32 {
    if h.read::<u8>(this + 0x1c) == 0 {
        let t = h.rebase(TXT_INIT_FAILED);
        let f: u32 = h.read(h.rebase(SPRINTF_S_IAT));
        return h.call(f, Conv::Cdecl, &[buf, 0x100, t]);
    }
    let m = mark(h, this, 0xf8, n);
    let (fmt, value) = match n {
        0 => {
            let i: u32 = h.read(this);
            let names = h.rebase(BATTLE_NAMES);
            (FMT_SSS, h.read(names.wrapping_add(i.wrapping_mul(4))))
        }
        1..=4 => {
            let on = h.read::<u8>(this + 3 + n) != 0;
            (FMT_SSS, yes_no(h, on, false))
        }
        5 => (FMT_SSD, h.read(this + 0xc)),
        6 => (FMT_SSD, h.read(this + 0x10)),
        7 => (FMT_SSD, u32::from(h.read::<u8>(this + 8))),
        8..=15 => (FMT_SSD, u32::from(h.read::<u8>(this + 0x14 + (n - 8)))),
        16 => {
            let i: u32 = h.read(this);
            (FMT_SSD, save_string(h, i))
        }
        _ => {
            let d = h.rebase(TXT_DASHES);
            return line(h, buf, FMT_S, &[d]);
        }
    };
    let label: u32 = h.read(this + 0xb0 + 4 * n);
    line(h, buf, fmt, &[m, label, value])
}

/// `GameParamConfigTweakUitl::getLineText(this; n, buf)` (`0x006b72e0`,
/// thiscall, `ret 8`): `"init failed"` before the initialisation (the
/// byte `+0x10` 0); else the cursor mark (`+0xe4`), the label (`+0xa4 +
/// 4 n`) and: the bytes `+8`, `+7`, `+5`, `+6`, `+0xa`, `+9` and the
/// word `0x00cccacc` as true/false (0..6), `GetU32_018763e0()`'s
/// -1/0/1 as false/keep/true (7), `GetFullArenaLocalization()` and the
/// byte `0x00cccab8` as True/False (8, 13), `[+0]` (9), the play time
/// `+0xc` as hours:minutes:seconds (10), nothing (11, 12), -1 B/s (14);
/// other lines `"------------"`. `sprintf_s`'s `eax`. Line 7 with any
/// other value read a word of the original's frame beside its
/// three-entry table (no setting holds one: the getter's word is the
/// tri-state); the port shows `"keep"` then.
pub fn game_param_line<H: Host>(h: &mut H, this: u32, n: u32, buf: u32) -> u32 {
    if h.read::<u8>(this + 0x10) == 0 {
        let t = h.rebase(TXT_INIT_FAILED);
        let f: u32 = h.read(h.rebase(SPRINTF_S_IAT));
        return h.call(f, Conv::Cdecl, &[buf, 0x100, t]);
    }
    let m = mark(h, this, 0xe4, n);
    if n > 14 {
        let d = h.rebase(TXT_DASHES);
        return line(h, buf, FMT_S, &[d]);
    }
    let label: u32 = h.read(this + 0xa4 + 4 * n);
    let flag = |h: &H, off: u32| -> u32 { yes_no(h, h.read::<u8>(this + off) != 0, false) };
    match n {
        0..=5 => {
            let v = flag(h, [8, 7, 5, 6, 0xa, 9][n as usize]);
            line(h, buf, FMT_SSS, &[m, label, v])
        }
        6 => {
            let on = h.read::<u32>(h.rebase(PARAM_FLAG_CC)) != 0;
            let v = yes_no(h, on, false);
            line(h, buf, FMT_SSS, &[m, label, v])
        }
        7 => {
            let v = h.call_sym(GET_U32_018763E0, Conv::Cdecl, &[]);
            let t = match v.cast_signed() {
                -1 => TXT_FALSE,
                1 => TXT_TRUE,
                _ => TXT_KEEP,
            };
            let t = h.rebase(t);
            line(h, buf, FMT_SSS, &[m, label, t])
        }
        8 => {
            let on = h.call_sym(GET_FULL_ARENA_LOCALIZATION, Conv::Cdecl, &[]) != 0;
            let v = yes_no(h, on, true);
            line(h, buf, FMT_SS_S, &[m, label, v])
        }
        9 => {
            let v: u32 = h.read(this);
            line(h, buf, FMT_SSD, &[m, label, v])
        }
        10 => {
            let t: u32 = h.read(this + 0xc);
            let (s, minutes) = (t % 60, t / 60);
            line(h, buf, FMT_TIME, &[m, label, minutes / 60, minutes % 60, s])
        }
        11 | 12 => line(h, buf, FMT_SS, &[m, label]),
        13 => {
            let on = h.read::<u8>(h.rebase(PARAM_FLAG_B8)) != 0;
            let v = yes_no(h, on, true);
            line(h, buf, FMT_SS_S, &[m, label, v])
        }
        _ => line(h, buf, FMT_RATE, &[m, label, u32::MAX]),
    }
}

/// GDraw's context (`gdraw_createContext_00a35780` stores it).
pub const GDRAW_CONTEXT: u32 = 0x01a8_58b4;

/// The two draw lists' reset in [`gdraw_reset_frame`]: flags `+0xc`
/// bits 1 and 2 cleared, `+0x198` = the old `+0x1a0`, `+0x19c` = the old
/// `+0x1a4`, `+0x1a0`/`+0x1a4` the context's two words. The old `+0x1a4`.
fn gdraw_reset_list(h: &mut impl Host, list: u32, a: u32, b: u32) -> u32 {
    let old_a: u32 = h.read(list + 0x1a0);
    let f: u32 = h.read(list + 0xc);
    h.write(list + 0xc, f & !6);
    h.write(list + 0x198, old_a);
    let old_b: u32 = h.read(list + 0x1a4);
    h.write(list + 0x1a4, b);
    h.write(list + 0x19c, old_b);
    h.write(list + 0x1a0, a);
    old_b
}

/// GDraw's frame reset (`0x00a37fe0`, cdecl; Iggy's render command run
/// through `IggyRenderCommand_trampoline_00a34b00` on the render
/// thread): the device context's `ClearState` (`ID3D11DeviceContext`
/// slot 110, `[ctx+4]`), then on the context: `+0xab8` 1, `+0x9bc` the
/// old `+0x9b8`, `+0x9b8`, `+0x9a4`, `+0x9b4` 0, and its two draw lists
/// (`+0xab0`, then `+0xab4`) reset to the words `+0xac0`/`+0xac4` (read
/// once, before). `eax`: the second list's old `+0x1a4`.
pub fn gdraw_reset_frame(h: &mut impl Host) -> u32 {
    let g = h.rebase(GDRAW_CONTEXT);
    let ctx: u32 = h.read(g);
    let dc: u32 = h.read(ctx + 4);
    let vt: u32 = h.read(dc);
    let f: u32 = h.read(vt + 0x1b8);
    h.call(f, Conv::Stdcall, &[dc]);
    let ctx: u32 = h.read(g);
    let last: u32 = h.read(ctx + 0x9b8);
    let first: u32 = h.read(ctx + 0xab0);
    let b: u32 = h.read(ctx + 0xac4);
    let a: u32 = h.read(ctx + 0xac0);
    h.write(ctx + 0xab8, 1u32);
    h.write(ctx + 0x9bc, last);
    for off in [0x9b8, 0x9a4, 0x9b4] {
        h.write(ctx + off, 0u32);
    }
    gdraw_reset_list(h, first, a, b);
    let second: u32 = h.read(ctx + 0xab4);
    gdraw_reset_list(h, second, a, b)
}

/// `StdList_buyNode_00686bb0` (stdcall `(next, prev)`): a 0x24-byte list
/// node; `StdString_assignSubstr_00403a10` (thiscall `(src, pos, n)`).
pub const LIST_BUY_NODE_24: u32 = 0x0068_6bb0;
pub const STRING_ASSIGN_SUBSTR: u32 = 0x0040_3a10;

/// `std::list<{std::string, u32}>::_Buynode(this; next, prev, &value)`
/// (`0x00680490`, thiscall, `ret 0xc`; `NameList_pushBack_00681ad0` and
/// two more): a node from `StdList_buyNode_00686bb0(next, prev)`, its
/// value at `+8` copy-constructed from `value` (an empty string assigned
/// all of `value`'s, then the word `+0x18`). The node.
pub fn name_list_buy_node(h: &mut impl Host, next: u32, prev: u32, value: u32) -> u32 {
    let node = h.call_sym(LIST_BUY_NODE_24, Conv::Stdcall, &[next, prev]);
    let s = node.wrapping_add(8);
    if s == 0 {
        return node;
    }
    h.write(s + 0x14, 0xfu32);
    h.write(s + 0x10, 0u32);
    h.write(s, 0u8);
    h.call_sym(
        STRING_ASSIGN_SUBSTR,
        Conv::Thiscall(s),
        &[value, 0, u32::MAX],
    );
    let w: u32 = h.read(value + 0x18);
    h.write(s + 0x18, w);
    node
}

/// The callees of [`button_material`].
pub const CLUSTER_MANAGER_GET: u32 = 0x0069_a1a0;
pub const CLUSTER_LOAD: u32 = 0x0069_b940;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
pub const CLUSTER_ITER_SEEK: u32 = 0x0046_4f30;
pub const SCENE_CURSOR_SEEK4: u32 = 0x0065_ecb0;
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
pub const LINEAR_SAMPLER: u32 = 0x0065_e050;
pub const PARAM_BY_NAME: u32 = 0x0056_cbf0;
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
pub const MEMCPY: u32 = 0x0094_9256;
/// The class descriptors the two cursors look for (`PTexture2D`, the
/// effect), the parameter names (`.rdata`).
pub const TEXTURE_2D_DESCRIPTOR: u32 = 0x00ca_37d0;
pub const EFFECT_DESCRIPTOR: u32 = 0x00ca_5268;
pub const PARAM_TEXTURE: u32 = 0x00b3_017c;
pub const PARAM_COLOUR: u32 = 0x00b4_7a48;

/// A cluster cursor in the frame at `at` (8 words): five 0s, the list
/// head `list`, its first object (0 when the list is empty: the head
/// points at itself), the class descriptor `desc`.
fn cursor(h: &mut impl Host, at: u32, list: u32, desc: u32) {
    let first: u32 = h.read(list);
    let first = if first == list { 0 } else { first };
    for k in 0..5 {
        h.write(at + 4 * k, 0u32);
    }
    h.write(at + 0x14, list);
    h.write(at + 0x18, first);
    h.write(at + 0x1c, h.rebase(desc));
}

/// `RemapButtonIcon`'s button material (`0x006e8390`, `(name, &cluster,
/// &texture, &material, &instance)` callee pops 0x14, `ecx` unread; from
/// `RemapButtonIcon_setButtons_006e78e0`). `frame`: 0x30 bytes of game
/// memory, the original's `ebp - 0x30` (its cursor and texture
/// descriptor live there; the callees get pointers into it). The cluster
/// `name` loaded (`ClusterManager::loadPCluster`) into `*cluster` and,
/// when there is one, the clusters fixed up (the application asked
/// first); `*texture` 0, then the cluster's first `PTexture2D` (a cursor
/// over its list `+0x1c`) when found. Without a material yet: a cursor
/// over the scene's effect cluster (`[g_PhyreScene + 0xfa84]`) finds the
/// effect, whose `createMaterial(cluster, 0, 0)` gives `*material`, and
/// `*instance` its first word. Then the texture descriptor (0x42, 0, the
/// scene's linear sampler 2, the texture) is set on the material's
/// parameter buffer (`+4`) for the shader parameter named at
/// `0x00b3017c`, and the four floats 1.0 copied into the buffer at the
/// parameter named at `0x00b47a48` when it fits (its offset `+8` at
/// least 4, offset plus size `+0xa & 0x1fff` within the buffer's size).
/// `eax`: `memcpy`'s, or offset plus size when it doesn't fit.
pub fn button_material(
    h: &mut impl Host,
    frame: u32,
    name: u32,
    cluster: u32,
    texture: u32,
    material: u32,
    instance: u32,
) -> u32 {
    let ebp = frame + 0x30;
    let cm = h.call_sym(CLUSTER_MANAGER_GET, Conv::Cdecl, &[]);
    let cl = h.call_sym(CLUSTER_LOAD, Conv::Thiscall(cm), &[name]);
    h.write(cluster, cl);
    if cl != 0 {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[cluster, 1]);
    }
    h.write(texture, 0u32);
    let cl: u32 = h.read(cluster);
    let it = ebp - 0x20;
    if cl != 0 {
        cursor(h, it, cl.wrapping_add(0x1c), TEXTURE_2D_DESCRIPTOR);
        h.call_sym(CLUSTER_ITER_SEEK, Conv::Thiscall(it), &[]);
        if h.read::<u32>(ebp - 0x14) != 0 {
            let a: u32 = h.read(ebp - 0x10);
            let b: u32 = h.read(ebp - 0x20);
            h.write(texture, a.wrapping_add(b));
        }
    }
    let scene_at = h.rebase(PHYRE_SCENE);
    if h.read::<u32>(material) == 0 {
        let scene: u32 = h.read(scene_at);
        let effects: u32 = h.read(scene + 0xfa84);
        cursor(h, it, effects.wrapping_add(0x1c), EFFECT_DESCRIPTOR);
        h.call_sym(SCENE_CURSOR_SEEK4, Conv::Thiscall(it), &[]);
        let a: u32 = h.read(ebp - 0x20);
        let b: u32 = h.read(ebp - 0x10);
        let effect: u32 = h.read(b.wrapping_add(a));
        let scene: u32 = h.read(scene_at);
        let effects: u32 = h.read(scene + 0xfa84);
        let mat = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(effect), &[effects, 0, 0]);
        h.write(material, mat);
        let first: u32 = h.read(mat);
        h.write(instance, first);
    }
    let tex: u32 = h.read(texture);
    let scene: u32 = h.read(scene_at);
    let desc = ebp - 0x10;
    h.write(desc, 0x42u32);
    h.write(ebp - 4, tex);
    let sampler = h.call_sym(LINEAR_SAMPLER, Conv::Thiscall(scene), &[2]);
    for k in 0..4 {
        h.write(frame + 4 * k, 1.0f32.to_bits());
    }
    h.write(ebp - 8, sampler);
    h.write(ebp - 0xc, 0u32);
    let shader: u32 = h.read(instance);
    let n = h.rebase(PARAM_TEXTURE);
    let p = h.call_sym(PARAM_BY_NAME, Conv::Thiscall(shader), &[n]);
    let mat: u32 = h.read(material);
    let buffer: u32 = h.read(mat + 4);
    h.call_sym(SET_TEXTURE_CHECKED, Conv::Thiscall(buffer), &[p, desc]);
    let mat: u32 = h.read(material);
    let shader: u32 = h.read(instance);
    let buffer: u32 = h.read(mat + 4);
    let n = h.rebase(PARAM_COLOUR);
    let q = h.call_sym(PARAM_BY_NAME, Conv::Thiscall(shader), &[n]);
    let off = u32::from(h.read::<u16>(q + 8));
    let size = u32::from(h.read::<u16>(q + 0xa) & 0x1fff);
    let end = off + size;
    if end > h.read::<u32>(buffer) || off < 4 {
        return end;
    }
    h.call_sym(
        MEMCPY,
        Conv::Cdecl,
        &[buffer.wrapping_add(off), frame, size],
    )
}

/// `SkipPrompt_buildQuad_006e7940` (stdcall: four `f32`s, mesh,
/// instance, materials, material).
pub const BUILD_QUAD: u32 = 0x006e_7940;

/// `RemapButtonIcon_setLayout_006e8140(this; a[4], b[4])` (thiscall, eight
/// `f32` words, `ret 0x20`): the three button quads built: the first two
/// (slots `+0x18` and `+0x34`) at `a`, the third (`+0x50`) at `b`; each
/// gets its slot's mesh (`+0x14`), instance (`+0x18`), materials (`+4`)
/// and material word (`+0`) as `buildQuad`'s last four words. The floats
/// pass as `fld dword`/`fstp dword` copy them (a signalling NaN quiet).
/// The last `buildQuad`'s `eax`.
pub fn remap_set_layout(h: &mut impl Host, this: u32, a: [u32; 4], b: [u32; 4]) -> u32 {
    let mut r = 0;
    for (slot, q) in [(0x18u32, a), (0x34, a), (0x50, b)] {
        let mat: u32 = h.read(this + slot);
        let mut w: Vec<u32> = q.iter().map(|&x| quiet(x)).collect();
        w.extend([this + slot + 0x14, this + slot + 0x18, this + slot + 4, mat]);
        r = h.call_sym(BUILD_QUAD, Conv::Stdcall, &w);
    }
    r
}

/// `"MapUnload"`, `"%s_MapUnload"` (`.rdata`).
pub const TXT_MAP_UNLOAD: u32 = 0x00b4_a5ec;
pub const FMT_MAP_UNLOAD: u32 = 0x00b4_a5f8;

/// The memory report's map-unload line (`0x006dcdc0`, thiscall on the
/// report; `PhyreScene_fieldMapUnload`): when the bytes `+0` and `+1`
/// are set, a zeroed 0x100-byte buffer (`frame`, game memory, the
/// original's `ebp - 0x104`) gets `"MapUnload"` when the 0x100 bytes at
/// `+0xc` (the map's name) are all 0, else `"%s_MapUnload"` with that
/// name (`sprintf_s`); the buffer is not used further (the report line
/// was left out of the release). `eax`: `sprintf_s`'s; 0 when the report
/// is off (the original's is its cookie word there; no caller reads it).
pub fn report_map_unload(h: &mut impl Host, this: u32, frame: u32) -> u32 {
    if h.read::<u8>(this) == 0 || h.read::<u8>(this + 1) == 0 {
        return 0;
    }
    h.call_sym(MEMSET, Conv::Cdecl, &[frame, 0, 0x100]);
    let mut name = [0u8; 0x100];
    h.read_bytes(this + 0xc, &mut name);
    let f: u32 = h.read(h.rebase(SPRINTF_S_IAT));
    if name.iter().all(|&b| b == 0) {
        let t = h.rebase(TXT_MAP_UNLOAD);
        h.call(f, Conv::Cdecl, &[frame, 0x100, t, this + 0xc])
    } else {
        let t = h.rebase(FMT_MAP_UNLOAD);
        h.call(f, Conv::Cdecl, &[frame, 0x100, t, this + 0xc])
    }
}

/// `Phyre_PRedBlackTree_remove` (thiscall `(tree; node)`) and
/// `Phyre_PFreeList_reset` (thiscall).
pub const RB_TREE_REMOVE: u32 = 0x0057_5000;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;

/// A Phyre red-black tree of owned buffers emptied (`0x00636970`,
/// thiscall; `PhyreScene_releaseVFXMeshes`, `TreeSet_dtor_00633f70` and
/// three more): while the tree's first node (`+0xc`; none when it is the
/// tree itself) exists, it is removed (`PRedBlackTree::remove`) and its
/// two buffers `+0x10` and `+0xc` freed (`PhyreFree`) unless their bit 0
/// (borrowed) is set; then the node free list at `+0x18` reset. `eax`:
/// the reset's.
pub fn rb_tree_release(h: &mut impl Host, this: u32) -> u32 {
    loop {
        let first: u32 = h.read(this + 0xc);
        if first == this || first == 0 {
            break;
        }
        h.call_sym(RB_TREE_REMOVE, Conv::Thiscall(this), &[first]);
        for off in [0x10u32, 0xc] {
            let p: u32 = h.read(first + off);
            if p & 1 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
            }
        }
    }
    h.call_sym(FREE_LIST_RESET, Conv::Thiscall(this + 0x18), &[])
}
