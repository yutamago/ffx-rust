//! `opus_0283_draw` (`0x00a76770`): the `opus_0283` effect's draw, a
//! 32 x 16 quad grid whose 33 x 17 vertices bulge out to a sphere of the
//! part's radius (`docs/formats/vfx.md`, "High range").

#![allow(clippy::many_single_char_names)]

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{self, Reg};

    /// Calls the game function at `sym` (rebased) through `call`.
    fn go<M: Mem>(
        mem: &mut M,
        sym: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// `g_GraphicSkipFrame`: nothing is drawn while it is set.
    const SKIP_FRAME: u32 = 0x012f_b7d0;

    const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    const OP_UNIT_M_SCALE: u32 = 0x007e_a700;
    const OP_TRANS_M: u32 = 0x007e_a550;
    const OPUS_PARENT_GET: u32 = 0x0080_1f00;
    const OP_MUL_M: u32 = 0x007e_83b0;
    const OP_M_COPY: u32 = 0x007e_8010;
    const GET_RENDER_DATA_BY_INSTANCE: u32 = 0x0064_53d0;
    const UPDATE_VFX_RENDER_DATA: u32 = 0x0071_8610;
    const GRAPHIC_VFX_UPDATE: u32 = 0x0064_58a0;
    const INIT_VFX_RENDER_DATA: u32 = 0x0071_1180;

    /// The `f64` constants the table reads from `.rdata`.
    const K_13: u32 = 0x00b4_b470;
    const K_104: u32 = 0x00b7_c578;
    const K_8: u32 = 0x00b9_22a8;
    const K_128: u32 = 0x00b4_c170;
    const K_MINUS_128: u32 = 0x00b7_c580;
    /// 1.3329999446868896 (the `f32` 1.333 as a double).
    const K_ASPECT: u32 = 0x00b7_c570;
    /// 1/256.
    const K_256TH: u32 = 0x00b5_0198;
    /// 2^-21.
    const K_SCALE: u32 = 0x00b5_8398;

    /// The quads: 16 rows of 32, from `b + 0x30`, 0x28 bytes each.
    const ROWS: u32 = 16;
    const COLS: u32 = 32;
    /// Where the vertex table starts (`b + 0x5040`) and the end the request
    /// gets (`b + 0xd040`).
    const TABLE: u32 = 0x5040;
    const END: u32 = 0xd040;

    #[allow(clippy::cast_possible_truncation)] // the original's 16-bit stores
    fn half(x: u32) -> u16 {
        x as u16
    }

    /// The packet header at `b` and the quads after it: `[b]` 0, `+4` 0x20,
    /// `+8` 0x5040, `+0xc` 0, `+0x10` 0x08000200, `+0x14` 0x880000, `u16
    /// +0x20` 0x600, `u16 +0x22` the low half of `+0x10` (read back), `+0x28`
    /// 0x1a00. Quad `(row, col)`, `u = 0x80 col`, `v = 0xd0 row`: colours
    /// `+0..+0xc` 0x80808080; the vertex numbers `+0x10` `33 row + col` and
    /// one on, `+0x14` 33 below and one on; the texture coordinates `(u, v)`
    /// at `+0x18`, `(u + 0x80, v)`, `(u, v + 0xd0)`, `(u + 0x80, v + 0xd0)`
    /// (`u16` each). The byte after the last quad's `+0x28` is 0xff (the
    /// end tag's type).
    fn quads(mem: &mut impl Mem, b: u32) {
        let at = |o: u32| b.wrapping_add(o);
        mem.write(at(0x10), 0x0800_0200u32);
        mem.write(b, 0u32);
        mem.write(at(4), 0x20u32);
        mem.write(at(8), TABLE);
        mem.write(at(0xc), 0u32);
        mem.write(at(0x14), 0x0088_0000u32);
        mem.write(at(0x20), 0x600u16);
        let x: u16 = mem.read(at(0x10));
        mem.write(at(0x22), x);
        mem.write(at(0x28), 0x1a00u32);
        let mut e = at(0x30);
        for row in 0..ROWS {
            let v = row * 0xd0;
            for col in 0..COLS {
                let u = col * 0x80;
                let q = |o: u32| e.wrapping_add(o);
                mem.write(q(0x18), half(u));
                let n = col + row * 33;
                mem.write(q(0x10), half(n));
                mem.write(q(0x12), half(n + 1));
                mem.write(q(0x14), half(n + 33));
                mem.write(q(0x16), half(n + 34));
                mem.write(q(0x1a), half(v));
                mem.write(q(0x1e), half(v));
                mem.write(q(0x1c), half(u + 0x80));
                mem.write(q(0x20), half(u));
                mem.write(q(0x22), half(v + 0xd0));
                mem.write(q(0x24), half(u + 0x80));
                mem.write(q(0x26), half(v + 0xd0));
                for k in 0..4u32 {
                    mem.write(q(4 * k), 0x8080_8080u32);
                }
                e = e.wrapping_add(0x28);
            }
        }
        mem.write(e.wrapping_add(1), 0xffu8);
    }

    fn constant(mem: &impl Mem, sym: u32) -> Reg {
        Reg::constant(mem.read(mem.rebase(sym)))
    }

    /// The 33 x 17 vertices (three `i16` each, from `b + 0x5040`) for the
    /// radius `r = f32 rec +0x30` and the bulge `k = f32(f32 rec +0x34 /
    /// 256)`: with `x = 8 j - 128` and `y = 13 i - 104` (`f32`), `d =
    /// f32((y^2 + x^2) + r^2)`, `s = f32(sqrt d)` and `q = f32(f32(s -
    /// f32(f32(s - r) x k)) / s)` (each step stored as `f32`), the vertex is
    /// `ftol(q x x x 128 x 1.333)`, `ftol(q x y x 128 x 1.333)`, `ftol(q x r x
    /// -128)` (low halves). The 1.333 is the `f64` of the `f32` (`0x00b7c570`).
    fn vertices(mem: &mut impl Mem, b: u32, rec: u32) {
        let radius = f32::from_bits(mem.read(rec.wrapping_add(0x30)));
        // through a local, as `fld`/`fstp`
        let r = x87::round_trip(radius);
        let bulge = f32::from_bits(mem.read(rec.wrapping_add(0x34)));
        let k = (Reg::load(bulge) * constant(mem, K_256TH)).store();
        let r2 = Reg::load(r) * Reg::load(r);
        let (c13, c104) = (constant(mem, K_13), constant(mem, K_104));
        let (c8, c128) = (constant(mem, K_8), constant(mem, K_128));
        let (aspect, m128) = (constant(mem, K_ASPECT), constant(mem, K_MINUS_128));
        let mut out = b.wrapping_add(TABLE);
        for i in 0..=16u16 {
            let y = (Reg::load(f32::from(i)) * c13 - c104).store();
            let y2 = Reg::load(y) * Reg::load(y);
            for j in 0..=32u16 {
                let x = (Reg::load(f32::from(j)) * c8 - c128).store();
                let x2 = Reg::load(x) * Reg::load(x);
                let d = (y2 + x2 + r2).store();
                let s = Reg::load(d).sqrt().store();
                // the radius reread from the part here
                let radius = f32::from_bits(mem.read(rec.wrapping_add(0x30)));
                let t = (Reg::load(s) - Reg::load(radius)).store();
                let t = Reg::load(t).mul_m32(k).store();
                let t = Reg::load(s).sub_m32(t).store();
                let q = Reg::load(s).divr_m32(t).store();
                let vx = Reg::load(q).mul_m32(x) * c128 * aspect;
                let vy = Reg::load(q).mul_m32(y) * c128 * aspect;
                let vz = Reg::load(q).mul_m32(r) * m128;
                for (o, v) in [(0u32, vx), (2, vy), (4, vz)] {
                    let n = x87::ftol(mem, v.value());
                    mem.write(out.wrapping_add(o), half(n));
                }
                out = out.wrapping_add(6);
            }
        }
    }

    /// `opus_0283_draw(obj, rec)` (`0x00a76770`; from `opp_part_a76720`
    /// kind 0x1000). Nothing while `g_GraphicSkipFrame` is set. Else, with
    /// `id` = `MagicFile_GetMagicId()` and `b = [obj +0x328]`: the quads
    /// ([`quads`]) and the vertex table ([`vertices`]); the request at `obj
    /// +0x100`: `+0` 0x90, `+4` `b + 0xd040`, `u16 +8` `u16 rec +0xc2`, `+0xc`,
    /// `+0x10`, `+0x14`, `+0x1c`, `+0x38`, `+0x3c`, `+0x44` 0, `+0x18` `[rec
    /// +0xc8]`, `+0x20` `obj + 0xc0`, `+0x40` `[rec +0xc4]`. The matrix `obj
    /// +0x80`: `op_unit_m_scale(m, f32(f32 rec +0x3c x 2^-21))`,
    /// `op_trans_m(m, m, rec + 0x90)`, `op_mul_m(m, opus_parent_get(obj,
    /// rec), m)`, copied to `obj +0xc0` (`op_m_copy`). Then `+0x34` = `[obj
    /// +0x354]`, `+0x38` = `b + [b +8]`. With render data for `(0, id)`
    /// (`graphicVFXGetRenderDataByInstance`): `updateVFXRenderData(0, id,
    /// part, obj + 0x100)` and `graphicVFXUpdate(0, id)`; else
    /// `InitVFXRenderData(0, id, part, obj + 0x100, 8)`. `part` is a local
    /// (`frame - 0x54`, or `frame - 0x74` for the init) of which only `+0x1c`
    /// (= `b`) is set, the only word those read. Callees through `call`.
    pub fn opus_0283_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        rec: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u32>(mem.rebase(SKIP_FRAME)) != 0 {
            return;
        }
        let id = go(mem, MAGIC_FILE_GET_MAGIC_ID, &[], call);
        let b: u32 = mem.read(obj.wrapping_add(0x328));
        quads(mem, b);
        vertices(mem, b, rec);
        let req = obj.wrapping_add(0x100);
        let at = |o: u32| req.wrapping_add(o);
        let x: u32 = mem.read(rec.wrapping_add(0xc8));
        mem.write(at(0x18), x);
        mem.write(at(0x10), 0u32);
        mem.write(at(0x14), 0u32);
        let x: u16 = mem.read(rec.wrapping_add(0xc2));
        mem.write(at(8), x);
        mem.write(req, 0x90u32);
        mem.write(at(0xc), 0u32);
        mem.write(at(0x1c), 0u32);
        let x: u32 = mem.read(rec.wrapping_add(0xc4));
        mem.write(at(0x40), x);
        mem.write(at(0x38), 0u32);
        mem.write(at(0x3c), 0u32);
        mem.write(at(0x20), obj.wrapping_add(0xc0));
        mem.write(at(0x44), 0u32);
        mem.write(at(4), b.wrapping_add(END));
        let size = f32::from_bits(mem.read(rec.wrapping_add(0x3c)));
        let scale = (Reg::load(size) * constant(mem, K_SCALE)).store();
        let m = obj.wrapping_add(0x80);
        go(mem, OP_UNIT_M_SCALE, &[m, scale.to_bits()], call);
        go(mem, OP_TRANS_M, &[m, m, rec.wrapping_add(0x90)], call);
        let parent = go(mem, OPUS_PARENT_GET, &[obj, rec], call);
        go(mem, OP_MUL_M, &[m, parent, m], call);
        go(mem, OP_M_COPY, &[obj.wrapping_add(0xc0), m], call);
        let x: u32 = mem.read(obj.wrapping_add(0x354));
        mem.write(at(0x34), x);
        let off: u32 = mem.read(b.wrapping_add(8));
        mem.write(at(0x38), off.wrapping_add(b));
        if go(mem, GET_RENDER_DATA_BY_INSTANCE, &[0, id], call) != 0 {
            let part = frame.wrapping_sub(0x54);
            mem.write(part.wrapping_add(0x1c), b);
            go(mem, UPDATE_VFX_RENDER_DATA, &[0, id, part, req], call);
            go(mem, GRAPHIC_VFX_UPDATE, &[0, id], call);
        } else {
            let part = frame.wrapping_sub(0x74);
            mem.write(part.wrapping_add(0x1c), b);
            go(mem, INIT_VFX_RENDER_DATA, &[0, id, part, req, 8], call);
        }
    }
}
