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

/// `0x00628f00`'s callees: `PInputMapper::getStickAxis` (thiscall `(this;
/// axis, pad)` -> `ST0`), `PInput_GetDevice(kind, index)` (cdecl),
/// `Phyre_Vector2_normalize_0062a0d0(out, v)` (cdecl), `_CIcos`.
pub const AXIS_CALLS: [u32; 4] = [0x0062_8470, 0x0062_3c90, 0x0062_a0d0, 0x0094_98c2];
/// The input semantics (`PInputTypeSemantic` vtables) the value is read
/// for, in the original's order: keys, pad stick, pad button, mouse
/// button, mouse X, mouse Y, touch X, touch Y, two pinches, touch
/// pressure, touch other, motion (four), motion rate (three), motion
/// gravity (three).
pub const AXIS_SEMANTICS: [u32; 22] = [
    0x00c6_8228,
    0x00c6_8240,
    0x00c6_8234,
    0x00c6_824c,
    0x00c6_8258,
    0x00c6_8264,
    0x00c6_8270,
    0x00c6_827c,
    0x00c6_8288,
    0x00c6_8294,
    0x00c6_82a0,
    0x00c6_82ac,
    0x00c6_82b8,
    0x00c6_82c4,
    0x00c6_82d0,
    0x00c6_82dc,
    0x00c6_82e8,
    0x00c6_82f4,
    0x00c6_8300,
    0x00c6_830c,
    0x00c6_8318,
    0x00c6_8324,
];
/// The constants: `g_ChrClothFlip` (-1.0), the mouse step (`f64` of
/// f32 0.05), the pinch angle (`f64` pi/6), `g_Vu0FixedRound` (0.5).
pub const AXIS_CONSTANTS: [u32; 4] = [0x00b9_2340, 0x00b4_12e8, 0x00b4_12f0, 0x00b9_2220];
/// `0x00628f00`'s frame below `ebp` (the normalised vectors down to
/// `ebp - 0x30`), in words.
pub const AXIS_FRAME_WORDS: u32 = 0x30 / 4;

pub mod axis {
    //! `0x00628f00` over game memory.

