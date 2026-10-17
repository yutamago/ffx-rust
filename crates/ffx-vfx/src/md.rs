//! The `op1_md_draw_*` model draws: they write their vertices straight
//! into a render-data buffer (positions, texture coordinates, colours and
//! indices), so the ports reproduce every float the PC renderer reads.

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87::{self, ADD, MUL, Reg, SUB};

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    fn f(mem: &impl Mem, a: u32) -> f32 {
        mem.read(a)
    }

    fn c64(mem: &impl Mem, a: u32) -> f64 {
        mem.read(g(mem, a))
    }

    /// The screen mapping of `op1_md_draw_eiabm_sphe`: `ox`, `oy` (the
    /// camera's projected centre, `+0x30`/`+0x34` over `+0x3c`, less 2048,
    /// plus 256 and 208), `sx` = width / 512, `sy` = height / 416, `z` (the
    /// camera's `+0x2c`, 1 when 0), `asp` = `z * 4 sy / (3 sx)`, half the
    /// width and height; each an `f32`.
    #[derive(Debug, Clone, Copy)]
    struct Screen {
        ox: f32,
        oy: f32,
        sx: f32,
        sy: f32,
        z: f32,
        asp: f32,
        cx: f32,
        cy: f32,
    }

    impl Screen {
        /// `(a * k + o) * s - c` or, with `minus`, `(o - a * k) * s - c`,
        /// in 53 bits, as `f32`.
        #[allow(clippy::many_single_char_names)]
        fn map(a: i16, k: f32, o: f32, s: f32, c: f32, minus: bool) -> f32 {
            let p = Reg::Num(f64::from(a)).mem(MUL, k);
            let t = if minus {
                Reg::load_f32(o).reg(SUB, p)
            } else {
                p.reg(ADD, Reg::load_f32(o))
            };
            t.mem(MUL, s).reg(SUB, Reg::load_f32(c)).store()
        }

        fn x(&self, a: i16, minus: bool) -> f32 {
            Self::map(a, self.asp, self.ox, self.sx, self.cx, minus)
        }

        fn y(&self, a: i16, minus: bool) -> f32 {
            Self::map(a, self.z, self.oy, self.sy, self.cy, minus)
        }
    }

    /// `op1_md_draw_eiabm_sphe(obj, part, n, e)` (`0x00a68140`): one
    /// screen-facing quad of a model's sphere effect into the
    /// `NoTexture` render data (`graphicAbmapGetVertexInfo("NoTexture",
    /// 7) +0x94`), at vertex `v` = `(7 n + e) * 4` for `e >= 0`, else
    /// `(7 n - e) * 4 - 4`. Nothing when `g_GraphicSkipFrame` is set.
    /// - Colours (`+0x14`, four per vertex): the part's 12 colour bytes
    ///   (`obj + obj[1] + 0x10`.., the last four twice) times the tint
    ///   (`part` bytes `+0x18`.. times 1/128, `FFXVu0MulVector`), / 255.
    /// - Positions (`+0xc`, three per vertex): with tint alpha 0 all at
    ///   (-10000, -10000); else the quad of the two vertices `u16 +0x1e`
    ///   and `+0x20` of the part (`i16` x, y at `obj + obj[2]`, 6 bytes
    ///   each) through the screen mapping: (+a, +b), (-c, -d), (+c, +d),
    ///   (-a, -b); depth `[part +0x44] +0x38`.
    /// - For `e >= 0` also texture coordinates (0, 0, 1) each (`+0x10`),
    ///   six indices `v, v+1, v+2, v+2, v+1, v+3` at `f32(v) * 1.5`
    ///   (`+0x1c`), and the vertex and index counts (`+4`, `+8`) up by 4
    ///   and 6.
    ///
    /// Callees through `call`; the locals lie below `frame` (from `frame -
    /// 0xa4`). Returns 0.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn op1_md_draw_eiabm_sphe<M: Mem>(
        mem: &mut M,
        obj: u32,
        part: u32,
        n: u32,
        e: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let div: x87::Op = |a, b| a / b;
        let (tint, k, wh, cols) = (
            frame.wrapping_sub(0x54),
            frame.wrapping_sub(0x64),
            frame.wrapping_sub(0xa4),
            frame.wrapping_sub(0x44),
        );
        for i in 0..4u32 {
            mem.write(tint + 4 * i, 1.0f32);
        }
        let eighth: u32 = mem.read(g(mem, 0x00b5_1640));
        for i in 0..4u32 {
            mem.write(k + 4 * i, x87::load(eighth));
        }
        let pd: u32 = mem.read(part.wrapping_add(0x44));
        let depth = f32::from_bits(x87::load(mem.read(pd.wrapping_add(0x38))));
        if mem.read::<u32>(g(mem, 0x012f_b7d0)) != 0 {
            return 0;
        }
        let verts = obj.wrapping_add(mem.read(obj.wrapping_add(8)));
        let off: u32 = mem.read(obj.wrapping_add(4));
        let cam: u32 = mem.read(part.wrapping_add(0x20));
        let w3c = f(mem, cam.wrapping_add(0x3c));
        let ox = Reg::load_f32(f(mem, cam.wrapping_add(0x30)))
            .mem(div, w3c)
            .constant(SUB, c64(mem, 0x00b5_1bc0))
            .constant(ADD, c64(mem, 0x00b9_22f0))
            .store();
        let oy = Reg::load_f32(f(mem, cam.wrapping_add(0x34)))
            .mem(div, w3c)
            .constant(SUB, c64(mem, 0x00b5_1bc0))
            .constant(ADD, c64(mem, 0x00b5_4c70))
            .store();
        for i in 0..4u32 {
            let b = f32::from(mem.read::<u8>(part.wrapping_add(0x18 + i)));
            mem.write(tint + 4 * i, b);
        }
        let mulv = g(mem, 0x006e_d5d0);
        call(mem, mulv, &[tint, k, tint]);
        let font = g(mem, 0x0064_0db0);
        call(mem, font, &[wh, wh + 4]);
        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
        let (wf, hf) = (mem.read::<i32>(wh) as f32, mem.read::<i32>(wh + 4) as f32);
        let sx = Reg::load_f32(wf)
            .constant(MUL, c64(mem, 0x00b4_1cc0))
            .store();
        let sy = Reg::load_f32(hf)
            .constant(div, c64(mem, 0x00b4_1d30))
            .store();
        let z0 = x87::load_f32(f(mem, cam.wrapping_add(0x2c)));
        let z = if z0 == 0.0 { 1.0 } else { z0 };
        let num = Reg::load_f32(sy)
            .constant(MUL, c64(mem, 0x00b4_3d80))
            .reg(MUL, Reg::load_f32(z));
        let den = Reg::load_f32(sx).constant(MUL, c64(mem, 0x00b2_c5c8));
        let asp = num.reg(div, den).store();
        let half = c64(mem, 0x00b9_2220);
        let cx = Reg::load_f32(wf).constant(MUL, half).store();
        let cy = Reg::load_f32(hf).constant(MUL, half).store();
        let s = Screen {
            ox,
            oy,
            sx,
            sy,
            z,
            asp,
            cx,
            cy,
        };
        let name = g(mem, 0x00b4_1b50);
        let lookup = g(mem, 0x0063_8fd0);
        let tex = call(mem, lookup, &[name, 7]);
        let rd: u32 = mem.read(tex.wrapping_add(0x94));
        let ec = e.cast_signed();
        let v = if ec >= 0 {
            n.wrapping_mul(7).wrapping_add(e).wrapping_mul(4)
        } else {
            n.wrapping_mul(7)
                .wrapping_sub(e)
                .wrapping_mul(4)
                .wrapping_sub(4)
        };
        // the colours: 12 bytes, the last four twice
        let src = obj.wrapping_add(off).wrapping_add(0x10);
        for i in 0..16u32 {
            let j = if i < 12 { i } else { i - 4 };
            let b = f32::from(mem.read::<u8>(src.wrapping_add(j)));
            mem.write(cols + 4 * i, b);
        }
        for q in 0..4u32 {
            let c = cols + 0x10 * q;
            call(mem, mulv, &[c, tint, c]);
        }
        let k255 = c64(mem, 0x00b1_70c0);
        let col: u32 = mem.read(rd.wrapping_add(0x14));
        let cbase = col.wrapping_add(v.wrapping_mul(16));
        for i in 0..16u32 {
            let x = Reg::load_f32(f(mem, cols + 4 * i))
                .constant(div, k255)
                .store();
            mem.write(cbase.wrapping_add(4 * i), x);
        }
        let pos: u32 = mem.read(rd.wrapping_add(0xc));
        let pbase = pos.wrapping_add(v.wrapping_mul(12));
        let mut p = [0f32; 12];
        if mem.read::<u8>(part.wrapping_add(0x1b)) == 0 {
            let far = f(mem, g(mem, 0x00b4_1db0));
            for q in 0..4 {
                p[3 * q] = far;
                p[3 * q + 1] = far;
                p[3 * q + 2] = depth;
            }
        } else {
            let at = |mem: &M, field: u32, o: u32| {
                let i = u32::from(mem.read::<u16>(obj.wrapping_add(off).wrapping_add(field)));
                mem.read::<i16>(verts.wrapping_add(i * 6 + o))
            };
            let (a, b) = (at(mem, 0x1e, 0), at(mem, 0x1e, 2));
            let (c, d) = (at(mem, 0x20, 0), at(mem, 0x20, 2));
            p = [
                s.x(a, false),
                s.y(b, false),
                depth,
                s.x(c, true),
                s.y(d, true),
                depth,
                s.x(c, false),
                s.y(d, false),
                depth,
                s.x(a, true),
                s.y(b, true),
                depth,
            ];
        }
        for (i, x) in (0u32..).zip(p) {
            mem.write(pbase.wrapping_add(4 * i), x);
        }
        if ec < 0 {
            return 0;
        }
        let uv: u32 = mem.read(rd.wrapping_add(0x10));
        let ubase = uv.wrapping_add(v.wrapping_mul(12));
        for q in 0..4u32 {
            mem.write(ubase + 12 * q, [0.0f32, 0.0, 1.0]);
        }
        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
        let vf = v.cast_signed() as f32;
        let at = ffx_bridge::x87::ftol(
            mem,
            Reg::load_f32(vf)
                .constant(MUL, c64(mem, 0x00b4_1d00))
                .value(),
        );
        let idx: u32 = mem.read(rd.wrapping_add(0x1c));
        let ibase = idx.wrapping_add(at.wrapping_mul(2));
        #[allow(clippy::cast_possible_truncation)] // the u16 indices
        let v16 = v as u16;
        for (i, d) in (0u32..).zip([0u16, 1, 2, 2, 1, 3]) {
            mem.write(ibase.wrapping_add(2 * i), v16.wrapping_add(d));
        }
        let nv: u32 = mem.read(rd.wrapping_add(4));
        mem.write(rd.wrapping_add(4), nv.wrapping_add(4));
        let ni: u32 = mem.read(rd.wrapping_add(8));
        mem.write(rd.wrapping_add(8), ni.wrapping_add(6));
        0
    }

    /// The lit variant of a primitive: where its normal indices lie in the
    /// record, and the locals of its lighting (below `frame`).
    #[derive(Debug, Clone, Copy)]
    struct Lit {
        /// The record offset of the `u16` normal indices.
        normals: u32,
        /// The transformed vertices (16 bytes each, `w` 1).
        verts: u32,
        /// The normals (16 bytes each, `w` 0).
        norms: u32,
        /// The light matrix.
        matrix: u32,
        /// The lit result.
        res: u32,
        /// `true` when the matrix sums take the second row's product
        /// first (the triangles), else the first row's (the quads).
        p1_first: bool,
    }

    /// One primitive kind of `op1_md_draw_eiabm`.
    #[derive(Debug, Clone, Copy)]
    struct Prim {
        /// Vertices: 3 (a triangle) or 4 (a quad, indices `0 1 2 2 1 3`).
        n: u32,
        /// The record offset of the `u16` vertex indices (the colours, `4
        /// n` bytes, come first).
        verts: u32,
        /// The colour local (below `frame`), `4 n` floats.
        cols: u32,
        /// The lighting, for the lit kinds.
        lit: Option<Lit>,
    }

    /// The shared state of one draw.
    struct Draw {
        s: Screen,
        depth: f32,
        part: u32,
        rd: u32,
        verts: u32,
        normals: u32,
        frame: u32,
        tint: u32,
        mulv: u32,
        apply: u32,
        /// `part` bit `0x40`: light the lit kinds.
        lit: bool,
        /// The normalised rotation rows (below `frame`).
        norm: u32,
    }

    /// `f32` element `i` of render-data stream `field`, its pointer read
    /// anew as the original does for every store.
    fn put_f32(mem: &mut impl Mem, rd: u32, field: u32, i: u32, x: f32) {
        let base: u32 = mem.read(rd.wrapping_add(field));
        mem.write(base.wrapping_add(i.wrapping_mul(4)), x);
    }

    /// `_CIsqrt` at 53 bits (`fsqrt`), stored as `f32`.
    fn sqrt_f32(r: Reg) -> f32 {
        match r {
            Reg::Num(x) if x < 0.0 => f32::from_bits(x87::INDEFINITE),
            #[allow(clippy::cast_possible_truncation)] // the fstp m32 rounding
            Reg::Num(x) => x.sqrt() as f32,
            Reg::Nan(b) => f32::from_bits(b),
        }
    }

    /// The lighting clamp: each of the four floats at `at` below 0 to 0.
    fn clamp(mem: &mut impl Mem, at: u32) {
        for c in 0..4u32 {
            let x: f32 = mem.read(at + 4 * c);
            if x < 0.0 {
                mem.write(at + 4 * c, 0.0f32);
            }
        }
    }

    impl Draw {
        /// One primitive record at `data`.
        #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
        fn prim<M: Mem>(
            &self,
            mem: &mut M,
            k: Prim,
            data: u32,
            call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        ) {
            let div: x87::Op = |a, b| a / b;
            let rd = self.rd;
            let vtx = |mem: &M, field: u32, j: u32, o: u32| {
                let i = u32::from(mem.read::<u16>(data.wrapping_add(field + 2 * j)));
                mem.read::<i16>(self.verts.wrapping_add(i * 6 + o))
            };
            // the positions
            let v: u32 = mem.read(rd.wrapping_add(4));
            let p = v.wrapping_mul(3);
            for j in 0..k.n {
                let x = self.s.x(vtx(mem, k.verts, j, 0), false);
                put_f32(mem, rd, 0xc, p.wrapping_add(3 * j), x);
                let y = self.s.y(vtx(mem, k.verts, j, 2), false);
                put_f32(mem, rd, 0xc, p.wrapping_add(3 * j + 1), y);
                put_f32(mem, rd, 0xc, p.wrapping_add(3 * j + 2), self.depth);
            }
            // the texture coordinates
            for j in 0..k.n {
                for (c, x) in (0u32..).zip([0.0f32, 0.0, 1.0]) {
                    put_f32(mem, rd, 0x10, p.wrapping_add(3 * j + c), x);
                }
            }
            // the colours, times the tint
            let cols = self.frame.wrapping_sub(k.cols);
            for i in 0..4 * k.n {
                let b = f32::from(mem.read::<u8>(data.wrapping_add(i)));
                mem.write(cols + 4 * i, b);
            }
            for j in 0..k.n {
                let c = cols + 16 * j;
                call(mem, self.mulv, &[c, self.tint, c]);
            }
            if let (Some(l), true) = (k.lit, self.lit) {
                self.light(mem, k, l, data, cols, call);
            }
            let k255 = c64(mem, 0x00b1_70c0);
            let v: u32 = mem.read(rd.wrapping_add(4));
            let c0 = v.wrapping_mul(4);
            for i in 0..4 * k.n {
                let x = Reg::load_f32(f(mem, cols + 4 * i))
                    .constant(div, k255)
                    .store();
                put_f32(mem, rd, 0x14, c0.wrapping_add(i), x);
            }
            // the indices and counts
            let v: u32 = mem.read(rd.wrapping_add(4));
            let at: u32 = mem.read(rd.wrapping_add(8));
            let order: &[u16] = if k.n == 3 {
                &[0, 1, 2]
            } else {
                &[0, 1, 2, 2, 1, 3]
            };
            #[allow(clippy::cast_possible_truncation)] // the u16 indices
            let v16 = v as u16;
            for (i, d) in (0u32..).zip(order) {
                let base: u32 = mem.read(rd.wrapping_add(0x1c));
                mem.write(
                    base.wrapping_add(at.wrapping_add(i).wrapping_mul(2)),
                    v16.wrapping_add(*d),
                );
            }
            let nv: u32 = mem.read(rd.wrapping_add(4));
            mem.write(rd.wrapping_add(4), nv.wrapping_add(k.n));
            let ni: u32 = mem.read(rd.wrapping_add(8));
            #[allow(clippy::cast_possible_truncation)] // 3 or 6
            mem.write(rd.wrapping_add(8), ni.wrapping_add(order.len() as u32));
        }

        /// The lighting of a lit primitive: each vertex's colour times its
        /// four lights (`[part +0x28]`, 16 bytes each: position, intensity)
        /// through the normal.
        #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
        fn light<M: Mem>(
            &self,
            mem: &mut M,
            k: Prim,
            l: Lit,
            data: u32,
            cols: u32,
            call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        ) {
            let div: x87::Op = |a, b| a / b;
            let divr: x87::Op = |a, b| b / a;
            let part = self.part;
            let scale = Reg::Num(f64::from(mem.read::<u8>(part.wrapping_add(0x4d))))
                .constant(MUL, c64(mem, 0x00b1_70b8))
                .store();
            let (vl, nl) = (
                self.frame.wrapping_sub(l.verts),
                self.frame.wrapping_sub(l.norms),
            );
            let quarter = c64(mem, 0x00b5_0190);
            for j in 0..k.n {
                let i = u32::from(mem.read::<u16>(data.wrapping_add(k.verts + 2 * j)));
                for c in 0..3u32 {
                    let a = mem.read::<i16>(self.verts.wrapping_add(i * 6 + 2 * c));
                    mem.write(vl + 16 * j + 4 * c, f32::from(a));
                }
                mem.write(vl + 16 * j + 12, 1.0f32);
            }
            for j in 0..k.n {
                let i = u32::from(mem.read::<u16>(data.wrapping_add(l.normals + 2 * j)));
                for c in 0..3u32 {
                    let a = mem.read::<i16>(self.normals.wrapping_add(i * 6 + 2 * c));
                    let x = Reg::Num(f64::from(a)).constant(MUL, quarter).store();
                    mem.write(nl + 16 * j + 4 * c, x);
                }
                mem.write(nl + 16 * j + 12, 0.0f32);
            }
            for j in 0..k.n {
                let m: u32 = mem.read(part.wrapping_add(0x2c));
                call(mem, self.apply, &[vl + 16 * j, m, vl + 16 * j]);
            }
            let (om, res) = (
                self.frame.wrapping_sub(l.matrix),
                self.frame.wrapping_sub(l.res),
            );
            for j in 0..k.n {
                // the light directions over their squared distances
                let lights: u32 = mem.read(part.wrapping_add(0x28));
                let pv = vl + 16 * j;
                let (px, py, pz) = (f(mem, pv), f(mem, pv + 4), f(mem, pv + 8));
                let mut rows = [[0f32; 4]; 4];
                for (i, row) in (0u32..).zip(&mut rows) {
                    let li = lights.wrapping_add(16 * i);
                    let at = |o: u32| f(mem, li.wrapping_add(o));
                    let dx = Reg::load_f32(at(0)).mem(SUB, px).store();
                    let dy = Reg::load_f32(at(4)).mem(SUB, py).store();
                    let dz = Reg::load_f32(at(8)).mem(SUB, pz).store();
                    let sq = |x: f32| Reg::load_f32(x).reg(MUL, Reg::load_f32(x));
                    let a = sq(dx).constant(ADD, 0.0).store();
                    let b = Reg::load_f32(a).reg(ADD, sq(dy)).store();
                    let r = Reg::load_f32(b).reg(ADD, sq(dz)).store();
                    let w = at(0xc);
                    let t = Reg::load_f32(dx).mem(MUL, w).store();
                    row[0] = Reg::load_f32(t).reg(div, Reg::load_f32(r)).store();
                    let u = Reg::load_f32(dy).mem(MUL, w).store();
                    row[1] = Reg::load_f32(u).reg(div, Reg::load_f32(r)).store();
                    let z = Reg::load_f32(dz).mem(MUL, w).store();
                    row[2] = Reg::load_f32(r).mem(divr, z).store();
                }
                // times the normalised rotation, the translation below
                for r in 0..3u32 {
                    let nr = self.norm + 16 * r;
                    let (n0, n1, n2) = (f(mem, nr), f(mem, nr + 4), f(mem, nr + 8));
                    for (m, row) in (0u32..).zip(&rows) {
                        let p0 = Reg::load_f32(row[0]).mem(MUL, n0);
                        let p1 = Reg::load_f32(row[1]).mem(MUL, n1);
                        let s = if l.p1_first {
                            p1.reg(ADD, p0)
                        } else {
                            p0.reg(ADD, p1)
                        };
                        let x = s.reg(ADD, Reg::load_f32(row[2]).mem(MUL, n2)).store();
                        mem.write(om + 16 * r + 4 * m, x);
                    }
                }
                let mt: u32 = mem.read(part.wrapping_add(0x2c));
                for c in 0..4u32 {
                    let x = x87::load(mem.read(mt.wrapping_add(0x30 + 4 * c)));
                    mem.write(om + 0x30 + 4 * c, x);
                }
                call(mem, self.apply, &[res, om, nl + 16 * j]);
                clamp(mem, res);
                let m24: u32 = mem.read(part.wrapping_add(0x24));
                call(mem, self.apply, &[res, m24, res]);
                clamp(mem, res);
                for c in 0..4u32 {
                    let x = Reg::load_f32(f(mem, res + 4 * c)).mem(MUL, scale).store();
                    mem.write(res + 4 * c, x);
                }
                let cj = cols + 16 * j;
                for c in 0..3u32 {
                    let x = Reg::load_f32(f(mem, res + 4 * c))
                        .mem(MUL, f(mem, cj + 4 * c))
                        .store();
                    mem.write(cj + 4 * c, x);
                }
            }
        }
    }

    /// `op1_md_draw_eiabm(obj, part, slot)` (`0x00a657c0`): a model's
    /// primitives straight into the `NoTexture` render data
    /// (`graphicAbmapGetVertexInfo("NoTexture", slot) +0x94`, `slot` 4 or 8 from
    /// the callers; positions
    /// `+0xc`, texture coordinates `+0x10`, colours `+0x14`, indices
    /// `+0x1c`, vertex and index counts `+4`, `+8`). Nothing when
    /// `g_GraphicSkipFrame` is set.
    ///
    /// `obj`: `i16 +0x10` primitives in all, in blocks from `obj + obj[1]`
    /// (a 16-byte header: kind `i8 +1`, count `i16 +2`; the records follow,
    /// the next header after them); vertices at `obj + obj[2]`, normals at
    /// `obj + obj[3]` (three `i16` each). The kinds:
    /// - 0, 1: triangles of 20 or 28 bytes (12 colour bytes, vertex
    ///   indices `u16 +0xc`.., for 1 normal indices `+0x14`..);
    /// - 4, 5: quads of 24 or 32 bytes (16 colour bytes, vertex indices
    ///   `+0x10`.., for 5 normal indices `+0x18`..);
    /// - 2, 3 and 6, 7: skipped (32, 40 or 48 bytes a record).
    ///
    /// Each vertex goes through the screen mapping of
    /// `op1_md_draw_eiabm_sphe` at depth `[part +0x44] +0x38`, with
    /// texture coordinates (0, 0, 1); each colour times the tint (`part`
    /// bytes `+0x18`.. times 1/128, `FFXVu0MulVector`), then / 255; the
    /// indices `v, v+1, v+2` (and `v+2, v+1, v+3` for quads).
    ///
    /// With `part` bit `0x40` the lit kinds (1, 5) also light each vertex:
    /// the vertex through `[part +0x2c]` (`FFXVu0ApplyMatrix`), the four
    /// lights' directions over their squared distances times the rotation
    /// of `[part +0x2c]` with its rows normalised (`_CIsqrt`, once per
    /// draw), applied to the normal, clamped at 0, through `[part +0x24]`,
    /// clamped, times `part +0x4d` / 128, times the colour's red, green
    /// and blue.
    ///
    /// Callees through `call`; the locals lie below `frame` (from `frame -
    /// 0x448`). Returns 0.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn op1_md_draw_eiabm<M: Mem>(
        mem: &mut M,
        obj: u32,
        part: u32,
        slot: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let div: x87::Op = |a, b| a / b;
        let (tint, k, w_at, h_at) = (
            frame.wrapping_sub(0x14),
            frame.wrapping_sub(0x194),
            frame.wrapping_sub(0x444),
            frame.wrapping_sub(0x448),
        );
        for i in 0..4u32 {
            mem.write(tint + 4 * i, 1.0f32);
        }
        let eighth: u32 = mem.read(g(mem, 0x00b5_1640));
        for i in 0..4u32 {
            mem.write(k + 4 * i, x87::load(eighth));
        }
        let pd: u32 = mem.read(part.wrapping_add(0x44));
        let depth = f32::from_bits(x87::load(mem.read(pd.wrapping_add(0x38))));
        if mem.read::<u32>(g(mem, 0x012f_b7d0)) != 0 {
            return 0;
        }
        let name = g(mem, 0x00b4_1b50);
        let lookup = g(mem, 0x0063_8fd0);
        let tex = call(mem, lookup, &[name, slot]);
        let rd: u32 = mem.read(tex.wrapping_add(0x94));
        let mut left = i32::from(mem.read::<i16>(obj.wrapping_add(0x10)));
        let verts = obj.wrapping_add(mem.read(obj.wrapping_add(8)));
        let mut hdr = obj.wrapping_add(mem.read(obj.wrapping_add(4)));
        let normals = obj.wrapping_add(mem.read(obj.wrapping_add(0xc)));
        let cam: u32 = mem.read(part.wrapping_add(0x20));
        let w3c = f(mem, cam.wrapping_add(0x3c));
        let ox = Reg::load_f32(f(mem, cam.wrapping_add(0x30)))
            .mem(div, w3c)
            .constant(SUB, c64(mem, 0x00b5_1bc0))
            .constant(ADD, c64(mem, 0x00b9_22f0))
            .store();
        let oy = Reg::load_f32(f(mem, cam.wrapping_add(0x34)))
            .mem(div, w3c)
            .constant(SUB, c64(mem, 0x00b5_1bc0))
            .constant(ADD, c64(mem, 0x00b5_4c70))
            .store();
        let font = g(mem, 0x0064_0db0);
        call(mem, font, &[w_at, h_at]);
        #[allow(clippy::cast_precision_loss)] // fild, fstp m32
        let (wf, hf) = (mem.read::<i32>(w_at) as f32, mem.read::<i32>(h_at) as f32);
        let sx = Reg::load_f32(wf)
            .constant(MUL, c64(mem, 0x00b4_1cc0))
            .store();
        let sy = Reg::load_f32(hf)
            .constant(div, c64(mem, 0x00b4_1d30))
            .store();
        let z0 = x87::load_f32(f(mem, cam.wrapping_add(0x2c)));
        let z = if z0 == 0.0 { 1.0 } else { z0 };
        let num = Reg::load_f32(sy)
            .constant(MUL, c64(mem, 0x00b4_3d80))
            .reg(MUL, Reg::load_f32(z));
        let den = Reg::load_f32(sx).constant(MUL, c64(mem, 0x00b2_c5c8));
        let asp = num.reg(div, den).store();
        let half = c64(mem, 0x00b9_2220);
        let cx = Reg::load_f32(wf).constant(MUL, half).store();
        let cy = Reg::load_f32(hf).constant(MUL, half).store();
        for i in 0..4u32 {
            let b = f32::from(mem.read::<u8>(part.wrapping_add(0x18 + i)));
            mem.write(tint + 4 * i, b);
        }
        let mulv = g(mem, 0x006e_d5d0);
        call(mem, mulv, &[tint, k, tint]);
        let lit = mem.read::<u8>(part) & 0x40 != 0;
        let norm = frame.wrapping_sub(0x2c4);
        if lit {
            // the rotation rows of [part +0x2c], normalised
            let m: u32 = mem.read(part.wrapping_add(0x2c));
            for r in 0..3u32 {
                let row = m.wrapping_add(16 * r);
                let (r0, r1, r2) = (
                    f(mem, row),
                    f(mem, row.wrapping_add(4)),
                    f(mem, row.wrapping_add(8)),
                );
                let sq = |x: f32| Reg::load_f32(x).reg(MUL, Reg::load_f32(x));
                let root = sqrt_f32(sq(r0).reg(ADD, sq(r1)).reg(ADD, sq(r2)));
                let inv = Reg::Num(1.0).reg(div, Reg::load_f32(root)).store();
                let out = norm + 16 * r;
                let n0 = Reg::load_f32(r0).reg(MUL, Reg::load_f32(inv)).store();
                mem.write(out, n0);
                mem.write(out + 4, Reg::load_f32(inv).mem(MUL, r1).store());
                mem.write(out + 8, Reg::load_f32(inv).mem(MUL, r2).store());
                let w = x87::load(mem.read(row.wrapping_add(0xc)));
                mem.write(out + 12, w);
            }
            for c in 0..4u32 {
                let x = x87::load(mem.read(m.wrapping_add(0x30 + 4 * c)));
                mem.write(frame.wrapping_sub(0x294) + 4 * c, x);
            }
        }
        let d = Draw {
            s: Screen {
                ox,
                oy,
                sx,
                sy,
                z,
                asp,
                cx,
                cy,
            },
            depth,
            part,
            rd,
            verts,
            normals,
            frame,
            tint,
            mulv,
            apply: g(mem, 0x006e_d2e0),
            lit,
            norm,
        };
        let mut data = hdr.wrapping_add(0x10);
        while left > 0 {
            let count = |mem: &M| i32::from(mem.read::<i16>(hdr.wrapping_add(2)));
            let (prim, size) = match mem.read::<i8>(hdr.wrapping_add(1)) {
                0 => (
                    Some(Prim {
                        n: 3,
                        verts: 0xc,
                        cols: 0xd4,
                        lit: None,
                    }),
                    0x14,
                ),
                1 => (
                    Some(Prim {
                        n: 3,
                        verts: 0xc,
                        cols: 0x44,
                        lit: Some(Lit {
                            normals: 0x14,
                            verts: 0x144,
                            norms: 0x1c4,
                            matrix: 0x3c4,
                            res: 0x54,
                            p1_first: true,
                        }),
                    }),
                    0x1c,
                ),
                2 => (None, 0x20),
                3 | 6 => (None, 0x28),
                4 => (
                    Some(Prim {
                        n: 4,
                        verts: 0x10,
                        cols: 0x114,
                        lit: None,
                    }),
                    0x18,
                ),
                5 => (
                    Some(Prim {
                        n: 4,
                        verts: 0x10,
                        cols: 0xa4,
                        lit: Some(Lit {
                            normals: 0x18,
                            verts: 0x184,
                            norms: 0x204,
                            matrix: 0x384,
                            res: 0x64,
                            p1_first: false,
                        }),
                    }),
                    0x20,
                ),
                7 => (None, 0x30),
                _ => (None, 0),
            };
            match prim {
                Some(p) => {
                    let mut i = 0;
                    while i < count(mem) {
                        d.prim(mem, p, data, call);
                        data = data.wrapping_add(size);
                        i += 1;
                    }
                }
                None => {
                    let n = count(mem);
                    if n > 0 {
                        data = data.wrapping_add(n.cast_unsigned().wrapping_mul(size));
                    }
                }
            }
            left = left.wrapping_sub(count(mem));
            hdr = data;
            data = hdr.wrapping_add(0x10);
        }
        0
    }

    /// The emulated EE register `v0` (`g_Ee_Gpr +0x20`, `0x00c8f528`),
    /// which `op_md_spr_init_x64` copies through.
    pub const SPR_BOUNCE: Global<[u32; 4]> = Global::new(0x00c8_f528);

    /// `op_md_spr_init_x64(obj, init, work)` (`0x00a69940`): the 0x50-byte
    /// `init` into `work +0x300` (16 bytes at a time through
    /// [`SPR_BOUNCE`], which keeps the last), then:
    /// - `+0x338`, `+0x33c` when 0: `obj + obj[2]`, `obj + obj[3]`;
    /// - `+0x390` = `+0x304 + 16 obj.i16[+0x12]`, `+0x394` = `+0x304`,
    ///   `+0x39c` = `+0x390 + 16 obj.i16[+0x14]`;
    /// - the packet at `+0x100`: tag `0x70000005`, 0, GIF tag `0x8001`,
    ///   `0xeeeeeeee`, `+0x128` 6, `+0x130`/`+0x134` the bytes `+0x308`,
    ///   `+0x309` (or, when that `u16` is 0, `obj +0x16`, `+0x17`),
    ///   `+0x138` 0x42, `+0x140`.. the four dwords at `0x011a07f0` (bit 3
    ///   of `+0x300`) or `0x011a0740`, `+0x158` 8, and `+0x150`: with `obj`
    ///   bit 4 `init.u16[+0x54] << 24 | [+0x52] << 14 | [+0x50] << 4 |
    ///   10`, else 0 with `obj` bit 2, 5 without.
    ///
    /// Returns `work + 0x300`.
    #[allow(clippy::many_single_char_names)]
    pub fn op_md_spr_init_x64(mem: &mut impl Mem, obj: u32, init: u32, work: u32) -> u32 {
        let bounce = SPR_BOUNCE.ptr(mem).addr;
        for b in 0..5u32 {
            for i in 0..4u32 {
                let x: u32 = mem.read(init.wrapping_add(16 * b + 4 * i));
                mem.write(bounce + 4 * i, x);
            }
            for i in 0..4u32 {
                let x: u32 = mem.read(bounce + 4 * i);
                mem.write(work.wrapping_add(0x300 + 16 * b + 4 * i), x);
            }
        }
        let w = |o: u32| work.wrapping_add(o);
        let mut a: u32 = mem.read(w(0x338));
        if a == 0 {
            a = obj.wrapping_add(mem.read(obj.wrapping_add(8)));
        }
        let mut b: u32 = mem.read(w(0x33c));
        if b == 0 {
            b = obj.wrapping_add(mem.read(obj.wrapping_add(0xc)));
        }
        mem.write(w(0x33c), b);
        mem.write(w(0x338), a);
        let n12 = i32::from(mem.read::<i16>(obj.wrapping_add(0x12))).cast_unsigned();
        let base: u32 = mem.read(w(0x304));
        mem.write(w(0x390), (n12 << 4).wrapping_add(base));
        let base: u32 = mem.read(w(0x304));
        mem.write(w(0x394), base);
        let n14 = i32::from(mem.read::<i16>(obj.wrapping_add(0x14))).cast_unsigned();
        let at: u32 = mem.read(w(0x390));
        let end = (n14 << 4).wrapping_add(at);
        mem.write(w(0x100), 0x7000_0005u32);
        mem.write(w(0x39c), end);
        mem.write(w(0x108), 0u32);
        mem.write(w(0x110), 0x8001u32);
        mem.write(w(0x118), 0xeeee_eeeeu32);
        mem.write(w(0x128), 6u32);
        let (lo, hi) = if mem.read::<u16>(w(0x308)) == 0 {
            (obj.wrapping_add(0x16), obj.wrapping_add(0x17))
        } else {
            (w(0x308), w(0x309))
        };
        let x = u32::from(mem.read::<u8>(lo));
        mem.write(w(0x130), x);
        let x = u32::from(mem.read::<u8>(hi));
        mem.write(w(0x134), x);
        mem.write(w(0x138), 0x42u32);
        let src = if mem.read::<u8>(w(0x300)) & 8 != 0 {
            0x011a_07f0
        } else {
            0x011a_0740
        };
        let src = g(mem, src);
        for i in 0..4u32 {
            let x: u32 = mem.read(src + 4 * i);
            mem.write(w(0x140 + 4 * i), x);
        }
        let flags: u32 = mem.read(obj);
        let gs = if flags & 0x10 != 0 {
            let h = |o: u32| u32::from(mem.read::<u16>(init.wrapping_add(o)));
            (h(0x54) << 24) | (h(0x52) << 14) | (h(0x50) << 4) | 0xa
        } else if flags & 4 != 0 {
            0
        } else {
            5
        };
        mem.write(w(0x150), gs);
        mem.write(w(0x158), 8u32);
        w(0x300)
    }
}
