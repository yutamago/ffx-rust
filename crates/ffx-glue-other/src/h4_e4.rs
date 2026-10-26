//! Continuation round 3 of list `h4-glue-other`, batch 4: UI records
//! handed to `graphicDrawUIElement`: the active character's cursor, the
//! sequence rectangle and the mode-2 gradient box.
//!
//! Each builds a record of 0x98 bytes in its frame (corners at `+0`,
//! `+0x20`, `+0x40`, colour words at `+0x10`, `+0x30`, `+0x50`) and hands
//! it to `graphicDrawUIElement(rec, texture, a, b, c)`. The words the
//! original leaves unset are 0 here (as `ffx_system`'s record builders
//! do); the tests zero the original's frame alike.
//!
//! The batch also holds the distortion grid's weights and the button
//! icons' UV, size and position remaps (x87 at 53 bits, as
//! [`crate::h4_e2`]).

// `fcom`/`fucom` compare exactly, as `==` does.
#![allow(clippy::float_cmp)]

use ffx_bridge::x87::{add, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{int_f32, pass};

/// `graphicDrawUIElement(rec, texture, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// Bytes of a UI record.
pub const RECORD: u32 = 0x98;

/// Zeroes the record at `rec`.
fn clear(h: &mut impl Host, rec: u32) {
    for k in 0..RECORD / 4 {
        h.write(rec.wrapping_add(4 * k), 0u32);
    }
}

/// The colour `c`'s bytes (r, g, b, a) as four words at `at`.
fn colour(h: &mut impl Host, at: u32, c: u32) {
    for k in 0..4u32 {
        h.write(at.wrapping_add(4 * k), (c >> (8 * k)) & 0xff);
    }
}

/// `TOMkpActiveChrCursor(x0, y0, x1, y1, x2, y2, c1, c0)` (cdecl; `rec`
/// its record at `ebp - 0x9c`): the triangle cursor over the active
/// character: corners `(float)` of the six integers, colour `c0` at the
/// first corner and `c1` at the other two, `+0x80` 0x14;
/// `graphicDrawUIElement(rec, 0, 0, 1, 0)`. Returns its `eax`.
#[allow(clippy::too_many_arguments)]
pub fn active_chr_cursor(h: &mut impl Host, rec: u32, xy: [u32; 6], c1: u32, c0: u32) -> u32 {
    clear(h, rec);
    for (k, off) in [0u32, 4, 0x20, 0x24, 0x40, 0x44].into_iter().enumerate() {
        h.write(rec.wrapping_add(off), int_f32(xy[k].cast_signed()));
    }
    colour(h, rec.wrapping_add(0x10), c0);
    colour(h, rec.wrapping_add(0x30), c1);
    colour(h, rec.wrapping_add(0x50), c1);
    h.write(rec.wrapping_add(0x80), 0x14u32);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, 0, 0, 1, 0])
}

/// `TOSeqGetSceneState_008ae530(i, name)` (cdecl).
pub const SCENE_STATE: u32 = 0x008a_e530;
/// The scene slot's name, "bwakk" (`0x00b5ed2c`).
pub const BWAKK: u32 = 0x00b5_ed2c;

/// `TOSeqDrawRect_008fdf20(p)` (cdecl; `rec` its record at `ebp -
/// 0x9c`): the rectangle of `p` (i16 x `+0xc`, y `+0xe`, w `+0x10`, h
/// `+0x12`; colours: bytes `+4..+8` top left, `+8..+0xc` bottom right) as
/// `graphicDrawUIElement(rec, 0, 1, 0, 0)`. While scene 2's "bwakk" slot
/// is in state 3, a 7 x 0x16 rectangle starts at y 0x43 (written back).
/// Returns the draw's `eax`.
pub fn seq_draw_rect(h: &mut impl Host, rec: u32, p: u32) -> u32 {
    clear(h, rec);
    let name = h.rebase(BWAKK);
    if h.call_sym(SCENE_STATE, Conv::Cdecl, &[2, name]) == 3
        && h.read::<u16>(p.wrapping_add(0xe)) == 7
        && h.read::<u16>(p.wrapping_add(0x12)) == 0x16
    {
        h.write(p.wrapping_add(0xe), 0x43u16);
    }
    for k in 0..4u32 {
        let v = u32::from(h.read::<u8>(p.wrapping_add(4 + k)));
        h.write(rec.wrapping_add(0x10 + 4 * k), v);
        let v = u32::from(h.read::<u8>(p.wrapping_add(8 + k)));
        h.write(rec.wrapping_add(0x30 + 4 * k), v);
    }
    let x = i32::from(h.read::<i16>(p.wrapping_add(0xc)));
    let y = i32::from(h.read::<i16>(p.wrapping_add(0xe)));
    let w = i32::from(h.read::<i16>(p.wrapping_add(0x10)));
    let hh = i32::from(h.read::<i16>(p.wrapping_add(0x12)));
    h.write(rec, int_f32(x));
    h.write(rec.wrapping_add(4), int_f32(y));
    h.write(rec.wrapping_add(0x20), int_f32(x.wrapping_add(w)));
    h.write(rec.wrapping_add(0x24), int_f32(y.wrapping_add(hh)));
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, 0, 1, 0, 0])
}

