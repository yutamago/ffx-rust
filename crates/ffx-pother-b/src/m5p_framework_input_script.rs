//! The application script (list `phyre-framework-input`, batch 6): the
//! script proxy's command table, the command parser and dispatcher, the
//! script buffer's loader, and the pad's mapping bind.
//!
//! A script is text of `name(args)` commands, blanks and `#` comment lines
//! between them. The proxy's table (`+0x14`, count `+0x18`) holds 0x48-byte
//! records: the handler (cdecl `(owner, args)`) and its name (0x40 bytes
//! at `+4`). The repeat command (the table's first name, `0x00b401a8`)
//! makes the next command run `n` more times (`+0x24`).

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// The proxy's commands, in registration order: `(handler, name)`.
pub const COMMANDS: [(u32, u32); 16] = [
    (0x0062_3660, 0x00b4_01a8),
    (0x0062_5720, 0x00b4_01d0),
    (0x0062_35d0, 0x00b4_01dc),
    (0x0062_4900, 0x00b4_01e8),
    (0x0062_4d70, 0x00b4_01f8),
    (0x0062_4890, 0x00b4_0204),
    (0x0062_4810, 0x00b4_0210),
    (0x0062_4850, 0x00b4_0218),
    (0x0062_3060, 0x00b4_0220),
    (0x0062_2d40, 0x00b4_022c),
    (0x0062_2ed0, 0x00b4_0238),
    (0x0062_4910, 0x00b4_0244),
    (0x0062_4410, 0x00b4_0250),
    (0x0062_3460, 0x00b4_025c),
    (0x0062_3310, 0x00b4_0264),
    (0x0062_4d30, 0x00b4_026c),
];
/// `PApplicationScriptProxy::addCommand(handler, name)` (thiscall,
/// `ret 8`; in the list).
pub const ADD_COMMAND: u32 = 0x0062_bab0;
/// The repeat command's name.
pub const REPEAT_NAME: u32 = 0x00b4_01a8;
/// `"%d"`.
pub const FORMAT_INT: u32 = 0x00b0_f604;
/// `"Unrecognised command %s(%s)\n"`.
pub const MSG_UNRECOGNISED: u32 = 0x00b4_01b0;
/// `"Badly formed command ..."` (printed with the command start).
pub const MSG_BADLY_FORMED: u32 = 0x00b4_018c;
/// `"Unable to open script %s"`-style message of the loader.
pub const MSG_OPEN_FAILED: u32 = 0x00b4_015c;
/// `Phyre::PhyrePrintf(level, fmt, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// Import slots (cdecl): `_stricmp`, `sscanf_s`.
pub const STRICMP_SLOT: u32 = 0x00b0_c398;
pub const SSCANF_SLOT: u32 = 0x00b0_c418;
/// The script buffer's scanners (thiscall; in the list).
pub const SKIP_BLANK: u32 = 0x0062_cf90;
pub const FIND_SPACE: u32 = 0x0062_7810;
/// `PApplicationScriptBuffer::findNextCommand(&name, &args)` (thiscall,
/// `ret 8`; in the list).
pub const FIND_NEXT_COMMAND: u32 = 0x0062_76e0;
/// `PStreamReaderFile` on the loader's frame: constructor `(path)`
/// (thiscall, `ret 4`), is-open, read-all into a `PArray` `(array, 1)`
/// (thiscall, `ret 8`; in the list), the file's close, destructor.
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_READ_ALL: u32 = 0x0057_dd60;
pub const FILE_CLOSE: u32 = 0x0060_7d80;
pub const READER_DTOR: u32 = 0x0057_67c0;
/// `PApplicationScriptBuffer`'s vtable.
pub const SCRIPT_BUFFER_VTABLE: u32 = 0x00b3_fc8c;

/// The pad field an axis id 1..=10 reads (`+0x370..+0x394`), or `None`.
#[must_use]
pub const fn pad_axis_field(id: u32) -> Option<u32> {
    match id {
        1 => Some(0x370),
        2 => Some(0x37c),
        3 => Some(0x388),
        4 => Some(0x374),
        5 => Some(0x380),
        6 => Some(0x38c),
        7 => Some(0x378),
        8 => Some(0x384),
        9 => Some(0x390),
        10 => Some(0x394),
        _ => None,
    }
}

