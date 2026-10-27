//! List `h4-glue-other`, second agent (h4b), round 2 batch 2: the shape
//! cursor packet, the PS2 debug assert and `GDraw`'s three-input filter
//! pass.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

use crate::h4_e1::{REMAP_X2, REMAP_Y2, int_f32, pass};
use crate::h4b_b::{GDRAW_STATE, GDRAW_WARNING};
use crate::h4b_f::{
    ALLOC_TARGET, BIND_TOP, DRAW_QUAD, MAP_BUFFER, RectAt, TARGET_PROJECTION, TARGET_VIEWPORT,
    begin_checks, place_target,
};

/// The `f32` constant at image address `sym`, through `fld`/`fstp`.
fn konst(h: &impl Host, sym: u32) -> u32 {
    pass(h.read(h.rebase(sym)))
}

/// The `f64` constant at image address `sym`.
fn konst64(h: &impl Host, sym: u32) -> f64 {
    h.read(h.rebase(sym))
}

// ---- TOMakePktShapeCursor

/// `graphicGetTime()` (cdecl, `ST0`).
pub const GET_TIME: u32 = 0x0064_1410;
/// `_CIcos` (the import thunk).
pub const CI_COS: u32 = 0x0094_98c2;
/// `TOMkpShapeXYWHUV(id, x, y, w, h, u0, v0, u1, v1)` (cdecl).
pub const SHAPE_XYWHUV: u32 = 0x0090_3bb0;
/// `TOCopyWindowLayout_008fa7d0(i)` (cdecl).
pub const COPY_LAYOUT: u32 = 0x008f_a7d0;
/// `TOGetShapTextureName(id)` (cdecl).
pub const SHAPE_TEXTURE_NAME: u32 = 0x008a_c8c0;
/// `TOGetImageWH(id, &w, &h)` (cdecl).
pub const IMAGE_WH: u32 = 0x008a_c400;
/// `graphicDrawUIElement(record, name, a, b, c)` (cdecl).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// `g_InvokedFromSelectCursor`.
pub const SELECT_CURSOR: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0187_16f0);

