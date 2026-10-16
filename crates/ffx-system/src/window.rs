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
}
