//! M5 stage 1, list `pc-ui`, batch 8: the debug GUI's window openers
//! (`Gu_WindowCreate` with fixed rectangles and procedures), its packet
//! argument readers, a byte writer and a file-list selector fill.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the originals' argument words
pub mod bridge {
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
}
