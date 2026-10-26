//! Continuation round 5 of list `h4-glue-other`, batch 1: the battle and
//! menu UI's small x87 drawers (the damage digits, the battle icons, the
//! limit background and special gauge, the ratio number, the simple help
//! message, the wait window's cursor), three `graphicDrawUIElement`
//! records (a shape cell, a sequence layout image, the menu icon) and
//! the movie layer's `movie_disp2`.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`. Many of these build a
//! `TOMkpShapeXYWHUV` argument list from image constants, some through
//! `graphicUiRemapX2`/`Y2` (`ST0`). Records: [`crate::h4_e4`] (the words
//! the original leaves unset are 0 here).

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{GET_TIME, REMAP_X2, REMAP_Y2, SHAPE_UV, SHAPE_UV_C2, int_f32, pass};
use crate::h4_e4::{DRAW_UI_ELEMENT, RECORD};

/// The `f32` constant at image address `sym` through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

/// `graphicUiRemapX2(constant at sym)` stored (`fstp dword`).
fn remap_x(h: &mut impl Host, sym: u32) -> u32 {
    let v = konst(h, sym);
    store(h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v]))
}

/// `graphicUiRemapY2(constant at sym)` stored (`fstp dword`).
fn remap_y(h: &mut impl Host, sym: u32) -> u32 {
    let v = konst(h, sym);
    store(h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v]))
}

/// The UV words of a shape call: each a constant (`fld dword`), or `0`
/// for an `fldz`.
fn uv(h: &impl Host, syms: [u32; 4]) -> [u32; 4] {
    syms.map(|s| if s == 0 { 0 } else { konst(h, s) })
}

// ---- the damage digits

/// `TOMakePktBtlDamageNumRGBA(n, x, y, r, g, b, a)` (cdecl): digit cell
/// `n` (5 per row: column `n % 5`, row `n / 5`, both toward zero) of the
/// damage font as `TOMkpShapeXYWHUVC2(400, x, y, remapX2(0x00b656c4),
/// remapY2(0x00b5ee90), u0, v0, u1, v1, c, c)` with `u0 = (col * K0 +
/// K1) * K2`, `v0 = (row * 65 + K3) * K2` and `u1`/`v1` those plus
/// `0x00b66c40`/`0x00b66c48` (`f64` constants), `c` the four bytes.
/// `remapY2` runs first. Returns the shape call's `eax`.
pub fn damage_num(h: &mut impl Host, n: u32, x: u32, y: u32, rgba: [u32; 4]) -> u32 {
    let n = n.cast_signed();
    let row = n.wrapping_div(5);
    let col = n.wrapping_sub(row.wrapping_mul(5));
    let c = (((rgba[3] << 8 | rgba[2]) << 8 | rgba[1]) << 8) | rgba[0];
    let k2 = konst64(h, 0x00b5_8b50);
    let u0 = add(
        mul(load(int_f32(col)), konst64(h, 0x00b6_5ad0)),
        konst64(h, 0x00b6_6d28),
    );
    let u0 = store(mul(u0, k2));
    let v0 = store(mul(
        add(load(int_f32(row.wrapping_mul(65))), konst64(h, 0x00b6_6cf8)),
        k2,
    ));
    let v1 = store(add(konst64(h, 0x00b6_6c48), load(v0)));
    let u1 = store(add(konst64(h, 0x00b6_6c40), load(u0)));
    let hh = remap_y(h, 0x00b5_ee90);
    let w = remap_x(h, 0x00b6_56c4);
    let args = [
        400,
        int_f32(x.cast_signed()),
        int_f32(y.cast_signed()),
        w,
        hh,
        u0,
        v0,
        u1,
        v1,
        c,
        c,
    ];
    h.call_sym(SHAPE_UV_C2, Conv::Cdecl, &args)
}

// ---- the battle icons

