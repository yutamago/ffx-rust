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

const K32: &str = "KERNEL32.dll";
const U32: &str = "USER32.dll";

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
];
