//! The ability map's (sphere grid's) float-heavy draws: the grid marks and
//! the map of nodes, links and icons.

use ffx_bridge::{Global, Mem};

use crate::abmap::{ABMAP, SIN_TABLE};
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
    let quads = mark_quads(mem);
    let (ld, st) = (x87::load, x87::store);
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

/// `g_GraphicSkipFrame` (`0x012fb7d0`): set, a frame is skipped.
pub const SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);
/// `graphicAbmapGetVertexInfo(name, n)` (`0x00638fd0`).
pub const GET_VERTEX_INFO: u32 = 0x0063_8fd0;
/// `g_StrNoTexture` (`0x00b41b50`): `"NoTexture"`.
pub const NO_TEXTURE: u32 = 0x00b4_1b50;

/// The grid marks' pulsing texture quads (two strips of four corners) for
/// the frame counter `g_Abmap + 0x11698`: `p` runs -1..1 over 512 frames,
/// the first strip spans `p..p + 2`, the second `1 - p .. 3 - p` across and
/// that plus 0.5 down. Each step rounded to `f32` as the originals store.
fn mark_quads(mem: &impl Mem) -> [[[u32; 4]; 4]; 2] {
    let g = ABMAP.get(mem);
    #[allow(clippy::cast_possible_wrap)] // 9 bits
    let frame = (mem.read::<u32>(g.wrapping_add(0x11698)) & 0x1ff) as i32;
    let frame = x87::load(x87::store(f64::from(frame)));
    let p = x87::store(frame * 0.003_906_25 - 1.0);
    let (ld, st) = (x87::load, x87::store);
    let q = st(ld(p) + 2.0);
    let r = st(1.0 - ld(p));
    let r5 = st(ld(r) + 0.5);
    let s = st(ld(r) + 2.0);
    let u = st(ld(r5) + 2.0);
    let (one, zero) = (0x3f80_0000, 0);
    [
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
    ]
}

/// `AbmapGridMarkUVs()` (`0x00a4ac20`, name ours; from `eiAbmDraw`
/// `0x00a545a0`): the texture coordinates of the grid marks drawn through
/// the vertex info `"NoTexture"` (`graphicAbmapGetVertexInfo("NoTexture",
/// 10)`, its `+0x94`, the coordinates at its `+0x18`). Nothing while a
/// frame is skipped ([`SKIP_FRAME`]). For each set cell of the 17 x 20
/// grid (marks `g_Abmap + 0x12e60`) the eight pulsing quad corners of
/// [`abmap_draw_grid_marks`] are scaled by their grid corner's scale
/// (`+0xc` of the corner records at `g_Abmap + 0x116c0`) through the
/// emulated VU0 registers (`0x00c8f7d0`, the scale at `0x00c8f7e0`) into
/// the packet buffer [`PACKET`] (eight vectors), then the next 16
/// coordinates get `x * 512` and `1 - y * 512` of each.
pub fn abmap_grid_mark_uvs<M, C>(mem: &mut M, call: &mut C)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let marks = ABMAP.get(mem).wrapping_add(MARKS);
    if SKIP_FRAME.get(mem) != 0 {
        return;
    }
    let (f, name) = (mem.rebase(GET_VERTEX_INFO), mem.rebase(NO_TEXTURE));
    let info = call(mem, f, &[name, 10]);
    let info = mem.read::<u32>(info.wrapping_add(0x94));
    let quads = mark_quads(mem);
    let corners = ABMAP.get(mem).wrapping_add(CORNERS);
    let mut marked = 0u32;
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLS {
            if mem.read::<u8>(marks.wrapping_add(row * GRID_COLS + col)) == 0 {
                continue;
            }
            let c00 = corners.wrapping_add(row * CORNER_ROW + col * 0x10);
            let corner = [c00, c00 + 0x10, c00 + CORNER_ROW, c00 + CORNER_ROW + 0x10];
            let out = PACKET.get(mem);
            let corners_twice = corner.iter().cycle();
            for (k, (vec, &c)) in (0u32..).zip(quads.into_iter().flatten().zip(corners_twice)) {
                let scale = mem.read::<u32>(c + 0xc);
                let v = scale_vector(mem, vec, scale);
                mem.write(out.wrapping_add(0x10 * k), v);
            }
            let uv = mem
                .read::<u32>(info.wrapping_add(0x18))
                .wrapping_add(0x40 * marked);
            let (ld, st) = (x87::load, x87::store);
            for k in 0..8u32 {
                let x = mem.read::<u32>(out.wrapping_add(0x10 * k));
                mem.write(uv.wrapping_add(8 * k), st(ld(x) * 512.0));
                let y = mem.read::<u32>(out.wrapping_add(0x10 * k + 4));
                mem.write(uv.wrapping_add(8 * k + 4), st(1.0 - ld(y) * 512.0));
            }
            marked += 1;
        }
    }
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

