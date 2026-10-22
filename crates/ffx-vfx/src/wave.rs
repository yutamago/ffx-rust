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

    /// The slope mean of the wave meshes: `prev` and `cur` (4096ths, a
    /// quarter turn subtracted) as signed half turns, averaged toward
    /// zero.
    fn mean(prev: u32, cur: u32) -> u32 {
        let s = |x: u32| {
            if x >= 0x800 {
                x.cast_signed() - 0x1000
            } else {
                x.cast_signed()
            }
        };
        #[allow(clippy::manual_midpoint)] // cdq; sub; sar: toward zero
        let m = (s(cur) + s(prev)) / 2;
        m.cast_unsigned() & 0xfff
    }

    /// `opus_0154_wave_omd_make(obj, wave, uv, omd, view, proj)`
    /// (`0x007f67c0`): the 33 x 33 vertex disc of effect 0154 (1024 quads
    /// at `+0x30`, vertices `+0xc040`, normals `+0x10450`), its colours
    /// all `0x80808080` (the first through `g_pppColorConShift`, which is
    /// then cleared; the emulated `v0` keeps the colour).
    ///
    /// - Sector `j`, ring `i`: the angle `128 j + u` (`u` growing by
    ///   `wave +0x4c` per ring), radius from 0 by a step from `obj +0x30`
    ///   growing by `+0x34` (ring 32 also adding 5 steps), normal (cos,
    ///   sin).
    /// - Heights `(A_end - A) sin(ang_end - ang)` from the ring
    ///   recurrences of `wave +0x30`.. (as in `opus_wave_omd_make`, ring
    ///   32 adding `100 Z` to `A`), `w` 1.
    /// - Normals tilt by the running mean of the slope angle
    ///   (`op_deg_get`, the later rings measured backwards).
    /// - Texture coordinates: the outer radius projected through `proj *
    ///   view` over `z tan(fovy / 2)` (`1.7777` wide), centred at
    ///   `(1 + p) / 2`, scaled by `wave +0x3c` / 256, in 4096ths.
    ///
    /// Callees through `call`, `fcall` (`ST0`) and `tan` (the CRT's
    /// `_CItan`); the locals lie below `frame` (from `frame - 0xe4`).
    /// Returns the end of the normals.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )]
    pub fn opus_0154_wave_omd_make<M: Mem>(
        mem: &mut M,
        obj: u32,
        wave: u32,
        uv: u32,
        omd: u32,
        view: u32,
        proj: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
        tan: &mut impl FnMut(&mut M, f64) -> f64,
    ) -> u32 {
        const N: u32 = 33;
        let h = |o: u32| omd.wrapping_add(o);
        mem.write(h(0x10), 0x0441_0400u32);
        mem.write(h(8), 0xc040u32);
        mem.write(h(0xc), 0x0001_0450u32);
        mem.write(h(0), 0x10u32);
        mem.write(h(4), 0x20u32);
        mem.write(h(0x14), 0x441u16);
        mem.write(h(0x21), 6u8);
        let x: u16 = mem.read(h(0x10));
        mem.write(h(0x22), x);
        let x: u32 = mem.read(uv);
        mem.write(h(0x28), x);
        let x: u32 = mem.read(uv.wrapping_add(4));
        mem.write(h(0x2c), x);
        let mut vp = mem.read::<u32>(h(8)).wrapping_add(omd);
        let mut np = mem.read::<u32>(h(0xc)).wrapping_add(omd);
        // the disc
        let five = Reg::Num(c64(mem, 0x00b4_1d20));
        let k128 = Reg::Num(c64(mem, 0x00b4_c170));
        for j in 0..N {
            let mut r = 0f32;
            let mut step = q(f(mem, obj.wrapping_add(0x30)));
            let mut u = 0f32;
            #[allow(clippy::cast_precision_loss)] // fild, fstp m32: exact
            let jb = Reg::load_f32(j as f32).reg(MUL, k128);
            for i in 0..N {
                if i == 0x20 {
                    r = Reg::load_f32(step).reg(MUL, five).mem(ADD, r).store();
                }
                let t = Reg::load_f32(u).reg(ADD, jb).store();
                let a = ffx_bridge::x87::ftol(mem, f64::from(t)) & 0xfff;
                let ca = (a + 0x400) & 0xfff;
                let c = q(sin(mem, ca));
                mem.write(np, c);
                let sn = q(sin(mem, a));
                mem.write(np.wrapping_add(4), sn);
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
                u = Reg::load_f32(u)
                    .mem(ADD, f(mem, wave.wrapping_add(0x4c)))
                    .store();
                vp = vp.wrapping_add(0x10);
                np = np.wrapping_add(0x10);
            }
        }
        // the ring recurrences, first to their ends
        let start = |mem: &M| {
            (
                q(f(mem, wave.wrapping_add(0x40))),
                q(f(mem, wave.wrapping_add(0x44))),
                q(f(mem, wave.wrapping_add(0x30))),
                q(f(mem, wave.wrapping_add(0x34))),
            )
        };
        let (mut amp, mut z, mut ang, mut dang) = start(mem);
        for _ in 0..N {
            z = Reg::load_f32(z)
                .mem(ADD, f(mem, wave.wrapping_add(0x48)))
                .store();
            amp = Reg::load_f32(z).mem(ADD, amp).store();
            dang = Reg::load_f32(dang)
                .mem(ADD, f(mem, wave.wrapping_add(0x38)))
                .store();
            ang = Reg::load_f32(dang).mem(ADD, ang).store();
        }
        let (amp_end, ang_end) = (q(amp), q(ang));
        let (mut amp, mut z, mut ang, mut dang) = start(mem);
        let hundred = Reg::Num(c64(mem, 0x00b9_22e0));
        for i in 0..N {
            if i == 0x20 {
                amp = hundred
                    .reg(MUL, Reg::load_f32(z))
                    .reg(ADD, Reg::load_f32(amp))
                    .store();
            }
            let base = mem.read::<u32>(h(8)).wrapping_add(16 * i).wrapping_add(0xc);
            let idx = ffx_bridge::x87::ftol(
                mem,
                Reg::load_f32(ang_end).reg(SUB, Reg::load_f32(ang)).value(),
            );
            let hgt = Reg::load_f32(amp_end)
                .reg(SUB, Reg::load_f32(amp))
                .reg(MUL, Reg::load_f32(sin(mem, idx)))
                .store();
            let mut p = base.wrapping_add(omd);
            for _ in 0..N {
                mem.write(p.wrapping_sub(4), hgt);
                mem.write(p, 1.0f32);
                p = p.wrapping_add(0x210);
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
        // the normals tilt by the running slope mean
        let deg = g(mem, 0x007e_7ed0);
        let slope = |mem: &mut M, fcall: &mut dyn FnMut(&mut M, u32, &[u32]) -> f64, i: u32| {
            let at = mem.read::<u32>(h(8)).wrapping_add(16 * i).wrapping_add(omd);
            let w = |mem: &M, o: u32| q(f(mem, at.wrapping_add(o))).to_bits();
            let args = if i == 0 {
                [w(mem, 0), w(mem, 8), w(mem, 0x10), w(mem, 0x18), 0]
            } else {
                [w(mem, 0x10), w(mem, 0x18), w(mem, 0), w(mem, 8), 0]
            };
            #[allow(clippy::cast_possible_truncation)] // fstp m32
            let d = fcall(mem, deg, &args) as f32;
            d
        };
        let mut d = slope(mem, fcall, 0);
        let mut prev = ffx_bridge::x87::ftol(mem, f64::from(d)).wrapping_sub(0x400) & 0xfff;
        for i in 0..N {
            let cur = if i == 0 {
                prev
            } else {
                if i != 0x20 {
                    d = slope(mem, fcall, i);
                }
                ffx_bridge::x87::ftol(mem, f64::from(d)).wrapping_sub(0x400) & 0xfff
            };
            prev = mean(prev, cur);
            let co = q(sin(mem, prev + 0x400));
            let si = q(sin(mem, prev));
            let mut p = mem
                .read::<u32>(h(0xc))
                .wrapping_add(16 * i)
                .wrapping_add(omd);
            for _ in 0..N {
                let x = Reg::load_f32(f(mem, p)).reg(MUL, Reg::load_f32(si)).store();
                mem.write(p, x);
                let y = Reg::load_f32(f(mem, p.wrapping_add(4)))
                    .reg(MUL, Reg::load_f32(si))
                    .store();
                mem.write(p.wrapping_add(4), y);
                mem.write(p.wrapping_add(8), co);
                p = p.wrapping_add(0x210);
            }
        }
        // the outer radius
        let mut big = 0f32;
        let mut step = q(f(mem, obj.wrapping_add(0x30)));
        for _ in 0..N {
            step = Reg::load_f32(step)
                .mem(ADD, f(mem, obj.wrapping_add(0x34)))
                .store();
            big = Reg::load_f32(big).reg(ADD, Reg::load_f32(step)).store();
        }
        // the projected extent
        let fov = fcall(mem, g(mem, 0x0063_9570), &[]);
        let half = Reg::Num(fov).constant(MUL, c64(mem, 0x00b9_2220)).value();
        #[allow(clippy::cast_possible_truncation)] // fstp m32
        let t = q(tan(mem, half) as f32);
        let (ta4, te4, t64, t24, t14) = (
            frame.wrapping_sub(0xa4),
            frame.wrapping_sub(0xe4),
            frame.wrapping_sub(0x64),
            frame.wrapping_sub(0x24),
            frame.wrapping_sub(0x14),
        );
        let copy = g(mem, 0x006e_d360);
        call(mem, copy, &[ta4, view]);
        call(mem, copy, &[te4, proj]);
        call(mem, g(mem, 0x006e_d5c0), &[t64, te4, ta4]);
        let apply = g(mem, 0x006e_d2e0);
        let aspect = c64(mem, 0x00b4_3d68);
        let put4 = |mem: &mut M, v: [f32; 4]| {
            for (i, x) in (0u32..).zip(v) {
                mem.write(t14 + 4 * i, x);
            }
        };
        let pz = |mem: &M| Reg::load_f32(f(mem, t24 + 8)).mem(MUL, t);
        put4(mem, [0.0, 0.0, 0.0, 1.0]);
        call(mem, apply, &[t24, t64, t14]);
        let u0 = Reg::load_f32(f(mem, t24))
            .reg(DIV, pz(mem).constant(MUL, aspect))
            .store();
        let v0 = pz(mem).mem(|a, b| b / a, f(mem, t24 + 4)).store();
        mem.write(t14, q(big));
        call(mem, apply, &[t24, t64, t14]);
        let u_r = Reg::load_f32(f(mem, t24))
            .reg(DIV, pz(mem).constant(MUL, aspect))
            .store();
        // fchs
        let negb = f32::from_bits(big.to_bits() ^ 0x8000_0000);
        mem.write(t14, q(negb));
        call(mem, apply, &[t24, t64, t14]);
        let u_l = Reg::load_f32(f(mem, t24))
            .reg(DIV, pz(mem).constant(MUL, aspect))
            .store();
        put4(mem, [0.0, q(big), 0.0, 1.0]);
        call(mem, apply, &[t24, t64, t14]);
        let v_r = Reg::load_f32(f(mem, t24 + 4)).reg(DIV, pz(mem)).store();
        mem.write(t14 + 4, q(negb));
        call(mem, apply, &[t24, t64, t14]);
        let v_l = Reg::load_f32(f(mem, t24 + 4)).reg(DIV, pz(mem)).store();
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
        let dc = abs(Reg::load_f32(v_r).reg(SUB, Reg::load_f32(v0)).store());
        let dd = abs(Reg::load_f32(v_l).reg(SUB, Reg::load_f32(v0)).store());
        let half_r = Reg::Num(c64(mem, 0x00b9_2220));
        let one = Reg::Num(1.0);
        let u_c = Reg::load_f32(u0).reg(ADD, one).reg(MUL, half_r).store();
        let v_c = one.mem(ADD, v0).reg(MUL, half_r).store();
        let two_r = Reg::load_f32(big).reg(ADD, Reg::load_f32(big));
        let us = Reg::load_f32(db)
            .reg(ADD, Reg::load_f32(da))
            .reg(DIV, two_r)
            .store();
        let us = Reg::load_f32(us).reg(MUL, half_r).store();
        let vs = Reg::load_f32(dd)
            .reg(ADD, Reg::load_f32(dc))
            .reg(DIV, two_r)
            .store();
        let vs = half_r.reg(MUL, Reg::load_f32(vs)).store();
        let rot = Reg::load_f32(f(mem, wave.wrapping_add(0x3c)))
            .constant(MUL, c64(mem, 0x00b5_0198))
            .store();
        let scale = Reg::Num(c64(mem, 0x00b5_15c8));
        let (ucr, vcr, rotr) = (Reg::load_f32(u_c), Reg::load_f32(v_c), Reg::load_f32(rot));
        let texc = |mem: &M, x: f32, y: f32| -> (u16, u16) {
            let u = Reg::load_f32(x)
                .mem(MUL, us)
                .reg(MUL, rotr)
                .reg(ADD, ucr)
                .reg(MUL, scale)
                .value();
            let v = Reg::load_f32(vs)
                .mem(MUL, y)
                .reg(MUL, rotr)
                .reg(ADD, vcr)
                .reg(MUL, scale)
                .value();
            #[allow(clippy::cast_possible_truncation)] // the u16 stores
            (
                ffx_bridge::x87::ftol(mem, u) as u16,
                0x1000u16.wrapping_sub(ffx_bridge::x87::ftol(mem, v) as u16),
            )
        };
        // the quads, the colours constant
        let regs = crate::md::bridge::SPR_BOUNCE.ptr(mem).addr;
        let shift = g(mem, 0x00c8_b500);
        let mut rec = omd.wrapping_add(0x30);
        for s in 0..32u32 {
            let a = 0x80 * s;
            let ca = (a + 0x400) & 0xfff;
            let a2 = (a + 0x80) & 0xfff;
            let ca2 = (ca + 0x80) & 0xfff;
            let mut r = 0f32;
            let mut step = q(f(mem, obj.wrapping_add(0x30)));
            for k in 0..32u32 {
                #[allow(clippy::cast_possible_truncation)] // u16 indices
                let v = (N * s + k) as u16;
                for (n, i) in (0u32..).zip([
                    v,
                    v.wrapping_add(1),
                    v.wrapping_add(0x21),
                    v.wrapping_add(0x22),
                ]) {
                    mem.write(rec.wrapping_add(0x10 + 2 * n), i);
                }
                let sh: u32 = mem.read(shift);
                mem.write(regs, 0x8080_8080u32);
                mem.write(regs + 4, 0xffff_ffffu32);
                mem.write(rec.wrapping_add(sh), 0x8080_8080u32);
                for o in [4u32, 8, 0xc] {
                    let c: u32 = mem.read(regs);
                    mem.write(rec.wrapping_add(o), c);
                }
                mem.write(shift, 0u32);
                let sa = sin(mem, a);
                let y = Reg::load_f32(sa).reg(MUL, Reg::load_f32(r)).store();
                let x = Reg::load_f32(sin(mem, ca))
                    .reg(MUL, Reg::load_f32(r))
                    .store();
                let (uu, vv) = texc(mem, x, y);
                mem.write(rec.wrapping_add(0x18), uu);
                mem.write(rec.wrapping_add(0x1a), vv);
                let y = Reg::load_f32(sin(mem, a2))
                    .reg(MUL, Reg::load_f32(r))
                    .store();
                let x = Reg::load_f32(r).mem(MUL, sin(mem, ca2)).store();
                let (uu, vv) = texc(mem, x, y);
                mem.write(rec.wrapping_add(0x20), uu);
                mem.write(rec.wrapping_add(0x22), vv);
                step = Reg::load_f32(step)
                    .mem(ADD, f(mem, obj.wrapping_add(0x34)))
                    .store();
                r = Reg::load_f32(r).reg(ADD, Reg::load_f32(step)).store();
                let y = Reg::load_f32(sa).reg(MUL, Reg::load_f32(r)).store();
                let x = Reg::load_f32(sin(mem, ca))
                    .reg(MUL, Reg::load_f32(r))
                    .store();
                let (uu, vv) = texc(mem, x, y);
                mem.write(rec.wrapping_add(0x1c), uu);
                mem.write(rec.wrapping_add(0x1e), vv);
                let y = Reg::load_f32(sin(mem, a2))
                    .reg(MUL, Reg::load_f32(r))
                    .store();
                let x = Reg::load_f32(r).mem(MUL, sin(mem, ca2)).store();
                let (uu, vv) = texc(mem, x, y);
                mem.write(rec.wrapping_add(0x24), uu);
                mem.write(rec.wrapping_add(0x26), vv);
                rec = rec.wrapping_add(0x28);
            }
        }
        mem.write(rec.wrapping_add(1), 0xffu8);
        let n = i32::from(mem.read::<i16>(h(0x14))).cast_unsigned();
        (n << 4).wrapping_add(omd).wrapping_add(mem.read(h(0xc)))
    }

    /// `opus_0706_noise_draw(obj)` (`0x007f73d0`, a draw callback): one
    /// wave mesh of the 0706 noise effect. `obj` holds the opus set
    /// (`+0`), the part (`+4`), the wave (`+8`), the oef (`+0xc`) and its
    /// draw record `d` (`+0x10`), and the matrices at `+0x20`, `+0x60` and
    /// `+0xa0`. In a 0x400-byte scratch `s` (`op_spr_alloc`): the draw
    /// head at `s + 0x100` (`opus_mdc_head_set(head, s, set, part, -1)`,
    /// then `+0` = 0x44c, `+4` = `0x017c0870`, `+0x1c` = 0) with the part's
    /// lights (`opus_mdc_lights_get` of `d`'s ten bytes `+8`..`+0x11`), bytes
    /// `+0x4c`/`+0x4d` = `ftol(part f32 +0x3c / +0x4c x 0.5)`, `+0x50` =
    /// 0x01ff0001, `+0x54` = 0x019f0001; `s + 0x80` = the `+0x20` matrix,
    /// `s + 0xc0` = `inverse(g_pppScreenMatrix)` (kept in
    /// `g_pppDttVertexMatrix`) x `+0xa0` x `+0x60` x it. The model goes to
    /// the 0x20000-byte buffer `0x012e9624` (allocated on first use;
    /// `0x012e9628` = the free end, set by `opus_wave_omd_make(part, wave,
    /// tex0, at, obj, head)` with the oef's texture 0 for `u16 d +6`);
    /// head `+0x38` = the model's end (`at + at +8`). Then the renderer
    /// instance `(0x15, part)`: updated when it has render data, else
    /// created with flags `alpha function(head byte +8) | z(head +0x44) |
    /// 0x40 for head bit 3 | 0x800`; its DTT updated either way.
    /// `op_spr_free(0x400)`'s result is returned. All callees through
    /// `call`.
    ///
    /// The locals are below `frame` (the original's frame pointer, down to
    /// `frame - 0x68`: the render data request there, the texture at
    /// `frame - 0x30`, the light numbers at `frame - 0x10`).
    #[allow(clippy::many_single_char_names)]
    pub fn opus_0706_noise_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul};
        let w = |mem: &M, k: u32| mem.read::<u32>(obj.wrapping_add(4 * k));
        let go = |mem: &mut M,
                  call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                  at: u32,
                  args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let spr = go(mem, call, 0x007e_6a00, &[0x400]);
        let d = w(mem, 4);
        let (set, part, wave) = (w(mem, 0), w(mem, 1), w(mem, 2));
        let tex = frame.wrapping_sub(0x30);
        let no = u32::from(mem.read::<u16>(d.wrapping_add(6)));
        go(mem, call, 0x007e_5260, &[w(mem, 3), no, tex]);
        let (base, end) = (g(mem, 0x012e_9624), g(mem, 0x012e_9628));
        let at = if mem.read::<u32>(base) == 0 {
            let p = go(mem, call, 0x0068_7020, &[0x20000]);
            mem.write(base, p);
            mem.write(end, p);
            p
        } else {
            mem.read(end)
        };
        let head = spr.wrapping_add(0x100);
        go(mem, call, 0x0080_2450, &[head, spr, set, part, u32::MAX]);
        mem.write(head, 0x44cu32);
        mem.write(head.wrapping_add(4), g(mem, 0x017c_0870));
        mem.write(head.wrapping_add(0x1c), 0u32);
        let nos = frame.wrapping_sub(0x10);
        let ws: [u32; 2] = mem.read(d.wrapping_add(8));
        mem.write(nos, ws);
        let h: u16 = mem.read(d.wrapping_add(0x10));
        mem.write(nos.wrapping_add(8), h);
        go(mem, call, 0x0080_2bd0, &[head, spr, set, part, nos]);
        let half = c64(mem, 0x00b9_2220);
        for (to, from) in [(0x4c, 0x3c), (0x4d, 0x4c)] {
            let v = mul(load(mem.read(part.wrapping_add(from))), half);
            #[allow(clippy::cast_possible_truncation)] // the low byte in al
            mem.write(head.wrapping_add(to), ftol(mem, v) as u8);
        }
        mem.write(head.wrapping_add(0x50), 0x01ff_0001u32);
        mem.write(head.wrapping_add(0x54), 0x019f_0001u32);
        let (m80, mc0) = (spr.wrapping_add(0x80), spr.wrapping_add(0xc0));
        go(mem, call, 0x007e_8010, &[m80, obj.wrapping_add(0x20)]);
        go(mem, call, 0x007e_83b0, &[mc0, obj.wrapping_add(0x60), m80]);
        go(mem, call, 0x007e_83b0, &[mc0, obj.wrapping_add(0xa0), mc0]);
        let (dtt, screen) = (g(mem, 0x0230_fc60), g(mem, 0x0230_fe60));
        go(mem, call, 0x006e_d460, &[dtt, screen]);
        go(mem, call, 0x007e_83b0, &[mc0, dtt, mc0]);
        let free = go(mem, call, 0x007f_94e0, &[part, wave, tex, at, obj, head]);
        mem.write(end, free);
        let size: u32 = mem.read(at.wrapping_add(8));
        mem.write(head.wrapping_add(0x38), size.wrapping_add(at));
        let req = frame.wrapping_sub(0x68);
        if go(mem, call, 0x0064_53d0, &[0x15, part]) == 0 {
            let alpha = u32::from(mem.read::<u8>(head.wrapping_add(8)));
            let a = go(mem, call, 0x0071_2c30, &[alpha]);
            let z_in: u32 = mem.read(head.wrapping_add(0x44));
            let z = go(mem, call, 0x0071_2d00, &[z_in]);
            let mut flags = a | z;
            if mem.read::<u8>(head) & 8 != 0 {
                flags |= 0x40;
            }
            go(
                mem,
                call,
                0x0071_1180,
                &[0x15, part, req, head, flags | 0x800],
            );
        } else {
            go(mem, call, 0x0071_8610, &[0x15, part, req, head]);
        }
        go(mem, call, 0x0064_5330, &[0x15, part]);
        go(mem, call, 0x007e_6a60, &[0x400])
    }

    /// `opus_0154_wave_draw(set, unit, a2, nos, lights, view, proj, a7)`
    /// (`0x007f65d0`; `opus_wave_cmd_7f5f70`), `frame` the original's frame
    /// pointer (the render data request at `frame - 0x28`; only its `+0x1c`,
    /// the shape, is written): a scratch block (`op_spr_alloc(0x400)`) and
    /// one of two 0x14860-byte shapes (`0x012e9618[0x012e9620]`, made with
    /// `user_aligned_malloc` the first time; the index flips) that
    /// `opus_0154_wave_omd_make(unit, a2, nos, shape, view, proj, a7)` fills.
    /// The head at scratch `+0x100` as in [`opus_0706_noise_draw`] (word
    /// 0x44c, `+4` = 0x01f80000, lights from `lights`, the base and gain
    /// bytes `ftol(unit +0x3c / 2)`, `ftol(unit +0x4c / 2)`), `+0x38` the
    /// shape's data, the shape's flags bit 4 (`[nos] & 0x3fff` below
    /// 0x3400) or bit 2, matrices `+0x80` = `view`, `+0xc0` = `proj x view`,
    /// `opus_wave_light_007f60b0(unit, shape, head)`; then instance
    /// `(0x3400 <= [nos] & 0x3fff, 0x9a)` updated (`graphicVFXPath3Update`)
    /// or made (`InitVFXPath3RenderData`, flags from the alpha, z and head
    /// bit 3). Returns `op_spr_free`'s `eax`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn opus_0154_wave_draw<M: Mem>(
        mem: &mut M,
        args: [u32; 8],
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, mul};
        let [set, unit, a2, nos, lights, view, proj, a7] = args;
        let go = |mem: &mut M,
                  call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                  at: u32,
                  args: &[u32]| {
            let f = g(mem, at);
            call(mem, f, args)
        };
        let spr = go(mem, call, 0x007e_6a00, &[0x400]);
        let (shapes, index) = (g(mem, 0x012e_9618), g(mem, 0x012e_9620));
        let slot = shapes.wrapping_add(mem.read::<u32>(index).wrapping_mul(4));
        if mem.read::<u32>(slot) == 0 {
            let p = go(mem, call, 0x0068_6ff0, &[0x14860, 0x10]);
            mem.write(slot, p);
        }
        let shape: u32 = mem.read(slot);
        let flip = mem.read::<u32>(index) ^ 1;
        mem.write(index, flip);
        go(
            mem,
            call,
            0x007f_67c0,
            &[unit, a2, nos, shape, view, proj, a7],
        );
        let head = spr.wrapping_add(0x100);
        go(mem, call, 0x0080_2450, &[head, spr, set, unit, u32::MAX]);
        mem.write(head, 0x44cu32);
        mem.write(head.wrapping_add(4), 0x01f8_0000u32);
        mem.write(head.wrapping_add(0x1c), 0u32);
        go(mem, call, 0x0080_2bd0, &[head, spr, set, unit, lights]);
        let half = c64(mem, 0x00b9_2220);
        for (to, from) in [(0x4c, 0x3c), (0x4d, 0x4c)] {
            let v = mul(load(mem.read(unit.wrapping_add(from))), half);
            #[allow(clippy::cast_possible_truncation)] // the low byte in al
            mem.write(head.wrapping_add(to), ftol(mem, v) as u8);
        }
        mem.write(head.wrapping_add(0x50), 0x01ff_0001u32);
        mem.write(head.wrapping_add(0x54), 0x019f_0001u32);
        let data = mem.read::<u32>(shape.wrapping_add(8)).wrapping_add(shape);
        mem.write(head.wrapping_add(0x38), data);
        let high = mem.read::<u32>(nos) & 0x3fff >= 0x3400;
        let flags: u32 = mem.read(shape);
        let flags = if high {
            flags & !0x10 | 4
        } else {
            flags & !4 | 0x10
        };
        mem.write(shape, flags);
        let (m80, mc0) = (spr.wrapping_add(0x80), spr.wrapping_add(0xc0));
        go(mem, call, 0x007e_8010, &[m80, view]);
        go(mem, call, 0x007e_83b0, &[mc0, proj, m80]);
        go(mem, call, 0x007f_60b0, &[unit, shape, head]);
        let req = frame.wrapping_sub(0x28);
        mem.write(req.wrapping_add(0x1c), shape);
        let lo = u32::from(high);
        if go(mem, call, 0x0064_53d0, &[lo, 0x9a]) == 0 {
            let alpha = u32::from(mem.read::<u8>(head.wrapping_add(8)));
            let a = go(mem, call, 0x0071_2c30, &[alpha]);
            let z_in: u32 = mem.read(head.wrapping_add(0x44));
            let mut f = a | go(mem, call, 0x0071_2d00, &[z_in]);
            if mem.read::<u8>(head) & 8 != 0 {
                f |= 0x40;
            }
            go(mem, call, 0x0071_0fd0, &[lo, 0x9a, req, head, f]);
        } else {
            go(mem, call, 0x0071_8610, &[lo, 0x9a, req, head]);
            go(mem, call, 0x0064_5600, &[lo, 0x9a]);
        }
        go(mem, call, 0x007e_6a60, &[0x400])
    }
}
