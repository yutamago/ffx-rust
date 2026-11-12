//! Win32 stand-ins the boot reaches past the D3D11 device (the loader
//! and the job threads' start-up): deterministic and platform-free, as
//! the rest of [`crate::win32`] (docs/host.md, "Imports").

use ffx_bridge::Mem as _;

use crate::exec::Exec;
use crate::imports::ImportFn;
use crate::table::{Args, Conv, Ret};

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn one(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(1)
}

/// `wsprintfA/W(buffer, format, ...)`: `USER32`'s own formatter (no
/// floats; `%s %c %d %i %u %x %X %p`, flags `-` and `0`, a width, `l`/`h`
/// sizes, at most 1024 characters with the NUL). The variable arguments
/// are read from the caller's stack (`Args::slots`).
#[allow(clippy::too_many_lines, clippy::many_single_char_names)] // one formatter
fn wsprintf(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let mem = e.mem();
    let unit = if wide { 2 } else { 1 };
    let ch = |at: u32| -> u32 {
        if wide {
            u32::from(mem.read::<u16>(at))
        } else {
            u32::from(mem.read::<u8>(at))
        }
    };
    let (buf, mut fmt) = (a(args, 0), a(args, 1));
    let mut next = args.slots + 8;
    let mut arg = || {
        let v: u32 = mem.read(next);
        next += 4;
        v
    };
    let mut out: Vec<u32> = Vec::new();
    loop {
        let c = ch(fmt);
        fmt += unit;
        if c == 0 {
            break;
        }
        if c != u32::from(b'%') {
            out.push(c);
            continue;
        }
        let (mut left, mut zero, mut width) = (false, false, 0usize);
        let mut c = ch(fmt);
        fmt += unit;
        loop {
            match u8::try_from(c).unwrap_or(0) {
                b'-' => left = true,
                b'0' => zero = true,
                b'#' => {}
                _ => break,
            }
            c = ch(fmt);
            fmt += unit;
        }
        while let Some(d) = char::from_u32(c).and_then(|x| x.to_digit(10)) {
            width = width * 10 + d as usize;
            c = ch(fmt);
            fmt += unit;
        }
        // `wsprintf`'s precision only limits strings; sizes: `l`, `h`
        let mut narrow_arg = !wide;
        while matches!(u8::try_from(c).unwrap_or(0), b'l' | b'h' | b'.') {
            if c == u32::from(b'h') {
                narrow_arg = true;
            }
            if c == u32::from(b'l') {
                narrow_arg = false;
            }
            c = ch(fmt);
            fmt += unit;
        }
        let text: Vec<u32> = match u8::try_from(c).unwrap_or(0) {
            b's' => {
                let mut p = arg();
                let mut v = Vec::new();
                while p != 0 && v.len() < 1024 {
                    let x = if narrow_arg {
                        u32::from(mem.read::<u8>(p))
                    } else {
                        u32::from(mem.read::<u16>(p))
                    };
                    if x == 0 {
                        break;
                    }
                    v.push(x);
                    p += if narrow_arg { 1 } else { 2 };
                }
                v
            }
            b'c' => vec![arg() & 0xffff],
            b'd' | b'i' => i32::from_ne_bytes(arg().to_ne_bytes())
                .to_string()
                .bytes()
                .map(u32::from)
                .collect(),
            b'u' => arg().to_string().bytes().map(u32::from).collect(),
            b'x' => format!("{:x}", arg()).bytes().map(u32::from).collect(),
            b'X' | b'p' => format!("{:X}", arg()).bytes().map(u32::from).collect(),
            0 => break,
            _ => vec![c],
        };
        let pad = width.saturating_sub(text.len());
        let fill = if zero && !left {
            u32::from(b'0')
        } else {
            u32::from(b' ')
        };
        if !left {
            out.extend(std::iter::repeat_n(fill, pad));
        }
        out.extend(text);
        if left {
            out.extend(std::iter::repeat_n(u32::from(b' '), pad));
        }
    }
    out.truncate(1023);
    let mut m = e.mem();
    let mut at = buf;
    for &c in out.iter().chain(&[0]) {
        if wide {
            m.write(at, u16::try_from(c).unwrap_or(u16::MAX));
        } else {
            m.write(at, u8::try_from(c).unwrap_or(b'?'));
        }
        at += unit;
    }
    Ret::eax(u32::try_from(out.len()).unwrap_or(0))
}

