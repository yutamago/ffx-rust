//! M5 stage 1, round 9 (`tail9`, list `vfx`): the opus handler at
//! `0x00809f90` (entry 0x90 of the opus table `0x00c48ec8`, the entry at
//! `0x00c49108`; 0x4c5 bytes, `0x00809f90..0x0080a455`), the map's colour
//! commands: by the command nibble (`host + 0x21a`, bits 12-15) the
//! colour scale, the back colour, the field vectors, the draw mask, the
//! YIQ colour pair and the CLUT green pass.
//!
//! Callees stay calls to the originals (cdecl). x87 code is written as
//! the formulas at 53 bits (`ffx_bridge::x87`), each value rounded to
//! `f32` where the original stores it.

/// The originals over game memory.
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The callees (cdecl): `yiRenderDisable(bits)`, `yiRenderEnable(bits)`,
    /// `yiSetColorScale(rgba*)`, `opus_map_back_color_restore_0080a4b0`,
    /// `opus_colour_scale2_0080a470(r, g, b, a)`, `opus_map_rgba_free`,
    /// `opus_map_back_color_set_0080a4d0(rgba*)`, `yiSetFieldVec4i(a, b,
    /// c, d)`, `yiGetDrawMask()`, `yiSetDrawMask(m)`,
    /// `yiSetRenderParam(v)`, `yiSetFieldVec6(a..f)`,
    /// `StoreComputedToDeref_00643b10(v)`, `graphicSetColourPair_00643b30(r,
    /// g, b, r2, g2, b2, second)`, `graphicSetMaterialColourScale2(r, g, b,
    /// a)`, `yiClutGreenRender_00908400(a*, b*)`,
    /// `YiRenderStatus_clearBit8_00908440()`,
    /// `graphicUpdateMapYIQ_thunk_006449c0()`.
    pub const RENDER_DISABLE: u32 = 0x0090_9e10;
    pub const RENDER_ENABLE: u32 = 0x0090_9e20;
    pub const SET_COLOR_SCALE: u32 = 0x0090_a270;
    pub const BACK_COLOR_RESTORE: u32 = 0x0080_a4b0;
    pub const COLOUR_SCALE2: u32 = 0x0080_a470;
    pub const RGBA_FREE: u32 = 0x0080_ac00;
    pub const BACK_COLOR_SET: u32 = 0x0080_a4d0;
    pub const FIELD_VEC4I: u32 = 0x0090_a1d0;
    pub const GET_DRAW_MASK: u32 = 0x0090_8960;
    pub const SET_DRAW_MASK: u32 = 0x0090_a2b0;
    pub const SET_RENDER_PARAM: u32 = 0x0090_8580;
    pub const FIELD_VEC6: u32 = 0x0090_a230;
    pub const STORE_COMPUTED: u32 = 0x0064_3b10;
    pub const COLOUR_PAIR: u32 = 0x0064_3b30;
    pub const MATERIAL_SCALE2: u32 = 0x0064_3670;
    pub const CLUT_GREEN: u32 = 0x0090_8400;
    pub const CLEAR_BIT8: u32 = 0x0090_8440;
    pub const UPDATE_YIQ: u32 = 0x0064_49c0;
    /// `g_Vu0_VfMove` (4 words) and `g_Ee_Gpr` (its first 4 words).
    pub const VU0_VF_MOVE: u32 = 0x00c8_f980;
    pub const EE_GPR: u32 = 0x00c8_f508;
    /// The doubles the handler scales by: the field colour (`0x00b921e0`),
    /// the YIQ colour (`0x00b50198`) and the colour scale (`0x00b170b8`).
    pub const K_FIELD: u32 = 0x00b9_21e0;
    pub const K_YIQ: u32 = 0x00b5_0198;
    pub const K_SCALE: u32 = 0x00b1_70b8;

    /// The two vec4s the CLUT green pass gets, as `f32` words at the
    /// original's 16-aligned `ebp - 0x30` and `ebp - 0x20`.
    pub const VECTORS: u32 = 0x20;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn k(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `x * k` rounded to `f32` (the original's `fstp dword`).
    fn scaled(x: f64, k: f64) -> u32 {
        x87::store(x87::mul(x, k))
    }

    /// The colour scale of `rgba` (`host + 0..+0xc`, `f32`) from the
    /// command's colour (`cmd_data + 0x40..+0x4c`) times the scale
    /// constant, each clamped to 0..1 (NaN kept), set (`yiRenderEnable(4)`,
    /// `yiSetColorScale`, `graphicSetMaterialColourScale2`).
    fn colour_scale(h: &mut impl Host, host: u32, data: u32) {
        let ks = k(&*h, K_SCALE);
        for i in 0..4_u32 {
            let v = f(&*h, data.wrapping_add(0x40 + 4 * i));
            h.write(host.wrapping_add(4 * i), scaled(v, ks));
        }
        for i in 0..4_u32 {
            let at = host.wrapping_add(4 * i);
            // fldz; fcom: stored when 0 > v
            if 0.0 > f(&*h, at) {
                h.write(at, 0_u32);
            }
        }
        for i in 0..4_u32 {
            let at = host.wrapping_add(4 * i);
            // fld1; fcom: stored when 1 < v
            if 1.0 < f(&*h, at) {
                h.write(at, x87::store(1.0));
            }
        }
        cdecl(h, RENDER_ENABLE, &[4]);
        cdecl(h, SET_COLOR_SCALE, &[host]);
        let c: [u32; 4] = [0, 4, 8, 0xc].map(|o| h.read(host.wrapping_add(o)));
        cdecl(h, MATERIAL_SCALE2, &c);
    }

    /// `opus_colour_scale2_0080a470` with `host + 0..+0xc`.
    fn colour_scale2(h: &mut impl Host, host: u32) {
        let c: [u32; 4] = [0, 4, 8, 0xc].map(|o| h.read(host.wrapping_add(o)));
        cdecl(h, COLOUR_SCALE2, &c);
    }

    /// The opus handler `0x00809f90(host, cmd)` (cdecl) -> the next
    /// command: `cmd + 2`, or `cmd` for an unknown nibble. The command's
    /// data is `[host + 0x220]`; the host's first 16 bytes are the colour
    /// scale. By the nibble of `[host + 0x21a]` (u16):
    /// - 0: the data's colour (`+0x40..+0x4c`) into `g_Vu0_VfMove`, then
    ///   the colour scale (below);
    /// - 0x1000: `opus_map_rgba_free`;
    /// - 0x2000: rendering bit 4 off, 1 on, the scale 1.0, the back colour
    ///   restored, `opus_colour_scale2` of the scale;
    /// - 0x3000: bit 4 on, 1 off, the scale from `g_Ee_Gpr`, the back colour
    ///   set from it, `opus_colour_scale2`;
    /// - 0x4000: bit 0x100 on, the field vector of the data's
    ///   `+0x40/+0x44/+0x48/+0x3c` truncated (`ftol2_sse`);
    /// - 0x5000: bit 0x100 off; 0x6000: the back colour from the data;
    /// - 0x7000/0x8000: draw mask bit 4 on/off;
    /// - 0x9000: render parameter 8, the field vec6 (the data's colour
    ///   times the YIQ constant, `+0x30..+0x38` times the field one),
    ///   `StoreComputed(1)`, the colour pair (both products, the second
    ///   unrounded between them), second 0, the YIQ update;
    /// - 0xa000: bit 8 cleared, `StoreComputed(0)`, the YIQ update;
    /// - 0xb000: the two vectors (the data's colour times the YIQ constant,
    ///   `+0x30..+0x38` times the field one, w 1.0) at `vectors` (the
    ///   original's `ebp - 0x30`, `- 0x20`) to the CLUT green pass, bit 8
    ///   cleared, the colour pair from them (the second scaled again),
    ///   second 1, the YIQ update;
    /// - any other: the colour scale.
    ///
    /// The colour scale: the data's colour times the scale constant, each
    /// clamped to 0..1 (NaN kept), `yiRenderEnable(4)`, `yiSetColorScale`,
    /// `graphicSetMaterialColourScale2`.
    #[allow(clippy::too_many_lines)]
    pub fn opus_colour(h: &mut impl Host, vectors: u32, host: u32, cmd: u32) -> u32 {
        let nibble = u32::from(h.read::<u16>(host.wrapping_add(0x21a))) & 0xf000;
        let data: u32 = h.read(host.wrapping_add(0x220));
        let d = |o: u32| data.wrapping_add(o);
        let next = cmd.wrapping_add(2);
        match nibble {
            0 => {
                let to = h.rebase(VU0_VF_MOVE);
                for i in 0..4_u32 {
                    let v: u32 = h.read(d(0x40 + 4 * i));
                    h.write(to.wrapping_add(4 * i), v);
                }
                colour_scale(h, host, data);
                next
            }
            0x1000 => {
                cdecl(h, RGBA_FREE, &[]);
                next
            }
            0x2000 => {
                cdecl(h, RENDER_DISABLE, &[4]);
                cdecl(h, RENDER_ENABLE, &[1]);
                for i in 0..4_u32 {
                    h.write(host.wrapping_add(4 * i), x87::store(1.0));
                }
                cdecl(h, SET_COLOR_SCALE, &[host]);
                cdecl(h, BACK_COLOR_RESTORE, &[d(0x40)]);
                colour_scale2(h, host);
                next
            }
            0x3000 => {
                cdecl(h, RENDER_ENABLE, &[4]);
                cdecl(h, RENDER_DISABLE, &[1]);
                let gpr = h.rebase(EE_GPR);
                for i in 0..4_u32 {
                    let v: u32 = h.read(gpr.wrapping_add(4 * i));
                    h.write(host.wrapping_add(4 * i), v);
                }
                cdecl(h, SET_COLOR_SCALE, &[host]);
                cdecl(h, BACK_COLOR_SET, &[host]);
                colour_scale2(h, host);
                next
            }
            0x4000 => {
                cdecl(h, RENDER_ENABLE, &[0x100]);
                let a = x87::ftol(&*h, f(&*h, d(0x3c)));
                let b = x87::ftol(&*h, f(&*h, d(0x48)));
                let c = x87::ftol(&*h, f(&*h, d(0x44)));
                let e = x87::ftol(&*h, f(&*h, d(0x40)));
                cdecl(h, FIELD_VEC4I, &[e, c, b, a]);
                next
            }
            0x5000 => {
                cdecl(h, RENDER_DISABLE, &[0x100]);
                next
            }
            0x6000 => {
                cdecl(h, BACK_COLOR_SET, &[d(0x40)]);
                next
            }
            0x7000 | 0x8000 => {
                let m = cdecl(h, GET_DRAW_MASK, &[]);
                let m = if nibble == 0x7000 { m | 4 } else { m & !4 };
                cdecl(h, SET_DRAW_MASK, &[m]);
                next
            }
            0x9000 => {
                cdecl(h, SET_RENDER_PARAM, &[8]);
                let (kf, ky) = (k(&*h, K_FIELD), k(&*h, K_YIQ));
                let six = [
                    scaled(f(&*h, d(0x40)), ky),
                    scaled(f(&*h, d(0x44)), ky),
                    scaled(f(&*h, d(0x48)), ky),
                    scaled(f(&*h, d(0x30)), kf),
                    scaled(f(&*h, d(0x34)), kf),
                    scaled(f(&*h, d(0x38)), kf),
                ];
                cdecl(h, FIELD_VEC6, &six);
                cdecl(h, STORE_COMPUTED, &[1]);
                let both = |x: f64| x87::store(x87::mul(x87::mul(x, kf), ky));
                let pair = [
                    scaled(f(&*h, d(0x40)), ky),
                    scaled(f(&*h, d(0x44)), ky),
                    scaled(f(&*h, d(0x48)), ky),
                    both(f(&*h, d(0x30))),
                    both(f(&*h, d(0x34))),
                    both(f(&*h, d(0x38))),
                    0,
                ];
                cdecl(h, COLOUR_PAIR, &pair);
                cdecl(h, UPDATE_YIQ, &[]);
                next
            }
            0xa000 => {
                cdecl(h, CLEAR_BIT8, &[]);
                cdecl(h, STORE_COMPUTED, &[0]);
                cdecl(h, UPDATE_YIQ, &[]);
                next
            }
            0xb000 => {
                let (kf, ky) = (k(&*h, K_FIELD), k(&*h, K_YIQ));
                let (v0, v1) = (vectors, vectors.wrapping_add(0x10));
                let one = x87::store(1.0);
                let first = [
                    scaled(f(&*h, d(0x40)), ky),
                    scaled(f(&*h, d(0x44)), ky),
                    scaled(f(&*h, d(0x48)), ky),
                    one,
                ];
                let second = [
                    scaled(f(&*h, d(0x30)), kf),
                    scaled(f(&*h, d(0x34)), kf),
                    scaled(f(&*h, d(0x38)), kf),
                    one,
                ];
                for i in 0..4_u32 {
                    h.write(v0.wrapping_add(4 * i), first[i as usize]);
                    h.write(v1.wrapping_add(4 * i), second[i as usize]);
                }
                cdecl(h, CLUT_GREEN, &[v0, v1]);
                cdecl(h, CLEAR_BIT8, &[]);
                let ky = k(&*h, K_YIQ);
                let pair = [
                    h.read(v0),
                    h.read(v0.wrapping_add(4)),
                    h.read(v0.wrapping_add(8)),
                    scaled(f(&*h, v1), ky),
                    scaled(f(&*h, v1.wrapping_add(4)), ky),
                    scaled(f(&*h, v1.wrapping_add(8)), ky),
                    1,
                ];
                cdecl(h, COLOUR_PAIR, &pair);
                cdecl(h, UPDATE_YIQ, &[]);
                next
            }
            _ => {
                colour_scale(h, host, data);
                cmd
            }
        }
    }
}
