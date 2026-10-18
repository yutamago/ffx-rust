//! The ability map's (sphere grid's) float-heavy draws: the grid marks and
//! the map of nodes, links and icons.

use ffx_bridge::{Global, Mem};

use crate::abmap::ABMAP;
use crate::x87;

/// The GS packet buffer both draws fill (`0x01a860ec`, a pointer).
pub const PACKET: Global<u32> = Global::new(0x01a8_60ec);
/// The emulated VU0 registers the inlined vector scalings go through:
/// `vf` at `0x00c8f7d0` (four `f32`s) and the scale at `0x00c8f7e0`.
pub const VU0_VECTOR: u32 = 0x00c8_f7d0;
/// `graphicDrawUIAbmapElement(vertices, scratch, n)` (`0x0063e930`).
pub const DRAW_ELEMENT: u32 = 0x0063_e930;

/// The scratch [`abmap_draw_grid_marks`] needs: the element's four
/// vertices (0x80 bytes) and the callee's 0x108-byte buffer.
pub const GRID_WORK: u32 = 0x200;

/// The 17 x 20 grid: a cell's mark byte at `g_Abmap + 0x12e60`, its
/// corner `(x, y, _, scale)` records (`i32, i32, _, f32`) from
/// `g_Abmap + 0x116c0`, 21 to a row.
const GRID_ROWS: u32 = 17;
const GRID_COLS: u32 = 20;
const MARKS: u32 = 0x12e60;
const CORNERS: u32 = 0x116c0;
const CORNER_ROW: u32 = 0x150;

/// `vec`'s `x, y, z` times `scale`, `w` kept, through the emulated VU0
/// registers (the original's inlined `rcbgScaleVector`): the scale is
/// stored to `0x00c8f7e0` first, the result left in `0x00c8f7d0`.
fn scale_vector<M: Mem>(mem: &mut M, vec: [u32; 4], scale: u32) -> [u32; 4] {
    let vf = mem.rebase(VU0_VECTOR);
    let s = x87::copy(scale);
    mem.write(vf.wrapping_add(0x10), s);
    let s = x87::load(s);
    let out = [
        x87::store(x87::load(vec[0]) * s),
        x87::store(x87::load(vec[1]) * s),
        x87::store(s * x87::load(vec[2])),
        vec[3],
    ];
    for (i, w) in (0u32..).zip(out) {
        mem.write(vf.wrapping_add(4 * i), w);
    }
    out
}

/// A screen coordinate from a grid integer: `v / 16 - 2048 + centre`
/// (the `f32` the original `fild`s it to, then at 53 bits).
fn screen(v: i32, centre: f64) -> u32 {
    let v = x87::load(x87::store(f64::from(v)));
    x87::store(v * 0.0625 - 2048.0 + centre)
}

