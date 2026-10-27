//! M4 list `h4-glue-other`, agent h4y: the particle fill-buffer job
//! `JobSchedule_pppPart_fillBuffer_drawShp_00727110` (`0x00727110`), the
//! shape parts' quads.
//!
//! It runs on the job workers (`JobSchedule_pppPart_fillBuffer`,
//! `docs/hooking.md` "Threads"); the shim hands it a zeroed per-call frame.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicLevelGetNameHash()` (cdecl).
pub const LEVEL_HASH: u32 = 0x0064_1da0;
/// `FFXVu0MulMatrix(dst, a, b)` (cdecl).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `FFXVu0MulVector(dst, a, b)` (cdecl).
pub const MUL_VECTOR: u32 = 0x006e_d5d0;
/// `VertexGroupApplyZMatZClip(m, a, b, c, d, flags)` (cdecl) -> nonzero
/// when clipped.
pub const ZCLIP: u32 = 0x0071_19d0;
/// `graphicVFXUpdateParameterOnly(instance, depth, 0, 0)` (cdecl).
pub const UPDATE_PARAMETER: u32 = 0x0064_5a50;
/// `graphicUseOptimizedVFXShader(instance, opaque)` (cdecl).
pub const USE_OPTIMIZED: u32 = 0x0064_4ac0;
/// `g_ScreenMatrix` (`0x02311480`).
pub const SCREEN_MATRIX: u32 = 0x0231_1480;
/// `f64` constants: 1/16 (`0x00b921e0`, positions), 1/128 (`0x00b170b8`,
/// the part's colour and the fade), 1/256 (`0x00b50198`, vertex colours),
/// 1/4096 (`0x00b50190`, texture coordinates), 8 (`0x00b922a8`) and 32
/// (`0x00b412f8`, the fade ramp), 1e-5 (`0x00b501e8`, the depth's w).
pub const INV_16: u32 = 0x00b9_21e0;
/// See [`INV_16`].
pub const INV_128: u32 = 0x00b1_70b8;
/// See [`INV_16`].
pub const INV_256: u32 = 0x00b5_0198;
/// See [`INV_16`].
pub const INV_4096: u32 = 0x00b5_0190;
/// See [`INV_16`].
pub const EIGHT: u32 = 0x00b9_22a8;
/// See [`INV_16`].
pub const THIRTY_TWO: u32 = 0x00b4_12f8;
/// See [`INV_16`].
pub const MIN_W: u32 = 0x00b5_01e8;
/// `f32` constants: 128 (`0x00b501bc`, the fade's top), 0.5
/// (`0x00b92194`, the opaque limit), -1 (`0x00b92340`, the default
/// depth).
pub const F128: u32 = 0x00b5_01bc;
/// See [`F128`].
pub const HALF: u32 = 0x00b9_2194;
/// See [`F128`].
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// The level whose depth uses the part's own matrix
/// (`graphicLevelGetNameHash` 0xea7).
pub const OWN_MATRIX_LEVEL: u32 = 0xea7;

/// Bytes of [`draw_shp`]'s frame: the original's `ebp - 0x1b8` up to its
/// cookie, the blocks its callees see (see the offsets below).
pub const FRAME: u32 = 0x1b4;
/// `ebp-0x1b8`: `g_ScreenMatrix` times the part's matrix (the fade).
pub const FADE_MATRIX: u32 = 0;
/// `ebp-0x178`: the part's matrix to the screen (the depth).
pub const PART_SCREEN: u32 = 0x40;
/// `ebp-0x138`: the z matrix to the screen (the depth).
pub const ZMAT_SCREEN: u32 = 0x80;
/// `ebp-0xc8`: the quad of a four-corner shape (flag 8).
pub const QUAD_CORNERS: u32 = 0xf0;
/// `ebp-0x88`: the quad of a rectangle shape.
pub const QUAD_RECT: u32 = 0x130;
/// `ebp-0x44`: the part's colour (bytes `+0xc8..` / 128).
pub const SCALE: u32 = 0x174;
/// `ebp-0x34`: the last primitive's second corner (the depth).
pub const LAST: u32 = 0x184;
/// `ebp-0x24`: the first vertex through the fade matrix.
pub const FADE: u32 = 0x194;
/// `ebp-0x14`: the primitive's colour.
pub const COLOUR: u32 = 0x1a4;

