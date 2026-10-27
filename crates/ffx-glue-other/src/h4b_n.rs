//! List `h4-glue-other`, second agent (h4b), round 4 batch 1: the window
//! layout shape drawer `TOAdpEasyShapeXYWHC2`.
//!
//! Floats follow the x87 at 53 bits ([`crate::h4_e1`]): `f64` in the
//! original's order, `f32` at each `fstp dword`; where the original
//! stores a value and goes on from the unrounded register, so does the
//! port. The frame is the original's: every store it makes there, the
//! integer and `f64` temporaries included, is made at the same `ebp`
//! offset, so callees (`BeginScissor_XYUV`, `graphicDrawUIElement`) see
//! what they see in the game.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{int_f32, pass};

/// `TOGetShapTextureName(id)` (cdecl).
pub const SHAPE_TEXTURE_NAME: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl).
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `TOCopyWindowLayout_008fa7d0(i)` (cdecl): images 0x3e00.
pub const COPY_LAYOUT_7D0: u32 = 0x008f_a7d0;
/// `TOCopyWindowLayout_008fa450(i)` (cdecl): image 0x3f00.
pub const COPY_LAYOUT_450: u32 = 0x008f_a450;
/// `TOCopyWindowLayout_008fa790(i)` (cdecl): image 0x3dc0.
pub const COPY_LAYOUT_790: u32 = 0x008f_a790;
/// `BeginScissor_XYUV(&x0, &y0, &x1, &y1, &u0, &v0, &u1, &v1)` (cdecl).
pub const BEGIN_SCISSOR: u32 = 0x008e_5a20;
/// `ShapeDataModify(record, id)` (cdecl).
pub const SHAPE_DATA_MODIFY: u32 = 0x008e_8a10;
/// `graphicDrawUIElement(record, name, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;

/// `g_TOScaleX` (`f32`): the shape and font width scale.
pub const SCALE_X: Global<u32> = Global::new(0x00c5_d778);
/// `g_TOScaleY` (`f32`): the height scale.
pub const SCALE_Y: Global<u32> = Global::new(0x00c5_d77c);
/// `g_TOEasyShapeAlt`: the alternative placement (0.8 scales).
pub const SHAPE_ALT: Global<u32> = Global::new(0x0187_16e8);
/// `g_TOEasyShapeAltNudge`: with [`SHAPE_ALT`], the extra pixel nudges.
pub const SHAPE_ALT_NUDGE: Global<u32> = Global::new(0x0187_16ec);

/// The `f64` constants (image addresses).
const K8: u32 = 0x00b9_22a8;
const K07: u32 = 0x00b6_6c70;
const K08: u32 = 0x00b6_6c78;
const K2: u32 = 0x00b9_2270;
const K06: u32 = 0x00b9_2228;
const K09: u32 = 0x00b4_9958;
const K10: u32 = 0x00b9_22b0;
const K0125: u32 = 0x00b9_2200;
const K056: u32 = 0x00b6_6c60;
const K3: u32 = 0x00b2_c5c8;
const K4: u32 = 0x00b4_3d80;
const K12: u32 = 0x00b4_8340;
const K14444: u32 = 0x00b6_6c80;
const K2M9: u32 = 0x00b4_1cc0;
const K2M10: u32 = 0x00b5_8b50;
/// The `f32` constants.
const K10F: u32 = 0x00b9_2318;
const K8F: u32 = 0x00b9_2314;
const K001F: u32 = 0x00b1_f2b0;

/// Words of [`easy_shape_xywhc2`]'s frame: `ebp - 0xe8` .. `ebp - 4`.
pub const EASY_FRAME_WORDS: usize = 0x39;

/// The `f64` constant at image address `sym`.
fn k64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

/// The `f32` constant at image address `sym`, as `fld dword` loads it.
fn k32(h: &impl Host, sym: u32) -> f64 {
    load(h.read(h.rebase(sym)))
}

/// The frame at the original's `ebp`.
struct Frame(u32);

impl Frame {
    fn get(&self, h: &impl Host, off: u32) -> u32 {
        h.read(self.0 - off)
    }
    /// `fld dword [ebp - off]`.
    fn f(&self, h: &impl Host, off: u32) -> f64 {
        load(self.get(h, off))
    }
    fn set(&self, h: &mut impl Host, off: u32, v: u32) {
        h.write(self.0 - off, v);
    }
    /// `fstp qword [ebp - off]`.
    fn set64(&self, h: &mut impl Host, off: u32, v: f64) {
        h.write(self.0 - off, v);
    }
    /// `fstp dword [ebp - off]` of `v`.
    fn st(&self, h: &mut impl Host, off: u32, v: f64) {
        self.set(h, off, store(v));
    }
    /// An integer through `[ebp - 0xa0]` and `fild`/`fstp` back there
    /// twice: its `f32`, which it returns as loaded.
    fn int_twice(&self, h: &mut impl Host, n: i32) -> f64 {
        self.set(h, A0, n.cast_unsigned());
        let nf = int_f32(n);
        self.set(h, A0, nf);
        load(nf)
    }
}

// The frame's words (`ebp` offsets).
/// The entry counter.
const I: u32 = 0xe8;
/// The image's width and height (`TOGetImageWH`).
const IMG_W: u32 = 0xe4;
const IMG_H: u32 = 0xe0;
/// The texture coordinates `BeginScissor_XYUV` clips: v0, u0, u1, v1.
const V0: u32 = 0xdc;
const U0: u32 = 0xd0;
const U1: u32 = 0xcc;
const V1: u32 = 0xc8;
/// The texture name and the layout.
const NAME: u32 = 0xd8;
const LAYOUT: u32 = 0xd4;
/// The `f64` temporary at `ebp - 0xc4`.
const C4: u32 = 0xc4;
/// The scaled width and height.
const SX: u32 = 0xbc;
const SY: u32 = 0xac;
/// The edges: x0, y0, x1, y1.
const X0: u32 = 0xb4;
const Y0: u32 = 0xb8;
const X1: u32 = 0xb0;
const Y1: u32 = 0xa8;
/// The `f64` temporary at `ebp - 0xa4` and the integer one at `ebp - 0xa0`.
const A4: u32 = 0xa4;
const A0: u32 = 0xa0;
/// The quad record (`graphicDrawUIElement`), 0x98 bytes.
const REC: u32 = 0x9c;

