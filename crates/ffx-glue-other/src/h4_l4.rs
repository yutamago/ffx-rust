//! Continuation round 10 of list `h4-glue-other`, batch 3: a menu list
//! item's shape sprite.

use ffx_bridge::x87::{add, div, load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::CI_COS;

// ---- menu_list_drawShape_00774cb0

/// `TOGetShapTextureName(id)` (cdecl) -> the texture's name.
pub const SHAPE_TEXTURE_NAME: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl): writes two floats.
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `graphicGetTime()` (cdecl) -> the time in `ST0`.
pub const GET_TIME: u32 = 0x0064_1410;
/// `graphicDrawUIElement(element, texture, count, a, b)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// The shape table's group starts (`0x0112a0a4`, a pointer to i32s), its
/// sprites (`0x0112a020`, a pointer to 12-byte records) and its texture
/// table (`0x0112a0a8`, 16-byte records: `+0` and `+8`).
pub const SHAPE_GROUPS: u32 = 0x0112_a0a4;
/// See [`SHAPE_GROUPS`].
pub const SHAPE_SPRITES: u32 = 0x0112_a020;
/// See [`SHAPE_GROUPS`].
pub const SHAPE_TEXTURES: u32 = 0x0112_a0a8;
/// The menu's texture mode (`0x00c416c0`): 5, 8 or 0xb.
pub const TEXTURE_MODE: u32 = 0x00c4_16c0;
/// The v nudge (`0x00b47c68`, f64), the time scale (`0x00b922a8`) and the
/// wobble size (`0x00b92270`).
pub const V_NUDGE: u32 = 0x00b4_7c68;
/// See [`V_NUDGE`].
pub const TIME_SCALE: u32 = 0x00b9_22a8;
/// See [`V_NUDGE`].
pub const WOBBLE: u32 = 0x00b9_2270;
/// Bytes of [`draw_shape`]'s frame, from the original's `ebp - 0xb8`: the
/// texture's width (`+0`) and height (`+0x18`) `TOGetImageWH` writes, then
/// at `+0x1c` the element `graphicDrawUIElement` gets: (x, y, u, v) `+0`,
/// colour bytes as words `+0x10`, (x, y, u, v) `+0x20`, colour `+0x30`,
/// and words 0 at `+0x88`, `+0x90` and `+0x94`; the words between are
/// never written. The other words below the element are the original's
/// temporaries.
pub const SHAPE_ICON_FRAME: u32 = 0xb4;

/// `f64(sext(n)) / w` as `fild`, `fdiv dword`, `fstp dword`.
fn ratio(n: u16, w: u32) -> u32 {
    store(div(f64::from(n as i16), load(w)))
}