/// `TOMakePktBtlIconA`'s cells: remapY2 (height), remapX2 (width), then
/// u, v, u1, v1 (0: `fldz`).
const BTL_ICONS: [[u32; 6]; 4] = [
    [
        0x00b5_9830,
        0x00b4_9988,
        0x00b6_67e0,
        0x00b6_6dc0,
        0x00b6_56e4,
        0x00b6_6dd8,
    ],
    [
        0x00b3_eb9c,
        0x00b6_56d8,
        0x00b6_6df4,
        0x00b6_6db8,
        0x00b6_6e1c,
        0x00b6_6dd0,
    ],
    [
        0x00b5_9830,
        0x00b6_6d90,
        0x00b6_6db4,
        0x00b6_6dbc,
        0x00b6_6dc8,
        0x00b6_67e8,
    ],
    [
        0x00b5_ee90,
        0x00b6_6d80,
        0,
        0x00b6_6db8,
        0x00b6_5868,
        0x00b5_ef64,
    ],
];

/// `TOMakePktBtlIconA(kind, x, y, alpha)` (cdecl): battle icon `kind`
/// (0..=3, from a jump table) as `TOMkpShapeXYWHUVC2(600, x, y, w, h, u,
/// v, u1, v1, c, c)` with `c = alpha << 24 | 0x808080`. Another kind draws
/// nothing and leaves `c` in `eax`; else the shape call's `eax`.
pub fn btl_icon(h: &mut impl Host, kind: u32, x: u32, y: u32, alpha: u32) -> u32 {
    let c = alpha << 24 | 0x0080_8080;
    let Some(cell) = BTL_ICONS.get(kind as usize) else {
        return c;
    };
    let hh = remap_y(h, cell[0]);
    let w = remap_x(h, cell[1]);
    let [u0, v0, u1, v1] = uv(h, [cell[2], cell[3], cell[4], cell[5]]);
    let args = [
        600,
        int_f32(x.cast_signed()),
        int_f32(y.cast_signed()),
        w,
        hh,
        u0,
        v0,
        u1,
        v1,
        c,
        c,
    ];
    h.call_sym(SHAPE_UV_C2, Conv::Cdecl, &args)
}

// ---- the limit background

/// One `TOMkpShapeXYWHUV(-1, ...)` of [`limit_bg`]: the remaps (in call
/// order: y2 for h, x2 for w, y2 for y, x2 for x), then u, v, u1, v1.
fn limit_part(h: &mut impl Host, remaps: [u32; 4], uvs: [u32; 4]) -> u32 {
    let hh = remap_y(h, remaps[0]);
    let w = remap_x(h, remaps[1]);
    let y = remap_y(h, remaps[2]);
    let x = remap_x(h, remaps[3]);
    let [u0, v0, u1, v1] = uv(h, uvs);
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[0xffff_ffff, x, y, w, hh, u0, v0, u1, v1],
    )
}

/// `TODrawLimitBG()` (cdecl): the limit menu's three background plates,
/// each `TOMkpShapeXYWHUV(-1, x, y, w, h, u, v, u1, v1)` of remapped and
/// plain image constants. Returns the last call's `eax`.
pub fn limit_bg(h: &mut impl Host) -> u32 {
    limit_part(
        h,
        [0x00b4_1d88, 0x00b5_f434, 0x00b4_3dc0, 0x00b5_ef1c],
        [0, 0x00b5_f38c, 0x00b5_f434, 0x00b5_f3a0],
    );
    limit_part(
        h,
        [0x00b5_f348, 0x00b5_f434, 0x00b5_f3c4, 0x00b5_ef1c],
        [0, 0x00b5_f3a8, 0x00b5_f434, 0x00b5_efa0],
    );
    limit_part(
        h,
        [0x00b5_f31c, 0x00b5_f42c, 0x00b5_f3d0, 0x00b5_f368],
        [0x00b9_2308, 0x00b5_4c78, 0x00b5_f430, 0x00b5_f374],
    )
}

// ---- the limit special gauge

