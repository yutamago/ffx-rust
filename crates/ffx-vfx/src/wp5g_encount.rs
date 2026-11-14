//! M5 finish, WP5 part 6 (wp5g): the broken-screen encounter's two draw
//! steps (`0x007f75e0` for `opus` op 10, `0x007f7a40` for op 11), which
//! `opu_encount`'s ops 10/11 hand to `op_func_final_set` and the OT draw
//! (`op_ot_draw`) calls with `(enc, cmd)`: `enc` is the block set
//! `opus_encount4_init` built (`crate::opus`). Neither is in the dump;
//! both were read from the machine code. The build without Unicorn
//! stopped at the first in every battle.
//!
//! Each step: the unit (`opus_no_get(a, b, cmd +4)`), a projection matrix
//! (`op_view_screen_matrix`, then every element set), the lights
//! (`opus_mdc_lights_get` with a 0x45c request in the frame), then per
//! live block (byte `+0` set; 0x70 bytes from `enc +0x110`) a scale by
//! `+0x10`, a rotation by `+0x50` with the angle
//! `(unit +0x3c - k) * s + u16 +2 * t`, the block's world matrix
//! (`view * enc +0x60 * block`), and each vertex (`+8`, 16 bytes;
//! `i16 rec +0x12` of them) through it, divided by `w`, scaled to the
//! screen and handed with its normal (`+0xc`) to
//! `graphicBrokenScreenUpdatePolygonData(i, mode, j, ...)`; the block's
//! matrix to `graphicBrokenScreenUpdatePolygonWMatrix(i, mode, m)`. Op 10
//! (mode 2) also hands the light colour to
//! `graphicBrokenScreenUpdateLightParameters`; op 11 (mode 1) asks for the
//! unit's fog distances (`ftol` of `+0x30`/`+0x34` times a factor) and
//! touches `op_oef2_a_get(a +0x60 [b +0x1e], 2, 1)`.
//!
//! The original keeps everything in an aligned frame and passes its
//! addresses to the callees; the port does the same in `frame`, laid out
//! as the original's `ebp`-relative locals (`frame` is that `ebp`; the
//! lowest local is `ebp - 0x22c`).

pub mod bridge {
    use ffx_bridge::x87::{add, div, ftol, load, mul, store, sub};
    use ffx_bridge::{Global, Mem};

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    /// The callees: `opus_no_get(a, b, no)`, `op_view_screen_matrix(m)`,
    /// `opus_mdc_lights_get(req, 0, a, b, data)`,
    /// `graphicBrokenScreenUpdateLightParameters(r, g, b, &x, &y, &z, &i)`,
    /// `op_oef2_a_get(oef, 2, 1)`, `op_xyz_scale(out, v, s)`,
    /// `op_unit_m_scale2(m, v)`, `op_rt_m(out, m, r, t)`,
    /// `op_mul_m(out, a, b)`, `op_apply_m_w1(out, m, v)`,
    /// `graphicBrokenScreenUpdatePolygonData(i, mode, j, &x, &y, &z, nx,
    /// ny, nz, 0, 0)` and `graphicBrokenScreenUpdatePolygonWMatrix(i, mode,
    /// m)`; all cdecl.
    pub const OPUS_NO_GET: u32 = 0x0080_1060;
    pub const VIEW_SCREEN_MATRIX: u32 = 0x0080_1ea0;
    pub const MDC_LIGHTS_GET: u32 = 0x0080_2bd0;
    pub const LIGHT_PARAMETERS: u32 = 0x0063_94b0;
    pub const OEF2_A_GET: u32 = 0x007e_3810;
    pub const XYZ_SCALE: u32 = 0x007e_a850;
    pub const UNIT_M_SCALE2: u32 = 0x007e_a730;
    pub const RT_M: u32 = 0x007e_9850;
    pub const MUL_M: u32 = 0x007e_83b0;
    pub const APPLY_M_W1: u32 = 0x007e_79a0;
    pub const POLYGON_DATA: u32 = 0x0063_9500;
    pub const POLYGON_W_MATRIX: u32 = 0x0063_9540;
    /// The light colour the op-10 step hands on (three `f32`s).
    pub const LIGHT_COLOUR: u32 = 0x012e_9634;

