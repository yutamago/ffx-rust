//! M5 stage 1, list `pc-ui`, batch 8: the debug GUI's window openers
//! (`Gu_WindowCreate` with fixed rectangles and procedures), its packet
//! argument readers, a byte writer and a file-list selector fill.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    pub const WINDOW_CREATE: u32 = 0x0084_a920;
    pub const WINDOW_SET_LOCAL_TRANS: u32 = 0x0084_b620;
    pub const SET_RESOURCE: u32 = 0x0083_fe10;
    pub const PACKET_GET_PTR: u32 = 0x0083_b650;
    /// `MSVCR110!atoi`.
    pub const ATOI_IAT: u32 = 0x00b0_c40c;
    pub const RCBG_INIT: u32 = 0x0092_1ce0;
    pub const RCUI_INIT: u32 = 0x0093_4390;
    pub const RCUI_CLOSE_WINDOW: u32 = 0x0093_7f50;
    pub const ENABLE_GET_TEXTURE: u32 = 0x0083_0320;
    pub const BUTTON_SET_TOGGLE: u32 = 0x0084_93e0;
    pub const MAKE_FILE_LIST: u32 = 0x0085_09e0;
    pub const SELECTOR_RESET: u32 = 0x0084_9f40;
    pub const SELECTOR_SET_ITEM: u32 = 0x0084_9fd0;
    /// `g_GuByteCursor` (name ours): where `0x00853400` writes.
    pub const BYTE_CURSOR: u32 = 0x0132_5908;
    /// `g_GuPacketArgCount` (name ours) and the empty string returned
    /// past it.
    pub const PACKET_ARG_COUNT: u32 = 0x0132_5910;
    pub const EMPTY: u32 = 0x00b8_fbeb;
    /// `g_GuSpriteWindowOpen`, `g_GuMemGraphOpen` (names ours).
    pub const SPRITE_WINDOW_OPEN: u32 = 0x0132_5916;
    pub const MEM_GRAPH_OPEN: u32 = 0x0132_5954;
    pub const RCUI_WORK: u32 = 0x0193_c0f0;
    pub const YN_MAP_SYSTEM: u32 = 0x0193_bf80;
    /// `g_GuTextureButton` (name ours): `Gui_ButtonSetTexture`'s button.
    pub const TEXTURE_BUTTON: u32 = 0x0130_3768;
    /// The file list's pattern (`0x0084d3a0`).
    pub const FILE_PATTERN: u32 = 0x00b5_abcc;

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn set_g(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    /// `Gu_WindowCreate(x, y, w, h, flags, proc)` with `proc` rebased.
    pub fn window(h: &mut impl Host, rect: [u32; 5], proc_sym: u32) -> u32 {
        let p = h.rebase(proc_sym);
        h.call_sym(
            WINDOW_CREATE,
            Conv::Cdecl,
            &[rect[0], rect[1], rect[2], rect[3], rect[4], p],
        )
    }

    /// `Gui_SpriteWindowInit_00853780()`: the sprite window (once,
    /// [`SPRITE_WINDOW_OPEN`]); its `eax` (0 when open already: the
    /// original's is its caller's).
    pub fn sprite_window_init(h: &mut impl Host) -> u32 {
        if h.read::<u8>(h.rebase(SPRITE_WINDOW_OPEN)) != 0 {
            return 0;
        }
        let w = window(h, [0x14, 0xa, 0x78, 0x46, 6], 0x0085_3520);
        let at = h.rebase(SPRITE_WINDOW_OPEN);
        h.write(at, 1u8);
        w
    }

    /// `Gui_MemGraph()`: the memory graph window (once,
    /// [`MEM_GRAPH_OPEN`]), its local transparency, `Sg_SetResource(0x11,
    /// 0)`.
    pub fn mem_graph(h: &mut impl Host) -> u32 {
        if g(h, MEM_GRAPH_OPEN) != 0 {
            return 0;
        }
        let w = window(h, [8, 0, 0x1e, 0xc1, 0xe], 0x0085_5020);
        h.call_sym(WINDOW_SET_LOCAL_TRANS, Conv::Cdecl, &[w, 1, 0x10]);
        set_g(h, MEM_GRAPH_OPEN, 1);
        h.call_sym(SET_RESOURCE, Conv::Cdecl, &[0x11, 0])
    }

    /// `0x00853400(c)`: the byte at [`BYTE_CURSOR`], which moves on.
    pub fn put_byte(h: &mut impl Host, c: u32) {
        let p = g(h, BYTE_CURSOR);
        #[allow(clippy::cast_possible_truncation)] // the byte argument
        h.write(p, c as u8);
        set_g(h, BYTE_CURSOR, p.wrapping_add(1));
    }

    /// `Gu_PacketArg_00853140(i)`: packet argument `i` (a string), the
    /// empty string from `i` = [`PACKET_ARG_COUNT`] on (signed).
    pub fn packet_arg(h: &mut impl Host, i: u32) -> u32 {
        let pk = h.call_sym(PACKET_GET_PTR, Conv::Cdecl, &[]);
        if i.cast_signed() >= g(h, PACKET_ARG_COUNT).cast_signed() {
            return h.rebase(EMPTY);
        }
        h.read(pk.wrapping_add(i.wrapping_mul(4)))
    }

    /// `Gu_PacketArgInt_00853170(i)`: `atoi` of packet argument `i` (a
    /// tail jump with the argument slot replaced).
    pub fn packet_arg_int(h: &mut impl Host, i: u32) -> u32 {
        let pk = h.call_sym(PACKET_GET_PTR, Conv::Cdecl, &[]);
        let s = if i.cast_signed() >= g(h, PACKET_ARG_COUNT).cast_signed() {
            h.rebase(EMPTY)
        } else {
            h.read(pk.wrapping_add(i.wrapping_mul(4)))
        };
        let f: u32 = h.read(h.rebase(ATOI_IAT));
        h.call(f, Conv::Cdecl, &[s])
    }

    /// `Gui_ChrInfo()`: the character info window.
    pub fn chr_info(h: &mut impl Host) -> u32 {
        window(h, [0x28, 4, 0x140, 0x60, 0xe], 0x0085_37d0)
    }

    /// `Gui_ClockInit()`: the clock window.
    pub fn clock_init(h: &mut impl Host) -> u32 {
        window(h, [8, 0x9e, 0x64, 0x3c, 6], 0x0085_3ec0)
    }

    /// `Gui_YanagiInit()`: without the UI work, the background (when the
    /// map system is off) and `rcuiInit` (a tail jump); else that UI
    /// window closed.
    pub fn yanagi_init(h: &mut impl Host) -> u32 {
        let w = g(h, RCUI_WORK);
        if w != 0 {
            return h.call_sym(RCUI_CLOSE_WINDOW, Conv::Cdecl, &[w]);
        }
        if g(h, YN_MAP_SYSTEM) == 0 {
            h.call_sym(RCBG_INIT, Conv::Cdecl, &[]);
        }
        h.call_sym(RCUI_INIT, Conv::Cdecl, &[])
    }

    /// `Gui_ButtonSetTexture()`: with the texture button, its toggle 8 set
    /// to `1 - Ch_EnableGetTexture()`.
    pub fn button_set_texture(h: &mut impl Host) -> u32 {
        if g(h, TEXTURE_BUTTON) == 0 {
            return 0;
        }
        let e = h.call_sym(ENABLE_GET_TEXTURE, Conv::Cdecl, &[]);
        let b = g(h, TEXTURE_BUTTON);
        h.call_sym(
            BUTTON_SET_TOGGLE,
            Conv::Cdecl,
            &[b, 8, 1u32.wrapping_sub(e)],
        )
    }

    /// `Gu_SelectorFillFiles_0084d3a0(list, sel, dir)`: the directory's
    /// files listed (`sg_make_filelist(list, dir, pattern)`), the selector
    /// reset to their count and given each name (`list +8` the names).
    pub fn selector_fill_files(h: &mut impl Host, list: u32, sel: u32, dir: u32) {
        let pat = h.rebase(FILE_PATTERN);
        h.call_sym(MAKE_FILE_LIST, Conv::Cdecl, &[list, dir, pat]);
        let n: u32 = h.read(list);
        h.call_sym(SELECTOR_RESET, Conv::Cdecl, &[sel, n]);
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(list).cast_signed() {
            let names: u32 = h.read(list + 8);
            let name: u32 = h.read(names.wrapping_add(4 * i));
            h.call_sym(SELECTOR_SET_ITEM, Conv::Cdecl, &[sel, i, name, i]);
            i += 1;
        }
    }

    // ---- the debug GUI's printers and character selector (batch 9) ----

    pub const SET_PRINTF_FUNC: u32 = 0x0085_10c0;
    pub const SG_VFPRINTF: u32 = 0x0085_11d0;
    pub const PUT_BYTE: u32 = 0x0085_3400;
    pub const LINE_PRINTF: u32 = 0x0085_33b0;
    /// `MSVCR110!vsprintf`, `MSVCR110!sprintf` (cdecl).
    pub const VSPRINTF_IAT: u32 = 0x00b0_c430;
    pub const SPRINTF_IAT: u32 = 0x00b0_c3dc;
    pub const PRINT_SET_XY: u32 = 0x0084_8840;
    pub const GU_PRINTF: u32 = 0x0084_8880;
    /// `g_GuLogY` (name ours): the log window's next line (+7 a line).
    pub const LOG_Y: u32 = 0x0132_5920;
    pub const SETTING_SAVE_LIGHT: u32 = 0x0084_d8a0;
    pub const GET_USER_NAME: u32 = 0x0082_0610;
    pub const PC_WRITE: u32 = 0x0083_afe0;
    pub const DEBUG_LINE: u32 = 0x023c_4000;
    /// The light settings file's header lines and its path format.
    pub const LIGHT_HEADER: u32 = 0x00b5_b61c;
    pub const LIGHT_VERSION: u32 = 0x00b5_b62c;
    pub const LIGHT_PATH: u32 = 0x00b5_b5e8;
    pub const CHR_NB_USE: u32 = 0x0082_85e0;
    /// `g_GuChrSelector` (name ours), `g_ChrCount`, `g_ChrArray`.
    pub const CHR_SELECTOR: u32 = 0x0132_5918;
    pub const CHR_COUNT: u32 = 0x023c_44e0;
    pub const CHR_ARRAY: u32 = 0x023c_44e4;

    /// `Gu_LinePrintf_008533b0(fmt, ...)` -> the length: `fmt` with the
    /// caller's arguments (`va` = their address) printed byte by byte
    /// through `Gu_PutByte_00853400` (the printer swapped in and back),
    /// then `"\r\n"`.
    pub fn line_printf(h: &mut impl Host, fmt: u32, va: u32) -> u32 {
        let put = h.rebase(PUT_BYTE);
        let old = h.call_sym(SET_PRINTF_FUNC, Conv::Cdecl, &[put]);
        let n = h.call_sym(SG_VFPRINTF, Conv::Cdecl, &[0, fmt, va]);
        h.call_sym(SET_PRINTF_FUNC, Conv::Cdecl, &[old]);
        let p = g(h, BYTE_CURSOR);
        h.write(p, 0x0a0du16);
        set_g(h, BYTE_CURSOR, p.wrapping_add(2));
        n
    }

    /// `Gu_LogPrintf_00853e40(fmt, ...)`: `vsprintf` into `buf` (1 KB, the
    /// original's `ebp - 0x404`) printed at the log's next line.
    pub fn log_printf(h: &mut impl Host, fmt: u32, va: u32, buf: u32) -> u32 {
        let f: u32 = h.read(h.rebase(VSPRINTF_IAT));
        h.call(f, Conv::Cdecl, &[buf, fmt, va]);
        let y = g(h, LOG_Y);
        h.call_sym(PRINT_SET_XY, Conv::Cdecl, &[0, y]);
        let y = g(h, LOG_Y);
        set_g(h, LOG_Y, y.wrapping_add(7));
        h.call_sym(GU_PRINTF, Conv::Cdecl, &[buf])
    }

    /// `Gui_SettingSaveLightFile_00853310()`: the light settings printed
    /// into the packet buffer (two header lines, `Gui_SettingSaveLight`)
    /// and, when anything was printed, written to the PC as
    /// `sprintf(g_DebugLine, path, user)`.
    pub fn save_light_file(h: &mut impl Host) -> u32 {
        let pk = h.call_sym(PACKET_GET_PTR, Conv::Cdecl, &[]);
        set_g(h, BYTE_CURSOR, pk);
        let a = h.rebase(LIGHT_HEADER);
        h.call_sym(LINE_PRINTF, Conv::Cdecl, &[a]);
        let b = h.rebase(LIGHT_VERSION);
        h.call_sym(LINE_PRINTF, Conv::Cdecl, &[b, 0x197]);
        let mut eax = h.call_sym(SETTING_SAVE_LIGHT, Conv::Cdecl, &[]);
        let len = g(h, BYTE_CURSOR).wrapping_sub(pk);
        if len != 0 {
            let user = h.call_sym(GET_USER_NAME, Conv::Cdecl, &[]);
            let line = h.rebase(DEBUG_LINE);
            let path = h.rebase(LIGHT_PATH);
            let f: u32 = h.read(h.rebase(SPRINTF_IAT));
            h.call(f, Conv::Cdecl, &[line, path, user]);
            eax = h.call_sym(PC_WRITE, Conv::Cdecl, &[pk, len, line]);
        }
        eax
    }

    /// `Gu_ChrSelectorFill_00853dc0()` -> 0, or -1 with no characters in
    /// use: the selector reset to the count in use and given each used
    /// character (`+2` set; its name `+4`), the array re-read each step.
    pub fn chr_selector_fill(h: &mut impl Host) -> u32 {
        let n = h.call_sym(CHR_NB_USE, Conv::Cdecl, &[]);
        let sel = g(h, CHR_SELECTOR);
        h.call_sym(SELECTOR_RESET, Conv::Cdecl, &[sel, n]);
        if n == 0 {
            return 0xffff_ffff;
        }
        let mut c = g(h, CHR_ARRAY);
        let mut idx = 0u32;
        loop {
            let end = g(h, CHR_ARRAY).wrapping_add(g(h, CHR_COUNT).wrapping_mul(0x880));
            if c == end {
                break;
            }
            if h.read::<u8>(c + 2) != 0 {
                let name: u32 = h.read(c + 4);
                let sel = g(h, CHR_SELECTOR);
                h.call_sym(SELECTOR_SET_ITEM, Conv::Cdecl, &[sel, idx, name, c]);
                idx += 1;
            }
            c = c.wrapping_add(0x880);
        }
        0
    }

    // ---- the angle wrap and the help window (batch 10) ----

    /// The wrap's bounds and step (`f64`): pi, 2 pi, -pi (each with a zero
    /// low word).
    pub const WRAP_HI: u32 = 0x00b5_82f0;
    pub const WRAP_STEP: u32 = 0x00b5_9638;
    pub const WRAP_LO: u32 = 0x00b5_9648;
    pub const WINDOW_TO_FRONT: u32 = 0x0084_b770;
    pub const WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
    /// `MSVCR110!strncpy`.
    pub const STRNCPY_IAT: u32 = 0x00b0_c368;
    /// `g_GuHelpText` (64 bytes), `g_GuHelpWindow`, its position (names
    /// ours).
    pub const HELP_TEXT: u32 = 0x0132_59a0;
    pub const HELP_WINDOW: u32 = 0x0132_5950;
    pub const HELP_X: u32 = 0x0132_59e0;
    pub const HELP_Y: u32 = 0x0132_59e4;

    fn konst(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read::<u64>(h.rebase(sym)))
    }

    /// `Gu_WrapAngle_*(x)` -> `ST0` (three copies: `0x0084d330`,
    /// `0x00853000`, `0x00854200`): while above pi, `x = f32(x - 2 pi)`;
    /// then while below -pi, `x = f32(x + 2 pi)` (a NaN passes through).
    pub fn wrap_angle(h: &impl Host, x: u32) -> f32 {
        let hi = konst(h, WRAP_HI);
        let step = konst(h, WRAP_STEP);
        let lo = konst(h, WRAP_LO);
        let mut v = x87::load(x);
        while v > hi {
            v = x87::load(x87::store(x87::sub(v, step)));
        }
        while v < lo {
            v = x87::load(x87::store(x87::add(v, step)));
        }
        #[allow(clippy::cast_possible_truncation)] // v is an f32 value
        let r = v as f32;
        r
    }

    /// `Gu_HelpWindow_00855360(text)`: with a text, it is copied (64
    /// bytes, `strncpy`), the help window made when missing and brought to
    /// the front (a tail jump); without, the window closed and cleared.
    pub fn help_window(h: &mut impl Host, text: u32) -> u32 {
        if text != 0 {
            let dst = h.rebase(HELP_TEXT);
            let f: u32 = h.read(h.rebase(STRNCPY_IAT));
            h.call(f, Conv::Cdecl, &[dst, text, 0x40]);
            let mut w = g(h, HELP_WINDOW);
            if w == 0 {
                let x = g(h, HELP_X);
                let y = g(h, HELP_Y);
                w = window(h, [x, y, 0, 0xa, 0], 0x0085_5290);
                set_g(h, HELP_WINDOW, w);
            }
            return h.call_sym(WINDOW_TO_FRONT, Conv::Cdecl, &[w]);
        }
        let w = g(h, HELP_WINDOW);
        if w == 0 {
            return 0;
        }
        let eax = h.call_sym(WINDOW_CLOSE_TREE, Conv::Cdecl, &[w]);
        set_g(h, HELP_WINDOW, 0);
        eax
    }
}
