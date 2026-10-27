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
