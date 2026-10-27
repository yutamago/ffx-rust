//! Round h4x of list `h4-glue-other`: the escape menu's video setting page
//! (`FFEscMenu_initializeVideoSettingPage`, `0x006fcca0`).
//!
//! The page adds twelve rows with `callFlashFunction_6(this,
//! "addVideoPageItem", row, type, label, current, list, extra)`. Each row's
//! choices are an Iggy string array (`IggyStringArray_ctor_006fcbc0`) built
//! from a list of C strings: the screen resolutions (`"%dx%d"` into a heap
//! block), localised texts (`LocString` objects of 0x110 bytes in the
//! frame, destroyed after the row with `eh_vector_destructor_iterator`) or
//! the multipliers `"1x"` .. `"16x"` (built in the frame). The row's
//! current choice comes from `g_Settings` (directly, through a small table
//! or by a search) and is also stored in the page object (`+0xa4` ..
//! `+0xd0`). Last, under the render lock, the MSAA/HDAO pop-up message
//! (`callFlashFunction_3`) with whether the render interface's `+0xb4`
//! (`0x00c94fb4`) is at most `0xa000`.
//!
//! The values are built in the frame, at the original's `ebp` offsets
//! (callees see their addresses); the value setters never write word 1,
//! so the Flash calls pass what the frame holds there (0 in the game;
//! junk-read policy, brief 2026-10-24).

use ffx_graphic::host::{Conv, Host};

use crate::h4_j2::{
    CALL_FLASH_6, LOC_LOAD, LOC_TO_VALUE, PERCENT_D, SETTINGS, SPRINTF_IAT, VALUE_INT,
    VALUE_STRING, flash_args, value,
};
use crate::h4_j4::{CALL_FLASH_3, VALUE_BOOL};

/// `GetDerefU32_00a28d10(this)` (thiscall, no stack words): `[[this +
/// 0x6c]]`, the menu's Iggy root.
pub const IGGY_ROOT: u32 = 0x00a2_8d10;
/// `IggyStringArray_ctor_006fcbc0(array; root, names, n)` (thiscall, ret
/// 0xc) -> `array`: `+0` the root, `+4` the Iggy array of the `n` C
/// strings at `names`.
pub const STRING_ARRAY: u32 = 0x006f_cbc0;
/// `IggyValue_setType9_00a28dd0(value, v)` (cdecl): an array value.
pub const VALUE_ARRAY: u32 = 0x00a2_8dd0;
/// `_IggyValueRefFree@8`'s import slot (`0x00b0c820`, stdcall, 2 words).
pub const VALUE_REF_FREE_IAT: u32 = 0x00b0_c820;
/// `operator_new_006304c0(bytes)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_04c0;
/// `operator_delete_00630540(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0540;
/// `eh_vector_destructor_iterator(first, size, n, dtor)` (stdcall, 4
/// words).
pub const VECTOR_DTOR: u32 = 0x0094_96d2;
/// `LocString`'s destructor (`0x0068e260`), handed to [`VECTOR_DTOR`].
pub const LOC_DTOR: u32 = 0x0068_e260;
/// `PRenderScopeLock_enter_00463d50(lock)` (thiscall) -> `lock`.
pub const LOCK_ENTER: u32 = 0x0046_3d50;
/// `PRenderScopeLock_leave_00463e90(lock)` (thiscall): its `eax` is the
/// page's.
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The render interface singleton's guard word (`0x00c9628c`, bit 0).
pub const RENDER_GUARD: u32 = 0x00c9_628c;
/// The render interface singleton (`0x00c94f00`).
pub const RENDER_INTERFACE: u32 = 0x00c9_4f00;
/// `Phyre_PRendering_Construct583580_004fee70(this)` (thiscall): the
/// singleton's constructor.
pub const RENDER_CONSTRUCT: u32 = 0x004f_ee70;
/// `atexit` (`0x00949207`, cdecl).
pub const ATEXIT: u32 = 0x0094_9207;
/// The singleton's destructor (`0x00afb710`), handed to [`ATEXIT`].
pub const RENDER_DESTROY: u32 = 0x00af_b710;
/// The render interface's `+0xb4` (`0x00c94fb4`); the pop-up's flag is
/// whether it is at most `0xa000` (signed; likely the Direct3D feature
/// level, `0xa000` = 10.0).
pub const RENDER_LEVEL: u32 = 0x00c9_4fb4;

