//! M5 `system-a`: the PC settings object (`GameSetting.ini`,
//! `docs/formats/platform.md`) and the key-name tables of the PC menus.
//!
//! The settings object (0x63c bytes): 24 words of values (`+0x00`; the
//! quality preset at `+0x0c`), three binding tables of 81 words (`+0x60`
//! FFX, `+0x1a4` FFX-2, `+0x2e8` Last Mission), the ini path (UTF-16,
//! `+0x430`), and the flags `+0x638` (word), `+0x63a` (byte).
//!
//! The preset table (`0x00b7cba8`, game data read from `FFX.exe`) has 24
//! rows of four words: presets 0..2 (preset 0's column is also the
//! constructor's defaults) and a fourth word no function here reads; the
//! byte mask at `0x00b7cb90` says which values a preset fixes.

/// The preset table: 24 rows of 16 bytes (presets 0..2, then a word).
pub const PRESETS: u32 = 0x00b7_cba8;
/// Which of the 24 values the presets fix (one byte each).
pub const PRESET_MASK: u32 = 0x00b7_cb90;
/// `crt_memset_thunk` (`0x0094964c`, cdecl, `MSVCR110!memset`).
pub const MEMSET: u32 = 0x0094_964c;

/// The keyboard key-name table (`0x00c8abe0`, 0x80 words).
pub const KEYBOARD_NAMES: u32 = 0x00c8_abe0;
/// The controller key-name table (`0x00c8a680`, 0x80 words).
pub const CONTROLLER_NAMES: u32 = 0x00c8_a680;

/// `Settings_IsFfxBindingSlot(slot)`: the FFX table's slots outside
/// 0x29..=0x30 and 0x4b..=0x50 but for 0x1e, 0x20, 0x23, 0x41, 0x43 and
/// 0x46 (signed compares).
#[must_use]
pub const fn is_ffx_slot(slot: i32) -> bool {
    !(matches!(slot, 0x29..=0x30 | 0x4b..=0x50)
        || matches!(slot, 0x23 | 0x46 | 0x1e | 0x20 | 0x41 | 0x43))
}

/// `Settings_IsFfx2BindingSlot(slot)`: outside 0x2e..=0x30 and
/// 0x4e..=0x50 but for 0x1e, 0x20, 0x23..=0x27, 0x41, 0x43 and
/// 0x46..=0x4a.
#[must_use]
pub const fn is_ffx2_slot(slot: i32) -> bool {
    !(matches!(slot, 0x2e..=0x30 | 0x4e..=0x50)
        || matches!(
            slot,
            0x23 | 0x46
                | 0x24
                | 0x25
                | 0x26
                | 0x27
                | 0x47
                | 0x48
                | 0x49
                | 0x4a
                | 0x1e
                | 0x20
                | 0x41
                | 0x43
        ))
}

/// `Settings_IsLmBindingSlot(slot)`: every slot but 0x1d, 0x1e, 0x20,
/// 0x22..=0x27, 0x29, 0x2a, 0x2c, 0x2d, 0x40, 0x41, 0x43, 0x45..=0x4c.
#[must_use]
pub const fn is_lm_slot(slot: i32) -> bool {
    !matches!(
        slot,
        0x2c | 0x2d
            | 0x1d
            | 0x22
            | 0x40
            | 0x45
            | 0x29
            | 0x2a
            | 0x4b
            | 0x4c
            | 0x23
            | 0x46
            | 0x24
            | 0x25
            | 0x26
            | 0x27
            | 0x47
            | 0x48
            | 0x49
            | 0x4a
            | 0x1e
            | 0x20
            | 0x41
            | 0x43
    )
}

/// The originals over game memory.
pub mod bridge {
    use super::{CONTROLLER_NAMES, KEYBOARD_NAMES, MEMSET, PRESET_MASK, PRESETS};
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    /// A slot filter's `eax`: `al` the answer, the upper bytes the
    /// slot's (the originals compare in `eax` and set `al`).
    #[must_use]
    pub const fn slot_eax(slot: u32, yes: bool) -> u32 {
        (slot & !0xff) | yes as u32
    }