/// `IDirectInput8A::EnumDevices(this, class, callback, ref, flags)`: the
/// system mouse and keyboard every PC has (`DIDEVICEINSTANCEA`, 0x244
/// bytes: `dwDevType` `+0x24` 0x12 mouse, 0x13 keyboard; the game's
/// callback `0x00623330` makes its Phyre input devices from them), each
/// handed to `callback(instance, ref)` (stdcall) while it answers
/// `DIENUM_CONTINUE`. `class` 0 (all), 2 (pointers) or 3 (keyboards).
pub fn dinput_enum_devices(e: &mut Exec, args: &Args) -> Ret {
    let w = |i: usize| args.stack.get(i).copied().unwrap_or(0);
    let (class, callback, user) = (w(1), w(2), w(3));
    let inst = e.alloc(0x244);
    if inst == 0 || callback == 0 {
        return Ret::eax(0x8000_4005);
    }
    // GUID_SysMouse / GUID_SysKeyboard: {6F1D2B60/61-D5A0-11CF-BFC7-444553540000}
    let devices: [(u32, u32, u8, &[u8]); 2] = [
        (2, 0x6f1d_2b60, 0x12, b"Mouse"),
        (3, 0x6f1d_2b61, 0x13, b"Keyboard"),
    ];
    for (c, guid, kind, label) in devices {
        if class != 0 && class != c {
            continue;
        }
        let mut mem = e.mem();
        mem.write_bytes(inst, &[0u8; 0x244]);
        mem.write(inst, 0x244u32);
        for at in [inst + 4, inst + 0x14] {
            mem.write(at, guid);
            mem.write(at + 4, 0x11cf_d5a0u32);
            mem.write_bytes(at + 8, &[0xbf, 0xc7, 0x44, 0x45, 0x53, 0x54, 0x00, 0x00]);
        }
        mem.write(inst + 0x24, u32::from(kind) | (1 << 8));
        mem.write_bytes(inst + 0x28, label);
        mem.write_bytes(inst + 0x12c, label);
        match e.call_stdcall(callback, &[inst, user]) {
            Ok(1) => {}
            _ => break,
        }
    }
    e.release(inst);
    Ret::eax(0)
}

/// `GetCursorPos(&pt)`: far outside the window, where the harness's
/// background games keep the pointer the game reads (docs/harness.md, "The
/// mouse in background games"), so no menu sees a hover.
fn cursor_pos(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write(a(args, 0), [-32108i32, -32131]);
    Ret::eax(1)
}

/// `GetWindowInfo(hwnd, &info)` / `GetClipCursor(&rect)`: the fake
/// window, 1920 x 1080 at the origin (as `GetClientRect`).
fn window_info(e: &mut Exec, args: &Args) -> Ret {
    let info = a(args, 1);
    let mut mem = e.mem();
    mem.write_bytes(info + 4, &[0u8; 0x38]);
    mem.write(info + 4, [0u32, 0, 1920, 1080]);
    mem.write(info + 0x14, [0u32, 0, 1920, 1080]);
    // WS_VISIBLE | WS_POPUP, active
    mem.write(info + 0x24, 0x9000_0000u32);
    mem.write(info + 0x2c, 1u32);
    Ret::eax(1)
}

fn clip_cursor_rect(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write(a(args, 0), [0u32, 0, 1920, 1080]);
    Ret::eax(1)
}

/// `FileTimeToSystemTime(&ft, &st)`: the calendar date of a FILETIME
/// (100 ns since 1601-01-01, UTC; proleptic Gregorian).
fn file_time_to_system_time(e: &mut Exec, args: &Args) -> Ret {
    let ft: u64 = e.mem().read(a(args, 0));
    let ms_all = ft / 10_000;
    let days = ms_all / 86_400_000;
    let ms_day = ms_all % 86_400_000;
    // days since 1601-01-01 (a Monday) to y/m/d
    let civil = |days: u64| -> (u64, u64, u64) {
        // days from 0000-03-01: 1601-01-01 is 134,774 days before
        // 1970-01-01, which is day 719,468
        let shifted = days + 584_694;
        let era = shifted / 146_097;
        let doe = shifted % 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + u64::from(month <= 2);
        (year, month, day)
    };
    let (year, month, day) = civil(days);
    let weekday = (days + 1) % 7;
    #[allow(clippy::cast_possible_truncation)] // calendar fields
    let st = [
        year as u16,
        month as u16,
        weekday as u16,
        day as u16,
        (ms_day / 3_600_000) as u16,
        (ms_day / 60_000 % 60) as u16,
        (ms_day / 1000 % 60) as u16,
        (ms_day % 1000) as u16,
    ];
    e.mem().write(a(args, 1), st);
    Ret::eax(1)
}