/// `AbmapDrawGridMarks()` (`0x00a49f10`, name ours; from the ability map's
/// draw `0x00a54660`): for each set cell of the 17 x 20 grid a GS packet
/// at [`PACKET`] (two four-vertex strips: texture quads pulsing with the
/// frame `g_Abmap + 0x11698`, scaled per corner, colour 0x40, the corners'
/// positions), and the same two quads through `graphicDrawUIAbmapElement`
/// (texture coordinates times 512, colour 0x80). `work` is [`GRID_WORK`]
/// bytes. Returns the original's `eax` (its row pointer past the grid).
pub fn abmap_draw_grid_marks<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let g = ABMAP.get(mem);
    let at = |o: u32| g.wrapping_add(o);
    // the pulse: p runs -1..1 over 512 frames
    #[allow(clippy::cast_possible_wrap)] // 9 bits
    let frame = (mem.read::<u32>(at(0x11698)) & 0x1ff) as i32;
    let frame = x87::load(x87::store(f64::from(frame)));
    let p = x87::store(frame * 0.003_906_25 - 1.0);
    let (ld, st) = (x87::load, x87::store);
    let q = st(ld(p) + 2.0);
    let r = st(1.0 - ld(p));
    let r5 = st(ld(r) + 0.5);
    let s = st(ld(r) + 2.0);
    let u = st(ld(r5) + 2.0);
    let (one, zero) = (0x3f80_0000, 0);
    // the texture quads of the two strips, corner by corner
    let quads = [
        [
            [p, p, one, zero],
            [q, p, one, zero],
            [p, q, one, zero],
            [q, q, one, zero],
        ],
        [
            [r, r5, one, zero],
            [s, r5, one, zero],
            [r, u, one, zero],
            [s, u, one, zero],
        ],
    ];
    let (elem, scratch) = (work, work.wrapping_add(0x80));

    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            if mem.read::<u8>(at(MARKS + row * GRID_COLS + col)) == 0 {
                continue;
            }
            let c00 = at(CORNERS + row * CORNER_ROW + col * 0x10);
            let corners = [c00, c00 + 0x10, c00 + CORNER_ROW, c00 + CORNER_ROW + 0x10];
            let pkt = PACKET.get(mem);
            let w = |o: u32| pkt.wrapping_add(o);
            for h in 0..2u32 {
                let base = h * 0xd0;
                mem.write(w(base), 4 | h << 15);
                mem.write(w(base + 4), 0x302a_4000u32);
                mem.write(w(base + 8), 0x512u32);
                mem.write(w(base + 0xc), 0u32);
                for (k, &c) in (0u32..).zip(&corners) {
                    let v = base + 0x10 + k * 0x30;
                    mem.write(w(v + 0x20), mem.read::<u32>(c));
                    mem.write(w(v + 0x24), mem.read::<u32>(c + 4));
                    mem.write(w(v + 0x28), 0x2000u32);
                    mem.write(w(v + 0x2c), 0u32);
                }
            }
            for (h, strip) in (0u32..).zip(quads) {
                for (k, (&c, vec)) in (0u32..).zip(corners.iter().zip(strip)) {
                    let scale = mem.read::<u32>(c + 0xc);
                    let st = scale_vector(mem, vec, scale);
                    for (i, x) in (0u32..).zip(st) {
                        mem.write(w(h * 0xd0 + 0x10 + k * 0x30 + 4 * i), x);
                    }
                }
            }
            for h in 0..2u32 {
                for k in 0..4u32 {
                    for i in 0..4u32 {
                        mem.write(w(h * 0xd0 + 0x20 + k * 0x30 + 4 * i), 0x40u32);
                    }
                }
            }
            // the element: per vertex x, y, u, v and the colour 0x80
            for k in 0..4u32 {
                for i in 4..8u32 {
                    mem.write(elem + k * 0x20 + 4 * i, 0x80u32);
                }
            }
            for h in 0..2u32 {
                for k in 0..4u32 {
                    let v = w(h * 0xd0 + 0x10 + k * 0x30);
                    let e = elem + k * 0x20;
                    mem.write(e, screen(mem.read::<i32>(v + 0x20), 256.0));
                    mem.write(e + 4, screen(mem.read::<i32>(v + 0x24), 208.0));
                    mem.write(e + 8, st(ld(mem.read::<u32>(v)) * 512.0));
                    mem.write(e + 0xc, st(512.0 * ld(mem.read::<u32>(v + 4))));
                }
                let f = mem.rebase(DRAW_ELEMENT);
                call(mem, f, &[elem, scratch, 10]);
            }
        }
    }
    at(0x1182c + GRID_ROWS * CORNER_ROW)
}

/// The scratch [`abmap_draw_map`] needs: the node's local matrix (0x40
/// bytes) and the element's four vertices (0x80 bytes).
pub const MAP_WORK: u32 = 0xc0;

/// `AbmapMakeNodeMatrix(out, pos, scale)` (`0x00a5ad30`): a node's local
/// matrix.
pub const NODE_MATRIX: u32 = 0x00a5_ad30;
/// `Vec2SubScalarsFlags(dst, src, x, y)` (`0x0075e870`, the rest of the
/// vector by value): `dst.xy = src.xy - (x, y)`, the VU0 status flags at
/// [`CLIP_FLAGS`] (zero `0x40`/sign `0x80` among them).
pub const VU0_CLIP: u32 = 0x0075_e870;
/// The node kind templates (`0x01a85f78`, pointers by the node's `+6`).
pub const NODE_KINDS: u32 = 0x01a8_5f78;

/// The emulated VU0 registers [`abmap_draw_map`] leaves its work in, as
/// the original's inlined macro code does: `Q`, `ACC`, the vectors
/// `0x00c8f7d0`.. and the two matrices.
const VU_Q: u32 = 0x00c8_f788;
const VU_ACC: u32 = 0x00c8_f790;
const VU_E0: u32 = 0x00c8_f7e0;
const VU_F0: u32 = 0x00c8_f7f0;
const VU_800: u32 = 0x00c8_f800;
const VU_810: u32 = 0x00c8_f810;
const VU_830: u32 = 0x00c8_f830;
const VU_VIEW: u32 = 0x00c8_f910;
const VU_OUT: u32 = 0x00c8_f940;
const VU_LOCAL: u32 = 0x00c8_f950;
/// `vf00` (`0, 0, 0, 1`) of the emulated VU0.
const VF00: u32 = 0x00c0_a004;
/// The emulated EE registers the original's `qmtc2`-style loads pass
/// through (`0x00c8f588`, six words), the zero register and the clip flags.
const EE_TEMP: u32 = 0x00c8_f588;
const EE_ZERO: u32 = 0x00c8_f508;
/// The VU0 clip flags (`0x00c8fac4`).
pub const CLIP_FLAGS: u32 = 0x00c8_fac4;

