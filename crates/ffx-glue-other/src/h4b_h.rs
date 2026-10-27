//! List `h4-glue-other`, second agent (h4b), round 2 batch 3: the message
//! window's text pass, the sphere grid's cell quads and a nine-part
//! window frame.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{REMAP_X2, REMAP_Y2, int_f32, pass};

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

// ---- TODrawMessageWaitNamesp

/// `g_GraphicMapId`.
pub const GRAPHIC_MAP_ID: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0133_0844);
/// `g_CurrentMapNo`.
pub const CURRENT_MAP_NO: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x012f_bc40);
/// The flag that hides the messages on map 0x17e (`0x01871538`).
pub const HIDE_17E: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0187_1538);
/// `TOMakePktScissor(x0, y0, x1, y1)` (cdecl).
pub const PKT_SCISSOR: u32 = 0x008f_dee0;
/// `TOGetGroup(c)` (cdecl).
pub const GET_GROUP: u32 = 0x008d_f610;
/// `TOGetLangColonWidth_008df4f0(kind, font)` (cdecl).
pub const COLON_WIDTH: u32 = 0x008d_f4f0;
/// `TODrawChangeMacro2(window, code, pen)` (cdecl).
pub const CHANGE_MACRO2: u32 = 0x008b_5960;
/// `TOFirstInitMessageNormal(window, text, pen)` (cdecl).
pub const FIRST_INIT: u32 = 0x008d_f520;
/// `TOSetDrawInfo(code, window, pen)` (cdecl).
pub const SET_DRAW_INFO: u32 = 0x008d_f8f0;
/// `TOGetChangeMacroWordCount(table, text)` (cdecl).
pub const MACRO_WORD_COUNT: u32 = 0x008b_6b00;
/// `TOGetChangeMacroWidth(table, text, font)` (cdecl, `ST0`).
pub const MACRO_WIDTH: u32 = 0x008b_6a30;
/// `TOConvAlbhedWord(in, out)` (cdecl).
pub const CONV_ALBHED: u32 = 0x0090_6780;
/// `TOGetFontInfo(text, out)` (cdecl).
pub const GET_FONT_INFO: u32 = 0x008b_7240;
/// `TOMakePktFont1612(pen, glyph)` (cdecl).
pub const FONT1612: u32 = 0x008f_c7c0;
/// `TOSkipCode(text)` (cdecl).
pub const SKIP_CODE: u32 = 0x008d_fd80;
/// The pen (`0x025d09a0`, 7 words: colour, x, y, two more floats, ...,
/// `+0x1a` the font, `+0x1b` the alpha).
pub const PEN: u32 = 0x025d_09a0;
/// The glyph record `TOGetFontInfo` fills.
pub const GLYPH: u32 = 0x0184_1d50;

/// Stores `v` into the window argument's word, as the original does.
fn to_slot(h: &mut impl Host, slot: Option<u32>, v: u32) {
    if let Some(s) = slot {
        h.write(s, v);
    }
}

/// The pen's font byte cleared on map 0x17e (`& 0` there, `& 0xff`
/// elsewhere).
fn font_mask(h: &mut impl Host) {
    let pen = h.rebase(PEN);
    let keep: u8 = if GRAPHIC_MAP_ID.get(h) == 0x17e {
        0
    } else {
        0xff
    };
    let f: u8 = h.read(pen + 0x1a);
    h.write(pen + 0x1a, f & keep);
}

/// `f32` at `at` plus or minus the `f64` constant `c`, stored back.
fn bump(h: &mut impl Host, at: u32, c: u32, minus: bool) {
    let v = load(h.read(at));
    let k = konst64(h, c);
    h.write(at, store(if minus { sub(v, k) } else { add(v, k) }));
}