/// `BeginScissor_XY(&x0, &y0, &x1, &y1)` (cdecl; 0 when clipped away).
pub const BEGIN_SCISSOR: u32 = 0x008e_58e0;
/// Bytes of [`cross_box_mode2`]'s frame: the corners (y0, x0, y1, x1)
/// then the record.
pub const BOX_FRAME: u32 = 0x10 + RECORD;

/// `TODrawCrossBoxXYWHC2Mode2_008f4df0(x, y, w, h, c0, c1)` (cdecl; `frame`
/// from `ebp - 0xac`: y0, x0, y1, x1, then the record): as
/// `TODrawCrossBoxXYWHC2` (an untextured gradient box from `c0` to `c1`,
/// corners `x, y` and `x + w, y + h` clipped by `BeginScissor_XY`), drawn
/// with `graphicDrawUIElement(rec, 0, 1, 2, 0)`. Returns 0 when clipped
/// away, else the draw's `eax`.
#[allow(clippy::many_single_char_names)]
pub fn cross_box_mode2(h: &mut impl Host, frame: u32, [x, y, w, hh, c0, c1]: [u32; 6]) -> u32 {
    let (y0, x0, y1, x1) = (frame, frame + 4, frame + 8, frame + 0xc);
    let rec = frame + 0x10;
    clear(h, rec);
    h.write(x0, pass(x));
    h.write(y0, pass(y));
    h.write(x1, store(add(load(x), load(w))));
    h.write(y1, store(add(load(y), load(hh))));
    if h.call_sym(BEGIN_SCISSOR, Conv::Cdecl, &[x0, y0, x1, y1]) == 0 {
        return 0;
    }
    for (from, off) in [(x0, 0u32), (y0, 4), (x1, 0x20), (y1, 0x24)] {
        let v = pass(h.read(from));
        h.write(rec.wrapping_add(off), v);
    }
    colour(h, rec.wrapping_add(0x10), c0);
    colour(h, rec.wrapping_add(0x30), c1);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[rec, 0, 1, 2, 0])
}

// ---- the distortion grid's weights

/// `Distortion_fillGridOnes_006c28e0(this)` (thiscall): the distortion
/// grid's weights (`[this + 0x6c]`: 9 rows of 36 floats) all 1.0. Returns
/// what the original leaves in `eax`: its write cursor, the grid plus
/// 0x528.
pub fn distortion_fill_ones(h: &mut impl Host, this: u32) -> u32 {
    let grid: u32 = h.read(this.wrapping_add(0x6c));
    let one = 1f32.to_bits();
    for k in 0..9 * 36u32 {
        h.write(grid.wrapping_add(4 * k), one);
    }
    grid.wrapping_add(0x18 + 9 * 0x90)
}

// ---- button icons

/// The icon cell's width in UV (`0x00b4b448`, f64 1/12).
pub const ICON_U_SIZE: u32 = 0x00b4_b448;
/// The icon cell's height in UV (`0x00b92200`, f64 1/8).
pub const ICON_V_SIZE: u32 = 0x00b9_2200;