fn read4<M: Mem>(mem: &M, a: u32) -> [u32; 4] {
    [0, 4, 8, 12].map(|o| mem.read::<u32>(a.wrapping_add(o)))
}

fn write4<M: Mem>(mem: &mut M, a: u32, v: [u32; 4]) {
    for (i, w) in (0u32..).zip(v) {
        mem.write(a.wrapping_add(4 * i), w);
    }
}

/// `v` through `m` as VU0 macro code computes it (`vmulax`, `vmadday`,
/// `vmaddaz`, `vmaddw`: each step rounded to `f32`): the result and the
/// accumulator before the last step.
fn vu_transform(m: [[u32; 4]; 4], v: [u32; 4]) -> ([u32; 4], [u32; 4]) {
    let (ld, st) = (x87::load, x87::store);
    let mut acc = [0u32; 4];
    let mut out = [0u32; 4];
    for i in 0..4 {
        let a = st(ld(m[0][i]) * ld(v[0]));
        let a = st(ld(a) + ld(m[1][i]) * ld(v[1]));
        acc[i] = st(ld(a) + ld(m[2][i]) * ld(v[2]));
        out[i] = st(ld(acc[i]) + ld(m[3][i]) * ld(v[3]));
    }
    (out, acc)
}

/// Two `i16`s into a VU0 vector the way the original moves them, through
/// the emulated EE registers (left as `a, b, sign a, sign b, b, sign b`):
/// `(a * scale, b * scale, vf00.z, vf00.w)`, also left in `0x00c8f7d0`.
fn load_pair<M: Mem>(mem: &mut M, a: i16, b: i16, scale: f64) -> [u32; 4] {
    let (a, b) = (i32::from(a), i32::from(b));
    let (sa, sb) = ((a >> 31).cast_unsigned(), (b >> 31).cast_unsigned());
    let ee = mem.rebase(EE_TEMP);
    for (i, w) in (0u32..).zip([
        a.cast_unsigned(),
        b.cast_unsigned(),
        sa,
        sb,
        b.cast_unsigned(),
        sb,
    ]) {
        mem.write(ee + 4 * i, w);
    }
    let vf00 = read4(mem, mem.rebase(VF00));
    let int = |v: i32| x87::load(x87::store(f64::from(v)));
    let v = [
        x87::store(int(a) * scale),
        x87::store(int(b) * scale),
        x87::copy(vf00[2]),
        x87::copy(vf00[3]),
    ];
    write4(mem, mem.rebase(VU0_VECTOR), v);
    v
}