/// `TOAdpEasyShapeXYWHC2(ret, id, x, y, w, h, c0, c1)` (cdecl): draws
/// window layout shape `id` at `x`, `y`; returns `ret` (the original's
/// last `mov eax, [ebp+8]`).
///
/// The id picks the image and the layout (`TOCopyWindowLayout_*`, a count
/// then 12-byte entries: u, v `u8`, dx, dy `i8`, w, h `u16`, x and y
/// anchors `u16`): ids 0..=198 and 200..=398 image 0x3e00 and layout
/// `id` / `id - 200` of `TOCopyWindowLayout_008fa7d0`, 400..=598 image
/// 0x3f00 (`_008fa450`), 600..=649 image 0x3dc0 (`_008fa790`). Each entry
/// (for id 0x3e only the last) is drawn as one quad: its width and height
/// are `w`, `h` (or the entry's when 0) times `g_TOScaleX`/`Y`, scaled
/// again for ids 600..=611 (the scale kept for the next entry), its
/// corner `x + dx`, `y + dy` (anchor word 1 puts it on the other edge),
/// with per-id nudges; `BeginScissor_XYUV` clips it, and when anything is
/// left the record goes to `graphicDrawUIElement(record, name, 1, 0, 0)`
/// with the UVs over the image's size and colours `c0` (top) and `c1`
/// (bottom) byte by byte.
///
/// `frame` is the original's `ebp - 0xe8` ([`EASY_FRAME_WORDS`] words).
///
/// Junk reads (brief 2026-10-24, 2026-10-27): an id in none of the ranges
/// leaves the name and the layout a never-written stack word
/// (`[ebp-0xa0]`) and the image size never written (`[ebp-0xe4]`,
/// `[ebp-0xe0]`); the port uses 0, so it reads its count at address 0
/// (the original reads through a stale word). The record's words
/// `+0x40..+0x84` and `+0x8c` are never written; the shim's frame holds
/// 0 there.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn easy_shape_xywhc2(
    h: &mut impl Host,
    frame: u32,
    ret: u32,
    id: u32,
    x: i32,
    y: i32,
    w: i32,
    ht: i32,
    c0: u32,
    c1: u32,
) -> u32 {
    let f = Frame(frame + 0xe8);
    f.set(h, 0xc, 0);
    f.set(h, 8, 0);
    let mut layout = 0;
    if id <= 0xc6 {
        layout = pick(h, &f, 0x3e00, COPY_LAYOUT_7D0, id);
    } else {
        // junk read: [ebp-0xa0] never written; 0 (brief 2026-10-24)
        f.set(h, NAME, 0);
    }
    f.set(h, LAYOUT, layout);
    if id.wrapping_sub(200) <= 0xc6 {
        layout = pick(h, &f, 0x3e00, COPY_LAYOUT_7D0, id - 200);
        f.set(h, LAYOUT, layout);
    }
    if id.wrapping_sub(400) <= 0xc6 {
        layout = pick(h, &f, 0x3f00, COPY_LAYOUT_450, id - 400);
        f.set(h, LAYOUT, layout);
    }
    let rel = id.wrapping_sub(600);
    if rel <= 0x31 {
        layout = pick(h, &f, 0x3dc0, COPY_LAYOUT_790, rel);
        f.set(h, LAYOUT, layout);
    }
    let small = rel <= 0xb;
    f.set(h, I, 0);
    let wf = int_f32(w);
    f.set(h, A0, wf);
    f.st(h, SX, mul(load(wf), load(SCALE_X.get(h))));
    let hf = int_f32(ht);
    f.set(h, A0, hf);
    f.st(h, SY, mul(load(hf), load(SCALE_Y.get(h))));
    let mut i = 0u32;
    loop {
        let count: u32 = h.read(layout);
        if i >= count {
            break;
        }
        if id != 0x3e || i >= count.wrapping_sub(1) {
            entry(
                h,
                &f,
                layout + 4 + 12 * i,
                id,
                small,
                [x, y, w, ht],
                [c0, c1],
            );
        }
        i += 1;
        f.set(h, I, i);
    }
    ret
}

/// The name, the image size and the layout for one id range; returns
/// the layout.
fn pick(h: &mut impl Host, f: &Frame, image: u32, copy: u32, i: u32) -> u32 {
    let name = h.call_sym(SHAPE_TEXTURE_NAME, Conv::Cdecl, &[image]);
    f.set(h, NAME, name);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[image, f.0 - IMG_W, f.0 - IMG_H]);
    h.call_sym(copy, Conv::Cdecl, &[i])
}

/// One layout entry at `e`: the scales, the edges, the UVs, the clip and
/// the draw.
fn entry(
    h: &mut impl Host,
    f: &Frame,
    e: u32,
    id: u32,
    small: bool,
    [x, y, w, ht]: [i32; 4],
    colours: [u32; 2],
) {
    if w == 0 {
        let n = u32::from(h.read::<u16>(e + 4));
        f.set(h, A0, n);
        f.set64(h, A4, f64::from(n));
        f.st(h, SX, mul(f64::from(n), load(SCALE_X.get(h))));
    }
    if ht == 0 {
        let n = u32::from(h.read::<u16>(e + 6));
        f.set(h, A0, n);
        f.set64(h, A4, f64::from(n));
        f.st(h, SY, mul(f64::from(n), load(SCALE_Y.get(h))));
    }
    let alt = SHAPE_ALT.get(h) != 0;
    if small {
        let (sx, sy) = (f.f(h, SX), f.f(h, SY));
        if alt {
            f.st(h, SX, mul(sx, k64(h, K06)));
            f.st(h, SY, mul(sy, k64(h, K09)));
        } else {
            let k10 = k64(h, K10);
            f.st(h, SX, mul(div(mul(sx, k64(h, K8)), k10), k64(h, K07)));
            f.st(h, SY, mul(sy, k64(h, K07)));
        }
    }
    if id.wrapping_sub(0x1e0) <= 9 {
        let sy = f.f(h, SY);
        f.st(h, SY, mul(mul(sy, k64(h, K10)), k64(h, K0125)));
    }
    let sx = f.f(h, SX);
    edges_x(h, f, e, small, alt, x, sx);
    let sy = f.f(h, SY);
    edges_y(h, f, e, id, small, alt, y, sy);
    if small {
        let k3 = k64(h, K3);
        f.st(h, X0, add(f.f(h, X0), k3));
        f.st(h, X1, add(k3, f.f(h, X1)));
        let k12 = k64(h, K12);
        f.st(h, Y0, add(f.f(h, Y0), k12));
        f.st(h, Y1, add(k12, f.f(h, Y1)));
    }
    uvs(h, f, e, w, ht);
    if id.wrapping_sub(0x1df) <= 0xa {
        if k32(h, K10F) < sy {
            if h.read::<i8>(e + 2) <= 4 {
                f.st(h, X0, add(f.f(h, X0), 1.0));
                f.st(h, X1, add(f.f(h, X1), 1.0));
            } else {
                let k2 = k64(h, K2);
                f.st(h, X0, sub(f.f(h, X0), k2));
                f.st(h, X1, sub(f.f(h, X1), k2));
            }
        }
        for off in [U0, U1, V0, V1] {
            f.st(h, off, sub(f.f(h, off), 1.0));
        }
    }
    if id.wrapping_sub(0x1ea) <= 9 {
        let mut sy = sy;
        // `fucomp`: equal, not unordered.
        #[allow(clippy::float_cmp)]
        let eight = f.f(h, V1) == k32(h, K8F);
        if eight {
            f.st(h, V1, sub(f.f(h, V1), 1.0));
            f.st(h, SY, sub(sy, 1.0));
            sy = f.f(h, SY);
        }
        let dy = i32::from(h.read::<i8>(e + 3));
        f.set(h, A0, dy.cast_unsigned());
        f.st(h, U0, sub(f.f(h, U0), 1.0));
        f.st(h, U1, sub(f.f(h, U1), 1.0));
        let k = k64(h, K14444);
        f.st(h, SY, mul(sy, k));
        f.set64(h, C4, f64::from(dy));
        let t = mul(k, f64::from(dy));
        let yf = int_f32(y);
        f.set(h, A0, yf);
        f.st(h, Y0, add(t, load(yf)));
        f.st(h, Y1, add(f.f(h, Y0), f.f(h, SY)));
    }
    let b = f.0;
    let clipped = h.call_sym(
        BEGIN_SCISSOR,
        Conv::Cdecl,
        &[
            b - X0,
            b - Y0,
            b - X1,
            b - Y1,
            b - U0,
            b - V0,
            b - U1,
            b - V1,
        ],
    );
    if clipped != 0 {
        record(h, f, id, colours);
    }
}

/// The left and right edges (`[ebp-0xb4]`, `[ebp-0xb0]`) from the entry's
/// dx and anchor word (`+8`); `sx` is the scaled width.
fn edges_x(h: &mut impl Host, f: &Frame, e: u32, small: bool, alt: bool, x: i32, sx: f64) {
    let dx = i32::from(h.read::<i8>(e + 2));
    let far = h.read::<u16>(e + 8) == 1;
    let temp = if far { A4 } else { C4 };
    if small {
        f.set(h, A0, dx.cast_unsigned());
        f.set64(h, temp, f64::from(dx));
        let t = if alt {
            let m = mul(f64::from(dx), k64(h, K08));
            f.set64(h, temp, f64::from(x));
            add(m, f64::from(x))
        } else {
            let m = mul(f64::from(dx), k64(h, K056));
            let xf = int_f32(x);
            f.set(h, A0, xf);
            add(m, load(xf))
        };
        if far {
            f.st(h, X0, add(sx, t));
            f.st(h, X1, t);
        } else {
            f.st(h, X0, t);
            f.st(h, X1, add(sx, t));
        }
        return;
    }
    let n = dx.wrapping_add(x);
    match (far, alt) {
        (true, true) => {
            f.set(h, A0, n.cast_unsigned());
            f.set64(h, C4, f64::from(n));
            f.st(h, X0, add(mul(sx, k64(h, K08)), f64::from(n)));
            f.set(h, X1, int_f32(n));
        }
        (false, true) => {
            f.set(h, A0, n.cast_unsigned());
            f.set(h, X0, int_f32(n));
            f.set64(h, C4, f64::from(n));
            f.st(h, X1, add(mul(sx, k64(h, K08)), f64::from(n)));
            if SHAPE_ALT_NUDGE.get(h) != 0 {
                f.st(h, X1, add(f.f(h, X1), k64(h, K2)));
            }
        }
        (true, false) => {
            let nf = f.int_twice(h, n);
            f.st(h, X0, add(sx, nf));
            f.st(h, X1, nf);
        }
        (false, false) => {
            let nf = f.int_twice(h, n);
            f.st(h, X0, nf);
            f.st(h, X1, add(sx, nf));
        }
    }
}