/// `TOMkpBtlLimitSpecialGauge(x, y, w)` (cdecl): the gauge's left cap at
/// `x - remapX2(0x00b656c0)`, its middle `w` wide at `x`, its right cap
/// at `x + w`, all at `y` and `remapY2(0x00b5ee98)` high, as
/// `TOMkpShapeXYWHUV(400, ...)`. Returns the last call's `eax`.
pub fn limit_special_gauge(h: &mut impl Host, x: u32, y: u32, w: u32) -> u32 {
    let fy = int_f32(y.cast_signed());
    let fx = int_f32(x.cast_signed());
    // the left cap
    let hh = remap_y(h, 0x00b5_ee98);
    let cw = remap_x(h, 0x00b6_56c0);
    let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[konst(h, 0x00b6_56c0)]);
    let left = store(sub(load(fx), r));
    let [u0, v0, u1, v1] = uv(h, [0x00b6_6b70, 0x00b6_6b5c, 0x00b6_6b7c, 0x00b5_f0a8]);
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[400, left, fy, cw, hh, u0, v0, u1, v1],
    );
    // the middle
    let hh = remap_y(h, 0x00b5_ee98);
    let fw = int_f32(w.cast_signed());
    let [u0, v0, u1, v1] = uv(h, [0x00b6_6b7c, 0x00b6_6b5c, 0x00b6_6ba0, 0x00b5_f0a8]);
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[400, fx, fy, fw, hh, u0, v0, u1, v1],
    );
    // the right cap
    let hh = remap_y(h, 0x00b5_ee98);
    let cw = remap_x(h, 0x00b6_56c0);
    let right = int_f32(x.wrapping_add(w).cast_signed());
    let [u0, v0, u1, v1] = uv(h, [0x00b6_6ba0, 0x00b6_6b5c, 0x00b6_6ba4, 0x00b5_f0a8]);
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[400, right, fy, cw, hh, u0, v0, u1, v1],
    )
}

// ---- the ratio number

/// `TOMkpShapeXYC2(shape, x, y, c0, c1)` (cdecl).
pub const SHAPE_XY_C2: u32 = 0x0090_3a50;
/// `TOCtrlPkt()` (cdecl).
pub const CTRL_PKT: u32 = 0x008f_3a30;

/// Two decimal digits of `v` (tens, then ones), each `quotient + '0'` as
/// the original's byte; `v` keeps the remainder.
fn two_digits(v: i32) -> [u8; 2] {
    let tens = v.wrapping_div(10);
    let ones = v.wrapping_sub(tens.wrapping_mul(10));
    [
        (tens as u8).wrapping_add(b'0'),
        (ones as u8).wrapping_add(b'0'),
    ]
}

/// `TOMkpBtlRatioNumber_00901490(ratio, x, y, c)` (cdecl): `ratio` (a
/// float) as "dd.dd": the integer part `ftol(ratio)` and the fraction
/// `ftol((ratio - (float)int) * 0x00b922e0)`, two digits each (a value
/// outside 0..=99 gives other bytes, as the original's). Each digit byte
/// `'0'..='9'` draws `TOMkpShapeXYC2(byte + 0x1b0, x, y, c, c)` and moves
/// x by 16; a '.' draws shape 0x1df at `x - 5` and moves x by 5; a 0 byte
/// ends the string. Then `TOCtrlPkt()`, whose `eax` it returns.
pub fn ratio_number(h: &mut impl Host, ratio: u32, x: u32, y: u32, c: u32) -> u32 {
    let r = load(ratio);
    let whole = ftol(h, r).cast_signed();
    let frac = sub(r, load(int_f32(whole)));
    let frac = ftol(h, mul(frac, konst64(h, 0x00b9_22e0))).cast_signed();
    let [a, b] = two_digits(whole);
    let [d, e] = two_digits(frac);
    let text = [a, b, b'.', d, e];
    let mut x = x;
    for &ch in &text {
        if ch == 0 {
            break;
        }
        if ch.is_ascii_digit() {
            let shape = u32::from(ch).wrapping_add(0x1b0);
            h.call_sym(SHAPE_XY_C2, Conv::Cdecl, &[shape, x, y, c, c]);
            x = x.wrapping_add(0x10);
        }
        if ch == b'.' {
            x = x.wrapping_sub(5);
            h.call_sym(SHAPE_XY_C2, Conv::Cdecl, &[0x1df, x, y, c, c]);
            x = x.wrapping_add(0xa);
        }
    }
    h.call_sym(CTRL_PKT, Conv::Cdecl, &[])
}

// ---- the simple help message

