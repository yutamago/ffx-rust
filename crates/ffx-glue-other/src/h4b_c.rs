//! List `h4-glue-other`, second agent (h4b), batch 3: the menus' cross
//! cursor and the message windows' selection cursor (x87).
//!
//! Floats follow the x87 at 53 bits ([`ffx_bridge::x87`]): `f64` in the
//! original's order, `f32` at each `fstp dword`; image constants through
//! `fld`/`fstp` are [`pass`]ed. `graphicUiRemapX2`/`Y2` and
//! `graphicGetTime` return their float in `ST0`.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{CI_COS, GET_TIME, REMAP_X2, REMAP_Y2, SHAPE_UV, int_f32, pass};

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

/// `graphicUiRemapX2(constant at sym)`: `ST0`.
fn remap_x(h: &mut impl Host, sym: u32) -> f64 {
    let v = konst(h, sym);
    h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v])
}

/// `graphicUiRemapY2(constant at sym)`: `ST0`.
fn remap_y(h: &mut impl Host, sym: u32) -> f64 {
    let v = konst(h, sym);
    h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v])
}

// ---- TkMn2DrawCrossCursor

/// `graphicFontGetScreenWH(&w, &h)` (cdecl; 1920 and 1080).
pub const FONT_SCREEN_WH: u32 = 0x0064_0db0;

/// The arrow of [`cross_cursor`]: `TOMkpShapeXYWHUV(shape, x, y,
/// remapX2(0x00b5eec0), remapY2(0x00b5ee90), u, v, u1, v1)`, the last four
/// image constants (`uv`), the height asked first.
fn arrow(h: &mut impl Host, shape: u32, x: u32, y: u32, uv: [u32; 4]) -> u32 {
    let [u0, v0, u1, v1] = uv.map(|c| konst(h, c));
    let height = store(remap_y(h, 0x00b5_ee90));
    let width = store(remap_x(h, 0x00b5_eec0));
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[shape, x, y, width, height, u0, v0, u1, v1],
    )
}

/// The screen scale of [`cross_cursor`]'s kinds 4 and 5: the font
/// screen's width through `f32` times `0x00b41cc0`, its height through
/// `f32` over `0x00b41d30` (both stored as `f32`); `(sx, sy)`.
fn screen_scale(h: &impl Host, w: u32, hh: u32) -> (u32, u32) {
    let sx = store(mul(load(int_f32(w.cast_signed())), konst64(h, 0x00b4_1cc0)));
    let sy = store(div(
        load(int_f32(hh.cast_signed())),
        konst64(h, 0x00b4_1d30),
    ));
    (sx, sy)
}

/// `TkMn2DrawCrossCursor(x, y, kind)` (cdecl): the menus' arrow cursor.
/// Kind 0: shape 200 at `x - remapX2(0x00b5eec0) + cos(time *
/// 0x00b922a8) * 0x00b2c5c8` (the wobble), `y - remapY2(0x00b3eb98)`.
/// Kinds 4 and 5: shape 400 at a screen position scaled from the font
/// screen (`graphicFontGetScreenWH`): `x / sx + 0x00b922f0` and `y / sy +
/// 0x00b54c70` (`f64` constants), kind 4 less `remapX2(0x00b304f8)` and
/// `remapY2(0x00b5ee64)`, kind 5 only `y` less `remapY2(0x00b5ee64)`;
/// each kind its own UV constants. Another kind draws nothing.
///
/// The original keeps its temporaries in its argument slots (`slots`, the
/// caller's first argument word): kind 0 the final x, y and the cosine;
/// kind 4 x, y and the screen width (an integer, `FontGetScreenWH` writes
/// it there); kind 5 y and the shape's x. `frame` is two words for the
/// font screen's size (the original's locals). Returns the shape call's
/// `eax`, or `kind` for another kind.
pub fn cross_cursor(
    h: &mut impl Host,
    frame: u32,
    x: u32,
    y: u32,
    kind: u32,
    slots: Option<u32>,
) -> u32 {
    match kind {
        0 => {
            let x0 = load(x);
            let rx = remap_x(h, 0x00b5_eec0);
            let mut xs = store(sub(x0, rx));
            let time = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
            let angle = mul(time, konst64(h, 0x00b9_22a8));
            let cos = h.rebase(CI_COS);
            let c = h.call_intrinsic(cos, angle).to_bits();
            xs = store(add(mul(load(c), konst64(h, 0x00b2_c5c8)), load(xs)));
            let y0 = load(y);
            let ry = remap_y(h, 0x00b3_eb98);
            let ys = store(sub(y0, ry));
            write_slots(h, slots, [Some(xs), Some(ys), Some(c)]);
            arrow(
                h,
                200,
                pass(xs),
                pass(ys),
                [0x00b5_1b9c, 0x00b6_5854, 0x00b6_57fc, 0x00b6_585c],
            )
        }
        4 => {
            // the width goes into the third argument slot
            let wp = slots.map_or(frame, |s| s + 8);
            let (w, hh) = font_screen(h, wp, frame + 4);
            let (sx, sy) = screen_scale(h, w, hh);
            let xa = store(add(div(load(x), load(sx)), konst64(h, 0x00b9_22f0)));
            let rx = remap_x(h, 0x00b3_04f8);
            let xs = store(sub(load(xa), rx));
            let ya = store(add(div(load(y), load(sy)), konst64(h, 0x00b5_4c70)));
            let ry = remap_y(h, 0x00b5_ee64);
            let ys = store(sub(load(ya), ry));
            write_slots(h, slots, [Some(xs), Some(ys), Some(w)]);
            arrow(
                h,
                400,
                pass(xs),
                pass(ys),
                [0x00b5_f0fc, 0x00b5_f0ec, 0x00b5_f108, 0x00b5_f100],
            )
        }
        5 => {
            let (w, hh) = font_screen(h, frame, frame + 4);
            let (sx, sy) = screen_scale(h, w, hh);
            let ya = store(add(div(load(y), load(sy)), konst64(h, 0x00b5_4c70)));
            let ry = remap_y(h, 0x00b5_ee64);
            let ys = store(sub(load(ya), ry));
            let [u0, v0, u1, v1] =
                [0x00b5_f108, 0x00b5_f0cc, 0x00b5_f0fc, 0x00b5_f0e8].map(|c| konst(h, c));
            let height = store(remap_y(h, 0x00b5_ee90));
            let width = store(remap_x(h, 0x00b5_eec0));
            let xa = store(add(div(load(x), load(sx)), konst64(h, 0x00b9_22f0)));
            write_slots(h, slots, [None, Some(ys), Some(xa)]);
            h.call_sym(
                SHAPE_UV,
                Conv::Cdecl,
                &[400, pass(xa), pass(ys), width, height, u0, v0, u1, v1],
            )
        }
        _ => kind,
    }
}

