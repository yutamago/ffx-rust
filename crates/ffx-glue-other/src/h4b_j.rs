//! List `h4-glue-other`, second agent (h4b), round 3 batch 1: the battle
//! wait windows of the summon list and the right-hand command list
//! (`TODrawWaitBtlWinSummon`, `TODrawWaitBtlWinRight`), and the panel
//! they share with `TOBtlDrawCommandWindow`'s: the pulsing cursor plate,
//! three plate rows sliding in 0.1 s apart with their gloss strips, the
//! water ripples over the rows, and the command cursor's position.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`. The remaps
//! (`graphicUiRemapX2`/`Y2`) are called in the original's order, each
//! result stored as the `f32` argument word it becomes.

use ffx_bridge::x87::{add, div, ftol, ftol2, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{CI_COS, GET_TIME, REMAP_X2, REMAP_Y2, SHAPE_UV, SHAPE_UV_C2, int_f32, pass};
use crate::h4_g1::{COMMAND_WINDOW, SET_CUR_POS};

/// `DrawWaterWaveShapeC2(x, y, w, h, u, v, uw, vh, su, sv, c0, c1)`
/// (cdecl).
pub const WATER: u32 = 0x008e_7d30;
/// `TODrawScissorXYWH(x, y, w, h)` (cdecl).
pub const SCISSOR_XYWH: u32 = 0x008f_9230;
/// `TOBtlDrawSummonPrim(chr, x, y)` (cdecl).
pub const SUMMON_PRIM: u32 = 0x008a_0ba0;
/// `TOMkpRotateShapeXYWHUVC2(shape, x, y, w, h, u0, v0, u1, v1, c0, c1,
/// angle)` (cdecl).
pub const ROTATE_SHAPE: u32 = 0x0090_2ef0;

/// The sheet image, shape -1 of `TOMkpShapeXYWHUVC2`.
pub(crate) const SHEET: u32 = u32::MAX;
/// The plate rows' grey (alpha 0x60), the gloss's (0x40).
pub(crate) const PLATE_GREY: u32 = 0x6080_8080;
pub(crate) const GLOSS_GREY: u32 = 0x4080_8080;
/// The ripples' fade ends: transparent grey, half grey.
pub(crate) const CLEAR: u32 = 0x0080_8080;
/// A row of the panel, 0x34 PS2 units.
pub(crate) const ROW_PITCH: i32 = 0x34;
/// The summon list's entry for "every summon" (`TOBtlDrawSummonPrim`
/// 0xf, 0x10 and 0x11).
const ALL_SUMMONS: u16 = 0x30ff;

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
pub(crate) fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
pub(crate) fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

/// `graphicUiRemapX2(v)` in `ST0`.
pub(crate) fn remap_x(h: &mut impl Host, v: u32) -> f64 {
    h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v])
}

/// `graphicUiRemapY2(v)` in `ST0`.
pub(crate) fn remap_y(h: &mut impl Host, v: u32) -> f64 {
    h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v])
}

/// `graphicUiRemapX2(v)` stored as an argument word.
pub(crate) fn rx(h: &mut impl Host, v: u32) -> u32 {
    store(remap_x(h, v))
}

/// `graphicUiRemapY2(v)` stored as an argument word.
pub(crate) fn ry(h: &mut impl Host, v: u32) -> u32 {
    store(remap_y(h, v))
}

/// [`rx`] of the image constant at `sym`.
pub(crate) fn rxk(h: &mut impl Host, sym: u32) -> u32 {
    let v = konst(h, sym);
    rx(h, v)
}

/// [`ry`] of the image constant at `sym`.
pub(crate) fn ryk(h: &mut impl Host, sym: u32) -> u32 {
    let v = konst(h, sym);
    ry(h, v)
}

/// The window's `i16` at `+off`, sign-extended.
pub(crate) fn half(h: &impl Host, w: u32, off: u32) -> i32 {
    i32::from(h.read::<i16>(w.wrapping_add(off)))
}