/// `TOMakePktShapeCursor(kind, x, y)` (cdecl): the menus' hand cursor.
///
/// Kind 0: the bobbing hand, `TOMkpShapeXYWHUV(400, x', y', w, h,
/// 0x00b51b9c, 0x00b66c08, 0x00b66b74, 0x00b66c0c)` with `h =
/// remapY2(0x00b5ee90)`, `w = remapX2(0x00b5eec0)`, `y' = y -
/// remapY2(0x00b3eb98)` and `x' = x + bob - remapX2(0x00b5eec0)`, the bob
/// `cos(graphicGetTime() * 0x00b922a8) * 0x00b2c5c8` (each stored as
/// `f32`; `x + bob` kept as `f64`). Returns that call's `eax`.
///
/// Otherwise a window layout's cell: kinds 4 and 5 use layout 8 and 9,
/// `x` 30 left or right, and draw flag 0; another kind uses layout 0 and
/// flag 1 (see the junk read below). Every entry of the layout
/// (`TOCopyWindowLayout_008fa7d0`; a count, then 12-byte entries: u, v
/// bytes, dx, dy `i8`, w, h `u16`) after the first rewrites the quad
/// record in the frame: its edges at fractions of the cell (3/20 and
/// 51/80 of the width, or 8/20 and 18/20 when
/// `g_InvokedFromSelectCursor`; 3/20 and 17/20 of the height) plus
/// `x + dx`, `y + dy`; its UVs `(u + 1) / iw`, `(v + 1) / ih`,
/// `(w + u - 1) / iw`, `(h + v - 1) / ih` over image 0x3e00's size
/// (`TOGetImageWH` into the frame); grey colours. The record goes to
/// `graphicDrawUIElement(record, name, flag, 0, 0)` with the name
/// `TOGetShapTextureName(0x3e00)`; returns its `eax`.
///
/// `frame` is the original's `ebp - 0xb8` (45 words): `+0` the image
/// height, `+4` its width, `+8` the 64-bit temporary, `+0xc` the name,
/// `+0x10` the flag / bob, `+0x14` the layout or a float temporary,
/// `+0x18` the integer temporary, `+0x1c` the record (0x98 bytes; `+0x80`
/// 0, `+0x88` the layout word, `+0x90` and `+0x94` 0).
///
/// Junk reads (brief 2026-10-24): for a kind other than 0, 4 and 5 the
/// original's layout number and record word `+0x88` are a never-written
/// stack word (`[ebp-0xac]`); the port uses 0. A layout of fewer than two
/// entries leaves the record's edges, UVs and colours never written, and
/// the record's words `+0x40..+0x7c`, `+0x84` and `+0x8c` are never written
/// at all; the port's frame (zeroed by the shim) holds 0 there.
#[allow(clippy::too_many_lines)]
pub fn shape_cursor(h: &mut impl Host, frame: u32, kind: u32, x: u32, y: u32) -> u32 {
    let ebp = frame + 0xb8;
    h.write(ebp - 0xa8, 1u32);
    h.write(ebp - 0xc, 0u32);
    h.write(ebp - 8, 0u32);
    if kind == 0 {
        let time = h.call_sym_st0_f64(GET_TIME, Conv::Cdecl, &[]);
        let angle = mul(time, konst64(h, 0x00b9_22a8));
        let cos = h.rebase(CI_COS);
        let c = h.call_intrinsic(cos, angle).to_bits();
        h.write(ebp - 0xa4, c);
        let bob = store(mul(load(c), konst64(h, 0x00b2_c5c8)));
        h.write(ebp - 0xa8, bob);
        let uv = [
            konst(h, 0x00b5_1b9c),
            konst(h, 0x00b6_6c08),
            konst(h, 0x00b6_6b74),
            konst(h, 0x00b6_6c0c),
        ];
        let arg = konst(h, 0x00b5_ee90);
        let hh = store(h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[arg]));
        let arg = konst(h, 0x00b5_eec0);
        let w = store(h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[arg]));
        let yf = int_f32(y.cast_signed());
        h.write(ebp - 0xa4, yf);
        let y64 = load(yf);
        h.write(ebp - 0xb0, y64);
        let arg = konst(h, 0x00b3_eb98);
        let r = h.call_sym_st0_f64(REMAP_Y2, Conv::Cdecl, &[arg]);
        let y1 = store(sub(y64, r));
        let xf = int_f32(x.cast_signed());
        let sum = add(load(xf), load(bob));
        h.write(ebp - 0xb0, sum);
        let arg = konst(h, 0x00b5_eec0);
        let r = h.call_sym_st0_f64(REMAP_X2, Conv::Cdecl, &[arg]);
        let x1 = store(sub(sum, r));
        h.write(ebp - 0xa4, x1);
        return h.call_sym(
            SHAPE_XYWHUV,
            Conv::Cdecl,
            &[0x190, x1, y1, w, hh, uv[0], uv[1], uv[2], uv[3]],
        );
    }
    let mut x = x.cast_signed();
    let (layout_id, word) = match kind {
        4 => {
            h.write(ebp - 0xa8, 0u32);
            x = x.wrapping_sub(0x1e);
            (8, 0)
        }
        5 => {
            h.write(ebp - 0xa8, 0u32);
            x = x.wrapping_add(0x1e);
            (9, 0)
        }
        // junk read: [ebp-0xac] never written; 0 (brief 2026-10-24)
        _ => (0, 0),
    };
    let layout = h.call_sym(COPY_LAYOUT, Conv::Cdecl, &[layout_id]);
    h.write(ebp - 0xa4, layout);
    h.write(ebp - 0x1c, 0u32);
    h.write(ebp - 0x14, word);
    let name = h.call_sym(SHAPE_TEXTURE_NAME, Conv::Cdecl, &[0x3e00]);
    h.write(ebp - 0xac, name);
    h.call_sym(IMAGE_WH, Conv::Cdecl, &[0x3e00, ebp - 0xb4, ebp - 0xb8]);
    let y = y.cast_signed();
    let mut i = 0u32;
    while i < h.read::<u32>(layout) {
        if i != 0 {
            let e = layout + 4 + 12 * i;
            for k in [0x8c, 0x88, 0x84, 0x80, 0x6c, 0x68, 0x64, 0x60] {
                h.write(ebp - k, 0x80u32);
            }
            let select = SELECT_CURSOR.get(h) != 0;
            let dx = i32::from(h.read::<i8>(e + 2));
            let dy = i32::from(h.read::<i8>(e + 3));
            let w = i32::from(h.read::<u16>(e + 4));
            let ht = i32::from(h.read::<u16>(e + 6));
            let (a, b) = if select {
                (w * 8 / 20, w * 18 / 20)
            } else {
                (w * 3 / 20, w * 51 / 80)
            };
            let edge = |q: i32, base: i32, d: i32| int_f32(d.wrapping_add(q.wrapping_add(base)));
            h.write(ebp - 0x9c, edge(a, x, dx));
            h.write(ebp - 0x7c, edge(b, x, dx));
            h.write(ebp - 0x98, edge(ht * 3 / 20, y, dy));
            h.write(ebp - 0x78, edge(ht * 17 / 20, y, dy));
            let u = i32::from(h.read::<u8>(e));
            let v = i32::from(h.read::<u8>(e + 1));
            let iw = load(h.read(ebp - 0xb4));
            h.write(ebp - 0x94, store(div(load(int_f32(u + 1)), iw)));
            let ih = load(h.read(ebp - 0xb8));
            h.write(ebp - 0x90, store(div(load(int_f32(v + 1)), ih)));
            let u = i32::from(h.read::<u8>(e));
            let iw = load(h.read(ebp - 0xb4));
            h.write(ebp - 0x74, store(div(load(int_f32(w + u - 1)), iw)));
            let v = i32::from(h.read::<u8>(e + 1));
            let ht = i32::from(h.read::<u16>(e + 6));
            let last = int_f32(v - 1 + ht);
            h.write(ebp - 0xa0, last);
            let ih = load(h.read(ebp - 0xb8));
            h.write(ebp - 0x70, store(div(load(last), ih)));
        }
        i += 1;
    }
    let flag: u32 = h.read(ebp - 0xa8);
    h.call_sym(
        DRAW_UI_ELEMENT,
        Conv::Cdecl,
        &[ebp - 0x9c, name, flag, 0, 0],
    )
}

// ---- yiAssert

