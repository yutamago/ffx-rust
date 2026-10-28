//! List `h4-glue-other`, second agent (h4b), round 3 batch 3: the battle
//! wait window of the main command list (`TODrawWaitBtlWinMain`): the
//! panel and decorations of [`crate::h4b_j`] and [`crate::h4b_k`], the
//! badges for one, two or three usable commands, the scroll bar and the
//! rows of command names.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`.

use ffx_bridge::x87::{add, div, ftol, ftol2, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{SHAPE_UV_C2, int_f32};
use crate::h4_g1::{SET_CUR_POS, WAIT_WIN_SKIP};
use crate::h4b_j::{
    MAIN_OPENING, RIGHT_PANEL, ROW_PITCH, SHEET, Third, cursor_plate, fade_shapes, half, idiv, int,
    konst, konst64, opening, pulse, rings, ripples, rx, rxk, ryk, slide_rows, unit,
};
use crate::h4b_k::{CHECK_LEFT, sparkle_window};

/// `DrawCrossMenuScrollParts(x, y, w, h, top, rows, total)` (cdecl).
pub const SCROLL_PARTS: u32 = 0x008e_6cc0;
/// `TOMakePktScissor(x0, y0, x1, y1)` (cdecl).
pub const PKT_SCISSOR: u32 = 0x008f_dee0;
/// `TOCheckBtlCommandUse(chr, command)` (cdecl): -4 hides a command.
pub const CHECK_USE: u32 = 0x0089_aca0;
/// `MsGetComData(command, &strings)` (cdecl): the command's record.
pub const COM_DATA: u32 = 0x0079_a4b0;
/// `ToGetBtlEasyFontWidth(text, &width, alpha, scale, extra)` (cdecl).
pub const FONT_WIDTH: u32 = 0x0090_5290;
/// `ToMakeBtlEasyEdgeFont(text, x, y, alpha, scale, extra)` (cdecl).
pub const EDGE_FONT: u32 = 0x0090_5930;

/// Bytes of [`wait_win_main`]'s frame: the width `ToGetBtlEasyFontWidth`
/// writes (`ebp - 0x30`) and the strings pointer `MsGetComData` writes
/// (`ebp - 0x2c`).
pub const MAIN_FRAME: u32 = 8;

/// The constants of a badge's three pieces: `a` the plate (`v1, u1, v0,
/// u0`, then the height, width and y remapped), `b_mul` the blinking
/// strip's travel and `b_y` its y, `c` the label (`v1, u1, v0, u0`, the
/// width and y remapped).
#[derive(Clone, Copy)]
struct Badge {
    a: [u32; 7],
    b_mul: u32,
    b_y: u32,
    c: [u32; 6],
}

/// One usable command's badge.
const ONE: Badge = Badge {
    a: [
        0x00b5_f340,
        0x00b5_efd4,
        0x00b4_1318,
        0x00b5_f378,
        0x00b5_f2f0,
        0x00b5_f3b0,
        0x00b5_f3f0,
    ],
    b_mul: 0x00b5_f278,
    b_y: 0x00b5_f3f4,
    c: [
        0x00b5_f0f8,
        0x00b5_f0e0,
        0x00b5_f0f0,
        0x00b5_f0a8,
        0x00b5_f390,
        0x00b5_f3f8,
    ],
};

/// Two usable commands' badge.
const TWO: Badge = Badge {
    a: [
        0x00b5_f320,
        0x00b5_f3dc,
        0x00b5_f30c,
        0x00b5_f380,
        0x00b4_1d84,
        0x00b5_f394,
        0x00b5_f3f0,
    ],
    b_mul: 0x00b5_f268,
    b_y: 0x00b5_f3f4,
    c: [
        0x00b5_f0e4,
        0x00b5_f0bc,
        0x00b5_f0d0,
        0x00b5_8354,
        0x00b5_f364,
        0x00b5_f3f8,
    ],
};

/// `badge` with its plate, strip and label rows at the y constants
/// `ys` (the three commands' badge draws two's one row up).
const fn moved(badge: Badge, ys: [u32; 3]) -> Badge {
    let mut b = badge;
    b.a[6] = ys[0];
    b.b_y = ys[1];
    b.c[5] = ys[2];
    b
}

/// The badges' slide: `(1 - p) × 0x00b5f270`, `p` the fade of
/// [`fade_shapes`] (`f32` bits).
fn badge_offset(h: &mut impl Host, w: u32) -> u32 {
    let open: u32 = h.read(w.wrapping_add(0xe0));
    let p = store(div(
        sub(load(open), konst64(h, 0x00b3_eb60)),
        konst64(h, 0x00b4_8308),
    ));
    let p = unit(load(p));
    store(mul(sub(1.0, p), konst64(h, 0x00b5_f270)))
}

/// A badge's plate (shape -1, pulsing like the cursor plate) at the
/// slide plus `0x00b5f220` (`x`, `f32` bits).
fn badge_plate(h: &mut impl Host, b: Badge, x: u32) {
    let colour = pulse(h);
    let [v1, u1, v0, u0] = [b.a[0], b.a[1], b.a[2], b.a[3]].map(|c| konst(h, c));
    let hh = ryk(h, b.a[4]);
    let ww = rxk(h, b.a[5]);
    let y = ryk(h, b.a[6]);
    let xx = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[SHEET, xx, y, ww, hh, u0, v0, u1, v1, colour, colour],
    );
}