/// `idiv`: quotient and remainder (0, 0 for a zero divisor, where the
/// original faults).
pub(crate) fn idiv(a: i32, b: i32) -> (i32, i32) {
    if b == 0 {
        (0, 0)
    } else {
        (a.wrapping_div(b), a.wrapping_rem(b))
    }
}

/// `int` through `fild`, `fstp dword`, `fld dword`.
pub(crate) fn int(n: i32) -> f64 {
    load(int_f32(n))
}

/// `0 < x < 1` held: below 0 gives 0, above 1 gives 1, NaN passes (the
/// two `fcom`s of the fades).
pub(crate) fn unit(x: f64) -> f64 {
    // `clamp` keeps a NaN and -0, as the two compares do
    x.clamp(0.0, 1.0)
}

/// The y constants (image `f64`s) and the ripples' start time (`f32`) of a
/// wait window's panel; the rest of the panel's constants are shared.
#[derive(Clone, Copy)]
pub struct Panel {
    /// The plate rows' and the upper ripples' top.
    plate_top: u32,
    /// The gloss strips' top.
    gloss_top: u32,
    /// The lower ripples' top.
    ripple_low: u32,
    /// When the ripples start (seconds since the window opened).
    ripples_from: u32,
}

/// The summon window's panel: rows from 835 (`0x00b5f2b8`).
pub const SUMMON_PANEL: Panel = Panel {
    plate_top: 0x00b5_f2b8,
    gloss_top: 0x00b5_f2c0,
    ripple_low: 0x00b5_f2d0,
    ripples_from: 0x00b3_eb44,
};

/// The right window's panel (`0x00b5f2a8`, ...).
pub const RIGHT_PANEL: Panel = Panel {
    plate_top: 0x00b5_f2a8,
    gloss_top: 0x00b5_f2b0,
    ripple_low: 0x00b5_f2c8,
    ripples_from: 0x00b9_2194,
};

/// The window's open time: `+0xe0` becomes `graphicGetTime() - +0xdc`
/// while `+0xdc` is above 0 (`fcom`; NaN gives 0), else 0. Returns it.
pub fn open_time(h: &mut impl Host, w: u32) -> u32 {
    let start: u32 = h.read(w.wrapping_add(0xdc));
    let t = if 0.0 < load(start) {
        let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let start: u32 = h.read(w.wrapping_add(0xdc));
        store(sub(now, load(start)))
    } else {
        0
    };
    h.write(w.wrapping_add(0xe0), t);
    t
}

/// The pulsing grey: alpha `trunc(cos(5 t) × 32 + 96)` (`_CIcos` of
/// `graphicGetTime() × 0x00b41d20`, `fistp qword` chopped).
pub fn pulse(h: &mut impl Host) -> u32 {
    let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
    let angle = mul(now, konst64(h, 0x00b4_1d20));
    let cos = h.rebase(CI_COS);
    let c = h.call_intrinsic(cos, angle).to_bits();
    let alpha = ftol2(add(
        mul(load(c), konst64(h, 0x00b4_12f8)),
        konst64(h, 0x00b5_ee40),
    ));
    (alpha << 24).wrapping_add(CLEAR)
}

/// The cursor plate: on the cursor's row of the panel (cursor `+0x42` less
/// the first visible entry `+0x40`, over the columns `+0x38`), `+0x62`
/// wide at the panel's left (remapped 154, `0x00b5f344`), grey with alpha
/// `trunc(cos(5 t) × 32 + 96)` (`_CIcos`, `fistp qword` chopped).
pub fn cursor_plate(h: &mut impl Host, w: u32, p: Panel) {
    let colour = pulse(h);
    let uv = [0x00b5_f43c, 0x00b5_f324, 0x00b4_a1ac, 0x00b5_f34c].map(|c| konst(h, c));
    let hh = ryk(h, 0x00b1_f2c8);
    let pitch = int_f32(half(h, w, 0x62));
    let at = half(h, w, 0x42).wrapping_sub(half(h, w, 0x40));
    let row = idiv(at, half(h, w, 0x38)).0.wrapping_mul(ROW_PITCH);
    let y = store(add(int(row), konst64(h, p.plate_top)));
    let y = ry(h, y);
    let x = rxk(h, 0x00b5_f344);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            SHEET, x, y, pitch, hh, uv[0], uv[1], uv[2], uv[3], colour, colour,
        ],
    );
}

