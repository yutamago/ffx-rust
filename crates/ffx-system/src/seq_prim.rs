//! Menu-sequence primitives drawn as window-layout shapes
//! (`TOMakePktSeqPrimLayout_008fe0b0`, the scene sequences' type 6).
//!
//! A window layout (`TOCopyWindowLayout_008fa7d0`/`_008fa450`) is a
//! count word and 12-byte parts: sheet texel `u`, `v` (bytes), offset
//! `dx`, `dy` (`i8`), size `w`, `h` (`u16`) and two flip words. Each part
//! becomes a 0x98-byte UI record for `graphicDrawUIElement`.

/// The bridges over game memory.
pub mod bridge {
    use ffx_bridge::Mem;

    /// `x` rounded to a dword (`fstp dword`).
    #[allow(clippy::cast_possible_truncation)]
    fn dword(x: f64) -> f32 {
        x as f32
    }

    /// An integer as `fild` (exact) and `fstp dword` leave it.
    #[allow(clippy::cast_precision_loss)]
    fn int_f32(n: i32) -> f32 {
        n as f32
    }

    /// Scratch [`make_pkt_seq_prim_layout`] needs: the 0x98-byte UI
    /// record `graphicDrawUIElement` and `ShapeDataModify` read, then the
    /// sheet's image width and height (`f32`, from `TOGetImageWH`).
    pub const SEQ_PRIM_LAYOUT_SCRATCH: u32 = 0xa0;

    /// One window-layout part of [`make_pkt_seq_prim_layout`]: 12 bytes
    /// after the layout's count word.
    struct LayoutPart {
        /// The sheet texel of the top left corner.
        u: u8,
        v: u8,
        /// The offset of the top left corner, and the size.
        dx: i8,
        dy: i8,
        w: u16,
        h: u16,
        /// 1 mirrors the part horizontally / vertically.
        flip_x: u16,
        flip_y: u16,
    }

    impl LayoutPart {
        fn read(mem: &impl Mem, at: u32) -> Self {
            Self {
                u: mem.read(at),
                v: mem.read(at + 1),
                dx: mem.read(at + 2),
                dy: mem.read(at + 3),
                w: mem.read(at + 4),
                h: mem.read(at + 6),
                flip_x: mem.read(at + 8),
                flip_y: mem.read(at + 0xa),
            }
        }
    }

