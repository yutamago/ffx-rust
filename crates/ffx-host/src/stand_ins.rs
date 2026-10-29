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

/// `IggyPlayerCreateFromMemory(data, size, config)`: a player handle (a
/// zeroed block on the heap, opaque to the game), where the null Iggy
/// stand-in's 0 makes `IggyMenu_ctor_00a288f0` keep a null player and
/// fault in `SetFields_00a3dd00`. Until `ffx-swf` plays the movie.
fn iggy_player(e: &mut Exec, _: &Args) -> Ret {
    let p = e.alloc(0x100);
    if p != 0 {
        e.mem().write_bytes(p, &[0u8; 0x100]);
    }
    Ret::eax(p)
}

/// `GetSystemTime(&st)`: the fixed clock of `GetSystemTimeAsFileTime`
/// (FILETIME `0x01d0_0000_0000_0000`), 2014-11-14 11:41:59.374, a Friday.
fn system_time(e: &mut Exec, args: &Args) -> Ret {
    e.mem()
        .write(a(args, 0), [2014u16, 11, 5, 14, 11, 41, 59, 374]);
    Ret::VOID
}

const K32: &str = "KERNEL32.dll";
const IGGY: &str = "iggy_w32.dll";
const U32: &str = "USER32.dll";

/// The stand-ins: DLL, name, convention, stack words.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    // the C runtime's file names: ANSI code page
    (K32, "AreFileApisANSI", Conv::Stdcall, 0, one),
    (K32, "GetSystemTime", Conv::Stdcall, 1, system_time),
    (U32, "wsprintfW", Conv::Cdecl, 2, |e, x| {
        wsprintf(e, x, true)
    }),
    (U32, "wsprintfA", Conv::Cdecl, 2, |e, x| {
        wsprintf(e, x, false)
    }),
    (
        IGGY,
        "_IggyPlayerCreateFromMemory@12",
        Conv::Stdcall,
        3,
        iggy_player,
    ),
];