/// The three plate rows, `t` seconds (`f32` bits) into the first one's
/// slide, each next one 0.1 s (`0x00b921f0`) behind: a plate at `x =
/// a t / 0.1 - b` (`0x00b5f290`, `0x00b5f280`), held to `0x00b5f468`
/// below `0x00b5f460` and to 154 above it, less `0x00b922c8`, and its
/// gloss strip at `x`. The plate's `u` runs from 0 to `0x00b5f3c0`, or
/// with `flipped` from there to `0x00b5f408`.
pub fn slide_rows(h: &mut impl Host, t: u32, p: Panel, flipped: bool) {
    let mut t = t;
    for k in 0..3 {
        let tf = load(t);
        let raw = store(sub(
            div(mul(konst64(h, 0x00b5_f290), tf), konst64(h, 0x00b9_21f0)),
            konst64(h, 0x00b5_f280),
        ));
        let xf = load(raw);
        let right = konst(h, 0x00b5_f344);
        let x = if xf < konst64(h, 0x00b5_f460) {
            konst(h, 0x00b5_f468)
        } else if load(right) < xf {
            right
        } else {
            raw
        };
        t = store(sub(tf, konst64(h, 0x00b9_21f0)));
        let row = int(k * ROW_PITCH);
        // the plate
        let (u0, u1) = if flipped {
            (konst(h, 0x00b5_f3c0), konst(h, 0x00b5_f408))
        } else {
            (0, konst(h, 0x00b5_f3c0))
        };
        let v1 = konst(h, 0x00b1_f2c8);
        let hh = ry(h, v1);
        let ww = rxk(h, 0x00b5_f3c8);
        let y = store(add(row, konst64(h, p.plate_top)));
        let y = ry(h, y);
        let xx = store(sub(load(x), konst64(h, 0x00b9_22c8)));
        let xx = rx(h, xx);
        h.call_sym(
            SHAPE_UV_C2,
            Conv::Cdecl,
            &[SHEET, xx, y, ww, hh, u0, 0, u1, v1, PLATE_GREY, PLATE_GREY],
        );
        // the gloss strip
        let band = mul(int(k), konst64(h, 0x00b5_f1f0));
        let v1 = store(add(band, konst64(h, 0x00b5_f278)));
        let u1 = konst(h, 0x00b5_f44c);
        let v0 = store(add(band, konst64(h, 0x00b5_f260)));
        let u0 = konst(h, 0x00b5_f438);
        let hh = ryk(h, 0x00b5_f300);
        let ww = rxk(h, 0x00b5_f370);
        let y = store(add(row, konst64(h, p.gloss_top)));
        let y = ry(h, y);
        let xx = rx(h, pass(x));
        h.call_sym(
            SHAPE_UV_C2,
            Conv::Cdecl,
            &[SHEET, xx, y, ww, hh, u0, v0, u1, v1, GLOSS_GREY, GLOSS_GREY],
        );
    }
}

/// One `DrawWaterWaveShapeC2` of the ripples: tile `x` (the panel's left
/// 154 or `0x00b5f3ac`), strip top `y`, `u`, `v`, the fade's colours.
fn ripple(h: &mut impl Host, x: u32, y: u32, u: u32, v: u32, c: [u32; 2]) {
    let speed = konst(h, 0x00b4_3db4);
    let vh = konst(h, 0x00b3_eb94);
    let uw = konst(h, 0x00b5_ef1c);
    let hh = ry(h, vh);
    let ww = rx(h, uw);
    let yy = ry(h, y);
    let xx = rxk(h, x);
    h.call_sym(
        WATER,
        Conv::Cdecl,
        &[xx, yy, ww, hh, u, v, uw, vh, speed, speed, c[0], c[1]],
    );
}