    /// `Settings_IsFfxBindingSlot(slot)` (cdecl) -> `eax` ([`slot_eax`]).
    #[must_use]
    pub const fn is_ffx_slot(slot: u32) -> u32 {
        slot_eax(slot, super::is_ffx_slot(slot.cast_signed()))
    }

    /// `Settings_IsFfx2BindingSlot(slot)` (cdecl) -> `eax`.
    #[must_use]
    pub const fn is_ffx2_slot(slot: u32) -> u32 {
        slot_eax(slot, super::is_ffx2_slot(slot.cast_signed()))
    }

    /// `Settings_IsLmBindingSlot(slot)` (cdecl) -> `eax`.
    #[must_use]
    pub const fn is_lm_slot(slot: u32) -> u32 {
        slot_eax(slot, super::is_lm_slot(slot.cast_signed()))
    }

    /// A key-name table's word at `slot` when `slot` is 0..0x80, else
    /// `None`.
    fn name_at(mem: &impl Mem, table: u32, slot: u32) -> Option<u32> {
        if slot >= 0x80 {
            // also the negative slots (`js`)
            return None;
        }
        Some(mem.read(mem.rebase(table).wrapping_add(4 * slot)))
    }

    /// `0x00405a90(slot)` / `0x00405ac0(slot)` (cdecl): whether the
    /// keyboard (`KEYBOARD_NAMES`) or controller table has a name at
    /// `slot`. Returns `eax` (`al` the answer, the upper bytes the
    /// slot's).
    pub fn has_name(mem: &impl Mem, table: u32, slot: u32) -> u32 {
        let yes = name_at(mem, table, slot).is_some_and(|n| n != 0);
        slot_eax(slot, yes)
    }

    /// `KeyNames_getKeyboard_00405af0(slot)` /
    /// `KeyNames_getController_00405b10(slot)` (cdecl): the name at
    /// `slot`, or 0.
    pub fn get_name(mem: &impl Mem, table: u32, slot: u32) -> u32 {
        name_at(mem, table, slot).unwrap_or(0)
    }

    /// `KeyNames_setKeyboard_00405b30(slot, name)` /
    /// `KeyNames_setController_00405b60(slot, name)` (cdecl): the name at
    /// `slot` becomes `name` unless it is 0. Returns `al` (1 when set;
    /// the original leaves `name`'s upper bytes in `eax` then, and the
    /// caller's otherwise: its callers read only `al`).
    pub fn set_name(mem: &mut impl Mem, table: u32, slot: u32, name: u32) -> u8 {
        match name_at(mem, table, slot) {
            Some(n) if n != 0 => {
                let at = mem.rebase(table).wrapping_add(4 * slot);
                mem.write(at, name);
                1
            }
            _ => 0,
        }
    }

    /// `0x00401690(text; out, max)` (`ecx` = the text, `edx` = `out`, one
    /// stack word the caller pops: `fastcall-caller-pops`): the decimal
    /// list in the NUL-terminated `text` (a binding line of
    /// `GameSetting.ini`) into the words at `out`, one per `,` or the end,
    /// up to `max` (signed; at least one word is written when `text` is
    /// not null). Characters other than digits, `,` and NUL are skipped
    /// (signed byte compares: bytes 0x80.. are not digits); a number is
    /// `acc * 10 + c - '0'` in wrapping 32 bits. Returns the count (0 for
    /// a null `text`). The same loop is inlined in `Settings_SetValue` for
    /// the other two tables.
    pub fn parse_list(mem: &mut impl Mem, text: u32, out: u32, max: u32) -> u32 {
        let mut count = 0u32;
        let mut acc = 0u32;
        let mut p = text;
        if p == 0 {
            return 0;
        }
        loop {
            let c: u8 = mem.read(p);
            let s = c.cast_signed();
            if (0x30..=0x39).contains(&s) {
                acc = acc
                    .wrapping_mul(10)
                    .wrapping_add(u32::from(c))
                    .wrapping_sub(0x30);
            } else if c == b',' || c == 0 {
                mem.write(out.wrapping_add(count.wrapping_mul(4)), acc);
                count = count.wrapping_add(1);
                if count.cast_signed() >= max.cast_signed() {
                    break;
                }
                acc = 0;
            } else {
                // `inc edi; jne`: the next byte, unless the pointer wraps
                p = p.wrapping_add(1);
                if p == 0 {
                    break;
                }
                continue;
            }
            let c: u8 = mem.read(p);
            if c == 0 {
                break;
            }
            p = p.wrapping_add(1);
            if p == 0 {
                break;
            }
        }
        count
    }