/// `dbgPrintf(format, ...)` (cdecl).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `sceGsSyncV(0)` (cdecl).
pub const GS_SYNC_V: u32 = 0x0062_f890;
/// `Sg_GetDBuffDC(&flags)` (cdecl).
pub const GET_DBUFF_DC: u32 = 0x0082_0530;
/// `debug_exit_trace(code)` (cdecl).
pub const EXIT_TRACE: u32 = 0x0062_f510;
/// `sceGsSyncPath(0, 0)` (cdecl).
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
/// `sceGsResetPath()` (cdecl).
pub const GS_RESET_PATH: u32 = 0x0062_f7d0;
/// `FlushCache(0)` (cdecl).
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `sceDmaGetChan(n, addr, size)` (cdecl).
pub const DMA_GET_CHAN: u32 = 0x0062_f6e0;
/// `sceDmaSync(chan, ...)` (cdecl).
pub const DMA_SYNC: u32 = 0x0062_f730;
/// `sceDmaSendN(chan, addr, size)` (cdecl).
pub const DMA_SEND_N: u32 = 0x0062_f720;
/// `yiPrintGs_0090aaa0(text, x, y, cols, rows, center, &colour)` (cdecl).
pub const PRINT_GS: u32 = 0x0090_aaa0;
/// The CRT's `vprintf` import slot.
pub const VPRINTF: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c48c);
/// The CRT's `vsprintf` import slot.
pub const VSPRINTF: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c430);
/// The draw context `Sg_GetDBuffDC` gave (`0x019350e8`).
pub const ASSERT_DC: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x0193_50e8);
/// The assert screen's GS packet (`0x019350f0`, 0x11 quadwords).
pub const ASSERT_PACKET: u32 = 0x0193_50f0;

/// `x << 20 | y << 4` of a GS `XYZ` word (the 12.4 fixed-point corner).
fn gs_xy(row: u32, col: u32) -> u32 {
    (row << 4) << 16 | col << 4
}