/// The ripples, once the window has been open `ripples_from` seconds
/// (`+0xe0`; NaN: none): over each of the three rows an upper and a lower
/// strip of two tiles (`DrawWaterWaveShapeC2`), `v` 20 and 50 per row
/// (`0x00b49968`, `0x00b1f2c0`), the right tile's `u` + 210
/// (`0x00b49970`).
pub fn ripples(h: &mut impl Host, w: u32, p: Panel) {
    let from = konst(h, p.ripples_from);
    let open: u32 = h.read(w.wrapping_add(0xe0));
    // `fcomp`; `test ah, 0x41; jp`: on when from <= open
    let on = load(from) <= load(open);
    if !on {
        return;
    }
    for k in 0..3 {
        let next = int(k + 1);
        let v_up = store(mul(konst64(h, 0x00b4_9968), next));
        let u_left = mul(0.0, int(k));
        let u_left32 = store(u_left);
        let row = int(k * ROW_PITCH);
        let y_up = store(add(row, konst64(h, p.plate_top)));
        ripple(h, 0x00b5_f344, y_up, u_left32, v_up, [CLEAR, GLOSS_GREY]);
        let u_right = store(add(u_left, konst64(h, 0x00b4_9970)));
        ripple(h, 0x00b5_f3ac, y_up, u_right, v_up, [GLOSS_GREY, CLEAR]);
        let v_low = store(mul(next, konst64(h, 0x00b1_f2c0)));
        let y_low = store(add(row, konst64(h, p.ripple_low)));
        ripple(h, 0x00b5_f344, y_low, u_left32, v_low, [CLEAR, GLOSS_GREY]);
        ripple(h, 0x00b5_f3ac, y_low, u_right, v_low, [GLOSS_GREY, CLEAR]);
    }
}

/// The command cursor (`TOBtlSetCurPos(0, x, y)`): `x = ftol(ftol(+0x78) +
/// remapX2(0x00b5ef64) × column)` (the remap unrounded), `y =
/// ftol(remapY2(row × 0x34 + 5) + +0x7c)`, the cursor `+0x42` in rows
/// from the first visible entry `+0x40` of `+0x38` columns.
pub fn set_cursor(h: &mut impl Host, w: u32) {
    let left = ftol(h, load(h.read(w.wrapping_add(0x78))));
    let cursor = half(h, w, 0x42);
    let cols = half(h, w, 0x38);
    let k = konst(h, 0x00b5_ef64);
    let cell = remap_x(h, k);
    let (_, col) = idiv(cursor, cols);
    let x = ftol(h, add(mul(cell, int(col)), f64::from(left.cast_signed())));
    let first = half(h, w, 0x40);
    let row = idiv(cursor.wrapping_sub(first), cols).0;
    let y = int_f32(row.wrapping_mul(ROW_PITCH).wrapping_add(5));
    let y = remap_y(h, y);
    let top: u32 = h.read(w.wrapping_add(0x7c));
    let y = ftol(h, add(y, load(top)));
    h.call_sym(SET_CUR_POS, Conv::Cdecl, &[0, x, y]);
}

/// The summon names' x: `ftol(remapX2(0x00b5f35c))`.
fn name_x(h: &mut impl Host) -> u32 {
    let k = konst(h, 0x00b5_f35c);
    let v = remap_x(h, k);
    ftol(h, v)
}

/// A summon name's y: `ftol(remapY2(0x00b5f32c) + 0x00b5f240)`, the remap
/// doubled first for the third row.
fn name_y(h: &mut impl Host, twice: bool) -> u32 {
    let k = konst(h, 0x00b5_f32c);
    let r = remap_y(h, k);
    let r = if twice { add(r, r) } else { r };
    ftol(h, add(r, konst64(h, 0x00b5_f240)))
}