    /// `Settings_ValidateBindings(list; n, filter)` (`ecx` = the binding
    /// table, `edx` = its word count, `filter` = the slot filter, a cdecl
    /// `(slot) -> al` the caller pushes and pops:
    /// `fastcall-caller-pops`). 0 when a slot the filter takes has a key
    /// code outside 0..0x80 or one without a name, or when two such slots
    /// share a nonzero code; else 1. Slots 14..=27 and 44, 45 are
    /// controller codes (`CONTROLLER_NAMES`), the rest keyboard codes; a
    /// zero slot at 0x31 or above is skipped before the filter is asked
    /// (unsigned `jae`; counts and indices compare signed). The filter is
    /// called once per slot in the first pass, and again for each pair in
    /// the second (outer slot, then each later slot with a code).
    pub fn validate_bindings(h: &mut impl Host, list: u32, n: u32, filter: u32) -> u32 {
        let n = n.cast_signed();
        let at = |i: i32| list.wrapping_add(4u32.wrapping_mul(i.cast_unsigned()));
        let mut i = 0i32;
        while i < n {
            let code: u32 = h.read(at(i));
            if (code != 0 || i < 0x31)
                && h.call(filter, Conv::Cdecl, &[i.cast_unsigned()]) & 0xff != 0
            {
                let table = if (14..=27).contains(&i) || i.wrapping_sub(0x2c).cast_unsigned() <= 1 {
                    CONTROLLER_NAMES
                } else {
                    KEYBOARD_NAMES
                };
                if code >= 0x80 {
                    // also the negative codes (`js`)
                    return 0;
                }
                let name: u32 = h.read(h.rebase(table).wrapping_add(4 * code));
                if name == 0 {
                    return 0;
                }
            }
            i += 1;
        }
        let mut i = 0i32;
        while i < n {
            let code: u32 = h.read(at(i));
            if code != 0 && h.call(filter, Conv::Cdecl, &[i.cast_unsigned()]) & 0xff != 0 {
                let mut j = i + 1;
                while j < n {
                    let other: u32 = h.read(at(j));
                    if other != 0 && h.call(filter, Conv::Cdecl, &[j.cast_unsigned()]) & 0xff != 0 {
                        let first: u32 = h.read(at(i));
                        if first == other {
                            return 0;
                        }
                    }
                    j += 1;
                }
            }
            i += 1;
        }
        1
    }

