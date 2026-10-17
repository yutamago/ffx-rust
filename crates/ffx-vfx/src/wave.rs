//! The opus wave models: `opus_wave_omd_make` builds a 17 x 25 vertex
//! ring mesh (a disc of 16 sectors and 24 rings) as an OMD model, with
//! lit vertex colours and screen-projected texture coordinates.

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87::{ADD, MUL, Op, Reg, SUB};

    const DIV: Op = |a, b| a / b;

    fn g(mem: &impl Mem, a: u32) -> u32 {
        Global::<u32>::new(a).ptr(mem).addr
    }

    fn f(mem: &impl Mem, a: u32) -> f32 {
        mem.read(a)
    }

    fn c64(mem: &impl Mem, a: u32) -> f64 {
        mem.read(g(mem, a))
    }

    /// `g_pppSinTable[i & 0xfff]`.
    fn sin(mem: &impl Mem, i: u32) -> f32 {
        f(mem, g(mem, 0x00c4_4bf0) + 4 * (i & 0xfff))
    }

    /// `fld m32; fstp m32`: a copy that quiets a signalling NaN.
    fn q(x: f32) -> f32 {
        crate::x87::load_f32(x)
    }

    /// `fistp m32` with truncation: toward zero, the integer indefinite
    /// outside `i32`.
    fn fistp_trunc(r: Reg) -> u32 {
        ffx_bridge::x87::cvttsd2si(r.value())
    }

    /// The vertex count (17 rows of 25), the quad count (16 x 24).
    const VERTS: u32 = 0x1a9;

    /// `opus_wave_omd_make(obj, wave, uv, omd, view, light)` (`0x007f94e0`):
    /// the wave's OMD at `omd` (`+0x30` 384 quad records of 0x28 bytes and
    /// an end tag, vertices at `+0x4840`, normals at `+0x62d0`, 425 each).
    ///
    /// - Sector `j` (angle `0x100 j`), ring `i`: the vertex at radius `r`
    ///   (from 0, step `obj +0x30`, the step growing by `obj +0x34`) on
    ///   the angle's cosine and sine, normal (cos, sin); height per ring
    ///   `A sin(ang)` (`wave +0x40`, `+0x30`, growing by `+0x44`/`+0x48`
    ///   and `+0x34`/`+0x38`), `w` 1.
    /// - Each ring's normals tilt by the running mean of the ring's
    ///   slope angle (`op_deg_get` on consecutive vertices): x, y times
    ///   its sine, z its cosine.
    /// - Colours: the light `[light +0x28]` (position, intensity) over the
    ///   squared distance to the vertex through `[light +0x2c]`, dotted
    ///   with the normal (at least 0), times 1.7 `light +0x4d` / 128 and
    ///   the colour `[light +0x24]`, plus `light +0x4c` / 128; as bytes
    ///   times 128 (alpha also fading by ring).
    /// - Texture coordinates: the disc's extent projected (`op_rtp_w1`
    ///   through `view +0xa0 * view +0x60 * view +0x20`) at the centre and
    ///   at the outer radius both ways, mapped to 4096ths.
    ///
    /// Callees through `call` and (returning `ST0`) `fcall`; the locals
    /// lie below `frame` (from `frame - 0x84`). Returns the end of the
    /// normals.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )]
    pub fn opus_wave_omd_make<M: Mem>(
        mem: &mut M,
        obj: u32,
        wave: u32,
        uv: u32,
        omd: u32,
        view: u32,
        light: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let h = |o: u32| omd.wrapping_add(o);
        mem.write(h(0x14), 0x1a9u16);
        mem.write(h(0x10), 0x01a9_0180u32);
        mem.write(h(8), 0x4840u32);
        mem.write(h(0xc), 0x62d0u32);
        mem.write(h(0), 0x10u32);
        mem.write(h(4), 0x20u32);
        mem.write(h(0x21), 6u8);
        let x: u16 = mem.read(h(0x10));
        mem.write(h(0x22), x);
        let x: u32 = mem.read(uv);
        mem.write(h(0x28), x);
        let x: u32 = mem.read(uv.wrapping_add(4));
        mem.write(h(0x2c), x);
        let vbase = mem.read::<u32>(h(8)).wrapping_add(omd);
        let nbase = mem.read::<u32>(h(0xc)).wrapping_add(omd);
        // the disc: 17 sectors of 25 vertices
        let (mut vp, mut np) = (vbase, nbase);
        for j in 0..17u32 {
            let a = (0x100 * j) & 0xfff;
            let ca = (a + 0x400) & 0xfff;
            let mut r = 0f32;
            let mut step = q(f(mem, obj.wrapping_add(0x30)));
            for _ in 0..25 {
                let c = q(sin(mem, ca));
                mem.write(np, c);
                let s = q(sin(mem, a));
                mem.write(np.wrapping_add(4), s);
                let rs = Reg::load_f32(r).mem(MUL, sin(mem, a)).store();
                let x = Reg::load_f32(sin(mem, ca))
                    .reg(MUL, Reg::load_f32(r))
                    .store();
                mem.write(vp, x);
                mem.write(vp.wrapping_add(4), rs);
                step = Reg::load_f32(step)
                    .mem(ADD, f(mem, obj.wrapping_add(0x34)))
                    .store();
                r = Reg::load_f32(r).reg(ADD, Reg::load_f32(step)).store();
                vp = vp.wrapping_add(0x10);
                np = np.wrapping_add(0x10);
            }
        }
        // the heights: per ring, A sin(ang), the same in every sector
        let mut amp = q(f(mem, wave.wrapping_add(0x40)));
        let mut z = q(f(mem, wave.wrapping_add(0x44)));
        let mut ang = q(f(mem, wave.wrapping_add(0x30)));
        let mut dang = q(f(mem, wave.wrapping_add(0x34)));
        for i in 0..25u32 {
            let idx = ffx_bridge::x87::ftol(mem, f64::from(ang));
            let za = Reg::load_f32(sin(mem, idx))
                .reg(MUL, Reg::load_f32(amp))
                .store();
            let base = mem.read::<u32>(h(8)).wrapping_add(0xc).wrapping_add(16 * i);
            let mut p = base.wrapping_add(omd);
            for _ in 0..17 {
                mem.write(p.wrapping_sub(4), za);
                mem.write(p, 1.0f32);
                p = p.wrapping_add(0x190);
            }
            z = Reg::load_f32(z)
                .mem(ADD, f(mem, wave.wrapping_add(0x48)))
                .store();
            amp = Reg::load_f32(amp).reg(ADD, Reg::load_f32(z)).store();
            dang = Reg::load_f32(dang)
                .mem(ADD, f(mem, wave.wrapping_add(0x38)))
                .store();
            ang = Reg::load_f32(ang).reg(ADD, Reg::load_f32(dang)).store();
        }
        // the normals tilt by the running mean of each ring's slope
        let deg = g(mem, 0x007e_7ed0);
        let slope = |mem: &mut M, fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64, i: u32| {
            let at = mem.read::<u32>(h(8)).wrapping_add(16 * i).wrapping_add(omd);
            let w = |mem: &M, o: u32| q(f(mem, at.wrapping_add(o))).to_bits();
            let args = [w(mem, 0), w(mem, 8), w(mem, 0x10), w(mem, 0x18), 0];
            #[allow(clippy::cast_possible_truncation)] // fstp m32
            let d = fcall(mem, deg, &args) as f32;
            d
        };
        let mut d = slope(mem, fcall, 0);
        let mut prev = ffx_bridge::x87::ftol(mem, f64::from(d)).wrapping_sub(0x400) & 0xfff;
        for i in 0..25u32 {
            let cur = if i == 0 {
                prev
            } else {
                if i != 0x18 {
                    d = slope(mem, fcall, i);
                }
                ffx_bridge::x87::ftol(mem, f64::from(d)).wrapping_sub(0x400) & 0xfff
            };
            let c = if cur >= 0x800 {
                cur.cast_signed() - 0x1000
            } else {
                cur.cast_signed()
            };
            let p0 = if prev >= 0x800 {
                prev.cast_signed() - 0x1000
            } else {
                prev.cast_signed()
            };
            #[allow(clippy::manual_midpoint)] // cdq; sub; sar: toward zero
            let mean = (c + p0) / 2;
            prev = mean.cast_unsigned() & 0xfff;
            let co = q(sin(mem, prev + 0x400));
            let si = q(sin(mem, prev));
            let mut p = mem
                .read::<u32>(h(0xc))
                .wrapping_add(16 * i)
                .wrapping_add(omd);
            for _ in 0..17 {
                let x = Reg::load_f32(f(mem, p)).reg(MUL, Reg::load_f32(si)).store();
                mem.write(p, x);
                let y = Reg::load_f32(f(mem, p.wrapping_add(4)))
                    .reg(MUL, Reg::load_f32(si))
                    .store();
                mem.write(p.wrapping_add(4), y);
                mem.write(p.wrapping_add(8), co);
                p = p.wrapping_add(0x190);
            }
        }
        // the colours
        let (t44, t34, t84, t24, t14) = (
            frame.wrapping_sub(0x44),
            frame.wrapping_sub(0x34),
            frame.wrapping_sub(0x84),
            frame.wrapping_sub(0x24),
            frame.wrapping_sub(0x14),
        );
        let (apply, sub, len2, scale, dot) = (
            g(mem, 0x006e_d2e0),
            g(mem, 0x006e_d940),
            g(mem, 0x006e_d9b0),
            g(mem, 0x006e_d8f0),
            g(mem, 0x0062_fce0),
        );
        let (k17, k128) = (c64(mem, 0x00b5_83a0), c64(mem, 0x00b1_70b8));
        let mut cols = vec![[0f32; 4]; VERTS as usize];
        let (mut vk, mut nk) = (vbase, nbase);
        for col in &mut cols {
            let m: u32 = mem.read(light.wrapping_add(0x2c));
            call(mem, apply, &[t44, m, vk]);
            let l: u32 = mem.read(light.wrapping_add(0x28));
            call(mem, sub, &[t34, l, t44]);
            let l: u32 = mem.read(light.wrapping_add(0x28));
            let i = f64::from(q(f(mem, l.wrapping_add(0xc))));
            let ll = fcall(mem, len2, &[t34]);
            let k = Reg::Num(ll).reg(|a, b| b / a, Reg::Num(i)).store();
            let k = q(k);
            call(mem, scale, &[t34, t34, k.to_bits()]);
            #[allow(clippy::cast_possible_truncation)] // fstp m32
            let ip = fcall(mem, dot, &[t34, nk]) as f32;
            let ip = q(ip);
            let ipc = if ip < 0.0 { 0.0 } else { ip };
            let b4d = f64::from(mem.read::<u8>(light.wrapping_add(0x4d)));
            let b4c = f64::from(mem.read::<u8>(light.wrapping_add(0x4c)));
            let f2 = Reg::load_f32(ipc)
                .constant(MUL, k17)
                .constant(MUL, b4d)
                .constant(MUL, k128)
                .store();
            let f1 = Reg::Num(k128).constant(MUL, b4c).store();
            let c: u32 = mem.read(light.wrapping_add(0x24));
            let (r2, r1) = (Reg::load_f32(f2), Reg::load_f32(f1));
            col[0] = r2.mem(MUL, f(mem, c)).reg(ADD, r1).store();
            col[1] = Reg::load_f32(f(mem, c.wrapping_add(4)))
                .reg(MUL, r2)
                .reg(ADD, r1)
                .store();
            col[2] = Reg::load_f32(f(mem, c.wrapping_add(8)))
                .reg(MUL, r2)
                .reg(ADD, r1)
                .store();
            col[3] = r2
                .reg(MUL, Reg::load_f32(f(mem, c.wrapping_add(0xc))))
                .reg(ADD, r1)
                .store();
            vk = vk.wrapping_add(0x10);
            nk = nk.wrapping_add(0x10);
        }
        // the outer radius
        let mut big = 0f32;
        let mut step = q(f(mem, obj.wrapping_add(0x30)));
        for _ in 0..25 {
            step = Reg::load_f32(step)
                .mem(ADD, f(mem, obj.wrapping_add(0x34)))
                .store();
            big = Reg::load_f32(big).reg(ADD, Reg::load_f32(step)).store();
        }
        // the projected extent
        let mulm = g(mem, 0x007e_83b0);
        call(
            mem,
            mulm,
            &[t84, view.wrapping_add(0x60), view.wrapping_add(0x20)],
        );
        call(mem, mulm, &[t84, view.wrapping_add(0xa0), t84]);
        let r0 = crate::ppp::bridge::R0.ptr(mem).addr;
        let rtp = g(mem, 0x007e_9b50);
        let (k_u, k_v) = (c64(mem, 0x00b5_83c8), c64(mem, 0x00b5_83c0));
        let proj = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, y: bool| {
            call(mem, rtp, &[t24, t84, t14]);
            let (o, base, kk) = if y {
                (4, 0x7200, k_v)
            } else {
                (0, 0x6ab0, k_u)
            };
            let n = mem.read::<u32>(t24 + o).wrapping_sub(base).cast_signed();
            #[allow(clippy::cast_precision_loss)] // fild, fstp m32
            let nf = n as f32;
            Reg::load_f32(nf).constant(DIV, kk).store()
        };
        for i in 0..4u32 {
            let x: u32 = mem.read(r0 + 4 * i);
            mem.write(t14 + 4 * i, x);
        }
        let u0 = proj(mem, call, false);
        let v0f = {
            // the second value of the same projection
            let n = mem.read::<u32>(t24 + 4).wrapping_sub(0x7200).cast_signed();
            #[allow(clippy::cast_precision_loss)]
            let nf = n as f32;
            Reg::load_f32(nf).constant(DIV, k_v).store()
        };
        mem.write(t14, q(big));
        let u_r = proj(mem, call, false);
        // fchs
        let negb = f32::from_bits(big.to_bits() ^ 0x8000_0000);
        mem.write(t14, q(negb));
        let u_l = proj(mem, call, false);
        for i in 0..4u32 {
            let x: u32 = mem.read(r0 + 4 * i);
            mem.write(t14 + 4 * i, x);
        }
        mem.write(t14 + 4, q(big));
        let v_r = proj(mem, call, true);
        mem.write(t14 + 4, q(negb));
        let v_l = proj(mem, call, true);
        // the extents from the centre, as positive
        let zero = Reg::Num(0.0);
        let abs = |x: f32| -> f32 {
            if x < 0.0 {
                zero.reg(SUB, Reg::load_f32(x)).store()
            } else {
                x
            }
        };
        let da = abs(Reg::load_f32(u_r).reg(SUB, Reg::load_f32(u0)).store());
        let db = abs(Reg::load_f32(u_l).reg(SUB, Reg::load_f32(u0)).store());
        let dc = abs(Reg::load_f32(v_r).reg(SUB, Reg::load_f32(v0f)).store());
        let dd = abs(Reg::load_f32(v_l).reg(SUB, Reg::load_f32(v0f)).store());
        let two_r = Reg::load_f32(big).reg(ADD, Reg::load_f32(big));
        let us = Reg::load_f32(db)
            .reg(ADD, Reg::load_f32(da))
            .reg(DIV, two_r)
            .store();
        let vs = Reg::load_f32(dd)
            .reg(ADD, Reg::load_f32(dc))
            .reg(DIV, two_r)
            .store();
        let scale4096 = Reg::Num(c64(mem, 0x00b5_15c8));
        let byte128 = Reg::load_f32(f(mem, g(mem, 0x00b5_01bc)));
        let (u0r, v0r) = (Reg::load_f32(u0), Reg::load_f32(v0f));
        let tex = |mem: &M, r: f32, a: u32, ca: u32| -> (u16, u16) {
            let y = Reg::load_f32(r).mem(MUL, sin(mem, a)).store();
            let x = Reg::load_f32(sin(mem, ca))
                .reg(MUL, Reg::load_f32(r))
                .store();
            let u = Reg::load_f32(x)
                .mem(MUL, us)
                .reg(ADD, u0r)
                .reg(MUL, scale4096)
                .value();
            let v = Reg::load_f32(vs)
                .mem(MUL, y)
                .reg(ADD, v0r)
                .reg(MUL, scale4096)
                .value();
            #[allow(clippy::cast_possible_truncation)] // the u16 stores
            (
                ffx_bridge::x87::ftol(mem, u) as u16,
                0x1000u16.wrapping_sub(ffx_bridge::x87::ftol(mem, v) as u16),
            )
        };
        // the quads: sector by sector, ring by ring
        let mut rec = omd.wrapping_add(0x30);
        for s in 0..16u32 {
            let a = (0x100 * s) & 0xfff;
            let ca = (a + 0x400) & 0xfff;
            let a2 = (a + 0x100) & 0xfff;
            let ca2 = (ca + 0x100) & 0xfff;
            let mut r = 0f32;
            let mut step = q(f(mem, obj.wrapping_add(0x30)));
            for k in 0..24u32 {
                #[allow(clippy::cast_possible_truncation)] // u16 indices
                let v = (25 * s + k) as u16;
                let idx = [v, v.wrapping_add(1), v.wrapping_add(25), v.wrapping_add(26)];
                for (n, i) in (0u32..).zip(idx) {
                    mem.write(rec.wrapping_add(0x10 + 2 * n), i);
                }
                let fade = 0x80i32 - (0x80 * k).cast_signed() / 24;
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let af = f32::from((fade as u32 & 0xff) as u8);
                for (n, i) in (0u32..).zip(idx) {
                    let c = cols[usize::from(i)];
                    for ch in 0..3u32 {
                        let x = Reg::load_f32(c[ch as usize]).reg(MUL, byte128);
                        #[allow(clippy::cast_possible_truncation)] // the low byte
                        mem.write(rec.wrapping_add(4 * n + ch), fistp_trunc(x) as u8);
                    }
                    let x = Reg::load_f32(c[3]).reg(MUL, Reg::load_f32(af));
                    #[allow(clippy::cast_possible_truncation)] // the low byte
                    mem.write(rec.wrapping_add(4 * n + 3), fistp_trunc(x) as u8);
                }
                let (u, vv) = tex(mem, r, a, ca);
                mem.write(rec.wrapping_add(0x18), u);
                mem.write(rec.wrapping_add(0x1a), vv);
                let (u, vv) = tex(mem, r, a2, ca2);
                mem.write(rec.wrapping_add(0x20), u);
                mem.write(rec.wrapping_add(0x22), vv);
                step = Reg::load_f32(step)
                    .mem(ADD, f(mem, obj.wrapping_add(0x34)))
                    .store();
                r = Reg::load_f32(r).reg(ADD, Reg::load_f32(step)).store();
                let (u, vv) = tex(mem, r, a, ca);
                mem.write(rec.wrapping_add(0x1c), u);
                mem.write(rec.wrapping_add(0x1e), vv);
                let (u, vv) = tex(mem, r, a2, ca2);
                mem.write(rec.wrapping_add(0x24), u);
                mem.write(rec.wrapping_add(0x26), vv);
                rec = rec.wrapping_add(0x28);
            }
        }
        mem.write(rec.wrapping_add(1), 0xffu8);
        let n = i32::from(mem.read::<i16>(h(0x14))).cast_unsigned();
        (n << 4).wrapping_add(mem.read(h(0xc))).wrapping_add(omd)
    }
}