/// The part's six instances (`+0x14`) and render data (`+0x2c`).
const SLOTS: usize = 6;

fn f64_at(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

fn f32_at(h: &impl Host, at: u32) -> f64 {
    load(h.read(at))
}

/// A word as `fld dword` then `fstp dword` (a signalling NaN comes out
/// quiet).
fn copy_f32(h: &mut impl Host, dst: u32, src: u32) {
    let v = store(f32_at(h, src));
    h.write(dst, v);
}

/// A signed short times `scale`, stored as `f32`.
fn short_f32(h: &impl Host, at: u32, scale: f64) -> u32 {
    store(mul(f64::from(h.read::<i16>(at)), scale))
}

/// The render-data slot of a primitive by its mode byte (`+4`): 0x48 1,
/// 0x42 2, 0x46 3, 0x41 4, 0x88 5, anything else 0.
#[must_use]
pub fn mode_slot(mode: u8) -> usize {
    match mode {
        0x48 => 1,
        0x42 => 2,
        0x46 => 3,
        0x41 => 4,
        0x88 => 5,
        _ => 0,
    }
}

/// The shape's layout: four corners (flag 8: 0x18-byte records, the
/// corners' eight shorts, then the uv's four at `+0x10`) or a rectangle
/// (0x10-byte records, x0 y0 x1 y1, uv at `+8`).
#[derive(Clone, Copy)]
struct Layout {
    corners: bool,
    stride: u32,
    uv: u32,
    quad: u32,
    /// The record's second corner (the depth point of the last one).
    last: u32,
}

const CORNERS: Layout = Layout {
    corners: true,
    stride: 0x18,
    uv: 0x10,
    quad: QUAD_CORNERS,
    last: 0xc,
};
const RECT: Layout = Layout {
    corners: false,
    stride: 0x10,
    uv: 8,
    quad: QUAD_RECT,
    last: 4,
};

/// The quad's four vertices `(x, y, 0, 1)` from the record's shorts / 16:
/// the four corners in order, or a rectangle's (x0 y0), (x1 y0), (x0 y1),
/// (x1 y1).
fn build_quad(h: &mut impl Host, l: Layout, quad: u32, rec: u32, inv16: f64) {
    let xy: [(u32, u32); 4] = if l.corners {
        [(0, 2), (4, 6), (8, 0xa), (0xc, 0xe)]
    } else {
        [(0, 2), (4, 2), (0, 6), (4, 6)]
    };
    let one = 1f32.to_bits();
    for (v, (x, y)) in xy.into_iter().enumerate() {
        let at = quad + 0x10 * v as u32;
        let xv = short_f32(h, rec + x, inv16);
        let yv = short_f32(h, rec + y, inv16);
        h.write(at, xv);
        h.write(at + 4, yv);
        h.write(at + 8, 0u32);
        h.write(at + 0xc, one);
    }
}

/// The render data's segment block (`[data + 0x94]`) word at `off`.
fn segment(h: &impl Host, data: u32, off: u32) -> u32 {
    let seg: u32 = h.read(data.wrapping_add(0x94));
    h.read(seg.wrapping_add(off))
}

/// The fade of a primitive (part flag 0x400): its first vertex through
/// the fade matrix, w * 8 - 32 clamped: below 0 gives 0, above 128 gives
/// 1, else / 128; the colour is scaled by it.
fn fade(h: &mut impl Host, frame: u32, vtx: u32) {
    let v = frame + FADE;
    copy_f32(h, v, vtx);
    copy_f32(h, v + 4, vtx + 4);
    h.write(v + 8, 0u32);
    h.write(v + 0xc, 1f32.to_bits());
    h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, frame + FADE_MATRIX, v]);
    let w = f32_at(h, v + 0xc);
    let x = load(store(sub(mul(w, f64_at(h, EIGHT)), f64_at(h, THIRTY_TWO))));
    let inv128 = f64_at(h, INV_128);
    let a = if x < 0.0 {
        store(mul(0.0, inv128))
    } else if f32_at(h, h.rebase(F128)) < x {
        1f32.to_bits()
    } else {
        store(mul(x, inv128))
    };
    h.write(v + 0xc, a);
    for j in 0..4 {
        let at = frame + COLOUR + 4 * j;
        let c = store(mul(load(a), f32_at(h, at)));
        h.write(at, c);
    }
}