/// `g_BtlSimpleHelpMes`: 16-byte entries (shown, then the two message
/// words).
pub const SIMPLE_HELP: u32 = 0x0133_f510;
/// `ToGetCrossExtMesFontWidth(a, b, &width, scale, one)` (cdecl).
pub const MES_FONT_WIDTH: u32 = 0x0090_5320;
/// `TOMkpCrossExtMesFontLClutTypeRGBA(a, b, x, y, 0, 1, r, g, b, a,
/// scale, one)` (cdecl).
pub const MES_FONT_DRAW: u32 = 0x0090_1700;

/// `TOBtlDrawSimpleHelpMes(i)` (cdecl; `width` the word the original
/// keeps in its argument slot, which `ToGetCrossExtMesFontWidth` fills):
/// when entry `i` of [`SIMPLE_HELP`] is shown, its plate (a grey
/// `TOMkpShapeXYWHUVC2`), its frame (shape 0x3e) and its text centred:
/// x `(K - width) * K2` (`f64` constants `0x00b41d38`, `0x00b92220`).
/// Returns the text call's `eax`, or `None` (the caller's) when hidden.
pub fn simple_help(h: &mut impl Host, width: u32, i: u32) -> Option<u32> {
    // the slot holds the argument until the width call fills it
    h.write(width, i);
    let entry = h.rebase(SIMPLE_HELP).wrapping_add(i << 4);
    if h.read::<u32>(entry) == 0 {
        return None;
    }
    // the plate
    let hh = remap_y(h, 0x00b5_eea8);
    let w = remap_x(h, 0x00b5_f030);
    let y = remap_y(h, 0x00b4_3db8);
    let x = remap_x(h, 0x00b5_eef0);
    let [u0, v0, u1, v1] = uv(h, [0, 0x00b5_efb0, 0x00b5_f030, 0x00b5_efcc]);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            0xffff_ffff,
            x,
            y,
            w,
            hh,
            u0,
            v0,
            u1,
            v1,
            0x8080_8080,
            0x0080_8080,
        ],
    );
    // the frame
    let hh = remap_y(h, 0x00b5_ee64);
    let w = remap_x(h, 0x00b5_eedc);
    let y = remap_y(h, 0x00b5_eed0);
    let x = remap_x(h, 0x00b5_eef0);
    let [u0, v0, u1, v1] = uv(h, [0x00b5_edf4, 0x00b5_ee20, 0x00b5_ee0c, 0x00b5_ee2c]);
    h.call_sym(SHAPE_UV, Conv::Cdecl, &[0x3e, x, y, w, hh, u0, v0, u1, v1]);
    // the text
    let a: u32 = h.read(entry.wrapping_add(4));
    let b: u32 = h.read(entry.wrapping_add(8));
    let scale = konst(h, 0x00b5_ee10);
    let one = 1f32.to_bits();
    h.call_sym(MES_FONT_WIDTH, Conv::Cdecl, &[a, b, width, scale, one]);
    let ty = remap_y(h, 0x00b5_eee4);
    let wv = load(h.read(width));
    let tx = store(mul(
        sub(konst64(h, 0x00b4_1d38), wv),
        konst64(h, 0x00b9_2220),
    ));
    let a: u32 = h.read(entry.wrapping_add(4));
    let b: u32 = h.read(entry.wrapping_add(8));
    let scale = konst(h, 0x00b5_ee10);
    Some(h.call_sym(
        MES_FONT_DRAW,
        Conv::Cdecl,
        &[a, b, tx, ty, 0, 1, 0x80, 0x80, 0x80, 0x80, scale, one],
    ))
}

// ---- the wait window's cursor

/// The battle wait window's skip flag (`0x0133d6e0`): 1 skips
/// `TODrawWaitBtlWinSub`.
pub const WAIT_WIN_SKIP: Global<u32> = Global::new(0x0133_d6e0);
/// `TOBtlDrawCommandWindow(w)` (cdecl).
pub const COMMAND_WINDOW: u32 = 0x0089_f370;
/// `TOBtlSetCurPos(kind, x, y)` (cdecl).
pub const SET_CUR_POS: u32 = 0x0089_99f0;