/// The top and bottom edges (`[ebp-0xb8]`, `[ebp-0xa8]`) from the entry's
/// dy and anchor word (`+0xa`); `sy` is the scaled height.
#[allow(clippy::too_many_arguments)]
fn edges_y(h: &mut impl Host, f: &Frame, e: u32, id: u32, small: bool, alt: bool, y: i32, sy: f64) {
    let dy = i32::from(h.read::<i8>(e + 3));
    let far = h.read::<u16>(e + 0xa) == 1;
    let nudge = |h: &mut _| SHAPE_ALT_NUDGE.get(h) != 0;
    let n = dy.wrapping_add(y);
    match (small, alt, far) {
        (true, true, true) => {
            let nf = f.int_twice(h, n);
            f.st(h, Y0, add(nf, sy));
            f.st(h, Y1, nf);
            if nudge(h) {
                f.st(h, Y1, sub(f.f(h, Y1), 1.0));
            }
        }
        (true, true, false) => {
            let nf = f.int_twice(h, n);
            f.st(h, Y0, nf);
            f.st(h, Y1, add(nf, sy));
            if nudge(h) {
                f.st(h, Y1, sub(f.f(h, Y1), 1.0));
            }
        }
        (true, false, far) => {
            f.set(h, A0, dy.cast_unsigned());
            f.set64(h, C4, f64::from(dy));
            let m = mul(f64::from(dy), k64(h, K07));
            let yf = int_f32(y);
            f.set(h, A0, yf);
            let t = add(m, load(yf));
            if far {
                f.st(h, Y0, add(sy, t));
                f.st(h, Y1, t);
            } else {
                f.st(h, Y0, t);
                f.st(h, Y1, add(t, sy));
            }
        }
        (false, true, true) => {
            f.set(h, A0, n.cast_unsigned());
            let m = mul(k64(h, K08), sy);
            f.set64(h, C4, f64::from(n));
            f.st(h, Y0, add(m, f64::from(n)));
            f.set(h, Y1, int_f32(n));
            if nudge(h) {
                f.st(h, Y1, add(f.f(h, Y1), k64(h, K4)));
            }
        }
        (false, true, false) => {
            f.set(h, A0, n.cast_unsigned());
            f.set(h, Y0, int_f32(n));
            let m = mul(k64(h, K08), sy);
            f.set64(h, C4, f64::from(n));
            f.st(h, Y1, add(m, f64::from(n)));
            if nudge(h) {
                // Ids 0x25c..=0x262 are in 600..=611 (`small`), so only
                // the last arm runs; the original has the other two.
                if id == 0x25c || id == 0x25d {
                    f.st(h, Y0, add(f.f(h, Y0), k64(h, K2)));
                    f.st(h, Y1, add(f.f(h, Y1), k64(h, K8)));
                } else if id == 0x261 || id == 0x262 {
                    f.st(h, Y1, add(f.f(h, Y1), k64(h, K8)));
                } else {
                    f.st(h, Y1, add(f.f(h, Y1), k64(h, K3)));
                }
            }
        }
        (false, false, far) => {
            let nf = f.int_twice(h, n);
            if far {
                f.st(h, Y0, add(nf, sy));
                f.st(h, Y1, nf);
            } else {
                f.st(h, Y0, nf);
                f.st(h, Y1, add(nf, sy));
            }
        }
    }
}

/// The texture rectangle in pixels: u0, v0 the entry's u, v; u1, v1 its
/// width and height past them (one less when `w`/`h` is given).
fn uvs(h: &mut impl Host, f: &Frame, e: u32, w: i32, ht: i32) {
    let u = u32::from(h.read::<u8>(e));
    let v = u32::from(h.read::<u8>(e + 1));
    f.set(h, A0, u);
    f.set(h, U0, int_f32(u.cast_signed()));
    f.set(h, A0, v);
    f.set(h, V0, int_f32(v.cast_signed()));
    let mut n = u32::from(h.read::<u16>(e + 4));
    if w != 0 {
        n = n.wrapping_sub(1);
    }
    let n = n.wrapping_add(u);
    f.set(h, A0, n);
    f.set(h, U1, int_f32(n.cast_signed()));
    let mut n = u32::from(h.read::<u16>(e + 6));
    if ht != 0 {
        n = n.wrapping_sub(1);
    }
    let n = n.wrapping_add(v);
    f.set(h, A0, n);
    f.set(h, V1, int_f32(n.cast_signed()));
}

/// The record from the clipped edges and UVs, the per-id UV fixes, and
/// the draw.
fn record(h: &mut impl Host, f: &Frame, id: u32, [c0, c1]: [u32; 2]) {
    let r = REC;
    f.set(h, r, pass(f.get(h, X0)));
    f.set(h, r - 4, pass(f.get(h, Y0)));
    let iw = f.f(h, IMG_W);
    f.st(h, r - 8, div(f.f(h, U0), iw));
    let ih = f.f(h, IMG_H);
    f.st(h, r - 0xc, div(f.f(h, V0), ih));
    for k in 0..4 {
        f.set(h, r - 0x10 - 4 * k, (c0 >> (8 * k)) & 0xff);
        f.set(h, r - 0x30 - 4 * k, (c1 >> (8 * k)) & 0xff);
    }
    f.set(h, 0x14, 0);
    f.set(h, r - 0x20, pass(f.get(h, X1)));
    f.set(h, r - 0x24, pass(f.get(h, Y1)));
    f.st(h, r - 0x28, div(f.f(h, U1), iw));
    f.st(h, r - 0x2c, div(f.f(h, V1), ih));
    let small_uv = k32(h, K001F);
    if f.f(h, r - 8) == 0.0 {
        f.st(h, r - 8, small_uv);
    }
    let u0 = f.f(h, r - 8);
    if f.f(h, r - 0x28) == 0.0 {
        f.st(h, r - 0x28, small_uv);
    }
    if id == 0x3e {
        h.call_sym(SHAPE_DATA_MODIFY, Conv::Cdecl, &[f.0 - r, 0x3e]);
    } else {
        if id == 0x261 || id == 0x262 {
            f.st(h, r - 0x2c, sub(f.f(h, r - 0x2c), k64(h, K2M9)));
        }
        if id == 0x260 {
            f.st(h, r - 8, 0.0);
        } else if id == 0xda || id == 0xd8 {
            f.st(h, r - 8, add(u0, k64(h, K2M10)));
        }
    }
    let name = f.get(h, NAME);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[f.0 - r, name, 1, 0, 0]);
}

// ---- rcPrimDraw2D

