//! M4 list `h4-glue-other`, agent h4z: the particle fill-buffer job
//! `JobSchedule_pppPart_fillBuffer_drawMdl` (`0x0071f320`), the model
//! parts' primitives: the set-up, the groups and the four triangle kinds
//! (0..3); the quad kinds (4..7) are in [`crate::h4z_quad`].
//!
//! It runs on the job workers (`JobSchedule_pppPart_fillBuffer`,
//! `docs/hooking.md` "Threads"); the shim hands it a zeroed per-call frame
//! of 83 KB, as the original's alloca (the job workers commit 128 KB).
//!
//! A group's records fill one 0x6c-byte segment (the group's number in the
//! model) of a render data's geometry (`[rd + 0x94]`): `+4` the vertex
//! count, `+8` the index count, `+0xc` the vertices (3 `f32`), `+0x10` the
//! normals, `+0x14` the colours (4 `f32`), `+0x18` the texture coordinates
//! (2 `f32`), `+0x1c` the `u16` indices. The render data's `+0xc0` byte is
//! the instance mode: 0 one render data, 1 four depth bands (the render
//! data `part + 0xc0..`, by the primitive's mean depth), 2 2000 triangles
//! per render data; others act as 0 for the primitives and as 2 at the end.

use ffx_bridge::x87::{add, div, load, mul, store, sub};
use ffx_graphic::host::{Conv, Host};

/// `graphicLevelGetNameHash()` (cdecl).
pub const LEVEL_HASH: u32 = 0x0064_1da0;
/// `MagicFile_GetMagicId()` (cdecl).
pub const MAGIC_ID: u32 = 0x009d_a3b0;
/// `ModelBuffer_GetBufferByIndex(index)` (cdecl).
pub const BUFFER_BY_INDEX: u32 = 0x0072_c190;
/// `get_ptr32_func(p)` (cdecl).
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `FFXVu0MulMatrix(dst, a, b)` (cdecl).
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `FFXVu0ApplyMatrix(dst, m, v)` (cdecl).
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `FFXVu0MulVector(dst, a, b)` (cdecl).
pub const MUL_VECTOR: u32 = 0x006e_d5d0;
/// `pppMdl_projectPositions_007161f0(part, job)` (cdecl).
pub const PROJECT_POSITIONS: u32 = 0x0071_61f0;
/// `pppMdl_copyPositions_007163e0(part, job)` (cdecl).
pub const COPY_POSITIONS: u32 = 0x0071_63e0;
/// `memset` through its thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `useShaderPointLight()` (cdecl).
pub const USE_POINT_LIGHT: u32 = 0x0071_e6f0;
/// `pppMdl_pointLightColours_0071e820(rd, segment + 1, job, group, n)`
/// (cdecl).
pub const POINT_LIGHT_COLOURS: u32 = 0x0071_e820;
/// `graphicVFXUpdateParameterOnly(instance, depth, 0, 0)` (cdecl).
pub const UPDATE_PARAMETER: u32 = 0x0064_5a50;
/// `graphicUseOptimizedVFXShader(instance, opaque)` (cdecl).
pub const USE_OPTIMIZED: u32 = 0x0064_4ac0;
/// `g_ScreenMatrix` (`0x02311480`).
pub const SCREEN_MATRIX: u32 = 0x0231_1480;
/// `f64` constants: 1/4096 (`0x00b50190`, texture coordinates and
/// normals), 0.33333 (`0x00b501f0`, a triangle's mean depth), 0.25
/// (`0x00b34f90`), 0 (`0x00b0db90`), 3 (`0x00b2c5c8`) and 4 (`0x00b43d80`,
/// the bands), 0.5 (`0x00b92220`, a band's middle), 1e-5 (`0x00b501e8`,
/// the depth's w), 0.1 (`0x00b921f0`), 1000 (`0x00b0dba0`), 1.333333
/// (`0x00b50208`, magic 0x9a's x scale).
pub const INV_4096: u32 = 0x00b5_0190;
/// See [`INV_4096`].
pub const THIRD: u32 = 0x00b5_01f0;
/// See [`INV_4096`].
pub const QUARTER: u32 = 0x00b3_4f90;
/// See [`INV_4096`].
pub const ZERO: u32 = 0x00b0_db90;
/// See [`INV_4096`].
pub const THREE: u32 = 0x00b2_c5c8;
/// See [`INV_4096`].
pub const FOUR: u32 = 0x00b4_3d80;
/// See [`INV_4096`].
pub const HALF: u32 = 0x00b9_2220;
/// See [`INV_4096`].
pub const MIN_W: u32 = 0x00b5_01e8;
/// See [`INV_4096`].
pub const TENTH: u32 = 0x00b9_21f0;
/// See [`INV_4096`].
pub const THOUSAND: u32 = 0x00b0_dba0;
/// See [`INV_4096`].
pub const X_SCALE: u32 = 0x00b5_0208;
/// `f32` constants: 1e10 (`0x00b50228`) and -1e10 (`0x00b5022c`, the depth
/// range's start), 0.5 (`0x00b92194`, the opaque test), -1 (`0x00b92340`),
/// -100000 (`0x00b368b8`) and 100000 (`0x00b10f50`, the bounds).
pub const RANGE_MIN: u32 = 0x00b5_0228;
/// See [`RANGE_MIN`].
pub const RANGE_MAX: u32 = 0x00b5_022c;
/// See [`RANGE_MIN`].
pub const OPAQUE: u32 = 0x00b9_2194;
/// See [`RANGE_MIN`].
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// See [`RANGE_MIN`].
pub const BOUND_LOW: u32 = 0x00b3_68b8;
/// See [`RANGE_MIN`].
pub const BOUND_HIGH: u32 = 0x00b1_0f50;