/// The primitive's texture coordinates from the record's four shorts /
/// 4096 (u0, t1, t2, d): u0 1 - t1, u0 + t2 1 - t1, u0 1 - t1 - d,
/// u0 + t2 1 - t1 - d (1 - t1 kept unrounded for the last).
fn uvs(h: &mut impl Host, uv: u32, src: u32, inv4096: f64) {
    let u0 = short_f32(h, src, inv4096);
    let t1 = short_f32(h, src + 2, inv4096);
    let t2 = short_f32(h, src + 4, inv4096);
    let d = short_f32(h, src + 6, inv4096);
    let top = sub(1.0, load(t1));
    let v0 = store(top);
    let u1 = store(add(load(t2), load(u0)));
    let v1 = store(sub(top, load(d)));
    for (k, w) in [u0, v0, u1, v0, u0, v1, u1, v1].into_iter().enumerate() {
        h.write(uv + 4 * k as u32, w);
    }
}

/// Two triangles of the quad whose first vertex is `4 n`: n, n+2, n+1 and
/// n+2, n+3, n+1 (u16).
fn indices(h: &mut impl Host, at: u32, n: u32) {
    let b = (n << 2) as u16;
    for (k, d) in [0u16, 2, 1, 2, 3, 1].into_iter().enumerate() {
        h.write(at + 2 * k as u32, b.wrapping_add(d));
    }
}

/// What a primitive's fill leaves for the job: the per-slot counts and
/// whether every colour stayed at or below 0.5.
struct Fill {
    counts: [u32; SLOTS],
    opaque: u32,
}