/// The emulated VU0 registers [`abmap_project_grid`] leaves its work in:
/// the transformed corner (`0x00c8f930`) and the grid point (`0x00c8f940`).
const VU_930: u32 = 0x00c8_f930;
/// `setZSFlag` (`0x0042ef30`) and `setSSFlag` (`0x0042ef20`): the sticky
/// zero and sign bits of [`CLIP_FLAGS`].
const ZS_FLAG: u32 = 0x40;
const SS_FLAG: u32 = 0x80;

/// The status bits a VU0 `vsub.xy` against `0` sets (`setZSFlag`,
/// `setSSFlag`), then `restoreVf00Register` (`0x0042ef00`): vf00 back to
/// `(0, 0, 0, 1)`. A NaN sets neither.
fn vu_flags_xy<M: Mem>(mem: &mut M, d: [u32; 2]) {
    let [a, b] = d.map(x87::load);
    let flags = mem.rebase(CLIP_FLAGS);
    if a == 0.0 || b == 0.0 {
        mem.write(flags, mem.read::<u32>(flags) | ZS_FLAG);
    }
    if a < 0.0 || b < 0.0 {
        mem.write(flags, mem.read::<u32>(flags) | SS_FLAG);
    }
    write4(mem, mem.rebase(VF00), [0, 0, 0, 0x3f80_0000]);
}

/// The grid of [`abmap_project_grid`]: 18 rows from y -2336 and 21
/// columns from x -2560, 256 apart (map units).
const GRID_POINTS: (u32, u32) = (18, 21);

