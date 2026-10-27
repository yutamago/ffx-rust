//! List `h4-glue-other`, second agent (h4b), round 3 batch 2: the battle
//! wait window of the left-hand command list (`TODrawWaitBtlWinLeft`),
//! on the panel of [`crate::h4b_j`], with the sparkles over a full
//! overdrive-style badge.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`.

use ffx_bridge::x87::{add, div, ftol, ftol2, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{SHAPE_UV, SHAPE_UV_C2, int_f32};
use crate::h4_g1::COMMAND_WINDOW;
use crate::h4b_j::{
    CLEAR, RIGHT_PANEL, SHEET, Third, cursor_if_shown, cursor_plate, fade_shapes, half, konst,
    konst64, opening, rings, ripples, rx, rxk, ry, ryk, slide_rows,
};

/// `MsCheckLeftWindow(chr)` (cdecl): 0, 1 (a command without a cost) or
/// 2 (one the actor can pay).
pub const CHECK_LEFT: u32 = 0x0078_f700;

/// The sparkles' scale `C` (`0x00b92270`), their size `g` being from
/// `0x00b5ee60` to `0x00b4b390` (above it `0x00b5ee70`).
const SPARKLE_SCALE: u32 = 0x00b9_2270;
const SPARKLE_BASE: u32 = 0x00b4_b390;
/// The sparkle period `0x00b48308`, the first sparkle's window
/// `0x00b48328`.
const PERIOD: u32 = 0x00b4_8308;
const FIRST_END: u32 = 0x00b4_8328;
/// Full alpha (`0x00b4c170`).
const ALPHA: u32 = 0x00b4_c170;

/// A sparkle's arguments `x, y, w, h, u0, v0, u1, v1` for size `g` (`f32`
/// bits): `C g` square, at `0x00b5f230 - g`, `0x00b5f2e0 - g`; the remaps
/// in the original's order.
fn sparkle_args(h: &mut impl Host, g: u32) -> [u32; 8] {
    let gf = load(g);
    let uv = [0x00b5_f360, 0x00b5_eea4, 0x00b5_f37c, 0x00b5_f338].map(|c| konst(h, c));
    let size = store(mul(konst64(h, SPARKLE_SCALE), gf));
    let hh = ry(h, size);
    let ww = rx(h, size);
    let y = store(sub(konst64(h, 0x00b5_f2e0), gf));
    let y = ry(h, y);
    let x = store(sub(konst64(h, 0x00b5_f230), gf));
    let x = rx(h, x);
    [x, y, ww, hh, uv[0], uv[1], uv[2], uv[3]]
}

/// A growing sparkle `d` seconds into its time: size `g = d C K + B`
/// (`K` `0x00b5f1b8`), alpha `trunc(full - d P C)` (`P` `0x00b5ee40`).
fn growing(h: &mut impl Host, d: f64) {
    let c = konst64(h, SPARKLE_SCALE);
    let g = store(add(
        mul(mul(d, c), konst64(h, 0x00b5_f1b8)),
        konst64(h, SPARKLE_BASE),
    ));
    let alpha = ftol2(sub(
        konst64(h, ALPHA),
        mul(mul(d, konst64(h, 0x00b5_ee40)), c),
    ));
    let colour = (alpha << 24).wrapping_add(CLEAR);
    let a = sparkle_args(h, g);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[
            SHEET, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], colour, colour,
        ],
    );
}

