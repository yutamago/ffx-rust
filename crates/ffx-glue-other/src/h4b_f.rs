//! List `h4-glue-other`, second agent (h4b), round 2 batch 1: the sphere
//! grid's three textured quads, `GDraw`'s two render-target begins and the
//! cross-font string drawer.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{REMAP_Y2, int_f32, pass};
use crate::h4b_b::{GDRAW_STATE, GDRAW_WARNING, QUAD_COLOUR};

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

// ---- the sphere grid's quads

/// `g_Abmap` (`0x02305834`), the sphere grid's data.
pub const ABMAP: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0230_5834);
/// The CRT's `sprintf` import slot.
pub const SPRINTF: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c3dc);
/// `TOGetFFXLangJmp()` (cdecl).
pub const GET_LANG: u32 = 0x008a_c2f0;
/// `graphicDrawUIElement(record, name, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// `graphicDrawUIAbmapElement(record, name, kind)` (cdecl).
pub const DRAW_ABMAP_ELEMENT: u32 = 0x0063_e930;
/// The GS packet buffer pointer (`0x01a860ec`, two `user_malloc(0x200)`
/// blocks from `eiAbmStart`).
pub const PACKET: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x01a8_60ec);
/// `_CIsin` (the import thunk).
pub const CI_SIN: u32 = 0x0094_98c8;

/// The record's colour words: `0x80` (grey, half alpha) for each of a
/// vertex's four.
const GREY: u32 = 0x80;

/// The grid texture's name (`12288_19.dds.phyre`) into `buf` by the grid
/// type `g_Abmap + 0x116a4`: type 0 from `dat00`, type 2 from `dat12`
/// (`sprintf` through its import slot, read once per type), and again
/// from `menu_cn` for language 0xb. Another type leaves `buf` as it is:
/// the original then hands `graphicDrawUIElement` its never-written
/// stack buffer (see the callers).
fn abmap_texture_name(h: &mut impl Host, abmap: u32, buf: u32) {
    let fmt = match h.read::<u32>(abmap + 0x116a4) {
        0 => 0x00b7_b4c4,
        2 => 0x00b7_b510,
        _ => return,
    };
    let sprintf = SPRINTF.get(h);
    let (fmt, root, cn, tex) = (
        h.rebase(fmt),
        h.rebase(0x00b7_b45c),
        h.rebase(0x00b7_b4e4),
        h.rebase(0x00b7_b450),
    );
    h.call(sprintf, Conv::Cdecl, &[buf, fmt, root, tex]);
    if h.call_sym(GET_LANG, Conv::Cdecl, &[]) == 0xb {
        h.call(sprintf, Conv::Cdecl, &[buf, fmt, cn, tex]);
    }
}

/// `AbmapDrawGridTexture_00a4c460(level, size)` (cdecl): a GS sprite
/// packet at `[0x01a860ec]` (its fixed words, the level `<< 24` as a
/// 64-bit word at `+0x40`, `size` at `+0x58` and `+0x68`), then the grid
/// texture (`abmap_texture_name`) as a `graphicDrawUIElement(record,
/// name, 1, 0, 0)` record in the frame: two zero floats, the image's
/// constants `0x00b7b618`, `0x00b7b5fc`, three zeros, `2 * level`,
/// `0x00b41d9c`, `0x00b41d98`, `0x00b7b640`, `0x00b7b610`, three zeros,
/// `2 * level`; `+0x90` and `+0x94` zero.
///
/// `frame` is the original's `ebp - 0x19c` (the record; the name buffer
/// at `+0x98`, 256 bytes). Returns the draw's `eax`.
///
/// Junk read: for a grid type other than 0 and 2 the original passes its
/// name buffer unwritten (`[ebp-0x104]` never written); the port's is
/// zeroed, an empty name (brief 2026-10-24).
pub fn grid_texture(h: &mut impl Host, frame: u32, level: u32, size: u32) -> u32 {
    let ebp = frame + 0x19c;
    h.write(ebp - 0x10c, 0u32);
    h.write(ebp - 0x108, 0u32);
    let out = PACKET.get(h);
    h.write(out, 0x8001u32);
    let again = PACKET.get(h);
    h.write(out + 8, 0x0055_eeeeu32);
    h.write(again + 4, 0x6023_4000u32);
    let fixed: [(u32, u32); 21] = [
        (0xc, 0),
        (0x10, 0x3100_0108),
        (0x14, 0),
        (0x18, 0x4e),
        (0x1c, 0),
        (0x20, 0x3_0000),
        (0x24, 0),
        (0x28, 0x47),
        (0x2c, 0),
        (0x30, 0x44),
        (0x34, 0),
        (0x38, 0x42),
        (0x3c, 0),
        (0x48, 1),
        (0x4c, 0),
        (0x50, 0x7000),
        (0x54, 0x7300),
        (0x5c, 0),
        (0x60, 0x9000),
        (0x64, 0x8d00),
        (0x6c, 0),
    ];
    for (off, v) in fixed {
        h.write(out + off, v);
    }
    let wide = i64::from(level.cast_signed()) << 24;
    h.write(out + 0x44, (wide >> 32) as u32);
    h.write(out + 0x40, wide as u32);
    h.write(out + 0x58, size);
    h.write(out + 0x68, size);
    let two = level.wrapping_mul(2);
    let rec = [
        0,
        0,
        konst(h, 0x00b7_b618),
        konst(h, 0x00b7_b5fc),
        0,
        0,
        0,
        two,
        konst(h, 0x00b4_1d9c),
        konst(h, 0x00b4_1d98),
        konst(h, 0x00b7_b640),
        konst(h, 0x00b7_b610),
        0,
        0,
        0,
        two,
    ];
    for (k, v) in rec.iter().enumerate() {
        h.write(frame + 4 * k as u32, *v);
    }
    let buf = ebp - 0x104;
    let abmap = ABMAP.get(h);
    // junk read: [ebp-0x104] never written for other grid types; 0 (brief 2026-10-24)
    abmap_texture_name(h, abmap, buf);
    h.call_sym(DRAW_UI_ELEMENT, Conv::Cdecl, &[frame, buf, 1, 0, 0])
}