/// The part's primitives, each into its slot's render data: a quad
/// (through the part's matrix, or clipped against the z matrix: then
/// its indices are zero), the colour (scaled by the part's and the
/// fade), the texture coordinates and two triangles.
#[allow(clippy::too_many_arguments)] // the original's locals
fn primitives(
    h: &mut impl Host,
    frame: u32,
    part: u32,
    shape: u32,
    by_kind: u32,
    datas: &[u32; SLOTS],
    zmat: u32,
    faded: bool,
) -> Fill {
    let l = if h.read::<u8>(shape) & 8 != 0 {
        CORNERS
    } else {
        RECT
    };
    let mut f = Fill {
        counts: [0; SLOTS],
        opaque: 1,
    };
    let matrix = part + 0x48;
    let inv16 = f64_at(h, INV_16);
    let inv256 = f64_at(h, INV_256);
    let inv4096 = f64_at(h, INV_4096);
    let mut rec = shape + 0x20;
    let mut prim = shape.wrapping_add(h.read(shape + 4));
    let mut k = 0i32;
    while k < i32::from(h.read::<i16>(shape + 2)) {
        let slot = if by_kind == 0 {
            0
        } else {
            mode_slot(h.read(prim + 4))
        };
        let n = f.counts[slot];
        let data = datas[slot];
        let vtx = segment(h, data, 0xc).wrapping_add(n.wrapping_mul(0x30));
        let quad = frame + l.quad;
        build_quad(h, l, quad, rec, inv16);
        let clipped = zmat != 0 && {
            let flags = i32::from(h.read::<i16>(shape)).cast_unsigned();
            let args = [quad, matrix, zmat, part + 0x110, part + 0x150, flags];
            h.call_sym(ZCLIP, Conv::Cdecl, &args) != 0
        };
        if clipped {
            let at = segment(h, data, 0x1c).wrapping_add(n.wrapping_mul(0xc));
            for w in 0..3 {
                h.write(at + 4 * w, 0u32);
            }
            f.counts[slot] = n.wrapping_add(1);
        } else {
            if zmat == 0 {
                h.call_sym(MUL_MATRIX, Conv::Cdecl, &[quad, matrix, quad]);
            }
            for v in 0..4 {
                for c in 0..3 {
                    copy_f32(h, vtx + 0xc * v + 4 * c, quad + 0x10 * v + 4 * c);
                }
            }
            if k == i32::from(h.read::<i16>(shape + 2)) - 1 {
                let x = short_f32(h, rec + l.last, inv16);
                let y = short_f32(h, rec + l.last + 2, inv16);
                h.write(frame + LAST, x);
                h.write(frame + LAST + 4, y);
                h.write(frame + LAST + 8, 0u32);
                h.write(frame + LAST + 0xc, 1f32.to_bits());
            }
            for j in 0..4 {
                let c = store(mul(f64::from(h.read::<u8>(prim + j)), inv256));
                h.write(frame + COLOUR + 4 * j, c);
            }
            if faded {
                fade(h, frame, vtx);
            }
            let col = segment(h, data, 0x14).wrapping_add(n.wrapping_mul(0x40));
            for v in 0..4 {
                let args = [col + 0x10 * v, frame + SCALE, frame + COLOUR];
                h.call_sym(MUL_VECTOR, Conv::Cdecl, &args);
            }
            let half = f32_at(h, h.rebase(HALF));
            if (0..4).any(|j| half < f32_at(h, col + 4 * j)) {
                f.opaque = 0;
            }
            let uv = segment(h, data, 0x18).wrapping_add(n.wrapping_mul(0x20));
            uvs(h, uv, rec + l.uv, inv4096);
            let at = segment(h, data, 0x1c).wrapping_add(n.wrapping_mul(0xc));
            indices(h, at, n);
            f.counts[slot] = n.wrapping_add(1);
        }
        rec = rec.wrapping_add(l.stride);
        prim = prim.wrapping_add(8);
        k += 1;
    }
    f
}

/// The depth of the part: the last primitive's second corner through
/// `g_ScreenMatrix` times the z matrix (the part's own on level 0xea7) or
/// the part's matrix; z / w when |w| > 1e-5, else -1. Reads the corner
/// from the frame: junk read: [ebp-0x34..-0x28] never written when the
/// part has no primitive or its last is clipped; 0 (brief 2026-10-24; the
/// shim's frame starts zeroed).
fn depth(h: &mut impl Host, frame: u32, part: u32, zmat: u32, level: u32) -> u32 {
    let mut out = store(f32_at(h, h.rebase(MINUS_ONE)));
    let (m, src) = if zmat == 0 {
        (frame + PART_SCREEN, part + 0x48)
    } else if level == OWN_MATRIX_LEVEL {
        (frame + ZMAT_SCREEN, part + 0x48)
    } else {
        (frame + ZMAT_SCREEN, zmat)
    };
    for w in 0..16 {
        let v: u32 = h.read(src + 4 * w);
        h.write(m + 4 * w, v);
    }
    let screen = h.rebase(SCREEN_MATRIX);
    h.call_sym(MUL_MATRIX, Conv::Cdecl, &[m, screen, m]);
    let v = frame + LAST;
    h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, m, v]);
    let w = f32_at(h, v + 0xc);
    if load(store(w.abs())) > f64_at(h, MIN_W) {
        out = store(div(f32_at(h, v + 8), w));
    }
    out
}

