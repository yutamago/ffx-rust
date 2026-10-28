//! M5 stage 1, list `menu`, batch 5: the `Gu_` selector (a popup list
//! window), the message box and the host file list.
//!
//! - **Selector** (as `Gu_SelectorOpen` fills it, on top of
//!   [`crate::gu`]'s fields): `+0x00` the scroll offset in pixels (`i16`,
//!   9 per row), `+0x02` the selected row, `+0x04` the item count (`i16`),
//!   `+0x08` item names, `+0x0c` item values, `+0x10` visible rows (`i32`),
//!   `+0x14` return the mouse on close, `+0x18`/`+0x1c` the mouse when
//!   opened, `+0x20` the owner window, `+0x24` the owner's argument,
//!   `+0x28` a callback that sees every message first.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    use crate::gu::bridge::MOUSE_Y;
    use crate::m5m_a::bridge::{POPUP_WINDOW, client_mouse};
    use crate::m5m_b::bridge::{window_add_control, window_create};

    pub const GU_PRINT_SET_COLOR: u32 = 0x0084_8740;
    pub const GU_PRINT_SET_X: u32 = 0x0084_8820;
    pub const GU_PRINT_SET_Y: u32 = 0x0084_8860;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const GU_PRINT_GET_WIDTH: u32 = 0x0084_8d60;
    pub const GU_WINDOW_MOUSE_IN: u32 = 0x0084_9490;
    pub const GU_SELECTOR_FIT_WINDOW: u32 = 0x0084_a000;
    pub const GU_WINDOW_START_DRAG: u32 = 0x0084_a580;
    pub const GU_WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
    pub const SG_MAKE_FILELIST: u32 = 0x0085_09e0;
    pub const SG_SET_MOUSE: u32 = 0x0085_1ae0;
    /// The selector's and the message box's window procedures.
    pub const SELECTOR_PROC: u32 = 0x0084_9e30;
    pub const MESSAGE_BOX_PROC: u32 = 0x0084_9a10;
    /// The message box's button label, the file list command's format
    /// and its output file name, in `.rdata`.
    pub const OK_LABEL: u32 = 0x00b5_1c04;
    pub const FILE_LIST_FORMAT: u32 = 0x00b5_aaec;
    pub const FILE_LIST_OUTPUT: u32 = 0x00b5_aadc;
    /// The import slot of the CRT's `sprintf`.
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn i16_at(h: &impl Host, p: u32, o: u32) -> i32 {
        i32::from(h.read::<i16>(at(p, o)))
    }

    /// The common tail of the selector openers: the window's argument is
    /// the selector, the mouse remembered, the window the popup, fitted
    /// (`Gu_SelectorFitWindow`, whose `eax` is returned).
    fn selector_attach(h: &mut impl Host, selector: u32, window: u32) -> u32 {
        h.write(at(window, 0x98), selector);
        let x = crate::gu::bridge::MOUSE_X.get(h) as u32;
        h.write(at(selector, 0x18), x);
        POPUP_WINDOW.set(h, window);
        let y = MOUSE_Y.get(h) as u32;
        h.write(at(selector, 0x1c), y);
        cdecl(h, GU_SELECTOR_FIT_WINDOW, &[window, selector])
    }

    /// `Gu_SelectorOpen(selector, owner, arg)` (`0x00849d60`): a popup
    /// window at (190, 40), 120x101, for the selector.
    pub fn selector_open(h: &mut impl Host, selector: u32, owner: u32, arg: u32) -> u32 {
        h.write(at(selector, 0x24), arg);
        h.write(at(selector, 0x20), owner);
        let proc_ = h.rebase(SELECTOR_PROC);
        let w = window_create(h, 0xbe, 0x28, 0x78, 0x65, 0, proc_);
        selector_attach(h, selector, w)
    }

    /// `Gu_SelectorOpenXYWH(selector, owner, arg, x, y, w, h)`
    /// (`0x00849dc0`): the same at a given place, its height a whole
    /// number of 9-pixel rows plus 2.
    #[allow(clippy::too_many_arguments)]
    pub fn selector_open_xywh(
        h: &mut impl Host,
        selector: u32,
        owner: u32,
        arg: u32,
        x: u32,
        y: u32,
        w: u32,
        ht: u32,
    ) -> u32 {
        h.write(at(selector, 0x24), arg);
        h.write(at(selector, 0x20), owner);
        let rows = ((ht as i32) / 9).wrapping_mul(9).wrapping_add(2);
        let proc_ = h.rebase(SELECTOR_PROC);
        let win = window_create(h, x, y, w, rows as u32, 0, proc_);
        selector_attach(h, selector, win)
    }

    /// `Gu_MessageBox(text)` (`0x00849970`): a window centred at y 82, 30
    /// high, as wide as the text plus 30 (60 at least), with the text
    /// (kind 2, centred) and an OK button 0x4d2 (kind 1). Returns the
    /// button.
    pub fn message_box(h: &mut impl Host, text: u32) -> u32 {
        let tw = cdecl(h, GU_PRINT_GET_WIDTH, &[text]) as i32;
        let ww = tw.wrapping_add(0x1e).max(0x3c);
        let proc_ = h.rebase(MESSAGE_BOX_PROC);
        let win = window_create(
            h,
            ((0x200 - ww) / 2) as u32,
            0x52,
            ww as u32,
            0x1e,
            0,
            proc_,
        );
        let x = (i16_at(h, win, 0x10).wrapping_sub(tw) / 2) as u32;
        let c = window_add_control(h, win, u32::MAX, text, x, 2, tw as u32, 10);
        h.write(at(c, 0x2c), 2u16);
        let x = ((i16_at(h, win, 0x10) - 0x28) / 2) as u32;
        let ok = h.rebase(OK_LABEL);
        let b = window_add_control(h, win, 0x4d2, ok, x, 0xf, 0x28, 10);
        h.write(at(b, 0x2c), 1u16);
        b
    }

    /// `Gu_MessageBoxProc(window, msg, arg)` (`0x00849a10`): on create
    /// (0x8001) the window's rectangle jumps to its target; the OK button
    /// released (0x8006, id 0x4d2) starts the close; on paint (0x8011) the
    /// mouse over it scrolls and drags ([`client_mouse`]) and it stops
    /// being modal. Returns 0.
    #[allow(clippy::collapsible_match)] // the call stays out of a guard
    pub fn message_box_proc(h: &mut impl Host, window: u32, msg: u32, arg: u32) -> u32 {
        match msg {
            0x8001 => {
                for o in [0x14, 0x16, 0x18, 0x1a] {
                    let v = h.read::<u16>(at(window, o + 8));
                    h.write(at(window, o), v);
                }
            }
            0x8006 => {
                if arg == 0x4d2 {
                    cdecl(h, GU_WINDOW_START_DRAG, &[window]);
                }
            }
            0x8011 => {
                if cdecl(h, GU_WINDOW_MOUSE_IN, &[window]) != 0 {
                    client_mouse(h, window);
                    let f: u32 = h.read(at(window, 0x88));
                    h.write(at(window, 0x88), f & !1);
                }
            }
            _ => {}
        }
        0
    }

    /// `Gu_SelectorProc(window, msg, arg)` (`0x00849e30`): every message
    /// but create goes first to the selector's callback `+0x28` (unless
    /// the window is fading, flags 0x70); then the scroll bar captured
    /// (0x8003) puts the mouse back when asked (`Sg_SetMouse`), a left
    /// release outside any row (0x8006, id -1) tells the owner (0x8012
    /// with its argument) and closes, a right release (0x800a) closes
    /// the tree, and paint (0x8011) draws the rows ([`selector_paint`]).
    /// Returns 0.
    pub fn selector_proc(h: &mut impl Host, window: u32, msg: u32, arg: u32) -> u32 {
        let (mut sel, mut owner) = (0, 0);
        if msg != 0x8001 {
            sel = h.read(at(window, 0x98));
            let cb: u32 = h.read(at(sel, 0x28));
            owner = h.read(at(sel, 0x20));
            if cb != 0 && h.read::<u8>(at(window, 0x88)) & 0x70 == 0 {
                h.call(cb, Conv::Cdecl, &[window, msg, arg]);
            }
        }
        match msg {
            0x8003 => {
                if h.read::<u32>(at(sel, 0x14)) != 0 {
                    let (x, y): (u32, u32) = (h.read(at(sel, 0x18)), h.read(at(sel, 0x1c)));
                    cdecl(h, SG_SET_MOUSE, &[x, y]);
                }
            }
            0x8006 => {
                if arg == u32::MAX {
                    let f: u32 = h.read(at(owner, 0x7c));
                    POPUP_WINDOW.set(h, 0);
                    if f != 0 {
                        let a: u32 = h.read(at(sel, 0x24));
                        h.call(f, Conv::Cdecl, &[owner, 0x8012, a]);
                    }
                    cdecl(h, GU_WINDOW_START_DRAG, &[window]);
                }
            }
            0x800a => {
                let f: u32 = h.read(at(window, 0x88));
                h.write(at(window, 0x88), f | 2);
                POPUP_WINDOW.set(h, 0);
                cdecl(h, GU_WINDOW_CLOSE_TREE, &[window]);
            }
            0x8011 => selector_paint(h, window, sel),
            _ => {}
        }
        0
    }

    /// `Gu_SelectorPaint(window, selector)` (`0x00849c00`): the mouse
    /// above or below the window scrolls the list (clamped to the items),
    /// the row under the mouse becomes the selection, and the visible
    /// rows print in black, 9 pixels apart, the selection twice (one
    /// pixel right: bold).
    pub fn selector_paint(h: &mut impl Host, window: u32, sel: u32) {
        let d = MOUSE_Y.get(h).wrapping_sub(i16_at(h, window, 0xe));
        if d < 0 {
            let s = h.read::<u16>(sel);
            h.write(sel, s.wrapping_add(d as u16));
        }
        let n: i32 = h.read(at(sel, 0x10));
        if d >= n.wrapping_mul(9) {
            let s = h.read::<u16>(sel);
            let nine = (n as u16).wrapping_mul(9);
            h.write(sel, s.wrapping_add((d as u16).wrapping_sub(nine)));
        }
        if h.read::<i16>(sel) < 0 {
            h.write(sel, 0u16);
        }
        let total = h.read::<u16>(at(sel, 4));
        if i32::from(h.read::<i16>(sel)) >= (i32::from(total as i16).wrapping_sub(n) << 3) {
            h.write(sel, (total.wrapping_sub(n as u16)) << 3);
        }
        let row = (d / 9).max(0).min(n.wrapping_sub(1));
        let first = i32::from(h.read::<i16>(sel)) / 8;
        h.write(at(sel, 2), (row.wrapping_add(first)) as u16);
        cdecl(h, GU_PRINT_SET_COLOR, &[0, 0, 0, 0xff]);
        let mut y = 0u32;
        let mut idx = first;
        let mut i = 0i32;
        while i < h.read::<i32>(at(sel, 0x10)) {
            cdecl(h, GU_PRINT_SET_X, &[0]);
            cdecl(h, GU_PRINT_SET_Y, &[y]);
            let names: u32 = h.read(at(sel, 8));
            let s: u32 = h.read(at(names, (idx as u32).wrapping_mul(4)));
            cdecl(h, GU_PRINTF, &[s]);
            if idx == i32::from(h.read::<i16>(at(sel, 2))) {
                cdecl(h, GU_PRINT_SET_X, &[1]);
                let names: u32 = h.read(at(sel, 8));
                let s: u32 = h.read(at(names, (idx as u32).wrapping_mul(4)));
                cdecl(h, GU_PRINTF, &[s]);
            }
            y = y.wrapping_add(9);
            i += 1;
            idx = idx.wrapping_add(1);
        }
    }

    /// `strcmp`'s sign: the first differing byte, unsigned.
    fn strcmp(h: &impl Host, a: u32, b: u32) -> i32 {
        let mut i = 0u32;
        loop {
            let (x, y) = (h.read::<u8>(at(a, i)), h.read::<u8>(at(b, i)));
            if x != y {
                return if x < y { -1 } else { 1 };
            }
            if x == 0 {
                return 0;
            }
            i = i.wrapping_add(1);
        }
    }

    /// `Gu_SelectorSortItem(selector)` (`0x0084a180`): the items sorted by
    /// name (a selection sort, names and values swapped together).
    pub fn selector_sort_item(h: &mut impl Host, sel: u32) {
        let mut i = 0i32;
        while i < i32::from(h.read::<i16>(at(sel, 4))) {
            let mut min = i;
            let names: u32 = h.read(at(sel, 8));
            let mut j = i;
            while j < i32::from(h.read::<i16>(at(sel, 4))) {
                let a: u32 = h.read(at(names, (min as u32).wrapping_mul(4)));
                let b: u32 = h.read(at(names, (j as u32).wrapping_mul(4)));
                if strcmp(h, a, b) > 0 {
                    min = j;
                }
                j += 1;
            }
            for table in [8, 0xc] {
                let t: u32 = h.read(at(sel, table));
                let (pi, pm) = (at(t, (i as u32) * 4), at(t, (min as u32) * 4));
                let (vi, vm): (u32, u32) = (h.read(pi), h.read(pm));
                h.write(pi, vm);
                let t: u32 = h.read(at(sel, table));
                h.write(at(t, (min as u32) * 4), vi);
            }
            i += 1;
        }
    }

    /// `Gu_MakeFileList(list, path)` (`0x008498e0`, frame at `ebp`): the
    /// path copied to `ebp - 0x104` with `/` turned into `\`, the host
    /// command `"host0:EXEC:cmd /c dir <path> /b > \sugilist.tmp"` built
    /// at `ebp - 0x204`, and `sg_make_filelist(list, command, 0)`. Returns
    /// its `eax`.
    pub fn make_file_list(h: &mut impl Host, ebp: u32, list: u32, path: u32) -> u32 {
        let local = ebp.wrapping_sub(0x104);
        let mut i = 0u32;
        loop {
            let b = h.read::<u8>(at(path, i));
            h.write(at(local, i), b);
            i = i.wrapping_add(1);
            if b == 0 {
                break;
            }
        }
        let mut p = local;
        while h.read::<u8>(p) != 0 {
            if h.read::<u8>(p) == b'/' {
                h.write(p, b'\\');
            }
            p = at(p, 1);
        }
        let cmd = ebp.wrapping_sub(0x204);
        let (fmt, out) = (h.rebase(FILE_LIST_FORMAT), h.rebase(FILE_LIST_OUTPUT));
        let slot = h.rebase(SPRINTF_SLOT);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, &[cmd, fmt, local, out]);
        cdecl(h, SG_MAKE_FILELIST, &[list, cmd, 0])
    }
}