/// Writes a sphere grid quad record: the four vertices' `(x, y, u, v)`
/// at `+0`, `+0x20`, `+0x40`, `+0x60`. The colours (`+0x10..+0x1c` of
/// each) are written apart.
fn quad_vertices(h: &mut impl Host, rec: u32, v: [[u32; 4]; 4]) {
    for (i, vert) in v.iter().enumerate() {
        for (k, w) in vert.iter().enumerate() {
            h.write(rec + 0x20 * i as u32 + 4 * k as u32, *w);
        }
    }
}

/// The 16 colour words of a quad record: `0x80` each.
fn quad_grey(h: &mut impl Host, rec: u32) {
    for i in 0..4u32 {
        for k in 0..4u32 {
            h.write(rec + 0x20 * i + 0x10 + 4 * k, GREY);
        }
    }
}

/// `AbmapDrawLangTitle_00a45ab0(_, x)` (cdecl): the language's title over
/// a grid panel, two `graphicDrawUIAbmapElement(record, name, 6)` quads
/// in the frame. The cell `(a, b)` of the title strip comes from
/// `TOGetFFXLangJmp`: (1, 0) for 1 and 0x12, (`0x00b92268`, 0) for 2,
/// (1, 1) for 3, (0, 1) for 5, else (0, 0) (`ebp - 0x1a4`, `- 0x1a8`).
/// The frame (`x` at the left): `x .. x + 0x00b7b6a8` by the constants
/// `0x00b7b6d4` .. `0x00b7b6e4`, UVs 0 .. `0x00b7b5e4` and `0x00b5ed84` ..
/// `0x00b7b5ec`; the title: `x + 0x00b7b690 - k` .. `k + (x +
/// 0x00b7b698)` (k = 6 for language 0xb, else 0) by `0x00b7b6dc` ..
/// `0x00b7b6e0`, UVs from the cell (`a * 0x00b4c170 * 0x00b58b50`, `b *
/// 0x00b7b6a0 * 0x00b58b50`, the far edges with `a + 1`, `b + 1`).
///
/// `frame` is the original's `ebp - 0x1ac`: `+0` the title's `k`, `+4`
/// b, `+8` a, `+0xc` the float temporary, `+0x10` the record (0x80
/// bytes), `+0xa0`/`+0xa4` zero, `+0xa8` the name buffer the callee
/// fills. Returns the second draw's `eax`.
#[allow(clippy::similar_names)]
pub fn lang_title(h: &mut impl Host, frame: u32, x: u32) -> u32 {
    let ebp = frame + 0x1ac;
    let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
    h.write(ebp - 0x1ac, 0u32);
    h.write(ebp - 0x10c, 0u32);
    h.write(ebp - 0x108, 0u32);
    let one = 1.0f32.to_bits();
    let (a, b) = match lang {
        1 | 0x12 => (one, 0),
        2 => (konst(h, 0x00b9_2268), 0),
        3 => (one, one),
        5 => (0, one),
        _ => (0, 0),
    };
    h.write(ebp - 0x1a4, a);
    h.write(ebp - 0x1a8, b);
    let rec = ebp - 0x19c;
    let name = ebp - 0x104;
    let x64 = load(x);
    let (y0, y1) = (konst(h, 0x00b7_b6d4), konst(h, 0x00b7_b6e4));
    let x1 = store(add(x64, konst64(h, 0x00b7_b6a8)));
    h.write(ebp - 0x1a0, x1);
    let (u1, v0, v1) = (
        konst(h, 0x00b7_b5e4),
        konst(h, 0x00b5_ed84),
        konst(h, 0x00b7_b5ec),
    );
    let xp = pass(x);
    quad_vertices(
        h,
        rec,
        [
            [xp, y0, 0, v0],
            [xp, y1, 0, v1],
            [x1, y0, u1, v0],
            [x1, y1, u1, v1],
        ],
    );
    quad_grey(h, rec);
    h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, name, 6]);
    let k: i32 = if h.call_sym(GET_LANG, Conv::Cdecl, &[]) == 0xb {
        h.write(ebp - 0x1ac, 6u32);
        6
    } else {
        0
    };
    let kf = load(int_f32(k));
    let x0 = store(sub(add(konst64(h, 0x00b7_b690), x64), kf));
    let (y0, y1) = (konst(h, 0x00b7_b6dc), konst(h, 0x00b7_b6e0));
    let x1 = store(add(kf, add(x64, konst64(h, 0x00b7_b698))));
    let (a, b) = (load(a), load(b));
    let (d4, d5, d6) = (
        konst64(h, 0x00b4_c170),
        konst64(h, 0x00b5_8b50),
        konst64(h, 0x00b7_b6a0),
    );
    let u0 = store(mul(mul(a, d4), d5));
    let v0 = store(mul(mul(b, d6), d5));
    let v1 = store(mul(mul(d6, add(b, 1.0)), d5));
    let u1 = store(mul(d5, mul(d4, add(1.0, a))));
    h.write(ebp - 0x1a0, u1);
    quad_vertices(
        h,
        rec,
        [
            [x0, y0, u0, v0],
            [x0, y1, u0, v1],
            [x1, y0, u1, v0],
            [x1, y1, u1, v1],
        ],
    );
    h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, name, 6])
}

