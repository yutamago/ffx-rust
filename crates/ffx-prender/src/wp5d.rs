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
