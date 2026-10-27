//! Round h4x of list `h4-glue-other`, batch 2: the shape fields' VFX
//! render data (`updateVFXRenderDataShapeField`, `0x0071a4e0`).
//!
//! A shape record (`+0` flags, `+2` its quad count, `+4` the offset of its
//! colours, `+0x10` its id, quads from `+0x20`) appends its quads to the
//! render data's buffers (the segment at `+0x94`): four vertices through
//! the object's matrix (or `VertexGroupApplyZMatZClip`, which may clip the
//! quad), four colours (the record's RGBA times the object's tint, with
//! `+2` bit 0x400 faded by the first vertex's depth), four uvs and six
//! indices. Records with flag bit 3 hold four corners per quad (0x18
//! bytes); the others two (0x10 bytes). When the buffers were empty, the
//! render data also gets the fill matrix, its depth correction (the first
//! quad's mean z/w on screen), wide bounds and is made visible.
//!
//! x87: every value is computed at 53 bits in the original's order and
//! rounded where the original stores a `dword` (`ffx_bridge::x87`).

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicLevelGetNameHash()` (cdecl).
pub const LEVEL_HASH: u32 = 0x0064_1da0;
/// `g_GraphicSkipFrame` (`0x012fb7d0`).
pub const SKIP_FRAME: u32 = 0x012f_b7d0;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const RENDER_DATA: u32 = 0x0064_53d0;
/// `FFXVu0MulMatrix(dst, a, b)` (cdecl).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `g_ScreenMatrix` (`0x02311480`).
pub const SCREEN_MATRIX: u32 = 0x0231_1480;
/// `graphicDynGeoMemManagerGetCurrentBufferId()` (cdecl).
pub const CURRENT_BUFFER: u32 = 0x0063_ff50;
/// `VertexGroupApplyZMatZClip(vertices, m, second, fill2, fill3, flags)`
/// (cdecl) -> 0 drawn, else clipped.
pub const ZCLIP: u32 = 0x0071_19d0;
/// `g_pppFillMatrix2` (`0x0230fc20`).
pub const FILL_MATRIX2: u32 = 0x0230_fc20;
/// `g_pppFillMatrix3` (`0x0230fbe0`).
pub const FILL_MATRIX3: u32 = 0x0230_fbe0;
/// `g_pppFillMatrix` (`0x0230fca0`).
pub const FILL_MATRIX: u32 = 0x0230_fca0;
/// `g_AlphaScale` (`0x00c3a4d0`, f32).
pub const ALPHA_SCALE: u32 = 0x00c3_a4d0;
/// `g_UseOptimizedVFXShaderArg` (`0x00c3a4dc`): cleared when a colour is
/// above one half.
pub const OPTIMIZED_SHADER: u32 = 0x00c3_a4dc;
/// `g_VfxYiqMode` (`0x00c3a504`, -1 off).
pub const YIQ_MODE: u32 = 0x00c3_a504;
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const MAP_NO: u32 = 0x012f_bc40;
/// `graphicVFXEnableYIQ(lo, hi)` (cdecl).
pub const ENABLE_YIQ: u32 = 0x0064_5350;
/// `graphicVFXSetZCorrectionValue(lo, hi, f32)` (cdecl).
pub const SET_Z_CORRECTION: u32 = 0x0064_5750;
/// `graphicVFXSetVisible(lo, hi, visible)` (cdecl).
pub const SET_VISIBLE: u32 = 0x0064_5710;

/// `f64` constants: 1/16 (`0x00b921e0`), 1/256 (`0x00b50198`), 1/128
/// (`0x00b170b8`), 8 (`0x00b922a8`), 32 (`0x00b412f8`), 1/4096
/// (`0x00b50190`), 0.875 (`0x00b43d58`), 0.125 (`0x00b92200`), 0.25
/// (`0x00b34f90`), 2 (`0x00b92270`).
pub const INV_16: u32 = 0x00b9_21e0;
/// See [`INV_16`].
pub const INV_256: u32 = 0x00b5_0198;
/// See [`INV_16`].
pub const INV_128: u32 = 0x00b1_70b8;
/// See [`INV_16`].
pub const EIGHT: u32 = 0x00b9_22a8;
/// See [`INV_16`].
pub const THIRTY_TWO: u32 = 0x00b4_12f8;
/// See [`INV_16`].
pub const INV_4096: u32 = 0x00b5_0190;
/// See [`INV_16`].
pub const SEVEN_EIGHTHS: u32 = 0x00b4_3d58;
/// See [`INV_16`].
pub const EIGHTH: u32 = 0x00b9_2200;
/// See [`INV_16`].
pub const QUARTER: u32 = 0x00b3_4f90;
/// See [`INV_16`].
pub const TWO: u32 = 0x00b9_2270;
/// `f32` constants: 128 (`0x00b501bc`), 0.5 (`0x00b92194`), 0.125
/// (`0x00b49368`), 0.375 (`0x00b4936c`), 0.625 (`0x00b49370`), 0.25
/// (`0x00b92180`), 0.75 (`0x00b43d18`), -1 (`0x00b92340`), -1e6
/// (`0x00b501d4`), 1e6 (`0x00b2be08`).
pub const F128: u32 = 0x00b5_01bc;
/// See [`F128`].
pub const F_HALF: u32 = 0x00b9_2194;
/// See [`F128`].
pub const F_EIGHTH: u32 = 0x00b4_9368;
/// See [`F128`].
pub const F_THREE_EIGHTHS: u32 = 0x00b4_936c;
/// See [`F128`].
pub const F_FIVE_EIGHTHS: u32 = 0x00b4_9370;
/// See [`F128`].
pub const F_QUARTER: u32 = 0x00b9_2180;
/// See [`F128`].
pub const F_THREE_QUARTERS: u32 = 0x00b4_3d18;
/// See [`F128`].
pub const F_MINUS_ONE: u32 = 0x00b9_2340;
/// See [`F128`].
pub const F_LOW: u32 = 0x00b5_01d4;
/// See [`F128`].
pub const F_HIGH: u32 = 0x00b2_be08;

/// The level whose record `0x32c0` has its uvs remapped (`0x672`).
const UV_LEVEL: u32 = 0x672;
/// The level whose record `0x3980` gets the depth correction -1 (`0x1bbd`).
const DEPTH_LEVEL: u32 = 0x1bbd;

/// Bytes of [`update_shape_field`]'s frame, from the original's `ebp -
/// 0xd4` up to `ebp`: the screen-times-object matrix at `+0` (`ebp -
/// 0xd4`, made when the object's `+2` has bit 0x400), the four-corner
/// quad's vertices at `+0x40` (`ebp - 0x94`), the two-corner quad's at
/// `+0x80` (`ebp - 0x54`), each four vectors (x, y, 0, 1); the fade's
/// vector at `+0xc0` (`ebp - 0x14`).
pub const SHAPE_FRAME: u32 = 0xd4;