/// `AbmapDrawPadArrow_00a4f290(item, top, x)` (cdecl): a list's bobbing
/// arrow. Left edge `x + 2 + item.x` (`i16` at `+8`) less the bob
/// `g_Abmap + 0x116b4` / `0x00b2c5c8`. The bob moves with the step
/// counter `+0x116b8`: positive, `+= sin(step / 0x00b4b468 * 0x00b7b688) *
/// 0x00b92270` (the `_CIsin` result stored as `f32`) and the step counts
/// up; negative, the same with `-step` subtracted; at 0xb or -1 the step
/// turns round. Top edge `item.y + (item.row - top) * 20 - 2` (`i16`s at
/// `+0xa`, `+0x18`); the quad is `0x00b587d8` wide and `0x00b49968` high,
/// UVs `0x00b5f13c` .. `0x00b7b630`, `0x00b5ed60` .. `0x00b66b7c`, drawn
/// with the grid texture (`graphicDrawUIAbmapElement(record, name, 6)`).
///
/// `frame` is the original's `ebp - 0x1ac`: `+0` the 64-bit bob, `+8` the
/// left edge, `+0xc` the float temporary, `+0x10` the record, `+0xa0` and
/// `+0xa4` zero, `+0xa8` the name buffer. Returns the draw's `eax`.
///
/// Junk read: a grid type other than 0 and 2 leaves the name buffer
/// (`[ebp-0x104]`) never written; the port's is zeroed (brief
/// 2026-10-24).
#[allow(clippy::similar_names)]
pub fn pad_arrow(h: &mut impl Host, frame: u32, item: u32, top: u32, x: u32) -> u32 {
    let ebp = frame + 0x1ac;
    let first = ABMAP.get(h);
    let col = i32::from(h.read::<i16>(item + 8));
    let n = x.cast_signed().wrapping_add(2).wrapping_add(col);
    let left = load(int_f32(n));
    let mut abmap = ABMAP.get(h);
    let bob = div(load(h.read(first + 0x116b4)), konst64(h, 0x00b2_c5c8));
    let left = store(sub(left, bob));
    h.write(ebp - 0x1a4, left);
    let step: i32 = h.read(abmap + 0x116b8);
    if step > 0 {
        let k = load(int_f32(step));
        let angle = mul(div(k, konst64(h, 0x00b4_b468)), konst64(h, 0x00b7_b688));
        let sin = h.rebase(CI_SIN);
        let s = h.call_intrinsic(sin, angle).to_bits();
        let v = add(
            mul(load(s), konst64(h, 0x00b9_2270)),
            load(h.read(first + 0x116b4)),
        );
        h.write(first + 0x116b4, store(v));
        let c: u32 = h.read(abmap + 0x116b8);
        h.write(abmap + 0x116b8, c.wrapping_add(1));
        abmap = ABMAP.get(h);
    }
    let step: i32 = h.read(abmap + 0x116b8);
    if step < 0 {
        let old = load(h.read(abmap + 0x116b4));
        h.write(frame, old);
        let k = load(int_f32(step.wrapping_neg()));
        let angle = mul(div(k, konst64(h, 0x00b4_b468)), konst64(h, 0x00b7_b688));
        let sin = h.rebase(CI_SIN);
        let s = h.call_intrinsic(sin, angle).to_bits();
        let v = sub(old, mul(load(s), konst64(h, 0x00b9_2270)));
        h.write(abmap + 0x116b4, store(v));
        let c: u32 = h.read(abmap + 0x116b8);
        h.write(abmap + 0x116b8, c.wrapping_add(1));
        abmap = ABMAP.get(h);
    }
    let step: i32 = h.read(abmap + 0x116b8);
    if step == 0xb || step == -1 {
        h.write(abmap + 0x116b8, step.wrapping_neg());
        abmap = ABMAP.get(h);
    }
    let row = i32::from(h.read::<i16>(item + 0x18)).wrapping_sub(top.cast_signed());
    let y = i32::from(h.read::<i16>(item + 0xa))
        .wrapping_add(row.wrapping_mul(20))
        .wrapping_sub(2);
    h.write(ebp - 0x10c, 0u32);
    h.write(ebp - 0x108, 0u32);
    let top_edge = int_f32(y);
    h.write(ebp - 0x1a0, top_edge);
    let buf = ebp - 0x104;
    // junk read: [ebp-0x104] never written for other grid types; 0 (brief 2026-10-24)
    abmap_texture_name(h, abmap, buf);
    let rec = ebp - 0x19c;
    let bottom = store(add(load(top_edge), konst64(h, 0x00b4_9968)));
    let right = store(add(load(left), konst64(h, 0x00b5_87d8)));
    h.write(ebp - 0x1a4, right);
    let (u0, v0, v1, u1) = (
        konst(h, 0x00b5_f13c),
        konst(h, 0x00b5_ed60),
        konst(h, 0x00b6_6b7c),
        konst(h, 0x00b7_b630),
    );
    quad_vertices(
        h,
        rec,
        [
            [left, top_edge, u0, v0],
            [left, bottom, u0, v1],
            [right, top_edge, u1, v0],
            [right, bottom, u1, v1],
        ],
    );
    quad_grey(h, rec);
    h.call_sym(DRAW_ABMAP_ELEMENT, Conv::Cdecl, &[rec, buf, 6])
}