/// Bytes of [`draw_mdl`]'s frame: the original's `ebp - 0x14a20` up to its
/// cookie. Offsets in it are written as the original's `ebp` offsets
/// ([`slot`]).
pub const FRAME: u32 = 0x1_4a1c;
const BASE: u32 = 0x1_4a20;

/// The frame address of the original's `[ebp - off]`.
#[must_use]
pub fn slot(frame: u32, off: u32) -> u32 {
    frame.wrapping_add(BASE.wrapping_sub(off))
}

/// The position callees' part: they read its `+0x1c` (the model).
pub const FAKE_PART: u32 = 0x1_4a20;
/// The position callees' job (0x50 bytes): `+0` the part's flags, `+4` the
/// positions ([`POSITIONS`]), `+8` the part's `+0x1e` byte, `+0x20` its
/// `+0x20` matrix, `+0x24`/`+0x28`/`+0x2c` its `+0x190`/`+0x150`/`+0x110`,
/// `+0x38` its `+0x14` positions (or the model buffer's), `+0x3c` its
/// `+0x100` normals, `+0x44` its `+0x60` matrix (or 0), `+0x4d` its
/// `+0x1d0` byte.
pub const JOB: u32 = 0x1_4910;
/// The 5120 transformed positions (16 bytes each).
pub const POSITIONS: u32 = 0x1_4898;
/// The positions' depths (512 `f32`, mode 1).
pub const DEPTHS: u32 = 0x898;
/// The screen matrix times the part's.
pub const MATRIX: u32 = 0x98;
/// A position through [`MATRIX`].
pub const VECTOR: u32 = 0x54;
/// The bands' middle depths (`f32` x 4).
pub const BAND_DEPTH: u32 = 0x44;
/// The bands' low depths (`f32` x 4).
pub const LOW: u32 = 0x34;
/// The bands' high depths (`f32` x 4).
pub const HIGH: u32 = 0x24;
/// The part's colour scale (`+0xb0`, 4 `f32`).
pub const SCALE: u32 = 0x14;
/// The render data `part + 0xc0..` (modes other than 0).
const RDS: u32 = 0x1_498c;

/// What stays the same for the whole part.
pub(crate) struct Mdl {
    pub part: u32,
    pub frame: u32,
    /// The render data's `+0xc0` byte, sign-extended.
    pub mode: i32,
    pub rds: [u32; 4],
    /// The model buffer, or 0.
    pub mb: u32,
    /// The positions are `f32` (16-byte stride, `floats`), else `i16`
    /// triples (`shorts`, the part's `+0x14`).
    pub float_src: bool,
    pub floats: u32,
    pub shorts: u32,
    /// The part's `+0x100`: `i16` normal triples.
    pub normals: u32,
    pub inv4096: f64,
    pub third: f64,
    /// The part's `+0x104`.
    pub flag104: u32,
    /// `MagicFile_GetMagicId()` at the start.
    pub magic0: u32,
}

/// What a group sets.
pub(crate) struct Group {
    pub header: u32,
    /// The first record.
    pub recs: u32,
    /// The segment's offset in the geometry (`0x6c` x its number).
    pub seg: u32,
    /// The segment's number + 1.
    pub seg1: u32,
    /// The model buffer's texture coordinates, colours and vertices for
    /// the segment (`+0x24`, `+0x9c`, `+0x114`), or 0.
    pub uvbuf: u32,
    pub colbuf: u32,
    pub verts: u32,
    /// The texture offsets (`f32`).
    pub u_off: u32,
    pub v_off: u32,
    /// The header's `+8` & 0x3fff.
    pub code: u32,
}

/// The mutable state the original keeps in its frame.
pub(crate) struct State {
    /// The render data last filled (`[ebp-0x14944]`).
    pub cur: u32,
    /// Every colour so far is at most 0.5 (`[ebp-0x14950]`).
    pub opaque: u32,
}

pub(crate) fn f32_at(h: &impl Host, at: u32) -> f64 {
    load(h.read(at))
}

pub(crate) fn f64_at(h: &impl Host, sym: u32) -> f64 {
    f64::from_bits(h.read(h.rebase(sym)))
}

/// An `f32` as `fld dword` then `fstp dword` (a signalling NaN quieted).
pub(crate) fn copy_f32(h: &mut impl Host, from: u32, to: u32) {
    let v = store(f32_at(h, from));
    h.write(to, v);
}

/// The segment of render data `rd` the group fills.
pub(crate) fn seg(h: &impl Host, rd: u32, g: &Group) -> u32 {
    h.read::<u32>(rd.wrapping_add(0x94)).wrapping_add(g.seg)
}

/// `n` floats from `from` to `to`.
pub(crate) fn copy_floats(h: &mut impl Host, from: u32, to: u32, n: u32) {
    for i in 0..n {
        copy_f32(h, from.wrapping_add(4 * i), to.wrapping_add(4 * i));
    }
}

/// The corners' positions (the `u16` indices at `rec + idx`) into
/// `dst` (12 bytes each): from the `f32` positions or the `i16` triples.
pub(crate) fn corners(h: &mut impl Host, m: &Mdl, rec: u32, idx: &[u32], dst: u32) {
    for (i, &o) in (0u32..).zip(idx) {
        let p = u32::from(h.read::<u16>(rec + o));
        let to = dst.wrapping_add(0xc * i);
        if m.float_src {
            copy_floats(h, m.floats.wrapping_add(16 * p), to, 3);
        } else {
            for k in 0..3 {
                let s = h.read::<i16>(m.shorts.wrapping_add(6 * p + 2 * k));
                h.write(to.wrapping_add(4 * k), f32::from(s).to_bits());
            }
        }
    }
}