    /// `0x00402f00(this; 2 words)` (thiscall, `ret 8`): the
    /// `std::stringstream` constructor (the two stack words are never
    /// read; the caller only reserves them).
    pub const STRINGSTREAM_CTOR: u32 = 0x0040_2f00;
    /// `0x00404a90` (`ecx` = an `ostream`, one stack word the caller pops
    /// and the callee never reads): `os << ','`; returns the stream.
    /// Called as a thiscall with no stack words (its `ret` pops none).
    pub const PUT_COMMA: u32 = 0x0040_4a90;
    /// `0x00402ff0(this; out)` (thiscall, `ret 4`): `str()` into `out`.
    pub const STRINGSTREAM_STR: u32 = 0x0040_2ff0;
    /// `operator delete` (cdecl).
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    /// Import slots (MSVCP110): `ostream::operator<<(int)`,
    /// `streambuf::setg`, `~basic_streambuf`, `~basic_iostream`,
    /// `~basic_ios`.
    pub const OSTREAM_INT_IAT: u32 = 0x00b0_c2b8;
    pub const SETG_IAT: u32 = 0x00b0_c270;
    pub const STREAMBUF_DTOR_IAT: u32 = 0x00b0_c30c;
    pub const IOSTREAM_DTOR_IAT: u32 = 0x00b0_c290;
    pub const IOS_DTOR_IAT: u32 = 0x00b0_c2f8;
    /// The `stringstream`'s `basic_ios` vtable and the `stringbuf` vtable
    /// its destructor stores.
    pub const STRINGSTREAM_IOS_VTABLE: u32 = 0x00b7_cb6c;
    pub const STRINGBUF_VTABLE: u32 = 0x00b7_cb2c;
    /// `Settings_JoinBindings`' frame: the stream at `ebp - 0xcc`.
    pub const JOIN_FRAME: u32 = 0xd0;

    /// Calls the import in `slot` (its `ffx.sym` address).
    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `Settings_JoinBindings(out; list, n)` (`ecx` = the `std::string`
    /// to fill, `edx` = the words, `n` on the stack, the caller pops:
    /// `fastcall-caller-pops`): the `n` words (signed count) written to a
    /// `std::stringstream` at `ebp - 0xcc` as `a,b,c` (`operator<<(int)`,
    /// `0x00404a90` for each comma), `str()` into `out`, then the stream's
    /// destructor inlined: the `basic_ios` vtable through the vbase
    /// offset, the displacement word below it, the `stringbuf`'s buffer
    /// freed when its flag (`ebp - 0x78` bit 0) says it owns one,
    /// `setg(0, 0, 0)`, the three put/get pointers cleared, the flag and
    /// `ebp - 0x7c` cleared, `~basic_streambuf`, `~basic_iostream`,
    /// `~basic_ios`. Returns `out`. `ebp` is the original's frame
    /// ([`JOIN_FRAME`] bytes below it); the SEH registration and the
    /// cookie (`ebp - 0x10` up) are not written.
    pub fn join_bindings(h: &mut impl Host, ebp: u32, out: u32, list: u32, n: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        let ss = at(0xcc);
        h.write(at(0x14), out);
        h.write(at(0x18), 0u32);
        h.call_sym(STRINGSTREAM_CTOR, Conv::Thiscall(ss), &[0, 0]);
        h.write(at(4), 0u32);
        let os = at(0xbc);
        let mut i = 0i32;
        while i < n.cast_signed() {
            let word: u32 = h.read(list.wrapping_add(4u32.wrapping_mul(i.cast_unsigned())));
            let stream = if i == 0 {
                os
            } else {
                h.call_sym(PUT_COMMA, Conv::Thiscall(os), &[])
            };
            import(h, OSTREAM_INT_IAT, Conv::Thiscall(stream), &[word]);
            i += 1;
        }
        // `mov esi, [ebp - 0x14]`: `out` read back
        let out: u32 = h.read(at(0x14));
        h.call_sym(STRINGSTREAM_STR, Conv::Thiscall(ss), &[out]);
        h.write(at(4), u32::MAX);
        // ~stringstream, inlined
        let vbtable: u32 = h.read(ss);
        let dist: u32 = h.read(vbtable.wrapping_add(4));
        let ios_vtable = h.rebase(STRINGSTREAM_IOS_VTABLE);
        h.write(ss.wrapping_add(dist), ios_vtable);
        // read again: the store may have landed on the vbase pointer
        let vbtable: u32 = h.read(ss);
        let dist: u32 = h.read(vbtable.wrapping_add(4));
        h.write(at(0xd0).wrapping_add(dist), dist.wrapping_sub(0x68));
        let flags: u8 = h.read(at(0x78));
        let buf = at(0xb4);
        h.write(buf, h.rebase(STRINGBUF_VTABLE));
        if flags & 1 != 0 {
            let p: u32 = h.read(at(0xa8));
            let block: u32 = h.read(p);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[block]);
        }
        import(h, SETG_IAT, Conv::Thiscall(buf), &[0, 0, 0]);
        for off in [0xa4u32, 0x94, 0x84] {
            let p: u32 = h.read(at(off));
            h.write(p, 0u32);
        }
        let flags: u32 = h.read(at(0x78));
        h.write(at(0x78), flags & !1);
        h.write(at(0x7c), 0u32);
        import(h, STREAMBUF_DTOR_IAT, Conv::Thiscall(buf), &[]);
        import(h, IOSTREAM_DTOR_IAT, Conv::Thiscall(at(0xac)), &[]);
        import(h, IOS_DTOR_IAT, Conv::Thiscall(at(0x64)), &[]);
        out
    }