/// `"%dx%d"` (`0x00b4c6e8`).
pub const RESOLUTION_FORMAT: u32 = 0x00b4_c6e8;
/// `"spinnerWithList"` (`0x00b4c6f0`).
pub const SPINNER_WITH_LIST: u32 = 0x00b4_c6f0;
/// `"addVideoPageItem"` (`0x00b4c700`).
pub const ADD_ITEM: u32 = 0x00b4_c700;
/// `"spinner"` (`0x00b4c714`).
pub const SPINNER: u32 = 0x00b4_c714;
/// `"spinnerWithBar"` (`0x00b4c71c`).
pub const SPINNER_WITH_BAR: u32 = 0x00b4_c71c;
/// `"1x"`, `"2x"`, `"4x"`, `"8x"`: four 4-byte strings (`0x00b4c72c`); the
/// page copies three bytes of each.
pub const MULTIPLIERS: u32 = 0x00b4_c72c;
/// `"videoSettingMSAAHDAOPopUpMessageInit"` (`0x00b4c73c`).
pub const POPUP_INIT: u32 = 0x00b4_c73c;

/// Bytes of [`video_setting_page`]'s frame, from the original's `ebp -
/// 0xac8` up to `ebp`. The `ebp` offsets it uses:
/// - `0x7c`: the Iggy string array object ([`STRING_ARRAY`]'s `this`; two
///   words);
/// - `0x74`, `0xa4c`, `0xa5c`, `0xa6c`, `0xa7c`, `0xa8c`: the row's values
///   (16 bytes each);
/// - `0x7f0`: the label's `LocString`; `0x6e0`, `0x5d0`, `0x4c0`, `0x3b0`,
///   `0x2a0`, `0x190`: the choices' `LocString`s;
/// - the choices' pointer lists at `0xa3c` (two), `0x1c` (three), `0x20`
///   (four), `0x24` (five), `0x28` (six), each ending at `0x14` (or
///   `0xa38`);
/// - `0x1c`: the current resolution's text (row 0); `0x64`: the brightness
///   number (row 2, 0x20 bytes); `0x44` .. `0x2c`: the multipliers (row 5,
///   five strings of five bytes);
/// - `0x80` (three words), `0x74` (four words): index tables (rows 4, 6,
///   7); `0xa90`: row 6's index, row 7's fallback;
/// - `0xa98`: the render lock; `0xa10`, `0x900`: the pop-up's
///   `LocString`s; `0xaa8`, `0xab8`, `0xac8`: its values.
pub const VIDEO_FRAME: u32 = 0xac8;

/// The choices' `LocString` slots (`ebp` offsets); a row with `n` texts
/// uses the last `n`.
const LOC_SLOTS: [u32; 6] = [0x6e0, 0x5d0, 0x4c0, 0x3b0, 0x2a0, 0x190];