/// The colours (`n` corners) into `dst`: the colour buffer's (`src`, 4 `f32`
/// each) or the record's bytes; times the part's scale; any above 0.5
/// clears the opaque flag.
pub(crate) fn colours(
    h: &mut impl Host,
    m: &Mdl,
    st: &mut State,
    rec: u32,
    src: Option<u32>,
    dst: u32,
    n: u32,
) {
    match src {
        Some(src) => copy_floats(h, src, dst, 4 * n),
        None => {
            for j in 0..4 * n {
                let b = h.read::<u8>(rec + j);
                h.write(dst.wrapping_add(4 * j), f32::from(b).to_bits());
            }
        }
    }
    let scale = slot(m.frame, SCALE);
    for i in 0..n {
        let v = dst.wrapping_add(0x10 * i);
        h.call_sym(MUL_VECTOR, Conv::Cdecl, &[v, scale, v]);
    }
    if st.opaque != 0 {
        let half = f32_at(h, h.rebase(OPAQUE));
        for j in 0..4 * n {
            if half < f32_at(h, dst.wrapping_add(4 * j)) {
                st.opaque = 0;
                break;
            }
        }
    }
}

/// The triangle's three indices (base `b`): `3b`, `3b + 1`, `3b + 2` as
/// the original orders them (`+2` holds `3b + 2`, `+4` holds `3b + 1`).
pub(crate) fn tri_indices(h: &mut impl Host, indices: u32, b: u32) {
    let i = b.wrapping_mul(3);
    let at = indices.wrapping_add(i.wrapping_mul(2));
    h.write(at.wrapping_add(2), i.wrapping_add(2) as u16);
    h.write(at, i as u16);
    h.write(at.wrapping_add(4), i.wrapping_add(1) as u16);
}

/// The texture coordinates of `n` corners into `dst`: from the model
/// buffer's (`src`, `f32` pairs: u + u offset, 1 - (v + v offset)) or the
/// record's `u16` pairs at `rec` (`i16` when `signed`, less `shift`: (u +
/// offset) / 4096 and 1 - (v + offset) / 4096, the sums rounded to `f32`).
#[allow(clippy::too_many_arguments)] // the record's shape
pub(crate) fn uvs(
    h: &mut impl Host,
    m: &Mdl,
    g: &Group,
    rec: u32,
    src: Option<u32>,
    dst: u32,
    n: u32,
    signed: bool,
    shift: i32,
) {
    let uo = load(g.u_off);
    let vo = load(g.v_off);
    for i in 0..n {
        let (u, v) = match src {
            Some(src) => {
                let s = src.wrapping_add(8 * i);
                let u = add(f32_at(h, s), uo);
                let v = sub(1.0, add(f32_at(h, s.wrapping_add(4)), vo));
                (u, v)
            }
            None => {
                let word = |at: u32| {
                    let w = h.read::<u16>(at);
                    let v = if signed {
                        i32::from(w.cast_signed())
                    } else {
                        i32::from(w)
                    };
                    f64::from(v - shift)
                };
                let a = word(rec + 4 * i);
                let b = word(rec + 4 * i + 2);
                let u = mul(load(store(add(a, uo))), m.inv4096);
                let v = sub(1.0, mul(load(store(add(b, vo))), m.inv4096));
                (u, v)
            }
        };
        let to = dst.wrapping_add(8 * i);
        h.write(to, store(u));
        h.write(to.wrapping_add(4), store(v));
    }
}

/// The normals of `n` corners into `dst` (12 bytes each): with the flags'
/// 0x40 the part's `i16` triples (the `u16` indices at `rec`) / 4096, else
/// (0, 0, 1).
pub(crate) fn normals(h: &mut impl Host, m: &Mdl, rec: u32, dst: u32, n: u32) {
    let lit = h.read::<u8>(slot(m.frame, JOB)) & 0x40 != 0;
    for i in 0..n {
        let to = dst.wrapping_add(0xc * i);
        if lit {
            let p = u32::from(h.read::<u16>(rec + 2 * i));
            for k in 0..3 {
                let s = h.read::<i16>(m.normals.wrapping_add(6 * p + 2 * k));
                h.write(to.wrapping_add(4 * k), store(mul(f64::from(s), m.inv4096)));
            }
        } else {
            h.write(to, 0f32.to_bits());
            h.write(to.wrapping_add(4), 0f32.to_bits());
            h.write(to.wrapping_add(8), 1f32.to_bits());
        }
    }
}

/// The depth band holding `mean` (low <= mean <= high, the first), or 4.
pub(crate) fn band(h: &impl Host, frame: u32, mean: f64) -> usize {
    for j in 0..4 {
        let lo = f32_at(h, slot(frame, LOW) + 4 * j);
        let hi = f32_at(h, slot(frame, HIGH) + 4 * j);
        if lo <= mean && hi >= mean {
            return j as usize;
        }
    }
    4
}

/// A triangle's mean depth (`f32`): `(d[+0xe] + d[+0xc] + d[+0x10]) x
/// 0.33333`.
fn tri_mean(h: &impl Host, m: &Mdl, rec: u32) -> f64 {
    let d = |o: u32| {
        let i = u32::from(h.read::<u16>(rec + o));
        f32_at(h, slot(m.frame, DEPTHS).wrapping_add(4 * i))
    };
    load(store(mul(add(add(d(0xe), d(0xc)), d(0x10)), m.third)))
}