    /// `TOMakePktSeqPrimLayout_008fe0b0(prim)` (`0x008fe0b0`, own name):
    /// a menu-sequence primitive of type 6 (the caller `0x00a7bbc0`)
    /// drawn as a window-layout shape. `prim` holds the colour bytes
    /// (`+4..+7`, and the words `+4`/`+8` as corner colours), the
    /// position (`i16 +0xc`, `+0xe`), the scale in percent (`i16 +0x10`,
    /// `+0x12`, 100 = none) and the shape (`u16 +0x14`):
    ///
    /// - 0x19b, 0x19c, 0x19f, 0x1a3, 0x1fb: nothing;
    /// - 0x1a1 at `y` up to 0x14a: the gauge quad `TOMkpShapeXYWHUVC2(-1,
    ///   x0 - X2(w / 2), Y2(535 - h / 2), X2(w), Y2(h), 1759, 1, 1835, 77,
    ///   +4, +8)` with `w`, `h` = 120 x the scales, `x0` 160 when `x` is
    ///   0x74, else 388;
    /// - 0xf0: shape 200 at `(x - X2(10), y - Y2(185))`;
    /// - below 600 otherwise: each part of window layout `shape`
    ///   (`TOCopyWindowLayout_008fa7d0(shape)` below 200, `(shape - 200)`
    ///   below 400, both on sheet 0x3e00; `TOCopyWindowLayout_008fa450(
    ///   shape - 400)` on 0x3f00 below 600) as a UI record: corners
    ///   `ftol` of the scaled offsets plus the position (swapped by the
    ///   part's flip words), UVs over the sheet size, the four colour
    ///   bytes at both corners, `+0x80/+0x88/+0x90/+0x94` zero (once).
    ///   0x19e and 0x1fc first go through `ShapeDataModify(record,
    ///   0x102d / 0x1fc)`; 0x19d draws the first part only as the cursor
    ///   `0x008f59c0` and shape 400 (unless its `y` is past 300), 0x1f7
    ///   as shape 400; the rest `graphicDrawUIElement(record, texture,
    ///   1, 0, 0)`.
    ///
    /// From 600 up the original takes a stale stack word as the layout
    /// (undefined); the port draws nothing and returns the shape. Sizes
    /// through `graphicUiRemapX2`/`Y2` (`remap(mem, y_axis, v)`), callees
    /// through `call` (absolute addresses); `scratch` holds
    /// [`SEQ_PRIM_LAYOUT_SCRATCH`] bytes (the port's record). Returns the
    /// original's `eax`: the shape when nothing is drawn, 0x14a for a
    /// gauge below its line, the last shape call's result, the colour byte
    /// `+7` for an empty layout, 0x3000 for a cursor past 300, else the
    /// layout.
    pub fn make_pkt_seq_prim_layout<M: Mem>(
        mem: &mut M,
        prim: u32,
        scratch: u32,
        mut remap: impl FnMut(&mut M, bool, f32) -> f32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut run = |mem: &mut M, sym: u32, args: &[u32]| {
            let f = mem.rebase(sym);
            call(mem, f, args)
        };
        let kind: u16 = mem.read(prim + 0x14);
        if matches!(kind, 0x19f | 0x1a3 | 0x19b | 0x19c | 0x1fb) {
            return u32::from(kind);
        }
        if kind == 0x1a1 {
            return seq_prim_gauge(mem, prim, &mut remap, &mut run);
        }
        if kind == 0xf0 {
            let y = f64::from(mem.read::<i16>(prim + 0xe));
            let x = f64::from(mem.read::<i16>(prim + 0xc));
            let h = remap(mem, true, 186.0);
            let w = remap(mem, false, 89.0);
            let sy = f64::from(remap(mem, true, 185.0));
            let sy = dword(y - sy);
            let sx = f64::from(remap(mem, false, 10.0));
            let sx = dword(x - sx);
            let uv = [1.0 / 128.0, 0.0, 97.0 / 1024.0, 186.0 / 1024.0];
            let [u0, v0, u1, v1] = uv.map(f32::to_bits);
            let args = [
                200,
                sx.to_bits(),
                sy.to_bits(),
                w.to_bits(),
                h.to_bits(),
                u0,
                v0,
                u1,
                v1,
            ];
            return run(mem, 0x0090_3bb0, &args);
        }
        let (copy, index, sheet) = match kind {
            0..200 => (0x008f_a7d0, kind, 0x3e00),
            200..400 => (0x008f_a7d0, kind - 200, 0x3e00),
            400..600 => (0x008f_a450, kind - 400, 0x3f00),
            _ => return u32::from(kind),
        };
        let record = scratch;
        let (tex_w, tex_h) = (record + 0x98, record + 0x9c);
        let layout = run(mem, copy, &[u32::from(index)]);
        let texture = run(mem, 0x008a_c8c0, &[sheet]);
        run(mem, 0x008a_c400, &[sheet, tex_w, tex_h]);
        let colours: [u32; 4] = [4, 5, 6, 7].map(|k| u32::from(mem.read::<u8>(prim + k)));
        for off in [0x80, 0x88, 0x90, 0x94] {
            mem.write(record + off, 0u32);
        }
        if mem.read::<u32>(layout) == 0 {
            return colours[3];
        }
        let mut i = 0u32;
        loop {
            let part = LayoutPart::read(mem, layout + 4 + 0xc * i);
            fill_part_record(mem, prim, &part, record, colours);
            let kind: u16 = mem.read(prim + 0x14);
            match kind {
                0x19d => return seq_prim_cursor(mem, record, colours[3], &mut remap, &mut run),
                0x1f7 => return seq_prim_marker(mem, record, colours[3], &mut remap, &mut run),
                0x19e | 0x1fb | 0x1fc => {
                    let id = match kind {
                        0x19e => 0x102d,
                        // reached only when a callee rewrote the shape
                        0x1fb if i == 0 => 0x19b,
                        other => u32::from(other),
                    };
                    run(mem, 0x008e_8a10, &[record, id]);
                }
                _ => {}
            }
            run(mem, 0x0063_eee0, &[record, texture, 1, 0, 0]);
            i += 1;
            if i >= mem.read::<u32>(layout) {
                return layout;
            }
        }
    }