/// `graphicFontGetScreenWH(wp, hp)`: `(*wp, *hp)`.
fn font_screen(h: &mut impl Host, wp: u32, hp: u32) -> (u32, u32) {
    h.call_sym(FONT_SCREEN_WH, Conv::Cdecl, &[wp, hp]);
    (h.read(wp), h.read(hp))
}

/// The argument slot words [`cross_cursor`] leaves (x, y, kind's slot).
fn write_slots(h: &mut impl Host, slots: Option<u32>, words: [Option<u32>; 3]) {
    if let Some(s) = slots {
        for (k, w) in words.iter().enumerate() {
            if let Some(v) = w {
                h.write(s + 4 * k as u32, *v);
            }
        }
    }
}

// ---- TODrawSelectCursor

/// `g_MsgWindows`: the message windows by index.
pub const MSG_WINDOWS: u32 = 0x0186_76f0;
/// The windows' selection records (`0x01868a90`, by index).
pub const SELECTIONS: u32 = 0x0186_8a90;
/// The selection cursor's record (`0x025d09a0`, seven words: colour,
/// x0, y0, x1, y1, two words).
pub const SELECT_RECORD: u32 = 0x025d_09a0;
/// `TOIntToMes(value, out, digits, mode)` (cdecl).
pub const INT_TO_MES: u32 = 0x008b_8290;
/// `TOGetMessageWH(window, text, &w, &h, kind, flags)` (cdecl).
pub const GET_MESSAGE_WH: u32 = 0x008b_9650;
/// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (cdecl).
pub const CROSS_BOX: u32 = 0x008f_4b20;
/// `SetInvokedFromSelectCursor(flag)` (cdecl).
pub const SET_INVOKED: u32 = 0x008e_89c0;
/// `TkMn2DrawCrossCursor(x, y, kind)` (cdecl).
pub const CROSS_CURSOR: u32 = 0x008c_0670;

/// The current choice's (`i8 +2`) `i16` at `off` of selection `sel`
/// (8 bytes a choice), through `fild`.
fn choice_word(h: &impl Host, sel: u32, off: u32) -> f64 {
    let c = i32::from(h.read::<i8>(sel + 2)) as u32;
    f64::from(h.read::<i16>(sel.wrapping_add(c.wrapping_mul(8)).wrapping_add(off)))
}

/// `TODrawSelectCursor(window)` (cdecl): the selection cursor of message
/// window `window`. The record at `0x025d09a0` is cleared; a window of
/// kind 2 (`i16 +0x2c`) arms its selection (`+1` from 0 to 1). An armed
/// selection (`+1` == 1) fills the record: colour `0x80107070`, the
/// current choice's (`i8 +2`; `i16` x `+0x3c`, y `+0x3e`, width `+0x3a`
/// at 8 bytes per choice) box from the window's origin (`f32 +0x48`,
/// `+0x4c`), y less and the bottom more `0x00b92270`. A number-entry
/// window (`i16 +0x1c` == 2) instead draws a box over the current digit:
/// the value (`+0xc`, `i8 +5` digits) as text in the frame (`frame`, the
/// original's `ebp - 0x38`: `+0` height, `+4`/`+8` widths, `+0x14` the
/// text), measured up to and before the digit (`TOGetMessageWH`), the box
/// `TODrawCrossBoxXYWHC2(x, y - 0x00b922c0, w, 0x00b54f84, c, c)`; any
/// other the cross cursor at `(x, y - 0x00b4b468)` (kind 0, with the
/// select-cursor flag set around it). Returns the last call's `eax`
/// (unarmed: the window's or the selection's pointer, as the original
/// leaves it).
pub fn select_cursor(h: &mut impl Host, frame: u32, window: u32) -> u32 {
    let ebp = frame + 0x38;
    let rec = h.rebase(SELECT_RECORD);
    for k in 0..7 {
        h.write(rec + 4 * k, 0u32);
    }
    let windows = h.rebase(MSG_WINDOWS);
    let sels = h.rebase(SELECTIONS);
    let slot = |base: u32| base.wrapping_add(window.wrapping_mul(4));
    let win: u32 = h.read(slot(windows));
    if h.read::<u16>(win + 0x2c) == 2 {
        let sel: u32 = h.read(slot(sels));
        if h.read::<u8>(sel + 1) == 0 {
            h.write(sel + 1, 1u8);
        }
    }
    let sel: u32 = h.read(slot(sels));
    if h.read::<u8>(sel + 1) != 1 {
        // eax: the last record pointer the original loaded
        return if h.read::<u16>(win + 0x2c) == 2 {
            sel
        } else {
            win
        };
    }
    let win: u32 = h.read(slot(windows));
    h.write(rec, 0x8010_7070u32);
    // the first choice word also stays in the frame (`ebp - 0x34`)
    let c = i32::from(h.read::<i8>(sel + 2)) as u32;
    let first: i16 = h.read(sel.wrapping_add(c.wrapping_mul(8)).wrapping_add(0x3c));
    h.write(ebp - 0x34, i32::from(first));
    let x0 = store(add(choice_word(h, sel, 0x3c), load(h.read(win + 0x48))));
    h.write(rec + 4, x0);
    let k = konst64(h, 0x00b9_2270);
    let y0 = store(sub(
        add(choice_word(h, sel, 0x3e), load(h.read(win + 0x4c))),
        k,
    ));
    h.write(rec + 8, y0);
    let x1 = store(add(choice_word(h, sel, 0x3a), load(h.read(rec + 4))));
    h.write(rec + 0xc, x1);
    let y1 = store(add(k, load(h.read(rec + 8))));
    h.write(rec + 0x10, y1);
    if h.read::<u16>(win + 0x1c) == 2 {
        let text = ebp - 0x24;
        let digits = i32::from(h.read::<i8>(sel + 5)) as u32;
        let value: u32 = h.read(sel + 0xc);
        h.call_sym(INT_TO_MES, Conv::Cdecl, &[value, text, digits, 1]);
        let sel: u32 = h.read(slot(sels));
        let c = i32::from(h.read::<i8>(sel + 2)) as u32;
        h.write(text.wrapping_add(c).wrapping_add(1), 0u8);
        let kind = u32::from(h.read::<u8>(rec + 0x1a));
        h.call_sym(
            GET_MESSAGE_WH,
            Conv::Cdecl,
            &[window, text, ebp - 0x30, ebp - 0x38, kind, 0],
        );
        let sel: u32 = h.read(slot(sels));
        let c = i32::from(h.read::<i8>(sel + 2)) as u32;
        h.write(text.wrapping_add(c), 0u8);
        let kind = u32::from(h.read::<u8>(rec + 0x1a));
        h.call_sym(
            GET_MESSAGE_WH,
            Conv::Cdecl,
            &[window, text, ebp - 0x34, ebp - 0x38, kind, 0],
        );
        let sel: u32 = h.read(slot(sels));
        let win: u32 = h.read(slot(windows));
        let before: u32 = h.read(ebp - 0x34);
        let x = store(add(
            add(
                f64::from(h.read::<i16>(sel + 0x3c)),
                load(h.read(win + 0x48)),
            ),
            load(before),
        ));
        h.write(rec + 4, x);
        let y = store(add(
            f64::from(h.read::<i16>(sel + 0x3e)),
            load(h.read(win + 0x4c)),
        ));
        h.write(rec + 8, y);
        let bh = konst(h, 0x00b5_4f84);
        let w = store(sub(load(h.read(ebp - 0x30)), load(before)));
        let yy = store(sub(load(y), konst64(h, 0x00b9_22c0)));
        h.write(ebp - 0x30, yy);
        let c = 0x8000_8080;
        return h.call_sym(
            CROSS_BOX,
            Conv::Cdecl,
            &[pass(x), pass(yy), pass(w), bh, c, c],
        );
    }
    h.call_sym(SET_INVOKED, Conv::Cdecl, &[1]);
    let yy = store(sub(load(h.read(rec + 8)), konst64(h, 0x00b4_b468)));
    h.write(ebp - 0x30, yy);
    let x: u32 = h.read(rec + 4);
    h.call_sym(CROSS_CURSOR, Conv::Cdecl, &[pass(x), pass(yy), 0]);
    h.call_sym(SET_INVOKED, Conv::Cdecl, &[0])
}