/// The render data a mode-2 primitive `k` (of `per` per render data)
/// goes to: the `k / per`th after `part + 0xc0` in the original's frame
/// (past the fourth: its other locals, as the original reads them).
pub(crate) fn split_rd(h: &impl Host, m: &Mdl, q: i32) -> u32 {
    if (0..4).contains(&q) {
        m.rds[q.cast_unsigned() as usize]
    } else {
        h.read(slot(m.frame, RDS).wrapping_add((q << 2).cast_unsigned()))
    }
}

/// Where triangle `k` goes: (render data, slot). Mode 1: the band of its
/// mean depth (or the first), at the segment's index count / 3, both
/// counts then + 3; mode 2: 2000 per render data; else the current one at
/// `k`.
fn tri_target(
    h: &mut impl Host,
    m: &Mdl,
    g: &Group,
    st: &mut State,
    rec: u32,
    k: i32,
) -> (u32, u32) {
    match m.mode {
        1 => {
            let mean = tri_mean(h, m, rec);
            let j = band(h, m.frame, mean);
            let rd = m.rds[if j == 4 { 0 } else { j }];
            let s = seg(h, rd, g);
            let b = h.read::<u32>(s.wrapping_add(8)) / 3;
            let s = seg(h, rd, g);
            let n: u32 = h.read(s.wrapping_add(8));
            h.write(s.wrapping_add(8), n.wrapping_add(3));
            let s = seg(h, rd, g);
            let n: u32 = h.read(s.wrapping_add(4));
            h.write(s.wrapping_add(4), n.wrapping_add(3));
            st.cur = rd;
            (rd, b)
        }
        2 => {
            let q = k.wrapping_mul(3) / 6000;
            let rd = split_rd(h, m, q);
            st.cur = rd;
            (rd, k.wrapping_sub(q.wrapping_mul(2000)).cast_unsigned())
        }
        _ => (st.cur, k.cast_unsigned()),
    }
}

/// After a mode-1 group of `n` primitives: each band's segment gets the
/// indices and vertices past what it filled cleared (`ipv` indices and
/// `vpv` vertices per primitive, `isize`/`vsize` bytes each) and its
/// counts set to the group's.
#[allow(clippy::too_many_arguments)] // the two buffers' shapes
pub(crate) fn close_bands(
    h: &mut impl Host,
    g: &Group,
    rd: u32,
    n: i32,
    ipv: u32,
    vpv: u32,
    vbytes: u32,
) {
    let s = seg(h, rd, g);
    let filled: u32 = h.read(s.wrapping_add(8));
    let at = h
        .read::<u32>(s.wrapping_add(0x1c))
        .wrapping_add(filled.wrapping_mul(2));
    let size = n
        .cast_unsigned()
        .wrapping_sub(filled / ipv)
        .wrapping_mul(2 * ipv);
    h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, size]);
    let s = seg(h, rd, g);
    let filled: u32 = h.read(s.wrapping_add(4));
    let at = h
        .read::<u32>(s.wrapping_add(0xc))
        .wrapping_add(filled.wrapping_mul(0xc));
    let size = n
        .cast_unsigned()
        .wrapping_sub(filled / vpv)
        .wrapping_mul(vbytes);
    h.call_sym(MEMSET, Conv::Cdecl, &[at, 0, size]);
    let s = seg(h, rd, g);
    h.write(s.wrapping_add(4), n.wrapping_mul(vpv.cast_signed()));
    let s = seg(h, rd, g);
    h.write(s.wrapping_add(8), n.wrapping_mul(ipv.cast_signed()));
}

/// The group's primitive count (`i16` at `+2`, read each time as the
/// original does).
pub(crate) fn count(h: &impl Host, g: &Group) -> i32 {
    i32::from(h.read::<i16>(g.header + 2))
}

/// `pppMdl_pointLightColours_0071e820` when the flags' 0x40 is set and
/// `useShaderPointLight()` says no.
pub(crate) fn point_light(h: &mut impl Host, m: &Mdl, g: &Group, rd: u32, n: u32) {
    if h.read::<u8>(slot(m.frame, JOB)) & 0x40 != 0
        && h.call_sym(USE_POINT_LIGHT, Conv::Cdecl, &[]) == 0
    {
        let job = slot(m.frame, JOB);
        h.call_sym(
            POINT_LIGHT_COLOURS,
            Conv::Cdecl,
            &[rd, g.seg1, job, g.header, n],
        );
    }
}

/// [`point_light`] for each band: the segment's vertex count.
fn point_light_band(h: &mut impl Host, m: &Mdl, g: &Group, rd: u32) {
    if h.read::<u8>(slot(m.frame, JOB)) & 0x40 != 0
        && h.call_sym(USE_POINT_LIGHT, Conv::Cdecl, &[]) == 0
    {
        let n = h.read::<u32>(seg(h, rd, g).wrapping_add(4));
        let job = slot(m.frame, JOB);
        h.call_sym(
            POINT_LIGHT_COLOURS,
            Conv::Cdecl,
            &[rd, g.seg1, job, g.header, n],
        );
    }
}