/// `graphicGetNowUIType()` (cdecl).
pub const GET_NOW_UI_TYPE: u32 = 0x0064_1350;
/// `graphicIsMiniMapInBlitzBall()` (cdecl).
pub const MINIMAP_IN_BLITZ: u32 = 0x0064_1c70;
/// `graphicIs3DScene()` (cdecl).
pub const IS_3D_SCENE: u32 = 0x0064_1b90;
/// `graphicSetMiniMapType(kind)` (cdecl).
pub const SET_MINIMAP_TYPE: u32 = 0x0064_3720;
/// `memset` (the import thunk).
pub const MEMSET: u32 = 0x0094_964c;
/// `rcPacketDataInit(packet)` (cdecl).
pub const PACKET_INIT: u32 = 0x0092_7a20;
/// `rcPacketDataOpenAdr(packet, buffer)` (cdecl).
pub const PACKET_OPEN_ADR: u32 = 0x0092_7a70;
/// `rcPacketDataCloseAdr(packet)` (cdecl).
pub const PACKET_CLOSE_ADR: u32 = 0x0092_75b0;
/// `rcPacketDataDraw(packet)` (cdecl).
pub const PACKET_DRAW: u32 = 0x0092_7600;
/// `rcVectorToFixed_009273e0(dst, v)` (cdecl).
pub const VECTOR_TO_FIXED: u32 = 0x0092_73e0;
/// `fiosUnifyFilename(src, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;

/// `g_RcPrim2DPacket`: the packet buffer rcPrimDraw2D opens.
pub const PRIM_PACKET: Global<u32> = Global::new(0x00c4_98dc);
/// `g_RcPrim2DOrigin` (`i32`, 0x800): the map coordinates' origin.
pub const PRIM_ORIGIN: Global<i32> = Global::new(0x00c5_e76c);
/// `g_RcPrim2DCenterX` (`i32`, 0x100): the minimap's centre x.
pub const PRIM_CENTER_X: Global<i32> = Global::new(0x00c5_e770);
/// `g_RcPrim2DCenterY` (`i32`, 0xd0): the minimap's centre y.
pub const PRIM_CENTER_Y: Global<i32> = Global::new(0x00c5_e774);
/// `g_RcPrim2DCirclePath`: the circle texture's path.
pub const PRIM_CIRCLE_PATH: Global<u32> = Global::new(0x00c5_e778);
/// `g_RcPrim2DCircleName`: its unified name, 0x100 bytes, made once.
pub const PRIM_CIRCLE_NAME: u32 = 0x0193_bf88;

/// Words of [`prim_draw_2d`]'s frame: `ebp - 0x114` .. `ebp - 4`.
pub const PRIM_FRAME_WORDS: usize = 0x44;

const K100: u32 = 0x00b9_22e0;
const K256: u32 = 0x00b9_22f0;
const K208: u32 = 0x00b5_4c70;
const K075: u32 = 0x00b5_1bd0;
const K512: u32 = 0x00b4_1d38;
const K05: u32 = 0x00b9_2220;

/// The minimap placement constants and offsets, as the original loads
/// them.
struct Place {
    /// The origin, the centre x and y (`f32` through `fild`/`fstp`).
    o: f64,
    cx: f64,
    cy: f64,
    /// 0.75 and 512.
    k: f64,
    k512: f64,
    /// The x offset (`f64`) and the y offset (`f32`).
    x: f64,
    yf: f64,
}

impl Place {
    /// An x on the near side: `((x - o) + cx) * k + off`.
    fn near(&self, x: f64) -> f64 {
        add(mul(add(sub(x, self.o), self.cx), self.k), self.x)
    }
    /// An x on the far side: `(512 - (cx - (x - o)) * k) - off`.
    fn far(&self, x: f64) -> f64 {
        sub(self.far_raw(x), self.x)
    }
    /// `512 - (cx - (x - o)) * k`.
    fn far_raw(&self, x: f64) -> f64 {
        sub(self.k512, mul(sub(self.cx, sub(x, self.o)), self.k))
    }
    /// A y: `((y - o) + cy) +- off`.
    fn y(&self, y: f64, near: bool) -> f64 {
        let t = add(sub(y, self.o), self.cy);
        if near {
            add(t, self.yf)
        } else {
            sub(t, self.yf)
        }
    }
}

/// `ftol(colour[i] * z) >> 8` (`sar`).
fn channel(h: &impl Host, colour: u32, i: u32, v: u32) -> u32 {
    let c = load(h.read(colour + 4 * i));
    let z = load(h.read(v + 8));
    (ftol(h, mul(c, z)).cast_signed() >> 8).cast_unsigned()
}

/// `rcPrimDraw2D(unused, verts, count, colour)` (cdecl): draws a 2D
/// primitive on the minimap and returns `rcPacketDataDraw`'s `eax`.
///
/// `verts` are `count` vectors of four floats (x, y, z, w), `colour` four
/// floats. Each vertex goes into the packet `g_RcPrim2DPacket`
/// (`rcPacketDataOpenAdr`) as r, g, b (`ftol(colour[i] * z) >> 8`), a
/// (`ftol(colour[3])`) and its 12.4 fixed point x, y
/// (`rcVectorToFixed_009273e0`); then by `count` a record in the frame
/// (cleared by `memset`) goes to `graphicDrawUIElement`: 2 a line, 3 a
/// triangle (colours `ftol` of the vertices' integers as `f32`), 4 a
/// quad (vertices 0, 2, 3, 1; with x3 - x0 >= 100 it also sets the
/// minimap type 1..4 by the side), 0x41 the guide circle
/// (`g_RcPrim2DCircleName`, made by `fiosUnifyFilename` once). The map
/// coordinates are placed around the centre at 0.75, mirrored from 512 on
/// the far side, with the offsets 27, 20 (0 in Blitzball).
///
/// `frame` is the original's `ebp - 0x114` ([`PRIM_FRAME_WORDS`] words).
///
/// Junk reads (brief 2026-10-24): with `count` 0 or less the original
/// loads three never-written words (`[ebp-0xbc]`, `[ebp-0xc0]`,
/// `[ebp-0xc4]`) into registers, but nothing uses them then (the switch
/// draws nothing for such a count); the port reads nothing.
pub fn prim_draw_2d(h: &mut impl Host, frame: u32, verts: u32, count: i32, colour: u32) -> u32 {
    let f = Frame(frame + 0x114);
    let ui = h.call_sym(GET_NOW_UI_TYPE, Conv::Cdecl, &[]);
    f.set(h, 0xc8, ui);
    if h.call_sym(MINIMAP_IN_BLITZ, Conv::Cdecl, &[]) == 0 {
        f.set(h, 0xb0, 0x1b);
        f.set(h, 0xb4, 0x14);
    } else {
        f.set(h, 0xb0, 0);
        f.set(h, 0xb4, 0);
    }
    h.call_sym(MEMSET, Conv::Cdecl, &[f.0 - 0x9c, 0, 0x98]);
    h.call_sym(IS_3D_SCENE, Conv::Cdecl, &[]);
    f.set(h, 0xc, 0);
    f.set(h, 8, 0);
    let p = f.0 - 0xd0;
    h.call_sym(PACKET_INIT, Conv::Cdecl, &[p]);
    let buffer = PRIM_PACKET.get(h);
    h.call_sym(PACKET_OPEN_ADR, Conv::Cdecl, &[p, buffer]);
    let mut cur: u32 = f.get(h, 0xcc);
    let mut last = [0u32; 3];
    let mut k = 0;
    while k < count {
        let v = verts.wrapping_add(k.cast_unsigned().wrapping_mul(16));
        f.set(h, 0xac, v);
        let r = channel(h, colour, 0, v);
        f.set(h, 0xa8, r);
        f.set(h, 0xc4, r);
        let g = channel(h, colour, 1, v);
        f.set(h, 0xd4, g);
        f.set(h, 0xc0, g);
        let b = channel(h, colour, 2, v);
        f.set(h, 0xa0, b);
        f.set(h, 0xbc, b);
        let a = ftol(h, load(h.read(colour + 12)));
        h.write(cur, r);
        h.write(cur + 4, g);
        f.set(h, 0xb8, a);
        h.write(cur + 8, b);
        h.write(cur + 0xc, a);
        if count == 3 && k < 3 {
            let at = 0x104 - 0x10 * k.cast_unsigned();
            for (i, src) in [0xc4, 0xc0, 0xbc, 0xb8].into_iter().enumerate() {
                let n: u32 = f.get(h, src);
                f.set(h, at - 4 * i as u32, int_f32(n.cast_signed()));
            }
        }
        h.call_sym(VECTOR_TO_FIXED, Conv::Cdecl, &[f.0 - 0x114, v]);
        let fx: u32 = f.get(h, 0x114);
        h.write(cur + 0x10, fx);
        let fy: u32 = f.get(h, 0x110);
        h.write(cur + 0x14, fy);
        h.write(cur + 0x18, 0u32);
        h.write(cur + 0x1c, 0u32);
        k += 1;
        cur = cur.wrapping_add(0x20);
        last = [f.get(h, 0xa8), f.get(h, 0xd4), f.get(h, 0xa0)];
    }
    let drawn = match count {
        2 => Some(prim_line(h, &f, verts, last, ui)),
        3 => Some(prim_triangle(h, &f, verts, ui)),
        4 => Some(prim_quad(h, &f, verts, ui)),
        0x41 => Some(prim_circle(h, &f, verts, ui)),
        _ => None,
    };
    if let Some([name, a, b, c]) = drawn {
        h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[f.0 - REC, name, a, b, c]);
    }
    f.set(h, 0xcc, cur);
    h.call_sym(PACKET_CLOSE_ADR, Conv::Cdecl, &[p]);
    h.call_sym(PACKET_DRAW, Conv::Cdecl, &[p])
}