/// Where a row's choices come from.
enum Choices {
    /// Localised texts: the messages (in construction order) and the
    /// pointer list's `ebp` offset.
    Texts(&'static [u32], u32),
    /// The multipliers `"1x"` .. `"16x"` built in the frame.
    Multipliers,
    /// No list: the setting as a `"%d"` number, an empty array.
    Number,
}

/// How a row's setting becomes its current choice.
#[derive(Clone, Copy)]
enum Index {
    /// The setting itself.
    Direct,
    /// `table[setting]`, the table written to the frame at the `ebp`
    /// offset first.
    Table(u32, &'static [u32]),
    /// The position of the setting in the table (written at the `ebp`
    /// offset), or the previous row's index when it is not there.
    Search(u32, &'static [u32]),
}

/// Rows 1 .. 11.
struct Row {
    /// The `g_Settings` word.
    setting: u32,
    /// The page object's word that keeps the choice.
    field: u32,
    index: Index,
    choices: Choices,
    /// The last value's integer.
    extra: i32,
    /// The label's message.
    label: u32,
    /// The type's string.
    kind: u32,
}

const ROWS: [Row; 11] = [
    Row {
        setting: 0x08,
        field: 0xa8,
        index: Index::Direct,
        choices: Choices::Texts(&[0x235, 0x236, 0x237], 0x1c),
        extra: -1,
        label: 0x233,
        kind: SPINNER,
    },
    Row {
        setting: 0x24,
        field: 0xac,
        index: Index::Direct,
        choices: Choices::Number,
        extra: 0x32,
        label: 0x238,
        kind: SPINNER_WITH_BAR,
    },
    Row {
        setting: 0x28,
        field: 0xb0,
        index: Index::Direct,
        choices: Choices::Texts(&[0x23d, 0x23c], 0xa3c),
        extra: -1,
        label: 0x23a,
        kind: SPINNER,
    },
    Row {
        setting: 0x34,
        field: 0xb4,
        index: Index::Table(0x80, &[2, 1, 0]),
        choices: Choices::Texts(&[0x24b, 0x24c, 0x24d], 0x1c),
        extra: 1,
        label: 0x249,
        kind: SPINNER,
    },
    Row {
        setting: 0x2c,
        field: 0xb8,
        index: Index::Direct,
        choices: Choices::Multipliers,
        extra: 2,
        label: 0x23e,
        kind: SPINNER,
    },
    Row {
        setting: 0x30,
        field: 0xbc,
        index: Index::Table(0x74, &[0, 3, 2, 1]),
        choices: Choices::Texts(&[0x243, 0x246, 0x247, 0x248], 0x20),
        extra: 1,
        label: 0x244,
        kind: SPINNER,
    },
    Row {
        setting: 0x38,
        field: 0xc0,
        index: Index::Search(0x74, &[0, 2, 4, 8]),
        choices: Choices::Texts(&[0x250, 0x251, 0x252, 0x253], 0x20),
        extra: 1,
        label: 0x24e,
        kind: SPINNER,
    },
    Row {
        setting: 0x3c,
        field: 0xc4,
        index: Index::Direct,
        choices: Choices::Texts(&[0x257, 0x256], 0xa3c),
        extra: -1,
        label: 0x254,
        kind: SPINNER,
    },
    Row {
        setting: 0x40,
        field: 0xc8,
        index: Index::Direct,
        choices: Choices::Texts(&[0x25a, 0x25b, 0x25c, 0x25d, 0x25e, 0x25f], 0x28),
        extra: 1,
        label: 0x258,
        kind: SPINNER,
    },
    Row {
        setting: 0x44,
        field: 0xcc,
        index: Index::Direct,
        choices: Choices::Texts(&[0x263, 0x262], 0xa3c),
        extra: 1,
        label: 0x260,
        kind: SPINNER,
    },
    Row {
        setting: 0x48,
        field: 0xd0,
        index: Index::Direct,
        choices: Choices::Texts(&[0x266, 0x267, 0x268, 0x269], 0x20),
        extra: 1,
        label: 0x264,
        kind: SPINNER,
    },
];

/// The page's frame: `ebp - off` for the original's offsets.
#[derive(Clone, Copy)]
struct Frame(u32);

impl Frame {
    fn at(self, off: u32) -> u32 {
        self.0.wrapping_sub(off)
    }
}

/// `FFEscMenu_initializeVideoSettingPage(this)` (`0x006fcca0`, thiscall;
/// `frame` see [`VIDEO_FRAME`]): the resolution row when the page has a
/// resolution list (`+0xdc`), the eleven rows of [`ROWS`], then the
/// MSAA/HDAO pop-up. Returns the render lock's `leave` `eax`.
///
/// A setting out of its list's range picks a pointer from beyond the list
/// in the frame, as the original does (the game's settings are in range).
pub fn video_setting_page(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let f = Frame(frame.wrapping_add(VIDEO_FRAME));
    let resolutions: u32 = h.read(this.wrapping_add(0xdc));
    if resolutions != 0 {
        resolution_row(h, f, this, resolutions);
    }
    let mut previous = 0;
    for (k, row) in ROWS.iter().enumerate() {
        previous = choice_row(h, f, this, k as u32 + 1, row, previous);
    }
    popup(h, f, this)
}

/// Row 0: the screen resolutions (`+0xd4` of them, 0x1c bytes each at
/// `+0xdc`, the current one `+0xd8`), each `"%dx%d"` into a heap block of
/// 10 bytes per entry, listed in a second block; afterwards `+0xa4` is the
/// current one and both blocks are freed.
fn resolution_row(h: &mut impl Host, f: Frame, this: u32, resolutions: u32) {
    let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
    let format = h.rebase(RESOLUTION_FORMAT);
    let current: u32 = h.read(this.wrapping_add(0xd8));
    let entry = resolutions.wrapping_add(current.wrapping_mul(0x1c));
    let (width, height): (u32, u32) = (h.read(entry), h.read(entry.wrapping_add(4)));
    h.call(sprintf, Conv::Cdecl, &[f.at(0x1c), format, width, height]);
    let count: u32 = h.read(this.wrapping_add(0xd4));
    let texts = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[array_bytes(count, 10)]);
    let count: u32 = h.read(this.wrapping_add(0xd4));
    let list = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[array_bytes(count, 4)]);
    let mut k = 0u32;
    let mut text = texts;
    while k < h.read::<u32>(this.wrapping_add(0xd4)) {
        h.write(list.wrapping_add(4 * k), text);
        let table: u32 = h.read(this.wrapping_add(0xdc));
        let entry = table.wrapping_add(k.wrapping_mul(0x1c));
        let (width, height): (u32, u32) = (h.read(entry), h.read(entry.wrapping_add(4)));
        h.call(sprintf, Conv::Cdecl, &[text, format, width, height]);
        k += 1;
        text = text.wrapping_add(10);
    }
    h.call_sym(VALUE_INT, Conv::Cdecl, &[f.at(0xa4c), u32::MAX]);
    let count: u32 = h.read(this.wrapping_add(0xd4));
    let root = h.call_sym(IGGY_ROOT, Conv::Thiscall(this), &[]);
    let array = h.call_sym(
        STRING_ARRAY,
        Conv::Thiscall(f.at(0x7c)),
        &[root, list, count],
    );
    let iggy: u32 = h.read(array.wrapping_add(4));
    h.call_sym(VALUE_ARRAY, Conv::Cdecl, &[f.at(0xa8c), iggy]);
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[f.at(0xa5c), f.at(0x1c)]);
    let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(f.at(0x7f0)), &[0x231]);
    h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[f.at(0xa6c)]);
    let kind = h.rebase(SPINNER_WITH_LIST);
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[f.at(0xa7c), kind]);
    h.call_sym(VALUE_INT, Conv::Cdecl, &[f.at(0x74), 0]);
    let values = [0x74, 0xa7c, 0xa6c, 0xa5c, 0xa8c, 0xa4c].map(|off| value(h, f.at(off)));
    let name = h.rebase(ADD_ITEM);
    h.call_sym_wide(CALL_FLASH_6, &flash_args::<26>(this, name, &values));
    free_array(h, f);
    let current: u32 = h.read(this.wrapping_add(0xd8));
    h.write(this.wrapping_add(0xa4), current);
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[texts]);
    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[list]);
}

