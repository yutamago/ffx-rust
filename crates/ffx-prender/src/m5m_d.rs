//! M5 stage 1, list `map`, batch 4: the debug GUI's item lists, number
//! windows and the percentage readout (`rcui*`).
//!
//! None of it ran in a coverage run. Callees stay calls to the originals,
//! cdecl as their `ret` says.

/// `Gu_WindowAddButton(window, id, text, x, y, w, h)` and
/// `Gu_WindowAddStatic(...)` (cdecl).
pub const ADD_BUTTON: u32 = 0x0084_a470;
pub const ADD_STATIC: u32 = 0x0084_a4d0;
/// `Gu_PrintSetColor(r, g, b, a)`, `Gu_Printf(fmt, ...)` (cdecl).
pub const PRINT_SET_COLOR: u32 = 0x0084_8740;
pub const PRINTF: u32 = 0x0084_8880;
/// The readout's threshold (f64 50.0) and `"%02d%%"`.
pub const PERCENT_RED_FROM: u32 = 0x00b1_f2c0;
pub const PERCENT_FORMAT: u32 = 0x00b6_87d4;
/// `rcuiWindowCreate`, `Gu_WindowSetArg(window, arg)`,
/// `rcuiValueGet(p, kind)` (cdecl), the number window's procedure.
pub const RCUI_WINDOW_CREATE: u32 = 0x0093_7ef0;
pub const WINDOW_SET_ARG: u32 = 0x0084_b560;
pub const VALUE_GET: u32 = 0x0093_c210;
pub const NUMBER_PROC: u32 = 0x0093_c3f0;
/// `g_RcuiWork`.
pub const RCUI_WORK: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0193_c0f0);
pub const SET_REG: u32 = 0x0092_0450;
pub const RESTORE_REG: u32 = 0x0092_0250;
pub const PACKET_INIT: u32 = 0x0092_7a20;
pub const PACKET_OPEN: u32 = 0x0092_7a70;
pub const PACKET_CLOSE: u32 = 0x0092_75b0;
pub const PACKET_DRAW: u32 = 0x0092_7600;
/// `g_RcPrim2DPacket`.
pub const PRIM_2D_PACKET: u32 = 0x00c4_98dc;
pub const GUIDE_OBJECT: u32 = 0x0092_c740;
pub const GUIDE_SELECTED_A: u32 = 0x0092_c250;
pub const GUIDE_SELECTED_B: u32 = 0x0092_c4c0;
pub const POINT_GET: u32 = 0x0083_56b0;
pub const POINT_FIX: u32 = 0x0091_ffd0;
pub const POINT_MATRIX: u32 = 0x0092_0020;
pub const VEC_ORIGIN: u32 = 0x0092_cc20;
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
pub const DIVIDE_W: u32 = 0x006e_d420;
pub const PACKET_RECT: u32 = 0x0092_7630;
pub const WRAP_92C600: u32 = 0x0092_c5e0;
/// The point rects' size (f32) and colours (two RGBA float records).
pub const POINT_W: u32 = 0x00b9_22f8;
pub const POINT_H: u32 = 0x00b6_7df4;
pub const POINT_COLOUR_ON: u32 = 0x00c5_e7a8;
pub const POINT_COLOUR_OFF: u32 = 0x00c5_e7f8;

/// The map lists (`host:/ffx/proj/map/master/info/mapid.csv`, the
/// battle maps' `.../btlmaster/info/btlmap...`), `","`, `isdigit`
/// (import slot), `strncpy` (the CRT thunk), the chunk and parse calls.
pub const STAGE_LIST: u32 = 0x00b6_8698;
pub const BATTLE_STAGE_LIST: u32 = 0x00b6_88fc;
pub const COMMA: u32 = 0x00b8_f884;
pub const ISDIGIT: u32 = 0x00b0_c498;
pub const STRNCPY: u32 = 0x0094_9256;
pub const CHUNK_INIT: u32 = 0x0092_b840;
pub const CHUNK_LOAD_FROM_FILE: u32 = 0x0092_b880;
pub const CHUNK_FINI: u32 = 0x0092_b7f0;
pub const PARSE_INIT: u32 = 0x0092_bf50;
pub const PARSE_SET: u32 = 0x0092_c0a0;
pub const PARSE_SET_MODE: u32 = 0x0092_c0c0;
pub const PARSE_ADD_SEPARATORS: u32 = 0x0092_c0f0;
pub const PARSE_ADVANCE_LINE: u32 = 0x0092_b9b0;
pub const PARSE_IS_END: u32 = 0x0092_bfb0;
pub const PARSE_GET_INT: u32 = 0x0092_bc50;
pub const PARSE_FINI: u32 = 0x0092_ba60;