/// The origin through the frame (`[ebp-0xa0]`, `[ebp-0xa8]`).
fn origin(h: &mut impl Host, f: &Frame) -> f64 {
    let o = int_f32(PRIM_ORIGIN.get(h));
    f.set(h, A0, o);
    f.set(h, 0xa8, o);
    load(o)
}

/// Centre `g` through the frame (`[ebp-0xa0]`, `[ebp-0xac]`).
fn center(h: &mut impl Host, f: &Frame, g: Global<i32>) -> f64 {
    let c = int_f32(g.get(h));
    f.set(h, A0, c);
    f.set(h, 0xac, c);
    load(c)
}

/// The x offset through the `f64` temporary.
fn x_offset(h: &mut impl Host, f: &Frame) -> f64 {
    let x = f64::from(f.get(h, 0xb0).cast_signed());
    f.set64(h, A4, x);
    x
}

/// The y offset through the frame (`[ebp-0xa0]`, `[ebp-0xa8]`).
fn y_offset(h: &mut impl Host, f: &Frame) -> f64 {
    let y = int_f32(f.get(h, 0xb4).cast_signed());
    f.set(h, A0, y);
    f.set(h, 0xa8, y);
    load(y)
}

/// Vertex `k`'s x and y.
fn vxy(h: &impl Host, verts: u32, k: u32) -> (f64, f64) {
    (
        load(h.read(verts.wrapping_add(16 * k))),
        load(h.read(verts.wrapping_add(16 * k + 4))),
    )
}

/// The placement with its x side decided (`near`: `256 > cx + (x0 - o)`);
/// the y side is decided later. Returns it and the side.
fn place_x(h: &mut impl Host, f: &Frame, verts: u32) -> (Place, bool) {
    let o = origin(h, f);
    let cx = center(h, f, PRIM_CENTER_X);
    let (x0, _) = vxy(h, verts, 0);
    let s = add(cx, sub(x0, o));
    let near = k64(h, K256) > s;
    let x = x_offset(h, f);
    let place = Place {
        o,
        cx,
        cy: 0.0,
        k: k64(h, K075),
        k512: k64(h, K512),
        x,
        yf: 0.0,
    };
    (place, near)
}

/// The y side (`near`: `208 > (y0 - o) + cy`), with the y offset.
fn place_y(h: &mut impl Host, f: &Frame, verts: u32, pl: &mut Place) -> bool {
    pl.cy = center(h, f, PRIM_CENTER_Y);
    let (_, y0) = vxy(h, verts, 0);
    let t = add(sub(y0, pl.o), pl.cy);
    let near = k64(h, K208) > t;
    pl.yf = y_offset(h, f);
    near
}

/// The record's vertex `n` colour (`ebp - 0x8c - 0x20n`, four words).
fn rec_colour(h: &mut impl Host, f: &Frame, n: u32, rgba: [u32; 4]) {
    for (i, c) in rgba.into_iter().enumerate() {
        f.set(h, 0x8c - 0x20 * n - 4 * i as u32, c);
    }
}

/// The vertex colour words the loop left (`[ebp-0xc4]` .. `[ebp-0xb8]`).
fn left_colour(h: &impl Host, f: &Frame) -> [u32; 4] {
    [
        f.get(h, 0xc4),
        f.get(h, 0xc0),
        f.get(h, 0xbc),
        f.get(h, 0xb8),
    ]
}

/// Count 2: a line; its second x and y half a pixel on.
fn prim_line(h: &mut impl Host, f: &Frame, verts: u32, last: [u32; 3], ui: u32) -> [u32; 4] {
    let a: u32 = f.get(h, 0xb8);
    rec_colour(h, f, 0, [last[0], last[1], last[2], a]);
    let (mut pl, near) = place_x(h, f, verts);
    let half = k64(h, K05);
    let (x0, _) = vxy(h, verts, 0);
    let (x1, _) = vxy(h, verts, 1);
    if near {
        f.st(h, 0x9c, pl.near(x0));
        let m = mul(add(sub(x1, pl.o), pl.cx), pl.k);
        f.st(h, 0x7c, add(pl.x, add(m, half)));
    } else {
        f.st(h, 0x9c, pl.far(x0));
        f.st(h, 0x7c, sub(add(pl.far_raw(x1), half), pl.x));
    }
    let near = place_y(h, f, verts, &mut pl);
    let (_, y0) = vxy(h, verts, 0);
    let (_, y1) = vxy(h, verts, 1);
    f.st(h, 0x98, pl.y(y0, near));
    let t = add(add(sub(y1, pl.o), pl.cy), half);
    f.st(h, 0x78, if near { add(t, pl.yf) } else { sub(t, pl.yf) });
    let rgba = left_colour(h, f);
    rec_colour(h, f, 1, rgba);
    [0, 1, 0, ui]
}

/// Count 3: a triangle; the colours from the integers kept as `f32`.
fn prim_triangle(h: &mut impl Host, f: &Frame, verts: u32, ui: u32) -> [u32; 4] {
    let (mut pl, near) = place_x(h, f, verts);
    for k in 0..3 {
        let (x, _) = vxy(h, verts, k);
        f.st(
            h,
            0x9c - 0x20 * k,
            if near { pl.near(x) } else { pl.far(x) },
        );
    }
    let near = place_y(h, f, verts, &mut pl);
    for k in 0..3 {
        let (_, y) = vxy(h, verts, k);
        f.st(h, 0x98 - 0x20 * k, pl.y(y, near));
    }
    for n in 0..3 {
        let mut rgba = [0; 4];
        for (i, c) in rgba.iter_mut().enumerate() {
            let v = f.f(h, 0x104 - 0x10 * n - 4 * i as u32);
            *c = ftol(h, v);
        }
        rec_colour(h, f, n, rgba);
    }
    [0, 1, 1, ui]
}

/// Count 4: a quad, vertices 0, 2, 3, 1; a wide one (`!(x3 - x0 < 100)`)
/// sets the minimap type by its sides and draws with 1 for the UI type.
fn prim_quad(h: &mut impl Host, f: &Frame, verts: u32, ui: u32) -> [u32; 4] {
    let (x0, y0) = vxy(h, verts, 0);
    let (x3, _) = vxy(h, verts, 3);
    // `fcomp` / `jp`: not below 100, unordered included.
    let wide = sub(x3, x0).partial_cmp(&k64(h, K100)) != Some(std::cmp::Ordering::Less);
    let (mut pl, near) = place_x(h, f, verts);
    for (n, k) in [0u32, 2, 3, 1].into_iter().enumerate() {
        let (x, _) = vxy(h, verts, k);
        f.st(
            h,
            0x9c - 0x20 * n as u32,
            if near { pl.near(x) } else { pl.far(x) },
        );
    }
    if wide {
        let cy = int_f32(PRIM_CENTER_Y.get(h));
        f.set(h, A0, cy);
        let t = add(sub(y0, pl.o), load(cy));
        let low = t < k64(h, K208);
        let kind = match (near, low) {
            (true, true) => 1,
            (true, false) => 2,
            (false, true) => 3,
            (false, false) => 4,
        };
        h.call_sym(SET_MINIMAP_TYPE, Conv::Cdecl, &[kind]);
    }
    let rgba = left_colour(h, f);
    rec_colour(h, f, 0, rgba);
    pl.o = origin(h, f);
    let near = place_y(h, f, verts, &mut pl);
    for (n, k) in [0u32, 2, 3, 1].into_iter().enumerate() {
        let (_, y) = vxy(h, verts, k);
        f.st(h, 0x98 - 0x20 * n as u32, pl.y(y, near));
    }
    for n in 1..4 {
        rec_colour(h, f, n, rgba);
    }
    [0, 1, 4, if wide { 1 } else { ui }]
}

