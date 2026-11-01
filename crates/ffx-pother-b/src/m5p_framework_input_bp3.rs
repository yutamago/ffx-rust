//! M5 stage 1, list `phyre-framework-input`, round big-phyre3
//! (`ffx_pother_b::m5p_framework_input_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_framework_input.rs` (`bp3_*`).
//!
//! The stream reader, the text array and the `sscanf_s` targets stay in
//! the original's frame below `ebp` (the shim passes the top of a
//! `StackFrame`, the tests the original's frame address).

/// `0x0062b5f0`'s callees: the `PStreamReaderFile` constructor `(path)`,
/// `isOpen`, `readAll(&array, terminate)`, the file's close (on the
/// reader `+0x14`), the destructor (all thiscall), `PhyrePrintf`,
/// `nextToken_00628e40(line, end, ':')` (cdecl), the input log array's
/// `resize90` (thiscall on `0x00cc9d14`), `PhyreFree`.
pub const INPUT_LOG_CALLS: [u32; 9] = [
    0x0057_66d0,
    0x0057_6980,
    0x0057_dd60,
    0x0060_7d80,
    0x0057_67c0,
    0x0043_52f0,
    0x0062_8e40,
    0x0062_c1f0,
    0x0042_fb20,
];
/// The CRT imports (slots): `strchr`, `sscanf_s`, `strncmp`, `strncpy`,
/// `_stricmp`.
pub const INPUT_LOG_IMPORTS: [u32; 5] = [
    0x00b0_c3c0,
    0x00b0_c418,
    0x00b0_c3e4,
    0x00b0_c368,
    0x00b0_c398,
];
/// The texts: the open error, `"%d:"`, `"KB"`, `"%d:%d:%d"`, the
/// missing keyboard warning, `"IK"`, `":%d"`, the embedded script
/// command message, the unknown identifier warning.
pub const INPUT_LOG_TEXTS: [u32; 9] = [
    0x00b4_03c8,
    0x00b4_03dc,
    0x00b4_02f0,
    0x00b4_03e0,
    0x00b4_03ec,
    0x00b4_031c,
    0x00b4_0424,
    0x00b4_0428,
    0x00b4_0468,
];
/// The input log array (`{count, data}`, 0x90-byte entries) and the
/// missing keyboard warning's once byte.
pub const INPUT_LOG_GLOBALS: [u32; 2] = [0x00cc_9d14, 0x00cc_9eed];
/// `0x0062b5f0`'s frame below `ebp` (the reader at `ebp - 0x60`), in
/// words.
pub const INPUT_LOG_FRAME_WORDS: u32 = 0x60 / 4;

pub mod input_log {
    //! `0x0062b5f0` over game memory.