/// `TODrawWaitBtlWinSub(w)` (cdecl): unless [`WAIT_WIN_SKIP`] is 1, the
/// window's open time (`+0xe0`: `graphicGetTime() - start` while the
/// start `+0xdc` is above 0, else 0), its command window, and when the
/// cursor (i16 `+0x42`) lies in the visible rows (from i16 `+0x40`,
/// `+0x3a` rows of `+0x38` columns) the cursor at x `ftol(+0x78) +
/// column * i16 +0x62` and y `ftol(remapY2(row * 0x34 + 5) + +0x7c)`.
/// Returns 0, or `None` (the caller's `eax`) when skipped. (A zero
/// column count never reaches the division: the cursor is then past the
/// rows.)
pub fn wait_win_sub(h: &mut impl Host, w: u32) -> Option<u32> {
    if WAIT_WIN_SKIP.get(h) == 1 {
        return None;
    }
    let start: u32 = h.read(w.wrapping_add(0xdc));
    // `fldz; fcom start`: the time only when 0 < start (not unordered)
    let open = if 0.0 < load(start) {
        let t = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let start: u32 = h.read(w.wrapping_add(0xdc));
        store(sub(t, load(start)))
    } else {
        0
    };
    h.write(w.wrapping_add(0xe0), open);
    h.call_sym(COMMAND_WINDOW, Conv::Cdecl, &[w]);
    let first = i32::from(h.read::<i16>(w.wrapping_add(0x40)));
    let cursor = i32::from(h.read::<i16>(w.wrapping_add(0x42)));
    if cursor < first {
        return Some(0);
    }
    let cols = i32::from(h.read::<i16>(w.wrapping_add(0x38)));
    let rows = i32::from(h.read::<i16>(w.wrapping_add(0x3a)));
    if cursor >= rows.wrapping_mul(cols).wrapping_add(first) || cols == 0 {
        return Some(0);
    }
    let x0 = ftol(h, load(h.read(w.wrapping_add(0x78))));
    let step = i32::from(h.read::<i16>(w.wrapping_add(0x62)));
    let x = (cursor % cols)
        .wrapping_mul(step)
        .wrapping_add(x0.cast_signed());
    let row = (cursor - first) / cols;
    let ry = pass(int_f32(row.wrapping_mul(0x34).wrapping_add(5)));
    let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[ry]);
    let y = ftol(h, add(r, load(h.read(w.wrapping_add(0x7c)))));
    h.call_sym(SET_CUR_POS, Conv::Cdecl, &[0, x.cast_unsigned(), y]);
    Some(0)
}

// ---- UI records

/// `TOGetShapTextureName(id)` (cdecl).
pub const SHAPE_TEXTURE: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl).
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `BeginScissor_XYUV(&x0, &y0, &x1, &y1, &u0, &v0, &u1, &v1)` (cdecl).
pub const BEGIN_SCISSOR_UV: u32 = 0x008e_5a20;

/// Bytes of the frames of [`shape_cell`] and [`seq_layout_image`]: the
/// image's width and height, a temporary, then the record.
pub const CELL_FRAME: u32 = 0xc + RECORD;
/// Bytes of [`menu_icon`]'s frame: height, width, two temporaries, the
/// record.
pub const ICON_FRAME: u32 = 0x10 + RECORD;

/// Zeroes the record at `rec`.
fn clear(h: &mut impl Host, rec: u32) {
    for k in 0..RECORD / 4 {
        h.write(rec.wrapping_add(4 * k), 0u32);
    }
}

/// The record's corners `(x0, y0, u0, v0)` at `+0` and `(x1, y1, u1, v1)`
/// at `+0x20`, colour words `c0` at `+0x10` and `c1` at `+0x30`.
fn corners(h: &mut impl Host, rec: u32, a: [u32; 4], b: [u32; 4], c0: [u32; 4], c1: [u32; 4]) {
    for k in 0..4u32 {
        let i = k as usize;
        h.write(rec.wrapping_add(4 * k), a[i]);
        h.write(rec.wrapping_add(0x20 + 4 * k), b[i]);
        h.write(rec.wrapping_add(0x10 + 4 * k), c0[i]);
        h.write(rec.wrapping_add(0x30 + 4 * k), c1[i]);
    }
}

