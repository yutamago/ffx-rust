//! The camera-facing model draws (`pppDrawMdlCameraDraw`,
//! `pppDrawMdlCameraLoopDraw`): a model record drawn with the object's
//! colour through a render request on the stack, like the other
//! `pppDrawMdl*` draws (`docs/formats/vfx.md`, "Model draws").

/// The functions over game memory.
pub mod bridge {
    use crate::draw::bridge::{ENV_SLOT, GET_PTR32, g};
    use crate::mdl::bridge::{MDL, SEMI2, ee_colour_at, fmod, submit};
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, ftol, load, mul, store};

    const IS_PTR32_NULL: u32 = 0x00a4_46a0;
    const UNIT_MATRIX: u32 = 0x006e_d9a0;
    const COPY_MATRIX: u32 = 0x006e_d360;
    const INVERSE_MATRIX: u32 = 0x006e_d450;
    const MUL_MATRIX: u32 = 0x006e_d5c0;
    const COPY_VECTOR_XYZ: u32 = 0x0062_fca0;
    const OP_RT_M_YXZ: u32 = 0x007e_9a30;
    const LENGTH_3: u32 = 0x0073_e140;
    const TRX_TEX_GET: u32 = 0x0091_9ba0;
    const TRX_TEX_SET_FRAME: u32 = 0x0091_9bd0;
    const CAMERA_TEX: u32 = 0x0073_dfe0;
    const CAMERA_LOOP_INIT: u32 = 0x0074_00f0;
    /// `g_pppColorConShift`.
    const COLOR_SHIFT: u32 = 0x00c8_b500;
    /// `g_pppCameraMatrix`.
    const CAMERA_MATRIX: u32 = 0x0230_ff60;
    /// `g_pppWorldMatrix`.
    const WORLD_MATRIX: u32 = 0x0230_ffa0;
    /// `g_pppViewMatrix`.
    const VIEW_MATRIX: u32 = 0x0230_ff20;
    /// `g_PppPacketPos`.
    const PACKET_POS: u32 = 0x0230_fd2c;
    /// The request's `+4` word.
    const REQUEST_WORD4: u32 = 0x00c4_98dc;
    /// The request's two matrix-table pointers (`+0x24`, `+0x28`).
    const REQUEST_TABLES: [u32; 2] = [0x0230_fd60, 0x0230_fda0];
    /// `vf00`, the emulated VU0's constant register.
    const VF00: u32 = 0x00c0_a004;
    /// The `f64` angle period of the positions (32768).
    const PERIOD: u32 = 0x00b5_1588;
    /// The `f64` float-to-16.16 scale (65536).
    const FIXED: u32 = 0x00b5_1558;
    /// The `f64` angle scale of the camera draw's rotation (651.8986...,
    /// 4096 / 2pi).
    const ANGLE: u32 = 0x00b5_1590;

    fn at(base: u32, off: u32) -> u32 {
        base.wrapping_add(off)
    }

    fn f64_at(mem: &impl Mem, a: u32) -> f64 {
        mem.read(g(mem, a))
    }

    /// `dst += src` on two floats in memory (`fld src; fadd dst; fstp
    /// dst`); returns the sum's bits.
    fn add_to(mem: &mut impl Mem, dst: u32, src: u32) -> u32 {
        let v = store(add(load(mem.read(src)), load(mem.read(dst))));
        mem.write(dst, v);
        v
    }

    /// The texture-animation step both draws share: with the key's speed
    /// byte (`+0x21`) set, the counter byte `count` steps, and when it
    /// reaches the key's byte `limit`, the texture `Yn_FpTrxTexGet(key
    /// byte tex)` (if there is one) moves to its next frame (byte `frame`,
    /// back to 0 at byte `frame + 1`) with the counter cleared.
    #[allow(clippy::too_many_arguments)]
    fn tex_step<M: Mem>(
        mem: &mut M,
        key: u32,
        limit: u32,
        tex: u32,
        count: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if mem.read::<u8>(at(key, 0x21)) == 0 {
            return;
        }
        let c = mem.read::<u8>(count).wrapping_add(1);
        mem.write(count, c);
        if c < mem.read::<u8>(at(key, limit)) {
            return;
        }
        let id = u32::from(mem.read::<u8>(at(key, tex)));
        let t = call(mem, g(mem, TRX_TEX_GET), &[id]);
        if t == 0 {
            return;
        }
        let mut n = mem.read::<u8>(frame).wrapping_add(1);
        mem.write(count, 0u8);
        if n >= mem.read::<u8>(frame.wrapping_add(1)) {
            n = 0;
        }
        mem.write(frame, n);
        call(mem, g(mem, TRX_TEX_SET_FRAME), &[t, u32::from(n)]);
    }

    /// The object's colour ([`ee_colour_at`] of work area 0) at the
    /// request's `+0x18` (through `g_pppColorConShift`); whether its alpha
    /// is not 0.
    fn colour<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        req: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let off: u32 = mem.read(mem.read::<u32>(at(prog, 0xc)));
        let c = ee_colour_at(mem, obj.wrapping_add(off), true, call);
        let shift = g(mem, COLOR_SHIFT);
        let sh: u32 = mem.read(shift);
        mem.write(at(req, 0x18).wrapping_add(sh), c);
        mem.write(shift, 0u32);
        mem.read::<u8>(at(req, 0x1b)) != 0
    }

    /// The model record of the key's model (`+4`), its data
    /// (`get_ptr32` of its `+0x1c`), or `None` for `0xffff`.
    fn model_record<M: Mem>(
        mem: &mut M,
        key: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<(u32, u32)> {
        let n: u32 = mem.read(at(key, 4));
        if n == 0xffff {
            return None;
        }
        let env: u32 = mem.read(mem.read::<u32>(g(mem, ENV_SLOT)));
        let rec = mem.read::<u32>(at(env, 0x20)).wrapping_add(n << 5);
        let word: u32 = mem.read(at(rec, 0x1c));
        let model = call(mem, g(mem, GET_PTR32), &[word]);
        Some((rec, model))
    }

    /// The request's blend `+8`: the key's `u16` at `blend`, or the
    /// model's `+0x16` when its low byte is `0xff`; `+0xa` and `+0xc`
    /// cleared, `+0x10`/`+0x14` the two positions in 16.16 (`_ftol2_sse`
    /// of `x * 65536`).
    fn blend_and_fixed(
        mem: &mut impl Mem,
        req: u32,
        key: u32,
        blend: u32,
        model: u32,
        xy: [u32; 2],
    ) {
        mem.write(at(req, 4), mem.read::<u32>(g(mem, REQUEST_WORD4)));
        let b = if mem.read::<u8>(at(key, blend)) == 0xff {
            mem.read::<u16>(at(model, 0x16))
        } else {
            mem.read::<u16>(at(key, blend))
        };
        mem.write(at(req, 8), b);
        mem.write(at(req, 0xa), 0u8);
        mem.write(at(req, 0xc), 0u32);
        let k = f64_at(mem, FIXED);
        let x = ftol(mem, mul(load(mem.read(xy[0])), k));
        mem.write(at(req, 0x10), x);
        let y = ftol(mem, mul(k, load(mem.read(xy[1]))));
        mem.write(at(req, 0x14), y);
    }

    /// `pppDrawMdlCameraDraw(obj, key, prog)` (`0x0073d930`), cdecl;
    /// `frame` the original's aligned frame pointer. The work area `w` is
    /// at `obj + 0xa0 + [[prog + 0xc] + 8]`. Nothing for a key model (`+4`) of
    /// `0xffff` or one without data. While `w +0x88` is a null `ptr32`,
    /// `pppDrawMdlCameraTex(key, w)` sets it up. Unless `g_pppStopStatus`:
    /// with `w` byte `+0x8f` clear, two `[pos, speed, accel]` channels
    /// (`+0x70`, `+0x7c`; on the key's frame its `+8..+0x1c` added) step,
    /// else the positions move by `w +0x60`/`+0x64`; both wrap with
    /// `_CIfmod(x, 32768)`; then the texture animation (key `+0x21`,
    /// `+0x20`; `w +0x8c..+0x8e`). The rotation `w +0x50..+0x58` times
    /// 4096/2pi (the double at `0x00b51590`) goes to `op_rt_m_yxz(w, M,
    /// angles, vf00)` with `M` a unit matrix moved to `w +0x40` plus the
    /// inverse camera's position; the request (at `frame - 0x1a0`) takes
    /// `w x obj +0x10` through the camera (at `frame - 0xa8`, and a copy at
    /// `frame - 0x128` for `+0x20`). `pppVu0Length_3` runs on the object's
    /// rows 0 to 2 for nothing (through `fcall`). With a non-zero alpha the
    /// render data is updated or created (no path 3). Callees through
    /// `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_mdl_camera_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let offs = rd(mem, at(prog, 0xc));
        let w = obj.wrapping_add(0xa0).wrapping_add(rd(mem, at(offs, 8)));
        let Some((rec, model)) = model_record(mem, key, call) else {
            return;
        };
        if model == 0 {
            return;
        }
        if call(mem, g(mem, IS_PTR32_NULL), &[rd(mem, at(w, 0x88))]) != 0 {
            call(mem, g(mem, CAMERA_TEX), &[key, w]);
        }
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            if mem.read::<u8>(at(w, 0x8f)) == 0 {
                if rd(mem, key) == rd(mem, at(obj, 0xc)) {
                    for k in 0..6 {
                        add_to(mem, at(w, 0x70 + 4 * k), at(key, 8 + 4 * k));
                    }
                }
                let t = add_to(mem, at(w, 0x74), at(w, 0x78));
                let p = store(add(load(t), load(rd(mem, at(w, 0x70)))));
                mem.write(at(w, 0x70), p);
                let u = add_to(mem, at(w, 0x80), at(w, 0x84));
                let q = store(add(load(u), load(rd(mem, at(w, 0x7c)))));
                mem.write(at(w, 0x7c), q);
            } else {
                add_to(mem, at(w, 0x70), at(w, 0x60));
                add_to(mem, at(w, 0x7c), at(w, 0x64));
            }
            let period = f64_at(mem, PERIOD);
            for o in [0x70, 0x7c] {
                let x = store(fmod(load(rd(mem, at(w, o))), period));
                mem.write(at(w, o), x);
            }
            tex_step(mem, key, 0x21, 0x20, at(w, 0x8c), at(w, 0x8d), call);
        }
        let at_f = |o: u32| frame.wrapping_sub(o);
        // the rotation's angles and vf00
        let (angles, one) = (at_f(0x24), at_f(0x14));
        let k = f64_at(mem, ANGLE);
        for e in 0..3 {
            let a = store(mul(load(rd(mem, at(w, 0x50 + 4 * e))), k));
            mem.write(at(angles, 4 * e), a);
        }
        mem.write(at(angles, 0xc), 0u32);
        let vf00: [u32; 4] = mem.read(g(mem, VF00));
        mem.write(one, vf00);
        for row in [0x10, 0x20, 0x30] {
            fcall(mem, g(mem, LENGTH_3), &[at(obj, row)]);
        }
        let m = at_f(0x68);
        call(mem, g(mem, UNIT_MATRIX), &[m]);
        let req = at_f(0x1a0);
        if !colour(mem, obj, prog, req, call) {
            return;
        }
        let d = mem.read::<u8>(at(key, 0x29));
        let mut flags = if d & 2 != 0 {
            4
        } else if d & 1 != 0 {
            0x80
        } else {
            0
        };
        if mem.read::<u8>(at(key, 0x24)) != 0 {
            flags |= 0x10;
        }
        if mem.read::<u8>(at(key, 0x25)) != 0 {
            flags = flags & !0x10 | 0x2000;
        }
        if mem.read::<u8>(at(key, 0x26)) != 0 {
            flags |= 0x200;
        }
        if mem.read::<u8>(at(key, 0x27)) != 0 {
            flags |= 8;
        }
        if mem.read::<u8>(at(key, 0x28)) == 0 {
            flags |= 0x20;
        }
        mem.write(req, flags);
        blend_and_fixed(mem, req, key, 0x22, model, [at(w, 0x70), at(w, 0x7c)]);
        let (copy, world) = (at_f(0x128), at_f(0xa8));
        mem.write(at(req, 0x20), copy);
        mem.write(at(req, 0x2c), at(obj, 0x10));
        mem.write(at(req, 0x1c), 0u32);
        mem.write(at(req, 0x44), 0u32);
        mem.write(at(req, 0x24), g(mem, REQUEST_TABLES[0]));
        mem.write(at(req, 0x28), g(mem, REQUEST_TABLES[1]));
        mem.write(at(req, 0x30), world);
        let t0 = call(mem, g(mem, GET_PTR32), &[rd(mem, at(obj, 0x90))]);
        mem.write(at(req, 0x38), t0);
        let t1 = call(mem, g(mem, GET_PTR32), &[rd(mem, at(obj, 0x94))]);
        mem.write(at(req, 0x3c), t1);
        mem.write(at(req, 0x40), 0u32);
        let camera = g(mem, CAMERA_MATRIX);
        let inverse = at_f(0xe8);
        call(mem, g(mem, INVERSE_MATRIX), &[inverse, camera]);
        // the unit matrix moved to w +0x40 plus the inverse camera's position
        for e in 0..3 {
            let p = load(rd(mem, at(w, 0x40 + 4 * e)));
            let c = load(rd(mem, at(inverse, 0x30 + 4 * e)));
            mem.write(at(m, 0x30 + 4 * e), store(add(p, c)));
        }
        call(mem, g(mem, OP_RT_M_YXZ), &[w, m, angles, one]);
        call(mem, g(mem, MUL_MATRIX), &[world, w, at(obj, 0x10)]);
        call(mem, g(mem, MUL_MATRIX), &[world, camera, world]);
        call(mem, g(mem, COPY_MATRIX), &[copy, world]);
        let packet = rd(mem, g(mem, PACKET_POS));
        mem.write(at(req, 0x34), packet);
        submit(mem, obj, rec, req, SEMI2, call);
    }

    /// `pppDrawMdlCameraLoopDraw(obj, key, prog)` (`0x0073fa50`), cdecl;
    /// `frame` the original's aligned frame pointer. Work area `w` as in
    /// [`ppp_draw_mdl_camera_draw`]; while its `+0` is a null `ptr32`,
    /// `pppDrawMdlCameraLoopInit_007400f0(key, w)` sets it up. Unless
    /// `g_pppStopStatus`: with `w` byte `+0x1c` set, two `[pos, speed,
    /// accel]` channels (`+4`, `+0x10`; on the key's frame its `+0xc`,
    /// `+0x10`, `+0x18`, `+0x1c` added to the speeds and accelerations)
    /// step and wrap with `_CIfmod(x, 32768)`; then the texture animation
    /// (key `+0x21`, `+0x23`, `+0x22`; `w +0x1d..+0x1f`). With a non-zero
    /// alpha, the request (at `frame - 0x1c0`) takes, with key byte
    /// `+0x20`, the inverse camera (`frame - 0x148`, its position copied
    /// to `frame - 0x118` by `FFXVu0CopyVectorXYZ(frame - 0x98, ..)`), the
    /// world through it (`frame - 0x48` = world x obj `+0x10`, `frame -
    /// 0x88` = camera x that), else the object's `+0x50` and `+0x10`
    /// copies; flags from key byte `+0x26` and the model's first byte,
    /// `0x200`/`0x100` for key `+0x27`/`+0x28`, and for `+0x29` `0x2000`
    /// (no `0x10`) with a view matrix (`g_pppViewMatrix x obj +0x10`, at
    /// `frame - 0x108`) in `+0x30`; the blend from key `+0x24`. The render
    /// data is updated or created (path 3 for flag 4). Callees through
    /// `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_mdl_camera_loop_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        prog: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let offs = rd(mem, at(prog, 0xc));
        let w = obj.wrapping_add(0xa0).wrapping_add(rd(mem, at(offs, 8)));
        let Some((_, model)) = model_record(mem, key, call) else {
            return;
        };
        if model == 0 {
            return;
        }
        if call(mem, g(mem, IS_PTR32_NULL), &[rd(mem, w)]) != 0 {
            call(mem, g(mem, CAMERA_LOOP_INIT), &[key, w]);
        }
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            if mem.read::<u8>(at(w, 0x1c)) != 0 {
                if rd(mem, key) == rd(mem, at(obj, 0xc)) {
                    for (dst, src) in [(8, 0xc), (0xc, 0x10), (0x14, 0x18), (0x18, 0x1c)] {
                        add_to(mem, at(w, dst), at(key, src));
                    }
                }
                let period = f64_at(mem, PERIOD);
                let t = add_to(mem, at(w, 8), at(w, 0xc));
                let u = add_to(mem, at(w, 0x14), at(w, 0x18));
                for (o, speed) in [(4, t), (0x10, u)] {
                    let x = load(store(add(load(speed), load(rd(mem, at(w, o))))));
                    mem.write(at(w, o), store(fmod(x, period)));
                }
            }
            tex_step(mem, key, 0x23, 0x22, at(w, 0x1d), at(w, 0x1e), call);
        }
        let at_f = |o: u32| frame.wrapping_sub(o);
        let req = at_f(0x1c0);
        if !colour(mem, obj, prog, req, call) {
            return;
        }
        let (m, world) = (at_f(0x88), at_f(0x48));
        if mem.read::<u8>(at(key, 0x20)) != 0 {
            let inverse = at_f(0x148);
            call(
                mem,
                g(mem, INVERSE_MATRIX),
                &[inverse, g(mem, CAMERA_MATRIX)],
            );
            let w_copy = at_f(0xc8);
            call(mem, g(mem, COPY_MATRIX), &[w_copy, g(mem, WORLD_MATRIX)]);
            call(mem, g(mem, COPY_VECTOR_XYZ), &[at_f(0x98), at_f(0x118)]);
            call(mem, g(mem, MUL_MATRIX), &[world, w_copy, at(obj, 0x10)]);
            call(mem, g(mem, MUL_MATRIX), &[m, g(mem, CAMERA_MATRIX), world]);
        } else {
            call(mem, g(mem, COPY_MATRIX), &[m, at(obj, 0x50)]);
            call(mem, g(mem, COPY_MATRIX), &[world, at(obj, 0x10)]);
        }
        let d = mem.read::<u8>(at(key, 0x26));
        let mut flags = if d & 2 != 0 {
            4
        } else if d & 1 != 0 {
            0x80
        } else {
            0
        };
        let m0 = mem.read::<u8>(model);
        if m0 & 1 == 0 {
            flags |= 8;
        }
        if m0 & 2 == 0 {
            flags |= 0x10;
        }
        if mem.read::<u8>(at(key, 0x27)) != 0 {
            flags |= 0x200;
        }
        if mem.read::<u8>(at(key, 0x28)) != 0 {
            flags |= 0x100;
        }
        let view = if mem.read::<u8>(at(key, 0x29)) != 0 {
            flags = flags & !0x10 | 0x2000;
            let v = at_f(0x108);
            call(
                mem,
                g(mem, MUL_MATRIX),
                &[v, g(mem, VIEW_MATRIX), at(obj, 0x10)],
            );
            v
        } else {
            0
        };
        mem.write(req, flags);
        mem.write(at(req, 0x30), view);
        blend_and_fixed(mem, req, key, 0x24, model, [at(w, 4), at(w, 0x10)]);
        mem.write(at(req, 0x20), m);
        mem.write(at(req, 0x2c), world);
        mem.write(at(req, 0x1c), 0u32);
        mem.write(at(req, 0x44), 0u32);
        mem.write(at(req, 0x24), g(mem, REQUEST_TABLES[0]));
        mem.write(at(req, 0x28), g(mem, REQUEST_TABLES[1]));
        let packet = rd(mem, g(mem, PACKET_POS));
        mem.write(at(req, 0x34), packet);
        let t0 = call(mem, g(mem, GET_PTR32), &[rd(mem, at(obj, 0x90))]);
        mem.write(at(req, 0x38), t0);
        let t1 = call(mem, g(mem, GET_PTR32), &[rd(mem, at(obj, 0x94))]);
        mem.write(at(req, 0x3c), t1);
        mem.write(at(req, 0x40), 0u32);
        // the model record read again for the renderer
        let n = rd(mem, at(key, 4));
        let env = rd(mem, rd(mem, g(mem, ENV_SLOT)));
        let rec = rd(mem, at(env, 0x20)).wrapping_add(n << 5);
        submit(mem, obj, rec, req, MDL, call);
    }
}
