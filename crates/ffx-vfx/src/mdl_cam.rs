//! The camera-facing and sea model draws (`pppDrawMdlCameraDraw`,
//! `pppDrawMdlCameraLoopDraw`, `pppDrawMdlSeaDraw`): a model record drawn
//! with the object's colour through a render request on the stack, like
//! the other `pppDrawMdl*` draws (`docs/formats/vfx.md`, "Model draws").

/// The functions over game memory.
pub mod bridge {
    use crate::draw::bridge::{ENV_SLOT, GET_PTR32, NAME_HASH, g, instance};
    use crate::mdl::bridge::{
        MDL, SEMI2, create, ee_colour_at, fmod, has_render_data, submit, update,
    };
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, div, ftol, load, mul, store};

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

    /// Two `[pos, speed, accel]` channels at `w + p1` and `w + p2`: each
    /// speed takes its acceleration, then each position `fmod(f32(pos +
    /// speed), 32768)` (the double at `0x00b51588`), in that order.
    fn step_channels(mem: &mut impl Mem, w: u32, p1: u32, p2: u32) {
        let period = f64_at(mem, PERIOD);
        let t = add_to(mem, at(w, p1 + 4), at(w, p1 + 8));
        let u = add_to(mem, at(w, p2 + 4), at(w, p2 + 8));
        for (o, speed) in [(p1, t), (p2, u)] {
            let x = load(store(add(load(speed), load(mem.read(at(w, o))))));
            mem.write(at(w, o), store(fmod(x, period)));
        }
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
                step_channels(mem, w, 4, 0x10);
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

    const SEA_SETUP: u32 = 0x0073_b5c0;
    const STORE_COMPUTED: u32 = 0x0091_af10;
    const IS_TEXTURE_LOADED: u32 = 0x0064_5400;
    const UPDATE_TEXTURE: u32 = 0x0064_5aa0;
    const DYN_GEO_UPDATE_TEXTURE: u32 = 0x0064_0060;
    const ALPHA_FUNCTION: u32 = 0x0071_2c30;
    const Z_MAT: u32 = 0x0071_2d00;
    /// The import slot of the C runtime's `sprintf`.
    const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `"trxtex_%d_%d_%d.dds.phyre"`.
    const TRXTEX_FORMAT: u32 = 0x00b5_1568;

    /// The levels (`graphicLevelGetNameHash`) `pppDrawMdlSeaDraw` draws
    /// on.
    pub const SEA_LEVELS: [u32; 14] = [
        0x199, 0x17e6, 0x24c9, 0xa95, 0x245b, 0xa27, 0x170a, 0xe58, 0x1b3b, 0xb3, 0x1acd, 0x99,
        0xe6c, 0xf48,
    ];

    /// `pppDrawMdlSeaDraw(obj, key, prog)` (`0x0073abe0`), cdecl; `frame`
    /// the original's aligned frame pointer. Only on the [`SEA_LEVELS`] and
    /// for a key model (`+4`) other than `0xffff`; the work area `w` (as
    /// in [`ppp_draw_mdl_camera_draw`]) is set up by
    /// `pppDrawMdlSeaDrawSetup(key, w)` while its `+0x70` is a null
    /// `ptr32` (nothing when that fails). Unless `g_pppStopStatus`: on the
    /// key's frame its `+8..+0x1c` are added to the channels `+0x50`,
    /// `+0x5c`, which step ([`step_channels`]); with `w` byte `+0x77` the
    /// model's vertices (`i16` triples at its `+8` offset, `i16 +0x12` of
    /// them, those whose flag byte in `ptr32 w +0x6c` is set) become the
    /// rest positions (`ptr32 w +0x68`) plus a wave: with `s = x + z`
    /// (rest, as `f32`) the phases `ftol((w[+0x40..+0x48] + s) *
    /// key[+0x40..+0x48])` pick `g_pppSinTable[phase >> 4 & 0xfff]`, times
    /// key `+0x20..+0x28`, truncated, added to each coordinate (x from
    /// phase 0, y 1, z 2); then the phases move by key `+0x30..+0x38`. The
    /// texture animation (key `+0x51` both the speed and the limit, `+0x50`
    /// the texture; `w +0x74..+0x76`) clears the counter even without a
    /// texture and, after the frame step, loads
    /// `trxtex_<frame>_<a>_<b>.dds.phyre` (`a`, `b` from
    /// `pppStoreComputed_0091af10`; `sprintf` into `frame - 0x208`) and
    /// hands it to the instance when `graphicVFXIsTextureLoaded` finds it.
    /// The colour is the inlined form of [`ee_colour_at`]; with a non-zero
    /// alpha the request at `frame - 0x2e0` (flags 4, `0x14` with key byte
    /// `+0x54`, `0x2000` without `0x10` and `g_pppViewMatrix x obj +0x10`
    /// in `+0x30` for `+0x55`, `0x200`/`8` for `+0x56`/`+0x57`; the blend
    /// from `+0x52`; positions `w +0x50`, `+0x5c` in 16.16; the object's
    /// `+0x50`/`+0x10` matrices) updates the render data (path 3 for flag
    /// 4, then `graphicDynamicGeometryUpdateTexture`) or creates it.
    /// Callees through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_draw_mdl_sea_draw<M: Mem>(
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
        let level = call(mem, g(mem, NAME_HASH), &[]);
        if !SEA_LEVELS.contains(&level) {
            return;
        }
        let Some((_, model)) = model_record(mem, key, call) else {
            return;
        };
        let state = call(mem, g(mem, GET_PTR32), &[rd(mem, at(w, 0x70))]);
        if state == 0 && call(mem, g(mem, SEA_SETUP), &[key, w]) == 0 {
            return;
        }
        let at_f = |o: u32| frame.wrapping_sub(o);
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            if rd(mem, key) == rd(mem, at(obj, 0xc)) {
                for k in 0..6 {
                    add_to(mem, at(w, 0x50 + 4 * k), at(key, 8 + 4 * k));
                }
            }
            step_channels(mem, w, 0x50, 0x5c);
            if mem.read::<u8>(at(w, 0x77)) != 0 {
                let rest = call(mem, g(mem, GET_PTR32), &[rd(mem, at(w, 0x68))]);
                let verts = model.wrapping_add(rd(mem, at(model, 8)));
                let flags = call(mem, g(mem, GET_PTR32), &[rd(mem, at(w, 0x6c))]);
                let sin = crate::ppp::bridge::SIN.ptr(mem).addr;
                let kf = |mem: &M, o: u32| load(rd(mem, at(key, o)));
                let wf = |mem: &M, o: u32| load(rd(mem, at(w, o)));
                let mut i = 0u32;
                while i.cast_signed() < i32::from(mem.read::<i16>(at(model, 0x12))) {
                    if mem.read::<u8>(flags.wrapping_add(i)) != 0 {
                        let (src, dst) = (rest.wrapping_add(6 * i), verts.wrapping_add(6 * i));
                        let x = i32::from(mem.read::<i16>(src));
                        let z = i32::from(mem.read::<i16>(at(src, 4)));
                        let s = load(store(f64::from(x.wrapping_add(z))));
                        let p1 = ftol(mem, mul(add(wf(mem, 0x44), s), kf(mem, 0x44)));
                        let p2 = ftol(mem, mul(add(wf(mem, 0x48), s), kf(mem, 0x48)));
                        let p0 = ftol(mem, mul(add(s, wf(mem, 0x40)), kf(mem, 0x40)));
                        for (k, phase) in [(0u32, p0), (1, p1), (2, p2)] {
                            let entry = ((phase.cast_signed() >> 4) & 0xfff).cast_unsigned();
                            let t = load(mem.read(sin.wrapping_add(4 * entry)));
                            let v = ftol(mem, mul(t, kf(mem, 0x20 + 4 * k)));
                            #[allow(clippy::cast_possible_truncation)] // 16-bit add
                            let v = (v as u16).wrapping_add(mem.read::<u16>(at(src, 2 * k)));
                            mem.write(at(dst, 2 * k), v);
                        }
                    }
                    i = i.wrapping_add(1);
                }
                for k in 0..3 {
                    add_to(mem, at(w, 0x40 + 4 * k), at(key, 0x30 + 4 * k));
                }
            }
            sea_texture(mem, obj, key, w, frame, call);
        }
        let req = at_f(0x2e0);
        if !colour_inline(mem, obj, prog, req, call) {
            return;
        }
        let n = rd(mem, at(key, 4));
        let env = rd(mem, rd(mem, g(mem, ENV_SLOT)));
        let rec = rd(mem, at(env, 0x20)).wrapping_add(n << 5);
        let model = call(mem, g(mem, GET_PTR32), &[rd(mem, at(rec, 0x1c))]);
        let mut flags = 4u32;
        if mem.read::<u8>(at(key, 0x54)) != 0 {
            flags = 0x14;
        }
        let view = if mem.read::<u8>(at(key, 0x55)) != 0 {
            flags = flags & !0x10 | 0x2000;
            let v = at_f(0x248);
            call(
                mem,
                g(mem, MUL_MATRIX),
                &[v, g(mem, VIEW_MATRIX), at(obj, 0x10)],
            );
            v
        } else {
            0
        };
        if mem.read::<u8>(at(key, 0x56)) != 0 {
            flags |= 0x200;
        }
        if mem.read::<u8>(at(key, 0x57)) != 0 {
            flags |= 8;
        }
        mem.write(req, flags);
        mem.write(at(req, 0x30), view);
        blend_and_fixed(mem, req, key, 0x52, model, [at(w, 0x50), at(w, 0x5c)]);
        mem.write(at(req, 0x20), at(obj, 0x50));
        mem.write(at(req, 0x2c), at(obj, 0x10));
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
        if has_render_data(mem, obj, call) {
            update(mem, obj, rec, req, true, call);
            let [lo, hi] = instance(mem, obj);
            call(mem, g(mem, DYN_GEO_UPDATE_TEXTURE), &[lo, hi]);
            return;
        }
        let blend = u32::from(mem.read::<u8>(at(req, 8)));
        let mut alpha = call(mem, g(mem, ALPHA_FUNCTION), &[blend]);
        if rd(mem, req) & 0x2000 == 0 {
            alpha |= call(mem, g(mem, Z_MAT), &[rd(mem, at(req, 0x44))]);
        }
        if mem.read::<u8>(req) & 8 != 0 {
            alpha |= 0x40;
        }
        create(mem, obj, rec, req, true, alpha, call);
    }

    /// The sea draw's texture animation (see [`ppp_draw_mdl_sea_draw`]).
    #[allow(clippy::many_single_char_names)]
    fn sea_texture<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        w: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let speed = mem.read::<u8>(at(key, 0x51));
        if speed == 0 {
            return;
        }
        let c = mem.read::<u8>(at(w, 0x74)).wrapping_add(1);
        mem.write(at(w, 0x74), c);
        if c < speed {
            return;
        }
        let id = u32::from(mem.read::<u8>(at(key, 0x50)));
        let t = call(mem, g(mem, TRX_TEX_GET), &[id]);
        if t != 0 {
            let mut n = mem.read::<u8>(at(w, 0x75)).wrapping_add(1);
            if n >= mem.read::<u8>(at(w, 0x76)) {
                n = 0;
            }
            mem.write(at(w, 0x75), n);
            call(mem, g(mem, TRX_TEX_SET_FRAME), &[t, u32::from(n)]);
            let (a, b) = (frame.wrapping_sub(0x2e4), frame.wrapping_sub(0x2e8));
            call(mem, g(mem, STORE_COMPUTED), &[t, a, b]);
            let (name, out) = (frame.wrapping_sub(0x208), frame.wrapping_sub(0x108));
            let sprintf: u32 = mem.read(g(mem, SPRINTF_SLOT));
            let args = [
                name,
                g(mem, TRXTEX_FORMAT),
                u32::from(n),
                mem.read(a),
                mem.read(b),
            ];
            call(mem, sprintf, &args);
            if call(mem, g(mem, IS_TEXTURE_LOADED), &[name, out, 0, 0, 0, 0]) != 0 {
                let [lo, hi] = instance(mem, obj);
                call(mem, g(mem, UPDATE_TEXTURE), &[lo, hi, out]);
            }
        }
        mem.write(at(w, 0x74), 0u8);
    }

    /// [`colour`] with the `pextlh` steps inlined (the original writes the
    /// emulated registers itself).
    fn colour_inline<M: Mem>(
        mem: &mut M,
        obj: u32,
        prog: u32,
        req: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> bool {
        let off: u32 = mem.read(mem.read::<u32>(at(prog, 0xc)));
        let c = ee_colour_at(mem, obj.wrapping_add(off), false, call);
        let shift = g(mem, COLOR_SHIFT);
        let sh: u32 = mem.read(shift);
        mem.write(at(req, 0x18).wrapping_add(sh), c);
        mem.write(shift, 0u32);
        mem.read::<u8>(at(req, 0x1b)) != 0
    }

    // ---- pppKeMdlBmpDraw ------------------------------------------------

    const BMP_SPR_ALLOC: u32 = 0x007e_6a00;
    const BMP_SPR_FREE: u32 = 0x007e_6a60;
    const DBG_PRINTF: u32 = 0x0062_f500;
    const SHR7: u32 = 0x0074_9050;
    const SCALE_VECTOR: u32 = 0x006e_d8f0;
    const SUB_VECTOR: u32 = 0x006e_d940;
    const NORMALIZE: u32 = 0x006e_d6f0;
    const APPLY_MATRIX: u32 = 0x006e_d2e0;
    const BMP_BODY: u32 = 0x0074_8b00;
    /// `g_pppStopStatus`.
    const STOP_STATUS: u32 = 0x0230_fd34;
    /// `g_pppCurrentPart`.
    const CURRENT_PART: u32 = 0x0230_ffe4;
    /// `g_pppRandTarget`: a target that is not an object.
    const RAND_TARGET: u32 = 0x00d5_2398;
    /// The `f32` 1/128 the colour is scaled by.
    const BMP_COLOUR_SCALE: u32 = 0x00b5_1640;
    /// The debug lines: the three edge cases' and the corner's.
    const BMP_FORMATS: [u32; 4] = [0x00b5_1618, 0x00b5_1620, 0x00b5_1628, 0x00b5_1630];

    /// `pppKeMdlBmpDraw(obj, key, ctrl)` (`0x007490d0`), cdecl; `frame` the
    /// original's aligned frame pointer.
    ///
    /// The first time (`obj +0xc` and key `+0` both 0, not
    /// `g_pppStopStatus`) it prepares the model's triangles in place: the
    /// records from the shape set's `[[env]+0x20] + key[+0x28] * 32 + 0x1c`
    /// (`+[+4]`, kind byte `+1` 2, `i16 +2` triangles of 32 bytes after a
    /// 0x10-byte head, until kind 0xff; another kind frees the 0x50-byte
    /// `op_spr_alloc` block and returns) with the vertices of
    /// `key[+0x24]`'s set (`+[+8]`, 6 bytes each). Per triangle, from its
    /// three `u16` points (`+0x14..+0x1e`) and vertex indices (`+0xc`,
    /// `+0xe`, `+0x10`), the longest edge in x picks the corner order;
    /// the block gets the edges' `f32` x and y, the slope term, and the
    /// three vertices as `i32`, and the triangle's first 12 bytes the
    /// first vertex's words. Triangle `key[+0x3c]` (counted down) goes to
    /// `dbgPrintf`.
    ///
    /// Then, with both shapes and a target (`key[+0x2c]` an object of the
    /// part's records, `+0xa0 + key[+0x30]`, not `g_pppRandTarget`): work
    /// area 1's colour `>> 7` / 128, the world matrix times the object's,
    /// the direction from its position to the target (normalized, through
    /// the inverse, normalized), and `pppKeMdlBmpDraw_Body_00748b00` on a
    /// block at `ebp - 0x100` (matrix, `obj + 0x50`, packet position, the
    /// two shape sets, colour twice, direction, `key[+0x3c]`); the packet
    /// position is the block's after.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_ke_mdl_bmp_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let mut go = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let at = |o: u32| frame.wrapping_sub(o);
        let shape = |mem: &M, k: u32| {
            let sets = rd(mem, rd(mem, rd(mem, g(mem, ENV_SLOT))).wrapping_add(0x20));
            rd(
                mem,
                sets.wrapping_add(k.wrapping_mul(32)).wrapping_add(0x1c),
            )
        };
        let (k24, k28) = (key.wrapping_add(0x24), key.wrapping_add(0x28));
        if rd(mem, g(mem, STOP_STATUS)) == 0
            && rd(mem, obj.wrapping_add(0xc)) == rd(mem, key)
            && rd(mem, obj.wrapping_add(0xc)) == 0
        {
            if rd(mem, k24) == 0xffff || rd(mem, k28) == 0xffff {
                return;
            }
            let spr = go(mem, BMP_SPR_ALLOC, &[0x50]);
            let s = shape(mem, rd(mem, k24));
            let verts = go(mem, GET_PTR32, &[s]);
            let verts = verts.wrapping_add(rd(mem, verts.wrapping_add(8)));
            let s = shape(mem, rd(mem, k28));
            let mut blk = go(mem, GET_PTR32, &[s]);
            blk = blk.wrapping_add(rd(mem, blk.wrapping_add(4)));
            loop {
                if mem.read::<u8>(blk.wrapping_add(1)) != 2 {
                    go(mem, BMP_SPR_FREE, &[0x50]);
                    return;
                }
                let count = i32::from(mem.read::<i16>(blk.wrapping_add(2)));
                let mut e = blk.wrapping_add(0x10);
                let next = e.wrapping_add(count.cast_unsigned().wrapping_mul(32));
                let mut c = count;
                while c > 0 {
                    bmp_triangle(mem, &mut go, spr, verts, e, c, key);
                    c -= 1;
                    e = e.wrapping_add(0x20);
                }
                blk = next;
                if mem.read::<u8>(blk.wrapping_add(1)) == 0xff {
                    break;
                }
            }
            go(mem, BMP_SPR_FREE, &[0x50]);
        }
        if rd(mem, k24) == 0xffff || rd(mem, k28) == 0xffff {
            return;
        }
        let k = rd(mem, key.wrapping_add(0x2c));
        if k == 0xffff_ffff {
            return;
        }
        let recs = rd(mem, rd(mem, g(mem, CURRENT_PART)).wrapping_add(0x3c));
        let other = go(
            mem,
            GET_PTR32,
            &[rd(
                mem,
                recs.wrapping_add(k.wrapping_mul(16)).wrapping_add(4),
            )],
        );
        let target = rd(mem, key.wrapping_add(0x30))
            .wrapping_add(0xa0)
            .wrapping_add(other);
        if target == g(mem, RAND_TARGET) {
            return;
        }
        let (colour, mat, inv, dir, block) = (at(0x24), at(0x68), at(0xa8), at(0x14), at(0x100));
        let area1 = obj
            .wrapping_add(0xa0)
            .wrapping_add(rd(mem, rd(mem, ctrl.wrapping_add(0xc)).wrapping_add(4)));
        go(mem, SHR7, &[colour, area1]);
        let scale = rd(mem, g(mem, BMP_COLOUR_SCALE));
        go(mem, SCALE_VECTOR, &[colour, colour, store(load(scale))]);
        let world = g(mem, WORLD_MATRIX);
        go(mem, MUL_MATRIX, &[mat, world, obj.wrapping_add(0x10)]);
        go(mem, SUB_VECTOR, &[dir, mat.wrapping_add(0x30), target]);
        go(mem, NORMALIZE, &[dir, dir]);
        mem.write(block, [mat, obj.wrapping_add(0x50)]);
        mem.write(block.wrapping_add(0x18), [colour, colour]);
        let packet = rd(mem, g(mem, PACKET_POS));
        mem.write(block.wrapping_add(8), packet);
        let s = shape(mem, rd(mem, k24));
        let v = go(mem, GET_PTR32, &[s]);
        mem.write(block.wrapping_add(0xc), v);
        let s = shape(mem, rd(mem, k28));
        let t = go(mem, GET_PTR32, &[s]);
        mem.write(block.wrapping_add(0x10), t);
        go(mem, INVERSE_MATRIX, &[inv, mat]);
        go(mem, APPLY_MATRIX, &[dir, inv, dir]);
        go(mem, NORMALIZE, &[dir, dir]);
        mem.write(block.wrapping_add(0x20), dir);
        let n = rd(mem, key.wrapping_add(0x3c));
        mem.write(block.wrapping_add(0x24), n);
        go(mem, BMP_BODY, &[block]);
        let packet = rd(mem, block.wrapping_add(8));
        mem.write(g(mem, PACKET_POS), packet);
    }

    /// One triangle `e` (number `c`, counted down) of
    /// [`ppp_ke_mdl_bmp_draw`]'s preparation into the block `spr`.
    #[allow(clippy::many_single_char_names)]
    fn bmp_triangle<M: Mem>(
        mem: &mut M,
        go: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        spr: u32,
        verts: u32,
        e: u32,
        c: i32,
        key: u32,
    ) {
        let u = |mem: &M, o: u32| i32::from(mem.read::<u16>(e.wrapping_add(o)));
        let f = |v: i32| store(f64::from(v));
        let put = |mem: &mut M, o: u32, v: u32| mem.write(spr.wrapping_add(o), v);
        let (x0, y0, x1, y1, x2, y2) = (
            u(mem, 0x14),
            u(mem, 0x16),
            u(mem, 0x18),
            u(mem, 0x1a),
            u(mem, 0x1c),
            u(mem, 0x1e),
        );
        // the edges' x: 2-1, 0-2 as floats, 1-0 through ftol
        put(mem, 0, f(x2.wrapping_sub(x1)));
        put(mem, 0x10, f(x0.wrapping_sub(x2)));
        let d = x1.wrapping_sub(x0);
        put(mem, 0x20, d.cast_unsigned());
        let a0 = load(store(load(mem.read(spr)).abs()));
        put(mem, 4, store(a0));
        let a4 = load(store(load(mem.read(spr.wrapping_add(0x10))).abs()));
        put(mem, 0x14, store(a4));
        let ad = d.wrapping_abs();
        put(mem, 0x24, ad.cast_unsigned());
        let adf = f64::from(ad);
        // the longest edge picks the corners: (dx, dy, x, y, three vertices)
        let (case, first, dy, dx, y, idx) = if (a4 > a0 && adf > a4) || (a4 <= a0 && adf > a0) {
            (
                1,
                f(d),
                f(y1 - y0),
                f(x2 - x0),
                f(y2 - y0),
                [0x10, 0xe, 0xc],
            )
        } else if a4 > a0 {
            (
                0,
                mem.read::<u32>(spr.wrapping_add(0x10)),
                f(y0 - y2),
                f(x1 - x2),
                f(y1 - y2),
                [0xe, 0xc, 0x10],
            )
        } else {
            (
                2,
                mem.read::<u32>(spr),
                f(y2 - y1),
                f(x0 - x1),
                f(y0 - y1),
                [0xc, 0x10, 0xe],
            )
        };
        put(mem, 0, store(load(first)));
        put(mem, 4, dy);
        put(mem, 0x10, dx);
        let slope = div(mul(load(dx), load(dy)), load(mem.read(spr)));
        put(mem, 0x14, store(slope));
        put(mem, 0x18, y);
        for (k, o) in (0u32..).zip(idx) {
            let v = verts.wrapping_add(6 * u32::from(mem.read::<u16>(e.wrapping_add(o))));
            for j in 0..3 {
                let w = i32::from(mem.read::<i16>(v.wrapping_add(2 * j)));
                put(mem, 0x20 + 0x10 * k + 4 * j, w.cast_unsigned());
            }
        }
        let n = mem.read::<u32>(key.wrapping_add(0x3c)).cast_signed();
        if c == n {
            let fmt = g(mem, BMP_FORMATS[case]);
            go(mem, DBG_PRINTF, &[fmt]);
        }
        let mut words = [0u32; 3];
        for (j, w) in (0u32..).zip(&mut words) {
            *w = store(load(mem.read(spr.wrapping_add(0x20 + 4 * j))));
            mem.write(e.wrapping_add(4 * j), *w);
        }
        if c == n {
            let mut args = vec![g(mem, BMP_FORMATS[3])];
            for w in words {
                let b = load(w).to_bits();
                #[allow(clippy::cast_possible_truncation)]
                args.extend([b as u32, (b >> 32) as u32]);
            }
            go(mem, DBG_PRINTF, &args);
        }
    }
}