/// The quads' layout: the record's flag bit 3.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// Two corners (`+0` x0, `+2` y0, `+4` x1, `+6` y1), uvs at `+8`; 0x10
    /// bytes.
    Corners2,
    /// Four corners (eight i16), uvs at `+0x10`; 0x18 bytes.
    Corners4,
}

impl Layout {
    fn stride(self) -> u32 {
        match self {
            Self::Corners2 => 0x10,
            Self::Corners4 => 0x18,
        }
    }
    /// The vertices' frame offset.
    fn vertices(self) -> u32 {
        match self {
            Self::Corners2 => 0x80,
            Self::Corners4 => 0x40,
        }
    }
    fn uvs(self) -> u32 {
        match self {
            Self::Corners2 => 8,
            Self::Corners4 => 0x10,
        }
    }
}

fn f64_at(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

fn f32_at(h: &impl Host, at: u32) -> f64 {
    load(h.read(at))
}

fn image_f32(h: &impl Host, sym: u32) -> f64 {
    f32_at(h, h.rebase(sym))
}

/// An i16 field as the original's `fild`.
fn short(h: &impl Host, at: u32) -> f64 {
    f64::from(h.read::<i16>(at))
}

/// The x87's `fucom` equality: NaN equals nothing.
#[allow(clippy::float_cmp)]
fn equal(a: f64, b: f64) -> bool {
    a == b
}

/// `updateVFXRenderDataShapeField(lo, hi, shape, obj)` (`0x0071a4e0`,
/// cdecl; `frame` see [`SHAPE_FRAME`]) -> 1, or 0 when the instance has no
/// render data. Nothing on a skipped frame (`g_GraphicSkipFrame`).
///
/// Junk read (brief 2026-10-24): the depth correction averages the first
/// quad's four z/w (`ebp - 0x130` .. `- 0x124`), which the original writes
/// only when the buffers were empty and that quad was drawn; otherwise it
/// reads stale stack words there, and the port uses 0.
pub fn update_shape_field(
    h: &mut impl Host,
    frame: u32,
    lo: u32,
    hi: u32,
    shape: u32,
    obj: u32,
) -> u32 {
    let level = h.call_sym(LEVEL_HASH, Conv::Cdecl, &[]);
    if h.read::<u32>(h.rebase(SKIP_FRAME)) != 0 {
        return 1;
    }
    let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[lo, hi]);
    if rd == 0 {
        return 0;
    }
    if h.read::<u32>(obj + 0x1c) != 0 {
        h.write(rd + 0xb6, 1u8);
    }
    let segment: u32 = h.read(rd + 0x94);
    let first = h.read::<u32>(segment + 8) / 6 == 0;
    if h.read::<u16>(obj + 2) & 0x400 != 0 {
        let m: u32 = h.read(obj + 0x10);
        let screen = h.rebase(SCREEN_MATRIX);
        h.call_sym(MUL_MATRIX, Conv::Cdecl, &[frame, screen, m]);
    }
    let layout = if h.read::<u8>(shape) & 8 == 0 {
        Layout::Corners2
    } else {
        Layout::Corners4
    };
    let depth = quads(h, frame, rd, shape, obj, level, layout);
    if first {
        let fill = h.rebase(FILL_MATRIX);
        for w in 0..16 {
            let v: u32 = h.read(fill + 4 * w);
            h.write(rd + 4 * w, v);
        }
        for off in [0xc, 0x1c, 0x2c] {
            h.write(rd + off, 0u32);
        }
        h.write(rd + 0x3c, 1f32.to_bits());
        let yiq = h.read::<u32>(h.rebase(YIQ_MODE));
        if yiq != u32::MAX && h.read::<u32>(h.rebase(MAP_NO)) == 0x188 {
            h.call_sym(ENABLE_YIQ, Conv::Cdecl, &[lo, hi]);
        }
    }
    h.write(rd + 0x80, 0u32);
    if first {
        let id = h.read::<u32>(shape + 0x10) & 0x3fff;
        let correction = if level == DEPTH_LEVEL && id == 0x3980 {
            store(image_f32(h, F_MINUS_ONE))
        } else {
            let sum = add(
                add(add(load(depth[1]), load(depth[0])), load(depth[2])),
                load(depth[3]),
            );
            store(mul(sum, f64_at(h, QUARTER)))
        };
        h.call_sym(SET_Z_CORRECTION, Conv::Cdecl, &[lo, hi, correction]);
        let low = store(image_f32(h, F_LOW));
        for off in [0x98, 0x9c, 0xa0] {
            h.write(rd + off, low);
        }
        let high = store(image_f32(h, F_HIGH));
        for off in [0xa4, 0xa8, 0xac] {
            h.write(rd + off, high);
        }
        h.call_sym(SET_VISIBLE, Conv::Cdecl, &[lo, hi, 1]);
    }
    1
}

