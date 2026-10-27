//! M4 list `h4-glue-other`, agent h4y: the particle fill-buffer job
//! `JobSchedule_pppPart_fillBuffer_drawMdlDtt` (`0x007256c0`), the DTT
//! model parts' triangles.
//!
//! It runs on the job workers (`JobSchedule_pppPart_fillBuffer`,
//! `docs/hooking.md` "Threads"); the shim hands it a zeroed per-call frame
//! of 80 KB, as the original's (the job workers commit 128 KB).

use ffx_bridge::x87::{add, div, load, mul, sqrt, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicIsInCut2DScene()` (cdecl).
pub const IN_CUT_2D: u32 = 0x0064_1c00;
/// `graphicGetCut2DSceneBias()` (cdecl) -> `ST0`.
pub const CUT_2D_BIAS: u32 = 0x0064_1130;
/// `KeDttMdl_UseSingleInstance()` (cdecl).
pub const SINGLE_INSTANCE: u32 = 0x0072_e6d0;
/// `ModelBuffer_GetBufferByIndex(index)` (cdecl).
pub const BUFFER_BY_INDEX: u32 = 0x0072_c190;
/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `FFXVu0CopyVector(dst, src)` (cdecl).
pub const COPY_VECTOR: u32 = 0x006e_d370;
/// `FFXVu0CopyMatrix(dst, src)` (cdecl).
pub const COPY_MATRIX: u32 = 0x006e_d360;
/// `FFXVu0SubVector(dst, a, b)` (cdecl).
pub const SUB_VECTOR: u32 = 0x006e_d940;
/// `sceVu0VectorLengthSquared(v)` (cdecl) -> `ST0` (an `f32`).
pub const LENGTH_SQUARED: u32 = 0x006e_d9b0;
/// `memset` through its thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `MagicFile_GetMagicId()` (cdecl).
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// `graphicVFXUpdateParameterOnly(instance, depth, 0, 0)` (cdecl).
pub const UPDATE_PARAMETER: u32 = 0x0064_5a50;
/// `g_MovieFading` (`0x01340824`).
pub const MOVIE_FADING: u32 = 0x0134_0824;
/// `f64` constants: 0.25 (`0x00b34f90`), 0 (`0x00b0db90`), 3
/// (`0x00b2c5c8`), 4 (`0x00b43d80`; the four depth bands), 0.33333
/// (`0x00b501f0`, a triangle's mean depth), 1/255 (`0x00b34258`), 1e-6
/// (`0x00b92188`), 2048.5 (`0x00b50218`), 448 (`0x00b50210`), 0.5
/// (`0x00b92220`), 1.333333 (`0x00b50200`).
pub const QUARTER: u32 = 0x00b3_4f90;
/// See [`QUARTER`].
pub const ZERO: u32 = 0x00b0_db90;
/// See [`QUARTER`].
pub const THREE: u32 = 0x00b2_c5c8;
/// See [`QUARTER`].
pub const FOUR: u32 = 0x00b4_3d80;
/// See [`QUARTER`].
pub const THIRD: u32 = 0x00b5_01f0;
/// See [`QUARTER`].
pub const INV_255: u32 = 0x00b3_4258;
/// See [`QUARTER`].
pub const MIN_LENGTH: u32 = 0x00b9_2188;
/// See [`QUARTER`].
pub const CENTRE: u32 = 0x00b5_0218;
/// See [`QUARTER`].
pub const HEIGHT: u32 = 0x00b5_0210;
/// See [`QUARTER`].
pub const HALF: u32 = 0x00b9_2220;
/// See [`QUARTER`].
pub const CUT_SCALE: u32 = 0x00b5_0200;
/// `f32` constants: 2^24 (`0x00b41da0`, the depth minimum's start), 512
/// (`0x00b41d9c`) and 682 (`0x00b50220`, the width in a 2D cut or not),
/// 4096 (`0x00b50224`, the screen's limit), -1e6 (`0x00b501d4`) and 1e6
/// (`0x00b2be08`, the bounds), -1 (`0x00b92340`).
pub const MIN_START: u32 = 0x00b4_1da0;
/// See [`MIN_START`].
pub const WIDTH_CUT: u32 = 0x00b4_1d9c;
/// See [`MIN_START`].
pub const WIDTH: u32 = 0x00b5_0220;
/// See [`MIN_START`].
pub const LIMIT: u32 = 0x00b5_0224;
/// See [`MIN_START`].
pub const BOUND_LOW: u32 = 0x00b5_01d4;
/// See [`MIN_START`].
pub const BOUND_HIGH: u32 = 0x00b2_be08;
/// See [`MIN_START`].
pub const MINUS_ONE: u32 = 0x00b9_2340;

/// Bytes of [`draw_mdl_dtt`]'s frame: the original's `ebp - 0x14114` up
/// to its cookie. Offsets in it are written as the original's `ebp`
/// offsets ([`slot`]): the transformed points (`-0x14114`, 16 bytes
/// each, then the depths, one `f32` each), the twelve colours
/// (`-0x114`), the vertices through `+0xf0` (`-0xd4`, `-0xc4`, `-0x94`),
/// the bands (`-0xb4` low, `-0xa4` high), the model's origin (`-0x84`), the
/// vertices (`-0x64`, `-0x74`, `-0x24`) and their projections (`-0x34`,
/// `-0x44`, `-0x54`), each vertex's w then holding its 1/w (`-0x58`,
/// `-0x68`, `-0x18`), and the direction from the origin / texture coordinate
/// (`-0x14`).
pub const FRAME: u32 = 0x1_4110;
const BASE: u32 = 0x1_4114;

/// The frame address of the original's `[ebp - off]`.
#[must_use]
pub fn slot(frame: u32, off: u32) -> u32 {
    frame + (BASE - off)
}

const POINTS: u32 = 0x1_4114;
const COLOURS: u32 = 0x114;
const LOW: u32 = 0xb4;
const HIGH: u32 = 0xa4;
const ORIGIN: u32 = 0x84;
const DIR: u32 = 0x14;

/// The three corners' frame vectors: source copy, projection, through
/// `+0xf0`, and their offsets in the vertex and index records.
struct Corner {
    copy: u32,
    proj: u32,
    extra: u32,
    /// The texture coordinates' copy table (its frame cursor is ours).
    table: usize,
}

const CORNERS: [Corner; 3] = [
    Corner {
        copy: 0x64,
        proj: 0x34,
        extra: 0xd4,
        table: 0,
    },
    Corner {
        copy: 0x74,
        proj: 0x44,
        extra: 0xc4,
        table: 1,
    },
    Corner {
        copy: 0x24,
        proj: 0x54,
        extra: 0x94,
        table: 2,
    },
];

fn f64_at(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

fn f32_at(h: &impl Host, at: u32) -> f64 {
    load(h.read(at))
}

/// An image `f32` constant as `fld dword` then `fstp dword`.
fn image_f32(h: &impl Host, sym: u32) -> u32 {
    store(f32_at(h, h.rebase(sym)))
}

/// What stays the same for the whole part.
#[allow(clippy::struct_excessive_bools)] // the part's flag words
struct Job {
    part: u32,
    /// `+0x70`: the view matrix.
    view: u32,
    /// `+0x16c`: the vertices also go through `+0xf0`.
    extra: bool,
    /// `+0x154`: the texture coordinates are exchanged with the table.
    swap: bool,
    /// `+0x158`: the table gives them (else takes them).
    from_table: bool,
    /// `+0x160`: the push scales with the direction's z.
    by_z: bool,
    /// The screen width: 512 in a 2D cut, else 682.
    width: f64,
    cut: bool,
    bias: u32,
    /// The part's texture offsets (`+0x164`, `+0x168`) times 0.5.
    half_u: f64,
    half_v: f64,
    single: bool,
    datas: [u32; 4],
}

/// The points through `+0x30` and the view (`+0x70`), with their depths
/// (z / w) after them; returns the depths' (min, max): from the model
/// buffer (`buffer`'s `+0x14`, 16-byte vectors) or the shape's i16
/// triples (`+8` relative).
fn project_points(
    h: &mut impl Host,
    frame: u32,
    p: &Job,
    shape: u32,
    buffer: u32,
    depths: u32,
) -> (u32, u32) {
    let mut max = store(0.0);
    let mut min = image_f32(h, MIN_START);
    let mut src = shape.wrapping_add(h.read(shape + 8));
    let mut i = 0i32;
    while i < i32::from(h.read::<i16>(shape + 0x12)) {
        let pt = slot(frame, POINTS).wrapping_add(16 * i.cast_unsigned());
        let (v, z, w) = if buffer != 0 {
            let at = h
                .read::<u32>(buffer + 0x14)
                .wrapping_add(16 * i.cast_unsigned());
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[pt, p.part + 0x30, at]);
            (slot(frame, 0x94), 0x8c, 0x88)
        } else {
            let v = slot(frame, 0x24);
            for k in 0..3 {
                let s = f64::from(h.read::<i16>(src + 2 * k));
                h.write(v + 4 * k, store(s));
            }
            h.write(v + 0xc, 1f32.to_bits());
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[pt, p.part + 0x30, v]);
            src = src.wrapping_add(6);
            (v, 0x1c, 0x18)
        };
        h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, p.view, pt]);
        let d = store(div(f32_at(h, slot(frame, z)), f32_at(h, slot(frame, w))));
        h.write(depths.wrapping_add(4 * i.cast_unsigned()), d);
        if load(max) < load(d) {
            max = d;
        }
        if load(min) > load(d) {
            min = d;
        }
        i += 1;
    }
    (min, max)
}

/// The four depth bands between min and max: low `min + k q`, high
/// `min + (k + 1) q` with q = (max - min) / 4 (the first low as 0 q, the
/// last high as 4 q, by the image's constants).
fn bands(h: &mut impl Host, frame: u32, min: u32, max: u32) {
    let lo = load(min);
    let q = load(store(mul(sub(load(max), lo), f64_at(h, QUARTER))));
    let low = slot(frame, LOW);
    let high = slot(frame, HIGH);
    h.write(low, store(add(mul(f64_at(h, ZERO), q), lo)));
    let t1 = store(add(q, lo));
    h.write(high, t1);
    h.write(low + 4, t1);
    let t2 = store(add(add(q, q), lo));
    h.write(high + 4, t2);
    h.write(low + 8, t2);
    let t3 = store(add(mul(f64_at(h, THREE), q), lo));
    h.write(high + 8, t3);
    h.write(low + 0xc, t3);
    h.write(high + 0xc, store(add(lo, mul(q, f64_at(h, FOUR)))));
}

/// Where a triangle goes: its render data and that one's buffers, and
/// its first vertex.
#[derive(Clone, Copy)]
struct Target {
    data: u32,
    vertices: u32,
    colours: u32,
    uvs: u32,
    indices: u32,
    base: u32,
}

fn target(h: &impl Host, data: u32, base: Option<u32>) -> Target {
    let seg: u32 = h.read(data.wrapping_add(0x94));
    let colours = h.read(seg.wrapping_add(0x14));
    let vertices = h.read(seg.wrapping_add(0xc));
    let indices = h.read(seg.wrapping_add(0x1c));
    let uvs = h.read(seg.wrapping_add(0x18));
    let base = base.unwrap_or_else(|| h.read::<u32>(seg.wrapping_add(8)) / 3);
    Target {
        data,
        vertices,
        colours,
        uvs,
        indices,
        base,
    }
}

/// The band of a triangle by the mean of its three depths (low <= mean
/// <= high, the first band that holds it), or the first render data.
fn pick(h: &impl Host, frame: u32, p: &Job, rec: u32, depths: u32) -> Target {
    let third = f64_at(h, THIRD);
    let depth = |k: u32| {
        let i = u32::from(h.read::<u16>(rec + k));
        f32_at(h, depths.wrapping_add(4 * i))
    };
    let mean = load(store(mul(
        add(add(depth(0xe), depth(0xc)), depth(0x10)),
        third,
    )));
    for j in 0..4 {
        let lo = f32_at(h, slot(frame, LOW) + 4 * j);
        let hi = f32_at(h, slot(frame, HIGH) + 4 * j);
        if lo <= mean && hi >= mean {
            return target(h, p.datas[j as usize], None);
        }
    }
    target(h, p.datas[0], None)
}

/// A corner through the view (and `+0xf0`), divided by its w: `false`
/// when it falls outside (0, 4096) on x or y (NaN stays inside).
fn corner(h: &mut impl Host, frame: u32, p: &Job, c: &Corner, index: u16) -> bool {
    let copy = slot(frame, c.copy);
    let proj = slot(frame, c.proj);
    let pt = slot(frame, POINTS).wrapping_add(16 * u32::from(index));
    h.call_sym(COPY_VECTOR, Conv::Cdecl, &[copy, pt]);
    h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[proj, p.view, copy]);
    if p.extra {
        let e = slot(frame, c.extra);
        h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[e, p.part + 0xf0, proj]);
    }
    // 1/w into the copy's w
    let iw = store(div(1.0, f32_at(h, proj + 0xc)));
    h.write(copy + 0xc, iw);
    let x = store(mul(f32_at(h, proj), load(iw)));
    h.write(proj, x);
    let y = store(mul(f32_at(h, proj + 4), load(iw)));
    h.write(proj + 4, y);
    let z = store(mul(load(iw), f32_at(h, proj + 8)));
    h.write(proj + 8, z);
    let limit = f32_at(h, h.rebase(LIMIT));
    let (x, y) = (load(x), load(y));
    !(x <= 0.0 || x >= limit || y <= 0.0 || y >= limit)
}

/// The corner's texture coordinates: its screen position pushed along its
/// direction from the model's origin (`-0x14`, normalised; scaled by `+0x140`/`+0x144`,
/// with `+0x160` also by its z) over its w, mapped to the texture
/// (centre 2048.5, width 682 or 512, height 448), plus the part's
/// offsets.
fn corner_uv(h: &mut impl Host, frame: u32, p: &Job, c: &Corner) -> (u32, u32) {
    let dir = slot(frame, DIR);
    h.call_sym(
        SUB_VECTOR,
        Conv::Cdecl,
        &[dir, slot(frame, c.copy), slot(frame, ORIGIN)],
    );
    let l = h.call_sym_st0(LENGTH_SQUARED, Conv::Cdecl, &[dir]);
    let s = load(store(sqrt(load(l.to_bits()))));
    let proj = slot(frame, c.proj);
    let x = f32_at(h, proj);
    let y = f32_at(h, proj + 4);
    let centre = f64_at(h, CENTRE);
    let half = f64_at(h, HALF);
    let (sx, sy) = if f64_at(h, MIN_LENGTH) > s {
        (x, y)
    } else {
        let r = load(store(div(1.0, s)));
        let k0 = f32_at(h, p.part + 0x140);
        let k1 = f32_at(h, p.part + 0x144);
        let dx = f32_at(h, dir);
        let dy = f32_at(h, dir + 4);
        let (a, b) = if p.by_z {
            let rz = mul(r, f32_at(h, dir + 8));
            (mul(mul(mul(r, dx), rz), k0), mul(mul(mul(r, dy), rz), k1))
        } else {
            (mul(mul(r, dx), k0), mul(mul(r, dy), k1))
        };
        let iw = f32_at(h, slot(frame, c.copy) + 0xc);
        (add(mul(a, iw), x), add(mul(iw, b), y))
    };
    let u = store(add(div(sub(sx, centre), p.width), half));
    let v = store(sub(half, div(sub(sy, centre), f64_at(h, HEIGHT))));
    (u, v)
}

/// The corner's texture coordinates after the part's offsets, exchanged
/// with the table, into the uv buffer.
fn store_uv(h: &mut impl Host, frame: u32, p: &Job, u: u32, v: u32, table: u32, at: u32) {
    let dir = slot(frame, DIR);
    h.write(dir, store(add(p.half_u, load(u))));
    h.write(dir + 4, store(add(p.half_v, load(v))));
    if p.swap {
        let args = if p.from_table {
            [dir, table]
        } else {
            [table, dir]
        };
        h.call_sym(COPY_VECTOR, Conv::Cdecl, &args);
    }
    let u = store(f32_at(h, dir));
    let v = store(f32_at(h, dir + 4));
    h.write(at, u);
    h.write(at + 4, v);
}

/// One triangle record (0x14 bytes: twelve colour bytes, three u16 point
/// indices at `+0xc`) into its target: the three corners (any outside
/// the screen zeroes the triangle's indices), the colours (the source's
/// times the part's `+0x130..` / 255) and texture coordinates; in a 2D cut
/// the v's are scaled by 1.333333 less the cut's bias.
#[allow(clippy::too_many_arguments)] // the original's cursors
fn triangle(
    h: &mut impl Host,
    frame: u32,
    p: &Job,
    rec: u32,
    k: u32,
    t: Target,
    colour_buffer: u32,
    tables: [u32; 3],
) {
    let seg: u32 = h.read(t.data.wrapping_add(0x94));
    let n: u32 = h.read(seg.wrapping_add(8));
    h.write(seg.wrapping_add(8), n.wrapping_add(3));
    let seg: u32 = h.read(t.data.wrapping_add(0x94));
    let n: u32 = h.read(seg.wrapping_add(4));
    h.write(seg.wrapping_add(4), n.wrapping_add(3));
    if colour_buffer == 0 {
        for j in 0..12 {
            let c = store(f64::from(h.read::<u8>(rec + j)));
            h.write(slot(frame, COLOURS) + 4 * j, c);
        }
    }
    let first = t.base.wrapping_mul(3);
    let colours = t.colours.wrapping_add(t.base.wrapping_mul(0x30));
    let source = if colour_buffer == 0 {
        slot(frame, COLOURS)
    } else {
        colour_buffer.wrapping_add(k.wrapping_mul(0x30))
    };
    let vtx = t.vertices.wrapping_add(t.base.wrapping_mul(0x24));
    for (i, c) in (0u32..).zip(&CORNERS) {
        let index = h.read::<u16>(rec + 0xc + 2 * i);
        if !corner(h, frame, p, c, index) {
            let at = t.indices.wrapping_add(2 * first);
            h.write(at, 0u32);
            h.write(at + 4, 0u16);
            return;
        }
        let src = slot(frame, if p.extra { c.extra } else { c.copy });
        for w in 0..3 {
            let v = store(f32_at(h, src + 4 * w));
            h.write(vtx + 0xc * i + 4 * w, v);
        }
        h.write(t.indices.wrapping_add(2 * (first + i)), (first + i) as u16);
    }
    let inv255 = f64_at(h, INV_255);
    for (i, c) in (0u32..).zip(&CORNERS) {
        for j in 0..4 {
            let at = 0x10 * i + 4 * j;
            let s = f32_at(h, source + at);
            let m = f32_at(h, p.part + 0x130 + 4 * j);
            h.write(colours + at, store(mul(mul(s, m), inv255)));
        }
        let (u, v) = corner_uv(h, frame, p, c);
        let at = t.uvs.wrapping_add(8 * (first + i));
        store_uv(h, frame, p, u, v, tables[c.table], at);
    }
    if p.cut {
        let scale = f64_at(h, CUT_SCALE);
        let bias = load(p.bias);
        let uv = t.uvs.wrapping_add(8 * first);
        for i in 0..3 {
            let at = uv + 8 * i + 4;
            let v = store(sub(mul(f32_at(h, at), scale), bias));
            h.write(at, v);
        }
    }
}

/// After a group: each band's render data gets its counts set to the
/// group's (3n) and the vertices and indices past what was filled
/// cleared.
fn close_group(h: &mut impl Host, p: &Job, n: u32) {
    for &data in &p.datas {
        let seg: u32 = h.read(data.wrapping_add(0x94));
        let filled = h.read::<u32>(seg.wrapping_add(4)) / 3;
        let at = h
            .read::<u32>(seg.wrapping_add(0x1c))
            .wrapping_add(h.read::<u32>(seg.wrapping_add(8)).wrapping_mul(2));
        let size = n.wrapping_sub(filled).wrapping_mul(6);
        h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, size]);
        let seg: u32 = h.read(data.wrapping_add(0x94));
        let filled = h.read::<u32>(seg.wrapping_add(8)) / 3;
        let at = h
            .read::<u32>(seg.wrapping_add(0xc))
            .wrapping_add(h.read::<u32>(seg.wrapping_add(4)).wrapping_mul(0xc));
        let size = n.wrapping_sub(filled).wrapping_mul(0x24);
        h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, size]);
        let seg: u32 = h.read(data.wrapping_add(0x94));
        h.write(seg.wrapping_add(4), n.wrapping_mul(3));
        let seg: u32 = h.read(data.wrapping_add(0x94));
        h.write(seg.wrapping_add(8), n.wrapping_mul(3));
    }
}

/// `JobSchedule_pppPart_fillBuffer_drawMdlDtt(part)` (`0x007256c0`,
/// cdecl; `frame` see [`FRAME`]) -> `part + 0xb0`. The DTT model
/// (`+0x150`: i16 point count `+0x12`, groups `+[+4]`, points `+[+8]` or
/// the model buffer of the first render data's u16 `+0xc2`) is projected;
/// each group (16-byte header: byte `+1` 0 to draw, 0xff after the last;
/// i16 count `+2`; 0x14-byte triangles) fills the render data (`+0x10`,
/// four, one per depth band; or only the first with
/// `KeDttMdl_UseSingleInstance`). Then each render data gets the part's
/// matrix (`+0xb0`, w column 0 0 0 1) and bounds of +-1e6, and its
/// instance (`+0x20`) the band's middle depth (magic 0x46 and 0x23a: 1,
/// -1 or the middle by `+0x154`/`+0x158`).
#[allow(clippy::too_many_lines)] // the original's set-up, groups and bands
pub fn draw_mdl_dtt(h: &mut impl Host, frame: u32, part: u32) -> u32 {
    let shape: u32 = h.read(part + 0x150);
    let s0 = store(f32_at(h, part + 0x164));
    let s1 = store(f32_at(h, part + 0x168));
    let groups0 = shape.wrapping_add(h.read(shape + 4));
    let count = i32::from(h.read::<i16>(shape + 0x12));
    let depths = slot(frame, POINTS).wrapping_add((count << 4).cast_unsigned());
    let cut =
        h.call_sym(IN_CUT_2D, Conv::Cdecl, &[]) != 0 && h.read::<u32>(h.rebase(MOVIE_FADING)) == 0;
    let (width, bias) = if cut {
        let w = image_f32(h, WIDTH_CUT);
        let b = h.call_sym_st0_f64(CUT_2D_BIAS, Conv::Cdecl, &[]);
        (w, store(b))
    } else {
        (image_f32(h, WIDTH), store(0.0))
    };
    let single = h.call_sym(SINGLE_INSTANCE, Conv::Cdecl, &[]) != 0;
    let mut datas = [0u32; 4];
    let mut instances = [0u32; 4];
    for k in 0..4 {
        datas[k as usize] = h.read(part + 0x10 + 4 * k);
        instances[k as usize] = h.read(part + 0x20 + 4 * k);
        // the model's origin: the +0x30 matrix's translation
        let l: u32 = h.read(part + 0x60 + 4 * k);
        h.write(slot(frame, ORIGIN) + 4 * k, l);
    }
    let half = f64_at(h, HALF);
    let p = Job {
        part,
        view: part + 0x70,
        extra: h.read::<u32>(part + 0x16c) != 0,
        swap: h.read::<u32>(part + 0x154) != 0,
        from_table: h.read::<u32>(part + 0x158) != 0,
        by_z: h.read::<u32>(part + 0x160) != 0,
        width: load(width),
        cut,
        bias,
        half_u: mul(load(s0), half),
        half_v: mul(half, load(s1)),
        single,
        datas,
    };
    let index = h.read::<u16>(datas[0].wrapping_add(0xc2));
    let buffer = if index == 0xffff {
        0
    } else {
        h.call_sym(BUFFER_BY_INDEX, Conv::Cdecl, &[u32::from(index)])
    };
    let (min, max) = project_points(h, frame, &p, shape, buffer, depths);
    bands(h, frame, min, max);
    let mut table = h.read::<u32>(part + 0x15c);
    let mut groups = groups0;
    let mut single_target = None;
    loop {
        let colour_buffer = if buffer == 0 {
            0
        } else {
            h.read(buffer + 0x9c)
        };
        let n = i32::from(h.read::<i16>(groups + 2));
        let mut rec = groups + 0x10;
        if h.read::<u8>(groups + 1) != 0 {
            break;
        }
        groups = rec.wrapping_add((n * 0x14).cast_unsigned());
        if p.single {
            single_target = Some(target(h, datas[0], Some(0)));
        }
        let mut k = 0i32;
        if n > 0 {
            let mut tables = [table, table + 0x10, table + 0x20];
            while k < n {
                let t = match single_target {
                    Some(t) => Target {
                        base: k.cast_unsigned(),
                        ..t
                    },
                    None => pick(h, frame, &p, rec, depths),
                };
                if p.single {
                    single_target = Some(t);
                }
                triangle(
                    h,
                    frame,
                    &p,
                    rec,
                    k.cast_unsigned(),
                    t,
                    colour_buffer,
                    tables,
                );
                for t in &mut tables {
                    *t = t.wrapping_add(0x30);
                }
                table = tables[0];
                rec = rec.wrapping_add(0x14);
                k += 1;
            }
        }
        close_group(h, &p, n.cast_unsigned());
        if h.read::<u8>(groups + 1) == 0xff {
            break;
        }
    }
    let bands_n = if p.single { 1 } else { 4 };
    let matrix = part + 0xb0;
    for j in 0..bands_n {
        let r = datas[j];
        h.call_sym(COPY_MATRIX, Conv::Cdecl, &[r, matrix]);
        for off in [0xc, 0x1c, 0x2c] {
            h.write(r.wrapping_add(off), 0u32);
        }
        h.write(r.wrapping_add(0x3c), 1f32.to_bits());
        let low = image_f32(h, BOUND_LOW);
        let high = image_f32(h, BOUND_HIGH);
        for k in 0..3 {
            h.write(r.wrapping_add(0x98 + 4 * k), low);
            h.write(r.wrapping_add(0xa4 + 4 * k), high);
        }
        let lo = f32_at(h, slot(frame, LOW) + 4 * j as u32);
        let hi = f32_at(h, slot(frame, HIGH) + 4 * j as u32);
        let mut depth = store(mul(add(hi, lo), half));
        let magic = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
        if magic == 0x46 || h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x23a {
            if !p.swap {
                depth = 1f32.to_bits();
            } else if !p.from_table {
                depth = image_f32(h, MINUS_ONE);
            }
        }
        let d = store(load(depth));
        h.call_sym(UPDATE_PARAMETER, Conv::Cdecl, &[instances[j], d, 0, 0]);
    }
    matrix
}