/// The badges' blink: `f` the fraction of `(open - 0x00b3eb60) ×
/// 0x00b92220` (`f32` bits) and its colour, alpha `trunc(G - f G)` (`G`
/// `0x00b41308`).
fn blink(h: &mut impl Host, w: u32) -> (u32, u32) {
    let open: u32 = h.read(w.wrapping_add(0xe0));
    let v = mul(
        sub(load(open), konst64(h, 0x00b3_eb60)),
        konst64(h, 0x00b9_2220),
    );
    let n = ftol(h, v);
    let f = store(sub(v, int(n.cast_signed())));
    let g = konst64(h, 0x00b4_1308);
    let alpha = ftol2(sub(g, mul(load(f), g)));
    (f, (alpha << 24).wrapping_add(0x0080_8080))
}

/// A badge's blinking strip at `f × b_mul + 0x00b5f220` plus the slide.
fn badge_strip(h: &mut impl Host, b: Badge, off: u32, f: u32, colour: u32) {
    let [v1, u1, v0, u0] =
        [0x00b5_f310, 0x00b5_f458, 0x00b5_f300, 0x00b5_f454].map(|c| konst(h, c));
    let hh = ryk(h, 0x00b5_ee6c);
    let ww = rxk(h, 0x00b1_f2c8);
    let y = ryk(h, b.b_y);
    let x = store(add(
        add(mul(load(f), konst64(h, b.b_mul)), konst64(h, 0x00b5_f220)),
        load(off),
    ));
    let xx = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[SHEET, xx, y, ww, hh, u0, v0, u1, v1, colour, colour],
    );
}

/// A badge's label (shape 0x190) at the slide plus `0x00b5f238` (`x`,
/// `f32` bits).
fn badge_label(h: &mut impl Host, b: Badge, x: u32) {
    let grey = 0x8080_8080;
    let [v1, u1, v0, u0] = [b.c[0], b.c[1], b.c[2], b.c[3]].map(|c| konst(h, c));
    let hh = ryk(h, 0x00b5_4f90);
    let ww = rxk(h, b.c[4]);
    let y = ryk(h, b.c[5]);
    let xx = rx(h, x);
    h.call_sym(
        SHAPE_UV_C2,
        Conv::Cdecl,
        &[0x190, xx, y, ww, hh, u0, v0, u1, v1, grey, grey],
    );
}

/// The badge of one or two usable commands: plate, strip, label.
fn badge(h: &mut impl Host, w: u32, b: Badge) {
    let off = badge_offset(h, w);
    let x = store(add(load(off), konst64(h, 0x00b5_f220)));
    badge_plate(h, b, x);
    let (f, colour) = blink(h, w);
    badge_strip(h, b, off, f, colour);
    let x = store(add(load(off), konst64(h, 0x00b5_f238)));
    badge_label(h, b, x);
}

/// Three usable commands: two's badge one row up, then one's.
fn badge3(h: &mut impl Host, w: u32) {
    let up = moved(TWO, [0x00b5_f3e4, 0x00b5_f3e8, 0x00b5_f3ec]);
    let off = badge_offset(h, w);
    let x = store(add(load(off), konst64(h, 0x00b5_f220)));
    badge_plate(h, up, x);
    badge_plate(h, ONE, x);
    let (f, colour) = blink(h, w);
    badge_strip(h, up, off, f, colour);
    badge_strip(h, ONE, off, f, colour);
    let x = store(add(load(off), konst64(h, 0x00b5_f238)));
    badge_label(h, up, x);
    badge_label(h, ONE, x);
}