/// `n * size` bytes, or `u32::MAX` when that overflows (the original's
/// `mul`/`seto`/`neg`/`or`).
fn array_bytes(n: u32, size: u32) -> u32 {
    n.saturating_mul(size)
}

/// `_IggyValueRefFree@8` of the string array object's two words.
fn free_array(h: &mut impl Host, f: Frame) {
    let free: u32 = h.read(h.rebase(VALUE_REF_FREE_IAT));
    let (root, iggy): (u32, u32) = (h.read(f.at(0x7c)), h.read(f.at(0x78)));
    h.call(free, Conv::Stdcall, &[root, iggy]);
}

/// Row `number` of [`ROWS`]; `previous` is the index row 6 left at `ebp -
/// 0xa90`. Returns that word afterwards.
fn choice_row(
    h: &mut impl Host,
    f: Frame,
    this: u32,
    number: u32,
    row: &Row,
    previous: u32,
) -> u32 {
    // the choices' C strings, and their list
    let (list, n) = match row.choices {
        Choices::Texts(messages, list) => {
            let slots = &LOC_SLOTS[LOC_SLOTS.len() - messages.len()..];
            for (&message, &slot) in messages.iter().zip(slots) {
                h.call_sym(LOC_LOAD, Conv::Thiscall(f.at(slot)), &[message]);
            }
            for (k, &slot) in slots.iter().enumerate() {
                h.write(f.at(list) + 4 * k as u32, f.at(slot));
            }
            (f.at(list), messages.len() as u32)
        }
        Choices::Multipliers => {
            multipliers(h, f);
            (f.at(0x24), 5)
        }
        Choices::Number => (0, 0),
    };
    let settings: u32 = h.read(h.rebase(SETTINGS));
    let setting: u32 = h.read(settings.wrapping_add(row.setting));
    let index = match row.index {
        Index::Direct => setting,
        Index::Table(at, table) => {
            write_table(h, f, at, table);
            h.read(f.at(at).wrapping_add(setting.wrapping_mul(4)))
        }
        Index::Search(at, table) => {
            write_table(h, f, at, table);
            table
                .iter()
                .position(|&v| v == setting)
                .map_or(previous, |k| k as u32)
        }
    };
    // row 6 (the only table at `ebp - 0x74`) keeps its index at `ebp -
    // 0xa90` for row 7
    let mut previous = previous;
    if let Index::Table(0x74, _) = row.index {
        h.write(f.at(0xa90), index);
        previous = index;
    }
    h.write(this.wrapping_add(row.field), index);
    if let Choices::Number = row.choices {
        let sprintf: u32 = h.read(h.rebase(SPRINTF_IAT));
        let format = h.rebase(PERCENT_D);
        h.call(sprintf, Conv::Cdecl, &[f.at(0x64), format, setting]);
    }
    h.call_sym(VALUE_INT, Conv::Cdecl, &[f.at(0x74), row.extra as u32]);
    let root = h.call_sym(IGGY_ROOT, Conv::Thiscall(this), &[]);
    let array = h.call_sym(STRING_ARRAY, Conv::Thiscall(f.at(0x7c)), &[root, list, n]);
    let iggy: u32 = h.read(array.wrapping_add(4));
    h.call_sym(VALUE_ARRAY, Conv::Cdecl, &[f.at(0xa7c), iggy]);
    let current = match row.choices {
        Choices::Texts(..) => h.read(list.wrapping_add(index.wrapping_mul(4))),
        Choices::Multipliers => f.at(0x44).wrapping_add(index.wrapping_mul(5)),
        Choices::Number => f.at(0x64),
    };
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[f.at(0xa6c), current]);
    let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(f.at(0x7f0)), &[row.label]);
    h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[f.at(0xa5c)]);
    let kind = h.rebase(row.kind);
    h.call_sym(VALUE_STRING, Conv::Cdecl, &[f.at(0xa8c), kind]);
    h.call_sym(VALUE_INT, Conv::Cdecl, &[f.at(0xa4c), number]);
    let values = [0xa4c, 0xa8c, 0xa5c, 0xa6c, 0xa7c, 0x74].map(|off| value(h, f.at(off)));
    let name = h.rebase(ADD_ITEM);
    h.call_sym_wide(CALL_FLASH_6, &flash_args::<26>(this, name, &values));
    free_array(h, f);
    if let Choices::Texts(messages, _) = row.choices {
        let n = messages.len() as u32;
        let first = f.at(LOC_SLOTS[LOC_SLOTS.len() - messages.len()]);
        let dtor = h.rebase(LOC_DTOR);
        h.call_sym(VECTOR_DTOR, Conv::Stdcall, &[first, 0x110, n, dtor]);
    }
    previous
}