/// `AbmapProjectGrid()` (`0x00a47630`, name ours; first in the ability
/// map's draw `0x00a54660`): the grid corners [`abmap_draw_grid_marks`]
/// and [`abmap_grid_mark_uvs`] use. Each grid point `(x, y, 0)` goes through
/// the view matrix `g_Abmap + 0x113e0` as VU0 macro code (point in
/// `0x00c8f940` with w 1, matrix copied to `0x00c8f950`, accumulator
/// `0x00c8f790`, result `0x00c8f930`; the last row times vf00.w). A point
/// with `w <= 0` (or NaN) is left alone. Else `Q = vf00.w / w`
/// (`0x00c8f788`), the point times `Q`, checked against the screen box
/// (1528, 1600)..(2576, 2304) as the original's `vsub` pair does (status
/// `0x00c8fac4` from the EE zero register, then the zero and sign bits;
/// vf00 restored), and its corner record (`0x10` bytes from `g_Abmap +
/// 0x116c0`) gets `x * 16`, `y * 16`, `z` (`ftol2_sse`) and `Q + vf00.x`;
/// the registers keep the original's last values (`0x00c8f7d0`..,
/// `0x00c8f800`.. the integers, `0x00c8f810` the projected point,
/// `0x00c8f830` `Q + vf00.x`). Then all 340 grid marks (`g_Abmap +
/// 0x12e60`) are set to 1.
pub fn abmap_project_grid<M: Mem>(mem: &mut M) {
    use ffx_bridge::x87::ftol;
    let (ld, st) = (x87::load, x87::store);
    let reg = |mem: &M, a: u32| mem.rebase(a);
    let one = 0x3f80_0000u32;
    let records = ABMAP.get(mem).wrapping_add(CORNERS);
    // vf00.w as the original holds it: read at the start and again after
    // each projected point (restoreVf00Register sets it to 1)
    let mut vf00_w = ld(mem.read::<u32>(reg(mem, VF00) + 0xc));
    let (rows, cols) = GRID_POINTS;
    for row in 0..rows {
        for col in 0..cols {
            let record = records.wrapping_add(0x10 * (row * cols + col));
            #[allow(clippy::cast_possible_wrap)] // small
            let (x, y) = (
                st(-2560.0 + 256.0 * f64::from(col as i32)),
                st(-2336.0 + 256.0 * f64::from(row as i32)),
            );
            write4(mem, reg(mem, VU_OUT), [x, y, 0, one]);
            let g = ABMAP.get(mem);
            let view: [[u32; 4]; 4] = [0, 0x10, 0x20, 0x30].map(|o| read4(mem, g + 0x113e0 + o));
            for (i, row) in (0u32..).zip(view) {
                write4(mem, reg(mem, VU_LOCAL) + 0x10 * i, row);
            }
            let (p, acc) = vu_transform(view, [x, y, 0, st(vf00_w)]);
            write4(mem, reg(mem, VU_ACC), acc);
            write4(mem, reg(mem, VU_930), p);
            // fcom: behind the eye, on it, or NaN
            let w = ld(p[3]);
            if w.is_nan() || w <= 0.0 {
                continue;
            }

            // projected: Q = vf00.w / w
            let q = st(vf00_w / w);
            mem.write(reg(mem, VU_Q), q);
            let qf = ld(q);
            let proj = [
                st(ld(p[0]) * qf),
                st(ld(p[1]) * qf),
                st(ld(p[2]) * qf),
                st(ld(p[3]) * qf),
            ];
            let vf00_x = ld(mem.read::<u32>(reg(mem, VF00)));
            let q_out = st(qf + vf00_x);
            mem.write(reg(mem, VU_830), q_out);

            // the screen box test, two vsub.xy against (1528, 1600) and
            // (2576, 2304), as the original leaves the registers
            let (box_min, box_max) = ([0x44bf_0000u32, 0x44c8_0000], [0x4521_0000u32, 0x4510_0000]);
            write4(
                mem,
                reg(mem, VU0_VECTOR),
                [box_min[0], box_min[1], proj[2], proj[3]],
            );
            mem.write(reg(mem, VU_E0), box_max[0]);
            mem.write(reg(mem, VU_E0) + 4, box_max[1]);
            write4(mem, reg(mem, VU_800), proj);
            let ee_zero = mem.read::<u32>(reg(mem, EE_ZERO));
            mem.write(reg(mem, CLIP_FLAGS), ee_zero);
            let low = [
                st(ld(proj[0]) - ld(box_min[0])),
                st(ld(proj[1]) - ld(box_min[1])),
            ];
            vu_flags_xy(mem, low);
            let high = [
                st(ld(box_max[0]) - ld(proj[0])),
                st(ld(box_max[1]) - ld(proj[1])),
            ];
            mem.write(reg(mem, VU_F0), high[0]);
            mem.write(reg(mem, VU_F0) + 4, high[1]);
            vu_flags_xy(mem, high);
            write4(mem, reg(mem, VU_810), proj);

            // the record: x and y in sixteenths, z, and Q + vf00.x
            let ints = [
                ftol(mem, ld(proj[0]) * 16.0),
                ftol(mem, ld(proj[1]) * 16.0),
                ftol(mem, ld(proj[2])),
                ftol(mem, ld(proj[3])),
            ];
            write4(mem, reg(mem, VU_800), ints);
            write4(mem, record, [ints[0], ints[1], ints[2], x87::copy(q_out)]);
            vf00_w = ld(mem.read::<u32>(reg(mem, VF00) + 0xc));
        }
    }
    let marks = ABMAP.get(mem).wrapping_add(MARKS);
    for k in 0..GRID_ROWS * GRID_COLS {
        mem.write(marks + k, 1u8);
    }
}

