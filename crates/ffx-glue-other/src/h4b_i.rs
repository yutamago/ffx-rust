//! List `h4-glue-other`, second agent (h4b), round 2 batch 4: the
//! menus' frame shapes.

use ffx_bridge::x87::{add, load, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{REMAP_X2, REMAP_Y2, pass};

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// `remap(v)` through its `ST0`.
fn remap(h: &mut impl Host, f: u32, v: u32) -> f64 {
    h.call_sym_st0_f64(f, Conv::Cdecl, &[v])
}

// ---- TODrawFrameShape_008f9c20

/// `TOMkpShapeXYWHUV(id, x, y, w, h, u0, v0, u1, v1)` (cdecl).
pub const SHAPE_UV: u32 = 0x0090_3bb0;
/// The frame's corner size, 3 (`f32`, remapped).
const CORNER: u32 = 0x00b3_eb94;
/// Two corners, 6 (`f32`, remapped).
const CORNERS: u32 = 0x00b9_2308;

/// A cell of [`frame_shape`]'s 3 x 3 grid: 0 the near edge (the frame's
/// x or y, a corner wide), 1 the middle (a corner in, the size less two
/// corners), 2 the far edge (a corner before the end).
#[derive(Clone, Copy)]
struct Cell {
    col: u8,
    row: u8,
    id: u32,
    uv: [Uv; 4],
}

/// A UV word: an image constant or zero.
#[derive(Clone, Copy)]
enum Uv {
    Image(u32),
    Zero,
}

/// The frame of [`frame_shape`]: `x`, `y`, `w`, `h`, `x + w`, `y + h`.
#[derive(Clone, Copy)]
struct Frame {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    right: f64,
    bottom: f64,
}

/// Draws `cell` (`TOMkpShapeXYWHUV`), its remaps in the original's order
/// (height, width, y, x); returns the call's `eax` and the y and x it
/// passed.
fn frame_cell(h: &mut impl Host, f: Frame, cell: Cell) -> (u32, u32, u32) {
    let hh = match cell.row {
        1 => store(sub(load(f.h), remap(h, REMAP_Y2, konst(h, CORNERS)))),
        _ => store(remap(h, REMAP_Y2, konst(h, CORNER))),
    };
    let w = match cell.col {
        1 => store(sub(load(f.w), remap(h, REMAP_X2, konst(h, CORNERS)))),
        _ => store(remap(h, REMAP_X2, konst(h, CORNER))),
    };
    let y = match cell.row {
        0 => pass(f.y),
        1 => store(add(remap(h, REMAP_Y2, konst(h, CORNER)), load(f.y))),
        _ => store(sub(f.bottom, remap(h, REMAP_Y2, konst(h, CORNER)))),
    };
    let x = match cell.col {
        0 => pass(f.x),
        1 => store(add(remap(h, REMAP_X2, konst(h, CORNER)), load(f.x))),
        _ => store(sub(f.right, remap(h, REMAP_X2, konst(h, CORNER)))),
    };
    let uv = cell.uv.map(|u| match u {
        Uv::Image(c) => konst(h, c),
        Uv::Zero => 0,
    });
    let eax = h.call_sym(
        SHAPE_UV,
        Conv::Cdecl,
        &[cell.id, x, y, w, hh, uv[0], uv[1], uv[2], uv[3]],
    );
    (eax, y, x)
}

/// `TODrawFrameShape_008f9c20(x, y, w, h, kind)` (cdecl, floats): a
/// window frame from `TOMkpShapeXYWHUV(200, ...)` pieces on a 3 x 3 grid
/// of corners 3 wide (`remapX2`/`remapY2(0x00b3eb94)`; the middle is the
/// size less `remap(0x00b92308)`). Kinds other than 0, 4 and 5 first draw
/// a strip under the frame (`remapY2(0x00b41d80)` high); kinds 1, 2, 4
/// and 5 (5 as shape -1) fill the middle with their own image cell, kind
/// 3 the whole frame; then the eight edge and corner pieces. Each piece
/// calls the remaps in the original's order and stores its edges as
/// `f32`. Returns the last piece's `eax`.
///
/// `slots` is the argument words' address: the original computes its
/// temporaries in the `x`, `w` and `kind` slots and leaves there the
/// last piece's x, the bottom middle's y and the bottom left's y.
#[allow(clippy::too_many_lines)]
pub fn frame_shape(
    h: &mut impl Host,
    x: u32,
    y: u32,
    w: u32,
    hh: u32,
    kind: u32,
    slots: Option<u32>,
) -> u32 {
    use Uv::{Image, Zero};
    let mut f = Frame {
        x,
        y,
        w,
        h: hh,
        right: 0.0,
        bottom: 0.0,
    };
    if kind != 0 && kind != 4 && kind != 5 {
        let strip = store(remap(h, REMAP_Y2, konst(h, 0x00b4_1d80)));
        let below = store(add(load(y), load(hh)));
        let uv = [
            0,
            konst(h, 0x00b6_5818),
            konst(h, 0x00b6_5800),
            konst(h, 0x00b6_581c),
        ];
        h.call_sym(
            SHAPE_UV,
            Conv::Cdecl,
            &[
                200,
                pass(x),
                below,
                pass(w),
                strip,
                uv[0],
                uv[1],
                uv[2],
                uv[3],
            ],
        );
    }
    let centre = |uv: [u32; 4]| Cell {
        col: 1,
        row: 1,
        id: 200,
        uv: uv.map(Image),
    };
    let middle = match kind {
        1 => Some(centre([0x00b6_5838, 0x00b6_5610, 0x00b5_f0d4, 0x00b6_5818])),
        2 => Some(centre([0x00b6_6bd4, 0x00b6_5610, 0x00b6_5834, 0x00b6_561c])),
        4 => Some(centre([0x00b6_6bdc, 0x00b6_5610, 0x00b6_6be0, 0x00b6_561c])),
        5 => Some(Cell {
            col: 1,
            row: 1,
            id: 0xffff_ffff,
            uv: [
                Image(0x00b6_6e30),
                Zero,
                Image(0x00b6_6e34),
                Image(0x00b3_eb9c),
            ],
        }),
        _ => None,
    };
    if let Some(cell) = middle {
        frame_cell(h, f, cell);
    } else if kind == 3 {
        let uv = [0x00b6_6be4, 0x00b6_5610, 0x00b6_5628, 0x00b6_561c].map(|c| konst(h, c));
        h.call_sym(
            SHAPE_UV,
            Conv::Cdecl,
            &[
                200,
                pass(x),
                pass(y),
                pass(w),
                pass(hh),
                uv[0],
                uv[1],
                uv[2],
                uv[3],
            ],
        );
    }
    let piece = |col: u8, row: u8, uv: [u32; 4]| Cell {
        col,
        row,
        id: 200,
        uv: uv.map(Image),
    };
    frame_cell(
        h,
        f,
        piece(0, 0, [0x00b4_936c, 0x00b6_580c, 0x00b4_1ca0, 0x00b6_5810]),
    );
    frame_cell(
        h,
        f,
        piece(1, 0, [0x00b4_1ca0, 0x00b6_580c, 0x00b6_5828, 0x00b6_5810]),
    );
    f.right = add(load(x), load(w));
    frame_cell(
        h,
        f,
        piece(2, 0, [0x00b6_5828, 0x00b6_580c, 0x00b6_6bd0, 0x00b6_5810]),
    );
    frame_cell(
        h,
        f,
        piece(0, 1, [0x00b4_936c, 0x00b6_5810, 0x00b4_1ca0, 0x00b6_5814]),
    );
    f.bottom = add(load(y), load(hh));
    let (_, y5, _) = frame_cell(
        h,
        f,
        piece(0, 2, [0x00b4_936c, 0x00b6_5814, 0x00b4_1ca0, 0x00b6_561c]),
    );
    let (_, y6, _) = frame_cell(
        h,
        f,
        piece(1, 2, [0x00b4_1ca0, 0x00b6_5814, 0x00b6_5828, 0x00b6_561c]),
    );
    frame_cell(
        h,
        f,
        piece(2, 1, [0x00b6_5828, 0x00b6_5810, 0x00b6_6bd0, 0x00b6_5814]),
    );
    let (eax, _, x8) = frame_cell(
        h,
        f,
        piece(2, 2, [0x00b6_5828, 0x00b6_5814, 0x00b6_6bd0, 0x00b6_561c]),
    );
    if let Some(s) = slots {
        h.write(s, x8);
        h.write(s + 8, y6);
        h.write(s + 0x10, y5);
    }
    eax
}