/// An index table's words at `ebp - at`.
fn write_table(h: &mut impl Host, f: Frame, at: u32, table: &[u32]) {
    for (k, &v) in table.iter().enumerate() {
        h.write(f.at(at) + 4 * k as u32, v);
    }
}

/// Row 5's choices: `"1x"`, `"2x"`, `"4x"`, `"8x"` (three bytes of each of
/// [`MULTIPLIERS`], two zero bytes after) at `ebp - 0x44` .. `- 0x30`,
/// `"16x"` at `ebp - 0x30` and a zero byte at `ebp - 0x2c`; their list at
/// `ebp - 0x24`.
fn multipliers(h: &mut impl Host, f: Frame) {
    let source = h.rebase(MULTIPLIERS);
    for k in 0..4u32 {
        let at = f.at(0x44) + 5 * k;
        let head: u16 = h.read(source + 4 * k);
        let tail: u8 = h.read(source + 4 * k + 2);
        h.write(at, head);
        h.write(at + 2, tail);
        h.write(at + 3, 0u16);
    }
    h.write(f.at(0x2c), 0u8);
    for k in 0..5u32 {
        h.write(f.at(0x24) + 4 * k, f.at(0x44) + 5 * k);
    }
    h.write(f.at(0x30), 0x0078_3631u32);
}

