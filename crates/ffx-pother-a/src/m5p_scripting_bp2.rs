//! M5 stage 1, list `scripting`, round big-phyre2
//! (`ffx_pother_a::m5p_scripting_bp2`): an occluder's silhouette planes
//! (`0x00a312c0`), the function that sizes a buffer at run time
//! (`_alloca_probe_16`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_scripting.rs` (`bp2_*`).
//!
//! The bridge is split at the alloca (`docs/porting.md`, "Run-time
//! frames and varargs"): [`bridge::silhouette_prepare`] does what the
//! original does before it and returns the buffer's size,
//! [`bridge::silhouette_finish`] the rest with the buffer's address. The
//! locals callees see live at the original's `ebp` offsets.
//!
//! Floats on the x87 at 53 bits, each store rounded to `f32`
//! ([`ffx_bridge::x87`]).

/// `PMatrix4_FromMatrix3x4_00451080` (thiscall `(out; m)`).
pub const FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_Inverse_004cc0c0` (cdecl `(out, m)` -> `out`).
pub const INVERSE: u32 = 0x004c_c0c0;
/// `PMatrix4_Copy_004c9430` (thiscall `(dst; src)`).
pub const COPY: u32 = 0x004c_9430;
/// `PMatrix4_TransformPoint3_004c9eb0` (thiscall `(m; out, p)` -> `out`).
pub const TRANSFORM3: u32 = 0x004c_9eb0;
/// `PMatrix4_Rotate3_004ca140` (thiscall `(m; out, v)`).
pub const ROTATE3: u32 = 0x004c_a140;
/// `PMatrix4_Transpose_004f93f0` (cdecl `(out, m)` -> `out`).
pub const TRANSPOSE: u32 = 0x004f_93f0;
/// `POccluderGeometry_plane_00a30eb0` (cdecl `(out, a, b, p)`).
pub const PLANE: u32 = 0x00a3_0eb0;
/// `POccluderGeometry_pushEdgeQuad_00a30600` (thiscall `(list; a, b, d)`).
pub const PUSH_EDGE_QUAD: u32 = 0x00a3_0600;
/// The edge-quad list `0x00a30600` fills.
pub const EDGE_QUADS: u32 = 0x01a8_5410;
/// The `f64` 2.0 each silhouette edge adds to the weight.
pub const TWO: u32 = 0x00b9_2270;
/// The `f32` -1.0 the axis signs take.
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// The most silhouette planes kept.
pub const MAX_PLANES: u32 = 12;
/// The words of `0x00a312c0`'s frame below `ebp`.
pub const SILHOUETTE_FRAME_WORDS: u32 = 0x2a4 / 4;

pub mod bridge {
    //! The original over game memory; `ebp` is where its frame pointer is.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `fstp dword` then `fld dword`.
    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn put(h: &mut impl Host, at: u32, x: f64) {
        h.write(at, x87::store(x));
    }

    fn put3(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    fn ld3(h: &impl Host, at: u32) -> [f64; 3] {
        [0, 4, 8].map(|o| ld(h, at + o))
    }

    /// `n` floats through the x87 (`fld`/`fstp` pairs).
    fn copy_floats(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let x = ld(h, src.wrapping_add(4 * k));
            put(h, dst.wrapping_add(4 * k), x);
        }
    }

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), w);
        }
    }

    /// `x * m` with `m` a memory operand (`fmul dword [m]`).
    fn mul_m(x: f64, h: &impl Host, m: u32) -> f64 {
        Reg::constant(x).mul_m32(f32::from_bits(h.read(m))).value()
    }

    /// `x - m` with `m` a memory operand (`fsub dword [m]`).
    fn sub_m(x: f64, h: &impl Host, m: u32) -> f64 {
        Reg::constant(x).sub_m32(f32::from_bits(h.read(m))).value()
    }

    /// The occluder's matrices and the camera in its space, up to the
    /// alloca of `0x00a312c0(item; owner, planes, block)`: the item's 3x4
    /// (`+4`) as a 4x4 at `ebp - 0x180` (the identity without one), kept
    /// at `ebp - 0x244`; its inverse (the identity without a 3x4) at
    /// `ebp - 0x294` and again at `ebp - 0x180`; the camera's position
    /// (`[owner + 0xb0] + 0x20`, zeros without a camera) at `ebp - 0x138`
    /// and through the inverse at `ebp - 0xdc`. The geometry (`[item]`)
    /// at `ebp - 0x118`. Returns the alloca's size: a byte per plane,
    /// four per group of four (`((count + 3) >> 2) * 4`).
    pub fn silhouette_prepare(h: &mut impl Host, ebp: u32, item: u32, owner: u32) -> u32 {
        let geometry: u32 = h.read(item);
        let local: u32 = h.read(item + 4);
        h.write(ebp - 0x118, geometry);
        let m = ebp - 0x180;
        let world = ebp - 0x244;
        let inverse = ebp - 0x294;
        if local == 0 {
            for k in 0..16u32 {
                let one = k % 5 == 0;
                h.write(m + 4 * k, if one { 1.0f32 } else { 0.0 }.to_bits());
            }
        } else {
            h.call_sym(FROM_3X4, Conv::Thiscall(m), &[local]);
        }
        copy_words(h, world, m, 16);
        if local == 0 {
            copy_words(h, inverse, m, 16);
        } else {
            let r = h.call_sym(INVERSE, Conv::Cdecl, &[m, world]);
            h.call_sym(COPY, Conv::Thiscall(inverse), &[r]);
        }
        let camera: u32 = h.read(owner + 0xb0);
        copy_words(h, m, inverse, 16);
        let c = if camera == 0 {
            [0.0; 3]
        } else {
            [0x20, 0x24, 0x28].map(|o| st(ld(h, camera + o)))
        };
        put3(h, ebp - 0xdc, c);
        put3(h, ebp - 0x138, c);
        let r = h.call_sym(TRANSFORM3, Conv::Thiscall(m), &[ebp - 0x1e4, ebp - 0x138]);
        copy_floats(h, ebp - 0xdc, r, 3);
        let count: u32 = h.read(geometry);
        (count.wrapping_add(3) >> 2).wrapping_mul(4)
    }

    /// The plane flags of [`silhouette_finish`]: for each group of four
    /// planes (from `+0x30`, rows x, y, z, d of 0x40 bytes), whether the
    /// camera `c` is behind each: `(x c.x + y c.y) + z c.z - d` (each
    /// product and sum rounded) below 0 (a NaN is not); a byte each.
    fn plane_flags(h: &mut impl Host, geometry: u32, groups: u32, c: [f64; 3], flags: u32) {
        let first = geometry.wrapping_add(0x30);
        let end = (groups << 6).wrapping_add(first);
        let mut g = first;
        let mut out = flags;
        while g < end {
            for k in 0..4u32 {
                let x = st(x87::mul(ld(h, g + 4 * k), c[0]));
                let y = st(x87::mul(ld(h, g + 0x10 + 4 * k), c[1]));
                let z = st(x87::mul(ld(h, g + 0x20 + 4 * k), c[2]));
                let s = st(x87::add(x, y));
                let s = st(x87::add(s, z));
                let v = st(sub_m(s, h, g + 0x30 + 4 * k));
                h.write(out + k, u8::from(v < 0.0));
            }
            g = g.wrapping_add(0x40);
            out = out.wrapping_add(4);
        }
    }

    /// One silhouette edge between vertices `v0` and `v1` (their world
    /// positions at `ebp - 0x100` and `ebp - 0xf0`): the plane through
    /// them and the camera (`ebp - 0x138`), the vertices in the order the
    /// first plane's flag says; its four words (raw) at `ebp - 0xdc` and,
    /// through the x87, at `entry`.
    fn edge_plane(
        h: &mut impl Host,
        ebp: u32,
        behind: bool,
        w0: [f64; 3],
        w1: [f64; 3],
        entry: u32,
    ) {
        let c = ld3(h, ebp - 0x138);
        let a = [0, 1, 2].map(|k| st(x87::sub(w0[k], c[k])));
        let b = [0, 1, 2].map(|k| st(x87::sub(w1[k], c[k])));
        let out = if behind {
            put3(h, ebp - 0x1a0, b);
            put3(h, ebp - 0x1b0, a);
            h.call_sym(
                PLANE,
                Conv::Cdecl,
                &[ebp - 0x204, ebp - 0x1b0, ebp - 0x1a0, ebp - 0x138],
            );
            ebp - 0x204
        } else {
            put3(h, ebp - 0x1c0, a);
            put3(h, ebp - 0x1f4, b);
            h.call_sym(
                PLANE,
                Conv::Cdecl,
                &[ebp - 0x1e4, ebp - 0x1f4, ebp - 0x1c0, ebp - 0x138],
            );
            ebp - 0x1e4
        };
        copy_words(h, ebp - 0xdc, out, 4);
        copy_floats(h, entry, ebp - 0xdc, 4);
    }

    /// The rest of `0x00a312c0` with the alloca's buffer at `flags`: the
    /// plane flags; then for each edge (plane-index pairs after the
    /// vertices, vertex-index pairs after them) whose two planes' flags
    /// differ, up to [`MAX_PLANES`]: the vertices' world positions (the
    /// 4x4 at `ebp - 0x244`) summed into a weighted centre (each edge
    /// adds 2.0 to the weight), the plane through the edge and the camera
    /// into the list at `ebp - 0xc4`, and below the cap the edge's quad
    /// pushed with that plane. With no such edge, `planes` is returned
    /// untouched. Else the centre's direction from the camera, rotated
    /// into the occluder's space (`ebp - 0x180`), picks the bounds' corner
    /// nearest the camera on each axis (`+0x10..` or `+0x20..`), which
    /// goes to world space; the free list entries are zeroed and the list
    /// goes out transposed, four planes per 0x40 bytes, from `planes`;
    /// `block` takes the transpose of the three world axes signed towards
    /// the camera (1 when the direction is below 0, else -1) with their
    /// distances through the corner. Returns the end of the planes
    /// written.
    #[allow(clippy::too_many_lines)] // one original
    pub fn silhouette_finish(
        h: &mut impl Host,
        ebp: u32,
        planes: u32,
        block: u32,
        flags: u32,
    ) -> u32 {
        let geometry: u32 = h.read(ebp - 0x118);
        let c = ld3(h, ebp - 0xdc);
        let groups = h.read::<u32>(geometry).wrapping_add(3) >> 2;
        plane_flags(h, geometry, groups, c, flags);
        let mut acc = [0.0f64; 4];
        let groups = h.read::<u32>(geometry).wrapping_add(3) >> 2;
        let vertices = (groups << 6).wrapping_add(0x30).wrapping_add(geometry);
        let k = groups
            .wrapping_mul(4)
            .wrapping_add(3)
            .wrapping_add(h.read(geometry + 4));
        let edges_bytes: u32 = h.read(geometry + 8);
        let mut edge = (k << 4).wrapping_add(geometry);
        let mut pair =
            geometry.wrapping_add(edges_bytes.wrapping_add(k.wrapping_mul(8)).wrapping_mul(2));
        let end = pair;
        if edge >= end {
            return planes;
        }
        let list = ebp - 0xc4;
        let two = f64::from_bits(h.read(h.rebase(TWO)));
        let world = ebp - 0x244;
        let mut count = 0u32;
        loop {
            let i0 = u32::from(h.read::<u8>(edge));
            let i1 = u32::from(h.read::<u8>(edge + 1));
            let f0: u8 = h.read(flags.wrapping_add(i0));
            let f1: u8 = h.read(flags.wrapping_add(i1));
            if f0 != f1 {
                let v0 = u32::from(h.read::<u8>(pair));
                let v1 = u32::from(h.read::<u8>(pair + 1));
                copy_words(h, ebp - 0x100, (v0 << 4).wrapping_add(vertices), 4);
                copy_words(h, ebp - 0xf0, (v1 << 4).wrapping_add(vertices), 4);
                let r = h.call_sym(
                    TRANSFORM3,
                    Conv::Thiscall(world),
                    &[ebp - 0x2a4, ebp - 0x100],
                );
                copy_floats(h, ebp - 0x100, r, 3);
                let r = h.call_sym(
                    TRANSFORM3,
                    Conv::Thiscall(world),
                    &[ebp - 0x254, ebp - 0xf0],
                );
                copy_floats(h, ebp - 0xf0, r, 3);
                let w0 = ld3(h, ebp - 0x100);
                let w1 = ld3(h, ebp - 0xf0);
                let s = [0, 1, 2].map(|k| st(x87::add(w1[k], w0[k])));
                for k in 0..3 {
                    acc[k] = st(x87::add(s[k], acc[k]));
                }
                acc[3] = st(x87::add(acc[3], two));
                let entry = list + 16 * count;
                edge_plane(h, ebp, f0 != 0, w0, w1, entry);
                count += 1;
                if count == MAX_PLANES {
                    break;
                }
                copy_floats(h, ebp - 0x1d0, ebp - 0xdc, 3);
                let quads = h.rebase(EDGE_QUADS);
                h.call_sym(
                    PUSH_EDGE_QUAD,
                    Conv::Thiscall(quads),
                    &[ebp - 0x100, ebp - 0xf0, ebp - 0x1d0],
                );
            }
            edge = edge.wrapping_add(2);
            pair = pair.wrapping_add(2);
            if edge >= end {
                break;
            }
        }
        if count == 0 {
            return planes;
        }
        silhouette_out(h, ebp, geometry, acc, count, planes, block)
    }

    /// The tail of [`silhouette_finish`] once some plane is kept.
    fn silhouette_out(
        h: &mut impl Host,
        ebp: u32,
        geometry: u32,
        acc: [f64; 4],
        count: u32,
        mut planes: u32,
        block: u32,
    ) -> u32 {
        let centre = [0, 1, 2].map(|k| st(x87::div(acc[k], acc[3])));
        let c = ebp - 0x138;
        let d = [(centre[0], 0u32), (centre[1], 4), (centre[2], 8)]
            .map(|(x, o)| st(sub_m(x, h, c + o)));
        put3(h, ebp - 0x1d0, d);
        h.call_sym(
            ROTATE3,
            Conv::Thiscall(ebp - 0x180),
            &[ebp - 0xdc, ebp - 0x1d0],
        );
        let dl = ld3(h, ebp - 0xdc);
        // the corner nearest the camera: the upper bound where the
        // direction is above 0 (a NaN takes the lower)
        let pick = |up: bool, lo: u32| geometry + if up { lo + 0x10 } else { lo };
        let z = st(ld(h, pick(0.0 < dl[2], 0x18)));
        let y = st(ld(h, pick(dl[1] > 0.0, 0x14)));
        let x = st(ld(h, pick(0.0 < dl[0], 0x10)));
        put3(h, ebp - 0xf0, [x, y, z]);
        let world = ebp - 0x244;
        let r = h.call_sym(
            TRANSFORM3,
            Conv::Thiscall(world),
            &[ebp - 0x254, ebp - 0xf0],
        );
        copy_floats(h, ebp - 0xf0, r, 3);
        let list = ebp - 0xc4;
        for k in count..MAX_PLANES {
            h.write(list + 16 * k, [0u32; 4]);
        }
        let m = ebp - 0x180;
        for j in 0..=((count - 1) >> 2) {
            copy_floats(h, m, list + 0x40 * j, 16);
            let out = planes;
            planes = planes.wrapping_add(0x40);
            let r = h.call_sym(TRANSPOSE, Conv::Cdecl, &[ebp - 0x294, m]);
            copy_floats(h, out, r, 16);
        }
        let flip = ld(h, h.rebase(MINUS_ONE));
        let sign = |v: f64| if v < 0.0 { 1.0 } else { flip };
        let cw = ld3(h, ebp - 0xf0);
        let (r0, r1, r2) = {
            let s0 = sign(dl[0]);
            let r0 = [
                st(x87::mul(ld(h, world), s0)),
                st(x87::mul(ld(h, world + 4), s0)),
                st(mul_m(s0, h, world + 8)),
            ];
            let s1 = sign(dl[1]);
            let r1 = [
                st(x87::mul(ld(h, world + 0x10), s1)),
                st(x87::mul(ld(h, world + 0x14), s1)),
                st(mul_m(s1, h, world + 0x18)),
            ];
            let s2 = sign(dl[2]);
            let r2 = [
                st(x87::mul(ld(h, world + 0x20), s2)),
                st(x87::mul(ld(h, world + 0x24), s2)),
                st(mul_m(s2, h, world + 0x28)),
            ];
            (r0, r1, r2)
        };
        let d0 = {
            let t = st(x87::mul(r0[0], cw[0]));
            let t = st(x87::add(x87::mul(r0[1], cw[1]), t));
            st(x87::add(t, x87::mul(r0[2], cw[2])))
        };
        let d1 = {
            let u = st(x87::mul(r1[0], cw[0]));
            let u = st(x87::add(x87::mul(cw[1], r1[1]), u));
            st(x87::add(u, x87::mul(r1[2], cw[2])))
        };
        let d2 = {
            let w = st(x87::mul(cw[0], r2[0]));
            let w = st(x87::add(w, x87::mul(r2[1], cw[1])));
            st(x87::add(w, x87::mul(r2[2], cw[2])))
        };
        let rows = [
            r0[0], r0[1], r0[2], d0, r1[0], r1[1], r1[2], d1, r2[0], r2[1], r2[2], d2, 0.0, 0.0,
            0.0, 0.0,
        ];
        h.write(m, rows.map(x87::store));
        let r = h.call_sym(TRANSPOSE, Conv::Cdecl, &[ebp - 0x294, m]);
        copy_floats(h, block, r, 16);
        planes
    }
}