// ---- GDraw's render-target begins

/// `gdraw_allocRenderTarget_00a3b610(stats)` (cdecl).
pub const ALLOC_TARGET: u32 = 0x00a3_b610;
/// `0x00a3c570(stats)` (cdecl): binds the top target.
pub const BIND_TOP: u32 = 0x00a3_c570;
/// `gdraw_drawRectQuad_00a3bd40(rect, stats)` (cdecl).
pub const DRAW_RECT_QUAD: u32 = 0x00a3_bd40;
/// `op_node_requeue_00a39f00(target)` (cdecl).
pub const REQUEUE_NODE: u32 = 0x00a3_9f00;
/// `0x00a3b480(target, stats)` (cdecl): gives a target back.
pub const RELEASE_TARGET: u32 = 0x00a3_b480;
/// `GDraw`'s viewport setter `0x00a3ce00(x, y, w, h)` (cdecl).
pub const GDRAW_VIEWPORT: u32 = 0x00a3_ce00;
/// `GDraw`'s projection setter `0x00a3c450(x, w, h, y)` (cdecl).
pub const GDRAW_PROJECTION: u32 = 0x00a3_c450;
/// `0x00a3cd30()` (cdecl): the target's viewport.
pub const TARGET_VIEWPORT: u32 = 0x00a3_cd30;
/// `0x00a3c350()` (cdecl): the target's projection.
pub const TARGET_PROJECTION: u32 = 0x00a3_c350;
/// `gdraw_MapBuffer_00a3cec0(resource)` (cdecl).
pub const MAP_BUFFER: u32 = 0x00a3_cec0;
/// `GDraw`'s quad draw `0x00a36090(colour, stats, x0, y0, x1, y1)` (cdecl).
pub const DRAW_QUAD: u32 = crate::h4b_b::DRAW_QUAD;

/// Where a begin keeps its rectangle words: the frame offsets (from
/// `ebp`) of the four words `gdraw_drawRectQuad` reads.
pub(crate) struct RectAt(pub(crate) u32);

/// The part both begins share once the target is on the stack and bound:
/// the target's slot (`(target - state - 0x438) / 0x38`, toward zero) gets
/// the region. Without `rect`: `ClearRenderTargetView` (context vtable
/// `+0xc8`) of the top's view (`+8`) with the zero colour, and the slot
/// covers the screen (`+0x6c0`, `+0x6c4`). With it: the region less the
/// origin (`+0x6f0`, `+0x6f4` unless `+0x6f8`), clamped to 0 and the
/// target size (`+0x6e0`, `+0x6e4`), then grown by 2 and clamped to the
/// screen for the clear quad (`gdraw_drawRectQuad` with the rectangle at
/// `at`). An empty one pops the stack again and gives the target back
/// (`op_node_requeue` for a pool block, else `0x00a3b480`): `Err` with
/// that call's `eax`. Then the viewport and projection: the target's own
/// (`0x00a3cd30`, `0x00a3c350`) or with `+0x6f8` the full `+0x6e0` x
/// `+0x6e4` (`0x00a3ce00`, `0x00a3c450`).
pub(crate) fn place_target(
    h: &mut impl Host,
    at: &RectAt,
    rect: u32,
    target: u32,
    stats: u32,
) -> Result<u32, u32> {
    let state = GDRAW_STATE.get(h);
    let slot = (target.wrapping_sub(state).wrapping_sub(0x438).cast_signed() / 0x38) as u32;
    let row = |s: u32, k: u32| {
        s.wrapping_add(slot.wrapping_mul(16))
            .wrapping_add(0x630 + k)
    };
    let s = if rect == 0 {
        let top: u32 = h.read(state + 0xaac);
        let ctx: u32 = h.read(state + 4);
        let view: u32 = h.read(h.read::<u32>(top) + 8);
        let f: u32 = h.read(h.read::<u32>(ctx) + 0xc8);
        let colour = h.rebase(QUAD_COLOUR);
        h.call(f, Conv::Stdcall, &[ctx, view, colour]);
        let s = GDRAW_STATE.get(h);
        h.write(row(s, 0), 0u32);
        h.write(row(s, 4), 0u32);
        let w: u32 = h.read(s + 0x6c0);
        h.write(row(s, 8), w);
        let hh: u32 = h.read(s + 0x6c4);
        h.write(row(s, 0xc), hh);
        s
    } else {
        let (ox, oy) = if h.read::<u32>(state + 0x6f8) == 0 {
            (h.read::<i32>(state + 0x6f0), h.read::<i32>(state + 0x6f4))
        } else {
            (0, 0)
        };
        let r = |k: u32| h.read::<i32>(rect + 4 * k);
        let x0 = r(0).wrapping_sub(ox).max(0);
        let y0 = r(1).wrapping_sub(oy).max(0);
        let x1 = r(2).wrapping_sub(ox).min(h.read::<i32>(state + 0x6e0));
        let y1 = r(3).wrapping_sub(oy).min(h.read::<i32>(state + 0x6e4));
        let a = x0.wrapping_sub(2).max(0);
        let b = y0.wrapping_sub(2).max(0);
        let c = x1.wrapping_add(2).min(h.read::<i32>(state + 0x6c0));
        let d = y1.wrapping_add(2).min(h.read::<i32>(state + 0x6c4));
        let quad = at.0;
        for (k, v) in [a, b, c, d].iter().enumerate() {
            h.write(quad + 4 * k as u32, *v);
        }
        if c <= a || d <= b {
            let top: u32 = h.read(state + 0xaac);
            h.write(state + 0xaac, top.wrapping_sub(0x1c));
            let pool: u32 = h.read(target + 0x18);
            return Err(if pool == state + 0x290 {
                h.call_sym(REQUEUE_NODE, Conv::Cdecl, &[target])
            } else {
                h.call_sym(RELEASE_TARGET, Conv::Cdecl, &[target, stats])
            });
        }
        h.call_sym(DRAW_RECT_QUAD, Conv::Cdecl, &[quad, stats]);
        let s = GDRAW_STATE.get(h);
        for (k, v) in [x0, y0, x1, y1].iter().enumerate() {
            h.write(row(s, 4 * k as u32), *v);
        }
        s
    };
    if h.read::<u32>(s + 0x6f8) == 0 {
        h.call_sym(TARGET_VIEWPORT, Conv::Cdecl, &[]);
        Ok(h.call_sym(TARGET_PROJECTION, Conv::Cdecl, &[]))
    } else {
        let (w, hh): (u32, u32) = (h.read(s + 0x6e0), h.read(s + 0x6e4));
        h.call_sym(GDRAW_VIEWPORT, Conv::Cdecl, &[0, 0, w, hh]);
        let s = GDRAW_STATE.get(h);
        let (w, hh): (u32, u32) = (h.read(s + 0x6e0), h.read(s + 0x6e4));
        Ok(h.call_sym(GDRAW_PROJECTION, Conv::Cdecl, &[0, w, hh, 0]))
    }
}