// ---- TOBtlDrawTidusLimitWindow

/// `TOMkpBtlLimitSpecialGauge(x, y, w)` (cdecl).
pub const SPECIAL_GAUGE: u32 = 0x0090_1300;
/// `ftol2_sse`'s address (the port computes it: [`ffx_bridge::x87::ftol`]).
pub const FTOL2_SSE: u32 = 0x0094_97e0;
/// `MsGetRomBtlText(id, second)` (cdecl).
pub const ROM_BTL_TEXT: u32 = 0x0078_f8f0;
/// `ToGetBtlEasyFontWidth(text, &w, a, scale, b)` (cdecl).
pub const EASY_FONT_WIDTH: u32 = 0x0090_5290;
/// `TOMkpCrossExtMesFontLClut(a, text, x, y, b, scale, c)` (cdecl).
pub const EXT_MES_FONT: u32 = 0x0090_16b0;
/// The limit windows' words (`i16`): kind `+0`, then the gauge and
/// layout values at `+2`..`+0xe` (`0x0133d734`).
pub const LIMIT_WORDS: u32 = 0x0133_d734;

/// An `i16` word of the limit window block, sign-extended.
fn word(h: &impl Host, off: u32) -> i32 {
    i32::from(h.read::<i16>(h.rebase(LIMIT_WORDS) + off))
}

/// One of [`tidus_limit_window`]'s gradient boxes: `TODrawCrossBoxXYWHC2(x,
/// remapY2(0x00b5efa8), f32(w * 0x00b92220), remapY2(0x00b5ee6c), c0,
/// c1)`, the height asked first; `x` from the half width.
fn tidus_box(h: &mut impl Host, c0: u32, c1: u32, right: bool) -> u32 {
    let half = konst64(h, 0x00b9_2220);
    let height = store(remap_y(h, 0x00b5_ee6c));
    let w = store(mul(f64::from(word(h, 0xc)), half));
    let y = store(remap_y(h, 0x00b5_efa8));
    let x = if right {
        store(add(
            mul(f64::from(word(h, 0xc)), half),
            f64::from(word(h, 8)),
        ))
    } else {
        int_f32(word(h, 8))
    };
    h.call_sym(CROSS_BOX, Conv::Cdecl, &[x, y, pass(w), height, c0, c1])
}

/// `TOBtlDrawTidusLimitWindow()` (cdecl): while the limit kind (`i16
/// 0x0133d734`) is 3 or 4: the special gauge at (`+2`,
/// `ftol(remapY2(0x00b5ef98))`, width `+6`), two gradient boxes (left
/// half and right half of width `+0xc` from x `+8`, colours `0x80008aff`
/// and `0x8036d2ff` swapped between them), the shape 400 (x `+0xe` less
/// `remapX2(0x00b54f90)`, y `remapY2(0x00b5ef90)`, size `remapX2(0x00b583d8)`
/// x `remapY2(0x00b54cf8)`, UVs from image constants) and the text 0x3010
/// centred on 512 (`(0x00b41d38 - width) * 0x00b92220`, at
/// `remapY2(0x00b5ef10)`, scale `0x00b5ee10`). The text's width goes
/// through the frame word `width` (the original's `ebp - 0xc`).
///
/// Several arguments are words the original leaves on its stack from the
/// call before (the gauge's width, the boxes' colours, the shape's UVs);
/// they are passed as such. Returns the text call's `eax`, or the kind
/// less 3 when nothing is drawn.
pub fn tidus_limit_window(h: &mut impl Host, width: u32) -> u32 {
    let kind = word(h, 0).wrapping_sub(3) as u32;
    if kind > 1 {
        return kind;
    }
    let w = word(h, 6) as u32;
    let r = remap_y(h, 0x00b5_ef98);
    let y = ffx_bridge::x87::ftol(h, r);
    let x = word(h, 2) as u32;
    h.call_sym(SPECIAL_GAUGE, Conv::Cdecl, &[x, y, w]);
    tidus_box(h, 0x8000_8aff, 0x8036_d2ff, false);
    tidus_box(h, 0x8036_d2ff, 0x8000_8aff, true);
    let x0 = f64::from(word(h, 0xe));
    let [u0, v0, u1, v1] =
        [0x00b5_ee00, 0x00b5_edd8, 0x00b5_ee08, 0x00b5_ede8].map(|c| konst(h, c));
    let height = store(remap_y(h, 0x00b5_4cf8));
    let wd = store(remap_x(h, 0x00b5_83d8));
    let y = store(remap_y(h, 0x00b5_ef90));
    let rx = remap_x(h, 0x00b5_4f90);
    let x = store(sub(x0, rx));
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[400, pass(x), y, wd, height, u0, v0, u1, v1],
    );
    let text = h.call_sym(ROM_BTL_TEXT, Conv::Cdecl, &[0x3010, 0]);
    limit_caption(h, text, width)
}