    use super::{AXIS_CALLS, AXIS_CONSTANTS, AXIS_SEMANTICS};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// A value through an `fstp m32` and back.
    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn f64_at(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// The deadzoned stick axis of a pad (`a` the axis compared, `b` the
    /// one read): the threshold `b+0x18` when `a+0x18 > |a+0x1c|` (else
    /// 0), the value `b+0x1c` (0 without `b` or the pad), 0 when its size
    /// is below the threshold; times `sign`.
    fn deadzone(h: &impl Host, pad: u32, (a_at, b_at): (u32, u32), sign: f64) -> u32 {
        let mut v = 0.0f64;
        let mut threshold = 0.0f64;
        if pad != 0 {
            let a: u32 = h.read(pad + a_at);
            let b: u32 = h.read(pad + b_at);
            let size = st(ld(h, a + 0x1c).abs());
            threshold = if ld(h, a + 0x18) > size {
                st(ld(h, b + 0x18))
            } else {
                0.0
            };
            if b != 0 {
                v = st(ld(h, b + 0x1c));
            }
        }
        if threshold > st(v.abs()) {
            v = 0.0;
        }
        x87::store(x87::mul(v, sign))
    }

    /// A pad button (`[pad + 0xd + key]` set): `first` (the value),
    /// times the trigger's own value for the analogue triggers 0x11 and
    /// 0x12 when the first pad (`this + 0x38`) is there (its slot `+0x10`
    /// with the key), `v` the value so far for the second; `None` when
    /// the button is not down.
    fn button(
        h: &mut impl Host,
        this: u32,
        pad: u32,
        key: u32,
        first: u32,
        scale: impl FnOnce(f64, f64) -> f64,
    ) -> Option<u32> {
        if pad == 0
            || key.wrapping_sub(0xb) >= 0x10
            || h.read::<u8>(pad.wrapping_add(0xd).wrapping_add(key)) == 0
        {
            return None;
        }
        if key != 0x12 && key != 0x11 {
            return Some(first);
        }
        let mut trigger = 0.0f64;
        let pad0: u32 = h.read(this + 0x38);
        if pad0 != 0 && key.wrapping_sub(0x11) <= 1 {
            let vt: u32 = h.read(pad0);
            let f: u32 = h.read(vt + 0x10);
            trigger = st(h.call_st0_f64(f, Conv::Thiscall(pad0), &[key]));
        }
        Some(x87::store(scale(trigger, x87::load(first))))
    }

    /// The motion values (device 4): the sign times its word at `+0xc0..`
    /// (four), `+0x30..` (three) or `+0x20..` (three) by semantic.
    fn motion(h: &impl Host, kind: u32, dev: u32, sign: f64) -> Option<u32> {
        let sem = AXIS_SEMANTICS.map(|a| h.rebase(a));
        let s = |k: u32| sem[k as usize];
        let at = match kind {
            k if k == s(12) => 0xc0,
            k if k == s(13) => 0xc4,
            k if k == s(14) => 0xc8,
            k if k == s(15) => 0xcc,
            k if k == s(16) => 0x30,
            k if k == s(17) => 0x34,
            k if k == s(18) => 0x38,
            k if k == s(19) => 0x20,
            k if k == s(20) => 0x24,
            k if k == s(21) => 0x28,
            _ => return None,
        };
        Some(x87::store(x87::mul(ld(h, dev + at), sign)))
    }

    /// A pinch (`+0x15f` set): the angle between the two touches'
    /// directions (`dev + 0x100` and `+0xe0` normalised into `n1`,
    /// `n2`; their dot product, the second products first, stored as a
    /// `float` and as a `double` at `ebp - 0x14`) under pi/6
    /// (`cos(pi/6) < dot`): half the sum of the two distances (`+0x108 +
    /// +0xe8` or `+0x10c + +0xec`) times `sign`.
    fn pinch(
        h: &mut impl Host,
        ebp: u32,
        dev: u32,
        (n1, n2): (u32, u32),
        (a, b): (u32, u32),
        sign: f64,
    ) -> Option<u32> {
        let [_, _, normalize, cos] = AXIS_CALLS;
        if h.read::<u8>(dev + 0x15f) == 0 {
            return None;
        }
        let p1 = h.call_sym(normalize, Conv::Cdecl, &[n1, dev + 0x100]);
        let p2 = h.call_sym(normalize, Conv::Cdecl, &[n2, dev + 0xe0]);
        let y = x87::mul(ld(h, p2 + 4), ld(h, p1 + 4));
        let x = st(x87::mul(ld(h, p1), ld(h, p2)));
        let dot = st(x87::add(y, x));
        h.write(ebp - 0x14, dot.to_bits());
        let angle = f64_at(h, AXIS_CONSTANTS[2]);
        let cos = h.rebase(cos);
        let c = f64::from(h.call_intrinsic(cos, angle));
        if c.partial_cmp(&dot) != Some(std::cmp::Ordering::Less) {
            return None;
        }
        let sum = st(x87::add(st(ld(h, dev + a)), st(ld(h, dev + b))));
        let half = f64_at(h, AXIS_CONSTANTS[3]);
        Some(x87::store(x87::mul(x87::mul(sum, sign), half)))
    }

    /// `Phyre_PFramework_PInputMapping_valueOf_00628f00(mapper; input)`
    /// (thiscall, ret 4) -> `ST0`: the input's (`semantic +8`, `index
    /// +0xc`, negated `+0x10`, keys `+0x18`/`+0x1c`, scale `+0x14`) value
    /// by its semantic: keys (devices `+0x80`), the pad stick (axis 1 via
    /// `getStickAxis`, 2, 4 and 5 deadzoned from the pad at `+0x38`), pad
    /// buttons (the two keys, the second negated), mouse buttons and
    /// steps (`+0xa0..+0xa2`, `+0xb0`/`+0xb4` times 0.05), touch values
    /// and pinches (device 3), motion (device 4); times the scale.
    #[allow(clippy::too_many_lines)] // the original's cascade of semantics
    pub fn value_of(h: &mut impl Host, ebp: u32, this: u32, input: u32) -> f32 {
        let [stick_axis, get_device, _, _] = AXIS_CALLS;
        let sem = AXIS_SEMANTICS.map(|a| h.rebase(a));
        let s = |k: usize| sem[k];
        let kind: u32 = h.read(input + 8);
        let index: u32 = h.read(input + 0xc);
        let minus = x87::store(x87::load(h.read(h.rebase(AXIS_CONSTANTS[0]))));
        let sign_bits = if h.read::<u8>(input + 0x10) == 0 {
            1.0f32.to_bits()
        } else {
            minus
        };
        let sign = x87::load(sign_bits);
        let neg = x87::store(-sign);
        let mut v = 0u32;
        let key1: u32 = h.read(input + 0x18);
        let key2: u32 = h.read(input + 0x1c);
        if kind == s(0) && index < 7 {
            let dev: u32 = h.read(this + 0x80 + 4 * index);
            if dev != 0 {
                let a = key1.wrapping_sub(0x1c);
                if a < 0x100 && h.read::<u8>(dev + 0x18 + a) != 0 {
                    v = sign_bits;
                }
                let b = key2.wrapping_sub(0x1c);
                if b < 0x100 && h.read::<u8>(dev + 0x18 + b) != 0 {
                    v = neg;
                }
            }
        }
        if kind == s(1) {
            let pad: u32 = h.read(this.wrapping_add(0x38).wrapping_add(index.wrapping_mul(4)));
            match key1 {
                1 => {
                    let r = h.call_sym_st0_f64(stick_axis, Conv::Thiscall(this), &[0, index]);
                    v = x87::store(x87::mul(r, sign));
                }
                2 => v = deadzone(h, pad, (0x38, 0x3c), -sign),
                4 => v = deadzone(h, pad, (0x44, 0x40), sign),
                5 => v = deadzone(h, pad, (0x40, 0x44), -sign),
                _ => {}
            }
        }
        if kind == s(2) && index < 0x12 {
            let pad: u32 = h.read(this + 0x38 + 4 * index);
            if let Some(r) = button(h, this, pad, key1, sign_bits, |t, _| x87::mul(t, sign)) {
                v = r;
            }
            let pad: u32 = h.read(this + 0x38 + 4 * index);
            if let Some(r) = button(h, this, pad, key2, neg, x87::mul) {
                v = r;
            }
        }
        if kind == s(3) {
            let byte = match key1 {
                0xb => Some(0xa0),
                0xc => Some(0xa2),
                0xd => Some(0xa1),
                _ => None,
            };
            if let Some(at) = byte {
                v = if h.read::<u8>(this + at) == 0 {
                    0
                } else {
                    1.0f32.to_bits()
                };
            }
        }
        let step = f64_at(h, AXIS_CONSTANTS[1]);
        for (k, word) in [(4usize, 0xb0u32), (5, 0xb4)] {
            if kind != s(k) {
                continue;
            }
            let on = match key1 {
                0 => true,
                0xb => h.read::<u8>(this + 0xa0) != 0,
                0xc => h.read::<u8>(this + 0xa2) != 0,
                0xd => h.read::<u8>(this + 0xa1) != 0,
                _ => false,
            };
            if on {
                let n = st(f64::from(h.read::<i32>(this + word)));
                v = x87::store(x87::mul(x87::mul(n, sign), step));
            }
        }
        let dev = h.call_sym(get_device, Conv::Cdecl, &[3, index]);
        if dev != 0 {
            let flag = |at: u32| h.read::<u8>(dev + at) != 0;
            let touch = flag(0x15c) && !flag(0x15e) && !flag(0x15f);
            let (pressed, held) = (flag(0x15e), flag(0x161));
            let times = |at: u32| x87::store(x87::mul(ld(h, dev + at), sign));
            let r = if kind == s(6) {
                touch.then(|| times(0x74))
            } else if kind == s(7) {
                touch.then(|| times(0x78))
            } else if kind == s(8) {
                pinch(h, ebp, dev, (ebp - 0x14, ebp - 0x20), (0x108, 0xe8), sign)
            } else if kind == s(9) {
                pinch(h, ebp, dev, (ebp - 0x28, ebp - 0x30), (0x10c, 0xec), sign)
            } else if kind == s(10) {
                pressed.then(|| times(0xcc))
            } else if kind == s(11) {
                held.then(|| times(0x158))
            } else {
                None
            };
            if let Some(r) = r {
                v = r;
            }
        }
        let dev = h.call_sym(get_device, Conv::Cdecl, &[4, index]);
        if dev != 0
            && let Some(r) = motion(h, kind, dev, sign)
        {
            v = r;
        }
        let scale = ld(h, input + 0x14);
        f32::from_bits(x87::store(x87::mul(scale, x87::load(v))))
    }
}