    /// The UI record at `record` for one layout part: both corners, their
    /// UVs over the sheet size after the record (`f32` width and height)
    /// and the four colour bytes at each.
    fn fill_part_record<M: Mem>(
        mem: &mut M,
        prim: u32,
        part: &LayoutPart,
        record: u32,
        colours: [u32; 4],
    ) {
        let [xs, ys] = seq_part_corners(mem, prim, part);
        let width = f64::from(mem.read::<f32>(record + 0x98));
        let height = f64::from(mem.read::<f32>(record + 0x9c));
        let (u, v) = (i32::from(part.u), i32::from(part.v));
        let u_end = int_f32(u + (i32::from(part.w) - 1));
        let v_end = int_f32(v - 1 + i32::from(part.h));
        let uvs = [
            [dword(f64::from(u) / width), dword(f64::from(v) / height)],
            [
                dword(f64::from(u_end) / width),
                dword(f64::from(v_end) / height),
            ],
        ];
        for (k, uv) in (0u32..).zip(uvs) {
            let at = record + 0x20 * k;
            mem.write(at, int_f32(xs[k as usize]));
            mem.write(at + 4, int_f32(ys[k as usize]));
            mem.write(at + 8, uv[0]);
            mem.write(at + 0xc, uv[1]);
            for (j, c) in (0u32..).zip(colours) {
                mem.write(at + 0x10 + 4 * j, c);
            }
        }
    }

    /// A layout part's two corners `[x0, x1], [y0, y1]`: its offsets
    /// (`dx`, `w + dx`; `dy`, `h + dy`) scaled by the primitive's percent
    /// (`/ 100 * scale` at double precision unless 100), `ftol`, plus the
    /// position; a flip word 1 swaps the pair.
    fn seq_part_corners<M: Mem>(mem: &mut M, prim: u32, part: &LayoutPart) -> [[i32; 2]; 2] {
        use ffx_bridge::x87::ftol;
        let dx = i32::from(part.dx);
        let dy = i32::from(part.dy);
        let mut xs = [f64::from(dx), f64::from(i32::from(part.w) + dx)];
        let mut ys = [f64::from(dy), f64::from(i32::from(part.h) + dy)];
        for (at, pair) in [(0x10, &mut xs), (0x12, &mut ys)] {
            let scale: i16 = mem.read(prim + at);
            if scale != 100 {
                let s = f64::from(scale);
                *pair = pair.map(|n| n / 100.0 * s);
            }
        }
        [(xs, part.flip_x, 0xc), (ys, part.flip_y, 0xe)].map(|(pair, flip, base)| {
            let base = i32::from(mem.read::<i16>(prim + base));
            let [first, second] = if flip == 1 { [pair[1], pair[0]] } else { pair };
            let first = ftol(mem, first).cast_signed().wrapping_add(base);
            let second = ftol(mem, second).cast_signed().wrapping_add(base);
            [first, second]
        })
    }

    /// The UI colour of the layout cursor and marker: colour byte `+7`
    /// as alpha over grey.
    fn seq_prim_grey(alpha: u32) -> u32 {
        (alpha << 24).wrapping_add(0x0080_8080)
    }

    /// A `TOMkpShapeXYWHUVC2` call with UVs `uv` and colours `c`.
    fn seq_prim_shape_c2<M: Mem>(
        mem: &mut M,
        run: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        shape: u32,
        [x, y, w, h]: [f32; 4],
        uv: [f32; 4],
        [c0, c1]: [u32; 2],
    ) -> u32 {
        let [u0, v0, u1, v1] = uv.map(f32::to_bits);
        let args = [
            shape,
            x.to_bits(),
            y.to_bits(),
            w.to_bits(),
            h.to_bits(),
            u0,
            v0,
            u1,
            v1,
            c0,
            c1,
        ];
        run(mem, 0x0090_3ee0, &args)
    }

