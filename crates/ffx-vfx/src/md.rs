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
}