/// The summon names (`TOBtlDrawSummonPrim`) at remapped x `0x00b5f35c`:
/// for the "all summons" entry (`0x30ff` under the cursor) the three
/// plates 0xf, 0x10 and 0x11 at y 0xb7, `remapY2(0x00b5f32c) + 0x00b5f240`
/// and twice the remap plus it; else the cursor's summon at the second
/// of those rows.
fn summon_names(h: &mut impl Host, w: u32) {
    let list: u32 = h.read(w.wrapping_add(0x20));
    let cursor = half(h, w, 0x42);
    let entry = list.wrapping_add(cursor.wrapping_mul(2).cast_unsigned());
    let id: u16 = h.read(entry);
    if id == ALL_SUMMONS {
        let x = name_x(h);
        h.call_sym(SUMMON_PRIM, Conv::Cdecl, &[0xf, x, 0xb7]);
        let y = name_y(h, false);
        let x = name_x(h);
        h.call_sym(SUMMON_PRIM, Conv::Cdecl, &[0x10, x, y]);
        let y = name_y(h, true);
        let x = name_x(h);
        h.call_sym(SUMMON_PRIM, Conv::Cdecl, &[0x11, x, y]);
    } else {
        let y = name_y(h, true);
        let x = name_x(h);
        let list: u32 = h.read(w.wrapping_add(0x20));
        let cursor = half(h, w, 0x42);
        let entry = list.wrapping_add(cursor.wrapping_mul(2).cast_unsigned());
        let chr = i32::from(h.read::<i16>(entry)).cast_unsigned();
        h.call_sym(SUMMON_PRIM, Conv::Cdecl, &[chr, x, y]);
    }
}

/// `TODrawWaitBtlWinSummon(w)` (`0x008a75f0`, cdecl): the summon list's
/// wait window. The open time ([`open_time`]), the panel
/// ([`SUMMON_PANEL`]: [`cursor_plate`], [`slide_rows`] from the open
/// time, [`ripples`] from 0.3 s), the command window
/// (`TOBtlDrawCommandWindow`), the full-screen scissor (0, 0, 512, 416),
/// the summon names and the cursor ([`set_cursor`]). Returns 0.
///
/// A zero column count faults in the original (`idiv`); the port takes
/// the quotient and remainder as 0 there.
pub fn wait_win_summon(h: &mut impl Host, w: u32) -> u32 {
    let open = open_time(h, w);
    cursor_plate(h, w, SUMMON_PANEL);
    slide_rows(h, pass(open), SUMMON_PANEL, false);
    ripples(h, w, SUMMON_PANEL);
    h.call_sym(COMMAND_WINDOW, Conv::Cdecl, &[w]);
    h.call_sym(SCISSOR_XYWH, Conv::Cdecl, &[0, 0, 0x200, 0x1a0]);
    summon_names(h, w);
    set_cursor(h, w);
    0
}

/// Stores `v` into the argument slot, as the original does.
fn to_slot(h: &mut impl Host, slot: Option<u32>, v: u32) {
    if let Some(s) = slot {
        h.write(s, v);
    }
}

/// The right window's opening: the plate `p` (0..1 of the opening time)
/// grows from the corner: `a = 0x00b5f1f8 × p` wide and high, at `(1 - p)
/// × 0x00b5f1d0` plus `0x00b922e8`, `0x00b5f2d8`. The window slot holds
/// `a`, the y, then the x.
fn opening_plate(h: &mut impl Host, p: f64, slot: Option<u32>, uv: OpeningUv) {
    let a = store(mul(konst64(h, 0x00b5_f1f8), p));
    to_slot(h, slot, a);
    let q = mul(sub(1.0, p), konst64(h, 0x00b5_f1d0));
    let v1 = konst(h, uv.v1);
    let u1 = konst(h, uv.u1);
    let u0 = konst(h, uv.u0);
    let hh = ry(h, a);
    let ww = rx(h, a);
    let y = store(add(q, konst64(h, 0x00b5_f2d8)));
    to_slot(h, slot, y);
    let y = ry(h, y);
    let x = store(add(q, konst64(h, 0x00b9_22e8)));
    to_slot(h, slot, x);
    let x = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            SHEET,
            x,
            y,
            ww,
            hh,
            u0,
            uv.v0,
            u1,
            v1,
            0x8080_8080,
            0x8080_8080,
        ],
    );
}