/// `TODrawMessageWaitNamesp(window)` (cdecl): one pass of a message
/// window's text into the pen at `0x025d09a0`, from the window's text
/// start (`+0x94` to the cursor `+0x8c`) and pen (`+0xb8`, 7 words, with
/// x, y, the two more floats and the colour from `+0x48..+0x58` and the
/// font `+0x1a`); the pen goes back to `+0xd4` at the end and the drawn
/// count `+0xa0` to `+0xa4`. Map 0xdf's window at (0x1fe, 0x185) moves 15
/// up, map 0x17e's draws nothing while `0x01871538` (returns at once),
/// else sets the scissor and moves (+2, -15). Then code by code while
/// `+0x9c` < `+0xa0` (`TOGetGroup`): 1 a line break (the first one in a
/// choice window adds the colon widths around `TODrawChangeMacro2` of
/// the name; later ones go back to the line's x and down a line: 16 on
/// maps 0xd4 and 0x15b, `0x00c5d784 * 16` for fonts 1 and 3, else
/// 0x00b667d8; `TOFirstInitMessageNormal` for kind 1); 2 a control code
/// (`TOSetDrawInfo`, tabs counting against `+0xa4`); 3 a macro (on the
/// first line of a non-choice window only counted, else drawn with
/// `TODrawChangeMacro2` after the two Al Bhed/choice layouts of maps
/// 0x15b and 0xd4 move the pen); 4 a glyph (counted, `+0xa0` grows up to
/// `+0xa8` unless `+0xb4` or `+0xac`; drawn with `TOMakePktFont1612`, on
/// the first line only in choice windows, Al Bhed words translated by
/// `TOConvAlbhedWord` into the frame). Returns 0.
///
/// `frame` is the original's `ebp - 0x14` (5 words: the 64-bit
/// temporary, the Al Bhed text, the line's x, the line count). `slot` is
/// the window argument's word: the original keeps integer and float
/// temporaries there.
#[allow(clippy::too_many_lines)]
pub fn message_wait_namesp(h: &mut impl Host, frame: u32, w: u32, slot: Option<u32>) -> u32 {
    let ebp = frame + 0x14;
    let start: u32 = h.read(w + 0x94);
    h.write(w + 0x8c, start);
    let pen = h.rebase(PEN);
    for k in 0..7u32 {
        let v: u32 = h.read(w + 0xb8 + 4 * k);
        h.write(pen + 4 * k, v);
    }
    for (to, from) in [
        (4u32, 0x48u32),
        (8, 0x4c),
        (0xc, 0x50),
        (0x10, 0x54),
        (0, 0x58),
    ] {
        let v: u32 = h.read(w + from);
        h.write(pen + to, v);
    }
    let font: u8 = h.read(w + 0x1a);
    h.write(pen + 0x1a, font);
    let map = GRAPHIC_MAP_ID.get(h);
    let mut x0 = pass(h.read(w + 0x48));
    h.write(ebp - 4, 0u32);
    h.write(ebp - 8, x0);
    if map == 0xdf {
        if h.read::<i16>(w + 0x14) == 0x1fe && h.read::<i16>(w + 0x16) == 0x185 {
            bump(h, pen + 8, 0x00b9_22c0, true);
            bump(h, pen + 0x10, 0x00b9_22c0, true);
        }
    } else if map == 0x17e {
        if HIDE_17E.get(h) != 0 {
            return 0;
        }
        h.call_sym(PKT_SCISSOR, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
        let y = load(h.read(pen + 8));
        let two = konst64(h, 0x00b9_2270);
        h.write(pen + 0x1a, 0u8);
        h.write(pen + 8, store(add(y, two)));
        let v = add(two, load(h.read(pen + 0x10)));
        h.write(pen + 0x10, store(v));
        bump(h, pen + 4, 0x00b9_22c0, true);
        bump(h, pen + 0xc, 0x00b9_22c0, true);
    }
    h.write(w + 0x9c, 0u32);
    let glyph = h.rebase(GLYPH);
    let mut lines: i32 = 0;
    if h.read::<i32>(w + 0xa0) > 0 {
        loop {
            let s: u32 = h.read(w + 0x8c);
            let c = u32::from(h.read::<u8>(s));
            match h.call_sym(GET_GROUP, Conv::Cdecl, &[c]) {
                1 => {
                    if lines == 0 {
                        if h.read::<i16>(w + 0x22) == 1 && h.read::<i16>(w + 0x24) > 0 {
                            let f = i32::from(h.read::<i16>(w + 0x1a)).cast_unsigned();
                            let a = h.call_sym(COLON_WIDTH, Conv::Cdecl, &[0, f]);
                            to_slot(h, slot, a);
                            let af = int_f32(a.cast_signed());
                            to_slot(h, slot, af);
                            let v = add(load(af), load(h.read(pen + 4)));
                            h.write(pen + 4, store(v));
                            h.call_sym(CHANGE_MACRO2, Conv::Cdecl, &[w, pen + 0x1c, pen]);
                            let f = i32::from(h.read::<i16>(w + 0x1a)).cast_unsigned();
                            let b = h.call_sym(COLON_WIDTH, Conv::Cdecl, &[1, f]);
                            to_slot(h, slot, b);
                            let bf = int_f32(b.cast_signed());
                            to_slot(h, slot, bf);
                            let v = add(load(bf), load(h.read(pen + 4)));
                            h.write(pen + 4, store(v));
                        }
                        x0 = pass(h.read(pen + 4));
                        h.write(ebp - 8, x0);
                    } else {
                        h.write(pen + 4, pass(x0));
                        let m = CURRENT_MAP_NO.get(h);
                        if m == 0xd4 || m == 0x15b {
                            bump(h, pen + 8, 0x00b3_13a0, false);
                        } else {
                            let f = h.read::<u16>(w + 0x1a);
                            if f == 1 || f == 3 {
                                let k = load(h.read(h.rebase(0x00c5_d784)));
                                let v = add(mul(k, konst64(h, 0x00b3_13a0)), load(h.read(pen + 8)));
                                h.write(pen + 8, store(v));
                            } else {
                                bump(h, pen + 8, 0x00b6_67d8, false);
                            }
                        }
                    }
                    lines += 1;
                    h.write(ebp - 4, lines);
                    if h.read::<i16>(w + 0xe) == 1 {
                        let s: u32 = h.read(w + 0x8c);
                        h.call_sym(FIRST_INIT, Conv::Cdecl, &[w, s.wrapping_add(1), pen]);
                    }
                }
                2 => {
                    let draw = if h.read::<u8>(s) == 9 {
                        let n: i32 = h.read(w + 0x9c);
                        let drawn: i32 = h.read(w + 0xa4);
                        if h.read::<u8>(s + 1) > 0xbf {
                            if n >= drawn {
                                h.call_sym(SET_DRAW_INFO, Conv::Cdecl, &[s, w, pen]);
                                let n = h.read::<u32>(w + 0x9c).wrapping_add(1);
                                h.write(w + 0x9c, n);
                                let n: u32 = h.read(w + 0x9c);
                                h.write(w + 0xa0, n);
                            } else {
                                h.write(w + 0x9c, n.wrapping_add(1));
                            }
                            false
                        } else {
                            n >= drawn
                        }
                    } else {
                        true
                    };
                    if draw {
                        h.call_sym(SET_DRAW_INFO, Conv::Cdecl, &[s, w, pen]);
                    }
                }
                3 => {
                    if lines == 0 && h.read::<i16>(w + 0x22) == 0 {
                        let t = i32::from(h.read::<i16>(w + 4)).cast_unsigned();
                        let n = h.call_sym(MACRO_WORD_COUNT, Conv::Cdecl, &[t, s]);
                        let c = h.read::<u32>(w + 0x9c).wrapping_add(n);
                        h.write(w + 0x9c, c);
                    } else {
                        macro_layout(h, ebp, w, lines, slot);
                        let s: u32 = h.read(w + 0x8c);
                        h.call_sym(CHANGE_MACRO2, Conv::Cdecl, &[w, s, pen]);
                    }
                }
                4 => {
                    let n = h.read::<u32>(w + 0x9c).wrapping_add(1);
                    h.write(w + 0x9c, n);
                    if h.read::<u32>(w + 0xb4) == 0 && h.read::<u32>(w + 0xac) == 0 {
                        let d: i32 = h.read(w + 0xa0);
                        if d < h.read::<i32>(w + 0xa8) {
                            h.write(w + 0xa0, d.wrapping_add(1));
                        }
                    }
                    if lines == 0 {
                        if h.read::<i16>(w + 0x22) == 1 {
                            let keep: u8 = if h.read::<u16>(w + 0x10) == 0x4000 {
                                0
                            } else {
                                0xff
                            };
                            let a: u8 = h.read(pen + 0x1b);
                            h.write(pen + 0x1b, a & keep);
                            let s: u32 = h.read(w + 0x8c);
                            h.call_sym(GET_FONT_INFO, Conv::Cdecl, &[s, glyph]);
                            font_mask(h);
                            h.call_sym(FONT1612, Conv::Cdecl, &[pen, glyph]);
                        }
                    } else {
                        let text = if h.read::<u16>(w + 0x10) == 0x4000 {
                            match h.call_sym(CONV_ALBHED, Conv::Cdecl, &[s, ebp - 0xc]) {
                                0 | 1 => h.write(pen + 0x1b, 0x60u8),
                                2 => h.write(pen + 0x1b, 0x56u8),
                                _ => {}
                            }
                            ebp - 0xc
                        } else {
                            s
                        };
                        h.call_sym(GET_FONT_INFO, Conv::Cdecl, &[text, glyph]);
                        font_mask(h);
                        h.call_sym(FONT1612, Conv::Cdecl, &[pen, glyph]);
                    }
                }
                _ => {}
            }
            let s: u32 = h.read(w + 0x8c);
            let next = h.call_sym(SKIP_CODE, Conv::Cdecl, &[s]);
            h.write(w + 0x8c, next);
            if h.read::<i32>(w + 0x9c) >= h.read::<i32>(w + 0xa0) {
                break;
            }
        }
    }
    let n: u32 = h.read(w + 0xa0);
    h.write(w + 0xa4, n);
    for k in 0..7u32 {
        let v: u32 = h.read(pen + 4 * k);
        h.write(w + 0xd4 + 4 * k, v);
    }
    0
}

/// The macro layouts of [`message_wait_namesp`]: an Al Bhed window
/// (`+0x10` 0x4000) past the first line uses alpha 0x56; on map 0x15b
/// with the window at (-200, 0xb4) and on map 0xd4 at (-30, 0xb4) the
/// lines move the pen (line 1 up by 0x00b41d20; 7 and 8 down by
/// 0x00b922b0 / 0x00b2c5c8 and right by 0x00b5af30; others down by
/// 0x00b2c5c8; then left by 0x00b49968), and lines 2 and 7 place the
/// macro: past `remapX2(0x00b54c7c)` (not right of it) only up by
/// `remapY2(0x00b3eb9c)` / `remapY2(0x00b1f2c8)`, else at the right edge
/// or centred by its width (`TOGetChangeMacroWidth`, kept in the slot)
/// and down by those.
fn macro_layout<H: Host>(h: &mut H, ebp: u32, w: u32, lines: i32, slot: Option<u32>) {
    let pen = h.rebase(PEN);
    if h.read::<u16>(w + 0x10) == 0x4000 && lines > 0 {
        h.write(pen + 0x1b, 0x56u8);
    }
    let m = CURRENT_MAP_NO.get(h);
    let at = |h: &mut H, x: i16| -> bool {
        h.read::<i16>(w + 0x14) == x && h.read::<i16>(w + 0x16) == 0xb4
    };
    let special = if m == 0x15b {
        at(h, -200)
    } else if m == 0xd4 {
        at(h, -30)
    } else {
        false
    };
    if !special {
        return;
    }
    let y = load(h.read(pen + 8));
    if lines == 1 {
        h.write(pen + 8, store(sub(y, konst64(h, 0x00b4_1d20))));
    } else if lines == 7 || lines == 8 {
        let k = if lines == 7 { 0x00b9_22b0 } else { 0x00b2_c5c8 };
        h.write(pen + 8, store(add(y, konst64(h, k))));
        bump(h, pen + 4, 0x00b5_af30, false);
    } else {
        h.write(pen + 8, store(add(y, konst64(h, 0x00b2_c5c8))));
    }
    bump(h, pen + 4, 0x00b4_9968, true);
    if lines != 2 && lines != 7 {
        return;
    }
    let xs = load(h.read(pen + 4));
    h.write(ebp - 0x14, xs);
    let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[konst(h, 0x00b5_4c7c)]);
    let (dy, w_hi, x_right, x_mid) = if lines == 2 {
        (0x00b3_eb9c, 0x00b5_8cd8, 0x00b6_58a4, 0x00b6_67e4)
    } else {
        (0x00b1_f2c8, 0x00b6_67e0, 0x00b4_3dd4, 0x00b6_67e8)
    };
    if r > xs {
        let t = i32::from(h.read::<i16>(w + 4)).cast_unsigned();
        let s: u32 = h.read(w + 0x8c);
        let font = u32::from(h.read::<u8>(pen + 0x1a));
        let width = store(h.call_sym_st0_f64(MACRO_WIDTH, Conv::Cdecl, &[t, s, font]));
        if let Some(sl) = slot {
            h.write(sl, width);
        }
        let wv = load(width);
        h.write(ebp - 0x14, wv);
        let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[konst(h, w_hi)]);
        let x = if r > wv {
            let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[konst(h, x_mid)]);
            sub(r, mul(wv, konst64(h, 0x00b9_2220)))
        } else {
            let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[konst(h, x_right)]);
            sub(r, wv)
        };
        h.write(pen + 4, store(x));
        let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[konst(h, dy)]);
        let v = add(r, load(h.read(pen + 8)));
        h.write(pen + 8, store(v));
    } else {
        let y = load(h.read(pen + 8));
        h.write(ebp - 0x14, y);
        let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[konst(h, dy)]);
        h.write(pen + 8, store(sub(y, r)));
    }
}