    /// `g_SettingsValueStrings`: 0x18 keys x 8 string pointers.
    pub const VALUE_STRINGS: u32 = 0x00c8_9f48;
    /// `g_SettingsKeyNames`: 0x18 string pointers.
    pub const KEY_NAMES: u32 = 0x00c8_a248;
    /// `"*"`, `Resolution`'s separator.
    pub const RESOLUTION_SEPARATOR: u32 = 0x00b7_c798;
    /// `"[ConfigFile] %s value invalid [%s].\n"`.
    pub const MSG_VALUE_INVALID: u32 = 0x00b7_ca5c;
    /// `"[ConfigFile] invalid Resolution at [%s].\n"`.
    pub const MSG_RESOLUTION_INVALID: u32 = 0x00b7_ca30;
    /// `std::string::find(this; s, pos, n)` (thiscall, `ret 0xc`).
    pub const STRING_FIND: u32 = 0x0040_3bd0;
    /// `std::string::substr(this; out, pos, n)` (thiscall, `ret 0xc`).
    pub const STRING_SUBSTR: u32 = 0x0040_2e20;
    /// MSVCR110 import slots: `_stricmp`, `printf`, `atoi`.
    pub const STRICMP_IAT: u32 = 0x00b0_c398;
    pub const PRINTF_IAT: u32 = 0x00b0_c380;
    pub const ATOI_IAT: u32 = 0x00b0_c40c;
    /// The three binding tables' slot filters.
    pub const FFX_FILTER: u32 = 0x0040_5cb0;
    pub const FFX2_FILTER: u32 = 0x0040_5d00;
    pub const LM_FILTER: u32 = 0x0040_5d80;
    /// `Settings_SetValue`'s frame: locals from `ebp - 0x50`.
    pub const SET_VALUE_FRAME: u32 = 0x50;

    /// A by-value `std::string`'s text: the buffer in place below 16
    /// bytes of capacity (`+0x14`), else the heap pointer at `+0`.
    fn c_str(h: &impl Host, s: u32) -> u32 {
        let cap: u32 = h.read(s.wrapping_add(0x14));
        if cap >= 0x10 { h.read(s) } else { s }
    }