/// The MSAA/HDAO pop-up: under the render lock (`ebp - 0xa98`), the
/// render interface made on first use, then `callFlashFunction_3(this,
/// "videoSettingMSAAHDAOPopUpMessageInit", [+0xb4] <= 0xa000, message
/// 0x38f's text, message 0x390's text)`. Returns the lock's `leave` `eax`.
fn popup(h: &mut impl Host, f: Frame, this: u32) -> u32 {
    let lock = f.at(0xa98);
    h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
    let guard = h.rebase(RENDER_GUARD);
    let bits: u32 = h.read(guard);
    if bits & 1 == 0 {
        h.write(guard, bits | 1);
        let interface = h.rebase(RENDER_INTERFACE);
        h.call_sym(RENDER_CONSTRUCT, Conv::Thiscall(interface), &[]);
        let destroy = h.rebase(RENDER_DESTROY);
        h.call_sym(ATEXIT, Conv::Cdecl, &[destroy]);
    }
    let level: i32 = h.read(h.rebase(RENDER_LEVEL));
    let low = u32::from(level <= 0xa000);
    let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(f.at(0xa10)), &[0x390]);
    h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[f.at(0xac8)]);
    let loc = h.call_sym(LOC_LOAD, Conv::Thiscall(f.at(0x900)), &[0x38f]);
    h.call_sym(LOC_TO_VALUE, Conv::Thiscall(loc), &[f.at(0xab8)]);
    h.call_sym(VALUE_BOOL, Conv::Cdecl, &[f.at(0xaa8), low]);
    let values = [0xaa8, 0xab8, 0xac8].map(|off| value(h, f.at(off)));
    let name = h.rebase(POPUP_INIT);
    h.call_sym_wide(CALL_FLASH_3, &flash_args::<14>(this, name, &values));
    h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[])
}