/// `int / size` as `fild`, `fstp dword`, `fld`, `fdiv dword`, stored.
fn ratio(n: i32, size: u32) -> u32 {
    store(div(load(int_f32(n)), load(size)))
}

/// `TOMkpShape2ECCCell_008f8d50(n, x, y, w, h, alpha)` (cdecl; `frame`
/// from `ebp - 0xa8`, [`CELL_FRAME`]): cell `n` of shape 0x2ecc (9 per
/// column of 170 x 14 texels: `u` from `n / 9`, `v` from `n % 9`, over
/// the image's size) at the float rectangle `x, y, w, h`, grey with alpha
/// word `alpha`; `graphicDrawUIElement(rec, texture, 1, 0, 0)`, whose
/// `eax` it returns.
pub fn shape_cell(h: &mut impl Host, frame: u32, [n, x, y, w, hh, alpha]: [u32; 6]) -> u32 {
    let (iw, ih, rec) = (frame, frame.wrapping_add(4), frame.wrapping_add(0xc));
    clear(h, rec);
    let tex = h.call_sym(SHAPE_TEXTURE, Conv::Cdecl, &[0x2ecc]);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[0x2ecc, iw, ih]);
    let n = n.cast_signed();
    let q = n.wrapping_div(9);
    let r = n.wrapping_sub(q.wrapping_mul(9));
    let (u, v) = (q.wrapping_mul(0xaa), r.wrapping_mul(0xe));
    let (wv, hv): (u32, u32) = (h.read(iw), h.read(ih));
    let a = [pass(x), pass(y), ratio(u, wv), ratio(v, hv)];
    let b = [
        store(add(load(x), load(w))),
        store(add(load(y), load(hh))),
        ratio(u.wrapping_add(0xaa), wv),
        ratio(v.wrapping_add(0xe), hv),
    ];
    let c = [0x80, 0x80, 0x80, alpha];
    corners(h, rec, a, b, c, c);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, tex, 1, 0, 0])
}

/// `TOSeqPrimLayoutImage_008f59c0(x, y, w, h, u, v, c0, c1)` (cdecl;
/// `frame` from `ebp - 0xa8`, [`CELL_FRAME`]): image 16000's texels `u,
/// v` to `u + w, v + h` drawn at the 1920 x 1080 rectangle `x, y, w, h`
/// scaled to 512 x 416 (`x * K0 / K1`, `y * K2 / K3`: `f64` constants),
/// white with alpha words `c0`, `c1`; `graphicDrawUIElement(rec,
/// texture, 1, 0, 0)`, whose `eax` it returns.
pub fn seq_layout_image(h: &mut impl Host, frame: u32, a: [u32; 8]) -> u32 {
    let [x, y, w, hh, u, v, c0, c1] = a.map(u32::cast_signed);
    let (iw, ih, rec) = (frame, frame.wrapping_add(4), frame.wrapping_add(0xc));
    clear(h, rec);
    let tex = h.call_sym(SHAPE_TEXTURE, Conv::Cdecl, &[16000]);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[16000, iw, ih]);
    let (k0, k1) = (konst64(h, 0x00b4_1d38), konst64(h, 0x00b4_1d68));
    let (k2, k3) = (konst64(h, 0x00b4_1d30), konst64(h, 0x00b4_1d58));
    let (wv, hv): (u32, u32) = (h.read(iw), h.read(ih));
    let sx = |n: i32| store(div(mul(load(int_f32(n)), k0), k1));
    let sy = |n: i32| store(div(mul(load(int_f32(n)), k2), k3));
    let p = [sx(x), sy(y), ratio(u, wv), ratio(v, hv)];
    let q = [
        sx(x.wrapping_add(w)),
        sy(y.wrapping_add(hh)),
        ratio(u.wrapping_add(w), wv),
        ratio(v.wrapping_add(hh), hv),
    ];
    let (c0, c1) = (c0.cast_unsigned(), c1.cast_unsigned());
    corners(h, rec, p, q, [0xff, 0xff, 0xff, c0], [0xff, 0xff, 0xff, c1]);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, tex, 1, 0, 0])
}

