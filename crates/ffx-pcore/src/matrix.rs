//! `PMatrix4` (16 floats, four rows of four) helpers: copies, builders,
//! row products. All x87 at 53 bits: a copy through `fld`/`fstp` keeps
//! every value but quiets a signalling NaN; products round once per x87
//! operation and once more where the original stores.
//!
//! The order of reads and writes is the original's, so an output that
//! overlaps an input gives what the original gives.

pub mod bridge {
    //! The originals over game memory.

    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::Host;

    /// `fld dword [src]` as an x87 value.
    fn ld(h: &impl Host, src: u32) -> f64 {
        x87::load(h.read(src))
    }

    /// `fstp dword [dst]`.
    fn st(h: &mut impl Host, dst: u32, x: f64) {
        h.write(dst, x87::store(x));
    }

    /// `fld dword [src]; fstp dword [dst]`.
    fn cp(h: &mut impl Host, dst: u32, src: u32) {
        let x = ld(h, src);
        st(h, dst, x);
    }

    /// Reads three floats.
    fn ld3(h: &impl Host, src: u32) -> [f64; 3] {
        [ld(h, src), ld(h, src + 4), ld(h, src + 8)]
    }

    /// Stores three floats.
    fn st3(h: &mut impl Host, dst: u32, v: [f64; 3]) {
        for (i, x) in v.into_iter().enumerate() {
            st(h, dst + 4 * i as u32, x);
        }
    }

    /// `0x00453340` / `0x004c9430(this, m)`: `this = m`; `this`. The first
    /// copies word by word, the second row by row (all four read first).
    pub fn copy(h: &mut impl Host, this: u32, m: u32, by_rows: bool) -> u32 {
        if by_rows {
            for r in 0..4 {
                let row: [u32; 4] = std::array::from_fn(|i| h.read(m + 16 * r + 4 * i as u32));
                for (i, w) in row.into_iter().enumerate() {
                    st(h, this + 16 * r + 4 * i as u32, x87::load(w));
                }
            }
        } else {
            for i in 0..16 {
                cp(h, this + 4 * i, m + 4 * i);
            }
        }
        this
    }

    /// `0x00450d40(this, r0, r1, r2, r3)`: the rows from four vectors;
    /// `this`.
    pub fn from_rows(h: &mut impl Host, this: u32, rows: [u32; 4]) -> u32 {
        for (r, src) in rows.into_iter().enumerate() {
            for i in 0..4 {
                cp(h, this + 16 * r as u32 + 4 * i, src + 4 * i);
            }
        }
        this
    }

    /// `0x00450de0(this, a, b, c, d)`: rows `(a, 0)`, `(b, 0)`, `(c, 0)`,
    /// `(d, 1)` from four 3-vectors (all read first); `this`.
    pub fn from_axes(h: &mut impl Host, this: u32, axes: [u32; 4]) -> u32 {
        let d = ld3(h, axes[3]);
        let c = ld3(h, axes[2]);
        let b = ld3(h, axes[1]);
        let a = ld3(h, axes[0]);
        for (r, v) in [a, b, c, d].into_iter().enumerate() {
            st3(h, this + 16 * r as u32, v);
            st(
                h,
                this + 16 * r as u32 + 0xc,
                if r == 3 { 1.0 } else { 0.0 },
            );
        }
        this
    }

    /// `PMatrix4_FromMatrix3x4` `0x00451080(this, m)`: from the 3x4 `m`
    /// (12 floats): rows `(m3, m7, m11, 0)`, `(m0, m1, m2, 0)`,
    /// `(m4, m5, m6, 0)`, `(m8, m9, m10, 1)`; `this`.
    pub fn from_3x4(h: &mut impl Host, this: u32, m: u32) -> u32 {
        let t = [ld(h, m + 0xc), ld(h, m + 0x1c), ld(h, m + 0x2c)];
        st3(h, this, t);
        st(h, this + 0xc, 0.0);
        for (r, src) in [(1u32, m), (2, m + 0x10), (3, m + 0x20)] {
            let v = ld3(h, src);
            st3(h, this + 16 * r, v);
            st(h, this + 16 * r + 0xc, if r == 3 { 1.0 } else { 0.0 });
        }
        this
    }