/// Kind 0 (0x14-byte records: colours, three corners): flat triangles.
/// Kind 1 (0x1c: + three normals): lit triangles. Returns the next header.
fn kind_flat(h: &mut impl Host, m: &Mdl, g: &Group, st: &mut State, lit: bool) -> u32 {
    let size = if lit { 0x1c } else { 0x14 };
    let mut rec = g.recs;
    let mut k = 0i32;
    while k < count(h, g) {
        let (rd, b) = tri_target(h, m, g, st, rec, k);
        if g.verts == 0 {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0xc))
                .wrapping_add(b.wrapping_mul(0x24));
            corners(h, m, rec, &[0xc, 0xe, 0x10], to);
        }
        let to = h
            .read::<u32>(seg(h, rd, g).wrapping_add(0x14))
            .wrapping_add(b.wrapping_mul(0x30));
        // kind 0 reads the colour buffer at the record's number, kind 1 at
        // the slot's
        let at = if lit { b } else { k.cast_unsigned() };
        let src = (g.colbuf != 0).then(|| g.colbuf.wrapping_add(at.wrapping_mul(0x30)));
        colours(h, m, st, rec, src, to, 3);
        if lit {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0x10))
                .wrapping_add(b.wrapping_mul(0x24));
            normals(h, m, rec + 0x14, to, 3);
        }
        let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
        tri_indices(h, idx, b);
        rec = rec.wrapping_add(size);
        k += 1;
    }
    if m.mode == 1 {
        for j in 0..4 {
            let rd = m.rds[j];
            if lit {
                point_light_band(h, m, g, rd);
            }
            close_bands(h, g, rd, count(h, g), 3, 3, 0x24);
        }
    } else if lit {
        let n = count(h, g).wrapping_mul(3).cast_unsigned();
        point_light(h, m, g, st.cur, n);
    }
    rec
}

/// Magic 0x9a's x scale of a triangle's three corners for the five
/// texture codes.
fn magic_x_scale(h: &mut impl Host, g: &Group, vtx: u32) {
    if h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x9a
        && matches!(g.code, 0x37c0 | 0x3780 | 0x3800 | 0x3840 | 0x3880)
    {
        let c = f64_at(h, X_SCALE);
        for i in 0..3 {
            let at = vtx.wrapping_add(0xc * i);
            let v = store(mul(f32_at(h, at), c));
            h.write(at, v);
        }
    }
}

/// The screen-space winding of three corners (`u16` indices at `rec +
/// idx`) in the projected positions: `false` (cull) when not negative.
pub(crate) fn front(h: &impl Host, m: &Mdl, rec: u32, idx: [u32; 3]) -> bool {
    let pos: u32 = h.read(slot(m.frame, JOB) + 4);
    let p = |o: u32| pos.wrapping_add(16 * u32::from(h.read::<u16>(rec + o)));
    let (p0, p1, p2) = (p(idx[0]), p(idx[1]), p(idx[2]));
    let a = load(store(sub(f32_at(h, p0), f32_at(h, p1))));
    let b = load(store(sub(f32_at(h, p0 + 4), f32_at(h, p1 + 4))));
    let c = load(store(sub(f32_at(h, p2), f32_at(h, p1))));
    let d = load(store(sub(f32_at(h, p2 + 4), f32_at(h, p1 + 4))));
    let cross = load(store(sub(mul(d, a), mul(c, b))));
    // fldz; fcompp; test ah, 0x41; jp: drawn when 0 > cross or unordered
    cross < 0.0 || cross.is_nan()
}

/// Kind 2 (0x20-byte records: colours, three corners, three `u16`
/// texture coordinate pairs at `+0x14`): textured triangles, culled by
/// winding with the flags' 0x10. Returns the next header.
fn kind_textured(h: &mut impl Host, m: &Mdl, g: &Group, st: &mut State) -> u32 {
    let mut rec = g.recs;
    let mut k = 0i32;
    while k < count(h, g) {
        let (rd, b) = tri_target(h, m, g, st, rec, k);
        if g.verts == 0 {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0xc))
                .wrapping_add(b.wrapping_mul(0x24));
            corners(h, m, rec, &[0xc, 0xe, 0x10], to);
        }
        let vtx = h
            .read::<u32>(seg(h, rd, g).wrapping_add(0xc))
            .wrapping_add(b.wrapping_mul(0x24));
        magic_x_scale(h, g, vtx);
        if h.read::<u8>(slot(m.frame, JOB)) & 0x10 != 0 && !front(h, m, rec, [0xc, 0xe, 0x10]) {
            let at = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0x1c))
                .wrapping_add(b.wrapping_mul(6));
            h.write(at, 0u32);
            h.write(at.wrapping_add(4), 0u16);
        } else {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0x14))
                .wrapping_add(b.wrapping_mul(0x30));
            let src = (g.colbuf != 0)
                .then(|| g.colbuf.wrapping_add(k.cast_unsigned().wrapping_mul(0x30)));
            colours(h, m, st, rec, src, to, 3);
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0x18))
                .wrapping_add(b.wrapping_mul(0x18));
            let src =
                (g.uvbuf != 0).then(|| g.uvbuf.wrapping_add(k.cast_unsigned().wrapping_mul(0x18)));
            uvs(h, m, g, rec + 0x14, src, to, 3, false, 0);
            let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
            tri_indices(h, idx, b);
        }
        rec = rec.wrapping_add(0x20);
        k += 1;
    }
    if m.mode == 1 {
        for rd in m.rds {
            close_bands(h, g, rd, count(h, g), 3, 3, 0x24);
        }
    }
    rec
}