/// `SystemTimeToTzSpecificLocalTime(tz, &utc, &local)`: UTC (the run's
/// time zone).
fn tz_local_time(e: &mut Exec, args: &Args) -> Ret {
    let st: [u16; 8] = e.mem().read(a(args, 1));
    e.mem().write(a(args, 2), st);
    Ret::eax(1)
}

/// `GetTimeZoneInformation(&tzi)`: UTC, no daylight time.
fn time_zone(e: &mut Exec, args: &Args) -> Ret {
    e.mem().write_bytes(a(args, 0), &[0u8; 172]);
    Ret::eax(0)
}

/// `GetFileTime(h, &created, &accessed, &written)`: the file's own times
/// ([`crate::files::Files::times`]; the save list sorts by them), the
/// fixed clock for a handle that is no file.
fn file_time(e: &mut Exec, args: &Args) -> Ret {
    let times = e
        .files()
        .times(a(args, 0))
        .unwrap_or([0x01d0_0000_0000_0000; 3]);
    for (i, t) in (1..4).zip(times) {
        let p = a(args, i);
        if p != 0 {
            e.mem().write(p, t);
        }
    }
    Ret::eax(1)
}

/// `XInputGetState` / `XInputSetState`: no controller
/// (`ERROR_DEVICE_NOT_CONNECTED`).
fn no_controller(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0x48f)
}

fn ret0(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

fn ret1(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(1)
}

/// `GetSystemTime(&st)`: the fixed clock of `GetSystemTimeAsFileTime`
/// (FILETIME `0x01d0_0000_0000_0000`), 2014-11-14 11:41:59.374, a Friday.
fn system_time(e: &mut Exec, args: &Args) -> Ret {
    e.mem()
        .write(a(args, 0), [2014u16, 11, 5, 14, 11, 41, 59, 374]);
    Ret::VOID
}

// ---- OLEAUT32's BSTRs (WP7, the free play's crash) ----------------------

/// A `BSTR` of `len` UTF-16 units on the arena heap: the byte length in
/// the word before the pointer, the units (copied from `from` when it
/// isn't 0, else zeros), a NUL unit after them. 0 when the heap is full
/// or `len` doesn't fit.
fn alloc_bstr(e: &mut Exec, from: u32, len: u32) -> u32 {
    let Some(bytes) = len.checked_mul(2).filter(|&b| b <= 0x7fff_fff0) else {
        return 0;
    };
    let block = e.alloc(4 + bytes + 2);
    if block == 0 {
        return 0;
    }
    let mut mem = e.mem();
    let mut units = vec![0u8; bytes as usize];
    if from != 0 {
        mem.read_bytes(from, &mut units);
    }
    mem.write(block, bytes);
    mem.write_bytes(block + 4, &units);
    mem.write(block + 4 + bytes, 0u16);
    block + 4
}

/// `SysAllocString(s)` (`OLEAUT32` ordinal 2, stdcall): a `BSTR` copy of
/// the NUL-terminated `s`; 0 for 0. `Input_IsXInputDevice`'s WMI names.
fn sys_alloc_string(e: &mut Exec, args: &Args) -> Ret {
    let s = a(args, 0);
    if s == 0 {
        return Ret::eax(0);
    }
    let mem = e.mem();
    let mut len = 0u32;
    while len < 0x1000_0000 && mem.read::<u16>(s + 2 * len) != 0 {
        len += 1;
    }
    Ret::eax(alloc_bstr(e, s, len))
}

/// `SysAllocStringLen(s, len)` (`OLEAUT32` ordinal 4, stdcall): a `BSTR`
/// of `len` units copied from `s` (zeros for 0). The resource strings of
/// `ComBstr_loadString_0061b330`, `BigFileStream_fileTaskUncompress`'s
/// and `Game_Main`'s error boxes.
fn sys_alloc_string_len(e: &mut Exec, args: &Args) -> Ret {
    Ret::eax(alloc_bstr(e, a(args, 0), a(args, 1)))
}

/// `SysFreeString(s)` (`OLEAUT32` ordinal 6, stdcall): the block back to
/// the heap; nothing for 0. The free play's crash (2026-11-11) was the
/// first one, `BigFileStream_fileTaskUncompress`'s `SysFreeString(0)`
/// before its error box.
fn sys_free_string(e: &mut Exec, args: &Args) -> Ret {
    let s = a(args, 0);
    if s != 0 {
        e.release(s - 4);
    }
    Ret::VOID
}

// ---- COM: no WMI here -----------------------------------------------------

/// `CoCreateInstance(clsid, outer, context, iid, &out)`: no class is
/// registered (`REGDB_E_CLASSNOTREG`, `*out` = 0). The only caller is
/// `Input_IsXInputDevice` (`CLSID_WbemLocator`), which then answers "not
/// an `XInput` device"; the host's `EnumDevices` offers no game controller,
/// so nothing reaches it yet.
fn co_create_instance(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 4) != 0 {
        let mut mem = e.mem();
        mem.write(a(args, 4), 0u32);
    }
    Ret::eax(0x8004_0154)
}