/// The limit windows' caption: `text` centred on 512 (its width through
/// the frame word `width`: `ToGetBtlEasyFontWidth(text, width, 0, scale,
/// 1)`), `TOMkpCrossExtMesFontLClut(0, text, (0x00b41d38 - width) *
/// 0x00b92220, remapY2(0x00b5ef10), 0, scale, 1)`, scale `0x00b5ee10`.
/// Returns the text call's `eax`.
fn limit_caption(h: &mut impl Host, text: u32, width: u32) -> u32 {
    let one = 1.0f32.to_bits();
    let scale = konst(h, 0x00b5_ee10);
    h.call_sym(EASY_FONT_WIDTH, Conv::Cdecl, &[text, width, 0, scale, one]);
    let scale = konst(h, 0x00b5_ee10);
    let y = store(remap_y(h, 0x00b5_ef10));
    let half = mul(
        sub(konst64(h, 0x00b4_1d38), load(h.read(width))),
        konst64(h, 0x00b9_2220),
    );
    let x = store(half);
    h.call_sym(
        EXT_MES_FONT,
        Conv::Cdecl,
        &[0, text, pass(x), y, 0, scale, one],
    )
}

// ---- TOBtlDrawLuluLimitWindow

/// `TOMkpBtlLimitLuluMidareCounter(count, x, y, colour)` (cdecl).
pub const LULU_COUNTER: u32 = 0x0090_1280;
/// `TODrawCrossBoxXYWHC2FW(x, y, w, h, c0, c1, fade)` (cdecl).
pub const CROSS_BOX_FW: u32 = 0x008f_4c40;
/// `graphicGetLocTextWithLength(id, out, size)` (cdecl).
pub const LOC_TEXT: u32 = 0x0064_12f0;
/// The battle's control mode (`getCurrentControlMode`: 0 or 1).
pub const CONTROL_MODE: u32 = 0x00c5_8ef0;

/// The Lulu window's box x: `0x00b5ee50 - remapX2(0x00b92318)`.
fn lulu_x(h: &mut impl Host) -> u32 {
    let r = remap_x(h, 0x00b9_2318);
    store(sub(konst64(h, 0x00b5_ee50), r))
}

/// `TOBtlDrawLuluLimitWindow()` (cdecl): while the Lulu limit kind (`i16
/// 0x0133d708`) is 3 or 4: the hit counter (`u8 +0x17` at `+0x10 + 10`,
/// `+0x12 - 8`), a black box (width `remapX2(0x00b4ae10) + i16 +6`), the
/// gauge box over it (width that times `u8 +0x17 * 0x00b921e0`, colours
/// the words `+8`/`+0xc`, the fade `ftol` of that width), the special
/// gauge (0xa0, `ftol(remapY2(0x00b5ef98))`, `+6`) and the caption: text
/// 0x3012 with a pad (control mode 1), the localised text 0x2e8 into the
/// frame's buffer with the keyboard (0). Offsets from `0x0133d708`.
///
/// `frame` is the original's `ebp - 0x110`: `+0` the caption's width,
/// `+0xc` the 0x100-byte text buffer. The original takes the text from
/// the never-written width word for another control mode, which the game
/// never sets (refs.py 0x00c58ef0: 4 refs; its only writer
/// `TOBtlCtrlLuluLimitWindow` stores `getCurrentControlMode`'s 0/1 and the
/// image holds 1); the port reads that word from its zeroed frame
/// (brief 2026-10-24). Returns the caption call's `eax`, or the kind less
/// 3 when nothing is drawn.
pub fn lulu_limit_window(h: &mut impl Host, frame: u32) -> u32 {
    let base = h.rebase(0x0133_d708);
    let kind = i32::from(h.read::<i16>(base)).wrapping_sub(3) as u32;
    if kind > 1 {
        return kind;
    }
    let count = u32::from(h.read::<u8>(base + 0x17));
    let x = (i32::from(h.read::<i16>(base + 0x10)) + 10) as u32;
    let y = (i32::from(h.read::<i16>(base + 0x12)) - 8) as u32;
    h.call_sym(LULU_COUNTER, Conv::Cdecl, &[count, x, y, 0x8080_8080]);
    // the black box
    let height = store(remap_y(h, 0x00b5_ee6c));
    let r = remap_x(h, 0x00b4_ae10);
    let w = store(add(r, f64::from(h.read::<i16>(base + 6))));
    let yv = store(remap_y(h, 0x00b5_efa8));
    let xv = lulu_x(h);
    let black = 0x8000_0000;
    h.call_sym(
        CROSS_BOX,
        Conv::Cdecl,
        &[xv, yv, pass(w), height, black, black],
    );
    // the gauge box
    let r = remap_x(h, 0x00b4_ae10);
    let fade = ffx_bridge::x87::ftol(h, add(r, f64::from(h.read::<i16>(base + 6))));
    let (c0, c1): (u32, u32) = (h.read(base + 8), h.read(base + 0xc));
    let height = store(remap_y(h, 0x00b5_ee6c));
    let r = remap_x(h, 0x00b4_ae10);
    let full = add(r, f64::from(h.read::<i16>(base + 6)));
    let ratio = mul(
        f64::from(h.read::<u8>(base + 0x17)),
        konst64(h, 0x00b9_21e0),
    );
    let w = store(mul(full, ratio));
    let yv = store(remap_y(h, 0x00b5_efa8));
    let xv = lulu_x(h);
    h.call_sym(
        CROSS_BOX_FW,
        Conv::Cdecl,
        &[xv, yv, pass(w), height, c0, c1, fade],
    );
    let gw = i32::from(h.read::<i16>(base + 6)) as u32;
    let r = remap_y(h, 0x00b5_ef98);
    let gy = ffx_bridge::x87::ftol(h, r);
    h.call_sym(SPECIAL_GAUGE, Conv::Cdecl, &[0xa0, gy, gw]);
    let mode: u32 = h.read(h.rebase(CONTROL_MODE));
    let text = match mode {
        1 => h.call_sym(ROM_BTL_TEXT, Conv::Cdecl, &[0x3012, 0]),
        0 => {
            h.call_sym(LOC_TEXT, Conv::Cdecl, &[0x2e8, frame + 0xc, 0x100]);
            frame + 0xc
        }
        // junk read: [ebp-0x110] never written; 0 (brief 2026-10-24)
        _ => h.read(frame),
    };
    limit_caption(h, text, frame)
}

// ---- TOBtlDrawCrossLimitGauge

/// `MsGetChrLimitGaugeMax(chr)` (cdecl).
pub const LIMIT_GAUGE_MAX: u32 = 0x0079_5590;
/// `MsGetChrLimitGauge(chr)` (cdecl).
pub const LIMIT_GAUGE: u32 = 0x0079_5550;
/// `cosf(x)` (cdecl, `f32` argument, `ST0`).
pub const COSF: u32 = 0x004c_eac0;
/// `TOMkpShapeXYWHUVC2(shape, x, y, w, h, u, v, u1, v1, c0, c1)` (cdecl).
pub const SHAPE_UV_C2: u32 = 0x0090_3ee0;