/// `menu_list_drawShape_00774cb0(unused, item, x, y)` (`0x00774cb0`,
/// cdecl; `frame` see [`SHAPE_ICON_FRAME`]): draws one shape sprite of a
/// menu list item: sprite `+0x16` (i16) of group `+0x18` (i16; none when
/// negative) in the shape table, offset (`+8`, `+0xa`), colour bytes
/// `+0x2c..+0x2f`, at (x, y). The texture and its size come from the
/// texture mode: 5 and 0xb the fixed textures 0x2f00 and 0x2f04, 8 the
/// sprite's texture id (`hi << 16 + lo` from the texture table); other
/// modes leave the name and size as the frame holds them (stack junk in
/// the original; 0 in the port, brief 2026-10-24/27). Mode 0xb nudges v
/// for the 0x13/0x2f00 sprite at (0, 0x1a0). Kind 2 (`+0`) wobbles both
/// x by `cos(time * scale) * size`.
#[allow(clippy::too_many_lines)] // the element's fields one by one
pub fn draw_shape(h: &mut impl Host, frame: u32, item: u32, x: u32, y: u32) -> u32 {
    let group = h.read::<i16>(item.wrapping_add(0x18));
    let start = if group < 0 {
        0
    } else {
        let groups: u32 = h.read(h.rebase(SHAPE_GROUPS));
        h.read::<u32>(groups.wrapping_add(4 * (i32::from(group) as u32)))
    };
    let sprite = (i32::from(h.read::<i16>(item.wrapping_add(0x16))) as u32).wrapping_add(start);
    let dx = i32::from(h.read::<i16>(item.wrapping_add(8)));
    let dy = i32::from(h.read::<i16>(item.wrapping_add(0xa)));
    let sprites: u32 = h.read(h.rebase(SHAPE_SPRITES));
    let record = sprites.wrapping_add(sprite.wrapping_mul(12));
    let texture = i32::from(h.read::<i16>(record.wrapping_add(8))) as u32;
    let entry = h
        .rebase(SHAPE_TEXTURES)
        .wrapping_add(texture.wrapping_mul(16));
    let hi_id: u32 = h.read(entry);
    let lo_id: u32 = h.read(entry.wrapping_add(8));
    let mode: u32 = h.read(h.rebase(TEXTURE_MODE));
    // junk read: [ebp-0xb0] (name), [ebp-0xb8] (w), [ebp-0xa0] (h) never
    // written for other modes; 0 (brief 2026-10-24)
    let (w_at, h_at) = (frame, frame + 0x18);
    let (name, w, hh) = {
        let id = match mode {
            5 => Some(0x2f00),
            8 => Some((hi_id << 16).wrapping_add(lo_id)),
            0xb => Some(0x2f04),
            _ => None,
        };
        if let Some(id) = id {
            let name = h.call_sym(SHAPE_TEXTURE_NAME, Conv::Cdecl, &[id]);
            h.call_sym(IMAGE_WH, Conv::Cdecl, &[id, w_at, h_at]);
            (name, h.read::<u32>(w_at), h.read::<u32>(h_at))
        } else {
            (0, 0, 0)
        }
    };
    let element = frame + 0x1c;
    h.write(element.wrapping_add(0x88), 0u32);
    h.write(element.wrapping_add(0x90), 0u32);
    h.write(element.wrapping_add(0x94), 0u32);
    h.write(element, store(f64::from(x as i32)));
    h.write(element.wrapping_add(4), store(f64::from(y as i32)));
    let d: u16 = h.read(record);
    let e4: u16 = h.read(record.wrapping_add(4));
    let e2: u16 = h.read(record.wrapping_add(2));
    let e6: u16 = h.read(record.wrapping_add(6));
    h.write(element.wrapping_add(8), ratio(d, w));
    let mut v0 = ratio(e4, hh);
    if lo_id == 0x2f00 && hi_id == 0x13 && mode == 0xb && d == 0 && e4 == 0x1a0 {
        let nudge = f64::from_bits(h.read(h.rebase(V_NUDGE)));
        v0 = store(add(load(v0), nudge));
    }
    h.write(element.wrapping_add(0xc), v0);
    let colour = [0u32, 1, 2, 3].map(|k| h.read::<u8>(item.wrapping_add(0x2c + k)));
    let x1 = (x as i32).wrapping_add(dx);
    let y1 = (y as i32).wrapping_add(dy);
    for (k, &c) in colour.iter().enumerate() {
        h.write(element.wrapping_add(0x10 + 4 * k as u32), u32::from(c));
    }
    h.write(element.wrapping_add(0x20), store(f64::from(x1)));
    h.write(element.wrapping_add(0x24), store(f64::from(y1)));
    h.write(element.wrapping_add(0x28), ratio(e2, w));
    h.write(element.wrapping_add(0x2c), ratio(e6, hh));
    for (k, &c) in colour.iter().enumerate() {
        h.write(element.wrapping_add(0x30 + 4 * k as u32), u32::from(c));
    }
    if h.read::<u8>(item) == 2 {
        let cos = h.rebase(CI_COS);
        for at in [element, element.wrapping_add(0x20)] {
            let time = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
            let scale = f64::from_bits(h.read(h.rebase(TIME_SCALE)));
            let c = f64::from(h.call_intrinsic(cos, mul(time, scale)));
            let size = f64::from_bits(h.read(h.rebase(WOBBLE)));
            let v = add(mul(c, size), load(h.read(at)));
            h.write(at, store(v));
        }
    }
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[element, name, 1, 0, 0])
}
