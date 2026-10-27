//! M4 list `h4-glue-other`, agent h4z: the quad kinds (4..7) of
//! `JobSchedule_pppPart_fillBuffer_drawMdl` (`0x0071f320`; the rest in
//! [`crate::h4z_mdl`]).
//!
//! A quad record starts with 16 colour bytes and four `u16` corners
//! (`+0x10`). Any mode but 0 sorts quads into the four depth bands (by the
//! mean of the four depths x 0.25); a quad takes 4 vertices and 6
//! indices (`4b, 4b + 2, 4b + 1, 4b + 2, 4b + 3, 4b + 1`).

use ffx_bridge::x87::{add, load, mul, store};
use ffx_graphic::host::{Conv, Host};

use crate::h4z_mdl::{
    DEPTHS, Group, JOB, LEVEL_HASH, MAGIC_ID, Mdl, QUARTER, State, X_SCALE, band, close_bands,
    colours, copy_floats, count, f32_at, f64_at, front, normals, seg, slot, uvs,
};

const CORNERS: [u32; 4] = [0x10, 0x12, 0x14, 0x16];

/// A quad's mean depth (`f32`): `(d[+0x12] + d[+0x10] + d[+0x14] +
/// d[+0x16]) x 0.25`.
fn mean(h: &impl Host, m: &Mdl, rec: u32) -> f64 {
    let d = |o: u32| {
        let i = u32::from(h.read::<u16>(rec + o));
        f32_at(h, slot(m.frame, DEPTHS).wrapping_add(4 * i))
    };
    let s = add(add(add(d(0x12), d(0x10)), d(0x14)), d(0x16));
    load(store(mul(s, f64_at(h, QUARTER))))
}

/// Where quad `k` goes: (render data, slot). Any mode but 0: the band of
/// its mean depth (or the first) at the segment's index count / 6, the
/// counts then + 6 and + 4; else the current render data at `k`.
fn target(h: &mut impl Host, m: &Mdl, g: &Group, st: &mut State, rec: u32, k: i32) -> (u32, u32) {
    if m.mode == 0 {
        return (st.cur, k.cast_unsigned());
    }
    let j = band(h, m.frame, mean(h, m, rec));
    let rd = m.rds[if j == 4 { 0 } else { j }];
    let b = h.read::<u32>(seg(h, rd, g).wrapping_add(8)) / 6;
    let s = seg(h, rd, g);
    let n: u32 = h.read(s.wrapping_add(8));
    h.write(s.wrapping_add(8), n.wrapping_add(6));
    let s = seg(h, rd, g);
    let n: u32 = h.read(s.wrapping_add(4));
    h.write(s.wrapping_add(4), n.wrapping_add(4));
    st.cur = rd;
    (rd, b)
}

/// The four corners into `dst`: with the flags' 0x400 the part's `+0x14`
/// positions read as `f32` vectors, else as [`crate::h4z_mdl`]'s corners.
fn corners(h: &mut impl Host, m: &Mdl, rec: u32, dst: u32) {
    if h.read::<u32>(slot(m.frame, JOB)) & 0x400 != 0 {
        for (i, o) in (0u32..).zip(CORNERS) {
            let p = u32::from(h.read::<u16>(rec + o));
            copy_floats(
                h,
                m.shorts.wrapping_add(16 * p),
                dst.wrapping_add(0xc * i),
                3,
            );
        }
    } else {
        crate::h4z_mdl::corners(h, m, rec, &CORNERS, dst);
    }
}

/// The quad's six indices at slot `b`.
fn indices(h: &mut impl Host, idx: u32, b: u32) {
    let v = b.wrapping_mul(4);
    let at = idx.wrapping_add(b.wrapping_mul(12));
    for (o, d) in [(2, 2), (6, 2), (0, 0), (4, 1), (8, 3), (0xa, 1)] {
        h.write(at.wrapping_add(o), v.wrapping_add(d) as u16);
    }
}

/// The culled quad's indices: 12 zero bytes.
fn cull(h: &mut impl Host, idx: u32, b: u32) {
    let at = idx.wrapping_add(b.wrapping_mul(12));
    for o in [0, 4, 8] {
        h.write(at.wrapping_add(o), 0u32);
    }
}

/// The vertices of slot `b` of `rd`'s segment.
fn vertices(h: &impl Host, rd: u32, g: &Group, b: u32) -> u32 {
    h.read::<u32>(seg(h, rd, g).wrapping_add(0xc))
        .wrapping_add(b.wrapping_mul(0x30))
}

/// The bands' close for quads (any mode but 0).
fn close(h: &mut impl Host, m: &Mdl, g: &Group) {
    if m.mode != 0 {
        for j in 0..4 {
            close_bands(h, g, m.rds[j], count(h, g), 6, 4, 0x30);
        }
    }
}