/// `a <= b` as `fcompp` and `test ah, 0x41; jp` decide it: false for NaN.
fn le(a: f64, b: f64) -> bool {
    a <= b
}

/// `TOBtlDrawCrossLimitGauge(chr, x, y, w, h)` (cdecl, floats): a
/// character's limit gauge. With the gauge full (max <= gauge, as `f32`
/// of the two integers) a pulsing frame first: shape -1 with the alpha
/// `ftol2((cosf(time * 0x00b2c5c8) * 0x00b412f8 + 0x00b5ee40))` (the x87's
/// truncating `fistp`) over `0x808080`. Then a black box (`x -
/// remapX2(0x00b92268)`, `y - remapY2(0x00b92268)`, `w +
/// remapX2(0x00b922f8)`, `h + remapY2(0x00b922f8)`), a grey one inside
/// (`x + remapX2(1)`, `y + remapY2(1)`, `w - remapX2(0x00b92268)`, `h -
/// remapY2(0x00b92268)`), and the filled part (width scaled by gauge /
/// max): blue when full, a blue gradient when the gauge is above 0
/// (ordered compares; NaN draws nothing).
///
/// The original computes the filled part in its last argument slot
/// (`slots`, the caller's first argument word): it ends holding the x.
pub fn cross_limit_gauge(
    h: &mut impl Host,
    chr: u32,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
    slots: Option<u32>,
) {
    let max = h.call_sym(LIMIT_GAUGE_MAX, Conv::Cdecl, &[chr]);
    let mf = int_f32(max.cast_signed());
    let gauge = h.call_sym(LIMIT_GAUGE, Conv::Cdecl, &[chr]);
    let gf = int_f32(gauge.cast_signed());
    let full = le(load(mf), load(gf));
    let partial = 0.0 < load(gf);
    let slot = slots.map(|s| s + 0x10);
    gauge_boxes(h, full, partial, (gf, mf), [x, y, w, hh], false, slot);
}

/// The limit gauges' drawing ([`cross_limit_gauge`],
/// [`status_limit_gauge`]): the pulsing frame when `full`, the black and
/// grey boxes, the filled part (width times `ratio.0 / ratio.1`, `f32`
/// bits) when `full` (blue) or `partial` (a gradient). `rect` is x, y, w,
/// h (`f32` bits). The pulse's cosine comes from `cosf` of the angle as
/// `f32`, or with `intrinsic` from `_CIcos` of the `f64` angle (stored as
/// `f32`). The filled part's x ends in the argument slot `slot`.
fn gauge_boxes(
    h: &mut impl Host,
    full: bool,
    partial: bool,
    ratio: (u32, u32),
    rect: [u32; 4],
    intrinsic: bool,
    slot: Option<u32>,
) {
    let [x, y, w, hh] = rect;
    if full {
        let time = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let angle = mul(time, konst64(h, 0x00b2_c5c8));
        let c = if intrinsic {
            let cos = h.rebase(CI_COS);
            load(h.call_intrinsic(cos, angle).to_bits())
        } else {
            h.call_sym_st0_f64(COSF, Conv::Cdecl, &[store(angle)])
        };
        let a = add(mul(c, konst64(h, 0x00b4_12f8)), konst64(h, 0x00b5_ee40));
        let colour = ffx_bridge::x87::ftol2(a)
            .wrapping_shl(24)
            .wrapping_add(0x80_8080);
        let [u0, v0, u1, v1] =
            [0x00b5_eff8, 0x00b5_ef38, 0x00b5_f01c, 0x00b5_ef3c].map(|k| konst(h, k));
        let fh = store(add(remap_y(h, 0x00b9_2314), load(hh)));
        let fw = store(add(remap_x(h, 0x00b4_7de8), load(w)));
        let ry = remap_y(h, 0x00b9_22f8);
        let fy = store(sub(load(y), ry));
        let rx = remap_x(h, 0x00b9_2308);
        let fx = store(sub(load(x), rx));
        h.call_sym(
            SHAPE_UV_C2,
            Conv::Cdecl,
            &[0xffff_ffff, fx, fy, fw, fh, u0, v0, u1, v1, colour, colour],
        );
    }
    let black = 0x8000_0000;
    let bh = store(add(remap_y(h, 0x00b9_22f8), load(hh)));
    let bw = store(add(remap_x(h, 0x00b9_22f8), load(w)));
    let ry = remap_y(h, 0x00b9_2268);
    let by = store(sub(load(y), ry));
    let rx = remap_x(h, 0x00b9_2268);
    let bx = store(sub(load(x), rx));
    h.call_sym(CROSS_BOX, Conv::Cdecl, &[bx, by, bw, bh, black, black]);
    let grey = 0x401a_1a1a;
    let ry = remap_y(h, 0x00b9_2268);
    let gh = store(sub(load(hh), ry));
    let rx = remap_x(h, 0x00b9_2268);
    let gw = store(sub(load(w), rx));
    let one = 1.0f32.to_bits();
    let gy = store(add(
        h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[one]),
        load(y),
    ));
    let gx = store(add(
        h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[one]),
        load(x),
    ));
    h.call_sym(CROSS_BOX, Conv::Cdecl, &[gx, gy, gw, gh, grey, grey]);
    let (c0, c1) = if full {
        (0x8000_53ff, 0x8000_53ff)
    } else if partial {
        (0x8000_d2ff, 0x8000_8aff)
    } else {
        return;
    };
    let ry = remap_y(h, 0x00b9_2268);
    let fh = store(sub(load(hh), ry));
    let rx = remap_x(h, 0x00b9_2268);
    let fw = store(div(mul(sub(load(w), rx), load(ratio.0)), load(ratio.1)));
    let fy = store(add(
        h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[one]),
        load(y),
    ));
    let fx = store(add(
        h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[one]),
        load(x),
    ));
    if let Some(s) = slot {
        h.write(s, fx);
    }
    h.call_sym(CROSS_BOX, Conv::Cdecl, &[fx, fy, fw, fh, c0, c1]);
}

// ---- TOBtlDrawStatusLimitGauge

/// The status window's limit gauge maximum (`i16 0x025d0aa4`).
pub const STATUS_GAUGE_MAX: u32 = 0x025d_0aa4;

