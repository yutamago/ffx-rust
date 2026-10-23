//! The tail draws (`pppKeShpTail*Draw`): a ribbon of shape sprites laid
//! along the ring of positions a tail object records
//! (`docs/formats/vfx.md`, "Tail draws").

/// The functions over game memory.
pub mod bridge {
    use crate::draw::bridge::{ENV_SLOT, GET_PTR32, g, instance};
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{add, div, load, mul, sqrt, store, sub};

    const SUB_VECTOR: u32 = 0x006e_d940;
    const DIV_VECTOR: u32 = 0x006e_d3a0;
    const COPY_MATRIX: u32 = 0x006e_d360;
    const UNIT_MATRIX: u32 = 0x006e_d9a0;
    const COPY_VECTOR: u32 = 0x006e_d370;
    const VEC_LEN: u32 = 0x0075_fda0;
    const GET_RENDER_DATA: u32 = 0x0064_53d0;
    const ALPHA_FUNCTION: u32 = 0x0071_2c30;
    const INIT_SHAPE_FIELD: u32 = 0x0071_13d0;
    const CLEAR_SHP: u32 = 0x0071_3290;
    const SHAPE_FIELD_SIZE: u32 = 0x0071_2bd0;
    const DYN_GEO_BUFFER: u32 = 0x0063_fdb0;
    const UPDATE_SHAPE_FIELD: u32 = 0x0071_a4e0;
    const VFX_UPDATE: u32 = 0x0064_58a0;
    const USE_OPTIMIZED_SHADER: u32 = 0x0064_4b10;
    /// `g_pppViewMatrix`.
    const VIEW_MATRIX: u32 = 0x0230_ff20;
    /// `g_PppPacketPos`.
    const PACKET_POS: u32 = 0x0230_fd2c;
    /// `g_pppCurrentPart`.
    const CURRENT_PART: u32 = 0x0230_ffe4;
    /// The `f64` 2^-14 the `2` draws scale alphas by.
    const ALPHA_SCALE: u32 = 0x00b5_1550;
    /// `g_Vu0_VfMove`.
    const VF_MOVE: u32 = 0x00c8_f980;

    /// The locals every tail draw keeps at the same place below its
    /// aligned frame pointer.
    const COLOUR: u32 = 0x54;
    const POS: u32 = 0x14;
    const A: u32 = 0x44;
    const B: u32 = 0x34;
    const D: u32 = 0x24;
    const REQ: u32 = 0x120;

    /// One tail draw's differences: where its locals are, the copies of
    /// the vector helpers next to it, and whether it is an `X` draw.
    #[derive(Debug, Clone, Copy)]
    pub struct Tail {
        /// The colour step vector.
        pub step: u32,
        /// The copy of the object's matrix.
        pub copy: u32,
        /// The request's matrix (row 3 the sprite's view position).
        pub matrix: u32,
        /// `pppFtoiRgba_*`.
        pub ftoi: u32,
        /// `vu_apply_matrix_*`.
        pub apply: u32,
        /// `vu_vec4_sub_*`.
        pub vsub: u32,
        /// `vu_vec4_add_*`.
        pub vadd: u32,
        /// The `X` draws: the key byte whose flag gives request flag
        /// 0x800, the next 0x400 and the one after the second matrix (work
        /// area 3); the part's scales are applied per sprite.
        pub x: Option<u32>,
        /// Without render data, frame 0's data instead of the frame's
        /// (`pppKeShpTailXDraw`).
        pub frame0: bool,
        /// The `2` draws: colours from key bytes, the scale from key `+0xc`
        /// to `+0x10`, the first sprite only with key byte `+0x22`.
        pub two: bool,
        /// Zero divisors give 0 and the buffer counts are tripled
        /// (`pppKeShpTail2Draw`).
        pub guarded: bool,
    }

    /// `pppKeShpTailDraw` (`0x0074c570`).
    pub const TAIL: Tail = Tail {
        step: 0xe8,
        copy: 0x98,
        matrix: 0xd8,
        ftoi: 0x0074_c280,
        apply: 0x0074_bff0,
        vsub: 0x0074_c3b0,
        vadd: 0x0074_bf10,
        x: None,
        frame0: false,
        two: false,
        guarded: false,
    };

    /// `pppKeShpTailXDraw` (`0x0074d570`).
    pub const TAIL_X: Tail = Tail {
        step: 0xa8,
        copy: 0xe8,
        matrix: 0x98,
        ftoi: 0x0074_d280,
        apply: 0x0074_cff0,
        vsub: 0x0074_d3b0,
        vadd: 0x0074_cef0,
        x: Some(0x22),
        frame0: true,
        two: false,
        guarded: false,
    };

    /// `pppKeShpTail2Draw` (`0x0074e570`).
    pub const TAIL2: Tail = Tail {
        step: 0xe8,
        copy: 0x98,
        matrix: 0xd8,
        ftoi: 0x0074_e280,
        apply: 0x0074_dff0,
        vsub: 0x0074_e3b0,
        vadd: 0x0074_def0,
        x: None,
        frame0: false,
        two: true,
        guarded: true,
    };

    /// `pppKeShpTail2XDraw` (`0x0074f5b0`).
    pub const TAIL2_X: Tail = Tail {
        step: 0xa8,
        copy: 0xe8,
        matrix: 0x98,
        ftoi: 0x0074_f2c0,
        apply: 0x0074_f030,
        vsub: 0x0074_f3f0,
        vadd: 0x0074_ef50,
        x: Some(0x23),
        frame0: false,
        two: true,
        guarded: false,
    };

