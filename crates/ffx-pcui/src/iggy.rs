//! Iggy data values (the 16-byte `IggyDataValue` the game fills before
//! calling into `ActionScript`) and the five handler slots of an Iggy menu.

/// `IggyDataValue.type` of a boolean.
pub const TYPE_BOOLEAN: u32 = 3;
/// `IggyDataValue.type` of a number (an `f64`).
pub const TYPE_NUMBER: u32 = 4;
/// `IggyDataValue.type` of a UTF-8 string (pointer and length).
pub const TYPE_STRING_UTF8: u32 = 5;

/// What an `IggyDataValue` holds after one of the setters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataValue {
    /// A boolean, stored as the 32-bit zero-extended byte.
    Boolean(u8),
    /// A number.
    Number(f64),
    /// A string: its address and its length without the terminator.
    Utf8 {
        /// Where the text is (the caller's buffer, not copied).
        text: u32,
        /// `strlen(text)`.
        len: u32,
    },
}

/// The number `iggyDataValueSet(v, int)` stores: the integer goes through
/// a 32-bit float first (`fild`, `fstp dword`, `fld dword`, `fstp qword`),
/// so integers beyond 2^24 lose their low bits.
#[must_use]
pub fn number_from_int(value: i32) -> f64 {
    #[allow(clippy::cast_precision_loss)] // the original rounds to f32
    f64::from(value as f32)
}

/// Number of handler slots in an Iggy menu.
pub const SLOTS: usize = 5;

/// The first free slot of a menu's five, if any (`used` is each slot's
/// in-use byte).
#[must_use]
pub fn free_slot(used: [bool; SLOTS]) -> Option<usize> {
    used.iter().position(|&u| !u)
}

/// What the slot adders leave in `eax`: 0 when every slot is taken, else
/// `al = 1` over the upper bytes of `last`, the last argument they loaded
/// into `eax`.
#[must_use]
pub const fn added_eax(added: bool, last: u32) -> u32 {
    if added { (last & !0xff) | 1 } else { 0 }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{DataValue, TYPE_BOOLEAN, TYPE_NUMBER, TYPE_STRING_UTF8};

    /// `FUN_00a28c00(_, f, arg)`: the trampoline the simple handlers call,
    /// `f(arg)`.
    pub const CALL_ARG: u32 = 0x00a2_8c00;

    /// Stores `value` into the `IggyDataValue` at `v`.
    pub fn store(mem: &mut impl Mem, v: u32, value: DataValue) {
        match value {
            DataValue::Boolean(b) => {
                mem.write(v, TYPE_BOOLEAN);
                mem.write(v.wrapping_add(8), u32::from(b));
            }
            DataValue::Number(x) => {
                mem.write(v, TYPE_NUMBER);
                mem.write(v.wrapping_add(8), x);
            }
            DataValue::Utf8 { text, len } => {
                mem.write(v, TYPE_STRING_UTF8);
                mem.write(v.wrapping_add(8), text);
                mem.write(v.wrapping_add(0xc), len);
            }
        }
    }

    /// `strlen` over game memory.
    fn strlen(mem: &impl Mem, s: u32) -> u32 {
        let mut n = 0u32;
        while mem.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `iggyDataValueSet(v, int)` (`0x00a28d30`): a number; nothing when
    /// `v` is null.
    pub fn set_int(mem: &mut impl Mem, v: u32, value: i32) {
        if v != 0 {
            store(mem, v, DataValue::Number(super::number_from_int(value)));
        }
    }

    /// `iggyDataValueSet(v, const char *)` (`0x00a28d70`): a UTF-8 string
    /// pointing at `text`; nothing when `v` is null.
    pub fn set_utf8(mem: &mut impl Mem, v: u32, text: u32) {
        if v != 0 {
            let len = strlen(mem, text);
            store(mem, v, DataValue::Utf8 { text, len });
        }
    }

    /// `iggyDataValueSet(v, bool)` (`0x00a28db0`): a boolean from the low
    /// byte of `value`; nothing when `v` is null.
    pub fn set_bool(mem: &mut impl Mem, v: u32, value: u32) {
        if v != 0 {
            #[allow(clippy::cast_possible_truncation)] // the original reads the byte
            store(mem, v, DataValue::Boolean(value as u8));
        }
    }

    /// `IggyMenu::setEnable(this, on)` (`0x00a28ef0`): the byte at `+0x70`.
    pub fn set_enable(mem: &mut impl Mem, this: u32, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        mem.write(this.wrapping_add(0x70), on as u8);
    }

    /// Slot `i` of the menu at `this` (`+4`, 0x14 bytes each).
    const fn slot(this: u32, i: usize) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // i < 5
        this.wrapping_add(4 + 0x14 * i as u32)
    }

    fn free_slot(mem: &impl Mem, this: u32) -> Option<u32> {
        let used = std::array::from_fn(|i| mem.read::<u8>(slot(this, i)) != 0);
        super::free_slot(used).map(|i| slot(this, i))
    }

    /// `IggyMenuHandlers_add(this, a, b, c, handler)` (`0x00a28e40`,
    /// thiscall): the first free slot takes the four words and is marked
    /// used; returns [`super::added_eax`].
    pub fn add(mem: &mut impl Mem, this: u32, words: [u32; 4]) -> u32 {
        let Some(s) = free_slot(mem, this) else {
            return super::added_eax(false, 0);
        };
        mem.write(s.wrapping_add(4), words);
        mem.write(s, 1u8);
        super::added_eax(true, words[3])
    }

    /// `IggyMenuHandlers_addCall(this, f, arg)` (`0x00a28df0`, thiscall):
    /// like [`add`] with `(0, f, arg, CALL_ARG)`, a handler that calls
    /// `f(arg)`.
    pub fn add_call(mem: &mut impl Mem, this: u32, f: u32, arg: u32) -> u32 {
        let Some(s) = free_slot(mem, this) else {
            return super::added_eax(false, 0);
        };
        let call = mem.rebase(CALL_ARG);
        mem.write(s.wrapping_add(4), [0, f, arg, call]);
        mem.write(s, 1u8);
        super::added_eax(true, arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_round_through_f32() {
        assert_eq!(number_from_int(16_777_217), 16_777_216.0);
        assert_eq!(number_from_int(-3), -3.0);
    }

    #[test]
    fn slots() {
        assert_eq!(free_slot([true, true, false, false, true]), Some(2));
        assert_eq!(free_slot([true; SLOTS]), None);
        assert_eq!(added_eax(true, 0x1234_5678), 0x1234_5601);
        assert_eq!(added_eax(false, 0x1234_5678), 0);
    }
}