/// `AbmapPlyTagRect(tag, out)` (`0x00a482d0`, name ours; three calls in
/// `0x00a5a4b0`): a player tag's screen rectangle.
/// [`crate::abmap::bridge::abmap_setup_ply_tag`] first (the tag's sizes
/// into [`crate::abmap::PLY_TAG`]). The tag's position (`+0x10`, in
/// `0x00c8f940`) goes through the view matrix `g_Abmap + 0x113e0` as VU0
/// macro code (matrix copied to `0x00c8f950`, accumulator `0x00c8f790`,
/// result `0x00c8f930`; the last row times vf00.w). Behind or on the eye
/// (`w < 0` or `w == 0`, not NaN) `out[0]` gets 1e11 and nothing else.
/// Else `Q = vf00.w / w` (`0x00c8f788`, `w` also at `0x00c8f7dc` and
/// `0x00c8f7e0`), the projected point in `0x00c8f7d0`, offset by the tag's
/// `i16 +0x46`/`+0x48 >> 4` over the zoom `g_Abmap + 0x11350`; the width
/// global (`PLY_TAG + 0x24`) gets `ftol(10 / zoom + width)`, and `out` the
/// rectangle `(x0, y0, x1, y1)` that has the point at the corner `+0x4e &
/// 3` picks (0 bottom left, 1 bottom right, 2 top right, 3 top left), the
/// height from `PLY_TAG + 0x20`.
pub fn abmap_ply_tag_rect<M: Mem>(mem: &mut M, tag: u32, out: u32) {
    let (ld, st) = (x87::load, x87::store);
    let reg = |mem: &M, a: u32| mem.rebase(a);
    crate::abmap::bridge::abmap_setup_ply_tag(mem, tag);
    let g = ABMAP.get(mem);
    let pos = read4(mem, tag.wrapping_add(0x10));
    write4(mem, reg(mem, VU_OUT), pos);
    let view: [[u32; 4]; 4] = [0, 0x10, 0x20, 0x30].map(|o| read4(mem, g + 0x113e0 + o));
    for (i, row) in (0u32..).zip(view) {
        write4(mem, reg(mem, VU_LOCAL) + 0x10 * i, row);
    }
    let vf00_w = mem.read::<u32>(reg(mem, VF00) + 0xc);
    let (p, acc) = vu_transform(view, [pos[0], pos[1], pos[2], vf00_w]);
    write4(mem, reg(mem, VU_ACC), acc);
    write4(mem, reg(mem, VU_930), p);
    let w = ld(p[3]);
    if w <= 0.0 {
        mem.write(out, x87::copy(mem.read::<u32>(reg(mem, FAR_AWAY))));
        return;
    }
    mem.write(reg(mem, VU_E0), x87::copy(p[3]));
    let q = st(ld(vf00_w) / w);
    mem.write(reg(mem, VU_Q), q);
    let qf = ld(q);
    let proj = [
        st(ld(p[0]) * qf),
        st(ld(p[1]) * qf),
        st(qf * ld(p[2])),
        p[3],
    ];
    write4(mem, reg(mem, VU0_VECTOR), proj);

    // the anchor point: the projection offset by the tag's own (12.4)
    let zoom = ld(mem.read::<u32>(g.wrapping_add(0x11350)));
    let offset = |mem: &M, at: u32| {
        let v = i32::from(mem.read::<i16>(tag.wrapping_add(at)) >> 4);
        ld(st(f64::from(v))) / zoom
    };
    let x = st(offset(mem, 0x46) + ld(proj[0]));
    let y = st(offset(mem, 0x48) + ld(proj[1]));
    let (width_at, height_at) = (
        reg(mem, crate::abmap::PLY_TAG + 0x24),
        reg(mem, crate::abmap::PLY_TAG + 0x20),
    );
    let ten = mem.read::<f64>(reg(mem, TEN_F64));
    let width = mem.read::<i32>(width_at);
    let width = ffx_bridge::x87::ftol(mem, ten / zoom + f64::from(width));
    mem.write(width_at, width);
    let int = |mem: &M, at: u32| ld(st(f64::from(mem.read::<i32>(at))));
    let (tag_w, tag_h) = (int(mem, width_at), int(mem, height_at));
    let (xf, yf) = (ld(x), ld(y));
    let rect = match mem.read::<u8>(tag.wrapping_add(0x4e)) & 3 {
        0 => [x, st(yf - tag_h), st(tag_w + xf), y],
        1 => [st(xf - tag_w), st(yf - tag_h), x, y],
        2 => [st(xf - tag_w), y, x, st(tag_h + yf)],
        _ => [x, y, st(tag_w + xf), st(tag_h + yf)],
    };
    write4(mem, out, rect);
}

/// 1e11 (`f32`, `.rdata`): the "far away" x of a tag behind the eye.
const FAR_AWAY: u32 = 0x00b7_b710;
/// 10.0 (`f64`, `.rdata`).
const TEN_F64: u32 = 0x00b9_22b0;