// ---- resources (crate::rsrc) ----------------------------------------------

/// `FindResourceW(module, name, type)`.
fn find_resource_w(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    Ret::eax(crate::rsrc::find(
        &mem,
        a(args, 0),
        a(args, 2),
        a(args, 1),
        None,
    ))
}

/// `FindResourceExW(module, type, name, language)`.
fn find_resource_ex_w(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let lang = a(args, 3) & 0xffff;
    Ret::eax(crate::rsrc::find(
        &mem,
        a(args, 0),
        a(args, 1),
        a(args, 2),
        Some(lang),
    ))
}

/// `LoadResource(module, res)`.
fn load_resource(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    Ret::eax(crate::rsrc::load(&mem, a(args, 0), a(args, 1)))
}

/// `SizeofResource(module, res)`.
fn sizeof_resource(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    Ret::eax(crate::rsrc::size(&mem, a(args, 1)))
}

// ---- the window's last calls ----------------------------------------------

/// `ShellExecuteW(hwnd, verb, file, params, dir, show)`: noted, not run;
/// "success" (42, above 32). The window's `WM_CLOSE`
/// (`Phyre_PFramework_PWindowWin32Base_WndProc`) starts the Steam
/// launcher, `<game folder>\FFX&X-2_LAUNCHER.exe`, before it destroys
/// the window: from `ffx-rust` that would start `FFX.exe` again.
fn shell_execute_w(e: &mut Exec, args: &Args) -> Ret {
    let file = read_wide(e, a(args, 2));
    crate::log::note(&format!("ShellExecuteW {file:?}: not started (ffx-rust)"));
    Ret::eax(42)
}

/// `ChangeDisplaySettingsA(mode, flags)`: noted, `DISP_CHANGE_SUCCESSFUL`.
/// The display mode is not changed (the harness's games run beside the
/// user's desktop); "Full Screen Mode" is then a window of that size.
fn change_display_settings_a(e: &mut Exec, args: &Args) -> Ret {
    let mode = a(args, 0);
    let what = if mode == 0 {
        "the registry's mode".to_string()
    } else {
        let mem = e.mem();
        let (w, h): (u32, u32) = (mem.read(mode + 0x6c), mem.read(mode + 0x70));
        format!("{w}x{h}")
    };
    crate::log::note(&format!(
        "ChangeDisplaySettingsA({what}, {:#x}): not applied",
        a(args, 1)
    ));
    Ret::eax(0)
}

/// `IggyGDrawSendWarning(iggy, format, ...)` (cdecl): `GDraw`'s warning
/// on a failed `D3D11` call or a full pool, noted.
fn iggy_gdraw_warning(e: &mut Exec, args: &Args) -> Ret {
    let mem = e.mem();
    let at = a(args, 1);
    let mut text = Vec::new();
    while at != 0 && text.len() < 256 {
        let c: u8 = mem.read(at + u32::try_from(text.len()).unwrap_or(0));
        if c == 0 {
            break;
        }
        text.push(c);
    }
    crate::log::note(&format!(
        "iggy: GDraw warning {:?}",
        String::from_utf8_lossy(&text)
    ));
    Ret::VOID
}

/// A NUL-terminated UTF-16 string (at most 520 units).
fn read_wide(e: &Exec, at: u32) -> String {
    let mem = e.mem();
    let mut s = Vec::new();
    while at != 0 && s.len() < 520 {
        let c: u16 = mem.read(at + 2 * u32::try_from(s.len()).unwrap_or(0));
        if c == 0 {
            break;
        }
        s.push(c);
    }
    String::from_utf16_lossy(&s)
}

const K32: &str = "KERNEL32.dll";
const U32: &str = "USER32.dll";
const OLEAUT: &str = "OLEAUT32.dll";

