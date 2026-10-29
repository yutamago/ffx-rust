//! M5 `main`: the PS2 debug GUI's helpers that build the debug menus
//! (buttons and static texts in the current column, the windows opened
//! once), the debug text buffer, the menu thunks and small string and
//! list helpers of the name-input debug screen; the sphere grid's exit
//! and parameter thunk.

/// The originals over game memory.
#[allow(clippy::many_single_char_names, clippy::verbose_bit_mask)] // registers; the tests as written
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `Gu_WindowAddButton(window, id, text, x, y, w, h)`,
    /// `Gu_WindowAddStatic(...)` (cdecl, seven words),
    /// `Gu_WindowControlSetHide(window, id, hide)` (cdecl).
    pub const ADD_BUTTON: u32 = 0x0084_a470;
    pub const ADD_STATIC: u32 = 0x0084_a4d0;
    pub const SET_HIDE: u32 = 0x0084_a670;
    /// The menu column's next y and the list of the controls added.
    pub const MENU_Y: u32 = 0x0132_412c;
    pub const MENU_LIST: u32 = 0x0132_3d6c;

    fn list_add(h: &mut impl Host, id: u32) -> u32 {
        let list: u32 = h.read(h.rebase(MENU_LIST));
        let n: u32 = h.read(list);
        h.write(list.wrapping_add(n.wrapping_mul(4)).wrapping_add(4), id);
        h.write(list, n.wrapping_add(1));
        n
    }

    fn next_y(h: &mut impl Host, step: u32) -> u32 {
        let at = h.rebase(MENU_Y);
        let y: u32 = h.read(at);
        h.write(at, y.wrapping_add(step));
        y
    }

    /// `sg_addButton(window, id, text)` (`0x00850850`, height 10, step
    /// 11) and `sg_addButtonS(window, id, text)` (`0x008508a0`, height 9,
    /// step 10) (cdecl): a 70-wide button at the column's y, the id
    /// listed, the control shown. Returns the show's `eax`.
    pub fn add_button(
        h: &mut impl Host,
        window: u32,
        id: u32,
        text: u32,
        height: u32,
        step: u32,
    ) -> u32 {
        let y: u32 = h.read(h.rebase(MENU_Y));
        h.call_sym(
            ADD_BUTTON,
            Conv::Cdecl,
            &[window, id, text, 0, y, 0x46, height],
        );
        next_y(h, step);
        list_add(h, id);
        h.call_sym(SET_HIDE, Conv::Cdecl, &[window, id, 1])
    }

    /// `sg_addStatic(window, id, text)` (`0x008508f0`, cdecl): a static
    /// text (height 10, step 10), shown, then listed. Returns the list's
    /// old count (left in `eax`).
    pub fn add_static(h: &mut impl Host, window: u32, id: u32, text: u32) -> u32 {
        let y: u32 = h.read(h.rebase(MENU_Y));
        h.call_sym(
            ADD_STATIC,
            Conv::Cdecl,
            &[window, id, text, 0, y, 0x46, 0xa],
        );
        next_y(h, 0xa);
        h.call_sym(SET_HIDE, Conv::Cdecl, &[window, id, 1]);
        list_add(h, id)
    }

    /// `Gu_WindowCreate(x, y, w, h, flags, callback)` (cdecl).
    pub const WINDOW_CREATE: u32 = 0x0084_a920;

    /// A debug window opened once: its open byte, the six words (the
    /// callback a code address) and whether the opener sets the byte.
    #[derive(Debug, Clone, Copy)]
    pub struct OnceWindow {
        pub open: u32,
        pub args: [u32; 6],
        pub marks: bool,
    }

    /// `FUN_00851db0`, `FUN_00851de0`, `FUN_00851e10`, `FUN_00851e40`
    /// (they set their byte) and `sasa_menu` (`0x00942110`; its window
    /// sets it).
    pub const WINDOWS: [(u32, OnceWindow); 5] = [
        (
            0x0085_1db0,
            OnceWindow {
                open: 0x0132_52c9,
                args: [0x78, 0xa, 0x46, 0x41, 6, 0x0085_1e70],
                marks: true,
            },
        ),
        (
            0x0085_1de0,
            OnceWindow {
                open: 0x0132_52c8,
                args: [0x78, 0xa, 0x78, 0x46, 6, 0x0085_2080],
                marks: true,
            },
        ),
        (
            0x0085_1e10,
            OnceWindow {
                open: 0x0132_52d0,
                args: [0x78, 0xa, 0x96, 0x50, 6, 0x0085_23e0],
                marks: true,
            },
        ),
        (
            0x0085_1e40,
            OnceWindow {
                open: 0x0132_52cb,
                args: [0x78, 0xa, 0x78, 0x5a, 6, 0x0085_2a30],
                marks: true,
            },
        ),
        (
            0x0094_2110,
            OnceWindow {
                open: 0x0193_c150,
                args: [0x64, 0x40, 0xf0, 0x6a, 0xe, 0x0093_ee10],
                marks: false,
            },
        ),
    ];

    /// The window `w` created unless its byte is set (then the byte set
    /// when the opener sets it). Returns nothing the callers read.
    pub fn open_once(h: &mut impl Host, w: &OnceWindow) {
        let open = h.rebase(w.open);
        if h.read::<u8>(open) != 0 {
            return;
        }
        let mut a = w.args;
        a[5] = h.rebase(a[5]);
        h.call_sym(WINDOW_CREATE, Conv::Cdecl, &a); // call width: 6 (x, y, w, h, flags, callback)
        if w.marks {
            h.write(open, 1u8);
        }
    }

    /// The debug text buffer's length and its bytes.
    pub const TEXT_LEN: u32 = 0x0132_414c;
    pub const TEXT: u32 = 0x0132_4150;

    /// `FUN_00850b50(c)` (cdecl): `c`'s low byte appended to the text
    /// buffer (`al` holds it after).
    pub fn text_put(h: &mut impl Host, c: u32) {
        let at = h.rebase(TEXT_LEN);
        let n: u32 = h.read(at);
        #[allow(clippy::cast_possible_truncation)] // the low byte, `mov al`
        h.write(h.rebase(TEXT).wrapping_add(n), c as u8);
        h.write(at, n.wrapping_add(1));
    }

    /// `Sg_EventjumpDispose`, `eiAbmParaGet` (`0x00a54860`) and the
    /// target `code_009436b0` jumps to (cdecl, no words).
    pub const EVENTJUMP_DISPOSE: u32 = 0x0083_b420;
    pub const ABM_PARA_GET: u32 = 0x00a5_4860;
    pub const TARGET_00947CC0: u32 = 0x0094_7cc0;

    /// A lone `jmp target` thunk: the target's `eax`.
    pub fn thunk(h: &mut impl Host, target: u32) -> u32 {
        h.call_sym(target, Conv::Cdecl, &[])
    }

    /// `user_free_thunk(p)`, `graphicAbmapDestory()`,
    /// `graphicSetFlipVsnc(on)`, `graphicActivateLoadingScreen()`
    /// (cdecl); the sphere grid's two buffers.
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const ABMAP_DESTROY: u32 = 0x0063_8fb0;
    pub const SET_FLIP_VSYNC: u32 = 0x0064_30e0;
    pub const LOADING_SCREEN: u32 = 0x0063_9130;
    pub const ABM_BUF_A: u32 = 0x01a8_60ec;
    pub const ABM_BUF_B: u32 = 0x01a8_60f0;

    /// `eiAbmExit()` (`0x00a54720`, cdecl): both buffers freed, the
    /// sphere grid's graphics destroyed, flip vsync on, then the loading
    /// screen's `eax` (a tail jump).
    pub fn abm_exit(h: &mut impl Host) -> u32 {
        let a: u32 = h.read(h.rebase(ABM_BUF_A));
        h.call_sym(USER_FREE, Conv::Cdecl, &[a]);
        let b: u32 = h.read(h.rebase(ABM_BUF_B));
        h.call_sym(USER_FREE, Conv::Cdecl, &[b]);
        h.call_sym(ABMAP_DESTROY, Conv::Cdecl, &[]);
        h.call_sym(SET_FLIP_VSYNC, Conv::Cdecl, &[1]);
        h.call_sym(LOADING_SCREEN, Conv::Cdecl, &[])
    }

    /// `FUN_00942030(dst, src)` (cdecl): `strcpy`; returns the address of
    /// `src`'s 0.
    pub fn str_copy(h: &mut impl Host, dst: u32, src: u32) -> u32 {
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(src.wrapping_add(k));
            h.write(dst.wrapping_add(k), c);
            if c == 0 {
                return src.wrapping_add(k);
            }
            k += 1;
        }
    }

    /// The name-input screen's current list, its lists' entry pointers
    /// and counts, and the selection word.
    pub const NAME_LIST: u32 = 0x025d_5f90;
    pub const NAME_ENTRIES: u32 = 0x025d_5f44;
    pub const NAME_COUNTS: u32 = 0x025d_5f4c;
    pub const NAME_SELECTION: u32 = 0x025d_5f7c;

    /// `FUN_00942320()` (cdecl): bit 1 cleared in every 12-byte entry of
    /// the current list (count signed), the selection -1. Returns the
    /// entry pointer past the last one cleared (the first when none).
    pub fn name_list_clear(h: &mut impl Host) -> u32 {
        let list: u32 = h.read(h.rebase(NAME_LIST));
        let mut p: u32 = h.read(h.rebase(NAME_ENTRIES).wrapping_add(list.wrapping_mul(4)));
        let n: u32 = h.read(h.rebase(NAME_COUNTS).wrapping_add(list.wrapping_mul(4)));
        let mut k = n.cast_signed();
        while k > 0 {
            let v: u32 = h.read(p);
            h.write(p, v & 0xfd);
            p = p.wrapping_add(0xc);
            k -= 1;
        }
        h.write(h.rebase(NAME_SELECTION), 0xffff_ffffu32);
        p
    }
}