/// `yiAssert(abort, part, format, ...)` (cdecl, varargs): the PS2
/// debug assert. Prints the banner, the part and the message (`vprintf`
/// with the arguments after `format`, `va` the address of the fourth
/// argument word) through `dbgPrintf`, waits 120 `sceGsSyncV(0)`, takes the
/// draw context (`Sg_GetDBuffDC` into the frame, kept at `0x019350e8`).
/// Without one it returns, or for `abort` prints "process abort." and
/// calls `debug_exit_trace(1)` and goes on. Then it draws a grey box
/// with a dark frame centred on the display (the GS packet at
/// `0x019350f0`, from the context's `+0x10` or `+0x38` frame word and
/// `+0x1c`), sends it (`FlushCache`, DMA channel 2), prints `"*ASSERT*"`,
/// the part (cut to 46 characters in place) and the formatted message
/// (`vsprintf` into the frame) with `yiPrintGs_0090aaa0`, and waits 120
/// syncs again; without `abort` it then calls `debug_exit_trace(1)`.
/// Returns the last call's `eax`.
///
/// `frame` is the original's `ebp - 0x101c` (0x1018 bytes): `+0` the
/// context's flags, `+4` the junk word, `+8` the part, `+0xc` the text x,
/// `+0x10` and `+0x14` the two colours, `+0x18` the message buffer.
///
/// Junk read: the frame word `[ebp-0x1018]` is never written; the
/// original keeps its top 12 bits under the context's 20 (`x ^ (x ^ v) &
/// 0xfffff`), and every use masks them off again. The port uses 0 (brief
/// 2026-10-24); nothing it writes depends on it.
#[allow(clippy::too_many_lines)]
pub fn yi_assert(
    h: &mut impl Host,
    frame: u32,
    abort: u32,
    part: u32,
    format: u32,
    va: u32,
) -> u32 {
    let ebp = frame + 0x101c;
    h.write(ebp - 0x1014, part);
    h.write(ebp - 0x1008, 0xff00_0000u32);
    h.write(ebp - 0x100c, 0xff20_2020u32);
    let banner = h.rebase(0x00b6_6fdc);
    let part_fmt = h.rebase(0x00b6_6ff4);
    let message = h.rebase(0x00b6_7000);
    let end = h.rebase(0x00b6_7044);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[banner]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[part_fmt, part]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[message]);
    let todo = h.rebase(0x00b6_7014);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[todo]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[end]);
    for _ in 0..0x78 {
        h.call_sym(GS_SYNC_V, Conv::Cdecl, &[0]);
    }
    let dc = h.call_sym(GET_DBUFF_DC, Conv::Cdecl, &[ebp - 0x101c]);
    ASSERT_DC.set(h, dc);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[banner]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[part_fmt, part]);
    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[message]);
    let vprintf = VPRINTF.get(h);
    h.call(vprintf, Conv::Cdecl, &[format, va]);
    let mut eax = h.call_sym(DBG_PRINTF, Conv::Cdecl, &[end]);
    if ASSERT_DC.get(h) == 0 {
        if abort == 0 {
            return eax;
        }
        let abort_text = h.rebase(0x00b6_705c);
        h.call_sym(DBG_PRINTF, Conv::Cdecl, &[abort_text]);
        h.call_sym(EXIT_TRACE, Conv::Cdecl, &[1]);
    }
    h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
    h.call_sym(GS_RESET_PATH, Conv::Cdecl, &[]);
    let flags: u8 = h.read(ebp - 0x101c);
    let dc = ASSERT_DC.get(h);
    // junk read: [ebp-0x1018] never written; 0 (brief 2026-10-24)
    let junk = 0u32;
    let p = h.rebase(ASSERT_PACKET);
    h.write(p, 0x8000u32);
    h.write(p + 4, 0x1003_0000u32);
    h.write(p + 8, 0xeu32);
    h.write(p + 0xc, 0u32);
    let fb: u32 = h.read(dc + if flags & 1 != 0 { 0x38 } else { 0x10 });
    let disp: u32 = h.read(dc + 0x1c);
    let v = junk ^ ((fb ^ junk) & 0xfffff);
    let w = (v >> 9) & 0x3f;
    let x0 = (w << 6).wrapping_sub(0x180) >> 1;
    h.write(ebp - 0x1010, x0);
    let row = ((disp >> 12) & 0x7fe).cast_signed() >> 1;
    let row = row.wrapping_sub(0xc0).cast_unsigned();
    let col = w << 5;
    h.write(p + 0x10, ((v >> 15) & 0x1f) << 24 | w << 16 | (v & 0x1ff));
    let at = |k: u32| p + k - 0xf0;
    let words: [(u32, u32); 50] = [
        (0x104, 0),
        (0x150, gs_xy(row.wrapping_sub(1), col.wrapping_sub(0xc1))),
        (
            0x160,
            gs_xy(row.wrapping_add(0x181), col.wrapping_add(0xc1)),
        ),
        (0x108, 0x4c),
        (0x10c, 0),
        (0x110, 0),
        (0x114, 0),
        (0x118, 0x18),
        (0x11c, 0),
        (0x120, 0x2_0000),
        (0x124, 0),
        (0x128, 0x47),
        (0x12c, 0),
        (0x130, 6),
        (0x134, 0),
        (0x138, 0),
        (0x13c, 0),
        (0x140, 0xff70_7070),
        (0x144, 0),
        (0x148, 1),
        (0x14c, 0),
        (0x154, 0),
        (0x158, 0xd),
        (0x15c, 0),
        (0x164, 0),
        (0x168, 5),
        (0x16c, 0),
        (0x170, 0xff20_2020),
        (0x174, 0),
        (0x178, 1),
        (0x17c, 0),
        (0x180, gs_xy(row, col.wrapping_sub(0xc0))),
        (
            0x190,
            gs_xy(row.wrapping_add(0x180), col.wrapping_add(0xc0)),
        ),
        (0x1b0, gs_xy(row.wrapping_add(0x20), col.wrapping_sub(0xb8))),
        (0x1c0, gs_xy(row.wrapping_add(0x30), col.wrapping_add(0xb8))),
        (0x1e0, gs_xy(row.wrapping_add(0x40), col.wrapping_sub(0xb8))),
        (0x184, 0),
        (0x188, 0xd),
        (0x18c, 0),
        (0x194, 0),
        (0x198, 5),
        (0x19c, 0),
        (0x1a0, 0xff00_0000),
        (0x1a4, 0),
        (0x1a8, 1),
        (0x1ac, 0),
        (0x1b4, 0),
        (0x1b8, 0xd),
        (0x1bc, 0),
        (0x1c4, 0),
    ];
    for (k, v) in words {
        h.write(at(k), v);
    }
    let words: [(u32, u32); 14] = [
        (0x1c8, 5),
        (0x1cc, 0),
        (0x1d0, 0xff00_0000),
        (0x1d4, 0),
        (0x1d8, 1),
        (0x1dc, 0),
        (0x1e4, 0),
        (0x1e8, 0xd),
        (0x1ec, 0),
        (
            0x1f0,
            gs_xy(row.wrapping_add(0x170), col.wrapping_add(0xb8)),
        ),
        (0x1f4, 0),
        (0x1f8, 5),
        (0x1fc, 0),
        (0xf0, 0x8010),
    ];
    for (k, v) in words {
        h.write(at(k), v);
    }
    h.call_sym(FLUSH_CACHE, Conv::Cdecl, &[0]);
    let chan = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2, 0, 0x100]);
    h.call_sym(DMA_SYNC, Conv::Cdecl, &[chan, 0, 0x100]);
    h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
    let chan = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2, p, 0x11]);
    h.call_sym(DMA_SEND_N, Conv::Cdecl, &[chan, p, 0x11]);
    h.call_sym(GS_SYNC_PATH, Conv::Cdecl, &[0, 0]);
    let chan = h.call_sym(DMA_GET_CHAN, Conv::Cdecl, &[2, 0, 0x100]);
    h.call_sym(DMA_SYNC, Conv::Cdecl, &[chan, 0, 0x100]);
    let star = h.rebase(0x00b6_706c);
    let y0 = row;
    h.call_sym(
        PRINT_GS,
        Conv::Cdecl,
        &[star, col, y0.wrapping_add(8), 0x2e, 1, 1, ebp - 0x100c],
    );
    let part: u32 = h.read(ebp - 0x1014);
    let x0: u32 = h.read(ebp - 0x1010);
    if part != 0 {
        let mut n = 0u32;
        while h.read::<u8>(part + n) != 0 {
            n += 1;
        }
        if n > 0x2e {
            h.write(part + 0x2e, 0u8);
        }
        h.call_sym(
            PRINT_GS,
            Conv::Cdecl,
            &[
                part,
                x0.wrapping_add(8),
                y0.wrapping_add(0x20),
                0x2e,
                1,
                0,
                ebp - 0x1008,
            ],
        );
    }
    if format != 0 {
        let vsprintf = VSPRINTF.get(h);
        h.call(vsprintf, Conv::Cdecl, &[ebp - 0x1004, format, va]);
        h.call_sym(
            PRINT_GS,
            Conv::Cdecl,
            &[
                ebp - 0x1004,
                x0.wrapping_add(8),
                y0.wrapping_add(0x40),
                0x2e,
                0x13,
                0,
                ebp - 0x1008,
            ],
        );
    }
    for _ in 0..0x78 {
        eax = h.call_sym(GS_SYNC_V, Conv::Cdecl, &[0]);
    }
    if abort == 0 {
        eax = h.call_sym(EXIT_TRACE, Conv::Cdecl, &[1]);
    }
    eax
}

