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
}
