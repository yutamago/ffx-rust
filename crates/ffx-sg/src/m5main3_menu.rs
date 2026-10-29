//! M5 `main`, round 3: the PS2 debug menu's windows (`SgGu_*`, the main
//! menu `WMainProc` and its pages, the character and shadow pages).
//!
//! The light editor's table is [`crate::m5main3_misc::bridge::LIGHTS`]:
//! six entries (0..=2 lights 1 to 3, 3 ambient, 4 the background colour) of
//! 0x1c bytes; the byte `0x01303c18 + index` says the entry's window is
//! open, `0x01303cc8` points at the entry a new window edits.

/// The originals over game memory.
#[allow(clippy::cast_possible_truncation, clippy::many_single_char_names)]
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `Gu_WindowCreate(x, y, w, h, flags, callback)`,
    /// `Gu_WindowSetTitle(window, text)`, `Gu_WindowSetArg(window, arg)`,
    /// `yiGetBackColor(out)` (writes four words).
    pub const WINDOW_CREATE: u32 = 0x0084_a920;
    pub const WINDOW_SET_TITLE: u32 = 0x0084_b6a0;
    pub const WINDOW_SET_ARG: u32 = 0x0084_b560;
    pub const GET_BACK_COLOR: u32 = 0x0090_86d0;

    /// The open bytes, one per light table entry.
    pub const LIGHT_OPEN: u32 = 0x0130_3c18;
    /// The entry the next editor window edits.
    pub const LIGHT_CURRENT: u32 = 0x0130_3cc8;
    /// The background colour's entry (index 4).
    pub const BG_ENTRY: u32 = 0x0130_3c90;
    /// The colour editor's and the light editor's window procedures, and
    /// the save bar's (`0x0084e590`).
    pub const COLOUR_PROC: u32 = 0x0084_e100;
    pub const LIGHT_PROC: u32 = 0x0084_dac0;
    pub const SAVE_PROC: u32 = 0x0084_e590;
    /// `"BGColor"`.
    pub const BG_TITLE: u32 = 0x00b2_3e9c;

    /// `SgGu_BgColor()` (`0x0084e7c0`, cdecl): unless its window is open
    /// (byte `0x01303c1c`), a colour editor window on the background
    /// colour's entry (`"BGColor"`, the entry as its argument), the entry's
    /// index set to 4 and its colour from `yiGetBackColor` (alpha -1),
    /// then its open byte set. `buf` is the original's 16-byte frame
    /// buffer (`ebp - 0x14`) the colour goes through.
    pub fn bg_color(h: &mut impl Host, buf: u32) {
        if h.read::<u8>(h.rebase(0x0130_3c1c)) != 0 {
            return;
        }
        let entry = h.rebase(BG_ENTRY);
        h.write(h.rebase(LIGHT_CURRENT), entry);
        let proc_ = h.rebase(COLOUR_PROC);
        let w = cdecl(h, WINDOW_CREATE, &[5, 5, 100, 0x46, 6, proc_]);
        let title = h.rebase(BG_TITLE);
        cdecl(h, WINDOW_SET_TITLE, &[w, title]);
        cdecl(h, WINDOW_SET_ARG, &[w, entry]);
        h.write(at(entry, 0x18), 4u16);
        cdecl(h, GET_BACK_COLOR, &[buf]);
        let rgb: [u32; 3] = h.read(buf);
        h.write(entry, rgb);
        h.write(at(entry, 0xc), u32::MAX);
        let i = i32::from(h.read::<i16>(at(entry, 0x18)));
        h.write(at(h.rebase(LIGHT_OPEN), i.cast_unsigned()), 1u8);
    }

    /// `SgGu_LightEdit()` (`0x0084e8c0`, cdecl): opens what is not open
    /// yet: the save bar (byte `0x01303c1d`), the ambient colour editor
    /// (entry 3) and the editors of lights 3, 2 and 1 (entries 2, 1, 0);
    /// each editor's entry becomes the current one and its open byte is
    /// set first (at the entry's own index `+0x18`).
    pub fn light_edit(h: &mut impl Host) {
        if h.read::<u8>(h.rebase(0x0130_3c1d)) == 0 {
            let p = h.rebase(SAVE_PROC);
            cdecl(h, WINDOW_CREATE, &[0x128, 0, 0x88, 0x22, 6, p]);
        }
        // (open byte, entry, height, procedure, title)
        let editors = [
            (0x0130_3c1b, 0x0130_3c74, 0x46, COLOUR_PROC, 0x00b5_af04),
            (0x0130_3c1a, 0x0130_3c58, 0xad, LIGHT_PROC, 0x00b5_af0c),
            (0x0130_3c19, 0x0130_3c3c, 0xad, LIGHT_PROC, 0x00b5_af14),
            (0x0130_3c18, 0x0130_3c20, 0xad, LIGHT_PROC, 0x00b5_af1c),
        ];
        for (open, entry, height, proc_, title) in editors {
            if h.read::<u8>(h.rebase(open)) != 0 {
                continue;
            }
            let entry = h.rebase(entry);
            let i = i32::from(h.read::<i16>(at(entry, 0x18)));
            h.write(h.rebase(LIGHT_CURRENT), entry);
            h.write(at(h.rebase(LIGHT_OPEN), i.cast_unsigned()), 1u8);
            let p = h.rebase(proc_);
            let w = cdecl(h, WINDOW_CREATE, &[5, 5, 100, height, 6, p]);
            let t = h.rebase(title);
            cdecl(h, WINDOW_SET_TITLE, &[w, t]);
        }
    }

    /// `Gu_WindowAddButton(window, id, text, x, y, w, h)`,
    /// `Gu_ButtonSetToggle(window, id, on)`, `Gu_ButtonSetToggleValue
    /// (window, id, value)` (cdecl).
    pub const WINDOW_ADD_BUTTON: u32 = 0x0084_a470;
    pub const BUTTON_SET_TOGGLE: u32 = 0x0084_9380;
    pub const BUTTON_SET_TOGGLE_VALUE: u32 = 0x0084_93e0;
    /// `sprintf`'s import slot.
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    /// `"%d"`, `"LightStock"` and the three buttons' labels (ids 9, 8,
    /// 10).
    pub const FMT_D: u32 = 0x00b0_f604;
    pub const STOCK_TITLE: u32 = 0x00b5_aeb0;
    pub const STOCK_LABELS: [u32; 3] = [0x00b5_aebc, 0x00b5_aec0, 0x00b5_aec8];
    /// The light table's seven stocks (0xa8 bytes each, from
    /// `0x01303780`) and the stock the buttons pick (`i8`).
    pub const STOCKS: u32 = 0x0130_3780;
    pub const STOCK: u32 = 0x0130_3778;
    /// `FUN_0084e3c0(stock, apply)` (loads a stock into the table) and
    /// `Gui_SettingSaveLightFile_00853310()`.
    pub const STOCK_LOAD: u32 = 0x0084_e3c0;
    pub const SAVE_LIGHT_FILE: u32 = 0x0085_3310;

    /// The light table (six entries, 0xa8 bytes) copied into stock `k`.
    fn stock_store(h: &mut impl Host, k: i32) {
        let table = h.rebase(crate::m5main3_misc::bridge::LIGHTS);
        let dst = at(h.rebase(STOCKS), k.wrapping_mul(0xa8).cast_unsigned());
        let mut row = [0u8; 0xa8];
        h.read_bytes(table, &mut row);
        h.write_bytes(dst, &row);
    }

    /// `FUN_0084e590(window, msg, id; slots)` (cdecl, the save bar's
    /// window procedure, `SAVE_PROC`). Create (`0x8001`): its open byte
    /// set, titled `"LightStock"`, seven toggle buttons `"0"`..`"6"` (the
    /// number printed into the caller's `msg` argument word, `slots +
    /// 4`, whose address the button gets), three more buttons (ids 9, 8,
    /// 10), the table in stock 0 and button 0 on. Close (`0x8003`): the
    /// open byte cleared. Release (`0x8007`): ids 0..=6 pick the stock,
    /// 8 stores the table in it, 9 loads it (`FUN_0084e3c0(stock, 1)`),
    /// 10 saves the light file; then the seven buttons show the stock.
    /// Returns 0.
    pub fn save_proc(h: &mut impl Host, window: u32, msg: u32, id: u32, slots: u32) -> u32 {
        let stock_at = h.rebase(STOCK);
        match msg {
            0x8001 => {
                h.write(h.rebase(0x0130_3c1d), 1u8);
                let t = h.rebase(STOCK_TITLE);
                cdecl(h, WINDOW_SET_TITLE, &[window, t]);
                let text = at(slots, 4);
                let fmt = h.rebase(FMT_D);
                for i in 0..7u32 {
                    let f: u32 = h.read(h.rebase(SPRINTF));
                    h.call(f, Conv::Cdecl, &[text, fmt, i]);
                    cdecl(
                        h,
                        WINDOW_ADD_BUTTON,
                        &[window, i, text, i * 0x12, 0, 0x10, 9],
                    );
                    cdecl(h, BUTTON_SET_TOGGLE, &[window, i, 1]);
                }
                for (id, x, w, label) in [(9, 0, 0x24, 0), (8, 0x24, 0x28, 1), (10, 0x4c, 0x28, 2)]
                {
                    let l = h.rebase(STOCK_LABELS[label]);
                    cdecl(h, WINDOW_ADD_BUTTON, &[window, id, l, x, 0xb, w, 9]);
                }
                stock_store(h, 0);
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, 0, 1]);
            }
            0x8003 => h.write(h.rebase(0x0130_3c1d), 0u8),
            0x8007 => {
                if id <= 6 {
                    #[allow(clippy::cast_possible_truncation)] // `mov [..], al`
                    h.write(stock_at, id as u8);
                }
                let stock = i32::from(h.read::<i8>(stock_at));
                match id {
                    8 => stock_store(h, stock),
                    9 => {
                        cdecl(h, STOCK_LOAD, &[stock.cast_unsigned(), 1]);
                    }
                    10 => {
                        cdecl(h, SAVE_LIGHT_FILE, &[]);
                    }
                    _ => {}
                }
                for i in 0..7 {
                    let on = i == i32::from(h.read::<i8>(stock_at));
                    cdecl(
                        h,
                        BUTTON_SET_TOGGLE_VALUE,
                        &[window, i.cast_unsigned(), u32::from(on)],
                    );
                }
            }
            _ => {}
        }
        0
    }
    /// `Gu_WindowGetLeft`, `Gu_WindowGetX`, `Gu_WindowGetWidth (window)`;
    /// the selector calls `Gu_SelectorAllocate()`, `ResetNbItem(sel, n)`,
    /// `SetItem(sel, i, text, value)`, `SetSelect(sel, i)`,
    /// `SetCallback(sel, f)`, `OpenXYWH(sel, window, id, x, y, w, h)`,
    /// `GetItemValue(sel)`; `Gu_WindowGetArg(window)`,
    /// `Gu_WindowSetWord30(window, v)`, `Gu_WindowControlSetHide(window,
    /// id, hide)`, `Gu_WindowSetClientH(window, h)`,
    /// `Gu_WindowControlSetText(window, id, text)` (all cdecl).
    pub const WINDOW_GET_LEFT: u32 = 0x0084_afb0;
    pub const WINDOW_GET_X: u32 = 0x0084_afc0;
    pub const WINDOW_GET_WIDTH: u32 = 0x0084_afa0;
    pub const SELECTOR_ALLOCATE: u32 = 0x0084_9bb0;
    pub const SELECTOR_RESET_NB_ITEM: u32 = 0x0084_9f40;
    pub const SELECTOR_SET_ITEM: u32 = 0x0084_9fd0;
    pub const SELECTOR_SET_SELECT: u32 = 0x0084_a0e0;
    pub const SELECTOR_SET_CALLBACK: u32 = 0x0084_9fc0;
    pub const SELECTOR_OPEN_XYWH: u32 = 0x0084_9dc0;
    pub const SELECTOR_GET_ITEM_VALUE: u32 = 0x0084_9d10;
    pub const WINDOW_GET_ARG: u32 = 0x0084_aec0;
    pub const WINDOW_SET_WORD30: u32 = 0x0084_b7e0;
    pub const CONTROL_SET_HIDE: u32 = 0x0084_a670;
    pub const WINDOW_SET_CLIENT_H: u32 = 0x0084_b580;
    pub const CONTROL_SET_TEXT: u32 = 0x0084_a710;
    /// `Sg_SetResource(i, v)`.
    pub const SET_RESOURCE: u32 = 0x0083_fe10;

    /// The main menu's eight pages (0x88 bytes from `0x01323d70`: a count
    /// of control ids, the ids, `+0x7c` the client height, `+0x80` the
    /// title text, `+0x84` the page's name for the selector), the page
    /// shown, the selected page and the menu's window.
    pub const PAGES: u32 = 0x0132_3d70;
    pub const PAGE: u32 = 0x0132_3d6c;
    pub const PAGE_SELECT: u32 = 0x0132_4128;
    pub const MENU_WINDOW: u32 = 0x0132_4130;
    /// The selector's callback, `FUN_0084f230`.
    pub const PAGE_CALLBACK: u32 = 0x0084_f230;

    /// `FUN_0084efd0(window)` (cdecl): the page selector pops up beside
    /// the main menu: at its left edge minus 70 when that edge is at
    /// least 70, else right of it; seven items (the page names at `+0x84`
    /// of each page, values k), the shown page selected, `FUN_0084f230` as
    /// its callback, 70 x 72 at the window's y. Returns
    /// `Gu_SelectorOpenXYWH`'s `eax`.
    pub fn page_popup(h: &mut impl Host, window: u32) -> u32 {
        let left = cdecl(h, WINDOW_GET_LEFT, &[window]);
        let y = cdecl(h, WINDOW_GET_X, &[window]);
        let x = if left.cast_signed() < 0x46 {
            left.wrapping_add(cdecl(h, WINDOW_GET_WIDTH, &[window]))
        } else {
            left.wrapping_sub(0x46)
        };
        let sel = cdecl(h, SELECTOR_ALLOCATE, &[]);
        cdecl(h, SELECTOR_RESET_NB_ITEM, &[sel, 7]);
        let names = at(h.rebase(PAGES), 0x84);
        for i in 0..7u32 {
            let text: u32 = h.read(at(names, 0x88 * i));
            cdecl(h, SELECTOR_SET_ITEM, &[sel, i, text, i]);
        }
        let cur: u32 = h.read(h.rebase(PAGE_SELECT));
        cdecl(h, SELECTOR_SET_SELECT, &[sel, cur]);
        let cb = h.rebase(PAGE_CALLBACK);
        cdecl(h, SELECTOR_SET_CALLBACK, &[sel, cb]);
        cdecl(
            h,
            SELECTOR_OPEN_XYWH,
            &[sel, window, 0xffff_fffe, x, y, 0x46, 0x48],
        )
    }

    /// `Sg_SelectMainMenu(window, page)` (`0x0084f270`, cdecl): shows page
    /// `page` (0 outside 0..=7) of the main menu: resource 0 = page, the
    /// shown page's controls hidden, the new page's shown, the client
    /// height and the title control `0x19` from the page. Returns the
    /// last call's `eax`.
    pub fn select_main_menu(h: &mut impl Host, window: u32, page: u32) -> u32 {
        h.write(h.rebase(MENU_WINDOW), window);
        let p = page.cast_signed();
        let page = if (0..=7).contains(&p) { page } else { 0 };
        h.write(h.rebase(PAGE_SELECT), page);
        cdecl(h, SET_RESOURCE, &[0, page]);
        let cur_at = h.rebase(PAGE);
        for (hide, next) in [(1, None), (0, Some(page))] {
            if let Some(p) = next {
                let to = at(h.rebase(PAGES), p * 0x88);
                h.write(cur_at, to);
            }
            let mut i = 0i32;
            loop {
                let cur: u32 = h.read(cur_at);
                if i >= h.read::<i32>(cur) {
                    break;
                }
                let id: u32 = h.read(at(cur, 4 + 4 * i.cast_unsigned()));
                cdecl(h, CONTROL_SET_HIDE, &[window, id, hide]);
                i += 1;
            }
        }
        let cur: u32 = h.read(cur_at);
        let hh: u32 = h.read(at(cur, 0x7c));
        cdecl(h, WINDOW_SET_CLIENT_H, &[window, hh]);
        let cur: u32 = h.read(cur_at);
        let text: u32 = h.read(at(cur, 0x80));
        cdecl(h, CONTROL_SET_TEXT, &[window, 0x19, text])
    }

    /// `Sg_SelectMainMenu`.
    pub const SELECT_MAIN_MENU: u32 = 0x0084_f270;

    /// `FUN_0084f230(window, msg)` (cdecl, the page selector's callback):
    /// on `0x8011` (picked) shows the picked item's page in the main menu
    /// window and sets the selector window's word `+0x30` to 1. Returns 0.
    pub fn page_picked(h: &mut impl Host, window: u32, msg: u32) -> u32 {
        if msg == 0x8011 {
            let sel = cdecl(h, WINDOW_GET_ARG, &[window]);
            let page = cdecl(h, SELECTOR_GET_ITEM_VALUE, &[sel]);
            let menu: u32 = h.read(h.rebase(MENU_WINDOW));
            cdecl(h, SELECT_MAIN_MENU, &[menu, page]);
            cdecl(h, WINDOW_SET_WORD30, &[window, 1]);
        }
        0
    }

    /// `Gu_PrintSetItalic(on)`, `Gu_PrintSetBold(on)`, `Ch_GetFuka(i)`,
    /// `Gu_Printf(fmt, ...)`; the load window's title and its five
    /// formats ("ANM:%d\n", "ELM", "ENV", "DRW", "ALL").
    pub const PRINT_SET_ITALIC: u32 = 0x0084_8760;
    pub const PRINT_SET_BOLD: u32 = 0x0084_8720;
    pub const GET_FUKA: u32 = 0x0083_3f60;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    pub const FUKA_TITLE: u32 = 0x00b5_b1dc;
    pub const FUKA_FORMATS: [u32; 5] = [
        0x00b5_b1e4,
        0x00b5_b1ec,
        0x00b5_b1f4,
        0x00b5_b1fc,
        0x00b5_b204,
    ];

    /// `WFukaProc(window, msg)` (`0x0084f490`, cdecl): the character
    /// load window: titled on create (`0x8001`); on paint (`0x8011`) in
    /// bold italics the four loads `Ch_GetFuka(1..=4)` and their sum
    /// (fetched again, 4 down to 1). Returns 0.
    pub fn fuka_proc(h: &mut impl Host, window: u32, msg: u32) -> u32 {
        match msg {
            0x8001 => {
                let t = h.rebase(FUKA_TITLE);
                cdecl(h, WINDOW_SET_TITLE, &[window, t]);
            }
            0x8011 => {
                cdecl(h, PRINT_SET_ITALIC, &[1]);
                cdecl(h, PRINT_SET_BOLD, &[1]);
                for (i, fmt) in (1..=4u32).zip(FUKA_FORMATS) {
                    let v = cdecl(h, GET_FUKA, &[i]);
                    let f = h.rebase(fmt);
                    cdecl(h, GU_PRINTF, &[f, v]);
                }
                let mut sum = 0u32;
                for i in (1..=4u32).rev() {
                    sum = sum.wrapping_add(cdecl(h, GET_FUKA, &[i]));
                }
                let f = h.rebase(FUKA_FORMATS[4]);
                cdecl(h, GU_PRINTF, &[f, sum]);
            }
            _ => {}
        }
        0
    }

    /// `Sg_MapGetDirSelect()`, `Sg_MapGetName(which)`,
    /// `sg_make_filelist(list, command, 0)`; `"sugilist.tmp"` and the
    /// host `dir` commands per map directory (master, sugi, btlmaster):
    /// of one map's files and of the directory.
    pub const MAP_GET_DIR_SELECT: u32 = 0x0084_4cd0;
    pub const MAP_GET_NAME: u32 = 0x0084_4cf0;
    pub const MAKE_FILELIST: u32 = 0x0085_09e0;
    pub const LIST_FILE: u32 = 0x00b5_aadc;
    pub const DIR_MAP_FILES: [u32; 3] = [0x00b5_b124, 0x00b5_b0e8, 0x00b5_b15c];
    pub const DIR_MAPS: [u32; 3] = [0x00b5_b040, 0x00b5_b078, 0x00b5_b0b0];
    /// The size of their command buffer (`ebp - 0x84`).
    pub const COMMAND: u32 = 0x80;

    /// `FUN_008506a0(list, sel)` (`of_map`) and `FUN_00850770(list, sel)`
    /// (cdecl): the host's file list of the current map's directory
    /// (`Sg_MapGetName(0)`) or of the map directory, by
    /// `Sg_MapGetDirSelect` (0 master, 1 sugi, 2 btlmaster), through
    /// `sg_make_filelist(list, command, 0)`; the selector `sel` gets one
    /// item per file (`list[2][i]`, value i). `buf` is the original's
    /// 0x80-byte command buffer (`ebp - 0x84`). Returns the last
    /// selector call's `eax`.
    ///
    /// Another directory value leaves the buffer unwritten:
    /// junk read: [ebp-0x84] never written; 0 (brief 2026-10-24): the
    /// shim's frame is zeroed, so the command is empty.
    pub fn file_list(h: &mut impl Host, list: u32, sel: u32, buf: u32, of_map: bool) -> u32 {
        let dir = cdecl(h, MAP_GET_DIR_SELECT, &[]);
        if dir <= 2 {
            let tmp = h.rebase(LIST_FILE);
            if of_map {
                let name = cdecl(h, MAP_GET_NAME, &[0]);
                let fmt = h.rebase(DIR_MAP_FILES[dir as usize]);
                let sprintf: u32 = h.read(h.rebase(SPRINTF));
                h.call(sprintf, Conv::Cdecl, &[buf, fmt, name, tmp]);
            } else {
                let fmt = h.rebase(DIR_MAPS[dir as usize]);
                let sprintf: u32 = h.read(h.rebase(SPRINTF));
                h.call(sprintf, Conv::Cdecl, &[buf, fmt, tmp]);
            }
        }
        cdecl(h, MAKE_FILELIST, &[list, buf, 0]);
        let n: u32 = h.read(list);
        let mut r = cdecl(h, SELECTOR_RESET_NB_ITEM, &[sel, n]);
        let mut i = 0i32;
        while i < h.read::<i32>(list) {
            let names: u32 = h.read(at(list, 8));
            let text: u32 = h.read(at(names, 4 * i.cast_unsigned()));
            let k = i.cast_unsigned();
            r = cdecl(h, SELECTOR_SET_ITEM, &[sel, k, text, k]);
            i += 1;
        }
        r
    }

    /// The main menu's map texts: the directory names (`0x00b5b228`
    /// "Master", `0x00b5b23c`, `0x00b5b244` "Battle"), and the labels of
    /// controls 0x1b and 9.
    pub const MAP_TEXTS: [u32; 5] = [
        0x00b5_b228,
        0x00b5_b23c,
        0x00b5_b244,
        0x00b5_b230,
        0x00b5_b234,
    ];
    pub const MAP_TEXT_SUGI: u32 = 0x00b5_adf0;

    /// `sg_mainmenu_mapname(window)` (`0x00850940`, cdecl): the main
    /// menu's map name (control 0x1a) and second name (10) from
    /// `Sg_MapGetName`, then by the map directory (0 master, 1 sugi, 2
    /// battle) the directory text (0x26) and the labels of 0x1b and 9.
    /// Returns the last call's `eax` (`dir - 2` for another directory).
    pub fn mainmenu_mapname(h: &mut impl Host, window: u32) -> u32 {
        let name = cdecl(h, MAP_GET_NAME, &[0]);
        cdecl(h, CONTROL_SET_TEXT, &[window, 0x1a, name]);
        let name = cdecl(h, MAP_GET_NAME, &[1]);
        cdecl(h, CONTROL_SET_TEXT, &[window, 0xa, name]);
        let dir = cdecl(h, MAP_GET_DIR_SELECT, &[]);
        let texts = match dir {
            0 => [MAP_TEXTS[0], MAP_TEXTS[3], MAP_TEXTS[4]],
            1 => [MAP_TEXTS[1], MAP_TEXT_SUGI, MAP_TEXTS[3]],
            2 => [MAP_TEXTS[2], MAP_TEXTS[3], MAP_TEXTS[4]],
            _ => return dir.wrapping_sub(2),
        };
        let mut r = 0;
        for (id, t) in [0x26, 0x1b, 9].into_iter().zip(texts) {
            let t = h.rebase(t);
            r = cdecl(h, CONTROL_SET_TEXT, &[window, id, t]);
        }
        r
    }
    /// `Gu_WindowAddStatic(window, id, text, x, y, w, h)` and
    /// `Gu_WindowControlSetTextf(window, id, fmt, ...)` (cdecl).
    pub const WINDOW_ADD_STATIC: u32 = 0x0084_a4d0;
    pub const CONTROL_SET_TEXTF: u32 = 0x0084_a810;
    /// The spin control's down and up arrows and its empty value text.
    pub const ARROW_DOWN: u32 = 0x00b5_adcc;
    pub const ARROW_UP: u32 = 0x00b5_add4;
    pub const EMPTY: u32 = 0x00b8_fbeb;

    /// A spin control's value text: `Gu_WindowControlSetTextf(window,
    /// +0x18, +0x1c, +0x20)` when it has a format; else 0 (`eax`).
    fn spin_text(h: &mut impl Host, window: u32, spin: u32) -> u32 {
        let fmt: u32 = h.read(at(spin, 0x1c));
        if fmt == 0 {
            return 0;
        }
        let id: u32 = h.read(at(spin, 0x18));
        let v: u32 = h.read(at(spin, 0x20));
        cdecl(h, CONTROL_SET_TEXTF, &[window, id, fmt, v])
    }

    /// `FUN_00851c70(window, spin, x, y, h, label, w, wv)` (cdecl): a spin
    /// control's row: the label (a static, id -1, `w` wide), the down
    /// arrow (`+0x14`), the value static (`+0x18`, `wv` wide), the up
    /// arrow (`+0x10`), all `h` high at `y`, then its value text. Returns
    /// [`spin_text`]'s `eax`.
    #[allow(clippy::too_many_arguments)]
    pub fn spin_add(
        h: &mut impl Host,
        window: u32,
        spin: u32,
        x: u32,
        y: u32,
        hh: u32,
        label: u32,
        w: u32,
        wv: u32,
    ) -> u32 {
        cdecl(
            h,
            WINDOW_ADD_STATIC,
            &[window, u32::MAX, label, x, y, w, hh],
        );
        let x = x.wrapping_add(w);
        let down: u32 = h.read(at(spin, 0x14));
        let t = h.rebase(ARROW_DOWN);
        cdecl(h, WINDOW_ADD_BUTTON, &[window, down, t, x, y, 0xc, hh]);
        let x = x.wrapping_add(0xc);
        let id: u32 = h.read(at(spin, 0x18));
        let t = h.rebase(EMPTY);
        cdecl(h, WINDOW_ADD_STATIC, &[window, id, t, x, y, wv, hh]);
        let up: u32 = h.read(at(spin, 0x10));
        let t = h.rebase(ARROW_UP);
        cdecl(
            h,
            WINDOW_ADD_BUTTON,
            &[window, up, t, x.wrapping_add(wv), y, 0xc, hh],
        );
        spin_text(h, window, spin)
    }

    /// `FUN_00851b20(window, msg, id, spin)` (cdecl): a spin control's
    /// arrows. Only for its up (`+0x10`, +1) and down (`+0x14`, -1)
    /// buttons; the repeat delay `+0x2c` counts down first. A press
    /// (`0x8005`), repeat (`0x8009`) or `0x800d` with no delay left steps
    /// by `+4`, `+0xc` or `+8` (added times the direction when the flag
    /// byte `+0`, `+2` or `+1` is set, else set to the step; a negative
    /// step jumps to the maximum `+0x24`, or the minimum `+0x28` going
    /// down); then the value `+0x20` wraps modulo the maximum (minimum -1)
    /// or is clamped to minimum..=maximum when minimum < maximum, and its
    /// text is set. Releases (`0x8006`, `0x800a`, `0x800e`) clear the
    /// delay, `0x8007`, `0x800b`, `0x800f` set it to 10. Returns what the
    /// original leaves in `eax`.
    pub fn spin_msg(h: &mut impl Host, window: u32, msg: u32, id: u32, spin: u32) -> u32 {
        let up: u32 = h.read(at(spin, 0x10));
        let down: u32 = h.read(at(spin, 0x14));
        if id != up && id != down {
            return id;
        }
        let dir: i32 = if id == down { -1 } else { 1 };
        let delay_at = at(spin, 0x2c);
        let delay: u32 = h.read(delay_at);
        if delay != 0 {
            h.write(delay_at, delay.wrapping_sub(1));
        }
        let k = msg.wrapping_sub(0x8005);
        // the jump table's index byte, left in `eax`
        let (mode, index) = match k {
            0 => (1, 0),
            1 | 5 | 9 => {
                h.write(delay_at, 0u32);
                return 1;
            }
            2 | 6 | 10 => {
                h.write(delay_at, 10u32);
                return 2;
            }
            4 => (2, 3),
            8 => (3, 4),
            3 | 7 => return 5,
            _ => return k,
        };
        if h.read::<u32>(delay_at) != 0 {
            return index;
        }
        let (flag, step) = match mode {
            1 => (h.read::<i8>(spin), h.read::<i32>(at(spin, 4))),
            2 => (h.read::<i8>(at(spin, 2)), h.read::<i32>(at(spin, 0xc))),
            _ => (h.read::<i8>(at(spin, 1)), h.read::<i32>(at(spin, 8))),
        };
        let value_at = at(spin, 0x20);
        if step < 0 {
            let v: u32 = h.read(at(spin, if dir < 0 { 0x28 } else { 0x24 }));
            h.write(value_at, v);
        } else if flag == 0 {
            h.write(value_at, step);
        } else {
            let v = h.read::<i32>(value_at).wrapping_add(step.wrapping_mul(dir));
            h.write(value_at, v);
        }
        let lo: i32 = h.read(at(spin, 0x28));
        if lo == -1 {
            let max: i32 = h.read(at(spin, 0x24));
            let v: i32 = h.read(value_at);
            // `idiv`: a zero maximum faults in the original
            let r = if max == 0 { 0 } else { v.wrapping_rem(max) };
            h.write(value_at, r);
            if r < 0 {
                h.write(value_at, r.wrapping_add(max));
            }
        } else {
            let hi: i32 = h.read(at(spin, 0x24));
            if lo < hi {
                if h.read::<i32>(value_at) < lo {
                    h.write(value_at, lo);
                }
                if h.read::<i32>(value_at) > hi {
                    h.write(value_at, hi);
                }
            }
        }
        spin_text(h, window, spin)
    }

    /// `g_DebugLine` (`0x023c4000`), the text buffer the debug windows
    /// `sprintf` into.
    pub const DEBUG_LINE: u32 = 0x023c_4000;
    /// `Ch_GuiSetTalk(i, open)`, `Ch_GuiGetMouthTh()`,
    /// `Ch_GuiSetMouthTh(th)` (cdecl).
    pub const GUI_SET_TALK: u32 = 0x0083_40d0;
    pub const GUI_GET_MOUTH_TH: u32 = 0x0083_4070;
    pub const GUI_SET_MOUTH_TH: u32 = 0x0083_40a0;
    /// The mouth window's open byte, its "open" counter (`i8`), title,
    /// formats and button labels.
    pub const MOUTH_OPEN: u32 = 0x0132_52c9;
    pub const MOUTH_COUNT: u32 = 0x0132_52ca;
    pub const MOUTH_TITLE: u32 = 0x00b5_ae58;
    pub const FMT_OPEN: u32 = 0x00b5_b50c;
    pub const FMT_TH: u32 = 0x00b5_b514;
    /// (id, text, x, y, w) of its buttons, then its static.
    pub const MOUTH_BUTTONS: [(u32, u32, u32, u32, u32); 6] = [
        (0, 0x00b4_88dc, 0, 0, 0x3c),
        (1, 0x00b5_b51c, 0, 0xa, 0x3c),
        (2, 0x00b5_b524, 0, 0x14, 0x3c),
        (3, 0x00b5_b52c, 0, 0x1e, 0x3c),
        (7, 0x00b5_5220, 0, 0x28, 0xa),
        (6, 0x00b7_c998, 0xa, 0x28, 0x28),
    ];
    pub const MOUTH_PLUS: (u32, u32, u32, u32, u32) = (5, 0x00b4_8250, 0x32, 0x28, 0xa);

    /// `sprintf(g_DebugLine, fmt, v)` and the control's text set to it.
    fn line_text(h: &mut impl Host, window: u32, id: u32, fmt: u32, v: u32) {
        let line = h.rebase(DEBUG_LINE);
        let fmt = h.rebase(fmt);
        let sprintf: u32 = h.read(h.rebase(SPRINTF));
        h.call(sprintf, Conv::Cdecl, &[line, fmt, v]);
        cdecl(h, CONTROL_SET_TEXT, &[window, id, line]);
    }

    /// The mouth threshold shown as `th / 10` in control 6.
    fn mouth_th_text(h: &mut impl Host, window: u32) {
        let th = cdecl(h, GUI_GET_MOUTH_TH, &[]).cast_signed() / 10;
        line_text(h, window, 6, FMT_TH, th.cast_unsigned());
    }

    /// `FUN_00851e70(window, msg, id)` (cdecl): the mouth window. Create
    /// (`0x8001`): "Mouth", four toggle buttons (0..=3), the threshold
    /// arrows 7 and 5 around its static 6 (`"th:%d"`, the threshold / 10).
    /// Close (`0x8003`): its open byte cleared. Release (`0x8007`): the
    /// picked toggle on (button 1 also steps the open counter 0..=3 and
    /// shows it), `Ch_GuiSetTalk(i, counter)` for it, the others off; 5
    /// and 7 move the threshold by 10 up or down. Returns 0.
    pub fn mouth_proc(h: &mut impl Host, window: u32, msg: u32, id: u32) -> u32 {
        match msg {
            0x8001 => {
                let t = h.rebase(MOUTH_TITLE);
                cdecl(h, WINDOW_SET_TITLE, &[window, t]);
                for (i, (bid, text, x, y, w)) in MOUTH_BUTTONS.into_iter().enumerate() {
                    let t = h.rebase(text);
                    let f = if i == 5 {
                        WINDOW_ADD_STATIC
                    } else {
                        WINDOW_ADD_BUTTON
                    };
                    cdecl(h, f, &[window, bid, t, x, y, w, 9]);
                }
                let (bid, text, x, y, w) = MOUTH_PLUS;
                let t = h.rebase(text);
                cdecl(h, WINDOW_ADD_BUTTON, &[window, bid, t, x, y, w, 9]);
                mouth_th_text(h, window);
                for i in 0..4 {
                    cdecl(h, BUTTON_SET_TOGGLE, &[window, i, 1]);
                }
            }
            0x8003 => h.write(h.rebase(MOUTH_OPEN), 0u8),
            0x8007 => {
                let count_at = h.rebase(MOUTH_COUNT);
                for i in 0..4u32 {
                    if i == id {
                        cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, i, 1]);
                        if i == 1 {
                            let c = h.read::<u8>(count_at).wrapping_add(1) & 3;
                            h.write(count_at, c);
                            line_text(h, window, 1, FMT_OPEN, u32::from(c));
                        }
                        let c = i32::from(h.read::<i8>(count_at)).cast_unsigned();
                        cdecl(h, GUI_SET_TALK, &[i, c]);
                    } else {
                        cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, i, 0]);
                    }
                }
                let delta: i32 = match id {
                    5 => 10,
                    7 => -10,
                    _ => return 0,
                };
                let th = cdecl(h, GUI_GET_MOUTH_TH, &[]).wrapping_add(delta.cast_unsigned());
                cdecl(h, GUI_SET_MOUTH_TH, &[th]);
                mouth_th_text(h, window);
            }
            _ => {}
        }
        0
    }
    /// A spin control's set-up (`FUN_00852080`, `FUN_008523e0` write
    /// these words before `FUN_00851c70`): steps 1, 1, -1, 10 with the
    /// add flags of the press and the repeat, the up, down and value
    /// ids, the format, value, maximum and minimum, the delay cleared.
    #[derive(Debug, Clone, Copy)]
    pub struct Spin {
        pub up: u32,
        pub down: u32,
        pub id: u32,
        pub fmt: u32,
        pub value: Option<u32>,
        pub max: u32,
        pub min: u32,
    }

    /// Writes `s` into the spin control at `at_` in the original's order.
    fn spin_init(h: &mut impl Host, sp: u32, s: &Spin) {
        h.write(at(sp, 0x10), s.up);
        h.write(at(sp, 0x14), s.down);
        h.write(at(sp, 0x18), s.id);
        h.write(at(sp, 0x1c), s.fmt);
        if let Some(v) = s.value {
            h.write(at(sp, 0x20), v);
        }
        h.write(at(sp, 0x28), s.min);
        h.write(at(sp, 0x24), s.max);
        h.write(sp, 1u16);
        h.write(at(sp, 2), 1u8);
        h.write(at(sp, 4), 1u32);
        h.write(at(sp, 8), u32::MAX);
        h.write(at(sp, 0xc), 10u32);
        h.write(at(sp, 0x2c), 0u32);
    }

    /// The physics window's two spin controls (cloth y and xz, in 1/100).
    pub const CLOTH_SPINS: [u32; 2] = [0x0132_5758, 0x0132_5788];
    /// `Ch_ClothGetParaEdit()`, `Ch_ClothSetParaEdit(v)`,
    /// `Ch_ClothResetAll()`, `Gu_WrapAngle_00853000(a)` (`ST0`),
    /// `Sg_GlobalSetWind(x, y, z)` (`f32`), `Ch_ClothSetY(f)`,
    /// `Ch_ClothSetXZ(f)` (cdecl).
    pub const CLOTH_GET_PARA_EDIT: u32 = 0x0082_4610;
    pub const CLOTH_SET_PARA_EDIT: u32 = 0x0082_4a30;
    pub const CLOTH_RESET_ALL: u32 = 0x0083_3800;
    pub const WRAP_ANGLE: u32 = 0x0085_3000;
    pub const GLOBAL_SET_WIND: u32 = 0x0082_4e30;
    pub const CLOTH_SET_Y: u32 = 0x0082_4a50;
    pub const CLOTH_SET_XZ: u32 = 0x0082_4a40;
    /// The physics window's open byte, wind toggle counter, title, button
    /// labels, spin labels, `"%d.%02d"`; pi and 0.5 (`f32`), 100.0
    /// (`f64`).
    pub const PHYSICS_OPEN: u32 = 0x0132_52c8;
    pub const WIND_COUNT: u32 = 0x0132_52cc;
    pub const PHYSICS_TITLE: u32 = 0x00b3_c3ec;
    pub const PHYSICS_BUTTONS: [(u32, u32, u32); 3] = [
        (4, 0x00b5_b5cc, 0),
        (3, 0x00b5_b5d4, 0xa),
        (0, 0x00b5_54dc, 0x14),
    ];
    pub const PHYSICS_LABELS: [u32; 2] = [0x00b8_f804, 0x00b5_b5dc];
    pub const FMT_HUNDREDTHS: u32 = 0x00b5_b5e0;
    pub const F_PI: u32 = 0x00b5_ae68;
    pub const F_HALF: u32 = 0x00b9_2194;
    pub const C_100: u32 = 0x00b9_22e0;

    /// `v / 100` as the original rounds it for `Ch_ClothSetY/XZ`.
    fn hundredths(h: &impl Host, v: u32) -> u32 {
        let f = x87::load(x87::store(f64::from(v.cast_signed())));
        let k: f64 = h.read(h.rebase(C_100));
        x87::store(x87::div(f, k))
    }

    /// `FUN_00852080(window, msg, id)` (cdecl): the physics window. Both
    /// spin controls see the message first (`FUN_00851b20`, the xz one
    /// first). Create (`0x8001`): "Physics", Reset (4), wind (3) and
    /// edit (0) toggles, the y and xz spins (ids 5/7/9 and 6/8/10, 1.00,
    /// 0..=300 with minimum 0 and 1). Close (`0x8003`): open byte
    /// cleared. Release (`0x8007`): 0 toggles the cloth editing, 3 the
    /// wind (odd: `Sg_GlobalSetWind(0.5, wrap(pi), 0)`, even: all 0), 4
    /// resets the cloth. Paint (`0x8011`): the toggles, both values as
    /// `"%d.%02d"` and to `Ch_ClothSetY` / `Ch_ClothSetXZ` (`/ 100`, rounded
    /// through the caller's `msg` argument word, `slots + 4`).
    /// Returns 0.
    pub fn physics_proc(h: &mut impl Host, window: u32, msg: u32, id: u32, slots: u32) -> u32 {
        for sp in [CLOTH_SPINS[1], CLOTH_SPINS[0]] {
            let sp = h.rebase(sp);
            spin_msg(h, window, msg, id, sp);
        }
        match msg {
            0x8001 => {
                let t = h.rebase(PHYSICS_TITLE);
                cdecl(h, WINDOW_SET_TITLE, &[window, t]);
                for (bid, text, y) in PHYSICS_BUTTONS {
                    let t = h.rebase(text);
                    cdecl(h, WINDOW_ADD_BUTTON, &[window, bid, t, 0, y, 0x36, 9]);
                }
                cdecl(h, BUTTON_SET_TOGGLE, &[window, 0, 1]);
                cdecl(h, BUTTON_SET_TOGGLE, &[window, 3, 1]);
                for (k, (ids, min, y)) in [((5, 7, 9), 0, 0x1e), ((6, 8, 10), 1, 0x29)]
                    .into_iter()
                    .enumerate()
                {
                    let sp = h.rebase(CLOTH_SPINS[k]);
                    let s = Spin {
                        up: ids.0,
                        down: ids.1,
                        id: ids.2,
                        fmt: 0,
                        value: Some(100),
                        max: 300,
                        min,
                    };
                    spin_init(h, sp, &s);
                    let label = h.rebase(PHYSICS_LABELS[k]);
                    spin_add(h, window, sp, 0, y, 10, label, 0x2c, 0x28);
                }
            }
            0x8003 => h.write(h.rebase(PHYSICS_OPEN), 0u8),
            0x8007 => match id {
                0 => {
                    let v = cdecl(h, CLOTH_GET_PARA_EDIT, &[]);
                    cdecl(h, CLOTH_SET_PARA_EDIT, &[1u32.wrapping_sub(v)]);
                }
                3 => {
                    let at_ = h.rebase(WIND_COUNT);
                    let n = h.read::<u32>(at_).wrapping_add(1);
                    h.write(at_, n);
                    if n & 1 != 0 {
                        let pi: u32 = h.read(h.rebase(F_PI));
                        let a =
                            h.call_sym_st0(WRAP_ANGLE, Conv::Cdecl, &[x87::store(x87::load(pi))]);
                        let half: u32 = h.read(h.rebase(F_HALF));
                        let a = x87::store(x87::load(a.to_bits()));
                        cdecl(h, GLOBAL_SET_WIND, &[x87::store(x87::load(half)), a, 0]);
                    } else {
                        cdecl(h, GLOBAL_SET_WIND, &[0, 0, 0]);
                    }
                }
                4 => {
                    cdecl(h, CLOTH_RESET_ALL, &[]);
                }
                _ => {}
            },
            0x8011 => {
                let v = cdecl(h, CLOTH_GET_PARA_EDIT, &[]);
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, 0, v]);
                let w = h.read::<u32>(h.rebase(WIND_COUNT)) & 1;
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, 3, w]);
                for (sp, set) in [
                    (CLOTH_SPINS[0], CLOTH_SET_Y),
                    (CLOTH_SPINS[1], CLOTH_SET_XZ),
                ] {
                    let sp = h.rebase(sp);
                    let v: i32 = h.read(at(sp, 0x20));
                    let id: u32 = h.read(at(sp, 0x18));
                    let fmt = h.rebase(FMT_HUNDREDTHS);
                    let (q, r) = (v.wrapping_div(100), v.wrapping_rem(100));
                    let args = [window, id, fmt, q.cast_unsigned(), r.cast_unsigned()];
                    cdecl(h, CONTROL_SET_TEXTF, &args);
                    let v: u32 = h.read(at(sp, 0x20));
                    let f = hundredths(h, v);
                    // the original rounds through its `msg` argument word
                    h.write(at(slots, 4), f);
                    cdecl(h, set, &[f]);
                }
            }
            _ => {}
        }
        0
    }
    /// The shadow window's five spin controls (colour r, g, b, direction
    /// a, b) and their (up, down, value, maximum, minimum, label).
    pub const SHADOW_SPINS: [(u32, Spin, u32); 5] = [
        (0x0132_5638, shadow_spin(0xd, 0xff, 0), 0x00b8_f7e8),
        (0x0132_5668, shadow_spin(0xe, 0xff, 0), 0x00b8_f7bc),
        (0x0132_5698, shadow_spin(0xf, 0xff, 0), 0x00b8_f7ac),
        (0x0132_56c8, shadow_spin(0x10, 0x168, u32::MAX), 0x00b8_f7c0),
        (0x0132_56f8, shadow_spin(0x11, 0x5a, 1), 0x00b8_f7f8),
    ];

    const fn shadow_spin(up: u32, max: u32, min: u32) -> Spin {
        Spin {
            up,
            down: up + 5,
            id: up + 10,
            fmt: FMT_D,
            value: None,
            max,
            min,
        }
    }

    /// `Ch_DrawSetShadowMode(m)`, `Ch_DrawGetShadowSemi()`,
    /// `Sg_SetMovieGomiAvoid(v)`, `Ch_DrawGetShadowColor(&r, &g, &b)`,
    /// `Ch_DrawGetShadowDir(&a, &b)` (one word each), `dbgPrintf`,
    /// `Ch_DrawGetShadowMode()`, `Ch_DrawSetShadowColor(r, g, b)`,
    /// `Ch_DrawSetShadowDir(a, b)`, `Gu_WindowControlSetDisable(window,
    /// id, off)` (cdecl).
    pub const SET_SHADOW_MODE: u32 = 0x0083_0140;
    pub const GET_SHADOW_SEMI: u32 = 0x0082_ec70;
    pub const SET_MOVIE_GOMI_AVOID: u32 = 0x0083_0150;
    pub const GET_SHADOW_COLOR: u32 = 0x0082_ebf0;
    pub const GET_SHADOW_DIR: u32 = 0x0082_ec30;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const GET_SHADOW_MODE: u32 = 0x0082_ec60;
    pub const SET_SHADOW_COLOR: u32 = 0x0083_0100;
    pub const SET_SHADOW_DIR: u32 = 0x0083_0120;
    pub const CONTROL_SET_DISABLE: u32 = 0x0084_a620;
    /// The shadow window's open byte, title, its mode buttons' labels,
    /// the dump button's label, and the dump's three formats.
    pub const SHADOW_OPEN: u32 = 0x0132_52d0;
    pub const SHADOW_TITLE: u32 = 0x00b7_c868;
    pub const SHADOW_BUTTONS: [u32; 5] = [
        0x00b5_b5a0,
        0x00b5_b5a8,
        0x00b5_b5b0,
        0x00b5_b5b8,
        0x00b5_b5bc,
    ];
    pub const SHADOW_DUMP: u32 = 0x00b5_b5c4;
    pub const SHADOW_FORMATS: [u32; 3] = [0x00b5_b55c, 0x00b5_b580, 0x00b7_cae0];
    /// The original's frame below `ebp` (the colour and direction words,
    /// the mode).
    pub const SHADOW_FRAME: u32 = 0x14;

    /// The shadow colour and direction into the caller's `msg` word
    /// (`slots + 4`, r) and the frame (`ebp - 4` g, `- 8` b, `- 0xc` a,
    /// `- 0x10` b); returns the five words.
    fn shadow_get(h: &mut impl Host, slots: u32, ebp: u32) -> [u32; 5] {
        let r = at(slots, 4);
        let at_ = |o: u32| ebp.wrapping_sub(o);
        cdecl(h, GET_SHADOW_COLOR, &[r, at_(4), at_(8)]);
        cdecl(h, GET_SHADOW_DIR, &[at_(0xc), at_(0x10)]);
        [
            h.read(r),
            h.read(at_(4)),
            h.read(at_(8)),
            h.read(at_(0xc)),
            h.read(at_(0x10)),
        ]
    }

    /// The five values into the spin controls' `+0x20`.
    fn shadow_store(h: &mut impl Host, v: [u32; 5]) {
        for ((sp, _, _), v) in SHADOW_SPINS.into_iter().zip(v) {
            let sp = h.rebase(sp);
            h.write(at(sp, 0x20), v);
        }
    }

    /// `FUN_008523e0(window, msg, id; slots)` (cdecl, frame at `ebp`, 0x14
    /// bytes): the shadow window. The five spin controls see the message
    /// first. Create (`0x8001`): "Shadow", the five mode toggles 0..=4,
    /// toggle 5 too, the spins from the current colour and direction
    /// (through the caller's `msg` word and the frame, as the original),
    /// the dump button 6, client height 0x41. Close (`0x8003`): open byte
    /// cleared. Release (`0x8007`): 0..=4 set the shadow mode, 5 toggles
    /// the semi flag (`Sg_SetMovieGomiAvoid`), 6 prints the settings as C
    /// code. Paint (`0x8011`): the toggles; in mode 0 the spins take the
    /// current values, else the shadow takes the spins'; the spins and
    /// their labels (8..=0x1b, -1) are disabled below mode 3. Returns 0.
    pub fn shadow_proc(
        h: &mut impl Host,
        window: u32,
        msg: u32,
        id: u32,
        slots: u32,
        ebp: u32,
    ) -> u32 {
        for (sp, _, _) in SHADOW_SPINS {
            let sp = h.rebase(sp);
            spin_msg(h, window, msg, id, sp);
        }
        match msg {
            0x8001 => {
                let t = h.rebase(SHADOW_TITLE);
                cdecl(h, WINDOW_SET_TITLE, &[window, t]);
                for (i, (text, y)) in SHADOW_BUTTONS
                    .into_iter()
                    .zip([0, 0xa, 0x14, 0x1e, 0x28])
                    .enumerate()
                {
                    let t = h.rebase(text);
                    cdecl(h, WINDOW_ADD_BUTTON, &[window, i as u32, t, 0, y, 0x3c, 9]);
                }
                for i in 0..6 {
                    cdecl(h, BUTTON_SET_TOGGLE, &[window, i, 1]);
                }
                let v = shadow_get(h, slots, ebp);
                shadow_store(h, v);
                for (k, (sp, s, label)) in SHADOW_SPINS.into_iter().enumerate() {
                    let sp = h.rebase(sp);
                    let s = Spin {
                        fmt: h.rebase(s.fmt),
                        ..s
                    };
                    spin_init(h, sp, &s);
                    let label = h.rebase(label);
                    let y = 0xb * k as u32;
                    spin_add(h, window, sp, 0x41, y, 10, label, 10, 0x28);
                }
                let t = h.rebase(SHADOW_DUMP);
                cdecl(h, WINDOW_ADD_BUTTON, &[window, 6, t, 0, 0x37, 0x8c, 9]);
                cdecl(h, WINDOW_SET_CLIENT_H, &[window, 0x41]);
            }
            0x8003 => h.write(h.rebase(SHADOW_OPEN), 0u8),
            0x8007 => match id {
                0..=4 => {
                    cdecl(h, SET_SHADOW_MODE, &[id]);
                }
                5 => {
                    let v = cdecl(h, GET_SHADOW_SEMI, &[]);
                    cdecl(h, SET_MOVIE_GOMI_AVOID, &[1u32.wrapping_sub(v)]);
                }
                6 => {
                    let v = shadow_get(h, slots, ebp);
                    let f = h.rebase(SHADOW_FORMATS[0]);
                    cdecl(h, DBG_PRINTF, &[f, v[0], v[1], v[2]]);
                    let f = h.rebase(SHADOW_FORMATS[1]);
                    cdecl(h, DBG_PRINTF, &[f, v[3], v[4]]);
                    let f = h.rebase(SHADOW_FORMATS[2]);
                    cdecl(h, DBG_PRINTF, &[f]);
                }
                _ => {}
            },
            0x8011 => {
                let mode = cdecl(h, GET_SHADOW_MODE, &[]);
                h.write(ebp.wrapping_sub(0x14), mode);
                for i in 0..5 {
                    cdecl(
                        h,
                        BUTTON_SET_TOGGLE_VALUE,
                        &[window, i, u32::from(mode == i)],
                    );
                }
                let semi = cdecl(h, GET_SHADOW_SEMI, &[]);
                cdecl(h, BUTTON_SET_TOGGLE_VALUE, &[window, 5, semi]);
                if mode == 0 {
                    let v = shadow_get(h, slots, ebp);
                    shadow_store(h, v);
                } else {
                    let mut v = [0u32; 5];
                    for (k, w) in v.iter_mut().enumerate() {
                        let sp = h.rebase(SHADOW_SPINS[k].0);
                        *w = h.read(at(sp, 0x20));
                    }
                    cdecl(h, SET_SHADOW_COLOR, &[v[0], v[1], v[2]]);
                    cdecl(h, SET_SHADOW_DIR, &[v[3], v[4]]);
                }
                let off = u32::from(h.read::<i32>(ebp.wrapping_sub(0x14)) < 3);
                for i in 8..0x1c {
                    cdecl(h, CONTROL_SET_DISABLE, &[window, i, off]);
                }
                cdecl(h, CONTROL_SET_DISABLE, &[window, u32::MAX, off]);
            }
            _ => {}
        }
        0
    }
}
