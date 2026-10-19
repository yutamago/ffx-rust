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

    /// The file `MacroDic_Load` reads last (`0x01869ec0`).
    pub const MACRO_DIC_FILE: Global<u32> = Global::new(0x0186_9ec0);
    /// The swapped-language macro dictionary (`0x01869eb4`, after
    /// [`MACRO_DIC`]).
    pub const MACRO_DIC_SWAP: Global<u32> = Global::new(0x0186_9eb4);

    /// `MacroDic_Load()` (`0x008b85c0`): reads `"macrodic"`
    /// (`TKMallocAndReadEventMessage`, `0x0088cac0`) into
    /// [`MACRO_DIC_FILE`], keeps `set_ptr32` of it in [`MACRO_DIC`] and
    /// turns its 16 section offsets into pointers (offset plus file, 0 for
    /// offsets below 1). With `TOGetFFXLangSwapFlag() == 1` it does the same
    /// for `TODVDFileLoad(4, 4)` into [`MACRO_DIC_SWAP`]. Callees through
    /// `call`; returns what the original leaves in `eax` (the swap flag, or the
    /// second dictionary's last `get_ptr32_func`).
    pub fn macro_dic_load<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let name = mem.rebase(0x00b6_55d0);
        let file = call(mem, 0x0088_cac0, &[name]);
        load_macro_dic(mem, &mut call, file, MACRO_DIC);
        let swap = call(mem, 0x008a_c390, &[]);
        if swap != 1 {
            return swap;
        }
        let file = call(mem, 0x008a_ba60, &[4, 4]);
        load_macro_dic(mem, &mut call, file, MACRO_DIC_SWAP)
    }

    /// One dictionary of [`macro_dic_load`]: `file` into
    /// [`MACRO_DIC_FILE`], `set_ptr32(file)` into `table`, then each of the
    /// 16 words (the file and the table read again for each) through
    /// `get_ptr32_func`. Returns the last `get_ptr32_func`.
    fn load_macro_dic<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        file: u32,
        table: Global<u32>,
    ) -> u32 {
        MACRO_DIC_FILE.set(mem, file);
        let handle = call(mem, 0x00a4_4700, &[file]);
        table.set(mem, handle);
        let mut handle = handle;
        let mut last = 0;
        for k in (0..0x40).step_by(4) {
            let file = MACRO_DIC_FILE.get(mem);
            let offset: i32 = mem.read(file.wrapping_add(k));
            let value = if offset > 0 {
                offset.cast_unsigned().wrapping_add(file)
            } else {
                0
            };
            last = call(mem, 0x00a4_4650, &[handle]);
            mem.write(last.wrapping_add(k), value);
            handle = table.get(mem);
        }
        last
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
    /// `+0x10` (a step outside 0..4 reads the original's stack; here it
    /// adds 0); after the fourth the step restarts and `+0x14` becomes
    /// `0x008a2b10`. Returns 0.
    pub fn anim_step_8a2ad0(mem: &mut impl Mem, rec: u32) -> u32 {
        const STEPS: [i8; 4] = [4, 7, 11, 16];
        let c: i8 = mem.read(rec.wrapping_add(0xf));
        let add = usize::try_from(c)
            .ok()
            .and_then(|k| STEPS.get(k))
            .copied()
            .unwrap_or(0);
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

    /// The second offset animation's steps (`0x008a2b60`): -16, -11, -7,
    /// -4 added to the half `+0x10`.
    const STEPS_8A2B60: [i8; 4] = [-16, -11, -7, -4];

    /// `0x008a2b10(rec)` and `0x008a2bc0(rec)`: one step of a 4-step slide
    /// of the half `+0x12` a quarter of the way from row `i8 +0xd` to row
    /// `i8 +0xc` (0x16 per row, the quarter truncated toward 0) per step
    /// (the step counter `+0xf` counts first); after the fourth `+0x12`
    /// lands on row `+0xc` (a 16-bit product), the counter restarts and
    /// the step function `+0x14` becomes `next` (`0x008a2b60` for
    /// `0x008a2b10`, `0x008a2c10` for `0x008a2bc0`). Returns 0.
    #[allow(clippy::cast_possible_truncation)] // `add word`
    pub fn slide_step(mem: &mut impl Mem, rec: u32, next: u32) -> u32 {
        let to = i32::from(mem.read::<i8>(rec.wrapping_add(0xc)));
        let from = i32::from(mem.read::<i8>(rec.wrapping_add(0xd)));
        let step = mem.read::<i8>(rec.wrapping_add(0xf)).wrapping_add(1);
        mem.write(rec.wrapping_add(0xf), step);
        let quarter = (to - from) * 0x16 / 4;
        let y: i16 = mem.read(rec.wrapping_add(0x12));
        mem.write(rec.wrapping_add(0x12), y.wrapping_add(quarter as i16));
        if step >= 4 {
            mem.write(rec.wrapping_add(0xf), 0u8);
            mem.write(rec.wrapping_add(0x12), (to as i16).wrapping_mul(0x16));
            let next = mem.rebase(next);
            mem.write(rec.wrapping_add(0x14), next);
        }
        0
    }

    /// The end of an animation: the counter restarts, the step function
    /// is cleared, and a record whose row (`i8 +0xc`) is 0x10 or more is
    /// switched off (byte `+0` 0).
    fn anim_end(mem: &mut impl Mem, rec: u32) {
        mem.write(rec.wrapping_add(0xf), 0u8);
        mem.write(rec.wrapping_add(0x14), 0u32);
        if mem.read::<i8>(rec.wrapping_add(0xc)) >= 0x10 {
            mem.write(rec, 0u8);
        }
    }

    /// `0x008a2b60(rec)`: [`anim_step_8a2ad0`] backwards: the step `+0xf`
    /// picks -16, -11, -7 or -4 to add to the half `+0x10` (a step outside
    /// 0..4 reads the original's stack; here it adds 0); after the fourth
    /// the animation ends ([`anim_end`]). Returns 0.
    pub fn anim_step_8a2b60(mem: &mut impl Mem, rec: u32) -> u32 {
        let c: i8 = mem.read(rec.wrapping_add(0xf));
        let add = usize::try_from(c)
            .ok()
            .and_then(|k| STEPS_8A2B60.get(k))
            .copied()
            .unwrap_or(0);
        let v: i16 = mem.read(rec.wrapping_add(0x10));
        mem.write(rec.wrapping_add(0x10), v.wrapping_add(i16::from(add)));
        let c = c.wrapping_add(1);
        mem.write(rec.wrapping_add(0xf), c);
        if c >= 4 {
            anim_end(mem, rec);
        }
        0
    }

    /// `0x008a2c10(rec)`: a four-frame wait (`+0xf`) before the animation
    /// ends ([`anim_end`]). Returns 0.
    pub fn anim_wait_8a2c10(mem: &mut impl Mem, rec: u32) -> u32 {
        let c = mem.read::<i8>(rec.wrapping_add(0xf)).wrapping_add(1);
        mem.write(rec.wrapping_add(0xf), c);
        if c >= 4 {
            anim_end(mem, rec);
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

    /// `TORelayoutMessagesForLanguage_008b09b0()` (`0x008b09b0`): after
    /// the text language changes, every open message window (`*(slot +
    /// 0x14)` in state 1..=3 at `i16 +0x2c`, with a text at `+0x94`) takes
    /// its other text set: with English (`MsGetSaveConfigEnglish`) the
    /// words `+0x74..+0x7c`, else `+0x5c..+0x64`, into `+0x90..+0x98`.
    /// Then it is laid out again: `TOSetupMessageWindow_008b8de0(msg)`,
    /// `TOSetMessageWindowPosition(i, slot +0xc, +8, +0xa)` (`i16`s),
    /// `TOSnapWindowRect(*(slot + 0x10))`, and `TOWideWindowRect_008b8d10(i,
    /// msg + 0x48, x, y, w, h)` with `w`, `h` the `i16`s at `+0x44`,
    /// `+0x46` and `x`, `y` the text centred in the box `+0x40`, `+0x42`
    /// from the origin `f32 +0x30`, `+0x34` (`(i16) ftol(f32((box - size)
    /// / 2) + origin)`). Callees through `call`.
    #[allow(clippy::many_single_char_names)]
    pub fn relayout_messages_for_language<M: Mem>(
        mem: &mut M,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{add, ftol, load};
        const ENGLISH: u32 = 0x0078_51d0;
        let slots = MSG_WINDOW_SLOTS.ptr(mem).addr;
        let slot = |mem: &M, i: u32| mem.read::<u32>(slots + 4 * i);
        let message = |mem: &M, i: u32| mem.read::<u32>(slot(mem, i).wrapping_add(0x14));
        let half = |mem: &M, at: u32| i32::from(mem.read::<i16>(at)).cast_unsigned();
        for i in 0..8u32 {
            let msg = message(mem, i);
            let state: i16 = mem.read(msg.wrapping_add(0x2c));
            if !(1..=3).contains(&state) || mem.read::<u32>(msg.wrapping_add(0x94)) == 0 {
                continue;
            }
            let from = if call(mem, ENGLISH, &[]) == 0 {
                0x5c
            } else {
                0x74
            };
            for k in 0..3 {
                let msg = message(mem, i);
                let v: u32 = mem.read(msg.wrapping_add(from + 4 * k));
                mem.write(msg.wrapping_add(0x90 + 4 * k), v);
            }
            let msg = message(mem, i);
            call(mem, 0x008b_8de0, &[msg]);
            let s = slot(mem, i);
            let (anchor, x, y) = (half(mem, s + 0xc), half(mem, s + 8), half(mem, s + 0xa));
            call(mem, 0x008a_f850, &[i, anchor, x, y]);
            let rect = mem.read::<u32>(slot(mem, i).wrapping_add(0x10));
            call(mem, 0x0090_6e90, &[rect]);
            let msg = message(mem, i);
            let field = |off: u32| i32::from(mem.read::<i16>(msg.wrapping_add(off)));
            let (w, h) = (field(0x44), field(0x46));
            // the text centred in the box, from the window's origin
            let centre = |mem: &M, room: i32, origin: u32| {
                #[allow(clippy::cast_precision_loss)] // fild then fstp dword
                let offset = f64::from(room as f32);
                let at = add(offset, load(mem.read(msg.wrapping_add(origin))));
                #[allow(clippy::cast_possible_truncation)] // cwde: the low half
                let v = ftol(mem, at) as i16;
                i32::from(v).cast_unsigned()
            };
            let y = centre(mem, (field(0x42) - h) / 2, 0x34);
            let x = centre(mem, (field(0x40) - w) / 2, 0x30);
            let (w, h) = (w.cast_unsigned(), h.cast_unsigned());
            call(mem, 0x008b_8d10, &[i, msg.wrapping_add(0x48), x, y, w, h]);
        }
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

    /// A yes/no window (the record `TOMenuCloseYesNoWindow` closes): `+0x24`
    /// the message, `+0x28` the state, `u16 +0x2c` the answer, `+0x37` a
    /// click on "yes", `+0x45` the outcome (0 open, 1 answered, 0xff
    /// cancelled), `u16 +0x48` the cursor (0 the first choice), `+0x58`
    /// and `+0x5c` the two choices (no first choice: a message the pad
    /// dismisses), `i32 +0x60` the window's y offset, `+0x90` (not 1: any
    /// press dismisses the message), `+0x94` the icon (1 the shape 0xf3, 2
    /// the scene `scene7`).
    mod yes_no {
        pub const TEXT: u32 = 0x24;
        pub const STATE: u32 = 0x28;
        pub const ANSWER: u32 = 0x2c;
        pub const CLICK_YES: u32 = 0x37;
        pub const OUTCOME: u32 = 0x45;
        pub const CURSOR: u32 = 0x48;
        pub const YES: u32 = 0x58;
        pub const NO: u32 = 0x5c;
        pub const Y_OFFSET: u32 = 0x60;
        pub const ANY_PRESS: u32 = 0x90;
        pub const ICON: u32 = 0x94;
        /// `TOMenuGetControlPad()`, `TOMenuGetControlPadRep()`,
        /// `TOMenuGetControlPadTrg()` and `SndSepPlaySimple(sfx)`.
        pub const GET_PAD: u32 = 0x008b_e410;
        pub const GET_PAD_REP: u32 = 0x008b_e470;
        pub const GET_PAD_TRG: u32 = 0x008b_e4b0;
        pub const PLAY: u32 = 0x0088_6e80;
        /// The cursor and confirm sound, the cancel sound.
        pub const SFX_MOVE: u32 = 0x8000_0001;
        pub const SFX_CANCEL: u32 = 0x8000_0004;
    }

    /// `TOMenuCtrlYesNoWindow(win)` (`0x008e2e20`): one frame of a yes/no
    /// window's state machine, on the menu pad (`TOMenuGetControlPad()`,
    /// then the repeat and trigger words). State 0 opens it (outcome 0,
    /// cursor 1) and goes to 1 with choices, else to 2 (3 when `+0x90` is
    /// 1). 1: circle (trigger bit 0x20, a click when `+0x37` is set) or
    /// cross (0x40, the cancel sound) end it at once; else a repeated up
    /// or down (0x5000) clicks and flips the cursor. 2 waits for any
    /// trigger (a click), 3 not; both go to 4 for the next frame. 4: the
    /// answer is the cursor, outcome 1, state 5; the cancel (6): answer
    /// 1, outcome 0xff, state 7. 5, 7 and the rest do nothing. Callees
    /// through `call`.
    pub fn menu_ctrl_yes_no_window<M: Mem>(
        mem: &mut M,
        win: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use yes_no::{ANSWER, CURSOR, OUTCOME, PLAY, SFX_CANCEL, SFX_MOVE, STATE};
        let at = |off: u32| win.wrapping_add(off);
        call(mem, yes_no::GET_PAD, &[]);
        let repeat = call(mem, yes_no::GET_PAD_REP, &[]);
        let trigger = call(mem, yes_no::GET_PAD_TRG, &[]) & 0xffff;
        let mut state: u32 = mem.read(at(STATE));
        loop {
            match state {
                0 => {
                    mem.write(at(OUTCOME), 0u8);
                    mem.write(at(CURSOR), 1u16);
                    let next = if mem.read::<u32>(at(yes_no::YES)) != 0 {
                        1
                    } else if mem.read::<u32>(at(yes_no::ANY_PRESS)) == 1 {
                        2
                    } else {
                        3
                    };
                    mem.write(at(STATE), next);
                    return;
                }
                1 => {
                    if trigger & 0x20 != 0 {
                        if mem.read::<u8>(at(yes_no::CLICK_YES)) != 0 {
                            call(mem, PLAY, &[SFX_MOVE]);
                        }
                        state = 4;
                    } else if trigger & 0x40 != 0 {
                        call(mem, PLAY, &[SFX_CANCEL]);
                        state = 6;
                    } else {
                        if repeat & 0x5000 != 0 {
                            call(mem, PLAY, &[SFX_MOVE]);
                            let cursor = mem.read::<u16>(at(CURSOR));
                            mem.write(at(CURSOR), u16::from(cursor == 0));
                        }
                        return;
                    }
                }
                2 | 3 => {
                    if state == 2 {
                        if trigger == 0 {
                            return;
                        }
                        call(mem, PLAY, &[SFX_MOVE]);
                    }
                    mem.write(at(STATE), 4u32);
                    return;
                }
                4 => {
                    mem.write(at(OUTCOME), 1u8);
                    mem.write(at(ANSWER), mem.read::<u16>(at(CURSOR)));
                    mem.write(at(STATE), 5u32);
                    state = 5;
                }
                6 => {
                    mem.write(at(OUTCOME), 0xffu8);
                    mem.write(at(ANSWER), 1u16);
                    mem.write(at(STATE), 7u32);
                    state = 7;
                }
                _ => return,
            }
        }
    }

    /// The scratch [`menu_draw_yes_no_window`] needs: the message's width
    /// and height and the second choice's width (`f32`s).
    pub const DRAW_YES_NO_WORK: u32 = 12;

    /// `TOMenuDrawYesNoWindow(win)` (`0x008e2f80`): a yes/no window's
    /// plate, icon, message and choices. The message is measured
    /// (`TOGetMessageWH1612_008b6cb0(0, text, &w, &h, 0.78, 1, 0, 0)`);
    /// with choices the height grows by two `Y2(48)` and the second
    /// choice is measured (`ToGetBtlEasyFontWidth`). The plate is the text
    /// plus a margin of `X2(34)`/`Y2(15)` a side (`X2(80)` more for an
    /// icon, and at least `Y2(60)` plus the margins high, the text then
    /// centred), centred on the 512×416 screen and moved down by `+0x60`
    /// (`TODrawMenuPlateXYWHType(x, y, w, h, 10)`). Icon 1 is the shape
    /// 0xf3 (`X2(60)`×`Y2(60)`) at the margin, 2 places `scene7` on layer
    /// 1 (`TOSeqStartScene`, `TOSeqSetScenePosByName` at the plate's
    /// corner, `ftol`). The message goes at the margin (raised by `Y2(3)`
    /// without choices); the choices centred on the second one's width
    /// `Y2(9)` below the message and `Y2(48)` apart, the cross cursor
    /// (`TkMn2DrawCrossCursor(x - 5, y + 1, 0)`) by the one at `+0x48`.
    /// Texts drawn by `TOMkpCrossExtMesFontLClut(0, text, x, y, 0, 0.78,
    /// 1)`. All in x87 order (53-bit, `f32` at each store); the remaps
    /// (`graphicUiRemapX2`/`Y2`) through `fcall`, the rest through `call`;
    /// the sizes in `work` ([`DRAW_YES_NO_WORK`]).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn menu_draw_yes_no_window<M: Mem>(
        mem: &mut M,
        win: u32,
        work: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        use ffx_bridge::x87::{add, ftol, load, mul, round_trip, store, sub};
        use yes_no::{CURSOR, ICON, NO, TEXT, YES};
        const MESSAGE_WH: u32 = 0x008b_6cb0;
        const EASY_WIDTH: u32 = 0x0090_5290;
        const PLATE: u32 = 0x008f_5f70;
        const SHAPE: u32 = 0x0090_3bb0;
        const SEQ_START: u32 = 0x008a_f0d0;
        const SEQ_SET_POS: u32 = 0x008a_f060;
        const MES: u32 = 0x0090_16b0;
        const CROSS_CURSOR: u32 = 0x008c_0670;
        const REMAP_X2: u32 = 0x0064_47e0;
        const REMAP_Y2: u32 = 0x0064_4820;
        /// The text scale (`0x00b5ee10`).
        const SCALE: f32 = 0.78;
        let one = 1.0f32.to_bits();
        let field = |mem: &M, off: u32| mem.read::<u32>(win.wrapping_add(off));
        let mut remap = |mem: &mut M, y: bool, v: f32| {
            fcall(mem, if y { REMAP_Y2 } else { REMAP_X2 }, &[v.to_bits()])
        };
        // f32 cells as bits; registers as f64
        #[allow(clippy::cast_precision_loss)] // fild then fstp dword: rounded
        let int_f32 = |v: i32| (v as f32).to_bits();
        let (w_at, h_at, no_w_at) = (work, work + 4, work + 8);
        mem.write(no_w_at, 0u32);
        let (mut corner_x, mut corner_y) = (0u32, 0u32);

        let text = field(mem, TEXT);
        let scale = SCALE.to_bits();
        call(mem, MESSAGE_WH, &[0, text, w_at, h_at, scale, one, 0, 0]);
        let mut h: u32 = mem.read(h_at);
        let text_h = round_trip(f32::from_bits(h)).to_bits();
        if field(mem, YES) != 0 {
            let lines = remap(mem, true, 48.0);
            h = store(add(mul(lines, 2.0), load(h)));
            let no = field(mem, NO);
            call(mem, EASY_WIDTH, &[no, no_w_at, 0, scale, one]);
        }
        let mut margin_x = store(remap(mem, false, 34.0));
        let mut margin_y = store(remap(mem, true, 15.0));
        let mut w = store(add(mul(load(margin_x), 2.0), load(mem.read(w_at))));
        let rows = mul(load(margin_y), 2.0);
        h = store(add(rows, load(h)));
        if field(mem, ICON) != 0 {
            corner_x = round_trip(f32::from_bits(margin_x)).to_bits();
            corner_y = round_trip(f32::from_bits(margin_y)).to_bits();
            margin_x = store(add(remap(mem, false, 80.0), load(margin_x)));
            w = store(add(remap(mem, false, 80.0), load(w)));
            let shown = load(h);
            if add(remap(mem, true, 60.0), rows) > shown {
                // (the original stores the icon's y offset again here, the
                // same value)
                h = store(add(remap(mem, true, 60.0), rows));
                margin_y = store(mul(sub(load(h), load(text_h)), 0.5));
            }
        }
        let mut x = store(mul(sub(512.0, load(w)), 0.5));
        let top = mul(sub(416.0, load(h)), 0.5);
        let offset = int_f32(mem.read::<i32>(win.wrapping_add(yes_no::Y_OFFSET)));
        let mut y = store(add(top, load(offset)));
        call(mem, PLATE, &[x, y, w, h, 10]);

        let icon = field(mem, ICON);
        if icon == 1 {
            let size_y = store(remap(mem, true, 60.0));
            let size_x = store(remap(mem, false, 60.0));
            let at_y = store(add(load(y), load(corner_y)));
            let at_x = store(add(load(x), load(corner_x)));
            // u0, v0, u1, v1: 0.92676, 0.22852, 0.99902, 0.30176 (the
            // f32s at 0x00b65d5c, 0x00b65d54, 0x00b65d60, 0x00b65d58)
            let uv = [0x3f6d_4000u32, 0x3e6a_0000, 0x3f7f_c000, 0x3e9a_8000];
            call(
                mem,
                SHAPE,
                &[0xf3, at_x, at_y, size_x, size_y, uv[0], uv[1], uv[2], uv[3]],
            );
        }
        if field(mem, ICON) == 2 {
            let name = mem.rebase(0x00b6_6904);
            call(mem, SEQ_START, &[1, name]);
            let sy = ftol(mem, load(y));
            let sx = ftol(mem, load(x));
            call(mem, SEQ_SET_POS, &[1, name, sx, sy]);
        }

        x = store(add(load(x), load(margin_x)));
        let mut text_y = add(load(y), load(margin_y));
        if field(mem, YES) == 0 {
            text_y = sub(text_y, remap(mem, true, 3.0));
        }
        y = store(text_y);
        let text = field(mem, TEXT);
        call(mem, MES, &[0, text, x, y, 0, scale, one]);
        if field(mem, YES) == 0 {
            return;
        }

        // the choices
        x = store(mul(sub(512.0, load(mem.read(no_w_at))), 0.5));
        let gap = remap(mem, true, 9.0);
        y = store(add(gap, add(load(y), load(text_h))));
        let cursor = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, x, y| {
            let cy = store(add(load(y), 1.0));
            let cx = store(sub(load(x), 5.0));
            call(mem, CROSS_CURSOR, &[cx, cy, 0]);
        };
        let yes = field(mem, YES);
        call(mem, MES, &[0, yes, x, y, 0, scale, one]);
        if mem.read::<u16>(win.wrapping_add(CURSOR)) == 0 {
            cursor(mem, &mut call, x, y);
        }
        y = store(add(remap(mem, true, 48.0), load(y)));
        let no = field(mem, NO);
        call(mem, MES, &[0, no, x, y, 0, scale, one]);
        if mem.read::<u16>(win.wrapping_add(CURSOR)) != 0 {
            cursor(mem, &mut call, x, y);
        }
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
