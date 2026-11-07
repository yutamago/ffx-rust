//! M5 stage 1, round 9 (`tail9`, list `system-b`):
//! `JobSchedule_pppPart_fillBuffer_drawModelScroll` (`0x007269a0`), the
//! particle part job that fills a model's vertex buffers for the
//! scrolling-texture draw: every vertex transformed
//! (`FFXVu0ApplyMatrix`), then per triangle of the model's type-3 records
//! the positions (y scaled and shifted), the indices, the texture
//! coordinates (three modes: the record's, a second model's plus an
//! offset, or the normal through a second matrix; divided by the
//! transformed w) and the colours.
//!
//! The original keeps both vertex arrays in an 80 KB frame (their
//! addresses go to `FFXVu0ApplyMatrix`); the bridge takes them as one
//! buffer of `32 * n` bytes ([`bridge::arrays_size`]) the shim allocates.
//!
//! Junk reads (brief 2026-10-24): the original reads three frame words it
//! never writes on some paths, and the port uses 0 there:
//! - `[ebp-0x14078]` and `[ebp-0x14088]`, the next record of both models,
//!   when the first record is not of type 3 (the original walks on from
//!   stale stack words);
//! - `[ebp-0x34]` and `[ebp-0x30]`, the texture coordinates, in a mode
//!   past 2 (the original stores stale stack words as the coordinates).
//!
//! x87 code as the formulas at 53 bits, each value rounded to `f32`
//! where the original stores it.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `FFXVu0ApplyMatrix(out, matrix, v)` (cdecl).
    pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
    /// The doubles the job scales by: y (`0.5625`, `0x00b47c90`) and its
    /// shift (`350.0`, `0x00b501b0`), texture units (`1/4096`,
    /// `0x00b50190`), a half (`0x00b92220`), colours (`1/256`,
    /// `0x00b50198`) and the alpha byte (`1/128`, `0x00b170b8`).
    pub const Y_SCALE: u32 = 0x00b4_7c90;
    pub const Y_SHIFT: u32 = 0x00b5_01b0;
    pub const TEX_UNIT: u32 = 0x00b5_0190;
    pub const HALF: u32 = 0x00b9_2220;
    pub const COLOUR_UNIT: u32 = 0x00b5_0198;
    pub const ALPHA_UNIT: u32 = 0x00b1_70b8;
    /// The scratch the bridge's own `FFXVu0ApplyMatrix` calls use in mode
    /// 2: the normal (`+0`) and the result (`+0x10`).
    pub const SCRATCH: u32 = 0x20;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn k(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `fild` of an integer, stored as a float (`fstp dword`).
    fn int_float(v: i32) -> u32 {
        x87::store(f64::from(v))
    }

    /// The model's vertex count (`[[job + 0x14] + 0x10]`, i16): the
    /// buffer [`fill`] needs is `32 * count` bytes when it is positive.
    pub fn vertex_count(h: &impl Host, job: u32) -> i32 {
        let model: u32 = h.read(job.wrapping_add(0x14));
        i32::from(h.read::<i16>(model.wrapping_add(0x10)))
    }

    /// The bytes of the two vertex arrays for `count` vertices.
    #[must_use]
    pub fn arrays_size(count: i32) -> u32 {
        count.cast_unsigned().wrapping_mul(0x20)
    }

    /// `JobSchedule_pppPart_fillBuffer_drawModelScroll(job)` (cdecl) ->
    /// the `eax` it leaves (`[job + 0xcc]` without vertices, else the
    /// record past the last one). `arrays`: the raw (`+0`) and transformed
    /// (`+16 n`) vertices; `scratch`: [`SCRATCH`] bytes.
    ///
    /// `job`: `+0x10` the output (`+0x94` per record 0x6c-byte tables:
    /// `+0xc` positions, `+0x14` colours, `+0x18` texture coordinates,
    /// `+0x1c` indices), `+0x14` the model (`+4` records, `+8` positions
    /// (i16 triples), `+0xc` normals (i16 triples), `+0x10` the count),
    /// `+0x18` the second model (its records), `+0x20..+0x3c` two integer
    /// colours, `+0x40` the matrix, `+0x80` the normal matrix, `+0xc0`,
    /// `+0xc4` texture offsets, `+0xd0` the alpha scale, `+0xd4`, `+0xd8`
    /// the scroll, `+0xdc` the mode.
    #[allow(clippy::too_many_lines)]
    pub fn fill(h: &mut impl Host, arrays: u32, scratch: u32, job: u32) -> u32 {
        let at = |o: u32| job.wrapping_add(o);
        let model: u32 = h.read(at(0x14));
        let records0 = model.wrapping_add(h.read(model.wrapping_add(4)));
        let positions = model.wrapping_add(h.read(model.wrapping_add(8)));
        let normals = model.wrapping_add(h.read(model.wrapping_add(0xc)));
        let second: u32 = h.read(at(0x18));
        let records1 = second.wrapping_add(h.read(second.wrapping_add(4)));
        let n = i32::from(h.read::<i16>(model.wrapping_add(0x10)));
        let output: u32 = h.read(at(0x10));
        let (scroll_u, scroll_v, alpha) = (f(&*h, at(0xd4)), f(&*h, at(0xd8)), f(&*h, at(0xd0)));
        let (off_u, off_v) = (f(&*h, at(0xc0)), f(&*h, at(0xc4)));
        let mode: u32 = h.read(at(0xdc));
        let colours: [i32; 8] = [0, 1, 2, 3, 4, 5, 6, 7].map(|i| h.read(at(0x20 + 4 * i)));
        if n <= 0 {
            return h.read(at(0xcc));
        }
        let count = n.cast_unsigned();
        let transformed = arrays.wrapping_add(count.wrapping_mul(0x10));
        // every vertex, raw and transformed
        let one = x87::store(1.0);
        for i in 0..count {
            let src = positions.wrapping_add(i.wrapping_mul(6));
            let raw = arrays.wrapping_add(i.wrapping_mul(0x10));
            for j in 0..3 {
                let c = i32::from(h.read::<i16>(src.wrapping_add(2 * j)));
                h.write(raw.wrapping_add(4 * j), int_float(c));
            }
            h.write(raw.wrapping_add(0xc), one);
            let dst = transformed.wrapping_add(i.wrapping_mul(0x10));
            h.call_sym(APPLY_MATRIX, Conv::Cdecl, &[dst, at(0x40), raw]);
        }
        let (y_scale, y_shift, tex_unit) = (k(&*h, Y_SCALE), k(&*h, Y_SHIFT), k(&*h, TEX_UNIT));
        let (half, colour_unit, alpha_unit) =
            (k(&*h, HALF), k(&*h, COLOUR_UNIT), k(&*h, ALPHA_UNIT));
        let mut rec = records0;
        let mut rec1 = records1;
        // junk read: [ebp-0x14078] and [ebp-0x14088] never written before a
        // type-3 record; 0 (brief 2026-10-24)
        let (mut next0, mut next1) = (0_u32, 0_u32);
        // junk read: [ebp-0x34]/[ebp-0x30] never written in a mode past 2;
        // 0 (brief 2026-10-24)
        let (mut tex_u, mut tex_v) = (0_u32, 0_u32);
        let mut left = n;
        let mut table = 0_u32;
        let (normal_in, normal_out) = (scratch, scratch.wrapping_add(0x10));
        loop {
            if h.read::<u8>(rec.wrapping_add(1)) == 3 {
                let tables: u32 = h.read(output.wrapping_add(0x94));
                let t = |o: u32| tables.wrapping_add(table).wrapping_add(o);
                let mut idx_out: u32 = h.read(t(0x1c));
                let mut pos_out: u32 = h.read(t(0xc));
                let mut col_out: u32 = h.read(t(0x14));
                let mut uv_out: u32 = h.read(t(0x18));
                let mut item = rec.wrapping_add(0x10);
                let mut item1 = rec1.wrapping_add(0x10);
                let items = i32::from(h.read::<i16>(rec.wrapping_add(2)));
                let mut i = 0_i32;
                while i < items {
                    for corner in 0..3_u32 {
                        let v = u32::from(h.read::<u16>(item.wrapping_add(0xc + 2 * corner)));
                        let raw = arrays.wrapping_add(v.wrapping_mul(0x10));
                        for j in 0..3 {
                            let x = x87::store(f(&*h, raw.wrapping_add(4 * j)));
                            h.write(pos_out.wrapping_add(4 * j), x);
                        }
                        let y = x87::store(x87::mul(f(&*h, pos_out.wrapping_add(4)), y_scale));
                        let y = x87::store(x87::sub(x87::load(y), y_shift));
                        h.write(pos_out.wrapping_add(4), y);
                        pos_out = pos_out.wrapping_add(0xc);
                        let w = f(
                            &*h,
                            transformed
                                .wrapping_add(v.wrapping_mul(0x10))
                                .wrapping_add(0xc),
                        );
                        let w = x87::load(x87::store(w));
                        let index = i.cast_unsigned().wrapping_mul(3).wrapping_add(corner);
                        let [lo, hi, ..] = index.to_le_bytes();
                        h.write(idx_out, u16::from_le_bytes([lo, hi]));
                        idx_out = idx_out.wrapping_add(2);
                        // the texture coordinate before the division
                        let coord = |h: &dyn Fn(u32) -> u16, base: u32, o: u32, off: f64| {
                            let c = f64::from(h(base.wrapping_add(o)));
                            let s = x87::store(x87::add(c, off));
                            x87::store(x87::mul(x87::load(s), tex_unit))
                        };
                        let divide = |u: u32, v: u32| {
                            (
                                x87::store(x87::div(x87::load(u), w)),
                                x87::store(x87::div(x87::load(v), w)),
                            )
                        };
                        match mode {
                            0 | 1 => {
                                let base = if mode == 0 { item } else { item1 };
                                let rd = |a: u32| h.read::<u16>(a);
                                let mut u = coord(&rd, base, 0x14 + 4 * corner, off_u);
                                let mut v = coord(&rd, base, 0x16 + 4 * corner, off_v);
                                if mode == 1 {
                                    u = x87::store(x87::add(x87::load(u), scroll_u));
                                    v = x87::store(x87::add(x87::load(v), scroll_v));
                                }
                                (tex_u, tex_v) = divide(u, v);
                            }
                            2 => {
                                let ni =
                                    u32::from(h.read::<u16>(item.wrapping_add(0x20 + 2 * corner)));
                                let nrm = normals.wrapping_add(ni.wrapping_mul(6));
                                for j in 0..3 {
                                    let c = f64::from(h.read::<i16>(nrm.wrapping_add(2 * j)));
                                    h.write(
                                        normal_in.wrapping_add(4 * j),
                                        x87::store(x87::mul(c, tex_unit)),
                                    );
                                }
                                h.write(normal_in.wrapping_add(0xc), one);
                                h.call_sym(
                                    APPLY_MATRIX,
                                    Conv::Cdecl,
                                    &[normal_out, at(0x80), normal_in],
                                );
                                let o0 = f(&*h, normal_out);
                                let u = x87::store(x87::mul(x87::add(o0, 1.0), half));
                                let o1 = f(&*h, normal_out.wrapping_add(4));
                                let v = x87::store(x87::mul(x87::add(o1, 1.0), half));
                                h.write(normal_out.wrapping_add(8), one);
                                (tex_u, tex_v) = divide(u, v);
                            }
                            _ => {}
                        }
                        h.write(uv_out, tex_u);
                        let v1 = x87::store(x87::sub(1.0, x87::load(tex_v)));
                        h.write(uv_out.wrapping_add(4), v1);
                        uv_out = uv_out.wrapping_add(8);
                        let c = match mode {
                            0 => {
                                let b = |o: u32| {
                                    f64::from(h.read::<u8>(item.wrapping_add(4 * corner + o)))
                                };
                                let a = x87::store(x87::mul(x87::mul(b(3), alpha), alpha_unit));
                                [x87::store(b(0)), x87::store(b(1)), x87::store(b(2)), a]
                            }
                            1 => [0, 1, 2, 3].map(|i| int_float(colours[i])),
                            _ => [4, 5, 6, 7].map(|i| int_float(colours[i])),
                        };
                        for (j, v) in (0..3_u32).zip(c) {
                            h.write(
                                col_out.wrapping_add(4 * j),
                                x87::store(x87::mul(x87::load(v), colour_unit)),
                            );
                        }
                        let a = x87::store(x87::mul(x87::load(c[3]), colour_unit));
                        h.write(
                            col_out.wrapping_add(0xc),
                            x87::store(x87::mul(x87::load(a), half)),
                        );
                        col_out = col_out.wrapping_add(0x10);
                    }
                    i += 1;
                    item = item.wrapping_add(0x28);
                    item1 = item1.wrapping_add(0x28);
                }
                next0 = item;
                next1 = item1;
            }
            left -= i32::from(h.read::<i16>(rec.wrapping_add(2)));
            table = table.wrapping_add(0x6c);
            rec = next0;
            rec1 = next1;
            if left <= 0 {
                return rec;
            }
        }
    }
}