/// `JobSchedule_pppPart_fillBuffer_drawShp_00727110(part)` (`0x00727110`,
/// cdecl; `frame` see [`FRAME`]) -> 1. The part's shape (`+0x10`: flags
/// byte, i16 flags at `+0`, i16 count `+2`, colour records `+[+4]`, 8
/// bytes each, shape records from `+0x20`) fills its render data
/// (`+0x2c`, six; the slot by the colour record's mode when `+0xcc` is
/// set): vertices, colours, texture coordinates and indices. Then the
/// first render data gets the part's matrix (`+0xd0`, w column 0 0 0 1)
/// and a flag word cleared (`+0x80`); each instance (`+0x14`) whose bit
/// is set in the first render data's `+0xc8` gets a copy of it, the depth
/// and the opaque flag (every colour component at most 0.5).
///
/// The original reads its six render data and instances from arrays in
/// its frame by the first render data's u16 count (`+0xc4`); past six it
/// would read its other locals as handles. Counts are at most 4 in the
/// game (`createVFXRenderDataBuffer`), so the port stops at six.
pub fn draw_shp(h: &mut impl Host, frame: u32, part: u32) -> u32 {
    let shape: u32 = h.read(part + 0x10);
    let by_kind: u32 = h.read(part + 0xcc);
    let level = h.call_sym(LEVEL_HASH, Conv::Cdecl, &[]);
    let flags: u16 = h.read(part + 0x44);
    let zmat = if h.read::<u8>(part + 1) == 0 {
        0
    } else {
        part + 0x88
    };
    let inv128 = f64_at(h, INV_128);
    for j in 0..4 {
        let c = store(mul(f64::from(h.read::<u8>(part + 0xc8 + j)), inv128));
        h.write(frame + SCALE + 4 * j, c);
    }
    let mut instances = [0u32; SLOTS];
    let mut datas = [0u32; SLOTS];
    for k in 0..SLOTS {
        instances[k] = h.read(part + 0x14 + 4 * k as u32);
        datas[k] = h.read(part + 0x2c + 4 * k as u32);
    }
    let faded = flags & 0x400 != 0;
    if faded {
        let screen = h.rebase(SCREEN_MATRIX);
        h.call_sym(
            MUL_MATRIX,
            Conv::Cdecl,
            &[frame + FADE_MATRIX, screen, part + 0x48],
        );
    }
    let fill = primitives(h, frame, part, shape, by_kind, &datas, zmat, faded);
    let first = datas[0];
    for w in 0..16 {
        let v: u32 = h.read(part + 0xd0 + 4 * w);
        h.write(first.wrapping_add(4 * w), v);
    }
    for off in [0xc, 0x1c, 0x2c] {
        h.write(first.wrapping_add(off), 0u32);
    }
    h.write(first.wrapping_add(0x3c), 1f32.to_bits());
    let d = depth(h, frame, part, zmat, level);
    h.write(first.wrapping_add(0x80), 0u32);
    let d = store(load(d));
    if h.read::<u8>(first.wrapping_add(0xc8)) & 1 != 0 {
        h.call_sym(UPDATE_PARAMETER, Conv::Cdecl, &[instances[0], d, 0, 0]);
        h.call_sym(USE_OPTIMIZED, Conv::Cdecl, &[instances[0], fill.opaque]);
    }
    let mut i = 1u32;
    while i < u32::from(h.read::<u16>(first.wrapping_add(0xc4))) && (i as usize) < SLOTS {
        if h.read::<u32>(first.wrapping_add(0xc8)) & (1 << (i & 31)) != 0 {
            let dst = datas[i as usize];
            for w in 0..16 {
                let v: u32 = h.read(first.wrapping_add(4 * w));
                h.write(dst.wrapping_add(4 * w), v);
            }
            let flag: u32 = h.read(first.wrapping_add(0x80));
            h.write(dst.wrapping_add(0x80), flag);
            let inst = instances[i as usize];
            h.call_sym(UPDATE_PARAMETER, Conv::Cdecl, &[inst, d, 0, 0]);
            h.call_sym(USE_OPTIMIZED, Conv::Cdecl, &[inst, fill.opaque]);
        }
        i += 1;
    }
    1
}
