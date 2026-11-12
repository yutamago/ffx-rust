//! `MSVCR110`'s stdio for the no-DLL build (m5-wp5b, 2026-11-12): `fopen`,
//! `_wfopen`, `fclose`, `fread`, `fwrite`, `fseek`, `_fseeki64`, `ftell`,
//! `_ftelli64`, `feof`, `ferror`, `fflush`, `fgets`, `getc`, `ungetc`
//! over the host's files (`crate::files`, the game and documents folders).
//! With the player's DLLs loaded their own stdio runs (these are plain
//! `imports` entries: the DLL's export wins).
//!
//! A `FILE` is a 0x20-byte block on the arena heap laid out as MSVCR110's
//! (`_flag` at `+0xc`, `_file` at `+0x10`, `_charbuf` at `+0x14`); the
//! game only passes the pointer around. Streams are unbuffered: every call
//! goes to the file at once, so `ftell` is the file position, as the CRT's
//! is once its buffer is accounted for. Text mode (no `b` in the mode,
//! `_fmode` 0) writes `\n` as `\r\n` and reads `\r\n` as `\n`, a `^Z`
//! ending the input, as the CRT does.

#![allow(clippy::many_single_char_names)] // the C names

use ffx_bridge::Mem;

use crate::exec::Exec;
use crate::imports::{ImportFn, read_str};
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// The family.
pub const TABLE: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "fopen", Conv::Cdecl, 2, fopen),
    (CRT, "_wfopen", Conv::Cdecl, 2, wfopen),
    (CRT, "fclose", Conv::Cdecl, 1, fclose),
    (CRT, "fread", Conv::Cdecl, 4, fread),
    (CRT, "fwrite", Conv::Cdecl, 4, fwrite),
    (CRT, "fseek", Conv::Cdecl, 3, fseek),
    (CRT, "_fseeki64", Conv::Cdecl, 4, fseeki64),
    (CRT, "ftell", Conv::Cdecl, 1, ftell),
    (CRT, "_ftelli64", Conv::Cdecl, 1, ftelli64),
    (CRT, "feof", Conv::Cdecl, 1, feof),
    (CRT, "ferror", Conv::Cdecl, 1, ferror),
    (CRT, "fflush", Conv::Cdecl, 1, fflush),
    (CRT, "fgets", Conv::Cdecl, 3, fgets),
    (CRT, "getc", Conv::Cdecl, 1, getc),
    (CRT, "ungetc", Conv::Cdecl, 2, ungetc),
];

/// `_flag` bits: end of file, error, text mode, append, a pushed-back
/// byte in `_charbuf`.
const EOF_BIT: u32 = 0x10;
const ERR_BIT: u32 = 0x20;
const TEXT_BIT: u32 = 0x4000;
const APPEND_BIT: u32 = 0x8000;
const PUSHED_BIT: u32 = 0x1_0000;

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn flags(e: &Exec, f: u32) -> u32 {
    e.mem().read(f + 0xc)
}

fn set_flags(e: &Exec, f: u32, v: u32) {
    e.mem().write(f + 0xc, v);
}

fn handle(e: &Exec, f: u32) -> u32 {
    e.mem().read(f + 0x10)
}

fn open(e: &mut Exec, args: &Args, wide: bool) -> Ret {
    let (name, mode) = {
        let mem = e.mem();
        (
            read_str(&mem, a(args, 0), wide),
            read_str(&mem, a(args, 1), wide),
        )
    };
    let plus = mode.contains('+');
    let text = !mode.contains('b');
    let (write, disposition, append) = match mode.chars().next() {
        Some('r') => (plus, 3, false),
        Some('w') => (true, 2, false),
        Some('a') => (true, 4, true),
        _ => return Ret::eax(0),
    };
    let Some(h) = e.files().open(&name, write, disposition) else {
        return Ret::eax(0);
    };
    let f = e.alloc(0x20);
    let mut v = 0;
    if text {
        v |= TEXT_BIT;
    }
    if append {
        v |= APPEND_BIT;
    }
    let mut mem = e.mem();
    mem.write_bytes(f, &[0; 0x20]);
    mem.write(f + 0xc, v);
    mem.write(f + 0x10, h);
    Ret::eax(f)
}

