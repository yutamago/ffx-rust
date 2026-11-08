//! M5 round 11 (`tail11`): `updateVFXRenderDataShp_OShapeVu1`
//! (`0x0071c0b0`, cdecl `(lo, hi, shape, obj)` -> 1, or 0 without render
//! data or a segment buffer), the PS2 `OShapeVu1` shapes' VFX render data.
//!
//! The shape record (`+0` i16 flags, `+2` i16 quad count, `+0xf` its kind)
//! fills the render data's segment (`+0x94`) from its first quad: counts
//! `+8` = 6n, `+4` = 4n. Kind 3 is drawn here: four vertices per quad
//! through the object's matrix (or `VertexGroupApplyZMatZClip`, which may
//! clip the quad: its six indices are zeroed then), its colour (RGBA / 256,
//! alpha / `g_AlphaScale`, times the object's tint / 128) four times, its
//! uvs (/ 4096, v flipped) and six indices; then the fill matrix with the
//! w column (0, 0, 0, 1), bounds of +-100000 and made visible. Other kinds
//! only get the buffer, and a fill request for the job workers
//! (`JobSchedule_pppPart_getFillBufferRequest`) when the fill buffer is
//! valid.
//!
//! The quads are records from `shape + 0x18`: colour bytes at `+0`, then
//! with flag bit 3 four corners (eight i16 at `+8`) and uvs at `+0x18`,
//! 0x28 bytes; without it two corners (`+8` x0, y0, x1, y1: the rectangle's
//! four vertices) and uvs at `+0x10`, 0x20 bytes. Corners are divided by
//! the image's 15.0 (`0x00b922c0`; -15.0 at `0x00b501c8` for two corners).
//!
//! x87: every value is computed at 53 bits in the original's order and
//! rounded where the original stores a `dword` (`ffx_bridge::x87`).

/// The original as a bridge over [`ffx_graphic::host::Host`].
pub mod bridge {
    use ffx_bridge::x87::{add, div, load, mul, store, sub};
    use ffx_graphic::host::{Conv, Host};

    /// `graphicVFXGetDynamicGeometryDataByInstance(lo, hi)` (cdecl).
    pub const GEOMETRY: u32 = 0x0064_5380;
    /// `graphicVFXGetRenderDataByGeometry(geometry)` (cdecl).
    pub const RENDER_DATA: u32 = 0x0064_53b0;
    /// `graphicDynGeoFillBufferIsValid()` (cdecl).
    pub const FILL_VALID: u32 = 0x0063_fd70;
    /// `JobSchedule_pppPart_getFillBufferRequest(quads)` (cdecl).
    pub const FILL_REQUEST: u32 = 0x0072_8660;
    /// `graphicDynGeoMemManagerGetBufferForSegment(rd, segment, vertices,
    /// indices)` (cdecl).
    pub const SEGMENT_BUFFER: u32 = 0x0063_fdb0;
    /// `FFXVu0MulMatrix(dst, a, b)` (cdecl).
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    /// `VertexGroupApplyZMatZClip(vertices, m, second, fill2, fill3,
    /// flags)` (cdecl) -> 0 drawn, else clipped.
    pub const ZCLIP: u32 = 0x0071_19d0;
    /// `graphicVFXSetVisible(lo, hi, visible)` (cdecl).
    pub const SET_VISIBLE: u32 = 0x0064_5710;
    /// `g_pppFillMatrix` (`0x0230fca0`).
    pub const FILL_MATRIX: u32 = 0x0230_fca0;
    /// `g_pppFillMatrix2` (`0x0230fc20`).
    pub const FILL_MATRIX2: u32 = 0x0230_fc20;
    /// `g_pppFillMatrix3` (`0x0230fbe0`).
    pub const FILL_MATRIX3: u32 = 0x0230_fbe0;
    /// `g_AlphaScale` (`0x00c3a4d0`, f32).
    pub const ALPHA_SCALE: u32 = 0x00c3_a4d0;
    /// `f64` constants: 15 (`0x00b922c0`), -15 (`0x00b501c8`), 1/256
    /// (`0x00b50198`), 1/128 (`0x00b170b8`), 1/4096 (`0x00b50190`).
    pub const FIFTEEN: u32 = 0x00b9_22c0;
    /// See [`FIFTEEN`].
    pub const MINUS_FIFTEEN: u32 = 0x00b5_01c8;
    /// See [`FIFTEEN`].
    pub const INV_256: u32 = 0x00b5_0198;
    /// See [`FIFTEEN`].
    pub const INV_128: u32 = 0x00b1_70b8;
    /// See [`FIFTEEN`].
    pub const INV_4096: u32 = 0x00b5_0190;
    /// `f32` bounds: -100000 (`0x00b368b8`), 100000 (`0x00b10f50`).
    pub const F_LOW: u32 = 0x00b3_68b8;
    /// See [`F_LOW`].
    pub const F_HIGH: u32 = 0x00b1_0f50;