/// The checks both begins start with: a zero-sized screen (`+0x6d8`,
/// `+0x6dc`) or a full target stack (`+0xaac` + 0x1c past its own
/// address, unsigned) warns (`IggyGDrawSendWarning(0, text)`) and gives
/// `Err` with the warning's `eax`.
pub(crate) fn begin_checks(h: &mut impl Host) -> Result<(), u32> {
    let state = GDRAW_STATE.get(h);
    let stack = state + 0xaac;
    let next = h.read::<u32>(stack).wrapping_add(0x1c);
    let text = if h.read::<u32>(state + 0x6d8) == 0 || h.read::<u32>(state + 0x6dc) == 0 {
        0x00b7_a7d8
    } else if next >= stack {
        0x00b7_a7fc
    } else {
        return Ok(());
    };
    let f = GDRAW_WARNING.get(h);
    let text = h.rebase(text);
    Err(h.call(f, Conv::Cdecl, &[0, text]))
}

/// `gdraw_textureDrawBufferBegin_00a3ac90(rect, _, format, keep, stats)`
/// (stdcall, `GDraw`'s table): pushes a render target. Unless `keep`, a
/// new one (`gdraw_allocRenderTarget`; none gives 0); with `keep` the
/// entry's target is 0 and it records `rect` (x, y, w, h at `+4`).
/// The entry: target at `+0`, `format` at `+0x14`, `keep != 0` at
/// `+0x18`. Then `0x00a3c570(stats)` binds it and [`place_target`]
/// clears and sizes it (the rectangle in the frame). Returns 1, or 0
/// when it fails.
///
/// `frame` is the original's `ebp - 0x18` (the rectangle, 4 words).
pub fn texture_draw_begin(
    h: &mut impl Host,
    frame: u32,
    rect: u32,
    format: u32,
    keep: u32,
    stats: u32,
) -> u32 {
    if begin_checks(h).is_err() {
        return 0;
    }
    let state = GDRAW_STATE.get(h);
    let entry = h.read::<u32>(state + 0xaac).wrapping_add(0x1c);
    let target = if keep == 0 {
        let t = h.call_sym(ALLOC_TARGET, Conv::Cdecl, &[stats]);
        if t == 0 {
            return 0;
        }
        t
    } else {
        0
    };
    let state = if keep == 0 { GDRAW_STATE.get(h) } else { state };
    h.write(entry + 0x14, format);
    h.write(entry, target);
    let top = h.read::<u32>(state + 0xaac).wrapping_add(0x1c);
    h.write(state + 0xaac, top);
    let top: u32 = h.read(state + 0xaac);
    h.write(top + 0x18, u32::from(keep != 0));
    if keep != 0 {
        let values = [(0, None), (1, None), (2, Some(0)), (3, Some(1))];
        for (k, (a, b)) in values.into_iter().enumerate() {
            let mut v: u32 = h.read(rect + 4 * a);
            if let Some(b) = b {
                v = v.wrapping_sub(h.read(rect + 4 * b));
            }
            let top: u32 = h.read(state + 0xaac);
            h.write(top + 4 + 4 * k as u32, v);
        }
    }
    h.call_sym(BIND_TOP, Conv::Cdecl, &[stats]);
    match place_target(h, &RectAt(frame), rect, target, stats) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// `gdraw_filterPass_00a374e0(filter, rect, colour, stats)` (cdecl):
/// renders a filter's quad into a new target. The target
/// (`gdraw_allocRenderTarget`; none: nothing) is pushed as format 3, not
/// kept, bound and placed ([`place_target`], the rectangle at `ebp -
/// 0x24`). Then the filter's input: the pixel shader resource of
/// `filter + 0x1c` (its `+4`) and the sampler `state + 0x7b4`, or a null
/// resource (a zero word at `ebp - 0x14`); the pixel shader `state +
/// 0x254`; the filter's 20 constant words (`+0x88`) into the mapped
/// buffer `state + 0x990`, unmapped (vtable `+0x3c`) and bound
/// (`PSSetConstantBuffers`, the buffer at `ebp - 0x10`); the quad
/// `0x00a36090(colour, stats, x0, y0, x1, y1)` over `rect` (floats). The
/// filter's input gets the target's size (`+0x28`, `+0x2c` from
/// `+0xac0`, `+0xac4`). Last the target is popped: while the stack is
/// still valid the one below is bound (`0x00a3c570`, `0x00a3cd30`,
/// `0x00a3c350`) and `filter + 0x1c` takes the popped target, else 0.
/// Returns the last `eax`.
///
/// `frame` is the original's `ebp - 0x24` (9 words). `rect` must not be
/// null: the original reads it for the quad either way.
pub fn filter_pass(
    h: &mut impl Host,
    frame: u32,
    filter: u32,
    rect: u32,
    colour: u32,
    stats: u32,
) -> u32 {
    let ebp = frame + 0x24;
    if let Err(eax) = begin_checks(h) {
        return eax;
    }
    let state = GDRAW_STATE.get(h);
    let entry = h.read::<u32>(state + 0xaac).wrapping_add(0x1c);
    let target = h.call_sym(ALLOC_TARGET, Conv::Cdecl, &[stats]);
    h.write(ebp - 0x14, target);
    if target == 0 {
        return 0;
    }
    let state = GDRAW_STATE.get(h);
    h.write(entry + 0x14, 3u32);
    h.write(entry, target);
    let top = h.read::<u32>(state + 0xaac).wrapping_add(0x1c);
    h.write(state + 0xaac, top);
    let top: u32 = h.read(state + 0xaac);
    h.write(top + 0x18, 0u32);
    h.call_sym(BIND_TOP, Conv::Cdecl, &[stats]);
    if let Err(eax) = place_target(h, &RectAt(frame), rect, target, stats) {
        return eax;
    }
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let input: u32 = h.read(filter + 0x1c);
    let vt: u32 = h.read(ctx);
    if input == 0 {
        h.write(ebp - 0x14, 0u32);
        let f: u32 = h.read(vt + 0x20);
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, ebp - 0x14]);
    } else {
        let f: u32 = h.read(vt + 0x20);
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, input + 4]);
        let sampler = GDRAW_STATE.get(h) + 0x7b4;
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt + 0x28);
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, sampler]);
    }
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x24);
    let shader: u32 = h.read(state + 0x254);
    h.call(f, Conv::Stdcall, &[ctx, shader, 0, 0]);
    let state = GDRAW_STATE.get(h);
    let buffer: u32 = h.read(state + 0x990);
    let dst = h.call_sym(MAP_BUFFER, Conv::Cdecl, &[buffer]);
    let state = GDRAW_STATE.get(h);
    for k in 0..20u32 {
        let v: u32 = h.read(filter + 0x88 + 4 * k);
        h.write(dst + 4 * k, v);
    }
    let buffer: u32 = h.read(state + 0x990);
    let ctx: u32 = h.read(state + 4);
    h.write(ebp - 0x10, buffer);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x3c);
    h.call(f, Conv::Stdcall, &[ctx, buffer, 0]);
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x40);
    h.call(f, Conv::Stdcall, &[ctx, 1, 1, ebp - 0x10]);
    let r = |k: u32| int_f32(h.read::<i32>(rect + 4 * k));
    let (y1, x1, y0, x0) = (r(3), r(2), r(1), r(0));
    h.write(ebp - 0xc, x0);
    h.call_sym(DRAW_QUAD, Conv::Cdecl, &[colour, stats, x0, y0, x1, y1]);
    let state = GDRAW_STATE.get(h);
    let input: u32 = h.read(filter + 0x1c);
    if input != 0 {
        let w: u32 = h.read(state + 0xac0);
        h.write(input + 0x28, w);
        let hh: u32 = h.read(state + 0xac4);
        h.write(input + 0x2c, hh);
    }
    let stack = state + 0xaac;
    let popped: u32 = h.read(stack);
    h.write(stack, popped.wrapping_sub(0x1c));
    if h.read::<u32>(state + 0x6d8) != 0 && h.read::<u32>(state + 0x6dc) != 0 && popped < stack {
        h.call_sym(BIND_TOP, Conv::Cdecl, &[stats]);
        h.call_sym(TARGET_VIEWPORT, Conv::Cdecl, &[]);
        h.call_sym(TARGET_PROJECTION, Conv::Cdecl, &[]);
        let t: u32 = h.read(popped);
        h.write(filter + 0x1c, t);
        t
    } else {
        h.write(filter + 0x1c, 0u32);
        0
    }
}