/// `FFXVu0MulMatrix(out, a, b)` (`0x006ed5c0`).
const MUL_MATRIX: u32 = 0x006e_d5c0;
/// `op1_md_draw_eiabm(model, request, 4, layer)` (`0x00a657c0`).
const MD_DRAW: u32 = 0x00a6_57c0;
/// The ring models [`abmap_draw_chr_rings`] draws: the cursor's
/// (`0x023057fc`) and the characters' (`0x023057f8`).
const CURSOR_RING: u32 = 0x0230_57fc;
const CHR_RING: u32 = 0x0230_57f8;
/// The cursor ring's colours by character (`0x00c86580`) and each
/// character's ring colour (`0x00c86660`), RGBA words.
const CURSOR_COLORS: u32 = 0x00c8_6580;
const RING_COLORS: u32 = 0x00c8_6660;
/// The rings' render target (`0x01740870`).
const RING_TARGET: u32 = 0x0174_0870;
/// The emulated EE zero register (`0x00c8f508`, four words).
const EE_R0: u32 = 0x00c8_f508;
/// `.rdata` constants: the rings' depth (4, 3, 2: `f32`), the cursor's
/// scale (3, 125, 1/1280: `f64`) and the characters' (1/800: `f64`).
const RING_DEPTHS: [u32; 3] = [0x00b9_22f8, 0x00b3_eb94, 0x00b9_2268];
const CURSOR_SCALE: [u32; 3] = [0x00b2_c5c8, 0x00b7_b6b8, 0x00b7_b628];
const RING_SCALE: u32 = 0x00b7_b638;

/// [`abmap_draw_chr_rings`]' scratch, laid out as the original's frame:
/// the draw request (0x48 bytes) at 0, the model matrix at `+0x78`, the
/// vertex block (eight vectors) at `+0xb8`.
pub const RINGS_WORK: u32 = 0x138;
const RINGS_MATRIX: u32 = 0x78;
const RINGS_VERTS: u32 = 0xb8;

