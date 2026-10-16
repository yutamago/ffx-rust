//! The `Gu_` GUI layer: windows, their controls and selectors, the text
//! cursor, and the GS packets it queues.
//!
//! Layouts as these functions use them:
//!
//! - **Window:** `+0x00` next window (the open list, front first, head at
//!   `g_GuWindowList`), `+0x08` parent, `+0x0c`/`+0x0e` x/y (`i16`),
//!   `+0x10`/`+0x12` width/height, then two more copies of the rectangle at
//!   `+0x14` and `+0x1c` (`Gu_WindowOpen` puts the mouse in the first
//!   copy's position and zeroes its size), `+0x7c` message callback,
//!   `+0x80`/`+0x82` left/top border, `+0x84`/`+0x86` client width/height,
//!   `+0x88` flags (bit 1 cleared and bit 3 set on opening), `+0x90` control
//!   count (`i32`), `+0x94` controls, `+0x98` user argument.
//!
//!   x comes first: `Gu_PacketBeginTag` scales only `+0x16` (the copy of
//!   `+0x0e`) by the vertical scale, and `Gu_MouseInRect` and
//!   `Gu_DrawRectStrip` pair `+0x0c` with the width. So `0x01325240` is the
//!   mouse x, and Fahrenheit's (`BSim`) names `Gu_WindowGetX` (`+0x0e`),
//!   `Gu_MouseGetWindowX` and `Gu_MouseGetWindowY` have x and y swapped.
//! - **Control:** `0x44` bytes: `+0x2e` id (`u16`), `+0x40` flags
//!   ([`HIDE`], [`DISABLE`], [`TOGGLE`], [`TOGGLE_VALUE`]).
//! - **Selector:** `+0x02` selected (`i16`), `+0x04` count (`i16`), `+0x08`
//!   item names (pointers), `+0x0c` item values, `+0x14` "mouse fukki"
//!   (return-to-mouse), `+0x28` callback.
//! - **Packet:** 16-byte GIF quadwords at `g_GuPacket`, `g_GuPacketCount`
//!   of them used.

/// Control flag: hidden.
pub const HIDE: u32 = 1;
/// Control flag: disabled.
pub const DISABLE: u32 = 2;
/// Control flag: a toggle button.
pub const TOGGLE: u32 = 4;
/// Control flag: the toggle's state.
pub const TOGGLE_VALUE: u32 = 8;

/// Bytes per control.
pub const CONTROL: u32 = 0x44;

/// `flags` with the bits of `mask` taken from `value << shift`, as the
/// originals write it: `flags ^ ((value << shift) ^ flags) & mask`.
#[must_use]
pub const fn with_bits(flags: u32, mask: u32, shift: u32, value: u32) -> u32 {
    flags ^ (((value << shift) ^ flags) & mask)
}

/// `Gu_SelectorSetSelect`'s clamp: negative to 0; above `count` to
/// `count − 1`. `count` itself is kept (the original compares `>`).
#[must_use]
pub fn clamp_select(i: i32, count: i16) -> i16 {
    let i = i.max(0);
    let count = i32::from(count);
    #[allow(clippy::cast_possible_truncation)] // stored as i16 like the original
    if i > count {
        (count - 1) as i16
    } else {
        i as i16
    }
}

/// The two quadwords of `Gu_SetScissor`: a GIF tag (one A+D register) and
/// `SCISSOR_1` with x from `x0` to `x1` and y from `y0` to `y1`, the y
/// values in `scale / 256` steps. Low bounds below 0 become 0, high bounds
/// are capped at `0x1fd` and `0x19e`; the four fields are sign-extended
/// and combined with OR, so a negative high bound fills the bits above it.
#[must_use]
pub fn scissor_packet(scale: i32, x0: i32, x1: i32, y0: i32, y1: i32) -> [[u32; 4]; 2] {
    let y1 = scale.wrapping_mul(y1) >> 8;
    let y0 = scale.wrapping_mul(y0) >> 8;
    let x0 = x0.max(0);
    let x1 = x1.min(0x1fd);
    let y0 = y0.max(0);
    let y1 = y1.min(0x19e);
    let s = |v: i32| i64::from(v).cast_unsigned();
    let v = (((((s(y1) << 16) | s(y0)) << 16) | s(x1)) << 16) | s(x0);
    #[allow(clippy::cast_possible_truncation)]
    let (lo, hi) = (v as u32, (v >> 32) as u32);
    [[1, 0x1000_0000, 0xe, 0], [lo, hi, 0x40, 0]]
}

/// GS register fields packed the way the originals pack them: from the
/// highest field down, `v = (v << shift) | sign_extend(field)`, so a
/// negative field fills every bit above it.
#[must_use]
pub fn pack(fields: &[(i32, u32)]) -> u64 {
    fields.iter().fold(0u64, |v, &(x, shift)| {
        (v << shift) | i64::from(x).cast_unsigned()
    })
}