/// `AbmapDrawMap()` (`0x00a4ce20`, name ours; from the ability map's draw
/// `0x00a54660`): each node of the map (`g_Abmap + 8`, `0x10` bytes, count
/// at `g_Abmap`): its local matrix (`AbmapMakeNodeMatrix`) times the view
/// (`g_Abmap + 0x113e0`); its kind's four corners (`i16` pairs at the kind
/// block `+0x20`, in sixteenths) through it, the node dropped when one
/// lands behind the eye; the corners projected and checked against the
/// screen box with `Vec2SubScalarsFlags` (the original's clip test; its
/// result only stops the checks early), then a GS packet at [`PACKET`] (the
/// texture quad from the block's `+0x30` pairs, in 4096ths, per corner
/// `q`, colour 0x40) and the element to `graphicDrawUIAbmapElement` (the
/// texture window by kind, colour 0x40, alpha 0x80). Works through the
/// emulated VU0 and EE registers as the original's inlined macro code does
/// and leaves them as it does. `work` is [`MAP_WORK`] bytes. Returns 0.
#[allow(clippy::too_many_lines)] // one pass per node, as the original
pub fn abmap_draw_map<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let (ld, st) = (x87::load, x87::store);
    let (local, elem) = (work, work.wrapping_add(0x40));
    let r = |mem: &M, a: u32| mem.rebase(a);
    let g = ABMAP.get(mem);
    let mut left = i32::from(mem.read::<i16>(g)).cast_unsigned();
    let mut node = g.wrapping_add(8);
    while left != 0 {
        left -= 1;
        let g = ABMAP.get(mem);
        let kinds = r(mem, NODE_KINDS) + 4 * u32::from(mem.read::<u16>(node + 6));
        let tmpl = mem.read::<u32>(kinds);
        let blk = tmpl.wrapping_add(i32::from(mem.read::<i16>(tmpl + 0x10)).cast_unsigned());
        let f = r(mem, NODE_MATRIX);
        call(mem, f, &[local, node, 0x3f80_0000]);
        let next = |node: u32| node.wrapping_add(0x10);

        // the node's matrix: local times view, row by row
        let view: [[u32; 4]; 4] = [0, 0x10, 0x20, 0x30].map(|o| read4(mem, g + 0x113e0 + o));
        let lm: [[u32; 4]; 4] = [0, 0x10, 0x20, 0x30].map(|o| read4(mem, local + o));
        for (i, row) in (0u32..).zip(view) {
            write4(mem, r(mem, VU_VIEW) + 0x10 * i, row);
        }
        for (i, row) in (0u32..).zip(lm) {
            write4(mem, r(mem, VU_LOCAL) + 0x10 * i, row);
        }
        let mut m = [[0u32; 4]; 4];
        for (i, row) in (0u32..).zip(lm) {
            // (the accumulator it leaves is overwritten by the corners')
            let (out, _) = vu_transform(view, row);
            write4(mem, r(mem, VU_LOCAL) + 0x10 * i, out);
            m[i as usize] = out;
        }

        // the four corners, dropped with the node when one is behind the eye
        let mut corners = [[0u32; 4]; 4];
        let mut behind = false;
        for (k, c) in (0u32..).zip(&mut corners) {
            let a = mem.read::<i16>(blk + 0x20 + 4 * k);
            let b = mem.read::<i16>(blk + 0x22 + 4 * k);
            let v = load_pair(mem, a, b, 0.0625);
            let (out, acc) = vu_transform(m, v);
            write4(mem, r(mem, VU_ACC), acc);
            write4(mem, r(mem, VU_OUT), out);
            *c = out;
            if ld(out[3]) <= 0.0 {
                behind = true;
                break;
            }
        }
        if behind {
            node = next(node);
            continue;
        }

        // projected: each corner times its Q = vf00.w / w; q = Q + vf00.x
        let vf00 = read4(mem, r(mem, VF00));
        let mut screen_pos = [[0u32; 4]; 4];
        let mut q = [0u32; 4];
        for k in 0..4 {
            let c = corners[k];
            let qk = st(ld(vf00[3]) / ld(c[3]));
            mem.write(r(mem, VU_Q), qk);
            let qf = ld(qk);
            screen_pos[k] = [
                st(ld(c[0]) * qf),
                st(qf * ld(c[1])),
                st(qf * ld(c[2])),
                st(qf * ld(c[3])),
            ];
            q[k] = st(qf + ld(vf00[0]));
        }
        mem.write(r(mem, VU_830), q[3]);

        // the clip test against the screen box (1528, 1600)..(2576, 2304),
        // corner by corner while a zero or sign flag (0xc0) is set
        let (box_min, box_max) = ([0x44bf_0000u32, 0x44c8_0000], [0x4521_0000u32, 0x4510_0000]);
        let ee_zero = mem.read::<u32>(r(mem, EE_ZERO));
        for (k, &p) in screen_pos.iter().enumerate() {
            let vec = r(mem, VU0_VECTOR);
            let (z, w) = if k == 0 {
                write4(mem, r(mem, VU_E0), [box_max[0], box_max[1], 0, 0]);
                (0, 0)
            } else {
                (mem.read::<u32>(vec + 8), mem.read::<u32>(vec + 0xc))
            };
            write4(mem, vec, [box_min[0], box_min[1], z, w]);
            mem.write(r(mem, VU_E0), box_max[0]);
            mem.write(r(mem, VU_E0) + 4, box_max[1]);
            write4(mem, r(mem, VU_800), p);
            mem.write(r(mem, CLIP_FLAGS), ee_zero);
            let (clip, f0, s800, e0) = (
                r(mem, VU0_CLIP),
                r(mem, VU_F0),
                r(mem, VU_800),
                r(mem, VU_E0),
            );
            call(mem, clip, &[f0, s800, box_min[0], box_min[1], z, w]);
            let s = read4(mem, s800);
            call(mem, clip, &[f0, e0, s[0], s[1], s[2], s[3]]);
            if k < 3 && mem.read::<u8>(r(mem, CLIP_FLAGS)) & 0xc0 == 0 {
                break;
            }
        }

        // the texture quad: from the block's (+0x30) and size (+0x34) pairs
        let t0 = load_pair(
            mem,
            mem.read::<i16>(blk + 0x30),
            mem.read::<i16>(blk + 0x32),
            0.000_244_140_625,
        );
        let t1 = load_pair(
            mem,
            mem.read::<i16>(blk + 0x34),
            mem.read::<i16>(blk + 0x36),
            0.000_244_140_625,
        );
        let (x1, y1) = (st(ld(t0[0]) + ld(t1[0])), st(ld(t0[1]) + ld(t1[1])));
        let tex = [[t0[0], t0[1]], [x1, t0[1]], [t0[0], y1], [x1, y1]];

        let pkt = PACKET.get(mem);
        let w = |o: u32| pkt.wrapping_add(o);
        mem.write(w(0), 0x8001u32);
        mem.write(w(4), 0xd00a_4000u32);
        mem.write(w(8), 0x2512_512eu32);
        mem.write(w(0xc), 0x0005_1251u32);
        mem.write(w(0x10), mem.read::<u32>(blk + 0x10));
        mem.write(w(0x14), mem.read::<u32>(blk + 0x14));
        mem.write(w(0x18), 6u32);
        mem.write(w(0x1c), 0u32);
        let kind = match mem.read::<u32>(blk + 0x10) & 0x3fff {
            0x3500 => 0x2710,
            k => k,
        };
        // the positions in 16ths of a pixel, through `_ftol2_sse`
        for (k, p) in (0u32..).zip(screen_pos) {
            if k == 3 {
                write4(mem, r(mem, VU_810), p);
            }
            let v = base_of(k);
            for i in 0..2u32 {
                let n = ffx_bridge::x87::ftol(mem, ld(p[i as usize]) * 16.0);
                mem.write(r(mem, VU_800) + 4 * i, n);
                mem.write(w(v + 0x20 + 4 * i), n);
            }
            mem.write(w(v + 0x28), 0x3000u32);
            mem.write(w(v + 0x2c), if k == 0 { 0x8000u32 } else { 0 });
        }
        // STQ: the texture coordinates times q
        for (k, (t, qk)) in (0u32..).zip(tex.iter().zip(q)) {
            if k == 3 {
                mem.write(r(mem, VU_E0), x87::copy(qk));
            }
            let qf = ld(qk);
            let stq = [st(ld(t[0]) * qf), st(ld(t[1]) * qf), st(qf * 1.0), 0];
            write4(mem, r(mem, VU0_VECTOR), stq);
            write4(mem, w(base_of(k)), stq);
            write4(mem, w(base_of(k) + 0x10), [0x40; 4]);
        }

        // the element: screen position, texture coordinates by kind
        for k in 0..4u32 {
            let v = w(base_of(k));
            let e = elem + 0x20 * k;
            let qv = ld(mem.read::<u32>(v + 8));
            let mut s = ld(mem.read::<u32>(v)) / qv;
            let mut t = ld(mem.read::<u32>(v + 4)) / qv;
            (s, t) = (ld(st(s)), ld(st(t)));
            let (s, t) = match kind {
                0x2f00 => (s * 0.5 + 0.375, t),
                0x2710 => (s * 0.25 + 0.75, t * 0.5 + 0.5),
                0x3000 => (s * 0.5 + 0.5, t - 0.25),
                0x3200 => (s * 0.5, t),
                0x3600 => (s * 0.125 + 0.875, t * 0.25 + 0.25),
                _ => (s, t),
            };
            mem.write(e, screen(mem.read::<i32>(v + 0x20), 256.0));
            mem.write(e + 4, screen(mem.read::<i32>(v + 0x24), 208.0));
            mem.write(e + 8, st(s));
            mem.write(e + 0xc, st(t));
            write4(mem, e + 0x10, [0x40, 0x40, 0x40, 0x80]);
        }
        let f = r(mem, DRAW_ELEMENT);
        call(mem, f, &[elem, 0, 2]);
        node = next(node);
    }
    0
}

/// A vertex's `STQ` in the map's packet (`RGBA` at `+0x10`, `XYZ` at
/// `+0x20`).
const fn base_of(k: u32) -> u32 {
    0x20 + 0x30 * k
}