pub mod bridge {
    use super::{
        ADD_COMMAND, COMMANDS, Conv, FILE_CLOSE, FIND_NEXT_COMMAND, FIND_SPACE, FORMAT_INT, Host,
        MSG_BADLY_FORMED, MSG_OPEN_FAILED, MSG_UNRECOGNISED, Mem, PRINTF, READER_CTOR, READER_DTOR,
        READER_IS_OPEN, READER_READ_ALL, REPEAT_NAME, SCRIPT_BUFFER_VTABLE, SKIP_BLANK,
        SSCANF_SLOT, STRICMP_SLOT, pad_axis_field,
    };

    fn import(h: &impl Mem, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// The first `c` in `[start, end)`, or 0.
    fn find_byte(m: &impl Mem, start: u32, end: u32, c: u8) -> u32 {
        let mut p = start;
        while p < end {
            if m.read::<u8>(p) == c {
                return p;
            }
            p += 1;
        }
        0
    }

    /// `PApplicationScriptProxy::registerScriptCommands` (`0x0062bb60`):
    /// the sixteen [`COMMANDS`] added in order.
    pub fn register_commands(h: &mut impl Host, this: u32) {
        for (handler, name) in COMMANDS {
            let handler = h.rebase(handler);
            let name = h.rebase(name);
            h.call_sym(ADD_COMMAND, Conv::Thiscall(this), &[handler, name]);
        }
    }

    /// `PApplicationScriptBuffer::findNextCommand(&name, &args)`
    /// (`0x006276e0`, `ret 8`): the next `name(args)` after blanks;
    /// `*name` its start, `*args` the text inside the parentheses (both
    /// NUL-terminated in place), the cursor past it (the next blank, or
    /// the end of the text). `al` 1 when well formed; 0 at the end, with
    /// no parentheses, or out of order ("badly formed" printed). The rest
    /// of `eax` as the original leaves it.
    pub fn find_next_command(h: &mut impl Host, this: u32, name_out: u32, args_out: u32) -> u32 {
        if h.call_sym(SKIP_BLANK, Conv::Thiscall(this), &[]) == 0 {
            return 0;
        }
        let start: u32 = h.read(this + 0xc);
        h.write(name_out, start);
        let end: u32 = h.read(this + 0x10);
        let open = find_byte(h, start, end, b'(');
        let close = find_byte(h, start, end, b')');
        if open == 0 || close == 0 {
            return end & 0xffff_ff00;
        }
        h.write(this + 0xc, close);
        let mut next = h.call_sym(FIND_SPACE, Conv::Thiscall(this), &[]);
        if next == 0 {
            let at: u32 = h.read(this + 0xc);
            let mut p = at;
            while h.read::<u8>(p) != 0 {
                p += 1;
            }
            next = p;
        }
        h.write(this + 0xc, next);
        let name: u32 = h.read(name_out);
        if name < open && open < close && close < next {
            h.write(args_out, open + 1);
            h.write(open, 0u8);
            h.write(close, 0u8);
            return (args_out & 0xffff_ff00) | 1;
        }
        let msg = h.rebase(MSG_BADLY_FORMED);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, name]) & 0xffff_ff00
    }

    /// `PApplicationScript::executeNextCommand` (`0x00627480`): while a
    /// repeat is pending (`+0x24`) the repeat command's handler again
    /// (`+0x24` one less), with no arguments; else the next command
    /// (through the frame words `frame` = args, `frame + 4` = name):
    /// the repeat command sets `+0x24` from its `%d` argument (0), a known
    /// name runs its handler `(owner +0x20, args)` (its `eax`), anything
    /// else prints "Unrecognised command" (0x11). 0x11 at the end.
    pub fn execute_next(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let args_at = frame;
        let name_at = frame + 4;
        h.write(name_at, 0u32);
        h.write(args_at, 0u32);
        let stricmp = import(h, STRICMP_SLOT);
        let repeat = h.rebase(REPEAT_NAME);
        if h.read::<u32>(this + 0x24) != 0 {
            let mut i = 0;
            while i < h.read::<u32>(this + 0x18) {
                let table: u32 = h.read(this + 0x14);
                let rec = table.wrapping_add(i.wrapping_mul(0x48));
                if h.call(stricmp, Conv::Cdecl, &[rec + 4, repeat]) == 0 {
                    let left: u32 = h.read(this + 0x24);
                    h.write(this + 0x24, left.wrapping_sub(1));
                    let handler: u32 = h.read(rec);
                    let owner: u32 = h.read(this + 0x20);
                    return h.call(handler, Conv::Cdecl, &[owner, 0]);
                }
                i += 1;
            }
            return 0x11;
        }
        let ok = h.call_sym(FIND_NEXT_COMMAND, Conv::Thiscall(this), &[name_at, args_at]);
        if ok as u8 == 0 {
            return 0x11;
        }
        let mut i = 0;
        while i < h.read::<u32>(this + 0x18) {
            let name: u32 = h.read(name_at);
            if h.call(stricmp, Conv::Cdecl, &[name, repeat]) == 0 {
                let args: u32 = h.read(args_at);
                let sscanf = import(h, SSCANF_SLOT);
                let format = h.rebase(FORMAT_INT);
                h.call(sscanf, Conv::Cdecl, &[args, format, name_at]);
                let n: u32 = h.read(name_at);
                h.write(this + 0x24, n);
                return 0;
            }
            let table: u32 = h.read(this + 0x14);
            let rec = table.wrapping_add(i.wrapping_mul(0x48));
            let name: u32 = h.read(name_at);
            if h.call(stricmp, Conv::Cdecl, &[rec + 4, name]) == 0 {
                let handler: u32 = h.read(rec);
                let owner: u32 = h.read(this + 0x20);
                let args: u32 = h.read(args_at);
                return h.call(handler, Conv::Cdecl, &[owner, args]);
            }
            i += 1;
        }
        let name: u32 = h.read(name_at);
        let args: u32 = h.read(args_at);
        let msg = h.rebase(MSG_UNRECOGNISED);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, name, args]);
        0x11
    }

    /// `PApplicationScriptBuffer::PApplicationScriptBuffer(path)`
    /// (`0x0061fc20`, `ret 4`): its vtable and empty fields; a
    /// `PStreamReaderFile` on the frame block `reader` (0x20 bytes, the
    /// file at `+0x14`) opens `path`; when open, the whole file is read
    /// into the buffer's `PArray` (`+4`) and on success the cursor
    /// (`+0xc`) and end (`+0x10`) span it, then the file is closed; when
    /// not, "unable to open" is printed; the reader destroyed; `this`.
    pub fn buffer_ctor(h: &mut impl Host, this: u32, path: u32, reader: u32) -> u32 {
        let vt = h.rebase(SCRIPT_BUFFER_VTABLE);
        h.write(this, vt);
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0u32);
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]);
        if open as u8 == 0 {
            let msg = h.rebase(MSG_OPEN_FAILED);
            h.call_sym(PRINTF, Conv::Cdecl, &[1, msg, path]);
        } else {
            let r = h.call_sym(READER_READ_ALL, Conv::Thiscall(reader), &[this + 4, 1]);
            if r == 0 {
                let count: u32 = h.read(this + 4);
                let data: u32 = h.read(this + 8);
                h.write(this + 0xc, data);
                h.write(this + 0x10, (count & 0x7fff_ffff).wrapping_add(data));
            }
            h.call_sym(FILE_CLOSE, Conv::Thiscall(reader + 0x14), &[]);
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        this
    }

    /// `PInputDevicePad::bind(mapping, id)` (`0x00625dd0`, `ret 8`): a
    /// button id (0xb..=0x1a) reads the byte `this + 0x38d + id` (`+0x28`),
    /// an axis id (1..=10) the word [`pad_axis_field`] (`+0x2c`); 0x12 for
    /// a mapping already bound, 5 for other ids; else the mapping keeps
    /// the device and id and joins its list (tail of `this + 0xc`); 0.
    pub fn pad_bind(m: &mut impl Mem, this: u32, mapping: u32, id: u32) -> u32 {
        let (byte, word) = if id.wrapping_sub(0xb) <= 0xf {
            (this.wrapping_add(0x38d).wrapping_add(id), 0)
        } else if let Some(field) = pad_axis_field(id) {
            (0, this + field)
        } else {
            return 5;
        };
        if m.read::<u32>(mapping + 8) != 0 {
            return 0x12;
        }
        m.write(mapping + 0x2c, word);
        m.write(mapping + 0x28, byte);
        m.write(mapping + 8, this);
        m.write(mapping + 0xc, id);
        let list = this + 0xc;
        let last: u32 = m.read(list + 4);
        m.write(mapping + 4, last);
        m.write(mapping, list);
        m.write(last, mapping);
        m.write(list + 4, mapping);
        0
    }
}