    /// `Settings_SetValue(this; key, value)` (thiscall, `ret 0x1c`; `value`
    /// a `std::string` by value, six words at `value`, destroyed here):
    /// key's row of [`VALUE_STRINGS`] counted up to its first null (at
    /// most 8). A row of exactly one string is a special key: 1
    /// `Resolution` (`W*H`: `find("*")`, two `substr`s, `this[1] = atoi(W)
    /// << 16 | atoi(H) & 0xffff`; no `*`: the message, 1), 0x15 / 0x16 the
    /// FFX / FFX-2 binding lists (the inline [`parse_list`] loop into
    /// `+0x60` / `+0x1a4`, the count at `+0x42c`, the
    /// [`validate_bindings`] answer at byte `+0x638` / `+0x639`), 0x17
    /// Last Mission's (`0x00401690` into `+0x2e8`, `+0x63a`), any other
    /// `this[key] = atoi(value)`. Else the row's index matching `value`
    /// (`_stricmp`) goes to `this[key]`; none matches (an empty row too):
    /// the message ([`KEY_NAMES`]), 1. Returns 0 or 1. `ebp` is the
    /// original's frame ([`SET_VALUE_FRAME`] bytes below it; the cookie,
    /// SEH registration and state at `ebp - 0x14` up are not written but
    /// the state's byte 1 for the substrings).
    pub fn set_value(h: &mut impl Host, ebp: u32, this: u32, key: u32, value: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        h.write(at(0x48), this);
        h.write(at(0x50), key);
        let row = h.rebase(VALUE_STRINGS).wrapping_add(key.wrapping_shl(5));
        h.write(at(4), 0u32);
        let mut count = 8u32;
        h.write(at(0x4c), count);
        let mut i = 0u32;
        while i < 8 {
            let p: u32 = h.read(row.wrapping_add(4 * i));
            if p == 0 {
                count = i;
                h.write(at(0x4c), count);
                break;
            }
            i += 1;
        }
        let result = if count == 1 {
            set_special(h, ebp, this, key, value)
        } else {
            let mut found = 0u32;
            while found.cast_signed() < count.cast_signed() {
                let s = c_str(h, value);
                let p: u32 = h.read(row.wrapping_add(4 * found));
                if import(h, STRICMP_IAT, Conv::Cdecl, &[s, p]) == 0 {
                    break;
                }
                found += 1;
            }
            if found == count {
                let s = c_str(h, value);
                let name: u32 = h.read(h.rebase(KEY_NAMES).wrapping_add(key.wrapping_mul(4)));
                let fmt = h.rebase(MSG_VALUE_INVALID);
                import(h, PRINTF_IAT, Conv::Cdecl, &[fmt, name, s]);
                1
            } else {
                h.write(this.wrapping_add(key.wrapping_mul(4)), found);
                0
            }
        };
        let cap: u32 = h.read(value.wrapping_add(0x14));
        if cap >= 0x10 {
            let p: u32 = h.read(value);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
        result
    }

    /// [`set_value`]'s keys with a one-string row.
    fn set_special(h: &mut impl Host, ebp: u32, this: u32, key: u32, value: u32) -> u32 {
        let at = |off: u32| ebp.wrapping_sub(off);
        match key {
            1 => {
                let sep = h.rebase(RESOLUTION_SEPARATOR);
                let pos = h.call_sym(STRING_FIND, Conv::Thiscall(value), &[sep, 0, 1]);
                if pos == u32::MAX {
                    let s = c_str(h, value);
                    let fmt = h.rebase(MSG_RESOLUTION_INVALID);
                    import(h, PRINTF_IAT, Conv::Cdecl, &[fmt, s]);
                    return 1;
                }
                let (w, hh) = (at(0x44), at(0x2c));
                h.call_sym(STRING_SUBSTR, Conv::Thiscall(value), &[w, 0, pos]);
                h.write(at(4), 1u8);
                h.call_sym(
                    STRING_SUBSTR,
                    Conv::Thiscall(value),
                    &[hh, pos.wrapping_add(1), u32::MAX],
                );
                let s = c_str(h, w);
                let width = import(h, ATOI_IAT, Conv::Cdecl, &[s]) & 0xffff;
                let s = c_str(h, hh);
                let height = import(h, ATOI_IAT, Conv::Cdecl, &[s]) & 0xffff;
                h.write(this.wrapping_add(4), width << 16 | height);
                // ~string of both, the second one reset as well
                let cap: u32 = h.read(hh.wrapping_add(0x14));
                if cap >= 0x10 {
                    let p: u32 = h.read(hh);
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
                }
                h.write(hh.wrapping_add(0x14), 0xfu32);
                h.write(hh.wrapping_add(0x10), 0u32);
                h.write(hh, 0u8);
                let cap: u32 = h.read(w.wrapping_add(0x14));
                if cap >= 0x10 {
                    let p: u32 = h.read(w);
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
                }
                0
            }
            0x15..=0x17 => {
                let (table, filter, flag) = match key {
                    0x15 => (0x60u32, FFX_FILTER, 0x638u32),
                    0x16 => (0x1a4, FFX2_FILTER, 0x639),
                    _ => (0x2e8, LM_FILTER, 0x63a),
                };
                let s = c_str(h, value);
                let list = this.wrapping_add(table);
                let n = parse_list(h, s, list, 0x51);
                h.write(this.wrapping_add(0x42c), n);
                let filter = h.rebase(filter);
                let ok = validate_bindings(h, list, n, filter) != 0;
                h.write(this.wrapping_add(flag), u8::from(ok));
                0
            }
            _ => {
                let s = c_str(h, value);
                let v = import(h, ATOI_IAT, Conv::Cdecl, &[s]);
                h.write(this.wrapping_add(key.wrapping_mul(4)), v);
                0
            }
        }
    }

    /// `Settings_Format(this; buf)` (thiscall, `ret 4`): the ini text
    /// into `buf` (none for 0), returns its length.
    pub const FORMAT: u32 = 0x0040_22f0;
    /// `Settings_Parse(this; text)` (thiscall, `ret 4`).
    pub const PARSE: u32 = 0x0040_1e00;
    /// `Settings_CopyDefaultTables(this)` (thiscall).
    pub const COPY_DEFAULT_TABLES: u32 = 0x0040_2b00;
    /// `Settings_Save(this)` (thiscall, no stack words).
    pub const SAVE: u32 = 0x0040_1d80;
    /// MSVCR110 import slots: `_wfopen`, `fwrite`, `fclose`, `fseek`,
    /// `ftell`, `fread`.
    pub const WFOPEN_IAT: u32 = 0x00b0_c47c;
    pub const FWRITE_IAT: u32 = 0x00b0_c428;
    pub const FCLOSE_IAT: u32 = 0x00b0_c3ec;
    pub const FSEEK_IAT: u32 = 0x00b0_c3fc;
    pub const FTELL_IAT: u32 = 0x00b0_c4b4;
    pub const FREAD_IAT: u32 = 0x00b0_c3f4;
    /// `L"wb"` and `L"rb"`.
    pub const MODE_WB: u32 = 0x00b7_ca8c;
    pub const MODE_RB: u32 = 0x00b7_ca84;

    /// `Settings_Save(this)` (thiscall): `_wfopen(this + 0x430, L"wb")`
    /// (not checked for null), the length from `Settings_Format(this; 0)`,
    /// a buffer of length + 1 bytes (the original's `alloca`: `buffer`
    /// gets the size and returns the address), `Settings_Format` into it,
    /// `fwrite(buf, len, 1, f)`, `fclose(f)`. Returns `fclose`'s `eax`.
    pub fn save(h: &mut impl Host, this: u32, buffer: impl FnOnce(u32) -> u32) -> u32 {
        let mode = h.rebase(MODE_WB);
        let f = import(
            h,
            WFOPEN_IAT,
            Conv::Cdecl,
            &[this.wrapping_add(0x430), mode],
        );
        let len = h.call_sym(FORMAT, Conv::Thiscall(this), &[0]);
        let buf = buffer(len.wrapping_add(1));
        h.call_sym(FORMAT, Conv::Thiscall(this), &[buf]);
        import(h, FWRITE_IAT, Conv::Cdecl, &[buf, len, 1, f]);
        import(h, FCLOSE_IAT, Conv::Cdecl, &[f])
    }

    /// `Settings_Load(this)` (thiscall): `_wfopen(this + 0x430, L"rb")`;
    /// no file: `Settings_CopyDefaultTables` and `Settings_Save`, returning
    /// the latter's `eax`. Else the flags `+0x638` (word) and `+0x63a`
    /// cleared, the length by `fseek(f, 0, SEEK_END)` / `ftell` /
    /// `fseek(f, 0, SEEK_SET)`, a buffer of length + 0x20 bytes
    /// (`buffer`, the original's `alloca`), `fread(buf, len, 1, f)`,
    /// `fclose`, `Settings_Parse(this; buf)`, returning its `eax`.
    ///
    /// The original's buffer is uninitialised stack past the file's
    /// bytes (no terminator is written; the 0x20 spare bytes are what
    /// `Settings_Parse` finds there): junk read, the shim's buffer is
    /// zeroed (brief 2026-10-24).
    pub fn load(h: &mut impl Host, this: u32, buffer: impl FnOnce(u32) -> u32) -> u32 {
        let mode = h.rebase(MODE_RB);
        let f = import(
            h,
            WFOPEN_IAT,
            Conv::Cdecl,
            &[this.wrapping_add(0x430), mode],
        );
        if f == 0 {
            h.call_sym(COPY_DEFAULT_TABLES, Conv::Thiscall(this), &[]);
            return h.call_sym(SAVE, Conv::Thiscall(this), &[]);
        }
        h.write(this.wrapping_add(0x638), 0u16);
        h.write(this.wrapping_add(0x63a), 0u8);
        import(h, FSEEK_IAT, Conv::Cdecl, &[f, 0, 2]);
        let len = import(h, FTELL_IAT, Conv::Cdecl, &[f]);
        import(h, FSEEK_IAT, Conv::Cdecl, &[f, 0, 0]);
        let buf = buffer(len.wrapping_add(0x20));
        import(h, FREAD_IAT, Conv::Cdecl, &[buf, len, 1, f]);
        import(h, FCLOSE_IAT, Conv::Cdecl, &[f]);
        h.call_sym(PARSE, Conv::Thiscall(this), &[buf])
    }

    /// The keyboard table's address constant (for the shims).
    pub const KEYBOARD: u32 = KEYBOARD_NAMES;
    /// The controller table's address constant (for the shims).
    pub const CONTROLLER: u32 = CONTROLLER_NAMES;

    /// `Settings_Construct(this; path)` (thiscall, `ret 4`): the flags
    /// (`+0x638` = 0x101, `+0x63a` = 1), the 24 defaults from the preset
    /// table, the three binding tables cleared (`memset`, 0x144 bytes
    /// each) and the UTF-16 `path` copied to `+0x430` when given.
    /// Returns `this`.
    pub fn construct(h: &mut impl Host, this: u32, path: u32) -> u32 {
        h.write(this.wrapping_add(0x638), 0x101u16);
        h.write(this.wrapping_add(0x63a), 1u8);
        let presets = h.rebase(PRESETS);
        for i in 0..24 {
            let v: u32 = h.read(presets.wrapping_add(16 * i));
            h.write(this.wrapping_add(4 * i), v);
        }
        for table in [0x60u32, 0x1a4, 0x2e8] {
            h.call_sym(MEMSET, Conv::Cdecl, &[this.wrapping_add(table), 0, 0x144]);
        }
        if path != 0 {
            let dst = this.wrapping_add(0x430);
            let mut i = 0u32;
            loop {
                let c: u16 = h.read(path.wrapping_add(i));
                h.write(dst.wrapping_add(i), c);
                i = i.wrapping_add(2);
                if c == 0 {
                    break;
                }
            }
        }
        this
    }

    /// `Settings_DetectQuality(this)` (thiscall): `+0x0c` = the first
    /// preset 0..2 whose fixed values (`PRESET_MASK`) all equal the
    /// object's, else 3 (custom). Returns `this`.
    pub fn detect_quality(mem: &mut impl Mem, this: u32) -> u32 {
        let presets = mem.rebase(PRESETS);
        let mask = mem.rebase(PRESET_MASK);
        let mut level = 0u32;
        while level < 3 {
            let matches = (0..24u32).all(|k| {
                mem.read::<u8>(mask.wrapping_add(k)) == 0
                    || mem.read::<u32>(this.wrapping_add(4 * k))
                        == mem.read::<u32>(presets.wrapping_add(16 * k + 4 * level))
            });
            if matches {
                break;
            }
            level += 1;
        }
        mem.write(this.wrapping_add(0xc), level);
        this
    }
}