// ---- the sphere grid's cell quads

/// `g_Abmap`.
pub const ABMAP: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0230_5834);
/// The GS packet buffer pointer (`0x01a860ec`).
pub const PACKET: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x01a8_60ec);
/// `AbmapDrawScreenQuad_00a4b260()` (cdecl).
pub const SCREEN_QUAD: u32 = 0x00a4_b260;
/// `graphicDrawUIAbmapElement(record, name, kind)` (cdecl).
pub const DRAW_ABMAP_ELEMENT: u32 = 0x0063_e930;

/// A GS position word's screen coordinate: `v * k - base + off` (the
/// 12.4 fixed point scaled, less the GS origin, plus the half screen),
/// `v` through `fild`/`fstp dword`.
fn gs_coord(v: i32, k: [f64; 3]) -> u32 {
    store(add(sub(mul(load(int_f32(v)), k[0]), k[1]), k[2]))
}

/// `AbmapDrawGridCells_00a497b0()` (cdecl): the sphere grid's marked
/// cells. First the screen quad (`AbmapDrawScreenQuad_00a4b260`), then
/// for each of the 17 x 20 cells whose mark byte (`g_Abmap + 0x12e60`) is
/// set a GS sprite packet at `[0x01a860ec]` from the cell's corner
/// records (rows of 0x150 bytes from `g_Abmap + 0x1182c`, 16 bytes a
/// cell: positions, the texture's sizes as `f32`) and a
/// `graphicDrawUIAbmapElement(record, name, 1)` quad of it: positions
/// `v * 0x00b921e0 - 0x00b51bc0 + 0x00b922f0` (x) or `+ 0x00b54c70` (y),
/// UVs the packet's over its sizes, grey half-alpha colours. The top row
/// adds the quad mirrored above it, the last column the quad mirrored
/// to the right. Returns the last call's `eax`.
///
/// `frame` is the original's `ebp - 0x1a4` (the record at `+8`,
/// `+0x98`/`+0x9c` zero, the name buffer the callee fills at `+0xa0`;
/// the original's temporaries at `+0` and `+4`, which no callee sees,
/// are not kept).
#[allow(clippy::too_many_lines)]
pub fn grid_cells<H: Host>(h: &mut H, frame: u32) -> u32 {
    let ebp = frame + 0x1a4;
    let rec = ebp - 0x19c;
    let name = ebp - 0x104;
    let mut cell = ABMAP.get(h).wrapping_add(0x12e60);
    h.write(ebp - 0x10c, 0u32);
    h.write(ebp - 0x108, 0u32);
    let mut eax = h.call_sym(SCREEN_QUAD, Conv::Cdecl, &[]);
    let row0 = ABMAP.get(h).wrapping_add(0x1182c);
    let k = |h: &mut H| -> [f64; 4] {
        [
            konst64(h, 0x00b9_21e0),
            konst64(h, 0x00b5_1bc0),
            konst64(h, 0x00b9_22f0),
            konst64(h, 0x00b5_4c70),
        ]
    };
    for row in 0..0x11u32 {
        let base = row0.wrapping_add(0x150 * row);
        for col in 0..0x14u32 {
            let mark: u8 = h.read(cell);
            cell = cell.wrapping_add(1);
            if mark == 0 {
                continue;
            }
            let c = base.wrapping_add(0x10 * col);
            let p = PACKET.get(h);
            h.write(p, 0x8004u32);
            let again = PACKET.get(h);
            h.write(p + 8, 0x512u32);
            h.write(again + 4, 0x300a_4000u32);
            h.write(p + 0xc, 0u32);
            for (to, from) in [
                (0x30, 0x16c),
                (0x34, 0x168),
                (0x60, 0x15c),
                (0x64, 0x158),
                (0x90, 0x1c),
                (0x94, 0x18),
                (0xc0, 0xc),
                (0xc4, 8),
            ] {
                let v: u32 = h.read(c.wrapping_sub(from));
                h.write(p + to, v);
            }
            for (to, v) in [
                (0x38, 0x2100u32),
                (0x3c, 0x8000),
                (0x68, 0x2100),
                (0x6c, 0),
                (0x98, 0x2100),
                (0x9c, 0),
                (0xc8, 0x2100),
                (0xcc, 0),
                (0x14, 0),
                (0x10, 0),
            ] {
                h.write(p + to, v);
            }
            let a = pass(h.read(c.wrapping_sub(0x160)));
            h.write(p + 0x18, a);
            h.write(p + 0x1c, 0u32);
            h.write(p + 0x44, 0u32);
            let b = pass(h.read(c.wrapping_sub(0x150)));
            h.write(p + 0x48, b);
            h.write(p + 0x40, b);
            h.write(p + 0x4c, 0u32);
            h.write(p + 0x70, 0u32);
            let cc = pass(h.read(c.wrapping_sub(0x10)));
            h.write(p + 0x78, cc);
            h.write(p + 0x74, cc);
            h.write(p + 0x7c, 0u32);
            let d = pass(h.read(c));
            for to in [0xa8, 0xa4, 0xa0] {
                h.write(p + to, d);
            }
            h.write(p + 0xac, 0u32);
            for to in [
                0xb0, 0x80, 0x50, 0x20, 0xb4, 0x84, 0x54, 0x24, 0xb8, 0x88, 0x58, 0x28,
            ] {
                h.write(p + to, 0x40u32);
            }
            for to in [0xbc, 0x8c, 0x5c, 0x2c] {
                h.write(p + to, 0x80u32);
            }
            let [d1, d2, d3, d4] = k(h);
            let gx = |h: &mut H, off: u32| gs_coord(h.read(p + off), [d1, d2, d3]);
            let gy = |h: &mut H, off: u32| gs_coord(h.read(p + off), [d1, d2, d4]);
            let corners = [
                (gx(h, 0x30), gy(h, 0x34)),
                (gx(h, 0x60), gy(h, 0x64)),
                (gx(h, 0x90), gy(h, 0x94)),
                (gx(h, 0xc0), gy(h, 0xc4)),
            ];
            for (i, (x, y)) in corners.iter().enumerate() {
                h.write(rec + 0x20 * i as u32, *x);
                h.write(rec + 0x20 * i as u32 + 4, *y);
            }
            for (i, (u, v, s)) in [
                (0x10, 0x14, 0x18),
                (0x40, 0x44, 0x48),
                (0x70, 0x74, 0x78),
                (0xa0, 0xa4, 0xa8),
            ]
            .iter()
            .enumerate()
            {
                let su = store(div(load(h.read(p + u)), load(h.read(p + s))));
                h.write(rec + 0x20 * i as u32 + 8, su);
                let sv = store(div(load(h.read(p + v)), load(h.read(p + s))));
                h.write(rec + 0x20 * i as u32 + 0xc, sv);
            }
            for i in 0..4u32 {
                for (k, v) in [0x80u32, 0x80, 0x80, 0x40].iter().enumerate() {
                    h.write(rec + 0x20 * i + 0x10 + 4 * k as u32, *v);
                }
            }
            eax = h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, name, 1]);
            let mirror = |h: &mut H, a: u32, b: u32| -> i32 {
                let a: i32 = h.read(p + a);
                let b: i32 = h.read(p + b);
                a.wrapping_mul(2).wrapping_sub(b)
            };
            if row == 0 {
                let x0 = gx(h, 0x30);
                h.write(rec, x0);
                let y0 = gs_coord(mirror(h, 0x34, 0x94), [d1, d2, d4]);
                h.write(rec + 4, y0);
                let x1 = gx(h, 0x60);
                h.write(rec + 0x20, x1);
                let y1 = gs_coord(mirror(h, 0x64, 0xc4), [d1, d2, d4]);
                h.write(rec + 0x24, y1);
                let x2 = gx(h, 0x90);
                h.write(rec + 0x40, x2);
                let y2 = gy(h, 0x34);
                h.write(rec + 0x44, y2);
                let x3 = gx(h, 0xc0);
                h.write(rec + 0x60, x3);
                let y3 = gy(h, 0x64);
                h.write(rec + 0x64, y3);
                eax = h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, name, 1]);
            }
            if col == 0x13 {
                let x0 = gx(h, 0x60);
                h.write(rec, x0);
                let y0 = gy(h, 0x34);
                h.write(rec + 4, y0);
                let x1 = gs_coord(mirror(h, 0x60, 0x30), [d1, d2, d3]);
                h.write(rec + 0x20, x1);
                let y1 = gy(h, 0x64);
                h.write(rec + 0x24, y1);
                let x2 = gx(h, 0xc0);
                h.write(rec + 0x40, x2);
                let y2 = gy(h, 0x94);
                h.write(rec + 0x44, y2);
                let x3 = gs_coord(mirror(h, 0xc0, 0x90), [d1, d2, d3]);
                h.write(rec + 0x60, x3);
                let y3 = gy(h, 0xc4);
                h.write(rec + 0x64, y3);
                eax = h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, name, 1]);
            }
        }
    }
    // the original leaves its row pointer in `eax`
    let _ = eax;
    row0.wrapping_add(0x150 * 0x11)
}