/// Count 0x41: the guide circle, its x side by `512 - (cx - (x0 - o)) *
/// k`; full UVs, grey colours with the alpha.
fn prim_circle(h: &mut impl Host, f: &Frame, verts: u32, ui: u32) -> [u32; 4] {
    let cx = center(h, f, PRIM_CENTER_X);
    let o = origin(h, f);
    let mut pl = Place {
        o,
        cx,
        cy: 0.0,
        k: k64(h, K075),
        k512: k64(h, K512),
        x: 0.0,
        yf: 0.0,
    };
    let (x0, _) = vxy(h, verts, 0);
    let (x1, _) = vxy(h, verts, 1);
    let p = pl.far_raw(x0);
    let near = k64(h, K256) > p;
    pl.x = x_offset(h, f);
    if near {
        f.st(h, 0x9c, pl.near(x0));
        f.st(h, 0x7c, pl.near(x1));
    } else {
        f.st(h, 0x9c, sub(p, pl.x));
        f.st(h, 0x7c, pl.far(x1));
    }
    let near = place_y(h, f, verts, &mut pl);
    for k in 0..2 {
        let (_, y) = vxy(h, verts, k);
        f.st(h, 0x98 - 0x20 * k, pl.y(y, near));
    }
    let a: u32 = f.get(h, 0xb8);
    f.set(h, 0x94, 0);
    f.set(h, 0x90, 0);
    f.set(h, 0x74, 1f32.to_bits());
    f.set(h, 0x70, 1f32.to_bits());
    rec_colour(h, f, 0, [0xc0, 0x40, 0xc0, a]);
    rec_colour(h, f, 1, [0xc0, 0x40, 0xc0, a]);
    let name = h.rebase(PRIM_CIRCLE_NAME);
    if h.read::<u8>(name) == 0 {
        let path = PRIM_CIRCLE_PATH.get(h);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, name, 0x100]);
    }
    [name, 1, 0, ui]
}

// ---- 0x00a7bbc0: the menu scene's key interpolation

/// `TOSeqDrawRect_008fdf20(record)` (cdecl).
pub const SEQ_DRAW_RECT: u32 = 0x008f_df20;
/// `TOMakePktSeqPrimLine(record)`: a bare `ret`.
pub const SEQ_PRIM_LINE: u32 = 0x008f_e070;
/// `TOMakePktSeqPrimLayout_008fe0b0(record)` (cdecl).
pub const SEQ_PRIM_LAYOUT: u32 = 0x008f_e0b0;
/// `TOMakePktSeqPrimMes(record)` (cdecl).
pub const SEQ_PRIM_MES: u32 = 0x008f_e080;
/// `TOMakePktSeqPrimFunc(record)` (cdecl).
pub const SEQ_PRIM_FUNC: u32 = 0x008f_e040;
/// `g_TkMenuSceneOffsetX` / `Y` (`i32`): added with key flag 0x8000.
pub const SCENE_OFFSET_X: Global<u32> = Global::new(0x022f_b4fc);
pub const SCENE_OFFSET_Y: Global<u32> = Global::new(0x022f_b500);

/// Words of [`scene_key_lerp`]'s frame: the records, `ebp - 0x7c` ..
/// `ebp - 4`.
pub const KEY_FRAME_WORDS: usize = 0x1e;

/// One key value between `to` (`b`) and `from` (`a`): `b` when they are
/// equal, else `ftol((1 - t) * b + a * t)` over their `f32`s.
fn lerp(h: &impl Host, t: f64, b: i32, a: i32) -> u32 {
    if b == a {
        return b.cast_unsigned();
    }
    let bf = load(int_f32(b));
    let af = load(int_f32(a));
    ftol(h, add(mul(sub(1.0, t), bf), mul(af, t)))
}

/// A colour word between `b` and `a`, byte by byte.
fn lerp_colour(h: &impl Host, t: f64, b: u32, a: u32) -> u32 {
    let mut out = 0;
    for k in (0..4).rev() {
        let byte = |w: u32| ((w >> (8 * k)) & 0xff).cast_signed();
        out = (out << 8) | lerp(h, t, byte(b), byte(a));
    }
    out
}

/// The `i16` at `p + off`, sign-extended.
fn half(h: &impl Host, p: u32, off: u32) -> i32 {
    i32::from(h.read::<i16>(p + off))
}