/// The scroll bar (`DrawCrossMenuScrollParts`): at remapped
/// (`0x00b49990`, `0x00b4ae1c`), `0x00b92314` wide and 154 high, the
/// first visible line `+0x40 / +0x38`, `+0x3a` visible lines of `lines`.
fn scroll_bar(h: &mut impl Host, w: u32, lines: u16) {
    let cols = half(h, w, 0x38);
    let total = i32::from(lines.cast_signed()).cast_unsigned();
    let rows = half(h, w, 0x3a).cast_unsigned();
    let top = idiv(half(h, w, 0x40), cols).0.cast_unsigned();
    let hh = ryk(h, 0x00b5_f344);
    let ww = rxk(h, 0x00b9_2314);
    let y = ryk(h, 0x00b4_ae1c);
    let x = rxk(h, 0x00b4_9990);
    h.call_sym(SCROLL_PARTS, Conv::Cdecl, &[x, y, ww, hh, top, rows, total]);
}

/// The visible entries: from the first visible one (0 at least, the
/// count at most) for `cols × rows` entries, within the count.
fn visible(h: &impl Host, w: u32, count: i32) -> (i32, i32) {
    let first = half(h, w, 0x40);
    let start = if first > count {
        count
    } else if first < 0 {
        0
    } else {
        first
    };
    let end = half(h, w, 0x38)
        .wrapping_mul(half(h, w, 0x3a))
        .wrapping_add(start);
    let end = if end > count {
        count
    } else if end < 0 {
        0
    } else {
        end
    };
    (start, end)
}

/// The command names: each visible entry not `0xff` that
/// `TOCheckBtlCommandUse` does not hide (-4) has its name
/// (`MsGetComData`'s text) centred in its cell (`ToGetBtlEasyFontWidth`,
/// `ToMakeBtlEasyEdgeFont`; cells `+0x62` + 10 apart, rows 0x34 from
/// `+0x68`, `+0x7c`), scale `0x00b5ee10`. `alpha` is the frame word whose
/// low byte becomes whether the command is unusable (its upper bytes, the
/// fade's slide's, go to the callees as the original leaves them).
fn names(h: &mut impl Host, w: u32, frame: u32, mut alpha: u32) {
    let width = frame;
    let strings = frame.wrapping_add(4);
    let count: i32 = h.read(w.wrapping_add(0x24));
    let (start, end) = visible(h, w, count);
    let one = 1f32.to_bits();
    for c in start..end {
        let list: u32 = h.read(w.wrapping_add(0x20));
        let at = list.wrapping_add(c.wrapping_mul(2).cast_unsigned());
        let id: u16 = h.read(at);
        if id == 0xff {
            continue;
        }
        let chr = half(h, w, 8).cast_unsigned();
        let command = i32::from(id.cast_signed()).cast_unsigned();
        let usable = h.call_sym(CHECK_USE, Conv::Cdecl, &[chr, command]);
        if usable.cast_signed() == -4 {
            continue;
        }
        let list: u32 = h.read(w.wrapping_add(0x20));
        let at = list.wrapping_add(c.wrapping_mul(2).cast_unsigned());
        let command = i32::from(h.read::<i16>(at)).cast_unsigned();
        // junk read: [ebp-0x2c] never written when MsGetComData does not
        // write it (command kinds 1, 9 and past 0xb); 0 (brief 2026-10-24)
        let record = h.call_sym(COM_DATA, Conv::Cdecl, &[command, strings]);
        let base: u32 = h.read(strings);
        let text = u32::from(h.read::<u16>(record)).wrapping_add(base);
        let scale = konst(h, 0x00b5_ee10);
        alpha = (alpha & !0xff) | u32::from(usable.cast_signed() < 0);
        h.call_sym(FONT_WIDTH, Conv::Cdecl, &[text, width, alpha, scale, one]);
        let pitch = half(h, w, 0x62);
        let cols = half(h, w, 0x38);
        let measured: u32 = h.read(width);
        let left: u32 = h.read(w.wrapping_add(0x68));
        let x0 = ftol(
            h,
            add(
                mul(
                    sub(f64::from(pitch), load(measured)),
                    konst64(h, 0x00b9_2220),
                ),
                load(left),
            ),
        );
        let (_, col) = idiv(c, cols);
        let x = x0
            .cast_signed()
            .wrapping_add(col.wrapping_mul(pitch.wrapping_add(10)));
        let first = half(h, w, 0x40);
        let row = idiv(c.wrapping_sub(first), cols).0;
        let y = crate::h4b_j::remap_y(h, int_f32(row.wrapping_mul(ROW_PITCH)));
        let top: u32 = h.read(w.wrapping_add(0x7c));
        let y = ftol(h, add(y, load(top)));
        let scale = konst(h, 0x00b5_ee10);
        h.call_sym(
            EDGE_FONT,
            Conv::Cdecl,
            &[
                text,
                int_f32(x),
                int_f32(y.cast_signed()),
                alpha,
                scale,
                one,
            ],
        );
    }
}