    use super::{INPUT_LOG_CALLS, INPUT_LOG_GLOBALS, INPUT_LOG_IMPORTS, INPUT_LOG_TEXTS};
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A CRT import through its slot (cdecl).
    fn crt(h: &mut impl Host, k: usize, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(INPUT_LOG_IMPORTS[k]));
        h.call(f, Conv::Cdecl, args)
    }

    fn text(h: &impl Host, k: usize) -> u32 {
        h.rebase(INPUT_LOG_TEXTS[k])
    }

    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// Whether a line holds an entry: not a comment (`#`), a `:` in it,
    /// and a token before one (`nextToken`).
    fn is_entry(h: &mut impl Host, line: u32, end: u32) -> bool {
        let next_token = INPUT_LOG_CALLS[6];
        h.read::<u8>(line) != b'#'
            && crt(h, 0, &[line, u32::from(b':')]) != 0
            && h.call_sym(next_token, Conv::Cdecl, &[line, end, u32::from(b':')]) != 0
    }

    /// The text's lines (each `\n` before the end turned into a NUL):
    /// `(lines, entries)`.
    fn count_lines(h: &mut impl Host, data: u32, end: u32) -> (u32, u32) {
        let (mut lines, mut entries) = (0u32, 0u32);
        let mut at = data;
        loop {
            let line = at;
            let mut nl = None;
            if h.read::<u8>(at) == b'\n' {
                nl = Some(at);
            } else {
                while at < end {
                    at += 1;
                    if h.read::<u8>(at) == b'\n' {
                        nl = Some(at);
                        break;
                    }
                }
            }
            match nl {
                Some(p) if p < end => {
                    h.write::<u8>(p, 0);
                    if is_entry(h, line, end) {
                        entries += 1;
                    }
                    lines += 1;
                    at = p + 1;
                }
                _ => return (lines, entries),
            }
        }
    }

    /// The keyboard entry (`"KB"`): key, value and modifier scanned from
    /// `rest` (`"%d:%d:%d"` into `ebp - 0x10`, `- 0x2c`, `- 0x30`), kind 1
    /// when the application's input (`+0x2b8`) has a keyboard for the
    /// key (`+0x80 + 4 key`), else 0 (with a warning once); the three
    /// bytes at `+8`.
    fn keyboard(h: &mut impl Host, ebp: u32, app: u32, rest: u32, off: u32) {
        let printf = INPUT_LOG_CALLS[5];
        let [array, once] = INPUT_LOG_GLOBALS.map(|a| h.rebase(a));
        let table: u32 = h.read(array + 4);
        for at in [ebp - 0x2c, ebp - 0x10, ebp - 0x30] {
            h.write(at, 0xdead_beefu32);
        }
        let fmt = text(h, 3);
        crt(h, 1, &[rest, fmt, ebp - 0x10, ebp - 0x2c, ebp - 0x30]);
        let input: u32 = h.read(app + 0x2b8);
        let key: u32 = h.read(ebp - 0x10);
        let kind = if h.read::<u32>(input.wrapping_add(0x80).wrapping_add(key.wrapping_mul(4))) != 0
        {
            1
        } else {
            if h.read::<u8>(once) == 0 {
                h.write::<u8>(once, 1);
                let warning = text(h, 4);
                h.call_sym(printf, Conv::Cdecl, &[1, warning]);
            }
            0
        };
        let now: u32 = h.read(array + 4);
        h.write(now.wrapping_add(off), kind);
        let bytes = [ebp - 0x10, ebp - 0x2c, ebp - 0x30].map(|a| h.read::<u8>(a));
        h.write(table.wrapping_add(off + 8), bytes);
    }

    /// The input-key entry (`"IK"`): kind 2; its name (up to the first
    /// `:` of `rest`) copied to `+8` (`strncpy` of all of `rest`, then
    /// cut), `+0x88` 1 for `+` after the `:` else 2, `+0x8c` the byte
    /// `":%d"` scans after it (into `ebp - 0x34`); all 0 without a name.
    fn input_key(h: &mut impl Host, ebp: u32, rest: u32, off: u32) {
        let [array, _] = INPUT_LOG_GLOBALS.map(|a| h.rebase(a));
        let table: u32 = h.read(array + 4);
        h.write(table.wrapping_add(off), 2u32);
        let dst = h.read::<u32>(array + 4).wrapping_add(8 + off);
        let len = strlen(h, rest);
        let mut n = 0u32;
        let mut at = rest;
        if h.read::<u8>(rest) != b':' {
            while n < len {
                at += 1;
                n += 1;
                if h.read::<u8>(at) == b':' {
                    break;
                }
            }
        }
        if n == 0 {
            h.write::<u8>(dst, 0);
            h.write::<u8>(dst + 0x84, 0);
            h.write(dst + 0x80, 0u32);
            return;
        }
        crt(h, 3, &[dst, rest, len]);
        h.write::<u8>(dst.wrapping_add(n), 0);
        let mode = if h.read::<u8>(at + 1) == b'+' {
            1u32
        } else {
            2
        };
        h.write(ebp - 0x34, 0xdead_beefu32);
        h.write(dst + 0x80, mode);
        let fmt = text(h, 6);
        crt(h, 1, &[at + 2, fmt, ebp - 0x34]);
        let v: u8 = h.read(ebp - 0x34);
        h.write(dst + 0x84, v);
    }

    /// Any other token: the application's script command (`[app +
    /// 0x2c]`, 0x48-byte records at `+0x14`, `+0x18` of them) named so
    /// (`_stricmp`): kind 3, the command's name pointer and the
    /// parameters (`rest`, its last byte cut by the original's off-by-one
    /// NUL) at `+8`, a message; else kind 0 and a warning.
    fn command(h: &mut impl Host, app: u32, token: u32, rest: u32, off: u32, frame: u32) {
        let printf = INPUT_LOG_CALLS[5];
        let [array, _] = INPUT_LOG_GLOBALS.map(|a| h.rebase(a));
        let dst = h.read::<u32>(array + 4).wrapping_add(8 + off);
        let mut i = 0u32;
        loop {
            let scripts: u32 = h.read(app + 0x2c);
            if i >= h.read::<u32>(scripts + 0x18) {
                break;
            }
            let name = h.read::<u32>(scripts + 0x14).wrapping_add(4 + 0x48 * i);
            if crt(h, 4, &[name, token]) == 0 {
                let table: u32 = h.read(array + 4);
                h.write(table.wrapping_add(off), 3u32);
                let scripts: u32 = h.read(app + 0x2c);
                let found = h.read::<u32>(scripts + 0x14).wrapping_add(0x48 * i + 4);
                h.write(dst, found);
                let len = strlen(h, rest);
                if len == 0 {
                    h.write::<u8>(dst + 4, 0);
                } else {
                    crt(h, 3, &[dst + 4, rest, len]);
                    h.write::<u8>(dst.wrapping_add(len + 3), 0);
                }
                let message = text(h, 7);
                h.call_sym(printf, Conv::Cdecl, &[2, message, found, dst + 4, frame]);
                return;
            }
            i += 1;
        }
        let warning = text(h, 8);
        h.call_sym(printf, Conv::Cdecl, &[1, warning, token]);
        let table: u32 = h.read(array + 4);
        h.write(table.wrapping_add(off), 0u32);
    }

    /// The entries: per line (up to `lines`) while entries are left, a
    /// skipped comment or line without a `:`; else the frame (`"%d:"`
    /// into `ebp - 0x24`), the token (`nextToken`; without one the same
    /// line again), the frame stored at the entry's `+4` and the entry
    /// by its token ([`keyboard`], [`input_key`], [`command`]).
    #[allow(clippy::too_many_arguments)] // the original's locals
    fn read_entries(
        h: &mut impl Host,
        ebp: u32,
        app: u32,
        data: u32,
        end: u32,
        lines: u32,
        entries: u32,
    ) {
        let next_token = INPUT_LOG_CALLS[6];
        let [array, _] = INPUT_LOG_GLOBALS.map(|a| h.rebase(a));
        let mut at = data;
        let mut k = 0u32;
        let mut off = 0u32;
        h.write(ebp - 0x10, off);
        for _ in 0..lines {
            if k >= entries {
                return;
            }
            h.write(ebp - 0x24, 0xdead_beefu32);
            if h.read::<u8>(at) == b'#' || crt(h, 0, &[at, u32::from(b':')]) == 0 {
                at = at.wrapping_add(strlen(h, at) + 1);
                continue;
            }
            let fmt = text(h, 1);
            crt(h, 1, &[at, fmt, ebp - 0x24]);
            let token = h.call_sym(next_token, Conv::Cdecl, &[at, end, u32::from(b':')]);
            if token == 0 {
                continue;
            }
            let rest = token.wrapping_add(strlen(h, token) + 1);
            let frame: u32 = h.read(ebp - 0x24);
            let table: u32 = h.read(array + 4);
            h.write(table.wrapping_add(off + 4), frame);
            let kb = text(h, 2);
            let ik = text(h, 5);
            if crt(h, 2, &[token, kb, 2]) == 0 {
                keyboard(h, ebp, app, rest, off);
            } else if crt(h, 2, &[token, ik, 2]) == 0 {
                input_key(h, ebp, rest, off);
            } else {
                command(h, app, token, rest, off, frame);
            }
            k += 1;
            off += 0x90;
            h.write(ebp - 0x10, off);
            at = rest.wrapping_add(strlen(h, rest) + 1);
        }
    }

    /// `Phyre_PFramework_PApplicationScriptProxy_readInputLog(app, path)`
    /// (cdecl; Fahrenheit's name): the file read whole (a reader at
    /// `ebp - 0x60`, the text array at `ebp - 0x44`; 0x13 with a message
    /// when it doesn't open, `readAll`'s error otherwise); its lines
    /// counted ([`count_lines`]), the input log array (`0x00cc9d14`)
    /// sized to the entries, the entries read ([`read_entries`]); the
    /// text freed when owned; the reader destroyed. `readAll`'s result.
    pub fn read_input_log(h: &mut impl Host, ebp: u32, app: u32, path: u32) -> u32 {
        let [
            ctor,
            is_open,
            read_all,
            close,
            dtor,
            printf,
            _,
            resize,
            free,
        ] = INPUT_LOG_CALLS;
        let reader = ebp - 0x60;
        this_call(h, ctor, reader, &[path]);
        let result = if this_call(h, is_open, reader, &[]).to_le_bytes()[0] == 0 {
            let message = text(h, 0);
            h.call_sym(printf, Conv::Cdecl, &[2, message, path]);
            0x13
        } else {
            let arr = ebp - 0x44;
            h.write(arr, [0u32; 2]);
            let err = this_call(h, read_all, reader, &[arr, 1]);
            this_call(h, close, ebp - 0x4c, &[]);
            if err == 0 {
                let head: u32 = h.read(arr);
                let data: u32 = h.read(arr + 4);
                let end = data.wrapping_add(head & 0x7fff_ffff);
                let (lines, entries) = count_lines(h, data, end);
                if lines != 0 {
                    let array = h.rebase(INPUT_LOG_GLOBALS[0]);
                    this_call(h, resize, array, &[entries]);
                    read_entries(h, ebp, app, data, end, lines, entries);
                }
            }
            let head: u32 = h.read(arr);
            let data: u32 = h.read(arr + 4);
            if head >> 31 == 0 && data != 0 {
                h.call_sym(free, Conv::Cdecl, &[data]);
            }
            h.write(arr, [0u32; 2]);
            err
        };
        this_call(h, dtor, reader, &[]);
        result
    }
}