/// The record's quads into the segment's buffers, from its first free
/// quad on. Returns the first quad's four z/w on screen (`f32` bits; 0
/// where the original leaves them unwritten).
#[allow(clippy::too_many_arguments)]
fn quads(
    h: &mut impl Host,
    frame: u32,
    rd: u32,
    shape: u32,
    obj: u32,
    level: u32,
    layout: Layout,
) -> [u32; 4] {
    let mut depth = [0u32; 4];
    let colours = shape.wrapping_add(h.read(shape + 4));
    let segment: u32 = h.read(rd + 0x94);
    let used: u32 = h.read(segment + 8);
    let start = used / 6;
    let count = i32::from(h.read::<i16>(shape + 2));
    h.write(
        segment + 8,
        used.wrapping_add(count.wrapping_mul(6).cast_unsigned()),
    );
    let count = i32::from(h.read::<i16>(shape + 2));
    let v: u32 = h.read(segment + 4);
    h.write(
        segment + 4,
        v.wrapping_add(count.wrapping_mul(4).cast_unsigned()),
    );
    let id = h.call_sym(CURRENT_BUFFER, Conv::Cdecl, &[]);
    let slot = segment.wrapping_add(id.wrapping_mul(0x1c));
    let v: u32 = h.read(segment + 8);
    h.write(slot.wrapping_add(0x30), v);
    let v: u32 = h.read(segment + 4);
    h.write(slot.wrapping_add(0x2c), v);
    let vertices = frame + layout.vertices();
    let mut q = start;
    let mut entry = shape + 0x20;
    let mut colour = colours.wrapping_add(3);
    while q < h.read::<u32>(segment + 8) / 6 {
        corners(h, vertices, entry, layout);
        let m: u32 = h.read(obj + 0x10);
        let second: u32 = h.read(obj + 0x1c);
        let drawn = if second == 0 || second == m {
            h.call_sym(MUL_MATRIX, Conv::Cdecl, &[vertices, m, vertices]);
            true
        } else {
            let flags = i32::from(h.read::<i16>(shape)).cast_unsigned();
            let fill2 = h.rebase(FILL_MATRIX2);
            let fill3 = h.rebase(FILL_MATRIX3);
            let args = [vertices, m, second, fill2, fill3, flags];
            h.call_sym(ZCLIP, Conv::Cdecl, &args) == 0
        };
        let mut indexed = false;
        if drawn {
            // the vertices as transformed: x, y, z of each
            let vbuf: u32 = h.read(segment + 0xc);
            let out = vbuf.wrapping_add(q.wrapping_mul(0x30));
            for n in 0..4u32 {
                for k in 0..3u32 {
                    let x = store(f32_at(h, vertices + 16 * n + 4 * k));
                    h.write(out.wrapping_add(12 * n + 4 * k), x);
                }
            }
            if q == 0 {
                let screen = h.rebase(SCREEN_MATRIX);
                h.call_sym(MUL_MATRIX, Conv::Cdecl, &[vertices, screen, vertices]);
                for (n, d) in depth.iter_mut().enumerate() {
                    let at = vertices + 16 * n as u32;
                    let w = f32_at(h, at + 0xc);
                    *d = if equal(w, 0.0) {
                        0
                    } else {
                        store(div(f32_at(h, at + 8), w))
                    };
                }
            }
            if h.read::<u8>(obj + 7) != 0 && h.read::<u8>(colour) != 0 {
                quad_colour(h, frame, segment, obj, entry, colour, q);
                quad_uvs(h, segment, shape, entry, q, level, layout);
                let ibuf: u32 = h.read(segment + 0x1c);
                let i = ibuf.wrapping_add(q.wrapping_mul(12));
                let base = q.wrapping_mul(4);
                for (k, d) in [0u32, 2, 1, 2, 3, 1].into_iter().enumerate() {
                    h.write(i + 2 * k as u32, base.wrapping_add(d) as u16);
                }
                indexed = true;
            }
        }
        if !indexed {
            let seg: u32 = h.read(rd + 0x94);
            let ibuf: u32 = h.read(seg + 0x1c);
            let i = ibuf.wrapping_add(q.wrapping_mul(12));
            for k in 0..3 {
                h.write(i + 4 * k, 0u32);
            }
        }
        colour = colour.wrapping_add(8);
        q = q.wrapping_add(1);
        entry = entry.wrapping_add(layout.stride());
    }
    depth
}