/// `TODrawWaitBtlWinMain(w)` (`0x008a49b0`, cdecl): the main command
/// list's wait window. Nothing while the wait windows are off
/// (`0x0133d6e0` 1; `None`, the caller's `eax`). The open time; the
/// line count `(+0x24 - 1 + cols) / cols` (16 bits); while the opening
/// runs only its plate ([`MAIN_OPENING`]); then the panel as the left
/// window's, with one, two or three usable commands
/// (`MsCheckLeftWindow`, asked again for each) the badge, the ripples,
/// the scroll bar, the rings, the sparkles as the left window's, the
/// fading shapes ([`Third::Main`]), the scissor of the box `+0x78` to
/// `+0x84`, the window's draw step `+0x88`, the command names and the
/// cursor at `ftol(+0x78)` and `ftol(remapY2(row × 0x34 + 5) + +0x7c)`
/// (`TOBtlSetCurPos`). Returns 0.
///
/// `frame` holds the two words the original's callees write
/// ([`MAIN_FRAME`]). A zero column count faults in the original (`idiv`
/// at the start); the port takes 0 there.
#[allow(clippy::too_many_lines)]
pub fn wait_win_main(h: &mut impl Host, w: u32, frame: u32) -> Option<u32> {
    if WAIT_WIN_SKIP.get(h) == 1 {
        return None;
    }
    let Some(t) = opening(h, w, None, MAIN_OPENING) else {
        return Some(0);
    };
    let cols = half(h, w, 0x38);
    let count: i32 = h.read(w.wrapping_add(0x24));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // movzx ax
    let lines = idiv(count.wrapping_sub(1).wrapping_add(cols), cols).0 as u16;
    let chr = half(h, w, 8).cast_unsigned();
    let usable = h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]).cast_signed();
    cursor_plate(h, w, RIGHT_PANEL);
    slide_rows(h, t, RIGHT_PANEL, usable >= 1);
    if usable >= 1 {
        let chr = half(h, w, 8).cast_unsigned();
        if h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]) == 1 {
            badge(h, w, ONE);
        } else {
            let chr = half(h, w, 8).cast_unsigned();
            if h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]) == 2 {
                badge(h, w, TWO);
            } else {
                let chr = half(h, w, 8).cast_unsigned();
                if h.call_sym(CHECK_LEFT, Conv::Cdecl, &[chr]) == 3 {
                    badge3(h, w);
                }
            }
        }
    }
    let open: u32 = h.read(w.wrapping_add(0xe0));
    ripples(h, w, RIGHT_PANEL);
    scroll_bar(h, w, lines);
    rings(h);
    sparkle_window(h, w, open);
    let slide = fade_shapes(h, w, Third::Main);
    // the slide's `fstp qword [ebp-0x20]` leaves its high dword in the
    // word the names' alpha byte goes into
    #[allow(clippy::cast_possible_truncation)]
    let alpha = (slide.to_bits() >> 32) as u32;
    let edges = [0x84u32, 0x80, 0x7c, 0x78].map(|o| {
        let v: u32 = h.read(w.wrapping_add(o));
        ftol(h, load(v))
    });
    h.call_sym(
        PKT_SCISSOR,
        Conv::Cdecl,
        &[edges[3], edges[2], edges[1], edges[0]],
    );
    let step: u32 = h.read(w.wrapping_add(0x88));
    h.call(step, Conv::Cdecl, &[w]);
    names(h, w, frame, alpha);
    let first = half(h, w, 0x40);
    let cursor = half(h, w, 0x42);
    let row = idiv(cursor.wrapping_sub(first), half(h, w, 0x38)).0;
    let y = crate::h4b_j::remap_y(h, int_f32(row.wrapping_mul(ROW_PITCH).wrapping_add(5)));
    let top: u32 = h.read(w.wrapping_add(0x7c));
    let y = ftol(h, add(y, load(top)));
    let x = ftol(h, load(h.read(w.wrapping_add(0x78))));
    h.call_sym(SET_CUR_POS, Conv::Cdecl, &[0, x, y]);
    Some(0)
}