/// The badge and its sparkles (left window, two or more usable
/// commands, after the ripples' start): the badge (shape -1, alpha
/// 0x25), then by `m`, the time since `0x00b92220` modulo
/// `0x00b48348`: a pulsing sparkle while `m <= 0x00b48328` (its size
/// `frac(m / P) × 0x00b5f1a8 + 0x00b5f1c0` held to `0x00b5ee60` below and
/// `0x00b5ee70` above `B`), and three growing ones for `m` in
/// `[0x00b3eb44, 0x00b3eb68]`, `[0x00b3eb48, 0x00b43d28]` and
/// `[0x00b48328, 0x00b5f188]`, from `P`, `0x00b48320` and `0x00b48328`.
fn sparkles(h: &mut impl Host, w: u32) {
    let badge = 0x2580_8080;
    let v1 = konst(h, 0x00b5_f33c);
    let u1 = konst(h, 0x00b4_1d90);
    let v0 = konst(h, 0x00b5_eea0);
    let u0 = konst(h, 0x00b5_eee8);
    let hh = ryk(h, 0x00b5_f308);
    let ww = rxk(h, 0x00b5_f308);
    let y = ryk(h, 0x00b5_f414);
    let x = rxk(h, 0x00b5_f334);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[SHEET, x, y, ww, hh, u0, v0, u1, v1, badge, badge],
    );
    let open: u32 = h.read(w.wrapping_add(0xe0));
    let s = store(sub(load(open), konst64(h, 0x00b9_2220)));
    let cycle = konst64(h, 0x00b4_8348);
    let q = div(load(s), cycle);
    let n = ftol(h, q);
    let m = store(mul(sub(q, load(int_f32(n.cast_signed()))), cycle));
    let m = load(m);
    // `fcomp`; `test ah, 1; jne`: off when 0x00b48328 < m or unordered
    if m <= konst64(h, FIRST_END) {
        let r = div(m, konst64(h, PERIOD));
        let n = ftol(h, r);
        let frac = sub(r, load(int_f32(n.cast_signed())));
        let g = store(add(
            mul(frac, konst64(h, 0x00b5_f1a8)),
            konst64(h, 0x00b5_f1c0),
        ));
        let low = konst(h, 0x00b5_ee60);
        let g = if load(g) < load(low) {
            low
        } else if load(g) > konst64(h, SPARKLE_BASE) {
            konst(h, 0x00b5_ee70)
        } else {
            g
        };
        let a = sparkle_args(h, g);
        h.call_sym(
            SHAPE_UV,
            Conv::Cdecl,
            &[SHEET, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]],
        );
    }
    if load(konst(h, 0x00b3_eb44)) <= m && m <= konst64(h, 0x00b3_eb68) {
        growing(h, sub(m, konst64(h, PERIOD)));
    }
    if load(konst(h, 0x00b3_eb48)) <= m && m <= load(konst(h, 0x00b4_3d28)) {
        growing(h, sub(m, konst64(h, 0x00b4_8320)));
    }
    let from = konst64(h, FIRST_END);
    if from <= m && m <= load(konst(h, 0x00b5_f188)) {
        growing(h, sub(m, from));
    }
}

/// `TODrawWaitBtlWinLeft(w)` (`0x008a3680`, cdecl): the left-hand command
/// list's wait window. The opening as the right window's
/// ([`opening`]); then the panel ([`RIGHT_PANEL`], the plates' UVs
/// flipped while `MsCheckLeftWindow(i16 +8)` is at least 1), the
/// ripples, the rings, with two or more usable commands and the window
/// open past `0x00b92194` the badge and its sparkles, the fading shapes
/// ([`Third::Left`]), the command window and the cursor when shown.
/// Returns 0.
///
/// `slot` is the window argument's word, as in
/// [`crate::h4b_j::wait_win_right`].
pub fn wait_win_left(h: &mut impl Host, w: u32, slot: Option<u32>) -> u32 {
    let Some(t) = opening(h, w, slot) else {
        return 0;
    };
    let chr = half(h, w, 8).cast_unsigned();
    let usable = h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]).cast_signed();
    cursor_plate(h, w, RIGHT_PANEL);
    slide_rows(h, t, RIGHT_PANEL, usable >= 1);
    let open: u32 = h.read(w.wrapping_add(0xe0));
    ripples(h, w, RIGHT_PANEL);
    rings(h);
    let chr = half(h, w, 8).cast_unsigned();
    let usable = h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]).cast_signed();
    // `fcomp`; `test ah, 5; jp`: on when 0x00b92194 < the open time
    if usable >= 2 && load(konst(h, 0x00b9_2194)) < load(open) {
        sparkles(h, w);
    }
    fade_shapes(h, w, Third::Left);
    h.call_sym(COMMAND_WINDOW, Conv::Cdecl, &[w]);
    cursor_if_shown(h, w);
    0
}