/// The stand-ins: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    // the C runtime's file names: ANSI code page
    (K32, "AreFileApisANSI", Conv::Stdcall, 0, one),
    (K32, "GetSystemTime", Conv::Stdcall, 1, system_time),
    (U32, "GetCursorPos", Conv::Stdcall, 1, cursor_pos),
    (U32, "GetWindowInfo", Conv::Stdcall, 2, window_info),
    (U32, "GetClipCursor", Conv::Stdcall, 1, clip_cursor_rect),
    (U32, "TranslateMessage", Conv::Stdcall, 1, ret1),
    (U32, "DispatchMessageA", Conv::Stdcall, 1, ret0),
    (U32, "DispatchMessageW", Conv::Stdcall, 1, ret0),
    (U32, "DefWindowProcA", Conv::Stdcall, 4, ret0),
    (U32, "SendMessageA", Conv::Stdcall, 4, ret0),
    (U32, "SendMessageW", Conv::Stdcall, 4, ret0),
    (U32, "PostMessageA", Conv::Stdcall, 4, ret1),
    (U32, "ReleaseDC", Conv::Stdcall, 2, ret1),
    (
        K32,
        "FileTimeToSystemTime",
        Conv::Stdcall,
        2,
        file_time_to_system_time,
    ),
    (
        K32,
        "SystemTimeToTzSpecificLocalTime",
        Conv::Stdcall,
        3,
        tz_local_time,
    ),
    (K32, "GetTimeZoneInformation", Conv::Stdcall, 1, time_zone),
    (K32, "GetFileTime", Conv::Stdcall, 4, file_time),
    (K32, "SetEndOfFile", Conv::Stdcall, 1, ret1),
    (K32, "FormatMessageA", Conv::Stdcall, 7, ret0),
    (K32, "HeapDestroy", Conv::Stdcall, 1, ret1),
    (
        "XINPUT9_1_0.dll",
        "XInputGetState",
        Conv::Stdcall,
        2,
        no_controller,
    ),
    (
        "XINPUT9_1_0.dll",
        "XInputSetState",
        Conv::Stdcall,
        2,
        no_controller,
    ),
    (U32, "wsprintfW", Conv::Cdecl, 2, |e, x| {
        wsprintf(e, x, true)
    }),
    (U32, "wsprintfA", Conv::Cdecl, 2, |e, x| {
        wsprintf(e, x, false)
    }),
    // WP7 (docs/host.md, "Imports": the triage of the missing imports)
    (OLEAUT, "ORDINAL 2", Conv::Stdcall, 1, sys_alloc_string),
    (OLEAUT, "ORDINAL 4", Conv::Stdcall, 2, sys_alloc_string_len),
    (OLEAUT, "ORDINAL 6", Conv::Stdcall, 1, sys_free_string),
    (
        "ole32.dll",
        "CoCreateInstance",
        Conv::Stdcall,
        5,
        co_create_instance,
    ),
    // E_NOTIMPL; only after a CoCreateInstance that succeeded
    (
        "ole32.dll",
        "CoSetProxyBlanket",
        Conv::Stdcall,
        8,
        |_, _| Ret::eax(0x8000_4001),
    ),
    ("ole32.dll", "CoUninitialize", Conv::Stdcall, 0, |_, _| {
        Ret::VOID
    }),
    (K32, "FindResourceW", Conv::Stdcall, 3, find_resource_w),
    (K32, "FindResourceExW", Conv::Stdcall, 4, find_resource_ex_w),
    (K32, "LoadResource", Conv::Stdcall, 2, load_resource),
    // the loaded resource is its bytes already
    (K32, "LockResource", Conv::Stdcall, 1, |_, x| {
        Ret::eax(a(x, 0))
    }),
    (K32, "SizeofResource", Conv::Stdcall, 2, sizeof_resource),
    (
        "SHELL32.dll",
        "ShellExecuteW",
        Conv::Stdcall,
        6,
        shell_execute_w,
    ),
    (
        U32,
        "ChangeDisplaySettingsA",
        Conv::Stdcall,
        2,
        change_display_settings_a,
    ),
    // SetWindowsHookExA is a stand-in (no real hook, so Windows never
    // calls PWindowWin32_MouseHookProc, the only CallNextHookEx caller)
    (U32, "UnhookWindowsHookEx", Conv::Stdcall, 1, ret1),
    (U32, "CallNextHookEx", Conv::Stdcall, 4, ret0),
    // the embedded mode's parent (PWindowWin32_AttachToHost,
    // PApplication_InitWindowAndApp_00629ab0); Windows forwards it
    (U32, "SetParent", Conv::Stdcall, 2, ret0),
    // the window's WM_CLOSE / WM_DESTROY; Windows forwards both
    (U32, "DestroyWindow", Conv::Stdcall, 1, ret1),
    (U32, "PostQuitMessage", Conv::Stdcall, 1, |_, _| Ret::VOID),
    (
        "iggy_w32.dll",
        "IggyGDrawSendWarning",
        Conv::Cdecl,
        2,
        iggy_gdraw_warning,
    ),
];