// ---- TOAdpCrossEasyStrFontSClutRGBA

/// `TOGetGroup(c)` (cdecl).
pub const GET_GROUP: u32 = 0x008d_f610;
/// `TOGetFontInfo(text, out)` (cdecl).
pub const GET_FONT_INFO: u32 = 0x008b_7240;
/// `graphicSetUiDrawType(t)` (cdecl).
pub const SET_DRAW_TYPE: u32 = 0x0064_3af0;
/// `graphicFontSetUiShadowType(t)` (cdecl).
pub const SET_SHADOW_TYPE: u32 = 0x0064_0dd0;
/// `TOMakePktBtlEasyFontShadow(pen, glyph, scale)` (cdecl).
pub const FONT_SHADOW: u32 = 0x008f_bae0;
/// `TOMakePktBtlEasyFont(pen, glyph, scale)` (cdecl).
pub const FONT: u32 = 0x008f_b940;
/// `TOAddFontWidthSpace(pen, glyph, scale)` (cdecl).
pub const FONT_WIDTH_SPACE: u32 = 0x008e_8ed0;
/// `TOSkipCode(text)` (cdecl).
pub const SKIP_CODE: u32 = 0x008d_fd80;
/// `graphicFontGetScreenWH(&w, &h)` (cdecl).
pub const FONT_SCREEN_WH: u32 = 0x0064_0db0;
/// `crt_memset_thunk(p, c, n)`.
pub const MEMSET: u32 = 0x0094_964c;
/// The glyph record `TOGetFontInfo` fills (0x40 bytes; `+0x1c` the
/// glyph width).
pub const GLYPH: u32 = 0x0184_1d50;
/// The pen: colour bytes `+0`..`+3`, x `+4`, y `+8`, three zero words,
/// `+0x18` 0, `+0x1a` 0 and `+0x1b` the alpha.
pub const PEN: u32 = 0x025d_09a0;