    /// `PMatrix4_toWorldMatrix3x4` `0x00451190(dst, m)`: the 3x4 of rows
    /// 1-3 of `m`, with row 0's first three in the last column; `dst`.
    pub fn to_world_3x4(h: &mut impl Host, dst: u32, m: u32) -> u32 {
        let row0: [u32; 4] = std::array::from_fn(|i| h.read(m + 4 * i as u32));
        for r in 0..3 {
            let row: [u32; 4] = std::array::from_fn(|i| h.read(m + 16 * (r + 1) + 4 * i as u32));
            for (i, w) in row.into_iter().enumerate() {
                st(h, dst + 16 * r + 4 * i as u32, x87::load(w));
            }
        }
        for (r, w) in row0.into_iter().take(3).enumerate() {
            st(h, dst + 16 * r as u32 + 0xc, x87::load(w));
        }
        dst
    }

    /// The identity's stores in `0x00451bb0`'s order (word index, 1 or 0).
    const IDENTITY_ORDER: [(u32, bool); 16] = [
        (0, true),
        (1, false),
        (2, false),
        (3, false),
        (4, false),
        (6, false),
        (7, false),
        (5, true),
        (8, false),
        (9, false),
        (11, false),
        (10, true),
        (12, false),
        (13, false),
        (14, false),
        (15, true),
    ];

    /// `PMatrix4_Identity` `0x00451bb0(out)`: the identity; `out`.
    pub fn identity(h: &mut impl Host, out: u32) -> u32 {
        for (i, one) in IDENTITY_ORDER {
            st(h, out + 4 * i, if one { 1.0 } else { 0.0 });
        }
        out
    }

    /// `0x00451fe0(dst)`: `dst` as the 3x4 of the identity
    /// ([`to_world_3x4`]) when set. `al = 1`.
    pub fn world_identity(h: &mut impl Host, scratch: u32, dst: u32) -> u8 {
        if dst != 0 {
            world_identity_into(h, scratch, dst);
        }
        1
    }

    /// `0x004530a0(this)`: a camera-like object's defaults (`this` as the
    /// identity's 3x4, then the words `+0x30 .. +0x148`, five constants
    /// from `0x00b10f38 .. 0x00b10f54`); `this`. `scratch`: 16 words for
    /// the identity.
    pub fn init_4530a0(h: &mut impl Host, scratch: u32, this: u32) -> u32 {
        world_identity_into(h, scratch, this);
        let c38 = constant(h, 0x00b1_0f38);
        let c3c = constant(h, 0x00b1_0f3c);
        let c40 = constant(h, 0x00b1_0f40);
        let c44 = constant(h, 0x00b1_0f44);
        let c48 = constant(h, 0x00b1_0f48);
        let c4c = constant(h, 0x00b1_0f4c);
        let c50 = constant(h, 0x00b1_0f50);
        let c54 = constant(h, 0x00b1_0f54);
        // three 4x4 identities at +0x30, +0x70, +0xb0 ... as the original
        // stores them: ones at the listed offsets, zeros at the rest
        let ones = [0x30, 0x44, 0x58, 0x6c, 0x70, 0x84, 0x98, 0xac];
        for off in (0x30..=0xac).step_by(4) {
            st(h, this + off, if ones.contains(&off) { 1.0 } else { 0.0 });
        }
        st(h, this + 0xb4, c38);
        h.write(this + 0xb0, 0u32);
        st(h, this + 0xb8, c4c);
        st(h, this + 0xbc, 0.0);
        h.write(this + 0xcc, 0u32);
        st(h, this + 0xc0, 0.0);
        st(h, this + 0xc4, 0.0);
        st(h, this + 0xd0, c48);
        st(h, this + 0xd4, c44);
        st(h, this + 0xd8, c40);
        st(h, this + 0xe0, c50);
        st(h, this + 0xe4, c54);
        st(h, this + 0xe8, 1.0);
        st(h, this + 0xec, 0.0);
        for off in [0xf0, 0xf4, 0xf8] {
            st(h, this + off, c3c);
        }
        st(h, this + 0x100, 1.0);
        st(h, this + 0x104, 1.0);
        st(h, this + 0x108, 0.0);
        st(h, this + 0x10c, 0.0);
        for off in [0x110, 0x114, 0x118] {
            st(h, this + off, c4c);
        }
        for off in [0x11c, 0x120, 0x124, 0x128] {
            st(h, this + off, 1.0);
        }
        for off in (0x12c..=0x148).step_by(4) {
            st(h, this + off, 0.0);
        }
        this
    }