/// `TOBtlDrawStatusLimitGauge(gauge, x, y, w, h)` (cdecl, the rest
/// floats): [`cross_limit_gauge`]'s drawing for a gauge value against the
/// maximum `i16 0x025d0aa4`: full when `gauge >= max`, partial when
/// `gauge > 0` (signed integers). The filled part's x ends in the fifth
/// argument slot (`slots`).
pub fn status_limit_gauge(
    h: &mut impl Host,
    gauge: u32,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
    slots: Option<u32>,
) {
    let max = i32::from(h.read::<i16>(h.rebase(STATUS_GAUGE_MAX)));
    let g = gauge.cast_signed();
    let ratio = (int_f32(g), int_f32(max));
    let slot = slots.map(|s| s + 0x10);
    gauge_boxes(h, g >= max, g > 0, ratio, [x, y, w, hh], false, slot);
}

// ---- TkMn2DrawCrossLimitParam

/// `TkMn2DrawCrossLimitParam(x, y, w, h, value, max)` (cdecl, floats and
/// two integers): the menus' limit gauge, [`cross_limit_gauge`]'s drawing
/// with the pulse from `_CIcos`; full when `value >= max`, partial when
/// `value > 0`. The filled part's x ends in the fourth argument slot
/// (`slots`).
#[allow(clippy::too_many_arguments)]
pub fn cross_limit_param(
    h: &mut impl Host,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
    value: u32,
    max: u32,
    slots: Option<u32>,
) {
    let (v, m) = (value.cast_signed(), max.cast_signed());
    let ratio = (int_f32(v), int_f32(m));
    let slot = slots.map(|s| s + 0xc);
    gauge_boxes(h, v >= m, v > 0, ratio, [x, y, w, hh], true, slot);
}

// ---- the battle end's legend weapon notice

/// `g_BtlEndFadeNotice` (`i16`, 0..0x1000).
pub const END_FADE: u32 = 0x0186_a170;
/// `g_BtlEndLegendText`.
pub const END_TEXT: u32 = 0x025c_c590;
/// `TOMenuOpenPktBuffTmp()` (cdecl).
pub const OPEN_PKT_TMP: u32 = 0x008b_ef30;
/// `TOMenuDrawKickTmp()` (cdecl).
pub const DRAW_KICK_TMP: u32 = 0x008b_ea20;
/// `TOGetMessageWH1612_008b6cb0(kind, text, &w, &h, scale, extra, a, b)`.
pub const MESSAGE_WH_1612: u32 = 0x008b_6cb0;
/// `TODrawMenuPlateXYWHType(x, y, w, h, kind)` (cdecl).
pub const MENU_PLATE: u32 = 0x008f_5f70;
/// `TODrawScissorXYWH(x, y, w, h)` (cdecl).
pub const SCISSOR_XYWH: u32 = 0x008f_9230;

/// The x87's `fistp` under a truncating control word: the low 32 bits of
/// the 64-bit truncation (indefinite gives 0).
fn trunc(x: f64) -> u32 {
    ffx_bridge::x87::ftol2(x)
}

/// An unsigned integer as the original converts it: `fild` of the signed
/// word, `2^32` (the `f32` at `0x00b11f14`) added when negative, stored.
fn u2f(h: &impl Host, n: u32) -> u32 {
    let v = f64::from(n.cast_signed());
    if n.cast_signed() < 0 {
        store(add(v, load(h.read(h.rebase(0x00b1_1f14)))))
    } else {
        store(v)
    }
}

/// `TkBtlEndDrawLegendNotice_008bc100()` (cdecl): while the fade
/// `g_BtlEndFadeNotice` is open and there is a text: the text's size
/// (`TOGetMessageWH1612_008b6cb0(7, text, &w, &h, 0x00b5ee10, 1, 0, 0)`
/// into the frame, `frame` the original's `ebp - 0xc`: `+0` w, `+4` h),
/// padded (`0x00b65648`, `0x00b922b0`); the text's corner centred on 512 x
/// 416 (`0x00b41d38`, `0x00b41d30`, times `0x00b92220`, truncated); the
/// plate scaled by the fade (`fade * w * 0x00b50190`, `0x00b50190 * (h *
/// 0x00b515c8)`) and centred alike, drawn as plate 10 at unsigned-to-float
/// corners, the scissor to it (`ftol` of its size), the text at its corner
/// plus (0x12, 4), the full scissor and the packet kick. The frame ends
/// with the plate's size. Returns the kick's `eax` (nothing drawn: none
/// the callers read).
pub fn legend_notice(h: &mut impl Host, frame: u32) -> u32 {
    let fade = h.read::<i16>(h.rebase(END_FADE));
    let text: u32 = h.read(h.rebase(END_TEXT));
    if fade == 0 || text == 0 {
        return 0;
    }
    h.call_sym(OPEN_PKT_TMP, Conv::Cdecl, &[]);
    h.write(frame, 0u32);
    h.write(frame + 4, 0u32);
    let scale = konst(h, 0x00b5_ee10);
    let one = 1.0f32.to_bits();
    let text: u32 = h.read(h.rebase(END_TEXT));
    h.call_sym(
        MESSAGE_WH_1612,
        Conv::Cdecl,
        &[7, text, frame, frame + 4, scale, one, 0, 0],
    );
    let w = store(add(load(h.read(frame)), konst64(h, 0x00b6_5648)));
    let hh = store(add(load(h.read(frame + 4)), konst64(h, 0x00b9_22b0)));
    let a = konst64(h, 0x00b4_1d38);
    let b = konst64(h, 0x00b4_1d30);
    let half = konst64(h, 0x00b9_2220);
    let x0 = trunc(mul(sub(a, load(w)), half));
    let y0 = trunc(mul(sub(b, load(hh)), half));
    let fade = f64::from(h.read::<i16>(h.rebase(END_FADE)));
    let c = konst64(h, 0x00b5_0190);
    let pw = store(mul(mul(fade, load(w)), c));
    let ph = store(mul(c, mul(load(hh), konst64(h, 0x00b5_15c8))));
    h.write(frame, pw);
    h.write(frame + 4, ph);
    let x1 = trunc(mul(sub(a, load(pw)), half));
    let y1 = trunc(mul(half, sub(b, load(ph))));
    let (fx, fy) = (u2f(h, x1), u2f(h, y1));
    h.call_sym(
        MENU_PLATE,
        Conv::Cdecl,
        &[pass(fx), pass(fy), pass(pw), pass(ph), 10],
    );
    let sh = ffx_bridge::x87::ftol(h, load(ph));
    let sw = ffx_bridge::x87::ftol(h, load(pw));
    h.call_sym(SCISSOR_XYWH, Conv::Cdecl, &[x1, y1, sw, sh]);
    let ty = u2f(h, y0.wrapping_add(4));
    let tx = u2f(h, x0.wrapping_add(0x12));
    let scale = konst(h, 0x00b5_ee10);
    let text: u32 = h.read(h.rebase(END_TEXT));
    h.call_sym(
        EXT_MES_FONT,
        Conv::Cdecl,
        &[7, text, pass(tx), pass(ty), 0, scale, one],
    );
    h.call_sym(SCISSOR_XYWH, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    h.call_sym(DRAW_KICK_TMP, Conv::Cdecl, &[])
}

// ---- TkMn2DrawCrossCharFace

/// `TOGetShapTextureName(id)` (cdecl).
pub const SHAPE_TEXTURE_NAME: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl).
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `AtelGetAlbhedRikku()` (cdecl).
pub const ALBHED_RIKKU: u32 = 0x0086_a7e0;
/// `graphicDrawUIElement(record, texture, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// `g_TkMn2FontShade`: four words 8 bytes apart (`0x023cc458`..`0x023cc470`).
pub const FONT_SHADE: u32 = 0x023c_c458;