fn fopen(e: &mut Exec, args: &Args) -> Ret {
    open(e, args, false)
}

fn wfopen(e: &mut Exec, args: &Args) -> Ret {
    open(e, args, true)
}

/// `fclose(f)`: the file closed; 0 (EOF for NULL).
fn fclose(e: &mut Exec, args: &Args) -> Ret {
    let f = a(args, 0);
    if f == 0 {
        return Ret::eax(u32::MAX);
    }
    let h = handle(e, f);
    e.files().close(h);
    set_flags(e, f, 0);
    Ret::eax(0)
}

/// `fwrite(buf, size, count, f)`: the items written (text mode: `\n` as
/// `\r\n`).
fn fwrite(e: &mut Exec, args: &Args) -> Ret {
    let (buf, size, count, f) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let n = size.wrapping_mul(count);
    if f == 0 || n == 0 {
        return Ret::eax(0);
    }
    let mut data = vec![0u8; n as usize];
    e.mem().read_bytes(buf, &mut data);
    let v = flags(e, f);
    if v & TEXT_BIT != 0 {
        let mut t = Vec::with_capacity(data.len() + data.len() / 8);
        for &b in &data {
            if b == b'\n' {
                t.push(b'\r');
            }
            t.push(b);
        }
        data = t;
    }
    let h = handle(e, f);
    if v & APPEND_BIT != 0 {
        e.files().seek(h, 0, 2);
    }
    if e.files().write(h, &data).is_some() {
        Ret::eax(count)
    } else {
        set_flags(e, f, v | ERR_BIT);
        Ret::eax(0)
    }
}

/// One byte of the stream (a pushed-back one first; text mode: `\r\n` as
/// `\n`, `^Z` as the end); `None` at the end.
fn next_byte(e: &mut Exec, f: u32) -> Option<u8> {
    let v = flags(e, f);
    if v & PUSHED_BIT != 0 {
        set_flags(e, f, v & !PUSHED_BIT);
        return Some(e.mem().read(f + 0x14));
    }
    let h = handle(e, f);
    let b = *e.files().read(h, 1)?.first()?;
    if v & TEXT_BIT == 0 {
        return Some(b);
    }
    if b == 0x1a {
        e.files().seek(h, -1, 1);
        return None;
    }
    if b == b'\r' {
        match e.files().read(h, 1).and_then(|r| r.first().copied()) {
            Some(b'\n') => return Some(b'\n'),
            Some(_) => {
                e.files().seek(h, -1, 1);
            }
            None => {}
        }
    }
    Some(b)
}

/// `fread(buf, size, count, f)`: whole items read (the end flag set on a
/// short read).
fn fread(e: &mut Exec, args: &Args) -> Ret {
    let (buf, size, count, f) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let n = size.wrapping_mul(count) as usize;
    if f == 0 || n == 0 {
        return Ret::eax(0);
    }
    let v = flags(e, f);
    let data = if v & (TEXT_BIT | PUSHED_BIT) == 0 {
        let h = handle(e, f);
        e.files().read(h, n).unwrap_or_default()
    } else {
        let mut d = Vec::with_capacity(n);
        while d.len() < n {
            match next_byte(e, f) {
                Some(b) => d.push(b),
                None => break,
            }
        }
        d
    };
    e.mem().write_bytes(buf, &data);
    if data.len() < n {
        let v = flags(e, f);
        set_flags(e, f, v | EOF_BIT);
    }
    Ret::eax(u32::try_from(data.len()).unwrap_or(0) / size.max(1))
}

fn seek(e: &mut Exec, f: u32, off: i64, origin: u32) -> Ret {
    if f == 0 || origin > 2 {
        return Ret::eax(u32::MAX);
    }
    let h = handle(e, f);
    let v = flags(e, f);
    set_flags(e, f, v & !(EOF_BIT | PUSHED_BIT));
    match e.files().seek(h, off, origin) {
        Some(_) => Ret::eax(0),
        None => Ret::eax(u32::MAX),
    }
}

