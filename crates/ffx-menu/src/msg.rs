//! Message tables and message-window choices (`Msg_*`).
//!
//! An event's message table (`docs/formats/text.md`) is an array of 8-byte
//! entries (`u16` text offset, `u16` flags, `u16` alternate offset, `u16`
//! alternate flags) followed by the strings; offsets are from the table's
//! start. An offset of 0 falls back to entry *i* of the `.ebp`'s own table
//! (its text offset, also for the alternate). The script context keeps the
//! event's table at `+0x34` and a second one (`.ebp` section 5) at `+0x38`.

/// Bytes per message table entry.
pub const ENTRY: u32 = 8;

/// Where entry `i`'s text is: `table + offset`, or, when `offset` is 0,
/// `fallback + fallback_offset` (the `.ebp` table's entry `i`).
#[must_use]
pub const fn resolve(table: u32, offset: u16, fallback: u32, fallback_offset: u16) -> u32 {
    if offset == 0 {
        fallback.wrapping_add(fallback_offset as u32)
    } else {
        table.wrapping_add(offset as u32)
    }
}

/// A message window's choice cursors (bytes `+0x1e`, `+0x1f`, `+0x22`,
/// `+0x23`): two over the options, two over the number selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursors {
    /// `+0x1e`, compared signed against the option count.
    pub option: i8,
    /// `+0x1f`, likewise.
    pub option2: i8,
    /// `+0x22`, compared unsigned against the selector count.
    pub number: u8,
    /// `+0x23`, likewise.
    pub number2: u8,
}

/// `Msg_ClampChoices`: with mode bit 2, the cursors clamped to the counts
/// in the message's flags (`window +0x16`: options in the high byte, number
/// selector digits in the low nibble). Without options bit 2 goes and both
/// option cursors become 0; without digits bit `0x40` goes and both number
/// cursors become 0. Returns the new mode.
#[must_use]
pub fn clamp_choices(flags: u16, mode: u32, c: Cursors) -> (u32, Cursors) {
    if mode & 2 == 0 {
        return (mode, c);
    }
    let mut mode = mode;
    let mut c = c;
    let options = u32::from(flags >> 8);
    let digits = flags & 0xf;
    if digits == 0 {
        c.number = 0;
        c.number2 = 0;
        mode &= !0x40;
    } else {
        #[allow(clippy::cast_possible_truncation)] // a nibble
        let last = (digits as u8).wrapping_sub(1);
        if u16::from(c.number) >= digits {
            c.number = last;
        }
        if u16::from(c.number2) >= digits {
            c.number2 = last;
        }
    }
    if options == 0 {
        mode &= !2;
        c.option = 0;
        c.option2 = 0;
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // a byte
        let last = (options as u8).wrapping_sub(1) as i8;
        #[allow(clippy::cast_possible_wrap)] // at most 255
        let n = options as i32;
        if i32::from(c.option) >= n {
            c.option = last;
        }
        if i32::from(c.option2) >= n {
            c.option2 = last;
        }
    }
    (mode, c)
}