/// The quad's four vertices (x, y, 0, 1) at `vertices`, the i16 corners
/// times 1/16.
fn corners(h: &mut impl Host, vertices: u32, entry: u32, layout: Layout) {
    let inv16 = f64_at(h, INV_16);
    let mut c = [0u32; 8];
    for (k, v) in c.iter_mut().enumerate() {
        *v = store(mul(short(h, entry + 2 * k as u32), inv16));
    }
    // (x, y) of each vertex
    let xy = match layout {
        Layout::Corners4 => [(c[0], c[1]), (c[2], c[3]), (c[4], c[5]), (c[6], c[7])],
        Layout::Corners2 => [(c[0], c[1]), (c[2], c[1]), (c[0], c[3]), (c[2], c[3])],
    };
    for (n, (x, y)) in xy.into_iter().enumerate() {
        let at = vertices + 16 * n as u32;
        h.write(at, x);
        h.write(at + 4, y);
        h.write(at + 8, 0u32);
        h.write(at + 0xc, 1f32.to_bits());
    }
}

/// The quad's colour: the record's RGBA (`colour - 3` ..) by 1/256 (alpha
/// by `g_AlphaScale`) times the object's tint (`+4` ..) by 1/128; with the
/// object's `+2` bit 0x400 faded by the first vertex's w on screen (`(w *
/// 8 - 32) / 128`: 0 below 0, 1 above 128). Four copies; a channel above
/// one half (or NaN) clears `g_UseOptimizedVFXShaderArg`.
#[allow(clippy::too_many_arguments)]
fn quad_colour(
    h: &mut impl Host,
    frame: u32,
    segment: u32,
    obj: u32,
    entry: u32,
    colour: u32,
    q: u32,
) {
    let inv256 = f64_at(h, INV_256);
    let rgba: [u8; 4] = std::array::from_fn(|k| h.read(colour - 3 + k as u32));
    let mut r = store(mul(f64::from(rgba[0]), inv256));
    let mut g = store(mul(f64::from(rgba[1]), inv256));
    let mut b = store(mul(inv256, f64::from(rgba[2])));
    let mut a = store(div(f64::from(rgba[3]), image_f32(h, ALPHA_SCALE)));
    let inv128 = f64_at(h, INV_128);
    let tint: [u8; 4] = std::array::from_fn(|k| h.read(obj + 4 + k as u32));
    r = store(mul(mul(f64::from(tint[0]), inv128), load(r)));
    g = store(mul(mul(f64::from(tint[1]), inv128), load(g)));
    b = store(mul(mul(f64::from(tint[2]), inv128), load(b)));
    a = store(mul(mul(inv128, f64::from(tint[3])), load(a)));
    if h.read::<u16>(obj + 2) & 0x400 != 0 {
        let inv16 = f64_at(h, INV_16);
        let v = frame + 0xc0;
        let x = store(mul(short(h, entry), inv16));
        let y = store(mul(inv16, short(h, entry + 2)));
        h.write(v, x);
        h.write(v + 4, y);
        h.write(v + 8, 0u32);
        h.write(v + 0xc, 1f32.to_bits());
        h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, frame, v]);
        let w = store(sub(
            mul(f32_at(h, v + 0xc), f64_at(h, EIGHT)),
            f64_at(h, THIRTY_TWO),
        ));
        let w = load(w);
        // fcom 0 with w: below 0 -> 0; above 128 -> 1; else (NaN too) w/128
        let fade = if 0.0 > w {
            store(mul(0.0, inv128))
        } else if image_f32(h, F128) < w {
            1f32.to_bits()
        } else {
            store(mul(w, inv128))
        };
        // the original keeps each step in the vector's w
        h.write(v + 0xc, fade);
        let k = load(fade);
        r = store(mul(k, load(r)));
        g = store(mul(k, load(g)));
        b = store(mul(k, load(b)));
        a = store(mul(k, load(a)));
    }
    let cbuf: u32 = h.read(segment + 0x14);
    let out = cbuf.wrapping_add(q.wrapping_mul(0x40));
    for n in 0..4u32 {
        for (k, v) in [r, g, b, a].into_iter().enumerate() {
            h.write(out.wrapping_add(16 * n + 4 * k as u32), v);
        }
    }
    let half = image_f32(h, F_HALF);
    // the original's fcom chain: not at most one half, NaN included
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if [r, g, b, a].into_iter().any(|v| !(load(v) <= half)) {
        h.write(h.rebase(OPTIMIZED_SHADER), 0u32);
    }
}