/// `rcuiMouseMoveGet`'s callees: the window's mouse x / y, the window
/// calls and the throwaway window's procedure.
pub const MOUSE_X: u32 = 0x0084_9ae0;
pub const MOUSE_Y: u32 = 0x0084_9af0;
pub const WINDOW_CREATE: u32 = 0x0084_a920;
pub const WINDOW_UPDATE: u32 = 0x0084_9b90;
pub const WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
pub const MOUSE_WINDOW_PROC: u32 = 0x0093_c200;

/// `sprintf` (import slot); `"%d"`, `"%f"`.
pub const SPRINTF: u32 = 0x00b0_c3dc;
pub const FMT_D: u32 = 0x00b0_f604;
pub const FMT_F: u32 = 0x00b4_0274;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    /// `rcuiAddItems(window, items)`: each 0x1c-byte item until a kind
    /// of 0: kind 1 `Gu_WindowAddButton(window, id, text, x, y, w, h)`,
    /// kind 2 `Gu_WindowAddStatic(window, 1, text, x, y, w, h)`, others
    /// nothing. `eax`: the last item's callee's, or `kind - 2` for another
    /// kind; with no items the caller's (0 here; no caller reads it).
    pub fn add_items(h: &mut impl Host, window: u32, items: u32) -> u32 {
        let mut eax = 0;
        let mut item = items;
        while word(h, item) != 0 {
            let kind = word(h, item);
            let w: [u32; 6] = std::array::from_fn(|k| word(h, item + 4 + 4 * k as u32));
            eax = match kind {
                1 => cdecl(h, ADD_BUTTON, &[window, w[0], w[1], w[2], w[3], w[4], w[5]]),
                2 => cdecl(h, ADD_STATIC, &[window, 1, w[1], w[2], w[3], w[4], w[5]]),
                k => k.wrapping_sub(2),
            };
            item = item.wrapping_add(0x1c);
        }
        eax
    }

    /// `rcuiPercentPrint_00930ba0(x)`: the colour (`Gu_PrintSetColor(r,
    /// 0, 0, 0x80)`, red `4 x - 200` (truncated) from 50 % up, NaN
    /// included; 0 below), `Gu_Printf("%02d%%", x)`, the colour back to
    /// black -> its `eax`.
    pub fn percent_print(h: &mut impl Host, x: u32) -> u32 {
        let v = x87::load(x);
        let from: f64 = h.read(h.rebase(PERCENT_RED_FROM));
        let red = if v < from {
            0
        } else {
            ffx_bridge::x87::ftol(h, v)
                .wrapping_mul(4)
                .wrapping_sub(0xc8)
        };
        cdecl(h, PRINT_SET_COLOR, &[red, 0, 0, 0x80]);
        let n = ffx_bridge::x87::ftol(h, v);
        let fmt = h.rebase(PERCENT_FORMAT);
        cdecl(h, PRINTF, &[fmt, n]);
        cdecl(h, PRINT_SET_COLOR, &[0, 0, 0, 0x80])
    }

    /// `rcbgDrawGuidePackets_0092c130()` (frame: the register block at
    /// `ebp - 0x34`, the packet at `ebp - 0x3c`): `rcbgSetReg` of the
    /// block (0x3000f, 0; 0x47, 0; 0x44, 0; 0x42, 0; 0, 0; -1, -1), the
    /// packet opened on `g_RcPrim2DPacket`; for each of `g_RcuiWork`'s
    /// `+0x824` objects (`+0x870` list) `FUN_0092c740(pkt, obj)`, and for
    /// the selected one (`+0x8b0`) `FUN_0092c250` (`+0x8b4` 0) or
    /// `FUN_0092c4c0`; the packet closed and drawn, `rcbgRestoreReg` ->
    /// its `eax`.
    pub fn draw_guide_packets(h: &mut impl Host, ebp: u32) -> u32 {
        let w = RCUI_WORK.get(h);
        let regs = ebp - 0x34;
        for (off, v) in [
            (0x2c_u32, 0x47_u32),
            (0x28, 0),
            (0x1c, 0x42),
            (0x18, 0),
            (0x14, 0),
            (0x10, 0),
            (0xc, 0xffff_ffff),
            (8, 0xffff_ffff),
            (0x34, 0x3_000f),
            (0x30, 0),
            (0x24, 0x44),
            (0x20, 0),
        ] {
            h.write(ebp - off, v);
        }
        cdecl(h, SET_REG, &[regs]);
        let pkt = ebp - 0x3c;
        cdecl(h, PACKET_INIT, &[pkt]);
        let buf = word(h, h.rebase(PRIM_2D_PACKET));
        cdecl(h, PACKET_OPEN, &[pkt, buf]);
        let mut i = 0_i32;
        while i < word(h, w.wrapping_add(0x824)).cast_signed() {
            let obj = word(h, w.wrapping_add(0x870 + 4 * i.cast_unsigned()));
            cdecl(h, GUIDE_OBJECT, &[pkt, obj]);
            if i.cast_unsigned() == word(h, w.wrapping_add(0x8b0)) {
                let f = if word(h, w.wrapping_add(0x8b4)) == 0 {
                    GUIDE_SELECTED_A
                } else {
                    GUIDE_SELECTED_B
                };
                cdecl(h, f, &[pkt, obj]);
            }
            i += 1;
        }
        cdecl(h, PACKET_CLOSE, &[pkt]);
        cdecl(h, PACKET_DRAW, &[pkt]);
        cdecl(h, RESTORE_REG, &[])
    }

    /// `rcbgDrawGuidePoints_0092c4c0(pkt, obj)` (frame: a point record at
    /// `ebp - 0x54`, a matrix at `- 0x94`, a vector at `- 0x14`): for each
    /// of `g_RcuiWork`'s `+0x8cc` points of `obj`: the point
    /// (`0x008356b0(obj, i, 0, rec)`, `0x0091ffd0(rec)`), its matrix
    /// (`0x00920020`), the origin (`0x0092cc20`) through it
    /// (`FFXVu0ApplyMatrix 0x006ed2e0`, the w divide `0x006ed420`), a rect
    /// packet at its x, y sized by the floats at `0x00b922f8` / `0x00b67df4`,
    /// coloured by `0x00c5e7a8` for the selected point (`+0x8d0`), else
    /// `0x00c5e7f8`; then the selected point again into
    /// `Wrap_0092c600_0092c5e0(pkt, rec)` -> its `eax`.
    pub fn draw_guide_points(h: &mut impl Host, ebp: u32, pkt: u32, obj: u32) -> u32 {
        let w = RCUI_WORK.get(h);
        let rec = ebp - 0x54;
        let m = ebp - 0x94;
        let v = ebp - 0x14;
        let mut i = 0_u32;
        while i.cast_signed() < word(h, w.wrapping_add(0x8cc)).cast_signed() {
            cdecl(h, POINT_GET, &[obj, i, 0, rec]);
            cdecl(h, POINT_FIX, &[rec]);
            cdecl(h, POINT_MATRIX, &[m, rec]);
            cdecl(h, VEC_ORIGIN, &[v]);
            cdecl(h, APPLY_MATRIX, &[v, m, v]);
            let ww = word(h, ebp - 8);
            cdecl(h, DIVIDE_W, &[v, v, x87::store(x87::load(ww))]);
            let selected = i == word(h, w.wrapping_add(0x8d0));
            let colour = h.rebase(if selected {
                POINT_COLOUR_ON
            } else {
                POINT_COLOUR_OFF
            });
            let x = x87::store(x87::load(word(h, v)));
            let y = x87::store(x87::load(word(h, v + 4)));
            let pw = x87::store(x87::load(word(h, h.rebase(POINT_W))));
            let ph = x87::store(x87::load(word(h, h.rebase(POINT_H))));
            cdecl(h, PACKET_RECT, &[pkt, x, y, pw, ph, colour]);
            i += 1;
        }
        let sel = word(h, w.wrapping_add(0x8d0));
        cdecl(h, POINT_GET, &[obj, sel, 0, rec]);
        cdecl(h, POINT_FIX, &[rec]);
        cdecl(h, WRAP_92C600, &[pkt, rec])
    }

    /// `isdigit` of the parse cursor's byte.
    fn cursor_is_digit(h: &mut impl Host, parse: u32, isdigit: u32) -> u32 {
        let cur = word(h, parse + 4);
        let c: u8 = h.read(cur);
        h.call(isdigit, Conv::Cdecl, &[u32::from(c)])
    }

    /// `rcuiGetStageName2(no, out, battle)` (frame: the chunk at
    /// `ebp - 0x98`, the parse at `ebp - 0x90`) -> 1 when found: the map
    /// list (`mapid.csv`, or the battle maps' list with `battle`) read
    /// with `,` separators; lines up to the first that starts with a
    /// digit skipped; then line by line the first number compared with
    /// `no`; the matching line's text (6 characters, 8 for battle maps)
    /// copied to `out` and ended. The parse and chunk released; 0 when
    /// the file doesn't load or no line matches.
    pub fn stage_name2(h: &mut impl Host, ebp: u32, no: u32, out: u32, battle: u32) -> u32 {
        let (path, n) = if battle == 0 {
            (STAGE_LIST, 6)
        } else {
            (BATTLE_STAGE_LIST, 8)
        };
        let chunk = ebp - 0x98;
        let parse = ebp - 0x90;
        cdecl(h, CHUNK_INIT, &[chunk]);
        let path = h.rebase(path);
        let mut found = 0;
        if cdecl(h, CHUNK_LOAD_FROM_FILE, &[chunk, path]) != 0 {
            cdecl(h, PARSE_INIT, &[parse]);
            cdecl(h, PARSE_SET, &[parse, chunk]);
            cdecl(h, PARSE_SET_MODE, &[parse, 1]);
            let comma = h.rebase(COMMA);
            cdecl(h, PARSE_ADD_SEPARATORS, &[parse, comma]);
            let isdigit = word(h, h.rebase(ISDIGIT));
            if cursor_is_digit(h, parse, isdigit) == 0 {
                loop {
                    cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
                    if cursor_is_digit(h, parse, isdigit) != 0 {
                        break;
                    }
                }
            }
            if cdecl(h, PARSE_IS_END, &[parse]) == 0 {
                loop {
                    if cdecl(h, PARSE_GET_INT, &[parse]) == no {
                        let cur = word(h, parse + 4);
                        cdecl(h, STRNCPY, &[out, cur, n]);
                        h.write(out.wrapping_add(n), 0_u8);
                        found = 1;
                        break;
                    }
                    cdecl(h, PARSE_ADVANCE_LINE, &[parse]);
                    if cdecl(h, PARSE_IS_END, &[parse]) != 0 {
                        break;
                    }
                }
            }
            cdecl(h, PARSE_FINI, &[parse]);
        }
        cdecl(h, CHUNK_FINI, &[chunk]);
        found
    }

    /// `rcuiMouseMoveGet(&dx, &dy)`: the mouse position of the dragged
    /// window (`+0xbe8`, `0x00849ae0` / `0x00849af0`) less the drag
    /// origin (`+0xbf4`, `+0xbf8`) into the given words (dx under 2 in
    /// size becomes 0); then a throwaway window at the origin
    /// (`Gu_WindowCreate(x, y, 0, 0, 0, 0x0093c200)`, `0x00849b90`,
    /// `Gu_WindowCloseTree`) -> its `eax`.
    pub fn mouse_move_get(h: &mut impl Host, dx: u32, dy: u32) -> u32 {
        let w = RCUI_WORK.get(h);
        let id = word(h, w.wrapping_add(0xbe8));
        let x = cdecl(h, MOUSE_X, &[id]);
        let w = RCUI_WORK.get(h);
        let id = word(h, w.wrapping_add(0xbe8));
        let y = cdecl(h, MOUSE_Y, &[id]);
        if dx != 0 {
            let w = RCUI_WORK.get(h);
            let v = x.wrapping_sub(word(h, w.wrapping_add(0xbf4))).cast_signed();
            h.write(dx, v);
            if v < 2 && v > -2 {
                h.write(dx, 0_u32);
            }
        }
        if dy != 0 {
            let w = RCUI_WORK.get(h);
            let v = y.wrapping_sub(word(h, w.wrapping_add(0xbf8)));
            h.write(dy, v);
        }
        let w = RCUI_WORK.get(h);
        let (ox, oy) = (
            word(h, w.wrapping_add(0xbf4)),
            word(h, w.wrapping_add(0xbf8)),
        );
        let proc = h.rebase(MOUSE_WINDOW_PROC);
        let win = cdecl(h, WINDOW_CREATE, &[ox, oy, 0, 0, 0, proc]);
        cdecl(h, WINDOW_UPDATE, &[win]);
        cdecl(h, WINDOW_CLOSE_TREE, &[win])
    }

    fn c_len(h: &impl Mem, s: u32) -> u32 {
        let mut n = 0_u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// `rcuiNumberCreate(num, value, kind, title, parent)`: the number
    /// window (`rcuiWindowCreate(num, 8, 8, 0x98, 0x4e, 6, proc, title,
    /// parent)`, its argument the record); `+4` the value, `+8` its kind;
    /// the text at `+0xc`: kind 0xb a float (`"%f"`, cut at 10 characters,
    /// trailing zeros and a bare point dropped), else `"%d"` of
    /// `rcuiValueGet`; `+0x1c` its length, `+0x20` = 1, `+0x24` = `+0x28`
    /// = 0.
    pub fn number_create(
        h: &mut impl Host,
        num: u32,
        value: u32,
        kind: u32,
        title: u32,
        parent: u32,
    ) {
        let proc = h.rebase(NUMBER_PROC);
        cdecl(
            h,
            RCUI_WINDOW_CREATE,
            &[num, 8, 8, 0x98, 0x4e, 6, proc, title, parent],
        );
        let w = word(h, num);
        cdecl(h, WINDOW_SET_ARG, &[w, num]);
        h.write(num + 4, value);
        h.write(num + 8, kind);
        let text = num + 0xc;
        let sprintf = word(h, h.rebase(SPRINTF));
        if kind == 0xb {
            let x = x87::load(word(h, value)).to_bits();
            #[allow(clippy::cast_possible_truncation)] // the double's halves
            let (lo, hi) = (x as u32, (x >> 32) as u32);
            let fmt = h.rebase(FMT_F);
            h.call(sprintf, Conv::Cdecl, &[text, fmt, lo, hi]);
            h.write(num + 0x16, 0_u8);
            let len = c_len(h, text);
            h.write(num + 0x1c, len);
            let mut i = len.cast_signed() - 1;
            loop {
                if i < 0 {
                    break;
                }
                let at = text.wrapping_add(i.cast_unsigned());
                let c: u8 = h.read(at);
                if c != b'0' {
                    if c == b'.' {
                        h.write(at, 0_u8);
                    }
                    break;
                }
                h.write(at, 0_u8);
                i -= 1;
            }
        } else {
            let v = cdecl(h, VALUE_GET, &[value, kind]);
            let fmt = h.rebase(FMT_D);
            h.call(sprintf, Conv::Cdecl, &[text, fmt, v]);
        }
        let len = c_len(h, text);
        h.write(num + 0x1c, len);
        h.write(num + 0x20, 1_u32);
        h.write(num + 0x24, 0_u32);
        h.write(num + 0x28, 0_u32);
    }
}