/// The originals, where the game keeps the tables (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{Cursors, ENTRY, clamp_choices, resolve};

    /// The script context (`+0x34` the event's message table, `+0x38` the
    /// second one).
    pub const SCRIPT_CONTEXT: Global<u32> = Global::new(0x0132_6b28);
    /// The last message index looked up (written by every lookup).
    pub const MSG_LAST_INDEX: Global<i32> = Global::new(0x0132_5b70);
    /// What the lookups return without a table (an empty string).
    pub const MSG_NONE: Global<u8> = Global::new(0x0132_5b74);
    /// The `.ebp`'s own message table (section 1), set by `AtelEventSetUp`.
    pub const EBP_MESSAGES: Global<u32> = Global::new(0x023c_bd60);

    #[allow(clippy::cast_sign_loss)] // the original scales the index unsigned
    fn entry(table: u32, i: i32) -> u32 {
        table.wrapping_add((i as u32).wrapping_mul(ENTRY))
    }

    /// Entry `i`'s text (`field` 0) or alternate (`field` 4) in `table`,
    /// with the `.ebp` fallback.
    fn text(mem: &impl Mem, table: u32, i: i32, field: u32) -> u32 {
        let offset = mem.read::<u16>(entry(table, i).wrapping_add(field));
        if offset == 0 {
            let fallback = EBP_MESSAGES.get(mem);
            resolve(table, 0, fallback, mem.read::<u16>(entry(fallback, i)))
        } else {
            resolve(table, offset, 0, 0)
        }
    }

    /// The context's table at `+field`, after recording `i`.
    fn context_table(mem: &mut impl Mem, i: i32, field: u32) -> u32 {
        MSG_LAST_INDEX.set(mem, i);
        let ctx = SCRIPT_CONTEXT.get(mem);
        mem.read::<u32>(ctx.wrapping_add(field))
    }

    /// `Msg_Flags(i)` (`0x0086bf10`): entry `i`'s flags, 0 without a table.
    pub fn msg_flags(mem: &impl Mem, i: i32) -> u16 {
        let ctx = SCRIPT_CONTEXT.get(mem);
        let table = mem.read::<u32>(ctx.wrapping_add(0x34));
        if table == 0 {
            return 0;
        }
        mem.read::<u16>(entry(table, i).wrapping_add(2))
    }

    /// `Msg_TextAlt(i)` (`0x0086ba90`): entry `i`'s alternate text.
    pub fn msg_text_alt(mem: &mut impl Mem, i: i32) -> u32 {
        let table = context_table(mem, i, 0x34);
        if table == 0 {
            return MSG_NONE.ptr(mem).addr;
        }
        text(mem, table, i, 4)
    }

    /// `Msg_Text1(i)` (`0x0086b170`): entry `i`'s text in the second table.
    pub fn msg_text1(mem: &mut impl Mem, i: i32) -> u32 {
        let table = context_table(mem, i, 0x38);
        if table == 0 {
            return MSG_NONE.ptr(mem).addr;
        }
        text(mem, table, i, 0)
    }

    /// `Msg_Get(i, alternate, second)` (`0x0086bf70`).
    pub fn msg_get(mem: &mut impl Mem, i: i32, alternate: i32, second: i32) -> u32 {
        let table = context_table(mem, i, if second == 0 { 0x34 } else { 0x38 });
        if table == 0 {
            return MSG_NONE.ptr(mem).addr;
        }
        text(mem, table, i, if alternate == 0 { 0 } else { 4 })
    }

    /// `Msg_TableText(table, i)` (`0x0086bed0`): entry `i`'s text in `table`.
    pub fn msg_table_text(mem: &impl Mem, table: u32, i: i32) -> u32 {
        text(mem, table, i, 0)
    }

    /// `Msg_TableFlags(table, i)` (`0x0086bf00`): entry `i`'s flags in
    /// `table`.
    pub fn msg_table_flags(mem: &impl Mem, table: u32, i: i32) -> u16 {
        mem.read::<u16>(entry(table, i).wrapping_add(2))
    }

    /// `Msg_TableTextAlt(table, i)` (`0x0086ba60`): its alternate.
    pub fn msg_table_text_alt(mem: &impl Mem, table: u32, i: i32) -> u32 {
        text(mem, table, i, 4)
    }

    /// `Msg_ClampChoices(window, mode)` (`0x00871210`): see
    /// [`clamp_choices`]; writes back the cursors it changes.
    pub fn msg_clamp_choices(mem: &mut impl Mem, window: u32, mode: u32) -> u32 {
        if mode & 2 == 0 {
            return mode;
        }
        let at = |o: u32| window.wrapping_add(o);
        let before = Cursors {
            option: mem.read(at(0x1e)),
            option2: mem.read(at(0x1f)),
            number: mem.read(at(0x22)),
            number2: mem.read(at(0x23)),
        };
        let (mode, after) = clamp_choices(mem.read(at(0x16)), mode, before);
        for (o, old, new) in [
            (
                0x1e,
                before.option.cast_unsigned(),
                after.option.cast_unsigned(),
            ),
            (
                0x1f,
                before.option2.cast_unsigned(),
                after.option2.cast_unsigned(),
            ),
            (0x22, before.number, after.number),
            (0x23, before.number2, after.number2),
        ] {
            if old != new {
                mem.write(at(o), new);
            }
        }
        mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_clamp() {
        let c = Cursors {
            option: 5,
            option2: -1,
            number: 9,
            number2: 1,
        };
        // 3 options, 4 digits
        let (mode, d) = clamp_choices(0x0304, 0x42, c);
        assert_eq!(mode, 0x42);
        assert_eq!((d.option, d.option2, d.number, d.number2), (2, -1, 3, 1));
        // no options, no digits
        let (mode, d) = clamp_choices(0, 0x42, c);
        assert_eq!((mode, d), (0, Cursors::default()));
        // without bit 2 nothing changes
        assert_eq!(clamp_choices(0, 0x40, c), (0x40, c));
        assert_eq!(resolve(0x100, 0, 0x200, 8), 0x208);
        assert_eq!(resolve(0x100, 4, 0x200, 8), 0x104);
    }
}
