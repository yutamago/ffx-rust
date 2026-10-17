//! Battle-window state of the `TO` layer (`TOBtl*`): help and sign windows,
//! the cursor position stack, Lulu's limit table, target callbacks.

/// One entry of `TOBtlSetLuluLimitTbl`'s running sums of ASCII digits:
/// the previous entry (0 for the first) plus the digit's value (bytes,
/// wrapping).
#[must_use]
pub const fn lulu_limit_step(prev: u8, digit: u8) -> u8 {
    prev.wrapping_sub(0x30).wrapping_add(digit)
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The battle help window is open (`TOBtlCloseHelpWin` clears it).
    pub const HELP_WIN: Global<u8> = Global::new(0x0133_c93c);
    /// Set by `TOBtlInitAnm`.
    pub const INIT_ANM: Global<u8> = Global::new(0x0134_0452);
    /// The sign window (`TOBtlCloseSignWindow` clears it).
    pub const SIGN_WINDOW: Global<u32> = Global::new(0x0133_f520);
    /// The simple help message (`TOBtlCloseSimpleHelpMes` clears it).
    pub const SIMPLE_HELP: Global<u32> = Global::new(0x0133_f510);
    /// `TOGetBtlWindowControlEnable`'s flag.
    pub const WINDOW_CONTROL: Global<u32> = Global::new(0x00c5_8ef8);
    /// Cursor positions pushed (`i16`, at most 4).
    pub const CUR_POS_COUNT: Global<i16> = Global::new(0x0133_d6f0);
    /// The cursor positions: 8-byte records, x at `+0`, y at `+4`, z at `+6`.
    pub const CUR_POS: Global<[u16; 4]> = Global::new(0x0133_f7b4);
    /// Lulu's limit table (16 bytes).
    pub const LULU_LIMIT: Global<[u8; 16]> = Global::new(0x0133_f798);

    /// `TOBtlCloseHelpWin()` (`0x00890eb0`).
    pub fn close_help_win(mem: &mut impl Mem) {
        HELP_WIN.set(mem, 0);
    }

    /// `TOBtlInitAnm()` (`0x008a0cd0`).
    pub fn init_anm(mem: &mut impl Mem) {
        INIT_ANM.set(mem, 1);
    }

    /// `TOBtlCloseSignWindow()` (`0x00890ee0`).
    pub fn close_sign_window(mem: &mut impl Mem) {
        SIGN_WINDOW.set(mem, 0);
    }

    /// `TOBtlCloseSimpleHelpMes()` (`0x00890ef0`).
    pub fn close_simple_help(mem: &mut impl Mem) {
        SIMPLE_HELP.set(mem, 0);
    }

    /// `TOGetBtlWindowControlEnable()` (`0x0089b590`).
    pub fn window_control_enable(mem: &impl Mem) -> u32 {
        WINDOW_CONTROL.get(mem)
    }

    /// `TOBtlSetCurPos(x, y, z)` (`0x008999f0`): pushes a cursor position
    /// while fewer than 4 are pushed (the count is signed).
    pub fn set_cur_pos(mem: &mut impl Mem, x: u16, y: u16, z: u16) {
        let count = CUR_POS_COUNT.get(mem);
        if count >= 4 {
            return;
        }
        let r = CUR_POS
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(count).cast_unsigned().wrapping_mul(8));
        mem.write(r, x);
        mem.write(r.wrapping_add(4), y);
        mem.write(r.wrapping_add(6), z);
        CUR_POS_COUNT.set(mem, count.wrapping_add(1));
    }

    /// `TOBtlSetLuluLimitTbl(digits)` (`0x0089a2f0`): 16 entries, each
    /// read back from the table and the digits as the original goes.
    pub fn set_lulu_limit_table(mem: &mut impl Mem, digits: u32) {
        let t = LULU_LIMIT.ptr(mem).addr;
        for i in 0..16 {
            let prev = if i == 0 { 0 } else { mem.read::<u8>(t + i - 1) };
            let d = mem.read::<u8>(digits.wrapping_add(i));
            mem.write(t + i, super::lulu_limit_step(prev, d));
        }
    }

    /// `TOBtlSetTargetCallback(w)` (`0x008a1090`): the window's target
    /// handlers (`+0x90`, `+0x98`, `+0x9c`, `+0xa0`; `+0x94` cleared), code
    /// addresses of this image.
    pub fn set_target_callback(mem: &mut impl Mem, w: u32) {
        let at = |o: u32| w.wrapping_add(o);
        mem.write(at(0x94), 0u32);
        for (o, code) in [
            (0x98, 0x008a_8780),
            (0x9c, 0x008a_8be0),
            (0xa0, 0x008a_2ac0),
            (0x90, 0x008a_84c0),
        ] {
            let c = mem.rebase(code);
            mem.write(at(o), c);
        }
    }

    /// What `TOCloseBtlWindow` clears: a word, then nine words.
    pub const BTL_WINDOW_OPEN: Global<u32> = Global::new(0x023c_c08c);
    /// The nine words after it.
    pub const BTL_WINDOW_STATE: Global<[u32; 9]> = Global::new(0x0133_d74c);

    /// `TOCloseBtlWindow()` (`0x0089ade0`): returns 0 (left in `eax`).
    pub fn close_btl_window(mem: &mut impl Mem) -> u32 {
        BTL_WINDOW_OPEN.set(mem, 0);
        BTL_WINDOW_STATE.set(mem, [0; 9]);
        0
    }

    /// The battle command-window stack (`0x0133c900`): 0xf0 bytes per
    /// level, indexed by [`ACTION_DEPTH`].
    pub const ACTION_STACK: Global<[u8; 0xf0]> = Global::new(0x0133_c900);
    /// The current level (`0x023cc092`, i8).
    pub const ACTION_DEPTH: Global<i8> = Global::new(0x023c_c092);
    /// Stored at `+0x58` of the level left (`0x023cc088`, u16).
    pub const ACTION_PREV_VALUE: Global<u16> = Global::new(0x023c_c088);
    /// Cleared when the level left shows command 0x311e or 0x3029
    /// (`0x0133c94c`).
    pub const ACTION_MASK: Global<u32> = Global::new(0x0133_c94c);
    /// Restored from [`ACTION_SAVED`] when going back to a level whose
    /// `+0x52` is 1 (`0x0133d0d0`, 0x140 bytes).
    pub const ACTION_LIST: Global<[u8; 0x140]> = Global::new(0x0133_d0d0);
    /// See [`ACTION_LIST`] (`0x0133d210`).
    pub const ACTION_SAVED: Global<[u8; 0x140]> = Global::new(0x0133_d210);
    /// Zeroed with that restore (`0x0133f106`, u16).
    pub const ACTION_LIST_CURSOR: Global<u16> = Global::new(0x0133_f106);

    /// `TOBtlSetPrevAction()` (`0x0089a630`): back one level of the command
    /// windows. The level left: `+0x51` 5, `+0x58` [`ACTION_PREV_VALUE`],
    /// `+0x66` 0xff, [`ACTION_MASK`] cleared for commands 0x311e and 0x3029
    /// (`+0x5c`), and when its `+0x52` is 1 the next level's `+6` takes its
    /// `+6`. The level returned to: `+0x51` 2, its cursor `+0x5b` back one
    /// when at or past `+0x5a` and positive, and when its `+0x52` is 1 the
    /// list restored. 0 at level 0 (or below), else 1.
    pub fn set_prev_action(mem: &mut impl Mem) -> u32 {
        let depth = ACTION_DEPTH.get(mem);
        if depth <= 0 {
            return 0;
        }
        let base = ACTION_STACK.ptr(mem).addr;
        let level = |d: i8| base.wrapping_add(i32::from(d).cast_unsigned().wrapping_mul(0xf0));
        let e = level(depth);
        mem.write(e + 0x51, 5u8);
        let prev = ACTION_PREV_VALUE.get(mem);
        mem.write(e + 0x58, prev);
        mem.write(e + 0x66, 0xffu16);
        let command: u16 = mem.read(e + 0x5c);
        if command == 0x311e || command == 0x3029 {
            ACTION_MASK.set(mem, 0);
        }
        if mem.read::<u8>(e + 0x52) == 1 {
            let v: u16 = mem.read(e + 6);
            mem.write(e + 0xf6, v);
        }
        let depth = depth - 1;
        ACTION_DEPTH.set(mem, depth);
        let p = level(depth);
        mem.write(p + 0x51, 2u8);
        let cursor: i8 = mem.read(p + 0x5b);
        if cursor >= mem.read::<i8>(p + 0x5a) && cursor > 0 {
            mem.write(p + 0x5b, cursor - 1);
        }
        if mem.read::<u8>(p + 0x52) == 1 {
            let saved: [u8; 0x140] = mem.read(ACTION_SAVED.ptr(mem).addr);
            let list = ACTION_LIST.ptr(mem).addr;
            mem.write(list, saved);
            ACTION_LIST_CURSOR.set(mem, 0);
        }
        1
    }

    /// The damage values shown (`0x0133f108`): an `i16` count, then from
    /// `0x0133f110` up to 63 records of 16 bytes.
    pub const DAMAGE_COUNT: Global<i16> = Global::new(0x0133_f108);
    /// The first damage record.
    pub const DAMAGE_VALUES: Global<[u8; 16]> = Global::new(0x0133_f110);

    /// `TOBtlSetDamageValue(a, b, c, d, e, f, g, h)` (`0x00899a40`): a
    /// record (`u16`, `u16`, `u32`, `u32`, four bytes) appended; the new
    /// count, or -1 with 63 already.
    pub fn set_damage_value(mem: &mut impl Mem, a: [u32; 8]) -> i32 {
        add_damage_record(mem, a, true)
    }

    /// `0x0089baa0(a, b, d, e, f, g, h)`: [`set_damage_value`] without the
    /// word at `+4` (left as it was).
    pub fn set_damage_value_short(mem: &mut impl Mem, a: [u32; 7]) -> i32 {
        add_damage_record(mem, [a[0], a[1], 0, a[2], a[3], a[4], a[5], a[6]], false)
    }

    fn add_damage_record(mem: &mut impl Mem, a: [u32; 8], word4: bool) -> i32 {
        let n = DAMAGE_COUNT.get(mem);
        if n >= 0x3f {
            return -1;
        }
        let r = DAMAGE_VALUES
            .ptr(mem)
            .addr
            .wrapping_add(i32::from(n).cast_unsigned() << 4);
        #[allow(clippy::cast_possible_truncation)] // the low halves
        {
            mem.write(r, a[0] as u16);
            mem.write(r.wrapping_add(2), a[1] as u16);
        }
        if word4 {
            mem.write(r.wrapping_add(4), a[2]);
        }
        mem.write(r.wrapping_add(8), a[3]);
        for (k, w) in (0u32..).zip(&a[4..]) {
            mem.write(r.wrapping_add(0xc + k), w.to_le_bytes()[0]);
        }
        let n = n.wrapping_add(1);
        DAMAGE_COUNT.set(mem, n);
        i32::from(n)
    }

    /// The two target lists (`0x0133f0d8`, `0x0133f0ec`): a count, then
    /// two-byte entries whose first byte is a target bit.
    pub const TARGET_LISTS: [u32; 2] = [0x0133_f0d8, 0x0133_f0ec];
    /// The masks they are OR-ed into (`0x0133d0d4`, `0x0133d0d8`) and the
    /// union (`0x0133d0dc`).
    pub const TARGET_MASKS: Global<[u32; 3]> = Global::new(0x0133_d0d4);

    /// `TOBtlMakeTargetListBit()` (`0x008985b0`): each list's target bits
    /// OR-ed into its mask (stored only for a non-empty list), the union of
    /// both into the third; returns the sum of the counts.
    pub fn make_target_list_bit(mem: &mut impl Mem) -> i32 {
        let masks = TARGET_MASKS.ptr(mem).addr;
        let mut out = [0u32; 2];
        let mut total = 0i32;
        for (k, &list) in TARGET_LISTS.iter().enumerate() {
            let list = mem.rebase(list);
            let n: i32 = mem.read(list);
            let at = masks.wrapping_add(4 * u32::try_from(k).unwrap_or(0));
            let mut m: u32 = mem.read(at);
            for i in 0..n.max(0).cast_unsigned() {
                let bit: u8 = mem.read(list.wrapping_add(4 + 2 * i));
                m |= 1u32.wrapping_shl(u32::from(bit));
            }
            if n > 0 {
                mem.write(at, m);
            }
            out[k] = m;
            total = total.wrapping_add(n);
        }
        mem.write(masks.wrapping_add(8), out[0] | out[1]);
        total
    }
}
