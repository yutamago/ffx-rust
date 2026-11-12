//! `MSVCR110`'s environment, locale, time and process functions for the
//! no-DLL build (docs/host.md, "The C runtime DLLs", m5-wp5c): what the
//! game's callers (Lua's `os`/`io` libraries, libvpx's CPU probe and
//! threads, `PhyreInit`'s environment variable) see from the DLL in the
//! run's world: an environment that starts empty (the stand-in
//! `GetEnvironmentStringsW` gives an empty block, from which the DLL
//! builds its own), the "C" locale, the run's fixed `time` (UTC as the
//! local time), no command processor. With the DLLs loaded their own
//! run (these are plain `imports` entries).

#![allow(clippy::many_single_char_names)] // the C names

use std::collections::HashMap;

use ffx_bridge::Mem;

use crate::crt_str::{EINVAL, set_errno};
use crate::exec::Exec;
use crate::imports::{ImportFn, read_str};
use crate::table::{Args, Conv, Ret};

const CRT: &str = "MSVCR110.dll";

/// The run's C runtime state the DLL would keep in its `.data`.
#[derive(Debug, Default)]
pub struct CrtState {
    /// `_putenv_s`'s variables, in the order set (names compared without
    /// case, as the DLL's are).
    pub env: Vec<(String, Vec<u8>)>,
    /// Cells in the arena the functions hand out (`__iob_func`'s array,
    /// `localeconv`'s `lconv`, ...), by name.
    pub cells: HashMap<&'static str, u32>,
}

/// The cell `name`, if made.
#[must_use]
pub fn cell(e: &Exec, name: &'static str) -> Option<u32> {
    e.shared()
        .crt_state
        .lock()
        .ok()
        .and_then(|s| s.cells.get(name).copied())
}

/// Records the cell `name`.
pub fn set_cell(e: &Exec, name: &'static str, at: u32) {
    if let Ok(mut s) = e.shared().crt_state.lock() {
        s.cells.insert(name, at);
    }
}

/// The cell `name`, `make` filling a fresh one of `size` bytes the first
/// time.
fn cell_or(e: &mut Exec, name: &'static str, size: u32, make: impl FnOnce(&mut Exec, u32)) -> u32 {
    if let Some(at) = cell(e, name) {
        return at;
    }
    let at = e.alloc(size);
    e.mem().write_bytes(at, &vec![0; size as usize]);
    make(e, at);
    set_cell(e, name, at);
    at
}

pub const NO_DLL: &[(&str, &str, Conv, usize, ImportFn)] = &[
    (CRT, "getenv", Conv::Cdecl, 1, getenv),
    (CRT, "_dupenv_s", Conv::Cdecl, 3, dupenv_s),
    (CRT, "_putenv_s", Conv::Cdecl, 2, putenv_s),
    (CRT, "setlocale", Conv::Cdecl, 2, setlocale),
    (CRT, "localeconv", Conv::Cdecl, 0, localeconv),
    (CRT, "strerror", Conv::Cdecl, 1, strerror),
    (CRT, "clock", Conv::Cdecl, 0, clock),
    (CRT, "_gmtime64", Conv::Cdecl, 1, gmtime64),
    (CRT, "_localtime64", Conv::Cdecl, 1, gmtime64),
    (CRT, "_mktime64", Conv::Cdecl, 1, mktime64),
    (CRT, "_difftime64", Conv::Cdecl, 4, difftime64),
    (CRT, "strftime", Conv::Cdecl, 4, strftime),
    (CRT, "system", Conv::Cdecl, 1, system),
    (CRT, "_beginthreadex", Conv::Cdecl, 6, beginthreadex),
];

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

fn env_value(e: &Exec, name: &str) -> Option<Vec<u8>> {
    let s = e.shared().crt_state.lock().ok()?;
    s.env
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// `getenv(name)`: a variable `_putenv_s` set (the DLL's copy, kept per
/// name), or NULL.
fn getenv(e: &mut Exec, args: &Args) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), false);
    let Some(mut v) = env_value(e, &name) else {
        return Ret::eax(0);
    };
    let key: &'static str =
        Box::leak(format!("getenv {}", name.to_ascii_uppercase()).into_boxed_str());
    v.push(0);
    let len = u32::try_from(v.len()).unwrap_or(0);
    let at = cell_or(e, key, len.max(16) + 256, |_, _| {});
    if len <= 256 {
        e.mem().write_bytes(at, &v);
    }
    Ret::eax(at)
}