/// One character of [`cross_str`]'s conversion to the cross font's
/// codes, as the language asks (`TOGetFFXLangJmp` called as often as the
/// original does).
enum Code {
    /// One or two codes to write; then the next character.
    Out(u8, Option<u8>),
    /// Nothing written; the next character.
    Skip,
    /// The same character again: the original never steps past a letter
    /// for a language below 0 or above 8 other than 9 and 10, and loops
    /// for good, as the port does.
    Again,
}

/// [`Code`] for character `c`.
fn cross_code(h: &mut impl Host, c: u8) -> Code {
    let s = c.cast_signed();
    if (0x30..=0x39).contains(&s) {
        return Code::Out(c, None);
    }
    let (letter, add_low, sub_jp, add_cjk) = if (0x41..=0x5a).contains(&s) {
        (true, 0xf, 0x11, 0x19)
    } else if (0x61..=0x7a).contains(&s) {
        (true, 0xf, 0x17, 0x13)
    } else {
        (false, 0, 0, 0)
    };
    if letter {
        if h_lang(h) > 0 && h_lang(h) < 9 {
            return Code::Out(c.wrapping_add(add_low), None);
        }
        if h_lang(h) == 0 {
            return Code::Out(b',', Some(c.wrapping_sub(sub_jp)));
        }
        if h_lang(h) == 10 || h_lang(h) == 9 {
            return Code::Out(c.wrapping_add(add_cjk), None);
        }
        return Code::Again;
    }
    match c {
        b' ' => Code::Out(b':', None),
        b'-' => Code::Out(b'G', None),
        b'/' => Code::Out(b'I', None),
        b':' => Code::Out(b'J', None),
        _ => Code::Skip,
    }
}

/// `TOGetFFXLangJmp()` as a signed number.
fn h_lang(h: &mut impl Host) -> i32 {
    h.call_sym(GET_LANG, Conv::Cdecl, &[]).cast_signed()
}