/// The originals, on the game's windows and globals (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{
        CONTROL, DISABLE, HIDE, TOGGLE, TOGGLE_VALUE, clamp_select, scissor_packet, with_bits,
    };

    /// The text cursor's origin x (`Gu_PrintSetX` adds to it).
    pub const PRINT_BASE_X: Global<i32> = Global::new(0x0130_36e0);
    /// The text cursor x.
    pub const PRINT_X: Global<i32> = Global::new(0x0130_36e4);
    /// The text cursor's origin y.
    pub const PRINT_BASE_Y: Global<i32> = Global::new(0x0130_36e8);
    /// The text cursor y.
    pub const PRINT_Y: Global<i32> = Global::new(0x0130_36ec);
    /// The text colour, three `u32` (`Gu_PrintSetColor`).
    pub const PRINT_COLOR: Global<[u32; 3]> = Global::new(0x0130_370c);
    /// The vertical scale of scissor rectangles, 8.8 fixed point.
    pub const H_SCALE: Global<i32> = Global::new(0x00c4_df54);
    /// Quadwords queued in [`PACKET`].
    pub const PACKET_COUNT: Global<u32> = Global::new(0x0130_36c0);
    /// The GS packet buffer.
    pub const PACKET: Global<u32> = Global::new(0x0130_36c4);
    /// The open windows, front first.
    pub const WINDOW_LIST: Global<u32> = Global::new(0x0130_3734);
    /// The window being dragged.
    pub const DRAG_WINDOW: Global<u32> = Global::new(0x0130_3738);
    /// The mouse capture (two words, `Gu_MouseSetCapture`).
    pub const MOUSE_CAPTURE: Global<[u32; 2]> = Global::new(0x0130_3754);
    /// Whether the GUI is hidden (`Gu_Hide`).
    pub const HIDDEN: Global<u32> = Global::new(0x0130_375c);
    /// The mouse x.
    pub const MOUSE_X: Global<i32> = Global::new(0x0132_5240);
    /// The mouse y.
    pub const MOUSE_Y: Global<i32> = Global::new(0x0132_5244);

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `Gu_PrintSetX(x)` (`0x00848820`).
    pub fn gu_print_set_x(mem: &mut impl Mem, x: i32) {
        let v = PRINT_BASE_X.get(mem).wrapping_add(x);
        PRINT_X.set(mem, v);
    }

    /// `Gu_PrintSetY(y)` (`0x00848860`).
    pub fn gu_print_set_y(mem: &mut impl Mem, y: i32) {
        let v = PRINT_BASE_Y.get(mem).wrapping_add(y);
        PRINT_Y.set(mem, v);
    }

    /// `Gu_PrintSetColor(r, g, b)` (`0x00848740`).
    pub fn gu_print_set_color(mem: &mut impl Mem, r: u32, g: u32, b: u32) {
        PRINT_COLOR.set(mem, [r, g, b]);
    }

    /// `Gu_SetHScale(scale)` (`0x00848b30`).
    pub fn gu_set_h_scale(mem: &mut impl Mem, scale: i32) {
        H_SCALE.set(mem, scale);
    }

    /// `Gu_Hide(on)` (`0x00849780`): sets the flag, returns the old one.
    pub fn gu_hide(mem: &mut impl Mem, on: u32) -> u32 {
        let old = HIDDEN.get(mem);
        HIDDEN.set(mem, on);
        old
    }

    /// `Gu_MouseSetCapture(a, b)` (`0x00849b60`).
    pub fn gu_mouse_set_capture(mem: &mut impl Mem, a: u32, b: u32) {
        MOUSE_CAPTURE.set(mem, [a, b]);
    }

    /// `Gu_MouseGetWindowX(window)` (`0x00849ac0`): the mouse y less the
    /// window's y (`+0x0e`; the name is Fahrenheit's `BSim` guess).
    pub fn gu_mouse_get_window_x(mem: &impl Mem, window: u32) -> i32 {
        MOUSE_Y
            .get(mem)
            .wrapping_sub(i32::from(mem.read::<i16>(at(window, 0xe))))
    }

    /// `Gu_MouseGetWindowY(window)` (`0x00849aa0`): the mouse x less the
    /// window's x (`+0x0c`).
    pub fn gu_mouse_get_window_y(mem: &impl Mem, window: u32) -> i32 {
        MOUSE_X
            .get(mem)
            .wrapping_sub(i32::from(mem.read::<i16>(at(window, 0xc))))
    }

    /// `Gu_WindowIsDrag(window)` (`0x0084b200`).
    pub fn gu_window_is_drag(mem: &impl Mem, window: u32) -> bool {
        DRAG_WINDOW.get(mem) == window
    }

    /// `Gu_WindowGetX(window)` (`0x0084afc0`).
    pub fn gu_window_get_x(mem: &impl Mem, window: u32) -> i32 {
        i32::from(mem.read::<i16>(at(window, 0xe)))
    }

    /// `Gu_WindowGetWidth(window)` (`0x0084afa0`).
    pub fn gu_window_get_width(mem: &impl Mem, window: u32) -> i32 {
        i32::from(mem.read::<i16>(at(window, 0x18)))
    }

    /// `Gu_WindowGetArg(window)` (`0x0084aec0`).
    pub fn gu_window_get_arg(mem: &impl Mem, window: u32) -> u32 {
        mem.read(at(window, 0x98))
    }

    /// `Gu_WindowSetArg(window, arg)` (`0x0084b560`).
    pub fn gu_window_set_arg(mem: &mut impl Mem, window: u32, arg: u32) {
        mem.write(at(window, 0x98), arg);
    }

    /// `Gu_WindowSetParent(window, parent)` (`0x0084b660`).
    pub fn gu_window_set_parent(mem: &mut impl Mem, window: u32, parent: u32) {
        mem.write(at(window, 8), parent);
    }

    /// `Gu_WindowSetClientH(window, h)` (`0x0084b580`): the client height,
    /// and the window's heights (`+0x12`, `+0x1a`, `+0x22`) moved by as much.
    pub fn gu_window_set_client_h(mem: &mut impl Mem, window: u32, h: i32) {
        let old = i32::from(mem.read::<i16>(at(window, 0x86)));
        let d = i32::from(mem.read::<i16>(at(window, 0x12))).wrapping_sub(old);
        #[allow(clippy::cast_possible_truncation)] // 16-bit stores
        {
            mem.write(at(window, 0x86), h as i16);
            let n = h.wrapping_add(d) as i16;
            for o in [0x12, 0x1a, 0x22] {
                mem.write(at(window, o), n);
            }
        }
    }

    /// `Gu_WindowOpen(window)` (`0x0084b250`): marks it open, notes the
    /// mouse, zeroes its width, copies its rectangle to `+0x1c`, and moves
    /// it to the front of the open list (when it is in it, not first).
    pub fn gu_window_open(mem: &mut impl Mem, window: u32) {
        let flags = mem.read::<u32>(at(window, 0x88));
        mem.write(at(window, 0x88), (flags & !2) | 8);
        let head = WINDOW_LIST.get(mem);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 16-bit stores
        {
            mem.write(at(window, 0x14), MOUSE_X.get(mem) as u16);
            mem.write(at(window, 0x16), MOUSE_Y.get(mem) as u16);
        }
        mem.write(at(window, 0x18), 0u32);
        for o in [0xc, 0xe, 0x10, 0x12] {
            let v = mem.read::<u16>(at(window, o));
            mem.write(at(window, o + 0x10), v);
        }
        let _ = head; // read before the stores, as the original does
        gu_window_to_front(mem, window);
    }

    /// `Gu_WindowToFront(window)` (`0x0084b770`): moves it to the front of
    /// the open list when it is in it and not first.
    pub fn gu_window_to_front(mem: &mut impl Mem, window: u32) {
        let head = WINDOW_LIST.get(mem);
        let mut prev = 0;
        let mut cur = head;
        if cur == 0 {
            return;
        }
        while cur != window {
            prev = cur;
            cur = mem.read(cur);
            if cur == 0 {
                return;
            }
        }
        if prev != 0 {
            let next = mem.read::<u32>(window);
            mem.write(prev, next);
            mem.write(window, head);
            WINDOW_LIST.set(mem, window);
        }
    }

    /// `Gu_WindowStartMove(window)` (`0x0084a530`): notes the mouse at
    /// `+0x1c`, zeroes `+0x20`, sets flag `0x10`, and does the same for
    /// every open window whose parent it is (recursively).
    pub fn gu_window_start_move(mem: &mut impl Mem, window: u32) {
        let mut child = WINDOW_LIST.get(mem);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 16-bit stores
        {
            mem.write(at(window, 0x1c), MOUSE_X.get(mem) as u16);
            mem.write(at(window, 0x1e), MOUSE_Y.get(mem) as u16);
        }
        mem.write(at(window, 0x20), 0u32);
        let flags = mem.read::<u32>(at(window, 0x88));
        mem.write(at(window, 0x88), flags | 0x10);
        while child != 0 {
            if mem.read::<u32>(at(child, 8)) == window {
                gu_window_start_move(mem, child);
            }
            child = mem.read(child);
        }
    }

    /// `Gu_WindowSetPos(window, x, y)` (`0x0084b220`): `+0x0c`/`+0x0e` and
    /// their copies at `+0x14` and `+0x1c`.
    pub fn gu_window_set_pos(mem: &mut impl Mem, window: u32, x: u16, y: u16) {
        gu_window_set_x(mem, window, x);
        gu_window_set_y(mem, window, y);
    }

    /// The three copies of a rectangle field 8 bytes apart, from `at`.
    fn set3(mem: &mut impl Mem, at: u32, v: u16) {
        for k in 0..3 {
            mem.write(at.wrapping_add(8 * k), v);
        }
    }

    /// `Gu_WindowSetX(window, x)` (`0x0084b6e0`): `+0x0c` and its copies.
    pub fn gu_window_set_x(mem: &mut impl Mem, window: u32, x: u16) {
        set3(mem, at(window, 0xc), x);
    }

    /// `Gu_WindowSetY(window, y)` (`0x0084b700`): `+0x0e` and its copies.
    pub fn gu_window_set_y(mem: &mut impl Mem, window: u32, y: u16) {
        set3(mem, at(window, 0xe), y);
    }

    /// `Gu_WindowSetW(window, w)` (`0x0084b6b0`): the width and its copies;
    /// client width = `w − 2 × left border`.
    pub fn gu_window_set_w(mem: &mut impl Mem, window: u32, w: u16) {
        set3(mem, at(window, 0x10), w);
        let bx = mem.read::<u16>(at(window, 0x80));
        mem.write(at(window, 0x84), w.wrapping_sub(bx.wrapping_add(bx)));
    }

    /// `Gu_WindowSetH(window, h)` (`0x0084b5d0`): the height and its
    /// copies; client height = `h − top border − 2`.
    pub fn gu_window_set_h(mem: &mut impl Mem, window: u32, h: u16) {
        set3(mem, at(window, 0x12), h);
        let by = mem.read::<u16>(at(window, 0x82));
        mem.write(at(window, 0x86), h.wrapping_sub(by).wrapping_sub(2));
    }

    /// `Gu_RectSetX`/`Y`/`W`/`H(rect, v)` (`0x008492f0`, `0x00849310`,
    /// `0x008492d0`, `0x008492b0`): field `field` (0 x, 1 y, 2 w, 3 h) of a
    /// three-copy rectangle like a window's.
    pub fn gu_rect_set(mem: &mut impl Mem, rect: u32, field: u32, v: u16) {
        set3(mem, rect.wrapping_add(2 * field), v);
    }

    /// `Gu_WindowSetFlag1(window, on)` (`0x0084b680`): flag bit 0 from
    /// `on`'s low bit.
    pub fn gu_window_set_flag1(mem: &mut impl Mem, window: u32, on: u32) {
        let f = mem.read::<u32>(at(window, 0x88));
        mem.write(at(window, 0x88), with_bits(f, 1, 0, on));
    }

    /// `Gu_WindowToggleFlag1(window)` (`0x0084b7b0`): flips bit 0; returns
    /// it as 0 or -1.
    pub fn gu_window_toggle_flag1(mem: &mut impl Mem, window: u32) -> i32 {
        let f = mem.read::<u32>(at(window, 0x88)) ^ 1;
        mem.write(at(window, 0x88), f);
        -((f & 1).cast_signed())
    }

    /// `Gu_WindowGetFlag2(window)` (`0x0084af40`): bit 1 as 0 or -1.
    pub fn gu_window_get_flag2(mem: &impl Mem, window: u32) -> i32 {
        -((mem.read::<u32>(at(window, 0x88)) >> 1 & 1).cast_signed())
    }

    /// `Gu_SelectorGetSelect(selector)` (`0x00849d50`), sign-extended.
    pub fn gu_selector_get_select(mem: &impl Mem, selector: u32) -> i32 {
        i32::from(mem.read::<i16>(at(selector, 2)))
    }

    /// Whether the mouse is strictly inside `(x, y)`–`(x + w, y + h)`.
    fn mouse_in(mem: &impl Mem, x: i32, y: i32, w: i32, h: i32) -> bool {
        let (mx, my) = (MOUSE_X.get(mem), MOUSE_Y.get(mem));
        mx > x && mx < x.wrapping_add(w) && my > y && my < y.wrapping_add(h)
    }

    /// `Gu_MouseInRect(rect, dx, dy)` (`0x00849440`): an `i16` rectangle
    /// `{x, y, w, h}` moved by `(dx, dy)`.
    pub fn gu_mouse_in_rect(mem: &impl Mem, rect: u32, dx: i32, dy: i32) -> bool {
        let f = |o: u32| i32::from(mem.read::<i16>(rect.wrapping_add(o)));
        mouse_in(
            mem,
            f(0).wrapping_add(dx),
            f(2).wrapping_add(dy),
            f(4),
            f(6),
        )
    }

    /// `Gu_MouseInClientRect(window, dx, dy, w, h)` (`0x00849b00`): a
    /// rectangle in the window's client area.
    pub fn gu_mouse_in_client_rect(
        mem: &impl Mem,
        window: u32,
        dx: i32,
        dy: i32,
        w: i32,
        h: i32,
    ) -> bool {
        let field = |o: u32| i32::from(mem.read::<i16>(at(window, o)));
        let x = field(0xc).wrapping_add(dx).wrapping_add(field(0x80));
        let y = field(0xe).wrapping_add(dy).wrapping_add(field(0x82));
        mouse_in(mem, x, y, w, h)
    }

    /// `Gu_DrawStart(buffer)` (`0x00847b00`): a GIF tag for one A+D
    /// register at `buffer + 0x10` (the first quadword is left for its
    /// DMA tag), the packet and its data both there, two quadwords used,
    /// then `TEST_1` = (1, 1, 0, 0, 0, 0, 1, 2).
    pub fn gu_draw_start(mem: &mut impl Mem, buffer: u32) {
        mem.write(at(buffer, 0x10), [1u32, 0x1000_0000, 0xe]);
        PACKET_DATA.set(mem, buffer);
        PACKET.set(mem, buffer);
        mem.write(at(buffer, 0x1c), 0u32);
        PACKET_COUNT.set(mem, 2);
        gu_set_test(mem, [1, 1, 0, 0, 0, 0, 1, 2]);
    }

    /// `Gu_DrawRectStrip(rect)` (`0x00854f60`): the outline of an `i16`
    /// rectangle `{x, y, w, h}` as five `XYZF2` vertices, back to the start
    /// (the rectangle read again for each, as the original does).
    pub fn gu_draw_rect_strip(mem: &mut impl Mem, rect: u32) {
        // per vertex: add the width?, add the height?
        for (right, down) in [
            (false, false),
            (true, false),
            (true, true),
            (false, true),
            (false, false),
        ] {
            let field = |o: u32| i32::from(mem.read::<i16>(rect.wrapping_add(o)));
            let vx = if right {
                field(4).wrapping_add(field(0))
            } else {
                field(0)
            };
            let vy = if down {
                field(6).wrapping_add(field(2))
            } else {
                field(2)
            };
            gu_set_xyzf2(mem, vx, vy);
        }
    }

    /// `Gu_WindowSetSize(window, w, h)` (`0x0084b720`): `+0x10`/`+0x12`
    /// and their copies at `+0x18` and `+0x20`; the client size (`+0x84`,
    /// `+0x86`) is what the border (`+0x80`, `+0x82`) leaves, on 16 bits.
    pub fn gu_window_set_size(mem: &mut impl Mem, window: u32, w: u16, h: u16) {
        gu_window_set_w(mem, window, w);
        gu_window_set_h(mem, window, h);
    }

    /// `Gu_WindowSetFlag2(window, on)` (`0x0084b600`): flag bit 1 (the one
    /// `Gu_WindowOpen` clears) from `on`'s low bit.
    pub fn gu_window_set_flag2(mem: &mut impl Mem, window: u32, on: u32) {
        let f = mem.read::<u32>(at(window, 0x88));
        mem.write(at(window, 0x88), with_bits(f, 2, 1, on));
    }

    /// A signed 16-bit field of a window, widened (the `Gu_WindowGet*`).
    pub fn gu_window_field(mem: &impl Mem, window: u32, offset: u32) -> i32 {
        i32::from(mem.read::<i16>(at(window, offset)))
    }

    /// `Gu_SelectorGetItemValue(selector)` (`0x00849d10`): the selected
    /// item's value.
    pub fn gu_selector_get_item_value(mem: &impl Mem, selector: u32) -> u32 {
        let i = i32::from(mem.read::<i16>(at(selector, 2)));
        let values = mem.read::<u32>(at(selector, 0xc));
        #[allow(clippy::cast_sign_loss)]
        mem.read(values.wrapping_add((i as u32).wrapping_mul(4)))
    }

    /// The text style words (`Gu_PrintSetItalic`, `Gu_PrintSetBold`).
    pub const PRINT_ITALIC: Global<u32> = Global::new(0x0130_3718);
    /// See [`PRINT_ITALIC`].
    pub const PRINT_BOLD: Global<u32> = Global::new(0x0130_3720);
    /// The start of the text size block `Gu_PrintSetSize` fills.
    pub const PRINT_SIZE: Global<u32> = Global::new(0x0130_36dc);
    /// Window movement smoothing on (`Gu_WindowSetHokanEnable`).
    pub const WINDOW_HOKAN: Global<u32> = Global::new(0x00c4_df60);

    /// `Gu_PrintSetXY(x, y)` (`0x00848840`).
    pub fn gu_print_set_xy(mem: &mut impl Mem, x: i32, y: i32) {
        gu_print_set_x(mem, x);
        gu_print_set_y(mem, y);
    }

    /// `Gu_PrintSetSize(size)` (`0x00848770`), `size` in 1/256: the size
    /// itself at `0x013036dc` and `0x01303728`, `size·8/256` at `0x013036fc`
    /// and `0x01303700`, one more at `0x013036f0` and `0x013036f4` (and as
    /// the running maximum at `0x013036f8`), `size·4/256` at `0x0130371c`,
    /// `size/256` at `0x01303724`.
    pub fn gu_print_set_size(mem: &mut impl Mem, size: i32) {
        let base = PRINT_SIZE.ptr(mem).addr;
        let line = size.wrapping_mul(8) >> 8;
        let w = |o: u32| base + o - 0x0130_36dc;
        mem.write(w(0x0130_3728), size);
        mem.write(w(0x0130_36fc), line);
        mem.write(w(0x0130_36f0), line.wrapping_add(1));
        mem.write(w(0x0130_371c), size.wrapping_mul(4) >> 8);
        mem.write(w(0x0130_3700), line);
        mem.write(w(0x0130_3724), size >> 8);
        mem.write(w(0x0130_36dc), size);
        mem.write(w(0x0130_36f4), line.wrapping_add(1));
        if mem.read::<i32>(w(0x0130_36f8)) < line.wrapping_add(1) {
            mem.write(w(0x0130_36f8), line.wrapping_add(1));
        }
    }

    /// The packet's data buffer (`Gu_SetPrim` and its siblings write there).
    pub const PACKET_DATA: Global<u32> = Global::new(0x0130_36c8);
    /// The open GIF tag (`Gu_PacketBeginTag`).
    pub const PACKET_TAG: Global<u32> = Global::new(0x0130_36cc);
    /// The quadword index of the open GIF tag.
    pub const PACKET_TAG_START: Global<u32> = Global::new(0x0130_36d0);
    /// The drawing origin `Gu_PacketBeginTag` takes from a window (x, y).
    pub const PACKET_ORIGIN: Global<[i32; 2]> = Global::new(0x0130_36d4);
    /// The screen size the vertices are centred in (height, width).
    pub const SCREEN_SIZE: Global<[i32; 2]> = Global::new(0x023c_bbe0);

    /// Appends an A+D register write to the data buffer.
    fn put_register(mem: &mut impl Mem, value: u64, register: u32) {
        let n = PACKET_COUNT.get(mem);
        let p = PACKET_DATA.get(mem).wrapping_add(n.wrapping_mul(16));
        #[allow(clippy::cast_possible_truncation)]
        mem.write(p, [value as u32, (value >> 32) as u32, register, 0]);
        PACKET_COUNT.set(mem, n.wrapping_add(1));
    }

    /// `Gu_PacketBeginTag(window)` (`0x00848530`): opens a GIF tag (A+D,
    /// count filled in by [`gu_packet_end_tag`]) and takes the drawing
    /// origin from the window's position and border (none for null).
    pub fn gu_packet_begin_tag(mem: &mut impl Mem, window: u32) {
        let n = PACKET_COUNT.get(mem);
        let tag = PACKET.get(mem).wrapping_add(n.wrapping_mul(16));
        PACKET_TAG_START.set(mem, n);
        PACKET_COUNT.set(mem, n.wrapping_add(1));
        mem.write(tag, [0x4d2u32, 0x1000_0000, 0xe, 0]);
        PACKET_TAG.set(mem, tag);
        let origin = if window == 0 {
            [0, 0]
        } else {
            let f = |o| i32::from(mem.read::<i16>(at(window, o)));
            let x = f(0x14).wrapping_add(f(0x80));
            let y = f(0x16).wrapping_add(f(0x82)).wrapping_mul(H_SCALE.get(mem)) >> 8;
            [x, y]
        };
        PACKET_ORIGIN.set(mem, origin);
    }

    /// `Gu_PacketEndTag()` (`0x00848460`): the open tag's loop count (15
    /// bits) set to the quadwords written since, less one.
    pub fn gu_packet_end_tag(mem: &mut impl Mem) {
        let tag = PACKET_TAG.get(mem);
        let n = PACKET_COUNT
            .get(mem)
            .wrapping_sub(PACKET_TAG_START.get(mem))
            .wrapping_sub(1);
        let lo = mem.read::<u32>(tag);
        mem.write(tag, with_bits(lo, 0x7fff, 0, n));
    }

    /// `Gu_SetPrim(prim)` (`0x00848480`): `PRIM`, sign-extended to 64 bits.
    pub fn gu_set_prim(mem: &mut impl Mem, prim: i32) {
        put_register(mem, i64::from(prim).cast_unsigned(), 0);
    }

    /// `Gu_SetRgbaq(r, g, b, a)` (`0x008484c0`): `RGBAQ`, `a | 0x800000`
    /// as the top field (see [`super::pack`]).
    pub fn gu_set_rgbaq(mem: &mut impl Mem, [red, green, blue, alpha]: [i32; 4]) {
        let v = super::pack(&[(alpha | 0x80_0000, 0), (blue, 8), (green, 8), (red, 8)]);
        put_register(mem, v, 1);
    }

    /// `Gu_SetTest([ate, atst, aref, afail, date, datm, zte, ztst])`
    /// (`0x008485d0`, eight arguments): `TEST_1`.
    pub fn gu_set_test(mem: &mut impl Mem, f: [i32; 8]) {
        let [
            alpha_on,
            alpha_test,
            alpha_ref,
            alpha_fail,
            dest_on,
            dest_mode,
            depth_on,
            depth_test,
        ] = f;
        let v = super::pack(&[
            (depth_test, 0),
            (depth_on, 1),
            (dest_mode, 1),
            (dest_on, 1),
            (alpha_fail, 2),
            (alpha_ref, 8),
            (alpha_test, 3),
            (alpha_on, 1),
        ]);
        put_register(mem, v, 0x47);
    }

    /// `Gu_SetXyzf2(x, y)` (`0x00848680`): `XYZF2` (12.4 fixed point) at
    /// the drawing origin, centred in the GS's 4096-pixel space; `y` in
    /// `g_GuHScale` steps; Z and F 0.
    pub fn gu_set_xyzf2(mem: &mut impl Mem, x: i32, y: i32) {
        let [height, width] = SCREEN_SIZE.get(mem);
        let [origin_x, origin_y] = PACKET_ORIGIN.get(mem);
        let half = |size: i32| 0x1000i32.wrapping_sub(size) / 2;
        let gs_x = half(width).wrapping_add(origin_x).wrapping_add(x) << 4;
        let scaled = (H_SCALE.get(mem).wrapping_mul(y) >> 8).wrapping_add(origin_y);
        let gs_y = half(height).wrapping_add(scaled) << 4;
        let count = PACKET_COUNT.get(mem);
        let at = PACKET_DATA.get(mem).wrapping_add(count.wrapping_mul(16));
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 16-bit fields
        mem.write(at, [gs_x as u16, gs_y as u16]);
        mem.write(at.wrapping_add(4), [0u32, 4, 0]);
        PACKET_COUNT.set(mem, count.wrapping_add(1));
    }

    /// The first control with id `id` (compared as `u16` widened, so an
    /// `id` above `0xffff` matches none): its address.
    fn find_control(mem: &impl Mem, window: u32, id: u32) -> Option<u32> {
        let count = mem.read::<i32>(at(window, 0x90));
        let controls = mem.read::<u32>(at(window, 0x94));
        #[allow(clippy::cast_sign_loss)] // count > 0 here
        (0..count.max(0) as u32)
            .map(|i| controls.wrapping_add(i.wrapping_mul(CONTROL)))
            .find(|&c| u32::from(mem.read::<u16>(at(c, 0x2e))) == id)
    }

    fn set_first(mem: &mut impl Mem, window: u32, id: u32, mask: u32, shift: u32, value: u32) {
        if let Some(c) = find_control(mem, window, id) {
            let f = mem.read::<u32>(at(c, 0x40));
            mem.write(at(c, 0x40), with_bits(f, mask, shift, value));
        }
    }

    /// `Gu_ButtonGetToggleValue(window, id)` (`0x00849330`).
    pub fn gu_button_get_toggle_value(mem: &impl Mem, window: u32, id: u32) -> u32 {
        find_control(mem, window, id)
            .map_or(0, |c| (mem.read::<u32>(at(c, 0x40)) & TOGGLE_VALUE) >> 3)
    }

    /// `Gu_ButtonSetToggleValue(window, id, on)` (`0x008493e0`).
    pub fn gu_button_set_toggle_value(mem: &mut impl Mem, window: u32, id: u32, on: u32) {
        set_first(mem, window, id, TOGGLE_VALUE, 3, on);
    }

    /// `Gu_ButtonSetToggle(window, id, on)` (`0x00849380`).
    pub fn gu_button_set_toggle(mem: &mut impl Mem, window: u32, id: u32, on: u32) {
        set_first(mem, window, id, TOGGLE, 2, on);
    }

    /// `Gu_WindowControlSetHide(window, id, on)` (`0x0084a670`).
    pub fn gu_window_control_set_hide(mem: &mut impl Mem, window: u32, id: u32, on: u32) {
        set_first(mem, window, id, HIDE, 0, on);
    }

    /// `Gu_WindowControlSetDisable(window, id, on)` (`0x0084a620`): unlike
    /// its siblings, every control with the id (compared on 16 bits).
    pub fn gu_window_control_set_disable(mem: &mut impl Mem, window: u32, id: u32, on: u32) {
        let mut i = 0i32;
        let mut c = 0u32;
        while i < mem.read::<i32>(at(window, 0x90)) {
            let controls = mem.read::<u32>(at(window, 0x94));
            let p = controls.wrapping_add(c);
            #[allow(clippy::cast_possible_truncation)]
            if mem.read::<u16>(at(p, 0x2e)) == id as u16 {
                let f = mem.read::<u32>(at(p, 0x40));
                mem.write(at(p, 0x40), with_bits(f, DISABLE, 1, on));
            }
            i += 1;
            c = c.wrapping_add(CONTROL);
        }
    }

    /// `Gu_SelectorGetItemName(selector)` (`0x00849d30`): the selected item's name.
    pub fn gu_selector_get_item_name(mem: &impl Mem, selector: u32) -> u32 {
        let i = i32::from(mem.read::<i16>(at(selector, 2)));
        let names = mem.read::<u32>(at(selector, 8));
        #[allow(clippy::cast_sign_loss)]
        mem.read(names.wrapping_add((i as u32).wrapping_mul(4)))
    }

    /// `Gu_SelectorSetItem(selector, i, name, value)` (`0x00849fd0`).
    pub fn gu_selector_set_item(mem: &mut impl Mem, selector: u32, i: i32, name: u32, value: u32) {
        #[allow(clippy::cast_sign_loss)]
        let o = (i as u32).wrapping_mul(4);
        let names = mem.read::<u32>(at(selector, 8));
        mem.write(names.wrapping_add(o), name);
        let values = mem.read::<u32>(at(selector, 0xc));
        mem.write(values.wrapping_add(o), value);
    }

    /// `Gu_SelectorSetSelect(selector, i)` (`0x0084a0e0`): see [`clamp_select`].
    pub fn gu_selector_set_select(mem: &mut impl Mem, selector: u32, i: i32) {
        let count = mem.read::<i16>(at(selector, 4));
        mem.write(at(selector, 2), clamp_select(i, count));
    }

    /// Whether the C strings at `a` and `b` are equal.
    pub(crate) fn str_eq(mem: &impl Mem, a: u32, b: u32) -> bool {
        let mut i = 0u32;
        loop {
            let x = mem.read::<u8>(a.wrapping_add(i));
            if x != mem.read::<u8>(b.wrapping_add(i)) {
                return false;
            }
            if x == 0 {
                return true;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `Gu_SelectorSetSelectSTR(selector, name)` (`0x0084a110`): selects
    /// the first item named `name` (null names skipped); nothing when none
    /// is, or `name` is null.
    pub fn gu_selector_set_select_str(mem: &mut impl Mem, selector: u32, name: u32) {
        if name == 0 {
            return;
        }
        let count = i32::from(mem.read::<i16>(at(selector, 4)));
        let names = mem.read::<u32>(at(selector, 8));
        for i in 0..count.max(0) {
            #[allow(clippy::cast_sign_loss)]
            let item = mem.read::<u32>(names.wrapping_add(i as u32 * 4));
            if item != 0 && str_eq(mem, item, name) {
                #[allow(clippy::cast_possible_truncation)]
                mem.write(at(selector, 2), i as i16);
                return;
            }
        }
    }

    /// `Gu_SelectorSetMouseFukki(selector, on)` (`0x0084a0d0`).
    pub fn gu_selector_set_mouse_fukki(mem: &mut impl Mem, selector: u32, on: u32) {
        mem.write(at(selector, 0x14), on);
    }

    /// `Gu_SelectorSetCallback(selector, callback)` (`0x00849fc0`).
    pub fn gu_selector_set_callback(mem: &mut impl Mem, selector: u32, callback: u32) {
        mem.write(at(selector, 0x28), callback);
    }

    fn put_quad(mem: &mut impl Mem, packet: u32, index: u32, q: [u32; 4]) {
        mem.write(packet.wrapping_add(index.wrapping_mul(16)), q);
    }

    /// `Gu_SetScissor(x0, x1, y0, y1)` (`0x00848b40`): queues
    /// [`scissor_packet`].
    pub fn gu_set_scissor(mem: &mut impl Mem, x0: i32, x1: i32, y0: i32, y1: i32) {
        let n = PACKET_COUNT.get(mem);
        let packet = PACKET.get(mem);
        let [tag, data] = scissor_packet(H_SCALE.get(mem), x0, x1, y0, y1);
        put_quad(mem, packet, n, tag);
        put_quad(mem, packet, n.wrapping_add(1), data);
        PACKET_COUNT.set(mem, n.wrapping_add(2));
    }

    /// `Gu_SetTexFlush()` (`0x00848d10`): queues a GIF tag and `TEXFLUSH`;
    /// the register's data half is left as it was.
    pub fn gu_set_tex_flush(mem: &mut impl Mem) {
        let n = PACKET_COUNT.get(mem);
        let packet = PACKET.get(mem);
        put_quad(mem, packet, n, [1, 0x1000_0000, 0xe, 0]);
        let reg = packet
            .wrapping_add(n.wrapping_add(1).wrapping_mul(16))
            .wrapping_add(8);
        mem.write(reg, [0x3fu32, 0]);
        PACKET_COUNT.set(mem, n.wrapping_add(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_and_clamps() {
        assert_eq!(with_bits(0xff, TOGGLE_VALUE, 3, 0), 0xf7);
        assert_eq!(with_bits(0, TOGGLE_VALUE, 3, 3), 8);
        assert_eq!(with_bits(1, HIDE, 0, 2), 0);
        assert_eq!(clamp_select(-3, 5), 0);
        assert_eq!(clamp_select(5, 5), 5);
        assert_eq!(clamp_select(6, 5), 4);
    }

    #[test]
    fn scissor() {
        let [tag, data] = scissor_packet(0x100, 10, 300, 20, 400);
        assert_eq!(tag, [1, 0x1000_0000, 0xe, 0]);
        // SCAX0 10, SCAX1 300, SCAY0 20, SCAY1 400
        assert_eq!(data, [0xa | 0x12c << 16, 0x14 | 0x190 << 16, 0x40, 0]);
        // a negative high bound fills the fields above it
        let [_, data] = scissor_packet(0x100, 0, -1, 0, 0);
        assert_eq!(data[..2], [0xffff_0000, 0xffff_ffff]);
    }
}