/// `AbmapDrawChrRings()` (`0x00a4c8d0`, name ours; from the ability map's
/// draws `0x00a54660` and `eiAbmDraw`): the rings under the cursor and the
/// characters, each through `op1_md_draw_eiabm(model, request, 4, g_Abmap
/// [+0x116a4])`. The request (`work`, see [`RINGS_WORK`]) holds kind
/// (4, then 0x24), the target 0x01740870, 0x48, a colour word at `+0x18`
/// (alpha byte `+0x1b`), the model matrix pointer `+0x20` and the vertex
/// block's `+0x44`; its bytes `+0x0b` and `+0x30..+0x37` are left as they
/// are (the original never sets them). The vertex block is the EE zero
/// register (`0x00c8f508`) four times over, but vector 3 `(r0.x, r0.y,
/// depth, 1)` and a scale matrix in vectors 4..7 (`s` on the diagonal, the
/// zero register elsewhere, the position in row 7 with z 0, w 1), which
/// `FFXVu0MulMatrix` takes times the view `g_Abmap + 0x113e0`.
/// - The cursor: colour by the current character (`+0x115bc`) from
///   `0x00c86580`, alpha `+0x115c5 / 3`, depth 4, `s = (pulse +0x115a4 * 3 +
///   125) / 1280`, position `+0x11520`, model `0x023057fc`.
/// - Each character `k` of 8 (records 0x50 bytes from `g_Abmap + 0x11088`:
///   radius `+0x3c`, position `+0x00`, node `u16 +0x44`) with a radius
///   above 0, unless it shares the current character's node and is neither
///   it nor `k` 7: colour `0x00c86660[k]`, alpha `16 (1 + sin) + 0x60`
///   (truncated; the sine at `(k << 13) + (frame +0x11698 << 11)` 16ths
///   of the table), times the fade `+0x115f4` when it is below 1, depth 3
///   (2 from `k` 7 on), `s = radius / 800`, model `0x023057f8`.
pub fn abmap_draw_chr_rings<M, C>(mem: &mut M, work: u32, call: &mut C)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let (ld, st) = (x87::load, x87::store);
    let reg = |mem: &M, a: u32| mem.rebase(a);
    let (request, matrix, verts) = (work, work + RINGS_MATRIX, work + RINGS_VERTS);
    let g = ABMAP.get(mem);
    let one = 0x3f80_0000u32;

    // the request's fixed fields, written again before the characters
    let fill_request = |mem: &mut M, kind: u32| {
        mem.write(request, kind);
        mem.write(request + 4, reg(mem, RING_TARGET));
        mem.write(request + 8, 0x48u16);
        mem.write(request + 0xa, 0u8);
        for o in [0xc, 0x10, 0x14, 0x1c, 0x24, 0x28, 0x2c, 0x38, 0x3c, 0x40] {
            mem.write(request + o, 0u32);
        }
        mem.write(request + 0x20, matrix);
        mem.write(request + 0x44, verts);
    };
    // the vertex block: the zero register, but vector 3 and row 7
    let fill_verts = |mem: &mut M, depth: u32| {
        let r0 = read4(mem, reg(mem, EE_R0));
        for k in 0..8 {
            write4(mem, verts + 0x10 * k, r0);
        }
        mem.write(verts + 0x38, x87::copy(depth));
        mem.write(verts + 0x3c, one);
        mem.write(verts + 0x78, 0u32);
        mem.write(verts + 0x7c, one);
    };
    // the scale matrix's diagonal and position, then the model matrix
    let draw = |mem: &mut M, call: &mut C, s: u32, pos: [u32; 2], model: u32| {
        mem.write(verts + 0x40, s);
        mem.write(verts + 0x54, s);
        mem.write(verts + 0x68, s);
        mem.write(verts + 0x70, pos[0]);
        mem.write(verts + 0x74, pos[1]);
        let g = ABMAP.get(mem);
        let f = reg(mem, MUL_MATRIX);
        call(mem, f, &[matrix, g.wrapping_add(0x113e0), verts + 0x40]);
        let layer = mem.read::<u32>(ABMAP.get(mem).wrapping_add(0x116a4));
        let f = reg(mem, MD_DRAW);
        call(mem, f, &[model, request, 4, layer]);
    };

    // the cursor
    let depth = mem.read::<u32>(reg(mem, RING_DEPTHS[0]));
    fill_request(mem, 4);
    let chr = u32::from(mem.read::<u8>(g.wrapping_add(0x115bc)));
    let color = mem.read::<u32>(reg(mem, CURSOR_COLORS) + 4 * chr);
    mem.write(request + 0x18, color);
    mem.write(request + 0x1b, mem.read::<u8>(g.wrapping_add(0x115c5)) / 3);
    fill_verts(mem, depth);
    let model = mem.read::<u32>(reg(mem, CURSOR_RING));
    let [k3, k125, k1280] = CURSOR_SCALE.map(|a| mem.read::<f64>(reg(mem, a)));
    let pulse = ld(mem.read::<u32>(g.wrapping_add(0x115a4)));
    let s = st((pulse * k3 + k125) * k1280);
    let pos = [0x11520, 0x11524].map(|o| mem.read::<u32>(g.wrapping_add(o)));
    draw(mem, call, s, pos, model);

    // the characters
    let mut depth = mem.read::<u32>(reg(mem, RING_DEPTHS[1]));
    let g2 = ABMAP.get(mem);
    let frame = mem.read::<u16>(g2.wrapping_add(0x11698)).wrapping_shl(11);
    let chr = u32::from(mem.read::<u8>(g2.wrapping_add(0x115bc)));
    let chr_node = mem.read::<u16>(g2.wrapping_add(0x110cc + 0x50 * chr));
    fill_request(mem, 0x24);
    fill_verts(mem, depth);
    let scale = mem.read::<f64>(reg(mem, RING_SCALE));
    for k in 0..8u32 {
        if k == 7 {
            depth = mem.read::<u32>(reg(mem, RING_DEPTHS[2]));
            mem.write(verts + 0x38, depth);
        }
        let rec = g.wrapping_add(0x110c4 + 0x50 * k);
        let radius = ld(mem.read::<u32>(rec));
        if radius.is_nan() || radius <= 0.0 {
            continue;
        }
        let g = ABMAP.get(mem);
        let current = u32::from(mem.read::<u8>(g.wrapping_add(0x115bc)));
        if chr_node == mem.read::<u16>(rec + 8) && k != current && k != 7 {
            continue;
        }
        let color = mem.read::<u32>(reg(mem, RING_COLORS) + 4 * k);
        mem.write(request + 0x18, color);
        let at = ((k << 13).wrapping_add(u32::from(frame)) >> 4) & 0xfff;
        let sin = ld(mem.read::<u32>(reg(mem, SIN_TABLE) + 4 * at));
        #[allow(clippy::cast_possible_truncation)] // the low byte
        let alpha = (ffx_bridge::x87::cvttsd2si((1.0 + sin) * 16.0) as u8).wrapping_add(0x60);
        mem.write(request + 0x1b, alpha);
        let fade = ld(mem.read::<u32>(g.wrapping_add(0x115f4)));
        if fade < 1.0 {
            #[allow(clippy::cast_possible_truncation)] // the low byte
            let a = ffx_bridge::x87::ftol(mem, fade * f64::from(alpha)) as u8;
            mem.write(request + 0x1b, a);
        }
        let s = st(scale * radius);
        let pos = [rec - 0x3c, rec - 0x38].map(|a| mem.read::<u32>(a));
        let model = mem.read::<u32>(reg(mem, CHR_RING));
        draw(mem, call, s, pos, model);
    }
}

