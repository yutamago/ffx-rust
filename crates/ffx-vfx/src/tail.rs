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
}