/// `RemapButtonIcon_remapIconUV(kind, rec, u, v)` (stdcall, ret 0x10): the
/// UV corners of UI record `rec` (`+8`, `+0x28`, `+0x48`, `+0x68`) moved
/// to the icon cell at `(u, v)` (cell size `0x00b4b448` by `0x00b92200`):
/// a plain icon gets only its first two corners (`u, v` and `u + du, v +
/// dv`). Kind 4 (sign bit ignored), kinds 6 and 0xb fill all four corners:
/// a kind-4 record already laid out left to right (`+0x28 > +8`, `+0x2c ==
/// +0xc`) keeps that orientation, any other is turned a quarter (as kinds
/// 6 and 0xb always are, with their bottom corners swapped).
pub fn remap_icon_uv(h: &mut impl Host, kind: u32, rec: u32, u: u32, v: u32) {
    let at = |off: u32| rec.wrapping_add(off);
    let du: f64 = h.read(h.rebase(ICON_U_SIZE));
    let dv: f64 = h.read(h.rebase(ICON_V_SIZE));
    let (u, v) = (pass(u), pass(v));
    let u1 = store(add(load(u), du));
    let v1 = store(add(load(v), dv));
    if kind & 0x7fff_ffff == 4 {
        let a = load(h.read(at(8)));
        let b = load(h.read(at(0x28)));
        let c = load(h.read(at(0xc)));
        let d = load(h.read(at(0x2c)));
        let corners = if b > a && d == c {
            [
                (8, u),
                (0xc, v),
                (0x28, u1),
                (0x48, u1),
                (0x2c, v),
                (0x4c, v1),
                (0x6c, v1),
                (0x68, u),
            ]
        } else {
            [
                (8, u),
                (0xc, v),
                (0x28, u),
                (0x2c, v1),
                (0x48, u1),
                (0x68, u1),
                (0x4c, v1),
                (0x6c, v),
            ]
        };
        for (off, x) in corners {
            h.write(at(off), x);
        }
    } else if kind == 6 || kind == 0xb {
        for (off, x) in [
            (8, u),
            (0xc, v),
            (0x28, u),
            (0x2c, v1),
            (0x68, u1),
            (0x48, u1),
            (0x6c, v1),
            (0x4c, v),
        ] {
            h.write(at(off), x);
        }
    } else {
        for (off, x) in [(8, u), (0xc, v), (0x28, u1), (0x2c, v1)] {
            h.write(at(off), x);
        }
    }
}

/// `Virtuos_InputManager_getInstance()` (cdecl).
pub const INPUT_INSTANCE: u32 = 0x0063_1300;
/// `Virtuos_InputManager_getVibrationFlag(manager)` (thiscall).
pub const INPUT_VIBRATION: u32 = 0x0063_1320;
/// The icon's width scale (`0x00b43d90`, f64 9.0).
pub const ICON_W_SCALE: u32 = 0x00b4_3d90;
/// The icon's height scale (`0x00b4b468`, f64 12.0).
pub const ICON_H_SCALE: u32 = 0x00b4_b468;

/// `RemapButtonIcon_remapIconSize(this; kind, rec)` (thiscall, ret 8):
/// after asking the input manager's vibration flag (unused), the far
/// corners of UI record `rec` are placed at `x + this+0x160 * 9`, `y +
/// this+0x164 * 12` (`x`, `y` its first corner; the factors are the f64s
/// at `0x00b43d90`/`0x00b4b468`): the second corner for a plain icon
/// (kind 1, sign bit ignored, keeps its size), all three for kind 4 (as
/// [`remap_icon_uv`] orients them) and kinds 6 and 0xb.
pub fn remap_icon_size(h: &mut impl Host, this: u32, kind: u32, rec: u32) {
    let at = |off: u32| rec.wrapping_add(off);
    let manager = h.call_sym(INPUT_INSTANCE, Conv::Cdecl, &[]);
    h.call_sym(INPUT_VIBRATION, Conv::Thiscall(manager), &[]);
    let kw: f64 = h.read(h.rebase(ICON_W_SCALE));
    let kh: f64 = h.read(h.rebase(ICON_H_SCALE));
    let sw = load(h.read(this.wrapping_add(0x160)));
    let sh = load(h.read(this.wrapping_add(0x164)));
    let x = store(add(mul(sw, kw), load(h.read(rec))));
    let y = store(add(mul(sh, kh), load(h.read(at(4)))));
    let corners: &[(u32, u32)] = if kind == 0xb || kind == 6 {
        &[(0x40, x), (0x60, x), (0x24, y), (0x64, y)]
    } else if kind & 0x7fff_ffff == 4 {
        let a = load(h.read(at(8)));
        let b = load(h.read(at(0x28)));
        let c = load(h.read(at(0xc)));
        let d = load(h.read(at(0x2c)));
        if b > a && d == c {
            &[(0x20, x), (0x40, x), (0x44, y), (0x64, y)]
        } else {
            &[(0x60, x), (0x40, x), (0x44, y), (0x24, y)]
        }
    } else if kind & 0x7fff_ffff == 1 {
        &[]
    } else {
        &[(0x20, x), (0x24, y)]
    };
    for &(off, v) in corners {
        h.write(at(off), v);
    }
}