/// Two rings turning with the time (`TOMkpRotateShapeXYWHUVC2`, the
/// second backwards).
pub fn rings(h: &mut impl Host) {
    let grey = 0x8080_8080;
    let one = 1f32.to_bits();
    let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
    let angle = store(now);
    let a = [
        0x00b5_8cd8,
        0x00b5_f410,
        0x00b5_eec0,
        0x00b5_eec0,
        0,
        0x00b5_eea0,
        0x00b5_eee4,
        0x00b5_ef00,
    ]
    .map(|c| if c == 0 { one } else { konst(h, c) });
    h.call_sym(
        ROTATE_SHAPE,
        Conv::Cdecl,
        &[
            SHEET, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], grey, grey, angle,
        ],
    );
    let now = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
    let angle = store(-now);
    let a = [
        0x00b4_f5e4,
        0x00b5_f418,
        0x00b5_eea4,
        0x00b5_eea4,
        0x00b5_f038,
        0,
        0x00b5_f03c,
        0x00b5_eecc,
    ]
    .map(|c| if c == 0 { one } else { konst(h, c) });
    h.call_sym(
        ROTATE_SHAPE,
        Conv::Cdecl,
        &[
            SHEET, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], grey, grey, angle,
        ],
    );
}

/// The fading shapes' third piece: the right window's
/// (`TOMkpShapeXYWHUV`, at `0x00b5f288 - p × 0x00b5f270`) or the left
/// window's (`TOMkpShapeXYWHUVC2`, at the slide plus `0x00b5f250`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Third {
    Right,
    Left,
    /// The main window's: the left one's with `u` from 0 to `0x00b5f0b0`.
    Main,
}

/// Fading in over `0x00b48308` seconds after the opening (`p`, held to
/// 0..1; alpha `p × 0x00b4c170`): two shapes 0x190 sliding with `p ×
/// 0x00b5f218`, then the [`Third`]. Returns the slide (`f64`, which the
/// original keeps in its frame).
pub fn fade_shapes(h: &mut impl Host, w: u32, third: Third) -> f64 {
    let open: u32 = h.read(w.wrapping_add(0xe0));
    let p = store(div(
        sub(load(open), konst64(h, 0x00b3_eb60)),
        konst64(h, 0x00b4_8308),
    ));
    let p = unit(load(p));
    let alpha = ftol2(mul(konst64(h, 0x00b4_c170), p));
    let colour = (alpha << 24).wrapping_add(CLEAR);
    let slide = mul(p, konst64(h, 0x00b5_f218));
    let v1 = konst(h, 0x00b5_ee14);
    let u1 = konst(h, 0x00b5_edb8);
    let v0 = konst(h, 0x00b5_ee04);
    let hh = ryk(h, 0x00b5_eefc);
    let ww = rxk(h, 0x00b5_ef80);
    let y = ryk(h, 0x00b5_f40c);
    let x = store(sub(slide, konst64(h, 0x00b5_f1e8)));
    let x = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[0x190, x, y, ww, hh, 0, v0, u1, v1, colour, colour],
    );
    let v1 = konst(h, 0x00b5_f108);
    let u1 = konst(h, 0x00b5_ede0);
    let v0 = konst(h, 0x00b4_1cac);
    let u0 = konst(h, 0x00b5_f0c0);
    let hh = ryk(h, 0x00b5_4f90);
    let ww = rxk(h, 0x00b5_f358);
    let y = ryk(h, 0x00b5_f420);
    let x = store(add(slide, konst64(h, 0x00b5_f1b0)));
    let x = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[0x190, x, y, ww, hh, u0, v0, u1, v1, colour, colour],
    );
    if third != Third::Right {
        let v1 = konst(h, 0x00b5_f0c8);
        let (u0, u1) = if third == Third::Left {
            (konst(h, 0x00b5_f0dc), konst(h, 0x00b5_f11c))
        } else {
            (0, konst(h, 0x00b5_f0b0))
        };
        let v0 = konst(h, 0x00b5_f0c4);
        let hh = ryk(h, 0x00b5_ee74);
        let ww = rxk(h, 0x00b5_ef24);
        let y = ryk(h, 0x00b5_f420);
        let x = store(add(slide, konst64(h, 0x00b5_f250)));
        let x = rx(h, x);
        h.call_sym(
            SHAPE_UV_C2,
            Conv::Cdecl,
            &[0x190, x, y, ww, hh, u0, v0, u1, v1, colour, colour],
        );
        return slide;
    }
    let v1 = konst(h, 0x00b5_f0c8);
    let u1 = konst(h, 0x00b5_f0d4);
    let v0 = konst(h, 0x00b5_f0c4);
    let u0 = konst(h, 0x00b5_f0b4);
    let hh = ryk(h, 0x00b5_ee74);
    let ww = rxk(h, 0x00b5_ef24);
    let y = ryk(h, 0x00b5_f3f8);
    let x = store(sub(
        konst64(h, 0x00b5_f288),
        mul(p, konst64(h, 0x00b5_f270)),
    ));
    let x = rx(h, x);
    h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[0x190, x, y, ww, hh, u0, v0, u1, v1],
    );
    slide
}