    /// Four `f32` of `(i16 src[k] * u16 part[+0x70 + 2k]) >> 19` at `dst`
    /// (`imul`, `sar`, `fild`).
    fn scaled_colour(mem: &mut impl Mem, dst: u32, src: u32, part: u32) {
        for k in 0..4 {
            let c = i32::from(mem.read::<i16>(src.wrapping_add(2 * k)));
            let s = i32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * k)));
            let v = c.wrapping_mul(s) >> 19;
            mem.write(dst.wrapping_add(4 * k), store(f64::from(v)));
        }
    }

    /// The `2` draws' colour: three `f32` of `(u8 src[k] * u16
    /// part[+0x70 + 2k]) >> 12` at `dst`, the fourth that times `alpha`.
    fn byte_colour(mem: &mut impl Mem, dst: u32, src: u32, part: u32, alpha: f64) {
        for k in 0..4 {
            let c = i32::from(mem.read::<u8>(src.wrapping_add(k)));
            let s = i32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * k)));
            let v = store(f64::from(c.wrapping_mul(s) >> 12));
            let v = if k == 3 {
                store(mul(alpha, load(v)))
            } else {
                v
            };
            mem.write(dst.wrapping_add(4 * k), v);
        }
    }

    /// `acc < left` on two `f32` the way the draws' `fcomp` tests it: a
    /// NaN is short too.
    fn short_of(acc: u32, left: u32) -> bool {
        use std::cmp::Ordering::{Equal, Greater};
        !matches!(load(acc).partial_cmp(&load(left)), Some(Greater | Equal))
    }

    /// `x <= 0` on an `f32` the way the draws' `fcomp` tests it: a NaN is
    /// not.
    fn not_positive(x: u32) -> bool {
        load(x) <= 0.0
    }

    /// `pppKeShpTailDraw(obj, key, ctrl)` (`0x0074c570`),
    /// `pppKeShpTailXDraw` (`0x0074d570`), `pppKeShpTail2Draw`
    /// (`0x0074e570`) and `pppKeShpTail2XDraw` (`0x0074f5b0`) by [`Tail`],
    /// cdecl; `frame` the original's
    /// aligned frame pointer. Work areas `w` (0: ring size byte `+0`, head
    /// byte `+1`, frame time `u16 +2`, frame `u16 +4`, the ring of 16-byte
    /// positions from `+0x10`) and `c` (1: four `i16` of colour) at `obj
    /// +0xa0 + [[ctrl +0xc] + 4k]`. Unless the key's shape (`+4`) is
    /// `0xffff`, `n` = key `u16 +0x20` sprites, from the work colour to the
    /// key's (`+0x10`, both through the part's `+0x70` halves `>> 19`) in
    /// `n - 1` steps, scaled from 1 down to key `+0x18`, spaced key `+0x1c`
    /// along the ring (backwards from the head; `n` = 0 when that is 0)
    /// until the length is used up or the ring comes back to the head.
    /// The `2` draws take their colours from key bytes `+0x14` and `+0x18`
    /// (`>> 12`; the alphas times work area 1's `i16 +6` / 2^14), their
    /// scale from key `+0xc` to `+0x10`, the length's step as the
    /// spacing times the scale's step over the start, and draw the first
    /// sprite only with key byte `+0x22`.
    /// Each is a shape-field request (see [`Tail`] for the `X` draws) with
    /// the object's matrix rows 0 and 1 scaled by the part's `+0x60`,
    /// `+0x64` and the sprite's scale, and the view position of its point.
    /// Last the shape's frame time steps by key `u16 +8` (unless
    /// `g_pppStopStatus`). Callees through `call`, `KeVu0VecLen` through
    /// `fcall`.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )]
    pub fn ppp_ke_shp_tail_draw<M: Mem>(
        mem: &mut M,
        v: &Tail,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let mut go = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let at = |o: u32| frame.wrapping_sub(o);
        let (colour, pos, a, b, d, req) = (at(COLOUR), at(POS), at(A), at(B), at(D), at(REQ));
        let (step, copy, matrix) = (at(v.step), at(v.copy), at(v.matrix));
        let part = rd(mem, g(mem, CURRENT_PART));
        if rd(mem, key.wrapping_add(4)) == 0xffff {
            return;
        }
        let mut n = u32::from(mem.read::<u16>(key.wrapping_add(0x20)));
        let n1 = store(f64::from(n.wrapping_sub(1).cast_signed()));
        let offs = rd(mem, ctrl.wrapping_add(0xc));
        let area = |mem: &M, k: u32| {
            obj.wrapping_add(0xa0)
                .wrapping_add(rd(mem, offs.wrapping_add(4 * k)))
        };
        let c = area(mem, 1);
        if v.two {
            // work area 1's `i16 +6` times 2^-14 scales the alphas
            let k: f64 = mem.read(g(mem, ALPHA_SCALE));
            let alpha = mul(f64::from(mem.read::<i16>(c.wrapping_add(6))), k);
            let alpha = load(store(alpha));
            byte_colour(mem, colour, key.wrapping_add(0x14), part, alpha);
            byte_colour(mem, pos, key.wrapping_add(0x18), part, alpha);
        } else {
            scaled_colour(mem, colour, c, part);
            scaled_colour(mem, pos, key.wrapping_add(0x10), part);
        }
        go(mem, SUB_VECTOR, &[a, colour, pos]);
        go(mem, DIV_VECTOR, &[step, a, n1]);
        let w = area(mem, 0);
        let env24 = |mem: &M| rd(mem, rd(mem, rd(mem, g(mem, ENV_SLOT))).wrapping_add(0x24));
        let slot = |mem: &M| env24(mem).wrapping_add(rd(mem, key.wrapping_add(4)).wrapping_mul(4));
        mem.write(req.wrapping_add(0x10), matrix);
        let mut flags = 0x80u16;
        if let Some(x) = v.x {
            if mem.read::<u8>(key.wrapping_add(x)) != 0 {
                flags = 0x880;
            }
            if mem.read::<u8>(key.wrapping_add(x + 1)) != 0 {
                flags |= 0x400;
            }
        }
        mem.write(req.wrapping_add(2), flags);
        mem.write(req.wrapping_add(8), 0u32);
        mem.write(req.wrapping_add(0x18), 0u8);
        let second = if v
            .x
            .is_some_and(|x| mem.read::<u8>(key.wrapping_add(x + 2)) != 0)
        {
            area(mem, 3)
        } else {
            0
        };
        mem.write(req.wrapping_add(0x1c), second);
        mem.write(req.wrapping_add(0xc), 0u32);
        let s = slot(mem);
        let set = go(mem, GET_PTR32, &[rd(mem, s)]);
        let shape_frame = u32::from(mem.read::<u16>(w.wrapping_add(4)));
        let entry = set.wrapping_add(8 * shape_frame);
        let data_at = |mem: &M, e: u32| {
            set.wrapping_add(i32::from(mem.read::<i16>(e.wrapping_add(0x10))).cast_unsigned())
        };
        let mut data = data_at(mem, entry);
        go(mem, COPY_MATRIX, &[copy, obj.wrapping_add(0x10)]);
        if v.x.is_none() {
            // rows 0 and 1 by the part's scales, once
            for (row, scale) in [(0u32, 0x60u32), (0x10, 0x64)] {
                let k = load(rd(mem, part.wrapping_add(scale)));
                for e in 0..3 {
                    let at = copy.wrapping_add(row + 4 * e);
                    mem.write(at, store(mul(load(rd(mem, at)), k)));
                }
            }
        }
        go(mem, UNIT_MATRIX, &[matrix]);
        let render_data = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let [lo, hi] = instance(mem, obj);
            go(mem, GET_RENDER_DATA, &[lo, hi])
        };
        if render_data(mem, &mut go) == 0 {
            if v.frame0 {
                data = data_at(mem, set);
            }
            let byte = mem.read::<u8>(
                data.wrapping_add(rd(mem, data.wrapping_add(4)))
                    .wrapping_add(4),
            );
            let alpha = go(mem, ALPHA_FUNCTION, &[u32::from(byte)]);
            let count = u32::from(mem.read::<u16>(key.wrapping_add(0x20)));
            let s = slot(mem);
            let [lo, hi] = instance(mem, obj);
            go(mem, INIT_SHAPE_FIELD, &[lo, hi, s, count, alpha]);
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        // the sprite's scale and its step, the length left and its step
        let mut left = store(load(rd(mem, key.wrapping_add(0x1c))));
        let (mut scale, ds, dl) = if v.two {
            // from key +0xc to +0x10; the length's step by the scale's share
            let s0 = store(load(rd(mem, key.wrapping_add(0xc))));
            let ds = if v.guarded && load(n1) == 0.0 {
                0
            } else {
                let to = load(rd(mem, key.wrapping_add(0x10)));
                store(div(sub(load(s0), to), load(n1)))
            };
            let share = if v.guarded && load(s0) == 0.0 {
                0.0
            } else {
                div(load(ds), load(s0))
            };
            (s0, ds, store(mul(share, load(left))))
        } else {
            let to = load(rd(mem, key.wrapping_add(0x18)));
            let ds = store(div(sub(1.0, to), load(n1)));
            (1.0f32.to_bits(), ds, store(mul(load(left), load(ds))))
        };
        if load(left) == 0.0 {
            n = 0;
        }
        // the ring: the first segment from the head
        let head = u32::from(mem.read::<u8>(w.wrapping_add(1)));
        let last = u32::from(mem.read::<u8>(w)).wrapping_sub(1);
        let ring = w.wrapping_add(0x10);
        go(
            mem,
            COPY_VECTOR,
            &[pos, ring.wrapping_add(head.wrapping_mul(16))],
        );
        go(mem, COPY_VECTOR, &[a, pos]);
        let mut idx = if head == last {
            0
        } else {
            head.wrapping_add(1)
        };
        go(
            mem,
            COPY_VECTOR,
            &[b, ring.wrapping_add(idx.wrapping_mul(16))],
        );
        go(mem, SUB_VECTOR, &[d, b, a]);
        let mut along = left;
        let mut seg = store(fcall(mem, g(mem, VEC_LEN), &[d]));
        let mut acc = seg;
        // sizing: one per sprite there will be
        let mut count = n;
        let mut t = left;
        loop {
            if render_data(mem, &mut go) != 0 {
                let frame_no = u32::from(mem.read::<u16>(w.wrapping_add(4)));
                let s = slot(mem);
                let [lo, hi] = instance(mem, obj);
                go(mem, SHAPE_FIELD_SIZE, &[lo, hi, s, frame_no]);
            }
            count = count.wrapping_sub(1);
            if count == 0 {
                break;
            }
            t = store(sub(load(t), load(dl)));
            if not_positive(t) {
                break;
            }
        }
        if render_data(mem, &mut go) != 0 {
            let data = render_data(mem, &mut go);
            let parts = rd(mem, data.wrapping_add(0x94));
            let (mut vertices, mut indices) = (
                rd(mem, parts.wrapping_add(4)),
                rd(mem, parts.wrapping_add(8)),
            );
            if v.guarded {
                vertices = vertices.wrapping_mul(3);
                indices = indices.wrapping_mul(3);
            }
            if go(mem, DYN_GEO_BUFFER, &[data, 0, vertices, indices]) == 0 {
                return;
            }
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        let view = g(mem, VIEW_MATRIX);
        // the `2` draws start with the first step unless key byte +0x22
        let mut drawing = !v.two || mem.read::<u8>(key.wrapping_add(0x22)) != 0;
        'sprites: loop {
            if drawing {
                go(mem, v.ftoi, &[req.wrapping_add(4), colour]);
                if v.x.is_some() {
                    // the part's scale times the sprite's, rounded, per row
                    for (row, at_scale) in [(0u32, 0x60u32), (0x10, 0x64)] {
                        let p = load(rd(mem, part.wrapping_add(at_scale)));
                        let k = load(store(mul(p, load(scale))));
                        for e in 0..3 {
                            let m = load(rd(mem, copy.wrapping_add(row + 4 * e)));
                            mem.write(matrix.wrapping_add(row + 4 * e), store(mul(m, k)));
                        }
                    }
                } else {
                    for row in [0u32, 0x10] {
                        for e in 0..3 {
                            let m = load(rd(mem, copy.wrapping_add(row + 4 * e)));
                            mem.write(matrix.wrapping_add(row + 4 * e), store(mul(m, load(scale))));
                        }
                    }
                }
                go(mem, v.apply, &[matrix.wrapping_add(0x30), view, pos]);
                let packet = rd(mem, g(mem, PACKET_POS));
                mem.write(req.wrapping_add(0x14), packet);
                if render_data(mem, &mut go) != 0 {
                    let [lo, hi] = instance(mem, obj);
                    go(mem, UPDATE_SHAPE_FIELD, &[lo, hi, data, req]);
                }
            }
            drawing = true;
            n = n.wrapping_sub(1);
            if n == 0 {
                break;
            }
            go(mem, v.vsub, &[colour, colour, step]);
            scale = store(sub(load(scale), load(ds)));
            left = store(sub(load(left), load(dl)));
            if not_positive(left) {
                break;
            }
            // on along the ring until the segment holds the next point
            while short_of(acc, left) {
                let prev = idx;
                idx = if prev == last {
                    0
                } else {
                    prev.wrapping_add(1)
                };
                if idx == head {
                    break 'sprites;
                }
                let next: [u32; 4] = mem.read(ring.wrapping_add(idx.wrapping_mul(16)));
                let old: [u32; 4] = mem.read(b);
                mem.write(a, old);
                mem.write(g(mem, VF_MOVE), next);
                mem.write(b, next);
                go(mem, v.vsub, &[d, b, a]);
                along = store(sub(load(along), load(seg)));
                let dv = |mem: &M, k: u32| load(rd(mem, d.wrapping_add(4 * k)));
                let (x, y, z) = (dv(mem, 0), dv(mem, 1), dv(mem, 2));
                let sum = store(add(add(mul(x, x), mul(y, y)), mul(z, z)));
                seg = store(sqrt(load(sum)));
                acc = store(add(load(acc), load(seg)));
            }
            let r = load(store(div(load(along), load(seg))));
            for k in 0..4 {
                let dk = load(rd(mem, d.wrapping_add(4 * k)));
                mem.write(pos.wrapping_add(4 * k), store(mul(dk, r)));
            }
            go(mem, v.vadd, &[pos, a, pos]);
            along = store(add(load(left), load(along)));
            acc = store(sub(load(acc), load(left)));
        }
        if render_data(mem, &mut go) != 0 {
            let [lo, hi] = instance(mem, obj);
            go(mem, VFX_UPDATE, &[lo, hi]);
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, USE_OPTIMIZED_SHADER, &[lo, hi]);
        step_frame(mem, w, key, set, entry);
    }

    /// The shape's frame time steps by key `u16 +8` unless
    /// `g_pppStopStatus`: at the frame's length (`i16 entry +0x12`) the
    /// next frame, and past the last (`i16 set +6`) frame 0 when the
    /// frame's byte `+0x14` has bit 7 (time and frame both cleared), else
    /// the last frame held.
    fn step_frame(mem: &mut impl Mem, w: u32, key: u32, set: u32, entry: u32) {
        if crate::ppp::bridge::STOP.get(mem) != 0 {
            return;
        }
        let time_at = w.wrapping_add(2);
        let frame_at = w.wrapping_add(4);
        let time = mem
            .read::<u16>(time_at)
            .wrapping_add(mem.read::<u16>(key.wrapping_add(8)));
        mem.write(time_at, time);
        let length = mem.read::<u16>(entry.wrapping_add(0x12));
        if i32::from(time) < i32::from(length.cast_signed()) {
            return;
        }
        let next = mem.read::<u16>(frame_at).wrapping_add(1);
        mem.write(frame_at, next);
        mem.write(time_at, time.wrapping_sub(length));
        let frames = mem.read::<i16>(set.wrapping_add(6));
        if i32::from(next) < i32::from(frames) {
            return;
        }
        if mem.read::<u8>(entry.wrapping_add(0x14)) & 0x80 != 0 {
            mem.write(time_at, 0u32);
        } else {
            mem.write(time_at, 0u16);
            mem.write(frame_at, next.wrapping_sub(1));
        }
    }

    // ---- the `Tail3` draws: a 28-entry ring, random frames and sizes ----

    const SPR_ALLOC: u32 = 0x007e_6a00;
    const SPR_FREE: u32 = 0x007e_6a60;
    const RESTORE_VF00: u32 = 0x0042_ef00;
    const SHP_ALPHA: u32 = 0x0071_6550;
    /// `g_GraphicSkipFrame`.
    const SKIP_FRAME: u32 = 0x012f_b7d0;
    /// `g_pppSinTable`: 4096 `f32` sines over a turn.
    const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// `g_Vu0_Vf00`'s `x` and `w`.
    const VF00_X: u32 = 0x00c0_a004;
    const VF00_W: u32 = 0x00c0_a010;
    /// `g_Vu0_Q`.
    const VU0_Q: u32 = 0x00c8_f788;
    /// The `f64` 2^-19 the plain draw scales colours by.
    const COLOUR_SCALE: u32 = 0x00b5_1650;
    /// The `f64` 65535 the random size divides by.
    const RAND_RANGE: u32 = 0x00b1_70d8;
    /// The ring's entries.
    const RING: u32 = 28;
    /// The block `op_spr_alloc` hands out: the view matrix at `+0`, two
    /// scaled rows at `+0x80`, the sprite's matrix at `+0xc0`.
    const SPR_SIZE: u32 = 0x180;

    /// The emulated VU0 registers the `Tail3` draws use, by address
    /// (`g_Vu0Acc` 0x00c8f790 and the 16-byte registers after it).
    mod vf {
        /// `g_Vu0Acc`: the accumulator.
        pub const ACC: u32 = 0x00c8_f790;
        /// The sprite's matrix: rows 0, 1, 2 and the position.
        pub const ROW0: u32 = 0x00c8_f7b0;
        pub const ROW1: u32 = 0x00c8_f7c0;
        pub const POS: u32 = 0x00c8_f7e0;
        /// The part's scale rows (`pppKeShpTail3VfLoad` from the block's
        /// `+0x80`).
        pub const SCALE0: u32 = 0x00c8_f7f0;
        pub const SCALE1: u32 = 0x00c8_f800;
        /// The view matrix (from the block's `+0`).
        pub const VIEW: u32 = 0x00c8_f830;
        /// Sine and cosine of the sprite's turn, the two words after them
        /// and the saved row 0.
        pub const TURN: u32 = 0x00c8_f8f0;
        pub const SAVED: u32 = 0x00c8_f900;
        /// The segment's end `B`, the segment `D`, its start `A`, the
        /// point `P`, the object's row 0 copy.
        pub const B: u32 = 0x00c8_f930;
        pub const D: u32 = 0x00c8_f940;
        pub const A: u32 = 0x00c8_f950;
        pub const P: u32 = 0x00c8_f960;
        pub const T: u32 = 0x00c8_f970;
        /// `g_Vu0_VfMove`.
        pub const MOVE: u32 = 0x00c8_f980;
    }

    /// One `Tail3` draw's differences: its locals (offsets below the
    /// aligned frame pointer), the copies of the helpers next to it, and
    /// whether it is the `X` draw.
    #[derive(Debug, Clone, Copy)]
    pub struct Tail3 {
        /// The first colour (the one drawn).
        pub c0: u32,
        /// The second colour, then the ring's head point.
        pub c1: u32,
        /// The colours' difference, then the segment's start.
        pub a: u32,
        /// The colour step.
        pub step: u32,
        /// The segment's end.
        pub b: u32,
        /// The segment.
        pub d: u32,
        /// The shape-field request.
        pub req: u32,
        /// The object's matrix copy and the unit matrix (written only).
        pub copy: u32,
        pub unit: u32,
        /// The slots the draw reads for `g_Vu0_VfMove`'s `y`, `z`, `w`
        /// and never writes before (stale stack words, see
        /// [`ppp_ke_shp_tail3_draw`]).
        pub stale: [u32; 3],
        /// `vu_vec4_sub_*`, `pppFtoiRgba_*`, `pppKeShpTail3*VfLoad_*`,
        /// `pppKeShpTail3*VfCopy_*`.
        pub vsub: u32,
        pub ftoi: u32,
        pub vf_load: u32,
        pub vf_copy: u32,
        /// `pppKeShpTail3XDraw`: no stop/skip gates but the `0xffff` shape
        /// one, colours `>> 19` as integers, request flags from key bytes
        /// `+0x50..+0x52`, the render data asked for at each use, every
        /// segment's buffer, `g_Vu0_VfMove` kept across the walk, and the
        /// registers reloaded after each sprite.
        pub x: bool,
    }

    /// `pppKeShpTail3Draw` (`0x00750620`).
    pub const TAIL3: Tail3 = Tail3 {
        c0: 0x44,
        c1: 0x24,
        a: 0x34,
        step: 0x74,
        b: 0x64,
        d: 0x54,
        req: 0x140,
        copy: 0xb8,
        unit: 0xf8,
        stale: [0x10, 0xc, 0x8],
        vsub: 0x0075_01d0,
        ftoi: 0x0075_0130,
        vf_load: 0x0075_1740,
        vf_copy: 0x0075_1680,
        x: false,
    };

    /// `pppKeShpTail3XDraw` (`0x00751fc0`).
    pub const TAIL3_X: Tail3 = Tail3 {
        c0: 0x34,
        c1: 0x14,
        a: 0x24,
        step: 0x64,
        b: 0x54,
        d: 0x44,
        req: 0x130,
        copy: 0xe8,
        unit: 0xa8,
        stale: [0x16c, 0x168, 0x164],
        vsub: 0x0075_19d0,
        ftoi: 0x0075_1930,
        vf_load: 0x0075_1ab0,
        vf_copy: 0x0075_3080,
        x: true,
    };

    /// An `f32` copied through the x87 (`fld`/`fstp`).
    fn copy_f32(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = store(load(mem.read(src)));
        mem.write(dst, v);
    }

    /// `x <= 0` on an `f32` as the `Tail3` loops test it (`fcom` with 0;
    /// a NaN goes on).
    fn at_most_zero(x: u32) -> bool {
        load(x) <= 0.0
    }

    /// The frame `((base + r) / len) % frames` of the shape set picks for
    /// random number `r` (unsigned, the divisors sign-extended). The
    /// original faults on a zero divisor; the port takes frame 0 there.
    fn random_frame(base: u32, r: u32, len: u32, frames: u32) -> u32 {
        base.wrapping_add(r)
            .checked_div(len)
            .and_then(|q| q.checked_rem(frames))
            .unwrap_or(0)
    }

    /// The `Tail3` random sequence: `r * 0x80d + 7`, 16 bits.
    fn next_random(r: u32) -> u32 {
        r.wrapping_mul(0x80d).wrapping_add(7) & 0xffff
    }

    /// The colours: four `f32` from work area 0's `i16` pairs times the
    /// part's `u16 +0x70..+0x76`, by 2^-19 (`f64` constant; the `X` draw
    /// shifts the integer `>> 19`); the fourth times `alpha` too.
    #[allow(clippy::many_single_char_names)]
    fn tail3_colour(mem: &mut impl Mem, x: bool, dst: u32, src: u32, part: u32, alpha: f64) {
        let k: f64 = mem.read(g(mem, COLOUR_SCALE));
        for i in 0..4 {
            let c = i32::from(mem.read::<i16>(src.wrapping_add(2 * i)));
            let s = i32::from(mem.read::<u16>(part.wrapping_add(0x70 + 2 * i)));
            let prod = c.wrapping_mul(s);
            let v = if x {
                let v = load(store(f64::from(prod >> 19)));
                if i == 3 { mul(alpha, v) } else { v }
            } else {
                let v = mul(load(store(f64::from(prod))), k);
                if i == 3 { mul(v, alpha) } else { v }
            };
            mem.write(dst.wrapping_add(4 * i), store(v));
        }
    }

    /// `pppKeShpTail3Draw(obj, key, ctrl)` (`0x00750620`) and
    /// `pppKeShpTail3XDraw` (`0x00751fc0`) by [`Tail3`], cdecl; `frame`
    /// the original's aligned frame pointer.
    ///
    /// Work area 0 (`obj + 0xa0 + [[ctrl +0xc]]`) holds two colours (four
    /// `i16` each), a ring of 28 16-byte points from `+0x30` (head byte
    /// `+0x1f8`), a frame time `i32 +0x1f0` and a random seed `u16
    /// +0x1f6`; work area 1's `i16 +6` / 2^14 scales the alphas. `n` = key
    /// byte `+0x1c` sprites go from the first colour to the second in
    /// `n - 1` steps, sized from key `+0xc` down to `+0x10` and spaced
    /// along the ring (backwards from the head) by key `+0x18` times the
    /// size's share; `n` = 0 when that spacing is 0. Each sprite's size
    /// is also times `1 - r / 65535 * key +0x14` and its shape frame
    /// `((time + r) / len) % frames` for the next random `r` of the
    /// seed's sequence; with key `i16 +0x1e` it is turned by `r` too. The
    /// sprite's matrix is built in the emulated VU0 registers
    /// (`0x00c8f790..`) and copied to the `op_spr_alloc` block's `+0xc0`;
    /// key byte `+0x1d` draws the first sprite at the head. Last the work
    /// area's time steps by key `+8` (unless `g_pppStopStatus`).
    ///
    /// The draws put `g_Vu0_VfMove`'s `y`, `z`, `w` (and `0x00c8f8f8`,
    /// `0x00c8f8fc`) from stack words they never write first ([`Tail3`]'s
    /// `stale`; the plain draw's `y` is the last sprite's cosine once one
    /// was turned, the `X` draw's three are kept from the walk along the
    /// ring). The port reads the same slots of `frame`; nothing reads
    /// those registers before writing them again.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn ppp_ke_shp_tail3_draw<M: Mem>(
        mem: &mut M,
        v: &Tail3,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let mut go = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let at = |o: u32| frame.wrapping_sub(o);
        let (c0, c1, a, step, b, d, req) = (
            at(v.c0),
            at(v.c1),
            at(v.a),
            at(v.step),
            at(v.b),
            at(v.d),
            at(v.req),
        );
        let ga = |mem: &M, a: u32| g(mem, a);
        let render_data = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let [lo, hi] = instance(mem, obj);
            go(mem, GET_RENDER_DATA, &[lo, hi])
        };
        let part = rd(mem, ga(mem, CURRENT_PART));
        let mut data_rd = 0;
        if v.x {
            if rd(mem, key.wrapping_add(4)) == 0xffff {
                return;
            }
        } else {
            data_rd = render_data(mem, &mut go);
            if crate::ppp::bridge::STOP.get(mem) != 0 {
                return;
            }
        }
        let offs = rd(mem, ctrl.wrapping_add(0xc));
        let area = |mem: &M, k: u32| {
            obj.wrapping_add(0xa0)
                .wrapping_add(rd(mem, offs.wrapping_add(4 * k)))
        };
        let w = area(mem, 0);
        let time_at = w.wrapping_add(0x1f0);
        if !v.x && rd(mem, ga(mem, SKIP_FRAME)) != 0 {
            let t = rd(mem, time_at).wrapping_add(rd(mem, key.wrapping_add(8)));
            mem.write(time_at, t);
            return;
        }
        let spr = go(mem, SPR_ALLOC, &[SPR_SIZE]);
        let count = u32::from(mem.read::<u8>(key.wrapping_add(0x1c)));
        let mut n = count;
        let n1 = store(f64::from(count.wrapping_sub(1).cast_signed()));
        // the alphas' scale, then the two colours
        let k14: f64 = mem.read(ga(mem, ALPHA_SCALE));
        let a1 = f64::from(mem.read::<i16>(area(mem, 1).wrapping_add(6)));
        let alpha = load(store(mul(a1, k14)));
        tail3_colour(mem, v.x, c0, w, part, alpha);
        tail3_colour(mem, v.x, c1, w.wrapping_add(8), part, alpha);
        go(mem, SUB_VECTOR, &[a, c0, c1]);
        go(mem, DIV_VECTOR, &[step, a, n1]);
        // the request
        mem.write(req.wrapping_add(0xc), 0u32);
        mem.write(req.wrapping_add(8), 0u32);
        mem.write(req.wrapping_add(0x18), 0u8);
        mem.write(req.wrapping_add(0x10), spr.wrapping_add(0xc0));
        let mut flags = 0x80u16;
        let mut second = 0;
        if v.x {
            if mem.read::<u8>(key.wrapping_add(0x50)) != 0 {
                flags = 0x880;
            }
            if mem.read::<u8>(key.wrapping_add(0x51)) != 0 {
                flags |= 0x400;
            }
            if mem.read::<u8>(key.wrapping_add(0x52)) != 0 {
                second = area(mem, 3);
            }
        }
        mem.write(req.wrapping_add(2), flags);
        mem.write(req.wrapping_add(0x1c), second);
        let env24 = |mem: &M| rd(mem, rd(mem, rd(mem, ga(mem, ENV_SLOT))).wrapping_add(0x24));
        let slot = |mem: &M| env24(mem).wrapping_add(rd(mem, key.wrapping_add(4)).wrapping_mul(4));
        let s = slot(mem);
        let set = go(mem, GET_PTR32, &[rd(mem, s)]);
        go(mem, COPY_MATRIX, &[at(v.copy), obj.wrapping_add(0x10)]);
        go(mem, UNIT_MATRIX, &[at(v.unit)]);
        if v.x {
            data_rd = render_data(mem, &mut go);
        }
        if data_rd == 0 {
            let data0 = set
                .wrapping_add(i32::from(mem.read::<i16>(set.wrapping_add(0x10))).cast_unsigned());
            let byte = if v.x {
                go(mem, SHP_ALPHA, &[data0])
            } else {
                u32::from(
                    mem.read::<u8>(
                        data0
                            .wrapping_add(rd(mem, data0.wrapping_add(4)))
                            .wrapping_add(4),
                    ),
                )
            };
            let alpha_fn = go(mem, ALPHA_FUNCTION, &[byte]);
            let s = slot(mem);
            let [lo, hi] = instance(mem, obj);
            go(mem, INIT_SHAPE_FIELD, &[lo, hi, s, count, alpha_fn]);
            if !v.x {
                data_rd = render_data(mem, &mut go);
            }
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        // the size and its step, the spacing and its step
        let mut size = store(load(rd(mem, key.wrapping_add(0xc))));
        let to = load(rd(mem, key.wrapping_add(0x10)));
        let ds = store(div(sub(load(size), to), load(n1)));
        let mut left = store(load(rd(mem, key.wrapping_add(0x18))));
        let dl = store(mul(load(left), div(load(ds), load(size))));
        if load(left) == 0.0 {
            n = 0;
        }
        // the ring from the head: the first segment
        let head = u32::from(mem.read::<u8>(w.wrapping_add(0x1f8)));
        let ring = w.wrapping_add(0x30);
        let point = |i: u32| ring.wrapping_add(i.wrapping_mul(16));
        let next = |i: u32| if i == RING - 1 { 0 } else { i.wrapping_add(1) };
        go(mem, COPY_VECTOR, &[c1, point(head)]);
        go(mem, COPY_VECTOR, &[a, c1]);
        let mut idx = next(head);
        go(mem, COPY_VECTOR, &[b, point(idx)]);
        go(mem, SUB_VECTOR, &[d, b, a]);
        let mut along = left;
        let mut seg = store(fcall(mem, ga(mem, VEC_LEN), &[d]));
        let mut acc = seg;
        let base = rd(mem, time_at);
        let seed = u32::from(mem.read::<u16>(w.wrapping_add(0x1f6)));
        let len = i32::from(mem.read::<i16>(set.wrapping_add(0x12))).cast_unsigned();
        let frames = i32::from(mem.read::<i16>(set.wrapping_add(6))).cast_unsigned();
        // the block: the view matrix, the object's rows 0 and 1 by the
        // part's scales
        let view: [u32; 16] = mem.read(ga(mem, VIEW_MATRIX));
        mem.write(spr, view);
        mem.write(spr.wrapping_add(0xcc), 0u32);
        mem.write(spr.wrapping_add(0xdc), 0u32);
        let r = |mem: &M, a: u32| ga(mem, a);
        let part_scale: [u32; 4] = mem.read(part.wrapping_add(0x60));
        mem.write(r(mem, vf::MOVE), part_scale);
        let row0: [u32; 4] = mem.read(obj.wrapping_add(0x10));
        mem.write(r(mem, vf::T), row0);
        let row1: [u32; 4] = mem.read(obj.wrapping_add(0x20));
        mem.write(r(mem, vf::P), row1);
        let lf = |mem: &M, a: u32| load(rd(mem, a));
        for (dst, src, k) in [(vf::A, vf::T, vf::MOVE), (vf::D, vf::P, vf::MOVE + 4)] {
            let kx = lf(mem, r(mem, k));
            for e in 0..3 {
                let x = lf(mem, r(mem, src + 4 * e));
                mem.write(r(mem, dst + 4 * e), store(mul(x, kx)));
            }
        }
        let rows: [u32; 4] = mem.read(r(mem, vf::A));
        mem.write(spr.wrapping_add(0x80), rows);
        let rows: [u32; 4] = mem.read(r(mem, vf::D));
        mem.write(spr.wrapping_add(0x90), rows);
        go(mem, v.vf_load, &[spr]);
        // the registers: P the head point, A, D, B
        for (reg, local) in [(vf::P, c1), (vf::A, a), (vf::D, d), (vf::B, b)] {
            let x: [u32; 4] = mem.read(local);
            mem.write(r(mem, reg), x);
        }
        // sizing: one per sprite there will be
        let slot_frame = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, f: u32| {
            let s = slot(mem);
            let [lo, hi] = instance(mem, obj);
            go(mem, SHAPE_FIELD_SIZE, &[lo, hi, s, f]);
        };
        let mut sizing = n;
        let mut t = left;
        let mut rnd = seed;
        loop {
            rnd = next_random(rnd);
            let has = if v.x {
                render_data(mem, &mut go)
            } else {
                data_rd
            };
            if has != 0 {
                slot_frame(mem, &mut go, random_frame(base, rnd, len, frames));
            }
            sizing = sizing.wrapping_sub(1);
            if sizing == 0 {
                break;
            }
            t = store(sub(load(t), load(dl)));
            if at_most_zero(t) {
                break;
            }
        }
        // the geometry buffers
        if v.x {
            if render_data(mem, &mut go) != 0 {
                let data = render_data(mem, &mut go);
                let mut i = 0u32;
                while i.cast_signed() < rd(mem, data.wrapping_add(0x90)).cast_signed() {
                    let parts = rd(mem, data.wrapping_add(0x94)).wrapping_add(i.wrapping_mul(0x6c));
                    let (vertices, indices) = (
                        rd(mem, parts.wrapping_add(4)),
                        rd(mem, parts.wrapping_add(8)),
                    );
                    if go(mem, DYN_GEO_BUFFER, &[data, i, vertices, indices]) == 0 {
                        return;
                    }
                    i = i.wrapping_add(1);
                }
            }
        } else if data_rd != 0 {
            let parts = rd(mem, data_rd.wrapping_add(0x94));
            let (vertices, indices) = (
                rd(mem, parts.wrapping_add(4)),
                rd(mem, parts.wrapping_add(8)),
            );
            if go(mem, DYN_GEO_BUFFER, &[data_rd, 0, vertices, indices]) == 0 {
                return;
            }
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        let [mut vy, mut vz, mut vw] = v.stale.map(|o| rd(mem, at(o)));
        let mut rnd = seed;
        let mut drawing = mem.read::<u8>(key.wrapping_add(0x1d)) != 0;
        'sprites: loop {
            if !drawing {
                n = n.wrapping_sub(1);
                if n == 0 {
                    break;
                }
                go(mem, v.vsub, &[c0, c0, step]);
                size = store(sub(load(size), load(ds)));
                left = store(sub(load(left), load(dl)));
                if at_most_zero(left) {
                    break;
                }
                // on along the ring until the segment holds the next point
                while short_of(acc, left) {
                    idx = next(idx);
                    if idx == head {
                        break 'sprites;
                    }
                    for e in 0..4 {
                        copy_f32(mem, r(mem, vf::A + 4 * e), r(mem, vf::B + 4 * e));
                    }
                    let p: [u32; 4] = mem.read(point(idx));
                    mem.write(r(mem, vf::B), p);
                    for e in 0..4 {
                        let x = sub(
                            lf(mem, r(mem, vf::B + 4 * e)),
                            lf(mem, r(mem, vf::A + 4 * e)),
                        );
                        mem.write(r(mem, vf::D + 4 * e), store(x));
                    }
                    go(mem, RESTORE_VF00, &[]);
                    along = store(sub(load(along), load(seg)));
                    for e in 0..3 {
                        let x = lf(mem, r(mem, vf::D + 4 * e));
                        mem.write(r(mem, vf::MOVE + 4 * e), store(mul(x, x)));
                    }
                    let yx = add(lf(mem, r(mem, vf::MOVE + 4)), lf(mem, r(mem, vf::MOVE)));
                    mem.write(r(mem, vf::T), store(yx));
                    let sum = store(add(lf(mem, r(mem, vf::T)), lf(mem, r(mem, vf::MOVE + 8))));
                    if !v.x {
                        mem.write(r(mem, vf::MOVE), sum);
                    }
                    let q = store(sqrt(load(store(load(sum).abs()))));
                    mem.write(r(mem, VU0_Q), q);
                    seg = store(add(lf(mem, r(mem, VF00_X)), load(q)));
                    if v.x {
                        mem.write(r(mem, vf::MOVE), seg);
                        vy = rd(mem, r(mem, vf::MOVE + 4));
                        vz = rd(mem, r(mem, vf::MOVE + 8));
                        vw = rd(mem, r(mem, vf::MOVE + 12));
                    }
                    acc = store(add(load(seg), load(acc)));
                }
                // the point along the segment
                let q = load(store(div(load(along), load(seg))));
                for e in 0..4 {
                    let x = store(mul(lf(mem, r(mem, vf::D + 4 * e)), q));
                    mem.write(r(mem, vf::P + 4 * e), x);
                }
                if v.x {
                    mem.write(r(mem, vf::MOVE), [store(q), vy, vz, vw]);
                }
                for e in 0..4 {
                    let x = add(
                        lf(mem, r(mem, vf::P + 4 * e)),
                        lf(mem, r(mem, vf::A + 4 * e)),
                    );
                    mem.write(r(mem, vf::P + 4 * e), store(x));
                }
                along = store(add(load(along), load(left)));
                acc = store(sub(load(acc), load(left)));
            }
            drawing = false;
            // the sprite: its frame, colour and size
            rnd = next_random(rnd);
            let f = random_frame(base, rnd, len, frames);
            let data = set.wrapping_add(
                i32::from(mem.read::<i16>(set.wrapping_add(0x10).wrapping_add(8 * f)))
                    .cast_unsigned(),
            );
            go(mem, v.ftoi, &[req.wrapping_add(4), c0]);
            let range: f64 = mem.read(ga(mem, RAND_RANGE));
            let share = load(store(div(load(store(f64::from(rnd))), range)));
            let k = load(rd(mem, key.wrapping_add(0x14)));
            let h = load(store(sub(1.0, mul(share, k))));
            let sz = store(mul(h, load(size)));
            mem.write(r(mem, vf::MOVE), [sz, vy, vz, vw]);
            let szf = load(sz);
            for (dst, src) in [(vf::ROW0, vf::SCALE0), (vf::ROW1, vf::SCALE1)] {
                for e in 0..3 {
                    let x = store(mul(lf(mem, r(mem, src + 4 * e)), szf));
                    mem.write(r(mem, dst + 4 * e), x);
                }
            }
            // the point through the view matrix
            for i in 0..3u32 {
                let pv = lf(mem, r(mem, vf::P + 4 * i));
                for e in 0..4u32 {
                    let m = lf(mem, r(mem, vf::VIEW + 0x10 * i + 4 * e));
                    let x = if i == 0 {
                        mul(m, pv)
                    } else {
                        add(mul(m, pv), lf(mem, r(mem, vf::ACC + 4 * e)))
                    };
                    mem.write(r(mem, vf::ACC + 4 * e), store(x));
                }
            }
            let one = lf(mem, r(mem, VF00_W));
            for e in 0..4 {
                let m = lf(mem, r(mem, vf::VIEW + 0x30 + 4 * e));
                let x = add(mul(m, one), lf(mem, r(mem, vf::ACC + 4 * e)));
                mem.write(r(mem, vf::POS + 4 * e), store(x));
            }
            if mem.read::<i16>(key.wrapping_add(0x1e)) != 0 {
                turn(mem, rnd, vz, vw);
                vy = rd(mem, r(mem, vf::TURN + 4));
            }
            let packet = rd(mem, ga(mem, PACKET_POS));
            mem.write(req.wrapping_add(0x14), packet);
            go(mem, v.vf_copy, &[spr]);
            let has = if v.x {
                render_data(mem, &mut go)
            } else {
                data_rd
            };
            if has != 0 {
                let [lo, hi] = instance(mem, obj);
                go(mem, UPDATE_SHAPE_FIELD, &[lo, hi, data, req]);
            }
            if v.x {
                go(mem, v.vf_load, &[spr]);
            }
        }
        let has = if v.x {
            render_data(mem, &mut go)
        } else {
            data_rd
        };
        if has != 0 {
            let [lo, hi] = instance(mem, obj);
            go(mem, VFX_UPDATE, &[lo, hi]);
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, USE_OPTIMIZED_SHADER, &[lo, hi]);
        go(mem, SPR_FREE, &[SPR_SIZE]);
        if crate::ppp::bridge::STOP.get(mem) == 0 {
            let t = rd(mem, time_at).wrapping_add(rd(mem, key.wrapping_add(8)));
            mem.write(time_at, t);
        }
    }

    /// The sprite's turn by random number `r` (`g_pppSinTable` at `r >> 4`
    /// and a quarter turn on): rows 0 and 1 of the sprite's matrix
    /// (`0x00c8f7b0`, `0x00c8f7c0`, all four words) rotated in their
    /// plane, through the accumulator; sine and cosine kept at
    /// `0x00c8f8f0`, the old row 0 at `0x00c8f900`, and `z`, `w` (the
    /// draw's `g_Vu0_VfMove` words) at `0x00c8f8f8`.
    #[allow(clippy::many_single_char_names)]
    fn turn<M: Mem>(mem: &mut M, rnd: u32, z: u32, w: u32) {
        let r = |mem: &M, a: u32| g(mem, a);
        let lf = |mem: &M, a: u32| load(mem.read::<u32>(a));
        let table = r(mem, SIN_TABLE);
        let sin: u32 = mem.read(table.wrapping_add(4 * ((rnd >> 4) & 0xfff)));
        let cos: u32 = mem.read(table.wrapping_add(4 * (((rnd + 0x4000) >> 4) & 0xfff)));
        let (s, c) = (load(sin), load(cos));
        mem.write(r(mem, vf::TURN), [sin, cos, z, w]);
        let mut row0 = [0f64; 4];
        let mut row1 = [0f64; 4];
        for e in 0..4u32 {
            row0[e as usize] = lf(mem, r(mem, vf::ROW0 + 4 * e));
            row1[e as usize] = lf(mem, r(mem, vf::ROW1 + 4 * e));
        }
        for (e, &x) in (0u32..).zip(&row0) {
            mem.write(r(mem, vf::SAVED + 4 * e), store(x));
        }
        // row 0 = row 0 * cos + row 1 * sin
        for (e, &x) in (0u32..).zip(&row0) {
            mem.write(r(mem, vf::ACC + 4 * e), store(mul(x, c)));
        }
        for (e, &y) in (0u32..).zip(&row1) {
            let x = add(mul(y, s), lf(mem, r(mem, vf::ACC + 4 * e)));
            mem.write(r(mem, vf::ROW0 + 4 * e), store(x));
        }
        // row 1 = row 1 * cos - old row 0 * sin
        for (e, &y) in (0u32..).zip(&row1) {
            mem.write(r(mem, vf::ACC + 4 * e), store(mul(y, c)));
        }
        for e in 0..4u32 {
            let saved = lf(mem, r(mem, vf::SAVED + 4 * e));
            let x = sub(lf(mem, r(mem, vf::ACC + 4 * e)), mul(s, saved));
            mem.write(r(mem, vf::ROW1 + 4 * e), store(x));
        }
    }

    // ---- `pppKeShpTailPht`: a 27-entry ring drawn as distortion sprites ----

    const SET_PTR32: u32 = 0x00a4_4700;
    const GET_DBUFF_DC: u32 = 0x0082_0530;
    const APPLY_MATRIX: u32 = 0x006e_d2e0;
    const DTT_PRE: u32 = 0x0076_8d30;
    const DTT_GIF: u32 = 0x0076_7790;
    const OT_ADD: u32 = 0x007e_c420;
    /// `g_pppScreenMatrix`.
    const SCREEN_MATRIX: u32 = 0x0230_fe60;
    /// The `f64` 1/128 scaling the `Pht` alpha.
    const PHT_ALPHA_SCALE: u32 = 0x00b1_70b8;
    /// The `Pht` copies of the vector helpers.
    const PHT_VSUB: u32 = 0x0075_3db0;
    const PHT_VADD: u32 = 0x0075_3230;
    const PHT_FTOI: u32 = 0x0075_35a0;
    const PHT_APPLY: u32 = 0x0075_3310;
    const PHT_MUL_MATRIX: u32 = 0x0075_3670;

    /// `pppKeShpTailPhtDraw(obj, key, ctrl)` (`0x00754300`), cdecl; `frame`
    /// the original's aligned frame pointer.
    ///
    /// As the `Tail3` draws, on a ring of 27 points (work area 0 `+0x30`,
    /// head byte `+0x1f4`, time `i32 +0x1ec`, seed `u16 +0x1f2`), count key
    /// byte `+0x60` (`+0x61` draws the first at the head, `+0x62` turns
    /// them), colours work area 0's `i16 >> 7` (the alphas times work
    /// area 1's fourth `>> 7` / 128). Each sprite is a shape-field request
    /// (matrix: the object's rows 0, 1 times the part's `+0x60`/`+0x64`,
    /// the size and the random share; turned by `vu_mul_matrix` with a
    /// rotation about z) and a distortion sprite: the record at `ebp -
    /// 0x160` (the point through the view matrix, work area 1's colour,
    /// `g_pppScreenMatrix` twice, the packet position `+ 0x10`, work area
    /// 0's `+0x1e0` up through work area 2's matrix with key byte `+0x64`,
    /// the frame buffer's TEX words from `Sg_GetDBuffDC` (half by key
    /// byte `+0x63`), the sizes key `+0x28`/`+0x2c` times the part's
    /// scales) for `KeDttSpr_MkGifPk_Pre` and `KeDttSpr_MkGifPk`; when
    /// that returns nonzero the packet goes into the ordering table
    /// (`op_ot_add`). Last the time steps by key `+8` (no stop check).
    ///
    /// The ordering-table depth is the record's `+0x2c` / 12288, a word
    /// the draw never writes (the PC `KeDttSpr_MkGifPk` returns 0, so
    /// this never runs); the port reads the same slot of `frame`.
    #[allow(
        clippy::too_many_lines,
        clippy::many_single_char_names,
        clippy::similar_names
    )]
    pub fn ppp_ke_shp_tail_pht_draw<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) {
        let rd = |mem: &M, a: u32| mem.read::<u32>(a);
        let lf = |mem: &M, a: u32| load(rd(mem, a));
        let mut go = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let at = |o: u32| frame.wrapping_sub(o);
        let (c0, c1, a, b, d, step, req) = (
            at(0x54),
            at(0x14),
            at(0x44),
            at(0x34),
            at(0x24),
            at(0xa8),
            at(0x190),
        );
        let (copy, sprite, view_out, rec, dc_out, turn_m) = (
            at(0xe8),
            at(0x98),
            at(0x68),
            at(0x160),
            at(0x200),
            at(0x240),
        );
        let render_data = |mem: &mut M, go: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let [lo, hi] = instance(mem, obj);
            go(mem, GET_RENDER_DATA, &[lo, hi])
        };
        let part = rd(mem, g(mem, CURRENT_PART));
        let offs = rd(mem, ctrl.wrapping_add(0xc));
        let area = |mem: &M, k: u32| {
            obj.wrapping_add(0xa0)
                .wrapping_add(rd(mem, offs.wrapping_add(4 * k)))
        };
        let area2 = area(mem, 2);
        if rd(mem, key.wrapping_add(4)) == 0xffff {
            return;
        }
        let (w, w1) = (area(mem, 0), area(mem, 1));
        let count = u32::from(mem.read::<u8>(key.wrapping_add(0x60)));
        let mut n = count;
        let n1 = store(f64::from(count.wrapping_sub(1).cast_signed()));
        let shifted = |mem: &M, at: u32| load(store(f64::from(mem.read::<i16>(at) >> 7)));
        for k in 0..4 {
            let v = shifted(mem, w1.wrapping_add(2 * k));
            mem.write(rec.wrapping_add(0x10 + 4 * k), store(v));
        }
        let k128: f64 = mem.read(g(mem, PHT_ALPHA_SCALE));
        let alpha = load(store(mul(lf(mem, rec.wrapping_add(0x1c)), k128)));
        for (dst, src) in [(c0, w), (c1, w.wrapping_add(8))] {
            for k in 0..4 {
                let v = shifted(mem, src.wrapping_add(2 * k));
                let v = if k == 3 { mul(alpha, v) } else { v };
                mem.write(dst.wrapping_add(4 * k), store(v));
            }
        }
        go(mem, SUB_VECTOR, &[a, c0, c1]);
        go(mem, DIV_VECTOR, &[step, a, n1]);
        mem.write(req.wrapping_add(0xc), 0u32);
        mem.write(req.wrapping_add(8), 0u32);
        mem.write(req.wrapping_add(0x18), 0u8);
        mem.write(req.wrapping_add(0x10), sprite);
        mem.write(req.wrapping_add(0x1c), 0u32);
        mem.write(req.wrapping_add(2), 0x80u16);
        let env24 = |mem: &M| rd(mem, rd(mem, rd(mem, g(mem, ENV_SLOT))).wrapping_add(0x24));
        let slot = |mem: &M| env24(mem).wrapping_add(rd(mem, key.wrapping_add(4)).wrapping_mul(4));
        let s = slot(mem);
        let set = go(mem, GET_PTR32, &[rd(mem, s)]);
        go(mem, COPY_MATRIX, &[copy, obj.wrapping_add(0x10)]);
        go(mem, UNIT_MATRIX, &[sprite]);
        let screen = g(mem, SCREEN_MATRIX);
        let p = go(mem, SET_PTR32, &[screen]);
        mem.write(rec.wrapping_add(0x28), p);
        let p = go(mem, SET_PTR32, &[screen]);
        mem.write(rec.wrapping_add(0x20), p);
        let k28 = store(mul(
            lf(mem, key.wrapping_add(0x28)),
            lf(mem, part.wrapping_add(0x60)),
        ));
        let k2c = store(mul(
            lf(mem, key.wrapping_add(0x2c)),
            lf(mem, part.wrapping_add(0x64)),
        ));
        // the frame buffer's TEX words, by the buffer half
        let dc = go(mem, GET_DBUFF_DC, &[dc_out]);
        let half =
            u32::from(mem.read::<u8>(key.wrapping_add(0x63))).wrapping_add(rd(mem, dc_out)) & 1;
        let fb = dc.wrapping_add(half * 0x130);
        let t = rd(mem, fb.wrapping_add(0x60)).cast_signed();
        let psm = u64::from((((t >> 24) & 0x3f) | 0x2a80).cast_unsigned());
        let fbw = u64::from(((t >> 16) & 0x3f).cast_unsigned());
        let tbp = u64::from(((t & 0x1ff) << 5).cast_unsigned());
        let tex = (((psm << 6) | fbw) << 14) | tbp;
        mem.write(rec.wrapping_add(0x40), tex);
        let w84 = i64::from(rd(mem, fb.wrapping_add(0x84)).cast_signed());
        mem.write(rec.wrapping_add(0x48), w84);
        let w78 = rd(mem, fb.wrapping_add(0x78));
        mem.write(rec.wrapping_add(0x50), w78);
        let w7c = rd(mem, fb.wrapping_add(0x7c));
        mem.write(rec.wrapping_add(0x54), w7c);
        // work area 0's +0x1e0 up, through work area 2's matrix
        let up = store(-lf(mem, w.wrapping_add(0x1e0)));
        mem.write(rec.wrapping_add(0x30), [0u32, up, 0, 0]);
        let v = rec.wrapping_add(0x30);
        go(mem, APPLY_MATRIX, &[v, area2, v]);
        let b64 = u32::from(mem.read::<u8>(key.wrapping_add(0x64)));
        mem.write(rec.wrapping_add(0x3c), b64);
        if render_data(mem, &mut go) == 0 {
            let data0 = set
                .wrapping_add(i32::from(mem.read::<i16>(set.wrapping_add(0x10))).cast_unsigned());
            let byte = mem.read::<u8>(
                data0
                    .wrapping_add(rd(mem, data0.wrapping_add(4)))
                    .wrapping_add(4),
            );
            let alpha_fn = go(mem, ALPHA_FUNCTION, &[u32::from(byte)]);
            let s = slot(mem);
            let [lo, hi] = instance(mem, obj);
            go(mem, INIT_SHAPE_FIELD, &[lo, hi, s, count, alpha_fn]);
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        let mut size = store(lf(mem, key.wrapping_add(0xc)));
        let to = lf(mem, key.wrapping_add(0x10));
        let ds = store(div(sub(load(size), to), load(n1)));
        let mut left = store(lf(mem, key.wrapping_add(0x18)));
        let dl = store(mul(load(left), div(load(ds), load(size))));
        if load(left) == 0.0 {
            n = 0;
        }
        // the ring of 27 from the head
        let head = u32::from(mem.read::<u8>(w.wrapping_add(0x1f4)));
        let ring = w.wrapping_add(0x30);
        let point = |i: u32| ring.wrapping_add(i.wrapping_mul(16));
        let next = |i: u32| if i == 26 { 0 } else { i.wrapping_add(1) };
        go(mem, COPY_VECTOR, &[c1, point(head)]);
        go(mem, COPY_VECTOR, &[a, c1]);
        let mut idx = next(head);
        go(mem, COPY_VECTOR, &[b, point(idx)]);
        go(mem, SUB_VECTOR, &[d, b, a]);
        let mut along = left;
        let mut seg = store(fcall(mem, g(mem, VEC_LEN), &[d]));
        let mut acc = seg;
        let time_at = w.wrapping_add(0x1ec);
        let base = rd(mem, time_at);
        let seed = u32::from(mem.read::<u16>(w.wrapping_add(0x1f2)));
        let len = i32::from(mem.read::<i16>(set.wrapping_add(0x12))).cast_unsigned();
        let frames = i32::from(mem.read::<i16>(set.wrapping_add(6))).cast_unsigned();
        // the turn: a rotation about z, its fixed words now
        let one = 1.0f32.to_bits();
        for (k, v) in [
            (2u32, 0u32),
            (3, 0),
            (6, 0),
            (7, 0),
            (8, 0),
            (9, 0),
            (10, one),
            (15, one),
            (11, 0),
            (12, 0),
            (13, 0),
            (14, 0),
        ] {
            mem.write(turn_m.wrapping_add(4 * k), v);
        }
        let mut sizing = n;
        let mut t = left;
        let mut rnd = seed;
        loop {
            rnd = next_random(rnd);
            if render_data(mem, &mut go) != 0 {
                let s = slot(mem);
                let [lo, hi] = instance(mem, obj);
                let f = random_frame(base, rnd, len, frames);
                go(mem, SHAPE_FIELD_SIZE, &[lo, hi, s, f]);
            }
            sizing = sizing.wrapping_sub(1);
            if sizing == 0 {
                break;
            }
            t = store(sub(load(t), load(dl)));
            if at_most_zero(t) {
                break;
            }
        }
        if render_data(mem, &mut go) != 0 {
            let data = render_data(mem, &mut go);
            let parts = rd(mem, data.wrapping_add(0x94));
            let (vertices, indices) = (
                rd(mem, parts.wrapping_add(4)),
                rd(mem, parts.wrapping_add(8)),
            );
            if go(mem, DYN_GEO_BUFFER, &[data, 0, vertices, indices]) == 0 {
                return;
            }
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, CLEAR_SHP, &[lo, hi]);
        let mut rnd = seed;
        let mut drawing = mem.read::<u8>(key.wrapping_add(0x61)) != 0;
        'sprites: loop {
            if !drawing {
                n = n.wrapping_sub(1);
                if n == 0 {
                    break;
                }
                go(mem, PHT_VSUB, &[c0, c0, step]);
                size = store(sub(load(size), load(ds)));
                left = store(sub(load(left), load(dl)));
                if at_most_zero(left) {
                    break;
                }
                while short_of(acc, left) {
                    idx = next(idx);
                    if idx == head {
                        break 'sprites;
                    }
                    let old: [u32; 4] = mem.read(b);
                    mem.write(a, old);
                    let p: [u32; 4] = mem.read(point(idx));
                    mem.write(g(mem, VF_MOVE), p);
                    mem.write(b, p);
                    go(mem, PHT_VSUB, &[d, b, a]);
                    along = store(sub(load(along), load(seg)));
                    let dv = |mem: &M, k: u32| lf(mem, d.wrapping_add(4 * k));
                    let (x, y, z) = (dv(mem, 0), dv(mem, 1), dv(mem, 2));
                    let sum = store(add(add(mul(x, x), mul(y, y)), mul(z, z)));
                    seg = store(sqrt(load(sum)));
                    acc = store(add(load(acc), load(seg)));
                }
                let r = load(store(div(load(along), load(seg))));
                for k in 0..4 {
                    let x = store(mul(lf(mem, d.wrapping_add(4 * k)), r));
                    mem.write(c1.wrapping_add(4 * k), x);
                }
                go(mem, PHT_VADD, &[c1, a, c1]);
                along = store(add(load(left), load(along)));
                acc = store(sub(load(acc), load(left)));
            }
            drawing = false;
            rnd = next_random(rnd);
            let f = random_frame(base, rnd, len, frames);
            let data = set.wrapping_add(
                i32::from(mem.read::<i16>(set.wrapping_add(0x10).wrapping_add(8 * f)))
                    .cast_unsigned(),
            );
            go(mem, PHT_FTOI, &[req.wrapping_add(4), c0]);
            let range: f64 = mem.read(g(mem, RAND_RANGE));
            let share = load(store(div(load(store(f64::from(rnd))), range)));
            let h = load(store(sub(1.0, mul(share, lf(mem, key.wrapping_add(0x14))))));
            // the sprite's matrix: rows 0 and 1 by the part's scales
            let sz = load(size);
            let k0 = load(store(mul(mul(lf(mem, part.wrapping_add(0x60)), sz), h)));
            for e in 0..3 {
                let x = store(mul(lf(mem, copy.wrapping_add(4 * e)), k0));
                mem.write(sprite.wrapping_add(4 * e), x);
            }
            let k1 = load(store(mul(mul(sz, lf(mem, part.wrapping_add(0x64))), h)));
            for e in 0..3 {
                let x = store(mul(k1, lf(mem, copy.wrapping_add(0x10 + 4 * e))));
                mem.write(sprite.wrapping_add(0x10 + 4 * e), x);
            }
            let view = g(mem, VIEW_MATRIX);
            go(mem, PHT_APPLY, &[view_out, view, c1]);
            let p: [u32; 4] = mem.read(view_out);
            mem.write(rec, p);
            if mem.read::<u8>(key.wrapping_add(0x62)) != 0 {
                let table = g(mem, SIN_TABLE);
                let sin: u32 = mem.read(table.wrapping_add(4 * ((rnd >> 4) & 0xfff)));
                let cos: u32 = mem.read(table.wrapping_add(4 * (((rnd + 0x4000) >> 4) & 0xfff)));
                let (sin, cos) = (store(load(sin)), store(load(cos)));
                mem.write(turn_m, [cos, sin]);
                mem.write(turn_m.wrapping_add(0x10), [store(-load(sin)), cos]);
                go(mem, PHT_MUL_MATRIX, &[sprite, sprite, turn_m]);
            }
            let packet = rd(mem, g(mem, PACKET_POS));
            mem.write(req.wrapping_add(0x14), packet);
            if render_data(mem, &mut go) != 0 {
                let [lo, hi] = instance(mem, obj);
                go(mem, UPDATE_SHAPE_FIELD, &[lo, hi, data, req]);
            }
            let packet = rd(mem, g(mem, PACKET_POS));
            let q = go(mem, SET_PTR32, &[packet.wrapping_add(0x10)]);
            mem.write(rec.wrapping_add(0x24), q);
            copy_f32(mem, rec.wrapping_add(0x1c), c0.wrapping_add(0xc));
            let s28 = store(mul(mul(load(k28), sz), h));
            let s2c = store(mul(mul(sz, load(k2c)), h));
            mem.write(rec.wrapping_add(0x58), [s28, s2c]);
            go(mem, DTT_PRE, &[rec]);
            if go(mem, DTT_GIF, &[rec]) != 0 {
                let depth = rd(mem, rec.wrapping_add(0x2c)).cast_signed() / 12288;
                let q = rd(mem, rec.wrapping_add(0x24));
                let end = go(mem, GET_PTR32, &[q]);
                let packet = rd(mem, g(mem, PACKET_POS));
                let words = end.wrapping_sub(packet).cast_signed() >> 4;
                go(
                    mem,
                    OT_ADD,
                    &[packet, words.cast_unsigned(), depth.cast_unsigned(), 0],
                );
                let q = rd(mem, rec.wrapping_add(0x24));
                let end = go(mem, GET_PTR32, &[q]);
                mem.write(g(mem, PACKET_POS), end);
            }
        }
        if render_data(mem, &mut go) != 0 {
            let [lo, hi] = instance(mem, obj);
            go(mem, VFX_UPDATE, &[lo, hi]);
        }
        let [lo, hi] = instance(mem, obj);
        go(mem, USE_OPTIMIZED_SHADER, &[lo, hi]);
        let t = rd(mem, time_at).wrapping_add(rd(mem, key.wrapping_add(8)));
        mem.write(time_at, t);
    }
}