/// The button offsets' f64 constants: 5.0 (`0x00b41d20`), 5.5
/// (`0x00b3eb80`), 4.0 (`0x00b43d80`), 1.5 (`0x00b41d00`), 4.5
/// (`0x00b4b458`).
pub const BUTTON_OFFSETS: [u32; 5] = [
    0x00b4_1d20,
    0x00b3_eb80,
    0x00b4_3d80,
    0x00b4_1d00,
    0x00b4_b458,
];
/// The f32 offset pairs of kinds 9, 0xc and 0xd (`0x00ce6df4`,
/// `0x00ce6dd0`, `0x00ce6dd8`: x, then y).
pub const BUTTON_OFFSET_GLOBALS: [u32; 3] = [0x00ce_6df4, 0x00ce_6dd0, 0x00ce_6dd8];

/// How a kind moves a button.
enum Shift {
    /// Both x by `dx`, both y by `dy` (f64 constants; `true` subtracts).
    Const((u32, bool), (u32, bool)),
    /// By the f32 pair at the global.
    Global(u32),
}

/// `RemapButtonIcon_remapUIButtonPosition(a, rec, b, kind)` (stdcall, ret
/// 0x10; `a`, `b` unused): moves UI record `rec`'s two corners (`+0`,
/// `+4` and `+0x20`, `+0x24`) by the button kind's offset: kind 0 down 5,
/// 1 up 5.5, 2 right 4 and up 1.5, 3 left 4 and up 1.5, 4..7 down 4.5,
/// 8, 0xa, 0xb down 1.5 (x by +0.0 in all of these), 9, 0xc, 0xd by the
/// f32 pairs at `0x00ce6df4`, `0x00ce6dd0`, `0x00ce6dd8`. Returns `rec`,
/// or `kind` for any other kind (nothing moved).
pub fn remap_ui_button_position(h: &mut impl Host, rec: u32, kind: u32) -> u32 {
    let [k5, k55, k4, k15, k45] = BUTTON_OFFSETS;
    let zero = (0, false);
    let shift = match kind {
        0 => Shift::Const(zero, (k5, false)),
        1 => Shift::Const(zero, (k55, true)),
        2 => Shift::Const((k4, false), (k15, true)),
        3 => Shift::Const((k4, true), (k15, true)),
        4..=7 => Shift::Const(zero, (k45, false)),
        8 | 0xa | 0xb => Shift::Const(zero, (k15, false)),
        9 => Shift::Global(BUTTON_OFFSET_GLOBALS[0]),
        0xc => Shift::Global(BUTTON_OFFSET_GLOBALS[1]),
        0xd => Shift::Global(BUTTON_OFFSET_GLOBALS[2]),
        _ => return kind,
    };
    match shift {
        Shift::Const(dx, dy) => {
            for (first, second, (sym, minus)) in [(0u32, 0x20u32, dx), (4, 0x24, dy)] {
                let d: f64 = if sym == 0 { 0.0 } else { h.read(h.rebase(sym)) };
                let a = load(h.read(rec.wrapping_add(first)));
                let v = if minus { sub(a, d) } else { add(a, d) };
                h.write(rec.wrapping_add(first), store(v));
                let b = load(h.read(rec.wrapping_add(second)));
                let v = if minus { sub(b, d) } else { add(d, b) };
                h.write(rec.wrapping_add(second), store(v));
            }
        }
        Shift::Global(sym) => {
            let at = h.rebase(sym);
            for (off, g) in [(0u32, at), (0x20, at), (4, at + 4), (0x24, at + 4)] {
                let v = add(load(h.read(rec.wrapping_add(off))), load(h.read(g)));
                h.write(rec.wrapping_add(off), store(v));
            }
        }
    }
    rec
}