    /// Shape 0x1a1 of [`make_pkt_seq_prim_layout`]: the gauge quad.
    fn seq_prim_gauge<M: Mem>(
        mem: &mut M,
        prim: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f32,
        run: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let scale_x = dword(f64::from(mem.read::<i16>(prim + 0x10)) / 100.0);
        let scale_y = dword(f64::from(mem.read::<i16>(prim + 0x12)) / 100.0);
        if mem.read::<i16>(prim + 0xe) > 0x14a {
            return 0x14a;
        }
        let colours = [mem.read(prim + 4), mem.read(prim + 8)];
        let left = if mem.read::<i16>(prim + 0xc) == 0x74 {
            160.0
        } else {
            388.0
        };
        let high = f64::from(scale_y) * 120.0;
        let wide = 120.0 * f64::from(scale_x);
        let h = remap(mem, true, dword(high));
        let w = remap(mem, false, dword(wide));
        let y = remap(mem, true, dword(535.0 - 0.5 * high));
        let x = dword(left - f64::from(remap(mem, false, dword(0.5 * wide))));
        seq_prim_shape_c2(
            mem,
            run,
            u32::MAX,
            [x, y, w, h],
            [1759.0, 1.0, 1835.0, 77.0],
            colours,
        )
    }

    /// Shape 0x19d of [`make_pkt_seq_prim_layout`]: the first part's
    /// corner as `0x008f59c0(ftol(x * 1920 / 512 - 21), ftol(y * 1080 /
    /// 416 - 18), 0x14c, 0x20, 0x112, 0x3d, alpha, alpha)` and shape 400
    /// beside it; nothing when its `y` is past 300 (the original then
    /// returns the x87 status word of its comparison: stack top 6, no
    /// condition bits, 0x3000 plus whatever sticky exception flags the
    /// thread has; the port returns 0x3000).
    fn seq_prim_cursor<M: Mem>(
        mem: &mut M,
        record: u32,
        alpha: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f32,
        run: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::ftol;
        let y = f64::from(mem.read::<f32>(record + 4));
        if y > 300.0 {
            return 0x3000;
        }
        let cy = ftol(mem, y * 1080.0 / 416.0 - 18.0);
        let x = f64::from(mem.read::<f32>(record));
        let cx = ftol(mem, x * 1920.0 * (1.0 / 512.0) - 21.0);
        run(
            mem,
            0x008f_59c0,
            &[cx, cy, 0x14c, 0x20, 0x112, 0x3d, alpha, alpha],
        );
        let y = f64::from(mem.read::<f32>(record + 4));
        let c1 = seq_prim_grey(mem.read(record + 0x3c));
        let c0 = seq_prim_grey(mem.read(record + 0x1c));
        let h = remap(mem, true, 24.0);
        let w = remap(mem, false, 201.0);
        let sy = dword(y - f64::from(remap(mem, true, 14.0)));
        let dx = f64::from(remap(mem, false, 13.0));
        let sx = dword(dx + f64::from(mem.read::<f32>(record)));
        let uv = [64.0, 467.0, 343.0, 504.0].map(|n| n / 1024.0);
        seq_prim_shape_c2(mem, run, 400, [sx, sy, w, h], uv, [c0, c1])
    }

    /// Shape 0x1f7 of [`make_pkt_seq_prim_layout`]: shape 400 left of and
    /// above the first part's corner.
    fn seq_prim_marker<M: Mem>(
        mem: &mut M,
        record: u32,
        alpha: u32,
        remap: &mut impl FnMut(&mut M, bool, f32) -> f32,
        run: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let y = f64::from(mem.read::<f32>(record + 4));
        let x = f64::from(mem.read::<f32>(record));
        let colour = seq_prim_grey(alpha);
        let h = remap(mem, true, 48.0);
        let w = remap(mem, false, 58.0);
        let sy = dword(y - f64::from(remap(mem, true, 14.0)));
        let sx = dword(x - f64::from(remap(mem, false, 53.0)));
        let uv = [705.0, 647.0, 763.0, 695.0].map(|n| n / 1024.0);
        seq_prim_shape_c2(mem, run, 400, [sx, sy, w, h], uv, [colour, colour])
    }
}