/// `fseek(f, offset, origin)`.
fn fseek(e: &mut Exec, args: &Args) -> Ret {
    seek(
        e,
        a(args, 0),
        i64::from(a(args, 1).cast_signed()),
        a(args, 2),
    )
}

/// `_fseeki64(f, offset (two words), origin)`.
fn fseeki64(e: &mut Exec, args: &Args) -> Ret {
    let off = (u64::from(a(args, 2)) << 32 | u64::from(a(args, 1))).cast_signed();
    seek(e, a(args, 0), off, a(args, 3))
}

fn tell(e: &mut Exec, f: u32) -> Option<u64> {
    if f == 0 {
        return None;
    }
    let h = handle(e, f);
    let p = e.files().seek(h, 0, 1)?;
    Some(if flags(e, f) & PUSHED_BIT != 0 {
        p.saturating_sub(1)
    } else {
        p
    })
}

/// `ftell(f)`: the position, -1 on failure.
fn ftell(e: &mut Exec, args: &Args) -> Ret {
    let p = tell(e, a(args, 0));
    Ret::eax(p.and_then(|p| u32::try_from(p).ok()).unwrap_or(u32::MAX))
}

/// `_ftelli64(f)`: the position in `edx:eax`.
fn ftelli64(e: &mut Exec, args: &Args) -> Ret {
    Ret::u64(tell(e, a(args, 0)).unwrap_or(u64::MAX))
}

/// `feof(f)`: the end flag (0x10) or 0.
fn feof(e: &mut Exec, args: &Args) -> Ret {
    let f = a(args, 0);
    Ret::eax(if f == 0 { 0 } else { flags(e, f) & EOF_BIT })
}

/// `ferror(f)`: the error flag (0x20) or 0.
fn ferror(e: &mut Exec, args: &Args) -> Ret {
    let f = a(args, 0);
    Ret::eax(if f == 0 { 0 } else { flags(e, f) & ERR_BIT })
}

/// `fflush(f)`: nothing buffered; 0.
fn fflush(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

/// `fgets(buf, n, f)`: up to `n - 1` bytes, through a newline; `buf`, or
/// NULL when nothing was read.
fn fgets(e: &mut Exec, args: &Args) -> Ret {
    let (buf, n, f) = (a(args, 0), a(args, 1).cast_signed(), a(args, 2));
    if f == 0 || n <= 0 {
        return Ret::eax(0);
    }
    let mut out = Vec::new();
    while i32::try_from(out.len()).unwrap_or(i32::MAX) < n - 1 {
        match next_byte(e, f) {
            Some(b) => {
                out.push(b);
                if b == b'\n' {
                    break;
                }
            }
            None => {
                let v = flags(e, f);
                set_flags(e, f, v | EOF_BIT);
                break;
            }
        }
    }
    if out.is_empty() {
        return Ret::eax(0);
    }
    out.push(0);
    e.mem().write_bytes(buf, &out);
    Ret::eax(buf)
}

/// `getc(f)`: the next byte, EOF (-1) at the end.
fn getc(e: &mut Exec, args: &Args) -> Ret {
    let f = a(args, 0);
    if f == 0 {
        return Ret::eax(u32::MAX);
    }
    if let Some(b) = next_byte(e, f) {
        Ret::eax(u32::from(b))
    } else {
        let v = flags(e, f);
        set_flags(e, f, v | EOF_BIT);
        Ret::eax(u32::MAX)
    }
}

/// `ungetc(c, f)`: `c` pushed back (one byte); `c`, or EOF.
fn ungetc(e: &mut Exec, args: &Args) -> Ret {
    let (c, f) = (a(args, 0), a(args, 1));
    if f == 0 || c == u32::MAX {
        return Ret::eax(u32::MAX);
    }
    #[allow(clippy::cast_possible_truncation)] // (unsigned char) c
    e.mem().write(f + 0x14, c as u8);
    let v = flags(e, f);
    set_flags(e, f, (v | PUSHED_BIT) & !EOF_BIT);
    Ret::eax(c & 0xff)
}