/// `TkMn2DrawCrossCharFace(face, x, y, w, h)` (cdecl, floats): a face
/// cell as a `graphicDrawUIElement` record in the frame. Faces 0..=6
/// come from image 0x2ed0 (4 a row; face 6 is 7 for Al Bhed Rikku,
/// `AtelGetAlbhedRikku`), the others from 0x2ed4 (`face - 8`, 5 a row;
/// signed remainders), 100 x 100 cells as UVs over the image's size
/// (`TOGetImageWH` into the frame). The record: `x`, `y`, the cell's
/// u0/v0, the four shade words (`0x023cc470`, `468`, `460`, `458`), `x +
/// w`, `y + h`, u1/v1, the shades again, its last two words 0; the words
/// between stay as they are (the original never writes them).
///
/// `frame` is the original's `ebp - 0xb8`: `+0xc` the image height, `+0x10`
/// its width, `+0x1c` the record (0x98 bytes). Returns the draw's `eax`.
pub fn cross_char_face(
    h: &mut impl Host,
    frame: u32,
    face: u32,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
) -> u32 {
    let ebp = frame + 0xb8;
    let shade: [u32; 4] = [0x18, 0x10, 0x8, 0].map(|k| h.read(h.rebase(FONT_SHADE) + k));
    let rec = ebp - 0x9c;
    h.write(ebp - 0xc, 0u32);
    h.write(ebp - 8, 0u32);
    let (img, mut face) = if face <= 6 {
        (0x2ed0, face)
    } else {
        (0x2ed4, face)
    };
    let texture = h.call_sym(SHAPE_TEXTURE_NAME, Conv::Cdecl, &[img]);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[img, ebp - 0xa8, ebp - 0xac]);
    let (col, row) = if img == 0x2ed0 {
        if face == 6 && h.call_sym(ALBHED_RIKKU, Conv::Cdecl, &[]) == 1 {
            face = 7;
        }
        let k = face.cast_signed();
        (
            k.wrapping_rem(4).wrapping_mul(100),
            (k / 4).wrapping_mul(100),
        )
    } else {
        let k = face.wrapping_sub(8).cast_signed();
        (
            k.wrapping_rem(5).wrapping_mul(100),
            (k / 5).wrapping_mul(100),
        )
    };
    let iw = load(h.read(ebp - 0xa8));
    let ih = load(h.read(ebp - 0xac));
    let u0 = store(div(load(int_f32(col)), iw));
    let v0 = store(div(load(int_f32(row)), ih));
    let x1 = store(add(load(x), load(w)));
    let y1 = store(add(load(y), load(hh)));
    let iw = load(h.read(ebp - 0xa8));
    let ih = load(h.read(ebp - 0xac));
    let u1 = store(div(load(int_f32(col.wrapping_add(100))), iw));
    let v1 = store(div(load(int_f32(row.wrapping_add(100))), ih));
    let head = [
        pass(x),
        pass(y),
        u0,
        v0,
        shade[0],
        shade[1],
        shade[2],
        shade[3],
    ];
    let tail = [x1, y1, u1, v1, shade[0], shade[1], shade[2], shade[3]];
    for (k, v) in head.iter().chain(tail.iter()).enumerate() {
        h.write(rec + 4 * k as u32, *v);
    }
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, texture, 1, 0, 0])
}

// ---- Iggy's mouse event queue

/// `PInput_GetDevice(type, n)` (cdecl).
pub const GET_DEVICE: u32 = 0x0062_3c90;

/// A free event record of the queue at `q` (`+0x74` the count, five
/// 0x14-byte records from `+0x10`), its link cleared; 0 when full (the
/// original then writes through 0 and faults).
fn queue_take(h: &mut impl Host, q: u32) -> u32 {
    let n: i32 = h.read(q + 0x74);
    if n >= 5 {
        return 0;
    }
    let e = q
        .wrapping_add(0x10)
        .wrapping_add((n as u32).wrapping_mul(0x14));
    h.write(q + 0x74, n.wrapping_add(1));
    h.write(e + 0x10, 0u32);
    e
}

/// Appends record `e` to the queue's list (`+0xc` the head, `+0x10`
/// links). The original's walk only looks two records deep: with three
/// in the list it spins forever, and so does the port.
fn queue_append(h: &mut impl Host, q: u32, e: u32) {
    let head: u32 = h.read(q + 0xc);
    if head == 0 {
        h.write(q + 0xc, e);
        h.write(e + 0x10, 0u32);
        return;
    }
    let second: u32 = h.read(head + 0x10);
    let link = if second == 0 {
        head + 0x10
    } else {
        if h.read::<u32>(second + 0x10) != 0 {
            // `test eax, eax; jne` back to itself: the game hangs here
            loop {
                core::hint::spin_loop();
            }
        }
        second + 0x10
    };
    h.write(e + 0x10, 0u32);
    h.write(link, e);
}

/// `IggyMouseQueue_update_00a25ab0(this)` (thiscall): the mouse's events
/// for Iggy this frame. The queue is emptied; the mouse
/// (`PInput_GetDevice(0, 0)`) gives its buttons 0xb, 0xc, 0xd (vtable
/// `+0xc`; the first and third swapped when `+0x78` is set) and x, y and
/// the wheel (`+0x10`, floats in `ST0`). A move (6, x, y) always; for
/// buttons 0 and 2 (not 1) a press (2 k) or release (2 k + 1) against
/// the bytes `+0..+2` kept from the last frame, with x and y; a wheel
/// event (7) when the wheel is not 0 (NaN too). The three button bytes
/// are kept.
pub fn mouse_queue_update(h: &mut impl Host, q: u32) {
    h.write(q + 0x74, 0u32);
    h.write(q + 0xc, 0u32);
    let mouse = h.call_sym(GET_DEVICE, Conv::Cdecl, &[0, 0]);
    let b0 = button_state(h, mouse, 0xb);
    let b1 = button_state(h, mouse, 0xc);
    let b2 = button_state(h, mouse, 0xd);
    let mut b = [b0, b1, b2];
    if h.read::<u8>(q + 0x78) != 0 {
        b.swap(0, 2);
    }
    let x = store(axis(h, mouse, 1));
    let y = store(axis(h, mouse, 2));
    let wheel = store(axis(h, mouse, 3));
    let e = queue_take(h, q);
    h.write(e, 6u32);
    h.write(e + 4, pass(x));
    h.write(e + 8, pass(y));
    queue_append(h, q, e);
    for k in [0u32, 2] {
        let before = h.read::<u8>(q + k);
        let now = b[k as usize];
        let kind = match (before, now) {
            (0, 1) => 2 * k,
            (1, 0) => 2 * k + 1,
            _ => continue,
        };
        let e = queue_take(h, q);
        h.write(e + 4, pass(x));
        h.write(e, kind);
        h.write(e + 8, pass(y));
        queue_append(h, q, e);
    }
    // `fucomp` and `test ah, 0x44; jnp`: only an equal 0 skips it
    if load(wheel) != 0.0 {
        let e = queue_take(h, q);
        h.write(e + 0xc, pass(wheel));
        h.write(e, 7u32);
        queue_append(h, q, e);
    }
    for (k, v) in b.iter().enumerate() {
        h.write(q + k as u32, *v);
    }
}