// ---- the nine-part window frame (0x008f9280)

/// `TOMkpShapeXYWHUVC2(id, x, y, w, h, u0, v0, u1, v1, c0, c1)` (cdecl).
pub const SHAPE_UV_C2: u32 = 0x0090_3ee0;
/// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (cdecl).
pub const CROSS_BOX: u32 = 0x008f_4b20;
/// The frame's contents drawer `0x008f8d50(a, x, y, w, h, alpha)` (cdecl).
pub const FRAME_CONTENTS: u32 = 0x008f_8d50;
/// The frame part's width constant (`f32`), remapped for every part.
const PART: u32 = 0x00b4_1d8c;

/// `fistp qword` with the rounding control set to truncate, low word: the
/// value toward zero, or the indefinite integer's low word (0) out of
/// range.
fn fist_trunc(x: f64) -> u32 {
    #[allow(clippy::cast_precision_loss)] // 2^63, exact
    let limit = (1u64 << 63) as f64;
    if x.is_nan() || x >= limit || x < -limit {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)] // in range: its low word
        let v = x.trunc() as i64 as u32;
        v
    }
}

/// `remap(v)` through its `ST0`.
fn remap(h: &mut impl Host, f: u32, v: u32) -> f64 {
    h.call_sym_st0_f64(f, Conv::Cdecl, &[v])
}