/// `FUN_00a7bbc0(flags, from, to, t, mode)` (cdecl): a menu scene key
/// between key records `from` and `to` at `t` (`f32` bits), drawn by
/// its kind (`flags & 0x7ff`). Returns the original's `eax`.
///
/// Kinds 0..=3, 8 and 9 do nothing (`eax` 0, the switch's table byte).
/// Otherwise the
/// colours (`+4`, `+8`, byte by byte), x and y (`i16` `+0xc`, `+0xe`;
/// with flag 0x8000 plus `g_TkMenuSceneOffsetX`/`Y`) and the kind's
/// fields (`i16` from `+0x10`) are each `to`'s when equal to `from`'s,
/// else `ftol((1 - t) * to + from * t)` (the colours are `to`'s and
/// `from`'s as they are when the two records are one), and a record in
/// the frame goes to the kind's drawer:
/// - 4: a rectangle (w, h), `TOSeqDrawRect_008fdf20`, its `eax`; with
///   `mode` 1 nothing is drawn and `eax` is the height;
/// - 5: a line to the second point (offset like the first),
///   `TOMakePktSeqPrimLine` (a bare `ret`): `eax` the record's address;
/// - 6: a layout shape (`+0x10`, `+0x12`, the id `+0x14`),
///   `TOMakePktSeqPrimLayout_008fe0b0`;
/// - 7: a message (`+0x10`, `+0x12`, `+0x14`), `TOMakePktSeqPrimMes`;
/// - 10: a function primitive (`+0x10`..`+0x18`, `+0x1c`, `+0x1e`),
///   `TOMakePktSeqPrimFunc`;
/// - any other: nothing, `eax` the kind - 4.
///
/// `frame` is the original's `ebp - 0x7c` ([`KEY_FRAME_WORDS`] words):
/// kind 6's record at `+0`, 7's at `+0x18`, 5's at `+0x30`, 4's at
/// `+0x44`, 10's at `+0x58`. Each record's first word and kind 10's
/// `+0x1a` are never written; the drawers don't read them. The
/// original's integer and float temporaries below `ebp - 0x7c` are dead
/// after it returns and not kept. Past the early kinds the original
/// stores `flags & 0x8000` into its `flags` argument word; the port does
/// at `slots` (the caller's argument words) when given.
#[allow(clippy::too_many_arguments)]
pub fn scene_key_lerp<H: Host>(
    h: &mut H,
    frame: u32,
    flags: u32,
    from: u32,
    to: u32,
    t: u32,
    mode: u32,
    slots: Option<u32>,
) -> u32 {
    let kind = flags & 0x7ff;
    if matches!(kind, 0..=3 | 8 | 9) {
        // `eax` holds the jump table's byte for these kinds: 0.
        return 0;
    }
    let ebp = frame + 0x7c;
    let t = load(t);
    let (c1, c2) = if from == to {
        (h.read::<u32>(from + 4), h.read::<u32>(from + 8))
    } else {
        let c1 = lerp_colour(h, t, h.read(to + 4), h.read(from + 4));
        let c2 = lerp_colour(h, t, h.read(to + 8), h.read(from + 8));
        (c1, c2)
    };
    let field = |h: &H, off| lerp(h, t, half(h, to, off), half(h, from, off));
    let mut x = field(h, 0xc);
    let mut y = field(h, 0xe);
    let shift = flags & 0x8000 != 0;
    if let Some(s) = slots {
        h.write(s, flags & 0x8000);
    }
    if shift {
        x = x.wrapping_add(SCENE_OFFSET_X.get(h));
        y = y.wrapping_add(SCENE_OFFSET_Y.get(h));
    }
    // A record at `ebp - at`: the colours, x, y, then the `i16` fields.
    let record = |h: &mut H, at: u32, fields: &[u32]| {
        let r = ebp - at;
        h.write(r + 4, c1);
        h.write(r + 8, c2);
        h.write(r + 0xc, x as u16);
        h.write(r + 0xe, y as u16);
        for (k, &v) in fields.iter().enumerate() {
            h.write(r + 0x10 + 2 * k as u32, v as u16);
        }
        r
    };
    match kind {
        4 => {
            let w = field(h, 0x10);
            let ht = field(h, 0x12);
            let r = record(h, 0x38, &[w, ht]);
            if mode == 1 {
                return ht;
            }
            h.call_sym(SEQ_DRAW_RECT, Conv::Cdecl, &[r])
        }
        5 => {
            let mut x2 = field(h, 0x10);
            let mut y2 = field(h, 0x12);
            if shift {
                x2 = x2.wrapping_add(SCENE_OFFSET_X.get(h));
                y2 = y2.wrapping_add(SCENE_OFFSET_Y.get(h));
            }
            let r = record(h, 0x4c, &[x2, y2]);
            h.call_sym(SEQ_PRIM_LINE, Conv::Cdecl, &[r]);
            r
        }
        6 => {
            let id = field(h, 0x14);
            let a = field(h, 0x10);
            let b = field(h, 0x12);
            let r = record(h, 0x7c, &[a, b, id]);
            h.call_sym(SEQ_PRIM_LAYOUT, Conv::Cdecl, &[r])
        }
        7 => {
            let a = field(h, 0x10);
            let b = field(h, 0x12);
            let c = field(h, 0x14);
            let r = record(h, 0x64, &[a, b, c]);
            h.call_sym(SEQ_PRIM_MES, Conv::Cdecl, &[r])
        }
        10 => {
            let p14 = field(h, 0x14);
            let p16 = field(h, 0x16);
            let p18 = field(h, 0x18);
            let p10 = field(h, 0x10);
            let p12 = field(h, 0x12);
            let p1c = field(h, 0x1c);
            let p1e = field(h, 0x1e);
            let r = record(h, 0x24, &[p10, p12, p14, p16, p18]);
            h.write(r + 0x1c, p1c as u16);
            h.write(r + 0x1e, p1e as u16);
            h.call_sym(SEQ_PRIM_FUNC, Conv::Cdecl, &[r])
        }
        _ => kind.wrapping_sub(4),
    }
}

// ---- 0x00a51720: the sphere grid's unlit link

/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `FFXVu0DivVectorXYZ(dst, v, s)` (cdecl, `s` an `f32` word).
pub const VU0_DIV_VECTOR_XYZ: u32 = 0x006e_d420;
/// `graphicDrawUIAbmapElement(record, name, kind)` (cdecl).
pub const DRAW_ABMAP_ELEMENT: u32 = 0x0063_e930;
/// `g_Abmap`: the sphere grid data.
pub const ABMAP: Global<u32> = Global::new(0x0230_5834);
/// `g_AbmapLinkPacket`: the links' GS packet (made by `eiAbmStart`).
pub const ABMAP_LINK_PACKET: Global<u32> = Global::new(0x01a8_60f0);
/// `g_GsRegisterSink`: bit 0x100 busy.
pub const GS_SINK: Global<u32> = Global::new(0x0230_56e8);

/// Words of [`abmap_draw_link`]'s frame: `ebp - 0xa8` .. `ebp - 4`.
pub const LINK_FRAME_WORDS: usize = 0x29;

const K16: u32 = 0x00b3_13a0;
const K1_16: u32 = 0x00b9_21e0;
const K2048: u32 = 0x00b5_1bc0;
/// The UV constants (`f32`).
const UV_U0: u32 = 0x00b6_6b68;
const UV_V0: u32 = 0x00b4_9368;
const UV_U1: u32 = 0x00b7_b5d8;
const UV_V1: u32 = 0x00b6_6bac;

/// `[at] = v` for a 32-bit word.
fn put(h: &mut impl Host, at: u32, v: u32) {
    h.write(at, v);
}

/// `[at]` as a 32-bit word.
fn word(h: &impl Host, at: u32) -> u32 {
    h.read(at)
}

/// One point `v` (x, y, 0, 1 at `work + 0x10 k`) through `m` into
/// `work + dst`, then divided by the first point's w. Returns false when
/// the original stops: the first point's w not above 0 (and not NaN).
fn project(h: &mut impl Host, work: u32, k: u32, m: u32, dst: u32, check: bool) -> bool {
    let v = work + 0x10 * k;
    let d = work + dst;
    let abmap = ABMAP.get(h);
    h.call_sym(
        VU0_APPLY_MATRIX,
        Conv::Cdecl,
        &[d, abmap.wrapping_add(m), v],
    );
    if check {
        let w = load(word(h, work + 0x3c));
        if !(0.0 < w || w.is_nan()) {
            return false;
        }
    }
    let s = pass(word(h, work + 0x3c));
    h.call_sym(VU0_DIV_VECTOR_XYZ, Conv::Cdecl, &[d, d, s]);
    true
}

/// The 12.4 fixed point screen words of the three points from `b` on
/// (`work` offsets): the first point at `b`, the second and third 0x10
/// and 0x20 after it plus the first, their doubled x, y scaled down by
/// `s` at 0x30 and 0x40 after it plus the first. `itemp` is the frame
/// word the original passes its integers through.
fn fixed(h: &mut impl Host, f: &Frame, work: u32, b: u32, itemp: u32) {
    let k16 = k64(h, K16);
    let s = f.f(h, 0xa8);
    let at = |o: u32| work + o;
    let x0 = ftol(h, mul(load(word(h, at(0x30))), k16));
    put(h, at(b), x0);
    let y0 = ftol(h, mul(load(word(h, at(0x34))), k16));
    put(h, at(b + 4), y0);
    for (src, near, far) in [(0x40, 0x10, 0x30), (0x50, 0x20, 0x40)] {
        let v = ftol(h, mul(load(word(h, at(src))), k16));
        put(h, at(b + near), v);
        let v = ftol(h, mul(load(word(h, at(src + 4))), k16));
        put(h, at(b + near + 4), v);
        let x = load(word(h, at(src)));
        let xd = store(add(x, x));
        let v = word(h, at(b + near)).wrapping_add(word(h, at(b)));
        put(h, at(b + near), v);
        let v = word(h, at(b + near + 4)).wrapping_add(word(h, at(b + 4)));
        put(h, at(b + near + 4), v);
        f.set(h, 0xa4, xd);
        put(h, at(src), xd);
        let y = load(word(h, at(src + 4)));
        put(h, at(src + 4), store(add(y, y)));
        let v = ftol(h, mul(load(xd), k16));
        put(h, at(b + far), v);
        let v = ftol(h, mul(load(word(h, at(src + 4))), k16));
        put(h, at(b + far + 4), v);
        for (o, base) in [(0, b), (4, b + 4)] {
            let n = int_f32(word(h, at(b + far + o)).cast_signed());
            f.set(h, itemp, n);
            let q = ftol(h, div(load(n), s));
            put(h, at(b + far + o), q.wrapping_add(word(h, at(base))));
        }
    }
}