/// `FFXVu0UnitMatrix(m)` (`0x006ed9a0`).
const UNIT_MATRIX: u32 = 0x006e_d9a0;
/// The sphere grid's board models (`0x023056ec` for the expert grid,
/// `0x023056f0` for the standard one).
const BOARD_MODELS: [u32; 2] = [0x0230_56ec, 0x0230_56f0];
/// 7.0 (`f32`, `.rdata`): the board's depth.
const BOARD_DEPTH: u32 = 0x00b5_f2e8;
/// [`abmap_draw_board`]'s scratch, laid out as the original's frame: the
/// draw request (0x48 bytes) at 0, the model matrix at `+0x78`, the unit
/// matrix at `+0xb8`, the vertex block (four vectors) at `+0xf8`.
pub const BOARD_WORK: u32 = 0x138;

/// `AbmapDrawBoard()` (`0x00a4c700`, name ours; from the ability map's
/// draw `0x00a54660`): the grid's board model through
/// `op1_md_draw_eiabm(model, request, 8, g_Abmap[+0x116a4])`: the
/// expert grid's (`0x023056ec`, request `+0x08` 0x48) when the save menu
/// word (`MsGetSaveMenu`) has bits 14..15 = 2, else the standard one's
/// (`0x023056f0`, 0x44). The request is kind 0x34, the target 0x01740870,
/// colour 0x40404040, the model matrix (`FFXVu0UnitMatrix` times the view
/// `g_Abmap + 0x113e0`, `FFXVu0MulMatrix`) at `+0x20`, the unit matrix at
/// `+0x2c`, the vertex block at `+0x44` (the EE zero register three
/// times, then `(r0.x, r0.y, 7, 1)`); bytes `+0x0b`, `+0x30..+0x37` are
/// never set. `work` is [`BOARD_WORK`] bytes, 16-byte aligned.
pub fn abmap_draw_board<M, C>(mem: &mut M, work: u32, call: &mut C)
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let reg = |mem: &M, a: u32| mem.rebase(a);
    let (request, matrix, unit, verts) = (work, work + 0x78, work + 0xb8, work + 0xf8);
    let f = reg(mem, crate::abmap::MS_GET_SAVE_MENU);
    let menu = call(mem, f, &[]);
    let r0 = read4(mem, reg(mem, EE_R0));
    let depth = mem.read::<u32>(reg(mem, BOARD_DEPTH));
    for k in 0..3 {
        write4(mem, verts + 0x10 * k, r0);
    }
    write4(
        mem,
        verts + 0x30,
        [r0[0], r0[1], x87::copy(depth), 0x3f80_0000],
    );
    mem.write(request, 0x34u32);
    mem.write(request + 4, reg(mem, RING_TARGET));
    mem.write(request + 0xa, 0u8);
    for o in [0xc, 0x10, 0x14, 0x1c, 0x24, 0x28, 0x38, 0x3c, 0x40] {
        mem.write(request + o, 0u32);
    }
    mem.write(request + 0x18, 0x4040_4040u32);
    mem.write(request + 0x20, matrix);
    mem.write(request + 0x2c, unit);
    mem.write(request + 0x44, verts);
    let f = reg(mem, UNIT_MATRIX);
    call(mem, f, &[unit]);
    let view = ABMAP.get(mem).wrapping_add(0x113e0);
    let f = reg(mem, MUL_MATRIX);
    call(mem, f, &[matrix, view, unit]);
    let expert = mem.read::<u32>(menu) & 0xc000 == 0x8000;
    let (model, size) = if expert {
        (BOARD_MODELS[0], 0x48u16)
    } else {
        (BOARD_MODELS[1], 0x44)
    };
    mem.write(request + 8, size);
    let layer = mem.read::<u32>(ABMAP.get(mem).wrapping_add(0x116a4));
    let model = mem.read::<u32>(reg(mem, model));
    let f = reg(mem, MD_DRAW);
    call(mem, f, &[model, request, 8, layer]);
}