/// `_dupenv_s(buffer, count, name)`: a heap copy of the variable and its
/// length with the NUL, or NULL and 0; 0 (`EINVAL` for NULL out-pointers).
fn dupenv_s(e: &mut Exec, args: &Args) -> Ret {
    let (out, count, name) = (a(args, 0), a(args, 1), a(args, 2));
    if out == 0 || name == 0 {
        set_errno(e, EINVAL);
        return Ret::eax(EINVAL);
    }
    let text = read_str(&e.mem(), name, false);
    let (at, n) = match env_value(e, &text) {
        Some(mut v) => {
            v.push(0);
            let n = u32::try_from(v.len()).unwrap_or(0);
            let at = e.alloc(n);
            e.mem().write_bytes(at, &v);
            (at, n)
        }
        None => (0, 0),
    };
    let mut mem = e.mem();
    mem.write(out, at);
    if count != 0 {
        mem.write(count, n);
    }
    Ret::eax(0)
}

/// `_putenv_s(name, value)`: sets (an empty value removes) the variable;
/// 0, `EINVAL` for NULLs or a name with `=`.
fn putenv_s(e: &mut Exec, args: &Args) -> Ret {
    let (name, value) = (a(args, 0), a(args, 1));
    if name == 0 || value == 0 {
        set_errno(e, EINVAL);
        return Ret::eax(EINVAL);
    }
    let (n, v) = {
        let mem = e.mem();
        (read_str(&mem, name, false), read_str(&mem, value, false))
    };
    if n.is_empty() || n.contains('=') {
        set_errno(e, EINVAL);
        return Ret::eax(EINVAL);
    }
    if let Ok(mut s) = e.shared().crt_state.lock() {
        s.env.retain(|(k, _)| !k.eq_ignore_ascii_case(&n));
        if !v.is_empty() {
            s.env.push((n, v.into_bytes()));
        }
    }
    Ret::eax(0)
}

/// `setlocale(category, locale)`: the run stays in the "C" locale. A
/// query (NULL), `"C"`, `""` (the user's default, which the run takes to
/// be "C") and `"POSIX"`-less names: "C"; another name: NULL (the DLL
/// fails for names it doesn't know). A category outside 0..5: NULL.
fn setlocale(e: &mut Exec, args: &Args) -> Ret {
    let (category, locale) = (a(args, 0), a(args, 1));
    if category > 5 {
        set_errno(e, EINVAL);
        return Ret::eax(0);
    }
    if locale != 0 {
        let name = read_str(&e.mem(), locale, false);
        if !(name.is_empty() || name == "C") {
            return Ret::eax(0);
        }
    }
    let at = cell_or(e, "setlocale C", 4, |e, at| e.mem().write_bytes(at, b"C\0"));
    Ret::eax(at)
}

/// `localeconv()`: the "C" locale's `lconv` (VS2012's layout: 10 string
/// pointers, 8 `char`s, then the 8 wide strings): `"."` and empty
/// strings, `CHAR_MAX` for the numbers.
fn localeconv(e: &mut Exec, _: &Args) -> Ret {
    let at = cell_or(e, "lconv", 0x60, |e, at| {
        let strings = e.alloc(8);
        let mut mem = e.mem();
        mem.write_bytes(strings, b".\0\0\0.\0\0\0");
        // decimal_point; thousands_sep, grouping, int_curr_symbol,
        // currency_symbol, mon_decimal_point, mon_thousands_sep,
        // mon_grouping, positive_sign, negative_sign: ""
        mem.write(at, strings);
        for i in 1..10 {
            mem.write(at + 4 * i, strings + 1);
        }
        // int_frac_digits .. n_sign_posn
        mem.write_bytes(at + 40, &[0x7f; 8]);
        // _W_decimal_point and the other wide ones: L"." and L""
        mem.write(at + 48, strings + 4);
        for i in 1..8 {
            mem.write(at + 48 + 4 * i, strings + 2);
        }
    });
    Ret::eax(at)
}

/// `_sys_errlist` in the DLL (`0x1000d700`, in its `.text`: pointers to
/// the messages), its 43 entries and "Unknown error" after them.
const SYS_ERRLIST: u32 = 0x1000_d700;
const SYS_NERR: u32 = 43;

