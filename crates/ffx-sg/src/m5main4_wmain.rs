//! M5 stage 1, list `main`, round 4: `WMainProc`, the PS2 debug main
//! window's procedure (`0x0084f550`): the window's pages of buttons
//! (CHR, MAP, SYS, BTL, CAM, EFF, EVENT), its commands, its selectors and
//! its status refresh.
//!
//! The pages: `0x01323d6c` points at the page being filled (0x88 bytes:
//! `+0x7c` its row count, `+0x80` its title, `+0x84` its name), the row
//! counter is `0x0132412c` (from 0x17); the current page `0x01324128`.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the window, its state and values
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The window calls and the procedure's other callees (cdecl).
    pub const GET_ARG: u32 = 0x0084_aec0;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const SELECTOR_ALLOCATE: u32 = 0x0084_9bb0;
    pub const SET_ARG: u32 = 0x0084_b560;
    pub const ADD_BUTTON: u32 = 0x0084_a470;
    pub const ADD_STATIC: u32 = 0x0084_a4d0;
    pub const CHR_MENU_INIT: u32 = 0x0084_ca30;
    pub const SG_ADD_STATIC: u32 = 0x0085_08f0;
    pub const SG_ADD_BUTTON_S: u32 = 0x0085_08a0;
    pub const SG_ADD_BUTTON: u32 = 0x0085_0850;
    pub const MAPNAME: u32 = 0x0085_0940;
    pub const SET_TOGGLE: u32 = 0x0084_9380;
    pub const TOGGLE_VALUE: u32 = 0x0084_93e0;
    pub const GET_TOGGLE_VALUE: u32 = 0x0084_9330;
    pub const GETTH_DISP_ENABLE: u32 = 0x0082_0650;
    pub const GET_RESOURCE_STR: u32 = 0x0083_fc30;
    pub const GET_RESOURCE: u32 = 0x0083_fbe0;
    pub const SET_RESOURCE: u32 = 0x0083_fe10;
    pub const SELECT_MAIN_MENU: u32 = 0x0084_f270;
    pub const EVE_INIT: u32 = 0x0085_4e60;
    pub const LR_BUTTON_MODE: u32 = 0x0082_d840;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const WMAIN_PROC: u32 = 0x0084_f550;
    pub const SELECTOR_DISPOSE: u32 = 0x0084_9bd0;
    pub const CHR_SELECTOR: u32 = 0x0084_d270;
    pub const EVE_SELECTOR: u32 = 0x0085_4df0;
    pub const SELECTOR_ITEM_NAME: u32 = 0x0084_9d30;
    pub const MAP_SET_NAME: u32 = 0x0084_4fb0;
    pub const MAP_INIT: u32 = 0x0084_4d10;
    pub const SET_MOUSE_FUKKI: u32 = 0x0084_a0d0;
    pub const RESERVE_CALC_NORM: u32 = 0x0083_2550;
    pub const CAPTURE_ENABLE: u32 = 0x0083_a470;
    pub const COMMAND_CHR: u32 = 0x0084_cf60;
    pub const COMMAND_MENU: u32 = 0x0085_4c60;
    pub const MAP_GET_DIR: u32 = 0x0084_4cd0;
    pub const MAP_SET_DIR: u32 = 0x0084_4f90;
    pub const CONTROL_DISABLE: u32 = 0x0084_a620;
    pub const SET_TEXT: u32 = 0x0084_a710;
    pub const SET_TITLE: u32 = 0x0084_b6a0;
    pub const MASAKI_INIT: u32 = 0x007c_86e0;
    pub const DIR_LIST: u32 = 0x0085_0770;
    pub const FILE_LIST: u32 = 0x0085_06a0;
    pub const MAP_GET_NAME: u32 = 0x0084_4cf0;
    pub const SET_SELECT_STR: u32 = 0x0084_a110;
    pub const SELECTOR_OPEN: u32 = 0x0084_9d60;
    pub const MESSAGE_BOX: u32 = 0x0084_9970;
    pub const WINDOW_CREATE: u32 = 0x0084_a920;
    pub const FUKA_PROC: u32 = 0x0084_f490;
    pub const HIDE: u32 = 0x0084_9780;
    pub const TEX_GET: u32 = 0x0083_0320;
    pub const TEX_SET: u32 = 0x0083_0330;
    pub const TOGGLE_RESOURCE9: u32 = 0x0082_1e40;
    pub const SWITCH_DRAW_ID: u32 = 0x0084_5010;
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    /// The pages, the counters and the text buffer.
    pub const PAGE: u32 = 0x0132_3d6c;
    pub const ROWS: u32 = 0x0132_412c;
    pub const CUR_PAGE: u32 = 0x0132_4128;
    pub const LINE: u32 = 0x023c_4000;
    pub const DIR_COUNT: u32 = 0x0132_3d54;
    pub const FILE_COUNT: u32 = 0x0132_3d48;
    /// The clock (`0x023cbc28` flags, minutes `+4`, hours `+8`).
    pub const CLOCK_FLAGS: u32 = 0x023c_bc28;

    /// The window being created: its handle and the helpers on it.
    struct Build<'a, H: Host> {
        h: &'a mut H,
        w: u32,
    }

    impl<H: Host> Build<'_, H> {
        fn run(&mut self, f: u32, args: &[u32]) -> u32 {
            cdecl(self.h, f, args) // call width: 7 (Gu_WindowAddButton)
        }
        fn g(&self, a: u32) -> u32 {
            self.h.rebase(a)
        }
        fn button(&mut self, id: u32, text: u32) {
            let (w, t) = (self.w, self.g(text));
            self.run(SG_ADD_BUTTON, &[w, id, t]);
        }
        fn button_s(&mut self, id: u32, text: u32) {
            let (w, t) = (self.w, self.g(text));
            self.run(SG_ADD_BUTTON_S, &[w, id, t]);
        }
        fn label(&mut self, id: u32, text: u32) {
            let (w, t) = (self.w, self.g(text));
            self.run(SG_ADD_STATIC, &[w, id, t]);
        }
        fn rows(&mut self, n: u32) {
            let r = self.g(ROWS);
            let v: u32 = self.h.read(r);
            self.h.write(r, v.wrapping_add(n));
        }
        fn page(&mut self, at_: u32) {
            let p = self.g(PAGE);
            let page = self.g(at_);
            self.h.write(p, page);
            let r = self.g(ROWS);
            self.h.write(r, 0x17u32);
        }
        fn close(&mut self, title: u32, name: u32) {
            let page: u32 = self.h.read(self.g(PAGE));
            let rows: u32 = self.h.read(self.g(ROWS));
            self.h.write(at(page, 0x7c), rows);
            let (t, n) = (self.g(title), self.g(name));
            self.h.write(at(page, 0x80), t);
            self.h.write(at(page, 0x84), n);
        }
        fn toggle(&mut self, id: u32) {
            let w = self.w;
            self.run(SET_TOGGLE, &[w, id, 1]);
        }
        fn value(&mut self, id: u32, v: u32) {
            let w = self.w;
            self.run(TOGGLE_VALUE, &[w, id, v]);
        }
    }

    /// `WMainProc(window, msg, id)` (`0x0084f550`, cdecl). Its state (the
    /// window's argument) is two selectors. Returns 0.
    pub fn wmain_proc(h: &mut impl Host, window: u32, msg: u32, id: u32) -> u32 {
        let st = cdecl(h, GET_ARG, &[window]);
        match msg {
            0x8001 => create(h, window),
            0x8003 => {
                let sel: u32 = h.read(st);
                cdecl(h, SELECTOR_DISPOSE, &[sel]);
            }
            0x8006 => command(h, window, id, st),
            0x800a => match id {
                0xf => {
                    cdecl(h, CAPTURE_ENABLE, &[]);
                }
                0x19 => {
                    let cur = h.rebase(CUR_PAGE);
                    let mut p = h.read::<u32>(cur).wrapping_sub(1);
                    h.write(cur, p);
                    if p & 0x8000_0000 != 0 {
                        p = 6;
                        h.write(cur, p);
                    }
                    cdecl(h, SELECT_MAIN_MENU, &[window, p]);
                }
                0x45 => {
                    cdecl(h, RESERVE_CALC_NORM, &[]);
                }
                _ => {}
            },
            0x8011 => refresh(h, window),
            0x8012 => selector_done(h, window, id, st),
            0x8017 => {
                let cur = h.rebase(CUR_PAGE);
                let mut p = h.read::<u32>(cur).wrapping_sub(id);
                h.write(cur, p);
                if p.cast_signed() > 6 {
                    p = 0;
                    h.write(cur, p);
                } else if p & 0x8000_0000 != 0 {
                    p = 6;
                    h.write(cur, p);
                }
                cdecl(h, SELECT_MAIN_MENU, &[window, p]);
            }
            _ => {}
        }
        0
    }

    /// The window's creation (0x8001): its state, the header buttons and
    /// the seven pages.
    #[allow(clippy::too_many_lines)] // the original's straight list
    fn create(h: &mut impl Host, w: u32) {
        let st = cdecl(h, USER_MALLOC, &[8]);
        h.write(st, 0u32);
        h.write(at(st, 4), 0u32);
        let sel = cdecl(h, SELECTOR_ALLOCATE, &[]);
        h.write(st, sel);
        cdecl(h, SET_ARG, &[w, st]);
        let mut b = Build { h, w };
        for (f, id, text, x, y, wd, ht) in [
            (ADD_BUTTON, 0x4b, 0x00b5_b24c, 0, 0, 0xc, 9),
            (ADD_BUTTON, 0x4a, 0x00b5_5220, 0x10, 0, 0xc, 9),
            (ADD_STATIC, 0x48, 0x00b5_b250, 0x1d, 0, 0x1a, 9),
            (ADD_BUTTON, 0x49, 0x00b4_8250, 0x3c, 0, 0xc, 9),
            (ADD_BUTTON, 0x19, 0x00b5_ae60, 0, 0xa, 0x46, 0xa),
        ] {
            let t = b.g(text);
            b.run(f, &[w, id, t, x, y, wd, ht]);
        }
        // CHR: the character menu fills it
        let rows = b.g(ROWS);
        b.h.write(rows, 0x17u32);
        let chr = b.g(0x0132_3d70);
        let p = b.g(PAGE);
        b.h.write(p, chr);
        b.run(CHR_MENU_INIT, &[w, chr]);
        let page: u32 = b.h.read(p);
        let name = b.g(0x00b5_6a08);
        b.h.write(at(page, 0x84), name);
        // MAP
        b.page(0x0132_3df8);
        b.label(0x26, 0x00b8_fbeb);
        b.button_s(0x27, 0x00b5_b254);
        b.label(0x1a, 0x00b8_fbeb);
        b.button_s(0x1b, 0x00b5_adf0);
        b.label(0xa, 0x00b8_fbeb);
        b.button_s(9, 0x00b5_b230);
        b.run(MAPNAME, &[w]);
        b.button_s(0x52, 0x00b5_b25c);
        b.rows(1);
        b.button_s(0x29, 0x00b5_abe4);
        b.rows(2);
        for (id, text) in [
            (0xc, 0x00b5_ae48),
            (0xd, 0x00b5_b264),
            (0x44, 0x00b5_b26c),
            (0x56, 0x00b5_b274),
        ] {
            b.button_s(id, text);
        }
        b.rows(1);
        for (id, text) in [
            (0x4d, 0x00b5_ae40),
            (0x4f, 0x00b5_b27c),
            (0x63, 0x00b5_b280),
            (0x2b, 0x00b5_b28c),
        ] {
            b.button_s(id, text);
        }
        b.close(0x00b5_b294, 0x00b5_b230);
        for id in [0x44, 0xc, 0x52, 0x63] {
            b.toggle(id);
        }
        let shown = b.run(GETTH_DISP_ENABLE, &[]);
        b.value(0xc, 1u32.wrapping_sub(shown));
        // SYS
        b.page(0x0132_40a0);
        for (id, text) in [
            (0xf, 0x00b5_b29c),
            (0x16, 0x00b5_b2a4),
            (5, 0x00b5_b2ac),
            (0x38, 0x00b5_b2b4),
            (0x5b, 0x00b5_b2bc),
            (0x53, 0x00b5_b2c4),
            (0x33, 0x00b5_b2d0),
            (0x3d, 0x00b5_b2d8),
        ] {
            b.button(id, text);
        }
        b.rows(3);
        for (id, text) in [
            (0x43, 0x00b5_b2e4),
            (0x66, 0x00b5_b2ec),
            (0x35, 0x00b5_b2f4),
        ] {
            b.button(id, text);
        }
        b.rows(2);
        b.button(0x67, 0x00b5_b2fc);
        b.rows(2);
        b.button(0x25, 0x00b5_b304);
        b.toggle(0x66);
        b.close(0x00b5_b30c, 0x00b3_f8dc);
        // BTL
        b.page(0x0132_3e80);
        for (id, text) in [
            (0x1f, 0x00b5_b314),
            (0x23, 0x00b5_b31c),
            (0x20, 0x00b5_b324),
            (0x34, 0x00b5_b328),
            (0x58, 0x00b5_b330),
            (0x59, 0x00b5_b338),
            (0x42, 0x00b5_ae08),
        ] {
            b.button(id, text);
        }
        b.close(0x00b5_b340, 0x00b5_b348);
        // CAM
        b.page(0x0132_3f08);
        for (id, text) in [
            (0x1d, 0x00b5_b314),
            (0x1e, 0x00b5_b350),
            (0x24, 0x00b5_b354),
            (0x39, 0x00b5_b358),
            (0x50, 0x00b5_b360),
            (0x57, 0x00b5_b368),
        ] {
            b.button(id, text);
        }
        b.close(0x00b5_b370, 0x00b5_b378);
        // EFF
        b.page(0x0132_4018);
        b.button(0x5c, 0x00b5_b380);
        b.rows(2);
        for (id, text) in [
            (0x5d, 0x00b5_b388),
            (0x5e, 0x00b5_ae48),
            (0x5f, 0x00b5_b390),
            (0x60, 0x00b5_b398),
            (0x61, 0x00b5_b2b4),
            (0x62, 0x00b5_b3a0),
            (0x2a, 0x00b5_b3a8),
        ] {
            b.button(id, text);
        }
        b.close(0x00b5_b3b0, 0x00b5_b3b8);
        // EVENT
        b.page(0x0132_3f90);
        b.button(0x2e, 0x00b5_abe4);
        let name = b.run(GET_RESOURCE_STR, &[3]);
        b.run(SG_ADD_STATIC, &[w, 0x2d, name]);
        for (id, text) in [
            (0x2c, 0x00b4_02ac),
            (0x41, 0x00b5_ae08),
            (0x2f, 0x00b5_b3c0),
            (0x32, 0x00b5_b3c8),
            (0x37, 0x00b5_b3d0),
            (0x3c, 0x00b5_b3d8),
            (0x3b, 0x00b5_b3e0),
            (0x51, 0x00b5_b3e4),
            (0x55, 0x00b5_b3ec),
            (0x15, 0x00b5_b3f8),
            (0x6a, 0x00b5_b400),
            (0x6b, 0x00b5_ac20),
        ] {
            b.button(id, text);
        }
        b.toggle(0x6a);
        b.toggle(0x6b);
        b.value(0x6a, 0);
        b.value(0x6b, 1);
        b.close(0x00b5_b408, 0x00b5_54d4);
        let page = b.run(GET_RESOURCE, &[0, 0]);
        b.run(SELECT_MAIN_MENU, &[w, page]);
        b.run(EVE_INIT, &[]);
        for id in [0x67, 0x57, 0x4b] {
            b.toggle(id);
        }
        let lr = b.run(LR_BUTTON_MODE, &[]);
        b.value(0x4b, lr);
        b.value(0x57, 1);
        b.toggle(0x5c);
        let m = b.g(0x00b5_b410);
        b.run(DBG_PRINTF, &[m]);
        let off = b.run(GET_RESOURCE, &[0x12, 0]);
        b.value(0x5c, 1u32.wrapping_sub(off));
        b.run(WMAIN_PROC, &[w, 0x8006, 0x5c]);
    }

    /// `sprintf(0x023c4000, fmt, x)` and the text set on control `id`.
    fn set_number(h: &mut impl Host, w: u32, ids: &[u32], fmt: u32, x: u32) {
        let line = h.rebase(LINE);
        let f = h.rebase(fmt);
        let sprintf: u32 = h.read(h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[line, f, x]);
        for &id in ids {
            cdecl(h, SET_TEXT, &[w, id, line]); // call width: 3
        }
    }

    /// `1 - get()` given to `set`, and `report(get())` after when given.
    fn flip(h: &mut impl Host, get: u32, set: u32) {
        let v = cdecl(h, get, &[]);
        cdecl(h, set, &[1u32.wrapping_sub(v)]);
    }

    /// A map list selector: a new selector filled by `fill` (its count
    /// at `count`), opened on the current name (`which`) with `tag`, or
    /// "No items found."
    fn open_list(h: &mut impl Host, w: u32, st: u32, fill: u32, count: u32, which: u32, tag: u32) {
        let sel = cdecl(h, SELECTOR_ALLOCATE, &[]);
        h.write(at(st, 4), sel);
        let c = h.rebase(count);
        cdecl(h, fill, &[c, sel]);
        if h.read::<i32>(c) > 0 {
            let name = cdecl(h, MAP_GET_NAME, &[which]);
            let sel: u32 = h.read(at(st, 4));
            cdecl(h, SET_SELECT_STR, &[sel, name]);
            let sel: u32 = h.read(at(st, 4));
            cdecl(h, SELECTOR_OPEN, &[sel, w, tag]);
        } else {
            let m = h.rebase(0x00b5_accc);
            cdecl(h, MESSAGE_BOX, &[m]);
        }
    }

    /// A button (0x8006): the character and debug menu commands first,
    /// then the window's own.
    #[allow(clippy::too_many_lines)] // the original's jump table
    fn command(h: &mut impl Host, w: u32, id: u32, st: u32) {
        cdecl(h, COMMAND_CHR, &[w, id, st]);
        cdecl(h, COMMAND_MENU, &[w, id, st]);
        let call0 = |h: &mut _, f: u32| {
            cdecl(h, f, &[]);
        };
        match id {
            5 => {
                let p = h.rebase(FUKA_PROC);
                cdecl(h, WINDOW_CREATE, &[0x1b0, 0xa4, 0x4e, 0x3a, 6, p]);
            }
            8 => flip(h, TEX_GET, TEX_SET),
            9 => open_list(h, w, st, FILE_LIST, FILE_COUNT, 1, 0x16),
            0xb | 0xc | 0x5e => {
                let v = cdecl(h, TOGGLE_RESOURCE9, &[]);
                for c in [0xb, 0xc, 0x5e] {
                    cdecl(h, TOGGLE_VALUE, &[w, c, v]);
                }
            }
            0xd | 0x60 => {
                let v = cdecl(h, SWITCH_DRAW_ID, &[]);
                set_number(h, w, &[0xd, 0x60], 0x00b5_b434, v);
            }
            0xe => call0(h, 0x0085_1e10),
            0xf => call0(h, 0x0085_3780),
            0x11 => call0(h, 0x0084_e8c0),
            0x15 => call0(h, 0x0094_1830),
            0x16 => {
                let v = cdecl(h, HIDE, &[0]);
                cdecl(h, HIDE, &[v ^ 1]);
            }
            0x19 => {
                cdecl(h, 0x0084_efd0, &[w]);
            }
            0x1b => open_list(h, w, st, DIR_LIST, DIR_COUNT, 0, 0x1b),
            0x1d => {
                cdecl(h, MASAKI_INIT, &[0]);
            }
            0x1e => {
                cdecl(h, MASAKI_INIT, &[2]);
            }
            0x1f => {
                cdecl(h, MASAKI_INIT, &[1]);
            }
            0x20 => {
                cdecl(h, MASAKI_INIT, &[5]);
            }
            0x23 => {
                cdecl(h, MASAKI_INIT, &[3]);
            }
            0x24 => {
                cdecl(h, MASAKI_INIT, &[4]);
            }
            0x25 => call0(h, 0x0085_3ea0),
            0x27 => {
                let d = cdecl(h, MAP_GET_DIR, &[]).wrapping_add(1).cast_signed() % 3;
                let d = d.cast_unsigned();
                cdecl(h, MAP_SET_DIR, &[d]);
                cdecl(h, SET_RESOURCE, &[1, d]);
                cdecl(h, MAPNAME, &[w]);
                cdecl(h, CONTROL_DISABLE, &[w, 0x52, u32::from(d != 1)]);
            }
            0x29 => call0(h, MAP_INIT),
            0x2a => call0(h, 0x0082_0480),
            0x2b | 0x5f => {
                cdecl(h, 0x0093_0d80, &[0]);
            }
            0x33 => {
                cdecl(h, 0x0083_bb50, &[0]);
            }
            0x34 => {
                cdecl(h, MASAKI_INIT, &[6]);
            }
            0x35 => call0(h, 0x0084_e870),
            0x38 | 0x61 => call0(h, 0x0085_5430),
            0x39 => {
                cdecl(h, 0x0094_2110, &[0]);
            }
            0x3a => call0(h, 0x0084_4cb0),
            0x3d => {
                cdecl(h, 0x0083_bb50, &[1]);
            }
            0x40..=0x42 => call0(h, 0x0085_37b0),
            0x44 => {
                call0(h, 0x0082_1e30);
                let v = cdecl(h, 0x0082_0570, &[]);
                cdecl(h, TOGGLE_VALUE, &[w, 0x44, 1u32.wrapping_sub(v)]);
            }
            0x49 => {
                let v = cdecl(h, 0x0082_d810, &[]);
                cdecl(h, 0x0082_da60, &[v.wrapping_add(1)]);
            }
            0x4a => {
                let v = cdecl(h, 0x0082_d810, &[]);
                cdecl(h, 0x0082_da60, &[v.wrapping_sub(1)]);
            }
            0x4b => {
                let v = cdecl(h, LR_BUTTON_MODE, &[]);
                cdecl(h, 0x0082_daa0, &[v.wrapping_add(1)]);
            }
            0x4c | 0x4d => call0(h, 0x0084_e7c0),
            0x4f => call0(h, 0x0090_8f90),
            0x50 => {
                cdecl(h, MASAKI_INIT, &[9]);
            }
            0x52 => {
                let v = 1u32.wrapping_sub(cdecl(h, 0x0084_4ce0, &[]));
                cdecl(h, 0x0084_4fa0, &[v]);
                cdecl(h, TOGGLE_VALUE, &[w, 0x52, v]);
            }
            0x53 => call0(h, 0x0084_0e40),
            0x56 => {
                flip(h, 0x0083_17d0, 0x0083_1d00);
                let v = cdecl(h, 0x0083_17d0, &[]);
                set_number(h, w, &[0x56], 0x00b5_b440, v);
            }
            0x57 => {
                flip(h, 0x0082_0490, 0x0082_03a0);
                let v = cdecl(h, 0x0082_0490, &[]);
                cdecl(h, TOGGLE_VALUE, &[w, 0x57, v]);
            }
            0x58 => {
                cdecl(h, MASAKI_INIT, &[7]);
            }
            0x59 => {
                cdecl(h, MASAKI_INIT, &[8]);
            }
            0x5b => call0(h, 0x0085_4fd0),
            0x5c => {
                let v = 1u32.wrapping_sub(cdecl(h, GET_TOGGLE_VALUE, &[w, 0x5c]));
                cdecl(h, TOGGLE_VALUE, &[w, 0x5c, v]);
                let (m, f) = if v == 1 {
                    (0x00b5_b470, 0x007f_b940)
                } else {
                    (0x00b5_b44c, 0x007f_bc20)
                };
                let m = h.rebase(m);
                cdecl(h, DBG_PRINTF, &[m]);
                call0(h, f);
                cdecl(h, SET_RESOURCE, &[0x12, v]);
            }
            0x62 => call0(h, 0x0091_9b80),
            0x63 => flip(h, 0x0082_05d0, 0x0082_1b00),
            0x66 => {
                flip(h, 0x0083_3f50, 0x0083_57c0);
                let v = cdecl(h, 0x0083_3f50, &[]);
                cdecl(h, TOGGLE_VALUE, &[w, 0x66, v]);
            }
            0x67 => flip(h, 0x0082_05e0, 0x0082_1cb0),
            _ => {}
        }
    }

    /// The status refresh (0x8011): the clock title, the counters and
    /// the toggles' states.
    fn refresh(h: &mut impl Host, w: u32) {
        let clock = h.rebase(CLOCK_FLAGS);
        let blink = if h.read::<u8>(clock) & 1 != 0 {
            0x20
        } else {
            0x3a
        };
        let (hours, minutes): (u32, u32) = (h.read(at(clock, 8)), h.read(at(clock, 4)));
        let line = h.rebase(LINE);
        let f = h.rebase(0x00b5_b490);
        let sprintf: u32 = h.read(h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[line, f, hours, blink, minutes]);
        cdecl(h, SET_TITLE, &[w, line]); // call width: 2
        let v = cdecl(h, 0x0082_d7e0, &[]);
        set_number(h, w, &[0x48], 0x00b0_f604, v);
        let v = cdecl(h, 0x0082_05a0, &[]);
        set_number(h, w, &[0x55], 0x00b5_b4a0, v);
        let v = cdecl(h, 0x0082_05d0, &[]);
        cdecl(h, TOGGLE_VALUE, &[w, 0x63, v]);
        let v = cdecl(h, LR_BUTTON_MODE, &[]);
        cdecl(h, TOGGLE_VALUE, &[w, 0x4b, v]);
        let text = if cdecl(h, LR_BUTTON_MODE, &[]) == 0 {
            0x00b5_5224
        } else {
            0x00b5_b24c
        };
        let t = h.rebase(text);
        cdecl(h, SET_TEXT, &[w, 0x4b, t]);
        let v = cdecl(h, 0x0082_05e0, &[]);
        cdecl(h, TOGGLE_VALUE, &[w, 0x67, v]);
    }

    /// A selector's choice (0x8012): the character and event menus first,
    /// then the map lists (0x16 the file, 0x17 nothing more, 0x1b the
    /// directory).
    fn selector_done(h: &mut impl Host, w: u32, id: u32, st: u32) {
        cdecl(h, CHR_SELECTOR, &[w, id, st]);
        cdecl(h, EVE_SELECTOR, &[w, id]);
        let sel: u32 = h.read(at(st, 4));
        match id {
            0x16 => {
                let name = cdecl(h, SELECTOR_ITEM_NAME, &[sel]);
                cdecl(h, MAP_SET_NAME, &[name, 1]);
                cdecl(h, MAP_INIT, &[]);
                cdecl(h, MAPNAME, &[w]);
                let sel: u32 = h.read(at(st, 4));
                cdecl(h, SET_MOUSE_FUKKI, &[sel, 0]);
            }
            0x17 => {
                cdecl(h, SELECTOR_ITEM_NAME, &[sel]);
            }
            0x1b => {
                let name = cdecl(h, SELECTOR_ITEM_NAME, &[sel]);
                cdecl(h, MAP_SET_NAME, &[name, 0]);
                cdecl(h, MAPNAME, &[w]);
            }
            _ => {}
        }
    }
}