/// The CRT's `sprintf` (import slot `0x00b0c3dc`).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
/// `fiosUnifyFilename(path, out, size)` (`0x00679820`).
pub const FIOS_UNIFY_FILENAME: u32 = 0x0067_9820;
/// `graphicDrawUIElement(element, texture, a, b, c)` (`0x0063eee0`).
pub const DRAW_UI_ELEMENT: u32 = 0x0063_eee0;
/// `g_PppPacketPos`, moved on by each UI element.
pub const PPP_PACKET_POS: u32 = 0x0230_fd2c;
/// `"%s/dat00/%s/14080_19.dds.phyre"` and `"%s/dat12/%s/14080_19.dds.phyre"`
/// (`.rdata`), by the grid's layer (`g_Abmap +0x116a4`, 0 or 2).
pub const BAR_TEXTURE_FORMATS: [u32; 2] = [0x00b7_b484, 0x00b7_b4a4];
/// `"/FFX_Data/GameData/PS3Data/menu/abmap"` and `"tex/D3D11"`.
pub const BAR_TEXTURE_PARTS: [u32; 2] = [0x00b7_b45c, 0x00b7_b450];
/// The bar's texture corners (`f32`s, `.rdata`): `u0, v0, u1, v1` for the
/// two looks (the last argument 0 or not).
pub const BAR_UVS: [[u32; 4]; 2] = [
    [0x00b7_b644, 0x00b7_b5dc, 0x00b7_b648, 0x00b6_5620],
    [0x00b7_b64c, 0x00b7_b5dc, 0x00b7_b650, 0x00b7_b5e8],
];
/// The bytes of [`abmap_draw_bar`]'s frame: the element (0x98), the
/// unified path (0x100), the path (0x100).
pub const BAR_WORK: u32 = 0x298;

/// `AbmapDrawBar(x0, y0, x1, y1, look)` (`0x00a45e30`, from `0x00a46120`;
/// name ours): a UI element in `work` (the original's frame, see
/// [`BAR_WORK`]): corners `(x0, y0)` and `(x1, y1)` as floats (`+0`,
/// `+0x20`), their texture corners ([`BAR_UVS`], by `look`) at `+8`/`+0x28`,
/// each corner's four colour words 0x80, `+0x88`, `+0x90`, `+0x94` 0 (the
/// rest is never set); the texture path from [`BAR_TEXTURE_FORMATS`] for
/// layer 0 or 2 (another layer leaves the path buffer as it is) through
/// `fiosUnifyFilename`, then `graphicDrawUIElement(element, path, 1, 0,
/// 0)`, whose `eax` it returns, and `g_PppPacketPos` moves on 0x60.
pub fn abmap_draw_bar<M, C>(mem: &mut M, args: [u32; 5], work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let [x0, y0, x1, y1, look] = args;
    let (element, unified, path) = (work, work.wrapping_add(0x98), work.wrapping_add(0x198));
    let int_float = |v: u32| x87::store(f64::from(v.cast_signed()));
    let uv = BAR_UVS[usize::from(look != 0)].map(|c| x87::copy(mem.read::<u32>(mem.rebase(c))));
    for (corner, (x, y), (u, v)) in [
        (element, (x0, y0), (uv[0], uv[1])),
        (element.wrapping_add(0x20), (x1, y1), (uv[2], uv[3])),
    ] {
        mem.write(corner, [int_float(x), int_float(y), u, v]);
        mem.write(corner.wrapping_add(0x10), [0x80u32; 4]);
    }
    for o in [0x88, 0x90, 0x94] {
        mem.write(element.wrapping_add(o), 0u32);
    }
    let layer = mem.read::<u32>(ABMAP.get(mem).wrapping_add(0x116a4));
    let format = match layer {
        0 => Some(BAR_TEXTURE_FORMATS[0]),
        2 => Some(BAR_TEXTURE_FORMATS[1]),
        _ => None,
    };
    if let Some(format) = format {
        let f = mem.read::<u32>(mem.rebase(SPRINTF_SLOT));
        let [dir, kind] = BAR_TEXTURE_PARTS.map(|s| mem.rebase(s));
        let format = mem.rebase(format);
        call(mem, f, &[path, format, dir, kind]);
    }
    let f = mem.rebase(FIOS_UNIFY_FILENAME);
    call(mem, f, &[path, unified, 0x100]);
    let f = mem.rebase(DRAW_UI_ELEMENT);
    let eax = call(mem, f, &[element, unified, 1, 0, 0]);
    let pos = mem.rebase(PPP_PACKET_POS);
    mem.write(pos, mem.read::<u32>(pos).wrapping_add(0x60));
    eax
}