/// How a bar of [`frame_shape9`] gets its height.
#[derive(Clone, Copy)]
enum BarH {
    /// `remapY2(c)`.
    Remap(u32),
    /// The frame's height as it is.
    Keep(u32),
}

/// How a bar gets its y.
#[derive(Clone, Copy)]
enum BarY {
    /// A value as it is (the frame's `y + h`, or `y`).
    Keep(u32),
    /// `remapY2(1) + y`.
    One,
    /// `(y + h) - remapY2(c)`.
    Below(u32),
}

/// The frame's place: `x`, `y`, its width, `x + width` and `y + height`
/// (both kept as `f64`).
#[derive(Clone, Copy)]
struct Place {
    x: u32,
    y: u32,
    w1: u32,
    right: f64,
    bottom: f64,
}

/// One bar of [`frame_shape9`]: the part (0 left, 1 middle, 2 right),
/// height, y and colours. Calls the remaps in the original's order
/// (height, width, y, x) and returns the box call's `eax` and its x.
fn bar(
    h: &mut impl Host,
    part: u32,
    bh: BarH,
    by: BarY,
    at: Place,
    c0: u32,
    c1: u32,
) -> (u32, u32) {
    let hh = match bh {
        BarH::Remap(c) => store(remap(h, REMAP_Y2, konst(h, c))),
        BarH::Keep(v) => pass(v),
    };
    let w = if part == 1 {
        store(sub(load(at.w1), remap(h, REMAP_X2, konst(h, 0x00b5_f384))))
    } else {
        store(remap(h, REMAP_X2, konst(h, PART)))
    };
    let yy = match by {
        BarY::Keep(v) => pass(v),
        BarY::One => store(add(remap(h, REMAP_Y2, 1.0f32.to_bits()), load(at.y))),
        BarY::Below(c) => store(sub(at.bottom, remap(h, REMAP_Y2, konst(h, c)))),
    };
    let xx = match part {
        0 => pass(at.x),
        1 => store(add(remap(h, REMAP_X2, konst(h, PART)), load(at.x))),
        _ => store(sub(at.right, remap(h, REMAP_X2, konst(h, PART)))),
    };
    (
        h.call_sym(CROSS_BOX, Conv::Cdecl, &[xx, yy, w, hh, c0, c1]),
        xx,
    )
}

