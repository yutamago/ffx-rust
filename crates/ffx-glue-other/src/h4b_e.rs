//! List `h4-glue-other`, second agent (h4b), batch 5: the sphere grid's
//! full-screen quad.

use ffx_bridge::x87::{div, load, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::int_f32;

/// `graphicAbmapGetVertexInfo(name, n)` (cdecl).
pub const ABMAP_VERTEX_INFO: u32 = 0x0063_8fd0;
/// `graphicFontGetScreenWH(&w, &h)` (cdecl).
pub const FONT_SCREEN_WH: u32 = 0x0064_0db0;
/// `g_GraphicSkipFrame`.
pub const SKIP_FRAME: u32 = 0x012f_b7d0;
/// `"NoTexture"` (9 characters, copied without the terminator).
pub const NO_TEXTURE: u32 = 0x00b4_1b50;

/// `AbmapDrawScreenQuad_00a4b260()` (cdecl): unless the frame is skipped,
/// one untextured quad over the whole screen into the sphere grid's
/// vertex block for `"NoTexture"` (`graphicAbmapGetVertexInfo(name, 0)`,
/// its `+0x94`): the name copied to `+0x20`, `+0` cleared, four black
/// opaque colours (`+0x14`), the corners (`+0xc`) at `±w / 0x00b921f0`,
/// `±h / 0x00b921f0` (the font screen's size as `f32`, divided as `f64`,
/// stored as `f32`; w first, z 1), UVs 0 with a 1 (`+0x10`), indices 0,
/// 1, 3, 3, 1, 2 (`+0x1c`), four more vertices (`+4`) and six indices
/// (`+8`). `frame` is two words for the screen size.
pub fn screen_quad(h: &mut impl Host, frame: u32) {
    if h.read::<u32>(h.rebase(SKIP_FRAME)) != 0 {
        return;
    }
    let name = h.rebase(NO_TEXTURE);
    let info = h.call_sym(ABMAP_VERTEX_INFO, Conv::Cdecl, &[name, 0]);
    let v: u32 = h.read(info + 0x94);
    let dst: u32 = h.read(v + 0x20);
    let text: [u32; 2] = [h.read(name), h.read(name + 4)];
    h.write(dst, text[0]);
    h.write(dst + 4, text[1]);
    let last: u8 = h.read(name + 8);
    h.write(dst + 8, last);
    h.write(v, 0u32);
    let one = 1.0f32.to_bits();
    for k in 0..16u32 {
        let colours: u32 = h.read(v + 0x14);
        h.write(colours + 4 * k, if k % 4 == 3 { one } else { 0 });
    }
    h.call_sym(FONT_SCREEN_WH, Conv::Cdecl, &[frame, frame + 4]);
    let k: f64 = h.read(h.rebase(0x00b9_21f0));
    let pos = [
        (true, true),
        (false, true),
        (true, true),
        (false, false),
        (true, false),
        (false, false),
        (true, false),
        (false, true),
    ];
    for (i, (wide, negative)) in pos.iter().enumerate() {
        let n: i32 = h.read(if *wide { frame } else { frame + 4 });
        let x = load(int_f32(n));
        let x = if *negative { -x } else { x };
        let value = store(div(x, k));
        let corners: u32 = h.read(v + 0xc);
        let slot = 3 * (i as u32 / 2) + (i as u32 % 2);
        h.write(corners + 4 * slot, value);
        if i % 2 == 1 {
            let corners: u32 = h.read(v + 0xc);
            h.write(corners + 4 * (slot + 1), one);
        }
    }
    for k in 0..12u32 {
        let uvs: u32 = h.read(v + 0x10);
        h.write(uvs + 4 * k, if k % 3 == 2 { one } else { 0 });
    }
    for (k, i) in [0u16, 1, 3, 3, 1, 2].iter().enumerate() {
        let indices: u32 = h.read(v + 0x1c);
        h.write(indices + 2 * k as u32, *i);
    }
    let n: u32 = h.read(v + 4);
    h.write(v + 4, n.wrapping_add(4));
    let n: u32 = h.read(v + 8);
    h.write(v + 8, n.wrapping_add(6));
}

// ---- the config menu's row frame

/// `graphicUiRemapX2(x)` (cdecl, float in `ST0`).
pub const REMAP_X2: u32 = crate::h4_e1::REMAP_X2;
/// `graphicUiRemapY2(y)` (cdecl, float in `ST0`).
pub const REMAP_Y2: u32 = crate::h4_e1::REMAP_Y2;
/// `TODrawCrossBoxXYWHC2(x, y, w, h, c0, c1)` (cdecl).
pub const CROSS_BOX: u32 = 0x008f_4b20;

/// A box edge of [`row_frame`]: how one of x, y, w, h is made.
#[derive(Clone, Copy)]
enum Part {
    /// An argument or a kept value as it is (`fld`/`fstp`).
    Keep(u32),
    /// `remapX2(constant)`, stored.
    RemapX(u32),
    /// `remapY2(constant)`, stored.
    RemapY(u32),
    /// `remapX2(constant) + base` (`f32` base), stored.
    PlusX(u32, u32),
    /// `remapY2(1) + base`, stored.
    PlusYOne(u32),
    /// `base - remapX2(constant)` (`f64` base), stored.
    MinusX(f64, u32),
    /// `base - remapY2(constant)` (`f64` base), stored.
    MinusY(f64, u32),
}

/// A box of [`row_frame`]'s bevels: h, w, y, x, the colours, and the
/// argument slots each of h, w, y, x is also stored into.
type Bevel = (Part, Part, Part, Part, u32, u32, [Option<u32>; 4]);

/// Evaluates `p` (its remap call, if any) as the original does.
fn part(h: &mut impl Host, p: Part) -> u32 {
    match p {
        Part::Keep(v) => crate::h4_e1::pass(v),
        Part::RemapX(c) => {
            let v = crate::h4_e1::pass(h.read(h.rebase(c)));
            store(h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v]))
        }
        Part::RemapY(c) => {
            let v = crate::h4_e1::pass(h.read(h.rebase(c)));
            store(h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v]))
        }
        Part::PlusX(c, base) => {
            let v = crate::h4_e1::pass(h.read(h.rebase(c)));
            let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v]);
            store(ffx_bridge::x87::add(r, load(base)))
        }
        Part::PlusYOne(base) => {
            let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[1.0f32.to_bits()]);
            store(ffx_bridge::x87::add(r, load(base)))
        }
        Part::MinusX(base, c) => {
            let v = crate::h4_e1::pass(h.read(h.rebase(c)));
            let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[v]);
            store(ffx_bridge::x87::sub(base, r))
        }
        Part::MinusY(base, c) => {
            let v = crate::h4_e1::pass(h.read(h.rebase(c)));
            let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[v]);
            store(ffx_bridge::x87::sub(base, r))
        }
    }
}