/// The lit kinds' point light: with the flags' 0x40 and no shader point
/// light, each band's (its segment's vertex count) or the current render
/// data's (4n).
fn lights(h: &mut impl Host, m: &Mdl, g: &Group, st: &State) {
    if h.read::<u8>(slot(m.frame, JOB)) & 0x40 == 0
        || h.call_sym(crate::h4z_mdl::USE_POINT_LIGHT, Conv::Cdecl, &[]) != 0
    {
        return;
    }
    let job = slot(m.frame, JOB);
    if m.mode == 0 {
        let n = count(h, g).wrapping_mul(4).cast_unsigned();
        h.call_sym(
            crate::h4z_mdl::POINT_LIGHT_COLOURS,
            Conv::Cdecl,
            &[st.cur, g.seg1, job, g.header, n],
        );
    } else {
        for j in 0..4 {
            let rd = m.rds[j];
            let n = h.read::<u32>(seg(h, rd, g).wrapping_add(4));
            h.call_sym(
                crate::h4z_mdl::POINT_LIGHT_COLOURS,
                Conv::Cdecl,
                &[rd, g.seg1, job, g.header, n],
            );
        }
    }
}

/// Kind 4 (0x18-byte records): flat quads, culled by winding with the
/// flags' 0x10. Kind 5 (0x20: four normals at `+0x18`): lit quads (the
/// model buffer's vertices ignored). Returns the next header.
pub(crate) fn kind_flat(h: &mut impl Host, m: &Mdl, g: &Group, st: &mut State, lit: bool) -> u32 {
    let size = if lit { 0x20 } else { 0x18 };
    let mut rec = g.recs;
    let mut k = 0i32;
    while k < count(h, g) {
        let (rd, b) = target(h, m, g, st, rec, k);
        if lit || g.verts == 0 {
            let to = vertices(h, rd, g, b);
            corners(h, m, rec, to);
        }
        let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
        if !lit
            && h.read::<u8>(slot(m.frame, JOB)) & 0x10 != 0
            && !front(h, m, rec, [0x10, 0x12, 0x14])
        {
            cull(h, idx, b);
        } else {
            let to = h
                .read::<u32>(seg(h, rd, g).wrapping_add(0x14))
                .wrapping_add(b.wrapping_mul(0x40));
            let src = (g.colbuf != 0)
                .then(|| g.colbuf.wrapping_add(k.cast_unsigned().wrapping_mul(0x40)));
            colours(h, m, st, rec, src, to, 4);
            if lit {
                let to = h
                    .read::<u32>(seg(h, rd, g).wrapping_add(0x10))
                    .wrapping_add(b.wrapping_mul(0x30));
                normals(h, m, rec + 0x18, to, 4);
            }
            let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
            indices(h, idx, b);
        }
        rec = rec.wrapping_add(size);
        k += 1;
    }
    if lit {
        lights(h, m, g, st);
    }
    close(h, m, g);
    rec
}

/// Magic 0x241's x scale of a quad's four corners for the four texture
/// codes.
fn magic_x_scale(h: &mut impl Host, g: &Group, vtx: u32) {
    if h.call_sym(MAGIC_ID, Conv::Cdecl, &[]) == 0x241
        && matches!(g.code, 0x3900 | 0x39c0 | 0x3940 | 0x3980)
    {
        let c = f64_at(h, X_SCALE);
        for i in 0..4 {
            let at = vtx.wrapping_add(0xc * i);
            let v = store(mul(f32_at(h, at), c));
            h.write(at, v);
        }
    }
}

/// Kind 6 (0x28-byte records: `i16` texture coordinate pairs at `+0x18`):
/// textured quads, culled by winding with the flags' 0x10 but on level
/// 0x52b, normals (0, 0, 1); magics 0x66, 0x67 and 0x1ba (the part's)
/// shift texture page 0x3400's coordinates by -0x1000. A part whose
/// `+0x104` is 0x33e0 lays the group out instead ([`tiles`]); any other
/// nonzero `+0x104` draws nothing. Kind 7 (0x30: four normals at `+0x28`):
/// lit textured quads (the model buffer's vertices ignored). Returns the
/// next header.
pub(crate) fn kind_textured(
    h: &mut impl Host,
    m: &Mdl,
    g: &Group,
    st: &mut State,
    lit: bool,
) -> u32 {
    let size = if lit { 0x30 } else { 0x28 };
    let mut rec = g.recs;
    if !lit && m.flag104 == 0x33e0 {
        rec = tiles(h, m, g, st);
    }
    if lit || m.flag104 == 0 {
        let mut k = 0i32;
        while k < count(h, g) {
            let (rd, b) = target(h, m, g, st, rec, k);
            if lit || g.verts == 0 {
                let to = vertices(h, rd, g, b);
                corners(h, m, rec, to);
            }
            let culled = if lit {
                false
            } else {
                let to = vertices(h, rd, g, b);
                magic_x_scale(h, g, to);
                h.read::<u8>(slot(m.frame, JOB)) & 0x10 != 0
                    && h.call_sym(LEVEL_HASH, Conv::Cdecl, &[]) != 0x52b
                    && !front(h, m, rec, [0x10, 0x12, 0x14])
            };
            if culled {
                let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
                cull(h, idx, b);
            } else {
                let to = h
                    .read::<u32>(seg(h, rd, g).wrapping_add(0x14))
                    .wrapping_add(b.wrapping_mul(0x40));
                let src = (g.colbuf != 0)
                    .then(|| g.colbuf.wrapping_add(k.cast_unsigned().wrapping_mul(0x40)));
                colours(h, m, st, rec, src, to, 4);
                let to = h
                    .read::<u32>(seg(h, rd, g).wrapping_add(0x18))
                    .wrapping_add(b.wrapping_mul(0x20));
                let shifted = !lit && matches!(m.magic0, 0x66 | 0x67 | 0x1ba) && g.code == 0x3400;
                let src = (!shifted && g.uvbuf != 0)
                    .then(|| g.uvbuf.wrapping_add(k.cast_unsigned().wrapping_mul(0x20)));
                uvs(
                    h,
                    m,
                    g,
                    rec + 0x18,
                    src,
                    to,
                    4,
                    true,
                    if shifted { 0x1000 } else { 0 },
                );
                let idx = h.read::<u32>(seg(h, rd, g).wrapping_add(0x1c));
                indices(h, idx, b);
                let to = h
                    .read::<u32>(seg(h, rd, g).wrapping_add(0x10))
                    .wrapping_add(b.wrapping_mul(0x30));
                if lit {
                    normals(h, m, rec + 0x28, to, 4);
                } else {
                    for i in 0..4 {
                        let at = to.wrapping_add(0xc * i);
                        h.write(at, 0f32.to_bits());
                        h.write(at.wrapping_add(4), 0f32.to_bits());
                        h.write(at.wrapping_add(8), 1f32.to_bits());
                    }
                }
            }
            rec = rec.wrapping_add(size);
            k += 1;
        }
    }
    if lit {
        lights(h, m, g, st);
    }
    close(h, m, g);
    rec
}