/// `TOAdpCrossEasyStrFontSClutRGBA(text, x, y, a, r, g, b, alpha, scale,
/// space)` (cdecl, floats): `text` converted into the cross font's codes
/// in the frame (cleared first with `memset`; digits as they are, letters
/// shifted by language, `,` and a code for language 0, `' '`, `-`, `/`,
/// `:` to `:`, `G`, `I`, `J`, others dropped), then drawn glyph by glyph
/// with the pen at `0x025d09a0`: x, `y - remapY2(0x00b55140)` (stored in
/// `y`'s argument slot), colours `r, g, b, alpha` and `a` at `+0x1b`. A
/// line break (group 1) resets x and moves y by `0x00b313a0`. A glyph
/// (group 4) is looked up (`TOGetFontInfo` into `0x01841d50`) and drawn
/// with its shadow at `scale` and its width added
/// (`TOAddFontWidthSpace`); for languages 1..=8 the `J` and `I` codes
/// are centred in an 8-wide cell instead (`(0x00b922a8 - width) / ratio *
/// 0x00b92220`, the scale `scale * 0x00b48328`, then the advance `((8 -
/// width) * 0.5 + width) / ratio + space` times `scale`), `ratio` the
/// screen's `w / h / 0x00b43d60` (each step stored as `f32`).
///
/// `frame` is the original's `ebp - 0x118`: `+0` h, `+4` w, `+8` the
/// scaled scale, `+0xc` the 64-bit `y`, `+0x10` the ratio, `+0x14` the
/// text (256 bytes; the port's frame should be larger, as the
/// conversion can write two bytes a character). `slots` is the
/// argument words' address. Returns the last call's `eax`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn cross_str(
    h: &mut impl Host,
    frame: u32,
    text: u32,
    x: u32,
    y: u32,
    alpha_a: u32,
    rgba: [u32; 4],
    scale: u32,
    space: u32,
    slots: Option<u32>,
) -> u32 {
    let ebp = frame + 0x118;
    let buf = ebp - 0x104;
    h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
    let mut src = text;
    let mut out = buf;
    loop {
        let c: u8 = h.read(src);
        if c == 0 {
            break;
        }
        match cross_code(h, c) {
            Code::Out(a, b) => {
                h.write(out, a);
                out += 1;
                if let Some(b) = b {
                    h.write(out, b);
                    out += 1;
                }
                src += 1;
            }
            Code::Skip => src += 1,
            Code::Again => {}
        }
    }
    let y64 = load(y);
    h.write(ebp - 0x10c, y64);
    h.write(out, 0u8);
    let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[konst(h, 0x00b5_5140)]);
    let y = store(sub(y64, r));
    if let Some(s) = slots {
        h.write(s + 8, y);
    }
    h.call_sym(FONT_SCREEN_WH, Conv::Cdecl, &[ebp - 0x114, ebp - 0x118]);
    let w = load(int_f32(h.read(ebp - 0x114)));
    let hh = int_f32(h.read(ebp - 0x118));
    let ratio = store(div(w, load(hh)));
    let ratio = store(div(load(ratio), konst64(h, 0x00b4_3d60)));
    h.write(ebp - 0x108, ratio);
    let glyph = h.rebase(GLYPH);
    h.call_sym(MEMSET, Conv::Cdecl, &[glyph, 0, 0x40]);
    let pen = h.rebase(PEN);
    for (off, v) in [(0u32, 0u32), (0x18, 0), (4, 0)] {
        h.write(pen + off, v);
    }
    h.write(pen + 4, pass(x));
    h.write(pen + 8, 0u32);
    h.write(pen + 0xc, 0u32);
    h.write(pen + 8, pass(y));
    h.write(pen + 0x10, 0u32);
    h.write(pen + 0x14, 0u32);
    for (k, c) in rgba.iter().enumerate() {
        h.write(pen + k as u32, *c as u8);
    }
    h.write(pen + 0x1a, 0u8);
    h.write(pen + 0x1b, alpha_a as u8);
    // `eax` was cleared for the pen and holds the last byte moved
    let mut eax = alpha_a & 0xff;
    let mut at = buf;
    if h.read::<u8>(buf) == 0 {
        return eax;
    }
    loop {
        let c: u8 = h.read(at);
        match h.call_sym(GET_GROUP, Conv::Cdecl, &[u32::from(c)]) {
            1 => {
                h.write(pen + 4, pass(x));
                let v = add(load(h.read(pen + 8)), konst64(h, 0x00b3_13a0));
                h.write(pen + 8, store(v));
            }
            4 => {
                h.call_sym(GET_FONT_INFO, Conv::Cdecl, &[at, glyph]);
                let narrow = h_lang(h) > 0 && h_lang(h) < 9 && {
                    let c: u8 = h.read(at);
                    c == b'J' || c == b'I'
                };
                if narrow {
                    let width = load(h.read(glyph + 0x1c));
                    let v = add(
                        mul(
                            div(sub(konst64(h, 0x00b9_22a8), width), load(ratio)),
                            konst64(h, 0x00b9_2220),
                        ),
                        load(h.read(pen + 4)),
                    );
                    h.write(pen + 4, store(v));
                    h.call_sym(SET_DRAW_TYPE, Conv::Cdecl, &[1]);
                    h.call_sym(SET_SHADOW_TYPE, Conv::Cdecl, &[0]);
                    let s = store(mul(load(scale), konst64(h, 0x00b4_8328)));
                    h.write(ebp - 0x110, s);
                    h.call_sym(FONT_SHADOW, Conv::Cdecl, &[pen, glyph, pass(s)]);
                    h.call_sym(SET_SHADOW_TYPE, Conv::Cdecl, &[1]);
                    h.call_sym(FONT, Conv::Cdecl, &[pen, glyph, pass(s)]);
                    h.call_sym(SET_DRAW_TYPE, Conv::Cdecl, &[0]);
                    let width = load(h.read(glyph + 0x1c));
                    let half = mul(sub(konst64(h, 0x00b9_22a8), width), konst64(h, 0x00b9_2220));
                    let v = add(
                        mul(
                            add(div(add(width, half), load(ratio)), load(space)),
                            load(scale),
                        ),
                        load(h.read(pen + 4)),
                    );
                    h.write(pen + 4, store(v));
                } else {
                    h.call_sym(SET_DRAW_TYPE, Conv::Cdecl, &[1]);
                    h.call_sym(SET_SHADOW_TYPE, Conv::Cdecl, &[0]);
                    h.call_sym(FONT_SHADOW, Conv::Cdecl, &[pen, glyph, pass(scale)]);
                    h.call_sym(SET_SHADOW_TYPE, Conv::Cdecl, &[1]);
                    h.call_sym(FONT, Conv::Cdecl, &[pen, glyph, pass(scale)]);
                    h.call_sym(SET_DRAW_TYPE, Conv::Cdecl, &[0]);
                    h.call_sym(FONT_WIDTH_SPACE, Conv::Cdecl, &[pen, glyph, pass(scale)]);
                }
            }
            _ => {}
        }
        at = h.call_sym(SKIP_CODE, Conv::Cdecl, &[at]);
        eax = at;
        if h.read::<u8>(at) == 0 {
            return eax;
        }
    }
}