/// `TODrawFrameShape9_008f9280(a, x, y, alpha)` (cdecl, floats): a
/// window frame from nine parts at `(x, y)`, its width
/// `remapX2(0x00b5eff4)` and height `remapY2(0x00b583d8)`. The top row
/// is three shapes (`TOMkpShapeXYWHUVC2(-4, ...)`: the left corner, the
/// stretched middle and the right corner, tinted `alpha << 24 |
/// 0x808080`); then four rows of three `TODrawCrossBoxXYWHC2` bars
/// (left, middle, right) whose alpha is `alpha / 128` (the unsigned
/// value through `f32`) times 38, 90 (through `_ftol2_sse`), 90 again
/// (white) and 128, the x87 truncating where the original sets the
/// rounding control; last the contents
/// (`0x008f8d50(a, x', y', w, h, alpha)`). Every part calls
/// `graphicUiRemapX2` / `Y2` in the original's order and stores its edges
/// as `f32`. Returns the contents call's `eax`.
///
/// `slots` is the argument words' address: the original computes its
/// temporaries in the `alpha` slot (the last right bar's x stays there)
/// and leaves the contents' x and y in the `x` and `y` slots.
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn frame_shape9(
    h: &mut impl Host,
    a: u32,
    x: u32,
    y: u32,
    alpha: u32,
    slots: Option<u32>,
) -> u32 {
    let w1 = store(remap(h, REMAP_X2, konst(h, 0x00b5_eff4)));
    let h1 = store(remap(h, REMAP_Y2, konst(h, 0x00b5_83d8)));
    let tint = (alpha << 24).wrapping_add(0x0080_8080);
    let t = store(div(
        mul(konst64(h, 0x00b4_1d58), load(h1)),
        konst64(h, 0x00b4_1d30),
    ));
    let k = konst(h, PART);
    let w = store(remap(h, REMAP_X2, k));
    let grey = 0x0080_8080;
    let id = 0xffff_fffc;
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[id, pass(x), pass(y), w, h1, 0, 0, k, t, grey, tint],
    );
    let u = mul(
        mul(load(w1), konst64(h, 0x00b4_1d68)),
        konst64(h, 0x00b4_1cc0),
    );
    let v = store(sub(u, konst64(h, 0x00b5_4d60)));
    let mid = store(sub(load(w1), remap(h, REMAP_X2, konst(h, 0x00b5_f384))));
    let x1 = store(add(remap(h, REMAP_X2, k), load(x)));
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[id, x1, pass(y), mid, h1, k, 0, v, t, tint, tint],
    );
    let right = add(load(w1), load(x));
    let w = store(remap(h, REMAP_X2, k));
    let x2 = store(sub(right, remap(h, REMAP_X2, k)));
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[id, x2, pass(y), w, h1, v, 0, store(u), t, tint, grey],
    );
    // the bars' alphas: alpha as an unsigned number through f32, / 128
    let mut af = f64::from(alpha.cast_signed());
    if alpha.cast_signed() < 0 {
        af = add(af, load(h.read(h.rebase(0x00b1_1f14))));
    }
    let s = mul(load(store(af)), konst64(h, 0x00b1_70b8));
    let bottom = add(load(h1), load(y));
    let at = Place {
        x,
        y,
        w1,
        right,
        bottom,
    };
    let rows = |c: u32| [(0, c), (c, c), (c, 0)];
    let a38 = fist_trunc(mul(s, konst64(h, 0x00b5_f1d8))) << 24;
    let below = store(bottom);
    for (part, (c0, c1)) in rows(a38).into_iter().enumerate() {
        bar(
            h,
            part as u32,
            BarH::Remap(0x00b9_2308),
            BarY::Keep(below),
            at,
            c0,
            c1,
        );
    }
    let s90 = mul(s, konst64(h, 0x00b5_8cd0));
    let a90 = ffx_bridge::x87::ftol(h, s90) << 24;
    for (part, (c0, c1)) in rows(a90).into_iter().enumerate() {
        bar(h, part as u32, BarH::Keep(h1), BarY::Keep(y), at, c0, c1);
    }
    let white = (fist_trunc(s90) << 24).wrapping_add(0x00ff_ffff);
    let rows_white = [(0x00ff_ffff, white), (white, white), (white, 0x00ff_ffff)];
    for (part, (c0, c1)) in rows_white.into_iter().enumerate() {
        bar(
            h,
            part as u32,
            BarH::Remap(0x00b9_2268),
            BarY::One,
            at,
            c0,
            c1,
        );
    }
    let a128 = fist_trunc(mul(s, konst64(h, 0x00b4_c170))) << 24;
    let mut last_x = 0;
    for (part, (c0, c1)) in rows(a128).into_iter().enumerate() {
        let (_, xx) = bar(
            h,
            part as u32,
            BarH::Remap(0x00b3_eb94),
            BarY::Below(0x00b3_eb94),
            at,
            c0,
            c1,
        );
        last_x = xx;
    }
    let hh = store(remap(h, REMAP_Y2, konst(h, 0x00b5_f2f8)));
    let ww = store(remap(h, REMAP_X2, konst(h, 0x00b5_ef68)));
    let yy = store(add(remap(h, REMAP_Y2, konst(h, 0x00b9_2308)), load(y)));
    let xx = store(add(remap(h, REMAP_X2, konst(h, 0x00b5_f348)), load(x)));
    if let Some(sl) = slots {
        h.write(sl + 4, xx);
        h.write(sl + 8, yy);
        h.write(sl + 0xc, last_x);
    }
    h.call_sym(FRAME_CONTENTS, Conv::Cdecl, &[a, xx, yy, ww, hh, alpha])
}