/// The mouse's button `k` (vtable `+0xc`, thiscall): its low byte.
fn button_state(h: &mut impl Host, mouse: u32, k: u32) -> u8 {
    let f: u32 = h.read(h.read::<u32>(mouse) + 0xc);
    h.call(f, Conv::Thiscall(mouse), &[k]) as u8
}

/// The mouse's axis `k` (vtable `+0x10`, thiscall, `ST0`).
fn axis(h: &mut impl Host, mouse: u32, k: u32) -> f64 {
    let f: u32 = h.read(h.read::<u32>(mouse) + 0x10);
    h.call_st0_f64(f, Conv::Thiscall(mouse), &[k])
}

// ---- Iggy's mouse event dispatch

/// `AddrOfGlobal01a858cc_00a3d7c0()`: the menus' critical section.
pub const MENU_LOCK: u32 = 0x00a3_d7c0;
/// `GetGlobal01984c68_00a25a30()`: the mouse event queue.
pub const MOUSE_QUEUE: u32 = 0x00a2_5a30;
/// `0x00a25a40` (thiscall): the queue's first event (`+0xc`).
pub const QUEUE_FIRST: u32 = 0x00a2_5a40;
/// Import slots: `EnterCriticalSection`, `LeaveCriticalSection` (Win32,
/// stdcall) and Iggy's event makers (stdcall): none, mouse button, move,
/// wheel.
pub const ENTER_CS: Global<u32> = Global::new(0x00b0_c154);
pub const LEAVE_CS: Global<u32> = Global::new(0x00b0_c158);
pub const MAKE_EVENT_NONE: Global<u32> = Global::new(0x00b0_c840);
pub const MAKE_EVENT_MOVE: Global<u32> = Global::new(0x00b0_c844);
pub const MAKE_EVENT_BUTTON: Global<u32> = Global::new(0x00b0_c848);
pub const MAKE_EVENT_WHEEL: Global<u32> = Global::new(0x00b0_c84c);

/// `IggyMenuManager_dispatchMouse_00a26ce0(this)` (thiscall; `this` the
/// menu manager): under the menus' lock, each queued mouse event (list
/// from `0x00a25a40` on the queue, links `+0x10`) made into an Iggy
/// event in the frame (`_IggyMakeEventNone`, then by type: buttons 0..5
/// as Iggy buttons 1, 2, 5, 6, 3, 4; 6 a move to `ftol` x, y; 7 a wheel
/// of `u16(ftol(wheel * 0x00b5f208))`) and handed to every menu
/// (`+0xc` array, `+8` count's low 31 bits) through vtable `+4`
/// (thiscall(event, result)); then every menu's vtable `+0`.
///
/// `frame` is the original's `ebp - 0x28`: `+0` the result (0xc bytes),
/// `+0xc` the event. Returns `LeaveCriticalSection`'s `eax`.
pub fn dispatch_mouse(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let ev = frame + 0xc;
    let lock = h.call_sym(MENU_LOCK, Conv::Cdecl, &[]);
    let f = ENTER_CS.get(h);
    h.call(f, Conv::Stdcall, &[lock]);
    let q = h.call_sym(MOUSE_QUEUE, Conv::Cdecl, &[]);
    let mut e = h.call_sym(QUEUE_FIRST, Conv::Thiscall(q), &[]);
    while e != 0 {
        let f = MAKE_EVENT_NONE.get(h);
        h.call(f, Conv::Stdcall, &[ev]);
        let kind: u32 = h.read(e);
        match kind {
            0..=5 => {
                let button = [1, 2, 5, 6, 3, 4][kind as usize];
                let f = MAKE_EVENT_BUTTON.get(h);
                h.call(f, Conv::Stdcall, &[ev, button]);
            }
            6 => {
                let y = ffx_bridge::x87::ftol(h, load(h.read(e + 8)));
                let x = ffx_bridge::x87::ftol(h, load(h.read(e + 4)));
                let f = MAKE_EVENT_MOVE.get(h);
                h.call(f, Conv::Stdcall, &[ev, x, y]);
            }
            7 => {
                let k: f64 = h.read(h.rebase(0x00b5_f208));
                let v = ffx_bridge::x87::ftol(h, mul(load(h.read(e + 0xc)), k)) & 0xffff;
                let f = MAKE_EVENT_WHEEL.get(h);
                h.call(f, Conv::Stdcall, &[ev, v]);
            }
            _ => {}
        }
        each_menu(h, this, |h, menu| {
            let f: u32 = h.read(h.read::<u32>(menu) + 4);
            h.call(f, Conv::Thiscall(menu), &[ev, frame]);
        });
        e = h.read(e + 0x10);
    }
    each_menu(h, this, |h, menu| {
        let f: u32 = h.read(h.read::<u32>(menu));
        h.call(f, Conv::Thiscall(menu), &[]);
    });
    let lock = h.call_sym(MENU_LOCK, Conv::Cdecl, &[]);
    let f = LEAVE_CS.get(h);
    h.call(f, Conv::Stdcall, &[lock])
}

/// Calls `f` for every set entry of the manager's menu array (`+0xc`,
/// count `+8` low 31 bits, re-read each step).
fn each_menu<H: Host>(h: &mut H, this: u32, mut f: impl FnMut(&mut H, u32)) {
    let mut i = 0u32;
    while i < h.read::<u32>(this + 8) & 0x7fff_ffff {
        let menus: u32 = h.read(this + 0xc);
        let menu: u32 = h.read(menus + 4 * i);
        if menu != 0 {
            f(h, menu);
        }
        i += 1;
    }
}