    /// `fld dword [sym]` of an image constant.
    fn constant(h: &mut impl Host, sym: u32) -> f64 {
        let a = h.rebase(sym);
        ld(h, a)
    }

    /// The identity in `scratch`, then [`to_world_3x4`] into `dst`.
    fn world_identity_into(h: &mut impl Host, scratch: u32, dst: u32) {
        // the original's frame stores, in its order
        const ORDER: [(u32, bool); 16] = [
            (0, true),
            (1, false),
            (2, false),
            (3, false),
            (4, false),
            (6, false),
            (7, false),
            (8, false),
            (9, false),
            (11, false),
            (12, false),
            (13, false),
            (14, false),
            (5, true),
            (10, true),
            (15, true),
        ];
        for (i, one) in ORDER {
            st(h, scratch + 4 * i, if one { 1.0 } else { 0.0 });
        }
        to_world_3x4(h, dst, scratch);
    }

    /// `0x004c9360(this, m)`: `m`'s affine part (rows' fourth words
    /// `0, 0, 0, 1`); `this`.
    pub fn affine(h: &mut impl Host, this: u32, m: u32) -> u32 {
        for r in 0..4 {
            let v = ld3(h, m + 16 * r);
            st3(h, this + 16 * r, v);
            st(h, this + 16 * r + 0xc, if r == 3 { 1.0 } else { 0.0 });
        }
        this
    }

    /// The row vector `v` times `m`, component `c` (`0x004c9c20`'s sums;
    /// `point`: `v.w = 1` and the last term added, `0x004c9cb0`).
    fn row_times(m: &[f32; 16], v: &[f32; 4], c: usize, point: bool) -> f64 {
        let e = |i: usize| m[4 * i + c];
        let (first, second) = if c == 0 {
            // fld m4; fmul v1; fld v0; fmul m0
            (Reg::load(e(1)).mul_m32(v[1]), Reg::load(v[0]).mul_m32(e(0)))
        } else {
            // fld m[c]; fmul v0; fld m[4 + c]; fmul v1
            (Reg::load(e(0)).mul_m32(v[0]), Reg::load(e(1)).mul_m32(v[1]))
        };
        let mut s = first + second;
        s = s + Reg::load(e(2)).mul_m32(v[2]);
        s = if point {
            s.add_m32(e(3))
        } else {
            s + Reg::load(e(3)).mul_m32(v[3])
        };
        s.value()
    }

    /// Reads 16 floats.
    fn read16(h: &impl Host, m: u32) -> [f32; 16] {
        std::array::from_fn(|i| h.read(m + 4 * i as u32))
    }

    /// `PMatrix4_RowTimes` `0x004c9c20(m, out, v)`: `out = v m` (4-vector);
    /// `out`.
    pub fn row_times_vector(h: &mut impl Host, m: u32, out: u32, v: u32) -> u32 {
        for c in 0..4 {
            let mm = read16(h, m);
            let vv: [f32; 4] = std::array::from_fn(|i| h.read(v + 4 * i as u32));
            let x = row_times(&mm, &vv, c, false);
            st(h, out + 4 * c as u32, x);
        }
        out
    }

    /// `PMatrix4_TransformPoint` `0x004c9cb0(m, out, p)`: `out = (p, 1) m`;
    /// `out`.
    pub fn transform_point(h: &mut impl Host, m: u32, out: u32, p: u32) -> u32 {
        for c in 0..4 {
            let mm = read16(h, m);
            let pp: [f32; 4] = [h.read(p), h.read(p + 4), h.read(p + 8), 0.0];
            let x = row_times(&mm, &pp, c, true);
            st(h, out + 4 * c as u32, x);
        }
        out
    }

    /// `0x004c9bc0(m, out, b)`: `out = b m` row by row (each row of `b`
    /// times `m` into the frame, `scratch` 16 words, then copied); `out`.
    pub fn times_rows(h: &mut impl Host, scratch: u32, m: u32, out: u32, b: u32) -> u32 {
        for r in (0..4).rev() {
            row_times_vector(h, m, scratch + 16 * r, b + 16 * r);
        }
        from_rows(h, out, [scratch, scratch + 16, scratch + 32, scratch + 48])
    }
}