/// The opening plate's UVs: `u0`, `u1`, `v1` image constants, `v0` the
/// word itself.
#[derive(Clone, Copy)]
pub struct OpeningUv {
    u0: u32,
    v0: u32,
    u1: u32,
    v1: u32,
}

/// The side windows' opening plate.
pub const SIDE_OPENING: OpeningUv = OpeningUv {
    u0: 0x00b5_f444,
    v0: 0,
    u1: 0x00b5_f450,
    v1: 0x00b5_f31c,
};

/// The main window's opening plate (`v0` 1.0).
pub const MAIN_OPENING: OpeningUv = OpeningUv {
    u0: 0x00b5_f038,
    v0: 0x3f80_0000,
    u1: 0x00b5_f03c,
    v1: 0x00b5_eecc,
};

/// The side windows' start: the open time ([`open_time`]); while it is
/// below `0x00b3eb60`, only the opening plate ([`opening_plate`], the
/// fraction held to 0..1) and `None`; else the time since then (`f32`
/// bits).
pub fn opening(h: &mut impl Host, w: u32, slot: Option<u32>, uv: OpeningUv) -> Option<u32> {
    let open = open_time(h, w);
    let t = load(pass(open));
    let end = konst64(h, 0x00b3_eb60);
    // `fcom`; `test ah, 0x41; jne`: the opening while it is above t
    if end > t {
        let p = unit(load(store(div(t, end))));
        opening_plate(h, p, slot, uv);
        return None;
    }
    Some(store(sub(t, end)))
}

/// The cursor ([`set_cursor`]) when it (`+0x42`) is in the visible rows
/// (from `+0x40`, `+0x3a` rows of `+0x38` columns).
pub fn cursor_if_shown(h: &mut impl Host, w: u32) {
    let cursor = half(h, w, 0x42);
    let first = half(h, w, 0x40);
    if cursor < first {
        return;
    }
    let shown = half(h, w, 0x38)
        .wrapping_mul(half(h, w, 0x3a))
        .wrapping_add(first);
    if cursor >= shown {
        return;
    }
    set_cursor(h, w);
}

/// `TODrawWaitBtlWinRight(w)` (`0x008a6a60`, cdecl): the right-hand
/// command list's wait window. The open time ([`open_time`]); while it is
/// below `0x00b3eb60` only the opening plate ([`right_opening`], the
/// fraction held to 0..1). Then, `0x00b3eb60` later: the panel
/// ([`RIGHT_PANEL`]), the rings and the fading shapes, the command window
/// and, with the cursor (`+0x42`) in the visible rows (from `+0x40`,
/// `+0x3a` rows of `+0x38` columns), the cursor ([`set_cursor`]).
/// Returns 0.
///
/// `slot` is the window argument's word: the opening leaves its plate's
/// x there (after its size and y).
pub fn wait_win_right(h: &mut impl Host, w: u32, slot: Option<u32>) -> u32 {
    let Some(t) = opening(h, w, slot, SIDE_OPENING) else {
        return 0;
    };
    cursor_plate(h, w, RIGHT_PANEL);
    slide_rows(h, t, RIGHT_PANEL, false);
    ripples(h, w, RIGHT_PANEL);
    rings(h);
    fade_shapes(h, w, Third::Right);
    h.call_sym(COMMAND_WINDOW, Conv::Cdecl, &[w]);
    cursor_if_shown(h, w);
    0
}