/// `strerror(n)`: the DLL's message for `n` ("Unknown error" past the
/// table), copied to this thread's buffer as the DLL does (134 bytes).
fn strerror(e: &mut Exec, args: &Args) -> Ret {
    let n = a(args, 0).min(SYS_NERR);
    let text = e
        .shared()
        .crt_data
        .as_ref()
        .and_then(|d| d.c_string(d.word(SYS_ERRLIST + 4 * n)))
        .unwrap_or_else(|| b"Unknown error".to_vec());
    if e.crt.strerror_at == 0 {
        e.crt.strerror_at = e.alloc(134);
    }
    let at = e.crt.strerror_at;
    let mut v = text;
    v.truncate(133);
    v.push(0);
    e.mem().write_bytes(at, &v);
    Ret::eax(at)
}

/// `clock()`: milliseconds since the run started.
fn clock(e: &mut Exec, _: &Args) -> Ret {
    let ms = e.shared().started.elapsed().as_millis();
    Ret::eax(u32::try_from(ms).unwrap_or(u32::MAX))
}

/// Days since 1970-01-01 to (year, month 0-based, day 1-based).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 2 } else { mp - 10 }).unwrap_or(0);
    let y = yoe + era * 400 + i64::from(m < 2);
    (y, m, d)
}

/// (year, month 0-based, day) to days since 1970-01-01.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m < 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 10) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The `tm` fields (sec, min, hour, mday, mon, year - 1900, wday, yday,
/// isdst) of the time `t` (UTC).
fn tm_fields(t: i64) -> [i64; 9] {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    let yday = days - days_from_civil(y, 0, 1);
    let wday = (days + 4).rem_euclid(7);
    [
        secs % 60,
        (secs / 60) % 60,
        secs / 3600,
        i64::from(d),
        i64::from(m),
        y - 1900,
        wday,
        yday,
        0,
    ]
}

/// The latest time the DLL's 64-bit functions take (3000-12-31 23:59:59).
const MAX_TIME: i64 = 32_535_215_999;

fn write_tm(e: &Exec, at: u32, fields: &[i64; 9]) {
    let mut mem = e.mem();
    for (i, v) in (0u32..).zip(fields) {
        mem.write(at + 4 * i, i32::try_from(*v).unwrap_or(0).cast_unsigned());
    }
}

/// `_gmtime64(t)` / `_localtime64(t)` (the run's local time is UTC): the
/// thread's `tm`, or NULL (`EINVAL`) for a negative time or one past
/// 3000.
fn gmtime64(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if p == 0 {
        set_errno(e, EINVAL);
        return Ret::eax(0);
    }
    let t: i64 = e.mem().read::<u64>(p).cast_signed();
    if !(0..=MAX_TIME).contains(&t) {
        set_errno(e, EINVAL);
        return Ret::eax(0);
    }
    if e.crt.tm_at == 0 {
        e.crt.tm_at = e.alloc(36);
    }
    let at = e.crt.tm_at;
    write_tm(e, at, &tm_fields(t));
    Ret::eax(at)
}

/// `_mktime64(tm)`: the time of the local (= UTC) fields, normalized;
/// the fields rewritten normalized with `wday`/`yday`; -1 outside the
/// range.
fn mktime64(e: &mut Exec, args: &Args) -> Ret {
    let p = a(args, 0);
    if p == 0 {
        set_errno(e, EINVAL);
        return Ret::u64(u64::MAX);
    }
    let f: Vec<i64> = {
        let mem = e.mem();
        (0..6)
            .map(|i| i64::from(mem.read::<u32>(p + 4 * i).cast_signed()))
            .collect()
    };
    let (sec, min, hour, mday, mon, year) = (f[0], f[1], f[2], f[3], f[4], f[5]);
    let y = 1900 + year + mon.div_euclid(12);
    let m = mon.rem_euclid(12);
    let days = days_from_civil(y, m, 1) + mday - 1;
    let t = days * 86_400 + hour * 3600 + min * 60 + sec;
    if !(0..=MAX_TIME).contains(&t) {
        return Ret::u64(u64::MAX);
    }
    write_tm(e, p, &tm_fields(t));
    Ret::u64(t.cast_unsigned())
}