/// Kind 3 (0x28-byte records: kind 2's, three normals at `+0x20`): lit
/// textured triangles, each at its own number in the current render data
/// whatever the mode, counts left alone. Returns the next header.
fn kind_textured_lit(h: &mut impl Host, m: &Mdl, g: &Group, st: &mut State) -> u32 {
    let rd = st.cur;
    let mut rec = g.recs;
    let mut k = 0i32;
    while k < count(h, g) {
        let b = k.cast_unsigned();
        if g.verts == 0 {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0xc))
                .wrapping_add(b.wrapping_mul(0x24));
            corners(h, m, rec, &[0xc, 0xe, 0x10], to);
        }
        let to = h
            .read::<u32>(seg(h, rd, g).wrapping_add(0x14))
            .wrapping_add(b.wrapping_mul(0x30));
        let src = (g.colbuf != 0).then(|| g.colbuf.wrapping_add(b.wrapping_mul(0x30)));
        colours(h, m, st, rec, src, to, 3);
        let to = h
            .read::<u32>(seg(h, rd, g).wrapping_add(0x18))
            .wrapping_add(b.wrapping_mul(0x18));
        let src = (g.uvbuf != 0).then(|| g.uvbuf.wrapping_add(b.wrapping_mul(0x18)));
        uvs(h, m, g, rec + 0x14, src, to, 3, false, 0);
        let to = h
            .read::<u32>(seg(h, rd, g).wrapping_add(0x10))
            .wrapping_add(b.wrapping_mul(0x24));
        normals(h, m, rec + 0x20, to, 3);
        let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
        tri_indices(h, idx, b);
        rec = rec.wrapping_add(0x28);
        k += 1;
    }
    let n = count(h, g).wrapping_mul(3).cast_unsigned();
    point_light(h, m, g, rd, n);
    rec
}

/// The model's positions through [`MATRIX`] into [`VECTOR`], their depths
/// (z / w) into [`DEPTHS`]; returns their (min, max) (`f32`), starting
/// from (1e10, -1e10).
fn depths(h: &mut impl Host, m: &Mdl, model: u32) -> (u32, u32) {
    let frame = m.frame;
    let mut min = store(f32_at(h, h.rebase(RANGE_MIN)));
    let mut max = store(f32_at(h, h.rebase(RANGE_MAX)));
    let v = slot(frame, VECTOR);
    let mat = slot(frame, MATRIX);
    let mut i = 0u32;
    let mut src = m.shorts;
    loop {
        let p = h.call_sym(GET_PTR32, Conv::Cdecl, &[model]);
        if i.cast_signed() >= i32::from(h.read::<i16>(p + 0x12)) {
            break;
        }
        if m.mb != 0 {
            let at = h.read::<u32>(m.mb + 0x14).wrapping_add(16 * i);
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, mat, at]);
        } else {
            for k in 0..3 {
                let s = h.read::<i16>(src.wrapping_add(2 * k));
                h.write(v + 4 * k, f32::from(s).to_bits());
            }
            h.write(v + 0xc, 1f32.to_bits());
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[v, mat, v]);
            src = src.wrapping_add(6);
        }
        let d = store(div(f32_at(h, v + 8), f32_at(h, v + 0xc)));
        h.write(slot(frame, DEPTHS).wrapping_add(4 * i), d);
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

/// The four depth bands between min and max (as `drawMdlDtt`'s).
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

/// A render data's matrices and bounds: the flags into `+0xb0`, the part's
/// `+0x1d4` matrix times the job's `+0x20` (w column 0 0 0 1), and with a
/// job `+0x44` matrix that one too at `+0x40` (`+0x80` 1, else 0).
fn finish_rd(h: &mut impl Host, m: &Mdl, rd: u32, code: u32, magic: bool) {
    let job = slot(m.frame, JOB);
    let flags: u32 = h.read(job);
    h.write(rd.wrapping_add(0xb0), flags);
    let a = h.read::<u32>(job + 0x20);
    h.call_sym(MUL_MATRIX, Conv::Cdecl, &[rd, m.part + 0x1d4, a]);
    for off in [0xc, 0x1c, 0x2c] {
        h.write(rd.wrapping_add(off), 0f32.to_bits());
    }
    h.write(rd.wrapping_add(0x3c), 1f32.to_bits());
    let b: u32 = h.read(job + 0x44);
    if b == 0 {
        h.write(rd.wrapping_add(0x80), 0u32);
    } else {
        if magic
            && h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x207
            && matches!(code, 0x35c0 | 0x3680 | 0x3760 | 0x37a0 | 0x37c0)
        {
            let v = store(sub(f32_at(h, b + 0x38), f64_at(h, TENTH)));
            h.write(b + 0x38, v);
        }
        let b: u32 = h.read(job + 0x44);
        h.call_sym(
            MUL_MATRIX,
            Conv::Cdecl,
            &[rd.wrapping_add(0x40), m.part + 0x1d4, b],
        );
        h.write(rd.wrapping_add(0x80), 1u32);
    }
}

/// The bounds +-100000 (the image's `f32`s).
fn bounds(h: &mut impl Host, rd: u32) {
    let lo = store(f32_at(h, h.rebase(BOUND_LOW)));
    let hi = store(f32_at(h, h.rebase(BOUND_HIGH)));
    for k in 0..3 {
        h.write(rd.wrapping_add(0x98 + 4 * k), lo);
    }
    for k in 0..3 {
        h.write(rd.wrapping_add(0xa4 + 4 * k), hi);
    }
}

/// The part's depth: the matrix's `[3][2] / [3][3]` (-1 when `|[3][3]|` is
/// not above 1e-5).
fn part_depth(h: &impl Host, frame: u32) -> u32 {
    let z = f32_at(h, slot(frame, 0x5c));
    let a = load(store(z.abs()));
    if a > f64_at(h, MIN_W) {
        store(div(f32_at(h, slot(frame, 0x60)), z))
    } else {
        store(f32_at(h, h.rebase(MINUS_ONE)))
    }
}