/// The packet's fixed words (twice, as the original writes them), after
/// waiting for the GS (`g_GsRegisterSink` bit 0x100).
fn packet_head(h: &mut impl Host, work: u32) {
    while GS_SINK.get(h) & 0x100 != 0 {}
    for _ in 0..2 {
        let p = ABMAP_LINK_PACKET.get(h);
        let tag = (word(h, work + 0x1e0) << 15) | 0x3000_4000;
        put(h, p, 0x8005);
        put(h, p + 8, 0x55e);
        let p2 = ABMAP_LINK_PACKET.get(h);
        put(h, p2 + 4, tag);
        put(h, p + 0xc, 0);
        for k in 0..5 {
            let v = word(h, work + 0x140 + 4 * k);
            let q = p + 0x10 + 0x30 * k;
            put(h, q, v);
            put(h, q + 4, (v.cast_signed() >> 31).cast_unsigned());
            put(h, q + 8, 1);
            put(h, q + 0xc, 0);
        }
        for (o, src, hi) in [
            (0x28, 0x1e4, 0x8000),
            (0x38, 0x1e4, 0),
            (0x58, 0x1e4, 0),
            (0x68, 0x1e4, 0),
            (0x88, 0x1e8, 0),
            (0x98, 0x1e8, 0),
            (0xb8, 0x1e8, 0),
            (0xc8, 0x1e8, 0),
            (0xe8, 0x1e4, 0),
            (0xf8, 0x1e4, 0),
        ] {
            let v = word(h, work + src);
            put(h, p + o, v);
            put(h, p + o + 4, hi);
        }
    }
}

/// The link quad from the packet's screen words, drawn twice
/// (`graphicDrawUIAbmapElement` kinds 9 and 0xc, other UVs).
fn link_quad(h: &mut impl Host, f: &Frame, work: u32) {
    let p = ABMAP_LINK_PACKET.get(h);
    for (o, src) in [
        (0x20, 0x70),
        (0x24, 0x74),
        (0x30, 0xe0),
        (0x34, 0xe4),
        (0x50, 0x80),
        (0x54, 0x84),
        (0x60, 0xf0),
        (0x64, 0xf4),
        (0x80, 0x90),
        (0x84, 0x94),
        (0x90, 0x100),
        (0x94, 0x104),
        (0xb0, 0xa0),
        (0xb4, 0xa4),
        (0xc0, 0x110),
        (0xc4, 0x114),
        (0xe0, 0x60),
        (0xe4, 0x64),
        (0xf0, 0xd0),
        (0xf4, 0xd4),
    ] {
        let v = word(h, work + src);
        let q = if o == 0x24 {
            ABMAP_LINK_PACKET.get(h)
        } else {
            p
        };
        put(h, q + o, v);
    }
    let k = k64(h, K1_16);
    let k2048 = k64(h, K2048);
    let k256 = k64(h, K256);
    let k208 = k64(h, K208);
    // Vertex n's x and y from the packet words at `src`.
    for (n, src) in [0x80u32, 0x90, 0xb0, 0xc0].into_iter().enumerate() {
        let r = 0x9c - 0x20 * n as u32;
        for (o, off) in [(0, k256), (4, k208)] {
            let v = int_f32(word(h, p + src + o).cast_signed());
            f.set(h, A0, v);
            f.st(h, r - o, add(sub(mul(load(v), k), k2048), off));
        }
        for i in 0..3 {
            f.set(h, r - 0x10 - 4 * i, 0x40);
        }
        f.set(h, r - 0x1c, 0x80);
    }
    let [u0, v0, u1, v1] = [UV_U0, UV_V0, UV_U1, UV_V1].map(|c| pass(h.read(h.rebase(c))));
    let rec = f.0 - REC;
    for (uv, kind) in [
        ([u0, v0, u1, v0, u0, v1, u1, v1], 9),
        ([u0, 0, u1, 0, u0, u1, u1, u1], 0xc),
    ] {
        for n in 0..4 {
            f.set(h, 0x94 - 0x20 * n, uv[2 * n as usize]);
            f.set(h, 0x90 - 0x20 * n, uv[2 * n as usize + 1]);
        }
        h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, 0, kind]);
    }
}

/// `AbmapDrawLink_00a51720(work, points, count)` (cdecl, no result any
/// caller reads): an unlit sphere grid link through `count` steps of
/// three points (x, y pairs; the first step's three at `points`, each
/// next one 0x18 bytes on from `points + 0x10`).
///
/// Each point goes through the grid's matrices (`FFXVu0ApplyMatrix`, the
/// first with `g_Abmap + 0x113e0`, the others `+0x114e0`) and is divided
/// by the first's w (`FFXVu0DivVectorXYZ`); the whole link stops when
/// that w is not above 0 (NaN goes on). The points become 12.4 fixed
/// point words in `work` (x 16 through `ftol`; the outer two doubled and
/// scaled down by `g_Abmap + 0x11350` x 0.7). The first step fills the
/// GS packet's fixed part (`g_AbmapLinkPacket`, after a busy-wait on
/// `g_GsRegisterSink` bit 0x100, written twice as the original does);
/// every next one writes the packet's corners and draws the link as a
/// quad in the frame twice (`graphicDrawUIAbmapElement` 9 and 0xc), then
/// moves its words down to the previous step's.
///
/// The original counts down its `count` argument word; the port does at
/// `slots + 8` (the caller's argument words) when given. With a count of
/// 1 the original runs on past 0 (until a w stops it), and so does the
/// port.
///
/// `frame` is the original's `ebp - 0xa8` ([`LINK_FRAME_WORDS`] words):
/// the scale, two temporaries, the quad record (`+0xc`, 0x98 bytes; its
/// words `+0x80..+0x8c` never written: the shim's frame holds 0).
pub fn abmap_draw_link(
    h: &mut impl Host,
    frame: u32,
    work: u32,
    points: u32,
    count: u32,
    slots: Option<u32>,
) {
    let f = Frame(frame + 0xa8);
    let abmap = ABMAP.get(h);
    let scale = load(word(h, abmap.wrapping_add(0x1_1350)));
    f.st(h, 0xa8, mul(scale, k64(h, K07)));
    let v = word(h, points);
    put(h, work, v);
    let v = word(h, points + 4);
    put(h, work + 4, v);
    put(h, work + 8, 0);
    put(h, work + 0xc, 0x3f80_0000);
    f.set(h, 0xc, 0);
    f.set(h, 8, 0);
    if !project(h, work, 0, 0x1_13e0, 0x30, true) {
        return;
    }
    for k in 1..3 {
        let v = word(h, points + 8 * k);
        put(h, work + 0x10 * k, v);
        let v = word(h, points + 8 * k + 4);
        put(h, work + 0x10 * k + 4, v);
        put(h, work + 0x10 * k + 8, 0);
        put(h, work + 0x10 * k + 0xc, 0x3f80_0000);
        if k == 2 {
            f.set(h, A0, points + 0x10);
        }
        project(h, work, k, 0x1_14e0, 0x30 + 0x10 * k, false);
    }
    fixed(h, &f, work, 0x60, 0xa4);
    packet_head(h, work);
    let mut left = count.wrapping_sub(1);
    if let Some(s) = slots {
        h.write(s + 8, left);
    }
    let mut p = points + 0x10;
    loop {
        for k in 0..3 {
            let src = p + 8 + 8 * k;
            let v = word(h, src);
            put(h, work + 0x10 * k, v);
            let v = word(h, src + 4);
            put(h, work + 0x10 * k + 4, v);
            let (m, check) = if k == 0 {
                (0x1_13e0, true)
            } else {
                (0x1_14e0, false)
            };
            if !project(h, work, k, m, 0x30 + 0x10 * k, check) {
                return;
            }
        }
        p += 0x18;
        fixed(h, &f, work, 0xd0, A0);
        link_quad(h, &f, work);
        for i in 0..20 {
            let v = word(h, work + 0xd0 + 4 * i);
            put(h, work + 0x60 + 4 * i, v);
        }
        left = left.wrapping_sub(1);
        if let Some(s) = slots {
            h.write(s + 8, left);
        }
        if left == 0 {
            return;
        }
    }
}