    /// The two steps.
    pub const STEP_LIT: u32 = 0x007f_75e0;
    pub const STEP_FOG: u32 = 0x007f_7a40;

    /// Frame bytes the steps use below their `ebp` (`sub esp, 0x238`).
    pub const FRAME: u32 = 0x238;

    /// The step `0x007f75e0` (op 10: `lit`, mode 2) or `0x007f7a40` (op
    /// 11, mode 1), cdecl `(enc, cmd)`; returns `enc` (the original's
    /// `eax`). `frame` is the original's `ebp` (locals from `frame -
    /// 0x22c`, zeroed by the caller).
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn encount4_draw<M: Mem>(
        mem: &mut M,
        enc: u32,
        cmd: u32,
        frame: u32,
        lit: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut go = |mem: &mut M, at: u32, args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let l = |o: u32| frame.wrapping_sub(o);
        let at = |p: u32, o: u32| p.wrapping_add(o);
        let word = |mem: &M, p: u32| mem.read::<u32>(p);
        let konst = |mem: &M, a: u32| mem.read::<f64>(g(mem, a));
        let single = |mem: &M, a: u32| mem.read::<u32>(g(mem, a));
        let (a, b) = (word(mem, at(enc, 0x10)), word(mem, at(enc, 0x14)));
        let no = u32::from(mem.read::<u16>(at(cmd, 4)));
        let unit = go(mem, OPUS_NO_GET, &[a, b, no]);
        // the projection: the view-screen matrix, then every element
        let view = l(0x58);
        go(mem, VIEW_SCREEN_MATRIX, &[view]);
        let first = store(load(single(mem, 0x00b4_3d18)));
        let one = 1.0f32.to_bits();
        let m: [u32; 16] = [first, 0, 0, 0, 0, one, 0, 0, 0, 0, one, one, 0, 0, 0, 0];
        mem.write(view, m);
        // the lights request (0x45c) in the frame
        let req = l(0x200);
        mem.write(req, 0x45cu32);
        mem.write(l(0x1fc), word(mem, at(a, 0x28)));
        mem.write(l(0x1f8), mem.read::<u16>(at(unit, 0xc2)));
        mem.write(l(0x1f4), 0u32);
        if lit {
            mem.write(l(0x1f0), 0u32);
            // junk read: [ebp-0x1ec] never written; 0 (brief 2026-10-24)
            mem.write(l(0x1ec), 0u32);
            mem.write(l(0x1e8), 0x8080_8080u32);
            mem.write(l(0x1e4), word(mem, at(unit, 0xc8)));
        } else {
            let k = konst(mem, 0x00b5_1558);
            let near = ftol(mem, mul(load(word(mem, at(unit, 0x30))), k));
            mem.write(l(0x1f0), near);
            let far = ftol(mem, mul(k, load(word(mem, at(unit, 0x34)))));
            mem.write(l(0x1ec), far);
            mem.write(l(0x1e8), word(mem, at(unit, 0xc8)));
            mem.write(l(0x1e4), 0u32);
        }
        for (o, p) in [
            (0x1e0, 0x108),
            (0x1dc, 0x148),
            (0x1d8, 0x188),
            (0x1d4, 0xc8),
        ] {
            mem.write(l(o), l(p));
        }
        mem.write(l(0x1bc), 0u32);
        let data = cmd
            .wrapping_add(i32::from(mem.read::<i16>(at(cmd, 6))).cast_unsigned())
            .wrapping_sub(2);
        go(mem, MDC_LIGHTS_GET, &[req, 0, a, b, data]);
        if lit {
            // the light: its intensity (byte +0x4d) and colour
            let i = f64::from(mem.read::<u8>(at(req, 0x4d)));
            mem.write(l(0x228), store(mul(i, konst(mem, 0x00b1_70b8))));
            let colour = g(mem, LIGHT_COLOUR);
            mem.write(colour, store(load(word(mem, l(0x158)))));
            mem.write(at(colour, 4), store(load(word(mem, l(0x154)))));
            let blue = mul(load(word(mem, l(0x150))), konst(mem, 0x00b5_1648));
            mem.write(at(colour, 8), store(blue));
            for (to, from) in [(0x220, 0x148), (0x22c, 0x144), (0x224, 0x140)] {
                mem.write(l(to), store(load(word(mem, l(from)))));
            }
            go(
                mem,
                LIGHT_PARAMETERS,
                &[
                    colour,
                    at(colour, 4),
                    at(colour, 8),
                    l(0x220),
                    l(0x22c),
                    l(0x224),
                    l(0x228),
                ],
            );
        } else {
            let k = u32::from(mem.read::<u8>(at(b, 0x1e)));
            let oef = word(mem, at(a, 0x60 + 4 * k));
            go(mem, OEF2_A_GET, &[oef, 2, 1]);
        }
        let count = |mem: &M| u32::from(mem.read::<u16>(at(enc, 6)));
        if count(mem) == 0 {
            return enc;
        }
        let mode = if lit { 2 } else { 1 };
        let (xform, scale, world) = (l(0x108), l(0xc8), l(0x58));
        let (s, r, sv) = (l(0x78), l(0x88), l(0x68));
        let (x, y, z, w) = (l(0x14), l(0x10), l(0xc), l(0x8));
        let mut i = 0u32;
        loop {
            let blk = at(enc, 0x110 + 0x70 * i);
            if mem.read::<u8>(blk) != 0 {
                let c = store(load(single(mem, 0x00b5_8394)));
                mem.write(at(s, 0xc), 0u32);
                go(mem, XYZ_SCALE, &[s, at(blk, 0x10), c]);
                let tilt = mul(
                    sub(load(word(mem, at(unit, 0x3c))), konst(mem, 0x00b5_15c8)),
                    konst(mem, 0x00b5_1648),
                );
                let turn = f64::from(mem.read::<u16>(at(blk, 2)));
                let angle = store(add(tilt, mul(turn, konst(mem, 0x00b4_3d80))));
                mem.write(at(r, 0xc), 0u32);
                go(mem, XYZ_SCALE, &[r, at(blk, 0x50), angle]);
                let d = store(load(single(mem, 0x00b5_1b9c)));
                mem.write(sv, [c, d, c]);
                go(mem, UNIT_M_SCALE2, &[scale, sv]);
                go(mem, RT_M, &[scale, scale, r, s]);
                go(mem, MUL_M, &[xform, world, at(enc, 0x60)]);
                go(mem, MUL_M, &[xform, xform, scale]);
                mem.write(l(0x1c8), word(mem, at(blk, 8)));
                mem.write(l(0x1c4), word(mem, at(blk, 0xc)));
                let mut j = 0u32;
                while j.cast_signed() < i32::from(mem.read::<i16>(at(word(mem, at(blk, 4)), 0x12)))
                {
                    let v = at(word(mem, at(blk, 8)), 16 * j);
                    for k in 0..3 {
                        let e = store(load(word(mem, at(v, 4 * k))));
                        mem.write(at(x, 4 * k), e);
                    }
                    mem.write(w, one);
                    go(mem, APPLY_M_W1, &[x, xform, x]);
                    let ww = load(word(mem, w));
                    let n = at(word(mem, at(blk, 0xc)), 16 * j);
                    let px = store(div(load(word(mem, x)), ww));
                    mem.write(x, px);
                    let py = store(div(load(word(mem, y)), ww));
                    mem.write(y, py);
                    let pz = add(
                        mul(load(word(mem, z)), konst(mem, 0x00b9_2220)),
                        konst(mem, 0x00b5_83a8),
                    );
                    mem.write(z, store(pz));
                    let sx = mul(load(word(mem, x)), konst(mem, 0x00b9_2270));
                    mem.write(x, store(sx));
                    let sy = mul(load(word(mem, y)), konst(mem, 0x00b2_c5d0));
                    mem.write(y, store(sy));
                    let sz = mul(
                        sub(load(word(mem, z)), konst(mem, 0x00b3_13a0)),
                        konst(mem, 0x00b1_70b8),
                    );
                    mem.write(z, store(sz));
                    go(
                        mem,
                        POLYGON_DATA,
                        &[i, mode, j, x, y, z, n, at(n, 4), at(n, 8), 0, 0],
                    );
                    j += 1;
                }
                go(mem, POLYGON_W_MATRIX, &[i, mode, scale]);
            }
            i += 1;
            if i >= count(mem) {
                break;
            }
        }
        enc
    }
}