/// `MenuConfig_DrawRowFrame_008cb530(x, y)` (cdecl, floats): the config
/// menu row's frame, 13 gradient boxes (`TODrawCrossBoxXYWHC2`) from the
/// row's size (`W = remapX2(0x00b5f430)`, `H = remapY2(0x00b3eb9c)`, both
/// stored) and the edges `x + W`, `y + H` (kept as `f64`): a shadow, two
/// rules and the cap above, the four-colour body, the light bevel and the
/// dark bevel below. Each edge asks `graphicUiRemapY2`/`X2` in the
/// original's order (height, width, y, x). The original computes the last
/// boxes' edges in its argument slots (`slots`): x ends holding the last
/// box's x, y the second-to-last's y. Returns the last box's `eax`.
#[allow(clippy::too_many_lines)]
pub fn row_frame(h: &mut impl Host, x: u32, y: u32, slots: Option<u32>) -> u32 {
    use Part::{Keep, MinusX, MinusY, PlusX, PlusYOne, RemapX, RemapY};
    let w0 = part(h, RemapX(0x00b5_f430));
    let h0 = part(h, RemapY(0x00b3_eb9c));
    let y64 = ffx_bridge::x87::add(load(h0), load(y));
    let yf = store(y64);
    let wf = load(w0);
    let x64 = ffx_bridge::x87::add(load(w0), load(x));
    let boxes: [(Part, Part, Part, Part, u32, u32); 7] = [
        (
            RemapY(0x00b9_2308),
            RemapX(0x00b4_bf08),
            Keep(yf),
            Keep(x),
            0,
            0x2600_0000,
        ),
        (
            RemapY(0x00b9_2308),
            MinusX(wf, 0x00b6_5c0c),
            Keep(yf),
            PlusX(0x00b4_bf08, x),
            0x2600_0000,
            0x2600_0000,
        ),
        (
            RemapY(0x00b9_2308),
            RemapX(0x00b4_bf08),
            Keep(yf),
            MinusX(x64, 0x00b4_bf08),
            0x2600_0000,
            0,
        ),
        (
            Keep(h0),
            RemapX(0x00b4_bf08),
            Keep(y),
            Keep(x),
            0x0070_212a,
            0x4070_212a,
        ),
        (
            Keep(h0),
            RemapX(0x00b5_ef78),
            Keep(y),
            PlusX(0x00b4_bf08, x),
            0x4070_212a,
            0x4070_212a,
        ),
        (
            Keep(h0),
            RemapX(0x00b5_ef78),
            Keep(y),
            PlusX(0x00b6_5c14, x),
            0x4070_212a,
            0x4003_0303,
        ),
        (
            Keep(h0),
            RemapX(0x00b4_bf08),
            Keep(y),
            PlusX(0x00b6_5c18, x),
            0x4003_0303,
            0x0003_0303,
        ),
    ];
    let mut eax = 0;
    for (ph, pw, py, px, c0, c1) in boxes {
        let hh = part(h, ph);
        let w = part(h, pw);
        let yy = part(h, py);
        let xx = part(h, px);
        eax = h.call_sym(CROSS_BOX, Conv::Cdecl, &[xx, yy, w, hh, c0, c1]);
    }
    let slot = |k: u32| slots.map(|s| s + 4 * k);
    // the bevels; the slot writes as the original makes them
    let lower: [Bevel; 6] = [
        (
            RemapY(0x00b9_2268),
            RemapX(0x00b4_bf08),
            PlusYOne(y),
            Keep(x),
            0x00ff_ffff,
            0x5aff_ffff,
            [None; 4],
        ),
        (
            RemapY(0x00b9_2268),
            MinusX(wf, 0x00b6_5c0c),
            PlusYOne(y),
            PlusX(0x00b4_bf08, x),
            0x5aff_ffff,
            0x5aff_ffff,
            [None; 4],
        ),
        (
            RemapY(0x00b9_2268),
            RemapX(0x00b4_bf08),
            PlusYOne(y),
            MinusX(x64, 0x00b4_bf08),
            0x5aff_ffff,
            0x00ff_ffff,
            [None, None, slot(1), slot(1)],
        ),
        (
            RemapY(0x00b3_eb94),
            RemapX(0x00b4_bf08),
            MinusY(y64, 0x00b3_eb94),
            Keep(x),
            0,
            0x8000_0000,
            [None, None, slot(1), None],
        ),
        (
            RemapY(0x00b3_eb94),
            MinusX(wf, 0x00b6_5c0c),
            MinusY(y64, 0x00b3_eb94),
            PlusX(0x00b4_bf08, x),
            0x8000_0000,
            0x8000_0000,
            [None, slot(1), slot(1), slot(0)],
        ),
        (
            RemapY(0x00b3_eb94),
            RemapX(0x00b4_bf08),
            MinusY(y64, 0x00b3_eb94),
            MinusX(x64, 0x00b4_bf08),
            0x8000_0000,
            0,
            [None, None, slot(0), slot(0)],
        ),
    ];
    for (ph, pw, py, px, c0, c1, to) in lower {
        let hh = part(h, ph);
        let w = part(h, pw);
        if let Some(s) = to[1] {
            h.write(s, w);
        }
        let yy = part(h, py);
        if let Some(s) = to[2] {
            h.write(s, yy);
        }
        let xx = part(h, px);
        if let Some(s) = to[3] {
            h.write(s, xx);
        }
        eax = h.call_sym(CROSS_BOX, Conv::Cdecl, &[xx, yy, w, hh, c0, c1]);
    }
    eax
}
