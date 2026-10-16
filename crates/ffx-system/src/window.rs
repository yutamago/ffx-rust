//! The `TO` window and message-window tables and the message macro
//! commands (`TOGetWindowStatus`, `TOCloseMessage`, `TOInitMacroCmd`, ...).
//!
//! A window is a pointer in [`bridge::WINDOWS`]: `i16` in use at `+0`, type
//! at `+4`, state at `+8`, one handler per state from `+0x60`, the current
//! handler at `+0x74`. A message window ([`bridge::MSG_WINDOWS`]) has the
//! same shape further out: in use at `+0`, state at `+0x2c`, `+0x2e` the
//! status `TOGetMessageStatus` reports, handlers from `+0x100`, the current
//! one at `+0x114`. Closing sets state 3 and takes its handler.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// The windows: one pointer each.
    pub const WINDOWS: Global<u32> = Global::new(0x0187_6da8);
    /// The message windows: one pointer each.
    pub const MSG_WINDOWS: Global<u32> = Global::new(0x0186_76f0);
    /// `g_MsgVars`: per message window, 32 pointers to macro command
    /// records (type `u8` at `+0` and `+1`, value at `+4`).
    pub const MSG_VARS: Global<u32> = Global::new(0x0186_9ab0);
    /// `g_MacroDic`: the macro dictionary sections (one pointer each).
    pub const MACRO_DIC: Global<u32> = Global::new(0x0186_9eb0);

    /// Entry `i` of a pointer table (not checked).
    fn entry(mem: &impl Mem, table: Global<u32>, i: i32) -> u32 {
        mem.read(
            table
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `TOGetWindowStructAdrs(i)` (`0x00906fb0`).
    pub fn window(mem: &impl Mem, i: i32) -> u32 {
        entry(mem, WINDOWS, i)
    }

    /// `TOGetWindowType(i)` (`0x00906c40`).
    pub fn window_type(mem: &impl Mem, i: i32) -> i32 {
        i32::from(mem.read::<i16>(window(mem, i).wrapping_add(4)))
    }

    /// `TOGetWindowStatus(i)` (`0x00906c20`).
    pub fn window_status(mem: &impl Mem, i: i32) -> i32 {
        i32::from(mem.read::<i16>(window(mem, i).wrapping_add(8)))
    }

    /// Sets state 3 and its handler, when window `i` of `table` is in use
    /// (the pointer read again after the state is written, as the
    /// original does).
    fn close(
        mem: &mut impl Mem,
        table: Global<u32>,
        i: i32,
        state: u32,
        handlers: u32,
        current: u32,
    ) {
        let w = entry(mem, table, i);
        if mem.read::<i16>(w) == 0 {
            return;
        }
        mem.write(w.wrapping_add(state), 3i16);
        let w = entry(mem, table, i);
        let s = i32::from(mem.read::<i16>(w.wrapping_add(state)));
        let h: u32 = mem.read(
            w.wrapping_add(handlers)
                .wrapping_add(s.cast_unsigned().wrapping_mul(4)),
        );
        mem.write(w.wrapping_add(current), h);
    }

    /// `TOCloseWindow(i)` (`0x00906ba0`).
    pub fn close_window(mem: &mut impl Mem, i: i32) {
        close(mem, WINDOWS, i, 8, 0x60, 0x74);
    }

    /// `TOGetMessageStatus(i)` (`0x008b7ca0`).
    pub fn message_status(mem: &impl Mem, i: i32) -> i32 {
        i32::from(mem.read::<i16>(entry(mem, MSG_WINDOWS, i).wrapping_add(0x2e)))
    }

    /// `TOCloseMessage(i)` (`0x008b5810`).
    pub fn close_message(mem: &mut impl Mem, i: i32) {
        close(mem, MSG_WINDOWS, i, 0x2c, 0x100, 0x114);
    }

    /// `TOSetMessageFunc(i, n)` (`0x008b8af0`): the current handler becomes
    /// handler `n`, when the window is in use and that handler is set.
    pub fn set_message_func(mem: &mut impl Mem, i: i32, n: i32) {
        let w = entry(mem, MSG_WINDOWS, i);
        if mem.read::<i16>(w) == 0 {
            return;
        }
        let h: u32 = mem.read(
            w.wrapping_add(0x100)
                .wrapping_add(n.cast_unsigned().wrapping_mul(4)),
        );
        if h != 0 {
            mem.write(w.wrapping_add(0x114), h);
        }
    }

    /// Macro command record `n` of message window `i`.
    fn macro_cmd(mem: &impl Mem, i: i32, n: i32) -> u32 {
        entry(mem, MSG_VARS, (i << 5).wrapping_add(n))
    }

    /// `TOBtlSetMacroCommandType(i, n, type)` (`0x008b57c0`): both type
    /// bytes.
    pub fn set_macro_command_type(mem: &mut impl Mem, i: i32, n: i32, ty: u8) {
        let r = macro_cmd(mem, i, n);
        mem.write(r, ty);
        let r = macro_cmd(mem, i, n);
        mem.write(r.wrapping_add(1), ty);
    }

    /// `TOBtlSetMacroCommandValue(i, n, value)` (`0x008b57f0`).
    pub fn set_macro_command_value(mem: &mut impl Mem, i: i32, n: i32, value: u32) {
        let r = macro_cmd(mem, i, n);
        mem.write(r.wrapping_add(4), value);
    }

    /// `TOInitMacroCmd(i)` (`0x008b81d0`): all 32 records of window `i`
    /// unset (types 0xff, value -1).
    pub fn init_macro_cmd(mem: &mut impl Mem, i: i32) {
        for n in 0..32 {
            // the record pointer is read again before each store
            let r = macro_cmd(mem, i, n);
            mem.write(r, 0xffu8);
            let r = macro_cmd(mem, i, n);
            mem.write(r.wrapping_add(1), 0xffu8);
            let r = macro_cmd(mem, i, n);
            mem.write(r.wrapping_add(4), u32::MAX);
        }
    }

    /// `MacroDic_Section(n, table)` (`0x008b70a0`): entry `n - 19` of
    /// macro dictionary `table` (through `get_ptr32_func`, the identity).
    pub fn macro_dic_section(mem: &impl Mem, n: i32, table: i32) -> u32 {
        let d = crate::stub::ptr32(entry(mem, MACRO_DIC, table));
        mem.read(
            d.wrapping_sub(0x4c)
                .wrapping_add(n.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `TOGetSaveDicAdrs(table)` (`0x008b7f60`): `+0x2c` of the dictionary.
    pub fn save_dic(mem: &impl Mem, table: i32) -> u32 {
        let d = crate::stub::ptr32(entry(mem, MACRO_DIC, table));
        mem.read(d.wrapping_add(0x2c))
    }

    /// `TOCloseMessageWindow(i)` (`0x008ab8f0`): [`close_window`], then
    /// [`close_message`] (a tail jump).
    pub fn close_message_window(mem: &mut impl Mem, i: i32) {
        close_window(mem, i);
        close_message(mem, i);
    }

    /// The eight message-window objects `TOSetMessageWindowActive` and
    /// `TOSetMessageWindowValue` index (pointers).
    pub const MSG_WINDOW_SLOTS: Global<[u32; 8]> = Global::new(0x0186_5b3c);

    /// `TOSetMessageWindowActive(i)` (`0x008af7d0`): the `u16` at `+6` of
    /// every slot's object to 0 (in order), then window `i`'s to 1 (`i`
    /// unchecked). Returns that object (left in `eax`).
    pub fn set_message_window_active(mem: &mut impl Mem, i: i32) -> u32 {
        let base = MSG_WINDOW_SLOTS.ptr(mem).addr;
        for k in 0..8 {
            let w: u32 = mem.read(base + 4 * k);
            mem.write(w.wrapping_add(6), 0u16);
        }
        let w: u32 = mem.read(base.wrapping_add(i.cast_unsigned().wrapping_mul(4)));
        mem.write(w.wrapping_add(6), 1u16);
        w
    }

    /// `TOSetMessageWindowValue(i, v)` (`0x008afb90`): the `u16` at `+4` of
    /// window `i`'s object (`i` unchecked). Returns the `eax` it leaves:
    /// `v`'s low half in `ax` over `i`'s high half.
    pub fn set_message_window_value(mem: &mut impl Mem, i: i32, v: u32) -> u32 {
        let base = MSG_WINDOW_SLOTS.ptr(mem).addr;
        let w: u32 = mem.read(base.wrapping_add(i.cast_unsigned().wrapping_mul(4)));
        mem.write(w.wrapping_add(4), (v & 0xffff) as u16);
        i.cast_unsigned() & 0xffff_0000 | v & 0xffff
    }

    /// `g_MessageWindowMode` (`TOGetMessageWindowMode`,
    /// `TOSetMessageWindowMode`).
    pub const MESSAGE_WINDOW_MODE: Global<u32> = Global::new(0x0186_59bc);
    /// The pad byte `TOIsMessageStatus3Pressed` tests (bit 0x20).
    pub const PAD_BYTE_25D09D4: Global<u8> = Global::new(0x025d_09d4);
    /// The 16 `u16`s at a stride of 16 bytes `TOMenuSet_018762ee` writes.
    pub const MENU_018762EE: Global<u16> = Global::new(0x0187_62ee);

    /// `TOSetMessageWindowMode(v)` (`0x008af840`): returns `v`.
    pub fn set_message_window_mode(mem: &mut impl Mem, v: u32) -> u32 {
        MESSAGE_WINDOW_MODE.set(mem, v);
        v
    }

    /// `TOIsMessageStatus3Pressed(i)` (`0x008b8530`): 1 when message
    /// window `i`'s status is 3 and bit 0x20 of the pad byte is set.
    pub fn is_message_status3_pressed(mem: &impl Mem, i: i32) -> u32 {
        u32::from(message_status(mem, i) == 3 && PAD_BYTE_25D09D4.get(mem) & 0x20 != 0)
    }

    /// `TOMenuSet_018762ee(i, v)` (`0x00900f60`): for `i < 16` (signed),
    /// the `u16` at `0x018762ee + 16 i`.
    pub fn menu_set_018762ee(mem: &mut impl Mem, i: i32, v: u32) {
        if i < 16 {
            let at = MENU_018762EE
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(16));
            mem.write(at, (v & 0xffff) as u16);
        }
    }

    /// The `TO` texture and window tables `0x008b7c40` reads: pointers,
    /// `+0xc` of each returned.
    pub const TABLE_01868A90: Global<u32> = Global::new(0x0186_8a90);

    /// `TOMacroSetType(win, n, t)` (`0x008b8960`): byte 0 of macro record
    /// `n` of message window `win` (the battle form, `0x008b57c0`, sets
    /// byte 1 too).
    pub fn macro_set_type(mem: &mut impl Mem, win: i32, n: i32, t: u8) {
        let r = macro_record(mem, win, n);
        mem.write(r, t);
    }

    /// `TOMacroSetValue(win, n, v)` (`0x008b8980`): word 1 of the record;
    /// returns `v`.
    pub fn macro_set_value(mem: &mut impl Mem, win: i32, n: i32, v: u32) -> u32 {
        let r = macro_record(mem, win, n);
        mem.write(r.wrapping_add(4), v);
        v
    }

    fn macro_record(mem: &impl Mem, win: i32, n: i32) -> u32 {
        let i = win
            .cast_unsigned()
            .wrapping_shl(5)
            .wrapping_add(n.cast_unsigned());
        mem.read(MSG_VARS.ptr(mem).addr.wrapping_add(i.wrapping_mul(4)))
    }

    /// `TOGetTable_01868a90(i)` (`0x008b7c40`): `+0xc` of entry `i`.
    pub fn table_01868a90(mem: &impl Mem, i: i32) -> u32 {
        let e: u32 = mem.read(
            TABLE_01868A90
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        );
        mem.read(e.wrapping_add(0xc))
    }

    /// `TOSetWindowRGBA(i, r, g, b, a)` (`0x00906d90`): window `i`'s colour
    /// bytes at `+0x58`..`+0x5b` (the pointer read again for each).
    pub fn set_window_rgba(mem: &mut impl Mem, i: i32, rgba: [u8; 4]) {
        for (k, c) in (0..4u32).zip(rgba) {
            let w: u32 = mem.read(
                WINDOWS
                    .ptr(mem)
                    .addr
                    .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
            );
            mem.write(w.wrapping_add(0x58 + k), c);
        }
    }

    /// `0x008b84f0(win, n, v)`: byte 1 of macro record `n` of message
    /// window `win` (the battle form of [`macro_set_type`]'s second byte).
    pub fn macro_set_byte1(mem: &mut impl Mem, win: i32, n: i32, v: u8) {
        let r = macro_record(mem, win, n);
        mem.write(r.wrapping_add(1), v);
    }

    fn table_entry(mem: &impl Mem, i: i32) -> u32 {
        mem.read(
            TABLE_01868A90
                .ptr(mem)
                .addr
                .wrapping_add(i.cast_unsigned().wrapping_mul(4)),
        )
    }

    /// `0x008b7fb0(i)`: byte 1 (the kind) of entry `i`, sign-extended.
    pub fn table_kind(mem: &impl Mem, i: i32) -> i32 {
        i32::from(mem.read::<i8>(table_entry(mem, i).wrapping_add(1)))
    }

    /// `0x008b7fd0(i)`: `entry + 0x18` for a kind-2 entry `i`, else 0.
    pub fn table_kind2_data(mem: &impl Mem, i: i32) -> u32 {
        let e = table_entry(mem, i);
        if mem.read::<u8>(e.wrapping_add(1)) == 2 {
            e.wrapping_add(0x18)
        } else {
            0
        }
    }

    /// `0x008b7ff0(i)`: byte 7 (sign-extended) of a kind-2 entry `i`, else
    /// -1.
    pub fn table_kind2_byte7(mem: &impl Mem, i: i32) -> i32 {
        let e = table_entry(mem, i);
        if mem.read::<u8>(e.wrapping_add(1)) == 2 {
            i32::from(mem.read::<i8>(e.wrapping_add(7)))
        } else {
            -1
        }
    }

    /// `TOAllCloseMessageWindow()` (`0x008ab7b0`): [`close_window`] and
    /// [`close_message`] for all eight windows.
    pub fn all_close_message_window(mem: &mut impl Mem) {
        for i in 0..8 {
            close_window(mem, i);
            close_message(mem, i);
        }
    }

    /// `TOControlWindow(i)` (`0x00906bd0`): an open window's control
    /// function (`+0x74`, passed to `call` with the window), else 0.
    pub fn control_window<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = entry(mem, WINDOWS, i);
        if mem.read::<u16>(w) == 0 {
            return 0;
        }
        let f: u32 = mem.read(w.wrapping_add(0x74));
        call(mem, f, &[w])
    }

    /// `TODrawWindow(i)` (`0x00906bf0`): an open window's draw function
    /// (`+0x78`) with the window; a closed one returns its address.
    pub fn draw_window<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = entry(mem, WINDOWS, i);
        if mem.read::<u16>(w) == 0 {
            return w;
        }
        let f: u32 = mem.read(w.wrapping_add(0x78));
        call(mem, f, &[w])
    }

    /// `TOControlMessage(i)` (`0x008b5850`): an open (state 1) message
    /// window's control function (`+0x114`) with the window, when set;
    /// else 0.
    pub fn control_message<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let m = entry(mem, MSG_WINDOWS, i);
        if mem.read::<u16>(m) != 1 {
            return 0;
        }
        let f: u32 = mem.read(m.wrapping_add(0x114));
        if f == 0 {
            return 0;
        }
        call(mem, f, &[m])
    }

    /// `0x008b8ca0(i)`: when the byte at `*(msg + 0x98)` is set, the
    /// message window's function `+0xf8` with the window; else that
    /// pointer (left in `eax`).
    pub fn message_call_f8<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let m = entry(mem, MSG_WINDOWS, i);
        let flag: u32 = mem.read(m.wrapping_add(0x98));
        if mem.read::<u8>(flag) == 0 {
            return flag;
        }
        let f: u32 = mem.read(m.wrapping_add(0xf8));
        call(mem, f, &[m])
    }

    /// The pad-status latch `TOGetControlPadStatus` keeps (`0x00c5a03c`).
    pub const PAD_STATUS_LATCH: Global<u8> = Global::new(0x00c5_a03c);

    /// `TOGetControlPadStatus(i)` (`0x008b6b70`): for a message window in
    /// state 2 (`+0x2c`), the latch set and its function `+0xfc` called with
    /// `i`; otherwise the latch cleared and `i` returned.
    pub fn control_pad_status<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let m = entry(mem, MSG_WINDOWS, i);
        if mem.read::<u16>(m.wrapping_add(0x2c)) != 2 {
            PAD_STATUS_LATCH.set(mem, 0);
            return i.cast_unsigned();
        }
        if PAD_STATUS_LATCH.get(mem) == 0 {
            PAD_STATUS_LATCH.set(mem, 1);
        }
        let f: u32 = mem.read(m.wrapping_add(0xfc));
        call(mem, f, &[i.cast_unsigned()])
    }

    /// `0x008b8010(i)`: for a message window with `+0x1e` 1 whose table
    /// entry (`0x01868a90`) has a kind of 1 or more, its byte 2
    /// (sign-extended); else -1.
    pub fn message_table_byte2(mem: &impl Mem, i: i32) -> i32 {
        let m = entry(mem, MSG_WINDOWS, i);
        if mem.read::<u16>(m.wrapping_add(0x1e)) == 1 {
            let e = table_entry(mem, i);
            if mem.read::<i8>(e.wrapping_add(1)) >= 1 {
                return i32::from(mem.read::<i8>(e.wrapping_add(2)));
            }
        }
        -1
    }

    /// `TOSetPolyG4A_XYWH(p, x, y, w, h)` (`0x009048a0`): the four corners
    /// of a quad as 16-bit pairs: `(x, y)`, `(x + w, y)`, `(x, y + h)`,
    /// `(x + w, y + h)`.
    #[allow(clippy::many_single_char_names)] // the original's parameters
    pub fn set_poly_g4a_xywh(mem: &mut impl Mem, p: u32, x: u32, y: u32, w: u32, h: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        let (x0, x1, y0, y1) = (
            x as u16,
            x.wrapping_add(w) as u16,
            y as u16,
            y.wrapping_add(h) as u16,
        );
        mem.write(p, [x0, x1, x0, x1, y0, y0, y1, y1]);
    }

    /// The pair `0x008bdfe0` keeps (`0x025cc584` previous, `0x025cc588`
    /// current) and the flag it derives (`0x0186a178`, u16).
    pub const PAIR_PREV: Global<u32> = Global::new(0x025c_c584);
    /// See [`PAIR_PREV`].
    pub const PAIR_CUR: Global<u32> = Global::new(0x025c_c588);
    /// See [`PAIR_PREV`].
    pub const PAIR_SAME: Global<u16> = Global::new(0x0186_a178);

    /// `0x008bdfe0(prev, cur)`: the previous value (`prev`, or the stored
    /// current one for 0) and `cur` stored; the flag (and the result) 0x1000
    /// when they are equal, else 0.
    pub fn set_pair(mem: &mut impl Mem, prev: u32, cur: u32) -> u32 {
        let prev = if prev == 0 { PAIR_CUR.get(mem) } else { prev };
        PAIR_PREV.set(mem, prev);
        PAIR_CUR.set(mem, cur);
        let same: u16 = if prev == cur { 0x1000 } else { 0 };
        PAIR_SAME.set(mem, same);
        u32::from(same)
    }

    /// The 16 entries `0x008ff140` clears (`0x018762e0`, 16 bytes each).
    pub const ENTRIES_18762E0: Global<[u32; 4]> = Global::new(0x0187_62e0);

    /// `0x008ff140(i)`: entry `i` (below 16, signed) zeroed; returns its
    /// address (`i` itself otherwise).
    pub fn clear_entry_18762e0(mem: &mut impl Mem, i: i32) -> u32 {
        if i >= 16 {
            return i.cast_unsigned();
        }
        let at = ENTRIES_18762E0
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_shl(4));
        mem.write(at, [0u32; 4]);
        at
    }

    /// The screen height half `TOGetScreenWH` reads (`0x00c5d774`; the
    /// width is `pkt::bridge::SCREEN_W`).
    pub const SCREEN_H: Global<u16> = Global::new(0x00c5_d774);

    /// `0x009048f0(w, h)`: the low halves of `w` and `h` as the screen size.
    pub fn set_screen_wh(mem: &mut impl Mem, w: u32, h: u32) {
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        {
            crate::pkt::bridge::SCREEN_W.set(mem, w as u16);
            SCREEN_H.set(mem, h as u16);
        }
    }

    /// `0x008b9640(i)`: message window `i`'s object.
    pub fn message_window(mem: &impl Mem, i: i32) -> u32 {
        entry(mem, MSG_WINDOWS, i)
    }

    /// `0x008b9a50(i)`: entry `i` of the `0x01868a90` table.
    pub fn table_01868a90_entry(mem: &impl Mem, i: i32) -> u32 {
        table_entry(mem, i)
    }

    /// `0x008ca370(i, v)`: for `i` up to 3, byte `0x0186a42d`, `0x0186a42f`,
    /// `0x0186a429` or `0x0186a42b` set (in that order of `i`).
    pub fn set_byte_186a429(mem: &mut impl Mem, i: u32, v: u8) {
        const AT: [u32; 4] = [0x0186_a42d, 0x0186_a42f, 0x0186_a429, 0x0186_a42b];
        if let Some(&at) = AT.get(i as usize) {
            mem.write(mem.rebase(at), v);
        }
    }

    /// The three-state switch `0x00906740` steps (`0x018763e0`).
    pub const SWITCH_18763E0: Global<i32> = Global::new(0x0187_63e0);

    /// `0x00906740()`: the switch steps -1 to 0, 0 to 1, 1 to -1 (other
    /// values stay); returns the new value (or the unchanged one).
    pub fn step_switch(mem: &mut impl Mem) -> i32 {
        let v = SWITCH_18763E0.get(mem);
        let next = match v {
            -1 => 0,
            0 => 1,
            1 => -1,
            other => return other,
        };
        SWITCH_18763E0.set(mem, next);
        next
    }

    /// `0x0089e4f0(a, b, c)`: a 12-byte entry appended at `0x013402b8`
    /// (count `0x01340438`): `+0` 1, `+4` `a`, `+6` `b` (16 bits each), `+8`
    /// `c`. Returns `c`.
    pub fn push_entry_13402b8(mem: &mut impl Mem, a: u32, b: u32, c: u32) -> u32 {
        let count = Global::<u32>::new(0x0134_0438);
        let used = count.get(mem);
        let e = mem.rebase(0x0134_02b8).wrapping_add(used.wrapping_mul(12));
        mem.write(e, 1u16);
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        {
            mem.write(e.wrapping_add(4), a as u16);
            mem.write(e.wrapping_add(6), b as u16);
        }
        mem.write(e.wrapping_add(8), c);
        count.set(mem, used.wrapping_add(1));
        c
    }

    /// `0x008be050(v)`: for each of the byte-count `0x01869f18` entries
    /// (14 bytes from `0x01869f20`), `+4` = `v` and, when `v` is 3, `+2` =
    /// twice the entries after it.
    pub fn set_entries_1869f20(mem: &mut impl Mem, v: u32) {
        let count = u32::from(Global::<u8>::new(0x0186_9f18).get(mem));
        let base = mem.rebase(0x0186_9f22);
        for k in 0..count {
            let e = base + 0xe * k;
            #[allow(clippy::cast_possible_truncation)] // 16-bit stores
            {
                mem.write(e + 2, v as u16);
                if v == 3 {
                    mem.write(
                        e,
                        (count.wrapping_sub(k).wrapping_sub(1) as u16).wrapping_mul(2),
                    );
                }
            }
        }
    }

    /// `0x008a2ad0(rec)`: one step of a 4-step offset animation: the
    /// signed byte step `+0xf` picks 4, 7, 11 or 16 to add to the half
    /// `+0x10`; after the fourth the step restarts and `+0x14` becomes
    /// `0x008a2b10`. Returns 0.
    pub fn anim_step_8a2ad0(mem: &mut impl Mem, rec: u32) -> u32 {
        const STEPS: [i8; 4] = [4, 7, 11, 16];
        let c: i8 = mem.read(rec.wrapping_add(0xf));
        let add = STEPS[usize::try_from(c).expect("a step of 0..4")];
        let v: i16 = mem.read(rec.wrapping_add(0x10));
        mem.write(rec.wrapping_add(0x10), v.wrapping_add(i16::from(add)));
        let c = c.wrapping_add(1);
        mem.write(rec.wrapping_add(0xf), c);
        if c >= 4 {
            mem.write(rec.wrapping_add(0xf), 0u8);
            let next = mem.rebase(0x008a_2b10);
            mem.write(rec.wrapping_add(0x14), next);
        }
        0
    }

    /// The angle `0x00890f30` advances (`0x0133fa7c`, f32).
    pub const ANGLE_133FA7C: Global<f32> = Global::new(0x0133_fa7c);

    /// `0x00890f30()`: the angle advanced by the double at `0x00b5ee38`
    /// (pi / 30), stored as a float; past pi (`0x00b92288`) it wraps by
    /// 2 pi (`0x00b39378`).
    pub fn advance_angle(mem: &mut impl Mem) {
        let step = Global::<f64>::new(0x00b5_ee38).get(mem);
        let pi = Global::<f64>::new(0x00b9_2288).get(mem);
        let turn = Global::<f64>::new(0x00b3_9378).get(mem);
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let v = (f64::from(ANGLE_133FA7C.get(mem)) + step) as f32;
        ANGLE_133FA7C.set(mem, v);
        if f64::from(v) > pi {
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            ANGLE_133FA7C.set(mem, (f64::from(v) - turn) as f32);
        }
    }

    /// `0x00906c80(i)`: window `i` opened: `+0` 1, `+8` 0, its function
    /// `+0x60` called with it, then `+8` 1 and `+0x74` = the function at
    /// `+0x60 + 4 * (+8)`. Returns that function.
    pub fn open_window<M: Mem>(
        mem: &mut M,
        i: i32,
        call: impl FnOnce(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let w = entry(mem, WINDOWS, i);
        mem.write(w, 1u16);
        let w = entry(mem, WINDOWS, i);
        mem.write(w.wrapping_add(8), 0u16);
        let w = entry(mem, WINDOWS, i);
        let f: u32 = mem.read(w.wrapping_add(0x60));
        call(mem, f, &[w]);
        let w = entry(mem, WINDOWS, i);
        mem.write(w.wrapping_add(8), 1u16);
        let w = entry(mem, WINDOWS, i);
        let k = i32::from(mem.read::<i16>(w.wrapping_add(8)));
        let f: u32 = mem.read(
            w.wrapping_add(0x60)
                .wrapping_add(k.cast_unsigned().wrapping_mul(4)),
        );
        mem.write(w.wrapping_add(0x74), f);
        f
    }

    /// `0x008b8560(i)`: message window `i` opened: `+0` 1, `+0x2c` 0, its
    /// function `+0x100` called with it, `+0x114` = `+0x104`, then (a tail
    /// call) its function `+0xf8` with it.
    pub fn open_message<M: Mem>(
        mem: &mut M,
        i: i32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let m = entry(mem, MSG_WINDOWS, i);
        mem.write(m, 1u16);
        let m = entry(mem, MSG_WINDOWS, i);
        mem.write(m.wrapping_add(0x2c), 0u16);
        let m = entry(mem, MSG_WINDOWS, i);
        let f: u32 = mem.read(m.wrapping_add(0x100));
        call(mem, f, &[m]);
        let m = entry(mem, MSG_WINDOWS, i);
        let v: u32 = mem.read(m.wrapping_add(0x104));
        mem.write(m.wrapping_add(0x114), v);
        let m = entry(mem, MSG_WINDOWS, i);
        let f: u32 = mem.read(m.wrapping_add(0xf8));
        call(mem, f, &[m])
    }

    /// `0x008ab800(i)`: message-window slot `i`'s (`0x01865b3c`) three
    /// parts cleared: halves `+0`, `+8` of `*(slot + 0x10)`, `+0`, `+0x2c` of
    /// `*(slot + 0x14)`, bytes `+0`, `+1` of `*(slot + 0x18)`. Returns the
    /// third part.
    pub fn clear_message_slot(mem: &mut impl Mem, i: u32) -> u32 {
        let at = MSG_WINDOW_SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(4));
        let mut last = 0;
        for (off, field, wide) in [
            (0x10, 0, true),
            (0x14, 0, true),
            (0x18, 0, false),
            (0x10, 8, true),
            (0x14, 0x2c, true),
            (0x18, 1, false),
        ] {
            let slot: u32 = mem.read(at);
            let p: u32 = mem.read(slot.wrapping_add(off));
            if wide {
                mem.write(p.wrapping_add(field), 0u16);
            } else {
                mem.write(p.wrapping_add(field), 0u8);
            }
            last = p;
        }
        last
    }

    /// `0x009047f0(p, k, r, g, b, a)`: corner `k` (0..=3) of a G4 quad's
    /// colours (bytes `+0x10 + k`, `+0x14 + k`, `+0x18 + k`, `+0x1c + k`), or
    /// all four corners for a larger `k`.
    pub fn set_quad_rgba(mem: &mut impl Mem, p: u32, k: u32, rgba: [u8; 4]) {
        for (c, v) in (0u32..).zip(rgba) {
            let at = p.wrapping_add(0x10 + 4 * c);
            if k <= 3 {
                mem.write(at.wrapping_add(k), v);
            } else {
                mem.write(at, [v; 4]);
            }
        }
    }

    /// `0x008b8b20(i, x, y, w, h)`: message window `i`'s area: halves `+0x40`
    /// `w`, `+0x42` `h`, floats `+0x30` `x`, `+0x34` `y`, `+0x38` `x + w`,
    /// `+0x3c` `y + h`.
    #[allow(clippy::cast_precision_loss)] // fild then fstp dword
    pub fn set_message_area(mem: &mut impl Mem, i: i32, r: [i32; 4]) {
        let m = entry(mem, MSG_WINDOWS, i);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 16-bit stores
        {
            mem.write(m.wrapping_add(0x40), r[2] as u16);
            mem.write(m.wrapping_add(0x42), r[3] as u16);
        }
        let f = [r[0], r[1], r[0].wrapping_add(r[2]), r[1].wrapping_add(r[3])];
        mem.write(m.wrapping_add(0x30), f.map(|v| v as f32));
    }

    /// `0x00900f80(i, x, y, d)`: entry `i` (below 16) of [`ENTRIES_18762E0`]
    /// moved to `(x, y)` (halves `+2`, `+4`; `+0` 1) unless it is already
    /// there; unless its `+0xe` is 1, it restarts: `+8` 0, `+6` `d`, `+0xc`
    /// `d / 14 + 2`.
    #[allow(clippy::many_single_char_names)] // the original's parameters
    pub fn move_entry_18762e0(mem: &mut impl Mem, i: i32, x: i32, y: i32, d: i32) {
        if i >= 16 {
            return;
        }
        let e = ENTRIES_18762E0
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned() << 4);
        if x == i32::from(mem.read::<i16>(e + 2)) && y == i32::from(mem.read::<i16>(e + 4)) {
            return;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 16-bit stores
        {
            mem.write(e, 1u16);
            mem.write(e + 2, x as u16);
            mem.write(e + 4, y as u16);
            if mem.read::<u16>(e + 0xe) == 1 {
                return;
            }
            mem.write(e + 8, 0u32);
            mem.write(e + 6, d as u16);
            mem.write(e + 0xc, (d / 14 + 2) as u16);
        }
    }

    /// `TOAllClearMessageWindow()` (`0x008ab590`): [`clear_message_slot`]
    /// for the eight slots in order (inlined); returns the last third part.
    pub fn all_clear_message_windows(mem: &mut impl Mem) -> u32 {
        let mut last = 0;
        for i in 0..8 {
            last = clear_message_slot(mem, i);
        }
        last
    }

    /// `0x008ade10(i)`: message-window slot `i`'s object (`0x01865b3c`)
    /// marked (`u16` 1 at `+0`), then [`open_window`] and [`open_message`]
    /// for `i`; what the latter returns.
    pub fn open_window_and_message<M: Mem>(
        mem: &mut M,
        i: i32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let at = MSG_WINDOW_SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_mul(4));
        let slot: u32 = mem.read(at);
        mem.write(slot, 1u16);
        open_window(mem, i, &mut call);
        open_message(mem, i, call)
    }

    /// The open yes/no window `TOMenuCloseYesNoWindow` closes
    /// (`0x023cc120`, labelled `g_TkMn2ListMax2` after its other reader).
    pub const YES_NO_WINDOW: Global<u32> = Global::new(0x023c_c120);

    /// `TOMenuCloseYesNoWindow()` (`0x008e2de0`): with a window open, in
    /// state 2 (`+0x94`) its scene `0x00b66904` stopped on layer 1
    /// (`TOSeqStopScene`), then `+0x41` = 1 and the pointer cleared. Returns
    /// the window (0 with none open).
    pub fn menu_close_yes_no_window(mem: &mut impl Mem) -> u32 {
        let w = YES_NO_WINDOW.get(mem);
        if w == 0 {
            return 0;
        }
        if mem.read::<u32>(w.wrapping_add(0x94)) == 2 {
            let name = mem.rebase(0x00b6_6904);
            crate::scene::bridge::stop_scene(mem, 1, name);
        }
        let w = YES_NO_WINDOW.get(mem);
        mem.write(w.wrapping_add(0x41), 1u8);
        YES_NO_WINDOW.set(mem, 0);
        w
    }

    /// `TOControlMessageWindowForMenu()` (`0x008ab9a0`): for each of the
    /// eight message-window slots in use (`u16` at its object): the window
    /// controlled ([`control_window`]), the message ([`control_message`]),
    /// a non-zero window result setting the message function
    /// ([`set_message_func`] with the `i16` at `+8` of `*(slot + 0x10)`), and
    /// a slot in state 1 (`+6`) polling the pad ([`control_pad_status`]).
    /// Then with both menu pad words (`0x025d09d2`, `+2`) 0x100 the debug
    /// print flag advances (a tail call, its sign-extended byte returned);
    /// else 0x100.
    pub fn control_message_window_for_menu<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let slots = MSG_WINDOW_SLOTS.ptr(mem).addr;
        for i in 0..8i32 {
            let at = slots.wrapping_add(i.cast_unsigned() * 4);
            let s: u32 = mem.read(at);
            if mem.read::<u16>(s) == 0 {
                continue;
            }
            let r = control_window(mem, i, &mut call);
            control_message(mem, i, &mut call);
            if r != 0 {
                let s: u32 = mem.read(at);
                let p: u32 = mem.read(s.wrapping_add(0x10));
                let n = i32::from(mem.read::<i16>(p.wrapping_add(8)));
                set_message_func(mem, i, n);
            }
            let s: u32 = mem.read(at);
            if mem.read::<u16>(s.wrapping_add(6)) == 1 {
                control_pad_status(mem, i, &mut call);
            }
        }
        let pad = crate::pad::bridge::FIRST.get(mem);
        if pad[0] == 0x100 && pad[1] == 0x100 {
            crate::pad::bridge::add_print_flag(mem);
            let v = crate::pad::bridge::PRINT_FLAG.get(mem);
            return i32::from(v.cast_signed()).cast_unsigned();
        }
        0x100
    }
}