/// `TOAddPktMenuIconRGBA(ret, x, y, icon, r, g, b, a)` (cdecl; `frame`
/// from `ebp - 0xac`, [`ICON_FRAME`]; the colour and icon arguments are
/// bytes): menu icon `icon` (17 per row of 15 x 18 texels) of image
/// 0x3dc0 at `x, y`, 10 x 16 large (`f64` constants), in the colour;
/// `+0x80` 500. Drawn (`graphicDrawUIElement(rec, texture, 1, 0, 0)`)
/// unless `BeginScissor_XYUV` clips it away. Returns `ret`.
pub fn menu_icon(h: &mut impl Host, frame: u32, a: [u32; 8]) -> u32 {
    let [ret, x, y, icon, r, g, b, al] = a;
    let (ih, iw, rec) = (frame, frame.wrapping_add(4), frame.wrapping_add(0x10));
    clear(h, rec);
    let tex = h.call_sym(SHAPE_TEXTURE, Conv::Cdecl, &[0x3dc0]);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[0x3dc0, iw, ih]);
    let rgba = [r & 0xff, g & 0xff, b & 0xff, al & 0xff];
    let icon = (icon & 0xff).cast_signed();
    let (u, v) = ((icon % 17) * 15, (icon / 17) * 18);
    let (wv, hv): (u32, u32) = (h.read(iw), h.read(ih));
    let fx = int_f32(x.cast_signed());
    let fy = int_f32(y.cast_signed());
    let p = [fx, fy, ratio(u, wv), ratio(v, hv)];
    let q = [
        store(add(load(fx), konst64(h, 0x00b9_22b0))),
        store(add(load(fy), konst64(h, 0x00b3_13a0))),
        ratio(u + 15, wv),
        ratio(v + 18, hv),
    ];
    corners(h, rec, p, q, rgba, rgba);
    h.write(rec.wrapping_add(0x80), 500u32);
    let at = |o: u32| rec.wrapping_add(o);
    let clip = [
        at(0),
        at(4),
        at(0x20),
        at(0x24),
        at(8),
        at(0xc),
        at(0x28),
        at(0x2c),
    ];
    if h.call_sym(BEGIN_SCISSOR_UV, Conv::Cdecl, &clip) != 0 {
        h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, tex, 1, 0, 0]);
    }
    ret
}

// ---- the movie layer

/// `PMatrix4_Copy_00453340(out; m)` (thiscall).
pub const MATRIX_COPY: u32 = 0x0045_3340;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `PhyreScene_setEnableRenderWorldToTexture(scene; on, &m, v0[4], v1[4],
/// e)` (thiscall).
pub const RENDER_TO_TEXTURE: u32 = 0x0066_d000;
/// Bytes of [`movie_disp2`]'s frame: the matrix.
pub const DISP2_FRAME: u32 = 0x40;

/// `movie_disp2(on, m, v0, v1, e)` (cdecl; `frame` the matrix at `ebp -
/// 0x60`): the identity, or a copy of `m` (`PMatrix4_Copy`) when `m` is
/// not null, and the four floats at `v0` and at `v1` (0 when null;
/// through `fld`/`fstp`), handed with `on != 0` and `e` to
/// `PhyreScene_setEnableRenderWorldToTexture`, whose `eax` it returns.
pub fn movie_disp2(h: &mut impl Host, frame: u32, a: [u32; 5]) -> u32 {
    let [on, m, v0, v1, e] = a;
    for k in 0..16u32 {
        let v = if k % 5 == 0 { 1f32.to_bits() } else { 0 };
        h.write(frame.wrapping_add(4 * k), v);
    }
    if m != 0 {
        h.call_sym(MATRIX_COPY, Conv::Thiscall(frame), &[m]);
    }
    let mut args = [0u32; 11];
    args[0] = u32::from(on != 0);
    args[1] = frame;
    for (k, p) in [v0, v1].into_iter().enumerate() {
        if p != 0 {
            for i in 0..4u32 {
                args[2 + 4 * k + i as usize] = pass(h.read(p.wrapping_add(4 * i)));
            }
        }
    }
    args[10] = e;
    let scene = PHYRE_SCENE.get(h);
    h.call_sym(RENDER_TO_TEXTURE, Conv::Thiscall(scene), &args)
}