/// Magics 0x133, 0x25d, 0x233 and 0x264 (one call each until one
/// matches) with the job's `+8` byte 'D'.
fn magic_flat(h: &mut impl Host, frame: u32) -> bool {
    let mut hit = false;
    for id in [0x133, 0x25d, 0x233, 0x264] {
        if h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == id {
            hit = true;
            break;
        }
    }
    hit && h.read::<u8>(slot(frame, JOB) + 8) == 0x44
}

/// `JobSchedule_pppPart_fillBuffer_drawMdl(part)` (`0x0071f320`, cdecl;
/// `frame` see [`FRAME`]) -> the last callee's `eax` (mode 0:
/// `graphicUseOptimizedVFXShader`'s; else the mode, or render data 0's
/// `u16` `+0xc4` when that ends the loop early).
///
/// The part (`+0x18` the model: `i16` `+0x10` primitives in all,
/// groups at `+[+4]`; `+0x14` its `i16` positions; `+0xc0..` the render
/// data; `+0xf0..` their instances; `+0xb0` the colour scale; `+0x10` the
/// flags: 0x10 projected positions (cull), 0x40 lit, 0x400 `f32`
/// positions) is filled group by group (16-byte header: byte `+1` the
/// kind, `i16` `+2` the count, `+8` the texture page: offsets from the
/// part's `+0xa0`/`+0xa4` scroll, the low 14 bits a code).
#[allow(clippy::too_many_lines)] // the original's set-up, groups and ends
pub fn draw_mdl(h: &mut impl Host, frame: u32, part: u32) -> u32 {
    let hash = h.call_sym(LEVEL_HASH, Conv::Cdecl, &[]);
    let rd0: u32 = h.read(part + 0xc0);
    let a0: u32 = h.read(part + 0xa0);
    let a1: u32 = h.read(part + 0xa4);
    let mode = i32::from(h.read::<i8>(rd0.wrapping_add(0xc0)));
    let mut st = State {
        cur: rd0,
        opaque: 1,
    };
    let magic0 = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
    let index = h.read::<u16>(rd0.wrapping_add(0xc2));
    let mb = if index == 0xffff {
        0
    } else {
        h.call_sym(BUFFER_BY_INDEX, Conv::Cdecl, &[u32::from(index)])
    };
    let flag104: u32 = h.read(part + 0x104);
    let mut instances = [0u32; 4];
    for k in 0..4 {
        instances[k as usize] = h.read(part + 0xf0 + 4 * k);
        let s: u32 = h.read(part + 0xb0 + 4 * k);
        h.write(slot(frame, SCALE) + 4 * k, s);
    }
    let mut rds = [0u32; 4];
    if mode != 0 {
        for k in 0..4 {
            rds[k as usize] = h.read(part + 0xc0 + 4 * k);
            h.write(slot(frame, RDS) + 4 * k, rds[k as usize]);
        }
    }
    let model: u32 = h.read(part + 0x18);
    h.write(slot(frame, FAKE_PART) + 0x1c, model);
    let mut remaining = i32::from(h.read::<i16>(model.wrapping_add(0x10)));
    let p = h.call_sym(GET_PTR32, Conv::Cdecl, &[model]);
    let off: u32 = h.read(p.wrapping_add(4));
    let p = h.call_sym(GET_PTR32, Conv::Cdecl, &[model]);
    let mut header = p.wrapping_add(off);
    let job = slot(frame, JOB);
    let flags: u32 = h.read(part + 0x10);
    h.write(job, flags);
    let shorts: u32 = h.read(part + 0x14);
    h.write(job + 0x38, shorts);
    let byte = h.read::<u8>(part + 0x1e);
    h.write(job + 8, byte);
    h.write(job + 0x20, part + 0x20);
    let second = if h.read::<u8>(part + 1) == 0 {
        h.write(job + 0x44, 0u32);
        part + 0x20
    } else {
        h.write(job + 0x44, part + 0x60);
        part + 0x60
    };
    let screen = h.rebase(SCREEN_MATRIX);
    h.call_sym(
        MUL_MATRIX,
        Conv::Cdecl,
        &[slot(frame, MATRIX), screen, second],
    );
    let normals: u32 = h.read(part + 0x100);
    h.write(job + 0x3c, normals);
    h.write(job + 0x2c, part + 0x110);
    h.write(job + 0x28, part + 0x150);
    h.write(job + 0x24, part + 0x190);
    let byte = h.read::<u8>(part + 0x1d0);
    h.write(job + 0x4d, byte);
    h.write(job + 4, slot(frame, POSITIONS));
    let mut m = Mdl {
        part,
        frame,
        mode,
        rds,
        mb,
        float_src: false,
        floats: 0,
        shorts,
        normals,
        inv4096: 0.0,
        third: 0.0,
        flag104,
        magic0,
    };
    if mode == 1 {
        let (min, max) = depths(h, &m, model);
        bands(h, frame, min, max);
    }
    let flags = h.read::<u8>(job);
    let float_src = if flags & 0x40 == 0 && flags & 0x10 != 0 {
        if mb != 0 {
            let f: u32 = h.read(job);
            h.write(job, f | 0x400);
            let p: u32 = h.read(mb + 0x14);
            h.write(job + 0x38, p);
        }
        h.call_sym(
            PROJECT_POSITIONS,
            Conv::Cdecl,
            &[slot(frame, FAKE_PART), job],
        );
        0
    } else if mb == 0 && h.read::<u32>(job) & 0x400 == 0 {
        h.call_sym(COPY_POSITIONS, Conv::Cdecl, &[slot(frame, FAKE_PART), job]);
        1
    } else {
        flag104
    };
    let (float_src, floats) = if mb != 0 {
        (1, h.read::<u32>(mb + 0x14))
    } else {
        (float_src, h.read::<u32>(job + 4))
    };
    m.float_src = float_src != 0;
    m.floats = floats;
    let mut code = hash;
    let mut seg1 = 0u32;
    while remaining > 0 {
        m.inv4096 = f64_at(h, INV_4096);
        m.third = f64_at(h, THIRD);
        let (uvbuf, colbuf, verts) = if mb == 0 {
            (0, 0, 0)
        } else {
            let uv = h.read(mb + 0x24 + 4 * seg1);
            let col = h.read(mb + 0x9c + 4 * seg1);
            let v = if mode == 0 {
                h.read(mb + 0x114 + 4 * seg1)
            } else {
                0
            };
            (uv, col, v)
        };
        seg1 += 1;
        let lo: u32 = h.read(header + 8);
        let hi: u32 = h.read(header + 0xc);
        let s = 1u32 << ((lo >> 26) & 0xf);
        let t = 1u32 << (((lo >> 30) | (hi << 2)) & 0xf);
        code = lo & 0x3fff;
        let mut u_off = store(f64::from(((a0 % (s << 16)) >> 4) / s));
        let mut v_off = store(f64::from(((a1 % (t << 16)) >> 4) / t));
        if mb != 0 {
            u_off = store(mul(load(u_off), m.inv4096));
            v_off = store(mul(load(v_off), m.inv4096));
        }
        let g = Group {
            header,
            recs: header + 0x10,
            seg: seg1.wrapping_sub(1).wrapping_mul(0x6c),
            seg1,
            uvbuf,
            colbuf,
            verts,
            u_off,
            v_off,
            code,
        };
        if verts != 0 {
            for off in [0xc, 0x34, 0x50] {
                let at = seg(h, st.cur, &g).wrapping_add(off);
                h.write(at, verts);
            }
        }
        let next = match h.read::<i8>(header + 1) {
            0 => kind_flat(h, &m, &g, &mut st, false),
            1 => kind_flat(h, &m, &g, &mut st, true),
            2 => kind_textured(h, &m, &g, &mut st),
            3 => kind_textured_lit(h, &m, &g, &mut st),
            4 => crate::h4z_quad::kind_flat(h, &m, &g, &mut st, false),
            5 => crate::h4z_quad::kind_flat(h, &m, &g, &mut st, true),
            6 => crate::h4z_quad::kind_textured(h, &m, &g, &mut st, false),
            7 => crate::h4z_quad::kind_textured(h, &m, &g, &mut st, true),
            _ => g.recs,
        };
        remaining -= i32::from(h.read::<i16>(header + 2));
        header = next;
    }
    if mode == 0 {
        let rd = st.cur;
        finish_rd(h, &m, rd, code, true);
        let mut depth = part_depth(h, frame);
        if magic_flat(h, frame) {
            depth = store(f32_at(h, h.rebase(MINUS_ONE)));
        }
        let m1 = h.call_sym(MAGIC_ID, Conv::Cdecl, &[]);
        if (m1 == 0xbc || h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x15f) && code == 0x3740 {
            let d = div(f32_at(h, slot(frame, 0x60)), f32_at(h, slot(frame, 0x5c)));
            depth = store(add(d, f64_at(h, THOUSAND)));
        }
        if hash == 0x1f7
            && (code <= 0x3580 || matches!(code, 0x3780 | 0x3940 | 0x3980) || code >= 0x3a00)
        {
            depth = store(f32_at(h, h.rebase(MINUS_ONE)));
        }
        h.write(slot(frame, BAND_DEPTH), depth);
        bounds(h, rd);
        let inst = instances[0];
        h.call_sym(UPDATE_PARAMETER, Conv::Cdecl, &[inst, depth, 0, 0]);
        return h.call_sym(USE_OPTIMIZED, Conv::Cdecl, &[inst, st.opaque]);
    }
    for j in 0..4u32 {
        let bd = slot(frame, BAND_DEPTH);
        if mode == 1 {
            let hi = f32_at(h, slot(frame, HIGH) + 4 * j);
            let lo = f32_at(h, slot(frame, LOW) + 4 * j);
            h.write(bd + 4 * j, store(mul(add(hi, lo), f64_at(h, HALF))));
            if magic_flat(h, frame) {
                let v = store(f32_at(h, h.rebase(MINUS_ONE)));
                h.write(bd + 4 * j, v);
            }
        } else {
            let n = u32::from(h.read::<u16>(m.rds[0].wrapping_add(0xc4)));
            if j >= n {
                return n;
            }
            h.write(bd, part_depth(h, frame));
        }
        let rd = m.rds[j as usize];
        finish_rd(h, &m, rd, code, false);
        bounds(h, rd);
        let inst = instances[j as usize];
        // junk read: [ebp-0x40..-0x38] never written outside mode 1; 0 (brief 2026-10-24)
        let depth: u32 = h.read(bd + 4 * j);
        h.call_sym(UPDATE_PARAMETER, Conv::Cdecl, &[inst, depth, 0, 0]);
        h.call_sym(USE_OPTIMIZED, Conv::Cdecl, &[inst, st.opaque]);
    }
    mode.cast_unsigned()
}