/// Kind 6 for a part whose `+0x104` is 0x33e0: every record of group
/// `n` (from 0) overwrites quad `n` of the current render data's first
/// segment (the `i16` positions, the colour bytes times the scale, the
/// texture coordinates shifted by -0x1000 then into tile (n % 8, 2 - n /
/// 8) of an 8 x 3 sheet, the indices); no opaque test. Returns the next
/// header.
fn tiles(h: &mut impl Host, m: &Mdl, g: &Group, st: &State) -> u32 {
    let n = g.seg1.wrapping_sub(1).cast_signed();
    let col = load(store(f64::from(n % 8)));
    let row = load(store(f64::from(2 - n / 8)));
    let q = g.seg1.wrapping_sub(1);
    let mut rec = g.recs;
    let mut k = 0i32;
    while k < count(h, g) {
        let rd = st.cur;
        let geo: u32 = h.read(rd.wrapping_add(0x94));
        let vtx = h
            .read::<u32>(geo.wrapping_add(0xc))
            .wrapping_add(q.wrapping_mul(0x30));
        for (i, o) in (0u32..).zip(CORNERS) {
            let p = u32::from(h.read::<u16>(rec + o));
            for c in 0..3 {
                let s = h.read::<i16>(m.shorts.wrapping_add(6 * p + 2 * c));
                h.write(vtx.wrapping_add(0xc * i + 4 * c), f32::from(s).to_bits());
            }
        }
        let geo: u32 = h.read(rd.wrapping_add(0x94));
        let col_at = h
            .read::<u32>(geo.wrapping_add(0x14))
            .wrapping_add(q.wrapping_mul(0x40));
        for j in 0..16 {
            let b = h.read::<u8>(rec + j);
            h.write(col_at.wrapping_add(4 * j), f32::from(b).to_bits());
        }
        let scale = slot(m.frame, crate::h4z_mdl::SCALE);
        for i in 0..4 {
            let v = col_at.wrapping_add(0x10 * i);
            h.call_sym(crate::h4z_mdl::MUL_VECTOR, Conv::Cdecl, &[v, scale, v]);
        }
        let geo: u32 = h.read(st.cur.wrapping_add(0x94));
        let uv = h
            .read::<u32>(geo.wrapping_add(0x18))
            .wrapping_add(q.wrapping_mul(0x20));
        uvs(h, m, g, rec + 0x18, None, uv, 4, true, 0x1000);
        let eighth = f64_at(h, EIGHTH);
        let third = f64_at(h, THIRD_F32);
        for i in 0..4 {
            let at = uv.wrapping_add(8 * i);
            let u = store(mul(add(f32_at(h, at), col), eighth));
            h.write(at, u);
            let v = store(mul(add(f32_at(h, at + 4), row), third));
            h.write(at + 4, v);
        }
        let geo: u32 = h.read(st.cur.wrapping_add(0x94));
        let idx: u32 = h.read(geo.wrapping_add(0x1c));
        indices(h, idx, q);
        rec = rec.wrapping_add(0x28);
        k += 1;
    }
    rec
}

/// `f64` constants of [`tiles`]: 0.125 (`0x00b92200`) and 0.333333
/// (`0x00b501f8`).
pub const EIGHTH: u32 = 0x00b9_2200;
/// See [`EIGHTH`].
pub const THIRD_F32: u32 = 0x00b5_01f8;