/// `_difftime64(t1, t0)`: `t1 - t0` as a double in `ST0`.
fn difftime64(_: &mut Exec, args: &Args) -> Ret {
    let t1 = (u64::from(a(args, 0)) | (u64::from(a(args, 1)) << 32)).cast_signed();
    let t0 = (u64::from(a(args, 2)) | (u64::from(a(args, 3)) << 32)).cast_signed();
    #[allow(clippy::cast_precision_loss)] // (double) of the difference
    let d = (t1 - t0) as f64;
    Ret {
        st0: Some(d),
        ..Ret::VOID
    }
}

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `strftime(buf, max, fmt, tm)` in the "C" locale: the conversions of
/// VS2012's `_Strftime` (`%a %A %b %B %c %d %H %I %j %m %M %p %S %U %w %W
/// %x %X %y %Y %z %Z %%`, the `#` flag ignored); the length without the
/// NUL, or 0 (the buffer emptied) when it doesn't fit.
fn strftime(e: &mut Exec, args: &Args) -> Ret {
    let (buf, max, fmt, tm) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let (f, t) = {
        let mem = e.mem();
        let f = read_str(&mem, fmt, false);
        let t: Vec<i64> = (0..9)
            .map(|i| i64::from(mem.read::<u32>(tm + 4 * i).cast_signed()))
            .collect();
        (f, t)
    };
    let (sec, min, hour, mday, mon, year, wday, yday) =
        (t[0], t[1], t[2], t[3], t[4], t[5], t[6], t[7]);
    let name = |list: &[&str], i: i64| {
        usize::try_from(i)
            .ok()
            .and_then(|i| list.get(i))
            .copied()
            .unwrap_or("?")
            .to_string()
    };
    let week = |first: i64| (yday + 7 - (wday - first).rem_euclid(7)) / 7;
    let date = format!("{:02}/{:02}/{:02}", mon + 1, mday, year.rem_euclid(100));
    let time = format!("{hour:02}:{min:02}:{sec:02}");
    let mut out = String::new();
    let mut chars = f.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut k = chars.next();
        if k == Some('#') {
            k = chars.next();
        }
        let piece = match k {
            Some('a') => name(&DAYS, wday)[..3].to_string(),
            Some('A') => name(&DAYS, wday),
            Some('b') => name(&MONTHS, mon)[..3].to_string(),
            Some('B') => name(&MONTHS, mon),
            Some('c') => format!("{date} {time}"),
            Some('d') => format!("{mday:02}"),
            Some('H') => format!("{hour:02}"),
            Some('I') => format!("{:02}", (hour + 11) % 12 + 1),
            Some('j') => format!("{:03}", yday + 1),
            Some('m') => format!("{:02}", mon + 1),
            Some('M') => format!("{min:02}"),
            Some('p') => (if hour < 12 { "AM" } else { "PM" }).to_string(),
            Some('S') => format!("{sec:02}"),
            Some('U') => format!("{:02}", week(0)),
            Some('w') => format!("{wday}"),
            Some('W') => format!("{:02}", week(1)),
            Some('x') => date.clone(),
            Some('X') => time.clone(),
            Some('y') => format!("{:02}", year.rem_euclid(100)),
            Some('Y') => format!("{}", year + 1900),
            Some('z' | 'Z') => "Coordinated Universal Time".to_string(),
            Some('%') => "%".to_string(),
            // an unknown conversion: the invalid-parameter path
            _ => {
                if buf != 0 && max != 0 {
                    e.mem().write(buf, 0u8);
                }
                set_errno(e, EINVAL);
                return Ret::eax(0);
            }
        };
        out.push_str(&piece);
    }
    let bytes = out.into_bytes();
    if u32::try_from(bytes.len()).unwrap_or(u32::MAX) >= max {
        if buf != 0 && max != 0 {
            e.mem().write(buf, 0u8);
        }
        return Ret::eax(0);
    }
    let mut v = bytes;
    let n = u32::try_from(v.len()).unwrap_or(0);
    v.push(0);
    e.mem().write_bytes(buf, &v);
    Ret::eax(n)
}

/// `system(command)`: no command processor in the run (`COMSPEC` is not
/// set: the DLL's `system` then fails): 0 for NULL (none present), -1
/// with `ENOENT` for a command.
fn system(e: &mut Exec, args: &Args) -> Ret {
    if a(args, 0) == 0 {
        return Ret::eax(0);
    }
    set_errno(e, 2);
    Ret::eax(u32::MAX)
}

/// `_beginthreadex(security, stack, start, arg, flags, id)`: a game
/// thread running `start(arg)` (stdcall), suspended for
/// `CREATE_SUSPENDED`; its handle, `*id` its id.
fn beginthreadex(e: &mut Exec, args: &Args) -> Ret {
    let suspended = a(args, 4) & 4 != 0;
    let (handle, id) = e.spawn_thread(a(args, 2), a(args, 3), suspended, "_beginthreadex");
    if a(args, 5) != 0 {
        e.mem().write(a(args, 5), id);
    }
    Ret::eax(handle)
}