// ---- GDraw's three-input filter pass

/// `0x00a3c1b0(dst, texture)` (cdecl): a texture's transform into a
/// constant block.
pub const TEXTURE_TRANSFORM: u32 = 0x00a3_c1b0;

/// Binds pixel shader input `slot`: `PSSetShaderResources(slot, 1, ...)`
/// with the input's view (`+4`) and the sampler `state + 0x7b4`, or a
/// null view from the zero word at `null` (context vtable `+0x20`,
/// `+0x28`; the context read from the state each time).
fn bind_input(h: &mut impl Host, slot: u32, input: u32, null: u32) {
    let ctx: u32 = h.read(GDRAW_STATE.get(h) + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x20);
    if input == 0 {
        h.write(null, 0u32);
        h.call(f, Conv::Stdcall, &[ctx, slot, 1, null]);
    } else {
        h.call(f, Conv::Stdcall, &[ctx, slot, 1, input + 4]);
        let sampler = GDRAW_STATE.get(h) + 0x7b4;
        let f: u32 = h.read(h.read::<u32>(ctx) + 0x28);
        h.call(f, Conv::Stdcall, &[ctx, slot, 1, sampler]);
    }
}

/// `gdraw_filterPass3_00a38c40(filter, rect, colour, kind, stats)`
/// (cdecl): `gdraw_filterPass_00a374e0` with three inputs
/// (`filter + 0x1c`, `+0x20`, `+0x24` in slots 0..2; a null one from the
/// frame words `ebp - 0x14`, `- 0x10`, `- 0xc`), the pixel shader
/// `state + 0x104 + (filter.b[0xf] + kind * 16) * 8`, and the constants:
/// the first two inputs' texture transforms (`0x00a3c1b0` into `+0`,
/// `+0x10` of the mapped `state + 0x98c`), the filter's `+0x88..+0x94`
/// and `+0xa8..+0xb4` (`+0x20`, `+0x30`), `-filter[0x98] / w`,
/// `-filter[0x9c] / h` (the screen size as `f32`), `filter[0xa0]` and 0
/// (`+0x40`); then
/// the quad and the pop as there, all three inputs getting the target
/// size. Returns the last `eax`.
///
/// `frame` is the original's `ebp - 0x2c` (11 words): the rectangle at
/// `+0`, the buffer word at `+0x10`, the float temporary at `+0x14`, the
/// target and the null views at `+0x18..+0x20`.
#[allow(clippy::too_many_lines)]
pub fn filter_pass3(
    h: &mut impl Host,
    frame: u32,
    filter: u32,
    rect: u32,
    colour: u32,
    kind: u32,
    stats: u32,
) -> u32 {
    let ebp = frame + 0x2c;
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
    for (slot, off, null) in [(0, 0x1c, 0x14), (1, 0x20, 0x10), (2, 0x24, 0xc)] {
        let input: u32 = h.read(filter + off);
        bind_input(h, slot, input, ebp - null);
    }
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x24);
    let k = u32::from(h.read::<u8>(filter + 0xf)).wrapping_add(kind.wrapping_shl(4));
    let shader: u32 = h.read(state.wrapping_add(k.wrapping_mul(8)).wrapping_add(0x104));
    h.call(f, Conv::Stdcall, &[ctx, shader, 0, 0]);
    let state = GDRAW_STATE.get(h);
    let buffer: u32 = h.read(state + 0x98c);
    let dst = h.call_sym(MAP_BUFFER, Conv::Cdecl, &[buffer]);
    let t0: u32 = h.read(filter + 0x1c);
    h.call_sym(TEXTURE_TRANSFORM, Conv::Cdecl, &[dst, t0]);
    let t1: u32 = h.read(filter + 0x20);
    h.call_sym(TEXTURE_TRANSFORM, Conv::Cdecl, &[dst + 0x10, t1]);
    let state = GDRAW_STATE.get(h);
    for (src, to) in [(0x88, 0x20), (0xa8, 0x30)] {
        let v: [u32; 4] = [0, 4, 8, 0xc].map(|k| pass(h.read(filter + src + k)));
        for (k, w) in v.iter().enumerate() {
            h.write(dst + to + 4 * k as u32, *w);
        }
    }
    let hh = int_f32(h.read(state + 0x6c4));
    let y = store(div(-load(h.read(filter + 0x9c)), load(hh)));
    let z = pass(h.read(filter + 0xa0));
    let state = GDRAW_STATE.get(h);
    let w = int_f32(h.read(state + 0x6c0));
    let x = store(div(-load(h.read(filter + 0x98)), load(w)));
    h.write(ebp - 0x18, w);
    h.write(ebp - 8, y);
    h.write(ebp - 4, z);
    for (k, v) in [x, y, z, 0].iter().enumerate() {
        h.write(dst + 0x40 + 4 * k as u32, *v);
    }
    let state = GDRAW_STATE.get(h);
    let buffer: u32 = h.read(state + 0x98c);
    let ctx: u32 = h.read(state + 4);
    h.write(ebp - 0x1c, buffer);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x3c);
    h.call(f, Conv::Stdcall, &[ctx, buffer, 0]);
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x40);
    h.call(f, Conv::Stdcall, &[ctx, 1, 1, ebp - 0x1c]);
    let r = |k: u32| int_f32(h.read::<i32>(rect + 4 * k));
    let (y1, x1, y0, x0) = (r(3), r(2), r(1), r(0));
    h.write(ebp - 0x18, x0);
    h.call_sym(DRAW_QUAD, Conv::Cdecl, &[colour, stats, x0, y0, x1, y1]);
    let inputs: [u32; 3] = [0x1c, 0x20, 0x24].map(|k| h.read(filter + k));
    let state = GDRAW_STATE.get(h);
    let (w, hh): (u32, u32) = (h.read(state + 0xac0), h.read(state + 0xac4));
    h.write(ebp - 8, inputs[0]);
    h.write(ebp - 4, hh);
    for input in inputs {
        if input != 0 {
            h.write(input + 0x28, w);
            h.write(input + 0x2c, hh);
        }
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

// ---- GDraw's render state

/// The D3D "out of memory" result (`E_OUTOFMEMORY`).
const OUT_OF_MEMORY: u32 = 0x8007_000e;

/// `Map(resource, 0, WRITE_DISCARD, 0, out)` (context vtable `+0x38`) of
/// the buffer at state `+buffer`: the data, or `None` for a null one or
/// after the warning for a failure (`"GDraw D3D out of memory in %s%s"` or
/// `"GDraw D3D error in %s%s: 0x%08x"`, "of buffer").
fn map_buffer(h: &mut impl Host, buffer: u32, out: u32) -> Option<u32> {
    let state = GDRAW_STATE.get(h);
    let ctx: u32 = h.read(state + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x38);
    let res: u32 = h.read(state + buffer);
    let hr = h.call(f, Conv::Stdcall, &[ctx, res, 0, 4, 0, out]);
    if hr.cast_signed() < 0 {
        let warn = GDRAW_WARNING.get(h);
        let (what, of) = (h.rebase(0x00b6_868c), h.rebase(0x00b7_a408));
        if hr == OUT_OF_MEMORY {
            let text = h.rebase(0x00b7_a4ec);
            h.call(warn, Conv::Cdecl, &[0, text, what, of]);
        } else {
            let text = h.rebase(0x00b7_a50c);
            h.call(warn, Conv::Cdecl, &[0, text, what, of, hr]);
        }
        return None;
    }
    let data: u32 = h.read(out);
    (data != 0).then_some(data)
}

/// `gdraw_setRenderState_00a3c6c0(format, r, _, colour)` (cdecl):
/// `GDraw`'s draw state for render state `r`. The vertex shader of
/// `format` (`state + 0x274 + format * 8`, context vtable `+0x2c`); the
/// vertex constants (mapped `state + 0x984`): the transform from `r +
/// 0x2c` (or with flag 4 of `r + 4` the state's `+0x7a4`/`+0x7a8` scale)
/// with the depth `1 - (r.depth + 1) * 0x00b7a960`, `r + 0x28`'s four
/// words, with `r.b[8]` eight words from `r + 0x50`, and the twelve
/// projection words (`+0x74c` when `+0x77c`, else `+0x71c`); unmapped
/// and bound (`VSSetConstantBuffers`). The blend state `r.b[0xc]` when
/// it changed (`+0x9c8`, `OMSetBlendState(+0x7d4 + 4 * b, colour, ~0)`);
/// the pixel shader: blend 5 uses `+0x9c + 8 * r.b[0xd]`, else
/// `+0xc + 8 * (n + 3 * r.b[9])` (n 0, 1 or 2 by the colour transform
/// `r + 0x18` and its `+6`), none for a stencil-only draw (`r.b[0x11]`
/// with kind other than 5) and a reset blend state for kind 5. The
/// texture `r + 0x1c` (its view and the sampler
/// `+0x7b4 + 4 * (r.b[0xa] + 4 * r.b[0xb])`, or a null view), the pixel
/// constants (mapped `+0x988`):
/// `r + 0x30`, the colour transform's `i16`s / `0x00b170c0` (or zeros),
/// with kind 4 `r + 0x70`, with blend 5 `colour`'s four words. With flag
/// 8 the scissor (`r + 0x40`, less the origin `+0x6f0` or, on the
/// screen's own target, moved by `+0x6c8 - +0x6e8`), else a pending
/// full-screen scissor reset (`+0x9c4`). Last the depth-stencil state:
/// `gdraw_getDepthStencilState(flags >> 1 & 1, flags & 1, r.b[0x10],
/// r.b[0x11])` with reference 0xff when either byte is set, else the
/// cached `+0x7f0` one. Returns 1.
///
/// `frame` is the original's `ebp - 0x64` (24 words: the two mapped
/// results, the temporaries, the null view, both scissors).
///
/// The original passes `r.b[0x10]` and `r.b[0x11]` with stale upper bytes
/// (registers' leftovers); `gdraw_getDepthStencilState` reads the bytes
/// only, and the port passes them zero-extended.
#[allow(clippy::too_many_lines)]
pub fn set_render_state(h: &mut impl Host, frame: u32, format: u32, r: u32, colour: u32) -> u32 {
    let ebp = frame + 0x64;
    let state = GDRAW_STATE.get(h);
    let ctx0: u32 = h.read(state + 4);
    h.write(ebp - 0x3c, ctx0);
    let f: u32 = h.read(h.read::<u32>(ctx0) + 0x2c);
    let shader: u32 = h.read(
        state
            .wrapping_add(format.wrapping_mul(8))
            .wrapping_add(0x274),
    );
    h.call(f, Conv::Stdcall, &[ctx0, shader, 0, 0]);
    if let Some(dst) = map_buffer(h, 0x984, ebp - 0x58) {
        let base = GDRAW_STATE.get(h);
        h.write(ebp - 0x28, base);
        let v = load(int_f32(h.read(r)));
        let depth = store(sub(1.0, mul(add(v, 1.0), konst64(h, 0x00b7_a960))));
        h.write(ebp - 0x30, depth);
        let words = if h.read::<u8>(r + 4) & 4 == 0 {
            let m: u32 = h.read(r + 0x2c);
            let m: [u32; 6] = [0, 4, 8, 0xc, 0x10, 0x14].map(|k| pass(h.read(m + k)));
            [m[0], m[1], depth, m[4], m[2], m[3], 0, m[5]]
        } else {
            let (a, b) = (pass(h.read(base + 0x7a4)), pass(h.read(base + 0x7a8)));
            [a, 0, depth, 0, 0, b, 0, 0]
        };
        for (k, w) in words.iter().enumerate() {
            h.write(dst + 4 * k as u32, *w);
        }
        let src: u32 = h.read(r + 0x28);
        for k in 0..4u32 {
            let w: u32 = h.read(src + 4 * k);
            h.write(dst + 0x20 + 4 * k, w);
        }
        if h.read::<u8>(r + 8) != 0 {
            for k in 0..8u32 {
                let w: u32 = h.read(r + 0x50 + 4 * k);
                h.write(dst + 0x30 + 4 * k, w);
            }
        }
        let proj = if h.read::<u32>(base + 0x77c) == 0 {
            base + 0x71c
        } else {
            base + 0x74c
        };
        for k in 0..12u32 {
            let w: u32 = h.read(proj + 4 * k);
            h.write(dst + 0x50 + 4 * k, w);
        }
        let ctx: u32 = h.read(base + 4);
        let f: u32 = h.read(h.read::<u32>(ctx) + 0x3c);
        let res: u32 = h.read(base + 0x984);
        h.call(f, Conv::Stdcall, &[ctx, res, 0]);
        let slot = GDRAW_STATE.get(h) + 0x984;
        let f: u32 = h.read(h.read::<u32>(ctx0) + 0x1c);
        h.call(f, Conv::Stdcall, &[ctx0, 0, 1, slot]);
    }
    let blend = u32::from(h.read::<u8>(r + 0xc));
    let mut state = GDRAW_STATE.get(h);
    let quad_colour = h.rebase(crate::h4b_b::QUAD_COLOUR);
    if blend != h.read::<u32>(state + 0x9c8) {
        h.write(state + 0x9c8, blend);
        let f: u32 = h.read(h.read::<u32>(ctx0) + 0x8c);
        let b: u32 = h.read(state + 0x7d4 + 4 * blend);
        h.call(f, Conv::Stdcall, &[ctx0, b, quad_colour, 0xffff_ffff]);
        state = GDRAW_STATE.get(h);
    }
    if blend == 5 {
        let k = u32::from(h.read::<u8>(r + 0xd));
        let s: u32 = h.read(state + 0x9c + 8 * k);
        set_ps(h, ctx0, s);
    } else {
        let cx: u32 = h.read(r + 0x18);
        let kind = u32::from(h.read::<u8>(r + 9));
        let n = if cx == 0 {
            0
        } else if h.read::<u16>(cx + 6) == 0 {
            1
        } else {
            2
        };
        let s: u32 = h.read(state.wrapping_add((n + 3 * kind) * 8).wrapping_add(0xc));
        if h.read::<u8>(r + 0x11) != 0 {
            if kind == 5 {
                let b: u32 = h.read(state + 0x7ec);
                h.write(state + 0x9c8, 0xffff_ffffu32);
                let f: u32 = h.read(h.read::<u32>(ctx0) + 0x8c);
                h.call(f, Conv::Stdcall, &[ctx0, b, quad_colour, 0xffff_ffff]);
                set_ps(h, ctx0, s);
            } else {
                set_ps(h, ctx0, 0);
            }
        } else {
            set_ps(h, ctx0, s);
        }
    }
    let a = u32::from(h.read::<u8>(r + 0xa));
    let tex: u32 = h.read(r + 0x1c);
    h.write(ebp - 0x28, a);
    let b = u32::from(h.read::<u8>(r + 0xb));
    h.write(ebp - 0x34, b);
    let ctx: u32 = h.read(GDRAW_STATE.get(h) + 4);
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x20);
    if tex == 0 {
        h.write(ebp - 0x38, 0u32);
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, ebp - 0x38]);
    } else {
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, tex + 4]);
        let sampler = GDRAW_STATE
            .get(h)
            .wrapping_add(a.wrapping_add(b.wrapping_mul(4)).wrapping_mul(4))
            .wrapping_add(0x7b4);
        let f: u32 = h.read(h.read::<u32>(ctx) + 0x28);
        h.call(f, Conv::Stdcall, &[ctx, 0, 1, sampler]);
    }
    if let Some(dst) = map_buffer(h, 0x988, ebp - 0x64) {
        for k in 0..4u32 {
            let w: u32 = h.read(r + 0x30 + 4 * k);
            h.write(dst + 4 * k, w);
        }
        let cx: u32 = h.read(r + 0x18);
        if cx == 0 {
            for k in 0..4u32 {
                h.write(dst + 0x10 + 4 * k, 0u32);
            }
        } else {
            let d = konst64(h, 0x00b1_70c0);
            for k in 0..4u32 {
                let cx: u32 = h.read(r + 0x18);
                let v = f64::from(h.read::<i16>(cx + 2 * k));
                h.write(ebp - 0x28, i32::from(h.read::<i16>(cx + 2 * k)));
                h.write(ebp - 0x2c, v);
                h.write(dst + 0x10 + 4 * k, store(div(v, d)));
            }
        }
        if h.read::<u8>(r + 9) == 4 {
            for k in 0..4u32 {
                let w: u32 = h.read(r + 0x70 + 4 * k);
                h.write(dst + 0x20 + 4 * k, w);
            }
        }
        if h.read::<u8>(r + 0xc) == 5 {
            for k in 0..4u32 {
                let w: u32 = h.read(colour + 4 * k);
                h.write(dst + 0x30 + 4 * k, w);
            }
        }
        let state = GDRAW_STATE.get(h);
        let ctx: u32 = h.read(state + 4);
        let f: u32 = h.read(h.read::<u32>(ctx) + 0x3c);
        let res: u32 = h.read(state + 0x988);
        h.call(f, Conv::Stdcall, &[ctx, res, 0]);
        let slot = GDRAW_STATE.get(h) + 0x988;
        let f: u32 = h.read(h.read::<u32>(ctx0) + 0x40);
        h.call(f, Conv::Stdcall, &[ctx0, 0, 1, slot]);
    }
    let flags: u32 = h.read(r + 4);
    let mut state = GDRAW_STATE.get(h);
    if flags & 8 != 0 {
        let screen = h.read::<u32>(state + 0xaac) == state + 0x9cc;
        h.write(state + 0x9c4, 1u32);
        let p = |k: u32| h.read::<u32>(r + 0x40 + 4 * k);
        let rect = if screen {
            let (ox, sx, oy) = (
                h.read::<u32>(state + 0x6e8),
                h.read::<u32>(state + 0x6c8),
                h.read::<u32>(state + 0x6ec),
            );
            let sy: u32 = h.read(GDRAW_STATE.get(h) + 0x6cc);
            [
                p(0).wrapping_sub(ox).wrapping_add(sx),
                p(1).wrapping_sub(oy).wrapping_add(sy),
                p(2).wrapping_sub(ox).wrapping_add(sx),
                p(3).wrapping_sub(oy).wrapping_add(sy),
            ]
        } else {
            let (ox, oy) = (h.read::<u32>(state + 0x6f0), h.read::<u32>(state + 0x6f4));
            [
                p(0).wrapping_sub(ox),
                p(1).wrapping_sub(oy),
                p(2).wrapping_sub(ox),
                p(3).wrapping_sub(oy),
            ]
        };
        for (k, v) in rect.iter().enumerate() {
            h.write(ebp - 0x14 + 4 * k as u32, *v);
        }
        let f: u32 = h.read(h.read::<u32>(ctx0) + 0xb4);
        h.call(f, Conv::Stdcall, &[ctx0, 1, ebp - 0x14]);
        state = GDRAW_STATE.get(h);
    } else if h.read::<u32>(state + 0x9c4) != 0 {
        let (x, y, w, hh): (u32, u32, u32, u32) = (
            h.read(state + 0x6fc),
            h.read(state + 0x700),
            h.read(state + 0x704),
            h.read(state + 0x708),
        );
        let rect = [x, y, w.wrapping_add(x), hh.wrapping_add(y)];
        for (k, v) in rect.iter().enumerate() {
            h.write(ebp - 0x24 + 4 * k as u32, *v);
        }
        let ctx: u32 = h.read(state + 4);
        let f: u32 = h.read(h.read::<u32>(ctx) + 0xb4);
        h.call(f, Conv::Stdcall, &[ctx, 1, ebp - 0x24]);
        state = GDRAW_STATE.get(h);
        h.write(state + 0x9c4, 0u32);
    }
    let read_mask = u32::from(h.read::<u8>(r + 0x10));
    let write_mask = u32::from(h.read::<u8>(r + 0x11));
    let flags: u32 = h.read(r + 4);
    let f: u32 = h.read(h.read::<u32>(ctx0) + 0x90);
    if read_mask | write_mask != 0 {
        let ds = h.call_sym(
            DEPTH_STENCIL_STATE,
            Conv::Cdecl,
            &[(flags >> 1) & 1, flags & 1, read_mask, write_mask],
        );
        h.call(f, Conv::Stdcall, &[ctx0, ds, 0xff]);
    } else {
        let k = (flags & 1) + ((flags >> 1) & 1) * 2;
        let ds: u32 = h.read(state + 0x7f0 + 4 * k);
        h.call(f, Conv::Stdcall, &[ctx0, ds, 0]);
    }
    1
}

/// `gdraw_getDepthStencilState_00a3d020(depth, write, read_mask,
/// write_mask)` (cdecl).
pub const DEPTH_STENCIL_STATE: u32 = 0x00a3_d020;

/// `PSSetShader(shader, 0, 0)` (context vtable `+0x24`).
fn set_ps(h: &mut impl Host, ctx: u32, shader: u32) {
    let f: u32 = h.read(h.read::<u32>(ctx) + 0x24);
    h.call(f, Conv::Stdcall, &[ctx, shader, 0, 0]);
}