/// The quad's uvs: the i16 `u`, `v`, `du`, `dv` (at the layout's offset) by
/// 1/4096, `v` flipped (`1 - v`); on level 0x672 the record 0x32c0 has `u`
/// at 1/8, 3/8, 5/8, 7/8 moved down by 1/8, `v` at 1/4, 3/4 by 1/4, and its
/// sizes doubled.
#[allow(clippy::too_many_arguments)]
fn quad_uvs(
    h: &mut impl Host,
    segment: u32,
    shape: u32,
    entry: u32,
    q: u32,
    level: u32,
    layout: Layout,
) {
    let at = entry + layout.uvs();
    let inv4096 = f64_at(h, INV_4096);
    let mut u = store(mul(short(h, at), inv4096));
    let mut v = store(mul(short(h, at + 2), inv4096));
    let (du, dv);
    if level == UV_LEVEL && h.read::<u32>(shape + 0x10) & 0x3fff == 0x32c0 {
        let uf = load(u);
        let eighths = [F_EIGHTH, F_THREE_EIGHTHS, F_FIVE_EIGHTHS];
        if eighths.iter().any(|&c| equal(image_f32(h, c), uf))
            || equal(f64_at(h, SEVEN_EIGHTHS), uf)
        {
            u = store(sub(uf, f64_at(h, EIGHTH)));
        }
        let vf = load(v);
        if equal(image_f32(h, F_QUARTER), vf) || equal(image_f32(h, F_THREE_QUARTERS), vf) {
            v = store(sub(vf, f64_at(h, QUARTER)));
        }
        let two = f64_at(h, TWO);
        du = store(mul(mul(short(h, at + 4), two), inv4096));
        dv = store(mul(inv4096, mul(two, short(h, at + 6))));
    } else {
        du = store(mul(short(h, at + 4), inv4096));
        dv = store(mul(inv4096, short(h, at + 6)));
    }
    let top = sub(1.0, load(v));
    let w = store(top);
    let x1 = store(add(load(du), load(u)));
    let y1 = store(sub(top, load(dv)));
    let ubuf: u32 = h.read(segment + 0x18);
    let out = ubuf.wrapping_add(q.wrapping_mul(0x20));
    for (k, value) in [u, w, x1, w, u, y1, x1, y1].into_iter().enumerate() {
        h.write(out.wrapping_add(4 * k as u32), value);
    }
}