    /// Bytes of [`update_shp_oshape_vu1`]'s frame, from the original's
    /// `ebp - 0x84` up to `ebp - 4`: the two-corner quad's four vertices
    /// (x, y, 0, 1) at `+0` (`ebp - 0x84`), the four-corner quad's at
    /// `+0x40` (`ebp - 0x44`); the transforms see them.
    pub const SHP_FRAME: u32 = 0x80;

    /// The quads' layout: the record's flag bit 3.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Layout {
        /// Two corners, 0x20-byte records.
        Corners2,
        /// Four corners, 0x28-byte records.
        Corners4,
    }

    impl Layout {
        fn stride(self) -> u32 {
            match self {
                Self::Corners2 => 0x20,
                Self::Corners4 => 0x28,
            }
        }
        /// The vertices' frame offset.
        fn vertices(self) -> u32 {
            match self {
                Self::Corners2 => 0,
                Self::Corners4 => 0x40,
            }
        }
        /// The uvs' record offset.
        fn uvs(self) -> u32 {
            match self {
                Self::Corners2 => 0x10,
                Self::Corners4 => 0x18,
            }
        }
        /// The corners' divisor.
        fn divisor(self) -> u32 {
            match self {
                Self::Corners2 => MINUS_FIFTEEN,
                Self::Corners4 => FIFTEEN,
            }
        }
    }

    fn f64_at(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    fn f32_at(h: &impl Host, at: u32) -> f64 {
        load(h.read(at))
    }

    /// An i16 field as the original's `fild`.
    fn short(h: &impl Host, at: u32) -> f64 {
        f64::from(h.read::<i16>(at))
    }

    /// The segment's buffer at `off` (re-read from the render data each
    /// time, as the original).
    fn buffer(h: &impl Host, rd: u32, off: u32) -> u32 {
        let segment: u32 = h.read(rd + 0x94);
        h.read(segment.wrapping_add(off))
    }

    /// Copies 16 words from `from` to `to` (`rep movsd`).
    fn copy16(h: &mut impl Host, from: u32, to: u32) {
        for w in 0..16u32 {
            let v: u32 = h.read(from.wrapping_add(4 * w));
            h.write(to.wrapping_add(4 * w), v);
        }
    }

    /// `updateVFXRenderDataShp_OShapeVu1(lo, hi, shape, obj)` (`0x0071c0b0`,
    /// cdecl; `frame` see [`SHP_FRAME`]) -> 1, or 0 when the instance has no
    /// render data or the segment gets no buffer.
    pub fn update_shp_oshape_vu1(
        h: &mut impl Host,
        frame: u32,
        lo: u32,
        hi: u32,
        shape: u32,
        obj: u32,
    ) -> u32 {
        let geometry = h.call_sym(GEOMETRY, Conv::Cdecl, &[lo, hi]);
        let rd = h.call_sym(RENDER_DATA, Conv::Cdecl, &[geometry]);
        if rd == 0 {
            return 0;
        }
        if h.read::<u32>(obj + 0x1c) != 0 {
            h.write(rd + 0xb6, 1u8);
        }
        let count = i32::from(h.read::<i16>(shape + 2));
        let segment: u32 = h.read(rd + 0x94);
        let kind3 = h.read::<u8>(shape + 0xf) == 3;
        h.write(segment + 8, count.wrapping_mul(6).cast_unsigned());
        let count = i32::from(h.read::<i16>(shape + 2));
        let segment: u32 = h.read(rd + 0x94);
        h.write(segment + 4, count.wrapping_mul(4).cast_unsigned());
        if kind3 {
            let layout = if h.read::<u8>(shape) & 8 == 0 {
                Layout::Corners2
            } else {
                Layout::Corners4
            };
            if segment_buffer(h, rd) == 0 {
                return 0;
            }
            quads(h, frame, rd, shape, obj, layout);
        } else {
            if h.call_sym(FILL_VALID, Conv::Cdecl, &[]) != 0 {
                fill_request(h, geometry, rd, shape, obj);
            }
            if segment_buffer(h, rd) == 0 {
                return 0;
            }
        }
        // the original reads the kind again here
        if h.read::<u8>(shape + 0xf) == 3 {
            let fill = h.rebase(FILL_MATRIX);
            copy16(h, fill, rd);
            for off in [0xc, 0x1c, 0x2c] {
                h.write(rd + off, 0u32);
            }
            h.write(rd + 0x3c, 1f32.to_bits());
            h.write(rd + 0x80, 0u32);
            let low = store(f32_at(h, h.rebase(F_LOW)));
            for off in [0xa0, 0x9c, 0x98] {
                h.write(rd + off, low);
            }
            let high = store(f32_at(h, h.rebase(F_HIGH)));
            for off in [0xac, 0xa8, 0xa4] {
                h.write(rd + off, high);
            }
            h.call_sym(SET_VISIBLE, Conv::Cdecl, &[lo, hi, 1]);
        }
        1
    }

    /// `graphicDynGeoMemManagerGetBufferForSegment(rd, 0, +4, +8)` of the
    /// render data's segment.
    fn segment_buffer(h: &mut impl Host, rd: u32) -> u32 {
        let segment: u32 = h.read(rd + 0x94);
        let vertices: u32 = h.read(segment + 4);
        let indices: u32 = h.read(segment + 8);
        h.call_sym(SEGMENT_BUFFER, Conv::Cdecl, &[rd, 0, vertices, indices])
    }

    /// The job workers' fill request (kinds other than 3): the shape, the
    /// object's matrices, tint and flags, the three fill matrices.
    fn fill_request(h: &mut impl Host, geometry: u32, rd: u32, shape: u32, obj: u32) {
        let count = i32::from(h.read::<i16>(shape + 2)).cast_unsigned();
        let req = h.call_sym(FILL_REQUEST, Conv::Cdecl, &[count]);
        h.write(req, 1u8);
        let second = h.read::<u32>(obj + 0x1c) != 0;
        h.write(req + 1, u8::from(second));
        h.write(req + 0x10, shape);
        let flags = u32::from(h.read::<u16>(obj + 2));
        h.write(req + 0x44, flags);
        h.write(req + 0x14, geometry);
        h.write(req + 0x2c, rd);
        let m: u32 = h.read(obj + 0x10);
        let second = h.read::<u8>(req + 1) != 0;
        copy16(h, m, req + 0x48);
        if second {
            let m: u32 = h.read(obj + 0x1c);
            copy16(h, m, req + 0x88);
        }
        let tint: u32 = h.read(obj + 4);
        h.write(req + 0xc8, tint);
        h.write(rd + 0xc1, 1u8);
        h.write(req + 0xcc, 0u32);
        for (sym, off) in [
            (FILL_MATRIX, 0xd0),
            (FILL_MATRIX2, 0x110),
            (FILL_MATRIX3, 0x150),
        ] {
            let at = h.rebase(sym);
            copy16(h, at, req + off);
        }
    }

    /// Kind 3's quads into the segment's buffers, from quad 0.
    fn quads(h: &mut impl Host, frame: u32, rd: u32, shape: u32, obj: u32, layout: Layout) {
        let vertices = frame + layout.vertices();
        let mut q = 0u32;
        let mut record = shape + 0x18;
        while q.cast_signed() < i32::from(h.read::<i16>(shape + 2)) {
            let positions = buffer(h, rd, 0xc);
            corners(h, vertices, record, layout);
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
            if drawn {
                // the vertices as transformed: x, y, z of each
                let out = positions.wrapping_add(q.wrapping_mul(0x30));
                for n in 0..4u32 {
                    for k in 0..3u32 {
                        let x = store(f32_at(h, vertices + 16 * n + 4 * k));
                        h.write(out.wrapping_add(12 * n + 4 * k), x);
                    }
                }
                quad_colour(h, rd, obj, record, q);
                quad_uvs(h, rd, record, q, layout);
                let i = buffer(h, rd, 0x1c).wrapping_add(q.wrapping_mul(12));
                let base = q.wrapping_mul(4);
                for (k, d) in [0u32, 2, 1, 2, 3, 1].into_iter().enumerate() {
                    h.write(i.wrapping_add(2 * k as u32), base.wrapping_add(d) as u16);
                }
            } else {
                let i = buffer(h, rd, 0x1c).wrapping_add(q.wrapping_mul(12));
                for k in 0..3u32 {
                    h.write(i.wrapping_add(4 * k), 0u32);
                }
            }
            q = q.wrapping_add(1);
            record = record.wrapping_add(layout.stride());
        }
    }

    /// The quad's four vertices (x, y, 0, 1) at `vertices`: the i16 corners
    /// divided by the layout's divisor.
    fn corners(h: &mut impl Host, vertices: u32, record: u32, layout: Layout) {
        let d = f64_at(h, layout.divisor());
        let n = match layout {
            Layout::Corners2 => 4,
            Layout::Corners4 => 8,
        };
        let mut c = [0u32; 8];
        for (k, v) in c.iter_mut().enumerate().take(n) {
            *v = store(div(short(h, record + 8 + 2 * k as u32), d));
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

    /// The quad's colour: the record's RGBA (`+0` ..) by 1/256 (alpha by
    /// `g_AlphaScale`) times the object's tint (`+4` ..) by 1/128, four
    /// copies.
    fn quad_colour(h: &mut impl Host, rd: u32, obj: u32, record: u32, q: u32) {
        let inv256 = f64_at(h, INV_256);
        let rgba: [u8; 4] = std::array::from_fn(|k| h.read(record + k as u32));
        let r = store(mul(f64::from(rgba[0]), inv256));
        let g = store(mul(f64::from(rgba[1]), inv256));
        let b = store(mul(inv256, f64::from(rgba[2])));
        let alpha_scale = f32_at(h, h.rebase(ALPHA_SCALE));
        let a = store(div(f64::from(rgba[3]), alpha_scale));
        let inv128 = f64_at(h, INV_128);
        let tint: [u8; 4] = std::array::from_fn(|k| h.read(obj + 4 + k as u32));
        let r = store(mul(mul(f64::from(tint[0]), inv128), load(r)));
        let g = store(mul(mul(f64::from(tint[1]), inv128), load(g)));
        let b = store(mul(mul(f64::from(tint[2]), inv128), load(b)));
        let a = store(mul(mul(inv128, f64::from(tint[3])), load(a)));
        let out = buffer(h, rd, 0x14).wrapping_add(q.wrapping_mul(0x40));
        for n in 0..4u32 {
            for (k, v) in [r, g, b, a].into_iter().enumerate() {
                h.write(out.wrapping_add(16 * n + 4 * k as u32), v);
            }
        }
    }

    /// The quad's uvs: the i16 `u`, `v`, `du`, `dv` by 1/4096, `v` flipped
    /// (`1 - v`), placed per layout.
    fn quad_uvs(h: &mut impl Host, rd: u32, record: u32, q: u32, layout: Layout) {
        let at = record + layout.uvs();
        let inv4096 = f64_at(h, INV_4096);
        let u = store(mul(short(h, at), inv4096));
        let v = store(mul(short(h, at + 2), inv4096));
        let du = store(mul(short(h, at + 4), inv4096));
        let dv = store(mul(inv4096, short(h, at + 6)));
        let top = sub(1.0, load(v));
        let w = store(top);
        let x1 = store(add(load(du), load(u)));
        let y1 = store(sub(top, load(dv)));
        let uvs = match layout {
            Layout::Corners4 => [u, w, x1, w, u, y1, x1, y1],
            Layout::Corners2 => [x1, y1, u, y1, x1, w, u, w],
        };
        let out = buffer(h, rd, 0x18).wrapping_add(q.wrapping_mul(0x20));
        for (k, value) in uvs.into_iter().enumerate() {
            h.write(out.wrapping_add(4 * k as u32), value);
        }
    }
}
